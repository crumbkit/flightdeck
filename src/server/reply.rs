//! What the server sends back: the refusals with their problem documents,
//! and the headers that every reply carries.

use std::io::Cursor;

use serde::Serialize;
use tiny_http::{Header, Response};

use super::StartError;
use super::assets::{self, Asset};

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

/// The content type of a refusal, an RFC 9457 problem document.
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
    /// The body is larger than [`super::BODY_LIMIT`].
    TooLarge,
    /// The body is not the JSON document that the route takes.
    Body,
    /// The request has no token, a wrong one, or a malformed header.
    Unauthorized,
    /// The launch code is wrong, used or expired.
    CodeRefused,
    /// No route or asset has the path.
    NotFound,
}

impl Refusal {
    /// Every refusal, in the order its document lists them.
    pub(super) const ALL: [Self; 10] = [
        Self::Host,
        Self::CrossSite,
        Self::Origin,
        Self::Method,
        Self::ContentType,
        Self::TooLarge,
        Self::Body,
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
            Self::Body => 400,
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
            Self::Body => "flightdeck::request::body",
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
            Self::Body => "The request body is not the JSON document that the route takes.",
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

/// A reply before it gets its headers.
#[derive(Debug)]
pub(super) enum Reply {
    /// A JSON document.
    Json(Vec<u8>),
    /// One of the embedded assets.
    Asset(&'static Asset),
    /// A refusal.
    Refused(Refusal),
}

/// The fixed parts of every reply, parsed once when the server starts, so
/// building a reply cannot fail.
#[derive(Debug)]
pub(super) struct Replies {
    security: Vec<Header>,
    content_types: Vec<(&'static str, Header)>,
    authenticate: Header,
    problems: Vec<(Refusal, Vec<u8>)>,
}

impl Replies {
    /// Parses the fixed headers and writes the problem document of each
    /// refusal.
    pub(super) fn new() -> Result<Self, StartError> {
        let security = SECURITY
            .iter()
            .map(|(name, value)| header(name, value))
            .collect::<Result<_, _>>()?;
        let content_types = [JSON, PROBLEM]
            .into_iter()
            .chain(assets::ALL.iter().map(Asset::content_type))
            .map(|value| header("Content-Type", value).map(|header| (value, header)))
            .collect::<Result<_, _>>()?;
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
            security,
            content_types,
            authenticate: header("WWW-Authenticate", "Bearer")?,
            problems,
        })
    }

    /// Returns the response of `reply`, with the security headers and its
    /// content type.
    pub(super) fn response(&self, reply: Reply) -> Response<Cursor<Vec<u8>>> {
        let (status, content_type, body) = match reply {
            Reply::Json(body) => (200, JSON, body),
            Reply::Asset(asset) => (200, asset.content_type(), asset.body().to_vec()),
            Reply::Refused(refusal) => (refusal.status(), PROBLEM, self.problem(refusal)),
        };
        let mut response = Response::from_data(body).with_status_code(status);
        for header in &self.security {
            response.add_header(header.clone());
        }
        if let Some((_, header)) = self
            .content_types
            .iter()
            .find(|(value, _)| *value == content_type)
        {
            response.add_header(header.clone());
        }
        // A 401 reply names the scheme it needs (RFC 9110).
        if status == 401 {
            response.add_header(self.authenticate.clone());
        }
        response
    }

    /// Returns the problem document of `refusal`.
    fn problem(&self, refusal: Refusal) -> Vec<u8> {
        self.problems
            .iter()
            .find(|(each, _)| *each == refusal)
            .map(|(_, body)| body.clone())
            .unwrap_or_default()
    }
}

/// Parses one fixed header.
fn header(name: &'static str, value: &'static str) -> Result<Header, StartError> {
    Header::from_bytes(name, value).map_err(|()| StartError::Header(name))
}
