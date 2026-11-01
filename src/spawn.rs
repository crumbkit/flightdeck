//! Starts a crumbkit tool as a child process and captures what it prints.
//!
//! This module and its children are the only places that start a process
//! (PROC-1), and the module and its child `stream` are the only places that
//! start a thread (SVC-1). They have three operations. [`open_browser`]
//! opens the opener page in the default browser, and its module states its
//! five items of PROC-2. `stream` runs one command that the runner checked
//! against the tool's model, and its module states its own five items of
//! PROC-2. `describe` runs one of the fixed queries that read a tool's
//! description of itself. Each query of `describe` states these five items
//! of PROC-2:
//!
//! - Program: the name or path that the person gave. A name without a `/`
//!   is found on the `PATH` that flightdeck inherited, and the person who
//!   chose the tool is trusted to supply its bytes.
//! - Arguments: the fixed argument vector of the [`Query`], with no shell.
//!   The ids that `schemas show` takes follow `--`, so an id that starts with
//!   `-` stays an id.
//! - Environment: the child inherits flightdeck's environment, plus the
//!   variables that [`Program::env`] adds.
//! - Streams and deadline: standard input is closed. Standard output is kept
//!   up to [`STDOUT_LIMIT`] bytes, and standard error keeps its last
//!   [`STDERR_LIMIT`] bytes. The query has one deadline, which covers the
//!   wait for the child and every read from its pipes.
//! - End: on Unix, the child runs in a process group of its own. When the
//!   child exits, or when the deadline passes, `describe` kills that whole
//!   group and only then reaps the child. The child stays unreaped until the
//!   kill, so its process id, which is also the group's id, cannot pass to
//!   another process first. Killing the group also stops a descendant that
//!   the child left running, such as a background process that holds the
//!   child's standard output open. Elsewhere, `describe` kills the child
//!   alone when the deadline passes. A query only reads, so a stopped child
//!   leaves no partial work.
//!
//! A descendant that left the child's process group, such as one that called
//! `setsid`, is not stopped (PROC-3). When it holds a pipe open, the reads
//! end at the deadline and the query ends as [`Exit::TimedOut`], but the
//! descendant keeps running.

// The module holds each child it starts, so the type that the baseline keeps
// out of other modules appears here.
#![expect(
    clippy::disallowed_types,
    reason = "spawn owns flightdeck's child processes (PROC-1)"
)]

use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::process::{Child, ExitStatus, Stdio};
use std::thread::Scope;
use std::time::{Duration, Instant};

mod browser;
mod stream;

#[cfg(test)]
mod tests;

pub use self::browser::{BROWSER_TIME_LIMIT, open_browser};
pub use self::stream::{
    DRAIN_GRACE, Escalation, INTERRUPT_GRACE, LINE_LIMIT, Limit, Limits, QUEUE, TERMINATE_GRACE,
    TOTAL_LIMIT,
};
pub(crate) use self::stream::{Ended, Event, Events, Running, Stop, finish, stream};

/// The time limit of each query.
///
/// RUST-10: one fixed limit covers every tool. It handles a tool that hangs
/// or waits for input it will never get, and it misses a tool whose
/// description is slow to build, such as one that asks a network service
/// for its commands. A configured limit replaces it when such a tool needs
/// one.
pub const TIME_LIMIT: Duration = Duration::from_secs(10);

/// The most bytes of standard output that a query keeps. A tool's whole
/// description is far smaller, so a larger output is not a description.
pub const STDOUT_LIMIT: usize = 16 * 1024 * 1024;

/// The most bytes at the end of standard error that a query keeps.
pub const STDERR_LIMIT: usize = 64 * 1024;

/// The size of each read from a child's pipe.
const CHUNK: usize = 8 * 1024;

/// A program to start, by name or path, and the variables that its runs get
/// beside flightdeck's own environment.
#[derive(Clone, Debug)]
pub struct Program {
    program: OsString,
    env: Vec<(OsString, OsString)>,
}

impl Program {
    /// Returns the program with this name or path and no added variables.
    #[must_use]
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            env: Vec::new(),
        }
    }

    /// Returns the program with the variable `name` set to `value` in every
    /// run.
    #[must_use]
    pub fn env(mut self, name: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((name.into(), value.into()));
        self
    }

    /// Returns the name or path that the program was given.
    #[must_use]
    pub fn name(&self) -> &OsStr {
        &self.program
    }
}

/// A query that reads part of a crumbkit tool's description of itself.
#[derive(Clone, Copy, Debug)]
pub enum Query<'a> {
    /// `<tool> --version`, which prints the tool's name and version.
    Version,
    /// `commands list`, with every command, argument, example and column.
    CommandsList,
    /// `schemas list`, with every document and the commands that print it.
    SchemasList,
    /// `schemas show` of these document ids, which crumbkit takes at most
    /// 100 at a time.
    SchemasShow(&'a [String]),
}

impl<'a> Query<'a> {
    /// Returns the query's argument vector after the program.
    #[must_use]
    pub(crate) fn words(&self) -> Vec<&'a str> {
        match self {
            Self::Version => vec!["--version"],
            Self::CommandsList => {
                vec![
                    "commands",
                    "list",
                    "--json",
                    "--full",
                    "--all",
                    "--no-input",
                ]
            }
            Self::SchemasList => {
                vec!["schemas", "list", "--json", "--full", "--all", "--no-input"]
            }
            Self::SchemasShow(ids) => {
                let mut words = vec!["schemas", "show", "--json", "--no-input", "--"];
                words.extend(ids.iter().map(String::as_str));
                words
            }
        }
    }
}

/// How a query's child process ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// The child exited with this code.
    Code(i32),
    /// A signal ended the child before it exited.
    Signal,
    /// The time limit passed, and flightdeck stopped the child.
    TimedOut,
    /// flightdeck could not learn how the child ended, because waiting for it
    /// or reading its output failed.
    Unknown,
}

/// The standard output of a query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stdout {
    /// Every byte the child printed.
    Whole(Vec<u8>),
    /// The child printed more than [`STDOUT_LIMIT`] bytes, which the query
    /// read and dropped.
    OverLimit,
}

/// What a query's child process printed and how it ended.
#[derive(Clone, Debug)]
pub struct Finished {
    exit: Exit,
    stdout: Stdout,
    stderr: Vec<u8>,
}

impl Finished {
    /// Returns how the child ended.
    #[must_use]
    pub const fn exit(&self) -> Exit {
        self.exit
    }

    /// Returns the child's standard output.
    #[must_use]
    pub const fn stdout(&self) -> &Stdout {
        &self.stdout
    }

    /// Returns the last [`STDERR_LIMIT`] bytes of the child's standard error.
    #[must_use]
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }
}

/// A query whose child process could not be started or followed.
#[derive(Debug)]
pub enum SpawnError {
    /// The program does not exist or cannot be started.
    Start(io::Error),
    /// Waiting for the child, or stopping it, failed.
    Wait(io::Error),
    /// Reading the child's standard output or standard error failed.
    Read(io::Error),
}

impl std::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Start(_) => "cannot start the program",
            Self::Wait(_) => "cannot wait for the program",
            Self::Read(_) => "cannot read what the program printed",
        })
    }
}

impl std::error::Error for SpawnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Start(error) | Self::Wait(error) | Self::Read(error) => Some(error),
        }
    }
}

/// Runs `query` against `program`, waits at most `time_limit` for it, and
/// returns what it printed and how it ended.
///
/// # Errors
///
/// Returns [`SpawnError::Start`] when the program does not exist or cannot
/// be started, and the other variants when waiting for it or reading its
/// output fails. The child is stopped and reaped on every path.
pub(crate) fn describe(
    program: &Program,
    query: &Query<'_>,
    time_limit: Duration,
) -> Result<Finished, SpawnError> {
    let deadline = Instant::now() + time_limit;
    let mut child = start(program, &query.words()).map_err(SpawnError::Start)?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    // Each pipe has its own reader, so a child that fills one pipe while
    // flightdeck reads the other never blocks. The scope joins every thread
    // before it returns.
    std::thread::scope(|scope| {
        let stdout = scope.spawn(move || {
            let mut pipe = Timed::new(stdout, deadline);
            read_capped(&mut pipe, STDOUT_LIMIT).map(|read| (read, pipe.expired))
        });
        let stderr = scope.spawn(move || {
            let mut pipe = Timed::new(stderr, deadline);
            read_tail(&mut pipe, STDERR_LIMIT).map(|read| (read, pipe.expired))
        });
        let exit = wait(scope, &mut child, deadline).map_err(SpawnError::Wait);
        let stdout = joined(stdout.join());
        let stderr = joined(stderr.join());
        let exit = exit?;
        let (stdout, stdout_expired) = stdout?;
        let (stderr, stderr_expired) = stderr?;
        Ok(Finished {
            exit: if stdout_expired || stderr_expired {
                Exit::TimedOut
            } else {
                exit
            },
            stdout,
            stderr,
        })
    })
}

/// Starts `program` with `words`, standard input closed and both output
/// streams piped, in a process group of its own on Unix.
fn start(program: &Program, words: &[&str]) -> io::Result<Child> {
    #[expect(
        clippy::disallowed_methods,
        reason = "spawn owns flightdeck's child processes (PROC-1)"
    )]
    let mut command = std::process::Command::new(&program.program);
    command
        .args(words)
        .envs(program.env.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    #[expect(
        clippy::disallowed_methods,
        reason = "spawn owns flightdeck's child processes (PROC-1)"
    )]
    command.spawn()
}

/// Waits for `child` to exit until `deadline`, kills its process group in
/// either case, and then reaps it.
///
/// A thread in `scope` waits for the child to exit without reaping it, so
/// the group's id stays the child's until the kill. That thread ends once
/// the child has exited, which the kill ensures.
#[cfg(unix)]
fn wait<'scope>(
    scope: &'scope Scope<'scope, '_>,
    child: &mut Child,
    deadline: Instant,
) -> io::Result<Exit> {
    use rustix::process::{Pid, Signal, kill_process_group};

    let group = Pid::from_child(child);
    let (exited, notice) = std::sync::mpsc::sync_channel(1);
    let waiter = scope.spawn(move || {
        let waited = wait_unreaped(group);
        // The channel holds the one notice, so the send never blocks. It
        // fails only when `wait` has returned, and then nobody needs it.
        exited.send(()).ok();
        waited
    });
    let timed_out = notice
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .is_err();
    // The kill fails when every process in the group has exited, and that
    // is no failure. A child that moved to another group is killed alone,
    // which cannot fail while the child is unreaped.
    if kill_process_group(group, Signal::KILL).is_err() && timed_out {
        child.kill()?;
    }
    let waited = waiter.join().unwrap_or_else(|_| {
        Err(io::Error::other(
            "the thread that waited for the program stopped",
        ))
    });
    let status = child.wait()?;
    waited?;
    Ok(if timed_out {
        Exit::TimedOut
    } else {
        exit_of(status)
    })
}

/// Waits until the process `pid` exits, and leaves it unreaped.
#[cfg(unix)]
fn wait_unreaped(pid: rustix::process::Pid) -> io::Result<()> {
    use rustix::process::{WaitId, WaitIdOptions, waitid};

    loop {
        match waitid(
            WaitId::Pid(pid),
            WaitIdOptions::EXITED | WaitIdOptions::NOWAIT,
        ) {
            Ok(_) => return Ok(()),
            Err(rustix::io::Errno::INTR) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

/// Waits for `child` until `deadline`, kills it when the deadline passes,
/// and reaps it. Its descendants keep running (PROC-3).
#[cfg(not(unix))]
fn wait<'scope>(
    _scope: &'scope Scope<'scope, '_>,
    child: &mut Child,
    deadline: Instant,
) -> io::Result<Exit> {
    use wait_timeout::ChildExt as _;

    match child.wait_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(Some(status)) => Ok(exit_of(status)),
        Ok(None) => {
            child.kill()?;
            child.wait()?;
            Ok(Exit::TimedOut)
        }
        Err(error) => {
            child.kill().ok();
            child.wait()?;
            Err(error)
        }
    }
}

/// Returns how a child with `status` ended.
fn exit_of(status: ExitStatus) -> Exit {
    status.code().map_or(Exit::Signal, Exit::Code)
}

/// Returns a reader thread's result, or a read error when the thread
/// panicked, which only a broken invariant in the reader can cause.
fn joined<T>(result: std::thread::Result<io::Result<T>>) -> Result<T, SpawnError> {
    match result {
        Ok(read) => read.map_err(SpawnError::Read),
        Err(_) => Err(SpawnError::Read(io::Error::other(
            "the thread that read the program's output stopped",
        ))),
    }
}

/// A child's pipe whose reads end at a deadline. A read after the deadline
/// returns the end of the stream and sets `expired`. A pipe that is absent
/// reads as empty.
struct Timed<R> {
    pipe: Option<R>,
    deadline: Instant,
    expired: bool,
}

impl<R> Timed<R> {
    const fn new(pipe: Option<R>, deadline: Instant) -> Self {
        Self {
            pipe,
            deadline,
            expired: false,
        }
    }
}

#[cfg(unix)]
impl<R: Read + std::os::fd::AsFd> Read for Timed<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use rustix::event::{PollFd, PollFlags, Timespec, poll};

        let Some(pipe) = self.pipe.as_mut() else {
            return Ok(0);
        };
        if self.expired {
            return Ok(0);
        }
        loop {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            let timeout = Timespec::try_from(remaining).map_err(io::Error::other)?;
            let mut fds = [PollFd::new(pipe, PollFlags::IN)];
            match poll(&mut fds, Some(&timeout)) {
                Ok(0) => {
                    self.expired = true;
                    return Ok(0);
                }
                Ok(_) => break,
                Err(rustix::io::Errno::INTR) => {}
                Err(error) => return Err(error.into()),
            }
        }
        pipe.read(buf)
    }
}

/// RUST-10: elsewhere than Unix, a read waits for the pipe without the
/// deadline, so a descendant that holds the pipe open holds the query too.
/// A read that polls the pipe's handle replaces it when flightdeck runs on
/// such a system.
#[cfg(not(unix))]
impl<R: Read> Read for Timed<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.pipe.as_mut() {
            Some(pipe) => pipe.read(buf),
            None => Ok(0),
        }
    }
}

/// Reads `reader` to its end and keeps it whole when it holds at most
/// `limit` bytes. It reads past the limit and drops the rest, so the child
/// never blocks on a full pipe.
pub(crate) fn read_capped(mut reader: impl Read, limit: usize) -> io::Result<Stdout> {
    let mut kept = Vec::new();
    let most = u64::try_from(limit).map_or(u64::MAX, |limit| limit.saturating_add(1));
    (&mut reader).take(most).read_to_end(&mut kept)?;
    if kept.len() <= limit {
        return Ok(Stdout::Whole(kept));
    }
    io::copy(&mut reader, &mut io::sink())?;
    Ok(Stdout::OverLimit)
}

/// Reads `reader` to its end and keeps its last `limit` bytes.
pub(crate) fn read_tail(mut reader: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut kept = Vec::new();
    let mut chunk = [0; CHUNK];
    loop {
        let read = reader.read(&mut chunk)?;
        let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
            break;
        };
        kept.extend_from_slice(bytes);
        if kept.len() > limit.saturating_mul(2).max(CHUNK) {
            kept.drain(..kept.len() - limit);
        }
    }
    if kept.len() > limit {
        kept.drain(..kept.len() - limit);
    }
    Ok(kept)
}
