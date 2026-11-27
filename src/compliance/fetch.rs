//! `flightdeck compliance fetch`: the newest report of each repository the
//! config names, written into the reports directory that `compliance build`
//! reads, with a record of the runs it came from.
//!
//! For a `github-artifact` source, [`fetch`] lists the newest finished runs
//! of the source's workflow on its branch, finds the newest one that holds
//! the artifact, downloads it, and writes `<owner>/<repo>.json`, the report
//! unchanged, and `<owner>/<repo>.fetch.json`, a
//! `flightdeck/compliance-fetch/v1` record of the newest run, the run the
//! report came from and the commits on the branch since the report's
//! commit. It reaches GitHub only through gh (`crate::spawn::run_gh`), which
//! reads its own token. A `path` source is copied, and a `url` source is
//! refused.
//!
//! Each file is written to a temporary file and renamed into place, the
//! report before its record, so a crash never leaves a record that names a
//! report the directory does not hold. When no run holds the artifact, the
//! old report is removed only after the new record is written. A repository
//! that fails keeps its old files, and the others are still written.

pub(crate) mod record;

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use crumbkit::help::FileFormat;
use crumbkit::{Code, Document, Problem, Row};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use self::record::{FetchRecord, FetchedRun};
use super::ComplianceError;
use super::config::{Repository, RepositoryName, Source, SourceKind};
use crate::spawn::{Exit, GhQuery, Program, SpawnError, Stdout, branch_name, plain_name, run_gh};

/// The code of a fetch that cannot start gh.
pub const NO_GH: Code = Code::from_spelling("flightdeck::compliance::no_gh");

/// The code of a repository whose query of GitHub gh refused or could not
/// make, such as for a missing repository, a token without Actions read or
/// a rate limit.
pub const GH_FAILED: Code = Code::from_spelling("flightdeck::compliance::gh_failed");

/// The code of a repository whose source is a `url`, which the fetch does
/// not read yet.
pub const URL_SOURCE_UNSUPPORTED: Code =
    Code::from_spelling("flightdeck::compliance::url_source_unsupported");

/// The code of a repository for which the config names no source.
pub const NO_SOURCE: Code = Code::from_spelling("flightdeck::compliance::no_source");

/// The code of a source that lacks a key its kind needs, or holds a name
/// the fetch will not pass to gh.
pub const SOURCE_INVALID: Code = Code::from_spelling("flightdeck::compliance::source_invalid");

/// The code of a `path` source whose file cannot be read.
pub const SOURCE_MISSING: Code = Code::from_spelling("flightdeck::compliance::source_missing");

/// The code of an artifact that does not hold exactly one file.
pub const ARTIFACT_UNEXPECTED: Code =
    Code::from_spelling("flightdeck::compliance::artifact_unexpected");

/// The code of a repository whose files cannot be written.
pub const WRITE_FAILED: Code = Code::from_spelling("flightdeck::compliance::write_failed");

/// The code of a report whose commits since cannot be counted, because it
/// names no commit.
pub const NO_COMMIT: Code = Code::from_spelling("flightdeck::compliance::no_commit");

/// The code of a report whose commits since cannot be counted, because
/// GitHub refused the compare, such as for a commit that is gone.
pub const COMPARE_FAILED: Code = Code::from_spelling("flightdeck::compliance::compare_failed");

/// The most bytes of a report that the fetch copies, as the build reads.
const REPORT_LIMIT: u64 = 16 * 1024 * 1024;

/// The most characters of gh's standard error that a row's message keeps.
const MESSAGE_LIMIT: usize = 300;

/// The document that `flightdeck compliance fetch` prints.
pub(crate) const COMPLIANCE_FETCH: Document<Vec<FetchRow>> = Document::new(
    "flightdeck",
    "compliance-fetched",
    1,
    "Each repository the config names, failed ones first, with what the fetch wrote for it: its newest run, the run its report came from and the commits since, or why it failed.",
);

/// The record that `flightdeck compliance fetch` writes beside each report
/// from GitHub, which `compliance build` reads.
pub(crate) const COMPLIANCE_FETCH_RECORD: FileFormat = FileFormat::new(
    Document::<FetchRecord>::new(
        "flightdeck",
        "compliance-fetch",
        1,
        "The record beside a report that compliance fetch wrote: the newest run of the repository's workflow on its branch, the run the report came from and the commits on the branch since the report's commit.",
    ),
);

/// What `flightdeck compliance fetch` is given.
#[derive(Clone, Debug)]
pub struct FetchRequest {
    /// The config file.
    pub config: PathBuf,
    /// The directory to write `<owner>/<repo>.json` into for each
    /// repository.
    pub reports: PathBuf,
    /// gh, found on `PATH`, or `None` when `PATH` cannot find it the same
    /// way from every directory.
    pub gh: Option<Program>,
    /// When the fetch runs.
    pub fetched_at: jiff::Timestamp,
}

/// What the fetch did for one repository.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FetchState {
    /// The newest run uploaded its report, which the fetch wrote.
    Fetched,
    /// The newest run uploaded no report. The fetch wrote an earlier run's
    /// report when one holds it, or else removed the old report.
    NoReport,
    /// The `path` source's file was copied.
    Copied,
    /// Nothing was written, and the repository's old files are as they
    /// were.
    Failed,
}

/// One repository's line in what `flightdeck compliance fetch` prints.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct FetchRow {
    /// The repository, `owner/repo`.
    pub repository: String,
    /// `fetched`, `no_report`, `copied` or `failed`.
    pub state: FetchState,
    /// The number of the newest finished run on the branch; null when the
    /// workflow has none or the fetch failed.
    pub newest_run: Option<u64>,
    /// The number of the run the written report came from; null when none
    /// holds the artifact.
    pub report_run: Option<u64>,
    /// Commits on the branch since the report's commit; null when not
    /// counted.
    pub commits_since: Option<u64>,
    /// Why the repository failed, or why its commits since are null.
    pub code: Option<String>,
    /// What gh or the file system said, for people; null when nothing
    /// failed.
    pub message: Option<String>,
}

impl FetchRow {
    /// Whether the repository failed.
    fn failed(&self) -> bool {
        self.state == FetchState::Failed
    }
}

impl Row for FetchRow {
    const COLUMNS: &'static [&'static str] = &[
        "repository",
        "state",
        "newest_run",
        "report_run",
        "commits_since",
        "code",
    ];
    const LONG_COLUMNS: &'static [&'static str] = &["message"];
    const UNCUT_COLUMNS: &'static [&'static str] = &["repository"];
    const PROBLEM: Option<Problem<Self>> = Some(Problem::by("state", Self::failed));
}

/// Why one repository failed: its code and what happened.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Failed {
    code: Code<'static>,
    message: String,
}

impl Failed {
    /// A failure of `code` saying `message`.
    fn new(code: Code<'static>, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Fetches each repository's report that `request`'s config names, and
/// returns one row per repository, failed ones first and the rest in the
/// config's order.
///
/// # Errors
///
/// Returns [`ComplianceError::ConfigInvalid`] when the config cannot be read
/// or used, and [`ComplianceError::NoGh`] when a repository's source is a
/// GitHub artifact and gh does not start, before anything is written. A
/// repository that fails is a row and no error.
pub fn fetch(request: &FetchRequest) -> Result<Vec<FetchRow>, ComplianceError> {
    let config = super::read_config(&request.config)?;
    let base = request
        .config
        .parent()
        .map_or_else(PathBuf::new, Path::to_path_buf);
    let needs_gh = config.repositories.iter().any(|repository| {
        repository
            .source
            .as_ref()
            .is_some_and(|source| source.kind == SourceKind::GithubArtifact)
    });
    let gh = if needs_gh {
        Some(started_gh(request.gh.as_ref())?)
    } else {
        None
    };
    let fetched_at = format!("{:.0}", request.fetched_at);
    let mut rows: Vec<FetchRow> = config
        .repositories
        .iter()
        .map(|repository| {
            let name = repository.name.to_string();
            fetch_one(
                repository,
                &request.reports,
                &base,
                gh.as_ref(),
                &fetched_at,
            )
            .unwrap_or_else(|failed| FetchRow {
                repository: name,
                state: FetchState::Failed,
                newest_run: None,
                report_run: None,
                commits_since: None,
                code: Some(failed.code.as_str().to_owned()),
                message: Some(failed.message),
            })
        })
        .collect();
    // A stable sort keeps the config's order within the failed rows and
    // within the others.
    rows.sort_by_key(|row| !row.failed());
    Ok(rows)
}

/// Returns `program` once `gh --version` shows that it starts.
///
/// # Errors
///
/// Returns [`ComplianceError::NoGh`] when there is no program, it cannot
/// start, or it exits with a code other than 0.
fn started_gh(program: Option<&Program>) -> Result<Program, ComplianceError> {
    let no_gh = |message: String| ComplianceError::NoGh { message };
    let program = program
        .map(|program| crate::spawn::gh(program.clone()))
        .ok_or_else(|| no_gh("PATH holds a relative directory, so gh cannot be found the same way from every directory".to_owned()))?;
    match run_gh(&program, &GhQuery::Version) {
        Ok(finished) if finished.exit() == Exit::Code(0) => Ok(program),
        Ok(finished) => Err(no_gh(format!(
            "gh --version ended with {:?}: {}",
            finished.exit(),
            last_line(finished.stderr())
        ))),
        Err(error) => Err(no_gh(format!(
            "{error}: {}",
            std::error::Error::source(&error).map_or_else(String::new, ToString::to_string)
        ))),
    }
}

/// Fetches one repository's report into `reports`.
fn fetch_one(
    repository: &Repository,
    reports: &Path,
    base: &Path,
    gh: Option<&Program>,
    fetched_at: &str,
) -> Result<FetchRow, Failed> {
    let source = repository.source.as_ref().ok_or_else(|| {
        Failed::new(
            NO_SOURCE,
            "the config names no [source] and the repository names none of its own",
        )
    })?;
    let place = Place::new(reports, &repository.name);
    match source.kind {
        SourceKind::Url => Err(Failed::new(
            URL_SOURCE_UNSUPPORTED,
            "the fetch reads a github-artifact or a path source, and not a url yet",
        )),
        SourceKind::Path => copy(&place, &repository.name, source, base),
        SourceKind::GithubArtifact => {
            let gh = gh.ok_or_else(|| Failed::new(GH_FAILED, "gh is not started"))?;
            let github = GithubSource::of(source)?;
            from_github(gh, &place, &repository.name, &github, fetched_at)
        }
    }
}

/// Where one repository's files go: `<reports>/<owner>/` and the two names
/// in it.
struct Place {
    dir: PathBuf,
    report: PathBuf,
    record: PathBuf,
}

impl Place {
    /// The place of `name` in `reports`.
    fn new(reports: &Path, name: &RepositoryName) -> Self {
        let dir = reports.join(name.owner());
        Self {
            report: dir.join(format!("{}.json", name.repo())),
            record: dir.join(format!("{}.fetch.json", name.repo())),
            dir,
        }
    }

    /// Creates the owner's directory.
    fn create(&self) -> Result<(), Failed> {
        std::fs::create_dir_all(&self.dir).map_err(|error| write_failed(&self.dir, &error))
    }
}

/// The failure of a write to `path`.
fn write_failed(path: &Path, error: &std::io::Error) -> Failed {
    Failed::new(WRITE_FAILED, format!("{}: {error}", path.display()))
}

/// Copies a `path` source's file into place, and removes a fetch record
/// that described an earlier source's runs.
fn copy(
    place: &Place,
    name: &RepositoryName,
    source: &Source,
    base: &Path,
) -> Result<FetchRow, Failed> {
    let path = source
        .path
        .as_deref()
        .ok_or_else(|| Failed::new(SOURCE_INVALID, "a path source names no path"))?;
    let from = base.join(path);
    let bytes = read_limited(&from)
        .map_err(|error| Failed::new(SOURCE_MISSING, format!("{}: {error}", from.display())))?;
    place.create()?;
    write_file(&place.dir, &place.report, &bytes)?;
    remove(&place.record)?;
    Ok(FetchRow {
        repository: name.to_string(),
        state: FetchState::Copied,
        newest_run: None,
        report_run: None,
        commits_since: None,
        code: None,
        message: None,
    })
}

/// A `github-artifact` source whose names the fetch passes to gh.
struct GithubSource<'a> {
    workflow: &'a str,
    artifact: &'a str,
    branch: &'a str,
}

impl<'a> GithubSource<'a> {
    /// Returns the source's workflow, artifact and branch.
    fn of(source: &'a Source) -> Result<Self, Failed> {
        let need = |value: Option<&'a String>, key: &str| {
            value.map(String::as_str).ok_or_else(|| {
                Failed::new(
                    SOURCE_INVALID,
                    format!("a github-artifact source names no {key}"),
                )
            })
        };
        let workflow = need(source.workflow.as_ref(), "workflow")?;
        let artifact = need(source.artifact.as_ref(), "artifact")?;
        let branch = need(source.branch.as_ref(), "branch")?;
        for (key, value) in [("workflow", workflow), ("artifact", artifact)] {
            if !plain_name(value) {
                return Err(Failed::new(
                    SOURCE_INVALID,
                    format!(
                        "the {key} {value} is not a name of letters, digits, dots, dashes and underscores"
                    ),
                ));
            }
        }
        if !branch_name(branch) {
            return Err(Failed::new(
                SOURCE_INVALID,
                format!(
                    "the branch {branch:?} starts with a dash or holds a space or a control character"
                ),
            ));
        }
        Ok(Self {
            workflow,
            artifact,
            branch,
        })
    }
}

/// One run as GitHub's list of a workflow's runs gives it. Every other
/// field, such as the actor who started the run, is dropped.
#[derive(Deserialize)]
struct RunJson {
    id: u64,
    run_number: u64,
    head_sha: String,
    head_branch: Option<String>,
    run_started_at: Option<String>,
    created_at: String,
    html_url: String,
}

/// GitHub's page of a workflow's runs.
#[derive(Deserialize)]
struct RunsJson {
    workflow_runs: Vec<RunJson>,
}

/// One artifact as GitHub's list of a run's artifacts gives it.
#[derive(Deserialize)]
struct ArtifactJson {
    name: String,
    expired: bool,
}

/// GitHub's page of a run's artifacts.
#[derive(Deserialize)]
struct ArtifactsJson {
    artifacts: Vec<ArtifactJson>,
}

/// The part of a report that the fetch reads: the commit it measured.
#[derive(Deserialize)]
struct ReportCommit {
    repository: Option<ReportRepository>,
}

/// The report's repository, of which the fetch reads the commit.
#[derive(Deserialize)]
struct ReportRepository {
    commit: Option<String>,
}

/// Runs `query` and returns what it printed, or the row's failure with
/// gh's last line of standard error.
fn ask(gh: &Program, query: &GhQuery) -> Result<Vec<u8>, Failed> {
    let words = query
        .words()
        .iter()
        .take(4)
        .map(|word| word.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let finished = run_gh(gh, query).map_err(|error| {
        let detail = match &error {
            SpawnError::Start(inner) | SpawnError::Run(inner) => inner.to_string(),
        };
        Failed::new(GH_FAILED, format!("gh {words}: {error}: {detail}"))
    })?;
    match (finished.exit(), finished.stdout()) {
        (Exit::Code(0), Stdout::Whole(bytes)) => Ok(bytes.clone()),
        (Exit::Code(0), Stdout::OverLimit) => Err(Failed::new(
            GH_FAILED,
            format!("gh {words} printed more than flightdeck reads"),
        )),
        (Exit::TimedOut, _) => Err(Failed::new(
            GH_FAILED,
            format!("gh {words} did not end within its time limit"),
        )),
        (exit, _) => Err(Failed::new(
            GH_FAILED,
            format!(
                "gh {words} ended with {exit:?}: {}",
                last_line(finished.stderr())
            ),
        )),
    }
}

/// Parses `bytes`, which `what` printed, as JSON of `T`.
fn parsed<T: for<'de> Deserialize<'de>>(bytes: &[u8], what: &str) -> Result<T, Failed> {
    serde_json::from_slice(bytes).map_err(|error| {
        Failed::new(
            GH_FAILED,
            format!("gh printed {what} that flightdeck cannot read: {error}"),
        )
    })
}

/// Returns the last line of `stderr` that holds text, cut to
/// [`MESSAGE_LIMIT`] characters.
fn last_line(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let line = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();
    line.chars().take(MESSAGE_LIMIT).collect()
}

/// Fetches one repository's report from its GitHub runs.
fn from_github(
    gh: &Program,
    place: &Place,
    name: &RepositoryName,
    source: &GithubSource<'_>,
    fetched_at: &str,
) -> Result<FetchRow, Failed> {
    let repository = name.to_string();
    let runs: RunsJson = parsed(
        &ask(
            gh,
            &GhQuery::Runs {
                repository: repository.clone(),
                workflow: source.workflow.to_owned(),
                branch: source.branch.to_owned(),
            },
        )?,
        "a list of runs",
    )?;
    let mut newest: Option<FetchedRun> = None;
    let mut report: Option<FetchedRun> = None;
    for run in &runs.workflow_runs {
        let artifacts: ArtifactsJson = parsed(
            &ask(
                gh,
                &GhQuery::Artifacts {
                    repository: repository.clone(),
                    run_id: run.id,
                    artifact: source.artifact.to_owned(),
                },
            )?,
            "a list of artifacts",
        )?;
        let uploaded = artifacts
            .artifacts
            .iter()
            .any(|artifact| artifact.name == source.artifact && !artifact.expired);
        let fetched = FetchedRun {
            run_id: run.id,
            run_number: run.run_number,
            commit: run.head_sha.clone(),
            branch: run
                .head_branch
                .clone()
                .unwrap_or_else(|| source.branch.to_owned()),
            started_at: run
                .run_started_at
                .clone()
                .unwrap_or_else(|| run.created_at.clone()),
            url: run.html_url.clone(),
            uploaded,
        };
        if newest.is_none() {
            newest = Some(fetched.clone());
        }
        if uploaded {
            report = Some(fetched);
            break;
        }
    }
    place.create()?;
    let mut record = FetchRecord {
        schema: record::SCHEMA.to_owned(),
        repository,
        fetched_at: fetched_at.to_owned(),
        workflow: source.workflow.to_owned(),
        branch: source.branch.to_owned(),
        artifact: source.artifact.to_owned(),
        newest_run: newest,
        report_run: report,
        commits_since: None,
        commits_since_code: None,
    };
    if let Some(run) = &record.report_run {
        let downloaded = download(gh, place, &record.repository, run.run_id, source.artifact)?;
        let commit = report_commit(&downloaded.bytes);
        match commit {
            None => record.commits_since_code = Some(NO_COMMIT.as_str().to_owned()),
            Some(commit) => match commits_since(gh, &record.repository, &commit, source.branch) {
                Ok(count) => record.commits_since = Some(count),
                Err(_) => record.commits_since_code = Some(COMPARE_FAILED.as_str().to_owned()),
            },
        }
        std::fs::rename(&downloaded.path, &place.report)
            .map_err(|error| write_failed(&place.report, &error))?;
        write_record(place, &record)?;
    } else {
        write_record(place, &record)?;
        remove(&place.report)?;
    }
    let fetched = record.newest_run.as_ref().is_some_and(|run| run.uploaded);
    Ok(FetchRow {
        repository: record.repository.clone(),
        state: if fetched {
            FetchState::Fetched
        } else {
            FetchState::NoReport
        },
        newest_run: record.newest_run.as_ref().map(|run| run.run_number),
        report_run: record.report_run.as_ref().map(|run| run.run_number),
        commits_since: record.commits_since,
        code: record.commits_since_code.clone(),
        message: None,
    })
}

/// A report that gh downloaded: the temporary file it is in, kept until
/// it is renamed into place, and its bytes.
struct Downloaded {
    path: PathBuf,
    bytes: Vec<u8>,
    // The directory is removed when this is dropped, after the report is
    // renamed out of it.
    _dir: tempfile::TempDir,
}

/// Downloads `run_id`'s artifact into a temporary directory in the owner's
/// directory, so the report can be renamed into place, and returns its one
/// file.
fn download(
    gh: &Program,
    place: &Place,
    repository: &str,
    run_id: u64,
    artifact: &str,
) -> Result<Downloaded, Failed> {
    let dir = tempfile::Builder::new()
        .prefix(".fetch-")
        .tempdir_in(&place.dir)
        .map_err(|error| write_failed(&place.dir, &error))?;
    ask(
        gh,
        &GhQuery::Download {
            repository: repository.to_owned(),
            run_id,
            artifact: artifact.to_owned(),
            dir: dir.path().to_path_buf(),
        },
    )?;
    let entries: Vec<PathBuf> = std::fs::read_dir(dir.path())
        .map_err(|error| write_failed(dir.path(), &error))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    let [path] = entries.as_slice() else {
        return Err(Failed::new(
            ARTIFACT_UNEXPECTED,
            format!(
                "the artifact {artifact} of run {run_id} holds {} files, and the fetch reads one report",
                entries.len()
            ),
        ));
    };
    if !path.is_file() {
        return Err(Failed::new(
            ARTIFACT_UNEXPECTED,
            format!(
                "the artifact {artifact} of run {run_id} holds a directory, and the fetch reads one report"
            ),
        ));
    }
    let bytes = read_limited(path).map_err(|error| {
        Failed::new(ARTIFACT_UNEXPECTED, format!("{}: {error}", path.display()))
    })?;
    Ok(Downloaded {
        path: path.clone(),
        bytes,
        _dir: dir,
    })
}

/// Returns the commit a report names, or `None` when it names none or is
/// not JSON. The build reads the report in full and says what is wrong
/// with it.
fn report_commit(bytes: &[u8]) -> Option<String> {
    let report: ReportCommit = serde_json::from_slice(bytes).ok()?;
    let commit = report.repository?.commit?;
    let hex = commit.bytes().all(|byte| byte.is_ascii_hexdigit());
    (hex && matches!(commit.len(), 40 | 64)).then_some(commit)
}

/// Returns how many commits `branch` is ahead of `commit`.
fn commits_since(
    gh: &Program,
    repository: &str,
    commit: &str,
    branch: &str,
) -> Result<u64, Failed> {
    let bytes = ask(
        gh,
        &GhQuery::Compare {
            repository: repository.to_owned(),
            base: commit.to_owned(),
            branch: branch.to_owned(),
        },
    )?;
    String::from_utf8_lossy(&bytes)
        .trim()
        .parse()
        .map_err(|_| Failed::new(GH_FAILED, "gh printed a count that is not a number"))
}

/// Writes `record` to its place through a temporary file.
fn write_record(place: &Place, record: &FetchRecord) -> Result<(), Failed> {
    // serde_json fails only on a map key that is not a string, and the
    // record's keys are all strings.
    let mut bytes = serde_json::to_vec_pretty(record).unwrap_or_default();
    bytes.push(b'\n');
    write_file(&place.dir, &place.record, &bytes)
}

/// Writes `bytes` to `path` through a temporary file in `dir` that is
/// renamed over the old file only once it is whole.
fn write_file(dir: &Path, path: &Path, bytes: &[u8]) -> Result<(), Failed> {
    let failed = |error: std::io::Error| write_failed(path, &error);
    let mut temporary = tempfile::Builder::new()
        .prefix(".fetch-")
        .tempfile_in(dir)
        .map_err(failed)?;
    temporary.write_all(bytes).map_err(failed)?;
    temporary.as_file().sync_all().map_err(failed)?;
    temporary
        .persist(path)
        .map_err(|error| failed(error.error))?;
    Ok(())
}

/// Removes the file at `path`, which may already be gone.
fn remove(path: &Path) -> Result<(), Failed> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(write_failed(path, &error)),
    }
}

/// Reads at most [`REPORT_LIMIT`] bytes of the file at `path`.
fn read_limited(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(REPORT_LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > REPORT_LIMIT {
        return Err(std::io::Error::other(format!(
            "the file is larger than {REPORT_LIMIT} bytes"
        )));
    }
    Ok(bytes)
}
