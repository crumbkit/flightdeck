//! `flightdeck workspaces list` and `flightdeck workspaces show` against the
//! fake mxv in `tests/fixtures/mxv`. The text and JSON each prints are held
//! in golden files under `tests/golden/workspaces/`, which
//! `SNAPSHOTS=overwrite cargo test` rewrites.

#![cfg(test)]
#![cfg(unix)]
#![expect(
    clippy::disallowed_methods,
    reason = "the test starts the flightdeck binary"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// Returns the path of the fake mxv.
fn fake_mxv() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mxv/mxv")
}

/// Runs flightdeck with `args`, `FLIGHTDECK_MXV` naming `mxv` and the
/// variables `vars`, and returns what it printed.
fn flightdeck_with(mxv: &Path, args: &[&str], vars: &[(&str, &str)]) -> Output {
    let state = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_flightdeck"));
    command
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("XDG_STATE_HOME", state.path())
        .env("FLIGHTDECK_MXV", mxv)
        .stdin(Stdio::null());
    for (name, value) in vars {
        command.env(name, value);
    }
    command.output().unwrap()
}

/// Runs flightdeck with `args` against the fake mxv.
fn flightdeck(args: &[&str]) -> Output {
    flightdeck_with(&fake_mxv(), args, &[])
}

/// Compares `actual` with the golden file `name`, or rewrites the file when
/// `SNAPSHOTS=overwrite` is set.
fn golden(name: &str, actual: &[u8]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/workspaces")
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

/// Returns the exit code and fails the test with standard error when it
/// is not `expected`.
fn assert_exit(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn workspaces_list_prints_a_table_of_every_workspace() {
    let output = flightdeck(&["workspaces", "list"]);

    assert_exit(&output, 0);
    golden("list.txt", &output.stdout);
}

#[test]
fn workspaces_list_json_full_prints_the_workspaces_pages_document() {
    let output = flightdeck(&["workspaces", "list", "--json", "--full"]);

    assert_exit(&output, 0);
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema"], "flightdeck/workspaces-list/v1");
    assert_eq!(document["summary"]["running"], 1);
    golden("list.json", &output.stdout);
}

#[test]
fn workspaces_show_json_prints_the_one_workspace_pages_document() {
    let output = flightdeck(&["workspaces", "show", "reading-list", "--json"]);

    assert_exit(&output, 0);
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema"], "flightdeck/workspaces-show/v1");
    assert_eq!(document["data"]["name"], "reading-list");
    golden("show.json", &output.stdout);
}

#[test]
fn workspaces_show_prints_the_checkouts_and_units_as_text() {
    let output = flightdeck(&["workspaces", "show", "reading-list"]);

    assert_exit(&output, 0);
    golden("show.txt", &output.stdout);
}

#[test]
fn workspaces_show_of_an_unknown_name_fails_with_not_found_and_names_the_list() {
    let output = flightdeck(&["workspaces", "show", "no-such-workspace", "--json"]);

    assert_exit(&output, 1);
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::workspaces::not_found");
    assert_eq!(failure["recovery"]["call"], "flightdeck workspaces list");
}

#[test]
fn workspaces_show_refuses_a_name_that_starts_with_a_dash_before_mxv_runs() {
    let directory = tempfile::tempdir().unwrap();
    let log = directory.path().join("runs");

    let output = flightdeck_with(
        &fake_mxv(),
        &["workspaces", "show", "--", "-rf"],
        &[("FAKE_MXV_LOG", log.to_str().unwrap())],
    );

    assert_exit(&output, 2);
    assert!(!log.exists());
}

#[test]
fn a_missing_mxv_fails_with_a_hint_that_names_the_variable() {
    let output = flightdeck_with(
        Path::new("/no/such/mxv"),
        &["workspaces", "list", "--json"],
        &[],
    );

    assert_exit(&output, 1);
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::workspaces::mxv_missing");
    assert!(
        failure["recovery"]["text"]
            .as_str()
            .unwrap()
            .contains("FLIGHTDECK_MXV"),
        "{failure}"
    );
}

#[test]
fn a_failed_mxv_list_fails_with_mxvs_own_code() {
    let output = flightdeck_with(
        &fake_mxv(),
        &["workspaces", "list", "--json"],
        &[("FAKE_MXV_LIST", "fail")],
    );

    assert_exit(&output, 1);
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::workspaces::mxv_failed");
    assert_eq!(failure["fields"]["mxv_code"], "mxv::store::unreadable");
}

#[test]
fn an_mxv_without_proxy_status_still_lists_and_says_why_routes_are_missing() {
    let output = flightdeck_with(
        &fake_mxv(),
        &["workspaces", "list", "--json", "--full"],
        &[("FAKE_MXV_PROXY", "refuse")],
    );

    assert_exit(&output, 0);
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["summary"]["routes"]["state"], "unavailable");
}
