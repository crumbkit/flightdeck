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
//! - End: [`open_browser`] waits for the opener and reads its exit status.
//!   When the time limit passes, it kills the opener alone and reaps it. The
//!   browser that the opener starts is a descendant that flightdeck never
//!   stops, because opening it is the operation's purpose (PROC-3).

use std::ffi::OsStr;
use std::io;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use wait_timeout::ChildExt as _;

use super::{Exit, SpawnError, exit_of};

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
/// the other Unix systems, and on any other system it fails to start the
/// opener, so `flightdeck open` prints the page's path instead. A Windows
/// opener replaces this when flightdeck supports Windows.
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
    #[expect(
        clippy::disallowed_methods,
        reason = "spawn owns flightdeck's child processes (PROC-1)"
    )]
    let mut command = std::process::Command::new(opener);
    command
        .arg(page)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[expect(
        clippy::disallowed_methods,
        reason = "spawn owns flightdeck's child processes (PROC-1)"
    )]
    let mut child = command.spawn().map_err(SpawnError::Start)?;
    if let Some(status) = child.wait_timeout(time_limit).map_err(SpawnError::Wait)? {
        return Ok(exit_of(status));
    }
    // An opener that exits between the timed wait and the kill leaves
    // nothing to kill, and that refusal is no failure.
    if let Err(error) = child.kill()
        && child.try_wait().map_err(SpawnError::Wait)?.is_none()
    {
        return Err(SpawnError::Wait(error));
    }
    child.wait().map_err(SpawnError::Wait)?;
    Ok(Exit::TimedOut)
}
