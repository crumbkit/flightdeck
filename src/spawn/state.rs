//! A fresh state directory for the runs that flightdeck makes on its own
//! account, so that a crumbkit tool writes their rows to a usage log that
//! nobody reads rather than to the person's.
//!
//! A crumbkit tool logs each run under `XDG_STATE_HOME`. The loader's
//! queries of a tool's description and the help measurement are flightdeck's
//! runs, not the person's or an agent's, so each runs with
//! `XDG_STATE_HOME` set to a new temporary directory. The directory is
//! private to the user, as `tempfile` makes it, and is removed when the
//! [`FreshState`] drops, on every path.

use std::io;

use tempfile::TempDir;

use super::Program;

/// The variable under which a crumbkit tool keeps its usage log.
const STATE_VARIABLE: &str = "XDG_STATE_HOME";

/// A new, empty temporary directory that serves as the state directory of
/// flightdeck's own runs, removed when the value drops.
#[derive(Debug)]
pub(crate) struct FreshState {
    directory: TempDir,
}

impl FreshState {
    /// Makes a new temporary state directory.
    ///
    /// # Errors
    ///
    /// Returns the error of a directory that cannot be made, such as one in
    /// a full or unwritable temporary directory.
    pub(crate) fn new() -> io::Result<Self> {
        Ok(Self {
            directory: tempfile::tempdir()?,
        })
    }

    /// Returns `program` with `XDG_STATE_HOME` set to this directory in
    /// every run, after any value that `program` already sets.
    pub(crate) fn keep_out(&self, program: &Program) -> Program {
        program
            .clone()
            .env(STATE_VARIABLE, self.directory.path().as_os_str())
    }
}
