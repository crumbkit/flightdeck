//! The weight of each command's path through the tool's help: the pages a
//! reader passes to reach it, which [`measure`] counts by running each
//! page's `-h` once.
//!
//! A command's path is the tool's own page, the page of each group above
//! the command and the command's own page, such as `shelf -h`,
//! `shelf books -h` and `shelf books list -h`. Commands share the pages
//! above them, so each distinct page runs once.

use crumbkit_process::Cancel;
use serde::Serialize;

use super::Tool;
use crate::spawn::{self, Exit, HelpLimits, HelpPage, Program};

/// The most distinct pages that one measurement runs.
pub const MOST_PAGES: usize = 400;

/// The distinct help pages of a tool's commands, in guide order, and the
/// commands left out because their pages passed [`MOST_PAGES`].
#[derive(Debug, PartialEq, Eq)]
pub struct Pages {
    paths: Vec<Vec<String>>,
    left_out: usize,
}

impl Pages {
    /// Returns each page's command path, `[]` for the tool's own page.
    #[must_use]
    pub fn paths(&self) -> &[Vec<String>] {
        &self.paths
    }

    /// Returns how many commands have a page beyond [`MOST_PAGES`].
    #[must_use]
    pub const fn left_out(&self) -> usize {
        self.left_out
    }
}

/// Returns the distinct help pages of `tool`'s commands in guide order: the
/// tool's own page, then for each command in the order `commands list`
/// prints them, each group page above it and its own page, each page the
/// first time it appears. It keeps at most `most` pages, and counts the
/// commands whose path needs a page beyond them.
#[must_use]
pub fn pages(tool: &Tool, most: usize) -> Pages {
    let mut paths: Vec<Vec<String>> = Vec::new();
    let mut left_out = 0;
    let add = |path: Vec<String>, paths: &mut Vec<Vec<String>>| {
        if paths.contains(&path) {
            return true;
        }
        if paths.len() >= most {
            return false;
        }
        paths.push(path);
        true
    };
    add(Vec::new(), &mut paths);
    for command in tool.commands() {
        let words = command.path().words();
        let mut kept = true;
        for end in 1..=words.len() {
            let path = words.get(..end).unwrap_or_default().to_vec();
            kept &= add(path, &mut paths);
        }
        if !kept {
            left_out += 1;
        }
    }
    Pages { paths, left_out }
}

/// The document of `GET /api/weight`: each page's bytes or why it has
/// none, the commands left out, and the problem of a measurement that
/// could not start.
#[derive(Debug, Serialize)]
pub struct Weight {
    pages: Vec<Measured>,
    left_out: usize,
    /// Why no page was measured, such as a temporary directory that could
    /// not be made, or `None` when the measurement ran.
    problem: Option<String>,
}

impl Weight {
    /// Returns each page that the measurement ran or meant to run.
    #[must_use]
    pub fn pages(&self) -> &[Measured] {
        &self.pages
    }

    /// Returns why no page was measured, or `None`.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }
}

/// One help page: its command path, and its bytes or why it has none.
#[derive(Debug, Serialize)]
pub struct Measured {
    path: Vec<String>,
    bytes: Option<u64>,
    /// Why the page has no bytes: `exit`, `signal`, `empty`, `timed_out`,
    /// `too_large`, `not_started`, `not_run` or `refused`, or `None` when it
    /// has them.
    failure: Option<&'static str>,
    /// The exit code or signal number of a page that failed with one.
    exit: Option<i32>,
}

impl Measured {
    /// Returns the page's command path.
    #[must_use]
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Returns the page's bytes, or `None` when it has none.
    #[must_use]
    pub const fn bytes(&self) -> Option<u64> {
        self.bytes
    }

    /// Returns why the page has no bytes, or `None`.
    #[must_use]
    pub const fn failure(&self) -> Option<&'static str> {
        self.failure
    }

    fn of(path: Vec<String>, page: HelpPage) -> Self {
        let (bytes, failure, exit) = match page {
            HelpPage::Bytes(bytes) => (Some(bytes), None, None),
            HelpPage::Exit(Exit::Code(code)) => (None, Some("exit"), Some(code)),
            HelpPage::Exit(Exit::Signal(signal)) => (None, Some("signal"), Some(signal)),
            HelpPage::Exit(Exit::TimedOut) | HelpPage::TimedOut => (None, Some("timed_out"), None),
            HelpPage::Exit(Exit::Unknown) | HelpPage::NotStarted => {
                (None, Some("not_started"), None)
            }
            HelpPage::Empty => (None, Some("empty"), None),
            HelpPage::TooLarge => (None, Some("too_large"), None),
            HelpPage::NotRun => (None, Some("not_run"), None),
            HelpPage::Refused => (None, Some("refused"), None),
        };
        Self {
            path,
            bytes,
            failure,
            exit,
        }
    }
}

/// Measures the help pages of `tool`, whose program is `program`, within
/// `limits`, until `stop` is cancelled. A measurement that cannot make its
/// temporary state directory measures nothing and says why.
#[must_use]
pub fn measure(program: &Program, tool: &Tool, limits: HelpLimits, stop: &Cancel) -> Weight {
    let pages = pages(tool, MOST_PAGES);
    match spawn::measure_help(program, pages.paths(), limits, stop) {
        Ok(measured) => Weight {
            pages: pages
                .paths
                .into_iter()
                .zip(measured)
                .map(|(path, page)| Measured::of(path, page))
                .collect(),
            left_out: pages.left_out,
            problem: None,
        },
        Err(error) => Weight {
            pages: Vec::new(),
            left_out: pages.left_out,
            problem: Some(format!("cannot make a temporary state directory: {error}")),
        },
    }
}
