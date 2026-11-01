//! A child's pipe whose reads end a grace after the child has been reaped,
//! so a descendant that holds the pipe open cannot hold up the run.

use std::io::{self, Read};
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use rustix::event::{PollFd, PollFlags, Timespec, poll};

use super::queue::Lines;

#[cfg(test)]
mod tests;

/// How long a run's readers keep reading after the child has been reaped.
/// It lets them read what the child printed just before it exited.
pub const DRAIN_GRACE: Duration = Duration::from_secs(1);

/// How often a read that waits for the pipe checks whether the child has
/// been reaped.
///
/// The reap is a time in the run's queue, which the read checks between
/// waits. A second pipe whose writer the follower closes would wake the
/// read at once, but std on macOS creates a pipe and sets close-on-exec in
/// two steps, so a child that another thread starts in between would hold
/// that writer open.
const REAP_CHECK: Duration = Duration::from_millis(100);

/// A child's pipe that reads until the child has been reaped and then for
/// [`DRAIN_GRACE`] more, after which a read returns the end of the stream.
#[derive(Debug)]
pub(super) struct Drained<R> {
    pipe: R,
    lines: Lines,
}

impl<R> Drained<R> {
    /// Returns `pipe`, read until `lines` marks the reap and then for
    /// [`DRAIN_GRACE`].
    pub(super) const fn new(pipe: R, lines: Lines) -> Self {
        Self { pipe, lines }
    }
}

impl<R: Read + AsFd> Read for Drained<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let wait = match self.lines.reaped_at() {
                None => REAP_CHECK,
                Some(reaped) => {
                    let left = (reaped + DRAIN_GRACE).saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Ok(0);
                    }
                    left.min(REAP_CHECK)
                }
            };
            let timeout = Timespec::try_from(wait).map_err(io::Error::other)?;
            let mut fds = [PollFd::new(&self.pipe, PollFlags::IN)];
            match poll(&mut fds, Some(&timeout)) {
                Ok(0) | Err(rustix::io::Errno::INTR) => {}
                Ok(_) => return self.pipe.read(buf),
                Err(error) => return Err(error.into()),
            }
        }
    }
}
