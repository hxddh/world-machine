//! A harbour saved by every World Machine release since v0.20 opens,
//! replays event for event, reads back through its file unchanged, and
//! plays on.
//!
//! The fixtures were written on each release's own tree, in a scratch
//! worktree, by that release's fixture writer (`v022_worlds.rs` for v0.20
//! to v0.22: a warm player's first ninety days; `v023_worlds.rs` for v0.23
//! and v0.24: a builder's first 120 days). v0.20 and v0.21 record the same
//! events for that player, so their files are the same. They must never
//! be rewritten.

use tiny_society::TinySociety;
use world_document::WorldDocument;
use world_projection::ProjectionIntent::InvokeCommand;

const RELEASES: [&str; 5] = ["v020", "v021", "v022", "v023", "v024"];
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
        // Opened as the app opens a file: written before files said who
        // wrote them, so schema 0, and played on as usual.
        let path = format!(
            "{}/tests/fixtures/{release}-harbour.world",
            env!("CARGO_MANIFEST_DIR")
        );
        let opened = world_library::DurableWorldSession::open_file(path.into(), &registry).unwrap();
        assert_eq!(opened.writer().schema, 0, "{release}");
        assert_eq!(opened.read_only_reason(), None, "{release}");
        assert_eq!(
            opened.snapshot(),
            registry.open_archive(&archive).unwrap().snapshot()
        );
    }
}
