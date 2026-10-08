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

/// Invariant 5: an event with no `caused_by` is of a kind listed in
/// `tests/golden/uncaused-kinds.txt`, a list that may only shrink; every
/// other event says what caused it.
#[test]
fn events_without_a_cause_are_only_of_kinds_that_start_a_chain() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut events = Vec::new();
    for place in ["mars", "maple", "ice"] {
        events.extend(world_pack_testkit::invariants::played_events(
            &registry,
            &root.join(format!("tests/fixtures/v026-{place}.world")),
            pocket_universe::NUDGE_COMMAND,
            30,
        ));
    }
    world_pack_testkit::invariants::assert_uncaused_kinds(
        "mars, maple and ice v026 + 30 days each",
        &events,
        &root.join("tests/golden/uncaused-kinds.txt"),
    );
}
