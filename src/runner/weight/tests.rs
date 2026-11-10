use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::WeightCell;
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

#[test]
fn a_measurement_is_complete_only_without_a_problem_or_a_page_that_did_not_run() {
    assert!(complete().is_complete());
    assert!(measured(HelpPage::TimedOut).is_complete());
    assert!(!partial().is_complete());
    assert!(!Weight::of_pages(Vec::new(), Some("cannot make a directory")).is_complete());
}

#[test]
fn a_complete_measurement_is_kept_and_never_measured_again() {
    let cell = WeightCell::default();
    let first = cell.get(complete);

    let later = cell.get(partial);

    assert!(std::sync::Arc::ptr_eq(&first, &later));
    assert!(later.is_complete());
}

#[test]
fn an_incomplete_measurement_is_measured_again_by_the_next_caller() {
    let cell = WeightCell::default();
    let first = cell.get(partial);

    let second = cell.get(complete);

    assert!(!first.is_complete());
    assert!(second.is_complete());
}

#[test]
fn a_caller_that_waited_on_a_measurement_gets_its_result_even_when_incomplete_and_measures_nothing()
{
    let cell = WeightCell::default();
    let measures = AtomicUsize::new(0);
    let measure = || {
        measures.fetch_add(1, Ordering::SeqCst);
        #[expect(
            clippy::disallowed_methods,
            reason = "the measurement lasts long enough for the second caller to wait on it"
        )]
        std::thread::sleep(Duration::from_millis(300));
        partial()
    };

    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(|| cell.get(measure));
        #[expect(
            clippy::disallowed_methods,
            reason = "the test lets the first caller start measuring before the second asks"
        )]
        std::thread::sleep(Duration::from_millis(100));
        let second = scope.spawn(|| cell.get(measure));
        (first.join().unwrap(), second.join().unwrap())
    });

    assert_eq!(measures.load(Ordering::SeqCst), 1);
    assert!(std::sync::Arc::ptr_eq(&first, &second));
    assert!(!second.is_complete());
}
