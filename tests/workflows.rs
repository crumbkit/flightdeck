//! `flightdeck workflows list` and `flightdeck workflows show` against the
//! fake board in `tests/fixtures/board/fake`. The text and JSON each prints
//! are held in golden files under `tests/golden/workflows/`, which
//! `SNAPSHOTS=overwrite cargo test` rewrites, and the page's routes send the
//! same JSON.

#![cfg(test)]
#![cfg(unix)]
#![expect(
    clippy::disallowed_methods,
    reason = "the test starts the flightdeck binary"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// Returns the path of the fake board.
fn fake_board() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/board/fake/board")
}

/// Runs flightdeck with `args`, `FLIGHTDECK_BOARD_PROGRAM` naming `board`
/// and the variables `vars`, and returns what it printed.
fn flightdeck_with(board: &Path, args: &[&str], vars: &[(&str, &str)]) -> Output {
    let state = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_flightdeck"));
    command
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("XDG_STATE_HOME", state.path())
        .env("FLIGHTDECK_BOARD_PROGRAM", board)
        .stdin(Stdio::null());
    for (name, value) in vars {
        command.env(name, value);
    }
    command.output().unwrap()
}

/// Runs flightdeck with `args` against the fake board.
fn flightdeck(args: &[&str]) -> Output {
    flightdeck_with(&fake_board(), args, &[])
}

/// Compares `actual` with the golden file `name`, or rewrites the file when
/// `SNAPSHOTS=overwrite` is set.
fn golden(name: &str, actual: &[u8]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/workflows")
        .join(name);
    if std::env::var_os("SNAPSHOTS").is_some_and(|value| value == "overwrite") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read(&path).unwrap_or_default();
    assert_eq!(
        String::from_utf8_lossy(actual),
        String::from_utf8_lossy(&expected),
        "{} differs; run SNAPSHOTS=overwrite cargo test to rewrite it",
        path.display()
    );
}

/// Fails the test with standard error when the exit code is not `expected`.
fn assert_exit(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn workflows_list_prints_each_graph_with_its_node_count_and_status() {
    let output = flightdeck(&["workflows", "list"]);

    assert_exit(&output, 0);
    golden("list.txt", &output.stdout);
}

#[test]
fn workflows_list_json_full_prints_the_document_the_rail_reads() {
    let output = flightdeck(&["workflows", "list", "--json", "--full"]);

    assert_exit(&output, 0);
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema"], "flightdeck/workflows-list/v1");
    assert_eq!(document["summary"]["graphs"], 5);
    golden("list.json", &output.stdout);
}

#[test]
fn workflows_show_json_prints_the_document_the_page_draws() {
    let output = flightdeck(&["workflows", "show", "development-flow", "--json"]);
    let tangled = flightdeck(&["workflows", "show", "tangled", "--json"]);

    assert_exit(&output, 0);
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema"], "flightdeck/workflows-show/v1");
    assert_eq!(document["data"]["graph"]["title"], "Development flow");
    golden("show-development-flow.json", &output.stdout);
    assert_exit(&tangled, 0);
    golden("show-tangled.json", &tangled.stdout);
}

#[test]
fn workflows_show_prints_the_graph_and_its_live_records_as_text() {
    let output = flightdeck(&["workflows", "show", "development-flow"]);
    let tangled = flightdeck(&["workflows", "show", "tangled"]);

    assert_exit(&output, 0);
    golden("show-development-flow.txt", &output.stdout);
    golden("show-tangled.txt", &tangled.stdout);
}

#[test]
fn workflows_show_of_an_unknown_slug_fails_with_not_found_and_names_the_list() {
    let output = flightdeck(&["workflows", "show", "no-such-flow", "--json"]);

    assert_exit(&output, 1);
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::workflows::not_found");
    assert_eq!(failure["recovery"]["call"], "flightdeck workflows list");
}

#[test]
fn workflows_show_refuses_a_slug_that_is_not_lowercase_words_before_board_runs() {
    let directory = tempfile::tempdir().unwrap();
    let log = directory.path().join("runs");

    let output = flightdeck_with(
        &fake_board(),
        &["workflows", "show", "Development"],
        &[("FAKE_BOARD_LOG", log.to_str().unwrap())],
    );

    assert_exit(&output, 2);
    assert!(!log.exists());
}

#[test]
fn a_missing_board_fails_with_not_found_and_a_hint_that_names_the_variable() {
    let output = flightdeck_with(
        Path::new("/no/such/board"),
        &["workflows", "list", "--json"],
        &[],
    );

    assert_exit(&output, 1);
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::board::not_found");
    assert!(
        failure["recovery"]["text"]
            .as_str()
            .unwrap()
            .contains("FLIGHTDECK_BOARD_PROGRAM"),
        "{failure}"
    );
}

#[test]
fn flightdeck_board_names_the_board_that_board_reads() {
    let directory = tempfile::tempdir().unwrap();
    let log = directory.path().join("runs");

    let output = flightdeck_with(
        &fake_board(),
        &["workflows", "list"],
        &[
            ("FAKE_BOARD_LOG", log.to_str().unwrap()),
            ("FLIGHTDECK_BOARD", "/boards/home"),
        ],
    );

    assert_exit(&output, 0);
    let runs = std::fs::read_to_string(&log).unwrap();
    assert!(runs.starts_with("-C /boards/home query "), "{runs}");
}
