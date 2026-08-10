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
        assert_eq!(
            path_from_open_url("file:///tmp/Shared%20World.world").unwrap(),
            PathBuf::from("/tmp/Shared World.world")
        );
    }

    #[test]
    fn accepts_plain_paths_for_non_bundle_launchers() {
        assert_eq!(
            path_from_open_url("/tmp/portable.world").unwrap(),
            PathBuf::from("/tmp/portable.world")
        );
    }

    #[test]
    fn rejects_non_file_urls() {
        assert!(path_from_open_url("https://example.com/demo.world").is_err());
    }
}
