//! End-to-end tests of the `flightdeck completion` commands and of the
//! completion request that a shell makes. Each test runs the built binary
//! with `HOME` and the XDG directories set to a temporary directory of its
//! own, so no test writes into the real home directory.

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

/// Runs the built `flightdeck` binary with `args`, a cleared environment,
/// `HOME` set to `home` and the XDG directories under it. `extra` adds
/// environment variables to the run.
fn flightdeck(home: &Path, args: &[&str], extra: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_flightdeck"));
    command
        .args(args)
        .current_dir(home)
        .env_clear()
        .env("HOME", home)
        .env("XDG_STATE_HOME", home.join("state"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"));
    for (name, value) in extra {
        command.env(name, value);
    }
    command.output().unwrap()
}

/// Returns the standard output of a run as text.
fn text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

/// Returns an empty temporary home.
fn home() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn completion_install_writes_fish_completion_naming_flightdeck_under_the_config_directory() {
    let home = home();

    let output = flightdeck(
        home.path(),
        &["completion", "install", "--shell", "fish", "--json"],
        &[],
    );

    assert_eq!(output.status.code(), Some(0));
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["data"][0]["state"], "written");
    let file = home.path().join("config/fish/completions/flightdeck.fish");
    let script = fs::read_to_string(file).unwrap();
    assert!(script.contains("flightdeck"), "{script}");
}

#[test]
fn completion_check_exits_with_1_and_names_a_missing_file() {
    let home = home();

    let output = flightdeck(
        home.path(),
        &["completion", "check", "--shell", "fish"],
        &[],
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output).contains("missing"), "{}", text(&output));
}

#[test]
fn completion_check_exits_with_0_after_install() {
    let home = home();
    flightdeck(
        home.path(),
        &["completion", "install", "--shell", "fish"],
        &[],
    );

    let output = flightdeck(
        home.path(),
        &["completion", "check", "--shell", "fish"],
        &[],
    );

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
}

#[test]
fn completion_print_writes_a_fish_registration_that_runs_flightdeck() {
    let home = home();

    let output = flightdeck(
        home.path(),
        &["completion", "print", "--shell", "fish"],
        &[],
    );

    assert_eq!(output.status.code(), Some(0));
    let script = text(&output);
    assert!(script.contains("COMPLETE=fish"), "{script}");
    assert!(script.contains("flightdeck"), "{script}");
    assert!(!home.path().join("config/fish").exists());
}

#[test]
fn completion_install_reads_the_shell_from_the_shell_variable() {
    let home = home();

    flightdeck(
        home.path(),
        &["completion", "install"],
        &[("SHELL", "/usr/bin/fish")],
    );

    assert!(
        home.path()
            .join("config/fish/completions/flightdeck.fish")
            .exists()
    );
}

#[test]
fn the_complete_variable_makes_flightdeck_print_a_fish_registration() {
    let home = home();

    let output = flightdeck(home.path(), &[], &[("COMPLETE", "fish")]);

    assert_eq!(output.status.code(), Some(0));
    let script = text(&output);
    assert!(script.contains("complete"), "{script}");
    assert!(script.contains("COMPLETE=fish"), "{script}");
}

#[test]
fn a_completion_request_completes_the_names_of_flightdeck_commands() {
    let home = home();

    let output = flightdeck(
        home.path(),
        &["--", "flightdeck", "com"],
        &[("COMPLETE", "fish"), ("_CLAP_COMPLETE_INDEX", "1")],
    );

    assert_eq!(output.status.code(), Some(0));
    assert!(text(&output).contains("completion"), "{}", text(&output));
}
