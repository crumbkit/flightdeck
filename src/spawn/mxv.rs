//! The operation that reads mxv's workspaces: it runs `mxv list`,
//! `mxv status` of one workspace and `mxv proxy status`, each of which mxv
//! declares read-only, and captures the JSON document each prints. It
//! states the five items of PROC-2:
//!
//! - Program: `FLIGHTDECK_MXV` when it is set, or else `mxv` found on the
//!   `PATH` that flightdeck inherited. The person who installed mxv or set
//!   the variable is trusted to supply its bytes.
//! - Arguments: the fixed argument vector of each [`MxvQuery`], with no
//!   shell. A workspace's name reaches mxv inside the one word
//!   `--workspace=<name>`, and [`WorkspaceName`] has refused a name that is
//!   empty, starts with `-` or holds a control character or `/` before any
//!   query starts.
//! - Environment: the child inherits flightdeck's environment, plus the
//!   variables that [`Program::env`] adds and `MXV_PROGRAM` set to
//!   flightdeck's own name, so mxv records the runs in its usage log as
//!   flightdeck's.
//! - Streams and deadline: standard input is closed. Standard output is kept
//!   up to the limit of [`MxvLimits`], [`MXV_STDOUT_LIMIT`] unless a test
//!   lowers it, and standard error keeps its last [`STDERR_LIMIT`] bytes.
//!   Every query of one call shares one deadline, [`MXV_TIME_LIMIT`] after
//!   the call starts, which covers the wait for each child and every read
//!   from its pipes, so one page's query ends within that time. At most
//!   [`MXV_AT_ONCE`] queries run at a time, each on a thread of a
//!   `std::thread::scope` that joins it. The caller's stop starts no
//!   further query and cancels the ones in progress, which the server's
//!   stop uses so that a query never holds up flightdeck's stop.
//! - End: as for `describe`, crumbkit-process starts each child in a process
//!   group of its own on Unix, or in a job object on Windows, and stops the
//!   whole group or ends the job when the child exits, its deadline passes
//!   or the stop cancels it, and only then reaps it. A query only reads, so
//!   a stopped child leaves no partial work.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crumbkit_process::{Cancel, Capture, Outcome, OverLimit, RunOptions};

use super::{Exit, Finished, Program, STDERR_LIMIT, SpawnError, Stdout};
use crate::lock::lock;
use crate::workspaces::WorkspaceName;

#[cfg(unix)]
#[cfg(test)]
mod tests;

/// The time within which every query of one call ends. `mxv status` of one
/// workspace takes a few tens of milliseconds, so a machine with a hundred
/// workspaces lists them in about a second.
pub const MXV_TIME_LIMIT: Duration = Duration::from_secs(10);

/// The most bytes of standard output that a query keeps. `mxv list` of a
/// hundred workspaces prints well under a megabyte.
pub const MXV_STDOUT_LIMIT: usize = 8 * 1024 * 1024;

/// The most queries that run at a time.
pub const MXV_AT_ONCE: usize = 4;

/// One of the read-only commands of mxv that flightdeck runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MxvQuery {
    /// `mxv list --all`, with every workspace.
    List,
    /// `mxv status` of the named workspace, with each unit and checkout.
    Status(WorkspaceName),
    /// `mxv proxy status`, with each route that a workspace declares.
    ProxyStatus,
}

impl MxvQuery {
    /// Returns the query's argument vector after the program.
    #[must_use]
    pub fn words(&self) -> Vec<String> {
        let mut words: Vec<String> = match self {
            Self::List => vec!["list".to_owned(), "--all".to_owned()],
            Self::Status(name) => vec!["status".to_owned(), format!("--workspace={name}")],
            Self::ProxyStatus => vec!["proxy".to_owned(), "status".to_owned()],
        };
        words.extend(["--json", "--full", "--no-input"].map(str::to_owned));
        words
    }
}

/// The limits of one call, which the tests shorten.
#[derive(Clone, Copy, Debug)]
pub struct MxvLimits {
    time: Duration,
    stdout: usize,
}

impl MxvLimits {
    /// Returns the limits of [`MXV_TIME_LIMIT`] and [`MXV_STDOUT_LIMIT`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            time: MXV_TIME_LIMIT,
            stdout: MXV_STDOUT_LIMIT,
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

    /// Returns the budget of a call that starts now: its deadline, the
    /// call's time from now, and the bytes of standard output per query.
    /// Each run of a call that runs several in turn reads the one budget,
    /// so the whole call ends by its deadline.
    #[must_use]
    pub fn start(self) -> Budget {
        Budget {
            deadline: Instant::now() + self.time,
            stdout: self.stdout,
        }
    }
}

/// The deadline of one call and the bytes of standard output per query.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    deadline: Instant,
    stdout: usize,
}

impl Default for MxvLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs each of `queries` against `program` within `budget`, at most
/// [`MXV_AT_ONCE`] at a time, until `stop` cancels them, and returns each
/// query's result in the order of `queries`. A query that never started,
/// because the stop came first, ends as [`Exit::Unknown`] with no output.
pub(crate) fn run_all(
    program: &Program,
    queries: &[MxvQuery],
    budget: Budget,
    stop: &Cancel,
) -> Vec<Result<Finished, SpawnError>> {
    let program = program.marked("mxv");
    let results: Mutex<Vec<Result<Finished, SpawnError>>> =
        Mutex::new(queries.iter().map(|_| Ok(not_run())).collect());
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..MXV_AT_ONCE.min(queries.len()) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(query) = queries.get(index) else {
                        return;
                    };
                    if stop.is_cancelled() {
                        return;
                    }
                    let result = run_one(&program, query, budget, stop);
                    if let Some(slot) = lock(&results).get_mut(index) {
                        *slot = result;
                    }
                }
            });
        }
    });
    results
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Runs `query` until the deadline of `budget` or `stop`, keeping at most
/// the budget's bytes of its standard output.
fn run_one(
    program: &Program,
    query: &MxvQuery,
    budget: Budget,
    stop: &Cancel,
) -> Result<Finished, SpawnError> {
    let settings = RunOptions::new()
        .stdout(Capture::whole(budget.stdout).over(OverLimit::Sink))
        .stderr(Capture::tail(STDERR_LIMIT))
        .deadline(budget.deadline)
        .cancel(stop);
    let finished = crumbkit_process::run(program.command(&query.words()), &settings)?;
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
