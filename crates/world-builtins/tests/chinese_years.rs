//! Both built-in Packs in Chinese over three years, as a player sees
//! them, read against the catalogs the app installs: Tiny Society's
//! harbour, and each of Pocket Universe's three places. A line is left
//! partly English when, translated, it still has a word of two or more
//! Latin letters that is not a resident's name; residents keep their
//! names in Latin letters everywhere, and every place is called by its
//! Chinese name.
//!
//! ```text
//! cargo test --release -p world-builtins --test chinese_years -- --ignored --nocapture
//! ```
//!
//! Set `WORLD_MACHINE_UNTRANSLATED` to a file to write every line left
//! partly English there, one a line, with the place it came from, and
//! `WORLD_MACHINE_SENTENCES` as well to add each of its sentences that is
//! not in Chinese on its own, marked `#`.

use std::collections::BTreeSet;
use world_projection::{CanvasItemKind, ProjectionIntent::InvokeCommand, ProjectionSnapshot};

/// Under half a percent of what each place shows may stay partly English.
const BAR: f64 = 0.995;

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
    for item in &snapshot.canvas.items {
        if item.kind == CanvasItemKind::Actor {
            names.extend(item.label.split_whitespace().map(str::to_string));
        } else if !item.label.trim().is_empty() {
            shown.insert(item.label.clone());
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

/// What a place showed, the names in it, and the stories it told.
type Shown = (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>);

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
    let (mut shown, mut names) = (BTreeSet::new(), names());
    for day in 0..days {
        let snapshot = session.snapshot();
        read(&snapshot, &mut shown, &mut names);
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
    (shown, names, told)
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
    let (mut shown, mut names) = (BTreeSet::new(), names());
    read(&session.snapshot(), &mut shown, &mut names);
    session.handle(InvokeCommand(seed.into())).unwrap();
    for period in 0..periods {
        let snapshot = session.snapshot();
        read(&snapshot, &mut shown, &mut names);
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
    (shown, names, told)
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

#[test]
#[ignore]
fn three_years_of_both_packs_are_shown_in_chinese() {
    let catalog = catalog();
    let places = std::thread::scope(|scope| {
        let tiny = scope.spawn(|| tiny_society(1_080));
        let mars =
            scope.spawn(|| pocket_universe(pocket_universe::SEED_MARS_COLONY_COMMAND, 1_080));
        let maple =
            scope.spawn(|| pocket_universe(pocket_universe::SEED_1980S_TOWN_COMMAND, 1_080));
        let penguins = scope
            .spawn(|| pocket_universe(pocket_universe::SEED_PENGUIN_CIVILIZATION_COMMAND, 1_080));
        [
            ("Tiny Society", tiny.join().unwrap()),
            ("Mars Colony", mars.join().unwrap()),
            ("Maple Street", maple.join().unwrap()),
            ("Penguin Civilization", penguins.join().unwrap()),
        ]
    });
    let mut report = String::new();
    let mut short = Vec::new();
    for (place, (shown, names, told)) in &places {
        // The stories alone keep to the same bar.
        let untold = left(&catalog, told, names);
        let told_share = 1.0 - untold.len() as f64 / told.len().max(1) as f64;
        eprintln!(
            "{place}: {} of {} story texts partly English",
            untold.len(),
            told.len()
        );
        for (text, translated) in untold.iter().take(30) {
            eprintln!("  story: {text}  =>  {translated}");
        }
        assert!(told.len() > 20, "{place} told only {} stories", told.len());
        if told_share < BAR {
            short.push(format!(
                "{place} stories {:.2}%",
                (1.0 - told_share) * 100.0
            ));
        }
        let left = left(&catalog, shown, names);
        let share = 1.0 - left.len() as f64 / shown.len() as f64;
        eprintln!(
            "{place}: {} of {} shown texts partly English ({:.2}%)",
            left.len(),
            shown.len(),
            (1.0 - share) * 100.0
        );
        for (text, translated) in left.iter().take(30) {
            eprintln!("  {text}  =>  {translated}");
        }
        for (text, translated) in &left {
            report.push_str(&format!("{place}\t{text}\t{translated}\n"));
            if std::env::var("WORLD_MACHINE_SENTENCES").is_ok() {
                for sentence in sentences(text) {
                    let alone = catalog
                        .translate(sentence)
                        .unwrap_or_else(|| sentence.into());
                    if english(names, &alone) {
                        report.push_str(&format!("#\t{sentence}\t{alone}\n"));
                    }
                }
            }
        }
        if share < BAR {
            short.push(format!("{place} {:.2}%", (1.0 - share) * 100.0));
        }
    }
    if let Ok(path) = std::env::var("WORLD_MACHINE_UNTRANSLATED") {
        std::fs::write(path, report).unwrap();
    }
    assert!(short.is_empty(), "partly English: {}", short.join(", "));
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
    let names = names();
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

/// People keep their names in Latin letters in Chinese, everywhere: no
/// catalog gives a person's name, whole or first, a Chinese one, so a name
/// never shows one way in one line and another way in the next.
#[test]
fn people_keep_their_names() {
    let catalog = catalog();
    for name in tiny_society::people_names()
        .into_iter()
        .chain(pocket_universe::people_names())
    {
        let first = name.split_whitespace().next().unwrap_or(name);
        for name in [name, first] {
            let shown = catalog.exact(name).unwrap_or(name);
            assert_eq!(shown, name, "{name} is shown as {shown}");
        }
    }
}
