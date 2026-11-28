//! `flightdeck plugin install` and `flightdeck plugin call`, run as Board
//! runs them: install with XDG directories of the test's own, and call with
//! one board/call/v1 on standard input and the fake board as `BOARD_BIN`.

#![cfg(test)]
#![cfg(unix)]
#![expect(
    clippy::disallowed_methods,
    reason = "the test starts the flightdeck binary"
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

/// Returns the path of the fake board.
fn fake_board() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/board/fake/board")
}

/// Runs flightdeck with `args`, the variables `vars` and `input` on
/// standard input, and returns what it printed.
fn flightdeck(args: &[&str], vars: &[(&str, &Path)], input: &[u8]) -> Output {
    let state = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_flightdeck"));
    command
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("XDG_STATE_HOME", state.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in vars {
        command.env(name, value);
    }
    let mut child = command.spawn().unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn plugin_install_writes_the_plugin_under_xdg_data_home_and_says_how_to_enable_it() {
    let data = tempfile::tempdir().unwrap();

    let output = flightdeck(
        &["plugin", "install", "--json"],
        &[("XDG_DATA_HOME", data.path())],
        b"",
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema"], "flightdeck/plugin-install/v1");
    assert_eq!(document["data"]["replaced"], false);
    let plugin = data.path().join("board/plugins/flightdeck");
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(plugin.join("plugin.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["name"], "flightdeck");
    assert!(
        plugin
            .join("types/flightdeck/workflow-graph/v1.schema.json")
            .is_file()
    );
    let text = flightdeck(
        &["plugin", "install"],
        &[("XDG_DATA_HOME", data.path())],
        b"",
    );
    assert!(
        String::from_utf8_lossy(&text.stdout).contains("board plugin enable flightdeck"),
        "{}",
        String::from_utf8_lossy(&text.stdout)
    );
}

#[test]
fn plugin_call_answers_one_request_on_standard_input_with_one_document() {
    let request = json!({
        "$type": "board/call/v1",
        "capability": "check",
        "input": {"path": "/workflow-graphs/tangled"},
        "board": {"root": "/boards/home"},
    });

    let output = flightdeck(
        &["plugin", "call"],
        &[("BOARD_BIN", &fake_board())],
        request.to_string().as_bytes(),
    );
    let refused = flightdeck(&["plugin", "call"], &[("BOARD_BIN", &fake_board())], b"{}");

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(answer["$type"], "board/call-result/v1");
    assert_eq!(answer["output"]["problems"].as_array().unwrap().len(), 1);
    assert_eq!(refused.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(error["$type"], "board/error/v1");
    assert_eq!(error["code"], "flightdeck::check::bad_input");
}

#[test]
fn plugin_install_with_no_data_directory_fails_with_no_data_home() {
    let output = flightdeck(&["plugin", "install", "--json"], &[], b"");

    assert_eq!(output.status.code(), Some(1));
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::plugin::no_data_home");
}
