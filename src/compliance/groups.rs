//! The groups of rules, such as `rust` and `server`, and the state each
//! rolls up to in each repository.
//!
//! A group's members are the rules whose `groups` name it in any report,
//! so a rule such as `rust-server` counts in both `rust` and `server`. The
//! build recomputes each roll-up from the cells, worst first, because a
//! cell turns stale only at the build's time. It keeps the counts each
//! report gave the group.

use std::collections::{BTreeMap, BTreeSet};

use schemars::JsonSchema;
use serde::Serialize;

use super::document::Row;
use super::report::Report;
use super::state::{self, CellState};

/// One group of rules and its roll-up in each repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Group {
    /// The group's path, such as `/rules/groups/rust`.
    pub id: String,
    /// The group's name for people, or its path when no report names it.
    pub title: String,
    /// The ids of the rules in the group, by path.
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
    let mut members: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for report in reports.iter().flatten() {
        for group in &report.groups {
            titles.entry(&group.id).or_insert(&group.title);
            members.entry(&group.id).or_default();
        }
        for rule in &report.rules {
            for group in &rule.groups {
                members.entry(group).or_default().insert(&rule.id);
            }
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
                    let states = rows
                        .iter()
                        .filter(|row| row.source.is_some() && rules.contains(row.id.as_str()))
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
                members: rules.into_iter().map(str::to_owned).collect(),
                cells,
            }
        })
        .collect()
}
