//! `flightdeck open <TOOL>`: load the tool's model, start the page's
//! server with the one runner of the process, write the opener page, print
//! two lines, open the browser, and serve until Ctrl-C, SIGTERM or the idle
//! limit.
//!
//! The command prints no result on standard output. It writes its two lines
//! to standard error, where crumbkit writes messages about a run, and exits
//! with 0 when the server stops. A failure before the server starts prints
//! through crumbkit and exits with 1.

use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crumbkit::{Code, Failure, Recovery, Run, Sink};
use crumbkit_process::Status;
use signal_hook::consts::{SIGINT, SIGTERM};

use crate::cli::OpenArgs;
use crate::clock::{Clock as _, SystemClock};
use crate::model::{self, LoadError};
use crate::opener::Opener;
use crate::runner::{self, Mode, Runner};
use crate::server::{Ended, IDLE_LIMIT, Listener, Server, Session, Settings, StartError};
use crate::spawn::{self, Exit, Program};

#[cfg(test)]
mod tests;

/// The code of a page's server that could not start or stopped answering.
pub const SERVER_FAILED: Code = Code::new("flightdeck", "server", "failed");

/// The code of a bare tool name that flightdeck cannot look up the same way
/// from every directory, because an entry of `PATH` cannot be made absolute.
pub const TOOL_SEARCH_PATH: Code = Code::new("flightdeck", "tool", "search_path");

/// The exit codes of a second Ctrl-C or SIGTERM while `flightdeck open`
/// stops, which ends the process at once. The tool record declares them,
/// and [`forced_exit`] gives the same numbers.
pub(crate) const FORCED_EXITS: [crumbkit::tool::Exit; 2] = [
    crumbkit::tool::Exit::new(
        130,
        "A second Ctrl-C while flightdeck stopped ended it at once and left the opener page behind.",
    )
    .commands(&[&["open"]]),
    crumbkit::tool::Exit::new(
        143,
        "A second SIGTERM while flightdeck stopped ended it at once and left the opener page behind.",
    )
    .commands(&[&["open"]]),
];

/// Why `flightdeck open` stopped before or while it served the page.
#[derive(Debug, Failure)]
pub enum OpenError {
    /// A relative entry of `PATH` cannot be made absolute, so a bare tool
    /// name could start a different program in a run's directory.
    #[failure(
        code = TOOL_SEARCH_PATH,
        hint = "give the tool's path",
        alternative = Recovery::hint("run flightdeck open from another directory"),
    )]
    SearchPath,
    /// The tool's model could not be loaded.
    #[failure(forward)]
    Load(LoadError),
    /// flightdeck could not watch for Ctrl-C and SIGTERM.
    #[failure(code = SERVER_FAILED, recovery = Recovery::Retry)]
    Signals(io::Error),
    /// The server could not listen or build its replies.
    #[failure(
        code = SERVER_FAILED,
        hint = "check that flightdeck may listen on 127.0.0.1",
        alternative = Recovery::Retry,
    )]
    Start(StartError),
    /// The operating system's random source failed.
    #[failure(code = SERVER_FAILED, recovery = Recovery::Retry)]
    Random(getrandom::Error),
    /// The opener page could not be written.
    #[failure(
        code = SERVER_FAILED,
        hint = "check that the temporary directory is writable",
        alternative = Recovery::Retry,
    )]
    Opener(io::Error),
    /// The server stopped accepting connections.
    #[failure(code = SERVER_FAILED, recovery = Recovery::Retry)]
    Serve(io::Error),
}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SearchPath => f.write_str("a relative PATH entry cannot be made absolute"),
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
            Self::SearchPath => None,
            Self::Load(error) => error.source(),
            Self::Signals(error) | Self::Opener(error) | Self::Serve(error) => Some(error),
            Self::Start(error) => Some(error),
            Self::Random(error) => Some(error),
        }
    }
}

/// Runs `flightdeck open` with `args`, and prints a failure through
/// `run`.
pub(crate) fn run(run: &mut Run, args: &OpenArgs) -> ExitCode {
    #[expect(
        clippy::disallowed_methods,
        reason = "crumbkit's Run prints only documents and failures, so open writes its two lines to standard error here (CLI-1)"
    )]
    let mut lines = io::stderr();
    match open(args, &mut lines) {
        Ok(()) => crumbkit::events::ended(0, Some(0)),
        Err(failure) => run.fail(&mut Sink::standard(), &failure),
    }
}

/// Loads the tool, serves its page until it stops, and writes the lines
/// that a person reads to `lines`. A line that cannot be written, as when
/// standard error is closed, is skipped.
fn open(args: &OpenArgs, lines: &mut dyn Write) -> Result<(), OpenError> {
    let launch = launch_directory();
    #[expect(
        clippy::disallowed_methods,
        reason = "a run in another directory must find the tool that this PATH found at launch"
    )]
    let search = std::env::var_os("PATH");
    let program = Program::at_launch(&args.tool, launch.as_deref(), search.as_deref())
        .ok_or(OpenError::SearchPath)?;
    let tool = model::load(&program, spawn::TIME_LIMIT).map_err(OpenError::Load)?;
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
    let commands = match tool.commands().len() {
        1 => "1 command".to_owned(),
        count => format!("{count} commands"),
    };
    let name = format!("{} {}", printable(tool.name()), printable(tool.version()));
    let mode = if args.read_only {
        Mode::ReadOnly
    } else {
        Mode::Full
    };
    let runner = Runner::new(
        tool,
        program,
        mode,
        Arc::new(SystemClock),
        runner::Settings::new(),
    )
    .map_err(OpenError::Random)?;
    let server = Server::new(
        listener,
        session,
        runner,
        Settings::new(),
        launch.as_deref().and_then(shown_directory),
    )
    .map_err(OpenError::Start)?;

    // The origin comes first, so a tool's name cannot push it out of view.
    let _ = writeln!(lines, "{origin} · {name} · {commands}");
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

/// Returns the directory flightdeck started in, or `None` when it cannot
/// read it.
fn launch_directory() -> Option<PathBuf> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the page shows the directory that flightdeck started in, and a relative tool path resolves against it"
    )]
    std::env::current_dir().ok()
}

/// Returns `directory` for the page to show, or `None` when the page could
/// not send it back, because its path is not UTF-8 or holds a control
/// character, which the runner refuses.
fn shown_directory(directory: &Path) -> Option<&str> {
    directory.to_str().filter(|text| !model::has_control(text))
}

/// Returns `text` with each control character and each bidirectional
/// formatting character written as `\u{..}`, so text that a tool prints
/// about itself cannot move the cursor, rewrite the line, set the clipboard
/// or reorder the address when flightdeck prints it.
fn printable(text: &str) -> String {
    text.chars()
        .map(|character| {
            if character.is_control() || is_bidi_control(character) {
                character.escape_unicode().to_string()
            } else {
                character.to_string()
            }
        })
        .collect()
}

/// Returns true for the characters that change the direction of text:
/// the marks, the embeddings and overrides, and the isolates.
const fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )
}

/// Returns the exit code of a second `signal` while flightdeck stops: the
/// code that a shell gives a process that the signal ended, 130 for Ctrl-C
/// and 143 for SIGTERM.
fn forced_exit(signal: i32) -> i32 {
    Status::Signal(signal).shell_code()
}

/// Sets `stop` on Ctrl-C and SIGTERM. A second signal while flightdeck
/// stops ends the process at once with the code that [`forced_exit`] gives
/// and leaves the opener directory behind.
fn watch_signals(stop: &Arc<AtomicBool>) -> io::Result<()> {
    for signal in [SIGINT, SIGTERM] {
        // signal-hook runs the handlers in the order they were registered,
        // so the shutdown sees the flag that only an earlier signal set.
        signal_hook::flag::register_conditional_shutdown(
            signal,
            forced_exit(signal),
            Arc::clone(stop),
        )?;
        signal_hook::flag::register(signal, Arc::clone(stop))?;
    }
    Ok(())
}
