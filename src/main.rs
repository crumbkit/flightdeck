//! The `flightdeck` binary, which runs the program that the library holds.

use std::process::ExitCode;

fn main() -> ExitCode {
    crumbkit_flightdeck::run_from(std::env::args_os())
}
