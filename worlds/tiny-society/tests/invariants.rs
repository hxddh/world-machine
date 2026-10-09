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

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    registry
}

/// Invariant 5: every event the harbour records names its cause, unless it
/// is of a kind declared a root in `tests/root-kinds.txt` (the player's
/// deeds, a day passing, the harbour's beginning), and roots are under a
/// tenth of what happens. The fixture's own recorded events keep their old
/// form; only what is played on is checked.
#[test]
fn every_event_played_on_names_its_cause_or_is_a_declared_root() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let events = world_pack_testkit::invariants::played_events(
        &registry(),
        &root.join("tests/fixtures/v026-harbour.world"),
        "tiny-society.let-day-pass",
        30,
    );
    world_pack_testkit::invariants::assert_caused(
        "harbour v026 + 30 days",
        &events,
        &root.join("tests/root-kinds.txt"),
    );
}

/// The same over a new harbour's first year, from its first event.
#[test]
fn a_new_harbours_first_year_names_its_causes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let events = world_pack_testkit::invariants::created_events(
        &registry(),
        tiny_society::TINY_SOCIETY_PACK_ID,
        &[],
        "tiny-society.let-day-pass",
        120,
    );
    world_pack_testkit::invariants::assert_caused(
        "a new harbour + 120 days",
        &events,
        &root.join("tests/root-kinds.txt"),
    );
}
