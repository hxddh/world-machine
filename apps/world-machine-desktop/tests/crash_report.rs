//! An opt-in crash report (1.0 P6): a panic writes a report on this
//! computer, the next launch offers it once, and nothing is sent without the
//! player choosing to. Its own test binary, because it installs a panic hook.

use std::fs;
use std::path::PathBuf;
use world_machine_desktop::app_settings::{AppSettings, CrashReports};
use world_machine_desktop::crash_reports::{self, Sending};
use world_machine_desktop::shell::{self, Support};

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "world-machine-crash-report-test-{}-{}",
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
fn a_panic_leaves_a_report_that_is_offered_once_and_never_sent() {
    let dir = temp_dir().join("Crashes");
    let written = std::sync::Arc::new(std::sync::Mutex::new(Vec::<PathBuf>::new()));
    let noted = written.clone();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(crash_reports::hook(
        dir.clone(),
        || "world-machine-1700000000-1.log".to_string(),
        |location, message| {
            format!("World Machine crashed\nPanicked at: {location}\nMessage: {message}\n")
        },
        move |_, _, path| {
            if let Some(path) = path {
                noted.lock().unwrap().push(path.to_path_buf());
            }
        },
    )));
    let outcome = std::panic::catch_unwind(|| panic!("the harbour fell into the sea"));
    std::panic::set_hook(previous);
    assert!(outcome.is_err());

    // The report is on this computer, and is all that was written.
    let written = written.lock().unwrap().clone();
    assert_eq!(written.len(), 1, "{written:?}");
    let files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(files, vec!["world-machine-1700000000-1.log".to_string()]);

    // The next launch finds it, while the player has not said never.
    let settings = AppSettings::empty();
    assert_eq!(
        settings.crash_reports,
        CrashReports::Ask,
        "asked by default"
    );
    let offered = crash_reports::pending(&dir);
    assert_eq!(offered.len(), 1);
    assert!(offered[0]
        .text
        .contains("Message: the harbour fell into the sea"));
    assert!(
        offered[0].text.contains("crash_report.rs"),
        "where it panicked"
    );

    // Sending is made only when the player asks for it, and nothing goes
    // until they send it themselves. With a support address it is a link
    // for their own mail program: to support, with the report.
    let address = "support@example.org";
    match crash_reports::sending(Support::from_address(Some(address)), &offered[0]) {
        Sending::Mail(link) => {
            assert!(link.starts_with(&format!("mailto:{address}?")), "{link}");
            assert!(link.contains("harbour%20fell"));
        }
        other => panic!("with an address, Send Report writes an e-mail: {other:?}"),
    }
    // With none, the report is copied whole for a new issue on GitHub,
    // which the window opens (shell::ISSUES_URL); no mail link is made.
    match crash_reports::sending(Support::from_address(None), &offered[0]) {
        Sending::Issue(text) => {
            assert!(text.contains("Message: the harbour fell into the sea"));
            assert!(text.contains(&offered[0].path.display().to_string()));
            assert!(!text.contains("mailto:"));
        }
        other => panic!("with no address, Send Report copies for an issue: {other:?}"),
    }
    assert!(shell::ISSUES_URL.starts_with("https://github.com/"));
    // The build as it ships takes one of the two, never a placeholder.
    let shipped = crash_reports::sending(Support::current(), &offered[0]);
    match shell::SUPPORT_EMAIL {
        Some(address) => {
            assert!(!address.ends_with(".invalid"), "{address}");
            assert!(matches!(shipped, Sending::Mail(_)));
        }
        None => assert!(matches!(shipped, Sending::Issue(_))),
    }

    // Offered once: the launch after does not offer it again.
    crash_reports::mark_offered(&dir, &offered).unwrap();
    assert!(crash_reports::pending(&dir).is_empty());
    // The report itself is kept, for the player to find later.
    assert!(offered[0].path.exists());
    let _ = fs::remove_dir_all(dir.parent().unwrap());
}
