//! The clock that the runner reads, which a test replaces (BASE-5).

use std::fmt;
use std::time::Instant;

/// A source of the current instant.
pub trait Clock: Send + Sync + fmt::Debug {
    /// Returns the current instant.
    fn now(&self) -> Instant;
}

/// The system's monotonic clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
