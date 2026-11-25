//! The `flightdeck/compliance/v1` document: one column per repository, one
//! row per rule or suite, and each cell's state, built from the reports and
//! the config.
//!
//! Rows join on a rule's path and source, and on a suite's path. A rule
//! that a report does not hold reads absent in that column. A column whose
//! report is missing or invalid reads that code in every cell, and the
//! other columns still render.

use std::collections::BTreeMap;
use std::fmt;

use jiff::Timestamp;
use schemars::JsonSchema;
use serde::Serialize;

use super::config::Config;
use super::drift::Drift;
use super::groups::{self, Group};
use super::report::{Refused, Report, RuleFacts, RuleRow, SourceRun, SuiteRow, SuiteRun, Unread};
use super::state::{self, CellState};

/// The code of a column whose repository has no report file.
pub const NO_REPORT: &str = "flightdeck::compliance::no_report";

/// The code of a column whose report file is not a report the build reads.
pub const INVALID_REPORT: &str = "flightdeck::compliance::invalid_report";

/// The seconds in a day.
const DAY_S: u64 = 24 * 60 * 60;

/// The compliance of every repository the config names, as the pages draw
/// it.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct ComplianceDocument {
    /// The pages' title, from the config.
    pub title: String,
    /// When the build ran, in UTC.
    pub built_at: String,
    /// When the next build runs, from the config's `[build] every_hours`;
    /// null when the config names no interval.
    pub next_build_at: Option<String>,
    /// The most whole days since a report or a suite's run before it reads
    /// stale.
    pub stale_after_days: u32,
    /// The controls job's run that built the document, from `--build-run`
    /// and `--build-url`; null when neither was given.
    pub build_run: Option<BuildRun>,
    /// The imposed rule set that drift is measured against.
    pub imposed: ImposedSet,
    /// How many rule and suite cells read each state, in every column.
    pub counts: StateCounts,
    /// How many repositories' reports read each state.
    pub reports: ReportCounts,
    /// Each repository, in the config's order.
    pub repositories: Vec<RepositoryColumn>,
    /// Each rule and suite: measured rules, zero rules, suites and then
    /// written rules, each by path.
    pub rows: Vec<Row>,
    /// Each group of rules and its roll-up, by path.
    pub groups: Vec<Group>,
}

/// The controls job's run that built the document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct BuildRun {
    /// The run's label for people, such as `Build run #412`; null when
    /// only its address was given.
    pub label: Option<String>,
    /// The run's address; null when only its label was given.
    pub url: Option<String>,
}

/// The imposed rule set, as the config names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ImposedSet {
    /// The set's name for people.
    pub name: String,
    /// The rules board's origin in identity form.
    pub from: String,
    /// The set's root in the rules board.
    pub root: String,
    /// The set's current commit, from `--rules-commit`; null when the build
    /// was not told it.
    pub commit: Option<String>,
}

/// How many cells read each state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, JsonSchema)]
pub struct StateCounts {
    /// Cells that read failing.
    pub failing: usize,
    /// Cells that read stale.
    pub stale: usize,
    /// Cells that read not readable.
    pub not_readable: usize,
    /// Cells that read current.
    pub current: usize,
    /// Cells that read written.
    pub written: usize,
    /// Cells that read absent.
    pub absent: usize,
}

impl StateCounts {
    /// Counts one more cell of `state`.
    fn add(&mut self, state: CellState) {
        let count = match state {
            CellState::Failing => &mut self.failing,
            CellState::Stale => &mut self.stale,
            CellState::NotReadable => &mut self.not_readable,
            CellState::Current => &mut self.current,
            CellState::Written => &mut self.written,
            CellState::Absent => &mut self.absent,
        };
        *count += 1;
    }
}

/// How many repositories' reports read each state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ReportCounts {
    /// Reports no older than the window.
    pub current: usize,
    /// Reports older than the window.
    pub stale: usize,
    /// Repositories with no report file.
    pub no_report: usize,
    /// Report files the build cannot read.
    pub invalid_report: usize,
}

/// What a repository's report reads as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReportState {
    /// The report is no older than the window.
    Current,
    /// The report is older than the window, so its current cells read
    /// stale.
    Stale,
    /// No report file is at the repository's path.
    NoReport,
    /// The report file is not a report the build reads.
    InvalidReport,
}

/// One repository's column.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct RepositoryColumn {
    /// The repository's name, as the config gives it.
    pub name: String,
    /// The heading the pages show it under, from the config.
    pub group: Option<String>,
    /// What the repository's report reads as.
    pub report: ReportState,
    /// `flightdeck::compliance::no_report` or
    /// `flightdeck::compliance::invalid_report`; null when the report was
    /// read.
    pub code: Option<String>,
    /// The path of the field that made the report invalid, such as
    /// `rules[0].result`; null otherwise.
    pub field: Option<String>,
    /// What made the report invalid; null otherwise.
    pub message: Option<String>,
    /// When the report was made; null without a readable report.
    pub generated: Option<String>,
    /// Whole seconds from the report to the build; null without a
    /// readable report.
    pub age_s: Option<u64>,
    /// The commit the report read; null when it read none.
    pub commit: Option<String>,
    /// When that commit was made; null when unknown.
    pub committed: Option<String>,
    /// Whether a tracked file differed from the commit; null without a
    /// readable report.
    pub uncommitted: Option<bool>,
    /// The Board version that made the report.
    pub board_version: Option<String>,
    /// The rules board's origin the report read.
    pub rules_from: Option<String>,
    /// The rules board's commit the report read.
    pub rules_commit: Option<String>,
    /// The CI run that made the report, copied from the report; null for a
    /// local report.
    pub source_run: Option<SourceRun>,
    /// Where the repository differs from the imposed set; null without a
    /// readable report.
    pub drift: Option<Drift>,
    /// Each refusal and warning the report met, as it lists them.
    pub refused: Vec<Refused>,
    /// How many of the column's rule and suite cells read each state.
    pub counts: StateCounts,
}

/// Which part of the matrix a row sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    /// A budget or version rule, which a report measures.
    Measured,
    /// A zero rule, read from a suite's newest run.
    Zero,
    /// A compliance suite, from its newest clean full run.
    Suite,
    /// A pattern, judged or written rule, or one whose checker a report
    /// cannot read, which a report lists and does not measure.
    Written,
}

/// One rule or suite and its cell in each repository.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct Row {
    /// The rule's or the suite's path.
    pub id: String,
    /// The part of the matrix the row sits in.
    pub section: Section,
    /// `imposed` or `repository`, from the report's `rules[].source`; null
    /// for a suite.
    pub source: Option<String>,
    /// What the files must do, from the first report that holds the rule;
    /// null for a suite or a rule that cannot be read.
    pub requirement: Option<String>,
    /// Why the requirement exists; null as for `requirement`.
    pub purpose: Option<String>,
    /// `standing` or `obligation`; null for a suite.
    pub kind: Option<String>,
    /// When the rule is shown to an agent; null for a suite.
    pub show: Option<String>,
    /// What a program checks, as the first report that holds the rule
    /// gives it; null for a suite or a rule without one.
    pub checker: Option<serde_json::Value>,
    /// The paths of the groups the rule is in, in any report.
    pub groups: Vec<String>,
    /// How many cases the suite lists, from the first report that holds
    /// it; null for a rule.
    pub cases: Option<u64>,
    /// The row's cell in each repository, in the order of the document's
    /// `repositories`.
    pub cells: Vec<Cell>,
}

/// One rule's or suite's cell in one repository.
#[derive(Clone, Debug, PartialEq, Serialize, JsonSchema)]
pub struct Cell {
    /// The repository's name.
    pub repository: String,
    /// What the cell reads.
    pub state: CellState,
    /// Why the cell is not readable: the report's or a refusal's code, or
    /// the column's code; null otherwise.
    pub code: Option<String>,
    /// The rule's `result` as the report gives it; null for a suite or an
    /// absent rule.
    pub result: Option<String>,
    /// What the rule's checker measured; null for a suite or an absent
    /// rule.
    pub facts: Option<RuleFacts>,
    /// The suite's newest qualifying run as the report gives it, with its
    /// commits since; null for a rule or a suite with no run.
    pub run: Option<SuiteRun>,
    /// Whole seconds from the suite's run to the build; null for a rule or
    /// a suite with no run.
    pub age_s: Option<u64>,
}

impl Cell {
    /// The cell of a row that does not reach `repository`.
    fn absent(repository: &str) -> Self {
        Self::coded(repository, CellState::Absent, None)
    }

    /// A cell of `state` and `code`, with no facts.
    fn coded(repository: &str, state: CellState, code: Option<&str>) -> Self {
        Self {
            repository: repository.to_owned(),
            state,
            code: code.map(str::to_owned),
            result: None,
            facts: None,
            run: None,
            age_s: None,
        }
    }
}

/// What the build is given besides the config and the reports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Inputs {
    /// When the build runs.
    pub(crate) built_at: Timestamp,
    /// The controls job's run's label, from `--build-run`.
    pub(crate) build_run: Option<String>,
    /// The controls job's run's address, from `--build-url`.
    pub(crate) build_url: Option<String>,
    /// The imposed set's current commit, from `--rules-commit`.
    pub(crate) rules_commit: Option<String>,
}

/// Whole seconds from `earlier` to `later`, 0 when `earlier` is later.
fn seconds_between(earlier: Timestamp, later: Timestamp) -> u64 {
    u64::try_from(later.as_second().saturating_sub(earlier.as_second())).unwrap_or(0)
}

/// Writes `at` as RFC 3339 text in UTC, to the second.
fn utc(at: Timestamp) -> String {
    format!("{at:.0}")
}

/// The key that rows join on: their section's order first, then the path,
/// then the source, so the document's rows come out in order.
type RowKey = (Section, String, Option<String>);

/// Returns the section of a rule whose checker is `checker`.
fn section_of(checker: Option<&serde_json::Value>) -> Section {
    match checker
        .and_then(|checker| checker.get("check"))
        .and_then(serde_json::Value::as_str)
    {
        Some("budget" | "version") => Section::Measured,
        Some("zero") => Section::Zero,
        _ => Section::Written,
    }
}

/// Builds the document from `config`, the result of reading each of its
/// repositories' reports in the same order, and `inputs`.
pub(crate) fn build(
    config: &Config,
    reports: &[Result<Report, Unread>],
    inputs: &Inputs,
) -> ComplianceDocument {
    let window_s = u64::from(config.stale_after_days).saturating_mul(DAY_S);
    let names: Vec<String> = config
        .repositories
        .iter()
        .map(|repository| repository.name.to_string())
        .collect();
    let readable: Vec<Option<&Report>> =
        reports.iter().map(|report| report.as_ref().ok()).collect();
    let mut rows: BTreeMap<RowKey, Row> = BTreeMap::new();
    for report in readable.iter().flatten() {
        for rule in &report.rules {
            let key = (
                section_of(rule.checker.as_ref()),
                rule.id.clone(),
                Some(rule.source.clone()),
            );
            let row = rows.entry(key).or_insert_with(|| empty_rule_row(rule));
            for group in &rule.groups {
                if !row.groups.contains(group) {
                    row.groups.push(group.clone());
                }
            }
        }
        for suite in &report.suites {
            rows.entry((Section::Suite, suite.id.clone(), None))
                .or_insert_with(|| empty_suite_row(suite));
        }
    }
    let mut rows: Vec<Row> = rows.into_values().collect();
    for row in &mut rows {
        row.groups.sort();
        row.cells = names
            .iter()
            .zip(reports)
            .map(|(name, report)| cell_of(row, name, report, inputs.built_at, window_s))
            .collect();
    }
    let groups = groups::roll_up(&names, &readable, &rows);
    let mut counts = StateCounts::default();
    let mut report_counts = ReportCounts::default();
    let repositories = config
        .repositories
        .iter()
        .zip(reports)
        .enumerate()
        .map(|(column, (repository, report))| {
            let mut column_counts = StateCounts::default();
            for cell in rows.iter().filter_map(|row| row.cells.get(column)) {
                column_counts.add(cell.state);
                counts.add(cell.state);
            }
            let column = column_of(
                repository.name.to_string(),
                repository.group.clone(),
                report,
                config,
                inputs,
                window_s,
                column_counts,
            );
            match column.report {
                ReportState::Current => report_counts.current += 1,
                ReportState::Stale => report_counts.stale += 1,
                ReportState::NoReport => report_counts.no_report += 1,
                ReportState::InvalidReport => report_counts.invalid_report += 1,
            }
            column
        })
        .collect();
    let next_build_at = config.every_hours.and_then(|hours| {
        inputs
            .built_at
            .checked_add(jiff::SignedDuration::from_hours(i64::from(hours)))
            .ok()
            .map(utc)
    });
    let build_run = (inputs.build_run.is_some() || inputs.build_url.is_some()).then(|| BuildRun {
        label: inputs.build_run.clone(),
        url: inputs.build_url.clone(),
    });
    ComplianceDocument {
        title: config.title.clone(),
        built_at: utc(inputs.built_at),
        next_build_at,
        stale_after_days: config.stale_after_days,
        build_run,
        imposed: ImposedSet {
            name: config.imposed.name.clone(),
            from: config.imposed.from.clone(),
            root: config.imposed.root.clone(),
            commit: inputs.rules_commit.clone(),
        },
        counts,
        reports: report_counts,
        repositories,
        rows,
        groups,
    }
}

/// A rule's row with the first report's description of it and no cells.
fn empty_rule_row(rule: &RuleRow) -> Row {
    Row {
        id: rule.id.clone(),
        section: section_of(rule.checker.as_ref()),
        source: Some(rule.source.clone()),
        requirement: rule.requirement.clone(),
        purpose: rule.purpose.clone(),
        kind: Some(rule.kind.clone()),
        show: Some(rule.show.clone()),
        checker: rule.checker.clone(),
        groups: Vec::new(),
        cases: None,
        cells: Vec::new(),
    }
}

/// A suite's row with the first report's case count and no cells.
fn empty_suite_row(suite: &SuiteRow) -> Row {
    Row {
        id: suite.id.clone(),
        section: Section::Suite,
        source: None,
        requirement: None,
        purpose: None,
        kind: None,
        show: None,
        checker: None,
        groups: Vec::new(),
        cases: suite.cases,
        cells: Vec::new(),
    }
}

/// Returns `row`'s cell in the column of `repository`, whose report is
/// `report`, at `built_at` with a window of `window_s` seconds.
fn cell_of(
    row: &Row,
    repository: &str,
    report: &Result<Report, Unread>,
    built_at: Timestamp,
    window_s: u64,
) -> Cell {
    let report = match report {
        Ok(report) => report,
        Err(Unread::Missing) => {
            return Cell::coded(repository, CellState::NotReadable, Some(NO_REPORT));
        }
        Err(Unread::Invalid { .. }) => {
            return Cell::coded(repository, CellState::NotReadable, Some(INVALID_REPORT));
        }
    };
    let stale_report = seconds_between(report.generated, built_at) > window_s;
    let mut cell = if row.section == Section::Suite {
        let Some(suite) = report.suites.iter().find(|suite| suite.id == row.id) else {
            return Cell::absent(repository);
        };
        let age_s = suite.run.as_ref().and_then(|run| {
            run.finished
                .parse::<Timestamp>()
                .ok()
                .map(|finished| seconds_between(finished, built_at))
        });
        Cell {
            repository: repository.to_owned(),
            state: state::of_suite(suite.run.as_ref(), suite.code.as_deref(), age_s, window_s),
            code: suite.code.clone(),
            result: None,
            facts: None,
            run: suite.run.clone(),
            age_s,
        }
    } else {
        let Some(rule) = report
            .rules
            .iter()
            .find(|rule| rule.id == row.id && Some(&rule.source) == row.source.as_ref())
        else {
            return Cell::absent(repository);
        };
        let found = state::of_result(&rule.result);
        Cell {
            repository: repository.to_owned(),
            state: if stale_report {
                state::in_stale_report(found)
            } else {
                found
            },
            code: rule.code.clone(),
            result: Some(rule.result.clone()),
            facts: Some(rule.facts.clone()),
            run: None,
            age_s: None,
        }
    };
    if let Some(refused) = report
        .refused
        .iter()
        .find(|refused| refused.rules.contains(&row.id))
    {
        cell.state = CellState::NotReadable;
        cell.code = Some(refused.code.clone());
    }
    cell
}

/// Returns the column of the repository `name`, under `group`, whose
/// report is `report`, with `counts` its cells' counts.
fn column_of(
    name: String,
    group: Option<String>,
    report: &Result<Report, Unread>,
    config: &Config,
    inputs: &Inputs,
    window_s: u64,
    counts: StateCounts,
) -> RepositoryColumn {
    let mut column = RepositoryColumn {
        name,
        group,
        report: ReportState::NoReport,
        code: None,
        field: None,
        message: None,
        generated: None,
        age_s: None,
        commit: None,
        committed: None,
        uncommitted: None,
        board_version: None,
        rules_from: None,
        rules_commit: None,
        source_run: None,
        drift: None,
        refused: Vec::new(),
        counts,
    };
    match report {
        Err(Unread::Missing) => {
            column.code = Some(NO_REPORT.to_owned());
        }
        Err(Unread::Invalid { field, message }) => {
            column.report = ReportState::InvalidReport;
            column.code = Some(INVALID_REPORT.to_owned());
            column.field.clone_from(field);
            column.message = Some(message.clone());
        }
        Ok(report) => {
            let age_s = seconds_between(report.generated, inputs.built_at);
            column.report = if age_s > window_s {
                ReportState::Stale
            } else {
                ReportState::Current
            };
            column.generated = Some(utc(report.generated));
            column.age_s = Some(age_s);
            column.commit.clone_from(&report.repository.commit);
            column.committed.clone_from(&report.repository.committed);
            column.uncommitted = Some(report.repository.uncommitted);
            column.board_version = Some(report.board.version.clone());
            column.rules_from.clone_from(&report.board.rules_from);
            column.rules_commit.clone_from(&report.board.rules_commit);
            column.source_run.clone_from(&report.source_run);
            column.drift = Some(Drift::of(
                report,
                &config.imposed,
                inputs.rules_commit.as_deref(),
            ));
            column.refused.clone_from(&report.refused);
        }
    }
    column
}

impl fmt::Display for ComplianceDocument {
    /// Prints a heading line and one line per repository: its name, its
    /// report's time, and how many of its cells fail and are stale and how
    /// many differences from the imposed set it holds.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{}, built {}: {} failing, {} stale, {} of {} repositories without a report",
            self.title,
            self.built_at,
            self.counts.failing,
            self.counts.stale,
            self.reports.no_report + self.reports.invalid_report,
            self.repositories.len(),
        )?;
        let width = self
            .repositories
            .iter()
            .map(|column| column.name.len())
            .max()
            .unwrap_or(0);
        for column in &self.repositories {
            let name = &column.name;
            match column.report {
                ReportState::NoReport => writeln!(f, "{name:width$}  no report")?,
                ReportState::InvalidReport => writeln!(
                    f,
                    "{name:width$}  invalid report{}",
                    column
                        .field
                        .as_ref()
                        .map_or_else(String::new, |field| format!(" at {field}"))
                )?,
                ReportState::Current | ReportState::Stale => {
                    let stale = if column.report == ReportState::Stale {
                        " (stale)"
                    } else {
                        ""
                    };
                    writeln!(
                        f,
                        "{name:width$}  report {}{stale}  {} failing  {} stale  {} drift",
                        column.generated.as_deref().unwrap_or("unknown"),
                        column.counts.failing,
                        column.counts.stale,
                        column.drift.as_ref().map_or(0, Drift::count),
                    )?;
                }
            }
        }
        Ok(())
    }
}
