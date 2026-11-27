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
pub mod site;
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
    ReportState, RepositoryColumn, Row, Section, SiteFiles, StateCounts,
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

/// The code of a binary whose embedded page has no place for the
/// document, so `--out` writes only `compliance.json`.
pub const NO_PAGE: Code = Code::from_spelling("flightdeck::compliance::no_page");

/// The name of the document that `--out` writes.
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
        hint = "create the directory or make it writable, then build again; the earlier file that failed to be replaced is unchanged",
    )]
    OutFailed {
        /// What failed.
        message: String,
    },
    /// The page embedded in the binary has no place for the document, so
    /// only `compliance.json` was written.
    #[failure(
        code = NO_PAGE,
        hint = "run `pnpm run build` in ui/, which writes the page with its marker, then build flightdeck again; compliance.json was written",
    )]
    NoPage,
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
            Self::OutFailed { message } => write!(f, "cannot write the site: {message}"),
            Self::NoPage => write!(
                f,
                "the page embedded in flightdeck has no place for the document, so only {OUT_FILE} was written"
            ),
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

/// The files that [`write_out`] wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Written {
    /// The path of `compliance.json`.
    pub document: PathBuf,
    /// The path of `compliance.html`.
    pub page: PathBuf,
    /// How many bytes `compliance.html` holds.
    pub page_bytes: u64,
}

impl Written {
    /// Whether the page is over the most bytes a Flare share holds.
    #[must_use]
    pub const fn over_flare_cap(&self) -> bool {
        self.page_bytes > site::FLARE_CAP
    }

    /// Returns the codes of the warnings about the page.
    #[must_use]
    pub fn warnings(&self) -> Vec<Code<'static>> {
        if self.over_flare_cap() {
            vec![site::OVER_FLARE_CAP]
        } else {
            Vec::new()
        }
    }

    /// Returns the files as the printed document's `site` names them.
    #[must_use]
    pub fn files(&self) -> SiteFiles {
        SiteFiles {
            document: self.document.display().to_string(),
            page: self.page.display().to_string(),
            page_bytes: self.page_bytes,
            warnings: self
                .warnings()
                .iter()
                .map(|code| code.as_str().to_owned())
                .collect(),
        }
    }

    /// Returns the line that the text output prints after a build with
    /// `--out`: both files, the page's size and, for a page over Flare's
    /// cap, the warning.
    #[must_use]
    pub fn note(&self) -> String {
        let mut note = format!(
            "Wrote {} and {}, {} bytes",
            self.document.display(),
            self.page.display(),
            self.page_bytes
        );
        if self.over_flare_cap() {
            note.push_str(&format!(
                ". Warning {}: {} is over Flare's 4 MiB cap, so publish it as a workflow artifact only",
                site::OVER_FLARE_CAP.as_str(),
                site::PAGE_FILE
            ));
        }
        note
    }
}

/// Writes `document` to `compliance.json` in `out`, then the page that
/// draws it, built from the page embedded in the binary, to
/// `compliance.html`. Each goes through a temporary file in `out` that is
/// renamed over the old file only once it is whole.
///
/// # Errors
///
/// Returns [`ComplianceError::OutFailed`] when `out` is missing or a file
/// cannot be written or renamed; the file that failed is then unchanged.
/// Returns [`ComplianceError::NoPage`] when the embedded page has no place
/// for the document, after `compliance.json` is written.
pub fn write_out(out: &Path, document: &ComplianceDocument) -> Result<Written, ComplianceError> {
    write_out_with(out, document, site::BUNDLE)
}

/// Writes as [`write_out`] does, with `template` in place of the page
/// embedded in the binary.
///
/// # Errors
///
/// As [`write_out`].
pub fn write_out_with(
    out: &Path,
    document: &ComplianceDocument,
    template: &[u8],
) -> Result<Written, ComplianceError> {
    let json = document.to_json();
    let document_path = write_file(out, OUT_FILE, &json)?;
    let page = site::page(template, &json).ok_or(ComplianceError::NoPage)?;
    let page_path = write_file(out, site::PAGE_FILE, &page)?;
    Ok(Written {
        document: document_path,
        page: page_path,
        page_bytes: u64::try_from(page.len()).unwrap_or(u64::MAX),
    })
}

/// Writes `bytes` to `name` in `out` through a temporary file that is
/// renamed over the old file only once it is whole.
fn write_file(out: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, ComplianceError> {
    let failed = |error: std::io::Error| ComplianceError::OutFailed {
        message: format!("{}: {error}", out.join(name).display()),
    };
    let mut temporary = tempfile::Builder::new()
        .prefix(".compliance-")
        .tempfile_in(out)
        .map_err(failed)?;
    temporary.write_all(bytes).map_err(failed)?;
    temporary.as_file().sync_all().map_err(failed)?;
    let path = out.join(name);
    temporary
        .persist(&path)
        .map_err(|error| failed(error.error))?;
    Ok(path)
}
