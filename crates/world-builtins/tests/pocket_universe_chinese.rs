//! Pocket Universe in Chinese: a year of each place, as a player sees it.

use std::collections::BTreeSet;
use world_projection::ProjectionIntent::InvokeCommand;

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

/// Everything a year of each place shows, and every name in it.
fn shown(periods: usize) -> (BTreeSet<String>, BTreeSet<String>) {
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
        let first = session.snapshot();
        shown.extend(first.commands.iter().map(|c| c.title.clone()));
        shown.extend(first.commands.iter().map(|c| c.detail.clone()));
        session.handle(InvokeCommand(seed.into())).unwrap();
        for period in 0..periods {
            let snapshot = session.snapshot();
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
            shown.extend(texts.into_iter().filter(|text| !text.trim().is_empty()));
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
            if period % 5 == 2 {
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

fn left_in_english(
    catalog: &world_i18n::Catalog,
    shown: &BTreeSet<String>,
    names: &BTreeSet<String>,
) -> Vec<(String, String)> {
    let english = |text: &str| {
        text.split(|c: char| !c.is_ascii_alphabetic())
            .filter(|word| word.len() > 1 && !names.contains(*word))
            .count()
    };
    shown
        .iter()
        .filter_map(|text| {
            let translated = catalog.translate(text).unwrap_or_else(|| text.clone());
            (english(&translated) > 0).then(|| (text.clone(), translated))
        })
        .collect()
}

/// A year of every place: nearly every sentence it shows is in the
/// catalogs, and what is left is only names.
#[test]
#[ignore = "until the Pocket Universe catalog is written"]
fn a_year_of_pocket_universe_is_shown_in_chinese() {
    let catalog = catalog();
    let (shown, names) = shown(100);
    let left = left_in_english(&catalog, &shown, &names);
    let share = 1.0 - left.len() as f64 / shown.len() as f64;
    eprintln!(
        "{} of {} shown texts fully in Chinese ({:.1}%)",
        shown.len() - left.len(),
        shown.len(),
        share * 100.0
    );
    for (text, translated) in left.iter().take(40) {
        eprintln!("  {text}  =>  {translated}");
    }
    assert!(share >= 0.95, "only {:.1}% in Chinese", share * 100.0);
}

/// Every text a year of each place shows that is not yet in Chinese, one
/// a line, for translating:
/// `WORLD_MACHINE_UNTRANSLATED=file cargo test -p world-builtins --test pocket_universe_chinese -- --ignored`
#[test]
#[ignore]
fn write_untranslated() {
    let Ok(path) = std::env::var("WORLD_MACHINE_UNTRANSLATED") else {
        return;
    };
    let catalog = catalog();
    let (shown, names) = shown(200);
    let left = left_in_english(&catalog, &shown, &names);
    let text = left
        .into_iter()
        .map(|(text, _)| text)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, text).unwrap();
}
