//! How a run ended, read from its last document, its exit and why
//! flightdeck stopped it.

use serde_json::Value;

use super::end_failure::EndFailure;
use crate::model::{Command, ExitMeaning, Form, Tool, ToolFailure};
use crate::spawn::{Ended, Exit, Limit, Limits, SpawnError, Stop};

#[cfg(test)]
mod tests;

/// How a run ended, which is a run's last event.
#[derive(Debug)]
pub enum Outcome {
    /// The tool printed its result and exited with 0.
    Result,
    /// The tool printed its result, with failures in a batch or problem
    /// rows in a listing, and exited with 1 or a code it declares.
    ResultWithFailures,
    /// The tool printed its failure document.
    ToolFailure(Box<ToolFailure>),
    /// The run was cancelled and the tool printed no failure document, or
    /// the tool exited with 130 after Ctrl-C with no document that explains
    /// how it ended. A cancelled run is cancelled even when its tool printed
    /// its document and exited with 0.
    Cancelled,
    /// flightdeck's own failure: no document, an output limit, or a program
    /// that could not be followed.
    Failed(EndFailure),
}

impl Outcome {
    /// Returns the outcome's kind as the stream's `ended` line tags it,
    /// such as `tool_failure`.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Result => "result",
            Self::ResultWithFailures => "result_with_failures",
            Self::ToolFailure(_) => "tool_failure",
            Self::Cancelled => "cancelled",
            Self::Failed(_) => "failed",
        }
    }
}

/// What the outcome of one run reads beside the run's output: the tool's
/// name, the command's path, the line that ran, the command's form and
/// declared exits, and the limits of the run's output.
#[derive(Debug)]
pub(super) struct Ending {
    tool: String,
    path: String,
    line: String,
    form: Option<Form>,
    exits: Vec<ExitMeaning>,
    limits: Limits,
}

impl Ending {
    /// Returns the ending of a run of `command` of `tool` as `line` within
    /// `limits`.
    pub(super) fn new(tool: &Tool, command: &Command, line: String, limits: Limits) -> Self {
        Self {
            tool: tool.name().to_owned(),
            path: command.path().to_string(),
            line,
            form: command.result().form(),
            exits: command.exits().to_vec(),
            limits,
        }
    }

    /// Returns the outcome of a run whose [`Tail`] is `last` and which
    /// ended as `ended`.
    pub(super) fn outcome(&self, last: Option<&str>, ended: Result<Ended, SpawnError>) -> Outcome {
        match ended {
            Ok(ended) => outcome(self, last, ended.exit(), ended.stopped(), ended.stderr()),
            Err(source) => Outcome::Failed(EndFailure::Spawn {
                tool: self.tool.clone(),
                source,
            }),
        }
    }

    /// Returns the meaning that the command declares for `code`.
    fn meaning(&self, code: i32) -> Option<&str> {
        self.exits
            .iter()
            .find(|exit| i32::from(exit.exit()) <= code && code <= i32::from(exit.last()))
            .map(ExitMeaning::meaning)
    }
}

/// The end of a run's standard output that may hold its last document:
/// the last line that starts with `{`, and every line after it, joined by
/// line feeds. A one-line document is its own line. A document that a
/// crumbkit tool that sets `JsonLayout::Pretty` prints starts with a `{`
/// line, and every line after it is indented or closes it, so the tail
/// holds the whole document. A stream's items are one line each, so the
/// tail is its last line. It is `None` until the run prints a line.
///
/// RUST-10: the tail finds the start of a document from the layout that
/// crumbkit prints, so the runner keeps one document and not the whole
/// output, and needs no knowledge of the tool's layout. It handles a
/// document on one line and one indented as `serde_json::to_writer_pretty`
/// writes it. It misses a document that does not start with `{` at the
/// start of a line, and an indented document whose lines a full queue
/// dropped, after a cancel or a shutdown or after the child exited by
/// itself with the page behind, which then ends without a document. Output with no line that starts with `{` stays in the tail
/// whole, within the run's output limit. A parser that reads every JSON value of standard output
/// whatever its layout replaces it if a tool prints another layout.
#[derive(Debug, Default)]
pub(super) struct Tail(Option<String>);

impl Tail {
    /// Returns the tail that holds `line` alone.
    pub(super) const fn of(line: String) -> Self {
        Self(Some(line))
    }

    /// Adds `line`, the next line of standard output. A line that starts
    /// with `{` starts the tail again.
    pub(super) fn push(&mut self, line: &str) {
        match &mut self.0 {
            Some(tail) if !line.starts_with('{') => {
                tail.push('\n');
                tail.push_str(line);
            }
            _ => self.0 = Some(line.to_owned()),
        }
    }

    /// Returns the tail, or `None` when the run printed no line.
    pub(super) fn get(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

/// What the last document of a run holds.
enum Last {
    /// Nothing, or something other than a crumbkit document.
    Nothing,
    /// A document other than the tool's failure.
    Document,
    /// The tool's failure document.
    Failure(Box<ToolFailure>),
}

/// Returns what `tail`, a run's [`Tail`], holds for the tool `tool`. A
/// document is a JSON object with a `schema` string, and the failure
/// document's schema is `<tool>/failure/v1`.
fn last_of(tool: &str, tail: Option<&str>) -> Last {
    let Some(Value::Object(document)) = tail.and_then(|tail| serde_json::from_str(tail).ok())
    else {
        return Last::Nothing;
    };
    match document.get("schema") {
        Some(Value::String(schema)) if *schema == format!("{tool}/failure/v1") => {
            serde_json::from_value(Value::Object(document))
                .map_or(Last::Nothing, |failure| Last::Failure(Box::new(failure)))
        }
        Some(Value::String(_)) => Last::Document,
        _ => Last::Nothing,
    }
}

/// Returns the outcome of a run of `ending` whose [`Tail`] is `last`,
/// which ended with `exit`, was stopped for `stopped`, and printed `stderr`
/// on standard error.
pub(super) fn outcome(
    ending: &Ending,
    last: Option<&str>,
    exit: Exit,
    stopped: Option<Stop>,
    stderr: &[u8],
) -> Outcome {
    if let Some(Stop::Limit(limit)) = stopped {
        let bytes = match limit {
            Limit::Line => ending.limits.line(),
            Limit::Total => ending.limits.total(),
        };
        return Outcome::Failed(EndFailure::OutputLimit { limit, bytes });
    }
    let declared = |code: i32| code == 1 || ending.meaning(code).is_some();
    match (last_of(&ending.tool, last), exit) {
        (Last::Failure(failure), _) => Outcome::ToolFailure(failure),
        _ if stopped == Some(Stop::Cancel) => Outcome::Cancelled,
        (Last::Document, Exit::Code(0)) => Outcome::Result,
        (Last::Nothing, Exit::Code(0)) if ending.form == Some(Form::Stream) && last.is_none() => {
            Outcome::Result
        }
        (Last::Document, Exit::Code(code)) if declared(code) => Outcome::ResultWithFailures,
        _ if exit == Exit::Code(130) => Outcome::Cancelled,
        _ => {
            let meaning = match exit {
                Exit::Code(code) => ending.meaning(code).map(str::to_owned),
                Exit::Signal(_) | Exit::TimedOut | Exit::Unknown => None,
            };
            Outcome::Failed(EndFailure::no_document(
                (ending.path.clone(), ending.line.clone()),
                exit,
                meaning,
                stderr,
            ))
        }
    }
}
