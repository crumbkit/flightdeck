use crumbkit::Failure as _;

use super::{Extra, build};
use crate::model::{CommandsList, Tool};
use crate::runner::failure::RUN_REFUSED;
use crate::runner::{RunFailure, RunRequest, Value};

/// shelf's `commands list` document at the commit that the install script
/// pins.
const SHELF: &str = include_str!("../../../tests/fixtures/newer/commands-list-with-goals.json");

fn shelf() -> Tool {
    let commands: CommandsList = serde_json::from_str(SHELF).unwrap();
    Tool::assemble("shelf 0.1.0", commands, Vec::new()).unwrap()
}

fn many(texts: &[&str]) -> Value {
    Value::Many(texts.iter().map(|&text| text.to_owned()).collect())
}

fn reason(failure: &RunFailure) -> String {
    assert_eq!(failure.code(), &RUN_REFUSED);
    failure
        .fields()
        .into_iter()
        .find(|field| field.name() == "reason")
        .map(|field| format!("{:?}", field.value()))
        .unwrap()
}

#[test]
fn options_follow_the_path_as_one_word_each_and_positionals_follow_the_separator() {
    let tool = shelf();
    let request = RunRequest::new(["books", "show"])
        .value("--from", Value::One("ids.txt".to_owned()))
        .value("<ID>", many(&["2", "5"]));

    let (_, vector) = build(&tool, &request).unwrap();

    assert_eq!(
        vector.argv(Extra::None).words(),
        [
            "books",
            "show",
            "--from=ids.txt",
            "--json",
            "--full",
            "--no-input",
            "--",
            "2",
            "5"
        ]
    );
}

#[test]
fn dry_run_and_yes_go_after_flightdecks_flags_and_before_the_separator() {
    let tool = shelf();
    let request = RunRequest::new(["books", "remove"]).value("<ID>", many(&["2"]));
    let (_, vector) = build(&tool, &request).unwrap();

    for (extra, flag) in [(Extra::DryRun, "--dry-run"), (Extra::Yes, "--yes")] {
        assert_eq!(
            vector.argv(extra).words(),
            [
                "books",
                "remove",
                "--json",
                "--full",
                "--no-input",
                flag,
                "--",
                "2"
            ]
        );
    }
}

#[test]
fn the_line_quotes_each_value_and_puts_the_separator_before_a_positional_that_starts_with_a_dash() {
    let tool = shelf();
    let request = RunRequest::new(["books", "show"])
        .value("--from", Value::One("my ids.txt".to_owned()))
        .value("<ID>", many(&["-2", "5"]));

    let (_, vector) = build(&tool, &request).unwrap();

    assert_eq!(
        vector.line(),
        "shelf books show --from='my ids.txt' -- -2 5"
    );
}

#[test]
fn a_value_of_the_wrong_arity_is_refused() {
    let tool = shelf();

    for (path, name, value) in [
        (&["books", "list"], "--all", Value::One("yes".to_owned())),
        (&["books", "list"], "--limit", Value::Flag(true)),
        (&["books", "show"], "<ID>", Value::One("2".to_owned())),
    ] {
        let request = RunRequest::new(path.iter().copied()).value(name, value);

        let failure = build(&tool, &request).unwrap_err();

        assert_eq!(reason(&failure), r#"Text("arity")"#, "{name}");
    }
}

#[test]
fn a_value_that_holds_a_nul_byte_is_refused() {
    let tool = shelf();
    let request = RunRequest::new(["books", "show"]).value("<ID>", many(&["2\0"]));

    let failure = build(&tool, &request).unwrap_err();

    assert_eq!(reason(&failure), r#"Text("nul")"#);
}

#[test]
fn a_false_flag_and_an_empty_list_are_left_out() {
    let tool = shelf();
    let request = RunRequest::new(["books", "list"])
        .value("--all", Value::Flag(false))
        .value("--limit", Value::One("2".to_owned()));
    let empty = RunRequest::new(["books", "show"]).value("<ID>", many(&[]));

    let (_, vector) = build(&tool, &request).unwrap();
    let (_, empty) = build(&tool, &empty).unwrap();

    assert_eq!(vector.line(), "shelf books list --limit=2");
    assert_eq!(empty.line(), "shelf books show");
}
