//! The `flightdeck/compliance-config/v1` file: the TOML that names the
//! repositories a compliance build reads, the imposed rule set they are
//! compared with, and how old a run may be before it reads stale.
//!
//! [`Config::parse`] reads the text into the typed config, or returns a
//! [`ConfigProblem`] that names the line of the first thing it refuses. The
//! config holds no token: a source names where a report comes from, and
//! the command that fetches it reads its credential elsewhere.

use std::collections::BTreeSet;
use std::fmt;

use schemars::JsonSchema;
use serde::Deserialize;
use toml::Spanned;

/// The id that the config's `schema` key must hold.
pub(crate) const SCHEMA: &str = "flightdeck/compliance-config/v1";

/// The imposed set's root when the config names none.
const DEFAULT_ROOT: &str = "/rules";

/// The age in days after which a run reads stale when the config names
/// none, as answered on 23 November.
const DEFAULT_STALE_AFTER_DAYS: u32 = 7;

/// A `flightdeck/compliance-config/v1` file, as a person writes it.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "flightdeck/compliance-config/v1")]
pub(crate) struct ConfigFile {
    /// `flightdeck/compliance-config/v1`.
    #[schemars(with = "String")]
    schema: Spanned<String>,
    /// The pages' title, such as `Compliance`.
    title: String,
    /// The most whole days since a report or a suite's run before it reads
    /// stale, 7 when left out.
    #[serde(default)]
    stale_after_days: Option<u32>,
    /// Where each repository's report comes from, unless the repository
    /// names its own source. The build reads only the reports directory,
    /// so it checks the source's shape and `fetch` will read it.
    #[serde(default, rename = "source")]
    _source: Option<Source>,
    /// Where the suites' runs come from, which the build checks and does
    /// not read.
    #[serde(default, rename = "audits")]
    _audits: Option<Audits>,
    /// How often the controls job builds the pages.
    #[serde(default)]
    build: Option<Build>,
    /// The imposed rule set that every repository's rules are compared
    /// with.
    imposed: ImposedFile,
    /// Each repository whose report the build reads, in the order the
    /// pages show them.
    #[schemars(with = "Vec<RepositoryFile>")]
    repositories: Vec<Spanned<RepositoryFile>>,
}

/// Where a report or a suite's runs come from. The build reads only files;
/// `flightdeck compliance fetch` will read the other kinds.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Source {
    /// `github-artifact`, `path` or `url`.
    kind: SourceKind,
    /// The workflow file whose newest successful run holds the artifact,
    /// such as `ci.yml`.
    #[serde(default)]
    workflow: Option<String>,
    /// The artifact's name, such as `rules-report`.
    #[serde(default)]
    artifact: Option<String>,
    /// The branch whose runs count, such as `main`.
    #[serde(default)]
    branch: Option<String>,
    /// The report's path, for the `path` kind.
    #[serde(default)]
    path: Option<String>,
    /// The report's address, for the `url` kind.
    #[serde(default)]
    url: Option<String>,
}

/// The kinds of place a report comes from.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SourceKind {
    /// An artifact of a GitHub Actions run.
    GithubArtifact,
    /// A file.
    Path,
    /// An address that serves the report.
    Url,
}

/// Where the suites' runs come from.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Audits {
    /// `board`, for suites in the controls repository's own board, or
    /// `github-artifact`.
    kind: AuditsKind,
    /// The board's directory, for the `board` kind.
    #[serde(default)]
    path: Option<String>,
    /// The artifact's name, for the `github-artifact` kind.
    #[serde(default)]
    artifact: Option<String>,
}

/// The kinds of place the suites' runs come from.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AuditsKind {
    /// The controls repository's own board.
    Board,
    /// An artifact of a GitHub Actions run.
    GithubArtifact,
}

/// How often the controls job builds the pages.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Build {
    /// The hours between two builds, which give the next build's time.
    #[schemars(range(min = 1))]
    pub(crate) every_hours: u32,
}

/// The imposed rule set, as the config names it.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ImposedFile {
    /// The set's name for people, such as `platform-controls`.
    name: String,
    /// The rules board's origin in identity form, such as
    /// `owner/controls`, which each report's `board.rules_from` must name.
    from: String,
    /// The set's root in the rules board, `/rules` when left out, which
    /// each report's `imposed.root` must name.
    #[serde(default)]
    root: Option<String>,
}

/// One repository, as the config names it.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RepositoryFile {
    /// The repository's name, `owner/repo`. Its report is
    /// `<reports>/<owner>/<repo>.json`.
    name: String,
    /// The heading the pages show the repository under.
    #[serde(default)]
    group: Option<String>,
    /// Where this repository's report comes from, in place of the
    /// config's `[source]`.
    #[serde(default, rename = "source")]
    _source: Option<Source>,
}

/// A config the build can use: every default applied and every repository
/// named once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    /// The pages' title.
    pub(crate) title: String,
    /// The most whole days before a report or a run reads stale.
    pub(crate) stale_after_days: u32,
    /// The hours between two builds, when the config names them.
    pub(crate) every_hours: Option<u32>,
    /// The imposed rule set.
    pub(crate) imposed: Imposed,
    /// Each repository, in the config's order.
    pub(crate) repositories: Vec<Repository>,
}

/// The imposed rule set that drift is measured against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Imposed {
    /// The set's name for people.
    pub(crate) name: String,
    /// The rules board's origin in identity form.
    pub(crate) from: String,
    /// The set's root in the rules board.
    pub(crate) root: String,
}

/// One repository that the build reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Repository {
    /// The repository's name, `owner/repo`.
    pub(crate) name: RepositoryName,
    /// The heading the pages show it under.
    pub(crate) group: Option<String>,
}

/// A repository's name, `owner/repo`, whose two parts are safe to join
/// onto the reports directory.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RepositoryName {
    owner: String,
    repo: String,
}

impl RepositoryName {
    /// Returns the name, or `None` when it is not `owner/repo` with an
    /// owner of letters, digits and dashes and a repository of letters,
    /// digits, dots, dashes and underscores that is not `.` or `..`.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let (owner, repo) = text.split_once('/')?;
        let owner_ok =
            !owner.is_empty() && owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        let repo_ok = !repo.is_empty()
            && repo != "."
            && repo != ".."
            && repo
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
        (owner_ok && repo_ok).then(|| Self {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
        })
    }

    /// Returns the owner's part.
    pub(crate) fn owner(&self) -> &str {
        &self.owner
    }

    /// Returns the repository's part.
    pub(crate) fn repo(&self) -> &str {
        &self.repo
    }
}

impl fmt::Display for RepositoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.owner, self.repo)
    }
}

/// Why a config cannot be used, with the line of the text it concerns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConfigProblem {
    /// The line, from 1, or `None` when the problem has no place.
    pub(crate) line: Option<usize>,
    /// What is wrong.
    pub(crate) message: String,
}

impl Config {
    /// Reads `text`, a `flightdeck/compliance-config/v1` file.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigProblem`] when the text is not TOML, a key is
    /// unknown or of the wrong type, `schema` names another id, a
    /// repository's name is not `owner/repo`, or a repository is named
    /// twice.
    pub(crate) fn parse(text: &str) -> Result<Self, ConfigProblem> {
        let file: ConfigFile = toml::from_str(text).map_err(|error| ConfigProblem {
            line: error.span().map(|span| line_of(text, span.start)),
            message: error.message().trim_end().to_owned(),
        })?;
        if file.schema.get_ref() != SCHEMA {
            return Err(ConfigProblem {
                line: Some(line_of(text, file.schema.span().start)),
                message: format!(
                    "schema is {}, and flightdeck reads {SCHEMA}",
                    file.schema.get_ref()
                ),
            });
        }
        let mut seen = BTreeSet::new();
        let mut repositories = Vec::with_capacity(file.repositories.len());
        for spanned in file.repositories {
            let line = Some(line_of(text, spanned.span().start));
            let repository = spanned.into_inner();
            let name = RepositoryName::parse(&repository.name).ok_or_else(|| ConfigProblem {
                line,
                message: format!(
                    "repository {} is not a name of the form owner/repo",
                    repository.name
                ),
            })?;
            // GitHub names differ only in case for one repository.
            if !seen.insert(name.to_string().to_ascii_lowercase()) {
                return Err(ConfigProblem {
                    line,
                    message: format!("repository {name} is named twice"),
                });
            }
            repositories.push(Repository {
                name,
                group: repository.group,
            });
        }
        Ok(Self {
            title: file.title,
            stale_after_days: file.stale_after_days.unwrap_or(DEFAULT_STALE_AFTER_DAYS),
            every_hours: file.build.map(|build| build.every_hours),
            imposed: Imposed {
                name: file.imposed.name,
                from: file.imposed.from,
                root: file.imposed.root.unwrap_or_else(|| DEFAULT_ROOT.to_owned()),
            },
            repositories,
        })
    }
}

/// Returns the line, from 1, that holds the byte at `offset` of `text`.
fn line_of(text: &str, offset: usize) -> usize {
    let before = text.get(..offset).unwrap_or(text);
    before.bytes().filter(|byte| *byte == b'\n').count() + 1
}
