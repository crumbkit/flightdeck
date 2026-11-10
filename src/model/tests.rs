use super::figures::{FiguresList, Usage};
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

/// shelf's `self figures list --json --full --all` after a few runs.
const FIGURES: &str = include_str!("../../tests/fixtures/shelf/self-figures-list.json");

/// shelf's `self usage --json --full` after the same runs.
const USAGE: &str = include_str!("../../tests/fixtures/shelf/self-usage.json");

/// Returns shelf's tool with the costs that `figures` and `usage` give it.
fn shelf_with_costs(figures: &str, usage: Option<&str>) -> Tool {
    let commands: CommandsList = serde_json::from_str(WITH_GOALS).unwrap();
    let mut tool = Tool::assemble("shelf 0.1.1", commands, Vec::new()).unwrap();
    let figures = FiguresList::read(figures.as_bytes());
    let usage = usage.and_then(|usage| Usage::read(usage.as_bytes()));
    if let Some(figures) = figures {
        tool.add_costs(&figures, usage.as_ref());
    }
    tool
}

/// Returns `document` with `edit` applied to its parsed JSON.
fn edited(document: &str, edit: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut value: serde_json::Value = serde_json::from_str(document).unwrap();
    edit(&mut value);
    value.to_string()
}

#[test]
fn books_show_reads_every_figure_and_its_findings_in_the_order_shelf_prints_them() {
    let tool = shelf_with_costs(FIGURES, Some(USAGE));

    let show = tool.command(&["books", "show"]).unwrap();
    let cost = show.cost().unwrap();
    let figures = cost.figures();
    assert_eq!(show.runs(), 5);
    assert_eq!(
        (
            figures.help_pages(),
            figures.help_before(),
            figures.refused(),
            figures.failed(),
            figures.p95_ms(),
            figures.p95_out_bytes(),
        ),
        (Some(1), Some(1), Some(1), Some(2), Some(0), Some(74))
    );
    let findings: Vec<(&str, u64, Option<&str>)> = cost
        .findings()
        .unwrap()
        .iter()
        .map(|finding| (finding.finding(), finding.runs(), finding.code()))
        .collect();
    assert_eq!(
        findings,
        [
            ("one id at a time", 4, None),
            ("identical repeat", 2, None),
            ("retry after failure", 2, Some("shelf::books::not_found")),
            ("usage error", 1, None),
        ]
    );
    assert_eq!(tool.figures_since(), Some("2026-11-03"));
}

#[test]
fn a_command_that_the_figures_do_not_list_and_the_loaders_own_queries_have_no_cost() {
    let tool = shelf_with_costs(FIGURES, Some(USAGE));

    for words in [
        ["books", "remove"].as_slice(),
        &["self", "figures", "list"],
        &["self", "usage"],
    ] {
        let command = tool.command(words).unwrap();
        assert!(command.cost().is_none(), "{words:?}");
        assert_eq!(command.runs(), 0, "{words:?}");
    }
}

#[test]
fn fields_that_a_newer_crumbkit_adds_are_ignored_and_the_known_ones_still_read() {
    let figures = edited(FIGURES, |document| {
        for row in document["data"].as_array_mut().unwrap() {
            row["p50_ms"] = serde_json::json!(3);
        }
        document["summary"]["window"] = serde_json::json!("7d");
    });
    let usage = edited(USAGE, |document| {
        for row in document["data"].as_array_mut().unwrap() {
            row["severity"] = serde_json::json!("high");
        }
    });

    let tool = shelf_with_costs(&figures, Some(&usage));

    let cost = tool.command(&["books", "show"]).unwrap().cost().unwrap();
    assert_eq!(cost.figures().failed(), Some(2));
    assert_eq!(cost.findings().unwrap().len(), 4);
}

#[test]
fn a_figure_that_an_older_crumbkit_leaves_out_reads_as_not_reported() {
    let figures = edited(FIGURES, |document| {
        for row in document["data"].as_array_mut().unwrap() {
            let row = row.as_object_mut().unwrap();
            for name in [
                "help_pages",
                "help_before",
                "refused",
                "failed",
                "p95_ms",
                "p95_out_bytes",
            ] {
                row.remove(name);
            }
        }
        document.as_object_mut().unwrap().remove("summary");
    });

    let tool = shelf_with_costs(&figures, None);

    let show = tool.command(&["books", "show"]).unwrap();
    let figures = show.cost().unwrap().figures();
    assert_eq!(show.runs(), 5);
    assert_eq!(
        (
            figures.help_pages(),
            figures.help_before(),
            figures.refused(),
            figures.failed(),
            figures.p95_ms(),
            figures.p95_out_bytes(),
        ),
        (None, None, None, None, None, None)
    );
    assert_eq!(tool.figures_since(), None);
}

#[test]
fn figures_of_another_schema_give_no_command_a_cost() {
    let figures = edited(FIGURES, |document| {
        document["schema"] = serde_json::json!("shelf/books-list/v1");
    });

    let tool = shelf_with_costs(&figures, Some(USAGE));

    assert!(
        tool.commands()
            .iter()
            .all(|command| command.cost().is_none())
    );
    assert_eq!(tool.figures_since(), None);
}

#[test]
fn findings_of_another_schema_leave_the_figures_without_findings() {
    let usage = edited(USAGE, |document| {
        document["schema"] = serde_json::json!("shelf/self-figures-list/v1");
    });

    let tool = shelf_with_costs(FIGURES, Some(&usage));

    let cost = tool.command(&["books", "show"]).unwrap().cost().unwrap();
    assert_eq!(cost.figures().failed(), Some(2));
    assert_eq!(cost.findings(), None);
}

#[test]
fn a_finding_row_without_a_code_field_reads_as_no_code() {
    let usage = edited(USAGE, |document| {
        for row in document["data"].as_array_mut().unwrap() {
            row.as_object_mut().unwrap().remove("code");
        }
    });

    let tool = shelf_with_costs(FIGURES, Some(&usage));

    let cost = tool.command(&["books", "show"]).unwrap().cost().unwrap();
    assert!(cost.findings().unwrap().iter().all(|f| f.code().is_none()));
}

#[test]
fn a_since_that_is_not_a_day_is_left_out() {
    let figures = edited(FIGURES, |document| {
        document["summary"]["since"] = serde_json::json!("last week");
    });

    let tool = shelf_with_costs(&figures, None);

    assert_eq!(tool.figures_since(), None);
    assert!(tool.command(&["books", "show"]).unwrap().cost().is_some());
}

#[test]
fn figures_that_give_no_command_a_cost_name_no_day() {
    let figures = edited(FIGURES, |document| {
        document["data"]
            .as_array_mut()
            .unwrap()
            .retain(|row| row["command"] == "self usage");
    });

    let tool = shelf_with_costs(&figures, Some(USAGE));

    assert!(
        tool.commands()
            .iter()
            .all(|command| command.cost().is_none())
    );
    assert_eq!(tool.figures_since(), None);
}
