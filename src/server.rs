//! The page's local HTTP server.
//!
//! [`Listener::bind`] listens on 127.0.0.1 on a port that the system picks.
//! [`Server::run`] serves until the caller's stop flag is set or
//! [`IDLE_LIMIT`] passes without an authenticated request. One thread
//! accepts connections, and each connection gets a thread of its own, at
//! most [`CONNECTION_LIMIT`] at a time. A connection holds one request:
//! `http` reads its head within [`HEAD_LIMIT`] bytes and [`REQUEST_TIME`],
//! the request passes the checks of `request::check`, and only then may its
//! route read a body of at most [`BODY_LIMIT`] bytes. Every reply carries the
//! security headers of `reply` and `Connection: close`. The routes are these:
//!
//! - `POST /api/session` exchanges the launch code for the token, once.
//! - `GET /api/model` returns the tool's model, whether the page is
//!   read-only and the directory that flightdeck started in.
//! - `POST /api/runs` starts a run, `POST /api/previews` starts a preview,
//!   and `POST /api/confirmations/<id>` starts a confirmed run. Each takes a
//!   [`RunRequest`] and answers with the stream of `stream`, or with the
//!   runner's failure document and the status of `failure_status`.
//! - `POST /api/runs/<id>/cancel` cancels a run, and answers 204, or 404
//!   when the run is not running.
//! - `POST /api/choices` takes a [`ChoicesRequest`], runs the read-only
//!   source of an inferred link, and answers with its [`Choices`] as one
//!   JSON document, or with the runner's failure document.
//! - Any other `/api/` path gets 404.
//! - `GET /` and the other asset paths serve the embedded page.
//!
//! Every `/api/` path except `/api/session` needs the token. One
//! [`Runner`] serves every connection. When the server stops, it stops
//! accepting, lets the runner's shutdown end every run, and then joins
//! every thread (SVC-2). After the runner's shutdown, each stream has one
//! write time in all to write its remaining lines. `docs/security.md` states what these
//! guards defend against.

mod assets;
mod document;
mod http;
mod reply;
mod request;
mod secret;
mod session;
mod stream;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::sync::{Mutex, OnceLock};
use std::thread::Scope;
use std::time::{Duration, Instant, SystemTime};

use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};

use self::assets::Asset;
pub use self::http::{BODY_LIMIT, HEAD_LIMIT, HEADER_LIMIT, REQUEST_TIME, WRITE_TIME};
use self::reply::{Refusal, Replies, Reply};
use self::request::{API, Address, Head, Method};
pub use self::session::{CODE_LIFETIME, Session};
use self::stream::{Pacing, Stream};
use crate::lock::lock;
use crate::model::Tool;
use crate::runner::{Choices, ChoicesRequest, MOST_RUNS, Mode, RunFailure, RunRequest, Runner};

/// How long the server runs without an authenticated request.
pub const IDLE_LIMIT: Duration = Duration::from_secs(30 * 60);

/// How often the server checks its stop flag and its idle time. This
/// bounds how long Ctrl-C takes.
pub const POLL: Duration = Duration::from_millis(250);

/// The most connections that the server holds at once. A connection over
/// the limit gets a 503 reply and is closed.
pub const CONNECTION_LIMIT: usize = 16;

/// How long a stream goes without output before the server writes an
/// `alive` line, which also tells it whether the client is still there.
pub const KEEPALIVE: Duration = Duration::from_secs(15);

// Each run's stream holds a connection until the run ends, and a cancel
// needs a connection of its own. The runner's cap leaves at least half the
// connections free while every run streams.
const _: () = assert!(MOST_RUNS * 2 <= CONNECTION_LIMIT);

/// The source of the current time, which tests replace to move time on
/// without waiting.
pub trait Clock: Sync {
    /// Returns the current time by the monotonic clock and the wall clock.
    fn now(&self) -> Moment;
}

/// The system's monotonic clock and wall clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Moment {
        Moment::new(Instant::now(), SystemTime::now())
    }
}

/// One moment by the monotonic clock and by the wall clock.
///
/// On macOS the monotonic clock stops while the machine sleeps, and the
/// wall clock keeps counting. A time limit that both clocks measure ends
/// when either says that it has passed, so a sleeping machine cannot keep a
/// launch code or an idle session valid for longer than its limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Moment {
    monotonic: Instant,
    wall: SystemTime,
}

impl Moment {
    /// Returns the moment at `monotonic` by the monotonic clock and `wall`
    /// by the wall clock.
    #[must_use]
    pub const fn new(monotonic: Instant, wall: SystemTime) -> Self {
        Self { monotonic, wall }
    }

    /// Returns the moment by the wall clock.
    #[must_use]
    pub const fn wall(self) -> SystemTime {
        self.wall
    }

    /// Returns how long has passed from `earlier` to this moment: the
    /// longer of the two clocks' measures. A wall clock set back measures
    /// no time.
    #[must_use]
    pub fn since(self, earlier: Self) -> Duration {
        let monotonic = self.monotonic.saturating_duration_since(earlier.monotonic);
        let wall = self.wall.duration_since(earlier.wall).unwrap_or_default();
        monotonic.max(wall)
    }
}

/// The server's limits.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    idle_limit: Duration,
    poll: Duration,
    keepalive: Duration,
    request_time: Duration,
    write_time: Duration,
}

impl Settings {
    /// Returns the settings with [`IDLE_LIMIT`], [`POLL`], [`KEEPALIVE`],
    /// [`REQUEST_TIME`] and [`WRITE_TIME`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            idle_limit: IDLE_LIMIT,
            poll: POLL,
            keepalive: KEEPALIVE,
            request_time: REQUEST_TIME,
            write_time: WRITE_TIME,
        }
    }

    /// Returns the settings with `idle_limit` in place of [`IDLE_LIMIT`].
    #[must_use]
    pub const fn idle_limit(mut self, idle_limit: Duration) -> Self {
        self.idle_limit = idle_limit;
        self
    }

    /// Returns the settings with `poll` in place of [`POLL`].
    #[must_use]
    pub const fn poll(mut self, poll: Duration) -> Self {
        self.poll = poll;
        self
    }

    /// Returns the settings with `keepalive` in place of [`KEEPALIVE`].
    #[must_use]
    pub const fn keepalive(mut self, keepalive: Duration) -> Self {
        self.keepalive = keepalive;
        self
    }

    /// Returns the settings with `request_time` in place of
    /// [`REQUEST_TIME`].
    #[must_use]
    pub const fn request_time(mut self, request_time: Duration) -> Self {
        self.request_time = request_time;
        self
    }

    /// Returns the settings with `write_time` in place of [`WRITE_TIME`].
    #[must_use]
    pub const fn write_time(mut self, write_time: Duration) -> Self {
        self.write_time = write_time;
        self
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

/// A server that listens on 127.0.0.1 and has not started to answer.
pub struct Listener {
    listener: TcpListener,
    port: u16,
}

impl Listener {
    /// Listens on 127.0.0.1 on a port that the system picks.
    ///
    /// # Errors
    ///
    /// Returns [`StartError::Listen`] when the system refuses the socket.
    pub fn bind() -> Result<Self, StartError> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(StartError::Listen)?;
        let port = listener.local_addr().map_err(StartError::Listen)?.port();
        Ok(Self { listener, port })
    }

    /// Returns the port that the system picked.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Returns the server's origin, such as `http://127.0.0.1:52710`.
    #[must_use]
    pub fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl fmt::Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Listener")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

/// A server that could not start.
#[derive(Debug)]
pub enum StartError {
    /// The system refused to listen on 127.0.0.1.
    Listen(io::Error),
    /// The model or a fixed reply could not be written as JSON.
    Json(serde_json::Error),
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Listen(_) => f.write_str("cannot listen on 127.0.0.1"),
            Self::Json(_) => f.write_str("cannot write the model as JSON"),
        }
    }
}

impl std::error::Error for StartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Listen(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}

/// Why a server stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// The caller set the stop flag, as Ctrl-C does.
    Stopped,
    /// No authenticated request arrived within the idle limit.
    Idle,
}

/// The document of `GET /api/model`.
#[derive(Serialize)]
struct ModelReply<'a> {
    directory: Option<&'a str>,
    read_only: bool,
    tool: &'a Tool,
}

/// The document of `POST /api/session`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Exchange {
    code: SecretString,
}

/// The server: its listener, what its connections share, and the channel
/// on which the watcher learns of a failure or a closed session.
pub struct Server {
    listener: Listener,
    shared: Shared,
    wakes: Receiver<Wake>,
}

/// What every connection's thread reads, and the session it changes.
struct Shared {
    address: Address,
    session: Mutex<Session>,
    model: Vec<u8>,
    replies: Replies,
    connections: Connections,
    runner: Runner,
    settings: Settings,
    wake: SyncSender<Wake>,
    cut_off: OnceLock<Instant>,
}

/// Why the watcher wakes before its poll.
#[derive(Debug)]
enum Wake {
    /// The accept thread stopped on this error.
    Failed(io::Error),
    /// A request found the session closed by the idle limit.
    Closed,
}

impl Server {
    /// Returns the server that answers on `listener` with `session`, and
    /// runs commands with `runner`. It serves the model of the runner's
    /// tool, marked read-only when the runner is.
    ///
    /// # Errors
    ///
    /// Returns [`StartError::Json`] when a reply that never changes cannot
    /// be built.
    pub fn new(
        listener: Listener,
        session: Session,
        runner: Runner,
        settings: Settings,
        directory: Option<&str>,
    ) -> Result<Self, StartError> {
        let model = serde_json::to_vec(&ModelReply {
            directory,
            read_only: runner.mode() == Mode::ReadOnly,
            tool: runner.tool(),
        })
        .map_err(StartError::Json)?;
        // One waiting wake is enough: the watcher stops at the first.
        let (wake, wakes) = mpsc::sync_channel(1);
        Ok(Self {
            shared: Shared {
                address: Address::new(listener.port()),
                session: Mutex::new(session),
                model,
                replies: Replies::new(settings.write_time)?,
                connections: Connections::default(),
                runner,
                settings,
                wake,
                cut_off: OnceLock::new(),
            },
            listener,
            wakes,
        })
    }

    /// Serves until `stop` is set or the idle limit passes, and returns
    /// which happened. It checks both each time [`Settings::poll`] passes.
    ///
    /// Before it returns, it stops accepting connections, shuts the runner
    /// down, which lets each run end within the runner's shutdown grace and
    /// cancels the rest, closes the reading side of every open connection,
    /// and joins every thread it started. A stream whose run ended writes its
    /// `ended` line before its thread ends. Once the runner's shutdown has
    /// returned, each stream has [`Settings::write_time`] in all to write
    /// the lines it has left, so the stop takes at most that long after the
    /// runner's shutdown.
    ///
    /// # Errors
    ///
    /// Returns the error of a listener that stopped accepting connections.
    pub fn run(self, clock: &dyn Clock, stop: &AtomicBool) -> io::Result<Ended> {
        let Self {
            listener,
            shared,
            wakes,
        } = self;
        let shared = &shared;
        let socket = &listener.listener;
        // Dropping `closing` is the closing signal, which the accept thread
        // reads from `closed`.
        let (closing, closed) = mpsc::channel::<()>();
        std::thread::scope(|scope| {
            let acceptor = scope.spawn(move || accept(scope, socket, shared, clock, &closed));
            let ended = shared.watch(clock, stop, &wakes);
            drop(closing);
            // The accept thread sees the signal once it accepts again, so
            // the server connects to itself to wake it. When the connect
            // fails, the accept thread has already ended. When the listen
            // backlog is full, the connect gives up after one poll, and a
            // connection already waiting in the backlog wakes the thread.
            let own = SocketAddr::from((Ipv4Addr::LOCALHOST, listener.port));
            drop(TcpStream::connect_timeout(&own, shared.settings.poll));
            // Once the accept thread has ended, no connection opens after
            // the ones that `close_reads` closes. A panic in it, which only a
            // broken invariant can cause, left no connection to close.
            acceptor.join().ok();
            shared.runner.shutdown();
            // Every run has ended, so each stream has only its last lines
            // left, and it has one write time in all to write them. A line
            // already being written ends by its own deadline, which is no
            // later.
            let cut_off = Instant::now() + shared.settings.write_time;
            shared.cut_off.set(cut_off).ok();
            shared.connections.close_reads();
            ended
        })
    }
}

/// Accepts connections until the closing signal, and answers each on a
/// thread of `scope`. A connection over [`CONNECTION_LIMIT`] gets a 503
/// reply. A failure for lack of descriptors, memory or buffers, or an
/// aborted connection, waits one poll and accepts again. Any other failure
/// wakes the watcher and ends the thread.
fn accept<'scope>(
    scope: &'scope Scope<'scope, '_>,
    listener: &'scope TcpListener,
    shared: &'scope Shared,
    clock: &'scope dyn Clock,
    closed: &Receiver<()>,
) {
    loop {
        let accepted = listener.accept();
        if matches!(closed.try_recv(), Err(TryRecvError::Disconnected)) {
            return;
        }
        let mut connection = match accepted {
            Ok((connection, _)) => connection,
            Err(error) if passes(&error) => match closed.recv_timeout(shared.settings.poll) {
                Err(RecvTimeoutError::Timeout) => continue,
                Ok(()) | Err(RecvTimeoutError::Disconnected) => return,
            },
            Err(error) => {
                // A full channel already holds a wake that stops the server.
                shared.wake.try_send(Wake::Failed(error)).ok();
                return;
            }
        };
        match shared.connections.admit(&connection) {
            Some(slot) => {
                scope.spawn(move || {
                    shared.answer(&mut connection, clock);
                    drop(slot);
                });
            }
            None => shared.refuse_busy(&mut connection),
        }
    }
}

/// Returns true for a failure to accept that passes once the system frees
/// a descriptor, memory or a buffer, or that ends one connection only.
#[cfg(unix)]
fn passes(error: &io::Error) -> bool {
    use rustix::io::Errno;

    Errno::from_io_error(error).is_some_and(|errno| {
        [
            Errno::MFILE,
            Errno::NFILE,
            Errno::CONNABORTED,
            Errno::NOBUFS,
            Errno::NOMEM,
        ]
        .contains(&errno)
    })
}

/// Returns true for a failure to accept that passes once the system frees
/// a socket, memory or a buffer, or that ends one connection only: the
/// Windows Sockets errors WSAEMFILE (10024) and WSAENOBUFS (10055), and the
/// kinds of an aborted connection and of no memory.
#[cfg(not(unix))]
fn passes(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(10024 | 10055))
        || matches!(
            error.kind(),
            io::ErrorKind::ConnectionAborted | io::ErrorKind::OutOfMemory
        )
}

impl Shared {
    /// Waits until `stop` is set, the idle limit passes or a wake arrives
    /// from `wakes`.
    fn watch(
        &self,
        clock: &dyn Clock,
        stop: &AtomicBool,
        wakes: &Receiver<Wake>,
    ) -> io::Result<Ended> {
        loop {
            if stop.load(Ordering::SeqCst) {
                return Ok(Ended::Stopped);
            }
            if lock(&self.session).close_when_idle(clock.now(), self.settings.idle_limit) {
                return Ok(Ended::Idle);
            }
            match wakes.recv_timeout(self.settings.poll) {
                Ok(Wake::Failed(error)) => return Err(error),
                Ok(Wake::Closed) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(io::Error::other("the server stopped accepting connections"));
                }
            }
        }
    }

    /// Reads one request from `connection`, answers it, and closes the
    /// connection.
    fn answer(&self, connection: &mut TcpStream, clock: &dyn Clock) {
        let request = match http::read_request(connection, self.settings.request_time) {
            Ok(Some(request)) => request,
            // A client that closes or stalls before its head ends gets no
            // reply.
            Ok(None) => {
                connection.shutdown(Shutdown::Both).ok();
                return;
            }
            Err(refusal) => return self.refuse(connection, refusal),
        };
        let answer = self
            .route(&request, connection, clock)
            .unwrap_or_else(|refusal| Answer::Reply(Reply::Refused(refusal)));
        // A write fails only when the client has closed the connection or
        // stopped reading, and then nobody is left to tell. A failed stream
        // drops its run, which cancels it.
        let written = match answer {
            Answer::Stream(stream) => {
                let pacing = Pacing {
                    keepalive: self.settings.keepalive,
                    write_time: self.settings.write_time,
                    cut_off: &self.cut_off,
                };
                stream::write(connection, stream, clock, &pacing)
            }
            Answer::Reply(reply) => self.replies.write(connection, &reply),
        };
        end(connection, written);
    }

    /// Writes `refusal` to `connection` and closes it.
    fn refuse(&self, connection: &mut TcpStream, refusal: Refusal) {
        let written = self.replies.write(connection, &Reply::Refused(refusal));
        end(connection, written);
    }

    /// Writes the busy refusal to `connection` on the accept thread and
    /// closes the connection without draining it, so the accept thread
    /// never waits for a client. The reply fits in the socket's send
    /// buffer, so its write does not wait either. What the client has
    /// already sent is read and dropped without waiting.
    fn refuse_busy(&self, connection: &mut TcpStream) {
        // The client may already have closed the connection.
        self.replies
            .write(connection, &Reply::Refused(Refusal::Busy))
            .ok();
        connection.shutdown(Shutdown::Write).ok();
        http::discard_arrived(connection);
    }

    /// Checks `request`, then returns the reply of its route, which reads
    /// the body from `connection` when it takes one.
    fn route(
        &self,
        request: &http::Request,
        connection: &mut TcpStream,
        clock: &dyn Clock,
    ) -> Result<Answer, Refusal> {
        let head = request.head();
        request::check(head, &self.address)?;
        let route = Route::of(head.method(), head.path());
        if route.needs_token() {
            self.authenticate(head, clock)?;
        }
        match route {
            Route::Session => {
                let body = request.body(connection)?;
                let exchange: Exchange =
                    serde_json::from_slice(&body).map_err(|_| Refusal::Body)?;
                let mut session = lock(&self.session);
                let token = session.exchange(&exchange.code, clock.now())?;
                // The token is hex digits, which JSON needs no escape for.
                let document = format!(r#"{{"token":"{}"}}"#, token.expose_secret());
                Ok(Answer::Reply(Reply::Json(document.into_bytes())))
            }
            Route::Model => Ok(Answer::Reply(Reply::Json(self.model.clone()))),
            Route::Runs => {
                let run = run_request(request, connection)?;
                Ok(streamed(
                    self.runner.run(&run).map(|run| Stream::Run(Box::new(run))),
                ))
            }
            Route::Previews => {
                let run = run_request(request, connection)?;
                Ok(streamed(
                    self.runner
                        .preview(&run)
                        .map(|preview| Stream::Preview(Box::new(preview))),
                ))
            }
            Route::Confirmation(id) => {
                let run = run_request(request, connection)?;
                let confirmed = id
                    .parse()
                    .and_then(|id| self.runner.confirm(&id, &run))
                    .map(|run| Stream::Run(Box::new(run)));
                Ok(streamed(confirmed))
            }
            Route::Choices => {
                let body = request.body(connection)?;
                let choices: ChoicesRequest =
                    serde_json::from_slice(&body).map_err(|_| Refusal::Body)?;
                Ok(Answer::Reply(match self.runner.choices(&choices) {
                    Ok(found) => Reply::Json(choices_json(&found)),
                    Err(failure) => failed(&failure),
                }))
            }
            Route::Cancel(id) => Ok(Answer::Reply(
                match id.parse().and_then(|id| self.runner.cancel(id)) {
                    Ok(()) => Reply::Done,
                    Err(failure) => failed(&failure),
                },
            )),
            Route::Asset(asset) => Ok(Answer::Reply(Reply::Asset(asset))),
            Route::ApiNotFound | Route::NotFound => Err(Refusal::NotFound),
        }
    }

    /// Accepts the request's token. A request that finds the session
    /// closed by the idle limit wakes the watcher, which stops the server.
    fn authenticate(&self, head: &Head, clock: &dyn Clock) -> Result<(), Refusal> {
        let mut session = lock(&self.session);
        let authenticated = session.authenticate(
            head.one("Authorization"),
            clock.now(),
            self.settings.idle_limit,
        );
        if session.is_closed() {
            // A full channel already holds a wake that stops the server.
            self.wake.try_send(Wake::Closed).ok();
        }
        authenticated
    }
}

/// Reads the body of `request` from `connection` as a run request.
fn run_request(request: &http::Request, connection: &mut TcpStream) -> Result<RunRequest, Refusal> {
    let body = request.body(connection)?;
    serde_json::from_slice(&body).map_err(|_| Refusal::Body)
}

/// Returns `choices` as JSON.
fn choices_json(choices: &Choices) -> Vec<u8> {
    // serde_json fails only on a map key that is not a string, and the
    // choices have no map.
    serde_json::to_vec(choices).unwrap_or_default()
}

/// What a route answers: a reply of known length, or a run's stream.
enum Answer {
    /// A reply of known length, which `Replies::write` writes.
    Reply(Reply),
    /// A run's stream of JSON lines, which `stream::write` writes.
    Stream(Stream),
}

/// Returns the answer of a run route: the stream, or the runner's failure.
fn streamed(started: Result<Stream, RunFailure>) -> Answer {
    match started {
        Ok(stream) => Answer::Stream(stream),
        Err(failure) => Answer::Reply(failed(&failure)),
    }
}

/// Returns the reply of `failure`: its failure document with the status of
/// [`failure_status`].
fn failed(failure: &RunFailure) -> Reply {
    Reply::Failure {
        status: failure_status(failure),
        document: document::to_json(failure),
    }
}

/// Returns the HTTP status of a runner's failure. A request that the runner
/// refuses gets 422, a cancel that finds no run gets 404, a runner that is
/// full or shutting down gets 503, and a tool that will not start or a
/// system that gives no random bytes gets 500. A choices source that ends
/// without a listing gets 502, and one that outlasts the choices time gets
/// 504.
fn failure_status(failure: &RunFailure) -> u16 {
    match failure {
        RunFailure::Refused { .. }
        | RunFailure::NeedsConfirmation { .. }
        | RunFailure::ReadOnly { .. }
        | RunFailure::Confirmation { .. }
        | RunFailure::Directory { .. } => 422,
        RunFailure::NotRunning { .. } => 404,
        RunFailure::Busy { .. } | RunFailure::Closed => 503,
        RunFailure::Spawn { .. } | RunFailure::Random { .. } => 500,
        RunFailure::NoListing { .. } => 502,
        RunFailure::ChoicesTime { .. } => 504,
    }
}

/// Ends a connection after its reply. After a reply that was written: no
/// more writes, then what the client still sends is read and dropped for a
/// short time, so the client reads the reply before the connection closes.
/// After a write that failed, the connection is shut down both ways at
/// once.
fn end(connection: &mut TcpStream, written: io::Result<()>) {
    // The client may already have closed the connection.
    if written.is_ok() {
        connection.shutdown(Shutdown::Write).ok();
        http::drain(connection);
    } else {
        connection.shutdown(Shutdown::Both).ok();
    }
}

/// The open connections, at most [`CONNECTION_LIMIT`], each with a handle
/// that closes it when the server stops.
#[derive(Debug, Default)]
struct Connections {
    open: Mutex<BTreeMap<u64, TcpStream>>,
    next: AtomicU64,
}

impl Connections {
    /// Takes `connection` when fewer than [`CONNECTION_LIMIT`] are open, and
    /// returns its slot, which frees the place when it is dropped.
    fn admit(&self, connection: &TcpStream) -> Option<Slot<'_>> {
        let handle = connection.try_clone().ok()?;
        let mut open = lock(&self.open);
        if open.len() >= CONNECTION_LIMIT {
            return None;
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        open.insert(id, handle);
        Some(Slot {
            connections: self,
            id,
        })
    }

    /// Closes the reading side of every open connection, so a thread that
    /// waits for a request or its body ends at once. A thread that writes
    /// its reply, such as the last lines of a stream, ends by its write
    /// deadline.
    fn close_reads(&self) {
        for connection in lock(&self.open).values() {
            // A connection that the client has closed refuses the shutdown.
            connection.shutdown(Shutdown::Read).ok();
        }
    }
}

/// The place of one open connection.
#[derive(Debug)]
struct Slot<'a> {
    connections: &'a Connections,
    id: u64,
}

impl Drop for Slot<'_> {
    fn drop(&mut self) {
        lock(&self.connections.open).remove(&self.id);
    }
}

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("listener", &self.listener)
            .field("settings", &self.shared.settings)
            .finish_non_exhaustive()
    }
}

/// The path prefix of a confirmed run, which the confirmation id follows.
const CONFIRMATIONS: &str = "/api/confirmations/";

/// The path prefix of a run, which its id and `/cancel` follow.
const RUNS: &str = "/api/runs/";

/// Where a request goes.
#[derive(Debug)]
enum Route<'a> {
    /// `POST /api/session`.
    Session,
    /// `GET /api/model`.
    Model,
    /// `POST /api/runs`.
    Runs,
    /// `POST /api/previews`.
    Previews,
    /// `POST /api/confirmations/<id>`, with the id as the page sent it.
    Confirmation(&'a str),
    /// `POST /api/runs/<id>/cancel`, with the id as the page sent it.
    Cancel(&'a str),
    /// `POST /api/choices`.
    Choices,
    /// A `GET` of an embedded asset.
    Asset(&'static Asset),
    /// Another path under `/api/`, which needs the token before it gets 404.
    ApiNotFound,
    /// Any other request.
    NotFound,
}

impl<'a> Route<'a> {
    /// Returns the route of a request with `method` and `path`.
    fn of(method: Method, path: &'a str) -> Self {
        if method == Method::Post {
            if let Some(id) = path.strip_prefix(CONFIRMATIONS) {
                return Self::Confirmation(id);
            }
            if let Some(id) = path
                .strip_prefix(RUNS)
                .and_then(|rest| rest.strip_suffix("/cancel"))
            {
                return Self::Cancel(id);
            }
        }
        match (method, path) {
            (Method::Post, "/api/session") => Self::Session,
            (Method::Get, "/api/model") => Self::Model,
            (Method::Post, "/api/runs") => Self::Runs,
            (Method::Post, "/api/previews") => Self::Previews,
            (Method::Post, "/api/choices") => Self::Choices,
            _ if path.starts_with(API) => Self::ApiNotFound,
            (Method::Get, _) => assets::find(path).map_or(Self::NotFound, Self::Asset),
            _ => Self::NotFound,
        }
    }

    /// Returns true when the route needs the token: every `/api/` path
    /// except `/api/session`.
    const fn needs_token(&self) -> bool {
        !matches!(self, Self::Session | Self::Asset(_) | Self::NotFound)
    }
}
