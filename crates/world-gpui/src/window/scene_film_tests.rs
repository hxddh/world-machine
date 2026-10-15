//! The return film as a cut of scenes (v0.30 S3d): a beat that is a
//! recorded Event the World stages plays its scene for as long as the
//! scene lasts, its camera, cast and words the scene's; any other beat
//! plays as before. Clocks are set, never waited on.

use super::*;
use crate::diorama::{self, staging};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};
use world_projection::{
    Act, BriefingItem, BriefingItemKind, BriefingProjection, Cue, Mark, MomentKind, SceneScript,
    Stance, StoryPage, StoryRequest,
};

/// A World that stages the scenes it is given, and counts how often it is
/// asked.
struct Staging {
    snapshot: ProjectionSnapshot,
    scenes: BTreeMap<SelectionId, SceneScript>,
    asked: Rc<Cell<usize>>,
}

impl crate::ProjectionController for Staging {
    fn snapshot(&self) -> ProjectionSnapshot {
        self.snapshot.clone()
    }

    fn handle(&mut self, _: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
        Ok(self.snapshot.clone())
    }

    fn story(&mut self, request: StoryRequest) -> Option<StoryPage> {
        self.asked.set(self.asked.get() + 1);
        match request {
            StoryRequest::Scene(id) => self
                .scenes
                .get(&SelectionId::Event(id))
                .cloned()
                .map(StoryPage::Scene),
            _ => None,
        }
    }
}

/// The recorded Event `id`, as a beat selects it.
fn event(id: u64) -> SelectionId {
    SelectionId::from_stable_key(&format!("event-{id}")).expect("an Event")
}

fn beat(title: &str, selection: Option<SelectionId>) -> BriefingItem {
    BriefingItem {
        selection,
        title: title.into(),
        detail: String::new(),
        kind: BriefingItemKind::Beat,
        tone: world_projection::Tone::Neutral,
    }
}

/// An outing the player arranged: whoever was asked goes from home to
/// the place, and a table is laid there (`chronicle::scene`'s blocking).
fn outing(snapshot: &ProjectionSnapshot, stage: &diorama::Stage) -> SceneScript {
    let festival = staging::sample(snapshot, stage, MomentKind::Festival).expect("a place");
    let (place, a) = (festival.place.unwrap(), festival.cast[0].0);
    let cue = |at: f32, who: Option<SelectionId>, act: Act| Cue { at, who, act };
    SceneScript {
        id: "outing".into(),
        kind: MomentKind::Outing,
        place: Some(place),
        seconds: 8.0,
        cast: vec![(a, None)],
        cues: vec![
            cue(
                0.0,
                None,
                Act::Prop {
                    prop: world_projection::Prop::Table,
                    at: Mark::Beside(place, 1),
                    shown: true,
                },
            ),
            cue(
                0.0,
                None,
                Act::Camera {
                    on: Mark::Place(place),
                    zoom: 0.8,
                },
            ),
            cue(
                0.0,
                Some(a),
                Act::WalkTo {
                    to: Mark::Home(a),
                    stance: Stance::Standing,
                },
            ),
            cue(
                0.5,
                Some(a),
                Act::WalkTo {
                    to: Mark::Place(place),
                    stance: Stance::Walking,
                },
            ),
            cue(4.0, Some(a), Act::Strike(Stance::Celebrating)),
            cue(5.0, Some(a), Act::Say("What a day for it.".into())),
        ],
        caption: "Out for the afternoon".into(),
        ..SceneScript::default()
    }
}

/// The three-year town back from a while away: a festival, an outing
/// the player arranged, an Event the World stages nothing for, and a note
/// that is no Event at all.
/// The three-year town, as its fixture keeps it.
fn town() -> ProjectionSnapshot {
    diorama::tests::real_1082()
}

fn film() -> (ProjectionSnapshot, BTreeMap<SelectionId, SceneScript>) {
    let mut snapshot = town();
    let stage = diorama::stage_at(&snapshot, 1100.0, 800.0, diorama::Clock::at(11));
    let mut festival = staging::sample(&snapshot, &stage, MomentKind::Festival).unwrap();
    // One view all through, so no frame here bakes another.
    for cue in &mut festival.cues {
        if let Act::Camera { zoom, .. } = &mut cue.act {
            *zoom = 0.8;
        }
    }
    let scenes = BTreeMap::from([
        (event(9001), festival),
        (event(9002), outing(&snapshot, &stage)),
    ]);
    snapshot.briefing = Some(BriefingProjection {
        items: vec![
            beat("The town danced", Some(event(9001))),
            beat("Out for the afternoon", Some(event(9002))),
            beat("The tide came in", Some(event(9003))),
            beat("A note", None),
        ],
        eyebrow: String::new(),
        title: String::new(),
        returned: true,
    });
    (snapshot, scenes)
}

fn view_of(
    snapshot: ProjectionSnapshot,
    scenes: BTreeMap<SelectionId, SceneScript>,
) -> (ProjectionView, Rc<Cell<usize>>) {
    let asked = Rc::new(Cell::new(0));
    let mut view = ProjectionView::controlled(Staging {
        snapshot,
        scenes,
        asked: asked.clone(),
    });
    view.looking.opening = None;
    view.retelling = Some(0);
    (view, asked)
}

/// Every beat that is a recorded Event the World stages plays its scene
/// and lasts as long as it does; an arranged outing always plays; a beat
/// with no scene lasts a beat's while, as before. Each beat's scene is
/// asked for once, however many frames it is told over.
#[test]
fn every_return_beat_with_a_scene_plays_it_for_its_length() {
    let (snapshot, scenes) = film();
    let (mut view, asked) = view_of(snapshot, scenes.clone());
    let expect = [
        Some(&scenes[&event(9001)]),
        Some(&scenes[&event(9002)]),
        None,
        None,
    ];
    for (index, scene) in expect.into_iter().enumerate() {
        view.retelling = Some(index);
        let before = asked.get();
        for _ in 0..5 {
            let seconds = view.beat_seconds();
            match scene {
                Some(scene) => {
                    assert_eq!(seconds, scene.seconds, "beat {index}");
                    assert_eq!(view.played_scene().as_deref(), Some(scene));
                }
                None => {
                    assert_eq!(seconds, world_window::BEAT_SECONDS, "beat {index}");
                    assert!(view.played_scene().is_none());
                }
            }
        }
        // Asked once, and only for a beat that is an Event.
        let event = index < 3;
        assert_eq!(asked.get() - before, usize::from(event), "beat {index}");
    }
    // The outing is staged as the World blocked it: the one asked out
    // walks to the place and celebrates there.
    let outing = &scenes[&event(9002)];
    assert_eq!(outing.kind, MomentKind::Outing);
}

/// In the window: a beat with a scene is held past a beat's while, for
/// the scene's length, and then the film goes on; over it, the scene's
/// cast stands where the scene has them and says only the scene's words.
#[gpui::test]
fn a_scene_holds_its_beat_and_then_the_film_goes_on(cx: &mut gpui::TestAppContext) {
    use gpui::VisualTestContext;
    let (snapshot, scenes) = film();
    let festival = scenes[&event(9001)].clone();
    let window = cx.add_window(move |_, _| view_of(snapshot, scenes).0);
    let view = window.root(cx).expect("the World");
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(px(1100.0), px(800.0)));
    cx.run_until_parked();
    // The first frames bake the place right here: the film is put back on
    // its first beat until a frame of it is drawn with the beat still there.
    for _ in 0..5 {
        view.update(cx, |view, cx| {
            view.retelling = Some(0);
            view.looking.beat_at = Some(Instant::now());
            cx.notify();
        });
        cx.run_until_parked();
        if view.read_with(cx, |view, _| view.retelling) == Some(0) {
            break;
        }
    }
    // Past a beat's while, but well inside the scene: still on it.
    view.update(cx, |view, cx| {
        view.looking.beat_at =
            Some(Instant::now() - Duration::from_secs_f32(world_window::BEAT_SECONDS + 1.0));
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.retelling, Some(0), "the scene is still playing");
    });
    // Past the scene's own length: the next beat.
    view.update(cx, |view, cx| {
        view.looking.beat_at =
            Some(Instant::now() - Duration::from_secs_f32(festival.seconds + 0.2));
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.retelling, Some(1), "the next beat");
    });
}

/// What the window says over a scene: only its recorded words, by
/// whoever said them.
#[gpui::test]
fn the_window_draws_the_scene_its_cast_and_its_words(cx: &mut gpui::TestAppContext) {
    use gpui::VisualTestContext;
    let (snapshot, scenes) = film();
    let festival = scenes[&event(9001)].clone();
    let speaker = festival.speech()[0].0.unwrap();
    let words = festival.speech()[0].1.to_string();
    let window = cx.add_window(move |_, _| {
        let (mut view, _) = view_of(snapshot, scenes);
        // Dancing, and the line just said.
        view.looking.beat_clock = Some(5.0);
        view
    });
    let view = window.root(cx).expect("the World");
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(px(1100.0), px(800.0)));
    cx.run_until_parked();
    view.update(cx, |view, cx| {
        view.retelling = Some(0);
        cx.notify();
    });
    cx.run_until_parked();
    // The words heard are the scene's, by whoever said them.
    let said = view.read_with(cx, |view, _| view.looking.babbled.clone());
    assert_eq!(said, Some((speaker, words)));
}

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

/// The card of a return beat keeps clear of its scene: at every camera
/// cue (once the camera is there) and at its end, whoever the camera is
/// on (or else whom the scene is about) and every prop it has out stand
/// clear of the card, docked where it docks for the whole beat (v0.30
/// S3c round 3: a farewell's ferry and its leaver under the card).
#[test]
fn the_card_keeps_clear_of_each_scene_at_every_camera_cue() {
    // The stage as the window lays it out, under its gauges.
    let size = (1100.0, 848.0 - world_window::CHROME);
    let snapshot = town();
    for hour in [11, 14] {
        let stage = diorama::stage_at(&snapshot, size.0, size.1, diorama::Clock::at(hour));
        for kind in KINDS {
            let script = staging::sample(&snapshot, &stage, kind).expect("a scene");
            let dock = world_window::scene_dock(&script, &stage, &snapshot, size);
            let card = world_window::film_card_rect(dock, size);
            let cameras = script
                .cues
                .iter()
                .filter_map(|cue| match cue.act {
                    Act::Camera { on, .. } => Some((cue.at, on)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let moments = cameras
                .iter()
                .enumerate()
                .map(|(n, (at, on))| {
                    let next = cameras
                        .get(n + 1)
                        .map_or(script.seconds, |(next, _)| next - 0.01);
                    ((at + staging::CAMERA_EASE).min(next), *on)
                })
                .chain([(script.seconds - 0.3, cameras.last().expect("a camera").1)]);
            for (t, on) in moments {
                let staged = staging::stage_scene(&script, &stage, &snapshot, t);
                let subject = match on {
                    Mark::Place(id) if script.cast.iter().any(|(who, _)| *who == id) => id,
                    _ => script.cast[0].0,
                };
                let under = world_window::scene_under_card(&staged, &stage, card, size.0);
                let named = format!("person {subject:?}");
                assert!(
                    !under.contains(&named),
                    "{kind:?} at {hour}h, {t}: {named} under the card {dock:?}"
                );
                let props = under
                    .iter()
                    .filter(|what| what.starts_with("prop"))
                    .collect::<Vec<_>>();
                assert!(
                    props.is_empty(),
                    "{kind:?} at {hour}h, {t}: {props:?} under the card {dock:?}"
                );
            }
        }
    }
}

/// A return beat's card stands in one place for the whole of the beat,
/// however far into it the window is first drawn: its place is the
/// scene's, not the moment's (v0.30 S3c round 3: across a storm it went
/// left, centre, right and back).
#[gpui::test]
fn a_beat_card_stands_still_through_its_scene(cx: &mut gpui::TestAppContext) {
    use gpui::VisualTestContext;
    let snapshot = town();
    let stage = diorama::stage_at(
        &snapshot,
        1100.0,
        848.0 - world_window::CHROME,
        diorama::Clock::at(11),
    );
    let storm = staging::sample(&snapshot, &stage, MomentKind::Storm).expect("a storm");
    let mut briefed = snapshot.clone();
    briefed.briefing = Some(BriefingProjection {
        items: vec![beat("The storm", Some(event(9001)))],
        eyebrow: String::new(),
        title: String::new(),
        returned: true,
    });
    let scenes = BTreeMap::from([(event(9001), storm.clone())]);
    let window = cx.add_window(move |_, _| view_of(briefed, scenes).0);
    let view = window.root(cx).expect("the World");
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(px(1100.0), px(848.0)));
    cx.run_until_parked();
    let mut rects = Vec::new();
    for n in 0..=6 {
        let t = storm.seconds * n as f32 / 6.0;
        view.update(cx, |view, cx| {
            view.retelling = Some(0);
            view.looking.beat_clock = Some(t);
            // As if the window were first drawn this far in.
            view.looking.film_dock = None;
            cx.notify();
        });
        cx.run_until_parked();
        let card = cx.debug_bounds("bottom-card").expect("the card");
        rects.push((
            f32::from(card.origin.x).round(),
            f32::from(card.size.width).round(),
        ));
        // No taller than the room the film keeps for it.
        assert!(
            f32::from(card.origin.y) >= 848.0 - 16.0 - world_window::FILM_CARD_ROOM - 1.0,
            "{card:?}"
        );
    }
    assert!(rects.windows(2).all(|pair| pair[0] == pair[1]), "{rects:?}");
}

/// The World window over the three-year town playing `script`, `t`
/// seconds in, drawn through GPUI and the reference rasteriser (no text).
fn scene_picture(script: &SceneScript, t: f32, size: (f32, f32)) -> image::RgbaImage {
    use crate::offscreen::Raster;
    use gpui::{App, HeadlessAppContext, NoopTextSystem};
    let mut snapshot = town();
    snapshot.briefing = Some(BriefingProjection {
        items: vec![beat(script.kind.id(), Some(event(9001)))],
        eyebrow: String::new(),
        title: String::new(),
        returned: true,
    });
    let scenes = BTreeMap::from([(event(9001), script.clone())]);
    let mut cx = HeadlessAppContext::with_platform(
        std::sync::Arc::new(NoopTextSystem::new()),
        std::sync::Arc::new(()),
        || Some(Box::new(Raster(Default::default())) as Box<dyn gpui::PlatformHeadlessRenderer>),
    );
    let window = cx
        .open_window(
            gpui::size(px(size.0), px(size.1)),
            move |_, cx: &mut App| {
                cx.new(|_| {
                    let (mut view, _) = view_of(snapshot.clone(), scenes.clone());
                    view.looking.beat_clock = Some(t);
                    view
                })
            },
        )
        .expect("a window");
    cx.run_until_parked();
    // Again, once its looks are baked and its poses posed.
    cx.update_window(window.into(), |_, window, _| window.refresh())
        .expect("the window");
    cx.run_until_parked();
    cx.capture_screenshot(window.into()).expect("a picture")
}

/// Pictures for the art director: the start, middle and end of each kind
/// of scene a return film plays most, on the three-year town in the
/// World window at its own zoom, and a strip of six moments of one whole
/// festival beat. Written to `WORLD_GPUI_SCENES`.
#[test]
#[ignore = "pictures: WORLD_GPUI_SCENES=dir cargo test -p world-gpui -- --ignored scene_pictures"]
fn scene_pictures() {
    let Some(dir) = std::env::var_os("WORLD_GPUI_SCENES") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    crate::scene::pin_hour(Some(14));
    let size = (1100.0, 848.0);
    let snapshot = town();
    // Staged on the stage the window lays out, under its gauges.
    let stage = diorama::stage_at(
        &snapshot,
        size.0,
        size.1 - world_window::CHROME,
        diorama::Clock::at(14),
    );
    let kinds = [
        MomentKind::Arrival,
        MomentKind::Quarrel,
        MomentKind::Festival,
        MomentKind::Build,
        MomentKind::Birth,
        MomentKind::Farewell,
        MomentKind::Storm,
    ];
    let only = std::env::var("WORLD_GPUI_SCENE_KIND").ok();
    for kind in kinds {
        if only.as_deref().is_some_and(|only| only != kind.id()) {
            continue;
        }
        let script = staging::sample(&snapshot, &stage, kind).expect("a scene");
        for (name, t) in [
            ("start", 0.3),
            ("middle", script.seconds / 2.0),
            ("end", script.seconds - 0.3),
        ] {
            scene_picture(&script, t, size)
                .save(dir.join(format!("{}-{name}.png", kind.id())))
                .expect("a picture");
        }
    }
    // Whole beats, six moments of each.
    for kind in [MomentKind::Festival, MomentKind::Storm] {
        if only.as_deref().is_some_and(|only| only != kind.id()) {
            continue;
        }
        let script = staging::sample(&snapshot, &stage, kind).unwrap();
        let frames = (0..6)
            .map(|n| scene_picture(&script, script.seconds * n as f32 / 5.0, size))
            .collect::<Vec<_>>();
        let (w, h) = (frames[0].width() / 2, frames[0].height() / 2);
        let mut strip = image::RgbaImage::from_pixel(w * 3 + 12, h * 2 + 6, image::Rgba([255; 4]));
        for (n, frame) in frames.iter().enumerate() {
            let small = image::imageops::resize(frame, w, h, image::imageops::FilterType::Triangle);
            let (x, y) = ((n as u32 % 3) * (w + 6), (n as u32 / 3) * (h + 6));
            image::imageops::overlay(&mut strip, &small, i64::from(x), i64::from(y));
        }
        strip
            .save(dir.join(format!("{}-beat-strip.png", kind.id())))
            .expect("a picture");
    }
    crate::scene::pin_hour(None);
}

/// With Reduce Motion on, a beat's scene is held at one moment, its
/// payoff in view (its line said), however far into the beat the wall
/// clock is, rather than played in jumps a few frames a second; and the
/// beat still lasts as long as its scene.
#[gpui::test]
fn with_reduce_motion_a_scene_holds_still_and_its_beat_still_lasts(cx: &mut gpui::TestAppContext) {
    use gpui::VisualTestContext;
    let (snapshot, scenes) = film();
    let festival = scenes[&event(9001)].clone();
    let (speaker, words) = festival.speech()[0];
    let (speaker, words) = (speaker.unwrap(), words.to_string());
    // Well before the line is said.
    let said = festival
        .cues
        .iter()
        .find(|cue| matches!(cue.act, Act::Say(_)))
        .expect("a line")
        .at;
    assert!(said > 1.0);
    cx.update(|cx| cx.set_reduce_motion(true));
    let window = cx.add_window(move |_, _| {
        let (mut view, _) = view_of(snapshot, scenes);
        view.looking.beat_clock = Some(0.5);
        view
    });
    let view = window.root(cx).expect("the World");
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(px(1100.0), px(800.0)));
    cx.run_until_parked();
    view.update(cx, |view, cx| {
        view.retelling = Some(0);
        cx.notify();
    });
    cx.run_until_parked();
    // Held where its line is said, half a second into the beat.
    let heard = view.read_with(cx, |view, _| view.looking.babbled.clone());
    assert_eq!(heard, Some((speaker, words)));
    view.read_with(cx, |view, _| {
        assert_eq!(view.retelling, Some(0), "still on the beat");
    });
    // Past the scene's own length: the film goes on.
    view.update(cx, |view, cx| {
        view.looking.beat_clock = Some(festival.seconds + 0.2);
        cx.notify();
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_ne!(view.retelling, Some(0), "the next beat");
    });
    cx.update(|_, cx| cx.set_reduce_motion(false));
}

/// A World that stages a beat's scene in whatever language the app is
/// shown in.
struct Speaking {
    snapshot: ProjectionSnapshot,
    scene: SceneScript,
}

impl crate::ProjectionController for Speaking {
    fn snapshot(&self) -> ProjectionSnapshot {
        self.snapshot.clone()
    }

    fn handle(&mut self, _: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
        Ok(self.snapshot.clone())
    }

    fn story(&mut self, request: StoryRequest) -> Option<StoryPage> {
        let StoryRequest::Scene(_) = request else {
            return None;
        };
        let words = match world_i18n::language() {
            world_i18n::Language::SimplifiedChinese => "真是个好日子。",
            _ => "What a day for it.",
        };
        let mut scene = self.scene.clone();
        for cue in &mut scene.cues {
            if let Act::Say(said) = &mut cue.act {
                *said = words.into();
            }
        }
        Some(StoryPage::Scene(scene))
    }
}

/// A change of language shows a beat's scene anew, its words in the new
/// language, though the World itself has not moved on.
#[test]
fn a_change_of_language_asks_for_the_beat_scene_again() {
    let (snapshot, scenes) = film();
    let scene = scenes[&event(9002)].clone();
    let mut view = ProjectionView::controlled(Speaking { snapshot, scene });
    view.retelling = Some(1);
    let words = |view: &ProjectionView| {
        view.played_scene()
            .map(|script| script.speech()[0].1.to_string())
    };
    world_i18n::set_thread_language(Some(world_i18n::Language::English));
    view.show_in_language_now();
    view.beat_scene();
    assert_eq!(words(&view).as_deref(), Some("What a day for it."));
    world_i18n::set_thread_language(Some(world_i18n::Language::SimplifiedChinese));
    view.show_in_language_now();
    let shown = words(&view);
    world_i18n::set_thread_language(None);
    assert_eq!(shown.as_deref(), Some("真是个好日子。"));
}
