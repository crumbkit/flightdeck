//! The checks that every request passes before the server routes it. They
//! read only the request line and the headers.

use tiny_http::{Header, Method, Request};

use super::BODY_LIMIT;
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

/// The parts of a request that the checks and the routes read.
pub(super) struct Head<'a> {
    method: &'a Method,
    path: &'a str,
    headers: &'a [Header],
    body_length: Option<usize>,
}

impl<'a> Head<'a> {
    /// Returns the head of `request`, whose path leaves out the query.
    pub(super) fn of(request: &'a Request) -> Self {
        let url = request.url();
        Self {
            method: request.method(),
            path: url.split_once('?').map_or(url, |(path, _)| path),
            headers: request.headers(),
            body_length: request.body_length(),
        }
    }

    /// Returns the request's method.
    pub(super) const fn method(&self) -> &Method {
        self.method
    }

    /// Returns the request's path, such as `/api/model`.
    pub(super) const fn path(&self) -> &str {
        self.path
    }

    /// Returns the value of the header `name` when the request has exactly
    /// one, or `None` when it has none or several.
    pub(super) fn one(&self, name: &'static str) -> Option<&'a str> {
        match self.values(name).as_slice() {
            [value] => Some(value),
            _ => None,
        }
    }

    /// Returns true when the request has the header `name`.
    fn has(&self, name: &'static str) -> bool {
        !self.values(name).is_empty()
    }

    /// Returns every value of the header `name`, in any case.
    fn values(&self, name: &'static str) -> Vec<&'a str> {
        self.headers
            .iter()
            .filter(|header| header.field.equiv(name))
            .map(|header| header.value.as_str())
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
pub(super) fn check(head: &Head<'_>, address: &Address) -> Result<(), Refusal> {
    if head.one("Host") != Some(address.host.as_str()) {
        return Err(Refusal::Host);
    }
    let post = match head.method() {
        Method::Get => false,
        Method::Post => true,
        _ => return Err(Refusal::Method),
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
    if head.body_length.is_some_and(|length| length > BODY_LIMIT) {
        return Err(Refusal::TooLarge);
    }
    Ok(())
}

/// Returns true when the browser says that the page itself sent the
/// request, or that a person typed or chose the address.
fn same_origin(head: &Head<'_>) -> bool {
    matches!(head.one("Sec-Fetch-Site"), Some("same-origin" | "none"))
}

/// Returns true for a `GET` that navigates the browser to a path outside
/// the API.
fn page_navigation(head: &Head<'_>) -> bool {
    *head.method() == Method::Get
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
