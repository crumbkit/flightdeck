use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crumbkit_process::Cancel;
use tempfile::TempDir;

use super::{MxvLimits, MxvQuery, run_all};
use crate::spawn::{Exit, Program, SpawnError, Stdout};
use crate::workspaces::WorkspaceName;

/// Writes a shell script named `mxv` that runs `body`, and returns its
/// directory and path.
fn fake_mxv(body: &str) -> (TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("mxv");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (directory, path)
}

/// Returns the name `text`.
fn name(text: &str) -> WorkspaceName {
    WorkspaceName::new(text).unwrap()
}

#[test]
fn each_query_has_a_fixed_argument_vector_with_the_name_inside_one_word() {
    assert_eq!(
        MxvQuery::List.words(),
        ["list", "--all", "--json", "--full", "--no-input"]
    );
    assert_eq!(
        MxvQuery::Status(name("reading-list")).words(),
        [
            "status",
            "--workspace=reading-list",
            "--json",
            "--full",
            "--no-input"
        ]
    );
    assert_eq!(
        MxvQuery::ProxyStatus.words(),
        ["proxy", "status", "--json", "--full", "--no-input"]
    );
}

#[test]
fn every_query_runs_and_its_result_keeps_the_order_of_the_queries() {
    let (_directory, mxv) = fake_mxv(r#"printf '%s|' "$@""#);
    let queries = [
        MxvQuery::Status(name("a")),
        MxvQuery::List,
        MxvQuery::Status(name("b")),
        MxvQuery::ProxyStatus,
        MxvQuery::Status(name("c")),
    ];

    let finished = run_all(
        &Program::new(&mxv),
        &queries,
        MxvLimits::new().start(),
        &Cancel::new(),
    );

    let printed: Vec<String> = finished
        .into_iter()
        .map(|each| match each.unwrap().stdout() {
            Stdout::Whole(bytes) => String::from_utf8(bytes.clone()).unwrap(),
            Stdout::OverLimit => panic!("over the limit"),
        })
        .collect();
    assert_eq!(
        printed,
        [
            "status|--workspace=a|--json|--full|--no-input|",
            "list|--all|--json|--full|--no-input|",
            "status|--workspace=b|--json|--full|--no-input|",
            "proxy|status|--json|--full|--no-input|",
            "status|--workspace=c|--json|--full|--no-input|",
        ]
    );
}

#[test]
fn mxv_records_the_runs_as_flightdecks() {
    let (_directory, mxv) = fake_mxv(r#"printf '%s' "$MXV_PROGRAM""#);

    let finished = run_all(
        &Program::new(&mxv),
        &[MxvQuery::List],
        MxvLimits::new().start(),
        &Cancel::new(),
    );

    let [Ok(only)] = finished.as_slice() else {
        panic!("one run");
    };
    assert_eq!(only.stdout(), &Stdout::Whole(b"flightdeck".to_vec()));
}

#[test]
fn a_query_that_outlasts_the_time_limit_is_stopped_and_reported_as_timed_out() {
    let (_directory, mxv) = fake_mxv("exec sleep 30");
    let started = Instant::now();

    let finished = run_all(
        &Program::new(&mxv),
        &[MxvQuery::List, MxvQuery::ProxyStatus],
        MxvLimits::new().time(Duration::from_millis(200)).start(),
        &Cancel::new(),
    );

    assert!(started.elapsed() < Duration::from_secs(5));
    for each in finished {
        assert_eq!(each.unwrap().exit(), Exit::TimedOut);
    }
}

#[test]
fn output_past_the_limit_is_dropped_and_reported() {
    let (_directory, mxv) = fake_mxv("head -c 4096 /dev/zero");

    let finished = run_all(
        &Program::new(&mxv),
        &[MxvQuery::List],
        MxvLimits::new().stdout(1024).start(),
        &Cancel::new(),
    );

    let [Ok(only)] = finished.as_slice() else {
        panic!("one run");
    };
    assert_eq!(only.stdout(), &Stdout::OverLimit);
}

#[test]
fn a_cancel_stops_the_running_queries_and_starts_no_more() {
    let (directory, mxv) = fake_mxv(r#"echo run >> "$0.runs"; exec sleep 30"#);
    let stop = Cancel::new();
    let queries: Vec<MxvQuery> = (0..12)
        .map(|index| MxvQuery::Status(name(&format!("w{index}"))))
        .collect();
    let started = Instant::now();

    let finished = std::thread::scope(|scope| {
        let running = scope.spawn(|| {
            run_all(
                &Program::new(&mxv),
                &queries,
                MxvLimits::new().start(),
                &stop,
            )
        });
        wait_for_runs(&directory.path().join("mxv.runs"));
        stop.cancel();
        running.join().unwrap()
    });

    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(finished.len(), 12);
    let runs = std::fs::read_to_string(directory.path().join("mxv.runs")).unwrap();
    assert!(runs.lines().count() < 12, "{runs}");
}

#[test]
fn a_missing_program_fails_to_start() {
    let finished = run_all(
        &Program::new("/no/such/mxv"),
        &[MxvQuery::List],
        MxvLimits::new().start(),
        &Cancel::new(),
    );

    let [Err(error)] = finished.as_slice() else {
        panic!("one failure");
    };
    assert!(matches!(error, SpawnError::Start(_)), "{error:?}");
}

/// Waits until the file at `path` exists.
fn wait_for_runs(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        assert!(Instant::now() < deadline, "no run started");
        #[expect(
            clippy::disallowed_methods,
            reason = "the test waits for a child that it cannot observe otherwise"
        )]
        std::thread::sleep(Duration::from_millis(5));
    }
}
