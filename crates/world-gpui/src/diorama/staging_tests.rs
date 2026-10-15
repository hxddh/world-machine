//! A recorded Event's scene on the living stage (v0.30 S3c): where the
//! cast stands and walks at its start, middle and end, the camera on
//! them, nobody in a building or on a prop, and only the recorded words
//! said. Every clock here is pinned: a scene is a function of its script,
//! the stage and `t`.

use super::living_stage as ls;
use super::staging::*;
use super::*;
use world_projection::{Act, AgeStage, Cue, Look, Mark, MomentKind, Prop, SceneScript};

/// The kinds a return film stages most, as the art director judges them.
const KINDS: [MomentKind; 7] = [
    MomentKind::Arrival,
    MomentKind::Quarrel,
    MomentKind::Festival,
    MomentKind::Build,
    MomentKind::Birth,
    MomentKind::Farewell,
    MomentKind::Storm,
];

const VIEW: (f32, f32) = (1100.0, 848.0);

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
        .map(|path| {
            let json = std::fs::read_to_string(&path).expect("a fixture");
            let wire: world_pack_protocol::ProjectionSnapshotWire =
                serde_json::from_str(&json).expect("a snapshot");
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            (
                name,
                ProjectionSnapshot::try_from(wire).expect("a snapshot"),
            )
        })
        .collect()
}

/// The three-year town at eleven, as the window lays it out.
fn town() -> (ProjectionSnapshot, Stage) {
    let snapshot = super::tests::real_1082();
    let stage = stage_at(&snapshot, VIEW.0, VIEW.1, Clock::at(11));
    (snapshot, stage)
}

fn scene(snapshot: &ProjectionSnapshot, stage: &Stage, kind: MomentKind) -> SceneScript {
    sample(snapshot, stage, kind).expect("a scene")
}

fn who(snapshot: &ProjectionSnapshot, id: SelectionId) -> usize {
    snapshot
        .canvas
        .items
        .iter()
        .position(|item| item.id == id)
        .expect("on the canvas")
}

fn place_x(stage: &Stage, snapshot: &ProjectionSnapshot, script: &SceneScript) -> f32 {
    let index = who(snapshot, script.place.expect("a place"));
    stage
        .buildings
        .iter()
        .find(|spot| spot.index == index)
        .expect("standing")
        .x
}

fn person(staged: &Staged, index: usize) -> &StagedPerson {
    staged.person(index).expect("staged")
}

/// What the camera shows of the stage: left and right.
fn seen(stage: &Stage, camera: Camera) -> (f32, f32) {
    let half = stage.view_w / camera.zoom / 2.0;
    (camera.x - half, camera.x + half)
}

#[test]
fn a_scene_is_a_function_of_its_script_its_stage_and_the_moment() {
    let (snapshot, stage) = town();
    for kind in KINDS {
        let script = scene(&snapshot, &stage, kind);
        for t in [0.0, 1.3, 4.0, script.seconds] {
            assert_eq!(
                stage_scene(&script, &stage, &snapshot, t),
                stage_scene(&script, &stage, &snapshot, t),
                "{kind:?} at {t}"
            );
        }
        // Held to its length.
        assert_eq!(
            stage_scene(&script, &stage, &snapshot, script.seconds + 5.0),
            stage_scene(&script, &stage, &snapshot, script.seconds),
        );
    }
}

#[test]
fn an_arrival_comes_on_from_the_edge_with_a_bag_and_is_waved_in() {
    let (snapshot, stage) = town();
    let script = scene(&snapshot, &stage, MomentKind::Arrival);
    let fig = stage.figure_h;
    let x = place_x(&stage, &snapshot, &script);
    let a = who(&snapshot, script.cast[0].0);
    let start = stage_scene(&script, &stage, &snapshot, 0.0);
    let middle = stage_scene(&script, &stage, &snapshot, 2.0);
    let end = stage_scene(&script, &stage, &snapshot, script.seconds);
    // From away to the left, walking in, to the place.
    assert!(person(&start, a).x < x - fig * 4.0, "{start:?}");
    let walking = person(&middle, a);
    assert!(
        walking.walk.is_some() && walking.facing > 0.0,
        "{walking:?}"
    );
    // At the place, on the nearest open ground to it.
    assert!((person(&end, a).x - x).abs() < fig * 6.0, "{end:?}");
    assert_eq!(person(&end, a).stance, Stance::Waving);
    // The bag goes with them.
    let bag = |staged: &Staged| {
        staged
            .props
            .iter()
            .find(|prop| prop.prop == Prop::Suitcase)
            .copied()
            .expect("the bag")
    };
    assert_eq!(bag(&middle).held_by, Some(a));
    assert!((bag(&middle).x - person(&middle, a).x).abs() < fig * 0.5);
    // The ferry at the left edge of the view, and gone by the end.
    let ferry = middle.props.iter().find(|prop| prop.prop == Prop::Ferry);
    assert!(ferry.is_some_and(|ferry| ferry.x < x), "{middle:?}");
    assert!(end.props.iter().all(|prop| prop.prop != Prop::Ferry));
    // Those already here wave at the end.
    for (id, _) in script.cast.iter().skip(1) {
        assert_eq!(person(&end, who(&snapshot, *id)).stance, Stance::Waving);
    }
}

#[test]
fn a_quarrel_has_two_face_to_face_and_one_goes_off_home() {
    let (snapshot, stage) = town();
    let script = scene(&snapshot, &stage, MomentKind::Quarrel);
    let fig = stage.figure_h;
    let (a, b) = (
        who(&snapshot, script.cast[0].0),
        who(&snapshot, script.cast[1].0),
    );
    let start = stage_scene(&script, &stage, &snapshot, 0.0);
    let (pa, pb) = (person(&start, a), person(&start, b));
    // Face to face, a clear gap between them, and cross.
    let gap = (pa.x - pb.x).abs();
    assert!(
        gap >= fig * APART * 0.99 && gap < fig * 3.0,
        "{pa:?} {pb:?}"
    );
    assert_eq!(pa.facing, (pb.x - pa.x).signum(), "facing each other");
    assert_eq!(pb.facing, (pa.x - pb.x).signum(), "facing each other");
    assert_eq!(pa.stance, Stance::Talking);
    assert_eq!(pa.mood, Some(world_projection::Mood::Cross));
    assert_eq!(pb.mood, Some(world_projection::Mood::Cross));
    // Half way, one has turned away and is walking home.
    let middle = stage_scene(&script, &stage, &snapshot, 4.5);
    assert!(person(&middle, b).walk.is_some(), "{middle:?}");
    // By the end they are home and gone in, or (a home too far to reach)
    // well away.
    let end = stage_scene(&script, &stage, &snapshot, script.seconds);
    match end.person(b) {
        None => assert!(end.indoors.contains(&b), "{end:?}"),
        Some(walked) => assert!((walked.x - pa.x).abs() > gap + fig * 2.0, "{end:?}"),
    }
    assert_eq!(person(&end, a).stance, Stance::LookingAround);
}

/// A quarrel reads in the body, not only in faces too small to see: one
/// squares up with hands on hips and the other folds their arms (never
/// the open hand of talking), the one who goes stomps off having turned
/// their back, and the one left folds their arms and turns away.
#[test]
fn a_quarrel_is_in_how_they_stand() {
    let (snapshot, stage) = town();
    let script = scene(&snapshot, &stage, MomentKind::Quarrel);
    let (a, b) = (
        who(&snapshot, script.cast[0].0),
        who(&snapshot, script.cast[1].0),
    );
    let mut t = 0.0;
    while t <= script.seconds {
        let staged = stage_scene(&script, &stage, &snapshot, t);
        for index in [a, b] {
            let Some(person) = staged.person(index) else {
                continue;
            };
            let posture = person.posture.expect("cross in the body");
            match person.walk {
                Some(_) => assert_eq!(posture, Posture::Stomping, "at {t}: {person:?}"),
                None => assert!(
                    matches!(posture, Posture::ArmsFolded | Posture::HandsOnHips),
                    "at {t}: {person:?}"
                ),
            }
        }
        t += 0.25;
    }
    let start = stage_scene(&script, &stage, &snapshot, 0.0);
    assert_eq!(person(&start, a).posture, Some(Posture::HandsOnHips));
    assert_eq!(person(&start, b).posture, Some(Posture::ArmsFolded));
    // Just before setting off, their back already turned.
    let leaving = script
        .cues
        .iter()
        .find(|cue| {
            cue.who == Some(script.cast[1].0)
                && cue.at > 0.0
                && matches!(cue.act, Act::WalkTo { .. })
        })
        .expect("one goes")
        .at;
    let going = stage_scene(&script, &stage, &snapshot, leaving + 0.5);
    let (from, to, ..) = person(&going, b).walk.expect("off");
    let turning = stage_scene(&script, &stage, &snapshot, leaving - 0.3);
    let pb = person(&turning, b);
    assert_eq!(pb.facing, (to - from).signum(), "{pb:?}");
    // Left alone, arms folded, turned from where the other went.
    let end = stage_scene(&script, &stage, &snapshot, script.seconds);
    let pa = person(&end, a);
    assert_eq!(pa.posture, Some(Posture::ArmsFolded));
    assert_eq!(pa.facing, (pa.x - to).signum(), "{pa:?} turned away");
}

#[test]
fn a_festival_gathers_everyone_under_bunting_to_dance() {
    let (snapshot, stage) = town();
    let script = scene(&snapshot, &stage, MomentKind::Festival);
    let fig = stage.figure_h;
    let x = place_x(&stage, &snapshot, &script);
    let start = stage_scene(&script, &stage, &snapshot, 0.0);
    let middle = stage_scene(&script, &stage, &snapshot, 2.0);
    let dancing = stage_scene(&script, &stage, &snapshot, 5.0);
    assert!(start.props.iter().any(|prop| prop.prop == Prop::Bunting));
    assert!(start.props.iter().any(|prop| prop.prop == Prop::Table));
    assert!(middle.people.iter().any(|person| person.walk.is_some()));
    for person in &dancing.people {
        assert_eq!(person.stance, Stance::Dancing, "{person:?}");
        assert!((person.x - x).abs() < fig * 5.0, "{person:?}");
        assert!(person.walk.is_none());
    }
    // The camera comes in closer after the dancing starts.
    let (before, after) = (start.camera.unwrap(), dancing.camera.unwrap());
    let end = stage_scene(&script, &stage, &snapshot, script.seconds)
        .camera
        .unwrap();
    assert!(end.zoom > before.zoom && after.zoom <= end.zoom);
}

#[test]
fn a_build_goes_up_in_scaffolding_and_comes_down_to_a_cheer() {
    let (snapshot, stage) = town();
    let script = scene(&snapshot, &stage, MomentKind::Build);
    let working = stage_scene(&script, &stage, &snapshot, 2.0);
    let end = stage_scene(&script, &stage, &snapshot, script.seconds);
    let scaffold = working
        .props
        .iter()
        .find(|prop| prop.prop == Prop::Scaffold)
        .copied()
        .expect("scaffolding");
    let tools = working
        .props
        .iter()
        .find(|prop| prop.prop == Prop::Tools)
        .copied()
        .expect("tools");
    // One work site: the tools against the scaffolding, at its foot, and
    // everyone working at it, on its ground.
    let fig = stage.figure_h;
    assert!(
        (tools.x - scaffold.x).abs() <= scaffold.w / 2.0 + tools.w,
        "{tools:?}"
    );
    assert!((tools.y - scaffold.y).abs() < 1.0, "{tools:?} {scaffold:?}");
    assert!(
        (scaffold.y - stage.feet).abs() < fig * 0.6,
        "on the people's ground"
    );
    for person in &working.people {
        assert!(
            (person.x - scaffold.x).abs() < scaffold.w / 2.0 + fig * 3.0,
            "{person:?} {scaffold:?}"
        );
    }
    assert!(working
        .people
        .iter()
        .all(|person| person.stance == Stance::Working));
    // At it with hammers.
    assert!(working
        .people
        .iter()
        .any(|person| person.posture == Some(Posture::Hammering)));
    assert!(end.props.is_empty(), "{:?}", end.props);
    assert!(end
        .people
        .iter()
        .all(|person| person.stance == Stance::Celebrating));
}

#[test]
fn a_birth_a_farewell_and_a_storm_play_their_parts() {
    let (snapshot, stage) = town();
    let fig = stage.figure_h;
    // A birth: the cradle out, the neighbours come in close and wave.
    let birth = scene(&snapshot, &stage, MomentKind::Birth);
    let x = place_x(&stage, &snapshot, &birth);
    let start = stage_scene(&birth, &stage, &snapshot, 0.0);
    let end = stage_scene(&birth, &stage, &snapshot, birth.seconds);
    assert!(start.props.iter().any(|prop| prop.prop == Prop::Cradle));
    for (id, _) in birth.cast.iter().skip(2) {
        let index = who(&snapshot, *id);
        // From further along to beside the place (the nearest open
        // ground to it).
        assert!(
            (person(&end, index).x - x).abs() < fig * 6.0,
            "{x} {:?}",
            person(&end, index)
        );
        assert!((person(&end, index).x - person(&start, index).x).abs() > 1.0);
        assert_eq!(person(&end, index).stance, Stance::Celebrating);
    }
    // A farewell: bag in hand, off to the right edge, the ferry waiting.
    let farewell = scene(&snapshot, &stage, MomentKind::Farewell);
    let a = who(&snapshot, farewell.cast[0].0);
    let start = stage_scene(&farewell, &stage, &snapshot, 0.0);
    let going = stage_scene(&farewell, &stage, &snapshot, 5.0);
    let end = stage_scene(&farewell, &stage, &snapshot, farewell.seconds);
    assert!(person(&going, a).walk.is_some() && person(&going, a).facing > 0.0);
    // Gone off the edge, or well on the way.
    match end.person(a) {
        None => assert!(end.indoors.contains(&a), "{end:?}"),
        Some(going) => assert!(going.x > person(&start, a).x + fig * 3.0),
    }
    let ferry = going.props.iter().find(|prop| prop.prop == Prop::Ferry);
    assert!(ferry.is_some_and(|ferry| ferry.x > person(&start, a).x));
    // A storm: rain from the start, everyone off home, a lamp lit.
    let storm = scene(&snapshot, &stage, MomentKind::Storm);
    let start = stage_scene(&storm, &stage, &snapshot, 0.0);
    let end = stage_scene(&storm, &stage, &snapshot, storm.seconds);
    assert!(start.rain && end.rain);
    assert!(start.storm < 0.1 && end.storm > 0.99, "the storm sets in");
    assert!(
        !end.indoors.is_empty() || end.people.iter().all(|person| person.walk.is_some()),
        "home and in, or still hurrying: {end:?}"
    );
    assert!(end.props.iter().any(|prop| prop.prop == Prop::Lamp));
    let moved = storm
        .cast
        .iter()
        .filter(|(id, _)| {
            let index = who(&snapshot, *id);
            end.indoors.contains(&index)
                || (person(&end, index).x - person(&start, index).x).abs() > fig
        })
        .count();
    assert!(moved >= 1, "someone hurried home");
}

/// A birth's cradle is never left alone: its parent stands beside it the
/// whole scene, on the people's line, back from the water.
#[test]
fn a_cradle_has_its_parent_beside_it_all_through() {
    let snapshot = super::tests::real_1082();
    // As the tests lay it out, and as the World window does (under its
    // gauges), at two hours.
    for (hour, height) in [(11, VIEW.1), (14, VIEW.1 - 52.0)] {
        let stage = stage_at(&snapshot, VIEW.0, height, Clock::at(hour));
        cradle_by_parent(&snapshot, &stage);
    }
}

fn cradle_by_parent(snapshot: &ProjectionSnapshot, stage: &Stage) {
    let (snapshot, stage) = (snapshot.clone(), stage.clone());
    let fig = stage.figure_h;
    let script = scene(&snapshot, &stage, MomentKind::Birth);
    let parent = who(&snapshot, script.cast[1].0);
    let mut t = 0.0;
    while t <= script.seconds {
        let staged = stage_scene(&script, &stage, &snapshot, t);
        let cradle = staged
            .props
            .iter()
            .find(|prop| prop.prop == Prop::Cradle)
            .expect("the cradle");
        let by = person(&staged, parent);
        assert!(by.walk.is_none(), "at {t}: {by:?}");
        assert!(
            (by.x - cradle.x).abs() < fig * 2.0,
            "at {t}: {by:?} {cradle:?}"
        );
        // On land, on its parent's line, back from the water's edge.
        assert!(
            (cradle.y - by.y).abs() < fig * 0.2,
            "at {t}: {cradle:?} {by:?}"
        );
        assert!(cradle.y < stage.front - fig * 0.5, "at {t}: {cradle:?}");
        t += 0.25;
    }
}

/// A farewell ends with its payoff in view: the one leaving aboard the
/// ferry, waving, and the ferry still there to be seen.
#[test]
fn a_farewell_ends_aboard_the_ferry() {
    let (snapshot, stage) = town();
    let script = scene(&snapshot, &stage, MomentKind::Farewell);
    let a = who(&snapshot, script.cast[0].0);
    let end = stage_scene(&script, &stage, &snapshot, script.seconds - 0.3);
    let leaver = person(&end, a);
    assert!(leaver.aboard, "{leaver:?}");
    assert_eq!(leaver.stance, Stance::Waving);
    let ferry = end
        .props
        .iter()
        .find(|prop| prop.prop == Prop::Ferry)
        .expect("the ferry");
    assert!(
        (leaver.x - ferry.x).abs() < ferry.w / 2.0,
        "{leaver:?} {ferry:?}"
    );
    let (left, right) = seen(&stage, end.camera.expect("a camera"));
    assert!(leaver.x > left && leaver.x < right, "in view");
    assert!(ferry.x - ferry.w / 2.0 < right, "the ferry in view");
}

/// Everyone of the town out in a storm's view heads home out of the rain,
/// or off out of the view: by its end nobody stands about on the shore, in
/// any fixture.
#[test]
fn a_storm_leaves_the_quay_empty() {
    for (name, snapshot) in fixtures() {
        let stage = stage_at(&snapshot, VIEW.0, VIEW.1, Clock::at(11));
        let Some(script) = sample(&snapshot, &stage, MomentKind::Storm) else {
            continue;
        };
        let end = stage_scene(&script, &stage, &snapshot, script.seconds);
        let (left, right) = seen(&stage, end.camera.expect("a camera"));
        let fig = stage.figure_h;
        let shown = |x: f32| x > left + fig * 0.2 && x < right - fig * 0.2;
        for person in &end.people {
            assert!(
                person.walk.is_some() || !shown(person.x),
                "{name}: {person:?} still out in it"
            );
        }
        for spot in stage.people.iter().filter(|spot| shown(spot.x)) {
            assert!(
                end.person(spot.index).is_some() || end.indoors.contains(&spot.index),
                "{name}: {spot:?} left standing in the rain"
            );
        }
    }
}

/// At each camera cue: once the camera has eased there, whoever it is on
/// is in view (it follows them); and when it is on the place, everyone
/// standing is in view by the end of the shot, but whoever the scene sends
/// off out of it (home in the rain, away on the ferry).
#[test]
fn the_cast_is_in_view_at_each_camera_cue() {
    let (snapshot, stage) = town();
    let fig = stage.figure_h;
    for kind in KINDS {
        let script = scene(&snapshot, &stage, kind);
        let cameras = script
            .cues
            .iter()
            .filter_map(|cue| match cue.act {
                Act::Camera { on, .. } => Some((cue.at, on)),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (n, (at, on)) in cameras.iter().enumerate() {
            let shot_end = cameras
                .get(n + 1)
                .map_or(script.seconds, |(next, _)| next - 0.01);
            let t = match on {
                Mark::Place(id) if script.place == Some(*id) => shot_end,
                _ => (at + CAMERA_EASE).min(shot_end),
            };
            let staged = stage_scene(&script, &stage, &snapshot, t);
            let (left, right) = seen(&stage, staged.camera.expect("a camera"));
            let shown = |x: f32| x > left + fig * 0.2 && x < right - fig * 0.2;
            let sent_off = |index: usize| {
                script.cues.iter().any(|cue| {
                    cue.at > 0.0
                        && cue.at <= t
                        && cue.who == Some(snapshot.canvas.items[index].id)
                        && matches!(
                            cue.act,
                            Act::WalkTo {
                                to: Mark::Home(_) | Mark::Offstage(_),
                                ..
                            }
                        )
                })
            };
            match on {
                Mark::Place(id) if script.place == Some(*id) => {
                    for person in staged
                        .people
                        .iter()
                        .filter(|p| p.walk.is_none() && !sent_off(p.index))
                    {
                        assert!(shown(person.x), "{kind:?} at {t}: {person:?}");
                    }
                }
                Mark::Place(id) => {
                    // (Unless they have gone in at their door.)
                    if let Some(person) = staged.person(who(&snapshot, *id)) {
                        assert!(shown(person.x), "{kind:?} at {t}: {person:?}");
                    }
                }
                _ => {}
            }
        }
    }
}

/// In every fixture, every kind, every quarter second: nobody staged has
/// their feet in a building; nobody standing is on a prop or on a thing;
/// none of the cast standing is less than a body's width from another
/// (unless a cue puts them right up against each other); and no prop that
/// stands on the ground (nor a pole of the bunting) is on a thing, a
/// building's face or behind a post in the water.
#[test]
fn nobody_staged_is_in_a_building_or_on_a_prop() {
    let mut scenes = 0;
    for ((name, snapshot), hour) in fixtures()
        .into_iter()
        .flat_map(|fixture| [(fixture.clone(), 11), (fixture, 14)])
    {
        let stage = stage_at(&snapshot, VIEW.0, VIEW.1, Clock::at(hour));
        let name = format!("{name} at {hour}");
        let footprints = footprints(&stage, &snapshot);
        let ground = Ground::of(&stage, &snapshot);
        let setting = art::Setting::from_key(snapshot.canvas.setting.as_deref());
        let fig = stage.figure_h;
        let things = stage
            .things
            .iter()
            .filter(|spot| {
                let depth = Depth::at(spot.y, stage.height);
                depth > Depth::Street && depth < Depth::Water
            })
            .filter(|spot| {
                let shape = snapshot.canvas.items[spot.index]
                    .shape
                    .unwrap_or(MarkShape::Parcel);
                !ls::lies_flat(shape, spot.y, stage.front)
            })
            .filter(|spot| (spot.y - stage.feet).abs() < fig * 0.9)
            .map(|spot| {
                (
                    spot.x,
                    thing_width(&stage, &snapshot, spot, setting).max(spot.w) / 2.0,
                )
            })
            .collect::<Vec<_>>();
        // Tall, thin things in the water in front of the shore.
        let posts = stage
            .things
            .iter()
            .filter(|spot| spot.y >= stage.front - 1.0)
            .filter(|spot| {
                matches!(
                    snapshot.canvas.items[spot.index].shape,
                    Some(
                        MarkShape::Signpost
                            | MarkShape::Lamp
                            | MarkShape::Lantern
                            | MarkShape::Flag
                    )
                )
            })
            .map(|spot| (spot.x, spot.w * 0.3 + fig * 0.3))
            .collect::<Vec<_>>();
        for kind in KINDS {
            let Some(script) = sample(&snapshot, &stage, kind) else {
                continue;
            };
            scenes += 1;
            let together = script
                .cues
                .iter()
                .filter_map(|cue| match cue.act {
                    Act::WalkTo {
                        to: Mark::Beside(other, 0),
                        ..
                    } => Some((cue.who?, other)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let with_part = script
                .cues
                .iter()
                .filter_map(|cue| {
                    let who = cue.who?;
                    snapshot.canvas.items.iter().position(|item| item.id == who)
                })
                .collect::<BTreeSet<_>>();
            let paired = |a: SelectionId, b: SelectionId| {
                together.contains(&(a, b)) || together.contains(&(b, a))
            };
            let mut t = 0.0;
            while t <= script.seconds {
                let staged = stage_scene(&script, &stage, &snapshot, t);
                let standing = staged
                    .people
                    .iter()
                    .filter(|person| person.walk.is_none())
                    .collect::<Vec<_>>();
                for person in &staged.people {
                    for footprint in &footprints {
                        assert!(
                            !ls::inside(*footprint, person.x, person.y),
                            "{name} {kind:?} at {t}: {person:?} in {footprint:?}"
                        );
                    }
                }
                for person in &standing {
                    for prop in staged.props.iter().filter(|prop| {
                        prop.held_by.is_none()
                            && matches!(
                                prop.prop,
                                Prop::Table
                                    | Prop::Cradle
                                    | Prop::Tools
                                    | Prop::Wreath
                                    | Prop::Lamp
                                    | Prop::Bench
                            )
                    }) {
                        let on = (person.x - prop.x).abs() < prop.w / 2.0 + fig * 0.21
                            && (person.y - prop.y).abs() < fig * 0.44;
                        assert!(!on, "{name} {kind:?} at {t}: {person:?} on {prop:?}");
                    }
                    for (x, half) in &things {
                        // At its side, or a step in front of it.
                        let on = (person.x - x).abs() < half + fig * 0.21
                            && (person.y - stage.feet).abs() < fig * 0.44;
                        assert!(!on, "{name} {kind:?} at {t}: {person:?} on a thing at {x}");
                    }
                    // Nobody of the town about is stood on either.
                    for spot in stage
                        .people
                        .iter()
                        .filter(|spot| !with_part.contains(&spot.index))
                    {
                        let on = (person.x - spot.x).abs() < fig * 0.42
                            && (person.y - spot.y).abs() < fig * 0.7;
                        assert!(!on, "{name} {kind:?} at {t}: {person:?} on {spot:?}");
                    }
                    for other in &standing {
                        if other.index == person.index || paired(person.who, other.who) {
                            continue;
                        }
                        let apart = (person.x - other.x).abs();
                        assert!(
                            apart >= fig * APART * 0.99 || (person.y - other.y).abs() >= fig * 0.7,
                            "{name} {kind:?} at {t}: {person:?} on {other:?}"
                        );
                    }
                }
                // Scaffolding, the whole of it, ladder too, stands clear
                // of every thing but what it is round, and of the posts.
                for prop in staged
                    .props
                    .iter()
                    .filter(|prop| prop.prop == Prop::Scaffold)
                {
                    let own = script.cues.iter().find_map(|cue| match cue.act {
                        Act::Prop {
                            prop: Prop::Scaffold,
                            at: Mark::Place(id),
                            ..
                        } => snapshot.canvas.items.iter().position(|item| item.id == id),
                        _ => None,
                    });
                    let own_x = own.and_then(|own| {
                        stage
                            .things
                            .iter()
                            .find(|spot| spot.index == own)
                            .map(|spot| spot.x)
                    });
                    let (from, to) = scaffold_span(prop.x, prop.w);
                    for (x, half) in things
                        .iter()
                        .filter(|(x, _)| own_x.is_none_or(|own| (own - x).abs() > 0.5))
                    {
                        assert!(
                            to <= x - half || from >= x + half,
                            "{name} {kind:?} at {t}: {prop:?} over a thing at {x}"
                        );
                    }
                    for (x, reach) in &posts {
                        assert!(
                            to <= x - reach * 0.5 || from >= x + reach * 0.5,
                            "{name} {kind:?} at {t}: {prop:?} over a post at {x}"
                        );
                    }
                }
                // What stands on the ground stands on open ground.
                for prop in staged.props.iter().filter(|prop| prop.held_by.is_none()) {
                    let feet = match prop.prop {
                        Prop::Table | Prop::Cradle | Prop::Wreath | Prop::Lamp | Prop::Bench => {
                            vec![prop.x]
                        }
                        Prop::Bunting => vec![prop.x - prop.w / 2.0, prop.x + prop.w / 2.0],
                        _ => continue,
                    };
                    for x in feet {
                        assert!(
                            ground.open(x, prop.y, fig * 0.5, usize::MAX),
                            "{name} {kind:?} at {t}: {prop:?} not on open ground at {x}"
                        );
                        for (tx, half) in &things {
                            // At its side, or a step in front of it.
                            assert!(
                                (x - tx).abs() >= *half
                                    || (prop.y - stage.feet).abs() >= fig * 0.44,
                                "{name} {kind:?} at {t}: {prop:?} on a thing at {tx}"
                            );
                        }
                    }
                }
                t += 0.25;
            }
        }
    }
    assert!(scenes >= 7 * 8, "{scenes} scenes");
}

/// Nobody says anything in a scene but the words the script records, by
/// whoever it records; and nothing before the first of them.
#[test]
fn speech_is_only_what_the_script_records() {
    let (snapshot, stage) = town();
    for kind in KINDS {
        let script = scene(&snapshot, &stage, kind);
        let lines = script.speech();
        let first = script
            .cues
            .iter()
            .find(|cue| matches!(cue.act, Act::Say(_)))
            .map(|cue| cue.at)
            .expect("a line");
        let mut t = 0.0;
        while t <= script.seconds {
            let staged = stage_scene(&script, &stage, &snapshot, t);
            match &staged.bubble {
                Some((who, words, _)) => {
                    assert!(t >= first);
                    assert!(
                        lines
                            .iter()
                            .any(|(by, line)| *by == Some(*who) && line == words),
                        "{kind:?}: {words}"
                    );
                }
                // Not yet said, or its speaker has gone in.
                None => assert!(
                    t < first
                        || staged
                            .people
                            .iter()
                            .all(|person| lines.iter().all(|(by, _)| *by != Some(person.who))),
                    "{kind:?} at {t}"
                ),
            }
            t += 0.5;
        }
    }
    // A line recorded by nobody on the stage is not put in anyone's mouth.
    let (snapshot, stage) = town();
    let mut script = scene(&snapshot, &stage, MomentKind::Quarrel);
    for cue in &mut script.cues {
        if matches!(cue.act, Act::Say(_)) {
            cue.who = None;
        }
    }
    assert!(stage_scene(&script, &stage, &snapshot, 3.0)
        .bubble
        .is_none());
}

/// A cast member with no cues of their own is not staged; nor is anyone
/// the script does not cast.
#[test]
fn only_the_cast_with_a_part_is_staged() {
    let (snapshot, stage) = town();
    let mut script = scene(&snapshot, &stage, MomentKind::Quarrel);
    let idle = stage
        .people
        .iter()
        .map(|spot| snapshot.canvas.items[spot.index].id)
        .find(|id| script.cast.iter().all(|(cast, _)| cast != id))
        .expect("someone else out");
    script.cast.push((idle, None));
    let staged = stage_scene(&script, &stage, &snapshot, 2.0);
    assert!(staged.person(who(&snapshot, idle)).is_none());
    assert_eq!(staged.people.len(), script.cast.len() - 1);
}

/// While a scene plays, its cast stand and walk where it has them and
/// everyone else keeps their day, exactly as without it.
#[test]
fn living_takes_the_scene_for_its_cast_and_the_day_for_everyone_else() {
    let (snapshot, stage) = town();
    let script = scene(&snapshot, &stage, MomentKind::Festival);
    let staged = stage_scene(&script, &stage, &snapshot, 2.0);
    let cast = cast_on(&stage, &staged);
    let pinned = BTreeSet::new();
    let day = living(&stage, &snapshot, 2.0, Daylight::Day, &pinned, None);
    let lives = living_staged(
        &cast,
        &snapshot,
        2.0,
        Daylight::Day,
        &pinned,
        None,
        Some(&staged),
    );
    assert_eq!(lives.len(), cast.people.len());
    for (spot, life) in cast.people.iter().zip(&lives) {
        match staged.person(spot.index) {
            Some(person) => {
                assert!((life.x - person.x).abs() < 0.5, "{person:?} {life:?}");
                assert_eq!(spot.y, person.y.max(stage.feet - stage.figure_h * 0.4));
                assert_eq!(life.pose.stride.is_some(), person.walk.is_some());
            }
            None => {
                // Where their day has them, held there while the scene
                // plays (no errand, no stroll), so the scene is staged
                // clear of them; what they do there is their day's.
                let at = stage
                    .people
                    .iter()
                    .position(|other| other.index == spot.index)
                    .expect("out before");
                let before = &day[at];
                assert!(
                    (life.x - stage.people[at].x).abs() < 4.0,
                    "{life:?} {:?}",
                    stage.people[at]
                );
                assert!(life.pose.stride.is_none(), "{life:?}");
                if before.pose.stride.is_none() {
                    assert_eq!(life.using, before.using);
                }
            }
        }
    }
    // Nobody of the cast is left indoors.
    for person in &staged.people {
        assert!(cast.inside.iter().all(|(who, _)| *who != person.index));
    }
}

/// A frame dressed for a scene shows its props and rain, and none of it
/// changes what the place's looks are baked from.
#[test]
fn a_dressed_frame_is_baked_as_before() {
    let (snapshot, stage) = town();
    let script = scene(&snapshot, &stage, MomentKind::Storm);
    let staged = stage_scene(&script, &stage, &snapshot, 4.0);
    let living = living(
        &stage,
        &snapshot,
        0.0,
        Daylight::Day,
        &BTreeSet::new(),
        None,
    );
    let frame = super::frame(
        &snapshot,
        &stage,
        &living,
        Camera::whole(&stage),
        0.0,
        Daylight::Day,
        &Glows::new(),
        1.0,
    );
    let dressed = frame.clone().dressed(staged.dress());
    assert_eq!(
        super::stage_view::bake_key(&frame),
        super::stage_view::bake_key(&dressed)
    );
    // A storm, once it has set in.
    assert_eq!(dressed.shown_weather(), Weather::Storm);
    assert!(!dressed.scene_props().is_empty());
}

/// A script's cues with no camera of their own still give a view, and a
/// scene with no one to stage stages no one.
#[test]
fn a_bare_script_stages_nothing_and_looks_at_its_place() {
    let (snapshot, stage) = town();
    let place = stage.buildings[0].index;
    let script = SceneScript {
        place: Some(snapshot.canvas.items[place].id),
        seconds: 5.0,
        cues: vec![Cue {
            at: 0.0,
            who: None,
            act: Act::Camera {
                on: Mark::Offstage(true),
                zoom: 1.0,
            },
        }],
        ..SceneScript::default()
    };
    let staged = stage_scene(&script, &stage, &snapshot, 1.0);
    assert!(staged.people.is_empty() && staged.props.is_empty());
    assert!(staged.camera.is_some());
}

/// The town with `id` gone from it (left, or died), laid out as the
/// window lays it out now.
fn without(snapshot: &ProjectionSnapshot, id: SelectionId) -> (ProjectionSnapshot, Stage) {
    let mut gone = snapshot.clone();
    gone.canvas.items.retain(|item| item.id != id);
    let stage = stage_at(&gone, VIEW.0, VIEW.1, Clock::at(11));
    (gone, stage)
}

/// A scene drawn as the window draws it: its cast put on the stage, and
/// everyone drawn from the World as the scene draws it.
fn drawn(script: &SceneScript, stage: &Stage, snapshot: &ProjectionSnapshot, t: f32) -> Frame {
    let staged = stage_scene(script, stage, snapshot, t);
    let world = scene_snapshot(script, snapshot);
    let cast = cast_on(stage, &staged);
    let lives = living_staged(
        &cast,
        &world,
        t,
        Daylight::Day,
        &BTreeSet::new(),
        None,
        Some(&staged),
    );
    super::frame(
        &world,
        &cast,
        &lives,
        Camera::whole(&cast),
        0.0,
        Daylight::Day,
        &Glows::new(),
        1.0,
    )
}

/// A farewell played back after its leaver has gone still has them: put
/// back on the canvas as the script has them look, staged with their bag,
/// off to the ferry and aboard it, and drawn (v0.30 S3c review: the one
/// leaving was dropped, the farewell played without them). One of the
/// cast with no part of their own (gone too) is not brought back.
#[test]
fn a_farewell_whose_leaver_has_gone_still_stages_and_draws_them() {
    let (snapshot, stage) = town();
    let mut script = scene(&snapshot, &stage, MomentKind::Farewell);
    let leaver = script.cast[0].0;
    let look = world_projection::Look {
        clothes: Some(0xc0302a),
        ..Default::default()
    };
    script.cast[0].1 = Some(look);
    let (gone, stage) = without(&snapshot, leaver);
    // Someone else long gone, cast but with nothing to do.
    let dead = SelectionId::from_stable_key("entity-999999").expect("an id");
    script.cast.push((dead, Some(look)));
    let world = scene_snapshot(&script, &gone);
    assert_eq!(world.canvas.items.len(), gone.canvas.items.len() + 1);
    let index = gone.canvas.items.len();
    assert_eq!(world.canvas.items[index].id, leaver);
    assert_eq!(world.canvas.items[index].look, Some(look));
    // Made once: the canvas it made needs nothing more.
    assert!(matches!(
        scene_snapshot(&script, &world),
        std::borrow::Cow::Borrowed(_)
    ));
    // Staged: bag in hand at the start, walking off, aboard at the end;
    // the same whether the window hands it the World or the scene's.
    let start = stage_scene(&script, &stage, &gone, 0.0);
    assert_eq!(start, stage_scene(&script, &stage, &world, 0.0));
    let at_start = person(&start, index);
    assert_eq!(at_start.who, leaver);
    assert!(start
        .props
        .iter()
        .any(|prop| prop.prop == Prop::Suitcase && prop.held_by == Some(index)));
    let going = stage_scene(&script, &stage, &gone, 5.0);
    assert!(person(&going, index).walk.is_some());
    let end = stage_scene(&script, &stage, &gone, script.seconds - 0.3);
    assert!(person(&end, index).aboard, "{end:?}");
    assert!(end.people.iter().all(|person| person.who != dead));
    // Drawn, as the script has them look.
    let frame = drawn(&script, &stage, &gone, 2.0);
    let figure = frame
        .people
        .iter()
        .find(|person| person.index == index)
        .map(|person| person.figure)
        .expect("the leaver drawn");
    assert_eq!(figure, Figure::of(&leaver.stable_key(), Some(look)));
    assert!(frame.people.len() <= world.canvas.items.len());
}

/// A birth played back years on draws its baby as the baby they were, as
/// the script has them look, not as they look today; and everyone else of
/// the cast as they looked then too.
#[test]
fn a_birth_played_later_draws_the_baby_as_a_baby() {
    let (snapshot, stage) = town();
    let mut script = scene(&snapshot, &stage, MomentKind::Birth);
    let parent = script.cast[1].0;
    // Someone out now, grown, who was the baby.
    let baby = stage
        .people
        .iter()
        .map(|spot| &snapshot.canvas.items[spot.index])
        .find(|item| {
            script.cast.iter().all(|(who, _)| *who != item.id)
                && item
                    .look
                    .is_none_or(|look| look.age != Some(AgeStage::Baby))
        })
        .map(|item| item.id)
        .expect("someone grown");
    let then = Look {
        age: Some(AgeStage::Baby),
        ..Default::default()
    };
    script.cast.push((baby, Some(then)));
    script.cues.push(Cue {
        at: 0.0,
        who: Some(baby),
        act: Act::WalkTo {
            to: Mark::Beside(parent, 0),
            stance: Stance::Standing,
        },
    });
    let index = who(&snapshot, baby);
    let world = scene_snapshot(&script, &snapshot);
    assert_eq!(world.canvas.items.len(), snapshot.canvas.items.len());
    assert_eq!(world.canvas.items[index].look, Some(then));
    let figure = |frame: &Frame| {
        frame
            .people
            .iter()
            .find(|person| person.index == index)
            .map(|person| person.figure)
    };
    let frame = drawn(&script, &stage, &snapshot, 3.0);
    assert_eq!(figure(&frame).expect("the baby drawn").age, Age::Baby);
    // Today, as they are.
    let today = living(
        &stage,
        &snapshot,
        3.0,
        Daylight::Day,
        &BTreeSet::new(),
        None,
    );
    let now = super::frame(
        &snapshot,
        &stage,
        &today,
        Camera::whole(&stage),
        0.0,
        Daylight::Day,
        &Glows::new(),
        1.0,
    );
    assert_ne!(figure(&now).expect("out today").age, Age::Baby);
}

/// With Reduce Motion on a scene is shown at one moment of it: its last
/// line said by whoever says it, the camera come to rest, nobody of the
/// cast caught mid-walk where that can be helped, every prop out all
/// there; the same moment however often it is asked for.
#[test]
fn a_still_scene_is_held_where_its_payoff_shows() {
    let (snapshot, stage) = town();
    for kind in KINDS {
        let script = scene(&snapshot, &stage, kind);
        let still = stage_still(&script, &stage, &snapshot);
        assert_eq!(still, stage_still(&script, &stage, &snapshot));
        let t = still.t;
        assert!((0.0..=script.seconds).contains(&t), "{kind:?} {t}");
        assert_eq!(still, stage_scene(&script, &stage, &snapshot, t));
        // Said, if it is ever heard as the scene plays (a storm's speaker
        // can be in out of the rain before their line).
        let heard = (0..=(script.seconds * 4.0) as usize).any(|n| {
            stage_scene(&script, &stage, &snapshot, n as f32 / 4.0)
                .bubble
                .is_some()
        });
        assert_eq!(
            still.bubble.is_some(),
            heard,
            "{kind:?} at {t}: the line said"
        );
        // The camera has come to rest on its latest view.
        let settled = script
            .cues
            .iter()
            .filter(|cue| matches!(cue.act, Act::Camera { .. }) && cue.at <= t)
            .all(|cue| t - cue.at >= CAMERA_EASE - 1e-4);
        assert!(settled, "{kind:?} at {t}");
        // Every prop out at that moment is all there.
        assert!(
            still
                .props
                .iter()
                .filter(|prop| !is_transport(prop.prop))
                .all(|prop| prop.shown >= 1.0),
            "{kind:?} at {t}: {:?}",
            still.props
        );
    }
}
