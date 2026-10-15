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

/// Every saved harbour, opened, shows its people doing something at ten in
/// the morning: at least six of them out working, carrying the catch,
/// keeping a stall, sitting on a bench or with someone.
#[test]
fn a_lived_harbour_has_people_doing_things_at_ten() {
    let registry = registry();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut fixtures = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "world"))
        .collect::<Vec<_>>();
    fixtures.sort();
    assert!(!fixtures.is_empty());
    for fixture in fixtures {
        let bytes = std::fs::read(&fixture).unwrap();
        let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
        let session = registry.open_archive(&archive).unwrap();
        let snapshot = session.snapshot();
        let busy = world_pack_testkit::town::busy_at(&snapshot, 10);
        assert!(busy >= 6, "{}: {busy} busy at ten", fixture.display());
    }
}

/// The S2 bar, on screen (v0.30 S2d, S1c): every saved harbour, opened and
/// seen in the World window with the camera at rest at each hour from nine
/// to four, has at least six residents on its first screen seen doing
/// something (working, keeping a stall, carrying, sitting where their day
/// has them) or with someone, or where fewer than six are out of doors in
/// the whole place, every one of them. Measured on the window's own layout
/// and opening view, the hour pinned. (Until S1c it was six or everyone on
/// the screen, however few that was.)
#[test]
fn six_residents_busy_on_first_screen_by_day() {
    let registry = registry();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut fixtures = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "world"))
        .collect::<Vec<_>>();
    fixtures.sort();
    assert!(!fixtures.is_empty());
    let (mut short, mut seen, mut full) = (Vec::new(), 0, 0);
    for fixture in fixtures {
        let bytes = std::fs::read(&fixture).unwrap();
        let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
        let session = registry.open_archive(&archive).unwrap();
        let snapshot = session.snapshot();
        for hour in 9..=16 {
            let screen = world_gpui::first_screen_busy(&snapshot, (1100.0, 848.0), hour);
            // Out of doors in the whole place at that hour, as the window
            // lays it out.
            let stage = world_gpui::diorama::stage_at(
                &snapshot,
                1100.0,
                848.0,
                world_gpui::diorama::Clock::at(hour),
            );
            let rest = world_gpui::diorama::Camera::rest_zoom(&stage);
            let outdoors = stage
                .people
                .iter()
                .filter(|spot| stage.shows(spot.index, rest))
                .count();
            seen += 1;
            full += usize::from(screen.busy() >= 6);
            // Two-sided: at least six busy, or all that are out where
            // fewer are; and nobody counted who is not out.
            if screen.busy() < 6.min(outdoors) || screen.residents.len() > outdoors {
                short.push(format!(
                    "{} at {hour}: {} busy of {:?}, {outdoors} out",
                    fixture.display(),
                    screen.busy(),
                    screen.residents
                ));
            }
        }
    }
    assert!(short.is_empty(), "{}", short.join("\n"));
    // And most first screens have six or more.
    assert!(
        full * 2 > seen,
        "{full} of {seen} first screens with six busy"
    );
}

/// The first screen at rest, morning to night (v0.30 S1c round 2): in every
/// fixture, and in a harbour on its first day, at every hour from nine to
/// ten at night, the window's opening view meets the art director's bar
/// (`world_gpui::first_screen_bar`): six busy by day, or everyone out; six
/// by lamplight of an evening, or everyone not abed; no rows turned to the
/// camera, one talking at a time, most turned to whoever they are with;
/// and nobody anywhere on or in a thing.
#[test]
fn the_first_screen_at_rest_morning_to_night() {
    let registry = registry();
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut fixtures = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "world"))
        .collect::<Vec<_>>();
    fixtures.sort();
    let mut places = fixtures
        .into_iter()
        .map(|fixture| {
            let bytes = std::fs::read(&fixture).unwrap();
            let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
            let session = registry.open_archive(&archive).unwrap();
            (fixture.display().to_string(), session.snapshot())
        })
        .collect::<Vec<_>>();
    // A harbour on its first day: eight people, and the art director's
    // first screen.
    let mut young = registry.create(tiny_society::TINY_SOCIETY_PACK_ID).unwrap();
    builder(&mut young, 1);
    places.push(("day 1".into(), young.snapshot()));
    let (mut short, mut evenings) = (Vec::new(), 0);
    for (name, snapshot) in &places {
        for hour in 9..=22 {
            match world_gpui::first_screen_bar(snapshot, (1100.0, 848.0), hour) {
                Ok(busy) => evenings += usize::from((19..=21).contains(&hour) && busy >= 6),
                Err(why) => short.push(format!("{name} at {hour}: {why}")),
            }
        }
    }
    assert!(short.is_empty(), "{}", short.join("\n"));
    // Most evenings are inhabited: six or more by lamplight.
    assert!(
        evenings * 2 > places.len() * 3,
        "{evenings} evenings of six"
    );
}
