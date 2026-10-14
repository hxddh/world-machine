//! What residents are seen doing where their day has them (v0.30 S2d):
//! sitting on a bench's seat, a ring round a lamp at dusk, a keeper behind
//! the stall, companions together, a load carried and a dance; errands as
//! a function of the World and the clock; and nobody's feet in a building.

use super::living_stage as ls;
use super::*;
use world_projection::RoutineStop;

/// The fixture `name` (from `tests/fixtures`).
fn fixture(name: &str) -> ProjectionSnapshot {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("{name}.json"));
    let json = std::fs::read_to_string(path).expect("the fixture");
    let wire: world_pack_protocol::ProjectionSnapshotWire =
        serde_json::from_str(&json).expect("a snapshot");
    ProjectionSnapshot::try_from(wire).expect("a snapshot")
}

/// Every fixture the scene's tests have.
fn fixtures() -> Vec<(String, ProjectionSnapshot)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut names = std::fs::read_dir(dir)
        .expect("the fixtures")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then(|| path.file_stem()?.to_str().map(String::from))?
        })
        .collect::<Vec<_>>();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let snapshot = fixture(&name);
            (name, snapshot)
        })
        .collect()
}

const VIEW: (f32, f32) = (1100.0, 848.0);

/// The three-year town with a day of its residents rewritten from
/// eight in the morning: three on the bench, three round a lamp (two of
/// them together), one keeping a stall and one carrying a load to it, and
/// two dancing together in front of a house. What the record would say
/// (`RoutineStop`), so the stage reads it as it reads a Pack's.
pub(crate) struct Staged {
    pub(crate) snapshot: ProjectionSnapshot,
    pub(crate) bench: usize,
    pub(crate) lamp: usize,
    pub(crate) stall: usize,
    pub(crate) sitters: Vec<usize>,
    /// A fourth who comes to sit on the bench: no room on it, a stool beside.
    pub(crate) stool: usize,
    pub(crate) ring: Vec<usize>,
    pub(crate) keeper: usize,
    pub(crate) carrier: usize,
    pub(crate) dancers: (usize, usize),
}

pub(crate) fn staged() -> Staged {
    let mut snapshot = fixtures()
        .into_iter()
        .find(|(name, _)| name.ends_with("-day-1082"))
        .map(|(_, snapshot)| snapshot)
        .expect("the three-year fixture");
    let stage = stage_at(&snapshot, VIEW.0, VIEW.1, Clock::at(12));
    let items = &snapshot.canvas.items;
    let on_land = |shapes: &[MarkShape]| {
        stage
            .things
            .iter()
            .filter(|spot| stage.depth_of(spot) != Depth::Water)
            .filter(|spot| !stage.shown_from.contains_key(&spot.index))
            .find(|spot| {
                items[spot.index]
                    .shape
                    .is_some_and(|shape| shapes.contains(&shape))
            })
            .map(|spot| spot.index)
            .expect("one on the land")
    };
    let bench = on_land(&[MarkShape::Bench]);
    let lamp = on_land(&[MarkShape::Lamp, MarkShape::Lantern]);
    let stall = on_land(&[MarkShape::Stall]);
    let house = stage
        .buildings
        .iter()
        .find(|spot| items[spot.index].shape == Some(MarkShape::House))
        .map(|spot| spot.index)
        .expect("a house");
    let people = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.kind == CanvasItemKind::Actor && !item.day.is_empty())
        .map(|(index, _)| index)
        .take(11)
        .collect::<Vec<_>>();
    assert_eq!(people.len(), 11);
    let id = |index: usize| snapshot.canvas.items[index].id;
    let plan = [
        (people[0], bench, Stance::Sitting, None),
        (people[1], bench, Stance::Sitting, None),
        (people[2], bench, Stance::Sitting, None),
        (people[3], lamp, Stance::Talking, Some(people[4])),
        (people[4], lamp, Stance::Talking, Some(people[3])),
        (people[5], lamp, Stance::Talking, None),
        (people[6], stall, Stance::Talking, None),
        (people[7], stall, Stance::Carrying, None),
        (people[8], house, Stance::Dancing, Some(people[9])),
        (people[9], house, Stance::Dancing, Some(people[8])),
        (people[10], bench, Stance::Sitting, None),
    ];
    let plan = plan
        .iter()
        .map(|(person, at, stance, with)| (*person, id(*at), *stance, with.map(id)))
        .collect::<Vec<_>>();
    for (person, at, stance, with) in plan {
        let home = snapshot.canvas.items[person].day[0].at;
        snapshot.canvas.items[person].day = vec![
            RoutineStop {
                from_hour: 0,
                at: home,
                inside: true,
                stance: None,
                with: None,
            },
            RoutineStop {
                from_hour: 8,
                at,
                inside: false,
                stance: Some(stance),
                with,
            },
        ];
    }
    Staged {
        snapshot,
        bench,
        lamp,
        stall,
        sitters: people[0..3].to_vec(),
        stool: people[10],
        ring: people[3..6].to_vec(),
        keeper: people[6],
        carrier: people[7],
        dancers: (people[8], people[9]),
    }
}

/// The frame a window draws of `snapshot` at `clock`, `seconds` in, the
/// camera on stage `x`.
fn frame_at(snapshot: &ProjectionSnapshot, clock: Clock, x: f32, seconds: f32) -> (Stage, Frame) {
    let stage = stage_at(snapshot, VIEW.0, VIEW.1, clock);
    let daylight = crate::scene::daylight_at(u32::from(clock.hour));
    let living = living(&stage, snapshot, seconds, daylight, &BTreeSet::new(), None);
    let camera = Camera::around(&stage, 1.0, x, VIEW.1 / 2.0);
    let frame = frame(
        snapshot,
        &stage,
        &living,
        camera,
        seconds,
        daylight,
        &Glows::new(),
        1.0,
    );
    (stage, frame)
}

fn spot_of(stage: &Stage, index: usize) -> Spot {
    *stage
        .things
        .iter()
        .chain(&stage.people)
        .find(|spot| spot.index == index)
        .expect("on the stage")
}

fn drawn(frame: &Frame, index: usize) -> &PersonPaint {
    frame
        .people
        .iter()
        .find(|person| person.index == index)
        .expect("drawn")
}

/// Whoever's day sits them on a bench sits on its seat: spread along the
/// whole plank, both legs showing, a fourth on a stool beside it; feet on the ground at its foot, hips at the top of its plank; never
/// a seat of their own, never floating (the art bible's §3 and §5).
#[test]
fn sitters_feet_on_the_seat() {
    let staged = staged();
    let clock = Clock::at(15);
    let stage = stage_at(&staged.snapshot, VIEW.0, VIEW.1, clock);
    let bench = spot_of(&stage, staged.bench);
    let (stage, frame) = frame_at(&staged.snapshot, clock, bench.x, 3.0);
    let setting = art::Setting::from_key(staged.snapshot.canvas.setting.as_deref());
    let seat = seat_height(&stage, &staged.snapshot, &bench, setting);
    let thing = frame
        .things
        .iter()
        .find(|thing| thing.index == staged.bench)
        .expect("the bench drawn");
    let (bench_x, bench_base) = frame.at(thing.x, thing.base);
    let mut xs = Vec::new();
    for sitter in &staged.sitters {
        let what = stage.doing[sitter];
        assert_eq!(what.fixture, Some((staged.bench, Use::Seat)));
        let person = drawn(&frame, *sitter);
        assert_eq!(person.stance, Stance::Sitting);
        // Feet at the bench's foot, just in front of it.
        assert!(
            (person.y - bench_base).abs() <= 1.0,
            "feet at {} and the bench's foot at {bench_base}",
            person.y
        );
        // Along the seat.
        assert!(
            (person.x - bench_x).abs() <= thing.w * 0.4,
            "{} off the bench at {bench_x}",
            person.x
        );
        // The seat's height, for theirs.
        let ratio = person.seat.expect("on the bench's seat");
        assert!((ratio * person.height - seat).abs() < 1.0);
        xs.push(person.x);
        // Drawn on it: the shoes on the ground, the hips on the plank.
        let drawing = person.drawing.as_ref().expect("a drawing");
        let on = super::people::seated(drawing, ratio, false);
        let drop = world_projection::drop_of(Stance::Sitting);
        let bottoms = on
            .parts
            .iter()
            .filter(|part| part.shows_in(Stance::Sitting))
            .filter_map(|part| match &part.shape {
                world_projection::DrawShape::Rect { y, .. } => Some(*y - drop),
                _ => None,
            })
            .collect::<Vec<_>>();
        let lowest = bottoms.iter().copied().fold(f32::MAX, f32::min);
        assert!(
            lowest.abs() < 0.01,
            "the shoes on the ground, not at {lowest}"
        );
        // Both legs: two shoes on the ground, one behind the other.
        let shoes = bottoms.iter().filter(|bottom| bottom.abs() < 0.01).count();
        assert!(shoes >= 2, "{shoes} shoe on the ground");
        let hips = on
            .parts
            .iter()
            .filter(|part| part.stances == [Stance::Sitting])
            .filter_map(|part| match &part.shape {
                world_projection::DrawShape::Rect { y, h, .. } if *y + *h > 0.36 + ratio - 0.2 => {
                    Some(*y - drop)
                }
                _ => None,
            })
            .fold(f32::MIN, f32::max);
        assert!(
            (hips - ratio).abs() < 0.05,
            "hips at {hips}, the seat at {ratio}"
        );
    }
    xs.sort_by(f32::total_cmp);
    for pair in xs.windows(2) {
        assert!(pair[1] - pair[0] >= stage.figure_h * people::SEAT_ROOM * 0.9);
    }
    // Along the whole plank, not bunched in its middle.
    let spread = xs.last().copied().unwrap_or(0.0) - xs.first().copied().unwrap_or(0.0);
    assert!(
        spread >= thing.w * 0.55,
        "{spread} of a bench {} wide",
        thing.w
    );
    // A fourth, with no room on it, sits on a stool beside it.
    assert_eq!(
        stage.doing[&staged.stool].fixture,
        Some((staged.bench, Use::Beside))
    );
    let stool = drawn(&frame, staged.stool);
    assert_eq!(stool.stance, Stance::Sitting);
    assert!(stool.seat.is_none(), "a seat of their own");
    assert!((stool.x - bench_x).abs() > thing.w * 0.45);
}

/// Two whose day has them somewhere together stand side by side, facing
/// each other, talking round a lamp or dancing.
#[test]
fn pairs_adjacent_and_facing() {
    let staged = staged();
    for (hour, (a, b)) in [(15, staged.dancers), (20, (staged.ring[0], staged.ring[1]))] {
        let clock = Clock::at(hour);
        let stage = stage_at(&staged.snapshot, VIEW.0, VIEW.1, clock);
        let daylight = crate::scene::daylight_at(hour as u32);
        let at = |index: usize| {
            stage
                .people
                .iter()
                .position(|spot| spot.index == index)
                .expect("out")
        };
        for seconds in [0.0, 13.0, 41.0] {
            let lives = living(
                &stage,
                &staged.snapshot,
                seconds,
                daylight,
                &BTreeSet::new(),
                None,
            );
            let (la, lb) = (lives[at(a)], lives[at(b)]);
            let (sa, sb) = (stage.people[at(a)], stage.people[at(b)]);
            assert_eq!(stage.doing[&a].with, Some(b));
            let apart = (la.x - lb.x).abs();
            assert!(
                apart <= stage.figure_h * people::TOGETHER * 1.6 && apart > stage.figure_h * 0.3,
                "{apart} apart at {hour}"
            );
            assert!((sa.y - sb.y).abs() < stage.figure_h * 0.5);
            assert_eq!(
                la.pose.facing.signum(),
                (lb.x - la.x).signum(),
                "facing each other"
            );
            assert_eq!(
                lb.pose.facing.signum(),
                (la.x - lb.x).signum(),
                "facing each other"
            );
        }
    }
    // The dancers dance, a beat at a time.
    let (_, frame) = frame_at(&staged.snapshot, Clock::at(15), 0.0, 2.0);
    let beats = (0..8)
        .map(|step| {
            let (_, frame) = frame_at(&staged.snapshot, Clock::at(15), 0.0, step as f32 * 0.21);
            drawn(&frame, staged.dancers.0).beat
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(drawn(&frame, staged.dancers.0).stance, Stance::Dancing);
    assert!(beats.len() >= 3, "{beats:?}");
}

/// After dusk those gathered at a lamp stand round it in a ring, either
/// side and a little in front, facing it, never in its post nor hidden
/// behind it.
#[test]
fn gatherers_outside_the_lamp_post_footprint() {
    let staged = staged();
    for hour in [19, 20, 21] {
        let clock = Clock::at(hour);
        let stage = stage_at(&staged.snapshot, VIEW.0, VIEW.1, clock);
        let lamp = spot_of(&stage, staged.lamp);
        let (stage, frame) = frame_at(&staged.snapshot, clock, lamp.x, 5.0);
        let setting = art::Setting::from_key(staged.snapshot.canvas.setting.as_deref());
        let w = thing_width(&stage, &staged.snapshot, &lamp, setting);
        let p = stage.figure_h * lamp.scale;
        // The post, and a body's half width round it.
        let post = w * 0.2 + p * 0.2;
        let daylight = crate::scene::daylight_at(hour as u32);
        let lives = living(
            &stage,
            &staged.snapshot,
            5.0,
            daylight,
            &BTreeSet::new(),
            None,
        );
        for gatherer in &staged.ring {
            assert_eq!(
                stage.doing[gatherer].fixture,
                Some((staged.lamp, Use::Ring))
            );
            let at = stage
                .people
                .iter()
                .position(|spot| spot.index == *gatherer)
                .expect("out");
            let (spot, life) = (stage.people[at], lives[at]);
            assert!(
                (life.x - lamp.x).abs() > post,
                "{} in the post at {}",
                life.x,
                lamp.x
            );
            assert!(
                (life.x - lamp.x).abs() < p * 2.0,
                "{} far from the lamp at {}",
                life.x,
                lamp.x
            );
            assert!(spot.y >= lamp.y, "never behind the post");
            assert!(spot.y - lamp.y < p, "round it, not out in front");
            // Facing it, or whoever they came with, beside them.
            let toward = match stage.doing[gatherer].with {
                Some(with) => spot_of(&stage, with).x,
                None => lamp.x,
            };
            assert_eq!(
                life.pose.facing.signum(),
                (toward - life.x).signum(),
                "facing in"
            );
            assert_eq!(drawn(&frame, *gatherer).stance, Stance::Talking);
        }
    }
}

/// A stall's keeper stands behind it, drawn before it so its counter hides
/// their legs; whoever brings it a load stands at its side with it.
#[test]
fn the_keeper_stands_behind_the_stall_in_depth() {
    let staged = staged();
    let clock = Clock::at(11);
    let stage = stage_at(&staged.snapshot, VIEW.0, VIEW.1, clock);
    let stall = spot_of(&stage, staged.stall);
    let (stage, frame) = frame_at(&staged.snapshot, clock, stall.x, 1.0);
    assert_eq!(
        stage.doing[&staged.keeper].fixture,
        Some((staged.stall, Use::Keep))
    );
    let keeper = spot_of(&stage, staged.keeper);
    let p = stage.figure_h * stall.scale;
    assert!((keeper.x - stall.x).abs() < p * 0.2);
    assert!(
        keeper.y < stall.y && keeper.y > stall.y - p * 0.4,
        "just behind"
    );
    // In the window's own plan: the keeper's picture is drawn before every
    // picture of the stall it meets, so the counter hides their legs.
    let thing = frame
        .things
        .iter()
        .find(|thing| thing.index == staged.stall)
        .expect("drawn");
    // The carrier with their load at its side.
    assert_eq!(
        stage.doing[&staged.carrier].fixture,
        Some((staged.stall, Use::Beside))
    );
    let carrier = drawn(&frame, staged.carrier);
    assert_eq!(carrier.stance, Stance::Carrying);
    assert!((carrier.x - frame.at(thing.x, 0.0).0).abs() > thing.w * 0.45);
    let person = drawn(&frame, staged.keeper).clone();
    let frame = std::sync::Arc::new(frame);
    let mut hand = super::stage_view::Stagehand::new(1.0);
    let (plan, ..) = hand.frame(&frame, &staged.snapshot, true);
    let (plan, ..) = if plan.images() == 0 {
        hand.frame(&frame, &staged.snapshot, true)
    } else {
        (plan, Vec::new(), Vec::new(), Default::default())
    };
    let mut stall_pictures = std::collections::HashSet::new();
    let mut baked = std::collections::HashSet::new();
    for slot in &hand.slots {
        for (item, _, sprite) in &slot.look.things {
            if *item == staged.stall {
                stall_pictures.insert(std::sync::Arc::as_ptr(&sprite.image));
            }
        }
        for image in slot.look.images() {
            baked.insert(std::sync::Arc::as_ptr(&image));
        }
    }
    let order = plan
        .layers
        .iter()
        .flat_map(|layer| layer.draws.iter())
        .enumerate()
        .filter_map(|(at, draw)| match draw {
            ls::Draw::Image { image, .. } => {
                Some((at, std::sync::Arc::as_ptr(image), draw.bounds()))
            }
            ls::Draw::Live { .. } => None,
        })
        .collect::<Vec<_>>();
    // The keeper's pose: a picture no look holds, standing where they do.
    let keeper = order
        .iter()
        .filter(|(_, image, _)| !baked.contains(image))
        .filter(|(_, _, (x, y, w, h))| {
            person.x > *x && person.x < x + w && (y + h - person.y).abs() < person.height * 0.3
        })
        .map(|(at, _, bounds)| (*at, *bounds))
        .collect::<Vec<_>>();
    assert!(!keeper.is_empty(), "the keeper's pose is drawn");
    let mut met = 0;
    for (at, image, (x, y, w, h)) in &order {
        if !stall_pictures.contains(image) {
            continue;
        }
        for (keeper_at, (kx, ky, kw, kh)) in &keeper {
            if kx < &(x + w) && x < &(kx + kw) && ky < &(y + h) && y < &(ky + kh) {
                met += 1;
                assert!(keeper_at < at, "the keeper is drawn over the stall");
            }
        }
    }
    assert!(met > 0, "the keeper stands where the stall is");
    // Where people are painted over the still layers (the strip, covers),
    // the stall is painted again over its keeper.
    assert_eq!(person.behind, Some(staged.stall));
}

/// Where everyone is and what they do is a function of the World and the
/// clock: the same snapshot at the same moment is the same town whatever
/// `seconds` says (which only breathes and turns heads), errands included,
/// and over an hour errands do take people out and back.
#[test]
fn living_is_a_function_of_snapshot_and_clock() {
    let staged = staged();
    let mut moved = 0;
    for into_hour in (0..3600).step_by(30) {
        let clock = Clock {
            hour: 10,
            into_hour: into_hour as f32,
        };
        let stage = stage_at(&staged.snapshot, VIEW.0, VIEW.1, clock);
        let again = stage_at(&staged.snapshot, VIEW.0, VIEW.1, clock);
        assert_eq!(stage, again);
        let at = |seconds: f32| {
            living(
                &stage,
                &staged.snapshot,
                seconds,
                Daylight::Day,
                &BTreeSet::new(),
                None,
            )
        };
        let (a, b) = (at(2.0), at(37.0));
        assert_eq!(a, at(2.0));
        for ((one, other), spot) in a.iter().zip(&b).zip(&stage.people) {
            assert!(
                (one.x - other.x).abs() <= 6.0,
                "{} moved with seconds",
                spot.index
            );
            assert_eq!(one.pose.stride.is_some(), other.pose.stride.is_some());
            assert_eq!((one.using, one.loaded), (other.using, other.loaded));
            if (one.x - spot.x).abs() > stage.figure_h {
                moved += 1;
            }
        }
    }
    assert!(moved > 0, "nobody ran an errand all hour");
    // A pure function of who, the day, the hour and how far into it.
    let errand = |day, hour, into| people::errand_at(0x5eed, day, hour, into, 12.0);
    assert_eq!(errand(4, 10, 700.0), errand(4, 10, 700.0));
    let busy = (0..3600)
        .filter(|second| errand(4, 10, *second as f32).is_some())
        .count();
    assert!(busy > 300 && busy < 2400, "{busy} s of the hour on errands");
    assert_ne!(
        (0..3600)
            .map(|second| errand(4, 10, second as f32).is_some())
            .collect::<Vec<_>>(),
        (0..3600)
            .map(|second| errand(5, 10, second as f32).is_some())
            .collect::<Vec<_>>(),
        "another day, other errands"
    );
    // Never over an hour's end.
    for hour in 0..24 {
        assert!(errand(4, hour, 3597.0).is_none() && errand(4, hour, 3599.9).is_none());
    }
}

/// Carrying shows a load in the arms, and a dance moves from beat to beat.
#[test]
fn a_load_is_carried_and_a_dance_has_beats() {
    let staged = staged();
    let (_, frame) = frame_at(&staged.snapshot, Clock::at(11), 0.0, 1.0);
    let carrier = drawn(&frame, staged.carrier).clone();
    let picture = |person: &PersonPaint| {
        let mut canvas = painter::Canvas::new(80, 120, 1.0, (-40.0, -110.0)).expect("a canvas");
        paint_person_body(&mut canvas, &frame, person, (0.0, 0.0), [1.0, 1.0, 1.0]);
        canvas.pixmap.data().to_vec()
    };
    let carrying = picture(&PersonPaint {
        height: 90.0,
        ..carrier.clone()
    });
    let working = picture(&PersonPaint {
        height: 90.0,
        stance: Stance::Working,
        ..carrier.clone()
    });
    assert_ne!(carrying, working, "a load in the arms");
    let dancer = PersonPaint {
        height: 90.0,
        stance: Stance::Dancing,
        ..carrier
    };
    let beats = (0..4)
        .map(|beat| {
            picture(&PersonPaint {
                beat: Some(beat),
                ..dancer.clone()
            })
        })
        .collect::<Vec<_>>();
    for pair in beats.windows(2) {
        assert_ne!(pair[0], pair[1], "each beat its own");
    }
}

/// Nobody's feet are ever inside a building's footprint: not on an errand
/// at any moment of the hour, nor on a bench, round a lamp or behind a
/// stall, in every fixture at every hour of the day and the evening.
#[test]
fn nobody_on_an_errand_or_at_a_fixture_is_in_a_building() {
    let mut worlds = fixtures();
    worlds.push(("staged".into(), staged().snapshot));
    let mut inside = Vec::new();
    for (name, snapshot) in &worlds {
        for hour in [9, 12, 15, 19, 20, 21, 22] {
            let daylight = crate::scene::daylight_at(hour);
            for into_hour in (0..3600).step_by(20) {
                let clock = Clock {
                    hour: hour as u8,
                    into_hour: into_hour as f32,
                };
                let stage = stage_at(snapshot, VIEW.0, VIEW.1, clock);
                let footprints = people::footprints(&stage, snapshot);
                let lives = living(&stage, snapshot, 4.0, daylight, &BTreeSet::new(), None);
                for (spot, life) in stage.people.iter().zip(&lives) {
                    for footprint in &footprints {
                        if ls::inside(*footprint, life.x, spot.y) {
                            inside.push(format!(
                                "{name}: resident {} at ({:.0}, {:.0}) at {hour}h{into_hour}s is in {footprint:?}",
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

/// Every lamp on the land of the three-year town, with six gathered at it
/// from dusk (19 to 22): each of its ring stands on open ground within a
/// stride and a half of it, in no building's footprint, never at or behind
/// a building's foot where its picture is (v0.30 S2d round 2: a gatherer at
/// the hill beacon read as standing on a house's wall), and on no thing,
/// props too (round 3: the beacon's ring, moved down the hill, stood on the
/// bathing pool's rail); balanced either side of the lamp.
#[test]
fn every_lamp_ring_stands_on_open_ground() {
    let base = staged().snapshot;
    let stage = stage_at(&base, VIEW.0, VIEW.1, Clock::at(20));
    let items = &base.canvas.items;
    let lamps = stage
        .things
        .iter()
        .filter(|spot| stage.depth_of(spot) != Depth::Water)
        .filter(|spot| {
            matches!(
                items[spot.index].shape,
                Some(MarkShape::Lamp | MarkShape::Lantern)
            )
        })
        .map(|spot| spot.index)
        .collect::<Vec<_>>();
    assert!(lamps.len() >= 2, "{lamps:?}");
    let people = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.kind == CanvasItemKind::Actor && !item.day.is_empty())
        .map(|(index, _)| index)
        .skip(11)
        .take(6)
        .collect::<Vec<_>>();
    let setting = art::Setting::from_key(base.canvas.setting.as_deref());
    let mut wrong = Vec::new();
    // Each lamp as it stands, and again with a prop (a cart, a pool, a
    // crate) put down right beside it, where its ring would stand.
    let prop = stage
        .things
        .iter()
        .find(|spot| {
            matches!(
                items[spot.index].shape,
                Some(
                    MarkShape::Well | MarkShape::Planter | MarkShape::Parcel | MarkShape::Fountain
                )
            ) && items[spot.index].px.is_some()
                && stage.depth_of(spot) != Depth::Water
        })
        .map(|spot| spot.index)
        .expect("a prop");
    let cases = lamps
        .iter()
        .flat_map(|lamp| [(*lamp, false), (*lamp, true)])
        .collect::<Vec<_>>();
    for (lamp, crowded) in cases {
        let mut snapshot = base.clone();
        if crowded {
            let lamp_item = &snapshot.canvas.items[lamp];
            let (px, y) = (lamp_item.px.unwrap_or(0.0), lamp_item.y);
            let item = &mut snapshot.canvas.items[prop];
            item.px = Some(px + 0.05);
            item.y = y;
        }
        let at = snapshot.canvas.items[lamp].id;
        for person in &people {
            let home = snapshot.canvas.items[*person].day[0].at;
            snapshot.canvas.items[*person].day = vec![
                RoutineStop {
                    from_hour: 0,
                    at: home,
                    inside: true,
                    stance: None,
                    with: None,
                },
                RoutineStop {
                    from_hour: 18,
                    at,
                    inside: false,
                    stance: Some(Stance::Talking),
                    with: None,
                },
            ];
        }
        for hour in 19..=22 {
            let stage = stage_at(&snapshot, VIEW.0, VIEW.1, Clock::at(hour));
            let footprints = people::footprints(&stage, &snapshot);
            let (lamp_x, lamp_y) = (spot_of(&stage, lamp).x, spot_of(&stage, lamp).y);
            let (mut left, mut right) = (0_usize, 0_usize);
            for spot in &stage.people {
                if stage.doing.get(&spot.index).and_then(|what| what.fixture)
                    != Some((lamp, Use::Ring))
                {
                    continue;
                }
                let p = stage.figure_h * spot.scale;
                if spot.x < lamp_x {
                    left += 1;
                } else {
                    right += 1;
                }
                let inside = footprints
                    .iter()
                    .any(|footprint| ls::inside(*footprint, spot.x, spot.y));
                let on_a_wall = stage.buildings.iter().any(|building| {
                    let (w, h) = building_size(&stage, &snapshot, building, setting);
                    (spot.x - building.x).abs() < w / 2.0 + p * 0.25
                        && spot.y < building.y + p * 0.3
                        && spot.y > building.y - h - p * 0.2
                });
                // Near it: within a stride and a half.
                let near = (spot.x - lamp_x).abs() <= p * people::RING_MOST + 0.5
                    && (spot.y - lamp_y).abs() <= p + 0.5;
                // And on no thing, props too (a pool's rail, a pier).
                let on_a_thing = stage.things.iter().any(|other| {
                    let w = thing_width(&stage, &snapshot, other, setting).max(other.w);
                    other.index != lamp
                        && (spot.x - other.x).abs() < w / 2.0 + p * 0.1
                        && (spot.y - other.y).abs() < p * 0.4
                });
                if inside || on_a_wall || !near || on_a_thing {
                    wrong.push(format!(
                        "lamp {lamp} at {hour}h: {} at ({:.0}, {:.0}): in a building {inside}, \
                         on a wall {on_a_wall}, near {near}, on a thing {on_a_thing}",
                        spot.index, spot.x, spot.y
                    ));
                }
            }
            if left + right > 0 {
                assert!(
                    left.abs_diff(right) <= 2,
                    "lamp {lamp} at {hour}h: {left} left, {right} right"
                );
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Where people are painted over the still layers (the strip, covers,
/// postcards: v0.30 S2d round 2's close-ups), a stall's keeper is hidden
/// below its counter too: their legs leave no pixel there.
#[test]
fn the_keeper_s_legs_are_hidden_by_the_counter_when_painted() {
    use gpui::prelude::*;
    let staged = staged();
    let stage = stage_at(&staged.snapshot, VIEW.0, VIEW.1, Clock::at(11));
    let stall = spot_of(&stage, staged.stall);
    let living = living(
        &stage,
        &staged.snapshot,
        1.0,
        Daylight::Day,
        &BTreeSet::new(),
        None,
    );
    let frame = super::frame(
        &staged.snapshot,
        &stage,
        &living,
        Camera::around(&stage, 2.2, stall.x, VIEW.1 / 2.0),
        1.0,
        Daylight::Day,
        &Glows::new(),
        1.0,
    )
    .at_hour(11.0);
    let keeper = drawn(&frame, staged.keeper).clone();
    let without = Frame {
        people: frame
            .people
            .iter()
            .filter(|person| person.index != staged.keeper)
            .cloned()
            .collect(),
        ..frame.clone()
    };
    let picture = |frame: Frame| {
        crate::offscreen::draw(VIEW.0, VIEW.1, move || {
            let frame = frame.clone();
            gpui::canvas(
                |_, _, _| (),
                move |bounds, _, window, _| super::paint(&frame, bounds, window),
            )
            .size_full()
            .into_any_element()
        })
    };
    let (with, without) = (picture(frame), picture(without));
    let k = with.width() as f32 / VIEW.0;
    let (x0, x1) = (
        ((keeper.x - keeper.height * 0.12) * k) as u32,
        ((keeper.x + keeper.height * 0.12) * k) as u32,
    );
    let (y0, y1) = (
        ((keeper.y - keeper.height * 0.3) * k) as u32,
        (keeper.y * k) as u32,
    );
    let (mut differ, mut all) = (0, 0);
    for y in y0..y1.min(with.height()) {
        for x in x0..x1.min(with.width()) {
            all += 1;
            let (a, b) = (with.get_pixel(x, y).0, without.get_pixel(x, y).0);
            if a.iter().zip(b).any(|(a, b)| a.abs_diff(b) > 6) {
                differ += 1;
            }
        }
    }
    assert!(all > 0);
    assert!(
        differ * 20 <= all,
        "{differ} of {all} pixels of their legs show"
    );
}
