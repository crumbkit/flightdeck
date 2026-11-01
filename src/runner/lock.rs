//! The one place where the runner takes a lock (BASE-50).

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Locks `mutex`, and takes the data of a lock that a panicking thread
/// poisoned.
///
/// The runner's locks guard a map of confirmation ids and a list of runs.
/// Every change to either is one insert, removal or replacement, so a panic
/// cannot leave the data half-changed.
pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
