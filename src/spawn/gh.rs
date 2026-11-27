//! The operation that reads GitHub for `flightdeck compliance fetch`: it
//! runs `gh api` to list a workflow's runs and a run's artifacts and to
//! compare two commits, and `gh run download` to fetch one artifact. It
//! states the five items of PROC-2:
//!
//! - Program: `gh` found on the `PATH` that flightdeck inherited. The person
//!   who installed gh is trusted to supply its bytes.
//! - Arguments: the fixed argument vector of each [`GhQuery`], with no
//!   shell. A repository's name has passed `RepositoryName`'s check, a
//!   workflow's file name, an artifact's name and a branch have passed
//!   [`plain_name`] or [`branch_name`], and each value that a person wrote
//!   reaches gh inside one word that starts with a flag or a path, such as
//!   `--name=<artifact>`, `branch=<branch>` after `-f`, or
//!   `repos/<owner>/<repo>/…`, so none can be read as a flag.
//! - Environment: the child inherits flightdeck's environment, so gh reads
//!   its own token from `GH_TOKEN` or its login. flightdeck never reads,
//!   prints or passes a token. It adds `GH_PROMPT_DISABLED=1`, so gh never
//!   waits for an answer, and `GH_NO_UPDATE_NOTIFIER=1`, so gh asks no
//!   server for its own updates.
//! - Streams and deadline: standard input is closed. Standard output is kept
//!   up to [`STDOUT_LIMIT`] bytes, and standard error keeps its last
//!   [`STDERR_LIMIT`] bytes. Each query has its own deadline:
//!   [`GH_TIME_LIMIT`] for a query of the API, and [`GH_DOWNLOAD_LIMIT`]
//!   for a download, which covers the wait for the child and every read
//!   from its pipes. The queries run one after another.
//! - End: as for `describe`, crumbkit-process starts each child in a process
//!   group of its own on Unix, or in a job object on Windows, and stops the
//!   whole group or ends the job when the child exits or its deadline
//!   passes, and only then reaps it. A query of the API only reads. A
//!   download writes only into a temporary directory that the fetch
//!   removes, so a stopped child leaves no partial report.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crumbkit_process::{Capture, Outcome, OverLimit, RunOptions};

use super::{Exit, Finished, Program, STDERR_LIMIT, STDOUT_LIMIT, SpawnError, Stdout};

/// The time limit of each query of GitHub's API.
///
/// RUST-10: one fixed limit covers every query. It handles a network that
/// hangs, and it misses a slow GitHub that answers after a minute. A
/// configured limit replaces it when a runner needs one.
pub const GH_TIME_LIMIT: Duration = Duration::from_secs(60);

/// The time limit of one artifact's download, which holds a report of at
/// most 16 MiB.
pub const GH_DOWNLOAD_LIMIT: Duration = Duration::from_secs(300);

/// How many of a workflow's newest runs the fetch lists to find the newest
/// one that holds the artifact.
pub const RUNS_LISTED: u32 = 10;

/// One query of GitHub that the fetch runs through gh.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum GhQuery {
    /// `gh --version`, which shows that gh starts.
    Version,
    /// The newest finished runs of `workflow` on `branch`, newest first.
    Runs {
        /// The repository, `owner/repo`.
        repository: String,
        /// The workflow's file name, such as `ci.yml`.
        workflow: String,
        /// The branch, such as `main`.
        branch: String,
    },
    /// The artifacts of one run that are named `artifact`.
    Artifacts {
        /// The repository, `owner/repo`.
        repository: String,
        /// The run's id.
        run_id: u64,
        /// The artifact's name.
        artifact: String,
    },
    /// The download of one run's artifact into `dir`.
    Download {
        /// The repository, `owner/repo`.
        repository: String,
        /// The run's id.
        run_id: u64,
        /// The artifact's name.
        artifact: String,
        /// The directory the artifact's files are unpacked into.
        dir: PathBuf,
    },
    /// How many commits `branch` is ahead of the commit `base`.
    Compare {
        /// The repository, `owner/repo`.
        repository: String,
        /// The earlier commit, as 40 or 64 hexadecimal digits.
        base: String,
        /// The branch, such as `main`.
        branch: String,
    },
}

impl GhQuery {
    /// Returns the query's argument vector after the program.
    #[must_use]
    pub(crate) fn words(&self) -> Vec<OsString> {
        let words: Vec<String> = match self {
            Self::Version => vec!["--version".to_owned()],
            Self::Runs {
                repository,
                workflow,
                branch,
            } => vec![
                "api".to_owned(),
                "--method".to_owned(),
                "GET".to_owned(),
                format!("repos/{repository}/actions/workflows/{workflow}/runs"),
                "-f".to_owned(),
                format!("branch={branch}"),
                "-f".to_owned(),
                "status=completed".to_owned(),
                "-f".to_owned(),
                format!("per_page={RUNS_LISTED}"),
            ],
            Self::Artifacts {
                repository,
                run_id,
                artifact,
            } => vec![
                "api".to_owned(),
                "--method".to_owned(),
                "GET".to_owned(),
                format!("repos/{repository}/actions/runs/{run_id}/artifacts"),
                "-f".to_owned(),
                format!("name={artifact}"),
            ],
            Self::Download {
                repository,
                run_id,
                artifact,
                dir,
            } => {
                let mut dir_word = OsString::from("--dir=");
                dir_word.push(dir);
                return vec![
                    "run".into(),
                    "download".into(),
                    run_id.to_string().into(),
                    format!("--repo={repository}").into(),
                    format!("--name={artifact}").into(),
                    dir_word,
                ];
            }
            Self::Compare {
                repository,
                base,
                branch,
            } => vec![
                "api".to_owned(),
                format!(
                    "repos/{repository}/compare/{base}...{}",
                    percent_encoded(branch)
                ),
                "--jq".to_owned(),
                ".ahead_by".to_owned(),
            ],
        };
        words.into_iter().map(OsString::from).collect()
    }

    /// Returns the query's deadline from now.
    fn deadline(&self) -> Instant {
        Instant::now()
            + match self {
                Self::Download { .. } => GH_DOWNLOAD_LIMIT,
                _ => GH_TIME_LIMIT,
            }
    }
}

/// Returns `text` with every byte other than a letter, a digit, `-`, `.`,
/// `_` and `~` written as `%XX`, so a branch such as `release/1.0` stays one
/// segment of a path.
fn percent_encoded(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// Whether `text` may name a workflow file or an artifact: one to 255
/// letters, digits, dots, dashes and underscores that do not start with a
/// dash.
#[must_use]
pub(crate) fn plain_name(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 255
        && !text.starts_with('-')
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Whether `text` may name a branch: one to 255 characters with no control
/// character and no space, that do not start with a dash, as Git allows.
#[must_use]
pub(crate) fn branch_name(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 255
        && !text.starts_with('-')
        && !text.chars().any(|c| c.is_control() || c == ' ')
}

/// Returns `gh`, found on `PATH`, with the variables every query adds.
#[must_use]
pub(crate) fn gh(program: Program) -> Program {
    program
        .env("GH_PROMPT_DISABLED", "1")
        .env("GH_NO_UPDATE_NOTIFIER", "1")
}

/// Runs `query` against `program`, waits for it until its deadline, and
/// returns what it printed and how it ended.
///
/// # Errors
///
/// Returns [`SpawnError::Start`] when the program does not exist or cannot
/// be started, and [`SpawnError::Run`] when waiting for it or reading its
/// output fails. The child is stopped and reaped on every path.
pub(crate) fn run_gh(program: &Program, query: &GhQuery) -> Result<Finished, SpawnError> {
    let settings = RunOptions::new()
        .stdout(Capture::whole(STDOUT_LIMIT).over(OverLimit::Sink))
        .stderr(Capture::tail(STDERR_LIMIT))
        .deadline(query.deadline());
    let finished = crumbkit_process::run(program.command(&query.words()), &settings)?;
    let exit = match finished.outcome() {
        Outcome::TimedOut => Exit::TimedOut,
        _ => Exit::of(finished.status()),
    };
    let (stdout, stderr) = finished.into_output();
    Ok(Finished {
        exit,
        stdout: if stdout.over_limit() {
            Stdout::OverLimit
        } else {
            Stdout::Whole(stdout.into_bytes())
        },
        stderr: stderr.into_bytes(),
    })
}

#[cfg(test)]
mod tests {
    use super::{GhQuery, branch_name, plain_name};

    #[test]
    fn a_branch_with_a_slash_stays_one_segment_of_the_compare_path() {
        let words = GhQuery::Compare {
            repository: "owner/app".to_owned(),
            base: "a".repeat(40),
            branch: "release/1.0".to_owned(),
        }
        .words();
        assert_eq!(
            words[1],
            format!("repos/owner/app/compare/{}...release%2F1.0", "a".repeat(40)).as_str()
        );
    }

    #[test]
    fn a_name_that_starts_with_a_dash_or_holds_a_slash_is_refused() {
        assert!(plain_name("ci.yml"));
        assert!(!plain_name("--jq"));
        assert!(!plain_name("../ci.yml"));
        assert!(branch_name("release/1.0"));
        assert!(!branch_name("-main"));
        assert!(!branch_name("main\n"));
    }
}
