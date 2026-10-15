//! Places saved by World Machine v0.23, with plots built, things made by
//! hand and questions answered, still open, replay event for event and play
//! on under today's story rules.
//!
//! The fixtures were written by `write_the_v023_fixtures` on the v0.23 tree
//! (commit f6701f6: a builder's first 120 periods in each place) and must
//! never be rewritten.

use pocket_universe::{
    PocketUniverse, NUDGE_COMMAND, POCKET_UNIVERSE_PACK_VERSION, SEED_1980S_TOWN_COMMAND,
    SEED_MARS_COLONY_COMMAND, SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use world_document::WorldDocument;

const PLACES: [(&str, &str); 3] = [
    ("mars", SEED_MARS_COLONY_COMMAND),
    ("maple", SEED_1980S_TOWN_COMMAND),
    ("ice", SEED_PENGUIN_CIVILIZATION_COMMAND),
];

fn fixture(place: &str) -> String {
    format!(
        "{}/tests/fixtures/v023-{place}.world",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// The builder: the first question each period; every third period a plot
/// built when one is on offer, or else something made by hand; the period
/// let pass.
fn builder(universe: &mut PocketUniverse, periods: usize) {
    for period in 1..=periods {
        let snapshot = universe.projection_snapshot();
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != NUDGE_COMMAND)
        {
            let _ = universe.invoke_projection_command(&answer.id.clone());
        }
        if period % 3 == 0 {
            let snapshot = universe.projection_snapshot();
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
                let _ = universe.invoke_projection_command(&deed);
            }
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
}

/// Writes the fixtures. Run once, on the v0.23 tree only:
/// `WRITE_V023_FIXTURE=1 cargo test -p pocket-universe --test v023_worlds -- --ignored`.
#[test]
#[ignore]
fn write_the_v023_fixtures() {
    if std::env::var_os("WRITE_V023_FIXTURE").is_none() {
        return;
    }
    for (place, seed) in PLACES {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        builder(&mut universe, 120);
        let archive = universe.archive().unwrap();
        std::fs::write(
            fixture(place),
            WorldDocument::new(archive).to_bytes().unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn v023_places_open_replay_and_play_on() {
    for (place, _) in PLACES {
        let bytes = std::fs::read(fixture(place)).expect("the v0.23 fixture");
        let archive = WorldDocument::from_bytes(&bytes).unwrap().archive;
        assert_eq!(
            archive.pack.version, POCKET_UNIVERSE_PACK_VERSION,
            "{place}"
        );
        assert!(
            archive
                .events
                .iter()
                .any(|event| event.kind == "plot_finished"),
            "{place}: a builder's place"
        );
        let mut universe = PocketUniverse::resume_archive(&archive).unwrap();
        assert_eq!(
            universe.archive().unwrap().events,
            archive.events,
            "{place}"
        );
        let replayed = universe.world().replay().unwrap();
        assert_eq!(replayed.state(), universe.world().state(), "{place}");
        let before = archive.events.len();
        builder(&mut universe, 15);
        let after = universe.archive().unwrap();
        assert!(after.events.len() > before, "{place}");
        assert_eq!(after.events[..before], archive.events[..], "{place}");
        let reopened = PocketUniverse::resume_archive(&after).unwrap();
        assert_eq!(
            reopened.projection_snapshot(),
            universe.projection_snapshot(),
            "{place}"
        );
    }
}

/// Every saved place, opened, shows its people doing something at ten in
/// the morning: at least six of them out working, carrying, keeping a
/// stall, sitting on a bench or with someone.
#[test]
fn a_lived_place_has_people_doing_things_at_ten() {
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
        let universe = PocketUniverse::resume_archive(&archive).unwrap();
        let snapshot = universe.projection_snapshot();
        let busy = world_pack_testkit::town::busy_at(&snapshot, 10);
        assert!(busy >= 6, "{}: {busy} busy at ten", fixture.display());
    }
}

/// The S2 bar, on screen (v0.30 S2d, S1c): every saved place, opened and
/// seen in the World window with the camera at rest at each hour from nine
/// to four, has at least six residents on its first screen seen doing
/// something (working, keeping a stall, carrying, sitting where their day
/// has them) or with someone, or where fewer than six are out of doors in
/// the whole place, every one of them. Measured on the window's own layout
/// and opening view, the hour pinned. (Until S1c it was six or everyone on
/// the screen, however few that was.)
#[test]
fn six_residents_busy_on_first_screen_by_day() {
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
        let universe = PocketUniverse::resume_archive(&archive).unwrap();
        let snapshot = universe.projection_snapshot();
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
    // The places of a keeper or two open on two to seven residents; the
    // grown ones on six or more.
    assert!(
        full > 0 && seen > 0,
        "{full} of {seen} first screens with six busy"
    );
}

/// The first screen at rest, morning to night (v0.30 S1c round 2): in every
/// fixture, and on Ares two weeks in, at every hour from nine to ten at
/// night, the window's opening view meets the art director's bar
/// (`world_gpui::first_screen_bar`): six busy by day, or everyone out; six
/// by lamplight of an evening, or everyone not abed; no rows turned to the
/// camera, one talking at a time, most turned to whoever they are with;
/// and nobody anywhere on or in a thing.
#[test]
fn the_first_screen_at_rest_morning_to_night() {
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
            let universe = PocketUniverse::resume_archive(&archive).unwrap();
            (
                fixture.display().to_string(),
                universe.projection_snapshot(),
            )
        })
        .collect::<Vec<_>>();
    // Ares two weeks in: six colonists, and the art director's first
    // screen.
    let mut ares = PocketUniverse::new().unwrap();
    ares.invoke_projection_command(SEED_MARS_COLONY_COMMAND)
        .unwrap();
    builder(&mut ares, 14);
    places.push(("ares day 14".into(), ares.projection_snapshot()));
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
