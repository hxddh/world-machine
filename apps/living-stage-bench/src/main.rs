//! The living stage benchmark (v0.30 S0): a whole place baked once into
//! sprites, composited by GPUI on the GPU every frame, with hundreds of
//! people walking, boats riding the swell and chimneys smoking, while the
//! camera pans and zooms and the light runs through a day.
//!
//! ```bash
//! cargo run --release -p living-stage-bench
//! cargo run --release -p living-stage-bench -- --seconds 30 --walkers 300
//! # On Linux (Xvfb, or a desktop):
//! cargo run --release -p living-stage-bench --features linux-window
//! ```
//!
//! It prints what the bake took, then after `--seconds` (default 20) a
//! line beginning `living-stage:` with each frame's plan time (from the
//! view starting to render to the last image painted: the CPU's share of
//! a frame, which must stay under 8 ms for 120 Hz), the frames a second
//! the display reached, and how many images each frame drew. The first
//! frames hand every image to the GPU unseen and are not measured.
//!
//! Options: `--world <snapshot.json>` (default the three-year fixture in
//! `crates/world-gpui/tests/fixtures/`), `--walkers N` (in a window's
//! width at zoom 1, default 300), `--seconds S`, `--levels 1,2,3`
//! (device pixels a stage pixel; default the display's scale times 1,
//! 1.5 and 2), `--size 1280x800`, `--hour H` (hold one hour instead of
//! running through a day).
//!
//! `--window` runs the living stage as the World window draws it (v0.30
//! S1a) instead: looks baked lazily off the thread as the camera pans
//! along the place and zooms in and out, residents where their routines
//! put them, and prints a `living-stage window:` line and the pixels the
//! stage held. `--scale 2` bakes for a display of twice the pixels (a
//! simulated Retina display on a 1x one).

#![forbid(unsafe_code)]

#[cfg(gui)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    window::run()
}

#[cfg(not(gui))]
fn main() {
    eprintln!(
        "living-stage-bench was built without a window: on Linux build it with \
         `--features linux-window`. For a headless run see \
         `cargo run --release -p world-gpui --example living_stage_bench -- headless`."
    );
}

#[cfg(gui)]
mod window {
    use gpui::{prelude::*, px, size, App, Bounds, WindowBounds, WindowOptions};
    use std::cell::{Cell, RefCell};
    use std::path::{Path, PathBuf};
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use world_gpui::diorama::living_stage::{self as stage, Light, Walkers};
    use world_projection::ProjectionSnapshot;

    struct Options {
        world: Option<PathBuf>,
        walkers: usize,
        seconds: f32,
        levels: Option<Vec<f32>>,
        size: (f32, f32),
        hour: Option<f32>,
        window: bool,
        scale: Option<f32>,
    }

    fn options() -> Options {
        let mut args = std::env::args().skip(1);
        let mut options = Options {
            world: None,
            walkers: 300,
            seconds: 20.0,
            levels: None,
            size: (1280.0, 800.0),
            hour: None,
            window: false,
            scale: None,
        };
        while let Some(arg) = args.next() {
            let mut value = || args.next().unwrap_or_default();
            match arg.as_str() {
                "--world" => options.world = Some(PathBuf::from(value())),
                "--walkers" => options.walkers = value().parse().unwrap_or(300),
                "--seconds" => options.seconds = value().parse().unwrap_or(20.0),
                "--hour" => options.hour = value().parse().ok(),
                "--window" => options.window = true,
                "--scale" => options.scale = value().parse().ok(),
                "--levels" => {
                    options.levels = Some(
                        value()
                            .split(',')
                            .filter_map(|level| level.trim().parse().ok())
                            .collect(),
                    )
                }
                "--size" => {
                    let value = value();
                    if let Some((w, h)) = value.split_once('x') {
                        options.size = (w.parse().unwrap_or(1280.0), h.parse().unwrap_or(800.0));
                    }
                }
                other => eprintln!("living-stage: ignoring {other}"),
            }
        }
        options
    }

    /// The three-year town among the scene's fixtures.
    fn default_world() -> Option<PathBuf> {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/world-gpui/tests/fixtures");
        let mut found = std::fs::read_dir(dir)
            .ok()?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with("-day-1082.json"))
            })
            .collect::<Vec<_>>();
        found.sort();
        found.into_iter().next()
    }

    fn load(path: &Path) -> Result<ProjectionSnapshot, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let wire: world_pack_protocol::ProjectionSnapshotWire = serde_json::from_str(&json)?;
        let mut snapshot =
            ProjectionSnapshot::try_from(wire).map_err(|error| format!("{error:?}"))?;
        snapshot.weather = world_projection::Weather::Clear;
        Ok(snapshot)
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let options = options();
        let path = options
            .world
            .clone()
            .or_else(default_world)
            .ok_or("no World snapshot: pass --world <snapshot.json>")?;
        let snapshot = load(&path)?;
        if options.window {
            return run_window(options, snapshot);
        }
        let application = gpui_platform::application();
        application.run(move |cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(options.size.0), px(options.size.1)), cx);
            let stats = Rc::new(RefCell::new(stage::Stats::default()));
            let done = Rc::new(Cell::new(false));
            let window = {
                let (stats, done) = (stats.clone(), done.clone());
                let snapshot = snapshot.clone();
                let levels = options.levels.clone();
                let (walkers, hour, seconds, view) =
                    (options.walkers, options.hour, options.seconds, options.size);
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        ..Default::default()
                    },
                    move |window, cx| {
                        let scale = window.scale_factor();
                        let levels =
                            levels.unwrap_or_else(|| vec![scale, scale * 1.5, scale * 2.0]);
                        let started = Instant::now();
                        let baked = Arc::new(stage::bake(&snapshot, view, &levels, &Light::ALL));
                        println!(
                            "living-stage bake: display scale {scale}, levels {levels:?}, {:.0} ms wall on {} threads, {} sprites, {:.1} MB of pixels (stage {:.0} pt wide, {} residents)",
                            started.elapsed().as_secs_f64() * 1000.0,
                            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
                            baked.count(),
                            baked.bytes() as f64 / 1e6,
                            baked.width,
                            baked.residents.len(),
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
                        let walkers = Arc::new(Walkers::spread(&baked, walkers, 7));
                        cx.new(|_| stage::Bench {
                            baked,
                            walkers,
                            time: stage::Time::Real(Instant::now()),
                            stats,
                            hour,
                            animate: true,
                            until: Some(Duration::from_secs_f32(seconds)),
                            done,
                            warmed: 0,
                        })
                    },
                )
            };
            let window = match window {
                Ok(window) => window,
                Err(error) => {
                    eprintln!("living-stage: could not open a window: {error}");
                    cx.quit();
                    return;
                }
            };
            cx.activate(true);
            cx.spawn(async move |cx| {
                // The measured run starts when the first frame after the
                // warm-up is drawn; it ends after `seconds` of it.
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(100))
                        .await;
                    if done.get() {
                        break;
                    }
                }
                let scale = window
                    .update(cx, |_, window, _| window.scale_factor())
                    .unwrap_or(0.0);
                println!(
                    "living-stage: {} | display scale {scale} | {} {}",
                    stats.borrow().summary(),
                    std::env::consts::OS,
                    std::env::consts::ARCH,
                );
                cx.update(|cx| cx.quit());
            })
            .detach();
        });
        Ok(())
    }

    /// The living stage as the World window draws it: looks baked lazily,
    /// residents from their routines, everything in depth.
    fn run_window(
        options: Options,
        snapshot: ProjectionSnapshot,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use world_gpui::diorama::stage_view::{set_display_scale, StageBench};
        let application = gpui_platform::application();
        application.run(move |cx: &mut App| {
            set_display_scale(options.scale);
            let bounds = Bounds::centered(None, size(px(options.size.0), px(options.size.1)), cx);
            let bench = StageBench::new(
                Arc::new(snapshot.clone()),
                stage::Time::Real(Instant::now()),
                options.hour,
                Some(Duration::from_secs_f32(options.seconds)),
            );
            let (stats, done, memory, opened, bakes) = (
                bench.stats.clone(),
                bench.done.clone(),
                bench.memory.clone(),
                bench.opened.clone(),
                bench.bakes.clone(),
            );
            let window = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                move |_, cx| cx.new(|_| bench),
            );
            let window = match window {
                Ok(window) => window,
                Err(error) => {
                    eprintln!("living-stage: could not open a window: {error}");
                    cx.quit();
                    return;
                }
            };
            cx.activate(true);
            let began = Instant::now();
            cx.spawn(async move |cx| {
                let mut most = 0;
                while !done.get() {
                    cx.background_executor()
                        .timer(Duration::from_millis(100))
                        .await;
                    most = most.max(memory.get().1);
                }
                let display = window
                    .update(cx, |_, window, _| window.scale_factor())
                    .unwrap_or(0.0);
                let (now, peak) = memory.get();
                println!(
                    "living-stage window: {} | display scale {display}, baked for {} | first settled after {} frames ({:.1} s) | {} {}",
                    stats.borrow().summary(),
                    options.scale.unwrap_or(display),
                    opened.get().unwrap_or(0),
                    began.elapsed().as_secs_f32() - options.seconds,
                    std::env::consts::OS,
                    std::env::consts::ARCH,
                );
                println!(
                    "living-stage window pixel memory: {:.1} MB held at the end, {:.1} MB at most",
                    now as f64 / 1e6,
                    peak.max(most) as f64 / 1e6
                );
                println!("living-stage window {}", stats.borrow().bar());
                println!(
                    "living-stage window {}",
                    world_gpui::diorama::stage_view::bake_summary(&bakes.borrow())
                );
                cx.update(|cx| cx.quit());
            })
            .detach();
        });
        Ok(())
    }
}
