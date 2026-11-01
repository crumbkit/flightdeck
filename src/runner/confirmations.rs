//! The confirmation ids that a preview issues, each bound to one vector.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::RunFailure;
use super::failure::ConfirmationProblem as Problem;
use super::lock::lock;
use super::vector::Vector;

#[cfg(test)]
mod tests;

/// The most confirmation ids that wait to be used.
///
/// RUST-10: when a preview would pass this number, the oldest id is
/// dropped. It handles a page that previews many commands and confirms few,
/// and it misses a person who keeps more than this many previews open and
/// confirms the oldest. Sixty-four open previews are more than a page shows.
const MOST: usize = 64;

/// A random id that a preview issues and a confirmation takes, printed as
/// 32 lowercase hex digits.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConfirmationId(u128);

/// Prints the id as 32 lowercase hex digits.
impl fmt::Display for ConfirmationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// Leaves the id out, so a debug print of a preview does not hold it.
impl fmt::Debug for ConfirmationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConfirmationId(..)")
    }
}

/// Reads an id as the page sends it, and refuses any text other than 32
/// lowercase hex digits as an unknown id.
impl FromStr for ConfirmationId {
    type Err = RunFailure;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let unknown = || RunFailure::Confirmation {
            problem: Problem::Unknown,
        };
        let hex = |byte: u8| matches!(byte, b'0'..=b'9' | b'a'..=b'f');
        if text.len() != 32 || !text.bytes().all(hex) {
            return Err(unknown());
        }
        u128::from_str_radix(text, 16)
            .map(Self)
            .map_err(|_| unknown())
    }
}

/// The vector that an id confirms, and when the id was issued.
#[derive(Debug)]
struct Issued {
    vector: Vector,
    at: Instant,
}

/// The ids that wait to be used.
#[derive(Debug)]
pub(super) struct Confirmations {
    life: Duration,
    issued: Mutex<HashMap<ConfirmationId, Issued>>,
}

impl Confirmations {
    /// Returns an empty store whose ids work for `life`.
    pub(super) fn new(life: Duration) -> Self {
        Self {
            life,
            issued: Mutex::new(HashMap::new()),
        }
    }

    /// Issues a new id for `vector` at `now`, after dropping the expired
    /// ids and, at [`MOST`], the oldest one.
    pub(super) fn issue(
        &self,
        vector: Vector,
        now: Instant,
    ) -> Result<ConfirmationId, getrandom::Error> {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes)?;
        let id = ConfirmationId(u128::from_ne_bytes(bytes));
        let mut issued = lock(&self.issued);
        issued.retain(|_, issued| now.saturating_duration_since(issued.at) < self.life);
        if issued.len() >= MOST
            && let Some(oldest) = issued
                .iter()
                .min_by_key(|(_, issued)| issued.at)
                .map(|(id, _)| *id)
        {
            issued.remove(&oldest);
        }
        issued.insert(id, Issued { vector, at: now });
        Ok(id)
    }

    /// Uses up `id`, and returns `Ok` when it was issued for `vector` and
    /// has not expired at `now`.
    pub(super) fn take(
        &self,
        id: &ConfirmationId,
        vector: &Vector,
        now: Instant,
    ) -> Result<(), Problem> {
        let issued = lock(&self.issued).remove(id).ok_or(Problem::Unknown)?;
        if now.saturating_duration_since(issued.at) >= self.life {
            return Err(Problem::Expired);
        }
        if issued.vector != *vector {
            return Err(Problem::Mismatch);
        }
        Ok(())
    }
}
