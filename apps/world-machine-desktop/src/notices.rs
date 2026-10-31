//! What the app tells about itself, bundled at build time: the
//! third-party notices (`acknowledgements.tsv`, written by
//! `scripts/generate-acknowledgements.py`), What's New (the latest release
//! in `CHANGELOG.md`) and Known Issues (the player-facing sections of
//! `docs/KNOWN_ISSUES.md`).
//!
//! Pure text: the windows (`notes_window.rs`) draw the blocks this makes.
//! The notes themselves are in English, as the repository writes them.

/// The generated third-party notices.
pub const ACKNOWLEDGEMENTS: &str = include_str!("../acknowledgements.tsv");

/// The changelog, of which the latest release is shown as What's New.
pub const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");

/// The known issues, of which the sections a player meets are shown.
pub const KNOWN_ISSUES: &str = include_str!("../../../docs/KNOWN_ISSUES.md");

/// The sections of `docs/KNOWN_ISSUES.md` a player meets; the rest are for
/// the people who make the game.
pub const PLAYER_ISSUE_SECTIONS: &[&str] = &["Installation", "Using Worlds"];

/// One third-party crate the app is built with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notice {
    pub name: String,
    pub version: String,
    pub licence: String,
    pub source: String,
}

/// Every crate in the notices file, in its order (by name).
pub fn acknowledgements() -> Vec<Notice> {
    parse_acknowledgements(ACKNOWLEDGEMENTS)
}

/// The titles of the three notes windows, in English.
pub const WHATS_NEW: &str = "What's New";
pub const KNOWN_ISSUES_TITLE: &str = "Known Issues";
pub const ACKNOWLEDGEMENTS_TITLE: &str = "Acknowledgements";

/// What the app carries that is not a crate, as the Acknowledgements window
/// says it (and the generated file's header records it).
pub const ASSET_NOTES: [&str; 3] = [
    "Fonts: none are bundled; the app draws with the system's own fonts.",
    "Sounds: none are bundled; every sound is synthesised by world-sound.",
    "Pictures: the app's art is World Machine's own (Apache-2.0).",
];

/// Every fixed line the notes windows show, in English: for the
/// translation check.
pub fn every_line() -> Vec<&'static str> {
    let mut lines = vec![
        WHATS_NEW,
        KNOWN_ISSUES_TITLE,
        ACKNOWLEDGEMENTS_TITLE,
        "These notes are in English.",
        "Help → Report a Problem… tells us about anything else.",
        "World Machine is built with the work of many others. Each piece of software below is used under the licence named over it.",
        "Every crate in Cargo.lock when the app was built, for every platform.",
    ];
    lines.extend(ASSET_NOTES);
    lines
}

/// The header's lines that say what is not a crate (fonts, sounds,
/// pictures), without their `#`.
pub fn asset_notes() -> Vec<&'static str> {
    ACKNOWLEDGEMENTS
        .lines()
        .filter_map(|line| line.strip_prefix("# "))
        .filter(|line| {
            ["Fonts:", "Sounds:", "Pictures:"]
                .iter()
                .any(|label| line.starts_with(label))
        })
        .collect()
}

pub fn parse_acknowledgements(text: &str) -> Vec<Notice> {
    text.lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| {
            let mut fields = line.split('\t');
            Some(Notice {
                name: fields.next()?.to_string(),
                version: fields.next()?.to_string(),
                licence: fields.next()?.to_string(),
                source: fields.next().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// A piece of a note, as a window draws it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Block {
    Heading(String),
    Paragraph(String),
    Bullet(String),
}

/// What's New: the newest release in the changelog, its heading first.
pub fn whats_new() -> Vec<Block> {
    whats_new_in(CHANGELOG)
}

pub fn whats_new_in(changelog: &str) -> Vec<Block> {
    let Some(at) = changelog.find("\n## ") else {
        return Vec::new();
    };
    let rest = &changelog[at + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |end| end + 3);
    blocks(&rest[..end])
}

/// Known Issues: the sections of `docs/KNOWN_ISSUES.md` a player meets.
pub fn known_issues() -> Vec<Block> {
    known_issues_in(KNOWN_ISSUES, PLAYER_ISSUE_SECTIONS)
}

pub fn known_issues_in(text: &str, sections: &[&str]) -> Vec<Block> {
    let mut shown = Vec::new();
    let mut keep = false;
    let mut section = String::new();
    let flush = |section: &mut String, keep: bool, shown: &mut Vec<Block>| {
        if keep {
            shown.extend(blocks(section));
        }
        section.clear();
    };
    for line in text.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            flush(&mut section, keep, &mut shown);
            keep = sections.contains(&title.trim());
        }
        section.push_str(line);
        section.push('\n');
    }
    flush(&mut section, keep, &mut shown);
    shown
}

/// Markdown, plainly: headings, bullets (a bullet's wrapped lines joined),
/// and paragraphs, with emphasis, code marks and links reduced to their
/// words.
pub fn blocks(markdown: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut current: Option<Block> = None;
    let finish = |current: &mut Option<Block>, out: &mut Vec<Block>| {
        if let Some(block) = current.take() {
            out.push(block);
        }
    };
    for raw in markdown.lines() {
        let line = raw.trim();
        if line.is_empty() {
            finish(&mut current, &mut out);
            continue;
        }
        if let Some(title) = line.strip_prefix('#') {
            finish(&mut current, &mut out);
            out.push(Block::Heading(plain(title.trim_start_matches('#').trim())));
            continue;
        }
        let bullet = line
            .strip_prefix("- ")
            .or_else(|| line.strip_prefix("* "))
            .filter(|_| raw.starts_with(['-', '*']));
        if let Some(text) = bullet {
            finish(&mut current, &mut out);
            current = Some(Block::Bullet(plain(text)));
            continue;
        }
        // A nested bullet or a wrapped line carries on the block before.
        let text = plain(line.trim_start_matches("- ").trim_start_matches("* "));
        match &mut current {
            Some(Block::Bullet(words)) | Some(Block::Paragraph(words)) => {
                words.push(' ');
                words.push_str(&text);
            }
            _ => current = Some(Block::Paragraph(text)),
        }
    }
    finish(&mut current, &mut out);
    out
}

/// A line of markdown as words: `**bold**`, `` `code` `` and `[text](url)`
/// keep their text and lose their marks.
pub fn plain(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find('[') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        match after.find("](") {
            Some(close) if !after[..close].contains('[') => {
                let text = &after[..close];
                let target = &after[close + 2..];
                out.push_str(text);
                rest = match target.find(')') {
                    Some(end) => &target[end + 1..],
                    None => "",
                };
            }
            _ => {
                out.push('[');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out.replace("**", "").replace('`', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_reads_as_plain_words() {
        assert_eq!(
            plain("**Not notarized.** See [INSTALL.md](INSTALL.md) and `fm`."),
            "Not notarized. See INSTALL.md and fm."
        );
        assert_eq!(plain("a [b] c"), "a [b] c");
        let text = "# Title\n\nSome words\nwrapped on.\n\n- one\n  carried on\n- two\n  - nested\n";
        assert_eq!(
            blocks(text),
            vec![
                Block::Heading("Title".into()),
                Block::Paragraph("Some words wrapped on.".into()),
                Block::Bullet("one carried on".into()),
                Block::Bullet("two nested".into()),
            ]
        );
    }

    #[test]
    fn whats_new_is_the_newest_release_alone() {
        let changelog =
            "# Changelog\n\nIntro.\n\n## v2 (today)\n\n- **New.** Thing.\n\n## v1\n\n- Old.\n";
        assert_eq!(
            whats_new_in(changelog),
            vec![
                Block::Heading("v2 (today)".into()),
                Block::Bullet("New. Thing.".into()),
            ]
        );
        // The real one: a release heading, then what it brought.
        let shown = whats_new();
        assert!(
            matches!(shown.first(), Some(Block::Heading(title)) if title.starts_with('v')),
            "{shown:?}"
        );
        assert!(shown.len() > 3, "{shown:?}");
    }

    #[test]
    fn known_issues_are_the_players_sections() {
        let text = "# Known\n\n## Installation\n\n- A.\n\n## The look\n\n- Art.\n\n## Using Worlds\n\n- B.\n";
        assert_eq!(
            known_issues_in(text, PLAYER_ISSUE_SECTIONS),
            vec![
                Block::Heading("Installation".into()),
                Block::Bullet("A.".into()),
                Block::Heading("Using Worlds".into()),
                Block::Bullet("B.".into()),
            ]
        );
        // Every player section is still in the real file: renaming one
        // there must be followed here.
        let shown = known_issues();
        for section in PLAYER_ISSUE_SECTIONS {
            assert!(
                shown.contains(&Block::Heading((*section).to_string())),
                "{section} is gone from docs/KNOWN_ISSUES.md"
            );
        }
        assert!(
            shown
                .iter()
                .filter(|b| matches!(b, Block::Bullet(_)))
                .count()
                > 5
        );
    }

    #[test]
    fn the_notices_list_crates_and_say_what_is_not_one() {
        let notices = acknowledgements();
        assert!(notices.len() > 100, "{}", notices.len());
        assert!(notices.iter().all(|notice| !notice.licence.is_empty()));
        assert!(notices.iter().any(|notice| notice.name == "gpui"));
        assert_eq!(
            asset_notes(),
            ASSET_NOTES,
            "the window says what the file says"
        );
    }
}
