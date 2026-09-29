//! Pocket Universe in Chinese past its first year: three years of each
//! place, as a player sees it, read against the catalogs the app installs.
//! What each place's own years bring (its works, its people coming and
//! going, its new festivals and what its turns ask) is in the catalogs as
//! well as its first year is.
//!
//! ```text
//! cargo test --release -p pocket-universe --test chinese_years -- --ignored --nocapture
//! ```

use std::collections::BTreeSet;
use world_projection::ProjectionIntent::InvokeCommand;

/// The catalogs the app installs for Pocket Universe: the Systems', Tiny
/// Society's (whose shared lines Pocket Universe also says) and its own,
/// and its residents' own lines worked out from them.
fn catalog() -> world_i18n::Catalog {
    let mut catalog = world_i18n::Catalog::default();
    catalog.extend(include_str!(
        "../../../crates/world-builtins/locales/systems.zh-Hans.tsv"
    ));
    catalog.extend(include_str!("../../tiny-society/locales/zh-Hans.tsv"));
    catalog.extend(include_str!("../locales/zh-Hans.tsv"));
    let mut voices = String::new();
    for (lines, slots) in pocket_universe::voice_templates() {
        for line in lines {
            let mut start = 0;
            let mut sentences = Vec::new();
            for (index, _) in line.match_indices(['.', '!', '?']) {
                if line[index + 1..].starts_with(' ') {
                    sentences.push(&line[start..=index]);
                    start = index + 2;
                }
            }
            sentences.push(&line[start..]);
            for sentence in sentences {
                for (english, chinese) in catalog.expand(sentence, slots) {
                    voices.push_str(&format!("{english}\t{chinese}\n"));
                }
            }
        }
    }
    catalog.extend(&voices);
    catalog
}

/// Everything `periods` of each place shows, looked at every `every`
/// periods, and every name in it.
fn shown(periods: usize, every: usize) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    let mut shown = BTreeSet::new();
    let mut names = BTreeSet::new();
    for seed in [
        pocket_universe::SEED_MARS_COLONY_COMMAND,
        pocket_universe::SEED_1980S_TOWN_COMMAND,
        pocket_universe::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let mut session = registry
            .create(pocket_universe::POCKET_UNIVERSE_PACK_ID)
            .unwrap();
        session.handle(InvokeCommand(seed.into())).unwrap();
        for period in 0..periods {
            let snapshot = session.snapshot();
            if period % every == 0 {
                for item in &snapshot.canvas.items {
                    names.extend(item.label.split_whitespace().map(str::to_string));
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
                texts.extend(snapshot.book.iter().map(|entry| entry.name.clone()));
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
                shown.extend(texts.into_iter().filter(|text| !text.trim().is_empty()));
            }
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
    }
    (shown, names)
}

/// Three years of every place: nearly every sentence it shows, in its
/// later years as in its first, is in the catalogs, and what is left is
/// only names. Set `WORLD_MACHINE_UNTRANSLATED` to a file to write what
/// is left there.
#[test]
#[ignore]
fn three_years_of_pocket_universe_are_shown_in_chinese() {
    let catalog = catalog();
    let (shown, names) = shown(1_080, 3);
    let english = |text: &str| {
        text.split(|c: char| !c.is_ascii_alphabetic())
            .filter(|word| word.len() > 1 && !names.contains(*word))
            .count()
    };
    let left = shown
        .iter()
        .filter_map(|text| {
            let translated = catalog.translate(text).unwrap_or_else(|| text.clone());
            (english(&translated) > 0).then(|| (text.clone(), translated))
        })
        .collect::<Vec<_>>();
    let share = 1.0 - left.len() as f64 / shown.len() as f64;
    eprintln!(
        "{} of {} shown texts fully in Chinese ({:.1}%)",
        shown.len() - left.len(),
        shown.len(),
        share * 100.0
    );
    for (text, translated) in left.iter().take(60) {
        eprintln!("  {text}  =>  {translated}");
    }
    if let Ok(path) = std::env::var("WORLD_MACHINE_UNTRANSLATED") {
        let text = left
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(path, text).unwrap();
    }
    assert!(share >= 0.95, "only {:.1}% in Chinese", share * 100.0);
}
