//! The runs in progress, which own the threads that follow their children
//! (SVC-1).

use std::sync::Mutex;
use std::sync::mpsc::Receiver;

use super::lock::lock;
use super::{RunFailure, RunId};
use crate::spawn::{Event, Running};

/// The runs of one runner.
#[derive(Debug)]
pub(super) struct Runs {
    most: usize,
    state: Mutex<State>,
}

/// The runs in progress and the id of the next one.
#[derive(Debug)]
struct State {
    last: u64,
    running: Vec<(RunId, Running)>,
}

impl Runs {
    /// Returns no runs, of which at most `most` may run at once.
    pub(super) const fn new(most: usize) -> Self {
        Self {
            most,
            state: Mutex::new(State {
                last: 0,
                running: Vec::new(),
            }),
        }
    }

    /// Joins the runs that have ended, then starts a run with `start` when
    /// fewer than the most are running, and returns its id and events.
    ///
    /// The lock is held while `start` runs, so two starts at once cannot
    /// pass the most between them.
    pub(super) fn start(
        &self,
        start: impl FnOnce() -> Result<(Running, Receiver<Event>), RunFailure>,
    ) -> Result<(RunId, Receiver<Event>), RunFailure> {
        let mut state = lock(&self.state);
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

    /// Stops every run and waits for each to end.
    pub(super) fn shutdown(&self) {
        let running = std::mem::take(&mut lock(&self.state).running);
        for (_, run) in &running {
            run.stop();
        }
        for (_, run) in running {
            run.join();
        }
    }
}
