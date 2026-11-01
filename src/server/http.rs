//! HTTP/1.1 on one connection: reading a request's head and body within
//! their limits, and writing a reply's bytes within their deadlines.
//!
//! Every reply carries `Connection: close`, and the server closes the
//! connection after it, so a connection holds one request. A body is read
//! only when the route asks for it, after the request has passed the checks
//! of `request::check`.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use super::reply::Refusal;
use super::request::{Head, Method};

#[cfg(test)]
mod tests;

/// The most bytes of a request's head, its request line and headers, that
/// the server reads.
pub const HEAD_LIMIT: usize = 16 * 1024;

/// The most headers that a request may have.
pub const HEADER_LIMIT: usize = 32;

/// The most bytes of a request body that the server reads.
pub const BODY_LIMIT: usize = 64 * 1024;

/// How long a client has to send a request's head and body together.
pub const REQUEST_TIME: Duration = Duration::from_secs(10);

/// How long the client may take, by default, to read a whole reply of
/// known length, or one line of a stream.
pub const WRITE_TIME: Duration = Duration::from_secs(10);

/// The most bytes that the server reads and drops after a reply, so that a
/// client that is still sending gets the reply before the connection
/// closes.
const DRAIN_LIMIT: usize = 64 * 1024;

/// How long the server reads and drops after a reply.
const DRAIN_TIME: Duration = Duration::from_millis(500);

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

    /// Reads the request's body from `connection`. `request::check` has
    /// refused a declared length over [`BODY_LIMIT`]. A body that does not
    /// arrive whole before the request's deadline is refused with
    /// [`Refusal::Timeout`].
    pub(super) fn body(&self, connection: &mut TcpStream) -> Result<Vec<u8>, Refusal> {
        let length = self.head.body_length().unwrap_or(0);
        let length = usize::try_from(length).map_err(|_| Refusal::TooLarge)?;
        let mut body = self.early_body.clone();
        body.truncate(length);
        let mut chunk = [0; 4096];
        while body.len() < length {
            let read =
                read_by(connection, self.deadline, &mut chunk).map_err(|_| Refusal::Timeout)?;
            let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
                return Err(Refusal::Timeout);
            };
            let wanted = length - body.len();
            body.extend_from_slice(bytes.get(..wanted).unwrap_or(bytes));
        }
        Ok(body)
    }
}

/// Reads a request's head from `connection` within `time`, which also
/// bounds the read of its body.
///
/// The server searches for the end of the head only in the bytes that each
/// read adds, and parses the head once, when it has found the end.
///
/// # Errors
///
/// Returns `Ok(None)` for a connection that closes or passes the deadline
/// before it sends a whole head, which gets no reply. Returns a refusal for
/// a head over [`HEAD_LIMIT`] bytes or [`HEADER_LIMIT`] headers, and for one
/// that is not HTTP/1.x that the server reads.
pub(super) fn read_request(
    connection: &mut TcpStream,
    time: Duration,
) -> Result<Option<Request>, Refusal> {
    let deadline = Instant::now() + time;
    let mut buffer = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let Ok(read) = read_by(connection, deadline, &mut chunk) else {
            return Ok(None);
        };
        let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
            return Ok(None);
        };
        let searched = buffer.len();
        buffer.extend_from_slice(bytes);
        if ends_head(&buffer, searched) {
            let (head, end) = parse_head(&buffer)?;
            let early_body = buffer.get(end..).unwrap_or_default().to_vec();
            return Ok(Some(Request {
                head,
                early_body,
                deadline,
            }));
        }
        if buffer.len() > HEAD_LIMIT {
            return Err(Refusal::HeadTooLarge);
        }
    }
}

/// Returns true when `buffer` holds the empty line that ends a head, a line
/// break followed by `\r\n` or `\n`, in the bytes from `searched` on or in
/// the three bytes before them, where such a line may have started.
fn ends_head(buffer: &[u8], searched: usize) -> bool {
    let tail = buffer.get(searched.saturating_sub(3)..).unwrap_or_default();
    tail.windows(2).any(|pair| pair == b"\n\n") || tail.windows(3).any(|three| three == b"\n\r\n")
}

/// Parses a whole head at the start of `bytes` and returns it with the
/// length of the head in bytes.
///
/// A head over [`HEAD_LIMIT`] bytes or with more than [`HEADER_LIMIT`]
/// headers is refused as too large. A request with `Transfer-Encoding`,
/// with a `Content-Length` that is not one decimal number, with a header
/// value that is not UTF-8, or that is not HTTP/1.x, is malformed.
fn parse_head(bytes: &[u8]) -> Result<(Head, usize), Refusal> {
    let mut headers = [httparse::EMPTY_HEADER; HEADER_LIMIT];
    let mut parsed = httparse::Request::new(&mut headers);
    let end = match parsed.parse(bytes) {
        Ok(httparse::Status::Complete(end)) if end <= HEAD_LIMIT => end,
        Ok(httparse::Status::Complete(_)) | Err(httparse::Error::TooManyHeaders) => {
            return Err(Refusal::HeadTooLarge);
        }
        // A head whose end the server found and that still parses as
        // partial is no head that the server reads.
        Ok(httparse::Status::Partial) | Err(_) => return Err(Refusal::Malformed),
    };
    let method = match parsed.method {
        Some("GET") => Method::Get,
        Some("POST") => Method::Post,
        _ => Method::Other,
    };
    let path = parsed.path.ok_or(Refusal::Malformed)?.to_owned();
    let mut fields = Vec::with_capacity(parsed.headers.len());
    for header in parsed.headers.iter() {
        let value = std::str::from_utf8(header.value).map_err(|_| Refusal::Malformed)?;
        fields.push((header.name.to_owned(), value.to_owned()));
    }
    let head = Head::new(method, path, fields);
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
    Ok((head.with_body_length(body_length), end))
}

/// Reads into `chunk` from `connection`, waiting no later than `deadline`.
fn read_by(connection: &mut TcpStream, deadline: Instant, chunk: &mut [u8]) -> io::Result<usize> {
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
pub(super) fn drain(connection: &mut TcpStream) {
    let deadline = Instant::now() + DRAIN_TIME;
    let mut left = DRAIN_LIMIT;
    let mut chunk = [0; 4096];
    while left > 0 {
        match read_by(connection, deadline, &mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(read) => left = left.saturating_sub(read),
        }
    }
}

/// Reads and drops what the client has already sent, at most
/// [`DRAIN_LIMIT`] bytes, without waiting for more, so that closing the
/// connection does not reset it before the client reads the reply.
pub(super) fn discard_arrived(connection: &mut TcpStream) {
    if connection.set_nonblocking(true).is_err() {
        return;
    }
    let mut left = DRAIN_LIMIT;
    let mut chunk = [0; 4096];
    while left > 0 {
        match connection.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(read) => left = left.saturating_sub(read),
        }
    }
}

/// How long the writes of a reply may wait for the client to read.
#[derive(Clone, Copy, Debug)]
enum Pace {
    /// Every write of the reply, its head and its body, ends by this one
    /// deadline.
    Whole(Instant),
    /// Each line of a stream has this long from when its write starts.
    Line(Duration),
}

/// The connection of a reply, which writes each piece within the reply's
/// deadline.
#[derive(Debug)]
pub(super) struct Body<'w> {
    connection: &'w mut TcpStream,
    pace: Pace,
}

impl<'w> Body<'w> {
    /// Returns the writer of a reply of known length, whose head and body
    /// the client must read within `time` in all.
    pub(super) fn whole(connection: &'w mut TcpStream, time: Duration) -> Self {
        Self {
            connection,
            pace: Pace::Whole(Instant::now() + time),
        }
    }

    /// Returns the writer of a stream, each of whose lines the client must
    /// read within `time`.
    ///
    /// A stream's body ends when the server closes the connection, so its
    /// own last line says that it is whole. The run routes' last line is
    /// the `ended` record, and a stream that closes without it was cut off.
    pub(super) const fn lines(connection: &'w mut TcpStream, time: Duration) -> Self {
        Self {
            connection,
            pace: Pace::Line(time),
        }
    }

    /// Makes every later write end by `deadline` in all, or by the reply's
    /// own deadline when that is earlier. The server cuts its streams off
    /// when it stops, so a client that reads slowly cannot hold the stop
    /// for longer than one write time.
    pub(super) fn cut_off(&mut self, deadline: Instant) {
        self.pace = match self.pace {
            Pace::Whole(own) => Pace::Whole(own.min(deadline)),
            Pace::Line(_) => Pace::Whole(deadline),
        };
    }

    /// Writes `bytes`, such as the head or one line of a stream with its
    /// newline. TCP sends each write at once, so the client receives it
    /// without a flush.
    ///
    /// # Errors
    ///
    /// Returns the error of a write, as when the client has closed the
    /// connection or the reply's deadline has passed.
    pub(super) fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        let deadline = match self.pace {
            Pace::Whole(deadline) => deadline,
            Pace::Line(time) => Instant::now() + time,
        };
        write_by(self.connection, bytes, deadline)
    }
}

/// Writes `bytes` to `connection` by `deadline`. Each write waits at most
/// until the deadline, and the deadline is checked before each one.
fn write_by(connection: &mut TcpStream, mut bytes: &[u8], deadline: Instant) -> io::Result<()> {
    while !bytes.is_empty() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        connection.set_write_timeout(Some(remaining))?;
        match connection.write(bytes) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => bytes = bytes.get(written..).unwrap_or_default(),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// Returns the reason phrase of `status`.
pub(super) const fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}
