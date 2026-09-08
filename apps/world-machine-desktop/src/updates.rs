//! Update check against the GitHub Releases API.
//!
//! One request per launch to `api.github.com`, made with the `curl` that
//! ships with macOS so the app carries no HTTP client. The request sends
//! nothing about the user beyond what any HTTPS request does; the response
//! is the latest release's tag and page URL. `WORLD_MACHINE_NO_UPDATE_CHECK=1`
//! turns the check off, which the screenshot job and privacy-conscious users
//! rely on. A failed or slow check is silent: Home simply shows no banner.

use std::env;
use std::process::Command;
use std::time::Duration;

use crate::build_info;

pub const DISABLE_ENV: &str = "WORLD_MACHINE_NO_UPDATE_CHECK";
const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/hxddh/world-machine/releases/latest";
const TIMEOUT: Duration = Duration::from_secs(6);

/// A release newer than the running build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub version: String,
    pub url: String,
}

pub fn enabled() -> bool {
    env::var_os(DISABLE_ENV).is_none()
}

/// Blocking; run it on the background executor. `None` means "nothing newer
/// or could not tell", which the caller treats the same way.
pub fn check() -> Option<AvailableUpdate> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            &TIMEOUT.as_secs().to_string(),
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            &format!("User-Agent: world-machine/{}", build_info::APP_VERSION),
            LATEST_RELEASE_URL,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    newer_than_current(
        &String::from_utf8_lossy(&output.stdout),
        build_info::APP_VERSION,
    )
}

fn newer_than_current(body: &str, current: &str) -> Option<AvailableUpdate> {
    let release: serde_json::Value = serde_json::from_str(body).ok()?;
    let tag = release.get("tag_name")?.as_str()?;
    let url = release.get("html_url")?.as_str()?.to_string();
    if release.get("prerelease").and_then(|v| v.as_bool()) == Some(true) {
        return None;
    }
    let candidate = tag.strip_prefix('v').unwrap_or(tag);
    (parse_version(candidate)? > parse_version(current)?).then(|| AvailableUpdate {
        version: candidate.to_string(),
        url,
    })
}

/// `major.minor.patch`, ignoring any `-pre.N` suffix so a stable build never
/// offers a pre-release and a pre-release build sees its own stable version.
fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.split('-').next()?;
    let mut parts = core.split('.').map(|part| part.parse::<u64>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(tag: &str, prerelease: bool) -> String {
        format!(
            r#"{{"tag_name":"{tag}","prerelease":{prerelease},"html_url":"https://github.com/hxddh/world-machine/releases/tag/{tag}"}}"#
        )
    }

    #[test]
    fn newer_stable_release_is_offered() {
        let update = newer_than_current(&body("v0.3.0", false), "0.2.0").unwrap();
        assert_eq!(update.version, "0.3.0");
        assert!(update.url.ends_with("/tag/v0.3.0"));
    }

    #[test]
    fn same_or_older_release_is_not_offered() {
        assert_eq!(newer_than_current(&body("v0.2.0", false), "0.2.0"), None);
        assert_eq!(newer_than_current(&body("v0.1.0", false), "0.2.0"), None);
    }

    #[test]
    fn pre_releases_and_garbage_are_ignored() {
        assert_eq!(
            newer_than_current(&body("v0.9.0-pre.1", true), "0.2.0"),
            None
        );
        assert_eq!(newer_than_current("not json", "0.2.0"), None);
        assert_eq!(
            newer_than_current(r#"{"tag_name":"nightly"}"#, "0.2.0"),
            None
        );
    }

    #[test]
    fn version_parsing_drops_pre_release_suffixes() {
        assert_eq!(parse_version("0.2.0-pre.3"), Some((0, 2, 0)));
        assert_eq!(parse_version("1.10.2"), Some((1, 10, 2)));
        assert_eq!(parse_version("1.2"), None);
    }
}
