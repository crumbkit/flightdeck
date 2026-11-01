//! The threads that read a child's two pipes and the thread that waits for
//! its exit, all in the follower's scope.

#![expect(
    clippy::disallowed_types,
    reason = "spawn owns flightdeck's child processes (PROC-1)"
)]

use std::io::{self, BufRead, BufReader, Read as _};
use std::process::{Child, ChildStderr, ChildStdout};
use std::sync::mpsc::SyncSender;
use std::thread::{Scope, ScopedJoinHandle};

use rustix::process::Pid;

use super::drain::Drained;
use super::queue::Lines;
use super::{Control, Limit, Limits, Stop, Stopper};
use crate::spawn::{STDERR_LIMIT, read_tail, wait_unreaped};

#[cfg(test)]
mod tests;

/// The threads that read a child's streams and the thread that waits for
/// its exit.
pub(super) struct Readers<'scope> {
    stdout: ScopedJoinHandle<'scope, io::Result<()>>,
    stderr: ScopedJoinHandle<'scope, io::Result<Vec<u8>>>,
    exit: ScopedJoinHandle<'scope, ()>,
}

impl<'scope> Readers<'scope> {
    /// Starts the readers of the child's `stdout` and `stderr` in `scope`,
    /// and the waiter of `child`'s exit. Standard output goes to `lines`
    /// within `limits`, and `stopper` stops the child when it passes them.
    #[expect(
        clippy::disallowed_methods,
        reason = "spawn owns flightdeck's threads (SVC-1), and the scope joins these"
    )]
    pub(super) fn start(
        scope: &'scope Scope<'scope, '_>,
        limits: Limits,
        stopper: &Stopper,
        lines: &Lines,
        (stdout, stderr): (Option<ChildStdout>, Option<ChildStderr>),
        child: &Child,
    ) -> io::Result<Self> {
        let stdout = stdout.map(|pipe| Drained::new(pipe, lines.clone()));
        let stderr = stderr.map(|pipe| Drained::new(pipe, lines.clone()));
        let (lines, stopper) = (lines.clone(), stopper.clone());
        let pid = Pid::from_child(child);
        let exited = stopper.control.clone();
        let named = |name: &str| std::thread::Builder::new().name(name.to_owned());
        Ok(Self {
            stdout: named("flightdeck stdout")
                .spawn_scoped(scope, move || read_lines(stdout, limits, &lines, &stopper))?,
            stderr: named("flightdeck stderr").spawn_scoped(scope, move || {
                stderr.map_or_else(|| Ok(Vec::new()), |pipe| read_tail(pipe, STDERR_LIMIT))
            })?,
            exit: named("flightdeck exit")
                .spawn_scoped(scope, move || wait_exited(pid, &exited))?,
        })
    }

    /// Waits for the three threads, and returns what the two readers read.
    pub(super) fn join(
        self,
    ) -> (
        std::thread::Result<io::Result<()>>,
        std::thread::Result<io::Result<Vec<u8>>>,
    ) {
        let stdout = self.stdout.join();
        let stderr = self.stderr.join();
        let _ = self.exit.join();
        (stdout, stderr)
    }
}

/// Waits until the child `pid` exits, without reaping it, and tells the
/// thread that follows it. A failed wait is told as an exit too, and the
/// follower's own wait then reads the child's status.
fn wait_exited(pid: Pid, control: &SyncSender<Control>) {
    let _ = wait_unreaped(pid);
    let _ = control.send(Control::Exited);
}

/// Reads standard output line by line and puts each line on `lines`. When
/// the output passes a limit, it stops the child and reads the rest of the
/// pipe without keeping it, so the child never blocks on a full pipe.
fn read_lines(
    pipe: Option<Drained<ChildStdout>>,
    limits: Limits,
    lines: &Lines,
    stopper: &Stopper,
) -> io::Result<()> {
    let Some(pipe) = pipe else {
        return Ok(());
    };
    let mut reader = BufReader::new(pipe);
    // Once the caller has dropped its side, nobody reads the lines, and the
    // reader reads the rest of the pipe without keeping it.
    let mut listening = true;
    let over = deliver(&mut reader, limits, &mut |line| {
        listening = listening && lines.push(line);
    })?;
    if let Some(limit) = over {
        stopper.stop(Stop::Limit(limit));
        io::copy(&mut reader, &mut io::sink())?;
    }
    Ok(())
}

/// Passes each line of `reader` to `line` until the reader ends or a line
/// or the total passes `limits`, and returns the limit it passed, if any.
/// The lines after that limit are not passed.
fn deliver(
    reader: &mut impl BufRead,
    limits: Limits,
    line: &mut impl FnMut(String),
) -> io::Result<Option<Limit>> {
    let most = u64::try_from(limits.line())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    let mut total: usize = 0;
    loop {
        bytes.clear();
        let read = reader.by_ref().take(most).read_until(b'\n', &mut bytes)?;
        if read == 0 {
            return Ok(None);
        }
        total = total.saturating_add(read);
        if total > limits.total() {
            return Ok(Some(Limit::Total));
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        } else if read > limits.line() {
            return Ok(Some(Limit::Line));
        }
        line(String::from_utf8_lossy(&bytes).into_owned());
    }
}
