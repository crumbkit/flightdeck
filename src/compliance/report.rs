//! The reader of Board's `rules/report/v1` document, which
//! `board rules report` prints in each repository's CI run.
//!
//! The types copy only the fields the build uses. They read every field
//! that is an open enum in Board, such as a rule's `result`, as text, so a
//! value that a later Board adds reaches the state table as unknown and
//! never fails the whole report. A field of the wrong type does fail it,
//! and [`read`] names the field's path.

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The `$type` a report must hold.
pub(crate) const REPORT_TYPE: &str = "rules/report/v1";

/// The most bytes of a report that the build reads. A report holds lists
/// cut at 20 items, so a real one is far smaller.
const REPORT_LIMIT: u64 = 16 * 1024 * 1024;

/// One repository's `rules/report/v1` document.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Report {
    /// When `board rules report` ran.
    pub(crate) generated: jiff::Timestamp,
    /// The repository the report measured.
    pub(crate) repository: RepositoryRead,
    /// The Board that made the report and the board its rules came from.
    pub(crate) board: BoardRead,
    /// The GitHub Actions run that made the report, or null.
    pub(crate) source_run: Option<SourceRun>,
    /// The imposed rule set.
    pub(crate) imposed: ImposedRead,
    /// Each rule that reaches the repository.
    pub(crate) rules: Vec<RuleRow>,
    /// Each group and the state its members roll up to.
    pub(crate) groups: Vec<GroupRow>,
    /// Each compliance suite that names the repository.
    pub(crate) suites: Vec<SuiteRow>,
    /// Where the repository's own rules differ from the imposed set.
    pub(crate) drift: DriftRead,
    /// Each refusal and warning met while reading.
    pub(crate) refused: Vec<Refused>,
}

/// The repository a report measured.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RepositoryRead {
    /// The commit the report read, or null.
    pub(crate) commit: Option<String>,
    /// When that commit was made, or null.
    pub(crate) committed: Option<String>,
    /// Whether a tracked file differed from the commit.
    pub(crate) uncommitted: bool,
}

/// The Board that made a report and the board holding its rules.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct BoardRead {
    /// Board's version.
    pub(crate) version: String,
    /// The rules board's origin in identity form, or null.
    pub(crate) rules_from: Option<String>,
    /// The commit HEAD names in the rules board, or null.
    pub(crate) rules_commit: Option<String>,
}

/// The GitHub Actions run that made a report. The document copies it as it
/// is.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct SourceRun {
    /// The repository whose workflow ran, from `GITHUB_REPOSITORY`.
    pub repository: String,
    /// The run's id, from `GITHUB_RUN_ID`.
    pub run_id: u64,
    /// The run's number in its workflow, from `GITHUB_RUN_NUMBER`.
    pub run_number: u64,
    /// The commit the run checked out, from `GITHUB_SHA`.
    pub commit: String,
}

/// The imposed rule set a report compared the repository's rules with.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ImposedRead {
    /// The set's root, such as `/rules`.
    pub(crate) root: String,
}

/// One rule that reaches a repository, as a report holds it.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RuleRow {
    /// The rule's path.
    pub(crate) id: String,
    /// `imposed` or `repository`.
    pub(crate) source: String,
    /// The paths of the groups the rule is in.
    pub(crate) groups: Vec<String>,
    /// What the files must do, or null when the rule cannot be read.
    pub(crate) requirement: Option<String>,
    /// Why the requirement exists, or null.
    pub(crate) purpose: Option<String>,
    /// `standing` or `obligation`.
    pub(crate) kind: String,
    /// When the rule is shown to an agent.
    pub(crate) show: String,
    /// What a program checks, or null.
    pub(crate) checker: Option<serde_json::Value>,
    /// What the rule found.
    pub(crate) result: String,
    /// Why the result is `unreadable`, or null.
    pub(crate) code: Option<String>,
    /// The facts the rule's checker measured.
    #[serde(flatten)]
    pub(crate) facts: RuleFacts,
}

/// The facts a rule's checker measured, which a cell copies.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct RuleFacts {
    /// What a budget's files use, or null for any other rule.
    pub used: Option<u64>,
    /// A budget's max, or null for any other rule.
    pub max: Option<u64>,
    /// The files a budget or version rule read, at most 20.
    pub files: Vec<String>,
    /// Each file over a per-file budget, at most 20.
    pub over: Vec<FileOver>,
    /// Each pin older than a version rule's minimum, at most 20.
    pub behind: Vec<PinBehind>,
    /// How many items each list above left out.
    pub omitted: Omitted,
    /// What a zero rule's suite's latest run reported, or null.
    pub zero: Option<ZeroRead>,
}

/// One file over a per-file budget.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct FileOver {
    /// The file, from the repository's root.
    pub file: String,
    /// What it uses.
    pub used: u64,
}

/// One pin older than a version rule's minimum.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PinBehind {
    /// The file, from the repository's root.
    pub file: String,
    /// The line the pin is on, from 1.
    pub line: u64,
    /// The version pinned.
    pub found: String,
}

/// How many items each list of a rule's facts left out.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct Omitted {
    /// Files left out of `files`.
    pub files: u64,
    /// Files left out of `over`.
    pub over: u64,
    /// Pins left out of `behind`.
    pub behind: u64,
}

/// What a zero rule's suite's latest run reported.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct ZeroRead {
    /// The suite, by its logical path.
    pub suite: String,
    /// The run, by its logical path, or null when none qualifies.
    pub run: Option<String>,
    /// Whole seconds from the run's finish to the report, or null.
    pub age_s: Option<u64>,
    /// Each measure the rule names and the number the run recorded.
    pub measures: BTreeMap<String, Option<f64>>,
}

/// One group, as a report rolls it up.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct GroupRow {
    /// The group's path.
    pub(crate) id: String,
    /// The group's name for people.
    pub(crate) title: String,
    /// How many of the group's members found each result.
    pub(crate) counts: BTreeMap<String, u64>,
}

/// One compliance suite that names the repository.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SuiteRow {
    /// The suite, by its logical path.
    pub(crate) id: String,
    /// How many cases the suite lists, or null.
    pub(crate) cases: Option<u64>,
    /// The newest qualifying run, or null.
    pub(crate) run: Option<SuiteRun>,
    /// Why `runs.latest` refused, or null.
    pub(crate) code: Option<String>,
}

/// A suite's newest qualifying run. A cell copies it as it is.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct SuiteRun {
    /// The run, by its logical path.
    pub id: String,
    /// When the run finished.
    pub finished: String,
    /// The commit the run's `versions` name, or null.
    pub commit: Option<String>,
    /// `pass` or `fail`, or a later outcome.
    pub outcome: String,
    /// Each case that failed or erred.
    pub failed: Vec<FailedCase>,
    /// Commits on the repository since the run's commit, or null when Board
    /// could not count them.
    pub commits_since: Option<u64>,
    /// Why `commits_since` is null, or null when it was counted.
    pub commits_since_code: Option<String>,
}

/// One case that failed or erred.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct FailedCase {
    /// The case's id.
    pub case: String,
    /// The run's note on the case, or null.
    pub note: Option<String>,
}

/// Where a repository's own rules differ from the imposed set.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct DriftRead {
    /// The repository's own rules with no imposed rule of the same path.
    pub(crate) extra: Vec<String>,
    /// The imposed rules the repository also holds with different fields.
    pub(crate) differs: Vec<Differs>,
    /// The imposed rules that reach no file in the repository.
    pub(crate) not_reached: Vec<String>,
}

/// One imposed rule that the repository holds with different fields.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct Differs {
    /// The imposed rule's path.
    pub id: String,
    /// The fields whose values differ.
    pub fields: Vec<String>,
}

/// One refusal or warning a report met.
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct Refused {
    /// Board's or the plugin's code.
    pub code: String,
    /// The rules or groups it concerns, by path.
    pub rules: Vec<String>,
    /// What happened, for people.
    pub message: String,
}

/// Why a repository's column has no report to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Unread {
    /// No file is at the report's path.
    Missing,
    /// The file is there and is not a report this build reads.
    Invalid {
        /// The path of the field that is wrong, such as `rules[0].result`,
        /// or `None` when the file is not JSON or cannot be read.
        field: Option<String>,
        /// What is wrong.
        message: String,
    },
}

/// Reads the report at `path`.
///
/// # Errors
///
/// Returns [`Unread::Missing`] when no file is at `path`, and
/// [`Unread::Invalid`] when the file cannot be read, is larger than the
/// build reads, is not JSON, holds another `$type` or holds a field of the
/// wrong type.
pub(crate) fn read(path: &Path) -> Result<Report, Unread> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(Unread::Missing),
        Err(error) => {
            return Err(Unread::Invalid {
                field: None,
                message: format!("the report cannot be read: {error}"),
            });
        }
    };
    let mut bytes = Vec::new();
    file.take(REPORT_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Unread::Invalid {
            field: None,
            message: format!("the report cannot be read: {error}"),
        })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > REPORT_LIMIT {
        return Err(Unread::Invalid {
            field: None,
            message: format!("the report is larger than {REPORT_LIMIT} bytes"),
        });
    }
    parse(&bytes)
}

/// Reads a report from `bytes`.
///
/// # Errors
///
/// Returns [`Unread::Invalid`] as [`read`] does.
pub(crate) fn parse(bytes: &[u8]) -> Result<Report, Unread> {
    /// The one field read first, so another document's type is named
    /// before its other fields are.
    #[derive(Deserialize)]
    struct Head {
        #[serde(rename = "$type")]
        kind: Option<serde_json::Value>,
    }
    let head: Head = serde_json::from_slice(bytes).map_err(|error| Unread::Invalid {
        field: None,
        message: format!("the report is not a JSON object: {error}"),
    })?;
    if head.kind.as_ref().and_then(serde_json::Value::as_str) != Some(REPORT_TYPE) {
        return Err(Unread::Invalid {
            field: Some("$type".to_owned()),
            message: format!(
                "$type is {}, and flightdeck reads {REPORT_TYPE}",
                head.kind
                    .map_or_else(|| "missing".to_owned(), |kind| kind.to_string())
            ),
        });
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
        let field = error.path().to_string();
        Unread::Invalid {
            message: error.into_inner().to_string(),
            field: Some(field),
        }
    })
}
