use gpui::Application;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use url::Url;

static OPEN_URLS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

fn queue() -> &'static Mutex<Vec<String>> {
    OPEN_URLS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn install(application: &Application) {
    application.on_open_urls(|urls| {
        if let Ok(mut pending) = queue().lock() {
            pending.extend(urls);
        }
    });
    // Windows and Linux hand a double-clicked file to a new launch on its
    // command line, not as an open event: those are opened the same way.
    let files = files_from_args(std::env::args_os().skip(1));
    if !files.is_empty() {
        if let Ok(mut pending) = queue().lock() {
            pending.extend(files);
        }
    }
}

/// The files named on a command line: every argument that is not an
/// option (macOS's own `-psn_…` included) and names a file.
pub fn files_from_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Vec<String> {
    args.into_iter()
        .filter_map(|arg| arg.into_string().ok())
        .filter(|arg| !arg.starts_with('-'))
        .filter(|arg| std::path::Path::new(arg).is_file())
        .collect()
}

pub fn drain_paths() -> Vec<Result<PathBuf, String>> {
    let urls = match queue().lock() {
        Ok(mut pending) => std::mem::take(&mut *pending),
        Err(_) => return vec![Err("system open-event queue is poisoned".into())],
    };
    urls.into_iter()
        .map(|url| path_from_open_url(&url))
        .collect()
}

pub fn path_from_open_url(value: &str) -> Result<PathBuf, String> {
    match Url::parse(value) {
        Ok(url) if url.scheme() == "file" => url
            .to_file_path()
            .map_err(|_| format!("could not convert file URL to a local path: {value}")),
        Ok(url) => Err(format!(
            "unsupported system open URL scheme: {}",
            url.scheme()
        )),
        Err(_) => Ok(PathBuf::from(value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_percent_encoded_file_urls() {
        // A file URL names a drive on Windows.
        let (url, path) = if cfg!(windows) {
            (
                "file:///C:/tmp/Shared%20World.world",
                r"C:\tmp\Shared World.world",
            )
        } else {
            (
                "file:///tmp/Shared%20World.world",
                "/tmp/Shared World.world",
            )
        };
        assert_eq!(path_from_open_url(url).unwrap(), PathBuf::from(path));
    }

    #[test]
    fn accepts_plain_paths_for_non_bundle_launchers() {
        assert_eq!(
            path_from_open_url("/tmp/portable.world").unwrap(),
            PathBuf::from("/tmp/portable.world")
        );
    }

    #[test]
    fn a_command_line_names_only_existing_files() {
        let dir = std::env::temp_dir().join(format!("wm-args-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Meadow.world");
        std::fs::write(&file, b"x").unwrap();
        let found = files_from_args([
            std::ffi::OsString::from("-psn_0_12345"),
            file.clone().into_os_string(),
            dir.join("missing.world").into_os_string(),
        ]);
        assert_eq!(found, vec![file.to_string_lossy().into_owned()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_non_file_urls() {
        assert!(path_from_open_url("https://example.com/demo.world").is_err());
    }
}
