//! Drawing the things a World is made of: people as small figures in the
//! clothes of their work, places as the buildings they are, vehicles,
//! boats and parcels, and the landscape they stand in. Everything is
//! painted from shapes, so any World a Pack describes can be drawn without
//! shipping a picture.

use gpui::{point, px, quad, rgb, size, BorderStyle, Bounds, Hsla, PathBuilder, Window};
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

pub fn rect(window: &mut Window, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    window.paint_quad(quad(
        Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
        px(radius.min(w / 2.0).min(h / 2.0)),
        colour,
        px(0.0),
        colour,
        BorderStyle::default(),
    ));
}

pub fn circle(window: &mut Window, cx: f32, cy: f32, r: f32, colour: Hsla) {
    rect(window, cx - r, cy - r, r * 2.0, r * 2.0, r, colour);
}

pub fn polygon(window: &mut Window, points: &[(f32, f32)], colour: Hsla) {
    let Some(first) = points.first() else {
        return;
    };
    let mut path = PathBuilder::fill();
    path.move_to(point(px(first.0), px(first.1)));
    for (x, y) in &points[1..] {
        path.line_to(point(px(*x), px(*y)));
    }
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, colour);
    }
}

pub fn line(window: &mut Window, from: (f32, f32), to: (f32, f32), width: f32, colour: Hsla) {
    let mut path = PathBuilder::stroke(px(width));
    path.move_to(point(px(from.0), px(from.1)));
    path.line_to(point(px(to.0), px(to.1)));
    if let Ok(path) = path.build() {
        window.paint_path(path, colour);
    }
}

/// An ellipse, as eight curved segments.
pub fn ellipse(window: &mut Window, cx: f32, cy: f32, rx: f32, ry: f32, colour: Hsla) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    let step = std::f32::consts::TAU / 8.0;
    let reach = 1.0 / (step / 2.0).cos();
    let at = |angle: f32, scale: f32| {
        point(
            px(cx + rx * scale * angle.cos()),
            px(cy + ry * scale * angle.sin()),
        )
    };
    let mut path = PathBuilder::fill();
    path.move_to(at(0.0, 1.0));
    for index in 0..8 {
        let start = index as f32 * step;
        path.curve_to(at(start + step, 1.0), at(start + step / 2.0, reach));
    }
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, colour);
    }
}

/// The top half of an ellipse standing on the line through (`cx`, `cy`).
pub fn dome(window: &mut Window, cx: f32, cy: f32, rx: f32, ry: f32, colour: Hsla) {
    if rx <= 0.0 || ry <= 0.0 {
        return;
    }
    let step = std::f32::consts::PI / 4.0;
    let reach = 1.0 / (step / 2.0).cos();
    let at = |angle: f32, scale: f32| {
        point(
            px(cx + rx * scale * angle.cos()),
            px(cy - ry * scale * angle.sin()),
        )
    };
    let mut path = PathBuilder::fill();
    path.move_to(at(0.0, 1.0));
    for index in 0..4 {
        let start = index as f32 * step;
        path.curve_to(at(start + step, 1.0), at(start + step / 2.0, reach));
    }
    path.close();
    if let Ok(path) = path.build() {
        window.paint_path(path, colour);
    }
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
}

impl Figure {
    pub fn of(key: &str, look: Option<Look>) -> Self {
        let seed = seed_of(key);
        let look = look.unwrap_or_default();
        let pick = |list: &[u32], salt: u32| list[((seed >> salt) as usize) % list.len()];
        Self {
            clothes: hex(look.clothes.unwrap_or_else(|| pick(&CLOTHES, 0))),
            hair: hex(look.hair.unwrap_or_else(|| pick(&HAIR, 7))),
            skin: hex(look.skin.unwrap_or_else(|| pick(&SKIN, 13))),
            carries: look.carries,
            bird: look.bird,
        }
    }
}

/// How a figure stands this frame: the swing of a walk (none when still),
/// a small rise and fall while it breathes, and which way it faces.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub stride: Option<f32>,
    pub bob: f32,
    pub facing: f32,
}

/// Someone standing with their feet at (`x`, `y`), `height` tall.
pub fn paint_figure(window: &mut Window, x: f32, y: f32, height: f32, figure: &Figure, pose: Pose) {
    let u = height / 10.0;
    ellipse(window, x, y, 2.2 * u, 0.55 * u, gpui::black().opacity(0.16));
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
    // Legs, swinging when walking.
    for side in [-1.0_f32, 1.0] {
        let step = swing * side * 0.7 * u;
        rect(
            window,
            x + side * 0.75 * u - 0.5 * u + step,
            y - 3.2 * u,
            1.0 * u,
            3.2 * u,
            0.4 * u,
            trousers,
        );
        rect(
            window,
            x + side * 0.75 * u - 0.6 * u + step,
            y - 0.45 * u,
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

fn paint_bird(window: &mut Window, x: f32, y: f32, u: f32, figure: &Figure, pose: Pose) {
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
fn paint_carry(window: &mut Window, x: f32, y: f32, u: f32, carry: Carry) {
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
pub fn paint_portrait(window: &mut Window, bounds: Bounds<gpui::Pixels>, figure: &Figure) {
    let x = f32::from(bounds.origin.x);
    let y = f32::from(bounds.origin.y);
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    let backdrop = shade(figure.clothes, 0.78);
    rect(window, x, y, w, h, w.min(h) * 0.24, backdrop);
    // The figure drawn large enough that its head fills the upper half,
    // standing below the frame so only head and shoulders show.
    let height = h * 1.7;
    let feet = y + h * 1.62;
    paint_figure(
        window,
        x + w / 2.0,
        feet,
        height,
        &Figure {
            carries: None,
            ..*figure
        },
        Pose::default(),
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
        }
    }
}

/// A place drawn as the building it is, standing with its base centred on
/// (`x`, `base`), `w` wide and `h` tall.
pub fn paint_building(
    window: &mut Window,
    x: f32,
    base: f32,
    w: f32,
    h: f32,
    shape: MarkShape,
    palette: &Palette,
) {
    let left = x - w / 2.0;
    let top = base - h;
    ellipse(
        window,
        x,
        base,
        w * 0.55,
        h * 0.05,
        gpui::black().opacity(0.12),
    );
    let door = |window: &mut Window, height: f32| {
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
        MarkShape::House | MarkShape::Lamp => {
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
            let mut arch = PathBuilder::stroke(px(h * 0.08));
            arch.move_to(point(px(left + w * 0.12), px(base)));
            arch.curve_to(
                point(px(left + w * 0.88), px(base)),
                point(px(x), px(deck - h * 0.25)),
            );
            if let Ok(path) = arch.build() {
                window.paint_path(path, stone);
            }
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
        MarkShape::Rover | MarkShape::Boat | MarkShape::Parcel => {
            paint_thing(window, x, base, w, shape, palette, 0.0);
        }
    }
}

/// A thing drawn standing (or floating) with its base centred on (`x`,
/// `base`), `w` wide: a rover, a boat, a parcel. `sway` rocks a boat.
pub fn paint_thing(
    window: &mut Window,
    x: f32,
    base: f32,
    w: f32,
    shape: MarkShape,
    palette: &Palette,
    sway: f32,
) {
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
        other => paint_building(window, x, base, w, w, other, palette),
    }
}

/// A heart for warmth, a crack for strain, three dots for not yet either,
/// in a small round bubble centred on (`x`, `y`).
pub fn paint_bond(
    window: &mut Window,
    x: f32,
    y: f32,
    r: f32,
    tone: world_projection::CanvasLinkTone,
) {
    use world_projection::CanvasLinkTone;
    circle(window, x, y + 1.0, r, gpui::black().opacity(0.08));
    circle(window, x, y, r, gpui::white());
    let at = |dx: f32, dy: f32| point(px(x + dx * r), px(y + dy * r));
    match tone {
        CanvasLinkTone::Warm => {
            let mut heart = PathBuilder::fill();
            heart.move_to(at(0.0, 0.55));
            heart.curve_to(at(-0.55, -0.1), at(-0.6, 0.2));
            heart.curve_to(at(0.0, -0.2), at(-0.35, -0.6));
            heart.curve_to(at(0.55, -0.1), at(0.35, -0.6));
            heart.curve_to(at(0.0, 0.55), at(0.6, 0.2));
            heart.close();
            if let Ok(path) = heart.build() {
                window.paint_path(path, hex(0xd9534f));
            }
        }
        CanvasLinkTone::Strained => {
            let mut crack = PathBuilder::stroke(px(2.0));
            crack.move_to(at(-0.2, -0.55));
            crack.line_to(at(0.15, -0.1));
            crack.line_to(at(-0.15, 0.1));
            crack.line_to(at(0.2, 0.55));
            if let Ok(path) = crack.build() {
                window.paint_path(path, hex(0x9a3a32));
            }
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
