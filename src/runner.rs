//! Runs a tool's commands for the page.
//!
//! The page sends a [`RunRequest`]: a command's path and a value for each
//! argument it sets, by the argument's name in the model. The runner checks
//! the request against the tool's model and builds the argument vector
//! itself, so the page never sends a command line. A command's effects
//! decide how it starts:
//!
//! - [`Runner::run`] starts a read-only or additive command at once, and
//!   refuses a destructive command, or one whose effects are unknown, with
//!   [`RUN_NEEDS_CONFIRMATION`].
//! - [`Runner::preview`] runs the command with `--dry-run` when it takes
//!   that flag, and issues a [`ConfirmationId`] bound to the exact vector.
//! - [`Runner::confirm`] runs the vector that the id was issued for, with
//!   `--yes` when the command takes it. An id works once, for ten minutes by
//!   default, and only for the same vector.
//!
//! In [`Mode::ReadOnly`] each of the three refuses every command that is
//! not read-only. A started [`Run`] yields each line of standard output as
//! it arrives, then its [`Outcome`], and [`Runner::cancel`] stops it.

mod clock;
mod confirmations;
mod failure;
mod lock;
mod outcome;
mod request;
mod runs;
mod trace;
mod vector;

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

pub use self::clock::{Clock, SystemClock};
pub use self::confirmations::ConfirmationId;
use self::confirmations::Confirmations;
pub use self::failure::{
    ConfirmationProblem, RUN_BUSY, RUN_CONFIRMATION, RUN_NEEDS_CONFIRMATION, RUN_NO_DOCUMENT,
    RUN_NOT_RUNNING, RUN_OUTPUT_LIMIT, RUN_RANDOM, RUN_READ_ONLY, RUN_REFUSED, RUN_SPAWN, Reason,
    RunFailure, Takes,
};
use self::outcome::Ending;
pub use self::outcome::Outcome;
pub use self::request::{RunRequest, Value};
use self::runs::Runs;
use self::trace::Trace;
pub(crate) use self::vector::Argv;
use self::vector::{Extra, Vector};
use crate::model::{Command, Effects, Tool};
use crate::spawn::{self, Event, Program};
pub use crate::spawn::{Escalation, Limit, Limits};

/// How long a confirmation id works after the preview that issued it.
pub const CONFIRMATION_LIFE: Duration = Duration::from_secs(10 * 60);

/// The most runs of one runner at once.
///
/// RUST-10: one number covers every session. It handles a person who keeps
/// a few streams open beside a short run, and it refuses a page that wants
/// more at once. A configured number replaces it when a tool's page needs
/// more.
pub const MOST_RUNS: usize = 8;

/// Which commands a runner may run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Every command, a destructive one only after a confirmation.
    Full,
    /// Only read-only commands, as `--read-only` asks.
    ReadOnly,
}

/// The limits and graces of a runner's runs.
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    limits: Limits,
    escalation: Escalation,
    most_runs: usize,
    confirmation_life: Duration,
}

impl Settings {
    /// Returns the default settings: [`Limits::default`],
    /// [`Escalation::default`], [`MOST_RUNS`] and [`CONFIRMATION_LIFE`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            limits: Limits::default(),
            escalation: Escalation::default(),
            most_runs: MOST_RUNS,
            confirmation_life: CONFIRMATION_LIFE,
        }
    }

    /// Returns the settings with these limits of standard output.
    #[must_use]
    pub const fn limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    /// Returns the settings with these graces after each signal of a
    /// cancel.
    #[must_use]
    pub const fn escalation(mut self, escalation: Escalation) -> Self {
        self.escalation = escalation;
        self
    }

    /// Returns the settings with at most `most` runs at once.
    #[must_use]
    pub const fn most_runs(mut self, most: usize) -> Self {
        self.most_runs = most;
        self
    }

    /// Returns the settings with confirmation ids that work for `life`.
    #[must_use]
    pub const fn confirmation_life(mut self, life: Duration) -> Self {
        self.confirmation_life = life;
        self
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the commands of one tool for one flightdeck session.
#[derive(Debug)]
pub struct Runner {
    tool: Tool,
    program: Program,
    mode: Mode,
    settings: Settings,
    clock: Arc<dyn Clock>,
    trace: Trace,
    confirmations: Confirmations,
    runs: Runs,
}

impl Runner {
    /// Returns the runner of `tool`, whose program is `program`, with a new
    /// trace id for the session.
    ///
    /// # Errors
    ///
    /// Returns [`RunFailure::Random`] when the system gives no random bytes
    /// for the trace id.
    pub fn new(
        tool: Tool,
        program: Program,
        mode: Mode,
        clock: Arc<dyn Clock>,
        settings: Settings,
    ) -> Result<Self, RunFailure> {
        let trace = Trace::new().map_err(|source| RunFailure::Random { source })?;
        Ok(Self {
            tool,
            program,
            mode,
            confirmations: Confirmations::new(settings.confirmation_life),
            runs: Runs::new(settings.most_runs),
            settings,
            clock,
            trace,
        })
    }

    /// Returns the model of the tool that the runner runs.
    #[must_use]
    pub const fn tool(&self) -> &Tool {
        &self.tool
    }

    /// Starts the command of `request` when it is read-only or additive.
    ///
    /// # Errors
    ///
    /// Returns [`RunFailure::Refused`] for a request that the model does not
    /// allow, [`RunFailure::ReadOnly`] in read-only mode,
    /// [`RunFailure::NeedsConfirmation`] for a destructive command or one
    /// whose effects are unknown, [`RunFailure::Busy`] when the most runs
    /// are running, and [`RunFailure::Spawn`] when the program will not
    /// start.
    pub fn run(&self, request: &RunRequest) -> Result<Run, RunFailure> {
        let (command, vector) = self.check(request)?;
        if Class::of(command) == Class::Destructive {
            return Err(RunFailure::NeedsConfirmation {
                path: command.path().to_string(),
            });
        }
        self.start(command, &vector, Extra::None)
    }

    /// Issues a confirmation id for the command of `request`, and runs it
    /// with `--dry-run` when the command takes that flag.
    ///
    /// # Errors
    ///
    /// Returns the failures of [`Runner::run`], except
    /// [`RunFailure::NeedsConfirmation`], and [`RunFailure::Random`] when
    /// the system gives no random bytes for the id.
    pub fn preview(&self, request: &RunRequest) -> Result<Preview, RunFailure> {
        let (command, vector) = self.check(request)?;
        let run = if vector.takes_dry_run() {
            Some(self.start(command, &vector, Extra::DryRun)?)
        } else {
            None
        };
        let line = vector.line().to_owned();
        let confirmation = self
            .confirmations
            .issue(vector, self.clock.now())
            .map_err(|source| RunFailure::Random { source })?;
        Ok(Preview {
            confirmation,
            line,
            run,
        })
    }

    /// Starts the command of `request` with `--yes`, when `confirmation`
    /// was issued for the same vector and has not expired or been used.
    ///
    /// # Errors
    ///
    /// Returns the failures of [`Runner::run`], except
    /// [`RunFailure::NeedsConfirmation`], and
    /// [`RunFailure::Confirmation`] for an id that is unknown, used,
    /// expired or issued for another vector. A refused id is used up.
    pub fn confirm(
        &self,
        confirmation: &ConfirmationId,
        request: &RunRequest,
    ) -> Result<Run, RunFailure> {
        let (command, vector) = self.check(request)?;
        self.confirmations
            .take(confirmation, &vector, self.clock.now())
            .map_err(|problem| RunFailure::Confirmation { problem })?;
        self.start(command, &vector, Extra::Yes)
    }

    /// Stops the run `id`: SIGINT, then SIGTERM and SIGKILL after the
    /// graces of the settings. The run then ends with the lines that
    /// arrived and its outcome.
    ///
    /// # Errors
    ///
    /// Returns [`RunFailure::NotRunning`] when no run with the id is
    /// running.
    pub fn cancel(&self, id: RunId) -> Result<(), RunFailure> {
        self.runs.cancel(id)
    }

    /// Cancels every run and waits for each to end.
    pub fn shutdown(&self) {
        self.runs.shutdown();
    }

    /// Returns the command and the vector of `request`, refusing a request
    /// that the model or the mode does not allow.
    fn check(&self, request: &RunRequest) -> Result<(&Command, Vector), RunFailure> {
        let (command, vector) = vector::build(&self.tool, request)?;
        if self.mode == Mode::ReadOnly && Class::of(command) != Class::ReadOnly {
            return Err(RunFailure::ReadOnly {
                path: command.path().to_string(),
            });
        }
        Ok((command, vector))
    }

    /// Starts `vector` of `command` with `extra`.
    fn start(&self, command: &Command, vector: &Vector, extra: Extra) -> Result<Run, RunFailure> {
        let program = self
            .program
            .clone()
            .env("TRACEPARENT", self.trace.next_parent());
        let argv = vector.argv(extra);
        let Settings {
            limits, escalation, ..
        } = self.settings;
        let (id, events) = self.runs.start(|| {
            spawn::stream(&program, &argv, limits, escalation).map_err(|source| RunFailure::Spawn {
                tool: self.tool.name().to_owned(),
                source,
            })
        })?;
        Ok(Run {
            id,
            line: vector.line().to_owned(),
            events: Some(events),
            last: None,
            ending: Ending::new(&self.tool, command, vector.line(), limits),
        })
    }
}

/// How a command's effects decide how it starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    /// The command changes nothing.
    ReadOnly,
    /// The command adds without destroying.
    Additive,
    /// The command may destroy data, or declares no effects.
    Destructive,
}

impl Class {
    /// Returns the class of `command`.
    const fn of(command: &Command) -> Self {
        match command.effects() {
            Effects::Known {
                read_only: true, ..
            } => Self::ReadOnly,
            Effects::Known {
                destructive: false, ..
            } => Self::Additive,
            Effects::Known { .. } | Effects::Unknown => Self::Destructive,
        }
    }
}

/// The id of one run of a runner, which counts from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RunId(u64);

/// Prints the number, such as `3`.
impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Reads a run id as the page sends it, such as `3`.
impl FromStr for RunId {
    type Err = RunFailure;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.parse()
            .map(Self)
            .map_err(|_| RunFailure::NotRunning { run: None })
    }
}

/// One event of a run.
#[derive(Debug)]
pub enum RunEvent {
    /// One line of standard output, as it arrived.
    Line(String),
    /// How the run ended, which is the last event.
    Outcome(Outcome),
}

/// A started run, which yields its events in order.
#[derive(Debug)]
pub struct Run {
    id: RunId,
    line: String,
    events: Option<Receiver<Event>>,
    last: Option<String>,
    ending: Ending,
}

impl Run {
    /// Returns the run's id, which [`Runner::cancel`] takes.
    #[must_use]
    pub const fn id(&self) -> RunId {
        self.id
    }

    /// Returns the command line that the run runs, POSIX-quoted, without
    /// the flags that flightdeck adds, such as `shelf books remove 2`.
    #[must_use]
    pub fn line(&self) -> &str {
        &self.line
    }
}

/// Yields each line of standard output as it arrives, then the outcome.
/// Waiting for the next event blocks until the child prints a line or the
/// run ends.
impl Iterator for Run {
    type Item = RunEvent;

    fn next(&mut self) -> Option<RunEvent> {
        let events = self.events.as_ref()?;
        let ended = match events.recv() {
            Ok(Event::Line(line)) => {
                self.last = Some(line.clone());
                return Some(RunEvent::Line(line));
            }
            Ok(Event::End(ended)) => ended,
            Err(_) => Err(spawn::SpawnError::Read(std::io::Error::other(
                "the thread that followed the program stopped",
            ))),
        };
        self.events = None;
        let outcome = self.ending.outcome(self.last.as_deref(), ended);
        Some(RunEvent::Outcome(outcome))
    }
}

/// What a preview issued: the confirmation id, the line it confirms, and
/// the dry run when the command takes `--dry-run`.
#[derive(Debug)]
pub struct Preview {
    confirmation: ConfirmationId,
    line: String,
    run: Option<Run>,
}

impl Preview {
    /// Returns the id that [`Runner::confirm`] takes.
    #[must_use]
    pub const fn confirmation(&self) -> ConfirmationId {
        self.confirmation
    }

    /// Returns the command line that the confirmation runs, without
    /// `--yes`.
    #[must_use]
    pub fn line(&self) -> &str {
        &self.line
    }

    /// Returns the dry run, or `None` for a command without `--dry-run`.
    #[must_use]
    pub fn into_run(self) -> Option<Run> {
        self.run
    }
}
