//! Icebridge's drawings: what penguins build on the ice. Snow nests and
//! snow domes, ice blocks, kelp on racks, pebbles and whale bone, lanterns
//! on the floe. Nothing here would stand on a shore of grass.

use super::Pen;
use crate::art::{hex, shade};

fn snow(p: &Pen) -> gpui::Hsla {
    p.k.pale
}

fn ice(_: &Pen) -> gpui::Hsla {
    hex(0xbfe0ee)
}

/// A snow dome of cut blocks, from `u - r` to `u + r`, `rv` tall, with
/// its low door tunnel on the right.
fn blocks_dome(p: &mut Pen, u: f32, r: f32, rv: f32) {
    let body = snow(p);
    p.dome(u, 0.0, r, rv, body);
    p.dome(u - r * 0.25, 0.0, r * 0.55, rv * 0.9, shade(body, 0.06));
    let seam = hex(0xc8dce8);
    for step in 1..4 {
        let t = step as f32 / 4.0;
        let v = rv * t;
        let s = r * (1.0 - t * t).sqrt();
        p.line((u - s, v), (u + s, v), 0.006, seam);
    }
    p.dome(u + r * 0.75, 0.0, r * 0.35, rv * 0.42, shade(body, -0.04));
    p.dome(u + r * 0.85, 0.0, r * 0.18, rv * 0.26, shade(p.k.dark, 0.3));
}

/// A pebble.
fn pebble(p: &mut Pen, u: f32, v: f32, r: f32, tone: f32) {
    let stone = p.k.stone;
    p.ell(u, v, r, r * 0.7 * p.w / p.h, shade(stone, tone));
}

pub fn snow_nest(p: &mut Pen) {
    p.shadow(0.46);
    blocks_dome(p, -0.06, 0.4, 0.8);
    for step in 0..6 {
        let u = -0.42 + step as f32 * 0.14;
        pebble(p, u, 0.03, 0.05, (step % 3) as f32 * 0.08 - 0.08);
    }
    p.rect(-0.14, 0.62, 0.02, 0.66, p.roof);
}

pub fn ice_store(p: &mut Pen) {
    p.shadow(0.46);
    let block = ice(p);
    for row in 0..4 {
        let v = row as f32 * 0.16;
        let off = if row % 2 == 0 { 0.0 } else { 0.09 };
        for step in 0..5 {
            let u = -0.44 + off + step as f32 * 0.18;
            if u > 0.38 {
                continue;
            }
            p.rr(
                u,
                v,
                u + 0.17,
                v + 0.15,
                0.02,
                shade(block, ((row + step) % 3) as f32 * 0.06 - 0.06),
            );
        }
    }
    p.rect(-0.46, 0.64, 0.46, 0.72, snow(p));
    p.arch(-0.1, 0.1, 0.0, 0.34, shade(p.k.dark, 0.2));
    for u in [-0.3, 0.3] {
        p.ell(u, 0.8, 0.1, 0.04, p.k.accent);
    }
}

pub fn ice_spire(p: &mut Pen) {
    p.shadow(0.3);
    let body = ice(p);
    p.poly(
        &[
            (-0.24, 0.0),
            (0.24, 0.0),
            (0.12, 0.5),
            (0.02, 1.0),
            (-0.1, 0.56),
        ],
        body,
    );
    p.poly(
        &[(-0.02, 0.0), (0.24, 0.0), (0.12, 0.5), (0.02, 1.0)],
        shade(body, 0.2),
    );
    p.line((-0.06, 0.2), (0.04, 0.7), 0.01, shade(body, -0.15));
    p.rect(-0.3, 0.0, 0.3, 0.06, snow(p));
}

pub fn snow_dome(p: &mut Pen) {
    p.shadow(0.48);
    blocks_dome(p, -0.04, 0.44, 0.9);
    p.circ(-0.1, 0.55, 0.04, p.glass);
}

pub fn kelp_frond(p: &mut Pen) {
    p.shadow(0.2);
    let kelp = p.k.leaf;
    for (index, lean) in [-0.18_f32, 0.0, 0.16].iter().enumerate() {
        let tall = 0.8 + index as f32 * 0.08;
        p.curve(
            (0.0, 0.0),
            (lean * 2.0, tall * 0.5),
            (*lean, tall),
            0.04,
            shade(kelp, index as f32 * 0.08),
        );
        p.ell(*lean, tall, 0.05, 0.05, shade(kelp, 0.15));
    }
    p.ell(0.0, 0.02, 0.18, 0.04, p.k.stone);
}

pub fn lantern_post(p: &mut Pen) {
    p.shadow(0.2);
    let bone = p.k.brass;
    p.ell(0.0, 0.04, 0.14, 0.05, snow(p));
    p.poly(
        &[(-0.05, 0.0), (0.05, 0.0), (0.03, 0.8), (-0.03, 0.8)],
        bone,
    );
    p.rr(-0.1, 0.8, 0.1, 0.96, 0.03, ice(p));
    p.light(0.0, 0.88, 0.04, p.k.accent);
    p.dome(0.0, 0.96, 0.08, 0.04, bone);
}

pub fn ice_bridge(p: &mut Pen) {
    let body = ice(p);
    p.ell(0.0, 0.06, 0.52, 0.1, p.k.water);
    p.poly(
        &[
            (-0.5, 0.0),
            (-0.3, 0.0),
            (-0.1, 0.5),
            (0.1, 0.5),
            (0.3, 0.0),
            (0.5, 0.0),
            (0.5, 0.7),
            (-0.5, 0.7),
        ],
        body,
    );
    p.rect(-0.5, 0.7, 0.5, 0.8, snow(p));
    p.poly(
        &[(-0.3, 0.0), (0.3, 0.0), (0.1, 0.5), (-0.1, 0.5)],
        shade(p.k.water, 0.2).opacity(0.0),
    );
    for u in [-0.36, 0.3] {
        p.line((u, 0.1), (u + 0.06, 0.6), 0.008, shade(body, -0.15));
    }
}

pub fn kayak(p: &mut Pen) {
    let hide = p.k.wood;
    p.poly(&[(-0.5, 0.5), (0.5, 0.5), (0.36, 0.1), (-0.36, 0.1)], hide);
    p.poly(
        &[(-0.5, 0.5), (0.5, 0.5), (0.44, 0.38), (-0.44, 0.38)],
        shade(hide, 0.15),
    );
    p.ell(0.0, 0.52, 0.1, 0.06, p.k.dark);
    p.line((-0.3, 0.9), (0.3, 0.3), 0.02, p.k.brass);
    p.ell(-0.3, 0.9, 0.04, 0.1, p.k.accent);
    p.ell(0.3, 0.3, 0.04, 0.1, p.k.accent);
    p.ell(0.0, 0.06, 0.4, 0.08, p.k.water.opacity(0.4));
}

pub fn ice_ledge(p: &mut Pen) {
    let body = ice(p);
    p.ell(0.0, 0.06, 0.52, 0.12, p.k.water);
    p.poly(&[(-0.5, 0.0), (0.5, 0.0), (0.46, 0.7), (-0.46, 0.66)], body);
    p.rect(-0.5, 0.66, 0.5, 0.8, snow(p));
    for u in [-0.3, 0.0, 0.28] {
        p.poly(
            &[(u - 0.04, 0.66), (u + 0.04, 0.66), (u, 0.42)],
            shade(body, 0.2),
        );
    }
}

pub fn kelp_garden(p: &mut Pen) {
    p.ell(0.0, 0.2, 0.5, 0.2, p.k.water);
    for step in 0..7 {
        let u = -0.4 + step as f32 * 0.13;
        let tall = 0.5 + 0.3 * ((step * 3) % 4) as f32 / 4.0;
        p.curve(
            (u, 0.1),
            (u + 0.08, tall * 0.6),
            (u - 0.02, tall),
            0.03,
            shade(p.k.leaf, (step % 2) as f32 * 0.1),
        );
    }
    p.rect(-0.5, 0.0, 0.5, 0.06, snow(p));
}

pub fn wind_shelter(p: &mut Pen) {
    p.shadow(0.48);
    let body = snow(p);
    p.poly(&[(-0.48, 0.0), (0.2, 0.0), (0.0, 0.9), (-0.3, 0.84)], body);
    p.poly(&[(-0.1, 0.0), (0.2, 0.0), (0.0, 0.9)], shade(body, -0.06));
    for v in [0.2, 0.4, 0.6] {
        p.line(
            (-0.46 + v * 0.18, v),
            (0.2 - v * 0.22, v),
            0.006,
            hex(0xc8dce8),
        );
    }
    p.poly(&[(0.3, 0.0), (0.5, 0.0), (0.44, 0.3)], shade(ice(p), 0.1));
}

pub fn ice_bench(p: &mut Pen) {
    p.shadow(0.46);
    let body = ice(p);
    for u in [-0.32, 0.32] {
        p.rr(u - 0.1, 0.0, u + 0.1, 0.42, 0.02, shade(body, -0.05));
    }
    p.rr(-0.48, 0.42, 0.48, 0.58, 0.03, body);
    p.rect(-0.48, 0.54, 0.48, 0.6, snow(p));
    p.rr(-0.3, 0.58, 0.3, 0.66, 0.03, p.k.leaf);
}

pub fn ice_well(p: &mut Pen) {
    p.shadow(0.44);
    let body = ice(p);
    p.ell(0.0, 0.3, 0.42, 0.12, shade(body, -0.1));
    p.rect(-0.42, 0.0, 0.42, 0.3, body);
    for u in [-0.28, -0.08, 0.12, 0.32] {
        p.line((u, 0.0), (u, 0.3), 0.006, hex(0xa8c8d8));
    }
    p.ell(0.0, 0.3, 0.32, 0.08, p.k.water);
    p.line((0.3, 0.3), (0.2, 0.9), 0.014, p.k.brass);
    p.line((0.2, 0.9), (0.0, 0.8), 0.006, p.k.dark);
    p.line((0.0, 0.8), (0.0, 0.34), 0.006, p.k.dark);
}

pub fn kelp_swing(p: &mut Pen) {
    p.shadow(0.4);
    let bone = p.k.brass;
    p.curve((-0.4, 0.0), (-0.44, 0.9), (0.0, 0.94), 0.04, bone);
    p.curve((0.4, 0.0), (0.44, 0.9), (0.0, 0.94), 0.04, bone);
    let swing = 0.06 * p.sway;
    for u in [-0.08, 0.08] {
        p.curve(
            (u, 0.92),
            (u + 0.02, 0.6),
            (u + swing, 0.24),
            0.014,
            p.k.leaf,
        );
    }
    p.ell(swing, 0.22, 0.12, 0.03, p.k.leaf);
}

pub fn geyser(p: &mut Pen) {
    p.shadow(0.4);
    let stone = p.k.stone;
    p.ell(0.0, 0.08, 0.4, 0.1, shade(stone, -0.1));
    p.ell(0.0, 0.1, 0.2, 0.05, p.k.water);
    let steam = p.k.pale.opacity(0.7);
    for (u, v, r) in [
        (0.0, 0.3, 0.1),
        (0.04, 0.5, 0.13),
        (-0.04, 0.7, 0.15),
        (0.06, 0.88, 0.12),
    ] {
        p.circ(u, v, r, steam);
    }
    p.line(
        (0.0, 0.1),
        (0.0, 0.6),
        0.04,
        shade(p.k.water, 0.5).opacity(0.8),
    );
}

pub fn snow_marker(p: &mut Pen) {
    p.shadow(0.2);
    p.ell(0.0, 0.03, 0.14, 0.04, snow(p));
    p.rect(-0.03, 0.0, 0.03, 0.9, p.k.dark);
    p.poly(&[(0.03, 0.9), (0.32, 0.84), (0.03, 0.76)], p.k.accent);
    for v in [0.2, 0.4, 0.6] {
        p.rect(-0.035, v, 0.035, v + 0.06, p.k.accent);
    }
}

pub fn nesting_box(p: &mut Pen) {
    p.shadow(0.4);
    let block = ice(p);
    p.rr(-0.36, 0.0, 0.36, 0.56, 0.04, block);
    p.rect(-0.36, 0.5, 0.36, 0.62, snow(p));
    p.arch(-0.14, 0.14, 0.0, 0.34, shade(p.k.dark, 0.2));
    p.ell(0.0, 0.08, 0.1, 0.05, p.k.pale);
    for u in [-0.26, 0.26] {
        pebble(p, u, 0.66, 0.05, 0.0);
    }
}

pub fn pebble_planters(p: &mut Pen) {
    for (u, r) in [(-0.3, 0.16), (0.02, 0.18), (0.32, 0.14)] {
        for step in 0..5 {
            let a = u - r + step as f32 * r * 0.5;
            pebble(p, a, 0.08, r * 0.3, (step % 3) as f32 * 0.08 - 0.08);
        }
        for step in 0..3 {
            let a = u - r * 0.5 + step as f32 * r * 0.5;
            p.curve((a, 0.12), (a + 0.04, 0.4), (a - 0.02, 0.6), 0.02, p.k.leaf);
        }
    }
}

/// A penguin carved from ice on a snow plinth: a face, flippers out.
pub fn ice_sculpture(p: &mut Pen) {
    p.shadow(0.3);
    let body = ice(p);
    let dark = hex(0x6a9ab0);
    p.rect(-0.26, 0.0, 0.26, 0.2, snow(p));
    p.ell(0.0, 0.5, 0.2, 0.3, body);
    p.ell(0.04, 0.48, 0.12, 0.22, shade(body, 0.25));
    p.circ(0.0, 0.84, 0.13, body);
    p.face(0.0, 0.84, 0.13, dark);
    p.poly(&[(0.1, 0.8), (0.24, 0.78), (0.1, 0.76)], p.k.accent);
    for side in [-1.0_f32, 1.0] {
        p.poly(
            &[(side * 0.18, 0.62), (side * 0.36, 0.44), (side * 0.2, 0.5)],
            body,
        );
    }
}

pub fn message_stone(p: &mut Pen) {
    p.shadow(0.3);
    let stone = p.k.stone;
    p.poly(
        &[
            (-0.26, 0.0),
            (0.26, 0.0),
            (0.22, 0.6),
            (0.0, 0.76),
            (-0.22, 0.62),
        ],
        stone,
    );
    for (index, v) in [0.46, 0.34, 0.22].iter().enumerate() {
        let w = 0.14 - index as f32 * 0.02;
        p.line((-w, *v), (w, *v), 0.014, shade(stone, -0.35));
    }
    pebble(p, 0.0, 0.8, 0.06, 0.2);
}

pub fn fish_stall(p: &mut Pen) {
    p.shadow(0.46);
    let block = ice(p);
    p.rr(-0.44, 0.0, 0.44, 0.36, 0.03, block);
    p.rect(-0.44, 0.34, 0.44, 0.4, snow(p));
    for step in 0..5 {
        let u = -0.32 + step as f32 * 0.16;
        p.ell(u, 0.44, 0.07, 0.04, hex(0x9ab0c0));
        p.poly(
            &[(u + 0.07, 0.44), (u + 0.11, 0.48), (u + 0.11, 0.4)],
            hex(0x9ab0c0),
        );
    }
    for u in [-0.4, 0.4] {
        p.rect(u - 0.02, 0.4, u + 0.02, 0.84, p.k.brass);
    }
    p.pennants((-0.4, 0.82), (0.4, 0.82), 6, &[p.k.accent, p.k.second]);
}

pub fn fish_crate(p: &mut Pen) {
    p.shadow(0.4);
    let wood = p.k.wood;
    p.rect(-0.34, 0.0, 0.34, 0.5, wood);
    for v in [0.16, 0.33] {
        p.line((-0.34, v), (0.34, v), 0.008, shade(wood, -0.25));
    }
    for step in 0..4 {
        let u = -0.24 + step as f32 * 0.16;
        p.ell(u, 0.56, 0.08, 0.06, hex(0x9ab0c0));
    }
    p.rect(-0.36, 0.48, 0.36, 0.52, snow(p));
}

pub fn snow_wall(p: &mut Pen) {
    let body = snow(p);
    for row in 0..3 {
        let v = row as f32 * 0.26;
        let off = if row % 2 == 0 { 0.0 } else { 0.1 };
        for step in 0..5 {
            let u = -0.5 + off + step as f32 * 0.2;
            if u > 0.34 {
                continue;
            }
            p.rr(
                u,
                v,
                u + 0.19,
                v + 0.25,
                0.02,
                shade(body, -0.03 * ((row + step) % 3) as f32),
            );
        }
    }
    p.line((-0.5, 0.26), (0.5, 0.26), 0.004, hex(0xc8dce8));
}

pub fn fishing_hole(p: &mut Pen) {
    p.ell(0.0, 0.3, 0.48, 0.28, snow(p));
    p.ell(0.0, 0.32, 0.22, 0.12, shade(ice(p), -0.1));
    p.ell(0.0, 0.32, 0.16, 0.08, p.k.water);
    p.line((0.3, 0.3), (0.06, 0.9), 0.014, p.k.brass);
    p.curve((0.06, 0.9), (0.02, 0.7), (0.0, 0.34), 0.004, p.k.dark);
}

pub fn vault_ice_house(p: &mut Pen) {
    p.shadow(0.46);
    let block = ice(p);
    p.rr(-0.44, 0.0, 0.44, 0.66, 0.04, block);
    for v in [0.16, 0.33, 0.5] {
        p.line((-0.44, v), (0.44, v), 0.006, shade(block, -0.18));
    }
    p.dome(0.0, 0.66, 0.44, 0.24, snow(p));
    p.rr(-0.14, 0.0, 0.14, 0.4, 0.06, p.k.wood);
    p.ell(0.0, 0.52, 0.08, 0.04, hex(0x9ab0c0));
}

pub fn creche(p: &mut Pen) {
    p.shadow(0.48);
    let body = snow(p);
    p.poly(
        &[
            (-0.48, 0.0),
            (-0.46, 0.4),
            (-0.1, 0.5),
            (0.3, 0.44),
            (0.48, 0.0),
        ],
        body,
    );
    p.ell(0.0, 0.08, 0.34, 0.06, shade(body, -0.08));
    for (u, v) in [(-0.2, 0.12), (0.0, 0.14), (0.2, 0.12)] {
        p.ell(u, v + 0.08, 0.07, 0.1, hex(0x9a9a92));
        p.circ(u, v + 0.2, 0.05, hex(0x9a9a92));
        p.circ(u + 0.02, v + 0.21, 0.01, p.k.dark);
    }
}

pub fn bridge_lanterns(p: &mut Pen) {
    p.rect(-0.5, 0.0, 0.5, 0.16, ice(p));
    p.rect(-0.5, 0.16, 0.5, 0.2, snow(p));
    for u in [-0.36, 0.0, 0.36] {
        p.rect(u - 0.015, 0.2, u + 0.015, 0.8, p.k.brass);
        p.rr(u - 0.06, 0.8, u + 0.06, 0.94, 0.02, ice(p));
        p.light(u, 0.87, 0.025, p.k.accent);
    }
    p.curve((-0.36, 0.78), (-0.18, 0.66), (0.0, 0.78), 0.006, p.k.dark);
    p.curve((0.0, 0.78), (0.18, 0.66), (0.36, 0.78), 0.006, p.k.dark);
}

pub fn berg_lookout(p: &mut Pen) {
    let body = ice(p);
    p.poly(
        &[
            (-0.44, 0.0),
            (0.44, 0.0),
            (0.3, 0.44),
            (0.14, 0.7),
            (-0.2, 0.66),
            (-0.36, 0.34),
        ],
        body,
    );
    p.poly(
        &[
            (0.0, 0.0),
            (0.44, 0.0),
            (0.3, 0.44),
            (0.14, 0.7),
            (0.0, 0.6),
        ],
        shade(body, 0.2),
    );
    for step in 0..5 {
        let v = 0.08 + step as f32 * 0.12;
        p.rect(
            -0.3 + step as f32 * 0.04,
            v,
            -0.18 + step as f32 * 0.04,
            v + 0.03,
            snow(p),
        );
    }
    p.rect(-0.04, 0.66, 0.0, 0.96, p.k.brass);
    p.poly(&[(0.0, 0.96), (0.2, 0.9), (0.0, 0.84)], p.k.accent);
}

pub fn sea_slide(p: &mut Pen) {
    let body = snow(p);
    p.ell(0.3, 0.04, 0.24, 0.06, p.k.water);
    p.poly(
        &[
            (-0.5, 0.0),
            (-0.5, 0.86),
            (-0.3, 0.86),
            (0.3, 0.1),
            (0.5, 0.06),
            (0.5, 0.0),
        ],
        body,
    );
    p.curve(
        (-0.36, 0.84),
        (0.0, 0.5),
        (0.36, 0.1),
        0.05,
        shade(ice(p), -0.05),
    );
    p.curve(
        (-0.36, 0.84),
        (0.0, 0.5),
        (0.36, 0.1),
        0.015,
        shade(ice(p), 0.2),
    );
}

pub fn council_ring(p: &mut Pen) {
    for (index, u) in [-0.42, -0.26, -0.08, 0.1, 0.28, 0.44].iter().enumerate() {
        let v = if index == 0 || index == 5 {
            0.06
        } else {
            0.14 - (u * 0.2_f32).abs()
        };
        let tall = 0.5 + 0.15 * (index % 2) as f32;
        p.rr(
            u - 0.06,
            v,
            u + 0.06,
            v + tall,
            0.03,
            shade(p.k.stone, (index % 3) as f32 * 0.07),
        );
        p.rect(u - 0.06, v + tall - 0.06, u + 0.06, v + tall, snow(p));
    }
}

pub fn kelp_beds(p: &mut Pen) {
    p.ell(0.0, 0.2, 0.5, 0.2, shade(p.k.water, 0.1));
    for step in 0..10 {
        let u = -0.42 + step as f32 * 0.093;
        p.ell(
            u,
            0.24,
            0.05,
            0.1,
            shade(p.k.leaf, (step % 3) as f32 * 0.08 - 0.05),
        );
    }
    for u in [-0.46, 0.46] {
        p.rect(u - 0.01, 0.1, u + 0.01, 0.6, p.k.brass);
    }
    p.line((-0.46, 0.5), (0.46, 0.5), 0.006, p.k.dark);
}

pub fn thaw_marker(p: &mut Pen) {
    p.shadow(0.2);
    p.rect(-0.03, 0.0, 0.03, 0.92, p.k.brass);
    for step in 0..6 {
        let v = 0.1 + step as f32 * 0.13;
        p.line((0.03, v), (0.14, v), 0.012, p.k.dark);
    }
    p.circ(0.0, 0.96, 0.05, p.k.accent);
    p.ell(0.0, 0.04, 0.3, 0.06, ice(p));
    p.line((-0.3, 0.04), (0.3, 0.02), 0.006, p.k.water);
}

pub fn second_vault(p: &mut Pen) {
    p.shadow(0.46);
    blocks_dome(p, -0.08, 0.38, 0.7);
    p.rr(0.2, 0.0, 0.46, 0.36, 0.03, ice(p));
    p.rect(0.2, 0.34, 0.46, 0.4, snow(p));
    p.ell(0.33, 0.44, 0.08, 0.04, hex(0x9ab0c0));
}

pub fn rope_bridge(p: &mut Pen) {
    let body = ice(p);
    p.poly(&[(-0.5, 0.0), (-0.3, 0.0), (-0.3, 0.5), (-0.5, 0.5)], body);
    p.poly(&[(0.3, 0.0), (0.5, 0.0), (0.5, 0.5), (0.3, 0.5)], body);
    p.ell(0.0, 0.06, 0.3, 0.06, p.k.water);
    p.curve((-0.3, 0.5), (0.0, 0.3), (0.3, 0.5), 0.02, p.k.wood);
    p.curve((-0.3, 0.76), (0.0, 0.56), (0.3, 0.76), 0.01, p.k.leaf);
    for step in 0..6 {
        let t = (step as f32 + 0.5) / 6.0;
        let u = -0.3 + 0.6 * t;
        let v = 0.5 - 0.2 * 4.0 * t * (1.0 - t) * 0.5;
        p.line((u, v), (u, v + 0.26), 0.006, p.k.leaf);
    }
    for u in [-0.3, 0.3] {
        p.rect(u - 0.015, 0.5, u + 0.015, 0.8, p.k.brass);
    }
}

pub fn song_stone(p: &mut Pen) {
    p.shadow(0.3);
    let stone = p.k.stone;
    p.poly(
        &[
            (-0.2, 0.0),
            (0.2, 0.0),
            (0.24, 0.5),
            (0.1, 0.8),
            (-0.14, 0.76),
            (-0.24, 0.44),
        ],
        stone,
    );
    for (u, v) in [(-0.06, 0.5), (0.06, 0.34), (0.0, 0.62)] {
        p.circ(u, v, 0.04, shade(stone, -0.4));
    }
    for r in [0.1, 0.18] {
        p.curve(
            (0.24 + r * 0.2, 0.6 - r),
            (0.28 + r, 0.6),
            (0.24 + r * 0.2, 0.6 + r),
            0.01,
            p.k.second,
        );
    }
}

pub fn egg_warmer(p: &mut Pen) {
    p.shadow(0.44);
    let stone = p.k.stone;
    for step in 0..6 {
        let u = -0.36 + step as f32 * 0.144;
        pebble(p, u, 0.08, 0.08, (step % 2) as f32 * 0.1);
    }
    p.ell(0.0, 0.2, 0.3, 0.12, shade(p.k.wood, 0.2));
    p.ell(0.0, 0.26, 0.08, 0.1, p.k.pale);
    p.ell(0.12, 0.24, 0.06, 0.08, hex(0xf2ead8));
    for u in [-0.1, 0.1] {
        p.curve(
            (u, 0.44),
            (u + 0.04, 0.6),
            (u, 0.74),
            0.008,
            stone.opacity(0.5),
        );
    }
}

pub fn ridge_steps(p: &mut Pen) {
    let body = snow(p);
    p.poly(
        &[(-0.5, 0.0), (0.5, 0.0), (0.5, 0.86), (0.2, 0.86)],
        shade(body, -0.05),
    );
    for step in 0..6 {
        let u = -0.4 + step as f32 * 0.14;
        let v = step as f32 * 0.14;
        p.rect(u, v, u + 0.16, v + 0.06, ice(p));
        p.rect(u, v + 0.06, u + 0.16, v + 0.08, body);
    }
    p.line((-0.44, 0.1), (0.3, 0.96), 0.008, p.k.brass);
}

pub fn deep_ledge(p: &mut Pen) {
    let body = ice(p);
    p.rect(-0.5, 0.0, 0.5, 0.3, shade(p.k.water, -0.2));
    p.poly(
        &[
            (-0.5, 0.3),
            (0.3, 0.3),
            (0.46, 0.42),
            (0.46, 0.56),
            (-0.5, 0.56),
        ],
        body,
    );
    p.rect(-0.5, 0.56, 0.46, 0.62, snow(p));
    p.line((0.3, 0.62), (0.4, 0.96), 0.012, p.k.brass);
    p.curve((0.4, 0.96), (0.46, 0.7), (0.44, 0.2), 0.004, p.k.dark);
    p.ell(0.44, 0.18, 0.02, 0.03, p.k.accent);
}

pub fn story_circle(p: &mut Pen) {
    p.ell(0.0, 0.3, 0.48, 0.24, shade(snow(p), -0.06));
    for step in 0..9 {
        let a = step as f32 * std::f32::consts::TAU / 9.0;
        pebble(
            p,
            a.cos() * 0.42,
            0.3 + a.sin() * 0.2,
            0.05,
            (step % 3) as f32 * 0.08,
        );
    }
    p.circ(0.0, 0.3, 0.08, p.k.accent);
    p.circ(0.0, 0.3, 0.04, hex(0xffd27a));
}

pub fn breathing_hole(p: &mut Pen) {
    p.ell(0.0, 0.4, 0.48, 0.34, snow(p));
    p.ell(0.0, 0.38, 0.26, 0.18, shade(ice(p), -0.1));
    p.ell(0.0, 0.38, 0.2, 0.13, shade(p.k.water, -0.1));
    p.ell(-0.06, 0.4, 0.06, 0.03, shade(p.k.water, 0.4));
    for u in [-0.3, 0.3] {
        p.rect(u - 0.01, 0.46, u + 0.01, 0.9, p.k.accent);
    }
}

pub fn aurora_seat(p: &mut Pen) {
    p.shadow(0.44);
    let body = ice(p);
    p.rr(-0.4, 0.0, 0.4, 0.3, 0.03, body);
    p.poly(
        &[(-0.4, 0.3), (-0.2, 0.3), (-0.36, 0.86), (-0.46, 0.8)],
        shade(body, 0.1),
    );
    p.rect(-0.4, 0.28, 0.4, 0.34, snow(p));
    p.rr(-0.2, 0.34, 0.3, 0.4, 0.02, p.k.leaf);
    p.curve(
        (0.0, 0.7),
        (0.2, 0.96),
        (0.46, 0.8),
        0.03,
        p.k.second.opacity(0.5),
    );
}

pub fn nest_row(p: &mut Pen) {
    p.shadow(0.48);
    for u in [-0.32, 0.0, 0.32] {
        for step in 0..4 {
            let a = u - 0.12 + step as f32 * 0.08;
            pebble(p, a, 0.06, 0.05, (step % 2) as f32 * 0.08);
        }
        p.ell(u, 0.2, 0.12, 0.1, shade(p.k.stone, 0.15));
        p.ell(u, 0.24, 0.05, 0.07, p.k.pale);
    }
}

pub fn kelp_racks(p: &mut Pen) {
    p.shadow(0.46);
    let bone = p.k.brass;
    for u in [-0.42, 0.0, 0.42] {
        p.rect(u - 0.018, 0.0, u + 0.018, 0.86, bone);
    }
    for v in [0.84, 0.54] {
        p.line((-0.44, v), (0.44, v), 0.014, bone);
        for step in 0..8 {
            let u = -0.38 + step as f32 * 0.107;
            p.curve(
                (u, v),
                (u + 0.03, v - 0.12),
                (u - 0.01, v - 0.24),
                0.02,
                shade(p.k.leaf, (step % 2) as f32 * 0.1),
            );
        }
    }
}

pub fn bridge_gate(p: &mut Pen) {
    p.shadow(0.44);
    let body = ice(p);
    for u in [-0.34, 0.34] {
        p.rr(u - 0.1, 0.0, u + 0.1, 0.8, 0.02, body);
        p.dome(u, 0.8, 0.1, 0.08, snow(p));
    }
    p.curve((-0.3, 0.7), (0.0, 0.96), (0.3, 0.7), 0.04, p.k.brass);
    for step in 0..4 {
        let u = -0.18 + step as f32 * 0.12;
        p.rect(u - 0.01, 0.0, u + 0.01, 0.46, p.k.brass);
    }
    p.line((-0.24, 0.4), (0.24, 0.4), 0.012, p.k.brass);
    p.light(0.0, 0.84, 0.02, p.k.accent);
}

pub fn seal_watch(p: &mut Pen) {
    p.shadow(0.3);
    let bone = p.k.brass;
    p.line((-0.2, 0.0), (-0.1, 0.7), 0.024, bone);
    p.line((0.2, 0.0), (0.1, 0.7), 0.024, bone);
    p.line((-0.16, 0.3), (0.16, 0.34), 0.012, bone);
    p.rr(-0.22, 0.7, 0.22, 0.84, 0.02, ice(p));
    p.rect(-0.22, 0.82, 0.22, 0.88, snow(p));
    p.ell(0.36, 0.08, 0.14, 0.06, hex(0x7a8088));
    p.circ(0.46, 0.14, 0.04, hex(0x7a8088));
}

pub fn snow_hall(p: &mut Pen) {
    p.shadow(0.48);
    let body = snow(p);
    p.rr(-0.48, 0.0, 0.48, 0.5, 0.2, body);
    p.dome(0.0, 0.3, 0.46, 0.64, body);
    p.dome(-0.1, 0.3, 0.26, 0.56, shade(body, 0.06));
    for step in 1..4 {
        let v = 0.3 + step as f32 * 0.15;
        let s = 0.46 * (1.0 - ((v - 0.3) / 0.64).powi(2)).max(0.0).sqrt();
        p.line((-s, v), (s, v), 0.006, hex(0xc8dce8));
    }
    p.arch(-0.12, 0.12, 0.0, 0.34, shade(p.k.dark, 0.2));
    p.pennants((-0.4, 0.46), (0.4, 0.46), 7, &[p.k.accent, p.k.second]);
}

pub fn run_markers(p: &mut Pen) {
    p.ell(0.0, 0.14, 0.5, 0.1, p.k.water);
    for (u, tall) in [(-0.36, 0.5), (-0.1, 0.66), (0.14, 0.8), (0.38, 0.62)] {
        p.rect(u - 0.012, 0.1, u + 0.012, tall, p.k.brass);
        p.poly(
            &[(u, tall), (u + 0.1, tall - 0.04), (u, tall - 0.08)],
            p.k.accent,
        );
    }
    for u in [-0.2, 0.26] {
        p.ell(u, 0.14, 0.05, 0.02, hex(0x9ab0c0));
    }
}

pub fn far_lantern(p: &mut Pen) {
    p.shadow(0.24);
    p.ell(0.0, 0.06, 0.3, 0.08, ice(p));
    p.rect(-0.02, 0.06, 0.02, 0.8, p.k.dark);
    p.rr(-0.12, 0.8, 0.12, 0.96, 0.03, p.k.accent);
    p.rr(-0.08, 0.82, 0.08, 0.94, 0.02, hex(0xffd27a));
    p.circ(0.0, 0.88, 0.2, hex(0xffd27a).opacity(0.18));
}

pub fn chick_slide(p: &mut Pen) {
    let body = snow(p);
    p.poly(
        &[(-0.4, 0.0), (-0.4, 0.66), (-0.24, 0.66), (0.4, 0.0)],
        body,
    );
    p.curve((-0.3, 0.64), (0.0, 0.3), (0.38, 0.04), 0.04, ice(p));
    for step in 0..4 {
        let v = step as f32 * 0.15;
        p.rect(-0.48, v, -0.4, v + 0.06, ice(p));
    }
    p.ell(0.3, 0.12, 0.05, 0.07, hex(0x9a9a92));
    p.circ(0.33, 0.2, 0.035, hex(0x9a9a92));
}

pub fn bone_arch(p: &mut Pen) {
    p.shadow(0.44);
    let bone = p.k.brass;
    p.curve((-0.4, 0.0), (-0.5, 0.9), (0.0, 0.96), 0.06, bone);
    p.curve((0.4, 0.0), (0.5, 0.9), (0.0, 0.96), 0.06, bone);
    p.curve(
        (-0.4, 0.0),
        (-0.5, 0.9),
        (0.0, 0.96),
        0.02,
        shade(bone, 0.25),
    );
    for u in [-0.4, 0.4] {
        p.ell(u, 0.04, 0.1, 0.05, snow(p));
    }
}

pub fn salt_pans(p: &mut Pen) {
    for (u, r) in [(-0.3, 0.18), (0.1, 0.2), (0.38, 0.1)] {
        p.ell(u, 0.4, r, 0.34, p.k.stone);
        p.ell(u, 0.42, r * 0.8, 0.24, p.k.water);
        p.ell(u - r * 0.2, 0.44, r * 0.4, 0.1, p.k.pale);
    }
}

pub fn thaw_channel(p: &mut Pen) {
    let body = ice(p);
    p.rect(-0.5, 0.0, 0.5, 0.9, snow(p));
    p.poly(&[(-0.5, 0.3), (0.5, 0.2), (0.5, 0.56), (-0.5, 0.66)], body);
    p.poly(
        &[(-0.5, 0.4), (0.5, 0.3), (0.5, 0.46), (-0.5, 0.56)],
        p.k.water,
    );
    for u in [-0.3, 0.1, 0.36] {
        p.ell(u, 0.44 - u * 0.1, 0.05, 0.04, shade(body, 0.2));
    }
}

pub fn swim_pool(p: &mut Pen) {
    p.ell(0.0, 0.3, 0.5, 0.3, snow(p));
    p.ell(0.0, 0.3, 0.42, 0.22, shade(ice(p), -0.1));
    p.ell(0.0, 0.3, 0.36, 0.16, p.k.water);
    p.ell(-0.1, 0.3, 0.08, 0.05, hex(0x2d3a4a));
    p.circ(-0.02, 0.34, 0.04, hex(0x2d3a4a));
    for u in [-0.46, 0.46] {
        p.rect(u - 0.01, 0.3, u + 0.01, 0.9, p.k.accent);
    }
    p.line((-0.46, 0.86), (0.46, 0.86), 0.006, p.k.dark);
}

pub fn pebble_market(p: &mut Pen) {
    p.shadow(0.48);
    for (u, ink) in [(-0.26, p.k.accent), (0.26, p.k.second)] {
        p.rr(u - 0.2, 0.0, u + 0.2, 0.32, 0.03, ice(p));
        p.rect(u - 0.2, 0.3, u + 0.2, 0.34, snow(p));
        for step in 0..4 {
            pebble(
                p,
                u - 0.12 + step as f32 * 0.08,
                0.38,
                0.035,
                (step % 3) as f32 * 0.1,
            );
        }
        for side in [-0.18, 0.18] {
            p.rect(u + side - 0.012, 0.34, u + side + 0.012, 0.8, p.k.brass);
        }
        p.poly(&[(u - 0.22, 0.8), (u + 0.22, 0.8), (u, 0.96)], ink);
    }
}

pub fn night_beacon(p: &mut Pen) {
    p.shadow(0.3);
    let body = ice(p);
    for row in 0..5 {
        let v = row as f32 * 0.14;
        let s = 0.24 - row as f32 * 0.035;
        p.rr(
            -s,
            v,
            s,
            v + 0.13,
            0.02,
            shade(body, (row % 2) as f32 * 0.1),
        );
    }
    p.rect(-0.08, 0.7, 0.08, 0.74, snow(p));
    p.circ(0.0, 0.84, 0.1, hex(0xffd27a));
    p.circ(0.0, 0.84, 0.22, hex(0xffd27a).opacity(0.2));
}

pub fn elders_ramp(p: &mut Pen) {
    let body = snow(p);
    p.poly(
        &[(-0.5, 0.0), (0.5, 0.0), (0.5, 0.7), (0.3, 0.7)],
        shade(body, -0.04),
    );
    p.poly(
        &[(-0.5, 0.0), (-0.3, 0.0), (0.34, 0.66), (0.3, 0.7)],
        ice(p),
    );
    for step in 0..6 {
        let t = step as f32 / 6.0;
        let (u, v) = (-0.36 + t * 0.66, 0.06 + t * 0.6);
        p.rect(u - 0.01, v, u + 0.01, v + 0.24, p.k.brass);
    }
    p.line((-0.36, 0.3), (0.3, 0.9), 0.012, p.k.brass);
}

pub fn name_wall(p: &mut Pen) {
    let body = ice(p);
    p.rr(-0.48, 0.0, 0.48, 0.84, 0.03, body);
    p.dome(0.0, 0.8, 0.48, 0.2, snow(p));
    for row in 0..4 {
        for col in 0..3 {
            let (u, v) = (-0.36 + col as f32 * 0.28, 0.14 + row as f32 * 0.17);
            let w = 0.08 + 0.04 * ((row + col) % 2) as f32;
            p.line((u, v), (u + w, v), 0.014, hex(0x6a9ab0));
        }
    }
}

pub fn shell_horn(p: &mut Pen) {
    p.shadow(0.2);
    p.rect(-0.02, 0.0, 0.02, 0.62, p.k.brass);
    p.poly(
        &[(0.0, 0.66), (0.36, 0.84), (0.4, 0.66), (0.36, 0.54)],
        hex(0xf2d8cc),
    );
    p.ell(0.38, 0.69, 0.04, 0.14, hex(0xe8b8a8));
    p.curve((0.06, 0.66), (0.2, 0.7), (0.34, 0.76), 0.006, hex(0xc89888));
    for r in [0.08, 0.14] {
        p.curve(
            (0.44 + r * 0.2, 0.69 - r),
            (0.46 + r, 0.69),
            (0.44 + r * 0.2, 0.69 + r),
            0.008,
            p.k.second,
        );
    }
}

pub fn floe_bridge(p: &mut Pen) {
    p.rect(-0.5, 0.0, 0.5, 0.24, p.k.water);
    for (u0, u1) in [(-0.5, -0.24), (0.24, 0.5)] {
        p.poly(
            &[(u0, 0.14), (u1, 0.14), (u1 - 0.02, 0.5), (u0 + 0.02, 0.5)],
            ice(p),
        );
        p.rect(u0 + 0.02, 0.46, u1 - 0.02, 0.54, snow(p));
    }
    for step in 0..7 {
        let u = -0.24 + step as f32 * 0.08;
        p.rect(u, 0.44, u + 0.06, 0.52, p.k.wood);
    }
    p.line((-0.24, 0.72), (0.24, 0.72), 0.01, p.k.leaf);
    for u in [-0.24, 0.24] {
        p.rect(u - 0.012, 0.5, u + 0.012, 0.76, p.k.brass);
    }
}

pub fn chick_nursery(p: &mut Pen) {
    p.shadow(0.48);
    blocks_dome(p, 0.1, 0.36, 0.7);
    for (u, v) in [(-0.38, 0.0), (-0.24, 0.0)] {
        p.ell(u, v + 0.12, 0.06, 0.1, hex(0x9a9a92));
        p.circ(u, v + 0.24, 0.045, hex(0x9a9a92));
        p.circ(u + 0.015, v + 0.25, 0.01, p.k.dark);
    }
    p.rect(-0.2, 0.62, -0.02, 0.66, p.k.accent);
}

pub fn song_circle(p: &mut Pen) {
    p.ell(0.0, 0.2, 0.48, 0.16, shade(snow(p), -0.05));
    for (index, u) in [-0.4, -0.2, 0.0, 0.2, 0.4].iter().enumerate() {
        let tall = 0.4 + 0.2 * (1.0 - (u * 2.0_f32).abs());
        p.rr(
            u - 0.05,
            0.12,
            u + 0.05,
            0.12 + tall,
            0.03,
            shade(ice(p), index as f32 * 0.03),
        );
    }
    p.circ(0.0, 0.94, 0.03, p.k.second);
    p.line((0.03, 0.94), (0.03, 1.0), 0.008, p.k.second);
}

pub fn fish_larder(p: &mut Pen) {
    p.shadow(0.4);
    let body = ice(p);
    p.rr(-0.34, 0.0, 0.34, 0.56, 0.03, body);
    p.rect(-0.34, 0.54, 0.34, 0.62, snow(p));
    p.rr(-0.24, 0.1, 0.24, 0.44, 0.02, shade(body, -0.15));
    for v in [0.16, 0.26, 0.36] {
        p.ell(0.0, v, 0.14, 0.035, hex(0x9ab0c0));
    }
    p.line((-0.34, 0.66), (0.34, 0.66), 0.0, body);
    p.poly(&[(-0.12, 0.62), (0.12, 0.62), (0.0, 0.86)], p.k.accent);
}

pub fn lantern_ring(p: &mut Pen) {
    p.ell(0.0, 0.2, 0.48, 0.14, shade(snow(p), -0.05));
    for step in 0..7 {
        let a = step as f32 * std::f32::consts::TAU / 7.0;
        let (u, v) = (a.cos() * 0.4, 0.2 + a.sin() * 0.12);
        p.rect(u - 0.01, v, u + 0.01, v + 0.3, p.k.brass);
        p.light(u, v + 0.34, 0.03, p.k.accent);
    }
}

pub fn story_berg(p: &mut Pen) {
    let body = ice(p);
    p.poly(
        &[
            (-0.4, 0.0),
            (0.4, 0.0),
            (0.3, 0.5),
            (0.06, 0.9),
            (-0.2, 0.7),
            (-0.34, 0.36),
        ],
        body,
    );
    p.poly(
        &[(0.0, 0.0), (0.4, 0.0), (0.3, 0.5), (0.06, 0.9)],
        shade(body, 0.2),
    );
    for (u, v) in [(-0.1, 0.3), (0.1, 0.44), (-0.04, 0.58)] {
        p.circ(u, v, 0.04, hex(0x6a9ab0));
    }
    for u in [-0.46, 0.46] {
        p.rr(u - 0.05, 0.0, u + 0.05, 0.1, 0.02, p.k.stone);
    }
}

pub fn pebble_garden(p: &mut Pen) {
    p.ell(0.0, 0.4, 0.48, 0.36, shade(snow(p), -0.06));
    for step in 0..14 {
        let a = step as f32 * 2.4;
        let r = 0.1 + (step as f32 * 0.03) % 0.3;
        pebble(
            p,
            a.cos() * r,
            0.4 + a.sin() * r * 0.6,
            0.04,
            (step % 3) as f32 * 0.1 - 0.05,
        );
    }
    p.circ(0.0, 0.4, 0.05, p.k.accent);
}

pub fn snow_house(p: &mut Pen) {
    p.shadow(0.46);
    blocks_dome(p, -0.1, 0.34, 0.66);
    blocks_dome(p, 0.26, 0.2, 0.4);
    p.circ(-0.14, 0.4, 0.03, p.glass);
}

pub fn ice_slide(p: &mut Pen) {
    p.shadow(0.4);
    let body = ice(p);
    p.poly(
        &[(-0.44, 0.0), (-0.44, 0.9), (-0.3, 0.9), (-0.3, 0.0)],
        snow(p),
    );
    for step in 0..5 {
        let v = step as f32 * 0.18;
        p.rect(-0.3, v, -0.2, v + 0.04, body);
    }
    p.curve((-0.3, 0.88), (0.1, 0.8), (0.46, 0.04), 0.06, body);
    p.curve(
        (-0.3, 0.88),
        (0.1, 0.8),
        (0.46, 0.04),
        0.02,
        shade(body, 0.25),
    );
}

pub fn ice_market(p: &mut Pen) {
    p.shadow(0.48);
    let body = ice(p);
    for step in 0..3 {
        let u = -0.32 + step as f32 * 0.32;
        p.rr(u - 0.14, 0.0, u + 0.14, 0.3, 0.02, body);
        p.rect(u - 0.14, 0.28, u + 0.14, 0.32, snow(p));
    }
    for u in [-0.46, 0.46] {
        p.rect(u - 0.015, 0.0, u + 0.015, 0.76, p.k.brass);
    }
    p.curve((-0.46, 0.76), (0.0, 0.9), (0.46, 0.76), 0.05, p.k.second);
    for (u, ink) in [(-0.32, hex(0x9ab0c0)), (0.0, p.k.leaf), (0.32, p.k.stone)] {
        p.ell(u, 0.36, 0.08, 0.04, ink);
    }
}

pub fn carving_hall(p: &mut Pen) {
    p.shadow(0.48);
    blocks_dome(p, -0.04, 0.44, 0.84);
    p.ell(-0.24, 0.12, 0.05, 0.1, ice(p));
    p.circ(-0.24, 0.26, 0.04, ice(p));
    p.line((0.3, 0.0), (0.44, 0.3), 0.014, p.k.brass);
    p.poly(&[(0.4, 0.26), (0.5, 0.36), (0.46, 0.22)], p.k.metal);
}

pub fn lookout_post(p: &mut Pen) {
    p.shadow(0.24);
    let bone = p.k.brass;
    p.rect(-0.03, 0.0, 0.03, 0.8, bone);
    for step in 0..6 {
        let v = 0.08 + step as f32 * 0.12;
        p.line((-0.1, v), (0.1, v), 0.012, bone);
    }
    p.rr(-0.18, 0.8, 0.18, 0.88, 0.02, ice(p));
    p.rect(-0.18, 0.86, 0.18, 0.9, snow(p));
    p.poly(&[(0.0, 0.9), (0.0, 1.0), (0.14, 0.95)], p.k.accent);
}

pub fn skating_rink(p: &mut Pen) {
    p.ell(0.0, 0.4, 0.5, 0.36, snow(p));
    p.ell(0.0, 0.4, 0.44, 0.28, hex(0xd8ecf2));
    for (bend, v) in [(0.2, 0.5), (-0.2, 0.34)] {
        p.curve(
            (-0.3, v),
            (0.0, v + bend * 0.5),
            (0.3, v - 0.02),
            0.006,
            hex(0xa8c8d8),
        );
    }
    p.ell(0.14, 0.44, 0.04, 0.08, hex(0x2d3a4a));
    p.circ(0.14, 0.58, 0.03, hex(0x2d3a4a));
}

pub fn bell_post(p: &mut Pen) {
    p.shadow(0.24);
    let bone = p.k.brass;
    for u in [-0.22, 0.22] {
        p.rect(u - 0.025, 0.0, u + 0.025, 0.86, bone);
    }
    p.rect(-0.26, 0.84, 0.26, 0.9, bone);
    p.line((0.0, 0.84), (0.0, 0.78), 0.008, p.k.dark);
    p.poly(
        &[(-0.1, 0.56), (0.1, 0.56), (0.06, 0.78), (-0.06, 0.78)],
        hex(0xf2d8cc),
    );
    p.circ(0.0, 0.54, 0.02, p.k.dark);
    p.ell(0.0, 0.04, 0.34, 0.06, snow(p));
}

pub fn snow_arch(p: &mut Pen) {
    p.shadow(0.44);
    let body = snow(p);
    p.poly(
        &[
            (-0.46, 0.0),
            (-0.24, 0.0),
            (-0.24, 0.5),
            (0.24, 0.5),
            (0.24, 0.0),
            (0.46, 0.0),
            (0.46, 0.66),
            (-0.46, 0.66),
        ],
        body,
    );
    p.dome(0.0, 0.5, 0.24, 0.26, body);
    p.dome(0.0, 0.66, 0.46, 0.3, body);
    for v in [0.2, 0.4] {
        p.line((-0.46, v), (-0.24, v), 0.006, hex(0xc8dce8));
        p.line((0.24, v), (0.46, v), 0.006, hex(0xc8dce8));
    }
    p.circ(0.0, 0.84, 0.04, p.k.accent);
}

pub fn warming_hut(p: &mut Pen) {
    p.shadow(0.46);
    blocks_dome(p, -0.06, 0.38, 0.72);
    p.rect(0.06, 0.6, 0.14, 0.9, p.k.stone);
    for (u, v, r) in [(0.1, 0.94, 0.05), (0.16, 1.0, 0.06)] {
        p.circ(u, v - 0.04, r, p.k.pale.opacity(0.6));
    }
    p.circ(-0.1, 0.36, 0.06, hex(0xffd27a));
}

pub fn kayak_shelter(p: &mut Pen) {
    p.shadow(0.48);
    let body = snow(p);
    p.rr(-0.48, 0.0, 0.48, 0.66, 0.1, body);
    p.rr(-0.4, 0.0, 0.4, 0.5, 0.06, shade(p.k.dark, 0.4));
    for (v, ink) in [(0.1, p.k.wood), (0.28, shade(p.k.wood, 0.2))] {
        p.poly(
            &[(-0.38, v + 0.08), (0.38, v + 0.08), (0.3, v), (-0.3, v)],
            ink,
        );
    }
    p.line((-0.3, 0.44), (0.3, 0.4), 0.012, p.k.brass);
}

pub fn whale_watch(p: &mut Pen) {
    p.shadow(0.3);
    let body = ice(p);
    p.poly(&[(-0.3, 0.0), (0.3, 0.0), (0.2, 0.62), (-0.2, 0.62)], body);
    p.rect(-0.26, 0.62, 0.26, 0.68, snow(p));
    p.rect(-0.02, 0.68, 0.02, 0.9, p.k.brass);
    p.poly(&[(0.02, 0.9), (0.2, 0.86), (0.02, 0.82)], p.k.accent);
    p.curve((0.3, 0.06), (0.44, 0.3), (0.36, 0.44), 0.03, p.k.dark);
    p.poly(&[(0.3, 0.44), (0.42, 0.52), (0.46, 0.4)], p.k.dark);
}

pub fn sail_sled(p: &mut Pen) {
    p.shadow(0.44);
    let wood = p.k.wood;
    p.line((-0.44, 0.06), (0.4, 0.06), 0.03, wood);
    p.curve((0.4, 0.06), (0.5, 0.08), (0.46, 0.2), 0.03, wood);
    p.rect(-0.36, 0.08, 0.3, 0.2, shade(wood, 0.1));
    p.line((-0.04, 0.2), (-0.04, 0.98), 0.016, p.k.brass);
    p.poly(
        &[(-0.02, 0.96), (-0.02, 0.26), (0.4 + 0.04 * p.sway, 0.26)],
        p.k.pale,
    );
    p.line((-0.02, 0.24), (0.4, 0.24), 0.012, p.k.brass);
}

pub fn moss_patch(p: &mut Pen) {
    p.ell(0.0, 0.3, 0.48, 0.3, p.k.stone);
    for step in 0..8 {
        let u = -0.36 + step as f32 * 0.1;
        let v = 0.34 + 0.12 * ((step * 3) % 4) as f32 / 4.0;
        p.circ(u, v, 0.07, shade(p.k.leaf, (step % 3) as f32 * 0.12));
    }
}

pub fn seal_fence(p: &mut Pen) {
    p.ell(0.0, 0.06, 0.5, 0.08, p.k.water);
    for step in 0..6 {
        let u = -0.44 + step as f32 * 0.176;
        p.rect(u - 0.02, 0.0, u + 0.02, 0.8, p.k.brass);
    }
    for v in [0.36, 0.66] {
        p.curve((-0.46, v), (0.0, v - 0.08), (0.46, v), 0.012, p.k.leaf);
    }
}

pub fn far_beacon(p: &mut Pen) {
    p.shadow(0.3);
    let bone = p.k.brass;
    p.line((-0.2, 0.0), (0.0, 0.8), 0.024, bone);
    p.line((0.2, 0.0), (0.0, 0.8), 0.024, bone);
    p.line((0.0, 0.0), (0.0, 0.8), 0.02, bone);
    p.poly(
        &[(-0.14, 0.8), (0.14, 0.8), (0.1, 0.88), (-0.1, 0.88)],
        p.k.dark,
    );
    p.poly(
        &[
            (-0.1, 0.88),
            (-0.04, 1.0),
            (0.0, 0.92),
            (0.06, 1.0),
            (0.1, 0.88),
        ],
        hex(0xf29a3c),
    );
}

pub fn snow_maze(p: &mut Pen) {
    let body = snow(p);
    p.ell(0.0, 0.3, 0.5, 0.3, shade(body, -0.06));
    for (u0, v0, u1, v1) in [
        (-0.42, 0.14, 0.3, 0.14),
        (-0.3, 0.3, 0.42, 0.3),
        (-0.42, 0.46, 0.2, 0.46),
        (-0.2, 0.6, 0.3, 0.6),
    ] {
        p.rr(u0, v0, u1, v0 + 0.08, 0.02, body);
        p.rect(u0, v1, u1, v1 + 0.02, hex(0xc8dce8));
    }
    p.rr(0.28, 0.14, 0.36, 0.38, 0.02, body);
}

pub fn glow_stones(p: &mut Pen) {
    for (index, u) in [-0.34, -0.1, 0.14, 0.36].iter().enumerate() {
        let r = 0.08 + 0.03 * (index % 2) as f32;
        p.circ(*u, r * 0.9, r * 1.8, hex(0xb9f3d3).opacity(0.25));
        p.ell(*u, r * 0.9, r, r * 1.6, hex(0xb9f3d3));
    }
    p.rect(-0.5, 0.0, 0.5, 0.05, snow(p));
}

pub fn snow_flag(p: &mut Pen) {
    p.shadow(0.2);
    p.ell(0.0, 0.04, 0.16, 0.05, snow(p));
    p.rect(-0.015, 0.0, 0.015, 0.98, p.k.brass);
    p.poly(
        &[(0.015, 0.96), (0.4, 0.9 + 0.02 * p.sway), (0.015, 0.8)],
        p.k.second,
    );
    p.poly(&[(0.015, 0.92), (0.2, 0.89), (0.015, 0.85)], p.k.pale);
}

pub fn pennant_line(p: &mut Pen) {
    let bone = p.k.brass;
    for u in [-0.46, 0.46] {
        p.rect(u - 0.015, 0.0, u + 0.015, 0.9, bone);
        p.ell(u, 0.02, 0.06, 0.03, snow(p));
    }
    p.pennants(
        (-0.46, 0.88),
        (0.46, 0.88),
        8,
        &[p.k.accent, p.k.second, p.k.pale],
    );
}

pub fn geyser_fountain(p: &mut Pen) {
    p.shadow(0.4);
    let body = ice(p);
    p.rr(-0.36, 0.0, 0.36, 0.2, 0.04, body);
    p.ell(0.0, 0.2, 0.36, 0.05, snow(p));
    p.ell(0.0, 0.2, 0.26, 0.035, p.k.water);
    let water = shade(p.k.water, 0.5).opacity(0.85);
    p.line((0.0, 0.2), (0.0, 0.82), 0.05, water);
    for side in [-1.0_f32, 1.0] {
        p.curve(
            (0.0, 0.8),
            (side * 0.2, 0.96),
            (side * 0.26, 0.26),
            0.016,
            water,
        );
    }
    p.circ(0.0, 0.88, 0.1, p.k.pale.opacity(0.6));
}

pub fn seaweed_bed(p: &mut Pen) {
    p.ell(0.0, 0.3, 0.5, 0.3, p.k.stone);
    for step in 0..8 {
        let u = -0.38 + step as f32 * 0.11;
        p.curve(
            (u, 0.2),
            (u + 0.06, 0.5),
            (u - 0.02, 0.7),
            0.035,
            shade(hex(0x6a5a2a), (step % 2) as f32 * 0.1),
        );
    }
}

pub fn lichen_patch(p: &mut Pen) {
    for (u, r) in [(-0.26, 0.2), (0.16, 0.24)] {
        p.ell(u, 0.3, r, 0.3, p.k.stone);
        for step in 0..4 {
            let a = u - r * 0.6 + step as f32 * r * 0.4;
            p.circ(
                a,
                0.4,
                r * 0.18,
                [hex(0xd8a84a), hex(0x9ab04a), hex(0xd88a4a)][step % 3],
            );
        }
    }
}

pub fn fishing_hole_bench(p: &mut Pen) {
    p.ell(0.0, 0.14, 0.5, 0.14, snow(p));
    p.ell(0.24, 0.14, 0.14, 0.06, p.k.water);
    p.rr(-0.44, 0.28, 0.0, 0.38, 0.02, ice(p));
    for u in [-0.4, -0.04] {
        p.rect(u - 0.03, 0.1, u + 0.03, 0.28, shade(ice(p), -0.1));
    }
    p.line((0.0, 0.4), (0.3, 0.9), 0.012, p.k.brass);
    p.curve((0.3, 0.9), (0.28, 0.5), (0.24, 0.16), 0.004, p.k.dark);
}

pub fn second_nests(p: &mut Pen) {
    p.shadow(0.48);
    blocks_dome(p, -0.24, 0.24, 0.5);
    blocks_dome(p, 0.2, 0.26, 0.56);
    for u in [-0.46, 0.0, 0.46] {
        pebble(p, u, 0.03, 0.04, 0.0);
    }
}

pub fn ice_beacon(p: &mut Pen) {
    p.shadow(0.3);
    let body = ice(p);
    p.poly(&[(-0.22, 0.0), (0.22, 0.0), (0.1, 0.8), (-0.1, 0.8)], body);
    p.poly(
        &[(0.0, 0.0), (0.22, 0.0), (0.1, 0.8), (0.0, 0.8)],
        shade(body, 0.2),
    );
    p.circ(0.0, 0.88, 0.09, p.k.accent);
    p.circ(0.0, 0.88, 0.18, p.k.accent.opacity(0.2));
}

pub fn ice_survey(p: &mut Pen) {
    p.shadow(0.36);
    let bone = p.k.brass;
    p.line((-0.24, 0.0), (0.0, 0.56), 0.016, bone);
    p.line((0.24, 0.0), (0.0, 0.56), 0.016, bone);
    p.rr(-0.1, 0.56, 0.1, 0.66, 0.02, ice(p));
    p.rect(0.36, 0.0, 0.4, 0.9, p.k.dark);
    for step in 0..4 {
        p.rect(
            0.36,
            0.1 + step as f32 * 0.2,
            0.4,
            0.2 + step as f32 * 0.2,
            p.k.accent,
        );
    }
    p.ell(0.0, 0.04, 0.3, 0.05, snow(p));
}

/// A garden of ice sculptures on snow plinths: a leaping fish, a seal and
/// a spiral, each with its own sheen.
pub fn sculpture_garden(p: &mut Pen) {
    p.shadow(0.48);
    let body = ice(p);
    for u in [-0.32, 0.0, 0.32] {
        p.rect(u - 0.1, 0.0, u + 0.1, 0.14, snow(p));
    }
    // A fish leaping.
    p.ell(-0.32, 0.4, 0.07, 0.16, body);
    p.poly(&[(-0.32, 0.56), (-0.4, 0.66), (-0.24, 0.66)], body);
    p.circ(-0.3, 0.3, 0.012, hex(0x6a9ab0));
    // A seal, nose up, with a face.
    p.ell(0.0, 0.26, 0.1, 0.12, body);
    p.circ(0.02, 0.42, 0.06, body);
    p.face(0.02, 0.42, 0.06, hex(0x6a9ab0));
    // A spiral rising.
    p.poly(
        &[
            (0.24, 0.14),
            (0.4, 0.14),
            (0.34, 0.5),
            (0.3, 0.74),
            (0.26, 0.5),
        ],
        body,
    );
    p.curve(
        (0.26, 0.2),
        (0.38, 0.34),
        (0.28, 0.46),
        0.012,
        shade(body, 0.3),
    );
    p.curve(
        (0.28, 0.46),
        (0.36, 0.56),
        (0.3, 0.66),
        0.012,
        shade(body, 0.3),
    );
}
