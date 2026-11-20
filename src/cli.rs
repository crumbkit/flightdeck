//! flightdeck's command-line grammar and tool record.

use std::ffi::OsString;

use clap::{Args, Parser, Subcommand};
use crumbkit::commands::{COMMANDS_LIST, CommandsList};
use crumbkit::completion::{
    COMPLETION_CHECK, COMPLETION_INSTALL, COMPLETION_PRINT, CompletionCheck, CompletionInstall,
    CompletionPrint,
};
use crumbkit::help::{Effects, Example, Help, Leaf};
use crumbkit::schemas::{SCHEMAS_LIST, SCHEMAS_SHOW, SchemasList, SchemasShow};
use crumbkit::tool::Failures;
use crumbkit::{NoInput, Tool, ViewFlags};
use crumbkit_skills::{Skill, skills_command};

use crate::workspaces::{WORKSPACES_LIST, WORKSPACES_SHOW, WorkspaceName};

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

    /// The flag that tells flightdeck that nobody can answer a question.
    #[command(flatten)]
    pub(crate) no_input: NoInput,
}

/// flightdeck's tool record: `open` as a verb of the root, and the examples,
/// effects and document of each command.
pub const TOOL: Tool = Tool::new()
    .failures(Failures::new().exits(&crate::open::FORCED_EXITS))
    .help(
    Help::new()
        .root_verbs(
            &["open"],
            "flightdeck opens a page for a tool, and a tool is no resource of flightdeck's own.",
        )
        .leaves(&[
            Leaf::new(&["open"])
                .examples(&[
                    Example::new(&[], "Open the Workspaces page in the browser")
                        .needs(&["browser"]),
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
            Leaf::new(&["workspaces", "list"])
                .examples(&[
                    Example::new(&[], "List the local mxv workspaces").needs(&["mxv"]),
                    Example::new(
                        &["--json", "--full"],
                        "Print the document that the Workspaces page draws",
                    )
                    .needs(&["mxv"]),
                ])
                .effects(Effects::read_only())
                .document(WORKSPACES_LIST),
            Leaf::new(&["workspaces", "show"])
                .examples(&[Example::new(
                    &["reading-list", "--json"],
                    "Print the document that the page of the workspace reading-list draws",
                )
                .needs(&["mxv"])])
                .effects(Effects::read_only())
                .document(WORKSPACES_SHOW),
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
            FlightdeckSkills::install_leaf(&["skills", "install"]),
            FlightdeckSkills::check_leaf(&["skills", "check"]),
            FlightdeckSkills::print_leaf(&["skills", "print"]),
            Leaf::new(&["completion", "install"])
                .examples(&[
                    Example::new(&["--shell", "fish"], "Install flightdeck's completion for fish"),
                    Example::new(
                        &["--shell", "zsh", "--force"],
                        "Install it for zsh, replacing a file or a link that install did not write",
                    ),
                ])
                .effects(Effects::additive().idempotent())
                .document(COMPLETION_INSTALL),
            Leaf::new(&["completion", "check"])
                .examples(&[Example::new(
                    &["--shell", "zsh"],
                    "Report zsh's completion file when it is missing or differs",
                )])
                .effects(Effects::read_only())
                .document(COMPLETION_CHECK),
            Leaf::new(&["completion", "print"])
                .examples(&[Example::new(
                    &["--shell", "fish"],
                    "Print the registration script for fish",
                )])
                .effects(Effects::read_only())
                .document(COMPLETION_PRINT),
            crumbkit_starmap::list_leaf(&["self", "environment", "list"]),
            Leaf::new(&["schemas", "show"])
                .examples(&[Example::new(
                    &["flightdeck/schemas-list/v1"],
                    "Print the JSON Schema of the document that schemas list prints",
                )])
                .effects(Effects::read_only())
                .document(SCHEMAS_SHOW),
        ]),
);

skills_command!(
    /// The commands for flightdeck's skill.
    pub(crate) FlightdeckSkills,
    "flightdeck"
);

/// flightdeck's agent skill, which `flightdeck skills install` writes into
/// the agent skills directories.
pub(crate) const SKILL: Skill = Skill::new(
    "flightdeck",
    env!("CARGO_PKG_VERSION"),
    include_str!("../skill/SKILL.md"),
);

/// The resources that flightdeck serves, and the verb `open`.
#[derive(Debug, Subcommand)]
pub(crate) enum Resource {
    /// Open a local web page of the mxv workspaces and a tool's commands.
    Open(OpenArgs),
    /// Read the local mxv workspaces.
    Workspaces {
        /// What to do with the workspaces.
        #[command(subcommand)]
        command: WorkspacesCommand,
    },
    /// Explore flightdeck's commands.
    Commands {
        /// What to do with the commands.
        #[command(subcommand)]
        command: CommandsCommand,
    },
    /// Give coding agents flightdeck's skill.
    Skills {
        /// What to do with the skill.
        #[command(subcommand)]
        command: FlightdeckSkills,
    },
    /// Set up flightdeck's completion in a shell.
    Completion {
        /// What to do with the completion.
        #[command(subcommand)]
        command: CompletionCommand,
    },
    /// Explore the JSON documents that flightdeck prints.
    Schemas {
        /// What to do with the documents.
        #[command(subcommand)]
        command: SchemasCommand,
    },
    /// Review flightdeck itself.
    #[command(name = "self")]
    SelfCommand {
        /// What to review.
        #[command(subcommand)]
        command: SelfCommand,
    },
}

/// The commands that review flightdeck itself.
#[derive(Debug, Subcommand)]
pub(crate) enum SelfCommand {
    /// Explore the environment variables that flightdeck reads.
    Environment {
        /// What to do with them.
        #[command(subcommand)]
        verb: EnvironmentVerb,
    },
}

/// What to do with flightdeck's environment variables.
#[derive(Debug, Subcommand)]
pub(crate) enum EnvironmentVerb {
    /// List the variables that flightdeck and crumbkit read, with their rules and values.
    List,
}

/// The commands that read the local mxv workspaces.
#[derive(Debug, Subcommand)]
pub(crate) enum WorkspacesCommand {
    /// List the local mxv workspaces with their units, routes and ports.
    List,
    /// Show one local mxv workspace with its checkouts and units.
    Show(WorkspacesShowArgs),
}

/// The arguments of `flightdeck workspaces show`.
#[derive(Debug, Args)]
pub(crate) struct WorkspacesShowArgs {
    /// The workspace's name, as `flightdeck workspaces list` prints it.
    #[arg(value_name = "NAME")]
    pub(crate) name: WorkspaceName,
}

/// The commands for flightdeck's own commands.
#[derive(Debug, Subcommand)]
pub(crate) enum CommandsCommand {
    /// List a branch of flightdeck's commands with their arguments, examples and columns.
    List(CommandsList),
}

/// The commands for flightdeck's shell completion.
#[derive(Debug, Subcommand)]
pub(crate) enum CompletionCommand {
    /// Install flightdeck's completion where the shell loads it.
    Install(CompletionInstall),
    /// Check that flightdeck's installed completion file matches this flightdeck.
    Check(CompletionCheck),
    /// Print flightdeck's completion registration script for a shell.
    Print(CompletionPrint),
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
    /// The crumbkit tool, by a name on PATH or a path; without one, the page shows the workspaces.
    #[arg(value_name = "TOOL")]
    pub(crate) tool: Option<OsString>,

    /// Run only the tool's read-only commands from the page.
    #[arg(long)]
    pub(crate) read_only: bool,

    /// Print the path of the page to open instead of opening the browser.
    #[arg(long)]
    pub(crate) no_browser: bool,
}
