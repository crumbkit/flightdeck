use std::path::{Path, PathBuf};

use crumbkit_process::Cancel;

use super::{Mxv, RouteLink, Routes, WorkspaceName, WorkspaceRow, WorkspacesError, list, show};
use crate::spawn::Program;

/// Returns the path of the fake mxv in `tests/fixtures/mxv`.
fn fake_mxv() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mxv/mxv")
}

/// Returns the fake mxv with the variable `name` set to `value`.
fn mxv_with(name: &str, value: &str) -> Mxv {
    Mxv::new(Program::new(fake_mxv()).env(name, value))
}

/// Returns the row of the workspace `name`.
fn row<'a>(rows: &'a [WorkspaceRow], name: &str) -> &'a WorkspaceRow {
    rows.iter().find(|row| row.name == name).unwrap()
}

#[test]
fn the_list_joins_each_workspace_with_its_status_and_its_routes() {
    let listed = list(&Mxv::new(Program::new(fake_mxv())), &Cancel::new()).unwrap();

    let names: Vec<&str> = listed.rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "broken-status",
            "notes-scratch",
            "reading-list",
            "shelf-import"
        ]
    );
    let reading = row(&listed.rows, "reading-list");
    assert_eq!(reading.branch.as_deref(), Some("reading-list"));
    assert_eq!(reading.checkouts, ["reading", "shelf"]);
    assert_eq!(reading.ports.get("web"), Some(&41001));
    assert_eq!(reading.declared, Some(3));
    assert_eq!(reading.running, Some(2));
    assert_eq!(reading.pinned, Some(false));
    assert_eq!(
        reading.routes,
        [RouteLink {
            route: "web".to_owned(),
            url: "https://web.reading-list.mxv.localhost".to_owned(),
            unit: Some("web".to_owned()),
        }]
    );
    assert_eq!(row(&listed.rows, "notes-scratch").pinned, Some(true));
    assert_eq!(listed.summary.routes, Routes::Available);
}

#[test]
fn the_summary_counts_workspaces_with_a_unit_running_and_those_with_units_and_none_running() {
    let listed = list(&Mxv::new(Program::new(fake_mxv())), &Cancel::new()).unwrap();

    assert_eq!(listed.summary.workspaces, 4);
    assert_eq!(listed.summary.running, 1);
    assert_eq!(listed.summary.stopped, 1);
}

#[test]
fn a_failed_status_marks_its_own_row_and_no_other() {
    let listed = list(&Mxv::new(Program::new(fake_mxv())), &Cancel::new()).unwrap();

    let broken = row(&listed.rows, "broken-status");
    let failure = broken.status_error.as_ref().unwrap();
    assert_eq!(failure.code.as_deref(), Some("mxv::manifest::invalid"));
    assert_eq!(broken.declared, None);
    assert_eq!(broken.units, None);
    assert_eq!(broken.pinned, None);
    for other in ["notes-scratch", "reading-list", "shelf-import"] {
        assert!(row(&listed.rows, other).status_error.is_none(), "{other}");
    }
}

#[test]
fn an_mxv_without_proxy_status_leaves_routes_unavailable_and_still_lists() {
    let listed = list(&mxv_with("FAKE_MXV_PROXY", "refuse"), &Cancel::new()).unwrap();

    assert_eq!(
        listed.summary.routes,
        Routes::Unavailable {
            reason: "this mxv has no proxy status command".to_owned()
        }
    );
    assert!(listed.rows.iter().all(|row| row.routes.is_empty()));
    assert_eq!(row(&listed.rows, "reading-list").running, Some(2));
}

#[test]
fn a_failed_list_reports_mxvs_own_code() {
    let error = list(&mxv_with("FAKE_MXV_LIST", "fail"), &Cancel::new()).unwrap_err();

    let WorkspacesError::MxvFailed { command, code, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(*command, "list");
    assert_eq!(code.as_deref(), Some("mxv::store::unreadable"));
}

#[test]
fn a_missing_mxv_is_reported_as_missing() {
    let error = list(&Mxv::new(Program::new("/no/such/mxv")), &Cancel::new()).unwrap_err();

    assert!(
        matches!(&error, WorkspacesError::MxvMissing { program } if program == "mxv"),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "cannot start mxv: install mxv, or set FLIGHTDECK_MXV to its path"
    );
}

#[test]
fn show_leaves_out_a_route_that_the_proxy_does_not_serve_even_when_it_names_a_unit() {
    let name = WorkspaceName::new("reading-list").unwrap();

    let shown = show(&Mxv::new(Program::new(fake_mxv())), &name, &Cancel::new()).unwrap();

    let api = shown.units.iter().find(|unit| unit.name == "api").unwrap();
    assert!(api.routes.is_empty(), "{:?}", api.routes);
}

#[test]
fn show_gives_each_checkout_and_each_unit_with_its_route() {
    let name = WorkspaceName::new("reading-list").unwrap();

    let shown = show(&Mxv::new(Program::new(fake_mxv())), &name, &Cancel::new()).unwrap();

    assert_eq!(shown.name, "reading-list");
    assert!(!shown.pinned);
    let reading = &shown.checkouts[0];
    assert_eq!(reading.folder, "reading");
    assert_eq!(reading.branch.as_deref(), Some("reading-list"));
    assert_eq!(reading.changes, Some(2));
    let units: Vec<(&str, &str, Option<bool>)> = shown
        .units
        .iter()
        .map(|unit| (unit.name.as_str(), unit.state.as_str(), unit.ready))
        .collect();
    assert_eq!(
        units,
        [
            ("api", "running", Some(true)),
            ("web", "running", Some(false)),
            ("cache", "exited", None)
        ]
    );
    assert_eq!(
        shown.units[1].routes,
        ["https://web.reading-list.mxv.localhost"]
    );
    assert_eq!(
        shown.units[1].ports,
        ["READING_WEB_PORT=41001", "READING_API_PORT=41002"]
    );
    assert_eq!(shown.units[2].code, Some(137));
    assert!(shown.units[0].routes.is_empty());
}

#[test]
fn show_of_a_name_that_mxv_does_not_know_is_not_found() {
    let name = WorkspaceName::new("no-such-workspace").unwrap();

    let error = show(&Mxv::new(Program::new(fake_mxv())), &name, &Cancel::new()).unwrap_err();

    assert!(
        matches!(&error, WorkspacesError::NotFound { name } if name == "no-such-workspace"),
        "{error:?}"
    );
}

#[test]
fn neither_document_holds_an_absolute_path_a_log_or_sandbox_text() {
    let mxv = Mxv::new(Program::new(fake_mxv()));
    let listed = list(&mxv, &Cancel::new()).unwrap();
    let name = WorkspaceName::new("reading-list").unwrap();
    let shown = show(&mxv, &name, &Cancel::new()).unwrap();

    for text in [
        String::from_utf8(listed.to_json()).unwrap(),
        String::from_utf8(shown.to_json()).unwrap(),
    ] {
        assert!(!text.contains("/home/"), "{text}");
        assert!(!text.contains(".log"), "{text}");
        assert!(!text.contains("seatbelt"), "{text}");
        assert!(!text.contains("mirror"), "{text}");
    }
}
