use serde_json::{Value as Json, json};

use super::{MOST_CHOICES, read, source_request};
use crate::model::{Command, CommandsList, Tool};
use crate::runner::{RunRequest, Value};

/// shelf's `commands list` document at the commit that the install script
/// pins.
const SHELF: &str = include_str!("../../../tests/fixtures/shelf/commands-list.json");

/// The fake tool unruly's `commands list` document.
const UNRULY: &str = include_str!("../../../tests/fixtures/runner/unruly-commands-list.json");

fn tool(document: &str, version: &str) -> Tool {
    let commands: CommandsList = serde_json::from_str(document).unwrap();
    Tool::assemble(version, commands, Vec::new()).unwrap()
}

fn books_list(shelf: &Tool) -> &Command {
    shelf.command(&["books", "list"]).unwrap()
}

/// Returns a `books list` document of `rows`.
fn listing(rows: Vec<Json>, truncated: bool) -> String {
    json!({"schema": "shelf/books-list/v1", "data": rows, "truncated": truncated}).to_string()
}

/// Returns a book row with the id `id`.
fn book(id: Json) -> Json {
    json!({"id": id, "title": "Emma", "author": "Jane Austen", "year": 1815, "pages": 474})
}

fn values(document: &str) -> Vec<String> {
    let shelf = tool(SHELF, "shelf 0.1.0");
    read(books_list(&shelf), document)
        .unwrap()
        .choices()
        .iter()
        .map(|choice| choice.value().to_owned())
        .collect()
}

#[test]
fn a_source_with_more_rows_than_the_most_choices_gives_the_first_of_them_marked_truncated() {
    let shelf = tool(SHELF, "shelf 0.1.0");
    let rows = (1..=MOST_CHOICES + 1).map(|id| book(json!(id))).collect();

    let choices = read(books_list(&shelf), &listing(rows, false)).unwrap();

    assert_eq!(choices.choices().len(), MOST_CHOICES);
    assert_eq!(
        choices.choices().last().unwrap().value(),
        MOST_CHOICES.to_string()
    );
    assert!(choices.truncated());
}

#[test]
fn a_listing_that_the_tool_cut_short_gives_choices_marked_truncated() {
    let shelf = tool(SHELF, "shelf 0.1.0");

    let choices = read(books_list(&shelf), &listing(vec![book(json!(1))], true)).unwrap();

    assert_eq!(choices.choices().len(), 1);
    assert!(choices.truncated());
}

#[test]
fn an_id_of_text_or_a_number_is_a_choice_and_any_other_id_is_left_out() {
    let rows = vec![
        book(json!("b-7")),
        book(json!(null)),
        book(json!(2)),
        book(json!({"id": 3})),
        book(json!(true)),
        json!({"title": "No id"}),
    ];

    assert_eq!(values(&listing(rows, false)), ["b-7", "2"]);
}

#[test]
fn a_label_holds_the_compact_columns_other_than_id_and_leaves_out_the_long_ones() {
    let shelf = tool(SHELF, "shelf 0.1.0");

    let choices = read(books_list(&shelf), &listing(vec![book(json!(1))], false)).unwrap();

    assert_eq!(choices.labels(), ["title", "author", "year"]);
    assert_eq!(
        choices.choices()[0].label(),
        [json!("Emma"), json!("Jane Austen"), json!(1815)]
    );
}

#[test]
fn output_that_is_no_listing_gives_no_choices() {
    let shelf = tool(SHELF, "shelf 0.1.0");

    for output in [
        "",
        "not json",
        r#"{"schema":"shelf/books-show/v1","data":{"id":1}}"#,
        "{\"schema\":\"shelf/books-list/v1\",\"data\":[],\"truncated\":false}\nmore",
    ] {
        assert_eq!(read(books_list(&shelf), output), None, "{output}");
    }
}

#[test]
fn the_source_runs_with_all_when_it_takes_it_and_with_nothing_else() {
    let shelf = tool(SHELF, "shelf 0.1.0");
    let unruly = tool(UNRULY, "unruly 0.1.0");
    let roster = unruly.command(&["child", "roster"]).unwrap();

    assert_eq!(
        source_request(books_list(&shelf)),
        RunRequest::new(["books", "list"]).value("--all", Value::Flag(true))
    );
    assert_eq!(source_request(roster), RunRequest::new(["child", "roster"]));
}
