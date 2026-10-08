//! The kernel's invariants, checked against the harbour's saved Worlds
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
            tiny_society::TinySociety::resume_archive(archive)
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
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let events = world_pack_testkit::invariants::played_events(
        &registry,
        &root.join("tests/fixtures/v026-harbour.world"),
        "tiny-society.let-day-pass",
        30,
    );
    world_pack_testkit::invariants::assert_uncaused_kinds(
        "harbour v026 + 30 days",
        &events,
        &root.join("tests/golden/uncaused-kinds.txt"),
    );
}
