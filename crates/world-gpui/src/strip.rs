//! A World as a band along the edge of a screen: its landscape in a strip
//! a hand tall (or, along the side of a screen, a column of such bands),
//! its people walking the length of it now and then, and the day's letter
//! arriving in it as a small envelope to click, or drag out, and read.
//! Click someone and they wave and say something: the World's own line for
//! them, read from what it shows and never sent back to it.
//!
//! A strip lives on the desktop all day, so it draws only while something
//! in it moves, never faster than [`MOST_FRAMES_A_SECOND`], not at all
//! while everything stands still, and neither draws nor wakes while it
//! cannot be seen (covered, minimised, or on a sleeping display). Everything here is presentation: who
//! walks where follows the local clock and a seed, never anything the World
//! records.

use crate::art::{self, Pose};
use crate::diorama::{self, Camera, Glows, Living, Stage};
use crate::scene::{self, Daylight};
use crate::ui;
use gpui::{
    canvas, div, point, prelude::*, px, size, App, Bounds, Context, FontWeight, Hsla, IntoElement,
    Pixels, Render, Role, SharedString, SpringConfig, SpringState, Styled, Subscription, Task,
    Window,
};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{Duration, Instant};
use world_projection::ProjectionSnapshot;
use world_projection::SelectionId;
use world_theme::tokens;

/// How tall a strip is, in points.
pub const HEIGHT: f32 = 140.0;

/// How wide a strip down the side of a screen is, in points.
pub const WIDTH: f32 = 180.0;

/// How long someone says their line after a click, in seconds.
pub const SAYING_SECONDS: f32 = 5.0;

/// How far the envelope must be dragged to open, in points.
pub const DRAG_OPENS: f32 = 16.0;

/// The most a strip ever draws in a second while something moves: under
/// fifteen, so a strip left on all day costs next to nothing.
pub const MOST_FRAMES_A_SECOND: u32 = 12;

/// The time between two frames while something moves.
pub const FRAME: Duration = Duration::from_nanos(1_000_000_000 / MOST_FRAMES_A_SECOND as u64);

/// While nothing moves, the strip still looks up this often, so the light
/// follows the hour. Not a frame of motion: one redraw.
pub const LIGHT_CHECK: Duration = Duration::from_secs(10 * 60);

/// One walk begins this often, by one resident at a time, in turn.
pub const OUTING_SECONDS: f32 = 30.0;

/// How long after the strip opens the first walk begins.
const LEAD_SECONDS: f32 = 3.0;

/// The longest a walk takes; a longer way is walked a little faster.
const LONGEST_WALK: f32 = OUTING_SECONDS * 0.6;

/// The longest a new letter takes to drop into place.
pub const LETTER_SECONDS: f32 = 1.6;

/// The spring a new letter drops on: a little under critically damped
/// (a damping ratio near 0.6), so it lands with one small bounce.
pub const LETTER_SPRING: SpringConfig = SpringConfig::new(90.0, 11.0, 1.0);

/// How close to rest, as a share of the drop, a letter counts as landed.
const LETTER_REST: f32 = 0.004;

/// Which edge of the screen a strip lies along.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum Edge {
    #[default]
    Bottom,
    Top,
    Left,
    Right,
}

impl Edge {
    pub const ALL: [Edge; 4] = [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right];

    /// Down the side of the screen rather than across it.
    pub fn vertical(self) -> bool {
        matches!(self, Edge::Left | Edge::Right)
    }

    pub fn id(self) -> &'static str {
        match self {
            Edge::Bottom => "bottom",
            Edge::Top => "top",
            Edge::Left => "left",
            Edge::Right => "right",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|edge| edge.id() == id)
    }

    /// What the strip's menu calls it.
    pub fn label(self) -> &'static str {
        match self {
            Edge::Bottom => "Along the Bottom",
            Edge::Top => "Along the Top",
            Edge::Left => "Down the Left",
            Edge::Right => "Down the Right",
        }
    }
}

/// Where a strip goes on a display whose usable area (without the menu bar
/// and the Dock) is `visible`: along the top or bottom the full width and
/// [`HEIGHT`] tall; down a side the full height and [`WIDTH`] wide.
pub fn band(visible: Bounds<Pixels>, edge: Edge) -> Bounds<Pixels> {
    let height = px(HEIGHT.min(f32::from(visible.size.height)));
    let width = px(WIDTH.min(f32::from(visible.size.width)));
    match edge {
        Edge::Top => Bounds::new(visible.origin, size(visible.size.width, height)),
        Edge::Bottom => Bounds::new(
            point(
                visible.origin.x,
                visible.origin.y + visible.size.height - height,
            ),
            size(visible.size.width, height),
        ),
        Edge::Left => Bounds::new(visible.origin, size(width, visible.size.height)),
        Edge::Right => Bounds::new(
            point(
                visible.origin.x + visible.size.width - width,
                visible.origin.y,
            ),
            size(width, visible.size.height),
        ),
    }
}

/// How a strip is cut into bands: one across a wide strip; down a tall
/// one, as many as fit, each a hand tall, the place running on from the
/// end of one band to the start of the next like lines of writing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rows {
    pub count: usize,
    pub w: f32,
    pub h: f32,
}

impl Rows {
    pub fn of(width: f32, height: f32) -> Self {
        if height <= width {
            return Self {
                count: 1,
                w: width,
                h: height,
            };
        }
        let count = ((height / HEIGHT).floor() as usize).max(1);
        Self {
            count,
            w: width,
            h: height / count as f32,
        }
    }

    /// How long the place is, all its bands laid end to end.
    pub fn length(&self) -> f32 {
        self.w * self.count as f32
    }

    /// The window onto band `row` of `stage`, as a stage of its own width
    /// and a camera over its stretch of the place.
    pub fn view(&self, stage: &Stage, row: usize) -> (Stage, Camera) {
        let start = stage.width / 2.0 - self.length() / 2.0;
        let mut band = stage.clone();
        band.view_w = self.w;
        let camera = Camera {
            zoom: 1.0,
            x: start + self.w * (row as f32 + 0.5),
            y: stage.height / 2.0,
            fold: 0.0,
        };
        (band, camera)
    }
}

/// Whether letting go of the envelope at `to`, having taken it at `from`,
/// opens the letter.
pub fn drag_opens(from: (f32, f32), to: (f32, f32)) -> bool {
    (to.0 - from.0).hypot(to.1 - from.1) >= DRAG_OPENS
}

/// What someone on a strip says when clicked: the last thing their World
/// shows them saying, or else what they answer when asked, or else a
/// greeting. Read from the snapshot only.
pub fn line_for(snapshot: &ProjectionSnapshot, who: SelectionId) -> String {
    snapshot
        .voices
        .iter()
        .rev()
        .find(|voice| voice.speaker == who)
        .map(|voice| voice.line.clone())
        .or_else(|| {
            snapshot
                .talks
                .iter()
                .find(|talk| talk.who == who)
                .map(|talk| talk.answer.clone())
        })
        .filter(|line| !line.trim().is_empty())
        .unwrap_or_else(|| ui::t("Hello!").to_string())
}

/// How long to wait before the next frame: a frame's time while something
/// moves and the strip can be seen, and no frame at all while nothing
/// moves or nobody could see it drawn.
pub fn next_frame_delay(moving: bool, visible: bool) -> Option<Duration> {
    (moving && visible).then_some(FRAME)
}

/// When the strip should next wake, if at all: the next frame while
/// something moves; otherwise when something next starts to move
/// (`next_motion` seconds from now), or the next look at the light,
/// whichever is sooner. While the strip cannot be seen it does not wake
/// at all: it draws once, at once, when it can be seen again.
pub fn wake_after(moving: bool, visible: bool, next_motion: Option<f32>) -> Option<Duration> {
    if !visible {
        return None;
    }
    Some(next_frame_delay(moving, visible).unwrap_or_else(|| {
        next_motion
            .map(|seconds| Duration::from_secs_f32(seconds.max(0.0)))
            .map_or(LIGHT_CHECK, |motion| motion.min(LIGHT_CHECK))
    }))
}

/// A number from 0 to 1 that only `seed` and `turn` decide.
fn chance(seed: u32, turn: u64) -> f32 {
    let mut x = (u64::from(seed) << 32) ^ turn.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^= x >> 33;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// Who walks where on a strip: one resident at a time, in turn, sets out
/// from where they stand to somewhere else along the strip, and stays
/// there until their next turn. Nobody walks at night.
#[derive(Clone, Debug, PartialEq)]
pub struct Outings {
    homes: Vec<f32>,
    seeds: Vec<u32>,
    left: f32,
    right: f32,
    reach: f32,
    speed: f32,
}

/// Where someone is on the strip at a moment, and whether they are walking.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Whereabouts {
    pub x: f32,
    /// Which way they face: -1 left, 1 right.
    pub facing: f32,
    /// How far through their walk they are, while they walk.
    pub walking: Option<f32>,
}

impl Outings {
    pub fn new(stage: &Stage, snapshot: &ProjectionSnapshot) -> Self {
        let items = &snapshot.canvas.items;
        let (homes, seeds) = stage
            .people
            .iter()
            .map(|spot| {
                let seed = items
                    .get(spot.index)
                    .map(|item| art::seed_of(&item.id.stable_key()))
                    .unwrap_or(spot.index as u32);
                (spot.x, seed)
            })
            .unzip();
        let margin = stage.width * 0.03 + stage.figure_h;
        Self {
            homes,
            seeds,
            left: margin,
            right: (stage.width - margin).max(margin),
            reach: (stage.width * 0.3).min(stage.figure_h * 30.0).max(1.0),
            speed: (stage.figure_h * 1.6).max(24.0),
        }
    }

    pub fn people(&self) -> usize {
        self.homes.len()
    }

    /// Where `person` stands after their `turn`th walk (-1: before any).
    fn spot(&self, person: usize, turn: i64) -> f32 {
        let home = self.homes[person];
        if turn < 0 {
            return home.clamp(self.left, self.right);
        }
        let offset = chance(self.seeds[person], turn as u64) * 2.0 - 1.0;
        (home + offset * self.reach).clamp(self.left, self.right)
    }

    /// How long a walk of `distance` takes.
    fn walk_seconds(&self, distance: f32) -> f32 {
        (distance / self.speed).clamp(1.0, LONGEST_WALK)
    }

    /// Which outing is under way `seconds` in, if any has begun.
    fn outing(seconds: f32) -> Option<i64> {
        (seconds >= LEAD_SECONDS).then(|| ((seconds - LEAD_SECONDS) / OUTING_SECONDS) as i64)
    }

    /// Where `person` is `seconds` in. `still` (Reduce Motion) has them
    /// arrive without walking.
    pub fn at(&self, person: usize, seconds: f32, night: bool, still: bool) -> Whereabouts {
        let count = self.people() as i64;
        let first = Whereabouts {
            x: self.spot(person, -1),
            facing: if self.seeds[person].is_multiple_of(2) {
                1.0
            } else {
                -1.0
            },
            walking: None,
        };
        if night || count == 0 {
            return first;
        }
        let Some(outing) = Self::outing(seconds) else {
            return first;
        };
        // The latest outing that was this person's.
        let mine = outing - (outing - person as i64).rem_euclid(count);
        if mine < 0 {
            return first;
        }
        let turn = mine / count;
        let start = LEAD_SECONDS + mine as f32 * OUTING_SECONDS;
        let from = self.spot(person, turn - 1);
        let to = self.spot(person, turn);
        let facing = if (to - from).abs() < 0.5 {
            first.facing
        } else {
            (to - from).signum()
        };
        let progress = (seconds - start) / self.walk_seconds((to - from).abs());
        if still || progress >= 1.0 || (to - from).abs() < 0.5 {
            return Whereabouts {
                x: to,
                facing,
                walking: None,
            };
        }
        Whereabouts {
            x: from + (to - from) * diorama::ease(progress),
            facing,
            walking: Some(progress.max(0.0)),
        }
    }

    /// Whether anybody is walking `seconds` in.
    pub fn moving(&self, seconds: f32, night: bool, still: bool) -> bool {
        (0..self.people()).any(|person| self.at(person, seconds, night, still).walking.is_some())
    }

    /// How many seconds from `seconds` until the next walk sets out, or
    /// `None` if nobody will (nobody lives here, or it is night).
    pub fn next_start(&self, seconds: f32, night: bool) -> Option<f32> {
        if night || self.people() == 0 {
            return None;
        }
        let next = Self::outing(seconds).map_or(0, |outing| outing + 1);
        Some(LEAD_SECONDS + next as f32 * OUTING_SECONDS - seconds)
    }

    /// Everyone as the diorama draws them `seconds` in.
    pub fn living(&self, seconds: f32, night: bool, still: bool) -> Vec<Living> {
        (0..self.people())
            .map(|person| {
                let here = self.at(person, seconds, night, still);
                Living {
                    x: here.x,
                    pose: Pose {
                        stride: here.walking.map(|_| (seconds * 1.8).fract()),
                        bob: 0.0,
                        facing: here.facing,
                        ..Pose::default()
                    },
                    stance: None,
                }
            })
            .collect()
    }
}

/// How far above its resting place a letter that arrived `since` seconds
/// ago still is, as a share of the drop (1 just arrived, 0 at rest), or
/// `None` once it has landed. It falls on [`LETTER_SPRING`], GPUI's
/// damped spring, and bounces once off the ground as it lands.
pub fn letter_drop(since: f32) -> Option<f32> {
    let start = SpringState {
        position: 1.0,
        velocity: 0.0,
    };
    let lands = LETTER_SPRING
        .settle_time(start, 0.0, LETTER_REST)
        .as_secs_f32()
        .min(LETTER_SECONDS);
    (since < lands).then(|| {
        LETTER_SPRING
            .step(start, 0.0, since.max(0.0))
            .position
            .abs()
    })
}

/// Lays a World out along a strip `width` by `height`: the diorama's
/// stage, with its people drawn a little larger so they can be seen
/// walking in so short a band.
pub fn stage(snapshot: &ProjectionSnapshot, width: f32, height: f32) -> Stage {
    let mut stage = diorama::stage(snapshot, width, height);
    stage.figure_h = stage.figure_h.max(height * 0.24);
    stage
}

/// Works out one frame of a strip `seconds` in: the landscape standing
/// still, and everybody where their outings have them.
pub fn frame(
    snapshot: &ProjectionSnapshot,
    stage: &Stage,
    outings: &Outings,
    seconds: f32,
    daylight: Daylight,
    still: bool,
) -> diorama::Frame {
    let rows = Rows {
        count: 1,
        w: stage.view_w,
        h: stage.height,
    };
    frames(
        snapshot,
        stage,
        outings,
        rows,
        seconds,
        daylight,
        still,
        &BTreeMap::new(),
    )
    .remove(0)
}

/// Works out one frame of each of a strip's bands `seconds` in, with
/// everyone clicked on (`poked`: how many seconds ago) waving.
#[allow(clippy::too_many_arguments)]
pub fn frames(
    snapshot: &ProjectionSnapshot,
    stage: &Stage,
    outings: &Outings,
    rows: Rows,
    seconds: f32,
    daylight: Daylight,
    still: bool,
    poked: &BTreeMap<SelectionId, f32>,
) -> Vec<diorama::Frame> {
    let mut lives = outings.living(seconds, daylight == Daylight::Night, still);
    diorama::wave(&mut lives, stage, snapshot, poked, still);
    // The sky, the water and the trees are drawn at one moment always, so
    // a strip at rest is a picture and draws nothing.
    (0..rows.count.max(1))
        .map(|row| {
            let (band, camera) = rows.view(stage, row);
            diorama::frame(
                snapshot,
                &band,
                &lives,
                camera,
                0.0,
                daylight,
                &Glows::new(),
                1.0,
            )
        })
        .collect()
}

/// Whether anyone clicked on is still waving, `poked` seconds ago each.
pub fn waving(poked: &BTreeMap<SelectionId, f32>, still: bool) -> bool {
    !still
        && poked
            .values()
            .any(|ago| (0.0..diorama::WAVE_SECONDS).contains(ago))
}

/// Seconds until the next change a still strip must draw: the end of a
/// wave or of a line being said, or the next walk.
pub fn next_change(next_walk: Option<f32>, poked: &BTreeMap<SelectionId, f32>) -> Option<f32> {
    poked
        .values()
        .flat_map(|ago| [diorama::WAVE_SECONDS - ago, SAYING_SECONDS - ago])
        .filter(|left| *left > 0.0)
        .chain(next_walk)
        .reduce(f32::min)
}

/// Who, if anyone, is drawn under the point `at` in a band's frame: the
/// canvas item index of the person nearest in front.
pub fn person_at(frame: &diorama::Frame, at: (f32, f32)) -> Option<usize> {
    frame
        .people
        .iter()
        .filter(|person| {
            let half = (person.height * 0.35).max(8.0);
            (at.0 - person.x).abs() <= half
                && at.1 <= person.y + 4.0
                && at.1 >= person.y - person.height - 4.0
        })
        .min_by(|a, b| (a.x - at.0).abs().total_cmp(&(b.x - at.0).abs()))
        .map(|person| person.index)
}

type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

/// A World as a strip along the edge of a screen. Double-click opens the
/// full World; the envelope, when there is a letter, opens it to read.
pub struct StripView {
    snapshot: ProjectionSnapshot,
    /// When the strip first drew. Every time the strip reads comes from
    /// GPUI's executor clock, never `Instant::now()`: the executor's clock
    /// is the one its timers (the strip's wake-ups) run on, and in a test
    /// window it is the fake clock `advance_clock` moves. Reading the wall
    /// clock instead left the letter still falling in a release build,
    /// where almost no real time passes between frames.
    started: Option<Instant>,
    /// The strip's clock at the last thing it saw: a frame or a click.
    now: Option<Instant>,
    /// How many letters the strip has seen arrive.
    letters_seen: usize,
    /// When the latest letter arrived in the strip.
    letter_at: Option<Instant>,
    reading: bool,
    /// The one wake-up waiting, if any: dropping it cancels it.
    wake: Option<Task<()>>,
    /// Hears when the strip is covered or shown again.
    visibility: Option<Subscription>,
    on_open: Option<Handler>,
    on_close: Option<Handler>,
    on_choose: Option<Chooser>,
    drawn: u64,
    /// Who was clicked, and when.
    poked: BTreeMap<SelectionId, Instant>,
    /// Who is saying something, what, and since when.
    saying: Option<(SelectionId, String, Instant)>,
    /// The envelope being dragged: where it was taken and where it is.
    dragging: Option<((f32, f32), (f32, f32))>,
    /// The strip's own menu, open at a point.
    menu: Option<(f32, f32)>,
    /// Where the strip lies, and whether it stays in front, for its menu.
    edge: Edge,
    in_front: bool,
    /// The last frames drawn, one a band, and where each band is.
    bands: Vec<(Bounds<Pixels>, diorama::Frame)>,
    /// Where the envelope rests.
    letter_spot: (f32, f32),
}

/// What the strip's own menu offers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StripChoice {
    Edge(Edge),
    InFront,
    NextDisplay,
}

type Chooser = Rc<dyn Fn(StripChoice, &mut Window, &mut App)>;

impl StripView {
    pub fn new(snapshot: ProjectionSnapshot) -> Self {
        Self {
            snapshot,
            started: None,
            now: None,
            letters_seen: 0,
            letter_at: None,
            reading: false,
            wake: None,
            visibility: None,
            on_open: None,
            on_close: None,
            on_choose: None,
            drawn: 0,
            poked: BTreeMap::new(),
            saying: None,
            dragging: None,
            menu: None,
            edge: Edge::default(),
            in_front: false,
            bands: Vec::new(),
            letter_spot: (0.0, 0.0),
        }
    }

    /// What a choice in the strip's own menu (a right-click) does, and
    /// where the strip lies now, for the menu to tick.
    pub fn on_choose(
        mut self,
        edge: Edge,
        in_front: bool,
        choose: impl Fn(StripChoice, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.edge = edge;
        self.in_front = in_front;
        self.on_choose = Some(Rc::new(choose));
        self
    }

    /// Someone on the strip is clicked: they wave and say their line.
    pub fn poke(&mut self, who: SelectionId, now: Instant) {
        self.poked
            .retain(|_, at| now.duration_since(*at).as_secs_f32() < SAYING_SECONDS);
        self.poked.insert(who, now);
        self.saying = Some((who, line_for(&self.snapshot, who), now));
        self.now = Some(self.now.map_or(now, |seen| seen.max(now)));
    }

    /// What is being said on the strip now, by whom.
    pub fn saying(&self) -> Option<(SelectionId, &str)> {
        self.saying
            .as_ref()
            .filter(|(_, _, at)| {
                self.now
                    .map_or(Duration::ZERO, |now| now.duration_since(*at))
                    .as_secs_f32()
                    < SAYING_SECONDS
            })
            .map(|(who, line, _)| (*who, line.as_str()))
    }

    /// Whether the letter is open to read.
    pub fn is_reading(&self) -> bool {
        self.reading
    }

    /// What a double-click does: open the full World.
    pub fn on_open(mut self, open: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(open));
        self
    }

    /// What the close button does.
    pub fn on_close(mut self, close: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(close));
        self
    }

    /// Whether the strip already shows `snapshot`, cheaply, so a caller can
    /// skip handing over the same World again.
    pub fn is_showing(&self, snapshot: &ProjectionSnapshot) -> bool {
        key(&self.snapshot) == key(snapshot)
    }

    /// Shows the World as it is now. A new letter arrives in the strip.
    pub fn set_snapshot(&mut self, snapshot: ProjectionSnapshot, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }

    /// The World as the strip shows it.
    pub fn snapshot(&self) -> &ProjectionSnapshot {
        &self.snapshot
    }

    /// How many times the strip has drawn since it opened.
    pub fn frames_drawn(&self) -> u64 {
        self.drawn
    }

    /// Whether a wake-up is waiting: false while the strip sleeps.
    pub fn is_waiting(&self) -> bool {
        self.wake.is_some()
    }

    fn schedule(&mut self, delay: Duration, window: &mut Window, cx: &mut Context<Self>) {
        self.wake = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update_in(cx, |_, window, cx| {
                if window.is_visible() {
                    cx.notify();
                }
            });
        }));
    }

    /// Listens, once, for the strip being covered or shown: covered, the
    /// waiting wake-up is dropped; shown again, it draws at once, and that
    /// frame picks the pace up from there.
    fn watch_visibility(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.visibility.is_some() {
            return;
        }
        self.visibility = Some(
            cx.observe_window_visibility(window, |this, visibility, _, cx| {
                if visibility.is_visible() {
                    cx.notify();
                } else {
                    this.wake = None;
                }
            }),
        );
    }

    fn render_letter(
        &self,
        drop: f32,
        rest: (f32, f32),
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let (x, y) = match self.dragging {
            // Dragged, it follows the pointer from where it was taken.
            Some((from, to)) => (rest.0 + to.0 - from.0, rest.1 + to.1 - from.1),
            None => (rest.0, rest.1 - drop * (rest.1 + LETTER_H + 4.0)),
        };
        letter_button(letter_from(&self.snapshot), self.reading, x, y)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                    let at = (f32::from(event.position.x), f32::from(event.position.y));
                    this.dragging = Some((at, at));
                    cx.stop_propagation();
                }),
            )
            .on_click(cx.listener(|this, event: &gpui::ClickEvent, _, cx| {
                cx.stop_propagation();
                // Let go of where it was dragged to, far enough, it opens;
                // clicked, it opens or is put away.
                let to = event.position();
                match this.dragging.take() {
                    Some((from, _)) if drag_opens(from, (f32::from(to.x), f32::from(to.y))) => {
                        this.reading = true
                    }
                    _ => this.reading = !this.reading,
                }
                cx.notify();
            }))
    }

    fn render_reading(&self, width: f32, height: f32) -> Option<gpui::Stateful<gpui::Div>> {
        let letter = self.snapshot.letters.last()?;
        let from = letter_from(&self.snapshot);
        let card = div()
            .id("strip-reading")
            .role(Role::Article)
            .aria_label(from.clone())
            .absolute();
        // Down a side the letter opens across the column; along the top or
        // bottom, beside the envelope.
        let card = if height > width {
            card.left(px(8.0))
                .right(px(8.0))
                .top(px(34.0))
                .h(px((height - 48.0).clamp(40.0, 360.0)))
        } else {
            let card_w = (width - 2.0 * LETTER_W - 48.0).clamp(160.0, 520.0);
            card.right(px(LETTER_W + 40.0))
                .top(px(10.0))
                .w(px(card_w))
                .h(px((height - 20.0).max(40.0)))
        };
        Some(
            card.p_3()
                .flex()
                .flex_col()
                .gap_1()
                .rounded_lg()
                .shadow_md()
                .bg(ui::color(tokens::SURFACE))
                .text_color(ui::color(tokens::TEXT))
                .overflow_y_scroll()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ui::color(tokens::TEXT_SECONDARY))
                        .child(from),
                )
                .child(ui::body(letter.note.clone()))
                .on_click(|_, _, cx| cx.stop_propagation()),
        )
    }

    /// What someone clicked on says, in a bubble over them.
    fn render_saying(&self, width: f32) -> Option<gpui::Div> {
        let (who, line) = self.saying()?;
        let (band, frame) = self.bands.iter().find(|(band, frame)| {
            frame.people.iter().any(|person| {
                self.snapshot
                    .canvas
                    .items
                    .get(person.index)
                    .map(|item| item.id)
                    == Some(who)
                    && person.x >= 0.0
                    && person.x <= f32::from(band.size.width)
            })
        })?;
        let person = frame.people.iter().find(|person| {
            self.snapshot
                .canvas
                .items
                .get(person.index)
                .map(|item| item.id)
                == Some(who)
        })?;
        let bubble_w = (width - 16.0).min(260.0);
        let x = (f32::from(band.origin.x) + person.x - bubble_w / 2.0)
            .clamp(8.0, width - bubble_w - 8.0);
        let top = f32::from(band.origin.y) + (person.y - person.height - 44.0).max(4.0);
        Some(
            div()
                .absolute()
                .left(px(x))
                .top(px(top))
                .w(px(bubble_w))
                .px_2()
                .py_1()
                .rounded_lg()
                .shadow_sm()
                .bg(ui::color(tokens::SURFACE))
                .text_color(ui::color(tokens::TEXT))
                .text_xs()
                .child(crate::wrap::text(line.to_string())),
        )
    }

    /// The strip's own menu: where it lies, whether it stays in front,
    /// and which display it is on.
    fn render_menu(
        &self,
        at: (f32, f32),
        width: f32,
        height: f32,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let menu_w = 176.0;
        let item = |id: &'static str, label: SharedString, ticked: bool, choice: StripChoice| {
            div()
                .id(id)
                .role(Role::MenuItem)
                .aria_label(label.clone())
                .px_2()
                .py(px(3.0))
                .rounded_md()
                .flex()
                .gap_1()
                .cursor_pointer()
                .hover(|style| style.bg(ui::color(tokens::ROW_HOVER)))
                .child(div().w(px(12.0)).child(if ticked { "✓" } else { "" }))
                .child(label)
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.menu = None;
                    cx.notify();
                    if let Some(choose) = this.on_choose.clone() {
                        choose(choice, window, cx);
                    }
                }))
        };
        let mut menu = div()
            .id("strip-menu")
            .role(Role::Menu)
            .aria_label(ui::t("Strip"))
            .absolute()
            .left(px(at.0.clamp(4.0, (width - menu_w - 4.0).max(4.0))))
            .top(px(at.1.clamp(4.0, (height - 150.0).max(4.0))))
            .w(px(menu_w))
            .p_1()
            .flex()
            .flex_col()
            .rounded_lg()
            .shadow_md()
            .bg(ui::color(tokens::SURFACE))
            .text_color(ui::color(tokens::TEXT))
            .text_xs()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.menu = None;
                cx.notify();
            }));
        for (id, edge) in [
            ("strip-top", Edge::Top),
            ("strip-bottom", Edge::Bottom),
            ("strip-left", Edge::Left),
            ("strip-right", Edge::Right),
        ] {
            menu = menu.child(item(
                id,
                ui::t(edge.label()),
                self.edge == edge,
                StripChoice::Edge(edge),
            ));
        }
        menu.child(item(
            "strip-in-front",
            ui::t("Always in Front"),
            self.in_front,
            StripChoice::InFront,
        ))
        .child(item(
            "strip-next-display",
            ui::t("Move to Next Display"),
            false,
            StripChoice::NextDisplay,
        ))
    }
}

const LETTER_W: f32 = 34.0;
const LETTER_H: f32 = 24.0;

/// Who the latest letter is from, as its card heads it.
fn letter_from(snapshot: &ProjectionSnapshot) -> SharedString {
    snapshot
        .letters
        .last()
        .and_then(|letter| {
            snapshot
                .canvas
                .items
                .iter()
                .find(|item| item.id == letter.from)
        })
        .and_then(|item| item.label.split_whitespace().next().map(str::to_string))
        .map(|name| ui::t(format!("From {name}")))
        .unwrap_or_else(|| ui::t("From a friend"))
}

/// The envelope at `x`, `y`: a button that opens the letter to read, or
/// puts it away again, and says which to a screen reader.
pub(crate) fn letter_button(
    from: SharedString,
    reading: bool,
    x: f32,
    y: f32,
) -> gpui::Stateful<gpui::Div> {
    let label = if reading {
        ui::t("Put the letter away")
    } else {
        ui::t(format!("A letter. {from}. Open it to read"))
    };
    ui::named(div().id("strip-letter"), label)
        .role(Role::Button)
        .aria_expanded(reading)
        .absolute()
        .left(px(x))
        .top(px(y))
        .w(px(LETTER_W))
        .h(px(LETTER_H))
        .cursor_pointer()
        .child(
            canvas(
                |_, _, _| (),
                |bounds, _, window, _| paint_envelope(bounds, window),
            )
            .size_full(),
        )
}

/// The small round button that puts the strip away.
pub(crate) fn close_button(ink: Hsla) -> gpui::Stateful<gpui::Div> {
    ui::named(div().id("strip-close"), "Close the strip")
        .role(Role::Button)
        .absolute()
        .top(px(6.0))
        .right(px(8.0))
        .size(px(18.0))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(ink)
        .bg(Hsla {
            a: 0.55,
            ..gpui::white()
        })
        .cursor_pointer()
        .child("×")
}

/// What changes between one snapshot of a World and the next, cheaply.
fn key(snapshot: &ProjectionSnapshot) -> (String, usize, usize, usize, usize, String) {
    (
        snapshot.title.clone(),
        snapshot.canvas.items.len(),
        snapshot.timeline.items.len(),
        snapshot.letters.len(),
        snapshot.canvas.marks.len(),
        format!("{:?}", snapshot.world_time),
    )
}

/// A small envelope: a cream card with its flap folded down.
fn paint_envelope(bounds: Bounds<Pixels>, window: &mut Window) {
    let x = f32::from(bounds.origin.x);
    let y = f32::from(bounds.origin.y);
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    let shadow = Hsla {
        a: 0.25,
        ..art::hex(0x2a2018)
    };
    art::rect(window, x + 1.5, y + 2.0, w, h, 2.5, shadow);
    art::rect(window, x, y, w, h, 2.5, art::hex(0xfbf4e4));
    art::polygon(
        window,
        &[
            (x + 1.0, y + 1.0),
            (x + w - 1.0, y + 1.0),
            (x + w / 2.0, y + h * 0.58),
        ],
        art::hex(0xeadcc0),
    );
    art::line(
        window,
        (x + 1.0, y + 1.0),
        (x + w / 2.0, y + h * 0.58),
        1.0,
        art::hex(0xb9a27c),
    );
    art::line(
        window,
        (x + w - 1.0, y + 1.0),
        (x + w / 2.0, y + h * 0.58),
        1.0,
        art::hex(0xb9a27c),
    );
    art::circle(window, x + w / 2.0, y + h * 0.56, 3.0, art::hex(0xc0463a));
}

impl Render for StripView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.drawn += 1;
        let now = cx.background_executor().now();
        self.now = Some(now);
        let seconds = now
            .duration_since(*self.started.get_or_insert(now))
            .as_secs_f32();
        // The day's letter arrives in the strip when it opens and whenever
        // another comes.
        if self.snapshot.letters.len() > self.letters_seen {
            self.letters_seen = self.snapshot.letters.len();
            self.letter_at = Some(now);
        }
        if self.snapshot.letters.is_empty() {
            self.reading = false;
        }
        let viewport = window.viewport_size();
        let (width, height) = (f32::from(viewport.width), f32::from(viewport.height));
        let rows = Rows::of(width, height);
        let stage = stage(&self.snapshot, rows.length(), rows.h);
        let outings = Outings::new(&stage, &self.snapshot);
        let daylight = scene::daylight_now();
        let night = daylight == Daylight::Night;
        let still = cx.reduce_motion();
        self.poked
            .retain(|_, at| now.duration_since(*at).as_secs_f32() < SAYING_SECONDS);
        let poked = self
            .poked
            .iter()
            .map(|(who, at)| (*who, now.duration_since(*at).as_secs_f32()))
            .collect::<BTreeMap<_, _>>();
        let frames = frames(
            &self.snapshot,
            &stage,
            &outings,
            rows,
            seconds,
            daylight,
            still,
            &poked,
        );
        self.bands = frames
            .into_iter()
            .enumerate()
            .map(|(row, frame)| {
                let origin = point(px(0.0), px(rows.h * row as f32));
                (Bounds::new(origin, size(px(rows.w), px(rows.h))), frame)
            })
            .collect();
        let drop = self
            .letter_at
            .filter(|_| !still)
            .and_then(|at| letter_drop(now.duration_since(at).as_secs_f32()));
        let moving =
            outings.moving(seconds, night, still) || drop.is_some() || waving(&poked, still);
        self.watch_visibility(window, cx);
        match wake_after(
            moving,
            window.is_visible(),
            next_change(outings.next_start(seconds, night), &poked),
        ) {
            Some(wake) => self.schedule(wake, window, cx),
            None => self.wake = None,
        }

        // The envelope rests at the end of the last band, at its feet.
        let last_row = rows.count.saturating_sub(1) as f32;
        let letter_rest = (
            width - LETTER_W - 22.0,
            rows.h * last_row + (stage.feet - LETTER_H - 2.0).max(4.0),
        );
        let has_letter = !self.snapshot.letters.is_empty();
        self.letter_spot = letter_rest;
        let ink: Hsla = ui::color(tokens::TEXT).into();
        let bands = self
            .bands
            .iter()
            .map(|(bounds, frame)| {
                let frame = frame.clone();
                div()
                    .absolute()
                    .left(bounds.origin.x)
                    .top(bounds.origin.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
                    .overflow_hidden()
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, _, window, _| diorama::paint(&frame, bounds, window),
                        )
                        .size_full(),
                    )
            })
            .collect::<Vec<_>>();

        div()
            .id("strip")
            .role(Role::Region)
            .aria_label(self.snapshot.title.clone())
            .aria_description(ui::t(
                "Click someone to say hello. Double-click to open the World",
            ))
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(art::hex(
                self.snapshot.scenery.map_or(0xb9d6ea, |s| s.sky_top),
            ))
            .on_click(cx.listener(|this, event: &gpui::ClickEvent, window, cx| {
                if event.click_count() >= 2 {
                    if let Some(open) = this.on_open.clone() {
                        open(window, cx);
                    }
                    return;
                }
                if this.menu.take().is_some() || std::mem::take(&mut this.reading) {
                    cx.notify();
                    return;
                }
                let at = event.position();
                let at = (f32::from(at.x), f32::from(at.y));
                let hit = this.bands.iter().find_map(|(band, frame)| {
                    band.contains(&point(px(at.0), px(at.1))).then(|| {
                        let local = (
                            at.0 - f32::from(band.origin.x),
                            at.1 - f32::from(band.origin.y),
                        );
                        person_at(frame, local)
                    })?
                });
                if let Some(who) = hit.and_then(|index| this.snapshot.canvas.items.get(index)) {
                    let who = who.id;
                    this.poke(who, cx.background_executor().now());
                    cx.notify();
                }
            }))
            .on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                    if this.on_choose.is_some() {
                        this.menu =
                            Some((f32::from(event.position.x), f32::from(event.position.y)));
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if let Some((from, _)) = this.dragging {
                    if event.pressed_button != Some(gpui::MouseButton::Left) {
                        this.dragging = None;
                    } else {
                        this.dragging = Some((
                            from,
                            (f32::from(event.position.x), f32::from(event.position.y)),
                        ));
                    }
                    cx.notify();
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseUpEvent, _, cx| {
                    if let Some((from, _)) = this.dragging.take() {
                        let to = (f32::from(event.position.x), f32::from(event.position.y));
                        if drag_opens(from, to) {
                            this.reading = true;
                        }
                        cx.notify();
                    }
                }),
            )
            .children(bands)
            .children(self.render_saying(width))
            .when(has_letter, |strip| {
                strip.child(self.render_letter(drop.unwrap_or(0.0), letter_rest, cx))
            })
            .when(self.reading, |strip| {
                strip.children(self.render_reading(width, height))
            })
            .child(
                close_button(ink).on_click(cx.listener(|this, _, window, cx| {
                    cx.stop_propagation();
                    if let Some(close) = this.on_close.clone() {
                        close(window, cx);
                    }
                })),
            )
            .when_some(self.menu, |strip, at| {
                strip.child(self.render_menu(at, width, height, cx))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_projection::{CanvasItem, CanvasItemKind, CanvasProjection, Letter, SelectionId};

    fn entity(id: u64) -> SelectionId {
        SelectionId::from_stable_key(&format!("entity-{id}")).expect("an entity key")
    }

    fn item(id: u64, kind: CanvasItemKind, x: f32, at: Option<u64>) -> CanvasItem {
        CanvasItem {
            id: entity(id),
            kind,
            label: format!("Item {id}"),
            detail: String::new(),
            x,
            y: 0.5,
            changes: Vec::new(),
            shape: None,
            at: at.map(entity),
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

    /// A town of `people` and a handful of places.
    fn town(people: u64) -> ProjectionSnapshot {
        let mut items = (0..5)
            .map(|id| item(100 + id, CanvasItemKind::Place, id as f32 / 5.0, None))
            .collect::<Vec<_>>();
        for id in 0..people {
            items.push(item(
                id + 1,
                CanvasItemKind::Actor,
                id as f32 / people as f32,
                Some(100 + id % 5),
            ));
        }
        ProjectionSnapshot {
            canvas: CanvasProjection {
                items,
                links: Vec::new(),
                marks: Vec::new(),
                ..Default::default()
            },
            letters: vec![Letter {
                from: entity(1),
                note: "The pier is mended.".into(),
                moment: entity(1),
            }],
            ..ProjectionSnapshot::default()
        }
    }

    fn outings(people: u64, width: f32) -> (ProjectionSnapshot, Stage, Outings) {
        let snapshot = town(people);
        let stage = stage(&snapshot, width, HEIGHT);
        let outings = Outings::new(&stage, &snapshot);
        (snapshot, stage, outings)
    }

    /// Plays a strip for `seconds` the way the window does: after each
    /// frame it sleeps as long as [`wake_after`] says, and it draws when it
    /// wakes. Returns when each frame was drawn and whether anything moved
    /// then.
    fn play(outings: &Outings, seconds: f32, letter: bool) -> Vec<(f32, bool)> {
        let mut drawn = Vec::new();
        let mut now = 0.0_f32;
        while now < seconds {
            let drop = letter.then(|| letter_drop(now)).flatten();
            let moving = outings.moving(now, false, false) || drop.is_some();
            drawn.push((now, moving));
            now += wake_after(moving, true, outings.next_start(now, false))
                .expect("a strip in view always wakes again")
                .as_secs_f32();
        }
        drawn
    }

    #[test]
    fn pacing_is_no_frames_when_still_and_under_fifteen_a_second_when_not() {
        assert_eq!(next_frame_delay(false, true), None);
        let delay = next_frame_delay(true, true).expect("a frame while moving");
        assert!(delay >= Duration::from_secs_f64(1.0 / 15.0));
        assert!(1.0 / delay.as_secs_f64() < 15.0);
        // Still, it sleeps until the next walk, or the light check.
        let wake = |moving, next| wake_after(moving, true, next);
        assert_eq!(wake(false, Some(7.5)), Some(Duration::from_secs_f32(7.5)));
        assert_eq!(wake(false, None), Some(LIGHT_CHECK));
        assert_eq!(wake(false, Some(1e6)), Some(LIGHT_CHECK));
        assert_eq!(wake(true, Some(0.01)), Some(FRAME));
    }

    /// Covered, minimised or on a sleeping display, a strip asks for no
    /// frame and no wake-up at all, whatever is moving in it and however
    /// soon the next walk starts.
    #[test]
    fn a_strip_nobody_can_see_neither_draws_nor_wakes() {
        for moving in [false, true] {
            assert_eq!(next_frame_delay(moving, false), None);
            for next in [None, Some(0.0), Some(2.0), Some(1e6)] {
                assert_eq!(wake_after(moving, false, next), None);
            }
        }
    }

    /// The v0.18 bar, in the style of the window's busy-frame benchmark:
    /// over ten minutes of a town of eight on a wide screen, no second ever
    /// draws fifteen frames, and every stretch where nothing moves draws
    /// nothing but the frame that starts the next walk.
    #[test]
    fn a_strip_stays_under_fifteen_frames_a_second_and_still_when_nothing_moves() {
        let (_, _, outings) = outings(8, 2560.0);
        let minutes = 10.0;
        let drawn = play(&outings, minutes * 60.0, true);
        // The frame budget: any one-second window holds fewer than 15.
        for (index, (at, _)) in drawn.iter().enumerate() {
            let in_second = drawn[index..]
                .iter()
                .take_while(|(later, _)| *later < at + 1.0)
                .count();
            assert!(in_second < 15, "{in_second} frames in the second from {at}");
        }
        // Idle stillness: a frame where nothing moves is only ever the one
        // that wakes for the next walk, and nothing is drawn in between.
        for pair in drawn.windows(2) {
            let ((at, moving), (next, _)) = (pair[0], pair[1]);
            if !moving {
                let starts = outings.next_start(at, false).unwrap();
                assert!((next - at - starts).abs() < 0.01, "woke early at {at}");
            }
        }
        // And it is still most of the time: far fewer frames than a strip
        // drawn at its budget throughout.
        let budget = (minutes * 60.0) as usize * MOST_FRAMES_A_SECOND as usize;
        assert!(drawn.len() < budget * 7 / 10, "{} of {budget}", drawn.len());
        let still = (0..(minutes * 600.0) as u32)
            .filter(|tenth| !outings.moving(*tenth as f32 / 10.0, false, false))
            .count();
        assert!(still as f32 > minutes * 600.0 * 0.3, "still {still} tenths");
    }

    /// At night, with Reduce Motion, or with nobody in the World, a strip
    /// draws once and then only looks up at the light.
    #[test]
    fn a_strip_with_nothing_to_move_draws_nothing() {
        let (_, _, busy) = outings(6, 1800.0);
        for tenth in 0..6000 {
            let seconds = tenth as f32 / 10.0;
            assert!(!busy.moving(seconds, true, false));
            assert!(!busy.moving(seconds, false, true));
        }
        assert_eq!(
            wake_after(false, true, busy.next_start(100.0, true)),
            Some(LIGHT_CHECK)
        );
        let (_, _, empty) = outings(0, 1800.0);
        assert!(!empty.moving(50.0, false, false));
        assert_eq!(empty.next_start(50.0, false), None);
        // A landed letter moves nothing.
        assert_eq!(letter_drop(LETTER_SECONDS + 0.1), None);
        assert!(letter_drop(0.0).is_some_and(|drop| (drop - 1.0).abs() < 1e-6));
    }

    /// Residents walk the strip's length between them, and walk rather
    /// than jump: from one frame to the next nobody moves far.
    #[test]
    fn residents_walk_along_the_strip_and_never_jump() {
        let (_, stage, outings) = outings(6, 2000.0);
        let mut seen = vec![(f32::MAX, f32::MIN); outings.people()];
        let mut last = (0..outings.people())
            .map(|person| outings.at(person, 0.0, false, false).x)
            .collect::<Vec<_>>();
        let step = FRAME.as_secs_f32();
        let frames = (1800.0 / step) as u32;
        for frame in 1..frames {
            let seconds = frame as f32 * step;
            for (person, was) in last.iter_mut().enumerate() {
                let here = outings.at(person, seconds, false, false);
                assert!(here.x >= 0.0 && here.x <= stage.width);
                assert!(
                    (here.x - *was).abs() < stage.figure_h,
                    "{person} jumped at {seconds}"
                );
                *was = here.x;
                seen[person].0 = seen[person].0.min(here.x);
                seen[person].1 = seen[person].1.max(here.x);
            }
        }
        // Everyone went somewhere, and between them they covered most of it.
        assert!(seen.iter().all(|(low, high)| high - low > stage.figure_h));
        let low = seen.iter().map(|s| s.0).fold(f32::MAX, f32::min);
        let high = seen.iter().map(|s| s.1).fold(f32::MIN, f32::max);
        assert!(high - low > stage.width * 0.6, "{low}..{high}");
    }

    #[test]
    fn a_strip_lies_along_the_chosen_edge_of_the_usable_screen() {
        let visible = Bounds::new(point(px(0.0), px(25.0)), size(px(1512.0), px(870.0)));
        let bottom = band(visible, Edge::Bottom);
        assert_eq!(bottom.size.width, px(1512.0));
        assert_eq!(bottom.size.height, px(HEIGHT));
        assert_eq!(bottom.origin.y + bottom.size.height, px(895.0));
        let top = band(visible, Edge::Top);
        assert_eq!(top.origin.y, px(25.0));
        // A second display to the left keeps its own origin.
        let left = Bounds::new(point(px(-1920.0), px(0.0)), size(px(1920.0), px(1080.0)));
        assert_eq!(band(left, Edge::Bottom).origin.x, px(-1920.0));
        // Down a side: the full usable height, a column wide.
        let side = band(visible, Edge::Left);
        assert_eq!(side.origin, visible.origin);
        assert_eq!(side.size, size(px(WIDTH), px(870.0)));
        let right = band(visible, Edge::Right);
        assert_eq!(right.origin.x + right.size.width, px(1512.0));
        assert_eq!(right.origin.y, px(25.0));
        for edge in Edge::ALL {
            assert_eq!(Edge::from_id(edge.id()), Some(edge));
            assert_eq!(
                edge.vertical(),
                band(visible, edge).size.height > band(visible, edge).size.width
            );
        }
    }

    /// Down a side of the screen, the place runs in bands a hand tall, one
    /// under the next; across the top or bottom it is one band, drawn
    /// exactly as before.
    #[test]
    fn a_tall_strip_runs_the_place_in_bands() {
        let rows = Rows::of(WIDTH, 870.0);
        assert_eq!(rows.count, 6);
        assert!((rows.h - 145.0).abs() < 0.01);
        assert_eq!(rows.length(), WIDTH * 6.0);
        assert_eq!(Rows::of(1512.0, HEIGHT).count, 1);

        let snapshot = town(8);
        let whole = stage(&snapshot, 1512.0, HEIGHT);
        let outings = Outings::new(&whole, &snapshot);
        let one = frames(
            &snapshot,
            &whole,
            &outings,
            Rows::of(1512.0, HEIGHT),
            40.0,
            Daylight::Day,
            false,
            &BTreeMap::new(),
        );
        assert_eq!(one.len(), 1);
        assert_eq!(
            format!("{:?}", one[0]),
            format!(
                "{:?}",
                frame(&snapshot, &whole, &outings, 40.0, Daylight::Day, false)
            )
        );

        let long = stage(&snapshot, rows.length(), rows.h);
        let outings = Outings::new(&long, &snapshot);
        let bands = frames(
            &snapshot,
            &long,
            &outings,
            rows,
            40.0,
            Daylight::Day,
            false,
            &BTreeMap::new(),
        );
        assert_eq!(bands.len(), 6);
        // Everyone is in some band, and in only one.
        let seen = bands
            .iter()
            .flat_map(|frame| {
                frame
                    .people
                    .iter()
                    .filter(|person| person.x >= 0.0 && person.x < rows.w)
                    .map(|person| person.index)
            })
            .collect::<Vec<_>>();
        let mut unique = seen.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), seen.len(), "{seen:?}");
        assert_eq!(unique.len(), 8, "{seen:?}");
    }

    /// Click someone and they wave and say their World's line for them;
    /// the strip reads it from the snapshot and changes nothing.
    #[test]
    fn a_clicked_resident_says_their_worlds_line() {
        let mut snapshot = town(3);
        let mara = entity(1);
        let leo = entity(2);
        snapshot.voices.push(world_projection::Voice {
            moment: mara,
            speaker: mara,
            line: "Bread's out of the oven.".into(),
        });
        assert_eq!(line_for(&snapshot, mara), "Bread's out of the oven.");
        assert_eq!(line_for(&snapshot, leo), "Hello!");

        let whole = stage(&snapshot, 1200.0, HEIGHT);
        let outings = Outings::new(&whole, &snapshot);
        let frame = frame(&snapshot, &whole, &outings, 0.0, Daylight::Day, false);
        let someone = frame
            .people
            .iter()
            .find(|person| person.x > 0.0 && person.x < 1200.0)
            .expect("someone in view");
        let at = (someone.x, someone.y - someone.height * 0.5);
        assert_eq!(person_at(&frame, at), Some(someone.index));
        assert_eq!(
            person_at(&frame, (someone.x, someone.y - someone.height * 3.0)),
            None
        );

        let before = snapshot.clone();
        let mut view = StripView::new(snapshot);
        let now = Instant::now();
        view.poke(mara, now);
        assert_eq!(view.saying(), Some((mara, "Bread's out of the oven.")));
        assert_eq!(view.snapshot(), &before, "nothing about the World changes");
    }

    /// A wave moves for a moment at the strip's pace and then stops; the
    /// line stays until it is done being said, and the strip wakes once to
    /// take it away, not before.
    #[test]
    fn a_wave_keeps_the_pace_and_then_the_strip_is_still() {
        let who = entity(1);
        let mut drawn = Vec::new();
        let mut now = 0.0_f32;
        let poked_at = 0.0_f32;
        while now < 12.0 {
            let poked = [(who, now - poked_at)]
                .into_iter()
                .filter(|(_, ago)| *ago < SAYING_SECONDS)
                .collect::<BTreeMap<_, _>>();
            let moving = waving(&poked, false);
            drawn.push((now, moving));
            let Some(wake) = wake_after(moving, true, next_change(None, &poked)) else {
                break;
            };
            now += wake.as_secs_f32();
        }
        let during = drawn
            .iter()
            .filter(|(at, _)| *at < diorama::WAVE_SECONDS)
            .count();
        assert!(during as f32 <= diorama::WAVE_SECONDS * MOST_FRAMES_A_SECOND as f32 + 1.0);
        assert!(during >= 3, "the wave is seen: {drawn:?}");
        let after = drawn
            .iter()
            .filter(|(at, _)| *at >= diorama::WAVE_SECONDS)
            .map(|(at, _)| *at)
            .collect::<Vec<_>>();
        assert!(
            after.len() <= 3,
            "still once the wave is done, but for taking the line away: {after:?}"
        );
        assert!(
            after.iter().any(|at| (at - SAYING_SECONDS).abs() < 0.05),
            "{after:?}"
        );
        // Reduce Motion: a wave is a raised hand, never motion.
        let poked = BTreeMap::from([(who, 0.1)]);
        assert!(!waving(&poked, true));
    }

    #[test]
    fn dragging_the_envelope_far_enough_opens_it() {
        assert!(!drag_opens((10.0, 10.0), (12.0, 14.0)));
        assert!(drag_opens((10.0, 10.0), (10.0, 10.0 + DRAG_OPENS)));
        assert!(drag_opens((40.0, 40.0), (20.0, 25.0)));
    }

    /// Working out a strip's frame (the stage, everyone's walk, the frame)
    /// for a busy town on a wide screen fits in 4 ms, like the window's.
    /// Timed in a release build; a debug build only checks it finishes.
    #[test]
    fn a_strip_frame_is_worked_out_in_4_ms() {
        let snapshot = town(15);
        let runs = 200;
        let started = Instant::now();
        for run in 0..runs {
            let stage = stage(&snapshot, 3008.0, HEIGHT);
            let outings = Outings::new(&stage, &snapshot);
            let frame = frame(
                &snapshot,
                &stage,
                &outings,
                run as f32 / MOST_FRAMES_A_SECOND as f32,
                Daylight::Day,
                false,
            );
            assert_eq!(frame.people.len(), 15);
        }
        let each = started.elapsed() / runs;
        if !cfg!(debug_assertions) {
            assert!(each < Duration::from_millis(4), "{each:?} a frame");
        }
    }

    /// A new letter falls on GPUI's spring: from the top, down past its
    /// place, one small bounce, and landed well inside [`LETTER_SECONDS`];
    /// never outside the strip, and never jumping between two frames.
    #[test]
    fn a_letter_drops_on_a_spring_and_bounces_once() {
        assert!(letter_drop(0.0).is_some_and(|drop| (drop - 1.0).abs() < 1e-6));
        let step = FRAME.as_secs_f32();
        let mut frames = Vec::new();
        let mut at = 0.0;
        while let Some(drop) = letter_drop(at) {
            assert!((0.0..=1.0).contains(&drop), "{drop} at {at}");
            frames.push(drop);
            at += step;
        }
        assert!(at <= LETTER_SECONDS + step, "landed after {at}s");
        // Down, back up a little, down again: one bounce.
        let turns = frames
            .windows(3)
            .filter(|w| (w[1] - w[0]) * (w[2] - w[1]) < 0.0)
            .count();
        assert!(turns >= 1, "{frames:?}");
        let bounce = frames
            .iter()
            .skip_while(|drop| **drop > 0.05)
            .fold(0.0_f32, |high, drop| high.max(*drop));
        assert!(bounce > 0.02 && bounce < 0.2, "bounce {bounce}");
        for pair in frames.windows(2) {
            assert!((pair[0] - pair[1]).abs() < 0.5, "{frames:?}");
        }
        assert_eq!(letter_drop(LETTER_SECONDS + 0.1), None);
    }

    /// The envelope and the close button are buttons a screen reader can
    /// name; the envelope says whether the letter is open.
    #[test]
    fn the_letter_and_close_button_are_named_buttons() {
        let (role, node) =
            crate::ui::accessible(&letter_button("From Mara".into(), false, 0.0, 0.0));
        assert_eq!(role, Some(Role::Button));
        assert_eq!(node.label(), Some("A letter. From Mara. Open it to read"));
        assert_eq!(node.is_expanded(), Some(false));
        let (_, open) = crate::ui::accessible(&letter_button("From Mara".into(), true, 0.0, 0.0));
        assert_eq!(open.label(), Some("Put the letter away"));
        assert_eq!(open.is_expanded(), Some(true));
        let (role, node) = crate::ui::accessible(&close_button(gpui::black()));
        assert_eq!(role, Some(Role::Button));
        assert_eq!(node.label(), Some("Close the strip"));
        assert_eq!(
            letter_from(&town(3)).as_ref(),
            "From Item",
            "named by the first word of who wrote it"
        );
    }

    /// Through GPUI's own test window: dragging the envelope out opens the
    /// letter, and a click on someone makes them say their line.
    #[gpui::test]
    fn the_envelope_drags_open_and_a_resident_answers_a_click(cx: &mut gpui::TestAppContext) {
        use gpui::{Modifiers, MouseButton, VisualTestContext};
        let window = cx.add_window(|_, _| StripView::new(town(3)));
        let view = window.root(cx).expect("the strip");
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        // Once it has landed.
        cx.executor().advance_clock(Duration::from_secs(3));
        cx.run_until_parked();
        let (x, y) = view.read_with(cx, |strip, _| strip.letter_spot);
        let from = point(px(x + LETTER_W / 2.0), px(y + LETTER_H / 2.0));
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        let to = point(from.x - px(60.0), from.y - px(10.0));
        cx.simulate_mouse_move(to, Some(MouseButton::Left), Modifiers::none());
        cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert!(
            view.read_with(cx, |strip, _| strip.is_reading()),
            "dragged open"
        );

        // Put it away, then click someone.
        let person = view.read_with(cx, |strip, _| {
            strip.bands.iter().find_map(|(band, frame)| {
                frame
                    .people
                    .iter()
                    .find(|person| person.x > 10.0 && person.x < f32::from(band.size.width) - 10.0)
                    .map(|person| {
                        point(
                            band.origin.x + px(person.x),
                            band.origin.y + px(person.y - person.height * 0.5),
                        )
                    })
            })
        });
        let person = person.expect("someone in view");
        cx.executor().advance_clock(Duration::from_secs(2));
        cx.simulate_click(point(px(4.0), px(30.0)), Modifiers::none());
        cx.run_until_parked();
        assert!(
            !view.read_with(cx, |strip, _| strip.is_reading()),
            "a click puts it away"
        );
        cx.executor().advance_clock(Duration::from_secs(2));
        cx.simulate_click(person, Modifiers::none());
        cx.run_until_parked();
        assert!(
            view.read_with(cx, |strip, _| strip.saying().is_some()),
            "they say something"
        );
    }

    /// Through GPUI's own test window: a strip with a letter falling
    /// draws at its pace; covered, it drops its wake-up and draws nothing
    /// however long it stays covered; shown again, it draws once at once
    /// and takes up its pace.
    #[gpui::test]
    fn a_covered_strip_sleeps_and_draws_at_once_when_shown(cx: &mut gpui::TestAppContext) {
        use gpui::{VisualTestContext, WindowVisibility};
        let window = cx.add_window(|_, _| StripView::new(town(3)));
        let view = window.root(cx).expect("the strip");
        let cx = &mut VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        let drawn =
            |cx: &mut VisualTestContext| view.read_with(cx, |strip, _| strip.frames_drawn());
        let waiting =
            |cx: &mut VisualTestContext| view.read_with(cx, |strip, _| strip.is_waiting());
        let first = drawn(cx);
        assert!(first >= 1);
        assert!(waiting(cx), "a strip in view always has its next wake-up");

        cx.simulate_visibility_change(WindowVisibility::Hidden);
        cx.run_until_parked();
        assert!(!waiting(cx), "covered, the wake-up is dropped");
        let covered = drawn(cx);
        cx.executor().advance_clock(LIGHT_CHECK * 3);
        cx.run_until_parked();
        assert_eq!(drawn(cx), covered, "nothing is drawn while covered");
        assert!(!waiting(cx));

        cx.simulate_visibility_change(WindowVisibility::Visible);
        cx.run_until_parked();
        assert_eq!(drawn(cx), covered + 1, "one frame at once when shown");
        assert!(waiting(cx), "and the pace picks up again");
    }
}
