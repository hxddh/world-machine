//! A World as a band along the edge of a screen: its landscape in a strip
//! a hand tall, its people walking the length of it now and then, and the
//! day's letter arriving in it as a small envelope to click and read.
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
use std::rc::Rc;
use std::time::{Duration, Instant};
use world_projection::ProjectionSnapshot;
use world_theme::tokens;

/// How tall a strip is, in points.
pub const HEIGHT: f32 = 140.0;

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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Edge {
    #[default]
    Bottom,
    Top,
}

/// Where a strip goes on a display whose usable area (without the menu bar
/// and the Dock) is `visible`: the full width, [`HEIGHT`] tall, along
/// `edge`.
pub fn band(visible: Bounds<Pixels>, edge: Edge) -> Bounds<Pixels> {
    let height = px(HEIGHT.min(f32::from(visible.size.height)));
    let y = match edge {
        Edge::Top => visible.origin.y,
        Edge::Bottom => visible.origin.y + visible.size.height - height,
    };
    Bounds::new(point(visible.origin.x, y), size(visible.size.width, height))
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
    let lives = outings.living(seconds, daylight == Daylight::Night, still);
    // The sky, the water and the trees are drawn at one moment always, so
    // a strip at rest is a picture and draws nothing.
    diorama::frame(
        snapshot,
        stage,
        &lives,
        Camera::whole(stage),
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    )
}

type Handler = Rc<dyn Fn(&mut Window, &mut App)>;

/// A World as a strip along the edge of a screen. Double-click opens the
/// full World; the envelope, when there is a letter, opens it to read.
pub struct StripView {
    snapshot: ProjectionSnapshot,
    started: Instant,
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
    drawn: u64,
}

impl StripView {
    pub fn new(snapshot: ProjectionSnapshot) -> Self {
        Self {
            snapshot,
            started: Instant::now(),
            letters_seen: 0,
            letter_at: None,
            reading: false,
            wake: None,
            visibility: None,
            on_open: None,
            on_close: None,
            drawn: 0,
        }
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
        let (x, y) = rest;
        let y = y - drop * (y + LETTER_H + 4.0);
        letter_button(letter_from(&self.snapshot), self.reading, x, y).on_click(cx.listener(
            |this, _, _, cx| {
                this.reading = !this.reading;
                cx.stop_propagation();
                cx.notify();
            },
        ))
    }

    fn render_reading(&self, width: f32, height: f32) -> Option<gpui::Stateful<gpui::Div>> {
        let letter = self.snapshot.letters.last()?;
        let from = letter_from(&self.snapshot);
        let card_w = (width - 2.0 * LETTER_W - 48.0).clamp(160.0, 520.0);
        Some(
            div()
                .id("strip-reading")
                .role(Role::Article)
                .aria_label(from.clone())
                .absolute()
                .right(px(LETTER_W + 40.0))
                .top(px(10.0))
                .w(px(card_w))
                .h(px((height - 20.0).max(40.0)))
                .p_3()
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
        let now = Instant::now();
        let seconds = now.duration_since(self.started).as_secs_f32();
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
        let stage = stage(&self.snapshot, width, height);
        let outings = Outings::new(&stage, &self.snapshot);
        let daylight = scene::daylight_now();
        let night = daylight == Daylight::Night;
        let still = cx.reduce_motion();
        let frame = frame(&self.snapshot, &stage, &outings, seconds, daylight, still);
        let drop = self
            .letter_at
            .filter(|_| !still)
            .and_then(|at| letter_drop(now.duration_since(at).as_secs_f32()));
        let moving = outings.moving(seconds, night, still) || drop.is_some();
        self.watch_visibility(window, cx);
        match wake_after(
            moving,
            window.is_visible(),
            outings.next_start(seconds, night),
        ) {
            Some(wake) => self.schedule(wake, window, cx),
            None => self.wake = None,
        }

        let letter_rest = (
            width - LETTER_W - 22.0,
            (stage.feet - LETTER_H - 2.0).max(4.0),
        );
        let has_letter = !self.snapshot.letters.is_empty();
        let ink: Hsla = ui::color(tokens::TEXT).into();

        div()
            .id("strip")
            .role(Role::Region)
            .aria_label(self.snapshot.title.clone())
            .aria_description(ui::t("Double-click to open the World"))
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
                } else if this.reading {
                    this.reading = false;
                    cx.notify();
                }
            }))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| diorama::paint(&frame, bounds, window),
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
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
