//! `flightdeck open <TOOL>`: load the tool's model, start the page's
//! server, write the opener page, print two lines, open the browser, and
//! serve until Ctrl-C, SIGTERM or the idle limit.
//!
//! The command prints no result on standard output. It writes its two lines
//! to standard error, where crumbkit writes messages about a run, and exits
//! with 0 when the server stops. A failure before the server starts prints
//! through crumbkit and exits with 1.

use std::fmt;
use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crumbkit::{Code, Failure, Output, Sink};
use signal_hook::consts::{SIGINT, SIGTERM};

use crate::cli::OpenArgs;
use crate::model::{self, LoadError};
use crate::opener::Opener;
use crate::server::{
    Clock as _, Ended, IDLE_LIMIT, Listener, Server, Session, Settings, StartError, SystemClock,
};
use crate::spawn::{self, Exit, Program};

/// The code of a page's server that could not start or stopped answering.
pub const SERVER_FAILED: Code = Code::new("flightdeck", "server", "failed");

/// The exit code of a second Ctrl-C or SIGTERM while flightdeck stops,
/// which ends the process at once.
const FORCED_EXIT: i32 = 130;

/// Why `flightdeck open` stopped before or while it served the page.
#[derive(Debug, Failure)]
pub enum OpenError {
    /// The tool's model could not be loaded.
    #[failure(forward)]
    Load(LoadError),
    /// flightdeck could not watch for Ctrl-C and SIGTERM.
    #[failure(code = SERVER_FAILED, hint = "run flightdeck open again")]
    Signals(io::Error),
    /// The server could not listen or build its replies.
    #[failure(
        code = SERVER_FAILED,
        hint = "check that flightdeck may listen on 127.0.0.1, then run flightdeck open again"
    )]
    Start(StartError),
    /// The operating system's random source failed.
    #[failure(code = SERVER_FAILED, hint = "run flightdeck open again")]
    Random(getrandom::Error),
    /// The opener page could not be written.
    #[failure(
        code = SERVER_FAILED,
        hint = "check that the temporary directory is writable, then run flightdeck open again"
    )]
    Opener(io::Error),
    /// The server stopped accepting connections.
    #[failure(code = SERVER_FAILED, hint = "run flightdeck open again")]
    Serve(io::Error),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load(error) => error.fmt(f),
            Self::Signals(_) => f.write_str("cannot watch for Ctrl-C"),
            Self::Start(_) => f.write_str("cannot start the page's server"),
            Self::Random(_) => f.write_str("cannot read the system's random source"),
            Self::Opener(_) => f.write_str("cannot write the opener page"),
            Self::Serve(_) => f.write_str("the page's server stopped accepting connections"),
        }
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Load(error) => error.source(),
            Self::Signals(error) | Self::Opener(error) | Self::Serve(error) => Some(error),
            Self::Start(error) => Some(error),
            Self::Random(error) => Some(error),
        }
    }
}

/// Runs `flightdeck open` with `args`, and prints a failure through
/// `output`.
pub(crate) fn run(output: &mut Output, args: &OpenArgs) -> ExitCode {
    #[expect(
        clippy::disallowed_methods,
        reason = "crumbkit's Output prints only documents and failures, so open writes its two lines to standard error here (CLI-1)"
    )]
    let mut lines = io::stderr();
    match open(args, &mut lines) {
        Ok(()) => crumbkit::events::ended(0, Some(0)),
        Err(failure) => {
            output.print_failure(&mut Sink::standard(), &failure);
            let exit = output.failure_exit(failure.code());
            crumbkit::events::ended(exit, Some(0))
        }
    }
}

/// Loads the tool, serves its page until it stops, and writes the lines
/// that a person reads to `lines`. A line that cannot be written, as when
/// standard error is closed, is skipped.
fn open(args: &OpenArgs, lines: &mut dyn Write) -> Result<(), OpenError> {
    let tool =
        model::load(&Program::new(&args.tool), spawn::TIME_LIMIT).map_err(OpenError::Load)?;
    let stop = Arc::new(AtomicBool::new(false));
    watch_signals(&stop).map_err(OpenError::Signals)?;
    let clock = SystemClock;
    let listener = Listener::bind().map_err(OpenError::Start)?;
    let origin = listener.origin();
    let session = Session::new(clock.now()).map_err(OpenError::Random)?;
    // A new session always holds its launch code.
    let code = session
        .launch_code()
        .ok_or_else(|| OpenError::Opener(io::Error::other("the session has no launch code")))?;
    let opener = Opener::write(&origin, code).map_err(OpenError::Opener)?;
    let server = Server::new(listener, session, &tool, args.read_only, Settings::new())
        .map_err(OpenError::Start)?;

    let commands = match tool.commands().len() {
        1 => "1 command".to_owned(),
        count => format!("{count} commands"),
    };
    let _ = writeln!(
        lines,
        "{} {} · {commands} · {origin}",
        tool.name(),
        tool.version()
    );
    let page = opener.page().display();
    if args.no_browser {
        let _ = writeln!(
            lines,
            "Open {page} in a browser. Press Ctrl-C to stop flightdeck."
        );
    } else {
        let _ = writeln!(
            lines,
            "Opening the browser. Press Ctrl-C to stop flightdeck."
        );
        if !matches!(spawn::open_browser(opener.page()), Ok(Exit::Code(0))) {
            let _ = writeln!(lines, "The browser did not open. Open {page} in a browser.");
        }
    }

    let ended = server.run(&clock, &stop).map_err(OpenError::Serve)?;
    if ended == Ended::Idle {
        let minutes = IDLE_LIMIT.as_secs() / 60;
        let _ = writeln!(
            lines,
            "flightdeck stopped after {minutes} minutes without a request from the page."
        );
    }
    // Dropping the opener removes its directory before flightdeck exits.
    drop(opener);
    Ok(())
}

/// Sets `stop` on Ctrl-C and SIGTERM. A second signal while flightdeck
/// stops, as when a request it is reading never ends, ends the process at
/// once with exit code 130 and leaves the opener directory behind.
fn watch_signals(stop: &Arc<AtomicBool>) -> io::Result<()> {
    for signal in [SIGINT, SIGTERM] {
        // signal-hook runs the handlers in the order they were registered,
        // so the shutdown sees the flag that only an earlier signal set.
        signal_hook::flag::register_conditional_shutdown(signal, FORCED_EXIT, Arc::clone(stop))?;
        signal_hook::flag::register(signal, Arc::clone(stop))?;
    }
    Ok(())
}
