//! The Compliance site that `flightdeck compliance build --out` writes:
//! `compliance.html`, one file that holds the page's script, its styles
//! and the document, beside `compliance.json`.
//!
//! The browser tests in `ui/tests/acceptance/compliance.spec.ts` open the
//! file and draw its pages; these tests read its bytes.

#![cfg(test)]
#![expect(
    clippy::disallowed_methods,
    reason = "the test starts the flightdeck binary"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crumbkit_flightdeck::compliance::{self, BuildRequest, ComplianceError, site};
use serde_json::Value;

/// Returns the path of `relative` in the repository.
fn here(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// The time of the fixture reports, plus an hour.
fn built_at() -> jiff::Timestamp {
    let text =
        std::fs::read_to_string(here("tests/fixtures/compliance/reports/owner/app.json")).unwrap();
    let report: Value = serde_json::from_str(&text).unwrap();
    let generated: jiff::Timestamp = report["generated"].as_str().unwrap().parse().unwrap();
    generated
        .checked_add(jiff::SignedDuration::from_hours(1))
        .unwrap()
}

/// The document of the fixtures.
fn document() -> compliance::ComplianceDocument {
    compliance::build(&BuildRequest {
        config: here("tests/fixtures/compliance/compliance.toml"),
        reports: here("tests/fixtures/compliance/reports"),
        built_at: built_at(),
        build_run: Some("Build run #412".to_owned()),
        build_url: None,
        rules_commit: None,
        rules_branch: None,
    })
    .unwrap()
}

/// Runs `flightdeck compliance build --out <out>` over the fixtures with
/// `extra`.
fn build_into(out: &Path, extra: &[&str]) -> Output {
    build_with(
        &here("tests/fixtures/compliance/compliance.toml"),
        out,
        extra,
    )
}

/// Runs `flightdeck compliance build` with `config` and the fixture
/// reports, writing into `out`, with `extra` arguments after the rest.
fn build_with(config: &Path, out: &Path, extra: &[&str]) -> Output {
    let state = tempfile::tempdir().unwrap();
    let at = format!("{:.0}", built_at());
    let reports = here("tests/fixtures/compliance/reports");
    Command::new(env!("CARGO_BIN_EXE_flightdeck"))
        .args([
            "compliance",
            "build",
            "--config",
            config.to_str().unwrap(),
            "--reports",
            reports.to_str().unwrap(),
            "--built-at",
            &at,
            "--out",
            out.to_str().unwrap(),
        ])
        .args(extra)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("XDG_STATE_HOME", state.path())
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

/// Returns the text of the document element in `page`, with each `<`
/// read back as `<`.
fn inlined_document(page: &str) -> Value {
    let open = format!(
        "<script type=\"application/json\" id=\"{}\">",
        site::DOCUMENT_ID
    );
    let start = page.find(&open).expect("the page holds the document") + open.len();
    let end = start + page[start..].find("</script>").unwrap();
    serde_json::from_str(&page[start..end]).unwrap()
}

/// Returns `page` with the text of every script and style element left out,
/// so what remains is the page's markup.
fn markup(page: &str) -> String {
    let mut rest = page;
    let mut kept = String::new();
    while let Some(at) = rest.find(['<']) {
        kept.push_str(&rest[..at]);
        rest = &rest[at..];
        let element = ["<script", "<style"]
            .into_iter()
            .find(|open| rest.starts_with(open));
        match element {
            Some(open) => {
                let close = format!("</{}", &open[1..]);
                let tag_end = rest.find('>').unwrap() + 1;
                kept.push_str(&rest[..tag_end]);
                let body_end = rest.find(&close).unwrap();
                rest = &rest[body_end..];
            }
            None => {
                kept.push('<');
                rest = &rest[1..];
            }
        }
    }
    kept.push_str(rest);
    kept
}

/// Returns the text of every style element in `page`.
fn styles(page: &str) -> String {
    page.split("<style")
        .skip(1)
        .map(|part| {
            let body = &part[part.find('>').unwrap() + 1..];
            &body[..body.find("</style").unwrap()]
        })
        .collect()
}

#[test]
fn out_writes_compliance_html_holding_the_document_that_compliance_json_holds() {
    let out = tempfile::tempdir().unwrap();
    let output = build_into(out.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let page = std::fs::read_to_string(out.path().join("compliance.html")).unwrap();
    let json: Value =
        serde_json::from_slice(&std::fs::read(out.path().join("compliance.json")).unwrap())
            .unwrap();
    assert_eq!(inlined_document(&page), json);
    assert!(!page.contains(site::MARKER), "the marker is replaced");
}

#[test]
fn the_build_names_both_files_and_the_pages_size() {
    let out = tempfile::tempdir().unwrap();
    let output = build_into(out.path(), &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let size = std::fs::metadata(out.path().join("compliance.html"))
        .unwrap()
        .len();
    assert!(
        stderr.contains(&format!(
            "Wrote {} and {}, {size} bytes",
            out.path().join("compliance.json").display(),
            out.path().join("compliance.html").display()
        )),
        "{stderr}"
    );
    assert!(!stderr.contains(site::OVER_FLARE_CAP.as_str()), "{stderr}");
}

#[test]
fn the_json_output_names_both_files_the_pages_size_and_no_warning() {
    let out = tempfile::tempdir().unwrap();
    let output = build_into(out.path(), &["--json"]);
    assert!(output.status.success(), "{output:?}");
    let printed: Value = serde_json::from_slice(&output.stdout).unwrap();
    let size = std::fs::metadata(out.path().join("compliance.html"))
        .unwrap()
        .len();
    assert_eq!(
        printed["data"]["site"],
        serde_json::json!({
            "document": out.path().join("compliance.json").to_str().unwrap(),
            "page": out.path().join("compliance.html").to_str().unwrap(),
            "page_bytes": size,
            "warnings": [],
        })
    );
    let written: Value =
        serde_json::from_slice(&std::fs::read(out.path().join("compliance.json")).unwrap())
            .unwrap();
    assert!(written["data"].get("site").is_none(), "{written}");
}

#[test]
fn the_page_loads_nothing_outside_itself() {
    let out = tempfile::tempdir().unwrap();
    let output = build_into(out.path(), &[]);
    assert_eq!(output.status.code(), Some(0));
    let page = std::fs::read_to_string(out.path().join("compliance.html")).unwrap();
    let markup = markup(&page);
    for attribute in ["src=", "href=", "srcset="] {
        assert!(
            !markup.contains(attribute),
            "the markup names a file with {attribute}: {markup}"
        );
    }
    assert!(!markup.contains("<link"), "{markup}");
    let css = styles(&page);
    assert!(!css.is_empty(), "the styles are inlined");
    for (at, _) in css.match_indices("url(") {
        assert!(
            css[at + 4..]
                .trim_start_matches(['"', '\''])
                .starts_with("data:"),
            "the styles name a file: {}",
            &css[at..css.len().min(at + 80)]
        );
    }
    assert!(!css.contains("@import"), "{css}");
    assert!(!css.contains("@font-face"), "the page uses system fonts");
}

#[test]
fn a_note_that_holds_a_closing_script_tag_stays_inside_the_document() {
    let template = format!("<!doctype html><body>{}</body>", site::MARKER);
    let json = br#"{"note":"</script><b>bold</b>"}"#;
    let page = String::from_utf8(site::page(template.as_bytes(), json).unwrap()).unwrap();
    assert_eq!(page.matches("</script>").count(), 1, "{page}");
    assert_eq!(
        inlined_document(&page),
        serde_json::json!({ "note": "</script><b>bold</b>" })
    );
}

#[test]
fn a_page_without_the_marker_fails_with_no_page_after_writing_compliance_json() {
    let out = tempfile::tempdir().unwrap();
    let result = compliance::write_out_with(out.path(), &document(), b"<!doctype html>");
    assert!(matches!(result, Err(ComplianceError::NoPage)), "{result:?}");
    assert!(out.path().join("compliance.json").exists());
    assert!(!out.path().join("compliance.html").exists());
}

#[test]
fn a_page_with_the_marker_is_written_beside_compliance_json() {
    let out = tempfile::tempdir().unwrap();
    let template = format!("<!doctype html>{}", site::MARKER);
    let written = compliance::write_out_with(out.path(), &document(), template.as_bytes()).unwrap();
    assert_eq!(written.page, out.path().join("compliance.html"));
    assert!(!written.over_flare_cap());
    assert!(!written.note().contains(site::OVER_FLARE_CAP.as_str()));
}

#[test]
fn a_page_over_flares_cap_is_written_with_the_over_flare_cap_warning() {
    let out = tempfile::tempdir().unwrap();
    let padding = " ".repeat(usize::try_from(site::FLARE_CAP).unwrap());
    let template = format!("<!doctype html>{}{padding}", site::MARKER);
    let written = compliance::write_out_with(out.path(), &document(), template.as_bytes()).unwrap();
    assert!(written.over_flare_cap());
    assert!(
        written
            .note()
            .contains("Warning flightdeck::compliance::over_flare_cap"),
        "{}",
        written.note()
    );
    assert_eq!(
        std::fs::metadata(&written.page).unwrap().len(),
        written.page_bytes
    );
    // The printed document's `site` carries the same warning, so `--json`
    // reports it too.
    let mut printed = document();
    printed.site = Some(written.files());
    let json: Value = serde_json::from_slice(&printed.to_json()).unwrap();
    assert_eq!(
        json["data"]["site"]["warnings"],
        serde_json::json!([site::OVER_FLARE_CAP.as_str()])
    );
    assert_eq!(json["data"]["site"]["page_bytes"], written.page_bytes);
}

#[test]
fn the_embedded_page_has_a_place_for_the_document() {
    assert!(site::page(site::BUNDLE, b"{}").is_some());
}
