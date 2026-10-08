//! Each place's words over three years, for the five players: nothing
//! said too often in year three, nothing told more than twice in a week
//! across cards, the return film, notes and strips, nothing sad in the
//! first five periods, no seam by period 1,080 and no engine word anywhere
//! (`world_pack_testkit::words`).
//!
//! `cargo test -p pocket-universe --lib words_tests -- --ignored --nocapture`;
//! `PLACES=mars` plays only that place, `PLAYERS=Last` only that player,
//! `WORDS_DAYS=360` fewer periods.

use crate::PocketUniverse;
use world_pack_testkit::players::{self, Player, PlayerWorld};
use world_pack_testkit::seams;
use world_pack_testkit::words::Words;

/// Plays `player` in the place `seed` makes for `days` periods, watching
/// every word.
pub(crate) fn words_of(place: &str, seed: &str, player: Player, days: usize) -> Words {
    let label = format!("{place} {player:?}");
    let mut words = Words::new(label.clone(), crate::first_minutes::UNKIND)
        .filler(&crate::handwork::filler())
        .filler(&["{said} Just like last year."]);
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    players::play_watched(
        label.clone(),
        player,
        days,
        universe,
        |day, universe, after| {
            let told =
                (day.day % 30 == 0 || day.day == days).then(|| crate::seams_tests::told(universe));
            words.watch(
                day.day,
                day.began,
                day.events_before,
                universe.world(),
                after,
                told,
                |since| universe.returned(since),
            );
            for letter in seams::self_told(universe.world(), after) {
                words.seams.insert(format!("{label}: {letter}"));
            }
            if day.day == days {
                words.look_at_strips(&crate::moments::moments(universe.world()), after);
                for moment in crate::moments::moments(universe.world()) {
                    if let Some(missing) = crate::seams_tests::unshown(&moment) {
                        words
                            .seams
                            .insert(format!("{label}: {:?} shows no {missing}", moment.title));
                    }
                }
            }
        },
    );
    words
}

fn days() -> usize {
    std::env::var("WORDS_DAYS")
        .ok()
        .and_then(|days| days.parse().ok())
        .unwrap_or(players::DAYS)
}

#[test]
#[ignore]
fn five_players_words_over_three_years_in_every_place() {
    let days = days();
    let places = std::env::var("PLACES").unwrap_or_default();
    let runs = crate::players::PLACES
        .into_iter()
        .filter(|(place, _)| places.is_empty() || places.contains(place))
        .flat_map(|(place, seed)| {
            players::players_asked()
                .into_iter()
                .map(move |player| (place, seed, player))
        })
        .collect::<Vec<_>>();
    let mut failed = Vec::new();
    for batch in runs.chunks(5) {
        let words = std::thread::scope(|scope| {
            batch
                .iter()
                .map(|(place, seed, player)| {
                    std::thread::Builder::new()
                        .stack_size(64 << 20)
                        .spawn_scoped(scope, move || words_of(place, seed, *player, days))
                        .unwrap()
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        for words in &words {
            failed.extend(words.report());
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// The five players' first month reads well in every place: nothing sad
/// or unkind in the first periods, no engine word and no seam.
#[test]
fn the_first_month_reads_well_for_every_player_in_every_place() {
    let mut failed = Vec::new();
    for (place, seed) in crate::players::PLACES {
        for player in players::PLAYERS {
            let words = words_of(place, seed, player, 30);
            failed.extend(words.sad.iter().cloned());
            failed.extend(words.engine.iter().cloned());
            failed.extend(words.seams.iter().cloned());
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

/// No two places open their people's troubles in the same words: the
/// first of each kind of trouble put to the player reads differently in
/// each place.
#[test]
fn each_place_puts_its_first_troubles_in_its_own_words() {
    use std::collections::BTreeMap;
    let mut first = BTreeMap::<String, Vec<(String, String)>>::new();
    for (place, seed) in crate::players::PLACES {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        for _ in 0..60 {
            universe
                .invoke_projection_command(crate::NUDGE_COMMAND)
                .unwrap();
        }
        let world = universe.world();
        let mut seen = std::collections::BTreeSet::new();
        for event in world.events_of_kind(&["situation_came_up"]) {
            let (Some(world_core::Value::Text(kind)), Some(world_core::Value::Text(said))) =
                (event.payload.get("kind"), event.payload.get("said"))
            else {
                continue;
            };
            if !seen.insert(kind.clone()) {
                continue;
            }
            // The words around the names.
            let mut words = said.clone();
            for who in event.actor.into_iter().chain(event.targets.iter().copied()) {
                words = words.replace(&lives::first_name(world.state(), who), "X");
            }
            first
                .entry(kind.clone())
                .or_default()
                .push((place.to_string(), words));
        }
    }
    for (kind, opened) in &first {
        for (at, (place, words)) in opened.iter().enumerate() {
            for (other, theirs) in &opened[at + 1..] {
                assert_ne!(
                    words, theirs,
                    "{place} and {other} both open a {kind} with {words:?}"
                );
            }
        }
    }
    assert!(!first.is_empty(), "no troubles in sixty periods");
}
