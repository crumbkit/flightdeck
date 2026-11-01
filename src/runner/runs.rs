//! The runs in progress, which own the threads that follow their children
//! (SVC-1).

use std::sync::Mutex;
use std::time::Duration;

use super::{RunFailure, RunId};
use crate::lock::lock;
use crate::spawn::{self, Events, Running};

/// The runs of one runner.
#[derive(Debug)]
pub(super) struct Runs {
    most: usize,
    state: Mutex<State>,
}

/// The runs in progress, the id of the next one, and whether the runner
/// has shut down.
#[derive(Debug)]
struct State {
    last: u64,
    running: Vec<(RunId, Running)>,
    closed: bool,
}

impl Runs {
    /// Returns no runs, of which at most `most` may run at once.
    pub(super) const fn new(most: usize) -> Self {
        Self {
            most,
            state: Mutex::new(State {
                last: 0,
                running: Vec::new(),
                closed: false,
            }),
        }
    }

    /// Joins the runs that have ended, then starts a run with `start` when
    /// the runner has not shut down and fewer than the most are running,
    /// and returns its id and events.
    ///
    /// The lock is held while `start` runs, so two starts at once cannot
    /// pass the most between them.
    pub(super) fn start(
        &self,
        start: impl FnOnce() -> Result<(Running, Events), RunFailure>,
    ) -> Result<(RunId, Events), RunFailure> {
        let mut state = lock(&self.state);
        if state.closed {
            return Err(RunFailure::Closed);
        }
        let (ended, running): (Vec<_>, Vec<_>) = std::mem::take(&mut state.running)
            .into_iter()
            .partition(|(_, run)| run.is_finished());
        state.running = running;
        for (_, run) in ended {
            run.join();
        }
        if state.running.len() >= self.most {
            return Err(RunFailure::Busy { most: self.most });
        }
        let (running, events) = start()?;
        state.last = state.last.saturating_add(1);
        let id = RunId(state.last);
        state.running.push((id, running));
        Ok((id, events))
    }

    /// Stops the run `id`.
    pub(super) fn cancel(&self, id: RunId) -> Result<(), RunFailure> {
        let state = lock(&self.state);
        match state.running.iter().find(|(own, _)| *own == id) {
            Some((_, run)) if !run.is_finished() => {
                run.stop();
                Ok(())
            }
            _ => Err(RunFailure::NotRunning { run: Some(id) }),
        }
    }

    /// Refuses every new run, lets the runs in progress end within `grace`,
    /// then stops the rest and waits for each to end (SVC-2).
    pub(super) fn shutdown(&self, grace: Duration) {
        let running = {
            let mut state = lock(&self.state);
            state.closed = true;
            std::mem::take(&mut state.running)
        };
        spawn::finish(running.into_iter().map(|(_, run)| run).collect(), grace);
    }
}
