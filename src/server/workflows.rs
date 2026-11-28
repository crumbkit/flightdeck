//! The workflows routes: `GET /api/workflows` answers with the document of
//! `flightdeck workflows list --json --full`, and
//! `GET /api/workflows/<slug>` with the document of `flightdeck workflows
//! show <slug> --json`, both from the library that the commands call.
//!
//! One workflows query runs at a time, and a request that arrives while one
//! runs waits for it, so the page's polls run at most the two Board
//! processes of one query at once and hold Board's index only lightly while
//! agents write. The server's stop cancels the query in progress.

use std::sync::Mutex;

use crumbkit::Failure;
use crumbkit_process::Cancel;

use super::document;
use super::reply::Reply;
use crate::lock::lock;
use crate::workflows::{self, Board, Slug, WorkflowsError};

/// The workflows routes' Board, their stop, and the turn that one query
/// holds at a time.
#[derive(Debug)]
pub(super) struct Workflows {
    board: Board,
    stop: Cancel,
    turn: Mutex<()>,
}

impl Workflows {
    /// Returns the routes of `board`.
    pub(super) fn new(board: Board) -> Self {
        Self {
            board,
            stop: Cancel::new(),
            turn: Mutex::new(()),
        }
    }

    /// Answers `GET /api/workflows`.
    pub(super) fn list(&self) -> Reply {
        let _turn = lock(&self.turn);
        reply(workflows::list(&self.board, &self.stop).map(|list| list.to_json()))
    }

    /// Answers `GET /api/workflows/<slug>`, with `slug` as the page sent
    /// it. A slug that is not lowercase words joined by hyphens gets 422
    /// and starts no query.
    pub(super) fn show(&self, slug: &str) -> Reply {
        let slug = match Slug::new(slug) {
            Ok(slug) => slug,
            Err(reason) => return reply(Err(WorkflowsError::Slug { reason })),
        };
        let _turn = lock(&self.turn);
        reply(workflows::show(&self.board, &slug, &self.stop).map(|shown| shown.to_json()))
    }

    /// Returns whether Board answers, even with a failure, so the page
    /// shows its Workflows entry. Only a Board that will not start answers
    /// false.
    pub(super) fn answers(&self) -> bool {
        let _turn = lock(&self.turn);
        workflows::list(&self.board, &self.stop).map_or_else(|error| !error.no_board(), |_| true)
    }

    /// Cancels the query in progress and every later one, as the server
    /// stops.
    pub(super) fn stop(&self) {
        self.stop.cancel();
    }
}

/// Returns the reply of a query's result.
fn reply(result: Result<Vec<u8>, WorkflowsError>) -> Reply {
    match result {
        Ok(body) => Reply::Json(body),
        Err(error) => Reply::Failure {
            status: status_of(&error),
            document: document::to_json(&error as &dyn Failure),
        },
    }
}

/// Returns the HTTP status of a workflows failure: 422 for a slug that
/// flightdeck refuses, 404 for a slug with no graph, 500 for a Board that
/// will not start, and 502 for a Board query that failed.
const fn status_of(error: &WorkflowsError) -> u16 {
    match error {
        WorkflowsError::Slug { .. } => 422,
        WorkflowsError::NotFound { .. } => 404,
        WorkflowsError::BoardMissing { .. } => 500,
        WorkflowsError::BoardFailed { .. } => 502,
    }
}
