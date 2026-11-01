//! The page's two secrets, the launch code and the token, and the one way
//! to compare a presented value with them.

use secrecy::{ExposeSecret as _, SecretString};
use subtle::ConstantTimeEq as _;

/// The number of random bytes in each secret.
const BYTES: usize = 32;

/// A launch code or a token: 32 random bytes from the operating system,
/// written as 64 lowercase hex digits, so that it fits in a URL fragment and
/// in a header without escaping. Its `Debug` prints a placeholder.
#[derive(Debug)]
pub(super) struct Secret {
    text: SecretString,
}

impl Secret {
    /// Returns a new secret from the operating system's random source.
    pub(super) fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0_u8; BYTES];
        getrandom::fill(&mut bytes)?;
        let text: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok(Self { text: text.into() })
    }

    /// Returns true when `presented` is this secret. The comparison takes
    /// the same time for every presented value of the secret's length, so
    /// its timing tells a caller nothing about how much of a guess is right.
    pub(super) fn matches(&self, presented: &str) -> bool {
        self.text
            .expose_secret()
            .as_bytes()
            .ct_eq(presented.as_bytes())
            .into()
    }

    /// Returns the secret, which only the opener page and the token reply
    /// write out.
    pub(super) const fn text(&self) -> &SecretString {
        &self.text
    }
}
