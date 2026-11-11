//! The page's local HTTP server, driven in-process on a port the system
//! picks. Each test serves shelf, one of crumbkit's example tools that
//! `scripts/install-test-tools.sh` installs, through `support/serving.rs`,
//! and sends plain HTTP requests. The clock is injected, so no test waits
//! for a deadline. `tests/routes.rs` tests the routes that run commands.

#![cfg(test)]

#[path = "support/http.rs"]
#[expect(dead_code, reason = "these tests use only part of the shared client")]
mod http;
#[path = "support/serving.rs"]
#[expect(dead_code, reason = "these tests use only part of the shared harness")]
mod serving;
#[path = "support/signals.rs"]
mod signals;

use std::path::{Path, PathBuf};
use std::time::Duration;

use crumbkit_flightdeck::server::{
    BODY_LIMIT, CONNECTION_LIMIT, Ended, HEAD_LIMIT, HEADER_LIMIT, IDLE_LIMIT, Settings,
};
use http::{Call, Reply, raw};
use serving::{Setup, Then, serving};

/// The headers that every reply carries, with their values.
const SECURITY_HEADERS: [(&str, &str); 6] = [
    ("Cache-Control", "no-store"),
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "no-referrer"),
    ("Cross-Origin-Resource-Policy", "same-origin"),
    ("Cross-Origin-Opener-Policy", "same-origin"),
    (
        "Content-Security-Policy",
        "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
    ),
];

/// Asserts that `reply` is a refusal with `status` and `code`.
fn assert_refused(reply: &Reply, status: u16, code: &str) {
    assert_eq!(
        (reply.status, reply.code().as_str()),
        (status, code),
        "{}",
        String::from_utf8_lossy(&reply.body)
    );
    assert_eq!(
        reply.header("Content-Type"),
        Some("application/problem+json")
    );
}

#[test]
fn a_request_without_a_token_gets_401_with_a_problem_document() {
    serving(Setup::shelf(), |page| {
        let reply = Call::get(page.port, "/api/model").send(page.port);

        assert_refused(&reply, 401, "flightdeck::session::unauthorized");
        assert_eq!(reply.json()["status"], 401);
        assert_eq!(reply.header("WWW-Authenticate"), Some("Bearer"));
        Then::Stop
    });
}

#[test]
fn a_wrong_token_of_the_right_length_gets_401() {
    serving(Setup::shelf(), |page| {
        let token = page.token();
        let wrong: String = token
            .chars()
            .map(|c| if c == '0' { '1' } else { '0' })
            .collect();
        assert_eq!(wrong.len(), token.len());

        assert_refused(
            &page.model(&wrong),
            401,
            "flightdeck::session::unauthorized",
        );
        Then::Stop
    });
}

#[test]
fn a_malformed_authorization_header_gets_401() {
    serving(Setup::shelf(), |page| {
        let token = page.token();
        for value in [
            format!("Basic {token}"),
            format!("bearer {token}"),
            format!("Bearer  {token}"),
            format!("Bearer {token}0"),
            format!("Bearer {token}, Bearer {token}"),
            "Bearer".to_owned(),
            String::new(),
        ] {
            let reply = Call::get(page.port, "/api/model")
                .header("Authorization", &value)
                .send(page.port);
            assert_refused(&reply, 401, "flightdeck::session::unauthorized");
        }
        Then::Stop
    });
}

#[test]
fn a_launch_code_exchanges_for_a_token_of_64_hex_digits_that_opens_the_api() {
    serving(Setup::shelf(), |page| {
        let token = page.token();

        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(page.model(&token).status, 200);
        Then::Stop
    });
}

#[test]
fn the_same_launch_code_a_second_time_is_refused() {
    serving(Setup::shelf(), |page| {
        page.token();

        let body = format!(r#"{{"code":"{}"}}"#, page.code);
        let reply = Call::post(page.port, "/api/session", &body).send(page.port);

        assert_refused(&reply, 401, "flightdeck::session::code_refused");
        Then::Stop
    });
}

#[test]
fn a_launch_code_older_than_60_seconds_is_refused() {
    serving(Setup::shelf(), |page| {
        page.clock.advance(Duration::from_secs(61));

        let body = format!(r#"{{"code":"{}"}}"#, page.code);
        let reply = Call::post(page.port, "/api/session", &body).send(page.port);

        assert_refused(&reply, 401, "flightdeck::session::code_refused");
        Then::Stop
    });
}

#[test]
fn a_launch_code_is_refused_once_the_wall_clock_passes_60_seconds_while_the_monotonic_clock_stood_still()
 {
    serving(Setup::shelf(), |page| {
        page.clock.sleep(Duration::from_secs(61));

        let body = format!(r#"{{"code":"{}"}}"#, page.code);
        let reply = Call::post(page.port, "/api/session", &body).send(page.port);

        assert_refused(&reply, 401, "flightdeck::session::code_refused");
        Then::Stop
    });
}

#[test]
fn a_wrong_launch_code_is_refused_and_leaves_the_right_one_valid() {
    serving(Setup::shelf(), |page| {
        let wrong = "0".repeat(page.code.len());
        let reply = Call::post(
            page.port,
            "/api/session",
            &format!(r#"{{"code":"{wrong}"}}"#),
        )
        .send(page.port);
        assert_refused(&reply, 401, "flightdeck::session::code_refused");

        assert_eq!(page.model(&page.token()).status, 200);
        Then::Stop
    });
}

#[test]
fn a_session_body_that_is_not_the_exchange_document_is_refused() {
    serving(Setup::shelf(), |page| {
        for body in ["", "{", r#"{"code":7}"#, r#"{"code":"a","extra":1}"#] {
            let reply = Call::post(page.port, "/api/session", body).send(page.port);
            assert_refused(&reply, 400, "flightdeck::request::body");
        }
        Then::Stop
    });
}

#[test]
fn a_host_other_than_the_servers_own_address_is_refused() {
    serving(Setup::shelf(), |page| {
        let token = page.token();
        for host in [
            format!("localhost:{}", page.port),
            "127.0.0.1".to_owned(),
            format!("127.0.0.1:{}", page.port + 1),
            format!("evil.example:{}", page.port),
        ] {
            let reply = Call::get(page.port, "/api/model")
                .bearer(&token)
                .header("Host", &host)
                .send(page.port);
            assert_refused(&reply, 403, "flightdeck::request::host");
        }
        let reply = Call::get(page.port, "/").without("Host").send(page.port);
        assert_refused(&reply, 403, "flightdeck::request::host");
        Then::Stop
    });
}

#[test]
fn a_foreign_origin_is_refused_on_any_method() {
    serving(Setup::shelf(), |page| {
        let token = page.token();
        let foreign = format!("http://localhost:{}", page.port);
        let get = Call::get(page.port, "/api/model")
            .bearer(&token)
            .header("Origin", &foreign)
            .send(page.port);
        let post = Call::post(page.port, "/api/session", r#"{"code":"0"}"#)
            .header("Origin", "http://evil.example")
            .send(page.port);

        assert_refused(&get, 403, "flightdeck::request::origin");
        assert_refused(&post, 403, "flightdeck::request::origin");
        Then::Stop
    });
}

#[test]
fn a_post_without_an_origin_is_refused() {
    serving(Setup::shelf(), |page| {
        let body = format!(r#"{{"code":"{}"}}"#, page.code);
        let reply = Call::post(page.port, "/api/session", &body)
            .without("Origin")
            .send(page.port);

        assert_refused(&reply, 403, "flightdeck::request::origin");
        assert_eq!(page.model(&page.token()).status, 200);
        Then::Stop
    });
}

#[test]
fn a_cross_site_or_same_site_request_is_refused() {
    serving(Setup::shelf(), |page| {
        let token = page.token();
        for site in ["cross-site", "same-site"] {
            let get = Call::get(page.port, "/api/model")
                .bearer(&token)
                .header("Sec-Fetch-Site", site)
                .send(page.port);
            let script = Call::get(page.port, "/")
                .header("Sec-Fetch-Site", site)
                .header("Sec-Fetch-Mode", "no-cors")
                .send(page.port);
            assert_refused(&get, 403, "flightdeck::request::cross_site");
            assert_refused(&script, 403, "flightdeck::request::cross_site");
        }
        Then::Stop
    });
}

#[test]
fn a_cross_site_navigation_to_the_page_is_served_because_the_opener_page_starts_one() {
    serving(Setup::shelf(), |page| {
        let token = page.token();
        let navigate = |path: &str| {
            Call::get(page.port, path)
                .bearer(&token)
                .header("Sec-Fetch-Site", "cross-site")
                .header("Sec-Fetch-Mode", "navigate")
                .send(page.port)
        };

        assert_eq!(navigate("/").status, 200);
        assert_refused(
            &navigate("/api/model"),
            403,
            "flightdeck::request::cross_site",
        );
        Then::Stop
    });
}

#[test]
fn a_same_origin_request_is_served() {
    serving(Setup::shelf(), |page| {
        let token = page.token();
        let reply = Call::get(page.port, "/api/model")
            .bearer(&token)
            .header("Sec-Fetch-Site", "same-origin")
            .header("Origin", &format!("http://127.0.0.1:{}", page.port))
            .send(page.port);

        assert_eq!(reply.status, 200);
        Then::Stop
    });
}

#[test]
fn a_post_that_is_not_json_is_refused() {
    serving(Setup::shelf(), |page| {
        let body = format!("code={}", page.code);
        for content_type in [
            Some("application/x-www-form-urlencoded"),
            Some("text/plain"),
            Some("multipart/form-data; boundary=x"),
            None,
        ] {
            let call = Call::post(page.port, "/api/session", &body);
            let call = match content_type {
                Some(value) => call.header("Content-Type", value),
                None => call.without("Content-Type"),
            };
            assert_refused(
                &call.send(page.port),
                415,
                "flightdeck::request::content_type",
            );
        }
        Then::Stop
    });
}

#[test]
fn a_json_content_type_with_a_charset_is_accepted() {
    serving(Setup::shelf(), |page| {
        let body = format!(r#"{{"code":"{}"}}"#, page.code);
        let reply = Call::post(page.port, "/api/session", &body)
            .header("Content-Type", "application/json; charset=utf-8")
            .send(page.port);

        assert_eq!(reply.status, 200);
        Then::Stop
    });
}

#[test]
fn an_options_request_is_refused_without_cors_headers() {
    serving(Setup::shelf(), |page| {
        let reply = Call::new("OPTIONS", page.port, "/api/session")
            .header("Origin", &format!("http://127.0.0.1:{}", page.port))
            .header("Access-Control-Request-Method", "POST")
            .send(page.port);

        assert_refused(&reply, 405, "flightdeck::request::method");
        assert!(
            reply
                .headers
                .iter()
                .all(|(name, _)| !name.to_ascii_lowercase().starts_with("access-control-")),
            "{:?}",
            reply.headers
        );
        Then::Stop
    });
}

#[test]
fn a_body_over_the_size_limit_is_refused() {
    serving(Setup::shelf(), |page| {
        let reply = Call::post(page.port, "/api/session", "")
            .body(vec![b' '; BODY_LIMIT + 1])
            .send(page.port);

        assert_refused(&reply, 413, "flightdeck::request::too_large");
        Then::Stop
    });
}

#[test]
fn an_unknown_path_gets_404_and_an_unknown_api_path_needs_the_token_first() {
    serving(Setup::shelf(), |page| {
        let token = page.token();

        let unknown = Call::get(page.port, "/nothing-here").send(page.port);
        let api_without_token = Call::get(page.port, "/api/nothing").send(page.port);
        let api_with_token = Call::get(page.port, "/api/nothing")
            .bearer(&token)
            .send(page.port);

        assert_refused(&unknown, 404, "flightdeck::request::not_found");
        assert_refused(&api_without_token, 401, "flightdeck::session::unauthorized");
        assert_refused(&api_with_token, 404, "flightdeck::request::not_found");
        Then::Stop
    });
}

#[test]
fn every_reply_carries_the_security_headers_and_no_cors_header() {
    serving(Setup::shelf(), |page| {
        let token = page.token();
        let replies = [
            Call::get(page.port, "/").send(page.port),
            page.model(&token),
            Call::get(page.port, "/api/model").send(page.port),
            Call::get(page.port, "/nothing-here").send(page.port),
            Call::get(page.port, "/")
                .header("Host", "localhost")
                .send(page.port),
            Call::new("OPTIONS", page.port, "/").send(page.port),
            Call::post(page.port, "/api/session", "")
                .body(vec![b' '; BODY_LIMIT + 1])
                .send(page.port),
        ];

        for reply in &replies {
            for (name, value) in SECURITY_HEADERS {
                assert_eq!(
                    reply.header(name),
                    Some(value),
                    "{name} on a {} reply",
                    reply.status
                );
            }
            assert_eq!(reply.header("Access-Control-Allow-Origin"), None);
        }
        Then::Stop
    });
}

#[test]
fn the_root_path_serves_the_built_page_as_html() {
    serving(Setup::shelf(), |page| {
        let reply = Call::get(page.port, "/").send(page.port);

        assert_eq!(reply.status, 200);
        assert_eq!(
            reply.header("Content-Type"),
            Some("text/html; charset=utf-8")
        );
        assert_eq!(
            reply.body,
            std::fs::read(ui_dist().join("index.html")).unwrap()
        );
        Then::Stop
    });
}

/// The directory of the page's built files.
fn ui_dist() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/dist")
}

/// Returns every file under `directory`, by its path relative to `root`.
fn files_under(root: &Path, directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_under(root, &path));
        } else {
            files.push(path.strip_prefix(root).unwrap().to_path_buf());
        }
    }
    files
}

/// Returns the names of the files under `ui/dist/assets` that `text`
/// names after `marker` and before the closing quote, such as `app.js`
/// for `src="/assets/app.js"`.
fn assets_named(text: &str, marker: &str) -> Vec<String> {
    text.split(marker)
        .skip(1)
        .filter_map(|rest| rest.split(['"', '`', '\'']).next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_first_load_size_counts_every_file_that_the_page_loads_before_it_runs() {
    let root = ui_dist();
    let html = std::fs::read_to_string(root.join("index.html")).unwrap();
    let mut first_load: Vec<String> = assets_named(&html, "\"/assets/");
    // Each file that a first-load script imports statically loads before
    // the page runs too.
    let mut index = 0;
    while let Some(name) = first_load.get(index).cloned() {
        index += 1;
        if !name.ends_with(".js") {
            continue;
        }
        let script = std::fs::read_to_string(root.join("assets").join(&name)).unwrap();
        for marker in ["from\"./", "import\"./"] {
            for imported in assets_named(&script, marker) {
                if !first_load.contains(&imported) {
                    first_load.push(imported);
                }
            }
        }
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/package.json")).unwrap(),
    )
    .unwrap();
    let counted: Vec<&str> = manifest["size-limit"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "first load")
        .unwrap()["path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap())
        .collect();

    assert!(first_load.contains(&"app.js".to_owned()), "{first_load:?}");
    for name in &first_load {
        let path = format!("dist/assets/{name}");
        assert!(
            counted.contains(&path.as_str()),
            "the first load reads {path}, which the size step's first load does not count: {counted:?}"
        );
    }
}

#[test]
fn every_built_file_of_the_page_is_served_with_its_bytes_and_its_media_type() {
    let root = ui_dist();
    let files = files_under(&root, &root);
    assert!(files.len() >= 3, "ui/dist holds {files:?}");
    serving(Setup::shelf(), |page| {
        for file in &files {
            // A URL path separates its segments with `/` on every system.
            let segments: Vec<&str> = file
                .components()
                .map(|segment| segment.as_os_str().to_str().unwrap())
                .collect();
            let name = segments.join("/");
            let path = if name == "index.html" {
                "/".to_owned()
            } else {
                format!("/{name}")
            };
            let reply = Call::get(page.port, &path).send(page.port);

            let media_type = match file.extension().and_then(|ext| ext.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "text/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("woff2") => "font/woff2",
                other => panic!(
                    "ui/dist holds {name}, of a type the server has no media type for: {other:?}"
                ),
            };
            assert_eq!(reply.status, 200, "{path}");
            assert_eq!(reply.header("Content-Type"), Some(media_type), "{path}");
            assert_eq!(
                reply.body,
                std::fs::read(root.join(file)).unwrap(),
                "{path}"
            );
        }
        Then::Stop
    });
}

#[test]
fn the_model_route_returns_shelfs_model_with_read_only_as_started() {
    for read_only in [false, true] {
        let setup = if read_only {
            Setup::shelf().read_only()
        } else {
            Setup::shelf()
        };
        serving(setup, |page| {
            let reply = page.model(&page.token());

            assert_eq!(reply.status, 200);
            assert_eq!(reply.header("Content-Type"), Some("application/json"));
            let model = reply.json();
            assert_eq!(model["read_only"], read_only);
            assert_eq!(model["tool"]["name"], "shelf");
            assert_eq!(model["tool"]["commands"].as_array().unwrap().len(), 17);
            let books_show = &model["tool"]["commands"][1];
            assert_eq!(books_show["path"], serde_json::json!(["books", "show"]));
            assert_eq!(books_show["arguments"][0]["name"], "<ID>");
            assert_eq!(books_show["arguments"][0]["arity"]["kind"], "many");
            assert_eq!(books_show["effects"]["read_only"], true);
            assert_eq!(
                model["tool"]["links"][0]["target"],
                serde_json::json!(["books", "show"])
            );
            Then::Stop
        });
    }
}

#[test]
fn the_weight_route_needs_the_token_and_returns_the_bytes_of_each_help_page() {
    serving(Setup::shelf(), |page| {
        let refused = Call::get(page.port, "/api/weight").send(page.port);
        assert_refused(&refused, 401, "flightdeck::session::unauthorized");

        let reply = Call::get(page.port, "/api/weight")
            .bearer(&page.token())
            .send(page.port);

        assert_eq!(reply.status, 200);
        let weight = reply.json();
        assert_eq!(weight["problem"], serde_json::Value::Null);
        assert_eq!(weight["left_out"], 0);
        let pages = weight["pages"].as_array().unwrap();
        assert_eq!(pages[0]["path"], serde_json::json!([]));
        assert_eq!(pages[1]["path"], serde_json::json!(["books"]));
        assert!(
            pages.iter().all(|page| page["bytes"].as_u64().unwrap() > 0),
            "{pages:?}"
        );
        Then::Stop
    });
}

/// Writes a stand-in for shelf's help pages that appends its words to the
/// file `log` names and then runs `then`, and returns its directory and path.
#[cfg(unix)]
fn help_stand_in(log: &Path, then: &str) -> (tempfile::TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("stand-in");
    let script = format!("#!/bin/sh\necho \"$*\" >> '{}'\n{then}\n", log.display());
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (directory, path)
}

/// Returns the lines of the file at `log`, or none when it is not there.
#[cfg(unix)]
fn logged(log: &Path) -> Vec<String> {
    std::fs::read_to_string(log)
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

#[cfg(unix)]
#[test]
fn two_weight_requests_at_once_share_one_measurement_and_a_later_one_reuses_it() {
    let logs = tempfile::tempdir().unwrap();
    let log = logs.path().join("help");
    let (_directory, stand_in) = help_stand_in(&log, "/bin/sleep 0.2; echo page");
    serving(Setup::shelf().program(stand_in), |page| {
        let token = page.token();
        let weight = || {
            Call::get(page.port, "/api/weight")
                .bearer(&token)
                .send(page.port)
        };

        let (first, second) = std::thread::scope(|scope| {
            let first = scope.spawn(weight);
            let second = scope.spawn(weight);
            (first.join().unwrap(), second.join().unwrap())
        });
        let later = weight();

        assert_eq!((first.status, second.status, later.status), (200, 200, 200));
        assert_eq!(first.json(), second.json());
        assert_eq!(first.json(), later.json());
        let pages = first.json()["pages"].as_array().unwrap().len();
        assert_eq!(logged(&log).len(), pages, "{:?}", logged(&log));
        Then::Stop
    });
}

#[cfg(unix)]
#[test]
fn a_partial_measurement_is_kept_for_a_plain_request_and_measured_again_when_asked() {
    use crumbkit_flightdeck::runner;
    use crumbkit_flightdeck::spawn::HelpLimits;

    let logs = tempfile::tempdir().unwrap();
    let log = logs.path().join("help");
    let (_directory, stand_in) = help_stand_in(&log, "/bin/sleep 1; echo page");
    // The first help runs end after the measurement's limit, so no other
    // page starts and the measurement is partial.
    let limits = HelpLimits::new().total(Duration::from_millis(500));
    let setup = Setup::shelf()
        .program(stand_in)
        .runner(runner::Settings::new().help_limits(limits));
    serving(setup, |page| {
        let token = page.token();
        let weight = |path: &str| {
            Call::get(page.port, path)
                .bearer(&token)
                .send(page.port)
                .json()
        };
        let not_run = |weight: &serde_json::Value| {
            weight["pages"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|page| page["failure"] == "not_run")
                .count()
        };

        let first = weight("/api/weight");
        let runs_of_first = logged(&log).len();
        let plain = weight("/api/weight");
        let runs_after_plain = logged(&log).len();
        let again = weight("/api/weight?again=1");

        assert!(not_run(&first) > 0, "{first}");
        assert_eq!(plain, first);
        assert_eq!(runs_after_plain, runs_of_first);
        assert!(not_run(&again) > 0, "{again}");
        assert!(logged(&log).len() > runs_after_plain, "{:?}", logged(&log));
        Then::Stop
    });
}

#[cfg(unix)]
#[test]
fn stopping_the_server_during_a_measurement_ends_it_within_the_shutdown_bound() {
    let logs = tempfile::tempdir().unwrap();
    let log = logs.path().join("help");
    let (_directory, stand_in) = help_stand_in(&log, "exec /bin/sleep 30");
    let stopped = std::sync::OnceLock::new();
    serving(Setup::shelf().program(stand_in), |page| {
        let token = page.token();
        std::thread::scope(|scope| {
            let measuring = scope.spawn(|| {
                Call::get(page.port, "/api/weight")
                    .bearer(&token)
                    .send(page.port)
            });
            while logged(&log).is_empty() {
                #[expect(
                    clippy::disallowed_methods,
                    reason = "the test waits for the first help run to start"
                )]
                std::thread::sleep(Duration::from_millis(5));
            }
            stopped.set(std::time::Instant::now()).unwrap();
            page.stop();
            assert_eq!(measuring.join().unwrap().status, 200);
        });
        Then::Wait
    });

    // Server::run promises the write time after the runner's shutdown, which
    // has no run to wait for here. A help run would otherwise hold the stop
    // for its 2 s limit, and the measurement for up to 30 s.
    let took = stopped.get().unwrap().elapsed();
    assert!(took < Duration::from_secs(1), "the stop took {took:?}");
}

#[test]
fn the_server_stops_by_itself_after_the_idle_limit() {
    let ended = serving(Setup::shelf(), |page| {
        page.clock.advance(IDLE_LIMIT);
        Then::Wait
    });

    assert_eq!(ended, Ended::Idle);
}

#[test]
fn the_server_stops_after_the_idle_limit_by_the_wall_clock_while_the_monotonic_clock_stood_still() {
    let ended = serving(Setup::shelf(), |page| {
        page.clock.sleep(IDLE_LIMIT);
        Then::Wait
    });

    assert_eq!(ended, Ended::Idle);
}

#[test]
fn an_authenticated_request_restarts_the_idle_limit() {
    let ended = serving(Setup::shelf(), |page| {
        let token = page.token();
        page.clock.advance(IDLE_LIMIT - Duration::from_secs(60));
        assert_eq!(page.model(&token).status, 200);
        page.clock.advance(IDLE_LIMIT - Duration::from_secs(60));

        assert_eq!(page.model(&token).status, 200);
        Then::Stop
    });

    assert_eq!(ended, Ended::Stopped);
}

#[test]
fn a_refused_request_does_not_restart_the_idle_limit() {
    let ended = serving(Setup::shelf(), |page| {
        page.clock.advance(IDLE_LIMIT - Duration::from_secs(60));
        Call::get(page.port, "/api/model")
            .bearer(&"0".repeat(64))
            .send(page.port);
        page.clock.advance(Duration::from_secs(60));
        Then::Wait
    });

    assert_eq!(ended, Ended::Idle);
}

#[test]
fn a_confirmation_is_refused_as_expired_once_the_shared_clock_passes_the_runners_deadline() {
    serving(Setup::shelf(), |page| {
        let token = page.token();
        let body = serde_json::json!({"path": ["books", "remove"], "values": {"<ID>": ["2"]}});
        let post = |path: &str| Call::post(page.port, path, &body.to_string()).bearer(&token);
        let preview = post("/api/previews").stream(page.port).rest();
        let id = preview.last().unwrap()["confirmation"].as_str().unwrap();

        page.clock.advance(Duration::from_secs(10 * 60));
        let reply = post(&format!("/api/confirmations/{id}")).send(page.port);

        assert_refused(&reply, 422, "flightdeck::run::confirmation");
        assert_eq!(reply.json()["fields"]["reason"], "expired");
        Then::Stop
    });
}

#[cfg(unix)]
#[test]
fn the_opener_page_is_private_to_the_user_and_holds_the_code_only_in_its_fragment() {
    use std::os::unix::fs::PermissionsExt as _;

    serving(Setup::shelf(), |page| {
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&page.page), 0o600);
        assert_eq!(mode(page.page.parent().unwrap()), 0o700);

        let html = std::fs::read_to_string(&page.page).unwrap();
        let origin = format!("http://127.0.0.1:{}/#code=", page.port);
        assert_eq!(page.code.len(), 64);
        let uses: Vec<usize> = html.match_indices(&page.code).map(|(at, _)| at).collect();
        assert!(!uses.is_empty());
        for at in uses {
            assert!(
                html[..at].ends_with(&origin),
                "the code at byte {at} is not in the fragment: {html}"
            );
        }
        Then::Stop
    });
}

#[test]
fn a_huge_declared_length_with_a_wrong_host_is_refused_and_the_server_still_answers() {
    serving(Setup::shelf(), |page| {
        for length in [u64::MAX.to_string(), (1_u64 << 50).to_string()] {
            let head = format!(
                "POST /api/session HTTP/1.1\r\nHost: localhost\r\nContent-Length: {length}\r\n\r\n"
            );
            let reply = raw(page.port, head.as_bytes()).reply();

            assert_refused(&reply, 403, "flightdeck::request::host");
        }

        assert_eq!(Call::get(page.port, "/").send(page.port).status, 200);
        Then::Stop
    });
}

#[test]
fn a_refused_request_whose_body_never_arrives_does_not_block_the_next_request() {
    serving(Setup::shelf(), |page| {
        let head = format!(
            "GET /api/model HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Length: 5000\r\n\r\n",
            page.port
        );
        // The connection stays open and never sends the body it declares.
        let held = raw(page.port, head.as_bytes());

        assert_eq!(Call::get(page.port, "/").send(page.port).status, 200);
        assert_refused(&held.reply(), 401, "flightdeck::session::unauthorized");
        Then::Stop
    });
}

#[test]
fn a_connection_over_the_connection_limit_is_refused_and_the_others_are_still_answered() {
    serving(Setup::shelf(), |page| {
        // Each held connection sends part of a head and waits.
        let held: Vec<_> = (0..CONNECTION_LIMIT)
            .map(|_| raw(page.port, b"GET / HTTP/1.1\r\n"))
            .collect();

        let refused = Call::get(page.port, "/").send(page.port);

        assert_refused(&refused, 503, "flightdeck::request::busy");
        drop(held);
        // The server frees a slot once it sees a held connection close, so
        // the request is sent again until a slot is free.
        let answered = (0..1000)
            .map(|_| Call::get(page.port, "/").send(page.port))
            .find(|reply| reply.status != 503)
            .unwrap();
        assert_eq!(answered.status, 200);
        Then::Stop
    });
}

#[test]
fn a_request_head_over_the_size_limit_or_with_too_many_headers_is_refused() {
    serving(Setup::shelf(), |page| {
        let long = Call::get(page.port, "/")
            .header("X-Padding", &"a".repeat(HEAD_LIMIT))
            .send(page.port);
        let mut many = Call::get(page.port, "/");
        for index in 0..HEADER_LIMIT {
            many = many.header(&format!("X-Header-{index}"), "a");
        }
        let many = many.send(page.port);

        assert_refused(&long, 431, "flightdeck::request::head_too_large");
        assert_refused(&many, 431, "flightdeck::request::head_too_large");
        assert_eq!(Call::get(page.port, "/").send(page.port).status, 200);
        Then::Stop
    });
}

#[test]
fn every_reply_closes_its_connection() {
    serving(Setup::shelf(), |page| {
        let replies = [
            Call::get(page.port, "/").send(page.port),
            Call::get(page.port, "/api/model").send(page.port),
        ];

        for reply in replies {
            assert_eq!(reply.header("Connection"), Some("close"));
        }
        Then::Stop
    });
}

#[test]
fn a_session_request_whose_body_never_arrives_gets_408() {
    let settings = Settings::new()
        .poll(Duration::from_millis(5))
        .request_time(Duration::from_millis(200));
    serving(Setup::shelf().server(settings), |page| {
        let head = format!(
            "POST /api/session HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: 80\r\n\r\n",
            port = page.port
        );

        // The connection stays open and never sends the body it declares.
        let held = raw(page.port, head.as_bytes());

        assert_refused(&held.reply(), 408, "flightdeck::request::timeout");
        Then::Stop
    });
}

#[test]
fn a_token_that_arrives_after_the_idle_limit_is_refused_and_stops_the_server_before_its_next_poll()
{
    // The server next checks its idle time after 30 seconds, six times the
    // harness's wait for a server to stop by itself, so only the refused
    // request can stop it in time. A failed assertion stops it at that
    // poll.
    let settings = Settings::new().poll(Duration::from_secs(30));
    let ended = serving(Setup::shelf().server(settings), |page| {
        let token = page.token();
        page.clock.advance(IDLE_LIMIT);

        let reply = page.model(&token);

        assert_refused(&reply, 401, "flightdeck::session::unauthorized");
        Then::Wait
    });

    assert_eq!(ended, Ended::Idle);
}
