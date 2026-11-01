//! A child's pipe whose reads end a grace after the child has been reaped,
//! so a descendant that holds the pipe open cannot hold up the run.

use std::io::{self, PipeReader, Read};
use std::time::{Duration, Instant};

/// How long a run's readers keep reading after the child has been reaped.
/// It lets them read what the child printed just before it exited.
pub const DRAIN_GRACE: Duration = Duration::from_secs(1);

/// A child's pipe that reads until the child has been reaped and then for
/// [`DRAIN_GRACE`] more, after which a read returns the end of the stream.
///
/// The follower holds the writer of `reaped` and drops it once the child is
/// reaped, which makes `reaped` readable.
#[derive(Debug)]
pub(super) struct Drained<R> {
    pipe: R,
    reaped: PipeReader,
    #[cfg_attr(not(unix), expect(dead_code, reason = "only Unix polls the pipe"))]
    deadline: Option<Instant>,
}

impl<R> Drained<R> {
    /// Returns `pipe`, read until `reaped` is readable and then for
    /// [`DRAIN_GRACE`].
    pub(super) const fn new(pipe: R, reaped: PipeReader) -> Self {
        Self {
            pipe,
            reaped,
            deadline: None,
        }
    }
}

#[cfg(unix)]
impl<R: Read + std::os::fd::AsFd> Read for Drained<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use rustix::event::{PollFd, PollFlags, Timespec, poll};

        loop {
            let ready = match self.deadline {
                None => {
                    let mut fds = [
                        PollFd::new(&self.pipe, PollFlags::IN),
                        PollFd::new(&self.reaped, PollFlags::IN),
                    ];
                    match poll(&mut fds, None) {
                        Ok(_) if !fds[0].revents().is_empty() => true,
                        Ok(_) => {
                            self.deadline = Some(Instant::now() + DRAIN_GRACE);
                            false
                        }
                        Err(rustix::io::Errno::INTR) => false,
                        Err(error) => return Err(error.into()),
                    }
                }
                Some(deadline) => {
                    let left = deadline.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Ok(0);
                    }
                    let timeout = Timespec::try_from(left).map_err(io::Error::other)?;
                    let mut fds = [PollFd::new(&self.pipe, PollFlags::IN)];
                    match poll(&mut fds, Some(&timeout)) {
                        Ok(0) => return Ok(0),
                        Ok(_) => true,
                        Err(rustix::io::Errno::INTR) => false,
                        Err(error) => return Err(error.into()),
                    }
                }
            };
            if ready {
                return self.pipe.read(buf);
            }
        }
    }
}

/// RUST-10: elsewhere than Unix, a read waits for the pipe without the
/// grace, so a descendant that holds the pipe open holds the run too. A
/// read that polls the pipe's handle replaces it when flightdeck runs on
/// such a system.
#[cfg(not(unix))]
impl<R: Read> Read for Drained<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let _ = &self.reaped;
        self.pipe.read(buf)
    }
}
