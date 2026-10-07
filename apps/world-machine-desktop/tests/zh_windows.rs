//! Every window the app draws itself (Home, Settings, About, sharing, the
//! strip, a World's own bar) says its words in Chinese when shown in
//! Chinese: each fixed line it writes through the app's words is in the
//! catalogs, whole, with nothing left in English but names and a few
//! words that read the same in any language.

use std::path::Path;

/// Words that read the same in Chinese.
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
fn every_window_speaks_chinese_in_chinese() {
    let mut catalog = world_i18n::Catalog::parse(world_gpui::i18n::APP_ZH_HANS);
    for pack in world_builtins::ZH_HANS {
        catalog.extend(pack);
    }
    // The app's own words win, as the app installs them.
    catalog.extend(world_gpui::i18n::APP_ZH_HANS);
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
        "{} lines are not in Chinese:\n{}",
        missing.len(),
        missing.join("\n")
    );
    // Open, on every World on Home, is 打开, not a shop's 营业中.
    assert_eq!(catalog.translate("Open").as_deref(), Some("打开"));
}

/// Home's cards for starting a World: each built-in World's name and what
/// it is, as Home shows them, are in Chinese too.
#[test]
fn home_tells_every_world_in_chinese() {
    let mut catalog = world_i18n::Catalog::parse(world_gpui::i18n::APP_ZH_HANS);
    for pack in world_builtins::ZH_HANS {
        catalog.extend(pack);
    }
    catalog.extend(world_gpui::i18n::APP_ZH_HANS);
    let registry = world_builtins::registry().expect("the built-in Worlds");
    let mut missing = Vec::new();
    for descriptor in registry.descriptors() {
        for line in [&descriptor.title, &descriptor.description] {
            match catalog.translate(line) {
                Some(shown) if english_in(&shown).is_empty() => {}
                shown => missing.push(format!("{line} → {shown:?}")),
            }
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}
