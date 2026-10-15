//! The oldest and the newest saved harbours in `tests/fixtures` replay event
//! for event and play on to snapshots byte-identical with those the code
//! gave before the v0.25 Pack kit refactor (`tests/golden/replay/*.digest`).
//! Every saved harbour replays and plays on without a golden file
//! ([`every_fixture_replays_and_plays_on`]); until REVIEW_v0.29 S8 each had
//! digests of its own.

use std::path::Path;

/// The fixtures whose play-on is pinned to golden digests: the oldest and
/// the newest.
const GOLDEN_FIXTURES: [&str; 2] = ["v020-harbour", "v027-harbour"];
const PASS: &str = "tiny-society.let-day-pass";

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
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
        PASS,
        30,
    );
}

/// Every saved harbour replays event for event, plays on three days by the
/// builder, only appends to its events, shows snapshots whose wire JSON
/// decodes back to them, and reopens to what it showed.
#[test]
fn every_fixture_replays_and_plays_on() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    world_pack_testkit::replay::assert_every_fixture_replays_and_continues(
        &registry(),
        &root.join("tests/fixtures"),
        PASS,
        3,
    );
}
