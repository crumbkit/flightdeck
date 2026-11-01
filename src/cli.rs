//! flightdeck's command-line grammar and guide.

use clap::{Parser, Subcommand};
use crumbkit::ViewFlags;
use crumbkit::commands::{COMMANDS_LIST, CommandsList};
use crumbkit::help::{Effects, Example, Guide, Leaf};
use crumbkit::schemas::{SCHEMAS_LIST, SCHEMAS_SHOW, SchemasList, SchemasShow};

/// Start a local web page for a crumbkit tool (in development).
#[derive(Debug, Parser)]
#[command(name = "flightdeck", version)]
pub struct Cli {
    /// The resource the command acts on.
    #[command(subcommand)]
    pub(crate) resource: Resource,

    /// The flags that choose how the result prints.
    #[command(flatten)]
    pub(crate) view: ViewFlags,
}

/// flightdeck's leaves: the examples, effects and document of each of
/// crumbkit's own commands that flightdeck mounts.
pub const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["commands", "list"])
        .examples(&[
            Example::new(&[], "List every command"),
            Example::new(
                &["commands", "list", "--json", "--full"],
                "Print commands list with its arguments, examples and columns as JSON",
            ),
        ])
        .effects(Effects::read_only())
        .document(COMMANDS_LIST),
    Leaf::new(&["schemas", "list"])
        .examples(&[Example::new(
            &[],
            "List every document that flightdeck prints",
        )])
        .effects(Effects::read_only())
        .document(SCHEMAS_LIST),
    Leaf::new(&["schemas", "show"])
        .examples(&[Example::new(
            &["flightdeck/schemas-list/v1"],
            "Print the JSON Schema of the document that schemas list prints",
        )])
        .effects(Effects::read_only())
        .document(SCHEMAS_SHOW),
]);

/// The resources that flightdeck serves.
#[derive(Debug, Subcommand)]
pub(crate) enum Resource {
    /// Explore flightdeck's commands.
    Commands {
        /// What to do with the commands.
        #[command(subcommand)]
        command: CommandsCommand,
    },
    /// Explore the JSON documents that flightdeck prints.
    Schemas {
        /// What to do with the documents.
        #[command(subcommand)]
        command: SchemasCommand,
    },
}

/// The commands for flightdeck's own commands.
#[derive(Debug, Subcommand)]
pub(crate) enum CommandsCommand {
    /// List a branch of flightdeck's commands with their arguments, examples and columns.
    List(CommandsList),
}

/// The commands for the JSON documents that flightdeck prints.
#[derive(Debug, Subcommand)]
pub(crate) enum SchemasCommand {
    /// List the JSON documents that flightdeck prints, with a description of each.
    List(SchemasList),
    /// Print the JSON Schema of each document.
    Show(SchemasShow),
}
