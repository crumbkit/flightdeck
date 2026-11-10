use super::command::CommandPath;
use super::day::Day;
use super::figures::{FiguresList, FindingKind, Usage, is_built_in};
use super::runs::History;
use super::{CommandsList, Document, Effects, Listing, NotDocument, StepCommand, Tool};

/// shelf's `commands list` document, with `goals` on its root row.
const WITH_GOALS: &str = include_str!("../../tests/fixtures/shelf/commands-list.json");

/// shelf's `books list --json` document, with a next step that calls a
/// shelf command and one that runs another program first.
const WITH_NEXT: &str = include_str!("../../tests/fixtures/shelf/books-list-with-run-step.json");

/// unruly's `commands list` document, whose `child silent` is marked
/// `prints_no_result`.
const UNRULY: &str = include_str!("../../tests/fixtures/runner/unruly-commands-list.json");

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
fn a_command_path_with_a_word_that_starts_with_a_dash_or_an_empty_word_is_refused_when_the_model_loads()
 {
    for to in [
        r#""path":"shelf books --config=/x""#,
        r#""path":"shelf books  show""#,
    ] {
        let from = r#""path":"shelf books show""#;
        let commands: CommandsList = serde_json::from_str(&WITH_GOALS.replace(from, to)).unwrap();

        let refused = Tool::assemble("shelf 0.1.1", commands, Vec::new());

        assert!(matches!(refused, Err(NotDocument::OptionWord)), "{to}");
    }
}

#[test]
fn a_goal_or_a_schemas_list_path_that_does_not_parse_is_skipped() {
    let from = r#""commands":["books"]"#;
    assert!(WITH_GOALS.contains(from));
    let commands: CommandsList =
        serde_json::from_str(&WITH_GOALS.replace(from, r#""commands":["books","-x"]"#)).unwrap();
    let schemas: super::SchemasList = serde_json::from_value(serde_json::json!({
        "schema": "shelf/schemas-list/v1",
        "truncated": false,
        "data": [{
            "id": "shelf/books-list/v1",
            "kind": "output",
            "form": "document",
            "commands": ["shelf --books list", "shelf books list"],
        }],
    }))
    .unwrap();
    let documents = schemas
        .into_rows()
        .into_iter()
        .map(|row| Document::new(row, serde_json::json!({})))
        .collect();

    let tool = Tool::assemble("shelf 0.1.1", commands, documents).unwrap();

    let browse = &tool.headings()[0];
    assert_eq!(browse.title(), "Browse the catalogue");
    assert_eq!(browse.commands().len(), 4);
    assert_eq!(
        tool.document("shelf/books-list/v1").unwrap().commands(),
        [CommandPath::from_row("shelf books list").unwrap()]
    );
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
    let usage = usage.and_then(|usage| Usage::read(usage.as_bytes(), "shelf"));
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
    let findings: Vec<(FindingKind, u64, Option<&str>, Option<u64>)> = cost
        .findings()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                finding.kind(),
                finding.runs(),
                finding.code(),
                finding.p95_out_bytes(),
            )
        })
        .collect();
    assert_eq!(
        findings,
        [
            (FindingKind::Other, 4, None, Some(74)),
            (FindingKind::IdenticalRepeat, 2, None, Some(74)),
            (
                FindingKind::RetryAfterFailure,
                2,
                Some("books::not_found"),
                Some(74)
            ),
            (FindingKind::UsageError, 1, None, Some(74)),
        ]
    );
    assert_eq!(tool.figures_since(), Day::parse("2026-11-03"));
    let list = tool.command(&["books", "list"]).unwrap().cost().unwrap();
    assert_eq!(
        (list.figures().p50_ms(), list.figures().p50_out_bytes()),
        (Some(6), Some(235))
    );
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
fn the_loaders_queries_of_the_usage_log_are_marked_as_not_counted_and_every_other_command_as_counted()
 {
    let tool = shelf_with_costs(FIGURES, Some(USAGE));
    let counted = |words: &[&str]| {
        serde_json::to_value(tool.command(words).unwrap()).unwrap()["counted"].clone()
    };

    for words in [["self", "figures", "list"].as_slice(), &["self", "usage"]] {
        assert_eq!(counted(words), serde_json::json!(false), "{words:?}");
    }
    for words in [
        ["commands", "list"].as_slice(),
        &["schemas", "list"],
        &["schemas", "show"],
        &["books", "remove"],
    ] {
        assert_eq!(counted(words), serde_json::json!(true), "{words:?}");
    }
}

#[test]
fn crumbkits_own_commands_are_marked_built_in_and_the_tools_commands_are_not() {
    let tool = shelf_with_costs(FIGURES, Some(USAGE));
    let built_in = |words: &[&str]| {
        serde_json::to_value(tool.command(words).unwrap()).unwrap()["built_in"].clone()
    };

    for words in [
        ["commands", "list"].as_slice(),
        &["schemas", "list"],
        &["schemas", "show"],
        &["self", "figures", "list"],
        &["self", "usage"],
    ] {
        assert_eq!(built_in(words), serde_json::json!(true), "{words:?}");
    }
    for words in [["books", "list"].as_slice(), &["books", "remove"]] {
        assert_eq!(built_in(words), serde_json::json!(false), "{words:?}");
    }
    // shelf mounts no aliases, so the rule is checked on the path alone.
    assert!(is_built_in(
        &CommandPath::from_row("shelf aliases list").unwrap()
    ));
}

#[test]
fn the_model_sends_whether_a_command_prints_no_result() {
    let commands: CommandsList = serde_json::from_str(UNRULY).unwrap();
    let schema = serde_json::from_str(COMMANDS_LIST_SCHEMA).unwrap();
    let documents = vec![Document::output("unruly/commands-list/v1", schema)];
    let tool = Tool::assemble("unruly 0.1.0", commands, documents).unwrap();
    let prints_no_result = |words: &[&str]| {
        serde_json::to_value(tool.command(words).unwrap()).unwrap()["prints_no_result"].clone()
    };

    assert_eq!(
        prints_no_result(&["child", "silent"]),
        serde_json::json!(true)
    );
    assert_eq!(
        prints_no_result(&["child", "panic"]),
        serde_json::json!(false)
    );
}

#[test]
fn fields_that_a_newer_crumbkit_adds_are_ignored_and_the_known_ones_still_read() {
    let figures = edited(FIGURES, |document| {
        for row in document["data"].as_array_mut().unwrap() {
            row["p75_ms"] = serde_json::json!(3);
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
                "p50_ms",
                "p95_ms",
                "p50_out_bytes",
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
            figures.p50_ms(),
            figures.p50_out_bytes(),
        ),
        (None, None, None, None, None, None, None, None)
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
fn a_since_that_is_no_day_of_the_calendar_is_left_out_and_the_figures_still_read() {
    for since in [
        serde_json::json!("last week"),
        serde_json::json!("2026-13-01"),
        serde_json::json!("2026-02-30"),
        serde_json::json!(20_261_103),
    ] {
        let figures = edited(FIGURES, |document| {
            document["summary"]["since"] = since.clone();
        });

        let tool = shelf_with_costs(&figures, None);

        assert_eq!(tool.figures_since(), None, "{since}");
        assert!(tool.command(&["books", "show"]).unwrap().cost().is_some());
    }
}

#[test]
fn a_day_reads_only_from_a_date_of_the_calendar_and_serializes_as_its_parts() {
    assert_eq!(
        serde_json::to_value(Day::parse("2028-02-29").unwrap()).unwrap(),
        serde_json::json!({"year": 2028, "month": 2, "day": 29})
    );
    for text in [
        "2026-02-29",
        "2026-00-10",
        "2026-04-31",
        "2026-4-03",
        "2026-04-03-01",
        "",
    ] {
        assert_eq!(Day::parse(text), None, "{text}");
    }
}

#[test]
fn a_finding_of_a_kind_flightdeck_does_not_know_reads_as_other() {
    let usage = edited(USAGE, |document| {
        document["data"][0]["finding"] = serde_json::json!("");
        document["data"][1]["finding"] = serde_json::json!("mostly waiting");
    });

    let tool = shelf_with_costs(FIGURES, Some(&usage));

    let cost = tool.command(&["books", "show"]).unwrap().cost().unwrap();
    let kinds: Vec<FindingKind> = cost.findings().unwrap().iter().map(|f| f.kind()).collect();
    assert_eq!(kinds[..2], [FindingKind::Other, FindingKind::Other]);
}

#[test]
fn a_command_sends_the_runs_of_its_cost_and_zero_without_one() {
    let tool = shelf_with_costs(FIGURES, Some(USAGE));

    let reply = |words: &[&str]| serde_json::to_value(tool.command(words).unwrap()).unwrap();
    let show = reply(&["books", "show"]);
    let remove = reply(&["books", "remove"]);

    assert_eq!(show["runs"], 5);
    assert_eq!(show["cost"]["figures"]["runs"], 5);
    assert_eq!(show["cost"]["findings"][2]["kind"], "retry_after_failure");
    assert_eq!(remove["runs"], 0);
    assert_eq!(remove["cost"], serde_json::Value::Null);
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

/// One past the largest count the page reads exactly, 2^53.
const UNSAFE: u64 = 9_007_199_254_740_992;

/// Returns every number in `value` and its children.
fn numbers(value: &serde_json::Value) -> Vec<u64> {
    match value {
        serde_json::Value::Number(number) => number.as_u64().into_iter().collect(),
        serde_json::Value::Array(items) => items.iter().flat_map(numbers).collect(),
        serde_json::Value::Object(fields) => fields.values().flat_map(numbers).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn a_count_too_large_for_the_page_reads_as_not_reported_and_the_model_still_loads() {
    let figures = edited(FIGURES, |document| {
        for row in document["data"].as_array_mut().unwrap() {
            if row["command"] == "books show" {
                row["failed"] = serde_json::json!(UNSAFE);
                row["p95_out_bytes"] = serde_json::json!(u64::MAX);
            }
            if row["command"] == "books list" {
                row["runs"] = serde_json::json!(UNSAFE);
            }
        }
    });
    let usage = edited(USAGE, |document| {
        document["data"][0]["runs"] = serde_json::json!(UNSAFE);
        document["data"][1]["figures"]["p95_ms"] = serde_json::json!(UNSAFE);
    });

    let tool = shelf_with_costs(&figures, Some(&usage));

    let show = tool.command(&["books", "show"]).unwrap().cost().unwrap();
    assert_eq!(show.figures().failed(), None);
    assert_eq!(show.figures().p95_out_bytes(), None);
    assert_eq!(show.figures().refused(), Some(1));
    let findings = show.findings().unwrap();
    assert_eq!(findings.len(), 3);
    assert_eq!(findings[0].kind(), FindingKind::IdenticalRepeat);
    assert_eq!(findings[0].p95_ms(), None);
    assert!(tool.command(&["books", "list"]).unwrap().cost().is_none());
    let reply = serde_json::to_value(&tool).unwrap();
    assert!(numbers(&reply).into_iter().all(|number| number < UNSAFE));
}

#[test]
fn a_count_of_the_wrong_json_type_reads_as_not_reported_for_that_figure_alone() {
    let figures = edited(FIGURES, |document| {
        for row in document["data"].as_array_mut().unwrap() {
            if row["command"] == "books show" {
                row["failed"] = serde_json::json!("2");
                row["p50_ms"] = serde_json::json!(true);
                row["p95_ms"] = serde_json::json!(-1);
            }
        }
    });
    let usage = edited(USAGE, |document| {
        document["data"][1]["figures"]["p95_out_bytes"] = serde_json::json!([74]);
    });

    let tool = shelf_with_costs(&figures, Some(&usage));

    let show = tool.command(&["books", "show"]).unwrap().cost().unwrap();
    assert_eq!(show.figures().failed(), None);
    assert_eq!(show.figures().p50_ms(), None);
    assert_eq!(show.figures().p95_ms(), None);
    assert_eq!(show.figures().refused(), Some(1));
    assert!(tool.command(&["books", "list"]).unwrap().cost().is_some());
    let findings = show.findings().unwrap();
    assert_eq!(findings.len(), 4);
    assert_eq!(findings[1].p95_out_bytes(), None);
}

/// shelf's `self runs list --json --full --all` after the same runs.
const RUNS: &str = include_str!("../../tests/fixtures/shelf/self-runs-list.json");

/// A run's session, step, command, help, exit, code and out_bytes, the
/// fields that the tests compare.
type Compared<'a> = (
    u64,
    u64,
    &'a str,
    bool,
    Option<u8>,
    Option<&'a str>,
    Option<u64>,
);

/// Returns each run as the fields that the tests compare.
fn runs_of(history: &History) -> Vec<Compared<'_>> {
    history
        .runs()
        .iter()
        .map(|run| {
            (
                run.session(),
                run.step(),
                run.command(),
                run.help(),
                run.exit(),
                run.code(),
                run.out_bytes(),
            )
        })
        .collect()
}

#[test]
fn the_runs_list_reads_each_run_in_order_without_the_loaders_queries_or_the_tools_name() {
    let history = History::read(RUNS.as_bytes(), "shelf").unwrap();

    assert_eq!(
        runs_of(&history),
        [
            (1, 1, "books list", false, Some(0), None, Some(235)),
            (
                1,
                2,
                "books show",
                false,
                Some(1),
                Some("books::not_found"),
                Some(0)
            ),
            (
                1,
                3,
                "books show",
                false,
                Some(1),
                Some("books::not_found"),
                Some(0)
            ),
            (1, 4, "books show", true, Some(0), None, None),
            (1, 5, "books show", false, Some(0), None, Some(74)),
            (1, 6, "books show", false, Some(0), None, Some(74)),
            (1, 7, "books show", false, Some(2), Some("cli::usage"), None),
        ]
    );
    let first = &history.runs()[0];
    assert_eq!(first.caller(), "claude-code");
    assert!(first.flags().is_empty());
    let reply = serde_json::to_value(first).unwrap();
    assert!(reply["at"].as_u64().unwrap() > 1_700_000_000, "{reply}");
}

#[test]
fn a_run_row_that_does_not_read_is_left_out_and_the_others_still_read() {
    let runs = edited(RUNS, |document| {
        document["data"][0]["at"] = serde_json::json!("yesterday");
        document["data"][1]["ms"] = serde_json::json!("0");
        document["data"][2]["session"] = serde_json::json!(UNSAFE);
        document["data"][3]["flags"] = serde_json::json!("--json");
        document["data"][4]["exit"] = serde_json::json!(256);
        document["data"][5]["step"] = serde_json::json!(0);
        document["data"][6]["p99_ms"] = serde_json::json!(4);
    });

    let history = History::read(runs.as_bytes(), "shelf").unwrap();

    let steps: Vec<u64> = history.runs().iter().map(|run| run.step()).collect();
    assert_eq!(steps, [7]);
}

#[test]
fn the_loaders_version_query_is_left_out_and_the_tools_other_root_runs_stay() {
    let root = |help: bool, exit: u8, flags: serde_json::Value| {
        serde_json::json!({
            "session": 2, "step": 1, "at": "2026-11-10T10:14:45Z",
            "caller": "person", "command": "shelf", "flags": flags,
            "help": help, "exit": exit, "code": null, "ms": 1, "out_bytes": null,
        })
    };
    let runs = edited(RUNS, |document| {
        let data = document["data"].as_array_mut().unwrap();
        data.clear();
        data.push(root(false, 0, serde_json::json!([])));
        data.push(root(true, 0, serde_json::json!([])));
        data.push(root(false, 2, serde_json::json!([])));
        data.push(root(false, 0, serde_json::json!(["--json"])));
    });

    let history = History::read(runs.as_bytes(), "shelf").unwrap();

    let kept: Vec<(bool, Option<u8>, usize)> = history
        .runs()
        .iter()
        .map(|run| (run.help(), run.exit(), run.flags().len()))
        .collect();
    assert_eq!(
        kept,
        [(true, Some(0), 0), (false, Some(2), 0), (false, Some(0), 1)]
    );
}

#[test]
fn runs_of_another_schema_or_not_json_give_no_history() {
    let other = edited(RUNS, |document| {
        document["schema"] = serde_json::json!("shelf/self-usage/v1");
    });

    assert!(History::read(other.as_bytes(), "shelf").is_none());
    assert!(History::read(b"no runs", "shelf").is_none());
}

#[test]
fn a_tool_sends_its_history_and_null_without_one() {
    let mut tool = shelf_with_costs(FIGURES, Some(USAGE));
    let none = serde_json::to_value(&tool).unwrap();
    tool.set_history(History::read(RUNS.as_bytes(), "shelf"));

    let reply = serde_json::to_value(&tool).unwrap();

    assert_eq!(none["history"], serde_json::Value::Null);
    assert_eq!(reply["history"].as_array().unwrap().len(), 7);
    assert_eq!(reply["history"][1]["code"], "books::not_found");
}

#[test]
fn a_tool_that_is_not_found_offers_a_name_on_path_and_a_path_as_two_repairs() {
    use crumbkit::{Failure as _, Recovery};

    let failure = super::LoadError::NotFound {
        tool: "shelf".to_owned(),
        source: std::io::Error::from(std::io::ErrorKind::NotFound),
    };

    assert_eq!(
        failure.recovery(),
        Recovery::hint("give the name of a program on PATH")
    );
    assert_eq!(
        failure.alternatives(),
        [Recovery::hint("give the path of a program")]
    );
}

#[test]
fn a_state_directory_that_cannot_be_made_is_a_load_failure_that_says_so() {
    use crumbkit::Failure as _;

    let failure = super::LoadError::State {
        tool: "shelf".to_owned(),
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    };

    assert_eq!(failure.code(), &super::TOOL_LOAD);
    assert_eq!(
        failure.to_string(),
        "cannot make a temporary state directory for shelf's queries"
    );
}

/// Returns the pages of shelf's commands, at most `most`, as text.
fn shelf_pages(most: usize) -> (Vec<String>, usize) {
    let commands: CommandsList = serde_json::from_str(WITH_GOALS).unwrap();
    let tool = Tool::assemble("shelf 0.1.1", commands, Vec::new()).unwrap();
    let pages = super::weight::pages(&tool, most);
    let paths = pages.paths().iter().map(|path| path.join(" ")).collect();
    (paths, pages.left_out())
}

#[test]
fn the_help_pages_of_a_tool_with_nested_groups_are_each_distinct_page_once_in_guide_order() {
    let (paths, left_out) = shelf_pages(super::weight::MOST_PAGES);

    assert_eq!(paths.first().map(String::as_str), Some(""));
    assert_eq!(
        paths.iter().take(3).map(String::as_str).collect::<Vec<_>>(),
        ["", "books", "books list"]
    );
    let distinct: std::collections::BTreeSet<&String> = paths.iter().collect();
    assert_eq!(distinct.len(), paths.len());
    for page in ["self", "self figures", "self figures list"] {
        assert!(paths.iter().any(|path| path == page), "{page} in {paths:?}");
    }
    assert_eq!(left_out, 0);
}

#[test]
fn the_help_pages_stop_at_the_cap_and_count_the_commands_left_out() {
    let (all, _) = shelf_pages(super::weight::MOST_PAGES);
    let commands: CommandsList = serde_json::from_str(WITH_GOALS).unwrap();
    let tool = Tool::assemble("shelf 0.1.1", commands, Vec::new()).unwrap();

    let (paths, left_out) = shelf_pages(3);

    assert_eq!(paths, ["", "books", "books list"]);
    assert!(all.len() > 3);
    assert_eq!(left_out, tool.commands().len() - 1);
}
