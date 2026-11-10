//! The runner's weight cell, which keeps the latest measurement of the
//! tool's help pages and measures again only when the measurement failed or
//! the request asks.
//!
//! | Last measurement | A plain request | A request to measure again |
//! | --- | --- | --- |
//! | None | measures | measures |
//! | Failed: it has a problem | measures | measures |
//! | Partial: a page says `not_run` | gets it | measures |
//! | Complete | gets it | gets it |
//!
//! A complete measurement is never replaced. A partial one, which a tool
//! whose help always passes the 30-second limit gives every time, is served
//! to every plain request, so a page load never starts another measurement
//! of such a tool; only the page's Measure again button asks for one.
//!
//! At most one measurement runs at a time, because `measuring` is held
//! across it. A request that waited while a measurement ran gets that
//! measurement's weight, whatever it is, so concurrent requests never
//! multiply help runs. It knows a measurement ended while it waited because
//! the generation moved between its first look and its turn on `measuring`.

use std::sync::{Arc, Mutex};

use crate::lock::lock;
use crate::model::weight::Weight;

#[cfg(test)]
mod tests;

/// What a request asks of the cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ask {
    /// The latest measurement, measuring only when there is none or it
    /// failed.
    Kept,
    /// A new measurement, unless the latest one is complete.
    Again,
}

/// The last measurement and the generation that counts the measurements
/// that ended.
#[derive(Debug, Default)]
struct Last {
    generation: u64,
    weight: Option<Arc<Weight>>,
}

impl Last {
    /// Returns the weight that serves `ask` without a new measurement.
    fn serves(&self, ask: Ask) -> Option<&Arc<Weight>> {
        self.weight.as_ref().filter(|weight| {
            weight.is_complete() || (ask == Ask::Kept && weight.problem().is_none())
        })
    }
}

/// The latest measurement of the tool's help pages.
#[derive(Debug, Default)]
pub(super) struct WeightCell {
    last: Mutex<Last>,
    measuring: Mutex<()>,
    /// How many calls have looked at the cell and are on their way to
    /// measure or wait, which a test reads to know a second caller waits.
    #[cfg(test)]
    arrived: std::sync::atomic::AtomicUsize,
}

impl WeightCell {
    /// Returns the latest weight when it serves `ask`, or the weight of a
    /// measurement that ended while this call waited for it, or else the
    /// weight that `measure` returns now, which becomes the latest.
    pub(super) fn get(&self, ask: Ask, measure: impl FnOnce() -> Weight) -> Arc<Weight> {
        let seen = {
            let last = lock(&self.last);
            if let Some(weight) = last.serves(ask) {
                return Arc::clone(weight);
            }
            last.generation
        };
        #[cfg(test)]
        self.arrived
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let _measuring = lock(&self.measuring);
        {
            let last = lock(&self.last);
            if last.generation != seen
                && let Some(weight) = &last.weight
            {
                return Arc::clone(weight);
            }
        }
        let weight = Arc::new(measure());
        let mut last = lock(&self.last);
        last.generation += 1;
        last.weight = Some(Arc::clone(&weight));
        weight
    }

    /// Returns how many calls have looked at the cell and gone on to
    /// measure or wait.
    #[cfg(test)]
    fn arrived(&self) -> usize {
        self.arrived.load(std::sync::atomic::Ordering::SeqCst)
    }
}
