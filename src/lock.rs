//! The one place where flightdeck takes a lock (BASE-50).

use std::sync::{Condvar, LockResult, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Locks `mutex`, and takes the data of a lock that a panicking thread
/// poisoned.
///
/// flightdeck's locks guard a map of confirmation ids, a list of runs, the
/// queue of each run's lines, the server's session, the server's map of
/// open connections and the runner's last measurement of the help pages,
/// and one lock lets one measurement run at a time. Every change to one of
/// them is one insert, removal, replacement or flag, or a method of the
/// session that keeps it whole, so a panic cannot leave the data
/// half-changed.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    recover(mutex.lock())
}

/// Waits on `changed` with `guard`, and takes the data of a lock that a
/// panicking thread poisoned in the meantime.
pub(crate) fn wait<'a, T>(changed: &Condvar, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
    recover(changed.wait(guard))
}

/// Waits on `changed` with `guard` for at most `most`, as [`wait`] does.
pub(crate) fn wait_for<'a, T>(
    changed: &Condvar,
    guard: MutexGuard<'a, T>,
    most: Duration,
) -> MutexGuard<'a, T> {
    recover(changed.wait_timeout(guard, most)).0
}

/// Returns the data of a lock, poisoned or not.
fn recover<T>(result: LockResult<T>) -> T {
    result.unwrap_or_else(PoisonError::into_inner)
}
