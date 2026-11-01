//! flightdeck's own types for crumbkit's documents, checked against the
//! JSON Schemas that crumbkit checks in. The schemas under
//! `tests/fixtures/crumbkit/` are copies from the crumbkit commit that
//! `scripts/install-test-tools.sh` pins. Each test runs shelf, validates
//! what it prints against the schema, and reads the same document into
//! flightdeck's type, so a change in crumbkit's documents fails here.

#![cfg(test)]

use std::path::{Path, PathBuf};
use std::process::Command;

use crumbkit_flightdeck::model::{CommandsList, Recovery, SchemasList, SchemasShow, ToolFailure};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Returns the path of shelf as the install script built it, and fails the
/// test when the script has not run.
fn shelf() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-tools/bin/shelf");
    assert!(
        path.is_file(),
        "{} is missing; run scripts/install-test-tools.sh to install shelf and reading",
        path.display()
    );
    path
}

/// Runs shelf with `args` and temporary XDG directories, and returns the
/// JSON document it prints.
fn run_shelf(args: &[&str]) -> Value {
    let dirs = tempfile::tempdir().unwrap();
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs shelf directly to read its documents as crumbkit prints them"
    )]
    let output = Command::new(shelf())
        .args(args)
        .env("XDG_CONFIG_HOME", dirs.path().join("config"))
        .env("XDG_DATA_HOME", dirs.path().join("data"))
        .env("XDG_STATE_HOME", dirs.path().join("state"))
        .env("XDG_CACHE_HOME", dirs.path().join("cache"))
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Reads the checked-in schema named `name`, such as `commands-list-v1`.
fn crumbkit_schema(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/crumbkit")
        .join(format!("{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Validates `document` against crumbkit's schema `name`, and then reads it
/// into `T`. A tool prints its own name in the `schema` id, such as
/// `shelf/commands-list/v1`, and the checked-in schema names crumbkit, so
/// the id is respelled before the check.
fn validate_and_read<T: DeserializeOwned>(name: &str, mut document: Value) -> T {
    let id = format!("crumbkit/{}", name.replace("-v1", "/v1"));
    document["schema"] = Value::String(id);
    let validator = jsonschema::validator_for(&crumbkit_schema(name)).unwrap();
    let problems: Vec<String> = validator
        .iter_errors(&document)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
    serde_json::from_value(document).unwrap()
}

#[test]
fn shelfs_commands_list_matches_crumbkits_schema_and_reads_into_a_commands_list() {
    let document = run_shelf(&[
        "commands",
        "list",
        "--json",
        "--full",
        "--all",
        "--no-input",
    ]);

    let list: CommandsList = validate_and_read("commands-list-v1", document);

    assert!(!list.truncated());
}

#[test]
fn shelfs_schemas_list_matches_crumbkits_schema_and_reads_into_a_schemas_list() {
    let document = run_shelf(&["schemas", "list", "--json", "--full", "--all", "--no-input"]);

    let list: SchemasList = validate_and_read("schemas-list-v1", document);

    assert!(list.ids().any(|id| id == "shelf/books-list/v1"));
}

#[test]
fn shelfs_schemas_show_matches_crumbkits_schema_and_reads_into_a_schemas_show() {
    let document = run_shelf(&[
        "schemas",
        "show",
        "--json",
        "--no-input",
        "--",
        "shelf/books-list/v1",
    ]);

    let show: SchemasShow = validate_and_read("schemas-show-v1", document);

    assert_eq!(show.ids().collect::<Vec<_>>(), ["shelf/books-list/v1"]);
}

#[test]
fn shelfs_failure_document_matches_crumbkits_schema_and_reads_into_a_tool_failure() {
    let document = run_shelf(&["books", "list", "--limit", "101", "--json", "--no-input"]);

    let failure: ToolFailure = validate_and_read("failure-v1", document);

    assert_eq!(failure.code(), "shelf::cli::usage");
    assert_eq!(
        failure.recovery(),
        &Recovery::Run {
            call: "shelf books list --all --json --no-input".to_owned(),
            all: None,
        }
    );
    assert_eq!(failure.alternatives().len(), 1);
}
