//! The runner's weight cell, which keeps the measurement of the tool's help
//! pages for the server's life once a measurement is complete.
//!
//! The cell is empty, measuring or kept:
//!
//! | State | Held | Leaves on |
//! | --- | --- | --- |
//! | Empty | no weight, or an incomplete one with its generation | a request takes `measuring` and finds the generation unchanged |
//! | Measuring | one request holds `measuring` and measures; the others wait on it | the measurement returns: complete is kept, incomplete is empty with the next generation |
//! | Kept | a complete weight | never, until the server stops |
//!
//! At most one measurement runs at a time, because `measuring` is held
//! across it. A request that waited while a measurement ran gets that
//! measurement's weight, complete or not, so concurrent requests never
//! multiply help runs. It knows a measurement ended while it waited because
//! the generation moved between its first look and its turn on `measuring`.

use std::sync::{Arc, Mutex};

use crate::lock::lock;
use crate::model::weight::Weight;

#[cfg(test)]
mod tests;

/// The last measurement and the generation that counts the measurements
/// that ended.
#[derive(Debug, Default)]
struct Last {
    generation: u64,
    weight: Option<Arc<Weight>>,
}

/// The measurement of the tool's help pages, kept only once it is complete.
#[derive(Debug, Default)]
pub(super) struct WeightCell {
    last: Mutex<Last>,
    measuring: Mutex<()>,
}

impl WeightCell {
    /// Returns the kept weight, or the weight of a measurement that ended
    /// while this call waited for it, or else the weight that `measure`
    /// returns now, which the cell keeps when it is complete.
    pub(super) fn get(&self, measure: impl FnOnce() -> Weight) -> Arc<Weight> {
        let seen = {
            let last = lock(&self.last);
            if let Some(weight) = last.weight.as_ref().filter(|weight| weight.is_complete()) {
                return Arc::clone(weight);
            }
            last.generation
        };
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
}
