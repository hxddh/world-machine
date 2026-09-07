//! Local diagnostics: a rolling log file, a panic hook, and a copyable
//! diagnostics report.
//!
//! World Machine sends nothing anywhere. The log lives under
//! `~/Library/Logs/World Machine/` (or `WORLD_MACHINE_LOG_DIR`), rotates once
//! at 1 MiB, and only records what the app itself observed: startup facts,
//! status messages the user also saw, and panics. The report combines the
//! build identity, the host, the paths in use, and the tail of the log so a
//! bug report can be pasted in one step.

use std::env;
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::build_info;

pub const LOG_DIR_OVERRIDE_ENV: &str = "WORLD_MACHINE_LOG_DIR";
pub const LOG_FILE_NAME: &str = "world-machine.log";
pub const ROTATED_LOG_FILE_NAME: &str = "world-machine.log.1";
pub const ROTATE_AT_BYTES: u64 = 1024 * 1024;
const REPORT_TAIL_LINES: usize = 60;

pub const ISSUE_URL: &str =
    "https://github.com/hxddh/world-machine/issues/new?template=bug_report.yml";
pub const INSTALL_GUIDE_URL: &str =
    "https://github.com/hxddh/world-machine/blob/main/docs/INSTALL.md";
pub const RELEASES_URL: &str = "https://github.com/hxddh/world-machine/releases";

struct LogSink {
    path: PathBuf,
    file: Mutex<File>,
}

static SINK: OnceLock<Option<LogSink>> = OnceLock::new();

/// Where the log directory lives: the override, else Apple's per-user log
/// folder so Console.app and Finder both find it.
pub fn log_dir() -> Option<PathBuf> {
    if let Some(dir) = env::var_os(LOG_DIR_OVERRIDE_ENV) {
        return Some(PathBuf::from(dir));
    }
    let home = env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join("World Machine"),
    )
}

/// The log file path once the sink is open. `None` when logging could not
/// start; the app keeps running without it.
pub fn log_path() -> Option<&'static Path> {
    SINK.get()
        .and_then(|sink| sink.as_ref())
        .map(|sink| sink.path.as_path())
}

/// Opens the log (rotating first if it is large), writes the startup banner,
/// and installs a panic hook that records the panic before the default hook
/// prints it. Safe to call once; later calls are no-ops.
pub fn init() -> Option<PathBuf> {
    let sink = SINK.get_or_init(|| {
        let dir = log_dir()?;
        match open_sink(&dir) {
            Ok(sink) => Some(sink),
            Err(error) => {
                eprintln!(
                    "world-machine: diagnostics log unavailable at {}: {error}",
                    dir.display()
                );
                None
            }
        }
    });
    let path = sink.as_ref()?.path.clone();

    info(format!(
        "startup · {} · {}",
        build_info::display_label(),
        host_description()
    ));

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let location = panic
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let message = panic
            .payload()
            .downcast_ref::<&str>()
            .map(|message| message.to_string())
            .or_else(|| panic.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic payload".to_string());
        error(format!("panic at {location}: {message}"));
        previous(panic);
    }));

    Some(path)
}

fn open_sink(dir: &Path) -> io::Result<LogSink> {
    fs::create_dir_all(dir)?;
    let path = dir.join(LOG_FILE_NAME);
    rotate_if_needed(&path, ROTATE_AT_BYTES)?;
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok(LogSink {
        path,
        file: Mutex::new(file),
    })
}

/// Renames the log to its `.1` sibling once it reaches `limit` bytes. One
/// generation is enough: the tail of the current file is what a report needs.
pub fn rotate_if_needed(path: &Path, limit: u64) -> io::Result<()> {
    let size = match fs::metadata(path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if size < limit {
        return Ok(());
    }
    let rotated = path.with_file_name(ROTATED_LOG_FILE_NAME);
    fs::rename(path, rotated)
}

pub fn info(message: impl AsRef<str>) {
    write_line("INFO", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    write_line("ERROR", message.as_ref());
}

fn write_line(level: &str, message: &str) {
    let Some(Some(sink)) = SINK.get() else {
        return;
    };
    let line = format_line(SystemTime::now(), level, message);
    if let Ok(mut file) = sink.file.lock() {
        let _ = file.write_all(line.as_bytes());
    }
}

fn format_line(at: SystemTime, level: &str, message: &str) -> String {
    let mut line = String::with_capacity(message.len() + 40);
    let _ = write!(line, "{} {level:<5} ", timestamp(at));
    for (index, part) in message.lines().enumerate() {
        if index > 0 {
            line.push_str("\n    ");
        }
        line.push_str(part);
    }
    line.push('\n');
    line
}

/// UTC timestamp without a date/time dependency: `2026-09-07T10:32:05Z`.
pub fn timestamp(at: SystemTime) -> String {
    let seconds = at
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let days = seconds.div_euclid(86_400);
    let remainder = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        remainder / 3600,
        (remainder % 3600) / 60,
        remainder % 60
    )
}

// Howard Hinnant's days-to-civil algorithm; exact for the proleptic Gregorian calendar.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// `macOS 15.1 (24B83)` from SystemVersion.plist, or a fallback that still
/// names the platform.
pub fn host_description() -> String {
    let plist =
        fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist").unwrap_or_default();
    describe_host(&plist, env::consts::OS, env::consts::ARCH)
}

fn describe_host(system_version_plist: &str, os: &str, arch: &str) -> String {
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

/// Facts the About window shows and the report includes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    pub library_dir: Option<PathBuf>,
    pub pack_catalog_path: Option<PathBuf>,
    pub included_packs: Vec<String>,
}

static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();

pub fn record_environment(environment: Environment) {
    let _ = ENVIRONMENT.set(environment);
}

pub fn environment() -> Environment {
    ENVIRONMENT.get().cloned().unwrap_or_default()
}

/// The text behind "Copy diagnostics".
pub fn report() -> String {
    let tail = log_path()
        .map(|path| log_tail(path, REPORT_TAIL_LINES))
        .unwrap_or_default();
    render_report(
        &build_info::display_label(),
        &host_description(),
        &environment(),
        log_path(),
        &tail,
    )
}

fn render_report(
    build_label: &str,
    host: &str,
    environment: &Environment,
    log_path: Option<&Path>,
    log_tail: &str,
) -> String {
    let mut report = String::new();
    let _ = writeln!(report, "World Machine diagnostics");
    let _ = writeln!(report, "Build: {build_label}");
    let _ = writeln!(report, "Signing: ad-hoc, not notarized");
    let _ = writeln!(report, "Host: {host}");
    let _ = writeln!(
        report,
        "Worlds: {}",
        environment
            .library_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    let _ = writeln!(
        report,
        "Packs catalog: {}",
        environment
            .pack_catalog_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    let _ = writeln!(
        report,
        "Included Packs: {}",
        if environment.included_packs.is_empty() {
            "none found".to_string()
        } else {
            environment.included_packs.join(", ")
        }
    );
    let _ = writeln!(
        report,
        "Log: {}",
        log_path
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".to_string())
    );
    let _ = writeln!(report);
    let _ = writeln!(report, "Recent log lines:");
    if log_tail.trim().is_empty() {
        let _ = writeln!(report, "(none)");
    } else {
        report.push_str(log_tail);
        if !log_tail.ends_with('\n') {
            report.push('\n');
        }
    }
    report
}

fn log_tail(path: &Path, lines: usize) -> String {
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let len = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
    let window = 64 * 1024;
    let start = len.saturating_sub(window);
    if file.seek(SeekFrom::Start(start)).is_err() {
        return String::new();
    }
    let mut buffer = Vec::new();
    if file.read_to_end(&mut buffer).is_err() {
        return String::new();
    }
    let text = String::from_utf8_lossy(&buffer);
    let collected: Vec<&str> = text.lines().collect();
    let skip = collected.len().saturating_sub(lines);
    let mut tail = collected[skip..].join("\n");
    if !tail.is_empty() {
        tail.push('\n');
    }
    tail
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "world-machine-diagnostics-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn timestamps_are_utc_iso_8601() {
        let at = UNIX_EPOCH + Duration::from_secs(1_788_000_000);
        assert_eq!(timestamp(at), "2026-08-29T10:40:00Z");
        assert_eq!(timestamp(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        let leap = UNIX_EPOCH + Duration::from_secs(951_782_400);
        assert_eq!(timestamp(leap), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn log_lines_indent_continuations_under_one_timestamp() {
        let at = UNIX_EPOCH + Duration::from_secs(0);
        assert_eq!(
            format_line(at, "ERROR", "first\nsecond"),
            "1970-01-01T00:00:00Z ERROR first\n    second\n"
        );
        assert_eq!(
            format_line(at, "INFO", "one"),
            "1970-01-01T00:00:00Z INFO  one\n"
        );
    }

    #[test]
    fn rotation_moves_a_full_log_aside_and_leaves_small_logs_alone() {
        let dir = temp_dir("rotate");
        let path = dir.join(LOG_FILE_NAME);
        fs::write(&path, "small\n").unwrap();
        rotate_if_needed(&path, 100).unwrap();
        assert!(path.exists());
        assert!(!dir.join(ROTATED_LOG_FILE_NAME).exists());

        fs::write(&path, vec![b'x'; 100]).unwrap();
        rotate_if_needed(&path, 100).unwrap();
        assert!(!path.exists());
        assert_eq!(
            fs::metadata(dir.join(ROTATED_LOG_FILE_NAME)).unwrap().len(),
            100
        );

        rotate_if_needed(&dir.join("missing.log"), 1).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn host_description_reads_system_version_plist() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>ProductBuildVersion</key>
    <string>24B83</string>
    <key>ProductName</key>
    <string>macOS</string>
    <key>ProductVersion</key>
    <string>15.1</string>
</dict>
</plist>"#;
        assert_eq!(
            describe_host(plist, "macos", "aarch64"),
            "macOS 15.1 (24B83) · aarch64"
        );
        assert_eq!(describe_host("", "linux", "x86_64"), "linux · x86_64");
    }

    #[test]
    fn report_lists_build_paths_and_the_log_tail() {
        let dir = temp_dir("report");
        let log = dir.join(LOG_FILE_NAME);
        let mut lines = String::new();
        for index in 0..100 {
            let _ = writeln!(lines, "line {index}");
        }
        fs::write(&log, &lines).unwrap();

        let environment = Environment {
            library_dir: Some(PathBuf::from("/tmp/Worlds")),
            pack_catalog_path: Some(PathBuf::from("/tmp/Packs/catalog.json")),
            included_packs: vec!["pocket-universe 0.16.0".into()],
        };
        let report = render_report(
            "Pre-alpha 0.1.0 · build abc · aarch64",
            "macOS 15.1 (24B83) · aarch64",
            &environment,
            Some(&log),
            &log_tail(&log, 3),
        );
        assert!(report.contains("Build: Pre-alpha 0.1.0 · build abc · aarch64"));
        assert!(report.contains("Signing: ad-hoc, not notarized"));
        assert!(report.contains("Worlds: /tmp/Worlds"));
        assert!(report.contains("Included Packs: pocket-universe 0.16.0"));
        assert!(report.ends_with("line 97\nline 98\nline 99\n"));
        assert!(!report.contains("line 96\n"));

        let empty = render_report("b", "h", &Environment::default(), None, "");
        assert!(empty.contains("Log: unavailable"));
        assert!(empty.contains("Included Packs: none found"));
        assert!(empty.ends_with("(none)\n"));
        fs::remove_dir_all(dir).unwrap();
    }
}
