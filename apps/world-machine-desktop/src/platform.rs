//! What the app asks of the operating system, in one place.
//!
//! Everything else in the app is the same on every platform: the window is
//! GPUI's (`cfg(gui)`, see `build.rs`), sound goes out through `cpal` (Core
//! Audio on the Mac, WASAPI on Windows), and opening a link or revealing a
//! folder is GPUI's `open_url` and `reveal_path`. What is left differs from
//! one operating system to the next, and goes through [`Platform`]:
//!
//! - where the app keeps its settings and Worlds, and its log;
//! - one small HTTPS request (the update check), made with the `curl` the
//!   operating system ships, so the app carries no HTTP client;
//! - where the API key is kept (the login keychain on the Mac);
//! - whether the operating system asks for more contrast;
//! - how the machine is described in a diagnostics report.
//!
//! [`current`] is the running platform. On the Mac it is exactly what the
//! app did before the split. On Windows it keeps files under `%APPDATA%`,
//! uses Windows' own `curl.exe`, and has no key store yet (keeping a key
//! says so, and the voice reads as not configured). Linux builds the window
//! only for previews and the screenshot harness, so it keeps the Mac's
//! layout under `$HOME`.

use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// One HTTPS GET, as the update check makes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fetch<'a> {
    pub url: &'a str,
    pub headers: &'a [String],
    pub timeout: Duration,
}

/// What differs between operating systems. Small on purpose: anything GPUI
/// or `cpal` already does on every platform stays out of it.
pub trait Platform: Sync {
    /// A short name for logs and diagnostics.
    fn name(&self) -> &'static str;

    /// The folder the app keeps its settings, window state and Worlds in.
    fn support_dir(&self) -> Option<PathBuf>;

    /// The folder the log is written to.
    fn log_dir(&self) -> Option<PathBuf>;

    /// The body of a successful GET, or `None` for every kind of failure: a
    /// failed or slow request is silent, never an error to show.
    fn fetch(&self, request: &Fetch<'_>) -> Option<String>;

    /// Keep the API key, replacing any key already kept. The key has
    /// already been checked to be a plausible key.
    fn save_secret(&self, key: &str) -> Result<(), String>;

    /// The kept key, if there is one this app can read right now.
    fn load_secret(&self) -> Option<String>;

    /// Forget the key. Forgetting one that is not there is not a failure.
    fn clear_secret(&self) -> Result<(), String>;

    /// Whether the operating system asks for more contrast.
    fn increase_contrast(&self) -> bool;

    /// The operating system and machine, for a diagnostics report.
    fn host_description(&self) -> String;
}

/// The running platform.
pub fn current() -> &'static dyn Platform {
    #[cfg(target_os = "macos")]
    {
        &Mac
    }
    #[cfg(windows)]
    {
        &Windows
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        &OtherUnix
    }
}

/// The folder name the app's files go in, on every platform.
pub const APP_FOLDER: &str = "World Machine";

#[cfg(any(not(windows), test))]
fn home() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from)
}

#[cfg(any(not(windows), test))]
/// `~/Library/Application Support/World Machine`.
fn mac_support_dir() -> Option<PathBuf> {
    Some(
        home()?
            .join("Library")
            .join("Application Support")
            .join(APP_FOLDER),
    )
}

#[cfg(any(not(windows), test))]
/// `~/Library/Logs/World Machine`, where Console.app and Finder look.
fn mac_log_dir() -> Option<PathBuf> {
    Some(home()?.join("Library").join("Logs").join(APP_FOLDER))
}

/// The arguments `curl` is run with: fail on an HTTP error, no progress,
/// follow redirects, give up after the time limit.
pub fn curl_args(request: &Fetch<'_>) -> Vec<String> {
    let mut args = vec![
        "-fsSL".to_string(),
        "--max-time".to_string(),
        request.timeout.as_secs().max(1).to_string(),
    ];
    for header in request.headers {
        args.push("-H".to_string());
        args.push(header.clone());
    }
    args.push(request.url.to_string());
    args
}

fn curl(program: &std::path::Path, request: &Fetch<'_>) -> Option<String> {
    let output = Command::new(program)
        .args(curl_args(request))
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// `macOS 15.1 (24B83) · aarch64` from the Mac's SystemVersion.plist, or a
/// fallback that still names the platform.
pub fn describe_host(system_version_plist: &str, os: &str, arch: &str) -> String {
    let version = plist_string(system_version_plist, "ProductVersion");
    let build = plist_string(system_version_plist, "ProductBuildVersion");
    match (version, build) {
        (Some(version), Some(build)) => format!("macOS {version} ({build}) · {arch}"),
        (Some(version), None) => format!("macOS {version} · {arch}"),
        _ => format!("{os} · {arch}"),
    }
}

fn plist_string(plist: &str, key: &str) -> Option<String> {
    let marker = format!("<key>{key}</key>");
    let after_key = &plist[plist.find(&marker)? + marker.len()..];
    let start = after_key.find("<string>")? + "<string>".len();
    let end = after_key[start..].find("</string>")? + start;
    let value = after_key[start..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// The Mac: the app as it has always been.
#[cfg(any(target_os = "macos", test))]
pub struct Mac;

#[cfg(any(target_os = "macos", test))]
impl Platform for Mac {
    fn name(&self) -> &'static str {
        "macos"
    }

    fn support_dir(&self) -> Option<PathBuf> {
        mac_support_dir()
    }

    fn log_dir(&self) -> Option<PathBuf> {
        mac_log_dir()
    }

    fn fetch(&self, request: &Fetch<'_>) -> Option<String> {
        curl(std::path::Path::new("/usr/bin/curl"), request)
    }

    fn save_secret(&self, key: &str) -> Result<(), String> {
        crate::key_store::keychain::save(key)
    }

    fn load_secret(&self) -> Option<String> {
        crate::key_store::keychain::load()
    }

    fn clear_secret(&self) -> Result<(), String> {
        crate::key_store::keychain::clear()
    }

    fn increase_contrast(&self) -> bool {
        Command::new("/usr/bin/defaults")
            .args(["read", "com.apple.universalaccess", "increaseContrast"])
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "1")
    }

    fn host_description(&self) -> String {
        describe_host(
            &std::fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist")
                .unwrap_or_default(),
            env::consts::OS,
            env::consts::ARCH,
        )
    }
}

/// Windows: files under `%APPDATA%`, the log under `%LOCALAPPDATA%`, and
/// the `curl.exe` Windows has shipped since Windows 10 1803.
#[cfg(any(windows, test))]
pub struct Windows;

#[cfg(any(windows, test))]
impl Windows {
    const NO_KEY_STORE: &'static str =
        "keeping an API key is not built for Windows yet; the World keeps its own words";

    fn known_folder(variable: &str) -> Option<PathBuf> {
        env::var_os(variable)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }

    fn curl_exe() -> PathBuf {
        Self::known_folder("SystemRoot")
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
            .join("System32")
            .join("curl.exe")
    }
}

#[cfg(any(windows, test))]
impl Platform for Windows {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn support_dir(&self) -> Option<PathBuf> {
        Some(Self::known_folder("APPDATA")?.join(APP_FOLDER))
    }

    fn log_dir(&self) -> Option<PathBuf> {
        Some(
            Self::known_folder("LOCALAPPDATA")
                .or_else(|| Self::known_folder("APPDATA"))?
                .join(APP_FOLDER)
                .join("Logs"),
        )
    }

    fn fetch(&self, request: &Fetch<'_>) -> Option<String> {
        curl(&Self::curl_exe(), request)
    }

    fn save_secret(&self, _key: &str) -> Result<(), String> {
        Err(Self::NO_KEY_STORE.to_string())
    }

    fn load_secret(&self) -> Option<String> {
        None
    }

    fn clear_secret(&self) -> Result<(), String> {
        Ok(())
    }

    fn increase_contrast(&self) -> bool {
        false
    }

    fn host_description(&self) -> String {
        format!("Windows · {}", env::consts::ARCH)
    }
}

/// Linux and the other unixes: the window is built only for previews and
/// the screenshot harness (`--features linux-window`), so it keeps the
/// Mac's layout under `$HOME`, which those scripts set up. There is no key
/// store.
#[cfg(not(any(target_os = "macos", windows)))]
pub struct OtherUnix;

#[cfg(not(any(target_os = "macos", windows)))]
impl Platform for OtherUnix {
    fn name(&self) -> &'static str {
        env::consts::OS
    }

    fn support_dir(&self) -> Option<PathBuf> {
        mac_support_dir()
    }

    fn log_dir(&self) -> Option<PathBuf> {
        mac_log_dir()
    }

    fn fetch(&self, request: &Fetch<'_>) -> Option<String> {
        curl(std::path::Path::new("curl"), request)
    }

    fn save_secret(&self, _key: &str) -> Result<(), String> {
        Err("there is no key store on this platform".to_string())
    }

    fn load_secret(&self) -> Option<String> {
        None
    }

    fn clear_secret(&self) -> Result<(), String> {
        Ok(())
    }

    fn increase_contrast(&self) -> bool {
        false
    }

    fn host_description(&self) -> String {
        format!("{} · {}", env::consts::OS, env::consts::ARCH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curl_is_given_the_time_limit_the_headers_and_the_url_last() {
        let headers = vec!["Accept: application/json".to_string()];
        let args = curl_args(&Fetch {
            url: "https://example.com/x",
            headers: &headers,
            timeout: Duration::from_secs(6),
        });
        assert_eq!(
            args,
            [
                "-fsSL",
                "--max-time",
                "6",
                "-H",
                "Accept: application/json",
                "https://example.com/x"
            ]
        );
    }

    #[test]
    fn every_platform_keeps_its_files_in_a_world_machine_folder() {
        // The Mac's layout is pinned: moving it would lose every player's
        // Worlds.
        if let Some(dir) = Mac.support_dir() {
            assert!(dir.ends_with("Library/Application Support/World Machine"));
        }
        if let Some(dir) = Mac.log_dir() {
            assert!(dir.ends_with("Library/Logs/World Machine"));
        }
        if let Some(dir) = Windows.support_dir() {
            assert!(dir.ends_with(APP_FOLDER));
        }
        assert!(Windows::curl_exe().ends_with("System32/curl.exe"));
    }

    #[test]
    fn windows_says_it_cannot_keep_a_key_rather_than_pretending() {
        assert!(Windows.save_secret("sk-ant-test").is_err());
        assert_eq!(Windows.load_secret(), None);
        assert!(Windows.clear_secret().is_ok());
    }
}
