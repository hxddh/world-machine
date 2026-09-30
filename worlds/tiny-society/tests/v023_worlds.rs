//! A harbour saved by World Machine v0.23, with plots built, things made by
//! hand and questions answered, still opens, replays event for event and
//! plays on under today's story rules.
//!
//! The fixture was written by `write_the_v023_fixture` on the v0.23 tree
//! (commit f6701f6: a builder's first 120 days) and must never be rewritten.

use tiny_society::TinySociety;
use world_document::WorldDocument;
use world_projection::ProjectionIntent::InvokeCommand;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/v023-harbour.world"
);
const PASS: &str = "tiny-society.let-day-pass";

/// The builder: the first question each day; every third day a plot built
/// when one is on offer, or else something made by hand; the day let pass.
fn builder(session: &mut Box<dyn world_host::WorldSession>, days: usize) {
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
            let plot = snapshot
                .canvas
                .plots
                .iter()
                .flat_map(|plot| plot.offers.iter())
                .find(|offer| offer.unavailable.is_none())
                .map(|offer| offer.command.clone());
            let deed = plot.or_else(|| {
                snapshot
                    .commands
                    .iter()
                    .filter(|c| c.unavailable.is_none())
                    .find(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
                    .map(|c| c.id.clone())
            });
            if let Some(deed) = deed {
                let _ = session.handle(InvokeCommand(deed));
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

/// Writes the fixture. Run once, on the v0.23 tree only:
/// `WRITE_V023_FIXTURE=1 cargo test -p tiny-society --test v023_worlds -- --ignored`.
#[test]
#[ignore]
fn write_the_v023_fixture() {
    if std::env::var_os("WRITE_V023_FIXTURE").is_none() {
        return;
    }
    let registry = registry();
    let mut session = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
    builder(&mut session, 120);
    let archive = session.archive().unwrap().unwrap();
    std::fs::write(FIXTURE, WorldDocument::new(archive).to_bytes().unwrap()).unwrap();
}

#[test]
fn a_v023_harbour_opens_replays_and_plays_on() {
    let bytes = std::fs::read(FIXTURE).expect("the v0.23 fixture");
    let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
    assert_eq!(
        archive.pack.version,
        tiny_society::TINY_SOCIETY_PACK_VERSION,
        "the Pack version is unchanged since v0.23"
    );
    // Replayed through today's Pack, it is exactly the World it was.
    let society = TinySociety::resume_archive(&archive).unwrap();
    assert_eq!(society.archive().unwrap().events, archive.events);
    let replayed = society.world().replay().unwrap();
    assert_eq!(replayed.state(), society.world().state());
    // It was a builder's World: plots finished and things made by hand.
    let kinds = |kind: &str| {
        archive
            .events
            .iter()
            .filter(|event| event.kind == kind)
            .count()
    };
    assert!(kinds("plot_finished") > 0);
    // And it opens as a session and plays on under today's rules.
    let registry = registry();
    let mut session = registry.open_archive(&archive).unwrap();
    let before = session.archive().unwrap().unwrap().events.len();
    assert_eq!(before, archive.events.len());
    builder(&mut session, 15);
    let snapshot = session.snapshot();
    assert!(!snapshot.canvas.items.is_empty());
    let after = session.archive().unwrap().unwrap();
    assert!(after.events.len() > before);
    assert_eq!(after.events[..before], archive.events[..]);
    let reopened = registry.open_archive(&after).unwrap();
    assert_eq!(reopened.snapshot(), snapshot);
}
