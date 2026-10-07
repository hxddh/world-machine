//! A harbour saved by every World Machine release since v0.20 opens,
//! replays event for event, reads back through its file unchanged, and
//! plays on.
//!
//! The fixtures were written on each release's own tree, in a scratch
//! worktree, by a fixture writer run there:
//! - v0.20 and v0.22: `v022_worlds.rs`, a warm player's first ninety days;
//! - v0.21: a "last" player's first 120 days (the last answer each day,
//!   something made by hand every other day), written as v0.21's
//!   `WorldDocument` writes a file (v0.27; v0.21's first fixture was the
//!   warm player's, whose events v0.21 recorded exactly as v0.20 did, so
//!   its file was v0.20's byte for byte);
//! - v0.23 and v0.24: `v023_worlds.rs`, a builder's first 120 days;
//! - v0.25 and v0.26 (v0.26.1): the same builder played the app's way, a
//!   Library World made by `DurableWorldSession::create` and changed by
//!   `handle`, so the file is exactly what that release's app wrote.
//!
//! No two are the same file ([`no_two_fixtures_are_the_same_file`]). They
//! must never be rewritten.

use tiny_society::TinySociety;
use world_document::WorldDocument;
use world_projection::ProjectionIntent::InvokeCommand;

const RELEASES: [&str; 7] = ["v020", "v021", "v022", "v023", "v024", "v025", "v026"];
const PASS: &str = "tiny-society.let-day-pass";

fn fixture(release: &str) -> Vec<u8> {
    let path = format!(
        "{}/tests/fixtures/{release}-harbour.world",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    registry
}

#[test]
fn a_harbour_from_every_release_opens_replays_and_plays_on() {
    let registry = registry();
    for release in RELEASES {
        let bytes = fixture(release);
        let document = WorldDocument::from_bytes(&bytes).unwrap();
        let archive = document.archive.clone();
        assert!(!archive.events.is_empty(), "{release}");
        assert_eq!(
            archive.pack.version,
            tiny_society::TINY_SOCIETY_PACK_VERSION,
            "{release}: the Pack version is unchanged"
        );
        // Replayed through today's Pack, it is exactly the World it was.
        let society = TinySociety::resume_archive(&archive).unwrap();
        assert_eq!(
            society.archive().unwrap().events,
            archive.events,
            "{release}"
        );
        let replayed = society.world().replay().unwrap();
        assert_eq!(replayed.state(), society.world().state(), "{release}");
        // Written as today's app writes it, it reads back the same.
        let rewritten = WorldDocument::from_bytes(&document.to_bytes().unwrap()).unwrap();
        assert_eq!(rewritten.archive.events, archive.events, "{release}");
        // It opens as a session and plays on, keeping everything it had.
        let mut session = registry.open_archive(&archive).unwrap();
        let before = archive.events.len();
        for _ in 0..3 {
            session.handle(InvokeCommand(PASS.into())).unwrap();
        }
        let after = session.archive().unwrap().unwrap();
        assert!(after.events.len() > before, "{release}");
        assert_eq!(after.events[..before], archive.events[..], "{release}");
        let reopened = registry.open_archive(&after).unwrap();
        assert_eq!(reopened.snapshot(), session.snapshot(), "{release}");
        // Opened as the app opens a file, and played on as usual: written
        // before files said who wrote them (schema 0), or by v0.25's app
        // or later, which says so.
        let path = format!(
            "{}/tests/fixtures/{release}-harbour.world",
            env!("CARGO_MANIFEST_DIR")
        );
        let opened = world_library::DurableWorldSession::open_file(path.into(), &registry).unwrap();
        let says = release >= "v025";
        assert_eq!(opened.writer().schema, u32::from(says), "{release}");
        assert_eq!(opened.writer().app.is_some(), says, "{release}");
        assert_eq!(opened.read_only_reason(), None, "{release}");
        assert_eq!(
            opened.snapshot(),
            registry.open_archive(&archive).unwrap().snapshot()
        );
    }
}

/// Every release's fixture is a World of its own: no two files are the
/// same (v0.20's and v0.21's were, until v0.27).
#[test]
fn no_two_fixtures_are_the_same_file() {
    world_pack_testkit::replay::assert_fixtures_differ(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures"
    )));
}
