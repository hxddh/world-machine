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

/// The camera at rest frames the roofline over the people (v0.30 S1d, the art
/// director's "no sky or roofline at rest"): in every fixture, in windows
/// 700 to 1,600 high, the street's front roofs are whole in the window with
/// room over them, the people's line stands well above the card's band at
/// the window's foot (the lower 22%), and the camera stands as high up the
/// place as that allows: the people's line at 0.70 of the window or lower,
/// or else a band of sky over the horizon in the frame.
#[test]
fn the_camera_at_rest_frames_the_roofline_over_the_quay() {
    for (name, snapshot) in every_fixture() {
        for window_h in [700, 848, 1000, 1600] {
            let height = window_h as f32 - CHROME;
            let stage = stage_at(&snapshot, 1100.0, height, Clock::at(12));
            let camera = Camera::rest(&stage, stage.width / 2.0);
            let (_, line) = camera.at(&stage, 0.0, stage.feet);
            let roofs = stage.street - stage.building_h * row_aged(0.60, stage.grown).1;
            let (_, roofs) = camera.at(&stage, 0.0, roofs);
            let (_, horizon) = camera.at(&stage, 0.0, stage.horizon);
            assert!(
                line > height * 0.35 && line <= height * 0.75,
                "{name} {window_h}: the people's line at {:.2} of the window",
                line / height
            );
            // (In a short window the line goes as low as it may first.)
            assert!(
                roofs >= height * 0.02 || line >= height * (REST_LINE_LOWEST - 0.005),
                "{name} {window_h}: the front roofs at {:.2}, over the top",
                roofs / height
            );
            assert!(
                line >= height * 0.70 || horizon >= height * 0.02,
                "{name} {window_h}: the line at {:.2} and the horizon at {:.2}",
                line / height,
                horizon / height
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

/// A place as the art director saw it at the S1c sign-off, as the Packs
/// make it today (`tests/fixtures/rest`): the first Pack's town on its
/// first day, and the second's colony two weeks in (v0.30 S1d). The
/// fixtures above were saved before the days gathered people at hubs.
pub(crate) fn rest_fixture(name: &str) -> ProjectionSnapshot {
    let path = format!(
        "{}/tests/fixtures/rest/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(path).expect("a fixture");
    let wire: world_pack_protocol::ProjectionSnapshotWire =
        serde_json::from_str(&json).expect("a wire snapshot");
    ProjectionSnapshot::try_from(wire).expect("a snapshot")
}

/// Every ring at rest (v0.30 S1d), by item index of its middle, its members
/// by their position on the stage.
fn rings_of(stage: &Stage) -> BTreeMap<i64, Vec<usize>> {
    let mut rings = BTreeMap::<i64, Vec<usize>>::new();
    for (at, spot) in stage.people.iter().enumerate() {
        if let Some(middle) = stage.doing.get(&spot.index).and_then(|what| what.ring) {
            rings
                .entry((middle * 4.0).round() as i64)
                .or_default()
                .push(at);
        }
    }
    rings
}

/// A gathering at a hub stands in a ring, not a line (v0.30 S1d, the art
/// director: the first day "reads as one line in two depths"):
/// in every fixture, by day and of an evening, each ring has four or more,
/// stands at least 0.6 of a person deep from its back to its front (0.45
/// formed in front of whoever speaks, who stands at its back), has
/// one clearly behind another (closer across than 0.6 of a person, more
/// than half a person further back) where it has five or more, and
/// everyone off its middle faces into it.
#[test]
fn a_gathering_at_a_hub_stands_in_a_ring() {
    let (mut rings, mut failures) = (0, Vec::new());
    let places = every_fixture().into_iter().chain(
        ["first-day", "colony-day-14"].map(|name| (format!("rest/{name}"), rest_fixture(name))),
    );
    for (name, snapshot) in places {
        for hour in (9..=16).chain(19..=21) {
            let stage = stage_at(&snapshot, 1100.0, 796.0, Clock::at(hour));
            let fig = stage.figure_h;
            let daylight = crate::scene::daylight_at(u32::from(hour));
            let living = living(&stage, &snapshot, 3.0, daylight, &BTreeSet::new(), None);
            for (key, ring) in rings_of(&stage) {
                rings += 1;
                let middle = key as f32 / 4.0;
                let spots = ring.iter().map(|at| stage.people[*at]).collect::<Vec<_>>();
                let back = spots.iter().map(|s| s.y).fold(f32::MAX, f32::min);
                let front = spots.iter().map(|s| s.y).fold(f32::MIN, f32::max);
                if ring.len() < super::people::RING_LEAST {
                    failures.push(format!("{name} at {hour}: a ring of {}", ring.len()));
                }
                // (A ring formed in front of whoever speaks, its middle left
                // open for them at its back, is shallower: they are its back.)
                let open_middle = spots.iter().all(|s| (s.x - middle).abs() > fig * 0.3);
                let deep = if open_middle { 0.45 } else { 0.6 };
                if front - back < fig * deep {
                    failures.push(format!(
                        "{name} at {hour}: a ring {:.2} of a person deep",
                        (front - back) / fig
                    ));
                }
                let behind = spots.iter().any(|a| {
                    spots
                        .iter()
                        .any(|b| (a.x - b.x).abs() < fig * 0.6 && b.y - a.y > fig * 0.5)
                });
                if ring.len() >= 5 && !behind {
                    failures.push(format!("{name} at {hour}: nobody behind anyone"));
                }
                for at in &ring {
                    let x = living[*at].x;
                    if (middle - x).abs() > fig * 0.3
                        && living[*at].pose.facing * (middle - x) <= 0.0
                    {
                        failures.push(format!(
                            "{name} at {hour}: {} faces out of the ring",
                            snapshot.canvas.items[stage.people[*at].index].label
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(rings >= 10, "{rings} rings");
}

/// The first day opens on a ring (v0.30 S1d): at ten and at three
/// the window's opening view at rest has a ring of four or more of the
/// town in it, its middle well inside.
#[test]
fn the_first_day_opens_on_a_ring() {
    let snapshot = rest_fixture("first-day");
    for hour in [10, 15] {
        let screen = crate::first_screen_busy(&snapshot, (1100.0, 848.0), hour);
        let stage = stage_at(&snapshot, 1100.0, 848.0, Clock::at(hour));
        let half = stage.view_w / (2.0 * Camera::rest_zoom(&stage));
        let opened = rings_of(&stage).into_iter().any(|(key, ring)| {
            ring.len() >= 4 && (key as f32 / 4.0 - screen.centre).abs() <= half * 0.7
        });
        assert!(opened, "at {hour}: no ring on the first screen");
    }
}

/// The gathering forms round whoever speaks (v0.30 S1d, the art director:
/// in the colony two weeks in the speaker at the solar array stood a screen from
/// her four listeners): by day, whoever speaks first standing at what they
/// work at has the ring in front of them, its middle within a person of
/// them across and every one of it within four.
#[test]
fn the_gathering_forms_round_whoever_speaks() {
    let snapshot = rest_fixture("colony-day-14");
    let speaker = speaks_first(&snapshot).expect("someone speaks");
    let mut seen = 0;
    for hour in 9..=16 {
        let stage = stage_at(&snapshot, 1100.0, 796.0, Clock::at(hour));
        let fig = stage.figure_h;
        let Some(them) = stage.people.iter().find(|spot| spot.index == speaker) else {
            continue;
        };
        if stage
            .doing
            .get(&speaker)
            .is_none_or(|what| what.fixture.is_none())
        {
            continue;
        }
        // (At noon the Packs' bar holds the gathering to a share of the
        // town: no ring.)
        let Some((key, ring)) = rings_of(&stage).into_iter().min_by(|a, b| {
            (a.0 as f32 / 4.0 - them.x)
                .abs()
                .total_cmp(&(b.0 as f32 / 4.0 - them.x).abs())
        }) else {
            continue;
        };
        seen += 1;
        assert!(
            (key as f32 / 4.0 - them.x).abs() <= fig,
            "at {hour}: the ring's middle {:.1} people from whoever speaks",
            (key as f32 / 4.0 - them.x) / fig
        );
        for at in ring {
            assert!(
                (stage.people[at].x - them.x).abs() <= fig * 4.0,
                "at {hour}: {} {:.1} people from whoever speaks",
                snapshot.canvas.items[stage.people[at].index].label,
                (stage.people[at].x - them.x) / fig
            );
        }
    }
    assert!(seen >= 4, "{seen} hours with whoever speaks at work");
}

/// Laid ground has a kerb, not a soft edge (v0.30 S1d, the art director:
/// the paving by the clock post was "a soft cloud shape cut over the
/// beds"): cobbles, a plaza and paving painted over the field have a back
/// edge that is straight (where it begins down each column, across the
/// middle eight tenths, within a pixel and a half) and hard (the field to
/// the stone within three pixels).
#[test]
fn laid_ground_has_a_kerb_not_a_soft_edge() {
    use super::ground::{paint_patch, PatchPaint};
    let snapshot = rest_fixture("first-day");
    let stage = stage_at(&snapshot, 1100.0, 796.0, Clock::at(15));
    let daylight = crate::scene::daylight_at(15);
    let living = living(&stage, &snapshot, 0.0, daylight, &BTreeSet::new(), None);
    let frame = frame(
        &snapshot,
        &stage,
        &living,
        Camera::whole(&stage),
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    );
    let field = art::hex(0x7fa07a);
    let (w, h) = (320_u32, 200_u32);
    for patch in [
        world_art::Ground::Cobbles,
        world_art::Ground::Plaza,
        world_art::Ground::Paving,
    ] {
        assert!(super::ground::laid(patch));
        let mut canvas = Canvas::new(w, h, 1.0, (0.0, 0.0)).expect("a canvas");
        canvas
            .pixmap
            .fill(sk::Color::from_rgba8(0x7f, 0xa0, 0x7a, 0xff));
        let paint = PatchPaint {
            x0: 60.0,
            x1: 260.0,
            top: 60.0,
            bottom: 150.0,
            patch,
            water: false,
            seed: 7,
        };
        paint_patch(&mut canvas, &frame, &paint, field, 1.0);
        let data = canvas.pixmap.data();
        let off = |x: u32, y: u32| {
            let at = ((y * w + x) * 4) as usize;
            (i32::from(data[at]) - 0x7f).abs()
                + (i32::from(data[at + 1]) - 0xa0).abs()
                + (i32::from(data[at + 2]) - 0x7a).abs()
        };
        let mut tops = Vec::new();
        for x in (80..240).step_by(4) {
            let starts = (0..h).find(|y| off(x, *y) > 12).expect("the patch");
            let full = (starts..h).find(|y| off(x, *y) > 40).expect("the stone");
            assert!(
                full - starts <= 3,
                "{patch:?} at {x}: a soft edge {starts}..{full}"
            );
            tops.push(starts as f32);
        }
        let (least, most) = tops
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, m), y| (l.min(*y), m.max(*y)));
        assert!(
            most - least <= 1.5,
            "{patch:?}: a wandering edge {least}..{most}"
        );
    }
}

/// Everyone with something to say today, by item index: whose name goes
/// under their feet while they say it (v0.30 S1d round 2).
fn speaks_today(snapshot: &ProjectionSnapshot) -> BTreeSet<usize> {
    let today = snapshot
        .timeline
        .items
        .iter()
        .filter(|item| item.world_time == snapshot.world_time)
        .map(|item| item.id)
        .collect::<BTreeSet<_>>();
    snapshot
        .voices
        .iter()
        .filter(|voice| today.contains(&voice.moment))
        .filter_map(|voice| {
            snapshot
                .canvas
                .items
                .iter()
                .position(|item| item.id == voice.speaker)
        })
        .collect()
}

/// The rest fixtures (`tests/fixtures/rest`) by name, with every scene
/// fixture: the places as the Packs make them today, and as they were.
fn rest_and_every_fixture() -> Vec<(String, ProjectionSnapshot)> {
    ["first-day", "colony-day-14", "three-years"]
        .map(|name| (format!("rest/{name}"), rest_fixture(name)))
        .into_iter()
        .chain(every_fixture())
        .collect()
}

/// Each window's frame at rest along a place, its camera centred one
/// window after another, at `hour` in a window `window_h` high.
fn frames_at_rest(snapshot: &ProjectionSnapshot, hour: u8, window_h: f32) -> Vec<(f32, Frame)> {
    let stage = stage_at(snapshot, 1100.0, window_h - CHROME, Clock::at(hour));
    let daylight = crate::scene::daylight_at(u32::from(hour));
    let living = living(&stage, snapshot, 3.0, daylight, &BTreeSet::new(), None);
    let seen = stage.view_w / Camera::rest_zoom(&stage);
    let mut frames = Vec::new();
    let mut centre = seen / 2.0;
    while centre - seen / 2.0 < stage.width {
        let camera = Camera::rest(&stage, centre);
        frames.push((
            centre,
            frame(
                snapshot,
                &stage,
                &living,
                camera,
                3.0,
                daylight,
                &Glows::new(),
                1.0,
            ),
        ));
        centre += seen;
    }
    frames
}

/// A name is measured at its widest (v0.30 S1d round 3): a Chinese,
/// Japanese or Korean character takes a whole em of the small type, wider
/// than a Latin letter, and larger text makes the whole name larger.
#[test]
fn a_name_is_measured_at_its_widest() {
    assert!(name_w("林美玲", 1.0) >= 3.0 * 12.0 + 16.0);
    assert!(name_w("김민준", 1.0) >= 3.0 * 12.0 + 16.0);
    assert!(name_w("林美玲", 1.0) > name_w("Mia", 1.0));
    assert!(name_w("Zoë", 1.0) >= name_w("Zoe", 1.0));
    assert_eq!(name_w("Mia", 2.0), 2.0 * name_w("Mia", 1.0));
}

/// A name covers nobody (v0.30 S1d round 2, the art director: in the
/// rings the name of whoever spoke lay across the heads of those in front
/// of them; the art bible's §6): in the rest fixtures and every scene
/// fixture, at rest along the whole place, by day and of an evening, the
/// name of each who has something to say today has a place
/// ([`name_spot`]) clear of everyone else's silhouette.
#[test]
fn no_name_covers_anyone_at_rest() {
    let (mut named, mut failures) = (0, Vec::new());
    for (name, snapshot) in rest_and_every_fixture() {
        let voiced = speaks_today(&snapshot);
        for hour in [9, 10, 13, 15, 19, 21] {
            for (centre, frame) in frames_at_rest(&snapshot, hour, 848.0) {
                let standing = frame
                    .people
                    .iter()
                    .map(|person| (person.x, person.y, person.height))
                    .collect::<Vec<_>>();
                for (nth, person) in frame.people.iter().enumerate() {
                    if !voiced.contains(&person.index)
                        || person.x <= 0.0
                        || person.x >= frame.view_w
                        || person.pose.stride.is_some()
                    {
                        continue;
                    }
                    named += 1;
                    let label = &snapshot.canvas.items[person.index].label;
                    // Their own name, and a Chinese one as wide as most
                    // (v0.30 S1d round 3: a character takes a whole em).
                    for first in [
                        label.split_whitespace().next().unwrap_or_default(),
                        "林美玲",
                    ] {
                        let size = (name_w(first, 1.0), NAME_H);
                        let spot = name_spot(&standing, nth, size);
                        if spot.is_none_or(|spot| !name_clear(&standing, nth, spot, size)) {
                            failures.push(format!(
                                "{name} at {hour}, {centre:.0}: {label}'s name as {first}"
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
    assert!(named >= 40, "{named} names seen");
}

/// Nobody is cut by the window's top at rest (v0.30 S1d round 2, the art
/// director: in the town three years on, the raised camera sliced the people
/// round the pond across their heads): in the rest fixtures and every
/// scene fixture, in windows 700 to 1,600 high, by day and of an evening,
/// everyone standing in view is wholly in the frame or wholly out of it.
#[test]
fn nobody_is_cut_by_the_top_at_rest() {
    let mut failures = Vec::new();
    for (name, snapshot) in rest_and_every_fixture() {
        for window_h in [700.0, 848.0, 1000.0, 1600.0] {
            for hour in [10, 15, 21] {
                for (centre, frame) in frames_at_rest(&snapshot, hour, window_h) {
                    for person in &frame.people {
                        if person.x <= 0.0
                            || person.x >= frame.view_w
                            || person.pose.stride.is_some()
                        {
                            continue;
                        }
                        if person.y > 0.0 && person.y - person.height < 0.0 {
                            failures.push(format!(
                                "{name} {window_h} at {hour}, {centre:.0}: {} cut",
                                snapshot.canvas.items[person.index].label
                            ));
                        }
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The roofs are never given up for anyone (v0.30 S1d round 3, a review:
/// in a short window someone a screen away straddling the top could
/// lower it past the front roofs): in every fixture, in short windows,
/// with someone anywhere along the place standing across the top at
/// rest, the top stands no lower than it did or than the roofs' top,
/// whichever is lower: the roofs stay whole.
#[test]
fn nobody_off_screen_costs_the_roofs() {
    let mut tried = 0;
    for (name, snapshot) in rest_and_every_fixture() {
        for window_h in [700.0, 760.0, 848.0] {
            let stage = stage_at(&snapshot, 1100.0, window_h - CHROME, Clock::at(12));
            let zoom = Camera::rest_zoom(&stage);
            let seen = stage.height / zoom;
            let roofs = stage.street - stage.building_h * row_aged(0.60, stage.grown).1;
            let bound = roofs;
            let top = Camera::rest_y(&stage, zoom) - seen / 2.0;
            let Some(someone) = stage
                .people
                .iter()
                .find(|spot| stage.shows(spot.index, zoom))
            else {
                continue;
            };
            for share in [0.1, 0.4, 0.8] {
                tried += 1;
                let mut straddled = stage.clone();
                let mut settled = straddled.settled.as_ref().clone();
                settled.push((someone.index, top + stage.figure_h * share, 1.0));
                straddled.settled = std::sync::Arc::new(settled);
                let now = Camera::rest_y(&straddled, zoom) - seen / 2.0;
                assert!(
                    now <= top.max(bound) + 1e-3,
                    "{name} {window_h}: the top from {top:.1} to {now:.1}, past the roofs' {bound:.1}"
                );
            }
        }
    }
    assert!(tried >= 30, "{tried} tried");
}

/// The camera at rest keeps its height all through the hour (v0.30 S1d
/// round 3): whoever is still on their way as it begins, the camera at
/// rest stands where it will once all have arrived.
#[test]
fn the_rest_height_keeps_through_the_hour() {
    for (name, snapshot) in rest_and_every_fixture() {
        for hour in [8, 10, 15, 19] {
            let settled = stage_at(&snapshot, 1100.0, 796.0, Clock::at(hour));
            let zoom = Camera::rest_zoom(&settled);
            for into_hour in [1.0, 20.0, 90.0] {
                let early = stage_at(&snapshot, 1100.0, 796.0, Clock { hour, into_hour });
                assert_eq!(
                    Camera::rest_y(&early, zoom),
                    Camera::rest_y(&settled, zoom),
                    "{name} at {hour}, {into_hour} s in"
                );
            }
        }
    }
}
