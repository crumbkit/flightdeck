//! `flightdeck self environment list`, which crumbkit-starmap gives
//! flightdeck: it prints the variables that flightdeck's starmap.toml
//! declares and those that crumbkit reads, with their rules as data.

#![cfg(test)]
#![expect(
    clippy::disallowed_methods,
    reason = "the test starts the flightdeck binary"
)]

use std::process::{Command, Stdio};

use serde_json::Value;

#[test]
fn self_environment_list_prints_flightdecks_variables_with_their_rules_as_json() {
    let state = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_flightdeck"))
        .args(["self", "environment", "list", "--json", "--full"])
        .env_clear()
        .env("PATH", "/flightdeck/bin")
        .env("XDG_STATE_HOME", state.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema"], "flightdeck/self-environment-list/v1");
    let rows = document["data"].as_array().unwrap();
    let path = rows.iter().find(|row| row["name"] == "PATH").unwrap();
    assert_eq!(path["owner"], Value::Null);
    assert_eq!(path["state"], "set");
    assert_eq!(path["source_kind"]["kind"], "inherited");
    assert_eq!(path["type_name"], "path");
    assert_eq!(path["unset_rule"]["local"]["rule"], "optional");
    assert_eq!(path["value"]["text"], "/flightdeck/bin");
    let own: Vec<&str> = rows
        .iter()
        .filter(|row| row["owner"].is_null())
        .map(|row| row["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        own,
        [
            "PATH",
            "FLIGHTDECK_PORT",
            "FLIGHTDECK_MXV",
            "FLIGHTDECK_BOARD_PROGRAM",
            "FLIGHTDECK_BOARD",
            "BOARD_BIN",
            "XDG_DATA_HOME",
            "HOME",
        ]
    );
    assert!(
        rows.iter()
            .any(|row| row["name"] == "FLIGHTDECK_OUTPUT_BUDGET" && row["owner"] == "crumbkit"),
        "crumbkit's own variables follow flightdeck's: {document}"
    );
}
