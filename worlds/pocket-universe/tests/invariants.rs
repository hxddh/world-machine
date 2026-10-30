//! The kernel's invariant 5, checked against the places
//! (world-pack-testkit/src/invariants.rs). Invariant 6 is checked in
//! `determinism.rs`.

use std::path::Path;

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    registry
}

/// Invariant 5: every event a new place records names its cause, unless it
/// is of a kind declared a root in `tests/root-kinds.txt` (the player's
/// deeds, a period passing, the place's beginning), and roots are under a
/// tenth of what happens: over each place's first thirty periods, and over
/// a new Maple Street's first year.
#[test]
fn every_new_place_names_its_causes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registry = registry();
    for (place, seed, periods) in [
        ("mars", pocket_universe::SEED_MARS_COLONY_COMMAND, 30),
        (
            "ice",
            pocket_universe::SEED_PENGUIN_CIVILIZATION_COMMAND,
            30,
        ),
        ("maple", pocket_universe::SEED_1980S_TOWN_COMMAND, 120),
    ] {
        let events = world_pack_testkit::invariants::created_events(
            &registry,
            pocket_universe::POCKET_UNIVERSE_PACK_ID,
            &[seed],
            pocket_universe::NUDGE_COMMAND,
            periods,
        );
        world_pack_testkit::invariants::assert_caused(
            &format!("a new {place} + {periods} periods"),
            &events,
            &root.join("tests/root-kinds.txt"),
        );
    }
}
