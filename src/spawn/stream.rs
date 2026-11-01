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
//!   caller stops it with [`Running::stop`]. Once the child has been reaped,
//!   both pipes are read for [`DRAIN_GRACE`] more and then closed.
//! - End: one thread follows the child. On Unix it starts the child in a
//!   process group of its own. A stop sends SIGINT to the group, then
//!   SIGTERM and then SIGKILL after the graces of the [`Escalation`]. When
//!   the child has exited, the thread kills what is left of the group and
//!   then reaps the child, so the group's id cannot pass to another process
//!   in between. A descendant that left the group, such as one that called
//!   `setsid`, is not stopped (PROC-3). When it holds a pipe open, the run
//!   ends after the drain grace without what it prints later. A stopped
//!   command can leave partial work, such as some of the books it
//!   removes.

#![expect(
    clippy::disallowed_types,
    reason = "spawn owns flightdeck's child processes (PROC-1)"
)]

use std::io::{self, BufRead, BufReader, PipeReader, Read as _};
use std::process::{Child, ChildStderr, ChildStdout};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{JoinHandle, Scope, ScopedJoinHandle};
use std::time::{Duration, Instant};

pub use self::drain::DRAIN_GRACE;
use self::drain::Drained;
pub(crate) use self::queue::Events;
use self::queue::Lines;
use super::{Exit, Program, STDERR_LIMIT, SpawnError, exit_of, joined, read_tail, start};
use crate::runner::Argv;

mod drain;
mod queue;

#[cfg(test)]
mod tests;

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

/// How often the thread that follows a child without process groups looks
/// for a stop, while it waits for the child to exit.
#[cfg(not(unix))]
const STOP_CHECK: Duration = Duration::from_millis(50);

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
    End(Result<Ended, SpawnError>),
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
    /// The person cancelled the run.
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

/// A message to the thread that follows a child.
#[derive(Debug)]
enum Control {
    /// Stop the child.
    Stop(Stop),
    /// The child has exited and is not reaped yet.
    #[cfg_attr(
        not(unix),
        expect(dead_code, reason = "only Unix waits for an exit without reaping")
    )]
    Exited,
}

/// Sends a stop to the thread that follows a child, once, and marks it in
/// the queue, which then stops waiting for room.
#[derive(Clone, Debug)]
struct Stopper {
    lines: Lines,
    control: Sender<Control>,
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

    /// Returns true when the run has ended and sent its last event.
    pub(crate) fn is_finished(&self) -> bool {
        self.thread.is_finished()
    }

    /// Waits for the thread that follows the child to end.
    pub(crate) fn join(self) {
        // The thread returns nothing, and a panic in it has already ended
        // the run with a read error, which the queue sends.
        let _ = self.thread.join();
    }
}

/// Lets each of `runs` end by itself within `grace`, then stops those that
/// have not and waits for every run to end (SVC-2).
///
/// A stopped run ends within the graces of its [`Escalation`] and the
/// [`DRAIN_GRACE`], because the reader of a full queue drops lines once the
/// run is stopped and the reads of its pipes end after the drain grace.
pub(crate) fn finish(runs: Vec<Running>, grace: Duration) {
    let deadline = Instant::now() + grace;
    for run in &runs {
        if !run.stopper.lines.wait_ended(deadline) {
            run.stop();
        }
    }
    for run in runs {
        run.join();
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
    let (lines, end, events) = queue::queue(QUEUE);
    let (control, controls) = mpsc::channel();
    let stopper = Stopper {
        lines: lines.clone(),
        control,
    };
    let (started, start_result) = mpsc::channel();
    let program = program.clone();
    let words = argv.words().to_vec();
    let follower = Follower {
        limits,
        escalation,
        stopper: stopper.clone(),
        controls,
    };
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
        Ok(Ok(())) => Ok((running, events)),
        Ok(Err(error)) => {
            running.join();
            Err(SpawnError::Start(error))
        }
        Err(_) => {
            running.join();
            Err(SpawnError::Start(io::Error::other(
                "the thread that starts the program stopped",
            )))
        }
    }
}

/// What the thread that follows one child holds.
struct Follower {
    limits: Limits,
    escalation: Escalation,
    stopper: Stopper,
    controls: Receiver<Control>,
}

impl Follower {
    /// Follows `child` to its end, putting each line of its standard output
    /// on `lines`, and returns how it ended.
    fn follow(self, mut child: Child, lines: &Lines) -> Result<Ended, SpawnError> {
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        std::thread::scope(|scope| {
            // The readers' pipes end a grace after this writer is dropped,
            // which happens once the child is reaped or the readers fail to
            // start, and before the scope joins the readers.
            let readers = io::pipe().and_then(|(reaped, reaped_writer)| {
                let readers =
                    Readers::start(scope, &self, lines, (stdout, stderr), &reaped, &child)?;
                Ok((readers, reaped_writer))
            });
            let (readers, reaped_writer) = match readers {
                Ok(readers) => readers,
                Err(error) => {
                    kill_now(&mut child);
                    return Err(SpawnError::Wait(error));
                }
            };
            let waited = self.wait(&mut child);
            drop(reaped_writer);
            let stdout = joined(readers.stdout.join());
            let stderr = joined(readers.stderr.join());
            #[cfg(unix)]
            let _ = readers.exit.join();
            let (exit, stopped) = waited.map_err(SpawnError::Wait)?;
            stdout?;
            Ok(Ended {
                exit,
                stopped,
                stderr: stderr?,
            })
        })
    }

    /// Waits for `child` to exit, stopping it when a stop arrives, then
    /// kills what is left of its process group and reaps it.
    #[cfg(unix)]
    fn wait(&self, child: &mut Child) -> io::Result<(Exit, Option<Stop>)> {
        // The follower holds a sender of its own, so the receiver never
        // finds every sender gone.
        let stopped = match self.controls.recv() {
            Ok(Control::Stop(stop)) => Some(stop),
            Ok(Control::Exited) | Err(_) => None,
        };
        if stopped.is_some() {
            self.escalate(child);
        }
        // The child is a zombie until `wait` reaps it, so its id still names
        // its process group.
        signal_group(child, rustix::process::Signal::KILL);
        let status = child.wait()?;
        Ok((exit_of(status), stopped))
    }

    /// Sends SIGINT, SIGTERM and SIGKILL to `child`'s process group, each
    /// after the previous one's grace, until the child exits.
    #[cfg(unix)]
    fn escalate(&self, child: &Child) {
        use rustix::process::Signal;

        for (signal, grace) in [
            (Signal::INT, self.escalation.interrupt),
            (Signal::TERM, self.escalation.terminate),
        ] {
            signal_group(child, signal);
            if self.exited_within(grace) {
                return;
            }
        }
        signal_group(child, Signal::KILL);
        while let Ok(Control::Stop(_)) = self.controls.recv() {}
    }

    /// Returns true when the child exits within `grace`, or has exited.
    #[cfg(unix)]
    fn exited_within(&self, grace: Duration) -> bool {
        loop {
            match self.controls.recv_timeout(grace) {
                Ok(Control::Exited) | Err(RecvTimeoutError::Disconnected) => return true,
                Err(RecvTimeoutError::Timeout) => return false,
                // The stopper sends one stop, which `wait` has read, so no
                // other arrives.
                Ok(Control::Stop(_)) => {}
            }
        }
    }

    /// Waits for `child` to exit, killing it when a stop arrives.
    ///
    /// RUST-10: without process groups, a stop kills the child at once with
    /// no SIGINT or SIGTERM first, and its descendants keep running
    /// (PROC-3). The wait looks for a stop every [`STOP_CHECK`]. A job object
    /// with CTRL_BREAK replaces this when flightdeck supports Windows.
    #[cfg(not(unix))]
    fn wait(&self, child: &mut Child) -> io::Result<(Exit, Option<Stop>)> {
        use wait_timeout::ChildExt as _;

        let _ = self.escalation;
        loop {
            if let Some(status) = child.wait_timeout(STOP_CHECK)? {
                return Ok((exit_of(status), None));
            }
            if let Ok(Control::Stop(stop)) = self.controls.try_recv() {
                child.kill()?;
                return Ok((exit_of(child.wait()?), Some(stop)));
            }
        }
    }
}

/// The threads that read a child's streams, and on Unix the thread that
/// waits for its exit.
struct Readers<'scope> {
    stdout: ScopedJoinHandle<'scope, io::Result<()>>,
    stderr: ScopedJoinHandle<'scope, io::Result<Vec<u8>>>,
    #[cfg(unix)]
    exit: ScopedJoinHandle<'scope, ()>,
}

impl<'scope> Readers<'scope> {
    /// Starts the readers of `stdout` and `stderr` in `scope`, and on Unix
    /// the waiter of `child`'s exit.
    #[expect(
        clippy::disallowed_methods,
        reason = "spawn owns flightdeck's threads (SVC-1), and the scope joins these"
    )]
    fn start(
        scope: &'scope Scope<'scope, '_>,
        follower: &Follower,
        lines: &Lines,
        (stdout, stderr): (Option<ChildStdout>, Option<ChildStderr>),
        reaped: &PipeReader,
        child: &Child,
    ) -> io::Result<Self> {
        let limits = follower.limits;
        let lines = lines.clone();
        let stopper = follower.stopper.clone();
        let stdout = stdout.map(|pipe| reaped.try_clone().map(|reaped| Drained::new(pipe, reaped)));
        let stdout = stdout.transpose()?;
        let stderr = stderr.map(|pipe| reaped.try_clone().map(|reaped| Drained::new(pipe, reaped)));
        let stderr = stderr.transpose()?;
        #[cfg(unix)]
        let pid = rustix::process::Pid::from_child(child);
        #[cfg(unix)]
        let exited = follower.stopper.control.clone();
        #[cfg(not(unix))]
        let _ = child;
        let named = |name: &str| std::thread::Builder::new().name(name.to_owned());
        Ok(Self {
            stdout: named("flightdeck stdout")
                .spawn_scoped(scope, move || read_lines(stdout, limits, &lines, &stopper))?,
            stderr: named("flightdeck stderr").spawn_scoped(scope, move || {
                stderr.map_or_else(|| Ok(Vec::new()), |pipe| read_tail(pipe, STDERR_LIMIT))
            })?,
            #[cfg(unix)]
            exit: named("flightdeck exit")
                .spawn_scoped(scope, move || wait_exited(pid, &exited))?,
        })
    }
}

/// Kills `child` and reaps it, for a run whose readers did not start.
fn kill_now(child: &mut Child) {
    #[cfg(unix)]
    signal_group(child, rustix::process::Signal::KILL);
    #[cfg(not(unix))]
    let _ = child.kill();
    let _ = child.wait();
}

/// Sends `signal` to every process in `child`'s process group, which
/// `start` created with the child's id.
///
/// The kernel refuses only when no process of the group is left, and the
/// caller then waits for an exit that has already happened, so a refusal
/// needs no handling.
#[cfg(unix)]
fn signal_group(child: &Child, signal: rustix::process::Signal) {
    let group = rustix::process::Pid::from_child(child);
    let _ = rustix::process::kill_process_group(group, signal);
}

/// Waits until the child `pid` exits, without reaping it, and tells the
/// thread that follows it. A failed wait is told as an exit too, and the
/// follower's own wait then reads the child's status.
#[cfg(unix)]
fn wait_exited(pid: rustix::process::Pid, control: &Sender<Control>) {
    let _ = super::wait_unreaped(pid);
    let _ = control.send(Control::Exited);
}

/// How the delivery of standard output ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Delivered {
    /// Every line arrived.
    All,
    /// The output passed a limit, and the lines after it were not
    /// delivered.
    Over(Limit),
}

/// Reads standard output line by line and sends each line as an event.
/// When the output passes a limit, it stops the child and reads the rest of
/// the pipe without keeping it, so the child never blocks on a full pipe.
fn read_lines(
    pipe: Option<Drained<ChildStdout>>,
    limits: Limits,
    lines: &Lines,
    stopper: &Stopper,
) -> io::Result<()> {
    let Some(pipe) = pipe else {
        return Ok(());
    };
    let mut reader = BufReader::new(pipe);
    // A caller that dropped its events leaves the run going, and the lines
    // after that are dropped.
    let mut listening = true;
    let delivered = deliver(&mut reader, limits, &mut |line| {
        listening = listening && lines.push(line);
    })?;
    if let Delivered::Over(limit) = delivered {
        stopper.stop(Stop::Limit(limit));
        io::copy(&mut reader, &mut io::sink())?;
    }
    Ok(())
}

/// Passes each line of `reader` to `line` until the reader ends or a line
/// or the total passes `limits`.
fn deliver(
    reader: &mut impl BufRead,
    limits: Limits,
    line: &mut impl FnMut(String),
) -> io::Result<Delivered> {
    let most = u64::try_from(limits.line)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    let mut total: usize = 0;
    loop {
        bytes.clear();
        let read = reader.by_ref().take(most).read_until(b'\n', &mut bytes)?;
        if read == 0 {
            return Ok(Delivered::All);
        }
        total = total.saturating_add(read);
        if total > limits.total {
            return Ok(Delivered::Over(Limit::Total));
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        } else if read > limits.line {
            return Ok(Delivered::Over(Limit::Line));
        }
        line(String::from_utf8_lossy(&bytes).into_owned());
    }
}
