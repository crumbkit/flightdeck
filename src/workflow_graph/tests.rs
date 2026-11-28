use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Problem, WorkflowGraph, check, schema};

/// Returns the path of `relative` in the repository.
fn here(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// Reads the example record named `name`, such as `development-flow`.
fn example(name: &str) -> Value {
    let path = here(&format!("tests/fixtures/board/graphs/{name}.json"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Returns the problems that the check finds in `record`.
fn problems(record: &Value) -> Vec<Problem> {
    let graph: WorkflowGraph = serde_json::from_value(record.clone()).unwrap();
    check(&graph)
}

/// Returns the codes and pointers of `problems`, for a short assertion.
fn found(problems: &[Problem]) -> Vec<(String, String)> {
    problems
        .iter()
        .map(|problem| (problem.code.clone(), problem.pointer.clone()))
        .collect()
}

/// Returns whether the generated schema admits `record`.
fn admits(record: &Value) -> bool {
    jsonschema::validator_for(&schema())
        .unwrap()
        .is_valid(record)
}

const EXAMPLES: [&str; 3] = ["development-flow", "small-flow", "weekly-suite-loop"];

#[test]
fn the_generated_schema_equals_the_checked_in_schema_file() {
    let path = here("plugin/types/flightdeck/workflow-graph/v1.schema.json");
    let generated = super::schema_text();
    #[expect(
        clippy::disallowed_methods,
        reason = "SNAPSHOTS=overwrite rewrites the golden files, as the integration tests read it"
    )]
    let overwrite = std::env::var_os("SNAPSHOTS");
    if overwrite.is_some_and(|value| value == "overwrite") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &generated).unwrap();
        return;
    }
    let checked_in = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        checked_in,
        generated,
        "{} differs from the structs; run SNAPSHOTS=overwrite cargo test to rewrite it",
        path.display()
    );
}

#[test]
fn the_three_signed_example_records_pass_the_schema_and_the_check() {
    for name in EXAMPLES {
        let record = example(name);
        assert!(admits(&record), "{name} fails the schema");
        assert_eq!(problems(&record), Vec::new(), "{name}");
    }
}

#[test]
fn a_cite_that_holds_both_a_pointer_and_lines_fails_the_schema() {
    let mut record = example("small-flow");
    record["nodes"][1]["cite"] = json!({
        "source": 0,
        "at": "pointer",
        "pointer": "/phases/0",
        "lines": {"from": 1, "to": 2},
    });

    assert!(!admits(&record));
}

#[test]
fn the_schema_refuses_a_ninth_lane() {
    let mut record = example("small-flow");
    let lanes: Vec<Value> = (0..9)
        .map(|index| json!({"id": format!("lane-{index}"), "label": "A lane", "actor": "agent"}))
        .collect();
    record["lanes"] = Value::Array(lanes);

    assert!(!admits(&record));
}

#[test]
fn the_check_reports_an_edge_whose_end_names_nothing_with_its_pointer() {
    let mut record = example("small-flow");
    record["edges"][2]["to"] = json!("nowhere");

    assert_eq!(
        found(&problems(&record)),
        vec![(
            "flightdeck::graph::unknown_end".to_owned(),
            "/edges/2/to".to_owned()
        )]
    );
}

#[test]
fn the_check_reports_a_gate_that_lacks_a_fail_edge() {
    let mut record = example("small-flow");
    // The gate `head` gets a pass edge in place of its next edge, and has
    // no fail edge.
    record["edges"][3]["kind"] = json!("pass");

    assert_eq!(
        found(&problems(&record)),
        vec![(
            "flightdeck::graph::gate_edges".to_owned(),
            "/nodes/3".to_owned()
        )]
    );
}

#[test]
fn the_check_reports_a_cite_source_index_past_the_end_of_the_sources() {
    let mut record = example("small-flow");
    record["nodes"][1]["cite"]["source"] = json!(1);

    assert_eq!(
        found(&problems(&record)),
        vec![(
            "flightdeck::graph::cite_source".to_owned(),
            "/nodes/1/cite/source".to_owned()
        )]
    );
}

#[test]
fn the_check_reports_a_duplicate_id_a_parent_cycle_and_a_backwards_line_range() {
    let mut record = example("weekly-suite-loop");
    record["nodes"][2]["id"] = json!("read-suite");
    record["groups"][0]["parent"] = json!("week");
    record["groups"][1]["parent"] = json!("setup");
    record["nodes"][1]["cite"]["lines"] = json!({"from": 30, "to": 23});

    let codes: Vec<String> = problems(&record)
        .into_iter()
        .map(|problem| problem.code)
        .collect();
    for code in ["duplicate_id", "group_cycle", "lines_backwards"] {
        assert!(
            codes.contains(&format!("flightdeck::graph::{code}")),
            "{code} missing from {codes:?}"
        );
    }
}

#[test]
fn the_check_reports_a_fail_edge_that_leaves_a_step() {
    let mut record = example("small-flow");
    // Edge 11 is review-run's retry edge; review-run is a step.
    record["edges"][11]["kind"] = json!("fail");

    assert_eq!(
        found(&problems(&record)),
        vec![(
            "flightdeck::graph::not_a_gate".to_owned(),
            "/edges/11/kind".to_owned()
        )]
    );
}
