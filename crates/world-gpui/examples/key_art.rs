//! The key art (the art bible's §8), rendered by the engine itself from
//! fixed Worlds: the harbour at dusk in its second year, and each Pocket
//! Universe place by the same rules, at 3840×2160.
//!
//! ```bash
//! cargo run -p world-gpui --example key_art -- <out-dir>
//! ```
//!
//! Each picture is written to `<out-dir>/A-keyart-<place>.png`. The Worlds
//! are the scene's own fixtures (`tests/fixtures/`), so the same build
//! always draws the same pictures. It draws through the golden pictures'
//! CPU rasteriser, so it runs anywhere, without a GPU; text is not drawn.
//!
//! The composition:
//! - dusk, with the first lamps lit;
//! - the place's landmark (the lighthouse on the point) on the right third;
//! - a group of residents talking on the spine, in the lower left third;
//! - clear sky in the upper left, for the title.

#[path = "../src/offscreen.rs"]
mod offscreen;

use gpui::{prelude::*, AnyElement};
use world_gpui::diorama::{self, Camera, Clock, Glows};
use world_gpui::ladder;
use world_gpui::scene::Daylight;
use world_gpui::works::{Art, Family};
use world_projection::{CanvasItemKind, MarkShape, ProjectionSnapshot};

/// The window the key art is laid out for, in points: drawn at twice the
/// pixels, 3840×2160.
const WIDTH: f32 = 1920.0;
const HEIGHT: f32 = 1080.0;

/// Dusk: the low sun, long warm shadows, and the first lamps lit.
const HOUR: f32 = 19.1;

/// Where people are: out on the spine, before they go in for the evening.
const PEOPLE_HOUR: u8 = 16;

/// How close the camera comes: a window's width of the place, unfolded,
/// people at a size that reads.
const ZOOM: f32 = 1.0;

struct Shot {
    name: &'static str,
    fixture: &'static str,
    /// What stands on the right third: a drawing or a library art key.
    landmark: &'static [&'static str],
}

const SHOTS: [Shot; 4] = [
    Shot {
        name: "harbour",
        fixture: "harbour-day-163.json",
        landmark: &["lighthouse"],
    },
    Shot {
        name: "ares",
        fixture: "place-ares.json",
        landmark: &["control-tower", "comms-mast", "radio", "beacon-mast"],
    },
    Shot {
        name: "maple",
        fixture: "place-maple.json",
        landmark: &["water-tower", "radio", "arcade"],
    },
    Shot {
        name: "icebridge",
        fixture: "place-icebridge.json",
        landmark: &["ice-spire", "icebridge", "council"],
    },
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "key-art".into()));
    std::fs::create_dir_all(&out)?;
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let only = std::env::var("KEY_ART_ONLY").ok();
    for shot in SHOTS {
        if only.as_deref().is_some_and(|only| only != shot.name) {
            continue;
        }
        let json = std::fs::read_to_string(fixtures.join(shot.fixture))?;
        let wire: world_pack_protocol::ProjectionSnapshotWire = serde_json::from_str(&json)?;
        let mut snapshot =
            ProjectionSnapshot::try_from(wire).map_err(|error| format!("{error:?}"))?;
        // A clear evening, whatever the day's weather was.
        snapshot.weather = world_projection::Weather::Clear;
        let frame = compose(&snapshot, shot.landmark);
        let image = offscreen::draw(WIDTH, HEIGHT, move || painted(frame.clone()));
        let path = out.join(format!("A-keyart-{}.png", shot.name));
        image.save(&path)?;
        println!("{} ({}×{})", path.display(), image.width(), image.height());
    }
    if only.is_none() || only.as_deref() == Some("contact") {
        let path = out.join("A2-contact-sheet.png");
        contact_sheet(&fixtures)?.save(&path)?;
        println!("{}", path.display());
    }
    if only.is_none() {
        for family in [
            Family::Any,
            Family::Harbour,
            Family::Mars,
            Family::Street,
            Family::Ice,
        ] {
            let name = format!("{family:?}").to_lowercase();
            let path = out.join(format!("A-ladder-{name}.png"));
            ladder_sheet(family).save(&path)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}

/// The sign-off contact sheet (the art bible's last section): twelve
/// frames, each a window's width at zoom 1 on the busiest stretch, four
/// rows of three: the harbour's first screen on day 1 at noon, dusk and
/// night; the harbour in its ninth year (day 1,082) at noon, dusk and
/// night; and Ares, Maple Street and Icebridge at noon (row three) and at
/// night (row four). Their dusk is their key art.
fn contact_sheet(
    fixtures: &std::path::Path,
) -> Result<image::RgbaImage, Box<dyn std::error::Error>> {
    let load = |name: &str| -> Result<ProjectionSnapshot, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(fixtures.join(name))?;
        let wire: world_pack_protocol::ProjectionSnapshotWire = serde_json::from_str(&json)?;
        Ok(ProjectionSnapshot::try_from(wire).map_err(|error| format!("{error:?}"))?)
    };
    let times = [
        (12.5_f32, Daylight::Day),
        (19.2, Daylight::Dusk),
        (23.0, Daylight::Night),
    ];
    let mut frames = Vec::new();
    for name in ["harbour-day-1.json", "harbour-day-1082.json"] {
        let snapshot = load(name)?;
        for (hour, daylight) in times {
            frames.push((snapshot.clone(), hour, daylight));
        }
    }
    let places = [
        load("place-ares.json")?,
        load("place-maple.json")?,
        load("place-icebridge.json")?,
    ];
    // Rows three and four: the three places at noon, then at night (their
    // dusk is their key art).
    for (hour, daylight) in [times[0], times[2]] {
        for place in &places {
            frames.push((place.clone(), hour, daylight));
        }
    }
    let (w, h) = (960.0_f32, 600.0_f32);
    let (tile_w, tile_h) = (1280_u32, 800_u32);
    let mut sheet = image::RgbaImage::from_pixel(
        tile_w * 3 + 40,
        tile_h * 4 + 50,
        image::Rgba([0xf3, 0xee, 0xe3, 0xff]),
    );
    for (nth, (snapshot, hour, daylight)) in frames.into_iter().enumerate() {
        let stage = diorama::stage_at(&snapshot, w, h, Clock::at(hour as u8));
        let items = snapshot.canvas.items.clone();
        let centre = busiest(&stage, &items, w);
        let camera = Camera::around(&stage, 1.0, centre, h / 2.0);
        let living = diorama::living(&stage, &snapshot, 6.0, daylight, &Default::default(), None);
        let frame = diorama::frame(
            &snapshot,
            &stage,
            &living,
            camera,
            6.0,
            daylight,
            &Glows::new(),
            1.0,
        )
        .at_hour(hour)
        .at_seconds(6.0)
        .stilled(true);
        let image = offscreen::draw(w, h, move || painted(frame.clone()));
        let tile = image::imageops::resize(&image, tile_w, tile_h, image::imageops::Triangle);
        let (column, row) = (nth as u32 % 3, nth as u32 / 3);
        image::imageops::overlay(
            &mut sheet,
            &tile,
            (10 + column * (tile_w + 10)) as i64,
            (10 + row * (tile_h + 10)) as i64,
        );
    }
    Ok(sheet)
}

/// The scale ladder, for the art director: every drawing of a family
/// standing on one ground line, smallest to tallest, each beside a
/// grown-up's silhouette 1 P tall.
fn ladder_sheet(family: Family) -> image::RgbaImage {
    use world_gpui::brush::Brush;
    const P: f32 = 48.0;
    let setting = family.setting();
    let mut arts = Art::all()
        .filter(|art| art.family() == family)
        .filter_map(|art| {
            let subject = ladder::Subject::Thing(MarkShape::Parcel, setting, Some(art));
            let (w, _) = subject.sized(P, P)?;
            let tall = match ladder::of_art(art.key())? {
                ladder::Rung::Tall(tall) => tall,
                ladder::Rung::Wide(_) => 0.0,
            };
            Some((tall, w, art))
        })
        .collect::<Vec<_>>();
    arts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.key().cmp(b.2.key())));
    let (width, row) = (3600.0_f32, P * 7.0);
    // Lay out rows left to right, each drawing and its person.
    let mut rows: Vec<Vec<(f32, f32, Art)>> = vec![Vec::new()];
    let mut x = P;
    for (_, w, art) in arts {
        let room = w * 0.85 + P * 0.9;
        if x + room > width - P {
            rows.push(Vec::new());
            x = P;
        }
        rows.last_mut().expect("a row").push((x, w, art));
        x += room;
    }
    let height = row * rows.len() as f32 + P;
    let mut canvas = world_gpui::painter::Canvas::new(width as u32, height as u32, 1.0, (0.0, 0.0))
        .expect("a sheet");
    canvas
        .pixmap
        .fill(tiny_skia::Color::from_rgba8(0xf3, 0xee, 0xe3, 0xff));
    let ink = world_gpui::art::hex(0x3a3226);
    for (nth, members) in rows.iter().enumerate() {
        let base = row * (nth as f32 + 1.0);
        canvas.rect(0.0, base, width, 1.5, 0.0, ink.opacity(0.4));
        // The ladder's rungs: 1, 2, 3, 4, 5 and 6 P.
        for rung in 1..=6 {
            let y = base - P * rung as f32;
            canvas.rect(0.0, y, width, 1.0, 0.0, ink.opacity(0.08));
        }
        for (x, w, art) in members {
            let palette = world_gpui::art::Palette::of_in(art.key(), false, setting)
                .drawn_as(Some(art.key()));
            world_gpui::art::paint_thing(
                &mut canvas,
                x + w * 0.4,
                base,
                *w,
                MarkShape::Parcel,
                &palette,
                0.0,
            );
            // A grown-up beside it, 1 P tall.
            let px = x + w * 0.85 + P * 0.25;
            canvas.rect(
                px - P * 0.11,
                base - P * 0.78,
                P * 0.22,
                P * 0.78,
                P * 0.06,
                ink.opacity(0.55),
            );
            world_gpui::art::circle(
                &mut canvas,
                px,
                base - P * 0.88,
                P * 0.12,
                ink.opacity(0.55),
            );
        }
    }
    image::RgbaImage::from_raw(
        canvas.pixmap.width(),
        canvas.pixmap.height(),
        canvas.pixmap.data().to_vec(),
    )
    .expect("a picture")
}

/// The key art's frame of `snapshot`: dusk, the camera putting the
/// landmark on the right third and a group of people in the lower left.
fn compose(snapshot: &ProjectionSnapshot, landmark: &[&str]) -> diorama::Frame {
    // People where the late afternoon has them, out and about; the light
    // of a little later, the sun going down.
    let people_hour = std::env::var("KEY_ART_PEOPLE")
        .ok()
        .and_then(|hour| hour.parse().ok())
        .unwrap_or(PEOPLE_HOUR);
    let stage = diorama::stage_at(snapshot, WIDTH, HEIGHT, Clock::at(people_hour));
    if std::env::var_os("KEY_ART_DEBUG").is_some() {
        let items = &snapshot.canvas.items;
        eprintln!("width {} people {}", stage.width, stage.people.len());
        for spot in stage
            .buildings
            .iter()
            .chain(&stage.things)
            .chain(&stage.people)
        {
            let item = &items[spot.index];
            eprintln!(
                "{:7.0} {:5.0} {:?} {:?} {:?} {}",
                spot.x,
                spot.y,
                item.kind,
                item.art,
                snapshot.drawing_of(item).map(|d| d.id.clone()),
                item.label
            );
        }
    }
    let items = &snapshot.canvas.items;
    let named = |index: usize, key: &str| {
        let item = &items[index];
        item.art.as_deref() == Some(key)
            || snapshot
                .drawing_of(item)
                .is_some_and(|drawing| drawing.id == key)
    };
    let spots = || stage.buildings.iter().chain(stage.things.iter());
    let mark = landmark
        .iter()
        .find_map(|key| spots().find(|spot| named(spot.index, key)))
        .map(|spot| spot.x);
    let view = WIDTH / ZOOM;
    // The landmark on the right third; failing one, the busiest stretch.
    let centre = match mark {
        Some(x) => x - view / 6.0,
        None => busiest(&stage, items, view),
    };
    // Sky over a third of the frame, the water's edge at its foot.
    let view_h = HEIGHT / ZOOM;
    let top = stage.horizon - view_h * 0.34;
    let camera = Camera::around(&stage, ZOOM, centre, top + view_h / 2.0);
    let living = diorama::living(
        &stage,
        snapshot,
        6.0,
        Daylight::Dusk,
        &Default::default(),
        None,
    );
    let mut frame = diorama::frame(
        snapshot,
        &stage,
        &living,
        camera,
        6.0,
        Daylight::Dusk,
        &Glows::new(),
        1.0,
    )
    .at_hour(hour())
    .at_seconds(6.0)
    .stilled(true);
    // Bunting strung across the lane between two neighbouring buildings
    // in the picture, nearest the middle of its left two thirds: the town
    // dressed for an evening.
    let (left, right) = (camera.x - view / 2.0, camera.x + view / 6.0);
    let p = stage.figure_h;
    let along = frame.buildings_along();
    if let Some((a, b)) = along
        .windows(2)
        .filter(|pair| {
            let (a, b) = (pair[0], pair[1]);
            a.1 > left
                && b.1 < right
                && b.1 - a.1 < p * 6.0
                && b.1 - a.1 > (a.2 + b.2) / 2.0
                && !mark.is_some_and(|x| (a.1 - x).abs() < 1.0 || (b.1 - x).abs() < 1.0)
        })
        .min_by(|x, y| {
            let middle = (left + right) / 2.0;
            let off = |pair: &&[(usize, f32, f32)]| ((pair[0].1 + pair[1].1) / 2.0 - middle).abs();
            off(x).total_cmp(&off(y))
        })
        .map(|pair| (pair[0].0, pair[1].0))
    {
        frame.string_bunting(a, b);
    }
    // An evening's talk on the quay: in each group one talks, the others
    // listen, turned to them.
    let mut last = f32::MIN;
    let mut nth = 0;
    let mut order = (0..frame.people.len()).collect::<Vec<_>>();
    order.sort_by(|a, b| frame.people[*a].x.total_cmp(&frame.people[*b].x));
    for at in order {
        let person = &mut frame.people[at];
        nth = if person.x - last < person.height {
            nth + 1
        } else {
            0
        };
        last = person.x;
        if person.stance != world_projection::Stance::Walking {
            person.stance = if nth == 1 {
                world_projection::Stance::Talking
            } else {
                world_projection::Stance::Standing
            };
        }
    }
    frame
}

/// Where along the stage a window `view` wide sees the most people and
/// places.
fn busiest(stage: &diorama::Stage, items: &[world_projection::CanvasItem], view: f32) -> f32 {
    let mut best = (0, stage.width / 2.0);
    let mut x = view / 2.0;
    while x < stage.width - view / 2.0 {
        let seen = stage
            .people
            .iter()
            .chain(stage.buildings.iter())
            .filter(|spot| (spot.x - x).abs() < view / 2.0)
            .filter(|spot| items[spot.index].kind != CanvasItemKind::Object)
            .count();
        if seen > best.0 {
            best = (seen, x);
        }
        x += view / 8.0;
    }
    best.1
}

/// The hour the light is graded for: dusk, or `KEY_ART_HOUR` to look at
/// the same composition in another light.
fn hour() -> f32 {
    std::env::var("KEY_ART_HOUR")
        .ok()
        .and_then(|hour| hour.parse().ok())
        .unwrap_or(HOUR)
}

fn painted(frame: diorama::Frame) -> AnyElement {
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| diorama::paint(&frame, bounds, window),
    )
    .size_full()
    .into_any_element()
}
