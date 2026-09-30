//! Places saved by World Machine v0.23, with plots built, things made by
//! hand and questions answered, still open, replay event for event and play
//! on under today's story rules.
//!
//! The fixtures were written by `write_the_v023_fixtures` on the v0.23 tree
//! (commit f6701f6: a builder's first 120 periods in each place) and must
//! never be rewritten.

use pocket_universe::{
    PocketUniverse, NUDGE_COMMAND, POCKET_UNIVERSE_PACK_VERSION, SEED_1980S_TOWN_COMMAND,
    SEED_MARS_COLONY_COMMAND, SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use world_document::WorldDocument;

const PLACES: [(&str, &str); 3] = [
    ("mars", SEED_MARS_COLONY_COMMAND),
    ("maple", SEED_1980S_TOWN_COMMAND),
    ("ice", SEED_PENGUIN_CIVILIZATION_COMMAND),
];

fn fixture(place: &str) -> String {
    format!(
        "{}/tests/fixtures/v023-{place}.world",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// The builder: the first question each period; every third period a plot
/// built when one is on offer, or else something made by hand; the period
/// let pass.
fn builder(universe: &mut PocketUniverse, periods: usize) {
    for period in 1..=periods {
        let snapshot = universe.projection_snapshot();
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != NUDGE_COMMAND)
        {
            let _ = universe.invoke_projection_command(&answer.id.clone());
        }
        if period % 3 == 0 {
            let snapshot = universe.projection_snapshot();
            let plot = snapshot
                .canvas
                .plots
                .iter()
                .flat_map(|plot| plot.offers.iter())
                .find(|offer| offer.unavailable.is_none())
                .map(|offer| offer.command.clone());
            let deed = plot.or_else(|| {
                snapshot
                    .commands
                    .iter()
                    .filter(|c| c.unavailable.is_none())
                    .find(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
                    .map(|c| c.id.clone())
            });
            if let Some(deed) = deed {
                let _ = universe.invoke_projection_command(&deed);
            }
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
}

/// Writes the fixtures. Run once, on the v0.23 tree only:
/// `WRITE_V023_FIXTURE=1 cargo test -p pocket-universe --test v023_worlds -- --ignored`.
#[test]
#[ignore]
fn write_the_v023_fixtures() {
    if std::env::var_os("WRITE_V023_FIXTURE").is_none() {
        return;
    }
    for (place, seed) in PLACES {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        builder(&mut universe, 120);
        let archive = universe.archive().unwrap();
        std::fs::write(
            fixture(place),
            WorldDocument::new(archive).to_bytes().unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn v023_places_open_replay_and_play_on() {
    for (place, _) in PLACES {
        let bytes = std::fs::read(fixture(place)).expect("the v0.23 fixture");
        let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
        assert_eq!(
            archive.pack.version, POCKET_UNIVERSE_PACK_VERSION,
            "{place}"
        );
        assert!(
            archive
                .events
                .iter()
                .any(|event| event.kind == "plot_finished"),
            "{place}: a builder's place"
        );
        let mut universe = PocketUniverse::resume_archive(&archive).unwrap();
        assert_eq!(
            universe.archive().unwrap().events,
            archive.events,
            "{place}"
        );
        let replayed = universe.world().replay().unwrap();
        assert_eq!(replayed.state(), universe.world().state(), "{place}");
        let before = archive.events.len();
        builder(&mut universe, 15);
        let after = universe.archive().unwrap();
        assert!(after.events.len() > before, "{place}");
        assert_eq!(after.events[..before], archive.events[..], "{place}");
        let reopened = PocketUniverse::resume_archive(&after).unwrap();
        assert_eq!(
            reopened.projection_snapshot(),
            universe.projection_snapshot(),
            "{place}"
        );
    }
}
