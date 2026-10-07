//! The library of drawings: every work, made thing and building a World
//! can put up, each drawn as itself. A clock tower has a clock, a
//! telescope points at the sky, a bandstand is an open octagon under its
//! roof, and a statue has a face. A Pack names what something is
//! (`CanvasItem::art`), and this library draws it; anything it does not
//! name is drawn by its shape, in its setting's own way, so a cottage
//! never stands on Mars.
//!
//! Every drawing is a handful of flat shapes in a box that stands on the
//! ground, in a limited palette per setting. Nothing here is read back by
//! a World: drawings are presentation only.

use crate::art::{hex, shade, Palette, Setting};
use crate::brush::{Brush, Shape};
use gpui::Hsla;
use world_projection::MarkShape;

/// Which kind of place a drawing belongs to. A harbour drawing never
/// stands anywhere else.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Family {
    /// At home anywhere: a bench, a bandstand, a swing.
    Any,
    Harbour,
    Mars,
    Street,
    Ice,
}

impl Family {
    /// The setting it is drawn in on a contact sheet.
    pub fn setting(self) -> Setting {
        match self {
            Family::Any | Family::Harbour => Setting::Harbour,
            Family::Mars => Setting::Mars,
            Family::Street => Setting::Street,
            Family::Ice => Setting::Ice,
        }
    }

    /// Whether it may stand in `setting`.
    pub fn belongs_in(self, setting: Setting) -> bool {
        match self {
            Family::Any => true,
            Family::Harbour => setting == Setting::Harbour,
            Family::Mars => setting == Setting::Mars,
            Family::Street => setting == Setting::Street,
            Family::Ice => setting == Setting::Ice,
        }
    }
}

/// A drawing in the library.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Art(u16);

struct Def {
    key: &'static str,
    family: Family,
    /// How tall its box is for how wide.
    tall: f32,
    draw: fn(&mut Pen),
}

impl Art {
    /// The drawing a key names, if the library has it.
    pub fn from_key(key: &str) -> Option<Art> {
        use std::collections::HashMap;
        use std::sync::OnceLock;
        static INDEX: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();
        INDEX
            .get_or_init(|| {
                defs()
                    .iter()
                    .enumerate()
                    .map(|(index, def)| (def.key, index as u16))
                    .collect()
            })
            .get(key)
            .map(|index| Art(*index))
    }

    /// Every drawing in the library.
    pub fn all() -> impl Iterator<Item = Art> {
        (0..defs().len() as u16).map(Art)
    }

    fn def(self) -> &'static Def {
        &defs()[self.0 as usize]
    }

    pub fn key(self) -> &'static str {
        self.def().key
    }

    pub fn family(self) -> Family {
        self.def().family
    }

    /// How tall it stands for how wide.
    pub fn tall(self) -> f32 {
        self.def().tall
    }
}

/// What something on the scene is drawn as, when it is not its Pack's own
/// drawing: a drawing from the library, or the app's plain drawing of its
/// shape (the harbour's).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Drawn {
    Art(Art),
    Plain(MarkShape),
}

impl Drawn {
    /// The family of what is drawn: the plain shapes are the harbour's own,
    /// except those that are at home anywhere.
    pub fn family(self) -> Family {
        match self {
            Drawn::Art(art) => art.family(),
            Drawn::Plain(
                MarkShape::Flag
                | MarkShape::Bunting
                | MarkShape::Rover
                | MarkShape::Parcel
                | MarkShape::Sprouts,
            ) => Family::Any,
            Drawn::Plain(_) => Family::Harbour,
        }
    }

    /// A name for it: the art's key, or the harbour's shape.
    pub fn id(self) -> String {
        match self {
            Drawn::Art(art) => art.key().into(),
            Drawn::Plain(shape) => {
                let shape = format!("{shape:?}").to_lowercase();
                match self.family() {
                    Family::Harbour => format!("harbour-{shape}"),
                    _ => format!("plain-{shape}"),
                }
            }
        }
    }
}

/// What something is drawn as: its art if the library has it, otherwise
/// its setting's own drawing of its shape, otherwise the plain shape.
pub fn drawn(art: Option<&str>, shape: MarkShape, setting: Setting) -> Drawn {
    if let Some(art) = art.and_then(Art::from_key) {
        return Drawn::Art(art);
    }
    match kit(setting, shape) {
        Some(art) => Drawn::Art(art),
        None => Drawn::Plain(shape),
    }
}

/// The kind of place a cast stands in, from what its things are drawn as:
/// for a cover, which keeps the cast but not the place's setting. The
/// setting most of its drawings belong to, or the harbour.
pub fn setting_of_cast<'a>(arts: impl IntoIterator<Item = &'a str>) -> Setting {
    let mut counts = std::collections::BTreeMap::<Setting, usize>::new();
    for art in arts.into_iter().filter_map(Art::from_key) {
        let setting = match art.family() {
            Family::Mars => Setting::Mars,
            Family::Street => Setting::Street,
            Family::Ice => Setting::Ice,
            Family::Harbour => Setting::Harbour,
            Family::Any => continue,
        };
        *counts.entry(setting).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(setting, count)| (*count, *setting == Setting::Harbour))
        .map_or(Setting::Harbour, |(setting, _)| setting)
}

/// How a setting draws a shape nobody named: a dome on Mars is a
/// pressurised dome, a house on Maple Street a row house, a house on the
/// ice a snow nest. The harbour draws its shapes as they always were.
pub fn kit(setting: Setting, shape: MarkShape) -> Option<Art> {
    use MarkShape as M;
    let key = match setting {
        Setting::Harbour => return None,
        Setting::Mars => match shape {
            M::House => "hab-module",
            M::Shop => "supply-module",
            M::Tower => "comms-mast",
            M::Dome => "mars-dome",
            M::Tree => "grow-dome",
            M::Lamp => "beacon-mast",
            M::Bridge => "dome-walkway",
            M::Boat => "dust-sled",
            M::Pier => "landing-pad",
            M::Garden | M::Sprouts => "grow-tray",
            M::Lantern => "solar-lamp",
            M::Tent => "supply-tent",
            M::Bench => "metal-bench",
            M::Well => "condenser",
            M::Swing => "low-g-swing",
            M::Fountain => "mist-fountain",
            M::Signpost => "trail-marker",
            M::Birdhouse => "supply-cache",
            M::Planter => "planter-racks",
            M::Statue => "founders-statue",
            M::Postbox => "message-post",
            M::Stall => "cargo-depot",
            M::Parcel => "cargo-crate",
            M::Rover | M::Flag | M::Bunting => return None,
        },
        Setting::Street => match shape {
            M::House => "row-house",
            M::Shop => "storefront",
            M::Tower => "water-tower",
            M::Dome => "bandshell",
            M::Tree => "maple-tree",
            M::Lamp | M::Lantern => "streetlamp",
            M::Bridge => "steel-footbridge",
            M::Boat => "rowing-boat",
            M::Pier => "lake-dock",
            M::Garden | M::Sprouts => "flower-bed",
            M::Tent => "bus-shelter",
            M::Bench => "park-bench",
            M::Well => "water-pump",
            M::Swing => "playground-swing",
            M::Fountain => "drinking-fountain",
            M::Signpost => "street-sign",
            M::Birdhouse => "bird-feeder",
            M::Planter => "window-boxes",
            M::Statue => "mayor-statue",
            M::Postbox => "mailbox",
            M::Stall => "hot-dog-stand",
            M::Parcel => "newspaper-bundle",
            M::Rover => "city-bus",
            M::Flag | M::Bunting => return None,
        },
        Setting::Ice => match shape {
            M::House => "snow-nest",
            M::Shop => "ice-store",
            M::Tower => "ice-spire",
            M::Dome => "snow-dome",
            M::Tree => "kelp-frond",
            M::Lamp | M::Lantern => "lantern-post",
            M::Bridge => "ice-bridge",
            M::Boat => "kayak",
            M::Pier => "ice-ledge",
            M::Garden | M::Sprouts => "kelp-garden",
            M::Tent => "wind-shelter",
            M::Bench => "ice-bench",
            M::Well => "ice-well",
            M::Swing => "kelp-swing",
            M::Fountain => "geyser",
            M::Signpost => "snow-marker",
            M::Birdhouse => "nesting-box",
            M::Planter => "pebble-planters",
            M::Statue => "ice-sculpture",
            M::Postbox => "message-stone",
            M::Stall => "fish-stall",
            M::Parcel => "fish-crate",
            M::Rover | M::Flag | M::Bunting => return None,
        },
    };
    Art::from_key(key)
}

/// Paints what a building-sized item is drawn as, in the box `w` by `h`
/// standing on (`x`, `base`), if it is a drawing from the library; false
/// leaves it to the plain shapes.
pub fn paint(
    window: &mut dyn Brush,
    x: f32,
    base: f32,
    w: f32,
    h: f32,
    shape: MarkShape,
    palette: &Palette,
) -> bool {
    let Some(art) = palette.art.or_else(|| kit(palette.setting, shape)) else {
        return false;
    };
    paint_art(window, art, x, base, w, h, palette, 0.0);
    true
}

/// Paints a thing `w` wide from the library, if it is one; false leaves
/// it to the plain shapes.
pub fn paint_thing(
    window: &mut dyn Brush,
    x: f32,
    base: f32,
    w: f32,
    shape: MarkShape,
    palette: &Palette,
    sway: f32,
) -> bool {
    let Some(art) = palette.art.or_else(|| kit(palette.setting, shape)) else {
        return false;
    };
    // A thing's box is a little narrower than the spot it stands on, so
    // a bench is a bench beside a person and not a stage.
    let w = w * THING_SCALE;
    paint_art(window, art, x, base, w, w * art.tall(), palette, sway);
    true
}

/// How wide a thing from the library is drawn, for the width of its spot.
const THING_SCALE: f32 = 0.8;

/// Paints `art` in the box `w` by `h` standing on (`x`, `base`), keeping
/// its own proportions: as tall as the box allows, centred on its foot.
#[allow(clippy::too_many_arguments)]
pub fn paint_art(
    window: &mut dyn Brush,
    art: Art,
    x: f32,
    base: f32,
    w: f32,
    h: f32,
    palette: &Palette,
    sway: f32,
) {
    let tall = art.tall();
    let (bw, bh) = if w * tall > h {
        (h / tall, h)
    } else {
        (w, w * tall)
    };
    if bw <= 0.5 || bh <= 0.5 {
        return;
    }
    let mut pen = Pen {
        b: window,
        x,
        base,
        w: bw,
        h: bh,
        wall: palette.wall,
        roof: palette.roof,
        trim: palette.trim,
        glass: palette.glass,
        k: Kit::of(palette.setting),
        seed: palette.seed,
        sway,
    };
    (art.def().draw)(&mut pen);
}

/// A brush that paints everything in one colour: a drawing as its
/// silhouette on a far ridge. Soft shadows are left out.
struct Mono<'a> {
    inner: &'a mut dyn Brush,
    colour: Hsla,
}

impl Brush for Mono<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, _: Hsla) {
        self.inner.rect(x, y, w, h, radius, self.colour);
    }
    fn fill(&mut self, shape: &Shape, _: Hsla) {
        self.inner.fill(shape, self.colour);
    }
    fn stroke(&mut self, shape: &Shape, width: f32, _: Hsla) {
        self.inner.stroke(shape, width, self.colour);
    }
    fn soft(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: Hsla) {}
    fn gradient(&mut self, x: f32, y: f32, w: f32, h: f32, _: f32, _: (Hsla, f32), _: (Hsla, f32)) {
        self.inner.rect(x, y, w, h, 0.0, self.colour);
    }
}

/// Paints the silhouette of what `shape` is in `setting`, in the box `w`
/// by `h` on (`x`, `base`), in one `colour`, if the setting draws it its
/// own way: a dome on the far ridge of Mars rather than a cottage's gable.
/// False leaves it to the plain marks.
#[allow(clippy::too_many_arguments)]
pub fn paint_silhouette(
    window: &mut dyn Brush,
    setting: Setting,
    shape: MarkShape,
    x: f32,
    base: f32,
    w: f32,
    h: f32,
    colour: Hsla,
) -> bool {
    let Some(art) = kit(setting, shape) else {
        return false;
    };
    // Only buildings stand out against the sky that far off: a bench or a
    // statue on the horizon reads as nothing.
    if !matches!(
        shape,
        MarkShape::House | MarkShape::Shop | MarkShape::Tower | MarkShape::Dome | MarkShape::Lamp
    ) {
        return true;
    }
    let palette = Palette::of_in("ridge", false, setting);
    let mut mono = Mono {
        inner: window,
        colour,
    };
    paint_art(&mut mono, art, x, base, w, h, &palette, 0.0);
    true
}

/// The inks of a setting: few, and quiet, so a street holds together.
#[derive(Clone, Copy, Debug)]
pub struct Kit {
    pub wood: Hsla,
    pub stone: Hsla,
    pub metal: Hsla,
    pub dark: Hsla,
    /// A bright cloth or paint: the place's signature colour.
    pub accent: Hsla,
    /// A second, cooler one.
    pub second: Hsla,
    pub pale: Hsla,
    pub leaf: Hsla,
    pub bloom: Hsla,
    pub water: Hsla,
    pub ground: Hsla,
    pub brass: Hsla,
}

impl Kit {
    pub fn of(setting: Setting) -> Self {
        let k =
            |wood, stone, metal, dark, accent, second, pale, leaf, bloom, water, ground, brass| {
                Kit {
                    wood: hex(wood),
                    stone: hex(stone),
                    metal: hex(metal),
                    dark: hex(dark),
                    accent: hex(accent),
                    second: hex(second),
                    pale: hex(pale),
                    leaf: hex(leaf),
                    bloom: hex(bloom),
                    water: hex(water),
                    ground: hex(ground),
                    brass: hex(brass),
                }
            };
        match setting {
            Setting::Harbour => k(
                0x8a5a36, 0xc9c0b0, 0x4a5560, 0x2f3438, 0xc0463a, 0x2f4a6d, 0xf4f1ea, 0x5f8f4f,
                0xe8a0b0, 0x4f8fa8, 0xd8c9a0, 0xc9a04a,
            ),
            Setting::Mars => k(
                0x8a6a52, 0xb8795a, 0x6b6f78, 0x2e3038, 0xd9733b, 0x3f8f8a, 0xece6da, 0x6fa35a,
                0xf2c14e, 0x8fb8bd, 0xb8643e, 0xc9a04a,
            ),
            Setting::Street => k(
                0x7a5236, 0xb9b0a2, 0x4a4f58, 0x2c2a3a, 0xe0457b, 0x2bb3b1, 0xe8dcc4, 0x4f7f4a,
                0xf2c14e, 0x5a8fb0, 0x6a6770, 0xc8cdd2,
            ),
            Setting::Ice => k(
                0x9a8a78, 0x8e98a0, 0x3d4a5a, 0x2d3a4a, 0xe8963a, 0x7fb8d9, 0xf4f8fb, 0x5b6b3a,
                0xe8963a, 0x3f7f9a, 0xe6f1f6, 0xe8e0cc,
            ),
        }
    }
}

/// What a drawing is drawn with: its box, its paints, and a brush. Points
/// are `(u, v)`: `u` across the box from its middle (-0.5 to 0.5), `v` up
/// it from the ground (0 to 1). Sizes across are in widths of the box.
pub struct Pen<'a> {
    b: &'a mut dyn Brush,
    x: f32,
    base: f32,
    w: f32,
    h: f32,
    pub wall: Hsla,
    pub roof: Hsla,
    pub trim: Hsla,
    pub glass: Hsla,
    pub k: Kit,
    /// Something of its own, from its id, for small differences.
    pub seed: u32,
    /// How far it moves in the wind, -1 to 1.
    pub sway: f32,
}

impl Pen<'_> {
    fn at(&self, u: f32, v: f32) -> (f32, f32) {
        (self.x + u * self.w, self.base - v * self.h)
    }

    /// A box between two corners.
    pub fn rect(&mut self, u0: f32, v0: f32, u1: f32, v1: f32, colour: Hsla) {
        self.rr(u0, v0, u1, v1, 0.0, colour);
    }

    /// A box with round corners, `r` in widths.
    pub fn rr(&mut self, u0: f32, v0: f32, u1: f32, v1: f32, r: f32, colour: Hsla) {
        let (x0, y0) = self.at(u0.min(u1), v0.max(v1));
        let (x1, y1) = self.at(u0.max(u1), v0.min(v1));
        let (w, h) = (x1 - x0, y1 - y0);
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        self.b
            .rect(x0, y0, w, h, (r * self.w).min(w / 2.0).min(h / 2.0), colour);
    }

    pub fn poly(&mut self, points: &[(f32, f32)], colour: Hsla) {
        let points = points
            .iter()
            .map(|(u, v)| self.at(*u, *v))
            .collect::<Vec<_>>();
        crate::art::polygon(self.b, &points, colour);
    }

    /// An ellipse, `ru` in widths and `rv` in heights.
    pub fn ell(&mut self, u: f32, v: f32, ru: f32, rv: f32, colour: Hsla) {
        let (cx, cy) = self.at(u, v);
        crate::art::ellipse(self.b, cx, cy, ru * self.w, rv * self.h, colour);
    }

    /// A circle, `r` in widths.
    pub fn circ(&mut self, u: f32, v: f32, r: f32, colour: Hsla) {
        let (cx, cy) = self.at(u, v);
        crate::art::ellipse(self.b, cx, cy, r * self.w, r * self.w, colour);
    }

    /// A line `width` widths thick.
    pub fn line(&mut self, from: (f32, f32), to: (f32, f32), width: f32, colour: Hsla) {
        let (a, b) = (self.at(from.0, from.1), self.at(to.0, to.1));
        crate::art::line(self.b, a, b, (width * self.w).max(0.6), colour);
    }

    /// A curved line from `from` to `to`, bending toward `bend`.
    pub fn curve(
        &mut self,
        from: (f32, f32),
        bend: (f32, f32),
        to: (f32, f32),
        width: f32,
        colour: Hsla,
    ) {
        let (a, c, b) = (
            self.at(from.0, from.1),
            self.at(bend.0, bend.1),
            self.at(to.0, to.1),
        );
        let mut shape = Shape::new();
        shape.move_to(a.0, a.1).curve_to(b.0, b.1, c.0, c.1);
        self.b.stroke(&shape, (width * self.w).max(0.6), colour);
    }

    /// The top half of an ellipse standing on `v`.
    pub fn dome(&mut self, u: f32, v: f32, ru: f32, rv: f32, colour: Hsla) {
        let (cx, cy) = self.at(u, v);
        crate::art::dome(self.b, cx, cy, ru * self.w, rv * self.h, colour);
    }

    /// An opening with a round top: a door, an arched window.
    pub fn arch(&mut self, u0: f32, u1: f32, v0: f32, v1: f32, colour: Hsla) {
        let r = (u1 - u0).abs() / 2.0;
        let rv = r * self.w / self.h;
        let top = (v1 - rv).max(v0);
        self.rect(u0, v0, u1, top, colour);
        self.dome((u0 + u1) / 2.0, top, r, rv, colour);
    }

    /// A gable: a triangle from `u0` to `u1` on `v0`, peaking at `apex`.
    pub fn gable(&mut self, u0: f32, u1: f32, v0: f32, apex: f32, colour: Hsla) {
        self.poly(&[(u0, v0), ((u0 + u1) / 2.0, apex), (u1, v0)], colour);
    }

    /// A soft dark where it meets the ground, `ru` widths across.
    pub fn shadow(&mut self, ru: f32) {
        self.ell(0.0, 0.0, ru, 0.035, gpui::black().opacity(0.13));
    }

    /// A window: glass in a frame, lit at night.
    pub fn window(&mut self, u0: f32, v0: f32, u1: f32, v1: f32) {
        let frame = shade(self.trim, 0.1);
        self.rect(
            u0 - 0.01,
            v0 - 0.01 * self.w / self.h,
            u1 + 0.01,
            v1 + 0.01 * self.w / self.h,
            frame,
        );
        let glass = self.glass;
        self.rect(u0, v0, u1, v1, glass);
    }

    /// A round window.
    pub fn porthole(&mut self, u: f32, v: f32, r: f32) {
        let frame = shade(self.trim, 0.1);
        self.circ(u, v, r * 1.25, frame);
        let glass = self.glass;
        self.circ(u, v, r, glass);
    }

    /// A small face on a head at (`u`, `v`) of radius `r`, in `ink`: two
    /// eyes, a brow and a mouth, so a statue is someone.
    pub fn face(&mut self, u: f32, v: f32, r: f32, ink: Hsla) {
        let rv = r * self.w / self.h;
        for side in [-1.0_f32, 1.0] {
            self.circ(u + side * r * 0.38, v + rv * 0.1, r * 0.13, ink);
        }
        self.line(
            (u - r * 0.3, v - rv * 0.38),
            (u + r * 0.3, v - rv * 0.38),
            r * 0.12,
            ink,
        );
        self.line(
            (u - r * 0.05, v + rv * 0.05),
            (u + r * 0.05, v - rv * 0.15),
            r * 0.1,
            ink,
        );
    }

    /// A pitched roof over a wall from `u0` to `u1`, eaves at `v0`,
    /// ridge at `apex`, with a little overhang and its shadow.
    pub fn pitched(&mut self, u0: f32, u1: f32, v0: f32, apex: f32, colour: Hsla) {
        let eave = (u1 - u0) * 0.06;
        self.gable(u0 - eave, u1 + eave, v0, apex, colour);
        let under = shade(self.wall, -0.2);
        self.rect(u0, v0, u1, v0 - 0.02, under);
    }

    /// A small bright pixel of light, for a lamp or a beacon.
    pub fn light(&mut self, u: f32, v: f32, r: f32, colour: Hsla) {
        self.circ(u, v, r * 2.2, colour.opacity(0.25));
        self.circ(u, v, r, colour);
    }

    /// Bunting or a string of pennants from `from` to `to`, sagging.
    pub fn pennants(&mut self, from: (f32, f32), to: (f32, f32), count: usize, inks: &[Hsla]) {
        let sag = 0.06;
        let dark = self.k.dark;
        self.curve(
            from,
            ((from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0 - sag * 2.0),
            to,
            0.008,
            dark,
        );
        for index in 0..count {
            let t = (index as f32 + 0.5) / count as f32;
            let u = from.0 + (to.0 - from.0) * t;
            let v = from.1 + (to.1 - from.1) * t - sag * 4.0 * t * (1.0 - t);
            let size = (to.0 - from.0).abs() / count as f32 * 0.4;
            let ink = inks[index % inks.len()];
            self.poly(
                &[
                    (u - size, v),
                    (u + size, v),
                    (u, v - size * 1.8 * self.w / self.h),
                ],
                ink,
            );
        }
    }

    /// Flowers in a row from `u0` to `u1` at `v`, `count` of them.
    pub fn flowers(&mut self, u0: f32, u1: f32, v: f32, count: usize, size: f32) {
        let leaf = self.k.leaf;
        let inks = [self.k.bloom, self.k.pale, self.k.accent];
        for index in 0..count {
            let t = (index as f32 + 0.5) / count as f32;
            let u = u0 + (u1 - u0) * t;
            let lift = size * (1.2 + 0.6 * ((index * 7 + self.seed as usize) % 3) as f32) * self.w
                / self.h;
            self.line((u, v), (u, v + lift), size * 0.25, leaf);
            self.circ(u, v + lift, size * 0.55, inks[index % 3]);
        }
    }

    /// A round-topped tree: its trunk at `u`, its crown `r` widths across
    /// centred at `v`.
    pub fn tree(&mut self, u: f32, v: f32, r: f32) {
        let trunk = self.k.wood;
        let leaf = self.k.leaf;
        let rv = r * self.w / self.h;
        self.rect(u - r * 0.12, 0.0, u + r * 0.12, v, shade(trunk, -0.1));
        self.circ(u - r * 0.35, v - rv * 0.1, r * 0.7, shade(leaf, -0.08));
        self.circ(u + r * 0.35, v, r * 0.72, leaf);
        self.circ(u, v + rv * 0.4, r * 0.75, shade(leaf, 0.08));
    }
}

/// A key for how a drawing reads at a glance: its outline, filled, on a
/// small grid. Two drawings with the same outline read as the same thing
/// against the sky, whatever their colours.
pub fn silhouette(art: Art) -> u64 {
    use std::hash::{Hash, Hasher};
    let mask = outline(art, 40);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    mask.hash(&mut hasher);
    hasher.finish()
}

/// The drawing's outline as rows of filled cells, `side` cells across.
pub fn outline(art: Art, side: u32) -> Vec<bool> {
    let scale = 3;
    let w = side * scale;
    let h = ((side as f32 * art.tall()).ceil() as u32).max(1) * scale;
    let Some(mut canvas) = crate::painter::Canvas::new(w, h, 1.0, (0.0, 0.0)) else {
        return Vec::new();
    };
    let palette =
        Palette::of_in("outline", false, art.family().setting()).drawn_as(Some(art.key()));
    paint_art(
        &mut canvas,
        art,
        w as f32 / 2.0,
        h as f32,
        w as f32,
        h as f32,
        &palette,
        0.0,
    );
    let data = canvas.pixmap.data();
    let (cw, ch) = (w / scale, h / scale);
    let mut cells = Vec::with_capacity((cw * ch) as usize);
    for cy in 0..ch {
        for cx in 0..cw {
            let mut covered = 0;
            for dy in 0..scale {
                for dx in 0..scale {
                    let at = (((cy * scale + dy) * w + cx * scale + dx) * 4 + 3) as usize;
                    if data[at] > 100 {
                        covered += 1;
                    }
                }
            }
            cells.push(covered * 2 > scale * scale);
        }
    }
    cells
}

/// Every drawing in `arts`, each in its own setting's paints, in a grid
/// `columns` wide of `cell`-pixel cells on paper: for looking over the
/// library, and for the goldens.
pub fn contact_sheet(arts: &[Art], columns: u32, cell: u32) -> tiny_skia::Pixmap {
    let columns = columns.max(1);
    let rows = (arts.len() as u32).div_ceil(columns).max(1);
    let mut canvas = crate::painter::Canvas::new(columns * cell, rows * cell, 1.0, (0.0, 0.0))
        .expect("a contact sheet");
    canvas
        .pixmap
        .fill(tiny_skia::Color::from_rgba8(0xf3, 0xee, 0xe3, 0xff));
    for (index, art) in arts.iter().enumerate() {
        let (column, row) = (index as u32 % columns, index as u32 / columns);
        let (x0, y0) = ((column * cell) as f32, (row * cell) as f32);
        let setting = art.family().setting();
        let ground = Kit::of(setting).ground;
        let c = cell as f32;
        canvas.rect(
            x0 + 2.0,
            y0 + c * 0.86,
            c - 4.0,
            c * 0.12,
            2.0,
            ground.opacity(0.5),
        );
        let palette = Palette::of_in(art.key(), false, setting).drawn_as(Some(art.key()));
        paint_art(
            &mut canvas,
            *art,
            x0 + c / 2.0,
            y0 + c * 0.9,
            c * 0.84,
            c * 0.82,
            &palette,
            0.0,
        );
    }
    canvas.pixmap
}

fn defs() -> &'static [Def] {
    DEFS
}

macro_rules! library {
    ($( $key:literal, $family:ident, $tall:expr, $draw:path; )*) => {
        const DEFS: &[Def] = &[
            $( Def { key: $key, family: Family::$family, tall: $tall, draw: $draw }, )*
        ];
    };
}

mod harbour;
mod ice;
mod mars;
mod shared;
mod street;

library! {
    // ---- At home anywhere ---------------------------------------------
    "bandstand", Any, 0.95, shared::bandstand;
    "roofed-bandstand", Any, 1.05, shared::roofed_bandstand;
    "clock-tower", Any, 2.2, shared::clock_tower;
    "telescope", Any, 1.0, shared::telescope;
    "market-cross", Any, 1.5, shared::market_cross;
    "sundial", Any, 0.9, shared::sundial;
    "park-bench", Any, 0.42, shared::park_bench;
    "picnic-table", Any, 0.45, shared::picnic_table;
    "picnic-tables", Any, 0.4, shared::picnic_tables;
    "swing-set", Any, 0.8, shared::swing_set;
    "rowing-boat", Any, 0.4, shared::rowing_boat;
    "gazebo", Any, 1.0, shared::gazebo;
    "square-fountain", Any, 0.85, shared::square_fountain;
    "wishing-fountain", Any, 0.7, shared::wishing_fountain;
    "maypole", Any, 1.8, shared::maypole;
    "windmill", Any, 1.6, shared::windmill;
    "beehives", Any, 0.55, shared::beehives;
    "bee-garden", Any, 0.55, shared::bee_garden;
    "greenhouse", Any, 0.7, shared::glasshouse;
    "treehouse", Any, 1.3, shared::treehouse;
    "bleachers", Any, 0.5, shared::bleachers;
    "chess-tables", Any, 0.45, shared::chess_tables;
    "library", Any, 0.85, shared::library;
    "gym-hall", Any, 0.7, shared::gym_hall;
    "science-shed", Any, 0.75, shared::science_shed;
    "sailboat", Any, 1.1, shared::sailboat;
    "fishing-dock", Any, 0.45, shared::fishing_dock;
    "rope-swing", Any, 1.2, shared::rope_swing;
    "flower-beds", Any, 0.35, shared::flower_beds;
    "town-clock", Any, 1.9, shared::town_clock;
    "ice-cream-stand", Any, 0.95, shared::ice_cream_stand;
    "boathouse", Any, 0.85, shared::boathouse;
    "apple-tree", Any, 1.1, shared::apple_tree;
    "vegetable-patch", Any, 0.35, shared::vegetable_patch;
    "sunflowers", Any, 1.0, shared::sunflowers;
    "herb-pots", Any, 0.45, shared::herb_pots;
    "flower-boxes", Any, 0.5, shared::flower_boxes;
    "paper-lanterns", Any, 0.9, shared::paper_lanterns;
    "lamp-post", Any, 1.7, shared::lamp_post;
    "market-stall", Any, 0.9, shared::market_stall;
    "birdhouse", Any, 1.3, shared::birdhouse;
    "signpost", Any, 1.3, shared::signpost;
    "wall-fountain", Any, 0.9, shared::wall_fountain;
    "wellhead", Any, 0.8, shared::wellhead;
    "garden-swing", Any, 0.85, shared::garden_swing;
    "scaffold", Any, 1.0, shared::scaffold;
    // ---- The harbour --------------------------------------------------
    "lighthouse", Harbour, 2.4, harbour::lighthouse;
    "harbour-clock", Harbour, 1.9, harbour::harbour_clock;
    "sea-wall", Harbour, 0.45, harbour::sea_wall;
    "school-garden", Harbour, 0.55, harbour::school_garden;
    "fishers-statue", Harbour, 1.25, harbour::fishers_statue;
    "fisherman-statue", Harbour, 1.4, harbour::fisherman_statue;
    "roofed-well", Harbour, 1.0, harbour::roofed_well;
    "pillar-box", Harbour, 1.2, harbour::pillar_box;
    "lamp-box", Harbour, 1.3, harbour::lamp_box;
    "birdhouses", Harbour, 1.1, harbour::birdhouses;
    "fingerpost", Harbour, 1.4, harbour::fingerpost;
    "quay-planters", Harbour, 0.4, harbour::quay_planters;
    "lifeboat-station", Harbour, 0.9, harbour::lifeboat_station;
    "village-hall", Harbour, 0.7, harbour::village_hall;
    "bathing-huts", Harbour, 0.6, harbour::bathing_huts;
    "orchard", Harbour, 0.7, harbour::orchard;
    "smokehouse", Harbour, 1.0, harbour::smokehouse;
    "footbridge", Harbour, 0.45, harbour::footbridge;
    "cliff-path", Harbour, 0.8, harbour::cliff_path;
    "fish-market", Harbour, 0.6, harbour::fish_market;
    "bread-oven", Harbour, 0.8, harbour::bread_oven;
    "bell-tower", Harbour, 1.7, harbour::bell_tower;
    "puppet-theatre", Harbour, 1.1, harbour::puppet_theatre;
    "glasshouse", Harbour, 0.6, harbour::glasshouse_long;
    "rowing-club", Harbour, 0.85, harbour::rowing_club;
    "duck-pond", Harbour, 0.35, harbour::duck_pond;
    "row-cottages", Harbour, 0.75, harbour::row_cottages;
    "hill-beacon", Harbour, 1.6, harbour::hill_beacon;
    "ferry-shelter", Harbour, 0.75, harbour::ferry_shelter;
    "dovecote", Harbour, 1.5, harbour::dovecote;
    "seal-hide", Harbour, 0.7, harbour::seal_hide;
    "herb-garden", Harbour, 0.4, harbour::herb_garden;
    "tide-board", Harbour, 1.6, harbour::tide_board;
    "reading-room", Harbour, 0.95, harbour::reading_room;
    "boat-yard", Harbour, 0.7, harbour::boat_yard;
    "harbour-mural", Harbour, 0.55, harbour::harbour_mural;
    "lookout-tower", Harbour, 1.8, harbour::lookout_tower;
    "sea-pool", Harbour, 0.35, harbour::sea_pool;
    "net-loft", Harbour, 1.3, harbour::net_loft;
    "book-nook", Harbour, 1.0, harbour::book_nook;
    "tea-rooms", Harbour, 0.85, harbour::tea_rooms;
    "bread-cart", Harbour, 0.7, harbour::bread_cart;
    "pub-terrace", Harbour, 0.6, harbour::pub_terrace;
    "workshop", Harbour, 0.75, harbour::workshop;
    "bin-gate", Harbour, 0.6, harbour::bin_gate;
    "paddling-pool", Harbour, 0.35, harbour::paddling_pool;
    "lantern-walk", Harbour, 0.8, harbour::lantern_walk;
    "herring-shed", Harbour, 0.55, harbour::herring_shed;
    "windbreak", Harbour, 0.5, harbour::windbreak;
    "chapel", Harbour, 1.1, harbour::chapel;
    "jetty-ladder", Harbour, 0.6, harbour::jetty_ladder;
    "driftwood-bench", Harbour, 0.4, harbour::driftwood_bench;
    "oak-rope-swing", Harbour, 1.25, harbour::oak_rope_swing;
    "painted-stones", Harbour, 0.25, harbour::painted_stones;
    "notice-board", Harbour, 1.0, harbour::notice_board;
    "herb-bed", Harbour, 0.4, harbour::herb_bed;
    "cairn", Harbour, 0.9, harbour::cairn;
    "window-box-stand", Harbour, 0.6, harbour::window_box_stand;
    "bait-shed", Harbour, 0.9, harbour::bait_shed;
    "sandpit", Harbour, 0.3, harbour::sandpit;
    "bird-table", Harbour, 1.3, harbour::bird_table;
    "stepping-stones", Harbour, 0.25, harbour::stepping_stones;
    "book-box", Harbour, 1.3, harbour::book_box;
    "flag-line", Harbour, 1.0, harbour::flag_line;
    "net-rack", Harbour, 0.7, harbour::net_rack;
    "wildflowers", Harbour, 0.45, harbour::wildflowers;
    "wildflower-patch", Harbour, 0.32, harbour::wildflower_patch;
    "fire-pit", Harbour, 0.45, harbour::fire_pit;
    "hen-house", Harbour, 0.75, harbour::hen_house;
    "lookout-seat", Harbour, 1.3, harbour::lookout_seat;
    "jar-lanterns", Harbour, 1.3, harbour::jar_lanterns;
    "shell-path", Harbour, 0.2, harbour::shell_path;
    "boat-planter", Harbour, 0.45, harbour::boat_planter;
    "skittle-alley", Harbour, 0.4, harbour::skittle_alley;
    "weathervane", Harbour, 1.6, harbour::weathervane;
    "story-chair", Harbour, 1.0, harbour::story_chair;
    "kites", Harbour, 1.4, harbour::kites;
    "pool-name-boards", Harbour, 0.6, harbour::pool_name_boards;
    "pebble-mosaic", Harbour, 0.2, harbour::pebble_mosaic;
    "music-shed", Harbour, 0.9, harbour::music_shed;
    "apple-press", Harbour, 0.9, harbour::apple_press;
    "sail-loft", Harbour, 1.25, harbour::sail_loft;
    "crab-shack", Harbour, 0.8, harbour::crab_shack;
    "net-store", Harbour, 0.7, harbour::net_store;
    "radio-hut", Harbour, 1.3, harbour::radio_hut;
    "slipway", Harbour, 0.35, harbour::slipway;
    "fishers-shelter", Harbour, 0.7, harbour::fishers_shelter;
    "harbour-lamp", Harbour, 1.9, harbour::harbour_lamp;
    "boat-rack", Harbour, 0.7, harbour::boat_rack;
    "ice-house", Harbour, 0.7, harbour::ice_house;
    "mooring-bench", Harbour, 0.4, harbour::mooring_bench;
    "bookshop", Harbour, 1.0, harbour::bookshop;
    "cheese-shop", Harbour, 0.95, harbour::cheese_shop;
    "surgery", Harbour, 1.05, harbour::surgery;
    "pottery", Harbour, 0.95, harbour::pottery;
    "gallery", Harbour, 0.85, harbour::gallery;
    "quilting-room", Harbour, 0.85, harbour::quilting_room;
    "flower-stall", Harbour, 0.8, harbour::flower_stall;
    "sheepfold", Harbour, 0.4, harbour::sheepfold;
    "allotments", Harbour, 0.55, harbour::allotments;
    "weaving-shed", Harbour, 0.75, harbour::weaving_shed;
    "study-hut", Harbour, 0.9, harbour::study_hut;
    "story-stone", Harbour, 0.8, harbour::story_stone;
    "joinery", Harbour, 0.75, harbour::joinery;
    "summer-house", Harbour, 1.0, harbour::summer_house;
    "rose-arbour", Harbour, 1.0, harbour::rose_arbour;
    "frog-pond", Harbour, 0.35, harbour::frog_pond;
    "hilltop-swing", Harbour, 1.15, harbour::hilltop_swing;
    "clock-tower-square", Harbour, 2.4, harbour::clock_tower_square;
    "new-pier", Harbour, 0.5, harbour::new_pier;
    "point-lamp", Harbour, 1.4, harbour::point_lamp;
    // ---- Mars ---------------------------------------------------------
    "hab-module", Mars, 0.6, mars::hab_module;
    "supply-module", Mars, 0.65, mars::supply_module;
    "comms-mast", Mars, 2.2, mars::comms_mast;
    "mars-dome", Mars, 0.62, mars::mars_dome;
    "grow-dome", Mars, 0.6, mars::grow_dome;
    "beacon-mast", Mars, 2.0, mars::beacon_mast;
    "dome-walkway", Mars, 0.4, mars::dome_walkway;
    "dust-sled", Mars, 0.4, mars::dust_sled;
    "landing-pad", Mars, 0.25, mars::landing_pad;
    "grow-tray", Mars, 0.35, mars::grow_tray;
    "solar-lamp", Mars, 1.7, mars::solar_lamp;
    "supply-tent", Mars, 0.6, mars::supply_tent;
    "metal-bench", Mars, 0.42, mars::metal_bench;
    "condenser", Mars, 0.9, mars::condenser;
    "low-g-swing", Mars, 1.0, mars::low_g_swing;
    "mist-fountain", Mars, 0.85, mars::mist_fountain;
    "trail-marker", Mars, 1.2, mars::trail_marker;
    "supply-cache", Mars, 0.7, mars::supply_cache;
    "planter-racks", Mars, 0.7, mars::planter_racks;
    "founders-statue", Mars, 1.35, mars::founders_statue;
    "message-post", Mars, 1.3, mars::message_post;
    "cargo-depot", Mars, 0.6, mars::cargo_depot;
    "cargo-crate", Mars, 0.6, mars::cargo_crate;
    "sandbag-wall", Mars, 0.35, mars::sandbag_wall;
    "water-still", Mars, 0.9, mars::water_still;
    "mess-hall-dome", Mars, 0.5, mars::mess_hall_dome;
    "rover-shed", Mars, 0.6, mars::rover_shed;
    "relay-hut", Mars, 1.4, mars::relay_hut;
    "seed-vault", Mars, 0.7, mars::seed_vault;
    "storm-shelter", Mars, 0.5, mars::storm_shelter;
    "solar-field", Mars, 0.35, mars::solar_field;
    "greenhouse-annexe", Mars, 0.5, mars::greenhouse_annexe;
    "track-beacons", Mars, 0.6, mars::track_beacons;
    "clinic-bay", Mars, 0.65, mars::clinic_bay;
    "machine-shop", Mars, 0.85, mars::machine_shop;
    "observatory-dome", Mars, 0.85, mars::observatory_dome;
    "water-tank", Mars, 1.0, mars::water_tank;
    "dust-lock", Mars, 0.8, mars::dust_lock;
    "school-pod", Mars, 0.7, mars::school_pod;
    "cable-pylons", Mars, 0.8, mars::cable_pylons;
    "rec-dome", Mars, 0.7, mars::rec_dome;
    "radio-tower", Mars, 2.4, mars::radio_tower;
    "landing-stone", Mars, 0.9, mars::landing_stone;
    "algae-farm", Mars, 0.6, mars::algae_farm;
    "bunkhouse", Mars, 0.85, mars::bunkhouse;
    "rover-lift", Mars, 0.9, mars::rover_lift;
    "windbreak-panels", Mars, 0.5, mars::windbreak_panels;
    "trading-post", Mars, 0.7, mars::trading_post;
    "quiet-pod", Mars, 0.75, mars::quiet_pod;
    "library-module", Mars, 0.7, mars::library_module;
    "mine-headframe", Mars, 1.5, mars::mine_headframe;
    "flower-dome", Mars, 0.75, mars::flower_dome;
    "welcome-arch", Mars, 1.0, mars::welcome_arch;
    "hydroponics-dome", Mars, 0.6, mars::hydroponics_dome;
    "infirmary-dome", Mars, 0.7, mars::infirmary_dome;
    "mess-module", Mars, 0.55, mars::mess_module;
    "workshop-dome", Mars, 0.65, mars::workshop_dome;
    "music-pod", Mars, 0.8, mars::music_pod;
    "schoolroom-module", Mars, 0.6, mars::schoolroom_module;
    "fern-planter", Mars, 0.6, mars::fern_planter;
    "control-tower", Mars, 1.9, mars::control_tower;
    "rover-garage", Mars, 0.55, mars::rover_garage;
    "radio-dish", Mars, 1.1, mars::radio_dish;
    "landing-lights", Mars, 0.45, mars::landing_lights;
    "windsock", Mars, 1.4, mars::windsock;
    "fuel-tanks", Mars, 0.75, mars::fuel_tanks;
    "dust-shelter", Mars, 0.65, mars::dust_shelter;
    "solar-array", Mars, 0.6, mars::solar_array;
    "survey-station", Mars, 1.5, mars::survey_station;
    "ice-drill", Mars, 1.6, mars::ice_drill;
    "telescope-pad", Mars, 0.9, mars::telescope_pad;
    "crater-bench", Mars, 0.45, mars::crater_bench;
    "marker-cairn", Mars, 1.0, mars::marker_cairn;
    "greenhouse-tent", Mars, 0.6, mars::greenhouse_tent;
    "weather-mast", Mars, 2.0, mars::weather_mast;
    "lichen-garden", Mars, 0.35, mars::lichen_garden;
    "ridge-beacon", Mars, 1.5, mars::ridge_beacon;
    "second-habitat", Mars, 0.7, mars::second_habitat;
    "survey-rig", Mars, 1.2, mars::survey_rig;
    "dwarf-apple", Mars, 0.9, mars::dwarf_apple;
    "algae-beds", Mars, 0.35, mars::algae_beds;
    "red-moss", Mars, 0.3, mars::red_moss;
    "rest-canopy", Mars, 0.6, mars::rest_canopy;
    "fairy-lights-mars", Mars, 0.8, mars::fairy_lights;
    "crate-seat", Mars, 0.45, mars::crate_seat;
    "rover-seat", Mars, 0.55, mars::rover_seat;
    // ---- Maple Street, 1987 --------------------------------------------
    "row-house", Street, 0.95, street::row_house;
    "storefront", Street, 0.9, street::storefront;
    "water-tower", Street, 2.0, street::water_tower;
    "bandshell", Street, 0.75, street::bandshell;
    "maple-tree", Street, 1.3, street::maple_tree;
    "streetlamp", Street, 2.2, street::streetlamp;
    "steel-footbridge", Street, 0.5, street::steel_footbridge;
    "lake-dock", Street, 0.35, street::lake_dock;
    "flower-bed", Street, 0.35, street::flower_bed;
    "bus-shelter", Street, 0.65, street::bus_shelter;
    "water-pump", Street, 1.0, street::water_pump;
    "playground-swing", Street, 0.9, street::playground_swing;
    "drinking-fountain", Street, 1.0, street::drinking_fountain;
    "street-sign", Street, 1.9, street::street_sign;
    "bird-feeder", Street, 1.4, street::bird_feeder;
    "window-boxes", Street, 0.45, street::window_boxes;
    "mayor-statue", Street, 1.45, street::mayor_statue;
    "mailbox", Street, 1.0, street::mailbox;
    "hot-dog-stand", Street, 0.9, street::hot_dog_stand;
    "newspaper-bundle", Street, 0.5, street::newspaper_bundle;
    "parked-car", Street, 0.4, street::parked_car;
    "snack-bar", Street, 0.85, street::snack_bar;
    "call-in-booth", Street, 1.6, street::call_in_booth;
    "crosswalk", Street, 1.0, street::crosswalk;
    "mural-wall", Street, 0.55, street::mural_wall;
    "splash-pool", Street, 0.4, street::splash_pool;
    "roller-floor", Street, 0.45, street::roller_floor;
    "darkroom", Street, 0.8, street::darkroom;
    "clubhouse", Street, 0.9, street::clubhouse;
    "tall-antenna", Street, 2.6, street::tall_antenna;
    "garden-plots", Street, 0.45, street::garden_plots;
    "park-stage", Street, 0.7, street::park_stage;
    "streetlights", Street, 1.4, street::streetlights;
    "video-store", Street, 0.9, street::video_store;
    "dance-floor", Street, 0.7, street::dance_floor;
    "record-library", Street, 0.9, street::record_library;
    "basketball-court", Street, 1.0, street::basketball_court;
    "pay-phone", Street, 1.4, street::pay_phone;
    "drive-in-screen", Street, 0.8, street::drive_in_screen;
    "study-room", Street, 0.85, street::study_room;
    "median-beds", Street, 0.4, street::median_beds;
    "soapbox-track", Street, 0.5, street::soapbox_track;
    "neon-sign", Street, 1.4, street::neon_sign;
    "diner-board", Street, 1.1, street::diner_board;
    "radio-van", Street, 0.55, street::radio_van;
    "youth-centre", Street, 0.8, street::youth_centre;
    "winter-rink", Street, 0.4, street::winter_rink;
    "party-speakers", Street, 1.3, street::party_speakers;
    "computer-room", Street, 0.85, street::computer_room;
    "walk-of-fame", Street, 0.25, street::walk_of_fame;
    "school-greenhouse", Street, 0.6, street::school_greenhouse;
    "bus-depot", Street, 0.6, street::bus_depot;
    "time-capsule", Street, 0.8, street::time_capsule;
    "dog-run", Street, 0.45, street::dog_run;
    "rialto-marquee", Street, 1.2, street::rialto_marquee;
    "lake-path", Street, 0.4, street::lake_path;
    "street-gazebo", Street, 1.05, street::street_gazebo;
    "recording-studio", Street, 0.9, street::recording_studio;
    "market-stalls", Street, 0.6, street::market_stalls;
    "bank-clock", Street, 1.5, street::bank_clock;
    "skate-park", Street, 0.4, street::skate_park;
    "memorial-bench", Street, 0.5, street::memorial_bench;
    "arcade-upstairs", Street, 1.2, street::arcade_upstairs;
    "welcome-sign", Street, 0.8, street::welcome_sign;
    "transmitter", Street, 2.4, street::transmitter;
    "record-store", Street, 0.95, street::record_store;
    "diner", Street, 0.6, street::diner;
    "garage", Street, 0.65, street::garage;
    "arcade", Street, 0.9, street::arcade;
    "barber-shop", Street, 0.95, street::barber_shop;
    "newsstand", Street, 0.8, street::newsstand;
    "neon-lamp", Street, 1.6, street::neon_lamp;
    "quilt-shop", Street, 0.95, street::quilt_shop;
    "bike-rack", Street, 0.35, street::bike_rack;
    "school-bike-rack", Street, 0.5, street::school_bike_rack;
    "street-lamp-post", Street, 1.9, street::street_lamp_post;
    "flagpole-street", Street, 2.2, street::flagpole;
    "streamers", Street, 0.9, street::streamers;
    "fairy-lights-street", Street, 0.8, street::fairy_lights;
    "tomato-patch", Street, 0.45, street::tomato_patch;
    "rose-bed", Street, 0.45, street::rose_bed;
    "second-street", Street, 0.95, street::second_street;
    "radio-beacon", Street, 2.0, street::radio_beacon;
    "street-survey", Street, 1.1, street::street_survey;
    "tape-shelf", Street, 1.0, street::tape_shelf;
    "city-bus", Street, 0.4, street::city_bus;
    // ---- Icebridge ------------------------------------------------------
    "snow-nest", Ice, 0.55, ice::snow_nest;
    "ice-store", Ice, 0.65, ice::ice_store;
    "ice-spire", Ice, 1.9, ice::ice_spire;
    "snow-dome", Ice, 0.55, ice::snow_dome;
    "kelp-frond", Ice, 1.2, ice::kelp_frond;
    "lantern-post", Ice, 1.7, ice::lantern_post;
    "ice-bridge", Ice, 0.4, ice::ice_bridge;
    "kayak", Ice, 0.25, ice::kayak;
    "ice-ledge", Ice, 0.35, ice::ice_ledge;
    "kelp-garden", Ice, 0.4, ice::kelp_garden;
    "wind-shelter", Ice, 0.6, ice::wind_shelter;
    "ice-bench", Ice, 0.4, ice::ice_bench;
    "ice-well", Ice, 0.6, ice::ice_well;
    "kelp-swing", Ice, 1.1, ice::kelp_swing;
    "geyser", Ice, 1.0, ice::geyser;
    "snow-marker", Ice, 1.2, ice::snow_marker;
    "nesting-box", Ice, 0.6, ice::nesting_box;
    "pebble-planters", Ice, 0.4, ice::pebble_planters;
    "ice-sculpture", Ice, 1.3, ice::ice_sculpture;
    "message-stone", Ice, 0.8, ice::message_stone;
    "fish-stall", Ice, 0.75, ice::fish_stall;
    "fish-crate", Ice, 0.5, ice::fish_crate;
    "snow-wall", Ice, 0.4, ice::snow_wall;
    "fishing-hole", Ice, 0.35, ice::fishing_hole;
    "vault-ice-house", Ice, 0.7, ice::vault_ice_house;
    "creche", Ice, 0.6, ice::creche;
    "bridge-lanterns", Ice, 0.8, ice::bridge_lanterns;
    "berg-lookout", Ice, 1.6, ice::berg_lookout;
    "sea-slide", Ice, 0.5, ice::sea_slide;
    "council-ring", Ice, 0.4, ice::council_ring;
    "kelp-beds", Ice, 0.35, ice::kelp_beds;
    "thaw-marker", Ice, 1.3, ice::thaw_marker;
    "second-vault", Ice, 0.6, ice::second_vault;
    "rope-bridge", Ice, 0.5, ice::rope_bridge;
    "song-stone", Ice, 1.1, ice::song_stone;
    "egg-warmer", Ice, 0.5, ice::egg_warmer;
    "ridge-steps", Ice, 0.6, ice::ridge_steps;
    "deep-ledge", Ice, 0.45, ice::deep_ledge;
    "story-circle", Ice, 0.35, ice::story_circle;
    "breathing-hole", Ice, 0.3, ice::breathing_hole;
    "aurora-seat", Ice, 0.55, ice::aurora_seat;
    "nest-row", Ice, 0.4, ice::nest_row;
    "kelp-racks", Ice, 0.7, ice::kelp_racks;
    "bridge-gate", Ice, 1.0, ice::bridge_gate;
    "seal-watch", Ice, 1.6, ice::seal_watch;
    "snow-hall", Ice, 0.6, ice::snow_hall;
    "run-markers", Ice, 0.7, ice::run_markers;
    "far-lantern", Ice, 1.8, ice::far_lantern;
    "chick-slide", Ice, 0.6, ice::chick_slide;
    "bone-arch", Ice, 1.0, ice::bone_arch;
    "salt-pans", Ice, 0.25, ice::salt_pans;
    "thaw-channel", Ice, 0.3, ice::thaw_channel;
    "swim-pool", Ice, 0.35, ice::swim_pool;
    "pebble-market", Ice, 0.6, ice::pebble_market;
    "night-beacon", Ice, 1.9, ice::night_beacon;
    "elders-ramp", Ice, 0.4, ice::elders_ramp;
    "name-wall", Ice, 0.55, ice::name_wall;
    "shell-horn", Ice, 1.2, ice::shell_horn;
    "floe-bridge", Ice, 0.35, ice::floe_bridge;
    "chick-nursery", Ice, 0.55, ice::chick_nursery;
    "song-circle", Ice, 0.5, ice::song_circle;
    "fish-larder", Ice, 0.7, ice::fish_larder;
    "lantern-ring", Ice, 0.5, ice::lantern_ring;
    "story-berg", Ice, 1.2, ice::story_berg;
    "pebble-garden", Ice, 0.3, ice::pebble_garden;
    "snow-house", Ice, 0.6, ice::snow_house;
    "ice-slide", Ice, 0.7, ice::ice_slide;
    "ice-market", Ice, 0.65, ice::ice_market;
    "carving-hall", Ice, 0.65, ice::carving_hall;
    "lookout-post", Ice, 1.7, ice::lookout_post;
    "skating-rink", Ice, 0.3, ice::skating_rink;
    "bell-post", Ice, 1.5, ice::bell_post;
    "snow-arch", Ice, 0.9, ice::snow_arch;
    "warming-hut", Ice, 0.7, ice::warming_hut;
    "kayak-shelter", Ice, 0.6, ice::kayak_shelter;
    "whale-watch", Ice, 1.5, ice::whale_watch;
    "sail-sled", Ice, 1.0, ice::sail_sled;
    "moss-patch", Ice, 0.25, ice::moss_patch;
    "seal-fence", Ice, 0.4, ice::seal_fence;
    "far-beacon", Ice, 1.7, ice::far_beacon;
    "snow-maze", Ice, 0.35, ice::snow_maze;
    "glow-stones", Ice, 0.4, ice::glow_stones;
    "snow-flag", Ice, 1.6, ice::snow_flag;
    "pennant-line", Ice, 0.8, ice::pennant_line;
    "geyser-fountain", Ice, 1.2, ice::geyser_fountain;
    "seaweed-bed", Ice, 0.3, ice::seaweed_bed;
    "lichen-patch", Ice, 0.25, ice::lichen_patch;
    "fishing-hole-bench", Ice, 0.45, ice::fishing_hole_bench;
    "second-nests", Ice, 0.5, ice::second_nests;
    "ice-beacon", Ice, 1.8, ice::ice_beacon;
    "ice-survey", Ice, 1.2, ice::ice_survey;
    "sculpture-garden", Ice, 0.8, ice::sculpture_garden;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_a_plain_name_and_named_once() {
        let mut seen = std::collections::BTreeSet::new();
        for art in Art::all() {
            let key = art.key();
            assert!(
                key.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{key}"
            );
            assert!(seen.insert(key), "{key} twice");
            assert_eq!(Art::from_key(key), Some(art));
        }
    }

    #[test]
    fn no_two_drawings_in_the_library_share_an_outline() {
        let mut seen = std::collections::BTreeMap::new();
        let mut shared = Vec::new();
        for art in Art::all() {
            let cells = outline(art, 40);
            assert!(
                cells.iter().filter(|cell| **cell).count() >= 12,
                "{} draws almost nothing",
                art.key()
            );
            if let Some(other) = seen.insert(silhouette(art), art.key()) {
                shared.push(format!("{} and {}", art.key(), other));
            }
        }
        assert!(shared.is_empty(), "these share an outline: {shared:?}");
    }

    #[test]
    fn every_setting_but_the_harbour_draws_every_shape_its_own_way() {
        use MarkShape as M;
        let shapes = [
            M::House,
            M::Dome,
            M::Tower,
            M::Tree,
            M::Lamp,
            M::Shop,
            M::Bridge,
            M::Rover,
            M::Boat,
            M::Parcel,
            M::Stall,
            M::Bunting,
            M::Pier,
            M::Garden,
            M::Flag,
            M::Lantern,
            M::Tent,
            M::Bench,
            M::Sprouts,
            M::Well,
            M::Swing,
            M::Fountain,
            M::Signpost,
            M::Birdhouse,
            M::Planter,
            M::Statue,
            M::Postbox,
        ];
        for setting in [Setting::Mars, Setting::Street, Setting::Ice] {
            for shape in shapes {
                let drawn = drawn(None, shape, setting);
                assert!(
                    drawn.family().belongs_in(setting),
                    "{shape:?} in {setting:?} is {}",
                    drawn.id()
                );
            }
        }
    }

    /// Writes every drawing's contact sheet to `WORLD_GPUI_CONTACT_SHEET`,
    /// for looking over the library by eye.
    #[test]
    #[ignore]
    fn write_contact_sheets() {
        let Ok(dir) = std::env::var("WORLD_GPUI_CONTACT_SHEET") else {
            return;
        };
        for family in [
            Family::Any,
            Family::Harbour,
            Family::Mars,
            Family::Street,
            Family::Ice,
        ] {
            let arts = Art::all()
                .filter(|art| art.family() == family)
                .collect::<Vec<_>>();
            let sheet = contact_sheet(&arts, 10, 120);
            let path = format!("{dir}/{family:?}.png").to_lowercase();
            let image =
                image::RgbaImage::from_raw(sheet.width(), sheet.height(), sheet.data().to_vec())
                    .unwrap();
            image.save(&path).unwrap();
            let keys = arts
                .iter()
                .enumerate()
                .map(|(index, art)| format!("{index:3} {}", art.key()))
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(format!("{dir}/{family:?}.txt").to_lowercase(), keys).unwrap();
        }
    }
}
