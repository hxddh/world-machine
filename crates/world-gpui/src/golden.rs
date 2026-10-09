//! Golden pictures of the diorama, the strip and the postcard, drawn
//! offscreen through GPUI's own frame and compared with PNGs kept under
//! `tests/golden/`.
//!
//! GPUI's `Window::render_to_image` needs a headless renderer, and the only
//! one GPUI has is Metal's: on Linux (and in any `cargo test` without a
//! GPU) there is none. So these tests hand GPUI's test platform a small
//! reference rasteriser of their own ([`Raster`]) through
//! `HeadlessAppContext::with_platform`. It is given the very `Scene` GPUI
//! would give Metal (every quad, shadow and path the views painted, in
//! draw order, clipped to its content mask) and fills it on the CPU the
//! way GPUI's shaders do: rounded quads with borders and linear gradients,
//! blurred shadows, paths as triangles, and images (the painter's still
//! layers) sampled as GPUI samples them. Text is not drawn (the test
//! platform's text system shapes no glyphs). What the
//! pictures prove is that the scene GPUI paints is the one we meant: a
//! change in the art, the layout or the order of layers shows as a changed
//! picture. They are not Metal's pixels; on macOS a `render_to_image`
//! through `gpui_platform::current_headless_renderer()` would give those.
//!
//! Run with `WORLD_GPUI_UPDATE_GOLDEN=1` to draw the pictures afresh after
//! a change meant to alter them, and look at them before committing.

use crate::diorama::{self, Camera, Glows};
use crate::offscreen::*;
use crate::scene::Daylight;
use gpui::{
    div, prelude::*, px, size, AnyElement, App, HeadlessAppContext, NoopTextSystem,
    PlatformHeadlessRenderer, Window,
};
use image::RgbaImage;
use std::path::PathBuf;
use std::sync::Arc;
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasMark, MarkShape, ProjectionSnapshot, SelectionId,
};

/// A channel may differ by this much and still count as the same.
const CHANNEL_TOLERANCE: u8 = 6;
/// The share of pixels that may differ, for floating-point drift between
/// machines along anti-aliased edges.
const PIXEL_TOLERANCE: f64 = 0.002;

/// Compares `image` with the golden picture `name`, or keeps it as the
/// golden picture when asked to.
fn matches_golden(name: &str, image: &RgbaImage) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.png"));
    if std::env::var_os("WORLD_GPUI_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image.save(&path).expect("the golden picture is written");
        return;
    }
    let golden = image::open(&path)
        .unwrap_or_else(|error| {
            panic!(
                "no golden picture at {} ({error}); draw it with WORLD_GPUI_UPDATE_GOLDEN=1",
                path.display()
            )
        })
        .to_rgba8();
    let drawn = std::env::temp_dir().join(format!("world-gpui-{name}.png"));
    assert_eq!(
        golden.dimensions(),
        image.dimensions(),
        "{name} changed size"
    );
    let differing = golden
        .pixels()
        .zip(image.pixels())
        .filter(|(want, got)| {
            want.0
                .iter()
                .zip(got.0)
                .any(|(a, b)| a.abs_diff(b) > CHANNEL_TOLERANCE)
        })
        .count();
    let share = differing as f64 / (golden.width() * golden.height()) as f64;
    if share > PIXEL_TOLERANCE {
        let _ = image.save(&drawn);
        panic!(
            "{name} differs from its golden picture in {differing} pixels ({:.2}%); \
             drawn now: {}; if the change is meant, redraw with WORLD_GPUI_UPDATE_GOLDEN=1",
            share * 100.0,
            drawn.display()
        );
    }
}

fn id(n: u64) -> SelectionId {
    SelectionId::from_stable_key(&format!("entity-{n}")).expect("an entity key")
}

fn item(n: u64, kind: CanvasItemKind, label: &str, x: f32, at: Option<u64>) -> CanvasItem {
    CanvasItem {
        id: id(n),
        kind,
        label: label.into(),
        detail: String::new(),
        x,
        y: 0.5,
        changes: Vec::new(),
        shape: None,
        at: at.map(id),
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

/// A small harbour town: four places, four people, three things built.
fn town() -> ProjectionSnapshot {
    let places = ["Harbor", "Bakery", "Square", "Lighthouse"];
    let people = ["Mara Quinn", "Leo Park", "Nia Chen", "Evan Moss"];
    let mut snapshot = ProjectionSnapshot {
        title: "Harbor Town".into(),
        ..ProjectionSnapshot::default()
    };
    for (index, place) in places.iter().enumerate() {
        snapshot.canvas.items.push(item(
            100 + index as u64,
            CanvasItemKind::Place,
            place,
            index as f32 / 4.0,
            None,
        ));
    }
    for (index, person) in people.iter().enumerate() {
        snapshot.canvas.items.push(item(
            1 + index as u64,
            CanvasItemKind::Actor,
            person,
            index as f32 / 4.0,
            Some(100 + index as u64),
        ));
    }
    for shape in [MarkShape::House, MarkShape::Tree, MarkShape::Lamp] {
        snapshot.canvas.marks.push(CanvasMark {
            label: format!("{shape:?}"),
            shape,
            selection: None,
        });
    }
    snapshot.letters.push(world_projection::Letter {
        from: id(1),
        note: "The pier is mended.".into(),
        moment: id(1),
    });
    snapshot
}

/// A harbour on the water: the town with a boat at the quay, one of its
/// people at home indoors from nine at night, and the others out late.
fn harbour() -> ProjectionSnapshot {
    let mut snapshot = town();
    snapshot.scenery = Some(world_projection::Scenery {
        sky_top: 0x9cc6e6,
        sky_bottom: 0xf0ead8,
        far: 0x8fae7e,
        near: 0x4f86a8,
        sun: 0xffe2a0,
    });
    let mut boat = item(200, CanvasItemKind::Object, "Boat", 0.0, Some(100));
    boat.shape = Some(MarkShape::Boat);
    snapshot.canvas.items.push(boat);
    let mut tree = item(105, CanvasItemKind::Place, "Orchard", 0.9, None);
    tree.shape = Some(MarkShape::Tree);
    snapshot.canvas.items.push(tree);
    let home = id(101);
    let work = id(102);
    if let Some(leo) = snapshot
        .canvas
        .items
        .iter_mut()
        .find(|item| item.id == id(2))
    {
        leo.day = vec![
            world_projection::RoutineStop {
                from_hour: 7,
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
    snapshot
}

/// The diorama of `snapshot`, `width` by `height`, at a pinned moment.
fn diorama_frame(
    snapshot: &ProjectionSnapshot,
    width: f32,
    height: f32,
    daylight: Daylight,
    hour: f32,
) -> diorama::Frame {
    let stage = diorama::stage_at(
        snapshot,
        width,
        height,
        diorama::Clock::at(hour.floor() as u8),
    );
    let living = diorama::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
    diorama::frame(
        snapshot,
        &stage,
        &living,
        Camera::whole(&stage),
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    )
    .at_hour(hour)
}

fn painted(frame: diorama::Frame) -> AnyElement {
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| diorama::paint(&frame, bounds, window),
    )
    .size_full()
    .into_any_element()
}

#[test]
fn the_diorama_by_day_matches_its_golden_picture() {
    let frame = diorama_frame(&town(), 480.0, 300.0, Daylight::Day, 13.0);
    let image = draw(480.0, 300.0, move || painted(frame.clone()));
    let bare = image.pixels().filter(|pixel| pixel.0[3] < 255).count();
    assert!(bare == 0, "the diorama leaves {bare} pixels bare");
    matches_golden("diorama-day", &image);
}

#[test]
fn the_diorama_at_dusk_matches_its_golden_picture() {
    let frame = diorama_frame(&town(), 480.0, 300.0, Daylight::Dusk, 19.5);
    let image = draw(480.0, 300.0, move || painted(frame.clone()));
    matches_golden("diorama-dusk", &image);
}

/// The harbour at night: windows lit with their glow, someone at home
/// seen in one, and nobody else about but the night owls.
#[test]
fn the_diorama_at_night_matches_its_golden_picture() {
    let frame = diorama_frame(&harbour(), 480.0, 300.0, Daylight::Night, 23.0);
    let image = draw(480.0, 300.0, move || painted(frame.clone()));
    matches_golden("diorama-night", &image);
}

/// Each season on the harbour: blossom in spring, full summer, leaves in
/// autumn, and in winter snow on the roofs and the ground and ice at the
/// water's edge.
#[test]
fn each_season_matches_its_golden_picture() {
    use world_projection::{GroundCover, Season};
    for (name, season, cover, ice) in [
        (
            "season-spring",
            Season::Spring,
            Some(GroundCover::Blossom),
            false,
        ),
        ("season-summer", Season::Summer, None, false),
        (
            "season-autumn",
            Season::Autumn,
            Some(GroundCover::Leaves),
            false,
        ),
        (
            "season-winter",
            Season::Winter,
            Some(GroundCover::Snow),
            true,
        ),
    ] {
        let mut snapshot = harbour();
        snapshot.canvas.season = Some(season);
        snapshot.canvas.ground = cover;
        snapshot.canvas.ice = ice;
        let frame = diorama_frame(&snapshot, 480.0, 300.0, Daylight::Day, 11.0);
        let image = draw(480.0, 300.0, move || painted(frame.clone()));
        matches_golden(name, &image);
    }
}

/// A place three windows wide, panned to its far end: the camera follows
/// `px`, and the far hills move less than the ground.
#[test]
fn a_panorama_panned_along_matches_its_golden_picture() {
    let mut snapshot = harbour();
    snapshot.canvas.width = Some(3.0);
    let places = snapshot
        .canvas
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.kind != CanvasItemKind::Actor)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    for (n, index) in places.iter().enumerate() {
        snapshot.canvas.items[*index].px = Some(0.35 + n as f32 * 0.62);
    }
    let (width, height) = (480.0, 300.0);
    let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(13));
    assert_eq!(stage.width, width * 3.0);
    let living = diorama::living(
        &stage,
        &snapshot,
        0.0,
        Daylight::Day,
        &Default::default(),
        None,
    );
    let camera = Camera::around(&stage, 1.0, stage.width, height / 2.0);
    assert!((camera.x - (stage.width - width / 2.0)).abs() < 0.01);
    let frame = diorama::frame(
        &snapshot,
        &stage,
        &living,
        camera,
        0.0,
        Daylight::Day,
        &Glows::new(),
        1.0,
    )
    .at_hour(15.0);
    let image = draw(width, height, move || painted(frame.clone()));
    matches_golden("panorama-east", &image);
}

/// A view showing the World window's scene: still layers painted off the
/// window's thread.
struct SceneView(diorama::Frame);

impl gpui::Render for SceneView {
    fn render(&mut self, window: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .child(diorama::scene(self.0.clone(), window))
    }
}

/// The World window never waits for a picture: its first frame is drawn at
/// once with stand-ins (the sky's colours, the land's), the still layers
/// arrive from the painter's threads and fade in, and once they have the
/// scene is the same picture as one painted all at once.
#[test]
fn the_scene_draws_at_once_and_its_still_layers_arrive_and_fade_in() {
    crate::painter::paint_elsewhere(true);
    let frame = diorama_frame(&town(), 480.0, 300.0, Daylight::Day, 13.0);
    let mut cx =
        HeadlessAppContext::with_platform(Arc::new(NoopTextSystem::new()), Arc::new(()), || {
            Some(Box::new(Raster(Arc::default())) as Box<dyn PlatformHeadlessRenderer>)
        });
    let started = std::time::Instant::now();
    let window = cx
        .open_window(size(px(480.0), px(300.0)), move |_, cx: &mut App| {
            cx.new(|_| SceneView(frame.clone()))
        })
        .expect("a window");
    cx.run_until_parked();
    let first = cx.capture_screenshot(window.into()).expect("a picture");
    let bare = first.pixels().filter(|pixel| pixel.0[3] < 255).count();
    if let Some(path) = std::env::var_os("WORLD_GPUI_FIRST_FRAME") {
        let _ = first.save(path);
    }
    assert_eq!(bare, 0, "the first frame is whole, with stand-ins");
    let _ = started;
    // The still layers arrive, then fade in over a third of a second.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut settled = None;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(40));
        cx.update_window(window.into(), |_, window, _| window.refresh())
            .expect("a window");
        cx.run_until_parked();
        if crate::painter::idle() {
            // The pictures are handed to the display a frame's share at a
            // time: a few dozen frames more, then the fades.
            for _ in 0..60 {
                std::thread::sleep(std::time::Duration::from_millis(5));
                cx.update_window(window.into(), |_, window, _| window.refresh())
                    .expect("a window");
                cx.run_until_parked();
            }
            std::thread::sleep(std::time::Duration::from_millis(450));
            cx.update_window(window.into(), |_, window, _| window.refresh())
                .expect("a window");
            cx.run_until_parked();
            cx.update_window(window.into(), |_, window, _| window.refresh())
                .expect("a window");
            cx.run_until_parked();
            settled = Some(cx.capture_screenshot(window.into()).expect("a picture"));
            break;
        }
    }
    crate::painter::paint_elsewhere(false);
    let settled = settled.expect("the still layers arrive");
    matches_golden("diorama-day", &settled);
}

/// Paints `frame` in a window of its own, the still layers painted
/// elsewhere as in the app, until they settle: every frame drawn once all
/// the pictures are painted, and the settled frame.
fn settle(frame: diorama::Frame, (width, height): (f32, f32)) -> (Vec<RgbaImage>, RgbaImage) {
    let mut cx =
        HeadlessAppContext::with_platform(Arc::new(NoopTextSystem::new()), Arc::new(()), || {
            Some(Box::new(Raster(Arc::default())) as Box<dyn PlatformHeadlessRenderer>)
        });
    let window = cx
        .open_window(size(px(width), px(height)), move |_, cx: &mut App| {
            cx.new(|_| SceneView(frame.clone()))
        })
        .expect("a window");
    cx.run_until_parked();
    let refresh = |cx: &mut HeadlessAppContext| {
        cx.update_window(window.into(), |_, window, _| window.refresh())
            .expect("a window");
        cx.run_until_parked();
    };
    let _ = diorama::rough_overlap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !crate::painter::idle() {
        assert!(
            std::time::Instant::now() < deadline,
            "the still layers arrive"
        );
        let overlap = diorama::rough_overlap();
        assert!(
            overlap < 1.0,
            "the rough painting drawn where a sharp picture is: {overlap} px²"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
        refresh(&mut cx);
    }
    // Painted: the pictures are handed to the display a frame's share at a
    // time, and what arrives fades in. Every one of these frames is looked at.
    let mut between = Vec::new();
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        refresh(&mut cx);
        between.push(cx.capture_screenshot(window.into()).expect("a picture"));
        let overlap = diorama::rough_overlap();
        assert!(
            overlap < 1.0,
            "the rough painting drawn where a sharp picture is: {overlap} px²"
        );
    }
    std::thread::sleep(std::time::Duration::from_millis(450));
    refresh(&mut cx);
    refresh(&mut cx);
    let settled = cx.capture_screenshot(window.into()).expect("a picture");
    (between, settled)
}

/// How many pixels of `a` differ from `b` by more than a few levels.
fn differing(a: &RgbaImage, b: &RgbaImage) -> usize {
    a.pixels()
        .zip(b.pixels())
        .filter(|(p, q)| (0..4).any(|c| (p.0[c] as i16 - q.0[c] as i16).abs() > 4))
        .count()
}

/// The rough painting only ever stands in where a sharp picture is not
/// there yet: never under one, never over one. A settled scene is the same
/// with it as without it, and so is every frame once all the sharp pictures
/// are painted and with the display (the rough would otherwise show through
/// what is transparent in a picture fading in: its shadows doubled, its
/// tile's edge drawn). In a town and at a place of a second Pack.
#[test]
fn the_rough_painting_never_shows_where_a_sharp_picture_is() {
    crate::painter::paint_elsewhere(true);
    for (name, snapshot) in [("town", town()), ("maple", place("maple"))] {
        let size = (900.0, 560.0);
        let frame = || diorama_frame(&snapshot, size.0, size.1, Daylight::Day, 13.0);
        diorama::set_rough_off(false);
        let (between, settled) = settle(frame(), size);
        diorama::set_rough_off(true);
        let (_, without) = settle(frame(), size);
        diorama::set_rough_off(false);
        let tolerance = (size.0 * size.1 * 0.001) as usize;
        let off = differing(&settled, &without);
        if off > tolerance {
            let _ = settled.save(format!("/tmp/world-gpui-rough-{name}-with.png"));
            let _ = without.save(format!("/tmp/world-gpui-rough-{name}-without.png"));
        }
        assert!(
            off <= tolerance,
            "{name}: settled with the rough painting differs in {off} pixels"
        );
        // Once a frame matches, it stays matched: no rough drawn back.
        let first = between
            .iter()
            .position(|image| differing(image, &without) <= tolerance);
        for (index, image) in between
            .iter()
            .enumerate()
            .skip(first.unwrap_or(between.len()))
        {
            let off = differing(image, &without);
            if off > tolerance {
                let _ = image.save(format!("/tmp/world-gpui-rough-{name}-{index}.png"));
            }
            assert!(
                off <= tolerance,
                "{name}: frame {index} after painting differs in {off} pixels"
            );
        }
    }
    crate::painter::paint_elsewhere(false);
}

/// A new look fades in over the old one without drawing anything twice:
/// while it fades, no pixel is darker than both the old settled picture
/// and the new (a contact shadow drawn under its own replacement, half
/// faded, is darker than either). A town, at noon and then at three.
#[test]
fn a_new_look_fades_in_without_doubling_its_shadows() {
    crate::painter::paint_elsewhere(true);
    let snapshot = town();
    let (width, height) = (900.0, 560.0);
    let noon = diorama_frame(&snapshot, width, height, Daylight::Day, 12.0);
    let three = diorama_frame(&snapshot, width, height, Daylight::Day, 15.0);
    let mut cx =
        HeadlessAppContext::with_platform(Arc::new(NoopTextSystem::new()), Arc::new(()), || {
            Some(Box::new(Raster(Arc::default())) as Box<dyn PlatformHeadlessRenderer>)
        });
    let window = cx
        .open_window(size(px(width), px(height)), move |_, cx: &mut App| {
            cx.new(|_| SceneView(noon.clone()))
        })
        .expect("a window");
    cx.run_until_parked();
    let refresh = |cx: &mut HeadlessAppContext| {
        cx.update_window(window.into(), |_, window, _| window.refresh())
            .expect("a window");
        cx.run_until_parked();
    };
    let settle = |cx: &mut HeadlessAppContext| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        let mut frames = Vec::new();
        while !crate::painter::idle() {
            assert!(
                std::time::Instant::now() < deadline,
                "the still layers arrive"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
            refresh(cx);
            frames.push(cx.capture_screenshot(window.into()).expect("a picture"));
        }
        for _ in 0..40 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            refresh(cx);
            frames.push(cx.capture_screenshot(window.into()).expect("a picture"));
        }
        std::thread::sleep(std::time::Duration::from_millis(450));
        refresh(cx);
        refresh(cx);
        (
            frames,
            cx.capture_screenshot(window.into()).expect("a picture"),
        )
    };
    let (_, before) = settle(&mut cx);
    window
        .update(&mut cx, |view, _, cx| {
            view.0 = three.clone();
            cx.notify();
        })
        .expect("a window");
    let (between, after) = settle(&mut cx);
    crate::painter::paint_elsewhere(false);
    let tolerance = (width * height * 0.001) as usize;
    for (index, image) in between.iter().enumerate() {
        let darker = image
            .pixels()
            .zip(before.pixels().zip(after.pixels()))
            .filter(|(p, (a, b))| luma(p) + 10.0 < luma(a).min(luma(b)))
            .count();
        if darker > tolerance {
            let _ = image.save(format!("/tmp/world-gpui-fade-{index}.png"));
            let _ = before.save("/tmp/world-gpui-fade-before.png");
            let _ = after.save("/tmp/world-gpui-fade-after.png");
        }
        assert!(
            darker <= tolerance,
            "frame {index} of the fade: {darker} pixels darker than before and after"
        );
    }
}

/// How long this thread has been running on a CPU, so a frame is timed by
/// its own work and not by the other programs a busy machine is running
/// meanwhile.
fn on_cpu() -> std::time::Duration {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `now` is a valid timespec for clock_gettime to fill in.
    let status = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut now) };
    if status != 0 {
        return std::time::Duration::ZERO;
    }
    std::time::Duration::new(now.tv_sec as u64, now.tv_nsec as u32)
}

/// A moment strip over a scene: the snapshot it is drawn from, and the
/// moment.
type StripOver = Option<(ProjectionSnapshot, world_projection::Moment)>;

/// A view of a scene whose frame the test can change between frames, with
/// a moment's strip over it when the test puts one there.
struct LiveScene(
    std::rc::Rc<std::cell::RefCell<diorama::Frame>>,
    std::rc::Rc<std::cell::RefCell<StripOver>>,
);

impl gpui::Render for LiveScene {
    fn render(&mut self, window: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let frame = self.0.borrow().clone();
        let (width, height) = (frame.width_of_view(), frame.height_of_view());
        let started = std::time::Instant::now();
        let strip = self.1.borrow().as_ref().map(|(snapshot, moment)| {
            let layout = crate::window::strip_layout(width, height);
            div()
                .absolute()
                .left(px(layout.x))
                .top(px(layout.y))
                .child(crate::window::moment_strip(snapshot, moment, layout))
        });
        crate::painter::note_frame(started.elapsed());
        div()
            .size_full()
            .relative()
            .child(diorama::scene(frame, window))
            .children(strip)
    }
}

/// The v0.21 bar for hitches, tightened in v0.25: at 1440 by 900 at
/// twice the pixels, a three-year World's window never spends 8 ms of its
/// own CPU time on a frame, whether it is opening, the hour turning or the
/// view panning into places not yet painted: all of that is painted off
/// the window's thread and faded in, and the pictures are handed to the
/// display a frame's share at a time (copying them in is the window
/// thread's own work), the next ones ahead of being seen. Timed in a
/// release build.
#[test]
#[ignore = "a benchmark: cargo test --release -p world-gpui -- --ignored never_waits"]
fn a_three_year_world_never_waits_for_painting() {
    use std::time::{Duration, Instant};
    crate::painter::paint_elsewhere(true);
    let snapshot = crate::diorama::tests::three_years();
    let (width, height) = (1440.0_f32, 900.0_f32);
    let make_of = |snapshot: &ProjectionSnapshot, hour: f32, pan: f32, daylight: Daylight| {
        let stage = diorama::stage_at(snapshot, width, height, diorama::Clock::at(hour as u8));
        let living = diorama::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
        let camera = Camera::around(&stage, 1.0, stage.width / 2.0 + pan, height / 2.0);
        diorama::frame(
            snapshot,
            &stage,
            &living,
            camera,
            0.0,
            daylight,
            &Glows::new(),
            1.0,
        )
        .at_hour(hour)
    };
    let make = |hour: f32, pan: f32, daylight: Daylight| make_of(&snapshot, hour, pan, daylight);
    let shared = std::rc::Rc::new(std::cell::RefCell::new(make(12.0, 0.0, Daylight::Day)));
    let over = std::rc::Rc::new(std::cell::RefCell::new(None));
    let strip = over.clone();
    let mut cx =
        HeadlessAppContext::with_platform(Arc::new(NoopTextSystem::new()), Arc::new(()), || {
            Some(Box::new(Raster(Arc::default())) as Box<dyn PlatformHeadlessRenderer>)
        });
    let view = shared.clone();
    let started = on_cpu();
    let window = cx
        .open_window(size(px(width), px(height)), move |_, cx: &mut App| {
            cx.new(|_| LiveScene(view.clone(), strip.clone()))
        })
        .expect("a window");
    // Opening the window paints what its first frame shows right there,
    // before the window is shown (the rough painting of what the camera
    // sees, the sky and the hills): no frame the player sees waits for it,
    // so it has a bar of its own.
    let opening = on_cpu().saturating_sub(started);
    let mut worst = Duration::ZERO;
    let mut frames = 0;
    let settle = |cx: &mut HeadlessAppContext, worst: &mut Duration, frames: &mut u32| {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut quiet = 0;
        while Instant::now() < deadline && quiet < 40 {
            std::thread::sleep(Duration::from_millis(8));
            let ours = || {
                let profile = crate::painter::profile().lock().unwrap();
                [
                    "main: plan",
                    "main: draw still",
                    "cpu: plan",
                    "cpu: draw still",
                    "cpu: live",
                    "cpu: pictures",
                    "cpu: image",
                ]
                .map(|part| profile.get(part).copied().unwrap_or_default())
            };
            let before = ours();
            let usage = || {
                let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
                // Only Linux can ask for this thread alone; elsewhere the
                // whole process is counted, which only makes the report
                // generous (it is a report, not the bar).
                #[cfg(target_os = "linux")]
                let whom = libc::RUSAGE_THREAD;
                #[cfg(not(target_os = "linux"))]
                let whom = libc::RUSAGE_SELF;
                // SAFETY: `usage` is a valid rusage for getrusage to fill in.
                unsafe { libc::getrusage(whom, &mut usage) };
                (
                    usage.ru_utime.tv_sec as f64 * 1e3 + usage.ru_utime.tv_usec as f64 / 1e3,
                    usage.ru_stime.tv_sec as f64 * 1e3 + usage.ru_stime.tv_usec as f64 / 1e3,
                    usage.ru_minflt,
                )
            };
            let used = usage();
            let handed = crate::painter::HANDED.with(|handed| handed.get());
            let (started, cpu) = (Instant::now(), on_cpu());
            cx.update_window(window.into(), |_, window, _| window.refresh())
                .expect("a window");
            let (wall, took) = (started.elapsed(), on_cpu().saturating_sub(cpu));
            let after = ours();
            if took > Duration::from_millis(4) || wall > Duration::from_millis(16) {
                let part = |index: usize| after[index] - before[index];
                eprintln!(
                    "a slow frame, {took:?} on the CPU ({wall:?} by the clock): planning {:?}, \
                     drawing the still layers {:?}; on the CPU planning {:?}, still {:?}, \
                     live {:?}, pictures {:?}, the rest {:?}; images {:?}, {} new bytes",
                    part(0),
                    part(1),
                    part(2),
                    part(3),
                    part(4),
                    part(5),
                    took.saturating_sub(part(2) + part(3) + part(4) + part(5)),
                    part(6),
                    crate::painter::HANDED.with(|now| now.get()) - handed,
                );
                let now = usage();
                eprintln!(
                    "  user {:.2} ms, system {:.2} ms, {} page faults",
                    now.0 - used.0,
                    now.1 - used.1,
                    now.2 - used.2,
                );
            }
            *worst = (*worst).max(took);
            *frames += 1;
            quiet = if crate::painter::idle() { quiet + 1 } else { 0 };
        }
    };
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "opening: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    // The hour turns to dusk: every still layer is painted again.
    *shared.borrow_mut() = make(19.5, 0.0, Daylight::Dusk);
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "dusk: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    // The view pans a window and a half along, into tiles not painted.
    *shared.borrow_mut() = make(19.5, width * 1.5, Daylight::Dusk);
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "panned: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    // The still things boil: each of the other two drawings of the
    // buildings is painted, handed over ahead and cut to at once.
    for boil in 1..crate::hand::BOILS {
        let seconds = (boil as f32 + 0.5) / crate::hand::BOIL_RATE;
        let frame = make(19.5, width * 1.5, Daylight::Dusk).at_seconds(seconds);
        assert_eq!(frame.boil(), boil);
        *shared.borrow_mut() = frame;
        let mut phase = Duration::ZERO;
        settle(&mut cx, &mut phase, &mut frames);
        eprintln!(
            "boiled to drawing {boil}: the longest frame {:.2} ms",
            phase.as_secs_f64() * 1000.0
        );
        worst = worst.max(phase);
    }
    // A moment comes: its three panels are painted off the window's thread
    // and fade in over the scene.
    use world_projection::MomentKind;
    *over.borrow_mut() = Some((
        snapshot.clone(),
        moment_of(
            MomentKind::Wedding,
            "A wedding",
            [&[1, 2], &[1, 2, 3], &[1, 2, 3, 4]],
        ),
    ));
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "wedding strip: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    *over.borrow_mut() = Some((
        snapshot.clone(),
        moment_of(MomentKind::Birth, "A birth", [&[5, 6], &[5, 6], &[5, 6, 7]]),
    ));
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "birth strip: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    // The player's designs: two sails, a flag, two shop signs and a quilt
    // in view, each painted off the window's thread, then the hour turning
    // under them and the wind moving them.
    *over.borrow_mut() = None;
    let mut designed = snapshot.clone();
    let design = motif_of(&[9, 0, 2, 4], |x, y| {
        (x / 4 + y / 4) % 3 + usize::from(x == y)
    });
    for item in &mut designed.canvas.items {
        let key = item.id.stable_key();
        match key.as_str() {
            "entity-309" | "entity-312" => item.shape = Some(MarkShape::Boat),
            "entity-310" => item.shape = Some(MarkShape::Flag),
            "entity-120" | "entity-126" => item.shape = Some(MarkShape::Shop),
            "entity-118" => item.shape = Some(MarkShape::House),
            _ => continue,
        }
        item.pattern = Some(design.clone());
    }
    let frame = make_of(&designed, 19.5, 0.0, Daylight::Dusk);
    assert_eq!(frame.worn().len(), 6, "six designs are worn");
    *shared.borrow_mut() = frame;
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "six designs: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    *shared.borrow_mut() = make_of(&designed, 21.0, 0.0, Daylight::Night);
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "six designs at night: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    crate::painter::paint_elsewhere(false);
    eprintln!("{:?}", crate::painter::profile().lock().unwrap());
    eprintln!(
        "{frames} frames; the longest on the window's thread {:.2} ms of its own work \
         (opening {:.2} ms)",
        worst.as_secs_f64() * 1000.0,
        opening.as_secs_f64() * 1000.0
    );
    // Through every phase (the opening, the pan, the zooms, the boil) the
    // rough painting is never drawn where a sharp picture is: no frame
    // pays for the same ground twice.
    let overlap = diorama::rough_overlap();
    assert!(
        overlap < 1.0,
        "the rough painting drawn under or over a sharp picture: {overlap} px²"
    );
    if !cfg!(debug_assertions) {
        assert!(worst < Duration::from_millis(8), "{worst:?}");
        assert!(opening < Duration::from_millis(250), "opening {opening:?}");
    }
}

#[test]
fn the_strip_matches_its_golden_picture() {
    use crate::strip;
    let snapshot = town();
    let (width, height) = (720.0, strip::HEIGHT);
    let stage = strip::stage(&snapshot, width, height);
    let outings = strip::Outings::new(&stage, &snapshot);
    let frame = strip::frame(&snapshot, &stage, &outings, 0.0, Daylight::Day, false).at_hour(13.0);
    let image = draw(width, height, move || {
        div()
            .size_full()
            .relative()
            .child(painted(frame.clone()))
            .child(strip::letter_button(
                "From Mara".into(),
                false,
                width - 56.0,
                80.0,
            ))
            .child(strip::close_button(gpui::black()))
            .into_any_element()
    });
    matches_golden("strip", &image);
}

#[test]
fn the_postcard_matches_its_golden_picture() {
    let snapshot = town();
    let card = crate::postcard::postcard(&snapshot, None);
    let (width, height) = (480.0, 360.0);
    let frame = diorama_frame(&snapshot, width, height, Daylight::Day, 13.0);
    let image = draw(width, height, move || {
        div()
            .size_full()
            .relative()
            .child(painted(frame.clone()))
            .child(crate::window::postcard_paper(&card, width, height))
            .into_any_element()
    });
    matches_golden("postcard", &image);
}

/// A gentle pointer under a handle in the top right, and the zoom control
/// with its own pointer beside it, over the scene: where each sits, its
/// little arrow, and the accent edge that sets it apart from a card.
#[test]
fn the_pointers_and_the_zoom_control_match_their_golden_picture() {
    use crate::pointers::Pointer;
    use crate::window::{pointer_hint, zoom_button, Caret};
    let snapshot = town();
    let (width, height) = (720.0, 420.0);
    let frame = diorama_frame(&snapshot, width, height, Daylight::Day, 13.0);
    let image = draw(width, height, move || {
        div()
            .size_full()
            .relative()
            .child(painted(frame.clone()))
            .child(
                div()
                    .absolute()
                    .top(px(56.0))
                    .right(px(16.0))
                    .child(pointer_hint(Pointer::Drawer, Caret::Up, |_, _, _| {})),
            )
            .child(
                div()
                    .absolute()
                    .left(px(16.0))
                    .top(px(160.0))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_1()
                            .rounded_full()
                            .bg(gpui::white())
                            .child(zoom_button("zoom-in", "+", "Zoom in (+)", false))
                            .child(zoom_button("zoom-out", "−", "Zoom out (−)", true)),
                    )
                    .child(pointer_hint(Pointer::Zoom, Caret::Left, |_, _, _| {})),
            )
            .into_any_element()
    });
    matches_golden("pointers", &image);
}

/// The reference rasteriser itself: a red square half over a blue one,
/// a rounded corner, and a triangle, where they should be.
#[test]
fn the_reference_rasteriser_draws_quads_and_paths_in_order() {
    let image = draw(40.0, 20.0, || {
        div()
            .size_full()
            .relative()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .size(px(10.0))
                    .bg(gpui::blue()),
            )
            .child(
                div()
                    .absolute()
                    .left(px(5.0))
                    .top_0()
                    .size(px(10.0))
                    .bg(gpui::red()),
            )
            .child(
                div()
                    .absolute()
                    .left(px(20.0))
                    .top_0()
                    .size(px(20.0))
                    .rounded(px(10.0))
                    .bg(gpui::green()),
            )
            .into_any_element()
    });
    // The test platform draws at twice the size.
    assert_eq!(image.dimensions(), (80, 40));
    let at = |x, y| image.get_pixel(x, y).0;
    assert_eq!(at(4, 4), [0, 0, 255, 255], "blue where only blue is");
    assert_eq!(at(14, 4), [255, 0, 0, 255], "red laid over blue");
    assert_eq!(at(60, 20)[3], 255, "the circle's middle is filled");
    assert_eq!(at(41, 1)[3], 0, "its corner is round, and empty");
}

/// A look at someone's age.
fn aged(age: world_projection::AgeStage) -> world_projection::Look {
    world_projection::Look {
        age: Some(age),
        grey: age == world_projection::AgeStage::Elder,
        stoop: age == world_projection::AgeStage::Elder,
        ..Default::default()
    }
}

/// The harbour with people of every age: an elder with a cane, a child
/// and a teenager, a baby carried by their mother, and a baby asleep in a
/// pram by the lighthouse with nobody of theirs near.
fn generations() -> ProjectionSnapshot {
    use world_projection::AgeStage;
    let mut snapshot = harbour();
    let home = id(101);
    for (n, age) in [
        (1, AgeStage::Adult),
        (2, AgeStage::Elder),
        (3, AgeStage::Child),
        (4, AgeStage::Teen),
    ] {
        if let Some(person) = snapshot
            .canvas
            .items
            .iter_mut()
            .find(|item| item.id == id(n))
        {
            person.look = Some(aged(age));
            person.home = Some(home);
            person.day.clear();
        }
    }
    // Two of them in drawings of their own, as a Pack draws its people.
    let base = world_projection::person_base("golden-folk");
    for (n, variant) in [(1_u64, 7_u32), (4, 30)] {
        let name = format!("golden-folk-{n}");
        snapshot
            .drawings
            .push(world_projection::person(name.clone(), variant, &base));
        if let Some(person) = snapshot
            .canvas
            .items
            .iter_mut()
            .find(|item| item.id == id(n))
        {
            person.drawing = Some(name);
        }
    }
    let mut baby = item(5, CanvasItemKind::Actor, "Pip Quinn", 0.0, Some(100));
    baby.look = Some(aged(AgeStage::Baby));
    baby.home = Some(home);
    snapshot.canvas.items.push(baby);
    let mut sleeping = item(6, CanvasItemKind::Actor, "Wren Moss", 0.9, Some(103));
    sleeping.look = Some(aged(AgeStage::Baby));
    sleeping.home = Some(id(102));
    snapshot.canvas.items.push(sleeping);
    snapshot
}

#[test]
fn people_of_every_age_match_their_golden_picture() {
    let snapshot = generations();
    let frame = diorama_frame(&snapshot, 640.0, 360.0, Daylight::Day, 13.0);
    // The baby at the harbour is in their mother's arms, not on the ground;
    // the one by the lighthouse sleeps in a pram.
    let carried = frame
        .people
        .iter()
        .filter(|person| person.carrying.is_some())
        .count();
    assert_eq!(carried, 1);
    let babies_standing = frame
        .people
        .iter()
        .filter(|person| person.figure.age == crate::age::Age::Baby)
        .count();
    assert_eq!(babies_standing, 1, "one baby in a pram");
    let child = frame
        .people
        .iter()
        .find(|person| person.figure.age == crate::age::Age::Child)
        .unwrap();
    let grown = frame
        .people
        .iter()
        .find(|person| person.figure.age == crate::age::Age::Adult)
        .unwrap();
    assert!(child.height < grown.height * 0.7);
    let image = draw(640.0, 360.0, move || painted(frame.clone()));
    matches_golden("ages", &image);
}

/// A moment of `kind` in the town: its three panels at the bakery.
fn moment_of(
    kind: world_projection::MomentKind,
    title: &str,
    cast: [&[u64]; 3],
) -> world_projection::Moment {
    use world_projection::{Panel, PanelBeat};
    let panel = |beat: PanelBeat, cast: &[u64], caption: &str| Panel {
        caption: caption.into(),
        cast: cast.iter().map(|n| id(*n)).collect(),
        place: Some(id(101)),
        mood: None,
        beat,
        props: Vec::new(),
    };
    world_projection::Moment {
        id: format!("{kind:?}-12"),
        day: 12,
        kind,
        title: title.into(),
        panels: [
            panel(PanelBeat::Before, cast[0], "Before"),
            panel(PanelBeat::Moment, cast[1], "The moment"),
            panel(PanelBeat::After, cast[2], "After"),
        ],
        event: None,
    }
}

/// A wedding, a birth and a farewell, each as three panels in the town's
/// own look: the bakery behind, the people posed for each beat.
#[test]
fn a_wedding_a_birth_and_a_farewell_match_their_golden_pictures() {
    use world_projection::MomentKind;
    let snapshot = generations();
    for (name, moment) in [
        (
            "moment-wedding",
            moment_of(
                MomentKind::Wedding,
                "Mara and Leo marry",
                [&[1, 3], &[1, 3], &[1, 3, 4]],
            ),
        ),
        (
            "moment-birth",
            moment_of(
                MomentKind::Birth,
                "Pip is born",
                [&[1, 3], &[1, 5, 3], &[1, 5, 3]],
            ),
        ),
        (
            "moment-farewell",
            moment_of(
                MomentKind::Farewell,
                "Leo says goodbye",
                [&[2, 1], &[2, 1, 4], &[1, 4]],
            ),
        ),
    ] {
        let (width, height) = (900.0, 480.0);
        let layout = crate::window::strip_layout(width, height);
        let strip_snapshot = snapshot.clone();
        let image = draw(width, height, move || {
            div()
                .size_full()
                .bg(gpui::rgb(0x6f7a70))
                .relative()
                .child(div().absolute().left(px(layout.x)).top(px(layout.y)).child(
                    crate::window::moment_strip(&strip_snapshot, &moment, layout),
                ))
                .into_any_element()
        });
        matches_golden(name, &image);
    }
}

/// A moment at a place that wears the player's design is drawn with it:
/// the bakery's painted sign hangs from its bracket in every panel, as it
/// does on the scene.
#[test]
fn a_moment_at_a_designed_place_shows_the_design() {
    use world_projection::MomentKind;
    let mut snapshot = generations();
    let sign = designed()
        .canvas
        .items
        .iter()
        .find(|item| item.label == "Bakery")
        .and_then(|item| item.pattern.clone())
        .expect("the bakery's sign");
    let bakery = snapshot
        .canvas
        .items
        .iter_mut()
        .find(|item| item.id == id(101))
        .expect("the bakery");
    bakery.shape = Some(MarkShape::Shop);
    bakery.pattern = Some(sign);
    let moment = moment_of(
        MomentKind::Wedding,
        "Mara and Leo marry",
        [&[1, 3], &[1, 3], &[1, 3, 4]],
    );
    let scenes = crate::window::panel_scenes(&snapshot, &moment);
    assert!(
        scenes.iter().all(|scene| scene
            .place
            .as_ref()
            .is_some_and(|place| place.wears.is_some())),
        "every panel's place wears the design"
    );
    let (width, height) = (900.0, 480.0);
    let layout = crate::window::strip_layout(width, height);
    let image = draw(width, height, move || {
        div()
            .size_full()
            .bg(gpui::rgb(0x6f7a70))
            .relative()
            .child(
                div()
                    .absolute()
                    .left(px(layout.x))
                    .top(px(layout.y))
                    .child(crate::window::moment_strip(&snapshot, &moment, layout)),
            )
            .into_any_element()
    });
    matches_golden("moment-design", &image);
}

/// A design drawn from a rule over its squares, in palette places.
fn motif_of(colours: &[u8], rule: impl Fn(usize, usize) -> usize) -> world_projection::Pattern {
    let cells = (0..crate::mark::CELLS)
        .map(|at| {
            let index = rule(at % crate::mark::SIDE, at / crate::mark::SIDE);
            char::from_digit(index as u32, 16).unwrap()
        })
        .collect::<String>();
    world_projection::Design::new(&cells, colours)
        .expect("a design")
        .pattern()
}

/// The harbour with the player's designs: stripes and a sun on the
/// square's flag, chevrons on the boat's sail, a fish on the bakery's
/// sign, and a patchwork quilt at the lighthouse keeper's.
pub(crate) fn designed() -> ProjectionSnapshot {
    let mut snapshot = harbour();
    let flag = motif_of(&[8, 0, 2], |x, y| {
        let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
        if dx * dx + dy * dy < 12.0 {
            2
        } else {
            (y / 3) % 2
        }
    });
    let sail = motif_of(&[0, 3, 9], |x, y| {
        if y >= 13 {
            2
        } else if (x + y) % 8 < 3 {
            1
        } else {
            0
        }
    });
    let sign = motif_of(&[13, 9, 2, 0], |x, y| {
        let (dx, dy) = (x as f32 - 6.5, y as f32 - 7.5);
        if x == 0 || y == 0 || x == 15 || y == 15 {
            1
        } else if (dx / 4.2).powi(2) + (dy / 2.6).powi(2) < 1.0 {
            if x == 4 && y == 6 {
                3
            } else {
                1
            }
        } else if (10..=13).contains(&x) && (y as i32 - 7).unsigned_abs() as usize <= x - 10 {
            1
        } else if x == 14 || y == 14 || x == 1 || y == 1 {
            2
        } else {
            0
        }
    });
    let quilt = motif_of(&[11, 0, 5, 12, 7], |x, y| {
        let block = (x / 4 + y / 4 * 4) % 4;
        if (x % 4 == 1 || x % 4 == 2) && (y % 4 == 1 || y % 4 == 2) {
            1
        } else {
            [0, 2, 3, 4][block]
        }
    });
    let mut flagpole = item(
        201,
        CanvasItemKind::Object,
        "The square's flag",
        0.0,
        Some(102),
    );
    flagpole.shape = Some(MarkShape::Flag);
    flagpole.pattern = Some(flag);
    snapshot.canvas.items.push(flagpole);
    for item in &mut snapshot.canvas.items {
        match item.label.as_str() {
            "Boat" => item.pattern = Some(sail.clone()),
            "Bakery" => {
                item.shape = Some(MarkShape::Shop);
                item.pattern = Some(sign.clone());
            }
            "Lighthouse" => item.pattern = Some(quilt.clone()),
            _ => {}
        }
    }
    // Keep the lighthouse a home, where a quilt is aired.
    snapshot
}

/// The diorama of `snapshot` at `hour`, close on the item called `label`.
fn close_on(snapshot: &ProjectionSnapshot, label: &str, hour: f32) -> diorama::Frame {
    let (width, height) = (480.0, 300.0);
    let daylight = crate::scene::daylight_at(hour as u32);
    let stage = diorama::stage_at(snapshot, width, height, diorama::Clock::at(hour as u8));
    let index = snapshot
        .canvas
        .items
        .iter()
        .position(|item| item.label == label)
        .expect("the item");
    let living = diorama::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
    let mut camera = Camera::on(&stage, stage.frame_of(index).expect("on stage"));
    camera.zoom = 1.8;
    diorama::frame(
        snapshot,
        &stage,
        &living,
        Camera::around(&stage, camera.zoom, camera.x, camera.y),
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    )
    .at_hour(hour)
}

#[test]
fn designs_on_a_flag_a_sail_a_sign_and_a_quilt_match_their_golden_pictures() {
    let snapshot = designed();
    for (name, label, hour) in [
        ("design-flag", "The square's flag", 13.0),
        ("design-sail", "Boat", 13.0),
        ("design-sign", "Bakery", 13.0),
        ("design-quilt", "Lighthouse", 13.0),
        ("design-quilt-night", "Lighthouse", 22.0),
    ] {
        let frame = close_on(&snapshot, label, hour);
        let worn = frame.worn().len();
        assert_eq!(worn, 4, "{name}: every design is worn");
        let image = draw(480.0, 300.0, move || painted(frame.clone()));
        matches_golden(name, &image);
    }
}

// Composition (v0.24, owner A).

/// The day-1,082 harbour at `hour`, `width` by `height`, with the camera
/// `zoom` times closer (0 for as far out as it goes) at `pan` of the way
/// along it.
pub(crate) fn harbour_1082_frame(
    (width, height): (f32, f32),
    daylight: Daylight,
    hour: f32,
    zoom: f32,
    pan: f32,
) -> diorama::Frame {
    snapshot_frame(
        &crate::diorama::tests::harbour_1082(),
        (width, height),
        daylight,
        hour,
        zoom,
        pan,
    )
}

/// A snapshot's frame, as [`harbour_1082_frame`].
fn snapshot_frame(
    snapshot: &ProjectionSnapshot,
    (width, height): (f32, f32),
    daylight: Daylight,
    hour: f32,
    zoom: f32,
    pan: f32,
) -> diorama::Frame {
    let snapshot = snapshot.clone();
    let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(hour as u8));
    let living = diorama::living(&stage, &snapshot, 0.0, daylight, &Default::default(), None);
    let camera = Camera::around(&stage, zoom, stage.width * pan, height / 2.0);
    diorama::frame(
        &snapshot,
        &stage,
        &living,
        camera,
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    )
    .at_hour(hour)
}

/// Pictures of the composition to look at rather than check: the harbour
/// on day 1,082 at noon, dusk and night, and folded into its postcard.
/// Written to the folder `WORLD_GPUI_PICTURES` names.
#[test]
#[ignore = "pictures: WORLD_GPUI_PICTURES=dir cargo test -p world-gpui -- --ignored composition_pictures"]
fn composition_pictures() {
    let Some(dir) = std::env::var_os("WORLD_GPUI_PICTURES") else {
        return;
    };
    let dir = PathBuf::from(dir);
    let only = std::env::var("WORLD_GPUI_ONLY").unwrap_or_default();
    // `WORLD_GPUI_SNAPSHOT` names another World's wire JSON to picture.
    let (snapshot, prefix) = match std::env::var_os("WORLD_GPUI_SNAPSHOT") {
        Some(path) => {
            let json = std::fs::read_to_string(path).expect("the snapshot");
            let wire: world_pack_protocol::ProjectionSnapshotWire =
                serde_json::from_str(&json).expect("a wire snapshot");
            (
                ProjectionSnapshot::try_from(wire).expect("a snapshot"),
                "other-",
            )
        }
        None => (crate::diorama::tests::harbour_1082(), ""),
    };
    let (width, height) = (1100.0, 848.0);
    for (name, daylight, hour, zoom, pan) in [
        ("noon", Daylight::Day, 12.0, 1.0, 0.3),
        ("noon-east", Daylight::Day, 12.0, 1.0, 0.75),
        ("dusk", Daylight::Dusk, 19.5, 1.0, 0.3),
        ("night", Daylight::Night, 23.0, 1.0, 0.3),
        ("closer", Daylight::Day, 12.0, 1.8, 0.12),
        ("wide", Daylight::Day, 12.0, 0.7, 0.3),
        ("half-folded", Daylight::Day, 12.0, 0.6, 0.3),
        ("postcard-noon", Daylight::Day, 12.0, 0.0, 0.5),
        ("postcard-dusk", Daylight::Dusk, 19.5, 0.0, 0.5),
        ("postcard-night", Daylight::Night, 23.0, 0.0, 0.5),
    ] {
        if !only.is_empty() && !only.split(',').any(|want| want == name) {
            continue;
        }
        // `WORLD_GPUI_PAN=0.5` looks along the place there instead.
        let pan = std::env::var("WORLD_GPUI_PAN")
            .ok()
            .and_then(|pan| pan.parse().ok())
            .unwrap_or(pan);
        let frame = snapshot_frame(&snapshot, (width, height), daylight, hour, zoom, pan);
        let image = draw(width, height, move || painted(frame.clone()));
        image
            .save(dir.join(format!("{prefix}{name}.png")))
            .expect("a picture");
    }
}

/// The v0.24 bar for the whole-town view: zoomed right out, the day-1,082
/// harbour is a postcard that fills the window, by day and at night. The
/// town stands from under a third of the way down to the water at the
/// foot; above it is the sky (blue by day, dark at night, never bare
/// paper), and no row of the window is left unpainted.
#[test]
fn the_whole_town_is_a_postcard_that_fills_the_window() {
    let (width, height) = (550.0, 424.0);
    for (daylight, hour) in [(Daylight::Night, 23.0), (Daylight::Day, 12.0)] {
        let frame = harbour_1082_frame((width, height), daylight, hour, 0.0, 0.5);
        let snapshot = crate::diorama::tests::harbour_1082();
        let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(hour as u8));
        let camera = Camera::around(&stage, 0.0, stage.width / 2.0, height / 2.0);
        assert!((camera.fold - 1.0).abs() < 1e-4, "folded right up");
        // The back row's roofs, in the last row of the postcard.
        let roofs = stage.base - stage.building_h * 0.7;
        let top = camera.at(&stage, stage.width - 1.0, roofs).1;
        let town = (height - top) / height;
        assert!(
            town >= 0.7,
            "the town fills {:.0}% of the window",
            town * 100.0
        );
        let image = draw(width, height, move || painted(frame.clone()));
        let paper = image::Rgba([0xf2, 0xee, 0xe6, 0xff]);
        let near = |a: &image::Rgba<u8>, b: &image::Rgba<u8>| {
            a.0.iter().zip(b.0).all(|(x, y)| x.abs_diff(y) <= 3)
        };
        let bare = image.pixels().filter(|pixel| near(pixel, &paper)).count();
        if bare * 1000 >= image.pixels().len() {
            let _ =
                image.save(std::env::temp_dir().join(format!("world-gpui-postcard-{hour}.png")));
        }
        assert!(bare * 1000 < image.pixels().len(), "{bare} bare pixels");
        let band = image.height() * 15 / 100;
        let sky = image
            .enumerate_pixels()
            .filter(|(_, y, _)| *y < band)
            .map(|(_, _, pixel)| pixel.0)
            .fold([0u64; 3], |sum, p| {
                [
                    sum[0] + p[0] as u64,
                    sum[1] + p[1] as u64,
                    sum[2] + p[2] as u64,
                ]
            });
        let n = u64::from(image.width()) * u64::from(band);
        let (r, g, b) = (sky[0] / n, sky[1] / n, sky[2] / n);
        if daylight == Daylight::Night {
            assert!(r + g + b < 3 * 110 && b > r, "a night sky: {r} {g} {b}");
        } else {
            assert!(b > r && b > 150, "a day sky: {r} {g} {b}");
        }
    }
}

/// A lived place of Pocket Universe, as its Pack sent it over the wire.
fn place(name: &str) -> ProjectionSnapshot {
    let json = match name {
        "ares" => include_str!("../tests/fixtures/place-ares.json"),
        "maple" => include_str!("../tests/fixtures/place-maple.json"),
        _ => include_str!("../tests/fixtures/place-icebridge.json"),
    };
    let wire: world_pack_protocol::ProjectionSnapshotWire =
        serde_json::from_str(json).expect("a place's snapshot");
    ProjectionSnapshot::try_from(wire).expect("a snapshot")
}

/// Each place of Pocket Universe in its own clothes, at noon: Ares's domes
/// and hab modules on regolith under a butterscotch sky, Maple Street's
/// storefronts, cars and wires, Icebridge's snow nests, ice shelf and sea
/// ice. With `WORLD_GPUI_PLACE_SHOTS=<dir>` each is also drawn large there,
/// for looking at.
#[test]
fn each_pocket_universe_place_matches_its_golden_picture() {
    let shots = std::env::var("WORLD_GPUI_PLACE_SHOTS").ok();
    for name in ["ares", "maple", "icebridge"] {
        let snapshot = place(name);
        assert!(snapshot.canvas.setting.is_some(), "{name} says what it is");
        let frame = diorama_frame(&snapshot, 480.0, 300.0, Daylight::Day, 12.5);
        let image = draw(480.0, 300.0, move || painted(frame.clone()));
        matches_golden(&format!("place-{name}"), &image);
        if let Some(dir) = &shots {
            for (label, daylight, hour) in [
                ("noon", Daylight::Day, 12.5),
                ("dusk", Daylight::Dusk, 19.5),
                ("night", Daylight::Night, 23.0),
            ] {
                let snapshot = snapshot.clone();
                let stage =
                    diorama::stage_at(&snapshot, 1100.0, 700.0, diorama::Clock::at(hour as u8));
                let living =
                    diorama::living(&stage, &snapshot, 0.0, daylight, &Default::default(), None);
                let camera = Camera::around(&stage, 1.3, stage.width * 0.3, 700.0 * 0.55);
                let frame = diorama::frame(
                    &snapshot,
                    &stage,
                    &living,
                    camera,
                    0.0,
                    daylight,
                    &Glows::new(),
                    1.0,
                )
                .at_hour(hour);
                let image = draw(1100.0, 700.0, move || painted(frame.clone()));
                image.save(format!("{dir}/{name}-{label}.png")).unwrap();
            }
        }
    }
}

/// A World that stays as it is, whatever is asked of it.
struct Fixed(ProjectionSnapshot);

impl crate::ProjectionController for Fixed {
    fn snapshot(&self) -> ProjectionSnapshot {
        self.0.clone()
    }

    fn handle(
        &mut self,
        _: world_projection::ProjectionIntent,
    ) -> Result<ProjectionSnapshot, String> {
        Ok(self.0.clone())
    }
}

/// The whole World window over `snapshot`, `width` by `height`, drawn
/// through GPUI and the reference rasteriser, with its controls or, as a
/// photo is taken, without.
fn world_window(
    snapshot: &ProjectionSnapshot,
    (width, height): (f32, f32),
    controls: bool,
) -> RgbaImage {
    let mut cx =
        HeadlessAppContext::with_platform(Arc::new(NoopTextSystem::new()), Arc::new(()), || {
            Some(Box::new(Raster(Arc::default())) as Box<dyn PlatformHeadlessRenderer>)
        });
    let snapshot = snapshot.clone();
    let window = cx
        .open_window(size(px(width), px(height)), move |_, cx: &mut App| {
            cx.new(|_| {
                let mut view = crate::window::ProjectionView::controlled(Fixed(snapshot.clone()))
                    .with_strip(|_, _| {});
                view.looking.opening = None;
                view.looking.photographing = !controls;
                // With its controls, the drawer is open: what is read.
                view.looking.drawer = controls;
                view
            })
        })
        .expect("a window");
    cx.run_until_parked();
    cx.capture_screenshot(window.into()).expect("a picture")
}

/// How bright a pixel is, as the eye has it (0 to 255).
fn luma(pixel: &image::Rgba<u8>) -> f32 {
    let [r, g, b, _] = pixel.0;
    0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32
}

/// The v0.25 bar for the night, as v0.26 keeps it: after dark nothing the
/// World window lays over its scene (the gauges and handles, the zoom
/// control, the pills) is brighter than the scene's own sky. What the
/// player reads (the drawer here; the cards and pages too) keeps the
/// window's own light instead: paper under a lamp, warm and dimmer than by
/// day, its words as dark as ever. The window is drawn with its controls
/// and without (as a photo is taken), and with its reading surfaces lit and
/// dark as the night; whatever differs between the first two is the
/// interface, and whatever differs between the last two is what is read.
/// The words are not drawn here: they are dark on the lamp-lit paper and
/// light on the night's pills.
#[test]
fn at_night_nothing_over_the_scene_is_brighter_than_its_sky() {
    crate::scene::pin_hour(Some(23));
    let snapshot = crate::diorama::tests::harbour_1082();
    let (width, height) = (1100.0, 760.0);
    let with = world_window(&snapshot, (width, height), true);
    world_theme::set_reading_light(false);
    let all_dark = world_window(&snapshot, (width, height), true);
    world_theme::set_reading_light(true);
    let without = world_window(&snapshot, (width, height), false);
    crate::scene::pin_hour(None);
    world_theme::set_dark(false);
    // The sky: the top of the scene, under the handles' row and over the
    // hills. Its brightest, but for the few brightest points in it.
    let mut sky = without
        .enumerate_pixels()
        .filter(|(_, y, _)| (80..(height as u32) * 30 / 100).contains(y))
        .map(|(_, _, pixel)| luma(pixel))
        .collect::<Vec<_>>();
    sky.sort_by(f32::total_cmp);
    let brightest = sky[sky.len() * 995 / 1000];
    let differs = |a: &image::Rgba<u8>, b: &image::Rgba<u8>| {
        a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 8)
    };
    let (mut interface, mut read) = (Vec::new(), Vec::new());
    for ((pixel, dark), bare) in with.pixels().zip(all_dark.pixels()).zip(without.pixels()) {
        if differs(pixel, dark) {
            read.push(*pixel);
        } else if differs(pixel, bare) {
            interface.push(luma(pixel));
        }
    }
    assert!(
        interface.len() > 5_000,
        "the interface is drawn: {} pixels",
        interface.len()
    );
    let glaring = interface
        .iter()
        .filter(|luma| **luma > brightest + 2.0)
        .count();
    if glaring * 100 > interface.len() {
        let _ = with.save(std::env::temp_dir().join("world-gpui-night-interface.png"));
        let _ = without.save(std::env::temp_dir().join("world-gpui-night-scene.png"));
    }
    assert!(
        glaring * 100 <= interface.len(),
        "{glaring} of {} interface pixels are brighter than the sky's brightest ({brightest:.0})",
        interface.len()
    );
    // The drawer is read on lamp-lit paper: warm, and dimmer than the
    // day's white.
    // `WORLD_GPUI_NIGHT_SHOT=<file>` keeps the window with its drawer, for
    // looking at.
    if let Ok(path) = std::env::var("WORLD_GPUI_NIGHT_SHOT") {
        let _ = with.save(path);
    }
    if read.len() <= 20_000 {
        let _ = with.save(std::env::temp_dir().join("world-gpui-night-lit.png"));
        let _ = all_dark.save(std::env::temp_dir().join("world-gpui-night-dark.png"));
    }
    assert!(
        read.len() > 20_000,
        "the drawer keeps its light: {}",
        read.len()
    );
    let mut lumas = read.iter().map(luma).collect::<Vec<_>>();
    lumas.sort_by(f32::total_cmp);
    let paper = read
        .iter()
        .find(|pixel| luma(pixel) >= lumas[lumas.len() / 2])
        .expect("paper");
    let [r, g, b, _] = paper.0;
    assert!(r > g && g > b, "warm paper: {r} {g} {b}");
    assert!(
        (200.0..250.0).contains(&lumas[lumas.len() / 2]),
        "paper under a lamp, not the day's white: {}",
        lumas[lumas.len() / 2]
    );
}

/// The three value bands of a picture of the whole place (the art bible's
/// §4): the sky's, the land's and the near ground's or water's lightness,
/// each its median, and the land's lightest (but for its brightest
/// 0.5%). Rows by the stage's own lines: the horizon at 40%, the water's
/// edge at 80%; and the sky's light (its lightest tenth, down by the
/// horizon).
fn value_bands(image: &RgbaImage) -> (f32, f32, f32, f32, f32) {
    let h = image.height();
    let band = |from: f32, to: f32| {
        let mut values = image
            .enumerate_pixels()
            .filter(|(_, y, _)| (*y as f32) >= h as f32 * from && (*y as f32) < h as f32 * to)
            .map(|(_, _, pixel)| luma(pixel))
            .collect::<Vec<_>>();
        values.sort_by(f32::total_cmp);
        values
    };
    let (sky, land, water) = (band(0.0, 0.36), band(0.44, 0.76), band(0.86, 1.0));
    let median = |values: &[f32]| values[values.len() / 2];
    (
        median(&sky),
        median(&land),
        median(&water),
        land[land.len() * 995 / 1000],
        sky[sky.len() * 9 / 10],
    )
}

/// By day each place reads in three value bands: the sky lightest, the
/// land in the middle, the water (or near ground) darkest; and nothing on
/// the land is lighter than the sky (the art bible's §4).
#[test]
fn every_place_reads_in_three_value_bands_by_day() {
    let places = [
        ("harbour", crate::diorama::tests::harbour_1082()),
        ("ares", place("ares")),
        ("maple", place("maple")),
        ("icebridge", place("icebridge")),
    ];
    let mut wrong = Vec::new();
    for (name, snapshot) in places {
        let frame = diorama_frame(&snapshot, 480.0, 300.0, Daylight::Day, 13.0);
        let image = draw(480.0, 300.0, move || painted(frame.clone()));
        let (sky, land, water, lightest, sky_light) = value_bands(&image);
        if !(sky > land + 8.0 && land > water + 8.0) {
            wrong.push(format!(
                "{name}: sky {sky:.0}, land {land:.0}, water {water:.0}"
            ));
        }
        if lightest > sky_light + 2.0 {
            wrong.push(format!(
                "{name}: the land's lightest ({lightest:.0}) is lighter than the sky ({sky_light:.0})"
            ));
        }
        if !wrong.is_empty() {
            let _ = image.save(std::env::temp_dir().join(format!("world-gpui-bands-{name}.png")));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

// The ground as one wash (v0.27, owner A1).

/// The v0.27 bar for the ground: where the rows of a folded postcard meet,
/// the nearer row's field runs on from the row behind with no seam. For
/// the three-year harbour and each Pocket Universe place, zoomed right
/// out, each line where a nearer row's ground begins is no harsher a step
/// than the ground a little above and below it.
#[test]
fn the_ground_has_no_seam_where_the_postcard_rows_meet() {
    let (width, height) = (550.0, 424.0);
    let snapshots = [
        ("harbour", crate::diorama::tests::harbour_1082()),
        ("ares", place("ares")),
        ("maple", place("maple")),
        ("icebridge", place("icebridge")),
    ];
    for (name, snapshot) in snapshots {
        let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(12));
        let camera = Camera::around(&stage, 0.0, stage.width / 2.0, height / 2.0);
        let fold = diorama::Fold::of_stage(&stage);
        if fold.rows < 2 {
            continue;
        }
        let frame = snapshot_frame(&snapshot, (width, height), Daylight::Day, 12.0, 0.0, 0.5);
        let image = draw(width, height, move || painted(frame.clone()));
        // The step in light between two rows of pixels, at its median
        // across the window: a seam is a step all the way along.
        let step = |y: u32| {
            let mut steps = (0..image.width())
                .map(|x| (luma(image.get_pixel(x, y + 1)) - luma(image.get_pixel(x, y))).abs())
                .collect::<Vec<_>>();
            steps.sort_by(f32::total_cmp);
            steps[steps.len() / 2]
        };
        let field = stage.horizon + (stage.base - stage.horizon) * 0.3;
        for row in 0..fold.rows - 1 {
            let seen = camera.row(&fold, stage.view_w, stage.height, row);
            let y = seen.at(&stage, 0.0, field).1.round() as i64;
            let at = |dy: i64| (y + dy).clamp(1, image.height() as i64 - 2) as u32;
            let seam = (-6..=6).map(|dy| step(at(dy))).fold(0.0, f32::max);
            let around = [-16, -12, 12, 16]
                .map(|dy| step(at(dy)))
                .into_iter()
                .fold(0.0, f32::max);
            assert!(
                seam <= around + 2.5,
                "{name}: a seam where row {row} meets the row behind (y {y}): \
                 a step of {seam:.1} against {around:.1} around it"
            );
        }
    }
}

/// v0.27's year-three zoom-out: the nearer rows of the folded postcard
/// stayed a flat slab for good, because the still things' boil sent the
/// painter after each new drawing before the last one was with the
/// display, and those rows' layers never settled. Zoomed right out on a
/// three-year World, with the boil running as it runs in the window, every
/// layer settles, and nothing the camera sees is left unpainted.
#[test]
fn a_zoomed_out_town_settles_while_the_boil_runs() {
    use std::time::{Duration, Instant};
    crate::painter::paint_elsewhere(true);
    let snapshot = crate::diorama::tests::three_years();
    let (width, height) = (1100.0_f32, 848.0_f32);
    let make = |seconds: f32| {
        let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(12));
        let living = diorama::living(
            &stage,
            &snapshot,
            0.0,
            Daylight::Day,
            &Default::default(),
            None,
        );
        let least = Camera::least(&stage);
        assert!(least < 0.8, "a three-year World folds (least zoom {least})");
        let camera = Camera::around(&stage, least, stage.width / 2.0, height / 2.0);
        diorama::frame(
            &snapshot,
            &stage,
            &living,
            camera,
            0.0,
            Daylight::Day,
            &Glows::new(),
            1.0,
        )
        .at_hour(12.0)
        .at_seconds(seconds)
    };
    let shared = std::rc::Rc::new(std::cell::RefCell::new(make(0.0)));
    let strip = std::rc::Rc::new(std::cell::RefCell::new(None));
    let mut cx =
        HeadlessAppContext::with_platform(Arc::new(NoopTextSystem::new()), Arc::new(()), || {
            Some(Box::new(Raster(Arc::default())) as Box<dyn PlatformHeadlessRenderer>)
        });
    let view = shared.clone();
    let window = cx
        .open_window(size(px(width), px(height)), move |_, cx: &mut App| {
            cx.new(|_| LiveScene(view.clone(), strip.clone()))
        })
        .expect("a window");
    let started = Instant::now();
    let mut boils = std::collections::BTreeSet::new();
    let mut settled = false;
    while started.elapsed() < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(8));
        let frame = make(started.elapsed().as_secs_f32());
        boils.insert(frame.boil());
        *shared.borrow_mut() = frame;
        let (done, gaps) = cx
            .update_window(window.into(), |_, window, _| {
                window.refresh();
                (diorama::settled(window), diorama::unpainted(window))
            })
            .expect("a window");
        cx.run_until_parked();
        if done && gaps == 0 && boils.len() > 1 {
            settled = true;
            break;
        }
    }
    crate::painter::paint_elsewhere(false);
    assert!(boils.len() > 1, "the boil ran");
    assert!(settled, "every layer of the zoomed-out town settles");
}

/// The first Pack's lived town, as it was sent over the wire: on day 1,
/// 163, 358 or 1082 (the fixtures under `tests/fixtures/`).
fn town_on(day: u32) -> ProjectionSnapshot {
    let day = if [1, 163, 358].contains(&day) {
        day
    } else {
        1082
    };
    let path = format!(
        "{}/tests/fixtures/{TOWN}-day-{day}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(&path).expect("a lived town's fixture");
    let wire: world_pack_protocol::ProjectionSnapshotWire =
        serde_json::from_str(&json).expect("a town's snapshot");
    ProjectionSnapshot::try_from(wire).expect("a snapshot")
}

/// What the first Pack's fixtures are named for.
const TOWN: &str = "harbour";

/// What the tower on the point is drawn as.
const BEACON: &str = "lighthouse";

/// The frame of `snapshot` seen from `camera` (or the whole first window),
/// at `hour` with its part of the day's light.
fn frame_at(
    snapshot: &ProjectionSnapshot,
    (width, height): (f32, f32),
    hour: f32,
    camera: Option<&dyn Fn(&diorama::Stage) -> Camera>,
) -> diorama::Frame {
    let daylight = crate::scene::daylight_at(hour as u32);
    let stage = diorama::stage_at(snapshot, width, height, diorama::Clock::at(hour as u8));
    let living = diorama::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
    let camera = camera.map_or_else(|| Camera::whole(&stage), |at| at(&stage));
    diorama::frame(
        snapshot,
        &stage,
        &living,
        camera,
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    )
    .at_hour(hour)
}

/// Where the tower on the point stands along a town's stage, in stage
/// pixels.
fn beacon_x(snapshot: &ProjectionSnapshot, size: (f32, f32)) -> f32 {
    let frame = frame_at(snapshot, size, 13.0, None);
    frame
        .buildings_along()
        .into_iter()
        .find(|(index, ..)| {
            snapshot.canvas.items[*index]
                .art
                .as_deref()
                .is_some_and(|art| art.contains(BEACON))
        })
        .map_or(size.0 * 0.86, |(_, x, _)| x)
}

/// Where a picture darkens another of the same size, 0 to 255 of luma.
fn darkening(with: &RgbaImage, without: &RgbaImage) -> Vec<f32> {
    let luma =
        |p: &image::Rgba<u8>| 0.3 * p.0[0] as f32 + 0.59 * p.0[1] as f32 + 0.11 * p.0[2] as f32;
    without
        .pixels()
        .zip(with.pixels())
        .map(|(a, b)| (luma(a) - luma(b)).max(0.0))
        .collect()
}

/// The longest straight hard edge in a darkening map: a run of pixels
/// along a row (or down a column) where the dark steps by more than
/// `step` from one pixel to the next, in square pixels of length.
fn longest_hard_edge(dark: &[f32], width: usize, height: usize, step: f32) -> usize {
    let at = |x: usize, y: usize| dark[y * width + x];
    let mut longest = 0;
    // Edges along rows: a step down a column, the same for a run of x.
    for y in 0..height.saturating_sub(1) {
        let mut run = 0;
        for x in 0..width {
            if (at(x, y + 1) - at(x, y)).abs() > step {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
    }
    for x in 0..width.saturating_sub(1) {
        let mut run = 0;
        for y in 0..height {
            if (at(x + 1, y) - at(x, y)).abs() > step {
                run += 1;
                longest = longest.max(run);
            } else {
                run = 0;
            }
        }
    }
    longest
}

/// The v0.29 bar for shadows: no straight dark edge longer than 8 px under
/// a structure. Each frame is drawn with the buildings' shadows and
/// without, so what they lay is known exactly; an edge detector then looks
/// for a hard step in it (a soft ellipse falls off over many pixels; v0.28's
/// shadow cut square where the building's picture ended stepped at once).
/// Over the lived town on day 1 and in year three, at noon, in the low sun
/// of late afternoon, and at dusk, and zoomed on the tower on the point.
#[test]
fn no_shadow_has_a_straight_hard_edge() {
    // The detector itself: a hard-edged box is found, a soft ellipse not.
    let (w, h) = (120, 80);
    let boxed = (0..w * h)
        .map(|at| {
            let (x, y) = (at % w, at / w);
            if (30..90).contains(&x) && (20..50).contains(&y) {
                60.0
            } else {
                0.0
            }
        })
        .collect::<Vec<_>>();
    assert!(
        longest_hard_edge(&boxed, w, h, 20.0) > 8,
        "a box's edge is found"
    );
    let (width, height) = (1100.0_f32, 848.0_f32);
    let at = beacon_x(&town_on(1082), (width, height));
    let point = move |stage: &diorama::Stage| Camera::around(stage, 1.6, at, height * 0.62);
    for (day, camera) in [
        (1, None),
        (1082, None),
        (1082, Some(&point as &dyn Fn(&diorama::Stage) -> Camera)),
    ] {
        let snapshot = town_on(day);
        for hour in [13.0, 16.5, 19.0] {
            let frame = frame_at(&snapshot, (width, height), hour, camera);
            let with = {
                let frame = frame.clone();
                draw(width, height, move || painted(frame.clone()))
            };
            diorama::leave_out_shadows(true);
            let without = {
                let frame = frame.clone();
                draw(width, height, move || painted(frame.clone()))
            };
            diorama::leave_out_shadows(false);
            let dark = darkening(&with, &without);
            let most = dark.iter().copied().fold(0.0_f32, f32::max);
            let edge = longest_hard_edge(&dark, width as usize, height as usize, 20.0);
            assert!(
                most > 5.0,
                "day {day} at {hour}: the shadows lay something ({most})"
            );
            assert!(
                edge <= 8,
                "day {day} at {hour}: a straight hard shadow edge {edge} px long"
            );
        }
    }
}

/// The hue (degrees) and saturation (0 to 1) of a colour.
fn hue_of([r, g, b]: [f32; 3]) -> (f32, f32) {
    let (most, least) = (r.max(g).max(b), r.min(g).min(b));
    let chroma = most - least;
    if chroma <= 1e-6 {
        return (0.0, 0.0);
    }
    let hue = if most == r {
        60.0 * ((g - b) / chroma).rem_euclid(6.0)
    } else if most == g {
        60.0 * ((b - r) / chroma + 2.0)
    } else {
        60.0 * ((r - g) / chroma + 4.0)
    };
    (hue, chroma / most)
}

/// The mean colour of rows `from` to `to` (shares of the height) of a
/// picture, 0 to 1 each channel.
fn mean_colour(image: &RgbaImage, from: f32, to: f32) -> [f32; 3] {
    let (w, h) = image.dimensions();
    let (y0, y1) = ((h as f32 * from) as u32, (h as f32 * to) as u32);
    let mut sum = [0.0_f64; 3];
    let mut count = 0.0_f64;
    for y in y0..y1 {
        for x in 0..w {
            let p = image.get_pixel(x, y).0;
            for c in 0..3 {
                sum[c] += p[c] as f64 / 255.0;
            }
            count += 1.0;
        }
    }
    sum.map(|s| (s / count.max(1.0)) as f32)
}

/// The v0.29 bar for dusk: gold, not grey-olive fog. Over the town on
/// day 1, in its first year and in year three, clear and under cloud, the
/// light low over the place (the sky's lower half and the land, down to the
/// water) is within a gold hue band and warm enough to read as gold; and
/// every lamp on the spine is lit (held in `diorama::tests`).
#[test]
fn dusk_is_gold() {
    let (width, height) = (550.0_f32, 424.0_f32);
    for day in [1, 163, 1082] {
        for weather in [
            world_projection::Weather::Clear,
            world_projection::Weather::Cloudy,
        ] {
            for hour in [18.5, 19.0, 19.5] {
                let mut snapshot = town_on(day);
                snapshot.weather = weather;
                let frame = frame_at(&snapshot, (width, height), hour, None);
                let image = draw(width, height, move || painted(frame.clone()));
                let (hue, saturation) = hue_of(mean_colour(&image, 0.2, 0.72));
                let (sky_hue, sky_saturation) = hue_of(mean_colour(&image, 0.0, 0.3));
                eprintln!(
                    "day {day} {weather:?} at {hour}: hue {hue:.0} saturation {saturation:.2}; \
                     sky {sky_hue:.0} {sky_saturation:.2}"
                );
                assert!(
                    (DUSK_HUE.0..=DUSK_HUE.1).contains(&hue) && saturation >= DUSK_SATURATION,
                    "day {day}, {weather:?}, at {hour}: dusk is hue {hue:.0} saturation \
                     {saturation:.2}, not gold"
                );
                assert!(
                    (12.0..=DUSK_HUE.1).contains(&sky_hue) && sky_saturation >= 0.12,
                    "day {day}, {weather:?}, at {hour}: the dusk sky is hue {sky_hue:.0} \
                     saturation {sky_saturation:.2}, grey or violet, not gold"
                );
            }
        }
    }
}

/// The gold band dusk is held to: from orange to gold, in degrees, and how
/// saturated at least. Calibrated on the v0.27 key art's dusk (hue 41,
/// saturation 0.40 over the same rows) against v0.28's grey-olive dusk in
/// the real window (hue 42, saturation 0.26-0.28): the hue alone cannot
/// tell them apart, the saturation does.
const DUSK_HUE: (f32, f32) = (22.0, 48.0);
const DUSK_SATURATION: f32 = 0.36;

/// The v0.29 bars for people, on the lived town's first screen, by day
/// and at dusk, on day 1, in its first year and in year three: never more
/// than four standing in one row, and no two overlapping by more than a
/// fifth; and at dusk the spine is not empty (two groups out by the water).
#[test]
fn people_stand_in_groups_never_rows_or_blobs() {
    let (width, height) = (1100.0_f32, 848.0_f32);
    let mut failures = Vec::new();
    for day in [1, 163, 358, 1082] {
        let snapshot = town_on(day);
        for hour in [10.0, 13.0, 16.0, 19.0] {
            let frame = frame_at(&snapshot, (width, height), hour, None);
            let people = frame
                .people
                .iter()
                .filter(|person| person.x > 0.0 && person.x < width)
                .map(|person| (person.x, person.y, person.height))
                .collect::<Vec<_>>();
            let (row, overlap) = diorama::rows_and_overlap(&people);
            if row > 4 {
                failures.push(format!(
                    "day {day} at {hour}: {row} people stand in one row"
                ));
            }
            if overlap > 0.2 {
                failures.push(format!(
                    "day {day} at {hour}: two people overlap by {:.0}%",
                    overlap * 100.0
                ));
            }
            let least = if day == 1 { 3 } else { 4 };
            if hour == 19.0 && people.len() < least {
                failures.push(format!(
                    "day {day} at dusk: only {} people out on the first screen",
                    people.len()
                ));
            }
            if !failures.is_empty() && std::env::var_os("WORLD_GPUI_PEOPLE").is_some() {
                eprintln!("day {day} at {hour}: {people:?}");
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// For looking, not a bar: with `WORLD_GPUI_LOOK_DIR=<dir>`, the town
/// on day 1, in its first year and in year three at noon, in the low sun
/// and at dusk, whole and zoomed on the tower on the point, drawn large.
#[test]
#[ignore = "pictures to look at: WORLD_GPUI_LOOK_DIR=<dir> cargo test -p world-gpui -- --ignored look_at"]
fn look_at_the_town() {
    let Some(dir) = std::env::var_os("WORLD_GPUI_LOOK_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (width, height) = (1100.0_f32, 848.0_f32);
    for day in [1, 163, 1082] {
        let snapshot = town_on(day);
        let at = beacon_x(&snapshot, (width, height));
        let point = move |stage: &diorama::Stage| Camera::around(stage, 1.6, at, height * 0.62);
        for hour in [13.0, 16.5, 19.0, 19.5] {
            for (name, camera) in [
                ("whole", None),
                ("point", Some(&point as &dyn Fn(&diorama::Stage) -> Camera)),
            ] {
                let frame = frame_at(&snapshot, (width, height), hour, camera);
                let image = draw(width, height, move || painted(frame.clone()));
                image
                    .save(dir.join(format!("town-{day}-{hour}-{name}.png")))
                    .unwrap();
            }
        }
    }
}
