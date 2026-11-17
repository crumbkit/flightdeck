//! The `flightdeck` binary, which runs the program that the library holds.

use std::process::ExitCode;

fn main() -> ExitCode {
    // A shell's completion request ends the run here, before any other work.
    // flightdeck parses against `Cli::command()`, so completion offers what
    // parsing accepts.
    if let Some(code) =
        crumbkit::complete(<crumbkit_flightdeck::Cli as clap::CommandFactory>::command)
    {
        return code;
    }
    #[expect(
        clippy::disallowed_methods,
        reason = "BASE-6: main reads the environment once, and flightdeck reads the variables that starmap.toml declares from this snapshot"
    )]
    let vars: Vec<_> = std::env::vars_os().collect();
    crumbkit_flightdeck::run_from(std::env::args_os(), &vars)
}
