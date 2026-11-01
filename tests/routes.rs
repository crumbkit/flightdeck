//! The routes that run commands, through the page's server on a real
//! socket: `POST /api/runs`, `POST /api/previews`,
//! `POST /api/confirmations/<id>` and `POST /api/runs/<id>/cancel`. The
//! tests run shelf and reading, which `scripts/install-test-tools.sh`
//! installs, and `unruly`, the runner tests' fake tool.

#![cfg(test)]

#[path = "support/http.rs"]
#[expect(dead_code, reason = "these tests use only part of the shared client")]
mod http;
#[path = "support/serving.rs"]
#[expect(dead_code, reason = "these tests use only part of the shared harness")]
mod serving;

use std::time::{Duration, Instant};

use crumbkit_flightdeck::runner;
use crumbkit_flightdeck::server::Settings;
use http::{Call, Lines, Reply};
use serde_json::{Value, json};
use serving::{Running, Setup, Then, serving, test_tool, unruly};

/// A page that has exchanged its launch code.
struct Page<'a> {
    running: &'a Running<'a>,
    token: String,
}

impl<'a> Page<'a> {
    fn of(running: &'a Running<'a>) -> Self {
        Self {
            running,
            token: running.token(),
        }
    }

    /// Returns the `POST` of `body` to `path` with the token.
    fn post(&self, path: &str, body: &Value) -> Call {
        Call::post(self.running.port, path, &body.to_string()).bearer(&self.token)
    }

    /// Sends `body` to `path` and returns the whole reply.
    fn send(&self, path: &str, body: &Value) -> Reply {
        self.post(path, body).send(self.running.port)
    }

    /// Sends `body` to `path` and returns the stream, after checking that
    /// it is a stream.
    fn stream(&self, path: &str, body: &Value) -> Lines {
        let lines = self.post(path, body).stream(self.running.port);
        assert_eq!(lines.reply.status, 200);
        assert_eq!(
            lines.reply.header("Content-Type"),
            Some("application/x-ndjson")
        );
        lines
    }

    /// Runs `body` and returns every line of its stream.
    fn run(&self, body: &Value) -> Vec<Value> {
        self.stream("/api/runs", body).rest()
    }

    /// Previews `body` and returns the confirmation id that its stream
    /// carries.
    fn confirmation(&self, body: &Value) -> String {
        let lines = self.stream("/api/previews", body).rest();
        let ended = lines.last().unwrap();
        assert_eq!(ended["outcome"]["kind"], "result", "{lines:?}");
        ended["confirmation"].as_str().unwrap().to_owned()
    }

    /// Returns the ids of the books that `books list --all` prints.
    fn book_ids(&self) -> Vec<u64> {
        let lines = self.run(&json!({"path": ["books", "list"], "values": {"--all": true}}));
        let document: Value = serde_json::from_str(lines[1]["text"].as_str().unwrap()).unwrap();
        document["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|book| book["id"].as_u64().unwrap())
            .collect()
    }
}

/// Returns the request to remove the book `id`.
fn remove(id: &str) -> Value {
    json!({"path": ["books", "remove"], "values": {"<ID>": [id]}})
}

/// Asserts that `reply` is a runner's refusal with `status` and `code`, and
/// returns its failure document.
fn refused(reply: &Reply, status: u16, code: &str) -> Value {
    let document = reply.json();
    assert_eq!(
        (reply.status, document["code"].as_str()),
        (status, Some(code)),
        "{document}"
    );
    assert_eq!(
        reply.header("Content-Type"),
        Some("application/problem+json")
    );
    assert_eq!(document["schema"], "flightdeck/failure/v1");
    document
}

/// Returns the process id that `unruly child linger` prints on its first
/// line.
fn linger_pid(lines: &mut Lines) -> rustix::process::Pid {
    let started = lines.next_event().unwrap();
    assert_eq!(started["event"], "started");
    let line = lines.next_event().unwrap();
    let document: Value = serde_json::from_str(line["text"].as_str().unwrap()).unwrap();
    let pid = document["data"]["pid"].as_i64().unwrap();
    rustix::process::Pid::from_raw(i32::try_from(pid).unwrap()).unwrap()
}

/// Waits up to ten seconds for the process `pid` to be gone, and returns
/// whether it is.
fn gone(pid: rustix::process::Pid) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if rustix::process::test_kill_process(pid).is_err() {
            return true;
        }
        #[expect(
            clippy::disallowed_methods,
            reason = "nothing else tells the test when another process has exited"
        )]
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn a_hidden_command_is_refused_with_422_and_the_path_as_words() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);

        let reply = page.send("/api/runs", &json!({"path": ["aliases", "list"]}));

        let document = refused(&reply, 422, "flightdeck::run::refused");
        assert_eq!(document["fields"]["reason"], "unknown_command");
        assert_eq!(document["fields"]["path"], json!(["aliases", "list"]));
        Then::Stop
    });
}

#[test]
fn an_unknown_argument_is_refused_with_its_name_for_the_page_to_mark() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);

        let reply = page.send(
            "/api/runs",
            &json!({"path": ["books", "list"], "values": {"--from": "ids.txt"}}),
        );

        let document = refused(&reply, 422, "flightdeck::run::refused");
        assert_eq!(document["fields"]["argument"], "--from");
        assert_eq!(document["fields"]["reason"], "unknown_argument");
        Then::Stop
    });
}

#[test]
fn a_body_that_is_not_a_run_request_is_refused_with_400() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);

        let reply = page.send("/api/runs", &json!({"path": "books list"}));

        assert_eq!(reply.status, 400);
        assert_eq!(reply.code(), "flightdeck::request::body");
        Then::Stop
    });
}

#[test]
fn a_value_that_starts_with_a_dash_reaches_the_tool_as_the_options_value() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);

        let lines = page.run(&json!({"path": ["books", "list"], "values": {"--limit": "-1"}}));

        // shelf refuses -1 as a value of --limit, which it reads only when
        // -1 arrives as that value.
        let ended = lines.last().unwrap();
        assert_eq!(ended["outcome"]["kind"], "tool_failure", "{lines:?}");
        assert_eq!(ended["outcome"]["failure"]["code"], "shelf::cli::usage");
        assert_eq!(ended["outcome"]["failure"]["fields"]["value"], "-1");
        assert_eq!(ended["exit"], 2);
        Then::Stop
    });
}

#[test]
fn books_remove_without_a_confirmation_id_is_refused() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);

        let reply = page.send("/api/runs", &remove("2"));

        refused(&reply, 422, "flightdeck::run::needs_confirmation");
        assert_eq!(page.book_ids(), [1, 2, 3, 4, 5, 6]);
        Then::Stop
    });
}

#[test]
fn a_preview_streams_its_dry_run_and_ends_with_the_confirmation_id() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);

        let lines = page.stream("/api/previews", &remove("2")).rest();

        let started = &lines[0];
        assert_eq!(started["event"], "started");
        assert_eq!(started["preview"], true);
        assert_eq!(started["confirmation"], Value::Null);
        assert_eq!(started["line"], "shelf books remove 2");
        let ended = lines.last().unwrap();
        assert_eq!(ended["event"], "ended");
        assert_eq!(ended["outcome"]["kind"], "result");
        assert_eq!(ended["confirmation"].as_str().unwrap().len(), 32);
        assert_eq!(page.book_ids(), [1, 2, 3, 4, 5, 6]);
        Then::Stop
    });
}

#[test]
fn a_confirmation_id_for_books_remove_2_does_not_run_books_remove_3() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);
        let id = page.confirmation(&remove("2"));

        let reply = page.send(&format!("/api/confirmations/{id}"), &remove("3"));

        let document = refused(&reply, 422, "flightdeck::run::confirmation");
        assert_eq!(document["fields"]["reason"], "mismatch");
        assert_eq!(page.book_ids(), [1, 2, 3, 4, 5, 6]);
        Then::Stop
    });
}

#[test]
fn a_confirmation_id_runs_its_command_once() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);
        let id = page.confirmation(&remove("2"));
        let path = format!("/api/confirmations/{id}");

        let first = page.stream(&path, &remove("2")).rest();
        let second = page.send(&path, &remove("2"));

        assert_eq!(first.last().unwrap()["outcome"]["kind"], "result");
        let document = refused(&second, 422, "flightdeck::run::confirmation");
        assert_eq!(document["fields"]["reason"], "unknown");
        assert_eq!(page.book_ids(), [1, 3, 4, 5, 6]);
        Then::Stop
    });
}

#[test]
fn read_only_refuses_books_remove_at_preview_and_with_an_id() {
    serving(Setup::shelf().read_only(), |running| {
        let page = Page::of(running);
        let id = "0".repeat(32);

        let preview = page.send("/api/previews", &remove("2"));
        let confirm = page.send(&format!("/api/confirmations/{id}"), &remove("2"));

        refused(&preview, 422, "flightdeck::run::read_only");
        refused(&confirm, 422, "flightdeck::run::read_only");
        Then::Stop
    });
}

#[test]
fn a_preview_of_a_command_without_dry_run_is_one_started_line_with_the_id() {
    serving(Setup::of(unruly()), |running| {
        let page = Page::of(running);
        let both = json!({"path": ["child", "both"]});

        let lines = page.stream("/api/previews", &both).rest();
        let id = lines[0]["confirmation"].as_str().unwrap();
        let confirmed = page
            .stream(&format!("/api/confirmations/{id}"), &both)
            .rest();

        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(lines[0]["event"], "started");
        assert_eq!(lines[0]["preview"], false);
        assert_eq!(lines[0]["run"], Value::Null);
        assert_eq!(confirmed.last().unwrap()["outcome"]["kind"], "result");
        Then::Stop
    });
}

#[test]
fn a_stream_from_reading_sends_started_then_each_entry_as_its_own_line_then_ended() {
    let lists = tempfile::tempdir().unwrap();
    std::fs::write(lists.path().join("to-read.txt"), "Book 1\nBook 2\nBook 3\n").unwrap();
    let setup = Setup::of(test_tool("reading")).env("READING_DIR", lists.path());
    serving(setup, |running| {
        let page = Page::of(running);
        let request = json!({"path": ["entries", "stream"], "values": {"--list": "to-read"}});

        let lines = page.run(&request);

        let events: Vec<&str> = lines
            .iter()
            .map(|line| line["event"].as_str().unwrap())
            .collect();
        assert_eq!(events.first(), Some(&"started"));
        assert_eq!(events.last(), Some(&"ended"));
        let titles: Vec<String> = lines
            .iter()
            .filter(|line| line["event"] == "line")
            .map(|line| serde_json::from_str::<Value>(line["text"].as_str().unwrap()).unwrap())
            .filter_map(|document| document["data"]["title"].as_str().map(str::to_owned))
            .collect();
        assert_eq!(titles, ["Book 1", "Book 2", "Book 3"]);
        let started = &lines[0];
        assert_eq!(started["run"], "1");
        assert_eq!(started["line"], "reading entries stream --list=to-read");
        assert!(
            started["started_at"].as_str().unwrap().ends_with('Z'),
            "{started}"
        );
        assert_eq!(lines.last().unwrap()["exit"], 0);
        Then::Stop
    });
}

#[test]
fn cancel_ends_a_stream_with_exit_130_and_keeps_the_rows_that_arrived() {
    // The list prints far more than the pipe and the runner's queue hold,
    // so reading is still running when the test cancels it.
    const ENTRIES: usize = 100_000;
    let lists = tempfile::tempdir().unwrap();
    let titles: String = (1..=ENTRIES).map(|n| format!("Book {n}\n")).collect();
    std::fs::write(lists.path().join("to-read.txt"), titles).unwrap();
    // The graces are long enough that a loaded machine does not reach
    // SIGKILL before the tool handles SIGINT.
    let setup = Setup::of(test_tool("reading"))
        .env("READING_DIR", lists.path())
        .runner(runner::Settings::new().escalation(runner::Escalation::new(
            Duration::from_secs(2),
            Duration::from_secs(2),
        )));
    serving(setup, |running| {
        let page = Page::of(running);
        let request = json!({"path": ["entries", "stream"], "values": {"--list": "to-read"}});
        let mut lines = page.stream("/api/runs", &request);
        let started = lines.next_event().unwrap();
        let first = lines.next_event().unwrap();
        let run = started["run"].as_str().unwrap();
        let cancel = format!("/api/runs/{run}/cancel");

        let cancelled = page.send(&cancel, &json!({}));
        let rest = lines.rest();
        let again = page.send(&cancel, &json!({}));

        assert_eq!(cancelled.status, 204);
        assert!(cancelled.body.is_empty());
        assert_eq!(first["event"], "line");
        let ended = rest.last().unwrap();
        assert_eq!(ended["outcome"]["kind"], "cancelled", "{ended}");
        assert_eq!(ended["exit"], 130);
        let rows = rest.iter().filter(|line| line["event"] == "line").count();
        assert!(rows + 1 < ENTRIES, "{rows}");
        refused(&again, 404, "flightdeck::run::not_running");
        Then::Stop
    });
}

#[test]
fn a_missing_binary_is_refused_with_spawn() {
    let missing = tempfile::tempdir().unwrap();
    let setup = Setup::shelf().program(missing.path().join("no-such-tool"));
    serving(setup, |running| {
        let page = Page::of(running);

        let reply = page.send("/api/runs", &json!({"path": ["books", "list"]}));

        let document = refused(&reply, 422, "flightdeck::run::spawn");
        assert_eq!(document["fields"]["tool"], "shelf");
        Then::Stop
    });
}

#[test]
fn a_silent_run_streams_alive_lines_until_its_output() {
    let setup = Setup::of(unruly()).server(
        Settings::new()
            .poll(Duration::from_millis(5))
            .keepalive(Duration::from_millis(20)),
    );
    serving(setup, |running| {
        let page = Page::of(running);
        let linger = json!({"path": ["child", "linger"], "values": {"<SECONDS>": "0.3"}});

        let lines = page.run(&linger);

        let alive = lines
            .iter()
            .position(|line| line["event"] == "alive")
            .unwrap();
        let last_line = lines
            .iter()
            .rposition(|line| line["event"] == "line")
            .unwrap();
        assert!(alive < last_line, "{lines:?}");
        assert_eq!(lines.last().unwrap()["outcome"]["kind"], "result");
        Then::Stop
    });
}

#[test]
fn a_client_that_disconnects_stops_its_run() {
    let setup = Setup::of(unruly()).server(
        Settings::new()
            .poll(Duration::from_millis(5))
            .keepalive(Duration::from_millis(20)),
    );
    serving(setup, |running| {
        let page = Page::of(running);
        let linger = json!({"path": ["child", "linger"], "values": {"<SECONDS>": "30"}});
        let mut lines = page.stream("/api/runs", &linger);
        let pid = linger_pid(&mut lines);

        drop(lines);

        // The child ignores SIGINT and SIGTERM for thirty seconds, so only
        // the cancel of a dropped run ends it this soon.
        assert!(gone(pid), "the child of a dropped stream still runs");
        Then::Stop
    });
}

#[test]
fn stopping_the_server_cancels_a_run_that_outlasts_the_shutdown_grace_and_ends_its_stream() {
    let setup = Setup::of(unruly()).runner(
        runner::Settings::new()
            .shutdown_grace(Duration::from_millis(100))
            .escalation(runner::Escalation::new(
                Duration::from_millis(200),
                Duration::from_millis(200),
            )),
    );
    serving(setup, |running| {
        let page = Page::of(running);
        let linger = json!({"path": ["child", "linger"], "values": {"<SECONDS>": "30"}});
        let mut lines = page.stream("/api/runs", &linger);
        let pid = linger_pid(&mut lines);

        running.stop();
        let rest = lines.rest();

        let ended = rest.last().unwrap();
        assert_eq!(ended["event"], "ended", "{rest:?}");
        assert_eq!(ended["outcome"]["kind"], "cancelled");
        assert!(gone(pid), "the child outlived the server");
        Then::Wait
    });
}
