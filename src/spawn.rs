//! Starts a crumbkit tool as a child process and captures what it prints.
//!
//! This module is the only place that starts a process (PROC-1). Its one
//! operation, [`describe`], runs one of the fixed queries that read a tool's
//! description of itself. Each query states the five items of PROC-2:
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
//!   [`STDERR_LIMIT`] bytes. The query has one time limit, which covers the
//!   whole run.
//! - End: [`describe`] waits for the child and reads its exit status. When
//!   the time limit passes, it kills the child's process group on Unix and
//!   the child alone elsewhere, then reaps the child. A query only reads, so
//!   a stopped child leaves no partial work. A descendant that left the
//!   process group is not stopped (PROC-3).

// The module holds each child it starts, so the type that the baseline keeps
// out of other modules appears here.
#![expect(
    clippy::disallowed_types,
    reason = "spawn owns flightdeck's child processes (PROC-1)"
)]

use std::ffi::{OsStr, OsString};
use std::io::{self, Read};
use std::process::{Child, ChildStderr, ChildStdout, ExitStatus, Stdio};
use std::time::Duration;

use wait_timeout::ChildExt as _;

#[cfg(test)]
mod tests;

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
    pub fn words(&self) -> Vec<&'a str> {
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
/// output fails.
pub fn describe(
    program: &Program,
    query: &Query<'_>,
    time_limit: Duration,
) -> Result<Finished, SpawnError> {
    let mut child = start(program, &query.words()).map_err(SpawnError::Start)?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    // Each pipe has its own reader, so a child that fills one pipe while
    // flightdeck reads the other never blocks. The scope joins both readers
    // before it returns.
    std::thread::scope(|scope| {
        let stdout = scope.spawn(|| read_stdout(stdout));
        let stderr = scope.spawn(|| read_stderr(stderr));
        let exit = wait(&mut child, time_limit).map_err(SpawnError::Wait)?;
        let stdout = joined(stdout.join())?;
        let stderr = joined(stderr.join())?;
        Ok(Finished {
            exit,
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

/// Waits for `child` until `time_limit` passes, then stops it and reaps it.
fn wait(child: &mut Child, time_limit: Duration) -> io::Result<Exit> {
    if let Some(status) = child.wait_timeout(time_limit)? {
        return Ok(exit_of(status));
    }
    // A child that exits between the timed wait and the kill leaves no
    // process group to kill, and that refusal is no failure.
    if let Err(error) = stop(child)
        && child.try_wait()?.is_none()
    {
        return Err(error);
    }
    child.wait()?;
    Ok(Exit::TimedOut)
}

/// Kills every process in `child`'s process group, which `start` created
/// with the child's id.
#[cfg(unix)]
fn stop(child: &Child) -> io::Result<()> {
    let group = rustix::process::Pid::from_child(child);
    rustix::process::kill_process_group(group, rustix::process::Signal::KILL)?;
    Ok(())
}

/// Kills `child`. Its descendants keep running (PROC-3).
#[cfg(not(unix))]
fn stop(child: &mut Child) -> io::Result<()> {
    child.kill()
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

/// Reads all of standard output, keeping at most [`STDOUT_LIMIT`] bytes. It
/// reads to the end even past the limit, so the child never blocks on a
/// full pipe.
fn read_stdout(pipe: Option<ChildStdout>) -> io::Result<Stdout> {
    let Some(mut pipe) = pipe else {
        return Ok(Stdout::Whole(Vec::new()));
    };
    let mut kept = Vec::new();
    let mut chunk = [0; CHUNK];
    let mut over = false;
    loop {
        let read = pipe.read(&mut chunk)?;
        let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
            break;
        };
        if !over && kept.len() + bytes.len() <= STDOUT_LIMIT {
            kept.extend_from_slice(bytes);
        } else {
            over = true;
            kept = Vec::new();
        }
    }
    Ok(if over {
        Stdout::OverLimit
    } else {
        Stdout::Whole(kept)
    })
}

/// Reads all of standard error, keeping its last [`STDERR_LIMIT`] bytes.
fn read_stderr(pipe: Option<ChildStderr>) -> io::Result<Vec<u8>> {
    let Some(mut pipe) = pipe else {
        return Ok(Vec::new());
    };
    let mut kept = Vec::new();
    let mut chunk = [0; CHUNK];
    loop {
        let read = pipe.read(&mut chunk)?;
        let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
            break;
        };
        kept.extend_from_slice(bytes);
        if kept.len() > STDERR_LIMIT * 2 {
            kept.drain(..kept.len() - STDERR_LIMIT);
        }
    }
    if kept.len() > STDERR_LIMIT {
        kept.drain(..kept.len() - STDERR_LIMIT);
    }
    Ok(kept)
}
