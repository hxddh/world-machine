//! The living stage: the v0.30 S0 spike, not yet part of any window.
//!
//! The question it answers: can the still scene the CPU painter paints a
//! tile at a time be baked **once**, a sprite per drawing, at a few zoom
//! levels and in a few lights, so GPUI composites the whole place every
//! frame from images, with hundreds of people walking, boats riding the
//! swell and chimneys smoking, in a few milliseconds a frame?
//!
//! Everything is drawn by the same brush as the painter, from the same
//! [`Frame`]: the land tiles (`paint_land_tile`), each building's lit
//! sprite (`works::sprite_painted`), its shadow (`paint_building_shadow`),
//! the things on the spine, the people's poses (`age::paint_person`), the
//! sky and the far hills. What changes here is when they are painted: all
//! of it before the first frame, kept as GPUI images, and composited by
//! [`plan`] with GPUI's image primitive.
//!
//! GPUI has no custom shaders, so light is layered images:
//! - each light (noon, dusk, night) is baked as its own set of sprites,
//!   and a change of hour cross-fades one set into the next, layer by
//!   layer (so an opaque building over its land blends exactly); what
//!   stands in front of one another (buildings, things, people) is
//!   cross-faded picture by picture instead ([`cross_fade`]), so nobody
//!   ever shows through whoever stands in front of them;
//! - what the painter draws live (lamps, the beacon, things, people,
//!   smoke) is baked with the canvas's `window_soft`, so its soft light
//!   falls off as a window's blur does, not as a solid disc;
//! - lit windows' glow is a halo of its own over the buildings, faded in
//!   with the light and flickering a little by groups, with a pool of it
//!   on the ground.
//!
//! This module is presentation only, like the rest of the diorama: it
//! reads a snapshot and never anything the World records.

use super::*;
use gpui::{div, prelude::*, RenderImage};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A rectangle: x, y, width, height.
pub type Rect = (f32, f32, f32, f32);

/// The lights baked: each a set of sprites of its own.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Light {
    Noon,
    Dusk,
    Night,
}

impl Light {
    pub const ALL: [Light; 3] = [Light::Noon, Light::Dusk, Light::Night];

    /// The hour each light is graded for (the contact sheet's).
    pub fn hour(self) -> f32 {
        match self {
            Light::Noon => 12.5,
            Light::Dusk => 19.2,
            Light::Night => 23.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Light::Noon => "noon",
            Light::Dusk => "dusk",
            Light::Night => "night",
        }
    }

    pub(crate) fn daylight(self) -> Daylight {
        match self {
            Light::Noon => Daylight::Day,
            Light::Dusk => Daylight::Dusk,
            Light::Night => Daylight::Night,
        }
    }

    /// Whether its windows are lit.
    fn lit(self) -> bool {
        self != Light::Noon
    }

    /// The two lights an hour falls between, and how far from the first
    /// to the second (0 to 1).
    pub fn blend(hour: f32) -> (Light, Light, f32) {
        let hour = hour.rem_euclid(24.0);
        let smooth = |t: f32| {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        match hour {
            h if (8.0..16.5).contains(&h) => (Light::Noon, Light::Noon, 0.0),
            h if (16.5..19.2).contains(&h) => (Light::Noon, Light::Dusk, smooth((h - 16.5) / 2.7)),
            h if (19.2..20.5).contains(&h) => (Light::Dusk, Light::Dusk, 0.0),
            h if (20.5..23.0).contains(&h) => (Light::Dusk, Light::Night, smooth((h - 20.5) / 2.5)),
            h if (5.0..8.0).contains(&h) => (Light::Night, Light::Noon, smooth((h - 5.0) / 3.0)),
            _ => (Light::Night, Light::Night, 0.0),
        }
    }
}

/// One baked picture and where it stands: in stage pixels for what stands
/// on the stage, relative to an anchor for what moves (a person's feet, a
/// puff's middle).
#[derive(Clone)]
pub struct Sprite {
    pub image: Arc<RenderImage>,
    pub rect: Rect,
    /// The part of `rect` shown, for tiles that overlap their neighbours
    /// by a pixel.
    pub clip: Option<Rect>,
}

impl Sprite {
    /// A sprite of `pixmap`, whose top-left is the stage point `origin` at
    /// `scale` device pixels a stage pixel, cut to what it paints. Nothing
    /// if it paints nothing.
    pub(crate) fn cropped(pixmap: sk::Pixmap, origin: (f32, f32), scale: f32) -> Option<Self> {
        let (pixmap, (dx, dy)) = crop(pixmap)?;
        Some(Self::whole(
            pixmap,
            (origin.0 + dx as f32 / scale, origin.1 + dy as f32 / scale),
            scale,
        ))
    }

    fn whole(pixmap: sk::Pixmap, origin: (f32, f32), scale: f32) -> Self {
        let rect = (
            origin.0,
            origin.1,
            pixmap.width() as f32 / scale,
            pixmap.height() as f32 / scale,
        );
        Self {
            image: painter::image_of(pixmap),
            rect,
            clip: None,
        }
    }

    /// Its pixels' size in memory, straight BGRA.
    pub fn bytes(&self) -> usize {
        let size = self.image.size(0);
        (size.width.0.max(0) as usize) * (size.height.0.max(0) as usize) * 4
    }
}

/// The pixmap cut to the box of what it paints, and where that box began.
fn crop(pixmap: sk::Pixmap) -> Option<(sk::Pixmap, (u32, u32))> {
    let (w, h) = (pixmap.width() as usize, pixmap.height() as usize);
    let data = pixmap.data();
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for y in 0..h {
        let row = &data[y * w * 4..(y + 1) * w * 4];
        let mut any = false;
        for (x, pixel) in row.as_chunks::<4>().0.iter().enumerate() {
            if pixel[3] > 1 {
                x0 = x0.min(x);
                x1 = x1.max(x + 1);
                any = true;
            }
        }
        if any {
            y0 = y0.min(y);
            y1 = y + 1;
        }
    }
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    if x0 == 0 && y0 == 0 && x1 == w && y1 == h {
        return Some((pixmap, (0, 0)));
    }
    let rect = sk::IntRect::from_xywh(x0 as i32, y0 as i32, (x1 - x0) as u32, (y1 - y0) as u32)?;
    Some((pixmap.clone_rect(rect)?, (x0 as u32, y0 as u32)))
}

/// One resident's poses, relative to their feet, for a grown-up `P` tall
/// (scaled by their age): a walk cycle each way and standing each way.
#[derive(Clone)]
pub struct Poses {
    /// Facing left, then right: [`WALK_FRAMES`] steps each.
    pub walk: [Vec<Sprite>; 2],
    pub stand: [Option<Sprite>; 2],
}

/// How many steps of a walk are baked.
pub const WALK_FRAMES: usize = 8;

/// How many residents a walk is baked for; the rest only stand.
pub const WALKING_LOOKS: usize = 24;

/// How many steps of a puff's life are baked.
const PUFF_STAGES: usize = 8;

/// Everything of one light at one zoom level.
pub struct Look {
    pub light: Light,
    /// Device pixels a stage pixel.
    pub scale: f32,
    /// The sky, in view pixels.
    pub sky: Vec<Sprite>,
    /// The far hills, relative to their band's top-left.
    pub hills: Vec<Sprite>,
    pub land: Vec<Sprite>,
    pub shadows: Vec<Sprite>,
    /// What stands along the spine behind the things: lamps, jetties,
    /// a street's poles.
    pub back: Vec<Sprite>,
    /// Buildings, back to front, each by its item.
    pub buildings: Vec<(usize, Sprite)>,
    /// Lit windows' halos and their pools of light, each in one of
    /// [`GLOW_GROUPS`] flickering together, by the building they are of.
    pub glow: Vec<(usize, usize, Sprite)>,
    /// Things, back to front, each by its item and whether it rides the
    /// swell.
    pub things: Vec<(usize, bool, Sprite)>,
    /// What stands in front of everyone: the framing strip, bunting, the
    /// beacon's lamp.
    pub front: Vec<Sprite>,
    /// Every resident's poses, by resident.
    pub people: Vec<Poses>,
    /// A chimney's puff through its life, relative to its middle, for a
    /// cottage `building_w` wide.
    pub puffs: Vec<Sprite>,
    /// A glint on the water, relative to its middle.
    pub glint: Option<Sprite>,
    /// The light on the water (dusk's gold sheen deepening out to sea): a
    /// strip from the water's edge to the foot of the stage, the same all
    /// the way across, stretched to the window's width.
    pub water: Option<Sprite>,
    /// What of the place it was baked of.
    pub cut: Cut,
    /// Where each building and thing stood as it was baked, by item: its
    /// middle, its foot and how wide it was (a building's height too).
    pub anchors: std::collections::HashMap<usize, Anchor>,
    /// What it was baked of, piece by piece, and where each piece's
    /// sprites are: so a look baked again (the camera moved on, the World
    /// changed) takes from it whatever is the same.
    pub of: Pieces,
}

/// What a look was baked of, and where its pieces are in it.
#[derive(Default)]
pub struct Pieces {
    /// The World it was baked of (its bake key), or 0.
    pub world: u64,
    /// Everything of the frame a building's or thing's own sprite is
    /// painted from besides itself: the light, the scale, the place's
    /// colours and the hour, the weather and the season, the stage's lines.
    pub level: u64,
    /// Each building and thing baked, by item: what it is (all of its
    /// paint), and for a building in a lit hour what stands in front of it
    /// (whose shapes are taken out of its halo).
    items: HashMap<usize, (u64, u64)>,
    /// Each item's sprites, by item: its buildings', halos', things' and
    /// shadows' places in the look.
    by_item: HashMap<usize, [Vec<usize>; 4]>,
    /// Each land tile baked, by column and row, and its place in the look
    /// (none: it painted nothing).
    land: HashMap<(i32, i32), Option<usize>>,
    /// Each column of the props baked, and its places in the look, behind
    /// and in front.
    props: HashMap<i32, (Vec<usize>, Vec<usize>)>,
}

impl Pieces {
    fn of(&self, item: usize, kind: usize) -> &[usize] {
        self.by_item.get(&item).map_or(&[], |all| &all[kind])
    }
}

/// Where a building or thing stood as it was baked: its middle, its foot,
/// how wide and how tall (0 for a thing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub x: f32,
    pub base: f32,
    pub w: f32,
    pub h: f32,
    /// Whether it lies flat and is walked on (see [`lies_flat`]).
    pub flat: bool,
}

/// Whether a thing of `shape` with its foot at `base` lies flat on the
/// water for people to stand on, on a stage whose water begins at
/// `front`: a pier, a sea wall's landing, a bridge, a pool in the rocks.
/// It is ground, drawn under everyone, never stood in depth by its foot
/// at the waterline: someone standing on it or on the shore behind it
/// would be cut off at the waist (v0.30 S1a's stone landing).
pub fn lies_flat(shape: MarkShape, base: f32, front: f32) -> bool {
    matches!(shape, MarkShape::Pier | MarkShape::Bridge)
        || (base >= front - 1.0 && shape == MarkShape::Fountain)
}

/// What of a place one look is baked of.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    /// Only what stands over this stretch of the stage (from, to, in
    /// stage pixels), or all of it. A close zoom level is baked around the
    /// camera only: the whole of a long place that close is hundreds of
    /// megabytes.
    pub span: Option<(f32, f32)>,
    /// Every resident's poses (the benchmark's walkers), or none (the
    /// World window bakes the poses its residents strike as it needs them).
    pub people: bool,
    /// Only the land, its field fading in at the top: a nearer row of a
    /// folded postcard, which runs on from the row behind with no seam.
    pub fold_land: bool,
}

impl Cut {
    /// All of the place, everyone's poses with it.
    pub const WHOLE: Cut = Cut {
        span: None,
        people: true,
        fold_land: false,
    };

    /// Whether something reaching from stage `x0` to `x1` is in the cut.
    pub fn holds(&self, x0: f32, x1: f32) -> bool {
        self.span.is_none_or(|(from, to)| x1 >= from && x0 <= to)
    }

    /// Whether the cut shows all of the stage from `x0` to `x1`.
    pub fn covers(&self, x0: f32, x1: f32) -> bool {
        self.span
            .is_none_or(|(from, to)| x0 >= from - 0.5 && x1 <= to + 0.5)
    }
}

impl Look {
    /// The sprites of the building `item`.
    pub fn building_sprites(&self, item: usize) -> impl Iterator<Item = &Sprite> {
        self.of.of(item, 0).iter().map(|at| &self.buildings[*at].1)
    }

    /// The halos of the building `item`, each with its flickering group.
    pub fn halos(&self, item: usize) -> impl Iterator<Item = (usize, &Sprite)> {
        self.of.of(item, 1).iter().map(|at| {
            let (group, _, halo) = &self.glow[*at];
            (*group, halo)
        })
    }

    /// The sprites of the thing `item`.
    pub fn thing_sprites(&self, item: usize) -> impl Iterator<Item = &Sprite> {
        self.of.of(item, 2).iter().map(|at| &self.things[*at].2)
    }

    /// The shadows of the building `item`.
    fn shadows_of(&self, item: usize) -> impl Iterator<Item = &Sprite> {
        self.of.of(item, 3).iter().map(|at| &self.shadows[*at])
    }

    fn sprites(&self) -> impl Iterator<Item = &Sprite> {
        self.sky
            .iter()
            .chain(&self.hills)
            .chain(&self.land)
            .chain(&self.shadows)
            .chain(&self.back)
            .chain(self.buildings.iter().map(|(_, sprite)| sprite))
            .chain(self.glow.iter().map(|(_, _, sprite)| sprite))
            .chain(self.things.iter().map(|(_, _, sprite)| sprite))
            .chain(&self.front)
            .chain(self.people.iter().flat_map(|poses| {
                poses
                    .walk
                    .iter()
                    .flatten()
                    .chain(poses.stand.iter().flatten())
            }))
            .chain(&self.puffs)
            .chain(&self.glint)
            .chain(&self.water)
    }

    pub fn count(&self) -> usize {
        self.sprites().count()
    }

    /// Each layer's sprites and bytes, by name.
    pub fn breakdown(&self) -> Vec<(&'static str, usize, usize)> {
        let of = |name, sprites: Vec<&Sprite>| {
            (
                name,
                sprites.len(),
                sprites.iter().map(|sprite| sprite.bytes()).sum::<usize>(),
            )
        };
        vec![
            of("sky", self.sky.iter().collect()),
            of("hills", self.hills.iter().collect()),
            of("land", self.land.iter().collect()),
            of("shadows", self.shadows.iter().collect()),
            of("props behind", self.back.iter().collect()),
            of(
                "buildings",
                self.buildings.iter().map(|(_, sprite)| sprite).collect(),
            ),
            of(
                "glow",
                self.glow.iter().map(|(_, _, sprite)| sprite).collect(),
            ),
            of(
                "things",
                self.things.iter().map(|(_, _, sprite)| sprite).collect(),
            ),
            of("props in front", self.front.iter().collect()),
            of(
                "people",
                self.people
                    .iter()
                    .flat_map(|poses| {
                        poses
                            .walk
                            .iter()
                            .flatten()
                            .chain(poses.stand.iter().flatten())
                    })
                    .collect(),
            ),
            of(
                "smoke, glints, water",
                self.puffs
                    .iter()
                    .chain(&self.glint)
                    .chain(&self.water)
                    .collect(),
            ),
        ]
    }

    pub fn bytes(&self) -> usize {
        self.sprites().map(Sprite::bytes).sum()
    }

    /// Every image of it.
    pub fn images(&self) -> Vec<Arc<RenderImage>> {
        self.sprites().map(|sprite| sprite.image.clone()).collect()
    }
}

/// How many pixels wide the strip of the light on the water is.
const WATER_STRIP: u32 = 16;

/// How many groups of windows flicker together.
pub const GLOW_GROUPS: usize = 3;

/// Someone of the place where the stage has them: their poses, their feet
/// on the stage, which way they face (0 left, 1 right) and how near (1 on
/// the spine, less further back).
#[derive(Clone, Copy, Debug)]
pub struct Resident {
    pub look: usize,
    pub x: f32,
    pub y: f32,
    pub facing: usize,
    pub scale: f32,
}

/// What one bake took.
#[derive(Clone, Debug)]
pub struct BakeStat {
    pub light: Light,
    pub scale: f32,
    pub took: Duration,
    pub sprites: usize,
    pub bytes: usize,
}

/// A place baked: its geometry, and every look of it.
pub struct Baked {
    /// The window it is baked for, in points.
    pub view: (f32, f32),
    pub width: f32,
    pub horizon: f32,
    pub feet: f32,
    pub front: f32,
    pub figure_h: f32,
    pub building_w: f32,
    /// The far hills' band: its width, height, how far it reaches past
    /// the window's left edge at rest, and how far above the horizon.
    band: (f32, f32, f32, f32),
    pub lights: Vec<Light>,
    pub levels: Vec<f32>,
    looks: Vec<Look>,
    pub residents: Vec<Resident>,
    /// Chimneys: each pot, and how big its cottage is beside the usual.
    pub chimneys: Vec<(f32, f32, f32)>,
    /// Which way the wind takes the smoke, and how hard.
    pub blow: f32,
    pub stats: Vec<BakeStat>,
}

impl Baked {
    pub fn looks(&self) -> &[Look] {
        &self.looks
    }

    pub fn look(&self, level: usize, light: Light) -> Option<&Look> {
        let at = self.lights.iter().position(|l| *l == light)?;
        self.looks.get(level * self.lights.len() + at)
    }

    pub fn bytes(&self) -> usize {
        self.looks.iter().map(Look::bytes).sum()
    }

    /// Every image of every look, to hand to the display ahead of use.
    pub fn images(&self) -> Vec<Arc<RenderImage>> {
        self.looks
            .iter()
            .flat_map(|look| look.sprites().map(|sprite| sprite.image.clone()))
            .collect()
    }

    pub fn count(&self) -> usize {
        self.looks.iter().map(Look::count).sum()
    }

    pub fn took(&self) -> Duration {
        self.stats.iter().map(|stat| stat.took).sum()
    }

    /// The level to draw at for `scale` device pixels a stage pixel: the
    /// least as sharp, or the sharpest there is.
    pub fn level_for(&self, scale: f32) -> usize {
        self.levels
            .iter()
            .position(|level| *level >= scale * 0.92)
            .unwrap_or(self.levels.len().saturating_sub(1))
    }

    /// How many residents have a walk baked.
    pub fn walking_looks(&self) -> usize {
        self.residents.len().min(WALKING_LOOKS)
    }

    /// The ground every building stands on (see [`footprint`]).
    pub fn footprints(&self) -> Vec<(f32, f32, f32, f32)> {
        self.looks
            .first()
            .map(|look| {
                look.anchors
                    .values()
                    .filter(|anchor| anchor.h > 0.0)
                    .map(|anchor| footprint(anchor.x, anchor.base, anchor.w, anchor.h))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// A lane for someone walking the whole stage with their feet at `feet`,
/// kept within `(least, most)`: as near it as it can be without crossing
/// any of `footprints` (every building's ground).
pub fn free_lane(footprints: &[(f32, f32, f32, f32)], feet: f32, (least, most): (f32, f32)) -> f32 {
    let clear = |y: f32| {
        footprints
            .iter()
            .all(|(_, back, _, front)| y <= *back || y >= *front)
    };
    if clear(feet) {
        return feet;
    }
    // Just in front of or just behind whatever it would cross, the nearest
    // that is clear and in bounds.
    let mut edges = footprints
        .iter()
        .flat_map(|(_, back, _, front)| [*back - 0.5, *front + 0.5])
        .filter(|y| *y >= least && *y <= most && clear(*y))
        .collect::<Vec<_>>();
    edges.sort_by(|a, b| (a - feet).abs().total_cmp(&(b - feet).abs()));
    edges.first().copied().unwrap_or(feet)
}

/// The hour the stage puts people where they are: out and about.
const PEOPLE_HOUR: u8 = 16;

/// How many threads a bake runs on: every core but two. The window's own
/// thread keeps a core to plan frames on while a look bakes, and the
/// display (its compositor and driver) another (v0.30 S1a: a bake on every
/// core stretched frames to 30 ms and more on a three-core Mac; S1b: on
/// every core but one, a three-core runner still missed frames).
pub fn bake_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .saturating_sub(2)
        .max(1)
}

/// Runs `jobs` across the cores a bake may use, each taking the next job
/// as it is free.
fn across_cores<T: Send>(jobs: Vec<Box<dyn FnOnce() -> T + Send + '_>>) -> Vec<T> {
    let count = jobs.len();
    let threads = bake_threads().min(count.max(1));
    let slots = jobs
        .into_iter()
        .map(|job| std::sync::Mutex::new(Some(job)))
        .collect::<Vec<_>>();
    let out = (0..count)
        .map(|_| std::sync::Mutex::new(None))
        .collect::<Vec<_>>();
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let at = next.fetch_add(1, Ordering::Relaxed);
                if at >= count {
                    break;
                }
                let job = slots[at].lock().ok().and_then(|mut slot| slot.take());
                if let Some(job) = job {
                    let made = job();
                    if let Ok(mut slot) = out[at].lock() {
                        *slot = Some(made);
                    }
                }
            });
        }
    });
    out.into_iter()
        .filter_map(|slot| slot.into_inner().ok().flatten())
        .collect()
}

/// One piece of a look, as baked.
enum Piece {
    Sky(Sprite),
    Hills(Sprite),
    /// A land tile, by column and row.
    Land((i32, i32), Sprite),
    /// A building's shadow, by its item.
    Shadow(usize, Sprite),
    /// The props behind, by column.
    Back(i32, Sprite),
    /// A building: how far forward it stands, its item, its sprite.
    /// In a lit hour with its pixels kept, to hide the halos behind it.
    Building((f32, usize), Sprite, Option<Raw>),
    /// The halo of a building's windows, by where the building stands,
    /// in its flickering group: not yet a sprite, so what stands in front
    /// of the building can be taken out of it.
    Glow((f32, usize), usize, Raw),
    /// A halo taken from the look baked before, what stands in front of
    /// it already taken out.
    Halo((f32, usize), usize, Sprite),
    /// A building's pixels, painted again only to hide the halos behind
    /// it (its sprite taken from the look baked before).
    Mask((f32, usize), Raw),
    /// A thing: how far forward it stands, its item, whether it rides the
    /// swell, its sprite.
    Thing((f32, usize), bool, Sprite),
    /// The props in front, by column.
    Front(i32, Sprite),
    Person(usize, Box<Poses>),
    Puffs(Vec<Sprite>),
    Glint(Sprite),
    Water(Sprite),
}

/// Bakes `snapshot` for a window `view` points large: every light in
/// `lights` at every scale in `levels` (device pixels a stage pixel).
pub fn bake(
    snapshot: &ProjectionSnapshot,
    view: (f32, f32),
    levels: &[f32],
    lights: &[Light],
) -> Baked {
    let (vw, vh) = view;
    let stage = stage_at(snapshot, vw, vh, Clock::at(PEOPLE_HOUR));
    let camera = Camera::around(&stage, 1.0, stage.width / 2.0, vh / 2.0);
    let mut looks = Vec::new();
    let mut stats = Vec::new();
    let mut residents = Vec::new();
    let mut chimneys = Vec::new();
    let mut blow = 0.8;
    let mut frames = Vec::new();
    for light in lights {
        let living = living(
            &stage,
            snapshot,
            0.0,
            light.daylight(),
            &Default::default(),
            None,
        );
        let frame = frame(
            snapshot,
            &stage,
            &living,
            camera,
            0.0,
            light.daylight(),
            &Glows::new(),
            1.0,
        )
        .at_hour(light.hour())
        .stilled(true);
        if residents.is_empty() {
            residents = stage
                .people
                .iter()
                .zip(&living)
                .filter_map(|(spot, life)| {
                    frame.people.iter().position(|p| p.index == spot.index)?;
                    Some((spot, life))
                })
                .enumerate()
                .map(|(look, (spot, life))| Resident {
                    look,
                    x: life.x,
                    y: spot.y,
                    facing: usize::from(life.pose.facing >= 0.0),
                    scale: spot.scale,
                })
                .collect();
            blow = wind(&frame);
            chimneys = frame
                .buildings
                .iter()
                .filter(|b| {
                    b.shape == MarkShape::House
                        && b.drawing.is_none()
                        && b.palette.art.is_none()
                        && frame.setting.smokes()
                })
                .map(|b| {
                    let (x, y) = art::chimney_top(b.x, b.base, b.w, b.h, &b.palette);
                    (x, y, b.w / stage.building_w.max(1.0))
                })
                .collect();
        }
        frames.push((*light, frame, living));
    }
    for level in levels {
        for (light, frame, living) in &frames {
            let started = Instant::now();
            let look = bake_look(
                frame,
                &stage,
                living,
                (*light, *level),
                view,
                Cut::WHOLE,
                (0, None),
            );
            stats.push(BakeStat {
                light: *light,
                scale: *level,
                took: started.elapsed(),
                sprites: look.count(),
                bytes: look.bytes(),
            });
            looks.push(look);
        }
    }
    let band = frames
        .first()
        .map(|(_, frame, _)| {
            let band = Band::of(frame, vw, vh);
            (band.w, band.above + band.below, band.pad, band.above)
        })
        .unwrap_or((vw, vh * 0.4, 0.0, vh * 0.2));
    Baked {
        view,
        width: stage.width,
        horizon: stage.horizon,
        feet: stage.feet,
        front: stage.front,
        figure_h: stage.figure_h,
        building_w: stage.building_w,
        band,
        lights: lights.to_vec(),
        levels: levels.to_vec(),
        looks,
        residents,
        chimneys,
        blow,
        stats,
    }
}

/// Bakes one light at one scale, of what `cut` holds.
///
/// `world` is the bake key of the World `frame` is of, and `before` a
/// look baked earlier of the same light at the same scale: whatever of it
/// is the same is taken from it rather than painted again (its pictures
/// are already with the display, too). Of the same World, every piece
/// still in the cut (the camera moved on along a close level); of another
/// (a building built, a thing placed), each building and thing that has
/// not changed, nor anything standing in front of it. The look is the
/// same, pixel for pixel, as one baked from nothing.
pub(crate) fn bake_look(
    frame: &Frame,
    stage: &Stage,
    living: &[Living],
    (light, scale): (Light, f32),
    (vw, vh): (f32, f32),
    cut: Cut,
    (world, before): (u64, Option<&Look>),
) -> Look {
    type Job<'a> = Box<dyn FnOnce() -> Vec<Piece> + Send + 'a>;
    let mut jobs: Vec<Job> = Vec::new();
    let tint = frame.light();
    let whole = !cut.fold_land;
    let level = level_key(frame, light, scale);
    let before = before.filter(|look| {
        look.scale == scale
            && look.light == light
            && look.cut.fold_land == cut.fold_land
            && look.of.level == level
    });
    // Of the same World: every piece of it is the same where it was baked.
    let same = before.filter(|look| world != 0 && look.of.world == world);
    // The sky, soft all over: a third of the display's resolution.
    if let Some(same) = same.filter(|_| whole) {
        jobs.push(Box::new(move || {
            same.sky
                .iter()
                .cloned()
                .map(Piece::Sky)
                .chain(same.hills.iter().cloned().map(Piece::Hills))
                .collect()
        }));
    } else if whole {
        jobs.push(Box::new(move || {
            let s = (scale * SKY_RES).max(0.5);
            let Some(mut canvas) = Canvas::new(
                (vw * s).ceil() as u32,
                (vh * s).ceil() as u32,
                s,
                (0.0, 0.0),
            ) else {
                return Vec::new();
            };
            paint_sky_on(&mut canvas, (0, 0), frame, vw, vh);
            vec![Piece::Sky(Sprite::whole(canvas.pixmap, (0.0, 0.0), s))]
        }));
    }
    // The far hills, in strips a texture wide.
    let band = Band::of(frame, vw, vh);
    let hill_scale = (scale * 0.5).max(1.0);
    let tall = band.above + band.below;
    let strip = 1024.0 / hill_scale;
    let mut x0 = 0.0;
    while whole && same.is_none() && x0 < band.w {
        let w = strip.min(band.w - x0);
        let band = Band::of(frame, vw, vh);
        jobs.push(Box::new(move || {
            let Some(mut canvas) = Canvas::new(
                (w * hill_scale).ceil() as u32,
                (tall * hill_scale).ceil() as u32,
                hill_scale,
                (x0, 0.0),
            ) else {
                return Vec::new();
            };
            let at = ((x0 * hill_scale).round() as i32, 0);
            paint_band_on(&mut canvas, at, frame, &band);
            Sprite::cropped(canvas.pixmap, (x0, 0.0), hill_scale)
                .map(Piece::Hills)
                .into_iter()
                .collect()
        }));
        x0 += strip;
    }
    // The land, at half the display's resolution as the painter has it,
    // in its own tiles (each a pixel larger all round, shown cut to its
    // own square so neighbours blend real pixels where they meet).
    let land_scale = scale * 0.5;
    let tile_w = TILE as f32 / land_scale;
    let tiles = tiles_of_stage(frame, Layer::Land, land_scale)
        .into_iter()
        .filter(|(column, _)| cut.holds(*column as f32 * tile_w, (*column + 1) as f32 * tile_w))
        .collect::<Vec<_>>();
    let land_tiles = tiles.clone();
    for chunk in tiles.chunks(8) {
        let chunk = chunk.to_vec();
        jobs.push(Box::new(move || {
            let tile = TILE as f32 / land_scale;
            let pad = LAND_PAD as f32 / land_scale;
            chunk
                .into_iter()
                .filter_map(|(column, row)| {
                    if let Some(at) = same.and_then(|same| same.of.land.get(&(column, row))) {
                        let same = same?;
                        return at.map(|at| Piece::Land((column, row), same.land[at].clone()));
                    }
                    let pixmap = paint_land_tile(frame, column, row, land_scale)?;
                    let origin = (column as f32 * tile - pad, row as f32 * tile - pad);
                    let mut sprite = Sprite::whole(pixmap, origin, land_scale);
                    sprite.clip = Some((
                        column as f32 * tile,
                        row as f32 * tile,
                        // Exactly its own square: neighbours share an
                        // edge, which snaps to the same device pixel. Not
                        // on to the picture's edge (the display packs
                        // pictures side by side with no gap, and sampling
                        // the edge blends in its neighbour), and not over
                        // the next tile (a light fading in over itself
                        // twice there is a line): both were seams.
                        tile,
                        tile,
                    ));
                    Some(Piece::Land((column, row), sprite))
                })
                .collect()
        }));
    }
    // Each building: its shadow (soft, at the land's resolution), its lit
    // sprite, and in a lit hour the halo of its windows.
    let mut anchors = std::collections::HashMap::new();
    let reach_of = |w: f32, h: f32| w * 0.72 + h * 1.1;
    let held = frame
        .buildings
        .iter()
        .filter(|b| {
            whole && !b.moving() && cut.holds(b.x - reach_of(b.w, b.h), b.x + reach_of(b.w, b.h))
        })
        .collect::<Vec<_>>();
    // What each is, and in a lit hour what stands in front of it close
    // enough to be taken out of its halo.
    let keys = held
        .iter()
        .map(|b| (b.index, item_key(level, "building", b)))
        .collect::<HashMap<_, _>>();
    let fronts = held
        .iter()
        .map(|a| {
            let halo = halo_reach(a, scale);
            let front = held
                .iter()
                .filter(|b| {
                    light.lit()
                        && (b.base, b.index) > (a.base, a.index)
                        && meets(&halo, &body_of(b))
                })
                .map(|b| b.index)
                .collect::<Vec<_>>();
            (a.index, front)
        })
        .collect::<HashMap<_, _>>();
    let front_key = |item: usize| -> u64 {
        let mut key = Key::new("in front");
        for other in &fronts[&item] {
            key.add((other, keys[other]));
        }
        key.finish()
    };
    let mut items = HashMap::new();
    for b in &held {
        items.insert(b.index, (keys[&b.index], front_key(b.index)));
    }
    let taken = |item: usize| {
        before.filter(|look| {
            look.of.items.get(&item) == items.get(&item) && items.contains_key(&item)
        })
    };
    // A building painted afresh has its halo taken out behind whatever
    // stands in front of it: those taken from before are painted again
    // for their shapes alone.
    let fresh = held
        .iter()
        .filter(|b| taken(b.index).is_none())
        .map(|b| b.index)
        .collect::<HashSet<_>>();
    let masks = fresh
        .iter()
        .flat_map(|item| fronts[item].iter().copied())
        .filter(|item| !fresh.contains(item))
        .collect::<HashSet<_>>();
    for building in held {
        anchors.insert(
            building.index,
            Anchor {
                x: building.x,
                base: building.base,
                w: building.w,
                h: building.h,
                flat: false,
            },
        );
        let item = building.index;
        let at = (building.base, item);
        if let Some(look) = taken(item) {
            let mask = masks.contains(&item);
            jobs.push(Box::new(move || {
                let mut out = look
                    .shadows_of(item)
                    .map(|sprite| Piece::Shadow(item, sprite.clone()))
                    .collect::<Vec<_>>();
                out.extend(
                    look.halos(item)
                        .map(|(group, halo)| Piece::Halo(at, group, halo.clone())),
                );
                out.extend(
                    look.building_sprites(item)
                        .map(|sprite| Piece::Building(at, sprite.clone(), None)),
                );
                if mask {
                    let (sprite, _) = sprite_and_glass(frame, building, scale);
                    if let Some((pixmap, (dx, dy))) = crop(sprite.pixmap) {
                        let origin = (
                            sprite.origin.0 + dx as f32 / scale,
                            sprite.origin.1 + dy as f32 / scale,
                        );
                        out.push(Piece::Mask(
                            at,
                            Raw {
                                pixmap,
                                origin,
                                scale,
                            },
                        ));
                    }
                }
                out
            }));
            continue;
        }
        jobs.push(Box::new(move || {
            let mut out = Vec::new();
            let (w, h) = (building.w, building.h);
            let reach = reach_of(w, h);
            let (left, top) = (building.x - reach, building.base - h * 0.2);
            if let Some(mut canvas) = Canvas::new(
                (reach * 2.0 * land_scale).ceil() as u32,
                (h * 0.5 * land_scale).ceil() as u32,
                land_scale,
                (left, top),
            ) {
                paint_building_shadow(&mut canvas, frame, building);
                if let Some(sprite) = Sprite::cropped(canvas.pixmap, (left, top), land_scale) {
                    out.push(Piece::Shadow(item, sprite));
                }
            }
            let (sprite, glass) = sprite_and_glass(frame, building, scale);
            if let Some(glass) = glass.as_deref().filter(|_| light.lit()) {
                if let Some(halo) =
                    halo(&sprite.pixmap, glass, sprite.origin, scale, building, frame)
                {
                    out.push(Piece::Glow(at, item % GLOW_GROUPS, halo));
                }
            }
            if let Some((pixmap, (dx, dy))) = crop(sprite.pixmap) {
                let origin = (
                    sprite.origin.0 + dx as f32 / scale,
                    sprite.origin.1 + dy as f32 / scale,
                );
                let raw = light.lit().then(|| Raw {
                    pixmap: pixmap.clone(),
                    origin,
                    scale,
                });
                out.push(Piece::Building(
                    at,
                    Sprite::whole(pixmap, origin, scale),
                    raw,
                ));
            }
            out
        }));
    }
    // Things, lit as the live layer lights them.
    for thing in frame
        .things
        .iter()
        .filter(|t| whole && cut.holds(t.x - t.w, t.x + t.w))
    {
        anchors.insert(
            thing.index,
            Anchor {
                x: thing.x,
                base: thing.base,
                w: thing.w,
                h: 0.0,
                flat: lies_flat(thing.shape, thing.base, frame.front),
            },
        );
        let item = thing.index;
        let key = (item_key(level, "thing", thing), 0);
        items.insert(item, key);
        let at = (thing.base, item);
        if let Some(look) = before.filter(|look| look.of.items.get(&item) == Some(&key)) {
            jobs.push(Box::new(move || {
                look.of
                    .of(item, 2)
                    .iter()
                    .map(|n| {
                        let (_, boat, sprite) = &look.things[*n];
                        Piece::Thing(at, *boat, sprite.clone())
                    })
                    .collect()
            }));
            continue;
        }
        jobs.push(Box::new(move || {
            bake_thing(frame, thing, scale, tint)
                .map(|sprite| {
                    let boat = thing.shape == MarkShape::Boat;
                    Piece::Thing(at, boat, sprite)
                })
                .into_iter()
                .collect()
        }));
    }
    // What stands along the spine, behind the things and in front of
    // everyone, in tiles of the stage (empty ones dropped).
    // As high as a beacon's lamp and its halo, on its tower.
    let beacon = frame
        .buildings
        .iter()
        .filter(|b| is_beacon(b))
        .map(|b| b.base - b.h * 0.815 - frame.figure_h * 2.0)
        .fold(f32::INFINITY, f32::min);
    let top = (frame.feet - frame.figure_h * 3.2)
        .min(frame.base)
        .min(beacon)
        .max(0.0);
    let side = 384.0 / scale;
    let columns = (frame.width / side).ceil() as i32;
    let rows = ((vh - top) / side).ceil() as i32;
    let prop_columns = (0..columns)
        .filter(|column| whole && cut.holds(*column as f32 * side, (*column + 1) as f32 * side))
        .collect::<Vec<_>>();
    for &column in &prop_columns {
        if let Some((same, (back, front))) =
            same.and_then(|same| Some((same, same.of.props.get(&column)?)))
        {
            jobs.push(Box::new(move || {
                back.iter()
                    .map(|at| Piece::Back(column, same.back[*at].clone()))
                    .chain(
                        front
                            .iter()
                            .map(|at| Piece::Front(column, same.front[*at].clone())),
                    )
                    .collect()
            }));
            continue;
        }
        jobs.push(Box::new(move || {
            let mut out = Vec::new();
            for row in 0..rows {
                let origin = (column as f32 * side, top + row as f32 * side);
                let span = (origin.0, origin.0 + side);
                for front in [false, true] {
                    let Some(mut canvas) = Canvas::new(384, 384, scale, origin) else {
                        continue;
                    };
                    // The live layer's, softened as a window softens.
                    canvas.window_soft = true;
                    paint_props(&mut canvas, frame, span, front, tint, vh);
                    if let Some(sprite) = Sprite::cropped(canvas.pixmap, origin, scale) {
                        out.push(if front {
                            Piece::Front(column, sprite)
                        } else {
                            Piece::Back(column, sprite)
                        });
                    }
                }
            }
            out
        }));
    }
    // People: every resident standing each way, and the first few walking.
    let mut nth = 0;
    for (spot, _) in stage.people.iter().zip(living).filter(|_| cut.people) {
        let Some(person) = frame.people.iter().find(|p| p.index == spot.index) else {
            continue;
        };
        let look = nth;
        nth += 1;
        let walks = look < WALKING_LOOKS;
        jobs.push(Box::new(move || {
            vec![Piece::Person(
                look,
                Box::new(bake_poses(frame, person, scale, tint, walks)),
            )]
        }));
    }
    // A chimney's puff through its life, and a glint on the water.
    if let Some(same) = same.filter(|_| whole) {
        jobs.push(Box::new(move || {
            let mut out = vec![Piece::Puffs(same.puffs.clone())];
            out.extend(same.glint.clone().map(Piece::Glint));
            out.extend(same.water.clone().map(Piece::Water));
            out
        }));
    } else if whole {
        jobs.push(Box::new(move || {
            let night = frame.daylight == Daylight::Night;
            let smoke = if night {
                art::hex(0x8a90a0)
            } else {
                art::hex(0xcfcbc6)
            };
            let w = frame.building_h.max(1.0) * 0.8;
            let puffs = (0..PUFF_STAGES)
                .filter_map(|stage| {
                    let age = (stage as f32 + 0.5) / PUFF_STAGES as f32;
                    let r = w * (0.045 + age * 0.08);
                    let room = r * 1.8;
                    let mut canvas = Canvas::new(
                        (room * 2.0 * scale).ceil() as u32,
                        (room * 2.0 * scale).ceil() as u32,
                        scale,
                        (-room, -room),
                    )?;
                    canvas.window_soft = true;
                    canvas.soft(
                        0.0,
                        0.0,
                        r,
                        r * 0.85,
                        r * 0.5,
                        // Softened as a window softens, a puff spreads thinner
                        // than the painter's: at night, against the dark,
                        // that reads as fainter smoke, so it is denser there.
                        smoke.opacity(
                            (0.6 * if night { 1.35 } else { 1.0 })
                                * (1.0 - age)
                                * (0.3 + 0.7 * (age * 6.0).min(1.0)),
                        ),
                    );
                    Some(Sprite::whole(canvas.pixmap, (-room, -room), scale))
                })
                .collect::<Vec<_>>();
            let mut out = vec![Piece::Puffs(puffs)];
            let shimmer = gpui::white().opacity(if night { 0.12 } else { 0.3 });
            if let Some(mut canvas) = Canvas::new(
                (40.0 * scale).ceil() as u32,
                (6.0 * scale).ceil() as u32,
                scale,
                (-20.0, -3.0),
            ) {
                canvas.window_soft = true;
                canvas.soft(0.0, 0.0, 13.0, 1.2, 1.5, shimmer);
                out.push(Piece::Glint(Sprite::whole(
                    canvas.pixmap,
                    (-20.0, -3.0),
                    scale,
                )));
            }
            // The light on the water: the same all the way across, so a strip
            // a few pixels wide, at the land's resolution.
            let deep = vh - frame.front;
            if frame.water && deep > 1.0 {
                let s = land_scale.max(0.5);
                let wide = WATER_STRIP as f32 / s;
                if let Some(mut canvas) =
                    Canvas::new(WATER_STRIP, (deep * s).ceil() as u32, s, (0.0, frame.front))
                {
                    if paint_dusk_water(&mut canvas, frame, -wide, wide * 3.0, frame.front, vh)
                        > 0.0
                    {
                        out.push(Piece::Water(Sprite::whole(
                            canvas.pixmap,
                            (0.0, frame.front),
                            s,
                        )));
                    }
                }
            }
            out
        }));
    }
    let mut look = Look {
        light,
        scale,
        sky: Vec::new(),
        hills: Vec::new(),
        land: Vec::new(),
        shadows: Vec::new(),
        back: Vec::new(),
        buildings: Vec::new(),
        glow: Vec::new(),
        things: Vec::new(),
        front: Vec::new(),
        people: Vec::new(),
        puffs: Vec::new(),
        glint: None,
        water: None,
        cut,
        anchors,
        of: Pieces {
            world,
            level,
            items,
            ..Pieces::default()
        },
    };
    let mut people = Vec::new();
    let mut buildings = Vec::new();
    let mut things = Vec::new();
    let mut solid = Vec::new();
    let mut halos = Vec::new();
    let mut shadows_of = Vec::new();
    for piece in across_cores(jobs).into_iter().flatten() {
        match piece {
            Piece::Sky(sprite) => look.sky.push(sprite),
            Piece::Hills(sprite) => look.hills.push(sprite),
            Piece::Land(at, sprite) => {
                look.of.land.insert(at, Some(look.land.len()));
                look.land.push(sprite);
            }
            Piece::Shadow(item, sprite) => {
                shadows_of.push(item);
                look.shadows.push(sprite);
            }
            Piece::Back(column, sprite) => {
                look.of
                    .props
                    .entry(column)
                    .or_default()
                    .0
                    .push(look.back.len());
                look.back.push(sprite);
            }
            Piece::Building(at, sprite, raw) => {
                if let Some(raw) = raw {
                    solid.push((at, raw));
                }
                buildings.push((at, sprite));
            }
            Piece::Mask(at, raw) => solid.push((at, raw)),
            Piece::Glow(at, group, raw) => halos.push((at, group, Err(raw))),
            Piece::Halo(at, group, sprite) => halos.push((at, group, Ok(sprite))),
            Piece::Thing(at, boat, sprite) => things.push((at, boat, sprite)),
            Piece::Front(column, sprite) => {
                look.of
                    .props
                    .entry(column)
                    .or_default()
                    .1
                    .push(look.front.len());
                look.front.push(sprite);
            }
            Piece::Person(at, poses) => people.push((at, *poses)),
            Piece::Puffs(puffs) => look.puffs = puffs,
            Piece::Glint(sprite) => look.glint = Some(sprite),
            Piece::Water(sprite) => look.water = Some(sprite),
        }
    }
    people.sort_by_key(|(at, _)| *at);
    look.people = people.into_iter().map(|(_, poses)| poses).collect();
    // Back to front: what stands further back first, in the same order in
    // every light (by where each stands, never by its picture's size).
    let order = |a: &(f32, usize), b: &(f32, usize)| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1));
    // A halo is drawn over every building, so what stands in front of its
    // own is taken out of it: a lit window behind a stall's roof never
    // shines through the roof.
    look.glow = halos
        .into_iter()
        .filter_map(|(at, group, halo)| {
            let mut halo = match halo {
                Ok(done) => return Some((group, at.1, done)),
                Err(raw) => raw,
            };
            for (_, front) in solid
                .iter()
                .filter(|(other, _)| order(other, &at) == std::cmp::Ordering::Greater)
            {
                halo.hide_behind(front);
            }
            Some((
                group,
                at.1,
                Sprite::cropped(halo.pixmap, halo.origin, halo.scale)?,
            ))
        })
        .collect();
    buildings.sort_by(|a, b| order(&a.0, &b.0));
    things.sort_by(|a, b| order(&a.0, &b.0));
    look.buildings = buildings
        .into_iter()
        .map(|((_, item), sprite)| (item, sprite))
        .collect();
    look.things = things
        .into_iter()
        .map(|((_, item), boat, sprite)| (item, boat, sprite))
        .collect();
    // Where each piece went: every tile and column tried (one that painted
    // nothing too), and each item's sprites.
    for tile in land_tiles {
        look.of.land.entry(tile).or_insert(None);
    }
    for column in prop_columns {
        look.of.props.entry(column).or_default();
    }
    let by_item = &mut look.of.by_item;
    for (at, (item, _)) in look.buildings.iter().enumerate() {
        by_item.entry(*item).or_default()[0].push(at);
    }
    for (at, (_, item, _)) in look.glow.iter().enumerate() {
        by_item.entry(*item).or_default()[1].push(at);
    }
    for (at, (item, _, _)) in look.things.iter().enumerate() {
        by_item.entry(*item).or_default()[2].push(at);
    }
    for (at, item) in shadows_of.into_iter().enumerate() {
        by_item.entry(item).or_default()[3].push(at);
    }
    look
}

/// What of a frame a building's or thing's own sprite is painted from
/// besides itself, at `scale` in `light`, as a key.
fn level_key(frame: &Frame, light: Light, scale: f32) -> u64 {
    let s = frame.scenery;
    let mut key = Key::new("living-stage level");
    key.add(light as u8)
        .float(scale)
        .add((s.sky_top, s.sky_bottom, s.far, s.near, s.sun))
        .add(format!("{:?}", frame.look))
        .add(frame.daylight as u8)
        .float(frame.hour)
        .float(frame.seconds)
        .add(frame.still)
        .add(frame.weather as u8)
        .add(frame.season.map(|season| season as u8))
        .add(frame.cover.map(|cover| cover as u8))
        .add((frame.ice, frame.water, frame.blend_top))
        .add(frame.setting as u8);
    for line in [
        frame.view_w,
        frame.height,
        frame.horizon,
        frame.base,
        frame.feet,
        frame.figure_h,
        frame.front,
        frame.building_h,
    ] {
        key.float(line);
    }
    key.finish()
}

/// A building's or thing's paint, all of it, as a key.
fn item_key(level: u64, kind: &str, paint: &impl std::fmt::Debug) -> u64 {
    let mut key = Key::new("living-stage item");
    key.add(level).add(kind).add(format!("{paint:?}"));
    key.finish()
}

/// Where a building's halo can reach, at most: its sprite, padded as the
/// halo pads it, and the pool of light below.
fn halo_reach(b: &BuildingPaint, scale: f32) -> Rect {
    let pad = b.w * 0.2 + 24.0 / scale.max(0.25) + 2.0;
    let (x0, y0) = (b.x - b.w * 0.72 - pad, b.base - b.h * 1.28 - pad);
    let (x1, y1) = (b.x + b.w * 0.72 + pad, b.base + b.h * 0.34 + pad);
    (x0, y0, x1 - x0, y1 - y0)
}

/// Where a building's own pixels can be: its sprite's box.
fn body_of(b: &BuildingPaint) -> Rect {
    let (x0, y0) = (b.x - b.w * 0.72 - 1.0, b.base - b.h * 1.28 - 1.0);
    (x0, y0, b.w * 1.44 + 2.0, b.h * 1.32 + 2.0)
}

/// The halo of a building's lit windows: its lamplight found in its
/// sprite, spread and softened past the glass, with a pool of it on the
/// ground before it.
fn halo(
    pixmap: &sk::Pixmap,
    glass: &[f32],
    origin: (f32, f32),
    scale: f32,
    building: &BuildingPaint,
    frame: &Frame,
) -> Option<Raw> {
    // At half the sprite's resolution: a halo is soft.
    let k = 2usize;
    let (sw, sh) = (pixmap.width() as usize, pixmap.height() as usize);
    let radius = ((building.w * 0.06 * scale) / k as f32).max(2.0) as usize;
    let pad = radius * 3;
    let (w, h) = (sw / k + pad * 2, sh / k + pad * 2);
    let mut mask = vec![0.0_f32; w * h];
    let mut lit = 0.0;
    // Only the glass the painter found lit, never a warm board or a
    // sunlit roof that happens to be the colour of lamplight.
    for (index, warm) in glass.iter().enumerate().take(sw * sh) {
        let warm = warm.clamp(0.0, 1.0);
        if warm <= 0.0 {
            continue;
        }
        let (x, y) = (index % sw, index / sw);
        let at = (y / k + pad) * w + x / k + pad;
        mask[at] += warm / (k * k) as f32;
        lit += warm;
    }
    if lit < 4.0 {
        return None;
    }
    painter::blur_mask(&mut mask, w, h, radius);
    let glow: gpui::Rgba = if frame.look.as_ref().is_some_and(|look| look.glow.is_some()) {
        frame.glow().into()
    } else {
        gpui::Rgba {
            r: 1.0,
            g: 0.78,
            b: 0.42,
            a: 1.0,
        }
    };
    let hs = scale / k as f32;
    let left = origin.0 - pad as f32 / hs;
    let top = origin.1 - pad as f32 / hs;
    let mut canvas = Canvas::new(
        w as u32,
        h as u32 + (building.h * 0.3 * hs) as u32,
        hs,
        (left, top),
    )?;
    // Over the building's own walls and roof only a little of it shows:
    // it is light from the glass, not a lamp hung in front of the house.
    let pixels = pixmap.pixels();
    let solid = |at: usize| -> bool {
        let (mx, my) = (at % w, at / w);
        let (Some(x), Some(y)) = (mx.checked_sub(pad), my.checked_sub(pad)) else {
            return false;
        };
        let (x, y) = (x * k + k / 2, y * k + k / 2);
        x < sw
            && y < sh
            && pixels[y * sw + x].alpha() > 200
            && glass.get(y * sw + x).is_none_or(|lit| *lit < 0.5)
    };
    let data = canvas.pixmap.data_mut();
    for (at, value) in mask.iter().enumerate() {
        let over = if solid(at) { 0.35 } else { 1.0 };
        let alpha = (value * 2.2).min(1.0) * 0.5 * over;
        if alpha < 0.004 {
            continue;
        }
        let a = (alpha * 255.0).round() as u8;
        let pre = |channel: f32| (channel * alpha * 255.0).round().min(a as f32) as u8;
        data[at * 4] = pre(glow.r);
        data[at * 4 + 1] = pre(glow.g);
        data[at * 4 + 2] = pre(glow.b);
        data[at * 4 + 3] = a;
    }
    // A pool of the light on the ground before it, as much as it has lit.
    let share = (lit / (sw * sh) as f32 * 12.0).clamp(0.15, 1.0);
    let pool = Hsla::from(glow).opacity(0.22 * share);
    canvas.soft(
        building.x,
        building.base + building.h * 0.02,
        building.w * 0.7,
        building.h * 0.14,
        building.h * 0.22,
        pool,
    );
    Some(Raw {
        pixmap: canvas.pixmap,
        origin: (left, top),
        scale: hs,
    })
}

/// Pixels not yet made a sprite: their top-left on the stage, and device
/// pixels a stage pixel.
struct Raw {
    pixmap: sk::Pixmap,
    origin: (f32, f32),
    scale: f32,
}

impl Raw {
    /// Its stage rectangle.
    fn rect(&self) -> Rect {
        (
            self.origin.0,
            self.origin.1,
            self.pixmap.width() as f32 / self.scale,
            self.pixmap.height() as f32 / self.scale,
        )
    }

    /// How much `front` covers the stage point (`x`, `y`), 0 to 1.
    fn cover(&self, x: f32, y: f32) -> f32 {
        let (px, py) = (
            ((x - self.origin.0) * self.scale).floor(),
            ((y - self.origin.1) * self.scale).floor(),
        );
        if px < 0.0 || py < 0.0 {
            return 0.0;
        }
        let (px, py) = (px as u32, py as u32);
        self.pixmap
            .pixel(px, py)
            .map_or(0.0, |pixel| pixel.alpha() as f32 / 255.0)
    }

    /// Takes out of these pixels whatever `front` covers.
    fn hide_behind(&mut self, front: &Raw) {
        let (a, b) = (self.rect(), front.rect());
        if a.0 >= b.0 + b.2 || b.0 >= a.0 + a.2 || a.1 >= b.1 + b.3 || b.1 >= a.1 + a.3 {
            return;
        }
        let width = self.pixmap.width() as usize;
        let (origin, scale) = (self.origin, self.scale);
        for (at, pixel) in self.pixmap.pixels_mut().iter_mut().enumerate() {
            if pixel.alpha() == 0 {
                continue;
            }
            let x = origin.0 + ((at % width) as f32 + 0.5) / scale;
            let y = origin.1 + ((at / width) as f32 + 0.5) / scale;
            let keep = 1.0 - front.cover(x, y);
            if keep >= 1.0 {
                continue;
            }
            let k = |channel: u8| (channel as f32 * keep).round() as u8;
            if let Some(kept) = sk::PremultipliedColorU8::from_rgba(
                k(pixel.red()),
                k(pixel.green()),
                k(pixel.blue()),
                k(pixel.alpha()),
            ) {
                *pixel = kept;
            }
        }
    }
}

/// A thing as the live layer draws it, lit by `tint`, with its contact
/// shadow.
fn bake_thing(frame: &Frame, thing: &ThingPaint, scale: f32, tint: [f32; 3]) -> Option<Sprite> {
    let w = thing.w;
    let tall = thing
        .drawing
        .as_ref()
        .map(|drawing| w / drawing.aspect.max(0.05))
        .unwrap_or((w * 1.6).max(frame.figure_h * 2.0));
    let (left, top) = (thing.x - w * 0.95, thing.base - tall * 1.2 - w * 0.2);
    let (right, bottom) = (thing.x + w * 0.95, thing.base + w * 0.3);
    let mut canvas = Canvas::new(
        ((right - left) * scale).ceil() as u32,
        ((bottom - top) * scale).ceil() as u32,
        scale,
        (left, top),
    )?;
    canvas.window_soft = true;
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    let (x, base) = (thing.x, thing.base);
    if let Some(glow) = thing.glow {
        canvas.soft(
            x,
            base,
            w * 0.8,
            w * 0.16,
            w * 0.12,
            ground_glow(glow, 0.35),
        );
    }
    if thing.shape == MarkShape::Boat {
        let [r, g, b] = shadow_ink(frame.hour);
        let contact = Hsla::from(gpui::Rgba { r, g, b, a: 1.0 }).opacity(0.25);
        canvas.soft(
            x,
            base + w * 0.04,
            w * 0.5,
            w * 0.06,
            w * 0.06,
            contact.opacity(0.12),
        );
    } else {
        contact_shadow(&mut canvas, frame.hour, x, base, w * 0.9);
    }
    let mut tinted = Tint::new(&mut canvas, tint);
    let mut under = Under {
        inner: &mut tinted,
        most: if lit {
            1.0
        } else {
            sky_luma(&frame.scenery) * WALLS_UNDER_SKY
        },
    };
    let mut muted = Mute {
        inner: &mut under,
        toward: art::hex(frame.setting.place_paints().neutral()),
        share: if thing.muted { 0.7 } else { 0.0 },
    };
    let mut facing = Xform::about(
        &mut muted,
        (x, base),
        if thing.flip { -1.0 } else { 1.0 },
        1.0,
        0.0,
    );
    match &thing.drawing {
        Some(drawing) => art::paint_drawing(
            &mut facing,
            x,
            base,
            w,
            w / drawing.aspect,
            drawing,
            &Inks::of_place(&thing.palette),
            Stance::Standing,
            world_projection::Mood::Content,
            thing.sway * 0.3,
            0.0,
            1.0,
        ),
        None => art::paint_thing(&mut facing, x, base, w, thing.shape, &thing.palette, 0.0),
    }
    Sprite::cropped(canvas.pixmap, (left, top), scale)
}

/// What stands along the spine over the stage span `span`: behind the
/// things (plots staked out, what a lived-in place has about it, the
/// lit windows' light on the water, lamps, jetties, a street's poles and
/// wires) or in front of everyone (the framing strip, bunting, the
/// beacon's lamp). `bottom` is the foot of the stage.
fn paint_props(
    canvas: &mut Canvas,
    frame: &Frame,
    span: (f32, f32),
    front: bool,
    tint: [f32; 3],
    bottom: f32,
) {
    let screen = |x: f32, y: f32| (x, y);
    // Whatever reaches into the span at all, its light and halo too: a
    // lamp just past the edge still throws its halo and its pool over it,
    // so neither is ever cut off where one tile ends.
    let margin = frame.figure_h * 2.5;
    let seen = |x: f32, reach: f32| {
        x + reach + margin >= span.0 - 4.0 && x - reach - margin <= span.1 + 4.0
    };
    if front {
        paint_beacons(canvas, frame, &screen, &seen);
        paint_garlands(canvas, frame, &screen, tint);
        paint_framing_strip(
            canvas,
            frame,
            span.0 - 8.0,
            span.1 - span.0 + 16.0,
            &screen,
            tint,
        );
    } else {
        // The lit windows' light broken on the water (still: the stage's
        // glints move over it).
        paint_window_reflections(canvas, frame, &screen, &seen, bottom, 0.0);
        // Plots staked out on the ground, as the painter has them close up.
        let opacity = mark::plot_opacity(1.0);
        for plot in frame.plots.iter().filter(|plot| seen(plot.x, plot.w)) {
            mark::paint_plot(
                &mut Tint::new(canvas, tint),
                plot.x,
                plot.y,
                plot.w,
                opacity,
                false,
                plot.plot as f32,
            );
        }
        // A place lived in: washing, a cart, rowboats moored.
        super::lived::paint_lived_in(canvas, frame, &screen, &seen, tint);
        // The painter's lamps, halos and all: softened as a window
        // softens (the canvas's `window_soft`), each halo is the
        // painter's pinpoint glow, never a disc.
        paint_spine_lamps(canvas, frame, &screen, &seen, tint);
        paint_jetties(canvas, frame, &screen, &seen, tint);
        let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
        crate::setting::paint_street_life(
            &mut Tint::new(canvas, tint),
            &screen,
            frame.setting,
            frame.width,
            frame.view_w,
            frame.base,
            frame.front,
            frame.building_h,
            1.0,
            lit,
            seed_of_scenery(&frame.scenery),
            (span.0 - 8.0, span.1 + 8.0),
            (&street_lamps(frame), street_glow(frame)),
        );
    }
}

/// One resident's poses: standing each way, and (if `walks`) a walk cycle
/// each way, each with their contact shadow, for a grown-up `figure_h`
/// tall.
fn bake_poses(
    frame: &Frame,
    person: &PersonPaint,
    scale: f32,
    tint: [f32; 3],
    walks: bool,
) -> Poses {
    let height = frame.figure_h * person.figure.age.height();
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    let [r, g, b] = shadow_ink(frame.hour);
    let night = frame.daylight == Daylight::Night;
    let contact =
        Hsla::from(gpui::Rgba { r, g, b, a: 1.0 }).opacity(if night { 0.3 } else { 0.25 });
    let paint = |pose: Pose, stance: Stance| -> Option<Sprite> {
        let room = height * 0.75;
        let (left, top) = (-room, -height * 1.35);
        let mut canvas = Canvas::new(
            (room * 2.0 * scale).ceil() as u32,
            (height * 1.5 * scale).ceil() as u32,
            scale,
            (left, top),
        )?;
        canvas.window_soft = true;
        canvas.soft(
            0.0,
            0.0,
            height * 0.2,
            height * 0.045,
            height * 0.05,
            contact,
        );
        let mut lit_by = Tint::new(&mut canvas, tint);
        let mut tinted = Under {
            inner: &mut lit_by,
            most: if lit {
                1.0
            } else {
                sky_luma(&frame.scenery) * WALLS_UNDER_SKY
            },
        };
        // Someone sitting sits on something, as the painter has them.
        if stance == Stance::Sitting && !person.figure.bird {
            crate::setting::paint_seat(&mut tinted, frame.setting, 0.0, 0.0, height, pose.facing);
        }
        age::paint_person(
            &mut tinted,
            0.0,
            0.0,
            height,
            &person.figure,
            person.drawing.as_ref(),
            stance,
            person.mood,
            pose,
        );
        Sprite::cropped(canvas.pixmap, (left, top), scale)
    };
    let tau = std::f32::consts::TAU;
    // Where they are, as they are there: standing, or sitting on a seat.
    let still = if person.stance == Stance::Sitting {
        Stance::Sitting
    } else {
        Stance::Standing
    };
    let stand = [-1.0_f32, 1.0].map(|facing| {
        paint(
            Pose {
                facing,
                ..Pose::default()
            },
            still,
        )
    });
    let walk = [-1.0_f32, 1.0].map(|facing| {
        if !walks {
            return Vec::new();
        }
        (0..WALK_FRAMES)
            .filter_map(|step| {
                let phase = step as f32 / WALK_FRAMES as f32;
                paint(
                    Pose {
                        stride: Some(phase),
                        bob: (phase * tau).sin().abs() * height * 0.035,
                        facing,
                        squash: 1.0 - 0.03 * (phase * tau * 2.0).cos().max(0.0),
                        lean: facing * 0.018 * (phase * tau * 2.0).sin(),
                    },
                    Stance::Walking,
                )
            })
            .collect()
    });
    Poses { walk, stand }
}

/// Someone walking up and down the spine, for the benchmark and to show a
/// town that lives.
#[derive(Clone, Copy, Debug)]
pub struct Walker {
    /// Whose poses they walk in.
    pub look: usize,
    /// Where along the stage they started, and how fast they go (stage
    /// pixels a second; negative is leftward).
    pub from: f32,
    pub speed: f32,
    /// How far in front of or behind the spine's line their feet are.
    pub lane: f32,
    /// Where in their stride they started.
    pub phase: f32,
}

/// Everyone walking.
#[derive(Clone, Debug, Default)]
pub struct Walkers(pub Vec<Walker>);

impl Walkers {
    /// About `in_view` walkers in a window's width at zoom 1, spread along
    /// the whole stage, each in a resident's poses.
    pub fn spread(baked: &Baked, in_view: usize, seed: u32) -> Self {
        let looks = baked.walking_looks();
        if looks == 0 || in_view == 0 {
            return Self::default();
        }
        let count = (in_view as f32 * baked.width / baked.view.0.max(1.0)).ceil() as usize;
        let p = baked.figure_h;
        let spine = (baked.front - baked.feet).max(p * 0.2);
        let footprints = baked.footprints();
        Self(
            (0..count)
                .map(|n| {
                    let hash = |salt: u32| {
                        (painter::hash2(n as i32, salt as i32, seed) % 10_000) as f32 / 10_000.0
                    };
                    let pace = 0.75 + 0.5 * hash(1);
                    Walker {
                        look: n % looks,
                        from: hash(2) * baked.width,
                        speed: p * STROLL * pace * if hash(3) < 0.5 { -1.0 } else { 1.0 },
                        // A walker crosses the whole stage in their lane, so it
                        // keeps clear of every building's ground.
                        lane: free_lane(
                            &footprints,
                            baked.feet - p * 0.3 + (spine * 0.8 + p * 0.3) * hash(4),
                            (baked.feet - p * 0.3, baked.front),
                        ) - baked.feet,
                        phase: hash(5),
                    }
                })
                .collect(),
        )
    }

    /// Where `walker` is `seconds` in: x along the stage, facing (0 left,
    /// 1 right), and their step.
    fn at(walker: &Walker, seconds: f32, width: f32, p: f32) -> (f32, usize, usize) {
        let span = width.max(1.0);
        let travelled = walker.from + walker.speed * seconds;
        let lap = travelled.rem_euclid(span * 2.0);
        let (x, rightward) = if lap < span {
            (lap, walker.speed >= 0.0)
        } else {
            (span * 2.0 - lap, walker.speed < 0.0)
        };
        // A stride is about two thirds of a grown-up's height.
        let stride = p * STROLL / 1.7;
        let step = ((walker.speed.abs() * seconds / stride + walker.phase).rem_euclid(1.0)
            * WALK_FRAMES as f32) as usize;
        (x, usize::from(rightward), step.min(WALK_FRAMES - 1))
    }
}

/// The camera of the living stage: the stage point at the window's middle
/// and how close.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl View {
    /// The same view kept inside the stage.
    pub fn kept(self, baked: &Baked) -> Self {
        let (vw, vh) = baked.view;
        let zoom = self.zoom.max(1.0);
        let half_w = vw / zoom / 2.0;
        let half_h = vh / zoom / 2.0;
        Self {
            zoom,
            x: self.x.clamp(half_w, (baked.width - half_w).max(half_w)),
            y: self.y.clamp(half_h, (vh - half_h).max(half_h)),
        }
    }
}

/// What one frame of the living stage shows.
#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub view: View,
    pub seconds: f32,
    pub hour: f32,
    /// Device pixels a point.
    pub dpr: f32,
}

/// Something one frame draws: a baked picture where on screen (in
/// points, from the scene's corner) and the part of that shown, or
/// something drawn live within `rect` (a building rising into place, a
/// design on its cloth, the haze between a postcard's rows).
#[derive(Clone)]
pub enum Draw {
    Image {
        image: Arc<RenderImage>,
        rect: Rect,
        clip: Option<Rect>,
    },
    Live {
        rect: Rect,
        paint: Rc<LiveDraw>,
    },
}

/// Live drawing: the window, and the scene's top-left corner and size.
pub type LiveDraw = dyn Fn(&mut Window, Rect);

impl Draw {
    pub fn image(image: Arc<RenderImage>, rect: Rect, clip: Option<Rect>) -> Self {
        Draw::Image { image, rect, clip }
    }

    /// What of the screen it can touch.
    pub fn bounds(&self) -> Rect {
        match self {
            Draw::Image { rect, clip, .. } => clip.map_or(*rect, |clip| intersection(*rect, clip)),
            Draw::Live { rect, .. } => *rect,
        }
    }
}

/// Where two rectangles meet (nothing wide if they do not).
pub(crate) fn intersection(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.0.max(b.0), a.1.max(b.1));
    let (x1, y1) = ((a.0 + a.2).min(b.0 + b.2), (a.1 + a.3).min(b.1 + b.3));
    (x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

/// Whether two rectangles overlap.
fn meets(a: &Rect, b: &Rect) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Everything drawn together at one opacity: one pass.
#[derive(Clone, Default)]
pub struct Pass {
    pub opacity: f32,
    pub draws: Vec<Draw>,
}

/// Everything one frame draws, worked out before drawing.
#[derive(Clone, Default)]
pub struct Plan {
    pub layers: Vec<Pass>,
    /// How many images move this frame (people walking, boats, smoke,
    /// glints).
    pub moving: usize,
}

impl Plan {
    pub fn images(&self) -> usize {
        self.layers.iter().map(|layer| layer.draws.len()).sum()
    }
}

/// One picture of a frame, in painter's order, drawn at `opacity`.
#[derive(Clone)]
pub struct Item {
    pub opacity: f32,
    pub draw: Draw,
}

/// Puts `items`, in painter's order, into as few passes as keeps that
/// order (GPUI has no per-image opacity: a pass is a layer at one).
///
/// Each item goes into the first pass at its opacity that comes no
/// earlier than the last pass holding anything it overlaps: so whatever
/// it overlaps is drawn before it, exactly as if every item were drawn one
/// by one in order. Two lights cross-fading are then each picture drawn
/// whole in the first light and its second light faded straight over it,
/// before anything in front of it: nobody shows through whoever stands in
/// front of them, and pictures that do not overlap share their passes, so
/// a frame has as many passes as its crowd is deep, not as it is many.
pub fn compose(items: impl IntoIterator<Item = Item>) -> Vec<Pass> {
    // Each pass, and the box around everything in it.
    let mut passes: Vec<(Pass, Rect)> = Vec::new();
    for item in items {
        if item.opacity <= 0.001 {
            continue;
        }
        let rect = item.draw.bounds();
        if rect.2 <= 0.0 || rect.3 <= 0.0 {
            continue;
        }
        let floor = passes
            .iter()
            .rposition(|(pass, bounds)| {
                meets(bounds, &rect) && pass.draws.iter().any(|draw| meets(&draw.bounds(), &rect))
            })
            .unwrap_or(0);
        let at = passes[floor..]
            .iter()
            .position(|(pass, _)| pass.opacity == item.opacity)
            .map(|at| at + floor);
        match at {
            Some(at) => {
                let (pass, bounds) = &mut passes[at];
                *bounds = union(*bounds, rect);
                pass.draws.push(item.draw);
            }
            None => passes.push((
                Pass {
                    opacity: item.opacity,
                    draws: vec![item.draw],
                },
                rect,
            )),
        }
    }
    passes.into_iter().map(|(pass, _)| pass).collect()
}

/// The box around two rectangles.
fn union(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.0.min(b.0), a.1.min(b.1));
    let (x1, y1) = ((a.0 + a.2).max(b.0 + b.2), (a.1 + a.3).max(b.1 + b.3));
    (x0, y0, x1 - x0, y1 - y0)
}

/// What stands on the stage at a depth: the props along the spine, a
/// building, a thing or a person, with the pictures it draws (in both
/// lights, its glow, a design it wears).
pub struct Deep {
    /// Where its foot stands: further down is nearer.
    pub at: f32,
    /// Which comes first where two stand level: [`Rank`].
    pub rank: Rank,
    /// Who or what it is, so the same stage draws in the same order every
    /// frame and in every light.
    pub who: usize,
    pub items: Vec<Item>,
}

/// What a deep thing is, in the order things standing level are drawn:
/// the props along the spine, then buildings, things, people.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Rank {
    Props,
    Building,
    Thing,
    Person,
}

/// Everything standing on the stage, back to front: whatever stands
/// further back first, so a building in front of someone hides them and
/// someone in front of it stands over it (the S0 spike drew everyone over
/// every building: a walker on a tower's door).
pub fn in_depth(mut deep: Vec<Deep>) -> impl Iterator<Item = Item> {
    deep.sort_by(|a, b| {
        a.at.total_cmp(&b.at)
            .then(a.rank.cmp(&b.rank))
            .then(a.who.cmp(&b.who))
    });
    deep.into_iter().flat_map(|deep| deep.items)
}

/// A camera on the stage as a window of `view` sees it: what of the stage
/// it shows, and where a stage rectangle lands on screen.
#[derive(Clone, Copy, Debug)]
pub struct Lens {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub view: (f32, f32),
}

impl Lens {
    /// The stage it shows: left, top, right, bottom.
    pub fn seen(&self) -> (f32, f32, f32, f32) {
        let (vw, vh) = self.view;
        (
            self.x - vw / (2.0 * self.zoom),
            self.y - vh / (2.0 * self.zoom),
            self.x + vw / (2.0 * self.zoom),
            self.y + vh / (2.0 * self.zoom),
        )
    }

    /// Where a stage point lands on screen.
    pub fn at(&self, x: f32, y: f32) -> (f32, f32) {
        (
            (x - self.x) * self.zoom + self.view.0 / 2.0,
            (y - self.y) * self.zoom + self.view.1 / 2.0,
        )
    }

    /// Where a stage rectangle lands on screen, if it is in view.
    pub fn place(&self, rect: Rect) -> Option<Rect> {
        let (left, top, right, bottom) = self.seen();
        if rect.0 > right || rect.0 + rect.2 < left || rect.1 > bottom || rect.1 + rect.3 < top {
            return None;
        }
        let (x, y) = self.at(rect.0, rect.1);
        Some((x, y, rect.2 * self.zoom, rect.3 * self.zoom))
    }

    /// A sprite standing on the stage, drawn where it is, if in view.
    pub fn draw(&self, sprite: &Sprite) -> Option<Draw> {
        Some(Draw::image(
            sprite.image.clone(),
            self.place(sprite.rect)?,
            sprite.clip.and_then(|clip| self.place(clip)),
        ))
    }

    /// A sprite standing on the stage, moved by (`dx`, `dy`) stage pixels
    /// and grown `k` times about (`ax`, `ay`), if in view.
    pub fn draw_moved(
        &self,
        sprite: &Sprite,
        (dx, dy): (f32, f32),
        k: f32,
        (ax, ay): (f32, f32),
    ) -> Option<Draw> {
        let (x, y, w, h) = sprite.rect;
        let rect = (ax + (x - ax) * k + dx, ay + (y - ay) * k + dy, w * k, h * k);
        Some(Draw::image(sprite.image.clone(), self.place(rect)?, None))
    }
}

/// The far hills of `look`, in the band of a frame `baked`-like: they
/// move a share of the camera's pan, and stand on the horizon wherever the
/// camera has it.
pub(crate) fn hill_draws(
    look: &Look,
    band: (f32, f32, f32, f32),
    lens: &Lens,
    horizon: f32,
) -> Vec<Draw> {
    let (vw, _) = lens.view;
    let (_, _, pad, above) = band;
    let pan = lens.x - vw / 2.0;
    let bx = -pad - pan * HILLS;
    let squash = lens.zoom.clamp(0.45, 1.0);
    let by = lens.at(0.0, horizon).1 - above * squash;
    look.hills
        .iter()
        .filter_map(|sprite| {
            let x = bx + sprite.rect.0;
            (x < vw && x + sprite.rect.2 > 0.0).then(|| {
                Draw::image(
                    sprite.image.clone(),
                    (x, by + sprite.rect.1, sprite.rect.2, sprite.rect.3 * squash),
                    None,
                )
            })
        })
        .collect()
}

/// The land of `look` in view, the light on the water stretched across
/// it, and glints drifting on the water `seconds` in (counted in
/// `moving`).
fn land_draws(look: &Look, lens: &Lens, front: f32, seconds: f32, moving: &mut usize) -> Vec<Draw> {
    let mut out = look
        .land
        .iter()
        .filter_map(|sprite| lens.draw(sprite))
        .collect::<Vec<_>>();
    out.extend(water_draws(look, lens, front, seconds, moving));
    out
}

/// The light on the water of `look` stretched across the view, and
/// glints drifting on the water `seconds` in (counted in `moving`).
pub(crate) fn water_draws(
    look: &Look,
    lens: &Lens,
    front: f32,
    seconds: f32,
    moving: &mut usize,
) -> Vec<Draw> {
    let (left, _, right, _) = lens.seen();
    let vh = lens.view.1;
    let mut out = Vec::new();
    // The light on the water, stretched across the window: its middle
    // half only, so the strip's edges (which the display blends with
    // whatever picture it packed beside them) are stretched off the
    // window, never across it.
    if let Some(water) = &look.water {
        let (_, y, _, h) = water.rect;
        let span = right - left + 4.0;
        if let Some(rect) = lens.place((left - 2.0 - span * 0.5, y, span * 2.0, h)) {
            out.push(Draw::image(
                water.image.clone(),
                rect,
                lens.place((left - 2.0, y, span, h)),
            ));
        }
    }
    // Glints drifting on the water.
    if let Some(glint) = &look.glint {
        for n in 0..48 {
            let row = n / 8;
            let x = left
                + ((seconds * (6.0 + row as f32) + n as f32 * 137.0)
                    .rem_euclid(right - left + 80.0))
                - 40.0;
            let y = front + 14.0 + row as f32 * (vh - front) / 6.5;
            let pulse = 0.6 + 0.4 * (seconds * 1.1 + n as f32).sin();
            let (gx, gy, gw, gh) = glint.rect;
            if let Some(rect) = lens.place((x + gx * pulse, y + gy, gw * pulse, gh)) {
                *moving += 1;
                out.push(Draw::image(glint.image.clone(), rect, None));
            }
        }
    }
    out
}

/// Smoke from every chimney of `chimneys` (pot x, y and size): puffs
/// rising, spreading and thinning, carried by the wind `blow`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn smoke_draws(
    look: &Look,
    lens: &Lens,
    chimneys: &[(f32, f32, f32)],
    building_w: f32,
    blow: f32,
    seconds: f32,
    moving: &mut usize,
) -> Vec<Draw> {
    let mut out = Vec::new();
    if look.puffs.is_empty() || lens.zoom < 0.6 {
        return out;
    }
    let (left, _, right, _) = lens.seen();
    let h = building_w * 1.1;
    for (n, (px, py, size)) in chimneys.iter().enumerate() {
        if *px < left - h || *px > right + h {
            continue;
        }
        for puff in 0..5 {
            let age = (seconds * 0.3 + puff as f32 * 0.2 + n as f32 * 0.37) % 1.0;
            let rise = ease(age) * h * 0.55 * size;
            let drift = age * age * blow * building_w * 0.35 * size;
            let stage = ((age * PUFF_STAGES as f32) as usize).min(PUFF_STAGES - 1);
            let Some(sprite) = look.puffs.get(stage) else {
                continue;
            };
            let (x, y, w, hh) = sprite.rect;
            if let Some(rect) = lens.place((
                px + drift + x * size,
                py - rise + y * size,
                w * size,
                hh * size,
            )) {
                *moving += 1;
                out.push(Draw::image(sprite.image.clone(), rect, None));
            }
        }
    }
    out
}

/// How a building's lit windows flicker, by its group, `seconds` in.
pub fn flicker(group: usize, seconds: f32) -> f32 {
    let g = group as f32;
    0.86 + 0.14 * (seconds * (1.7 + g * 0.6) + g * 2.1).sin()
}

/// The ground a building stands on, as (left, back, right, front) in
/// stage pixels, for one standing at `x` with its foot at `base`, `w`
/// wide and `h` tall: its width, and back from its front a share of its
/// height. Nobody's feet are ever inside it: they would be inside the
/// building (the art director's S1 blocker, a walker on a tower's
/// door).
pub fn footprint(x: f32, base: f32, w: f32, h: f32) -> (f32, f32, f32, f32) {
    (x - w * 0.45, base - h * FOOTPRINT_DEPTH, x + w * 0.45, base)
}

/// How far back from its front a building's ground reaches, as a share of
/// its height.
pub const FOOTPRINT_DEPTH: f32 = 0.22;

/// Whether feet at (`x`, `y`) stand inside a footprint.
pub fn inside(footprint: (f32, f32, f32, f32), x: f32, y: f32) -> bool {
    let (left, back, right, front) = footprint;
    x > left && x < right && y > back && y < front
}

/// Works out one frame of the benchmark's living stage: every image it
/// draws, back to front, two lights cross-fading where the hour falls
/// between them, and everything standing on the stage drawn in depth.
pub fn plan(baked: &Baked, walkers: &Walkers, shot: &Shot) -> Plan {
    let view = shot.view.kept(baked);
    let level = baked.level_for(view.zoom * shot.dpr);
    let (a, b, t) = Light::blend(shot.hour);
    let first = baked
        .look(level, a)
        .or_else(|| baked.lights.first().and_then(|l| baked.look(level, *l)));
    let Some(first) = first else {
        return Plan::default();
    };
    let second = (t > 0.002 && b != a)
        .then(|| baked.look(level, b))
        .flatten();
    let (vw, vh) = baked.view;
    let lens = Lens {
        x: view.x,
        y: view.y,
        zoom: view.zoom,
        view: baked.view,
    };
    let (left, _, right, _) = lens.seen();
    let seconds = shot.seconds;
    let p = baked.figure_h;
    let mut moving = 0;
    // Each look and how much of it shows over what is before it.
    let looks = std::iter::once((first, 1.0))
        .chain(second.map(|second| (second, t)))
        .collect::<Vec<_>>();
    let mut items = Vec::new();
    let mut flat = |draws_of: &mut dyn FnMut(&Look) -> Vec<Draw>| {
        for (look, opacity) in &looks {
            items.extend(draws_of(look).into_iter().map(|draw| Item {
                opacity: *opacity,
                draw,
            }));
        }
    };
    flat(&mut |look| {
        look.sky
            .iter()
            .map(|sprite| Draw::image(sprite.image.clone(), (0.0, 0.0, vw, vh), None))
            .collect()
    });
    flat(&mut |look| hill_draws(look, baked.band, &lens, baked.horizon));
    let mut counted = false;
    flat(&mut |look| {
        let mut count = 0;
        let draws = land_draws(look, &lens, baked.front, seconds, &mut count);
        if !counted {
            moving += count;
            counted = true;
        }
        draws
    });
    flat(&mut |look| {
        look.shadows
            .iter()
            .filter_map(|sprite| lens.draw(sprite))
            .collect()
    });
    // Everything standing on the stage, in depth: the props along the
    // spine, buildings with their glow, things, residents and walkers.
    let mut deep = Vec::new();
    let both = |of: &dyn Fn(&Look) -> Vec<Draw>| -> Vec<Item> {
        looks
            .iter()
            .flat_map(|(look, opacity)| {
                of(look).into_iter().map(|draw| Item {
                    opacity: *opacity,
                    draw,
                })
            })
            .collect()
    };
    deep.push(Deep {
        // Half a figure behind the spine: whoever stands on the shore
        // stands over its lamps and poles.
        at: baked.feet - baked.figure_h * 0.5,
        rank: Rank::Props,
        who: 0,
        items: both(&|look| {
            look.back
                .iter()
                .filter_map(|sprite| lens.draw(sprite))
                .collect()
        }),
    });
    let shares = [1.0 - if second.is_some() { t } else { 0.0 }, t];
    for (item, sprite) in &first.buildings {
        let Some(anchor) = first.anchors.get(item) else {
            continue;
        };
        let mut items = both(&|look| {
            look.buildings
                .iter()
                .filter(|(other, _)| other == item)
                .filter_map(|(_, sprite)| lens.draw(sprite))
                .collect()
        });
        if items.is_empty() && lens.draw(sprite).is_none() {
            continue;
        }
        for (n, (look, _)) in looks.iter().enumerate() {
            for (group, _, halo) in look.glow.iter().filter(|(_, of, _)| of == item) {
                if let Some(draw) = lens.draw(halo) {
                    items.push(Item {
                        opacity: shares[n] * flicker(*group, seconds),
                        draw,
                    });
                }
            }
        }
        deep.push(Deep {
            at: anchor.base,
            rank: Rank::Building,
            who: *item,
            items,
        });
    }
    for (n, (item, boat, _)) in first.things.iter().enumerate() {
        let Some(anchor) = first.anchors.get(item) else {
            continue;
        };
        // Riding the swell.
        let (drift, bob) = if *boat {
            let seed = n as f32 * 1.37;
            (
                (seconds * 0.21 + seed).sin() * 3.0,
                (seconds * 1.3 + seed).sin() * 2.0 + (seconds * 0.7 + seed * 0.3).sin() * 1.2,
            )
        } else {
            (0.0, 0.0)
        };
        let items = both(&|look| {
            look.things
                .iter()
                .filter(|(other, _, _)| other == item)
                .filter_map(|(_, _, sprite)| lens.draw_moved(sprite, (drift, bob), 1.0, (0.0, 0.0)))
                .collect()
        });
        if *boat && !items.is_empty() {
            moving += 1;
        }
        deep.push(Deep {
            // What lies flat is ground: under everyone.
            at: if anchor.flat {
                f32::NEG_INFINITY
            } else {
                anchor.base + bob
            },
            rank: Rank::Thing,
            who: *item,
            items,
        });
    }
    // Residents where the stage has them, standing.
    for (n, resident) in baked.residents.iter().enumerate() {
        let k = resident.scale;
        let items = both(&|look| {
            look.people
                .get(resident.look)
                .and_then(|poses| poses.stand[resident.facing].as_ref())
                .and_then(|sprite| lens.draw_moved(sprite, (resident.x, resident.y), k, (0.0, 0.0)))
                .into_iter()
                .collect()
        });
        deep.push(Deep {
            at: resident.y,
            rank: Rank::Person,
            who: n,
            items,
        });
    }
    // Walkers, each in their lane.
    for (n, walker) in walkers.0.iter().enumerate() {
        let (x, facing, step) = Walkers::at(walker, seconds, baked.width, p);
        if x < left - p || x > right + p {
            continue;
        }
        let feet = baked.feet + walker.lane;
        let items = both(&|look| {
            look.people
                .get(walker.look)
                .and_then(|poses| poses.walk[facing].get(step))
                .and_then(|sprite| lens.draw_moved(sprite, (x, feet), 1.0, (0.0, 0.0)))
                .into_iter()
                .collect()
        });
        if !items.is_empty() {
            moving += 1;
        }
        deep.push(Deep {
            at: feet,
            rank: Rank::Person,
            who: baked.residents.len() + n,
            items,
        });
    }
    items.extend(in_depth(deep));
    let mut smoke = 0;
    for (look, opacity) in &looks {
        let mut count = 0;
        items.extend(
            smoke_draws(
                look,
                &lens,
                &baked.chimneys,
                baked.building_w,
                baked.blow,
                seconds,
                &mut count,
            )
            .into_iter()
            .map(|draw| Item {
                opacity: *opacity,
                draw,
            }),
        );
        smoke = smoke.max(count);
    }
    moving += smoke;
    for (look, opacity) in &looks {
        items.extend(
            look.front
                .iter()
                .filter_map(|sprite| lens.draw(sprite))
                .map(|draw| Item {
                    opacity: *opacity,
                    draw,
                }),
        );
    }
    Plan {
        layers: compose(items),
        moving,
    }
}

/// Draws one layer of a plan in `bounds`.
pub fn paint_layer(layer: &Pass, bounds: gpui::Bounds<gpui::Pixels>, window: &mut Window) {
    let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let at = |(x, y, w, h): Rect| Bounds::new(point(px(ox + x), px(oy + y)), size(px(w), px(h)));
    let scene = (
        ox,
        oy,
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
    );
    for draw in &layer.draws {
        match draw {
            Draw::Image { image, rect, clip } => {
                let clip = clip.map(at).unwrap_or(bounds).intersect(&bounds);
                let _ = window.paint_image(
                    clip,
                    at(*rect),
                    Corners::default(),
                    image.clone(),
                    0,
                    false,
                );
            }
            Draw::Live { paint, .. } => paint(window, scene),
        }
    }
}

/// A plan as GPUI elements: a layer each, at its opacity.
pub fn element(plan: Rc<Plan>) -> gpui::Div {
    let mut root = div().size_full().relative();
    for index in 0..plan.layers.len() {
        let opacity = plan.layers[index].opacity;
        let plan = plan.clone();
        let layer = gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window, _| paint_layer(&plan.layers[index], bounds, window),
        )
        .size_full();
        root = root.child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .opacity(opacity.min(1.0))
                .child(layer),
        );
    }
    root
}

/// How the benchmark's camera and light move: a slow pan across the whole
/// place and back, zooming in and out, through a day every forty seconds
/// (or held at one hour).
pub fn bench_shot(baked: &Baked, seconds: f32, dpr: f32, hour: Option<f32>) -> Shot {
    let (vw, vh) = baked.view;
    let zoom = 1.6 + 0.6 * (seconds * 0.31).sin();
    let half = vw / zoom / 2.0;
    let reach = (baked.width / 2.0 - half).max(0.0);
    let view = View {
        x: baked.width / 2.0 - reach * (seconds * 0.11).cos(),
        y: vh - vh / zoom / 2.0 - (baked.front - baked.feet) * 0.5,
        zoom,
    };
    Shot {
        view,
        seconds,
        hour: hour.unwrap_or(12.0 + seconds * 24.0 / 40.0),
        dpr,
    }
}

/// What the benchmark measured, frame by frame.
#[derive(Clone, Debug, Default)]
pub struct Stats {
    /// Each frame's plan: from the view starting to render to the last
    /// layer painted, in milliseconds.
    pub planned: Vec<f64>,
    /// The time between frames, in milliseconds.
    pub between: Vec<f64>,
    pub images: Vec<usize>,
    pub moving: Vec<usize>,
    pub last: Option<Instant>,
    pub started: Option<Instant>,
}

impl Stats {
    /// The v0.30 S1b bar for 120 Hz, as a line: each frame planned in 4 ms
    /// at the 99th percentile, and no interval between frames over 33 ms
    /// once the place first shows (the measured run starts there).
    pub fn bar(&self) -> String {
        let mut planned = self.planned.clone();
        planned.sort_by(f64::total_cmp);
        let p99 = planned
            .get(((planned.len().max(1) - 1) as f64 * 0.99).round() as usize)
            .copied()
            .unwrap_or(0.0);
        let hitches = self.between.iter().filter(|ms| **ms > 33.0).count();
        let met = |ok: bool| if ok { "met" } else { "NOT met" };
        format!(
            "120 Hz bar: plan p99 {p99:.3} ms (at most 4: {}) | intervals over 33 ms after first open {hitches} (none: {})",
            met(p99 <= 4.0),
            met(hitches == 0),
        )
    }

    /// A line of numbers for a log: plan mean, 95th percentile and worst,
    /// frames a second, and what each frame drew.
    pub fn summary(&self) -> String {
        let sorted = |values: &[f64]| {
            let mut values = values.to_vec();
            values.sort_by(f64::total_cmp);
            values
        };
        let percentile = |values: &[f64], p: f64| -> f64 {
            if values.is_empty() {
                return 0.0;
            }
            values[((values.len() - 1) as f64 * p).round() as usize]
        };
        let planned = sorted(&self.planned);
        let between = sorted(&self.between);
        let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len().max(1) as f64;
        let elapsed = match (self.started, self.last) {
            (Some(started), Some(last)) => (last - started).as_secs_f64(),
            _ => 0.0,
        };
        let fps = if elapsed > 0.0 {
            (self.planned.len().saturating_sub(1)) as f64 / elapsed
        } else {
            0.0
        };
        let images = self.images.iter().sum::<usize>() as f64 / self.images.len().max(1) as f64;
        let moving = self.moving.iter().sum::<usize>() as f64 / self.moving.len().max(1) as f64;
        let over = self.planned.iter().filter(|ms| **ms > 8.0).count();
        // A hitch: a frame the eye sees stop (two missed frames at 60 Hz),
        // and one it sees as a jolt.
        let hitches = self.between.iter().filter(|ms| **ms > 33.0).count();
        let jolts = self.between.iter().filter(|ms| **ms > 100.0).count();
        format!(
            "frames {} | plan ms mean {:.3} p50 {:.3} p95 {:.3} p99 {:.3} max {:.3} | over 8 ms {} | fps {:.1} | frame interval ms p50 {:.2} p95 {:.2} p99 {:.2} worst {:.2} | intervals over 33 ms {hitches}, over 100 ms {jolts} | images/frame mean {:.0} max {} | moving/frame mean {:.0} max {}",
            self.planned.len(),
            mean(&planned),
            percentile(&planned, 0.5),
            percentile(&planned, 0.95),
            percentile(&planned, 0.99),
            planned.last().copied().unwrap_or(0.0),
            over,
            fps,
            percentile(&between, 0.5),
            percentile(&between, 0.95),
            percentile(&between, 0.99),
            between.last().copied().unwrap_or(0.0),
            images,
            self.images.iter().max().copied().unwrap_or(0),
            moving,
            self.moving.iter().max().copied().unwrap_or(0),
        )
    }
}

/// Where the benchmark's time comes from: the clock, or a value set by
/// whoever drives the frames (a headless run).
pub enum Time {
    Real(Instant),
    Set(Rc<std::cell::Cell<f32>>),
}

/// The benchmark's view: the living stage, moving, measured every frame.
pub struct Bench {
    pub baked: Arc<Baked>,
    pub walkers: Arc<Walkers>,
    pub time: Time,
    pub stats: Rc<RefCell<Stats>>,
    /// Held at one hour, or through a day.
    pub hour: Option<f32>,
    /// Asks for the next frame as soon as this one is drawn.
    pub animate: bool,
    /// Stops measuring (and asks for no more frames) after this long.
    pub until: Option<Duration>,
    pub done: Rc<std::cell::Cell<bool>>,
    /// How many of the baked images have been handed to the display so
    /// far: the first frames hand them all over, a share a frame, unseen,
    /// so no image is first uploaded on the frame it first shows (a new
    /// zoom level or light). Frames until then are not measured.
    pub warmed: usize,
}

/// How many images one frame hands to the display ahead of use.
const WARM_PER_FRAME: usize = 400;

/// Hands `images` to the display without showing them: drawn a pixel
/// large at the scene's corner, under a mask that shows nothing.
pub fn warm(images: &[Arc<RenderImage>], bounds: gpui::Bounds<gpui::Pixels>, window: &mut Window) {
    let corner = Bounds::new(bounds.origin, size(px(1.0), px(1.0)));
    let hidden = gpui::ContentMask {
        bounds: Bounds::new(bounds.origin, size(px(0.0), px(0.0))),
    };
    window.with_content_mask(Some(hidden), |window| {
        for image in images {
            let _ = window.paint_image(corner, corner, Corners::default(), image.clone(), 0, false);
        }
    });
}

impl gpui::Render for Bench {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let started = Instant::now();
        let all = self.baked.images();
        let warming = self.warmed < all.len();
        let ahead =
            all[self.warmed.min(all.len())..(self.warmed + WARM_PER_FRAME).min(all.len())].to_vec();
        self.warmed += ahead.len();
        if warming {
            // The measured run's clock starts after the warm-up.
            if let Time::Real(began) = &mut self.time {
                *began = Instant::now();
            }
            window.request_animation_frame();
            return element(Rc::new(Plan::default())).child(
                gpui::canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| warm(&ahead, bounds, window),
                )
                .absolute()
                .size_full(),
            );
        }
        let seconds = match &self.time {
            Time::Real(began) => began.elapsed().as_secs_f32(),
            Time::Set(cell) => cell.get(),
        };
        {
            let mut stats = self.stats.borrow_mut();
            if let Some(last) = stats.last {
                let between = (started - last).as_secs_f64() * 1000.0;
                stats.between.push(between);
            }
            stats.started.get_or_insert(started);
            stats.last = Some(started);
        }
        let shot = bench_shot(&self.baked, seconds, window.scale_factor(), self.hour);
        let plan = Rc::new(plan(&self.baked, &self.walkers, &shot));
        {
            let mut stats = self.stats.borrow_mut();
            stats.images.push(plan.images());
            stats.moving.push(plan.moving);
        }
        let stats = self.stats.clone();
        let finished = self
            .until
            .zip(match &self.time {
                Time::Real(began) => Some(began.elapsed()),
                Time::Set(_) => None,
            })
            .is_some_and(|(until, elapsed)| elapsed >= until);
        if finished {
            self.done.set(true);
        } else if self.animate {
            window.request_animation_frame();
        }
        let _ = cx;
        element(plan).child(
            gpui::canvas(
                |_, _, _| (),
                move |_, _, _, _| {
                    let took = started.elapsed().as_secs_f64() * 1000.0;
                    stats.borrow_mut().planned.push(took);
                },
            )
            .absolute()
            .size_0(),
        )
    }
}
