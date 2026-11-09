//! The routes that run commands, through the page's server on a real
//! socket: `POST /api/runs`, `POST /api/previews`,
//! `POST /api/confirmations/<id>`, `POST /api/runs/<id>/cancel` and
//! `POST /api/choices`. The
//! tests run shelf and reading, which `scripts/install-test-tools.sh`
//! installs, and `unruly`, the runner tests' fake tool.
//!
//! A test that runs a fake tool written as a shell script, or that signals
//! a process through rustix, runs on Unix alone, because Windows runs no
//! shell script as a program and has no signals.

#![cfg(test)]

#[path = "support/http.rs"]
#[expect(dead_code, reason = "these tests use only part of the shared client")]
mod http;
#[path = "support/serving.rs"]
#[expect(dead_code, reason = "these tests use only part of the shared harness")]
mod serving;
#[path = "support/signals.rs"]
mod signals;

#[cfg(unix)]
use std::time::{Duration, Instant};

#[cfg(unix)]
use crumbkit_flightdeck::runner;
#[cfg(unix)]
use crumbkit_flightdeck::server::Settings;
use http::{Call, Lines, Reply};
use serde_json::{Value, json};
#[cfg(unix)]
use serving::unruly;
use serving::{Running, Setup, Then, serving, test_tool};

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

/// Returns the request for the choices of `argument` of `target` from the
/// rows of `source`.
fn choices(source: &[&str], target: &[&str], argument: &str) -> Value {
    json!({"source": source, "target": target, "argument": argument})
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

/// Returns the process id that `unruly child linger` or `unruly child spew`
/// prints on its first line.
#[cfg(unix)]
fn linger_pid(lines: &mut Lines) -> rustix::process::Pid {
    let started = lines.next_event().unwrap();
    assert_eq!(started["event"], "started");
    let line = lines.next_event().unwrap();
    let document: Value = serde_json::from_str(line["text"].as_str().unwrap()).unwrap();
    let pid = document["data"]["pid"].as_i64().unwrap();
    rustix::process::Pid::from_raw(i32::try_from(pid).unwrap()).unwrap()
}

/// Waits up to `limit` for the process `pid` to be gone, and returns
/// whether it is.
#[cfg(unix)]
fn gone_within(pid: rustix::process::Pid, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
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

/// Waits up to ten seconds for the process `pid` to be gone, and returns
/// whether it is.
#[cfg(unix)]
fn gone(pid: rustix::process::Pid) -> bool {
    gone_within(pid, Duration::from_secs(10))
}

#[test]
fn every_run_route_refuses_a_request_without_the_token_or_with_a_wrong_one_and_runs_nothing() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);
        let id = page.confirmation(&remove("2"));
        let routes = [
            ("/api/runs".to_owned(), remove("2")),
            ("/api/previews".to_owned(), remove("2")),
            (format!("/api/confirmations/{id}"), remove("2")),
            ("/api/runs/1/cancel".to_owned(), json!({})),
            (
                "/api/choices".to_owned(),
                choices(&["books", "list"], &["books", "show"], "<ID>"),
            ),
        ];
        let wrong = "0".repeat(64);

        for (path, body) in &routes {
            let call = || Call::post(running.port, path, &body.to_string());
            let without = call().send(running.port);
            let with_wrong = call().bearer(&wrong).send(running.port);

            for reply in [without, with_wrong] {
                assert_eq!(reply.status, 401, "{path}");
                assert_eq!(reply.code(), "flightdeck::session::unauthorized", "{path}");
            }
        }

        assert_eq!(page.book_ids(), [1, 2, 3, 4, 5, 6]);
        let confirmed = page.stream(&format!("/api/confirmations/{id}"), &remove("2"));
        assert_eq!(
            confirmed.rest().last().unwrap()["outcome"]["kind"],
            "result"
        );
        Then::Stop
    });
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
fn a_positional_value_that_starts_with_a_dash_reaches_the_tool_after_the_separator() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);

        let lines = page.stream("/api/previews", &remove("-1")).rest();

        // shelf reads -1 as the value of <ID> only after `--`. Before it,
        // shelf refuses -1 as an unknown flag and names no value.
        let ended = lines.last().unwrap();
        assert_eq!(ended["outcome"]["kind"], "tool_failure", "{lines:?}");
        let failure = &ended["outcome"]["failure"];
        assert_eq!(failure["code"], "shelf::cli::usage");
        assert_eq!(failure["fields"]["argument"], "<ID>...");
        assert_eq!(failure["fields"]["value"], "-1");
        assert_eq!(page.book_ids(), [1, 2, 3, 4, 5, 6]);
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

#[cfg(unix)]
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

#[cfg(unix)]
#[test]
fn with_every_run_streaming_a_cancel_ends_one_with_exit_130_and_keeps_the_rows_that_arrived() {
    serving(Setup::of(unruly()), |running| {
        let page = Page::of(running);
        let wait = json!({"path": ["child", "wait"]});
        // Every run that the runner allows streams, each on a connection
        // of its own, so the cancel needs a connection the runs leave free.
        let mut streams: Vec<Lines> = (0..runner::MOST_RUNS)
            .map(|_| page.stream("/api/runs", &wait))
            .collect();
        let mut first = streams.remove(0);
        let started = first.next_event().unwrap();
        let arrived: Vec<Value> = (0..3).map(|_| first.next_event().unwrap()).collect();
        let run = started["run"].as_str().unwrap();
        let cancel = format!("/api/runs/{run}/cancel");

        let cancelled = page.send(&cancel, &json!({}));
        let rest = first.rest();
        let again = page.send(&cancel, &json!({}));

        assert_eq!(cancelled.status, 204);
        assert!(cancelled.body.is_empty());
        let numbers: Vec<Value> = arrived
            .iter()
            .map(|line| {
                assert_eq!(line["event"], "line", "{line}");
                let document: Value = serde_json::from_str(line["text"].as_str().unwrap()).unwrap();
                document["data"]["n"].clone()
            })
            .collect();
        assert_eq!(numbers, [1, 2, 3]);
        let ended = rest.last().unwrap();
        assert_eq!(ended["outcome"]["kind"], "cancelled", "{ended}");
        assert_eq!(ended["exit"], 130);
        refused(&again, 404, "flightdeck::run::not_running");
        Then::Stop
    });
}

#[cfg(unix)]
#[test]
fn a_run_over_the_runners_cap_is_refused_with_503_and_busy() {
    let setup = Setup::of(unruly()).runner(runner::Settings::new().most_runs(1));
    serving(setup, |running| {
        let page = Page::of(running);
        let wait = json!({"path": ["child", "wait"]});
        let mut holding = page.stream("/api/runs", &wait);
        holding.next_event().unwrap();

        let reply = page.send("/api/runs", &wait);

        refused(&reply, 503, "flightdeck::run::busy");
        Then::Stop
    });
}

#[test]
fn a_missing_binary_is_refused_with_500_and_spawn() {
    let missing = tempfile::tempdir().unwrap();
    let setup = Setup::shelf().program(missing.path().join("no-such-tool"));
    serving(setup, |running| {
        let page = Page::of(running);

        let reply = page.send("/api/runs", &json!({"path": ["books", "list"]}));

        let document = refused(&reply, 500, "flightdeck::run::spawn");
        assert_eq!(document["fields"]["tool"], "shelf");
        Then::Stop
    });
}

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(unix)]
#[test]
fn a_client_that_stays_connected_but_stops_reading_has_its_run_stopped_after_the_write_time() {
    let write_time = Duration::from_millis(200);
    let setup = Setup::of(unruly())
        .server(
            Settings::new()
                .poll(Duration::from_millis(5))
                .write_time(write_time),
        )
        // The run's output has no total limit, so only the write deadline
        // can end it.
        .runner(runner::Settings::new().limits(runner::Limits::new(4096, usize::MAX)));
    serving(setup, |running| {
        let page = Page::of(running);
        let spew = json!({"path": ["child", "spew"]});
        let mut held = page.stream("/api/runs", &spew);
        let pid = linger_pid(&mut held);

        // The client keeps the connection open and reads nothing more, so
        // the socket's buffers fill and a write waits for the client. The
        // default write time is ten seconds, so a child gone in five shows
        // that the setting ended the stream.
        assert!(
            gone_within(pid, Duration::from_secs(5)),
            "the child of a stream that nobody reads still runs"
        );
        drop(held);
        Then::Stop
    });
}

#[cfg(unix)]
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

#[test]
fn the_choices_of_books_show_are_the_ids_of_the_six_books_of_books_list() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);

        let reply = page.send(
            "/api/choices",
            &choices(&["books", "list"], &["books", "show"], "<ID>"),
        );

        assert_eq!(reply.status, 200);
        assert_eq!(reply.header("Content-Type"), Some("application/json"));
        let document = reply.json();
        assert_eq!(document["source"], json!(["books", "list"]));
        assert_eq!(document["column"], "id");
        assert_eq!(document["truncated"], false);
        let values: Vec<&str> = document["choices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|choice| choice["value"].as_str().unwrap())
            .collect();
        assert_eq!(values, ["1", "2", "3", "4", "5", "6"]);
        Then::Stop
    });
}

#[test]
fn the_choices_route_refuses_a_source_that_is_not_read_only_and_runs_nothing() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);

        let reply = page.send(
            "/api/choices",
            &choices(&["books", "remove"], &["books", "show"], "<ID>"),
        );

        let document = refused(&reply, 422, "flightdeck::run::refused");
        assert_eq!(document["fields"]["reason"], "not_read_only");
        assert_eq!(document["fields"]["path"], json!(["books", "remove"]));
        assert_eq!(page.book_ids(), [1, 2, 3, 4, 5, 6]);
        Then::Stop
    });
}

#[test]
fn the_choices_route_refuses_a_source_that_the_model_did_not_infer() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);
        let uninferred = [
            // books summarize prints no id column.
            choices(&["books", "summarize"], &["books", "show"], "<ID>"),
            // --from is no positional whose value name is ID.
            choices(&["books", "list"], &["books", "show"], "--from"),
            // A link never goes from a command to itself.
            choices(&["books", "show"], &["books", "show"], "<ID>"),
            // A hidden command is no source.
            choices(&["aliases", "list"], &["books", "show"], "<ID>"),
        ];

        for body in &uninferred {
            let reply = page.send("/api/choices", body);

            let document = refused(&reply, 422, "flightdeck::run::refused");
            assert_eq!(document["fields"]["reason"], "not_inferred", "{body}");
        }
        Then::Stop
    });
}

#[test]
fn a_body_that_is_not_a_choices_request_is_refused_with_400() {
    serving(Setup::shelf(), |running| {
        let page = Page::of(running);
        let mut extra = choices(&["books", "list"], &["books", "show"], "<ID>");
        extra["values"] = json!({"--limit": "1"});

        let reply = page.send("/api/choices", &extra);

        assert_eq!(reply.status, 400);
        assert_eq!(reply.code(), "flightdeck::request::body");
        Then::Stop
    });
}

#[cfg(unix)]
#[test]
fn a_choices_request_over_the_runners_cap_is_refused_with_503_and_busy() {
    let setup = Setup::of(unruly()).runner(runner::Settings::new().most_runs(1));
    serving(setup, |running| {
        let page = Page::of(running);
        let mut holding = page.stream("/api/runs", &json!({"path": ["child", "wait"]}));
        holding.next_event().unwrap();

        let reply = page.send(
            "/api/choices",
            &choices(&["child", "roster"], &["child", "visit"], "<ID>"),
        );

        refused(&reply, 503, "flightdeck::run::busy");
        Then::Stop
    });
}

#[cfg(unix)]
#[test]
fn a_source_that_outlasts_the_choices_time_is_stopped_and_answered_with_504() {
    let setup = Setup::of(unruly()).runner(
        runner::Settings::new()
            .choices_time(Duration::from_millis(200))
            .escalation(runner::Escalation::new(
                Duration::from_millis(200),
                Duration::from_millis(200),
            )),
    );
    serving(setup, |running| {
        let page = Page::of(running);
        let started = Instant::now();

        // The runner reads the shared fake clock, which stands still, so a
        // second thread keeps moving it past the choices time until the
        // sender drops, which happens when the request returns or panics.
        let reply = std::thread::scope(|scope| {
            let (stop, receiver) = std::sync::mpsc::channel::<()>();
            let clock = running.clock;
            scope.spawn(move || {
                while receiver.recv_timeout(Duration::from_millis(50))
                    == Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                {
                    clock.advance(Duration::from_secs(1));
                }
            });
            let _stop = stop;
            page.send(
                "/api/choices",
                &choices(&["child", "roster"], &["child", "visit"], "<ID>"),
            )
        });

        assert!(started.elapsed() < Duration::from_secs(5));
        let document = refused(&reply, 504, "flightdeck::choices::time_limit");
        assert_eq!(document["fields"]["path"], "child roster");
        Then::Stop
    });
}

#[cfg(unix)]
#[test]
fn a_source_that_ends_without_a_listing_is_answered_with_502_and_how_it_ended() {
    serving(Setup::of(unruly()), |running| {
        let page = Page::of(running);

        let reply = page.send(
            "/api/choices",
            &choices(&["child", "ledger"], &["child", "visit"], "<ID>"),
        );

        let document = refused(&reply, 502, "flightdeck::choices::no_listing");
        assert_eq!(document["fields"]["path"], "child ledger");
        assert_eq!(document["fields"]["outcome"], "failed");
        Then::Stop
    });
}
