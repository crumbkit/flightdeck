//! `flightdeck compliance build` through the binary, against the config in
//! `tests/fixtures/compliance/` and the reports that the pinned Board
//! printed beside it (`tests/fixtures/board/make-reports.sh`).
//!
//! The document the fixtures build is `tests/fixtures/compliance/
//! compliance.json`, the sample that the pages and the headless packages
//! read, and the text summary is `tests/golden/compliance/summary.txt`.
//! `SNAPSHOTS=overwrite cargo test` rewrites both. The contract test needs
//! the Board that `scripts/install-test-tools.sh` installs.

#![cfg(test)]
#![expect(
    clippy::disallowed_methods,
    reason = "the test starts the flightdeck binary and the pinned board"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// Returns the path of `relative` in the repository.
fn here(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// The fixture config.
fn config() -> PathBuf {
    here("tests/fixtures/compliance/compliance.toml")
}

/// The fixture reports directory.
fn reports() -> PathBuf {
    here("tests/fixtures/compliance/reports")
}

/// Reads the JSON file at `path`.
fn json_file(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// One hour after the fixture reports were made, so every report reads
/// current and owner/app's suite run is within the window.
fn built_at() -> String {
    let report = json_file(&reports().join("owner/app.json"));
    let generated: jiff::Timestamp = report["generated"].as_str().unwrap().parse().unwrap();
    let at = generated
        .checked_add(jiff::SignedDuration::from_hours(1))
        .unwrap();
    format!("{at:.0}")
}

/// Runs flightdeck with `args` and a cleared environment.
fn flightdeck(args: &[&str]) -> Output {
    let state = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_flightdeck"))
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("XDG_STATE_HOME", state.path())
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// Runs `flightdeck compliance build` over the fixtures with `extra`.
fn build(extra: &[&str]) -> Output {
    let config = config();
    let reports = reports();
    let at = built_at();
    let mut args = vec![
        "compliance",
        "build",
        "--config",
        config.to_str().unwrap(),
        "--reports",
        reports.to_str().unwrap(),
        "--built-at",
        &at,
        "--build-run",
        "Build run #412",
    ];
    args.extend(extra);
    flightdeck(&args)
}

/// Compares `actual` with the file at `relative`, or rewrites it when
/// `SNAPSHOTS=overwrite` is set.
fn golden(relative: &str, actual: &[u8]) {
    let path = here(relative);
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

/// Fails with the standard error unless `output` exited with `code`.
fn assert_exit(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Fails the test with every place where `document` breaks `schema`.
fn assert_valid(schema: &Value, document: &Value) {
    let validator = jsonschema::validator_for(schema).unwrap();
    let problems: Vec<String> = validator
        .iter_errors(document)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn compliance_build_prints_a_line_per_repository_with_its_report_time_and_counts() {
    let output = build(&[]);
    assert_exit(&output, 0);
    golden("tests/golden/compliance/summary.txt", &output.stdout);
}

#[test]
fn compliance_build_json_prints_the_sample_document_that_matches_its_published_schema() {
    let output = build(&["--json"]);
    assert_exit(&output, 0);
    golden("tests/fixtures/compliance/compliance.json", &output.stdout);
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema"], "flightdeck/compliance/v1");
    assert_valid(
        &json_file(&here("tests/golden/schemas/flightdeck-compliance-v1.json")),
        &document,
    );
}

#[test]
fn out_writes_compliance_json_with_the_bytes_that_json_prints() {
    let out = tempfile::tempdir().unwrap();
    let output = build(&["--json", "--out", out.path().to_str().unwrap()]);
    assert_exit(&output, 0);
    let written = std::fs::read(out.path().join("compliance.json")).unwrap();
    assert_eq!(written, output.stdout);
    let names: Vec<_> = std::fs::read_dir(out.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, ["compliance.json"], "no temporary file is left");
}

#[test]
fn out_into_a_missing_directory_fails_with_out_failed() {
    let out = tempfile::tempdir().unwrap();
    let missing = out.path().join("missing");
    let output = build(&["--json", "--out", missing.to_str().unwrap()]);
    assert_exit(&output, 1);
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::compliance::out_failed");
}

#[cfg(unix)]
#[test]
fn a_failed_write_leaves_the_earlier_compliance_json_unchanged() {
    use std::os::unix::fs::PermissionsExt as _;

    let out = tempfile::tempdir().unwrap();
    let earlier = out.path().join("compliance.json");
    std::fs::write(&earlier, b"earlier\n").unwrap();
    let read_only = std::fs::Permissions::from_mode(0o555);
    std::fs::set_permissions(out.path(), read_only).unwrap();
    let output = build(&["--json", "--out", out.path().to_str().unwrap()]);
    std::fs::set_permissions(out.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_exit(&output, 1);
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::compliance::out_failed");
    assert_eq!(std::fs::read(&earlier).unwrap(), b"earlier\n");
}

#[test]
fn a_config_of_another_schema_fails_with_config_invalid_and_its_line() {
    let dir = tempfile::tempdir().unwrap();
    let text = std::fs::read_to_string(config()).unwrap().replace(
        "flightdeck/compliance-config/v1",
        "flightdeck/compliance-config/v9",
    );
    let path = dir.path().join("compliance.toml");
    std::fs::write(&path, text).unwrap();
    let reports = reports();
    let output = flightdeck(&[
        "compliance",
        "build",
        "--config",
        path.to_str().unwrap(),
        "--reports",
        reports.to_str().unwrap(),
        "--json",
    ]);
    assert_exit(&output, 1);
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::compliance::config_invalid");
    assert_eq!(failure["fields"]["line"], 3, "{failure}");
}

#[test]
fn an_invalid_report_fails_only_its_own_column() {
    let dir = tempfile::tempdir().unwrap();
    let owner = dir.path().join("owner");
    std::fs::create_dir(&owner).unwrap();
    for name in ["app", "ci"] {
        std::fs::copy(
            reports().join(format!("owner/{name}.json")),
            owner.join(format!("{name}.json")),
        )
        .unwrap();
    }
    std::fs::write(owner.join("api.json"), "{\"$type\": \"rules/report/v1\"}").unwrap();
    let config = config();
    let output = flightdeck(&[
        "compliance",
        "build",
        "--config",
        config.to_str().unwrap(),
        "--reports",
        dir.path().to_str().unwrap(),
        "--json",
    ]);
    assert_exit(&output, 0);
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let states: Vec<&Value> = document["data"]["repositories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|column| &column["report"])
        .collect();
    assert_eq!(
        states,
        ["current", "invalid_report", "current", "no_report"],
        "{document}"
    );
}

#[test]
fn the_sample_reports_validate_against_the_copy_of_boards_schema() {
    let schema = json_file(&here("tests/fixtures/board/rules-report-v1.schema.json"));
    for name in ["app", "api", "ci"] {
        assert_valid(
            &schema,
            &json_file(&reports().join(format!("owner/{name}.json"))),
        );
    }
}

#[test]
fn the_fixture_config_validates_against_the_published_config_schema() {
    let text = std::fs::read_to_string(config()).unwrap();
    let config: Value = toml::from_str(&text).unwrap();
    assert_valid(
        &json_file(&here(
            "tests/golden/schemas/flightdeck-compliance-config-v1.json",
        )),
        &config,
    );
}

/// The Board that `scripts/install-test-tools.sh` installs, or a failure
/// that says to run it. The script installs it on Unix alone.
#[cfg(unix)]
fn pinned_board() -> PathBuf {
    let path = here(&format!(
        "target/test-tools/bin/board{}",
        std::env::consts::EXE_SUFFIX
    ));
    assert!(
        path.is_file(),
        "{} is missing; run scripts/install-test-tools.sh to install the pinned Board",
        path.display()
    );
    path
}

#[cfg(unix)]
#[test]
fn the_copy_of_boards_report_schema_matches_what_the_pinned_board_installs() {
    // This Board serves the report's schema through `board plugin enable
    // rules`, which writes it into a board's `.board/types/`, and not
    // through `board schemas show`.
    let board = pinned_board();
    let home = tempfile::tempdir().unwrap();
    let controls = home.path().join("controls");
    std::fs::create_dir_all(controls.join(".board")).unwrap();
    std::fs::write(
        controls.join(".board/board.json"),
        r#"{"$type": "board/config/v1", "commit_on_put": false}"#,
    )
    .unwrap();
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(&controls)
        .output()
        .unwrap();
    assert!(git.status.success(), "{git:?}");
    let enabled = Command::new(&board)
        .args(["plugin", "enable", "rules"])
        .current_dir(&controls)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_STATE_HOME", home.path().join("state"))
        .env("XDG_CACHE_HOME", home.path().join("cache"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        enabled.status.success(),
        "{}",
        String::from_utf8_lossy(&enabled.stderr)
    );
    assert_eq!(
        json_file(&controls.join(".board/types/rules/report/v1.schema.json")),
        json_file(&here("tests/fixtures/board/rules-report-v1.schema.json")),
        "the pinned Board's rules/report/v1 differs from the copy; run tests/fixtures/board/make-reports.sh with it"
    );
}
