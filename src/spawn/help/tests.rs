use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crumbkit_process::{Cancel, Outcome, Status, StopReason};
use tempfile::TempDir;

use super::{HELP_LIMIT, HelpLimits, HelpPage, measure_help, page_of};
use crate::spawn::{Exit, Program};

/// Writes a shell script that runs `body` for `-h` of any page, and returns
/// its directory and path.
fn stand_in(body: &str) -> (TempDir, PathBuf) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("stand-in");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (directory, path)
}

/// Returns `-h` of the one page `words` of the stand-in that runs `body`.
fn measured(body: &str, limits: HelpLimits) -> HelpPage {
    let (_directory, path) = stand_in(body);
    let pages = [vec!["things".to_owned()]];
    measure_help(&Program::new(path), &pages, limits, &Cancel::new()).unwrap()[0]
}

#[test]
fn a_page_that_exits_with_a_code_other_than_0_gives_the_exit() {
    let page = measured("echo usage; exit 3", HelpLimits::new());

    assert_eq!(page, HelpPage::Exit(Exit::Code(3)));
}

#[test]
fn a_page_that_passes_its_time_limit_is_stopped_and_timed_out() {
    let limits = HelpLimits::new().run(Duration::from_millis(200));

    let page = measured("exec sleep 5", limits);

    assert_eq!(page, HelpPage::TimedOut);
}

/// Returns the page of a stand-in that prints `bytes` bytes and exits with 0.
fn printing(bytes: usize) -> HelpPage {
    measured(&format!("head -c {bytes} /dev/zero"), HelpLimits::new())
}

#[test]
fn a_page_far_over_64_kib_is_too_large() {
    assert_eq!(printing(HELP_LIMIT * 4), HelpPage::TooLarge);
}

#[test]
fn a_page_one_byte_over_64_kib_is_too_large_whether_the_child_exits_or_is_stopped_first() {
    // The child may exit before flightdeck stops it, or be stopped first,
    // depending on the system's pipe size and the scheduler. Each run tries
    // that race again, and every outcome must say the page is too large.
    for _ in 0..20 {
        assert_eq!(printing(HELP_LIMIT + 1), HelpPage::TooLarge);
    }
}

#[test]
fn a_page_of_exactly_64_kib_is_measured() {
    let bytes = u64::try_from(HELP_LIMIT).unwrap();

    assert_eq!(printing(HELP_LIMIT), HelpPage::Bytes(bytes));
}

#[test]
fn a_page_that_prints_nothing_is_empty() {
    assert_eq!(measured("exit 0", HelpLimits::new()), HelpPage::Empty);
}

#[test]
fn each_run_gets_only_the_programs_variables_and_a_fresh_state_directory() {
    let marks = tempfile::tempdir().unwrap();
    let mark = marks.path().join("env");
    let (_directory, path) = stand_in(&format!("env > '{}'; echo page", mark.display()));
    let program = Program::new(path)
        .env("ADDED", "yes")
        .env("XDG_STATE_HOME", "/the-persons-state");

    let pages = measure_help(&program, &[Vec::new()], HelpLimits::new(), &Cancel::new()).unwrap();

    assert_eq!(pages, [HelpPage::Bytes(5)]);
    let env = std::fs::read_to_string(&mark).unwrap();
    let state = env
        .lines()
        .find_map(|line| line.strip_prefix("XDG_STATE_HOME="))
        .unwrap();
    assert_ne!(state, "/the-persons-state");
    assert!(env.lines().any(|line| line == "ADDED=yes"), "{env}");
    assert!(!env.lines().any(|line| line.starts_with("HOME=")), "{env}");
}

#[test]
fn no_run_starts_once_the_measurement_has_passed_its_limit() {
    let (_directory, path) = stand_in("echo page");
    let pages: Vec<Vec<String>> = (0..8).map(|n| vec![n.to_string()]).collect();
    let limits = HelpLimits::new().total(Duration::ZERO);

    let measured = measure_help(&Program::new(path), &pages, limits, &Cancel::new()).unwrap();

    assert!(
        measured.iter().all(|page| *page == HelpPage::NotRun),
        "{measured:?}"
    );
}

#[test]
fn a_stopped_measurement_starts_no_run() {
    let marks = tempfile::tempdir().unwrap();
    let mark = marks.path().join("ran");
    let (_directory, path) = stand_in(&format!("touch '{}'; echo page", mark.display()));
    let pages: Vec<Vec<String>> = (0..8).map(|n| vec![n.to_string()]).collect();
    let stop = Cancel::new();
    stop.cancel();

    let measured = measure_help(&Program::new(path), &pages, HelpLimits::new(), &stop).unwrap();

    assert!(
        measured.iter().all(|page| *page == HelpPage::NotRun),
        "{measured:?}"
    );
    assert!(!mark.exists(), "a run started after the stop");
}

#[test]
fn a_stop_during_a_measurement_ends_the_runs_in_progress_at_once() {
    let (_directory, path) = stand_in("exec sleep 30");
    let pages: Vec<Vec<String>> = (0..8).map(|n| vec![n.to_string()]).collect();
    let limits = HelpLimits::new().run(Duration::from_secs(30));
    let stop = Cancel::new();
    let started = Instant::now();

    let measured = std::thread::scope(|scope| {
        let measuring = scope.spawn(|| measure_help(&Program::new(&path), &pages, limits, &stop));
        #[expect(
            clippy::disallowed_methods,
            reason = "the test lets the first runs start before it stops them"
        )]
        std::thread::sleep(Duration::from_millis(200));
        stop.cancel();
        measuring.join().unwrap().unwrap()
    });

    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the measurement took {:?} after its stop",
        started.elapsed()
    );
    assert!(
        measured.iter().all(|page| *page == HelpPage::NotRun),
        "{measured:?}"
    );
}

#[test]
fn a_page_whose_path_has_a_word_that_starts_with_a_dash_is_refused_without_a_run() {
    let marks = tempfile::tempdir().unwrap();
    let mark = marks.path().join("ran");
    let (_directory, path) = stand_in(&format!("touch '{}'; echo page", mark.display()));
    let pages = [vec!["books".to_owned(), "--delete-all".to_owned()]];

    let measured = measure_help(
        &Program::new(path),
        &pages,
        HelpLimits::new(),
        &Cancel::new(),
    )
    .unwrap();

    assert_eq!(measured, [HelpPage::Refused]);
    assert!(!mark.exists(), "the page ran");
}

#[test]
fn output_over_the_limit_is_too_large_even_when_the_child_exited_before_it_was_stopped() {
    // On Linux a child can print 65,537 bytes and exit before flightdeck
    // stops it, so the run reports an ordinary exit with the first 64 KiB.
    let ended = Outcome::Ended(Status::Code(0));

    assert_eq!(
        page_of(ended, Exit::Code(0), HELP_LIMIT, true),
        HelpPage::TooLarge
    );
    assert_eq!(
        page_of(ended, Exit::Code(3), HELP_LIMIT, true),
        HelpPage::TooLarge
    );
    assert_eq!(
        page_of(
            Outcome::Stopped(StopReason::OverLimit),
            Exit::Signal(9),
            HELP_LIMIT,
            true
        ),
        HelpPage::TooLarge
    );
    assert_eq!(
        page_of(ended, Exit::Code(0), HELP_LIMIT, false),
        HelpPage::Bytes(u64::try_from(HELP_LIMIT).unwrap())
    );
}
