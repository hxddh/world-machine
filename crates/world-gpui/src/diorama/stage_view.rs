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
//! The tile painter (`interface::scene`) still paints the strip, covers,
//! postcards and the golden pictures; the World window no longer uses it.
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
    let (_, soon, _) = Light::blend(hour + 0.25);
    if soon != a && !(fading && soon == b) {
        out.push(ahead(main(soon, level, view)));
    }
    out
}

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
            step: person.pose.stride.map(|stride| {
                ((stride.rem_euclid(1.0) * ls::WALK_FRAMES as f32) as u8)
                    .min(ls::WALK_FRAMES as u8 - 1)
            }),
            carrying: person.carrying.is_some(),
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
    let pose = match key.step {
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

/// What a bake hands back: the look, the frame it was baked from, and a
/// cottage's width on its stage.
type Baked = (Look, Frame, f32);

/// A look being baked off the window's thread.
struct Baking {
    held: Held,
    view: (f32, f32),
    out: Arc<Mutex<Option<Baked>>>,
    started: Instant,
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

/// How many bytes of new pictures a frame hands to the display: enough
/// that a look arrives in a few frames, few enough that no frame hitches.
const HAND_PER_FRAME: usize = 24 << 20;

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
    pub log: Vec<(Held, Duration, usize)>,
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
        }
    }

    /// The pixels it holds, in bytes: every look, and every pose.
    pub fn bytes(&self) -> usize {
        self.slots
            .iter()
            .map(|slot| slot.look.bytes())
            .sum::<usize>()
            + self.pose_bytes()
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
            }
        }
        let posed = self
            .posing
            .as_ref()
            .and_then(|posing| posing.out.lock().ok().and_then(|mut out| out.take()));
        if let Some(posed) = posed {
            let now = Instant::now();
            for (key, sprite) in posed {
                self.poses.insert(key, Posed { sprite, used: now });
            }
            self.posing = None;
        }
    }

    /// Holds a look just baked, not yet ready until the display has it.
    fn take(
        &mut self,
        held: Held,
        view: (f32, f32),
        (look, frame, building_w): Baked,
        took: Duration,
    ) {
        let images = look.images();
        let band = Band::of(&frame, view.0, view.1);
        self.log.push((held, took, look.bytes()));
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
            handed: 0,
            took,
        });
        self.peak = self.peak.max(self.bytes());
    }

    /// Starts baking the next look wanted, off the thread, or right here
    /// (`now`: a test, a cover).
    fn schedule(&mut self, wants: &[Want], now: bool) {
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
            if now {
                // Right here, only what this frame shows.
                if !want.shown {
                    return;
                }
                let baked = bake_one(&snapshot, view, want.light, scale, cut);
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
            let spawned = std::thread::Builder::new()
                .name("living-stage-bake".into())
                .spawn(move || {
                    let baked = bake_one(&snapshot, view, want.light, scale, cut);
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
                });
            }
            return;
        }
    }

    /// Lets go of every look no want keeps, returning their pictures for
    /// the display to forget.
    fn evict(&mut self, wants: &[Want]) -> Vec<Arc<RenderImage>> {
        let keep = keep(&self.held(), wants);
        let mut gone = Vec::new();
        let mut at = 0;
        self.slots.retain(|slot| {
            let kept = keep[at];
            at += 1;
            if !kept {
                gone.extend(slot.images.iter().take(slot.handed).cloned());
            }
            kept
        });
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
    fn hand_over(&mut self, now: bool) -> Vec<Arc<RenderImage>> {
        let mut out = Vec::new();
        let mut budget = HAND_PER_FRAME;
        for slot in &mut self.slots {
            if slot.held.ready {
                continue;
            }
            if now {
                slot.handed = slot.images.len();
            }
            while slot.handed < slot.images.len() && budget > 0 {
                let image = slot.images[slot.handed].clone();
                let size = image.size(0);
                let bytes = (size.width.0.max(0) as usize) * (size.height.0.max(0) as usize) * 4;
                budget = budget.saturating_sub(bytes.max(1));
                out.push(image);
                slot.handed += 1;
            }
            if slot.handed >= slot.images.len() {
                slot.held.ready = true;
            }
        }
        out
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
        {
            self.asked.push((key, person.clone()));
        }
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
        let ahead = self.hand_over(now);
        let gone = self.evict(&wants);
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
            && self.posing.is_none();
        let waiting = !settled || held.iter().any(|h| !h.ready);
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

/// Bakes one look of `snapshot` for a window `view` large.
fn bake_one(
    snapshot: &ProjectionSnapshot,
    view: (f32, f32),
    light: Light,
    scale: f32,
    cut: Cut,
) -> Baked {
    let (frame, stage) = prepare(snapshot, view, light, cut.fold_land);
    let look = ls::bake_look(&frame, &stage, &[], light, scale, view, cut);
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
    if frame.water && matches!(frame.weather, Weather::Rain | Weather::Storm) {
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
    let full = (0.0, 0.0, vw, vh);
    let mut items: Vec<Item> = Vec::new();
    let mut moving = 0;
    let rows = rows_of(frame).unwrap_or_else(|| vec![((**frame).clone(), (0.0, vw))]);
    let folded = rows.len() > 1;
    let back = Arc::new(rows[0].0.clone());
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
        let part = Arc::new(part);
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
            if building.moving() || baked.is_none() {
                // Rising into place, springing after a click, or not in
                // the look yet: drawn live.
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
                for (n, (view, opacity, _)) in looks.iter().enumerate() {
                    let lens = lens_of(&view.slot, part.camera, frame);
                    let Some(anchor) = view.look.anchors.get(&building.index) else {
                        continue;
                    };
                    let shift = (building.x - anchor.x, building.base - anchor.base);
                    for (_, sprite) in view
                        .look
                        .buildings
                        .iter()
                        .filter(|(of, _)| *of == building.index)
                    {
                        if let Some(draw) = lens.draw_moved(sprite, shift, 1.0, (0.0, 0.0)) {
                            items.push(Item {
                                opacity: *opacity,
                                draw: cut(draw),
                            });
                        }
                    }
                    for (group, _, halo) in view
                        .look
                        .glow
                        .iter()
                        .filter(|(_, of, _)| *of == building.index)
                    {
                        if let Some(draw) = lens.draw_moved(halo, shift, 1.0, (0.0, 0.0)) {
                            items.push(Item {
                                opacity: shares[n] * ls::flicker(*group, seconds),
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
            if draws_live(thing, &first.look, &part) {
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
                for (view, opacity, _) in &looks {
                    let lens = lens_of(&view.slot, part.camera, frame);
                    let Some(anchor) = view.look.anchors.get(&thing.index) else {
                        continue;
                    };
                    let shift = (thing.x - anchor.x, thing.base - anchor.base);
                    for (_, _, sprite) in view
                        .look
                        .things
                        .iter()
                        .filter(|(of, _, _)| *of == thing.index)
                    {
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
    (
        Plan {
            layers: ls::compose(items),
            moving,
        },
        (true, everyone_posed),
    )
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
            .borrow()
            .get(&id)
            .map_or((0, 0), |hand| (hand.bytes(), hand.peak))
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
        hand.frame(&frame, snapshot, sync)
    });
    for image in gone {
        let _ = window.drop_image(image);
    }
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
                move |bounds, _, window, _| ls::warm(&ahead, bounds, window),
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
        let stage = super::stage(&self.snapshot, w, h);
        let living = super::living(
            &stage,
            &self.snapshot,
            seconds,
            daylight,
            &Default::default(),
            None,
        );
        let camera = Self::camera(&stage, seconds);
        let frame = super::frame(
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
        let scene = living_scene(frame, &self.snapshot, window);
        let (bytes, peak) = stage_bytes(window);
        self.memory.set((bytes, peak));
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
