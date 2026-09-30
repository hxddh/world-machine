//! Maple Street's drawings, 1987: brick storefronts under awnings and
//! neon, row houses, streetlamps, bus shelters, a drive-in screen and a
//! radio station's masts. Nothing here would stand on a shore.

use super::Pen;
use crate::art::{hex, shade};

fn brick(p: &Pen) -> gpui::Hsla {
    p.wall
}

/// A shopfront: a brick or clapboard face with a parapet, a wide window,
/// a door, an awning in `awning` and a sign board above.
fn shopfront(p: &mut Pen, tall: f32, awning: gpui::Hsla, sign: gpui::Hsla) {
    let wall = brick(p);
    p.rect(-0.44, 0.0, 0.44, tall, wall);
    p.rect(-0.46, tall, 0.46, tall + 0.04, shade(wall, -0.2));
    for step in 0..5 {
        let v = tall * 0.62 + step as f32 * 0.04;
        if v < tall - 0.02 {
            p.line((-0.44, v), (0.44, v), 0.004, shade(wall, -0.12));
        }
    }
    p.rect(-0.34, tall * 0.62, 0.34, tall * 0.62 + 0.12, sign);
    p.rect(-0.38, 0.06, 0.12, 0.42, p.glass);
    p.rect(-0.4, 0.0, 0.14, 0.06, shade(wall, -0.25));
    p.rect(0.18, 0.0, 0.34, 0.44, shade(p.trim, 0.1));
    p.rect(0.2, 0.2, 0.32, 0.4, p.glass);
    for step in 0..6 {
        let u = -0.42 + step as f32 * 0.14;
        let ink = if step % 2 == 0 { awning } else { p.k.pale };
        p.poly(
            &[
                (u, 0.56),
                (u + 0.14, 0.56),
                (u + 0.16, 0.46),
                (u + 0.02, 0.46),
            ],
            ink,
        );
    }
}

/// Upper windows in a row.
fn upper(p: &mut Pen, v0: f32, v1: f32, count: usize) {
    for index in 0..count {
        let u = -0.32 + 0.64 * index as f32 / (count.max(2) - 1) as f32;
        p.window(u - 0.06, v0, u + 0.06, v1);
    }
}

pub fn row_house(p: &mut Pen) {
    let wall = brick(p);
    p.rect(-0.42, 0.0, 0.42, 0.78, wall);
    p.rect(-0.44, 0.78, 0.44, 0.84, shade(wall, -0.2));
    p.poly(
        &[(-0.44, 0.84), (-0.3, 0.98), (0.3, 0.98), (0.44, 0.84)],
        p.roof,
    );
    upper(p, 0.5, 0.66, 3);
    p.window(-0.34, 0.14, -0.1, 0.36);
    p.rect(0.06, 0.0, 0.2, 0.36, shade(p.trim, 0.15));
    p.rect(0.02, 0.36, 0.3, 0.4, p.k.pale);
    p.rect(0.0, 0.0, 0.3, 0.04, p.k.stone);
    p.rect(0.26, 0.04, 0.28, 0.36, p.k.pale);
}

pub fn storefront(p: &mut Pen) {
    let (awning, sign) = (p.roof, p.k.dark);
    shopfront(p, 0.9, awning, sign);
    p.rect(-0.26, 0.64, 0.26, 0.68, p.k.brass);
}

pub fn water_tower(p: &mut Pen) {
    p.shadow(0.3);
    let metal = p.k.metal;
    for u in [-0.26, -0.08, 0.08, 0.26] {
        p.line((u * 1.3, 0.0), (u, 0.62), 0.018, metal);
    }
    p.line((-0.3, 0.3), (0.3, 0.3), 0.012, metal);
    p.line((-0.3, 0.0), (0.26, 0.62), 0.008, metal);
    p.line((0.3, 0.0), (-0.26, 0.62), 0.008, metal);
    p.rect(-0.3, 0.62, 0.3, 0.86, p.k.pale);
    p.rect(-0.3, 0.72, 0.3, 0.78, p.k.second);
    p.gable(-0.32, 0.32, 0.86, 0.98, p.k.metal);
}

pub fn bandshell(p: &mut Pen) {
    p.shadow(0.48);
    let white = p.k.pale;
    p.rect(-0.48, 0.0, 0.48, 0.14, shade(p.k.stone, -0.1));
    p.dome(0.0, 0.14, 0.42, 0.8, white);
    for (index, r) in [0.34, 0.26, 0.18].iter().enumerate() {
        p.dome(
            0.0,
            0.14,
            *r,
            r * 1.9,
            shade(white, -0.08 * (index + 1) as f32),
        );
    }
    p.dome(0.0, 0.14, 0.1, 0.2, p.k.dark);
    p.curve((-0.42, 0.14), (0.0, 1.1), (0.42, 0.14), 0.014, p.k.accent);
}

pub fn maple_tree(p: &mut Pen) {
    p.shadow(0.3);
    let trunk = shade(p.k.wood, -0.1);
    p.rect(-0.04, 0.0, 0.04, 0.5, trunk);
    let leaf = hex(0xc0463a);
    for (u, v, r) in [
        (-0.2, 0.6, 0.2),
        (0.2, 0.62, 0.2),
        (0.0, 0.78, 0.24),
        (-0.1, 0.5, 0.16),
        (0.14, 0.48, 0.14),
    ] {
        p.circ(u, v, r, shade(leaf, u * 0.4));
    }
    p.circ(0.06, 0.82, 0.1, shade(leaf, 0.15));
    p.rect(-0.14, 0.0, 0.14, 0.04, p.k.stone);
}

pub fn streetlamp(p: &mut Pen) {
    p.shadow(0.2);
    let metal = shade(p.k.metal, 0.1);
    p.rect(-0.06, 0.0, 0.06, 0.05, metal);
    p.rect(-0.02, 0.05, 0.02, 0.9, metal);
    p.curve((0.0, 0.88), (0.06, 0.97), (0.26, 0.94), 0.02, metal);
    p.rr(0.18, 0.9, 0.38, 0.95, 0.02, metal);
    p.ell(0.28, 0.9, 0.08, 0.015, hex(0xffb347));
}

pub fn steel_footbridge(p: &mut Pen) {
    let (metal, paint) = (p.k.metal, p.k.second);
    for u in [-0.4, 0.4] {
        p.rect(u - 0.04, 0.0, u + 0.04, 0.62, shade(p.k.stone, -0.1));
    }
    p.rect(-0.5, 0.62, 0.5, 0.7, paint);
    for step in 0..9 {
        let u = -0.48 + step as f32 * 0.12;
        p.line((u, 0.7), (u + 0.06, 0.94), 0.008, paint);
        p.line((u + 0.12, 0.7), (u + 0.06, 0.94), 0.008, paint);
    }
    p.rect(-0.5, 0.94, 0.5, 0.98, paint);
    for step in 0..5 {
        let v = 0.06 + step as f32 * 0.12;
        p.line((0.44, v), (0.5, v), 0.01, metal);
    }
}

pub fn lake_dock(p: &mut Pen) {
    let wood = p.k.wood;
    p.ell(0.0, 0.06, 0.52, 0.1, p.k.water.opacity(0.6));
    p.rect(-0.5, 0.5, 0.5, 0.62, wood);
    for step in 0..6 {
        let u = -0.46 + step as f32 * 0.184;
        p.rect(u - 0.025, 0.0, u + 0.025, 0.5, shade(wood, -0.25));
    }
    p.rect(0.3, 0.62, 0.34, 0.9, wood);
    p.circ(0.32, 0.86, 0.06, p.k.accent);
    p.circ(0.32, 0.86, 0.03, p.k.ground);
}

pub fn flower_bed(p: &mut Pen) {
    p.shadow(0.48);
    p.rect(-0.46, 0.0, 0.46, 0.26, p.k.stone);
    p.rect(-0.46, 0.22, 0.46, 0.26, shade(p.k.stone, 0.12));
    p.flowers(-0.42, 0.42, 0.26, 10, 0.04);
}

pub fn bus_shelter(p: &mut Pen) {
    p.shadow(0.48);
    let metal = p.k.metal;
    p.rect(-0.46, 0.0, -0.42, 0.8, metal);
    p.rect(0.3, 0.0, 0.34, 0.8, metal);
    p.rect(-0.42, 0.06, 0.3, 0.76, shade(p.k.water, 0.5).opacity(0.45));
    p.rect(-0.5, 0.8, 0.4, 0.88, p.roof);
    p.rect(-0.3, 0.26, 0.2, 0.3, p.k.wood);
    p.rect(0.38, 0.0, 0.41, 0.96, metal);
    p.circ(0.4, 0.94, 0.08, p.k.second);
    p.rect(0.36, 0.9, 0.44, 0.98, p.k.pale);
    p.rect(-0.3, 0.4, -0.02, 0.66, p.k.accent.opacity(0.85));
}

pub fn water_pump(p: &mut Pen) {
    p.shadow(0.3);
    let iron = p.k.second;
    p.rect(-0.14, 0.0, 0.14, 0.06, shade(p.k.stone, -0.1));
    p.rr(-0.08, 0.06, 0.08, 0.7, 0.03, iron);
    p.dome(0.0, 0.7, 0.1, 0.08, iron);
    p.line((0.06, 0.52), (0.26, 0.44), 0.03, iron);
    p.line((-0.02, 0.74), (-0.3, 0.92), 0.02, iron);
    p.ell(0.3, 0.06, 0.14, 0.04, p.k.water);
}

pub fn playground_swing(p: &mut Pen) {
    p.shadow(0.46);
    let (red, dark) = (p.k.accent, p.k.dark);
    p.line((-0.44, 0.0), (-0.36, 0.92), 0.03, red);
    p.line((0.44, 0.0), (0.36, 0.92), 0.03, red);
    p.line((-0.4, 0.92), (0.4, 0.92), 0.035, red);
    let swing = 0.06 * p.sway;
    for u in [-0.08, 0.08] {
        p.line((u, 0.9), (u + swing, 0.24), 0.008, dark);
    }
    p.rr(-0.12 + swing, 0.2, 0.12 + swing, 0.25, 0.02, p.k.dark);
    p.line((-0.28, 0.9), (-0.28, 0.4), 0.008, dark);
    p.circ(-0.28, 0.34, 0.06, p.k.brass);
}

pub fn drinking_fountain(p: &mut Pen) {
    p.shadow(0.24);
    let metal = p.k.brass;
    p.rect(-0.12, 0.0, 0.12, 0.06, shade(p.k.stone, -0.1));
    p.rect(-0.06, 0.06, 0.06, 0.62, shade(p.k.stone, 0.05));
    p.ell(0.0, 0.66, 0.2, 0.06, metal);
    p.dome(0.0, 0.66, 0.2, -0.08, shade(metal, -0.15));
    p.rect(-0.02, 0.66, 0.02, 0.76, metal);
    p.curve(
        (0.0, 0.76),
        (0.1, 0.86),
        (0.12, 0.7),
        0.012,
        shade(p.k.water, 0.4),
    );
}

pub fn street_sign(p: &mut Pen) {
    p.shadow(0.2);
    let green = hex(0x2f6b4f);
    p.rect(-0.02, 0.0, 0.02, 0.92, p.k.metal);
    p.rr(-0.3, 0.84, 0.3, 0.92, 0.01, green);
    p.line((-0.24, 0.88), (0.2, 0.88), 0.01, p.k.pale);
    p.rr(-0.06, 0.72, 0.36, 0.8, 0.01, green);
    p.line((0.0, 0.76), (0.3, 0.76), 0.01, p.k.pale);
    p.rect(-0.14, 0.42, 0.14, 0.62, p.k.pale);
    p.rect(-0.12, 0.44, 0.12, 0.6, hex(0xc0302a));
}

pub fn bird_feeder(p: &mut Pen) {
    p.shadow(0.2);
    p.rect(-0.02, 0.0, 0.02, 0.62, p.k.metal);
    p.line((0.0, 0.62), (0.2, 0.7), 0.012, p.k.metal);
    p.line((0.2, 0.7), (0.2, 0.62), 0.006, p.k.dark);
    p.rr(0.14, 0.4, 0.26, 0.62, 0.03, p.glass);
    p.rect(0.15, 0.42, 0.25, 0.5, p.k.brass);
    p.dome(0.2, 0.62, 0.08, 0.04, p.k.accent);
    p.rect(0.1, 0.38, 0.3, 0.4, p.k.metal);
    p.ell(0.32, 0.44, 0.04, 0.03, p.k.second);
}

pub fn window_boxes(p: &mut Pen) {
    p.shadow(0.46);
    let wall = brick(p);
    p.rect(-0.46, 0.0, 0.46, 0.9, wall);
    for u in [-0.22, 0.22] {
        p.window(u - 0.14, 0.4, u + 0.14, 0.8);
        p.rect(u - 0.17, 0.3, u + 0.17, 0.4, p.k.second);
        p.flowers(u - 0.15, u + 0.15, 0.4, 5, 0.04);
    }
}

/// The mayor in bronze on a plinth: a hat in one hand, a face beneath.
pub fn mayor_statue(p: &mut Pen) {
    p.shadow(0.3);
    let bronze = hex(0x6f8a70);
    let dark = shade(bronze, -0.4);
    p.rect(-0.26, 0.0, 0.26, 0.3, p.k.stone);
    p.rect(-0.3, 0.28, 0.3, 0.32, shade(p.k.stone, 0.12));
    p.rect(-0.14, 0.12, 0.14, 0.2, p.k.brass);
    p.poly(
        &[(-0.13, 0.32), (0.13, 0.32), (0.11, 0.74), (-0.11, 0.74)],
        bronze,
    );
    p.line((0.0, 0.74), (0.0, 0.5), 0.01, dark);
    p.circ(0.0, 0.82, 0.09, bronze);
    p.face(0.0, 0.82, 0.09, dark);
    p.line((0.1, 0.7), (0.24, 0.58), 0.03, bronze);
    p.rect(0.2, 0.52, 0.32, 0.56, dark);
    p.rect(0.23, 0.56, 0.29, 0.62, dark);
}

pub fn mailbox(p: &mut Pen) {
    p.shadow(0.3);
    let blue = hex(0x2f4a8a);
    for u in [-0.18, 0.18] {
        p.rect(u - 0.03, 0.0, u + 0.03, 0.2, blue);
    }
    p.rr(-0.24, 0.2, 0.24, 0.72, 0.03, blue);
    p.dome(0.0, 0.72, 0.24, 0.18, blue);
    p.rect(-0.14, 0.62, 0.14, 0.66, p.k.pale);
    p.rect(-0.14, 0.34, 0.14, 0.5, shade(blue, 0.2));
    p.rect(-0.06, 0.38, 0.06, 0.46, p.k.pale);
}

pub fn hot_dog_stand(p: &mut Pen) {
    p.shadow(0.42);
    let white = p.k.pale;
    p.rr(-0.4, 0.14, 0.34, 0.5, 0.04, white);
    p.rect(-0.4, 0.4, 0.34, 0.46, p.k.brass);
    p.ell(-0.04, 0.3, 0.14, 0.05, hex(0xc0683f));
    p.ell(-0.04, 0.3, 0.1, 0.02, p.k.brass);
    for u in [-0.3, 0.24] {
        p.circ(u, 0.1, 0.08, p.k.dark);
    }
    p.line((0.34, 0.3), (0.5, 0.36), 0.014, p.k.metal);
    p.line((-0.02, 0.5), (-0.02, 0.8), 0.012, p.k.metal);
    for step in 0..6 {
        let u = -0.38 + step as f32 * 0.12;
        let ink = if step % 2 == 0 { p.k.accent } else { white };
        p.poly(&[(u, 0.9), (u + 0.12, 0.9), (u + 0.12, 0.8), (u, 0.8)], ink);
    }
}

pub fn newspaper_bundle(p: &mut Pen) {
    p.shadow(0.3);
    let paper = hex(0xe8e0cc);
    for (v, off) in [(0.0, 0.0), (0.26, 0.03), (0.52, -0.02)] {
        p.rect(-0.26 + off, v, 0.26 + off, v + 0.24, paper);
        p.line(
            (-0.26 + off, v + 0.12),
            (0.26 + off, v + 0.12),
            0.01,
            shade(paper, -0.3),
        );
    }
    p.line((0.0, 0.0), (0.0, 0.76), 0.012, p.k.wood);
}

pub fn parked_car(p: &mut Pen) {
    let body = p.roof;
    p.ell(0.0, 0.02, 0.48, 0.05, gpui::black().opacity(0.15));
    p.rr(-0.48, 0.14, 0.48, 0.5, 0.06, body);
    p.poly(
        &[(-0.26, 0.5), (0.2, 0.5), (0.1, 0.86), (-0.18, 0.86)],
        body,
    );
    p.poly(
        &[(-0.2, 0.52), (-0.04, 0.52), (-0.04, 0.8), (-0.14, 0.8)],
        p.glass,
    );
    p.poly(
        &[(-0.01, 0.52), (0.16, 0.52), (0.08, 0.8), (-0.01, 0.8)],
        p.glass,
    );
    p.rect(-0.48, 0.24, 0.48, 0.28, p.k.brass);
    for u in [-0.3, 0.3] {
        p.circ(u, 0.14, 0.11, p.k.dark);
        p.circ(u, 0.14, 0.05, p.k.brass);
    }
    p.rect(0.42, 0.3, 0.48, 0.38, hex(0xffe0a0));
}

pub fn snack_bar(p: &mut Pen) {
    let (awning, sign) = (p.k.brass, p.k.accent);
    shopfront(p, 0.84, awning, sign);
    p.circ(-0.1, 0.9, 0.08, p.k.brass);
    p.circ(-0.1, 0.9, 0.04, hex(0xc0683f));
    p.rect(0.02, 0.86, 0.26, 0.94, p.k.second);
}

pub fn call_in_booth(p: &mut Pen) {
    p.shadow(0.24);
    let body = p.k.accent;
    p.rr(-0.2, 0.0, 0.2, 0.72, 0.03, body);
    p.rect(-0.14, 0.1, 0.14, 0.56, p.glass);
    p.rect(-0.2, 0.6, 0.2, 0.66, p.k.pale);
    p.rect(-0.04, 0.3, 0.04, 0.44, p.k.dark);
    p.rect(-0.02, 0.72, 0.02, 0.9, p.k.metal);
    p.circ(0.0, 0.94, 0.05, p.k.dark);
    for r in [0.1, 0.16] {
        p.curve(
            (-r, 0.94 + r * 0.2),
            (0.0, 0.94 + r * 0.9),
            (r, 0.94 + r * 0.2),
            0.008,
            p.k.second,
        );
    }
}

pub fn crosswalk(p: &mut Pen) {
    let road = p.k.ground;
    p.rect(-0.5, 0.0, 0.5, 0.16, road);
    for step in 0..6 {
        let u = -0.42 + step as f32 * 0.16;
        p.poly(
            &[
                (u, 0.02),
                (u + 0.08, 0.02),
                (u + 0.06, 0.14),
                (u - 0.02, 0.14),
            ],
            p.k.pale,
        );
    }
    for u in [-0.46, 0.46] {
        p.rect(u - 0.012, 0.0, u + 0.012, 0.8, p.k.metal);
        p.circ(u, 0.84, 0.05, p.k.brass);
    }
    p.rr(0.36, 0.52, 0.56, 0.7, 0.02, p.k.dark);
    p.rect(0.4, 0.56, 0.52, 0.66, hex(0xffe0a0));
}

pub fn mural_wall(p: &mut Pen) {
    let wall = shade(p.k.stone, -0.1);
    p.rect(-0.5, 0.0, 0.5, 0.86, wall);
    p.curve(
        (-0.5, 0.86),
        (0.0, 0.7),
        (0.5, 0.86),
        0.04,
        shade(wall, -0.2),
    );
    p.rect(-0.44, 0.06, 0.44, 0.66, p.k.second);
    p.circ(-0.2, 0.46, 0.14, p.k.brass);
    p.poly(
        &[
            (-0.44, 0.06),
            (-0.1, 0.36),
            (0.1, 0.2),
            (0.44, 0.5),
            (0.44, 0.06),
        ],
        p.k.accent,
    );
    p.line((0.0, 0.54), (0.36, 0.6), 0.02, p.k.pale);
}

pub fn splash_pool(p: &mut Pen) {
    p.ell(0.0, 0.2, 0.48, 0.18, p.k.stone);
    p.ell(0.0, 0.22, 0.42, 0.13, shade(p.k.water, 0.3));
    for u in [-0.2, 0.0, 0.2] {
        p.curve(
            (u, 0.24),
            (u + 0.04, 0.7),
            (u + 0.1, 0.3),
            0.012,
            shade(p.k.water, 0.55),
        );
    }
    p.circ(0.3, 0.3, 0.05, p.k.accent);
}

pub fn roller_floor(p: &mut Pen) {
    p.poly(
        &[(-0.5, 0.0), (0.5, 0.0), (0.4, 0.34), (-0.4, 0.34)],
        shade(p.k.ground, 0.3),
    );
    p.ell(0.0, 0.17, 0.3, 0.1, shade(p.k.ground, 0.4));
    for (u, ink) in [(-0.3, p.k.accent), (0.2, p.k.second)] {
        p.rr(u - 0.06, 0.4, u + 0.06, 0.6, 0.02, ink);
        p.rect(u - 0.08, 0.36, u + 0.1, 0.42, ink);
        for w in [-0.05, 0.07] {
            p.circ(u + w, 0.34, 0.025, p.k.brass);
        }
    }
    for u in [-0.46, 0.46] {
        p.rect(u - 0.01, 0.0, u + 0.01, 0.62, p.k.metal);
    }
    p.pennants((-0.46, 0.6), (0.46, 0.6), 6, &[p.k.accent, p.k.second]);
}

pub fn darkroom(p: &mut Pen) {
    let wall = brick(p);
    p.rect(-0.44, 0.0, 0.44, 0.76, wall);
    p.rect(-0.46, 0.76, 0.46, 0.82, shade(wall, -0.2));
    p.rect(-0.34, 0.2, 0.0, 0.52, p.glass);
    p.rect(-0.34, 0.2, 0.0, 0.52, hex(0xc0302a).opacity(0.5));
    p.rect(0.12, 0.0, 0.3, 0.46, p.k.dark);
    p.rr(0.14, 0.5, 0.28, 0.58, 0.02, hex(0xc0302a));
    p.line((-0.3, 0.44), (-0.04, 0.44), 0.006, p.k.dark);
    for u in [-0.26, -0.16, -0.06] {
        p.rect(u - 0.03, 0.34, u + 0.03, 0.43, p.k.pale);
    }
}

pub fn clubhouse(p: &mut Pen) {
    let wood = shade(p.k.wood, 0.1);
    for u in [-0.36, 0.3] {
        p.rect(u - 0.03, 0.0, u + 0.03, 0.3, shade(wood, -0.2));
    }
    p.rect(-0.4, 0.3, 0.34, 0.74, wood);
    for step in 0..5 {
        let v = 0.36 + step as f32 * 0.08;
        p.line((-0.4, v), (0.34, v), 0.005, shade(wood, -0.25));
    }
    p.poly(
        &[(-0.46, 0.72), (0.4, 0.8), (0.4, 0.74), (-0.46, 0.66)],
        p.k.second,
    );
    p.rect(-0.3, 0.46, -0.06, 0.62, p.glass);
    p.rect(0.04, 0.3, 0.2, 0.6, shade(wood, -0.3));
    p.rect(-0.2, 0.64, 0.18, 0.7, p.k.accent);
    for step in 0..5 {
        p.line(
            (0.42, step as f32 * 0.07),
            (0.5, step as f32 * 0.07),
            0.01,
            wood,
        );
    }
    p.line((0.42, 0.0), (0.42, 0.32), 0.01, wood);
    p.line((0.5, 0.0), (0.5, 0.32), 0.01, wood);
}

pub fn tall_antenna(p: &mut Pen) {
    p.shadow(0.3);
    let (red, white) = (p.k.accent, p.k.pale);
    for step in 0..8 {
        let v = step as f32 * 0.115;
        let s = 0.16 - v * 0.13;
        let ink = if step % 2 == 0 { red } else { white };
        p.poly(
            &[
                (-s, v),
                (s, v),
                (s - 0.018, v + 0.115),
                (-s + 0.018, v + 0.115),
            ],
            ink,
        );
        p.line((-s, v), (s - 0.018, v + 0.115), 0.005, shade(ink, -0.3));
    }
    p.line((0.0, 0.92), (0.0, 1.0), 0.012, red);
    p.light(0.0, 1.0, 0.02, red);
    p.line((-0.45, 0.0), (0.0, 0.7), 0.004, p.k.dark);
}

pub fn garden_plots(p: &mut Pen) {
    p.shadow(0.48);
    for (u, ink) in [(-0.3, p.k.leaf), (0.0, hex(0xc0463a)), (0.3, p.k.brass)] {
        p.rect(u - 0.13, 0.0, u + 0.13, 0.16, shade(p.k.wood, -0.1));
        for step in 0..3 {
            let a = u - 0.08 + step as f32 * 0.08;
            p.line((a, 0.16), (a, 0.46), 0.01, p.k.wood);
            p.circ(a, 0.3, 0.04, p.k.leaf);
            p.circ(a + 0.02, 0.36, 0.025, ink);
        }
    }
    p.line((-0.48, 0.5), (0.48, 0.5), 0.0, p.k.leaf);
}

pub fn park_stage(p: &mut Pen) {
    p.shadow(0.48);
    let wood = p.k.wood;
    p.rect(-0.48, 0.0, 0.48, 0.26, shade(wood, -0.1));
    p.rect(-0.48, 0.24, 0.48, 0.28, shade(wood, 0.15));
    for u in [-0.44, 0.44] {
        p.rect(u - 0.02, 0.28, u + 0.02, 0.9, p.k.metal);
    }
    p.rect(-0.46, 0.86, 0.46, 0.94, p.k.metal);
    for u in [-0.3, 0.0, 0.3] {
        p.rr(u - 0.05, 0.8, u + 0.05, 0.86, 0.02, p.k.dark);
        p.poly(
            &[
                (u - 0.06, 0.8),
                (u + 0.06, 0.8),
                (u + 0.14, 0.28),
                (u - 0.14, 0.28),
            ],
            hex(0xfff2c0).opacity(0.25),
        );
    }
    p.rect(0.24, 0.28, 0.36, 0.48, p.k.dark);
    p.circ(0.3, 0.38, 0.04, shade(p.k.dark, 0.3));
}

pub fn streetlights(p: &mut Pen) {
    let metal = shade(p.k.metal, 0.1);
    for (u, tall) in [(-0.36, 0.84), (0.0, 0.92), (0.36, 0.84)] {
        p.rect(u - 0.015, 0.0, u + 0.015, tall, metal);
        p.line((u, tall), (u + 0.12, tall + 0.04), 0.014, metal);
        p.rr(u + 0.08, tall + 0.0, u + 0.2, tall + 0.05, 0.02, metal);
        p.circ(u + 0.14, tall - 0.005, 0.03, hex(0xffb347));
    }
}

pub fn video_store(p: &mut Pen) {
    let (awning, sign) = (p.k.second, hex(0x2a3a8a));
    shopfront(p, 0.88, awning, sign);
    p.rect(-0.3, 0.66, -0.08, 0.72, p.k.brass);
    p.rect(-0.04, 0.66, 0.26, 0.72, p.k.accent);
    for step in 0..5 {
        let u = -0.34 + step as f32 * 0.09;
        p.rect(
            u,
            0.1,
            u + 0.06,
            0.2,
            [p.k.accent, p.k.brass, p.k.second][step % 3],
        );
    }
}

pub fn dance_floor(p: &mut Pen) {
    p.shadow(0.48);
    let inks = [p.k.accent, p.k.second, p.k.brass, p.k.pale];
    for row in 0..2 {
        for col in 0..6 {
            let u = -0.48 + col as f32 * 0.16 + row as f32 * 0.04;
            let v = row as f32 * 0.1;
            p.poly(
                &[(u, v), (u + 0.16, v), (u + 0.16, v + 0.1), (u, v + 0.1)],
                inks[(row + col) % 4].opacity(0.85),
            );
        }
    }
    p.line((0.0, 1.0), (0.0, 0.86), 0.006, p.k.dark);
    p.circ(0.0, 0.8, 0.07, hex(0xc8cdd2));
    for (u, v) in [(-0.03, 0.83), (0.03, 0.78), (-0.02, 0.77)] {
        p.circ(u, v, 0.012, p.k.pale);
    }
    for u in [-0.3, 0.3] {
        p.poly(
            &[(0.0, 0.8), (u - 0.06, 0.2), (u + 0.06, 0.2)],
            hex(0xfff2c0).opacity(0.18),
        );
    }
}

pub fn record_library(p: &mut Pen) {
    let wall = brick(p);
    p.rect(-0.44, 0.0, 0.44, 0.84, wall);
    p.rect(-0.46, 0.84, 0.46, 0.9, shade(wall, -0.2));
    p.rect(-0.36, 0.12, 0.36, 0.56, p.glass);
    for step in 0..12 {
        let u = -0.34 + step as f32 * 0.057;
        p.rect(
            u,
            0.14,
            u + 0.05,
            0.32,
            [p.k.accent, p.k.dark, p.k.brass, p.k.second][step % 4],
        );
    }
    for (u, ink) in [(-0.2, p.k.dark), (0.14, p.k.dark)] {
        p.circ(u, 0.44, 0.08, ink);
        p.circ(u, 0.44, 0.025, p.k.accent);
    }
    p.circ(0.0, 0.7, 0.08, p.k.brass);
    p.circ(0.0, 0.7, 0.03, p.k.dark);
    p.rect(-0.02, 0.9, 0.02, 0.94, p.k.metal);
    p.circ(0.0, 0.98, 0.07, p.k.dark);
    p.circ(0.0, 0.98, 0.02, p.k.accent);
}

pub fn basketball_court(p: &mut Pen) {
    p.poly(
        &[(-0.5, 0.0), (0.5, 0.0), (0.4, 0.16), (-0.4, 0.16)],
        shade(p.k.ground, 0.15),
    );
    p.curve((-0.2, 0.0), (0.0, 0.2), (0.2, 0.0), 0.008, p.k.pale);
    p.rect(0.3, 0.0, 0.34, 0.8, p.k.metal);
    p.rect(0.12, 0.72, 0.34, 0.94, p.k.pale);
    p.rect(0.16, 0.76, 0.3, 0.86, shade(p.k.pale, -0.15));
    p.ell(0.1, 0.74, 0.07, 0.015, p.k.accent);
    for step in 0..3 {
        let u = 0.05 + step as f32 * 0.05;
        p.line((u, 0.74), (u + 0.02, 0.64), 0.006, p.k.pale);
    }
    p.circ(-0.2, 0.3, 0.06, hex(0xd9733b));
}

pub fn pay_phone(p: &mut Pen) {
    p.shadow(0.24);
    let metal = hex(0xc8cdd2);
    p.rect(-0.03, 0.0, 0.03, 0.5, p.k.metal);
    p.rr(-0.2, 0.46, 0.2, 0.9, 0.03, metal);
    p.rect(-0.2, 0.84, 0.2, 0.9, p.k.second);
    p.rr(-0.12, 0.54, 0.02, 0.8, 0.02, p.k.dark);
    p.rr(-0.1, 0.56, 0.0, 0.64, 0.02, shade(p.k.dark, 0.3));
    for row in 0..3 {
        for col in 0..2 {
            p.rect(
                0.06 + col as f32 * 0.05,
                0.6 + row as f32 * 0.06,
                0.09 + col as f32 * 0.05,
                0.63 + row as f32 * 0.06,
                p.k.dark,
            );
        }
    }
}

pub fn drive_in_screen(p: &mut Pen) {
    let metal = p.k.metal;
    for u in [-0.34, 0.34] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.4, metal);
        p.line((u, 0.0), (u - 0.1, 0.38), 0.01, metal);
    }
    p.rect(-0.48, 0.4, 0.48, 0.96, p.k.pale);
    p.rect(-0.44, 0.44, 0.44, 0.92, shade(p.k.pale, -0.05));
    p.rect(-0.48, 0.96, 0.48, 1.0, p.k.accent);
    p.circ(0.1, 0.72, 0.1, p.k.brass.opacity(0.5));
}

pub fn study_room(p: &mut Pen) {
    let wall = p.k.pale;
    p.rect(-0.44, 0.0, 0.44, 0.76, wall);
    p.poly(&[(-0.48, 0.76), (0.0, 0.94), (0.48, 0.76)], p.roof);
    for u in [-0.26, 0.0, 0.26] {
        p.arch(u - 0.08, u + 0.08, 0.28, 0.62, p.glass);
        p.rect(u - 0.06, 0.36, u + 0.06, 0.38, p.k.dark);
        p.circ(u + 0.03, 0.42, 0.02, hex(0x3f9a5a));
    }
    p.rect(-0.1, 0.0, 0.1, 0.22, shade(p.trim, 0.1));
}

pub fn median_beds(p: &mut Pen) {
    p.rect(-0.5, 0.0, 0.5, 0.12, p.k.stone);
    p.rect(-0.5, 0.0, 0.5, 0.03, p.k.brass);
    for step in 0..4 {
        let u = -0.36 + step as f32 * 0.24;
        p.circ(u, 0.24, 0.1, p.k.leaf);
    }
    p.flowers(-0.44, 0.44, 0.12, 12, 0.03);
    p.rect(0.3, 0.12, 0.33, 0.8, p.k.metal);
    p.circ(0.315, 0.84, 0.06, shade(p.k.leaf, -0.1));
}

pub fn soapbox_track(p: &mut Pen) {
    p.poly(
        &[(-0.5, 0.64), (-0.3, 0.64), (0.5, 0.0), (0.2, 0.0)],
        shade(p.k.ground, 0.2),
    );
    for step in 0..4 {
        let t = step as f32 / 4.0;
        p.rect(
            -0.42 + t * 0.78,
            0.6 - t * 0.64,
            -0.38 + t * 0.78,
            0.66 - t * 0.64,
            p.k.accent,
        );
    }
    p.rr(0.0, 0.2, 0.22, 0.32, 0.03, p.k.brass);
    for u in [0.03, 0.19] {
        p.circ(u, 0.18, 0.04, p.k.dark);
    }
    p.circ(0.08, 0.36, 0.035, p.k.accent);
    p.rect(-0.48, 0.64, -0.44, 0.98, p.k.metal);
    p.poly(&[(-0.44, 0.98), (-0.26, 0.94), (-0.44, 0.88)], p.k.pale);
}

pub fn neon_sign(p: &mut Pen) {
    p.shadow(0.2);
    let pink = p.k.accent;
    p.rect(-0.03, 0.0, 0.03, 0.4, p.k.metal);
    p.rr(-0.36, 0.4, 0.36, 0.92, 0.04, p.k.dark);
    p.rr(-0.32, 0.44, 0.32, 0.88, 0.04, pink.opacity(0.2));
    p.curve((-0.24, 0.52), (-0.24, 0.82), (0.0, 0.8), 0.02, pink);
    p.curve((0.0, 0.8), (0.24, 0.82), (0.24, 0.52), 0.02, p.k.second);
    p.line((-0.2, 0.62), (0.2, 0.62), 0.02, p.k.brass);
    p.poly(&[(0.0, 0.76), (0.06, 0.66), (-0.06, 0.66)], p.k.brass);
}

pub fn diner_board(p: &mut Pen) {
    p.shadow(0.3);
    for u in [-0.2, 0.2] {
        p.line((u * 1.4, 0.0), (u * 0.5, 0.8), 0.02, p.k.wood);
    }
    p.rect(-0.26, 0.2, 0.26, 0.78, hex(0x2a2a2a));
    p.rect(-0.28, 0.76, 0.28, 0.82, p.k.wood);
    for (v, w) in [(0.66, 0.36), (0.56, 0.26), (0.46, 0.32), (0.36, 0.2)] {
        p.line((-0.18, v), (-0.18 + w, v), 0.012, p.k.pale);
    }
    p.circ(0.14, 0.3, 0.03, p.k.accent);
}

pub fn radio_van(p: &mut Pen) {
    p.ell(0.0, 0.02, 0.48, 0.05, gpui::black().opacity(0.15));
    let body = p.k.pale;
    p.rr(-0.48, 0.14, 0.4, 0.66, 0.05, body);
    p.poly(&[(0.4, 0.14), (0.5, 0.14), (0.5, 0.42), (0.4, 0.58)], body);
    p.poly(&[(0.41, 0.42), (0.48, 0.4), (0.41, 0.54)], p.glass);
    p.rect(-0.48, 0.34, 0.4, 0.42, p.k.accent);
    p.rect(-0.3, 0.44, 0.1, 0.6, p.k.second);
    for u in [-0.3, 0.3] {
        p.circ(u, 0.14, 0.1, p.k.dark);
        p.circ(u, 0.14, 0.04, hex(0xc8cdd2));
    }
    p.line((-0.2, 0.66), (-0.2, 0.96), 0.014, p.k.metal);
    p.ell(-0.12, 0.96, 0.1, 0.04, hex(0xc8cdd2));
}

pub fn youth_centre(p: &mut Pen) {
    let wall = brick(p);
    p.rect(-0.48, 0.0, 0.48, 0.66, wall);
    p.rect(-0.48, 0.66, 0.48, 0.72, shade(wall, -0.2));
    p.rect(-0.3, 0.72, 0.3, 0.9, p.k.second);
    p.rect(-0.26, 0.76, 0.26, 0.86, p.k.pale);
    upper(p, 0.38, 0.56, 4);
    p.rect(-0.12, 0.0, 0.12, 0.3, p.glass);
    p.line((0.0, 0.0), (0.0, 0.3), 0.01, p.k.metal);
    p.rect(-0.46, 0.08, -0.26, 0.3, p.k.accent.opacity(0.7));
}

pub fn winter_rink(p: &mut Pen) {
    p.ell(0.0, 0.2, 0.5, 0.18, p.k.pale);
    p.ell(0.0, 0.2, 0.44, 0.13, hex(0xd8ecf2));
    p.curve((-0.3, 0.2), (0.0, 0.3), (0.3, 0.16), 0.006, hex(0xa8c8d8));
    for step in 0..8 {
        let u = -0.46 + step as f32 * 0.13;
        p.rect(u - 0.01, 0.2, u + 0.01, 0.46, p.k.wood);
    }
    p.pennants(
        (-0.46, 0.46),
        (0.46, 0.46),
        8,
        &[p.k.accent, p.k.second, p.k.brass],
    );
}

pub fn party_speakers(p: &mut Pen) {
    p.shadow(0.4);
    for (u, v) in [(-0.2, 0.0), (0.2, 0.0), (0.0, 0.46)] {
        p.rr(u - 0.18, v, u + 0.18, v + 0.44, 0.02, p.k.dark);
        p.circ(u, v + 0.14, 0.1, shade(p.k.dark, 0.25));
        p.circ(u, v + 0.14, 0.04, p.k.metal);
        p.circ(u, v + 0.34, 0.05, shade(p.k.dark, 0.25));
    }
    for r in [0.1, 0.18] {
        p.curve(
            (0.24, 0.9 - r),
            (0.24 + r, 0.9),
            (0.24, 0.9 + r * 0.5),
            0.01,
            p.k.accent,
        );
    }
}

pub fn computer_room(p: &mut Pen) {
    let wall = brick(p);
    p.rect(-0.46, 0.0, 0.46, 0.8, wall);
    p.rect(-0.48, 0.8, 0.48, 0.86, shade(wall, -0.2));
    p.rect(-0.38, 0.2, 0.38, 0.62, p.glass);
    for u in [-0.24, 0.0, 0.24] {
        p.rr(u - 0.08, 0.3, u + 0.08, 0.44, 0.01, hex(0xd8d0bc));
        p.rect(u - 0.06, 0.32, u + 0.06, 0.42, hex(0x3f7a5a));
        p.rect(u - 0.1, 0.24, u + 0.1, 0.28, hex(0xd8d0bc));
    }
    p.rect(-0.1, 0.0, 0.1, 0.16, shade(p.trim, 0.1));
}

pub fn walk_of_fame(p: &mut Pen) {
    p.rect(-0.5, 0.0, 0.5, 0.6, shade(p.k.ground, 0.35));
    for (index, u) in [-0.32, 0.0, 0.32].iter().enumerate() {
        let v = 0.3;
        p.rect(
            u - 0.12,
            v - 0.2,
            u + 0.12,
            v + 0.2,
            if index % 2 == 0 {
                p.k.accent
            } else {
                hex(0x2a2a2a)
            },
        );
        let star = [
            (0.0, 0.14),
            (0.04, 0.04),
            (0.13, 0.04),
            (0.06, -0.03),
            (0.08, -0.13),
            (0.0, -0.06),
            (-0.08, -0.13),
            (-0.06, -0.03),
            (-0.13, 0.04),
            (-0.04, 0.04),
        ];
        let points = star
            .iter()
            .map(|(a, b)| (u + a * 0.8, v + b * 1.2))
            .collect::<Vec<_>>();
        p.poly(&points, p.k.brass);
    }
}

pub fn school_greenhouse(p: &mut Pen) {
    p.shadow(0.46);
    let glass = shade(p.k.water, 0.55).opacity(0.7);
    p.rect(-0.44, 0.0, 0.44, 0.14, brick(p));
    p.rect(-0.44, 0.14, 0.44, 0.6, glass);
    p.poly(&[(-0.46, 0.6), (0.46, 0.6), (0.3, 0.9), (-0.3, 0.9)], glass);
    for step in 0..5 {
        let u = -0.34 + step as f32 * 0.17;
        p.circ(u, 0.24, 0.06, p.k.leaf);
        p.line((u, 0.14), (u, 0.6), 0.008, p.k.pale);
    }
    p.rect(-0.2, 0.62, 0.2, 0.7, p.k.brass);
}

pub fn bus_depot(p: &mut Pen) {
    let wall = shade(brick(p), 0.05);
    p.rect(-0.48, 0.0, 0.48, 0.7, wall);
    p.curve((-0.48, 0.7), (0.0, 0.96), (0.48, 0.7), 0.06, p.k.metal);
    for u in [-0.24, 0.24] {
        p.rect(u - 0.18, 0.0, u + 0.18, 0.5, p.k.dark);
    }
    p.rr(0.08, 0.02, 0.42, 0.34, 0.03, p.k.brass);
    p.rect(0.12, 0.2, 0.38, 0.3, p.glass);
    p.circ(0.16, 0.02, 0.04, p.k.dark);
    p.circ(0.36, 0.02, 0.04, p.k.dark);
    p.rect(-0.3, 0.56, 0.3, 0.64, p.k.second);
}

pub fn time_capsule(p: &mut Pen) {
    p.shadow(0.3);
    let stone = p.k.stone;
    p.rect(-0.3, 0.0, 0.3, 0.14, shade(stone, -0.1));
    p.rr(-0.22, 0.14, 0.22, 0.62, 0.04, stone);
    p.rect(-0.16, 0.28, 0.16, 0.5, p.k.brass);
    p.line((-0.1, 0.44), (0.1, 0.44), 0.01, shade(p.k.brass, -0.4));
    p.line((-0.08, 0.36), (0.08, 0.36), 0.01, shade(p.k.brass, -0.4));
    p.dome(0.0, 0.62, 0.22, 0.1, shade(stone, 0.1));
    p.circ(0.0, 0.76, 0.05, p.k.accent);
}

pub fn dog_run(p: &mut Pen) {
    p.shadow(0.48);
    let metal = p.k.metal;
    for step in 0..6 {
        let u = -0.46 + step as f32 * 0.184;
        p.rect(u - 0.01, 0.0, u + 0.01, 0.62, metal);
    }
    p.line((-0.46, 0.6), (0.46, 0.6), 0.01, metal);
    for step in 0..9 {
        let u = -0.46 + step as f32 * 0.115;
        p.line((u, 0.0), (u + 0.115, 0.6), 0.004, metal.opacity(0.6));
        p.line((u + 0.115, 0.0), (u, 0.6), 0.004, metal.opacity(0.6));
    }
    p.ell(0.0, 0.14, 0.12, 0.08, p.k.wood);
    p.circ(0.12, 0.24, 0.06, p.k.wood);
    p.line((-0.12, 0.14), (-0.2, 0.24), 0.02, p.k.wood);
    p.circ(-0.26, 0.06, 0.04, p.k.accent);
}

pub fn rialto_marquee(p: &mut Pen) {
    p.shadow(0.3);
    let wall = brick(p);
    p.rect(-0.44, 0.0, 0.44, 0.5, wall);
    p.rect(-0.2, 0.0, 0.2, 0.34, p.glass);
    p.poly(
        &[(-0.48, 0.5), (0.48, 0.5), (0.44, 0.66), (-0.44, 0.66)],
        p.k.dark,
    );
    p.rect(-0.4, 0.53, 0.4, 0.63, p.k.pale);
    for step in 0..9 {
        let u = -0.4 + step as f32 * 0.1;
        p.circ(u, 0.52, 0.012, p.k.brass);
        p.circ(u, 0.64, 0.012, p.k.brass);
    }
    p.rect(-0.08, 0.66, 0.08, 1.0, p.k.accent);
    for step in 0..5 {
        p.circ(0.0, 0.7 + step as f32 * 0.06, 0.03, p.k.brass);
    }
}

pub fn lake_path(p: &mut Pen) {
    p.ell(0.26, 0.2, 0.26, 0.14, p.k.water);
    p.poly(
        &[(-0.5, 0.0), (-0.3, 0.0), (0.2, 0.44), (0.06, 0.44)],
        shade(p.k.ground, 0.3),
    );
    p.line((-0.4, 0.0), (0.13, 0.44), 0.006, p.k.brass);
    for (u, v) in [(-0.3, 0.3), (0.4, 0.5), (-0.1, 0.62)] {
        p.circ(u, v, 0.08, p.k.leaf);
    }
    p.circ(-0.22, 0.12, 0.05, p.k.dark);
    p.circ(-0.1, 0.12, 0.05, p.k.dark);
    p.line((-0.22, 0.12), (-0.1, 0.12), 0.012, p.k.accent);
}

pub fn street_gazebo(p: &mut Pen) {
    p.shadow(0.44);
    let white = p.k.pale;
    p.rect(-0.42, 0.0, 0.42, 0.08, p.k.stone);
    for u in [-0.36, -0.12, 0.12, 0.36] {
        p.rect(u - 0.015, 0.08, u + 0.015, 0.56, white);
    }
    p.rect(-0.4, 0.24, 0.4, 0.26, white);
    p.rect(-0.44, 0.56, 0.44, 0.6, p.k.second);
    p.poly(
        &[
            (-0.48, 0.6),
            (-0.1, 0.84),
            (0.0, 0.98),
            (0.1, 0.84),
            (0.48, 0.6),
        ],
        p.roof,
    );
    p.pennants(
        (-0.36, 0.52),
        (0.36, 0.52),
        7,
        &[p.k.accent, p.k.pale, p.k.second],
    );
}

pub fn recording_studio(p: &mut Pen) {
    let wall = brick(p);
    p.rect(-0.44, 0.0, 0.44, 0.8, wall);
    p.rect(-0.46, 0.8, 0.46, 0.86, shade(wall, -0.2));
    p.rr(-0.2, 0.54, 0.2, 0.72, 0.04, hex(0xc0302a));
    p.line((-0.12, 0.63), (0.12, 0.63), 0.02, p.k.pale);
    p.rect(-0.1, 0.0, 0.1, 0.4, p.k.dark);
    p.porthole(0.0, 0.3, 0.05);
    for u in [-0.3, 0.3] {
        p.window(u - 0.07, 0.2, u + 0.07, 0.42);
    }
}

pub fn market_stalls(p: &mut Pen) {
    p.shadow(0.48);
    for (u, ink) in [(-0.25, p.k.accent), (0.25, p.k.second)] {
        for side in [-0.2, 0.2] {
            p.rect(u + side - 0.015, 0.0, u + side + 0.015, 0.72, p.k.metal);
        }
        p.rect(u - 0.22, 0.3, u + 0.22, 0.36, p.k.wood);
        for step in 0..3 {
            p.circ(
                u - 0.12 + step as f32 * 0.12,
                0.4,
                0.05,
                [p.k.brass, hex(0xc0463a), p.k.leaf][step],
            );
        }
        p.poly(
            &[
                (u - 0.26, 0.72),
                (u + 0.26, 0.72),
                (u + 0.22, 0.86),
                (u - 0.22, 0.86),
            ],
            ink,
        );
    }
}

pub fn bank_clock(p: &mut Pen) {
    let stone = p.k.stone;
    p.rect(-0.4, 0.0, 0.4, 0.6, stone);
    for u in [-0.3, -0.1, 0.1, 0.3] {
        p.rect(u - 0.035, 0.04, u + 0.035, 0.52, shade(stone, 0.12));
    }
    p.rect(-0.44, 0.52, 0.44, 0.6, shade(stone, -0.15));
    p.gable(-0.44, 0.44, 0.6, 0.72, shade(stone, 0.05));
    p.rect(-0.03, 0.72, 0.03, 0.78, p.k.metal);
    p.circ(0.0, 0.86, 0.1, p.k.brass);
    p.circ(0.0, 0.86, 0.08, p.k.pale);
    p.line((0.0, 0.86), (0.0, 0.92), 0.012, p.k.dark);
    p.line((0.0, 0.86), (0.05, 0.85), 0.014, p.k.dark);
}

pub fn skate_park(p: &mut Pen) {
    let concrete = shade(p.k.ground, 0.4);
    p.poly(
        &[
            (-0.5, 0.0),
            (0.5, 0.0),
            (0.5, 0.7),
            (0.36, 0.7),
            (0.3, 0.2),
            (0.0, 0.08),
            (-0.3, 0.2),
            (-0.36, 0.7),
            (-0.5, 0.7),
        ],
        concrete,
    );
    p.line((-0.5, 0.7), (-0.36, 0.7), 0.02, p.k.metal);
    p.line((0.36, 0.7), (0.5, 0.7), 0.02, p.k.metal);
    p.poly(
        &[
            (0.0, 0.3),
            (0.1, 0.34),
            (0.1, 0.36),
            (-0.1, 0.36),
            (-0.1, 0.34),
        ],
        p.k.accent,
    );
    p.circ(-0.06, 0.28, 0.02, p.k.dark);
    p.circ(0.06, 0.28, 0.02, p.k.dark);
    p.line((-0.2, 0.3), (-0.26, 0.5), 0.006, p.k.second);
}

pub fn memorial_bench(p: &mut Pen) {
    p.shadow(0.45);
    let (wood, metal) = (p.k.wood, p.k.dark);
    for u in [-0.36, 0.36] {
        p.line((u, 0.0), (u, 0.44), 0.035, metal);
    }
    p.rect(-0.46, 0.4, 0.46, 0.48, wood);
    for v in [0.6, 0.72] {
        p.rect(-0.44, v, 0.44, v + 0.08, shade(wood, 0.05));
    }
    p.rect(-0.12, 0.62, 0.12, 0.7, p.k.brass);
    p.flowers(0.36, 0.5, 0.0, 3, 0.04);
    p.rect(-0.37, 0.48, -0.34, 0.84, metal);
    p.rect(0.34, 0.48, 0.37, 0.84, metal);
}

pub fn arcade_upstairs(p: &mut Pen) {
    let wall = brick(p);
    p.rect(-0.44, 0.0, 0.44, 0.92, wall);
    p.rect(-0.46, 0.92, 0.46, 0.96, shade(wall, -0.2));
    p.rect(-0.38, 0.52, 0.38, 0.82, p.glass);
    for u in [-0.24, 0.0, 0.24] {
        p.rect(u - 0.06, 0.54, u + 0.06, 0.74, p.k.dark);
        p.rect(u - 0.04, 0.64, u + 0.04, 0.72, p.k.second);
    }
    p.rect(-0.4, 0.44, 0.4, 0.5, p.k.accent);
    p.rect(-0.36, 0.06, 0.36, 0.38, p.glass);
    p.rect(-0.08, 0.0, 0.08, 0.3, shade(p.trim, 0.1));
}

pub fn welcome_sign(p: &mut Pen) {
    p.shadow(0.44);
    let wood = p.k.wood;
    for u in [-0.34, 0.34] {
        p.rect(u - 0.025, 0.0, u + 0.025, 0.6, wood);
    }
    p.rr(-0.46, 0.26, 0.46, 0.66, 0.06, p.k.pale);
    p.rr(-0.42, 0.3, 0.42, 0.62, 0.05, hex(0x2f6b4f));
    p.line((-0.3, 0.52), (0.3, 0.52), 0.02, p.k.pale);
    p.line((-0.22, 0.4), (0.22, 0.4), 0.014, p.k.brass);
    let leaf = hex(0xc0463a);
    p.poly(
        &[
            (0.0, 0.84),
            (0.08, 0.72),
            (0.16, 0.76),
            (0.06, 0.66),
            (0.0, 0.62),
            (-0.06, 0.66),
            (-0.16, 0.76),
            (-0.08, 0.72),
        ],
        leaf,
    );
    p.flowers(-0.44, 0.44, 0.0, 9, 0.035);
}

pub fn transmitter(p: &mut Pen) {
    p.shadow(0.3);
    let (red, white) = (p.k.accent, p.k.pale);
    p.rr(-0.34, 0.0, 0.0, 0.2, 0.02, brick(p));
    for step in 0..9 {
        let v = step as f32 * 0.1;
        let ink = if step % 2 == 0 { red } else { white };
        p.rect(0.14, v, 0.2, v + 0.1, ink);
    }
    for (v, reach) in [(0.3, 0.44), (0.6, 0.4)] {
        p.line((0.17, v), (0.17 + reach * 0.7, 0.0), 0.004, p.k.dark);
        p.line((0.17, v), (0.17 - reach, 0.0), 0.004, p.k.dark);
    }
    p.ell(0.0, 0.7, 0.1, 0.04, hex(0xc8cdd2));
    p.line((0.14, 0.7), (0.06, 0.7), 0.01, p.k.metal);
    p.light(0.17, 0.94, 0.02, red);
}

pub fn record_store(p: &mut Pen) {
    let (awning, sign) = (p.k.dark, p.k.accent);
    shopfront(p, 0.9, awning, sign);
    p.rr(0.44, 0.5, 0.52, 0.9, 0.02, p.k.second);
    for step in 0..4 {
        p.circ(0.48, 0.56 + step as f32 * 0.1, 0.025, p.k.pale);
    }
    p.circ(0.0, 0.7, 0.06, p.k.dark);
    p.circ(0.0, 0.7, 0.02, p.k.brass);
    for u in [-0.28, -0.14, 0.0] {
        p.rect(
            u - 0.05,
            0.1,
            u + 0.05,
            0.24,
            [p.k.second, p.k.brass, p.k.accent][((u + 0.28) * 7.0) as usize % 3],
        );
    }
}

pub fn diner(p: &mut Pen) {
    let chrome = hex(0xc8cdd2);
    p.rr(-0.48, 0.06, 0.48, 0.74, 0.12, chrome);
    p.rect(-0.48, 0.3, 0.48, 0.36, p.k.accent);
    p.rect(-0.48, 0.14, 0.48, 0.18, p.k.accent);
    for step in 0..6 {
        let u = -0.38 + step as f32 * 0.13;
        p.rect(u, 0.42, u + 0.1, 0.62, p.glass);
    }
    p.rect(-0.48, 0.0, 0.48, 0.06, shade(chrome, -0.3));
    p.rr(-0.2, 0.74, 0.2, 0.96, 0.04, p.k.dark);
    p.line((-0.14, 0.85), (0.14, 0.85), 0.03, p.k.accent);
}

pub fn garage(p: &mut Pen) {
    let wall = p.k.pale;
    p.rect(-0.46, 0.0, 0.46, 0.64, wall);
    p.rect(-0.46, 0.64, 0.46, 0.76, p.k.second);
    p.rect(-0.36, 0.67, 0.36, 0.73, p.k.pale);
    for u in [-0.22, 0.2] {
        p.rect(u - 0.16, 0.0, u + 0.16, 0.48, shade(p.k.metal, 0.2));
        for v in [0.1, 0.2, 0.3, 0.4] {
            p.line((u - 0.16, v), (u + 0.16, v), 0.006, shade(p.k.metal, -0.1));
        }
    }
    p.rr(0.4, 0.0, 0.5, 0.4, 0.03, p.k.accent);
    p.circ(0.45, 0.3, 0.03, p.k.pale);
}

pub fn arcade(p: &mut Pen) {
    let wall = hex(0x2c2a3a);
    p.rect(-0.44, 0.0, 0.44, 0.9, wall);
    p.poly(
        &[(-0.3, 0.9), (0.3, 0.9), (0.2, 1.0), (-0.2, 1.0)],
        p.k.accent,
    );
    p.rect(-0.4, 0.64, 0.4, 0.84, p.k.dark);
    p.line((-0.34, 0.74), (0.34, 0.74), 0.03, p.k.accent);
    p.line((-0.3, 0.68), (0.3, 0.68), 0.012, p.k.second);
    p.rect(-0.38, 0.06, 0.38, 0.5, p.glass);
    for u in [-0.24, 0.0, 0.24] {
        p.rect(u - 0.07, 0.08, u + 0.07, 0.4, p.k.second);
        p.rect(u - 0.05, 0.26, u + 0.05, 0.36, p.k.brass);
    }
}

pub fn barber_shop(p: &mut Pen) {
    let (awning, sign) = (p.k.second, p.k.dark);
    shopfront(p, 0.9, awning, sign);
    p.rr(-0.5, 0.2, -0.42, 0.56, 0.03, p.k.pale);
    for step in 0..4 {
        let v = 0.22 + step as f32 * 0.08;
        p.line((-0.5, v), (-0.42, v + 0.06), 0.02, hex(0xc0302a));
    }
    p.circ(-0.46, 0.58, 0.03, hex(0xc8cdd2));
    p.rect(-0.3, 0.1, -0.14, 0.24, p.k.accent);
}

pub fn newsstand(p: &mut Pen) {
    p.shadow(0.44);
    p.rr(-0.4, 0.0, 0.4, 0.6, 0.03, hex(0x2f6b4f));
    p.rect(-0.34, 0.2, 0.34, 0.5, p.k.dark);
    for step in 0..6 {
        let u = -0.32 + step as f32 * 0.11;
        p.rect(
            u,
            0.22,
            u + 0.09,
            0.36,
            [p.k.pale, p.k.accent, p.k.brass][step % 3],
        );
    }
    p.poly(
        &[(-0.46, 0.6), (0.46, 0.6), (0.4, 0.76), (-0.4, 0.76)],
        p.k.brass,
    );
    p.rect(-0.3, 0.64, 0.3, 0.72, p.k.dark);
}

pub fn neon_lamp(p: &mut Pen) {
    p.shadow(0.2);
    p.rect(-0.02, 0.0, 0.02, 0.84, p.k.metal);
    p.rr(-0.14, 0.84, 0.14, 0.96, 0.05, p.k.accent);
    p.rr(-0.1, 0.86, 0.1, 0.94, 0.04, hex(0xffd0e8));
    p.circ(0.0, 0.9, 0.2, p.k.accent.opacity(0.15));
    p.line((-0.1, 0.3), (0.1, 0.3), 0.01, p.k.second);
}

pub fn quilt_shop(p: &mut Pen) {
    let (awning, sign) = (p.k.brass, p.k.second);
    shopfront(p, 0.84, awning, sign);
    p.gable(-0.46, 0.46, 0.88, 1.0, p.roof);
    let inks = [p.k.accent, p.k.pale, p.k.second, p.k.brass];
    for row in 0..3 {
        for col in 0..4 {
            p.rect(
                -0.34 + col as f32 * 0.1,
                0.1 + row as f32 * 0.09,
                -0.26 + col as f32 * 0.1,
                0.18 + row as f32 * 0.09,
                inks[(row + col) % 4],
            );
        }
    }
}

pub fn bike_rack(p: &mut Pen) {
    let metal = hex(0xc8cdd2);
    for step in 0..4 {
        let u = -0.36 + step as f32 * 0.24;
        p.curve((u - 0.08, 0.0), (u, 0.9), (u + 0.08, 0.0), 0.02, metal);
    }
    p.circ(-0.1, 0.3, 0.2, p.k.dark);
    p.circ(-0.1, 0.3, 0.17, p.k.ground.opacity(0.0));
    p.circ(0.3, 0.3, 0.2, p.k.dark);
    p.line((-0.1, 0.3), (0.14, 0.5), 0.02, p.k.accent);
    p.line((0.14, 0.5), (0.3, 0.3), 0.02, p.k.accent);
}

pub fn school_bike_rack(p: &mut Pen) {
    let metal = p.k.metal;
    p.rect(-0.48, 0.0, 0.48, 0.04, metal);
    for step in 0..7 {
        let u = -0.42 + step as f32 * 0.14;
        p.rect(u - 0.01, 0.0, u + 0.01, 0.28, metal);
    }
    for (u, ink) in [(-0.2, p.k.accent), (0.2, p.k.second)] {
        p.circ(u - 0.12, 0.2, 0.12, p.k.dark);
        p.circ(u + 0.12, 0.2, 0.12, p.k.dark);
        p.line((u - 0.12, 0.2), (u, 0.4), 0.02, ink);
        p.line((u, 0.4), (u + 0.12, 0.2), 0.02, ink);
        p.line((u - 0.02, 0.44), (u + 0.04, 0.44), 0.02, p.k.dark);
    }
    p.rect(-0.1, 0.6, 0.1, 0.9, p.k.second);
    p.rect(-0.012, 0.0, 0.012, 0.6, metal);
}

pub fn street_lamp_post(p: &mut Pen) {
    p.shadow(0.2);
    let iron = hex(0x2f4a3a);
    p.rect(-0.1, 0.0, 0.1, 0.06, iron);
    p.rect(-0.03, 0.06, 0.03, 0.82, iron);
    p.rr(-0.1, 0.82, 0.1, 0.94, 0.06, hex(0xf2ead0));
    p.dome(0.0, 0.94, 0.08, 0.04, iron);
}

pub fn flagpole(p: &mut Pen) {
    p.shadow(0.2);
    p.rect(-0.015, 0.0, 0.015, 0.98, hex(0xc8cdd2));
    p.circ(0.0, 0.99, 0.02, p.k.brass);
    let (u0, v0) = (0.015, 0.94);
    for step in 0..5 {
        let v = v0 - step as f32 * 0.04;
        let ink = if step % 2 == 0 {
            hex(0xc0302a)
        } else {
            p.k.pale
        };
        p.rect(u0, v - 0.04, u0 + 0.4, v, ink);
    }
    p.rect(u0, 0.86, u0 + 0.16, 0.94, hex(0x2f4a8a));
}

pub fn streamers(p: &mut Pen) {
    for u in [-0.46, 0.46] {
        p.rect(u - 0.012, 0.0, u + 0.012, 0.92, p.k.metal);
    }
    for (index, ink) in [p.k.accent, p.k.second, p.k.brass].iter().enumerate() {
        let sag = 0.3 + index as f32 * 0.12;
        p.curve((-0.46, 0.9), (0.0, 0.9 - sag), (0.46, 0.9), 0.02, *ink);
    }
    for u in [-0.2, 0.0, 0.2] {
        p.curve(
            (u, 0.66),
            (u + 0.04, 0.5),
            (u - 0.02, 0.4),
            0.012,
            p.k.accent,
        );
    }
}

pub fn fairy_lights(p: &mut Pen) {
    let wood = p.k.wood;
    p.rect(-0.46, 0.0, -0.42, 0.9, wood);
    p.rect(0.42, 0.0, 0.46, 0.7, wood);
    p.curve((-0.44, 0.88), (0.0, 0.5), (0.44, 0.68), 0.006, p.k.dark);
    for step in 0..10 {
        let t = (step as f32 + 0.5) / 10.0;
        let u = -0.44 + 0.88 * t;
        let v = 0.88 * (1.0 - t) + 0.68 * t - 0.4 * t * (1.0 - t) * 2.0;
        p.light(
            u,
            v - 0.03,
            0.013,
            [p.k.brass, p.k.accent, p.k.second, p.k.pale][step % 4],
        );
    }
}

pub fn tomato_patch(p: &mut Pen) {
    p.shadow(0.46);
    p.rect(-0.46, 0.0, 0.46, 0.1, shade(p.k.wood, -0.3));
    for u in [-0.3, 0.0, 0.3] {
        p.line((u, 0.1), (u, 0.96), 0.012, p.k.wood);
        for (v, side) in [(0.3, -1.0), (0.5, 1.0), (0.7, -1.0)] {
            p.circ(u + side * 0.05, v, 0.07, p.k.leaf);
            p.circ(u + side * 0.08, v - 0.06, 0.04, hex(0xc0302a));
        }
    }
}

pub fn rose_bed(p: &mut Pen) {
    p.shadow(0.46);
    p.rect(-0.46, 0.0, 0.46, 0.14, p.k.stone);
    for step in 0..6 {
        let u = -0.38 + step as f32 * 0.15;
        p.circ(u, 0.28, 0.1, shade(p.k.leaf, -0.1));
        p.circ(u + 0.02, 0.34, 0.045, hex(0xd8455a));
        p.circ(u - 0.04, 0.24, 0.035, hex(0xe87a8a));
    }
}

pub fn second_street(p: &mut Pen) {
    let wall = brick(p);
    for (u0, u1, tall, roof) in [(-0.48, -0.02, 0.8, p.roof), (0.02, 0.48, 0.66, p.k.second)] {
        p.rect(u0, 0.0, u1, tall, wall);
        p.rect(u0 - 0.01, tall, u1 + 0.01, tall + 0.04, shade(wall, -0.2));
        p.rect(u0, tall * 0.62, u1, tall * 0.62 + 0.04, roof);
        let mid = (u0 + u1) / 2.0;
        p.window(mid - 0.14, tall * 0.7, mid - 0.04, tall * 0.9);
        p.window(mid + 0.04, tall * 0.7, mid + 0.14, tall * 0.9);
        p.rect(mid - 0.16, 0.06, mid + 0.06, tall * 0.5, p.glass);
        p.rect(mid + 0.1, 0.0, mid + 0.18, tall * 0.5, shade(p.trim, 0.1));
    }
}

pub fn radio_beacon(p: &mut Pen) {
    p.shadow(0.2);
    let metal = p.k.metal;
    p.line((-0.14, 0.0), (0.0, 0.9), 0.014, metal);
    p.line((0.14, 0.0), (0.0, 0.9), 0.014, metal);
    for v in [0.3, 0.6] {
        let s = 0.14 - v * 0.15;
        p.line((-s, v), (s, v), 0.01, metal);
    }
    p.light(0.0, 0.94, 0.03, p.k.accent);
    for r in [0.14, 0.24] {
        p.curve(
            (-r, 0.94 + r * 0.3),
            (0.0, 0.94 + r),
            (r, 0.94 + r * 0.3),
            0.008,
            p.k.second,
        );
    }
}

pub fn street_survey(p: &mut Pen) {
    p.shadow(0.36);
    let wood = p.k.brass;
    p.line((-0.26, 0.0), (0.0, 0.56), 0.016, wood);
    p.line((0.26, 0.0), (0.0, 0.56), 0.016, wood);
    p.line((0.04, 0.0), (0.0, 0.56), 0.016, wood);
    p.rr(-0.12, 0.56, 0.12, 0.68, 0.03, p.k.dark);
    p.rect(0.12, 0.6, 0.22, 0.64, p.k.dark);
    p.rect(0.34, 0.0, 0.38, 0.9, p.k.pale);
    for step in 0..4 {
        p.rect(
            0.34,
            0.1 + step as f32 * 0.2,
            0.38,
            0.2 + step as f32 * 0.2,
            hex(0xc0302a),
        );
    }
}

/// A rental shelf of tapes under a little awning, a "NEW" card on top.
pub fn tape_shelf(p: &mut Pen) {
    p.shadow(0.4);
    let wood = p.k.wood;
    p.rect(-0.34, 0.0, 0.34, 0.74, shade(wood, -0.2));
    for (row, v) in [0.06, 0.28, 0.5].iter().enumerate() {
        p.rect(-0.3, *v, 0.3, v + 0.02, wood);
        for step in 0..7 {
            let u = -0.28 + step as f32 * 0.08;
            let ink = [p.k.dark, p.k.accent, p.k.second, hex(0x2a3a8a)][(step + row) % 4];
            p.rect(u, v + 0.02, u + 0.06, v + 0.19, ink);
            p.rect(u + 0.01, v + 0.12, u + 0.05, v + 0.15, p.k.pale);
        }
    }
    p.poly(
        &[(-0.4, 0.74), (0.4, 0.74), (0.34, 0.84), (-0.34, 0.84)],
        p.k.brass,
    );
    p.rr(0.1, 0.84, 0.36, 0.96, 0.02, p.k.accent);
    p.line((0.14, 0.9), (0.32, 0.9), 0.012, p.k.pale);
}

/// A 1987 city bus: a long box on six wheels, a row of windows, a route
/// board, a stripe down its side.
pub fn city_bus(p: &mut Pen) {
    p.ell(0.0, 0.02, 0.5, 0.05, gpui::black().opacity(0.15));
    let body = p.k.pale;
    p.rr(-0.5, 0.12, 0.5, 0.86, 0.04, body);
    p.rect(-0.5, 0.3, 0.5, 0.38, p.k.accent);
    p.rect(-0.5, 0.26, 0.5, 0.28, p.k.second);
    for step in 0..6 {
        let u = -0.44 + step as f32 * 0.14;
        p.rect(u, 0.5, u + 0.11, 0.74, p.glass);
    }
    p.rect(0.4, 0.14, 0.48, 0.74, shade(p.glass, -0.1));
    p.rect(0.38, 0.76, 0.49, 0.83, p.k.dark);
    p.line((0.4, 0.795), (0.47, 0.795), 0.012, p.k.brass);
    for u in [-0.34, 0.3] {
        p.circ(u, 0.12, 0.07, p.k.dark);
        p.circ(u, 0.12, 0.03, hex(0xc8cdd2));
    }
}
