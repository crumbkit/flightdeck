//! The bounded queue that carries a run's lines and its end from the
//! threads that follow the child to the caller.
//!
//! A full queue makes the reader of standard output wait, and the child
//! then waits on its full pipe. Once a stop has been sent, the reader drops
//! a line that finds the queue full, so a caller that stopped reading
//! cannot hold up a cancel or a shutdown. The end never waits for room.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use super::{Ended, Event};
use crate::lock::{lock, wait, wait_for};
use crate::spawn::SpawnError;

/// What the threads and the caller share.
#[derive(Debug)]
struct Shared {
    most: usize,
    state: Mutex<State>,
    changed: Condvar,
}

/// The queue's lines and what has happened to the run.
#[derive(Debug, Default)]
struct State {
    lines: VecDeque<String>,
    end: Option<Result<Ended, SpawnError>>,
    ended: bool,
    stopped: bool,
    dropped: bool,
}

/// Returns a queue of at most `most` lines: the side that adds lines, the
/// side that sends the end, and the caller's side.
pub(super) fn queue(most: usize) -> (Lines, End, Events) {
    let shared = Arc::new(Shared {
        most,
        state: Mutex::new(State::default()),
        changed: Condvar::new(),
    });
    (
        Lines(Arc::clone(&shared)),
        End(Arc::clone(&shared)),
        Events(shared),
    )
}

/// The side of the queue that adds lines, marks the stop and waits for the
/// end.
#[derive(Clone, Debug)]
pub(super) struct Lines(Arc<Shared>);

impl Lines {
    /// Adds `line`, and waits while the queue is full and no stop has been
    /// sent. A full queue after a stop drops the line. Returns false when
    /// the caller has dropped its side, and then nobody reads the line.
    pub(super) fn push(&self, line: String) -> bool {
        let shared = &*self.0;
        let mut state = lock(&shared.state);
        while state.lines.len() >= shared.most && !state.stopped && !state.dropped {
            state = wait(&shared.changed, state);
        }
        if state.dropped {
            return false;
        }
        if state.lines.len() < shared.most {
            state.lines.push_back(line);
            shared.changed.notify_all();
        }
        true
    }

    /// Marks the stop, and returns true when no stop was marked before.
    pub(super) fn stop(&self) -> bool {
        let mut state = lock(&self.0.state);
        let first = !state.stopped;
        state.stopped = true;
        self.0.changed.notify_all();
        first
    }

    /// Waits until the end has been sent or `deadline` passes, and returns
    /// true when the end has been sent.
    pub(super) fn wait_ended(&self, deadline: Instant) -> bool {
        let shared = &*self.0;
        let mut state = lock(&shared.state);
        while !state.ended {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return false;
            }
            state = wait_for(&shared.changed, state, left);
        }
        true
    }
}

/// The side of the queue that sends the end. When it is dropped without a
/// send, as when the thread that holds it panics, the end is a read error.
#[derive(Debug)]
pub(super) struct End(Arc<Shared>);

impl End {
    /// Sends how the run ended.
    pub(super) fn send(self, ended: Result<Ended, SpawnError>) {
        self.set(ended);
    }

    /// Sets the end unless it is set already.
    fn set(&self, ended: Result<Ended, SpawnError>) {
        let mut state = lock(&self.0.state);
        if !state.ended {
            state.end = Some(ended);
            state.ended = true;
            self.0.changed.notify_all();
        }
    }
}

impl Drop for End {
    fn drop(&mut self) {
        self.set(Err(SpawnError::Read(io::Error::other(
            "the thread that followed the program stopped",
        ))));
    }
}

/// The caller's side of the queue, which yields each line and then the end.
/// Dropping it tells the reader that nobody reads the lines.
#[derive(Debug)]
pub(crate) struct Events(Arc<Shared>);

impl Events {
    /// Waits for the next line or the end, and returns `None` once the end
    /// has been returned.
    pub(crate) fn recv(&self) -> Option<Event> {
        let shared = &*self.0;
        let mut state = lock(&shared.state);
        while state.lines.is_empty() && !state.ended {
            state = wait(&shared.changed, state);
        }
        if let Some(line) = state.lines.pop_front() {
            shared.changed.notify_all();
            return Some(Event::Line(line));
        }
        state.end.take().map(Event::End)
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        lock(&self.0.state).dropped = true;
        self.0.changed.notify_all();
    }
}
