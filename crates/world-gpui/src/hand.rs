//! The hand: what makes the still things of a scene look painted by a
//! person rather than printed by a machine. It draws through another
//! brush and, on the way,
//!
//! - lets every outline wander a little off its ruled line (wobble);
//! - draws each fill's edge a shade darker, where paint pools as it dries;
//! - lays a thin ink line around each shape, a touch off its fill, as a
//!   print whose plates did not quite meet (misregistration);
//! - leans every colour a little toward the place's few inks, so whatever
//!   stands there, from any drawing in the library, holds together as one
//!   limited palette.
//!
//! All of it is a pure function of where a point is and which drawing of
//! the boil it is ([`Hand::boil`]): no clock, no randomness, so the same
//! scene always paints the same pixels and the golden pictures hold. The
//! wobble is a field over the page, not over a shape, so two shapes that
//! share an edge still share it after the wobble, and a building painted a
//! tile at a time meets itself where the tiles meet.
//!
//! A still thing "boils" by being painted in [`BOILS`] drawings, shown in
//! turn [`BOIL_RATE`] times a second, as hand-drawn animation redraws a
//! still frame; Reduce Motion keeps the first.

use crate::brush::{Brush, Rect, Segment, Shape};
use gpui::{Hsla, RenderImage, Rgba};
use std::sync::Arc;

/// How many drawings of a still thing the boil goes through.
pub const BOILS: u32 = 3;
/// How many times a second the boil moves to its next drawing: gently,
/// well under the pace of anything that moves.
pub const BOIL_RATE: f32 = 2.5;

/// Which drawing of the boil shows `seconds` into looking, or the first
/// when everything is held still.
pub fn boil_at(seconds: f32, still: bool) -> u32 {
    if still || !seconds.is_finite() || seconds <= 0.0 {
        0
    } else {
        (seconds * BOIL_RATE) as u32 % BOILS
    }
}

/// How far apart, in drawing units, the wobble's turns are: a line
/// wanders over about this length.
const WAVE: f32 = 13.0;
/// How long the straight pieces a line is cut into are before it wobbles.
const STEP: f32 = 6.0;
/// How far a line wanders off where it was ruled, in drawing units.
const REACH: f32 = 0.55;
/// How far the ink line sits off its fill: its plate, slightly out.
const MISREGISTER: (f32, f32) = (0.45, -0.35);
/// How far a colour leans toward the nearest of the place's inks.
const LEAN: f32 = 0.2;
/// A shape narrower than this (in drawing units) is a detail: it wobbles,
/// but gets no darker edge or ink line, which would muddy it.
const DETAIL: f32 = 5.0;

/// A colour the hand leaves exactly as it is given, and draws no edge
/// around: a window's lamplight, which the painter finds by its colour.
const KEEP: [(u8, u8, u8); 1] = [(0xff, 0xd2, 0x7a)];

/// A brush that paints through `inner` by hand.
pub struct Hand<'a> {
    pub inner: &'a mut dyn Brush,
    /// Which drawing of the boil this is.
    pub boil: u32,
    /// The place's inks (0xRRGGBB) that colours lean toward; none keeps
    /// every colour as given.
    pub inks: &'static [u32],
}

impl<'a> Hand<'a> {
    pub fn new(inner: &'a mut dyn Brush, boil: u32, inks: &'static [u32]) -> Self {
        Self { inner, boil, inks }
    }

    fn salt(&self, line: u32) -> u32 {
        (self.boil % BOILS)
            .wrapping_mul(0x9e37_79b9)
            .wrapping_add(line.wrapping_mul(0x85eb_ca6b))
    }

    /// The colour leaned toward the nearest ink.
    fn lean(&self, colour: Hsla) -> Hsla {
        if self.inks.is_empty() || kept(colour) {
            return colour;
        }
        let rgba: Rgba = colour.into();
        let nearest = self
            .inks
            .iter()
            .map(|ink| {
                let (r, g, b) = channels(*ink);
                let d = (r - rgba.r).powi(2) * 0.3
                    + (g - rgba.g).powi(2) * 0.59
                    + (b - rgba.b).powi(2) * 0.11;
                (d, (r, g, b))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, ink)| ink);
        let Some((r, g, b)) = nearest else {
            return colour;
        };
        Rgba {
            r: rgba.r + (r - rgba.r) * LEAN,
            g: rgba.g + (g - rgba.g) * LEAN,
            b: rgba.b + (b - rgba.b) * LEAN,
            a: rgba.a,
        }
        .into()
    }
}

fn channels(ink: u32) -> (f32, f32, f32) {
    (
        ((ink >> 16) & 0xff) as f32 / 255.0,
        ((ink >> 8) & 0xff) as f32 / 255.0,
        (ink & 0xff) as f32 / 255.0,
    )
}

/// Whether a colour is one the hand must leave alone.
fn kept(colour: Hsla) -> bool {
    let rgba: Rgba = colour.into();
    let (r, g, b) = (rgba.r * 255.0, rgba.g * 255.0, rgba.b * 255.0);
    KEEP.iter().any(|(kr, kg, kb)| {
        (r - *kr as f32).abs() < 3.0 && (g - *kg as f32).abs() < 3.0 && (b - *kb as f32).abs() < 3.0
    })
}

/// Smooth noise over the page, from -1 to 1: values at the corners of a
/// lattice one unit apart, blended smoothly between them.
fn noise(x: f32, y: f32, salt: u32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (ix, iy) = (x0 as i32, y0 as i32);
    let at = |dx: i32, dy: i32| {
        crate::painter::hash2(ix.wrapping_add(dx), iy.wrapping_add(dy), salt) as f32
            / u32::MAX as f32
            * 2.0
            - 1.0
    };
    let top = at(0, 0) + (at(1, 0) - at(0, 0)) * sx;
    let bottom = at(0, 1) + (at(1, 1) - at(0, 1)) * sx;
    top + (bottom - top) * sy
}

/// Where the hand puts a point ruled at (`x`, `y`).
fn stray(x: f32, y: f32, salt: u32, reach: f32) -> (f32, f32) {
    let (u, v) = (x / WAVE, y / WAVE);
    (
        x + reach * noise(u, v, salt),
        y + reach * noise(u + 17.3, v - 9.1, salt ^ 0x5bd1_e995),
    )
}

/// The shape as the hand draws it: every straight line cut into short
/// pieces and every point moved by the wobble, the whole shape first
/// shifted by `offset`.
pub fn wobbled(shape: &Shape, salt: u32, reach: f32, offset: (f32, f32)) -> Shape {
    let mut out = Shape::new();
    let mut at = (0.0_f32, 0.0_f32);
    let mut start = at;
    let moved = |(x, y): (f32, f32)| stray(x + offset.0, y + offset.1, salt, reach);
    let line_to = |out: &mut Shape, from: (f32, f32), to: (f32, f32)| {
        let length = ((to.0 - from.0).powi(2) + (to.1 - from.1).powi(2)).sqrt();
        let pieces = ((length / STEP).ceil() as usize).clamp(1, 32);
        for index in 1..=pieces {
            let t = index as f32 / pieces as f32;
            out.line_p(moved((
                from.0 + (to.0 - from.0) * t,
                from.1 + (to.1 - from.1) * t,
            )));
        }
    };
    for segment in &shape.segments {
        match *segment {
            Segment::Move(x, y) => {
                out.move_p(moved((x, y)));
                at = (x, y);
                start = at;
            }
            Segment::Line(x, y) => {
                line_to(&mut out, at, (x, y));
                at = (x, y);
            }
            Segment::Curve(x, y, cx, cy) => {
                out.curve_p(moved((x, y)), moved((cx, cy)));
                at = (x, y);
            }
            Segment::Close => {
                if (at.0 - start.0).abs() + (at.1 - start.1).abs() > 0.01 {
                    line_to(&mut out, at, start);
                }
                out.close();
                at = start;
            }
        }
    }
    out
}

/// The box around a shape's points: left, top, right, bottom.
fn extent(shape: &Shape) -> Option<(f32, f32, f32, f32)> {
    let mut points = shape.segments.iter().flat_map(|segment| match *segment {
        Segment::Move(x, y) | Segment::Line(x, y) => vec![(x, y)],
        Segment::Curve(x, y, cx, cy) => vec![(x, y), (cx, cy)],
        Segment::Close => Vec::new(),
    });
    let (x, y) = points.next()?;
    Some(points.fold((x, y, x, y), |(l, t, r, b), (x, y)| {
        (l.min(x), t.min(y), r.max(x), b.max(y))
    }))
}

impl Brush for Hand<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        if w < 2.0 || h < 2.0 {
            let colour = self.lean(colour);
            self.inner.rect(x, y, w, h, radius, colour);
            return;
        }
        self.fill(&Shape::rounded(x, y, w, h, radius), colour);
    }

    fn fill(&mut self, shape: &Shape, colour: Hsla) {
        if colour.a <= 0.0 {
            return;
        }
        let paint = self.lean(colour);
        let drawn = wobbled(shape, self.salt(0), REACH, (0.0, 0.0));
        self.inner.fill(&drawn, paint);
        let Some((l, t, r, b)) = extent(shape) else {
            return;
        };
        if kept(colour) || (r - l).min(b - t) < DETAIL || colour.a < 0.6 {
            return;
        }
        // Where paint pools at the edge as it dries: a shade darker, soft.
        let pooled = crate::art::shade(paint, -0.28).opacity(0.16 * colour.a);
        self.inner.stroke(&drawn, 1.3, pooled);
        // The ink line, drawn on its own plate: its own wobble, a touch
        // off the fill.
        let inked = wobbled(shape, self.salt(1), REACH * 1.2, MISREGISTER);
        let ink = crate::art::shade(paint, -0.5).opacity(0.22 * colour.a);
        self.inner.stroke(&inked, 0.55, ink);
    }

    fn stroke(&mut self, shape: &Shape, width: f32, colour: Hsla) {
        let colour = self.lean(colour);
        let drawn = wobbled(shape, self.salt(1), REACH, MISREGISTER);
        self.inner.stroke(&drawn, width, colour);
    }

    fn soft(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, blur: f32, colour: Hsla) {
        self.inner.soft(cx, cy, rx, ry, blur, colour);
    }

    fn gradient(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle: f32,
        from: (Hsla, f32),
        to: (Hsla, f32),
    ) {
        let (from, to) = ((self.lean(from.0), from.1), (self.lean(to.0), to.1));
        self.inner.gradient(x, y, w, h, angle, from, to);
    }

    fn picture(&mut self, image: &Arc<RenderImage>, rect: Rect, clip: Rect) {
        self.inner.picture(image, rect, clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(shape: &Shape) -> Vec<(f32, f32)> {
        shape
            .segments
            .iter()
            .filter_map(|segment| match *segment {
                Segment::Move(x, y) | Segment::Line(x, y) | Segment::Curve(x, y, ..) => {
                    Some((x, y))
                }
                Segment::Close => None,
            })
            .collect()
    }

    #[test]
    fn the_hand_is_the_same_every_time_and_differs_between_drawings_of_the_boil() {
        let square = Shape::polygon(&[(10.0, 10.0), (90.0, 10.0), (90.0, 70.0), (10.0, 70.0)]);
        let once = wobbled(&square, 7, REACH, (0.0, 0.0));
        assert_eq!(once, wobbled(&square, 7, REACH, (0.0, 0.0)));
        assert_ne!(once, wobbled(&square, 8, REACH, (0.0, 0.0)));
        // Every point stays within reach of where it was ruled.
        for (x, y) in points(&once) {
            let on_edge = [
                (x - 10.0).abs(),
                (x - 90.0).abs(),
                (y - 10.0).abs(),
                (y - 70.0).abs(),
            ]
            .into_iter()
            .fold(f32::MAX, f32::min);
            assert!(on_edge <= REACH * 1.5, "({x}, {y}) strays {on_edge}");
        }
        // It is cut into pieces short enough to wander.
        assert!(points(&once).len() > 40, "{}", points(&once).len());
    }

    #[test]
    fn two_shapes_that_share_an_edge_still_share_it() {
        let wall = Shape::polygon(&[(0.0, 40.0), (60.0, 40.0), (60.0, 90.0), (0.0, 90.0)]);
        let roof = Shape::polygon(&[(0.0, 40.0), (30.0, 5.0), (60.0, 40.0)]);
        let wall = points(&wobbled(&wall, 3, REACH, (0.0, 0.0)));
        let roof = points(&wobbled(&roof, 3, REACH, (0.0, 0.0)));
        // The eave's corners land in the same place for both.
        assert!(wall.contains(&roof[0]) && wall.contains(&roof[roof.len() - 1]));
    }

    #[test]
    fn the_boil_holds_still_with_reduce_motion_and_turns_gently_otherwise() {
        assert_eq!(boil_at(12.3, true), 0);
        assert_eq!(boil_at(0.0, false), 0);
        let mut changes = 0;
        let mut last = 0;
        for step in 0..=1000 {
            let now = boil_at(step as f32 / 100.0, false);
            changes += usize::from(now != last);
            last = now;
        }
        // Ten seconds: two or three drawings a second, never more.
        assert!((20..=30).contains(&changes), "{changes}");
    }

    #[test]
    fn lamplight_is_left_exactly_as_it_is() {
        struct Record(Vec<Hsla>);
        impl Brush for Record {
            fn rect(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, c: Hsla) {
                self.0.push(c);
            }
            fn fill(&mut self, _: &Shape, c: Hsla) {
                self.0.push(c);
            }
            fn stroke(&mut self, _: &Shape, _: f32, c: Hsla) {
                self.0.push(c);
            }
            fn soft(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: Hsla) {}
            fn gradient(
                &mut self,
                _: f32,
                _: f32,
                _: f32,
                _: f32,
                _: f32,
                _: (Hsla, f32),
                _: (Hsla, f32),
            ) {
            }
        }
        let mut record = Record(Vec::new());
        let lamp = crate::art::hex(0xffd27a);
        Hand::new(&mut record, 1, crate::art::Setting::Harbour.inks())
            .rect(0.0, 0.0, 20.0, 20.0, 0.0, lamp);
        assert_eq!(record.0, vec![lamp], "a lit window is one fill, untouched");
    }
}
