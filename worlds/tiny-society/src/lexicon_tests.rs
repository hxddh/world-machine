//! The harbour's names, held in every language it speaks: every name the
//! Pack can produce is in its lexicon in English, Chinese and Japanese, and
//! none of its own lines reads as naming someone invented
//! (`world_pack_testkit::lexicon`).

use crate::MARA;
use world_core::Value;
use world_pack_testkit::lexicon;

/// What the player says in each language, for a hearing in it.
fn words(lang: &str) -> &'static str {
    match lang {
        "zh" => "最近有什么新鲜事吗？",
        "ja" => "最近なにか変わったことある？",
        _ => "Anything new lately?",
    }
}

fn has_han(text: &str) -> bool {
    text.chars().any(|c| matches!(c as u32, 0x4E00..=0x9FFF))
}

fn has_kana(text: &str) -> bool {
    text.chars().any(|c| matches!(c as u32, 0x3040..=0x30FF))
}

/// Every name the Pack can produce: everyone who lives or may come to
/// live here and the people they remember, every named thing in a World
/// that has lived a while, what its people speak of elsewhere, its days.
fn produced() -> Vec<String> {
    let branch = crate::red_team::opened();
    let mut names = crate::people_names()
        .into_iter()
        .chain(crate::speech::ELSEWHERE.iter().copied())
        .chain(
            crate::almanac::FESTIVALS
                .iter()
                .map(|festival| festival.name),
        )
        .map(str::to_string)
        .collect::<std::collections::BTreeSet<_>>();
    for entity in branch.world().state().entities() {
        // A System's own notes are nobody anyone speaks of.
        if ["lives", "calendar"].contains(&entity.kind.as_str()) {
            continue;
        }
        if let Some(Value::Text(name)) = entity.component("name") {
            names.insert(name.clone());
        }
    }
    names.into_iter().collect()
}

#[test]
fn every_name_the_harbour_can_produce_is_known_in_every_language() {
    let branch = crate::red_team::opened();
    let world = branch.world();
    let kit = crate::speech::kit(world.state());
    let mut missing = Vec::new();
    for name in produced() {
        // Only a name, not a day of no name ("the swallows").
        // Only a name in Latin letters, and not a day of no name ("the
        // swallows"), needs its forms in the other languages.
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
        for lang in ["en", "zh", "ja"] {
            let hearing = conversation::hearing_for(world, &kit, MARA, words(lang)).unwrap();
            let all = std::iter::once(name.clone())
                .chain(forms.iter().cloned())
                .collect::<Vec<_>>();
            for unknown in lexicon::unknown_names(&all, &hearing) {
                missing.push(format!("{name}: {unknown} unknown to a hearing in {lang}"));
            }
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
}

#[test]
fn none_of_the_harbours_own_lines_names_anyone_invented() {
    let branch = crate::red_team::opened();
    let world = branch.world();
    let kit = crate::speech::kit(world.state());
    let lines = lexicon::catalog_lines(
        &[("zh", crate::ZH_HANS), ("ja", crate::JA)],
        "Mara",
        crate::red_team::FIXTURES,
    );
    let failed = lexicon::invented_in_lines(&lines, |lang| {
        conversation::hearing_for(world, &kit, MARA, words(lang)).unwrap()
    });
    assert!(
        failed.is_empty(),
        "{} lines: {:#?}",
        failed.len(),
        &failed[..failed.len().min(80)]
    );
}
