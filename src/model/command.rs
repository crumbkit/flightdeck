//! One runnable command of a tool and the parts it is made of.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::argument::{Argument, CrumbkitFlag};
use super::error::NotDocument;
use super::figures::{Cost, counts, is_built_in};
use super::wire::CommandRow;
use crate::spawn::OwnRuns;

/// The words of a command below the tool's name, such as `books list`. It
/// serializes as the array of its words.
///
/// No word is empty or starts with `-`. A crumbkit tool reads such a word as
/// an option, so no command could have it, and a run, a preview, a choices
/// run or a help run that put it on a command line would pass the tool an
/// option. A `--` before the words would not help, because a crumbkit tool
/// reads every word after `--` as a value, not a command.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct CommandPath {
    words: Vec<String>,
}

impl CommandPath {
    /// Returns the path of a `commands list` row, whose first word is the
    /// tool's name.
    ///
    /// # Errors
    ///
    /// Returns [`NotDocument::OptionWord`] when a word below the tool's name
    /// is empty or starts with `-`.
    pub(super) fn from_row(path: &str) -> Result<Self, NotDocument> {
        Self::of(path.split(' ').skip(1)).map_err(|()| NotDocument::OptionWord(path.to_owned()))
    }

    /// Returns the path of `words`, or `Err` when one is empty or starts
    /// with `-`.
    fn of<'a>(words: impl IntoIterator<Item = &'a str>) -> Result<Self, ()> {
        let words = words
            .into_iter()
            .map(|word| {
                if word.is_empty() || word.starts_with('-') {
                    Err(())
                } else {
                    Ok(word.to_owned())
                }
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { words })
    }

    /// Returns the path's words, such as `["books", "list"]`.
    #[must_use]
    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// Returns the words of the resource that the command acts on: every
    /// word except the verb, such as `["books"]`.
    #[must_use]
    pub fn resource(&self) -> &[String] {
        self.words
            .split_last()
            .map_or(&[], |(_, resource)| resource)
    }

    /// Returns true when this path is `other` or a command below it.
    pub(super) fn starts_with(&self, other: &Self) -> bool {
        self.words.starts_with(&other.words)
    }

    /// Returns true when the path is these words.
    pub(super) fn is(&self, words: &[&str]) -> bool {
        self.words
            .iter()
            .map(String::as_str)
            .eq(words.iter().copied())
    }

    /// Returns this path with `words`, split at spaces, appended, or `None`
    /// when a word of `words` is empty or starts with `-`.
    pub(super) fn join(&self, words: &str) -> Option<Self> {
        Self::of(
            self.words
                .iter()
                .map(String::as_str)
                .chain(words.split(' ')),
        )
        .ok()
    }
}

/// Prints the words joined by spaces, such as `books list`.
impl fmt::Display for CommandPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.words.join(" "))
    }
}

/// One runnable command of a tool.
#[derive(Clone, Debug, Serialize)]
pub struct Command {
    path: CommandPath,
    about: Option<String>,
    long_about: Option<String>,
    arguments: Vec<Argument>,
    examples: Vec<Example>,
    result: ResultSpec,
    exits: Vec<ExitMeaning>,
    effects: Effects,
    class: Class,
    writes: Option<Writes>,
    #[serde(flatten)]
    cost: Costed,
    /// Whether the figures count the command's runs. Every command is
    /// counted when the tool leaves out flightdeck's own runs. On a tool
    /// that cannot, the queries that the loader runs as the person, such as
    /// `self usage`, are not counted, because their runs are left out of the
    /// figures by name (RUST-10 in `figures.rs`), so the page says so rather
    /// than showing that agents never ran them.
    counted: bool,
    /// Whether crumbkit adds the command to every tool, such as
    /// `self usage`. It is the row's `built_in` when the tool marks
    /// crumbkit's own commands, and otherwise comes from the rule and its
    /// RUST-10 in `figures.rs`. The both table leaves these commands out of
    /// its corners.
    built_in: bool,
    /// Whether `commands list` marks the command as one that prints no
    /// result, so its runs report no output size, which the page reads as
    /// no output rather than an unknown one.
    prints_no_result: bool,
}

impl Command {
    /// Returns the command of a `commands list` row, with only the row's own
    /// arguments. Its built-in mark is the row's `built_in` when
    /// `marks_built_in`, because the tool's crumbkit marks its own commands,
    /// and otherwise comes from the command's first word. It is counted
    /// unless `own_runs` says the tool's reports keep flightdeck's own runs
    /// and the command is one of the loader's queries of the usage log.
    ///
    /// # Errors
    ///
    /// Returns [`NotDocument::OptionWord`] when a word of the row's path is
    /// empty or starts with `-`.
    pub(super) fn from_row(
        row: CommandRow,
        marks_built_in: bool,
        own_runs: OwnRuns,
    ) -> Result<Self, NotDocument> {
        let path = CommandPath::from_row(&row.path)?;
        Ok(Self {
            counted: counts(&path.to_string(), own_runs),
            built_in: if marks_built_in {
                row.built_in
            } else {
                is_built_in(&path)
            },
            path,
            about: row.about,
            long_about: row.long_about,
            arguments: row.arguments.into_iter().map(Argument::from_row).collect(),
            examples: row.examples,
            result: ResultSpec {
                document: None,
                form: row.form,
                columns: row.columns,
            },
            exits: row.exits,
            class: Class::of(&row.effects),
            effects: row.effects,
            writes: row.writes,
            cost: Costed(None),
            prints_no_result: row.prints_no_result,
        })
    }

    /// Adds the global arguments of a group above the command, after the
    /// command's own, and skips any the command already has.
    pub(super) fn inherit<'a>(&mut self, globals: impl IntoIterator<Item = &'a Argument>) {
        for global in globals {
            if !self.arguments.iter().any(|own| own.name() == global.name()) {
                self.arguments.push(global.clone());
            }
        }
    }

    /// Records what the command costs the agents that call it, and how
    /// often it ran, from the tool's figures.
    pub(super) fn set_cost(&mut self, cost: Cost) {
        self.cost = Costed(Some(cost));
    }

    /// Records the id of the document that the command prints.
    pub(super) fn set_document(&mut self, id: String) {
        self.result.document = Some(id);
    }

    /// Returns the command's path below the tool's name.
    #[must_use]
    pub const fn path(&self) -> &CommandPath {
        &self.path
    }

    /// Returns the command's one-line about text.
    #[must_use]
    pub fn about(&self) -> Option<&str> {
        self.about.as_deref()
    }

    /// Returns the command's long about text, which `--help` prints.
    #[must_use]
    pub fn long_about(&self) -> Option<&str> {
        self.long_about.as_deref()
    }

    /// Returns every argument the command takes: its own, then the global
    /// ones of the groups above it.
    #[must_use]
    pub fn arguments(&self) -> &[Argument] {
        &self.arguments
    }

    /// Returns the arguments that belong to the tool, which a form shows as
    /// fields.
    pub fn tool_arguments(&self) -> impl Iterator<Item = &Argument> {
        self.arguments
            .iter()
            .filter(|argument| argument.crumbkit_flag().is_none())
    }

    /// Returns crumbkit's own flags that the command takes, each with its
    /// argument, which flightdeck sets itself.
    pub fn crumbkit_flags(&self) -> impl Iterator<Item = (CrumbkitFlag, &Argument)> {
        self.arguments
            .iter()
            .filter_map(|argument| argument.crumbkit_flag().map(|flag| (flag, argument)))
    }

    /// Returns the command's examples.
    #[must_use]
    pub fn examples(&self) -> &[Example] {
        &self.examples
    }

    /// Returns what the command prints.
    #[must_use]
    pub const fn result(&self) -> &ResultSpec {
        &self.result
    }

    /// Returns the exit codes that the tool declares for the command beyond
    /// crumbkit's own 0, 1 and 2.
    #[must_use]
    pub fn exits(&self) -> &[ExitMeaning] {
        &self.exits
    }

    /// Returns what the command changes.
    #[must_use]
    pub const fn effects(&self) -> &Effects {
        &self.effects
    }

    /// Returns how the command's effects decide how it starts.
    #[must_use]
    pub const fn class(&self) -> Class {
        self.class
    }

    /// Returns how the command guards its writes, or `None` for a command
    /// without guards.
    #[must_use]
    pub const fn writes(&self) -> Option<&Writes> {
        self.writes.as_ref()
    }

    /// Returns true when the tool's `commands list` marks the command as one
    /// that prints no result, and so takes none of `--fields`, `--long` and
    /// `--full`. A tool that [`Tool::marks_no_result`](super::Tool::marks_no_result)
    /// says cannot mark one leaves this false for every command.
    #[must_use]
    pub const fn prints_no_result(&self) -> bool {
        self.prints_no_result
    }

    /// Returns how often the command ran, from the tool's `self figures
    /// list`, or 0 when the tool keeps no figures or the call failed. The
    /// commands that the loader runs as the person, such as `self usage`,
    /// are always 0.
    #[must_use]
    pub const fn runs(&self) -> u64 {
        match &self.cost.0 {
            Some(cost) => cost.figures().runs(),
            None => 0,
        }
    }

    /// Returns what the command costs the agents that call it, from the
    /// tool's `self figures list` and `self usage`, or `None` when the
    /// figures hold no row for it. The commands that the loader runs as
    /// the person have none.
    #[must_use]
    pub const fn cost(&self) -> Option<&Cost> {
        self.cost.0.as_ref()
    }
}

/// A command's cost, which serializes as two fields of the command: `runs`,
/// which the cost's figures decide and the palette reads, and `cost`. The
/// runs are never kept apart from the cost, so the two cannot disagree.
#[derive(Clone, Debug)]
struct Costed(Option<Cost>);

impl Serialize for Costed {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap as _;

        let runs = self.0.as_ref().map_or(0, |cost| cost.figures().runs());
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("runs", &runs)?;
        map.serialize_entry("cost", &self.0)?;
        map.end()
    }
}

/// What a command prints.
#[derive(Clone, Debug, Serialize)]
pub struct ResultSpec {
    document: Option<String>,
    form: Option<Form>,
    columns: Vec<Column>,
}

impl ResultSpec {
    /// Returns the id of the document the command prints, such as
    /// `shelf/books-list/v1`, from the tool's `schemas list`.
    #[must_use]
    pub fn document(&self) -> Option<&str> {
        self.document.as_deref()
    }

    /// Returns the form of the output, or `None` for a command without a
    /// document.
    #[must_use]
    pub const fn form(&self) -> Option<Form> {
        self.form
    }

    /// Returns the columns of the rows the command prints.
    #[must_use]
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }
}

/// The form of a command's output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Form {
    /// One JSON document.
    Document,
    /// One JSON document on each line.
    Stream,
    /// Bytes, such as a file's contents.
    Bytes,
    /// A header line followed by bytes.
    Headed,
}

/// One example of a command, as a whole command line.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Example {
    command: String,
    about: String,
}

impl Example {
    /// Returns the command line, such as `shelf books show 2 5`.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Returns what the example does.
    #[must_use]
    pub fn about(&self) -> &str {
        &self.about
    }
}

/// One column of the rows a command prints.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Column {
    name: String,
    long: bool,
    #[serde(default)]
    description: Option<String>,
}

impl Column {
    /// Returns the column's JSON field name, such as `id`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns true when only `--long` prints the column.
    #[must_use]
    pub const fn long(&self) -> bool {
        self.long
    }

    /// Returns the column's description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// An exit code, or a range of them, that a tool declares for a command.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExitMeaning {
    exit: u8,
    last: u8,
    meaning: String,
}

impl ExitMeaning {
    /// Returns the first code of the range.
    #[must_use]
    pub const fn exit(&self) -> u8 {
        self.exit
    }

    /// Returns the last code of the range, which equals the first for one
    /// code.
    #[must_use]
    pub const fn last(&self) -> u8 {
        self.last
    }

    /// Returns what the codes mean.
    #[must_use]
    pub fn meaning(&self) -> &str {
        &self.meaning
    }
}

/// How a command's effects decide how it starts. It serializes as
/// `read_only`, `additive` or `destructive`, and the page reads it from the
/// model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    /// The command changes nothing.
    ReadOnly,
    /// The command adds without destroying.
    Additive,
    /// The command may destroy data, or declares no effects.
    Destructive,
}

impl Class {
    /// Returns the class that `effects` decide. A command that declares
    /// itself destructive is destructive, even when it also declares itself
    /// read-only, and a command that declares nothing is destructive.
    const fn of(effects: &Effects) -> Self {
        match effects {
            Effects::Known {
                destructive: true, ..
            }
            | Effects::Unknown => Self::Destructive,
            Effects::Known {
                read_only: true, ..
            } => Self::ReadOnly,
            Effects::Known { .. } => Self::Additive,
        }
    }
}

/// What a command changes. `commands list` prints `null` for a command that
/// declares nothing, and a caller treats such a command as one that may
/// destroy data. It serializes in the same form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(from = "Option<KnownEffects>", into = "Option<KnownEffects>")]
pub enum Effects {
    /// The command declares its effects with MCP's four hints.
    Known {
        /// The command changes nothing.
        read_only: bool,
        /// The command may delete or overwrite data.
        destructive: bool,
        /// Running the command twice changes no more than running it once.
        idempotent: bool,
        /// The command reaches things outside the tool, such as a network.
        open_world: bool,
    },
    /// The command declares no effects.
    #[default]
    Unknown,
}

/// The four hints of a command that declares its effects.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct KnownEffects {
    read_only: bool,
    destructive: bool,
    idempotent: bool,
    open_world: bool,
}

impl From<Option<KnownEffects>> for Effects {
    fn from(effects: Option<KnownEffects>) -> Self {
        match effects {
            Some(KnownEffects {
                read_only,
                destructive,
                idempotent,
                open_world,
            }) => Self::Known {
                read_only,
                destructive,
                idempotent,
                open_world,
            },
            None => Self::Unknown,
        }
    }
}

impl From<Effects> for Option<KnownEffects> {
    fn from(effects: Effects) -> Self {
        match effects {
            Effects::Known {
                read_only,
                destructive,
                idempotent,
                open_world,
            } => Some(KnownEffects {
                read_only,
                destructive,
                idempotent,
                open_world,
            }),
            Effects::Unknown => None,
        }
    }
}

/// How a command guards its writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Writes {
    replaces: bool,
    expect: bool,
    items: Option<Items>,
}

impl Writes {
    /// Returns true when the command replaces a value that already exists.
    #[must_use]
    pub const fn replaces(&self) -> bool {
        self.replaces
    }

    /// Returns true when the command takes `--expect` with the value it
    /// expects to replace.
    #[must_use]
    pub const fn expect(&self) -> bool {
        self.expect
    }

    /// Returns how a write of several items ends when one fails, or `None`
    /// for a command that writes one item.
    #[must_use]
    pub const fn items(&self) -> Option<Items> {
        self.items
    }
}

/// How a write of several items ends when one of them fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Items {
    /// The command writes every item or none.
    AllOrNone,
    /// The command writes each item that it can.
    PerItem,
}
