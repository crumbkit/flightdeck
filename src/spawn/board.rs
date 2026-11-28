//! The operation that reads a board: it runs `board query` with one of two
//! fixed SQL statements and `board log` of one path, each of which Board
//! declares read-only, and captures the JSON document each prints. It
//! states the five items of PROC-2:
//!
//! - Program: `FLIGHTDECK_BOARD_PROGRAM` when it is set, or else `board`
//!   found on the `PATH` that flightdeck inherited. Inside a plugin call it
//!   is the `BOARD_BIN` that Board gives the plugin. The person who
//!   installed Board or set the variable is trusted to supply its bytes.
//! - Arguments: the fixed argument vector of each [`BoardQuery`], with no
//!   shell, after the board it reads: `-C <dir>` when `FLIGHTDECK_BOARD`
//!   names a directory or a plugin call names the board's root, or else
//!   `--personal`. A query's SQL is one of two fixed statements. The only
//!   value inside one is a [`RecordType`], which admits only lowercase
//!   letters, digits, `-` and two `/`, so no quote reaches the SQL. A path
//!   that `board log` reads is a [`BoardPath`], which starts with `/`, so
//!   Board never reads it as a flag, and a commit is a [`Commit`] of 40 hex
//!   digits.
//! - Environment: the child inherits flightdeck's environment, plus the
//!   variables that [`Program::env`] adds.
//! - Streams and deadline: standard input is closed. Standard output is kept
//!   up to the limit of [`BoardLimits`], [`BOARD_STDOUT_LIMIT`] unless a
//!   test lowers it, and standard error keeps its last [`STDERR_LIMIT`]
//!   bytes. Every query of one call shares one deadline, [`BOARD_TIME_LIMIT`]
//!   after the call starts, which covers the wait for each child and every
//!   read from its pipes. At most [`BOARD_AT_ONCE`] queries run at a time,
//!   each on a thread of a `std::thread::scope` that joins it, so polls
//!   hold Board's index only lightly while agents write. The caller's stop
//!   starts no further query and cancels the ones in progress.
//! - End: as for `describe`, crumbkit-process starts each child in a process
//!   group of its own on Unix, or in a job object on Windows, and stops the
//!   whole group or ends the job when the child exits, its deadline passes
//!   or the stop cancels it, and only then reaps it. A query only reads, so
//!   a stopped child leaves no partial work.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crumbkit_process::{Cancel, Capture, Outcome, OverLimit, RunOptions};

use super::{Exit, Finished, Program, STDERR_LIMIT, SpawnError, Stdout, at_most};

#[cfg(unix)]
#[cfg(test)]
mod tests;

/// The time within which every query of one call ends.
pub const BOARD_TIME_LIMIT: Duration = Duration::from_secs(10);

/// The most bytes of standard output that a query keeps. Every change
/// record of a board of a thousand changes prints about 3 MiB.
pub const BOARD_STDOUT_LIMIT: usize = 16 * 1024 * 1024;

/// The most queries that run at a time.
pub const BOARD_AT_ONCE: usize = 2;

/// The path under which the workflow graph records live.
pub const GRAPHS_UNDER: &str = "/workflow-graphs/";

/// A record type, such as `changes/change/v2`: two words of lowercase
/// letters, digits and `-`, then `v` and digits, joined by `/`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecordType(String);

impl RecordType {
    /// Returns the type `text`, or `None` when it is not a record type.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        let mut parts = text.split('/');
        let word = |part: Option<&str>| {
            part.is_some_and(|part| {
                !part.is_empty()
                    && part.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
            })
        };
        let version = |part: Option<&str>| {
            part.and_then(|part| part.strip_prefix('v'))
                .is_some_and(|digits| {
                    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
                })
        };
        (word(parts.next())
            && word(parts.next())
            && version(parts.next())
            && parts.next().is_none())
        .then(|| Self(text.to_owned()))
    }

    /// Returns the type as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A board's logical path, such as `/flows/development`: it starts with `/`
/// and holds no control character.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardPath(String);

impl BoardPath {
    /// Returns the path `text`, or `None` when it is not a logical path.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        (text.starts_with('/') && !text.chars().any(char::is_control))
            .then(|| Self(text.to_owned()))
    }

    /// Returns the path as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A full commit id: 40 lowercase hex digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit(String);

impl Commit {
    /// Returns the commit `text`, or `None` when it is not 40 lowercase hex
    /// digits.
    #[must_use]
    pub fn new(text: &str) -> Option<Self> {
        (text.len() == 40
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        .then(|| Self(text.to_owned()))
    }
}

/// Which board the queries read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardAt {
    /// The person's own board, which `board --personal` opens.
    Personal,
    /// The board at or above this directory, which `board -C <dir>` opens.
    Directory(PathBuf),
}

impl BoardAt {
    /// Returns the words that choose the board, before Board's command.
    fn words(&self) -> Vec<OsString> {
        match self {
            Self::Personal => vec!["--personal".into()],
            Self::Directory(directory) => vec!["-C".into(), directory.clone().into_os_string()],
        }
    }
}

/// One of the read-only commands of Board that flightdeck runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardQuery {
    /// `board query` of every record under [`GRAPHS_UNDER`], with its path,
    /// status, code and body.
    Graphs,
    /// `board query` of every record of one type, with its path, status,
    /// code and body.
    OfType(RecordType),
    /// `board log` of one path, newest first: every commit with `Some` of
    /// no limit, or the newest one only, starting at the commit given or
    /// at the board's head.
    Log {
        /// The path whose history to read.
        path: BoardPath,
        /// The commit to start the walk at, or the head.
        at: Option<Commit>,
        /// Whether to read every commit, or the newest only.
        every: bool,
    },
}

impl BoardQuery {
    /// Returns the query's argument vector after the board's words.
    #[must_use]
    pub fn words(&self) -> Vec<String> {
        let mut words = match self {
            Self::Graphs => vec![
                "query".to_owned(),
                format!(
                    "SELECT logical, type, status, code, body FROM records WHERE substr(logical, 1, {}) = '{GRAPHS_UNDER}' ORDER BY logical",
                    GRAPHS_UNDER.len()
                ),
                "--all".to_owned(),
            ],
            Self::OfType(record_type) => vec![
                "query".to_owned(),
                format!(
                    "SELECT logical, type, status, code, body FROM records WHERE type = '{record_type}' ORDER BY logical"
                ),
                "--all".to_owned(),
            ],
            Self::Log { path, at, every } => {
                let mut words = vec!["log".to_owned(), path.as_str().to_owned()];
                if let Some(Commit(commit)) = at {
                    words.extend(["--at".to_owned(), commit.clone()]);
                }
                if *every {
                    words.push("--all".to_owned());
                } else {
                    words.extend(["--limit".to_owned(), "1".to_owned()]);
                }
                words
            }
        };
        words.extend(["--json", "--no-input"].map(str::to_owned));
        words
    }
}

/// The limits of one call, which the tests shorten.
#[derive(Clone, Copy, Debug)]
pub struct BoardLimits {
    time: Duration,
    stdout: usize,
}

impl BoardLimits {
    /// Returns the limits of [`BOARD_TIME_LIMIT`] and [`BOARD_STDOUT_LIMIT`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            time: BOARD_TIME_LIMIT,
            stdout: BOARD_STDOUT_LIMIT,
        }
    }

    /// Returns the limits with `time` for the whole call.
    #[must_use]
    pub const fn time(mut self, time: Duration) -> Self {
        self.time = time;
        self
    }

    /// Returns the limits with `stdout` bytes of standard output per query.
    #[must_use]
    pub const fn stdout(mut self, stdout: usize) -> Self {
        self.stdout = stdout;
        self
    }

    /// Returns the budget of a call that starts now.
    #[must_use]
    pub fn start(self) -> BoardBudget {
        BoardBudget {
            deadline: Instant::now() + self.time,
            stdout: self.stdout,
        }
    }
}

impl Default for BoardLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// The deadline of one call and the bytes of standard output per query.
#[derive(Clone, Copy, Debug)]
pub struct BoardBudget {
    deadline: Instant,
    stdout: usize,
}

/// Runs each of `queries` against `program` on the board `at` within
/// `budget`, at most [`BOARD_AT_ONCE`] at a time, until `stop` cancels
/// them, and returns each query's result in the order of `queries`. A
/// query that never started ends as [`Exit::Unknown`] with no output.
pub(crate) fn run_all(
    program: &Program,
    at: &BoardAt,
    queries: &[BoardQuery],
    budget: BoardBudget,
    stop: &Cancel,
) -> Vec<Result<Finished, SpawnError>> {
    at_most(
        BOARD_AT_ONCE,
        queries,
        || Ok(not_run()),
        || !stop.is_cancelled(),
        |query| run_one(program, at, query, budget, stop),
    )
}

/// Runs `query` until the deadline of `budget` or `stop`.
fn run_one(
    program: &Program,
    at: &BoardAt,
    query: &BoardQuery,
    budget: BoardBudget,
    stop: &Cancel,
) -> Result<Finished, SpawnError> {
    let settings = RunOptions::new()
        .stdout(Capture::whole(budget.stdout).over(OverLimit::Sink))
        .stderr(Capture::tail(STDERR_LIMIT))
        .deadline(budget.deadline)
        .cancel(stop);
    let mut words = at.words();
    words.extend(query.words().into_iter().map(OsString::from));
    let finished = crumbkit_process::run(program.command(&words), &settings)?;
    let exit = match finished.outcome() {
        Outcome::TimedOut => Exit::TimedOut,
        _ => Exit::of(finished.status()),
    };
    let (out, err) = finished.into_output();
    Ok(Finished {
        exit,
        stdout: if out.over_limit() {
            Stdout::OverLimit
        } else {
            Stdout::Whole(out.into_bytes())
        },
        stderr: err.into_bytes(),
    })
}

/// Returns the result of a query that never ran.
const fn not_run() -> Finished {
    Finished {
        exit: Exit::Unknown,
        stdout: Stdout::Whole(Vec::new()),
        stderr: Vec::new(),
    }
}
