//! A recorded Event's scene on the living stage (v0.30 S3c): where the
//! cast stands and walks at its start, middle and end, the camera on
//! them, nobody in a building or on a prop, and only the recorded words
//! said. Every clock here is pinned: a scene is a function of its script,
//! the stage and `t`.

use super::living_stage as ls;
use super::staging::*;
use super::*;
use world_projection::{Act, Cue, Mark, MomentKind, Prop, SceneScript};

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
    assert!((person(&end, a).x - x).abs() < fig * 2.0, "{end:?}");
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
    // Side by side: the nearest open ground beside them.
    assert!((pa.x - pb.x).abs() < fig * 3.0, "{pa:?} {pb:?}");
    assert_eq!(pa.facing, (pb.x - pa.x).signum(), "facing each other");
    assert_eq!(pb.facing, (pa.x - pb.x).signum(), "facing each other");
    assert_eq!(pa.stance, Stance::Talking);
    // Half way, one has turned away and is walking home.
    let middle = stage_scene(&script, &stage, &snapshot, 4.5);
    assert!(person(&middle, b).walk.is_some(), "{middle:?}");
    let end = stage_scene(&script, &stage, &snapshot, script.seconds);
    assert!(
        (person(&end, b).x - pa.x).abs() > (pb.x - pa.x).abs() + fig,
        "{end:?}"
    );
    assert_eq!(person(&end, a).stance, Stance::LookingAround);
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
    assert!(working.props.iter().any(|prop| prop.prop == Prop::Scaffold));
    assert!(working.props.iter().any(|prop| prop.prop == Prop::Tools));
    assert!(working
        .people
        .iter()
        .all(|person| person.stance == Stance::Working));
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
        assert_eq!(person(&end, index).stance, Stance::Waving);
    }
    // A farewell: bag in hand, off to the right edge, the ferry waiting.
    let farewell = scene(&snapshot, &stage, MomentKind::Farewell);
    let a = who(&snapshot, farewell.cast[0].0);
    let start = stage_scene(&farewell, &stage, &snapshot, 0.0);
    let going = stage_scene(&farewell, &stage, &snapshot, 5.0);
    let end = stage_scene(&farewell, &stage, &snapshot, farewell.seconds);
    assert!(person(&going, a).walk.is_some() && person(&going, a).facing > 0.0);
    assert!(person(&end, a).x > person(&start, a).x + fig * 3.0);
    let ferry = going.props.iter().find(|prop| prop.prop == Prop::Ferry);
    assert!(ferry.is_some_and(|ferry| ferry.x > person(&start, a).x));
    // A storm: rain from the start, everyone off home, a lamp lit.
    let storm = scene(&snapshot, &stage, MomentKind::Storm);
    let start = stage_scene(&storm, &stage, &snapshot, 0.0);
    let end = stage_scene(&storm, &stage, &snapshot, storm.seconds);
    assert!(start.rain && end.rain);
    assert!(end.props.iter().any(|prop| prop.prop == Prop::Lamp));
    let moved = storm
        .cast
        .iter()
        .filter(|(id, _)| {
            let index = who(&snapshot, *id);
            (person(&end, index).x - person(&start, index).x).abs() > fig
        })
        .count();
    assert!(moved >= 1, "someone hurried home");
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
                    let person = person(&staged, who(&snapshot, *id));
                    assert!(shown(person.x), "{kind:?} at {t}: {person:?}");
                }
                _ => {}
            }
        }
    }
}

/// In every fixture, every kind, every quarter second: nobody staged has
/// their feet in a building, and nobody standing is on a prop.
#[test]
fn nobody_staged_is_in_a_building_or_on_a_prop() {
    let mut scenes = 0;
    for (name, snapshot) in fixtures() {
        let stage = stage_at(&snapshot, VIEW.0, VIEW.1, Clock::at(11));
        let footprints = footprints(&stage, &snapshot);
        let fig = stage.figure_h;
        for kind in KINDS {
            let Some(script) = sample(&snapshot, &stage, kind) else {
                continue;
            };
            scenes += 1;
            let mut t = 0.0;
            while t <= script.seconds {
                let staged = stage_scene(&script, &stage, &snapshot, t);
                for person in &staged.people {
                    for footprint in &footprints {
                        assert!(
                            !ls::inside(*footprint, person.x, person.y),
                            "{name} {kind:?} at {t}: {person:?} in {footprint:?}"
                        );
                    }
                    if person.walk.is_some() {
                        continue;
                    }
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
                        let on = (person.x - prop.x).abs() < prop.w / 2.0 + fig * 0.1
                            && (person.y - prop.y).abs() < fig * 0.45;
                        assert!(!on, "{name} {kind:?} at {t}: {person:?} on {prop:?}");
                    }
                }
                t += 0.25;
            }
        }
    }
    assert!(scenes >= 7 * 4, "{scenes} scenes");
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
                None => assert!(t < first || kind == MomentKind::Arrival && t < 4.5),
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
                // Where they are and what they do is their day's; only
                // whom they turn to may change, with the cast about them.
                let at = stage
                    .people
                    .iter()
                    .position(|other| other.index == spot.index)
                    .expect("out before");
                let before = &day[at];
                assert_eq!(
                    (life.x, life.stance, life.using, life.loaded),
                    (before.x, before.stance, before.using, before.loaded)
                );
                assert_eq!(life.pose.stride, before.pose.stride);
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
    assert_eq!(dressed.shown_weather(), Weather::Rain);
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
