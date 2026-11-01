//! The runner against real crumbkit tools. These tests run shelf and
//! reading, crumbkit's example tools, and `unruly`, a fake tool under
//! `tests/fixtures/runner/` whose commands misbehave on purpose.
//! `scripts/install-test-tools.sh` installs shelf and reading under
//! `target/test-tools/bin`, and every test that needs them fails with the
//! script's name when they are missing.

#![cfg(test)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crumbkit::{Failure as _, FieldValue};
use crumbkit_flightdeck::model::{self, Tool};
use crumbkit_flightdeck::runner::{
    Clock, ConfirmationId, Escalation, Limits, Mode, Outcome, RUN_BUSY, RUN_CLOSED,
    RUN_CONFIRMATION, RUN_NEEDS_CONFIRMATION, RUN_NO_DOCUMENT, RUN_NOT_RUNNING, RUN_OUTPUT_LIMIT,
    RUN_READ_ONLY, RUN_REFUSED, RUN_SPAWN, RunEvent, RunId, RunRequest, Runner, Settings,
    SystemClock, Value,
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

/// Returns the path of the fake tool `unruly`.
fn unruly() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runner/unruly")
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

/// A clock that stands still until a test moves it forward.
#[derive(Debug)]
struct TestClock {
    start: Instant,
    passed: Mutex<Duration>,
}

impl TestClock {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            start: SystemClock.now(),
            passed: Mutex::new(Duration::ZERO),
        })
    }

    fn advance(&self, by: Duration) {
        *self.passed.lock().unwrap() += by;
    }
}

impl Clock for TestClock {
    fn now(&self) -> Instant {
        self.start + *self.passed.lock().unwrap()
    }
}

/// The settings of every test: a short grace after each signal, so a test
/// of cancel waits a fraction of a second instead of seven.
fn settings() -> Settings {
    Settings::new().escalation(Escalation::new(
        Duration::from_millis(200),
        Duration::from_millis(200),
    ))
}

/// A runner of one tool, and the directories and clock it uses.
struct Session {
    _dirs: TempDir,
    clock: Arc<TestClock>,
    runner: Runner,
}

/// Loads `program`'s model and returns a runner of it in `mode` with
/// `settings`.
fn session_of(dirs: TempDir, program: Program, mode: Mode, settings: Settings) -> Session {
    let tool = model::load(&program, TIME_LIMIT).unwrap();
    let clock = TestClock::new();
    let runner = Runner::new(tool, program, mode, clock.clone(), settings).unwrap();
    Session {
        _dirs: dirs,
        clock,
        runner,
    }
}

/// Returns a runner of one of crumbkit's example tools.
fn session(name: &str, mode: Mode) -> Session {
    let (dirs, program) = isolated(test_tool(name));
    session_of(dirs, program, mode, settings())
}

/// Returns a runner of the fake tool `unruly` with `settings`.
fn unruly_session(settings: Settings) -> Session {
    let (dirs, program) = isolated(unruly());
    session_of(dirs, program, Mode::Full, settings)
}

/// Returns the request for the command `path` with no values.
fn request(path: &[&str]) -> RunRequest {
    RunRequest::new(path.iter().copied())
}

/// Returns `Value::One` of `text`.
fn one(text: &str) -> Value {
    Value::One(text.to_owned())
}

/// Returns `Value::Many` of `texts`.
fn many(texts: &[&str]) -> Value {
    Value::Many(texts.iter().map(|&text| text.to_owned()).collect())
}

/// Reads every event of `run` and returns its lines and its outcome.
fn finish(run: impl Iterator<Item = RunEvent>) -> (Vec<String>, Outcome) {
    let mut lines = Vec::new();
    for event in run {
        match event {
            RunEvent::Line(line) => lines.push(line),
            RunEvent::Outcome(outcome) => return (lines, outcome),
        }
    }
    panic!("the run ended without an outcome");
}

/// Returns the code of a refusal or a failure.
fn code(failure: &impl crumbkit::Failure) -> &'static crumbkit::Code {
    failure.code()
}

/// Returns the value of the failure's field `name`.
fn field(failure: &impl crumbkit::Failure, name: &str) -> Option<FieldValue<'static>> {
    failure
        .fields()
        .into_iter()
        .find(|field| field.name() == name)
        .map(|field| field.value().clone().into_owned())
}

/// Returns the ids of the books that `books list --all` prints.
fn book_ids(runner: &Runner) -> Vec<u64> {
    let run = runner
        .run(&request(&["books", "list"]).value("--all", Value::Flag(true)))
        .unwrap();
    let (lines, outcome) = finish(run);
    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
    let document: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    document["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|book| book["id"].as_u64().unwrap())
        .collect()
}

/// Previews `request`, reads its dry run to the end, and returns the
/// confirmation id that the preview issued.
fn confirmation_of(runner: &Runner, request: &RunRequest) -> ConfirmationId {
    let mut preview = runner.preview(request).unwrap();
    let (_, outcome) = finish(&mut preview);
    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
    preview.confirmation().unwrap()
}

/// Returns the request to remove the book `id`.
fn remove(id: &str) -> RunRequest {
    request(&["books", "remove"]).value("<ID>", many(&[id]))
}

#[test]
fn a_hidden_command_an_alias_and_a_group_are_refused_because_commands_list_omits_them() {
    let shelf = session("shelf", Mode::Full);

    for path in [&["aliases", "list"][..], &["books", "ls"], &["books"]] {
        let failure = shelf.runner.run(&request(path)).unwrap_err();

        assert_eq!(code(&failure), &RUN_REFUSED, "{path:?}");
        assert_eq!(
            field(&failure, "reason"),
            Some(FieldValue::from("unknown_command"))
        );
    }
}

#[test]
fn an_argument_that_the_command_does_not_take_is_refused() {
    let shelf = session("shelf", Mode::Full);

    let failure = shelf
        .runner
        .run(&request(&["books", "list"]).value("--from", one("ids.txt")))
        .unwrap_err();

    assert_eq!(code(&failure), &RUN_REFUSED);
    assert_eq!(
        field(&failure, "argument"),
        Some(FieldValue::from("--from"))
    );
    assert_eq!(
        field(&failure, "reason"),
        Some(FieldValue::from("unknown_argument"))
    );
}

#[test]
fn a_value_outside_the_possible_values_is_refused() {
    let unruly = unruly_session(settings());

    let failure = unruly
        .runner
        .run(&request(&["child", "pick"]).value("--colour", one("blue")))
        .unwrap_err();

    assert_eq!(code(&failure), &RUN_REFUSED);
    assert_eq!(
        field(&failure, "reason"),
        Some(FieldValue::from("not_possible"))
    );
}

#[test]
fn crumbkit_flags_that_flightdeck_sets_itself_are_refused_from_the_page() {
    let shelf = session("shelf", Mode::Full);

    for flag in ["--json", "--yes", "--dry-run", "--fields", "--no-input"] {
        let value = if flag == "--fields" {
            many(&["id"])
        } else {
            Value::Flag(true)
        };
        let failure = shelf
            .runner
            .preview(&remove("2").value(flag, value))
            .unwrap_err();

        assert_eq!(code(&failure), &RUN_REFUSED, "{flag}");
        assert_eq!(
            field(&failure, "reason"),
            Some(FieldValue::from("reserved"))
        );
    }
}

#[test]
fn an_option_value_that_starts_with_a_dash_reaches_the_tool_as_the_options_value() {
    let shelf = session("shelf", Mode::Full);

    let run = shelf
        .runner
        .run(&request(&["books", "list"]).value("--limit", one("-1")))
        .unwrap();
    let (_, outcome) = finish(run);

    // shelf refuses -1 as a value of --limit, which it reads only when -1
    // arrives as that value.
    let Outcome::ToolFailure(failure) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(failure.code(), "shelf::cli::usage");
    assert_eq!(failure.fields()["argument"], "--limit");
    assert_eq!(failure.fields()["value"], "-1");
}

#[test]
fn a_positional_value_after_the_separator_is_never_read_as_a_flag() {
    let shelf = session("shelf", Mode::Full);

    let run = shelf
        .runner
        .run(&request(&["books", "show"]).value("<ID>", many(&["--all"])))
        .unwrap();
    let (_, outcome) = finish(run);

    let Outcome::ToolFailure(failure) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(failure.fields()["argument"], "<ID>...");
    assert_eq!(failure.fields()["value"], "--all");
}

#[test]
fn books_list_runs_at_once_and_its_one_line_is_shelfs_listing_document() {
    let shelf = session("shelf", Mode::Full);

    let run = shelf.runner.run(&request(&["books", "list"])).unwrap();
    assert_eq!(run.line(), "shelf books list");
    let (lines, outcome) = finish(run);

    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
    assert_eq!(lines.len(), 1);
    let document: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(document["schema"], "shelf/books-list/v1");
    assert_eq!(document["total"], 6);
}

#[test]
fn books_remove_without_a_confirmation_is_refused_with_needs_confirmation() {
    let shelf = session("shelf", Mode::Full);

    let failure = shelf.runner.run(&remove("2")).unwrap_err();

    assert_eq!(code(&failure), &RUN_NEEDS_CONFIRMATION);
    assert_eq!(book_ids(&shelf.runner), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn a_preview_runs_a_dry_run_that_changes_nothing() {
    let shelf = session("shelf", Mode::Full);

    let mut preview = shelf.runner.preview(&remove("2")).unwrap();
    assert_eq!(preview.line(), "shelf books remove 2");
    assert!(preview.confirmation().is_none());
    let (lines, outcome) = finish(&mut preview);

    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
    assert!(preview.confirmation().is_some());
    let document: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(document["dry_run"], true);
    assert_eq!(document["data"][0]["title"], "Frankenstein");
    assert_eq!(book_ids(&shelf.runner), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn a_confirmation_removes_the_book_once_and_its_id_is_refused_the_second_time() {
    let shelf = session("shelf", Mode::Full);
    let confirmation = confirmation_of(&shelf.runner, &remove("2"));

    let run = shelf.runner.confirm(&confirmation, &remove("2")).unwrap();
    let (_, outcome) = finish(run);
    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
    assert_eq!(book_ids(&shelf.runner), [1, 3, 4, 5, 6]);

    let failure = shelf
        .runner
        .confirm(&confirmation, &remove("2"))
        .unwrap_err();
    assert_eq!(code(&failure), &RUN_CONFIRMATION);
    assert_eq!(field(&failure, "reason"), Some(FieldValue::from("unknown")));
}

#[test]
fn an_id_issued_for_removing_book_2_cannot_remove_book_3() {
    let shelf = session("shelf", Mode::Full);
    let confirmation = confirmation_of(&shelf.runner, &remove("2"));

    let failure = shelf
        .runner
        .confirm(&confirmation, &remove("3"))
        .unwrap_err();

    assert_eq!(code(&failure), &RUN_CONFIRMATION);
    assert_eq!(
        field(&failure, "reason"),
        Some(FieldValue::from("mismatch"))
    );
    assert_eq!(book_ids(&shelf.runner), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn an_id_older_than_ten_minutes_is_refused_as_expired() {
    let shelf = session("shelf", Mode::Full);
    let confirmation = confirmation_of(&shelf.runner, &remove("2"));

    shelf.clock.advance(Duration::from_secs(10 * 60));
    let failure = shelf
        .runner
        .confirm(&confirmation, &remove("2"))
        .unwrap_err();

    assert_eq!(code(&failure), &RUN_CONFIRMATION);
    assert_eq!(field(&failure, "reason"), Some(FieldValue::from("expired")));
    assert_eq!(book_ids(&shelf.runner), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn read_only_mode_refuses_books_remove_at_preview_and_at_confirm() {
    let shelf = session("shelf", Mode::ReadOnly);
    let full = session("shelf", Mode::Full);
    let confirmation = confirmation_of(&full.runner, &remove("2"));

    let at_preview = shelf.runner.preview(&remove("2")).unwrap_err();
    let at_confirm = shelf
        .runner
        .confirm(&confirmation, &remove("2"))
        .unwrap_err();

    assert_eq!(code(&at_preview), &RUN_READ_ONLY);
    assert_eq!(code(&at_confirm), &RUN_READ_ONLY);
    assert_eq!(book_ids(&shelf.runner), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn readings_stream_delivers_lines_while_the_child_still_runs() {
    // The list prints about 7 MB, far more than the pipe and the runner's
    // queue hold, so reading cannot finish while the test reads only the
    // first line. A cancel after that line therefore finds it running.
    const ENTRIES: usize = 100_000;
    let lists = tempfile::tempdir().unwrap();
    let titles: String = (1..=ENTRIES).map(|n| format!("Book {n}\n")).collect();
    std::fs::write(lists.path().join("to-read.txt"), titles).unwrap();
    let (dirs, program) = isolated(test_tool("reading"));
    // The graces are long enough that a loaded machine does not reach
    // SIGKILL before the tool handles SIGINT.
    let reading = session_of(
        dirs,
        program.env("READING_DIR", lists.path()),
        Mode::Full,
        Settings::new().escalation(Escalation::new(
            Duration::from_secs(10),
            Duration::from_secs(10),
        )),
    );
    let mut run = reading
        .runner
        .run(&request(&["entries", "stream"]).value("--list", one("to-read")))
        .unwrap();

    let Some(RunEvent::Line(first)) = run.next() else {
        panic!("the stream's first event is not a line");
    };
    reading.runner.cancel(run.id()).unwrap();
    let (rest, outcome) = finish(run);

    let first: serde_json::Value = serde_json::from_str(&first).unwrap();
    assert_eq!(first["data"]["title"], "Book 1");
    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
    assert!(rest.len() + 1 < ENTRIES);
}

#[test]
fn cancel_ends_a_child_that_ignores_interrupt_and_terminate_and_keeps_the_lines_that_arrived() {
    let unruly = unruly_session(settings());
    let mut run = unruly.runner.run(&request(&["child", "hang"])).unwrap();
    let mut lines = Vec::new();
    while lines.len() < 2 {
        let Some(RunEvent::Line(line)) = run.next() else {
            panic!("the run ended before it printed two lines");
        };
        lines.push(line);
    }

    unruly.runner.cancel(run.id()).unwrap();
    let (rest, outcome) = finish(run);

    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
    assert_eq!(lines.len() + rest.len(), 2);
}

#[test]
fn a_child_that_exits_101_without_a_document_is_no_document_with_its_standard_error() {
    let unruly = unruly_session(settings());

    let run = unruly.runner.run(&request(&["child", "panic"])).unwrap();
    let (lines, outcome) = finish(run);

    let Outcome::Failed(failure) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(lines.is_empty());
    assert_eq!(code(&failure), &RUN_NO_DOCUMENT);
    assert_eq!(field(&failure, "exit"), Some(FieldValue::from(101)));
    assert_eq!(
        field(&failure, "stderr"),
        Some(FieldValue::list([
            "thread 'main' panicked at src/main.rs:3:5:",
            "the catalogue is gone",
        ]))
    );
}

#[test]
fn a_program_that_does_not_exist_is_refused_with_spawn() {
    let (dirs, program) = isolated(test_tool("shelf"));
    let tool: Tool = model::load(&program, TIME_LIMIT).unwrap();
    let missing = Program::new(dirs.path().join("no-such-tool"));
    let runner = Runner::new(tool, missing, Mode::Full, TestClock::new(), settings()).unwrap();

    let failure = runner.run(&request(&["books", "list"])).unwrap_err();

    assert_eq!(code(&failure), &RUN_SPAWN);
}

#[test]
fn a_line_longer_than_the_line_limit_ends_the_run_with_output_limit() {
    let unruly = unruly_session(settings().limits(Limits::new(1000, 1_000_000)));

    let run = unruly.runner.run(&request(&["child", "flood"])).unwrap();
    let (lines, outcome) = finish(run);

    let Outcome::Failed(failure) = outcome else {
        panic!("{outcome:?}");
    };
    assert!(lines.is_empty());
    assert_eq!(code(&failure), &RUN_OUTPUT_LIMIT);
    assert_eq!(field(&failure, "limit"), Some(FieldValue::from("line")));
}

#[test]
fn a_run_past_the_most_runs_at_once_is_refused_until_one_ends() {
    let unruly = unruly_session(settings().most_runs(1));
    let hang = unruly.runner.run(&request(&["child", "hang"])).unwrap();

    let failure = unruly.runner.run(&request(&["child", "pick"])).unwrap_err();
    assert_eq!(code(&failure), &RUN_BUSY);

    unruly.runner.cancel(hang.id()).unwrap();
    finish(hang);
    let (_, outcome) = finish(unruly.runner.run(&request(&["child", "pick"])).unwrap());
    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
}

#[test]
fn cancel_of_a_run_that_has_ended_is_refused_with_not_running() {
    let unruly = unruly_session(settings());
    let run = unruly.runner.run(&request(&["child", "pick"])).unwrap();
    let id = run.id();
    finish(run);

    let failure = unruly.runner.cancel(id).unwrap_err();

    assert_eq!(code(&failure), &RUN_NOT_RUNNING);
}

#[test]
fn two_runs_of_one_runner_share_the_trace_id_and_differ_in_parent_id() {
    let unruly = unruly_session(settings());
    let traceparent = || {
        let (lines, _) = finish(unruly.runner.run(&request(&["child", "trace"])).unwrap());
        let document: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        document["data"]["traceparent"].as_str().unwrap().to_owned()
    };

    let first = traceparent();
    let second = traceparent();

    let parts =
        |traceparent: &str| -> Vec<String> { traceparent.split('-').map(str::to_owned).collect() };
    let (first, second) = (parts(&first), parts(&second));
    for parts in [&first, &second] {
        assert_eq!(parts.len(), 4);
        assert_eq!((parts[0].as_str(), parts[3].as_str()), ("00", "01"));
        assert_eq!((parts[1].len(), parts[2].len()), (32, 16));
    }
    assert_eq!(first[1], second[1]);
    assert_ne!(first[2], second[2]);
}

#[test]
fn a_failed_dry_run_issues_no_confirmation_and_its_failure_names_the_dry_run_without_the_value() {
    let unruly = unruly_session(settings());
    let wipe = request(&["child", "wipe"]).value("<NAME>", one("bad"));

    let mut preview = unruly.runner.preview(&wipe).unwrap();
    let (_, outcome) = finish(&mut preview);

    assert!(preview.confirmation().is_none());
    let Outcome::Failed(failure) = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(code(&failure), &RUN_NO_DOCUMENT);
    assert_eq!(
        field(&failure, "line"),
        Some(FieldValue::from("unruly child wipe --dry-run bad"))
    );
    assert_eq!(
        field(&failure, "path"),
        Some(FieldValue::from("child wipe"))
    );
    let display = failure.to_string();
    let recovery = format!("{:?}", failure.recovery());
    assert!(!display.contains("bad"), "{display}");
    assert!(!recovery.contains("bad"), "{recovery}");
}

#[test]
fn a_confirmation_refused_as_busy_keeps_its_id_for_when_a_run_ends() {
    let unruly = unruly_session(settings().most_runs(1));
    let wipe = request(&["child", "wipe"]).value("<NAME>", one("old"));
    let confirmation = confirmation_of(&unruly.runner, &wipe);
    let hang = unruly.runner.run(&request(&["child", "hang"])).unwrap();

    let failure = unruly.runner.confirm(&confirmation, &wipe).unwrap_err();
    assert_eq!(code(&failure), &RUN_BUSY);

    unruly.runner.cancel(hang.id()).unwrap();
    finish(hang);
    let (_, outcome) = finish(unruly.runner.confirm(&confirmation, &wipe).unwrap());
    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
}

#[test]
fn a_command_that_declares_itself_read_only_and_destructive_needs_a_confirmation() {
    let unruly = unruly_session(settings());
    let both = request(&["child", "both"]);

    let failure = unruly.runner.run(&both).unwrap_err();
    assert_eq!(code(&failure), &RUN_NEEDS_CONFIRMATION);

    // The command takes no --dry-run, so its preview issues the id at once
    // and runs nothing.
    let mut preview = unruly.runner.preview(&both).unwrap();
    let confirmation = preview.confirmation().unwrap();
    assert!(preview.next().is_none());
    let (_, outcome) = finish(unruly.runner.confirm(&confirmation, &both).unwrap());
    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
}

#[test]
fn a_run_ends_soon_after_its_child_exits_while_a_process_in_another_session_holds_its_output() {
    let unruly = unruly_session(settings());
    let started = Instant::now();

    let (lines, outcome) = finish(unruly.runner.run(&request(&["child", "orphan"])).unwrap());

    // The process in the other session sleeps for ten seconds.
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
    assert_eq!(lines.len(), 1);
}

#[test]
fn cancel_and_shutdown_end_a_run_whose_caller_stopped_reading_it() {
    let unruly = unruly_session(settings());
    let mut run = unruly.runner.run(&request(&["child", "chatter"])).unwrap();
    let Some(RunEvent::Line(first)) = run.next() else {
        panic!("the run's first event is not a line");
    };

    // The child prints more lines than the runner's queue holds, and the
    // test reads none of them before the cancel and the shutdown.
    unruly.runner.cancel(run.id()).unwrap();
    unruly.runner.shutdown();
    let (rest, outcome) = finish(run);

    assert_eq!(first, "1");
    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
    assert!(rest.len() < 1000);
}

#[test]
fn shutdown_lets_a_run_finish_within_its_grace_and_cancels_a_run_that_outlasts_it() {
    let unruly = unruly_session(settings().shutdown_grace(Duration::from_secs(1)));
    let nap = unruly.runner.run(&request(&["child", "nap"])).unwrap();
    let hang = unruly.runner.run(&request(&["child", "hang"])).unwrap();
    let started = Instant::now();

    unruly.runner.shutdown();

    // The grace, the two graces of the cancel and the second in which the
    // readers drain the pipes come to under three seconds, and the child
    // that outlasts the grace sleeps for thirty.
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
    let (_, napped) = finish(nap);
    let (_, hung) = finish(hang);
    assert!(matches!(napped, Outcome::Result), "{napped:?}");
    assert!(matches!(hung, Outcome::Cancelled), "{hung:?}");
}

#[test]
fn a_runner_that_has_shut_down_refuses_every_new_run_with_closed() {
    let unruly = unruly_session(settings());
    unruly.runner.shutdown();

    let failure = unruly.runner.run(&request(&["child", "pick"])).unwrap_err();

    assert_eq!(code(&failure), &RUN_CLOSED);
}

#[test]
fn a_run_id_is_read_only_from_decimal_digits() {
    assert_eq!("3".parse::<RunId>().unwrap().to_string(), "3");
    for text in ["+3", "-3", " 3", ""] {
        let failure = text.parse::<RunId>().unwrap_err();

        assert_eq!(code(&failure), &RUN_NOT_RUNNING, "{text:?}");
    }
}

/// Starts a run of `request`, retrying while the runner is busy, and fails
/// the test when no place frees within ten seconds.
fn run_when_free(runner: &Runner, request: &RunRequest) -> crumbkit_flightdeck::runner::Run {
    let started = Instant::now();
    loop {
        match runner.run(request) {
            Ok(run) => return run,
            Err(failure) if code(&failure) == &RUN_BUSY => {
                assert!(
                    started.elapsed() < Duration::from_secs(10),
                    "no place freed"
                );
            }
            Err(failure) => panic!("{failure}"),
        }
    }
}

#[test]
fn cancel_ends_a_child_that_moved_into_flightdecks_own_process_group() {
    let unruly = unruly_session(settings());
    let mut run = unruly.runner.run(&request(&["child", "adopt"])).unwrap();
    let Some(RunEvent::Line(_)) = run.next() else {
        panic!("the run's first event is not a line");
    };
    let started = Instant::now();

    unruly.runner.cancel(run.id()).unwrap();
    let (_, outcome) = finish(run);

    // The child ignores SIGINT and SIGTERM and sleeps for thirty seconds, so
    // only a SIGKILL sent to the child itself ends it this soon.
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
}

#[test]
fn dropping_a_run_stops_its_child_and_frees_its_place() {
    let unruly = unruly_session(settings().most_runs(2));
    let hang = request(&["child", "hang"]);
    drop(unruly.runner.run(&hang).unwrap());
    drop(unruly.runner.run(&hang).unwrap());

    let run = run_when_free(&unruly.runner, &request(&["child", "pick"]));

    let (_, outcome) = finish(run);
    assert!(matches!(outcome, Outcome::Result), "{outcome:?}");
}

#[test]
fn a_cancelled_run_whose_queue_was_full_ends_by_the_last_line_that_its_child_printed() {
    let unruly = unruly_session(settings());
    let mut run = unruly
        .runner
        .run(&request(&["child", "interrupt"]))
        .unwrap();
    let Some(RunEvent::Line(_)) = run.next() else {
        panic!("the run's first event is not a line");
    };

    // On SIGINT the child prints 400 documents, more than the queue holds,
    // and then a line that is no document. The shutdown waits for the run
    // to end while the test reads nothing.
    unruly.runner.cancel(run.id()).unwrap();
    unruly.runner.shutdown();
    let (lines, outcome) = finish(run);

    assert!(lines.len() < 400);
    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
}

#[test]
fn a_cancelled_dry_run_whose_child_printed_its_document_is_cancelled_and_issues_no_confirmation() {
    let unruly = unruly_session(settings());
    let wipe = request(&["child", "wipe"]).value("<NAME>", one("slow"));
    let mut preview = unruly.runner.preview(&wipe).unwrap();
    let Some(RunEvent::Line(_)) = preview.next() else {
        panic!("the dry run's first event is not a line");
    };

    unruly.runner.cancel(preview.run_id().unwrap()).unwrap();
    let (_, outcome) = finish(&mut preview);

    // The dry run exits with 0 on SIGINT after its document.
    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
    assert!(preview.confirmation().is_none());
}

#[test]
fn dropping_a_runner_cancels_its_runs_and_waits_for_them() {
    let Session { _dirs, runner, .. } =
        unruly_session(settings().shutdown_grace(Duration::from_millis(100)));
    let hang = runner.run(&request(&["child", "hang"])).unwrap();
    let started = Instant::now();

    drop(runner);
    let (_, outcome) = finish(hang);

    // The child ignores SIGINT and SIGTERM and sleeps for thirty seconds, so
    // only the shutdown of the dropped runner ends it this soon. The grace
    // and the cancel's graces come to under a second and a half.
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
}

#[test]
fn a_run_can_be_cancelled_while_a_shutdown_waits_for_it() {
    let unruly = unruly_session(
        settings()
            .most_runs(1)
            .shutdown_grace(Duration::from_secs(30)),
    );
    let hang = unruly.runner.run(&request(&["child", "hang"])).unwrap();
    let started = Instant::now();

    std::thread::scope(|scope| {
        scope.spawn(|| unruly.runner.shutdown());
        // The only place is taken, so a run is refused as busy until the
        // shutdown closes the runner.
        loop {
            let failure = unruly.runner.run(&request(&["child", "pick"])).unwrap_err();
            if code(&failure) == &RUN_CLOSED {
                break;
            }
        }
        unruly.runner.cancel(hang.id()).unwrap();
    });

    // The shutdown's grace is thirty seconds, and the cancel ends it sooner.
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    let (_, outcome) = finish(hang);
    assert!(matches!(outcome, Outcome::Cancelled), "{outcome:?}");
}
