//! The model of one crumbkit tool, built from what the tool prints about
//! itself.
//!
//! [`load`] runs its queries through [`crate::spawn`], all within one time
//! limit: `--version`, `commands list`, `schemas list`, `schemas show` of
//! every document that `schemas list` names, at most 100 ids at a time,
//! and `self figures list`, `self usage` and `self runs list` when the tool
//! mounts them. The queries of the description log their runs to a fresh
//! state directory, so they stay out of the person's usage log. It
//! joins their documents into a [`Tool`]: its runnable commands with what
//! each costs the agents that call it, the goal headings that group them, the documents they print with
//! their JSON Schemas, and the links that crumbkit's command grammar
//! implies between them.

mod argument;
mod command;
mod day;
mod documents;
mod error;
mod figures;
mod links;
mod runs;
pub mod weight;
mod wire;

#[cfg(test)]
mod tests;

use std::io;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde::de::DeserializeOwned;
use unicode_general_category::{GeneralCategory, get_general_category};

pub use self::argument::{Argument, Arity, CrumbkitFlag, Help, Kind};
pub use self::command::{
    Class, Column, Command, CommandPath, Effects, Example, ExitMeaning, Form, Items, ResultSpec,
    Writes,
};
pub use self::day::Day;
pub use self::documents::{
    CommandsList, Document, DocumentForm, DocumentKind, Listing, NextStep, Recovery, SchemasList,
    SchemasShow, StepCommand, ToolFailure,
};
pub use self::error::{LoadError, NotDocument, Problem, TOOL_LOAD, TOOL_NOT_FOUND};
pub(crate) use self::error::{exit_code, last_lines};
pub use self::figures::{Cost, Figures, Finding, FindingKind};
use self::figures::{FiguresList, Usage};
pub(crate) use self::links::ID_COLUMN;
pub use self::links::Link;
pub use self::runs::{History, Run};
use self::wire::{CommandRow, GoalRow};
use crate::spawn::{self, Exit, FreshState, Program, Query, SpawnError, Stdout};

/// The most document ids that one `schemas show` takes.
pub(crate) const MOST_IDS: usize = 100;

/// The model of one crumbkit tool, which serializes as the page reads it.
#[derive(Clone, Debug, Serialize)]
pub struct Tool {
    name: String,
    version: String,
    about: Option<String>,
    headings: Vec<Heading>,
    commands: Vec<Command>,
    documents: Vec<Document>,
    links: Vec<Link>,
    /// The day the tool's figures start, such as 3 November 2026, or `None`
    /// when no command has a cost.
    figures_since: Option<Day>,
    /// Each run in the tool's usage log, in order within its session, or
    /// `None` when the tool does not mount `self runs list` or it failed.
    history: Option<Vec<Run>>,
    #[serde(skip)]
    marks_no_result: bool,
}

impl Tool {
    /// Returns the tool's name, such as `shelf`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the tool's version, such as `0.1.0`.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the tool's about line.
    #[must_use]
    pub fn about(&self) -> Option<&str> {
        self.about.as_deref()
    }

    /// Returns the headings that group the tool's commands, in help order.
    #[must_use]
    pub fn headings(&self) -> &[Heading] {
        &self.headings
    }

    /// Returns the tool's runnable commands, in the order `commands list`
    /// prints them.
    #[must_use]
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    /// Returns the runnable command with the path `words`, such as
    /// `["books", "list"]`.
    #[must_use]
    pub fn command(&self, words: &[&str]) -> Option<&Command> {
        self.commands
            .iter()
            .find(|command| command.path().is(words))
    }

    /// Returns the documents that the tool prints, each with its schema.
    #[must_use]
    pub fn documents(&self) -> &[Document] {
        &self.documents
    }

    /// Returns the document with the id `id`.
    #[must_use]
    pub fn document(&self, id: &str) -> Option<&Document> {
        self.documents.iter().find(|document| document.id() == id)
    }

    /// Returns true when the tool's `commands list` marks each command that
    /// prints no result with `prints_no_result`, as crumbkit does since
    /// crumbkit/crumbkit#291. The tool's own `<tool>/commands-list/v1`
    /// schema then declares the field. A tool on an earlier crumbkit, such
    /// as 0.1.1, gives such a command the same row as a command without a
    /// document, and [`Command::prints_no_result`] is false for both.
    #[must_use]
    pub const fn marks_no_result(&self) -> bool {
        self.marks_no_result
    }

    /// Returns the links between the tool's commands.
    #[must_use]
    pub fn links(&self) -> &[Link] {
        &self.links
    }

    /// Returns the day the tool's figures start, such as 3 November 2026, or
    /// `None` when no command has a cost or the tool printed no such day.
    #[must_use]
    pub const fn figures_since(&self) -> Option<Day> {
        self.figures_since
    }

    /// Returns each run in the tool's usage log, in order within its
    /// session, or `None` when the tool reported none.
    #[must_use]
    pub fn history(&self) -> Option<&[Run]> {
        self.history.as_deref()
    }

    /// Records the runs of `history`, or none.
    fn set_history(&mut self, history: Option<History>) {
        self.history = history.map(|history| history.runs().to_vec());
    }

    /// Gives each command the cost that `figures` and `usage` hold for its
    /// path, such as `books list`.
    /// The day the figures start is kept only when a command has a cost,
    /// because only a cost card shows it.
    fn add_costs(&mut self, figures: &FiguresList, usage: Option<&Usage>) {
        let mut costed = false;
        for command in &mut self.commands {
            if let Some(cost) = figures.cost(command.path(), usage) {
                command.set_cost(cost);
                costed = true;
            }
        }
        if costed {
            self.figures_since = figures.since();
        }
    }

    /// Joins the tool's documents into its model: the first line of
    /// `--version`, `commands list`, and the documents that `schemas list`
    /// and `schemas show` describe.
    ///
    /// The runnable commands are the rows that no other row is below. Each
    /// takes the global arguments of the rows above it, and names the
    /// document that lists it among the commands that print it.
    pub(crate) fn assemble(
        version_line: &str,
        list: CommandsList,
        documents: Vec<Document>,
    ) -> Result<Self, NotDocument> {
        // The tool's own schema of the document that `commands list` printed
        // declares `prints_no_result` when its crumbkit marks such commands.
        let marks_no_result = documents.iter().any(|document| {
            document.id() == list.schema() && document.declares_property("prints_no_result")
        });
        let rows = list.into_rows();
        let root = rows
            .iter()
            .find(|row| !row.path.contains(' '))
            .ok_or(NotDocument::NoRoot)?;
        let name = root.path.clone();
        let about = root.about.clone();
        let goals = root.goals.clone();
        let commands = commands_of(rows, &documents)?;
        let headings = headings_of(&goals, &commands).unwrap_or_else(|| {
            vec![Heading {
                title: name.clone(),
                commands: commands
                    .iter()
                    .map(|command| command.path().clone())
                    .collect(),
            }]
        });
        if has_control(&name) {
            return Err(NotDocument::Control(name));
        }
        if let Some(text) = commands.iter().find_map(control_name) {
            return Err(NotDocument::Control(text));
        }
        let version = version_line
            .strip_prefix(&name)
            .unwrap_or(version_line)
            .trim()
            .to_owned();
        let links = links::infer(&commands);
        Ok(Self {
            name,
            version,
            about,
            headings,
            commands,
            documents,
            links,
            figures_since: None,
            history: None,
            marks_no_result,
        })
    }
}

/// Returns `command`'s path when a word of it holds a character that
/// [`has_control`] finds, or else the name of the first of its arguments
/// that holds one, or `None`.
fn control_name(command: &Command) -> Option<String> {
    if command.path().words().iter().any(|word| has_control(word)) {
        return Some(command.path().to_string());
    }
    command
        .arguments()
        .iter()
        .map(Argument::name)
        .find(|name| has_control(name))
        .map(str::to_owned)
}

/// Returns true when `text` holds a control character, a format character
/// (Unicode's category Cf, such as a bidirectional control, a zero-width
/// space or a soft hyphen), or a line or paragraph separator, which a
/// display line would show as something else or not at all.
///
/// RUST-10: flightdeck refuses such a name or value, which is plainer than
/// quoting it. It handles every value that a person types on one line, and
/// it misses a value that holds a newline or a tab, such as a note of
/// several lines. Quoting each such character as `$'\n'` or `$'\u{202e}'`
/// replaces the refusal when a tool takes such a value.
pub(crate) fn has_control(text: &str) -> bool {
    text.chars().any(|c| {
        c.is_control()
            || get_general_category(c) == GeneralCategory::Format
            || matches!(c, '\u{2028}' | '\u{2029}')
    })
}

/// A goal heading of the tool's help and the commands under it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Heading {
    title: String,
    commands: Vec<CommandPath>,
}

impl Heading {
    /// Returns the heading, such as `Browse the catalogue`.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the runnable commands under the heading, in the order
    /// `commands list` prints them.
    #[must_use]
    pub fn commands(&self) -> &[CommandPath] {
        &self.commands
    }
}

/// Loads the model of the tool that `program` names, giving all of its
/// queries together at most `time_limit`.
///
/// # Errors
///
/// Returns [`LoadError::State`] when the temporary state directory of the
/// description's queries cannot be made, before any query runs,
/// [`LoadError::NotFound`] when the program does not exist or may
/// not be run, and [`LoadError::Load`] when a query cannot be started,
/// exits with a code other than 0, passes the time limit, or prints
/// something other than the document it names. A `schemas show` that leaves
/// out an id that `schemas list` named is a document of the wrong shape.
/// `self figures list`, `self usage` and `self runs list` fail no load:
/// when the figures fail, every command's runs stay 0 and it has no cost,
/// when the findings fail, each cost has no findings, and when the runs
/// fail, the tool has no history.
pub fn load(program: &Program, time_limit: Duration) -> Result<Tool, LoadError> {
    let tool = program.name().to_string_lossy();
    let deadline = Instant::now() + time_limit;
    let ask_as = |program: &Program, query: Query<'_>| {
        let remaining = deadline.saturating_duration_since(Instant::now());
        Answer::of(program, &tool, &query, remaining)
    };
    // The queries of the tool's description log their runs to a fresh state
    // directory, which is removed when `load` returns. The figures, the
    // findings and the runs read the person's usage log, so they run as
    // `program`.
    let state = FreshState::new().map_err(|source| LoadError::State {
        tool: tool.clone().into_owned(),
        source,
    })?;
    let describer = state.keep_out(program);
    let describe = |query: Query<'_>| ask_as(&describer, query);
    let ask = |query: Query<'_>| ask_as(program, query);

    let version = describe(Query::Version)?;
    let version_line = String::from_utf8_lossy(&version.stdout);
    let version_line = version_line.lines().next().unwrap_or_default();

    let commands_answer = describe(Query::CommandsList)?;
    let commands: CommandsList = commands_answer.parse()?;
    commands_answer.check(commands.schema(), "commands-list", commands.truncated())?;

    let schemas_answer = describe(Query::SchemasList)?;
    let schemas: SchemasList = schemas_answer.parse()?;
    schemas_answer.check(schemas.schema(), "schemas-list", schemas.truncated())?;

    // `schemas show` exits with 1 when it cannot show an id, and that exit
    // fails the query, so a document read here showed every id it printed.
    let ids: Vec<String> = schemas.ids().map(str::to_owned).collect();
    let mut in_order = Vec::with_capacity(ids.len());
    for chunk in ids.chunks(MOST_IDS) {
        let answer = describe(Query::SchemasShow(chunk))?;
        let document: SchemasShow = answer.parse()?;
        answer.check(document.schema(), "schemas-show", false)?;
        let mut shown = document.into_data();
        for id in chunk {
            let index = shown
                .iter()
                .position(|shown| &shown.id == id)
                .ok_or_else(|| {
                    answer.fail(Problem::NotDocument(NotDocument::MissingSchema(id.clone())))
                })?;
            in_order.push(shown.swap_remove(index).schema);
        }
    }

    let documents = schemas
        .into_rows()
        .into_iter()
        .zip(in_order)
        .map(|(row, schema)| Document::new(row, schema))
        .collect();
    let mut tool = Tool::assemble(version_line, commands, documents)
        .map_err(|problem| commands_answer.fail(Problem::NotDocument(problem)))?;

    // The figures only order the page's palette and fill its cost cards, so
    // a tool that does not mount them is never asked, and a failed call
    // leaves every count at 0. The findings and the runs only add to the
    // figures, so they are asked for only beside them, and all three run
    // at once.
    if tool.command(&figures::FIGURES_COMMAND).is_some() {
        let asks_usage = tool.command(&figures::USAGE_COMMAND).is_some();
        let asks_runs = tool.command(&runs::RUNS_COMMAND).is_some();
        let (figures, usage, history) = std::thread::scope(|scope| {
            let usage = asks_usage.then(|| scope.spawn(|| ask(Query::Usage)));
            let history = asks_runs.then(|| scope.spawn(|| ask(Query::RunsList)));
            let figures = ask(Query::FiguresList);
            let joined = |handle: std::thread::ScopedJoinHandle<'_, _>| {
                handle
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            };
            (figures, usage.map(joined), history.map(joined))
        });
        tool.set_history(
            history
                .and_then(Result::ok)
                .and_then(|answer| History::read(&answer.stdout, &tool.name)),
        );
        let usage = usage
            .and_then(Result::ok)
            .and_then(|answer| Usage::read(&answer.stdout, &tool.name));
        if let Some(figures) = figures
            .ok()
            .and_then(|answer| FiguresList::read(&answer.stdout))
        {
            tool.add_costs(&figures, usage.as_ref());
        }
    }
    Ok(tool)
}

/// The standard output of one query that exited with 0, and what a failure
/// of the query names.
struct Answer<'a> {
    tool: &'a str,
    command: String,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl<'a> Answer<'a> {
    /// Runs `query` against `program`, which the person named `tool`, and
    /// returns its output, or the failure of a query that did not exit
    /// with 0.
    fn of(
        program: &Program,
        tool: &'a str,
        query: &Query<'_>,
        time_limit: Duration,
    ) -> Result<Self, LoadError> {
        let command = query.words().join(" ");
        let finished = match spawn::describe(program, query, time_limit) {
            Ok(finished) => finished,
            Err(SpawnError::Start(source))
                if matches!(
                    source.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::PermissionDenied
                ) =>
            {
                return Err(LoadError::NotFound {
                    tool: tool.to_owned(),
                    source,
                });
            }
            Err(SpawnError::Start(error)) => {
                let problem = Problem::Start(error);
                return Err(LoadError::load(tool, command, problem, Exit::Unknown, &[]));
            }
            Err(SpawnError::Run(error)) => {
                let problem = Problem::Read(error);
                return Err(LoadError::load(tool, command, problem, Exit::Unknown, &[]));
            }
        };
        let exit = finished.exit();
        let fail =
            |problem| LoadError::load(tool, command.clone(), problem, exit, finished.stderr());
        match (exit, finished.stdout()) {
            (Exit::Code(0), Stdout::Whole(stdout)) => Ok(Self {
                tool,
                command,
                stdout: stdout.clone(),
                stderr: finished.stderr().to_vec(),
            }),
            (Exit::Code(0), Stdout::OverLimit) => Err(fail(Problem::StdoutLimit)),
            (Exit::TimedOut, _) => Err(fail(Problem::TimeLimit)),
            (Exit::Code(_) | Exit::Signal(_) | Exit::Unknown, _) => Err(fail(Problem::Exit)),
        }
    }

    /// Returns the failure of this query with `problem`.
    fn fail(&self, problem: Problem) -> LoadError {
        LoadError::load(
            self.tool,
            self.command.clone(),
            problem,
            Exit::Code(0),
            &self.stderr,
        )
    }

    /// Parses the output as the document `T`.
    fn parse<T: DeserializeOwned>(&self) -> Result<T, LoadError> {
        serde_json::from_slice(&self.stdout)
            .map_err(|error| self.fail(Problem::NotDocument(NotDocument::Json(error))))
    }

    /// Refuses a document whose id `schema` does not end with
    /// `/<name>/v1`, or a listing that leaves out rows.
    fn check(&self, schema: &str, name: &str, truncated: bool) -> Result<(), LoadError> {
        if !documents::is_document(schema, name) {
            let schema = NotDocument::Schema(schema.to_owned());
            return Err(self.fail(Problem::NotDocument(schema)));
        }
        if truncated {
            return Err(self.fail(Problem::NotDocument(NotDocument::Truncated)));
        }
        Ok(())
    }
}

/// Returns the headings of the root row's `goals`, with the runnable
/// `commands` under each, or `None` when the root row has no goals. A goal's
/// command that does not parse as a path matches no command, so it is
/// skipped.
///
/// RUST-10: only the root's goals become headings. A goal heading of a
/// group below the root, which a newer crumbkit may print, is ignored, and a
/// command that no root goal names is under no heading. Nested headings
/// replace this when the page groups commands below the root.
fn headings_of(goals: &[GoalRow], commands: &[Command]) -> Option<Vec<Heading>> {
    if goals.is_empty() {
        return None;
    }
    let root = CommandPath::default();
    let headings = goals
        .iter()
        .map(|goal| {
            let prefixes: Vec<CommandPath> = goal
                .commands
                .iter()
                .filter_map(|words| root.join(words))
                .collect();
            Heading {
                title: goal.heading.clone(),
                commands: commands
                    .iter()
                    .map(Command::path)
                    .filter(|path| prefixes.iter().any(|prefix| path.starts_with(prefix)))
                    .cloned()
                    .collect(),
            }
        })
        .collect();
    Some(headings)
}

/// Returns the runnable commands of `rows`, each with the global arguments
/// of the rows above it and the id of the document it prints.
///
/// RUST-10: a command takes only the global arguments of its groups. A
/// group's required argument that is not global, which the person gives
/// before the command's own words, is not carried to its commands, so the
/// page cannot ask for it. crumbkit 0.1.0's tools have no such argument.
/// Carrying each group's arguments with their position replaces this when a
/// tool has one.
///
/// # Errors
///
/// Returns [`NotDocument::OptionWord`] when a row's path has a word that is
/// empty or starts with `-`.
fn commands_of(rows: Vec<CommandRow>, documents: &[Document]) -> Result<Vec<Command>, NotDocument> {
    let leaves: Vec<bool> = rows.iter().map(|row| is_leaf(row, &rows)).collect();
    let mut groups = Vec::new();
    let mut commands = Vec::new();
    for (row, leaf) in rows.into_iter().zip(leaves) {
        let command = Command::from_row(row)?;
        if leaf {
            commands.push(command);
        } else {
            groups.push(command);
        }
    }
    for command in &mut commands {
        for group in &groups {
            if command.path().starts_with(group.path()) {
                command.inherit(
                    group
                        .arguments()
                        .iter()
                        .filter(|argument| argument.global()),
                );
            }
        }
        if let Some(document) = documents
            .iter()
            .find(|document| document.commands().contains(command.path()))
        {
            command.set_document(document.id().to_owned());
        }
    }
    Ok(commands)
}

/// Returns true when no row of `rows` is below `row`.
///
/// RUST-10: crumbkit's grammar gives each runnable command no subcommands,
/// so a row with rows below it is a group. A command that both runs and has
/// subcommands counts as a group and is left out of the runnable commands.
/// A field on the row that marks a runnable command replaces this when
/// crumbkit prints one.
fn is_leaf(row: &CommandRow, rows: &[CommandRow]) -> bool {
    let prefix = format!("{} ", row.path);
    !rows.iter().any(|other| other.path.starts_with(&prefix))
}
