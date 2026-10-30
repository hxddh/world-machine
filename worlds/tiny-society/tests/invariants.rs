//! The kernel's invariant 5, checked against the harbour
//! (world-pack-testkit/src/invariants.rs). Invariant 6 is checked in
//! `determinism.rs`.

use std::path::Path;

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    registry
}

/// Invariant 5: every event a new harbour records over its first year
/// names its cause, unless it is of a kind declared a root in
/// `tests/root-kinds.txt` (the player's deeds, a day passing, the harbour's
/// beginning), and roots are under a tenth of what happens.
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
