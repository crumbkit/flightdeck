//! The clock that the runner and the server read, which a test replaces
//! (BASE-5).

use std::fmt;
use std::time::{Duration, Instant, SystemTime};

/// The source of the current time, which tests replace to move time on
/// without waiting.
pub trait Clock: Send + Sync + fmt::Debug {
    /// Returns the current time by the monotonic clock and the wall clock.
    fn now(&self) -> Moment;
}

/// The system's monotonic clock and wall clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Moment {
        Moment::new(Instant::now(), SystemTime::now())
    }
}

/// One moment by the monotonic clock and by the wall clock.
///
/// On macOS the monotonic clock stops while the machine sleeps, and the
/// wall clock keeps counting. A time limit that both clocks measure ends
/// when either says that it has passed, so a sleeping machine cannot keep a
/// launch code or an idle session valid for longer than its limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Moment {
    monotonic: Instant,
    wall: SystemTime,
}

impl Moment {
    /// Returns the moment at `monotonic` by the monotonic clock and `wall`
    /// by the wall clock.
    #[must_use]
    pub const fn new(monotonic: Instant, wall: SystemTime) -> Self {
        Self { monotonic, wall }
    }

    /// Returns the moment by the monotonic clock.
    #[must_use]
    pub const fn instant(self) -> Instant {
        self.monotonic
    }

    /// Returns the moment by the wall clock.
    #[must_use]
    pub const fn wall(self) -> SystemTime {
        self.wall
    }

    /// Returns how long has passed from `earlier` to this moment: the
    /// longer of the two clocks' measures. A wall clock set back measures
    /// no time.
    #[must_use]
    pub fn since(self, earlier: Self) -> Duration {
        let monotonic = self.monotonic.saturating_duration_since(earlier.monotonic);
        let wall = self.wall.duration_since(earlier.wall).unwrap_or_default();
        monotonic.max(wall)
    }
}
