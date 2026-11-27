//! The `flightdeck/compliance-fetch/v1` record: what `flightdeck compliance
//! fetch` learned of a repository's GitHub runs, which it writes beside the
//! repository's report as `<owner>/<repo>.fetch.json` and which the build
//! reads.
//!
//! The record names runs by their number, address, commit and time. It
//! holds no token and no person: GitHub's run records name the actor who
//! started a run, and the fetch drops that field.

use std::io::Read as _;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::compliance::report::Unread;

/// The id that a record's `schema` holds.
pub(crate) const SCHEMA: &str = "flightdeck/compliance-fetch/v1";

/// The start of every run's address that a record may hold: a page on
/// GitHub, which the pages link to.
pub(crate) const RUN_URL_PREFIX: &str = "https://github.com/";

/// The most bytes of a record that the build reads. A record is under a
/// kilobyte.
const RECORD_LIMIT: u64 = 1024 * 1024;

/// What one fetch learned of a repository's runs on its branch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(title = "flightdeck/compliance-fetch/v1")]
pub struct FetchRecord {
    /// `flightdeck/compliance-fetch/v1`.
    pub schema: String,
    /// The repository, `owner/repo`.
    pub repository: String,
    /// When the fetch ran, in UTC.
    pub fetched_at: String,
    /// The workflow file whose runs the fetch listed, such as `ci.yml`.
    pub workflow: String,
    /// The branch whose runs the fetch listed, such as `main`.
    pub branch: String,
    /// The artifact that holds the report, such as `rules-report`.
    pub artifact: String,
    /// The newest finished run of the workflow on the branch; null when the
    /// workflow has none.
    pub newest_run: Option<FetchedRun>,
    /// The run that the report beside the record came from, which is the
    /// newest run or an earlier one; null when no listed run holds the
    /// artifact, and then no report is beside the record.
    pub report_run: Option<FetchedRun>,
    /// Commits on the branch since the report's commit, as GitHub's compare
    /// counts them; null when there is no report or they could not be
    /// counted.
    pub commits_since: Option<u64>,
    /// Why a report's `commits_since` is null:
    /// `flightdeck::compliance::no_commit` for a report that names no
    /// commit, or `flightdeck::compliance::compare_failed` when GitHub
    /// refused the compare. Null otherwise.
    pub commits_since_code: Option<String>,
}

/// One finished run of a repository's workflow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FetchedRun {
    /// The run's id.
    pub run_id: u64,
    /// The run's number in its workflow.
    pub run_number: u64,
    /// The commit the run checked out.
    pub commit: String,
    /// The branch the run ran on.
    pub branch: String,
    /// When the run started, in UTC.
    pub started_at: String,
    /// The run's page on GitHub.
    pub url: String,
    /// Whether the run holds the report's artifact, unexpired.
    pub uploaded: bool,
}

/// Reads the record of the repository `expected` at `path`, or `None` when
/// no file is there.
///
/// # Errors
///
/// Returns [`Unread::Invalid`] when the file cannot be read, is larger than
/// the build reads, or [`parse`] refuses it.
pub(crate) fn read(path: &Path, expected: &str) -> Result<Option<FetchRecord>, Unread> {
    let invalid = |message: String| Unread::Invalid {
        field: None,
        message,
    };
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(invalid(format!("the fetch record cannot be read: {error}")));
        }
    };
    let mut bytes = Vec::new();
    file.take(RECORD_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| invalid(format!("the fetch record cannot be read: {error}")))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > RECORD_LIMIT {
        return Err(invalid(format!(
            "the fetch record is larger than {RECORD_LIMIT} bytes"
        )));
    }
    parse(&bytes, expected).map(Some)
}

/// Reads the record of the repository `expected` from `bytes`.
///
/// # Errors
///
/// Returns [`Unread::Invalid`], whose message starts "the fetch record",
/// when the bytes are not JSON, a field is missing or of the wrong type,
/// `schema` names another id, the record names another repository, or a
/// run's `url` does not start with [`RUN_URL_PREFIX`].
pub(crate) fn parse(bytes: &[u8], expected: &str) -> Result<FetchRecord, Unread> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let record: FetchRecord =
        serde_path_to_error::deserialize(&mut deserializer).map_err(|error| {
            let path = error.path().to_string();
            Unread::Invalid {
                field: (path != ".").then_some(path),
                message: format!("the fetch record is invalid: {}", error.into_inner()),
            }
        })?;
    if record.schema != SCHEMA {
        return Err(Unread::Invalid {
            field: Some("schema".to_owned()),
            message: format!(
                "the fetch record's schema is {}, and flightdeck reads {SCHEMA}",
                record.schema
            ),
        });
    }
    if !record.repository.eq_ignore_ascii_case(expected) {
        return Err(Unread::Invalid {
            field: Some("repository".to_owned()),
            message: format!(
                "the fetch record is of {}, and its path is that of {expected}",
                record.repository
            ),
        });
    }
    for (field, run) in [
        ("newest_run.url", record.newest_run.as_ref()),
        ("report_run.url", record.report_run.as_ref()),
    ] {
        if let Some(run) = run
            && !run.url.starts_with(RUN_URL_PREFIX)
        {
            return Err(Unread::Invalid {
                field: Some(field.to_owned()),
                message: format!(
                    "the fetch record's {field} is {:?}, which is not a page under {RUN_URL_PREFIX}",
                    run.url
                ),
            });
        }
    }
    Ok(record)
}
