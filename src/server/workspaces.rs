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
        let held = Held::new(&self.show, name);
        let answered = held.flight().get((), || {
            answer(
                workspaces::show(&self.mxv, held.name(), &self.stop).map(|shown| shown.to_json()),
            )
        });
        drop(held);
        reply(&answered)
    }

    /// Cancels the queries in progress and every later one, as the server
    /// stops.
    pub(super) fn stop(&self) {
        self.stop.cancel();
    }
}

/// One request's hold on the show flight of a name. The map holds a name's
/// flight only while a request holds it, so names asked once do not pile
/// up: when the last hold drops, even in a panic, it removes the flight.
struct Held<'a> {
    flights: &'a Mutex<HashMap<WorkspaceName, Arc<Flight<()>>>>,
    name: WorkspaceName,
    flight: Arc<Flight<()>>,
}

impl<'a> Held<'a> {
    /// Holds the flight of `name` in `flights`, making it when none runs.
    fn new(
        flights: &'a Mutex<HashMap<WorkspaceName, Arc<Flight<()>>>>,
        name: WorkspaceName,
    ) -> Self {
        let flight = Arc::clone(lock(flights).entry(name.clone()).or_default());
        Self {
            flights,
            name,
            flight,
        }
    }

    /// Returns the held flight.
    fn flight(&self) -> &Flight<()> {
        &self.flight
    }

    /// Returns the name whose flight this holds.
    const fn name(&self) -> &WorkspaceName {
        &self.name
    }
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        // Each hold lets go of its flight under the map's lock, so the
        // last one sees only the map's own reference and removes it.
        let mut flights = lock(self.flights);
        // An empty flight takes the held one's place, so the held one is
        // dropped here, while the lock is held.
        drop(std::mem::take(&mut self.flight));
        if flights
            .get(&self.name)
            .is_some_and(|flight| Arc::strong_count(flight) == 1)
        {
            flights.remove(&self.name);
        }
    }
}

/// Returns the value of the ASCII hex digit `byte`, or
/// [`NameError::Escape`] for any other byte, a sign included.
fn hex_digit(byte: u8) -> Result<u8, NameError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(NameError::Escape),
    }
}

/// Returns `escaped` with each `%XX`, two ASCII hex digits, replaced by its
/// byte, or [`NameError::Escape`] when an escape is not two hex digits or
/// the bytes are not UTF-8.
fn decoded(escaped: &str) -> Result<String, NameError> {
    let mut bytes = Vec::with_capacity(escaped.len());
    let mut rest = escaped.as_bytes();
    while let Some((&byte, after)) = rest.split_first() {
        if byte == b'%' {
            let (&[high, low], after) = after.split_first_chunk::<2>().ok_or(NameError::Escape)?;
            bytes.push(hex_digit(high)? << 4 | hex_digit(low)?);
            rest = after;
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
