//! How often each command ran, read from the tool's `self figures list`.
//!
//! crumbkit's usage log records each run of a tool, and `self figures list`
//! counts them per command. The page uses a command's runs only to order
//! palette matches of equal rank, so a tool without the figures, or a
//! figures call that fails, leaves every count at 0 and is no error.

use serde::Deserialize;

use super::documents::Listing;

/// The words of the command that prints the figures.
pub(super) const COMMAND: [&str; 3] = ["self", "figures", "list"];

/// One row of `self figures list`, with the fields flightdeck reads.
#[derive(Debug, Deserialize)]
struct FigureRow {
    /// The command's path below the tool's name, such as `books list`.
    command: String,
    runs: u64,
}

/// Returns each command's path and runs from the standard output of
/// `self figures list`, or nothing when the output is not that document.
pub(super) fn read(stdout: &[u8]) -> Vec<(String, u64)> {
    let Ok(listing) = serde_json::from_slice::<Listing<FigureRow>>(stdout) else {
        return Vec::new();
    };
    if !listing.schema().ends_with("/self-figures-list/v1") {
        return Vec::new();
    }
    listing
        .into_data()
        .into_iter()
        .map(|row| (row.command, row.runs))
        .collect()
}
