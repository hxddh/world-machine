//! Lived places, played now: a builder's place (the first question each
//! period, something built or made every third one) in each of the three
//! places at 90 and 120 periods (the ages the v0.x saves had), as the window
//! opens on each.
//! Until 1.0 these bars were held on places saved by v0.20–v0.27; 1.0
//! starts its own save format, so the places are played afresh, the same
//! way.

use pocket_universe::{
    NUDGE_COMMAND, POCKET_UNIVERSE_PACK_ID, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use std::sync::OnceLock;
use world_projection::ProjectionSnapshot;

const PLACES: [(&str, &str); 3] = [
    ("mars", SEED_MARS_COLONY_COMMAND),
    ("maple", SEED_1980S_TOWN_COMMAND),
    ("ice", SEED_PENGUIN_CIVILIZATION_COMMAND),
];

fn registry() -> world_host::WorldRegistry {
    let mut registry = world_host::WorldRegistry::new();
    registry
        .register(pocket_universe::pocket_universe_registration())
        .unwrap();
    registry
}

/// Each place played by the builder to `ages`, labelled.
fn played(ages: &[usize]) -> Vec<(String, ProjectionSnapshot)> {
    let registry = registry();
    PLACES
        .iter()
        .flat_map(|(place, seed)| {
            world_pack_testkit::replay::lived(
                &registry,
                POCKET_UNIVERSE_PACK_ID,
                &[seed],
                NUDGE_COMMAND,
                ages,
            )
            .into_iter()
            .map(move |(age, snapshot)| (format!("{place} {age}"), snapshot))
        })
        .collect()
}

/// The lived places, played once for every test here.
fn lived() -> &'static [(String, ProjectionSnapshot)] {
    static LIVED: OnceLock<Vec<(String, ProjectionSnapshot)>> = OnceLock::new();
    LIVED.get_or_init(|| played(&[90, 120]))
}

/// Every lived place, opened, shows its people doing something at ten in
/// the morning: at least six of them out working, carrying, keeping a
/// stall, sitting on a bench or with someone.
#[test]
fn a_lived_place_has_people_doing_things_at_ten() {
    for (name, snapshot) in lived() {
        let busy = world_pack_testkit::town::busy_at(snapshot, 10);
        assert!(busy >= 6, "{name}: {busy} busy at ten");
    }
}

/// The S2 bar, on screen (v0.30 S2d, S1c): every lived place, seen in the
/// World window with the camera at rest at each hour from nine to four, has
/// at least six residents on its first screen seen doing something
/// (working, keeping a stall, carrying, sitting where their day has them)
/// or with someone, or where fewer than six are out of doors in the whole
/// place, every one of them. Measured on the window's own layout and
/// opening view, the hour pinned.
#[test]
fn six_residents_busy_on_first_screen_by_day() {
    let (mut short, mut seen, mut full) = (Vec::new(), 0, 0);
    for (name, snapshot) in lived() {
        for hour in 9..=16 {
            let screen = world_gpui::first_screen_busy(snapshot, (1100.0, 848.0), hour);
            // Out of doors in the whole place at that hour, as the window
            // lays it out.
            let stage = world_gpui::diorama::stage_at(
                snapshot,
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
                    "{name} at {hour}: {} busy of {:?}, {outdoors} out",
                    screen.busy(),
                    screen.residents
                ));
            }
        }
    }
    assert!(short.is_empty(), "{}", short.join("\n"));
    assert!(
        full > 0 && seen > 0,
        "{full} of {seen} first screens with six busy"
    );
}

/// The first screen at rest, morning to night (v0.30 S1c round 2): in every
/// lived place, and on Ares two weeks in, at every hour from nine to ten at
/// night, the window's opening view meets the art director's bar
/// (`world_gpui::first_screen_bar`): six busy by day, or everyone out; six
/// by lamplight of an evening, or everyone not abed; no rows turned to the
/// camera, one talking at a time, most turned to whoever they are with;
/// and nobody anywhere on or in a thing.
#[test]
fn the_first_screen_at_rest_morning_to_night() {
    let mut places = lived().to_vec();
    // Ares two weeks in: six colonists, and the art director's first
    // screen.
    places.extend(world_pack_testkit::replay::lived(
        &registry(),
        POCKET_UNIVERSE_PACK_ID,
        &[SEED_MARS_COLONY_COMMAND],
        NUDGE_COMMAND,
        &[14],
    ));
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
