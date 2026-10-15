//! One day a day (v0.30 S4) in a Pocket Universe: a new sitting brings the
//! place's morning (the ritual System), once a period, all of it following
//! from the period's beginning; and nothing of it is decided again when
//! the World is opened from its record.

use pocket_universe::{
    PocketUniverse, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use std::collections::BTreeSet;
use world_projection::SITTING_BEGINS_COMMAND;

#[test]
fn every_place_has_a_morning_a_sitting_a_period() {
    for seed in [
        SEED_MARS_COLONY_COMMAND,
        SEED_1980S_TOWN_COMMAND,
        SEED_PENGUIN_CIVILIZATION_COMMAND,
    ] {
        let mut world = PocketUniverse::new().unwrap();
        // Before a place is chosen there is no morning to have.
        assert!(world
            .invoke_projection_command(SITTING_BEGINS_COMMAND)
            .is_err());
        world.invoke_projection_command(seed).unwrap();
        // Its first period opens on its welcome, not a morning.
        assert!(world
            .invoke_projection_command(SITTING_BEGINS_COMMAND)
            .is_err());
        world.invoke_projection_command(NUDGE_COMMAND).unwrap();
        let mut lines = Vec::new();
        for _ in 0..4 {
            world.invoke_projection_command(NUDGE_COMMAND).unwrap();
            let came = world
                .invoke_projection_command(SITTING_BEGINS_COMMAND)
                .unwrap_or_else(|error| panic!("{seed}: no morning: {error}"));
            // Once a period: a second sitting the same period has none.
            assert!(world
                .invoke_projection_command(SITTING_BEGINS_COMMAND)
                .is_err());
            let state = world.world();
            let event = state.event(came).unwrap();
            assert_eq!(event.kind, ritual::MORNING_CAME);
            assert_eq!(
                state.event(event.caused_by[0]).unwrap().kind,
                "day_began",
                "{seed}: the morning follows the period's beginning"
            );
            let snapshot = world.projection_snapshot();
            let morning = snapshot.morning.expect("this period's morning");
            let find = morning.find.expect("a find");
            lines.push(find.line);
            lines.extend(morning.tended.map(|tended| tended.line));
            assert!(snapshot
                .book
                .iter()
                .any(|entry| entry.shelf == ritual::FINDS_SHELF));
        }
        let unique = lines.iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), lines.len(), "{seed}: {lines:#?}");
        // The record alone rebuilds it all, the mornings too.
        let replayed = world.world().replay().unwrap();
        assert_eq!(replayed.state(), world.world().state());
    }
}
