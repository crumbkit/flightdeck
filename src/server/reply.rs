//! What the server sends back: the refusals with their problem documents,
//! the runner's refusals, and the headers that every reply, including a
//! run's stream, carries.

use std::io;
use std::net::TcpStream;
use std::time::Duration;

use serde::Serialize;

use super::StartError;
use super::assets::Asset;
use super::http::{self, Body};

#[cfg(test)]
mod tests;

/// The content security policy of every reply. The page may load scripts,
/// styles, images, fonts and API replies from the server alone, and nothing
/// may frame it, change its base URL or submit a form from it.
const CONTENT_SECURITY_POLICY: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// The headers that every reply carries.
const SECURITY: [(&str, &str); 6] = [
    ("Cache-Control", "no-store"),
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "no-referrer"),
    ("Cross-Origin-Resource-Policy", "same-origin"),
    ("Cross-Origin-Opener-Policy", "same-origin"),
    ("Content-Security-Policy", CONTENT_SECURITY_POLICY),
];

/// The content type of a JSON reply.
const JSON: &str = "application/json";

/// The content type of a refusal, an RFC 9457 problem document, and of a
/// runner's refusal, a failure document.
const PROBLEM: &str = "application/problem+json";

/// A request that the server refuses. Each refusal has a stable code, which
/// its problem document names, and says nothing about the secret that a
/// request presented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    /// The `Host` header is not the server's own address.
    Host,
    /// The browser says that another site sent the request.
    CrossSite,
    /// The `Origin` header is another origin, or a `POST` has none.
    Origin,
    /// The method is not one that the server answers, such as `OPTIONS`.
    Method,
    /// A `POST` body is not declared as JSON.
    ContentType,
    /// The body is larger than [`http::BODY_LIMIT`].
    TooLarge,
    /// The request's head is larger than [`http::HEAD_LIMIT`] or has more
    /// than [`http::HEADER_LIMIT`] headers.
    HeadTooLarge,
    /// The request is not HTTP/1.x that the server reads, such as one with
    /// `Transfer-Encoding` or two `Content-Length` headers.
    Malformed,
    /// The server already holds as many connections as it takes.
    Busy,
    /// The body is not the JSON document that the route takes.
    Body,
    /// The body did not arrive whole within the request's time.
    Timeout,
    /// The request has no token, a wrong one, or a malformed header.
    Unauthorized,
    /// The launch code is wrong, used or expired.
    CodeRefused,
    /// No route or asset has the path.
    NotFound,
}

impl Refusal {
    /// Every refusal, in the order its document lists them.
    pub(super) const ALL: [Self; 14] = [
        Self::Host,
        Self::CrossSite,
        Self::Origin,
        Self::Method,
        Self::ContentType,
        Self::TooLarge,
        Self::HeadTooLarge,
        Self::Malformed,
        Self::Busy,
        Self::Body,
        Self::Timeout,
        Self::Unauthorized,
        Self::CodeRefused,
        Self::NotFound,
    ];

    /// Returns the HTTP status of the refusal.
    pub(super) const fn status(self) -> u16 {
        match self {
            Self::Host | Self::CrossSite | Self::Origin => 403,
            Self::Method => 405,
            Self::ContentType => 415,
            Self::TooLarge => 413,
            Self::HeadTooLarge => 431,
            Self::Malformed | Self::Body => 400,
            Self::Busy => 503,
            Self::Timeout => 408,
            Self::Unauthorized | Self::CodeRefused => 401,
            Self::NotFound => 404,
        }
    }

    /// Returns the refusal's stable code, such as
    /// `flightdeck::session::unauthorized`.
    pub(super) const fn code(self) -> &'static str {
        match self {
            Self::Host => "flightdeck::request::host",
            Self::CrossSite => "flightdeck::request::cross_site",
            Self::Origin => "flightdeck::request::origin",
            Self::Method => "flightdeck::request::method",
            Self::ContentType => "flightdeck::request::content_type",
            Self::TooLarge => "flightdeck::request::too_large",
            Self::HeadTooLarge => "flightdeck::request::head_too_large",
            Self::Malformed => "flightdeck::request::malformed",
            Self::Busy => "flightdeck::request::busy",
            Self::Body => "flightdeck::request::body",
            Self::Timeout => "flightdeck::request::timeout",
            Self::Unauthorized => "flightdeck::session::unauthorized",
            Self::CodeRefused => "flightdeck::session::code_refused",
            Self::NotFound => "flightdeck::request::not_found",
        }
    }

    /// Returns the one sentence that the problem document's `title` holds.
    const fn title(self) -> &'static str {
        match self {
            Self::Host => "The Host header is not the address that flightdeck listens on.",
            Self::CrossSite => "Another site sent the request.",
            Self::Origin => "The request comes from another origin, or a POST names no origin.",
            Self::Method => "flightdeck answers only GET and POST.",
            Self::ContentType => "A POST must send its body as application/json.",
            Self::TooLarge => "The request body is larger than flightdeck reads.",
            Self::HeadTooLarge => "The request line and headers are larger than flightdeck reads.",
            Self::Malformed => "The request is not an HTTP/1.1 request that flightdeck reads.",
            Self::Busy => "flightdeck is answering as many connections as it takes. Try again.",
            Self::Body => "The request body is not the JSON document that the route takes.",
            Self::Timeout => "The request body did not arrive in time.",
            Self::Unauthorized => "The request needs the page's token.",
            Self::CodeRefused => {
                "The launch code is not valid. Run flightdeck open again to get a new one."
            }
            Self::NotFound => "Nothing is at this path.",
        }
    }
}

/// The RFC 9457 problem document of a refusal, with flightdeck's `code`.
#[derive(Serialize)]
struct Problem {
    #[serde(rename = "type")]
    kind: &'static str,
    title: &'static str,
    status: u16,
    code: &'static str,
}

/// A reply of known length before it gets its headers.
#[derive(Debug)]
pub(super) enum Reply {
    /// A JSON document.
    Json(Vec<u8>),
    /// One of the embedded assets.
    Asset(&'static Asset),
    /// A refusal.
    Refused(Refusal),
    /// A runner's failure, with the runner's failure document and the
    /// status that `failure_status` gives it.
    Failure {
        /// The HTTP status.
        status: u16,
        /// The `flightdeck/failure/v1` document.
        document: Vec<u8>,
    },
    /// 204 with no body, as a cancel answers.
    Done,
}

/// The problem documents of the refusals, written once when the server
/// starts, so writing a reply builds no JSON, and how long a client has to
/// read a reply.
#[derive(Debug)]
pub(super) struct Replies {
    problems: Vec<(Refusal, Vec<u8>)>,
    write_time: Duration,
}

impl Replies {
    /// Writes the problem document of each refusal, for replies that a
    /// client must read within `write_time`.
    pub(super) fn new(write_time: Duration) -> Result<Self, StartError> {
        let problems = Refusal::ALL
            .into_iter()
            .map(|refusal| {
                let problem = Problem {
                    kind: "about:blank",
                    title: refusal.title(),
                    status: refusal.status(),
                    code: refusal.code(),
                };
                serde_json::to_vec(&problem)
                    .map(|body| (refusal, body))
                    .map_err(StartError::Json)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            problems,
            write_time,
        })
    }

    /// Writes `reply` to `connection` with the security headers and its
    /// content type.
    ///
    /// # Errors
    ///
    /// Returns the error of a write, as when the client has closed the
    /// connection.
    pub(super) fn write(&self, connection: &mut TcpStream, reply: &Reply) -> io::Result<()> {
        let (status, content_type, body): (u16, &str, &[u8]) = match reply {
            Reply::Json(body) => (200, JSON, body),
            Reply::Asset(asset) => (200, asset.content_type(), asset.body()),
            Reply::Refused(refusal) => (refusal.status(), PROBLEM, self.problem(*refusal)),
            Reply::Failure { status, document } => (*status, PROBLEM, document),
            Reply::Done => (204, "", &[]),
        };
        let mut body_writer = start(
            connection,
            status,
            content_type,
            Some(body.len()),
            self.write_time,
        )?;
        body_writer.send(body)
    }

    /// Returns the problem document of `refusal`.
    fn problem(&self, refusal: Refusal) -> &[u8] {
        self.problems
            .iter()
            .find(|(each, _)| *each == refusal)
            .map_or(&[], |(_, body)| body.as_slice())
    }
}

/// Writes the head of a reply with `status`, `content_type` (none when it
/// is empty), the security headers and a body of `length` bytes, and
/// returns the writer of its body. A reply of known length has `write_time`
/// for its head and body together. A body of unknown length, a stream of
/// lines, gives each line `write_time` and ends when the server closes the
/// connection.
///
/// # Errors
///
/// Returns the error of a write, as when the client has closed the
/// connection.
pub(super) fn start<'w>(
    connection: &'w mut TcpStream,
    status: u16,
    content_type: &str,
    length: Option<usize>,
    write_time: Duration,
) -> io::Result<Body<'w>> {
    let mut head = format!("HTTP/1.1 {status} {}\r\n", http::reason(status));
    for (name, value) in SECURITY {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if !content_type.is_empty() {
        head.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    // A 401 reply names the scheme it needs (RFC 9110).
    if status == 401 {
        head.push_str("WWW-Authenticate: Bearer\r\n");
    }
    // A 204 reply has no body and no length (RFC 9110).
    if let Some(length) = length.filter(|_| status != 204) {
        head.push_str(&format!("Content-Length: {length}\r\n"));
    }
    head.push_str("Connection: close\r\n\r\n");
    let mut body = match length {
        Some(_) => Body::whole(connection, write_time),
        None => Body::lines(connection, write_time),
    };
    body.send(head.as_bytes())?;
    Ok(body)
}
