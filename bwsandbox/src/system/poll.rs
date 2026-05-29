use rustix::event::{PollFd, PollFlags};
use rustix::io::Errno;
use std::io::ErrorKind;
use std::os::fd::{AsFd, BorrowedFd};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Poll<'a> {
    fd: BorrowedFd<'a>,
}

impl<'a> Poll<'a> {
    pub fn new<Fd: AsFd>(fd: &'a Fd) -> Self {
        Self { fd: fd.as_fd() }
    }

    fn poll(&self, timeout: Duration, flags: PollFlags) -> Result<PollFlags, Errno> {
        let timeout = timeout.try_into().expect("bad duration");
        let mut fds = [PollFd::new(&self.fd, flags)];
        let ready = rustix::event::poll(&mut fds, Some(&timeout))?;
        if ready == 0 {
            return Err(Errno::TIMEDOUT);
        }
        Ok(fds[0].revents())
    }

    pub fn poll_in(&self, timeout: Duration) -> Result<(), std::io::Error> {
        let now = Instant::now();

        loop {
            let timeout = timeout.saturating_sub(now.elapsed());
            if timeout.is_zero() {
                return Err(ErrorKind::TimedOut.into());
            }

            let flags = match self.poll(timeout, PollFlags::IN | PollFlags::HUP) {
                Err(Errno::INTR) => continue,
                Err(e) => return Err(e.into()),
                Ok(v) => v,
            };

            if flags.contains(PollFlags::HUP) && !flags.contains(PollFlags::IN) {
                return Err(ErrorKind::UnexpectedEof.into());
            }

            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const SHORT: Duration = Duration::from_millis(100);
    const LONG: Duration = Duration::from_secs(5);

    #[test]
    fn poll_in_data_available() {
        let (rx, mut tx) = std::io::pipe().unwrap();
        std::io::Write::write_all(&mut tx, b"hello").unwrap();
        let poll = Poll::new(&rx);
        poll.poll_in(SHORT).unwrap();
    }

    #[test]
    fn poll_in_timeout_no_data() {
        let (rx, _tx) = std::io::pipe().unwrap();
        let poll = Poll::new(&rx);
        let err = poll.poll_in(SHORT).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::TimedOut);
    }

    #[test]
    fn poll_in_hup_no_data() {
        let (rx, tx) = std::io::pipe().unwrap();
        drop(tx); // close write end → POLLHUP
        let poll = Poll::new(&rx);
        let err = poll.poll_in(SHORT).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn poll_in_hup_with_data() {
        // POLLIN | POLLHUP simultaneously — data should win
        let (rx, mut tx) = std::io::pipe().unwrap();
        std::io::Write::write_all(&mut tx, b"hello").unwrap();
        drop(tx); // close write end with data still in buffer
        let poll = Poll::new(&rx);
        // should succeed — data is available even though write end closed
        poll.poll_in(SHORT).unwrap();
    }

    #[test]
    fn poll_in_unblocks_when_data_arrives() {
        let (rx, mut tx) = std::io::pipe().unwrap();
        let poll = Poll::new(&rx);
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            std::io::Write::write_all(&mut tx, b"hello").unwrap();
        });
        poll.poll_in(LONG).unwrap();
        handle.join().unwrap();
    }
}
