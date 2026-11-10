//! What each command costs the agents that call it, read from the tool's
//! `self figures list` and `self usage`.
//!
//! crumbkit's usage log records each run of a tool. `self figures list`
//! sums the runs of each command into its figures, such as how many failed
//! and the 95th percentile of their wall time, and `self usage` reports the
//! patterns in those runs where callers waste effort, such as a retry after
//! a failure. The page orders palette matches of equal rank by a command's
//! runs and shows the rest on the command's cost card. A tool without the
//! figures, or a call that fails or prints another document, leaves the
//! command with no figures and is no error.
//!
//! Every figure is optional, because a tool on an older crumbkit-usage
//! prints fewer of them, and serde ignores the fields that a newer one adds.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::command::CommandPath;

/// The words of the command that prints the figures.
pub(super) const FIGURES_COMMAND: [&str; 3] = ["self", "figures", "list"];

/// The words of the command that prints the findings.
pub(super) const USAGE_COMMAND: [&str; 2] = ["self", "usage"];

/// The commands that the loader runs to describe the tool, by their path
/// below the tool's name.
///
/// RUST-10: the usage log counts the loader's own runs of these commands
/// beside the person's, one each per `flightdeck open`, so their counts
/// would grow until they outranked the tool's own commands, and their cost
/// cards would show the loader's runs as an agent's. The figures cannot
/// tell the two apart, so these commands keep no runs, no figures and no
/// findings, and the person's own runs of them are missed. The IDs
/// chooser's runs of a link's source, such as `books list`, are counted
/// too, at most once per source each time the page loads, because the page
/// keeps the choices until it reloads. Those counts are kept, because the
/// source is one of the tool's own commands and leaving it out would drop
/// the person's runs of it, so the chooser's runs raise its rank among
/// palette matches of equal text and add to its cost card. A usage log that
/// records which program ran each command, so flightdeck's own runs can be
/// left out, replaces this.
const LOADER_QUERIES: [&str; 5] = [
    "commands list",
    "schemas list",
    "schemas show",
    "self figures list",
    "self usage",
];

/// The figures of one command's runs, as `self figures list` prints them.
/// Each figure is `None` when the tool does not report it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Figures {
    /// The runs that did not print a help page.
    runs: u64,
    /// The runs that printed the command's help page.
    #[serde(default)]
    help_pages: Option<u64>,
    /// The help pages that each run's session read since its last run
    /// that printed no help page, summed over the runs.
    #[serde(default)]
    help_before: Option<u64>,
    /// The runs whose command line the parser refused.
    #[serde(default)]
    refused: Option<u64>,
    /// The runs that failed.
    #[serde(default)]
    failed: Option<u64>,
    /// The 95th percentile of the runs' wall time in milliseconds.
    #[serde(default)]
    p95_ms: Option<u64>,
    /// The 95th percentile of the bytes that the runs wrote to standard
    /// output.
    #[serde(default)]
    p95_out_bytes: Option<u64>,
}

impl Figures {
    /// Returns the runs that did not print a help page.
    #[must_use]
    pub const fn runs(&self) -> u64 {
        self.runs
    }

    /// Returns the runs that printed the command's help page.
    #[must_use]
    pub const fn help_pages(&self) -> Option<u64> {
        self.help_pages
    }

    /// Returns the help pages that sessions read before their runs.
    #[must_use]
    pub const fn help_before(&self) -> Option<u64> {
        self.help_before
    }

    /// Returns the runs whose command line the parser refused.
    #[must_use]
    pub const fn refused(&self) -> Option<u64> {
        self.refused
    }

    /// Returns the runs that failed.
    #[must_use]
    pub const fn failed(&self) -> Option<u64> {
        self.failed
    }

    /// Returns the 95th percentile of the runs' wall time in milliseconds.
    #[must_use]
    pub const fn p95_ms(&self) -> Option<u64> {
        self.p95_ms
    }

    /// Returns the 95th percentile of the bytes the runs printed.
    #[must_use]
    pub const fn p95_out_bytes(&self) -> Option<u64> {
        self.p95_out_bytes
    }
}

/// One finding of `self usage` about a command: a pattern in its runs that
/// suggests where callers waste effort.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Finding {
    /// What the pattern is, such as `retry after failure`.
    finding: String,
    /// The runs that show the pattern.
    runs: u64,
    /// The tool's words for what the pattern suggests.
    suggestion: String,
    /// The code of the failure that the runs retried, for a retry after
    /// failure only.
    #[serde(default)]
    code: Option<String>,
}

impl Finding {
    /// Returns what the pattern is, such as `retry after failure`.
    #[must_use]
    pub fn finding(&self) -> &str {
        &self.finding
    }

    /// Returns the runs that show the pattern.
    #[must_use]
    pub const fn runs(&self) -> u64 {
        self.runs
    }

    /// Returns the tool's words for what the pattern suggests.
    #[must_use]
    pub fn suggestion(&self) -> &str {
        &self.suggestion
    }

    /// Returns the code of the failure that the runs retried.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }
}

/// What a command costs the agents that call it: its figures, and the
/// findings of `self usage` about it, or `None` when that call failed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Cost {
    figures: Figures,
    findings: Option<Vec<Finding>>,
}

impl Cost {
    /// Returns the command's figures.
    #[must_use]
    pub const fn figures(&self) -> &Figures {
        &self.figures
    }

    /// Returns the findings about the command, or `None` when the tool's
    /// `self usage` failed or printed another document.
    #[must_use]
    pub fn findings(&self) -> Option<&[Finding]> {
        self.findings.as_deref()
    }
}

/// One row of `self figures list`.
#[derive(Debug, Deserialize)]
struct FigureRow {
    /// The command's path below the tool's name, such as `books list`.
    command: String,
    #[serde(flatten)]
    figures: Figures,
}

/// One row of `self usage`.
#[derive(Debug, Deserialize)]
struct FindingRow {
    /// The command's path below the tool's name, such as `books show`.
    command: String,
    #[serde(flatten)]
    finding: Finding,
}

/// The summary of the runs that a report read.
#[derive(Debug, Default, Deserialize)]
struct Summary {
    /// The day the report's window starts, such as `2026-11-03`.
    #[serde(default)]
    since: Option<String>,
}

/// A document of rows that `self figures list` or `self usage` prints.
#[derive(Debug, Deserialize)]
struct Report<T> {
    schema: String,
    data: Vec<T>,
    #[serde(default)]
    summary: Summary,
}

impl<T: for<'de> Deserialize<'de>> Report<T> {
    /// Reads `stdout` as the document whose id ends with `/<name>/v1`.
    fn read(stdout: &[u8], name: &str) -> Option<Self> {
        let report: Self = serde_json::from_slice(stdout).ok()?;
        report
            .schema
            .ends_with(&format!("/{name}/v1"))
            .then_some(report)
    }
}

/// The figures of each command, by its path, and the day they start.
#[derive(Debug, Default)]
pub(super) struct FiguresList {
    by_command: HashMap<String, Figures>,
    since: Option<String>,
}

impl FiguresList {
    /// Reads the standard output of `self figures list`, or returns `None`
    /// when it is not that document. The loader's own queries are left
    /// out.
    pub(super) fn read(stdout: &[u8]) -> Option<Self> {
        let report = Report::<FigureRow>::read(stdout, "self-figures-list")?;
        Some(Self {
            by_command: report
                .data
                .into_iter()
                .filter(|row| !is_loader_query(&row.command))
                .map(|row| (row.command, row.figures))
                .collect(),
            since: report.summary.since.filter(|since| is_day(since)),
        })
    }

    /// Returns the day the figures' window starts, such as `2026-11-03`.
    pub(super) fn since(&self) -> Option<&str> {
        self.since.as_deref()
    }

    /// Returns the cost of the command at `path`, with the findings about
    /// it, or `None` when the figures hold no row for it.
    pub(super) fn cost(&self, path: &CommandPath, usage: Option<&Usage>) -> Option<Cost> {
        let path = path.to_string();
        let figures = self.by_command.get(&path)?.clone();
        Some(Cost {
            figures,
            findings: usage.map(|usage| usage.about(&path)),
        })
    }
}

/// The findings of `self usage`, by the path of their command.
#[derive(Debug, Default)]
pub(super) struct Usage {
    by_command: HashMap<String, Vec<Finding>>,
}

impl Usage {
    /// Reads the standard output of `self usage`, or returns `None` when it
    /// is not that document. Findings about the loader's own queries are
    /// left out.
    pub(super) fn read(stdout: &[u8]) -> Option<Self> {
        let report = Report::<FindingRow>::read(stdout, "self-usage")?;
        let mut by_command: HashMap<String, Vec<Finding>> = HashMap::new();
        for row in report.data {
            if !is_loader_query(&row.command) {
                by_command.entry(row.command).or_default().push(row.finding);
            }
        }
        Some(Self { by_command })
    }

    /// Returns the findings about the command at `path`, in the order the
    /// tool printed them.
    fn about(&self, path: &str) -> Vec<Finding> {
        self.by_command.get(path).cloned().unwrap_or_default()
    }
}

/// Returns true when `command` is one of the loader's own queries.
fn is_loader_query(command: &str) -> bool {
    LOADER_QUERIES.contains(&command)
}

/// Returns true when `text` is a day written as `YYYY-MM-DD`.
fn is_day(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}
