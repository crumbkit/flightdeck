//! `flightdeck compliance fetch` through the binary, against a fake `gh` on
//! `PATH` that replays the GitHub answers recorded in
//! `tests/fixtures/gh/github/`, then `compliance build` over what it wrote.
//!
//! The recorded repositories are:
//!
//! - owner/ci, whose newest run (#7) uploaded its report, which is
//!   `tests/fixtures/compliance/reports/owner/ci.json`, with 3 commits on
//!   main since;
//! - owner/app, whose three newest runs (#15, #14 and #13) ran code from a
//!   fork or for a pull request and uploaded a report, whose next run (#12)
//!   uploaded nothing and whose earlier run (#11) uploaded owner/app's
//!   fixture report, and whose compare GitHub refuses;
//! - owner/api, whose one run's artifact has expired;
//! - owner/gone, whose runs GitHub refuses with HTTP 404;
//! - owner/bad, whose one run's address is not a GitHub page.
//!
//! The fake gh is a shell script, so these tests run on Unix alone.

#![cfg(test)]
#![cfg(unix)]
#![expect(
    clippy::disallowed_methods,
    reason = "the test starts the flightdeck binary"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// The token the tests give gh, which nothing flightdeck writes may hold.
const TOKEN: &str = "fixture-token-that-must-not-leak-0123456789";

/// Returns the path of `relative` in the repository.
fn here(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// The config of the fetch tests: the recorded repositories, a path source
/// and a url source.
const CONFIG: &str = r#"schema = "flightdeck/compliance-config/v1"
title = "Compliance"

[source]
kind = "github-artifact"
workflow = "ci.yml"
artifact = "rules-report"
branch = "main"
every_hours = 24

[imposed]
name = "controls"
from = "owner/controls"

[[repositories]]
name = "owner/ci"

[[repositories]]
name = "owner/app"

[[repositories]]
name = "owner/api"

[[repositories]]
name = "owner/gone"

[[repositories]]
name = "owner/local"
source = { kind = "path", path = "local.json" }

[[repositories]]
name = "owner/web"
source = { kind = "url", url = "https://reports.example.invalid/web.json" }
"#;

/// A directory with the config, the path source's report and an empty
/// reports directory, and the file the fake gh logs its arguments to.
struct Place {
    root: tempfile::TempDir,
}

impl Place {
    /// Returns a place whose config is `config`.
    fn new(config: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("compliance.toml"), config).unwrap();
        std::fs::write(root.path().join("local.json"), b"{\"local\": true}\n").unwrap();
        std::fs::create_dir(root.path().join("reports")).unwrap();
        Self { root }
    }

    /// The config's path.
    fn config(&self) -> PathBuf {
        self.root.path().join("compliance.toml")
    }

    /// The reports directory.
    fn reports(&self) -> PathBuf {
        self.root.path().join("reports")
    }

    /// The path of `relative` in the reports directory.
    fn report(&self, relative: &str) -> PathBuf {
        self.reports().join(relative)
    }

    /// Writes `text` to `relative` in the reports directory.
    fn seed(&self, relative: &str, text: &str) {
        let path = self.report(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// The arguments the fake gh was started with, one run per line.
    fn gh_log(&self) -> String {
        std::fs::read_to_string(self.root.path().join("gh.log")).unwrap_or_default()
    }

    /// Runs flightdeck with `args`, with the fake gh on `PATH` when `with_gh`
    /// is true. Without it, `PATH` names only an empty directory, because a
    /// CI runner installs a real gh in `/usr/bin`.
    fn flightdeck(&self, args: &[&str], with_gh: bool) -> Output {
        let state = self.root.path().join("state");
        let path = if with_gh {
            format!("{}:/usr/bin:/bin", here("tests/fixtures/gh/bin").display())
        } else {
            let empty = self.root.path().join("empty-bin");
            std::fs::create_dir_all(&empty).unwrap();
            empty.display().to_string()
        };
        Command::new(env!("CARGO_BIN_EXE_flightdeck"))
            .args(args)
            .env_clear()
            .env("PATH", path)
            .env("XDG_STATE_HOME", state)
            .env("GH_TOKEN", TOKEN)
            .env("GH_ANSWERS", here("tests/fixtures/gh/github"))
            .env("GH_LOG", self.root.path().join("gh.log"))
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    /// Runs `flightdeck compliance fetch --json --full` over the place.
    fn fetch(&self, with_gh: bool) -> Output {
        let config = self.config();
        let reports = self.reports();
        self.flightdeck(
            &[
                "compliance",
                "fetch",
                "--config",
                config.to_str().unwrap(),
                "--reports",
                reports.to_str().unwrap(),
                "--json",
                "--full",
            ],
            with_gh,
        )
    }

    /// Reads the JSON file at `relative` in the reports directory.
    fn json(&self, relative: &str) -> Value {
        serde_json::from_slice(&std::fs::read(self.report(relative)).unwrap()).unwrap()
    }
}

/// Fails with the standard error unless `output` exited with `code`.
fn assert_exit(output: &Output, code: i32) {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The rows that a `--json` fetch printed.
fn rows(output: &Output) -> Vec<Value> {
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    document["data"].as_array().unwrap().clone()
}

/// The row of `repository` among `rows`.
fn row<'a>(rows: &'a [Value], repository: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["repository"] == repository)
        .unwrap_or_else(|| panic!("no row of {repository} in {rows:?}"))
}

/// The bytes of the fixture report of `name`.
fn fixture_report(name: &str) -> Vec<u8> {
    std::fs::read(here(&format!(
        "tests/fixtures/compliance/reports/owner/{name}.json"
    )))
    .unwrap()
}

#[test]
fn fetch_writes_the_report_and_its_fetch_record_for_a_run_that_uploaded() {
    let place = Place::new(CONFIG);
    let output = place.fetch(true);
    assert_exit(&output, 1);
    assert_eq!(
        std::fs::read(place.report("owner/ci.json")).unwrap(),
        fixture_report("ci"),
        "the report is written unchanged"
    );
    let record = place.json("owner/ci.fetch.json");
    assert_eq!(record["schema"], "flightdeck/compliance-fetch/v1");
    assert_eq!(record["repository"], "owner/ci");
    assert_eq!(record["newest_run"]["run_number"], 7);
    assert_eq!(record["newest_run"]["uploaded"], true);
    assert_eq!(
        record["newest_run"]["url"],
        "https://github.com/owner/ci/actions/runs/9001"
    );
    assert_eq!(record["report_run"]["run_id"], 9001);
    assert_eq!(record["commits_since"], 3);
    assert_eq!(record["commits_since_code"], Value::Null);
    let ci = rows(&output);
    let ci = row(&ci, "owner/ci");
    assert_eq!(ci["state"], "fetched");
    assert_eq!(
        (&ci["newest_run"], &ci["report_run"]),
        (&7.into(), &7.into())
    );
}

#[test]
fn fetch_names_the_newest_run_when_only_an_earlier_run_uploaded() {
    let place = Place::new(CONFIG);
    let output = place.fetch(true);
    assert_exit(&output, 1);
    assert_eq!(
        std::fs::read(place.report("owner/app.json")).unwrap(),
        fixture_report("app")
    );
    let record = place.json("owner/app.fetch.json");
    assert_eq!(record["newest_run"]["run_number"], 12);
    assert_eq!(record["newest_run"]["uploaded"], false);
    assert_eq!(
        record["newest_run"]["commit"],
        "e19b04f0e5a3c2d1b0a9f8e7d6c5b4a3f2e1d0c9"
    );
    assert_eq!(record["report_run"]["run_number"], 11);
    // GitHub refused the compare, so the count is left out with its code.
    assert_eq!(record["commits_since"], Value::Null);
    assert_eq!(
        record["commits_since_code"],
        "flightdeck::compliance::compare_failed"
    );
    let rows = rows(&output);
    assert_eq!(row(&rows, "owner/app")["state"], "no_report");
}

#[test]
fn an_expired_artifact_leaves_a_fetch_record_with_no_report_and_removes_the_old_report() {
    let place = Place::new(CONFIG);
    place.seed("owner/api.json", "old report\n");
    let output = place.fetch(true);
    assert_exit(&output, 1);
    assert!(!place.report("owner/api.json").exists());
    let record = place.json("owner/api.fetch.json");
    assert_eq!(record["newest_run"]["run_number"], 3);
    assert_eq!(record["newest_run"]["uploaded"], false);
    assert_eq!(record["report_run"], Value::Null);
    let rows = rows(&output);
    assert_eq!(row(&rows, "owner/api")["state"], "no_report");
}

#[test]
fn a_refused_run_list_fails_its_row_with_ghs_message_and_leaves_its_old_files() {
    let place = Place::new(CONFIG);
    place.seed("owner/gone.json", "old report\n");
    place.seed("owner/gone.fetch.json", "old record\n");
    let output = place.fetch(true);
    assert_exit(&output, 1);
    let rows = rows(&output);
    let gone = row(&rows, "owner/gone");
    assert_eq!(gone["state"], "failed");
    assert_eq!(gone["code"], "flightdeck::compliance::gh_failed");
    assert!(
        gone["message"].as_str().unwrap().contains("HTTP 404"),
        "{gone}"
    );
    assert_eq!(
        std::fs::read_to_string(place.report("owner/gone.json")).unwrap(),
        "old report\n"
    );
    assert_eq!(
        std::fs::read_to_string(place.report("owner/gone.fetch.json")).unwrap(),
        "old record\n"
    );
}

#[test]
fn a_path_source_is_copied_and_a_url_source_is_refused() {
    let place = Place::new(CONFIG);
    place.seed(
        "owner/local.fetch.json",
        "a record of runs the copy does not have\n",
    );
    let output = place.fetch(true);
    assert_exit(&output, 1);
    assert_eq!(
        std::fs::read_to_string(place.report("owner/local.json")).unwrap(),
        "{\"local\": true}\n"
    );
    assert!(!place.report("owner/local.fetch.json").exists());
    let rows = rows(&output);
    assert_eq!(row(&rows, "owner/local")["state"], "copied");
    let web = row(&rows, "owner/web");
    assert_eq!(web["state"], "failed");
    assert_eq!(
        web["code"],
        "flightdeck::compliance::url_source_unsupported"
    );
    assert!(!place.report("owner/web.json").exists());
}

#[test]
fn fetch_prints_failed_rows_first_and_the_rest_in_the_configs_order() {
    let place = Place::new(CONFIG);
    let output = place.fetch(true);
    assert_exit(&output, 1);
    let order: Vec<String> = rows(&output)
        .iter()
        .map(|row| {
            format!(
                "{} {}",
                row["repository"].as_str().unwrap(),
                row["state"].as_str().unwrap()
            )
        })
        .collect();
    assert_eq!(
        order,
        [
            "owner/gone failed",
            "owner/web failed",
            "owner/ci fetched",
            "owner/app no_report",
            "owner/api no_report",
            "owner/local copied",
        ]
    );
}

#[test]
fn fetch_exits_0_when_no_repository_failed() {
    let config = CONFIG
        .split("[[repositories]]")
        .take(3)
        .collect::<Vec<_>>()
        .join("[[repositories]]");
    let place = Place::new(&config);
    let output = place.fetch(true);
    assert_exit(&output, 0);
    assert_eq!(rows(&output).len(), 2);
}

#[test]
fn neither_a_fetch_record_nor_ghs_arguments_hold_the_token_or_the_runs_actor() {
    let place = Place::new(CONFIG);
    let output = place.fetch(true);
    assert_exit(&output, 1);
    let log = place.gh_log();
    assert!(log.contains("repos/owner/ci/actions"), "{log}");
    let mut texts = vec![log, String::from_utf8_lossy(&output.stdout).into_owned()];
    for name in ["ci", "app", "api"] {
        texts.push(
            std::fs::read_to_string(place.report(&format!("owner/{name}.fetch.json"))).unwrap(),
        );
    }
    for text in texts {
        assert!(!text.contains(TOKEN), "{text}");
        assert!(!text.contains("actor"), "{text}");
        assert!(!text.contains("a-person"), "{text}");
    }
}

#[test]
fn without_gh_on_path_fetch_fails_with_no_gh_and_writes_nothing() {
    let place = Place::new(CONFIG);
    let output = place.fetch(false);
    assert_exit(&output, 1);
    let failure: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(failure["code"], "flightdeck::compliance::no_gh");
    assert_eq!(std::fs::read_dir(place.reports()).unwrap().count(), 0);
}

#[test]
fn a_failed_write_fails_its_row_and_leaves_the_old_files_intact() {
    use std::os::unix::fs::PermissionsExt as _;

    let config = CONFIG
        .split("[[repositories]]")
        .take(2)
        .collect::<Vec<_>>()
        .join("[[repositories]]");
    let place = Place::new(&config);
    place.seed("owner/ci.json", "old report\n");
    let owner = place.report("owner");
    std::fs::set_permissions(&owner, std::fs::Permissions::from_mode(0o555)).unwrap();
    let output = place.fetch(true);
    std::fs::set_permissions(&owner, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_exit(&output, 1);
    let rows = rows(&output);
    let ci = row(&rows, "owner/ci");
    assert_eq!(ci["code"], "flightdeck::compliance::write_failed", "{ci}");
    assert_eq!(
        std::fs::read_to_string(place.report("owner/ci.json")).unwrap(),
        "old report\n"
    );
    assert!(!place.report("owner/ci.fetch.json").exists());
}

#[test]
fn a_build_over_the_fetched_reports_names_each_columns_runs_and_the_rules_branch() {
    let place = Place::new(CONFIG);
    assert_exit(&place.fetch(true), 1);
    let config = place.config();
    let reports = place.reports();
    let output = place.flightdeck(
        &[
            "compliance",
            "build",
            "--config",
            config.to_str().unwrap(),
            "--reports",
            reports.to_str().unwrap(),
            "--built-at",
            "2026-11-26T06:00:00Z",
            "--rules-branch",
            "main",
            "--json",
        ],
        false,
    );
    assert_exit(&output, 0);
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let data = &document["data"];
    assert_eq!(data["imposed"]["branch"], "main");
    let column = |name: &str| {
        data["repositories"]
            .as_array()
            .unwrap()
            .iter()
            .find(|column| column["name"] == name)
            .unwrap()
            .clone()
    };
    let ci = column("owner/ci");
    assert_eq!(ci["report"], "current");
    assert_eq!(ci["commits_since"], 3);
    assert_eq!(ci["every_hours"], 24);
    assert_eq!(ci["newest_run"]["age_s"], 7 * 3600 + 50 * 60);
    let app = column("owner/app");
    assert_eq!(app["report"], "no_report");
    assert_eq!(app["newest_run"]["run_number"], 12);
    assert_eq!(app["report_run"]["run_number"], 11);
    let handlers = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "/rules/app/handlers")
        .expect("owner/app's earlier report still makes its rows");
    let app_index = data["repositories"]
        .as_array()
        .unwrap()
        .iter()
        .position(|column| column["name"] == "owner/app")
        .unwrap();
    assert_eq!(handlers["cells"][app_index]["state"], "not_readable");
    assert_eq!(
        handlers["cells"][app_index]["code"],
        "flightdeck::compliance::no_report"
    );
    let api = column("owner/api");
    assert_eq!(api["report"], "no_report");
    assert_eq!(api["newest_run"]["uploaded"], false);
    let schema: Value = serde_json::from_slice(
        &std::fs::read(here("tests/golden/schemas/flightdeck-compliance-v1.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let problems: Vec<String> = validator
        .iter_errors(&document)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(problems.is_empty(), "{problems:#?}");
}

/// The config of `CONFIG` with only its first `count` repositories.
fn first_repositories(count: usize) -> String {
    CONFIG
        .split("[[repositories]]")
        .take(count + 1)
        .collect::<Vec<_>>()
        .join("[[repositories]]")
}

#[test]
fn fetch_passes_over_runs_from_a_fork_or_for_a_pull_request_for_the_report_and_the_newest_run() {
    let place = Place::new(CONFIG);
    let output = place.fetch(true);
    assert_exit(&output, 1);
    let record = place.json("owner/app.fetch.json");
    assert_eq!(record["newest_run"]["run_number"], 12, "{record}");
    assert_eq!(record["report_run"]["run_number"], 11, "{record}");
    assert_eq!(
        std::fs::read(place.report("owner/app.json")).unwrap(),
        fixture_report("app")
    );
    let log = place.gh_log();
    for run in ["9103", "9104", "9105"] {
        assert!(!log.contains(&format!("runs/{run}/")), "{log}");
        assert!(!log.contains(&format!("download {run} ")), "{log}");
    }
    assert!(log.contains("runs/9102/artifacts"), "{log}");
}

#[test]
fn a_run_whose_address_is_not_a_github_page_fails_its_row_and_writes_nothing() {
    let place = Place::new(&CONFIG.replace("name = \"owner/ci\"", "name = \"owner/bad\""));
    let output = place.fetch(true);
    assert_exit(&output, 1);
    let rows = rows(&output);
    let bad = row(&rows, "owner/bad");
    assert_eq!(bad["state"], "failed");
    assert_eq!(bad["code"], "flightdeck::compliance::gh_failed");
    assert!(
        bad["message"]
            .as_str()
            .unwrap()
            .contains("https://github.com/"),
        "{bad}"
    );
    assert!(!place.report("owner/bad.json").exists());
    assert!(!place.report("owner/bad.fetch.json").exists());
}

#[test]
fn a_record_that_cannot_be_written_puts_the_old_report_back() {
    let place = Place::new(&first_repositories(1));
    place.seed("owner/ci.json", "old report\n");
    // A directory where the record goes, which no file can be renamed over.
    place.seed("owner/ci.fetch.json/held", "a file\n");
    let output = place.fetch(true);
    assert_exit(&output, 1);
    let rows = rows(&output);
    let ci = row(&rows, "owner/ci");
    assert_eq!(ci["state"], "failed", "{ci}");
    assert_eq!(ci["code"], "flightdeck::compliance::write_failed", "{ci}");
    assert_eq!(
        std::fs::read_to_string(place.report("owner/ci.json")).unwrap(),
        "old report\n"
    );
    assert!(place.report("owner/ci.fetch.json/held").exists());
}

#[test]
fn a_record_that_cannot_be_written_leaves_no_report_where_there_was_none() {
    let place = Place::new(&first_repositories(1));
    place.seed("owner/ci.fetch.json/held", "a file\n");
    let output = place.fetch(true);
    assert_exit(&output, 1);
    assert!(!place.report("owner/ci.json").exists());
}

#[test]
fn fetch_removes_the_temporary_files_a_killed_fetch_left_in_an_owners_directory() {
    let place = Place::new(&first_repositories(1));
    place.seed("owner/.fetch-AbC123/rules-report.json", "half a report\n");
    place.seed("owner/.fetch-XyZ789", "half a record\n");
    place.seed("owner/keep.json", "another file\n");
    let output = place.fetch(true);
    assert_exit(&output, 0);
    let left: Vec<String> = std::fs::read_dir(place.report("owner"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".fetch-"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
    assert!(place.report("owner/keep.json").exists());
}
