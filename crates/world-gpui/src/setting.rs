//! Each kind of place dressed as itself: what grows and lies about on its
//! ground, what stands along its street, what floats on its water, and
//! what people sit on. A harbour's shore has grass, flowers, fence posts
//! and reeds; Mars has rocks, craters and rover tracks under a
//! butterscotch sky; a 1987 street has its sidewalk, road, parked cars,
//! poles and overhead wires; the ice has sastrugi, pebbles, an ice shelf
//! and floes on the sea. Nothing from the harbour appears anywhere else.
//!
//! All of it is seeded by the place, so the same place always has the same
//! props and a still picture never changes.

use crate::art::{self, Setting};
use crate::brush::{Brush, Shape};
use crate::painter::{self, Canvas};
use gpui::Hsla;
use world_projection::GroundCover;

/// Where the ground's props go, in stage pixels, and what it looks like.
pub struct Ground {
    pub setting: Setting,
    /// The stretch of stage being painted.
    pub from: f32,
    pub to: f32,
    /// The top and bottom of the near ground, where things grow.
    pub strip_top: f32,
    pub meadow: f32,
    /// The top of the foreground (water or near ground), and the stage's
    /// foot.
    pub front: f32,
    pub height: f32,
    /// How wide the whole panorama is, and one view of it.
    pub width: f32,
    pub view_w: f32,
    /// How large props are drawn.
    pub k: f32,
    pub seed: u32,
    pub ground: Hsla,
    pub near: Hsla,
    pub cover: Option<GroundCover>,
    pub water: bool,
}

impl Ground {
    /// How many views wide the panorama is.
    fn per(&self) -> f32 {
        self.width / self.view_w.max(1.0)
    }

    /// The `index`th scattered spot for `salt`: where along the panorama,
    /// how far down (0 to 1), and a seed.
    fn scatter(&self, index: i32, salt: u32) -> (f32, f32, u32) {
        let seed = painter::hash2(index, salt as i32, self.seed);
        (
            (seed % 10_000) as f32 / 10_000.0 * self.width,
            ((seed / 10_000) % 1000) as f32 / 1000.0,
            seed,
        )
    }

    fn seen(&self, x: f32, reach: f32) -> bool {
        x + reach >= self.from && x - reach <= self.to
    }
}

/// What grows and lies about on the near ground, and on dry ground the
/// foreground too, each place its own.
pub fn paint_ground_props(canvas: &mut Canvas, g: &Ground) {
    match g.setting {
        Setting::Harbour => harbour_props(canvas, g),
        Setting::Mars => mars_props(canvas, g),
        Setting::Street => street_props(canvas, g),
        Setting::Ice => ice_props(canvas, g),
    }
}

fn harbour_props(canvas: &mut Canvas, g: &Ground) {
    let (k, per) = (g.k, g.per());
    let blade = match g.cover {
        Some(GroundCover::Snow) => art::shade(g.ground, -0.12),
        _ => art::shade(g.ground, -0.22),
    };
    for index in 0..((48.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 1);
        if !g.seen(x, 0.0) {
            continue;
        }
        let y = g.strip_top + t * (g.meadow - g.strip_top);
        let tall = (9.0 + (seed >> 24) as f32 % 8.0) * k;
        for lean in [-1.0_f32, 0.0, 1.0] {
            art::line(
                canvas,
                (x + lean * 2.0 * k, y),
                (x + lean * 4.0 * k, y - tall * (1.0 - lean.abs() * 0.25)),
                1.3 * k,
                blade,
            );
        }
    }
    let flower_inks = [0xf2d0e0_u32, 0xfff2b0, 0xffffff, 0xd8c8f2];
    let flowering = !matches!(
        g.cover,
        Some(GroundCover::Snow | GroundCover::Frost | GroundCover::Dust)
    );
    for index in 0..((16.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 2);
        if !g.seen(x, 0.0) {
            continue;
        }
        let y = g.strip_top + t * (g.meadow - g.strip_top);
        if seed % 3 == 0 || !flowering {
            art::ellipse(canvas, x, y, 5.0 * k, 3.0 * k, art::shade(g.ground, -0.35));
            art::ellipse(
                canvas,
                x - 1.0 * k,
                y - 1.0 * k,
                3.0 * k,
                1.6 * k,
                art::shade(g.ground, 0.12),
            );
        } else {
            let ink = art::hex(flower_inks[((seed >> 8) % 4) as usize]);
            art::line(canvas, (x, y), (x, y - 10.0 * k), 1.2 * k, blade);
            art::circle(canvas, x, y - 10.5 * k, 3.4 * k, ink);
            art::circle(canvas, x, y - 10.5 * k, 1.3 * k, art::hex(0xe8b040));
        }
    }
    // A short run of fence posts here and there.
    let post = art::shade(g.ground, -0.45);
    for index in 0..(per.ceil() as i32) {
        let (x, _, _) = g.scatter(index, 3);
        if x + 80.0 * k < g.from || x > g.to {
            continue;
        }
        for post_index in 0..4 {
            let px0 = x + post_index as f32 * 24.0 * k;
            art::rect(
                canvas,
                px0,
                g.strip_top - 6.0 * k,
                4.0 * k,
                20.0 * k,
                1.0,
                post,
            );
        }
        art::line(
            canvas,
            (x, g.strip_top + 1.0 * k),
            (x + 76.0 * k, g.strip_top + 1.0 * k),
            1.4 * k,
            post,
        );
    }
    if !g.water {
        // On dry ground the foreground itself has stones and grass too.
        for index in 0..((16.0 * per) as i32) {
            let (x, t, seed) = g.scatter(index, 5);
            if !g.seen(x, 0.0) {
                continue;
            }
            let y = g.front + 10.0 + t * (g.height - g.front - 14.0);
            if seed % 2 == 0 {
                art::ellipse(canvas, x, y, 7.0 * k, 4.0 * k, art::shade(g.near, -0.25));
            } else {
                for lean in [-1.0_f32, 1.0] {
                    art::line(
                        canvas,
                        (x, y),
                        (x + lean * 4.0 * k, y - 9.0 * k),
                        1.4 * k,
                        art::shade(g.near, 0.2),
                    );
                }
            }
        }
    }
}

/// A rock lying on the ground: dark underside, a lit top, its shadow.
fn rock(canvas: &mut Canvas, x: f32, y: f32, r: f32, ink: Hsla) {
    canvas.soft(
        x + r * 0.5,
        y + r * 0.25,
        r * 1.3,
        r * 0.35,
        r * 0.4,
        gpui::black().opacity(0.14),
    );
    art::ellipse(canvas, x, y - r * 0.3, r, r * 0.62, art::shade(ink, -0.28));
    art::ellipse(
        canvas,
        x - r * 0.18,
        y - r * 0.52,
        r * 0.7,
        r * 0.38,
        art::shade(ink, 0.1),
    );
}

fn mars_props(canvas: &mut Canvas, g: &Ground) {
    let (k, per) = (g.k, g.per());
    let dust = art::shade(g.ground, 0.12);
    // Rover tracks along the near ground: two faint ruts, wandering.
    for (lane, salt) in [(0.35_f32, 11_u32), (0.72, 12)] {
        let mid = g.strip_top + lane * (g.meadow - g.strip_top);
        for rut in [-1.0_f32, 1.0] {
            let mut track = Shape::new();
            let mut x = g.from;
            let wobble = |x: f32| {
                mid + rut * 3.5 * k + 5.0 * k * (x / (0.43 * g.view_w) + salt as f32).sin()
            };
            track.move_to(x, wobble(x));
            while x < g.to + 24.0 {
                x += 24.0;
                track.line_to(x, wobble(x));
            }
            canvas.stroke(&track, 1.6 * k, art::shade(g.ground, -0.2).opacity(0.35));
        }
    }
    // Craters: a dark bowl under a lit rim.
    for index in 0..((5.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 13);
        let r = (10.0 + (seed >> 20) as f32 % 18.0) * k;
        if !g.seen(x, r * 2.0) {
            continue;
        }
        let y = g.strip_top + t * (g.meadow - g.strip_top);
        art::ellipse(canvas, x, y, r * 1.2, r * 0.34, art::shade(g.ground, 0.14));
        art::ellipse(
            canvas,
            x,
            y + r * 0.04,
            r,
            r * 0.26,
            art::shade(g.ground, -0.22),
        );
        art::ellipse(
            canvas,
            x + r * 0.2,
            y + r * 0.08,
            r * 0.6,
            r * 0.14,
            art::shade(g.ground, -0.1),
        );
    }
    // Rocks, many small and a few large.
    for index in 0..((26.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 14);
        if !g.seen(x, 20.0) {
            continue;
        }
        let y = g.strip_top + t * (g.meadow - g.strip_top);
        let r = (2.5 + ((seed >> 16) % 9) as f32 * if seed % 7 == 0 { 1.4 } else { 0.5 }) * k;
        rock(canvas, x, y, r, g.ground);
    }
    // Wind ripples in the dust.
    for index in 0..((14.0 * per) as i32) {
        let (x, t, _) = g.scatter(index, 15);
        if !g.seen(x, 30.0) {
            continue;
        }
        let y = g.strip_top + t * (g.meadow - g.strip_top);
        for ripple in 0..3 {
            let dy = ripple as f32 * 3.0 * k;
            let mut arc = Shape::new();
            arc.move_to(x - 14.0 * k, y + dy)
                .curve_to(x + 14.0 * k, y + dy, x, y + dy - 4.0 * k);
            canvas.stroke(&arc, 1.0 * k, dust.opacity(0.5));
        }
    }
    // The near regolith: bigger rocks and darker ripples.
    for index in 0..((26.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 16);
        if !g.seen(x, 40.0) {
            continue;
        }
        let y = g.front + 12.0 + t * (g.height - g.front - 16.0);
        let r = (4.0 + ((seed >> 16) % 12) as f32 + if seed % 9 == 0 { 14.0 } else { 0.0 })
            * k
            * (0.6 + t);
        rock(canvas, x, y, r, g.near);
    }
}

fn street_props(canvas: &mut Canvas, g: &Ground) {
    let (k, per) = (g.k, g.per());
    // The sidewalk: pale concrete along the foot of the buildings, cut
    // into slabs, a kerb at its edge.
    let walk = art::hex(0xc9c9c6);
    let (top, bottom) = (g.strip_top - 4.0 * k, g.meadow);
    canvas.rect(g.from, top, g.to - g.from, bottom - top, 0.0, walk);
    canvas.rect(
        g.from,
        top,
        g.to - g.from,
        2.0 * k,
        0.0,
        art::shade(walk, -0.12),
    );
    let slab = 38.0 * k;
    let mut x = (g.from / slab).floor() * slab;
    while x < g.to {
        art::line(
            canvas,
            (x, top),
            (x - 6.0 * k, bottom),
            0.8 * k,
            art::shade(walk, -0.18).opacity(0.6),
        );
        x += slab;
    }
    canvas.rect(
        g.from,
        bottom - 3.0 * k,
        g.to - g.from,
        3.0 * k,
        0.0,
        art::shade(walk, -0.28),
    );
    // A strip of verge with a street tree now and then.
    for index in 0..((6.0 * per) as i32) {
        let (x, _, seed) = g.scatter(index, 21);
        if !g.seen(x, 30.0) {
            continue;
        }
        let y = top + 2.0 * k;
        canvas.rect(
            x - 12.0 * k,
            y - 1.0,
            24.0 * k,
            4.0 * k,
            1.0,
            art::shade(g.ground, -0.3),
        );
        let leaf = if seed % 2 == 0 {
            art::hex(0x4f7f4a)
        } else {
            art::hex(0xc0463a)
        };
        art::rect(
            canvas,
            x - 1.5 * k,
            y - 26.0 * k,
            3.0 * k,
            26.0 * k,
            1.0,
            art::hex(0x5a4030),
        );
        art::circle(
            canvas,
            x - 5.0 * k,
            y - 30.0 * k,
            9.0 * k,
            art::shade(leaf, -0.08),
        );
        art::circle(canvas, x + 5.0 * k, y - 32.0 * k, 10.0 * k, leaf);
        art::circle(canvas, x, y - 38.0 * k, 8.0 * k, art::shade(leaf, 0.1));
    }
    // A hydrant and a newspaper box here and there.
    for index in 0..((4.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 22);
        if !g.seen(x, 20.0) {
            continue;
        }
        let y = top + (bottom - top) * (0.3 + 0.4 * t);
        if seed % 2 == 0 {
            let red = art::hex(0xc0302a);
            art::rect(
                canvas,
                x - 3.0 * k,
                y - 12.0 * k,
                6.0 * k,
                12.0 * k,
                2.0 * k,
                red,
            );
            art::rect(
                canvas,
                x - 5.0 * k,
                y - 9.0 * k,
                10.0 * k,
                3.0 * k,
                1.0,
                red,
            );
            art::circle(canvas, x, y - 12.0 * k, 3.0 * k, art::shade(red, -0.15));
        } else {
            let blue = art::hex(0x2f4a8a);
            art::rect(
                canvas,
                x - 5.0 * k,
                y - 14.0 * k,
                10.0 * k,
                12.0 * k,
                1.0,
                blue,
            );
            art::rect(
                canvas,
                x - 4.0 * k,
                y - 12.0 * k,
                8.0 * k,
                4.0 * k,
                0.0,
                art::hex(0xe8e0cc),
            );
            art::rect(
                canvas,
                x - 4.0 * k,
                y - 2.0 * k,
                1.5 * k,
                2.0 * k,
                0.0,
                blue,
            );
            art::rect(
                canvas,
                x + 2.5 * k,
                y - 2.0 * k,
                1.5 * k,
                2.0 * k,
                0.0,
                blue,
            );
        }
    }
    if !g.water {
        // The road: asphalt with a dashed line down its middle, and the
        // odd patch and manhole.
        let lane = g.front + (g.height - g.front) * 0.42;
        let dash = 30.0 * k;
        let mut x = (g.from / (dash * 2.0)).floor() * dash * 2.0;
        while x < g.to {
            canvas.rect(
                x,
                lane - 1.5 * k,
                dash,
                3.0 * k,
                1.0,
                art::hex(0xf2c14e).opacity(0.85),
            );
            x += dash * 2.0;
        }
        for index in 0..((5.0 * per) as i32) {
            let (x, t, seed) = g.scatter(index, 23);
            if !g.seen(x, 30.0) {
                continue;
            }
            let y = g.front + 10.0 * k + t * (g.height - g.front - 20.0 * k);
            if seed % 2 == 0 {
                art::ellipse(canvas, x, y, 8.0 * k, 2.5 * k, art::shade(g.near, -0.25));
                art::ellipse(canvas, x, y, 6.0 * k, 1.6 * k, art::shade(g.near, 0.1));
            } else {
                canvas.rect(
                    x,
                    y,
                    26.0 * k,
                    9.0 * k,
                    2.0,
                    art::shade(g.near, 0.08).opacity(0.7),
                );
            }
        }
    }
}

fn ice_props(canvas: &mut Canvas, g: &Ground) {
    let (k, per) = (g.k, g.per());
    let snow = art::hex(0xf6f9fb);
    let shade = art::hex(0xb8cfdd);
    // Sastrugi: long wind-cut ridges of snow, lit on top and blue below.
    for index in 0..((16.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 31);
        let long = (26.0 + (seed >> 18) as f32 % 30.0) * k;
        if !g.seen(x, long) {
            continue;
        }
        let y = g.strip_top + t * (g.meadow - g.strip_top);
        let mut ridge = Shape::new();
        ridge
            .move_to(x - long, y)
            .curve_to(x + long, y, x - long * 0.2, y - 5.0 * k)
            .close();
        canvas.fill(&ridge, snow.opacity(0.85));
        let mut under = Shape::new();
        under.move_to(x - long * 0.7, y + 0.5).curve_to(
            x + long,
            y + 0.5,
            x + long * 0.2,
            y + 3.0 * k,
        );
        canvas.stroke(&under, 1.2 * k, shade.opacity(0.6));
    }
    // Blocks of ice lying about, and the penguins' pebbles.
    for index in 0..((8.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 32);
        if !g.seen(x, 20.0) {
            continue;
        }
        let y = g.strip_top + t * (g.meadow - g.strip_top);
        let s = (5.0 + (seed >> 16) as f32 % 8.0) * k;
        canvas.soft(
            x + s * 0.4,
            y + 1.0,
            s * 1.2,
            s * 0.3,
            s * 0.3,
            art::hex(0x7fa8c0).opacity(0.25),
        );
        art::polygon(
            canvas,
            &[
                (x - s, y),
                (x + s * 0.8, y),
                (x + s * 0.6, y - s * 1.1),
                (x - s * 0.7, y - s * 0.9),
            ],
            art::hex(0xcfe6f0),
        );
        art::polygon(
            canvas,
            &[
                (x + s * 0.1, y),
                (x + s * 0.8, y),
                (x + s * 0.6, y - s * 1.1),
                (x + s * 0.05, y - s * 1.0),
            ],
            art::hex(0xe8f4f8),
        );
    }
    for index in 0..((22.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 33);
        if !g.seen(x, 6.0) {
            continue;
        }
        let y = g.strip_top + t * (g.meadow - g.strip_top);
        let r = (1.8 + (seed >> 20) as f32 % 2.0) * k;
        let ink = art::hex([0x8e98a0_u32, 0x6f7880, 0xa8b0b6][(seed % 3) as usize]);
        art::ellipse(canvas, x, y, r, r * 0.7, ink);
    }
}

/// Floes and pack ice on the water in front of the ice shelf.
pub fn paint_sea_ice(canvas: &mut Canvas, g: &Ground) {
    if g.setting != Setting::Ice || !g.water {
        return;
    }
    let (k, per) = (g.k, g.per());
    let floe = art::hex(0xeef5f8);
    let under = art::hex(0x9cc4d6);
    for index in 0..((10.0 * per) as i32) {
        let (x, t, seed) = g.scatter(index, 34);
        let size = (14.0 + (seed >> 16) as f32 % 34.0) * k * (0.6 + t * 0.8);
        if !g.seen(x, size * 2.0) {
            continue;
        }
        let y = g.front + 22.0 * k + t * (g.height - g.front - 30.0 * k);
        let flat = size * 0.22;
        let cut = ((seed >> 8) % 5) as f32 * 0.08;
        let top = [
            (x - size, y),
            (x - size * (0.7 - cut), y - flat),
            (x + size * (0.5 + cut), y - flat * 1.1),
            (x + size, y - flat * 0.2),
            (x + size * 0.6, y + flat * 0.4),
        ];
        canvas.soft(
            x,
            y + flat * 0.9,
            size * 1.1,
            flat * 0.5,
            flat * 0.6,
            art::hex(0x1f4f66).opacity(0.25),
        );
        art::polygon(
            canvas,
            &[
                (x - size, y),
                (x + size, y - flat * 0.2),
                (x + size * 0.96, y + flat * 0.3),
                (x - size * 0.96, y + flat * 0.5),
            ],
            under,
        );
        art::polygon(canvas, &top, floe);
    }
}

/// The colours of the quay's deck and its wall down to the water: dressed
/// stone in a harbour, packed snow on blue ice at the ice shelf.
pub fn quay_inks(setting: Setting, ground: Hsla) -> (Hsla, Hsla) {
    match setting {
        Setting::Ice => (art::hex(0xf1f6f9), art::hex(0x9ccbe0)),
        _ => (
            crate::diorama::mix(art::hex(0xd9cdb6), ground, 0.2),
            crate::diorama::mix(art::hex(0xb9ab92), ground, 0.2),
        ),
    }
}

/// Icicles hanging from the ice shelf's edge, from `from` to `to`, the
/// edge at `y(x)`, `deep` long at most; nothing anywhere but the ice.
#[allow(clippy::too_many_arguments)]
pub fn paint_shelf_edge(
    canvas: &mut Canvas,
    setting: Setting,
    (from, to): (f32, f32),
    y: &dyn Fn(f32) -> f32,
    deep: f32,
    k: f32,
    seed: u32,
) {
    if setting != Setting::Ice {
        return;
    }
    let step = 9.0 * k;
    let mut x = (from / step).floor() * step;
    while x < to {
        let hash = painter::hash2((x / step) as i32, 35, seed);
        let long = deep * (0.2 + (hash % 100) as f32 / 160.0);
        let top = y(x);
        art::polygon(
            canvas,
            &[(x - 2.5 * k, top), (x + 2.5 * k, top), (x, top + long)],
            art::hex(0xe4f3f9).opacity(0.9),
        );
        x += step;
    }
}

/// Where along the panorama the street's poles stand, in stage pixels.
fn pole_spots(width: f32, view_w: f32) -> Vec<f32> {
    let gap = view_w.max(1.0) * 0.5;
    let count = (width / gap).ceil() as i32 + 1;
    (0..count)
        .map(|index| index as f32 * gap + gap * 0.3)
        .collect()
}

/// What stands along a street between its buildings and its road: utility
/// poles with the overhead wires sagging between them, and cars parked at
/// the kerb. `at` maps a stage point to the screen; `z` is the zoom.
#[allow(clippy::too_many_arguments)]
pub fn paint_street_life(
    window: &mut dyn Brush,
    at: &dyn Fn(f32, f32) -> (f32, f32),
    setting: Setting,
    width: f32,
    view_w: f32,
    base: f32,
    front: f32,
    building_h: f32,
    z: f32,
    lit: bool,
    seed: u32,
    (left, right): (f32, f32),
) {
    if setting != Setting::Street {
        return;
    }
    let pole = art::hex(0x5a4a3a);
    let wire = art::hex(0x2c2a3a).opacity(0.7);
    // The poles stand at the kerb, their tops above the roofs.
    let foot = front - 2.0;
    let tall = (foot - (base - building_h * 1.3)) * 0.7;
    let spots = pole_spots(width, view_w);
    let on_screen = |x: f32| (left - 80.0..right + 80.0).contains(&x);
    let mut tops: Vec<(f32, f32)> = Vec::new();
    for x in &spots {
        let (sx, sy) = at(*x, foot);
        let (_, ty) = at(*x, foot - tall);
        tops.push((sx, ty));
        if !on_screen(sx) {
            continue;
        }
        art::rect(window, sx - 1.6 * z, ty, 3.2 * z, sy - ty, 1.0, pole);
        art::rect(
            window,
            sx - 12.0 * z,
            ty + 6.0 * z,
            24.0 * z,
            2.2 * z,
            0.0,
            pole,
        );
        art::rect(
            window,
            sx - 8.0 * z,
            ty + 14.0 * z,
            16.0 * z,
            2.0 * z,
            0.0,
            pole,
        );
        for dx in [-10.0_f32, 10.0, -6.0, 6.0] {
            art::circle(
                window,
                sx + dx * z,
                ty + if dx.abs() > 8.0 { 6.0 } else { 14.0 } * z,
                1.4 * z,
                art::hex(0xd8d0bc),
            );
        }
        // A cobra-head streetlamp on its arm, lit after dusk.
        let arm_y = ty + tall * z * 0.22;
        let mut arm = Shape::new();
        arm.move_to(sx, arm_y + 4.0 * z).curve_to(
            sx + 26.0 * z,
            arm_y,
            sx + 12.0 * z,
            arm_y - 4.0 * z,
        );
        window.stroke(&arm, 1.8 * z, art::hex(0x6b6f78));
        art::rect(
            window,
            sx + 20.0 * z,
            arm_y - 1.5 * z,
            14.0 * z,
            4.5 * z,
            2.0 * z,
            art::hex(0x6b6f78),
        );
        if lit {
            let glow = art::hex(0xffc96b);
            art::rect(
                window,
                sx + 21.0 * z,
                arm_y + 2.0 * z,
                12.0 * z,
                2.0 * z,
                1.0,
                glow,
            );
            window.soft(
                sx + 27.0 * z,
                sy,
                46.0 * z,
                10.0 * z,
                12.0 * z,
                glow.opacity(0.4),
            );
            window.soft(
                sx + 27.0 * z,
                arm_y + 3.0 * z,
                10.0 * z,
                6.0 * z,
                6.0 * z,
                glow.opacity(0.6),
            );
            let mut cone = Shape::new();
            cone.move_to(sx + 22.0 * z, arm_y + 3.0 * z)
                .line_to(sx + 32.0 * z, arm_y + 3.0 * z)
                .line_to(sx + 60.0 * z, sy)
                .line_to(sx - 6.0 * z, sy)
                .close();
            window.fill(&cone, glow.opacity(0.1));
        }
        // A transformer drum on every third.
        if (x / view_w * 3.0) as i32 % 3 == 0 {
            art::rect(
                window,
                sx + 2.0 * z,
                ty + 20.0 * z,
                7.0 * z,
                12.0 * z,
                2.0 * z,
                art::hex(0x7a7f88),
            );
        }
    }
    for pair in tops.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        if x1 < left - 80.0 || x0 > right + 80.0 {
            continue;
        }
        for (drop, lift) in [(10.0_f32, 6.0_f32), (-10.0, 6.0), (6.0, 14.0)] {
            let sag = (x1 - x0).abs() * 0.06;
            let mut line = Shape::new();
            let (a, b) = (
                (x0 + drop * z, y0 + lift * z),
                (x1 + drop * z, y1 + lift * z),
            );
            line.move_to(a.0, a.1)
                .curve_to(b.0, b.1, (a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0 + sag);
            window.stroke(&line, 0.9 * z.max(0.6), wire);
        }
    }
    // Cars parked at the kerb, in the colours of 1987.
    let paints = [
        0xc0463a_u32,
        0x2f4a8a,
        0xe8dcc4,
        0x3f6a4a,
        0xf2c14e,
        0x7a4b8a,
    ];
    let kerb = front + 4.0;
    let car_w = building_h * 0.62;
    let count = (width / (view_w * 0.3)) as i32;
    for index in 0..count {
        let spot = painter::hash2(index, 41, seed);
        if spot.is_multiple_of(3) {
            continue;
        }
        let x = (index as f32 + 0.3 + (spot % 100) as f32 / 250.0) * view_w * 0.3;
        let (sx, sy) = at(x, kerb);
        if !on_screen(sx) {
            continue;
        }
        let w = car_w * z;
        let paint = art::hex(paints[(spot >> 8) as usize % paints.len()]);
        paint_car(
            window,
            sx,
            sy + w * 0.2,
            w,
            paint,
            lit,
            spot.is_multiple_of(2),
        );
    }
}

/// A parked car seen side on, `w` wide, standing on `base`.
pub fn paint_car(
    window: &mut dyn Brush,
    x: f32,
    base: f32,
    w: f32,
    paint: Hsla,
    lit: bool,
    facing_left: bool,
) {
    let h = w * 0.36;
    let dir = if facing_left { -1.0 } else { 1.0 };
    window.soft(
        x,
        base,
        w * 0.55,
        h * 0.12,
        h * 0.15,
        gpui::black().opacity(0.2),
    );
    art::rect(
        window,
        x - w * 0.5,
        base - h * 0.75,
        w,
        h * 0.5,
        h * 0.12,
        paint,
    );
    art::polygon(
        window,
        &[
            (x - w * 0.28 * dir, base - h * 0.74),
            (x + w * 0.24 * dir, base - h * 0.74),
            (x + w * 0.14 * dir, base - h * 1.12),
            (x - w * 0.18 * dir, base - h * 1.12),
        ],
        paint,
    );
    let glass = if lit {
        art::hex(0x3a4250)
    } else {
        art::hex(0x9fb4c4)
    };
    art::polygon(
        window,
        &[
            (x - w * 0.22 * dir, base - h * 0.76),
            (x - 0.02 * w * dir, base - h * 0.76),
            (x - 0.02 * w * dir, base - h * 1.06),
            (x - w * 0.15 * dir, base - h * 1.06),
        ],
        glass,
    );
    art::polygon(
        window,
        &[
            (x + 0.02 * w * dir, base - h * 0.76),
            (x + w * 0.2 * dir, base - h * 0.76),
            (x + w * 0.12 * dir, base - h * 1.06),
            (x + 0.02 * w * dir, base - h * 1.06),
        ],
        glass,
    );
    art::rect(
        window,
        x - w * 0.5,
        base - h * 0.5,
        w,
        h * 0.06,
        0.0,
        art::hex(0xc8cdd2),
    );
    for side in [-0.3_f32, 0.3] {
        art::circle(
            window,
            x + side * w,
            base - h * 0.2,
            h * 0.24,
            art::hex(0x24222a),
        );
        art::circle(
            window,
            x + side * w,
            base - h * 0.2,
            h * 0.1,
            art::hex(0xc8cdd2),
        );
    }
    let lamp = if lit {
        art::hex(0xffe6a0)
    } else {
        art::hex(0xf2ead8)
    };
    art::rect(
        window,
        x + w * 0.46 * dir - w * 0.03,
        base - h * 0.66,
        w * 0.06,
        h * 0.12,
        1.0,
        lamp,
    );
}

/// Something to sit on under someone sitting down, `h` tall standing, with
/// their feet at (`x`, `y`): a bench in a harbour or on a street, a crate
/// on Mars, a block of snow on the ice. Drawn before them, so they sit on
/// it.
pub fn paint_seat(window: &mut dyn Brush, setting: Setting, x: f32, y: f32, h: f32, facing: f32) {
    let seat = y - h * 0.2;
    let back = x - facing * h * 0.14;
    let w = h * 0.36;
    window.soft(
        x,
        y,
        w * 0.7,
        h * 0.03,
        h * 0.04,
        gpui::black().opacity(0.16),
    );
    match setting {
        Setting::Harbour | Setting::Street => {
            let wood = art::hex(if setting == Setting::Street {
                0x6a4a36
            } else {
                0x8a5a36
            });
            let iron = art::hex(0x3a3f48);
            for dx in [-0.42_f32, 0.42] {
                art::rect(
                    window,
                    x + dx * w - h * 0.012,
                    seat,
                    h * 0.024,
                    y - seat,
                    1.0,
                    iron,
                );
            }
            art::rect(window, x - w * 0.5, seat - h * 0.03, w, h * 0.04, 1.0, wood);
            art::rect(
                window,
                back - h * 0.012,
                seat - h * 0.22,
                h * 0.024,
                h * 0.22,
                1.0,
                iron,
            );
            art::rect(
                window,
                back - facing * h * 0.01 - h * 0.02,
                seat - h * 0.2,
                h * 0.04,
                h * 0.14,
                1.0,
                art::shade(wood, 0.08),
            );
        }
        Setting::Mars => {
            let crate_ = art::hex(0xece6da);
            art::rect(
                window,
                x - w * 0.45,
                seat - h * 0.02,
                w * 0.9,
                y - seat + h * 0.02,
                2.0,
                crate_,
            );
            art::rect(
                window,
                x - w * 0.45,
                seat + (y - seat) * 0.45,
                w * 0.9,
                h * 0.025,
                0.0,
                art::hex(0xd9733b),
            );
        }
        Setting::Ice => {
            let block = art::hex(0xcfe6f0);
            art::rect(
                window,
                x - w * 0.45,
                seat - h * 0.02,
                w * 0.9,
                y - seat + h * 0.02,
                2.0,
                block,
            );
            art::rect(
                window,
                x - w * 0.45,
                seat - h * 0.02,
                w * 0.9,
                h * 0.03,
                1.0,
                art::hex(0xf6f9fb),
            );
        }
    }
}

/// Clouds for a setting: dust veils on Mars, the scenery's own otherwise.
pub fn cloud_ink(setting: Setting, cloud: Hsla, far: Hsla) -> Hsla {
    match setting {
        Setting::Mars => {
            crate::diorama::mix(cloud, art::shade(far, 0.35), 0.6).opacity(cloud.a * 0.55)
        }
        _ => cloud,
    }
}

/// Every drawing a setting's ground, street and seats are made of, as
/// names: for checking that nothing from the harbour stands elsewhere.
pub fn props(setting: Setting) -> &'static [&'static str] {
    match setting {
        Setting::Harbour => &[
            "grass",
            "wildflowers",
            "fence-posts",
            "reeds",
            "field-stones",
            "gulls",
            "chimney-smoke",
            "bench-seat",
        ],
        Setting::Mars => &[
            "rover-tracks",
            "craters",
            "rocks",
            "dust-ripples",
            "dust-veils",
            "crate-seat",
        ],
        Setting::Street => &[
            "sidewalk",
            "street-trees",
            "hydrants",
            "newspaper-boxes",
            "road",
            "utility-poles",
            "overhead-wires",
            "parked-cars",
            "bench-seat",
        ],
        Setting::Ice => &[
            "sastrugi",
            "ice-blocks",
            "pebbles",
            "sea-ice",
            "ice-shelf",
            "icicles",
            "snow-seat",
        ],
    }
}

/// The props only a harbour has: grass, flowers, fences, reeds, gulls.
pub const HARBOUR_ONLY: &[&str] = &[
    "grass",
    "wildflowers",
    "fence-posts",
    "reeds",
    "field-stones",
    "gulls",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_from_the_harbour_is_strewn_anywhere_else() {
        for setting in [Setting::Mars, Setting::Street, Setting::Ice] {
            for prop in props(setting) {
                assert!(!HARBOUR_ONLY.contains(prop), "{prop} in {setting:?}");
            }
            assert!(!setting.has_gulls(), "gulls over {setting:?}");
        }
        assert_eq!(Setting::Mars.water(), Some(false));
        assert_eq!(Setting::Street.water(), Some(false));
        assert!(!Setting::Mars.smokes());
    }
}
