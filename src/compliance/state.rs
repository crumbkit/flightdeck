//! The state table of 23 November: what each report result and suite run
//! reads as on the pages.
//!
//! - A budget or version rule reads failing when it is over or behind,
//!   and current when it is within.
//! - A zero rule reads current at `zero`, stale at `stale` and `no_run`,
//!   and failing at `not_zero` and `missing_measure`.
//! - A rule this Board does not measure reads written.
//! - The repository's own rule, `unmeasured`, an `unreadable` rule and a
//!   result this build does not know read not readable.
//! - A suite reads failing when its newest clean full run failed a case,
//!   stale when it has no such run or the run is older than the config's
//!   `stale_after_days` at the build's time, and current otherwise.
//!
//! Stale is by age only, as answered on 23 November: commits since a run
//! are shown beside the state and never change it.

use schemars::JsonSchema;
use serde::Serialize;

use super::report::SuiteRun;

/// What one cell reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CellState {
    /// A budget is over, a pin is behind, a measure is not 0, or the
    /// newest run failed a case.
    Failing,
    /// The report or the suite's newest clean run is older than the
    /// window, or no clean run exists.
    Stale,
    /// The cell cannot be judged: the report holds a refusal, a result
    /// this build does not know, or a rule the repository holds itself.
    NotReadable,
    /// The rule is met, or the suite's newest clean run passed within the
    /// window.
    Current,
    /// The rule has no checker that a report measures.
    Written,
    /// The rule or suite does not reach this repository.
    Absent,
}

impl CellState {
    /// Every state, worst first: the order in which a group's roll-up
    /// picks the state of its members.
    pub const WORST_FIRST: [Self; 6] = [
        Self::Failing,
        Self::Stale,
        Self::NotReadable,
        Self::Current,
        Self::Written,
        Self::Absent,
    ];

    /// Returns the state's word, as the document spells it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Failing => "failing",
            Self::Stale => "stale",
            Self::NotReadable => "not_readable",
            Self::Current => "current",
            Self::Written => "written",
            Self::Absent => "absent",
        }
    }
}

/// Returns the worst of `states`, by [`CellState::WORST_FIRST`], or
/// [`CellState::Absent`] when there are none.
pub(crate) fn worst(states: impl IntoIterator<Item = CellState>) -> CellState {
    // The enum's order is the worst-first order, so the least is the worst.
    states.into_iter().min().unwrap_or(CellState::Absent)
}

/// Returns the state of a rule whose report found `result`.
pub(crate) fn of_result(result: &str) -> CellState {
    match result {
        "within" | "zero" => CellState::Current,
        "over" | "behind" | "not_zero" | "missing_measure" => CellState::Failing,
        "stale" | "no_run" => CellState::Stale,
        "written" => CellState::Written,
        // `unmeasured`, `unreadable` and any result a later Board adds.
        _ => CellState::NotReadable,
    }
}

/// Returns the state of a rule or suite cell whose report is older than
/// the window: a current cell reads stale, and every other state stays,
/// because a failure the report found is still the newest fact.
pub(crate) const fn in_stale_report(state: CellState) -> CellState {
    match state {
        CellState::Current => CellState::Stale,
        other => other,
    }
}

/// Returns the state of a suite whose newest qualifying run is `run`, with
/// `code` the reason `runs.latest` refused, when the run is `age_s`
/// seconds old at the build's time and the window is `window_s` seconds.
pub(crate) fn of_suite(
    run: Option<&SuiteRun>,
    code: Option<&str>,
    age_s: Option<u64>,
    window_s: u64,
) -> CellState {
    if code.is_some() {
        return CellState::NotReadable;
    }
    let Some(run) = run else {
        return CellState::Stale;
    };
    match run.outcome.as_str() {
        "fail" => CellState::Failing,
        "pass" if age_s.is_some_and(|age| age > window_s) => CellState::Stale,
        "pass" if age_s.is_some() => CellState::Current,
        // A run whose finish this build cannot read, or a later outcome.
        _ => CellState::NotReadable,
    }
}
