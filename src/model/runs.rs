//! Each run in the tool's usage log, in order within its session, read from
//! the tool's `self runs list`.
//!
//! The page derives from these runs what no figure reports: the sessions
//! that ran each command, its failures by code, which command followed
//! which, each session's lane of runs and the flags that each command's
//! runs passed. A tool without `self runs list`, or a call that fails or
//! prints another document, leaves the tool with no history and is no
//! error.
//!
//! This module is the one reader of the document, so a change to its
//! fields changes only this file and the page's schema of a run.

use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::figures::{is_loader_query, is_safe, read_listing, tool_prefix, without_tool};

/// The words of the command that prints the runs.
pub(super) const RUNS_COMMAND: [&str; 3] = ["self", "runs", "list"];

/// One run in the usage log, as the page reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Run {
    session: u64,
    step: u64,
    /// When the run ended, in whole seconds since the Unix epoch.
    at: u64,
    caller: String,
    command: String,
    flags: Vec<String>,
    help: bool,
    exit: Option<u8>,
    code: Option<String>,
    ms: u64,
    out_bytes: Option<u64>,
}

impl Run {
    /// Returns the run's session, numbered from 1 in the order of each
    /// session's first run.
    #[must_use]
    pub const fn session(&self) -> u64 {
        self.session
    }

    /// Returns the run's place in its session, from 1. The loader's own
    /// queries are left out, so the steps of a session can skip numbers.
    #[must_use]
    pub const fn step(&self) -> u64 {
        self.step
    }

    /// Returns when the run ended, in whole seconds since the Unix epoch.
    #[must_use]
    pub const fn at(&self) -> u64 {
        self.at
    }

    /// Returns the agent that ran it, such as `claude-code` or `person`.
    #[must_use]
    pub fn caller(&self) -> &str {
        &self.caller
    }

    /// Returns the command's path below the tool's name, such as
    /// `books show`, or the tool's name for its root command.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Returns the names of the flags that the caller typed, never their
    /// values.
    #[must_use]
    pub fn flags(&self) -> &[String] {
        &self.flags
    }

    /// Returns true when the run printed a help page.
    #[must_use]
    pub const fn help(&self) -> bool {
        self.help
    }

    /// Returns the run's exit code, or `None` when it ended before it
    /// exited.
    #[must_use]
    pub const fn exit(&self) -> Option<u8> {
        self.exit
    }

    /// Returns the code of the run's failure or refusal without the tool's
    /// name in front, such as `books::not_found`.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns the run's wall time in milliseconds.
    #[must_use]
    pub const fn ms(&self) -> u64 {
        self.ms
    }

    /// Returns the bytes the run wrote to standard output, or `None` when
    /// the run did not reach `finish`.
    #[must_use]
    pub const fn out_bytes(&self) -> Option<u64> {
        self.out_bytes
    }
}

/// One row of `self runs list` as the tool prints it. serde ignores the
/// fields the page does not draw, such as the trace ids, and the fields
/// that a newer crumbkit-usage adds.
#[derive(Debug, Deserialize)]
struct RunRow {
    session: u64,
    step: u64,
    at: String,
    caller: String,
    command: String,
    flags: Vec<String>,
    help: bool,
    exit: Option<u8>,
    code: Option<String>,
    ms: u64,
    out_bytes: Option<u64>,
}

impl RunRow {
    /// Returns true when the row is the loader's `<tool> --version`: the
    /// root command, with no flags, no help page and exit 0.
    ///
    /// RUST-10: the log records neither `--version` nor which program ran
    /// the tool, so this matches by the row's shape. It also leaves out any
    /// other run of the bare root command that succeeds without a flag,
    /// which a crumbkit tool prints only for `--version`. A log that records
    /// the calling program replaces this, as it replaces `LOADER_QUERIES`.
    fn is_version_query(&self, tool: &str) -> bool {
        self.command == tool && self.flags.is_empty() && !self.help && self.exit == Some(0)
    }

    /// Returns the run, or `None` when its time is not RFC 3339 or a count
    /// is too large for the page.
    fn checked(self, prefix: &str) -> Option<Run> {
        let counts = [
            Some(self.session),
            Some(self.step),
            Some(self.ms),
            self.out_bytes,
        ];
        if !counts.into_iter().flatten().all(is_safe) {
            return None;
        }
        let at = humantime::parse_rfc3339(&self.at)
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();
        Some(Run {
            session: self.session,
            step: self.step,
            at,
            caller: self.caller,
            command: self.command,
            flags: self.flags,
            help: self.help,
            exit: self.exit,
            code: self.code.map(|code| without_tool(code, prefix)),
            ms: self.ms,
            out_bytes: self.out_bytes,
        })
    }
}

/// Each run in the tool's usage log, grouped by session and in order
/// within each session, as `self runs list` prints them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct History {
    runs: Vec<Run>,
}

impl History {
    /// Reads the standard output of `self runs list` from the tool named
    /// `tool`, or returns `None` when it is not that document. A row that
    /// does not read is left out alone, and so are the loader's own
    /// queries.
    pub(super) fn read(stdout: &[u8], tool: &str) -> Option<Self> {
        let listing = read_listing::<serde_json::Value>(stdout, "self-runs-list")?;
        let prefix = tool_prefix(tool);
        let runs = listing
            .into_data()
            .into_iter()
            .filter_map(|row| serde_json::from_value::<RunRow>(row).ok())
            .filter(|row| !is_loader_query(&row.command) && !row.is_version_query(tool))
            .filter_map(|row| row.checked(&prefix))
            .collect();
        Some(Self { runs })
    }

    /// Returns the runs in the order the tool printed them.
    #[must_use]
    pub fn runs(&self) -> &[Run] {
        &self.runs
    }
}
