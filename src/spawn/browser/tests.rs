use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;

use super::{Exit, open_with};

/// Writes a shell script named `opener` that runs `body`, and returns its
/// directory and path.
fn fake_opener(body: &str) -> (TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("opener");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (directory, path)
}

#[test]
fn the_opener_gets_the_page_path_as_its_one_argument() {
    let (directory, opener) = fake_opener(r#"printf '%s\n' "$#" "$@" > "$0.args""#);
    let page = Path::new("/tmp/a page with spaces/open.html");

    let exit = open_with(opener.as_os_str(), page, Duration::from_secs(10)).unwrap();

    assert_eq!(exit, Exit::Code(0));
    let args = std::fs::read_to_string(directory.path().join("opener.args")).unwrap();
    assert_eq!(args, "1\n/tmp/a page with spaces/open.html\n");
}

#[test]
fn an_opener_that_never_exits_is_stopped_at_the_time_limit() {
    let (_directory, opener) = fake_opener("exec sleep 30");

    let exit = open_with(
        opener.as_os_str(),
        Path::new("/tmp/open.html"),
        Duration::from_millis(100),
    )
    .unwrap();

    assert_eq!(exit, Exit::TimedOut);
}
