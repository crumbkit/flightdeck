//! flightdeck's own types for crumbkit's documents, checked against the
//! JSON Schemas that crumbkit checks in. The schemas under
//! `tests/fixtures/crumbkit/` are copies from the crumbkit commit that
//! `scripts/install-test-tools.sh` pins. Each test runs shelf, validates
//! what it prints against the schema, and reads the same document into
//! flightdeck's type, so a change in crumbkit's documents fails here.

#![cfg(test)]

use std::path::{Path, PathBuf};
use std::process::Command;

use crumbkit_flightdeck::model::{
    CommandsList, Recovery, SchemasList, SchemasShow, ToolFailure, Variable,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Returns the path of shelf as the install script built it, and fails the
/// test when the script has not run.
fn shelf() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/test-tools/bin/shelf{}",
        std::env::consts::EXE_SUFFIX
    ));
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

/// Returns the schema that shelf prints for its own document `id`, such as
/// `shelf/self-usage/v1`.
fn shelf_schema(id: &str) -> Value {
    let shown = run_shelf(&["schemas", "show", "--json", "--no-input", "--", id]);
    shown["data"][0]["schema"].clone()
}

/// Returns the properties of the rows of the listing that `schema`
/// describes, following one `$ref` into its `$defs`.
fn row_properties<'a>(schema: &'a Value, path: &[&str]) -> &'a serde_json::Map<String, Value> {
    let resolve = |value: &'a Value| -> &'a Value {
        value["$ref"]
            .as_str()
            .and_then(|reference| reference.strip_prefix("#/$defs/"))
            .map_or(value, |name| &schema["$defs"][name])
    };
    let mut node = resolve(&schema["properties"]["data"]["items"]);
    for name in path {
        node = resolve(&node["properties"][*name]);
    }
    node["properties"].as_object().unwrap()
}

/// The fields of a command's figures that flightdeck reads.
const FIGURE_FIELDS: [&str; 7] = [
    "runs",
    "help_pages",
    "help_before",
    "refused",
    "failed",
    "p95_ms",
    "p95_out_bytes",
];

#[test]
fn shelfs_figures_fixture_matches_its_own_schema_which_declares_every_figure_flightdeck_reads() {
    let schema = shelf_schema("shelf/self-figures-list/v1");
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/shelf/self-figures-list.json")).unwrap();

    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&fixture));
    let properties = row_properties(&schema, &[]);
    for field in ["command"].iter().chain(&FIGURE_FIELDS) {
        assert!(properties.contains_key(*field), "{field}");
    }
    assert!(schema["properties"]["summary"].is_object());
}

#[test]
fn shelfs_usage_fixture_matches_its_own_schema_which_declares_every_field_flightdeck_reads() {
    let schema = shelf_schema("shelf/self-usage/v1");
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/shelf/self-usage.json")).unwrap();

    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&fixture));
    let properties = row_properties(&schema, &[]);
    for field in ["finding", "runs", "command", "suggestion", "code"] {
        assert!(properties.contains_key(field), "{field}");
    }
    let figures = row_properties(&schema, &["figures"]);
    for field in FIGURE_FIELDS {
        assert!(figures.contains_key(field), "{field}");
    }
}

/// The fields of a variable of `self environment list` that flightdeck
/// reads.
const VARIABLE_FIELDS: [&str; 16] = [
    "name",
    "type",
    "type_name",
    "unset",
    "unset_rule",
    "source",
    "source_kind",
    "value",
    "show",
    "bind",
    "purpose",
    "effect",
    "values",
    "check",
    "group",
    "owner",
];

#[test]
fn the_environment_fixture_matches_crumbkit_starmaps_schema_which_declares_every_field_flightdeck_reads()
 {
    let schema = environment_schema();
    let mut fixture: Value =
        serde_json::from_str(include_str!("fixtures/starmap/self-environment-list.json")).unwrap();
    // The tool prints its own name in the id, and the schema names crumbkit.
    fixture["schema"] = Value::String("crumbkit/self-environment-list/v1".to_owned());

    let validator = jsonschema::validator_for(&schema).unwrap();
    let problems: Vec<String> = validator
        .iter_errors(&fixture)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
    let properties = row_properties(&schema, &[]);
    for field in VARIABLE_FIELDS {
        assert!(properties.contains_key(field), "{field}");
    }
    let variables: Vec<Variable> = serde_json::from_value(fixture["data"].clone()).unwrap();
    assert_eq!(variables.len(), 16);
}

/// Returns crumbkit-starmap's schema of `self environment list`.
fn environment_schema() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/starmap/self-environment-list-v1.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Returns the words of the closed list `name` in the schema's `$defs`,
/// such as `TypeName`, from each `const` of its `oneOf`.
fn closed_list(schema: &Value, name: &str, key: Option<&str>) -> Vec<String> {
    schema["$defs"][name]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|each| {
            let word = key.map_or(each, |key| &each["properties"][key]);
            word["const"].as_str().unwrap().to_owned()
        })
        .collect()
}

/// Returns the fixture's first variable with `edit` applied, read into
/// flightdeck's `Variable` and sent back as the page receives it.
fn read_back(edit: impl FnOnce(&mut Value)) -> Value {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/starmap/self-environment-list.json")).unwrap();
    let mut row = fixture["data"][0].clone();
    edit(&mut row);
    let variable: Variable = serde_json::from_value(row).unwrap();
    serde_json::to_value(variable).unwrap()
}

#[test]
fn flightdeck_reads_every_word_of_the_closed_lists_that_crumbkit_starmaps_schema_declares() {
    let schema = environment_schema();

    let types = closed_list(&schema, "TypeName", None);
    assert!(types.contains(&"socket_address".to_owned()), "{types:?}");
    for word in &types {
        let sent = read_back(|row| row["type_name"] = Value::String(word.clone()));
        assert_eq!(sent["type_name"], *word);
    }

    for word in closed_list(&schema, "SourceWord", None) {
        let sent = read_back(|row| row["source_kind"] = serde_json::json!({ "kind": word }));
        assert_eq!(sent["source_kind"]["kind"], *word);
    }

    for word in closed_list(&schema, "Rule", Some("rule")) {
        let rule = if word == "default" {
            serde_json::json!({ "rule": word, "default": "8080" })
        } else {
            serde_json::json!({ "rule": word })
        };
        let sent = read_back(|row| row["unset_rule"] = serde_json::json!({ "local": rule }));
        assert_eq!(sent["unset_rule"]["local"]["rule"], *word);
    }
}
