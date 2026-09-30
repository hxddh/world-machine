//! Drawings at home in any setting: a bandstand, a clock tower, a
//! telescope, benches and swings, fountains, trees and beds.

use super::Pen;
use crate::art::{hex, shade};

/// An open octagon on a plinth under a pointed roof: slim columns, a
/// railing, and music stands.
pub fn bandstand(p: &mut Pen) {
    p.shadow(0.46);
    let (stone, white) = (p.k.stone, p.k.pale);
    // The plinth, three steps up.
    p.rr(-0.46, 0.0, 0.46, 0.2, 0.02, shade(stone, -0.05));
    p.rect(-0.46, 0.18, 0.46, 0.22, shade(stone, -0.2));
    p.rect(-0.1, 0.0, 0.1, 0.07, shade(stone, -0.25));
    p.rect(-0.14, 0.07, 0.14, 0.13, shade(stone, -0.15));
    // Posts and railing.
    let roof = p.roof;
    for u in [-0.42, -0.21, 0.0, 0.21, 0.42] {
        p.rect(u - 0.012, 0.22, u + 0.012, 0.3, white);
    }
    p.rect(-0.43, 0.29, 0.43, 0.31, white);
    // Music stands on the floor.
    let dark = p.k.dark;
    for u in [-0.2, 0.05, 0.28] {
        p.line((u, 0.22), (u, 0.33), 0.008, dark);
        p.rect(u - 0.04, 0.33, u + 0.04, 0.37, dark);
    }
    // Bunting from post to post, and no roof: an open-air stage.
    let (accent, second) = (p.k.accent, p.k.second);
    for u in [-0.44, 0.44] {
        p.rect(u - 0.012, 0.22, u + 0.012, 0.62, shade(roof, -0.1));
        p.circ(u, 0.63, 0.02, p.k.brass);
    }
    p.pennants((-0.44, 0.6), (0.44, 0.6), 9, &[accent, white, second]);
}

/// The bandstand under its roof: an ogee dome of tin with a finial, on
/// slender iron columns.
pub fn roofed_bandstand(p: &mut Pen) {
    p.shadow(0.46);
    let (stone, white, roof) = (p.k.stone, p.k.pale, p.roof);
    p.rr(-0.44, 0.0, 0.44, 0.18, 0.02, shade(stone, -0.05));
    p.rect(-0.44, 0.16, 0.44, 0.2, shade(stone, -0.2));
    p.rect(-0.12, 0.0, 0.12, 0.08, shade(stone, -0.22));
    for u in [-0.38, -0.19, 0.0, 0.19, 0.38] {
        p.rect(u - 0.012, 0.2, u + 0.012, 0.58, white);
    }
    // Railing with little balusters.
    p.rect(-0.4, 0.29, 0.4, 0.31, white);
    for step in 0..16 {
        let u = -0.38 + step as f32 * 0.05;
        p.line((u, 0.2), (u, 0.29), 0.006, shade(white, -0.15));
    }
    // Eaves with a fringe of scallops.
    p.rect(-0.47, 0.57, 0.47, 0.62, shade(roof, -0.15));
    for step in 0..10 {
        let u = -0.44 + step as f32 * 0.098;
        p.dome(u, 0.57, 0.045, -0.03, shade(roof, -0.15));
    }
    // The roof, curving in then out to a point.
    p.poly(
        &[
            (-0.47, 0.62),
            (-0.3, 0.72),
            (-0.14, 0.82),
            (-0.03, 0.9),
            (0.0, 0.94),
            (0.03, 0.9),
            (0.14, 0.82),
            (0.3, 0.72),
            (0.47, 0.62),
        ],
        roof,
    );
    p.poly(
        &[(-0.2, 0.66), (0.0, 0.92), (-0.08, 0.66)],
        shade(roof, 0.18),
    );
    p.line((0.0, 0.93), (0.0, 1.0), 0.012, p.k.brass);
    p.circ(0.0, 0.97, 0.018, p.k.brass);
}

/// A tall stone tower with a great round clock face, a louvred belfry
/// and a pyramid roof.
pub fn clock_tower(p: &mut Pen) {
    let (wall, roof) = (p.k.stone, p.roof);
    p.shadow(0.4);
    p.rect(-0.34, 0.0, 0.34, 0.06, shade(wall, -0.2));
    p.rect(-0.3, 0.06, 0.3, 0.72, wall);
    // Quoins down the edges, and a string course.
    for step in 0..9 {
        let v = 0.08 + step as f32 * 0.07;
        let side = if step % 2 == 0 { 0.05 } else { 0.03 };
        p.rect(-0.3, v, -0.3 + side, v + 0.03, shade(wall, 0.12));
        p.rect(0.3 - side, v, 0.3, v + 0.03, shade(wall, 0.12));
    }
    p.rect(-0.33, 0.46, 0.33, 0.48, shade(wall, -0.15));
    p.arch(-0.08, 0.08, 0.06, 0.2, shade(p.trim, -0.1));
    p.window(-0.05, 0.28, 0.05, 0.38);
    // The clock.
    let face = p.k.pale;
    let dark = p.k.dark;
    p.circ(0.0, 0.6, 0.22, shade(wall, -0.22));
    p.circ(0.0, 0.6, 0.19, face);
    for hour in 0..12 {
        let angle = hour as f32 * std::f32::consts::PI / 6.0;
        let (su, sv) = (angle.sin() * 0.155, angle.cos() * 0.155 * p.w / p.h);
        p.circ(su, 0.6 + sv, 0.012, dark);
    }
    p.line((0.0, 0.6), (0.0, 0.6 + 0.13 * p.w / p.h), 0.018, dark);
    p.line((0.0, 0.6), (0.09, 0.6 - 0.03 * p.w / p.h), 0.022, dark);
    // The belfry, with louvres, and the roof.
    p.rect(-0.26, 0.72, 0.26, 0.84, shade(wall, 0.06));
    for u in [-0.12, 0.12] {
        p.arch(u - 0.07, u + 0.07, 0.74, 0.83, shade(dark, 0.2));
    }
    p.rect(-0.3, 0.84, 0.3, 0.86, shade(wall, -0.18));
    p.gable(-0.3, 0.3, 0.86, 0.99, roof);
    p.line((0.0, 0.98), (0.0, 1.0), 0.012, p.k.brass);
}

/// A brass telescope on a wooden tripod, tilted up at the sky, with a
/// stool beside it.
pub fn telescope(p: &mut Pen) {
    p.shadow(0.36);
    let (wood, brass, dark) = (p.k.wood, p.k.brass, p.k.dark);
    // The tripod.
    p.line((-0.02, 0.5), (-0.26, 0.0), 0.03, wood);
    p.line((0.02, 0.5), (0.22, 0.0), 0.03, wood);
    p.line((0.0, 0.5), (0.02, 0.0), 0.028, shade(wood, -0.15));
    p.circ(0.0, 0.5, 0.045, dark);
    // The tube, wider at the sky end, with bands.
    p.poly(
        &[(-0.34, 0.33), (-0.3, 0.27), (0.42, 0.82), (0.36, 0.92)],
        brass,
    );
    p.poly(
        &[(0.3, 0.76), (0.44, 0.8), (0.4, 0.97), (0.26, 0.88)],
        shade(brass, -0.12),
    );
    for t in [0.25, 0.55] {
        let (u, v) = (-0.32 + 0.7 * t, 0.3 + 0.58 * t);
        p.line(
            (u - 0.03, v + 0.04),
            (u + 0.03, v - 0.04),
            0.022,
            shade(brass, -0.3),
        );
    }
    p.ell(0.35, 0.9, 0.04, 0.05, shade(p.glass, 0.2));
    // The eyepiece and the stool.
    p.rect(-0.38, 0.24, -0.32, 0.3, dark);
    p.rect(-0.46, 0.14, -0.3, 0.17, wood);
    for u in [-0.44, -0.32] {
        p.line((u, 0.14), (u, 0.0), 0.02, shade(wood, -0.2));
    }
}

/// A stepped stone base, a tall shaft and a cross-head with a ball: where
/// a market once met.
pub fn market_cross(p: &mut Pen) {
    let stone = p.k.stone;
    p.shadow(0.4);
    p.rect(-0.4, 0.0, 0.4, 0.06, shade(stone, -0.2));
    p.rect(-0.3, 0.06, 0.3, 0.12, shade(stone, -0.1));
    p.rect(-0.2, 0.12, 0.2, 0.18, stone);
    p.rect(-0.1, 0.18, 0.1, 0.24, shade(stone, 0.08));
    p.rect(-0.045, 0.24, 0.045, 0.8, stone);
    p.rect(-0.07, 0.4, 0.07, 0.43, shade(stone, -0.15));
    p.rect(-0.18, 0.72, 0.18, 0.77, stone);
    p.rect(-0.045, 0.77, 0.045, 0.9, stone);
    p.circ(0.0, 0.94, 0.05, shade(stone, 0.12));
}

/// A dial on a pedestal, its gnomon casting a line.
pub fn sundial(p: &mut Pen) {
    let stone = p.k.stone;
    p.shadow(0.3);
    p.rect(-0.2, 0.0, 0.2, 0.07, shade(stone, -0.2));
    p.poly(
        &[(-0.1, 0.07), (0.1, 0.07), (0.06, 0.55), (-0.06, 0.55)],
        stone,
    );
    p.rect(-0.12, 0.3, 0.12, 0.34, shade(stone, -0.1));
    p.ell(0.0, 0.6, 0.3, 0.06, shade(stone, 0.1));
    p.ell(0.0, 0.62, 0.26, 0.045, shade(p.k.brass, 0.2));
    p.poly(&[(-0.02, 0.63), (0.18, 0.63), (-0.02, 0.8)], p.k.brass);
}

/// A slatted wooden bench with iron ends.
pub fn park_bench(p: &mut Pen) {
    p.shadow(0.45);
    let (wood, metal) = (p.k.wood, p.k.metal);
    for u in [-0.36, 0.36] {
        p.line((u, 0.0), (u, 0.48), 0.035, metal);
        p.line((u - 0.06, 0.0), (u + 0.02, 0.45), 0.03, metal);
    }
    p.rect(-0.46, 0.44, 0.46, 0.5, wood);
    p.rect(-0.46, 0.52, 0.46, 0.57, shade(wood, 0.08));
    for v in [0.68, 0.8, 0.92] {
        p.rect(-0.44, v, 0.44, v + 0.07, shade(wood, 0.04));
    }
    p.rect(-0.37, 0.57, -0.34, 0.98, metal);
    p.rect(0.34, 0.57, 0.37, 0.98, metal);
}

/// One picnic table: a top and two benches on A-frame legs.
pub fn picnic_table(p: &mut Pen) {
    p.shadow(0.45);
    let wood = p.k.wood;
    p.rect(-0.4, 0.8, 0.4, 0.9, wood);
    p.rect(-0.46, 0.44, 0.46, 0.52, shade(wood, -0.08));
    for side in [-1.0_f32, 1.0] {
        let u = side * 0.26;
        p.line((u - 0.12, 0.0), (u + 0.08, 0.82), 0.04, shade(wood, -0.2));
        p.line((u + 0.12, 0.0), (u - 0.08, 0.82), 0.04, shade(wood, -0.2));
    }
}

/// Two picnic tables side by side, one smaller behind.
pub fn picnic_tables(p: &mut Pen) {
    p.shadow(0.48);
    let wood = p.k.wood;
    for (u0, s) in [(-0.22, 0.26), (0.24, 0.24)] {
        p.rect(u0 - s, 0.72, u0 + s, 0.8, wood);
        p.rect(u0 - s - 0.03, 0.4, u0 + s + 0.03, 0.46, shade(wood, -0.08));
        p.line(
            (u0 - s * 0.6, 0.0),
            (u0 - s * 0.1, 0.74),
            0.035,
            shade(wood, -0.2),
        );
        p.line(
            (u0 + s * 0.6, 0.0),
            (u0 + s * 0.1, 0.74),
            0.035,
            shade(wood, -0.2),
        );
    }
}

/// An A-frame with two swings, one mid-swing.
pub fn swing_set(p: &mut Pen) {
    p.shadow(0.46);
    let (metal, wood) = (p.k.second, p.k.wood);
    for side in [-1.0_f32, 1.0] {
        p.line((side * 0.44, 0.0), (side * 0.38, 0.95), 0.03, metal);
        p.line((side * 0.3, 0.0), (side * 0.38, 0.95), 0.03, metal);
    }
    p.line((-0.4, 0.95), (0.4, 0.95), 0.035, metal);
    let dark = p.k.dark;
    let swing = 0.05 * p.sway;
    for (u, tilt) in [(-0.16, 0.0), (0.14, 0.06 + swing)] {
        p.line((u - 0.06, 0.94), (u - 0.06 + tilt, 0.3), 0.008, dark);
        p.line((u + 0.06, 0.94), (u + 0.06 + tilt, 0.3), 0.008, dark);
        p.rect(u - 0.08 + tilt, 0.27, u + 0.08 + tilt, 0.31, wood);
    }
}

/// A clinker rowing boat with oars shipped.
pub fn rowing_boat(p: &mut Pen) {
    let paint = p.roof;
    p.poly(
        &[(-0.48, 0.75), (0.48, 0.75), (0.34, 0.1), (-0.3, 0.1)],
        paint,
    );
    p.poly(
        &[(-0.46, 0.66), (0.46, 0.66), (0.43, 0.54), (-0.43, 0.54)],
        shade(paint, 0.2),
    );
    p.rect(-0.47, 0.72, 0.47, 0.8, p.k.pale);
    p.line((-0.2, 0.9), (0.46, 0.7), 0.03, p.k.wood);
    p.line((-0.3, 0.72), (0.3, 0.95), 0.03, p.k.wood);
    p.ell(0.0, 0.08, 0.36, 0.08, p.k.water.opacity(0.35));
}

/// An open hexagonal pavilion with a pointed roof and lattice rails.
pub fn gazebo(p: &mut Pen) {
    p.shadow(0.44);
    let (white, roof) = (p.k.pale, p.roof);
    p.rect(-0.42, 0.0, 0.42, 0.08, shade(p.k.wood, -0.1));
    for u in [-0.38, -0.13, 0.13, 0.38] {
        p.rect(u - 0.015, 0.08, u + 0.015, 0.56, white);
    }
    for u in [-0.38, 0.13] {
        for step in 0..4 {
            let a = u + step as f32 * 0.065;
            p.line((a, 0.1), (a + 0.065, 0.26), 0.006, white);
            p.line((a + 0.065, 0.1), (a, 0.26), 0.006, white);
        }
    }
    p.rect(-0.4, 0.26, 0.4, 0.28, white);
    p.rect(-0.44, 0.55, 0.44, 0.6, shade(white, -0.08));
    p.poly(&[(-0.48, 0.6), (0.0, 0.93), (0.48, 0.6)], roof);
    p.poly(&[(-0.16, 0.6), (0.0, 0.93), (0.06, 0.6)], shade(roof, 0.15));
    p.circ(0.0, 0.95, 0.025, p.k.brass);
}

/// A round basin with a tiered middle and water rising from the top.
pub fn square_fountain(p: &mut Pen) {
    p.shadow(0.46);
    let (stone, water) = (p.k.stone, p.k.water);
    p.rect(-0.46, 0.0, 0.46, 0.2, shade(stone, -0.08));
    p.ell(0.0, 0.2, 0.46, 0.06, shade(stone, 0.1));
    p.ell(0.0, 0.2, 0.4, 0.04, shade(water, 0.2));
    p.rect(-0.05, 0.2, 0.05, 0.5, stone);
    p.ell(0.0, 0.5, 0.24, 0.05, shade(stone, 0.1));
    p.rect(-0.03, 0.5, 0.03, 0.72, stone);
    p.ell(0.0, 0.72, 0.12, 0.03, shade(stone, 0.1));
    let spray = shade(water, 0.45).opacity(0.85);
    p.line((0.0, 0.74), (0.0, 0.95), 0.025, spray);
    for side in [-1.0_f32, 1.0] {
        p.curve(
            (0.0, 0.93),
            (side * 0.12, 1.0),
            (side * 0.14, 0.72),
            0.014,
            spray,
        );
        p.curve(
            (side * 0.2, 0.5),
            (side * 0.32, 0.55),
            (side * 0.34, 0.22),
            0.014,
            spray,
        );
    }
}

/// A low round wishing well of a fountain with coins glinting.
pub fn wishing_fountain(p: &mut Pen) {
    p.shadow(0.44);
    let (stone, water) = (p.k.stone, p.k.water);
    p.rect(-0.42, 0.0, 0.42, 0.3, stone);
    for step in 0..5 {
        let u = -0.42 + step as f32 * 0.21;
        p.rect(u, 0.12, u + 0.005, 0.3, shade(stone, -0.2));
    }
    p.ell(0.0, 0.3, 0.42, 0.07, shade(stone, 0.12));
    p.ell(0.0, 0.3, 0.36, 0.05, water);
    for u in [-0.15, 0.05, 0.2] {
        p.circ(u, 0.3, 0.015, p.k.brass);
    }
    p.rect(-0.03, 0.3, 0.03, 0.55, shade(stone, -0.1));
    p.circ(0.0, 0.6, 0.07, shade(stone, 0.05));
    let spray = shade(water, 0.45).opacity(0.85);
    p.curve((0.0, 0.62), (0.18, 0.9), (0.24, 0.36), 0.014, spray);
    p.curve((0.0, 0.62), (-0.18, 0.9), (-0.24, 0.36), 0.014, spray);
}

/// A tall pole with a garland crown and ribbons streaming down.
pub fn maypole(p: &mut Pen) {
    p.shadow(0.3);
    let wood = p.k.pale;
    p.rect(-0.02, 0.0, 0.02, 0.95, wood);
    p.circ(0.0, 0.96, 0.03, p.k.brass);
    p.ell(0.0, 0.88, 0.1, 0.02, p.k.leaf);
    let inks = [p.k.accent, p.k.second, p.k.bloom, p.k.leaf];
    for (index, end) in [-0.45_f32, -0.3, -0.15, 0.15, 0.3, 0.45].iter().enumerate() {
        let ink = inks[index % inks.len()];
        p.curve(
            (0.0, 0.88),
            (end * 0.6, 0.5 + 0.1 * p.sway),
            (*end, 0.05),
            0.012,
            ink,
        );
    }
    for index in 0..6 {
        let v = 0.2 + index as f32 * 0.11;
        p.line((-0.02, v), (0.02, v + 0.05), 0.012, inks[index % 4]);
    }
}

/// A tower mill: a tapering tower, a cap, and four latticed sails.
pub fn windmill(p: &mut Pen) {
    p.shadow(0.3);
    let (wall, roof, wood) = (p.k.pale, p.roof, p.k.wood);
    p.poly(
        &[(-0.22, 0.0), (0.22, 0.0), (0.14, 0.6), (-0.14, 0.6)],
        wall,
    );
    p.rect(-0.2, 0.28, 0.2, 0.3, shade(wall, -0.15));
    p.window(-0.04, 0.4, 0.04, 0.46);
    p.arch(-0.06, 0.06, 0.0, 0.14, shade(wood, -0.2));
    p.dome(0.0, 0.6, 0.17, 0.09, roof);
    // Sails, turned a little.
    let hub = (0.0, 0.64);
    let turn = 0.35;
    for arm in 0..4 {
        let angle = turn + arm as f32 * std::f32::consts::FRAC_PI_2;
        let (du, dv) = (angle.cos() * 0.46, angle.sin() * 0.46 * p.w / p.h);
        let tip = (hub.0 + du, hub.1 + dv);
        p.line(hub, tip, 0.018, shade(wood, -0.2));
        let (nu, nv) = (-angle.sin() * 0.09, angle.cos() * 0.09 * p.w / p.h);
        let mid = (hub.0 + du * 0.3, hub.1 + dv * 0.3);
        p.poly(
            &[mid, tip, (tip.0 + nu, tip.1 + nv), (mid.0 + nu, mid.1 + nv)],
            p.k.pale.opacity(0.9),
        );
        for t in [0.45, 0.65, 0.85] {
            let a = (hub.0 + du * t, hub.1 + dv * t);
            p.line(a, (a.0 + nu, a.1 + nv), 0.006, shade(wood, -0.1));
        }
    }
    p.circ(hub.0, hub.1, 0.03, p.k.dark);
}

/// Two stacked hives with little roofs, bees about.
pub fn beehives(p: &mut Pen) {
    p.shadow(0.42);
    let (white, roof) = (p.k.pale, p.roof);
    for u in [-0.22, 0.22] {
        p.rect(u - 0.12, 0.0, u + 0.12, 0.06, shade(p.k.wood, -0.2));
        for (index, v) in [0.06, 0.28, 0.5].iter().enumerate() {
            let inset = 0.02 * index as f32;
            p.rect(
                u - 0.15 + inset,
                *v,
                u + 0.15 - inset,
                v + 0.2,
                shade(white, -0.04 * index as f32),
            );
        }
        p.gable(u - 0.19, u + 0.19, 0.7, 0.9, roof);
        p.rect(u - 0.06, 0.08, u + 0.06, 0.1, p.k.dark);
    }
    for (u, v) in [(0.0, 0.8), (0.06, 0.66), (-0.05, 0.94)] {
        p.circ(u, v, 0.015, p.k.brass);
    }
}

/// Straw skeps among flowers.
pub fn bee_garden(p: &mut Pen) {
    p.shadow(0.46);
    let straw = p.k.brass;
    p.flowers(-0.48, 0.48, 0.02, 12, 0.03);
    for u in [-0.2, 0.18] {
        p.rect(u - 0.12, 0.0, u + 0.12, 0.2, shade(p.k.wood, -0.2));
        p.dome(u, 0.2, 0.14, 0.42, straw);
        for v in [0.3, 0.42, 0.52] {
            p.line((u - 0.12, v), (u + 0.12, v), 0.006, shade(straw, -0.25));
        }
        p.rect(u - 0.03, 0.2, u + 0.03, 0.25, p.k.dark);
    }
}

/// A small span-roofed glasshouse of panes, green inside.
pub fn glasshouse(p: &mut Pen) {
    p.shadow(0.42);
    let frame = p.k.pale;
    let glass = shade(p.k.water, 0.55).opacity(0.8);
    p.rect(-0.4, 0.0, 0.4, 0.12, shade(p.k.stone, -0.1));
    p.rect(-0.4, 0.12, 0.4, 0.58, glass);
    p.poly(&[(-0.42, 0.58), (0.0, 0.92), (0.42, 0.58)], glass);
    for (u, v, r) in [(-0.22, 0.28, 0.1), (0.0, 0.34, 0.12), (0.2, 0.27, 0.1)] {
        p.circ(u, v, r, p.k.leaf.opacity(0.85));
    }
    for step in 0..5 {
        let u = -0.4 + step as f32 * 0.2;
        p.line((u, 0.12), (u, 0.58), 0.01, frame);
    }
    p.line((-0.4, 0.35), (0.4, 0.35), 0.01, frame);
    p.line((-0.42, 0.58), (0.0, 0.92), 0.014, frame);
    p.line((0.0, 0.92), (0.42, 0.58), 0.014, frame);
    p.line((0.0, 0.92), (0.0, 0.58), 0.01, frame);
}

/// A plank house up in an old tree, a ladder hanging down.
pub fn treehouse(p: &mut Pen) {
    let wood = p.k.wood;
    p.tree(0.0, 0.62, 0.38);
    p.rect(-0.26, 0.42, 0.26, 0.46, shade(wood, -0.2));
    p.rect(-0.22, 0.46, 0.22, 0.66, wood);
    for v in [0.51, 0.56, 0.61] {
        p.line((-0.22, v), (0.22, v), 0.005, shade(wood, -0.2));
    }
    p.pitched(-0.22, 0.22, 0.66, 0.78, p.roof);
    p.window(-0.07, 0.52, 0.05, 0.6);
    let dark = shade(wood, -0.3);
    p.line((0.14, 0.42), (0.18, 0.0), 0.01, dark);
    p.line((0.24, 0.42), (0.28, 0.0), 0.01, dark);
    for step in 0..6 {
        let v = 0.05 + step as f32 * 0.065;
        p.line((0.16, v), (0.27, v), 0.01, dark);
    }
}

/// Stepped wooden seats on a steel frame.
pub fn bleachers(p: &mut Pen) {
    p.shadow(0.48);
    let (wood, metal) = (p.k.wood, p.k.metal);
    for row in 0..4 {
        let v = 0.18 + row as f32 * 0.22;
        let u0 = -0.46 + row as f32 * 0.08;
        p.rect(u0, v, 0.46, v + 0.06, shade(wood, row as f32 * 0.04));
        p.line((u0 + 0.02, v), (u0 + 0.02, 0.0), 0.015, metal);
    }
    p.line((0.44, 0.0), (0.44, 0.84), 0.02, metal);
    p.line((-0.46, 0.0), (0.46, 0.0), 0.02, metal);
    p.line((0.44, 0.84), (0.3, 0.84), 0.015, metal);
}

/// Two square tables with chequered tops and stools.
pub fn chess_tables(p: &mut Pen) {
    p.shadow(0.46);
    let (stone, dark) = (p.k.stone, p.k.dark);
    for u in [-0.22, 0.22] {
        p.rect(u - 0.03, 0.0, u + 0.03, 0.6, shade(stone, -0.1));
        p.rect(u - 0.16, 0.6, u + 0.16, 0.68, stone);
        for step in 0..4 {
            if step % 2 == 0 {
                let a = u - 0.16 + step as f32 * 0.08;
                p.rect(a, 0.66, a + 0.08, 0.68, dark);
            }
        }
        for side in [-1.0_f32, 1.0] {
            p.rect(
                u + side * 0.2 - 0.04,
                0.0,
                u + side * 0.2 + 0.04,
                0.34,
                shade(stone, -0.05),
            );
        }
    }
}

/// A classical little library: pediment, columns and a round window.
pub fn library(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.42, 0.0, 0.42, 0.06, shade(p.k.stone, -0.15));
    p.rect(-0.38, 0.06, 0.38, 0.62, wall);
    for u in [-0.3, -0.12, 0.12, 0.3] {
        p.rect(u - 0.03, 0.06, u + 0.03, 0.56, shade(wall, 0.15));
    }
    p.rect(-0.4, 0.56, 0.4, 0.62, shade(wall, -0.12));
    p.gable(-0.44, 0.44, 0.62, 0.86, roof);
    p.porthole(0.0, 0.72, 0.05);
    p.arch(-0.07, 0.07, 0.06, 0.34, shade(p.trim, -0.1));
    for u in [-0.21, 0.21] {
        p.arch(u - 0.05, u + 0.05, 0.22, 0.44, p.glass);
    }
}

/// A long hall with a curved roof and a row of high windows.
pub fn gym_hall(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.48, 0.0, 0.48, 0.6, wall);
    p.dome(0.0, 0.6, 0.5, 0.3, roof);
    for step in 0..5 {
        let u = -0.36 + step as f32 * 0.18;
        p.window(u - 0.05, 0.4, u + 0.05, 0.52);
    }
    p.rect(-0.1, 0.0, 0.1, 0.26, shade(p.trim, -0.1));
    p.line((0.0, 0.0), (0.0, 0.26), 0.008, shade(p.trim, 0.2));
}

/// A shed with a weather vane of an anemometer and a rain gauge.
pub fn science_shed(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.36, 0.0, 0.36, 0.5, wall);
    p.poly(&[(-0.4, 0.5), (0.4, 0.62), (0.4, 0.56), (-0.4, 0.46)], roof);
    p.window(-0.26, 0.24, -0.06, 0.4);
    p.rect(0.08, 0.0, 0.24, 0.36, shade(p.trim, -0.1));
    let metal = p.k.metal;
    p.line((0.3, 0.6), (0.3, 0.95), 0.012, metal);
    for side in [-1.0_f32, 1.0] {
        p.line((0.3, 0.95), (0.3 + side * 0.1, 0.95), 0.008, metal);
        p.dome(0.3 + side * 0.1, 0.95, 0.03, 0.03, metal);
    }
    p.rect(-0.46, 0.0, -0.42, 0.34, metal);
    p.rect(-0.47, 0.34, -0.41, 0.44, shade(p.k.water, 0.4));
}

/// A little sailing dinghy with a white sail.
pub fn sailboat(p: &mut Pen) {
    let paint = p.roof;
    p.poly(
        &[(-0.44, 0.22), (0.46, 0.22), (0.32, 0.05), (-0.34, 0.05)],
        paint,
    );
    p.rect(-0.45, 0.2, 0.46, 0.24, p.k.pale);
    p.line((-0.02, 0.22), (-0.02, 0.98), 0.018, p.k.wood);
    p.poly(
        &[(0.0, 0.95), (0.0, 0.3), (0.36 + 0.04 * p.sway, 0.3)],
        p.k.pale,
    );
    p.poly(
        &[(-0.04, 0.9), (-0.04, 0.32), (-0.3, 0.32)],
        shade(p.k.pale, -0.08),
    );
    p.line((-0.02, 0.28), (0.38, 0.28), 0.012, p.k.wood);
    p.ell(0.0, 0.05, 0.4, 0.04, p.k.water.opacity(0.35));
}

/// A timber jetty on piles with a rod rest and a bucket.
pub fn fishing_dock(p: &mut Pen) {
    let wood = p.k.wood;
    p.rect(-0.5, 0.42, 0.5, 0.5, wood);
    for step in 0..6 {
        let u = -0.46 + step as f32 * 0.184;
        p.rect(u - 0.025, 0.0, u + 0.025, 0.42, shade(wood, -0.25));
    }
    p.ell(0.0, 0.03, 0.5, 0.05, p.k.water.opacity(0.4));
    p.line((0.3, 0.5), (0.1, 0.98), 0.012, p.k.dark);
    p.curve((0.1, 0.98), (0.2, 0.9), (0.26, 0.2), 0.004, p.k.dark);
    p.rect(-0.3, 0.5, -0.18, 0.62, p.k.second);
}

/// A rope and a knotted seat hanging from a strong branch.
pub fn rope_swing(p: &mut Pen) {
    let wood = p.k.wood;
    p.rect(-0.4, 0.0, -0.26, 0.9, shade(wood, -0.1));
    p.line((-0.3, 0.82), (0.4, 0.86), 0.05, shade(wood, -0.1));
    p.circ(-0.3, 0.92, 0.2, p.k.leaf);
    p.circ(0.1, 0.96, 0.2, shade(p.k.leaf, 0.08));
    p.circ(0.36, 0.9, 0.14, shade(p.k.leaf, -0.06));
    let swing = 0.06 * p.sway;
    p.line(
        (0.2, 0.84),
        (0.2 + swing, 0.22),
        0.012,
        shade(p.k.wood, 0.3),
    );
    p.circ(0.2 + swing, 0.2, 0.05, p.k.wood);
}

/// Raised beds bright with flowers, edged in timber.
pub fn flower_beds(p: &mut Pen) {
    p.shadow(0.48);
    let wood = p.k.wood;
    for u in [-0.25, 0.25] {
        p.rect(u - 0.22, 0.0, u + 0.22, 0.3, shade(wood, -0.05));
        p.rect(u - 0.22, 0.26, u + 0.22, 0.3, shade(wood, 0.1));
        p.flowers(u - 0.2, u + 0.2, 0.3, 6, 0.045);
    }
}

/// A clock on an ornate post, two faces, in the middle of a square.
pub fn town_clock(p: &mut Pen) {
    p.shadow(0.3);
    let metal = p.k.dark;
    p.rect(-0.14, 0.0, 0.14, 0.08, metal);
    p.poly(
        &[(-0.08, 0.08), (0.08, 0.08), (0.04, 0.2), (-0.04, 0.2)],
        metal,
    );
    p.rect(-0.03, 0.2, 0.03, 0.72, metal);
    p.rect(-0.06, 0.4, 0.06, 0.42, p.k.brass);
    p.circ(0.0, 0.81, 0.19, metal);
    p.circ(0.0, 0.81, 0.15, p.k.pale);
    p.line((0.0, 0.81), (0.0, 0.81 + 0.1 * p.w / p.h), 0.014, metal);
    p.line((0.0, 0.81), (0.08, 0.81), 0.016, metal);
    p.circ(0.0, 0.99, 0.03, p.k.brass);
}

/// A kiosk with a striped canopy and a giant cone on top.
pub fn ice_cream_stand(p: &mut Pen) {
    p.shadow(0.42);
    let (white, accent) = (p.k.pale, p.k.bloom);
    p.rr(-0.34, 0.0, 0.34, 0.46, 0.03, white);
    p.rect(-0.34, 0.3, 0.34, 0.34, accent);
    p.rect(-0.26, 0.34, 0.26, 0.44, p.glass);
    for step in 0..6 {
        let u = -0.38 + step as f32 * 0.127;
        let ink = if step % 2 == 0 { accent } else { white };
        p.poly(
            &[(u, 0.6), (u + 0.127, 0.6), (u + 0.127, 0.5), (u, 0.5)],
            ink,
        );
    }
    p.rect(-0.38, 0.6, 0.38, 0.62, shade(accent, -0.2));
    p.poly(&[(-0.08, 0.66), (0.08, 0.66), (0.0, 0.84)], p.k.brass);
    p.circ(0.0, 0.7, 0.1, p.k.bloom);
    p.circ(0.0, 0.76, 0.07, p.k.pale);
}

/// A boathouse: a big gable of weatherboard with wide doors onto the
/// water, and rails down to it.
pub fn boathouse(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.44, 0.0, 0.44, 0.52, wall);
    for step in 0..8 {
        let v = 0.06 + step as f32 * 0.06;
        p.line((-0.44, v), (0.44, v), 0.004, shade(wall, -0.12));
    }
    p.pitched(-0.44, 0.44, 0.52, 0.9, roof);
    p.arch(-0.26, 0.26, 0.0, 0.42, shade(p.k.dark, 0.1));
    p.rect(-0.24, 0.0, -0.01, 0.38, shade(p.trim, 0.05));
    p.rect(0.01, 0.0, 0.24, 0.38, shade(p.trim, -0.05));
    p.porthole(0.0, 0.66, 0.05);
    p.ell(0.0, 0.0, 0.46, 0.03, p.k.water.opacity(0.5));
}

/// An apple tree heavy with fruit.
pub fn apple_tree(p: &mut Pen) {
    p.shadow(0.35);
    p.tree(0.0, 0.62, 0.36);
    for (u, v) in [
        (-0.2, 0.62),
        (0.12, 0.7),
        (0.24, 0.54),
        (-0.05, 0.82),
        (0.0, 0.52),
    ] {
        p.circ(u, v, 0.03, p.k.accent);
    }
}

/// Rows of cabbages and beans on poles.
pub fn vegetable_patch(p: &mut Pen) {
    p.shadow(0.48);
    p.rect(-0.48, 0.0, 0.48, 0.12, shade(p.k.wood, -0.35));
    for step in 0..5 {
        let u = -0.4 + step as f32 * 0.14;
        p.circ(u, 0.16, 0.055, p.k.leaf);
        p.circ(u, 0.18, 0.035, shade(p.k.leaf, 0.2));
    }
    for u in [0.32, 0.42] {
        p.line((u, 0.1), (u + 0.02, 0.92), 0.012, p.k.wood);
        for v in [0.3, 0.5, 0.7] {
            p.circ(u + 0.02, v, 0.03, shade(p.k.leaf, -0.1));
        }
    }
}

/// Sunflowers taller than a child.
pub fn sunflowers(p: &mut Pen) {
    p.shadow(0.4);
    for (u, tall) in [(-0.3, 0.72), (-0.05, 0.9), (0.2, 0.8), (0.38, 0.62)] {
        p.line((u, 0.0), (u, tall), 0.02, p.k.leaf);
        p.ell(u + 0.06, tall * 0.5, 0.06, 0.03, p.k.leaf);
        p.circ(u, tall, 0.1, hex(0xf2c14e));
        p.circ(u, tall, 0.045, shade(p.k.wood, -0.3));
    }
}

/// Herbs in terracotta pots of three sizes.
pub fn herb_pots(p: &mut Pen) {
    p.shadow(0.44);
    let pot = hex(0xc0683f);
    for (u, s) in [(-0.28, 0.14), (0.02, 0.18), (0.3, 0.12)] {
        p.poly(
            &[
                (u - s, s * 1.6),
                (u + s, s * 1.6),
                (u + s * 0.75, 0.0),
                (u - s * 0.75, 0.0),
            ],
            pot,
        );
        p.rect(
            u - s * 1.05,
            s * 1.5,
            u + s * 1.05,
            s * 1.7,
            shade(pot, 0.1),
        );
        for side in [-0.5_f32, 0.0, 0.5] {
            p.circ(u + side * s, s * 1.8 + 0.08, s * 0.4, p.k.leaf);
        }
    }
}

/// Window boxes of flowers on a low trellis.
pub fn flower_boxes(p: &mut Pen) {
    p.shadow(0.42);
    let wood = p.k.wood;
    for u in [-0.4, 0.4] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.8, shade(wood, -0.2));
    }
    for v in [0.14, 0.52] {
        p.rect(-0.42, v, 0.42, v + 0.14, p.roof);
        p.flowers(-0.38, 0.38, v + 0.14, 8, 0.04);
    }
}

/// Paper lanterns hung along a line between two posts.
pub fn paper_lanterns(p: &mut Pen) {
    let wood = p.k.wood;
    for u in [-0.46, 0.46] {
        p.rect(u - 0.015, 0.0, u + 0.015, 0.95, wood);
    }
    p.curve((-0.46, 0.92), (0.0, 0.7), (0.46, 0.92), 0.008, p.k.dark);
    let inks = [p.k.accent, p.k.bloom, p.k.brass];
    for step in 0..5 {
        let t = (step as f32 + 0.5) / 5.0;
        let u = -0.46 + 0.92 * t;
        let v = 0.92 - 0.44 * t * (1.0 - t) * 2.0;
        p.line((u, v), (u, v - 0.06), 0.006, p.k.dark);
        p.ell(u, v - 0.13, 0.06, 0.08, inks[step % 3]);
        p.rect(u - 0.03, v - 0.06, u + 0.03, v - 0.05, p.k.dark);
    }
}

/// A cast-iron lamp post with a lantern head.
pub fn lamp_post(p: &mut Pen) {
    p.shadow(0.2);
    let metal = p.k.dark;
    p.rect(-0.1, 0.0, 0.1, 0.06, metal);
    p.poly(
        &[(-0.05, 0.06), (0.05, 0.06), (0.025, 0.8), (-0.025, 0.8)],
        metal,
    );
    p.rect(-0.08, 0.3, 0.08, 0.32, metal);
    p.line((-0.12, 0.8), (0.12, 0.8), 0.02, metal);
    p.poly(
        &[(-0.1, 0.82), (0.1, 0.82), (0.13, 0.94), (-0.13, 0.94)],
        p.glass,
    );
    p.poly(&[(-0.15, 0.94), (0.15, 0.94), (0.0, 1.0)], metal);
}

/// A trestle stall under a striped awning, produce on the counter.
pub fn market_stall(p: &mut Pen) {
    p.shadow(0.46);
    let (wood, accent, white) = (p.k.wood, p.roof, p.k.pale);
    for u in [-0.4, 0.4] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.8, shade(wood, -0.1));
    }
    p.rect(-0.44, 0.36, 0.44, 0.42, wood);
    p.rect(-0.42, 0.06, 0.42, 0.36, shade(wood, -0.25));
    for (step, u) in [-0.3, -0.12, 0.08, 0.26].iter().enumerate() {
        p.dome(
            *u,
            0.42,
            0.07,
            0.07,
            [p.k.accent, p.k.leaf, p.k.brass, p.k.bloom][step],
        );
    }
    for step in 0..7 {
        let u = -0.46 + step as f32 * 0.131;
        let ink = if step % 2 == 0 { accent } else { white };
        p.poly(
            &[
                (u, 0.86),
                (u + 0.131, 0.86),
                (u + 0.14, 0.72),
                (u + 0.01, 0.72),
            ],
            ink,
        );
        p.dome(u + 0.07, 0.72, 0.065, -0.04, ink);
    }
}

/// One birdhouse on a pole, a round door and a perch.
pub fn birdhouse(p: &mut Pen) {
    p.shadow(0.2);
    let wood = p.k.wood;
    p.rect(-0.03, 0.0, 0.03, 0.62, shade(wood, -0.2));
    p.rect(-0.16, 0.62, 0.16, 0.86, p.wall);
    p.gable(-0.22, 0.22, 0.86, 1.0, p.roof);
    p.circ(0.0, 0.76, 0.05, p.k.dark);
    p.line((0.0, 0.68), (0.0, 0.65), 0.02, wood);
}

/// A signpost with one arm.
pub fn signpost(p: &mut Pen) {
    p.shadow(0.2);
    let wood = p.k.wood;
    p.rect(-0.03, 0.0, 0.03, 0.95, shade(wood, -0.1));
    p.poly(
        &[
            (0.03, 0.74),
            (0.38, 0.74),
            (0.46, 0.8),
            (0.38, 0.86),
            (0.03, 0.86),
        ],
        p.roof,
    );
    p.line((0.08, 0.8), (0.32, 0.8), 0.012, p.k.pale);
    p.rect(-0.04, 0.95, 0.04, 0.98, wood);
}

/// A drinking fountain: a stone pillar with a spout and a basin.
pub fn wall_fountain(p: &mut Pen) {
    p.shadow(0.3);
    let stone = p.k.stone;
    p.rect(-0.14, 0.0, 0.14, 0.9, stone);
    p.dome(0.0, 0.9, 0.14, 0.08, shade(stone, 0.1));
    p.circ(0.0, 0.66, 0.06, shade(stone, -0.2));
    p.rect(-0.02, 0.6, 0.12, 0.63, p.k.brass);
    p.poly(
        &[(-0.3, 0.36), (0.3, 0.36), (0.22, 0.22), (-0.22, 0.22)],
        shade(stone, -0.1),
    );
    p.line((0.12, 0.6), (0.14, 0.38), 0.012, shade(p.k.water, 0.4));
}

/// A round well with a winding handle, no roof.
pub fn wellhead(p: &mut Pen) {
    p.shadow(0.4);
    let stone = p.k.stone;
    p.rect(-0.3, 0.0, 0.3, 0.4, stone);
    for v in [0.13, 0.26] {
        p.line((-0.3, v), (0.3, v), 0.006, shade(stone, -0.2));
    }
    p.ell(0.0, 0.4, 0.3, 0.05, shade(stone, 0.12));
    let wood = p.k.wood;
    for u in [-0.26, 0.26] {
        p.rect(u - 0.02, 0.4, u + 0.02, 0.8, wood);
    }
    p.line((-0.3, 0.72), (0.34, 0.72), 0.03, shade(wood, -0.15));
    p.line((0.34, 0.72), (0.4, 0.62), 0.02, p.k.dark);
    p.line((0.0, 0.72), (0.0, 0.55), 0.006, p.k.dark);
    p.rect(-0.05, 0.47, 0.05, 0.55, p.k.metal);
}

/// A bench seat hung on chains from a frame: a garden swing seat.
pub fn garden_swing(p: &mut Pen) {
    p.shadow(0.45);
    let (wood, dark) = (p.k.wood, p.k.dark);
    for side in [-1.0_f32, 1.0] {
        p.line((side * 0.46, 0.0), (side * 0.38, 0.9), 0.03, wood);
    }
    p.rect(-0.42, 0.88, 0.42, 0.94, wood);
    p.gable(-0.46, 0.46, 0.94, 1.0, p.roof);
    let swing = 0.03 * p.sway;
    for u in [-0.26, 0.26] {
        p.line((u, 0.88), (u + swing, 0.32), 0.006, dark);
    }
    let seat = p.roof;
    p.rect(-0.3 + swing, 0.28, 0.3 + swing, 0.33, seat);
    p.rect(-0.3 + swing, 0.33, -0.27 + swing, 0.56, seat);
    p.rect(0.27 + swing, 0.33, 0.3 + swing, 0.56, seat);
    p.rect(-0.3 + swing, 0.48, 0.3 + swing, 0.53, seat);
}

/// Something going up: a faint outline of the work behind a frame of
/// poles and planks, a ladder against it, a bucket on a rope.
pub fn scaffold(p: &mut Pen) {
    p.shadow(0.46);
    let wood = p.k.wood;
    let faint = p.wall.opacity(0.45);
    p.rect(-0.36, 0.0, 0.3, 0.6, faint);
    p.gable(-0.4, 0.34, 0.6, 0.84, p.roof.opacity(0.35));
    for u in [-0.42, -0.06, 0.36] {
        p.rect(u - 0.014, 0.0, u + 0.014, 0.9, shade(wood, 0.1));
    }
    for v in [0.3, 0.6, 0.88] {
        p.rect(-0.46, v, 0.4, v + 0.035, wood);
    }
    p.line((-0.42, 0.3), (-0.06, 0.6), 0.012, shade(wood, -0.2));
    p.line((-0.06, 0.3), (0.36, 0.6), 0.012, shade(wood, -0.2));
    for u in [0.42, 0.5] {
        p.line((u - 0.02, 0.0), (u - 0.1, 0.66), 0.012, shade(wood, 0.2));
    }
    for step in 0..6 {
        let t = step as f32 / 6.0;
        p.line(
            (0.4 - 0.08 * t, 0.06 + t * 0.6),
            (0.48 - 0.08 * t, 0.06 + t * 0.6),
            0.01,
            shade(wood, 0.2),
        );
    }
    p.line((-0.24, 0.88), (-0.24, 0.66), 0.006, p.k.dark);
    p.rect(-0.28, 0.6, -0.2, 0.67, p.k.metal);
}
