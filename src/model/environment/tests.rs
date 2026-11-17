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
            "unset": "default info",
            "source": "setting",
            "value": { "state": "default", "text": "info" },
            "show": "shown",
            "bind": null,
            "cache": null,
            "purpose": "The least severe message that shelf writes to its log.",
            "effect": "shelf logs at info.",
            "enumeration": "log-level",
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
