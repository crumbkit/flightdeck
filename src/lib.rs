//! flightdeck will start a local web page that lists, runs and draws the
//! commands of any tool built on crumbkit. It reads only the JSON documents
//! that the tool prints about itself, and links nothing of the tool.
//!
//! The library holds the whole program, and the `flightdeck` binary calls
//! [`run_from`]. It exports the parser and the guide for flightdeck's
//! convention tests. Its API makes no stability promise.

mod cli;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::CommandFactory as _;

pub use crate::cli::{Cli, GUIDE};
use crate::cli::{CommandsCommand, Resource, SchemasCommand};

/// Every failure code that flightdeck declares, which its convention tests
/// pass to crumbkit-test's checks.
pub const CODES: &[&crumbkit::Code] = &[];

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
    let (cli, mut output) = match crumbkit::parse_from::<Cli, _, _>(Cli::command(), &GUIDE, args) {
        Ok(parsed) => parsed,
        Err(stop) => return stop.exit_code(),
    };
    // Each command reads the command tree through `&mut output`, so it runs
    // before `finish` takes `output`.
    match cli.resource {
        Resource::Commands {
            command: CommandsCommand::List(args),
        } => {
            let result = crumbkit::commands::list(&mut output, &GUIDE, &args);
            crumbkit::finish(output, crumbkit::commands::COMMANDS_LIST, result)
        }
        Resource::Schemas { command } => match command {
            SchemasCommand::List(args) => {
                let result = crumbkit::schemas::list(&mut output, &GUIDE, &args);
                crumbkit::finish(output, crumbkit::schemas::SCHEMAS_LIST, result)
            }
            SchemasCommand::Show(args) => {
                let result = crumbkit::schemas::show(&mut output, &GUIDE, &args);
                crumbkit::finish(output, crumbkit::schemas::SCHEMAS_SHOW, result)
            }
        },
    }
}
