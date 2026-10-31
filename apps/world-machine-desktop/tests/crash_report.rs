//! An opt-in crash report (1.0 P6): a panic writes a report on this
//! computer, the next launch offers it once, and nothing is sent without the
//! player choosing to. Its own test binary, because it installs a panic hook.

use std::fs;
use std::path::PathBuf;
use world_machine_desktop::app_settings::{AppSettings, CrashReports};
use world_machine_desktop::crash_reports;

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

    // Sending is a link for the player's own mail program, made only when
    // they ask for it: to support, with the report, and nothing else
    // happens until they send it there.
    let link = crash_reports::mailto(world_machine_desktop::shell::SUPPORT_EMAIL, &offered[0]);
    assert!(link.starts_with(&format!(
        "mailto:{}?",
        world_machine_desktop::shell::SUPPORT_EMAIL
    )));
    assert!(link.contains("harbour%20fell"));

    // Offered once: the launch after does not offer it again.
    crash_reports::mark_offered(&dir, &offered).unwrap();
    assert!(crash_reports::pending(&dir).is_empty());
    // The report itself is kept, for the player to find later.
    assert!(offered[0].path.exists());
    let _ = fs::remove_dir_all(dir.parent().unwrap());
}
