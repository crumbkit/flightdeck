//! What the page sends to run a command.

use std::collections::BTreeMap;

use serde::Deserialize;

#[cfg(test)]
mod tests;

/// A command to run and a value for each argument it sets, as the page
/// sends it. The runner checks it against the tool's model before it runs
/// anything.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRequest {
    path: Vec<String>,
    #[serde(default)]
    values: BTreeMap<String, Value>,
}

impl RunRequest {
    /// Returns the request for the command with the path `words`, such as
    /// `["books", "list"]`, and no values.
    #[must_use]
    pub fn new<W: Into<String>>(words: impl IntoIterator<Item = W>) -> Self {
        Self {
            path: words.into_iter().map(Into::into).collect(),
            values: BTreeMap::new(),
        }
    }

    /// Returns the request with `value` for the argument `name`, which is
    /// the name the model holds, such as `--limit` or `<ID>`.
    #[must_use]
    pub fn value(mut self, name: impl Into<String>, value: Value) -> Self {
        self.values.insert(name.into(), value);
        self
    }

    /// Returns the command's path.
    #[must_use]
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Returns the values by argument name.
    #[must_use]
    pub const fn values(&self) -> &BTreeMap<String, Value> {
        &self.values
    }
}

/// The value of one argument: JSON `true` or `false`, a string, or a list
/// of strings.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// A flag, which `false` leaves out.
    Flag(bool),
    /// One value.
    One(String),
    /// A list of values, which an empty list leaves out.
    Many(Vec<String>),
}
