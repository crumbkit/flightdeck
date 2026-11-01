use crumbkit::Failure as _;

use super::{Extra, build};
use crate::model::{CommandsList, Tool};
use crate::runner::failure::RUN_REFUSED;
use crate::runner::{RunFailure, RunRequest, Value};

/// shelf's `commands list` document at the commit that the install script
/// pins.
const SHELF: &str = include_str!("../../../tests/fixtures/newer/commands-list-with-goals.json");

/// The fake tool unruly's `commands list` document.
const UNRULY: &str = include_str!("../../../tests/fixtures/runner/unruly-commands-list.json");

fn shelf() -> Tool {
    let commands: CommandsList = serde_json::from_str(SHELF).unwrap();
    Tool::assemble("shelf 0.1.0", commands, Vec::new()).unwrap()
}

fn unruly() -> Tool {
    let commands: CommandsList = serde_json::from_str(UNRULY).unwrap();
    Tool::assemble("unruly 0.1.0", commands, Vec::new()).unwrap()
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
        vector.line(Extra::None),
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
fn a_value_that_holds_a_character_that_a_display_line_would_hide_is_refused() {
    let tool = shelf();

    let values = [
        "2\0",
        "2\n5",
        "\u{1b}[31m2",
        "\u{202e}2",
        "2\u{2066}",
        "2\u{2028}5",
        "2\u{2029}5",
        "2\u{200b}",
        "\u{feff}2",
        "2\u{ad}5",
        "2\u{180e}5",
        "2\u{2060}5",
        "2\u{2064}5",
        "2\u{e0001}",
    ];
    for value in values {
        let request = RunRequest::new(["books", "show"]).value("<ID>", many(&[value]));

        let failure = build(&tool, &request).unwrap_err();

        assert_eq!(reason(&failure), r#"Text("control")"#, "{value:?}");
    }
}

#[test]
fn a_positional_value_is_refused_when_an_earlier_positional_has_none() {
    let tool = unruly();
    let one = |text: &str| Value::One(text.to_owned());
    let second = RunRequest::new(["child", "pair"]).value("<SECOND>", one("x"));
    let first = RunRequest::new(["child", "pair"]).value("<FIRST>", one("x"));

    let failure = build(&tool, &second).unwrap_err();
    let (_, vector) = build(&tool, &first).unwrap();

    assert_eq!(reason(&failure), r#"Text("skipped")"#);
    assert_eq!(vector.line(Extra::None), "unruly child pair x");
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

    assert_eq!(vector.line(Extra::None), "shelf books list --limit=2");
    assert_eq!(empty.line(Extra::None), "shelf books show");
}
