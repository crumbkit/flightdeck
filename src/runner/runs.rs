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

/// The runs in progress, the starts that hold a place, the id of the last
/// run, and whether the runner has shut down.
#[derive(Debug)]
struct State {
    last: u64,
    running: Vec<(RunId, Running)>,
    starting: usize,
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
                starting: 0,
                closed: false,
            }),
        }
    }

    /// Joins the runs that have ended, then holds a place for a run when the
    /// runner has not shut down and fewer than the most are running or
    /// starting. It starts the run with `start` outside the lock, and
    /// returns its id and events.
    ///
    /// A run that starts while a shutdown takes the runs is stopped, joined
    /// and refused with [`RunFailure::Closed`].
    pub(super) fn start(
        &self,
        start: impl FnOnce() -> Result<(Running, Events), RunFailure>,
    ) -> Result<(RunId, Events), RunFailure> {
        {
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
            if state.running.len().saturating_add(state.starting) >= self.most {
                return Err(RunFailure::Busy { most: self.most });
            }
            state.starting = state.starting.saturating_add(1);
        }
        let started = start();
        let mut state = lock(&self.state);
        state.starting = state.starting.saturating_sub(1);
        let (running, events) = started?;
        if state.closed {
            drop(state);
            running.stop();
            running.join();
            return Err(RunFailure::Closed);
        }
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
    /// then stops the rest and waits for each to end (SVC-2). During the
    /// grace each run stays where [`Runs::cancel`] finds it.
    pub(super) fn shutdown(&self, grace: Duration) {
        let watches: Vec<_> = {
            let mut state = lock(&self.state);
            state.closed = true;
            state.running.iter().map(|(_, run)| run.watch()).collect()
        };
        spawn::wait_ended(&watches, grace);
        let running = std::mem::take(&mut lock(&self.state).running);
        for (_, run) in &running {
            run.stop();
        }
        for (_, run) in running {
            run.join();
        }
    }
}
