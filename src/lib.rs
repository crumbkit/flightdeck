//! flightdeck starts a local web page for any tool built on crumbkit. It
//! reads only the JSON documents that the tool prints about itself, and
//! links nothing of the tool.
//!
//! The library holds the whole program, and the `flightdeck` binary calls
//! [`run_from`]. It exports the parser and the tool record for flightdeck's
//! convention tests, and the [`clock`], [`model`], [`runner`], [`spawn`],
//! [`opener`] and [`server`] modules for its other tests. Its API makes no
//! stability promise.

mod cli;
pub mod clock;
pub mod compliance;
mod environment;
mod lock;
pub mod model;
mod open;
pub mod opener;
pub mod runner;
pub mod server;
pub mod spawn;
pub mod workspaces;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::CommandFactory as _;
use starmap::Env as _;

pub use crate::cli::{Cli, TOOL};
use crate::cli::{
    CommandsCommand, CompletionCommand, ComplianceBuildArgs, ComplianceCommand, EnvironmentVerb,
    Resource, SKILL, SchemasCommand, SelfCommand, WorkspacesCommand,
};
use crate::environment::Environment;
pub use crate::open::{ENVIRONMENT_INVALID, OpenError, SERVER_FAILED, TOOL_SEARCH_PATH};

/// Every failure code that flightdeck declares, in one list for each error
/// type, which its convention tests pass to crumbkit-test's checks.
pub const CODES: &[&[crumbkit::Code<'static>]] = &[
    compliance::ComplianceError::CODES,
    model::LoadError::CODES,
    OpenError::CODES,
    runner::RunFailure::CODES,
    runner::EndFailure::CODES,
    workspaces::WorkspacesError::CODES,
];

/// Runs flightdeck on `args`, whose first item is the program's name, with
/// `vars` as its process environment, and returns the process's exit code.
/// `main` reads the environment once (BASE-6), and flightdeck reads the
/// variables that starmap.toml declares from `vars`.
///
/// `crumbkit::parse_from` prints help, the version or a refused command
/// line and returns its exit code. `crumbkit::finish` prints the result or
/// the failure in the format that the output flags choose.
#[must_use = "the exit code reports how the command ended, so return it from main"]
pub fn run_from<I, T>(args: I, vars: &[(OsString, OsString)]) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let (cli, mut run) = match crumbkit::parse_from::<Cli>(&TOOL, args) {
        Ok(parsed) => parsed,
        Err(stop) => return stop.exit_code(),
    };
    // Each command reads the command tree through `&mut run`, so it runs
    // before `finish` takes `run`.
    match cli.resource {
        Resource::Open(args) => open::run(&mut run, &args, vars),
        Resource::Workspaces { command } => workspaces_command(run, &command, vars),
        Resource::Compliance {
            command: ComplianceCommand::Build(args),
        } => {
            let result = compliance_build(&args);
            crumbkit::finish(run, compliance::COMPLIANCE_BUILD, result)
        }
        Resource::Commands {
            command: CommandsCommand::List(args),
        } => {
            let result = crumbkit::commands::list(&mut run, &TOOL, &args);
            crumbkit::finish(run, crumbkit::commands::COMMANDS_LIST, result)
        }
        Resource::Skills { command } => crumbkit_skills::run(run, &[SKILL], command),
        Resource::Completion { command } => match command {
            CompletionCommand::Install(args) => {
                let result = crumbkit::completion::install(Cli::command, &args);
                crumbkit::finish(run, crumbkit::completion::COMPLETION_INSTALL, result)
            }
            CompletionCommand::Check(args) => {
                let result = crumbkit::completion::check(Cli::command, &args);
                crumbkit::finish(run, crumbkit::completion::COMPLETION_CHECK, result)
            }
            CompletionCommand::Print(args) => {
                let result = crumbkit::completion::print(Cli::command, &args);
                crumbkit::finish(run, crumbkit::completion::COMPLETION_PRINT, result)
            }
        },
        Resource::Schemas { command } => match command {
            SchemasCommand::List(args) => {
                let result = crumbkit::schemas::list(&mut run, &TOOL, &args);
                crumbkit::finish(run, crumbkit::schemas::SCHEMAS_LIST, result)
            }
            SchemasCommand::Show(args) => {
                let result = crumbkit::schemas::show(&mut run, &TOOL, &args);
                crumbkit::finish(run, crumbkit::schemas::SCHEMAS_SHOW, result)
            }
        },
        Resource::SelfCommand {
            command:
                SelfCommand::Environment {
                    verb: EnvironmentVerb::List,
                },
        } => {
            let result =
                crumbkit_starmap::environment(&mut run, <Environment as starmap::Env>::SPEC, vars);
            crumbkit::finish(run, crumbkit_starmap::SELF_ENVIRONMENT_LIST, result)
        }
    }
}

/// Runs `flightdeck compliance build`: builds the document, and with
/// `--out` writes it and the page that draws it, naming both files and the
/// page's size, with any warning about the page, in the text output's
/// note and in the printed document's `site`.
fn compliance_build(
    args: &ComplianceBuildArgs,
) -> Result<crumbkit::Prose<compliance::ComplianceDocument>, compliance::ComplianceError> {
    let request = compliance::BuildRequest {
        config: args.config.clone(),
        reports: args.reports.clone(),
        built_at: args.built_at.unwrap_or_else(jiff::Timestamp::now),
        build_run: args.build_run.clone(),
        build_url: args.build_url.clone(),
        rules_commit: args.rules_commit.clone(),
    };
    let mut document = compliance::build(&request)?;
    match &args.out {
        Some(out) => {
            let written = compliance::write_out(out, &document)?;
            document.site = Some(written.files());
            Ok(crumbkit::Prose::new(document).note(written.note()))
        }
        None => Ok(crumbkit::Prose::new(document)),
    }
}

/// Runs `flightdeck workspaces list` or `show` against the mxv that
/// `FLIGHTDECK_MXV` names, or `mxv` on `PATH`, and prints its document.
fn workspaces_command(
    mut run: crumbkit::Run,
    command: &WorkspacesCommand,
    vars: &[(OsString, OsString)],
) -> ExitCode {
    let environment = match Environment::from_vars(vars.iter().cloned()) {
        Ok(environment) => environment,
        Err(error) => {
            return run.fail(
                &mut crumbkit::Sink::standard(),
                &OpenError::Environment(error),
            );
        }
    };
    let launch = open::launch_directory();
    let mxv = workspaces::Mxv::at_launch(environment.mxv(), launch.as_deref(), environment.path());
    let stop = crumbkit_process::Cancel::new();
    match command {
        WorkspacesCommand::List => {
            let result = workspaces::list(&mxv, &stop).map(workspaces::WorkspacesList::listing);
            crumbkit::finish(run, workspaces::WORKSPACES_LIST, result)
        }
        WorkspacesCommand::Show(args) => {
            let result = workspaces::show(&mxv, &args.name, &stop).map(crumbkit::Prose::new);
            crumbkit::finish(run, workspaces::WORKSPACES_SHOW, result)
        }
    }
}
