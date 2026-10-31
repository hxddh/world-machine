//! No stale copy, in English, Chinese or Japanese (1.0 P1): nothing the app
//! says, and no line its catalogs keep, tells the player of the v0.29
//! clock (a World living a day for every six hours away, or catching up
//! time on a return), of the return the World voice once narrated, or of
//! "this Mac" anywhere but where the Mac's own model is meant.
//!
//! What is scanned: every string the desktop app and the World window
//! write in their sources, the demo's pacing, and every line of the app's
//! and the built-in Worlds' catalogs, both sides of the tab.

use std::path::{Path, PathBuf};

/// Words of the old clock and the old voice, in each language.
const STALE: &[&str] = &[
    // English.
    "six hours",
    "a day for every",
    "lives while you",
    "lived {} while you were away",
    "lived on without you",
    "could not be caught up",
    "once per return",
    "Each return is sent",
    "Coming back, a World tells you",
    "Follow the Mac",
    // Chinese.
    "每离开六小时",
    "补上你离开期间",
    "每次回归",
    "跟随 Mac",
    "当你回来时，世界会用自己的话讲述",
    // Japanese.
    "6時間ごと",
    "留守の間の時間を進め",
    "戻るたびに",
    "Macの設定に従う",
    "戻ってきたとき、何があったかをワールドが",
];

/// "This Mac" in each language: said only of what is a Mac's alone (its
/// own model, offered only on a Mac) and in About's privacy line on a Mac,
/// which says "this computer" everywhere else (`shell::privacy_line`).
const THIS_MAC: &[&str] = &["this Mac", "This Mac", "这台 Mac", "このMac"];
const MAC_ONLY: &[&str] = &[
    "Mac's own model",
    "this Mac · free",
    "files on this Mac",
    "自带的模型",
    "留在这台 Mac 上 · 免费",
    "这台 Mac 上的文件",
    "このMacのモデル",
    "このMacの中だけ · 無料",
    "このMac上のファイル",
];

fn sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source folder") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

/// The string literals of a Rust source, roughly: what is between quotes on
/// each line that is not a comment.
fn literals(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in source.lines() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        let mut parts = line.split('"');
        parts.next();
        while let Some(inside) = parts.next() {
            found.push(inside.to_string());
            parts.next();
        }
    }
    found
}

/// Every line the player can be shown, and where it was found.
fn everything_said() -> Vec<(String, String)> {
    let app = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&app.join("src"), &mut files);
    sources(&app.join("../../crates/world-gpui/src"), &mut files);
    let mut said = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(&file).expect("the source");
        for literal in literals(&source) {
            said.push((file.display().to_string(), literal));
        }
    }
    let pacing = std::fs::read_to_string(app.join("demo/pacing.json")).expect("the pacing");
    said.push(("demo/pacing.json".into(), pacing));
    let catalogs = [
        ("zh-Hans", world_gpui::i18n::APP_ZH_HANS),
        ("ja", world_gpui::i18n::APP_JA),
    ]
    .into_iter()
    .chain(world_builtins::ZH_HANS.map(|text| ("built-in zh-Hans", text)))
    .chain(world_builtins::JA.map(|text| ("built-in ja", text)));
    for (name, text) in catalogs {
        for line in text.lines() {
            for side in line.split('\t') {
                said.push((format!("{name} catalog"), side.to_string()));
            }
        }
    }
    said
}

#[test]
fn nothing_said_in_any_language_is_stale() {
    let said = everything_said();
    assert!(said.len() > 5_000, "{} lines read", said.len());
    let mut stale = Vec::new();
    for (place, line) in &said {
        for phrase in STALE {
            if line.contains(phrase) {
                stale.push(format!("{place}: {phrase:?} in {line:?}"));
            }
        }
        if THIS_MAC.iter().any(|mac| line.contains(mac))
            && !MAC_ONLY.iter().any(|own| line.contains(own))
        {
            stale.push(format!("{place}: \"this Mac\" in {line:?}"));
        }
    }
    assert!(stale.is_empty(), "{}", stale.join("\n"));
}

/// The scan finds what it looks for: each stale phrase, and "this Mac"
/// said of anything but the Mac's own model.
#[test]
fn the_scan_finds_stale_copy() {
    assert_eq!(
        literals(r#"    let x = ui::t("Follow the Mac"); // "not this""#),
        ["Follow the Mac", "not this"]
    );
    assert!(literals("    // \"Follow the Mac\"").is_empty());
    let catches = |line: &str| {
        STALE.iter().any(|phrase| line.contains(phrase))
            || (THIS_MAC.iter().any(|mac| line.contains(mac))
                && !MAC_ONLY.iter().any(|own| line.contains(own)))
    };
    assert!(catches("Nothing leaves this Mac."));
    assert!(catches("你每离开六小时，小镇就过去一天。"));
    assert!(catches("Macの設定に従う"));
    assert!(!catches("On, through this Mac's own model."));
    assert!(!catches("Nothing leaves this computer."));
}
