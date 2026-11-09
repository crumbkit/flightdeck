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
use serde_json::json;

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
    run_flightdeck(Command::new(env!("CARGO_BIN_EXE_flightdeck")), args, dirs)
}

/// Returns `command`, which runs flightdeck, with `args`, its streams
/// piped, and shelf's XDG directories in `dirs`.
fn run_flightdeck(mut command: Command, args: &[&std::ffi::OsStr], dirs: &Path) -> Command {
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
    /// Starts flightdeck on shelf and reads its two lines.
    fn start() -> Self {
        Self::start_on(&shelf())
    }

    /// Starts flightdeck on the tool at `tool` and reads its two lines.
    fn start_on(tool: &Path) -> Self {
        let dirs = tempfile::tempdir().unwrap();
        let command = flightdeck(
            &["open".as_ref(), tool.as_os_str(), "--no-browser".as_ref()],
            dirs.path(),
        );
        Self::start_with(command, dirs)
    }

    /// Starts flightdeck on the tool at `tool` with SIGINT ignored, as a
    /// background job of a non-interactive shell starts, and reads its two
    /// lines. perl ignores the signal and then replaces itself with
    /// flightdeck, which inherits the disposition.
    fn start_ignoring_interrupt_on(tool: &Path) -> Self {
        let dirs = tempfile::tempdir().unwrap();
        let mut perl = Command::new("perl");
        perl.args([
            "-e",
            r#"$SIG{INT} = "IGNORE"; exec @ARGV or die "exec: $!""#,
        ])
        .arg(env!("CARGO_BIN_EXE_flightdeck"));
        let command = run_flightdeck(
            perl,
            &["open".as_ref(), tool.as_os_str(), "--no-browser".as_ref()],
            dirs.path(),
        );
        Self::start_with(command, dirs)
    }

    /// Starts `command`, a flightdeck that serves until it is stopped, and
    /// reads its two lines.
    fn start_with(mut command: Command, dirs: tempfile::TempDir) -> Self {
        let mut child = command.spawn().unwrap();
        let mut stderr = BufReader::new(child.stderr.take().unwrap());
        let mut read_line = || {
            let mut line = String::new();
            stderr.read_line(&mut line).unwrap();
            line.trim_end_matches('\n').to_owned()
        };
        let lines = [read_line(), read_line()];
        let port = lines[0]
            .strip_prefix("http://127.0.0.1:")
            .and_then(|rest| rest.split_once(' '))
            .unwrap_or_else(|| panic!("no address first in {lines:?}"))
            .0
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

/// A test that fails while flightdeck runs stops it here, so no flightdeck
/// outlives the test: SIGTERM first, then SIGKILL when it has not exited
/// within two seconds. A flightdeck that `stop` already reaped is left
/// alone, because its process id may belong to another process by now.
impl Drop for Open {
    fn drop(&mut self) {
        use wait_timeout::ChildExt as _;

        if !matches!(self.child.try_wait(), Ok(None)) {
            return;
        }
        let pid = rustix::process::Pid::from_child(&self.child);
        // The process may have exited since `try_wait`, and it is unreaped,
        // so its id still names it.
        rustix::process::kill_process(pid, rustix::process::Signal::TERM).ok();
        if matches!(
            self.child.wait_timeout(std::time::Duration::from_secs(2)),
            Ok(Some(_))
        ) {
            return;
        }
        self.child.kill().ok();
        self.child.wait().ok();
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

    assert_eq!(
        open.lines[0],
        format!("http://127.0.0.1:{port} · shelf 0.2.0+git-45d0015e1f94 · 9 commands"),
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

/// Writes a fake crumbkit tool with one command, whose `--version` prints
/// `version_line`, and returns its directory and path.
fn fake_tool(version_line: &str) -> (tempfile::TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt as _;

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fake");
    let script = format!(
        r#"#!/bin/sh
case "$1 $2" in
"--version ") printf '%s\n' '{version_line}' ;;
"commands list") echo '{{"schema":"fake/commands-list/v1","truncated":false,"data":[{{"path":"fake","about":"A fake tool"}},{{"path":"fake things","about":"Things"}},{{"path":"fake things list","about":"Lists things"}}]}}' ;;
"schemas list") echo '{{"schema":"fake/schemas-list/v1","truncated":false,"data":[]}}' ;;
*) exit 2 ;;
esac
"#
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (directory, path)
}

#[test]
fn open_prints_the_control_characters_of_a_tools_version_escaped() {
    // An OSC 52 sequence would write to the clipboard, and the carriage
    // return and right-to-left override would hide the address.
    let (_directory, tool) = fake_tool("fake 1.0\x1b]52;c;aGk=\x07\r\u{202e}");
    let tool_line = std::process::Command::new(&tool)
        .arg("--version")
        .output()
        .unwrap();
    assert!(
        tool_line.stdout.contains(&0x1b),
        "the fake tool prints no ESC"
    );

    let open = Open::start_on(&tool);
    let line = open.lines[0].clone();
    let (exit, printed) = open.stop(rustix::process::Signal::TERM);

    assert_eq!(exit, Some(0), "{printed}");
    assert!(
        line.ends_with(" · fake 1.0\\u{1b}]52;c;aGk=\\u{7}\\u{d}\\u{202e} · 1 command"),
        "{line:?}"
    );
    assert!(!line.chars().any(char::is_control), "{line:?}");
}

#[test]
fn sigterm_during_a_streamed_run_lets_the_run_finish_and_its_stream_end_before_flightdeck_exits() {
    let unruly = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runner/unruly");
    let open = Open::start_on(&unruly);
    let directory = open.page.parent().unwrap().to_path_buf();
    let port = open.port;
    let token = exchange(
        port,
        &launch_code(&std::fs::read_to_string(&open.page).unwrap()),
    );
    let linger = json!({"path": ["child", "linger"], "values": {"<SECONDS>": "0.5"}});
    let mut lines = Call::post(port, "/api/runs", &linger.to_string())
        .bearer(&token)
        .stream(port);
    assert_eq!(lines.next_event().unwrap()["event"], "started");
    let first = lines.next_event().unwrap();
    let document: serde_json::Value =
        serde_json::from_str(first["text"].as_str().unwrap()).unwrap();
    let pid = rustix::process::Pid::from_raw(
        i32::try_from(document["data"]["pid"].as_i64().unwrap()).unwrap(),
    )
    .unwrap();

    // The child lingers for half a second, within the runner's shutdown
    // grace, so it finishes by itself.
    let (exit, printed) = open.stop(rustix::process::Signal::TERM);
    let rest = lines.rest();

    assert_eq!(exit, Some(0), "{printed}");
    let ended = rest.last().unwrap();
    assert_eq!(ended["event"], "ended", "{rest:?}");
    assert_eq!(ended["outcome"]["kind"], "result");
    assert!(
        rustix::process::test_kill_process(pid).is_err(),
        "the child outlived flightdeck"
    );
    assert!(!directory.exists(), "{} remains", directory.display());
}

#[test]
fn a_cancel_ends_a_run_with_exit_130_when_flightdeck_starts_with_interrupt_ignored() {
    // A child inherits flightdeck's signal dispositions. flightdeck handles
    // SIGINT itself, and a handled signal resets to its default on exec, so
    // the child can still handle the cancel's SIGINT.
    let unruly = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runner/unruly");
    let open = Open::start_ignoring_interrupt_on(&unruly);
    let port = open.port;
    let token = exchange(
        port,
        &launch_code(&std::fs::read_to_string(&open.page).unwrap()),
    );
    let wait = json!({"path": ["child", "wait"]});
    let mut lines = Call::post(port, "/api/runs", &wait.to_string())
        .bearer(&token)
        .stream(port);
    let started = lines.next_event().unwrap();
    assert_eq!(lines.next_event().unwrap()["event"], "line");
    let run = started["run"].as_str().unwrap();

    let cancelled = Call::post(port, &format!("/api/runs/{run}/cancel"), "{}")
        .bearer(&token)
        .send(port);
    let rest = lines.rest();

    assert_eq!(cancelled.status, 204);
    let ended = rest.last().unwrap();
    assert_eq!(ended["outcome"]["kind"], "cancelled", "{ended}");
    assert_eq!(ended["exit"], 130, "{ended}");
}

#[test]
fn open_serves_the_directory_it_started_in_and_a_relative_tool_path_still_runs_from_another() {
    let launch = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::create_dir(launch.path().join("tools")).unwrap();
    std::os::unix::fs::symlink(shelf(), launch.path().join("tools/shelf")).unwrap();
    let dirs = tempfile::tempdir().unwrap();
    let mut command = flightdeck(
        &[
            "open".as_ref(),
            "./tools/shelf".as_ref(),
            "--no-browser".as_ref(),
        ],
        dirs.path(),
    );
    command.current_dir(launch.path());
    let open = Open::start_with(command, dirs);
    let token = exchange(
        open.port,
        &launch_code(&std::fs::read_to_string(&open.page).unwrap()),
    );

    let model = Call::get(open.port, "/api/model")
        .bearer(&token)
        .send(open.port)
        .json();
    let list = json!({"path": ["books", "list"], "directory": elsewhere.path()});
    let lines = Call::post(open.port, "/api/runs", &list.to_string())
        .bearer(&token)
        .stream(open.port)
        .rest();

    assert_eq!(
        model["directory"],
        json!(launch.path().canonicalize().unwrap())
    );
    assert_eq!(
        lines.last().unwrap()["outcome"]["kind"],
        "result",
        "{lines:?}"
    );
}

/// Writes the shell script `body` at `path`, and makes it runnable.
fn write_script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt as _;

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Returns how many lines the log at `path` holds, or 0 before it exists.
fn starts(path: &Path) -> usize {
    std::fs::read_to_string(path).map_or(0, |text| text.lines().count())
}

#[test]
fn a_bare_tool_name_starts_only_the_tool_found_at_launch_and_never_one_planted_in_the_run_directory()
 {
    let launch = tempfile::tempdir().unwrap();
    let chosen = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let real = logs.path().join("real.log");
    let planted = logs.path().join("planted.log");
    write_script(
        &launch.path().join("bin/shelf"),
        &format!(
            "echo started >> '{}'\nexec '{}' \"$@\"",
            real.display(),
            shelf().display()
        ),
    );
    write_script(
        &chosen.path().join("bin/shelf"),
        &format!("echo started >> '{}'", planted.display()),
    );
    let dirs = tempfile::tempdir().unwrap();
    let mut command = flightdeck(
        &["open".as_ref(), "shelf".as_ref(), "--no-browser".as_ref()],
        dirs.path(),
    );
    command
        .current_dir(launch.path())
        .env("PATH", "bin:/usr/bin:/bin");
    let open = Open::start_with(command, dirs);
    let token = exchange(
        open.port,
        &launch_code(&std::fs::read_to_string(&open.page).unwrap()),
    );
    let before = starts(&real);

    let list = json!({"path": ["books", "list"], "directory": chosen.path()});
    let lines = Call::post(open.port, "/api/runs", &list.to_string())
        .bearer(&token)
        .stream(open.port)
        .rest();

    assert_eq!(
        lines.last().unwrap()["outcome"]["kind"],
        "result",
        "{lines:?}"
    );
    assert_eq!(starts(&planted), 0, "the planted program started");
    assert_eq!(starts(&real), before + 1, "the tool did not start once");
}
