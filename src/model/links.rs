//! Links from a cell of one command's rows to another command that takes
//! the cell's value.

use super::argument::{Argument, Kind};
use super::command::{Command, CommandPath};

/// The column whose cells an inferred link starts from.
const ID_COLUMN: &str = "id";

/// The value name of the positional argument that an inferred link fills.
const ID_VALUE: &str = "ID";

/// A link from a column of one command's rows to an argument of another
/// command, which takes the column's value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    source: CommandPath,
    column: String,
    target: CommandPath,
    argument: String,
    origin: Origin,
}

impl Link {
    /// Returns the command whose rows hold the column.
    #[must_use]
    pub const fn source(&self) -> &CommandPath {
        &self.source
    }

    /// Returns the column whose value the link carries, such as `id`.
    #[must_use]
    pub fn column(&self) -> &str {
        &self.column
    }

    /// Returns the command that the link opens.
    #[must_use]
    pub const fn target(&self) -> &CommandPath {
        &self.target
    }

    /// Returns the argument of the target that takes the value, such as
    /// `<ID>`.
    #[must_use]
    pub fn argument(&self) -> &str {
        &self.argument
    }

    /// Returns where the link comes from.
    #[must_use]
    pub const fn origin(&self) -> Origin {
        self.origin
    }
}

/// Where a link comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// flightdeck inferred the link from crumbkit's command grammar; the tool
    /// did not declare it.
    Inferred,
}

/// Returns the links that crumbkit's grammar implies between `commands`, in
/// the order of the commands.
///
/// A command whose rows have a column named `id` links it to each other
/// command of the same resource, the same path except the verb, that takes a
/// positional argument whose value name is `ID`.
pub(super) fn infer(commands: &[Command]) -> Vec<Link> {
    let mut links = Vec::new();
    for source in commands.iter().filter(|command| has_id_column(command)) {
        for target in commands {
            if target.path() == source.path()
                || target.path().resource() != source.path().resource()
            {
                continue;
            }
            if let Some(argument) = id_positional(target) {
                links.push(Link {
                    source: source.path().clone(),
                    column: ID_COLUMN.to_owned(),
                    target: target.path().clone(),
                    argument: argument.to_owned(),
                    origin: Origin::Inferred,
                });
            }
        }
    }
    links
}

/// Returns true when the rows of `command` have a column named `id`.
fn has_id_column(command: &Command) -> bool {
    command
        .result()
        .columns()
        .iter()
        .any(|column| column.name() == ID_COLUMN)
}

/// Returns the name of the positional argument of `command` whose value name
/// is `ID`, such as `<ID>`.
fn id_positional(command: &Command) -> Option<&str> {
    command
        .arguments()
        .iter()
        .find(|argument| {
            argument.kind() == Kind::Positional && argument.arity().value_names() == [ID_VALUE]
        })
        .map(Argument::name)
}
