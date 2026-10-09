//! Both built-in Packs in Chinese and in Japanese, as a player sees
//! them, read against the catalogs the app installs: Tiny Society's
//! harbour, and each of Pocket Universe's three places, played once and
//! read in both languages. A line is left partly untranslated when,
//! translated, it still has a word of two or more Latin letters that the
//! language does not write that way: in Chinese every name is written in
//! Chinese characters, and in Japanese in katakana (v0.29: names are
//! written in the reader's script). Every place is called by its own name in each language, no
//! line starts with a mark that ends or closes something, and Japanese
//! quotes with 「」.
//!
//! ```text
//! cargo test --release -p world-builtins --test chinese_years -- --ignored --nocapture
//! ```
//!
//! `LANGUAGE_DAYS` sets how long each place is played (a year by
//! default). Set `WORLD_MACHINE_UNTRANSLATED` to a file to write every
//! line left partly untranslated there, one a line, with the language and
//! place it came from, and `WORLD_MACHINE_SENTENCES` as well to add each
//! of its sentences that is not translated on its own, marked `#`.

use std::collections::BTreeSet;
use world_projection::{CanvasItemKind, ProjectionIntent::InvokeCommand, ProjectionSnapshot};

/// At most one in two thousand of what each place shows in a year may stay
/// partly untranslated.
const BAR: f64 = 0.9995;

fn catalog() -> world_i18n::Catalog {
    let mut catalog =
        world_builtins::ZH_HANS
            .iter()
            .fold(world_i18n::Catalog::default(), |mut all, text| {
                all.extend(text);
                all
            });
    catalog.extend(&world_builtins::zh_hans_voices());
    catalog
}

fn japanese_catalog() -> world_i18n::Catalog {
    let mut catalog =
        world_builtins::JA
            .iter()
            .fold(world_i18n::Catalog::default(), |mut all, text| {
                all.extend(text);
                all
            });
    catalog.extend(&world_builtins::ja_voices());
    catalog
}

/// Words Japanese itself writes in Latin letters.
const LATIN_IN_JAPANESE: [&str; 4] = ["MTV", "DJ", "CD", "TV"];

/// Marks no line may start with.
const NEVER_FIRST: [char; 12] = [
    '。', '、', '，', '」', '）', '！', '？', '：', '；', '』', '”', '.',
];

/// Words Chinese itself writes in Latin letters.
const LATIN_IN_CHINESE: [&str; 3] = ["MTV", "DJ", "CD"];

/// Every person's name in both Packs, and the words Chinese writes in
/// Latin letters: what may stay in Latin letters in a Chinese line.
fn names() -> BTreeSet<String> {
    tiny_society::people_names()
        .into_iter()
        .chain(pocket_universe::people_names())
        .flat_map(str::split_whitespace)
        .chain(LATIN_IN_CHINESE)
        .map(str::to_string)
        .collect()
}

/// Every text a player reads in `snapshot`, the names of its places and
/// things among them, and the names of the people in it.
fn read(snapshot: &ProjectionSnapshot, shown: &mut BTreeSet<String>, names: &mut BTreeSet<String>) {
    read_letters(snapshot, &mut BTreeSet::new(), shown, names);
}

/// The same, keeping every letter in `letters` as well.
fn read_letters(
    snapshot: &ProjectionSnapshot,
    letters: &mut BTreeSet<String>,
    shown: &mut BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    letters.extend(snapshot.letters.iter().map(|letter| letter.note.clone()));
    for item in &snapshot.canvas.items {
        if item.kind == CanvasItemKind::Actor {
            names.extend(item.label.split_whitespace().map(str::to_string));
        } else if !item.label.trim().is_empty() {
            shown.insert(item.label.clone());
        }
        // What someone does, or what a place is, as the person card and
        // the label under it say.
        if !item.detail.trim().is_empty() {
            shown.insert(item.detail.clone());
        }
    }
    let mut texts = vec![snapshot.title.clone()];
    texts.extend(snapshot.voices.iter().map(|voice| voice.line.clone()));
    for command in &snapshot.commands {
        texts.push(command.title.clone());
        texts.push(command.detail.clone());
        if let Some(question) = &command.question {
            texts.push(question.prompt.clone());
        }
    }
    texts.extend(snapshot.keepsakes.iter().map(|kept| kept.what.clone()));
    texts.extend(snapshot.keepsakes.iter().map(|kept| kept.note.clone()));
    texts.extend(
        snapshot
            .letters
            .iter()
            .rev()
            .take(2)
            .map(|letter| letter.note.clone()),
    );
    texts.extend(snapshot.book.iter().map(|entry| entry.name.clone()));
    texts.extend(snapshot.book.iter().map(|entry| entry.hint.clone()));
    texts.extend(
        snapshot
            .chapters
            .iter()
            .map(|chapter| chapter.title.clone()),
    );
    texts.extend(snapshot.goals.iter().map(|goal| goal.label.clone()));
    texts.extend(snapshot.gauges.iter().map(|gauge| gauge.label.clone()));
    if let Some(calendar) = &snapshot.calendar {
        texts.extend(calendar.season.clone());
        texts.extend(calendar.coming.clone());
    }
    if let Some(briefing) = &snapshot.briefing {
        texts.extend(briefing.items.iter().map(|item| item.title.clone()));
    }
    for moment in &snapshot.moments {
        texts.push(moment.title.clone());
        texts.extend(moment.panels.iter().map(|panel| panel.caption.clone()));
    }
    if let Some(almanac) = &snapshot.almanac {
        texts.push(almanac.title.clone());
        texts.extend(almanac.built.iter().cloned());
    }
    shown.extend(texts.into_iter().filter(|text| !text.trim().is_empty()));
}

/// What a place showed, the names in it, the stories it told, and every
/// letter written to the player.
type Shown = (
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
);

/// Every story the World tells at the end: the legend of everyone and
/// everything in the scene, every moment in the book and every year's
/// almanac, as a player reads them.
fn stories(session: &dyn world_host::WorldSession) -> BTreeSet<String> {
    use world_projection::{StoryPage, StoryRequest};
    let snapshot = session.snapshot();
    let mut texts = Vec::new();
    let mut ask = |request| match session.story(request).unwrap() {
        Some(StoryPage::Legend(legend)) => {
            for line in legend.lines {
                texts.push(line.text);
                texts.extend(line.because);
            }
        }
        Some(StoryPage::Moment(moment)) => {
            texts.push(moment.title);
            texts.extend(moment.panels.map(|panel| panel.caption));
        }
        Some(StoryPage::Almanac(almanac)) => {
            texts.push(almanac.title);
            texts.extend(almanac.built);
        }
        None => {}
    };
    for item in &snapshot.canvas.items {
        ask(StoryRequest::Legend(item.id));
    }
    for moment in snapshot
        .book
        .iter()
        .filter_map(|entry| entry.moment.clone())
    {
        ask(StoryRequest::Moment(moment));
    }
    for year in 1..=12 {
        ask(StoryRequest::Almanac(year));
    }
    texts
        .into_iter()
        .filter(|text| !text.trim().is_empty())
        .collect()
}

/// Three years of Tiny Society, played warmly: the first question each
/// day answered, something built every week.
fn tiny_society(days: usize) -> Shown {
    let registry = world_builtins::registry().unwrap();
    let mut session = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
    let (mut shown, mut names, mut letters) = (BTreeSet::new(), names(), BTreeSet::new());
    for day in 0..days {
        let snapshot = session.snapshot();
        read_letters(&snapshot, &mut letters, &mut shown, &mut names);
        let pick = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none())
            .map(|c| c.id.clone());
        if let Some(pick) = pick {
            let _ = session.handle(InvokeCommand(pick));
        }
        if day % 7 == 3 {
            if let Some(build) = snapshot
                .commands
                .iter()
                .find(|c| {
                    c.hand.as_ref().is_some_and(|hand| hand.verb == "Build")
                        && c.unavailable.is_none()
                })
                .map(|c| c.id.clone())
            {
                let _ = session.handle(InvokeCommand(build));
            }
        }
        session
            .handle(InvokeCommand("tiny-society.let-day-pass".into()))
            .unwrap();
    }
    let told = stories(session.as_ref());
    shown.extend(told.iter().cloned());
    (shown, names, told, letters)
}

/// Three years of one of Pocket Universe's places.
fn pocket_universe(seed: &str, periods: usize) -> Shown {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    let mut session = registry
        .create(pocket_universe::POCKET_UNIVERSE_PACK_ID)
        .unwrap();
    let (mut shown, mut names, mut letters) = (BTreeSet::new(), names(), BTreeSet::new());
    read(&session.snapshot(), &mut shown, &mut names);
    session.handle(InvokeCommand(seed.into())).unwrap();
    for period in 0..periods {
        let snapshot = session.snapshot();
        read_letters(&snapshot, &mut letters, &mut shown, &mut names);
        let pick = snapshot
            .commands
            .iter()
            .find(|c| {
                c.question.is_some()
                    && c.unavailable.is_none()
                    && c.id != pocket_universe::NUDGE_COMMAND
            })
            .map(|c| c.id.clone());
        if let Some(pick) = pick {
            let _ = session.handle(InvokeCommand(pick));
        }
        if period % 3 == 2 {
            if let Some(deed) = snapshot
                .commands
                .iter()
                .find(|c| c.hand.is_some() && c.unavailable.is_none())
            {
                let _ = session.handle(InvokeCommand(deed.id.clone()));
            }
        }
        session
            .handle(InvokeCommand(pocket_universe::NUDGE_COMMAND.into()))
            .unwrap();
    }
    let told = stories(session.as_ref());
    shown.extend(told.iter().cloned());
    (shown, names, told, letters)
}

fn english(names: &BTreeSet<String>, text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| word.len() > 1 && !names.contains(word))
}

/// A text's sentences, split as the catalogs split them.
fn sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut start, bytes) = (0, text.as_bytes());
    for index in 0..bytes.len() {
        if matches!(bytes[index], b'.' | b'!' | b'?')
            && bytes.get(index + 1) == Some(&b' ')
            && bytes
                .get(index + 2)
                .is_some_and(|next| next.is_ascii_uppercase() || *next == b'"')
        {
            out.push(text[start..=index].trim());
            start = index + 2;
        }
    }
    out.push(text[start..].trim());
    out
}

/// What of `shown` is left partly English, translated.
fn left(
    catalog: &world_i18n::Catalog,
    shown: &BTreeSet<String>,
    names: &BTreeSet<String>,
) -> Vec<(String, String)> {
    shown
        .iter()
        .filter_map(|text| {
            let translated = catalog.translate(text).unwrap_or_else(|| text.clone());
            english(names, &translated).then(|| (text.clone(), translated))
        })
        .collect()
}

/// What of `shown` reads wrong once translated: a line that starts with
/// a mark that ends something, or, in Japanese, quotes in “”.
fn misset(catalog: &world_i18n::Catalog, shown: &BTreeSet<String>, japanese: bool) -> Vec<String> {
    shown
        .iter()
        .filter_map(|text| {
            let translated = catalog.translate(text)?;
            let first = translated.trim_start().starts_with(NEVER_FIRST);
            let quotes = japanese && translated.contains(['“', '”']);
            (first || quotes).then(|| format!("{text}  =>  {translated}"))
        })
        .collect()
}

#[test]
#[ignore]
fn a_year_of_both_packs_is_shown_in_chinese_and_japanese() {
    let days: usize = std::env::var("LANGUAGE_DAYS")
        .ok()
        .and_then(|days| days.parse().ok())
        .unwrap_or(360);
    let places = std::thread::scope(|scope| {
        let tiny = scope.spawn(|| tiny_society(days));
        let mars = scope.spawn(|| pocket_universe(pocket_universe::SEED_MARS_COLONY_COMMAND, days));
        let maple = scope.spawn(|| pocket_universe(pocket_universe::SEED_1980S_TOWN_COMMAND, days));
        let penguins = scope
            .spawn(|| pocket_universe(pocket_universe::SEED_PENGUIN_CIVILIZATION_COMMAND, days));
        [
            ("Tiny Society", tiny.join().unwrap()),
            ("Mars Colony", mars.join().unwrap()),
            ("Maple Street", maple.join().unwrap()),
            ("Penguin Civilization", penguins.join().unwrap()),
        ]
    });
    let chinese = catalog();
    let japanese = japanese_catalog();
    // No name stays in Latin letters, in either language: only what the
    // language itself writes that way.
    let latin_in_japanese = LATIN_IN_JAPANESE
        .iter()
        .map(|word| word.to_string())
        .collect::<BTreeSet<_>>();
    let latin_in_chinese = LATIN_IN_CHINESE
        .iter()
        .map(|word| word.to_string())
        .collect::<BTreeSet<_>>();
    let mut report = String::new();
    let mut short = Vec::new();
    for (language, catalog, is_japanese) in [("zh", &chinese, false), ("ja", &japanese, true)] {
        for (place, (shown, _names, told, letters)) in &places {
            let allowed = if is_japanese {
                &latin_in_japanese
            } else {
                &latin_in_chinese
            };
            // No letter is left partly English: not one.
            let unlettered = left(catalog, letters, allowed);
            eprintln!(
                "{language} {place}: {} of {} letters partly untranslated",
                unlettered.len(),
                letters.len()
            );
            for (text, translated) in &unlettered {
                eprintln!("  letter: {text}  =>  {translated}");
            }
            if !unlettered.is_empty() {
                short.push(format!(
                    "{language} {place}: {} letters partly English",
                    unlettered.len()
                ));
            }
            // The stories alone keep to the same bar.
            let untold = left(catalog, told, allowed);
            let told_share = 1.0 - untold.len() as f64 / told.len().max(1) as f64;
            eprintln!(
                "{language} {place}: {} of {} story texts partly untranslated",
                untold.len(),
                told.len()
            );
            for (text, translated) in untold.iter().take(30) {
                eprintln!("  story: {text}  =>  {translated}");
            }
            assert!(told.len() > 20, "{place} told only {} stories", told.len());
            if told_share < BAR {
                short.push(format!(
                    "{language} {place} stories {:.2}%",
                    (1.0 - told_share) * 100.0
                ));
            }
            let left = left(catalog, shown, allowed);
            let share = 1.0 - left.len() as f64 / shown.len() as f64;
            eprintln!(
                "{language} {place}: {} of {} shown texts partly untranslated ({:.3}%)",
                left.len(),
                shown.len(),
                (1.0 - share) * 100.0
            );
            for (text, translated) in left.iter().take(40) {
                eprintln!("  {text}  =>  {translated}");
            }
            for (text, translated) in &left {
                report.push_str(&format!("{language}\t{place}\t{text}\t{translated}\n"));
                if std::env::var("WORLD_MACHINE_SENTENCES").is_ok() {
                    for sentence in sentences(text) {
                        let alone = catalog
                            .translate(sentence)
                            .unwrap_or_else(|| sentence.into());
                        if english(allowed, &alone) {
                            report.push_str(&format!("#\t{language}\t{sentence}\t{alone}\n"));
                        }
                    }
                }
            }
            if share < BAR {
                short.push(format!("{language} {place} {:.3}%", (1.0 - share) * 100.0));
            }
            let misset = misset(catalog, shown, is_japanese);
            for line in misset.iter().take(20) {
                eprintln!("  misset: {line}");
            }
            if !misset.is_empty() {
                short.push(format!("{language} {place}: {} lines misset", misset.len()));
            }
        }
    }
    if let Ok(path) = std::env::var("WORLD_MACHINE_UNTRANSLATED") {
        std::fs::write(path, report).unwrap();
    }
    assert!(
        short.is_empty(),
        "partly untranslated: {}",
        short.join(", ")
    );
}

/// How the catalogs show each line of a file, for translating:
/// `WORLD_MACHINE_LINES=lines.txt cargo test --release -p world-builtins --test chinese_years show_lines -- --ignored --nocapture`
/// prints each line, what it is shown as, and "BAD" if any of it is left
/// in English.
#[test]
#[ignore]
fn show_lines() {
    let catalog = catalog();
    let lines = match std::env::var("WORLD_MACHINE_LINES") {
        Ok(path) => std::fs::read_to_string(path).unwrap(),
        Err(_) => return,
    };
    let names = LATIN_IN_CHINESE
        .iter()
        .map(|word| word.to_string())
        .collect::<BTreeSet<_>>();
    for line in lines.lines().filter(|line| !line.is_empty()) {
        let shown = catalog.translate(line);
        let bad = shown.as_deref().is_none_or(|shown| english(&names, shown));
        eprintln!(
            "{}\t{line}\t{}",
            if bad { "BAD" } else { "ok" },
            shown.unwrap_or_default()
        );
    }
}

/// In Chinese, people's names are written in Chinese characters, whole
/// and first, the same in every line: every name has one in the Chinese
/// catalogs, and none keeps a Latin letter.
#[test]
fn people_have_their_names_in_chinese_characters() {
    let catalog = catalog();
    let mut missing = Vec::new();
    for name in tiny_society::people_names()
        .into_iter()
        .chain(pocket_universe::people_names())
    {
        let first = name.split_whitespace().next().unwrap_or(name);
        for name in [name, first] {
            let shown = catalog.exact(name);
            if !shown.is_some_and(|shown| !shown.chars().any(|c| c.is_ascii_alphabetic())) {
                missing.push(format!("{name}\t{}", shown.unwrap_or("")));
            }
        }
    }
    missing.sort();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "names not in Chinese characters:\n{}",
        missing.join("\n")
    );
}

/// In Japanese, people's names are written in katakana, whole and first,
/// the same in every line: every name has one in the Japanese catalogs.
#[test]
fn people_have_their_names_in_katakana_in_japanese() {
    let catalog =
        world_builtins::JA
            .iter()
            .fold(world_i18n::Catalog::default(), |mut all, text| {
                all.extend(text);
                all
            });
    let katakana = |text: &str| {
        text.chars()
            .all(|c| ('\u{30a0}'..='\u{30ff}').contains(&c) || "じいさんばあさん".contains(c))
    };
    for name in tiny_society::people_names()
        .into_iter()
        .chain(pocket_universe::people_names())
    {
        let first = name.split_whitespace().next().unwrap_or(name);
        for name in [name, first] {
            let shown = catalog.exact(name);
            assert!(
                shown.is_some_and(katakana),
                "{name} is shown as {shown:?} in Japanese"
            );
        }
    }
}

/// One name rule in each language, in every line of every catalog: a line
/// that names someone writes them in their one Chinese name in Chinese,
/// and their one katakana name in Japanese, never another way.
#[test]
fn one_name_rule_in_each_language() {
    let japanese = japanese_catalog();
    let chinese = catalog();
    let names = tiny_society::people_names()
        .into_iter()
        .chain(pocket_universe::people_names())
        .flat_map(str::split_whitespace)
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        // Names that are also words ("Snow", "Pebble Day", "the Sun's
        // Return"), where a line may mean the word.
        .filter(|name| !["Brr", "Frost", "Kelp", "Pebble", "Skip", "Snow", "Sun"].contains(name))
        .collect::<BTreeSet<_>>();
    let mut broken = Vec::new();
    for (language, texts) in [("zh", world_builtins::ZH_HANS), ("ja", world_builtins::JA)] {
        for line in texts.iter().flat_map(|text| text.lines()) {
            let Some((from, to)) = line.split_once('\t') else {
                continue;
            };
            if line.starts_with('#') || from.starts_with('=') {
                continue;
            }
            let words = from
                .split(|c: char| !c.is_alphanumeric())
                .collect::<BTreeSet<_>>();
            for name in names.iter().filter(|name| words.contains(*name)) {
                let written = match language {
                    "zh" => chinese.exact(name).map(str::to_string),
                    _ => japanese.exact(name).map(str::to_string),
                };
                if written.is_some_and(|written| !to.contains(&written)) {
                    broken.push(format!("{language}: {name} in {from:?} => {to:?}"));
                }
            }
        }
    }
    assert!(broken.is_empty(), "{}", broken.join("\n"));
}
