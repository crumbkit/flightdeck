//! How a run ended, read from its last line, its exit and why flightdeck
//! stopped it.

use serde_json::Value;

use super::failure::RunFailure;
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
    Failed(RunFailure),
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

    /// Returns the outcome of a run whose last line was `last` and which
    /// ended as `ended`.
    pub(super) fn outcome(&self, last: Option<&str>, ended: Result<Ended, SpawnError>) -> Outcome {
        match ended {
            Ok(ended) => outcome(self, last, ended.exit(), ended.stopped(), ended.stderr()),
            Err(source) => Outcome::Failed(RunFailure::Spawn {
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

/// What the last line of a run holds.
enum Last {
    /// Nothing, or something other than a crumbkit document.
    Nothing,
    /// A document other than the tool's failure.
    Document,
    /// The tool's failure document.
    Failure(Box<ToolFailure>),
}

/// Returns what `line` holds for the tool `tool`. A document is a JSON
/// object with a `schema` string, and the failure document's schema is
/// `<tool>/failure/v1`.
fn last_of(tool: &str, line: Option<&str>) -> Last {
    let Some(Value::Object(document)) = line.and_then(|line| serde_json::from_str(line).ok())
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

/// Returns the outcome of a run of `ending` whose last line was `last`,
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
        return Outcome::Failed(RunFailure::OutputLimit { limit, bytes });
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
                Exit::Signal | Exit::TimedOut | Exit::Unknown => None,
            };
            Outcome::Failed(RunFailure::no_document(
                (ending.path.clone(), ending.line.clone()),
                exit,
                meaning,
                stderr,
            ))
        }
    }
}
