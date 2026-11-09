//! The choices of an argument that an inferred link fills: the values of
//! the `id` column of the rows that the link's source command prints.
//!
//! [`Runner::choices`](super::Runner::choices) finds the link that a
//! [`ChoicesRequest`] names in the tool's model, runs the link's source
//! with the request of [`source_request`], and reads its rows with
//! [`read`].

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};

use super::request::{RunRequest, Value};
use crate::model::{Command, CommandPath, CrumbkitFlag, ID_COLUMN, Link};

#[cfg(test)]
mod tests;

/// The most choices of one reply. A source with more rows gives its first
/// `MOST_CHOICES`, marked truncated, and the page still takes a typed
/// value.
pub const MOST_CHOICES: usize = 1000;

/// The most bytes that a source run may print, on one line and in all. A
/// source that prints more ends with flightdeck's output limit, and the
/// route answers that it found no listing.
pub const CHOICES_OUTPUT: usize = 4 * 1024 * 1024;

/// The link whose choices the page asks for: its source command, its
/// target command and the target's argument. Each must match a link that
/// the tool's model inferred, and none of them reaches a command line. The
/// directory is the one the source runs in.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoicesRequest {
    source: Vec<String>,
    target: Vec<String>,
    argument: String,
    directory: Option<String>,
}

impl ChoicesRequest {
    /// Returns the request for the choices of `argument` of the command at
    /// `target` from the rows of the command at `source`.
    #[must_use]
    pub fn new<W: Into<String>>(
        source: impl IntoIterator<Item = W>,
        target: impl IntoIterator<Item = W>,
        argument: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into_iter().map(Into::into).collect(),
            target: target.into_iter().map(Into::into).collect(),
            argument: argument.into(),
            directory: None,
        }
    }

    /// Returns the source's path as the page sent it.
    #[must_use]
    pub fn source(&self) -> &[String] {
        &self.source
    }

    /// Returns true when the request names `link`.
    pub(super) fn names(&self, link: &Link) -> bool {
        link.source().words() == self.source.as_slice()
            && link.target().words() == self.target.as_slice()
            && link.argument() == self.argument
    }
}

/// Returns the request that runs `source` for its choices in the directory
/// of `choices`: its path, and `--all` when it takes it, so that the listing
/// holds every row.
pub(super) fn source_request(source: &Command, choices: &ChoicesRequest) -> RunRequest {
    let mut request = RunRequest::new(source.path().words().iter().cloned());
    if let Some(directory) = &choices.directory {
        request = request.directory(directory.as_str());
    }
    match source
        .crumbkit_flags()
        .find(|(flag, _)| *flag == CrumbkitFlag::All)
    {
        Some((_, all)) => request.value(all.name(), Value::Flag(true)),
        None => request,
    }
}

/// The choices of one argument, which serialize as the reply of
/// `POST /api/choices`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Choices {
    source: CommandPath,
    column: &'static str,
    labels: Vec<String>,
    choices: Vec<Choice>,
    truncated: bool,
}

impl Choices {
    /// Returns the choices, in the order of the source's rows.
    #[must_use]
    pub fn choices(&self) -> &[Choice] {
        &self.choices
    }

    /// Returns the names of the columns whose cells label each choice, in
    /// the source's column order.
    #[must_use]
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Returns true when the source has rows that the choices leave out.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }
}

/// One choice: the row's `id` as text, and the row's cells of the columns
/// that [`Choices::labels`] names.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Choice {
    value: String,
    label: Vec<Json>,
}

impl Choice {
    /// Returns the value that the choice fills in, such as `2`.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the cells that label the choice, such as the title, author
    /// and year of a book.
    #[must_use]
    pub fn label(&self) -> &[Json] {
        &self.label
    }
}

/// The parts of a listing that the choices read.
#[derive(Deserialize)]
struct Rows {
    data: Vec<Map<String, Json>>,
    truncated: bool,
}

/// Returns the choices in `document`, the standard output of a run of
/// `source`, or `None` when it is not one listing.
///
/// Each row whose `id` is text or a number gives a choice, and a row with
/// any other `id` gives none. Each choice is labelled by the cells of the
/// source's compact columns other than `id`. The choices hold at most
/// [`MOST_CHOICES`] rows, and are truncated when the source had more or
/// the tool cut its listing short.
pub(super) fn read(source: &Command, document: &str) -> Option<Choices> {
    let rows: Rows = serde_json::from_str(document).ok()?;
    let labels: Vec<String> = source
        .result()
        .columns()
        .iter()
        .filter(|column| !column.long() && column.name() != ID_COLUMN)
        .map(|column| column.name().to_owned())
        .collect();
    let mut valued = rows.data.iter().filter_map(|row| {
        let value = match row.get(ID_COLUMN)? {
            Json::String(text) => text.clone(),
            Json::Number(number) => number.to_string(),
            _ => return None,
        };
        Some((value, row))
    });
    let choices = valued
        .by_ref()
        .take(MOST_CHOICES)
        .map(|(value, row)| Choice {
            value,
            label: labels
                .iter()
                .map(|name| row.get(name).cloned().unwrap_or(Json::Null))
                .collect(),
        })
        .collect();
    let truncated = rows.truncated || valued.next().is_some();
    Some(Choices {
        source: source.path().clone(),
        column: ID_COLUMN,
        labels,
        choices,
        truncated,
    })
}
