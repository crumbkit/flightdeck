use super::{CommandsList, Effects, Listing, StepCommand, Tool};

/// shelf's `commands list` document with the `goals` field that a newer
/// crumbkit prints on a group's row.
const WITH_GOALS: &str = include_str!("../../tests/fixtures/newer/commands-list-with-goals.json");

/// shelf's `books list --json` document with the `next` field that a newer
/// crumbkit prints on a listing.
const WITH_NEXT: &str = include_str!("../../tests/fixtures/newer/books-list-with-next.json");

#[test]
fn goals_on_the_root_row_give_the_headings_in_help_order() {
    let commands: CommandsList = serde_json::from_str(WITH_GOALS).unwrap();

    let tool = Tool::assemble("shelf 0.1.0", commands, Vec::new()).unwrap();

    let headings: Vec<(&str, Vec<String>)> = tool
        .headings()
        .iter()
        .map(|heading| {
            let paths = heading.commands().iter().map(ToString::to_string).collect();
            (heading.title(), paths)
        })
        .collect();
    assert_eq!(
        headings,
        [
            (
                "Browse the catalogue",
                vec![
                    "books list".to_owned(),
                    "books show".to_owned(),
                    "books summarize".to_owned(),
                    "books remove".to_owned(),
                ]
            ),
            (
                "Explore the commands",
                vec![
                    "commands list".to_owned(),
                    "schemas list".to_owned(),
                    "schemas show".to_owned(),
                ]
            ),
            (
                "Review how shelf is used",
                vec!["self usage".to_owned(), "self figures list".to_owned()]
            ),
        ]
    );
}

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
