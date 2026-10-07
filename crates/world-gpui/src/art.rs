//! Drawing the things a World is made of: people as small figures in the
//! clothes of their work, places as the buildings they are, vehicles,
//! boats and parcels, and the landscape they stand in. Everything is
//! painted from shapes, so any World a Pack describes can be drawn without
//! shipping a picture.

use crate::brush::{Brush, Shape, Xform};
use gpui::{rgb, Bounds, Hsla};
use world_projection::{Carry, Look, MarkShape};

/// A colour from 0xRRGGBB.
pub fn hex(colour: u32) -> Hsla {
    rgb(colour).into()
}

/// A colour moved toward black (`by` < 0) or white (`by` > 0).
pub fn shade(colour: Hsla, by: f32) -> Hsla {
    let mut shaded = colour;
    shaded.l = if by < 0.0 {
        colour.l * (1.0 + by)
    } else {
        colour.l + (1.0 - colour.l) * by
    };
    shaded
}

pub fn rect(window: &mut dyn Brush, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
    window.rect(x, y, w, h, radius, colour);
}

pub fn circle(window: &mut dyn Brush, cx: f32, cy: f32, r: f32, colour: Hsla) {
    rect(window, cx - r, cy - r, r * 2.0, r * 2.0, r, colour);
}

pub fn polygon(window: &mut dyn Brush, points: &[(f32, f32)], colour: Hsla) {
    if points.is_empty() {
        return;
    }
    window.fill(&Shape::polygon(points), colour);
}

pub fn line(window: &mut dyn Brush, from: (f32, f32), to: (f32, f32), width: f32, colour: Hsla) {
    let mut shape = Shape::new();
    shape.move_to(from.0, from.1).line_to(to.0, to.1);
    window.stroke(&shape, width, colour);
}

/// An ellipse, as eight curved segments.
pub fn ellipse(window: &mut dyn Brush, cx: f32, cy: f32, rx: f32, ry: f32, colour: Hsla) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    window.fill(&Shape::ellipse(cx, cy, rx, ry), colour);
}

/// The top half of an ellipse standing on the line through (`cx`, `cy`).
pub fn dome(window: &mut dyn Brush, cx: f32, cy: f32, rx: f32, ry: f32, colour: Hsla) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    let step = std::f32::consts::PI / 4.0;
    let reach = 1.0 / (step / 2.0).cos();
    let at =
        |angle: f32, scale: f32| (cx + rx * scale * angle.cos(), cy - ry * scale * angle.sin());
    let mut shape = Shape::new();
    let (x, y) = at(0.0, 1.0);
    shape.move_to(x, y);
    for index in 0..4 {
        let start = index as f32 * step;
        let (x, y) = at(start + step, 1.0);
        let (qx, qy) = at(start + step / 2.0, reach);
        shape.curve_to(x, y, qx, qy);
    }
    shape.close();
    window.fill(&shape, colour);
}

/// A small stable number for anything with an id, so the same person is
/// always drawn the same.
pub fn seed_of(text: &str) -> u32 {
    text.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

const CLOTHES: [u32; 10] = [
    0x3f6fb0, 0xc8553d, 0x3c9a8f, 0xe8b33c, 0x7b4bb3, 0x2f7f86, 0xd9772b, 0x5b8c3a, 0xe06f8b,
    0x2e4a7a,
];
const HAIR: [u32; 6] = [0x2b1d14, 0x5a3b22, 0x8a4b2a, 0xd8b25a, 0x1a1414, 0x9a9a9a];
const SKIN: [u32; 6] = [0xf2cfae, 0xe0b18a, 0xc68a5f, 0xa86b45, 0x8d5a3b, 0xf0c49c];

/// How someone is drawn: a Pack's hints, and for anything it left out a
/// choice made from who they are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Figure {
    pub clothes: Hsla,
    pub hair: Hsla,
    pub skin: Hsla,
    pub carries: Option<Carry>,
    pub bird: bool,
    /// How old they look.
    pub age: crate::age::Age,
    /// Whether their back has bent with the years.
    pub stoop: bool,
}

impl Figure {
    pub fn of(key: &str, look: Option<Look>) -> Self {
        let seed = seed_of(key);
        let look = look.unwrap_or_default();
        let pick = |list: &[u32], salt: u32| list[((seed >> salt) as usize) % list.len()];
        let hair = hex(look.hair.unwrap_or_else(|| pick(&HAIR, 7)));
        Self {
            clothes: hex(look.clothes.unwrap_or_else(|| pick(&CLOTHES, 0))),
            hair: if crate::age::look_grey(&look) {
                crate::age::greyed(hair)
            } else {
                hair
            },
            skin: hex(look.skin.unwrap_or_else(|| pick(&SKIN, 13))),
            carries: look.carries,
            bird: look.bird,
            age: crate::age::Age::of(&look),
            stoop: crate::age::look_stoop(&look),
        }
    }
}

/// How a figure stands this frame: the swing of a walk (none when still),
/// a small rise and fall while it breathes, and which way it faces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub stride: Option<f32>,
    pub bob: f32,
    pub facing: f32,
    /// Taller than they are (above 1) or squashed (below 1), keeping
    /// their volume: stretched in a hop, squashed as they land.
    pub squash: f32,
    /// How far the top of them trails or leads the feet, as a share of
    /// their height: coats and hair swinging as they start and stop.
    pub lean: f32,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            stride: None,
            bob: 0.0,
            facing: 0.0,
            squash: 1.0,
            lean: 0.0,
        }
    }
}

impl Pose {
    /// Whether the figure is drawn as it stands, unsquashed and upright.
    fn plain(&self) -> bool {
        (self.squash - 1.0).abs() < 1e-3 && self.lean.abs() < 1e-3
    }
}

/// Where a foot is in a walk `phase` (0 to 1) along, for the foot on
/// `side`: how far forward of the hip (-1 behind to 1 ahead) and how high
/// it is lifted (0 to 1). A foot on the ground stays where it was put while
/// the body goes on over it; then it lifts and swings through to its next
/// step.
pub fn step(phase: f32, side: f32) -> (f32, f32) {
    let p = (phase + if side > 0.0 { 0.0 } else { 0.5 }).rem_euclid(1.0);
    if p < 0.5 {
        // Planted: it slides back evenly under a body moving on evenly.
        (1.0 - 4.0 * p, 0.0)
    } else {
        let swing = (p - 0.5) * 2.0;
        (
            2.0 * ease_in_out(swing) - 1.0,
            (swing * std::f32::consts::PI).sin(),
        )
    }
}

fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Someone standing with their feet at (`x`, `y`), `height` tall.
pub fn paint_figure(
    window: &mut dyn Brush,
    x: f32,
    y: f32,
    height: f32,
    figure: &Figure,
    pose: Pose,
) {
    if !pose.plain() {
        let mut posed = Xform::about(
            window,
            (x, y),
            1.0 / pose.squash.max(0.2).sqrt(),
            pose.squash,
            pose.lean,
        );
        let upright = Pose {
            squash: 1.0,
            lean: 0.0,
            ..pose
        };
        paint_figure(&mut posed, x, y, height, figure, upright);
        return;
    }
    let u = height / 10.0;
    if figure.bird {
        paint_bird(window, x, y, u, figure, pose);
        return;
    }
    let swing = pose
        .stride
        .map(|phase| (phase * std::f32::consts::TAU).sin())
        .unwrap_or(0.0);
    let lift = pose.bob;
    let trousers = hex(0x3b3f4a);
    let facing = if pose.facing < 0.0 { -1.0 } else { 1.0 };
    // Legs: in a walk each foot is planted while the body passes over it,
    // then lifts and swings through.
    for side in [-1.0_f32, 1.0] {
        let (reach, raised) = pose.stride.map_or((0.0, 0.0), |phase| step(phase, side));
        let along = reach * facing * 0.8 * u;
        let up = raised * 0.45 * u;
        rect(
            window,
            x + side * 0.75 * u - 0.5 * u + along,
            y - 3.2 * u - up,
            1.0 * u,
            3.2 * u,
            0.4 * u,
            trousers,
        );
        rect(
            window,
            x + side * 0.75 * u - 0.6 * u + along,
            y - 0.45 * u - up,
            1.3 * u,
            0.5 * u,
            0.25 * u,
            hex(0x2a2522),
        );
    }
    let top = y - 6.6 * u - lift;
    // Arms, behind the body, swinging against the legs.
    let sleeve = shade(figure.clothes, -0.18);
    for side in [-1.0_f32, 1.0] {
        let reach = -swing * side * 0.5 * u;
        rect(
            window,
            x + side * 1.75 * u - 0.45 * u + reach,
            top + 0.5 * u,
            0.9 * u,
            3.0 * u,
            0.45 * u,
            sleeve,
        );
        circle(
            window,
            x + side * 1.75 * u + reach,
            top + 3.6 * u,
            0.42 * u,
            figure.skin,
        );
    }
    // Body: a coat that widens a little toward the hem.
    polygon(
        window,
        &[
            (x - 1.45 * u, top + 0.3 * u),
            (x + 1.45 * u, top + 0.3 * u),
            (x + 1.75 * u, top + 3.6 * u),
            (x - 1.75 * u, top + 3.6 * u),
        ],
        figure.clothes,
    );
    rect(
        window,
        x - 1.45 * u,
        top,
        2.9 * u,
        1.0 * u,
        0.5 * u,
        figure.clothes,
    );
    // Head, hair, eyes.
    let head_y = top - 1.45 * u;
    circle(window, x, head_y, 1.55 * u, figure.skin);
    ellipse(
        window,
        x - pose.facing * 0.2 * u,
        head_y - 0.75 * u,
        1.6 * u,
        0.95 * u,
        figure.hair,
    );
    rect(
        window,
        x - 1.6 * u,
        head_y - 0.8 * u,
        0.55 * u,
        1.3 * u,
        0.25 * u,
        figure.hair,
    );
    rect(
        window,
        x + 1.05 * u,
        head_y - 0.8 * u,
        0.55 * u,
        1.3 * u,
        0.25 * u,
        figure.hair,
    );
    let eye = hex(0x2a2522);
    for side in [-1.0_f32, 1.0] {
        circle(
            window,
            x + side * 0.55 * u + pose.facing * 0.3 * u,
            head_y + 0.15 * u,
            0.17 * u,
            eye,
        );
    }
    if let Some(carry) = figure.carries {
        paint_carry(window, x + 2.0 * u, top + 3.4 * u, u, carry);
    }
}

fn paint_bird(window: &mut dyn Brush, x: f32, y: f32, u: f32, figure: &Figure, pose: Pose) {
    let waddle = pose
        .stride
        .map(|phase| (phase * std::f32::consts::TAU).sin() * 0.35 * u)
        .unwrap_or(0.0);
    let x = x + waddle;
    let black = hex(0x23262d);
    let orange = hex(0xf0a030);
    for side in [-1.0_f32, 1.0] {
        ellipse(
            window,
            x + side * 0.8 * u,
            y - 0.25 * u,
            0.75 * u,
            0.3 * u,
            orange,
        );
    }
    ellipse(window, x, y - 3.6 * u - pose.bob, 2.3 * u, 3.4 * u, black);
    ellipse(
        window,
        x + pose.facing * 0.3 * u,
        y - 3.1 * u - pose.bob,
        1.6 * u,
        2.6 * u,
        gpui::white(),
    );
    // Flippers.
    for side in [-1.0_f32, 1.0] {
        ellipse(
            window,
            x + side * 2.25 * u,
            y - 3.6 * u - pose.bob,
            0.55 * u,
            1.6 * u,
            black,
        );
    }
    let head_y = y - 7.0 * u - pose.bob;
    circle(window, x, head_y, 1.7 * u, black);
    for side in [-1.0_f32, 1.0] {
        circle(
            window,
            x + side * 0.6 * u + pose.facing * 0.3 * u,
            head_y - 0.1 * u,
            0.32 * u,
            gpui::white(),
        );
        circle(
            window,
            x + side * 0.6 * u + pose.facing * 0.4 * u,
            head_y - 0.1 * u,
            0.15 * u,
            black,
        );
    }
    let beak = x + pose.facing * 0.2 * u;
    polygon(
        window,
        &[
            (beak - 0.5 * u, head_y + 0.55 * u),
            (beak + 0.5 * u, head_y + 0.55 * u),
            (beak, head_y + 1.3 * u),
        ],
        orange,
    );
    // A scarf in their colour.
    rect(
        window,
        x - 1.9 * u,
        head_y + 1.3 * u,
        3.8 * u,
        0.8 * u,
        0.4 * u,
        figure.clothes,
    );
    rect(
        window,
        x + 0.8 * u,
        head_y + 1.6 * u,
        0.8 * u,
        1.8 * u,
        0.3 * u,
        figure.clothes,
    );
    if let Some(carry) = figure.carries {
        paint_carry(window, x + 2.6 * u, y - 3.0 * u - pose.bob, u, carry);
    }
}

/// What someone carries, held at (`x`, `y`).
fn paint_carry(window: &mut dyn Brush, x: f32, y: f32, u: f32, carry: Carry) {
    match carry {
        Carry::Tool => {
            rect(
                window,
                x - 0.2 * u,
                y - 1.6 * u,
                0.4 * u,
                2.2 * u,
                0.15 * u,
                hex(0x8a8f99),
            );
            rect(
                window,
                x - 0.6 * u,
                y - 1.9 * u,
                1.2 * u,
                0.6 * u,
                0.2 * u,
                hex(0x5f646d),
            );
        }
        Carry::Book => rect(
            window,
            x - 0.6 * u,
            y - 1.2 * u,
            1.2 * u,
            1.6 * u,
            0.15 * u,
            hex(0xb8433b),
        ),
        Carry::Bread => ellipse(window, x, y - 0.5 * u, 1.1 * u, 0.55 * u, hex(0xd39a52)),
        Carry::Fish => {
            ellipse(window, x, y, 1.0 * u, 0.45 * u, hex(0x7f9cb3));
            polygon(
                window,
                &[
                    (x + 0.8 * u, y),
                    (x + 1.5 * u, y - 0.5 * u),
                    (x + 1.5 * u, y + 0.5 * u),
                ],
                hex(0x7f9cb3),
            );
        }
        Carry::Basket => {
            ellipse(window, x, y - 0.1 * u, 1.0 * u, 0.7 * u, hex(0xa9773f));
            rect(
                window,
                x - 1.0 * u,
                y - 0.9 * u,
                2.0 * u,
                0.7 * u,
                0.2 * u,
                hex(0xc7a35b),
            );
        }
        Carry::Satchel => rect(
            window,
            x - 0.8 * u,
            y - 0.6 * u,
            1.6 * u,
            1.3 * u,
            0.3 * u,
            hex(0x8a5a33),
        ),
        Carry::Plant => {
            rect(
                window,
                x - 0.5 * u,
                y - 0.2 * u,
                1.0 * u,
                0.9 * u,
                0.2 * u,
                hex(0xb5523b),
            );
            circle(window, x, y - 0.8 * u, 0.8 * u, hex(0x4f9a5a));
        }
        Carry::Mug => {
            rect(
                window,
                x - 0.5 * u,
                y - 1.0 * u,
                1.0 * u,
                1.2 * u,
                0.2 * u,
                hex(0xf3efe6),
            );
            rect(
                window,
                x + 0.4 * u,
                y - 0.8 * u,
                0.4 * u,
                0.6 * u,
                0.2 * u,
                hex(0xd8d2c4),
            );
        }
    }
}

/// A head-and-shoulders portrait of someone inside `bounds`, for a card or
/// a return beat: the same person as on the scene.
pub fn paint_portrait(window: &mut dyn Brush, bounds: Bounds<gpui::Pixels>, figure: &Figure) {
    let x = f32::from(bounds.origin.x);
    let y = f32::from(bounds.origin.y);
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    let backdrop = shade(figure.clothes, 0.78);
    rect(window, x, y, w, h, w.min(h) * 0.24, backdrop);
    // The figure drawn large enough that its head fills the upper half,
    // standing below the frame so only head and shoulders show.
    crate::age::paint_bust(
        window,
        (x, y, w, h),
        h * 1.7,
        y + h * 1.62,
        &Figure {
            carries: None,
            ..*figure
        },
        None,
        world_projection::Stance::Standing,
        world_projection::Mood::Content,
    );
}

/// Someone's head and shoulders as the scene draws them: their Pack's own
/// drawing when it ships one, with the face of their mood, else the
/// figure the app draws for them. `talking` opens their mouth.
pub fn paint_likeness(
    window: &mut dyn Brush,
    bounds: Bounds<gpui::Pixels>,
    figure: &Figure,
    drawing: Option<&world_projection::Drawing>,
    mood: world_projection::Mood,
    talking: bool,
) {
    let Some(drawing) = drawing else {
        paint_portrait(window, bounds, figure);
        return;
    };
    let x = f32::from(bounds.origin.x);
    let y = f32::from(bounds.origin.y);
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    rect(
        window,
        x,
        y,
        w,
        h,
        w.min(h) * 0.24,
        shade(figure.clothes, 0.78),
    );
    // Drawn large enough that the head fills the upper half and the
    // shoulders the lower, standing below the frame.
    let height = h * 2.3;
    let base = y + h * 0.1 + height;
    let stance = if talking {
        world_projection::Stance::Talking
    } else {
        world_projection::Stance::Standing
    };
    crate::age::paint_bust(
        window,
        (x, y, w, h),
        height,
        base,
        figure,
        Some(drawing),
        stance,
        mood,
    );
}

/// Colours a building is painted in.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub wall: Hsla,
    pub roof: Hsla,
    pub trim: Hsla,
    /// Windows by day, and lit after dusk.
    pub glass: Hsla,
    /// What a home is like besides its colours: its storeys, roof, door,
    /// chimney and what is built on, all from its id.
    pub seed: u32,
    /// What it is drawn as, from the library of drawings, when its Pack
    /// says; otherwise its shape is drawn in its setting's own way.
    pub art: Option<crate::works::Art>,
    /// The kind of place it stands in.
    pub setting: Setting,
}

const ROOFS: [u32; 5] = [0xb5523b, 0x3f6a8a, 0x4a7a4f, 0x7a4b8a, 0x8a6a3a];
const WALLS: [u32; 4] = [0xefe3cf, 0xe6d6c2, 0xf2ead9, 0xdcd3c6];

impl Palette {
    pub fn of(key: &str, lit: bool) -> Self {
        let seed = seed_of(key);
        Self {
            wall: hex(WALLS[(seed as usize) % WALLS.len()]),
            roof: hex(ROOFS[((seed >> 5) as usize) % ROOFS.len()]),
            trim: hex(0x6b4a33),
            glass: if lit { hex(0xffd27a) } else { hex(0x5f7385) },
            seed: seed.wrapping_mul(0x9e37_79b9) ^ (seed >> 15),
            art: None,
            setting: Setting::Harbour,
        }
    }

    /// The colours of something standing in `setting`: its walls and
    /// roofs from that place's own few paints.
    pub fn of_in(key: &str, lit: bool, setting: Setting) -> Self {
        let mut palette = Self::of(key, lit);
        let seed = seed_of(key);
        let (walls, roofs, trim) = setting.paints();
        palette.wall = hex(walls[(seed as usize) % walls.len()]);
        palette.roof = hex(roofs[((seed >> 5) as usize) % roofs.len()]);
        palette.trim = hex(trim);
        palette.setting = setting;
        palette
    }

    /// Drawn as `art` from the library, if the app has it.
    pub fn drawn_as(mut self, art: Option<&str>) -> Self {
        self.art = art.and_then(crate::works::Art::from_key);
        self
    }
}

/// The kind of place a World is, to look at: what its ground, sky,
/// weather and props are, and how anything without a drawing of its own
/// is drawn there.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Setting {
    /// A green shore on the sea: the look of a World that says nothing.
    #[default]
    Harbour,
    /// Regolith under a butterscotch sky, domes and hab modules.
    Mars,
    /// A town street: storefronts, parked cars, wires and streetlamps.
    Street,
    /// Snow, ice shelves and sea ice.
    Ice,
}

impl Setting {
    pub const ALL: [Setting; 4] = [
        Setting::Harbour,
        Setting::Mars,
        Setting::Street,
        Setting::Ice,
    ];

    /// The setting a Pack names; one this app does not know is the plain
    /// harbour.
    pub fn from_key(key: Option<&str>) -> Self {
        match key {
            Some("mars") => Setting::Mars,
            Some("street") => Setting::Street,
            Some("ice") => Setting::Ice,
            _ => Setting::Harbour,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Setting::Harbour => "harbour",
            Setting::Mars => "mars",
            Setting::Street => "street",
            Setting::Ice => "ice",
        }
    }

    /// Whether its front is water: the harbour's sea and the sea beside the
    /// ice are; Mars's regolith and a street's road are not. `None` leaves
    /// it to the scenery's colours, for a World that says nothing.
    pub fn water(self) -> Option<bool> {
        match self {
            Setting::Harbour => None,
            Setting::Ice => Some(true),
            Setting::Mars | Setting::Street => Some(false),
        }
    }

    /// Whether gulls wheel over it.
    pub fn has_gulls(self) -> bool {
        self == Setting::Harbour
    }

    /// Whether chimneys smoke in it: only the harbour's cottages have them.
    pub fn smokes(self) -> bool {
        self == Setting::Harbour
    }

    /// The place's inks: the few colours everything painted there leans
    /// toward (see [`crate::hand`]), so its buildings, whatever drawing
    /// each comes from, read as one limited palette. Its own walls, roofs
    /// and trim, a deep shade to draw with, and a colour or two of the
    /// place itself.
    pub fn inks(self) -> &'static [u32] {
        match self {
            // White, cream and whitewash, brick, harbour blue, sea green,
            // ochre, umber, and a blue-black ink.
            Setting::Harbour => &[
                0xfbf9f4, 0xefe3cf, 0xdcd3c6, 0xb5523b, 0x3f6a8a, 0x4a7a4f, 0x8a6a3a, 0x6b4a33,
                0x2f3a45, 0xd9b45a,
            ],
            // White, panel white, rust, slate, teal, ochre, oxide, graphite
            // and the pink of the dust.
            Setting::Mars => &[
                0xfaf8f4, 0xece6da, 0xc8643a, 0x5d6470, 0x3f8f8a, 0xd9a441, 0x8a4a35, 0x4a4f5a,
                0xe2b39a,
            ],
            // White, brick, cream, clapboard, the awnings' teal, pink,
            // mustard, blue and green, and a violet-black.
            Setting::Street => &[
                0xfbf9f4, 0xa8553f, 0xe8dcc4, 0xc9b79a, 0x7f9aa0, 0x2bb3b1, 0xe0457b, 0xf2c14e,
                0x3a6ea5, 0x2f6b4f, 0x2c2a3a,
            ],
            // Snow, pale ice, ice blue, slate, kelp, a warm orange and a
            // berry red.
            Setting::Ice => &[
                0xf4f8fb, 0xdcebf3, 0x7fb8d9, 0x2d3a4a, 0x5b8a8f, 0xe8963a, 0xb03a48, 0x8a9aa8,
            ],
        }
    }

    /// The place's palette (the art bible's §4): five base colours and two
    /// accent hues. The bases are what the place is made of; the accents
    /// are for the few marks the eye should find, at most three a screen.
    pub fn place_paints(self) -> PlacePaints {
        match self {
            // Sea-green, sand, slate blue, warm stone and chalk; the
            // harbour's red and the lamp's gold.
            Setting::Harbour => PlacePaints {
                base: [0x6f9a86, 0xd9c7a0, 0x5d7590, 0xc9b49a, 0xe6dccb],
                accents: [0xb5523b, 0xe0a83a],
            },
            // Rust, dust, bone, oxide and smoke; teal and signal white.
            Setting::Mars => PlacePaints {
                base: [0xb5603c, 0xd9a27a, 0xe9e1d2, 0x7a3a26, 0x8a8078],
                accents: [0x2f8f8a, 0xf6f4ee],
            },
            // Brick, asphalt, lawn, cream and sky grey; teal and magenta.
            Setting::Street => PlacePaints {
                base: [0xa8553e, 0x4a4c52, 0x6f9a5a, 0xe2d6bc, 0x9aa6b4],
                accents: [0x2bb3b1, 0xe0457b],
            },
            // Snow, ice blue, slate, deep sea and pebble; fish orange and
            // lantern gold.
            Setting::Ice => PlacePaints {
                base: [0xe8eef3, 0xa8cce0, 0x5a6878, 0x2a4a68, 0x8a8a84],
                accents: [0xe8803a, 0xe8c050],
            },
        }
    }

    /// The few paints its buildings take their walls and roofs from, and
    /// the colour of their trim: a limited palette per place.
    pub fn paints(self) -> (&'static [u32], &'static [u32], u32) {
        match self {
            Setting::Harbour => (&WALLS, &ROOFS, 0x6b4a33),
            // Pale composite panels; burnt orange, slate and teal bands.
            Setting::Mars => (
                &[0xece6da, 0xe4ddd0, 0xf1ece3],
                &[0xc8643a, 0x5d6470, 0x3f8f8a, 0xd9a441],
                0x4a4f5a,
            ),
            // Brick, cream and painted clapboard; awning colours of 1987.
            Setting::Street => (
                &[0xa8553f, 0xe8dcc4, 0x8f4a3a, 0xc9b79a, 0x7f9aa0],
                &[0x2bb3b1, 0xe0457b, 0xf2c14e, 0x3a6ea5, 0x2f6b4f],
                0x2c2a3a,
            ),
            // Packed snow and blue ice; kelp, slate and a warm orange.
            Setting::Ice => (
                &[0xf4f8fb, 0xe8f1f6, 0xdcebf3],
                &[0x7fb8d9, 0x2d3a4a, 0xe8963a, 0x5b8a8f],
                0x2d3a4a,
            ),
        }
    }
}

/// A place's palette: five base colours and two accent hues, as
/// `0xRRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacePaints {
    pub base: [u32; 5],
    pub accents: [u32; 2],
}

impl PlacePaints {
    /// The base colour a muted accent, or a design shown by day, leans
    /// toward: the place's stone, its most neutral base.
    pub fn neutral(&self) -> u32 {
        *self
            .base
            .iter()
            .min_by(|a, b| hex(**a).s.total_cmp(&hex(**b).s))
            .unwrap_or(&self.base[0])
    }
}

/// Doors a home might have: its trim's brown, a deep green, a muted blue,
/// oxblood. Few, and quiet, so a street of them holds together.
const DOORS: [u32; 4] = [0x6b4a33, 0x3f5a48, 0x3e5873, 0x7a3b33];

/// A home: the harbour's family of cottages, each its own. From the seed
/// in its palette it takes a width, one or two storeys, a gabled or a
/// hipped roof and its pitch, a chimney at one end, a door colour, window
/// boxes or shutters, and perhaps a lean-to, a porch or a garden gate.
/// A home's make, from the seed in its palette.
struct House {
    storeys: u32,
    hipped: bool,
    pitch: f32,
    chimney_side: f32,
    door: u32,
    dressing: u32,
    annex: u32,
    gate: bool,
    width: f32,
}

impl House {
    fn of(palette: &Palette) -> Self {
        let seed = palette.seed;
        let pick =
            |salt: u32, choices: u32| (seed.rotate_right(salt) ^ (seed >> (salt % 13))) % choices;
        let storeys = if pick(3, 3) == 0 { 2 } else { 1 };
        Self {
            storeys,
            hipped: pick(6, 3) == 0,
            pitch: if storeys == 2 {
                0.3
            } else {
                [0.36, 0.42, 0.48][pick(9, 3) as usize]
            },
            chimney_side: if pick(12, 2) == 0 { -1.0 } else { 1.0 },
            door: DOORS[pick(15, DOORS.len() as u32) as usize],
            // 0 none, 1 window boxes, 2 shutters.
            dressing: pick(18, 3),
            // 0 and 1 nothing; 2 a lean-to; 3 a porch.
            annex: pick(21, 4),
            gate: pick(24, 4) == 0,
            width: [0.8, 0.86, 0.92][pick(27, 3) as usize],
        }
    }

    /// The body's middle and width, and a lean-to's width, for a house `w`
    /// wide centred on `x`.
    fn body(&self, x: f32, w: f32) -> (f32, f32, f32) {
        let lean_w = if self.annex == 2 { w * 0.22 } else { 0.0 };
        let body_w = (w * self.width - lean_w).max(w * 0.6);
        (x + self.chimney_side * lean_w / 2.0, body_w, lean_w)
    }
}

/// Where a home's chimney pot is, for its smoke.
pub fn chimney_top(x: f32, base: f32, w: f32, h: f32, palette: &Palette) -> (f32, f32) {
    let house = House::of(palette);
    let (body_x, body_w, _) = house.body(x, w);
    (
        body_x + house.chimney_side * body_w * 0.24,
        base - h + h * house.pitch * 0.1,
    )
}

/// Where a home's first window is (left, top, width, height), for a house
/// `w` by `h` standing on (`x`, `base`): where a quilt is seen by
/// lamplight, or aired over the sill.
pub fn first_window(x: f32, base: f32, w: f32, h: f32, palette: &Palette) -> (f32, f32, f32, f32) {
    let house = House::of(palette);
    let (body_x, body_w, _) = house.body(x, w);
    let wall_top = base - h + h * house.pitch;
    let window_w = w * 0.13;
    let (cx, top, window_h) = if house.storeys == 2 {
        (
            body_x - body_w * 0.24,
            wall_top + (base - wall_top) * 0.12,
            h * 0.11,
        )
    } else {
        (
            body_x - body_w * 0.28,
            wall_top + (base - wall_top) * 0.2,
            h * 0.14,
        )
    };
    (cx - window_w / 2.0, top, window_w, window_h)
}

fn paint_house(window: &mut dyn Brush, x: f32, base: f32, w: f32, h: f32, palette: &Palette) {
    let house = House::of(palette);
    let (storeys, hipped, pitch, chimney_side) =
        (house.storeys, house.hipped, house.pitch, house.chimney_side);
    let door_ink = hex(house.door);
    let (dressing, annex, gate) = (house.dressing, house.annex, house.gate);
    let (body_x, body_w, lean_w) = house.body(x, w);
    let (left, right) = (body_x - body_w / 2.0, body_x + body_w / 2.0);
    let top = base - h;
    let roof_h = h * pitch;
    let wall_top = top + roof_h;
    if annex == 2 {
        let lean_left = if chimney_side > 0.0 {
            left - lean_w
        } else {
            right
        };
        let lean_top = base - (base - wall_top) * 0.62;
        rect(
            window,
            lean_left,
            lean_top,
            lean_w,
            base - lean_top,
            1.0,
            shade(palette.wall, -0.06),
        );
        let (inner, outer) = if chimney_side > 0.0 {
            (left, lean_left)
        } else {
            (right, lean_left + lean_w)
        };
        polygon(
            window,
            &[
                (inner, lean_top - h * 0.08),
                (outer - (inner - outer).signum() * w * 0.02, lean_top + 1.0),
                (outer, lean_top + h * 0.02),
                (inner, lean_top + h * 0.02),
            ],
            shade(palette.roof, -0.08),
        );
        rect(
            window,
            lean_left + lean_w * 0.3,
            lean_top + (base - lean_top) * 0.3,
            lean_w * 0.4,
            (base - lean_top) * 0.3,
            1.0,
            palette.glass,
        );
    }
    rect(
        window,
        left,
        wall_top,
        body_w,
        base - wall_top,
        2.0,
        palette.wall,
    );
    // The chimney, behind the roof's slope at one end.
    let chimney_x = body_x + chimney_side * body_w * 0.24;
    rect(
        window,
        chimney_x - w * 0.05,
        top + roof_h * 0.1,
        w * 0.1,
        roof_h * 0.7,
        1.0,
        shade(palette.roof, -0.2),
    );
    let eave = w * 0.035;
    if hipped {
        polygon(
            window,
            &[
                (left - eave, wall_top + 2.0),
                (body_x - body_w * 0.2, top + roof_h * 0.12),
                (body_x + body_w * 0.2, top + roof_h * 0.12),
                (right + eave, wall_top + 2.0),
            ],
            palette.roof,
        );
    } else {
        polygon(
            window,
            &[
                (left - eave, wall_top + 2.0),
                (body_x, top),
                (right + eave, wall_top + 2.0),
            ],
            palette.roof,
        );
    }
    // The eave's shadow on the wall.
    rect(
        window,
        left,
        wall_top + 1.0,
        body_w,
        h * 0.03,
        0.0,
        shade(palette.wall, -0.18),
    );
    // Windows, and the door.
    let window_w = w * 0.13;
    let window_h = if storeys == 2 { h * 0.11 } else { h * 0.14 };
    let door_h = h * 0.26;
    let door_x = if storeys == 2 {
        body_x - chimney_side * body_w * 0.22
    } else {
        body_x
    };
    let mut panes = Vec::new();
    if storeys == 2 {
        let upper = wall_top + (base - wall_top) * 0.12;
        panes.push((body_x - body_w * 0.24, upper));
        panes.push((body_x + body_w * 0.24, upper));
        panes.push((body_x + chimney_side * body_w * 0.22, base - door_h * 0.95));
    } else {
        let row = wall_top + (base - wall_top) * 0.2;
        panes.push((body_x - body_w * 0.28, row));
        panes.push((body_x + body_w * 0.28, row));
    }
    for (cx, wy) in &panes {
        let wx = cx - window_w / 2.0;
        if dressing == 2 {
            for side in [-1.0_f32, 1.0] {
                rect(
                    window,
                    cx + side * (window_w / 2.0 + w * 0.025) - w * 0.02,
                    *wy,
                    w * 0.04,
                    window_h,
                    1.0,
                    shade(door_ink, 0.12),
                );
            }
        }
        rect(window, wx, *wy, window_w, window_h, 2.0, palette.glass);
        if dressing == 1 {
            rect(
                window,
                wx - w * 0.01,
                wy + window_h,
                window_w + w * 0.02,
                h * 0.035,
                1.0,
                hex(0x7a5a3a),
            );
            for bloom in 0..3 {
                circle(
                    window,
                    wx + window_w * (0.2 + 0.3 * bloom as f32),
                    wy + window_h,
                    w * 0.018,
                    hex(if bloom % 2 == 0 { 0x5f8f4f } else { 0xd46a6a }),
                );
            }
        }
    }
    rect(
        window,
        door_x - w * 0.07,
        base - door_h,
        w * 0.14,
        door_h,
        w * 0.03,
        door_ink,
    );
    if annex == 3 {
        // A porch: a little gabled canopy on two posts over the door.
        let porch_top = base - door_h - h * 0.1;
        polygon(
            window,
            &[
                (door_x - w * 0.13, porch_top + h * 0.08),
                (door_x, porch_top),
                (door_x + w * 0.13, porch_top + h * 0.08),
            ],
            shade(palette.roof, -0.05),
        );
        for side in [-1.0_f32, 1.0] {
            rect(
                window,
                door_x + side * w * 0.11 - w * 0.01,
                porch_top + h * 0.08,
                w * 0.02,
                base - porch_top - h * 0.08,
                0.0,
                shade(palette.wall, 0.2),
            );
        }
    }
    if gate {
        // A garden gate and a little paling beside the house.
        let side = -chimney_side;
        let from = body_x + side * (body_w / 2.0 + w * 0.02);
        let paling = hex(0xf1ece2);
        for post in 0..3 {
            let px0 = from + side * post as f32 * w * 0.05;
            rect(
                window,
                px0 - w * 0.008,
                base - h * 0.1,
                w * 0.016,
                h * 0.1,
                0.5,
                paling,
            );
        }
        rect(
            window,
            from.min(from + side * w * 0.1),
            base - h * 0.07,
            w * 0.1,
            h * 0.012,
            0.0,
            paling,
        );
    }
}

/// A place drawn as the building it is, standing with its base centred on
/// (`x`, `base`), `w` wide and `h` tall.
pub fn paint_building(
    window: &mut dyn Brush,
    x: f32,
    base: f32,
    w: f32,
    h: f32,
    shape: MarkShape,
    palette: &Palette,
) {
    if crate::works::paint(window, x, base, w, h, shape, palette) {
        return;
    }
    let left = x - w / 2.0;
    let top = base - h;
    let door = |window: &mut dyn Brush, height: f32| {
        rect(
            window,
            x - w * 0.07,
            base - height,
            w * 0.14,
            height,
            w * 0.03,
            palette.trim,
        );
    };
    match shape {
        MarkShape::House => paint_house(window, x, base, w, h, palette),
        MarkShape::Lamp => {
            let wall_top = top + h * 0.42;
            rect(
                window,
                left + w * 0.08,
                wall_top,
                w * 0.84,
                base - wall_top,
                2.0,
                palette.wall,
            );
            polygon(
                window,
                &[(left, wall_top + 2.0), (x, top), (left + w, wall_top + 2.0)],
                palette.roof,
            );
            rect(
                window,
                left + w * 0.62,
                top + h * 0.12,
                w * 0.1,
                h * 0.22,
                1.0,
                shade(palette.roof, -0.2),
            );
            for column in [0.22, 0.66] {
                rect(
                    window,
                    left + w * column,
                    wall_top + h * 0.12,
                    w * 0.14,
                    h * 0.14,
                    2.0,
                    palette.glass,
                );
            }
            door(window, h * 0.26);
        }
        MarkShape::Shop => {
            let wall_top = top + h * 0.22;
            rect(
                window,
                left + w * 0.06,
                wall_top,
                w * 0.88,
                base - wall_top,
                2.0,
                palette.wall,
            );
            rect(
                window,
                left + w * 0.02,
                top + h * 0.12,
                w * 0.96,
                h * 0.14,
                2.0,
                palette.roof,
            );
            // A striped awning over the window.
            let stripes = 7;
            let awning_top = wall_top + h * 0.12;
            for index in 0..stripes {
                let colour = if index % 2 == 0 {
                    palette.roof
                } else {
                    gpui::white()
                };
                let sx = left + w * 0.04 + index as f32 * w * 0.92 / stripes as f32;
                polygon(
                    window,
                    &[
                        (sx, awning_top),
                        (sx + w * 0.92 / stripes as f32, awning_top),
                        (
                            sx + w * 0.92 / stripes as f32 + w * 0.02,
                            awning_top + h * 0.16,
                        ),
                        (sx + w * 0.02, awning_top + h * 0.16),
                    ],
                    colour,
                );
            }
            rect(
                window,
                left + w * 0.14,
                awning_top + h * 0.22,
                w * 0.46,
                h * 0.3,
                2.0,
                palette.glass,
            );
            rect(
                window,
                left + w * 0.68,
                base - h * 0.34,
                w * 0.16,
                h * 0.34,
                2.0,
                palette.trim,
            );
        }
        MarkShape::Dome => {
            let dome_colour = shade(palette.wall, 0.35);
            rect(
                window,
                left + w * 0.02,
                base - h * 0.16,
                w * 0.96,
                h * 0.16,
                3.0,
                shade(palette.wall, -0.1),
            );
            dome(window, x, base - h * 0.16, w * 0.48, h * 0.8, dome_colour);
            rect(
                window,
                left,
                base - h * 0.2,
                w,
                h * 0.22,
                2.0,
                shade(palette.wall, -0.08),
            );
            let rib = shade(palette.wall, -0.18);
            for offset in [-0.28_f32, 0.0, 0.28] {
                line(
                    window,
                    (x + w * offset, base - h * 0.2),
                    (x + w * offset * 0.5, top + h * 0.1),
                    1.5,
                    rib,
                );
            }
            rect(
                window,
                x - w * 0.12,
                base - h * 0.2 - h * 0.18,
                w * 0.24,
                h * 0.14,
                3.0,
                palette.glass,
            );
            door(window, h * 0.2);
        }
        MarkShape::Tree => {
            // A greenhouse: a glass dome with plants inside.
            let glass = hex(0xcfe8e0).opacity(0.85);
            dome(window, x, base, w * 0.48, h * 0.9, glass);
            for (dx, dy, r) in [
                (-0.2, 0.3, 0.14),
                (0.05, 0.42, 0.18),
                (0.22, 0.28, 0.13),
                (-0.02, 0.2, 0.12),
            ] {
                circle(window, x + w * dx, base - h * dy, w * r, hex(0x4f9a5a));
            }
            rect(
                window,
                left + w * 0.02,
                base - h * 0.08,
                w * 0.96,
                h * 0.08,
                2.0,
                shade(palette.wall, -0.15),
            );
            let frame = hex(0xffffff).opacity(0.7);
            line(window, (x, base - h * 0.9), (x, base), 1.5, frame);
            line(
                window,
                (left + w * 0.1, base - h * 0.5),
                (left + w * 0.9, base - h * 0.5),
                1.5,
                frame,
            );
        }
        MarkShape::Tower => {
            // A lighthouse: a white tower banded in red, lamp at the top.
            let tower_top = top + h * 0.18;
            polygon(
                window,
                &[
                    (x - w * 0.16, base),
                    (x - w * 0.11, tower_top),
                    (x + w * 0.11, tower_top),
                    (x + w * 0.16, base),
                ],
                gpui::white(),
            );
            for band in [0.3, 0.62] {
                let by = tower_top + (base - tower_top) * band;
                let half = w * (0.11 + 0.05 * band);
                rect(window, x - half, by, half * 2.0, h * 0.1, 0.0, palette.roof);
            }
            rect(
                window,
                x - w * 0.13,
                tower_top - h * 0.1,
                w * 0.26,
                h * 0.11,
                2.0,
                palette.glass,
            );
            polygon(
                window,
                &[
                    (x - w * 0.16, tower_top - h * 0.1),
                    (x, top),
                    (x + w * 0.16, tower_top - h * 0.1),
                ],
                palette.roof,
            );
            rect(
                window,
                left + w * 0.2,
                base - h * 0.14,
                w * 0.6,
                h * 0.14,
                2.0,
                palette.wall,
            );
            door(window, h * 0.12);
        }
        MarkShape::Bridge => {
            let deck = base - h * 0.45;
            let stone = shade(palette.wall, -0.12);
            rect(window, left, deck, w, h * 0.1, 2.0, stone);
            for side in [0.0_f32, 1.0] {
                rect(
                    window,
                    left + side * (w - w * 0.12),
                    deck,
                    w * 0.12,
                    base - deck,
                    2.0,
                    stone,
                );
            }
            let mut arch = Shape::new();
            arch.move_to(left + w * 0.12, base)
                .curve_to(left + w * 0.88, base, x, deck - h * 0.25);
            window.stroke(&arch, h * 0.08, stone);
            for post in 0..6 {
                let px_ = left + w * (0.08 + 0.168 * post as f32);
                rect(
                    window,
                    px_,
                    deck - h * 0.14,
                    w * 0.02,
                    h * 0.14,
                    1.0,
                    palette.trim,
                );
            }
        }
        MarkShape::Rover
        | MarkShape::Boat
        | MarkShape::Parcel
        | MarkShape::Stall
        | MarkShape::Bunting
        | MarkShape::Pier
        | MarkShape::Garden
        | MarkShape::Flag
        | MarkShape::Lantern
        | MarkShape::Tent
        | MarkShape::Bench
        | MarkShape::Sprouts
        | MarkShape::Well
        | MarkShape::Swing
        | MarkShape::Fountain
        | MarkShape::Signpost
        | MarkShape::Birdhouse
        | MarkShape::Planter
        | MarkShape::Statue
        | MarkShape::Postbox => {
            paint_thing(window, x, base, w, shape, palette, 0.0);
        }
    }
}

/// A thing drawn standing (or floating) with its base centred on (`x`,
/// `base`), `w` wide: a rover, a boat, a parcel. `sway` rocks a boat.
pub fn paint_thing(
    window: &mut dyn Brush,
    x: f32,
    base: f32,
    w: f32,
    shape: MarkShape,
    palette: &Palette,
    sway: f32,
) {
    if crate::works::paint_thing(window, x, base, w, shape, palette, sway) {
        return;
    }
    match shape {
        MarkShape::Rover => {
            let h = w * 0.45;
            let body = hex(0xece6da);
            ellipse(
                window,
                x,
                base,
                w * 0.55,
                h * 0.1,
                gpui::black().opacity(0.14),
            );
            rect(window, x - w * 0.5, base - h * 0.75, w, h * 0.42, 4.0, body);
            rect(window, x + w * 0.02, base - h, w * 0.4, h * 0.3, 4.0, body);
            rect(
                window,
                x + w * 0.08,
                base - h * 0.95,
                w * 0.28,
                h * 0.18,
                3.0,
                palette.glass,
            );
            rect(
                window,
                x - w * 0.46,
                base - h * 0.52,
                w * 0.92,
                h * 0.07,
                1.0,
                palette.roof,
            );
            line(
                window,
                (x - w * 0.36, base - h * 0.75),
                (x - w * 0.42, base - h * 1.35),
                1.5,
                hex(0x6b6f78),
            );
            circle(
                window,
                x - w * 0.42,
                base - h * 1.38,
                w * 0.025,
                palette.roof,
            );
            for wheel in [-0.36_f32, 0.0, 0.36] {
                circle(
                    window,
                    x + w * wheel,
                    base - h * 0.2,
                    h * 0.2,
                    hex(0x34363c),
                );
                circle(
                    window,
                    x + w * wheel,
                    base - h * 0.2,
                    h * 0.08,
                    hex(0x8a8f99),
                );
            }
        }
        MarkShape::Boat => {
            let h = w * 0.55;
            let tilt = sway * h * 0.05;
            let hull = hex(0x9c3b2e);
            polygon(
                window,
                &[
                    (x - w * 0.5, base - h * 0.36 - tilt),
                    (x + w * 0.5, base - h * 0.36 + tilt),
                    (x + w * 0.36, base),
                    (x - w * 0.36, base),
                ],
                hull,
            );
            rect(
                window,
                x - w * 0.46,
                base - h * 0.42,
                w * 0.92,
                h * 0.07,
                1.0,
                gpui::white(),
            );
            rect(
                window,
                x - w * 0.12,
                base - h * 0.72,
                w * 0.3,
                h * 0.32,
                3.0,
                hex(0xf3efe6),
            );
            rect(
                window,
                x - w * 0.06,
                base - h * 0.65,
                w * 0.14,
                h * 0.12,
                2.0,
                palette.glass,
            );
            line(
                window,
                (x - w * 0.2, base - h * 0.4),
                (x - w * 0.2, base - h * 1.2),
                2.0,
                hex(0x6b4a33),
            );
        }
        MarkShape::Parcel => {
            let h = w * 0.6;
            rect(window, x - w / 2.0, base - h, w, h, 3.0, hex(0xc79a5b));
            rect(
                window,
                x - w * 0.07,
                base - h,
                w * 0.14,
                h,
                0.0,
                hex(0xd64545),
            );
            rect(
                window,
                x - w / 2.0,
                base - h * 0.58,
                w,
                h * 0.14,
                0.0,
                hex(0xd64545),
            );
        }
        MarkShape::Stall => {
            let h = w * 0.8;
            ellipse(
                window,
                x,
                base,
                w * 0.55,
                h * 0.06,
                gpui::black().opacity(0.12),
            );
            // The counter, its goods, two posts and a striped awning.
            rect(
                window,
                x - w * 0.45,
                base - h * 0.42,
                w * 0.9,
                h * 0.42,
                2.0,
                palette.wall,
            );
            rect(
                window,
                x - w * 0.45,
                base - h * 0.46,
                w * 0.9,
                h * 0.06,
                1.0,
                palette.trim,
            );
            for (dx, colour) in [
                (-0.28, 0xe0a33a),
                (-0.08, 0xd6553d),
                (0.12, 0x7fae5a),
                (0.3, 0xe0a33a),
            ] {
                circle(window, x + w * dx, base - h * 0.51, w * 0.06, hex(colour));
            }
            for dx in [-0.42_f32, 0.42] {
                rect(
                    window,
                    x + w * dx - 1.5,
                    base - h,
                    3.0,
                    h * 0.58,
                    0.0,
                    palette.trim,
                );
            }
            for stripe in 0..6 {
                let left = x - w * 0.5 + w * stripe as f32 / 6.0;
                polygon(
                    window,
                    &[
                        (left, base - h * 0.78),
                        (left + w / 6.0, base - h * 0.78),
                        (left + w / 6.0 + w * 0.02, base - h * 0.66),
                        (left + w * 0.02, base - h * 0.66),
                    ],
                    if stripe % 2 == 0 {
                        palette.roof
                    } else {
                        gpui::white()
                    },
                );
            }
            polygon(
                window,
                &[
                    (x - w * 0.5, base - h * 0.78),
                    (x, base - h),
                    (x + w * 0.5, base - h * 0.78),
                ],
                palette.roof,
            );
        }
        MarkShape::Bunting => {
            let h = w * 0.75;
            let (left, right) = (x - w * 0.5, x + w * 0.5);
            for pole in [left, right] {
                rect(window, pole - 1.5, base - h, 3.0, h, 0.0, hex(0x6b4a33));
            }
            let colours = [0xd64545, 0xe0a33a, 0x3f8fd6, 0x7fae5a, 0xe06f8b];
            let flags = 7;
            for flag in 0..flags {
                let t = (flag as f32 + 0.5) / flags as f32;
                let fx = left + (right - left) * t;
                // The string sags in the middle, and sways a little.
                let sag = (t * std::f32::consts::PI).sin() * h * 0.18 + sway * 1.5;
                let top = base - h * 0.95 + sag;
                polygon(
                    window,
                    &[
                        (fx - w * 0.05, top),
                        (fx + w * 0.05, top),
                        (fx, top + w * 0.12),
                    ],
                    hex(colours[flag % colours.len()]),
                );
            }
            line(
                window,
                (left, base - h * 0.95),
                (right, base - h * 0.95),
                1.0,
                hex(0x6b4a33).opacity(0.5),
            );
        }
        MarkShape::Pier => {
            let h = w * 0.35;
            let deck = base - h * 0.55;
            for pile in 0..5 {
                let px0 = x - w * 0.46 + w * 0.92 * pile as f32 / 4.0;
                rect(window, px0 - 2.0, deck, 4.0, h * 0.9, 1.0, hex(0x5a4030));
            }
            rect(
                window,
                x - w * 0.5,
                deck - h * 0.12,
                w,
                h * 0.16,
                1.0,
                hex(0xa8835a),
            );
            for plank in 0..10 {
                let px0 = x - w * 0.5 + w * plank as f32 / 10.0;
                line(
                    window,
                    (px0, deck - h * 0.12),
                    (px0, deck + h * 0.04),
                    1.0,
                    hex(0x7a5a3a),
                );
            }
            // A mooring post at its end.
            rect(
                window,
                x + w * 0.44,
                deck - h * 0.45,
                4.0,
                h * 0.35,
                1.0,
                hex(0x5a4030),
            );
        }
        MarkShape::Garden => {
            let h = w * 0.4;
            ellipse(
                window,
                x,
                base,
                w * 0.52,
                h * 0.1,
                gpui::black().opacity(0.1),
            );
            rect(
                window,
                x - w * 0.48,
                base - h * 0.35,
                w * 0.96,
                h * 0.35,
                3.0,
                hex(0x7a5a3a),
            );
            for (dx, dy) in [
                (-0.35, 0.5),
                (-0.15, 0.65),
                (0.05, 0.55),
                (0.25, 0.7),
                (0.4, 0.5),
            ] {
                circle(window, x + w * dx, base - h * dy, w * 0.09, hex(0x5f9a4a));
            }
            for (dx, dy, colour) in [
                (-0.3, 0.75, 0xe06f8b),
                (0.0, 0.8, 0xffd05a),
                (0.3, 0.9, 0xd64545),
                (-0.05, 0.6, 0xffffff),
            ] {
                circle(window, x + w * dx, base - h * dy, w * 0.035, hex(colour));
            }
        }
        MarkShape::Bench => {
            let h = w * 0.34;
            ellipse(
                window,
                x,
                base,
                w * 0.5,
                h * 0.14,
                gpui::black().opacity(0.12),
            );
            let wood = hex(0x9a6a3e);
            // Legs, seat and back.
            for dx in [-0.38, 0.34] {
                rect(
                    window,
                    x + w * dx,
                    base - h * 0.55,
                    w * 0.05,
                    h * 0.55,
                    1.0,
                    shade(wood, -0.3),
                );
            }
            rect(
                window,
                x - w * 0.45,
                base - h * 0.62,
                w * 0.9,
                h * 0.14,
                2.0,
                wood,
            );
            rect(window, x - w * 0.45, base - h, w * 0.9, h * 0.12, 2.0, wood);
            rect(
                window,
                x - w * 0.45,
                base - h * 0.82,
                w * 0.9,
                h * 0.1,
                2.0,
                shade(wood, -0.1),
            );
        }
        MarkShape::Well => {
            let h = w * 0.7;
            ellipse(
                window,
                x,
                base,
                w * 0.45,
                h * 0.08,
                gpui::black().opacity(0.12),
            );
            let stone = hex(0x9c9488);
            rect(
                window,
                x - w * 0.34,
                base - h * 0.4,
                w * 0.68,
                h * 0.4,
                3.0,
                stone,
            );
            for row in 0..2 {
                for column in 0..3 {
                    let offset = if row == 0 { 0.0 } else { 0.11 };
                    rect(
                        window,
                        x - w * 0.3 + w * (0.22 * column as f32 + offset),
                        base - h * (0.36 - 0.17 * row as f32),
                        w * 0.18,
                        h * 0.12,
                        1.5,
                        shade(stone, 0.12),
                    );
                }
            }
            for dx in [-0.3, 0.26] {
                rect(
                    window,
                    x + w * dx,
                    base - h * 0.95,
                    w * 0.04,
                    h * 0.55,
                    0.0,
                    hex(0x7a5534),
                );
            }
            polygon(
                window,
                &[
                    (x - w * 0.42, base - h * 0.88),
                    (x, base - h * 1.1),
                    (x + w * 0.42, base - h * 0.88),
                ],
                hex(0xa04a3a),
            );
            rect(
                window,
                x - w * 0.06,
                base - h * 0.7 + sway * 1.5,
                w * 0.12,
                h * 0.12,
                2.0,
                hex(0x6b6f78),
            );
        }
        MarkShape::Swing => {
            let h = w * 0.85;
            ellipse(
                window,
                x,
                base,
                w * 0.45,
                h * 0.06,
                gpui::black().opacity(0.1),
            );
            let frame = hex(0x7a5534);
            polygon(
                window,
                &[
                    (x - w * 0.42, base),
                    (x - w * 0.3, base - h),
                    (x - w * 0.26, base - h),
                    (x - w * 0.36, base),
                ],
                frame,
            );
            polygon(
                window,
                &[
                    (x + w * 0.42, base),
                    (x + w * 0.3, base - h),
                    (x + w * 0.26, base - h),
                    (x + w * 0.36, base),
                ],
                frame,
            );
            rect(
                window,
                x - w * 0.34,
                base - h,
                w * 0.68,
                h * 0.06,
                1.0,
                frame,
            );
            let swing = sway * w * 0.05;
            for dx in [-0.1, 0.1] {
                rect(
                    window,
                    x + w * dx + swing * 0.5,
                    base - h * 0.95,
                    1.2,
                    h * 0.62,
                    0.0,
                    hex(0x5b5d63),
                );
            }
            rect(
                window,
                x - w * 0.14 + swing,
                base - h * 0.34,
                w * 0.28,
                h * 0.05,
                1.5,
                hex(0xc9542f),
            );
        }
        MarkShape::Fountain => {
            let h = w * 0.62;
            ellipse(
                window,
                x,
                base,
                w * 0.5,
                h * 0.1,
                gpui::black().opacity(0.12),
            );
            let stone = hex(0xb3aca0);
            rect(
                window,
                x - w * 0.46,
                base - h * 0.3,
                w * 0.92,
                h * 0.3,
                6.0,
                stone,
            );
            ellipse(window, x, base - h * 0.3, w * 0.44, h * 0.08, hex(0x6fa8c9));
            rect(
                window,
                x - w * 0.06,
                base - h * 0.75,
                w * 0.12,
                h * 0.45,
                2.0,
                shade(stone, -0.1),
            );
            ellipse(window, x, base - h * 0.75, w * 0.2, h * 0.05, stone);
            let rise = 0.9 + 0.1 * sway;
            for dx in [-0.12, 0.0, 0.12] {
                circle(
                    window,
                    x + w * dx,
                    base - h * (0.85 + 0.12 * rise),
                    w * 0.035,
                    hex(0xa9d8ee).opacity(0.85),
                );
            }
        }
        MarkShape::Signpost => {
            let h = w * 0.95;
            let wood = hex(0x8a6038);
            rect(window, x - w * 0.03, base - h, w * 0.06, h, 0.0, wood);
            polygon(
                window,
                &[
                    (x, base - h * 0.9),
                    (x + w * 0.36, base - h * 0.9),
                    (x + w * 0.44, base - h * 0.84),
                    (x + w * 0.36, base - h * 0.78),
                    (x, base - h * 0.78),
                ],
                shade(wood, 0.15),
            );
            polygon(
                window,
                &[
                    (x, base - h * 0.66),
                    (x - w * 0.36, base - h * 0.66),
                    (x - w * 0.44, base - h * 0.6),
                    (x - w * 0.36, base - h * 0.54),
                    (x, base - h * 0.54),
                ],
                shade(wood, 0.1),
            );
        }
        MarkShape::Birdhouse => {
            let h = w * 1.0;
            rect(
                window,
                x - w * 0.025,
                base - h * 0.6,
                w * 0.05,
                h * 0.6,
                0.0,
                hex(0x6b6f78),
            );
            rect(
                window,
                x - w * 0.16,
                base - h * 0.88,
                w * 0.32,
                h * 0.3,
                2.0,
                hex(0xd9b36c),
            );
            polygon(
                window,
                &[
                    (x - w * 0.22, base - h * 0.86),
                    (x, base - h * 1.02),
                    (x + w * 0.22, base - h * 0.86),
                ],
                hex(0xa04a3a),
            );
            circle(window, x, base - h * 0.74, w * 0.05, hex(0x3a2a1a));
            circle(
                window,
                x + w * 0.1 + sway,
                base - h * 0.95,
                w * 0.045,
                hex(0x5a7fb0),
            );
        }
        MarkShape::Planter => {
            let h = w * 0.34;
            ellipse(
                window,
                x,
                base,
                w * 0.5,
                h * 0.12,
                gpui::black().opacity(0.1),
            );
            rect(
                window,
                x - w * 0.46,
                base - h * 0.5,
                w * 0.92,
                h * 0.5,
                2.0,
                hex(0x8a5a36),
            );
            for (index, colour) in [
                0xe06f8b_u32,
                0xffd05a,
                0xb46fe0,
                0xffffff,
                0xe0806f,
                0xffd05a,
            ]
            .iter()
            .enumerate()
            {
                let dx = -0.38 + 0.15 * index as f32;
                circle(window, x + w * dx, base - h * 0.62, w * 0.05, hex(0x5f9a4a));
                circle(window, x + w * dx, base - h * 0.8, w * 0.04, hex(*colour));
            }
        }
        MarkShape::Statue => {
            let h = w * 1.1;
            ellipse(
                window,
                x,
                base,
                w * 0.35,
                h * 0.05,
                gpui::black().opacity(0.12),
            );
            let stone = hex(0xc3bdb2);
            rect(
                window,
                x - w * 0.24,
                base - h * 0.28,
                w * 0.48,
                h * 0.28,
                1.5,
                shade(stone, -0.12),
            );
            rect(
                window,
                x - w * 0.1,
                base - h * 0.78,
                w * 0.2,
                h * 0.5,
                4.0,
                stone,
            );
            circle(window, x, base - h * 0.86, w * 0.09, stone);
            polygon(
                window,
                &[
                    (x + w * 0.1, base - h * 0.7),
                    (x + w * 0.26, base - h * 0.92),
                    (x + w * 0.3, base - h * 0.88),
                    (x + w * 0.12, base - h * 0.62),
                ],
                stone,
            );
        }
        MarkShape::Postbox => {
            let h = w * 0.9;
            ellipse(
                window,
                x,
                base,
                w * 0.25,
                h * 0.05,
                gpui::black().opacity(0.12),
            );
            let red = hex(0xc0302a);
            rect(
                window,
                x - w * 0.16,
                base - h * 0.85,
                w * 0.32,
                h * 0.85,
                4.0,
                red,
            );
            ellipse(
                window,
                x,
                base - h * 0.85,
                w * 0.18,
                h * 0.06,
                shade(red, -0.15),
            );
            rect(
                window,
                x - w * 0.1,
                base - h * 0.64,
                w * 0.2,
                h * 0.04,
                1.0,
                hex(0x2a1a1a),
            );
            rect(
                window,
                x - w * 0.18,
                base - h * 0.08,
                w * 0.36,
                h * 0.08,
                1.0,
                shade(red, -0.25),
            );
        }
        MarkShape::Sprouts => {
            let h = w * 0.28;
            ellipse(
                window,
                x,
                base,
                w * 0.5,
                h * 0.14,
                gpui::black().opacity(0.1),
            );
            rect(
                window,
                x - w * 0.46,
                base - h * 0.4,
                w * 0.92,
                h * 0.4,
                3.0,
                hex(0x6b4a2e),
            );
            // Rows of little shoots, two leaves each.
            for dx in [-0.34, -0.17, 0.0, 0.17, 0.34] {
                let stem_x = x + w * dx;
                rect(
                    window,
                    stem_x - 0.75,
                    base - h * 0.9,
                    1.5,
                    h * 0.5,
                    0.0,
                    hex(0x4f8a3c),
                );
                circle(
                    window,
                    stem_x - w * 0.03,
                    base - h * 0.9,
                    w * 0.035,
                    hex(0x6fb04f),
                );
                circle(
                    window,
                    stem_x + w * 0.03,
                    base - h * 0.95,
                    w * 0.035,
                    hex(0x6fb04f),
                );
            }
        }
        MarkShape::Flag => {
            let h = w * 0.9;
            rect(window, x - 1.5, base - h, 3.0, h, 0.0, hex(0x6b6f78));
            let wave = sway * w * 0.02;
            polygon(
                window,
                &[
                    (x + 1.5, base - h),
                    (x + w * 0.42, base - h * 0.9 + wave),
                    (x + 1.5, base - h * 0.78),
                ],
                palette.roof,
            );
        }
        MarkShape::Lantern => {
            let h = w * 0.95;
            rect(
                window,
                x - 2.0,
                base - h * 0.8,
                4.0,
                h * 0.8,
                1.0,
                hex(0x3c3f46),
            );
            circle(
                window,
                x,
                base - h * 0.86,
                w * 0.16,
                palette.glass.opacity(0.35),
            );
            rect(
                window,
                x - w * 0.08,
                base - h * 0.95,
                w * 0.16,
                h * 0.16,
                2.0,
                palette.glass,
            );
            rect(
                window,
                x - w * 0.1,
                base - h,
                w * 0.2,
                h * 0.05,
                1.0,
                hex(0x3c3f46),
            );
        }
        MarkShape::Tent => {
            let h = w * 0.65;
            ellipse(
                window,
                x,
                base,
                w * 0.55,
                h * 0.08,
                gpui::black().opacity(0.12),
            );
            polygon(
                window,
                &[(x - w * 0.5, base), (x, base - h), (x + w * 0.5, base)],
                palette.roof,
            );
            polygon(
                window,
                &[
                    (x - w * 0.12, base),
                    (x, base - h * 0.55),
                    (x + w * 0.12, base),
                ],
                shade(palette.roof, -0.25),
            );
        }
        other => paint_building(window, x, base, w, w, other, palette),
    }
}

/// A heart for warmth, a crack for strain, three dots for not yet either,
/// in a small round bubble centred on (`x`, `y`).
pub fn paint_bond(
    window: &mut dyn Brush,
    x: f32,
    y: f32,
    r: f32,
    tone: world_projection::CanvasLinkTone,
) {
    use world_projection::CanvasLinkTone;
    circle(window, x, y + 1.0, r, gpui::black().opacity(0.08));
    circle(window, x, y, r, gpui::white());
    let at = |dx: f32, dy: f32| (x + dx * r, y + dy * r);
    match tone {
        CanvasLinkTone::Warm => {
            let mut heart = Shape::new();
            let curve = |heart: &mut Shape, to: (f32, f32), control: (f32, f32)| {
                heart.curve_to(to.0, to.1, control.0, control.1);
            };
            let (sx, sy) = at(0.0, 0.55);
            heart.move_to(sx, sy);
            curve(&mut heart, at(-0.55, -0.1), at(-0.6, 0.2));
            curve(&mut heart, at(0.0, -0.2), at(-0.35, -0.6));
            curve(&mut heart, at(0.55, -0.1), at(0.35, -0.6));
            curve(&mut heart, at(0.0, 0.55), at(0.6, 0.2));
            heart.close();
            window.fill(&heart, hex(0xd9534f));
        }
        CanvasLinkTone::Strained => {
            let crack = [
                at(-0.2, -0.55),
                at(0.15, -0.1),
                at(-0.15, 0.1),
                at(0.2, 0.55),
            ];
            let mut shape = Shape::new();
            shape.move_to(crack[0].0, crack[0].1);
            for (x, y) in &crack[1..] {
                shape.line_to(*x, *y);
            }
            window.stroke(&shape, 2.0, hex(0x9a3a32));
        }
        CanvasLinkTone::Neutral => {
            for dx in [-0.4_f32, 0.0, 0.4] {
                circle(window, x + dx * r, y, r * 0.13, hex(0x6b6f78));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_person_always_looks_the_same_and_hints_win() {
        let nia = Figure::of("entity-11", None);
        assert_eq!(nia, Figure::of("entity-11", None));
        let hinted = Figure::of(
            "entity-11",
            Some(Look {
                clothes: Some(0x2f7f86),
                ..Look::default()
            }),
        );
        assert_eq!(hinted.clothes, hex(0x2f7f86));
        assert_eq!(
            hinted.hair, nia.hair,
            "what a Pack leaves out stays the same"
        );
    }

    #[test]
    fn shading_moves_toward_black_and_white() {
        let colour = hex(0x808080);
        assert!(shade(colour, -0.5).l < colour.l);
        assert!(shade(colour, 0.5).l > colour.l);
    }
}

/// The colours a drawing's roles are filled with.
#[derive(Clone, Copy, Debug)]
pub struct Inks {
    pub wall: Hsla,
    pub roof: Hsla,
    pub trim: Hsla,
    pub glass: Hsla,
    pub clothes: Hsla,
    pub hair: Hsla,
    pub skin: Hsla,
    /// Whether windows are lit, and glow.
    pub lit: bool,
}

impl Inks {
    /// With windows lit, or not.
    pub fn lit(mut self, lit: bool) -> Self {
        self.lit = lit;
        self
    }

    /// A place's colours.
    pub fn of_place(palette: &Palette) -> Self {
        Self {
            wall: palette.wall,
            roof: palette.roof,
            trim: palette.trim,
            glass: palette.glass,
            clothes: palette.roof,
            hair: palette.trim,
            skin: hex(0xf0c7a2),
            lit: false,
        }
    }

    /// A person's colours; their clothes stand in for walls and roof.
    pub fn of_person(figure: &Figure) -> Self {
        Self {
            wall: figure.clothes,
            roof: shade(figure.clothes, -0.2),
            trim: hex(0x3b3f4a),
            glass: hex(0xdfe8ee),
            clothes: figure.clothes,
            hair: figure.hair,
            skin: figure.skin,
            lit: false,
        }
    }

    fn of(&self, ink: world_projection::Ink) -> Hsla {
        use world_projection::Ink;
        match ink {
            Ink::Colour(colour) => hex(colour),
            Ink::Wall => self.wall,
            Ink::Roof => self.roof,
            Ink::Trim => self.trim,
            Ink::Glass => self.glass,
            Ink::Clothes => self.clothes,
            Ink::Hair => self.hair,
            Ink::Skin => self.skin,
            Ink::Shade => gpui::black().opacity(0.18),
        }
    }
}

/// A Pack's drawing, standing with its base centred on (`x`, `base`), `w`
/// wide and `h` tall, in one stance. `swing` is where a walk is in its
/// step, from -1 to 1; `bob` lifts it; a negative `facing` turns it round.
#[allow(clippy::too_many_arguments)]
pub fn paint_drawing(
    window: &mut dyn Brush,
    x: f32,
    base: f32,
    w: f32,
    h: f32,
    drawing: &world_projection::Drawing,
    inks: &Inks,
    stance: world_projection::Stance,
    mood: world_projection::Mood,
    swing: f32,
    bob: f32,
    facing: f32,
) {
    use world_projection::DrawShape;
    let flip = if facing < 0.0 { -1.0 } else { 1.0 };
    let drop = world_projection::drop_of(stance) * h;
    for part in drawing.parts.iter().filter(|part| part.shows(stance, mood)) {
        let step = part.swing * swing;
        let px = |dx: f32| x + (dx + step) * w * flip;
        let py = |dy: f32| base - dy * h - bob + drop;
        let colour = match part.ink {
            world_projection::Ink::Shade => inks.of(part.ink),
            ink => shade(inks.of(ink), part.tone),
        };
        match &part.shape {
            DrawShape::Rect {
                x: rx,
                y: ry,
                w: rw,
                h: rh,
                round,
            } => {
                let left = if flip > 0.0 { px(*rx) } else { px(rx + rw) };
                // A lit window throws a soft glow around itself.
                if inks.lit && part.ink == world_projection::Ink::Glass {
                    let (gw, gh) = (rw * w, rh * h);
                    rect(
                        window,
                        left - gw * 0.35,
                        py(ry + rh) - gh * 0.35,
                        gw * 1.7,
                        gh * 1.7,
                        gw.min(gh) * 0.6,
                        colour.opacity(0.22),
                    );
                }
                rect(window, left, py(ry + rh), rw * w, rh * h, round * w, colour);
            }
            DrawShape::Ellipse {
                x: cx,
                y: cy,
                rx,
                ry,
            } => ellipse(window, px(*cx), py(*cy), rx * w, ry * h, colour),
            DrawShape::Polygon { points } => {
                let points = points
                    .iter()
                    .map(|(dx, dy)| (px(*dx), py(*dy)))
                    .collect::<Vec<_>>();
                polygon(window, &points, colour);
            }
            DrawShape::Line { from, to, width } => line(
                window,
                (px(from.0), py(from.1)),
                (px(to.0), py(to.1)),
                width * w,
                colour,
            ),
        }
    }
}

/// Someone's own drawing, standing in `pose`: squashed or stretched,
/// leaning, and in a walk with each foot planted while the body passes over
/// it.
#[allow(clippy::too_many_arguments)]
pub fn paint_drawing_posed(
    window: &mut dyn Brush,
    x: f32,
    base: f32,
    w: f32,
    h: f32,
    drawing: &world_projection::Drawing,
    inks: &Inks,
    stance: world_projection::Stance,
    mood: world_projection::Mood,
    pose: Pose,
) {
    let swing = pose.stride.map(|phase| step(phase, 1.0).0).unwrap_or(0.0);
    if pose.plain() {
        paint_drawing(
            window,
            x,
            base,
            w,
            h,
            drawing,
            inks,
            stance,
            mood,
            swing,
            pose.bob,
            pose.facing,
        );
        return;
    }
    let mut posed = Xform::about(
        window,
        (x, base),
        1.0 / pose.squash.max(0.2).sqrt(),
        pose.squash,
        pose.lean,
    );
    paint_drawing(
        &mut posed,
        x,
        base,
        w,
        h,
        drawing,
        inks,
        stance,
        mood,
        swing,
        pose.bob,
        pose.facing,
    );
}
