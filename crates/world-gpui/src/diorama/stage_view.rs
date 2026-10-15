//! The living stage in the World window (v0.30 S1a): the town the player
//! sees, baked into sprites a look at a time and composited by GPUI every
//! frame, with everyone where the World's routines put them, in depth.
//!
//! - A *look* is one zoom level in one light (noon, dusk or night). It is
//!   baked off the window's thread when the camera first needs it: the
//!   level for the camera's zoom on this display, in the hour's light, and
//!   the next light as the hour cross-fades into it. The closer level is
//!   baked only around the camera: all of a long place that close is
//!   hundreds of megabytes.
//! - While a look is baking or being handed to the display, the nearest
//!   look already baked stands in for it, scaled: never a blank frame and
//!   never a rough one. Only before the very first look of a window is
//!   there the loading wash.
//! - Looks no camera is near (another level, a light the hour has left, a
//!   World as it was before it changed) are let go, so a window holds a
//!   look or two, not every level in every light.
//! - When the World changes (a new building, a story prop, the season,
//!   the weather), its looks are baked again in the background, the old
//!   ones kept on screen until the new are ready; whatever the old look
//!   lacks (a building just built) is drawn live in its place meanwhile.
//! - Residents are drawn from poses baked as they strike them, each pose
//!   once, and stand in depth with the buildings and things: someone
//!   behind a building is hidden by it, someone in front stands over it.
//!
//! The strip, covers, postcards and the golden pictures are still painted
//! a tile at a time, right where they are drawn (`interface::paint`); the
//! tiles painted off the window's thread, their rough stand-in and what
//! was painted ahead of the camera went with S1b.
//!
//! Presentation only, like the rest of the diorama: it reads a snapshot
//! and never anything the World records.

use super::living_stage::{
    self as ls, Cut, Deep, Draw, Item, Lens, Light, Look, Plan, Rank, Rect, Sprite,
};
use super::*;
use gpui::RenderImage;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The zoom levels baked, in device pixels a stage pixel for each of the
/// display's own: the place as it stands at rest, and close up.
pub const LEVELS: [f32; 2] = [1.0, 2.0];

/// Levels from this one up are baked around the camera only.
pub const SPANNED_FROM: usize = 1;

/// How far either side of the camera a level baked around it reaches, in
/// widths of the window.
pub const SPAN_REACH: f32 = 0.8;

/// The level to draw at for a camera `zoom` times closer: the least as
/// sharp, or the sharpest there is.
pub fn level_for(zoom: f32) -> usize {
    LEVELS
        .iter()
        .position(|level| *level >= zoom * 0.92)
        .unwrap_or(LEVELS.len() - 1)
}

/// What a look holds: all of the place, or only the land of a folded
/// postcard's nearer rows (its field fading in at the top).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Variant {
    Main,
    FoldLand,
}

/// A look the camera needs: of which World (its bake key), at which
/// level, in which light, and the stretch of the stage (from, to) it must
/// show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Want {
    pub key: u64,
    pub level: usize,
    pub light: Light,
    pub variant: Variant,
    pub view: (f32, f32),
    /// Whether this frame draws it (rather than baking it ahead).
    pub shown: bool,
}

/// A look a window holds: what it is of, the stretch of the stage it was
/// baked for (or all of it), and whether it is ready to show (baked and
/// handed to the display).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held {
    pub key: u64,
    pub level: usize,
    pub light: Light,
    pub variant: Variant,
    pub span: Option<(f32, f32)>,
    pub ready: bool,
}

impl Held {
    /// Whether it shows all of the stage from `view.0` to `view.1`.
    pub fn covers(&self, view: (f32, f32)) -> bool {
        self.span
            .is_none_or(|(from, to)| view.0 >= from - 0.5 && view.1 <= to + 0.5)
    }

    /// Whether it is exactly the look `want` asks for.
    pub fn answers(&self, want: &Want) -> bool {
        self.key == want.key
            && self.level == want.level
            && self.light == want.light
            && self.variant == want.variant
            && self.covers(want.view)
    }
}

/// How far a level is from the one wanted: the same, then sharper ones
/// (scaled down), nearest first, then blurrier ones (scaled up).
fn level_rank(have: usize, want: usize) -> usize {
    match have.cmp(&want) {
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => have - want,
        std::cmp::Ordering::Less => 10 + want - have,
    }
}

/// Which of `held` to draw for `want`: the very look if it is ready, and
/// otherwise the nearest ready one that shows what the camera sees: in
/// the same light before another, of this World before as it was, the
/// nearest level. Nothing only when nothing of the place is ready at all.
pub fn choose(held: &[Held], want: &Want) -> Option<usize> {
    held.iter()
        .enumerate()
        .filter(|(_, held)| held.ready && held.variant == want.variant && held.covers(want.view))
        .min_by_key(|(at, held)| {
            (
                held.light != want.light,
                held.key != want.key,
                level_rank(held.level, want.level),
                *at,
            )
        })
        .map(|(at, _)| at)
}

/// Which of `held` a window keeps: each look a want asks for (ready or on
/// its way to the display), and while one is not ready, the look standing
/// in for it. Everything else is let go.
pub fn keep(held: &[Held], wants: &[Want]) -> Vec<bool> {
    let mut keep = vec![false; held.len()];
    for want in wants {
        if let Some(at) = held.iter().position(|held| held.answers(want)) {
            keep[at] = true;
            if held[at].ready {
                continue;
            }
        }
        if let Some(at) = choose(held, want) {
            keep[at] = true;
        }
    }
    keep
}

/// The next look to bake, if none is baking: the first wanted that is
/// not held. With nothing at all to show yet for the first want, its
/// level baked whole comes first, being quicker to bake than a close one.
pub fn next_bake(held: &[Held], baking: bool, wants: &[Want]) -> Option<Want> {
    if baking {
        return None;
    }
    if let Some(first) = wants.first() {
        let whole = Want { level: 0, ..*first };
        if first.level > 0
            && choose(held, first).is_none()
            && !held.iter().any(|held| held.answers(&whole))
        {
            return Some(whole);
        }
    }
    wants
        .iter()
        .find(|want| !held.iter().any(|held| held.answers(want)))
        .copied()
}

/// The stretch of the stage a camera shows, for a stage `width` long seen
/// through a window `view_w` wide: all of it while it folds into a
/// postcard.
pub fn seen_by(camera: Camera, width: f32, view_w: f32) -> (f32, f32) {
    if camera.fold > 0.0 {
        return (0.0, width);
    }
    let half = view_w / (2.0 * camera.zoom.max(0.01));
    ((camera.x - half).max(0.0), (camera.x + half).min(width))
}

/// The looks a frame wants, first first: the camera's level in the hour's
/// light, the light it is fading into, a folded postcard's nearer land,
/// where the camera is heading, the level baked whole (to stand in while
/// a close one is baked again around a camera that has moved on), and the
/// next light shortly before the hour reaches it.
pub fn wants(
    key: u64,
    (width, view_w): (f32, f32),
    camera: Camera,
    heading: Option<Camera>,
    hour: f32,
) -> Vec<Want> {
    let level = level_for(camera.zoom);
    let view = seen_by(camera, width, view_w);
    let (a, b, t) = Light::blend(hour);
    let main = |light: Light, level: usize, view: (f32, f32)| Want {
        key,
        level,
        light,
        variant: Variant::Main,
        view,
        shown: true,
    };
    let ahead = |want: Want| Want {
        shown: false,
        ..want
    };
    let mut out = vec![main(a, level, view)];
    let fading = t > 0.002 && b != a;
    if fading {
        out.push(main(b, level, view));
    }
    if camera.fold > 0.0 {
        out.push(Want {
            variant: Variant::FoldLand,
            ..main(a, level, view)
        });
        if fading {
            out.push(Want {
                variant: Variant::FoldLand,
                ..main(b, level, view)
            });
        }
    }
    if let Some(to) = heading {
        let there = ahead(main(a, level_for(to.zoom), seen_by(to, width, view_w)));
        if !out
            .iter()
            .any(|want| want.level == there.level && want.view == there.view)
        {
            out.push(there);
        }
    }
    if level >= SPANNED_FROM {
        out.push(ahead(main(a, 0, view)));
    }
    // The lights of a while ahead, the one the hour fades into first: baked
    // before the fade begins, so it is ready on its first frame rather
    // than baked and handed over while the fade runs.
    let (soon_a, soon_b, soon_t) = Light::blend(hour + LIGHT_AHEAD);
    for soon in [soon_a, soon_b] {
        let fades = soon == soon_a || soon_t > 0.002;
        if fades
            && soon != a
            && !(fading && soon == b)
            && !out
                .iter()
                .any(|want| want.light == soon && want.level == level)
        {
            out.push(ahead(main(soon, level, view)));
        }
    }
    out
}

/// How far ahead of the hour (in hours) the next light is baked: long
/// enough for a look to bake and reach the display before its fade.
pub const LIGHT_AHEAD: f32 = 0.75;

/// Where a level is baked: around the middle of what the camera sees, for
/// a level baked around the camera, or all of the stage.
pub fn span_for(want: &Want, width: f32, view_w: f32) -> Option<(f32, f32)> {
    if want.level < SPANNED_FROM {
        return None;
    }
    let middle = (want.view.0 + want.view.1) / 2.0;
    let reach = (view_w * SPAN_REACH).max((want.view.1 - want.view.0) / 2.0 + view_w * 0.1);
    let span = ((middle - reach).max(0.0), (middle + reach).min(width));
    (span.0 > 0.0 || span.1 < width).then_some(span)
}

/// One pose someone strikes, baked once: who, how they look, how they
/// stand, which way they face, which step of a walk (none standing),
/// whether they carry a baby, at which level and in which light.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PoseKey {
    pub who: usize,
    pub figure: u64,
    pub stance: world_projection::Stance,
    pub mood: world_projection::Mood,
    pub right: bool,
    pub step: Option<u8>,
    pub carrying: bool,
    /// On a bench: how high its seat is for their height, in twentieths.
    pub seat: Option<u8>,
    pub level: usize,
    pub light: Light,
}

impl PoseKey {
    pub fn of(person: &PersonPaint, level: usize, light: Light) -> Self {
        let figure = &person.figure;
        let mut key = Key::new("figure");
        key.colour(figure.clothes)
            .colour(figure.hair)
            .colour(figure.skin)
            .add(figure.carries.map(|carry| std::mem::discriminant(&carry)))
            .add((figure.bird, figure.age, figure.stoop))
            .add(person.drawing.as_ref().map(|drawing| drawing.id.clone()));
        Self {
            who: person.index,
            figure: key.finish(),
            stance: person.stance,
            mood: person.mood,
            right: person.pose.facing >= 0.0,
            // A step of a walk, or a beat of a dance.
            step: person
                .pose
                .stride
                .map(|stride| {
                    ((stride.rem_euclid(1.0) * ls::WALK_FRAMES as f32) as u8)
                        .min(ls::WALK_FRAMES as u8 - 1)
                })
                .or(person.beat),
            carrying: person.carrying.is_some(),
            seat: person
                .seat
                .map(|seat| (seat * 20.0).round().clamp(0.0, 255.0) as u8),
            level,
            light,
        }
    }
}

/// Bakes one pose for `person` in the light of `frame`, at `scale` device
/// pixels a stage pixel, standing at (0, 0) as a grown-up of the stage is
/// tall at their age, with the soft shadow at their feet.
pub(super) fn bake_pose(
    frame: &Frame,
    person: &PersonPaint,
    key: &PoseKey,
    scale: f32,
) -> Option<Sprite> {
    let height = frame.figure_h * person.figure.age.height();
    let facing = if key.right { 1.0 } else { -1.0 };
    let tau = std::f32::consts::TAU;
    let dancing = key.stance == world_projection::Stance::Dancing;
    let pose = match key.step.filter(|_| !dancing) {
        Some(step) => {
            let phase = step as f32 / ls::WALK_FRAMES as f32;
            art::Pose {
                stride: Some(phase),
                bob: (phase * tau).sin().abs() * height * 0.035,
                facing,
                squash: 1.0 - 0.03 * (phase * tau * 2.0).cos().max(0.0),
                lean: facing * 0.018 * (phase * tau * 2.0).sin(),
            }
        }
        None => art::Pose {
            facing,
            ..art::Pose::default()
        },
    };
    let model = PersonPaint {
        x: 0.0,
        y: 0.0,
        height,
        pose,
        glow: None,
        seat: key.seat.map(|seat| seat as f32 / 20.0),
        beat: key.step.filter(|_| dancing),
        ..person.clone()
    };
    let room = height * 0.8;
    let (left, top) = (-room, -height * 1.4);
    let mut canvas = Canvas::new(
        (room * 2.0 * scale).ceil() as u32,
        (height * 1.55 * scale).ceil() as u32,
        scale,
        (left, top),
    )?;
    canvas.window_soft = true;
    let night = frame.daylight == Daylight::Night;
    let [r, g, b] = shadow_ink(frame.hour);
    let contact =
        Hsla::from(gpui::Rgba { r, g, b, a: 1.0 }).opacity(if night { 0.3 } else { 0.25 });
    canvas.soft(
        0.0,
        0.0,
        height * 0.2,
        height * 0.045,
        height * 0.05,
        contact,
    );
    paint_person_body(&mut canvas, frame, &model, (0.0, 0.0), frame.light());
    Sprite::cropped(canvas.pixmap, (left, top), scale)
}

/// The frames a look is baked from for `light`: the place as `snapshot`
/// lays it out in a window `view` large, everything standing in it
/// whatever the zoom (what the composition keeps for a closer look too),
/// and nothing that is drawn live (plots, people, designs). With
/// `fold_land`, a nearer row of a folded postcard.
pub(super) fn prepare(
    snapshot: &ProjectionSnapshot,
    (vw, vh): (f32, f32),
    light: Light,
    fold_land: bool,
) -> (Frame, Stage) {
    let stage = super::stage(snapshot, vw, vh);
    let daylight = light.daylight();
    let living = super::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
    let camera = Camera::around(&stage, 1.0, stage.width / 2.0, vh / 2.0);
    let mut out = super::frame(
        snapshot,
        &stage,
        &living,
        camera,
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    )
    .at_hour(light.hour())
    .stilled(true);
    let everything = super::frame(
        snapshot,
        &stage,
        &living,
        Camera {
            zoom: 1000.0,
            ..camera
        },
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    );
    out.buildings = everything.buildings;
    out.things = everything.things;
    out.plots.clear();
    out.people.clear();
    out.wearing.clear();
    out.bonds.clear();
    out.blend_top = fold_land;
    (out, stage)
}

/// What a frame's looks are of: everything a look shows that does not
/// change with the hour or the camera. A new building, a story prop, the
/// season, the weather or the window's size is a new key, and is baked
/// again.
pub fn bake_key(frame: &Frame) -> u64 {
    let mut key = Key::new("living-stage");
    let s = frame.scenery;
    key.float(frame.view_w)
        .float(frame.height)
        .float(frame.width)
        .add((s.sky_top, s.sky_bottom, s.far, s.near, s.sun))
        .add(frame.weather as u8)
        .add(frame.season.map(|season| season as u8))
        .add(frame.cover.map(|cover| cover as u8))
        .add((frame.ice, frame.water))
        .add(frame.setting as u8);
    if let Some(look) = &frame.look {
        key.add((look.key, look.glow, look.spine, look.haze));
        for lamp in &look.lamps {
            key.float(*lamp);
        }
    }
    for (_, building) in frame.standing.iter() {
        key.add((building.index, building.shape as u8))
            .float(building.x)
            .float(building.base)
            .float(building.w)
            .float(building.h)
            .add(building.palette.seed)
            .add(building.palette.art.is_some())
            .add(building.drawing.as_ref().map(|d| d.id.clone()))
            .add((building.flip, building.joins, building.inside.len()));
    }
    key.add(frame.things_key);
    for mark in frame.marks.iter().filter(|mark| mark.grow >= 1.0) {
        key.add(mark.shape as u8).float(mark.along);
    }
    for goal in &frame.goals {
        key.add((goal.shape as u8, goal.done, goal.parts));
    }
    for patch in &frame.patches {
        key.float(patch.x0)
            .float(patch.x1)
            .float(patch.top)
            .float(patch.bottom)
            .add((patch.patch, patch.water, patch.seed));
    }
    key.add(&frame.garlands);
    key.finish()
}

/// A look a window holds.
pub struct Slot {
    pub held: Held,
    pub look: Arc<Look>,
    /// The frame it was baked from: its light, for the poses baked to go
    /// with it.
    pub frame: Arc<Frame>,
    /// The window it was baked for, and its stage's width.
    pub view: (f32, f32),
    pub width: f32,
    /// The far hills' band: width, height, reach past the left edge, and
    /// height above the horizon.
    pub band: (f32, f32, f32, f32),
    pub horizon: f32,
    /// A cottage's width on its stage.
    pub building_w: f32,
    images: Vec<Arc<RenderImage>>,
    /// How many of its images have been handed to the display.
    handed: usize,
    pub took: Duration,
}

/// A look baked, as the log has it: which, how long it took, its bytes,
/// its pictures, and how many of them were taken from a look baked before
/// (already with the display).
#[derive(Clone, Copy, Debug)]
pub struct BakeLog {
    pub held: Held,
    pub took: Duration,
    pub bytes: usize,
    pub pictures: usize,
    pub taken: usize,
}

/// What a bake hands back: the look, the frame it was baked from, and a
/// cottage's width on its stage.
type Baked = (Look, Frame, f32);

/// A look being baked off the window's thread.
struct Baking {
    held: Held,
    view: (f32, f32),
    out: Arc<Mutex<Option<Baked>>>,
    started: Instant,
    /// The look it takes what is the same from.
    before: Option<Arc<Look>>,
}

/// A pose baked: what it is, and its picture (none if it draws nothing).
type Pose = (PoseKey, Option<Sprite>);

/// Poses being baked off the window's thread.
struct Posing {
    out: Arc<Mutex<Option<Vec<Pose>>>>,
    keys: Vec<PoseKey>,
}

/// A pose baked, and when it was last drawn.
struct Posed {
    sprite: Option<Sprite>,
    used: Instant,
}

/// At most this many pictures a frame hands to the display.
const HAND_IMAGES_PER_FRAME: usize = 48;

/// How many pictures a frame tells the display to forget: a look let go
/// is forgotten over a few frames, never all in one (64 a frame came to
/// several milliseconds of a frame's plan, and 24 at display scale 2 to a
/// 50 ms frame on CI's Mac). Eight a frame forgets a look of 200 pictures
/// in under half a second, faster than looks are let go, so those waiting
/// never pile up.
pub const FREE_PER_FRAME: usize = 8;

/// Past this many pictures waiting to be forgotten, a frame forgets its
/// share even while it hands pictures over.
const FREE_BACKLOG: usize = FREE_PER_FRAME * 4;

/// How many bytes of new pictures a frame hands to the display: enough
/// that a look arrives in well under a second, few enough that handing
/// them over (a copy into the display's memory on the window's thread)
/// takes about a millisecond on this machine. It learns how fast the
/// display takes pictures from the frames that handed some, and halves
/// after a frame that came late, growing back after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UploadBudget {
    /// The bytes the next frame may hand over.
    pub bytes: usize,
    /// How fast the display has taken pictures, in bytes a millisecond.
    rate: Option<f64>,
    /// The usual time between frames, in milliseconds.
    usual: Option<f64>,
}

impl Default for UploadBudget {
    fn default() -> Self {
        Self {
            bytes: Self::FIRST,
            rate: None,
            usual: None,
        }
    }
}

impl UploadBudget {
    /// The most a frame hands over, however fast the display.
    pub const MOST: usize = 2 << 20;
    /// Until the place first shows settled.
    pub const OPENING: usize = 4 << 20;
    /// The least: a look still arrives within a second or two.
    pub const LEAST: usize = 512 << 10;
    /// Before anything is measured.
    pub const FIRST: usize = 1 << 20;
    /// The time a frame may spend handing pictures over, in milliseconds.
    pub const SPEND_MS: f64 = 1.0;

    /// Learns from a frame: the bytes it handed over and how long that
    /// took (milliseconds), and the time since the frame before it.
    pub fn adapt(&mut self, handed: usize, took_ms: f64, interval_ms: Option<f64>) {
        // Only a handover large enough to time says how fast it goes.
        if handed >= 64 << 10 && took_ms > 0.01 {
            let rate = handed as f64 / took_ms;
            self.rate = Some(self.rate.map_or(rate, |old| old * 0.7 + rate * 0.3));
        }
        let mut bytes = self.rate.map_or(Self::FIRST as f64, |rate| {
            (rate * Self::SPEND_MS).clamp(Self::LEAST as f64, Self::MOST as f64)
        }) as usize;
        if let Some(interval) = interval_ms {
            let usual = self.usual.unwrap_or(interval);
            if interval > 20.0 && interval > usual * 1.8 {
                // A late frame: hand over less for a while.
                bytes = (self.bytes / 2).max(Self::LEAST).min(bytes);
            } else {
                // Growing back a quarter a frame, no faster.
                bytes = bytes.min(self.bytes + self.bytes / 4);
            }
            // A display that slows for good (a window behind others, a
            // slower refresh) is soon usual, and no longer late.
            self.usual = Some(usual * 0.9 + interval.min(usual * 3.0) * 0.1);
        }
        self.bytes = bytes.clamp(Self::LEAST, Self::MOST);
    }
}

/// What one frame does with the looks on their way to the display, one
/// thing at a time, so that no frame both shows a new look for the first
/// time and copies pictures to the display, or forgets them: `swap` shows
/// a look whose pictures the display now all has, `upload` hands more
/// over, `release` is how many pictures let go it tells the display to
/// forget.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Chores {
    pub swap: bool,
    pub upload: bool,
    pub release: usize,
}

/// A frame's chores: with a look all handed over (`complete`), showing
/// it; else with pictures or poses waiting (`waiting`), handing them
/// over; else forgetting pictures let go (`freeing` of them). Forgetting
/// is put off only while fewer than [`FREE_BACKLOG`] wait.
pub fn chores(complete: bool, waiting: bool, freeing: usize) -> Chores {
    let release = freeing.min(FREE_PER_FRAME);
    if complete {
        Chores {
            swap: true,
            upload: false,
            release: if freeing > FREE_BACKLOG { release } else { 0 },
        }
    } else if waiting {
        Chores {
            swap: false,
            upload: true,
            release: if freeing > FREE_BACKLOG { release } else { 0 },
        }
    } else {
        Chores {
            swap: false,
            upload: false,
            release,
        }
    }
}

/// What a frame of the living stage did, beside drawing: for telling what
/// a late frame or a slow plan came with.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameTags {
    /// Threads baking a look (and one more for poses) while it was drawn.
    pub baking: usize,
    /// A look baked off the thread was taken in.
    pub arrived: bool,
    /// A look was shown for the first time.
    pub swapped: bool,
    /// Pictures handed to the display (looks' and poses'), and their bytes.
    pub uploaded: usize,
    pub uploaded_bytes: usize,
    /// Pictures the display was told to forget.
    pub released: usize,
    /// Buildings, things and people drawn live for want of a picture, and
    /// people drawn in another of their poses meanwhile.
    pub live: usize,
    pub stand_ins: usize,
    /// The World was laid out again.
    pub laid_out: bool,
    /// Images the frame drew.
    pub images: usize,
}

/// How long a pose nobody has struck is kept.
const POSE_KEPT: Duration = Duration::from_secs(20);

/// How many poses one bake off the thread takes on.
const POSES_A_BAKE: usize = 48;

/// A window's living stage: the World it is of, the looks it holds and
/// is baking, and the poses its residents strike.
pub struct Stagehand {
    /// Device pixels a point.
    pub dpr: f32,
    key: u64,
    snapshot: Option<Arc<ProjectionSnapshot>>,
    view: (f32, f32),
    width: f32,
    pub slots: Vec<Slot>,
    baking: Option<Baking>,
    poses: HashMap<PoseKey, Posed>,
    posing: Option<Posing>,
    /// Poses asked for this frame and not baked yet, with whoever strikes
    /// them.
    asked: Vec<(PoseKey, PersonPaint)>,
    /// The most pixels it has held at once, in bytes.
    pub peak: usize,
    /// What each look took to bake.
    pub log: Vec<BakeLog>,
    /// Pictures let go, still to be forgotten by the display, a frame's
    /// share at a time.
    freeing: Vec<Arc<RenderImage>>,
    /// The bytes of its looks' pictures, until it takes or lets go of one.
    held_bytes: Option<usize>,
    /// Poses baked, waiting their turn to be handed to the display.
    arriving: Vec<Pose>,
    /// How much a frame hands to the display.
    pub budget: UploadBudget,
    /// When the last frame was, and what it did.
    last_frame: Option<Instant>,
    pub tags: FrameTags,
    /// How many items the last plan drew, for the next to start with room.
    items_hint: usize,
    /// Whether the stage has settled once since the window opened.
    opened: bool,
}

impl Default for Stagehand {
    fn default() -> Self {
        Self::new(1.0)
    }
}

/// What a frame of the stage came to: whether anything of the place was
/// shown, whether every look it wants was shown sharp with nothing on its
/// way, and whether more frames are needed to get there.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Report {
    pub shown: bool,
    pub settled: bool,
    pub waiting: bool,
}

impl Stagehand {
    pub fn new(dpr: f32) -> Self {
        Self {
            dpr,
            key: 0,
            snapshot: None,
            view: (0.0, 0.0),
            width: 0.0,
            slots: Vec::new(),
            baking: None,
            poses: HashMap::new(),
            posing: None,
            asked: Vec::new(),
            peak: 0,
            log: Vec::new(),
            freeing: Vec::new(),
            held_bytes: None,
            arriving: Vec::new(),
            budget: UploadBudget::default(),
            last_frame: None,
            tags: FrameTags::default(),
            items_hint: 0,
            opened: false,
        }
    }

    /// The pixels it holds, in bytes: every look (a picture two share
    /// counted once), and every pose.
    pub fn bytes(&mut self) -> usize {
        let looks = match self.held_bytes {
            Some(bytes) => bytes,
            None => {
                let mut seen = std::collections::HashSet::new();
                let bytes = self
                    .slots
                    .iter()
                    .flat_map(|slot| slot.images.iter())
                    .filter(|image| seen.insert(Arc::as_ptr(image)))
                    .map(|image| {
                        let size = image.size(0);
                        (size.width.0.max(0) as usize) * (size.height.0.max(0) as usize) * 4
                    })
                    .sum();
                self.held_bytes = Some(bytes);
                bytes
            }
        };
        looks + self.pose_bytes()
    }

    /// The pixels of its poses, in bytes.
    pub fn pose_bytes(&self) -> usize {
        self.poses
            .values()
            .filter_map(|posed| posed.sprite.as_ref())
            .map(Sprite::bytes)
            .sum()
    }

    /// How many poses it holds.
    pub fn pose_count(&self) -> usize {
        self.poses.len()
    }

    /// The looks it holds, as the policy sees them.
    pub fn held(&self) -> Vec<Held> {
        self.slots.iter().map(|slot| slot.held).collect()
    }

    /// Whether a look or poses are being baked.
    pub fn busy(&self) -> bool {
        self.baking.is_some() || self.posing.is_some()
    }

    /// Takes in what a frame is of: a new World (or one changed, or a
    /// window resized) keeps what it holds on screen until its own looks
    /// are baked.
    fn sit(&mut self, frame: &Frame, snapshot: &ProjectionSnapshot) {
        let key = bake_key(frame);
        if key != self.key || self.snapshot.is_none() {
            self.key = key;
            self.snapshot = Some(Arc::new(snapshot.clone()));
            self.view = (frame.view_w, frame.height);
            self.width = frame.width;
        }
    }

    /// Takes in the look baked off the thread, if it is done.
    fn collect(&mut self) {
        let done = self
            .baking
            .as_ref()
            .and_then(|baking| baking.out.lock().ok().and_then(|mut out| out.take()));
        if let Some(baked) = done {
            if let Some(baking) = self.baking.take() {
                self.take(baking.held, baking.view, baked, baking.started.elapsed());
                self.tags.arrived = true;
            }
        }
        let posed = self
            .posing
            .as_ref()
            .and_then(|posing| posing.out.lock().ok().and_then(|mut out| out.take()));
        if let Some(posed) = posed {
            // Handed to the display a frame's share at a time, like a look.
            self.arriving.extend(posed);
            self.posing = None;
        }
    }

    /// Takes in poses waiting their turn, up to `budget` bytes of them
    /// (at least one), returning the bytes and how many came with a
    /// picture. Each is drawn, and so handed to the display, this frame.
    fn admit_poses(&mut self, budget: &mut usize, count: &mut usize) -> (usize, usize) {
        let now = Instant::now();
        let (mut bytes, mut pictures) = (0, 0);
        while let Some((_, sprite)) = self.arriving.last() {
            let size = sprite.as_ref().map_or(0, Sprite::bytes);
            if *count == 0 || (size > *budget && pictures > 0) {
                break;
            }
            let Some((key, sprite)) = self.arriving.pop() else {
                break;
            };
            if sprite.is_some() {
                pictures += 1;
                *count -= 1;
            }
            bytes += size;
            *budget = budget.saturating_sub(size);
            self.poses.insert(key, Posed { sprite, used: now });
        }
        (bytes, pictures)
    }

    /// Holds a look just baked, not yet ready until the display has it.
    fn take(
        &mut self,
        held: Held,
        view: (f32, f32),
        (look, frame, building_w): Baked,
        took: Duration,
    ) {
        // Pictures taken from a look already with the display need no
        // handing over: they come first, counted as handed.
        let with_display = self
            .slots
            .iter()
            .flat_map(|slot| slot.images[..slot.handed].iter())
            .map(Arc::as_ptr)
            .collect::<std::collections::HashSet<_>>();
        let (mut images, fresh): (Vec<_>, Vec<_>) = look
            .images()
            .into_iter()
            .partition(|image| with_display.contains(&Arc::as_ptr(image)));
        let handed = images.len();
        images.extend(fresh);
        let band = Band::of(&frame, view.0, view.1);
        self.log.push(BakeLog {
            held,
            took,
            bytes: look.bytes(),
            pictures: images.len(),
            taken: handed,
        });
        self.slots.push(Slot {
            held: Held {
                ready: false,
                ..held
            },
            images,
            look: Arc::new(look),
            view,
            width: frame.width,
            band: (band.w, band.above + band.below, band.pad, band.above),
            horizon: frame.horizon,
            building_w,
            frame: Arc::new(frame),
            handed,
            took,
        });
        self.held_bytes = None;
        self.peak = self.peak.max(self.bytes());
    }

    /// Starts baking the next look wanted, off the thread, or right here
    /// (`now`: a test, a cover).
    fn schedule(&mut self, wants: &[Want], now: bool) {
        // A batch of poses is short: a look waits for it rather than
        // baking beside it on cores the window needs.
        if !now && self.posing.is_some() {
            return;
        }
        loop {
            let Some(want) = next_bake(&self.held(), self.baking.is_some(), wants) else {
                return;
            };
            let Some(snapshot) = self.snapshot.clone() else {
                return;
            };
            let span = span_for(&want, self.width, self.view.0);
            let held = Held {
                key: want.key,
                level: want.level,
                light: want.light,
                variant: want.variant,
                span,
                ready: false,
            };
            let cut = Cut {
                span,
                people: false,
                fold_land: want.variant == Variant::FoldLand,
            };
            // Painting in place holds the window's thread (a test, a
            // cover): a device pixel a stage pixel is enough there.
            let scale = LEVELS[want.level] * if now { 1.0 } else { self.dpr };
            let view = self.view;
            let started = Instant::now();
            let before = self
                .donor(&want, span)
                .map(|at| self.slots[at].look.clone());
            if now {
                // Right here, only what this frame shows.
                if !want.shown {
                    return;
                }
                let baked = bake_one(&snapshot, view, (want.light, scale), cut, before);
                self.take(held, view, baked, started.elapsed());
                // Shown at once: nothing to hand over a frame at a time.
                if let Some(slot) = self.slots.last_mut() {
                    slot.handed = slot.images.len();
                    slot.held.ready = true;
                }
                continue;
            }
            let out = Arc::new(Mutex::new(None));
            let sink = out.clone();
            let taking = before.clone();
            let spawned = std::thread::Builder::new()
                .name("living-stage-bake".into())
                .spawn(move || {
                    let baked = bake_one(&snapshot, view, (want.light, scale), cut, taking);
                    if let Ok(mut out) = sink.lock() {
                        *out = Some(baked);
                    }
                });
            if spawned.is_ok() {
                self.baking = Some(Baking {
                    held,
                    view,
                    out,
                    started,
                    before,
                });
            }
            return;
        }
    }

    /// The look already baked that has most of the look `want` (over
    /// `span`): of this World, the most of the stretch wanted; else of the
    /// World as it was. A bake takes from it whatever is the same.
    fn donor(&self, want: &Want, span: Option<(f32, f32)>) -> Option<usize> {
        let share = |slot: &Slot| {
            let (from, to) = slot.held.span.unwrap_or((0.0, self.width));
            let (want_from, want_to) = span.unwrap_or((0.0, self.width));
            (to.min(want_to) - from.max(want_from)).max(0.0)
        };
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| {
                slot.held.level == want.level
                    && slot.held.light == want.light
                    && slot.held.variant == want.variant
                    && slot.view == self.view
                    && share(slot) > 0.0
            })
            .max_by(|(_, a), (_, b)| {
                (a.held.key == want.key)
                    .cmp(&(b.held.key == want.key))
                    .then(share(a).total_cmp(&share(b)))
            })
            .map(|(at, _)| at)
    }

    /// Lets go of every look no want keeps, returning their pictures for
    /// the display to forget. A look a bake (under way, or next) takes
    /// from is kept until that bake is done.
    fn evict(&mut self, wants: &[Want]) -> Vec<Arc<RenderImage>> {
        let mut keep = keep(&self.held(), wants);
        let baking = self
            .baking
            .as_ref()
            .and_then(|baking| baking.before.clone());
        for (at, slot) in self.slots.iter().enumerate() {
            if baking
                .as_ref()
                .is_some_and(|before| Arc::ptr_eq(before, &slot.look))
            {
                keep[at] = true;
            }
        }
        let held = self.held();
        for want in wants {
            if held.iter().any(|held| held.answers(want)) {
                continue;
            }
            let span = span_for(want, self.width, self.view.0);
            if let Some(at) = self.donor(want, span) {
                keep[at] = true;
            }
        }
        let mut gone = Vec::new();
        let mut dropped = Vec::new();
        for (slot, kept) in std::mem::take(&mut self.slots).into_iter().zip(keep) {
            if kept {
                self.slots.push(slot);
            } else {
                dropped.push(slot);
            }
        }
        // A picture a look kept shares (taken into it when it was baked)
        // stays with the display.
        let shared = if dropped.is_empty() {
            std::collections::HashSet::new()
        } else {
            self.slots
                .iter()
                .flat_map(|slot| slot.images.iter())
                .map(Arc::as_ptr)
                .collect::<std::collections::HashSet<_>>()
        };
        for slot in &dropped {
            gone.extend(
                slot.images
                    .iter()
                    .take(slot.handed)
                    .filter(|image| !shared.contains(&Arc::as_ptr(image)))
                    .cloned(),
            );
        }
        if !dropped.is_empty() {
            self.held_bytes = None;
        }
        // Tens of megabytes of pixels are freed off the window's thread.
        let_go(dropped);
        // Poses of levels and lights no longer wanted, unused a while.
        let now = Instant::now();
        let wanted = |key: &PoseKey| {
            wants
                .iter()
                .any(|want| want.level == key.level && want.light == key.light)
        };
        self.poses.retain(|key, posed| {
            let since = now.duration_since(posed.used);
            let kept = since < POSE_KEPT || (wanted(key) && since < POSE_KEPT * 3);
            if !kept {
                if let Some(sprite) = &posed.sprite {
                    gone.push(sprite.image.clone());
                }
            }
            kept
        });
        gone
    }

    /// Hands the next share of each look on its way to the display, up to
    /// a frame's worth of bytes, returning those pictures; a look all of
    /// whose pictures the display has is ready.
    ///
    /// A look all of whose pictures were handed over on earlier frames is
    /// shown on a frame that hands nothing over (see [`chores`]); right
    /// here (`now`), everything is handed over and shown at once.
    fn hand_over(&mut self, now: bool, chores: Chores) -> Vec<Arc<RenderImage>> {
        let mut out = Vec::new();
        if now {
            for slot in &mut self.slots {
                slot.handed = slot.images.len();
                slot.held.ready = true;
            }
            let stamp = Instant::now();
            for (key, sprite) in self.arriving.drain(..) {
                self.poses.insert(
                    key,
                    Posed {
                        sprite,
                        used: stamp,
                    },
                );
            }
            return out;
        }
        if chores.swap {
            for slot in &mut self.slots {
                if !slot.held.ready && slot.handed >= slot.images.len() {
                    slot.held.ready = true;
                    self.tags.swapped = true;
                }
            }
            return out;
        }
        if !chores.upload {
            return out;
        }
        // Until the place first shows settled (the loading wash, then
        // everyone's first poses), as fast as the display takes them.
        let mut budget = if !self.opened {
            UploadBudget::OPENING
        } else {
            self.budget.bytes
        };
        let mut count = HAND_IMAGES_PER_FRAME;
        let mut full = false;
        for slot in &mut self.slots {
            if slot.held.ready || full {
                continue;
            }
            while slot.handed < slot.images.len() && count > 0 {
                let image = slot.images[slot.handed].clone();
                let size = image.size(0);
                let bytes = (size.width.0.max(0) as usize) * (size.height.0.max(0) as usize) * 4;
                // A picture larger than what is left waits for the next
                // frame, unless it is the frame's first.
                if bytes > budget && self.tags.uploaded > 0 {
                    full = true;
                    break;
                }
                count -= 1;
                budget = budget.saturating_sub(bytes.max(1));
                self.tags.uploaded += 1;
                self.tags.uploaded_bytes += bytes;
                out.push(image);
                slot.handed += 1;
            }
        }
        // Then poses, at least one a frame: a resident with none is drawn
        // live meanwhile, which costs the window's thread more.
        let (bytes, pictures) = self.admit_poses(&mut budget, &mut count);
        self.tags.uploaded += pictures;
        self.tags.uploaded_bytes += bytes;
        out
    }

    /// Whether a look's pictures are all with the display, not shown yet.
    fn complete(&self) -> bool {
        self.slots
            .iter()
            .any(|slot| !slot.held.ready && slot.handed >= slot.images.len())
    }

    /// Whether pictures or poses wait to be handed to the display.
    fn uploads_waiting(&self) -> bool {
        !self.arriving.is_empty()
            || self
                .slots
                .iter()
                .any(|slot| !slot.held.ready && slot.handed < slot.images.len())
    }

    /// Learns from what handing pictures over took on the frame before
    /// (`handed` bytes in `took_ms`), timed where they were drawn.
    pub fn learn_upload(&mut self, handed: usize, took_ms: f64) {
        let now = Instant::now();
        let interval = self
            .last_frame
            .map(|last| now.duration_since(last).as_secs_f64() * 1000.0);
        self.last_frame = Some(now);
        self.budget.adapt(handed, took_ms, interval);
    }

    /// The look to draw for `want`, if any is ready.
    pub fn pick(&self, want: &Want) -> Option<&Slot> {
        choose(&self.held(), want).map(|at| &self.slots[at])
    }

    /// The pose to draw `person` in at `level` in `light`: theirs if it is
    /// baked (asking for it otherwise), or the nearest of theirs that is.
    fn pose(&mut self, person: &PersonPaint, level: usize, light: Light) -> Option<Sprite> {
        let key = PoseKey::of(person, level, light);
        let now = Instant::now();
        if let Some(posed) = self.poses.get_mut(&key) {
            posed.used = now;
            return posed.sprite.clone();
        }
        if !self.asked.iter().any(|(asked, _)| *asked == key)
            && !self
                .posing
                .as_ref()
                .is_some_and(|posing| posing.keys.contains(&key))
            && !self.arriving.iter().any(|(arriving, _)| *arriving == key)
        {
            self.asked.push((key, person.clone()));
        }
        self.tags.stand_ins += 1;
        // Theirs in another pose meanwhile: the same light and facing
        // first, standing before a step of a walk, the same level.
        self.poses
            .iter()
            .filter(|(other, posed)| {
                other.who == key.who && other.figure == key.figure && posed.sprite.is_some()
            })
            .min_by_key(|(other, _)| {
                (
                    other.light != key.light,
                    other.right != key.right,
                    other.stance != key.stance,
                    other.step != key.step,
                    level_rank(other.level, key.level),
                )
            })
            .and_then(|(_, posed)| posed.sprite.clone())
    }

    /// Bakes the poses asked for this frame: right here (`now`), or off
    /// the thread, a batch at a time.
    fn pose_asked(&mut self, now: bool) {
        if self.asked.is_empty() || self.posing.is_some() {
            return;
        }
        // While a look bakes on a machine of few cores, poses wait for it,
        // unless someone has no pose at all (they would be drawn live,
        // which costs the window's thread far more).
        if !now && self.baking.is_some() && ls::bake_threads() < 3 {
            let bare = self.asked.iter().any(|(key, _)| {
                !self
                    .poses
                    .iter()
                    .any(|(have, posed)| have.who == key.who && posed.sprite.is_some())
            });
            if !bare {
                return;
            }
        }
        let frames = self
            .slots
            .iter()
            .filter(|slot| slot.held.variant == Variant::Main)
            .map(|slot| (slot.held.light, slot.frame.clone()))
            .collect::<HashMap<_, _>>();
        let asked = std::mem::take(&mut self.asked);
        let (batch, rest): (Vec<_>, Vec<_>) =
            asked.into_iter().enumerate().partition(|(at, (key, _))| {
                (now || *at < POSES_A_BAKE) && frames.contains_key(&key.light)
            });
        self.asked = rest.into_iter().map(|(_, asked)| asked).collect();
        if batch.is_empty() {
            return;
        }
        let dpr = self.dpr;
        let jobs = batch
            .into_iter()
            .filter_map(|(_, (key, person))| Some((key, person, frames.get(&key.light)?.clone())))
            .collect::<Vec<_>>();
        let bake = move |jobs: Vec<(PoseKey, PersonPaint, Arc<Frame>)>| {
            jobs.into_iter()
                .map(|(key, person, frame)| {
                    let scale = LEVELS[key.level] * if now { 1.0 } else { dpr };
                    (key, bake_pose(&frame, &person, &key, scale))
                })
                .collect::<Vec<_>>()
        };
        if now {
            let stamp = Instant::now();
            for (key, sprite) in bake(jobs) {
                self.poses.insert(
                    key,
                    Posed {
                        sprite,
                        used: stamp,
                    },
                );
            }
            return;
        }
        let keys = jobs.iter().map(|(key, _, _)| *key).collect();
        let out = Arc::new(Mutex::new(None));
        let sink = out.clone();
        let spawned = std::thread::Builder::new()
            .name("living-stage-poses".into())
            .spawn(move || {
                let posed = bake(jobs);
                if let Ok(mut out) = sink.lock() {
                    *out = Some(posed);
                }
            });
        if spawned.is_ok() {
            self.posing = Some(Posing { out, keys });
        }
    }

    /// One frame: takes in what it is of and whatever has been baked,
    /// starts baking what the camera wants next, lets go of what it no
    /// longer needs, and works out what to draw. Returns the plan, the
    /// pictures to hand the display unseen this frame, the pictures the
    /// display can forget, and how the frame went. `now` bakes everything
    /// right here (a test).
    pub fn frame(
        &mut self,
        frame: &Arc<Frame>,
        snapshot: &ProjectionSnapshot,
        now: bool,
    ) -> (Plan, Vec<Arc<RenderImage>>, Vec<Arc<RenderImage>>, Report) {
        self.tags = FrameTags::default();
        self.sit(frame, snapshot);
        self.collect();
        let wants = wants(
            self.key,
            (frame.width, frame.view_w),
            frame.camera,
            frame.heading(),
            frame.hour,
        );
        // Painting in place (a test, a cover), the whole level only: a
        // close one is a refinement baked off the thread, and baking it
        // here would hold the window's thread at every zoom and pan.
        let wants = if now {
            let mut whole = Vec::new();
            for want in wants {
                let want = Want {
                    level: 0,
                    view: (0.0, frame.width),
                    ..want
                };
                if !whole.contains(&want) {
                    whole.push(want);
                }
            }
            whole
        } else {
            wants
        };
        self.schedule(&wants, now);
        let chores = chores(self.complete(), self.uploads_waiting(), self.freeing.len());
        let ahead = self.hand_over(now, chores);
        let gone = self.evict(&wants);
        self.freeing.extend(gone);
        let take = if now {
            self.freeing.len()
        } else {
            chores.release.min(self.freeing.len())
        };
        let gone = self.freeing.drain(..take).collect::<Vec<_>>();
        self.tags.released = gone.len();
        let (plan, shown) = plan_window(self, frame, &wants);
        self.pose_asked(now);
        // Poses asked for and baked right here are drawn this same frame.
        let plan = if now && !self.poses.is_empty() && !shown.1 {
            plan_window(self, frame, &wants).0
        } else {
            plan
        };
        self.peak = self.peak.max(self.bytes());
        let held = self.held();
        let settled = wants
            .iter()
            .filter(|want| want.shown)
            .all(|want| held.iter().any(|h| h.ready && h.answers(want)))
            && self.baking.is_none()
            && self.asked.is_empty()
            && self.posing.is_none()
            && self.arriving.is_empty();
        self.opened |= settled;
        let waiting = !settled
            || held.iter().any(|h| !h.ready)
            || !self.arriving.is_empty()
            || !self.freeing.is_empty();
        self.tags.baking = if self.baking.is_some() {
            ls::bake_threads()
        } else {
            0
        } + usize::from(self.posing.is_some());
        self.tags.images = plan.images();
        (
            plan,
            ahead,
            gone,
            Report {
                shown: shown.0,
                settled,
                waiting,
            },
        )
    }
}

/// Frees `pixels` off the window's thread: freeing a look's tens of
/// megabytes there is a frame's worth of time.
pub(super) fn let_go<T: Send + 'static>(pixels: Vec<T>) {
    if pixels.is_empty() {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("living-stage-free".into())
        .spawn(move || drop(pixels));
}

/// Frees pictures the display has forgotten off the window's thread, each
/// once nothing else holds it: the last frame drawn with them may still be
/// on its way out, and the last to let go of a picture frees its pixels.
/// Whatever is still held after a couple of seconds is let go as it is.
pub(super) fn let_go_pictures(pictures: Vec<Arc<RenderImage>>) {
    if pictures.is_empty() {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("living-stage-free".into())
        .spawn(move || {
            let mut pictures = pictures;
            for _ in 0..40 {
                // Those held here alone are freed right here.
                pictures.retain(|picture| Arc::strong_count(picture) > 1);
                if pictures.is_empty() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });
}

/// Bakes one look of `snapshot` for a window `view` large.
fn bake_one(
    snapshot: &ProjectionSnapshot,
    view: (f32, f32),
    (light, scale): (Light, f32),
    cut: Cut,
    before: Option<Arc<Look>>,
) -> Baked {
    let (frame, stage) = prepare(snapshot, view, light, cut.fold_land);
    let world = bake_key(&frame);
    let look = ls::bake_look(
        &frame,
        &stage,
        &[],
        (light, scale),
        view,
        cut,
        (world, before.as_deref()),
    );
    (look, frame, stage.building_w)
}

/// Something drawn live within `rect`, clipped to `clip` if given.
fn live(rect: Rect, clip: Option<Rect>, paint: impl Fn(&mut Window, Rect) + 'static) -> Draw {
    let rect = clip.map_or(rect, |clip| ls::intersection(rect, clip));
    Draw::Live {
        rect,
        paint: Rc::new(move |window: &mut Window, scene: Rect| {
            let Some(clip) = clip else {
                paint(window, scene);
                return;
            };
            let bounds = Bounds::new(
                point(px(scene.0 + clip.0), px(scene.1 + clip.1)),
                size(px(clip.2.max(0.0)), px(clip.3.max(0.0))),
            );
            window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
                paint(window, scene)
            });
        }),
    }
}

/// A draw cut to `clip`, if given.
fn clipped(draw: Draw, clip: Option<Rect>) -> Draw {
    match (draw, clip) {
        (
            Draw::Image {
                image,
                rect,
                clip: own,
            },
            Some(clip),
        ) => Draw::Image {
            image,
            rect,
            clip: Some(own.map_or(clip, |own| ls::intersection(own, clip))),
        },
        (draw, _) => draw,
    }
}

/// What lies on the ground of `frame` that is drawn live, under everything
/// standing: the wet stones in the rain, the plots staked out (the one
/// pointed at brighter), and the light under whatever is lit up and the
/// long shadows of the residents by day.
fn paint_ground_live(
    window: &mut dyn Brush,
    frame: &Frame,
    (ox, oy, width, height): Rect,
    live_things: &[usize],
) {
    let t = frame.seconds;
    let z = frame.camera.zoom;
    let night = frame.daylight == Daylight::Night;
    let light = frame.light();
    let screen = |x: f32, y: f32| {
        let (sx, sy) = frame.at(x, y);
        (ox + sx, oy + sy)
    };
    let seen = |x: f32, reach: f32| {
        let sx = frame.at(x, 0.0).0;
        sx + reach >= 0.0 && sx - reach <= width
    };
    if frame.water && matches!(frame.shown_weather(), Weather::Rain | Weather::Storm) {
        let front = screen(0.0, frame.front).1;
        let reach = frame.figure_h * z * 0.9;
        let [r, g, b] = shadow_ink(frame.hour);
        let wet = Hsla::from(gpui::Rgba { r, g, b, a: 1.0 });
        window.gradient(
            ox,
            front - reach,
            width,
            reach,
            180.0,
            (wet.opacity(0.0), 0.0),
            (wet.opacity(0.2), 1.0),
        );
        let sheen = gpui::white().opacity(if night { 0.05 } else { 0.1 });
        for streak in 0..14 {
            let x =
                ox + ((streak as f32 * 197.0 - frame.pan() * z).rem_euclid(width + 60.0)) - 30.0;
            let y = front - reach * (0.15 + 0.6 * ((streak * 7) % 5) as f32 / 5.0);
            window.rect(x, y, 14.0 * z.min(1.5), 1.5, 0.75, sheen);
        }
    }
    let opacity = mark::plot_opacity(z);
    if opacity > 0.01 {
        for plot in &frame.plots {
            if !seen(plot.x, plot.w * z) {
                continue;
            }
            let (x, y) = screen(plot.x, plot.y);
            mark::paint_plot(
                &mut Tint::new(window, light),
                x,
                y,
                plot.w * z,
                opacity,
                frame.hot_plot == Some(plot.plot),
                t + plot.plot as f32,
            );
        }
    }
    for building in &frame.buildings {
        let Some(glow) = building.glow else {
            continue;
        };
        if !seen(building.x, building.w * z) {
            continue;
        }
        let (x, base) = screen(building.x, building.base);
        let (w, h) = (building.w * z, building.h * z);
        window.soft(
            x,
            base,
            w * 0.72,
            h * 0.2,
            h * 0.34,
            ground_glow(glow, 0.42),
        );
    }
    for thing in &frame.things {
        let Some(glow) = thing.glow.filter(|_| !live_things.contains(&thing.index)) else {
            continue;
        };
        if !seen(thing.x, thing.w * z) {
            continue;
        }
        let (x, base) = screen(thing.x, thing.base);
        let w = thing.w * z * (0.6 + 0.4 * ease(thing.grow));
        window.soft(
            x,
            base,
            w * 0.8,
            w * 0.16,
            w * 0.12,
            ground_glow(glow, 0.35),
        );
    }
    let _ = height;
    for person in &frame.people {
        paint_person_ground(window, frame, person, (ox, oy));
    }
}

/// Works out what a frame of the World window draws, back to front: the
/// sky and its clouds, the far hills, and for each row of the place (or
/// the whole place) its land, what lies on it, the shadows, everything
/// standing on it in depth, the smoke and what stands in front of all;
/// two lights cross-fading where the hour falls between them. Returns
/// the plan, whether anything of the place was shown, and whether every
/// resident was drawn in their own pose.
fn plan_window(hand: &mut Stagehand, frame: &Arc<Frame>, wants: &[Want]) -> (Plan, (bool, bool)) {
    let (vw, vh) = (frame.view_w, frame.height);
    let (a, b, t) = Light::blend(frame.hour);
    let Some(main) = wants.first().copied() else {
        return (Plan::default(), (false, true));
    };
    let fading = t > 0.002 && b != a;
    let first = hand.pick(&main).map(slot_view);
    let Some(first) = first else {
        return (Plan::default(), (false, true));
    };
    let second = fading
        .then(|| hand.pick(&Want { light: b, ..main }))
        .flatten()
        .filter(|slot| slot.held.light != first.light)
        .map(slot_view);
    let fold_land = |light: Light| {
        hand.pick(&Want {
            light,
            variant: Variant::FoldLand,
            ..main
        })
        .filter(|slot| slot.held.light == light)
        .map(|slot| slot.look.clone())
    };
    let looks = std::iter::once((first.clone(), 1.0))
        .chain(second.clone().map(|second| (second, t)))
        .map(|(view, opacity)| {
            let land = fold_land(view.light);
            (view, opacity, land)
        })
        .collect::<Vec<_>>();
    // Poses at the level of the looks wanted (the whole level only,
    // painting in place).
    let level = main.level;
    let seconds = frame.seconds;
    // Every other look ready, nearest first: what an item a look lacks is
    // drawn from (another level, another light) before it is drawn live.
    let mut spares = hand
        .slots
        .iter()
        .filter(|slot| slot.held.ready && slot.held.variant == Variant::Main)
        .map(slot_view)
        .collect::<Vec<_>>();
    spares.sort_by_key(|view| {
        (
            view.light != first.light,
            view.slot.frame.view_w != frame.view_w,
        )
    });
    let spare_for = |index: usize| -> Option<(LookView, f32, Option<Arc<Look>>)> {
        spares
            .iter()
            .find(|view| view.look.anchors.contains_key(&index))
            .map(|view| (view.clone(), 1.0, None))
    };
    let full = (0.0, 0.0, vw, vh);
    // Room for as many items as the last frame drew: growing the list a
    // piece at a time copies it again and again.
    let mut items: Vec<Item> = Vec::with_capacity(hand.items_hint + hand.items_hint / 8);
    let mut moving = 0;
    let mut live_drawn = 0;
    // The frame itself unless it folds into rows: a frame is every
    // resident with their drawing, and cloning it was a tenth of a
    // frame's work.
    let rows = match rows_of(frame) {
        Some(rows) => rows
            .into_iter()
            .map(|(part, span)| (Arc::new(part), span))
            .collect(),
        None => vec![(frame.clone(), (0.0, vw))],
    };
    let folded = rows.len() > 1;
    let back = rows[0].0.clone();
    // The sky, and its clouds.
    for (view, opacity, _) in &looks {
        for sprite in &view.look.sky {
            items.push(Item {
                opacity: *opacity,
                draw: Draw::image(sprite.image.clone(), full, None),
            });
        }
    }
    {
        let back = back.clone();
        items.push(Item {
            opacity: 1.0,
            draw: live(full, None, move |window, (ox, oy, w, h)| {
                // A scene's storm: cloud over the sun, before its clouds.
                super::staging::paint_overcast(window, &back, (ox, oy, w, h));
                paint_clouds(window, &back, ox, oy, w, h)
            }),
        });
    }
    // The far hills, and what is rising on the ridge.
    for (view, opacity, _) in &looks {
        let lens = lens_of(&view.slot, back.camera, frame);
        for draw in ls::hill_draws(&view.look, view.slot.band, &lens, view.slot.horizon) {
            items.push(Item {
                opacity: *opacity,
                draw,
            });
        }
    }
    {
        let back = back.clone();
        items.push(Item {
            opacity: 1.0,
            draw: live(full, None, move |window, (ox, oy, w, h)| {
                paint_rising(window, &back, ox, oy, w, h)
            }),
        });
    }
    let count = rows.len();
    let mut everyone_posed = true;
    for (row, (part, span)) in rows.into_iter().enumerate() {
        let clip = folded.then_some((span.0, 0.0, span.1 - span.0, vh));
        let cut = |draw: Draw| clipped(draw, clip);
        let flat = |items: &mut Vec<Item>, of: &dyn Fn(&LookView, &Lens) -> Vec<Draw>| {
            for (view, opacity, _) in &looks {
                let lens = lens_of(&view.slot, part.camera, frame);
                items.extend(of(view, &lens).into_iter().map(|draw| Item {
                    opacity: *opacity,
                    draw: cut(draw),
                }));
            }
        };
        // The land, and the light and glints on the water.
        for (view, opacity, land) in &looks {
            let lens = lens_of(&view.slot, part.camera, frame);
            let tiles = match land.as_ref().filter(|_| part.blend_top) {
                Some(land) => &land.land,
                None => &view.look.land,
            };
            items.extend(
                tiles
                    .iter()
                    .filter_map(|sprite| lens.draw(sprite))
                    .map(|draw| Item {
                        opacity: *opacity,
                        draw: cut(draw),
                    }),
            );
            let mut count = 0;
            items.extend(
                ls::water_draws(&view.look, &lens, frame.front, seconds, &mut count)
                    .into_iter()
                    .map(|draw| Item {
                        opacity: *opacity,
                        draw: cut(draw),
                    }),
            );
            if *opacity >= 1.0 {
                moving += count;
            }
        }
        // What lies on the ground, drawn live.
        let live_things = part
            .things
            .iter()
            .filter(|thing| draws_live(thing, &first.look, &part))
            .map(|thing| thing.index)
            .collect::<Vec<_>>();
        {
            let part = part.clone();
            items.push(Item {
                opacity: 1.0,
                draw: live(full, clip, move |window, scene| {
                    paint_ground_live(window, &part, scene, &live_things)
                }),
            });
        }
        flat(&mut items, &|view, lens| {
            view.look
                .shadows
                .iter()
                .filter_map(|sprite| lens.draw(sprite))
                .collect()
        });
        // Everything standing on the stage, in depth.
        let lens_first = lens_of(&first.slot, part.camera, frame);
        let screen_y = |x: f32, y: f32| lens_first.at(x, y).1;
        let mut deep = Vec::new();
        let both = |of: &dyn Fn(&LookView, &Lens) -> Vec<Draw>| -> Vec<Item> {
            looks
                .iter()
                .flat_map(|(view, opacity, _)| {
                    let lens = lens_of(&view.slot, part.camera, frame);
                    of(view, &lens).into_iter().map(|draw| Item {
                        opacity: *opacity,
                        draw: cut(draw),
                    })
                })
                .collect()
        };
        deep.push(Deep {
            // Half a figure behind the spine: whoever stands on the shore
            // stands over its lamps and poles, as the painter has them,
            // and whoever is further back stays behind them.
            at: screen_y(0.0, part.feet - part.figure_h * 0.5),
            rank: Rank::Props,
            who: 0,
            items: both(&|view, lens| {
                view.look
                    .back
                    .iter()
                    .filter_map(|sprite| lens.draw(sprite))
                    .collect()
            }),
        });
        let shares = [1.0 - if second.is_some() { t } else { 0.0 }, t];
        let light = frame.light();
        // Only what the window shows (and a little past it, for a halo).
        let on_screen = |x: f32, reach: f32| {
            let sx = lens_first.at(x, 0.0).0;
            sx + reach >= -vw * 0.1 && sx - reach <= vw * 1.1
        };
        for building in &part.buildings {
            if !on_screen(building.x, building.w * part.camera.zoom * 1.2) {
                continue;
            }
            let at = screen_y(building.x, building.base);
            let baked = first.look.anchors.get(&building.index).copied();
            let mut items = Vec::new();
            // The looks it is drawn from: the frame's own, or the nearest
            // other look that holds it.
            let spare: Vec<_>;
            let sources: &[_] = if baked.is_some() {
                &looks
            } else {
                spare = spare_for(building.index).into_iter().collect();
                &spare
            };
            if building.moving() || sources.is_empty() {
                // Rising into place, springing after a click, or not in
                // the look yet: drawn live.
                live_drawn += 1;
                let z = part.camera.zoom;
                let (x, base) = lens_first.at(building.x, building.base);
                let (w, h) = (building.w * z, building.h * z);
                let rect = (x - w * 0.8, base - h * 1.4, w * 1.6, h * 1.5);
                let (part, index) = (part.clone(), building.index);
                items.push(Item {
                    opacity: 1.0,
                    draw: live(rect, clip, move |window, (ox, oy, _, _)| {
                        if let Some(building) = part.buildings.iter().find(|b| b.index == index) {
                            let screen = |x: f32, y: f32| {
                                let (sx, sy) = part.at(x, y);
                                (ox + sx, oy + sy)
                            };
                            paint_building_live(window, &part, building, &screen, light);
                        }
                    }),
                });
            } else {
                for (n, (view, opacity, _)) in sources.iter().enumerate() {
                    let lens = lens_of(&view.slot, part.camera, frame);
                    let Some(anchor) = view.look.anchors.get(&building.index) else {
                        continue;
                    };
                    let shift = (building.x - anchor.x, building.base - anchor.base);
                    for sprite in view.look.building_sprites(building.index) {
                        if let Some(draw) = lens.draw_moved(sprite, shift, 1.0, (0.0, 0.0)) {
                            items.push(Item {
                                opacity: *opacity,
                                draw: cut(draw),
                            });
                        }
                    }
                    for (group, halo) in view.look.halos(building.index) {
                        if let Some(draw) = lens.draw_moved(halo, shift, 1.0, (0.0, 0.0)) {
                            items.push(Item {
                                opacity: shares[n] * ls::flicker(group, seconds),
                                draw: cut(draw),
                            });
                        }
                    }
                }
            }
            // A design the place wears, over it.
            if part.wearing.iter().any(|worn| worn.index == building.index)
                && part.pictures.contains_key(&building.index)
            {
                let z = part.camera.zoom;
                let (x, base) = lens_first.at(building.x, building.base);
                let (w, h) = (building.w * z, building.h * z);
                let rect = (x - w * 1.5, base - h * 1.6, w * 3.0, h * 1.9);
                let (part, index) = (part.clone(), building.index);
                items.push(Item {
                    opacity: 1.0,
                    draw: live(rect, clip, move |window, (ox, oy, _, _)| {
                        let screen = |x: f32, y: f32| {
                            let (sx, sy) = part.at(x, y);
                            (ox + sx, oy + sy)
                        };
                        paint_worn_places(window, &part, &screen, light, Some(index));
                    }),
                });
            }
            deep.push(Deep {
                at,
                rank: Rank::Building,
                who: building.index,
                items,
            });
        }
        for thing in &part.things {
            if !on_screen(thing.x, thing.w * part.camera.zoom * 1.6) {
                continue;
            }
            // What lies flat (a pier, a sea wall's landing) is ground,
            // under everyone standing on it or behind it.
            let at = if ls::lies_flat(thing.shape, thing.base, part.front) {
                f32::NEG_INFINITY
            } else {
                screen_y(thing.x, thing.base)
            };
            let mut items = Vec::new();
            let spare: Vec<_>;
            let sources: &[_] = if first.look.anchors.contains_key(&thing.index) {
                &looks
            } else {
                spare = spare_for(thing.index).into_iter().collect();
                &spare
            };
            if draws_live(thing, &first.look, &part)
                && (thing.grow < 1.0 || sources.is_empty() || worn(thing, &part))
            {
                live_drawn += 1;
                let z = part.camera.zoom;
                let (x, base) = lens_first.at(thing.x, thing.base);
                let w = thing.w * z;
                let rect = (x - w * 1.6, base - w * 3.2, w * 3.2, w * 3.6);
                let (part, index) = (part.clone(), thing.index);
                items.push(Item {
                    opacity: 1.0,
                    draw: live(rect, clip, move |window, (ox, oy, _, _)| {
                        if let Some(thing) = part.things.iter().find(|t| t.index == index) {
                            let screen = |x: f32, y: f32| {
                                let (sx, sy) = part.at(x, y);
                                (ox + sx, oy + sy)
                            };
                            paint_thing_live(window, &part, thing, &screen, light);
                        }
                    }),
                });
            } else {
                for (view, opacity, _) in sources {
                    let lens = lens_of(&view.slot, part.camera, frame);
                    let Some(anchor) = view.look.anchors.get(&thing.index) else {
                        continue;
                    };
                    let shift = (thing.x - anchor.x, thing.base - anchor.base);
                    for sprite in view.look.thing_sprites(thing.index) {
                        if let Some(draw) = lens.draw_moved(sprite, shift, 1.0, (0.0, 0.0)) {
                            items.push(Item {
                                opacity: *opacity,
                                draw: cut(draw),
                            });
                        }
                    }
                }
                if thing.shape == MarkShape::Boat {
                    moving += 1;
                }
            }
            deep.push(Deep {
                at,
                rank: Rank::Thing,
                who: thing.index,
                items,
            });
        }
        for person in &part.people {
            if person.x + person.height < 0.0 || person.x - person.height > vw {
                continue;
            }
            let mut items = Vec::new();
            let mut posed = true;
            for (view, opacity, _) in &looks {
                let Some(sprite) = hand.pose(person, level, view.light) else {
                    posed = false;
                    continue;
                };
                let k = person.height
                    / (view.slot.frame.figure_h * person.figure.age.height()).max(0.01);
                // A hop lifts them; a step's rise is in the step itself.
                let lift = if person.pose.stride.is_some() {
                    0.0
                } else {
                    person.pose.bob
                };
                let (sx, sy, w, h) = sprite.rect;
                let rect = (person.x + sx * k, person.y - lift + sy * k, w * k, h * k);
                if rect.0 > vw || rect.0 + rect.2 < 0.0 {
                    continue;
                }
                items.push(Item {
                    opacity: *opacity,
                    draw: cut(Draw::image(sprite.image.clone(), rect, None)),
                });
            }
            if !posed && items.is_empty() {
                // Nobody's pose is baked yet: drawn live, as the painter has them.
                live_drawn += 1;
                let person = person.clone();
                let h = person.height;
                let rect = (person.x - h, person.y - h * 1.5, h * 2.0, h * 1.6);
                let part = part.clone();
                items.push(Item {
                    opacity: 1.0,
                    draw: live(rect, clip, move |window, (ox, oy, _, _)| {
                        let (x, y) = (ox + person.x, oy + person.y);
                        window.soft(x, y, h * 0.2, h * 0.045, h * 0.05, contact_ink(&part));
                        paint_person_body(window, &part, &person, (x, y), light);
                    }),
                });
            }
            everyone_posed &= posed;
            if person.pose.stride.is_some() {
                moving += 1;
            }
            deep.push(Deep {
                at: person.y,
                rank: Rank::Person,
                who: person.index,
                items,
            });
        }
        // A scene's props, live, in depth with everyone: what is held just
        // in front of whoever holds it, bunting over everyone's heads.
        for (n, prop) in part.scene_props().iter().enumerate() {
            let z = part.camera.zoom;
            let (x, foot) = lens_first.at(prop.x, prop.y);
            let (w, h) = (prop.w * z, prop.h * z);
            if x + w < -vw * 0.1 || x - w > vw * 1.1 {
                continue;
            }
            let at = match (prop.prop, prop.held_by) {
                // Bunting stands on its poles: whoever is nearer stands in
                // front of them.
                (world_projection::Prop::Bunting, _) => foot,
                (_, Some(_)) => foot + 0.01,
                // Scaffolding stands just in front of what it is round.
                (world_projection::Prop::Scaffold, _) => foot + 0.5,
                _ => foot,
            };
            live_drawn += 1;
            let rect = (
                x - w * 0.8,
                foot - h * 1.3 - z * 20.0,
                w * 1.6,
                h * 1.5 + z * 24.0,
            );
            let (prop, fig, setting) = (*prop, part.figure_h * z, part.setting);
            let seconds = part.seconds;
            deep.push(Deep {
                at,
                rank: Rank::Person,
                who: usize::MAX - n,
                items: vec![Item {
                    opacity: 1.0,
                    draw: live(rect, clip, move |window, (ox, oy, _, _)| {
                        let mut tinted = Tint::new(window, light);
                        super::staging::paint_prop(
                            &mut tinted,
                            &prop,
                            (ox + x, oy + foot),
                            z,
                            fig,
                            setting,
                            seconds,
                        );
                    }),
                }],
            });
        }
        items.extend(ls::in_depth(deep));
        // Smoke from the chimneys.
        if part.weather != Weather::Storm {
            for (view, opacity, _) in &looks {
                let lens = lens_of(&view.slot, part.camera, frame);
                let chimneys = view.chimneys();
                let mut count = 0;
                items.extend(
                    ls::smoke_draws(
                        &view.look,
                        &lens,
                        &chimneys,
                        view.slot.building_w,
                        wind(&view.slot.frame),
                        seconds,
                        &mut count,
                    )
                    .into_iter()
                    .map(|draw| Item {
                        opacity: *opacity,
                        draw: cut(draw),
                    }),
                );
                if *opacity >= 1.0 {
                    moving += count;
                }
            }
        }
        // What stands in front of everyone: the framing strip, bunting,
        // the beacon's lamp.
        flat(&mut items, &|view, lens| {
            view.look
                .front
                .iter()
                .filter_map(|sprite| lens.draw(sprite))
                .collect()
        });
        // Bonds over the people's heads, and the haze over a row with
        // another in front of it.
        let haze = folded && row + 1 < count;
        if !part.bonds.is_empty() || haze {
            let part = part.clone();
            items.push(Item {
                opacity: 1.0,
                draw: live(full, clip, move |window, (ox, oy, _, h)| {
                    for (x, y, r, tone) in &part.bonds {
                        art::paint_bond(window, ox + x, oy + y, *r, *tone);
                    }
                    if haze {
                        paint_haze(window, &part, ox, oy, span, h);
                    }
                }),
            });
        }
    }
    hand.tags.live += live_drawn;
    hand.items_hint = items.len();
    (
        Plan {
            layers: ls::compose(items),
            moving,
        },
        (true, everyone_posed),
    )
}

/// Whether a thing wears a design whose picture is ready (a flag, a sail).
fn worn(thing: &ThingPaint, frame: &Frame) -> bool {
    frame.wearing.iter().any(|worn| worn.index == thing.index)
        && frame.pictures.contains_key(&thing.index)
}

/// Whether a thing is drawn live rather than from its baked picture: it
/// wears a design (a flag, a sail), it is rising into place, or the look
/// does not hold it yet.
fn draws_live(thing: &ThingPaint, look: &Look, frame: &Frame) -> bool {
    thing.grow < 1.0
        || !look.anchors.contains_key(&thing.index)
        || (frame.wearing.iter().any(|worn| worn.index == thing.index)
            && frame.pictures.contains_key(&thing.index))
}

/// A look as a frame draws it: the slot, its look and its light.
#[derive(Clone)]
struct LookView {
    slot: Rc<SlotView>,
    look: Arc<Look>,
    light: Light,
}

/// What a frame needs of a slot.
struct SlotView {
    frame: Arc<Frame>,
    view: (f32, f32),
    width: f32,
    band: (f32, f32, f32, f32),
    horizon: f32,
    building_w: f32,
}

impl std::ops::Deref for LookView {
    type Target = SlotView;
    fn deref(&self) -> &SlotView {
        &self.slot
    }
}

impl LookView {
    /// The chimneys of the place as it was baked: each pot, and how big
    /// its cottage is beside the usual.
    fn chimneys(&self) -> Vec<(f32, f32, f32)> {
        let frame = &self.slot.frame;
        frame
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
                (x, y, b.w / self.slot.building_w.max(1.0))
            })
            .collect()
    }
}

fn slot_view(slot: &Slot) -> LookView {
    LookView {
        slot: Rc::new(SlotView {
            frame: slot.frame.clone(),
            view: slot.view,
            width: slot.width,
            band: slot.band,
            horizon: slot.horizon,
            building_w: slot.building_w,
        }),
        look: slot.look.clone(),
        light: slot.held.light,
    }
}

/// The lens a look is drawn through for `camera`: the camera itself, or
/// for a look baked for another size of window (the window was resized
/// and the look of its new size is on its way), the camera scaled to it.
fn lens_of(slot: &SlotView, camera: Camera, frame: &Frame) -> Lens {
    let (vw, vh) = (frame.view_w, frame.height);
    let sy = vh / slot.view.1.max(1.0);
    let sx = frame.width / slot.width.max(1.0);
    Lens {
        x: camera.x / sx.max(0.01),
        y: camera.y / sy.max(0.01),
        zoom: camera.zoom * sy,
        view: (vw, vh),
    }
}

thread_local! {
    /// Each window's living stage, by window.
    static HANDS: std::cell::RefCell<HashMap<u64, Stagehand>> =
        std::cell::RefCell::new(HashMap::new());
}

/// Lets go of the living stages of the windows `gone`.
pub(super) fn forget(gone: &std::collections::HashSet<u64>) {
    HANDS.with(|hands| {
        hands
            .borrow_mut()
            .retain(|window, _| !gone.contains(window))
    });
}

/// How many bytes of pixels the living stage of `window` holds now, and
/// the most it has held.
pub fn stage_bytes(window: &Window) -> (usize, usize) {
    let id = window.window_handle().window_id().as_u64();
    HANDS.with(|hands| {
        hands
            .borrow_mut()
            .get_mut(&id)
            .map_or((0, 0), |hand| (hand.bytes(), hand.peak))
    })
}

/// The looks the living stage of `window` has baked.
pub fn bake_log(window: &Window) -> Vec<BakeLog> {
    let id = window.window_handle().window_id().as_u64();
    HANDS.with(|hands| {
        hands
            .borrow()
            .get(&id)
            .map_or(Vec::new(), |hand| hand.log.clone())
    })
}

/// The scene as the World window shows it: the living stage, its looks
/// baked off the window's thread as the camera needs them, and over it
/// the weather, the loading wash (only before the first look of a window
/// is ready) and the vignette.
pub fn living_scene(frame: Frame, snapshot: &ProjectionSnapshot, window: &mut Window) -> gpui::Div {
    use gpui::{ParentElement, Styled};
    let mut frame = frame;
    let sync = painter::synchronous();
    painter::begin_frame(!sync);
    painter::sweep(window);
    fetch_pictures(&mut frame, window, sync);
    let id = window.window_handle().window_id().as_u64();
    let dpr = display_scale(window);
    let frame = Arc::new(frame);
    let (plan, ahead, gone, report) = HANDS.with(|hands| {
        let mut hands = hands.borrow_mut();
        let hand = hands.entry(id).or_insert_with(|| Stagehand::new(dpr));
        if (hand.dpr - dpr).abs() > 1e-3 {
            // Another display: every look is baked again at its scale.
            *hand = Stagehand::new(dpr);
        }
        let (handed, took) = HANDED
            .with(|handed| handed.take())
            .filter(|(window, ..)| *window == id)
            .map_or((0, 0.0), |(_, bytes, took)| (bytes, took));
        hand.learn_upload(handed, took);
        let out = hand.frame(&frame, snapshot, sync);
        LAST_TAGS.with(|last| last.set(hand.tags));
        out
    });
    for image in &gone {
        let _ = window.drop_image(image.clone());
    }
    let_go_pictures(gone);
    LAST_PLAN.with(|last| last.set((plan.images(), plan.moving, plan.layers.len())));
    let wash = if sync {
        0.0
    } else {
        wash_over(window, report.shown, frame.still)
    };
    note_stage(window, &frame, report.shown, report.settled);
    if !sync {
        log_frame(window, wash, &frame);
    }
    if !sync && (report.waiting || wash > 0.0 || !painter::idle()) {
        window.request_animation_frame();
    }
    let mut root = ls::element(Rc::new(plan)).absolute().top_0().left_0();
    if !ahead.is_empty() {
        root = root.child(
            gpui::canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    // Timed, for how much the next frames hand over.
                    let began = Instant::now();
                    ls::warm(&ahead, bounds, window);
                    let took = began.elapsed().as_secs_f64() * 1000.0;
                    let bytes = ahead
                        .iter()
                        .map(|image| {
                            let size = image.size(0);
                            (size.width.0.max(0) as usize) * (size.height.0.max(0) as usize) * 4
                        })
                        .sum();
                    HANDED.with(|handed| handed.set(Some((id, bytes, took))));
                },
            )
            .absolute()
            .size_full(),
        );
    }
    let top = frame.clone();
    root.child(
        gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window: &mut Window, _| {
                let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
                let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                if wash > 0.0 {
                    paint_wash(window, &top, (ox, oy, w, h), wash);
                }
                // A scene's storm: the light cooler and dimmer, its
                // lamps lit through it, its rain heavy.
                super::staging::paint_storm(window, &top, (ox, oy, w, h));
                paint_weather(window, &top, ox, oy, w, h, (h / 848.0).clamp(0.3, 1.3));
                let bounds = Bounds::new(point(px(ox), px(oy)), size(px(w), px(h)));
                if painter::synchronous() {
                    paint_vignette_image(window, bounds);
                } else {
                    let dpr = window.scale_factor().max(0.5);
                    let mut key = Key::new("vignette");
                    key.float(w).float(h).float(dpr);
                    let key = key.finish();
                    match painter::ready(key) {
                        Some(painter::Ready::Image(image, _)) => {
                            let _ = window.paint_image(
                                bounds,
                                bounds,
                                Corners::default(),
                                image,
                                0,
                                false,
                            );
                        }
                        _ => painter::want(
                            window,
                            key,
                            false,
                            Box::new(move || paint_vignette(w, h, dpr * 0.25)),
                        ),
                    }
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full(),
    )
}

thread_local! {
    /// What the last frame of the living stage drew on this thread: its
    /// images, how many moved, and its passes.
    static LAST_PLAN: std::cell::Cell<(usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0)) };
}

/// What the last frame of the living stage drew on this thread: its
/// images, how many of them moved, and how many passes it took.
pub fn last_plan() -> (usize, usize, usize) {
    LAST_PLAN.with(|last| last.get())
}

thread_local! {
    /// What the last frame of the living stage on this thread did beside
    /// drawing.
    static LAST_TAGS: std::cell::Cell<FrameTags> = std::cell::Cell::new(FrameTags::default());
    /// The pictures the last frame handed to the display: in which
    /// window, their bytes, and how long that took (milliseconds).
    static HANDED: std::cell::Cell<Option<(u64, usize, f64)>> = const { std::cell::Cell::new(None) };
}

/// What the last frame of the living stage on this thread did beside
/// drawing (the World laid out again is the caller's to tell).
pub fn last_tags() -> FrameTags {
    LAST_TAGS.with(|last| last.get())
}

thread_local! {
    /// The display scale the living stage bakes for, if not the window's
    /// own: a benchmark on a 1x display baking as a Retina one would.
    static DISPLAY_SCALE: std::cell::Cell<Option<f32>> = const { std::cell::Cell::new(None) };
}

/// Bakes the living stage of every window on this thread for a display
/// of `scale` device pixels a point (none: the window's own).
pub fn set_display_scale(scale: Option<f32>) {
    DISPLAY_SCALE.with(|cell| cell.set(scale));
}

fn display_scale(window: &Window) -> f32 {
    DISPLAY_SCALE
        .with(|cell| cell.get())
        .unwrap_or_else(|| window.scale_factor())
        .max(0.5)
}

/// The living stage benchmark as the World window has it: a place's
/// residents where their routines put them, the camera panning along it
/// and zooming in and out, through a day (or at one hour), every look
/// baked lazily and let go, each frame measured.
pub struct StageBench {
    pub snapshot: Arc<ProjectionSnapshot>,
    pub time: ls::Time,
    pub stats: Rc<std::cell::RefCell<ls::Stats>>,
    /// Held at one hour, or through a day every forty seconds.
    pub hour: Option<f32>,
    /// Stops measuring after this long.
    pub until: Option<Duration>,
    pub done: Rc<std::cell::Cell<bool>>,
    /// The pixels the stage held at the last frame, and the most it held
    /// (bytes), and how many frames until it first settled.
    pub memory: Rc<std::cell::Cell<(usize, usize)>>,
    pub opened: Rc<std::cell::Cell<Option<usize>>>,
    /// The looks baked, as the stage logs them.
    pub bakes: Rc<std::cell::RefCell<Vec<BakeLog>>>,
    /// What each measured frame did beside drawing, in step with
    /// `stats.planned`.
    pub tags: Rc<std::cell::RefCell<Vec<FrameTags>>>,
    /// A scene played over and over, as a return film plays it: its cast
    /// staged, its props out and its camera instead of the pan.
    pub scene: Option<world_projection::MomentKind>,
    frames: usize,
}

impl StageBench {
    pub fn new(
        snapshot: Arc<ProjectionSnapshot>,
        time: ls::Time,
        hour: Option<f32>,
        until: Option<Duration>,
    ) -> Self {
        Self {
            snapshot,
            time,
            stats: Rc::default(),
            hour,
            until,
            done: Rc::default(),
            memory: Rc::default(),
            opened: Rc::default(),
            bakes: Rc::default(),
            tags: Rc::default(),
            scene: None,
            frames: 0,
        }
    }

    /// The camera `seconds` into the run: at rest, then panning along the
    /// place zoomed in, then out to rest again, every twenty seconds.
    pub fn camera(stage: &Stage, seconds: f32) -> Camera {
        let phase = (seconds / 20.0).fract();
        let zoom = 1.0 + 1.2 * (phase * std::f32::consts::TAU).sin().max(0.0);
        let x = stage.width / 2.0 + (stage.width / 2.0) * 0.8 * (seconds * 0.05).sin();
        Camera::around(stage, zoom, x, stage.height * 0.6)
    }
}

/// A line for a log of the looks a stage baked: how many, how long they
/// took in all, and how many of their pictures were taken from a look
/// baked before.
pub fn bake_summary(log: &[BakeLog]) -> String {
    let pictures = log.iter().map(|bake| bake.pictures).sum::<usize>();
    let taken = log.iter().map(|bake| bake.taken).sum::<usize>();
    format!(
        "looks baked {} ({:.0} ms in all, {:.0} MB) | pictures {pictures}, {taken} taken from a look baked before",
        log.len(),
        log.iter().map(|bake| bake.took.as_secs_f64() * 1000.0).sum::<f64>(),
        log.iter().map(|bake| bake.bytes).sum::<usize>() as f64 / 1e6,
    )
}

/// What a frame did, as a log has it in a few words: `-` for nothing.
pub fn tags_line(tags: &FrameTags) -> String {
    let mut out = Vec::new();
    if tags.baking > 0 {
        out.push(format!("bake x{}", tags.baking));
    }
    if tags.arrived {
        out.push("arrived".to_string());
    }
    if tags.swapped {
        out.push("swap".to_string());
    }
    if tags.uploaded > 0 {
        out.push(format!(
            "upload {} ({:.1} MB)",
            tags.uploaded,
            tags.uploaded_bytes as f64 / 1e6
        ));
    }
    if tags.released > 0 {
        out.push(format!("release {}", tags.released));
    }
    if tags.live > 0 {
        out.push(format!("live {}", tags.live));
    }
    if tags.stand_ins > 0 {
        out.push(format!("stand-ins {}", tags.stand_ins));
    }
    if tags.laid_out {
        out.push("layout".to_string());
    }
    if out.is_empty() {
        "-".to_string()
    } else {
        out.join(", ")
    }
}

/// The likeliest cause among what a frame and the frame before did, most
/// specific first: a look shown, pictures handed over or forgotten, a
/// look taken in, the World laid out again, something drawn live, a bake
/// beside it, or none of these.
pub fn cause_of(this: &FrameTags, before: &FrameTags) -> &'static str {
    let either = |of: &dyn Fn(&FrameTags) -> bool| of(this) || of(before);
    if either(&|tags| tags.swapped) {
        "swap"
    } else if either(&|tags| tags.uploaded > 0) {
        "upload"
    } else if either(&|tags| tags.released > 0) {
        "release"
    } else if either(&|tags| tags.arrived) {
        "arrived"
    } else if either(&|tags| tags.laid_out) {
        "layout"
    } else if either(&|tags| tags.live > 0) {
        "live"
    } else if either(&|tags| tags.baking > 0) {
        "bake"
    } else {
        "none"
    }
}

/// The causes in the order [`cause_of`] tries them.
const CAUSES: [&str; 8] = [
    "swap", "upload", "release", "arrived", "layout", "live", "bake", "none",
];

/// Lines telling what came with each late frame (an interval between
/// frames over `interval_ms`) and each slow plan (over `plan_ms`): how
/// many of each had each likeliest cause, and the five worst with what
/// that frame and the one before did. `tags` is in step with
/// `stats.planned`; an interval is laid to the frame it follows.
pub fn hitch_report(
    stats: &ls::Stats,
    tags: &[FrameTags],
    interval_ms: f64,
    plan_ms: f64,
) -> String {
    let none = FrameTags::default();
    let at = |frame: usize| tags.get(frame).unwrap_or(&none);
    let before = |frame: usize| frame.checked_sub(1).map_or(&none, at);
    let section = |name: String, late: Vec<(usize, f64)>| -> String {
        let mut counts = [0usize; CAUSES.len()];
        for (frame, _) in &late {
            let cause = cause_of(at(*frame), before(*frame));
            if let Some(index) = CAUSES.iter().position(|known| *known == cause) {
                counts[index] += 1;
            }
        }
        let mut out = format!(
            "{name}: {} — {}",
            late.len(),
            CAUSES
                .iter()
                .zip(counts)
                .map(|(cause, count)| format!("{cause} {count}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let mut worst = late;
        worst.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (frame, ms) in worst.into_iter().take(5) {
            out.push_str(&format!(
                "\n    {ms:.1} ms at frame {frame}: [{}] after [{}]",
                tags_line(at(frame)),
                tags_line(before(frame)),
            ));
        }
        out
    };
    // The interval from frame n to frame n + 1 is frame n's to answer for.
    let late = stats
        .between
        .iter()
        .enumerate()
        .filter(|(_, ms)| **ms > interval_ms)
        .map(|(frame, ms)| (frame, *ms))
        .collect();
    let slow = stats
        .planned
        .iter()
        .enumerate()
        .filter(|(_, ms)| **ms > plan_ms)
        .map(|(frame, ms)| (frame, *ms))
        .collect();
    let frames = tags.len().max(1) as f64;
    let share = |of: &dyn Fn(&FrameTags) -> bool| {
        100.0 * tags.iter().filter(|tags| of(tags)).count() as f64 / frames
    };
    format!(
        "{}\n  {}\n  of all frames: bake {:.0}%, upload {:.0}%, swap {:.0}%, release {:.0}%, live {:.0}%, layout {:.0}%",
        section(format!("hitches (intervals over {interval_ms:.1} ms)"), late),
        section(format!("slow plans (over {plan_ms:.1} ms)"), slow),
        share(&|tags| tags.baking > 0),
        share(&|tags| tags.uploaded > 0),
        share(&|tags| tags.swapped),
        share(&|tags| tags.released > 0),
        share(&|tags| tags.live > 0),
        share(&|tags| tags.laid_out),
    )
}

impl gpui::Render for StageBench {
    fn render(
        &mut self,
        window: &mut Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled};
        let started = Instant::now();
        let seconds = match &self.time {
            ls::Time::Real(began) => began.elapsed().as_secs_f32(),
            ls::Time::Set(cell) => cell.get(),
        };
        let size = window.viewport_size();
        let (w, h) = (f32::from(size.width), f32::from(size.height));
        let hour = self
            .hour
            .unwrap_or(12.0 + seconds * 24.0 / 40.0)
            .rem_euclid(24.0);
        let daylight = crate::scene::daylight_at(hour as u32);
        let laid = super::layouts_laid();
        let mut stage = super::stage(&self.snapshot, w, h);
        let laid_out = super::layouts_laid() != laid;
        // A scene, if one plays: staged as the window stages a return
        // film's beat, every frame.
        let staged = self
            .scene
            .and_then(|kind| super::staging::sample(&self.snapshot, &stage, kind))
            .map(|script| {
                let t = seconds.rem_euclid(script.seconds);
                super::staging::stage_scene(&script, &stage, &self.snapshot, t)
            });
        if let Some(staged) = &staged {
            stage = super::staging::cast_on(&stage, staged);
        }
        let living = super::living_staged(
            &stage,
            &self.snapshot,
            seconds,
            daylight,
            &Default::default(),
            None,
            staged.as_ref(),
        );
        let camera = staged
            .as_ref()
            .and_then(|staged| staged.camera)
            .unwrap_or_else(|| Self::camera(&stage, seconds));
        let mut frame = super::frame(
            &self.snapshot,
            &stage,
            &living,
            camera,
            seconds,
            daylight,
            &Glows::new(),
            1.0,
        )
        .at_hour(hour);
        if let Some(staged) = &staged {
            frame = frame.dressed(staged.dress());
        }
        let scene = living_scene(frame, &self.snapshot, window);
        let (bytes, peak) = stage_bytes(window);
        self.memory.set((bytes, peak));
        *self.bakes.borrow_mut() = bake_log(window);
        self.frames += 1;
        let settled = super::settled(window) && super::painted(window);
        if self.opened.get().is_none() {
            if settled {
                self.opened.set(Some(self.frames));
                // The measured run's clock starts once the place is shown.
                if let ls::Time::Real(began) = &mut self.time {
                    *began = Instant::now();
                }
            }
            window.request_animation_frame();
            return gpui::div().size_full().child(scene);
        }
        {
            let mut stats = self.stats.borrow_mut();
            if let Some(last) = stats.last {
                stats.between.push((started - last).as_secs_f64() * 1000.0);
            }
            stats.started.get_or_insert(started);
            stats.last = Some(started);
            let (images, moving, _) = last_plan();
            stats.images.push(images);
            stats.moving.push(moving);
            self.tags.borrow_mut().push(FrameTags {
                laid_out,
                ..last_tags()
            });
        }
        let finished = self.until.zip(match &self.time {
            ls::Time::Real(began) => Some(began.elapsed()),
            ls::Time::Set(_) => None,
        });
        if finished.is_some_and(|(until, elapsed)| elapsed >= until) {
            self.done.set(true);
        } else {
            window.request_animation_frame();
        }
        let stats = self.stats.clone();
        gpui::div().size_full().child(scene).child(
            gpui::canvas(
                |_, _, _| (),
                move |_, _, _, _| {
                    stats
                        .borrow_mut()
                        .planned
                        .push(started.elapsed().as_secs_f64() * 1000.0);
                },
            )
            .absolute()
            .size_0(),
        )
    }
}
