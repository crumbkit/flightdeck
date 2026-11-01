//! A plain HTTP/1.1 client for the server tests. It writes each request
//! byte by byte over a new connection, so a test controls every header,
//! including `Host`, and reads the whole reply after the server closes the
//! connection.

use std::io::{Read as _, Write as _};
use std::net::TcpStream;

/// One request that a test sends.
pub(crate) struct Call {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Call {
    /// Returns a `GET` of `path` with the `Host` that the server on `port`
    /// expects.
    pub(crate) fn get(port: u16, path: &str) -> Self {
        Self::new("GET", port, path)
    }

    /// Returns a `POST` of `body` to `path` with the `Host`, `Origin` and
    /// `Content-Type` that the server on `port` expects.
    pub(crate) fn post(port: u16, path: &str, body: &str) -> Self {
        let mut call = Self::new("POST", port, path)
            .header("Origin", &format!("http://127.0.0.1:{port}"))
            .header("Content-Type", "application/json");
        call.body = body.as_bytes().to_vec();
        call
    }

    /// Returns a request with the method `method`, such as `OPTIONS`, and
    /// the `Host` that the server on `port` expects.
    pub(crate) fn new(method: &str, port: u16, path: &str) -> Self {
        Self {
            method: method.to_owned(),
            path: path.to_owned(),
            headers: Vec::new(),
            body: Vec::new(),
        }
        .header("Host", &format!("127.0.0.1:{port}"))
    }

    /// Sets the header `name` to `value`, replacing any value it had.
    pub(crate) fn header(self, name: &str, value: &str) -> Self {
        let mut call = self.without(name);
        call.headers.push((name.to_owned(), value.to_owned()));
        call
    }

    /// Sets `Authorization: Bearer <token>`.
    pub(crate) fn bearer(self, token: &str) -> Self {
        self.header("Authorization", &format!("Bearer {token}"))
    }

    /// Removes the header `name`.
    pub(crate) fn without(mut self, name: &str) -> Self {
        self.headers
            .retain(|(header, _)| !header.eq_ignore_ascii_case(name));
        self
    }

    /// Replaces the body with `body`.
    pub(crate) fn body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Sends the request to the server on `port` and returns its reply.
    pub(crate) fn send(self, port: u16) -> Reply {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let mut request = format!("{} {} HTTP/1.1\r\n", self.method, self.path);
        for (name, value) in &self.headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            self.body.len()
        ));
        stream.write_all(request.as_bytes()).unwrap();
        // A server that refuses a request before it reads the body may
        // close the connection while the body is still being written.
        let _ = stream.write_all(&self.body);
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        Reply::parse(&bytes)
    }
}

/// The reply to one request.
#[derive(Debug)]
pub(crate) struct Reply {
    pub(crate) status: u16,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Vec<u8>,
}

impl Reply {
    /// Parses a whole HTTP/1.1 reply, whose body has a `Content-Length` or
    /// arrives in chunks.
    fn parse(bytes: &[u8]) -> Self {
        let end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap_or_else(|| panic!("no end of headers in {:?}", String::from_utf8_lossy(bytes)));
        let head = std::str::from_utf8(&bytes[..end]).unwrap();
        let mut lines = head.split("\r\n");
        let status_line = lines.next().unwrap();
        let status = status_line.split(' ').nth(1).unwrap().parse().unwrap();
        let headers = lines
            .map(|line| {
                let (name, value) = line.split_once(':').unwrap();
                (name.trim().to_owned(), value.trim().to_owned())
            })
            .collect();
        let mut reply = Self {
            status,
            headers,
            body: bytes[end + 4..].to_vec(),
        };
        if reply.header("Transfer-Encoding") == Some("chunked") {
            reply.body = unchunked(&reply.body);
        }
        reply
    }

    /// Returns the value of the header `name`, in any case.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Returns the body as JSON.
    pub(crate) fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&self.body)))
    }

    /// Returns the `code` of a refusal's problem document.
    pub(crate) fn code(&self) -> String {
        self.json()["code"].as_str().unwrap().to_owned()
    }
}

/// Returns the body that `chunked` carries in chunks, each a hex size line
/// followed by that many bytes, until a chunk of size 0.
fn unchunked(mut chunked: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    loop {
        let line_end = chunked.windows(2).position(|pair| pair == b"\r\n").unwrap();
        let size_line = std::str::from_utf8(&chunked[..line_end]).unwrap();
        let size = usize::from_str_radix(size_line.trim(), 16).unwrap();
        if size == 0 {
            return body;
        }
        let start = line_end + 2;
        body.extend_from_slice(&chunked[start..start + size]);
        chunked = &chunked[start + size + 2..];
    }
}

/// Returns the launch code in the opener page `page`, which holds it after
/// `#code=`.
pub(crate) fn launch_code(page: &str) -> String {
    let (_, after) = page.split_once("#code=").unwrap();
    after.chars().take_while(char::is_ascii_hexdigit).collect()
}

/// Exchanges `code` with the server on `port` and returns the token.
pub(crate) fn exchange(port: u16, code: &str) -> String {
    let reply = Call::post(port, "/api/session", &format!(r#"{{"code":"{code}"}}"#)).send(port);
    assert_eq!(
        reply.status,
        200,
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    reply.json()["token"].as_str().unwrap().to_owned()
}
