//! The camera at rest (v0.30 S1c): how big people are drawn, and what the
//! first screen shows of them.

use super::*;
use std::collections::BTreeSet;

/// The window's bar of gauges over the stage (`world_window::CHROME`).
const CHROME: f32 = 52.0;

/// Every scene fixture (`tests/fixtures`), by file name: the places of
/// both Packs from their first day to three years on.
pub(crate) fn every_fixture() -> Vec<(String, ProjectionSnapshot)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut paths = std::fs::read_dir(dir)
        .expect("the fixtures")
        .map(|entry| entry.expect("a fixture").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();
    paths.sort();
    assert!(paths.len() >= 10, "{} fixtures", paths.len());
    paths
        .into_iter()
        .map(|path| {
            let json = std::fs::read_to_string(&path).expect("a fixture");
            let wire: world_pack_protocol::ProjectionSnapshotWire =
                serde_json::from_str(&json).expect("a wire snapshot");
            let name = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            (
                name,
                ProjectionSnapshot::try_from(wire).expect("a snapshot"),
            )
        })
        .collect()
}

/// The art director's bar (v0.30 S1c): with the camera at rest, people
/// read as people, 120 to 160 px tall. In every fixture, in windows from
/// 700 to 1,600 px high, at noon and at dusk: a grown-up on the people's
/// line stands 120 to 160 px, the grown-ups standing in view stand 120 to
/// 160 px at the median, and none of them is under 105 or over 180 (the
/// depths of the people's ground make the nearer bigger).
#[test]
fn people_at_rest_120_to_160_px() {
    let mut seen = 0;
    for (name, snapshot) in every_fixture() {
        for window_h in (700..=1600).step_by(100) {
            let height = window_h as f32 - CHROME;
            for hour in [12, 19] {
                let stage = stage_at(&snapshot, 1100.0, height, Clock::at(hour));
                let rest = Camera::rest_zoom(&stage);
                let on_the_line = stage.figure_h * rest;
                assert!(
                    (120.0..=160.0).contains(&on_the_line),
                    "{name} in a window {window_h} high: {on_the_line:.0} px on the line"
                );
                let daylight = crate::scene::daylight_at(u32::from(hour));
                let living = living(&stage, &snapshot, 0.0, daylight, &BTreeSet::new(), None);
                for centre in [stage.width * 0.25, stage.width * 0.5, stage.width * 0.75] {
                    let camera = Camera::rest(&stage, centre);
                    assert!((camera.zoom - rest).abs() < 1e-4);
                    let frame = frame(
                        &snapshot,
                        &stage,
                        &living,
                        camera,
                        0.0,
                        daylight,
                        &Glows::new(),
                        1.0,
                    );
                    let mut heights = frame
                        .people
                        .iter()
                        .filter(|person| {
                            person.figure.age == crate::age::Age::Adult
                                && !person.figure.bird
                                && person.stance != Stance::Sitting
                                && (0.0..=frame.view_w).contains(&person.x)
                        })
                        .map(|person| person.height)
                        .collect::<Vec<_>>();
                    if heights.is_empty() {
                        continue;
                    }
                    seen += 1;
                    heights.sort_by(f32::total_cmp);
                    // The median of those at the line's own depths (not
                    // the lane in front, where a crowd overflows).
                    let line = stage.figure_h * camera.zoom;
                    let mut middle = heights
                        .iter()
                        .copied()
                        .filter(|h| *h <= line * 1.1)
                        .collect::<Vec<_>>();
                    if middle.is_empty() {
                        middle = heights.clone();
                    }
                    let median = middle[middle.len() / 2];
                    // The median of three or more: one or two may stand
                    // nearer or further.
                    assert!(
                        heights.len() < 3 || (120.0..=160.0).contains(&median),
                        "{name} {window_h} high at {hour}: median {median:.0} px of {heights:?}"
                    );
                    assert!(
                        heights[0] >= 105.0 && heights[heights.len() - 1] <= 180.0,
                        "{name} {window_h} high at {hour}: {heights:?}"
                    );
                }
            }
        }
    }
    assert!(seen > 100, "{seen} screens with grown-ups on them");
}

/// The camera at rest frames the street's front roofs down to the water's
/// edge: the people's line well above the card's band at the window's foot
/// (the lower 22%), and its water's edge in the window.
#[test]
fn the_camera_at_rest_frames_the_quay_above_the_card() {
    for (name, snapshot) in every_fixture() {
        for window_h in [700, 848, 1000, 1600] {
            let height = window_h as f32 - CHROME;
            let stage = stage_at(&snapshot, 1100.0, height, Clock::at(12));
            let camera = Camera::rest(&stage, stage.width / 2.0);
            let (_, line) = camera.at(&stage, 0.0, stage.feet);
            let (_, edge) = camera.at(&stage, 0.0, stage.front);
            assert!(
                line > height * 0.35 && line < height * 0.75,
                "{name} {window_h}: the people's line at {:.2} of the window",
                line / height
            );
            assert!(
                edge <= height * 0.79,
                "{name} {window_h}: the water's edge at {:.2}",
                edge / height
            );
        }
    }
}

/// The art bible's §6 at rest (v0.30 S1c), along every fixture's whole
/// place, window by window, at 10, 13, 16 and 19 o'clock: never more than
/// three standing in one row, no two overlapping by more than a fifth, and
/// of the pairs standing together (neighbours in a group), at least 60%
/// not both turned to the camera.
#[test]
fn rows_and_overlap_at_rest() {
    let (mut pairs, mut both_front) = (0, 0);
    let mut failures = Vec::new();
    for (name, snapshot) in every_fixture() {
        for hour in [10, 13, 16, 19] {
            let stage = stage_at(&snapshot, 1100.0, 848.0, Clock::at(hour));
            let daylight = crate::scene::daylight_at(u32::from(hour));
            let living = living(&stage, &snapshot, 3.0, daylight, &BTreeSet::new(), None);
            let rest = Camera::rest_zoom(&stage);
            let seen = stage.view_w / rest;
            let groups = group_centres(&stage.people, stage.figure_h);
            let mut centre = seen / 2.0;
            while centre - seen / 2.0 < stage.width {
                let camera = Camera::rest(&stage, centre);
                let frame = frame(
                    &snapshot,
                    &stage,
                    &living,
                    camera,
                    3.0,
                    daylight,
                    &Glows::new(),
                    1.0,
                );
                let on_screen = frame
                    .people
                    .iter()
                    .filter(|person| person.x > 0.0 && person.x < frame.view_w)
                    .collect::<Vec<_>>();
                let people = on_screen
                    .iter()
                    .map(|person| (person.x, person.y, person.height))
                    .collect::<Vec<_>>();
                let (row, _) = rows_and_overlap(&people);
                // Overlap among those standing: someone walking by passes in
                // front of others a moment, as people do.
                let standing = on_screen
                    .iter()
                    .filter(|person| person.pose.stride.is_none())
                    .map(|person| (person.x, person.y, person.height))
                    .collect::<Vec<_>>();
                let (_, overlap) = rows_and_overlap(&standing);
                if row > 3 {
                    failures.push(format!("{name} at {hour}, {centre:.0}: {row} in a row"));
                }
                if overlap > 0.2 {
                    failures.push(format!(
                        "{name} at {hour}, {centre:.0}: two overlap by {:.0}%",
                        overlap * 100.0
                    ));
                }
                // Neighbours in a group, both on this screen.
                let mut members = stage
                    .people
                    .iter()
                    .zip(&groups)
                    .filter_map(|(spot, group)| Some((group.as_ref()?, spot)))
                    .filter_map(|(group, spot)| {
                        let drawn = on_screen.iter().find(|p| p.index == spot.index)?;
                        Some(((*group * 10.0).round() as i64, spot.x, drawn.turn))
                    })
                    .collect::<Vec<_>>();
                members.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
                for pair in members.windows(2).filter(|pair| pair[0].0 == pair[1].0) {
                    pairs += 1;
                    both_front += usize::from(pair[0].2 == Turn::Front && pair[1].2 == Turn::Front);
                }
                centre += seen;
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(pairs >= 40, "{pairs} pairs seen");
    assert!(
        (pairs - both_front) * 10 >= pairs * 6,
        "{both_front} of {pairs} pairs both turned to the camera"
    );
}

/// Night reads as night at rest (v0.30 S1c round 2: at 21:00 the window
/// was nine tenths dusk, a sepia day): from nine the look the camera at
/// rest asks for first is the night light, alone, and at seven it is dusk.
#[test]
fn the_look_at_rest_by_nine_is_the_night() {
    use super::living_stage::Light;
    for (name, snapshot) in every_fixture() {
        let stage = stage_at(&snapshot, 1100.0, 796.0, Clock::at(21));
        let camera = Camera::rest(&stage, stage.width / 2.0);
        let looks = |hour: f32| {
            super::stage_view::wants(1, (stage.width, stage.view_w), camera, None, hour)
                .into_iter()
                .filter(|want| want.shown)
                .map(|want| want.light)
                .collect::<Vec<_>>()
        };
        for hour in [21.0, 21.5, 22.0, 23.0, 2.0] {
            assert_eq!(looks(hour), vec![Light::Night], "{name} at {hour}");
        }
        assert_eq!(looks(19.5), vec![Light::Dusk], "{name} at 19:30");
        assert_eq!(looks(20.5).first(), Some(&Light::Dusk), "{name} at 20:30");
        assert_eq!(Light::blend(21.0), (Light::Night, Light::Night, 0.0));
    }
}

/// Nobody stands on or in a thing at rest (v0.30 S1c round 2: a keeper
/// under a solar panel with its pole through her, two on a sea pool's
/// railing): in every fixture, every hour from nine to ten at night,
/// everyone shown at rest stands on open ground, in no footprint and on
/// no picture of a thing but the one they use.
#[test]
fn nobody_stands_on_a_thing_at_rest() {
    let mut failures = Vec::new();
    let mut seen = 0;
    for (name, snapshot) in every_fixture() {
        for hour in 9..=22 {
            let stage = stage_at(&snapshot, 1100.0, 796.0, Clock::at(hour));
            let rest = Camera::rest_zoom(&stage);
            seen += stage.people.len();
            for index in on_a_thing(&stage, &snapshot, rest) {
                failures.push(format!(
                    "{name} at {hour}: {} on a thing",
                    snapshot.canvas.items[index].label
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(seen > 500, "{seen} people looked at");
}

/// Gatherings at rest are not rows (v0.30 S1c round 2): on every screen at
/// rest along every fixture, by day and by lamplight, no three turned to the
/// camera stand in one row, at most one in each conversation talks (a hand
/// up) at once, and of those standing with someone at least half are turned
/// to them.
#[test]
fn gatherings_at_rest_face_each_other() {
    let mut failures = Vec::new();
    let (mut together, mut facing) = (0, 0);
    for (name, snapshot) in every_fixture() {
        for hour in [10, 13, 16, 19, 21] {
            let stage = stage_at(&snapshot, 1100.0, 796.0, Clock::at(hour));
            let seen = stage.view_w / Camera::rest_zoom(&stage);
            let mut centre = seen / 2.0;
            while centre - seen / 2.0 < stage.width {
                for seconds in [3.0, 9.0] {
                    let screen = gathering_at_rest(&snapshot, &stage, hour, centre, seconds);
                    together += screen.together;
                    facing += screen.facing;
                    if screen.front_in_a_row >= 3 {
                        failures.push(format!(
                            "{name} at {hour}, {centre:.0}: {} to the camera in a row",
                            screen.front_in_a_row
                        ));
                    }
                    if screen.most_talking > 1 {
                        failures.push(format!(
                            "{name} at {hour}, {centre:.0}: {} talking at once",
                            screen.most_talking
                        ));
                    }
                    if screen.together >= 2 && screen.facing * 2 < screen.together {
                        failures.push(format!(
                            "{name} at {hour}, {centre:.0}: {} of {} turned to someone",
                            screen.facing, screen.together
                        ));
                    }
                }
                centre += seen;
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(together >= 60, "{together} standing with someone");
    assert!(
        facing * 10 >= together * 7,
        "{facing} of {together} turned to someone"
    );
}

/// A cluster's patch is ground, not a veil (v0.30 S1c round 2: at rest a
/// plaza half seen through lay over the far field's beds as a pale ghost
/// by the clock post): every patch on land covers what is under it nine
/// tenths or more; one at the water lays nothing.
#[test]
fn a_patch_is_ground_not_a_veil() {
    for patch in world_art::Ground::ALL {
        let (rim, body) = super::ground::cover(patch, false);
        assert!(body >= 0.9, "{patch:?}: {body}");
        assert!(rim < body, "{patch:?}: a soft rim");
        assert_eq!(super::ground::cover(patch, true), (0.0, 0.0));
    }
}
