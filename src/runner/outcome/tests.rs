use crumbkit::{Failure as _, FieldValue};

use super::{Ending, Outcome, Tail, outcome};
use crate::model::{ExitMeaning, Form};
use crate::runner::failure::RUN_NO_DOCUMENT;
use crate::spawn::{Exit, Limits, Stop};

/// The ending of a run of `shelf books show` with `form` that declares
/// exit 3 as `the catalogue is locked`.
fn ending(form: Form) -> Ending {
    let exits: Vec<ExitMeaning> =
        serde_json::from_str(r#"[{"exit": 3, "last": 3, "meaning": "the catalogue is locked"}]"#)
            .unwrap();
    Ending {
        tool: "shelf".to_owned(),
        path: "books show".to_owned(),
        line: "shelf books show 2 7".to_owned(),
        form: Some(form),
        exits,
        limits: Limits::default(),
    }
}

const BATCH: &str = r#"{"schema":"shelf/books-show/v1","data":[],"failures":[{"code":"shelf::books::not_found","message":"no book 7","recovery":{"kind":"retry"}}]}"#;

fn field(outcome: &Outcome, name: &str) -> Option<FieldValue<'static>> {
    let Outcome::Failed(failure) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(failure.code(), &RUN_NO_DOCUMENT);
    failure
        .fields()
        .into_iter()
        .find(|field| field.name() == name)
        .map(|field| field.value().clone().into_owned())
}

#[test]
fn a_document_with_exit_1_or_a_declared_code_is_a_result_with_failures() {
    let ending = ending(Form::Document);

    for code in [1, 3] {
        let outcome = outcome(&ending, Some(BATCH), Exit::Code(code), None, b"");

        assert!(matches!(outcome, Outcome::ResultWithFailures), "{code}");
    }
}

#[test]
fn a_stream_that_prints_nothing_and_exits_0_is_an_empty_result() {
    let outcome = outcome(&ending(Form::Stream), None, Exit::Code(0), None, b"");

    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
}

#[test]
fn nothing_parseable_with_exit_130_is_cancelled_without_a_cancel() {
    let outcome = outcome(
        &ending(Form::Document),
        Some("partial {"),
        Exit::Code(130),
        None,
        b"",
    );

    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
}

#[test]
fn a_signal_without_a_cancel_is_no_document_with_a_null_exit() {
    let outcome = outcome(&ending(Form::Document), None, Exit::Signal(9), None, b"");

    assert_eq!(field(&outcome, "exit"), Some(FieldValue::Null));
}

#[test]
fn no_document_names_the_declared_meaning_of_its_exit_and_the_last_40_lines_of_standard_error() {
    let stderr: String = (1..=45).map(|n| format!("line {n}\n")).collect();

    let outcome = outcome(
        &ending(Form::Document),
        Some("not json"),
        Exit::Code(3),
        None,
        stderr.as_bytes(),
    );

    assert_eq!(
        field(&outcome, "meaning"),
        Some(FieldValue::from("the catalogue is locked"))
    );
    let Some(FieldValue::List(lines)) = field(&outcome, "stderr") else {
        panic!("no stderr list");
    };
    assert_eq!(lines.len(), 40);
    assert_eq!(lines.first(), Some(&FieldValue::from("line 6")));
}

#[test]
fn a_cancelled_run_whose_tool_printed_its_document_and_exited_0_is_cancelled() {
    let document = r#"{"schema":"shelf/books-show/v1","data":[]}"#;

    let outcome = outcome(
        &ending(Form::Document),
        Some(document),
        Exit::Code(0),
        Some(Stop::Cancel),
        b"",
    );

    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
}

#[test]
fn a_cancel_after_the_tool_printed_its_failure_keeps_the_failure() {
    let failure = r#"{"schema":"shelf/failure/v1","code":"shelf::cli::interrupted","message":"interrupted","recovery":{"kind":"retry"}}"#;

    let outcome = outcome(
        &ending(Form::Document),
        Some(failure),
        Exit::Code(130),
        Some(Stop::Cancel),
        b"",
    );

    let Outcome::ToolFailure(failure) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(failure.code(), "shelf::cli::interrupted");
}

/// Returns the tail of a run that printed `output`, line by line.
fn tail_of(output: &str) -> Tail {
    let mut tail = Tail::default();
    for line in output.lines() {
        tail.push(line);
    }
    tail
}

#[test]
fn the_tail_starts_again_at_each_line_that_starts_with_a_brace() {
    let tail = tail_of("{\"n\":1}\nnot json\n{\n  \"n\": 2\n}");

    assert_eq!(tail.get(), Some("{\n  \"n\": 2\n}"));
}

#[test]
fn an_indented_document_with_exit_0_is_a_result() {
    let document =
        "{\n  \"schema\": \"shelf/books-show/v1\",\n  \"data\": {\n    \"id\": 2\n  }\n}";
    let tail = tail_of(document);

    let outcome = outcome(
        &ending(Form::Document),
        tail.get(),
        Exit::Code(0),
        None,
        b"",
    );

    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
}

#[test]
fn an_indented_failure_document_is_the_tools_failure() {
    let failure = "{\n  \"schema\": \"shelf/failure/v1\",\n  \"code\": \"shelf::books::not_found\",\n  \"message\": \"no book 7\",\n  \"recovery\": {\n    \"kind\": \"retry\"\n  }\n}";
    let tail = tail_of(failure);

    let outcome = outcome(
        &ending(Form::Document),
        tail.get(),
        Exit::Code(3),
        None,
        b"",
    );

    let Outcome::ToolFailure(failure) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(failure.code(), "shelf::books::not_found");
}
