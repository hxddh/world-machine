use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

const WRITE_RETRY_GRANULARITY: Duration = Duration::from_millis(1);

pub(crate) fn configure(stdin: &std::process::ChildStdin) -> io::Result<()> {
    configure_nonblocking(stdin)
}

pub(crate) fn write_all_until(
    writer: &mut impl Write,
    mut bytes: &[u8],
    deadline: Instant,
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
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(timeout_error());
                }
                thread::sleep(remaining.min(WRITE_RETRY_GRANULARITY));
            }
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

#[cfg(not(unix))]
fn configure_nonblocking(_stdin: &std::process::ChildStdin) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WouldBlockThenWrite {
        unblock_at: Instant,
        written: Vec<u8>,
    }

    impl Write for WouldBlockThenWrite {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if Instant::now() < self.unblock_at {
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
            unblock_at: Instant::now() + Duration::from_millis(5),
            written: Vec::new(),
        };
        write_all_until(
            &mut writer,
            b"bounded frame",
            Instant::now() + Duration::from_millis(100),
        )
        .unwrap();
        assert_eq!(writer.written, b"bounded frame");
    }

    #[test]
    fn deadline_writer_stops_when_would_block_outlives_budget() {
        let mut writer = WouldBlockThenWrite {
            unblock_at: Instant::now() + Duration::from_secs(1),
            written: Vec::new(),
        };
        let started = Instant::now();
        let error = write_all_until(
            &mut writer,
            b"bounded frame",
            Instant::now() + Duration::from_millis(20),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_millis(250));
        assert!(writer.written.is_empty());
    }
}
