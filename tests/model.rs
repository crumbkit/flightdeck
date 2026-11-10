//! The model of a real crumbkit tool. These tests load shelf and reading,
//! crumbkit's example tools, and small fake tools written as shell scripts.
//! `scripts/install-test-tools.sh` installs shelf and reading under
//! `target/test-tools/bin`, and every test that needs them fails with the
//! script's name when they are missing.

#![cfg(test)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;

use crumbkit::{Failure as _, FieldValue};
use crumbkit_flightdeck::model::weight;
use crumbkit_flightdeck::model::{
    self, Arity, CrumbkitFlag, FindingKind, Kind, LoadError, TOOL_LOAD, TOOL_NOT_FOUND, Tool,
};
use crumbkit_flightdeck::spawn::{HelpLimits, Program, TIME_LIMIT};
use crumbkit_process::Cancel;
use tempfile::TempDir;

/// Returns the path of one of crumbkit's example tools that the install
/// script built, and fails the test when the script has not run.
fn test_tool(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/test-tools/bin")
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    assert!(
        path.is_file(),
        "{} is missing; run scripts/install-test-tools.sh to install shelf and reading",
        path.display()
    );
    path
}

/// The XDG variables that each test sets, and the directory of each under
/// the test's temporary directory.
const XDG: [(&str, &str); 4] = [
    ("XDG_CONFIG_HOME", "config"),
    ("XDG_DATA_HOME", "data"),
    ("XDG_STATE_HOME", "state"),
    ("XDG_CACHE_HOME", "cache"),
];

/// Returns `program` with its XDG directories in `dirs`.
fn within(program: impl Into<std::ffi::OsString>, dirs: &Path) -> Program {
    XDG.iter()
        .fold(Program::new(program), |program, (name, directory)| {
            program.env(name, dirs.join(directory))
        })
}

/// Returns `program` with its XDG directories in a new temporary directory,
/// which lives as long as the returned guard.
fn isolated(program: impl Into<std::ffi::OsString>) -> (TempDir, Program) {
    let dirs = tempfile::tempdir().unwrap();
    let program = within(program, dirs.path());
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

#[cfg(unix)]
#[test]
fn a_tool_that_indents_its_json_loads_the_same_model_and_figures_as_one_that_prints_one_line() {
    let shelf = test_tool("shelf");
    let pretty = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pretty/shelf");
    let dirs = tempfile::tempdir().unwrap();
    // One run of books list gives `self figures list` a count to print.
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs shelf directly so that its usage log counts one run"
    )]
    let ran = std::process::Command::new(&shelf)
        .args(["books", "list"])
        .envs(XDG.map(|(name, directory)| (name, dirs.path().join(directory))))
        .output()
        .unwrap();
    assert!(ran.status.success(), "{ran:?}");

    let one_line = model::load(&within(&shelf, dirs.path()), TIME_LIMIT).unwrap();
    let indented = model::load(&within(&pretty, dirs.path()), TIME_LIMIT).unwrap();

    assert_eq!(
        serde_json::to_value(&indented).unwrap(),
        serde_json::to_value(&one_line).unwrap()
    );
    let books_list = indented
        .commands()
        .iter()
        .find(|command| command.path().to_string() == "books list")
        .unwrap();
    assert_eq!(books_list.runs(), 1);
}

#[test]
fn shelf_has_ten_runnable_commands_its_version_and_its_about_line() {
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
            "self runs list",
        ]
    );
    assert_eq!(shelf.name(), "shelf");
    assert_eq!(shelf.version(), "0.2.0+git-644f776e29ca");
    assert_eq!(
        shelf.about(),
        Some("Browse a small catalogue of public-domain books")
    );
}

#[test]
fn shelf_and_reading_declare_prints_no_result_and_mark_none_of_their_commands() {
    for name in ["shelf", "reading"] {
        let tool = load(name);

        assert!(tool.marks_no_result(), "{name}");
        assert!(
            tool.commands()
                .iter()
                .all(|command| !command.prints_no_result()),
            "{name}"
        );
    }
}

#[test]
fn every_document_that_shelf_prints_has_its_schema_and_books_list_names_its_own() {
    let shelf = load("shelf");

    assert_eq!(shelf.documents().len(), 11);
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
        .map(|link| format!("{} -> {} {}", link.source(), link.target(), link.argument()))
        .collect();
    assert_eq!(
        links,
        [
            "books list -> books show <ID>",
            "books list -> books summarize <ID>",
            "books list -> books remove <ID>",
            "books show -> books summarize <ID>",
            "books show -> books remove <ID>",
            "books remove -> books show <ID>",
            "books remove -> books summarize <ID>",
            "schemas list -> schemas show <ID>",
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
fn shelfs_goal_headings_hold_its_commands_in_help_order() {
    let shelf = load("shelf");

    let headings: Vec<(&str, Vec<String>)> = shelf
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
                vec![
                    "self usage".to_owned(),
                    "self figures list".to_owned(),
                    "self runs list".to_owned(),
                ]
            ),
        ]
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

#[cfg(unix)]
#[test]
fn a_background_process_that_holds_standard_output_open_is_stopped_when_the_tool_exits() {
    // The shell exits at once, and its background `sleep` keeps the shell's
    // standard output open. Reading to the end of the pipe would wait for
    // the `sleep` unless flightdeck stops the whole process group. The time
    // limit is generous for a loaded machine and still well short of the
    // `sleep`, and a load that reached it would fail with another problem.
    let (directory, path) =
        fake_tool("sleep 60 &\necho $! > \"$(dirname \"$0\")/pid\"\necho 'hello'");
    let (_dirs, program) = isolated(path);
    let started = std::time::Instant::now();

    let error = model::load(&program, Duration::from_secs(20)).unwrap_err();

    assert!(
        started.elapsed() < Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(
        field(&error, "problem"),
        Some(FieldValue::from("not_document"))
    );
    let pid = std::fs::read_to_string(directory.path().join("pid")).unwrap();
    assert!(
        !running(pid.trim()),
        "the background sleep {pid} is still running"
    );
}

/// Returns whether the process with id `pid` runs. A process that has ended
/// but that its new parent has not yet reaped does not run.
#[cfg(unix)]
fn running(pid: &str) -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test asks ps about a process that the fake tool started"
    )]
    let output = std::process::Command::new("ps")
        .args(["-o", "stat=", "-p", pid])
        .output()
        .unwrap();
    let state = String::from_utf8_lossy(&output.stdout);
    !state.trim().is_empty() && !state.trim().starts_with('Z')
}

/// A fake tool that answers `--version`, `commands list`, `schemas list` and
/// `schemas show` like a crumbkit tool with one command, `fake things list`,
/// which prints `fake/things-list/v1`. Its `schemas show` prints `shown`,
/// and each answer comes after `pause` seconds.
#[cfg(unix)]
fn fake_crumbkit_tool(shown: &str, pause: &str) -> (TempDir, PathBuf) {
    fake_tool(&format!(
        r#"sleep {pause}
case "$1 $2" in
"--version ") echo 'fake 0.1.0' ;;
"commands list") echo '{{"schema":"fake/commands-list/v1","truncated":false,"data":[{{"path":"fake","about":"A fake tool"}},{{"path":"fake things","about":"Things"}},{{"path":"fake things list","about":"Lists things"}}]}}' ;;
"schemas list") echo '{{"schema":"fake/schemas-list/v1","truncated":false,"data":[{{"id":"fake/things-list/v1","kind":"output","form":"document","commands":["fake things list"]}}]}}' ;;
"schemas show") echo '{shown}' ;;
*) exit 2 ;;
esac"#
    ))
}

#[cfg(unix)]
#[test]
fn a_fake_tool_that_shows_every_schema_it_lists_loads() {
    let (_directory, path) = fake_crumbkit_tool(
        r#"{"schema":"fake/schemas-show/v1","data":[{"id":"fake/things-list/v1","schema":{"type":"object"}}],"failures":[]}"#,
        "0",
    );
    let (_dirs, program) = isolated(path);

    let tool = model::load(&program, TIME_LIMIT).unwrap();

    assert_eq!(tool.version(), "0.1.0");
    let command = tool.command(&["things", "list"]).unwrap();
    assert_eq!(command.result().document(), Some("fake/things-list/v1"));
}

#[cfg(unix)]
#[test]
fn a_tool_whose_schemas_show_leaves_out_a_listed_id_is_tool_load() {
    let (_directory, path) = fake_crumbkit_tool(
        r#"{"schema":"fake/schemas-show/v1","data":[],"failures":[]}"#,
        "0",
    );
    let (_dirs, program) = isolated(path);

    let error = model::load(&program, TIME_LIMIT).unwrap_err();

    assert_eq!(code(&error), &TOOL_LOAD);
    assert_eq!(
        field(&error, "command"),
        Some(FieldValue::from(
            "schemas show --json --no-input -- fake/things-list/v1"
        ))
    );
    assert_eq!(
        field(&error, "problem"),
        Some(FieldValue::from("not_document"))
    );
    assert!(error.to_string().contains("fake/things-list/v1"), "{error}");
}

#[cfg(unix)]
#[test]
fn a_tool_whose_commands_list_names_a_path_with_an_option_word_is_refused_at_load() {
    let (_directory, path) = fake_tool(
        r#"case "$1 $2" in
"--version ") echo 'fake 0.1.0' ;;
"commands list") echo '{"schema":"fake/commands-list/v1","truncated":false,"data":[{"path":"fake","about":"A fake tool"}, {"path":"fake things","about":"Things"},{"path":"fake things --config=/x","about":"Not a command"}]}' ;;
"schemas list") echo '{"schema":"fake/schemas-list/v1","truncated":false,"data":[]}' ;;
*) exit 2 ;;
esac"#,
    );
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
        error.to_string(),
        format!(
            "{} commands list --json --full --all --no-input names a command with an empty word or a word that starts with -",
            program.name().to_string_lossy()
        )
    );
}

#[cfg(unix)]
#[test]
fn a_file_that_may_not_be_run_is_tool_not_found() {
    use std::os::unix::fs::PermissionsExt as _;

    let (_directory, path) = fake_tool("echo 'never runs'");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    let (_dirs, program) = isolated(path);

    let error = model::load(&program, TIME_LIMIT).unwrap_err();

    assert_eq!(code(&error), &TOOL_NOT_FOUND);
}

#[test]
fn a_program_that_cannot_be_started_for_another_reason_is_tool_load_with_no_exit() {
    // A name with a NUL byte cannot pass to the operating system, so the
    // start fails with an error other than a missing or forbidden program.
    let (_dirs, program) = isolated("fake\0tool");

    let error = model::load(&program, TIME_LIMIT).unwrap_err();

    assert_eq!(code(&error), &TOOL_LOAD);
    assert_eq!(field(&error, "problem"), Some(FieldValue::from("start")));
    assert_eq!(field(&error, "exit"), Some(FieldValue::Null));
}

#[cfg(unix)]
#[test]
fn the_time_limit_covers_every_query_of_a_load_together() {
    // Each query takes 0.4 seconds, well within the limit of 1 second, but
    // the load as a whole passes it. Which query passes it depends on how
    // fast the machine starts each one, so the test names none.
    let (_directory, path) = fake_crumbkit_tool(
        r#"{"schema":"fake/schemas-show/v1","data":[{"id":"fake/things-list/v1","schema":{}}]}"#,
        "0.4",
    );
    let (_dirs, program) = isolated(path);

    let error = model::load(&program, Duration::from_secs(1)).unwrap_err();

    assert_eq!(
        field(&error, "problem"),
        Some(FieldValue::from("time_limit"))
    );
}

/// A fake tool like the one of [`fake_crumbkit_tool`] that also mounts
/// `self figures list`, which runs `figures`, a shell command.
#[cfg(unix)]
fn fake_tool_with_figures(figures: &str) -> (TempDir, PathBuf) {
    fake_tool(&format!(
        r#"case "$1 $2 $3" in
"--version  ") echo 'fake 0.1.0' ;;
"commands list "*) echo '{{"schema":"fake/commands-list/v1","truncated":false,"data":[{{"path":"fake","about":"A fake tool"}},{{"path":"fake things","about":"Things"}},{{"path":"fake things list","about":"Lists things"}},{{"path":"fake things count","about":"Counts things"}},{{"path":"fake self","about":"Reviews the use of fake"}},{{"path":"fake self figures","about":"Figures"}},{{"path":"fake self figures list","about":"Lists the figures"}}]}}' ;;
"schemas list "*) echo '{{"schema":"fake/schemas-list/v1","truncated":false,"data":[]}}' ;;
"self figures list") {figures} ;;
*) exit 2 ;;
esac"#
    ))
}

#[cfg(unix)]
#[test]
fn a_tool_that_mounts_self_figures_list_gives_each_listed_command_its_runs() {
    let (_directory, path) = fake_tool_with_figures(
        r#"echo '{"schema":"fake/self-figures-list/v1","data":[{"command":"fake","runs":3},{"command":"things list","runs":7},{"command":"self figures list","runs":5}],"total":3,"truncated":false}'"#,
    );
    let (_dirs, program) = isolated(path);

    let tool = model::load(&program, TIME_LIMIT).unwrap();

    let runs: Vec<(String, u64)> = tool
        .commands()
        .iter()
        .map(|command| (command.path().to_string(), command.runs()))
        .collect();
    assert_eq!(
        runs,
        [
            ("things list".to_owned(), 7),
            ("things count".to_owned(), 0),
            ("self figures list".to_owned(), 0),
        ]
    );
}

#[cfg(unix)]
#[test]
fn a_failing_figures_call_loads_the_tool_with_no_runs() {
    let (_directory, path) = fake_tool_with_figures("echo 'no figures' >&2; exit 1");
    let (_dirs, program) = isolated(path);

    let tool = model::load(&program, TIME_LIMIT).unwrap();

    assert!(tool.commands().iter().all(|command| command.runs() == 0));
}

#[cfg(unix)]
#[test]
fn a_tool_without_self_figures_list_loads_with_no_runs_and_is_never_asked_for_figures() {
    // The fake tool fails any query it does not answer with exit 2, and
    // writes a mark when it is asked for figures.
    let marks = tempfile::tempdir().unwrap();
    let mark = marks.path().join("asked");
    let (_directory, path) = fake_crumbkit_tool(
        r#"{"schema":"fake/schemas-show/v1","data":[{"id":"fake/things-list/v1","schema":{}}]}"#,
        "0",
    );
    let script = std::fs::read_to_string(&path).unwrap().replace(
        "*) exit 2 ;;",
        &format!(
            "\"self figures\") touch '{}' ;;\n*) exit 2 ;;",
            mark.display()
        ),
    );
    std::fs::write(&path, script).unwrap();
    let (_dirs, program) = isolated(path);

    let tool = model::load(&program, TIME_LIMIT).unwrap();

    assert!(tool.commands().iter().all(|command| command.runs() == 0));
    assert!(
        !mark.exists(),
        "the loader asked a tool without figures for them"
    );
}

#[test]
fn opening_shelf_again_counts_the_persons_runs_of_the_description_queries_and_none_of_the_loaders()
{
    let shelf = test_tool("shelf");
    let (dirs, program) = isolated(&shelf);
    shelf_directly(&shelf, dirs.path(), &["commands", "list", "--json"]);
    model::load(&program, TIME_LIMIT).unwrap();

    let opened = model::load(&program, TIME_LIMIT).unwrap();

    let command = |words: &[&str]| opened.command(words).unwrap();
    let counted = |words: &[&str]| serde_json::to_value(command(words)).unwrap()["counted"] == true;
    assert_eq!(command(&["commands", "list"]).runs(), 1);
    assert!(command(&["commands", "list"]).cost().is_some());
    for words in [["schemas", "list"].as_slice(), &["schemas", "show"]] {
        assert_eq!(command(words).runs(), 0, "{words:?}");
        assert!(counted(words), "{words:?}");
    }
}

#[test]
fn opening_shelf_again_counts_none_of_the_loaders_queries_of_the_usage_log() {
    let (_dirs, program) = isolated(test_tool("shelf"));
    model::load(&program, TIME_LIMIT).unwrap();

    let shelf = model::load(&program, TIME_LIMIT).unwrap();

    let runs = |words: &[&str]| shelf.command(words).unwrap().runs();
    for words in [
        ["self", "figures", "list"].as_slice(),
        &["self", "usage"],
        &["self", "runs", "list"],
    ] {
        assert_eq!(runs(words), 0, "{words:?}");
        assert!(shelf.command(words).unwrap().cost().is_none(), "{words:?}");
    }
    assert_eq!(runs(&["books", "list"]), 0);
}

#[test]
fn a_failed_books_show_that_an_agent_ran_again_gives_its_cost_card_the_failure_and_the_retry() {
    let shelf = test_tool("shelf");
    let dirs = tempfile::tempdir().unwrap();
    for _ in 0..2 {
        #[expect(
            clippy::disallowed_methods,
            reason = "the test runs shelf directly so that its usage log holds a failure and a retry"
        )]
        let ran = std::process::Command::new(&shelf)
            .args(["books", "show", "99"])
            .envs(XDG.map(|(name, directory)| (name, dirs.path().join(directory))))
            .output()
            .unwrap();
        assert!(!ran.status.success(), "{ran:?}");
    }

    let tool = model::load(&within(&shelf, dirs.path()), TIME_LIMIT).unwrap();

    let show = tool.command(&["books", "show"]).unwrap();
    let cost = show.cost().unwrap();
    assert_eq!(show.runs(), 2);
    assert_eq!(cost.figures().failed(), Some(2));
    let retries: Vec<(u64, Option<&str>)> = cost
        .findings()
        .unwrap()
        .iter()
        .filter(|finding| finding.kind() == FindingKind::RetryAfterFailure)
        .map(|finding| (finding.runs(), finding.code()))
        .collect();
    assert_eq!(retries, [(1, Some("books::not_found"))]);
    assert!(tool.figures_since().is_some());
}

/// A fake tool like the one of [`fake_tool_with_figures`] that also mounts
/// `self usage`, which runs `usage`, a shell command.
#[cfg(unix)]
fn fake_tool_with_usage(usage: &str) -> (TempDir, PathBuf) {
    fake_tool(&format!(
        r#"case "$1 $2 $3" in
"--version  ") echo 'fake 0.1.0' ;;
"commands list "*) echo '{{"schema":"fake/commands-list/v1","truncated":false,"data":[{{"path":"fake","about":"A fake tool"}},{{"path":"fake things","about":"Things"}},{{"path":"fake things list","about":"Lists things"}},{{"path":"fake self","about":"Reviews the use of fake"}},{{"path":"fake self usage","about":"Reports waste"}},{{"path":"fake self figures","about":"Figures"}},{{"path":"fake self figures list","about":"Lists the figures"}}]}}' ;;
"schemas list "*) echo '{{"schema":"fake/schemas-list/v1","truncated":false,"data":[]}}' ;;
"self figures list") echo '{{"schema":"fake/self-figures-list/v1","data":[{{"command":"things list","runs":7,"failed":2}}],"total":1,"truncated":false}}' ;;
"self usage --json") {usage} ;;
*) exit 2 ;;
esac"#
    ))
}

#[cfg(unix)]
#[test]
fn a_failing_usage_call_leaves_the_figures_without_findings() {
    let (_directory, path) = fake_tool_with_usage("echo 'no usage' >&2; exit 1");
    let (_dirs, program) = isolated(path);

    let tool = model::load(&program, TIME_LIMIT).unwrap();

    let cost = tool.command(&["things", "list"]).unwrap().cost().unwrap();
    assert_eq!(cost.figures().failed(), Some(2));
    assert_eq!(cost.findings(), None);
}

#[cfg(unix)]
#[test]
fn a_usage_call_that_answers_gives_the_command_its_findings() {
    let (_directory, path) = fake_tool_with_usage(
        r#"echo '{"schema":"fake/self-usage/v1","data":[{"finding":"slow command","runs":3,"command":"things list","suggestion":"the 95th percentile of its wall time is 1,200 ms","code":null}],"total":1,"truncated":false}'"#,
    );
    let (_dirs, program) = isolated(path);

    let tool = model::load(&program, TIME_LIMIT).unwrap();

    let cost = tool.command(&["things", "list"]).unwrap().cost().unwrap();
    let findings: Vec<(FindingKind, Option<u64>)> = cost
        .findings()
        .unwrap()
        .iter()
        .map(|finding| (finding.kind(), finding.p95_ms()))
        .collect();
    assert_eq!(findings, [(FindingKind::SlowCommand, None)]);
}

/// Runs `shelf` with `words` and its XDG directories in `dirs`, and returns
/// what it printed.
fn shelf_directly(shelf: &Path, dirs: &Path, words: &[&str]) -> Vec<u8> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs shelf directly to compare with flightdeck's measurement"
    )]
    let ran = std::process::Command::new(shelf)
        .args(words)
        .envs(XDG.map(|(name, directory)| (name, dirs.join(directory))))
        .output()
        .unwrap();
    assert!(ran.status.success(), "{ran:?}");
    ran.stdout
}

#[test]
fn measuring_shelfs_help_pages_counts_the_bytes_that_shelf_h_prints() {
    let (dirs, program) = isolated(test_tool("shelf"));
    let tool = model::load(&program, TIME_LIMIT).unwrap();
    let printed = shelf_directly(&test_tool("shelf"), dirs.path(), &["-h"]);

    let measured = weight::measure(&program, &tool, HelpLimits::new(), &Cancel::new());

    let root = &measured.pages()[0];
    assert!(root.path().is_empty());
    assert_eq!(root.bytes(), Some(u64::try_from(printed.len()).unwrap()));
    assert_eq!(measured.problem(), None);
}

/// Returns the figures of each of shelf's commands, as JSON, from a new load.
fn figures_of(program: &Program) -> Vec<serde_json::Value> {
    model::load(program, TIME_LIMIT)
        .unwrap()
        .commands()
        .iter()
        .map(|command| serde_json::to_value(command.cost().map(model::Cost::figures)).unwrap())
        .collect()
}

#[test]
fn measuring_shelfs_help_pages_leaves_the_persons_usage_figures_unchanged() {
    let shelf = test_tool("shelf");
    let (dirs, program) = isolated(&shelf);
    shelf_directly(&shelf, dirs.path(), &["books", "list"]);
    let tool = model::load(&program, TIME_LIMIT).unwrap();
    let before = figures_of(&program);

    let measured = weight::measure(&program, &tool, HelpLimits::new(), &Cancel::new());

    assert!(measured.pages().iter().all(|page| page.bytes().is_some()));
    assert_eq!(figures_of(&program), before);
    // A help page that shelf prints outside the measurement is counted, so
    // the measurement's pages would have shown had they reached the log.
    shelf_directly(&shelf, dirs.path(), &["books", "list", "-h"]);
    assert_ne!(figures_of(&program), before);
}
