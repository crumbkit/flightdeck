//! Starts a crumbkit tool as a child process and captures what it prints.
//!
//! This module and its children are the only places that start a process
//! (PROC-1). Every thread of flightdeck starts here, where the owner of each
//! run joins it, or in a `std::thread::scope` that joins it, as the
//! server's do (SVC-1). The module has three operations. [`open_browser`]
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
//! - End: crumbkit-process starts the child in a process group of its own
//!   on Unix, or in a job object on Windows. When the child exits, or when
//!   the deadline passes, it kills the whole group or ends the job, and only
//!   then reaps the child, so the child's process id cannot pass to another
//!   process first. Stopping the group also stops a descendant that the
//!   child left running, such as a background process that holds the
//!   child's standard output open. A query only reads, so a stopped child
//!   leaves no partial work.
//!
//! A descendant that left the child's process group, such as one that called
//! `setsid`, is not stopped (PROC-3). When it holds a pipe open, the reads
//! end [`DRAIN_GRACE`] after the child is reaped, and the query ends with
//! the child's own exit and what it printed by then. On Windows a process
//! outside the job that holds a pipe open holds the query until it closes
//! the pipe.

use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crumbkit_process::{Capture, Outcome, OverLimit, RunError, RunOptions, Status};

mod browser;
mod stream;

#[cfg(test)]
mod tests;

pub use self::browser::{BROWSER_TIME_LIMIT, open_browser};
pub use self::stream::{
    DRAIN_GRACE, Escalation, INTERRUPT_GRACE, LINE_LIMIT, Limit, Limits, QUEUE, TERMINATE_GRACE,
    TOTAL_LIMIT,
};
pub(crate) use self::stream::{Ended, Event, Events, Running, Silent, Stop, stream, wait_ended};

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

/// A program to start, by name or path, the variables that its runs get
/// beside flightdeck's own environment, and the directory that its runs
/// start in when that is not flightdeck's own.
#[derive(Clone, Debug)]
pub struct Program {
    program: OsString,
    env: Vec<(OsString, OsString)>,
    directory: Option<PathBuf>,
}

impl Program {
    /// Returns the program with this name or path and no added variables.
    #[must_use]
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
            env: Vec::new(),
            directory: None,
        }
    }

    /// Returns the program given as `program`, for a flightdeck that started
    /// in `launch` with the search path `search`, so that it resolves the
    /// same way in whichever directory a run starts.
    ///
    /// A relative path is made absolute against `launch`, because it would
    /// stop resolving once a child starts in another directory. A name
    /// without a separator stays a name for the `PATH` search, and each
    /// entry of `search` is made absolute against `launch`, because a child's
    /// search reads a relative or empty entry from the child's own directory,
    /// where a chosen directory's `./bin/<tool>` would start instead of the
    /// tool. Without `launch`, because flightdeck's own directory is gone, a
    /// relative path stays as it was given and relative entries are left out,
    /// and the load of the tool then reports a program that it cannot find.
    #[must_use]
    pub fn at_launch(
        program: impl Into<OsString>,
        launch: Option<&Path>,
        search: Option<&OsStr>,
    ) -> Self {
        let program = program.into();
        let path = Path::new(&program);
        let resolved = match launch {
            Some(launch) if path.components().count() > 1 && path.is_relative() => {
                launch.join(path).into_os_string()
            }
            _ => program,
        };
        let launched = Self::new(resolved);
        match search {
            Some(search) => launched.env("PATH", absolute_search(search, launch)),
            None => launched,
        }
    }

    /// Returns the program with the variable `name` set to `value` in every
    /// run.
    #[must_use]
    pub fn env(mut self, name: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((name.into(), value.into()));
        self
    }

    /// Returns the program with every run starting in `directory`, with
    /// `PWD` naming it, so a tool that reads `PWD` sees the directory as the
    /// page named it rather than flightdeck's own.
    #[must_use]
    pub fn current_dir(self, directory: impl Into<PathBuf>) -> Self {
        let directory = directory.into();
        let mut program = self.env("PWD", directory.clone());
        program.directory = Some(directory);
        program
    }

    /// Returns the name or path that the program was given.
    #[must_use]
    pub fn name(&self) -> &OsStr {
        &self.program
    }

    /// Returns the command that runs the program with `words`, its added
    /// variables and its directory. crumbkit-process sets its standard
    /// streams.
    fn command<S: AsRef<OsStr>>(&self, words: &[S]) -> std::process::Command {
        #[expect(
            clippy::disallowed_methods,
            reason = "spawn owns flightdeck's child processes (PROC-1)"
        )]
        let mut command = std::process::Command::new(&self.program);
        command
            .args(words)
            .envs(self.env.iter().map(|(name, value)| (name, value)));
        if let Some(directory) = &self.directory {
            command.current_dir(directory);
        }
        command
    }
}

/// Returns the search path `search` with each entry absolute against
/// `launch`, where an empty entry means `launch` itself. Without `launch` a
/// relative entry is left out.
///
/// RUST-10: an entry whose absolute form holds the search path's separator,
/// as a launch directory with a colon in its name makes, is left out, because
/// the format cannot hold it. It handles every other launch directory, and it
/// misses a tool found only through that entry, which the person can name by
/// its path instead.
fn absolute_search(search: &OsStr, launch: Option<&Path>) -> OsString {
    let entries: Vec<PathBuf> = env::split_paths(search)
        .filter_map(|entry| match launch {
            _ if entry.is_absolute() => Some(entry),
            Some(launch) if entry.as_os_str().is_empty() => Some(launch.to_path_buf()),
            Some(launch) => Some(launch.join(entry)),
            None => None,
        })
        .filter(|entry| env::join_paths([entry]).is_ok())
        .collect();
    // Each entry joined by itself, so together they join too.
    env::join_paths(entries).unwrap_or_default()
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
    /// `self figures list`, with how often each command ran, from the
    /// tool's usage log.
    FiguresList,
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
            Self::FiguresList => {
                vec!["self", "figures", "list", "--json", "--all", "--no-input"]
            }
        }
    }
}

/// How a query's child process ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    /// The child exited with this code.
    Code(i32),
    /// A signal, by its number, ended the child before it exited.
    Signal(i32),
    /// The time limit passed, and flightdeck stopped the child.
    TimedOut,
    /// flightdeck could not learn how the child ended, because it started
    /// no child or waiting for it failed.
    Unknown,
}

impl Exit {
    /// Returns the exit of a child that ended with `status`.
    fn of(status: Option<Status>) -> Self {
        match status {
            Some(Status::Code(code)) => Self::Code(code),
            Some(Status::Signal(signal)) => Self::Signal(signal),
            // RUST-10: `Status` may gain kinds; such a status has no code
            // that flightdeck shows, as an unknown exit has none.
            Some(_) | None => Self::Unknown,
        }
    }
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

/// A child process that could not be started or followed.
#[derive(Debug)]
pub enum SpawnError {
    /// The program does not exist or cannot be started.
    Start(io::Error),
    /// Waiting for the child, reaping it or reading what it printed failed
    /// after it started. The child was stopped and reaped.
    Run(io::Error),
}

impl From<RunError> for SpawnError {
    fn from(error: RunError) -> Self {
        match error {
            RunError::Start(error) => Self::Start(error),
            RunError::Run(error) => Self::Run(error),
            // RUST-10: `RunError` may gain kinds after it started the
            // child, which flightdeck reports as a failure to follow it.
            other => Self::Run(io::Error::other(other)),
        }
    }
}

impl std::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Start(_) => "cannot start the program",
            Self::Run(_) => "cannot follow the program to its end",
        })
    }
}

impl std::error::Error for SpawnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Start(error) | Self::Run(error) => Some(error),
        }
    }
}

/// Runs `query` against `program`, waits at most `time_limit` for it, and
/// returns what it printed and how it ended.
///
/// # Errors
///
/// Returns [`SpawnError::Start`] when the program does not exist or cannot
/// be started, and [`SpawnError::Run`] when waiting for it or reading its
/// output fails. The child is stopped and reaped on every path.
pub(crate) fn describe(
    program: &Program,
    query: &Query<'_>,
    time_limit: Duration,
) -> Result<Finished, SpawnError> {
    let settings = RunOptions::new()
        .stdout(Capture::Whole {
            limit: STDOUT_LIMIT,
            over: OverLimit::Sink,
        })
        .stderr(Capture::Tail {
            limit: STDERR_LIMIT,
        })
        .deadline(Instant::now() + time_limit);
    let finished = crumbkit_process::run(program.command(&query.words()), &settings)?;
    let exit = match finished.outcome() {
        Outcome::TimedOut => Exit::TimedOut,
        _ => Exit::of(finished.status()),
    };
    let (stdout, stderr) = finished.into_output();
    Ok(Finished {
        exit,
        stdout: if stdout.over_limit() {
            Stdout::OverLimit
        } else {
            Stdout::Whole(stdout.into_bytes())
        },
        stderr: stderr.into_bytes(),
    })
}
