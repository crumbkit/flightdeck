//! The workflow graphs of a board: the one library that `flightdeck
//! workflows list`, `flightdeck workflows show` and the page's workflows
//! routes call, so each page is one query and prints the same document as
//! its command.
//!
//! [`list`] reads every `flightdeck/workflow-graph/v1` record under
//! `/workflow-graphs/` into a `flightdeck/workflows-list/v1` document, with
//! each graph's node count and whether it is drawn, degraded or has
//! problems. [`show`] reads one graph into a `flightdeck/workflows-show/v1`
//! document: the record, the problems its check finds, the live records
//! each binding places on it, how many earlier readings `board log` keeps,
//! and whether its subject changed since the reading.
//!
//! Everything is derived on each read, and nothing is stored.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

use crumbkit::{Call, Code, Document, Failure, FieldValue, Listing, Prose, Recovery, Row};
use crumbkit_process::Cancel;
use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::spawn::{
    self, BoardAt, BoardLimits, BoardPath, BoardQuery, Commit, Exit, Finished, GRAPHS_UNDER,
    Program, RecordType, SpawnError, Stdout,
};
use crate::workflow_graph::{
    self, Binding, MAX_BINDINGS, MAX_EDGES, MAX_GROUPS, MAX_LANES, MAX_NODES, MAX_SOURCES, Op,
    Problem, WorkflowGraph,
};

mod wire;

#[cfg(unix)]
#[cfg(test)]
mod tests;

/// The program that flightdeck runs when `FLIGHTDECK_BOARD_PROGRAM` is
/// unset.
const BOARD: &str = "board";

/// The most records that one binding places. The rest are counted.
pub const PLACED_LIMIT: usize = 500;

/// The path under which every flow version lives. A version never changes,
/// so a reading of one is checked against the running flow, which Board
/// cannot name yet.
const FLOWS: &str = "/flows/";

/// The code of a Board that does not exist or will not start.
pub const BOARD_NOT_FOUND: Code = Code::from_spelling("flightdeck::board::not_found");

/// The code of a Board query that failed or printed no document.
pub const BOARD_FAILED: Code = Code::from_spelling("flightdeck::board::failed");

/// The code of a slug that names no graph.
pub const GRAPH_NOT_FOUND: Code = Code::from_spelling("flightdeck::workflows::not_found");

/// The code of a slug that flightdeck refuses before it runs Board.
pub const GRAPH_SLUG: Code = Code::from_spelling("flightdeck::workflows::slug");

/// The document that `flightdeck workflows list` prints.
pub(crate) const WORKFLOWS_LIST: Document<Listing<GraphRow, ListSummary>> = Document::new(
    "flightdeck",
    "workflows-list",
    1,
    "Each workflow graph record under /workflow-graphs/ in the board, with its title, subject, node count and whether Flightdeck draws it.",
)
.when_empty("the board holds no workflow graph");

/// The document that `flightdeck workflows show` prints.
pub(crate) const WORKFLOWS_SHOW: Document<Prose<WorkflowShow>> = Document::new(
    "flightdeck",
    "workflows-show",
    1,
    "One workflow graph with its problems, the live records each binding places on it, its earlier readings and whether its subject changed since.",
);

/// A graph's slug: lowercase letters and digits in words joined by `-`,
/// such as `development-flow`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Slug(String);

/// Why a slug was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlugError {
    /// The slug is not lowercase words joined by `-`.
    Shape,
}

impl SlugError {
    /// Returns the reason's name, for the failure's field.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Shape => "shape",
        }
    }
}

impl fmt::Display for SlugError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a graph's slug is lowercase letters and digits in words joined by -, such as development-flow")
    }
}

impl std::error::Error for SlugError {}

impl Slug {
    /// Returns the slug `text`, or why it is refused.
    ///
    /// # Errors
    ///
    /// Returns [`SlugError::Shape`] unless `text` is lowercase letters and
    /// digits in words joined by single hyphens.
    pub fn new(text: &str) -> Result<Self, SlugError> {
        let words = text.split('-');
        let mut any = false;
        for word in words {
            if word.is_empty()
                || !word
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            {
                return Err(SlugError::Shape);
            }
            any = true;
        }
        if any {
            Ok(Self(text.to_owned()))
        } else {
            Err(SlugError::Shape)
        }
    }

    /// Returns the slug as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the graph's path, `/workflow-graphs/<slug>`.
    fn path(&self) -> String {
        format!("{GRAPHS_UNDER}{}", self.0)
    }
}

impl std::str::FromStr for Slug {
    type Err = SlugError;

    fn from_str(text: &str) -> Result<Self, SlugError> {
        Self::new(text)
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The Board program that the queries run and the board they read, or no
/// program when `board` cannot be looked up the same way from every
/// directory.
#[derive(Clone, Debug)]
pub struct Board {
    program: Option<Program>,
    at: BoardAt,
    limits: BoardLimits,
}

impl Board {
    /// Returns the queries of `program` on the board `at`, with the
    /// default limits.
    #[must_use]
    pub const fn new(program: Program, at: BoardAt) -> Self {
        Self {
            program: Some(program),
            at,
            limits: BoardLimits::new(),
        }
    }

    /// Returns the Board that `configured`, the value of
    /// `FLIGHTDECK_BOARD_PROGRAM`, names, or `board` when it is unset, on
    /// the board in the directory that `board`, the value of
    /// `FLIGHTDECK_BOARD`, names, or the personal board when it is unset,
    /// for a flightdeck that started in `launch` with the search path
    /// `search`.
    #[must_use]
    pub fn at_launch(
        configured: Option<&OsStr>,
        board: Option<&OsStr>,
        launch: Option<&Path>,
        search: Option<&OsStr>,
    ) -> Self {
        let name = configured.unwrap_or_else(|| OsStr::new(BOARD));
        let at = board.map_or(BoardAt::Personal, |directory| {
            let directory = PathBuf::from(directory);
            BoardAt::Directory(match launch {
                Some(launch) if directory.is_relative() => launch.join(directory),
                _ => directory,
            })
        });
        Self {
            program: Program::at_launch(name, launch, search),
            at,
            limits: BoardLimits::new(),
        }
    }

    /// Returns the queries with `limits` in place of the default ones.
    #[must_use]
    pub const fn limits(mut self, limits: BoardLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Returns the file name of the program, for a failure that names it,
    /// so a reply never holds a directory of the person's machine.
    fn shown(&self) -> String {
        self.program.as_ref().map_or_else(
            || BOARD.to_owned(),
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

    /// Runs `queries` within `budget` and returns each one's result in
    /// order, or the error of a Board that cannot be looked up.
    fn run(
        &self,
        queries: &[BoardQuery],
        budget: spawn::BoardBudget,
        stop: &Cancel,
    ) -> Result<Vec<Result<Finished, SpawnError>>, WorkflowsError> {
        let program = self
            .program
            .as_ref()
            .ok_or_else(|| WorkflowsError::BoardMissing {
                program: self.shown(),
            })?;
        Ok(spawn::run_board(program, &self.at, queries, budget, stop))
    }
}

/// Why a workflows query gave no document.
#[derive(Debug, Failure)]
pub enum WorkflowsError {
    /// Board does not exist or will not start.
    #[failure(
        code = BOARD_NOT_FOUND,
        hint = "install Board, or set FLIGHTDECK_BOARD_PROGRAM to its path",
        field(program = program.as_str()),
    )]
    BoardMissing {
        /// The program that flightdeck tried to start.
        program: String,
    },
    /// A Board query failed, outlasted its time or printed no document.
    #[failure(
        code = BOARD_FAILED,
        recovery = Recovery::Retry,
        field(command = *command, board_code = code.as_deref().map_or(FieldValue::Null, FieldValue::from)),
    )]
    BoardFailed {
        /// The Board command that failed, such as `query`.
        command: &'static str,
        /// Board's own code, when it printed a failure document.
        code: Option<String>,
        /// What went wrong, in Board's words when it printed a failure.
        message: String,
    },
    /// The board holds no graph at this slug.
    #[failure(
        code = GRAPH_NOT_FOUND,
        recovery = Recovery::Run(Call::new(&["workflows", "list"])),
        field(slug = slug.as_str()),
    )]
    NotFound {
        /// The slug that was asked for.
        slug: String,
    },
    /// The slug is not one that flightdeck reads.
    #[failure(
        code = GRAPH_SLUG,
        hint = "give a slug that `flightdeck workflows list` prints",
        field(reason = reason.name()),
    )]
    Slug {
        /// Why the slug was refused.
        reason: SlugError,
    },
}

impl fmt::Display for WorkflowsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BoardMissing { program } => write!(
                f,
                "cannot start {program}: install Board, or set FLIGHTDECK_BOARD_PROGRAM to its path"
            ),
            Self::BoardFailed {
                command,
                code,
                message,
            } => match code {
                Some(code) => write!(f, "board {command} failed with {code}: {message}"),
                None => write!(f, "board {command} failed: {message}"),
            },
            Self::NotFound { slug } => {
                write!(f, "the board holds no graph at {GRAPHS_UNDER}{slug}")
            }
            Self::Slug { reason } => reason.fmt(f),
        }
    }
}

impl std::error::Error for WorkflowsError {}

impl WorkflowsError {
    /// Returns whether Board itself could not start, so the page hides its
    /// Workflows entry.
    #[must_use]
    pub const fn no_board(&self) -> bool {
        matches!(self, Self::BoardMissing { .. })
    }
}

/// A failure that Board reported for one query, or flightdeck's words for
/// a query that printed no failure document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct BoardError {
    /// Board's own code, such as `board::path::not_found`, or null when
    /// Board printed no failure document.
    pub code: Option<String>,
    /// What went wrong.
    pub message: String,
}

/// Whether Flightdeck draws a graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GraphStatus {
    /// The record passes its schema and the check, so the page draws it.
    Ok,
    /// Board marks the record degraded, or flightdeck cannot read it, so
    /// the page draws no graph and shows the code.
    Degraded,
    /// The check finds problems, so the page lists them and draws no graph.
    Problems,
}

impl fmt::Display for GraphStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ok => "ok",
            Self::Degraded => "degraded",
            Self::Problems => "problems",
        })
    }
}

/// One graph, as the Workflows rail and switch name it.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct GraphRow {
    /// The graph's slug, the last part of its path.
    pub slug: String,
    /// The graph's title, or null when flightdeck cannot read the record.
    pub title: Option<String>,
    /// The path of the record or file the graph reads, or null.
    pub subject: Option<String>,
    /// When the reading was written, or null.
    pub made: Option<String>,
    /// The number of nodes, or null.
    pub nodes: Option<usize>,
    /// Whether Flightdeck draws the graph.
    pub status: GraphStatus,
    /// Board's code, or flightdeck's, of a degraded record, or null.
    pub code: Option<String>,
    /// The number of problems that the check finds.
    pub problems: usize,
}

impl Row for GraphRow {
    const COLUMNS: &'static [&'static str] = &["slug", "title", "nodes", "status", "problems"];
    const LONG_COLUMNS: &'static [&'static str] = &["subject", "made", "code"];
    // A person copies the slug into `flightdeck workflows show`.
    const UNCUT_COLUMNS: &'static [&'static str] = &["slug"];
}

/// What the Workflows rail entry says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ListSummary {
    /// The number of graphs.
    pub graphs: usize,
    /// The number of graphs that Flightdeck draws.
    pub drawn: usize,
}

impl fmt::Display for ListSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} graphs, {} drawn", self.graphs, self.drawn)
    }
}

/// The document of the Workflows rail and switch.
#[derive(Clone, Debug)]
pub struct WorkflowsList {
    /// One row per graph, in the order of their paths.
    pub rows: Vec<GraphRow>,
    /// The counts.
    pub summary: ListSummary,
}

impl WorkflowsList {
    /// Returns the list as the listing that `flightdeck workflows list`
    /// prints.
    #[must_use]
    pub fn listing(self) -> Listing<GraphRow, ListSummary> {
        let total = self.rows.len();
        Listing::new(self.rows, total)
            .described("graphs")
            .singular("graph")
            .summary(self.summary)
    }

    /// Returns the list as the JSON that `flightdeck workflows list --json
    /// --full` prints, which the page's route sends.
    #[must_use]
    pub fn to_json(&self) -> Vec<u8> {
        /// The listing's document with every field.
        #[derive(Serialize)]
        struct Whole<'a> {
            schema: &'static str,
            data: &'a [GraphRow],
            total: usize,
            truncated: bool,
            summary: &'a ListSummary,
        }
        // serde_json fails only on a map key that is not a string, and the
        // document's keys are all strings.
        serde_json::to_vec(&Whole {
            schema: "flightdeck/workflows-list/v1",
            data: &self.rows,
            total: self.rows.len(),
            truncated: false,
            summary: &self.summary,
        })
        .unwrap_or_default()
    }
}

/// Whether the reading is as new as its subject.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Freshness {
    /// No commit after the reading's commit changed the subject.
    Current,
    /// A commit after the reading's commit changed the subject.
    Changed,
    /// Nothing is at the subject's path.
    Gone,
    /// Flightdeck did not compare the reading with its subject.
    Unchecked {
        /// Why not, such as a flow, whose running version Board cannot
        /// name yet.
        reason: String,
    },
}

/// One live record that a binding places on the graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Placed {
    /// The record's path, such as `/changes/add-a-page`.
    pub path: String,
    /// The record's `title`, or the last part of its path when it has none.
    pub title: String,
    /// The workflow version the record follows, from the binding's
    /// `version` field, or null when the binding names none.
    pub version: Option<String>,
    /// Whether the record follows the version that the graph reads.
    pub current: bool,
}

/// The live records of one binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct BindingState {
    /// The node or group the records sit at.
    pub target: String,
    /// The records placed there, by path, at most [`PLACED_LIMIT`].
    pub records: Vec<Placed>,
    /// The records that matched past [`PLACED_LIMIT`], which are not placed.
    pub left_out: usize,
    /// The records left out because Board marks them degraded, or a field
    /// a condition reads is not a string.
    pub skipped: usize,
    /// Why the records of the binding's type could not be read, or null.
    pub error: Option<BoardError>,
}

/// The document of the Workflows page for one graph.
#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct WorkflowShow {
    /// The graph's slug.
    pub slug: String,
    /// The graph's path, `/workflow-graphs/<slug>`.
    pub path: String,
    /// Whether Flightdeck draws the graph.
    pub status: GraphStatus,
    /// Board's code, or flightdeck's, of a degraded record, or null.
    pub code: Option<String>,
    /// The record, or null when flightdeck cannot read it.
    pub graph: Option<WorkflowGraph>,
    /// Each problem the check finds, which the page lists in place of the
    /// graph.
    pub problems: Vec<Problem>,
    /// The first size limit the graph passes, such as `at most 8 lanes`,
    /// when a board without the plugin's schema holds it, or null. The
    /// page then draws it as a run sheet on every screen.
    pub limit: Option<String>,
    /// The live records of each binding, in the order of the bindings.
    pub bindings: Vec<BindingState>,
    /// The number of earlier readings that `board log` keeps, or null when
    /// the log could not be read.
    pub readings: Option<usize>,
    /// Whether the reading is as new as its subject.
    pub freshness: Freshness,
    /// The version of the workflow that runs now, or null while Board
    /// cannot name it, as it cannot for any flow yet.
    pub running: Option<String>,
}

impl WorkflowShow {
    /// Returns the graph as the JSON that `flightdeck workflows show
    /// --json` prints, which the page's route sends.
    #[must_use]
    pub fn to_json(&self) -> Vec<u8> {
        /// The prose document, with no failures.
        #[derive(Serialize)]
        struct Whole<'a> {
            schema: &'static str,
            data: &'a WorkflowShow,
            failures: [(); 0],
        }
        // serde_json fails only on a map key that is not a string, and the
        // document's keys are all strings.
        serde_json::to_vec(&Whole {
            schema: "flightdeck/workflows-show/v1",
            data: self,
            failures: [],
        })
        .unwrap_or_default()
    }
}

impl fmt::Display for WorkflowShow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let title = self
            .graph
            .as_ref()
            .map_or(self.slug.as_str(), |graph| graph.graph.title.as_str());
        writeln!(f, "{title} ({})", self.path)?;
        if let Some(graph) = &self.graph {
            let graph = &graph.graph;
            writeln!(
                f,
                "reads {} at {}",
                graph.subject.reference,
                short_commit(&graph.subject.commit)
            )?;
            writeln!(
                f,
                "{} lanes, {} groups, {} nodes, {} edges",
                graph.lanes.len(),
                graph.groups.len(),
                graph.nodes.len(),
                graph.edges.len()
            )?;
        }
        match &self.code {
            Some(code) => writeln!(f, "status: {} ({code})", self.status)?,
            None => writeln!(f, "status: {}", self.status)?,
        }
        for problem in &self.problems {
            writeln!(
                f,
                "{} at {}: {}",
                problem.code, problem.pointer, problem.message
            )?;
        }
        for binding in &self.bindings {
            let older = binding
                .records
                .iter()
                .filter(|placed| !placed.current)
                .count();
            write!(f, "{}: {} placed", binding.target, binding.records.len())?;
            if older > 0 {
                write!(f, ", {older} on another version")?;
            }
            if binding.skipped > 0 {
                write!(f, ", {} skipped", binding.skipped)?;
            }
            if binding.left_out > 0 {
                write!(f, ", {} left out", binding.left_out)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Returns the first 8 characters of `commit`.
fn short_commit(commit: &str) -> &str {
    commit.get(..8).unwrap_or(commit)
}

/// One graph record that the graphs query read, as the list and the page
/// see it.
struct Read {
    slug: String,
    status: GraphStatus,
    code: Option<String>,
    graph: Option<WorkflowGraph>,
    problems: Vec<Problem>,
}

/// The code of a record that Board admits but flightdeck cannot read, such
/// as one under `/workflow-graphs/` of another type.
const UNREADABLE: &str = "flightdeck::workflows::unreadable";

/// Returns the graph record of `row`, or `None` when the row is not a
/// direct child of `/workflow-graphs/`.
fn read_row(row: &wire::Row) -> Option<Read> {
    let slug = row.logical.strip_prefix(GRAPHS_UNDER)?;
    if slug.is_empty() || slug.contains('/') {
        return None;
    }
    let degraded = |code: Option<String>| Read {
        slug: slug.to_owned(),
        status: GraphStatus::Degraded,
        code,
        graph: None,
        problems: Vec::new(),
    };
    if row.status != "ok" {
        return Some(degraded(row.code.clone()));
    }
    if row.record_type.as_deref() != Some(workflow_graph::TYPE) {
        return Some(degraded(Some(UNREADABLE.to_owned())));
    }
    let Some(graph) = row
        .body
        .as_deref()
        .and_then(|body| serde_json::from_str::<WorkflowGraph>(body).ok())
    else {
        return Some(degraded(Some(UNREADABLE.to_owned())));
    };
    let problems = workflow_graph::check(&graph);
    Some(Read {
        slug: slug.to_owned(),
        status: if problems.is_empty() {
            GraphStatus::Ok
        } else {
            GraphStatus::Problems
        },
        code: None,
        graph: Some(graph),
        problems,
    })
}

/// Reads every graph record of the board.
fn read_graphs(board: &Board, stop: &Cancel) -> Result<Vec<Read>, WorkflowsError> {
    let budget = board.limits.start();
    let mut answers = board.run(&[BoardQuery::Graphs], budget, stop)?.into_iter();
    let query: wire::Query =
        read(answers.next(), wire::QUERY).map_err(|answer| answer.into_error(board, "query"))?;
    Ok(query.rows().iter().filter_map(read_row).collect())
}

/// Lists every graph of the board.
///
/// # Errors
///
/// Returns [`WorkflowsError::BoardMissing`] when Board will not start, and
/// [`WorkflowsError::BoardFailed`] when its query fails.
pub fn list(board: &Board, stop: &Cancel) -> Result<WorkflowsList, WorkflowsError> {
    let rows: Vec<GraphRow> = read_graphs(board, stop)?
        .into_iter()
        .map(|read| GraphRow {
            title: read.graph.as_ref().map(|graph| graph.graph.title.clone()),
            subject: read
                .graph
                .as_ref()
                .map(|graph| graph.graph.subject.reference.clone()),
            made: read.graph.as_ref().map(|graph| graph.made.clone()),
            nodes: read.graph.as_ref().map(|graph| graph.graph.nodes.len()),
            slug: read.slug,
            status: read.status,
            code: read.code,
            problems: read.problems.len(),
        })
        .collect();
    let drawn = rows
        .iter()
        .filter(|row| row.status == GraphStatus::Ok)
        .count();
    Ok(WorkflowsList {
        summary: ListSummary {
            graphs: rows.len(),
            drawn,
        },
        rows,
    })
}

/// What the check of one graph found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checked {
    /// Flightdeck read the record, and the check found these problems, or
    /// none.
    Problems(Vec<Problem>),
    /// Board marks the record degraded, or flightdeck cannot read it, with
    /// this code.
    Degraded(Option<String>),
}

/// Checks the graph at `slug`, as the plugin's `flightdeck.check` does.
///
/// # Errors
///
/// Returns [`WorkflowsError::NotFound`] when the board holds no graph at
/// `slug`, and the errors of [`list`] when Board fails.
pub fn check(board: &Board, slug: &Slug, stop: &Cancel) -> Result<Checked, WorkflowsError> {
    let found = read_graphs(board, stop)?
        .into_iter()
        .find(|read| read.slug == slug.as_str())
        .ok_or_else(|| WorkflowsError::NotFound {
            slug: slug.to_string(),
        })?;
    Ok(match found.graph {
        Some(_) => Checked::Problems(found.problems),
        None => Checked::Degraded(found.code),
    })
}

/// Shows one graph with its problems, its live records, its earlier
/// readings and its subject's freshness.
///
/// # Errors
///
/// Returns [`WorkflowsError::NotFound`] when the board holds no graph at
/// `slug`, [`WorkflowsError::BoardMissing`] when Board will not start, and
/// [`WorkflowsError::BoardFailed`] when the graphs query fails. A failed
/// query of the bindings or the logs marks only what it reads.
pub fn show(board: &Board, slug: &Slug, stop: &Cancel) -> Result<WorkflowShow, WorkflowsError> {
    let found = read_graphs(board, stop)?
        .into_iter()
        .find(|read| read.slug == slug.as_str())
        .ok_or_else(|| WorkflowsError::NotFound {
            slug: slug.to_string(),
        })?;
    let path = slug.path();
    let Some(graph) = found.graph else {
        return Ok(WorkflowShow {
            slug: found.slug,
            path,
            status: found.status,
            code: found.code,
            graph: None,
            problems: Vec::new(),
            limit: None,
            bindings: Vec::new(),
            readings: None,
            freshness: Freshness::Unchecked {
                reason: "flightdeck cannot read the record".to_owned(),
            },
            running: None,
        });
    };

    let types: Vec<RecordType> = graph
        .graph
        .bindings
        .iter()
        .filter_map(|binding| RecordType::new(&binding.record_type))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let subject = &graph.graph.subject;
    let freshness_queries = freshness_queries(&subject.reference, &subject.commit);
    let mut queries: Vec<BoardQuery> = types.iter().cloned().map(BoardQuery::OfType).collect();
    let graph_log = BoardPath::new(&path).map(|graph_path| BoardQuery::Log {
        path: graph_path,
        at: None,
        every: true,
    });
    let logs_readings = graph_log.is_some();
    queries.extend(graph_log);
    queries.extend(freshness_queries.iter().cloned());
    let mut answers = board.run(&queries, board.limits.start(), stop)?.into_iter();

    let mut by_type: BTreeMap<String, Result<Vec<wire::Row>, BoardError>> = BTreeMap::new();
    for record_type in &types {
        let rows = read::<wire::Query>(answers.next(), wire::QUERY)
            .map(wire::Query::rows)
            .map_err(Answer::into_board_error);
        by_type.insert(record_type.to_string(), rows);
    }
    let readings = if logs_readings {
        read::<wire::Log>(answers.next(), wire::LOG)
            .ok()
            .map(|log| log.data.len().saturating_sub(1))
    } else {
        None
    };
    let freshness = freshness(&subject.reference, &mut answers);
    let bindings = graph
        .graph
        .bindings
        .iter()
        .map(|binding| place(binding, &subject.reference, &by_type))
        .collect();
    Ok(WorkflowShow {
        slug: found.slug,
        path,
        status: found.status,
        code: found.code,
        limit: limit_passed(&graph),
        problems: found.problems,
        graph: Some(graph),
        bindings,
        readings,
        freshness,
        running: None,
    })
}

/// Returns the queries that tell whether the subject at `reference`
/// changed after `commit`: the newest commit that changed it from the
/// head, and, except for a flow, the newest from `commit`.
fn freshness_queries(reference: &str, commit: &str) -> Vec<BoardQuery> {
    let Some(path) = BoardPath::new(reference) else {
        return Vec::new();
    };
    let mut queries = vec![BoardQuery::Log {
        path: path.clone(),
        at: None,
        every: false,
    }];
    if !reference.starts_with(FLOWS)
        && let Some(commit) = Commit::new(commit)
    {
        queries.push(BoardQuery::Log {
            path,
            at: Some(commit),
            every: false,
        });
    }
    queries
}

/// Returns whether the subject at `reference` changed since the reading,
/// from the answers of [`freshness_queries`].
fn freshness(
    reference: &str,
    answers: &mut impl Iterator<Item = Result<Finished, SpawnError>>,
) -> Freshness {
    if BoardPath::new(reference).is_none() {
        return Freshness::Unchecked {
            reason: "the subject is not a board path".to_owned(),
        };
    }
    let newest = |answer: Option<Result<Finished, SpawnError>>| {
        read::<wire::Log>(answer, wire::LOG).map(|log| log.data.into_iter().next())
    };
    let head = match newest(answers.next()) {
        Ok(Some(version)) if version.blob.is_some() => version,
        Ok(_) => return Freshness::Gone,
        Err(answer) if answer.code() == Some(PATH_NOT_FOUND) => return Freshness::Gone,
        Err(answer) => {
            return Freshness::Unchecked {
                reason: answer.into_board_error().message,
            };
        }
    };
    if reference.starts_with(FLOWS) {
        return Freshness::Unchecked {
            reason: "Board cannot name the running version of a flow yet".to_owned(),
        };
    }
    match newest(answers.next()) {
        Ok(Some(read_at)) if read_at.commit == head.commit => Freshness::Current,
        Ok(_) => Freshness::Changed,
        Err(answer) => Freshness::Unchecked {
            reason: answer.into_board_error().message,
        },
    }
}

/// Board's code of a path that names nothing.
const PATH_NOT_FOUND: &str = "board::path::not_found";

/// Returns the live records that `binding` places, from the records of
/// each type in `by_type`, marking those on another version than
/// `subject`.
fn place(
    binding: &Binding,
    subject: &str,
    by_type: &BTreeMap<String, Result<Vec<wire::Row>, BoardError>>,
) -> BindingState {
    let mut state = BindingState {
        target: binding.target.clone(),
        records: Vec::new(),
        left_out: 0,
        skipped: 0,
        error: None,
    };
    let rows = match by_type.get(&binding.record_type) {
        Some(Ok(rows)) => rows,
        Some(Err(error)) => {
            state.error = Some(error.clone());
            return state;
        }
        None => {
            state.error = Some(BoardError {
                code: None,
                message: format!("{} is not a record type", binding.record_type),
            });
            return state;
        }
    };
    for row in rows
        .iter()
        .filter(|row| under(&row.logical, &binding.under))
    {
        let record = if row.status == "ok" { row.json() } else { None };
        let Some(record) = record else {
            state.skipped += 1;
            continue;
        };
        match matches(binding, &record) {
            Matched::No => {}
            Matched::Unreadable => state.skipped += 1,
            Matched::Yes { version } => {
                if state.records.len() == PLACED_LIMIT {
                    state.left_out += 1;
                    continue;
                }
                let title = record.get("title").and_then(Value::as_str).map_or_else(
                    || {
                        row.logical
                            .rsplit('/')
                            .next()
                            .unwrap_or_default()
                            .to_owned()
                    },
                    str::to_owned,
                );
                state.records.push(Placed {
                    current: version.as_deref().is_none_or(|version| version == subject),
                    path: row.logical.clone(),
                    title,
                    version,
                });
            }
        }
    }
    state
}

/// Whether a record matches a binding.
enum Matched {
    /// Every condition holds, and the record follows this version.
    Yes {
        /// The version, or `None` when the binding names no version field.
        version: Option<String>,
    },
    /// A condition does not hold.
    No,
    /// A field that the binding reads is not a string.
    Unreadable,
}

/// Returns whether `record` matches every condition of `binding`.
fn matches(binding: &Binding, record: &Value) -> Matched {
    for condition in &binding.conditions {
        let holds = match record.pointer(&condition.field) {
            None => false,
            Some(Value::String(text)) => match condition.op {
                Op::Equals => *text == condition.value,
                Op::Under => under(text, &condition.value),
            },
            Some(_) => return Matched::Unreadable,
        };
        if !holds {
            return Matched::No;
        }
    }
    let version = match binding
        .version
        .as_deref()
        .map(|field| record.pointer(field))
    {
        None | Some(None) => None,
        Some(Some(Value::String(text))) => Some(text.clone()),
        Some(Some(_)) => return Matched::Unreadable,
    };
    Matched::Yes { version }
}

/// Returns whether the board path `path` is `parent` or below it.
fn under(path: &str, parent: &str) -> bool {
    let parent = parent.trim_end_matches('/');
    path == parent
        || path
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Returns the first size limit of the plugin's schema that `graph` passes,
/// such as `at most 8 lanes`, or `None`.
fn limit_passed(graph: &WorkflowGraph) -> Option<String> {
    let graph = &graph.graph;
    [
        (graph.lanes.len(), MAX_LANES, "lanes"),
        (graph.groups.len(), MAX_GROUPS, "groups"),
        (graph.nodes.len(), MAX_NODES, "nodes"),
        (graph.edges.len(), MAX_EDGES, "edges"),
        (graph.bindings.len(), MAX_BINDINGS, "bindings"),
        (graph.sources.len(), MAX_SOURCES, "sources"),
    ]
    .into_iter()
    .find(|(count, most, _)| count > most)
    .map(|(_, most, what)| format!("at most {most} {what}"))
}

/// Why a query gave no document of the kind it asks for.
#[derive(Debug)]
enum Answer {
    /// Board did not start.
    Missing,
    /// Board printed a failure document, or flightdeck's words for a query
    /// that printed none.
    Failed(BoardError),
}

impl Answer {
    /// Returns Board's code, when it printed a failure document.
    fn code(&self) -> Option<&str> {
        match self {
            Self::Missing => None,
            Self::Failed(error) => error.code.as_deref(),
        }
    }

    /// Returns the answer as the failure of one part of a document.
    fn into_board_error(self) -> BoardError {
        match self {
            Self::Missing => BoardError {
                code: None,
                message: "Board did not start".to_owned(),
            },
            Self::Failed(error) => error,
        }
    }

    /// Returns the answer as the error of a query of `board` whose
    /// `command`, such as `query`, failed.
    fn into_error(self, board: &Board, command: &'static str) -> WorkflowsError {
        match self {
            Self::Missing => WorkflowsError::BoardMissing {
                program: board.shown(),
            },
            Self::Failed(error) => WorkflowsError::BoardFailed {
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
        Answer::Failed(BoardError {
            code: None,
            message,
        })
    };
    let finished = match result {
        Some(Ok(finished)) => finished,
        Some(Err(SpawnError::Start(_))) => return Err(Answer::Missing),
        Some(Err(SpawnError::Run(_))) => {
            return Err(failed(
                "flightdeck could not follow Board to its end".to_owned(),
            ));
        }
        None => return Err(failed("the query did not run".to_owned())),
    };
    let bytes = match (finished.exit(), finished.stdout()) {
        (Exit::TimedOut, _) => {
            return Err(failed(format!(
                "Board did not answer within {} seconds",
                spawn::BOARD_TIME_LIMIT.as_secs()
            )));
        }
        (_, Stdout::OverLimit) => {
            return Err(failed(format!(
                "Board printed more than the {} MiB that flightdeck reads",
                spawn::BOARD_STDOUT_LIMIT / (1024 * 1024)
            )));
        }
        (_, Stdout::Whole(bytes)) => bytes,
    };
    let head: Option<wire::Head> = serde_json::from_slice(bytes).ok();
    match head.as_ref().map(|head| head.schema.as_str()) {
        Some(found) if found == schema => serde_json::from_slice(bytes).map_err(|_| {
            failed(format!(
                "Board printed a {schema} document that flightdeck cannot read"
            ))
        }),
        Some(wire::FAILURE) => match serde_json::from_slice::<wire::Failure>(bytes) {
            Ok(failure) => Err(Answer::Failed(BoardError {
                code: Some(failure.code),
                message: failure.message,
            })),
            Err(_) => Err(failed(
                "Board printed a failure that flightdeck cannot read".to_owned(),
            )),
        },
        _ => Err(failed(match finished.exit() {
            Exit::Code(code) => {
                format!("Board exited with code {code} and printed no {schema} document")
            }
            Exit::Signal(signal) => {
                format!("a signal ({signal}) ended Board before it printed a document")
            }
            Exit::TimedOut | Exit::Unknown => format!("Board printed no {schema} document"),
        })),
    }
}
