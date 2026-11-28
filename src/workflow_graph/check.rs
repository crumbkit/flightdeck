//! The check of a workflow graph: what its JSON Schema cannot see, such as
//! an edge to an id that names nothing. Each problem has a code, the JSON
//! pointer of the field at fault and a sentence.
//!
//! A pass or fail edge leaves a check or a wait, because a wait on the
//! owner, such as a sign-off, passes or fails as a check does. A check
//! that has a pass edge needs a fail edge, and a check that has a fail
//! edge needs a pass or next edge to go on by; a check with only next
//! edges stops the flow where it fails.
//!
//! Ids are unique among the lanes, among the groups and among the nodes.
//! A node and a group may share an id, as a start node and the phase it
//! starts often do, and an edge or a binding that names such an id names
//! the node.

use std::collections::{HashMap, HashSet};

use schemars::JsonSchema;
use serde::Serialize;

use super::{Cite, EdgeKind, NodeKind, WorkflowGraph};

/// The deepest that groups nest: a group, its parent and the parent's
/// parent.
const MAX_DEPTH: usize = 3;

/// One problem that the check found.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Problem {
    /// The problem's code, such as `flightdeck::graph::unknown_end`.
    pub code: String,
    /// The JSON pointer of the field at fault, such as `/edges/2/to`.
    pub pointer: String,
    /// What is wrong, as a sentence.
    pub message: String,
}

/// The problems of one graph, in the order of the record's fields.
struct Problems(Vec<Problem>);

impl Problems {
    /// Adds the problem `code` at `pointer`.
    fn add(&mut self, code: &str, pointer: String, message: String) {
        self.0.push(Problem {
            code: format!("flightdeck::graph::{code}"),
            pointer,
            message,
        });
    }
}

/// Returns every problem of `record` that its schema cannot see, or none.
#[must_use]
pub fn check(record: &WorkflowGraph) -> Vec<Problem> {
    let graph = &record.graph;
    let mut problems = Problems(Vec::new());
    let lanes = ids(
        graph.lanes.iter().map(|lane| lane.id.as_str()),
        "lanes",
        &mut problems,
    );
    let groups: HashMap<&str, usize> = graph
        .groups
        .iter()
        .enumerate()
        .map(|(index, group)| (group.id.as_str(), index))
        .collect();
    let nodes: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect();
    ids(
        graph.groups.iter().map(|group| group.id.as_str()),
        "groups",
        &mut problems,
    );
    ids(
        graph.nodes.iter().map(|node| node.id.as_str()),
        "nodes",
        &mut problems,
    );
    let sources = graph.sources.len() + 1;

    for (index, group) in graph.groups.iter().enumerate() {
        if let Some(parent) = &group.parent
            && !groups.contains_key(parent.as_str())
        {
            problems.add(
                "unknown_group",
                format!("/groups/{index}/parent"),
                format!(
                    "the group {} names a parent group {parent} that the record does not hold",
                    group.id
                ),
            );
        }
        cite(
            group.cite.as_ref(),
            &format!("/groups/{index}/cite"),
            sources,
            &mut problems,
        );
    }
    nesting(record, &groups, &mut problems);

    for (index, node) in graph.nodes.iter().enumerate() {
        if !lanes.contains(node.lane.as_str()) {
            problems.add(
                "unknown_lane",
                format!("/nodes/{index}/lane"),
                format!(
                    "the node {} sits in a lane {} that the record does not hold",
                    node.id, node.lane
                ),
            );
        }
        if let Some(group) = &node.group
            && !groups.contains_key(group.as_str())
        {
            problems.add(
                "unknown_group",
                format!("/nodes/{index}/group"),
                format!(
                    "the node {} sits in a group {group} that the record does not hold",
                    node.id
                ),
            );
        }
        cite(
            node.cite.as_ref(),
            &format!("/nodes/{index}/cite"),
            sources,
            &mut problems,
        );
    }

    let filled = filled_groups(record, &groups);
    for (index, edge) in graph.edges.iter().enumerate() {
        for (end, id) in [("from", &edge.from), ("to", &edge.to)] {
            let pointer = format!("/edges/{index}/{end}");
            if nodes.contains_key(id.as_str()) {
                continue;
            }
            if groups.contains_key(id.as_str()) {
                if !filled.contains(id.as_str()) {
                    problems.add(
                        "empty_group",
                        pointer,
                        format!("the edge's {end} is the group {id}, which holds no node"),
                    );
                }
            } else {
                problems.add(
                    "unknown_end",
                    pointer,
                    format!("the edge's {end} is {id}, which names no node or group"),
                );
            }
        }
        if matches!(edge.kind, EdgeKind::Pass | EdgeKind::Fail)
            && let Some(&from) = nodes.get(edge.from.as_str())
            && let Some(node) = graph.nodes.get(from)
            && !matches!(node.kind, NodeKind::Gate | NodeKind::Wait)
        {
            problems.add(
                "not_a_gate",
                format!("/edges/{index}/kind"),
                format!(
                    "a pass or fail edge leaves {}, which is not a check or a wait",
                    node.id
                ),
            );
        }
    }
    gates(record, &mut problems);

    for (index, binding) in graph.bindings.iter().enumerate() {
        let target = binding.target.as_str();
        if !groups.contains_key(target) && !nodes.contains_key(target) {
            problems.add(
                "unknown_target",
                format!("/bindings/{index}/target"),
                format!("the binding's target {target} names no node or group"),
            );
        }
    }
    problems.0
}

/// Returns the set of `ids` of the list named `list`, adding a problem for
/// each id that repeats an earlier one.
fn ids<'a>(
    ids: impl Iterator<Item = &'a str>,
    list: &str,
    problems: &mut Problems,
) -> HashSet<&'a str> {
    let mut seen = HashSet::new();
    for (index, id) in ids.enumerate() {
        if !seen.insert(id) {
            problems.add(
                "duplicate_id",
                format!("/{list}/{index}/id"),
                format!("the id {id} is used twice"),
            );
        }
    }
    seen
}

/// Adds a problem for a cite at `pointer` whose source is past the end of
/// the `sources` that the subject and the sources give, or whose line
/// range runs backwards.
fn cite(cite: Option<&Cite>, pointer: &str, sources: usize, problems: &mut Problems) {
    let Some(cite) = cite else {
        return;
    };
    let source = cite.source();
    if usize::try_from(source).map_or(true, |source| source >= sources) {
        problems.add(
            "cite_source",
            format!("{pointer}/source"),
            format!("the cite names source {source}, but the record names {sources}, counting the subject as 0"),
        );
    }
    if let Cite::Lines { lines, .. } = cite
        && lines.to < lines.from
    {
        problems.add(
            "lines_backwards",
            format!("{pointer}/lines"),
            format!(
                "the line range runs backwards, from {} to {}",
                lines.from, lines.to
            ),
        );
    }
}

/// Adds a problem for each group whose parents run in a cycle, and each
/// that nests deeper than [`MAX_DEPTH`].
fn nesting(record: &WorkflowGraph, groups: &HashMap<&str, usize>, problems: &mut Problems) {
    let parent_of = |index: usize| {
        record
            .graph
            .groups
            .get(index)
            .and_then(|group| group.parent.as_deref())
            .and_then(|parent| groups.get(parent).copied())
    };
    for index in 0..record.graph.groups.len() {
        let mut seen = HashSet::from([index]);
        let mut depth = 1;
        let mut at = index;
        while let Some(parent) = parent_of(at) {
            if !seen.insert(parent) {
                problems.add(
                    "group_cycle",
                    format!("/groups/{index}/parent"),
                    "the group's parents run in a cycle".to_owned(),
                );
                break;
            }
            depth += 1;
            at = parent;
        }
        if depth > MAX_DEPTH {
            problems.add(
                "group_depth",
                format!("/groups/{index}/parent"),
                format!("the group nests {depth} deep, and groups nest at most {MAX_DEPTH} deep"),
            );
        }
    }
}

/// Returns the ids of the groups that hold a node, directly or through a
/// group inside them.
fn filled_groups<'a>(record: &'a WorkflowGraph, groups: &HashMap<&str, usize>) -> HashSet<&'a str> {
    let mut filled = HashSet::new();
    for node in &record.graph.nodes {
        let mut at = node.group.as_deref();
        let mut steps = 0;
        while let Some(id) = at
            && steps <= record.graph.groups.len()
        {
            if let Some(&index) = groups.get(id)
                && let Some(group) = record.graph.groups.get(index)
            {
                filled.insert(group.id.as_str());
                at = group.parent.as_deref();
            } else {
                at = None;
            }
            steps += 1;
        }
    }
    filled
}

/// Adds a problem for each check with a pass edge and no fail edge, or a
/// fail edge and no pass or next edge.
fn gates(record: &WorkflowGraph, problems: &mut Problems) {
    for (index, node) in record.graph.nodes.iter().enumerate() {
        if node.kind != NodeKind::Gate {
            continue;
        }
        let leaving: Vec<EdgeKind> = record
            .graph
            .edges
            .iter()
            .filter(|edge| edge.from == node.id)
            .map(|edge| edge.kind)
            .collect();
        let pass = leaving.contains(&EdgeKind::Pass);
        let fail = leaving.contains(&EdgeKind::Fail);
        let onward = pass || leaving.contains(&EdgeKind::Next);
        if pass && !fail {
            problems.add(
                "gate_edges",
                format!("/nodes/{index}"),
                format!("the check {} has a pass edge and no fail edge", node.id),
            );
        } else if fail && !onward {
            problems.add(
                "gate_edges",
                format!("/nodes/{index}"),
                format!(
                    "the check {} has a fail edge and no pass or next edge",
                    node.id
                ),
            );
        }
    }
}
