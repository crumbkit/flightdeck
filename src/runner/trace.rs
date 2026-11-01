//! The trace context that every run of one session carries in
//! `TRACEPARENT`, in the W3C Trace Context format.

use std::sync::atomic::{AtomicU64, Ordering};

/// The session's trace id and the source of each run's parent id.
#[derive(Debug)]
pub(super) struct Trace {
    trace_id: u128,
    first_parent: u64,
    runs: AtomicU64,
}

impl Trace {
    /// Returns a trace with a random trace id and a random first parent id.
    pub(super) fn new() -> Result<Self, getrandom::Error> {
        let mut trace_id = [0; 16];
        let mut first_parent = [0; 8];
        getrandom::fill(&mut trace_id)?;
        getrandom::fill(&mut first_parent)?;
        Ok(Self {
            // W3C refuses an id of all zeros.
            trace_id: u128::from_ne_bytes(trace_id).max(1),
            first_parent: u64::from_ne_bytes(first_parent),
            runs: AtomicU64::new(0),
        })
    }

    /// Returns the `TRACEPARENT` of the next run, such as
    /// `00-<32 hex digits>-<16 hex digits>-01`.
    ///
    /// Each run's parent id is the first parent id plus the number of runs
    /// before it, so no two runs of a session share one. The ids are
    /// unpredictable to another session, which is all that a trace needs.
    pub(super) fn next_parent(&self) -> String {
        let parent = loop {
            let runs = self.runs.fetch_add(1, Ordering::Relaxed);
            let parent = self.first_parent.wrapping_add(runs);
            // W3C refuses a parent id of all zeros.
            if parent != 0 {
                break parent;
            }
        };
        format!("00-{:032x}-{parent:016x}-01", self.trace_id)
    }
}
