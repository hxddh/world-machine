//! A harbour is deterministic (invariant 6): played now by the builder,
//! its recorded events rebuild the very World that was played, with
//! nothing decided again, and played on from its save it records what it
//! records played straight through (see
//! `world_pack_testkit::replay::assert_fresh_world_replays`).

use tiny_society::TinySociety;
use world_projection::ProjectionIntent::InvokeCommand;

const PASS: &str = "tiny-society.let-day-pass";

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(tiny_society::tiny_society_registration())
        .unwrap();
    registry
}

#[test]
fn a_fresh_harbour_replays_to_the_world_it_played() {
    world_pack_testkit::replay::assert_fresh_world_replays(
        &registry(),
        tiny_society::TINY_SOCIETY_PACK_ID,
        &[],
        PASS,
        45,
        6,
    );
}

/// Its state, rebuilt from its events alone through the Pack's own restore,
/// is the state it was played to, and replays to itself again.
#[test]
fn a_fresh_harbours_events_rebuild_its_state() {
    let registry = registry();
    let mut session = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
    world_pack_testkit::replay::builder(&mut session, PASS, 40);
    session.handle(InvokeCommand(PASS.into())).unwrap();
    let archive = session.archive().unwrap().unwrap();
    let society = TinySociety::resume_archive(&archive).unwrap();
    assert_eq!(society.archive().unwrap().events, archive.events);
    let replayed = society.world().replay().unwrap();
    assert_eq!(replayed.state(), society.world().state());
    // Written as a World file and read back, it is the same history.
    let document = world_document::WorldDocument::new(archive.clone());
    let read = world_document::WorldDocument::from_bytes(&document.to_bytes().unwrap()).unwrap();
    assert_eq!(read.archive.events, archive.events);
    let again = TinySociety::resume_archive(&read.archive).unwrap();
    assert_eq!(again.world().state(), society.world().state());
}
