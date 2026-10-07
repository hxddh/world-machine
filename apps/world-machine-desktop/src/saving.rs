//! What the app says when a World's changes cannot be written as its window
//! closes or the app quits.
//!
//! A World's turns are written away from the turn itself; the last of them
//! is written as its window closes and as the app quits. Before v0.27 a
//! write that failed there was let go of in silence (on close) or only
//! logged (on quit), and the player's last turns were lost without a word.
//! Now the window stays open, or the app keeps running, and asks.

use world_library::LibraryError;

/// The choices when a World's window cannot write its last turns.
pub const CLOSE_CHOICES: [&str; 3] = ["Save As…", "Close Without Saving", "Keep Open"];
/// [`CLOSE_CHOICES`]: write the World to a new file.
pub const SAVE_AS: usize = 0;
/// [`CLOSE_CHOICES`]: close anyway, losing what was not written.
pub const CLOSE_ANYWAY: usize = 1;

/// The choices when the app is asked to quit and some World's last turns
/// cannot be written.
pub const QUIT_CHOICES: [&str; 2] = ["Keep World Machine Open", "Quit Without Saving"];
/// [`QUIT_CHOICES`]: quit anyway.
pub const QUIT_ANYWAY: usize = 1;

/// The question and its detail when the World `name`'s window is closing
/// and its last turns could not be written.
pub fn close_failure(name: &str, error: &LibraryError) -> (String, String) {
    (
        format!("{name}: the latest turns could not be saved"),
        format!(
            "{}. {} Save As… writes the whole World to a new file; Close Without Saving loses the turns since it was last saved.",
            sentence(&error.to_string()),
            advice(error)
        ),
    )
}

/// The question and its detail when the app is quitting and `failures`
/// (each a World's name and why it could not be written) were not saved.
pub fn quit_failure(failures: &[(String, String)]) -> (String, String) {
    let message = match failures {
        [(name, _)] => format!("{name}: the latest turns could not be saved"),
        _ => format!(
            "{} Worlds could not save their latest turns",
            failures.len()
        ),
    };
    let detail = failures
        .iter()
        .map(|(name, why)| format!("{name}: {}.", sentence(why)))
        .collect::<Vec<_>>()
        .join("\n");
    (
        message,
        format!(
            "{detail}\nKeep World Machine open to use Save As… in that World's window; quitting now loses those turns."
        ),
    )
}

/// What the player can do about `error`, in a sentence.
fn advice(error: &LibraryError) -> &'static str {
    match error {
        LibraryError::DocumentChanged(_) => {
            "Something else changed the World's file while it was open."
        }
        LibraryError::InUse(_) => "Another World Machine has this World open.",
        LibraryError::Io(io) if io.kind() == std::io::ErrorKind::StorageFull => "The disk is full.",
        _ => "The World's file could not be written.",
    }
}

/// `text` as the start of a sentence, without a full stop of its own.
fn sentence(text: &str) -> String {
    let text = text.trim().trim_end_matches('.');
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "It could not be written".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn a_close_that_cannot_save_says_why_and_what_each_choice_does() {
        let full = LibraryError::Io(io::Error::new(io::ErrorKind::StorageFull, "no space left"));
        let (message, detail) = close_failure("Harbour", &full);
        assert_eq!(message, "Harbour: the latest turns could not be saved");
        assert!(
            detail.starts_with("No space left. The disk is full."),
            "{detail}"
        );
        assert!(detail.contains("Save As…") && detail.contains("Close Without Saving"));
        let changed = LibraryError::DocumentChanged(PathBuf::from("/w/Harbour.world"));
        let (_, detail) = close_failure("Harbour", &changed);
        assert!(detail.contains("Something else changed"), "{detail}");
        assert_eq!(CLOSE_CHOICES[SAVE_AS], "Save As…");
        assert_eq!(CLOSE_CHOICES[CLOSE_ANYWAY], "Close Without Saving");
    }

    #[test]
    fn a_quit_that_cannot_save_names_every_world() {
        let one = vec![("Harbour".to_string(), "no space left".to_string())];
        let (message, detail) = quit_failure(&one);
        assert_eq!(message, "Harbour: the latest turns could not be saved");
        assert!(detail.starts_with("Harbour: No space left."), "{detail}");
        let two = vec![
            ("Harbour".to_string(), "no space left".to_string()),
            ("Ares".to_string(), "".to_string()),
        ];
        let (message, detail) = quit_failure(&two);
        assert_eq!(message, "2 Worlds could not save their latest turns");
        assert!(
            detail.contains("Ares: It could not be written."),
            "{detail}"
        );
        assert_eq!(QUIT_CHOICES[QUIT_ANYWAY], "Quit Without Saving");
    }
}
