//! Every saved place in `tests/fixtures` shows the same snapshot JSON, byte for
//! byte, as the code gave before the v0.25 speed work
//! (`tests/golden/snapshot_json/*.digest`; see `world_pack_testkit::snapshot_json`).

use std::path::Path;

#[test]
fn every_fixture_shows_the_same_snapshot_json() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    world_pack_testkit::snapshot_json::assert_snapshot_json_identical(
        &registry,
        &root.join("tests/fixtures"),
        &root.join("tests/golden/snapshot_json"),
        pocket_universe::NUDGE_COMMAND,
        12,
    );
}
