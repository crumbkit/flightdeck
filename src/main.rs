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
    crumbkit_flightdeck::run_from(std::env::args_os())
}
