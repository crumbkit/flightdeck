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

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::command::CommandPath;
use super::day::Day;
use super::documents::{Listing, Summary, is_document};

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

/// The largest count that the page reads exactly, 2^53 - 1, JavaScript's
/// `Number.MAX_SAFE_INTEGER`. A larger figure reads as one the tool does not
/// report, because the page would refuse the whole model for it.
const MOST_SAFE: u64 = 9_007_199_254_740_991;

/// Reads a count that a tool may leave out, print as `null`, or print too
/// large for the page, as `None` in each of those cases. A negative count,
/// a fraction and a count past `u64` read as `None` too.
fn safe_count<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let number = Option::<serde_json::Number>::deserialize(deserializer)?;
    Ok(number
        .as_ref()
        .and_then(serde_json::Number::as_u64)
        .filter(|count| *count <= MOST_SAFE))
}

/// The figures of one command's runs, as `self figures list` prints them.
/// Each figure is `None` when the tool does not report it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Figures {
    /// The runs that did not print a help page.
    runs: u64,
    /// The runs that printed the command's help page.
    help_pages: Option<u64>,
    /// The help pages that each run's session read since its last run
    /// that printed no help page, summed over the runs.
    help_before: Option<u64>,
    /// The runs whose command line the parser refused.
    refused: Option<u64>,
    /// The runs that failed.
    failed: Option<u64>,
    /// The 95th percentile of the runs' wall time in milliseconds.
    p95_ms: Option<u64>,
    /// The 95th percentile of the bytes that the runs wrote to standard
    /// output.
    p95_out_bytes: Option<u64>,
}

/// The figures of one row as the tool prints them, each read by
/// [`safe_count`].
#[derive(Debug, Deserialize)]
struct RawFigures {
    #[serde(default, deserialize_with = "safe_count")]
    runs: Option<u64>,
    #[serde(default, deserialize_with = "safe_count")]
    help_pages: Option<u64>,
    #[serde(default, deserialize_with = "safe_count")]
    help_before: Option<u64>,
    #[serde(default, deserialize_with = "safe_count")]
    refused: Option<u64>,
    #[serde(default, deserialize_with = "safe_count")]
    failed: Option<u64>,
    #[serde(default, deserialize_with = "safe_count")]
    p95_ms: Option<u64>,
    #[serde(default, deserialize_with = "safe_count")]
    p95_out_bytes: Option<u64>,
}

impl RawFigures {
    /// Returns the figures, or `None` when the row has no runs the page
    /// can read, which leaves its command without a cost.
    fn checked(self) -> Option<Figures> {
        Some(Figures {
            runs: self.runs?,
            help_pages: self.help_pages,
            help_before: self.help_before,
            refused: self.refused,
            failed: self.failed,
            p95_ms: self.p95_ms,
            p95_out_bytes: self.p95_out_bytes,
        })
    }
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

/// What a finding of `self usage` is. The tool's words for each kind live
/// here alone, and the page switches on the kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    /// A session ran the command again within 60 seconds of its failure.
    RetryAfterFailure,
    /// A session ran the same command line again within 60 seconds.
    IdenticalRepeat,
    /// The 95th percentile of the command's wall time is at least 1 second.
    SlowCommand,
    /// A session read at least 3 help pages before its first success.
    HelpLoop,
    /// The parser refused the command line.
    UsageError,
    /// The 95th percentile of the command's output is at least 20,000
    /// bytes.
    HeavyOutput,
    /// A kind that flightdeck does not know, such as one that a newer
    /// crumbkit-usage adds.
    Other,
}

impl FindingKind {
    /// Returns the kind that the tool's word `finding` names, such as
    /// `retry after failure`.
    fn of(finding: &str) -> Self {
        match finding {
            "retry after failure" => Self::RetryAfterFailure,
            "identical repeat" => Self::IdenticalRepeat,
            "slow command" => Self::SlowCommand,
            "help loop" => Self::HelpLoop,
            "usage error" => Self::UsageError,
            "heavy output" => Self::HeavyOutput,
            _ => Self::Other,
        }
    }
}

/// One finding of `self usage` about a command: a pattern in its runs that
/// suggests where callers waste effort, with the figures that describe it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Finding {
    kind: FindingKind,
    runs: u64,
    code: Option<String>,
    p95_ms: Option<u64>,
    p95_out_bytes: Option<u64>,
}

impl Finding {
    /// Returns what the pattern is.
    #[must_use]
    pub const fn kind(&self) -> FindingKind {
        self.kind
    }

    /// Returns the runs that show the pattern. A help loop counts the help
    /// pages that its sessions read.
    #[must_use]
    pub const fn runs(&self) -> u64 {
        self.runs
    }

    /// Returns the code of the failure that the runs retried, for a retry
    /// after failure, without the tool's name in front, such as
    /// `books::not_found`.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns the 95th percentile of the command's wall time in
    /// milliseconds, when the tool reports it beside the finding.
    #[must_use]
    pub const fn p95_ms(&self) -> Option<u64> {
        self.p95_ms
    }

    /// Returns the 95th percentile of the bytes the command printed, when
    /// the tool reports it beside the finding.
    #[must_use]
    pub const fn p95_out_bytes(&self) -> Option<u64> {
        self.p95_out_bytes
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
    figures: RawFigures,
}

/// One row of `self usage`.
#[derive(Debug, Deserialize)]
struct FindingRow {
    /// What the pattern is, in the tool's words.
    finding: String,
    #[serde(default, deserialize_with = "safe_count")]
    runs: Option<u64>,
    /// The command's path below the tool's name, such as `books show`.
    command: String,
    #[serde(default)]
    code: Option<String>,
    /// The command's figures, which `--full` prints.
    #[serde(default)]
    figures: Option<FindingFigures>,
}

/// The figures of a finding's command that the page's sentences use.
#[derive(Debug, Default, Deserialize)]
struct FindingFigures {
    #[serde(default, deserialize_with = "safe_count")]
    p95_ms: Option<u64>,
    #[serde(default, deserialize_with = "safe_count")]
    p95_out_bytes: Option<u64>,
}

/// Reads `stdout` as the listing whose id ends with `/<name>/v1`.
fn read_listing<T: DeserializeOwned>(stdout: &[u8], name: &str) -> Option<Listing<T>> {
    let listing: Listing<T> = serde_json::from_slice(stdout).ok()?;
    is_document(listing.schema(), name).then_some(listing)
}

/// The figures of each command, by its path, and the day they start.
#[derive(Debug, Default)]
pub(super) struct FiguresList {
    by_command: HashMap<String, Figures>,
    since: Option<Day>,
}

impl FiguresList {
    /// Reads the standard output of `self figures list`, or returns `None`
    /// when it is not that document. The loader's own queries are left
    /// out.
    pub(super) fn read(stdout: &[u8]) -> Option<Self> {
        let listing = read_listing::<FigureRow>(stdout, "self-figures-list")?;
        let since = listing.summary().and_then(Summary::since);
        Some(Self {
            by_command: listing
                .into_data()
                .into_iter()
                .filter(|row| !is_loader_query(&row.command))
                .filter_map(|row| Some((row.command, row.figures.checked()?)))
                .collect(),
            since,
        })
    }

    /// Returns the day the figures' window starts.
    pub(super) const fn since(&self) -> Option<Day> {
        self.since
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
    /// Reads the standard output of `self usage` from the tool named
    /// `tool`, or returns `None` when it is not that document. Findings
    /// about the loader's own queries are left out, and each code loses the
    /// tool's name in front, so `shelf::books::not_found` reads
    /// `books::not_found`.
    pub(super) fn read(stdout: &[u8], tool: &str) -> Option<Self> {
        let listing = read_listing::<FindingRow>(stdout, "self-usage")?;
        let prefix = format!("{tool}::");
        let mut by_command: HashMap<String, Vec<Finding>> = HashMap::new();
        for row in listing.into_data() {
            // A finding whose count the page cannot read is left out.
            let Some(runs) = row.runs.filter(|_| !is_loader_query(&row.command)) else {
                continue;
            };
            let figures = row.figures.unwrap_or_default();
            let code = row.code.map(|code| match code.strip_prefix(&prefix) {
                Some(rest) => rest.to_owned(),
                None => code,
            });
            by_command.entry(row.command).or_default().push(Finding {
                kind: FindingKind::of(&row.finding),
                runs,
                code,
                p95_ms: figures.p95_ms,
                p95_out_bytes: figures.p95_out_bytes,
            });
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
