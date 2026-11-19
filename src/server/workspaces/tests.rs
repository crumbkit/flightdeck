use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::Workspaces;
use crate::server::reply::Reply;
use crate::spawn::Program;
use crate::workspaces::Mxv;

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
