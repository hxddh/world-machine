//! Ares's drawings: pressurised domes and hab modules of pale composite
//! banded in burnt orange, masts and dishes, solar panels, rovers' sheds
//! and growing things under glass. Nothing here would stand on a shore.

use super::Pen;
use crate::art::{hex, shade};

fn panel(p: &Pen) -> gpui::Hsla {
    p.wall
}

/// A lying capsule of a module on short legs, from `u0` to `u1`, its
/// floor at `v0` and roof at `v1`, banded and with round windows.
fn capsule(p: &mut Pen, u0: f32, u1: f32, v0: f32, v1: f32, windows: usize) {
    let body = panel(p);
    let metal = p.k.metal;
    for u in [u0 + 0.08, u1 - 0.08] {
        p.line((u - 0.03, 0.0), (u, v0), 0.016, metal);
        p.line((u + 0.03, 0.0), (u, v0), 0.016, metal);
    }
    let r = (v1 - v0) * p.h / p.w / 2.0;
    p.rr(u0, v0, u1, v1, r, body);
    p.rr(u0, v0, u1, v0 + (v1 - v0) * 0.3, r * 0.6, shade(body, -0.1));
    let band = p.roof;
    p.rect(
        u0 + r * 0.6,
        v0 + (v1 - v0) * 0.62,
        u1 - r * 0.6,
        v0 + (v1 - v0) * 0.72,
        band,
    );
    for index in 0..windows {
        let u = u0 + (u1 - u0) * (index as f32 + 1.0) / (windows as f32 + 1.0);
        p.porthole(u, v0 + (v1 - v0) * 0.45, (v1 - v0) * 0.1 * p.h / p.w);
    }
}

/// A pressure dome: a white hemisphere on a ring, ribbed, a door.
fn pressure_dome(p: &mut Pen, u: f32, ru: f32, rv: f32) {
    let body = panel(p);
    p.rect(u - ru, 0.0, u + ru, 0.08, shade(body, -0.15));
    p.dome(u, 0.08, ru, rv, body);
    p.dome(u + ru * 0.25, 0.08, ru * 0.6, rv * 0.92, shade(body, 0.06));
    let rib = shade(body, -0.14);
    for t in [-0.5_f32, 0.0, 0.5] {
        p.curve(
            (u + ru * t, 0.08),
            (u + ru * t * 1.1, 0.08 + rv * 0.7),
            (u + ru * t * 0.2, 0.08 + rv * 0.98),
            0.006,
            rib,
        );
    }
}

/// An airlock door: a rounded hatch with its wheel.
fn hatch(p: &mut Pen, u: f32, v0: f32, half: f32, tall: f32) {
    p.rr(u - half, v0, u + half, v0 + tall, half * 0.6, p.k.metal);
    p.rr(
        u - half * 0.7,
        v0 + tall * 0.1,
        u + half * 0.7,
        v0 + tall * 0.9,
        half * 0.4,
        shade(p.k.metal, -0.2),
    );
    p.circ(u, v0 + tall * 0.5, half * 0.3, p.k.accent);
}

pub fn hab_module(p: &mut Pen) {
    p.shadow(0.46);
    capsule(p, -0.46, 0.46, 0.12, 0.86, 3);
    hatch(p, 0.36, 0.12, 0.05, 0.5);
}

pub fn supply_module(p: &mut Pen) {
    p.shadow(0.46);
    let body = panel(p);
    p.rr(-0.44, 0.0, 0.44, 0.7, 0.04, body);
    p.rect(-0.44, 0.52, 0.44, 0.6, p.roof);
    p.rect(-0.3, 0.0, 0.1, 0.44, shade(p.k.metal, -0.1));
    for v in [0.08, 0.16, 0.24, 0.32] {
        p.line((-0.3, v), (0.1, v), 0.006, shade(p.k.metal, 0.2));
    }
    for (u, v) in [(0.22, 0.0), (0.34, 0.0), (0.28, 0.14)] {
        p.rect(u - 0.06, v, u + 0.06, v + 0.14, p.k.accent);
    }
    p.rect(-0.2, 0.7, 0.2, 0.76, p.k.metal);
}

pub fn comms_mast(p: &mut Pen) {
    p.shadow(0.3);
    let metal = p.k.metal;
    p.rect(-0.2, 0.0, 0.2, 0.05, shade(panel(p), -0.1));
    p.rect(-0.03, 0.05, 0.03, 0.9, metal);
    for step in 0..6 {
        let v = 0.1 + step as f32 * 0.13;
        p.line((-0.12, v), (0.12, v + 0.06), 0.008, metal);
    }
    p.line((-0.25, 0.0), (0.0, 0.6), 0.006, metal);
    p.line((0.25, 0.0), (0.0, 0.6), 0.006, metal);
    p.ell(0.1, 0.72, 0.1, 0.05, panel(p));
    p.line((0.0, 0.9), (0.0, 1.0), 0.01, metal);
    p.light(0.0, 1.0, 0.02, p.k.accent);
}

pub fn mars_dome(p: &mut Pen) {
    p.shadow(0.48);
    pressure_dome(p, 0.0, 0.46, 0.84);
    hatch(p, 0.0, 0.0, 0.07, 0.3);
    p.porthole(-0.24, 0.34, 0.05);
    p.porthole(0.24, 0.34, 0.05);
}

pub fn grow_dome(p: &mut Pen) {
    p.shadow(0.46);
    let glass = shade(p.k.water, 0.4).opacity(0.75);
    p.rect(-0.44, 0.0, 0.44, 0.1, shade(panel(p), -0.15));
    for (u, v, r) in [(-0.2, 0.22, 0.12), (0.05, 0.3, 0.16), (0.24, 0.2, 0.1)] {
        p.circ(u, v, r, p.k.leaf);
    }
    p.dome(0.0, 0.1, 0.44, 0.84, glass);
    let frame = p.k.pale;
    for t in [-0.7_f32, -0.35, 0.0, 0.35, 0.7] {
        p.curve(
            (t * 0.44, 0.1),
            (t * 0.5, 0.6),
            (t * 0.1, 0.93),
            0.006,
            frame,
        );
    }
    p.curve((-0.4, 0.45), (0.0, 0.52), (0.4, 0.45), 0.006, frame);
}

pub fn beacon_mast(p: &mut Pen) {
    p.shadow(0.2);
    let metal = p.k.metal;
    p.rect(-0.1, 0.0, 0.1, 0.06, metal);
    p.rect(-0.025, 0.06, 0.025, 0.88, panel(p));
    for v in [0.3, 0.6] {
        p.rect(-0.03, v, 0.03, v + 0.08, p.k.accent);
    }
    p.rr(-0.07, 0.88, 0.07, 0.98, 0.03, p.k.dark);
    p.light(0.0, 0.93, 0.035, p.k.accent);
}

pub fn dome_walkway(p: &mut Pen) {
    let body = panel(p);
    for u in [-0.3, 0.0, 0.3] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.4, p.k.metal);
    }
    p.rr(-0.5, 0.4, 0.5, 0.82, 0.2, body);
    p.rect(-0.5, 0.4, 0.5, 0.5, shade(body, -0.12));
    for step in 0..6 {
        let u = -0.4 + step as f32 * 0.16;
        p.line((u, 0.4), (u, 0.82), 0.006, shade(body, -0.18));
        p.rr(u + 0.03, 0.58, u + 0.12, 0.7, 0.03, p.glass);
    }
}

pub fn dust_sled(p: &mut Pen) {
    p.shadow(0.46);
    let metal = p.k.metal;
    p.line((-0.46, 0.1), (0.42, 0.1), 0.03, metal);
    p.curve((0.42, 0.1), (0.52, 0.12), (0.48, 0.3), 0.03, metal);
    for u in [-0.3, 0.2] {
        p.line((u, 0.1), (u, 0.3), 0.02, metal);
    }
    p.rect(-0.4, 0.3, 0.3, 0.42, p.k.accent);
    p.rect(-0.34, 0.42, 0.0, 0.7, panel(p));
    p.rect(0.02, 0.42, 0.24, 0.6, shade(panel(p), -0.1));
    p.line((-0.34, 0.56), (0.0, 0.56), 0.01, p.k.dark);
}

pub fn landing_pad(p: &mut Pen) {
    let body = shade(p.k.ground, -0.15);
    p.ell(0.0, 0.3, 0.5, 0.3, shade(body, -0.2));
    p.ell(0.0, 0.34, 0.46, 0.24, body);
    p.ell(0.0, 0.34, 0.38, 0.18, shade(body, 0.1));
    let paint = p.k.pale;
    p.rect(-0.1, 0.24, -0.07, 0.44, paint);
    p.rect(0.07, 0.24, 0.1, 0.44, paint);
    p.rect(-0.1, 0.32, 0.1, 0.36, paint);
    for (u, v) in [(-0.44, 0.34), (0.44, 0.34), (0.0, 0.1), (0.0, 0.58)] {
        p.light(u, v, 0.018, p.k.accent);
    }
}

pub fn grow_tray(p: &mut Pen) {
    p.shadow(0.46);
    let body = panel(p);
    p.rect(-0.44, 0.2, 0.44, 0.44, body);
    p.rect(-0.44, 0.4, 0.44, 0.46, p.k.second);
    for u in [-0.38, 0.38] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.2, p.k.metal);
    }
    for step in 0..7 {
        let u = -0.36 + step as f32 * 0.12;
        p.ell(u - 0.02, 0.56, 0.02, 0.08, p.k.leaf);
        p.ell(u + 0.02, 0.58, 0.02, 0.1, shade(p.k.leaf, 0.12));
    }
}

pub fn solar_lamp(p: &mut Pen) {
    p.shadow(0.2);
    let metal = p.k.metal;
    p.rect(-0.1, 0.0, 0.1, 0.04, metal);
    p.rect(-0.02, 0.04, 0.02, 0.84, panel(p));
    p.poly(
        &[(-0.2, 0.9), (0.2, 0.98), (0.22, 0.94), (-0.18, 0.86)],
        hex(0x2a3a5a),
    );
    p.rr(-0.06, 0.74, 0.06, 0.82, 0.02, p.glass);
    p.rect(-0.08, 0.82, 0.08, 0.84, metal);
}

pub fn supply_tent(p: &mut Pen) {
    p.shadow(0.46);
    let cloth = p.k.accent;
    p.dome(0.0, 0.0, 0.46, 0.82, cloth);
    for t in [-0.5_f32, 0.0, 0.5] {
        p.curve(
            (t * 0.46, 0.0),
            (t * 0.5, 0.6),
            (t * 0.1, 0.82),
            0.008,
            shade(cloth, -0.25),
        );
    }
    p.dome(0.0, 0.0, 0.14, 0.34, shade(cloth, -0.35));
    p.rect(-0.46, 0.0, 0.46, 0.04, p.k.metal);
}

pub fn metal_bench(p: &mut Pen) {
    p.shadow(0.45);
    let (body, metal) = (panel(p), p.k.metal);
    for u in [-0.34, 0.34] {
        p.rect(u - 0.03, 0.0, u + 0.03, 0.44, metal);
    }
    p.rr(-0.46, 0.44, 0.46, 0.54, 0.03, body);
    p.rr(-0.44, 0.62, 0.44, 0.9, 0.05, body);
    p.rect(-0.44, 0.7, 0.44, 0.74, p.k.accent);
}

pub fn condenser(p: &mut Pen) {
    p.shadow(0.36);
    let (body, metal) = (panel(p), p.k.metal);
    p.rr(-0.3, 0.0, 0.3, 0.62, 0.06, body);
    for step in 0..6 {
        let v = 0.14 + step as f32 * 0.07;
        p.line((-0.26, v), (0.26, v), 0.012, shade(body, -0.2));
    }
    p.rect(-0.3, 0.62, 0.3, 0.68, p.k.second);
    p.rect(-0.04, 0.68, 0.04, 0.88, metal);
    p.dome(0.0, 0.88, 0.14, 0.06, metal);
    p.line((0.3, 0.2), (0.44, 0.2), 0.02, metal);
    p.line((0.44, 0.2), (0.44, 0.08), 0.02, metal);
    p.circ(0.44, 0.05, 0.02, p.k.water);
}

pub fn low_g_swing(p: &mut Pen) {
    p.shadow(0.44);
    let metal = p.k.metal;
    p.curve((-0.44, 0.0), (0.0, 1.4), (0.44, 0.0), 0.03, panel(p));
    let swing = 0.08 * p.sway;
    p.line((-0.04, 0.72), (-0.06 + swing, 0.3), 0.008, metal);
    p.line((0.04, 0.72), (0.06 + swing, 0.3), 0.008, metal);
    p.rr(-0.1 + swing, 0.26, 0.1 + swing, 0.32, 0.02, p.k.accent);
}

pub fn mist_fountain(p: &mut Pen) {
    p.shadow(0.4);
    let body = panel(p);
    p.rect(-0.4, 0.0, 0.4, 0.14, shade(body, -0.1));
    p.ell(0.0, 0.14, 0.4, 0.05, shade(p.k.water, 0.2));
    p.rr(-0.06, 0.14, 0.06, 0.5, 0.03, p.k.metal);
    p.rect(-0.1, 0.5, 0.1, 0.54, p.k.accent);
    let mist = shade(p.k.water, 0.6).opacity(0.5);
    for (u, v, r) in [
        (0.0, 0.64, 0.1),
        (-0.12, 0.74, 0.08),
        (0.12, 0.76, 0.09),
        (0.0, 0.86, 0.07),
    ] {
        p.circ(u, v, r, mist);
    }
}

pub fn trail_marker(p: &mut Pen) {
    p.shadow(0.2);
    p.rect(-0.03, 0.0, 0.03, 0.92, p.k.accent);
    for v in [0.2, 0.5, 0.8] {
        p.rect(-0.035, v, 0.035, v + 0.08, panel(p));
    }
    p.poly(&[(0.03, 0.9), (0.3, 0.84), (0.03, 0.76)], p.k.second);
    p.light(0.0, 0.95, 0.02, p.k.accent);
}

pub fn supply_cache(p: &mut Pen) {
    p.shadow(0.4);
    let body = panel(p);
    p.rr(-0.34, 0.0, 0.34, 0.6, 0.05, body);
    p.rect(-0.34, 0.26, 0.34, 0.3, p.k.metal);
    p.rect(-0.1, 0.5, 0.1, 0.56, p.k.accent);
    p.rr(-0.38, 0.6, 0.38, 0.7, 0.03, p.k.second);
    p.line((-0.34, 0.1), (0.34, 0.1), 0.006, shade(body, -0.2));
    p.rect(0.2, 0.14, 0.28, 0.22, p.k.accent);
}

pub fn planter_racks(p: &mut Pen) {
    p.shadow(0.44);
    let metal = p.k.metal;
    for u in [-0.4, 0.4] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.94, metal);
    }
    for v in [0.06, 0.38, 0.7] {
        p.rect(-0.42, v, 0.42, v + 0.1, panel(p));
        for step in 0..6 {
            let u = -0.34 + step as f32 * 0.136;
            p.circ(u, v + 0.14, 0.04, shade(p.k.leaf, (step % 2) as f32 * 0.12));
        }
    }
}

/// The colony's founders in white stone on a hexagonal plinth: two in
/// helmets, visors up, faces to the hills.
pub fn founders_statue(p: &mut Pen) {
    p.shadow(0.4);
    let stone = shade(panel(p), -0.05);
    let dark = shade(stone, -0.45);
    p.rect(-0.34, 0.0, 0.34, 0.24, shade(p.k.stone, -0.1));
    p.rect(-0.38, 0.22, 0.38, 0.26, shade(p.k.stone, 0.05));
    for (u, tall) in [(-0.14, 0.0), (0.14, 0.06)] {
        p.rr(u - 0.1, 0.26, u + 0.1, 0.66 + tall, 0.04, stone);
        p.rect(u - 0.06, 0.4, u + 0.06, 0.52, shade(stone, -0.1));
        p.circ(u, 0.76 + tall, 0.1, stone);
        p.circ(u, 0.75 + tall, 0.07, shade(stone, 0.1));
        p.face(u, 0.75 + tall, 0.07, dark);
    }
    p.line((0.26, 0.5), (0.36, 0.9), 0.016, p.k.metal);
    p.poly(&[(0.36, 0.9), (0.5, 0.86), (0.36, 0.8)], p.k.accent);
}

pub fn message_post(p: &mut Pen) {
    p.shadow(0.2);
    p.rect(-0.03, 0.0, 0.03, 0.6, p.k.metal);
    p.rr(-0.2, 0.6, 0.2, 0.9, 0.04, panel(p));
    p.rect(-0.2, 0.84, 0.2, 0.9, p.k.accent);
    p.rect(-0.12, 0.66, 0.12, 0.78, p.k.dark);
    p.light(0.14, 0.72, 0.015, p.k.leaf);
    p.line((0.12, 0.9), (0.12, 1.0), 0.008, p.k.metal);
}

pub fn cargo_depot(p: &mut Pen) {
    p.shadow(0.48);
    let body = panel(p);
    p.rect(-0.48, 0.0, 0.1, 0.46, p.k.accent);
    for step in 0..6 {
        let u = -0.44 + step as f32 * 0.09;
        p.line((u, 0.02), (u, 0.44), 0.01, shade(p.k.accent, -0.2));
    }
    p.rect(0.14, 0.0, 0.48, 0.3, p.k.second);
    p.rect(0.2, 0.3, 0.42, 0.52, body);
    p.line((0.2, 0.41), (0.42, 0.41), 0.008, shade(body, -0.25));
    p.rect(-0.48, 0.46, 0.1, 0.5, shade(p.k.accent, -0.2));
}

pub fn cargo_crate(p: &mut Pen) {
    p.shadow(0.3);
    let body = panel(p);
    p.rr(-0.28, 0.0, 0.28, 0.66, 0.03, body);
    p.rect(-0.28, 0.28, 0.28, 0.34, p.k.accent);
    p.line((-0.28, 0.0), (0.28, 0.66), 0.01, shade(body, -0.2));
    p.rect(-0.1, 0.5, 0.1, 0.58, p.k.dark);
}

pub fn sandbag_wall(p: &mut Pen) {
    let bag = hex(0xc9a47a);
    for row in 0..4 {
        let v = row as f32 * 0.2;
        let off = if row % 2 == 0 { 0.0 } else { 0.08 };
        for step in 0..6 {
            let u = -0.44 + off + step as f32 * 0.16;
            if u > 0.46 {
                continue;
            }
            p.rr(
                u - 0.08,
                v,
                u + 0.08,
                v + 0.2,
                0.05,
                shade(bag, ((row + step) % 3) as f32 * 0.05 - 0.05),
            );
        }
    }
    p.line((-0.3, 0.84), (0.3, 0.84), 0.0, bag);
}

pub fn water_still(p: &mut Pen) {
    p.shadow(0.4);
    let (body, metal) = (panel(p), p.k.metal);
    p.rr(-0.4, 0.0, -0.02, 0.62, 0.08, body);
    p.dome(-0.21, 0.62, 0.19, 0.12, body);
    p.rect(-0.4, 0.3, -0.02, 0.34, p.k.second);
    p.line((-0.21, 0.74), (-0.21, 0.86), 0.02, metal);
    p.curve((-0.21, 0.86), (0.2, 0.9), (0.24, 0.6), 0.02, metal);
    for step in 0..4 {
        let v = 0.58 - step as f32 * 0.09;
        p.ell(0.24, v, 0.06, 0.03, metal);
    }
    p.rect(0.14, 0.0, 0.34, 0.2, shade(p.k.water, 0.2));
    p.circ(0.24, 0.26, 0.015, p.k.water);
}

pub fn mess_hall_dome(p: &mut Pen) {
    p.shadow(0.48);
    capsule(p, -0.5, 0.1, 0.06, 0.42, 2);
    pressure_dome(p, 0.24, 0.26, 0.72);
    p.rr(0.14, 0.2, 0.34, 0.3, 0.04, p.glass);
    p.rr(0.14, 0.34, 0.34, 0.44, 0.04, p.glass);
}

pub fn rover_shed(p: &mut Pen) {
    p.shadow(0.48);
    let body = panel(p);
    p.dome(0.0, 0.0, 0.48, 0.92, body);
    for t in [-0.66_f32, -0.33, 0.33, 0.66] {
        p.line(
            (t * 0.48, 0.0),
            (t * 0.36, 0.7 * (1.0 - t * t).sqrt() + 0.1),
            0.006,
            shade(body, -0.15),
        );
    }
    p.rr(-0.26, 0.0, 0.26, 0.5, 0.06, shade(p.k.metal, -0.1));
    for v in [0.1, 0.2, 0.3, 0.4] {
        p.line((-0.24, v), (0.24, v), 0.008, shade(p.k.metal, 0.15));
    }
    p.rect(-0.26, 0.5, 0.26, 0.54, p.k.accent);
}

pub fn relay_hut(p: &mut Pen) {
    p.shadow(0.4);
    capsule(p, -0.34, 0.22, 0.06, 0.36, 1);
    let metal = p.k.metal;
    p.rect(0.3, 0.0, 0.33, 0.96, metal);
    for step in 0..5 {
        let v = 0.12 + step as f32 * 0.17;
        p.line((0.26, v), (0.37, v + 0.08), 0.006, metal);
    }
    p.ell(0.2, 0.84, 0.1, 0.04, panel(p));
    p.light(0.315, 0.98, 0.018, p.k.accent);
}

pub fn seed_vault(p: &mut Pen) {
    let rock = shade(p.k.ground, -0.15);
    p.poly(
        &[
            (-0.5, 0.0),
            (0.5, 0.0),
            (0.44, 0.44),
            (0.16, 0.8),
            (-0.2, 0.72),
            (-0.46, 0.4),
        ],
        rock,
    );
    p.poly(
        &[(-0.2, 0.72), (0.16, 0.8), (0.1, 0.6), (-0.1, 0.56)],
        shade(rock, 0.12),
    );
    p.poly(
        &[(-0.22, 0.0), (0.22, 0.0), (0.2, 0.46), (-0.2, 0.46)],
        panel(p),
    );
    p.poly(
        &[(-0.24, 0.46), (0.24, 0.46), (0.2, 0.52), (-0.2, 0.52)],
        p.k.accent,
    );
    hatch(p, 0.0, 0.0, 0.11, 0.38);
}

pub fn storm_shelter(p: &mut Pen) {
    let berm = shade(p.k.ground, -0.08);
    p.dome(0.0, 0.0, 0.5, 0.86, berm);
    for (u, v) in [(-0.3, 0.3), (0.2, 0.6), (0.34, 0.24), (-0.1, 0.7)] {
        p.ell(u, v, 0.05, 0.04, shade(berm, -0.2));
    }
    p.rr(-0.16, 0.0, 0.16, 0.46, 0.05, panel(p));
    hatch(p, 0.0, 0.0, 0.08, 0.36);
    p.rect(-0.16, 0.46, 0.16, 0.5, p.k.accent);
}

pub fn solar_field(p: &mut Pen) {
    let (cell, metal) = (hex(0x2a3a5a), p.k.metal);
    for (u, v) in [(-0.3, 0.0), (0.1, 0.0), (-0.1, 0.44)] {
        p.line((u + 0.1, v), (u + 0.1, v + 0.2), 0.02, metal);
        p.poly(
            &[
                (u - 0.14, v + 0.2),
                (u + 0.3, v + 0.2),
                (u + 0.36, v + 0.46),
                (u - 0.08, v + 0.46),
            ],
            cell,
        );
        for t in [0.33, 0.66] {
            p.line(
                (u - 0.14 + 0.44 * t, v + 0.2),
                (u - 0.08 + 0.44 * t, v + 0.46),
                0.006,
                shade(cell, 0.35),
            );
        }
        p.line(
            (u - 0.11, v + 0.33),
            (u + 0.33, v + 0.33),
            0.006,
            shade(cell, 0.35),
        );
    }
}

pub fn greenhouse_annexe(p: &mut Pen) {
    p.shadow(0.48);
    let glass = shade(p.k.water, 0.45).opacity(0.75);
    p.rect(-0.48, 0.0, 0.48, 0.1, shade(panel(p), -0.15));
    for step in 0..6 {
        let u = -0.4 + step as f32 * 0.16;
        p.circ(u, 0.2, 0.07, p.k.leaf);
    }
    p.rr(-0.48, 0.1, 0.48, 0.9, 0.4, glass);
    let frame = p.k.pale;
    for step in 0..7 {
        let u = -0.36 + step as f32 * 0.12;
        p.line((u, 0.1), (u, 0.84), 0.006, frame);
    }
    p.rr(-0.5, 0.0, -0.36, 0.7, 0.05, panel(p));
}

pub fn track_beacons(p: &mut Pen) {
    for (u, tall) in [(-0.36, 0.5), (0.0, 0.7), (0.36, 0.9)] {
        p.rect(u - 0.03, 0.0, u + 0.03, tall, panel(p));
        p.rect(u - 0.035, tall * 0.5, u + 0.035, tall * 0.6, p.k.accent);
        p.light(u, tall + 0.03, 0.025, p.k.accent);
    }
    p.line((-0.5, 0.02), (0.5, 0.02), 0.01, shade(p.k.ground, -0.3));
}

pub fn clinic_bay(p: &mut Pen) {
    p.shadow(0.46);
    capsule(p, -0.46, 0.46, 0.1, 0.8, 1);
    p.rect(0.08, 0.36, 0.2, 0.62, p.k.pale);
    p.rect(0.02, 0.44, 0.26, 0.54, hex(0x3f9a5a));
    p.rect(0.1, 0.38, 0.18, 0.6, hex(0x3f9a5a));
}

pub fn machine_shop(p: &mut Pen) {
    p.shadow(0.46);
    let body = panel(p);
    p.rr(-0.46, 0.0, 0.2, 0.56, 0.04, body);
    p.rect(-0.46, 0.4, 0.2, 0.46, p.roof);
    p.rect(-0.36, 0.0, -0.06, 0.34, shade(p.k.metal, -0.1));
    let metal = p.k.metal;
    p.rect(0.3, 0.0, 0.34, 0.9, metal);
    p.line((0.32, 0.9), (-0.1, 0.84), 0.02, p.k.accent);
    p.line((-0.06, 0.84), (-0.06, 0.64), 0.006, metal);
    p.rect(-0.1, 0.58, -0.02, 0.64, metal);
}

pub fn observatory_dome(p: &mut Pen) {
    p.shadow(0.4);
    let body = panel(p);
    p.rect(-0.34, 0.0, 0.34, 0.42, shade(body, -0.06));
    p.rect(-0.34, 0.38, 0.34, 0.42, p.k.accent);
    p.dome(0.0, 0.42, 0.36, 0.44, body);
    p.poly(
        &[(-0.06, 0.42), (0.06, 0.42), (0.06, 0.85), (-0.06, 0.85)],
        p.k.dark,
    );
    p.line((0.0, 0.6), (0.2, 0.98), 0.05, p.k.metal);
    p.porthole(-0.2, 0.22, 0.04);
    hatch(p, 0.18, 0.0, 0.05, 0.3);
}

pub fn water_tank(p: &mut Pen) {
    p.shadow(0.36);
    let metal = p.k.metal;
    for u in [-0.26, 0.26] {
        p.line((u, 0.0), (u * 0.7, 0.46), 0.02, metal);
    }
    p.line((-0.2, 0.2), (0.2, 0.2), 0.012, metal);
    p.circ(0.0, 0.66, 0.34, panel(p));
    p.circ(0.1, 0.72, 0.18, shade(panel(p), 0.08));
    p.rect(-0.34, 0.64, 0.34, 0.68, p.k.second);
    p.line((0.3, 0.46), (0.44, 0.0), 0.016, metal);
}

pub fn dust_lock(p: &mut Pen) {
    p.shadow(0.36);
    let body = panel(p);
    p.rr(-0.34, 0.0, 0.34, 0.76, 0.14, body);
    p.rect(-0.34, 0.6, 0.34, 0.66, p.k.accent);
    hatch(p, 0.0, 0.04, 0.16, 0.5);
    for side in [-1.0_f32, 1.0] {
        p.light(side * 0.26, 0.5, 0.02, p.k.leaf);
    }
    p.rect(-0.4, 0.0, 0.4, 0.04, shade(p.k.metal, -0.1));
}

pub fn school_pod(p: &mut Pen) {
    p.shadow(0.4);
    let body = panel(p);
    p.circ(0.0, 0.42, 0.4, body);
    p.rect(-0.3, 0.0, 0.3, 0.1, shade(body, -0.15));
    for (u, v) in [(-0.18, 0.46), (0.18, 0.46), (0.0, 0.64)] {
        p.porthole(u, v, 0.06);
    }
    p.circ(0.0, 0.9, 0.06, p.k.second);
    p.ell(0.0, 0.9, 0.12, 0.02, p.k.accent);
    p.rect(-0.08, 0.0, 0.08, 0.24, p.k.metal);
}

pub fn cable_pylons(p: &mut Pen) {
    let metal = p.k.metal;
    for (u, tall) in [(-0.34, 0.82), (0.34, 0.66)] {
        p.line((u - 0.08, 0.0), (u, tall), 0.014, metal);
        p.line((u + 0.08, 0.0), (u, tall), 0.014, metal);
        p.line((u - 0.06, tall * 0.3), (u + 0.06, tall * 0.3), 0.008, metal);
        p.line((u - 0.1, tall), (u + 0.1, tall), 0.014, p.k.accent);
    }
    p.curve((-0.5, 0.7), (-0.42, 0.78), (-0.34, 0.82), 0.008, p.k.dark);
    p.curve((-0.34, 0.82), (0.0, 0.55), (0.34, 0.66), 0.008, p.k.dark);
    p.curve((0.34, 0.66), (0.44, 0.6), (0.5, 0.56), 0.008, p.k.dark);
}

pub fn rec_dome(p: &mut Pen) {
    p.shadow(0.46);
    pressure_dome(p, 0.0, 0.44, 0.78);
    p.rr(-0.26, 0.18, 0.26, 0.44, 0.1, p.glass);
    for (u, ink) in [(-0.1, p.k.accent), (0.1, p.k.second)] {
        p.circ(u, 0.3, 0.05, ink);
    }
    p.line((0.3, 0.84), (0.44, 1.0), 0.01, p.k.metal);
    p.poly(&[(0.44, 1.0), (0.5, 0.94), (0.44, 0.9)], p.k.brass);
}

pub fn radio_tower(p: &mut Pen) {
    p.shadow(0.3);
    let metal = p.k.metal;
    p.line((-0.2, 0.0), (-0.04, 0.94), 0.016, metal);
    p.line((0.2, 0.0), (0.04, 0.94), 0.016, metal);
    for step in 0..9 {
        let v = step as f32 * 0.1;
        let s = 0.2 - v * 0.17;
        p.line((-s, v), (s - 0.017, v + 0.1), 0.006, metal);
        p.line((s, v), (-s + 0.017, v + 0.1), 0.006, metal);
    }
    for (v, side) in [(0.7, 1.0), (0.5, -1.0)] {
        p.ell(side * 0.14, v, 0.08, 0.035, panel(p));
        p.rect(
            side * 0.06 - 0.01,
            v - 0.01,
            side * 0.06 + 0.01,
            v + 0.01,
            metal,
        );
    }
    p.light(0.0, 0.97, 0.02, p.k.accent);
}

pub fn landing_stone(p: &mut Pen) {
    p.shadow(0.36);
    let rock = p.k.stone;
    p.poly(
        &[
            (-0.3, 0.0),
            (0.3, 0.0),
            (0.26, 0.5),
            (0.08, 0.62),
            (-0.24, 0.54),
        ],
        rock,
    );
    p.rr(-0.16, 0.2, 0.16, 0.42, 0.02, p.k.brass);
    p.line((-0.1, 0.34), (0.1, 0.34), 0.012, shade(p.k.brass, -0.4));
    p.line((-0.08, 0.28), (0.06, 0.28), 0.01, shade(p.k.brass, -0.4));
    p.line((0.24, 0.5), (0.3, 0.96), 0.014, p.k.metal);
    p.poly(&[(0.3, 0.96), (0.5, 0.9), (0.3, 0.8)], p.k.accent);
}

pub fn algae_farm(p: &mut Pen) {
    p.shadow(0.46);
    let metal = p.k.metal;
    for u in [-0.42, 0.42] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.9, metal);
    }
    for step in 0..6 {
        let v = 0.1 + step as f32 * 0.13;
        let ink = if step % 2 == 0 {
            hex(0x5a9a4a)
        } else {
            hex(0x3f8f6a)
        };
        p.rr(-0.42, v, 0.42, v + 0.09, 0.03, ink.opacity(0.85));
        p.ell(
            -0.2 + (step as f32 * 0.13) % 0.4,
            v + 0.05,
            0.03,
            0.02,
            p.k.pale.opacity(0.6),
        );
    }
}

pub fn bunkhouse(p: &mut Pen) {
    p.shadow(0.46);
    capsule(p, -0.46, 0.46, 0.06, 0.46, 3);
    let body = panel(p);
    let r = 0.2 * p.h / p.w;
    p.rr(-0.36, 0.48, 0.36, 0.88, r, body);
    p.rect(-0.36, 0.72, 0.36, 0.78, p.k.second);
    for u in [-0.18, 0.0, 0.18] {
        p.porthole(u, 0.62, 0.04);
    }
    p.line((0.42, 0.46), (0.42, 0.8), 0.012, p.k.metal);
    for v in [0.54, 0.62, 0.7] {
        p.line((0.38, v), (0.46, v), 0.01, p.k.metal);
    }
}

pub fn rover_lift(p: &mut Pen) {
    p.shadow(0.48);
    let metal = p.k.metal;
    for u in [-0.44, 0.44] {
        p.rect(u - 0.03, 0.0, u + 0.03, 0.92, metal);
    }
    p.rect(-0.47, 0.88, 0.47, 0.94, p.k.accent);
    p.rect(-0.4, 0.36, 0.4, 0.4, metal);
    p.rr(-0.3, 0.44, 0.24, 0.62, 0.04, panel(p));
    p.rr(0.0, 0.62, 0.22, 0.74, 0.03, panel(p));
    for u in [-0.2, 0.14] {
        p.circ(u, 0.44, 0.07, p.k.dark);
    }
    for u in [-0.3, 0.3] {
        p.line((u, 0.4), (u, 0.88), 0.006, p.k.dark);
    }
}

pub fn windbreak_panels(p: &mut Pen) {
    let body = shade(p.k.ground, 0.15);
    for (index, u) in [-0.34, 0.0, 0.34].iter().enumerate() {
        let lean = if index % 2 == 0 { 0.06 } else { -0.06 };
        p.poly(
            &[
                (u - 0.16, 0.0),
                (u + 0.16, 0.0),
                (u + 0.14 + lean, 0.84),
                (u - 0.14 + lean, 0.84),
            ],
            body,
        );
        p.line(
            (u - 0.14 + lean, 0.6),
            (u + 0.14 + lean, 0.6),
            0.02,
            p.k.accent,
        );
        p.line((*u, 0.0), (u + lean, 0.84), 0.006, shade(body, -0.2));
    }
}

pub fn trading_post(p: &mut Pen) {
    p.shadow(0.48);
    let body = panel(p);
    p.rr(-0.44, 0.0, 0.3, 0.6, 0.03, body);
    p.poly(
        &[(-0.48, 0.66), (0.34, 0.66), (0.44, 0.52), (-0.38, 0.52)],
        p.k.accent,
    );
    p.rect(-0.34, 0.2, 0.2, 0.26, shade(p.k.metal, -0.1));
    for (u, ink) in [(-0.26, p.k.second), (-0.1, p.k.brass), (0.06, p.k.leaf)] {
        p.rect(u - 0.06, 0.26, u + 0.06, 0.36, ink);
    }
    for (u, v) in [(0.38, 0.0), (0.46, 0.0), (0.42, 0.12)] {
        p.rect(u - 0.05, v, u + 0.05, v + 0.12, shade(body, -0.1));
    }
    p.line((0.3, 0.6), (0.3, 0.9), 0.01, p.k.metal);
    p.poly(&[(0.3, 0.9), (0.44, 0.86), (0.3, 0.8)], p.k.second);
}

pub fn quiet_pod(p: &mut Pen) {
    p.shadow(0.36);
    let body = panel(p);
    p.ell(0.0, 0.46, 0.34, 0.42, body);
    p.ell(0.08, 0.52, 0.18, 0.26, shade(body, 0.06));
    p.porthole(0.0, 0.5, 0.1);
    p.circ(0.03, 0.47, 0.05, p.k.leaf);
    p.rect(-0.2, 0.0, 0.2, 0.08, p.k.metal);
    p.ell(0.0, 0.88, 0.1, 0.03, p.k.second);
}

pub fn library_module(p: &mut Pen) {
    p.shadow(0.46);
    capsule(p, -0.46, 0.46, 0.1, 0.8, 0);
    p.rr(-0.32, 0.3, 0.32, 0.56, 0.04, p.glass);
    let inks = [p.k.accent, p.k.second, p.k.brass, p.k.leaf];
    for step in 0..10 {
        let u = -0.3 + step as f32 * 0.06;
        p.rect(
            u,
            0.32,
            u + 0.045,
            0.44 + 0.03 * (step % 3) as f32,
            inks[step % 4],
        );
    }
    p.line((-0.32, 0.46), (0.32, 0.46), 0.008, p.k.metal);
}

pub fn mine_headframe(p: &mut Pen) {
    p.shadow(0.4);
    let metal = p.k.metal;
    p.line((-0.3, 0.0), (-0.06, 0.86), 0.022, metal);
    p.line((0.3, 0.0), (0.06, 0.86), 0.022, metal);
    p.line((0.4, 0.0), (0.06, 0.8), 0.018, metal);
    for v in [0.28, 0.56] {
        let s = 0.3 - v * 0.28;
        p.line((-s, v), (s, v), 0.012, metal);
    }
    p.circ(0.0, 0.86, 0.12, p.k.accent);
    p.circ(0.0, 0.86, 0.08, shade(p.k.ground, 0.2));
    p.circ(0.0, 0.86, 0.02, p.k.dark);
    p.line((0.12, 0.86), (0.12, 0.0), 0.006, p.k.dark);
    p.rr(-0.2, 0.0, 0.2, 0.14, 0.02, panel(p));
}

pub fn flower_dome(p: &mut Pen) {
    p.shadow(0.48);
    let glass = shade(p.k.water, 0.45).opacity(0.7);
    p.rect(-0.48, 0.0, 0.48, 0.1, shade(panel(p), -0.15));
    p.flowers(-0.4, 0.4, 0.1, 12, 0.04);
    p.tree(0.0, 0.4, 0.18);
    p.dome(0.0, 0.1, 0.48, 0.88, glass);
    let frame = p.k.pale;
    for t in [-0.66_f32, -0.33, 0.0, 0.33, 0.66] {
        p.curve(
            (t * 0.48, 0.1),
            (t * 0.54, 0.64),
            (t * 0.12, 0.97),
            0.006,
            frame,
        );
    }
    for v in [0.4, 0.72] {
        let s = 0.48 * (1.0_f32 - ((v - 0.1_f32) / 0.88).powi(2)).sqrt();
        p.line((-s, v), (s, v), 0.006, frame);
    }
}

pub fn welcome_arch(p: &mut Pen) {
    p.shadow(0.44);
    let body = panel(p);
    p.curve((-0.42, 0.0), (0.0, 1.32), (0.42, 0.0), 0.08, body);
    p.curve((-0.42, 0.0), (0.0, 1.32), (0.42, 0.0), 0.02, p.k.accent);
    for (u, v) in [
        (-0.34, 0.4),
        (-0.2, 0.62),
        (0.0, 0.68),
        (0.2, 0.62),
        (0.34, 0.4),
    ] {
        p.light(u, v, 0.018, p.k.brass);
    }
    for u in [-0.42, 0.42] {
        p.rect(u - 0.06, 0.0, u + 0.06, 0.1, p.k.metal);
    }
}

pub fn hydroponics_dome(p: &mut Pen) {
    p.shadow(0.46);
    pressure_dome(p, 0.0, 0.46, 0.82);
    p.rr(
        -0.34,
        0.12,
        0.34,
        0.5,
        0.1,
        shade(p.k.water, 0.3).opacity(0.85),
    );
    for v in [0.2, 0.34] {
        p.line((-0.3, v), (0.3, v), 0.01, p.k.metal);
        for step in 0..6 {
            let u = -0.26 + step as f32 * 0.1;
            p.circ(u, v + 0.04, 0.03, p.k.leaf);
        }
    }
}

pub fn infirmary_dome(p: &mut Pen) {
    p.shadow(0.46);
    pressure_dome(p, -0.08, 0.38, 0.74);
    p.circ(-0.08, 0.44, 0.12, p.k.pale);
    p.rect(-0.12, 0.34, -0.04, 0.54, hex(0x3f9a5a));
    p.rect(-0.18, 0.4, 0.02, 0.48, hex(0x3f9a5a));
    capsule(p, 0.2, 0.5, 0.06, 0.36, 1);
}

pub fn mess_module(p: &mut Pen) {
    p.shadow(0.48);
    capsule(p, -0.48, 0.48, 0.1, 0.72, 0);
    p.rr(-0.36, 0.28, 0.36, 0.5, 0.06, p.glass);
    for u in [-0.24, 0.0, 0.24] {
        p.rect(u - 0.06, 0.3, u + 0.06, 0.34, p.k.dark);
        p.circ(u, 0.4, 0.03, p.k.dark);
    }
    p.rect(-0.04, 0.72, 0.04, 0.86, p.k.metal);
    p.circ(0.0, 0.9, 0.04, p.k.pale.opacity(0.6));
}

pub fn workshop_dome(p: &mut Pen) {
    p.shadow(0.48);
    pressure_dome(p, 0.0, 0.46, 0.7);
    p.rr(-0.24, 0.0, 0.24, 0.4, 0.04, shade(p.k.metal, -0.1));
    for v in [0.1, 0.2, 0.3] {
        p.line((-0.22, v), (0.22, v), 0.008, shade(p.k.metal, 0.2));
    }
    p.line((0.3, 0.6), (0.44, 0.96), 0.02, p.k.metal);
    p.line((0.44, 0.96), (0.5, 0.86), 0.014, p.k.accent);
}

pub fn music_pod(p: &mut Pen) {
    p.shadow(0.4);
    let body = panel(p);
    p.ell(0.0, 0.44, 0.4, 0.4, body);
    p.rect(-0.3, 0.0, 0.3, 0.08, p.k.metal);
    p.porthole(-0.14, 0.5, 0.08);
    p.circ(0.2, 0.4, 0.1, p.k.dark);
    p.circ(0.2, 0.4, 0.05, p.k.accent);
    for (u, v) in [(0.24, 0.9), (0.36, 0.84)] {
        p.circ(u, v, 0.03, p.k.dark);
        p.line((u + 0.025, v), (u + 0.025, v + 0.12), 0.008, p.k.dark);
    }
}

pub fn schoolroom_module(p: &mut Pen) {
    p.shadow(0.46);
    capsule(p, -0.46, 0.46, 0.08, 0.7, 0);
    p.rr(-0.32, 0.26, 0.12, 0.5, 0.04, p.glass);
    p.circ(-0.1, 0.38, 0.06, p.k.accent);
    p.ell(-0.1, 0.38, 0.1, 0.02, p.k.second);
    hatch(p, 0.3, 0.08, 0.06, 0.42);
    p.line((0.0, 0.7), (0.0, 0.92), 0.01, p.k.metal);
    p.poly(&[(0.0, 0.92), (0.16, 0.86), (0.0, 0.8)], p.k.brass);
}

pub fn fern_planter(p: &mut Pen) {
    p.shadow(0.4);
    let body = panel(p);
    p.poly(
        &[(-0.36, 0.4), (0.36, 0.4), (0.26, 0.0), (-0.26, 0.0)],
        body,
    );
    p.rect(-0.38, 0.36, 0.38, 0.42, p.k.accent);
    for (index, angle) in [-1.1_f32, -0.6, -0.2, 0.2, 0.6, 1.1].iter().enumerate() {
        let (su, sv) = (angle.sin() * 0.4, angle.cos() * 0.5);
        let ink = shade(p.k.leaf, (index % 2) as f32 * 0.12);
        p.curve(
            (0.0, 0.42),
            (su * 0.4, 0.42 + sv * 0.9),
            (su, 0.42 + sv * 0.8),
            0.03,
            ink,
        );
    }
}

pub fn control_tower(p: &mut Pen) {
    p.shadow(0.3);
    let body = panel(p);
    p.poly(&[(-0.2, 0.0), (0.2, 0.0), (0.12, 0.7), (-0.12, 0.7)], body);
    p.rect(-0.2, 0.3, 0.2, 0.34, p.k.accent);
    p.poly(
        &[(-0.3, 0.7), (0.3, 0.7), (0.26, 0.86), (-0.26, 0.86)],
        shade(body, -0.05),
    );
    p.poly(
        &[(-0.26, 0.74), (0.26, 0.74), (0.23, 0.83), (-0.23, 0.83)],
        p.glass,
    );
    p.rect(-0.28, 0.86, 0.28, 0.9, p.k.metal);
    p.line((0.0, 0.9), (0.0, 1.0), 0.01, p.k.metal);
    p.light(0.0, 1.0, 0.02, p.k.accent);
    hatch(p, 0.0, 0.0, 0.06, 0.24);
}

pub fn rover_garage(p: &mut Pen) {
    p.shadow(0.48);
    let body = panel(p);
    p.rr(-0.48, 0.0, 0.48, 0.66, 0.08, body);
    p.rect(-0.48, 0.5, 0.48, 0.56, p.k.second);
    for u in [-0.24, 0.24] {
        p.rr(u - 0.18, 0.0, u + 0.18, 0.42, 0.03, shade(p.k.metal, -0.1));
        for v in [0.1, 0.2, 0.3] {
            p.line((u - 0.16, v), (u + 0.16, v), 0.008, shade(p.k.metal, 0.2));
        }
    }
}

pub fn radio_dish(p: &mut Pen) {
    p.shadow(0.36);
    let metal = p.k.metal;
    p.line((-0.2, 0.0), (0.0, 0.4), 0.02, metal);
    p.line((0.2, 0.0), (0.0, 0.4), 0.02, metal);
    p.rect(-0.03, 0.3, 0.03, 0.5, metal);
    p.poly(
        &[
            (-0.42, 0.86),
            (-0.3, 0.6),
            (0.0, 0.44),
            (0.3, 0.5),
            (0.46, 0.7),
            (0.2, 0.9),
            (-0.1, 0.96),
        ],
        panel(p),
    );
    p.poly(
        &[
            (-0.3, 0.8),
            (-0.2, 0.62),
            (0.0, 0.52),
            (0.2, 0.58),
            (0.3, 0.7),
            (0.1, 0.84),
        ],
        shade(panel(p), -0.08),
    );
    p.line((0.0, 0.7), (0.14, 0.98), 0.01, metal);
    p.circ(0.14, 0.98, 0.02, p.k.accent);
}

pub fn landing_lights(p: &mut Pen) {
    for (index, u) in [-0.4, -0.2, 0.0, 0.2, 0.4].iter().enumerate() {
        let tall = 0.2 + index as f32 * 0.1;
        p.rect(u - 0.025, 0.0, u + 0.025, tall, panel(p));
        p.light(
            *u,
            tall + 0.04,
            0.03,
            if index % 2 == 0 {
                p.k.accent
            } else {
                p.k.brass
            },
        );
    }
}

pub fn windsock(p: &mut Pen) {
    p.shadow(0.2);
    let metal = p.k.metal;
    p.rect(-0.02, 0.0, 0.02, 0.94, metal);
    p.circ(0.0, 0.94, 0.03, metal);
    let sway = 0.04 * p.sway;
    for step in 0..4 {
        let (u0, u1) = (0.02 + step as f32 * 0.11, 0.13 + step as f32 * 0.11);
        let (top0, top1) = (
            0.94 - step as f32 * 0.02 + sway,
            0.94 - (step + 1) as f32 * 0.02 + sway,
        );
        let (s0, s1) = (0.08 - step as f32 * 0.012, 0.08 - (step + 1) as f32 * 0.012);
        let ink = if step % 2 == 0 { p.k.accent } else { p.k.pale };
        p.poly(
            &[
                (u0, top0),
                (u1, top1),
                (u1, top1 - s1 * 2.0),
                (u0, top0 - s0 * 2.0),
            ],
            ink,
        );
    }
}

pub fn fuel_tanks(p: &mut Pen) {
    p.shadow(0.48);
    let body = panel(p);
    for (u, r) in [(-0.24, 0.22), (0.24, 0.2)] {
        p.rr(u - r, 0.06, u + r, 0.82, r, body);
        p.rect(u - r, 0.5, u + r, 0.56, p.k.accent);
        p.rect(u - r * 0.6, 0.0, u - r * 0.4, 0.1, p.k.metal);
        p.rect(u + r * 0.4, 0.0, u + r * 0.6, 0.1, p.k.metal);
    }
    p.line((-0.02, 0.3), (0.04, 0.3), 0.02, p.k.metal);
}

pub fn dust_shelter(p: &mut Pen) {
    p.shadow(0.48);
    let cloth = shade(p.k.accent, 0.15);
    for u in [-0.42, 0.42] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.6, p.k.metal);
    }
    p.poly(
        &[
            (-0.5, 0.6),
            (0.5, 0.6),
            (0.5, 0.52),
            (0.0, 0.8),
            (-0.5, 0.52),
        ],
        cloth,
    );
    p.poly(&[(-0.46, 0.58), (0.46, 0.58), (0.0, 0.8)], cloth);
    p.poly(
        &[(-0.42, 0.56), (-0.1, 0.56), (-0.42, 0.1)],
        shade(cloth, -0.2).opacity(0.8),
    );
    p.rect(-0.2, 0.2, 0.3, 0.26, panel(p));
}

pub fn solar_array(p: &mut Pen) {
    let (cell, metal) = (hex(0x2a3a5a), p.k.metal);
    p.rect(-0.03, 0.0, 0.03, 0.46, metal);
    p.poly(
        &[(-0.48, 0.4), (0.48, 0.52), (0.44, 0.98), (-0.44, 0.84)],
        cell,
    );
    for t in [0.25, 0.5, 0.75] {
        let u = -0.48 + 0.96 * t;
        p.line(
            (u, 0.4 + 0.12 * t),
            (u - 0.04 * 0.0 + 0.0, 0.84 + 0.14 * t),
            0.006,
            shade(cell, 0.35),
        );
    }
    p.line((-0.46, 0.62), (0.46, 0.75), 0.006, shade(cell, 0.35));
}

pub fn survey_station(p: &mut Pen) {
    p.shadow(0.36);
    let metal = p.k.metal;
    capsule(p, -0.44, 0.1, 0.04, 0.34, 1);
    p.rect(0.24, 0.0, 0.28, 0.9, metal);
    p.rect(0.18, 0.9, 0.34, 0.96, p.k.accent);
    for (u, v) in [(0.36, 0.7), (0.16, 0.56)] {
        p.line((0.26, v), (u, v), 0.008, metal);
        p.circ(u, v, 0.03, panel(p));
    }
    p.line((0.26, 0.96), (0.26, 1.0), 0.006, metal);
}

pub fn ice_drill(p: &mut Pen) {
    p.shadow(0.36);
    let metal = p.k.metal;
    p.line((-0.3, 0.0), (0.0, 0.94), 0.022, metal);
    p.line((0.3, 0.0), (0.0, 0.94), 0.022, metal);
    p.line((-0.2, 0.3), (0.2, 0.3), 0.014, metal);
    p.rect(-0.03, 0.0, 0.03, 0.9, p.k.accent);
    for step in 0..6 {
        let v = 0.06 + step as f32 * 0.06;
        p.line((-0.05, v), (0.05, v + 0.03), 0.01, shade(p.k.accent, -0.3));
    }
    p.rr(-0.12, 0.62, 0.12, 0.76, 0.02, panel(p));
    p.ell(0.3, 0.06, 0.1, 0.04, p.k.pale);
}

pub fn telescope_pad(p: &mut Pen) {
    p.shadow(0.46);
    p.rect(-0.46, 0.0, 0.46, 0.08, shade(panel(p), -0.1));
    for u in [-0.3, 0.3] {
        p.line((u, 0.08), (u + 0.1, 0.0), 0.01, p.k.metal);
    }
    p.rect(-0.06, 0.08, 0.06, 0.4, p.k.metal);
    p.poly(
        &[(-0.3, 0.42), (-0.2, 0.32), (0.38, 0.84), (0.28, 0.94)],
        panel(p),
    );
    p.rect(-0.3, 0.42, -0.2, 0.36, p.k.dark);
    p.ell(0.34, 0.9, 0.06, 0.05, p.glass);
    p.line((-0.2, 0.5), (0.26, 0.9), 0.012, p.k.accent);
}

pub fn crater_bench(p: &mut Pen) {
    p.shadow(0.46);
    let rock = p.k.stone;
    p.poly(
        &[(-0.48, 0.0), (0.48, 0.0), (0.4, 0.4), (-0.4, 0.4)],
        shade(rock, -0.08),
    );
    p.rect(-0.4, 0.38, 0.4, 0.46, shade(rock, 0.1));
    p.poly(
        &[(-0.4, 0.46), (-0.1, 0.46), (-0.2, 0.9), (-0.44, 0.82)],
        shade(rock, -0.02),
    );
    p.rr(-0.36, 0.46, 0.36, 0.52, 0.02, p.k.accent);
    for (u, v) in [(-0.1, 0.2), (0.2, 0.26)] {
        p.ell(u, v, 0.06, 0.03, shade(rock, -0.25));
    }
}

pub fn marker_cairn(p: &mut Pen) {
    p.shadow(0.36);
    let rock = p.k.stone;
    for (u, v, r) in [
        (-0.2, 0.08, 0.14),
        (0.14, 0.08, 0.15),
        (-0.04, 0.26, 0.14),
        (0.06, 0.42, 0.11),
        (-0.02, 0.56, 0.08),
    ] {
        p.ell(u, v, r, 0.09, shade(rock, (u * 2.0).abs() - 0.1));
    }
    p.line((0.0, 0.6), (0.02, 0.98), 0.012, p.k.metal);
    p.poly(&[(0.02, 0.98), (0.26, 0.92), (0.02, 0.84)], p.k.accent);
}

pub fn greenhouse_tent(p: &mut Pen) {
    p.shadow(0.48);
    let glass = shade(p.k.water, 0.5).opacity(0.7);
    for step in 0..6 {
        let u = -0.38 + step as f32 * 0.15;
        p.circ(u, 0.16, 0.07, p.k.leaf);
    }
    p.poly(
        &[(-0.48, 0.0), (-0.36, 0.72), (0.36, 0.72), (0.48, 0.0)],
        glass,
    );
    for u in [-0.36, 0.0, 0.36] {
        p.line((u * 1.33, 0.0), (u, 0.72), 0.008, p.k.pale);
    }
    p.line((-0.36, 0.72), (0.36, 0.72), 0.012, p.k.pale);
    p.rect(-0.08, 0.72, 0.08, 0.76, p.k.accent);
}

pub fn weather_mast(p: &mut Pen) {
    p.shadow(0.2);
    let metal = p.k.metal;
    p.rect(-0.02, 0.0, 0.02, 0.92, metal);
    p.line((-0.2, 0.84), (0.2, 0.84), 0.01, metal);
    for side in [-1.0_f32, 1.0] {
        p.dome(side * 0.2, 0.84, 0.04, 0.03, panel(p));
    }
    p.rr(-0.08, 0.5, 0.08, 0.62, 0.02, panel(p));
    for v in [0.52, 0.56, 0.6] {
        p.line((-0.08, v), (0.08, v), 0.006, shade(panel(p), -0.3));
    }
    p.poly(&[(0.02, 0.96), (0.2, 0.94), (0.02, 0.92)], p.k.accent);
    p.line((-0.3, 0.0), (0.0, 0.5), 0.005, metal);
}

pub fn lichen_garden(p: &mut Pen) {
    let rock = p.k.stone;
    for (u, r) in [(-0.3, 0.16), (0.02, 0.2), (0.32, 0.14)] {
        p.ell(u, 0.12, r, 0.16, shade(rock, -0.05));
        for step in 0..3 {
            let a = u - r * 0.5 + step as f32 * r * 0.5;
            p.circ(
                a,
                0.2,
                r * 0.25,
                [p.k.leaf, p.k.brass, shade(p.k.leaf, 0.3)][step],
            );
        }
    }
    p.rect(-0.48, 0.0, 0.48, 0.04, p.k.accent);
}

pub fn ridge_beacon(p: &mut Pen) {
    p.shadow(0.3);
    let rock = shade(p.k.ground, -0.12);
    p.poly(&[(-0.4, 0.0), (0.4, 0.0), (0.2, 0.3), (-0.24, 0.26)], rock);
    p.rect(-0.03, 0.26, 0.03, 0.86, panel(p));
    p.rect(-0.035, 0.5, 0.035, 0.6, p.k.accent);
    p.poly(
        &[(-0.08, 0.86), (0.08, 0.86), (0.05, 0.96), (-0.05, 0.96)],
        p.k.dark,
    );
    p.light(0.0, 0.91, 0.04, p.k.accent);
}

pub fn second_habitat(p: &mut Pen) {
    p.shadow(0.48);
    capsule(p, -0.48, 0.1, 0.06, 0.48, 2);
    capsule(p, -0.2, 0.48, 0.5, 0.9, 2);
    p.rect(-0.1, 0.46, 0.0, 0.52, p.k.metal);
}

pub fn survey_rig(p: &mut Pen) {
    p.shadow(0.36);
    let metal = p.k.metal;
    p.line((-0.3, 0.0), (0.0, 0.62), 0.016, metal);
    p.line((0.3, 0.0), (0.0, 0.62), 0.016, metal);
    p.line((0.06, 0.0), (0.0, 0.62), 0.016, metal);
    p.rr(-0.14, 0.62, 0.14, 0.76, 0.03, panel(p));
    p.rect(0.14, 0.66, 0.24, 0.72, p.k.dark);
    p.line((0.34, 0.0), (0.38, 0.96), 0.012, metal);
    p.poly(&[(0.38, 0.96), (0.52, 0.92), (0.38, 0.86)], p.k.accent);
}

pub fn dwarf_apple(p: &mut Pen) {
    p.shadow(0.36);
    let body = panel(p);
    p.poly(
        &[(-0.26, 0.26), (0.26, 0.26), (0.2, 0.0), (-0.2, 0.0)],
        body,
    );
    p.rect(-0.28, 0.24, 0.28, 0.3, p.k.accent);
    p.rect(-0.03, 0.3, 0.03, 0.56, p.k.wood);
    p.circ(-0.12, 0.66, 0.16, p.k.leaf);
    p.circ(0.12, 0.7, 0.16, shade(p.k.leaf, 0.08));
    p.circ(0.0, 0.8, 0.15, shade(p.k.leaf, 0.14));
    for (u, v) in [(-0.14, 0.62), (0.1, 0.66), (0.02, 0.84)] {
        p.circ(u, v, 0.03, hex(0xc0463a));
    }
}

pub fn algae_beds(p: &mut Pen) {
    let body = panel(p);
    for (u, ink) in [(-0.26, hex(0x5a9a4a)), (0.24, hex(0x3f8f6a))] {
        p.rr(u - 0.22, 0.0, u + 0.22, 0.5, 0.04, body);
        p.rect(u - 0.18, 0.3, u + 0.18, 0.46, ink);
        p.ell(u, 0.4, 0.08, 0.03, shade(ink, 0.3));
    }
}

pub fn red_moss(p: &mut Pen) {
    let rock = p.k.stone;
    p.ell(0.0, 0.2, 0.48, 0.24, shade(rock, -0.15));
    for step in 0..9 {
        let u = -0.4 + step as f32 * 0.1;
        let v = 0.3 + 0.1 * ((step * 5) % 3) as f32;
        p.circ(u, v, 0.06, shade(hex(0xa8453a), (step % 3) as f32 * 0.1));
    }
}

pub fn rest_canopy(p: &mut Pen) {
    p.shadow(0.48);
    let cloth = p.k.second;
    for u in [-0.4, 0.4] {
        p.line((u, 0.0), (u * 0.8, 0.7), 0.016, p.k.metal);
    }
    p.curve((-0.5, 0.66), (0.0, 1.0), (0.5, 0.66), 0.08, cloth);
    p.rr(-0.3, 0.2, 0.3, 0.3, 0.03, panel(p));
    for u in [-0.24, 0.24] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.2, p.k.metal);
    }
}

pub fn fairy_lights(p: &mut Pen) {
    for u in [-0.46, 0.46] {
        p.rect(u - 0.015, 0.0, u + 0.015, 0.9, p.k.metal);
    }
    p.curve((-0.46, 0.88), (0.0, 0.5), (0.46, 0.88), 0.006, p.k.dark);
    for step in 0..9 {
        let t = (step as f32 + 0.5) / 9.0;
        let u = -0.46 + 0.92 * t;
        let v = 0.88 - 0.38 * 4.0 * t * (1.0 - t) * 0.5;
        p.light(
            u,
            v - 0.03,
            0.014,
            [p.k.brass, p.k.accent, p.k.second][step % 3],
        );
    }
}

/// A seat made of a supply crate with a folded blanket on it.
pub fn crate_seat(p: &mut Pen) {
    p.shadow(0.3);
    let body = panel(p);
    p.rr(-0.26, 0.0, 0.26, 0.62, 0.03, body);
    p.rect(-0.26, 0.26, 0.26, 0.32, p.k.accent);
    p.line((-0.26, 0.0), (0.26, 0.62), 0.01, shade(body, -0.2));
    p.rr(-0.22, 0.62, 0.24, 0.74, 0.03, p.k.second);
}

/// An old rover seat set on a stand: a padded seat and back on a post.
pub fn rover_seat(p: &mut Pen) {
    p.shadow(0.3);
    let metal = p.k.metal;
    p.rect(-0.2, 0.0, 0.2, 0.05, metal);
    p.rect(-0.03, 0.05, 0.03, 0.4, metal);
    p.rr(-0.26, 0.4, 0.22, 0.52, 0.04, p.k.dark);
    p.poly(
        &[(-0.26, 0.5), (-0.14, 0.5), (-0.2, 0.96), (-0.32, 0.94)],
        p.k.dark,
    );
    p.rr(-0.3, 0.8, -0.18, 0.92, 0.03, shade(p.k.dark, 0.2));
    p.rect(-0.22, 0.44, 0.2, 0.47, p.k.accent);
}
