//! One scale for everything (the art bible's §3): every drawing's height
//! in **P**, the height of a standing grown-up resident at the same depth.
//!
//! A postbox is 0.75 P, a door 1.15 P, a lamp post 1.8 P, a cottage's
//! ridge 2.75 P, a windmill 5 P. A thing is drawn so that what it paints
//! stands that tall beside the people at its depth, whatever the width of
//! the spot it was given; a long, low thing (a pier, a pond, a court) is
//! sized by its footprint instead, and its height follows its own drawing.
//!
//! Where the rungs come from:
//! - a drawing from the library: its row in the art catalog (`world-art`);
//! - a Pack's own drawing: the rung its Pack declares on it (none keeps the
//!   size its spot gives it);
//! - the plain shapes: [`of_plain`].
//!
//! How tall a drawing paints for the box it is given is measured, once per
//! drawing, by painting it: so the ladder holds for every drawing, the
//! library's and the Packs' alike, and a new drawing is held to it the
//! moment it is added (see the tests).
//!
//! Everything here is presentation.

use crate::art::{self, Palette, Setting};
use crate::brush::{Brush, Rect, Shape};
use crate::works::{self, Art, Drawn};
use gpui::{Hsla, RenderImage};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use world_projection::{Drawing, MarkShape, Mood, Stance};

/// A rung of the ladder: how tall something stands, or for a long, low
/// thing how wide it lies, in P.
pub use world_art::Rung;

/// The rung of a drawing from the library, from its row in the art
/// catalog. Every drawing has one: the test `every_drawing_has_a_rung`
/// keeps it so.
pub fn of_art(key: &str) -> Option<Rung> {
    world_art::rung(key)
}

/// Whether a drawing from the library stands in a garden or a field, and
/// so never taller than a grown-up. Held to it by a test.
pub fn in_a_field(key: &str) -> bool {
    world_art::is_field(key)
}

/// The rung of a plain shape (no library drawing, nor a Pack's own):
/// standing as a building, or as a thing.
pub fn of_plain(shape: MarkShape, building: bool) -> Rung {
    use MarkShape as M;
    use Rung::{Tall as T, Wide as W};
    if building {
        return match shape {
            M::House => T(2.75),
            M::Shop => T(3.0),
            M::Tower => T(5.0),
            M::Lamp => T(3.0),
            // A glasshouse, a domed hall.
            M::Dome | M::Tree => T(2.2),
            M::Bridge | M::Pier => W(4.5),
            // Anything else standing as a place is about a cottage.
            _ => T(2.75),
        };
    }
    match shape {
        M::Rover => T(1.0),
        // A working boat, its mast and all: its hull is about 0.5 P.
        M::Boat => T(1.2),
        M::Parcel => T(0.35),
        M::Stall => T(1.3),
        M::Bunting => T(2.2),
        M::Pier | M::Bridge => W(5.0),
        M::Garden => T(0.6),
        M::Flag => T(2.6),
        M::Lantern | M::Lamp => T(1.8),
        M::Tent => T(1.8),
        M::Bench => T(0.8),
        M::Sprouts => T(0.4),
        M::Well | M::Fountain => T(1.0),
        M::Swing => T(1.7),
        M::Signpost | M::Birdhouse => T(1.6),
        M::Planter => T(0.5),
        M::Statue => T(2.0),
        M::Postbox => T(0.75),
        M::House => T(2.75),
        M::Shop => T(3.0),
        M::Tower => T(5.0),
        M::Dome => T(2.8),
        M::Tree => T(3.5),
    }
}

/// How much of the box it is given a drawing paints: (across, up), as
/// shares of the box's width and height. A bench drawn in a box 1 wide
/// paints 0.8 across; a parcel's box is mostly air above it.
fn reach(subject: &Subject) -> (f32, f32) {
    static REACH: OnceLock<Mutex<HashMap<String, (f32, f32)>>> = OnceLock::new();
    let key = subject.key();
    if let Some(found) = REACH
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key)
    {
        return *found;
    }
    let (w, h) = subject.unit_box();
    let measured = measure(|brush, x, base| subject.paint(brush, x, base, w, h), w, h)
        .map(|(across, up)| (across / w, up / h))
        .unwrap_or((1.0, 1.0));
    REACH
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, measured);
    measured
}

/// What is being sized: a thing (drawn by its width), or a building
/// (drawn in a box), as the library, the plain shapes or the Pack draw it.
pub enum Subject<'a> {
    Thing(MarkShape, Setting, Option<Art>),
    Building(MarkShape, Setting, Option<Art>),
    Drawing(&'a Drawing),
}

impl Subject<'_> {
    fn drawn(&self) -> Option<Drawn> {
        match self {
            Subject::Thing(shape, setting, art) | Subject::Building(shape, setting, art) => {
                Some(match art {
                    Some(art) => Drawn::Art(*art),
                    None => works::drawn(None, *shape, *setting),
                })
            }
            Subject::Drawing(_) => None,
        }
    }

    fn key(&self) -> String {
        // The library's keys and the plain shapes' names can coincide, so
        // each says which it is.
        let drawn = match self.drawn() {
            Some(Drawn::Art(art)) => format!("art:{}", art.key()),
            Some(Drawn::Plain(shape)) => format!("plain:{shape:?}"),
            None => String::new(),
        };
        match self {
            Subject::Thing(..) => format!("thing:{drawn}"),
            Subject::Building(shape, ..) => format!("building:{shape:?}:{drawn}"),
            Subject::Drawing(drawing) => format!("drawing:{}:{}", drawing.id, drawing.aspect),
        }
    }

    /// The rung it stands on.
    pub fn rung(&self) -> Option<Rung> {
        match self {
            Subject::Drawing(drawing) => drawing.rung,
            _ => Some(match self.drawn()? {
                Drawn::Art(art) => of_art(art.key())?,
                Drawn::Plain(shape) => of_plain(shape, self.boxed()),
            }),
        }
    }

    /// Whether it is drawn in a box of its own (a building), rather than
    /// by its width alone (a thing, even one standing as a place).
    fn boxed(&self) -> bool {
        use MarkShape as M;
        match self {
            Subject::Thing(..) => false,
            Subject::Building(..) => match self.drawn() {
                Some(Drawn::Plain(shape)) => matches!(
                    shape,
                    M::House | M::Shop | M::Tower | M::Dome | M::Tree | M::Lamp | M::Bridge
                ),
                _ => true,
            },
            Subject::Drawing(_) => true,
        }
    }

    /// The box it is measured in: 100 high, as wide as it is usually
    /// drawn.
    fn unit_box(&self) -> (f32, f32) {
        match self {
            Subject::Building(..) if self.boxed() => match self.drawn() {
                Some(Drawn::Art(art)) => (100.0 / art.tall(), 100.0),
                _ => (115.0, 100.0),
            },
            Subject::Thing(..) | Subject::Building(..) => (100.0, 100.0),
            Subject::Drawing(drawing) => (100.0 * drawing.aspect, 100.0),
        }
    }

    fn palette(&self) -> Palette {
        let (setting, art) = match self {
            Subject::Thing(_, setting, art) | Subject::Building(_, setting, art) => {
                (*setting, *art)
            }
            Subject::Drawing(_) => (Setting::default(), None),
        };
        let mut palette = Palette::of_in("ladder", false, setting);
        palette.art = art;
        palette
    }

    /// Paints it standing on (`x`, `base`) in the box `w` by `h` (a thing
    /// is drawn by its width alone).
    pub fn paint(&self, brush: &mut dyn Brush, x: f32, base: f32, w: f32, h: f32) {
        let palette = self.palette();
        match self {
            Subject::Thing(shape, ..) => art::paint_thing(brush, x, base, w, *shape, &palette, 0.0),
            Subject::Building(shape, ..) => {
                art::paint_building(brush, x, base, w, h, *shape, &palette)
            }
            Subject::Drawing(drawing) => art::paint_drawing(
                brush,
                x,
                base,
                w,
                h,
                drawing,
                &art::Inks::of_place(&palette),
                Stance::Standing,
                Mood::Content,
                0.0,
                0.0,
                1.0,
            ),
        }
    }

    /// The box to draw it in so that it stands on its rung at a depth
    /// where a grown-up is `p` tall: (w, h). A thing's `h` is unused. A
    /// building's box keeps `nominal_w` (its spot's width) when its own
    /// drawing does not fix its proportions. `None`: it has no rung, and
    /// keeps the size its place gives it.
    pub fn sized(&self, p: f32, nominal_w: f32) -> Option<(f32, f32)> {
        let rung = self.rung()?;
        let (across, up) = reach(self);
        let (bw, bh) = self.unit_box();
        // The box that paints `rung`, scaled from the unit box.
        let k = match rung {
            Rung::Tall(tall) => tall * p / (up * bh).max(1e-3),
            Rung::Wide(wide) => wide * p / (across * bw).max(1e-3),
        };
        Some(match self {
            Subject::Thing(..) => (bw * k, bh * k),
            Subject::Building(..) => match (self.drawn(), rung) {
                (Some(Drawn::Plain(_)), Rung::Tall(_)) if self.boxed() => (nominal_w, bh * k),
                _ => (bw * k, bh * k),
            },
            Subject::Drawing(_) => (bw * k, bh * k),
        })
    }
}

/// How much ground an item takes when it is drawn on the ladder at a
/// depth where a grown-up is `p` tall, in the same units as `p`: the room
/// the composition should keep for it. As a place (`building`) or a
/// thing; drawn as the Pack's own `drawing`, or as its library art or
/// shape in `setting`. `None`: it has no rung (a person, or a drawing the
/// ladder does not know), and keeps the room it is given.
pub fn footprint(
    item: &world_projection::CanvasItem,
    drawing: Option<&Drawing>,
    setting: Setting,
    p: f32,
    building: bool,
) -> Option<f32> {
    let art = item.art.as_deref().and_then(Art::from_key);
    let shape = item.shape.unwrap_or(if building {
        MarkShape::House
    } else {
        MarkShape::Parcel
    });
    let subject = match drawing {
        Some(drawing) => Subject::Drawing(drawing),
        None if building => Subject::Building(shape, setting, art),
        None => Subject::Thing(shape, setting, art),
    };
    let (w, _) = subject.sized(p, p * 2.2)?;
    let (across, _) = reach(&subject);
    // A thing's box is wider than what it paints; a building's is what
    // it paints.
    Some(if subject.boxed() { w } else { w * across })
}

/// What a drawing paints when it is painted by `paint` standing on the
/// foot of a canvas: how far across and how far up from its foot it
/// reaches, in the units it was painted in. Soft shadows and glows are
/// left out; they are light on the ground, not the thing.
pub fn measure(paint: impl FnOnce(&mut dyn Brush, f32, f32), w: f32, h: f32) -> Option<(f32, f32)> {
    let side = (w.max(h) * 3.0).ceil() as u32;
    let mut canvas = crate::painter::Canvas::new(side, side, 1.0, (0.0, 0.0))?;
    let (x, base) = (side as f32 / 2.0, side as f32 * 0.8);
    {
        let mut hard = Hard(&mut canvas);
        paint(&mut hard, x, base);
    }
    let data = canvas.pixmap.data();
    let (mut top, mut left, mut right) = (u32::MAX, u32::MAX, 0);
    for row in 0..(base.ceil() as u32).min(side) {
        for column in 0..side {
            if data[((row * side + column) * 4 + 3) as usize] > 60 {
                top = top.min(row);
                left = left.min(column);
                right = right.max(column);
            }
        }
    }
    (top != u32::MAX).then(|| ((right + 1 - left) as f32, base - top as f32))
}

/// A brush that paints everything but soft light.
struct Hard<'a>(&'a mut dyn Brush);

impl Brush for Hard<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
        self.0.rect(x, y, w, h, radius, colour);
    }
    fn fill(&mut self, shape: &Shape, colour: Hsla) {
        self.0.fill(shape, colour);
    }
    fn stroke(&mut self, shape: &Shape, width: f32, colour: Hsla) {
        self.0.stroke(shape, width, colour);
    }
    fn soft(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: Hsla) {}
    #[allow(clippy::too_many_arguments)]
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
        self.0.gradient(x, y, w, h, angle, from, to);
    }
    fn picture(&mut self, image: &Arc<RenderImage>, rect: Rect, clip: Rect) {
        self.0.picture(image, rect, clip);
    }
}

#[cfg(test)]
#[path = "ladder_tests.rs"]
mod tests;
