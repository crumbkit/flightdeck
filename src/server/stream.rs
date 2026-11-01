//! The stream of JSON lines that a run route answers with: `started`, a
//! `line` for each line that the tool prints, `alive` during silence, and
//! `ended`, which is always the last line of a stream that runs a command.

use std::io;
use std::net::TcpStream;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::document::Document;
use super::http::Body;
use super::{Clock, reply};
use crate::model::ToolFailure;
use crate::runner::{Outcome, Preview, Run, RunEvent, Waited};
use crate::spawn::Exit;

/// The content type of a stream: one JSON object on each line.
const NDJSON: &str = "application/x-ndjson";

/// The line that the server writes after each keepalive interval without
/// output, which the page ignores. A failed write of it tells the server
/// that the client has gone.
const ALIVE: &[u8] = b"{\"event\":\"alive\"}\n";

/// What a run route streams: a run, or a preview with or without its dry
/// run. Each is boxed, so a reply that holds one stays small.
#[derive(Debug)]
pub(super) enum Stream {
    /// A run of `POST /api/runs` or `POST /api/confirmations/<id>`.
    Run(Box<Run>),
    /// A preview of `POST /api/previews`.
    Preview(Box<Preview>),
}

impl Stream {
    /// Waits at most `timeout` for the next event, and returns `None` once
    /// the outcome has been returned, or at once for a preview that runs
    /// nothing.
    fn next_timeout(&mut self, timeout: Duration) -> Option<Waited> {
        match self {
            Self::Run(run) => run.next_timeout(timeout),
            Self::Preview(preview) => preview.next_timeout(timeout),
        }
    }

    /// Returns the exit code of the child once the run has ended, as a
    /// shell reports it: the code it exited with, or 128 and the number of
    /// the signal that ended it, such as 130 for SIGINT. It returns `None`
    /// when flightdeck could not learn how the child ended.
    fn exit(&self) -> Option<i32> {
        let exit = match self {
            Self::Run(run) => run.exit(),
            Self::Preview(preview) => preview.exit(),
        };
        match exit? {
            Exit::Code(code) => Some(code),
            Exit::Signal(signal) => Some(128_i32.saturating_add(signal)),
            Exit::TimedOut | Exit::Unknown => None,
        }
    }

    /// Returns the `started` record, with `started_at` as RFC 3339 text.
    fn started(&self, started_at: String) -> Record<'_> {
        match self {
            Self::Run(run) => Record::Started {
                run: Some(run.id().to_string()),
                line: run.line(),
                started_at,
                preview: None,
            },
            Self::Preview(preview) => Record::Started {
                run: preview.run_id().map(|id| id.to_string()),
                line: preview.line(),
                started_at,
                preview: Some(PreviewStart {
                    preview: preview.run_id().is_some(),
                    confirmation: preview.confirmation().map(|id| id.to_string()),
                }),
            },
        }
    }

    /// Returns what a preview's `ended` record adds, or `None` for a run.
    fn ended(&self) -> Option<PreviewEnd> {
        match self {
            Self::Run(_) => None,
            Self::Preview(preview) => Some(PreviewEnd {
                confirmation: preview.confirmation().map(|id| id.to_string()),
            }),
        }
    }
}

/// One line of a stream.
#[derive(Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum Record<'a> {
    /// The run has started, or a preview that runs nothing has issued its
    /// id.
    Started {
        /// The id that the cancel route takes, or `None` for a preview that
        /// runs nothing.
        run: Option<String>,
        /// The command line, POSIX-quoted, without the flags that
        /// flightdeck adds.
        line: &'a str,
        /// When the run started, as RFC 3339 text in UTC.
        started_at: String,
        /// What a preview adds.
        #[serde(flatten)]
        preview: Option<PreviewStart>,
    },
    /// One line of the tool's standard output, as it arrived.
    Line {
        /// The line, without its newline.
        text: &'a str,
    },
    /// How the run ended, which is always the last line.
    Ended {
        /// The outcome, tagged by `kind`.
        outcome: Ending<'a>,
        /// The child's exit code as a shell reports it, or `None` when
        /// flightdeck could not learn how the child ended.
        exit: Option<i32>,
        /// How long the run took, in milliseconds.
        duration_ms: u64,
        /// What a preview adds.
        #[serde(flatten)]
        preview: Option<PreviewEnd>,
    },
}

/// What a preview's `started` record adds.
#[derive(Serialize)]
struct PreviewStart {
    /// True when the preview runs the command with `--dry-run`.
    preview: bool,
    /// The confirmation id, which a preview that runs nothing issues at
    /// once, and a dry run issues when it ends with a result.
    confirmation: Option<String>,
}

/// What a preview's `ended` record adds.
#[derive(Serialize)]
struct PreviewEnd {
    /// The confirmation id that the dry run issued, or `None` when it ended
    /// with anything other than a result.
    confirmation: Option<String>,
}

/// A run's outcome as the `ended` record carries it.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Ending<'a> {
    /// The tool printed its result and exited with 0.
    Result,
    /// The tool printed its result with failures.
    ResultWithFailures,
    /// The tool printed its failure document.
    ToolFailure {
        /// The tool's failure document.
        failure: &'a ToolFailure,
    },
    /// The run was cancelled.
    Cancelled,
    /// flightdeck's own failure.
    Failed {
        /// flightdeck's failure document.
        failure: Document<'a>,
    },
}

impl<'a> From<&'a Outcome> for Ending<'a> {
    fn from(outcome: &'a Outcome) -> Self {
        match outcome {
            Outcome::Result => Self::Result,
            Outcome::ResultWithFailures => Self::ResultWithFailures,
            Outcome::ToolFailure(failure) => Self::ToolFailure { failure },
            Outcome::Cancelled => Self::Cancelled,
            Outcome::Failed(failure) => Self::Failed {
                failure: Document(failure),
            },
        }
    }
}

/// How a stream is paced: how long it waits for output before it writes
/// `alive`, how long the client has to read each line, and the deadline
/// that the server sets for every stream once it stops.
pub(super) struct Pacing<'a> {
    /// How long the stream goes without output before it writes `alive`.
    pub(super) keepalive: Duration,
    /// How long the client has to read each line.
    pub(super) write_time: Duration,
    /// The deadline by which every remaining line is written, set once the
    /// server stops.
    pub(super) cut_off: &'a OnceLock<Instant>,
}

impl Pacing<'_> {
    /// Writes `bytes` to `body`, by the server's deadline when it has
    /// stopped.
    fn send(&self, body: &mut Body<'_>, bytes: &[u8]) -> io::Result<()> {
        if let Some(deadline) = self.cut_off.get() {
            body.cut_off(*deadline);
        }
        body.send(bytes)
    }
}

/// Writes `stream` to `connection` as a 200 reply of JSON lines, until its
/// `ended` line. It writes `alive` after each keepalive interval without
/// output. Once the server stops, the remaining lines share one deadline.
///
/// # Errors
///
/// Returns the error of a write, as when the client has closed the
/// connection or stopped reading. The stream is then dropped, which
/// cancels its run.
pub(super) fn write(
    connection: &mut TcpStream,
    mut stream: Stream,
    clock: &dyn Clock,
    pacing: &Pacing<'_>,
) -> io::Result<()> {
    let started = clock.now();
    let started_at = humantime::format_rfc3339_seconds(started.wall()).to_string();
    let mut body = reply::start(connection, 200, NDJSON, None, pacing.write_time)?;
    pacing.send(&mut body, &line(&stream.started(started_at))?)?;
    loop {
        let event = match stream.next_timeout(pacing.keepalive) {
            Some(Waited::Event(event)) => event,
            Some(Waited::Silent) => {
                pacing.send(&mut body, ALIVE)?;
                continue;
            }
            // A preview that runs nothing ends after its `started` line.
            None => return Ok(()),
        };
        let outcome = match event {
            RunEvent::Line(text) => {
                pacing.send(&mut body, &line(&Record::Line { text: &text })?)?;
                continue;
            }
            RunEvent::Outcome(outcome) => outcome,
        };
        let took = clock.now().since(started);
        let ended = Record::Ended {
            outcome: Ending::from(&outcome),
            exit: stream.exit(),
            duration_ms: u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
            preview: stream.ended(),
        };
        return pacing.send(&mut body, &line(&ended)?);
    }
}

/// Returns `record` as one line of JSON with its newline.
fn line(record: &Record<'_>) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(record).map_err(io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}
