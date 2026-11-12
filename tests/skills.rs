//! End-to-end tests of the `flightdeck skills` commands. Each test runs the
//! built binary with `HOME` set to a temporary directory of its own, so no
//! test installs into the real home directory.

#![cfg(test)]
// The tests start the flightdeck binary, which only src/spawn.rs does in the
// library.
#![expect(
    clippy::disallowed_methods,
    reason = "the tests start the flightdeck binary"
)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

/// Returns a temporary home with an empty `.agents/skills` directory.
fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    fs::create_dir_all(home.path().join(".agents").join("skills")).unwrap();
    home
}

/// Runs the built `flightdeck` binary with `args`, a cleared environment,
/// and `HOME` and the XDG directories under `home`.
fn flightdeck(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_flightdeck"))
        .args(args)
        .current_dir(home)
        .env_clear()
        .env("HOME", home)
        .env("XDG_STATE_HOME", home.join("state"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .output()
        .unwrap()
}

/// Returns the standard output of a run as a JSON value.
fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Returns the standard output of a run as text.
fn text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn install_writes_the_skill_with_metadata_naming_flightdeck() {
    let home = home();

    let output = flightdeck(home.path(), &["skills", "install", "--json"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(json(&output)["data"][0]["state"], "written");
    let installed =
        fs::read_to_string(home.path().join(".agents/skills/flightdeck/SKILL.md")).unwrap();
    assert!(
        installed.starts_with("---\nname: flightdeck\n"),
        "{installed}"
    );
    assert!(
        installed.contains("metadata:\n  tool: \"flightdeck\"\n  version: \""),
        "{installed}"
    );
}

#[test]
fn check_exits_with_1_and_names_a_missing_copy() {
    let home = home();

    let output = flightdeck(home.path(), &["skills", "check"]);

    assert_eq!(output.status.code(), Some(1));
    let stdout = text(&output);
    assert!(stdout.contains("missing"), "{stdout}");
    assert!(stdout.contains(".agents/skills"), "{stdout}");
}

#[test]
fn check_exits_with_0_and_prints_no_row_after_install() {
    let home = home();
    assert_eq!(
        flightdeck(home.path(), &["skills", "install"])
            .status
            .code(),
        Some(0)
    );

    let output = flightdeck(home.path(), &["skills", "check"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output), "");
}

#[test]
fn check_exits_with_1_when_the_installed_copy_was_edited() {
    let home = home();
    assert_eq!(
        flightdeck(home.path(), &["skills", "install"])
            .status
            .code(),
        Some(0)
    );
    let file = home.path().join(".agents/skills/flightdeck/SKILL.md");
    let edited = fs::read_to_string(&file).unwrap() + "An edit.\n";
    fs::write(&file, edited).unwrap();

    let output = flightdeck(home.path(), &["skills", "check", "--json"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(json(&output)["data"][0]["state"], "differs");
}

#[test]
fn print_prints_the_embedded_skill() {
    let home = home();

    let output = flightdeck(home.path(), &["skills", "print"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(text(&output), include_str!("../skill/SKILL.md"));
}
