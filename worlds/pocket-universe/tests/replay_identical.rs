//! The oldest and the newest saved places in `tests/fixtures` replay event
//! for event and play on to snapshots byte-identical with those the code
//! gave before the v0.25 Pack kit refactor (`tests/golden/replay/*.digest`).
//! Every saved place replays and plays on without a golden file
//! ([`every_fixture_replays_and_plays_on`]); until REVIEW_v0.29 S8 each had
//! digests of its own.

use std::path::Path;

/// The fixtures whose play-on is pinned to golden digests: the oldest and
/// the newest (the first and the last in file order, so two of the three
/// places).
const GOLDEN_FIXTURES: [&str; 2] = ["v020-ice", "v027-mars"];

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    registry
}

#[test]
fn the_oldest_and_newest_fixtures_replay_to_identical_snapshots() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    world_pack_testkit::replay::assert_replay_identical(
        &registry(),
        &root.join("tests/fixtures"),
        &root.join("tests/golden/replay"),
        &GOLDEN_FIXTURES,
        pocket_universe::NUDGE_COMMAND,
        30,
    );
}

/// Every saved place replays event for event, plays on three periods by the
/// builder, only appends to its events, shows snapshots whose wire JSON
/// decodes back to them, and reopens to what it showed.
#[test]
fn every_fixture_replays_and_plays_on() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    world_pack_testkit::replay::assert_every_fixture_replays_and_continues(
        &registry(),
        &root.join("tests/fixtures"),
        pocket_universe::NUDGE_COMMAND,
        3,
    );
}
