use crumbkit::Failure as _;

use super::{Extra, build};
use crate::model::{CommandsList, Document, Tool};
use crate::runner::failure::RUN_REFUSED;
use crate::runner::{RunFailure, RunRequest, Value};

/// shelf's `commands list` document at the commit that the install script
/// pins.
const SHELF: &str = include_str!("../../../tests/fixtures/shelf/commands-list.json");

/// The fake tool unruly's `commands list` document.
const UNRULY: &str = include_str!("../../../tests/fixtures/runner/unruly-commands-list.json");

fn shelf() -> Tool {
    let commands: CommandsList = serde_json::from_str(SHELF).unwrap();
    Tool::assemble("shelf 0.1.1", commands, Vec::new()).unwrap()
}

/// crumbkit's `commands list` schema at the commit that the install script
/// pins, which declares `prints_no_result`.
const COMMANDS_LIST_SCHEMA: &str =
    include_str!("../../../tests/fixtures/crumbkit/commands-list-v1.json");

/// unruly as its model loads without documents, as a tool on a crumbkit
/// before crumbkit/crumbkit#291 does, whose `commands list` cannot mark a
/// command that prints no result.
fn unruly() -> Tool {
    let commands: CommandsList = serde_json::from_str(UNRULY).unwrap();
    Tool::assemble("unruly 0.1.0", commands, Vec::new()).unwrap()
}

/// unruly with `commands list` as `edit` leaves it, and with crumbkit's
/// `commands list` schema as its own, as a tool on a crumbkit that marks a
/// command that prints no result.
fn marking_unruly(edit: impl FnOnce(&mut serde_json::Value)) -> Tool {
    let mut document: serde_json::Value = serde_json::from_str(UNRULY).unwrap();
    edit(&mut document);
    let commands: CommandsList = serde_json::from_value(document).unwrap();
    let schema = serde_json::from_str(COMMANDS_LIST_SCHEMA).unwrap();
    let documents = vec![Document::output("unruly/commands-list/v1", schema)];
    let tool = Tool::assemble("unruly 0.1.0", commands, documents).unwrap();
    assert!(tool.marks_no_result());
    tool
}

/// Removes `prints_no_result` from the row of `child silent`, which then
/// stands for a command without a document.
fn unmark_silent(document: &mut serde_json::Value) {
    let rows = document["data"].as_array_mut().unwrap();
    let silent = rows
        .iter_mut()
        .find(|row| row["path"] == "unruly child silent")
        .unwrap();
    silent
        .as_object_mut()
        .unwrap()
        .remove("prints_no_result")
        .unwrap();
}

fn many(texts: &[&str]) -> Value {
    Value::Many(texts.iter().map(|&text| text.to_owned()).collect())
}

fn reason(failure: &RunFailure) -> String {
    assert_eq!(failure.code(), RUN_REFUSED);
    field(failure, "reason")
}

/// Returns the debug print of the failure's field `name`.
fn field(failure: &RunFailure, name: &str) -> String {
    failure
        .fields()
        .into_iter()
        .find(|field| field.name() == name)
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
fn full_is_left_out_for_a_command_that_does_not_list_it_or_prints_a_headed_answer_or_bytes() {
    let tool = unruly();

    for path in [["child", "whole"], ["child", "answer"], ["child", "bytes"]] {
        let (_, vector) = build(&tool, &RunRequest::new(path)).unwrap();

        assert_eq!(
            vector.argv(Extra::None).words(),
            ["child", path[1], "--json", "--no-input", "--"]
        );
    }
}

#[test]
fn full_is_passed_to_a_command_that_lists_it_as_its_own_argument() {
    let tool = unruly();

    let (_, vector) = build(&tool, &RunRequest::new(["child", "rows"])).unwrap();

    assert_eq!(
        vector.argv(Extra::None).words(),
        ["child", "rows", "--json", "--full", "--no-input", "--"]
    );
}

#[test]
fn full_is_left_out_for_a_command_that_commands_list_marks_as_printing_no_result() {
    let tool = marking_unruly(|_| {});

    let (_, vector) = build(&tool, &RunRequest::new(["child", "silent"])).unwrap();

    assert_eq!(
        vector.argv(Extra::None).words(),
        ["child", "silent", "--json", "--no-input", "--"]
    );
}

#[test]
fn full_is_passed_to_a_command_without_a_form_that_a_marking_tool_leaves_unmarked() {
    let tool = marking_unruly(unmark_silent);

    let (_, vector) = build(&tool, &RunRequest::new(["child", "silent"])).unwrap();

    assert_eq!(
        vector.argv(Extra::None).words(),
        ["child", "silent", "--json", "--full", "--no-input", "--"]
    );
}

#[test]
fn full_is_passed_to_a_command_that_declares_a_document() {
    let tool = marking_unruly(|_| {});

    let (_, vector) = build(&tool, &RunRequest::new(["child", "said"])).unwrap();

    assert_eq!(
        vector.argv(Extra::None).words(),
        ["child", "said", "--json", "--full", "--no-input", "--"]
    );
}

/// A tool whose crumbkit cannot mark a command that prints no result gives
/// that command and a command without a document the same row, so this
/// pins the runner's RUST-10 trade-off: the second loses `--full` too.
#[test]
fn full_is_left_out_for_a_command_without_a_form_when_the_tool_cannot_mark_one_that_prints_no_result()
 {
    let tool = unruly();

    let (_, vector) = build(&tool, &RunRequest::new(["child", "silent"])).unwrap();

    assert_eq!(
        vector.argv(Extra::None).words(),
        ["child", "silent", "--json", "--no-input", "--"]
    );
}

#[test]
fn no_input_is_left_out_for_a_command_that_does_not_take_it() {
    let tool = unruly();

    let (_, vector) = build(&tool, &RunRequest::new(["child", "nap"])).unwrap();

    assert_eq!(
        vector.argv(Extra::None).words(),
        ["child", "nap", "--json", "--"]
    );
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
fn a_value_of_the_wrong_arity_is_refused_with_what_the_argument_takes_and_the_paths_words() {
    let tool = shelf();

    for (path, name, value, takes) in [
        (
            &["books", "list"],
            "--all",
            Value::One("yes".to_owned()),
            "flag",
        ),
        (&["books", "list"], "--limit", Value::Flag(true), "one"),
        (
            &["books", "show"],
            "<ID>",
            Value::One("2".to_owned()),
            "many",
        ),
    ] {
        let request = RunRequest::new(path.iter().copied()).value(name, value);

        let failure = build(&tool, &request).unwrap_err();

        assert_eq!(reason(&failure), r#"Text("arity")"#, "{name}");
        assert_eq!(field(&failure, "takes"), format!("Text({takes:?})"));
        assert_eq!(
            field(&failure, "path"),
            format!("List([Text({:?}), Text({:?})])", path[0], path[1])
        );
    }
}

#[test]
fn a_value_that_holds_a_character_that_a_display_line_would_hide_is_refused_by_its_index() {
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
        let request = RunRequest::new(["books", "show"]).value("<ID>", many(&["5", value]));

        let failure = build(&tool, &request).unwrap_err();

        assert_eq!(reason(&failure), r#"Text("control")"#, "{value:?}");
        assert_eq!(field(&failure, "index"), "Unsigned(1)", "{value:?}");
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
    assert_eq!(field(&failure, "earlier"), r#"Text("<FIRST>")"#);
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
