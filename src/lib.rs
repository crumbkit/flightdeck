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
mod lock;
pub mod model;
mod open;
pub mod opener;
pub mod runner;
pub mod server;
pub mod spawn;

use std::ffi::OsString;
use std::process::ExitCode;

pub use crate::cli::{Cli, TOOL};
use crate::cli::{CommandsCommand, Resource, SchemasCommand};
pub use crate::open::{OpenError, SERVER_FAILED, TOOL_SEARCH_PATH};

/// Every failure code that flightdeck declares, in one list for each error
/// type, which its convention tests pass to crumbkit-test's checks.
pub const CODES: &[&[&crumbkit::Code]] = &[
    model::LoadError::CODES,
    OpenError::CODES,
    runner::RunFailure::CODES,
    runner::EndFailure::CODES,
];

/// Runs flightdeck on `args`, whose first item is the program's name, and
/// returns the process's exit code.
///
/// `crumbkit::parse_from` prints help, the version or a refused command
/// line and returns its exit code. `crumbkit::finish` prints the result or
/// the failure in the format that the output flags choose.
#[must_use]
pub fn run_from<I, T>(args: I) -> ExitCode
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
        Resource::Open(args) => open::run(&mut run, &args),
        Resource::Commands {
            command: CommandsCommand::List(args),
        } => {
            let result = crumbkit::commands::list(&mut run, &TOOL, &args);
            crumbkit::finish(run, crumbkit::commands::COMMANDS_LIST, result)
        }
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
    }
}
