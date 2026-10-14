//! The living stage's own tests (v0.30 S1a): depth, the passes a frame
//! is drawn in, which looks a window keeps, that a frame is never blank
//! while a look is baked, and that nobody walks into a building.

use super::living_stage::{self as ls, compose, in_depth, Deep, Draw, Item, Rank};
use super::stage_view::{self as sv, Held, Stagehand, Variant, Want};
use super::*;
use std::sync::Arc;

/// Every fixture the scene's tests have: the first Pack's town through
/// three years, and the other Pack's places.
fn fixtures() -> Vec<(String, ProjectionSnapshot)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut paths = std::fs::read_dir(dir)
        .expect("the fixtures")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let json = std::fs::read_to_string(&path).ok()?;
            let wire: world_pack_protocol::ProjectionSnapshotWire =
                serde_json::from_str(&json).ok()?;
            let snapshot = ProjectionSnapshot::try_from(wire).ok()?;
            let name = path.file_stem()?.to_str()?.to_string();
            Some((name, snapshot))
        })
        .collect()
}

/// The first Pack's town on day 163.
fn day_163() -> ProjectionSnapshot {
    fixtures()
        .into_iter()
        .find(|(name, _)| name.ends_with("-day-163"))
        .map(|(_, snapshot)| snapshot)
        .expect("the day-163 fixture")
}

/// A plain picture-less draw over `rect`, told apart by its width.
fn draw(rect: (f32, f32, f32, f32)) -> Draw {
    Draw::Live {
        rect,
        paint: std::rc::Rc::new(|_, _| {}),
    }
}

/// Where each draw of `passes` lands in painter's order: its pass, and
/// its place in the pass.
fn order_of(passes: &[ls::Pass]) -> Vec<(ls::Rect, f32, usize)> {
    passes
        .iter()
        .enumerate()
        .flat_map(|(at, pass)| {
            pass.draws
                .iter()
                .map(move |draw| (draw.bounds(), pass.opacity, at))
        })
        .collect()
}

/// Whoever stands further back is drawn first; level, the props along the
/// spine, then buildings, things and people: so a building in front of
/// someone hides them, and someone in front of it stands over it.
#[test]
fn people_buildings_and_things_are_drawn_back_to_front() {
    let at = |at: f32, rank: Rank, who: usize, x: f32| Deep {
        at,
        rank,
        who,
        items: vec![Item {
            opacity: 1.0,
            draw: draw((x, 0.0, 1.0, 1.0)),
        }],
    };
    // A tower whose foot is at 500, someone behind it (480), someone
    // in front of it (520), a cart level with its foot, and the props of
    // the spine at 500 too.
    let order = in_depth(vec![
        at(520.0, Rank::Person, 7, 1.0),
        at(500.0, Rank::Thing, 3, 2.0),
        at(500.0, Rank::Building, 9, 3.0),
        at(480.0, Rank::Person, 8, 4.0),
        at(500.0, Rank::Props, 0, 5.0),
    ])
    .map(|item| item.draw.bounds().0)
    .collect::<Vec<_>>();
    assert_eq!(order, vec![4.0, 5.0, 3.0, 2.0, 1.0]);
}

/// The passes a frame is composed into keep painter's order wherever two
/// pictures overlap, and share a pass wherever they do not: a person in
/// front of a building is drawn after it even in another pass, and two
/// lights cross-fading show each picture once, its second light faded
/// straight over it before whoever stands in front.
#[test]
fn passes_keep_painters_order_where_pictures_overlap() {
    let building = (100.0, 0.0, 100.0, 100.0);
    let behind = (120.0, 50.0, 10.0, 30.0);
    let front = (150.0, 60.0, 10.0, 40.0);
    let far = (400.0, 60.0, 10.0, 40.0);
    let t = 0.4;
    let items = [behind, building, front, far]
        .into_iter()
        .flat_map(|rect| {
            [
                Item {
                    opacity: 1.0,
                    draw: draw(rect),
                },
                Item {
                    opacity: t,
                    draw: draw(rect),
                },
            ]
        })
        .collect::<Vec<_>>();
    let passes = compose(items.clone());
    let order = order_of(&passes);
    let pass_of = |rect, opacity: f32| {
        order
            .iter()
            .position(|(r, o, _)| *r == rect && *o == opacity)
            .map(|at| (order[at].2, at))
            .expect("drawn")
    };
    // Each overlapping pair keeps its order: the whole picture, its second
    // light over it, then whatever stands in front.
    let sequence = [behind, building, front];
    for pair in sequence.windows(2) {
        assert!(pass_of(pair[0], t) < pass_of(pair[1], 1.0), "{pair:?}");
    }
    for rect in [behind, building, front, far] {
        assert!(pass_of(rect, 1.0) < pass_of(rect, t));
    }
    // The one standing alone shares the first pair of passes.
    assert_eq!(pass_of(far, 1.0).0, 0);
    assert_eq!(pass_of(far, t).0, 1);
    // As many passes as the crowd is deep, not as it is many.
    assert_eq!(passes.len(), 6);
    // And a hundred people standing apart are two passes.
    let crowd = (0..100)
        .flat_map(|n| {
            let rect = (n as f32 * 20.0, 0.0, 10.0, 30.0);
            [
                Item {
                    opacity: 1.0,
                    draw: draw(rect),
                },
                Item {
                    opacity: t,
                    draw: draw(rect),
                },
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(compose(crowd).len(), 2);
}

fn want(level: usize, light: ls::Light, view: (f32, f32)) -> Want {
    Want {
        key: 1,
        level,
        light,
        variant: Variant::Main,
        view,
        shown: true,
    }
}

fn held(key: u64, level: usize, light: ls::Light, span: Option<(f32, f32)>, ready: bool) -> Held {
    Held {
        key,
        level,
        light,
        variant: Variant::Main,
        span,
        ready,
    }
}

/// While the look a frame wants is baked, the nearest look already baked
/// stands in for it: the other level of the same light, scaled; the World
/// as it was before it changed; another light; and never nothing while
/// anything of the place is ready. A close level baked around the camera
/// stands in only where it shows what the camera sees.
#[test]
fn a_look_being_baked_is_stood_in_for_never_left_blank() {
    use ls::Light::{Dusk, Noon};
    let view = (1000.0, 1600.0);
    let close = want(1, Noon, view);
    // Nothing ready at all: nothing to draw (the loading wash).
    assert_eq!(sv::choose(&[held(1, 1, Noon, None, false)], &close), None);
    // The whole level stands in for the close one being baked.
    let held_now = [
        held(1, 0, Noon, None, true),
        held(1, 1, Noon, Some((0.0, 900.0)), true),
    ];
    assert_eq!(sv::choose(&held_now, &close), Some(0));
    // Once the close one shows what the camera sees, it is drawn.
    let held_now = [
        held(1, 0, Noon, None, true),
        held(1, 1, Noon, Some((800.0, 2000.0)), true),
    ];
    assert_eq!(sv::choose(&held_now, &close), Some(1));
    // A close look stands in for the whole one (scaled down) before a
    // look of the World as it was, and that before another light.
    let whole = want(0, Noon, view);
    let held_now = [
        held(2, 0, Noon, None, true),
        held(1, 1, Noon, None, true),
        held(1, 0, Dusk, None, true),
    ];
    assert_eq!(sv::choose(&held_now, &whole), Some(1));
    assert_eq!(sv::choose(&held_now[..1], &whole), Some(0));
    assert_eq!(sv::choose(&held_now[2..], &whole), Some(0));
    // A look on its way to the display does not stand in yet.
    assert_eq!(sv::choose(&[held(1, 0, Noon, None, false)], &whole), None);
}

/// A window keeps only the looks its camera wants, and while one is not
/// ready, the one standing in for it; everything else is let go.
#[test]
fn looks_no_camera_is_near_are_let_go() {
    use ls::Light::{Dusk, Night, Noon};
    let view = (0.0, 1000.0);
    let wants = [want(0, Dusk, view), want(0, Night, view)];
    let held_now = [
        held(1, 0, Noon, None, true),  // a light the hour has left
        held(1, 1, Dusk, None, true),  // another level
        held(1, 0, Dusk, None, true),  // wanted
        held(1, 0, Night, None, true), // wanted
        held(2, 0, Dusk, None, true),  // the World as it was
    ];
    assert_eq!(
        sv::keep(&held_now, &wants),
        vec![false, false, true, true, false]
    );
    // While night is on its way to the display, dusk stands in for it and
    // the World as it was is still let go.
    let held_now = [
        held(1, 0, Dusk, None, true),
        held(1, 0, Night, None, false),
        held(2, 0, Night, None, true),
    ];
    assert_eq!(sv::keep(&held_now, &wants), vec![true, true, true]);
    // The World changed: its old look stays until the new one is ready.
    let held_now = [held(2, 0, Dusk, None, true)];
    assert_eq!(sv::keep(&held_now, &wants[..1]), vec![true]);
    let held_now = [held(2, 0, Dusk, None, true), held(1, 0, Dusk, None, true)];
    assert_eq!(sv::keep(&held_now, &wants[..1]), vec![false, true]);
    // With nothing to show, the whole level is baked before a close one.
    let close = [want(1, Dusk, view)];
    assert_eq!(
        sv::next_bake(&[], false, &close).map(|want| want.level),
        Some(0)
    );
    assert_eq!(sv::next_bake(&[], true, &close), None);
    let baked = [held(1, 0, Dusk, None, true)];
    assert_eq!(
        sv::next_bake(&baked, false, &close).map(|want| want.level),
        Some(1)
    );
}

/// A frame does one thing at a time with the looks on their way to the
/// display: it shows a look whose pictures the display has, or hands more
/// pictures over, or tells the display to forget pictures let go; never
/// a look shown on the frame that copies its last pictures. Forgetting
/// is put off only while a little is waiting.
#[test]
fn a_frame_shows_a_look_or_hands_pictures_over_never_both() {
    use sv::{chores, Chores};
    assert_eq!(
        chores(true, true, 10),
        Chores {
            swap: true,
            upload: false,
            release: 0
        }
    );
    assert_eq!(
        chores(false, true, 10),
        Chores {
            swap: false,
            upload: true,
            release: 0
        }
    );
    assert_eq!(
        chores(false, false, 6),
        Chores {
            swap: false,
            upload: false,
            release: 6
        }
    );
    // A frame forgets a few at most (24 at scale 2 came to a 50 ms frame).
    assert_eq!(chores(false, false, 24).release, sv::FREE_PER_FRAME);
    const { assert!(sv::FREE_PER_FRAME <= 8) };
    // A long queue to forget is forgotten a share a frame regardless.
    let long = chores(false, true, 10_000);
    assert!(long.upload && long.release > 0 && long.release < 10_000);
    assert!(chores(true, false, 10_000).release > 0);
    assert_eq!(chores(false, false, 0), Chores::default());
}

/// How much a frame hands to the display follows how fast the display
/// takes pictures (about a millisecond's worth, within bounds), halves
/// after a frame that came late, and grows back gradually.
#[test]
fn the_upload_budget_follows_the_display_and_backs_off_after_a_late_frame() {
    use sv::UploadBudget;
    let mut budget = UploadBudget::default();
    assert_eq!(budget.bytes, UploadBudget::FIRST);
    // A fast display: the most, reached a quarter a frame at a time.
    let mut last = budget.bytes;
    for _ in 0..40 {
        budget.adapt(4 << 20, 0.2, Some(8.3));
        assert!(budget.bytes <= last + last / 4 + 1);
        last = budget.bytes;
    }
    assert_eq!(budget.bytes, UploadBudget::MOST);
    // A late frame (four times the usual): half as much next.
    budget.adapt(0, 0.0, Some(40.0));
    assert_eq!(budget.bytes, UploadBudget::MOST / 2);
    // A slow display: about a millisecond's worth, never below the least.
    let mut slow = UploadBudget::default();
    for _ in 0..40 {
        slow.adapt(3 << 20, 2.0, Some(16.7));
    }
    let ms = (3 << 20) as f64 / 2.0 * UploadBudget::SPEND_MS;
    assert!((slow.bytes as f64 - ms).abs() < ms * 0.05, "{}", slow.bytes);
    let mut slowest = UploadBudget::default();
    for _ in 0..40 {
        slowest.adapt(1 << 20, 400.0, Some(16.7));
    }
    assert_eq!(slowest.bytes, UploadBudget::LEAST);
    // A display that is always slow (twenty frames a second) is not late.
    let mut steady = UploadBudget::default();
    for _ in 0..40 {
        steady.adapt(4 << 20, 0.2, Some(48.0));
    }
    assert_eq!(steady.bytes, UploadBudget::MOST);
    // Nor one that slows for good: soon its new pace is the usual one.
    for _ in 0..30 {
        budget.adapt(4 << 20, 0.2, Some(16.7));
    }
    for _ in 0..30 {
        budget.adapt(4 << 20, 0.2, Some(40.0));
    }
    assert_eq!(budget.bytes, UploadBudget::MOST);
}

/// A bake takes one thread on a machine of four cores or fewer, and
/// leaves two cores free on a larger one.
#[test]
fn a_bake_takes_one_thread_on_a_small_machine() {
    for cores in 1..=4 {
        assert_eq!(ls::bake_threads_for(cores), 1);
    }
    assert_eq!(ls::bake_threads_for(6), 4);
    assert_eq!(ls::bake_threads_for(16), 14);
}

/// The light an hour fades into is baked ahead, before its fade begins,
/// and only then: never at noon or late at dusk.
#[test]
fn the_next_light_is_baked_before_its_fade_begins() {
    use ls::Light::{Dusk, Night, Noon};
    let camera = Camera {
        x: 500.0,
        y: 400.0,
        zoom: 1.0,
        fold: 0.0,
    };
    let lights = |hour: f32| {
        sv::wants(1, (1000.0, 1000.0), camera, None, hour)
            .into_iter()
            .map(|want| (want.light, want.shown))
            .collect::<Vec<_>>()
    };
    assert_eq!(lights(12.0), vec![(Noon, true)]);
    // Before dusk's fade (16.5): dusk ahead, not shown.
    assert_eq!(lights(16.0), vec![(Noon, true), (Dusk, false)]);
    assert_eq!(lights(17.0), vec![(Noon, true), (Dusk, true)]);
    assert_eq!(lights(19.5), vec![(Dusk, true)]);
    assert_eq!(lights(20.0), vec![(Dusk, true), (Night, false)]);
    assert_eq!(lights(4.6), vec![(Night, true), (Noon, false)]);
}

/// A frame of `snapshot` in a window `view` large, through `camera`, at
/// `hour`, `seconds` in.
fn frame_of(
    snapshot: &ProjectionSnapshot,
    view: (f32, f32),
    camera: Option<(f32, f32)>,
    hour: f32,
    seconds: f32,
) -> Frame {
    let stage = super::stage(snapshot, view.0, view.1);
    let daylight = crate::scene::daylight_at(hour as u32);
    let living = super::living(
        &stage,
        snapshot,
        seconds,
        daylight,
        &Default::default(),
        None,
    );
    let camera = match camera {
        Some((zoom, x)) => Camera::around(&stage, zoom, x, view.1 / 2.0),
        None => Camera::around(&stage, 1.0, stage.width / 2.0, view.1 / 2.0),
    };
    super::frame(
        snapshot,
        &stage,
        &living,
        camera,
        seconds,
        daylight,
        &Glows::new(),
        1.0,
    )
    .at_hour(hour)
}

/// The living stage as a window drives it, its looks baked off the
/// thread: no frame after the first look is ready is ever blank, through
/// a zoom in, a pan along, an hour cross-fading and the World changing;
/// and once it settles it holds only the looks the camera wants.
#[test]
fn the_living_stage_is_never_blank_and_lets_go_of_what_it_no_longer_needs() {
    let snapshot = day_163();
    let view = (480.0, 320.0);
    let mut hand = Stagehand::new(1.0);
    let run = |hand: &mut Stagehand,
               snapshot: &ProjectionSnapshot,
               camera,
               hour: f32,
               until_settled: bool| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(240);
        let mut frames = 0;
        loop {
            let frame = Arc::new(frame_of(
                snapshot,
                view,
                camera,
                hour,
                frames as f32 * 0.016,
            ));
            let (plan, _, _, report) = hand.frame(&frame, snapshot, false);
            frames += 1;
            if hand.slots.iter().any(|slot| slot.held.ready) {
                assert!(
                    report.shown && plan.images() > 0,
                    "a blank frame {frames} in"
                );
            }
            if report.settled || (!until_settled && report.shown) {
                return frames;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the stage never settled"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    };
    // Opening: the first look arrives.
    run(&mut hand, &snapshot, None, 12.0, true);
    assert_eq!(hand.slots.len(), 1, "one look at rest by day");
    let at_rest = hand.bytes();
    // Zooming in: the close level is baked around the camera, the whole
    // one standing in meanwhile (and kept, for a pan).
    run(&mut hand, &snapshot, Some((2.0, 600.0)), 12.0, true);
    assert!(hand
        .slots
        .iter()
        .any(|slot| slot.held.level == 1 && slot.held.span.is_some()));
    // Panning along at that zoom, past what was baked.
    run(&mut hand, &snapshot, Some((2.0, 1400.0)), 12.0, true);
    // Back out: the close level is let go.
    run(&mut hand, &snapshot, None, 12.0, true);
    assert!(hand.slots.iter().all(|slot| slot.held.level == 0));
    let back = hand.bytes() - hand.pose_bytes();
    assert!(
        back.abs_diff(at_rest) * 20 < at_rest,
        "back to one look: {back} after {at_rest}"
    );
    // Dusk coming on: two lights, then one again.
    run(&mut hand, &snapshot, None, 18.0, true);
    assert_eq!(hand.slots.len(), 2, "two lights cross-fading");
    run(&mut hand, &snapshot, None, 19.5, true);
    assert_eq!(hand.slots.len(), 1, "one light again");
    // The World changes: the old look is drawn until the new one is ready.
    let mut grown = snapshot.clone();
    grown.canvas.season = Some(world_projection::Season::Winter);
    run(&mut hand, &grown, None, 19.5, true);
    assert_eq!(hand.slots.len(), 1);
}

/// Nobody's feet are ever inside a building's footprint: wherever their
/// routine takes them at any hour of the day, in every fixture, the
/// residents walk beside and in front of the buildings, never through
/// them (the art director's S1 blocker: a walker on a tower's door).
#[test]
fn nobody_walks_into_a_building() {
    let view = (1100.0, 848.0);
    let mut inside = Vec::new();
    for (name, snapshot) in fixtures() {
        let stage = super::stage(&snapshot, view.0, view.1);
        let frame = frame_of(&snapshot, view, None, 12.0, 0.0);
        let footprints = frame
            .standing
            .iter()
            .map(|(_, b)| (b.index, ls::footprint(b.x, b.base, b.w, b.h)))
            .collect::<Vec<_>>();
        for daylight in [crate::scene::Daylight::Day, crate::scene::Daylight::Dusk] {
            for step in 0..240 {
                let seconds = step as f32 * 0.5;
                let living = super::living(
                    &stage,
                    &snapshot,
                    seconds,
                    daylight,
                    &Default::default(),
                    None,
                );
                for (spot, life) in stage.people.iter().zip(&living) {
                    for (building, footprint) in &footprints {
                        if ls::inside(*footprint, life.x, spot.y) {
                            inside.push(format!(
                                "{name}: resident {} at ({:.0}, {:.0}) {seconds} s in is inside building {building} {footprint:?}",
                                spot.index, life.x, spot.y
                            ));
                        }
                    }
                }
            }
        }
    }
    inside.dedup_by(|a, b| a.split(" at ").next() == b.split(" at ").next());
    assert!(inside.is_empty(), "{}", inside.join("\n"));
}

/// Two worlds that differ in a building are two bake keys; the same world
/// at another hour or zoom is the same key.
#[test]
fn a_new_building_or_season_is_a_new_look_and_an_hour_or_zoom_is_not() {
    let snapshot = day_163();
    let view = (640.0, 400.0);
    let key = |snapshot: &ProjectionSnapshot, camera, hour| {
        sv::bake_key(&frame_of(snapshot, view, camera, hour, 0.0))
    };
    let at_rest = key(&snapshot, None, 12.0);
    assert_eq!(at_rest, key(&snapshot, None, 21.0));
    assert_eq!(at_rest, key(&snapshot, Some((2.0, 400.0)), 12.0));
    let mut winter = snapshot.clone();
    winter.canvas.season = Some(world_projection::Season::Winter);
    assert_ne!(at_rest, key(&winter, None, 12.0));
    let mut fewer = snapshot.clone();
    let building = fewer
        .canvas
        .items
        .iter()
        .position(|item| item.kind == CanvasItemKind::Place && item.px.is_some())
        .expect("a building");
    fewer.canvas.items[building].px = fewer.canvas.items[building].px.map(|px| px + 40.0);
    assert_ne!(at_rest, key(&fewer, None, 12.0));
}

/// Nobody standing on a pier, a sea wall's landing or a pool in the rocks,
/// or on the shore behind one, is hidden by it: what lies flat is ground,
/// and every resident's picture that meets one is drawn after it (v0.30
/// S1a's stone landing cut people off at the waist). Checked through the
/// window's own plan, the camera on each flat thing of the three-year
/// town, at several moments of everyone's day.
#[test]
fn nobody_is_hidden_by_what_they_stand_on() {
    let snapshot = fixtures()
        .into_iter()
        .find(|(name, _)| name.ends_with("-day-1082"))
        .map(|(_, snapshot)| snapshot)
        .expect("the three-year fixture");
    let view = (1100.0, 848.0);
    let whole = frame_of(&snapshot, view, None, 12.0, 0.0);
    let flats = whole
        .things
        .iter()
        .filter(|thing| ls::lies_flat(thing.shape, thing.base, whole.front))
        .map(|thing| thing.x)
        .collect::<Vec<_>>();
    assert!(!flats.is_empty(), "the town has a pier to stand on");
    let mut hand = Stagehand::new(1.0);
    let mut checked = 0;
    for x in flats {
        for seconds in [0.0, 7.0, 19.0, 31.0] {
            let frame = Arc::new(frame_of(&snapshot, view, Some((1.6, x)), 12.0, seconds));
            let (plan, ..) = hand.frame(&frame, &snapshot, true);
            // The flat things' pictures, and every picture of the looks.
            let mut flat = std::collections::HashSet::new();
            let mut baked = std::collections::HashSet::new();
            for slot in &hand.slots {
                for (item, _, sprite) in &slot.look.things {
                    if slot
                        .look
                        .anchors
                        .get(item)
                        .is_some_and(|anchor| anchor.flat)
                    {
                        flat.insert(Arc::as_ptr(&sprite.image));
                    }
                }
                for image in slot.look.images() {
                    baked.insert(Arc::as_ptr(&image));
                }
            }
            let order = plan
                .layers
                .iter()
                .enumerate()
                .flat_map(|(pass, layer)| layer.draws.iter().map(move |draw| (pass, draw)))
                .enumerate()
                .collect::<Vec<_>>();
            let image_of = |draw: &Draw| match draw {
                Draw::Image { image, .. } => Some(Arc::as_ptr(image)),
                Draw::Live { .. } => None,
            };
            for (at, (pass, draw)) in &order {
                let Some(image) = image_of(draw) else {
                    continue;
                };
                if !flat.contains(&image) {
                    continue;
                }
                // Residents' poses are the pictures no look holds.
                for (other_at, (other_pass, other)) in &order {
                    let Some(pose) = image_of(other) else {
                        continue;
                    };
                    if baked.contains(&pose) {
                        continue;
                    }
                    let (a, b) = (draw.bounds(), other.bounds());
                    let meets =
                        a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3;
                    if meets {
                        checked += 1;
                        assert!(
                            (other_pass, other_at) > (pass, at),
                            "a resident at {b:?} is drawn under the flat thing at {a:?}"
                        );
                    }
                }
            }
        }
    }
    assert!(checked > 0, "somebody stands on or behind a flat thing");
}

/// Bakes the look of `snapshot` in `light` at a device pixel a stage
/// pixel over `span` (all of it if none), taking from `before`.
fn bake_over(
    snapshot: &ProjectionSnapshot,
    view: (f32, f32),
    light: ls::Light,
    span: Option<(f32, f32)>,
    before: Option<&ls::Look>,
) -> ls::Look {
    let (frame, stage) = sv::prepare(snapshot, view, light, false);
    let cut = ls::Cut {
        span,
        people: false,
        fold_land: false,
    };
    let world = sv::bake_key(&frame);
    ls::bake_look(
        &frame,
        &stage,
        &[],
        (light, 1.0),
        view,
        cut,
        (world, before),
    )
}

/// Every sprite of a look, layer by layer: its layer, where it stands,
/// its item (if any) and its pixels.
fn pixels_of(look: &ls::Look) -> Vec<(&'static str, String, Vec<u8>)> {
    let of = |name: &'static str, tag: String, sprite: &ls::Sprite| {
        (
            name,
            format!("{tag} {:?} {:?}", sprite.rect, sprite.clip),
            sprite.image.as_bytes(0).unwrap_or_default().to_vec(),
        )
    };
    let mut out = Vec::new();
    for (name, sprites) in [
        ("sky", &look.sky),
        ("hills", &look.hills),
        ("land", &look.land),
        ("shadows", &look.shadows),
        ("back", &look.back),
        ("front", &look.front),
        ("puffs", &look.puffs),
    ] {
        out.extend(sprites.iter().map(|sprite| of(name, String::new(), sprite)));
    }
    out.extend(
        look.buildings
            .iter()
            .map(|(item, sprite)| of("building", item.to_string(), sprite)),
    );
    out.extend(
        look.glow
            .iter()
            .map(|(group, item, sprite)| of("halo", format!("{item} {group}"), sprite)),
    );
    out.extend(
        look.things
            .iter()
            .map(|(item, boat, sprite)| of("thing", format!("{item} {boat}"), sprite)),
    );
    out.extend(
        look.glint
            .iter()
            .map(|sprite| of("glint", String::new(), sprite)),
    );
    out.extend(
        look.water
            .iter()
            .map(|sprite| of("water", String::new(), sprite)),
    );
    out
}

/// How many of `look`'s pictures are `before`'s own.
fn taken_from(look: &ls::Look, before: &ls::Look) -> usize {
    let theirs = before
        .images()
        .iter()
        .map(Arc::as_ptr)
        .collect::<std::collections::HashSet<_>>();
    look.images()
        .iter()
        .filter(|image| theirs.contains(&Arc::as_ptr(image)))
        .count()
}

/// A look baked again takes what is the same from the look baked before
/// and is, pixel for pixel, the look baked from nothing (v0.30 S1b): the
/// camera moved on along a close level (the same World, another stretch
/// of it), and the World changed (a building moved along, every halo in
/// front of or behind it taken out again). By day and at night, when
/// what stands in front of a lit building is cut out of its halo.
#[test]
fn a_look_baked_again_is_the_same_as_one_baked_from_nothing() {
    let snapshot = day_163();
    let view = (480.0, 320.0);
    let width = super::stage(&snapshot, view.0, view.1).width;
    let mut moved = snapshot.clone();
    let building = moved
        .canvas
        .items
        .iter()
        .position(|item| item.kind == CanvasItemKind::Place && item.px.is_some())
        .expect("a building");
    moved.canvas.items[building].px = moved.canvas.items[building].px.map(|px| px + 0.05);
    for light in [ls::Light::Noon, ls::Light::Night] {
        // Along a close level: from one stretch to the next, overlapping.
        let first = Some((0.0, width * 0.6));
        let next = Some((width * 0.3, width * 0.9));
        let before = bake_over(&snapshot, view, light, first, None);
        let again = bake_over(&snapshot, view, light, next, Some(&before));
        let fresh = bake_over(&snapshot, view, light, next, None);
        assert!(
            taken_from(&again, &before) * 3 > again.count(),
            "{light:?}: a pan takes most of the look from before ({} of {})",
            taken_from(&again, &before),
            again.count()
        );
        assert!(
            pixels_of(&again) == pixels_of(&fresh),
            "{light:?}: a pan baked again is not the look baked from nothing"
        );
        assert_eq!(again.anchors, fresh.anchors);
        // The World changed: a building moved along.
        let before = bake_over(&snapshot, view, light, None, None);
        let again = bake_over(&moved, view, light, None, Some(&before));
        let fresh = bake_over(&moved, view, light, None, None);
        assert!(
            taken_from(&again, &before) > 0,
            "{light:?}: what did not change is taken from before"
        );
        assert!(
            pixels_of(&again) == pixels_of(&fresh),
            "{light:?}: a World changed baked again is not the look baked from nothing"
        );
        assert_eq!(again.anchors, fresh.anchors);
    }
}
