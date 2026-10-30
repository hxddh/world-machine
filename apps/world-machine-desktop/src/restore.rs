//! Restore from the last good save: when a World's file fails to load, Home
//! offers to put it back to its backup (`<name>.world.bak`, the file as it
//! was before its last save), which `world_library::restore_backup` checks
//! reads as a World file first. The damaged file is kept beside it, never
//! deleted.
//!
//! A World the 1.0 save contract refuses (a preview's, or a newer World
//! Machine's) is not offered it: its backup is no better, and a newer
//! World's answer is to update, never to go back. Nor is a World open in
//! another window.

use std::path::{Path, PathBuf};
use world_library::{FormatAge, LibraryError};

/// What the action that restores the last good save says.
pub const RESTORE_LABEL: &str = "Restore the last good save";

/// Whether a World whose file at `path` failed to open with `error` may be
/// put back to its last good save: it has a backup that opens, and nothing
/// but the file itself is in the way.
pub fn offers_restore(error: &LibraryError, path: &Path) -> bool {
    !matches!(error, LibraryError::InUse(_))
        && error.save_age() == FormatAge::Current
        && world_library::has_good_backup(path)
}

/// Puts the World file at `path` back to its last good save. Where the
/// damaged file was kept, if there was one.
pub fn restore_last_good(path: &Path) -> Result<Option<PathBuf>, LibraryError> {
    world_library::restore_backup(path)
}

/// What Home says once a World was put back to its last good save.
pub fn restored_message(title: &str) -> String {
    format!("Restored {title} from its last good save. The damaged file is kept beside it.")
}
