//! The parts of Board's documents that the workflows queries read. Each
//! struct names only the fields flightdeck copies, so a field that Board
//! adds is ignored.

use serde::Deserialize;
use serde_json::Value;

/// The schema id of `board query --json`.
pub(super) const QUERY: &str = "board/query-report/v2";

/// The schema id of `board log --json`.
pub(super) const LOG: &str = "board/log-report/v2";

/// The schema id of a failure that Board prints with `--json`.
pub(super) const FAILURE: &str = "board/failure/v1";

/// The field that every document of Board starts with.
#[derive(Deserialize)]
pub(super) struct Head {
    pub(super) schema: String,
}

/// A failure document.
#[derive(Deserialize)]
pub(super) struct Failure {
    pub(super) code: String,
    pub(super) message: String,
}

/// One row of a query in the order of the statement's columns: `logical`,
/// `type`, `status`, `code` and `body`.
type Columns = (
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

/// The rows of a query.
#[derive(Deserialize)]
pub(super) struct Query {
    pub(super) data: Vec<Columns>,
}

/// One row of a query, by its columns' names.
#[derive(Clone, Debug)]
pub(super) struct Row {
    /// The record's logical path, such as `/workflow-graphs/small-flow`.
    pub(super) logical: String,
    /// The record's type, or `None` when Board cannot tell it.
    pub(super) record_type: Option<String>,
    /// `ok`, or `degraded` when the record fails its checks.
    pub(super) status: String,
    /// Board's code of a degraded record.
    pub(super) code: Option<String>,
    /// The record as JSON text.
    pub(super) body: Option<String>,
}

impl Query {
    /// Returns the query's rows by their columns' names.
    pub(super) fn rows(self) -> Vec<Row> {
        self.data
            .into_iter()
            .map(|(logical, record_type, status, code, body)| Row {
                logical,
                record_type,
                status,
                code,
                body,
            })
            .collect()
    }
}

impl Row {
    /// Returns the record's body as JSON, or `None` when it has none or it
    /// does not parse.
    pub(super) fn json(&self) -> Option<Value> {
        self.body
            .as_deref()
            .and_then(|body| serde_json::from_str(body).ok())
    }
}

/// The commits of a log, newest first.
#[derive(Deserialize)]
pub(super) struct Log {
    pub(super) data: Vec<Version>,
}

/// One commit that changed a path.
#[derive(Deserialize)]
pub(super) struct Version {
    pub(super) commit: String,
    /// The file at the path in that commit, or `None` when the commit
    /// removed it.
    pub(super) blob: Option<String>,
}
