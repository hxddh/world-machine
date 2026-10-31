//! A sitting's morning (v0.30 S4) read in each language the app installs:
//! its letter, the day's choice, what was looked after and the find, in
//! both built-in Packs, each translated whole by the catalogs the app
//! installs, with no English word left in it.

use pocket_universe::{PocketUniverse, NUDGE_COMMAND, SEED_MARS_COLONY_COMMAND};
use std::collections::BTreeSet;
use tiny_society::TinySocietyBranch;
use world_projection::{CommandRole, Morning, ProjectionSnapshot, SITTING_BEGINS_COMMAND};

fn catalog(texts: &[&str]) -> world_i18n::Catalog {
    texts
        .iter()
        .fold(world_i18n::Catalog::default(), |mut all, text| {
            all.extend(text);
            all
        })
}

/// Words Chinese and Japanese write in Latin letters themselves.
const LATIN: [&str; 4] = ["MTV", "DJ", "CD", "TV"];

/// Every line a morning shows.
fn lines(morning: &Morning) -> Vec<String> {
    morning
        .letter
        .iter()
        .map(|letter| letter.note.clone())
        .chain(
            morning
                .question
                .iter()
                .map(|question| question.prompt.clone()),
        )
        .chain(morning.tended.iter().map(|line| line.line.clone()))
        .chain(morning.find.iter().map(|line| line.line.clone()))
        .collect()
}

/// The English left in `shown`: words of two or more Latin letters.
fn english_in(shown: &str) -> Vec<String> {
    shown
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| word.len() >= 2 && !LATIN.contains(word))
        .map(str::to_string)
        .collect()
}

/// Each morning's lines, read in Chinese and in Japanese: what is left
/// untranslated, as "language: line => shown".
fn untranslated(mornings: &[Morning]) -> Vec<String> {
    let zh = catalog(&world_builtins::ZH_HANS);
    let ja = catalog(&world_builtins::JA);
    let mut wrong = Vec::new();
    for morning in mornings {
        for line in lines(morning) {
            for (language, catalog) in [("zh", &zh), ("ja", &ja)] {
                let shown = catalog.translate(&line);
                let left = shown.as_deref().map(english_in);
                if left.as_ref().is_none_or(|left| !left.is_empty()) {
                    wrong.push(format!("{language}: {line} => {shown:?}"));
                }
            }
        }
    }
    wrong
}

fn pass_day(snapshot: &ProjectionSnapshot) -> String {
    snapshot
        .commands
        .iter()
        .find(|command| command.role == Some(CommandRole::PassesTime))
        .expect("the day's card")
        .id
        .clone()
}

#[test]
fn a_harbour_morning_reads_whole_in_chinese_and_japanese() {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let mut mornings = Vec::new();
    for _ in 0..8 {
        let day = pass_day(&branch.projection_snapshot());
        branch.invoke_projection_command(&day).unwrap();
        // A new harbour's first session opens each of its days on its
        // morning as it comes; every later day's comes when a sitting
        // begins.
        let came = branch
            .invoke_projection_command(SITTING_BEGINS_COMMAND)
            .is_ok();
        let morning = branch.projection_snapshot().morning.expect("a morning");
        mornings.push(morning);
        assert!(came || mornings.len() < 6, "a later day's morning");
    }
    let parts = mornings.iter().flat_map(lines).collect::<BTreeSet<_>>();
    assert!(parts.len() >= 8 * 3, "{parts:#?}");
    let wrong = untranslated(&mornings);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_colony_morning_reads_whole_in_chinese_and_japanese() {
    let mut world = PocketUniverse::new().unwrap();
    world
        .invoke_projection_command(SEED_MARS_COLONY_COMMAND)
        .unwrap();
    world.invoke_projection_command(NUDGE_COMMAND).unwrap();
    let mut mornings = Vec::new();
    for _ in 0..6 {
        world.invoke_projection_command(NUDGE_COMMAND).unwrap();
        world
            .invoke_projection_command(SITTING_BEGINS_COMMAND)
            .unwrap();
        mornings.push(world.projection_snapshot().morning.expect("a morning"));
    }
    let wrong = untranslated(&mornings);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
