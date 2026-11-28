//! The record type `flightdeck/workflow-graph/v1`: an agent's reading of one
//! workflow, such as a changes flow or a scheduled loop, as a graph of
//! lanes, groups, nodes and edges, with the bindings that place live
//! records on it. One record per workflow lives at `/workflow-graphs/<slug>`
//! in a board.
//!
//! The structs here are the type. [`schema`] generates its JSON Schema,
//! which the flightdeck Board plugin ships at
//! `plugin/types/flightdeck/workflow-graph/v1.schema.json`, and a test fails
//! when the two differ. [`check`] reports what a JSON Schema cannot see,
//! such as an edge to a node id that does not exist. The page and
//! `flightdeck.check` run the same check.

use std::borrow::Cow;

use schemars::generate::SchemaSettings;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

mod check;

#[cfg(test)]
mod tests;

pub use self::check::{Problem, check};

/// The record type's name.
pub const TYPE: &str = "flightdeck/workflow-graph/v1";

/// The most lanes a graph holds.
pub const MAX_LANES: usize = 8;
/// The most groups a graph holds.
pub const MAX_GROUPS: usize = 40;
/// The most nodes a graph holds.
pub const MAX_NODES: usize = 200;
/// The most edges a graph holds.
pub const MAX_EDGES: usize = 400;
/// The most bindings a graph holds.
pub const MAX_BINDINGS: usize = 40;
/// The most further sources a graph names.
pub const MAX_SOURCES: usize = 20;
/// The most conditions of one binding.
pub const MAX_CONDITIONS: usize = 8;

/// The pattern of an id: lowercase words of letters and digits joined by
/// hyphens.
const ID: &str = "^[a-z0-9]+(-[a-z0-9]+)*$";
/// The pattern of a one-line label.
const ONE_LINE: &str = "^[^\\n]*$";
/// The pattern of a JSON pointer.
const POINTER: &str = "^(/[^/]*)*$";
/// The pattern of a full commit id.
const COMMIT: &str = "^[0-9a-f]{40}$";
/// The pattern of a record type, such as `changes/change/v2`.
const RECORD_TYPE: &str = "^[a-z0-9-]+/[a-z0-9-]+/v[0-9]+$";

/// One workflow graph record: the `$type` tag, the [`Graph`], when it was
/// written and who wrote it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(
    title = "flightdeck/workflow-graph/v1",
    description = "An agent's reading of one workflow as a graph, at /workflow-graphs/<slug>: who acts in each lane, the phases and loops, the steps and checks, the moves between them, and how live records sit on them.",
    extend("additionalProperties" = false)
)]
pub struct WorkflowGraph {
    /// The record's type.
    #[serde(rename = "$type")]
    pub tag: Tag,
    /// The graph itself, which a proposal can embed.
    #[serde(flatten)]
    pub graph: Graph,
    /// When the agent wrote this reading, as RFC 3339 time.
    #[schemars(extend("format" = "date-time"))]
    pub made: String,
    /// Who wrote this reading.
    pub by: By,
}

/// The `$type` tag, which is always [`TYPE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tag;

impl Serialize for Tag {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(TYPE)
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = Cow::<str>::deserialize(deserializer)?;
        if text == TYPE {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "the record's $type is {text}, not {TYPE}"
            )))
        }
    }
}

impl JsonSchema for Tag {
    fn schema_name() -> Cow<'static, str> {
        "Tag".into()
    }

    fn inline_schema() -> bool {
        true
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "const": TYPE })
    }
}

/// The workflow's structure, from its title to its bindings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Graph {
    /// The workflow's name as people say it.
    #[schemars(length(min = 1, max = 120), regex(pattern = ONE_LINE))]
    pub title: String,
    /// What the workflow is for, in one or two sentences.
    #[schemars(extend("x-board-prose" = true))]
    pub summary: String,
    /// The record or file the agent read.
    pub subject: Source,
    /// Further records or files the agent read, such as a skill a step
    /// names.
    #[schemars(length(max = MAX_SOURCES))]
    pub sources: Vec<Source>,
    /// Who acts: one lane per actor.
    #[schemars(length(min = 1, max = MAX_LANES))]
    pub lanes: Vec<Lane>,
    /// The phases, loops and parallel sets that contain nodes.
    #[schemars(length(max = MAX_GROUPS))]
    pub groups: Vec<Group>,
    /// The steps, checks, waits, starts and ends.
    #[schemars(length(min = 1, max = MAX_NODES))]
    pub nodes: Vec<Node>,
    /// The moves between nodes.
    #[schemars(length(max = MAX_EDGES))]
    pub edges: Vec<Edge>,
    /// How live records sit on a node or a group.
    #[schemars(length(max = MAX_BINDINGS))]
    pub bindings: Vec<Binding>,
}

/// A record or file that the agent read, and the commit it read it at.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// The record or file, such as `/flows/development/3029fae590f5a628`.
    #[serde(rename = "ref")]
    #[schemars(extend("format" = "board-ref"))]
    pub reference: String,
    /// The commit of the board or mount that the agent read it at.
    #[schemars(regex(pattern = COMMIT))]
    pub commit: String,
}

/// What kind of actor works in a lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ActorKind {
    /// The board's owner.
    Owner,
    /// A coding agent.
    Agent,
    /// A program, such as Board.
    Program,
    /// A schedule that starts the work.
    Schedule,
}

/// One actor, whose nodes sit in its lane.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lane {
    /// Unique among the record's lanes.
    #[schemars(regex(pattern = ID))]
    pub id: String,
    /// Such as `Lead` or `Builder`.
    #[schemars(length(min = 1, max = 120), regex(pattern = ONE_LINE))]
    pub label: String,
    /// What kind of actor works in the lane.
    pub actor: ActorKind,
}

/// How the nodes inside a group relate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GroupKind {
    /// A phase of the workflow.
    Phase,
    /// A loop that runs again from its start.
    Loop,
    /// A set of nodes that run together.
    Parallel,
}

/// A phase, loop or parallel set that contains nodes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// Unique among the record's groups. A node may share it, and an edge
    /// that names it then names the node.
    #[schemars(regex(pattern = ID))]
    pub id: String,
    /// Such as `Plan`.
    #[schemars(length(min = 1, max = 120), regex(pattern = ONE_LINE))]
    pub label: String,
    /// How the nodes inside relate.
    pub kind: GroupKind,
    /// The group this one sits in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID))]
    pub parent: Option<String>,
    /// Where in a source the group comes from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cite: Option<Cite>,
}

/// What a node is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    /// Where an instance begins.
    Start,
    /// Work an actor does.
    Step,
    /// A check that passes or fails.
    Gate,
    /// The actor picks one of several edges.
    Choice,
    /// The flow waits on an actor outside it.
    Wait,
    /// Where an instance finishes.
    End,
}

/// A step, check, choice, wait, start or end.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Node {
    /// Unique among the record's nodes.
    #[schemars(regex(pattern = ID))]
    pub id: String,
    /// Short, such as `Owner signs the plan`.
    #[schemars(length(min = 1, max = 120), regex(pattern = ONE_LINE))]
    pub label: String,
    /// What the node is.
    pub kind: NodeKind,
    /// The id of the lane of whoever acts.
    #[schemars(regex(pattern = ID))]
    pub lane: String,
    /// The innermost group the node sits in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID))]
    pub group: Option<String>,
    /// What happens, for the side panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-board-prose" = true))]
    pub detail: Option<String>,
    /// For a check: whether a failure stops the flow or only warns.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    #[schemars(extend("default" = true))]
    pub blocks: bool,
    /// Whether the node applies only to some instances.
    #[serde(default, skip_serializing_if = "is_false")]
    #[schemars(extend("default" = false))]
    pub conditional: bool,
    /// For a conditional node, the condition, for people only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-board-prose" = true))]
    pub when: Option<String>,
    /// Where in a source the node comes from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cite: Option<Cite>,
    /// Records the step reads or writes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(with = "Vec<RefSchema>")]
    pub refs: Vec<String>,
}

/// The schema of a board-ref, which a list of refs names for each item.
struct RefSchema;

impl JsonSchema for RefSchema {
    fn schema_name() -> Cow<'static, str> {
        "Ref".into()
    }

    fn inline_schema() -> bool {
        true
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "string", "format": "board-ref" })
    }
}

/// Returns true, a node's `blocks` when the record leaves it out.
const fn yes() -> bool {
    true
}

/// Returns whether `value` is true, so a default `blocks` is left out.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes the field by reference"
)]
const fn is_true(value: &bool) -> bool {
    *value
}

/// Returns whether `value` is false, so a default `conditional` is left out.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes the field by reference"
)]
const fn is_false(value: &bool) -> bool {
    !*value
}

/// Where in a source a node or group comes from. `source` 0 is the subject,
/// and 1 the first of `sources`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "at", rename_all = "lowercase", deny_unknown_fields)]
pub enum Cite {
    /// A JSON pointer into a record source.
    Pointer {
        /// The index into the subject and then the sources.
        source: u32,
        /// Such as `/phases/0/checks/4`.
        #[schemars(regex(pattern = POINTER))]
        pointer: String,
    },
    /// A line range in a text source.
    Lines {
        /// The index into the subject and then the sources.
        source: u32,
        /// The inclusive range of lines in the file at its commit.
        lines: Lines,
    },
}

impl Cite {
    /// Returns the index of the cited source.
    #[must_use]
    pub const fn source(&self) -> u32 {
        match self {
            Self::Pointer { source, .. } | Self::Lines { source, .. } => *source,
        }
    }
}

/// An inclusive range of lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Lines {
    /// The first line, counting from 1.
    #[schemars(range(min = 1))]
    pub from: u32,
    /// The last line, counting from 1.
    #[schemars(range(min = 1))]
    pub to: u32,
}

/// What a move between nodes means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    /// The ordinary order.
    Next,
    /// A check passes.
    Pass,
    /// A check fails.
    Fail,
    /// Back to an earlier node after a fix.
    Retry,
    /// The end of a loop back to its start.
    Loop,
    /// Past a node that a condition leaves out.
    Skip,
}

/// A move from one node or group to another. An edge to a group enters its
/// first node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    /// The node or group the move leaves.
    #[schemars(regex(pattern = ID))]
    pub from: String,
    /// The node or group the move enters.
    #[schemars(regex(pattern = ID))]
    pub to: String,
    /// What the move means.
    pub kind: EdgeKind,
    /// Such as `owner rejects`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1, max = 120), regex(pattern = ONE_LINE))]
    pub label: Option<String>,
    /// The condition, for the side panel only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-board-prose" = true))]
    pub when: Option<String>,
}

/// Live records of one type under one path that sit at a node or a group.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    /// The node or group the records sit at.
    #[schemars(regex(pattern = ID))]
    pub target: String,
    /// The records' type, such as `changes/change/v2`.
    #[serde(rename = "type")]
    #[schemars(regex(pattern = RECORD_TYPE))]
    pub record_type: String,
    /// Where the records live, such as `/changes`.
    #[schemars(extend("format" = "board-ref"))]
    pub under: String,
    /// The conditions, every one of which must hold.
    #[serde(rename = "match")]
    #[schemars(length(max = MAX_CONDITIONS))]
    pub conditions: Vec<Condition>,
    /// The field that names the workflow version a record follows, such
    /// as `/flow`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = POINTER))]
    pub version: Option<String>,
}

/// How a condition compares a field with its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    /// The field is the string `value`.
    Equals,
    /// The field is a board path at or below `value`.
    Under,
}

/// One test of a live record's field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    /// The JSON pointer of the field, such as `/phase`.
    #[schemars(regex(pattern = POINTER))]
    pub field: String,
    /// How the field is compared.
    pub op: Op,
    /// Such as `plan`, or `/flows/development` for `under`.
    pub value: String,
}

/// Who wrote a record or acted: the session, the program, the person and
/// the agent's name, each when known, and at least one. It is the `by`
/// that other Board types hold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(rename = "Actor", extend("minProperties" = 1))]
pub struct By {
    /// The name the agent was given, such as `nightly scheduler`, as free
    /// text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(extend("x-board-prose" = true))]
    pub agent: Option<String>,
    /// The person who acted, such as `ana`, or their email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^([a-z0-9]+(-[a-z0-9]+)*|[^@\\s]+@[^@\\s]+)$"))]
    pub person: Option<String>,
    /// The program that acted, such as `claude-code` or `codex`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = ID))]
    pub program: Option<String>,
    /// The Claude Code session's `local_` id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = "^local_[0-9a-f-]+$"))]
    pub session: Option<String>,
}

/// Returns the JSON Schema of `flightdeck/workflow-graph/v1`, in draft
/// 2020-12, with `x-board-prose` on each prose field and `format:
/// board-ref` on each ref.
#[must_use]
pub fn schema() -> Value {
    let mut schema = SchemaSettings::draft2020_12()
        .into_generator()
        .into_root_schema_for::<WorkflowGraph>()
        .to_value();
    tidy(&mut schema);
    schema
}

/// Returns the schema as the bytes of the file the plugin ships: pretty
/// JSON with a final newline.
#[must_use]
pub fn schema_text() -> String {
    // serde_json fails only on a map key that is not a string, and a
    // schema's keys are all strings.
    let mut text = serde_json::to_string_pretty(&schema()).unwrap_or_default();
    text.push('\n');
    text
}

/// Removes the integer formats, such as `uint32`, that schemars writes and
/// a board does not know, and the null that schemars admits for an
/// optional field, since a record leaves such a field out.
fn tidy(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Array(types)) = map.get_mut("type") {
                types.retain(|kind| kind != "null");
                if let [only] = types.as_slice() {
                    let only = only.clone();
                    map.insert("type".to_owned(), only);
                }
            }
            if let Some(Value::Array(options)) = map.get("anyOf")
                && let [one, null] = options.as_slice()
                && null.get("type").is_some_and(|kind| kind == "null")
                && let Value::Object(one) = one.clone()
            {
                map.remove("anyOf");
                map.extend(one);
            }
            if map
                .get("format")
                .and_then(Value::as_str)
                .is_some_and(|format| format.starts_with("uint") || format.starts_with("int"))
            {
                map.remove("format");
            }
            for inner in map.values_mut() {
                tidy(inner);
            }
        }
        Value::Array(items) => {
            for inner in items {
                tidy(inner);
            }
        }
        _ => {}
    }
}
