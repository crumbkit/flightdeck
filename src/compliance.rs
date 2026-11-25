//! `flightdeck compliance build`: one compliance document from the
//! `rules/report/v1` file that each repository's CI run publishes, and a
//! config that names the repositories and the imposed rule set.
//!
//! [`build`] reads the config, then the report of each repository at
//! `<reports>/<owner>/<repo>.json`, and returns the
//! `flightdeck/compliance/v1` document. A missing or invalid report gives
//! its column a code, and the other columns still render. With `--out`,
//! [`write_out`] writes the document to `<out>/compliance.json` through a
//! temporary file that it renames only once the file is whole, so a failed
//! write leaves the earlier file as it was.
//!
//! The command reads only files. It needs no Board, network or token.

mod config;
mod document;
mod drift;
mod groups;
mod report;
mod state;

#[cfg(test)]
mod tests;

use std::fmt;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use crumbkit::help::FileFormat;
use crumbkit::{Code, Document, Failure, Prose};

pub use self::document::{
    BuildRun, Cell, ComplianceDocument, INVALID_REPORT, ImposedSet, NO_REPORT, ReportCounts,
    ReportState, RepositoryColumn, Row, Section, StateCounts,
};
pub use self::drift::Drift;
pub use self::groups::{Group, GroupCell};
pub use self::report::{
    Differs, FailedCase, FileOver, MeasureValue, Omitted, PinBehind, Refused, RuleFacts,
    RuleSource, SourceRun, SuiteRun, ZeroRead,
};
pub use self::state::CellState;

/// The code of a config that cannot be read or used.
pub const CONFIG_INVALID: Code = Code::from_spelling("flightdeck::compliance::config_invalid");

/// The code of an `--out` directory that the document cannot be written
/// into.
pub const OUT_FAILED: Code = Code::from_spelling("flightdeck::compliance::out_failed");

/// The name of the file that `--out` writes.
pub const OUT_FILE: &str = "compliance.json";

/// The document that `flightdeck compliance build` prints.
pub(crate) const COMPLIANCE_BUILD: Document<Prose<ComplianceDocument>> = Document::new(
    "flightdeck",
    "compliance",
    1,
    "Each repository's report as a column and each rule and suite as a row, with each cell's state, each group's roll-up and where each repository differs from the imposed rule set.",
);

/// The config file that `flightdeck compliance build --config` reads.
pub(crate) const COMPLIANCE_CONFIG: FileFormat = FileFormat::new(
    Document::<config::ConfigFile>::new(
        "flightdeck",
        "compliance-config",
        1,
        "The TOML file that names the repositories a compliance build reads, the imposed rule set and how old a report or run may be before it reads stale.",
    ),
);

/// What `flightdeck compliance build` is given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildRequest {
    /// The config file.
    pub config: PathBuf,
    /// The directory that holds `<owner>/<repo>.json` for each repository.
    pub reports: PathBuf,
    /// When the build runs.
    pub built_at: jiff::Timestamp,
    /// The controls job's run's label.
    pub build_run: Option<String>,
    /// The controls job's run's address.
    pub build_url: Option<String>,
    /// The imposed set's current commit.
    pub rules_commit: Option<String>,
}

/// Why a compliance build gave no document.
#[derive(Debug, Failure)]
pub enum ComplianceError {
    /// The config cannot be read, is not TOML, or breaks the config's
    /// rules.
    #[failure(
        code = CONFIG_INVALID,
        hint = "fix the config at the line named, then build again",
        field(line = line.map_or(crumbkit::FieldValue::Null, |line| crumbkit::FieldValue::from(u64::try_from(line).unwrap_or(u64::MAX)))),
    )]
    ConfigInvalid {
        /// The line, from 1, or `None` when the problem has no place.
        line: Option<usize>,
        /// What is wrong.
        message: String,
    },
    /// The document cannot be written into the `--out` directory.
    #[failure(
        code = OUT_FAILED,
        hint = "create the directory or make it writable, then build again; an earlier compliance.json there is unchanged",
    )]
    OutFailed {
        /// What failed.
        message: String,
    },
}

impl fmt::Display for ComplianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigInvalid {
                line: Some(line),
                message,
            } => write!(f, "the config is invalid at line {line}: {message}"),
            Self::ConfigInvalid {
                line: None,
                message,
            } => write!(f, "the config is invalid: {message}"),
            Self::OutFailed { message } => write!(f, "cannot write {OUT_FILE}: {message}"),
        }
    }
}

impl std::error::Error for ComplianceError {}

/// Builds the compliance document that `request` asks for.
///
/// # Errors
///
/// Returns [`ComplianceError::ConfigInvalid`] when the config cannot be
/// read or used. A report that is missing or invalid gives its column a
/// code and is no error.
pub fn build(request: &BuildRequest) -> Result<ComplianceDocument, ComplianceError> {
    let text = std::fs::read_to_string(&request.config).map_err(|error| {
        ComplianceError::ConfigInvalid {
            line: None,
            message: format!("the config cannot be read: {error}"),
        }
    })?;
    let config =
        config::Config::parse(&text).map_err(|problem| ComplianceError::ConfigInvalid {
            line: problem.line,
            message: problem.message,
        })?;
    let reports: Vec<_> = config
        .repositories
        .iter()
        .map(|repository| {
            let path = request
                .reports
                .join(repository.name.owner())
                .join(format!("{}.json", repository.name.repo()));
            report::read(&path, &repository.name.to_string())
        })
        .collect();
    let inputs = document::Inputs {
        built_at: request.built_at,
        build_run: request.build_run.clone(),
        build_url: request.build_url.clone(),
        rules_commit: request.rules_commit.clone(),
    };
    Ok(document::build(&config, &reports, &inputs))
}

impl ComplianceDocument {
    /// Returns the document as the JSON that `flightdeck compliance build
    /// --json` prints, which `--out` writes to `compliance.json`.
    #[must_use]
    pub fn to_json(&self) -> Vec<u8> {
        /// The prose document, with no failures.
        #[derive(serde::Serialize)]
        struct Whole<'a> {
            schema: String,
            data: &'a ComplianceDocument,
            failures: [(); 0],
        }
        // serde_json fails only on a map key that is not a string, and the
        // document's keys are all strings.
        let mut bytes = serde_json::to_vec(&Whole {
            schema: format!(
                "{}/{}/v{}",
                COMPLIANCE_BUILD.component(),
                COMPLIANCE_BUILD.name(),
                COMPLIANCE_BUILD.version()
            ),
            data: self,
            failures: [],
        })
        .unwrap_or_default();
        bytes.push(b'\n');
        bytes
    }
}

/// Writes `document` to `compliance.json` in `out`, through a temporary
/// file in `out` that is renamed over the old file only once it is whole.
///
/// # Errors
///
/// Returns [`ComplianceError::OutFailed`] when `out` is missing or the
/// file cannot be written or renamed. The earlier `compliance.json` is
/// then unchanged.
pub fn write_out(out: &Path, document: &ComplianceDocument) -> Result<PathBuf, ComplianceError> {
    let failed = |error: std::io::Error| ComplianceError::OutFailed {
        message: format!("{}: {error}", out.display()),
    };
    let mut temporary = tempfile::Builder::new()
        .prefix(".compliance-")
        .suffix(".json")
        .tempfile_in(out)
        .map_err(failed)?;
    temporary.write_all(&document.to_json()).map_err(failed)?;
    temporary.as_file().sync_all().map_err(failed)?;
    let path = out.join(OUT_FILE);
    temporary
        .persist(&path)
        .map_err(|error| failed(error.error))?;
    Ok(path)
}
