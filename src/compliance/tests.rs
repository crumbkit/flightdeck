//! Unit tests of the compliance build, against the reports that the pinned
//! Board printed into `tests/fixtures/compliance/reports/` and changes to
//! them made here.

use jiff::{SignedDuration, Timestamp};
use serde_json::{Value, json};

use super::config::{Config, ConfigProblem};
use super::document::{self, Cell, ComplianceDocument, Inputs, ReportState, Row, Section};
use super::report::{self, Report, RuleSource, Unread};
use super::state::{self, CellState};

/// The config of the fixtures.
const CONFIG: &str = include_str!("../../tests/fixtures/compliance/compliance.toml");

/// The report of owner/app.
const APP: &str = include_str!("../../tests/fixtures/compliance/reports/owner/app.json");

/// The report of owner/api.
const API: &str = include_str!("../../tests/fixtures/compliance/reports/owner/api.json");

/// The report of owner/ci, which GitHub Actions made.
const CI: &str = include_str!("../../tests/fixtures/compliance/reports/owner/ci.json");

/// Returns the report `text` as JSON, to change before it is read.
fn value(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

/// Reads `value` as a report.
fn read(value: &Value) -> Result<Report, Unread> {
    report::parse(&serde_json::to_vec(value).unwrap())
}

/// No report.
const fn missing() -> Result<Report, Unread> {
    Err(Unread::Missing)
}

/// The time each fixture report was made.
fn generated() -> Timestamp {
    value(APP)["generated"].as_str().unwrap().parse().unwrap()
}

/// The build's inputs `hours` hours after the fixtures were made.
fn at_hours(hours: i64) -> Inputs {
    Inputs {
        built_at: generated()
            .checked_add(SignedDuration::from_hours(hours))
            .unwrap(),
        ..Inputs::default()
    }
}

/// The build's inputs `days` days after the fixtures were made.
fn at_days(days: i64) -> Inputs {
    at_hours(days * 24)
}

/// The document of the fixture config over `reports`, one for each of its
/// four repositories, built with `inputs`.
fn built(reports: [Result<Report, Unread>; 4], inputs: &Inputs) -> ComplianceDocument {
    document::build(&Config::parse(CONFIG).unwrap(), &reports, inputs)
}

/// The document of the three fixture reports and no report for owner/gone,
/// one hour after the reports were made.
fn fixtures() -> ComplianceDocument {
    built(
        [
            read(&value(APP)),
            read(&value(API)),
            read(&value(CI)),
            missing(),
        ],
        &at_hours(1),
    )
}

/// The document of `app` as owner/app's report and no other report, at
/// `days` days.
fn app_alone(app: &Value, days: i64) -> ComplianceDocument {
    built([read(app), missing(), missing(), missing()], &at_days(days))
}

/// The row of `id` with `source`, or the suite row of `id` when `source` is
/// `None`.
fn row<'a>(document: &'a ComplianceDocument, id: &str, source: Option<RuleSource>) -> &'a Row {
    document
        .rows
        .iter()
        .find(|row| row.id == id && row.source == source)
        .unwrap_or_else(|| panic!("no row {id}"))
}

/// The cell of `row` in the column of `repository`.
fn cell<'a>(row: &'a Row, repository: &str) -> &'a Cell {
    row.cells
        .iter()
        .find(|cell| cell.repository == repository)
        .unwrap()
}

/// The state of the imposed rule `id` in `repository`.
fn rule_state(document: &ComplianceDocument, id: &str, repository: &str) -> CellState {
    cell(row(document, id, Some(RuleSource::Imposed)), repository).state
}

/// The group `id` of `document`.
fn group<'a>(document: &'a ComplianceDocument, id: &str) -> &'a super::Group {
    document.groups.iter().find(|group| group.id == id).unwrap()
}

// The state table: one test per row.

#[test]
fn a_budget_within_its_max_reads_current() {
    assert_eq!(state::of_result("within"), CellState::Current);
}

#[test]
fn a_budget_over_its_max_reads_failing() {
    assert_eq!(state::of_result("over"), CellState::Failing);
}

#[test]
fn a_pin_behind_its_minimum_reads_failing() {
    assert_eq!(state::of_result("behind"), CellState::Failing);
}

#[test]
fn a_zero_rule_whose_measures_are_0_reads_current() {
    assert_eq!(state::of_result("zero"), CellState::Current);
}

#[test]
fn a_zero_rule_with_a_measure_not_0_reads_failing() {
    assert_eq!(state::of_result("not_zero"), CellState::Failing);
}

#[test]
fn a_zero_rule_whose_run_lacks_a_measure_reads_failing() {
    assert_eq!(state::of_result("missing_measure"), CellState::Failing);
}

#[test]
fn a_zero_rule_with_no_run_reads_stale() {
    assert_eq!(state::of_result("no_run"), CellState::Stale);
}

#[test]
fn a_zero_rule_the_report_calls_stale_reads_stale() {
    assert_eq!(state::of_result("stale"), CellState::Stale);
}

#[test]
fn a_rule_the_report_does_not_measure_reads_written() {
    assert_eq!(state::of_result("written"), CellState::Written);
}

#[test]
fn the_repositorys_own_unmeasured_rule_reads_not_readable() {
    assert_eq!(state::of_result("unmeasured"), CellState::NotReadable);
}

#[test]
fn an_unreadable_rule_reads_not_readable() {
    assert_eq!(state::of_result("unreadable"), CellState::NotReadable);
}

#[test]
fn a_result_a_later_board_adds_reads_not_readable() {
    assert_eq!(state::of_result("waiting_approval"), CellState::NotReadable);
}

/// A suite run of `outcome` for the state table's suite rows.
fn suite_run(outcome: &str) -> report::SuiteRun {
    report::SuiteRun {
        id: "/suites/s/runs/1".to_owned(),
        finished: Some("2026-11-24T00:00:00Z".to_owned()),
        commit: None,
        outcome: outcome.to_owned(),
        failed: Vec::new(),
        commits_since: None,
        commits_since_code: None,
    }
}

#[test]
fn a_suite_whose_newest_run_failed_a_case_reads_failing() {
    let run = suite_run("fail");
    assert_eq!(
        state::of_suite(Some(&run), None, Some(10), 100),
        CellState::Failing
    );
}

#[test]
fn a_suite_whose_newest_run_passed_within_the_window_reads_current() {
    let run = suite_run("pass");
    assert_eq!(
        state::of_suite(Some(&run), None, Some(100), 100),
        CellState::Current
    );
}

#[test]
fn a_suite_whose_newest_run_passed_before_the_window_reads_stale() {
    let run = suite_run("pass");
    assert_eq!(
        state::of_suite(Some(&run), None, Some(101), 100),
        CellState::Stale
    );
}

#[test]
fn a_suite_with_no_clean_full_run_reads_stale() {
    assert_eq!(state::of_suite(None, None, None, 100), CellState::Stale);
}

#[test]
fn a_suite_whose_runs_board_refused_to_read_reads_not_readable() {
    assert_eq!(
        state::of_suite(None, Some("runs::latest::refused"), None, 100),
        CellState::NotReadable
    );
}

#[test]
fn a_suite_run_of_an_outcome_a_later_board_adds_reads_not_readable() {
    let run = suite_run("cancelled");
    assert_eq!(
        state::of_suite(Some(&run), None, Some(1), 100),
        CellState::NotReadable
    );
}

// The fixture reports through the whole build.

#[test]
fn the_fixture_reports_give_each_cell_the_state_of_its_result() {
    let document = fixtures();
    assert_eq!(
        rule_state(&document, "/rules/rust/file-size", "owner/app"),
        CellState::Current
    );
    assert_eq!(
        rule_state(&document, "/rules/rust/file-size", "owner/api"),
        CellState::Failing
    );
    assert_eq!(
        rule_state(&document, "/rules/toolchains/rust", "owner/api"),
        CellState::Failing
    );
    assert_eq!(
        rule_state(&document, "/rules/rust/server", "owner/app"),
        CellState::Written
    );
    let own = row(
        &document,
        "/rules/rust/server",
        Some(RuleSource::Repository),
    );
    assert_eq!(cell(own, "owner/app").state, CellState::NotReadable);
    assert_eq!(cell(own, "owner/api").state, CellState::Absent);
    let suite = row(&document, "/suites/api-rust-standard", None);
    assert_eq!(suite.section, Section::Suite);
    assert_eq!(cell(suite, "owner/api").state, CellState::Failing);
    assert_eq!(cell(suite, "owner/app").state, CellState::Absent);
}

#[test]
fn rows_come_in_sections_measured_zero_suite_then_written() {
    let document = fixtures();
    let sections: Vec<Section> = document.rows.iter().map(|row| row.section).collect();
    let mut sorted = sections.clone();
    sorted.sort();
    assert_eq!(sections, sorted);
    assert_eq!(
        row(
            &document,
            "/rules/payments/v1-signer",
            Some(RuleSource::Imposed)
        )
        .section,
        Section::Zero
    );
}

#[test]
fn a_suite_run_older_than_stale_after_days_at_the_build_reads_stale() {
    // owner/app's suite ran four and a half days before its report.
    let suite = |days| {
        let document = app_alone(&value(APP), days);
        cell(
            row(&document, "/suites/app-rust-standard", None),
            "owner/app",
        )
        .state
    };
    assert_eq!(suite(2), CellState::Current);
    assert_eq!(suite(3), CellState::Stale);
}

#[test]
fn a_report_older_than_stale_after_days_reads_stale_and_so_do_its_current_cells() {
    let document = built(
        [read(&value(APP)), read(&value(API)), missing(), missing()],
        &at_days(8),
    );
    assert_eq!(document.repositories[0].report, ReportState::Stale);
    assert_eq!(
        rule_state(&document, "/rules/rust/file-size", "owner/app"),
        CellState::Stale
    );
    assert_eq!(
        rule_state(&document, "/rules/rust/file-size", "owner/api"),
        CellState::Failing,
        "a failure the report found stays failing"
    );
    assert_eq!(
        rule_state(&document, "/rules/rust/server", "owner/app"),
        CellState::Written
    );
}

#[test]
fn a_missing_report_gives_its_column_no_report_and_the_others_render() {
    let document = fixtures();
    let gone = &document.repositories[3];
    assert_eq!(gone.report, ReportState::NoReport);
    assert_eq!(gone.code.as_deref(), Some(document::NO_REPORT));
    let file_size = row(
        &document,
        "/rules/rust/file-size",
        Some(RuleSource::Imposed),
    );
    assert_eq!(cell(file_size, "owner/gone").state, CellState::NotReadable);
    assert_eq!(
        cell(file_size, "owner/gone").code.as_deref(),
        Some(document::NO_REPORT)
    );
    assert_eq!(cell(file_size, "owner/app").state, CellState::Current);
    assert_eq!(document.reports.no_report, 1);
}

#[test]
fn a_report_with_a_field_of_the_wrong_type_gives_its_column_invalid_report_with_the_path() {
    let mut api = value(API);
    api["rules"][1]["result"] = json!(7);
    let unread = read(&api).unwrap_err();
    assert!(
        matches!(&unread, Unread::Invalid { field: Some(field), .. } if field == "rules[1].result"),
        "{unread:?}"
    );
    let document = built(
        [read(&value(APP)), Err(unread), read(&value(CI)), missing()],
        &at_days(0),
    );
    let api = &document.repositories[1];
    assert_eq!(api.report, ReportState::InvalidReport);
    assert_eq!(api.code.as_deref(), Some(document::INVALID_REPORT));
    assert_eq!(api.field.as_deref(), Some("rules[1].result"));
    assert_eq!(
        rule_state(&document, "/rules/rust/file-size", "owner/ci"),
        CellState::Current
    );
}

#[test]
fn a_document_of_another_type_is_an_invalid_report_at_its_type() {
    let mut app = value(APP);
    app["$type"] = json!("rules/check/v1");
    assert!(matches!(
        read(&app),
        Err(Unread::Invalid { field: Some(field), .. }) if field == "$type"
    ));
}

#[test]
fn a_warning_that_names_a_measured_rule_leaves_its_state() {
    let mut api = value(API);
    api["refused"] = json!([{
        "code": "rules::report::member_unmatched",
        "rules": ["/rules/rust/file-size"],
        "message": "A member pattern matches no rule.",
    }]);
    let document = built([missing(), read(&api), missing(), missing()], &at_days(0));
    let api_cell = cell(
        row(
            &document,
            "/rules/rust/file-size",
            Some(RuleSource::Imposed),
        ),
        "owner/api",
    );
    assert_eq!(api_cell.state, CellState::Failing);
    assert_eq!(api_cell.code, None);
}

#[test]
fn a_rule_source_that_is_neither_imposed_nor_repository_is_an_invalid_report_at_its_path() {
    let mut app = value(APP);
    app["rules"][2]["source"] = json!("inherited");
    assert!(
        matches!(
            read(&app),
            Err(Unread::Invalid { field: Some(field), .. }) if field == "rules[2].source"
        ),
        "{:?}",
        read(&app)
    );
}

#[test]
fn a_report_of_another_repository_is_an_invalid_report_at_its_name() {
    let report = read(&value(API)).unwrap();
    assert!(matches!(
        report::named(report, "owner/app"),
        Err(Unread::Invalid { field: Some(field), .. }) if field == "repository.name"
    ));
    let report = read(&value(API)).unwrap();
    assert!(
        report::named(report, "Owner/API").is_ok(),
        "GitHub names ignore case"
    );
}

#[test]
fn a_suite_run_that_records_no_time_reads_not_readable_and_the_report_still_reads() {
    let mut app = value(APP);
    app["suites"][0]["run"]["finished"] = Value::Null;
    let document = app_alone(&app, 0);
    let suite = cell(
        row(&document, "/suites/app-rust-standard", None),
        "owner/app",
    );
    assert_eq!(suite.state, CellState::NotReadable);
    assert_eq!(suite.age_s, None);
    assert_eq!(document.repositories[0].report, ReportState::Current);
}

#[test]
fn a_zero_rule_measure_may_be_a_number_a_label_a_flag_or_null() {
    let mut app = value(APP);
    let zero = app["rules"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|rule| rule["id"] == "/rules/payments/v1-signer")
        .unwrap();
    zero["zero"]["measures"] =
        json!({"remaining": 2, "label": "two left", "done": false, "unknown": null});
    let document = app_alone(&app, 0);
    let facts = cell(
        row(
            &document,
            "/rules/payments/v1-signer",
            Some(RuleSource::Imposed),
        ),
        "owner/app",
    )
    .facts
    .clone()
    .unwrap();
    let measures = facts.zero.unwrap().measures;
    assert_eq!(
        measures["remaining"],
        Some(super::MeasureValue::Number(2.0))
    );
    assert_eq!(
        measures["label"],
        Some(super::MeasureValue::Label("two left".to_owned()))
    );
    assert_eq!(measures["done"], Some(super::MeasureValue::Flag(false)));
    assert_eq!(measures["unknown"], None);
}

#[test]
fn two_reports_that_give_one_rule_different_checkers_share_one_row() {
    // owner/api read an older commit of the set, where the file-size rule
    // was a written rule.
    let mut api = value(API);
    for rule in api["rules"].as_array_mut().unwrap() {
        if rule["id"] == "/rules/rust/file-size" {
            rule["checker"] = Value::Null;
            rule["result"] = json!("written");
        }
    }
    let document = built(
        [read(&value(APP)), read(&api), missing(), missing()],
        &at_days(0),
    );
    let rows: Vec<&Row> = document
        .rows
        .iter()
        .filter(|row| row.id == "/rules/rust/file-size")
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].section,
        Section::Measured,
        "the first report's checker"
    );
    assert_eq!(cell(rows[0], "owner/api").state, CellState::Written);
    assert_eq!(document.repositories[1].counts.written, 2);
}

#[test]
fn a_report_on_the_rules_commit_gives_its_rule_the_section_ahead_of_an_earlier_column() {
    // owner/app read an older commit of the set, where the file-size rule
    // was a measured rule; owner/api read the current one, where it is a
    // written rule with another requirement.
    let mut app = value(APP);
    app["board"]["rules_commit"] = json!("0".repeat(40));
    let mut api = value(API);
    let current = api["board"]["rules_commit"].as_str().unwrap().to_owned();
    for rule in api["rules"].as_array_mut().unwrap() {
        if rule["id"] == "/rules/rust/file-size" {
            rule["checker"] = Value::Null;
            rule["result"] = json!("written");
            rule["requirement"] = json!("The current requirement.");
        }
    }
    let inputs = Inputs {
        rules_commit: Some(current),
        ..at_days(0)
    };
    let document = built([read(&app), read(&api), missing(), missing()], &inputs);
    let file_size = row(
        &document,
        "/rules/rust/file-size",
        Some(RuleSource::Imposed),
    );
    assert_eq!(file_size.section, Section::Written);
    assert_eq!(
        file_size.requirement.as_deref(),
        Some("The current requirement.")
    );
    assert_eq!(file_size.checker, None);
    assert_eq!(cell(file_size, "owner/app").state, CellState::Current);
}

// Drift: one test each.

/// The drift of `report` in the fixture config, with `rules_commit`.
fn drift_of(report: &Value, rules_commit: Option<&str>) -> super::Drift {
    let config = Config::parse(CONFIG).unwrap();
    super::Drift::of(&read(report).unwrap(), &config.imposed, rules_commit)
}

#[test]
fn drift_copies_the_reports_extra_rules() {
    assert_eq!(drift_of(&value(APP), None).extra, ["/rules/app/handlers"]);
}

#[test]
fn drift_copies_the_reports_differing_rules() {
    let drift = drift_of(&value(APP), None);
    assert_eq!(drift.differs.len(), 1);
    assert_eq!(drift.differs[0].id, "/rules/rust/server");
    assert_eq!(drift.differs[0].fields, ["requirement"]);
}

#[test]
fn drift_copies_the_imposed_rules_that_reach_no_file() {
    assert_eq!(drift_of(&value(API), None).not_reached, ["/rules/go/vet"]);
}

#[test]
fn a_report_from_another_rules_board_or_root_is_another_set() {
    assert!(!drift_of(&value(API), None).other_set);
    let mut from = value(API);
    from["board"]["rules_from"] = json!("someone/else");
    assert!(drift_of(&from, None).other_set);
    let mut root = value(API);
    root["imposed"]["root"] = json!("/rules/imposed");
    assert!(drift_of(&root, None).other_set);
}

#[test]
fn a_report_of_an_older_rules_commit_is_behind_and_without_rules_commit_behind_is_null() {
    let report = value(API);
    let read_commit = report["board"]["rules_commit"].as_str().unwrap().to_owned();
    assert_eq!(drift_of(&report, Some(&read_commit)).behind, Some(false));
    assert_eq!(drift_of(&report, Some(&"0".repeat(40))).behind, Some(true));
    assert_eq!(drift_of(&report, None).behind, None);
}

#[test]
fn drift_never_changes_a_cells_state() {
    let mut app = value(APP);
    app["board"]["rules_from"] = json!("someone/else");
    let document = app_alone(&app, 0);
    assert!(document.repositories[0].drift.as_ref().unwrap().other_set);
    assert_eq!(
        rule_state(&document, "/rules/rust/file-size", "owner/app"),
        CellState::Current
    );
}

// What the build copies from a report.

#[test]
fn the_source_run_copies_from_the_report_and_is_null_for_a_local_report() {
    let document = fixtures();
    assert_eq!(document.repositories[0].source_run, None);
    let ci = document.repositories[2].source_run.as_ref().unwrap();
    assert_eq!((ci.run_id, ci.run_number), (9001, 7));
    assert_eq!(ci.repository, "owner/ci");
    assert_eq!(Some(&ci.commit), document.repositories[2].commit.as_ref());
}

#[test]
fn commits_since_and_its_code_copy_from_the_suite_run_nulls_included() {
    let document = fixtures();
    let counted = cell(
        row(&document, "/suites/app-rust-standard", None),
        "owner/app",
    );
    let run = counted.run.as_ref().unwrap();
    assert_eq!(
        (run.commits_since, run.commits_since_code.as_deref()),
        (Some(1), None)
    );
    let mut app = value(APP);
    app["suites"][0]["run"]["commits_since"] = Value::Null;
    app["suites"][0]["run"]["commits_since_code"] = json!("rules::report::run_commit_missing");
    let document = app_alone(&app, 0);
    let uncounted = cell(
        row(&document, "/suites/app-rust-standard", None),
        "owner/app",
    );
    let run = uncounted.run.as_ref().unwrap();
    assert_eq!(
        (run.commits_since, run.commits_since_code.as_deref()),
        (None, Some("rules::report::run_commit_missing"))
    );
    assert_eq!(
        uncounted.state,
        CellState::Current,
        "commits since never change a state"
    );
}

#[test]
fn each_row_carries_the_source_the_report_gives_its_rule() {
    let document = fixtures();
    let sources: Vec<Option<RuleSource>> = document
        .rows
        .iter()
        .filter(|row| row.id == "/rules/rust/server")
        .map(|row| row.source)
        .collect();
    assert_eq!(
        sources,
        [Some(RuleSource::Imposed), Some(RuleSource::Repository)]
    );
}

// Groups.

#[test]
fn a_rule_in_two_groups_counts_in_both_roll_ups() {
    let document = fixtures();
    for id in ["/rules/groups/rust", "/rules/groups/server"] {
        let members = &group(&document, id).members;
        assert!(
            members.iter().any(|member| member == "/rules/rust/server"),
            "{id}: {members:?}"
        );
    }
    let rust = group(&document, "/rules/groups/rust");
    assert_eq!(rust.title, "Rust");
    let api = rust
        .cells
        .iter()
        .find(|cell| cell.repository == "owner/api")
        .unwrap();
    assert_eq!(api.state, CellState::Failing);
    assert_eq!(
        api.counts.as_ref().unwrap()["over"],
        1,
        "the report's counts are kept"
    );
}

#[test]
fn a_stale_member_makes_its_group_stale() {
    // owner/ci's rust group holds only current and written members, until
    // its report is older than the window.
    let rust_state = |days| {
        let document = built(
            [missing(), missing(), read(&value(CI)), missing()],
            &at_days(days),
        );
        group(&document, "/rules/groups/rust").cells[2].state
    };
    assert_eq!(rust_state(0), CellState::Current);
    assert_eq!(rust_state(8), CellState::Stale);
}

#[test]
fn the_repositorys_own_copy_of_an_imposed_rule_leaves_its_groups_roll_up_as_it_is() {
    // owner/app holds its own /rules/rust/server, which Board does not
    // measure. Board rolls owner/app's rust group up to passing.
    let document = fixtures();
    let rust = group(&document, "/rules/groups/rust");
    let app = rust
        .cells
        .iter()
        .find(|cell| cell.repository == "owner/app")
        .unwrap();
    assert_eq!(app.state, CellState::Current);
}

#[test]
fn a_roll_up_takes_the_worst_state_failing_first_and_absent_last() {
    assert_eq!(state::worst(CellState::WORST_FIRST), CellState::Failing);
    assert_eq!(
        state::worst([
            CellState::Written,
            CellState::Current,
            CellState::NotReadable
        ]),
        CellState::NotReadable
    );
    assert_eq!(state::worst([]), CellState::Absent);
}

// The config.

#[test]
fn the_fixture_config_reads_with_its_defaults_applied() {
    let config = Config::parse(CONFIG).unwrap();
    assert_eq!(config.imposed.root, "/rules");
    assert_eq!(config.every_hours, Some(24));
    assert_eq!(config.repositories.len(), 4);
}

#[test]
fn a_config_of_another_schema_is_refused_at_its_line() {
    let text = CONFIG.replace(
        "schema = \"flightdeck/compliance-config/v1\"",
        "schema = \"flightdeck/compliance-config/v2\"",
    );
    let problem = Config::parse(&text).unwrap_err();
    assert_eq!(problem.line, Some(3), "{}", problem.message);
}

#[test]
fn a_repository_named_twice_is_refused_at_its_second_entry() {
    let text = format!("{CONFIG}\n[[repositories]]\nname = \"Owner/App\"\n");
    let ConfigProblem { line, message } = Config::parse(&text).unwrap_err();
    assert!(message.contains("named twice"), "{message}");
    assert_eq!(line, Some(CONFIG.lines().count() + 2));
}

#[test]
fn a_repository_name_that_leaves_the_reports_directory_is_refused() {
    let text = format!("{CONFIG}\n[[repositories]]\nname = \"owner/..\"\n");
    let problem = Config::parse(&text).unwrap_err();
    assert!(
        problem.message.contains("owner/repo"),
        "{}",
        problem.message
    );
}

#[test]
fn a_config_that_is_not_toml_is_refused_at_its_line() {
    let problem =
        Config::parse("schema = \"flightdeck/compliance-config/v1\"\ntitle = \n").unwrap_err();
    assert_eq!(problem.line, Some(2), "{}", problem.message);
}

#[test]
fn a_key_the_config_does_not_know_is_refused() {
    let text = CONFIG.replace("stale_after_days = 7", "stale_after = 7");
    assert!(Config::parse(&text).is_err());
}

#[test]
fn the_next_build_is_every_hours_after_this_one() {
    let document = fixtures();
    let built: Timestamp = document.built_at.parse().unwrap();
    let next: Timestamp = document.next_build_at.as_deref().unwrap().parse().unwrap();
    assert_eq!(next.duration_since(built), SignedDuration::from_hours(24));
}

#[test]
fn a_stale_after_days_of_0_is_refused_at_its_line() {
    let text = CONFIG.replace("stale_after_days = 7", "stale_after_days = 0");
    let problem = Config::parse(&text).unwrap_err();
    assert_eq!(problem.line, Some(5), "{}", problem.message);
}

#[test]
fn an_every_hours_of_0_is_refused_at_its_line() {
    let text = CONFIG.replace("every_hours = 24", "every_hours = 0");
    let problem = Config::parse(&text).unwrap_err();
    assert!(
        problem.message.contains("every_hours"),
        "{}",
        problem.message
    );
    assert_eq!(problem.line, Some(18), "{}", problem.message);
}
