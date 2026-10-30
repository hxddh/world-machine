//! Determinism, on Worlds made now (World Machine 1.0).
//!
//! A fresh World is made and played by a fixed, deterministic builder;
//! then its recorded events are replayed, with nothing decided again, and
//! must give back the very World that was played: the same events, the
//! same state, the same snapshot (and the same snapshot wire JSON), looked
//! at once or twice. Played on from the reopened World, it must record
//! what the World played straight through records.
//!
//! Until 1.0 the same was checked against saved Worlds from every v0.x
//! release and digests of what they showed; 1.0 starts its own save
//! format, and the saves a release ships are checked by
//! [`crate::release`].

use world_host::{WorldRegistry, WorldSession};
use world_pack_protocol::ProjectionSnapshotWire;
use world_projection::ProjectionIntent::InvokeCommand;
use world_projection::ProjectionSnapshot;

/// The builder every determinism test uses: the first open question each
/// day; every third day a plot built when one is on offer, or else
/// something made by hand; then the day let pass with `pass`.
pub fn builder(session: &mut Box<dyn WorldSession>, pass: &str, days: usize) {
    for day in 1..=days {
        builder_day(session, pass, day, false);
    }
}

/// One builder day. With `strict`, a command the snapshot offered as
/// available must be accepted, so an answer or deed that has stopped
/// working is noticed.
pub fn builder_day(session: &mut Box<dyn WorldSession>, pass: &str, day: usize, strict: bool) {
    let snapshot = session.snapshot();
    if let Some(answer) = snapshot
        .commands
        .iter()
        .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != pass)
    {
        let taken = session.handle(InvokeCommand(answer.id.clone()));
        if strict {
            if let Err(error) = taken {
                panic!(
                    "day {day}: the available answer {} was refused: {error:?}",
                    answer.id
                );
            }
        }
    }
    if day.is_multiple_of(3) {
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
            let done = session.handle(InvokeCommand(deed.clone()));
            if strict {
                if let Err(error) = done {
                    panic!("day {day}: the available deed {deed} was refused: {error:?}");
                }
            }
        }
    }
    session
        .handle(InvokeCommand(pass.into()))
        .expect("the day passes");
}

/// A new World of `pack_id`, begun with the commands in `begin` (where it
/// begins, say).
pub fn begun(registry: &WorldRegistry, pack_id: &str, begin: &[&str]) -> Box<dyn WorldSession> {
    let mut session = registry.create(pack_id).expect("a new World");
    for command in begin {
        session
            .handle(InvokeCommand((*command).into()))
            .expect("the World begins");
    }
    session
}

/// Snapshots of a new World of `pack_id` (begun with `begin`) played by the
/// [`builder`], taken after each of the days in `ages` (in order): what a
/// lived World looks like at those ages, for tests that once read saved
/// Worlds. Each is labelled with its day.
pub fn lived(
    registry: &WorldRegistry,
    pack_id: &str,
    begin: &[&str],
    pass: &str,
    ages: &[usize],
) -> Vec<(String, ProjectionSnapshot)> {
    let mut session = begun(registry, pack_id, begin);
    let mut played = 0;
    let mut out = Vec::new();
    for &age in ages {
        assert!(age >= played, "ages in order");
        for day in played + 1..=age {
            builder_day(&mut session, pass, day, false);
        }
        played = age;
        out.push((format!("day {age}"), session.snapshot()));
    }
    out
}

/// A snapshot's process wire form (`ProjectionSnapshotWire`, what a Pack
/// process sends), written as JSON and read back: the JSON decodes to the
/// same wire value, and that to the same snapshot.
pub fn assert_wire_decodes(snapshot: &ProjectionSnapshot, at: &str) {
    let wire = ProjectionSnapshotWire::from(snapshot);
    let json = serde_json::to_string(&wire).expect("snapshot JSON");
    let decoded: ProjectionSnapshotWire =
        serde_json::from_str(&json).unwrap_or_else(|error| panic!("{at}: {error}"));
    assert!(
        decoded == wire,
        "{at}: the wire JSON reads back differently"
    );
    let back =
        ProjectionSnapshot::try_from(decoded).unwrap_or_else(|error| panic!("{at}: {error:?}"));
    assert!(
        back == *snapshot,
        "{at}: the wire form decodes to another snapshot"
    );
}

/// The snapshot's wire JSON, as a Pack process sends it.
fn wire_json(snapshot: &ProjectionSnapshot) -> String {
    serde_json::to_string(&ProjectionSnapshotWire::from(snapshot)).expect("snapshot JSON")
}

/// A new World of `pack_id` (begun with `begin`), played by the [`builder`]
/// strictly for `days` days, is deterministic:
///
/// - every day's snapshot is the same looked at again, and its wire JSON
///   decodes back to it;
/// - the World reopened from its own recorded events (nothing decided
///   again) replays them event for event and shows the very snapshot, and
///   the very snapshot JSON, it showed, every time it is reopened;
/// - played on `days_on` more days from the reopened World, it records
///   exactly what the World played straight through records, and shows the
///   same.
pub fn assert_fresh_world_replays(
    registry: &WorldRegistry,
    pack_id: &str,
    begin: &[&str],
    pass: &str,
    days: usize,
    days_on: usize,
) {
    let mut session = begun(registry, pack_id, begin);
    for day in 1..=days {
        builder_day(&mut session, pass, day, true);
        let snapshot = session.snapshot();
        assert!(
            session.snapshot() == snapshot,
            "day {day}: the same looked at again"
        );
        assert_wire_decodes(&snapshot, &format!("day {day}"));
    }
    let archive = session
        .archive()
        .expect("the World archives")
        .expect("the World keeps an archive");
    assert!(!archive.events.is_empty(), "playing records events");
    let shown = session.snapshot();
    let mut reopened = registry
        .open_archive(&archive)
        .unwrap_or_else(|error| panic!("the World reopens: {error:?}"));
    let replayed = reopened.archive().unwrap().unwrap();
    assert!(
        replayed.events == archive.events,
        "replayed event for event"
    );
    assert!(
        reopened.snapshot() == shown,
        "reopened, it shows what it showed"
    );
    assert_eq!(
        wire_json(&reopened.snapshot()),
        wire_json(&shown),
        "reopened, its snapshot JSON is byte for byte what it was"
    );
    assert!(
        registry.open_archive(&archive).unwrap().snapshot() == shown,
        "it reopens to the same snapshot each time"
    );
    // Played on from where it was saved, and straight through: the same.
    for day in days + 1..=days + days_on {
        builder_day(&mut session, pass, day, true);
        builder_day(&mut reopened, pass, day, true);
    }
    let straight = session.archive().unwrap().unwrap();
    let resumed = reopened.archive().unwrap().unwrap();
    assert!(
        straight.events.len() > archive.events.len() || days_on == 0,
        "playing on records events"
    );
    assert!(
        straight.events[..archive.events.len()] == archive.events[..],
        "playing on only appends"
    );
    assert!(
        resumed.events == straight.events,
        "played on from its save, it records what it records played straight through"
    );
    assert!(
        reopened.snapshot() == session.snapshot(),
        "and shows the same"
    );
}
