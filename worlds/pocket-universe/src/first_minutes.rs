//! A new player's first minutes in each place.

use crate::{
    PocketUniverse, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use world_projection::SelectionId;

const SEEDS: [&str; 3] = [
    SEED_MARS_COLONY_COMMAND,
    SEED_1980S_TOWN_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
];

/// Words a new player should not read in their first days: sorrow,
/// hardship, quarrels and being turned away.
pub(crate) const UNKIND: &[&str] = &[
    "sorry",
    "fell out",
    "had words",
    "shouting",
    "quarrel",
    "knows what they did",
    "done with",
    "don't talk to me",
    "are finished",
    "died",
    "passed away",
    "funeral",
    "goodbye",
    "farewell",
    "storm",
    "broken",
    "where were you",
    "sore.",
    "nobody listens",
    "lonely",
    "cried",
    "thinking of leaving",
];

fn unkind(line: &str) -> Option<&'static str> {
    let lower = line.to_lowercase();
    UNKIND.iter().copied().find(|word| lower.contains(word))
}

fn seeded(seed: &str) -> PocketUniverse {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    universe
}

/// Each place opens fair, with a hello before anything is asked.
#[test]
fn every_place_opens_fair_with_a_hello_first() {
    for seed in SEEDS {
        let universe = seeded(seed);
        let snapshot = universe.projection_snapshot();
        assert_eq!(snapshot.weather, world_projection::Weather::Clear, "{seed}");
        let events = universe.world().events();
        let hello = events.iter().position(|event| event.kind == "greeted");
        let asked = events.iter().position(|event| {
            event.payload.contains_key("storylet") || event.kind == "situation_arose"
        });
        assert!(hello.is_some(), "{seed}: nobody says hello");
        assert!(asked.is_none_or(|asked| hello < Some(asked)), "{seed}");
        let first = snapshot.voices.first().expect("someone speaks");
        assert!(
            events.iter().any(
                |event| event.kind == "greeted" && first.moment == SelectionId::Event(event.id)
            ),
            "{seed}: the first thing said is {:?}",
            first.line
        );
    }
}

/// The first build in each place goes up in scaffolding and is finished
/// the next period.
#[test]
fn every_first_build_shows_scaffolding_and_is_finished_the_next_period() {
    for seed in SEEDS {
        let mut universe = seeded(seed);
        let (build, plot) = universe
            .projection_snapshot()
            .canvas
            .plots
            .iter()
            .find_map(|plot| {
                plot.offers
                    .iter()
                    .find(|offer| offer.unavailable.is_none())
                    .map(|offer| (offer.command.clone(), plot.id.clone()))
            })
            .expect("a plot to build on");
        universe.invoke_projection_command(&build).unwrap();
        let work = hands::on_plots(universe.world().state())
            .into_iter()
            .find(|(on, _)| *on == plot)
            .map(|(_, id)| SelectionId::Entity(id))
            .unwrap();
        let shown = |universe: &PocketUniverse| {
            universe
                .projection_snapshot()
                .canvas
                .items
                .into_iter()
                .find(|item| item.id == work)
                .expect("on the scene")
        };
        assert_eq!(shown(&universe).art.as_deref(), Some("scaffold"), "{seed}");
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        let finished = shown(&universe);
        assert_ne!(finished.art.as_deref(), Some("scaffold"), "{seed}");
        assert!(finished.built.is_some(), "{seed}");
    }
}

/// Nothing sad or unkind is said to a new player in a place's first
/// days, whether they say yes or no.
#[test]
fn nothing_sad_or_unkind_in_any_places_first_days() {
    let mut found = std::collections::BTreeSet::new();
    for seed in SEEDS {
        for refusing in [false, true] {
            let mut universe = seeded(seed);
            for _ in 0..crate::arrival::FIRST_DAYS {
                let snapshot = universe.projection_snapshot();
                let asked = snapshot
                    .commands
                    .iter()
                    .filter(|c| c.question.is_some() && c.unavailable.is_none())
                    .map(|c| c.id.clone())
                    .collect::<Vec<_>>();
                let answer = if refusing {
                    asked.last()
                } else {
                    asked.first()
                };
                if let Some(answer) = answer {
                    universe.invoke_projection_command(answer).unwrap();
                }
                for line in crate::seams_tests::readable(&universe.projection_snapshot()) {
                    if let Some(word) = unkind(&line) {
                        found.insert(format!("{seed} {refusing}: {word:?} in {line:?}"));
                    }
                }
                universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
            }
        }
    }
    assert!(
        found.is_empty(),
        "{}",
        found.into_iter().collect::<Vec<_>>().join("\n")
    );
}

/// A place seeded before keeps its pace: a Mars colony saved by v0.23
/// has no first days kept, builds at the usual pace, and replays exactly.
#[test]
fn a_place_seeded_before_keeps_its_pace() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/v023-mars.world"
    ))
    .expect("the v0.23 Mars fixture");
    let archive = world_document::WorldDocument::from_bytes(&bytes)
        .unwrap()
        .archive;
    let universe = PocketUniverse::resume_archive(&archive).unwrap();
    let state = universe.world().state();
    assert_eq!(crate::arrival::arrived(state), None);
    assert!(!crate::arrival::first_days(state));
    assert_eq!(crate::handwork::kit(state).first_growing, None);
    assert_eq!(universe.archive().unwrap().events, archive.events);
    // A new one keeps its kind days only for its first days.
    let mut universe = seeded(SEED_MARS_COLONY_COMMAND);
    for _ in 0..crate::arrival::FIRST_DAYS {
        assert!(crate::arrival::first_days(universe.world().state()));
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    assert!(!crate::arrival::first_days(universe.world().state()));
}

/// In each place, saying yes to the first question puts something on the
/// scene at once, and whoever asked goes over to it.
#[test]
fn every_places_first_yes_is_seen_on_the_scene() {
    for seed in SEEDS {
        let mut universe = seeded(seed);
        let before = universe.projection_snapshot();
        let first = before
            .commands
            .iter()
            .find(|command| command.question.is_some() && command.unavailable.is_none())
            .expect("a first question")
            .clone();
        universe.invoke_projection_command(&first.id).unwrap();
        let after = universe.projection_snapshot();
        let new = after
            .canvas
            .items
            .iter()
            .filter(|item| !before.canvas.items.iter().any(|was| was.id == item.id))
            .map(|item| item.id)
            .collect::<Vec<_>>();
        assert!(!new.is_empty(), "{seed}: nothing new on the scene");
        let asker = first.asker.expect("someone asked");
        let now = after
            .canvas
            .items
            .iter()
            .find(|item| item.id == asker)
            .expect("the asker is on the scene");
        assert!(
            now.at.is_some_and(|at| new.contains(&at)),
            "{seed}: {} stays at {:?}",
            now.label,
            now.at
        );
    }
}
