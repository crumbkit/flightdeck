//! The workspaces routes: `GET /api/workspaces` answers with the document of
//! `flightdeck workspaces list --json --full`, and
//! `GET /api/workspaces/<name>` with the document of `flightdeck workspaces
//! show <name> --json`, both from the library that the commands call.
//!
//! At most one query of each route runs at a time. A request that arrives
//! while a query of the same kind runs waits for it, and gets that query's
//! answer when it asked for the same thing, as the weight measurement shares
//! its one measurement. A request that arrives after a query ended starts a
//! new one, so no answer outlives the request that it answered.
//!
//! The server's stop cancels the queries in progress, so a query never
//! holds up the stop.

use std::sync::{Arc, Mutex};

use crumbkit::Failure;
use crumbkit_process::Cancel;

use super::document;
use super::reply::Reply;
use crate::lock::lock;
use crate::workspaces::{self, Mxv, WorkspaceName, WorkspacesError};

#[cfg(unix)]
#[cfg(test)]
mod tests;

/// The workspaces routes' mxv, their stop, and the query of each route.
#[derive(Debug)]
pub(super) struct Workspaces {
    mxv: Mxv,
    stop: Cancel,
    list: Flight<()>,
    show: Flight<WorkspaceName>,
}

/// The status and body of a workspaces route's reply, which requests that
/// shared a query share.
#[derive(Debug)]
struct Answered {
    status: u16,
    body: Vec<u8>,
}

impl Workspaces {
    /// Returns the routes of `mxv`.
    pub(super) fn new(mxv: Mxv) -> Self {
        Self {
            mxv,
            stop: Cancel::new(),
            list: Flight::default(),
            show: Flight::default(),
        }
    }

    /// Answers `GET /api/workspaces`.
    pub(super) fn list(&self) -> Reply {
        let answered = self.list.get((), || {
            answer(workspaces::list(&self.mxv, &self.stop).map(|list| list.to_json()))
        });
        reply(&answered)
    }

    /// Answers `GET /api/workspaces/<name>`, with `name` as the page sent
    /// it. A name that flightdeck refuses gets 422 and starts no query.
    pub(super) fn show(&self, name: &str) -> Reply {
        let name = match WorkspaceName::new(name) {
            Ok(name) => name,
            Err(reason) => return reply(&answer(Err(WorkspacesError::Name { reason }))),
        };
        let answered = self.show.get(name.clone(), || {
            answer(workspaces::show(&self.mxv, &name, &self.stop).map(|shown| shown.to_json()))
        });
        reply(&answered)
    }

    /// Cancels the queries in progress and every later one, as the server
    /// stops.
    pub(super) fn stop(&self) {
        self.stop.cancel();
    }
}

/// Returns the status and body of a query's result.
fn answer(result: Result<Vec<u8>, WorkspacesError>) -> Answered {
    match result {
        Ok(body) => Answered { status: 200, body },
        Err(error) => Answered {
            status: status_of(&error),
            body: document::to_json(&error as &dyn Failure),
        },
    }
}

/// Returns the reply of `answered`.
fn reply(answered: &Answered) -> Reply {
    if answered.status == 200 {
        Reply::Json(answered.body.clone())
    } else {
        Reply::Failure {
            status: answered.status,
            document: answered.body.clone(),
        }
    }
}

/// Returns the HTTP status of a workspaces failure: 422 for a name that
/// flightdeck refuses, 404 for a workspace that mxv does not know, 500 for
/// an mxv that will not start, and 502 for an mxv query that failed.
pub(super) const fn status_of(error: &WorkspacesError) -> u16 {
    match error {
        WorkspacesError::Name { .. } => 422,
        WorkspacesError::NotFound { .. } => 404,
        WorkspacesError::MxvMissing { .. } => 500,
        WorkspacesError::MxvFailed { .. } => 502,
    }
}

/// One query at a time, and the answer of the last one with the key it
/// answered.
#[derive(Debug)]
struct Flight<K> {
    last: Mutex<Last<K>>,
    running: Mutex<()>,
}

/// The number of queries that have ended, and the last one's key and
/// answer.
#[derive(Debug)]
struct Last<K> {
    generation: u64,
    answered: Option<(K, Arc<Answered>)>,
}

impl<K> Default for Flight<K> {
    fn default() -> Self {
        Self {
            last: Mutex::new(Last {
                generation: 0,
                answered: None,
            }),
            running: Mutex::new(()),
        }
    }
}

impl<K: PartialEq> Flight<K> {
    /// Returns the answer of a query of `key` that ended while this call
    /// waited for its turn, or else the answer that `query` gives now.
    fn get(&self, key: K, query: impl FnOnce() -> Answered) -> Arc<Answered> {
        let seen = lock(&self.last).generation;
        let _running = lock(&self.running);
        {
            let last = lock(&self.last);
            if last.generation != seen
                && let Some((answered_key, answered)) = &last.answered
                && *answered_key == key
            {
                return Arc::clone(answered);
            }
        }
        let answered = Arc::new(query());
        let mut last = lock(&self.last);
        last.generation += 1;
        last.answered = Some((key, Arc::clone(&answered)));
        answered
    }
}
