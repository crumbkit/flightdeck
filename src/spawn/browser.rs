//! The operation that opens the opener page in the person's default browser.
//! It states the five items of PROC-2:
//!
//! - Program: `open` on macOS and `xdg-open` on other Unix systems, found on
//!   the `PATH` that flightdeck inherited. The system that provides them is
//!   trusted to supply their bytes.
//! - Arguments: one, the page's path, with no shell. The path holds no
//!   secret; the launch code is inside the file.
//! - Environment: the child inherits flightdeck's environment.
//! - Streams and deadline: all three streams are null, and the run has
//!   [`BROWSER_TIME_LIMIT`].
//! - End: [`open_browser`] runs the opener through crumbkit-process in the
//!   scope of the opener alone, waits for it and reads its exit status. When
//!   the time limit passes, it kills the opener alone and reaps it. The
//!   browser that the opener starts is a descendant that flightdeck never
//!   stops, because opening it is the operation's purpose (PROC-3).

use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use crumbkit_process::{Run, Scope};

use super::{Exit, Program, SpawnError};

#[cfg(unix)]
#[cfg(test)]
mod tests;

/// How long the opener may take to hand the page to the browser.
pub const BROWSER_TIME_LIMIT: Duration = Duration::from_secs(10);

/// The program that opens a file in the default browser on macOS.
#[cfg(target_os = "macos")]
const OPENER: Option<&str> = Some("open");

/// The program that opens a file in the default browser on other Unix
/// systems.
#[cfg(all(unix, not(target_os = "macos")))]
const OPENER: Option<&str> = Some("xdg-open");

/// RUST-10: flightdeck knows no opener outside Unix. It handles macOS and
/// the other Unix systems, and on Windows it fails to start the opener, so
/// `flightdeck open` prints the page's path for the person to open. A
/// Windows opener replaces this; `rundll32` or `cmd /c start` would take
/// the page's path as a `file:` URL, so a path with spaces or `&` survives
/// their own parsing of the command line.
#[cfg(not(unix))]
const OPENER: Option<&str> = None;

/// Opens `page` in the default browser and returns how the opener ended.
///
/// # Errors
///
/// Returns [`SpawnError::Start`] when the system has no opener or the
/// opener cannot start, and [`SpawnError::Wait`] when waiting for it fails.
pub fn open_browser(page: &Path) -> Result<Exit, SpawnError> {
    let Some(opener) = OPENER else {
        return Err(SpawnError::Start(io::Error::from(
            io::ErrorKind::Unsupported,
        )));
    };
    open_with(opener.as_ref(), page, BROWSER_TIME_LIMIT)
}

/// Runs `opener` with `page` as its one argument, waits at most
/// `time_limit` for it, and returns how it ended.
fn open_with(opener: &OsStr, page: &Path, time_limit: Duration) -> Result<Exit, SpawnError> {
    let settings = Run::new()
        .scope(Scope::Child)
        .deadline(Instant::now() + time_limit);
    let finished = crumbkit_process::run(Program::new(opener).command(&[page]), &settings)?;
    Ok(match finished.exit() {
        crumbkit_process::Exit::TimedOut => Exit::TimedOut,
        _ => Exit::of(finished.status()),
    })
}
