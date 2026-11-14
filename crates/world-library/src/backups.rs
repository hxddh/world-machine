//! A World file's backups and the small file operations around a save:
//! the `.bak` of the save before and the day backup, putting a good one
//! back, reading a summary from a file's start, and the temp names,
//! renames and folder syncs a save is made of.

#[allow(unused_imports)]
use super::*;

/// What a damaged World file's name gains when its last good backup is
/// put back in its place: it is kept beside it, never deleted.
pub const DAMAGED_SUFFIX: &str = ".damaged";

/// Whether the World file at `path` has a backup that opens as a World
/// file of this World Machine's: the last good save, to put back when the
/// file itself will not open ([`restore_backup`]).
pub fn has_good_backup(path: &Path) -> bool {
    good_backup(path).is_some()
}

/// The newest backup of the World file at `path` that reads as a World
/// file of this World Machine's: its `.bak`, else its day backup.
pub(crate) fn good_backup(path: &Path) -> Option<ReadFile> {
    read_file(&backup_path(path))
        .ok()
        .or_else(|| read_file(&day_backup_path(path)).ok())
}

/// Puts the World file at `path` back to its last good save, its backup
/// (see [`backup_path`]), for when the file itself fails to load: the
/// backup is checked to read as a World file of this World Machine's
/// first, and the file it replaces is kept beside it under
/// [`DAMAGED_SUFFIX`], never deleted. The backup stays as it is. Refused
/// while the World is open anywhere ([`LibraryError::InUse`]), and when
/// there is no good backup. Returns where the damaged file was kept, if
/// there was one.
pub fn restore_backup(path: &Path) -> Result<Option<PathBuf>, LibraryError> {
    let _lock = Lock::take(path)?;
    let good = match good_backup(path) {
        Some(good) => good,
        // Why the newest backup does not open.
        None => read_file(&backup_path(path))?,
    };
    let kept = if path.try_exists()? {
        let mut name = path
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_default();
        name.push(DAMAGED_SUFFIX);
        let mut damaged = path.with_file_name(&name);
        let mut attempt = 2;
        while damaged.try_exists()? {
            let mut numbered = name.clone();
            numbered.push(format!("-{attempt}"));
            damaged = path.with_file_name(numbered);
            attempt += 1;
        }
        fs::rename(path, &damaged)?;
        Some(damaged)
    } else {
        None
    };
    // Written as any save is, atomically; the file it would keep as a
    // backup was just moved aside, so the last good save stays the backup.
    if let Err(error) = atomic_write(path, &good.bytes) {
        if let Some(damaged) = &kept {
            let _ = fs::rename(damaged, path);
        }
        return Err(error.into());
    }
    Ok(kept)
}

/// Where the backup of the World file at `path` is kept.
pub fn backup_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(BACKUP_SUFFIX);
    path.with_file_name(name)
}

/// The summary at the start of the World file at `path`, read from its
/// first `head_bytes`, or from the whole file when they do not hold it.
pub(crate) fn summary_of_file(
    path: &Path,
    head_bytes: usize,
) -> Result<Option<DocumentSummary>, LibraryError> {
    let head = match read_head(path, head_bytes) {
        Ok(head) => head,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(LibraryError::Io(error)),
    };
    let summary = if head.len() < head_bytes {
        // The whole file.
        WorldDocument::summary_from_bytes(&head)?
    } else {
        match WorldDocument::summary_from_bytes(&head) {
            Ok(Some(summary)) => Some(summary),
            // Cut off before its summary ends: read whole.
            _ => match fs::read(path) {
                Ok(bytes) => WorldDocument::summary_from_bytes(&bytes)?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(LibraryError::Io(error)),
            },
        }
    };
    let summary = summary.ok_or_else(|| LibraryError::from(DocumentError::PreviewFile))?;
    summary.writer.check()?;
    Ok(Some(summary))
}

/// How much of a World file's start Home reads for its summary.
pub(crate) const SUMMARY_HEAD_BYTES: usize = 256 * 1024;

/// Up to `limit` bytes from the start of the file at `path`.
pub(crate) fn read_head(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let mut head = Vec::with_capacity(limit.min(64 * 1024));
    File::open(path)?
        .take(limit as u64)
        .read_to_end(&mut head)?;
    Ok(head)
}

/// Where the World file at `path` keeps its day backup:
/// `<name>.world.day.bak` (see [`DAY_BACKUP_SUFFIX`]).
pub fn day_backup_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    name.push(DAY_BACKUP_SUFFIX);
    path.with_file_name(name)
}

/// Keeps the World file at `path` as its day backup when the one there is
/// a day old or missing, and only if the file reads whole: a day backup is
/// always a World that opens. Nothing to keep is not an error, and a day
/// backup that cannot be kept never stops a save.
pub(crate) fn keep_day_backup(path: &Path) {
    let day = day_backup_path(path);
    let fresh = fs::metadata(&day)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age < DAY_BACKUP_AGE);
    if fresh || read_file(path).is_err() {
        return;
    }
    let file_name = day
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = unique_temp_path(&day, &file_name);
    // A copy, not a link: its time is when it was kept, so it is replaced
    // a day later, not a day after the file was last written.
    if fs::copy(path, &temp).is_ok() {
        let _ = fs::rename(&temp, &day);
    }
    let _ = fs::remove_file(&temp);
}

/// Keeps the World file at `path`, as it is, as its backup (see
/// [`backup_path`]), replacing the one before. A link where the folder
/// allows one (nothing is copied, and the save that follows replaces the
/// file rather than changing it), else a copy. Nothing to keep is not an
/// error.
pub(crate) fn keep_backup(path: &Path) -> io::Result<()> {
    let backup = backup_path(path);
    let file_name = backup
        .file_name()
        .ok_or_else(|| io::Error::other("world document path has no file name"))?
        .to_string_lossy()
        .into_owned();
    // A name of its own, so two writers (two apps on one Worlds folder, or
    // Home and a World's writer) never take each other's half-made file.
    let temp = unique_temp_path(&backup, &file_name);
    match fs::hard_link(path, &temp) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(_) => match fs::copy(path, &temp) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        },
    }
    let kept = fs::rename(&temp, &backup);
    // Gone once renamed; still there if it failed, or if the backup already
    // was this very file (a rename onto itself does nothing).
    let _ = fs::remove_file(&temp);
    kept
}

/// `fs::rename`, tried again for a moment on Windows, where an antivirus
/// scanner or the search indexer holding the file briefly refuses it.
pub(crate) fn rename_patiently(from: &Path, to: &Path) -> io::Result<()> {
    let mut tries = 0;
    loop {
        match fs::rename(from, to) {
            Err(error)
                if cfg!(windows)
                    && error.kind() == io::ErrorKind::PermissionDenied
                    && tries < 10 =>
            {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            result => return result,
        }
    }
}

/// Makes a rename in the folder of `path` last through a power cut, where
/// the system needs the folder itself synced for that (Linux). Best effort:
/// the file itself is already synced.
pub(crate) fn sync_folder(path: &Path) {
    #[cfg(unix)]
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        if let Ok(folder) = File::open(parent) {
            let _ = folder.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// A temporary name beside `path` that no other writer, in this process or
/// another, is using: `.<file name>.<process>-<count>.tmp`.
pub(crate) fn unique_temp_path(path: &Path, file_name: &str) -> PathBuf {
    static COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let count = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // The time too: a crash leaves its temp file behind, and a later run
    // given the same process id must not trip over it.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.with_file_name(format!(
        ".{file_name}.{}-{nanos}-{count}.tmp",
        std::process::id()
    ))
}

/// Replaces the file at `path` whole with `bytes`: written beside it,
/// synced, then renamed over it, so it is never left half written. For a
/// file the player chose to replace; nothing of it is kept.
pub(crate) fn replace_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::other("the file has no name"))?
        .to_string_lossy()
        .into_owned();
    let temp_path = unique_temp_path(path, &file_name);
    let written = (|| {
        let mut file = File::create_new(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    written
}

#[cfg(test)]
thread_local! {
    /// Makes every World file written on this thread fail to write, as a
    /// full disk or a folder taken away would.
    pub(crate) static WRITES_FAIL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    #[cfg(test)]
    if WRITES_FAIL.get() {
        return Err(io::Error::other("the World file could not be written"));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::other("world document path has no file name"))?
        .to_string_lossy();
    let temp_path = unique_temp_path(path, &file_name);
    let written = (|| {
        let mut file = File::create_new(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        // The file as it was is kept before it is replaced.
        keep_backup(path)?;
        keep_day_backup(path);
        rename_patiently(&temp_path, path)?;
        sync_folder(path);
        Ok(())
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    written
}

/// Writes a World file just read to `destination` as a new file, never
/// over one that is there (even one that appeared meanwhile).
pub(crate) fn export_read_file(
    file: ReadFile,
    destination: &Path,
    replace: bool,
) -> Result<(), LibraryError> {
    let bytes = file.document.to_bytes()?;
    match revision::create_new_file(destination, &bytes) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists && replace => {
            // A World open somewhere keeps its file.
            let _lock = Lock::take(destination)?;
            replace_file(destination, &bytes)?;
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(
            LibraryError::ExportDestinationExists(destination.to_path_buf()),
        ),
        Err(error) => Err(LibraryError::Io(error)),
    }
}
