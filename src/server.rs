//! The page's local HTTP server.
//!
//! [`Listener::bind`] listens on 127.0.0.1 on a port that the system picks.
//! [`Server::run`] answers one request at a time on the calling thread
//! until the caller's stop flag is set or [`IDLE_LIMIT`] passes without an
//! authenticated request. Each request passes the checks of
//! `request::check` before it is routed, and every reply carries the
//! security headers of `reply`. The routes are these:
//!
//! - `POST /api/session` exchanges the launch code for the token, once.
//! - `GET /api/model` returns the tool's model and whether the page is
//!   read-only. It needs the token.
//! - Any other `/api/` path needs the token, and then gets 404.
//! - `GET /` and the other asset paths serve the embedded page.
//!
//! `docs/security.md` states what these guards defend against.

mod assets;
mod reply;
mod request;
mod secret;
mod session;

use std::fmt;
use std::io::{self, Read as _};
use std::net::{Ipv4Addr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};
use tiny_http::{Method, Request};

use self::assets::Asset;
use self::reply::{Refusal, Replies, Reply};
use self::request::{API, Address, Head};
pub use self::session::{CODE_LIFETIME, Session};
use crate::model::Tool;

/// How long the server runs without an authenticated request.
pub const IDLE_LIMIT: Duration = Duration::from_secs(30 * 60);

/// How long the server waits for a request before it checks its stop flag
/// and its idle time again. This bounds how long Ctrl-C takes.
pub const POLL: Duration = Duration::from_millis(250);

/// The most bytes of a request body that the server reads.
pub const BODY_LIMIT: usize = 64 * 1024;

/// The source of the current time, which tests replace to move time on
/// without waiting.
pub trait Clock: Sync {
    /// Returns the current time.
    fn now(&self) -> Instant;
}

/// The system's monotonic clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
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
    server: tiny_http::Server,
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
        let server = tiny_http::Server::from_listener(listener, None)
            .map_err(|error| StartError::Listen(io::Error::other(error)))?;
        Ok(Self { server, port })
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
    /// A fixed header with this name is not a valid header.
    Header(&'static str),
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Listen(_) => f.write_str("cannot listen on 127.0.0.1"),
            Self::Json(_) => f.write_str("cannot write the model as JSON"),
            Self::Header(name) => write!(f, "the fixed header {name} is not valid"),
        }
    }
}

impl std::error::Error for StartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Listen(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Header(_) => None,
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
    address: Address,
    session: Session,
    model: Vec<u8>,
    replies: Replies,
    settings: Settings,
}

impl Server {
    /// Returns the server that answers on `listener` with `session`, and
    /// serves the model of `tool`, marked `read_only` when the page may run
    /// only its read-only commands.
    ///
    /// # Errors
    ///
    /// Returns [`StartError::Json`] or [`StartError::Header`] when a reply
    /// that never changes cannot be built.
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
            address: Address::new(listener.port()),
            listener,
            session,
            model,
            replies: Replies::new()?,
            settings,
        })
    }

    /// Answers requests until `stop` is set or the idle limit passes, and
    /// returns which happened. It checks both each time a request is
    /// answered and each time [`Settings::poll`] passes without one.
    ///
    /// # Errors
    ///
    /// Returns the error of a listener that stopped accepting connections.
    pub fn run(mut self, clock: &dyn Clock, stop: &AtomicBool) -> io::Result<Ended> {
        loop {
            if stop.load(Ordering::SeqCst) {
                return Ok(Ended::Stopped);
            }
            if self.session.idle_for(clock.now()) >= self.settings.idle_limit {
                return Ok(Ended::Idle);
            }
            if let Some(request) = self.listener.server.recv_timeout(self.settings.poll)? {
                self.answer(request, clock.now());
            }
        }
    }

    /// Answers one request.
    fn answer(&mut self, mut request: Request, now: Instant) {
        let reply = self.route(&mut request, now).unwrap_or_else(Reply::Refused);
        // A write fails only when the client has closed the connection, and
        // then nobody is left to tell.
        let _ = request.respond(self.replies.response(reply));
    }

    /// Checks `request`, then returns the reply of its route.
    fn route(&mut self, request: &mut Request, now: Instant) -> Result<Reply, Refusal> {
        let route = {
            let head = Head::of(request);
            request::check(&head, &self.address)?;
            let route = Route::of(head.method(), head.path());
            if route.needs_token() {
                self.session.authenticate(head.one("Authorization"), now)?;
            }
            route
        };
        match route {
            Route::Session => {
                let body = read_body(request)?;
                let exchange: Exchange =
                    serde_json::from_slice(&body).map_err(|_| Refusal::Body)?;
                let token = self.session.exchange(&exchange.code, now)?;
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

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("listener", &self.listener)
            .field("session", &self.session)
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
    fn of(method: &Method, path: &str) -> Self {
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

/// Reads the body of `request`, at most [`BODY_LIMIT`] bytes.
fn read_body(request: &mut Request) -> Result<Vec<u8>, Refusal> {
    let mut body = Vec::new();
    let limit = u64::try_from(BODY_LIMIT + 1).unwrap_or(u64::MAX);
    request
        .as_reader()
        .take(limit)
        .read_to_end(&mut body)
        .map_err(|_| Refusal::Body)?;
    if body.len() > BODY_LIMIT {
        return Err(Refusal::TooLarge);
    }
    Ok(body)
}
