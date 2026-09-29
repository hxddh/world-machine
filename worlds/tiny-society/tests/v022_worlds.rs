//! A harbour saved by World Machine v0.22, before plots, designs and names
//! existed, still opens, replays event for event and plays on.
//!
//! The fixture was written by `write_the_v022_fixture` on the v0.22.0 tree
//! (a warm player's first ninety days) and must never be rewritten.

use tiny_society::TinySociety;
use world_document::WorldDocument;
use world_projection::ProjectionIntent::InvokeCommand;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/v022-harbour.world"
);
const PASS: &str = "tiny-society.let-day-pass";

/// The warm player: the first question each day, something made every
/// third day, and the day let pass.
fn warm(session: &mut Box<dyn world_host::WorldSession>, days: usize) {
    for day in 1..=days {
        let snapshot = session.snapshot();
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
        {
            let _ = session.handle(InvokeCommand(answer.id.clone()));
        }
        if day % 3 == 0 {
            let snapshot = session.snapshot();
            if let Some(deed) = snapshot
                .commands
                .iter()
                .filter(|c| c.unavailable.is_none())
                .find(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
            {
                let _ = session.handle(InvokeCommand(deed.id.clone()));
            }
        }
        session.handle(InvokeCommand(PASS.into())).unwrap();
    }
}

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    registry
}

/// Writes the fixture. Run once, on the v0.22.0 tree only:
/// `WRITE_V022_FIXTURE=1 cargo test -p tiny-society --test v022_worlds -- --ignored`.
#[test]
#[ignore]
fn write_the_v022_fixture() {
    if std::env::var_os("WRITE_V022_FIXTURE").is_none() {
        return;
    }
    let registry = registry();
    let mut session = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
    warm(&mut session, 90);
    let archive = session.archive().unwrap().unwrap();
    std::fs::write(FIXTURE, WorldDocument::new(archive).to_bytes().unwrap()).unwrap();
}

#[test]
fn a_v022_harbour_opens_replays_and_plays_on() {
    let bytes = std::fs::read(FIXTURE).expect("the v0.22 fixture");
    let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
    // Replayed through today's Pack, it is exactly the World it was.
    let society = TinySociety::resume_archive(&archive).unwrap();
    assert_eq!(society.archive().unwrap().events, archive.events);
    let replayed = society.world().replay().unwrap();
    assert_eq!(replayed.state(), society.world().state());
    // And it opens as a session and plays on, plots and all.
    let registry = registry();
    let mut session = registry.open_archive(&archive).unwrap();
    let before = session.archive().unwrap().unwrap().events.len();
    assert_eq!(before, archive.events.len());
    warm(&mut session, 10);
    let snapshot = session.snapshot();
    assert!(!snapshot.canvas.items.is_empty());
    let after = session.archive().unwrap().unwrap();
    assert_eq!(after.events[..before], archive.events[..]);
    let reopened = registry.open_archive(&after).unwrap();
    assert_eq!(reopened.snapshot(), snapshot);
}
