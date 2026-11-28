use std::path::{Path, PathBuf};

use crumbkit_process::Cancel;
use serde_json::{Value, json};

use super::{Board, Freshness, GraphStatus, Slug, WorkflowShow, WorkflowsError, list, show};
use crate::spawn::{BoardAt, Program};

/// Returns the path of the fake board in the fixtures.
fn fake_board() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/board/fake/board")
}

/// Returns the fake board with the variables `vars`.
fn board(vars: &[(&str, &str)]) -> Board {
    let mut program = Program::new(fake_board());
    for (name, value) in vars {
        program = program.env(*name, *value);
    }
    Board::new(program, BoardAt::Personal)
}

/// Shows the graph `slug` of `board`.
fn shown(board: &Board, slug: &str) -> WorkflowShow {
    show(board, &Slug::new(slug).unwrap(), &Cancel::new()).unwrap()
}

/// Returns the paths placed on `target` and whether each is current.
fn placed(shown: &WorkflowShow, target: &str) -> Vec<(String, bool)> {
    shown
        .bindings
        .iter()
        .filter(|binding| binding.target == target)
        .flat_map(|binding| &binding.records)
        .map(|record| (record.path.clone(), record.current))
        .collect()
}

#[test]
fn workflows_list_gives_each_graph_with_its_node_count_and_status() {
    let listed = list(&board(&[]), &Cancel::new()).unwrap();

    let rows: Vec<(&str, Option<usize>, GraphStatus, usize)> = listed
        .rows
        .iter()
        .map(|row| (row.slug.as_str(), row.nodes, row.status, row.problems))
        .collect();
    assert_eq!(
        rows,
        [
            ("broken", None, GraphStatus::Degraded, 0),
            ("development-flow", Some(34), GraphStatus::Ok, 0),
            ("small-flow", Some(11), GraphStatus::Ok, 0),
            ("tangled", Some(11), GraphStatus::Problems, 1),
            ("weekly-suite-loop", Some(21), GraphStatus::Ok, 0),
        ]
    );
    assert_eq!(
        listed.rows[0].code.as_deref(),
        Some("board::schema::invalid")
    );
    assert_eq!((listed.summary.graphs, listed.summary.drawn), (5, 3));
}

#[test]
fn workflows_show_places_a_change_on_the_node_its_binding_names_when_every_condition_holds() {
    let shown = shown(&board(&[]), "development-flow");

    assert_eq!(
        placed(&shown, "build"),
        [("/changes/sample-build-current".to_owned(), true)]
    );
    // A merged change matches no binding, and a small-flow change is not
    // under /flows/development.
    let every: Vec<String> = shown
        .bindings
        .iter()
        .flat_map(|binding| &binding.records)
        .map(|record| record.path.clone())
        .collect();
    assert!(!every.contains(&"/changes/sample-done".to_owned()));
    assert!(!every.contains(&"/changes/sample-small-build".to_owned()));
    assert_eq!(shown.readings, Some(2));
}

#[test]
fn workflows_show_marks_a_change_on_another_flow_version_than_the_subject() {
    let shown = shown(&board(&[]), "development-flow");

    let plan = shown
        .bindings
        .iter()
        .find(|binding| binding.target == "plan")
        .unwrap();
    let older = plan
        .records
        .iter()
        .find(|record| record.path == "/changes/sample-plan-older")
        .unwrap();
    assert!(!older.current);
    assert_eq!(
        older.version.as_deref(),
        Some("/flows/development/23eb709b901bc3e3")
    );
    assert_eq!(older.title, "A plan on an older flow");
}

#[test]
fn workflows_show_skips_and_counts_a_degraded_bound_record() {
    let shown = shown(&board(&[]), "development-flow");

    let plan = shown
        .bindings
        .iter()
        .find(|binding| binding.target == "plan")
        .unwrap();
    // The degraded change and the change whose phase is a number.
    assert_eq!(plan.skipped, 2);
    assert!(
        plan.records
            .iter()
            .all(|record| record.path != "/changes/sample-degraded")
    );
}

#[test]
fn freshness_is_changed_when_a_commit_after_the_subjects_commit_changed_the_subject() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/board");
    let mut graph: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures.join("graphs/weekly-suite-loop.json")).unwrap(),
    )
    .unwrap();
    graph["subject"]["commit"] = json!("3d0e85285b4819d32b04cd569fe57a925558359d");
    let report = json!({
        "schema": "board/query-report/v2",
        "data": [["/workflow-graphs/weekly-suite-loop", "flightdeck/workflow-graph/v1", "ok", null, graph.to_string()]],
        "truncated": false,
        "summary": {"columns": ["logical", "type", "status", "code", "body"]},
    });
    let directory = tempfile::tempdir().unwrap();
    let graphs = directory.path().join("graphs.json");
    std::fs::write(&graphs, report.to_string()).unwrap();

    let earlier = shown(
        &board(&[("FAKE_BOARD_GRAPHS", graphs.to_str().unwrap())]),
        "weekly-suite-loop",
    );
    let current = shown(&board(&[]), "weekly-suite-loop");

    assert_eq!(earlier.freshness, Freshness::Changed);
    assert_eq!(current.freshness, Freshness::Current);
}

#[test]
fn a_flow_graph_is_not_checked_against_the_running_flow() {
    let shown = shown(&board(&[]), "small-flow");

    assert!(matches!(shown.freshness, Freshness::Unchecked { .. }));
    assert_eq!(shown.running, None);
}

#[test]
fn a_graph_the_check_rejects_shows_its_problems() {
    let shown = shown(&board(&[]), "tangled");

    assert_eq!(shown.status, GraphStatus::Problems);
    assert_eq!(shown.problems.len(), 1);
    assert_eq!(shown.problems[0].pointer, "/edges/2/to");
}

#[test]
fn a_slug_with_no_graph_is_not_found_and_a_missing_board_says_so() {
    let missing = show(
        &board(&[]),
        &Slug::new("no-such-flow").unwrap(),
        &Cancel::new(),
    );
    let no_board = list(
        &Board::new(Program::new("/no/such/board"), BoardAt::Personal),
        &Cancel::new(),
    );
    let refused = list(&board(&[("FAKE_BOARD_OPEN", "refuse")]), &Cancel::new());

    assert!(matches!(missing, Err(WorkflowsError::NotFound { .. })));
    assert!(no_board.is_err_and(|error| error.no_board()));
    assert!(matches!(
        refused,
        Err(WorkflowsError::BoardFailed { code: Some(code), .. }) if code == "board::cli::usage"
    ));
}

#[test]
fn a_slug_is_lowercase_words_joined_by_hyphens() {
    for refused in ["", "-a", "a-", "a--b", "A", "a/b", "a_b", "..", "a b"] {
        assert!(Slug::new(refused).is_err(), "{refused:?}");
    }
    assert!(Slug::new("development-flow-2").is_ok());
}
