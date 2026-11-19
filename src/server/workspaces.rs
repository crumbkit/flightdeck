//! The workspaces routes: `GET /api/workspaces` answers with the document of
//! `flightdeck workspaces list --json --full`, and
//! `GET /api/workspaces/<name>` with the document of `flightdeck workspaces
//! show <name> --json`, both from the library that the commands call.
//!
//! At most one list query, and one show query of each name, runs at a time.
//! A request that arrives while such a query runs waits for it and gets its
//! answer, as the weight measurement shares its one measurement, so a slow
//! `mxv status` of one workspace never delays another's page. A request that
//! arrives after a query ended starts a new one, so no answer outlives the
//! request that it answered.
//!
//! The server's stop cancels the queries in progress, so a query never
//! holds up the stop.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crumbkit::Failure;
use crumbkit_process::Cancel;

use super::document;
use super::reply::Reply;
use crate::lock::lock;
use crate::workspaces::{self, Mxv, NameError, WorkspaceName, WorkspacesError};

#[cfg(unix)]
#[cfg(test)]
mod tests;

/// The workspaces routes' mxv, their stop, the list's query, and the show
/// query of each name that one runs or waits for.
#[derive(Debug)]
pub(super) struct Workspaces {
    mxv: Mxv,
    stop: Cancel,
    list: Flight<()>,
    show: Mutex<HashMap<WorkspaceName, Arc<Flight<()>>>>,
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
            show: Mutex::new(HashMap::new()),
        }
    }

    /// Answers `GET /api/workspaces`.
    pub(super) fn list(&self) -> Reply {
        let answered = self.list.get((), || {
            answer(workspaces::list(&self.mxv, &self.stop).map(|list| list.to_json()))
        });
        reply(&answered)
    }

    /// Answers `GET /api/workspaces/<name>`, with `escaped` as the page
    /// sent it, percent-encoded. A name that does not decode, or that
    /// flightdeck refuses, gets 422 and starts no query.
    pub(super) fn show(&self, escaped: &str) -> Reply {
        let name = match decoded(escaped).and_then(|text| WorkspaceName::new(&text)) {
            Ok(name) => name,
            Err(reason) => return reply(&answer(Err(WorkspacesError::Name { reason }))),
        };
        let flight = Arc::clone(lock(&self.show).entry(name.clone()).or_default());
        let answered = flight.get((), || {
            answer(workspaces::show(&self.mxv, &name, &self.stop).map(|shown| shown.to_json()))
        });
        // The map holds a name's flight only while a request runs or waits
        // on it, so names asked once do not pile up.
        let mut flights = lock(&self.show);
        if Arc::strong_count(&flight) == 2 {
            flights.remove(&name);
        }
        drop(flights);
        reply(&answered)
    }

    /// Cancels the queries in progress and every later one, as the server
    /// stops.
    pub(super) fn stop(&self) {
        self.stop.cancel();
    }
}

/// Returns `escaped` with each `%XX` replaced by its byte, or
/// [`NameError::Escape`] when an escape is cut short or the bytes are not
/// UTF-8.
fn decoded(escaped: &str) -> Result<String, NameError> {
    let mut bytes = Vec::with_capacity(escaped.len());
    let mut rest = escaped.as_bytes();
    while let Some((&byte, after)) = rest.split_first() {
        if byte == b'%' {
            let hex = after.get(..2).ok_or(NameError::Escape)?;
            let text = std::str::from_utf8(hex).map_err(|_| NameError::Escape)?;
            bytes.push(u8::from_str_radix(text, 16).map_err(|_| NameError::Escape)?);
            rest = after.get(2..).unwrap_or_default();
        } else {
            bytes.push(byte);
            rest = after;
        }
    }
    String::from_utf8(bytes).map_err(|_| NameError::Escape)
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
