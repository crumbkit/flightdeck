//! crumbkit's JSON documents as flightdeck reads them: the listing envelope,
//! the documents of `commands list`, `schemas list` and `schemas show`, and
//! the failure document. Every type ignores the fields it leaves out, so a
//! newer crumbkit that adds a field still loads.

use serde::Deserialize;
use serde_json::Value;

use super::command::CommandPath;
use super::wire::CommandRow;

/// The envelope of a listing: one page of rows and what the tool knows
/// about the rows it left out.
#[derive(Clone, Debug, Deserialize)]
pub struct Listing<T> {
    schema: String,
    data: Vec<T>,
    truncated: bool,
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    next: Vec<NextStep>,
}

impl<T> Listing<T> {
    /// Returns the document's id, such as `shelf/books-list/v1`.
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Returns the rows of the page.
    #[must_use]
    pub fn data(&self) -> &[T] {
        &self.data
    }

    /// Returns true when the page leaves out rows that matched.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.truncated
    }

    /// Returns the number of rows that matched, when the tool counted them.
    #[must_use]
    pub const fn total(&self) -> Option<u64> {
        self.total
    }

    /// Returns the listing's next steps, which a newer crumbkit prints and
    /// 0.1.0 does not, so a listing from 0.1.0 has none.
    #[must_use]
    pub fn next(&self) -> &[NextStep] {
        &self.next
    }

    /// Returns the rows, consuming the listing.
    pub(super) fn into_data(self) -> Vec<T> {
        self.data
    }
}

/// One next step of a listing: a sentence and the commands that carry it
/// out.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct NextStep {
    text: String,
    commands: Vec<StepCommand>,
}

impl NextStep {
    /// Returns the step's sentence.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the commands that carry the step out, in order.
    #[must_use]
    pub fn commands(&self) -> &[StepCommand] {
        &self.commands
    }
}

/// One command of a next step.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StepCommand {
    /// A command of the tool itself, such as `shelf books show <ID>...`.
    Call(String),
    /// A command of another program, such as `rm removed.txt`.
    Run(String),
}

/// The document of `commands list`.
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct CommandsList {
    listing: Listing<CommandRow>,
}

impl CommandsList {
    /// Returns the document's id, such as `shelf/commands-list/v1`.
    #[must_use]
    pub fn schema(&self) -> &str {
        self.listing.schema()
    }

    /// Returns true when the list leaves out commands.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.listing.truncated()
    }

    /// Returns the rows, consuming the list.
    pub(super) fn into_rows(self) -> Vec<CommandRow> {
        self.listing.into_data()
    }
}

/// The document of `schemas list`.
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct SchemasList {
    listing: Listing<SchemaRow>,
}

impl SchemasList {
    /// Returns the document's id, such as `shelf/schemas-list/v1`.
    #[must_use]
    pub fn schema(&self) -> &str {
        self.listing.schema()
    }

    /// Returns true when the list leaves out documents.
    #[must_use]
    pub const fn truncated(&self) -> bool {
        self.listing.truncated()
    }

    /// Returns the id of each document.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.listing.data().iter().map(|row| row.id.as_str())
    }

    /// Returns the rows, consuming the list.
    pub(super) fn into_rows(self) -> Vec<SchemaRow> {
        self.listing.into_data()
    }
}

/// One row of `schemas list`.
#[derive(Debug, Deserialize)]
pub(super) struct SchemaRow {
    pub(super) id: String,
    pub(super) kind: DocumentKind,
    pub(super) form: DocumentForm,
    #[serde(default)]
    pub(super) description: Option<String>,
    /// The paths of the commands that print the document, with the tool's
    /// name first, such as `shelf books list`.
    #[serde(default)]
    pub(super) commands: Vec<String>,
}

/// The document of `schemas show`.
#[derive(Debug, Deserialize)]
pub(super) struct SchemasShow {
    pub(super) schema: String,
    pub(super) data: Vec<ShownSchema>,
    pub(super) failures: Vec<ToolFailure>,
}

/// One schema that `schemas show` printed.
#[derive(Debug, Deserialize)]
pub(super) struct ShownSchema {
    pub(super) id: String,
    pub(super) schema: Value,
}

/// One document that a tool prints, with its JSON Schema.
#[derive(Clone, Debug)]
pub struct Document {
    id: String,
    kind: DocumentKind,
    form: DocumentForm,
    description: Option<String>,
    commands: Vec<CommandPath>,
    schema: Value,
}

impl Document {
    /// Returns the document of a `schemas list` row with its schema.
    pub(super) fn new(row: SchemaRow, schema: Value) -> Self {
        Self {
            id: row.id,
            kind: row.kind,
            form: row.form,
            description: row.description,
            commands: row
                .commands
                .iter()
                .map(|path| CommandPath::from_row(path))
                .collect(),
            schema,
        }
    }

    /// Returns the commands that print the document.
    #[must_use]
    pub fn commands(&self) -> &[CommandPath] {
        &self.commands
    }

    /// Returns the document's id, such as `shelf/books-list/v1`.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns what kind of document it is.
    #[must_use]
    pub const fn kind(&self) -> DocumentKind {
        self.kind
    }

    /// Returns whether the document prints once or once on each line.
    #[must_use]
    pub const fn form(&self) -> DocumentForm {
        self.form
    }

    /// Returns the document's one-line description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the document's JSON Schema, draft 2020-12.
    #[must_use]
    pub const fn schema(&self) -> &Value {
        &self.schema
    }
}

/// What kind of document a tool lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentKind {
    /// A document that a command prints in its envelope.
    Output,
    /// A document that a command prints without the envelope.
    Unwrapped,
    /// A file that the tool reads, such as a settings file.
    File,
}

/// Whether a listed document prints once or once on each line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentForm {
    /// One JSON document.
    Document,
    /// One JSON document on each line.
    Stream,
}

/// The failure document that a crumbkit tool prints with `--json`, and each
/// failure of a batch.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ToolFailure {
    code: String,
    message: String,
    #[serde(default)]
    cause: Option<String>,
    #[serde(default)]
    fields: serde_json::Map<String, Value>,
    recovery: Recovery,
    #[serde(default)]
    alternatives: Vec<Recovery>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    index: Option<u64>,
    #[serde(default)]
    url: Option<String>,
}

impl ToolFailure {
    /// Returns the failure's code in the tool's spelling, such as
    /// `shelf::cli::usage`.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the failure's message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the failure's causes joined into one line.
    #[must_use]
    pub fn cause(&self) -> Option<&str> {
        self.cause.as_deref()
    }

    /// Returns the values at fault, by name.
    #[must_use]
    pub const fn fields(&self) -> &serde_json::Map<String, Value> {
        &self.fields
    }

    /// Returns the step that repairs the failure.
    #[must_use]
    pub const fn recovery(&self) -> &Recovery {
        &self.recovery
    }

    /// Returns the other steps that can repair the failure.
    #[must_use]
    pub fn alternatives(&self) -> &[Recovery] {
        &self.alternatives
    }

    /// Returns the key that a batch's failure failed on, such as a book id.
    #[must_use]
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// Returns the index of the key among a batch's keys.
    #[must_use]
    pub const fn index(&self) -> Option<u64> {
        self.index
    }

    /// Returns a page that explains the failure.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

/// A step that repairs a failure.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Recovery {
    /// Run this command of the tool.
    Run {
        /// The command line, such as `shelf books list`.
        call: String,
        /// The list field of the failure whose every value the call takes in
        /// place of its placeholder, such as `left_out`.
        #[serde(default)]
        all: Option<String>,
    },
    /// Run the same command again.
    Retry,
    /// Follow this advice.
    Hint {
        /// The advice.
        text: String,
    },
    /// No step repairs the failure.
    None {
        /// Why no step repairs it.
        reason: String,
    },
}
