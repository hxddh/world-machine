//! Words that arrive from outside (a model's answer, a World's hearing
//! handed to an app), held to plain text: the same rules as
//! `world_core::text`, kept here so asking a model links no World code
//! (the conversation System holds the two the same by a test).

/// Whether `c` changes how the text around it reads without being seen:
/// a control character (Unicode category Cc), a format character that
/// steers direction or hides text (most of category Cf), or a line or
/// paragraph separator (Zl, Zp).
///
/// The zero-width joiner and non-joiner (U+200D, U+200C) are kept: scripts
/// such as Persian and emoji sequences need them, and they cannot reorder
/// or hide anything.
pub fn is_hidden_control(c: char) -> bool {
    if c.is_control() {
        return true;
    }
    matches!(
        c as u32,
        0x00AD                  // soft hyphen
        | 0x0600..=0x0605       // Arabic number signs
        | 0x061C                // Arabic letter mark
        | 0x06DD | 0x070F | 0x0890..=0x0891 | 0x08E2
        | 0x180E                // Mongolian vowel separator
        | 0x200B                // zero-width space
        | 0x200E..=0x200F       // left-to-right and right-to-left marks
        | 0x2028..=0x2029       // line and paragraph separators
        | 0x202A..=0x202E       // embeddings and overrides
        | 0x2060..=0x2064       // word joiner and invisible operators
        | 0x2066..=0x206F       // isolates and deprecated format characters
        | 0xFEFF                // byte order mark
        | 0xFFF9..=0xFFFB       // interlinear annotation
        | 0x110BD | 0x110CD
        | 0x13430..=0x1343F
        | 0x1BCA0..=0x1BCA3
        | 0x1D173..=0x1D17A
        | 0xE0001 | 0xE0020..=0xE007F // tag characters
    )
}

/// `text` with every [hidden control](is_hidden_control) taken out, a
/// line break or tab read as a space, runs of spaces made one, and the
/// ends trimmed.
pub fn clean_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars() {
        let c = if matches!(c, '\n' | '\r' | '\t' | '\u{2028}' | '\u{2029}') {
            ' '
        } else {
            c
        };
        if is_hidden_control(c) {
            continue;
        }
        if c == ' ' {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.push(c);
    }
    out
}

/// Whether `text` needs no cleaning: [`clean_text`] would leave it as it
/// is.
pub fn is_clean_text(text: &str) -> bool {
    clean_text(text) == text
}

/// Whether `text` is something to record: at most `most` characters, and
/// none of them hidden.
pub fn plain(text: &str, most: usize) -> bool {
    let count = text.chars().count();
    count > 0 && count <= most && !text.chars().any(is_hidden_control)
}

/// Whether text that came from a model, or from an app about one, is clean
/// as it stands: [`plain`], and nothing [`clean_text`] would change.
pub fn clean(text: &str, most: usize) -> bool {
    plain(text, most) && is_clean_text(text)
}
