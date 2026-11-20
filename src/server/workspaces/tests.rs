use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::{Workspaces, decoded};
use crate::server::reply::Reply;
use crate::spawn::Program;
use crate::workspaces::{Mxv, NameError};

/// Returns the path of the fake mxv in `tests/fixtures/mxv`.
fn fake_mxv() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mxv/mxv")
}

/// Returns the routes of the fake mxv that waits `sleep` seconds before it
/// answers and logs each run to `log`.
fn slow_routes(sleep: &str, log: &Path) -> Workspaces {
    Workspaces::new(Mxv::new(
        Program::new(fake_mxv())
            .env("FAKE_MXV_SLEEP", sleep)
            .env("FAKE_MXV_LOG", log),
    ))
}

/// Returns the number of runs of `mxv list` in the log at `log`.
fn lists_in(log: &Path) -> usize {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.starts_with("list "))
        .count()
}

#[test]
fn requests_that_arrive_during_a_list_share_its_answer() {
    let directory = tempfile::tempdir().unwrap();
    let log = directory.path().join("runs");
    let routes = slow_routes("0.5", &log);

    let replies: Vec<Reply> = std::thread::scope(|scope| {
        let first = scope.spawn(|| routes.list());
        let second = scope.spawn(|| routes.list());
        vec![first.join().unwrap(), second.join().unwrap()]
    });

    assert_eq!(lists_in(&log), 1);
    for reply in replies {
        assert!(matches!(reply, Reply::Json(_)), "{reply:?}");
    }
}

#[test]
fn a_list_after_one_has_ended_runs_mxv_again() {
    let directory = tempfile::tempdir().unwrap();
    let log = directory.path().join("runs");
    let routes = slow_routes("0", &log);

    routes.list();
    routes.list();

    assert_eq!(lists_in(&log), 2);
}

#[test]
fn a_refused_name_gets_422_and_runs_no_mxv() {
    let directory = tempfile::tempdir().unwrap();
    let log = directory.path().join("runs");
    let routes = slow_routes("0", &log);

    let reply = routes.show("-rf");

    let Reply::Failure { status, document } = reply else {
        panic!("{reply:?}");
    };
    assert_eq!(status, 422);
    let document: serde_json::Value = serde_json::from_slice(&document).unwrap();
    assert_eq!(document["code"], "flightdeck::workspaces::name");
    assert!(!log.exists());
}

#[test]
fn the_stop_ends_a_query_in_progress_at_once() {
    let directory = tempfile::tempdir().unwrap();
    let routes = slow_routes("30", &directory.path().join("runs"));
    let started = Instant::now();

    let reply = std::thread::scope(|scope| {
        let listing = scope.spawn(|| routes.list());
        while !directory.path().join("runs").exists() {
            #[expect(
                clippy::disallowed_methods,
                reason = "the test waits for a child that it cannot observe otherwise"
            )]
            std::thread::sleep(Duration::from_millis(5));
        }
        routes.stop();
        listing.join().unwrap()
    });

    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(
        matches!(reply, Reply::Failure { status: 502, .. }),
        "{reply:?}"
    );
}

/// Returns the status of `reply`, 200 for JSON.
fn status_of(reply: &Reply) -> u16 {
    match reply {
        Reply::Json(_) => 200,
        Reply::Failure { status, .. } => *status,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_name_that_the_page_escaped_is_decoded_before_it_is_checked() {
    let directory = tempfile::tempdir().unwrap();
    let routes = slow_routes("0", &directory.path().join("runs"));

    assert_eq!(status_of(&routes.show("reading%2Dlist")), 200);
    assert_eq!(status_of(&routes.show("reading%2Flist")), 422);
    assert_eq!(status_of(&routes.show("reading%2")), 422);
    assert_eq!(status_of(&routes.show("%FF")), 422);
}

#[test]
fn a_name_of_dots_gets_422_and_runs_no_mxv() {
    let directory = tempfile::tempdir().unwrap();
    let log = directory.path().join("runs");
    let routes = slow_routes("0", &log);

    assert_eq!(status_of(&routes.show("..")), 422);
    assert_eq!(status_of(&routes.show("%2E")), 422);
    assert!(!log.exists());
}

#[test]
fn a_slow_status_of_one_workspace_does_not_hold_up_another() {
    let directory = tempfile::tempdir().unwrap();
    let routes = slow_routes("1", &directory.path().join("runs"));
    let started = Instant::now();

    let replies: Vec<Reply> = std::thread::scope(|scope| {
        let first = scope.spawn(|| routes.show("reading-list"));
        let second = scope.spawn(|| routes.show("shelf-import"));
        vec![first.join().unwrap(), second.join().unwrap()]
    });

    // One after the other, the two would take at least 2 seconds.
    assert!(started.elapsed() < Duration::from_millis(1900));
    for reply in &replies {
        assert_eq!(status_of(reply), 200, "{reply:?}");
    }
    assert!(routes.show.lock().unwrap().is_empty());
}

#[test]
fn an_escaped_name_that_the_rules_refuse_gets_422_and_runs_no_mxv() {
    let directory = tempfile::tempdir().unwrap();
    let log = directory.path().join("runs");
    let routes = slow_routes("0", &log);

    // An escaped dash, a NUL, an overlong encoding of `/`, and an escape
    // whose digits carry a sign.
    for escaped in ["%2Drf", "%00", "%C0%AF", "%+1", "%+1x", "%-1"] {
        assert_eq!(status_of(&routes.show(escaped)), 422, "{escaped}");
    }
    assert!(!log.exists());
}

#[test]
fn a_shared_show_leaves_no_flight_behind() {
    let directory = tempfile::tempdir().unwrap();
    let routes = slow_routes("0.3", &directory.path().join("runs"));

    std::thread::scope(|scope| {
        let first = scope.spawn(|| routes.show("reading-list"));
        let second = scope.spawn(|| routes.show("reading-list"));
        first.join().unwrap();
        second.join().unwrap();
    });

    assert!(routes.show.lock().unwrap().is_empty());
}

#[test]
fn an_escape_takes_exactly_two_hex_digits_and_no_sign() {
    assert_eq!(decoded("reading%2dlist").unwrap(), "reading-list");
    assert_eq!(decoded("%41%42").unwrap(), "AB");
    for escaped in ["%+1", "%-1", "%1", "%1g", "% 1", "%\u{e9}"] {
        assert_eq!(decoded(escaped), Err(NameError::Escape), "{escaped}");
    }
}
