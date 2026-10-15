//! The oldest and the newest saved places in `tests/fixtures` show the same
//! snapshot JSON, byte for byte, as the code gave before the v0.25 speed
//! work (`tests/golden/snapshot_json/*.digest`; see
//! `world_pack_testkit::snapshot_json`). Every place's wire JSON is checked
//! to decode back to its snapshot in `replay_identical.rs`.

use std::path::Path;

#[test]
fn the_oldest_and_newest_fixtures_show_the_same_snapshot_json() {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    world_pack_testkit::snapshot_json::assert_snapshot_json_identical(
        &registry,
        &root.join("tests/fixtures"),
        &root.join("tests/golden/snapshot_json"),
        &["v020-ice", "v027-mars"],
        pocket_universe::NUDGE_COMMAND,
        12,
    );
}
