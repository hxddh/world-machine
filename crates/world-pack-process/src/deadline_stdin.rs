use std::io::{self, Write};
use std::time::{Duration, Instant};

pub(crate) fn configure(stdin: &std::process::ChildStdin) -> io::Result<()> {
    configure_nonblocking(stdin)
}

pub(crate) fn write_all_until(
    stdin: &mut std::process::ChildStdin,
    bytes: &[u8],
    deadline: Instant,
) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;

        let fd = stdin.as_raw_fd();
        write_all_with_wait_until(stdin, bytes, deadline, || {
            wait_writable_fd(fd, deadline)
        })
    }

    #[cfg(not(unix))]
    {
        write_all_with_wait_until(stdin, bytes, deadline, || {
            let remaining = remaining(deadline)?;
            std::thread::sleep(remaining.min(Duration::from_millis(1)));
            Ok(())
        })
    }
}

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

#[cfg(not(unix))]
fn configure_nonblocking(_stdin: &std::process::ChildStdin) -> io::Result<()> {
    Ok(())
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
}
