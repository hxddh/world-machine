//! The player's mark on the place, as the scene shows it: plots staked out
//! on the ground where something could be built, and the designs the
//! player made, worn by flags, sails, shop signs and quilts.
//!
//! Everything here is presentation. A plot, its offers, a design and a
//! name are the World's (read from the snapshot); what is drawn is how the
//! painter shows them. A design's picture is painted off the window's
//! thread, as the still layers are, and drawn live: a flag waves and a sail
//! fills as the wind has them, with the hour's light and grade on the cloth
//! like on everything else.

use crate::art::{self, hex};
use crate::brush::{Brush, Shape};
use crate::painter::{self, Canvas, Key};
use gpui::{Hsla, RenderImage};
use std::sync::Arc;
use tiny_skia as sk;
use world_projection::{
    clean_name, command_with, CanvasItem, CanvasItemKind, Design, MarkShape, Pattern,
    ProjectionIntent, ProjectionSnapshot, Wears,
};
pub use world_projection::{Plot, PlotOffer};

/// How many cells a design is on a side.
pub const SIDE: usize = world_projection::PATTERN_SIDE;
/// How many cells a design has.
pub const CELLS: usize = SIDE * SIDE;
/// How many colours one design may use: one for each of the keys 1 to 8.
pub const SLOTS: usize = world_projection::MOST_PATTERN_COLOURS;

/// The fixed palette every design is painted from.
pub const PALETTE: [[u8; 3]; 16] = world_projection::PATTERN_PALETTE;

/// Each of the palette's colours by name, for a screen reader and the
/// pointer's tip.
pub const PALETTE_NAMES: [&str; 16] = [
    "Chalk white",
    "Soot",
    "Signal red",
    "Marigold",
    "Buttercup",
    "Meadow",
    "Bottle green",
    "Sky",
    "Harbour blue",
    "Navy",
    "Heather",
    "Rose",
    "Chestnut",
    "Sand",
    "Slate",
    "Peat",
];

/// The eight colours a new design starts with, as places in the palette:
/// light and dark, warm and cool, so anything can be drawn at once.
pub const STARTING_COLOURS: [u8; SLOTS] = [0, 9, 2, 4, 6, 7, 11, 12];

/// What a design is worn as.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Wear {
    /// A flag on a pole, flying in the wind.
    Flag,
    /// A boat's sail.
    Sail,
    /// A shop's hanging sign, a painted board.
    Sign,
    /// A quilt, out on the line by day and over a lit sill at night.
    Quilt,
}

impl From<Wears> for Wear {
    fn from(wears: Wears) -> Self {
        match wears {
            Wears::Flag => Self::Flag,
            Wears::Sail => Self::Sail,
            Wears::Sign => Self::Sign,
            Wears::Quilt => Self::Quilt,
        }
    }
}

impl Wear {
    /// What it is called, as the canvas's title says it: "Design for the
    /// flag".
    pub fn title(self) -> &'static str {
        match self {
            Self::Flag => "Design for the flag",
            Self::Sail => "Design for the sail",
            Self::Sign => "Design for the sign",
            Self::Quilt => "Design for the quilt",
        }
    }

    /// How wide the cloth is for its height.
    pub fn aspect(self) -> f32 {
        match self {
            Self::Flag => 1.3,
            Self::Sail => 0.78,
            Self::Sign => 1.25,
            Self::Quilt => 0.92,
        }
    }
}

/// A 16 by 16 design as the painter reads it: each cell an index into its
/// own few colours.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Motif {
    pub cells: [u8; CELLS],
    pub palette: Vec<[u8; 3]>,
}

impl Default for Motif {
    fn default() -> Self {
        Self {
            cells: [0; CELLS],
            palette: vec![PALETTE[0]],
        }
    }
}

impl Motif {
    /// A design from its stored form: 256 hex digits, row by row, each an
    /// index into `palette`. `None` if it is not one.
    pub fn parse(cells: &str, palette: &[[u8; 3]]) -> Option<Self> {
        if palette.is_empty() || palette.len() > 16 || cells.chars().count() != CELLS {
            return None;
        }
        let mut out = [0_u8; CELLS];
        for (slot, digit) in out.iter_mut().zip(cells.chars()) {
            let index = digit.to_digit(16)? as u8;
            if index as usize >= palette.len() {
                return None;
            }
            *slot = index;
        }
        Some(Self {
            cells: out,
            palette: palette.to_vec(),
        })
    }

    /// The design a World shows something wearing.
    pub fn of(pattern: &Pattern) -> Option<Self> {
        Self::parse(&pattern.cells, &pattern.palette)
    }

    /// The stored form of its cells: 256 hex digits.
    pub fn encode(&self) -> String {
        self.cells
            .iter()
            .map(|cell| char::from_digit(u32::from(*cell), 16).unwrap_or('0'))
            .collect()
    }

    /// The design as a World shows it.
    pub fn pattern(&self) -> Pattern {
        Pattern {
            cells: self.encode(),
            palette: self.palette.clone(),
        }
    }

    /// The colour of the cell in `column` and `row`.
    pub fn colour(&self, column: usize, row: usize) -> [u8; 3] {
        let index = self.cells[(row % SIDE) * SIDE + column % SIDE] as usize;
        self.palette.get(index).copied().unwrap_or(PALETTE[0])
    }

    /// A key that changes whenever the design does.
    pub fn key(&self) -> u64 {
        let mut key = Key::new("motif");
        key.add(self);
        key.finish()
    }
}

/// The plots a World offers, as the scene draws them.
pub fn plots_of(snapshot: &ProjectionSnapshot) -> &[Plot] {
    &snapshot.canvas.plots
}

/// The design something wears, if it wears one.
pub fn pattern_of(item: &CanvasItem) -> Option<Motif> {
    Motif::of(item.pattern.as_ref()?)
}

/// What something wears a design as, if anything: what the World says it
/// could wear, or for a design it wears already, what its shape suggests.
pub fn wear_of(item: &CanvasItem) -> Option<Wear> {
    if let Some(design) = &item.design {
        return Some(design.wears.into());
    }
    item.pattern.as_ref()?;
    match (item.kind, item.shape) {
        (CanvasItemKind::Actor, _) => None,
        (_, Some(MarkShape::Flag | MarkShape::Bunting)) => Some(Wear::Flag),
        (_, Some(MarkShape::Boat)) => Some(Wear::Sail),
        (_, Some(MarkShape::Shop | MarkShape::Stall)) => Some(Wear::Sign),
        _ => Some(Wear::Quilt),
    }
}

/// Whether the player may paint a design for something.
pub fn designable(item: &CanvasItem) -> bool {
    item.design.is_some()
}

/// Whether the player may name something.
pub fn nameable(item: &CanvasItem) -> bool {
    item.naming.is_some()
}

/// The names proposed for something, best first.
pub fn proposals(item: &CanvasItem) -> &[String] {
    item.naming
        .as_ref()
        .map(|naming| naming.proposals.as_slice())
        .unwrap_or_default()
}

/// What asks the World to paint `motif` on `item`: its design command with
/// the design as its argument. `None` if it takes no design, or the
/// design's colours are not the palette's.
pub fn design_intent(item: &CanvasItem, motif: &Motif) -> Option<ProjectionIntent> {
    let designable = item.design.as_ref()?;
    let design = Design::from_pattern(&motif.pattern()).ok()?;
    Some(ProjectionIntent::InvokeCommand(command_with(
        &designable.command,
        &design.text(),
    )))
}

/// What asks the World to name `item` `name`. `None` if it cannot be
/// named, or the name is not one.
pub fn name_intent(item: &CanvasItem, name: &str) -> Option<ProjectionIntent> {
    let naming = item.naming.as_ref()?;
    let name = clean_name(name).ok()?;
    Some(ProjectionIntent::InvokeCommand(command_with(
        &naming.command,
        &name,
    )))
}

/// Whether a name typed for something may be sent: a word or a few, on
/// one line, not too long.
pub fn valid_name(name: &str) -> bool {
    clean_name(name).is_ok()
}

/// A painted picture of a design, kept while it is shown.
#[derive(Clone)]
pub struct Picture(pub Arc<RenderImage>);

impl std::fmt::Debug for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Picture")
    }
}

/// A design worn by something on the stage.
#[derive(Clone, Debug)]
pub struct Worn {
    /// The item, by its index in the snapshot's canvas.
    pub index: usize,
    pub wear: Wear,
    pub pattern: Arc<Motif>,
}

/// How big a cloth is drawn on screen, in pixels, for something drawn
/// `w` pixels wide (a thing's width, or a building's).
pub fn cloth_size(wear: Wear, w: f32) -> (f32, f32) {
    let wide = match wear {
        Wear::Flag => w * 0.56,
        Wear::Sail => w * 0.5,
        Wear::Sign => w * 0.3,
        Wear::Quilt => w * 0.34,
    };
    (wide, wide / wear.aspect())
}

/// The picture a design is painted into for a cloth `w` by `h` pixels on
/// screen at `dpr`: sizes in steps, so a small zoom reuses it.
pub fn picture_size(w: f32, h: f32, dpr: f32) -> (u32, u32) {
    let step = |value: f32| {
        let device = (value * dpr).max(8.0);
        // Steps of an eighth of a doubling.
        let exponent = (device.log2() * 8.0).ceil() / 8.0;
        (2.0_f32.powf(exponent).round() as u32).clamp(8, 640)
    };
    (step(w), step(h))
}

/// The light a picture is painted in, rounded so it is painted again only
/// as the hour moves on noticeably.
pub fn rounded_light(light: [f32; 3]) -> [f32; 3] {
    light.map(|channel| (channel * 48.0).round() / 48.0)
}

/// The key of a design's picture.
pub fn picture_key(
    pattern: &Motif,
    wear: Wear,
    size: (u32, u32),
    light: [f32; 3],
    grade: ((u32, f32), (u32, f32)),
    mirror: bool,
) -> u64 {
    let mut key = Key::new("cloth");
    key.add((pattern.key(), wear, size, mirror))
        .add((grade.0 .0, grade.1 .0));
    for channel in light {
        key.float(channel);
    }
    key.float((grade.0 .1 * 64.0).round())
        .float((grade.1 .1 * 64.0).round());
    key.finish()
}

/// Paints a design as `wear` into a picture `size` device pixels, in the
/// hour's `light` (a multiplier per channel) and `grade` (the warm key
/// light and the cool shade), mirrored for a flag flying the other way.
/// Pure: the same design paints the same pixels.
pub fn paint_cloth(
    pattern: &Motif,
    wear: Wear,
    (width, height): (u32, u32),
    light: [f32; 3],
    grade: ((u32, f32), (u32, f32)),
    mirror: bool,
) -> Option<sk::Pixmap> {
    let mut canvas = Canvas::new(width, height, 1.0, (0.0, 0.0))?;
    let (w, h) = (width as usize, height as usize);
    let rgb = |hex: u32| {
        [
            ((hex >> 16) & 0xff) as f32 / 255.0,
            ((hex >> 8) & 0xff) as f32 / 255.0,
            (hex & 0xff) as f32 / 255.0,
        ]
    };
    let ((warm, warm_a), (cool, cool_a)) = grade;
    let (warm, cool) = (rgb(warm), rgb(cool));
    let chalk = PALETTE[0].map(|c| c as f32 / 255.0);
    let wood = rgb(0x6b4a33);
    // The darkest colour of the design binds a quilt's edge.
    let binding = pattern
        .palette
        .iter()
        .min_by_key(|c| c[0] as u32 * 3 + c[1] as u32 * 6 + c[2] as u32)
        .copied()
        .unwrap_or(PALETTE[15])
        .map(|c| c as f32 / 255.0 * 0.85);
    let frame = (w.min(h) as f32 * 0.085).max(2.0);
    let hem = (w as f32 * 0.07).max(2.0);
    let binding_w = (w.min(h) as f32 * 0.045).max(1.5);
    let data = canvas.pixmap.data_mut();
    for py in 0..h {
        for px in 0..w {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let (u, v) = (fx / w as f32, fy / h as f32);
            let mut alpha = 1.0_f32;
            // Where on the design this pixel is, and how lit.
            let (mut du, mut dv) = (if mirror { 1.0 - u } else { u }, v);
            let mut k = 1.0_f32;
            let mut colour: Option<[f32; 3]> = None;
            match wear {
                Wear::Flag => {
                    // The sleeve round the pole: plain canvas.
                    let from_pole = if mirror { w as f32 - fx } else { fx };
                    if from_pole < hem {
                        colour = Some(chalk);
                        k *= 0.9 + 0.1 * (from_pole / hem);
                    } else {
                        du = if mirror {
                            1.0 - (fx / (w as f32 - hem)).min(1.0)
                        } else {
                            ((fx - hem) / (w as f32 - hem)).max(0.0)
                        };
                    }
                }
                Wear::Sail => {
                    // A mainsail: the luff up the mast on the left, the
                    // foot along the boom, the leech curving between.
                    let reach = 0.08 + 0.92 * v.powf(0.9) + 0.07 * (v * std::f32::consts::PI).sin();
                    alpha = ((reach - u) * w as f32 + 0.5).clamp(0.0, 1.0);
                    // The belly: light where it fills, shade at the luff.
                    let across = (u / reach.max(0.05)).clamp(0.0, 1.0);
                    k *= 0.86 + 0.2 * (across * std::f32::consts::PI * 0.8).sin();
                    // The cloth's panels, seamed across.
                    let panel = (h as f32 / 7.0).max(3.0);
                    if (fy % panel) < 1.0 {
                        k *= 0.9;
                    }
                }
                Wear::Sign => {
                    let edge = fx.min(fy).min(w as f32 - fx).min(h as f32 - fy);
                    if edge < frame {
                        // The board's frame, bevelled: lit from the top left.
                        let lit = if fx.min(fy) < (w as f32 - fx).min(h as f32 - fy) {
                            1.18
                        } else {
                            0.78
                        };
                        colour = Some(wood);
                        k *= lit * (0.92 + 0.08 * (edge / frame));
                    } else {
                        du = (fx - frame) / (w as f32 - frame * 2.0);
                        dv = (fy - frame) / (h as f32 - frame * 2.0);
                        // A shadow the frame casts inside, and varnish.
                        let inner = edge - frame;
                        if inner < 1.5 {
                            k *= 0.82;
                        }
                        k *= 1.05 - 0.1 * dv;
                    }
                }
                Wear::Quilt => {
                    let edge = fx.min(fy).min(w as f32 - fx).min(h as f32 - fy);
                    if edge < binding_w {
                        colour = Some(binding);
                    } else {
                        du = (fx - binding_w) / (w as f32 - binding_w * 2.0);
                        dv = (fy - binding_w) / (h as f32 - binding_w * 2.0);
                        // Each block of four cells puffs up a little.
                        let (bu, bv) = ((du * 4.0).fract() - 0.5, (dv * 4.0).fract() - 0.5);
                        k *= 1.03 - 0.16 * (bu * bu + bv * bv);
                        // Quilted along the blocks: a light running stitch.
                        let (su, sv) = (du * 4.0, dv * 4.0);
                        let near = |t: f32, size: f32| {
                            let f = t - t.round();
                            (f.abs() * size) < 0.6 && t > 0.2 && t < 3.8
                        };
                        let dash = ((fx + fy) as u32 / 2).is_multiple_of(2);
                        if dash && (near(su, w as f32 / 4.0) || near(sv, h as f32 / 4.0)) {
                            colour = Some(chalk);
                            k *= 0.95;
                        }
                    }
                }
            }
            let base = colour.unwrap_or_else(|| {
                let column = ((du.clamp(0.0, 0.9999)) * SIDE as f32) as usize;
                let row = ((dv.clamp(0.0, 0.9999)) * SIDE as f32) as usize;
                // Each cell painted by hand: a touch lighter or darker.
                let jitter =
                    (painter::hash2(column as i32, row as i32, 0x3a7) % 1000) as f32 / 1000.0 - 0.5;
                k *= 1.0 + jitter * 0.05;
                pattern.colour(column, row).map(|c| c as f32 / 255.0)
            });
            // The weave of the cloth, very fine.
            if wear != Wear::Sign && (px + py) % 2 == 0 {
                k *= 0.975;
            }
            // The hour's grade: warm from the top left, cool low down.
            let diagonal = (u + v) * 0.5;
            let warm_w = warm_a * (1.0 - diagonal);
            let cool_w = cool_a * diagonal.powf(1.4);
            let mut out = [0.0_f32; 3];
            for channel in 0..3 {
                let lit = base[channel] * k;
                let graded =
                    lit * (1.0 - warm_w - cool_w) + warm[channel] * warm_w + cool[channel] * cool_w;
                out[channel] = (graded * light[channel]).clamp(0.0, 1.0);
            }
            let at = (py * w + px) * 4;
            data[at] = (out[0] * alpha * 255.0).round() as u8;
            data[at + 1] = (out[1] * alpha * 255.0).round() as u8;
            data[at + 2] = (out[2] * alpha * 255.0).round() as u8;
            data[at + 3] = (alpha * 255.0).round() as u8;
        }
    }
    // The paper it is all painted on.
    painter::grain(&mut canvas, (0, 0), 0.05, 0.04);
    Some(canvas.pixmap)
}

/// A plot as drawn this frame, in stage pixels at zoom 1.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotPaint {
    /// Which plot, by its index in the World's plots.
    pub plot: usize,
    pub x: f32,
    pub y: f32,
    pub w: f32,
}

/// How much of a plot shows at a zoom: all of it close up, fading out as
/// the place becomes a postcard.
pub fn plot_opacity(zoom: f32) -> f32 {
    let t = ((zoom - 0.45) / 0.4).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A plot staked out on the ground, centred on (`x`, `y`) where its front
/// edge lies, `w` wide on screen: a soft string between four stakes, a
/// pale patch of ground inside, and at one corner a small pennant on a
/// taller stake. `hot` when pointed at or chosen with the keys.
pub fn paint_plot(brush: &mut dyn Brush, x: f32, y: f32, w: f32, opacity: f32, hot: bool, t: f32) {
    if opacity <= 0.01 || w < 4.0 {
        return;
    }
    let deep = w * 0.2;
    let lift = if hot { 1.0 } else { 0.0 };
    // The four corners, the back edge a little shorter, as ground seen
    // from above and in front.
    let corners = [
        (x - w * 0.5, y),
        (x + w * 0.5, y),
        (x + w * 0.42, y - deep),
        (x - w * 0.42, y - deep),
    ];
    let ground = hex(0xf6efdc);
    brush.soft(
        x,
        y - deep * 0.5,
        w * 0.52,
        deep * 0.75,
        deep * 0.5,
        ground.opacity((0.16 + 0.12 * lift) * opacity),
    );
    let outline = Shape::polygon(&corners);
    // The string, and its shadow a hair below it.
    brush.stroke(
        &outline.mapped(|px, py| (px, py + 1.0)),
        1.2,
        gpui::black().opacity(0.16 * opacity),
    );
    let string = if hot { hex(0xfffaf0) } else { hex(0xeadfc4) };
    brush.stroke(
        &outline,
        if hot { 1.6 } else { 1.1 },
        string.opacity(0.9 * opacity),
    );
    // The stakes: nearer ones a little taller.
    let wood = hex(0x8a6a48);
    let stake = (w * 0.05).clamp(4.0, 10.0);
    for (index, (sx, sy)) in corners.iter().enumerate() {
        let tall = if index < 2 { stake } else { stake * 0.75 };
        brush.rect(
            sx - 1.0,
            sy - tall * 0.8,
            2.0,
            tall,
            0.5,
            wood.opacity(opacity),
        );
        brush.rect(
            sx - 1.0,
            sy - tall * 0.8,
            2.0,
            1.2,
            0.5,
            hex(0xc9a77c).opacity(opacity),
        );
    }
    // The marker: a pennant on a taller stake at the front left, stirring.
    let (mx, my) = corners[0];
    let pole = stake * 2.6 + lift * stake * 0.4;
    brush.rect(mx - 1.0, my - pole, 2.0, pole, 0.5, wood.opacity(opacity));
    let stir = (t * 2.3).sin() * pole * 0.05;
    let flag = hex(0xb8483a);
    brush.fill(
        &Shape::polygon(&[
            (mx + 1.0, my - pole),
            (mx + 1.0 + pole * 0.5, my - pole * 0.86 + stir),
            (mx + 1.0, my - pole * 0.7),
        ]),
        flag.opacity(opacity * if hot { 1.0 } else { 0.85 }),
    );
}

/// How the wind makes a flag's cloth ripple: how far each point along it
/// (`u`, 0 at the pole) lies above or below where it would hang, as a
/// fraction of its height, `t` seconds in.
pub fn ripple(u: f32, t: f32, wind: f32) -> f32 {
    let strength = (0.55 + 0.25 * wind.abs()).min(1.3);
    let phase = u * std::f32::consts::TAU * 1.1 - t * (3.2 + wind.abs() * 0.8);
    0.08 * strength * u.powf(0.8) * phase.sin()
}

/// How many strips a flag or sail `across` pixels is drawn in: about one
/// every three pixels, so the wave's edge stays smooth close up.
fn strips(across: f32) -> usize {
    (across / 3.0).clamp(12.0, 36.0) as usize
}

/// A flag flying from a pole at `pole_x`, its top edge at `top`, `cw` by
/// `ch` on screen, rippling `t` seconds in (still when `t` is held), in the
/// direction the `wind` blows. The picture was painted mirrored when it
/// blows to the left.
#[allow(clippy::too_many_arguments)]
pub fn paint_flag(
    brush: &mut dyn Brush,
    picture: &Picture,
    pole_x: f32,
    top: f32,
    cw: f32,
    ch: f32,
    t: f32,
    wind: f32,
) {
    let side = if wind < 0.0 { -1.0 } else { 1.0 };
    let left = if side > 0.0 { pole_x } else { pole_x - cw };
    let count = strips(cw);
    let strip = cw / count as f32;
    for index in 0..count {
        // Along the cloth from the pole, at the middle of the strip.
        let u = (index as f32 + 0.5) / count as f32;
        let dy = ripple(u, t, wind) * ch;
        let slope = (ripple(u + 0.02, t, wind) - ripple(u - 0.02, t, wind)) / 0.04;
        // Where the strip lies on screen.
        let x = if side > 0.0 {
            pole_x + index as f32 * strip
        } else {
            pole_x - (index + 1) as f32 * strip
        };
        // The picture is drawn whole and only this strip of it shows.
        brush.picture(
            &picture.0,
            (left, top + dy, cw, ch),
            (x - 0.3, top + dy - 1.0, strip + 0.6, ch + 2.0),
        );
        // Lit where the cloth turns up to the light, shaded where it
        // falls away.
        let shade = (-slope * side * 1.6).clamp(-1.0, 1.0);
        let colour = if shade > 0.0 {
            gpui::white().opacity(0.16 * shade)
        } else {
            gpui::black().opacity(0.2 * -shade)
        };
        if shade.abs() > 0.02 {
            brush.rect(x - 0.3, top + dy, strip + 0.6, ch, 0.0, colour);
        }
    }
}

/// A sail on a mast at `mast_x`, from `top` down `sh`, `sw` wide at the
/// foot, filling and easing with the wind `t` seconds in, the mast leaning
/// by `roll` radians about `pivot_y`.
#[allow(clippy::too_many_arguments)]
pub fn paint_sail(
    brush: &mut dyn Brush,
    picture: &Picture,
    mast_x: f32,
    top: f32,
    sw: f32,
    sh: f32,
    t: f32,
    wind: f32,
    roll: f32,
    pivot_y: f32,
) {
    let breathe = 0.75 + 0.25 * (t * 0.9).sin();
    let fill = (0.06 + 0.03 * wind.abs().min(2.0)) * breathe;
    let count = strips(sh);
    let strip = sh / count as f32;
    for index in 0..count {
        let v = (index as f32 + 0.5) / count as f32;
        let y = top + index as f32 * strip;
        // Filling: the middle of the sail stretches out further.
        let stretch = 1.0 + fill * (v * std::f32::consts::PI).sin();
        let lean = roll * (pivot_y - (y + strip / 2.0));
        brush.picture(
            &picture.0,
            (mast_x + lean, top, sw * stretch, sh),
            (
                mast_x + lean - 0.5,
                y - 0.3,
                sw * stretch + 1.0,
                strip + 0.6,
            ),
        );
    }
}

/// A shop's sign: a painted board hanging from an iron bracket out of the
/// wall at (`wall_x`, `y`), `bw` by `bh`, swinging a little.
#[allow(clippy::too_many_arguments)]
pub fn paint_sign(
    brush: &mut dyn Brush,
    picture: &Picture,
    wall_x: f32,
    y: f32,
    bw: f32,
    bh: f32,
    side: f32,
    swing: f32,
) {
    let iron = hex(0x2f2a28);
    let reach = bw * 1.15;
    let end = wall_x + side * reach;
    let centre = wall_x + side * (reach * 0.55);
    // The bracket, with a curl under it.
    art::line(brush, (wall_x, y), (end, y), 1.6, iron);
    art::line(
        brush,
        (wall_x, y + bh * 0.35),
        (wall_x + side * reach * 0.45, y),
        1.1,
        iron,
    );
    let (left, right) = (centre - bw / 2.0, centre + bw / 2.0);
    let drop = bh * 0.22;
    let sway = swing * bw * 0.04;
    for x in [left + bw * 0.12, right - bw * 0.12] {
        art::line(brush, (x, y), (x + sway, y + drop), 0.9, iron);
    }
    // Its shadow on the wall behind, then the board.
    brush.soft(
        centre + sway + side * bw * 0.06,
        y + drop + bh * 0.62,
        bw * 0.5,
        bh * 0.46,
        bh * 0.2,
        gpui::black().opacity(0.16),
    );
    brush.picture(
        &picture.0,
        (left + sway, y + drop, bw, bh),
        (left + sway, y + drop, bw, bh),
    );
}

/// A quilt out on the line between two posts, the line's middle at
/// (`x`, `line_y`), the posts standing on `ground`, `qw` by `qh`, stirring
/// in the wind `t` seconds in.
#[allow(clippy::too_many_arguments)]
pub fn paint_quilt_line(
    brush: &mut dyn Brush,
    picture: &Picture,
    x: f32,
    line_y: f32,
    ground: f32,
    qw: f32,
    qh: f32,
    t: f32,
    wind: f32,
) {
    let wood = hex(0x7a5a3e);
    let span = qw * 1.7;
    let (a, b) = (x - span / 2.0, x + span / 2.0);
    for post in [a, b] {
        brush.soft(post, ground, 3.0, 1.2, 1.5, gpui::black().opacity(0.18));
        brush.rect(
            post - 1.2,
            line_y - 3.0,
            2.4,
            ground - line_y + 3.0,
            0.8,
            wood,
        );
    }
    let sag = qh * 0.1;
    let mut line = Shape::new();
    line.move_to(a, line_y)
        .curve_to(b, line_y, x, line_y + sag * 2.0);
    brush.stroke(&line, 0.9, hex(0xe8e0cc));
    let hang = line_y + sag;
    let strips = 10;
    let strip = qh / strips as f32;
    for index in 0..strips {
        let v = (index as f32 + 0.5) / strips as f32;
        let dx = wind.signum() * qw * 0.05 * v * v * (0.6 + 0.4 * (t * 1.3 + v * 1.7).sin());
        let y = hang + index as f32 * strip;
        brush.picture(
            &picture.0,
            (x - qw / 2.0 + dx, hang, qw, qh),
            (x - qw / 2.0 + dx - 0.5, y - 0.3, qw + 1.0, strip + 0.6),
        );
    }
    // Pegs holding it to the line.
    for peg in [-0.4_f32, 0.0, 0.4] {
        brush.rect(x + peg * qw - 1.0, hang - 2.5, 2.0, 4.5, 0.6, hex(0xcaa679));
    }
}

/// A quilt aired over the sill of the window at `pane` (left, top,
/// width, height), hanging down the wall below it.
pub fn paint_quilt_sill(brush: &mut dyn Brush, picture: &Picture, pane: crate::brush::Rect) {
    let (wx, wy, ww, wh) = pane;
    let x = wx + ww / 2.0;
    let sill = wy + wh;
    let qw = ww * 1.25;
    let qh = qw / Wear::Quilt.aspect();
    brush.soft(
        x + 1.5,
        sill + qh * 0.5,
        qw * 0.5,
        qh * 0.5,
        3.0,
        gpui::black().opacity(0.18),
    );
    brush.picture(
        &picture.0,
        (x - qw / 2.0, sill - 2.0, qw, qh),
        (x - qw / 2.0, sill - 2.0, qw, qh),
    );
    brush.rect(
        x - qw / 2.0 - 1.0,
        sill - 3.0,
        qw + 2.0,
        2.2,
        0.6,
        hex(0x6b4a33),
    );
}

/// A quilt seen by lamplight at night on a bed through the window at
/// `pane` (left, top, width, height): the window lit, the quilt across its
/// lower part, a glow round it.
pub fn paint_quilt_window(
    brush: &mut dyn Brush,
    picture: &Picture,
    pane: crate::brush::Rect,
    glass: Hsla,
) {
    let (wx, wy, ww, wh) = pane;
    brush.soft(
        wx + ww / 2.0,
        wy + wh / 2.0,
        ww * 1.1,
        wh * 1.0,
        ww * 0.6,
        glass.opacity(0.3),
    );
    brush.rect(wx, wy, ww, wh, 2.0, glass);
    // The bed's quilt, seen from outside: its top edge turned down, the
    // rest below the sill out of sight.
    let inset = ww * 0.1;
    let top = wy + wh * 0.45;
    brush.picture(
        &picture.0,
        (
            wx + inset,
            top,
            ww - inset * 2.0,
            (ww - inset * 2.0) / Wear::Quilt.aspect(),
        ),
        (wx + inset, top, ww - inset * 2.0, wy + wh - top - 1.0),
    );
    brush.rect(
        wx + inset,
        top - 1.2,
        ww - inset * 2.0,
        1.6,
        0.5,
        hex(0xfff4dc),
    );
    // The glazing bar across it.
    brush.rect(
        wx + ww / 2.0 - 0.6,
        wy,
        1.2,
        wh,
        0.0,
        hex(0x6b4a33).opacity(0.7),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checks() -> Motif {
        let mut pattern = Motif {
            cells: [0; CELLS],
            palette: vec![PALETTE[0], PALETTE[2], PALETTE[1]],
        };
        for row in 0..SIDE {
            for column in 0..SIDE {
                pattern.cells[row * SIDE + column] =
                    ((row / 4 + column / 4) % 2) as u8 + if row == column { 1 } else { 0 };
            }
        }
        pattern
    }

    #[test]
    fn a_design_keeps_its_stored_form_exactly() {
        let pattern = checks();
        let stored = pattern.encode();
        assert_eq!(stored.len(), CELLS);
        assert_eq!(
            Motif::parse(&stored, &pattern.palette),
            Some(pattern.clone())
        );
        // An index past the palette, or the wrong length, is not a design.
        assert_eq!(
            Motif::parse(&stored.replace('1', "3"), &pattern.palette),
            None
        );
        assert_eq!(Motif::parse(&stored[1..], &pattern.palette), None);
        assert_eq!(Motif::parse(&stored, &[]), None);
    }

    #[test]
    fn a_design_goes_to_the_world_as_its_command_and_text() {
        let pattern = checks();
        let mut item = CanvasItem {
            design: Some(world_projection::Designable {
                command: "flag.design".into(),
                wears: Wears::Flag,
            }),
            ..Default::default()
        };
        let Some(ProjectionIntent::InvokeCommand(command)) = design_intent(&item, &pattern) else {
            panic!("a design command");
        };
        let (name, argument) = world_projection::command_argument(&command);
        assert_eq!(name, "flag.design");
        let design = Design::parse(argument.unwrap()).unwrap();
        assert_eq!(Motif::of(&design.pattern()), Some(pattern.clone()));
        // Something that takes no design is sent none.
        item.design = None;
        assert_eq!(design_intent(&item, &pattern), None);
        assert_eq!(wear_of(&item), None);
    }

    #[test]
    fn names_are_short_and_plain() {
        assert!(valid_name("Lark"));
        assert!(valid_name("小燕"));
        assert!(!valid_name("   "));
        assert!(!valid_name("Bad\u{7}name"));
        assert!(!valid_name(&"a".repeat(33)));
        let item = CanvasItem {
            naming: Some(world_projection::Naming {
                command: "boat.name".into(),
                proposals: vec!["Lark".into()],
            }),
            ..Default::default()
        };
        assert_eq!(
            name_intent(&item, "  Petrel "),
            Some(ProjectionIntent::InvokeCommand("boat.name=Petrel".into()))
        );
        assert_eq!(name_intent(&item, ""), None);
        assert_eq!(proposals(&item), ["Lark".to_string()]);
    }

    #[test]
    fn a_cloth_is_painted_the_same_every_time_and_lit_by_the_hour() {
        let pattern = checks();
        let grade = crate::diorama::grade_at(13.0);
        for wear in [Wear::Flag, Wear::Sail, Wear::Sign, Wear::Quilt] {
            let day = paint_cloth(&pattern, wear, (48, 40), [1.0; 3], grade, false).unwrap();
            let again = paint_cloth(&pattern, wear, (48, 40), [1.0; 3], grade, false).unwrap();
            assert_eq!(day.data(), again.data(), "{wear:?}");
            let night =
                paint_cloth(&pattern, wear, (48, 40), [0.34, 0.4, 0.6], grade, false).unwrap();
            let bright = |pixmap: &sk::Pixmap| {
                pixmap
                    .pixels()
                    .iter()
                    .map(|pixel| pixel.red() as u64 + pixel.green() as u64)
                    .sum::<u64>()
            };
            assert!(bright(&night) < bright(&day) * 3 / 4, "{wear:?}");
        }
        // A sail is cut to its shape: clear beyond the leech.
        let sail = paint_cloth(&pattern, Wear::Sail, (48, 60), [1.0; 3], grade, false).unwrap();
        assert_eq!(sail.pixel(46, 1).unwrap().alpha(), 0);
        assert_eq!(sail.pixel(1, 58).unwrap().alpha(), 255);
    }

    #[test]
    fn a_picture_is_repainted_only_in_steps() {
        let a = picture_size(40.0, 30.0, 2.0);
        let b = picture_size(40.5, 30.2, 2.0);
        assert_eq!(a, b);
        assert!(a.0 >= 80 && a.1 >= 60);
        assert_eq!(
            rounded_light([0.5001, 0.5, 1.0]),
            rounded_light([0.5, 0.5, 1.0])
        );
    }

    #[test]
    fn plots_fade_as_the_place_becomes_a_postcard() {
        assert_eq!(plot_opacity(1.0), 1.0);
        assert_eq!(plot_opacity(0.4), 0.0);
        assert!(plot_opacity(0.65) > 0.2 && plot_opacity(0.65) < 0.8);
    }
}
