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

/// A harbour's first month of mornings, played as a player plays it: each
/// day they answer what they are asked, every third day make something,
/// and the day passes. Every letter (a good share of them about what the
/// player did), everything looked after in the season's way, and every
/// find of the book reads whole in Chinese and in Japanese.
#[test]
fn a_month_of_harbour_mornings_reads_whole_in_chinese_and_japanese() {
    let mut branch = TinySocietyBranch::new_world().unwrap();
    let mut mornings = Vec::new();
    for day in 1..=35_u64 {
        let snapshot = branch.projection_snapshot();
        if let Some(answer) = snapshot.commands.iter().find(|command| {
            command.question.is_some()
                && command.unavailable.is_none()
                && command.role != Some(CommandRole::PassesTime)
        }) {
            let _ = branch.invoke_projection_command(&answer.id.clone());
        }
        if day.is_multiple_of(3) {
            let snapshot = branch.projection_snapshot();
            let deed = snapshot
                .commands
                .iter()
                .filter(|command| {
                    command.unavailable.is_none()
                        && command
                            .hand
                            .as_ref()
                            .is_some_and(|hand| hand.verb != "Undo" && hand.verb != "Suggest")
                })
                .nth(day as usize / 3 % 3)
                .map(|command| command.id.clone());
            if let Some(deed) = deed {
                let _ = branch.invoke_projection_command(&deed);
            }
        }
        let pass = pass_day(&branch.projection_snapshot());
        branch.invoke_projection_command(&pass).unwrap();
        let _ = branch.invoke_projection_command(SITTING_BEGINS_COMMAND);
        mornings.extend(branch.projection_snapshot().morning);
    }
    assert!(mornings.len() >= 34, "{} mornings", mornings.len());
    let citing = mornings
        .iter()
        .filter_map(|morning| morning.letter.as_ref())
        .filter(|letter| letter.note.contains(" you "))
        .count();
    assert!(citing >= 5, "{citing} letters about the player");
    let wrong = untranslated(&mornings);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Every line a morning, a letter or a use of something the player made is
/// told in, as the Systems write it, is in both languages' catalogs:
/// whole, or a sentence at a time, as lines are shown.
#[test]
fn every_line_of_mornings_letters_and_uses_is_in_both_catalogs() {
    let lines = ritual::lines()
        .into_iter()
        .chain(lives::letter_leads().into_iter().map(str::to_string))
        .chain(lives::about_you_lines().into_iter().map(str::to_string))
        .chain(hands::told_lines().into_iter().map(str::to_string))
        .collect::<Vec<_>>();
    let sentences = |line: &str| {
        let mut sentences = Vec::new();
        let mut start = 0;
        for (at, _) in line.match_indices(['.', '!', '?']) {
            if line[at + 1..].starts_with(' ') {
                sentences.push(line[start..=at].to_string());
                start = at + 2;
            }
        }
        sentences.push(line[start..].to_string());
        sentences
    };
    assert!(lines.len() > 150, "{}", lines.len());
    for (language, catalogs) in [("zh", world_builtins::ZH_HANS), ("ja", world_builtins::JA)] {
        let keys = catalogs
            .iter()
            .flat_map(|catalog| catalog.lines())
            .filter_map(|line| line.split_once('\t'))
            .map(|(english, _)| english)
            .collect::<BTreeSet<_>>();
        let missing = lines
            .iter()
            .filter(|line| {
                !keys.contains(line.as_str())
                    && !sentences(line)
                        .iter()
                        .all(|sentence| keys.contains(sentence.as_str()))
            })
            .collect::<Vec<_>>();
        assert!(missing.is_empty(), "{language}: {missing:#?}");
    }
}
