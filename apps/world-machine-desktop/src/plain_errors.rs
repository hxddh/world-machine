//! What the app says when something it asked the computer for goes wrong,
//! in the player's words: never a system's own error text ("ZBus Error:
//! Failed to connect…", v0.31's play), which goes to the diagnostics log
//! instead. Where the computer's own Save window cannot open (a Linux
//! desktop without its file portal), the file is saved anyway, in
//! Documents › World Machine, and the message says where.
//!
//! Every line here is written through `world_gpui::i18n::fill`, so it is
//! shown in the app's language; the catalogs hold each one.

use std::path::{Path, PathBuf};

/// The folder a file goes to when the Save window cannot open: World
/// Machine's own folder in `home`'s Documents, made if need be.
pub fn fallback_folder_in(home: &Path) -> std::io::Result<PathBuf> {
    let folder = home.join("Documents").join("World Machine");
    std::fs::create_dir_all(&folder)?;
    Ok(folder)
}

/// [`fallback_folder_in`] the player's home folder.
pub fn fallback_folder() -> std::io::Result<PathBuf> {
    // Windows names the home folder USERPROFILE.
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no home folder"))?;
    fallback_folder_in(Path::new(&home))
}

/// A path in `folder` for `file_name` that nothing is at yet: the name
/// itself, else the name with " 2", " 3"… before its extension.
pub fn free_path(folder: &Path, file_name: &str) -> PathBuf {
    let plain = folder.join(file_name);
    if !plain.exists() {
        return plain;
    }
    let (stem, extension) = match file_name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, format!(".{extension}")),
        _ => (file_name, String::new()),
    };
    (2..)
        .map(|n| folder.join(format!("{stem} {n}{extension}")))
        .find(|path| !path.exists())
        .unwrap_or(plain)
}

/// Saves what `write` writes to a free path for `file_name` in the
/// fallback folder (in `home`, or the player's own): where it went.
pub fn save_without_dialog(
    home: Option<&Path>,
    file_name: &str,
    write: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<PathBuf, String> {
    let folder = match home {
        Some(home) => fallback_folder_in(home),
        None => fallback_folder(),
    }
    .map_err(|error| error.to_string())?;
    let path = free_path(&folder, file_name);
    write(&path)?;
    Ok(path)
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

/// The Save window did not open, and the file was saved at `path`
/// instead.
pub fn saved_without_dialog(path: &Path) -> String {
    world_gpui::i18n::fill(
        "The Save window didn't open, so it was saved in Documents › World Machine as {file}.",
        &[("file", &file_name_of(path))],
    )
}

/// The Save window did not open, and saving in Documents failed too.
pub fn not_saved_without_dialog() -> String {
    world_gpui::i18n::fill(
        "The Save window didn't open, and it couldn't be saved in Documents either. Nothing was saved.",
        &[],
    )
}

/// The computer's window for choosing a file did not open.
pub fn no_file_window() -> String {
    world_gpui::i18n::fill(
        "Your computer's window for choosing a file didn't open. Nothing was opened.",
        &[],
    )
}

/// The window for choosing a file or a place closed before anything was
/// chosen in it.
pub fn file_window_closed() -> String {
    world_gpui::i18n::fill("The window closed before anything was chosen.", &[])
}

/// Why something could not be written or read, in a sentence, from the
/// system's own words for it: whether the disk is full, the folder is
/// closed to the app, or the file has gone; anything else is said plainly.
pub fn why(error: &str) -> String {
    let error = error.to_lowercase();
    let line = if error.contains("no space") || error.contains("storage full") {
        "The disk is full."
    } else if error.contains("permission denied") || error.contains("access is denied") {
        "World Machine isn't allowed to write there."
    } else if error.contains("no such file") || error.contains("not found") {
        "The file isn't there any more."
    } else {
        "Something on this computer got in the way."
    };
    world_gpui::i18n::fill(line, &[])
}

/// `what` could not be done, and why, in the player's words: "Couldn't
/// save the World code. The disk is full."
pub fn could_not(what: &str, error: &str) -> String {
    format!("{} {}", world_gpui::i18n::fill(what, &[]), why(error))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZBUS: &str = "ZBus Error: Failed to connect to socket /run/user/0/bus: No such file or directory (os error 2)";

    fn home(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "world-machine-plain-errors-{}-{nonce}-{label}",
            std::process::id()
        ))
    }

    /// v0.31 play: "Could not open the Save dialog: ZBus Error: …" in the
    /// title bar. Without the Save window the file is saved in Documents,
    /// and the message says where in words, with no system text in it.
    #[test]
    fn without_a_save_window_it_is_saved_in_documents_and_says_where() {
        let home = home("fallback");
        let saved = save_without_dialog(Some(&home), "Meadow.worldcode", |path| {
            std::fs::write(path, "wm2:code").map_err(|error| error.to_string())
        })
        .expect("saved without the dialog");
        assert_eq!(saved, home.join("Documents/World Machine/Meadow.worldcode"));
        assert_eq!(std::fs::read_to_string(&saved).unwrap(), "wm2:code");
        // A second one never writes over the first.
        let again = save_without_dialog(Some(&home), "Meadow.worldcode", |path| {
            std::fs::write(path, "wm2:later").map_err(|error| error.to_string())
        })
        .unwrap();
        assert_eq!(again.file_name().unwrap(), "Meadow 2.worldcode");
        let said = saved_without_dialog(&saved);
        assert_eq!(
            said,
            "The Save window didn't open, so it was saved in Documents › World Machine as Meadow.worldcode."
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn no_system_words_reach_the_player() {
        for said in [
            could_not("Couldn't save the World code.", ZBUS),
            could_not(
                "Couldn't save the World code.",
                "No space left on device (os error 28)",
            ),
            could_not(
                "Couldn't export this World.",
                "Permission denied (os error 13)",
            ),
            no_file_window(),
            file_window_closed(),
            not_saved_without_dialog(),
        ] {
            for raw in ["ZBus", "os error", "socket", "Error:", "/run/"] {
                assert!(!said.contains(raw), "{said}");
            }
            assert!(said.ends_with('.'), "{said}");
        }
        assert_eq!(
            could_not("Couldn't save the World code.", "No space left on device"),
            "Couldn't save the World code. The disk is full."
        );
    }

    /// Each line is in the app's Chinese and Japanese catalogs.
    #[test]
    fn every_line_is_in_chinese_and_japanese() {
        let lines = [
            "The Save window didn't open, so it was saved in Documents › World Machine as {file}.",
            "The Save window didn't open, and it couldn't be saved in Documents either. Nothing was saved.",
            "Your computer's window for choosing a file didn't open. Nothing was opened.",
            "The window closed before anything was chosen.",
            "The disk is full.",
            "World Machine isn't allowed to write there.",
            "The file isn't there any more.",
            "Something on this computer got in the way.",
            "Couldn't save the World code.",
            "Couldn't save this World.",
            "Couldn't export this World.",
            "Couldn't open that World file.",
        ];
        for (language, catalog) in [
            ("zh", world_gpui::i18n::APP_ZH_HANS),
            ("ja", world_gpui::i18n::APP_JA),
        ] {
            let keys = catalog
                .lines()
                .filter_map(|line| line.split_once('\t'))
                .map(|(english, _)| english)
                .collect::<std::collections::BTreeSet<_>>();
            for line in lines {
                assert!(keys.contains(line), "{language}: {line}");
            }
        }
    }
}
