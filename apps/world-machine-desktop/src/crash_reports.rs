//! Crash reports, kept on this computer and offered, never sent.
//!
//! A panic writes a report file into the crash folder (`diagnostics.rs`
//! installs [`hook`]). The next launch finds the reports not yet offered
//! ([`pending`]) and, unless the player chose never to be asked, offers
//! them once: the player may send it ([`sending`]: a new e-mail to support
//! with the report in it, which only opens their mail program, or, while
//! there is no support address, the report copied for a new issue on
//! GitHub), copy it, or show the file. Nothing here touches the network,
//! and nothing is sent without the player sending it themselves.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};

/// The file in the crash folder naming the reports already offered.
pub const OFFERED_FILE_NAME: &str = "offered.txt";

/// How much of a report goes into an e-mail's body: mail programs refuse
/// very long `mailto:` links, and the whole report is a file to attach.
pub const MAIL_BODY_LIMIT: usize = 1500;

/// Where a panic happened and what it said.
pub fn panic_details(panic: &PanicHookInfo<'_>) -> (String, String) {
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
    (location, message)
}

/// A panic hook that writes a report into `dir`: `report` makes its text
/// from where the panic happened and what it said, and `name` the file's
/// name. The path written is handed to `written`.
pub fn hook(
    dir: PathBuf,
    name: impl Fn() -> String + Send + Sync + 'static,
    report: impl Fn(&str, &str) -> String + Send + Sync + 'static,
    written: impl Fn(&str, &str, Option<&Path>) + Send + Sync + 'static,
) -> impl Fn(&PanicHookInfo<'_>) + Send + Sync + 'static {
    move |panic| {
        let (location, message) = panic_details(panic);
        let path = fs::create_dir_all(&dir)
            .ok()
            .and_then(|_| write_new(&dir, &name(), report(&location, &message).as_bytes()).ok());
        written(&location, &message, path.as_deref());
    }
}

/// Writes `bytes` to a new file in `dir` named `name`, or, when a file of
/// that name is already there (two crashes in one second), `name` with
/// `-2`, `-3`… before its extension: an earlier report is never overwritten.
pub fn write_new(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    let (stem, extension) = name.rsplit_once('.').unwrap_or((name, ""));
    for attempt in 1..=1000u32 {
        let candidate = match (attempt, extension) {
            (1, _) => name.to_string(),
            (_, "") => format!("{stem}-{attempt}"),
            (_, extension) => format!("{stem}-{attempt}.{extension}"),
        };
        let path = dir.join(candidate);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(bytes)?;
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "every crash report name is taken",
    ))
}

/// A crash report found on launch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrashReport {
    pub path: PathBuf,
    pub text: String,
}

impl CrashReport {
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

fn is_report(name: &str) -> bool {
    name.starts_with("world-machine-") && name.ends_with(".log")
}

fn offered(dir: &Path) -> Vec<String> {
    fs::read_to_string(dir.join(OFFERED_FILE_NAME))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// The app's crash reports in `dir` that have not been offered yet, oldest
/// first. A Pack's reports (in `Packs`) are not the app's.
pub fn pending(dir: &Path) -> Vec<CrashReport> {
    let offered = offered(dir);
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<CrashReport> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| is_report(name) && !offered.iter().any(|seen| seen == name))
        })
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            Some(CrashReport { path, text })
        })
        .collect();
    // Oldest first: by when each was written, and within one second by
    // name, a second report (`-2`) after the first.
    found.sort_by_cached_key(|report| {
        let name = report.name();
        let written = fs::metadata(&report.path)
            .and_then(|meta| meta.modified())
            .ok();
        (written, name.len(), name)
    });
    found
}

/// Records that `reports` were offered, so they are not offered again.
pub fn mark_offered(dir: &Path, reports: &[CrashReport]) -> io::Result<()> {
    if reports.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(dir)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(OFFERED_FILE_NAME))?;
    for report in reports {
        writeln!(file, "{}", report.name())?;
    }
    Ok(())
}

/// What "Send Report…" does with a report, by how support is reached
/// (`shell::Support`). Neither sends anything: the player does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Sending {
    /// Open this `mailto:` link in the player's own mail program.
    Mail(String),
    /// Put this text on the clipboard and open a new issue on GitHub
    /// (`shell::ISSUES_URL`) for the player to paste it into.
    Issue(String),
}

/// How "Send Report…" sends `report` when support is `support`: a new
/// e-mail to the support address, or, with no address, the report copied
/// for a new GitHub issue.
pub fn sending(support: crate::shell::Support, report: &CrashReport) -> Sending {
    match support.email() {
        Some(address) => Sending::Mail(mailto(address, report)),
        None => Sending::Issue(issue_text(report)),
    }
}

/// The report as it is pasted into a new issue: whole, with where its
/// file is.
pub fn issue_text(report: &CrashReport) -> String {
    format!(
        "World Machine crashed. The report it saved is below.\n\n```\n{}\n```\n\nFile: {}\n",
        report.text.trim(),
        report.path.display()
    )
}

/// A `mailto:` link to `address` with the report in its body (cut short
/// when long, saying where the whole file is). Opening it shows a new
/// e-mail in the player's own mail program; nothing is sent until they
/// send it.
pub fn mailto(address: &str, report: &CrashReport) -> String {
    let mut body = String::from(
        "World Machine crashed. The report it saved is below (the whole file is attached, or at the path given).\n\n",
    );
    let text = report.text.trim();
    if text.len() > MAIL_BODY_LIMIT {
        let mut end = MAIL_BODY_LIMIT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        body.push_str(&text[..end]);
        body.push_str("\n…\n");
    } else {
        body.push_str(text);
        body.push('\n');
    }
    body.push_str(&format!("\nFile: {}\n", report.path.display()));
    format!(
        "mailto:{address}?subject={}&body={}",
        encode("World Machine crash report"),
        encode(&body)
    )
}

/// Percent-encodes everything but the unreserved characters.
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "world-machine-crash-reports-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reports_are_offered_once_and_a_packs_are_not_the_apps() {
        let dir = temp_dir("offered");
        fs::create_dir_all(dir.join("Packs")).unwrap();
        fs::write(dir.join("Packs/world-machine-1-1.log"), "a Pack's").unwrap();
        fs::write(dir.join("notes.txt"), "not a report").unwrap();
        let first = write_new(&dir, "world-machine-1-2.log", b"first").unwrap();
        write_new(&dir, "world-machine-1-2.log", b"second").unwrap();
        let found = pending(&dir);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(found[0].path, first);
        assert_eq!(found[0].text, "first");
        mark_offered(&dir, &found[..1]).unwrap();
        let left = pending(&dir);
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].text, "second");
        mark_offered(&dir, &left).unwrap();
        assert!(pending(&dir).is_empty());
        // A missing folder has nothing to offer.
        assert!(pending(&dir.join("nowhere")).is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn the_mail_is_addressed_to_support_and_cut_short() {
        let report = CrashReport {
            path: PathBuf::from("/logs/Crashes/world-machine-1-2.log"),
            text: format!("Message: boom & bust\n{}", "x".repeat(4000)),
        };
        let link = mailto("support@example.invalid", &report);
        assert!(link.starts_with("mailto:support@example.invalid?subject=World%20Machine"));
        assert!(link.contains("boom%20%26%20bust"));
        assert!(link.contains("world-machine-1-2.log"));
        assert!(link.len() < 6000, "{}", link.len());
        assert!(!link.contains(' ') && !link.contains('\n'));
    }
}
