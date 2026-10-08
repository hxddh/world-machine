//! Writing a request frame to a Pack's standard input by a deadline.
//!
//! A Pack that stops reading must not hang the app. Two ways to bound the
//! write, one per kind of host:
//!
//! - **Unix:** the pipe is made non-blocking, and a write that would block
//!   waits in `poll` for the time left. This is the only `unsafe` in the
//!   package (two libc calls), which is why the crate denies `unsafe_code`
//!   and allows it in this module alone.
//! - **Everywhere else (Windows):** standard input is handed to a writer
//!   thread of its own, and the caller waits for that thread's word by the
//!   deadline. A Pack that never reads leaves the thread blocked until the
//!   caller gives up and kills the Pack, which breaks the pipe and ends the
//!   thread. No `unsafe`, and the same behaviour seen from the caller:
//!   `TimedOut` at the deadline, and the session ends.

use std::io;
#[cfg(any(unix, test))]
use std::io::Write;
use std::process::ChildStdin;
use std::time::{Duration, Instant};

/// A Pack's standard input, ready for bounded writes.
pub(crate) struct BoundedStdin {
    #[cfg(unix)]
    pipe: ChildStdin,
    #[cfg(not(unix))]
    pipe: threaded::ThreadedStdin,
}

/// Prepares a Pack's standard input for writes that end by a deadline.
pub(crate) fn configure(stdin: ChildStdin) -> io::Result<BoundedStdin> {
    #[cfg(unix)]
    {
        configure_nonblocking(&stdin)?;
        Ok(BoundedStdin { pipe: stdin })
    }
    #[cfg(not(unix))]
    {
        Ok(BoundedStdin {
            pipe: threaded::ThreadedStdin::spawn(stdin)?,
        })
    }
}

pub(crate) fn write_all_until(
    stdin: &mut BoundedStdin,
    bytes: &[u8],
    deadline: Instant,
) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;

        let fd = stdin.pipe.as_raw_fd();
        write_all_with_wait_until(&mut stdin.pipe, bytes, deadline, || {
            wait_writable_fd(fd, deadline)
        })
    }

    #[cfg(not(unix))]
    {
        stdin.pipe.write_all_until(bytes, deadline)
    }
}

/// The writer-thread path: what bounds a write where pipes cannot be made
/// non-blocking. Compiled on every host for its tests, used off unix.
#[cfg(any(not(unix), test))]
mod threaded {
    use super::{remaining, timeout_error};
    use std::io::{self, Write};
    use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
    use std::thread;
    use std::time::Instant;

    pub(crate) struct ThreadedStdin {
        frames: Option<Sender<Vec<u8>>>,
        written: Receiver<io::Result<()>>,
        /// A write that has not reported yet: a later frame must not be
        /// mistaken for it.
        busy: bool,
    }

    impl ThreadedStdin {
        pub(crate) fn spawn(mut pipe: impl Write + Send + 'static) -> io::Result<Self> {
            let (frames, inbox) = mpsc::channel::<Vec<u8>>();
            let (report, written) = mpsc::channel();
            thread::Builder::new()
                .name("pack-stdin".into())
                .spawn(move || {
                    // Ends when the session drops its sender (closing the
                    // pipe with it) or a write fails because the Pack is gone.
                    for frame in inbox {
                        let result = pipe.write_all(&frame).and_then(|()| pipe.flush());
                        let failed = result.is_err();
                        if report.send(result).is_err() || failed {
                            return;
                        }
                    }
                })?;
            Ok(Self {
                frames: Some(frames),
                written,
                busy: false,
            })
        }

        pub(crate) fn write_all_until(
            &mut self,
            bytes: &[u8],
            deadline: Instant,
        ) -> io::Result<()> {
            if self.busy {
                // The last write never finished in time; the session is
                // over, whatever the caller tries next.
                return Err(timeout_error());
            }
            remaining(deadline)?;
            let frames = self.frames.as_ref().ok_or_else(closed)?;
            frames.send(bytes.to_vec()).map_err(|_| closed())?;
            self.busy = true;
            match self.written.recv_timeout(remaining(deadline)?) {
                Ok(result) => {
                    self.busy = false;
                    result
                }
                Err(RecvTimeoutError::Timeout) => Err(timeout_error()),
                Err(RecvTimeoutError::Disconnected) => Err(closed()),
            }
        }
    }

    impl Drop for ThreadedStdin {
        fn drop(&mut self) {
            // Dropping the sender ends the thread's loop, which drops (and
            // so closes) the pipe once any write in progress returns.
            self.frames.take();
        }
    }

    fn closed() -> io::Error {
        io::Error::new(io::ErrorKind::BrokenPipe, "external Pack stdin is closed")
    }
}

#[cfg(any(unix, test))]
fn write_all_with_wait_until(
    writer: &mut impl Write,
    mut bytes: &[u8],
    deadline: Instant,
    mut wait_writable: impl FnMut() -> io::Result<()>,
) -> io::Result<()> {
    while !bytes.is_empty() {
        if deadline.saturating_duration_since(Instant::now()).is_zero() {
            return Err(timeout_error());
        }
        match writer.write(bytes) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "could not write Pack request frame",
                ));
            }
            Ok(written) => bytes = &bytes[written..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => wait_writable()?,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub(crate) fn remaining(deadline: Instant) -> io::Result<Duration> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        Err(timeout_error())
    } else {
        Ok(remaining)
    }
}

fn timeout_error() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "Pack request deadline elapsed")
}

#[cfg(unix)]
fn configure_nonblocking(stdin: &std::process::ChildStdin) -> io::Result<()> {
    use std::ffi::c_int;
    use std::os::fd::AsRawFd;

    const F_GETFL: c_int = 3;
    const F_SETFL: c_int = 4;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const O_NONBLOCK: c_int = 0o4000;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    const O_NONBLOCK: c_int = 0x0004;

    unsafe extern "C" {
        fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    }

    let fd = stdin.as_raw_fd();
    // SAFETY: `fd` is borrowed from a live ChildStdin and F_GETFL has no
    // additional variadic argument. The call does not take ownership of fd.
    let flags = unsafe { fcntl(fd, F_GETFL) };
    if flags == -1 {
        return Err(io::Error::last_os_error());
    }
    if flags & O_NONBLOCK != 0 {
        return Ok(());
    }

    // SAFETY: `fd` remains a live borrowed descriptor and F_SETFL expects one
    // integer flags argument. We preserve all existing status flags and only
    // add O_NONBLOCK; ownership and descriptor lifetime are unchanged.
    if unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn wait_writable_fd(fd: std::os::fd::RawFd, deadline: Instant) -> io::Result<()> {
    use std::ffi::{c_int, c_short};

    #[repr(C)]
    struct PollFd {
        fd: c_int,
        events: c_short,
        revents: c_short,
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    type Nfds = std::ffi::c_ulong;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    type Nfds = std::ffi::c_uint;

    const POLLOUT: c_short = 0x0004;

    unsafe extern "C" {
        fn poll(fds: *mut PollFd, nfds: Nfds, timeout: c_int) -> c_int;
    }

    loop {
        let timeout = poll_timeout_millis(deadline)?;
        let mut pollfd = PollFd {
            fd,
            events: POLLOUT,
            revents: 0,
        };
        // SAFETY: `pollfd` is a fully initialized single-element pollfd array,
        // `nfds` matches that one element, and poll only borrows it for this call.
        let ready = unsafe { poll(&mut pollfd, 1 as Nfds, timeout) };
        if ready > 0 {
            return Ok(());
        }
        if ready == 0 {
            return Err(timeout_error());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

#[cfg(unix)]
fn poll_timeout_millis(deadline: Instant) -> io::Result<std::ffi::c_int> {
    let remaining = remaining(deadline)?;
    let whole_millis = remaining.as_millis();
    let has_sub_millisecond = remaining.subsec_nanos() % 1_000_000 != 0;
    let rounded_up = whole_millis.saturating_add(u128::from(has_sub_millisecond));
    Ok(rounded_up
        .clamp(1, std::ffi::c_int::MAX as u128)
        .try_into()
        .expect("poll timeout is clamped to c_int::MAX"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WouldBlockThenWrite {
        remaining_blocks: usize,
        written: Vec<u8>,
    }

    impl Write for WouldBlockThenWrite {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.remaining_blocks > 0 {
                self.remaining_blocks -= 1;
                return Err(io::Error::from(io::ErrorKind::WouldBlock));
            }
            self.written.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn deadline_writer_retries_would_block_and_preserves_bytes() {
        let mut writer = WouldBlockThenWrite {
            remaining_blocks: 2,
            written: Vec::new(),
        };
        let mut waits = 0;
        write_all_with_wait_until(
            &mut writer,
            b"bounded frame",
            Instant::now() + Duration::from_millis(100),
            || {
                waits += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(waits, 2);
        assert_eq!(writer.written, b"bounded frame");
    }

    #[test]
    fn deadline_writer_propagates_wait_timeout_without_writing_later_bytes() {
        let mut writer = WouldBlockThenWrite {
            remaining_blocks: 1,
            written: Vec::new(),
        };
        let error = write_all_with_wait_until(
            &mut writer,
            b"bounded frame",
            Instant::now() + Duration::from_secs(1),
            || Err(timeout_error()),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(writer.written.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn poll_timeout_rounds_sub_millisecond_budget_up() {
        let timeout = poll_timeout_millis(Instant::now() + Duration::from_micros(500)).unwrap();
        assert_eq!(timeout, 1);
    }

    /// A pipe nobody reads: every write blocks until the test lets go.
    struct Stalled(std::sync::mpsc::Receiver<()>);

    impl Write for Stalled {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            let _ = self.0.recv();
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn the_writer_thread_writes_whole_frames_in_order() {
        let (sink, read) = std::sync::mpsc::channel::<Vec<u8>>();
        struct Collect(std::sync::mpsc::Sender<Vec<u8>>);
        impl Write for Collect {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                let _ = self.0.send(bytes.to_vec());
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut stdin = threaded::ThreadedStdin::spawn(Collect(sink)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        stdin.write_all_until(b"one\n", deadline).unwrap();
        stdin.write_all_until(b"two\n", deadline).unwrap();
        drop(stdin);
        let written: Vec<u8> = read.iter().flatten().collect();
        assert_eq!(written, b"one\ntwo\n");
    }

    #[test]
    fn the_writer_thread_gives_up_at_the_deadline_when_the_pack_stops_reading() {
        let (release, stalled) = std::sync::mpsc::channel();
        let mut stdin = threaded::ThreadedStdin::spawn(Stalled(stalled)).unwrap();
        let started = Instant::now();
        let error = stdin
            .write_all_until(b"frame", started + Duration::from_millis(150))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "the deadline did not hold"
        );
        // A stalled write is never mistaken for the next one's success.
        let again = stdin
            .write_all_until(b"next", Instant::now() + Duration::from_secs(1))
            .unwrap_err();
        assert_eq!(again.kind(), io::ErrorKind::TimedOut);
        // Killing the Pack breaks the pipe, which ends the thread.
        drop(release);
    }

    #[test]
    fn the_writer_thread_reports_a_pack_that_is_gone() {
        struct Gone;
        impl Write for Gone {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::from(io::ErrorKind::BrokenPipe))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut stdin = threaded::ThreadedStdin::spawn(Gone).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let first = stdin.write_all_until(b"frame", deadline).unwrap_err();
        assert_eq!(first.kind(), io::ErrorKind::BrokenPipe);
        let second = stdin.write_all_until(b"frame", deadline).unwrap_err();
        assert_eq!(second.kind(), io::ErrorKind::BrokenPipe);
    }

    /// The Windows path against a real child that never reads its input:
    /// the write ends by the deadline, and killing the child ends the thread.
    #[cfg(unix)]
    #[test]
    fn the_writer_thread_bounds_a_real_pipe_to_a_child_that_never_reads() {
        use std::process::{Command, Stdio};
        let mut child = Command::new("sleep")
            .arg("30")
            .stdin(Stdio::piped())
            .spawn()
            .expect("sleep runs");
        let pipe = child.stdin.take().unwrap();
        let mut stdin = threaded::ThreadedStdin::spawn(pipe).unwrap();
        // Larger than any pipe buffer, so the write must block.
        let frame = vec![b'x'; 8 * 1024 * 1024];
        let started = Instant::now();
        let error = stdin
            .write_all_until(&frame, started + Duration::from_millis(200))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(3));
        child.kill().unwrap();
        child.wait().unwrap();
        drop(stdin);
    }
}
