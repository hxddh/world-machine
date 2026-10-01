//! Every window the app draws itself (Home, Settings, About, sharing, the
//! strip, a World's own bar) says its words in Japanese when shown in
//! Japanese, as the Chinese test checks for Chinese: each fixed line it writes through the app's words is in the
//! catalogs, whole, with nothing left in English but names and a few
//! words that read the same in any language.

use std::path::Path;

/// Words that read the same in Japanese.
const ALLOWED: &[&str] = &[
    "World",
    "Machine",
    "Mac",
    "API",
    "Claude",
    "Apple",
    "Finder",
    "Esc",
    "OK",
    "fm",
    "usr",
    "bin",
    "WORLD_MACHINE_VOICE_MODEL",
    "English",
    "Pack",
    "Packs",
    "GitHub",
];

/// The fixed lines a source file writes through the app's words: the text
/// handed to `ui::t` and to the helpers that translate what they show.
fn fixed_lines(source: &str) -> Vec<String> {
    // A call and which of its arguments is the words: the first, or (for
    // a button, which takes an id first) the second.
    const CALLS: &[(&str, usize)] = &[
        ("ui::t(", 0),
        ("ui::row_title(", 0),
        ("ui::caption(", 0),
        ("ui::page_title(", 0),
        ("ui::heading(", 0),
        ("ui::section_label(", 0),
        ("ui::body(", 0),
        ("ui::detail(", 0),
        ("ui::button(", 1),
        ("ui::named(", 1),
        ("row(", 0),
        ("home_section_title(", 0),
        (".child(", 0),
        ("switch_row(", 0),
    ];
    let mut lines = Vec::new();
    for (call, nth) in CALLS {
        let mut rest = source;
        while let Some(at) = rest.find(call) {
            rest = &rest[at + call.len()..];
            let mut args = rest;
            let mut ok = true;
            for _ in 0..*nth {
                // Skip one argument: up to the next comma at this depth.
                let mut depth = 0;
                let mut end = None;
                for (i, c) in args.char_indices() {
                    match c {
                        '(' | '[' => depth += 1,
                        ')' | ']' if depth == 0 => break,
                        ')' | ']' => depth -= 1,
                        ',' if depth == 0 => {
                            end = Some(i + 1);
                            break;
                        }
                        _ => {}
                    }
                }
                match end {
                    Some(end) => args = &args[end..],
                    None => ok = false,
                }
            }
            let args = args.trim_start();
            if !ok || !args.starts_with('"') {
                continue;
            }
            let Some(end) = args[1..].find('"') else {
                continue;
            };
            let line = &args[1..1 + end];
            if !line.contains('{')
                && !line.contains('\\')
                && line.chars().any(|c| c.is_ascii_alphabetic())
            {
                lines.push(line.to_string());
            }
        }
    }
    lines.sort();
    lines.dedup();
    lines
}

fn english_in(line: &str) -> Vec<String> {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|word| word.len() >= 2 && word.chars().any(|c| c.is_ascii_alphabetic()))
        .filter(|word| !ALLOWED.contains(word))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_window_speaks_japanese_in_japanese() {
    let mut catalog = world_i18n::Catalog::parse(world_gpui::i18n::APP_JA);
    for pack in world_builtins::JA {
        catalog.extend(pack);
    }
    // The app's own words win, as the app installs them.
    catalog.extend(world_gpui::i18n::APP_JA);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut missing = Vec::new();
    let mut read = 0;
    for file in [
        "main.rs",
        "settings.rs",
        "about.rs",
        "sharing.rs",
        "strip_window.rs",
        "updates.rs",
    ] {
        let source = std::fs::read_to_string(root.join(file)).expect("the source");
        for line in fixed_lines(&source) {
            read += 1;
            match catalog.translate(&line) {
                Some(shown) if english_in(&shown).is_empty() => {}
                Some(shown) => missing.push(format!("{file}: {line} → {shown}")),
                None => missing.push(format!("{file}: {line}")),
            }
        }
    }
    assert!(read > 40, "{read} lines read");
    assert!(
        missing.is_empty(),
        "{} lines are not in Japanese:\n{}",
        missing.len(),
        missing.join("\n")
    );
    // Open, on every World on Home, is 開く, not a shop's 営業中.
    assert_eq!(catalog.translate("Open").as_deref(), Some("開く"));
}
