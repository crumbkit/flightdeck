//! flightdeck's command-line grammar and tool record.

use std::ffi::OsString;

use clap::{Args, Parser, Subcommand};
use crumbkit::Tool;
use crumbkit::ViewFlags;
use crumbkit::commands::{COMMANDS_LIST, CommandsList};
use crumbkit::help::{Effects, Example, Help, Leaf};
use crumbkit::schemas::{SCHEMAS_LIST, SCHEMAS_SHOW, SchemasList, SchemasShow};

/// Start a local web page for a crumbkit tool.
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

/// flightdeck's tool record: `open` as a verb of the root, and the examples,
/// effects and document of each command.
pub const TOOL: Tool = Tool::new().help(
    Help::new()
        .root_verbs(
            &["open"],
            "flightdeck opens a page for a tool, and a tool is no resource of flightdeck's own.",
        )
        .leaves(&[
            Leaf::new(&["open"])
                .examples(&[
                    Example::new(&["shelf"], "Open the page for shelf in the browser")
                        .needs(&["browser"]),
                    Example::new(
                        &["./target/debug/shelf", "--read-only", "--no-browser"],
                        "Serve a built shelf's read-only commands and print the page to open",
                    )
                    .needs(&["browser"]),
                ])
                .effects(Effects::read_only().open_world())
                .prints_no_result(),
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
        ]),
);

/// The resources that flightdeck serves, and the verb `open`.
#[derive(Debug, Subcommand)]
pub(crate) enum Resource {
    /// Open a local web page that lists and runs a tool's commands.
    Open(OpenArgs),
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

/// The arguments of `flightdeck open`.
#[derive(Debug, Args)]
pub(crate) struct OpenArgs {
    /// The crumbkit tool, by a name on PATH or a path.
    #[arg(value_name = "TOOL")]
    pub(crate) tool: OsString,

    /// Run only the tool's read-only commands from the page.
    #[arg(long)]
    pub(crate) read_only: bool,

    /// Print the path of the page to open instead of opening the browser.
    #[arg(long)]
    pub(crate) no_browser: bool,
}
