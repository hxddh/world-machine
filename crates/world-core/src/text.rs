//! Text that arrives from outside a World (a name typed by a player, a name
//! carried in a shared code, a guest's words), made safe to show and keep.
//!
//! Only characters that change how text around them is *read* are taken
//! out: control characters, bidirectional overrides, embeddings, isolates
//! and marks, invisible separators, and the tag characters that can hide
//! text inside other text. What remains is the text as its writer saw it.
//! This is about characters, not meaning: nothing here knows what a name
//! is for.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_and_invisible_characters_are_taken_out() {
        // "Ann" with a right-to-left override that would show "exe.txt"
        // backwards, an isolate, a zero-width space and a tag character.
        let hostile = "\u{202E}Ann\u{2066} \u{200B}Bea\u{E0041}\u{200F}";
        assert_eq!(clean_text(hostile), "Ann Bea");
        assert!(!is_clean_text(hostile));
    }

    #[test]
    fn line_breaks_become_spaces_and_ends_are_trimmed() {
        assert_eq!(clean_text("  Stormy\n\tPetrel \r\n"), "Stormy Petrel");
        assert_eq!(clean_text("a\u{2028}b"), "a b");
        assert_eq!(clean_text("\u{0}\u{7}\u{1b}[31m"), "[31m");
    }

    #[test]
    fn ordinary_text_in_any_script_is_kept() {
        for name in [
            "Ann",
            "Zoë",
            "李小龙",
            "مریم",
            "נועה",
            "\u{1F469}\u{200D}\u{1F467}",
            "Mehr\u{200C}dad",
        ] {
            assert_eq!(clean_text(name), name, "{name:?}");
            assert!(is_clean_text(name));
        }
    }
}
