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
//!   that flag. When the dry run ends with [`Outcome::Result`], or at once
//!   for a command without `--dry-run`, the [`Preview`] holds a
//!   [`ConfirmationId`] bound to the exact vector.
//! - [`Runner::confirm`] runs the vector that the id was issued for, with
//!   `--yes` when the command takes it. An id works once, for ten minutes by
//!   default, and only for the same vector.
//!
//! In [`Mode::ReadOnly`] each of the three refuses every command that is
//! not read-only. A started [`Run`] yields each line of standard output as
//! it arrives, then its [`Outcome`], and [`Runner::cancel`] stops it.
//!
//! One runner serves the whole flightdeck process. Its limits, such as
//! [`MOST_RUNS`] and the [`Limits`] of each run's output, are the caps on
//! the memory and the children of every page that the process serves.
//!
//! The runner trusts its caller to be the page that holds the session's
//! token. A read-only command still reads what its values name, such as a
//! file that `--from FILE` names, so the token is the only guard on what
//! such a command reads. Each child inherits flightdeck's environment, so
//! the server keeps its token out of that environment, and the runner
//! removes no variable.
//!
//! Each child also inherits the signal dispositions of the process that
//! embeds the runner. A process that ignores SIGINT therefore makes every
//! cancel escalate to SIGTERM or SIGKILL, because its children ignore the
//! cancel's SIGINT. `flightdeck open` installs handlers for SIGINT and
//! SIGTERM before it serves, and a handled signal resets to its default on
//! exec, so its children start with SIGINT at its default.

mod clock;
mod confirmations;
mod end_failure;
mod failure;
mod outcome;
mod request;
mod runs;
mod trace;
mod vector;

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub use self::clock::{Clock, SystemClock};
pub use self::confirmations::ConfirmationId;
use self::confirmations::Confirmations;
pub use self::end_failure::EndFailure;
pub use self::failure::{
    ConfirmationProblem, RUN_BUSY, RUN_CLOSED, RUN_CONFIRMATION, RUN_NEEDS_CONFIRMATION,
    RUN_NO_DOCUMENT, RUN_NOT_RUNNING, RUN_OUTPUT_LIMIT, RUN_RANDOM, RUN_READ_ONLY, RUN_REFUSED,
    RUN_SPAWN, Reason, RunFailure, Takes,
};
use self::outcome::Ending;
pub use self::outcome::Outcome;
pub use self::request::{RunRequest, Value};
use self::runs::Runs;
use self::trace::Trace;
pub(crate) use self::vector::Argv;
use self::vector::{Extra, Vector};
use crate::model::{Command, Effects, Tool};
use crate::spawn::{self, Event, Events, Exit, Program, Silent};
pub use crate::spawn::{DRAIN_GRACE, Escalation, Limit, Limits, QUEUE};

/// How long a confirmation id works after the preview that issued it.
pub const CONFIRMATION_LIFE: Duration = Duration::from_secs(10 * 60);

/// How long a shutdown lets the runs in progress end by themselves before
/// it cancels them.
pub const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

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
    shutdown_grace: Duration,
}

impl Settings {
    /// Returns the default settings: [`Limits::default`],
    /// [`Escalation::default`], [`MOST_RUNS`], [`CONFIRMATION_LIFE`] and
    /// [`SHUTDOWN_GRACE`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            limits: Limits::default(),
            escalation: Escalation::default(),
            most_runs: MOST_RUNS,
            confirmation_life: CONFIRMATION_LIFE,
            shutdown_grace: SHUTDOWN_GRACE,
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

    /// Returns the settings with a shutdown that lets runs end by
    /// themselves for `grace` before it cancels them.
    #[must_use]
    pub const fn shutdown_grace(mut self, grace: Duration) -> Self {
        self.shutdown_grace = grace;
        self
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs the commands of one tool for the whole flightdeck process.
///
/// A child inherits the embedding process's signal dispositions, so a
/// process that ignores SIGINT makes every cancel escalate to SIGTERM or
/// SIGKILL. Handle SIGINT before the first run, as `flightdeck open` does,
/// so the children start with SIGINT at its default.
#[derive(Debug)]
pub struct Runner {
    tool: Tool,
    program: Program,
    mode: Mode,
    settings: Settings,
    clock: Arc<dyn Clock>,
    trace: Trace,
    confirmations: Arc<Confirmations>,
    runs: Runs,
}

impl Runner {
    /// Returns the runner of `tool`, whose program is `program`, with a new
    /// trace id for the session.
    ///
    /// # Errors
    ///
    /// Returns the error of the system's random source when it gives no
    /// random bytes for the trace id.
    pub fn new(
        tool: Tool,
        program: Program,
        mode: Mode,
        clock: Arc<dyn Clock>,
        settings: Settings,
    ) -> Result<Self, getrandom::Error> {
        let trace = Trace::new()?;
        Ok(Self {
            tool,
            program,
            mode,
            confirmations: Arc::new(Confirmations::new(settings.confirmation_life)),
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

    /// Returns which commands the runner may run.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Starts the command of `request` when it is read-only or additive.
    ///
    /// # Errors
    ///
    /// Returns [`RunFailure::Refused`] for a request that the model does not
    /// allow, [`RunFailure::ReadOnly`] in read-only mode,
    /// [`RunFailure::NeedsConfirmation`] for a destructive command or one
    /// whose effects are unknown, [`RunFailure::Closed`] after a shutdown,
    /// [`RunFailure::Busy`] when the most runs are running, and
    /// [`RunFailure::Spawn`] when the program will not start.
    pub fn run(&self, request: &RunRequest) -> Result<Run, RunFailure> {
        let (command, vector) = self.check(request)?;
        if Class::of(command) == Class::Destructive {
            return Err(RunFailure::NeedsConfirmation {
                path: command.path().to_string(),
            });
        }
        self.start(command, &vector, Extra::None, || Ok(()))
    }

    /// Previews the command of `request`. A command that takes `--dry-run`
    /// runs with it, and the [`Preview`] holds a confirmation id once that
    /// dry run ends with [`Outcome::Result`]. A command without `--dry-run`
    /// runs nothing, and its preview holds the id at once.
    ///
    /// # Errors
    ///
    /// Returns the failures of [`Runner::run`], except
    /// [`RunFailure::NeedsConfirmation`], and [`RunFailure::Random`] when
    /// the system gives no random bytes for the id.
    pub fn preview(&self, request: &RunRequest) -> Result<Preview, RunFailure> {
        let (command, vector) = self.check(request)?;
        let line = vector.line(Extra::None);
        if vector.takes_dry_run() {
            let run = self.start(command, &vector, Extra::DryRun, || Ok(()))?;
            return Ok(Preview {
                line,
                confirmation: None,
                dry_run: Some(DryRun {
                    run,
                    vector,
                    confirmations: Arc::clone(&self.confirmations),
                    clock: Arc::clone(&self.clock),
                }),
            });
        }
        let confirmation = self
            .confirmations
            .issue(vector, self.clock.now())
            .map_err(|source| RunFailure::Random { source })?;
        Ok(Preview {
            line,
            confirmation: Some(confirmation),
            dry_run: None,
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
    /// expired or issued for another vector. The runner uses the id up once
    /// the request has passed the model and the mode and the runner is open
    /// and has room, so a confirmation refused with
    /// [`RunFailure::Confirmation`] or [`RunFailure::Spawn`] uses its id up.
    /// A confirmation refused with [`RunFailure::Refused`],
    /// [`RunFailure::ReadOnly`], [`RunFailure::Closed`] or
    /// [`RunFailure::Busy`] keeps its id.
    pub fn confirm(
        &self,
        confirmation: &ConfirmationId,
        request: &RunRequest,
    ) -> Result<Run, RunFailure> {
        let (command, vector) = self.check(request)?;
        self.start(command, &vector, Extra::Yes, || {
            self.confirmations
                .take(confirmation, &vector, self.clock.now())
                .map_err(|problem| RunFailure::Confirmation { problem })
        })
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

    /// Refuses every new run with [`RunFailure::Closed`], lets the runs in
    /// progress end by themselves within the shutdown grace of the
    /// settings, then cancels the rest and waits for each to end. Dropping
    /// the runner shuts it down.
    pub fn shutdown(&self) {
        self.runs.shutdown(self.settings.shutdown_grace);
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

    /// Starts `vector` of `command` with `extra`, once the runner has room
    /// for the run and `ready` allows it.
    fn start(
        &self,
        command: &Command,
        vector: &Vector,
        extra: Extra,
        ready: impl FnOnce() -> Result<(), RunFailure>,
    ) -> Result<Run, RunFailure> {
        let program = self
            .program
            .clone()
            .env("TRACEPARENT", self.trace.next_parent());
        let argv = vector.argv(extra);
        let Settings {
            limits, escalation, ..
        } = self.settings;
        let (id, events) = self.runs.start(|| {
            ready()?;
            spawn::stream(&program, &argv, limits, escalation).map_err(|source| RunFailure::Spawn {
                tool: self.tool.name().to_owned(),
                source,
            })
        })?;
        Ok(Run {
            id,
            line: vector.line(Extra::None),
            events: Some(events),
            last: None,
            exit: None,
            ending: Ending::new(&self.tool, command, vector.line(extra), limits),
        })
    }
}

/// Shuts the runner down, so no run's thread outlives it.
impl Drop for Runner {
    fn drop(&mut self) {
        self.shutdown();
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
    /// Returns the class of `command`. A command that declares itself
    /// destructive is destructive, even when it also declares itself
    /// read-only.
    const fn of(command: &Command) -> Self {
        match command.effects() {
            Effects::Known {
                destructive: true, ..
            }
            | Effects::Unknown => Self::Destructive,
            Effects::Known {
                read_only: true, ..
            } => Self::ReadOnly,
            Effects::Known { .. } => Self::Additive,
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

/// Reads a run id as the page sends it, such as `3`, and refuses any text
/// other than decimal digits, such as `+3`.
impl FromStr for RunId {
    type Err = RunFailure;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let not_running = || RunFailure::NotRunning { run: None };
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(not_running());
        }
        text.parse().map(Self).map_err(|_| not_running())
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

/// What a wait of at most a given time for a run's next event found.
#[derive(Debug)]
pub enum Waited {
    /// The run's next event.
    Event(RunEvent),
    /// No event arrived within the time.
    Silent,
}

/// A started run, which yields its events in order.
///
/// The run holds at most [`QUEUE`] lines that the caller has not read, and
/// then the child waits until the caller reads one. Dropping the run before
/// its outcome cancels it, as [`Runner::cancel`] does, and frees its place
/// among the runner's runs once the child has ended. After a cancel or a
/// shutdown, a line that finds the queue full is dropped, and the outcome
/// still reads the last line that the child printed.
#[derive(Debug)]
pub struct Run {
    id: RunId,
    line: String,
    events: Option<Events>,
    last: Option<String>,
    exit: Option<Exit>,
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

    /// Returns how the child ended once the run has yielded its outcome, or
    /// `None` before then and for a child that flightdeck could not follow
    /// to its end.
    #[must_use]
    pub const fn exit(&self) -> Option<Exit> {
        self.exit
    }

    /// Waits at most `timeout` for the next event, and returns
    /// [`Waited::Silent`] when none arrived. It returns `None` once the
    /// outcome has been returned.
    pub fn next_timeout(&mut self, timeout: Duration) -> Option<Waited> {
        match self.events.as_ref()?.recv_timeout(timeout) {
            Ok(event) => self.read(event).map(Waited::Event),
            Err(Silent) => Some(Waited::Silent),
        }
    }

    /// Returns the run's event for `event`, the next event of its child,
    /// and keeps the last line for the outcome.
    fn read(&mut self, event: Option<Event>) -> Option<RunEvent> {
        let ended = match event? {
            Event::Line(line) => {
                self.last = Some(line.clone());
                return Some(RunEvent::Line(line));
            }
            Event::End { ended, lost } => {
                self.last = lost.or_else(|| self.last.take());
                ended
            }
        };
        self.events = None;
        self.exit = ended.as_ref().ok().map(spawn::Ended::exit);
        let outcome = self.ending.outcome(self.last.as_deref(), ended);
        Some(RunEvent::Outcome(outcome))
    }
}

/// Yields each line of standard output as it arrives, then the outcome.
/// Waiting for the next event blocks until the child prints a line or the
/// run ends.
impl Iterator for Run {
    type Item = RunEvent;

    fn next(&mut self) -> Option<RunEvent> {
        let event = self.events.as_ref()?.recv();
        self.read(event)
    }
}

/// A preview: the line it would confirm, the dry run when the command takes
/// `--dry-run`, and the confirmation id once the preview has issued one.
///
/// The preview yields the dry run's events. When the dry run ends with
/// [`Outcome::Result`], the preview issues the id. An outcome of any other
/// kind issues none, and a cancelled dry run ends with
/// [`Outcome::Cancelled`]. Dropping the preview cancels its dry run.
#[derive(Debug)]
pub struct Preview {
    line: String,
    confirmation: Option<ConfirmationId>,
    dry_run: Option<DryRun>,
}

/// A preview's dry run, and what it needs to issue the id when the dry run
/// succeeds.
#[derive(Debug)]
struct DryRun {
    run: Run,
    vector: Vector,
    confirmations: Arc<Confirmations>,
    clock: Arc<dyn Clock>,
}

impl Preview {
    /// Returns the id that [`Runner::confirm`] takes, or `None` until the
    /// dry run has ended with [`Outcome::Result`].
    #[must_use]
    pub const fn confirmation(&self) -> Option<ConfirmationId> {
        self.confirmation
    }

    /// Returns the command line that the confirmation runs, without
    /// `--yes`.
    #[must_use]
    pub fn line(&self) -> &str {
        &self.line
    }

    /// Returns the id of the dry run, which [`Runner::cancel`] takes, or
    /// `None` for a command without `--dry-run`.
    #[must_use]
    pub fn run_id(&self) -> Option<RunId> {
        self.dry_run.as_ref().map(|dry_run| dry_run.run.id())
    }

    /// Returns how the dry run's child ended, as [`Run::exit`] does, or
    /// `None` for a command without `--dry-run`.
    #[must_use]
    pub fn exit(&self) -> Option<Exit> {
        self.dry_run.as_ref().and_then(|dry_run| dry_run.run.exit())
    }

    /// Waits at most `timeout` for the dry run's next event, as
    /// [`Run::next_timeout`] does, and issues the id as the preview's
    /// iterator does. It returns `None` at once for a command without
    /// `--dry-run`.
    pub fn next_timeout(&mut self, timeout: Duration) -> Option<Waited> {
        match self.dry_run.as_mut()?.run.next_timeout(timeout)? {
            Waited::Event(event) => Some(Waited::Event(self.issue(event))),
            Waited::Silent => Some(Waited::Silent),
        }
    }

    /// Returns `event`, and issues the id when it is the dry run's outcome
    /// [`Outcome::Result`]. When the system gives no random bytes for the
    /// id, the outcome is [`EndFailure::Random`] instead.
    fn issue(&mut self, event: RunEvent) -> RunEvent {
        let Some(dry_run) = &self.dry_run else {
            return event;
        };
        let RunEvent::Outcome(Outcome::Result) = event else {
            return event;
        };
        let now = dry_run.clock.now();
        RunEvent::Outcome(
            match dry_run.confirmations.issue(dry_run.vector.clone(), now) {
                Ok(confirmation) => {
                    self.confirmation = Some(confirmation);
                    Outcome::Result
                }
                Err(source) => Outcome::Failed(EndFailure::Random { source }),
            },
        )
    }
}

/// Yields each event of the dry run, and nothing for a command without
/// `--dry-run`. When the dry run ends with [`Outcome::Result`] the preview
/// issues its id, and when the system gives no random bytes
/// for the id the outcome is [`EndFailure::Random`] instead.
impl Iterator for Preview {
    type Item = RunEvent;

    fn next(&mut self) -> Option<RunEvent> {
        let event = self.dry_run.as_mut()?.run.next()?;
        Some(self.issue(event))
    }
}
