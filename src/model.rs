//! The model of one crumbkit tool, built from what the tool prints about
//! itself.
//!
//! [`load`] runs four queries through [`crate::spawn`]: `--version`,
//! `commands list`, `schemas list`, and `schemas show` of every document
//! that `schemas list` names, at most [`MOST_IDS`] ids at a time. It joins
//! their documents into a [`Tool`]: its runnable commands, the goal headings
//! that group them, the documents they print with their JSON Schemas, and
//! the links that crumbkit's command grammar implies between them.

mod argument;
mod command;
mod documents;
mod error;
mod links;
mod wire;

#[cfg(test)]
mod tests;

use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::Value;

pub use self::argument::{Argument, Arity, CrumbkitFlag, Help, Kind};
pub use self::command::{
    Column, Command, CommandPath, Effects, Example, ExitMeaning, Form, Items, ResultSpec, Writes,
};
use self::documents::SchemasShow;
pub use self::documents::{
    CommandsList, Document, DocumentForm, DocumentKind, Listing, NextStep, Recovery, SchemasList,
    StepCommand, ToolFailure,
};
pub use self::error::{LoadError, NotDocument, Problem, TOOL_LOAD, TOOL_NOT_FOUND};
pub use self::links::{Link, Origin};
use self::wire::CommandRow;
use crate::spawn::{self, Exit, Program, Query, SpawnError, Stdout};

/// The most document ids that one `schemas show` takes.
pub const MOST_IDS: usize = 100;

/// The model of one crumbkit tool.
#[derive(Clone, Debug)]
pub struct Tool {
    name: String,
    version: String,
    about: Option<String>,
    headings: Vec<Heading>,
    commands: Vec<Command>,
    documents: Vec<Document>,
    links: Vec<Link>,
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

    /// Returns the links between the tool's commands.
    #[must_use]
    pub fn links(&self) -> &[Link] {
        &self.links
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
        let rows = list.into_rows();
        let root = rows
            .iter()
            .find(|row| !row.path.contains(' '))
            .ok_or(NotDocument::NoRoot)?;
        let name = root.path.clone();
        let about = root.about.clone();
        let goals = headings_of(root, &rows);
        let commands = commands_of(rows, &documents);
        let headings = goals.unwrap_or_else(|| {
            vec![Heading {
                title: name.clone(),
                commands: commands
                    .iter()
                    .map(|command| command.path().clone())
                    .collect(),
            }]
        });
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
        })
    }
}

/// A goal heading of the tool's help and the commands under it.
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Loads the model of the tool that `program` names, giving each query at
/// most `time_limit`.
///
/// # Errors
///
/// Returns [`LoadError::NotFound`] when the program does not exist or will
/// not start, and [`LoadError::Load`] when a query exits with a code other
/// than 0, passes its time limit, or prints something other than the
/// document it names.
pub fn load(program: &Program, time_limit: Duration) -> Result<Tool, LoadError> {
    let tool = program.name().to_string_lossy();
    let ask = |query: Query<'_>| Answer::of(program, &tool, &query, time_limit);

    let version = ask(Query::Version)?;
    let version_line = String::from_utf8_lossy(&version.stdout);
    let version_line = version_line.lines().next().unwrap_or_default();

    let commands_answer = ask(Query::CommandsList)?;
    let commands: CommandsList = commands_answer.parse()?;
    commands_answer.check(commands.schema(), "commands-list", commands.truncated())?;

    let schemas_answer = ask(Query::SchemasList)?;
    let schemas: SchemasList = schemas_answer.parse()?;
    schemas_answer.check(schemas.schema(), "schemas-list", schemas.truncated())?;

    let ids: Vec<String> = schemas.ids().map(str::to_owned).collect();
    let mut shown = Vec::new();
    for chunk in ids.chunks(MOST_IDS) {
        let answer = ask(Query::SchemasShow(chunk))?;
        let document: SchemasShow = answer.parse()?;
        answer.check(&document.schema, "schemas-show", false)?;
        if !document.failures.is_empty() {
            return Err(answer.fail(Problem::Failures(document.failures.len())));
        }
        shown.extend(document.data);
    }

    let documents = schemas
        .into_rows()
        .into_iter()
        .map(|row| {
            let schema = shown
                .iter()
                .position(|shown| shown.id == row.id)
                .map_or(Value::Null, |index| shown.swap_remove(index).schema);
            Document::new(row, schema)
        })
        .collect();
    Tool::assemble(version_line, commands, documents)
        .map_err(|problem| commands_answer.fail(Problem::NotDocument(problem)))
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
            Err(SpawnError::Start(source)) => {
                return Err(LoadError::NotFound {
                    tool: tool.to_owned(),
                    source,
                });
            }
            Err(SpawnError::Wait(error) | SpawnError::Read(error)) => {
                let problem = Problem::Read(error);
                return Err(LoadError::load(tool, command, problem, Exit::Signal, &[]));
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
            (Exit::Code(_) | Exit::Signal, _) => Err(fail(Problem::Exit)),
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
        if !schema.ends_with(&format!("/{name}/v1")) {
            let schema = NotDocument::Schema(schema.to_owned());
            return Err(self.fail(Problem::NotDocument(schema)));
        }
        if truncated {
            return Err(self.fail(Problem::NotDocument(NotDocument::Truncated)));
        }
        Ok(())
    }
}

/// Returns the headings of the root row's goals, with the runnable commands
/// under each, or `None` when the root row has no goals.
///
/// RUST-10: only the root's goals become headings. A goal heading of a
/// group below the root, which a newer crumbkit may print, is ignored, and a
/// command that no root goal names is under no heading. Nested headings
/// replace this when the page groups commands below the root.
fn headings_of(root: &CommandRow, rows: &[CommandRow]) -> Option<Vec<Heading>> {
    if root.goals.is_empty() {
        return None;
    }
    let root_path = CommandPath::from_row(&root.path);
    let leaves: Vec<CommandPath> = leaf_rows(rows)
        .map(|row| CommandPath::from_row(&row.path))
        .collect();
    let headings = root
        .goals
        .iter()
        .map(|goal| {
            let prefixes: Vec<CommandPath> = goal
                .commands
                .iter()
                .map(|words| root_path.join(words))
                .collect();
            Heading {
                title: goal.heading.clone(),
                commands: leaves
                    .iter()
                    .filter(|leaf| prefixes.iter().any(|prefix| leaf.starts_with(prefix)))
                    .cloned()
                    .collect(),
            }
        })
        .collect();
    Some(headings)
}

/// Returns the runnable commands of `rows`, each with the global arguments
/// of the rows above it and the id of the document it prints.
fn commands_of(rows: Vec<CommandRow>, documents: &[Document]) -> Vec<Command> {
    let leaves: Vec<bool> = rows.iter().map(|row| is_leaf(row, &rows)).collect();
    let mut groups = Vec::new();
    let mut commands = Vec::new();
    for (row, leaf) in rows.into_iter().zip(leaves) {
        let command = Command::from_row(row);
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
    commands
}

/// Returns the rows that no other row is below.
fn leaf_rows(rows: &[CommandRow]) -> impl Iterator<Item = &CommandRow> {
    rows.iter().filter(|row| is_leaf(row, rows))
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
