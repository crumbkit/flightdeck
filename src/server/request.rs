//! The checks that every request passes before the server routes it. They
//! read only the request line and the headers.

use super::http::BODY_LIMIT;
use super::reply::Refusal;

/// The prefix of every API path.
pub(super) const API: &str = "/api/";

/// The `Host` and `Origin` values of the server's own address.
#[derive(Debug)]
pub(super) struct Address {
    host: String,
    origin: String,
}

impl Address {
    /// Returns the address of a server on 127.0.0.1 at `port`.
    pub(super) fn new(port: u16) -> Self {
        let host = format!("127.0.0.1:{port}");
        Self {
            origin: format!("http://{host}"),
            host,
        }
    }
}

/// A request's method, of those that the server tells apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Method {
    /// `GET`.
    Get,
    /// `POST`.
    Post,
    /// Any other method, which the server refuses.
    Other,
}

/// The parts of a request that the checks and the routes read.
#[derive(Debug)]
pub(super) struct Head {
    method: Method,
    path: String,
    headers: Vec<(String, String)>,
    body_length: Option<u64>,
}

impl Head {
    /// Returns the head with `method`, the request target `target`, whose
    /// path leaves out the query, and `headers`, with no declared body.
    pub(super) fn new(method: Method, target: String, headers: Vec<(String, String)>) -> Self {
        let path = match target.split_once('?') {
            Some((path, _)) => path.to_owned(),
            None => target,
        };
        Self {
            method,
            path,
            headers,
            body_length: None,
        }
    }

    /// Returns the head with the body length that `Content-Length`
    /// declares.
    pub(super) const fn with_body_length(mut self, body_length: Option<u64>) -> Self {
        self.body_length = body_length;
        self
    }

    /// Returns the request's method.
    pub(super) const fn method(&self) -> Method {
        self.method
    }

    /// Returns the request's path, such as `/api/model`.
    pub(super) fn path(&self) -> &str {
        &self.path
    }

    /// Returns the body length that the request declares.
    pub(super) const fn body_length(&self) -> Option<u64> {
        self.body_length
    }

    /// Returns the value of the header `name` when the request has exactly
    /// one, or `None` when it has none or several.
    pub(super) fn one(&self, name: &str) -> Option<&str> {
        match self.all(name).as_slice() {
            [value] => Some(value),
            _ => None,
        }
    }

    /// Returns true when the request has the header `name`.
    pub(super) fn has(&self, name: &str) -> bool {
        !self.all(name).is_empty()
    }

    /// Returns every value of the header `name`, in any case.
    pub(super) fn all(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .collect()
    }
}

/// Refuses a request that the page did not send, before the server reads
/// its body.
///
/// - The `Host` header is exactly `127.0.0.1:<port>`, which a page on
///   another name that resolves to 127.0.0.1 cannot send (DNS rebinding).
/// - The method is `GET` or `POST`, so a CORS preflight never succeeds.
/// - `Sec-Fetch-Site`, when the browser sends it, is `same-origin` or
///   `none`. A navigation to a page path is the exception, because the
///   opener page is a file whose navigation the browser marks
///   `cross-site`, and the page itself holds no secret.
/// - `Origin` is the server's own origin when a request has one, and every
///   `POST` has one.
/// - A `POST` declares its body as `application/json`, which a cross-site
///   form or a request without a preflight cannot send.
/// - A declared body length is at most [`BODY_LIMIT`].
pub(super) fn check(head: &Head, address: &Address) -> Result<(), Refusal> {
    if head.one("Host") != Some(address.host.as_str()) {
        return Err(Refusal::Host);
    }
    let post = match head.method() {
        Method::Get => false,
        Method::Post => true,
        Method::Other => return Err(Refusal::Method),
    };
    if head.has("Sec-Fetch-Site") && !same_origin(head) && !page_navigation(head) {
        return Err(Refusal::CrossSite);
    }
    match head.one("Origin") {
        Some(origin) if origin == address.origin => {}
        None if !post && !head.has("Origin") => {}
        _ => return Err(Refusal::Origin),
    }
    if post && !head.one("Content-Type").is_some_and(is_json) {
        return Err(Refusal::ContentType);
    }
    if head
        .body_length()
        .is_some_and(|length| usize::try_from(length).map_or(true, |length| length > BODY_LIMIT))
    {
        return Err(Refusal::TooLarge);
    }
    Ok(())
}

/// Returns true when the browser says that the page itself sent the
/// request, or that a person typed or chose the address.
fn same_origin(head: &Head) -> bool {
    matches!(head.one("Sec-Fetch-Site"), Some("same-origin" | "none"))
}

/// Returns true for a `GET` that navigates the browser to a path outside
/// the API.
fn page_navigation(head: &Head) -> bool {
    head.method() == Method::Get
        && head.one("Sec-Fetch-Mode") == Some("navigate")
        && !head.path().starts_with(API)
}

/// Returns true when `content_type` is `application/json`, with or without
/// parameters such as `charset=utf-8`.
fn is_json(content_type: &str) -> bool {
    let media_type = content_type
        .split_once(';')
        .map_or(content_type, |(media_type, _)| media_type);
    media_type.trim().eq_ignore_ascii_case("application/json")
}
