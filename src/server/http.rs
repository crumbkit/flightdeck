//! HTTP/1.1 on one connection: reading a request's head and body within
//! their limits, and writing a reply, whole or one piece at a time.
//!
//! Every reply carries `Connection: close`, and the server closes the
//! connection after it, so a connection holds one request. A body is read
//! only when the route asks for it, after the request has passed the checks
//! of `request::check`.

use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use super::reply::Refusal;
use super::request::{Head, Method};

/// The most bytes of a request's head, its request line and headers, that
/// the server reads.
pub const HEAD_LIMIT: usize = 16 * 1024;

/// The most headers that a request may have.
pub const HEADER_LIMIT: usize = 32;

/// The most bytes of a request body that the server reads.
pub const BODY_LIMIT: usize = 64 * 1024;

/// How long a client has to send a request's head and body together.
pub const REQUEST_TIME: Duration = Duration::from_secs(10);

/// How long one write of a reply may wait for the client to read.
pub const WRITE_TIME: Duration = Duration::from_secs(10);

/// The most bytes that the server reads and drops after a reply, so that a
/// client that is still sending gets the reply before the connection
/// closes.
const DRAIN_LIMIT: u64 = 64 * 1024;

/// How long the server reads and drops after a reply.
const DRAIN_TIME: Duration = Duration::from_millis(500);

/// A connection whose reads all end at one deadline.
pub(super) trait Timed: Read + Write {
    /// Sets how long the next read may wait.
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()>;
}

impl Timed for std::net::TcpStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        Self::set_read_timeout(self, timeout)
    }
}

/// A request whose head has been read, and the bytes of its body that
/// arrived with the head.
#[derive(Debug)]
pub(super) struct Request {
    head: Head,
    early_body: Vec<u8>,
    deadline: Instant,
}

impl Request {
    /// Returns the request's head.
    pub(super) const fn head(&self) -> &Head {
        &self.head
    }

    /// Reads the request's body from `connection`, which declares at most
    /// [`BODY_LIMIT`] bytes, as `request::check` ensures. A body that
    /// arrives short of its declared length before the deadline is refused.
    pub(super) fn body(&self, connection: &mut impl Timed) -> Result<Vec<u8>, Refusal> {
        let length = self.head.body_length().unwrap_or(0);
        let length = usize::try_from(length).map_err(|_| Refusal::TooLarge)?;
        if length > BODY_LIMIT {
            return Err(Refusal::TooLarge);
        }
        let mut body = self.early_body.clone();
        body.truncate(length);
        let mut chunk = [0; 4096];
        while body.len() < length {
            let read = read_by(connection, self.deadline, &mut chunk).map_err(|_| Refusal::Body)?;
            let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
                return Err(Refusal::Body);
            };
            let wanted = length - body.len();
            body.extend_from_slice(bytes.get(..wanted).unwrap_or(bytes));
        }
        Ok(body)
    }
}

/// Reads a request's head from `connection` within [`REQUEST_TIME`], which
/// also bounds the read of its body.
///
/// # Errors
///
/// Returns `Ok(None)` for a connection that closes or passes the deadline
/// before it sends a whole head, which gets no reply. Returns a refusal for
/// a head over [`HEAD_LIMIT`] bytes or [`HEADER_LIMIT`] headers, and for one
/// that is not HTTP/1.x that the server reads.
pub(super) fn read_request(connection: &mut impl Timed) -> Result<Option<Request>, Refusal> {
    let deadline = Instant::now() + REQUEST_TIME;
    let mut buffer = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let Ok(read) = read_by(connection, deadline, &mut chunk) else {
            return Ok(None);
        };
        let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
            return Ok(None);
        };
        buffer.extend_from_slice(bytes);
        let mut headers = [httparse::EMPTY_HEADER; HEADER_LIMIT];
        let mut parsed = httparse::Request::new(&mut headers);
        match parsed.parse(&buffer) {
            Ok(httparse::Status::Complete(end)) if end <= HEAD_LIMIT => {
                let head = head_of(&parsed)?;
                let early_body = buffer.get(end..).unwrap_or_default().to_vec();
                return Ok(Some(Request {
                    head,
                    early_body,
                    deadline,
                }));
            }
            Ok(httparse::Status::Partial) if buffer.len() <= HEAD_LIMIT => {}
            Ok(_) | Err(httparse::Error::TooManyHeaders) => return Err(Refusal::HeadTooLarge),
            Err(_) => return Err(Refusal::Malformed),
        }
    }
}

/// Returns the head of a parsed request. A request with
/// `Transfer-Encoding`, with a `Content-Length` that is not one decimal
/// number, or with a header value that is not UTF-8, is malformed.
fn head_of(parsed: &httparse::Request<'_, '_>) -> Result<Head, Refusal> {
    let method = match parsed.method {
        Some("GET") => Method::Get,
        Some("POST") => Method::Post,
        _ => Method::Other,
    };
    let path = parsed.path.ok_or(Refusal::Malformed)?.to_owned();
    let mut headers = Vec::with_capacity(parsed.headers.len());
    for header in parsed.headers.iter() {
        let value = std::str::from_utf8(header.value).map_err(|_| Refusal::Malformed)?;
        headers.push((header.name.to_owned(), value.to_owned()));
    }
    let head = Head::new(method, path, headers);
    if head.has("Transfer-Encoding") {
        return Err(Refusal::Malformed);
    }
    let body_length = match head.all("Content-Length").as_slice() {
        [] => None,
        [value] if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) => {
            // A length too large for a u64 is over the body limit too.
            Some(value.parse().unwrap_or(u64::MAX))
        }
        _ => return Err(Refusal::Malformed),
    };
    Ok(head.with_body_length(body_length))
}

/// Reads into `chunk` from `connection`, waiting no later than `deadline`.
fn read_by(connection: &mut impl Timed, deadline: Instant, chunk: &mut [u8]) -> io::Result<usize> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(io::ErrorKind::TimedOut.into());
    }
    connection.set_read_timeout(Some(remaining))?;
    connection.read(chunk)
}

/// Reads and drops what the client still sends after the reply, at most
/// [`DRAIN_LIMIT`] bytes for [`DRAIN_TIME`], so that closing a connection
/// with unread bytes does not reset it before the client reads the reply.
pub(super) fn drain(connection: &mut impl Timed) {
    let deadline = Instant::now() + DRAIN_TIME;
    let mut left = DRAIN_LIMIT;
    let mut chunk = [0; 4096];
    while left > 0 {
        match read_by(connection, deadline, &mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(read) => left = left.saturating_sub(u64::try_from(read).unwrap_or(u64::MAX)),
        }
    }
}

/// The head of a reply: its status, its headers, and the length of its
/// body when the server knows it before it writes the body.
#[derive(Debug)]
pub(super) struct ReplyHead<'a> {
    /// The HTTP status, such as 200.
    pub(super) status: u16,
    /// Every header except `Content-Length` and `Connection`, which the
    /// writer adds.
    pub(super) headers: &'a [(&'a str, &'a str)],
    /// The body's length, or `None` for a body that ends when the server
    /// closes the connection, such as a stream of lines.
    pub(super) length: Option<usize>,
}

/// Writes `head` to `connection` and returns the writer of the body.
///
/// # Errors
///
/// Returns the error of a write, as when the client has closed the
/// connection.
pub(super) fn start<'w, W: Write>(
    connection: &'w mut W,
    head: &ReplyHead<'_>,
) -> io::Result<Body<'w, W>> {
    let mut text = format!("HTTP/1.1 {} {}\r\n", head.status, reason(head.status));
    for (name, value) in head.headers {
        text.push_str(&format!("{name}: {value}\r\n"));
    }
    if let Some(length) = head.length {
        text.push_str(&format!("Content-Length: {length}\r\n"));
    }
    text.push_str("Connection: close\r\n\r\n");
    connection.write_all(text.as_bytes())?;
    connection.flush()?;
    Ok(Body { connection })
}

/// The body of a reply whose head has been written.
#[derive(Debug)]
pub(super) struct Body<'w, W: Write> {
    connection: &'w mut W,
}

impl<W: Write> Body<'_, W> {
    /// Writes `bytes`, such as one line of a stream with its newline, and
    /// flushes it to the client at once.
    ///
    /// # Errors
    ///
    /// Returns the error of a write, as when the client has closed the
    /// connection or has not read for [`WRITE_TIME`].
    pub(super) fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.connection.write_all(bytes)?;
        self.connection.flush()
    }
}

/// Returns the reason phrase of `status`.
const fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}
