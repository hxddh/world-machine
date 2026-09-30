//! Every saved harbour in `tests/fixtures` replays event for event and plays
//! on to snapshots byte-identical with those the code gave before the v0.25
//! Pack kit refactor (`tests/golden/replay/*.digest`).

use std::path::Path;

#[test]
fn every_fixture_replays_to_identical_snapshots() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    world_pack_testkit::replay::assert_replay_identical(
        &registry,
        &root.join("tests/fixtures"),
        &root.join("tests/golden/replay"),
        "tiny-society.let-day-pass",
        30,
    );
}
