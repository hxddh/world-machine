//! How the harbour's history is told over a long run (1.0 P4): chapters
//! past the twelfth are told as fully as the first, and what happens again
//! and again (a catch sold, a bench sat on) reads differently each time,
//! so a season of the same Event is not a season of the same line.

use crate::keeper::tests::warm_day;
use crate::projection::{narrated_title, CATCH_SOLD};
use crate::TinySocietyBranch;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// The days the warm player lives: past twenty chapters.
const DAYS: u64 = 220;

fn lived() -> &'static TinySocietyBranch {
    static LIVED: OnceLock<TinySocietyBranch> = OnceLock::new();
    LIVED.get_or_init(|| {
        let mut branch = TinySocietyBranch::new_world().unwrap();
        for day in 1..=DAYS {
            warm_day(&mut branch, day);
        }
        branch
    })
}

fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn sentences(text: &str) -> usize {
    text.matches(". ").count() + usize::from(text.ends_with('.'))
}

#[test]
fn chapters_past_the_twelfth_keep_their_length() {
    let chapters = lived().projection_snapshot().chapters;
    assert!(chapters.len() >= 18, "{} chapters", chapters.len());
    let mean = |chapters: &[world_projection::Chapter]| {
        chapters
            .iter()
            .map(|chapter| words(&chapter.summary))
            .sum::<usize>() as f64
            / chapters.len() as f64
    };
    let (early, late) = chapters.split_at(12);
    let (early_mean, late_mean) = (mean(early), mean(late));
    assert!(
        late_mean >= early_mean * 0.9,
        "chapters past 12 run to {late_mean:.0} words, the first twelve to {early_mean:.0}"
    );
    for chapter in late {
        assert!(
            sentences(&chapter.summary) >= crate::chapters::FULL && words(&chapter.summary) >= 25,
            "chapter {} is short: {}",
            chapter.number,
            chapter.summary
        );
    }
    // Filled out, a chapter still tells nothing twice.
    for chapter in &chapters {
        let told = chapter.summary.split(". ").collect::<Vec<_>>();
        let unique = told.iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), told.len(), "{}", chapter.summary);
    }
}

#[test]
fn what_happens_again_and_again_is_told_differently_each_time() {
    let world = lived().world();
    let mut told = BTreeMap::<&str, Vec<String>>::new();
    for event in world.events() {
        if let Some(title) = narrated_title(world, event) {
            told.entry(event.kind.as_str()).or_default().push(title);
        }
    }
    // A catch sold, day after day, never told the same way twice running,
    // and every way of telling it heard.
    let catches = &told["fish_sold"];
    assert!(catches.len() > CATCH_SOLD.len() * 2);
    assert!(catches.windows(2).all(|two| two[0] != two[1]));
    assert_eq!(
        catches.iter().collect::<BTreeSet<_>>().len(),
        CATCH_SOLD.len()
    );
    // Something the player made, used: never the same words twice running.
    let used = &told["enjoyed"];
    assert!(used.len() > 20, "{} uses", used.len());
    assert!(used.windows(2).all(|two| two[0] != two[1]), "{used:#?}");
    // Over all of it, at least half of what is told is told only once.
    let all = told.values().flatten().collect::<Vec<_>>();
    let distinct = all.iter().collect::<BTreeSet<_>>().len();
    assert!(
        distinct * 2 >= all.len(),
        "{distinct} of {} narrated summaries distinct",
        all.len()
    );
}

/// Every line chapters and the catch are told in is in the harbour's
/// Chinese and Japanese.
#[test]
fn every_new_line_of_the_history_is_in_both_catalogs() {
    for catalog in [crate::ZH_HANS, crate::JA] {
        let keys = catalog
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .map(|(english, _)| english)
            .collect::<BTreeSet<_>>();
        for line in crate::chapters::lines().into_iter().chain(CATCH_SOLD) {
            assert!(keys.contains(line), "untranslated: {line}");
        }
    }
}
