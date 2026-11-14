//! The operation that measures a tool's help pages: it runs `<tool> -h`,
//! `<tool> <group> -h` and `<tool> <command> -h` and counts the bytes each
//! prints. It states the five items of PROC-2:
//!
//! - Program: the name or path that the person gave, as for `describe`.
//! - Arguments: the words of one command path that the tool's model holds,
//!   then `-h`, with no shell. The model refuses a tool whose `commands
//!   list` names a path with a word that is empty or starts with `-`, so no
//!   such word reaches a help run as an option.
//! - Environment: cleared, then the variables that [`Program::env`] adds,
//!   such as the absolute `PATH` of [`Program::at_launch`], then
//!   `XDG_STATE_HOME` set to one [`FreshState`] for the whole measurement.
//!   A crumbkit tool logs a help request to its usage log, and with a
//!   cleared environment it can still find the person's home directory, so
//!   the fresh state directory keeps every help run out of the person's
//!   usage log. The directory is removed when the measurement ends, because
//!   the [`FreshState`] drops on every path.
//! - Streams and deadline: standard input and standard error are null.
//!   Standard output is kept up to [`HELP_LIMIT`] bytes, and a run that
//!   prints more is stopped. Each run has [`HELP_TIME_LIMIT`], and no run
//!   starts once [`MEASURE_LIMIT`] has passed since the measurement began.
//!   At most [`AT_ONCE`] runs go at a time, each on a thread of a
//!   `std::thread::scope` that joins it. The caller's stop starts no further
//!   run and cancels the runs in progress, which the runner's shutdown uses
//!   so that a measurement never holds up flightdeck's stop.
//! - End: as for `describe`, crumbkit-process stops the child's whole
//!   process group or job when it exits, its limit passes or the stop
//!   cancels it, and only then reaps it. A help page only reads, so a stopped child leaves no partial
//!   work.

use std::io;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crumbkit_process::{Cancel, Capture, Outcome, RunOptions, StopReason};

use super::{Exit, FreshState, Program};
use crate::lock::lock;

#[cfg(unix)]
#[cfg(test)]
mod tests;

/// The time limit of one help run. A crumbkit help page takes a few
/// milliseconds.
pub const HELP_TIME_LIMIT: Duration = Duration::from_secs(2);

/// The most bytes of one help page. A larger output is not a help page.
pub const HELP_LIMIT: usize = 64 * 1024;

/// The time after which no further help run starts.
pub const MEASURE_LIMIT: Duration = Duration::from_secs(30);

/// The most help runs at a time.
pub const AT_ONCE: usize = 4;

/// The limits of one measurement, which the tests shorten.
#[derive(Clone, Copy, Debug)]
pub struct HelpLimits {
    run: Duration,
    total: Duration,
}

impl HelpLimits {
    /// Returns the limits of [`HELP_TIME_LIMIT`] per run and
    /// [`MEASURE_LIMIT`] in all.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            run: HELP_TIME_LIMIT,
            total: MEASURE_LIMIT,
        }
    }

    /// Returns the limits with `run` per help run.
    #[must_use]
    pub const fn run(mut self, run: Duration) -> Self {
        self.run = run;
        self
    }

    /// Returns the limits with `total` for the whole measurement.
    #[must_use]
    pub const fn total(mut self, total: Duration) -> Self {
        self.total = total;
        self
    }
}

impl Default for HelpLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// How one help run went: the bytes of its page, or why it has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpPage {
    /// The run exited with 0 and printed this many bytes.
    Bytes(u64),
    /// The run exited with a code other than 0, or a signal ended it.
    Exit(Exit),
    /// The run exited with 0 and printed nothing.
    Empty,
    /// The run passed [`HELP_TIME_LIMIT`] and was stopped.
    TimedOut,
    /// The run printed more than [`HELP_LIMIT`] bytes and was stopped.
    TooLarge,
    /// The program would not start, or flightdeck could not follow it.
    NotStarted,
    /// [`MEASURE_LIMIT`] passed, or the measurement was stopped, before
    /// the run ended, so it never started or was cancelled.
    NotRun,
}

/// Runs `-h` of each of `pages`, each a command path such as
/// `["books", "list"]` or `[]` for the tool's own page, and returns each
/// page's result in the same order. Once `stop` is cancelled no further
/// run starts, and the runs in progress are cancelled.
///
/// # Errors
///
/// Returns the error of a temporary state directory that cannot be made,
/// before any run starts.
pub(crate) fn measure_help(
    program: &Program,
    pages: &[Vec<String>],
    limits: HelpLimits,
    stop: &Cancel,
) -> io::Result<Vec<HelpPage>> {
    let started = Instant::now();
    let state = FreshState::new()?;
    let program = state.keep_out(program);
    let results = Mutex::new(vec![HelpPage::NotRun; pages.len()]);
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..AT_ONCE.min(pages.len()) {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(words) = pages.get(index) else {
                        return;
                    };
                    if started.elapsed() >= limits.total || stop.is_cancelled() {
                        return;
                    }
                    let page = run_help(&program, words, limits.run, stop);
                    if let Some(slot) = lock(&results).get_mut(index) {
                        *slot = page;
                    }
                }
            });
        }
    });
    // The state directory is removed here, after every run has ended.
    drop(state);
    Ok(results
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner))
}

/// Runs `<program> <words> -h` with a cleared environment that holds only
/// `program`'s variables, until `stop` cancels it, and returns how it went.
fn run_help(program: &Program, words: &[String], time_limit: Duration, stop: &Cancel) -> HelpPage {
    let mut command = program.cleared_command(words);
    command.arg("-h");
    let settings = RunOptions::new()
        .stdout(Capture::whole(HELP_LIMIT))
        .deadline(Instant::now() + time_limit)
        .cancel(stop);
    let Ok(finished) = crumbkit_process::run(command, &settings) else {
        return HelpPage::NotStarted;
    };
    let exit = Exit::of(finished.status());
    let outcome = finished.outcome();
    let (stdout, _) = finished.into_output();
    page_of(outcome, exit, stdout.bytes().len(), stdout.over_limit())
}

/// Returns the page of a run that ended with `outcome` and `exit`, printing
/// `printed` bytes, of which `over` says whether they passed
/// [`HELP_LIMIT`]. Output over the limit is too large however the run
/// ended, because a child can print past the limit and exit before
/// crumbkit-process stops it, and the run then reports an ordinary exit
/// with only the first [`HELP_LIMIT`] bytes.
fn page_of(outcome: Outcome, exit: Exit, printed: usize, over: bool) -> HelpPage {
    match outcome {
        _ if over => HelpPage::TooLarge,
        Outcome::TimedOut => HelpPage::TimedOut,
        Outcome::Stopped(StopReason::OverLimit) => HelpPage::TooLarge,
        Outcome::Stopped(StopReason::Cancelled) => HelpPage::NotRun,
        _ => match exit {
            Exit::Code(0) if printed == 0 => HelpPage::Empty,
            // A page is at most HELP_LIMIT bytes, so its length fits.
            Exit::Code(0) => HelpPage::Bytes(u64::try_from(printed).unwrap_or(u64::MAX)),
            other => HelpPage::Exit(other),
        },
    }
}
