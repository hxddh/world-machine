use super::super::drawer::tests::a_full_drawer;
use super::*;
use crate::ui::accessible;

#[test]
fn the_noon_framing_is_framed_again_after_a_turn() {
    let size = (1280.0, 800.0);
    let kept = Some((3, size, 400.0));
    assert_eq!(kept_noon_view(kept, 3, size, 300.0), Some(400.0));
    // A turn built or moved something: the noon stage is framed again.
    assert_eq!(kept_noon_view(kept, 4, size, 300.0), None);
    assert_eq!(kept_noon_view(kept, 3, (1000.0, 800.0), 300.0), None);
    assert_eq!(kept_noon_view(kept, 3, size, 1100.0), None);
    assert_eq!(kept_noon_view(None, 3, size, 300.0), None);
}

#[test]
fn a_world_nobody_can_see_asks_for_no_frames_and_keeps_no_clock() {
    for active in [false, true] {
        for still in [false, true] {
            assert_eq!(pace(false, active, still), Pace::Asleep);
        }
    }
    assert_eq!(pace(true, false, false), Pace::Waiting);
    assert_eq!(pace(true, false, true), Pace::Waiting);
    assert_eq!(pace(true, true, true), Pace::Ticking);
    assert_eq!(pace(true, true, false), Pace::EveryFrame);
}

/// Through GPUI's test window: covered, the World's clock stops;
/// shown again, it is drawn at once and the clock starts again.
#[gpui::test]
fn a_covered_world_stops_its_clock_and_draws_when_shown(cx: &mut gpui::TestAppContext) {
    use gpui::{VisualTestContext, WindowVisibility};
    let window = cx.add_window(|_, _| ProjectionView::new(ProjectionSnapshot::default()));
    let view = window.root(cx).expect("the World");
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    let ticking =
        |cx: &mut VisualTestContext| view.read_with(cx, |view, _| view.looking.clock.is_some());
    assert!(ticking(cx), "a World in view keeps its clock");

    cx.simulate_visibility_change(WindowVisibility::Hidden);
    cx.run_until_parked();
    assert!(!ticking(cx), "covered, the clock stops");
    cx.executor().advance_clock(FRAME * 40);
    cx.run_until_parked();
    assert!(!ticking(cx), "and nothing starts it while covered");

    cx.simulate_visibility_change(WindowVisibility::Visible);
    cx.run_until_parked();
    assert!(ticking(cx), "shown, it is drawn and the clock starts again");
}

fn someone() -> SelectionId {
    SelectionId::from_stable_key("entity-7").expect("an entity key")
}

/// The zoom control takes the camera all the way out into the folded
/// postcard of a three-year World and all the way back in, a press at
/// a time, and no further either way. (Until v0.30 S1b it also held
/// the views the tile painter painted ahead for it to where a press
/// goes; the living stage paints nothing ahead.)
#[test]
fn the_zoom_control_goes_into_the_postcard_and_back() {
    let snapshot = crate::diorama::tests::three_years();
    let stage = diorama::stage_at(&snapshot, 1100.0, 848.0, diorama::Clock::at(12));
    let mut view = ProjectionView::new(snapshot);
    view.retelling = None;
    let middle = (stage.view_w / 2.0, stage.height / 2.0);
    let mut camera = Camera::whole(&stage);
    let mut folded = 0;
    let mut stopped = 0;
    for factor in [1.0 / ZOOM_STEP; 5].into_iter().chain([ZOOM_STEP; 9]) {
        let before = camera;
        let moved = view.zoom_at(&stage, camera, factor, middle);
        if moved {
            camera = view.free_target(&stage);
            assert!(
                (camera.zoom > before.zoom) == (factor > 1.0),
                "{before:?} by {factor}: {camera:?}"
            );
            folded += usize::from(camera.fold > 0.0);
        } else {
            stopped += 1;
        }
    }
    assert!(folded >= 2, "the zoom went into the folded postcard");
    assert!(stopped >= 1, "the zoom stops at the end of its travel");
    assert!(
        (camera.zoom - diorama::ZOOM_MOST).abs() < 1e-4,
        "{camera:?}"
    );
}

/// As a World opens on a question, the card waits while the first
/// thing said (a hello) is heard; a card that asks nothing does not.
#[test]
fn a_question_waits_for_the_first_thing_said_as_a_world_opens() {
    use world_projection::{Question, TimelineItem, Voice};
    let moment = SelectionId::from_stable_key("event-3").expect("an event key");
    let command = |question: Option<Question>| ProjectionCommand {
        id: "ask".into(),
        title: "Timber for the pier".into(),
        detail: String::new(),
        effects: Vec::new(),
        scenery: None,
        asker: Some(someone()),
        moves: Vec::new(),
        question,
        unavailable: None,
        hand: None,
        preview: None,
        role: None,
    };
    let mut snapshot = ProjectionSnapshot {
        world_time: 20,
        voices: vec![Voice {
            moment,
            speaker: someone(),
            line: "Oh, a new face! I'm Leo. Welcome to the harbour.".into(),
        }],
        commands: vec![command(Some(Question {
            id: "pier".into(),
            prompt: "Will you find timber for the pier?".into(),
        }))],
        ..ProjectionSnapshot::default()
    };
    snapshot.timeline.items.push(TimelineItem {
        id: moment,
        world_time: 20,
        title: "Leo came over to say hello".into(),
        subtitle: String::new(),
        caused_by: Vec::new(),
        routine: false,
    });
    assert_eq!(question_waits(&snapshot), LINE_SECONDS);
    snapshot.commands = vec![command(None)];
    assert_eq!(question_waits(&snapshot), 0.0, "only a question waits");
}

/// The answers on a question's card are buttons in a set, named by
/// what they say; one that cannot be chosen now says why.
#[test]
fn answers_and_verbs_are_named_controls() {
    let (role, node) = accessible(&answer_button(1, 3, "Stay home".into(), true, None));
    assert_eq!(role, Some(Role::Button));
    assert_eq!(node.label(), Some("Stay home"));
    assert_eq!(node.position_in_set(), Some(2));
    assert_eq!(node.size_of_set(), Some(3));
    let (role, node) = accessible(&answer_button(
        0,
        3,
        "Lend the boat".into(),
        false,
        Some("The boat is out"),
    ));
    assert_eq!(role, Some(Role::Button));
    assert_eq!(node.description(), Some("Not now: The boat is out"));
    let (role, node) = accessible(&verb_tab("Plant", true));
    assert_eq!(role, Some(Role::Tab));
    assert_eq!(node.label(), Some("Plant"));
    assert_eq!(node.is_selected(), Some(true));
    let (_, other) = accessible(&verb_tab("Give", false));
    assert_eq!(other.is_selected(), Some(false));
}

/// A place the tests can find a control by, as `ui::named` marks it.
fn named(name: &str) -> &'static str {
    Box::leak(format!("named: {name}").into_boxed_str())
}

fn tip(name: &str) -> &'static str {
    Box::leak(format!("tip: {name}").into_boxed_str())
}

/// Through GPUI's test window: every icon on the World (the hands,
/// the strip, the drawer, the zoom control, a card's arrows) shows its
/// name when the pointer rests on it, after a short delay and not at
/// once, and a screen reader hears the same name.
#[gpui::test]
fn icons_show_their_names_when_the_pointer_rests_on_them(cx: &mut gpui::TestAppContext) {
    use gpui::{Modifiers, VisualTestContext};
    let mut snapshot = a_full_drawer();
    let choice = |id: &str| ProjectionCommand {
        hand: None,
        ..deed(id, "Mend")
    };
    snapshot.commands = vec![
        deed("build", "Build"),
        deed("give", "Give"),
        choice("pier"),
        choice("nets"),
    ];
    let window = cx.add_window(move |_, _| {
        let mut view = ProjectionView::controlled(Still(snapshot.clone())).with_strip(|_, _| {});
        view.looking.opening = None;
        view
    });
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(px(1100.0), px(800.0)));
    cx.run_until_parked();
    for name in [
        "Make something (H)",
        "Show as a strip along the edge of the screen (⌥⌘S)",
        "The drawer: story, letters, keepsakes and the book (⌘I)",
        "Zoom in (+)",
        "Zoom out (−)",
        "Previous card (↑)",
        "Next card (↓)",
    ] {
        // As this computer's keyboard has its shortcuts.
        let name = &crate::ui::keys_here(name);
        let bounds = cx
            .debug_bounds(named(name))
            .unwrap_or_else(|| panic!("{name} is drawn"));
        cx.simulate_mouse_move(bounds.center(), None, Modifiers::none());
        cx.run_until_parked();
        assert!(cx.debug_bounds(tip(name)).is_none(), "{name}: not at once");
        cx.executor()
            .advance_clock(ui::TIP_DELAY + Duration::from_millis(50));
        cx.run_until_parked();
        assert!(
            cx.debug_bounds(tip(name)).is_some(),
            "{name}: shown on hover"
        );
        // Away from it, it goes.
        cx.simulate_mouse_move(gpui::point(px(540.0), px(400.0)), None, Modifiers::none());
        cx.run_until_parked();
    }
}

/// After dusk the paper over the scene dims, so no card glares over a
/// night sky; by day it is the usual white, and the dark appearance
/// keeps its own.
#[test]
fn cards_over_a_night_scene_are_a_dimmer_paper() {
    let bright = |colour: gpui::Rgba| colour.r + colour.g + colour.b;
    let day = paper_at(scene::Daylight::Day, false);
    let dusk = paper_at(scene::Daylight::Dusk, false);
    let night = paper_at(scene::Daylight::Night, false);
    assert!(bright(night) < bright(dusk) && bright(dusk) < bright(day));
    assert!(bright(night) > 2.2, "still paper, dark text still reads");
    assert_eq!(
        paper_at(scene::Daylight::Night, true),
        paper_at(scene::Daylight::Day, true)
    );
}

/// A thing of the hands is drawn as what it is, by the name its deed
/// gives it, and as its verb makes things when nothing matches.
#[test]
fn a_thing_of_the_hands_is_drawn_as_itself() {
    assert_eq!(
        hand_shape("tiny-society.hand.build.bench.12", "Build"),
        MarkShape::Bench
    );
    assert_eq!(
        hand_shape("tiny-society.hand.build.rowboat.3", "Build"),
        MarkShape::Boat
    );
    assert_eq!(
        hand_shape("tiny-society.hand.plant.herbs.3", "Plant"),
        MarkShape::Garden
    );
    assert_eq!(
        hand_shape("tiny-society.hand.plant.apple_tree.3", "Plant"),
        MarkShape::Tree
    );
    assert_eq!(
        hand_shape("hands.decorate.bunting.1", "Decorate"),
        MarkShape::Bunting
    );
}

/// A gauge says its reading, a number where the World counts one.
#[test]
fn a_gauge_shows_its_number() {
    let gauge = world_projection::Gauge {
        id: "money".into(),
        label: "Money in town".into(),
        value: 0.4,
        reading: "11,485".into(),
        tone: world_projection::Tone::Good,
    };
    let (role, node) = accessible(&hud_gauge(&gauge, 0.4, None));
    assert_eq!(role, Some(Role::Meter));
    assert_eq!(node.label(), Some("Money in town: 11,485"));
}

/// Every name an icon shows on hover is in each of the app's catalogs.
#[test]
fn every_icon_name_is_translated() {
    for catalog in crate::i18n::app_catalogs() {
        for name in [
            "Make something (H)",
            "Show as a strip along the edge of the screen (⌥⌘S)",
            "The drawer: story, letters, keepsakes and the book (⌘I)",
            "Zoom",
            "Zoom in (+)",
            "Zoom out (−)",
            "Previous card (↑)",
            "Next card (↓)",
            "Close the drawer",
            "Close",
            "Close the strip",
            "Put the letter away",
            "Dismiss this hint",
            "Turn the card over: what this would change (Space)",
            "Turn the card back (Space)",
        ] {
            assert!(catalog.exact(name).is_some(), "no translation for {name:?}");
        }
    }
}

/// A World whose snapshot never changes, for the window tests.
/// The place on its 1,082nd day, three years built.
fn day_1082() -> ProjectionSnapshot {
    crate::diorama::tests::real_1082()
}

struct Still(ProjectionSnapshot);

impl crate::ProjectionController for Still {
    fn snapshot(&self) -> ProjectionSnapshot {
        self.0.clone()
    }

    fn handle(&mut self, _: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
        Ok(self.0.clone())
    }
}

/// A deed for the hands: something to `verb`, here and now.
fn deed(id: &str, verb: &str) -> ProjectionCommand {
    ProjectionCommand {
        id: id.into(),
        title: format!("{verb} something"),
        detail: String::new(),
        effects: Vec::new(),
        scenery: None,
        asker: None,
        moves: Vec::new(),
        question: None,
        unavailable: None,
        hand: Some(world_projection::Hand {
            verb: verb.into(),
            thing: "Bench".into(),
            cost: None,
            at: Some(someone()),
        }),
        preview: None,
        role: None,
    }
}

/// The icon-like controls are named buttons, and the tooltip itself is
/// a tooltip to a screen reader.
#[test]
fn icon_controls_and_their_tips_have_roles() {
    let (role, node) = accessible(&zoom_button("zoom-in", "+", "Zoom in (+)", false));
    assert_eq!(role, Some(Role::Button));
    assert_eq!(node.label(), Some("Zoom in (+)"));
    let (role, node) = accessible(&ui::named(div().id("x"), "Close the drawer"));
    assert_eq!(role, None, "naming adds no role of its own");
    assert_eq!(node.label(), Some("Close the drawer"));
    let tip = gpui::div()
        .id("tip")
        .role(Role::Tooltip)
        .aria_label("Zoom in (+)");
    assert_eq!(accessible(&tip).0, Some(Role::Tooltip));
}

/// A pointer is a status a screen reader hears as it arrives, saying
/// what it points at, with a named button that puts it away.
#[test]
fn a_pointer_is_a_status_with_a_way_to_dismiss_it() {
    for pointer in Pointer::ORDER {
        for caret in [Caret::Up, Caret::Down, Caret::Left] {
            let (role, node) = accessible(&pointer_hint(pointer, caret, |_, _, _| {}));
            assert_eq!(role, Some(Role::Status));
            assert_eq!(
                node.label(),
                Some(crate::ui::keys_here(pointer.words()).as_str())
            );
        }
    }
}

/// The v0.24 bar: the question card never covers whoever asks it. On
/// the day-1,082 place at 1100 by 900 and 1440 by 900, for everyone
/// out on the quay at noon, looked at from anywhere along the place,
/// the card docked for them as asker covers no part of their figure.
#[test]
fn the_card_never_covers_its_asker() {
    use crate::diorama::{self, Camera, Glows};
    let snapshot = day_1082();
    for (width, window_h) in [(1100.0_f32, 900.0_f32), (1440.0, 900.0)] {
        let height = window_h - 52.0;
        let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(12));
        let lives = diorama::living(
            &stage,
            &snapshot,
            0.0,
            crate::scene::Daylight::Day,
            &Default::default(),
            None,
        );
        let mut askers = 0;
        for pan in 0..=20 {
            let camera = Camera::around(&stage, 1.0, stage.width * pan as f32 / 20.0, height / 2.0);
            let frame = diorama::frame(
                &snapshot,
                &stage,
                &lives,
                camera,
                0.0,
                crate::scene::Daylight::Day,
                &Glows::new(),
                1.0,
            );
            for person in &frame.people {
                if person.x < 0.0 || person.x > width {
                    continue;
                }
                askers += 1;
                let dock = card_dock(width, Some(person.x), &[]);
                let (left, right) = dock.span(width);
                let half = person.height * 0.4;
                assert!(
                    person.x + half <= left || person.x - half >= right,
                    "{width}: someone at {} covered by a card at {left}..{right}",
                    person.x
                );
                assert!(dock.w >= 400.0, "{width}: a card {} wide", dock.w);
            }
        }
        assert!(askers > 50, "{askers} askers seen");
    }
}

/// The v0.29 bar for English bubbles: every line the place says,
/// page by page, laid out for its bubble, leaves no line under three
/// tenths of the bubble's width and no word alone on a line.
#[test]
fn english_bubbles_read_evenly() {
    let mut lines = Vec::new();
    for snapshot in [day_1082(), super::super::drawer::tests::day_358()] {
        lines.extend(snapshot.voices.iter().map(|voice| voice.line.clone()));
        lines.extend(snapshot.talks.iter().map(|talk| talk.answer.clone()));
        lines.extend(
            snapshot
                .timeline
                .items
                .iter()
                .map(|item| item.title.clone()),
        );
        lines.extend(snapshot.letters.iter().map(|letter| letter.note.clone()));
        lines.extend(
            snapshot
                .chapters
                .iter()
                .map(|chapter| chapter.summary.clone()),
        );
    }
    lines.extend([
        "Watched the ferry come and go from the bench.".to_string(),
        "Evan came up the hill with a heavy crate.".to_string(),
        "Sat on the bench till my tea went cold.".to_string(),
    ]);
    let mut pages = 0;
    let mut short = Vec::new();
    for line in &lines {
        for page in speech_pages(line) {
            pages += 1;
            let text = page.lines().collect::<Vec<_>>().join(" ");
            let x = |i: usize| text_width(&text[..i]) as f32 * 7.4;
            let room = BUBBLE_ROOM - BUBBLE_PAD * 2.0;
            let ranges = crate::wrap::balanced(&text, room, x);
            if ranges.len() < 2 {
                continue;
            }
            let widths = ranges
                .iter()
                .map(|range| {
                    let end = range.start + text[range.clone()].trim_end().len();
                    x(end) - x(range.start)
                })
                .collect::<Vec<_>>();
            let widest = widths.iter().copied().fold(0.0, f32::max);
            let lone = ranges
                .iter()
                .any(|range| text[range.clone()].split_whitespace().count() < 2);
            if widths.iter().any(|w| *w < widest * 0.3) || lone {
                short.push(format!(
                    "{:?}",
                    ranges.iter().map(|r| &text[r.clone()]).collect::<Vec<_>>()
                ));
            }
        }
    }
    eprintln!("{pages} pages, {} uneven", short.len());
    assert!(pages > 20, "{pages} pages");
    assert!(short.is_empty(), "uneven bubbles:\n{}", short.join("\n"));
}

/// The drawer and Esc never move the camera: talking to someone moves
/// it in on them, and putting them away (Esc, the drawer) leaves it
/// where it is, never jumping to another part of the place.
#[gpui::test]
fn the_drawer_and_esc_never_move_the_camera(cx: &mut gpui::TestAppContext) {
    use gpui::VisualTestContext;
    let snapshot = day_1082();
    let someone = snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .nth(3)
        .map(|item| item.id)
        .expect("someone");
    let window = cx.add_window(move |_, _| {
        let mut view = ProjectionView::controlled(Still(snapshot.clone())).with_strip(|_, _| {});
        view.looking.opening = None;
        view
    });
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(px(1100.0), px(900.0)));
    cx.run_until_parked();
    let settle = |cx: &mut VisualTestContext| {
        std::thread::sleep(Duration::from_secs_f32(CAMERA_SECONDS + 0.2));
        window
            .update(cx, |_, _, cx| cx.notify())
            .expect("the window");
        cx.run_until_parked();
        window
            .update(cx, |view, _, _| view.looking.camera_now)
            .expect("the window")
            .expect("a camera")
    };
    window
        .update(cx, |view, _, cx| view.ask(someone, cx))
        .expect("the window");
    cx.run_until_parked();
    let talking = settle(cx);
    for (what, step) in [("Esc", 0_u8), ("the drawer", 1), ("the drawer shut", 2)] {
        window
            .update(cx, |view, _, cx| match step {
                0 => view.look_away(cx),
                _ => view.toggle_drawer(cx),
            })
            .expect("the window");
        cx.run_until_parked();
        let now = settle(cx);
        assert!(
            (now.x - talking.x).abs() < 1.0 && (now.zoom - talking.zoom).abs() < 0.01,
            "{what} moved the camera from {talking:?} to {now:?}"
        );
    }
}

/// With Reduce Motion on, talking to someone puts the camera on them at
/// once: it never glides there (P6a review).
#[gpui::test]
fn with_motion_reduced_the_camera_is_where_it_is_sent_at_once(cx: &mut gpui::TestAppContext) {
    use gpui::VisualTestContext;
    let snapshot = day_1082();
    let someone = snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .nth(3)
        .map(|item| item.id)
        .expect("someone");
    cx.update(|cx| cx.set_reduce_motion(true));
    let window = cx.add_window(move |_, _| {
        let mut view = ProjectionView::controlled(Still(snapshot.clone())).with_strip(|_, _| {});
        view.looking.opening = None;
        view
    });
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(px(1100.0), px(900.0)));
    cx.run_until_parked();
    let camera = |cx: &mut VisualTestContext| {
        window
            .update(cx, |_, _, cx| cx.notify())
            .expect("the window");
        cx.run_until_parked();
        window
            .update(cx, |view, _, _| view.looking.camera_now)
            .expect("the window")
            .expect("a camera")
    };
    let before = camera(cx);
    window
        .update(cx, |view, _, cx| view.ask(someone, cx))
        .expect("the window");
    let at_once = camera(cx);
    std::thread::sleep(Duration::from_secs_f32(CAMERA_SECONDS + 0.2));
    let settled = camera(cx);
    assert!(
        (at_once.x - before.x).abs() > 1.0 || (at_once.zoom - before.zoom).abs() > 0.01,
        "the camera moved in on them: {before:?} to {at_once:?}"
    );
    assert!(
        (at_once.x - settled.x).abs() < 0.5 && (at_once.zoom - settled.zoom).abs() < 0.001,
        "there at once, not gliding: {at_once:?} then {settled:?}"
    );
    cx.update(|_, cx| cx.set_reduce_motion(false));
}

/// The v0.29 bars for the talk card and speech: on the day-1,082 place
/// at 1100 by 900 and 1440 by 900, with the drawer shut and open, at
/// both text sizes, for everyone out on the ground looked at from
/// anywhere along it, the card of whoever is talked to (whom Find
/// lands on) is wholly inside the window and covers no part of them;
/// a bubble over anyone's head never covers them; and words from
/// anyone off the stage go to a tab inside it.
#[test]
fn no_card_or_bubble_covers_its_own_speaker_and_every_card_fits() {
    use crate::diorama::{self, Camera, Glows};
    let snapshot = day_1082();
    let mut seen = 0;
    for (width, window_h) in [(1100.0_f32, 900.0_f32), (1440.0, 900.0)] {
        let height = window_h - CHROME;
        let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(12));
        let lives = diorama::living(
            &stage,
            &snapshot,
            0.0,
            crate::scene::Daylight::Day,
            &Default::default(),
            None,
        );
        for pan in 0..=20 {
            for zoom in [1.0, 1.35] {
                let camera =
                    Camera::around(&stage, zoom, stage.width * pan as f32 / 20.0, height / 2.0);
                let frame = diorama::frame(
                    &snapshot,
                    &stage,
                    &lives,
                    camera,
                    0.0,
                    crate::scene::Daylight::Day,
                    &Glows::new(),
                    1.0,
                );
                for person in &frame.people {
                    let half = person.height * 0.4;
                    let head = person.y - person.height * 1.08;
                    let body = Area {
                        x: person.x - half,
                        y: person.y - person.height,
                        w: half * 2.0,
                        h: person.height,
                    };
                    if !on_screen(person.x, head, (width, height)) {
                        // Off the stage: words go to a tab inside it.
                        let (tab, _) =
                            place_tab(person.x, head, (220.0, 70.0), &[], (width, height));
                        assert!(
                            tab.x >= 0.0
                                && tab.x + tab.w <= width
                                && tab.y >= 0.0
                                && tab.y + tab.h <= height,
                            "{width}: a tab at {tab:?}"
                        );
                        continue;
                    }
                    seen += 1;
                    for drawer in [false, true] {
                        for scale in [1.0, 2.0] {
                            for talked in 0..=CONVERSATION_SHOWN {
                                let card = asking_place(
                                    person.x,
                                    head,
                                    (width, height),
                                    drawer,
                                    talked,
                                    scale,
                                );
                                let room = width - if drawer { DRAWER_WIDTH } else { 0.0 };
                                assert!(
                                    card.x >= 0.0
                                        && card.x + card.w <= room
                                        && card.y >= 0.0
                                        && card.y + card.h <= height,
                                    "{width}, drawer {drawer}, x{scale}: card {card:?} \
                                         outside the window"
                                );
                                if person.x <= room {
                                    assert!(
                                        !card.overlaps(&body),
                                        "{width}, drawer {drawer}: card {card:?} \
                                             covers whoever it is for at {body:?}"
                                    );
                                }
                            }
                        }
                    }
                    let placed = place_bubbles(
                        &[(person.x, head, 240.0, 70.0)],
                        &[Area {
                            x: 0.0,
                            y: 0.0,
                            w: width,
                            h: HUD_ROOM,
                        }],
                        (width, height),
                    );
                    for bubble in placed {
                        assert!(
                            !bubble.area.overlaps(&body),
                            "{width}: a bubble {:?} covers its speaker at {body:?}",
                            bubble.area
                        );
                    }
                }
            }
        }
    }
    assert!(seen > 100, "{seen} people seen");
}

/// Where each pointer goes keeps clear of the card and of the speech
/// over anyone standing in the middle of the ground, on a usual window.
#[test]
fn pointers_keep_off_the_card() {
    let (width, height) = (1100.0, 748.0);
    // The card: centred at the bottom, as tall as a question with
    // three answers.
    let card = Area {
        x: width / 2.0 - CARD_WIDTH / 2.0,
        y: height - 20.0 - 280.0,
        w: CARD_WIDTH,
        h: 300.0,
    };
    for pointer in Pointer::ORDER {
        assert!(
            !hint_area(pointer, width, height).overlaps(&card),
            "{pointer:?} would cover the card"
        );
    }
    let apart = Area {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    let touching = Area { x: 5.0, ..apart };
    let beside = Area { x: 10.0, ..apart };
    assert!(apart.overlaps(&touching));
    assert!(!apart.overlaps(&beside));
}

struct Arriving(bool);

impl Render for Arriving {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(ui::spring_in(
            div().debug_selector(|| "arriving".into()).size(px(40.0)),
            "arriving",
            self.0,
        ))
    }
}

/// A card springs up from a little below its place, and with Reduce
/// Motion on (GPUI's setting, and ours) it is in its place from the
/// first frame.
#[gpui::test]
fn a_card_springs_into_place_unless_motion_is_reduced(cx: &mut gpui::TestAppContext) {
    use gpui::VisualTestContext;
    let top = |still: bool, reduce: bool, cx: &mut gpui::TestAppContext| {
        cx.update(|cx| cx.set_reduce_motion(reduce));
        let window = cx.add_window(move |_, _| Arriving(still));
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        f32::from(cx.debug_bounds("arriving").expect("drawn").origin.y)
    };
    let moving = top(false, false, cx);
    assert!(moving > 10.0, "starts below its place: {moving}");
    assert_eq!(top(false, true, cx), 0.0, "GPUI's Reduce Motion");
    assert_eq!(top(true, false, cx), 0.0, "our own");
}

/// A page of speech ends where a sentence ends, never in the middle
/// of one: whole sentences share a page while they fit two lines, and
/// a sentence too long for two has a page of its own, taller. Nothing
/// is lost, in English or in Chinese.
#[test]
fn a_page_of_speech_never_stops_in_the_middle_of_a_sentence() {
    let lines = [
            "Here we are again. Last time: Evan took a week's work on the mainland. A week's carpentry on the mainland. Should I go?",
            "The boats came in late again, and the market will be quiet tomorrow because the whole of the north quay is still under repair. Come by!",
            "Is it 3.5 miles to the lighthouse? \"Nearer four,\" she said. We walked anyway.",
            "今天港口的船都回来了，大家都很高兴。晚上我们在码头一起吃饭吧，你也来吗？好！",
        ];
    for line in lines {
        let wanted = sentences(line);
        assert!(wanted.len() >= 2, "{wanted:?}");
        let pages = speech_pages(line);
        let flat = |text: &str| text.split_whitespace().collect::<String>();
        // Every page is whole sentences: the sentences, in order, make
        // up the pages exactly.
        let mut left = wanted.iter();
        for page in &pages {
            let mut made = String::new();
            while flat(&made) != flat(page) {
                let sentence = left
                    .next()
                    .unwrap_or_else(|| panic!("{page:?} stops mid-sentence in {pages:?}"));
                made.push_str(sentence);
                assert!(
                    flat(page).starts_with(&flat(&made)),
                    "{page:?} stops mid-sentence ({made:?}) in {pages:?}"
                );
            }
            // Two lines, or one long sentence as tall as it needs.
            let rows = page.lines().count();
            assert!(
                rows <= 2 || sentences(page).len() == 1 && rows <= PAGE_MOST,
                "{page:?}"
            );
        }
        assert!(left.next().is_none(), "nothing is lost: {pages:?}");
    }
    assert_eq!(
        sentences("Mara said so. \"Really?\" Yes."),
        vec!["Mara said so.", "\"Really?\"", "Yes."]
    );
}

/// The v0.25 bar for speech: a bubble never covers the interface (the
/// gauges and handles, the zoom control, the card) nor another bubble,
/// and stays on the stage, wherever its speaker stands: beside the
/// zoom control, under the handles, over the card, at either edge.
#[test]
fn speech_bubbles_never_cover_the_interface_or_each_other() {
    let (width, height) = (1100.0, 760.0);
    let interface = [
        Area {
            x: 0.0,
            y: 0.0,
            w: width,
            h: HUD_ROOM,
        },
        Area {
            x: 16.0,
            y: zoom_top(height),
            w: ZOOM_SIZE.0,
            h: ZOOM_SIZE.1,
        },
        Area {
            x: 270.0,
            y: height - CARD_MARGIN - CARD_ROOM,
            w: CARD_WIDTH,
            h: CARD_ROOM + CARD_MARGIN,
        },
    ];
    let long = "The boats came in late again, and the market will be quiet. Come by!";
    let pages = speech_pages(long);
    let (w, h) = bubble_size(&pages[0]);
    let check = |heads: &[(f32, f32)]| {
        let bubbles = heads
            .iter()
            .map(|(x, y)| (*x, *y, w, h))
            .collect::<Vec<_>>();
        let placed = place_bubbles(&bubbles, &interface, (width, height));
        assert_eq!(placed.len(), bubbles.len());
        for (index, bubble) in placed.iter().enumerate() {
            let area = bubble.whole();
            assert!(
                area.x >= 0.0
                    && area.y >= 0.0
                    && area.x + area.w <= width
                    && area.y + area.h <= height,
                "{heads:?}: bubble {index} leaves the stage: {area:?}"
            );
            for mine in bubble.parts() {
                for part in &interface {
                    assert!(
                        !mine.overlaps(part),
                        "{heads:?}: bubble {index} {mine:?} covers {part:?}"
                    );
                }
                for (other, them) in placed.iter().enumerate().take(index) {
                    for theirs in them.parts() {
                        assert!(
                            !mine.overlaps(&theirs),
                            "{heads:?}: bubbles {other} and {index} cover each other"
                        );
                    }
                }
            }
            // Its tail points down at its speaker's head, from over it.
            let (x, y) = heads[index];
            assert_eq!((bubble.head, bubble.foot), (x, y));
            assert!(bubble.area.y + bubble.area.h < y);
        }
    };
    // A speaker anywhere along the stage: by the zoom control, at
    // either edge, just over the card, under the handles.
    for head in [
        (70.0, zoom_top(height) + 90.0),
        (60.0, zoom_top(height) + 20.0),
        (8.0, 220.0),
        (width - 8.0, 400.0),
        (550.0, height - CARD_MARGIN - CARD_ROOM - 4.0),
        (700.0, HUD_ROOM + h + 30.0),
    ] {
        check(&[head]);
    }
    // Someone answering the player, beside the card that asks them,
    // while someone else speaks; three speaking at once, side by side.
    check(&[(500.0, 420.0), (560.0, 430.0)]);
    check(&[(420.0, 420.0), (480.0, 420.0), (540.0, 420.0)]);
    // With room, a bubble is right over its speaker.
    let alone = place_bubbles(&[(550.0, 400.0, w, h)], &interface, (width, height));
    assert_eq!(alone[0].area.x, 550.0 - w / 2.0);
    assert_eq!(alone[0].whole().y + alone[0].whole().h, 400.0);
}

/// Chinese is cut by width, but a closing mark (，。！？」) never
/// starts a line, and an opening one (「) never ends one: the
/// character before goes down with it, and every line still fits.
#[test]
fn chinese_lines_never_start_with_closing_punctuation() {
    // Eighteen characters fill a bubble line; each mark here falls
    // just where the line would otherwise break.
    let lines = [
        "今天港口的船都回来了大家都很高兴我们，晚上在码头一起吃饭吧你也来吗？",
        "今天港口的船都回来了大家都很高兴我们。」他笑着说今天港口的船都回来了。",
        "今天港口的船都回来了大家都很高兴我「们」今天港口的船都回来了大家都很！",
    ];
    for line in lines {
        let pages = speech_pages(line);
        let rows = pages
            .iter()
            .flat_map(|page| page.lines())
            .collect::<Vec<_>>();
        assert!(rows.len() >= 2, "{rows:?}");
        for row in &rows {
            let first = row.chars().next().expect("a row is never empty");
            assert!(!never_starts_a_line(first), "{row:?} starts with {first}");
            assert!(!row.ends_with('「'), "{row:?} ends opening a quote");
            assert!(row.chars().count() <= 18, "{row:?} is too wide");
        }
        assert_eq!(rows.concat(), line, "nothing is lost");
    }
}

/// The Next button, the keys and the beat's own time all go on to the
/// next beat the same way, its clock started again: a click once left
/// the clock running, and the next beat could go after 0.7–2 seconds.
#[gpui::test]
fn every_way_on_in_the_return_film_starts_the_beat_afresh(cx: &mut gpui::TestAppContext) {
    use gpui::{Modifiers, VisualTestContext};
    use world_projection::{BriefingItem, BriefingItemKind, BriefingProjection};
    let mut snapshot = day_1082();
    snapshot.briefing = Some(BriefingProjection {
        items: [
            "The pub found its feet again",
            "Jonas's catch reached the mainland",
            "A note",
        ]
        .into_iter()
        .map(|title| BriefingItem {
            selection: None,
            title: title.into(),
            detail: String::new(),
            kind: BriefingItemKind::Beat,
            tone: world_projection::Tone::Neutral,
        })
        .collect(),
        eyebrow: String::new(),
        title: String::new(),
        returned: true,
    });
    let window = cx.add_window(move |_, _| {
        let mut view = ProjectionView::controlled(Still(snapshot));
        view.looking.opening = None;
        view.retelling = Some(0);
        view
    });
    let view = window.root(cx).expect("the World");
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(px(1100.0), px(800.0)));
    cx.run_until_parked();
    // The first frames bake the place right here, seconds on a slow
    // machine, and the beat may go on by itself meanwhile: the film is
    // put back on its first beat until a frame of it is drawn with the
    // beat still there (once the place is baked, a frame is quick).
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
    // Where Next is, on that frame. From here no frame is drawn before
    // the click, so how long one takes never counts.
    let next = cx.debug_bounds("retelling-next").expect("Next");
    // A second into a beat: a clock left running by the click would
    // still be a second behind it.
    let late = || Some(Instant::now() - Duration::from_secs(1));
    // A click on Next, a second into the first beat.
    view.update(cx, |view, _| {
        view.looking.beat_at = late();
    });
    let clicked = Instant::now();
    cx.simulate_click(next.center(), Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.retelling, Some(1), "the next beat");
        assert!(view.looking.beat_at >= Some(clicked), "told from its start");
    });
    // The Right arrow, a second into that one.
    view.update(cx, |view, _| {
        view.looking.beat_at = late();
    });
    let pressed = Instant::now();
    cx.simulate_keystrokes("right");
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.retelling, Some(2), "the last beat");
        assert!(view.looking.beat_at >= Some(pressed), "told from its start");
    });
}

/// A return film of three beats (a catch sold, something the player
/// began finished, a note left), as the window shows it in English,
/// Chinese and Japanese; and each beat's subject.
fn film_in_every_language() -> (Vec<(&'static str, ProjectionSnapshot)>, [SelectionId; 3]) {
    use world_projection::{
        BriefingItem, BriefingItemKind, BriefingProjection, InspectorProjection, InspectorRow,
        InspectorSection,
    };
    let mut english = day_1082();
    let entity = |id: u64| SelectionId::from_stable_key(&format!("entity-{id}")).unwrap();
    // The place itself, a person and a thing built on it.
    let (place, person, thing) = (entity(101), entity(1), entity(702));
    let label = |id: SelectionId| {
        english
            .canvas
            .items
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.label.clone())
            .unwrap()
    };
    let (person_name, thing_name) = (label(person), label(thing));
    // Every thing on the scene by the name the World keeps for it, as
    // a real snapshot has them; the place's is lowercase.
    let place_name = label(place).to_lowercase();
    let titles = english
        .canvas
        .items
        .iter()
        .map(|item| {
            let title = if item.id == place {
                place_name.clone()
            } else {
                item.label.clone()
            };
            (item.id, title)
        })
        .collect::<Vec<_>>();
    for (item, title) in titles {
        english.inspectors.insert(
            item,
            InspectorProjection {
                selection: item,
                title,
                subtitle: String::new(),
                sections: Vec::new(),
            },
        );
    }
    let event = |id: u64, rows: Vec<(&str, String)>| {
        let selection = SelectionId::from_stable_key(&format!("event-{id}")).expect("an event");
        (
            selection,
            rows.into_iter()
                .map(|(label, value)| InspectorRow {
                    label: label.into(),
                    value,
                })
                .collect::<Vec<_>>(),
        )
    };
    let beats = [
        // A catch sold: who caught it, and where it went.
        (
            event(
                341,
                vec![
                    ("Who", person_name.clone()),
                    (
                        "With",
                        format!("{person_name}, {place_name}, Mainland Fish Market"),
                    ),
                ],
            ),
            person,
        ),
        // Something the player began, finished: no one did it.
        (event(329, vec![("With", thing_name.clone())]), thing),
        // A note left.
        (
            event(
                352,
                vec![("Who", person_name.clone()), ("With", person_name.clone())],
            ),
            person,
        ),
    ];
    let mut items = Vec::new();
    for ((selection, rows), _) in &beats {
        english.inspectors.insert(
            *selection,
            InspectorProjection {
                selection: *selection,
                title: String::new(),
                subtitle: String::new(),
                sections: vec![InspectorSection {
                    title: "Context".into(),
                    rows: rows.clone(),
                }],
            },
        );
        items.push(BriefingItem {
            selection: Some(*selection),
            title: "A beat".into(),
            detail: String::new(),
            kind: BriefingItemKind::Beat,
            tone: world_projection::Tone::Neutral,
        });
    }
    english.briefing = Some(BriefingProjection {
        eyebrow: String::new(),
        title: String::new(),
        items,
        returned: true,
    });
    // The scene as the window shows it in another language: its labels
    // translated, what the events say not.
    let shown_as = |names: [(SelectionId, &str); 3]| {
        let mut snapshot = english.clone();
        for item in &mut snapshot.canvas.items {
            if let Some((_, shown)) = names.iter().find(|(id, _)| *id == item.id) {
                item.label = (*shown).into();
            }
        }
        snapshot
    };
    let languages = [
        ("English", english.clone()),
        (
            "Chinese",
            shown_as([(person, "乔纳斯"), (thing, "港口灯"), (place, "港口")]),
        ),
        (
            "Japanese",
            shown_as([(person, "ジョナス"), (thing, "港の灯り"), (place, "港")]),
        ),
    ];
    let subjects = beats.map(|(_, subject)| subject);
    (languages.into(), subjects)
}

/// Each beat's words show within 2.5 s of Next, on its own subject, in
/// every language, on every run, however painting and the display go
/// (v0.29 round 2: beats went silent in three runs of four). Played on
/// a clock the test holds, never the wall clock.
#[test]
fn every_beat_speaks_on_its_subject_within_two_and_a_half_seconds_in_every_language() {
    let (languages, _) = film_in_every_language();
    for (language, snapshot) in &languages {
        let stage =
            crate::diorama::stage_at(snapshot, 1100.0, 748.0, crate::diorama::Clock::at(12));
        let beats = snapshot.briefing.as_ref().unwrap().beats();
        // The film twice over, so a subject comes round again.
        let views = beats
            .iter()
            .chain(beats.iter())
            .map(|beat| Camera::on(&stage, beat_box(snapshot, &stage, beat).expect("framed")))
            .collect::<Vec<_>>();
        let runs = super::super::glide::tests::film_runs(&views, Camera::whole(&stage));
        assert!(!runs.is_empty(), "{language}");
    }
}

/// Each beat of a return is about its own subject, by the World's own
/// names for things, whatever language the window is in: the camera
/// frames it, it glows, and its face is the one shown. In Chinese and
/// Japanese the scene's labels are translated and the event's names
/// are not, and the camera never moved (v0.29); a catch that left the
/// place framed the whole place, not the fisher.
#[test]
fn each_return_beat_frames_its_own_subject_in_every_language() {
    let (languages, subjects) = film_in_every_language();
    let beats = subjects.map(|subject| ((), subject));
    for (language, snapshot) in &languages {
        let stage =
            crate::diorama::stage_at(snapshot, 1100.0, 748.0, crate::diorama::Clock::at(12));
        let beats_shown = snapshot.briefing.as_ref().unwrap().beats();
        for (beat, (_, subject)) in beats_shown.into_iter().zip(&beats) {
            let targets = beat_targets(snapshot, beat);
            assert_eq!(
                targets,
                [*subject].into(),
                "{language}: what the beat is about"
            );
            // The face shown is the subject's.
            let face = super::super::beat_subject(snapshot, beat)
                .first()
                .map(|item| item.id);
            assert_eq!(face, Some(*subject), "{language}: the face");
            // The camera frames the subject itself, or where it is if
            // it is not out on the scene.
            let index = |id: SelectionId| {
                snapshot
                    .canvas
                    .items
                    .iter()
                    .position(|item| item.id == id)
                    .unwrap()
            };
            let own = stage.frame_of(index(*subject)).or_else(|| {
                let at = snapshot.canvas.items[index(*subject)].at?;
                stage.frame_of(index(at))
            });
            let framed = beat_box(snapshot, &stage, beat);
            let near = |a: f32, b: f32| (a - b).abs() < 0.01;
            assert!(
                match (framed, own) {
                    (Some(a), Some(b)) =>
                        near(a.0, b.0) && near(a.1, b.1) && near(a.2, b.2) && near(a.3, b.3),
                    _ => false,
                },
                "{language}: the camera on {subject:?}: {framed:?}, not {own:?}"
            );
        }
    }
}

#[test]
fn report_is_offered_only_under_a_line_the_ai_voice_wrote() {
    let exchange = |voiced: bool, reported: bool| world_projection::Exchange {
        who: someone(),
        words: "Is the pier hard to build?".into(),
        answer: "Not with the right timber.".into(),
        moment: SelectionId::from_stable_key("event-7").expect("an event key"),
        asks_for: None,
        voiced,
        reported,
    };
    // A built-in line is the World's own: never a Report link under it.
    assert_eq!(asking::report_command(&exchange(false, false)), None);
    assert_eq!(asking::report_command(&exchange(false, true)), None);
    // A line the AI voice wrote can be reported, once.
    assert_eq!(
        asking::report_command(&exchange(true, false)),
        Some((7, "conversation.report:7".to_string()))
    );
    assert_eq!(asking::report_command(&exchange(true, true)), None);
}

/// A World that answers whatever is said to someone with fresh things to
/// ask them, and keeps what it was asked.
struct Answering {
    now: ProjectionSnapshot,
    heard: std::rc::Rc<std::cell::RefCell<Vec<ProjectionIntent>>>,
}

/// What is offered to be asked of `who`, the `round`th time.
fn chips_for(who: SelectionId, round: usize) -> world_projection::Openers {
    world_projection::Openers {
        who,
        lines: (0..3)
            .map(|chip| format!("Question {chip} of round {round}?"))
            .collect(),
    }
}

impl crate::ProjectionController for Answering {
    fn snapshot(&self) -> ProjectionSnapshot {
        self.now.clone()
    }

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
        if let ProjectionIntent::Say { to, words, .. } = &intent {
            let round = self.now.exchanges.len() + 1;
            self.now.exchanges.push(world_projection::Exchange {
                who: *to,
                words: words.clone(),
                answer: format!("An answer to {words}"),
                moment: SelectionId::from_stable_key(&format!("event-{round}"))
                    .expect("an event key"),
                asks_for: None,
                voiced: false,
                reported: false,
            });
            self.now.openers = vec![chips_for(*to, round)];
        }
        self.heard.borrow_mut().push(intent);
        Ok(self.now.clone())
    }
}

/// Through GPUI's test window (1.0 P4a): someone's card shows three
/// things to ask them as chips, so typing is never needed; a click asks
/// one exactly as typing it would (said with the World's own ears); and
/// after the answer the chips are fresh, still there to click.
#[gpui::test]
fn chips_stand_under_the_talk_and_a_click_asks_one(cx: &mut gpui::TestAppContext) {
    use gpui::{Modifiers, VisualTestContext};
    let mut snapshot = day_1082();
    let someone = snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .nth(3)
        .map(|item| item.id)
        .expect("someone");
    snapshot.capabilities.talk = true;
    snapshot.exchanges.clear();
    snapshot.briefing = None;
    snapshot.openers = vec![chips_for(someone, 0)];
    let heard = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let answering = Answering {
        now: snapshot,
        heard: heard.clone(),
    };
    let window = cx.add_window(move |_, _| {
        let mut view = ProjectionView::controlled(answering);
        view.looking.opening = None;
        view
    });
    let view = window.root(cx).expect("the World");
    let cx = &mut VisualTestContext::from_window(window.into(), cx);
    cx.simulate_resize(gpui::size(px(1100.0), px(900.0)));
    cx.run_until_parked();
    view.update(cx, |view, cx| view.ask(someone, cx));
    let settle = |cx: &mut VisualTestContext| {
        std::thread::sleep(Duration::from_secs_f32(CAMERA_SECONDS + 0.2));
        for _ in 0..4 {
            view.update(cx, |_, cx| cx.notify());
            cx.executor().advance_clock(Duration::from_millis(500));
            cx.run_until_parked();
        }
    };
    settle(cx);
    const CHIPS: [&str; 3] = ["opener-0", "opener-1", "opener-2"];
    for round in 0..2 {
        let chips = CHIPS
            .iter()
            .map(|chip| cx.debug_bounds(chip))
            .collect::<Vec<_>>();
        assert!(
            chips.iter().all(Option::is_some),
            "round {round}: three chips under the talk"
        );
        let asked = chips_for(someone, round).lines[1].clone();
        cx.simulate_click(chips[1].expect("a chip").center(), Modifiers::none());
        cx.run_until_parked();
        let said = heard.borrow().iter().any(|intent| {
            matches!(intent, ProjectionIntent::Say { to, words, ears }
                if *to == someone && *words == asked && *ears == world_projection::Ears::World)
        });
        assert!(
            said,
            "round {round}: the chip is said: {:?}",
            heard.borrow()
        );
        view.read_with(cx, |view, _| {
            assert_eq!(
                view.snapshot.openers,
                vec![chips_for(someone, round + 1)],
                "fresh chips after the answer"
            );
        });
        settle(cx);
    }
}
