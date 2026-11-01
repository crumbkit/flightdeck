//! `flightdeck open`, run as the built binary. These tests start it with
//! `--no-browser` against shelf, which `scripts/install-test-tools.sh`
//! installs, read the two lines it prints, and stop it with a signal, as
//! Ctrl-C and a service manager do.

#![cfg(test)]
// The tests send Unix signals.
#![cfg(unix)]
// Only these tests start the binary, so the types and functions that the
// baseline keeps in src/spawn.rs appear here.
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the tests start the flightdeck binary and stop it with a signal"
)]

#[path = "support/http.rs"]
#[expect(dead_code, reason = "these tests use only part of the shared client")]
mod http;

use std::io::{BufRead as _, BufReader, Read as _};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, Command, Stdio};

use http::{Call, exchange, launch_code};

/// Returns the path of shelf, and fails the test when the install script
/// has not run.
fn shelf() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/test-tools/bin/shelf");
    assert!(
        path.is_file(),
        "{} is missing; run scripts/install-test-tools.sh to install shelf and reading",
        path.display()
    );
    path
}

/// Returns `flightdeck` with `args`, its streams piped, and shelf's XDG
/// directories in `dirs`.
fn flightdeck(args: &[&std::ffi::OsStr], dirs: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_flightdeck"));
    command
        .args(args)
        .env("XDG_CONFIG_HOME", dirs.join("config"))
        .env("XDG_DATA_HOME", dirs.join("data"))
        .env("XDG_STATE_HOME", dirs.join("state"))
        .env("XDG_CACHE_HOME", dirs.join("cache"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// A running `flightdeck open shelf --no-browser`, after its two lines.
struct Open {
    child: Child,
    stderr: BufReader<ChildStderr>,
    lines: [String; 2],
    port: u16,
    page: PathBuf,
    _dirs: tempfile::TempDir,
}

impl Open {
    /// Starts flightdeck and reads its two lines.
    fn start() -> Self {
        let dirs = tempfile::tempdir().unwrap();
        let shelf = shelf();
        let mut child = flightdeck(
            &["open".as_ref(), shelf.as_os_str(), "--no-browser".as_ref()],
            dirs.path(),
        )
        .spawn()
        .unwrap();
        let mut stderr = BufReader::new(child.stderr.take().unwrap());
        let mut read_line = || {
            let mut line = String::new();
            stderr.read_line(&mut line).unwrap();
            line.trim_end_matches('\n').to_owned()
        };
        let lines = [read_line(), read_line()];
        let port = lines[0]
            .rsplit_once("http://127.0.0.1:")
            .unwrap_or_else(|| panic!("no address in {lines:?}"))
            .1
            .parse()
            .unwrap();
        let page = lines[1]
            .strip_prefix("Open ")
            .and_then(|rest| rest.split_once(" in a browser."))
            .unwrap_or_else(|| panic!("no opener page in {lines:?}"))
            .0
            .into();
        Self {
            child,
            stderr,
            lines,
            port,
            page,
            _dirs: dirs,
        }
    }

    /// Sends `signal` to flightdeck, waits for it to exit, and returns its
    /// exit code and everything else it printed on either stream.
    fn stop(mut self, signal: rustix::process::Signal) -> (Option<i32>, String) {
        let pid = rustix::process::Pid::from_child(&self.child);
        rustix::process::kill_process(pid, signal).unwrap();
        let status = self.child.wait().unwrap();
        let mut printed = String::new();
        self.stderr.read_to_string(&mut printed).unwrap();
        self.child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut printed)
            .unwrap();
        (status.code(), printed)
    }
}

#[test]
fn open_on_a_missing_tool_prints_tool_not_found_and_starts_no_server() {
    let dirs = tempfile::tempdir().unwrap();
    let missing = dirs.path().join("no-such-tool");

    let output = flightdeck(
        &[
            "open".as_ref(),
            missing.as_os_str(),
            "--no-browser".as_ref(),
        ],
        dirs.path(),
    )
    .output()
    .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("flightdeck::tool::not_found"), "{stderr}");
    assert!(!stderr.contains("http://"), "{stderr}");
    assert!(output.stdout.is_empty());
}

#[test]
fn open_prints_two_lines_and_no_secret_and_stops_on_sigterm_with_exit_0() {
    let open = Open::start();
    let page_html = std::fs::read_to_string(&open.page).unwrap();
    let code = launch_code(&page_html);
    let port = open.port;

    assert!(
        open.lines[0].starts_with("shelf 0.1.0+git-31245d63b750 · 9 commands · http://127.0.0.1:"),
        "{:?}",
        open.lines
    );
    assert_eq!(
        open.lines[1],
        format!(
            "Open {} in a browser. Press Ctrl-C to stop flightdeck.",
            open.page.display()
        )
    );

    // Requests that succeed and requests that fail, so every reply and any
    // message they cause is in the output that the test searches.
    let token = exchange(port, &code);
    let mut replies = vec![
        Call::get(port, "/api/model").bearer(&token).send(port),
        Call::post(port, "/api/session", &format!(r#"{{"code":"{code}"}}"#)).send(port),
        Call::get(port, "/api/model").bearer(&token[1..]).send(port),
        Call::get(port, "/").header("Host", "localhost").send(port),
    ];
    replies.retain(|reply| reply.status != 200);
    let lines = open.lines.clone();
    let (exit, printed) = open.stop(rustix::process::Signal::TERM);

    assert_eq!(exit, Some(0), "{printed}");
    let mut everything = printed;
    for reply in &replies {
        everything.push_str(&String::from_utf8_lossy(&reply.body));
        for (name, value) in &reply.headers {
            everything.push_str(&format!("{name}: {value}\n"));
        }
    }
    for line in &lines {
        everything.push_str(line);
    }
    assert!(!everything.contains(&token), "the token reached the output");
    assert!(
        !everything.contains(&code),
        "the launch code reached the output"
    );
}

#[test]
fn ctrl_c_stops_open_with_exit_0_and_removes_the_opener_directory() {
    let open = Open::start();
    let directory = open.page.parent().unwrap().to_path_buf();
    assert!(directory.is_dir());

    let (exit, printed) = open.stop(rustix::process::Signal::INT);

    assert_eq!(exit, Some(0), "{printed}");
    assert!(!directory.exists(), "{} remains", directory.display());
}
