//! How a started run fails: flightdeck's own failures at a run's end.

use std::fmt;

use crumbkit::{Failure, FieldValue, Recovery};

use super::failure::{RUN_NO_DOCUMENT, RUN_OUTPUT_LIMIT, RUN_RANDOM, RUN_SPAWN, text_or_null};
use crate::model::{exit_code, last_lines};
use crate::spawn::{Exit, Limit, SpawnError};

/// flightdeck's own failure at the end of a started run, which
/// [`Outcome::Failed`](super::Outcome::Failed) carries.
#[derive(Debug, Failure)]
pub enum EndFailure {
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
    /// flightdeck could not follow the program to its end.
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
    /// The system gave no random bytes for the confirmation id that a
    /// successful dry run issues.
    #[failure(code = RUN_RANDOM, recovery = Recovery::Retry)]
    Random {
        /// Why the system gave none.
        source: getrandom::Error,
    },
}

impl EndFailure {
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

/// Prints a sentence for each failure, such as
/// `books remove exited with 2 and printed no document`.
impl fmt::Display for EndFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDocument { path, exit, .. } => match exit {
                Exit::Code(code) => {
                    write!(f, "{path} exited with {code} and printed no document")
                }
                Exit::Signal(_) | Exit::TimedOut | Exit::Unknown => {
                    write!(f, "{path} was ended by a signal and printed no document")
                }
            },
            Self::Spawn { tool, .. } => write!(f, "cannot follow {tool} to its end"),
            Self::OutputLimit { limit, bytes } => match limit {
                Limit::Line => write!(f, "the command printed a line longer than {bytes} bytes"),
                Limit::Total => write!(f, "the command printed more than {bytes} bytes"),
            },
            Self::Random { .. } => f.write_str("the system gave no random bytes"),
        }
    }
}

impl std::error::Error for EndFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spawn { source, .. } => Some(source),
            Self::Random { source } => Some(source),
            Self::NoDocument { .. } | Self::OutputLimit { .. } => None,
        }
    }
}

/// Returns the limit's name in a failure's fields.
const fn limit_name(limit: Limit) -> &'static str {
    match limit {
        Limit::Line => "line",
        Limit::Total => "total",
    }
}
