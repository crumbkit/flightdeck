//! The environment variables that a tool declares, read from the built-in
//! `self environment list` that crumbkit-starmap gives a tool which declares
//! its environment with starmap.
//!
//! Each row describes one variable: its type, what happens when it is
//! unset, where its value comes from, the value it had in the run and its
//! state. The variables that crumbkit reads in every tool follow the tool's
//! own, marked with their `owner`. A tool without the command, or a call that
//! fails or prints another document, leaves the tool with no environment and
//! is no error.
//!
//! This module is the one reader of the document, so a change to its fields
//! changes only this file and the page's schema of a variable.

use serde::{Deserialize, Serialize};

use super::figures::read_listing;

#[cfg(test)]
mod tests;

/// The words of the command that prints the environment.
pub(super) const ENVIRONMENT_COMMAND: [&str; 3] = ["self", "environment", "list"];

/// Where a variable's value comes from in the run, as crumbkit-starmap
/// names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueState {
    /// The run's environment sets the variable.
    Set,
    /// The run's environment leaves the variable unset, and its default
    /// applies.
    Default,
    /// The variable has no value.
    Unset,
    /// The run's environment sets a value that does not fit the type.
    Invalid,
}

/// The value of a variable in the run. A withheld value never has a text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value {
    state: ValueState,
    #[serde(default)]
    text: Option<String>,
}

/// One value of a named enumeration with the sentence that says what
/// choosing it means.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Described {
    value: String,
    description: String,
}

/// One variable as the page reads it, with the field names of the
/// built-in's document. Each optional field is `None` when the tool leaves
/// it out, as it does for a sentence that the declaration does not state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variable {
    name: String,
    /// The type of the value, such as `port` or `integer 1..=100`.
    #[serde(rename = "type")]
    kind: String,
    /// What happens when the variable has no value, such as `default 8080`
    /// or `optional; production required`.
    unset: String,
    /// Where the value comes from, such as `setting` or `secret, 1Password`.
    source: String,
    value: Value,
    /// `shown` or `withheld`.
    #[serde(default)]
    show: Option<String>,
    /// The variable's bind, such as a 1Password reference.
    #[serde(default)]
    bind: Option<String>,
    #[serde(default)]
    cache: Option<String>,
    #[serde(default)]
    purpose: Option<String>,
    /// What the tool does when the variable is unset.
    #[serde(default)]
    effect: Option<String>,
    #[serde(default)]
    enumeration: Option<String>,
    #[serde(default)]
    values: Option<Vec<Described>>,
    /// The check that the tool's own code makes of the value.
    #[serde(default)]
    check: Option<String>,
    #[serde(default)]
    group: Option<String>,
    /// The library that reads the variable in every tool, such as
    /// `crumbkit`, or `None` for the tool's own variable.
    #[serde(default)]
    owner: Option<String>,
}

impl Variable {
    /// Returns the variable's name, such as `SHELF_PORT`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns where the variable's value comes from in the run.
    #[must_use]
    pub const fn state(&self) -> ValueState {
        self.value.state
    }

    /// Returns the value's text, or `None` for a value that is unset or
    /// withheld.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.value.text.as_deref()
    }

    /// Returns the library that reads the variable in every tool, or `None`
    /// for the tool's own variable.
    #[must_use]
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    /// Returns the variable with no value text when the tool withholds its
    /// value. crumbkit-starmap never prints a withheld value's text, so this
    /// only guards against a tool that does: the page never holds a secret.
    fn guarded(mut self) -> Self {
        if self.show.as_deref() != Some("shown") {
            self.value.text = None;
        }
        self
    }
}

/// Reads the standard output of `self environment list --json --full`, or
/// returns `None` when it is not that document. A row that does not read is
/// left out alone, and a value whose `show` is not `shown` loses its text.
pub(super) fn read(stdout: &[u8]) -> Option<Vec<Variable>> {
    let listing = read_listing::<serde_json::Value>(stdout, "self-environment-list")?;
    Some(
        listing
            .into_data()
            .into_iter()
            .filter_map(|row| serde_json::from_value::<Variable>(row).ok())
            .map(Variable::guarded)
            .collect(),
    )
}
