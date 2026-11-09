//! The page's sign-in: a launch code that the browser exchanges once for the
//! token, and the token that every other API request carries.

use std::time::Duration;

use secrecy::{ExposeSecret as _, SecretString};

use super::reply::Refusal;
use super::secret::Secret;
use crate::clock::Moment;

/// How long a launch code stays valid after the server issues it.
pub const CODE_LIFETIME: Duration = Duration::from_secs(60);

/// The scheme of the `Authorization` header, with the space that follows it.
const BEARER: &str = "Bearer ";

/// The sign-in state of one server: its token, its launch code until the
/// page exchanges it or it expires, the time of the last authenticated
/// request, and whether the idle limit has closed the session.
#[derive(Debug)]
pub struct Session {
    token: Secret,
    code: Option<LaunchCode>,
    last_authenticated: Moment,
    closed: bool,
}

/// A launch code and the time the server issued it.
#[derive(Debug)]
struct LaunchCode {
    secret: Secret,
    issued: Moment,
}

impl Session {
    /// Returns a session whose launch code and token are new random
    /// secrets, with the code issued at `now`.
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system's random source.
    pub fn new(now: Moment) -> Result<Self, getrandom::Error> {
        Ok(Self {
            token: Secret::generate()?,
            code: Some(LaunchCode {
                secret: Secret::generate()?,
                issued: now,
            }),
            last_authenticated: now,
            closed: false,
        })
    }

    /// Returns the launch code that the opener page sends the browser with,
    /// or `None` once the page has exchanged it or it has expired.
    #[must_use]
    pub fn launch_code(&self) -> Option<&SecretString> {
        self.code.as_ref().map(|code| code.secret.text())
    }

    /// Exchanges `presented` for the token at `now`. A code works once and
    /// for [`CODE_LIFETIME`] after it was issued. A wrong code leaves the
    /// real one valid, so a guess cannot lock the page out.
    pub(super) fn exchange(
        &mut self,
        presented: &SecretString,
        now: Moment,
    ) -> Result<&SecretString, Refusal> {
        let Some(code) = &self.code else {
            return Err(Refusal::CodeRefused);
        };
        if now.since(code.issued) > CODE_LIFETIME {
            self.code = None;
            return Err(Refusal::CodeRefused);
        }
        if !code.secret.matches(presented.expose_secret()) {
            return Err(Refusal::CodeRefused);
        }
        self.code = None;
        self.last_authenticated = now;
        Ok(self.token.text())
    }

    /// Accepts the value of a request's one `Authorization` header when it
    /// is `Bearer` and the token, and records `now` as the time of the last
    /// authenticated request.
    ///
    /// Once `idle_limit` has passed since the last authenticated request,
    /// it refuses every token and closes the session, so the server stops
    /// even when this request arrives before the server next checks its
    /// idle time.
    pub(super) fn authenticate(
        &mut self,
        authorization: Option<&str>,
        now: Moment,
        idle_limit: Duration,
    ) -> Result<(), Refusal> {
        if self.close_when_idle(now, idle_limit) {
            return Err(Refusal::Unauthorized);
        }
        let presented = authorization
            .and_then(|value| value.strip_prefix(BEARER))
            .ok_or(Refusal::Unauthorized)?;
        if !self.token.matches(presented) {
            return Err(Refusal::Unauthorized);
        }
        self.last_authenticated = now;
        Ok(())
    }

    /// Returns true once the session has closed: the idle limit passed
    /// before a request, or the server found it passed.
    pub(super) const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Closes the session at `now` once `idle_limit` has passed since the
    /// last authenticated request, counting from the server's start before
    /// the first one, and returns true when it is closed.
    pub(super) fn close_when_idle(&mut self, now: Moment, idle_limit: Duration) -> bool {
        if self.idle_for(now) >= idle_limit {
            self.closed = true;
        }
        self.closed
    }

    /// Returns how long the server has gone without an authenticated
    /// request at `now`, counting from its start before the first one.
    fn idle_for(&self, now: Moment) -> Duration {
        now.since(self.last_authenticated)
    }
}
