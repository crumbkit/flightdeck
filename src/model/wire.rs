//! The rows of crumbkit's `commands list` document as the tool prints them.
//! The model's own types are built from these rows, and serde ignores every
//! field that a row holds and these types leave out, so a newer crumbkit that
//! adds a field still loads.

use serde::Deserialize;

use super::command::{Column, Effects, Example, ExitMeaning, Form, Writes};

/// One row of `commands list`: a group or a runnable command.
#[derive(Debug, Deserialize)]
pub(super) struct CommandRow {
    /// The command's path with the tool's name first, such as
    /// `shelf books list`.
    pub(super) path: String,
    pub(super) about: Option<String>,
    #[serde(default)]
    pub(super) long_about: Option<String>,
    #[serde(default)]
    pub(super) arguments: Vec<ArgumentRow>,
    #[serde(default)]
    pub(super) examples: Vec<Example>,
    #[serde(default)]
    pub(super) columns: Vec<Column>,
    #[serde(default)]
    pub(super) form: Option<Form>,
    #[serde(default)]
    pub(super) exits: Vec<ExitMeaning>,
    #[serde(default)]
    pub(super) effects: Effects,
    #[serde(default)]
    pub(super) writes: Option<Writes>,
    /// The goal headings of a group's help, which a newer crumbkit prints
    /// and 0.1.0 does not.
    #[serde(default)]
    pub(super) goals: Vec<GoalRow>,
}

/// One argument of a `commands list` row.
#[derive(Debug, Deserialize)]
pub(super) struct ArgumentRow {
    /// `--limit` for an option, `<ID>` for a positional argument.
    pub(super) name: String,
    pub(super) short: Option<String>,
    pub(super) value_names: Vec<String>,
    pub(super) required: bool,
    pub(super) many: bool,
    pub(super) defaults: Vec<String>,
    pub(super) possible_values: Vec<String>,
    pub(super) help: Option<String>,
    pub(super) long_help: Option<String>,
    pub(super) global: bool,
    pub(super) one_of: Vec<String>,
}

/// One goal heading of a group's help.
#[derive(Debug, Deserialize)]
pub(super) struct GoalRow {
    pub(super) heading: String,
    /// The commands under the heading, each named by its words below the
    /// group, such as `books`.
    pub(super) commands: Vec<String>,
}
