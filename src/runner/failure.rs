//! The refusals and failures of a run, each with a stable code.

use std::fmt;

use crumbkit::{Code, Failure, FieldValue, Recovery};

use super::RunId;
use crate::model::{exit_code, last_lines};
use crate::spawn::{Exit, Limit, SpawnError};

/// The code of a request that the tool's model does not allow: an unknown
/// path, argument or value.
pub const RUN_REFUSED: Code = Code::new("flightdeck", "run", "refused");

/// The code of a destructive command, or one with unknown effects, that
/// was run without a confirmation.
pub const RUN_NEEDS_CONFIRMATION: Code = Code::new("flightdeck", "run", "needs_confirmation");

/// The code of a command that is not read-only, in read-only mode.
pub const RUN_READ_ONLY: Code = Code::new("flightdeck", "run", "read_only");

/// The code of a confirmation id that is unknown, used, expired or issued
/// for another vector.
pub const RUN_CONFIRMATION: Code = Code::new("flightdeck", "run", "confirmation");

/// The code of a run that ended without a document that explains its exit.
pub const RUN_NO_DOCUMENT: Code = Code::new("flightdeck", "run", "no_document");

/// The code of a program that would not start, or that flightdeck could
/// not follow to its end.
pub const RUN_SPAWN: Code = Code::new("flightdeck", "run", "spawn");

/// The code of a run whose output passed one of flightdeck's limits.
pub const RUN_OUTPUT_LIMIT: Code = Code::new("flightdeck", "run", "output_limit");

/// The code of a run refused because the most runs are running.
pub const RUN_BUSY: Code = Code::new("flightdeck", "run", "busy");

/// The code of a cancel of a run that is not running.
pub const RUN_NOT_RUNNING: Code = Code::new("flightdeck", "run", "not_running");

/// The code of a run refused because the runner has shut down.
pub const RUN_CLOSED: Code = Code::new("flightdeck", "run", "closed");

/// The code of a system that gives no random bytes.
pub const RUN_RANDOM: Code = Code::new("flightdeck", "run", "random");

/// A run that was refused or that failed.
#[derive(Debug, Failure)]
pub enum RunFailure {
    /// The tool's model does not allow the request.
    #[failure(
        code = RUN_REFUSED,
        hint = "choose the command and its values from the tool's commands list",
        field(
            path = path.as_str(),
            argument = text_or_null(argument.as_deref()),
            reason = reason.name(),
        ),
    )]
    Refused {
        /// The command's path, such as `books list`.
        path: String,
        /// The argument at fault, or `None` for an unknown path.
        argument: Option<String>,
        /// Why the runner refused the request.
        reason: Reason,
    },
    /// The command may destroy data, or declares no effects, and runs only
    /// through a preview and a confirmation.
    #[failure(
        code = RUN_NEEDS_CONFIRMATION,
        hint = "preview the command, then confirm it with the id that the preview returns",
        field(path = path.as_str()),
    )]
    NeedsConfirmation {
        /// The command's path.
        path: String,
    },
    /// The runner is read-only, and the command is not.
    #[failure(
        code = RUN_READ_ONLY,
        hint = "start flightdeck without --read-only to run this command",
        field(path = path.as_str()),
    )]
    ReadOnly {
        /// The command's path.
        path: String,
    },
    /// The confirmation id does not confirm this request.
    #[failure(
        code = RUN_CONFIRMATION,
        hint = "preview the command again for a new confirmation id",
        field(reason = problem.name()),
    )]
    Confirmation {
        /// What is wrong with the id.
        problem: ConfirmationProblem,
    },
    /// The run ended without a document that explains its exit.
    ///
    /// Its sentence and its hint name only the command's path, because they
    /// may reach a log. The line, with every value, is a field for the page.
    #[failure(
        code = RUN_NO_DOCUMENT,
        hint = "run the command line of the line field in a terminal to see what it prints",
        field(
            path = path.as_str(),
            line = line.as_str(),
            exit = exit_code(*exit),
            meaning = text_or_null(meaning.as_deref()),
            stderr = FieldValue::list(stderr.iter().map(String::as_str)),
        ),
    )]
    NoDocument {
        /// The command's path, such as `books remove`.
        path: String,
        /// The command line that ran, with `--dry-run` for a dry run.
        line: String,
        /// How the child ended.
        exit: Exit,
        /// The meaning that the tool declares for the exit code.
        meaning: Option<String>,
        /// The last lines of the child's standard error.
        stderr: Vec<String>,
    },
    /// The program would not start, or flightdeck could not follow it.
    #[failure(
        code = RUN_SPAWN,
        hint = "check that {tool} exists and runs",
        field(tool = tool.as_str()),
    )]
    Spawn {
        /// The tool's name.
        tool: String,
        /// What went wrong.
        source: SpawnError,
    },
    /// The run's standard output passed a limit, so flightdeck stopped it.
    #[failure(
        code = RUN_OUTPUT_LIMIT,
        hint = "narrow the command with its arguments, such as a smaller --limit",
        field(limit = limit_name(*limit), bytes = *bytes),
    )]
    OutputLimit {
        /// The limit that the output passed.
        limit: Limit,
        /// The limit in bytes.
        bytes: usize,
    },
    /// The most runs are running.
    #[failure(code = RUN_BUSY, recovery = Recovery::Retry, field(most = *most))]
    Busy {
        /// The most runs at once.
        most: usize,
    },
    /// No run with the id is running.
    #[failure(
        code = RUN_NOT_RUNNING,
        recovery = Recovery::None { reason: "the run has already ended".to_owned() },
        field(run = run.map_or(FieldValue::Null, |run| FieldValue::from(run.0))),
    )]
    NotRunning {
        /// The run's id, or `None` for text that is no run id.
        run: Option<RunId>,
    },
    /// The runner has shut down, and starts no more runs.
    #[failure(
        code = RUN_CLOSED,
        recovery = Recovery::None { reason: "flightdeck is shutting down".to_owned() },
    )]
    Closed,
    /// The system gave no random bytes for an id.
    #[failure(code = RUN_RANDOM, recovery = Recovery::Retry)]
    Random {
        /// Why the system gave none.
        source: getrandom::Error,
    },
}

impl RunFailure {
    /// Returns the refusal of `argument` of the command at `path` for
    /// `reason`.
    pub(super) fn refused(path: String, argument: Option<&str>, reason: Reason) -> Self {
        Self::Refused {
            path,
            argument: argument.map(str::to_owned),
            reason,
        }
    }

    /// Returns the failure of a run of the command at `path` as `line` that
    /// ended with `exit` and no document, with the declared `meaning` of its
    /// exit and the last lines of `stderr`.
    pub(super) fn no_document(
        (path, line): (String, String),
        exit: Exit,
        meaning: Option<String>,
        stderr: &[u8],
    ) -> Self {
        Self::NoDocument {
            path,
            line,
            exit,
            meaning,
            stderr: last_lines(stderr),
        }
    }
}

/// Prints a sentence for each refusal and failure, such as
/// `books list takes no argument --from`.
impl fmt::Display for RunFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused {
                path,
                argument,
                reason,
            } => refusal(f, path, argument.as_deref().unwrap_or_default(), reason),
            Self::NeedsConfirmation { path } => write!(
                f,
                "{path} may destroy data, so it runs only after a preview and a confirmation"
            ),
            Self::ReadOnly { path } => write!(
                f,
                "flightdeck is read-only, and {path} is not a read-only command"
            ),
            Self::Confirmation { problem } => f.write_str(match problem {
                ConfirmationProblem::Unknown => "the confirmation id is unknown or already used",
                ConfirmationProblem::Expired => "the confirmation id has expired",
                ConfirmationProblem::Mismatch => {
                    "the confirmation id was issued for another command line"
                }
            }),
            Self::NoDocument { path, exit, .. } => match exit {
                Exit::Code(code) => {
                    write!(f, "{path} exited with {code} and printed no document")
                }
                Exit::Signal | Exit::TimedOut | Exit::Unknown => {
                    write!(f, "{path} was ended by a signal and printed no document")
                }
            },
            Self::Spawn { tool, .. } => write!(f, "cannot run {tool}"),
            Self::OutputLimit { limit, bytes } => match limit {
                Limit::Line => write!(f, "the command printed a line longer than {bytes} bytes"),
                Limit::Total => write!(f, "the command printed more than {bytes} bytes"),
            },
            Self::Busy { most } => write!(f, "{most} runs are running, which is the most at once"),
            Self::NotRunning { run: Some(run) } => write!(f, "run {} is not running", run.0),
            Self::NotRunning { run: None } => f.write_str("no run has that id"),
            Self::Closed => f.write_str("flightdeck is shutting down and starts no more runs"),
            Self::Random { .. } => f.write_str("the system gave no random bytes"),
        }
    }
}

/// Writes the sentence of a refusal of `argument` of `path`.
fn refusal(f: &mut fmt::Formatter<'_>, path: &str, argument: &str, reason: &Reason) -> fmt::Result {
    // The path of an unknown command and the name of an unknown argument
    // are what the page sent, unchecked, so the sentences leave them out.
    match reason {
        Reason::UnknownCommand => f.write_str("the tool lists no runnable command at that path"),
        Reason::UnknownArgument => write!(f, "{path} takes no argument of that name"),
        Reason::Reserved => write!(f, "flightdeck sets {argument} of {path} itself"),
        Reason::Arity { takes } => write!(f, "{argument} of {path} takes {}", takes.words()),
        // The value may be anything the page sent, so the sentence leaves it
        // out.
        Reason::NotPossible => {
            write!(
                f,
                "{argument} of {path} does not take the value it was sent"
            )
        }
        Reason::Control => write!(
            f,
            "a value of {argument} of {path} holds a control character, which the command line cannot show"
        ),
        Reason::Skipped { earlier } => write!(
            f,
            "{argument} of {path} takes a value only when {earlier} has one"
        ),
        Reason::Grouped => write!(
            f,
            "flightdeck cannot set {argument} of {path}, which takes several values at once"
        ),
    }
}

impl std::error::Error for RunFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn { source, .. } => Some(source),
            Self::Random { source } => Some(source),
            _ => None,
        }
    }
}

/// Why the runner refused a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The tool lists no runnable command at the path. A hidden command is
    /// never listed.
    UnknownCommand,
    /// The command takes no argument of that name.
    UnknownArgument,
    /// The argument is one of crumbkit's own flags, which flightdeck sets
    /// itself. The page may send only `--limit` and `--all`.
    Reserved,
    /// The value is not of the kind that the argument takes.
    Arity {
        /// What the argument takes.
        takes: Takes,
    },
    /// The value is not one of the argument's possible values. The reason
    /// leaves the value out, because it may be anything the page sent.
    NotPossible,
    /// A value holds a control character, such as a NUL byte, a newline or
    /// an escape, or a bidirectional control. A NUL byte reaches no
    /// program, and the others would make the display line show something
    /// other than what runs.
    Control,
    /// A positional value follows a positional argument without one, which
    /// the tool would read as the earlier argument's value.
    Skipped {
        /// The earlier positional argument, which has no value.
        earlier: String,
    },
    /// The argument takes several values in one occurrence.
    ///
    /// RUST-10: the runner writes each option as one `--name=value` word,
    /// which holds one value, so it refuses such an argument. crumbkit's
    /// tools have none. Writing the name and each value as separate words
    /// replaces this when a tool needs one.
    Grouped,
}

impl Reason {
    /// Returns the reason's name in a failure's fields, such as
    /// `unknown_argument`.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::UnknownCommand => "unknown_command",
            Self::UnknownArgument => "unknown_argument",
            Self::Reserved => "reserved",
            Self::Arity { .. } => "arity",
            Self::NotPossible => "not_possible",
            Self::Control => "control",
            Self::Skipped { .. } => "skipped",
            Self::Grouped => "grouped",
        }
    }
}

/// What kind of value an argument takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Takes {
    /// `true` or `false`.
    Flag,
    /// One string.
    One,
    /// A list of strings.
    Many,
}

impl Takes {
    /// Returns the words of a refusal, such as `one value`.
    const fn words(self) -> &'static str {
        match self {
            Self::Flag => "true or false",
            Self::One => "one value",
            Self::Many => "a list of values",
        }
    }
}

/// What is wrong with a confirmation id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmationProblem {
    /// The runner never issued the id, or it was already used.
    Unknown,
    /// The id is older than its life.
    Expired,
    /// The id was issued for another vector.
    Mismatch,
}

impl ConfirmationProblem {
    /// Returns the problem's name in a failure's fields, such as `expired`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Expired => "expired",
            Self::Mismatch => "mismatch",
        }
    }
}

/// Returns `text` as a field's value, or null.
fn text_or_null(text: Option<&str>) -> FieldValue<'_> {
    text.map_or(FieldValue::Null, FieldValue::from)
}

/// Returns the limit's name in a failure's fields.
const fn limit_name(limit: Limit) -> &'static str {
    match limit {
        Limit::Line => "line",
        Limit::Total => "total",
    }
}
