//! A page's server for the server and route tests, run in-process on a port
//! the system picks. It loads one tool's model, gives the server a runner of
//! the tool, writes the opener page, and reads the launch code back from it.
//! The server's clock is injected, so no test waits for the idle limit.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use crumbkit_flightdeck::model;
use crumbkit_flightdeck::opener::Opener;
use crumbkit_flightdeck::runner::{self, Mode, Runner};
use crumbkit_flightdeck::server::{Clock, Ended, Listener, Moment, Server, Session, Settings};
use crumbkit_flightdeck::spawn::{Program, TIME_LIMIT};
use tempfile::TempDir;

use crate::http::{Call, Reply, exchange, launch_code};

/// A clock that moves only when a test moves it. Its monotonic time and
/// its wall-clock time move apart when the test moves only the wall clock,
/// as on a machine that sleeps.
pub(crate) struct FakeClock {
    start: Instant,
    start_wall: SystemTime,
    millis: AtomicU64,
    wall_millis: AtomicU64,
}

impl FakeClock {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            start_wall: SystemTime::now(),
            millis: AtomicU64::new(0),
            wall_millis: AtomicU64::new(0),
        }
    }

    /// Moves both clocks forward by `by`.
    pub(crate) fn advance(&self, by: Duration) {
        let millis = u64::try_from(by.as_millis()).unwrap();
        self.millis.fetch_add(millis, Ordering::SeqCst);
        self.wall_millis.fetch_add(millis, Ordering::SeqCst);
    }

    /// Moves only the wall clock forward by `by`, as a machine's sleep
    /// does on a system whose monotonic clock stops while it sleeps.
    pub(crate) fn sleep(&self, by: Duration) {
        let millis = u64::try_from(by.as_millis()).unwrap();
        self.wall_millis.fetch_add(millis, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Moment {
        let millis = |clock: &AtomicU64| Duration::from_millis(clock.load(Ordering::SeqCst));
        Moment::new(
            self.start + millis(&self.millis),
            self.start_wall + millis(&self.wall_millis),
        )
    }
}

/// Returns the path of one of crumbkit's example tools that
/// `scripts/install-test-tools.sh` builds, and fails the test when the
/// script has not run.
pub(crate) fn test_tool(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/test-tools/bin")
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    assert!(
        path.is_file(),
        "{} is missing; run scripts/install-test-tools.sh to install shelf and reading",
        path.display()
    );
    path
}

/// Returns the path of the fake tool `unruly` of the runner's tests.
#[cfg(unix)]
pub(crate) fn unruly() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/runner/unruly")
}

/// What a test serves: the tool, the program that runs it, the runner's
/// mode and settings, and the server's settings.
pub(crate) struct Setup {
    tool: PathBuf,
    program: Option<PathBuf>,
    env: Vec<(String, OsString)>,
    mode: Mode,
    runner: runner::Settings,
    server: Settings,
}

impl Setup {
    /// Serves the tool at `tool` with every command, the runner's default
    /// settings with short graces after each signal, and a server that
    /// polls every 5 milliseconds.
    pub(crate) fn of(tool: PathBuf) -> Self {
        Self {
            tool,
            program: None,
            env: Vec::new(),
            mode: Mode::Full,
            runner: runner::Settings::new().escalation(runner::Escalation::new(
                Duration::from_millis(200),
                Duration::from_millis(200),
            )),
            server: Settings::new().poll(Duration::from_millis(5)),
        }
    }

    /// Serves shelf.
    pub(crate) fn shelf() -> Self {
        Self::of(test_tool("shelf"))
    }

    /// Serves only the tool's read-only commands, as `--read-only` does.
    pub(crate) fn read_only(mut self) -> Self {
        self.mode = Mode::ReadOnly;
        self
    }

    /// Runs `program` in place of the tool whose model the server serves.
    pub(crate) fn program(mut self, program: PathBuf) -> Self {
        self.program = Some(program);
        self
    }

    /// Gives the tool the variable `name` with `value`.
    pub(crate) fn env(mut self, name: &str, value: impl Into<OsString>) -> Self {
        self.env.push((name.to_owned(), value.into()));
        self
    }

    /// Gives the runner `settings`.
    pub(crate) fn runner(mut self, settings: runner::Settings) -> Self {
        self.runner = settings;
        self
    }

    /// Gives the server `settings`.
    pub(crate) fn server(mut self, settings: Settings) -> Self {
        self.server = settings;
        self
    }

    /// Returns `path` as a program with XDG directories under `dirs` and the
    /// variables of the setup.
    fn program_at(&self, path: &Path, dirs: &TempDir) -> Program {
        let mut program = Program::new(path);
        for (name, directory) in [
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_STATE_HOME", "state"),
            ("XDG_CACHE_HOME", "cache"),
        ] {
            program = program.env(name, dirs.path().join(directory));
        }
        for (name, value) in &self.env {
            program = program.env(name, value);
        }
        program
    }
}

/// A server that runs on a thread of the test, and what the test knows
/// about it.
pub(crate) struct Running<'a> {
    pub(crate) port: u16,
    pub(crate) code: String,
    pub(crate) page: PathBuf,
    pub(crate) clock: &'a FakeClock,
    stop: &'a AtomicBool,
}

impl Running<'_> {
    /// Stops the server, as Ctrl-C does, while the test goes on.
    pub(crate) fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    /// Exchanges the launch code and returns the token.
    pub(crate) fn token(&self) -> String {
        exchange(self.port, &self.code)
    }

    /// Sends `GET /api/model` with `token`.
    pub(crate) fn model(&self, token: &str) -> Reply {
        Call::get(self.port, "/api/model")
            .bearer(token)
            .send(self.port)
    }
}

/// What the harness does once the test's requests are done.
pub(crate) enum Then {
    /// Stop the server, as Ctrl-C does.
    Stop,
    /// Wait for the server to stop by itself.
    Wait,
}

/// Starts a server for `setup`, runs `test` against it, and returns how
/// the server ended. The tool's children start with SIGINT at its default,
/// as under `flightdeck open`.
pub(crate) fn serving(setup: Setup, test: impl FnOnce(&Running<'_>) -> Then) -> Ended {
    crate::signals::interrupt_reaches_children();
    let dirs = tempfile::tempdir().unwrap();
    let loaded = setup.program_at(&setup.tool, &dirs);
    let tool = model::load(&loaded, TIME_LIMIT).unwrap();
    let program = match &setup.program {
        Some(path) => setup.program_at(path, &dirs),
        None => loaded,
    };
    let runner = Runner::new(
        tool,
        program,
        setup.mode,
        Arc::new(runner::SystemClock),
        setup.runner,
    )
    .unwrap();
    let clock = FakeClock::new();
    let listener = Listener::bind().unwrap();
    let port = listener.port();
    let session = Session::new(clock.now()).unwrap();
    let opener = Opener::write(&listener.origin(), session.launch_code().unwrap()).unwrap();
    let code = launch_code(&std::fs::read_to_string(opener.page()).unwrap());
    let server = Server::new(listener, session, runner, setup.server, None).unwrap();
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let handle = scope.spawn(|| server.run(&clock, &stop));
        let running = Running {
            port,
            code,
            page: opener.page().to_path_buf(),
            clock: &clock,
            stop: &stop,
        };
        // A failed assertion in `test` unwinds through the guard, which
        // stops the server, so the scope can join its thread.
        let guard = StopOnDrop(&stop);
        match test(&running) {
            Then::Stop => drop(guard),
            Then::Wait => {
                std::mem::forget(guard);
                // A server that does not stop by itself fails the test
                // instead of hanging it. The stop flag lets the scope join
                // the server's thread after the panic.
                if !finished_within(&handle, STOP_LIMIT) {
                    stop.store(true, Ordering::SeqCst);
                    panic!("the server did not stop by itself within {STOP_LIMIT:?}");
                }
            }
        }
        handle.join().unwrap().unwrap()
    })
}

/// How long a test that waits for the server to stop by itself waits.
const STOP_LIMIT: Duration = Duration::from_secs(5);

/// Waits up to `limit` for the thread of `handle` to finish, and returns
/// whether it has.
fn finished_within<T>(handle: &std::thread::ScopedJoinHandle<'_, T>, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            return false;
        }
        #[expect(
            clippy::disallowed_methods,
            reason = "a scoped thread offers no join with a timeout"
        )]
        std::thread::sleep(Duration::from_millis(5));
    }
    true
}

/// Sets the stop flag when it is dropped.
struct StopOnDrop<'a>(&'a AtomicBool);

impl Drop for StopOnDrop<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
