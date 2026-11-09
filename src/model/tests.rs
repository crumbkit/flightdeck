use super::{CommandsList, Document, Effects, Listing, NotDocument, StepCommand, Tool};

/// shelf's `commands list` document, with `goals` on its root row.
const WITH_GOALS: &str = include_str!("../../tests/fixtures/shelf/commands-list.json");

/// shelf's `books list --json` document, with a next step that calls a
/// shelf command and one that runs another program first.
const WITH_NEXT: &str = include_str!("../../tests/fixtures/shelf/books-list-with-run-step.json");

/// crumbkit's `commands list` schema, which declares `prints_no_result`.
const COMMANDS_LIST_SCHEMA: &str =
    include_str!("../../tests/fixtures/crumbkit/commands-list-v1.json");

#[test]
fn null_effects_are_unknown() {
    let effects: Effects = serde_json::from_str("null").unwrap();

    assert_eq!(effects, Effects::Unknown);
}

#[test]
fn the_model_gives_each_command_the_class_that_its_effects_decide() {
    let hints = |read_only, destructive| {
        serde_json::json!({
            "read_only": read_only,
            "destructive": destructive,
            "idempotent": false,
            "open_world": false,
        })
    };
    let class_of = |effects: serde_json::Value| {
        let mut document: serde_json::Value = serde_json::from_str(WITH_GOALS).unwrap();
        for row in document["data"].as_array_mut().unwrap() {
            if row["path"] == "shelf books list" {
                row["effects"] = effects.clone();
            }
        }
        let commands: CommandsList = serde_json::from_value(document).unwrap();
        let tool = Tool::assemble("shelf 0.1.1", commands, Vec::new()).unwrap();
        let command = tool.command(&["books", "list"]).unwrap();
        serde_json::to_value(command).unwrap()["class"].clone()
    };

    assert_eq!(class_of(hints(true, false)), "read_only");
    assert_eq!(class_of(hints(false, false)), "additive");
    assert_eq!(class_of(hints(false, true)), "destructive");
    assert_eq!(class_of(hints(true, true)), "destructive");
    assert_eq!(class_of(serde_json::Value::Null), "destructive");
}

#[test]
fn a_listing_reads_its_next_steps_with_calls_and_runs() {
    let listing: Listing<serde_json::Value> = serde_json::from_str(WITH_NEXT).unwrap();

    let steps: Vec<(&str, &[StepCommand])> = listing
        .next()
        .iter()
        .map(|step| (step.text(), step.commands()))
        .collect();
    assert_eq!(
        steps,
        [
            (
                "takes up to 100 ids in one call.",
                &[StepCommand::Call("shelf books show <ID>...".to_owned())][..],
            ),
            (
                "Delete the file of removed books to put every book back, then list them.",
                &[
                    StepCommand::Run("rm /home/reader/.local/state/shelf/removed.txt".to_owned()),
                    StepCommand::Call("shelf books list".to_owned()),
                ][..],
            ),
        ]
    );
}

#[test]
fn null_goals_on_the_root_row_read_as_no_goals() {
    let mut document: serde_json::Value = serde_json::from_str(WITH_GOALS).unwrap();
    for row in document["data"].as_array_mut().unwrap() {
        if row.get("goals").is_some() {
            row["goals"] = serde_json::Value::Null;
        }
    }
    let commands: CommandsList = serde_json::from_value(document).unwrap();

    let tool = Tool::assemble("shelf 0.1.1", commands, Vec::new()).unwrap();

    let titles: Vec<&str> = tool.headings().iter().map(|h| h.title()).collect();
    assert_eq!(titles, ["shelf"]);
}

#[test]
fn null_next_steps_on_a_listing_read_as_no_next_steps() {
    let mut document: serde_json::Value = serde_json::from_str(WITH_NEXT).unwrap();
    document["next"] = serde_json::Value::Null;

    let listing: Listing<serde_json::Value> = serde_json::from_value(document).unwrap();

    assert!(listing.next().is_empty());
}

#[test]
fn a_command_word_or_an_argument_name_with_a_control_character_is_refused_when_the_model_loads() {
    for (from, to) in [
        (r#""--limit""#, r#""--li\u001bmit""#),
        (
            r#""path":"shelf books show""#,
            r#""path":"shelf books sh\u202eow""#,
        ),
    ] {
        assert!(WITH_GOALS.contains(from), "{from}");
        let commands: CommandsList = serde_json::from_str(&WITH_GOALS.replace(from, to)).unwrap();

        let refused = Tool::assemble("shelf 0.1.1", commands, Vec::new());

        assert!(matches!(refused, Err(NotDocument::Control)), "{to}");
    }
}

#[test]
fn a_tool_marks_no_result_only_when_its_own_commands_list_schema_declares_the_field() {
    let schema: serde_json::Value = serde_json::from_str(COMMANDS_LIST_SCHEMA).unwrap();
    let mut before = schema.clone();
    before["$defs"]["CommandRow"]["properties"]
        .as_object_mut()
        .unwrap()
        .remove("prints_no_result")
        .unwrap();
    let assemble = |documents| {
        let commands: CommandsList = serde_json::from_str(WITH_GOALS).unwrap();
        Tool::assemble("shelf 0.1.1", commands, documents).unwrap()
    };

    let marking = assemble(vec![Document::output(
        "shelf/commands-list/v1",
        schema.clone(),
    )]);
    let older = assemble(vec![Document::output("shelf/commands-list/v1", before)]);
    let elsewhere = assemble(vec![Document::output("other/commands-list/v1", schema)]);

    assert!(marking.marks_no_result());
    assert!(!older.marks_no_result());
    assert!(!elsewhere.marks_no_result());
}
