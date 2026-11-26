//! The local mxv workspaces: the one library that `flightdeck workspaces
//! list`, `flightdeck workspaces show` and the page's workspaces routes
//! call, so each page is one query and prints the same document as its
//! command.
//!
//! [`list`] runs `mxv list` and `mxv proxy status`, then `mxv status` of
//! each workspace, and joins them by workspace name into a
//! `flightdeck/workspaces-list/v1` document. [`show`] runs `mxv status` of
//! one workspace and `mxv proxy status` into a
//! `flightdeck/workspaces-show/v1` document. Both copy the fields they
//! name one by one, so no path, log file, sandbox text or service variable
//! that mxv prints reaches either document. mxv's own messages, such as a
//! manifest's error, are copied as mxv wrote them.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt;
use std::path::Path;

use crumbkit::{Call, Code, Document, Failure, FieldValue, Listing, Prose, Recovery, Row};
use crumbkit_process::Cancel;
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::spawn::{self, Exit, Finished, MxvLimits, MxvQuery, Program, SpawnError, Stdout};

mod name;
mod wire;

#[cfg(unix)]
#[cfg(test)]
mod tests;

pub use self::name::{NameError, WorkspaceName};

/// The target that every workspace of this machine belongs to.
const TARGET: &str = "local";

/// The program that flightdeck runs when `FLIGHTDECK_MXV` is unset.
const MXV: &str = "mxv";

/// The code of an mxv that does not exist or will not start.
pub const MXV_MISSING: Code = Code::from_spelling("flightdeck::workspaces::mxv_missing");

/// The code of an mxv query that failed or printed no document.
pub const MXV_FAILED: Code = Code::from_spelling("flightdeck::workspaces::mxv_failed");

/// The code of a workspace that mxv does not know.
pub const WORKSPACE_NOT_FOUND: Code = Code::from_spelling("flightdeck::workspaces::not_found");

/// The code of a workspace name that flightdeck refuses before it runs mxv.
pub const WORKSPACE_NAME: Code = Code::from_spelling("flightdeck::workspaces::name");

/// The document that `flightdeck workspaces list` prints.
pub(crate) const WORKSPACES_LIST: Document<Listing<WorkspaceRow, ListSummary>> = Document::new(
    "flightdeck",
    "workspaces-list",
    1,
    "Each local mxv workspace with its checkouts, ports, routes and units, and how many have a unit running.",
)
.when_empty("mxv lists no workspace");

/// The document that `flightdeck workspaces show` prints.
pub(crate) const WORKSPACES_SHOW: Document<Prose<WorkspaceShow>> = Document::new(
    "flightdeck",
    "workspaces-show",
    1,
    "One local mxv workspace with each checkout's branch, commit and changes, and each unit's state, ports and routes.",
);

/// The mxv program that the queries run, or `None` when `mxv` cannot be
/// looked up the same way from every directory.
#[derive(Clone, Debug)]
pub struct Mxv {
    program: Option<Program>,
    limits: MxvLimits,
}

impl Mxv {
    /// Returns the queries of `program`, with the default limits.
    #[must_use]
    pub const fn new(program: Program) -> Self {
        Self {
            program: Some(program),
            limits: MxvLimits::new(),
        }
    }

    /// Returns the mxv that `configured`, the value of `FLIGHTDECK_MXV`,
    /// names, or `mxv` when it is unset, for a flightdeck that started in
    /// `launch` with the search path `search`, as
    /// [`Program::at_launch`] resolves a tool.
    #[must_use]
    pub fn at_launch(
        configured: Option<&OsStr>,
        launch: Option<&Path>,
        search: Option<&OsStr>,
    ) -> Self {
        let name = configured.unwrap_or_else(|| OsStr::new(MXV));
        Self {
            program: Program::at_launch(name, launch, search),
            limits: MxvLimits::new(),
        }
    }

    /// Returns the queries with `limits` in place of the default ones.
    #[must_use]
    pub const fn limits(mut self, limits: MxvLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Returns the file name of the program, for a failure that names it.
    /// A path that `FLIGHTDECK_MXV` gives stays out of the failure, so a
    /// reply never holds a directory of the person's machine.
    fn shown(&self) -> String {
        self.program.as_ref().map_or_else(
            || MXV.to_owned(),
            |program| {
                let name = program.name();
                Path::new(name)
                    .file_name()
                    .unwrap_or(name)
                    .to_string_lossy()
                    .into_owned()
            },
        )
    }

    /// Runs `queries` and returns each one's result in order, or the error
    /// of an mxv that cannot be looked up.
    fn run(
        &self,
        queries: &[MxvQuery],
        budget: spawn::Budget,
        stop: &Cancel,
    ) -> Result<Vec<Result<Finished, SpawnError>>, WorkspacesError> {
        let program = self
            .program
            .as_ref()
            .ok_or_else(|| WorkspacesError::MxvMissing {
                program: self.shown(),
            })?;
        Ok(spawn::run_mxv(program, queries, budget, stop))
    }
}

/// Why a workspaces query gave no document.
#[derive(Debug, Failure)]
pub enum WorkspacesError {
    /// mxv does not exist or will not start.
    #[failure(
        code = MXV_MISSING,
        hint = "install mxv, or set FLIGHTDECK_MXV to its path",
        field(program = program.as_str()),
    )]
    MxvMissing {
        /// The program that flightdeck tried to start.
        program: String,
    },
    /// An mxv query failed, outlasted its time or printed no document.
    #[failure(
        code = MXV_FAILED,
        recovery = Recovery::Retry,
        field(command = *command, mxv_code = code.as_deref().map_or(FieldValue::Null, FieldValue::from)),
    )]
    MxvFailed {
        /// The mxv command that failed, such as `list`.
        command: &'static str,
        /// mxv's own code, when it printed a failure document.
        code: Option<String>,
        /// What went wrong, in mxv's words when it printed a failure.
        message: String,
    },
    /// mxv knows no workspace by this name.
    #[failure(
        code = WORKSPACE_NOT_FOUND,
        recovery = Recovery::Run(Call::new(&["workspaces", "list"])),
        field(name = name.as_str()),
    )]
    NotFound {
        /// The name that was asked for.
        name: String,
    },
    /// The name is not one that flightdeck passes to mxv.
    #[failure(
        code = WORKSPACE_NAME,
        hint = "give a name that `flightdeck workspaces list` prints",
        field(reason = reason.name()),
    )]
    Name {
        /// Why the name was refused.
        reason: NameError,
    },
}

impl fmt::Display for WorkspacesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MxvMissing { program } => write!(
                f,
                "cannot start {program}: install mxv, or set FLIGHTDECK_MXV to its path"
            ),
            Self::MxvFailed {
                command, message, ..
            } => write!(f, "mxv {command} failed: {message}"),
            Self::NotFound { name } => write!(f, "no workspace is named {name}"),
            Self::Name { reason } => reason.fmt(f),
        }
    }
}

impl std::error::Error for WorkspacesError {}

/// A failure that mxv reported for one query, or flightdeck's words for a
/// query that printed no failure document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct MxvError {
    /// mxv's own code, such as `mxv::workspace::not_found`, or null when
    /// mxv printed no failure document.
    pub code: Option<String>,
    /// What went wrong.
    pub message: String,
}

/// One route that the proxy serves for a workspace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct RouteLink {
    /// The route's name in the workspace's manifest.
    pub route: String,
    /// The URL that serves the route.
    pub url: String,
    /// The unit that listens on the route's port, or null when none does.
    pub unit: Option<String>,
}

/// One unit's name and state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct UnitState {
    /// The unit's name in the manifest.
    pub name: String,
    /// The unit's state as `mxv status` reports it, such as `running`.
    pub state: String,
}

/// Whether the proxy's routes could be read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Routes {
    /// `mxv proxy status` answered, so each workspace's routes are known.
    Available,
    /// `mxv proxy status` failed, so no workspace shows a route.
    Unavailable {
        /// Why the routes could not be read.
        reason: String,
    },
}

/// One local workspace, as the Workspaces page draws its card.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct WorkspaceRow {
    /// The workspace's name.
    pub name: String,
    /// The branch that the workspace's checkouts were created on, or null
    /// when mxv cannot read the workspace's record.
    pub branch: Option<String>,
    /// The folder of each checkout, which is its repository's name.
    pub checkouts: Vec<String>,
    /// The host port that the workspace holds for each named port.
    pub ports: BTreeMap<String, u16>,
    /// Why mxv cannot read the workspace's record, or null when it can.
    pub problem: Option<String>,
    /// Each route that the proxy serves for the workspace.
    pub routes: Vec<RouteLink>,
    /// The number of units that the workspace's manifest declares, or null
    /// when `mxv status` failed.
    pub declared: Option<usize>,
    /// The number of units that are running, or null when `mxv status`
    /// failed.
    pub running: Option<usize>,
    /// Whether `mxv reclaim` leaves the workspace running, or null when
    /// `mxv status` failed.
    pub pinned: Option<bool>,
    /// Each unit with its state, or null when `mxv status` failed.
    pub units: Option<Vec<UnitState>>,
    /// Why `mxv status` failed for this workspace, or null when it
    /// answered.
    pub status_error: Option<MxvError>,
}

impl Row for WorkspaceRow {
    const COLUMNS: &'static [&'static str] = &["name", "branch", "running", "declared", "pinned"];
    const LONG_COLUMNS: &'static [&'static str] = &["checkouts", "problem"];
    // A person copies the name into `flightdeck workspaces show`.
    const UNCUT_COLUMNS: &'static [&'static str] = &["name"];
}

/// What the Workspaces page's header says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ListSummary {
    /// The target that the workspaces belong to: `local`, this machine.
    pub target: &'static str,
    /// The number of workspaces.
    pub workspaces: usize,
    /// The number of workspaces with at least one unit running.
    pub running: usize,
    /// The number of workspaces that declare units and have none running.
    pub stopped: usize,
    /// Whether the proxy's routes could be read.
    pub routes: Routes,
}

impl fmt::Display for ListSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} running, {} stopped",
            self.target, self.running, self.stopped
        )?;
        if let Routes::Unavailable { reason } = &self.routes {
            write!(f, "\nroutes unavailable: {reason}")?;
        }
        Ok(())
    }
}

/// The document of the Workspaces page: one row per workspace and the
/// counts of its header.
#[derive(Clone, Debug)]
pub struct WorkspacesList {
    /// One row per workspace, in the order mxv lists them.
    pub rows: Vec<WorkspaceRow>,
    /// The header's counts and the state of the routes.
    pub summary: ListSummary,
}

impl WorkspacesList {
    /// Returns the list as the listing that `flightdeck workspaces list`
    /// prints.
    #[must_use]
    pub fn listing(self) -> Listing<WorkspaceRow, ListSummary> {
        let total = self.rows.len();
        Listing::new(self.rows, total)
            .described("workspaces")
            .singular("workspace")
            .summary(self.summary)
    }

    /// Returns the list as the JSON that `flightdeck workspaces list
    /// --json --full` prints, which the page's route sends.
    #[must_use]
    pub fn to_json(&self) -> Vec<u8> {
        /// The listing's document with every field.
        #[derive(Serialize)]
        struct Whole<'a> {
            schema: String,
            data: &'a [WorkspaceRow],
            total: usize,
            truncated: bool,
            summary: &'a ListSummary,
        }
        // serde_json fails only on a map key that is not a string, and the
        // document's keys are all strings.
        serde_json::to_vec(&Whole {
            schema: schema_id(WORKSPACES_LIST.component(), WORKSPACES_LIST.name()),
            data: &self.rows,
            total: self.rows.len(),
            truncated: false,
            summary: &self.summary,
        })
        .unwrap_or_default()
    }
}

/// One checkout of a workspace on the One workspace page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct CheckoutState {
    /// The checkout's folder in the workspace, which is its repository's
    /// name.
    pub folder: String,
    /// The branch that the checkout has checked out, or null when mxv
    /// cannot tell.
    pub branch: Option<String>,
    /// The commit that the checkout is at, or null when mxv cannot tell.
    pub commit: Option<String>,
    /// The number of files with uncommitted changes, or null when mxv
    /// cannot tell.
    pub changes: Option<u64>,
    /// Why mxv cannot read the checkout, or null when it can.
    pub problem: Option<String>,
}

/// One unit of a workspace on the One workspace page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct UnitDetail {
    /// The unit's name in the manifest.
    pub name: String,
    /// `process` or `container`.
    pub kind: String,
    /// The unit's state as `mxv status` reports it, such as `running`.
    pub state: String,
    /// Whether the unit has been ready since it last started, or null when
    /// it does not run.
    pub ready: Option<bool>,
    /// The ports that the unit receives, as `VARIABLE=port`, or publishes,
    /// as `port-name=port`.
    pub ports: Vec<String>,
    /// The exit code of an exited unit, or null.
    pub code: Option<i64>,
    /// The number of the signal that ended an exited unit, or null.
    pub signal: Option<i64>,
    /// Why mxv cannot tell the unit's state or exit code, or null.
    pub reason: Option<String>,
    /// The URL of each route that the proxy serves for the unit.
    pub routes: Vec<String>,
}

/// The document of the One workspace page.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct WorkspaceShow {
    /// The target that the workspace belongs to: `local`, this machine.
    pub target: &'static str,
    /// The workspace's name.
    pub name: String,
    /// Whether `mxv reclaim` leaves the workspace running.
    pub pinned: bool,
    /// Why mxv cannot read the workspace's manifest, or null when it can.
    pub manifest_error: Option<String>,
    /// mxv's code for the manifest's error, or null.
    pub manifest_error_code: Option<String>,
    /// Each checkout, in the order mxv reports them.
    pub checkouts: Vec<CheckoutState>,
    /// Each unit, in the order mxv reports them.
    pub units: Vec<UnitDetail>,
    /// Whether the proxy's routes could be read.
    pub routes: Routes,
}

impl WorkspaceShow {
    /// Returns the workspace as the JSON that `flightdeck workspaces show
    /// --json` prints, which the page's route sends.
    #[must_use]
    pub fn to_json(&self) -> Vec<u8> {
        /// The prose document, with no failures.
        #[derive(Serialize)]
        struct Whole<'a> {
            schema: String,
            data: &'a WorkspaceShow,
            failures: [(); 0],
        }
        // serde_json fails only on a map key that is not a string, and the
        // document's keys are all strings.
        serde_json::to_vec(&Whole {
            schema: schema_id(WORKSPACES_SHOW.component(), WORKSPACES_SHOW.name()),
            data: self,
            failures: [],
        })
        .unwrap_or_default()
    }
}

impl fmt::Display for WorkspaceShow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} / {}", self.target, self.name)?;
        for checkout in &self.checkouts {
            let branch = checkout.branch.as_deref().unwrap_or("unknown branch");
            let commit = checkout.commit.as_deref().map_or("", short_commit);
            let changes = checkout
                .changes
                .map_or_else(String::new, |count| format!(", {count} changed"));
            writeln!(f, "{}: {branch} {commit}{changes}", checkout.folder)?;
        }
        if let Some(error) = &self.manifest_error {
            writeln!(f, "manifest: {error}")?;
        }
        for unit in &self.units {
            writeln!(f, "{} ({}): {}", unit.name, unit.kind, unit.state)?;
        }
        Ok(())
    }
}

/// Returns the first 7 characters of `commit`.
fn short_commit(commit: &str) -> &str {
    commit.get(..7).unwrap_or(commit)
}

/// Returns the schema id of a document of `component` named `name`,
/// version 1, as crumbkit spells it.
fn schema_id(component: &str, name: &str) -> String {
    format!("{component}/{name}/v1")
}

/// Lists every workspace with its status and routes.
///
/// # Errors
///
/// Returns [`WorkspacesError::MxvMissing`] when mxv will not start, and
/// [`WorkspacesError::MxvFailed`] when `mxv list` fails. A failed
/// `mxv status` marks its own row, and a failed `mxv proxy status` leaves
/// the routes unavailable.
pub fn list(mxv: &Mxv, stop: &Cancel) -> Result<WorkspacesList, WorkspacesError> {
    let budget = mxv.limits.start();
    let mut first = mxv
        .run(&[MxvQuery::List, MxvQuery::ProxyStatus], budget, stop)?
        .into_iter();
    let listed: wire::List =
        read(first.next(), wire::LIST).map_err(|answer| answer.into_error(mxv, "list"))?;
    let (routes, proxy) = routes_of(first.next());
    let names: Vec<Result<WorkspaceName, NameError>> = listed
        .data
        .iter()
        .map(|row| WorkspaceName::new(&row.name))
        .collect();
    let queries: Vec<MxvQuery> = names
        .iter()
        .flatten()
        .map(|name| MxvQuery::Status(name.clone()))
        .collect();
    let mut statuses = mxv.run(&queries, budget, stop)?.into_iter();
    let rows: Vec<WorkspaceRow> = listed
        .data
        .into_iter()
        .zip(names)
        .map(|(row, name)| {
            let status = match name {
                Ok(_) => read::<wire::Status>(statuses.next(), wire::STATUS)
                    .map_err(|answer| answer.into_mxv_error()),
                Err(reason) => Err(MxvError {
                    code: None,
                    message: format!("flightdeck does not ask mxv about it: {reason}"),
                }),
            };
            workspace_row(row, status, &proxy)
        })
        .collect();
    let running = rows
        .iter()
        .filter(|row| row.running.is_some_and(|count| count > 0))
        .count();
    let stopped = rows
        .iter()
        .filter(|row| row.running == Some(0) && row.declared.is_some_and(|count| count > 0))
        .count();
    Ok(WorkspacesList {
        summary: ListSummary {
            target: TARGET,
            workspaces: rows.len(),
            running,
            stopped,
            routes,
        },
        rows,
    })
}

/// Shows one workspace with its checkouts, units and routes.
///
/// # Errors
///
/// Returns [`WorkspacesError::NotFound`] when mxv knows no workspace named
/// `name`, [`WorkspacesError::MxvMissing`] when mxv will not start, and
/// [`WorkspacesError::MxvFailed`] when `mxv status` fails otherwise.
pub fn show(
    mxv: &Mxv,
    name: &WorkspaceName,
    stop: &Cancel,
) -> Result<WorkspaceShow, WorkspacesError> {
    let budget = mxv.limits.start();
    let mut answers = mxv
        .run(
            &[MxvQuery::Status(name.clone()), MxvQuery::ProxyStatus],
            budget,
            stop,
        )?
        .into_iter();
    let status: wire::Status = read(answers.next(), wire::STATUS).map_err(|answer| {
        if answer.code() == Some(MXV_NOT_FOUND) {
            WorkspacesError::NotFound {
                name: name.to_string(),
            }
        } else {
            answer.into_error(mxv, "status")
        }
    })?;
    let (routes, proxy) = routes_of(answers.next());
    let mine: Vec<&wire::Route> = proxy
        .iter()
        .filter(|route| route.workspace == name.as_str() && route.url != wire::NOT_SERVED)
        .collect();
    let units = status
        .data
        .into_iter()
        .map(|unit| UnitDetail {
            routes: mine
                .iter()
                .filter(|route| route.unit.as_deref() == Some(unit.name.as_str()))
                .map(|route| route.url.clone())
                .collect(),
            name: unit.name,
            kind: unit.kind,
            state: unit.state,
            ready: unit.ready,
            ports: unit.ports,
            code: unit.code,
            signal: unit.signal,
            reason: unit.reason,
        })
        .collect();
    let summary = status.summary;
    Ok(WorkspaceShow {
        target: TARGET,
        name: name.to_string(),
        pinned: summary.pinned,
        manifest_error: summary.manifest_error,
        manifest_error_code: summary.manifest_error_code,
        checkouts: summary
            .checkouts
            .into_iter()
            .map(|checkout| CheckoutState {
                folder: checkout.name,
                branch: checkout.branch,
                commit: checkout.commit,
                changes: checkout.changes,
                problem: checkout.problem,
            })
            .collect(),
        units,
        routes,
    })
}

/// mxv's code of a workspace that it does not know.
const MXV_NOT_FOUND: &str = "mxv::workspace::not_found";

/// mxv's code of a command line that it refuses, which an mxv without
/// `proxy status` prints for that command.
const MXV_USAGE: &str = "mxv::cli::usage";

/// Returns the row of `row` from `mxv list`, joined with its status and
/// with the routes of `proxy` that name it. Only routes that the proxy
/// serves are kept.
fn workspace_row(
    row: wire::ListRow,
    status: Result<wire::Status, MxvError>,
    proxy: &[wire::Route],
) -> WorkspaceRow {
    let routes = proxy
        .iter()
        .filter(|route| route.workspace == row.name && route.url != wire::NOT_SERVED)
        .map(|route| RouteLink {
            route: route.route.clone(),
            url: route.url.clone(),
            unit: route.unit.clone(),
        })
        .collect();
    let mut joined = WorkspaceRow {
        branch: row.branch.filter(|branch| !branch.is_empty()),
        checkouts: row
            .checkouts
            .into_iter()
            .map(|checkout| checkout.name)
            .collect(),
        ports: row.ports,
        problem: row.problem.filter(|problem| !problem.is_empty()),
        routes,
        declared: None,
        running: None,
        pinned: None,
        units: None,
        status_error: None,
        name: row.name,
    };
    match status {
        Ok(status) => {
            joined.declared = Some(status.data.len());
            joined.running = Some(
                status
                    .data
                    .iter()
                    .filter(|unit| unit.state == "running")
                    .count(),
            );
            joined.pinned = Some(status.summary.pinned);
            joined.units = Some(
                status
                    .data
                    .into_iter()
                    .map(|unit| UnitState {
                        name: unit.name,
                        state: unit.state,
                    })
                    .collect(),
            );
        }
        Err(error) => joined.status_error = Some(error),
    }
    joined
}

/// Returns whether the routes could be read and the routes of the answer
/// of `mxv proxy status`, none when it failed.
fn routes_of(answer: Option<Result<Finished, SpawnError>>) -> (Routes, Vec<wire::Route>) {
    match read::<wire::ProxyStatus>(answer, wire::PROXY_STATUS) {
        Ok(proxy) => (Routes::Available, proxy.data),
        Err(answer) => {
            let reason = if answer.code() == Some(MXV_USAGE) {
                "this mxv has no proxy status command".to_owned()
            } else {
                answer.into_mxv_error().message
            };
            (Routes::Unavailable { reason }, Vec::new())
        }
    }
}

/// Why a query gave no document of the kind it asks for.
#[derive(Debug)]
enum Answer {
    /// mxv did not start.
    Missing,
    /// mxv printed a failure document, or flightdeck's words for a query
    /// that printed none.
    Failed(MxvError),
}

impl Answer {
    /// Returns mxv's code, when it printed a failure document.
    fn code(&self) -> Option<&str> {
        match self {
            Self::Missing => None,
            Self::Failed(error) => error.code.as_deref(),
        }
    }

    /// Returns the answer as the failure of a row.
    fn into_mxv_error(self) -> MxvError {
        match self {
            Self::Missing => MxvError {
                code: None,
                message: "mxv did not start".to_owned(),
            },
            Self::Failed(error) => error,
        }
    }

    /// Returns the answer as the error of a query of `mxv` whose
    /// `command`, such as `list`, failed.
    fn into_error(self, mxv: &Mxv, command: &'static str) -> WorkspacesError {
        match self {
            Self::Missing => WorkspacesError::MxvMissing {
                program: mxv.shown(),
            },
            Self::Failed(error) => WorkspacesError::MxvFailed {
                command,
                code: error.code,
                message: error.message,
            },
        }
    }
}

/// Reads the document whose schema id is `schema` from a query's result,
/// or says why there is none.
fn read<T: DeserializeOwned>(
    result: Option<Result<Finished, SpawnError>>,
    schema: &str,
) -> Result<T, Answer> {
    let failed = |message: String| {
        Answer::Failed(MxvError {
            code: None,
            message,
        })
    };
    let finished = match result {
        Some(Ok(finished)) => finished,
        Some(Err(SpawnError::Start(_))) => return Err(Answer::Missing),
        Some(Err(SpawnError::Run(_))) => {
            return Err(failed(
                "flightdeck could not follow mxv to its end".to_owned(),
            ));
        }
        None => return Err(failed("the query did not run".to_owned())),
    };
    let bytes = match (finished.exit(), finished.stdout()) {
        (Exit::TimedOut, _) => {
            return Err(failed(format!(
                "mxv did not answer within {} seconds",
                spawn::MXV_TIME_LIMIT.as_secs()
            )));
        }
        (_, Stdout::OverLimit) => {
            return Err(failed("mxv printed more than flightdeck reads".to_owned()));
        }
        (_, Stdout::Whole(bytes)) => bytes,
    };
    let head: Option<wire::Head> = serde_json::from_slice(bytes).ok();
    match head.as_ref().map(|head| head.schema.as_str()) {
        Some(found) if found == schema => serde_json::from_slice(bytes).map_err(|_| {
            failed(format!(
                "mxv printed a {schema} document that flightdeck cannot read"
            ))
        }),
        Some(wire::FAILURE) => match serde_json::from_slice::<wire::Failure>(bytes) {
            Ok(failure) => Err(Answer::Failed(MxvError {
                code: Some(failure.code),
                message: failure.message,
            })),
            Err(_) => Err(failed(
                "mxv printed a failure that flightdeck cannot read".to_owned(),
            )),
        },
        _ => Err(failed(match finished.exit() {
            Exit::Code(code) => {
                format!("mxv exited with code {code} and printed no {schema} document")
            }
            Exit::Signal(signal) => {
                format!("a signal ({signal}) ended mxv before it printed a document")
            }
            Exit::TimedOut | Exit::Unknown => format!("mxv printed no {schema} document"),
        })),
    }
}
