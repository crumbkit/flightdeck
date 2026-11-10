//! The refusals of a run before it starts, and the codes of every refusal
//! and failure of a run.

use std::fmt;

use crumbkit::{Code, Failure, FieldValue, Recovery};

use super::RunId;
use crate::spawn::SpawnError;

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

/// The code of a run asked to start in a directory that cannot be used.
pub const RUN_DIRECTORY: Code = Code::new("flightdeck", "run", "directory");

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

/// The code of a choices source that ended without a listing to read.
pub const CHOICES_NO_LISTING: Code = Code::new("flightdeck", "choices", "no_listing");

/// The code of a choices source that had not ended within the choices
/// time.
pub const CHOICES_TIME_LIMIT: Code = Code::new("flightdeck", "choices", "time_limit");

/// A run that the runner refused, or that could not start, and a choices
/// run that gave no choices. How a started run fails is an
/// [`EndFailure`](super::EndFailure).
#[derive(Debug, Failure)]
pub enum RunFailure {
    /// The tool's model does not allow the request.
    #[failure(
        code = RUN_REFUSED,
        hint = "choose the command and its values from the tool's commands list",
        field(
            path = FieldValue::list(path.iter().map(String::as_str)),
            argument = text_or_null(argument.as_deref()),
            reason = reason.name(),
            index = reason.index().map_or(FieldValue::Null, FieldValue::from),
            takes = reason.takes().map_or(FieldValue::Null, |takes| FieldValue::from(takes.name())),
            earlier = text_or_null(reason.earlier()),
        ),
    )]
    Refused {
        /// The command's path as the page sent it, one word at a time, such
        /// as `["books", "list"]`.
        path: Vec<String>,
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
    /// The request's directory cannot be used.
    #[failure(
        code = RUN_DIRECTORY,
        hint = "choose an absolute path to a directory that flightdeck can enter",
        field(reason = problem.name()),
    )]
    Directory {
        /// What is wrong with the directory.
        problem: DirectoryProblem,
    },
    /// The program would not start.
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
    /// The source of a choices run ended without a listing to read.
    #[failure(
        code = CHOICES_NO_LISTING,
        hint = "type the value, or run {path} to see why it fails",
        field(path = path.as_str(), outcome = *outcome),
    )]
    NoListing {
        /// The source's path.
        path: String,
        /// How the source ended: the kind of its outcome, or `not_listing`
        /// for a result that is not one listing.
        outcome: &'static str,
    },
    /// The source of a choices run had not ended within the choices time,
    /// and the runner stopped it.
    #[failure(
        code = CHOICES_TIME_LIMIT,
        hint = "type the value, or run {path} to list every row",
        field(path = path.as_str()),
    )]
    ChoicesTime {
        /// The source's path.
        path: String,
    },
}

impl RunFailure {
    /// Returns the refusal of `argument` of the command at `path` for
    /// `reason`.
    pub(super) fn refused(path: &[String], argument: Option<&str>, reason: Reason) -> Self {
        Self::Refused {
            path: path.to_vec(),
            argument: argument.map(str::to_owned),
            reason,
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
            } => refusal(
                f,
                &path.join(" "),
                argument.as_deref().unwrap_or_default(),
                reason,
            ),
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
            // The directory is what the page sent, unchecked, so the
            // sentences leave it out.
            Self::Directory { problem } => f.write_str(match problem {
                DirectoryProblem::NotAbsolute => "the working directory is not an absolute path",
                DirectoryProblem::Control => "the working directory holds a control character",
                DirectoryProblem::NotADirectory => {
                    "the working directory does not exist, is not a directory or cannot be entered"
                }
            }),
            Self::Spawn { tool, .. } => write!(f, "cannot run {tool}"),
            Self::Busy { most } => write!(f, "{most} runs are running, which is the most at once"),
            Self::NotRunning { run: Some(run) } => write!(f, "run {} is not running", run.0),
            Self::NotRunning { run: None } => f.write_str("no run has that id"),
            Self::Closed => f.write_str("flightdeck is shutting down and starts no more runs"),
            Self::Random { .. } => f.write_str("the system gave no random bytes"),
            Self::NoListing { path, .. } => {
                write!(f, "{path} ended without a listing of choices")
            }
            Self::ChoicesTime { path } => {
                write!(f, "{path} did not list its choices in time")
            }
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
        Reason::NotPossible { .. } => {
            write!(
                f,
                "{argument} of {path} does not take the value it was sent"
            )
        }
        Reason::Control { .. } => write!(
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
        // The page sent the source, target and argument unchecked, so the
        // sentence leaves them out.
        Reason::NotInferred => {
            f.write_str("the tool's model infers no link from that command to that argument")
        }
        Reason::NotReadOnly => write!(
            f,
            "{path} is not read-only, so flightdeck does not run it to list choices"
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
    NotPossible {
        /// Which item of a list value, counted from 0, or `None` for an
        /// argument that takes one value.
        index: Option<usize>,
    },
    /// A value holds a control character, such as a NUL byte, a newline or
    /// an escape, or a bidirectional control. A NUL byte reaches no
    /// program, and the others would make the display line show something
    /// other than what runs.
    Control {
        /// Which item of a list value, counted from 0, or `None` for an
        /// argument that takes one value.
        index: Option<usize>,
    },
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
    /// The model inferred no link from the source command to the target's
    /// argument, so the runner does not run the source for choices.
    NotInferred,
    /// The source of the choices is not read-only.
    NotReadOnly,
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
            Self::NotPossible { .. } => "not_possible",
            Self::Control { .. } => "control",
            Self::Skipped { .. } => "skipped",
            Self::Grouped => "grouped",
            Self::NotInferred => "not_inferred",
            Self::NotReadOnly => "not_read_only",
        }
    }

    /// Returns which item of a list value the reason is about, for the
    /// `index` field.
    #[must_use]
    pub const fn index(&self) -> Option<usize> {
        match self {
            Self::NotPossible { index } | Self::Control { index } => *index,
            _ => None,
        }
    }

    /// Returns what the argument takes, for the `takes` field of an arity
    /// refusal.
    #[must_use]
    pub const fn takes(&self) -> Option<Takes> {
        match self {
            Self::Arity { takes } => Some(*takes),
            _ => None,
        }
    }

    /// Returns the earlier positional argument without a value, for the
    /// `earlier` field of a skipped positional.
    #[must_use]
    pub fn earlier(&self) -> Option<&str> {
        match self {
            Self::Skipped { earlier } => Some(earlier),
            _ => None,
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
    /// Returns the name in a failure's `takes` field, such as `one`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Flag => "flag",
            Self::One => "one",
            Self::Many => "many",
        }
    }

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

/// What is wrong with a request's working directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectoryProblem {
    /// The path is not absolute, so it would depend on where flightdeck
    /// started.
    NotAbsolute,
    /// The path holds a control character, which the run's card could not
    /// show as it is.
    Control,
    /// Nothing at the path is a directory that flightdeck can enter.
    NotADirectory,
}

impl DirectoryProblem {
    /// Returns the problem's name in a failure's fields, such as
    /// `not_absolute`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotAbsolute => "not_absolute",
            Self::Control => "control",
            Self::NotADirectory => "not_a_directory",
        }
    }
}

/// Returns `text` as a field's value, or null.
pub(super) fn text_or_null(text: Option<&str>) -> FieldValue<'_> {
    text.map_or(FieldValue::Null, FieldValue::from)
}
