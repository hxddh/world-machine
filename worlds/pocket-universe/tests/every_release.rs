//! Places saved by every World Machine release since v0.20 open, replay
//! event for event, read back through their files unchanged, and play on.
//!
//! The fixtures were written on each release's own tree, in a scratch
//! worktree, by a fixture writer run there:
//! - v0.20 and v0.22: `v022_worlds.rs`, a warm player's first ninety
//!   periods;
//! - v0.21: a "last" player's first 120 periods (the last answer each
//!   period, something made by hand every other one), written as v0.21's
//!   `WorldDocument` writes a file (v0.27; v0.21's first fixtures were the
//!   warm player's, the same files as v0.20's byte for byte);
//! - v0.23 and v0.24: `v023_worlds.rs`, a builder's first 120 periods;
//! - v0.25, v0.26 (v0.26.1) and v0.27: the same builder played the app's way, a
//!   Library World made by `DurableWorldSession::create` and changed by
//!   `handle`, so each file is exactly what that release's app wrote.
//!
//! No two are the same file ([`no_two_fixtures_are_the_same_file`]). They
//! must never be rewritten.

use pocket_universe::{PocketUniverse, NUDGE_COMMAND, POCKET_UNIVERSE_PACK_VERSION};
use world_document::WorldDocument;

const RELEASES: [&str; 8] = [
    "v020", "v021", "v022", "v023", "v024", "v025", "v026", "v027",
];
const PLACES: [&str; 3] = ["mars", "maple", "ice"];

fn fixture(release: &str, place: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/{release}-{place}.world",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

#[test]
fn every_place_from_every_release_opens_replays_and_plays_on() {
    for release in RELEASES {
        for place in PLACES {
            let at = format!("{release}-{place}");
            let document = WorldDocument::from_bytes(&fixture(release, place)).unwrap();
            let archive = document.archive.clone();
            assert!(!archive.events.is_empty(), "{at}");
            assert_eq!(
                archive.pack.version, POCKET_UNIVERSE_PACK_VERSION,
                "{at}: the Pack version is unchanged"
            );
            let mut universe = PocketUniverse::resume_archive(&archive).unwrap();
            assert_eq!(universe.archive().unwrap().events, archive.events, "{at}");
            let replayed = universe.world().replay().unwrap();
            assert_eq!(replayed.state(), universe.world().state(), "{at}");
            let rewritten = WorldDocument::from_bytes(&document.to_bytes().unwrap()).unwrap();
            assert_eq!(rewritten.archive.events, archive.events, "{at}");
            let before = archive.events.len();
            for _ in 0..3 {
                universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
            }
            let after = universe.archive().unwrap();
            assert!(after.events.len() > before, "{at}");
            assert_eq!(after.events[..before], archive.events[..], "{at}");
            let reopened = PocketUniverse::resume_archive(&after).unwrap();
            assert_eq!(
                reopened.projection_snapshot(),
                universe.projection_snapshot(),
                "{at}"
            );
        }
    }
}

/// Every release's fixtures are Worlds of their own: no two files are the
/// same (v0.20's and v0.21's were, until v0.27).
#[test]
fn no_two_fixtures_are_the_same_file() {
    world_pack_testkit::replay::assert_fixtures_differ(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures"
    )));
}
