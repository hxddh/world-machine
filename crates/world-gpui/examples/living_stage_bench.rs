//! The living stage spike (v0.30 S0), measured without a GPU: the bake,
//! the frame plan through GPUI's own drawing (its test platform, at twice
//! the pixels like a Retina display), and pictures for the art director.
//!
//! ```bash
//! # What the bake costs, for the first day and the three-year town.
//! cargo run --release -p world-gpui --example living_stage_bench -- bake
//! # Frame plan times through GPUI, headless (CI): 300 walkers in view.
//! cargo run --release -p world-gpui --example living_stage_bench -- headless --seconds 10
//! # Noon, dusk and night, and each beside the CPU painter's frame.
//! cargo run --release -p world-gpui --example living_stage_bench -- shots <out-dir>
//! # The living stage as the World window draws it (S1a): looks baked
//! # lazily as the camera pans and zooms, residents from their routines;
//! # frame plan and pixel memory, for a display of scale `--dpr`.
//! cargo run --release -p world-gpui --example living_stage_bench -- window --dpr 2 --seconds 40
//! ```
//!
//! Options: `--world <snapshot.json>` (repeatable; default the fixtures
//! for day 1 and day 1,082 under `tests/fixtures/`), `--walkers N` (in a
//! window's width, default 300), `--seconds S`, `--levels 1,2,3` (device
//! pixels a stage pixel), `--size 1280x800`, and for `window`, `--hour H`
//! (default 12; `--hour day` runs through a day every forty seconds, as
//! CI's windowed run does).
//!
//! The windowed benchmark, on a real GPU, is `apps/living-stage-bench`.

#[path = "../src/offscreen.rs"]
mod offscreen;

use gpui::{prelude::*, px, size, App, HeadlessAppContext, NoopTextSystem};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use world_gpui::diorama::living_stage::{self as stage, Baked, Light, Walkers};
use world_projection::ProjectionSnapshot;

struct Options {
    mode: String,
    out: PathBuf,
    worlds: Vec<PathBuf>,
    walkers: usize,
    seconds: f32,
    levels: Vec<f32>,
    size: (f32, f32),
    dpr: f32,
    /// The hour held (none: through a day every forty seconds, as CI's
    /// windowed run has it).
    hour: Option<f32>,
}

fn options() -> Options {
    let mut args = std::env::args().skip(1);
    let mut options = Options {
        mode: "headless".into(),
        out: PathBuf::from("living-stage"),
        worlds: Vec::new(),
        walkers: 300,
        seconds: 10.0,
        levels: vec![1.0, 2.0, 3.0],
        size: (1280.0, 800.0),
        dpr: 1.0,
        hour: Some(12.0),
    };
    let mut positional = 0;
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_default();
        match arg.as_str() {
            "--world" => options.worlds.push(PathBuf::from(value())),
            "--walkers" => options.walkers = value().parse().unwrap_or(300),
            "--seconds" => options.seconds = value().parse().unwrap_or(10.0),
            "--dpr" => options.dpr = value().parse().unwrap_or(1.0),
            "--hour" => options.hour = value().parse().ok(),
            "--levels" => {
                options.levels = value()
                    .split(',')
                    .filter_map(|level| level.trim().parse().ok())
                    .collect()
            }
            "--size" => {
                let value = value();
                if let Some((w, h)) = value.split_once('x') {
                    options.size = (w.parse().unwrap_or(1280.0), h.parse().unwrap_or(800.0));
                }
            }
            other if positional == 0 => {
                options.mode = other.into();
                positional += 1;
            }
            other => options.out = PathBuf::from(other),
        }
    }
    if options.worlds.is_empty() {
        options.worlds = default_worlds();
    }
    options
}

/// The scene's own fixtures: the first day and the three-year town.
fn default_worlds() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut found = Vec::new();
    for suffix in ["-day-1.json", "-day-1082.json"] {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            let mut names = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.ends_with(suffix))
                })
                .collect::<Vec<_>>();
            names.sort();
            found.extend(names.into_iter().take(1));
        }
    }
    found
}

fn load(path: &Path) -> Result<ProjectionSnapshot, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string(path)?;
    let wire: world_pack_protocol::ProjectionSnapshotWire = serde_json::from_str(&json)?;
    let mut snapshot = ProjectionSnapshot::try_from(wire).map_err(|error| format!("{error:?}"))?;
    snapshot.weather = world_projection::Weather::Clear;
    Ok(snapshot)
}

fn name_of(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("world")
        .to_string()
}

fn print_bake(name: &str, baked: &Baked) {
    println!(
        "bake {name}: stage {:.0}x{:.0} pt, {} residents, {} chimneys",
        baked.width,
        baked.view.1,
        baked.residents.len(),
        baked.chimneys.len()
    );
    for stat in &baked.stats {
        println!(
            "  {:<5} at {:.1}x: {:>6.0} ms, {:>5} sprites, {:>7.1} MB",
            stat.light.name(),
            stat.scale,
            stat.took.as_secs_f64() * 1000.0,
            stat.sprites,
            stat.bytes as f64 / 1e6
        );
    }
    if let Some(look) = baked.looks().last() {
        let parts = look
            .breakdown()
            .into_iter()
            .map(|(name, count, bytes)| format!("{name} {count}/{:.1} MB", bytes as f64 / 1e6))
            .collect::<Vec<_>>();
        println!(
            "  {} at {:.1}x by layer (sprites/MB): {}",
            look.light.name(),
            look.scale,
            parts.join(", ")
        );
    }
    println!(
        "  total: {:.0} ms on {} threads, {} sprites, {:.1} MB of pixels",
        baked.took().as_secs_f64() * 1000.0,
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        baked.count(),
        baked.bytes() as f64 / 1e6
    );
}

/// A headless renderer that keeps the images GPUI hands it but draws
/// nothing: the frame is planned, never rasterised.
struct Plan(Arc<offscreen::Atlas>);

impl gpui::PlatformHeadlessRenderer for Plan {
    fn render_scene_to_image(
        &mut self,
        _: &gpui::Scene,
        _: gpui::Size<gpui::DevicePixels>,
    ) -> gpui::Result<image::RgbaImage> {
        Ok(image::RgbaImage::new(1, 1))
    }

    fn render_scene(
        &mut self,
        _: &gpui::Scene,
        _: gpui::Size<gpui::DevicePixels>,
    ) -> gpui::Result<()> {
        Ok(())
    }

    fn sprite_atlas(&self) -> Arc<dyn gpui::PlatformAtlas> {
        self.0.clone()
    }
}

fn headless(options: &Options) -> Result<(), Box<dyn std::error::Error>> {
    for path in &options.worlds {
        let snapshot = load(path)?;
        let started = Instant::now();
        let baked = Arc::new(stage::bake(
            &snapshot,
            options.size,
            &options.levels,
            &Light::ALL,
        ));
        let name = name_of(path);
        print_bake(&name, &baked);
        println!(
            "  (bake wall time {:.0} ms)",
            started.elapsed().as_secs_f64() * 1000.0
        );
        let walkers = Arc::new(Walkers::spread(&baked, options.walkers, 7));
        let stats = Rc::new(RefCell::new(stage::Stats::default()));
        let time = Rc::new(Cell::new(0.0_f32));
        let mut cx = HeadlessAppContext::with_platform(
            Arc::new(NoopTextSystem::new()),
            Arc::new(()),
            || Some(Box::new(Plan(Arc::default())) as Box<dyn gpui::PlatformHeadlessRenderer>),
        );
        let view = {
            let (baked, walkers, stats, time) =
                (baked.clone(), walkers.clone(), stats.clone(), time.clone());
            cx.open_window(
                size(px(options.size.0), px(options.size.1)),
                move |_, cx: &mut App| {
                    cx.new(|_| stage::Bench {
                        baked,
                        walkers,
                        time: stage::Time::Set(time),
                        stats,
                        hour: None,
                        animate: false,
                        until: None,
                        done: Rc::new(Cell::new(false)),
                        warmed: 0,
                    })
                },
            )?
        };
        // The first frames hand every image to the atlas, unmeasured.
        let warming = Instant::now();
        for _ in 0..1000 {
            cx.update_window(view.into(), |_, window, _| window.refresh())?;
            cx.run_until_parked();
            if !stats.borrow().planned.is_empty() {
                break;
            }
        }
        println!(
            "  handed {} images to the atlas in {:.0} ms",
            baked.count(),
            warming.elapsed().as_secs_f64() * 1000.0
        );
        stats.replace(stage::Stats::default());
        let began = Instant::now();
        let mut cycles = Vec::new();
        let step = 1.0 / 120.0;
        let mut t = 0.0;
        while began.elapsed() < Duration::from_secs_f32(options.seconds) {
            t += step;
            time.set(t);
            let cycle = Instant::now();
            cx.update_window(view.into(), |_, window, _| window.refresh())?;
            cx.run_until_parked();
            cycles.push(cycle.elapsed().as_secs_f64() * 1000.0);
        }
        let stats = stats.borrow();
        println!(
            "headless {name} ({:.0} simulated s at 120 Hz): {}",
            t,
            stats.summary()
        );
        cycles.sort_by(f64::total_cmp);
        println!(
            "  whole GPUI draw cycle ms: p50 {:.3} p95 {:.3} max {:.3}",
            cycles[cycles.len() / 2],
            cycles[(cycles.len() - 1) * 95 / 100],
            cycles.last().copied().unwrap_or(0.0)
        );
    }
    Ok(())
}

/// The living stage as the World window draws it, through GPUI's own
/// drawing headless: every look baked lazily off the thread as the camera
/// pans along the place and zooms in and out, residents where their
/// routines put them, in real time for `--seconds`. Prints the frame plan
/// and the pixels the stage held: at rest (zoom 1), zoomed in, and at most.
fn window(options: &Options) -> Result<(), Box<dyn std::error::Error>> {
    use world_gpui::diorama::stage_view::{set_display_scale, StageBench};
    for path in &options.worlds {
        let snapshot = Arc::new(load(path)?);
        let name = name_of(path);
        set_display_scale(Some(options.dpr));
        let mut cx = HeadlessAppContext::with_platform(
            Arc::new(NoopTextSystem::new()),
            Arc::new(()),
            || Some(Box::new(Plan(Arc::default())) as Box<dyn gpui::PlatformHeadlessRenderer>),
        );
        let bench = Rc::new(RefCell::new(None));
        let view = {
            let (snapshot, bench) = (snapshot.clone(), bench.clone());
            let until = Duration::from_secs_f32(options.seconds);
            let hour = options.hour;
            cx.open_window(
                size(px(options.size.0), px(options.size.1)),
                move |_, cx: &mut App| {
                    let view = StageBench::new(
                        snapshot,
                        stage::Time::Real(Instant::now()),
                        hour,
                        Some(until),
                    );
                    *bench.borrow_mut() = Some((
                        view.stats.clone(),
                        view.done.clone(),
                        view.memory.clone(),
                        view.opened.clone(),
                        view.bakes.clone(),
                    ));
                    cx.new(|_| view)
                },
            )?
        };
        let (stats, done, memory, opened, bakes) = bench.borrow().clone().ok_or("no bench")?;
        let started = Instant::now();
        let mut opened_at = None;
        let (mut at_rest, mut zoomed, mut most) = (0_usize, 0_usize, 0_usize);
        let mut measured: Option<Instant> = None;
        while !done.get() && started.elapsed() < Duration::from_secs(600) {
            cx.update_window(view.into(), |_, window, _| window.refresh())?;
            cx.run_until_parked();
            let (bytes, peak) = memory.get();
            most = most.max(peak);
            if opened.get().is_some() {
                if opened_at.is_none() {
                    opened_at = Some(started.elapsed());
                    measured = Some(Instant::now());
                }
                // The camera's cycle (StageBench::camera): zoomed in through
                // the first half of every twenty seconds, at rest after.
                let seconds = measured.map_or(0.0, |at| at.elapsed().as_secs_f32());
                let phase = (seconds / 20.0).fract();
                if (0.7..1.0).contains(&phase) {
                    at_rest = at_rest.max(bytes);
                } else if (0.2..0.3).contains(&phase) {
                    zoomed = zoomed.max(bytes);
                }
            }
            // Straight on to the next frame, as a display that wants
            // one at once would ask: a sleep here oversleeps by tens of
            // milliseconds while a look bakes, and would be counted as the
            // window's own hitch.
            std::thread::yield_now();
        }
        let stats = stats.borrow();
        println!(
            "window {name} at display scale {} ({:.0} s): first shown after {:.0} ms | {}",
            options.dpr,
            options.seconds,
            opened_at.map_or(f64::NAN, |at| at.as_secs_f64() * 1000.0),
            stats.summary()
        );
        println!(
            "  pixel memory MB: at rest (zoom 1) {:.1} | zoomed in (2.2) {:.1} | most held {:.1}",
            at_rest as f64 / 1e6,
            zoomed as f64 / 1e6,
            most as f64 / 1e6,
        );
        println!("  {}", stats.bar());
        println!(
            "  {}",
            world_gpui::diorama::stage_view::bake_summary(&bakes.borrow())
        );
    }
    set_display_scale(None);
    Ok(())
}

/// Where along the stage a window sees the most residents.
fn busiest(baked: &Baked, zoom: f32) -> f32 {
    let view = baked.view.0 / zoom;
    let mut best = (0, baked.width / 2.0);
    let mut x = view / 2.0;
    while x < baked.width - view / 2.0 {
        let seen = baked
            .residents
            .iter()
            .filter(|r| (r.x - x).abs() < view / 2.0)
            .count();
        if seen > best.0 {
            best = (seen, x);
        }
        x += view / 8.0;
    }
    best.1
}

/// One frame of the living stage, rasterised as GPUI would draw it.
fn composed(baked: &Arc<Baked>, walkers: &Arc<Walkers>, shot: stage::Shot) -> image::RgbaImage {
    let (baked, walkers) = (baked.clone(), walkers.clone());
    let (w, h) = baked.view;
    offscreen::draw(w, h, move || {
        let plan = Rc::new(stage::plan(&baked, &walkers, &shot));
        stage::element(plan).into_any_element()
    })
}

/// The same frame as the CPU painter paints it.
fn painted(
    snapshot: &ProjectionSnapshot,
    size: (f32, f32),
    view: stage::View,
    hour: f32,
    light: Light,
) -> image::RgbaImage {
    use world_gpui::diorama::{self, Camera, Clock, Glows};
    use world_gpui::scene::Daylight;
    let daylight = match light {
        Light::Noon => Daylight::Day,
        Light::Dusk => Daylight::Dusk,
        Light::Night => Daylight::Night,
    };
    let stage = diorama::stage_at(snapshot, size.0, size.1, Clock::at(16));
    let camera = Camera::around(&stage, view.zoom, view.x, view.y);
    let living = diorama::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
    let frame = diorama::frame(
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
    .stilled(true);
    offscreen::draw(size.0, size.1, move || {
        let frame = frame.clone();
        gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window, _| diorama::paint(&frame, bounds, window),
        )
        .size_full()
        .into_any_element()
    })
}

fn beside(top: &image::RgbaImage, bottom: &image::RgbaImage) -> image::RgbaImage {
    let gap = 12;
    let mut sheet = image::RgbaImage::from_pixel(
        top.width().max(bottom.width()),
        top.height() + bottom.height() + gap,
        image::Rgba([0x20, 0x20, 0x24, 0xff]),
    );
    image::imageops::overlay(&mut sheet, top, 0, 0);
    image::imageops::overlay(&mut sheet, bottom, 0, (top.height() + gap) as i64);
    sheet
}

fn shots(options: &Options) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&options.out)?;
    for path in &options.worlds {
        let snapshot = load(path)?;
        let name = name_of(path);
        let baked = Arc::new(stage::bake(
            &snapshot,
            options.size,
            &options.levels,
            &Light::ALL,
        ));
        print_bake(&name, &baked);
        // A few walkers along the spine, so the town is out and about.
        let walkers = Arc::new(Walkers::spread(&baked, 10, 3));
        let nobody = Arc::new(Walkers::default());
        let (w, h) = baked.view;
        for (zoom, tag) in [(1.0_f32, ""), (1.5, "-close")] {
            let x = busiest(&baked, zoom);
            let view = stage::View {
                x,
                y: h - h / zoom / 2.0,
                zoom,
            }
            .kept(&baked);
            for light in Light::ALL {
                let shot = stage::Shot {
                    view,
                    seconds: 6.0,
                    hour: light.hour(),
                    dpr: 2.0,
                };
                let started = Instant::now();
                let image = composed(&baked, &walkers, shot);
                let file = options
                    .out
                    .join(format!("{name}{tag}-{}.png", light.name()));
                image.save(&file)?;
                println!(
                    "{} ({:.1} s)",
                    file.display(),
                    started.elapsed().as_secs_f32()
                );
                // Beside the painter: the same view, the same people
                // (nobody walking), the painter on top.
                let sprites = composed(&baked, &nobody, shot);
                let painter = painted(&snapshot, (w, h), view, light.hour(), light);
                let file = options.out.join(format!(
                    "{name}{tag}-{}-painter-vs-sprites.png",
                    light.name()
                ));
                beside(&painter, &sprites).save(&file)?;
                println!("{}", file.display());
            }
        }
        // Halfway from dusk to night: the two lights cross-fading.
        let view = stage::View {
            x: busiest(&baked, 1.0),
            y: h / 2.0,
            zoom: 1.0,
        }
        .kept(&baked);
        let shot = stage::Shot {
            view,
            seconds: 6.0,
            hour: 21.75,
            dpr: 2.0,
        };
        let file = options.out.join(format!("{name}-crossfade.png"));
        composed(&baked, &walkers, shot).save(&file)?;
        println!("{}", file.display());
    }
    Ok(())
}

fn bake_only(options: &Options) -> Result<(), Box<dyn std::error::Error>> {
    for path in &options.worlds {
        let snapshot = load(path)?;
        let started = Instant::now();
        let baked = stage::bake(&snapshot, options.size, &options.levels, &Light::ALL);
        print_bake(&name_of(path), &baked);
        println!(
            "  (wall time {:.0} ms)",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = options();
    if options.worlds.is_empty() {
        return Err("no World snapshot: pass --world <snapshot.json>".into());
    }
    match options.mode.as_str() {
        "bake" => bake_only(&options),
        "shots" => shots(&options),
        "headless" => headless(&options),
        "window" => window(&options),
        other => Err(format!("unknown mode {other}: bake, headless, window or shots").into()),
    }
}
