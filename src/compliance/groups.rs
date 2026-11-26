//! The groups of rules, such as `rust` and `server`, and the state each
//! rolls up to in each repository.
//!
//! A group's members are the imposed rows whose `groups` name it, which
//! the build has merged from every report's rules, so a rule such as
//! `rust-server` counts in both `rust` and `server`. A repository's own
//! rules are left out, even its copy of an imposed rule, which Board never
//! measures: drift sits beside a state and never changes one. The build
//! recomputes each roll-up from the cells, worst first, because a
//! cell turns stale only at the build's time. It keeps the counts each
//! report gave the group.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Serialize;

use super::document::Row;
use super::report::{Report, RuleSource};
use super::state::{self, CellState};

/// One group of rules and its roll-up in each repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Group {
    /// The group's path, such as `/rules/groups/rust`.
    pub id: String,
    /// The group's name for people, or its path when no report names it.
    pub title: String,
    /// The ids of the imposed rules in the group, by path.
    pub members: Vec<String>,
    /// The group's roll-up in each repository, in the order of the
    /// document's `repositories`.
    pub cells: Vec<GroupCell>,
}

/// One group's roll-up in one repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct GroupCell {
    /// The repository's name.
    pub repository: String,
    /// The worst state of the members' cells in this repository, or
    /// `absent` when no member reaches it.
    pub state: CellState,
    /// How many of the group's members found each result, as the
    /// repository's report counted them; null when the report holds no
    /// such group.
    pub counts: Option<BTreeMap<String, u64>>,
}

/// Returns each group that `reports` name, by path, with its roll-up over
/// the cells of `rows` in each of `repositories`. `reports` holds the
/// readable report of each repository, or `None`, in the same order.
pub(crate) fn roll_up(
    repositories: &[String],
    reports: &[Option<&Report>],
    rows: &[Row],
) -> Vec<Group> {
    let mut titles: BTreeMap<&str, &str> = BTreeMap::new();
    // Each group's imposed rows by path. Rows are keyed by path and
    // source, so a path has one imposed row.
    let mut members: BTreeMap<&str, BTreeMap<&str, &Row>> = BTreeMap::new();
    for report in reports.iter().flatten() {
        for group in &report.groups {
            titles.entry(&group.id).or_insert(&group.title);
            members.entry(&group.id).or_default();
        }
    }
    for row in rows
        .iter()
        .filter(|row| row.source == Some(RuleSource::Imposed))
    {
        for group in &row.groups {
            members.entry(group).or_default().insert(&row.id, row);
        }
    }
    members
        .into_iter()
        .map(|(id, rules)| {
            let cells = repositories
                .iter()
                .zip(reports)
                .enumerate()
                .map(|(column, (repository, report))| {
                    let states = rules
                        .values()
                        .filter_map(|row| row.cells.get(column))
                        .map(|cell| cell.state);
                    GroupCell {
                        repository: repository.clone(),
                        state: state::worst(states),
                        counts: report.and_then(|report| {
                            report
                                .groups
                                .iter()
                                .find(|group| group.id == id)
                                .map(|group| group.counts.clone())
                        }),
                    }
                })
                .collect();
            Group {
                id: id.to_owned(),
                title: titles.get(id).map_or(id, |title| *title).to_owned(),
                members: rules.into_keys().map(str::to_owned).collect(),
                cells,
            }
        })
        .collect()
}
