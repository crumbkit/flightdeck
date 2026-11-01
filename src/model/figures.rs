//! How often each command ran, read from the tool's `self figures list`.
//!
//! crumbkit's usage log records each run of a tool, and `self figures list`
//! counts them per command. The page uses a command's runs only to order
//! palette matches of equal rank, so a tool without the figures, or a
//! figures call that fails, leaves every count at 0 and is no error.

use std::collections::HashMap;

use serde::Deserialize;

use super::documents::Listing;

/// The words of the command that prints the figures.
pub(super) const COMMAND: [&str; 3] = ["self", "figures", "list"];

/// The commands that the loader runs to describe the tool, by their path
/// below the tool's name.
///
/// RUST-10: the usage log counts the loader's own runs of these commands
/// beside the person's, one each per `flightdeck open`, so their counts
/// would grow until they outranked the tool's own commands. The figures
/// cannot tell the two apart, so these commands keep no runs, and the
/// person's own runs of them are missed. A usage log that records which
/// program ran each command, so the loader's runs can be left out, replaces
/// this.
const LOADER_QUERIES: [&str; 4] = [
    "commands list",
    "schemas list",
    "schemas show",
    "self figures list",
];

/// One row of `self figures list`, with the fields flightdeck reads.
#[derive(Debug, Deserialize)]
struct FigureRow {
    /// The command's path below the tool's name, such as `books list`.
    command: String,
    runs: u64,
}

/// Returns the runs of each command, by its path, from the standard output
/// of `self figures list`, or nothing when the output is not that document.
/// The loader's own queries are left out.
pub(super) fn read(stdout: &[u8]) -> HashMap<String, u64> {
    let Ok(listing) = serde_json::from_slice::<Listing<FigureRow>>(stdout) else {
        return HashMap::new();
    };
    if !listing.schema().ends_with("/self-figures-list/v1") {
        return HashMap::new();
    }
    listing
        .into_data()
        .into_iter()
        .filter(|row| !LOADER_QUERIES.contains(&row.command.as_str()))
        .map(|row| (row.command, row.runs))
        .collect()
}
