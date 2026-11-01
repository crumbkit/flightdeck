//! The thread that follows one child: it starts the readers of the child's
//! pipes, stops the child when asked, kills what is left of its process
//! group and reaps it.

#![expect(
    clippy::disallowed_types,
    reason = "spawn owns flightdeck's child processes (PROC-1)"
)]

use std::io;
use std::process::Child;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use rustix::process::{Pid, Signal};

use super::queue::Lines;
use super::readers::Readers;
use super::{Control, Ended, Escalation, Limits, Stop, Stopper};
use crate::spawn::{Exit, SpawnError, exit_of, joined};

/// What the thread that follows one child holds.
pub(super) struct Follower {
    limits: Limits,
    escalation: Escalation,
    stopper: Stopper,
    controls: Receiver<Control>,
}

impl Follower {
    /// Returns the follower of a run within `limits` that a stop escalates
    /// by `escalation`, which reads its stops and the child's exit from
    /// `controls`.
    pub(super) const fn new(
        limits: Limits,
        escalation: Escalation,
        stopper: Stopper,
        controls: Receiver<Control>,
    ) -> Self {
        Self {
            limits,
            escalation,
            stopper,
            controls,
        }
    }

    /// Follows `child` to its end, putting each line of its standard output
    /// on `lines`, and returns how it ended.
    pub(super) fn follow(self, mut child: Child, lines: &Lines) -> Result<Ended, SpawnError> {
        let pipes = (child.stdout.take(), child.stderr.take());
        std::thread::scope(|scope| {
            // The readers' reads end a grace after the reap is marked, which
            // happens once the child is reaped or the readers fail to start,
            // and before the scope joins the readers.
            let started = Readers::start(scope, self.limits, &self.stopper, lines, pipes, &child);
            let readers = match started {
                Ok(readers) => readers,
                Err(error) => {
                    kill_now(&mut child);
                    lines.reaped();
                    return Err(SpawnError::Wait(error));
                }
            };
            let waited = self.wait(&mut child);
            lines.reaped();
            let (stdout, stderr) = readers.join();
            let (exit, stopped) = waited.map_err(SpawnError::Wait)?;
            joined(stdout)?;
            Ok(Ended {
                exit,
                stopped,
                stderr: joined(stderr)?,
            })
        })
    }

    /// Waits for `child` to exit, stopping it when a stop arrives, then
    /// kills what is left of its process group and reaps it.
    fn wait(&self, child: &mut Child) -> io::Result<(Exit, Option<Stop>)> {
        // The follower holds a sender of its own, so the receiver never
        // finds every sender gone.
        let stopped = match self.controls.recv() {
            Ok(Control::Stop(stop)) => Some(stop),
            Ok(Control::Exited) | Err(_) => None,
        };
        if stopped.is_some() {
            self.escalate(child);
        }
        // The child is a zombie until `wait` reaps it, so its id still names
        // its process group.
        signal_group(child, Signal::KILL);
        let status = child.wait()?;
        Ok((exit_of(status), stopped))
    }

    /// Sends SIGINT, SIGTERM and SIGKILL to `child`'s process group, each
    /// after the previous one's grace, until the child exits.
    fn escalate(&self, child: &Child) {
        for (signal, grace) in [
            (Signal::INT, self.escalation.interrupt),
            (Signal::TERM, self.escalation.terminate),
        ] {
            signal_group(child, signal);
            if self.exited_within(grace) {
                return;
            }
        }
        signal_group(child, Signal::KILL);
        while let Ok(Control::Stop(_)) = self.controls.recv() {}
    }

    /// Returns true when the child exits within `grace`, or has exited.
    fn exited_within(&self, grace: Duration) -> bool {
        loop {
            match self.controls.recv_timeout(grace) {
                Ok(Control::Exited) | Err(RecvTimeoutError::Disconnected) => return true,
                Err(RecvTimeoutError::Timeout) => return false,
                // The stopper sends one stop, which `wait` has read, so no
                // other arrives.
                Ok(Control::Stop(_)) => {}
            }
        }
    }
}

/// Kills `child` and reaps it, for a run whose readers did not start.
fn kill_now(child: &mut Child) {
    signal_group(child, Signal::KILL);
    let _ = child.wait();
}

/// Sends `signal` to every process in `child`'s process group, which
/// `start` created with the child's id, and to the child itself when it has
/// moved to another group.
///
/// The child is unreaped while the follower signals it, so its id still
/// names it. The kernel refuses a group kill only when no process of the
/// group is left, and the caller then waits for an exit that has already
/// happened, so a refusal needs no handling.
fn signal_group(child: &Child, signal: Signal) {
    let pid = Pid::from_child(child);
    let _ = rustix::process::kill_process_group(pid, signal);
    // The child gets the signal once: through its group while it is in it,
    // and directly once it has left it.
    if rustix::process::getpgid(Some(pid)).ok() != Some(pid) {
        let _ = rustix::process::kill_process(pid, signal);
    }
}
