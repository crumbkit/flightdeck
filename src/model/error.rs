//! The failures of loading a tool's model.

use std::fmt;
use std::io;

use crumbkit::{Code, Failure, FieldValue, Recovery};

use crate::spawn::Exit;

/// The code of a tool that does not exist or will not start.
pub const TOOL_NOT_FOUND: Code = Code::new("flightdeck", "tool", "not_found");

/// The code of a tool whose description flightdeck cannot read.
pub const TOOL_LOAD: Code = Code::new("flightdeck", "tool", "load");

/// The most lines at the end of standard error that a failure names.
const STDERR_LINES: usize = 40;

/// A tool whose model flightdeck cannot load.
#[derive(Debug, Failure)]
pub enum LoadError {
    /// The name or path does not resolve to a program, or the program will
    /// not start.
    #[failure(
        code = TOOL_NOT_FOUND,
        hint = "give the name of a program on PATH",
        alternative = Recovery::hint("give the path of a program"),
        field(tool = tool.as_str()),
    )]
    NotFound {
        /// The name or path as the person gave it.
        tool: String,
        /// Why the program did not start.
        source: io::Error,
    },
    /// The temporary state directory of the description's queries cannot
    /// be made, so no query ran.
    #[failure(
        code = TOOL_LOAD,
        hint = "free space in the system's temporary directory and run flightdeck again",
        field(tool = tool.as_str()),
    )]
    State {
        /// The name or path as the person gave it.
        tool: String,
        /// Why the directory cannot be made.
        source: io::Error,
    },
    /// A query of the tool's description failed.
    #[failure(
        code = TOOL_LOAD,
        hint = "run {tool} {command} to see what it prints",
        field(
            tool = tool.as_str(),
            command = command.as_str(),
            problem = problem.name(),
            exit = exit_code(*exit),
            stderr = FieldValue::list(stderr.iter().map(String::as_str)),
        ),
    )]
    Load {
        /// The name or path as the person gave it.
        tool: String,
        /// The query's argument vector, joined by spaces.
        command: String,
        /// What went wrong.
        problem: Problem,
        /// How the query's process ended.
        exit: Exit,
        /// The last lines of the query's standard error.
        stderr: Vec<String>,
    },
}

impl LoadError {
    /// Returns the load failure of the query `command` of `tool`, with the
    /// last lines of `stderr`.
    pub(super) fn load(
        tool: &str,
        command: String,
        problem: Problem,
        exit: Exit,
        stderr: &[u8],
    ) -> Self {
        Self::Load {
            tool: tool.to_owned(),
            command,
            problem,
            exit,
            stderr: last_lines(stderr),
        }
    }
}

/// Prints `cannot start shelf` for a tool that will not start, and names
/// the query and its problem for the others, such as
/// `shelf commands list --json --full --all --no-input exited with 2`.
impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { tool, .. } => write!(f, "cannot start {tool}"),
            Self::State { tool, .. } => {
                write!(
                    f,
                    "cannot make a temporary state directory for {tool}'s queries"
                )
            }
            Self::Load {
                tool,
                command,
                problem,
                exit,
                ..
            } => {
                write!(f, "{tool} {command} ")?;
                match (problem, exit) {
                    (Problem::Exit, Exit::Code(code)) => write!(f, "exited with {code}"),
                    (Problem::Exit, _) => f.write_str("was ended by a signal"),
                    (Problem::Start(_), _) => f.write_str("could not be started"),
                    (Problem::TimeLimit, _) => f.write_str("did not finish within its time limit"),
                    (Problem::StdoutLimit, _) => f.write_str("printed more than flightdeck reads"),
                    (Problem::NotDocument(NotDocument::MissingSchema(id)), _) => {
                        write!(f, "printed no schema for {id}, which schemas list names")
                    }
                    (Problem::NotDocument(NotDocument::Control), _) => {
                        f.write_str("names a command or an argument with a control character")
                    }
                    (Problem::NotDocument(_), _) => {
                        f.write_str("printed something other than the crumbkit document it names")
                    }
                    (Problem::Read(_), _) => f.write_str("could not be followed to its end"),
                }
            }
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotFound { source, .. } | Self::State { source, .. } => Some(source),
            Self::Load { problem, .. } => problem.source(),
        }
    }
}

/// What went wrong with one query of a tool's description.
#[derive(Debug)]
pub enum Problem {
    /// The tool exited with a code other than 0, or a signal ended it.
    Exit,
    /// The program exists and may be run, but starting it failed.
    Start(io::Error),
    /// The tool did not finish within the time limit.
    TimeLimit,
    /// The tool printed more than flightdeck reads.
    StdoutLimit,
    /// The tool printed something other than the document that flightdeck
    /// expects.
    NotDocument(NotDocument),
    /// Waiting for the tool or reading its output failed.
    Read(io::Error),
}

impl Problem {
    /// Returns the problem's name in a failure's fields, such as
    /// `time_limit`.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Exit => "exit",
            Self::Start(_) => "start",
            Self::TimeLimit => "time_limit",
            Self::StdoutLimit => "stdout_limit",
            Self::NotDocument(_) => "not_document",
            Self::Read(_) => "read",
        }
    }

    /// Returns the error under the problem, when it has one.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotDocument(NotDocument::Json(error)) => Some(error),
            Self::Start(error) | Self::Read(error) => Some(error),
            _ => None,
        }
    }
}

/// Why what a tool printed is not the document flightdeck expects.
#[derive(Debug)]
pub enum NotDocument {
    /// The output is not JSON, or not JSON of the document's shape.
    Json(serde_json::Error),
    /// The document's id is not the one the query prints, such as
    /// `shelf/books-list/v1` in place of `shelf/commands-list/v1`.
    Schema(String),
    /// The document leaves out rows, although the query asked for every one.
    Truncated,
    /// `commands list` has no row for the tool itself.
    NoRoot,
    /// `schemas show` printed no schema for this id, which `schemas list`
    /// named.
    MissingSchema(String),
    /// A command word or an argument name holds a control character, which
    /// a command line that flightdeck shows could not show.
    Control,
}

/// Returns the exit code that a failure's `exit` field holds, or null when
/// the process did not exit by itself.
pub(crate) fn exit_code(exit: Exit) -> FieldValue<'static> {
    match exit {
        Exit::Code(code) => FieldValue::from(code),
        Exit::Signal(_) | Exit::TimedOut | Exit::Unknown => FieldValue::Null,
    }
}

/// Returns the last [`STDERR_LINES`] lines of `stderr`, read as UTF-8 with
/// each invalid sequence replaced.
pub(crate) fn last_lines(stderr: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text.lines().collect();
    let skip = lines.len().saturating_sub(STDERR_LINES);
    lines.into_iter().skip(skip).map(str::to_owned).collect()
}
