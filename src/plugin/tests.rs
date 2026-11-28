use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{MANIFEST, SCHEMA, call, install, plugins_directory};

/// Returns the path of the fake board in the fixtures.
fn fake_board() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/board/fake/board")
}

/// Calls the plugin with `request` against the fake board, and returns its
/// document and whether it failed.
fn answer(request: &Value) -> (Value, bool) {
    let bytes = serde_json::to_vec(request).unwrap();
    let answered = call(bytes.as_slice(), Some(fake_board().as_os_str()));
    (
        serde_json::from_slice(&answered.document).unwrap(),
        answered.failed,
    )
}

/// Returns a check request for `path`.
fn check_request(path: &str) -> Value {
    json!({
        "$type": "board/call/v1",
        "capability": "check",
        "input": {"path": path},
        "board": {"root": "/boards/home"},
    })
}

#[test]
fn the_manifest_names_the_shipped_type_and_flightdecks_version() {
    let manifest: Value = serde_json::from_str(MANIFEST).unwrap();

    assert_eq!(manifest["name"], "flightdeck");
    assert_eq!(manifest["types"], json!(["flightdeck/workflow-graph/v1"]));
    assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(manifest["run"], json!(["flightdeck", "plugin", "call"]));
    assert_eq!(SCHEMA, crate::workflow_graph::schema_text());
}

#[test]
fn a_plugin_call_for_check_answers_a_call_result_with_the_problems() {
    let (tangled, tangled_failed) = answer(&check_request("/workflow-graphs/tangled"));
    let (clean, _) = answer(&check_request("/workflow-graphs/small-flow"));

    assert!(!tangled_failed);
    assert_eq!(tangled["$type"], "board/call-result/v1");
    assert_eq!(tangled["output"]["path"], "/workflow-graphs/tangled");
    assert_eq!(
        tangled["output"]["problems"][0]["code"],
        "flightdeck::graph::unknown_end"
    );
    assert_eq!(tangled["output"]["problems"][0]["pointer"], "/edges/2/to");
    assert_eq!(clean["output"]["problems"], json!([]));
}

#[test]
fn a_plugin_call_refuses_a_path_that_is_not_a_graph_and_names_a_degraded_one() {
    let (outside, outside_failed) = answer(&check_request("/changes/a"));
    let (degraded, _) = answer(&check_request("/workflow-graphs/broken"));
    let (missing, _) = answer(&check_request("/workflow-graphs/no-such-flow"));
    let (garbled, _) = {
        let answered = call(&b"not json"[..], Some(fake_board().as_os_str()));
        (
            serde_json::from_slice::<Value>(&answered.document).unwrap(),
            answered.failed,
        )
    };

    assert!(outside_failed);
    assert_eq!(outside["$type"], "board/error/v1");
    assert_eq!(outside["code"], "flightdeck::check::bad_input");
    assert_eq!(degraded["code"], "flightdeck::check::degraded");
    assert_eq!(degraded["fields"]["board_code"], "board::schema::invalid");
    assert_eq!(missing["code"], "flightdeck::check::not_found");
    assert_eq!(garbled["code"], "flightdeck::check::bad_input");
}

#[test]
fn plugin_install_writes_the_manifest_and_schema_and_replaces_an_old_plugin() {
    let data = tempfile::tempdir().unwrap();
    let plugins = plugins_directory(Some(data.path()), None).unwrap();
    std::fs::create_dir_all(plugins.join("flightdeck")).unwrap();
    std::fs::write(plugins.join("flightdeck/plugin.json"), "old").unwrap();

    let installed = install(&plugins).unwrap();

    assert!(installed.replaced);
    assert_eq!(
        std::fs::read_to_string(plugins.join("flightdeck/plugin.json")).unwrap(),
        MANIFEST
    );
    assert_eq!(
        std::fs::read_to_string(
            plugins.join("flightdeck/types/flightdeck/workflow-graph/v1.schema.json")
        )
        .unwrap(),
        SCHEMA
    );
    let left: Vec<String> = std::fs::read_dir(&plugins)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["flightdeck"]);
}

#[test]
fn plugin_install_leaves_the_old_plugin_directory_whole_when_the_new_one_cannot_be_written() {
    let data = tempfile::tempdir().unwrap();
    let plugins = plugins_directory(Some(data.path()), None).unwrap();
    std::fs::create_dir_all(plugins.join("flightdeck")).unwrap();
    std::fs::write(plugins.join("flightdeck/plugin.json"), "old").unwrap();
    std::fs::set_permissions(&plugins, std::fs::Permissions::from_mode(0o555)).unwrap();

    let result = install(&plugins);
    std::fs::set_permissions(&plugins, std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(result.is_err());
    assert_eq!(
        std::fs::read_to_string(plugins.join("flightdeck/plugin.json")).unwrap(),
        "old"
    );
    assert_eq!(std::fs::read_dir(&plugins).unwrap().count(), 1);
}

#[test]
fn the_plugins_directory_is_under_xdg_data_home_or_else_home() {
    assert_eq!(
        plugins_directory(Some(Path::new("/data")), Some(Path::new("/home/a"))).unwrap(),
        Path::new("/data/board/plugins")
    );
    assert_eq!(
        plugins_directory(Some(Path::new("relative")), Some(Path::new("/home/a"))).unwrap(),
        Path::new("/home/a/.local/share/board/plugins")
    );
    assert!(plugins_directory(None, None).is_err());
}
