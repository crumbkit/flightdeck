//! Links from a cell of one command's rows to another command that takes
//! the cell's value.

use serde::Serialize;

use super::argument::{Argument, Kind};
use super::command::{Command, CommandPath};

/// The column whose cells an inferred link starts from.
const ID_COLUMN: &str = "id";

/// The value name of the positional argument that an inferred link fills.
const ID_VALUE: &str = "ID";

/// A link from the `id` column of one command's rows to an argument of
/// another command, which takes the column's value. flightdeck infers each
/// link from crumbkit's command grammar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Link {
    source: CommandPath,
    target: CommandPath,
    argument: String,
}

impl Link {
    /// Returns the command whose rows hold the column.
    #[must_use]
    pub const fn source(&self) -> &CommandPath {
        &self.source
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
                    target: target.path().clone(),
                    argument: argument.to_owned(),
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
