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
//! security headers of `reply` and `Connection: close`, and a route may
//! write its body one line at a time. The routes are these:
//!
//! - `POST /api/session` exchanges the launch code for the token, once.
//! - `GET /api/model` returns the tool's model and whether the page is
//!   read-only. It needs the token.
//! - Any other `/api/` path needs the token, and then gets 404.
//! - `GET /` and the other asset paths serve the embedded page.
//!
//! `docs/security.md` states what these guards defend against.

mod assets;
mod http;
mod reply;
mod request;
mod secret;
mod session;

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::Scope;
use std::time::{Duration, Instant, SystemTime};

use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};

use self::assets::Asset;
pub use self::http::{BODY_LIMIT, HEAD_LIMIT, HEADER_LIMIT, REQUEST_TIME, WRITE_TIME};
use self::reply::{Refusal, Replies, Reply};
use self::request::{API, Address, Method};
pub use self::session::{CODE_LIFETIME, Session};
use crate::lock::lock;
use crate::model::Tool;

/// How long the server runs without an authenticated request.
pub const IDLE_LIMIT: Duration = Duration::from_secs(30 * 60);

/// How often the server checks its stop flag and its idle time. This
/// bounds how long Ctrl-C takes.
pub const POLL: Duration = Duration::from_millis(250);

/// The most connections that the server holds at once. A connection over
/// the limit gets a 503 reply and is closed.
pub const CONNECTION_LIMIT: usize = 16;

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
}

impl Settings {
    /// Returns the settings with [`IDLE_LIMIT`] and [`POLL`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            idle_limit: IDLE_LIMIT,
            poll: POLL,
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
    read_only: bool,
    tool: &'a Tool,
}

/// The document of `POST /api/session`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Exchange {
    code: SecretString,
}

/// The server: its listener, its session and the replies it gives.
pub struct Server {
    listener: Listener,
    shared: Shared,
    settings: Settings,
}

/// What every connection's thread reads, and the session it changes.
struct Shared {
    address: Address,
    session: Mutex<Session>,
    model: Vec<u8>,
    replies: Replies,
    connections: Connections,
}

impl Server {
    /// Returns the server that answers on `listener` with `session`, and
    /// serves the model of `tool`, marked `read_only` when the page may run
    /// only its read-only commands.
    ///
    /// # Errors
    ///
    /// Returns [`StartError::Json`] when a reply that never changes cannot
    /// be built.
    pub fn new(
        listener: Listener,
        session: Session,
        tool: &Tool,
        read_only: bool,
        settings: Settings,
    ) -> Result<Self, StartError> {
        let model =
            serde_json::to_vec(&ModelReply { read_only, tool }).map_err(StartError::Json)?;
        Ok(Self {
            shared: Shared {
                address: Address::new(listener.port()),
                session: Mutex::new(session),
                model,
                replies: Replies::new()?,
                connections: Connections::default(),
            },
            listener,
            settings,
        })
    }

    /// Serves until `stop` is set or the idle limit passes, and returns
    /// which happened. It checks both each time [`Settings::poll`] passes.
    /// Before it returns, it closes every open connection and joins every
    /// thread it started.
    ///
    /// # Errors
    ///
    /// Returns the error of a listener that stopped accepting connections.
    pub fn run(self, clock: &dyn Clock, stop: &AtomicBool) -> io::Result<Ended> {
        let shared = &self.shared;
        let listener = &self.listener.listener;
        let closing = AtomicBool::new(false);
        let (failed, failure) = mpsc::channel();
        std::thread::scope(|scope| {
            let closing = &closing;
            let acceptor =
                scope.spawn(move || accept(scope, listener, shared, clock, closing, &failed));
            let ended = self.watch(clock, stop, &failure);
            // The accept thread sees `closing` once it accepts again, so
            // the server connects to itself to wake it. When the connect
            // fails, the accept thread has already ended.
            closing.store(true, Ordering::SeqCst);
            drop(TcpStream::connect((
                Ipv4Addr::LOCALHOST,
                self.listener.port,
            )));
            // Once the accept thread has ended, no connection opens after
            // the ones that `close_all` closes. A panic in it, which only a
            // broken invariant can cause, left no connection to close.
            acceptor.join().ok();
            shared.connections.close_all();
            ended
        })
    }

    /// Waits until `stop` is set, the idle limit passes or the accept thread
    /// fails.
    fn watch(
        &self,
        clock: &dyn Clock,
        stop: &AtomicBool,
        failure: &Receiver<io::Error>,
    ) -> io::Result<Ended> {
        loop {
            if stop.load(Ordering::SeqCst) {
                return Ok(Ended::Stopped);
            }
            if lock(&self.shared.session).idle_for(clock.now()) >= self.settings.idle_limit {
                return Ok(Ended::Idle);
            }
            match failure.recv_timeout(self.settings.poll) {
                Ok(error) => return Err(error),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(io::Error::other("the server stopped accepting connections"));
                }
            }
        }
    }
}

/// Accepts connections until `closing` is set, and answers each on a thread
/// of `scope`. A connection over [`CONNECTION_LIMIT`] gets a 503 reply. A
/// failure to accept goes to `failed`, and ends the thread.
fn accept<'scope>(
    scope: &'scope Scope<'scope, '_>,
    listener: &'scope TcpListener,
    shared: &'scope Shared,
    clock: &'scope dyn Clock,
    closing: &'scope AtomicBool,
    failed: &Sender<io::Error>,
) {
    loop {
        let accepted = listener.accept();
        if closing.load(Ordering::SeqCst) {
            return;
        }
        let mut connection = match accepted {
            Ok((connection, _)) => connection,
            Err(error) => {
                // The receiver is gone only after `run` has stopped watching.
                failed.send(error).ok();
                return;
            }
        };
        // A connection whose timeouts cannot be set is closed unanswered.
        if connection.set_write_timeout(Some(WRITE_TIME)).is_err() {
            continue;
        }
        match shared.connections.admit(&connection) {
            Some(slot) => {
                scope.spawn(move || {
                    shared.answer(&mut connection, clock);
                    drop(slot);
                });
            }
            None => shared.refuse(&mut connection, Refusal::Busy),
        }
    }
}

impl Shared {
    /// Reads one request from `connection`, answers it, and closes the
    /// connection.
    fn answer(&self, connection: &mut TcpStream, clock: &dyn Clock) {
        let request = match http::read_request(connection) {
            Ok(Some(request)) => request,
            // A client that closes or stalls before its head ends gets no
            // reply.
            Ok(None) => return,
            Err(refusal) => return self.refuse(connection, refusal),
        };
        let reply = self
            .route(&request, connection, clock)
            .unwrap_or_else(Reply::Refused);
        // A write fails only when the client has closed the connection or
        // stopped reading, and then nobody is left to tell.
        if self.replies.write(connection, &reply).is_ok() {
            finish(connection);
        }
    }

    /// Writes `refusal` to `connection` and closes it.
    fn refuse(&self, connection: &mut TcpStream, refusal: Refusal) {
        if self
            .replies
            .write(connection, &Reply::Refused(refusal))
            .is_ok()
        {
            finish(connection);
        }
    }

    /// Checks `request`, then returns the reply of its route, which reads
    /// the body from `connection` when it takes one.
    fn route(
        &self,
        request: &http::Request,
        connection: &mut TcpStream,
        clock: &dyn Clock,
    ) -> Result<Reply, Refusal> {
        let head = request.head();
        request::check(head, &self.address)?;
        let route = Route::of(head.method(), head.path());
        if route.needs_token() {
            lock(&self.session).authenticate(head.one("Authorization"), clock.now())?;
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
                Ok(Reply::Json(document.into_bytes()))
            }
            Route::Model => Ok(Reply::Json(self.model.clone())),
            Route::Asset(asset) => Ok(Reply::Asset(asset)),
            Route::ApiNotFound | Route::NotFound => Err(Refusal::NotFound),
        }
    }
}

/// Ends a connection after its reply: no more writes, then what the client
/// still sends is read and dropped for a short time, so the client reads
/// the reply before the connection closes.
fn finish(connection: &mut TcpStream) {
    // The client may already have closed the connection.
    connection.shutdown(Shutdown::Write).ok();
    http::drain(connection);
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

    /// Closes every open connection, so each connection's thread ends at
    /// its next read or write.
    fn close_all(&self) {
        for connection in lock(&self.open).values() {
            // A connection that the client has closed refuses the shutdown.
            connection.shutdown(Shutdown::Both).ok();
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
            .field("settings", &self.settings)
            .finish_non_exhaustive()
    }
}

/// Where a request goes.
#[derive(Debug)]
enum Route {
    /// `POST /api/session`.
    Session,
    /// `GET /api/model`.
    Model,
    /// A `GET` of an embedded asset.
    Asset(&'static Asset),
    /// Another path under `/api/`, which needs the token before it gets 404.
    ApiNotFound,
    /// Any other request.
    NotFound,
}

impl Route {
    /// Returns the route of a request with `method` and `path`.
    fn of(method: Method, path: &str) -> Self {
        match (method, path) {
            (Method::Post, "/api/session") => Self::Session,
            (Method::Get, "/api/model") => Self::Model,
            _ if path.starts_with(API) => Self::ApiNotFound,
            (Method::Get, _) => assets::find(path).map_or(Self::NotFound, Self::Asset),
            _ => Self::NotFound,
        }
    }

    /// Returns true when the route needs the token.
    const fn needs_token(&self) -> bool {
        matches!(self, Self::Model | Self::ApiNotFound)
    }
}
