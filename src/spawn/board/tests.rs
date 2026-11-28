use std::path::PathBuf;
use std::time::{Duration, Instant};

use crumbkit_process::Cancel;
use tempfile::TempDir;

use super::{BoardAt, BoardLimits, BoardPath, BoardQuery, Commit, RecordType, run_all};
use crate::spawn::{Exit, Program, Stdout};

/// Writes a shell script named `board` that runs `body`, and returns its
/// directory and path.
fn fake_board(body: &str) -> (TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("board");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (directory, path)
}

/// Returns what one finished query printed, as text.
fn text_of(finished: &crate::spawn::Finished) -> String {
    match finished.stdout() {
        Stdout::Whole(bytes) => String::from_utf8(bytes.clone()).unwrap(),
        Stdout::OverLimit => panic!("over the limit"),
    }
}

#[test]
fn the_board_operation_refuses_a_record_type_holding_a_quote_before_any_process_starts() {
    // A query of one type takes a RecordType, so a type with a quote has no
    // query to run, and no process can start for it.
    assert_eq!(RecordType::new("changes/change'--/v2"), None);
    assert_eq!(RecordType::new("changes/change/v2' OR '1"), None);
    assert_eq!(RecordType::new("Changes/change/v2"), None);
    assert_eq!(RecordType::new("changes/v2"), None);
    assert!(RecordType::new("changes/change/v2").is_some());
}

#[test]
fn each_query_names_its_board_and_has_a_fixed_argument_vector() {
    let (_directory, board) = fake_board(r#"printf '%s|' "$@""#);
    let queries = [
        BoardQuery::OfType(RecordType::new("changes/change/v2").unwrap()),
        BoardQuery::Log {
            path: BoardPath::new("/workflow-graphs/small-flow").unwrap(),
            at: Commit::new("07dee52642e50bb3f69169d5cafff3e30de876b2"),
            every: false,
        },
    ];

    let finished = run_all(
        &Program::new(&board),
        &BoardAt::Directory("/boards/home".into()),
        &queries,
        BoardLimits::new().start(),
        &Cancel::new(),
    );

    let printed: Vec<String> = finished
        .iter()
        .map(|each| text_of(each.as_ref().unwrap()))
        .collect();
    assert_eq!(
        printed,
        [
            "-C|/boards/home|query|SELECT logical, type, status, code, body FROM records WHERE type = 'changes/change/v2' ORDER BY logical|--all|--json|--no-input|",
            "-C|/boards/home|log|/workflow-graphs/small-flow|--at|07dee52642e50bb3f69169d5cafff3e30de876b2|--limit|1|--json|--no-input|",
        ]
    );
    let personal = run_all(
        &Program::new(&board),
        &BoardAt::Personal,
        &[BoardQuery::Graphs],
        BoardLimits::new().start(),
        &Cancel::new(),
    );
    assert!(text_of(personal[0].as_ref().unwrap()).starts_with("--personal|query|"));
}

#[test]
fn a_path_that_could_read_as_a_flag_is_no_board_path() {
    assert_eq!(BoardPath::new("--all"), None);
    assert_eq!(BoardPath::new("/a\nb"), None);
    assert_eq!(Commit::new("HEAD"), None);
}

#[test]
fn the_board_operation_cancels_a_query_that_passes_its_deadline() {
    let (_directory, board) = fake_board("sleep 30");
    let started = Instant::now();

    let finished = run_all(
        &Program::new(&board),
        &BoardAt::Personal,
        &[BoardQuery::Graphs],
        BoardLimits::new().time(Duration::from_millis(300)).start(),
        &Cancel::new(),
    );

    assert_eq!(finished[0].as_ref().unwrap().exit(), Exit::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(10));
}
