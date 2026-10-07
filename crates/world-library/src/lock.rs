//! One writer per World, across apps.
//!
//! The demo and the full app share a Worlds folder, and a World file can be
//! opened from anywhere, so two processes (or two windows of one) could each
//! hold a session on the same file; their writers would then refuse each
//! other's saves at best. While a World is open, its session holds an
//! exclusive advisory lock on a small lock file beside the World's file,
//! `.<file name>.lock`: a second session on the same file is refused with
//! [`LibraryError::InUse`], and Home's own changes to a World's file (a
//! rename, a removal) take the same lock for as long as they write.
//!
//! The lock is the operating system's (`flock` on macOS and Linux), so it is
//! let go of when its holder exits, however it exits. The lock file is
//! removed by the holder as it lets go; a holder checks after locking that
//! the file it locked is still the one at the path, so a lock file removed
//! meanwhile is never mistaken for a lock held. Where a folder cannot hold
//! a lock file (read-only, or a file system without locks) the World opens
//! unlocked, as before: the lock guards against a second writer, and must
//! never keep a World from being opened at all.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::path::{Path, PathBuf};

use crate::LibraryError;

/// Where the lock for the World file at `path` is kept.
pub fn lock_path(path: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(path.file_name().unwrap_or_default());
    name.push(".lock");
    path.with_file_name(name)
}

/// An exclusive hold on one World file, let go of when dropped.
#[derive(Debug, Default)]
pub(crate) struct Lock {
    held: Option<(File, PathBuf)>,
}

impl Lock {
    /// [`Lock::take`] for a World file about to be made, its folder made
    /// first if need be.
    pub(crate) fn take_new(path: &Path) -> Result<Self, LibraryError> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            let _ = fs::create_dir_all(parent);
        }
        Self::take(path)
    }

    /// No lock: for a World with no file of its own, and in tests.
    pub(crate) fn none() -> Self {
        Self::default()
    }

    /// Takes the lock for the World file at `path`, or refuses with
    /// [`LibraryError::InUse`] if another session holds it, in this process
    /// or another. A folder that cannot hold the lock gives no lock.
    pub(crate) fn take(path: &Path) -> Result<Self, LibraryError> {
        let at = lock_path(path);
        // A lock file removed between opening and locking it is tried again.
        for _ in 0..8 {
            let file = match open(&at) {
                Ok(file) => file,
                Err(_) => return Ok(Self::none()),
            };
            match file.try_lock() {
                Ok(()) => {}
                Err(TryLockError::WouldBlock) => {
                    return Err(LibraryError::InUse(path.to_path_buf()))
                }
                Err(TryLockError::Error(_)) => return Ok(Self::none()),
            }
            if still_there(&file, &at) {
                return Ok(Self {
                    held: Some((file, at)),
                });
            }
        }
        Ok(Self::none())
    }

    /// Whether a lock is held.
    #[cfg(test)]
    pub(crate) fn is_held(&self) -> bool {
        self.held.is_some()
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        if let Some((file, at)) = self.held.take() {
            // Removed while still held, so nobody can lock this file and
            // then find it gone; then let go of.
            let _ = fs::remove_file(&at);
            drop(file);
        }
    }
}

fn open(at: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(at)
}

/// Whether `file` is still the file at `at`, rather than one a holder
/// removed as it let go after this opened it.
#[cfg(unix)]
fn still_there(file: &File, at: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (file.metadata(), fs::metadata(at)) {
        (Ok(held), Ok(there)) => held.dev() == there.dev() && held.ino() == there.ino(),
        _ => false,
    }
}

#[cfg(not(unix))]
fn still_there(_file: &File, at: &Path) -> bool {
    at.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "world-machine-lock-{}-{nonce}-{label}",
            std::process::id()
        ))
    }

    #[test]
    fn one_holder_at_a_time_and_the_next_once_it_lets_go() {
        let root = temp_root("one");
        fs::create_dir_all(&root).unwrap();
        let world = root.join("Harbour.world");
        let first = Lock::take(&world).unwrap();
        assert!(first.is_held());
        assert!(lock_path(&world).is_file());
        assert!(matches!(
            Lock::take(&world),
            Err(LibraryError::InUse(path)) if path == world
        ));
        drop(first);
        assert!(!lock_path(&world).exists(), "the lock file goes with it");
        let second = Lock::take(&world).unwrap();
        assert!(second.is_held());
        // Another World in the same folder is not held by it.
        assert!(Lock::take(&root.join("Other.world")).unwrap().is_held());
        drop(second);
        let _ = fs::remove_dir_all(root);
    }

    /// Run by [`another_app_holding_a_world_keeps_this_one_from_writing_it`]
    /// in a process of its own: holds the lock named by its environment
    /// until it is killed.
    #[test]
    #[ignore]
    fn hold_a_lock_until_killed() {
        let Some(world) = std::env::var_os("WORLD_MACHINE_HOLD_LOCK") else {
            return;
        };
        let _lock = Lock::take(Path::new(&world)).unwrap();
        println!("held");
        std::thread::sleep(std::time::Duration::from_secs(60));
    }

    #[test]
    fn another_app_holding_a_world_keeps_this_one_from_writing_it() {
        use std::io::{BufRead, BufReader};
        use std::process::{Command, Stdio};
        let root = temp_root("process");
        fs::create_dir_all(&root).unwrap();
        let world = root.join("Shared.world");
        let mut other = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "lock::tests::hold_a_lock_until_killed",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("WORLD_MACHINE_HOLD_LOCK", &world)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut lines = BufReader::new(other.stdout.take().unwrap()).lines();
        assert!(
            lines.any(|line| line.is_ok_and(|line| line.contains("held"))),
            "the other process took the lock"
        );
        assert!(matches!(
            Lock::take(&world),
            Err(LibraryError::InUse(path)) if path == world
        ));
        // An app that dies holding it lets go of it, file and all.
        other.kill().unwrap();
        other.wait().unwrap();
        assert!(Lock::take(&world).unwrap().is_held());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_folder_that_cannot_hold_a_lock_still_opens() {
        let root = temp_root("missing");
        // No such folder: nowhere to put the lock file.
        let lock = Lock::take(&root.join("nowhere").join("Harbour.world")).unwrap();
        assert!(!lock.is_held());
    }

    #[test]
    fn the_lock_file_is_hidden_beside_the_world() {
        assert_eq!(
            lock_path(Path::new("/w/My Town.world")),
            PathBuf::from("/w/.My Town.world.lock")
        );
    }
}
