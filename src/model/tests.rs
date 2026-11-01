use super::{CommandsList, Effects, Listing, NotDocument, StepCommand, Tool};

/// shelf's `commands list` document, with `goals` on its root row.
const WITH_GOALS: &str = include_str!("../../tests/fixtures/shelf/commands-list.json");

/// shelf's `books list --json` document, with a next step that calls a
/// shelf command and one that runs another program first.
const WITH_NEXT: &str = include_str!("../../tests/fixtures/shelf/books-list-with-run-step.json");

#[test]
fn null_effects_are_unknown() {
    let effects: Effects = serde_json::from_str("null").unwrap();

    assert_eq!(effects, Effects::Unknown);
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
