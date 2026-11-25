//! Where one repository differs from the imposed rule set.
//!
//! Board computes the drift it can see from the report alone: the
//! repository's extra rules, the imposed rules it holds with other fields,
//! and the imposed rules that reach none of its files. The build copies
//! those and adds the two facts that need the config: whether the report
//! read another set, and whether it read an older commit of the set.
//! Drift sits beside a repository's cells and never changes their state.

use schemars::JsonSchema;
use serde::Serialize;

use super::config::Imposed;
use super::report::{Differs, Report};

/// Where one repository differs from the imposed rule set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Drift {
    /// The repository's own rules with no imposed rule of the same path,
    /// as the report lists them.
    pub extra: Vec<String>,
    /// The imposed rules the repository also holds with different fields,
    /// as the report lists them.
    pub differs: Vec<Differs>,
    /// The imposed rules that reach no file in the repository, as the
    /// report lists them.
    pub not_reached: Vec<String>,
    /// Whether the report read another rule set: its `board.rules_from`
    /// is not the config's `imposed.from`, or its `imposed.root` is not
    /// the config's `imposed.root`.
    pub other_set: bool,
    /// Whether the report read an older commit of the set: its
    /// `board.rules_commit` is not the commit `--rules-commit` names. Null
    /// when the build was given no `--rules-commit`.
    pub behind: Option<bool>,
}

impl Drift {
    /// Returns the drift of `report` from `imposed`, whose current commit
    /// is `rules_commit` when the build was told it.
    pub(crate) fn of(report: &Report, imposed: &Imposed, rules_commit: Option<&str>) -> Self {
        let other_set = report.board.rules_from.as_deref() != Some(imposed.from.as_str())
            || report.imposed.root != imposed.root;
        let behind =
            rules_commit.map(|commit| report.board.rules_commit.as_deref() != Some(commit));
        Self {
            extra: report.drift.extra.clone(),
            differs: report.drift.differs.clone(),
            not_reached: report.drift.not_reached.clone(),
            other_set,
            behind,
        }
    }

    /// Returns how many differences the drift holds: each listed rule, and
    /// one each for another set and an older commit.
    #[must_use]
    pub fn count(&self) -> usize {
        self.extra.len()
            + self.differs.len()
            + self.not_reached.len()
            + usize::from(self.other_set)
            + usize::from(self.behind == Some(true))
    }
}
