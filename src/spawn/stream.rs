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
//! - End: one thread follows the child through crumbkit-process, which
//!   starts it in a process group of its own on Unix, or in a job object on
//!   Windows. On Unix a stop sends SIGINT to the group, then SIGTERM and
//!   then SIGKILL after the graces of the [`Escalation`], and signals the
//!   child itself when the child has moved to another group. On Windows a
//!   stop ends the job at once. When the child has exited, crumbkit-process
//!   kills what is left of the group and then reaps the child, so the
//!   group's id cannot pass to another process in between. A descendant
//!   that left the group, such as one that called `setsid`, is not stopped
//!   (PROC-3). When it holds a pipe open, the run ends after the drain grace
//!   without what it prints later. A stopped command can leave partial
//!   work, such as some of the books it removes.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crumbkit_process::{Cancel, Capture, Lines, OverLimit, Run, Signal};

use std::ffi::OsStr;

use super::{Exit, Program, STDERR_LIMIT, SpawnError};
use crate::lock::{lock, wait, wait_for};
use crate::runner::Argv;

pub use crumbkit_process::DRAIN_GRACE;

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

/// The end of a run, which the thread that follows the child sets once the
/// run has returned, and which the caller takes after the last line.
#[derive(Debug, Default)]
struct Slot {
    state: Mutex<SlotState>,
    changed: Condvar,
}

/// Whether the run has ended, and its end until the caller takes it.
#[derive(Debug, Default)]
struct SlotState {
    ended: bool,
    end: Option<Result<Ended, SpawnError>>,
}

impl Slot {
    /// Sets the run's end, unless it was already set.
    fn set(&self, end: Result<Ended, SpawnError>) {
        let mut state = lock(&self.state);
        if !state.ended {
            state.ended = true;
            state.end = Some(end);
            self.changed.notify_all();
        }
    }

    /// Returns true once the run's end has been set.
    fn has_ended(&self) -> bool {
        lock(&self.state).ended
    }

    /// Waits until the end has been set, or until `deadline` when there is
    /// one, and returns whether it has been set.
    fn wait_ended(&self, deadline: Option<Instant>) -> bool {
        let mut state = lock(&self.state);
        while !state.ended {
            match deadline {
                None => state = wait(&self.changed, state),
                Some(deadline) => {
                    let left = deadline.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return false;
                    }
                    state = wait_for(&self.changed, state, left);
                }
            }
        }
        true
    }

    /// Takes the end once it has been set, or returns `None` when the
    /// caller has already taken it.
    fn take(&self) -> Option<Result<Ended, SpawnError>> {
        lock(&self.state).end.take()
    }
}

/// Sets the run's end as a read error when the thread that follows the
/// child stops before it sets one, which only a broken invariant in
/// crumbkit-process can cause.
struct Unset<'a>(&'a Slot);

impl Drop for Unset<'_> {
    fn drop(&mut self) {
        self.0.set(Err(SpawnError::Run(std::io::Error::other(
            "the thread that followed the program stopped",
        ))));
    }
}

/// A run in progress, which owns the thread that follows its child.
#[derive(Debug)]
pub(crate) struct Running {
    cancel: Cancel,
    slot: Arc<Slot>,
    thread: JoinHandle<()>,
}

impl Running {
    /// Stops the child, unless it was already stopped.
    pub(crate) fn stop(&self) {
        self.cancel.cancel();
    }

    /// Returns the watch that tells when the run has ended.
    pub(crate) fn watch(&self) -> Watch {
        Watch(Arc::clone(&self.slot))
    }

    /// Returns true once the run has sent its last event. The flag is set
    /// before the caller can receive that event, so a caller that has read
    /// the end finds the run finished. Its thread may still be returning,
    /// and [`Running::join`] waits for that.
    pub(crate) fn is_finished(&self) -> bool {
        self.slot.has_ended()
    }

    /// Waits for the thread that follows the child to end.
    pub(crate) fn join(self) {
        // The thread returns nothing, and a panic in it has already ended
        // the run with a read error, which `Unset` sets.
        let _ = self.thread.join();
    }
}

/// Tells when one run has ended, without owning it.
#[derive(Clone, Debug)]
pub(crate) struct Watch(Arc<Slot>);

/// Waits until each run of `watches` has ended or `grace` has passed, and
/// returns once every run has ended or the grace is over.
pub(crate) fn wait_ended(watches: &[Watch], grace: Duration) {
    let deadline = Instant::now() + grace;
    for Watch(slot) in watches {
        if !slot.wait_ended(Some(deadline)) {
            return;
        }
    }
}

/// The caller's side of a run: each line of standard output as it arrives,
/// then the end. Dropping it stops the run.
#[derive(Debug)]
pub(crate) struct Events {
    lines: Lines,
    slot: Arc<Slot>,
    cancel: Cancel,
}

/// No line and no end arrived within the time that a caller waited.
#[derive(Debug)]
pub(crate) struct Silent;

impl Events {
    /// Waits for the next line or the end, and returns `None` once the end
    /// has been returned.
    pub(crate) fn recv(&self) -> Option<Event> {
        match self.lines.recv() {
            Some(line) => Some(Event::Line(text(line))),
            None => {
                self.slot.wait_ended(None);
                self.end()
            }
        }
    }

    /// Waits at most `timeout` for the next line or the end, as
    /// [`Events::recv`] does, and returns [`Silent`] when neither arrived.
    pub(crate) fn recv_timeout(&self, timeout: Duration) -> Result<Option<Event>, Silent> {
        let deadline = Instant::now() + timeout;
        match self.lines.recv_timeout(timeout) {
            Ok(Some(line)) => Ok(Some(Event::Line(text(line)))),
            Ok(None) if self.slot.wait_ended(Some(deadline)) => Ok(self.end()),
            Ok(None) | Err(crumbkit_process::Silent) => Err(Silent),
        }
    }

    /// Returns the run's end once, with the line that a full queue dropped
    /// after a stop, or `None` once the end has been returned.
    fn end(&self) -> Option<Event> {
        let ended = self.slot.take()?;
        Some(Event::End {
            ended,
            lost: self.lines.lost().map(text),
        })
    }
}

impl Drop for Events {
    /// Stops a run whose caller no longer reads it. After the end the stop
    /// reaches nothing.
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Returns `line` as UTF-8, with each invalid sequence replaced.
fn text(line: Vec<u8>) -> String {
    String::from_utf8(line)
        .unwrap_or_else(|invalid| String::from_utf8_lossy(invalid.as_bytes()).into_owned())
}

/// Returns how a run that crumbkit-process followed to its end ended.
fn ended(finished: crumbkit_process::Finished) -> Ended {
    let stopped = match finished.exit() {
        crumbkit_process::Exit::Stopped(crumbkit_process::Stop::Cancelled) => Some(Stop::Cancel),
        crumbkit_process::Exit::Stopped(crumbkit_process::Stop::OverLimit) => {
            Some(Stop::Limit(if finished.stdout().long_line() {
                Limit::Line
            } else {
                Limit::Total
            }))
        }
        // RUST-10: `Stop` may gain reasons, and flightdeck sets no other
        // stop than its cancel and its limits, so another reason reads as a
        // cancel.
        crumbkit_process::Exit::Stopped(_) => Some(Stop::Cancel),
        _ => None,
    };
    let exit = Exit::of(finished.status());
    let (_, stderr) = finished.into_output();
    Ended {
        exit,
        stopped,
        stderr: stderr.into_bytes(),
    }
}

/// Starts `program` with `argv` and returns the run and its events.
///
/// # Errors
///
/// Returns [`SpawnError::Start`] when a program given as a path does not
/// exist or cannot be run, or when flightdeck cannot start the thread that
/// follows it. A program that fails to start for another reason ends the
/// run with [`SpawnError::Start`] as its end.
pub(crate) fn stream(
    program: &Program,
    argv: &Argv,
    limits: Limits,
    escalation: Escalation,
) -> Result<(Running, Events), SpawnError> {
    runnable(program.name()).map_err(SpawnError::Start)?;
    let (queue, lines) = crumbkit_process::line_queue(QUEUE);
    let cancel = Cancel::new();
    let slot = Arc::new(Slot::default());
    let settings = Run::new()
        .stdout(Capture::Lines {
            queue,
            line: limits.line,
            total: limits.total,
            over: OverLimit::Stop,
        })
        .stderr(Capture::Tail {
            limit: STDERR_LIMIT,
        })
        .cancel(&cancel)
        .escalation(
            crumbkit_process::Escalation::default()
                .step(Signal::Interrupt, escalation.interrupt)
                .step(Signal::Terminate, escalation.terminate),
        );
    let command = program.command(argv.words());
    let end = Arc::clone(&slot);
    #[expect(
        clippy::disallowed_methods,
        reason = "spawn owns flightdeck's threads (SVC-1), and Running joins this one"
    )]
    let thread = std::thread::Builder::new()
        .name("flightdeck run".to_owned())
        .spawn(move || {
            let unset = Unset(&end);
            let finished = crumbkit_process::run(command, &settings);
            end.set(finished.map(ended).map_err(SpawnError::from));
            drop(unset);
        })
        .map_err(SpawnError::Start)?;
    let running = Running {
        cancel: cancel.clone(),
        slot: Arc::clone(&slot),
        thread,
    };
    Ok((
        running,
        Events {
            lines,
            slot,
            cancel,
        },
    ))
}

/// Refuses a program given as a path that is not a file that may be run,
/// so a run of it is refused as it starts.
///
/// RUST-10: crumbkit-process starts the child on the thread that follows
/// it, so `stream` cannot learn that the start failed before it returns.
/// The check handles a program given as a path, which is how flightdeck's
/// person names a tool that is not on `PATH`. It misses a name that `PATH`
/// does not find and a file that the system cannot run; those end the run
/// with [`SpawnError::Start`] instead. A start notice from crumbkit-process
/// replaces it once the crate has one.
fn runnable(program: &OsStr) -> std::io::Result<()> {
    let path = std::path::Path::new(program);
    if path.components().count() < 2 {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        }
    }
    // Windows has no bit that marks a file to run, and it starts a path
    // without an extension as that path with `.exe`.
    #[cfg(not(unix))]
    if !path.is_file() && (path.extension().is_some() || !path.with_extension("exe").is_file()) {
        return Err(std::io::Error::from(std::io::ErrorKind::NotFound));
    }
    Ok(())
}
