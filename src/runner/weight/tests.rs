use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::{Ask, WeightCell};
use crate::model::weight::Weight;
use crate::spawn::HelpPage;

/// Returns a measurement whose one page has `page`.
fn measured(page: HelpPage) -> Weight {
    Weight::of_pages(vec![(Vec::new(), page)], None)
}

/// Returns a complete measurement.
fn complete() -> Weight {
    measured(HelpPage::Bytes(5))
}

/// Returns a measurement that stopped before its page ran.
fn partial() -> Weight {
    measured(HelpPage::NotRun)
}

/// Returns a measurement that could not start.
fn failed() -> Weight {
    Weight::of_pages(Vec::new(), Some("cannot make a directory"))
}

#[test]
fn a_measurement_is_complete_only_without_a_problem_or_a_page_that_did_not_run() {
    assert!(complete().is_complete());
    assert!(measured(HelpPage::TimedOut).is_complete());
    assert!(!partial().is_complete());
    assert!(!failed().is_complete());
}

#[test]
fn a_complete_measurement_is_kept_and_never_measured_again_even_when_asked() {
    let cell = WeightCell::default();
    let first = cell.get(Ask::Kept, complete);

    let later = cell.get(Ask::Again, partial);

    assert!(Arc::ptr_eq(&first, &later));
}

#[test]
fn a_partial_measurement_is_kept_for_a_plain_request_and_measured_again_when_asked() {
    let cell = WeightCell::default();
    let first = cell.get(Ask::Kept, partial);

    let plain = cell.get(Ask::Kept, complete);
    let again = cell.get(Ask::Again, complete);

    assert!(Arc::ptr_eq(&first, &plain));
    assert!(again.is_complete());
}

#[test]
fn a_failed_measurement_is_measured_again_by_the_next_plain_request() {
    let cell = WeightCell::default();
    cell.get(Ask::Kept, failed);

    let next = cell.get(Ask::Kept, complete);

    assert!(next.is_complete());
}

#[test]
fn a_caller_that_waited_on_a_measurement_gets_its_result_even_when_it_failed_and_measures_nothing()
{
    let cell = WeightCell::default();
    let measures = AtomicUsize::new(0);
    // The first measurement lasts until the second caller has looked at the
    // cell and is on its way to wait, so the second caller always waits.
    let measure = || {
        measures.fetch_add(1, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(10);
        while cell.arrived() < 2 && Instant::now() < deadline {
            std::thread::yield_now();
        }
        failed()
    };

    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| cell.get(Ask::Kept, measure));
        while cell.arrived() < 1 {
            std::thread::yield_now();
        }
        let second = scope.spawn(|| cell.get(Ask::Kept, measure));
        (first.join().unwrap(), second.join().unwrap())
    });

    assert_eq!(cell.arrived(), 2);
    assert_eq!(measures.load(Ordering::SeqCst), 1);
    assert!(Arc::ptr_eq(&first, &second));
}
