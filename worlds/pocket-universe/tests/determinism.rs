//! Every place is deterministic (invariant 6): played now by the builder,
//! its recorded events rebuild the very World that was played, with
//! nothing decided again, and played on from its save it records what it
//! records played straight through (see
//! `world_pack_testkit::replay::assert_fresh_world_replays`).

use pocket_universe::{
    PocketUniverse, NUDGE_COMMAND, POCKET_UNIVERSE_PACK_ID, SEED_1980S_TOWN_COMMAND,
    SEED_MARS_COLONY_COMMAND, SEED_PENGUIN_CIVILIZATION_COMMAND,
};

const SEEDS: [&str; 3] = [
    SEED_MARS_COLONY_COMMAND,
    SEED_1980S_TOWN_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
];

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    registry
}

#[test]
fn every_fresh_place_replays_to_the_world_it_played() {
    let registry = registry();
    for seed in SEEDS {
        world_pack_testkit::replay::assert_fresh_world_replays(
            &registry,
            POCKET_UNIVERSE_PACK_ID,
            &[seed],
            NUDGE_COMMAND,
            36,
            6,
        );
    }
}

/// Its state, rebuilt from its events alone through the Pack's own restore,
/// is the state it was played to, and replays to itself again.
#[test]
fn every_fresh_places_events_rebuild_its_state() {
    for seed in SEEDS {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        for _ in 0..30 {
            universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
        }
        let archive = universe.archive().unwrap();
        let resumed = PocketUniverse::resume_archive(&archive).unwrap();
        assert_eq!(resumed.archive().unwrap().events, archive.events, "{seed}");
        assert_eq!(resumed.world().state(), universe.world().state(), "{seed}");
        let replayed = resumed.world().replay().unwrap();
        assert_eq!(replayed.state(), resumed.world().state(), "{seed}");
        assert_eq!(
            resumed.projection_snapshot(),
            universe.projection_snapshot(),
            "{seed}"
        );
    }
}
