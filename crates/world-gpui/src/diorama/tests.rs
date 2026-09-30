use super::*;
use world_projection::{CanvasItem, CanvasItemKind, CanvasProjection};

#[test]
fn every_hour_has_a_light_of_its_own() {
    let grades = (0..24)
        .map(|hour| format!("{:?}", grade_at(hour as f32)))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(grades.len(), 24, "{grades:?}");
    // And it turns smoothly: half past is between the hours either side.
    let ((_, noon), _) = grade_at(12.0);
    let ((_, dusk), _) = grade_at(19.0);
    let ((_, between), _) = grade_at(15.5);
    assert!(between > noon.min(dusk) && between < noon.max(dusk));
}

fn entity(id: u64) -> SelectionId {
    SelectionId::from_stable_key(&format!("entity-{id}")).expect("an entity key")
}

fn item(id: u64, kind: CanvasItemKind, x: f32, at: Option<u64>) -> CanvasItem {
    CanvasItem {
        id: entity(id),
        kind,
        label: format!("Item {id}"),
        detail: String::new(),
        x,
        y: 0.5,
        changes: Vec::new(),
        shape: None,
        at: at.map(entity),
        look: None,
        drawing: None,
        stance: None,
        standing: None,
        mood: None,
        spot: None,
        px: None,
        home: None,
        day: Vec::new(),
        built: None,
        ..Default::default()
    }
}

fn harbour() -> ProjectionSnapshot {
    let mut items = vec![
        item(101, CanvasItemKind::Place, 0.1, None),
        item(102, CanvasItemKind::Place, 0.6, None),
        item(103, CanvasItemKind::Place, 0.6, None),
        item(104, CanvasItemKind::Place, 0.3, None),
        item(201, CanvasItemKind::Object, 0.0, Some(101)),
    ];
    for (id, x, at) in [
        (1, 0.1, None),
        (2, 0.7, Some(102)),
        (3, 0.3, Some(104)),
        (4, 0.7, Some(103)),
        (5, 0.8, Some(103)),
        (6, 0.2, Some(101)),
        (7, 0.0, Some(101)),
        (8, 0.4, Some(104)),
    ] {
        items.push(item(id, CanvasItemKind::Actor, x, at));
    }
    items[4].shape = Some(MarkShape::Boat);
    ProjectionSnapshot {
        canvas: CanvasProjection {
            items,
            links: Vec::new(),
            marks: Vec::new(),
            ..Default::default()
        },
        ..ProjectionSnapshot::default()
    }
}

#[test]
fn places_stand_in_order_and_people_stand_at_theirs_without_overlap() {
    let snapshot = harbour();
    let stage = stage(&snapshot, 1100.0, 848.0);
    let xs = stage
        .buildings
        .iter()
        .map(|spot| (snapshot.canvas.items[spot.index].x, spot.x))
        .collect::<Vec<_>>();
    for pair in xs.windows(2) {
        assert!(pair[0].0 <= pair[1].0 && pair[0].1 < pair[1].1, "{xs:?}");
    }
    let mut people = stage.people.clone();
    people.sort_by(|a, b| a.x.total_cmp(&b.x));
    for pair in people.windows(2) {
        assert!(pair[1].x - pair[0].x >= stage.figure_h * 0.62 - 0.01);
    }
    assert!(people
        .iter()
        .all(|spot| spot.x > 0.0 && spot.x < stage.width));
    // People at a place stand near it.
    let bakery = stage.buildings.iter().find(|spot| spot.index == 1).unwrap();
    let mara = stage.people.iter().find(|spot| spot.index == 6).unwrap();
    assert!((mara.x - bakery.x).abs() < stage.building_w);
    // A boat floats in the foreground beside its harbour.
    let boat = stage.things.iter().find(|spot| spot.index == 4).unwrap();
    assert!(boat.y > stage.front);
}

#[test]
fn the_same_world_always_lays_out_the_same() {
    let snapshot = harbour();
    assert_eq!(
        stage(&snapshot, 900.0, 700.0),
        stage(&snapshot, 900.0, 700.0)
    );
}

#[test]
fn nobody_wanders_at_night_or_while_they_are_needed() {
    let snapshot = harbour();
    let stage = stage(&snapshot, 1100.0, 848.0);
    let homes = stage.people.iter().map(|spot| spot.x).collect::<Vec<_>>();
    let pinned = BTreeSet::new();
    for second in 0..200 {
        let night = living(
            &stage,
            &snapshot,
            second as f32,
            Daylight::Night,
            &pinned,
            None,
        );
        for (life, home) in night.iter().zip(&homes) {
            assert!((life.x - home).abs() <= 3.0);
        }
    }
    let everyone = snapshot.canvas.items.iter().map(|item| item.id).collect();
    let day = living(&stage, &snapshot, 1000.0, Daylight::Day, &everyone, None);
    for (life, home) in day.iter().zip(&homes) {
        assert!((life.x - home).abs() <= 3.0);
    }
    // By day, over a few minutes, somebody goes somewhere.
    let wandered = (0..240).any(|second| {
        living(
            &stage,
            &snapshot,
            second as f32,
            Daylight::Day,
            &pinned,
            None,
        )
        .iter()
        .zip(&homes)
        .any(|(life, home)| (life.x - home).abs() > stage.figure_h)
    });
    assert!(wandered);
}

/// The v0.16 bar: everyone has at least four idle behaviours over a
/// few minutes of day, and a click brings a wave within the frame.
#[test]
fn everyone_has_idle_behaviours_and_waves_when_clicked() {
    let snapshot = harbour();
    let stage = stage(&snapshot, 1100.0, 848.0);
    let pinned = BTreeSet::new();
    let mut seen = vec![BTreeSet::new(); stage.people.len()];
    for tenth in 0..3000 {
        let lives = living(
            &stage,
            &snapshot,
            tenth as f32 / 10.0,
            Daylight::Day,
            &pinned,
            None,
        );
        for (index, life) in lives.iter().enumerate() {
            if let Some(stance) = life.stance {
                seen[index].insert(stance);
            }
        }
    }
    for (index, stances) in seen.iter().enumerate() {
        assert!(stances.len() >= 4, "person {index}: {stances:?}");
    }
    let mut lives = living(&stage, &snapshot, 5.0, Daylight::Day, &pinned, None);
    let who = snapshot.canvas.items[stage.people[0].index].id;
    let poked = [(who, 0.3_f32)].into_iter().collect();
    let before = lives[0].pose.bob;
    wave(&mut lives, &stage, &snapshot, &poked, false);
    assert_eq!(lives[0].stance, Some(Stance::Waving));
    assert!(lives[0].pose.bob > before, "a hop");
    let later = [(who, WAVE_SECONDS + 0.1)].into_iter().collect();
    let mut settled = living(&stage, &snapshot, 5.0, Daylight::Day, &pinned, None);
    wave(&mut settled, &stage, &snapshot, &later, false);
    assert_ne!(settled[0].stance, Some(Stance::Waving));
}

#[test]
fn every_hour_has_a_light_of_its_own_and_far_layers_move_least() {
    let grades = [
        Daylight::Dawn,
        Daylight::Day,
        Daylight::Dusk,
        Daylight::Night,
    ]
    .map(grade)
    .map(|((warm, a), (cool, b))| (warm, (a * 100.0) as u32, cool, (b * 100.0) as u32));
    let distinct = grades.iter().collect::<BTreeSet<_>>();
    assert_eq!(distinct.len(), 4);
    assert!(PARALLAX.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(PARALLAX.len() >= 3);
}

/// The v0.16 bar: working out a frame of a busy scene (fifteen people,
/// a dozen buildings, twenty things) fits well inside a 60 fps frame,
/// leaving the rest of the 16 ms for painting. Timed in a release
/// build; a debug build only checks it finishes.
#[test]
fn a_busy_frame_is_worked_out_in_4_ms() {
    let mut items = Vec::new();
    for id in 0..12 {
        items.push(item(
            100 + id,
            CanvasItemKind::Place,
            id as f32 / 12.0,
            None,
        ));
    }
    for id in 0..20 {
        items.push(item(
            300 + id,
            CanvasItemKind::Object,
            id as f32 / 20.0,
            Some(100 + id % 12),
        ));
    }
    for id in 0..15 {
        items.push(item(
            id + 1,
            CanvasItemKind::Actor,
            id as f32 / 15.0,
            Some(100 + id % 12),
        ));
    }
    let snapshot = ProjectionSnapshot {
        canvas: CanvasProjection {
            items,
            links: Vec::new(),
            marks: Vec::new(),
            ..Default::default()
        },
        ..ProjectionSnapshot::default()
    };
    let pinned = BTreeSet::new();
    let runs = 200;
    let started = std::time::Instant::now();
    for run in 0..runs {
        let stage = stage(&snapshot, 1400.0, 900.0);
        let lives = living(
            &stage,
            &snapshot,
            run as f32 / 60.0,
            Daylight::Day,
            &pinned,
            None,
        );
        let frame = frame(
            &snapshot,
            &stage,
            &lives,
            Camera::whole(&stage),
            run as f32 / 60.0,
            Daylight::Day,
            &Glows::new(),
            1.0,
        );
        assert_eq!(frame.people.len(), 15);
    }
    let each = started.elapsed() / runs;
    if !cfg!(debug_assertions) {
        assert!(each.as_micros() < 4_000, "{each:?} a frame");
    }
}

/// No motion over time is linear: every move between two places is
/// eased. Interpolations along a shape (a line, a curve) are drawing,
/// not motion, and are named here.
#[test]
fn no_motion_is_linear() {
    let lerp = regex_lite_like;
    let sources = [
        ("diorama/mod.rs", include_str!("mod.rs")),
        ("diorama/scene.rs", include_str!("scene.rs")),
        ("diorama/people.rs", include_str!("people.rs")),
        ("diorama/works.rs", include_str!("works.rs")),
        ("diorama/light.rs", include_str!("light.rs")),
        ("diorama/interface.rs", include_str!("interface.rs")),
        ("art.rs", include_str!("../art.rs")),
        ("scene.rs", include_str!("../scene.rs")),
        (
            "macos/world_window.rs",
            include_str!("../macos/world_window.rs"),
        ),
        ("macos.rs", include_str!("../macos.rs")),
    ];
    let drawing = [
        "let fx = left + (right - left) * t;",
        // Where a row stands between two others: layout, not motion.
        "return (l0 + (l1 - l0) * t, s0 + (s1 - s0) * t);",
        "|t: f32| point(from.x + (to.x - from.x) * t",
    ];
    for (name, source) in sources {
        let lines = source.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if !lerp(line) || drawing.iter().any(|allowed| line.contains(allowed)) {
                continue;
            }
            let eased = lines[index.saturating_sub(8)..=index]
                .iter()
                .any(|nearby| nearby.contains("ease(") || nearby.contains("with_easing"));
            assert!(
                eased,
                "{name}:{}: linear motion: {}",
                index + 1,
                line.trim()
            );
        }
    }
}

/// Whether a line moves something by a raw fraction of time: `(to -
/// from) * t`, `* progress`, `* phase`.
fn regex_lite_like(line: &str) -> bool {
    let code = line.split("//").next().unwrap_or_default();
    ["* t)", "* t,", "* t;", "* progress", "* phase"]
        .iter()
        .any(|tail| code.contains(tail))
        && code.contains(" - ")
        && code.contains(") *")
}

#[test]
fn a_camera_on_something_frames_it_and_never_looks_past_the_edges() {
    let snapshot = harbour();
    let stage = stage(&snapshot, 1100.0, 848.0);
    let whole = Camera::whole(&stage);
    assert_eq!(whole.at(&stage, 10.0, 20.0), (10.0, 20.0));
    let close = Camera::on(&stage, stage.frame_of(0).unwrap());
    assert!(close.zoom > 1.0 && close.zoom <= 1.8);
    let (left, top) = close.at(&stage, 0.0, 0.0);
    let (right, bottom) = close.at(&stage, stage.width, stage.height);
    assert!(left <= 0.01 && top <= 0.01);
    assert!(right >= stage.width - 0.01 && bottom >= stage.height - 0.01);
}

/// Fog never draws a band: down the scene its thickness never jumps,
/// zoomed out or right in.
#[test]
fn fog_has_no_hard_edges_at_any_zoom() {
    let (width, height) = (1100.0_f32, 848.0_f32);
    for horizon in [-400.0, -60.0, 0.0, 180.0, 300.0, 620.0] {
        for t in [0.0, 7.5, 31.0] {
            let layers = fog_layers(horizon, width, height, t);
            let thickness = |y: f32| {
                layers
                    .iter()
                    .filter(|layer| y >= layer.y && y < layer.y + layer.h)
                    .map(|layer| {
                        let at = (y - layer.y) / layer.h.max(1.0);
                        layer.top + (layer.bottom - layer.top) * at
                    })
                    .sum::<f32>()
            };
            let mut before = thickness(0.0);
            for y in 1..height as usize {
                let now = thickness(y as f32);
                assert!(
                    (now - before).abs() < 0.02,
                    "a band edge at {y} with the horizon at {horizon}"
                );
                before = now;
            }
        }
    }
}

/// The wheel zooms around the point under the pointer: it stays under
/// the pointer, and the view never runs past the stage.
#[test]
fn zooming_keeps_the_point_under_the_pointer_and_stays_on_stage() {
    let snapshot = harbour();
    let stage = stage(&snapshot, 1100.0, 848.0);
    let camera = Camera::around(&stage, 1.6, 500.0, 420.0);
    let (x, y) = camera.at(&stage, 500.0, 420.0);
    let back = camera.stage_point(&stage, x, y);
    assert!((back.0 - 500.0).abs() < 0.01 && (back.1 - 420.0).abs() < 0.01);
    let corner = Camera::around(&stage, 9.0, 0.0, 0.0);
    assert!(corner.zoom <= 2.2);
    let (left, top) = corner.at(&stage, 0.0, 0.0);
    assert!(left.abs() < 0.01 && top.abs() < 0.01);
}

/// A real harbour on day 1,082, a builder's three years on, as its
/// Pack sends it (only what the scene draws): the composition's tests.
pub(crate) fn harbour_1082() -> ProjectionSnapshot {
    let json = include_str!("../../tests/fixtures/harbour-day-1082.json");
    let wire: world_pack_protocol::ProjectionSnapshotWire =
        serde_json::from_str(json).expect("a wire snapshot");
    ProjectionSnapshot::try_from(wire).expect("a snapshot")
}

/// Every box the composition stands at `zoom`: buildings and things
/// (index, left, top, right, foot, building), as drawn.
fn boxes(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    zoom: f32,
) -> Vec<(usize, f32, f32, f32, f32, bool)> {
    let items = &snapshot.canvas.items;
    stage
        .buildings
        .iter()
        .map(|spot| (spot, true))
        .chain(stage.things.iter().map(|spot| (spot, false)))
        .filter(|(spot, _)| stage.shows(spot.index, zoom))
        .map(|(spot, building)| {
            let h = if building {
                stage.height_of(spot, items[spot.index].shape.unwrap_or_default())
            } else {
                spot.w * 1.1
            };
            (
                spot.index,
                spot.x - spot.w / 2.0,
                spot.y - h,
                spot.x + spot.w / 2.0,
                spot.y,
                building,
            )
        })
        .collect()
}

/// The v0.24 bar: on day 1,082 the harbour composes with nothing
/// standing on anything else, at every hour and at two window sizes.
/// Nothing in the same row overlaps; no thing overlaps a building
/// nearer than it (things are drawn over the buildings); no building is
/// more than half hidden by nearer ones; nobody stands on anybody or in
/// anything on the quay; and the composition keeps back little.
#[test]
fn a_three_year_harbour_composes_without_overlaps() {
    let snapshot = harbour_1082();
    for (width, height) in [(1100.0, 848.0), (1440.0, 848.0)] {
        for hour in [8, 12, 19, 23] {
            let stage = stage_at(&snapshot, width, height, Clock::at(hour));
            for zoom in [1.0, Camera::least(&stage)] {
                let all = boxes(&stage, &snapshot, zoom);
                let row = stage.building_h * 0.15;
                let mut hidden_most = 0;
                for a in &all {
                    let mut covered = 0.0;
                    for b in &all {
                        if a.0 == b.0 {
                            continue;
                        }
                        let across = (a.3.min(b.3) - a.1.max(b.1)).max(0.0);
                        let down = (a.4.min(b.4) - a.2.max(b.2)).max(0.0);
                        if (a.4 - b.4).abs() < row {
                            assert!(
                                across <= 0.5,
                                "{hour}h {width}: {} and {} overlap in a row",
                                a.0,
                                b.0
                            );
                        } else if b.5 && b.4 > a.4 && across * down > 0.0 {
                            assert!(
                                a.5,
                                "{hour}h {width}: thing {} runs into nearer building {}",
                                a.0, b.0
                            );
                            covered += across * down;
                        }
                    }
                    if a.5 && covered > 0.5 * (a.3 - a.1) * (a.4 - a.2) {
                        hidden_most += 1;
                    }
                }
                assert_eq!(
                    hidden_most, 0,
                    "{hour}h {width}: buildings more than half hidden"
                );
                let held = stage
                    .shown_from
                    .values()
                    .filter(|least| **least > zoom + 1e-4)
                    .count();
                let total = stage.buildings.len() + stage.things.len() + stage.people.len();
                // Up close almost nothing waits; on the postcard the
                // smallest things and the tightest neighbours do.
                let most = if zoom >= 1.0 { 0.1 } else { 0.3 };
                assert!(
                    held as f32 <= total as f32 * most,
                    "{hour}h {width} at {zoom}: {held} of {total} held back"
                );
                // And whatever waits shows once the camera comes close.
                let never = stage
                    .shown_from
                    .iter()
                    .filter(|(_, least)| **least > ZOOM_MOST)
                    .map(|(index, _)| *index)
                    .collect::<Vec<_>>();
                assert!(never.is_empty(), "{hour}h {width}: never shown {never:?}");
                let people = stage
                    .people
                    .iter()
                    .filter(|spot| stage.shows(spot.index, zoom))
                    .collect::<Vec<_>>();
                let lane = |spot: &Spot| spot.y > stage.feet + stage.figure_h * 0.25;
                for (n, a) in people.iter().enumerate() {
                    for b in people[n + 1..].iter().filter(|b| lane(b) == lane(a)) {
                        assert!(
                            (a.x - b.x).abs() >= stage.figure_h * 0.6,
                            "{} on {}",
                            a.index,
                            b.index
                        );
                    }
                    if lane(a) {
                        continue;
                    }
                    for thing in all
                        .iter()
                        .filter(|b| !b.5 && Depth::at(b.4, height) == Depth::Quay)
                    {
                        let half = a.w / 2.0;
                        assert!(
                            a.x + half <= thing.1 + 0.5 || a.x - half >= thing.3 - 0.5,
                            "{} stands in {}",
                            a.index,
                            thing.0
                        );
                    }
                }
            }
        }
    }
}

/// The v0.24 bar for the lower third: at noon on day 1,082 everyone out
/// and about stands on the quay in the window's lower third, and so do
/// the things there; the quay lies in it from the street's end to the
/// water.
#[test]
fn the_lower_third_is_where_people_walk() {
    let snapshot = harbour_1082();
    let (width, height) = (1100.0, 848.0);
    let stage = stage_at(&snapshot, width, height, Clock::at(12));
    let third = height * 2.0 / 3.0;
    let out = stage
        .people
        .iter()
        .filter(|spot| stage.shows(spot.index, 1.0))
        .collect::<Vec<_>>();
    assert!(out.len() >= 8, "{} out at noon", out.len());
    assert!(
        out.iter().all(|spot| spot.y > third),
        "everyone on the quay"
    );
    let quay_things = stage
        .things
        .iter()
        .filter(|spot| stage.depth_of(spot) == Depth::Quay)
        .count();
    assert!(quay_things >= 10, "{quay_things} things on the quay");
    let frame = frame(
        &snapshot,
        &stage,
        &living(
            &stage,
            &snapshot,
            0.0,
            Daylight::Day,
            &BTreeSet::new(),
            None,
        ),
        Camera::whole(&stage),
        0.0,
        Daylight::Day,
        &Glows::new(),
        1.0,
    );
    assert!(frame.quay_top() > third && frame.front > frame.quay_top());
    // Everyone on it faces the window at full size; each row further
    // back is smaller.
    let scales = ROWS.iter().map(|row| row.2).collect::<Vec<_>>();
    assert!(scales.windows(2).all(|pair| pair[0] < pair[1]));
}

/// The v0.24 bar for scale: a cottage stands two and a half to three
/// times a grown-up at the same depth, wherever it stands, as drawn.
#[test]
fn a_cottage_stands_two_and_a_half_to_three_times_a_person() {
    let snapshot = harbour_1082();
    for (width, height) in [(1100.0, 848.0), (1440.0, 900.0), (480.0, 300.0)] {
        let stage = stage_at(&snapshot, width, height, Clock::at(12));
        let lives = living(
            &stage,
            &snapshot,
            0.0,
            Daylight::Day,
            &BTreeSet::new(),
            None,
        );
        let frame = frame(
            &snapshot,
            &stage,
            &lives,
            Camera::whole(&stage),
            0.0,
            Daylight::Day,
            &Glows::new(),
            1.0,
        );
        let grown = frame
            .people
            .iter()
            .find(|person| person.figure.age == Age::Adult)
            .expect("a grown-up");
        let spot = stage
            .people
            .iter()
            .find(|spot| spot.index == grown.index)
            .unwrap();
        // A grown-up's height on the quay, at full size.
        let person = grown.height / spot.scale;
        let mut cottages = 0;
        for building in frame
            .buildings
            .iter()
            .filter(|b| b.shape == MarkShape::House && b.drawing.is_none())
        {
            let spot = stage
                .buildings
                .iter()
                .find(|spot| spot.index == building.index)
                .unwrap();
            let ratio = building.h / (person * spot.scale);
            assert!((2.5..=3.0).contains(&ratio), "{width}: {ratio}");
            cottages += 1;
        }
        assert!(cottages >= 10);
    }
}

/// A World three years on: a real one's snapshot when
/// `WORLD_GPUI_BENCH_SNAPSHOT` names its wire JSON (as
/// `dump_snapshot` prints it), else one of the same size: a harbour
/// three windows wide with 24 people on their day's rounds, 48 homes
/// and works, 20 things, 62 things built and 5 under way.
pub(crate) fn three_years() -> ProjectionSnapshot {
    if let Some(path) = std::env::var_os("WORLD_GPUI_BENCH_SNAPSHOT") {
        let json = std::fs::read_to_string(path).expect("the snapshot");
        let wire: world_pack_protocol::ProjectionSnapshotWire =
            serde_json::from_str(&json).expect("a wire snapshot");
        return ProjectionSnapshot::try_from(wire).expect("a snapshot");
    }
    let mut items = Vec::new();
    for id in 0..48_u64 {
        let mut place = item(100 + id, CanvasItemKind::Place, 0.0, None);
        place.px = Some(0.06 + 2.88 * id as f32 / 47.0);
        place.shape = Some(
            [
                MarkShape::House,
                MarkShape::House,
                MarkShape::Shop,
                MarkShape::Tree,
                MarkShape::Tower,
                MarkShape::House,
            ][id as usize % 6],
        );
        items.push(place);
    }
    for id in 0..20_u64 {
        let mut thing = item(300 + id, CanvasItemKind::Object, 0.0, Some(100 + id * 2));
        if id % 4 == 0 {
            thing.shape = Some(MarkShape::Boat);
        }
        items.push(thing);
    }
    for id in 0..24_u64 {
        let mut person = item(id + 1, CanvasItemKind::Actor, 0.0, Some(100 + id * 2));
        person.day = vec![
            world_projection::RoutineStop {
                from_hour: 7,
                at: entity(100 + (id * 5) % 48),
                inside: false,
            },
            world_projection::RoutineStop {
                from_hour: 21,
                at: entity(100 + id * 2),
                inside: id % 5 != 0,
            },
        ];
        items.push(person);
    }
    ProjectionSnapshot {
        scenery: Some(Scenery {
            sky_top: 0x9cc6e6,
            sky_bottom: 0xf0ead8,
            far: 0x8fae7e,
            near: 0x4f86a8,
            sun: 0xffe2a0,
        }),
        canvas: CanvasProjection {
            items,
            marks: (0..62)
                .map(|n| world_projection::CanvasMark {
                    label: format!("Work {n}"),
                    shape: [MarkShape::House, MarkShape::Tree, MarkShape::Lamp][n % 3],
                    selection: None,
                })
                .collect(),
            width: Some(3.0),
            season: Some(Season::Winter),
            ground: Some(GroundCover::Snow),
            ice: true,
            ..Default::default()
        },
        ..ProjectionSnapshot::default()
    }
}

/// At 1440 by 900 at twice the pixels: painting the still layers
/// afresh (a new hour, the weather, the season, a zoom) off the
/// window's thread takes well under a second, and working out and
/// drawing a frame over them under 8 ms. (That no frame on the window's
/// thread waits for painting is `a_three_year_world_never_waits_for_painting`.) What this measures is the CPU's part: painting the
/// images, and working out every primitive a frame hands GPUI. The
/// GPU's part (compositing a few images and the live primitives) needs
/// a Mac to measure.
#[test]
#[ignore = "a benchmark: cargo test --release -p world-gpui -- --ignored three_year"]
fn a_three_year_world_paints_within_its_frame_budget() {
    use crate::brush::Tally;
    use std::time::{Duration, Instant};
    let snapshot = three_years();
    let (width, height, dpr) = (1440.0_f32, 900.0_f32, 2.0_f32);
    let mut report = Vec::new();
    let mut worst_still = Duration::ZERO;
    for (daylight, hour) in [
        (Daylight::Day, 12.0),
        (Daylight::Dusk, 19.5),
        (Daylight::Night, 22.5),
    ] {
        let stage = stage_at(&snapshot, width, height, Clock::at(hour as u8));
        let camera = Camera::around(&stage, 1.0, stage.width / 2.0, height / 2.0);
        let lives = living(&stage, &snapshot, 0.0, daylight, &BTreeSet::new(), None);
        let frame = frame(
            &snapshot,
            &stage,
            &lives,
            camera,
            0.0,
            daylight,
            &Glows::new(),
            1.0,
        );
        let mut frame = frame;
        frame.hour = hour;
        let started = Instant::now();
        let sky = paint_sky(&frame, width, height, dpr * SKY_RES).expect("a sky");
        let band = Band::of(&frame, width, height);
        let hills = paint_band(&frame, &band, dpr).expect("hills");
        // The ground as the window paints it: the land's tiles, the
        // buildings' sprites, then the buildings' tiles, each across the
        // painter's threads.
        let scale = dpr;
        let key = ground_key(&frame, scale).finish();
        let frame_ref = &frame;
        let land = tiles_in_view(&frame, Layer::Land, scale * 0.5, 0);
        let jobs = land
            .iter()
            .map(|&(column, row)| {
                Box::new(move || {
                    paint_land_tile(frame_ref, column, row, scale * 0.5).map(painter::image_of)
                })
                    as Box<dyn FnOnce() -> Option<std::sync::Arc<gpui::RenderImage>> + Send + '_>
            })
            .collect::<Vec<_>>();
        let mut tiles = painter::parallel(jobs).len();
        let places = tiles_in_view(&frame, Layer::Buildings, scale, 0);
        prepare_sprites(&frame, key, scale, &places);
        let jobs = places
            .iter()
            .map(|&(column, row)| {
                Box::new(move || {
                    paint_building_tile(frame_ref, key, column, row, scale).map(painter::image_of)
                })
                    as Box<dyn FnOnce() -> Option<std::sync::Arc<gpui::RenderImage>> + Send + '_>
            })
            .collect::<Vec<_>>();
        tiles += painter::parallel(jobs).into_iter().flatten().count();
        let vignette = paint_vignette(width, height, dpr * 0.25).expect("a vignette");
        let took = started.elapsed();
        worst_still = worst_still.max(took);
        // What an image costs to hand to GPUI: straight BGRA.
        let started = Instant::now();
        let _ = (
            painter::image(&sky),
            painter::image(&hills),
            painter::image(&vignette),
        );
        let upload = started.elapsed();
        report.push(format!(
            "{daylight:?}: still layers {:.1} ms ({tiles} tiles), images {:.1} ms",
            took.as_secs_f64() * 1000.0,
            upload.as_secs_f64() * 1000.0
        ));
    }
    // Panning into a new column of tiles.
    let stage = stage_at(&snapshot, width, height, Clock::at(12));
    let lives = living(
        &stage,
        &snapshot,
        0.0,
        Daylight::Day,
        &BTreeSet::new(),
        None,
    );
    let camera = Camera::around(&stage, 1.0, stage.width / 2.0, height / 2.0);
    let frame_now = frame(
        &snapshot,
        &stage,
        &lives,
        camera,
        0.0,
        Daylight::Day,
        &Glows::new(),
        1.0,
    );
    let started = Instant::now();
    let column = ((camera.x + width / 2.0) * dpr / TILE as f32) as i32 + 1;
    let key = ground_key(&frame_now, dpr).finish();
    for row in 0..8 {
        let _ = paint_land_tile(&frame_now, column / 2, row / 2, dpr * 0.5);
        let _ = paint_building_tile(&frame_now, key, column, row, dpr);
    }
    let pan = started.elapsed();
    // Frames: laying out, living, working out and drawing what moves.
    let runs = 120;
    let mut tally = Tally::default();
    let started = Instant::now();
    for run in 0..runs {
        let seconds = run as f32 / 60.0;
        let stage = stage_at(&snapshot, width, height, Clock::at(12));
        let lives = living(
            &stage,
            &snapshot,
            seconds,
            Daylight::Day,
            &BTreeSet::new(),
            None,
        );
        let frame = frame(
            &snapshot,
            &stage,
            &lives,
            camera,
            seconds,
            Daylight::Day,
            &Glows::new(),
            1.0,
        );
        let _ = ground_key(&frame, dpr).finish();
        paint_clouds(&mut tally, &frame, 0.0, 0.0, width, height);
        paint_live(&mut tally, &frame, 0.0, 0.0, width, height, [1.0; 3]);
        paint_weather(&mut tally, &frame, 0.0, 0.0, width, height, 1.0);
    }
    let each = started.elapsed() / runs;
    report.push(format!(
            "panning into a new column: {:.1} ms; a frame: {:.2} ms, {} quads, {} paths, {} soft shapes",
            pan.as_secs_f64() * 1000.0,
            each.as_secs_f64() * 1000.0,
            tally.quads / runs as usize,
            tally.paths / runs as usize,
            tally.soft / runs as usize,
        ));
    eprintln!("{}", report.join("\n"));
    // Where the time went, summed over every thread.
    eprintln!("{:?}", painter::profile().lock().unwrap());
    if !cfg!(debug_assertions) {
        // Painted off the window's thread, the still layers need only
        // arrive soon: well within a second, then a third of one to
        // fade in.
        assert!(worst_still < Duration::from_millis(400), "{report:?}");
        assert!(each < Duration::from_millis(8), "{report:?}");
    }
}

/// A panorama three windows wide puts what has a `px` where it says,
/// the camera pans along it without looking past either end, and
/// zoomed right out it sees the whole of it.
#[test]
fn a_panorama_puts_things_at_their_px_and_the_camera_keeps_to_it() {
    let mut snapshot = harbour();
    snapshot.canvas.width = Some(3.0);
    snapshot.canvas.items[0].px = Some(2.5);
    let stage = stage_at(&snapshot, 1000.0, 800.0, Clock::at(12));
    assert_eq!(stage.width, 3000.0);
    assert_eq!(stage.view_w, 1000.0);
    let far = stage.buildings.iter().find(|spot| spot.index == 0).unwrap();
    assert!((far.x - 2500.0).abs() < 0.01);
    let east = Camera::around(&stage, 1.0, 1e6, 400.0);
    assert!((east.x - 2500.0).abs() < 0.01, "{east:?}");
    let (right, _) = east.at(&stage, stage.width, 0.0);
    assert!((right - 1000.0).abs() < 0.01);
    let west = Camera::around(&stage, 1.0, -1e6, 400.0);
    assert!((west.x - 500.0).abs() < 0.01);
    // Zoomed right out it folds into a postcard: two rows, each as
    // wide as the window, the second up the hill behind the first, and
    // the water at the window's foot.
    let whole = Camera::around(&stage, 0.01, 0.0, 0.0);
    assert!((whole.fold - 1.0).abs() < 1e-4, "{whole:?}");
    assert!((whole.zoom - 2.0 / 3.0).abs() < 1e-4, "{whole:?}");
    let (left, bottom) = whole.at(&stage, 0.0, stage.height);
    let (right, _) = whole.at(&stage, 1499.0, stage.height);
    assert!(left.abs() < 0.01 && (right - 1000.0).abs() < 1.0);
    assert!((bottom - 800.0).abs() < 0.01);
    let (back_left, back_bottom) = whole.at(&stage, 1500.0, stage.height);
    assert!(back_left.abs() < 0.01 && back_bottom < bottom - 50.0);
    // Without a width, nothing changes from one window.
    let plain = stage_at(&harbour(), 1000.0, 800.0, Clock::at(12));
    assert_eq!(plain.width, plain.view_w);
}

/// People follow their day: at work by noon, home and indoors (not
/// standing outside) at night, and in the first moments of an hour
/// that moves them, on their way.
#[test]
fn people_keep_to_their_day_and_are_indoors_at_night() {
    let mut snapshot = harbour();
    let (home, work) = (entity(101), entity(102));
    for index in 5..snapshot.canvas.items.len() {
        snapshot.canvas.items[index].day = vec![
            world_projection::RoutineStop {
                from_hour: 8,
                at: work,
                inside: false,
            },
            world_projection::RoutineStop {
                from_hour: 21,
                at: home,
                inside: true,
            },
        ];
    }
    let noon = stage_at(&snapshot, 1100.0, 848.0, Clock::at(12));
    let work_x = noon
        .buildings
        .iter()
        .find(|spot| spot.index == 1)
        .unwrap()
        .x;
    for spot in noon.people.iter().filter(|spot| spot.index >= 5) {
        assert!((spot.x - work_x).abs() < noon.building_w * 1.5, "{spot:?}");
    }
    assert!(noon.inside.is_empty());
    let night = stage_at(&snapshot, 1100.0, 848.0, Clock::at(23));
    assert!(night.people.iter().all(|spot| spot.index < 5));
    assert_eq!(night.inside.len(), snapshot.canvas.items.len() - 5);
    assert!(night.inside.iter().all(|(_, place)| *place == 0));
    // Just after eight, on the way to work from home.
    let leaving = stage_at(
        &snapshot,
        1100.0,
        848.0,
        Clock {
            hour: 8,
            into_hour: 0.5,
        },
    );
    assert!(!leaving.routes.is_empty());
    let lives = living(
        &leaving,
        &snapshot,
        0.5,
        Daylight::Day,
        &BTreeSet::new(),
        None,
    );
    assert!(lives.iter().any(|life| life.pose.stride.is_some()));
}

/// The painter is a pure function of what it is given: the same tile
/// twice is the same pixels.
#[test]
fn the_same_frame_paints_the_same_pixels() {
    let snapshot = harbour();
    let stage = stage_at(&snapshot, 640.0, 420.0, Clock::at(19));
    let lives = living(
        &stage,
        &snapshot,
        0.0,
        Daylight::Dusk,
        &BTreeSet::new(),
        None,
    );
    let frame = frame(
        &snapshot,
        &stage,
        &lives,
        Camera::whole(&stage),
        0.0,
        Daylight::Dusk,
        &Glows::new(),
        1.0,
    )
    .at_hour(19.5);
    let key = ground_key(&frame, 2.0).finish();
    let row = (stage.base * 2.0 / TILE as f32) as i32;
    let a = paint_building_tile(&frame, key, 1, row, 2.0).map(|p| p.data().to_vec());
    let b = paint_building_tile(&frame, key, 1, row, 2.0).map(|p| p.data().to_vec());
    assert_eq!(a, b);
    let a = paint_land_tile(&frame, 0, 1, 1.0).unwrap();
    let b = paint_land_tile(&frame, 0, 1, 1.0).unwrap();
    assert!(a.data() == b.data());
    assert_eq!(
        paint_sky(&frame, 640.0, 420.0, 1.0).unwrap().data(),
        paint_sky(&frame, 640.0, 420.0, 1.0).unwrap().data()
    );
}

/// With Reduce Motion a click still waves, but nobody hops; and nobody
/// near a speaker is left facing away.
#[test]
fn reduce_motion_keeps_the_wave_and_drops_the_hop_and_heads_turn_to_a_speaker() {
    let snapshot = harbour();
    let stage = stage(&snapshot, 1100.0, 848.0);
    let who = snapshot.canvas.items[stage.people[0].index].id;
    let poked = [(who, 0.2)].into_iter().collect();
    let mut hop = living(
        &stage,
        &snapshot,
        0.0,
        Daylight::Day,
        &BTreeSet::new(),
        None,
    );
    wave(&mut hop, &stage, &snapshot, &poked, false);
    let mut still = living(
        &stage,
        &snapshot,
        0.0,
        Daylight::Day,
        &BTreeSet::new(),
        None,
    );
    let rest = still[0].pose;
    wave(&mut still, &stage, &snapshot, &poked, true);
    assert_eq!(still[0].stance, Some(Stance::Waving));
    assert_eq!(still[0].pose, rest);
    assert!(hop[0].pose.bob > rest.bob && hop[0].pose.squash != 1.0);

    let lives = living(
        &stage,
        &snapshot,
        0.0,
        Daylight::Day,
        &BTreeSet::new(),
        None,
    );
    let mut frame = frame(
        &snapshot,
        &stage,
        &lives,
        Camera::whole(&stage),
        0.0,
        Daylight::Day,
        &Glows::new(),
        1.0,
    );
    let speaker = frame.people[0].clone();
    frame.listen(speaker.index, 1.0);
    for person in frame.people.iter().skip(1) {
        if (person.x - speaker.x).abs() < speaker.height * 5.0 && person.pose.stride.is_none() {
            assert_eq!(person.pose.facing.signum(), (speaker.x - person.x).signum());
        }
    }
}

/// A step plants each foot: while it is on the ground it moves back
/// evenly under the body, and only lifts while it swings through.
#[test]
fn feet_plant_while_on_the_ground() {
    let planted = (0..50)
        .map(|n| art::step(n as f32 / 100.0, 1.0))
        .collect::<Vec<_>>();
    assert!(planted.iter().all(|(_, lift)| *lift == 0.0));
    let steps = planted
        .windows(2)
        .map(|pair| pair[1].0 - pair[0].0)
        .collect::<Vec<_>>();
    assert!(steps
        .iter()
        .all(|step| (step - steps[0]).abs() < 1e-4 && *step < 0.0));
    assert!(art::step(0.75, 1.0).1 > 0.9, "the foot is lifted mid-swing");
}

/// What the still layers are keyed by does not move with the seconds:
/// a frame later they are the same images, and nothing is painted
/// again until the hour, the weather, the season, the zoom or the size
/// changes.
#[test]
fn the_still_layers_keep_their_keys_as_time_passes() {
    let snapshot = harbour();
    let stage = stage_at(&snapshot, 1100.0, 848.0, Clock::at(12));
    let keys = |seconds: f32| {
        let lives = living(
            &stage,
            &snapshot,
            seconds,
            Daylight::Day,
            &BTreeSet::new(),
            None,
        );
        let frame = frame(
            &snapshot,
            &stage,
            &lives,
            Camera::whole(&stage),
            seconds,
            Daylight::Day,
            &Glows::new(),
            1.0,
        )
        .at_hour(12.2);
        let mut look = Key::new("look");
        look_key(&frame, &mut look);
        (look.finish(), ground_key(&frame, 2.0).finish())
    };
    assert_eq!(keys(0.0), keys(7.3));
    assert_eq!(keys(0.0), keys(61.0));
}

/// Every slope in the painted layers is smooth: a 30° roof edge is
/// anti-aliased, and the light, the ink and the snow laid along it
/// follow it by coverage, never in whole-pixel steps. A staircase shows
/// as rows that take the effect in alternately larger and smaller
/// amounts; a smooth edge takes the same amount on every row.
#[test]
fn roof_slopes_are_smooth_and_what_is_laid_along_them_has_no_steps() {
    let roof = art::hex(0x3f6a8a);
    let slope = (30.0_f32).to_radians().tan();
    let paint = || {
        let mut canvas = Canvas::new(320, 140, 1.0, (0.0, 0.0)).unwrap();
        art::polygon(
            &mut canvas,
            &[
                (160.0, 20.0),
                (310.0, 20.0 + 150.0 * slope),
                (10.0, 20.0 + 150.0 * slope),
            ],
            roof,
        );
        canvas.pixmap
    };
    let bare = paint();
    let at = |pixmap: &sk::Pixmap, x: usize, y: usize| {
        let pixel = &pixmap.data()[(y * 320 + x) * 4..][..4];
        [
            pixel[0] as f32,
            pixel[1] as f32,
            pixel[2] as f32,
            pixel[3] as f32,
        ]
    };
    let rows = 40..90;
    // The edge itself is anti-aliased: every row crosses it through
    // pixels partly covered.
    for y in rows.clone() {
        let partial = (160..320).any(|x| (20.0..235.0).contains(&at(&bare, x, y)[3]));
        assert!(partial, "row {y} of the slope has no anti-aliased pixel");
    }
    // Where along each row of one slope an effect lies: the middle of
    // what it changed. Along a straight edge those middles lie on a
    // straight line; in steps, they jump about it.
    let middles = |before: &sk::Pixmap,
                   after: &sk::Pixmap,
                   rows: std::ops::Range<usize>,
                   xs: std::ops::Range<usize>| {
        rows.map(|y| {
            let (mut sum, mut weighted) = (0.0_f32, 0.0_f32);
            for x in xs.clone() {
                let (a, b) = (at(before, x, y), at(after, x, y));
                let change = (0..3).map(|c| (a[c] - b[c]).abs()).sum::<f32>();
                sum += change;
                weighted += change * x as f32;
            }
            (y as f32, if sum > 0.0 { weighted / sum } else { f32::NAN })
        })
        .collect::<Vec<_>>()
    };
    // A smooth line bends gently from row to row; steps jump.
    let straight = |name: &str, points: Vec<(f32, f32)>| {
        assert!(
            points.iter().all(|(_, x)| x.is_finite()),
            "{name} laid nothing"
        );
        let worst = points
            .windows(3)
            .map(|w| (w[2].1 - 2.0 * w[1].1 + w[0].1).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            worst < 0.3,
            "{name} steps along the slope: a jump of {worst:.2} px"
        );
    };
    let mut lit = paint();
    painter::rim(&mut lit, 1.0, 2, [1.0, 0.76, 0.48], 0.6);
    straight(
        "the low sun's line",
        middles(&bare, &lit, rows.clone(), 160..320),
    );
    let mut snowed = paint();
    painter::cap(&mut snowed, 4.0, [0.95, 0.97, 0.99], 0.94);
    straight("snow", middles(&bare, &snowed, rows.clone(), 0..160));
    // And the ink under an eave: the same roof turned over.
    let eave = || {
        let mut canvas = Canvas::new(320, 140, 1.0, (0.0, 0.0)).unwrap();
        art::polygon(
            &mut canvas,
            &[(10.0, 10.0), (310.0, 10.0), (160.0, 10.0 + 150.0 * slope)],
            roof,
        );
        canvas.pixmap
    };
    let bare = eave();
    let mut inked = eave();
    painter::ground_line(&mut inked, 1, 0.32);
    straight(
        "the ink under an eave",
        middles(&bare, &inked, 30..70, 0..160),
    );
}
