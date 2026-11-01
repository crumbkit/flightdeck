//! The model of a real crumbkit tool. These tests load shelf and reading,
//! crumbkit's example tools, and small fake tools written as shell scripts.
//! `scripts/install-test-tools.sh` installs shelf and reading under
//! `target/test-tools/bin`, and every test that needs them fails with the
//! script's name when they are missing.

#![cfg(test)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crumbkit::{Failure as _, FieldValue};
use crumbkit_flightdeck::model::{
    self, Arity, CrumbkitFlag, Kind, LoadError, Origin, TOOL_LOAD, TOOL_NOT_FOUND, Tool,
};
use crumbkit_flightdeck::spawn::{Program, TIME_LIMIT};
use tempfile::TempDir;

/// Returns the path of one of crumbkit's example tools that the install
/// script built, and fails the test when the script has not run.
fn test_tool(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/test-tools/bin")
        .join(name);
    assert!(
        path.is_file(),
        "{} is missing; run scripts/install-test-tools.sh to install shelf and reading",
        path.display()
    );
    path
}

/// Returns `program` with its XDG directories in a new temporary directory,
/// which lives as long as the returned guard.
fn isolated(program: impl Into<std::ffi::OsString>) -> (TempDir, Program) {
    let dirs = tempfile::tempdir().unwrap();
    let mut program = Program::new(program);
    for (name, directory) in [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_CACHE_HOME", "cache"),
    ] {
        program = program.env(name, dirs.path().join(directory));
    }
    (dirs, program)
}

/// Loads the model of one of crumbkit's example tools.
fn load(name: &str) -> Tool {
    let (_dirs, program) = isolated(test_tool(name));
    model::load(&program, TIME_LIMIT).unwrap()
}

/// Writes a shell script named `fake` that runs `body`, and returns its
/// directory and path. The kernel runs the script through its `#!` line, so
/// flightdeck still starts it without a shell of its own.
#[cfg(unix)]
fn fake_tool(body: &str) -> (TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fake");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (directory, path)
}

/// Returns the code of a load failure.
fn code(error: &LoadError) -> &'static crumbkit::Code {
    error.code()
}

/// Returns the value of the failure's field `name`.
fn field(error: &LoadError, name: &str) -> Option<FieldValue<'static>> {
    error
        .fields()
        .into_iter()
        .find(|field| field.name() == name)
        .map(|field| field.value().clone().into_owned())
}

#[test]
fn shelf_has_nine_runnable_commands_its_version_and_its_about_line() {
    let shelf = load("shelf");

    let paths: Vec<String> = shelf
        .commands()
        .iter()
        .map(|command| command.path().to_string())
        .collect();
    assert_eq!(
        paths,
        [
            "books list",
            "books show",
            "books summarize",
            "books remove",
            "commands list",
            "schemas list",
            "schemas show",
            "self usage",
            "self figures list",
        ]
    );
    assert_eq!(shelf.name(), "shelf");
    assert_eq!(shelf.version(), "0.1.0+git-31245d63b750");
    assert_eq!(
        shelf.about(),
        Some("Browse a small catalogue of public-domain books")
    );
}

#[test]
fn every_document_that_shelf_prints_has_its_schema_and_books_list_names_its_own() {
    let shelf = load("shelf");

    assert_eq!(shelf.documents().len(), 10);
    let books_list = shelf.command(&["books", "list"]).unwrap();
    let id = books_list.result().document().unwrap();
    assert_eq!(id, "shelf/books-list/v1");
    let document = shelf.document(id).unwrap();
    assert_eq!(
        document.schema()["title"],
        serde_json::json!("shelf/books-list/v1")
    );
}

#[test]
fn books_show_takes_many_ids_or_a_file_of_ids_from_from() {
    let shelf = load("shelf");
    let show = shelf.command(&["books", "show"]).unwrap();

    let ids = show
        .arguments()
        .iter()
        .find(|argument| argument.name() == "<ID>")
        .unwrap();
    assert_eq!(ids.kind(), Kind::Positional);
    assert_eq!(
        ids.arity(),
        &Arity::Many {
            value_names: vec!["ID".to_owned()]
        }
    );
    assert!(ids.required());
    assert_eq!(ids.one_of(), ["--from"]);
}

#[test]
fn the_nine_crumbkit_flags_are_sorted_apart_from_the_tools_own_arguments() {
    let shelf = load("shelf");

    let mut crumbkit_flags = BTreeSet::new();
    for command in shelf.commands() {
        crumbkit_flags.extend(command.crumbkit_flags().map(|(flag, _)| flag));
        for argument in command.tool_arguments() {
            assert!(
                CrumbkitFlag::from_name(argument.name()).is_none(),
                "{} of {} is crumbkit's",
                argument.name(),
                command.path()
            );
        }
    }
    assert_eq!(
        crumbkit_flags.into_iter().collect::<Vec<_>>(),
        CrumbkitFlag::ALL
    );

    let tool_arguments = |path: &[&str]| -> Vec<String> {
        let command = shelf.command(path).unwrap();
        command
            .tool_arguments()
            .map(|argument| argument.name().to_owned())
            .collect()
    };
    assert_eq!(tool_arguments(&["books", "list"]), Vec::<String>::new());
    assert_eq!(tool_arguments(&["books", "show"]), ["<ID>", "--from"]);
    assert_eq!(tool_arguments(&["books", "remove"]), ["<ID>"]);
}

#[test]
fn a_command_with_an_id_column_links_it_to_each_sibling_that_takes_an_id() {
    let shelf = load("shelf");

    let links: Vec<String> = shelf
        .links()
        .iter()
        .map(|link| {
            assert_eq!(link.origin(), Origin::Inferred);
            format!(
                "{}.{} -> {} {}",
                link.source(),
                link.column(),
                link.target(),
                link.argument()
            )
        })
        .collect();
    assert_eq!(
        links,
        [
            "books list.id -> books show <ID>",
            "books list.id -> books summarize <ID>",
            "books list.id -> books remove <ID>",
            "books show.id -> books summarize <ID>",
            "books show.id -> books remove <ID>",
            "books remove.id -> books show <ID>",
            "books remove.id -> books summarize <ID>",
            "schemas list.id -> schemas show <ID>",
        ]
    );
}

#[test]
fn a_command_whose_positional_is_not_an_id_is_never_a_link_target() {
    let shelf = load("shelf");
    let commands_list = shelf.command(&["commands", "list"]).unwrap();

    assert!(
        commands_list
            .arguments()
            .iter()
            .any(|argument| argument.kind() == Kind::Positional)
    );
    assert!(
        shelf
            .links()
            .iter()
            .all(|link| link.target() != commands_list.path())
    );
}

#[test]
fn a_tool_without_goals_has_one_heading_named_after_it_with_every_command() {
    let reading = load("reading");

    let headings = reading.headings();
    assert_eq!(headings.len(), 1);
    let heading = headings.first().unwrap();
    assert_eq!(heading.title(), "reading");
    let commands: Vec<_> = reading.commands().iter().map(|c| c.path()).collect();
    assert_eq!(heading.commands().iter().collect::<Vec<_>>(), commands);
}

#[test]
fn a_program_that_does_not_exist_is_tool_not_found() {
    let directory = tempfile::tempdir().unwrap();
    let (_dirs, program) = isolated(directory.path().join("no-such-tool"));

    let error = model::load(&program, TIME_LIMIT).unwrap_err();

    assert_eq!(code(&error), &TOOL_NOT_FOUND);
}

#[cfg(unix)]
#[test]
fn a_tool_that_prints_text_instead_of_json_is_tool_load_with_its_exit_and_standard_error() {
    let (_directory, path) = fake_tool("echo 'warming up' >&2\necho 'hello'");
    let (_dirs, program) = isolated(path);

    let error = model::load(&program, TIME_LIMIT).unwrap_err();

    assert_eq!(code(&error), &TOOL_LOAD);
    assert_eq!(
        field(&error, "command"),
        Some(FieldValue::from(
            "commands list --json --full --all --no-input"
        ))
    );
    assert_eq!(
        field(&error, "problem"),
        Some(FieldValue::from("not_document"))
    );
    assert_eq!(field(&error, "exit"), Some(FieldValue::from(0)));
    assert_eq!(
        field(&error, "stderr"),
        Some(FieldValue::list(["warming up"]))
    );
}

#[cfg(unix)]
#[test]
fn a_tool_that_never_exits_is_stopped_at_the_time_limit() {
    // The script's `sleep` is a child of the shell, so stopping the shell
    // alone would leave `sleep` holding the output pipe open.
    let (_directory, path) = fake_tool("sleep 30");
    let (_dirs, program) = isolated(path);

    let error = model::load(&program, Duration::from_millis(200)).unwrap_err();

    assert_eq!(code(&error), &TOOL_LOAD);
    assert_eq!(
        field(&error, "problem"),
        Some(FieldValue::from("time_limit"))
    );
    assert_eq!(field(&error, "exit"), Some(FieldValue::Null));
}
