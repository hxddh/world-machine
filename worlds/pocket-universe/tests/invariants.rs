//! The kernel's invariants, checked against the saved places
//! (world-pack-testkit/src/invariants.rs).

use std::path::Path;

/// Invariant 6: each fixture's recorded events rebuild the same World they
/// always did, with nothing decided again. The golden file is never
/// re-blessed.
#[test]
fn every_fixture_rebuilds_from_its_events_to_the_state_it_always_had() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    world_pack_testkit::invariants::assert_replayed_state_never_changes(
        &root.join("tests/fixtures"),
        &root.join("tests/golden/replayed-state.digest"),
        |archive| {
            pocket_universe::PocketUniverse::resume_archive(archive)
                .map(|world| world.world().state().clone())
                .map_err(|error| error.to_string())
        },
    );
}

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    registry
}

/// Invariant 5: every event a place records names its cause, unless it is
/// of a kind declared a root in `tests/root-kinds.txt` (the player's deeds,
/// a period passing, the place's beginning), and roots are under a tenth of
/// what happens. Each fixture's own recorded events keep their old form;
/// only what is played on is checked.
#[test]
fn every_event_played_on_names_its_cause_or_is_a_declared_root() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registry = registry();
    for place in ["mars", "maple", "ice"] {
        let events = world_pack_testkit::invariants::played_events(
            &registry,
            &root.join(format!("tests/fixtures/v026-{place}.world")),
            pocket_universe::NUDGE_COMMAND,
            30,
        );
        world_pack_testkit::invariants::assert_caused(
            &format!("{place} v026 + 30 periods"),
            &events,
            &root.join("tests/root-kinds.txt"),
        );
    }
}

/// The same over a new Maple Street's first year, from its first event.
#[test]
fn a_new_maple_streets_first_year_names_its_causes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let events = world_pack_testkit::invariants::created_events(
        &registry(),
        pocket_universe::POCKET_UNIVERSE_PACK_ID,
        &[pocket_universe::SEED_1980S_TOWN_COMMAND],
        pocket_universe::NUDGE_COMMAND,
        120,
    );
    world_pack_testkit::invariants::assert_caused(
        "a new Maple Street + 120 periods",
        &events,
        &root.join("tests/root-kinds.txt"),
    );
}
