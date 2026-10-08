//! The three places' names, held in every language they speak: every name
//! the Pack can produce is in its lexicon in English, Chinese and Japanese,
//! and none of its own lines reads as naming someone invented
//! (`world_pack_testkit::lexicon`).

use crate::{
    PocketUniverse, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use world_core::Value;
use world_pack_testkit::lexicon;

const SEEDS: [&str; 3] = [
    SEED_MARS_COLONY_COMMAND,
    SEED_1980S_TOWN_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
];

fn opened(seed: &str) -> PocketUniverse {
    let mut world = PocketUniverse::new().unwrap();
    world.invoke_projection_command(seed).unwrap();
    for _ in 0..3 {
        world.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    world
}

/// What the player says in each language, for a hearing in it.
fn words(lang: &str) -> &'static str {
    match lang {
        "zh" => "最近有什么新鲜事吗？",
        "ja" => "最近なにか変わったことある？",
        _ => "Anything new lately?",
    }
}

fn hearing(world: &PocketUniverse, lang: &str) -> conversation::Hearing {
    let state = world.world().state();
    let kit = crate::speech::kit(state);
    let who = crate::life::people_in(state)[0];
    conversation::hearing_for(world.world(), &kit, who, words(lang)).unwrap()
}

fn has_han(text: &str) -> bool {
    text.chars().any(|c| matches!(c as u32, 0x4E00..=0x9FFF))
}

fn has_kana(text: &str) -> bool {
    text.chars().any(|c| matches!(c as u32, 0x3040..=0x30FF))
}

#[test]
fn every_name_the_three_places_can_produce_is_known_in_every_language() {
    let mut missing = Vec::new();
    for seed in SEEDS {
        let world = opened(seed);
        let mut names = crate::people_names()
            .into_iter()
            .chain(
                ["sol", "week", "day"]
                    .into_iter()
                    .flat_map(crate::speech::elsewhere)
                    .copied(),
            )
            .chain(
                crate::almanac::MARS
                    .iter()
                    .chain(crate::almanac::TOWN)
                    .chain(crate::almanac::ICE)
                    .map(|festival| festival.name),
            )
            .map(str::to_string)
            .collect::<std::collections::BTreeSet<_>>();
        for entity in world.world().state().entities() {
            // A System's own notes are nobody anyone speaks of.
            if ["lives", "calendar", "story"].contains(&entity.kind.as_str()) {
                continue;
            }
            if let Some(Value::Text(name)) = entity.component("name") {
                names.insert(name.clone());
            }
        }
        let hearings = ["en", "zh", "ja"].map(|lang| (lang, hearing(&world, lang)));
        for name in names {
            if !name.chars().any(char::is_uppercase) || !name.is_ascii() {
                continue;
            }
            let forms = crate::speech::aliases(&name);
            if !forms.iter().any(|form| has_han(form)) {
                missing.push(format!("{name}: no Chinese form"));
            }
            if !forms.iter().any(|form| has_kana(form)) {
                missing.push(format!("{name}: no Japanese form"));
            }
            let all = std::iter::once(name.clone())
                .chain(forms.iter().cloned())
                .collect::<Vec<_>>();
            for (lang, hearing) in &hearings {
                for unknown in lexicon::unknown_names(&all, hearing) {
                    missing.push(format!("{name}: {unknown} unknown in {lang} ({seed})"));
                }
            }
        }
    }
    missing.sort();
    missing.dedup();
    assert!(missing.is_empty(), "{missing:#?}");
}

#[test]
fn none_of_the_three_places_own_lines_names_anyone_invented() {
    let world = opened(SEED_1980S_TOWN_COMMAND);
    let lines = lexicon::catalog_lines(
        &[
            ("zh", include_str!("../locales/zh-Hans.tsv")),
            ("ja", include_str!("../locales/ja.tsv")),
        ],
        "Donna",
        crate::red_team::FIXTURES,
    );
    let failed = lexicon::invented_in_lines(&lines, |lang| hearing(&world, lang));
    assert!(
        failed.is_empty(),
        "{} lines: {:#?}",
        failed.len(),
        &failed[..failed.len().min(80)]
    );
}
