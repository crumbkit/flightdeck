use super::{ValueState, read};

/// shelf's `self environment list --json --full`, as crumbkit-starmap
/// prints it for a tool that embeds `tests/fixtures/starmap/starmap.toml`.
const LISTED: &str = include_str!("../../../tests/fixtures/starmap/self-environment-list.json");

/// Returns `LISTED` with `edit` applied to its JSON.
fn edited(edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut document: serde_json::Value = serde_json::from_str(LISTED).unwrap();
    edit(&mut document);
    serde_json::to_vec(&document).unwrap()
}

#[test]
fn every_variable_reads_in_the_tools_order_with_crumbkits_after_the_tools_own() {
    let variables = read(LISTED.as_bytes()).unwrap();

    let names: Vec<(&str, Option<&str>)> = variables
        .iter()
        .map(|variable| (variable.name(), variable.owner()))
        .collect();
    assert_eq!(names.len(), 16);
    assert_eq!(names[0], ("SHELF_PORT", None));
    assert_eq!(names[6], ("SHELF_REGION", None));
    assert_eq!(names[7], ("SHELF_OUTPUT_BUDGET", Some("crumbkit")));
    assert_eq!(names[15], ("TRACESTATE", Some("crumbkit")));
}

#[test]
fn each_variable_sends_the_documents_field_names_and_null_for_a_field_the_tool_left_out() {
    let variables = read(LISTED.as_bytes()).unwrap();
    let level = variables
        .iter()
        .find(|variable| variable.name() == "SHELF_LOG_LEVEL")
        .unwrap();

    assert_eq!(
        serde_json::to_value(level).unwrap(),
        serde_json::json!({
            "name": "SHELF_LOG_LEVEL",
            "type": "log-level: one of error, warn, info, debug",
            "type_name": "enumeration",
            "unset": "default info",
            "unset_rule": { "local": { "rule": "default", "default": "info" }, "production": null },
            "source": "setting",
            "source_kind": { "kind": "setting", "bind": null },
            "value": { "state": "default", "text": "info" },
            "show": "shown",
            "bind": null,
            "purpose": "The least severe message that shelf writes to its log.",
            "effect": "shelf logs at info.",
            "values": [
                { "value": "error", "description": "Only failures." },
                { "value": "warn", "description": "Failures, and anything that may become one." },
                { "value": "info", "description": "Each command that runs, and what it changed." },
                { "value": "debug", "description": "Every step, for finding a fault." },
            ],
            "check": null,
            "group": null,
            "owner": null,
        })
    );
}

#[test]
fn a_withheld_value_has_no_text_even_when_a_tool_prints_one() {
    let stdout = edited(|document| {
        for row in document["data"].as_array_mut().unwrap() {
            if row["name"] == "SHELF_MIRROR_TOKEN" {
                row["value"]["text"] = "hunter2".into();
            }
        }
    });

    let variables = read(&stdout).unwrap();
    let token = variables
        .iter()
        .find(|variable| variable.name() == "SHELF_MIRROR_TOKEN")
        .unwrap();

    assert_eq!(token.state(), ValueState::Set);
    assert_eq!(token.text(), None);
    assert!(
        !serde_json::to_string(&variables)
            .unwrap()
            .contains("hunter2")
    );
}

#[test]
fn a_value_without_a_show_field_has_no_text() {
    let stdout = edited(|document| {
        for row in document["data"].as_array_mut().unwrap() {
            row.as_object_mut().unwrap().remove("show");
        }
    });

    let variables = read(&stdout).unwrap();

    assert!(variables.iter().all(|variable| variable.text().is_none()));
}

#[test]
fn a_row_that_does_not_read_is_left_out_and_the_others_still_read() {
    let stdout = edited(|document| {
        document["data"][1]["value"]["state"] = "unknown".into();
    });

    let variables = read(&stdout).unwrap();

    assert_eq!(variables.len(), 15);
    assert!(
        variables
            .iter()
            .all(|variable| variable.name() != "SHELF_DEBUG")
    );
}

#[test]
fn another_document_or_text_gives_no_environment() {
    let other = edited(|document| document["schema"] = "shelf/books-list/v1".into());

    assert!(read(&other).is_none());
    assert!(read(b"SHELF_PORT=8080").is_none());
}

/// Returns the fixture's variable `name` from `stdout`, read as the page
/// receives it.
fn sent(stdout: &[u8], name: &str) -> serde_json::Value {
    let variables = read(stdout).unwrap();
    let variable = variables
        .iter()
        .find(|variable| variable.name() == name)
        .unwrap();
    serde_json::to_value(variable).unwrap()
}

#[test]
fn a_rule_that_differs_in_production_and_a_bind_kind_are_sent_as_data() {
    let token = sent(LISTED.as_bytes(), "SHELF_MIRROR_TOKEN");

    assert_eq!(token["type_name"], "string");
    assert_eq!(
        token["unset_rule"],
        serde_json::json!({
            "local": { "rule": "optional" },
            "production": { "rule": "required" },
        })
    );
    assert_eq!(
        token["source_kind"],
        serde_json::json!({ "kind": "secret", "bind": "1Password" })
    );
}

#[test]
fn a_row_from_a_tool_without_the_structured_fields_reads_with_its_text_alone() {
    let stdout = edited(|document| {
        for row in document["data"].as_array_mut().unwrap() {
            let row = row.as_object_mut().unwrap();
            row.remove("type_name");
            row.remove("unset_rule");
            row.remove("source_kind");
        }
    });

    let port = sent(&stdout, "SHELF_PORT");

    assert_eq!(port["type"], "port");
    assert_eq!(port["unset"], "default 8080");
    assert_eq!(port["source"], "setting");
    assert_eq!(port["type_name"], serde_json::Value::Null);
    assert_eq!(port["unset_rule"], serde_json::Value::Null);
    assert_eq!(port["source_kind"], serde_json::Value::Null);
}

#[test]
fn a_type_or_source_word_that_a_later_starmap_adds_reads_as_other() {
    let stdout = edited(|document| {
        let row = &mut document["data"][0];
        row["type_name"] = "duration".into();
        row["source_kind"] = serde_json::json!({ "kind": "derived" });
    });

    let port = sent(&stdout, "SHELF_PORT");

    assert_eq!(port["type_name"], "other");
    assert_eq!(
        port["source_kind"],
        serde_json::json!({ "kind": "other", "bind": null })
    );
}

#[test]
fn an_unset_rule_that_does_not_read_is_left_out_and_the_row_still_reads() {
    let stdout = edited(|document| {
        document["data"][0]["unset_rule"] = serde_json::json!({ "local": { "rule": "later" } });
    });

    let port = sent(&stdout, "SHELF_PORT");

    assert_eq!(port["unset"], "default 8080");
    assert_eq!(port["unset_rule"], serde_json::Value::Null);
}
