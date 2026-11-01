//! The operation that runs one command of a tool for the page, [`stream`].
//!
//! Its five items of PROC-2:
//!
//! - Program: the tool's program, found as `describe` finds it.
//! - Arguments: the vector of a [`crate::runner::Argv`], which only the
//!   runner builds after it checks the command and each value against the
//!   tool's model. There is no shell.
//! - Environment: the child inherits flightdeck's environment, plus the
//!   variables of the [`Program`], which carry the run's `TRACEPARENT`.
//! - Streams and deadline: standard input is closed. Standard output is
//!   read line by line into a queue of at most [`QUEUE`] lines, from which
//!   the caller takes each line as it arrives, and a line or a total past
//!   the [`Limits`] stops the child. Standard error keeps its last
//!   [`super::STDERR_LIMIT`] bytes. A run has no deadline of its own; the
//!   caller stops it with [`Running::stop`], or by dropping its
//!   [`Events`]. Once the child has been reaped, both pipes are read for
//!   [`DRAIN_GRACE`] more and then closed.
//! - End: one thread follows the child, and it starts the child in a
//!   process group of its own. A stop sends SIGINT to the group, then
//!   SIGTERM and then SIGKILL after the graces of the [`Escalation`], and
//!   sends each signal to the child itself too when the child has moved to
//!   another group. When the child has exited, the thread kills what is
//!   left of the group and then reaps the child, so the group's id cannot
//!   pass to another process in between. A descendant that left the group,
//!   such as one that called `setsid`, is not stopped (PROC-3). When it
//!   holds a pipe open, the run ends after the drain grace without what it
//!   prints later. A stopped command can leave partial work, such as some
//!   of the books it removes.

// RUST-10: the stream needs Unix process groups and `poll`. That handles
// Linux and macOS, where flightdeck runs and its lanes build it, and it
// misses Windows. A job object with CTRL_BREAK and a read that waits on the
// pipe's handle replace them when flightdeck supports Windows.
#[cfg(not(unix))]
compile_error!("flightdeck runs a tool's commands only on Unix");

use std::sync::mpsc::{self, SyncSender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub use self::drain::DRAIN_GRACE;
use self::follower::Follower;
use self::queue::Lines;
use super::{Exit, Program, SpawnError, start};
use crate::runner::Argv;

mod drain;
mod follower;
mod queue;
mod readers;

/// The most bytes of one line of standard output that a run delivers.
///
/// RUST-10: one limit covers every tool. It handles a command that prints
/// its whole result as one document, as a large listing with `--all` does,
/// and it misses a tool whose single document is larger. A configured limit
/// replaces it when such a tool needs one.
pub const LINE_LIMIT: usize = 16 * 1024 * 1024;

/// The most bytes of standard output that a run delivers in all.
///
/// RUST-10: one limit covers every run. It keeps a stream that never ends
/// from filling the page's memory, and it ends a long stream that a person
/// wants to read to its end. A configured limit replaces it when a tool
/// needs a longer stream.
pub const TOTAL_LIMIT: usize = 64 * 1024 * 1024;

/// How long a child has after SIGINT before it gets SIGTERM.
pub const INTERRUPT_GRACE: Duration = Duration::from_secs(2);

/// How long a child has after SIGTERM before it gets SIGKILL.
pub const TERMINATE_GRACE: Duration = Duration::from_secs(5);

/// The most lines that wait for the caller to read them. When the caller
/// falls behind, the reader waits, and the child waits on its full pipe.
pub const QUEUE: usize = 256;

/// The most bytes of standard output that a run delivers, per line and in
/// all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    line: usize,
    total: usize,
}

impl Limits {
    /// Returns the limits of `line` bytes for one line and `total` bytes in
    /// all.
    #[must_use]
    pub const fn new(line: usize, total: usize) -> Self {
        Self { line, total }
    }

    /// Returns the most bytes of one line.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    /// Returns the most bytes in all.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.total
    }
}

impl Default for Limits {
    /// Returns [`LINE_LIMIT`] and [`TOTAL_LIMIT`].
    fn default() -> Self {
        Self::new(LINE_LIMIT, TOTAL_LIMIT)
    }
}

/// How long a stopped child has after each signal before the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Escalation {
    interrupt: Duration,
    terminate: Duration,
}

impl Escalation {
    /// Returns the graces of `interrupt` after SIGINT and `terminate` after
    /// SIGTERM.
    #[must_use]
    pub const fn new(interrupt: Duration, terminate: Duration) -> Self {
        Self {
            interrupt,
            terminate,
        }
    }
}

impl Default for Escalation {
    /// Returns [`INTERRUPT_GRACE`] and [`TERMINATE_GRACE`].
    fn default() -> Self {
        Self::new(INTERRUPT_GRACE, TERMINATE_GRACE)
    }
}

/// One event of a run, in the order it happened.
#[derive(Debug)]
pub(crate) enum Event {
    /// One line of standard output, without its newline, read as UTF-8 with
    /// each invalid sequence replaced.
    Line(String),
    /// The run ended. It is the last event.
    End {
        /// How the run ended.
        ended: Result<Ended, SpawnError>,
        /// The run's last line, when the queue was full after a stop and
        /// dropped it, so no line event carried it.
        lost: Option<String>,
    },
}

/// How a run ended.
#[derive(Debug)]
pub(crate) struct Ended {
    exit: Exit,
    stopped: Option<Stop>,
    stderr: Vec<u8>,
}

impl Ended {
    /// Returns how the child ended.
    pub(crate) const fn exit(&self) -> Exit {
        self.exit
    }

    /// Returns why flightdeck stopped the child, or `None` when it exited by
    /// itself.
    pub(crate) const fn stopped(&self) -> Option<Stop> {
        self.stopped
    }

    /// Returns the last bytes of the child's standard error.
    pub(crate) fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

/// Why flightdeck stopped a child.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    /// The person cancelled the run, or the caller dropped its events.
    Cancel,
    /// Standard output passed one of the [`Limits`].
    Limit(Limit),
}

/// Which of the [`Limits`] a run passed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    /// One line was longer than the line limit.
    Line,
    /// The output was longer than the total limit.
    Total,
}

/// A message to the thread that follows a child. Each kind is sent at most
/// once, so a channel of two never makes a sender wait.
#[derive(Debug)]
enum Control {
    /// Stop the child.
    Stop(Stop),
    /// The child has exited and is not reaped yet.
    Exited,
}

/// Sends a stop to the thread that follows a child, once, and marks it in
/// the queue, which then stops waiting for room.
#[derive(Clone, Debug)]
struct Stopper {
    lines: Lines,
    control: SyncSender<Control>,
}

impl Stopper {
    /// Sends `stop`, unless a stop was already sent.
    fn stop(&self, stop: Stop) {
        if self.lines.stop() {
            // The thread drops the receiver only once the child is reaped,
            // and then there is nothing left to stop.
            let _ = self.control.send(Control::Stop(stop));
        }
    }
}

/// A run in progress, which owns the thread that follows its child.
#[derive(Debug)]
pub(crate) struct Running {
    stopper: Stopper,
    thread: JoinHandle<()>,
}

impl Running {
    /// Stops the child, unless it was already stopped.
    pub(crate) fn stop(&self) {
        self.stopper.stop(Stop::Cancel);
    }

    /// Returns the watch that tells when the run has ended.
    pub(crate) fn watch(&self) -> Watch {
        Watch(self.stopper.lines.clone())
    }

    /// Returns true once the run has sent its last event. The flag is set
    /// before the caller can receive that event, so a caller that has read
    /// the end finds the run finished. Its thread may still be returning,
    /// and [`Running::join`] waits for that.
    pub(crate) fn is_finished(&self) -> bool {
        self.stopper.lines.has_ended()
    }

    /// Waits for the thread that follows the child to end.
    pub(crate) fn join(self) {
        // The thread returns nothing, and a panic in it has already ended
        // the run with a read error, which the queue sends.
        let _ = self.thread.join();
    }
}

/// Tells when one run has ended, without owning it.
#[derive(Clone, Debug)]
pub(crate) struct Watch(Lines);

/// Waits until each run of `watches` has ended or `grace` has passed, and
/// returns once every run has ended or the grace is over.
pub(crate) fn wait_ended(watches: &[Watch], grace: Duration) {
    let deadline = Instant::now() + grace;
    for Watch(lines) in watches {
        if !lines.wait_ended(deadline) {
            return;
        }
    }
}

/// The caller's side of a run: each line of standard output as it arrives,
/// then the end. Dropping it stops the run.
#[derive(Debug)]
pub(crate) struct Events {
    receiver: queue::Receiver,
    stopper: Stopper,
}

impl Events {
    /// Waits for the next line or the end, and returns `None` once the end
    /// has been returned.
    pub(crate) fn recv(&self) -> Option<Event> {
        self.receiver.recv()
    }
}

impl Drop for Events {
    /// Stops a run whose caller no longer reads it. After the end the stop
    /// reaches nothing.
    fn drop(&mut self) {
        self.stopper.stop(Stop::Cancel);
    }
}

/// Starts `program` with `argv` and returns the run and its events.
///
/// # Errors
///
/// Returns [`SpawnError::Start`] when the program does not exist or cannot
/// be started, or when flightdeck cannot start the thread that follows it.
pub(crate) fn stream(
    program: &Program,
    argv: &Argv,
    limits: Limits,
    escalation: Escalation,
) -> Result<(Running, Events), SpawnError> {
    let (lines, end, receiver) = queue::queue(QUEUE);
    let (control, controls) = mpsc::sync_channel(2);
    let stopper = Stopper {
        lines: lines.clone(),
        control,
    };
    let (started, start_result) = mpsc::sync_channel(1);
    let program = program.clone();
    let words = argv.words().to_vec();
    let follower = Follower::new(limits, escalation, stopper.clone(), controls);
    #[expect(
        clippy::disallowed_methods,
        reason = "spawn owns flightdeck's threads (SVC-1), and Running joins this one"
    )]
    let thread = std::thread::Builder::new()
        .name("flightdeck run".to_owned())
        .spawn(move || {
            let words: Vec<&str> = words.iter().map(String::as_str).collect();
            match start(&program, &words) {
                Ok(child) => {
                    let _ = started.send(Ok(()));
                    end.send(follower.follow(child, &lines));
                }
                Err(error) => {
                    let _ = started.send(Err(error));
                }
            }
        })
        .map_err(SpawnError::Start)?;
    let running = Running { stopper, thread };
    match start_result.recv() {
        Ok(Ok(())) => {
            let stopper = running.stopper.clone();
            Ok((running, Events { receiver, stopper }))
        }
        Ok(Err(error)) => {
            running.join();
            Err(SpawnError::Start(error))
        }
        Err(_) => {
            running.join();
            Err(SpawnError::Start(std::io::Error::other(
                "the thread that starts the program stopped",
            )))
        }
    }
}
