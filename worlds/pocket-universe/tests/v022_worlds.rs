//! Places saved by World Machine v0.22, before plots, designs and names
//! existed, still open, replay event for event and play on.
//!
//! The fixtures were written by `write_the_v022_fixtures` on the v0.22.0
//! tree (a warm player's first ninety periods in each place) and must
//! never be rewritten.

use pocket_universe::{
    PocketUniverse, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use world_document::WorldDocument;

const PLACES: [(&str, &str); 3] = [
    ("mars", SEED_MARS_COLONY_COMMAND),
    ("maple", SEED_1980S_TOWN_COMMAND),
    ("ice", SEED_PENGUIN_CIVILIZATION_COMMAND),
];

fn fixture(place: &str) -> String {
    format!(
        "{}/tests/fixtures/v022-{place}.world",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// The warm player: the first question each period, something made every
/// third period, and the period let pass.
fn warm(universe: &mut PocketUniverse, periods: usize) {
    for period in 1..=periods {
        let snapshot = universe.projection_snapshot();
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none())
        {
            let _ = universe.invoke_projection_command(&answer.id.clone());
        }
        if period % 3 == 0 {
            let snapshot = universe.projection_snapshot();
            if let Some(deed) = snapshot
                .commands
                .iter()
                .filter(|c| c.unavailable.is_none())
                .find(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
            {
                let _ = universe.invoke_projection_command(&deed.id.clone());
            }
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
}

/// Writes the fixtures. Run once, on the v0.22.0 tree only:
/// `WRITE_V022_FIXTURE=1 cargo test -p pocket-universe --test v022_worlds -- --ignored`.
#[test]
#[ignore]
fn write_the_v022_fixtures() {
    if std::env::var_os("WRITE_V022_FIXTURE").is_none() {
        return;
    }
    for (place, seed) in PLACES {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        warm(&mut universe, 90);
        let archive = universe.archive().unwrap();
        std::fs::write(
            fixture(place),
            WorldDocument::new(archive).to_bytes().unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn v022_places_open_replay_and_play_on() {
    for (place, _) in PLACES {
        let bytes = std::fs::read(fixture(place)).expect("the v0.22 fixture");
        let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
        let mut universe = PocketUniverse::resume_archive(&archive).unwrap();
        assert_eq!(
            universe.archive().unwrap().events,
            archive.events,
            "{place}"
        );
        let replayed = universe.world().replay().unwrap();
        assert_eq!(replayed.state(), universe.world().state(), "{place}");
        let before = archive.events.len();
        warm(&mut universe, 10);
        let after = universe.archive().unwrap();
        assert_eq!(after.events[..before], archive.events[..], "{place}");
        let reopened = PocketUniverse::resume_archive(&after).unwrap();
        assert_eq!(
            reopened.projection_snapshot(),
            universe.projection_snapshot(),
            "{place}"
        );
    }
}
