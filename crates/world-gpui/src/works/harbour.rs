//! The harbour's own drawings: what a fishing town on a green shore
//! builds, from its lighthouse and clock to a bait shed and a cairn.

use super::Pen;
use crate::art::{hex, shade};

/// A white tower banded in red, its lamp room glazed, on a stone foot.
pub fn lighthouse(p: &mut Pen) {
    let (white, red) = (p.k.pale, p.k.accent);
    p.shadow(0.3);
    p.rect(-0.3, 0.0, 0.3, 0.08, shade(p.k.stone, -0.1));
    p.poly(
        &[(-0.2, 0.08), (0.2, 0.08), (0.13, 0.78), (-0.13, 0.78)],
        white,
    );
    for (v0, v1) in [(0.26, 0.36), (0.52, 0.62)] {
        let half = |v: f32| 0.2 - 0.07 * (v - 0.08) / 0.7;
        p.poly(
            &[
                (-half(v0), v0),
                (half(v0), v0),
                (half(v1), v1),
                (-half(v1), v1),
            ],
            red,
        );
    }
    p.arch(-0.05, 0.05, 0.08, 0.18, shade(p.trim, -0.2));
    p.rect(-0.18, 0.78, 0.18, 0.8, p.k.dark);
    p.rect(-0.1, 0.8, 0.1, 0.9, p.glass);
    p.line((-0.18, 0.84), (0.18, 0.84), 0.01, p.k.dark);
    p.dome(0.0, 0.9, 0.12, 0.07, red);
    p.line((0.0, 0.96), (0.0, 1.0), 0.012, p.k.dark);
}

/// A pillar clock on the quay: a fluted iron column carrying a drum with
/// a face each side, a lamp above.
pub fn harbour_clock(p: &mut Pen) {
    p.shadow(0.3);
    let (iron, face) = (p.k.second, p.k.pale);
    p.rect(-0.16, 0.0, 0.16, 0.1, iron);
    p.rect(-0.08, 0.1, 0.08, 0.14, shade(iron, 0.15));
    p.rect(-0.045, 0.14, 0.045, 0.66, iron);
    for u in [-0.02, 0.02] {
        p.line((u, 0.16), (u, 0.64), 0.006, shade(iron, 0.25));
    }
    p.rr(-0.3, 0.64, 0.3, 0.88, 0.1, iron);
    for u in [-0.14, 0.14] {
        p.circ(u, 0.76, 0.12, p.k.brass);
        p.circ(u, 0.76, 0.1, face);
        p.line((u, 0.76), (u, 0.81), 0.012, p.k.dark);
        p.line((u, 0.76), (u + 0.05, 0.75), 0.014, p.k.dark);
    }
    p.dome(0.0, 0.88, 0.12, 0.06, iron);
    p.circ(0.0, 0.97, 0.03, p.k.brass);
}

/// A curving wall of dressed stone, coping on top, waves at its foot.
pub fn sea_wall(p: &mut Pen) {
    let stone = p.k.stone;
    p.poly(&[(-0.5, 0.0), (0.5, 0.0), (0.46, 0.7), (-0.46, 0.7)], stone);
    for row in 0..4 {
        let v = 0.14 + row as f32 * 0.14;
        p.line((-0.49, v), (0.49, v), 0.006, shade(stone, -0.2));
        for step in 0..6 {
            let u = -0.44 + step as f32 * 0.17 + if row % 2 == 0 { 0.0 } else { 0.08 };
            p.line((u, v), (u, v + 0.14), 0.006, shade(stone, -0.2));
        }
    }
    p.rect(-0.5, 0.7, 0.5, 0.82, shade(stone, 0.15));
    p.ell(0.0, 0.0, 0.52, 0.1, p.k.water.opacity(0.6));
    for u in [-0.3, 0.1, 0.36] {
        p.curve(
            (u - 0.08, 0.1),
            (u, 0.22),
            (u + 0.08, 0.1),
            0.02,
            shade(p.k.water, 0.5),
        );
    }
}

/// Raised beds behind a picket fence, a sunflower and a hand-painted sign.
pub fn school_garden(p: &mut Pen) {
    p.shadow(0.48);
    let (wood, white) = (p.k.wood, p.k.pale);
    p.rect(-0.46, 0.0, 0.1, 0.18, shade(wood, -0.1));
    p.flowers(-0.44, 0.08, 0.18, 7, 0.035);
    p.line((0.28, 0.0), (0.28, 0.82), 0.018, p.k.leaf);
    p.circ(0.28, 0.84, 0.08, p.k.brass);
    p.circ(0.28, 0.84, 0.035, shade(wood, -0.3));
    for step in 0..9 {
        let u = -0.48 + step as f32 * 0.12;
        p.poly(
            &[
                (u, 0.0),
                (u + 0.04, 0.0),
                (u + 0.04, 0.26),
                (u + 0.02, 0.3),
                (u, 0.26),
            ],
            white,
        );
    }
    p.rect(-0.48, 0.14, 0.52, 0.17, white);
    p.rect(0.36, 0.0, 0.38, 0.5, wood);
    p.rect(0.3, 0.42, 0.5, 0.56, p.roof);
}

/// A statue of the first fishers on a plinth: two figures hauling a net
/// between them, faces turned to the sea.
pub fn fishers_statue(p: &mut Pen) {
    p.shadow(0.44);
    let bronze = hex(0x7a8a70);
    let (dark, stone) = (shade(bronze, -0.35), p.k.stone);
    p.rect(-0.42, 0.0, 0.42, 0.26, stone);
    p.rect(-0.46, 0.24, 0.46, 0.29, shade(stone, 0.1));
    p.rect(-0.2, 0.08, 0.2, 0.16, shade(stone, -0.12));
    for (u, lean) in [(-0.2, 0.05), (0.2, -0.05)] {
        p.poly(
            &[
                (u - 0.08, 0.29),
                (u + 0.08, 0.29),
                (u + 0.07 + lean, 0.7),
                (u - 0.07 + lean, 0.7),
            ],
            bronze,
        );
        p.circ(u + lean, 0.8, 0.075, bronze);
        p.ell(u + lean, 0.86, 0.09, 0.025, dark);
        p.face(u + lean, 0.8, 0.075, dark);
        p.line((u + lean, 0.62), (u - lean * 4.0, 0.46), 0.03, bronze);
    }
    // The net between them.
    for step in 0..5 {
        let t = step as f32 / 4.0;
        p.line(
            (-0.18 + 0.36 * t, 0.46),
            (-0.14 + 0.28 * t, 0.34),
            0.006,
            dark,
        );
    }
    p.curve((-0.2, 0.46), (0.0, 0.4), (0.2, 0.46), 0.01, dark);
    p.curve((-0.16, 0.34), (0.0, 0.3), (0.16, 0.34), 0.01, dark);
}

/// A fisherman in stone on a plinth, sou'wester on, a rod over his
/// shoulder, looking out.
pub fn fisherman_statue(p: &mut Pen) {
    p.shadow(0.3);
    let stone = shade(p.k.stone, 0.1);
    let dark = shade(stone, -0.45);
    p.rect(-0.26, 0.0, 0.26, 0.28, shade(stone, -0.15));
    p.rect(-0.3, 0.26, 0.3, 0.3, shade(stone, -0.05));
    p.poly(
        &[(-0.12, 0.3), (0.12, 0.3), (0.1, 0.72), (-0.1, 0.72)],
        stone,
    );
    p.line((-0.03, 0.3), (-0.03, 0.5), 0.01, dark);
    p.circ(0.0, 0.8, 0.09, stone);
    p.face(0.0, 0.8, 0.09, dark);
    p.ell(0.0, 0.86, 0.15, 0.025, shade(stone, -0.1));
    p.dome(0.0, 0.86, 0.09, 0.06, shade(stone, -0.1));
    p.line((0.08, 0.66), (0.4, 1.0), 0.02, shade(stone, -0.2));
    p.line((0.1, 0.62), (0.08, 0.7), 0.04, stone);
}

/// A stone well under a little tiled roof, bucket on its rope.
pub fn roofed_well(p: &mut Pen) {
    p.shadow(0.4);
    let (stone, wood) = (p.k.stone, p.k.wood);
    p.rect(-0.3, 0.0, 0.3, 0.34, stone);
    for v in [0.11, 0.22] {
        p.line((-0.3, v), (0.3, v), 0.006, shade(stone, -0.2));
    }
    p.ell(0.0, 0.34, 0.3, 0.04, shade(stone, 0.12));
    for u in [-0.26, 0.26] {
        p.rect(u - 0.025, 0.34, u + 0.025, 0.78, wood);
    }
    p.line((-0.3, 0.66), (0.3, 0.66), 0.03, shade(wood, -0.15));
    p.line((0.0, 0.66), (0.0, 0.46), 0.006, p.k.dark);
    p.rect(-0.05, 0.4, 0.05, 0.48, p.k.metal);
    p.gable(-0.42, 0.42, 0.76, 1.0, p.roof);
}

/// A red pillar box with a crowned cap and its slot.
pub fn pillar_box(p: &mut Pen) {
    p.shadow(0.24);
    let red = p.k.accent;
    p.rect(-0.2, 0.0, 0.2, 0.06, shade(red, -0.3));
    p.rect(-0.16, 0.06, 0.16, 0.8, red);
    p.rect(-0.2, 0.78, 0.2, 0.84, shade(red, -0.15));
    p.dome(0.0, 0.84, 0.18, 0.12, red);
    p.rect(-0.11, 0.62, 0.11, 0.65, p.k.dark);
    p.rect(-0.08, 0.36, 0.08, 0.52, shade(red, 0.12));
    p.circ(0.0, 0.98, 0.03, p.k.brass);
}

/// A small letter box on a post, a lamp box.
pub fn lamp_box(p: &mut Pen) {
    p.shadow(0.2);
    let red = p.k.accent;
    p.rect(-0.03, 0.0, 0.03, 0.6, p.k.dark);
    p.rr(-0.16, 0.6, 0.16, 0.92, 0.03, red);
    p.dome(0.0, 0.92, 0.16, 0.06, shade(red, -0.1));
    p.rect(-0.09, 0.82, 0.09, 0.84, p.k.dark);
}

/// Three birdhouses on poles of different heights along a lane.
pub fn birdhouses(p: &mut Pen) {
    p.shadow(0.44);
    let wood = p.k.wood;
    for (u, tall, roof) in [
        (-0.3, 0.55, p.roof),
        (0.02, 0.75, p.k.second),
        (0.32, 0.45, p.k.accent),
    ] {
        p.rect(u - 0.02, 0.0, u + 0.02, tall, shade(wood, -0.2));
        p.rect(u - 0.1, tall, u + 0.1, tall + 0.14, p.k.pale);
        p.gable(u - 0.13, u + 0.13, tall + 0.14, tall + 0.24, roof);
        p.circ(u, tall + 0.08, 0.03, p.k.dark);
    }
}

/// A fingerpost with three arms pointing three ways, a finial on top.
pub fn fingerpost(p: &mut Pen) {
    p.shadow(0.24);
    let (white, dark) = (p.k.pale, p.k.dark);
    p.rect(-0.03, 0.0, 0.03, 0.92, white);
    p.circ(0.0, 0.94, 0.04, white);
    for (v, dir, long) in [(0.8, 1.0, 0.4), (0.68, -1.0, 0.36), (0.56, 1.0, 0.3)] {
        let end = dir * long;
        p.poly(
            &[
                (0.0, v),
                (end, v),
                (end + dir * 0.06, v - 0.04),
                (end, v - 0.08),
                (0.0, v - 0.08),
            ],
            white,
        );
        p.line(
            (dir * 0.06, v - 0.04),
            (end - dir * 0.02, v - 0.04),
            0.01,
            dark,
        );
    }
}

/// Half-barrel planters in a row along the quay, spilling flowers.
pub fn quay_planters(p: &mut Pen) {
    p.shadow(0.48);
    let wood = p.k.wood;
    for u in [-0.33, 0.0, 0.33] {
        p.poly(
            &[
                (u - 0.14, 0.42),
                (u + 0.14, 0.42),
                (u + 0.11, 0.0),
                (u - 0.11, 0.0),
            ],
            wood,
        );
        for v in [0.1, 0.32] {
            p.rect(u - 0.13, v, u + 0.13, v + 0.03, p.k.metal);
        }
        p.flowers(u - 0.12, u + 0.12, 0.42, 4, 0.05);
    }
}

/// A lifeboat station: a tall shed with wide blue doors, a slipway down
/// from them, and a flagstaff.
pub fn lifeboat_station(p: &mut Pen) {
    let (white, blue) = (p.k.pale, p.k.second);
    p.rect(-0.42, 0.14, 0.34, 0.66, white);
    p.pitched(-0.42, 0.34, 0.66, 0.9, p.k.accent);
    p.rect(-0.26, 0.14, 0.18, 0.54, blue);
    p.line((-0.04, 0.14), (-0.04, 0.54), 0.008, shade(blue, 0.3));
    p.porthole(-0.04, 0.73, 0.04);
    p.poly(
        &[(-0.3, 0.14), (0.22, 0.14), (0.36, 0.0), (-0.2, 0.0)],
        shade(p.k.stone, -0.1),
    );
    p.line((-0.14, 0.14), (-0.06, 0.0), 0.01, p.k.metal);
    p.line((0.06, 0.14), (0.14, 0.0), 0.01, p.k.metal);
    p.line((0.44, 0.0), (0.44, 0.98), 0.012, p.k.dark);
    p.poly(&[(0.44, 0.97), (0.44, 0.87), (0.34, 0.92)], p.k.accent);
}

/// A long village hall with tall windows, a porch and a bell-cote.
pub fn village_hall(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.46, 0.0, 0.46, 0.56, wall);
    p.pitched(-0.46, 0.46, 0.56, 0.84, roof);
    for u in [-0.34, -0.2, 0.2, 0.34] {
        p.arch(u - 0.045, u + 0.045, 0.16, 0.44, p.glass);
    }
    p.rect(-0.12, 0.0, 0.12, 0.4, shade(wall, 0.08));
    p.gable(-0.15, 0.15, 0.4, 0.52, shade(roof, -0.1));
    p.arch(-0.05, 0.05, 0.0, 0.3, shade(p.trim, -0.1));
    p.rect(0.24, 0.8, 0.34, 0.92, shade(wall, 0.05));
    p.circ(0.29, 0.85, 0.025, p.k.brass);
    p.gable(0.22, 0.36, 0.92, 1.0, roof);
}

/// Three bathing huts in a row, each striped its own colour.
pub fn bathing_huts(p: &mut Pen) {
    p.shadow(0.48);
    let inks = [p.k.accent, p.k.second, hex(0x5d8c74)];
    for (index, u) in [-0.32, 0.0, 0.32].iter().enumerate() {
        let ink = inks[index];
        p.rect(u - 0.13, 0.06, u + 0.13, 0.66, p.k.pale);
        for step in 0..3 {
            let a = u - 0.13 + step as f32 * 0.1;
            p.rect(a, 0.06, a + 0.04, 0.66, ink);
        }
        p.gable(u - 0.16, u + 0.16, 0.66, 0.88, shade(ink, -0.15));
        p.rect(u - 0.05, 0.06, u + 0.05, 0.4, shade(ink, -0.25));
        p.rect(u - 0.14, 0.0, u + 0.14, 0.06, p.k.wood);
    }
}

/// Three apple trees in a row, the middle one tallest.
pub fn orchard(p: &mut Pen) {
    p.shadow(0.48);
    for (u, v, r) in [(-0.3, 0.52, 0.2), (0.02, 0.62, 0.24), (0.32, 0.5, 0.19)] {
        p.tree(u, v, r);
        p.circ(u + r * 0.3, v, 0.025, p.k.accent);
        p.circ(u - r * 0.4, v + 0.08, 0.025, p.k.accent);
    }
}

/// A tarred smokehouse with a tall louvred vent and smoke rising.
pub fn smokehouse(p: &mut Pen) {
    let tar = shade(p.k.dark, 0.2);
    p.rect(-0.3, 0.0, 0.3, 0.46, tar);
    for step in 0..6 {
        let u = -0.25 + step as f32 * 0.1;
        p.line((u, 0.0), (u, 0.46), 0.006, shade(tar, 0.25));
    }
    p.pitched(-0.3, 0.3, 0.46, 0.64, shade(p.k.stone, -0.3));
    p.rect(-0.07, 0.6, 0.07, 0.84, tar);
    for v in [0.66, 0.72, 0.78] {
        p.line((-0.06, v), (0.06, v), 0.01, shade(tar, 0.4));
    }
    p.gable(-0.11, 0.11, 0.84, 0.92, tar);
    p.rect(-0.1, 0.0, 0.1, 0.3, shade(p.k.wood, -0.1));
    for (u, v, r) in [(0.04, 0.96, 0.05), (0.12, 1.0, 0.06)] {
        p.circ(u, v - 0.05, r, p.k.pale.opacity(0.6));
    }
}

/// A timber footbridge arching over a stream, with railings.
pub fn footbridge(p: &mut Pen) {
    let wood = p.k.wood;
    p.ell(0.0, 0.04, 0.46, 0.06, p.k.water.opacity(0.6));
    p.curve((-0.5, 0.1), (0.0, 0.62), (0.5, 0.1), 0.06, wood);
    for step in 0..7 {
        let t = step as f32 / 6.0;
        let u = -0.44 + 0.88 * t;
        let v = 0.12 + 0.44 * t * (1.0 - t) * 2.0;
        p.line((u, v), (u, v + 0.3), 0.012, shade(wood, -0.15));
    }
    p.curve(
        (-0.46, 0.42),
        (0.0, 0.94),
        (0.46, 0.42),
        0.02,
        shade(wood, 0.1),
    );
}

/// Steps cut up a slope with a rope rail on posts.
pub fn cliff_path(p: &mut Pen) {
    let (stone, wood) = (p.k.stone, p.k.wood);
    p.poly(
        &[(-0.5, 0.0), (0.5, 0.0), (0.5, 0.8), (0.3, 0.8)],
        shade(p.k.leaf, -0.1),
    );
    for step in 0..6 {
        let u = -0.4 + step as f32 * 0.15;
        let v = step as f32 * 0.13;
        p.rect(u, v, u + 0.16, v + 0.05, stone);
    }
    for step in 0..4 {
        let u = -0.36 + step as f32 * 0.24;
        let v = 0.1 + step as f32 * 0.21;
        p.rect(u - 0.012, v, u + 0.012, v + 0.2, wood);
    }
    p.curve((-0.36, 0.28), (-0.1, 0.4), (0.36, 0.93), 0.01, p.k.pale);
}

/// An open-sided fish market: a slate roof on posts, crates of fish on
/// trestles, a scale hanging.
pub fn fish_market(p: &mut Pen) {
    p.shadow(0.48);
    let (wood, slate) = (p.k.wood, shade(p.k.second, -0.1));
    for u in [-0.44, -0.15, 0.15, 0.44] {
        p.rect(u - 0.018, 0.0, u + 0.018, 0.62, wood);
    }
    p.poly(&[(-0.5, 0.62), (0.5, 0.62), (0.4, 0.9), (-0.4, 0.9)], slate);
    p.rect(-0.5, 0.6, 0.5, 0.64, shade(slate, -0.2));
    for u in [-0.3, 0.0, 0.3] {
        p.rect(u - 0.12, 0.22, u + 0.12, 0.34, shade(wood, 0.1));
        p.line((u - 0.1, 0.22), (u - 0.08, 0.0), 0.012, shade(wood, -0.2));
        p.line((u + 0.1, 0.22), (u + 0.08, 0.0), 0.012, shade(wood, -0.2));
        for f in 0..3 {
            let a = u - 0.07 + f as f32 * 0.07;
            p.ell(a, 0.37, 0.035, 0.02, hex(0xa8b8c0));
        }
    }
    p.line((0.15, 0.62), (0.15, 0.5), 0.006, p.k.dark);
    p.rect(0.11, 0.46, 0.19, 0.5, p.k.brass);
}

/// A brick beehive oven with an arched mouth and a squat chimney.
pub fn bread_oven(p: &mut Pen) {
    let brick = hex(0xb0603f);
    p.rect(-0.42, 0.0, 0.42, 0.22, shade(p.k.stone, -0.1));
    p.dome(0.0, 0.22, 0.38, 0.55, brick);
    for v in [0.36, 0.5, 0.62] {
        p.curve(
            (-0.34, v),
            (0.0, v + 0.06),
            (0.34, v),
            0.006,
            shade(brick, -0.25),
        );
    }
    p.arch(-0.12, 0.12, 0.22, 0.46, p.k.dark);
    p.circ(0.0, 0.3, 0.05, hex(0xff9a3c).opacity(0.9));
    p.rect(0.12, 0.62, 0.24, 0.9, shade(brick, -0.1));
    p.rect(0.1, 0.9, 0.26, 0.93, shade(brick, -0.3));
}

/// A little open belfry on a plain tower, the bell hanging in it.
pub fn bell_tower(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.26, 0.0, 0.26, 0.56, wall);
    p.arch(-0.07, 0.07, 0.0, 0.2, shade(p.trim, -0.1));
    p.window(-0.04, 0.32, 0.04, 0.42);
    p.rect(-0.3, 0.56, 0.3, 0.6, shade(wall, -0.15));
    p.rect(-0.26, 0.6, -0.18, 0.8, wall);
    p.rect(0.18, 0.6, 0.26, 0.8, wall);
    p.rect(-0.26, 0.8, 0.26, 0.83, wall);
    p.line((0.0, 0.8), (0.0, 0.75), 0.01, p.k.dark);
    p.poly(
        &[(-0.06, 0.64), (0.06, 0.64), (0.04, 0.75), (-0.04, 0.75)],
        p.k.brass,
    );
    p.gable(-0.32, 0.32, 0.83, 1.0, roof);
}

/// A puppet booth: striped sides, a painted pediment and a curtained
/// stage with a puppet peeping.
pub fn puppet_theatre(p: &mut Pen) {
    p.shadow(0.36);
    let (red, white) = (p.k.accent, p.k.pale);
    for step in 0..5 {
        let u = -0.32 + step as f32 * 0.128;
        let ink = if step % 2 == 0 { red } else { white };
        p.rect(u, 0.0, u + 0.128, 0.56, ink);
    }
    p.rect(-0.3, 0.56, 0.3, 0.8, p.k.second);
    p.rect(-0.24, 0.58, 0.24, 0.78, p.k.dark);
    p.poly(&[(-0.24, 0.78), (-0.1, 0.78), (-0.24, 0.6)], red);
    p.poly(&[(0.24, 0.78), (0.1, 0.78), (0.24, 0.6)], red);
    p.circ(0.0, 0.68, 0.05, hex(0xf0c7a2));
    p.dome(0.0, 0.7, 0.05, 0.04, p.k.brass);
    p.gable(-0.36, 0.36, 0.8, 0.98, red);
    p.circ(0.0, 0.86, 0.04, p.k.brass);
}

/// A long glasshouse with a white frame and greens inside.
pub fn glasshouse_long(p: &mut Pen) {
    p.shadow(0.48);
    let frame = p.k.pale;
    let glass = shade(p.k.water, 0.55).opacity(0.75);
    p.rect(-0.48, 0.0, 0.48, 0.12, shade(p.k.stone, -0.15));
    p.poly(
        &[
            (-0.48, 0.12),
            (0.48, 0.12),
            (0.48, 0.52),
            (0.1, 0.82),
            (-0.48, 0.52),
        ],
        glass,
    );
    for step in 0..7 {
        let u = -0.4 + step as f32 * 0.13;
        p.circ(u, 0.2, 0.06, p.k.leaf.opacity(0.85));
    }
    for step in 0..9 {
        let u = -0.48 + step as f32 * 0.12;
        p.line((u, 0.12), (u, 0.52), 0.008, frame);
    }
    p.line((-0.48, 0.52), (0.1, 0.82), 0.012, frame);
    p.line((0.1, 0.82), (0.48, 0.52), 0.012, frame);
    p.line((-0.48, 0.52), (0.48, 0.52), 0.01, frame);
}

/// A clubhouse with crossed oars on its gable and a rowing shell on a
/// rack in front.
pub fn rowing_club(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.4, 0.16, 0.4, 0.62, wall);
    p.pitched(-0.4, 0.4, 0.62, 0.94, roof);
    p.line((-0.12, 0.66), (0.12, 0.86), 0.02, p.k.wood);
    p.line((0.12, 0.66), (-0.12, 0.86), 0.02, p.k.wood);
    p.window(-0.3, 0.36, -0.14, 0.5);
    p.window(0.14, 0.36, 0.3, 0.5);
    p.rect(-0.07, 0.16, 0.07, 0.42, shade(p.trim, -0.1));
    for u in [-0.36, 0.36] {
        p.rect(u - 0.015, 0.0, u + 0.015, 0.16, p.k.wood);
    }
    p.poly(
        &[(-0.5, 0.16), (0.5, 0.16), (0.44, 0.1), (-0.44, 0.1)],
        p.k.accent,
    );
}

/// A duck pond with reeds and two ducks.
pub fn duck_pond(p: &mut Pen) {
    p.ell(0.0, 0.2, 0.48, 0.2, shade(p.k.leaf, -0.15));
    p.ell(0.0, 0.2, 0.42, 0.15, p.k.water);
    p.ell(-0.08, 0.24, 0.2, 0.04, shade(p.k.water, 0.3));
    for (u, dir) in [(-0.16, 1.0), (0.18, -1.0)] {
        p.ell(u, 0.24, 0.07, 0.06, p.k.pale);
        p.circ(u + dir * 0.06, 0.34, 0.035, hex(0x3f6a4a));
        p.poly(
            &[
                (u + dir * 0.09, 0.34),
                (u + dir * 0.13, 0.33),
                (u + dir * 0.09, 0.31),
            ],
            p.k.brass,
        );
    }
    for u in [0.36, 0.4, 0.44, -0.42, -0.38] {
        p.line((u, 0.2), (u + 0.02, 0.8), 0.012, shade(p.k.leaf, -0.1));
    }
    p.ell(0.41, 0.7, 0.012, 0.06, p.k.wood);
}

/// Two cottages joined under one long roof, two doors, two chimneys.
pub fn row_cottages(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.46, 0.0, 0.46, 0.5, wall);
    p.rect(0.0, 0.0, 0.46, 0.5, shade(wall, -0.05));
    for u in [-0.28, 0.28] {
        p.rect(u - 0.03, 0.62, u + 0.03, 0.9, shade(roof, -0.2));
    }
    p.poly(
        &[(-0.5, 0.5), (-0.36, 0.82), (0.36, 0.82), (0.5, 0.5)],
        roof,
    );
    for u in [-0.23, 0.23] {
        p.rect(u - 0.05, 0.0, u + 0.05, 0.24, shade(p.trim, -0.1));
        p.window(u - 0.2, 0.26, u - 0.1, 0.38);
        p.window(u + 0.1, 0.26, u + 0.2, 0.38);
    }
}

/// An iron fire basket high on a pole, flames in it.
pub fn hill_beacon(p: &mut Pen) {
    p.shadow(0.3);
    let iron = p.k.dark;
    p.line((-0.2, 0.0), (0.0, 0.7), 0.02, iron);
    p.line((0.2, 0.0), (0.0, 0.7), 0.02, iron);
    p.rect(-0.025, 0.0, 0.025, 0.74, iron);
    p.poly(
        &[(-0.18, 0.84), (0.18, 0.84), (0.1, 0.72), (-0.1, 0.72)],
        iron,
    );
    for u in [-0.12, -0.04, 0.04, 0.12] {
        p.line((u, 0.84), (u * 0.6, 0.72), 0.008, shade(iron, 0.3));
    }
    p.poly(
        &[
            (-0.14, 0.84),
            (-0.06, 0.98),
            (0.0, 0.88),
            (0.06, 1.0),
            (0.14, 0.84),
        ],
        hex(0xf29a3c),
    );
    p.poly(&[(-0.06, 0.84), (0.0, 0.94), (0.06, 0.84)], hex(0xffd27a));
}

/// A shelter for the ferry queue: a curved canopy on posts, a bench and a
/// timetable board.
pub fn ferry_shelter(p: &mut Pen) {
    p.shadow(0.46);
    let (paint, wood) = (p.k.second, p.k.wood);
    for u in [-0.42, 0.42] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.7, paint);
    }
    p.rect(-0.42, 0.18, -0.38, 0.7, paint);
    p.rect(-0.44, 0.0, -0.4, 0.66, shade(p.k.water, 0.5).opacity(0.7));
    p.curve((-0.5, 0.72), (0.0, 0.96), (0.5, 0.72), 0.06, p.roof);
    p.rect(-0.3, 0.24, 0.2, 0.3, wood);
    for u in [-0.26, 0.16] {
        p.rect(u - 0.012, 0.0, u + 0.012, 0.24, shade(wood, -0.2));
    }
    p.rect(0.26, 0.28, 0.4, 0.56, p.k.pale);
    for v in [0.34, 0.4, 0.46] {
        p.line((0.28, v), (0.38, v), 0.008, p.k.dark);
    }
    p.poly(&[(0.28, 0.5), (0.38, 0.5), (0.33, 0.54)], p.k.second);
}

/// A round dovecote on a post, conical roof, doves at its holes.
pub fn dovecote(p: &mut Pen) {
    p.shadow(0.2);
    let wood = p.k.wood;
    p.rect(-0.03, 0.0, 0.03, 0.5, shade(wood, -0.2));
    p.rect(-0.2, 0.5, 0.2, 0.78, p.k.pale);
    p.rect(-0.22, 0.49, 0.22, 0.52, shade(wood, -0.1));
    for row in 0..2 {
        for col in 0..3 {
            let (u, v) = (-0.12 + col as f32 * 0.12, 0.58 + row as f32 * 0.1);
            p.arch(u - 0.03, u + 0.03, v, v + 0.06, p.k.dark);
        }
    }
    p.gable(-0.26, 0.26, 0.78, 0.98, p.roof);
    p.ell(0.14, 0.53, 0.05, 0.02, p.k.pale);
    p.circ(0.18, 0.55, 0.02, p.k.pale);
}

/// A bird hide on stilts with a long slit window, steps up to it.
pub fn seal_hide(p: &mut Pen) {
    let wood = shade(p.k.wood, -0.1);
    for u in [-0.38, -0.1, 0.2, 0.38] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.3, shade(wood, -0.2));
    }
    p.rect(-0.42, 0.3, 0.42, 0.72, wood);
    for step in 0..6 {
        let v = 0.34 + step as f32 * 0.065;
        p.line((-0.42, v), (0.42, v), 0.005, shade(wood, -0.25));
    }
    p.poly(
        &[(-0.48, 0.72), (0.48, 0.8), (0.48, 0.74), (-0.48, 0.66)],
        shade(p.k.leaf, -0.25),
    );
    p.rect(-0.34, 0.5, 0.34, 0.56, p.k.dark);
    for step in 0..4 {
        let u = 0.42 + step as f32 * 0.02;
        p.line(
            (u, 0.3 - step as f32 * 0.075),
            (u + 0.04, 0.3 - step as f32 * 0.075),
            0.02,
            shade(wood, 0.1),
        );
    }
}

/// A knot garden: low box hedges in a pattern, a pot at its heart.
pub fn herb_garden(p: &mut Pen) {
    let hedge = shade(p.k.leaf, -0.15);
    p.ell(0.0, 0.18, 0.48, 0.16, shade(p.k.ground, -0.2));
    p.ell(0.0, 0.18, 0.44, 0.13, hedge);
    p.ell(0.0, 0.18, 0.36, 0.09, shade(p.k.leaf, 0.25));
    p.ell(0.0, 0.18, 0.28, 0.07, hedge);
    p.line((-0.44, 0.18), (0.44, 0.18), 0.03, hedge);
    let pot = hex(0xc0683f);
    p.poly(
        &[(-0.08, 0.4), (0.08, 0.4), (0.06, 0.22), (-0.06, 0.22)],
        pot,
    );
    p.circ(0.0, 0.48, 0.09, p.k.leaf);
    p.circ(-0.04, 0.56, 0.02, p.k.bloom);
    p.circ(0.05, 0.52, 0.02, p.k.bloom);
}

/// A tall tide board on a post standing in the water, marked in feet.
pub fn tide_board(p: &mut Pen) {
    p.ell(0.0, 0.05, 0.4, 0.06, p.k.water.opacity(0.6));
    p.rect(-0.03, 0.0, 0.03, 0.95, p.k.wood);
    p.rect(-0.16, 0.2, 0.16, 0.9, p.k.pale);
    for step in 0..8 {
        let v = 0.24 + step as f32 * 0.08;
        let long = if step % 2 == 0 { 0.14 } else { 0.07 };
        p.line((-0.14, v), (-0.14 + long, v), 0.012, p.k.dark);
    }
    p.rect(0.02, 0.2, 0.14, 0.9, p.k.accent);
    p.rect(-0.16, 0.2, 0.16, 0.34, p.k.water.opacity(0.5));
}

/// A reading room: a bay window full of books under a sign, a lamp lit.
pub fn reading_room(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.42, 0.0, 0.42, 0.72, wall);
    p.pitched(-0.42, 0.42, 0.72, 0.92, roof);
    p.poly(
        &[(-0.3, 0.22), (0.3, 0.22), (0.24, 0.16), (-0.24, 0.16)],
        shade(wall, -0.2),
    );
    p.rect(-0.28, 0.22, 0.28, 0.58, p.glass);
    for (step, ink) in [p.k.accent, p.k.second, p.k.brass, p.k.leaf, p.k.accent]
        .iter()
        .enumerate()
    {
        let u = -0.24 + step as f32 * 0.1;
        p.rect(u, 0.24, u + 0.07, 0.36, *ink);
        p.rect(u + 0.02, 0.4, u + 0.08, 0.5, shade(*ink, 0.2));
    }
    p.line((-0.28, 0.38), (0.28, 0.38), 0.01, p.trim);
    p.poly(
        &[(-0.34, 0.58), (0.34, 0.58), (0.3, 0.64), (-0.3, 0.64)],
        shade(roof, -0.1),
    );
    p.rect(-0.2, 0.66, 0.2, 0.72, p.k.dark);
    p.rect(0.3, 0.0, 0.4, 0.2, shade(p.trim, -0.1));
}

/// A boat on a cradle half built, ribs showing, planks going on.
pub fn boat_yard(p: &mut Pen) {
    p.shadow(0.48);
    let wood = p.k.wood;
    for u in [-0.3, 0.0, 0.3] {
        p.poly(
            &[
                (u - 0.08, 0.0),
                (u + 0.08, 0.0),
                (u + 0.04, 0.2),
                (u - 0.04, 0.2),
            ],
            shade(wood, -0.25),
        );
    }
    p.poly(
        &[(-0.46, 0.66), (0.46, 0.66), (0.3, 0.2), (-0.3, 0.2)],
        shade(wood, 0.05),
    );
    for step in 0..7 {
        let u = -0.36 + step as f32 * 0.12;
        p.line((u, 0.22), (u * 1.2, 0.7), 0.014, shade(wood, -0.3));
    }
    p.poly(
        &[(-0.46, 0.66), (0.46, 0.66), (0.42, 0.56), (-0.42, 0.56)],
        p.roof,
    );
    p.line((-0.48, 0.0), (-0.44, 0.9), 0.014, p.k.dark);
    p.line((0.48, 0.0), (0.44, 0.9), 0.014, p.k.dark);
    p.line((-0.44, 0.86), (0.44, 0.86), 0.012, p.k.dark);
}

/// A harbour wall painted with a map: coast, sea and a compass rose.
pub fn harbour_mural(p: &mut Pen) {
    p.rect(-0.48, 0.0, 0.48, 0.86, p.k.stone);
    p.rect(-0.48, 0.86, 0.48, 0.92, shade(p.k.stone, 0.12));
    p.rect(-0.44, 0.06, 0.44, 0.8, p.k.water);
    p.poly(
        &[
            (-0.44, 0.06),
            (-0.1, 0.06),
            (0.0, 0.3),
            (-0.12, 0.5),
            (0.04, 0.8),
            (-0.44, 0.8),
        ],
        p.k.leaf,
    );
    p.poly(&[(-0.3, 0.2), (-0.2, 0.3), (-0.3, 0.4)], p.k.brass);
    p.circ(0.26, 0.56, 0.1, p.k.pale);
    p.poly(
        &[(0.26, 0.74), (0.29, 0.56), (0.26, 0.38), (0.23, 0.56)],
        p.k.accent,
    );
    p.poly(
        &[(0.12, 0.56), (0.26, 0.59), (0.4, 0.56), (0.26, 0.53)],
        p.k.dark,
    );
    p.ell(0.18, 0.2, 0.05, 0.025, p.k.pale);
}

/// A timber lookout tower: braced legs, a ladder, a hut at the top.
pub fn lookout_tower(p: &mut Pen) {
    p.shadow(0.3);
    let wood = p.k.wood;
    for (a, b) in [((-0.26, 0.0), (-0.16, 0.7)), ((0.26, 0.0), (0.16, 0.7))] {
        p.line(a, b, 0.03, wood);
    }
    for v in [0.2, 0.45] {
        let s = 0.26 - v * 0.14;
        p.line((-s, v), (s, v + 0.2), 0.012, shade(wood, -0.2));
        p.line((s, v), (-s, v + 0.2), 0.012, shade(wood, -0.2));
    }
    p.rect(-0.24, 0.7, 0.24, 0.74, shade(wood, -0.15));
    p.rect(-0.2, 0.74, 0.2, 0.88, p.wall);
    p.rect(-0.16, 0.78, 0.16, 0.85, p.glass);
    p.poly(&[(-0.26, 0.88), (0.26, 0.88), (0.0, 1.0)], p.roof);
    for step in 0..8 {
        let v = 0.05 + step as f32 * 0.08;
        p.line((0.02, v), (0.1, v), 0.01, shade(wood, 0.15));
    }
}

/// A rock pool walled into a pool, a ladder and a railing.
pub fn sea_pool(p: &mut Pen) {
    let stone = p.k.stone;
    p.poly(
        &[
            (-0.5, 0.0),
            (0.5, 0.0),
            (0.44, 0.5),
            (0.2, 0.62),
            (-0.3, 0.56),
            (-0.48, 0.4),
        ],
        shade(stone, -0.25),
    );
    p.poly(
        &[(-0.4, 0.08), (0.4, 0.08), (0.36, 0.42), (-0.36, 0.42)],
        p.k.water,
    );
    p.rect(-0.4, 0.4, 0.4, 0.46, shade(stone, 0.1));
    p.ell(-0.1, 0.3, 0.18, 0.03, shade(p.k.water, 0.35));
    p.line((0.3, 0.46), (0.3, 0.2), 0.012, p.k.metal);
    p.line((0.36, 0.46), (0.36, 0.2), 0.012, p.k.metal);
    p.curve((0.3, 0.46), (0.33, 0.6), (0.36, 0.46), 0.012, p.k.metal);
    for u in [-0.36, -0.12, 0.12] {
        p.rect(u - 0.01, 0.46, u + 0.01, 0.7, p.k.pale);
    }
    p.line((-0.36, 0.7), (0.12, 0.7), 0.014, p.k.pale);
}

/// A tall net loft: a loft door up high with a hoist beam, nets hung.
pub fn net_loft(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.3, 0.0, 0.3, 0.78, wall);
    p.pitched(-0.3, 0.3, 0.78, 0.98, roof);
    p.rect(-0.1, 0.52, 0.1, 0.72, shade(p.trim, -0.1));
    p.line((0.0, 0.82), (0.0, 0.76), 0.014, p.k.wood);
    p.line((-0.02, 0.82), (0.4, 0.82), 0.02, p.k.wood);
    p.line((0.36, 0.82), (0.36, 0.4), 0.006, p.k.dark);
    p.rect(-0.1, 0.0, 0.1, 0.28, shade(p.trim, -0.2));
    p.window(-0.24, 0.34, -0.14, 0.44);
    let net = shade(p.k.second, 0.3);
    p.poly(
        &[(0.3, 0.3), (0.46, 0.34), (0.44, 0.02), (0.3, 0.06)],
        net.opacity(0.6),
    );
    for step in 0..4 {
        let v = 0.08 + step as f32 * 0.07;
        p.line((0.3, v), (0.45, v + 0.02), 0.006, net);
    }
}

/// A book cabinet for the school yard: glazed doors on shelves of
/// books, a roof to keep off the rain.
pub fn book_nook(p: &mut Pen) {
    p.shadow(0.3);
    let wood = p.k.wood;
    p.rect(-0.28, 0.0, 0.28, 0.7, wood);
    p.rect(-0.24, 0.06, 0.24, 0.66, shade(wood, -0.35));
    for (row, v) in [0.08, 0.28, 0.48].iter().enumerate() {
        for step in 0..5 {
            let u = -0.22 + step as f32 * 0.09;
            let ink = [p.k.accent, p.k.second, p.k.brass, p.k.leaf][(step + row) % 4];
            p.rect(u, *v, u + 0.07, v + 0.15, ink);
        }
    }
    p.rect(-0.24, 0.06, 0.24, 0.66, p.glass.opacity(0.25));
    p.line((0.0, 0.06), (0.0, 0.66), 0.01, wood);
    p.gable(-0.36, 0.36, 0.7, 0.9, p.roof);
}

/// Tea rooms: a bow-fronted window of cakes, a striped awning, a teapot
/// sign, a table outside.
pub fn tea_rooms(p: &mut Pen) {
    let (wall, roof) = (p.k.pale, p.roof);
    p.rect(-0.44, 0.0, 0.44, 0.72, wall);
    p.rect(-0.46, 0.72, 0.46, 0.8, shade(roof, -0.15));
    p.dome(0.0, 0.8, 0.3, 0.14, shade(roof, -0.1));
    p.rect(-0.34, 0.14, 0.1, 0.44, p.glass);
    for u in [-0.26, -0.12, 0.02] {
        p.dome(u, 0.2, 0.05, 0.06, p.k.bloom);
    }
    for step in 0..7 {
        let u = -0.44 + step as f32 * 0.126;
        let ink = if step % 2 == 0 { roof } else { p.k.pale };
        p.poly(
            &[
                (u, 0.6),
                (u + 0.126, 0.6),
                (u + 0.14, 0.48),
                (u + 0.014, 0.48),
            ],
            ink,
        );
    }
    p.rect(0.18, 0.0, 0.34, 0.4, shade(p.trim, -0.1));
    p.line((0.4, 0.72), (0.4, 0.66), 0.01, p.k.dark);
    p.ell(0.4, 0.62, 0.05, 0.04, p.k.second);
    p.rect(0.44, 0.63, 0.47, 0.64, p.k.second);
}

/// A hand cart of loaves on two big wheels, a handle out front.
pub fn bread_cart(p: &mut Pen) {
    p.shadow(0.42);
    let wood = p.k.wood;
    p.rect(-0.36, 0.3, 0.28, 0.56, shade(wood, 0.05));
    for step in 0..4 {
        let u = -0.28 + step as f32 * 0.14;
        p.ell(u, 0.6, 0.07, 0.05, hex(0xd9a55a));
    }
    p.line((0.28, 0.42), (0.5, 0.52), 0.02, wood);
    p.circ(-0.12, 0.2, 0.18, shade(wood, -0.3));
    p.circ(-0.12, 0.2, 0.14, p.k.ground);
    for angle in 0..4 {
        let a = angle as f32 * std::f32::consts::FRAC_PI_4;
        p.line(
            (-0.12 - a.cos() * 0.14, 0.2 - a.sin() * 0.14 * p.w / p.h),
            (-0.12 + a.cos() * 0.14, 0.2 + a.sin() * 0.14 * p.w / p.h),
            0.01,
            shade(wood, -0.3),
        );
    }
    p.line((0.2, 0.3), (0.24, 0.0), 0.015, shade(wood, -0.2));
}

/// A terrace of tables under parasols on a raised deck.
pub fn pub_terrace(p: &mut Pen) {
    p.shadow(0.48);
    let wood = p.k.wood;
    p.rect(-0.48, 0.0, 0.48, 0.12, shade(wood, -0.1));
    for step in 0..9 {
        let u = -0.46 + step as f32 * 0.115;
        p.line((u, 0.12), (u, 0.3), 0.01, shade(wood, 0.1));
    }
    p.line((-0.48, 0.3), (0.48, 0.3), 0.014, shade(wood, 0.1));
    for (u, ink) in [(-0.22, p.k.accent), (0.22, p.k.second)] {
        p.line((u, 0.12), (u, 0.8), 0.012, p.k.pale);
        p.poly(&[(u - 0.2, 0.72), (u + 0.2, 0.72), (u, 0.9)], ink);
        p.rect(u - 0.12, 0.36, u + 0.12, 0.4, wood);
        p.rect(u - 0.1, 0.4, u - 0.07, 0.44, p.k.brass);
    }
}

/// A workshop: a saw-tooth roof, big double doors and a lamp.
pub fn workshop(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.46, 0.0, 0.46, 0.56, wall);
    for step in 0..3 {
        let u = -0.46 + step as f32 * 0.307;
        p.poly(
            &[
                (u, 0.56),
                (u + 0.307, 0.56),
                (u + 0.307, 0.88),
                (u + 0.2, 0.88),
            ],
            roof,
        );
        p.poly(
            &[
                (u + 0.2, 0.62),
                (u + 0.3, 0.62),
                (u + 0.3, 0.84),
                (u + 0.22, 0.84),
            ],
            p.glass,
        );
    }
    p.rect(-0.26, 0.0, 0.1, 0.42, shade(p.trim, -0.05));
    p.line((-0.08, 0.0), (-0.08, 0.42), 0.01, shade(p.trim, 0.3));
    p.line((-0.26, 0.0), (0.1, 0.42), 0.01, shade(p.trim, 0.3));
    p.window(0.2, 0.2, 0.38, 0.38);
}

/// A slatted bin store with a latched gate, gulls kept out.
pub fn bin_gate(p: &mut Pen) {
    p.shadow(0.46);
    let wood = p.k.wood;
    p.rect(-0.46, 0.0, 0.46, 0.62, shade(wood, -0.15));
    for step in 0..9 {
        let u = -0.44 + step as f32 * 0.11;
        p.rect(u, 0.02, u + 0.08, 0.6, shade(wood, 0.05));
    }
    p.poly(
        &[(-0.5, 0.62), (0.5, 0.62), (0.46, 0.74), (-0.46, 0.74)],
        p.roof,
    );
    p.rect(0.05, 0.3, 0.12, 0.36, p.k.metal);
    p.line((-0.4, 0.1), (0.0, 0.52), 0.02, shade(wood, 0.15));
}

/// A round paddling pool with a rim, a toy boat afloat.
pub fn paddling_pool(p: &mut Pen) {
    p.ell(0.0, 0.2, 0.48, 0.18, p.k.pale);
    p.ell(0.0, 0.22, 0.42, 0.13, shade(p.k.water, 0.25));
    p.ell(0.1, 0.26, 0.14, 0.02, shade(p.k.water, 0.5));
    p.poly(
        &[(-0.2, 0.3), (-0.04, 0.3), (-0.08, 0.24), (-0.18, 0.24)],
        p.k.accent,
    );
    p.line((-0.12, 0.3), (-0.12, 0.52), 0.01, p.k.dark);
    p.poly(&[(-0.12, 0.5), (-0.12, 0.34), (-0.02, 0.34)], p.k.pale);
    p.rect(0.3, 0.3, 0.38, 0.42, p.k.brass);
}

/// Lanterns on hooked posts along a low wall.
pub fn lantern_walk(p: &mut Pen) {
    let stone = p.k.stone;
    p.rect(-0.5, 0.0, 0.5, 0.2, stone);
    p.rect(-0.5, 0.2, 0.5, 0.24, shade(stone, 0.12));
    for u in [-0.36, 0.0, 0.36] {
        p.rect(u - 0.012, 0.24, u + 0.012, 0.86, p.k.dark);
        p.line((u, 0.86), (u + 0.08, 0.86), 0.012, p.k.dark);
        p.line((u + 0.08, 0.86), (u + 0.08, 0.8), 0.008, p.k.dark);
        p.rr(u + 0.04, 0.64, u + 0.12, 0.8, 0.02, p.glass);
        p.gable(u + 0.03, u + 0.13, 0.8, 0.84, p.k.dark);
    }
}

/// A long low shed for salting herring, barrels stacked at its end.
pub fn herring_shed(p: &mut Pen) {
    let wood = shade(p.k.wood, -0.05);
    p.rect(-0.46, 0.0, 0.22, 0.58, wood);
    for step in 0..7 {
        let u = -0.42 + step as f32 * 0.095;
        p.line((u, 0.0), (u, 0.58), 0.005, shade(wood, -0.25));
    }
    p.pitched(-0.46, 0.22, 0.58, 0.82, shade(p.k.second, -0.1));
    p.rect(-0.18, 0.0, -0.02, 0.36, p.k.dark);
    for (u, v) in [(0.3, 0.0), (0.42, 0.0), (0.36, 0.2)] {
        p.rr(u - 0.06, v, u + 0.06, v + 0.2, 0.03, shade(p.k.wood, 0.1));
        p.line((u - 0.06, v + 0.06), (u + 0.06, v + 0.06), 0.01, p.k.metal);
        p.line((u - 0.06, v + 0.14), (u + 0.06, v + 0.14), 0.01, p.k.metal);
    }
}

/// Woven hurdle panels staked in a line, a windbreak.
pub fn windbreak(p: &mut Pen) {
    p.shadow(0.48);
    let willow = hex(0xa88a58);
    for u in [-0.33, 0.0, 0.33] {
        p.rect(u - 0.16, 0.08, u + 0.16, 0.86, willow);
        for step in 0..9 {
            let v = 0.12 + step as f32 * 0.08;
            let off = if step % 2 == 0 { -0.01 } else { 0.01 };
            p.line(
                (u - 0.16 + off, v),
                (u + 0.16 + off, v),
                0.02,
                shade(willow, if step % 2 == 0 { -0.15 } else { 0.08 }),
            );
        }
        for side in [-0.16, 0.16] {
            p.rect(
                u + side - 0.012,
                0.0,
                u + side + 0.012,
                0.92,
                shade(p.k.wood, -0.2),
            );
        }
    }
}

/// A small chapel with a pointed window of coloured glass and a bell
/// gable.
pub fn chapel(p: &mut Pen) {
    let (wall, roof) = (p.k.stone, shade(p.k.second, -0.15));
    p.rect(-0.3, 0.0, 0.3, 0.52, wall);
    p.gable(-0.34, 0.34, 0.52, 0.82, roof);
    p.poly(
        &[
            (-0.1, 0.18),
            (0.1, 0.18),
            (0.1, 0.38),
            (0.0, 0.48),
            (-0.1, 0.38),
        ],
        shade(wall, -0.3),
    );
    let inks = [p.k.accent, p.k.second, p.k.brass, p.k.leaf];
    for row in 0..3 {
        for col in 0..2 {
            let (u, v) = (-0.08 + col as f32 * 0.08, 0.2 + row as f32 * 0.08);
            p.rect(u, v, u + 0.07, v + 0.07, inks[(row + col) % 4].opacity(0.9));
        }
    }
    p.rect(-0.07, 0.82, 0.07, 0.96, wall);
    p.circ(0.0, 0.88, 0.03, p.k.brass);
    p.gable(-0.09, 0.09, 0.96, 1.0, roof);
    p.arch(-0.06, 0.06, 0.0, 0.14, shade(p.trim, -0.2));
}

/// The end of a jetty, a ladder down into the water and a lifebuoy.
pub fn jetty_ladder(p: &mut Pen) {
    let wood = p.k.wood;
    p.ell(0.0, 0.04, 0.5, 0.06, p.k.water.opacity(0.6));
    p.rect(-0.5, 0.52, 0.2, 0.6, wood);
    for u in [-0.44, -0.14, 0.16] {
        p.rect(u - 0.025, 0.0, u + 0.025, 0.52, shade(wood, -0.25));
    }
    for u in [0.26, 0.36] {
        p.line((u, 0.8), (u, 0.02), 0.014, p.k.metal);
    }
    for step in 0..7 {
        let v = 0.08 + step as f32 * 0.1;
        p.line((0.26, v), (0.36, v), 0.01, p.k.metal);
    }
    p.curve((0.26, 0.8), (0.31, 0.9), (0.36, 0.8), 0.014, p.k.metal);
    p.rect(-0.38, 0.6, -0.34, 0.9, wood);
    p.circ(-0.36, 0.78, 0.09, p.k.accent);
    p.circ(-0.36, 0.78, 0.05, p.k.ground);
}

/// A bench of driftwood: a bleached log on two stones.
pub fn driftwood_bench(p: &mut Pen) {
    p.shadow(0.46);
    let drift = hex(0xb8ab98);
    for u in [-0.3, 0.3] {
        p.ell(u, 0.14, 0.12, 0.16, p.k.stone);
    }
    p.poly(
        &[
            (-0.48, 0.3),
            (-0.2, 0.36),
            (0.3, 0.34),
            (0.48, 0.28),
            (0.46, 0.44),
            (0.1, 0.5),
            (-0.46, 0.46),
        ],
        drift,
    );
    p.line((-0.3, 0.42), (0.2, 0.44), 0.008, shade(drift, -0.25));
    p.line((0.3, 0.46), (0.42, 0.62), 0.02, drift);
}

/// A rope swing on an old oak, its seat a round of wood.
pub fn oak_rope_swing(p: &mut Pen) {
    let wood = shade(p.k.wood, -0.1);
    p.poly(
        &[(-0.4, 0.0), (-0.2, 0.0), (-0.24, 0.72), (-0.36, 0.72)],
        wood,
    );
    p.line((-0.28, 0.66), (0.36, 0.76), 0.04, wood);
    for (u, v, r) in [
        (-0.3, 0.84, 0.22),
        (0.02, 0.9, 0.24),
        (0.3, 0.82, 0.18),
        (-0.1, 0.72, 0.16),
    ] {
        p.circ(u, v, r, shade(p.k.leaf, (u * 0.2).abs() - 0.05));
    }
    let swing = 0.04 * p.sway;
    p.line((0.2, 0.72), (0.2 + swing, 0.2), 0.012, hex(0xc9b48a));
    p.ell(0.2 + swing, 0.18, 0.08, 0.025, p.k.wood);
}

/// Painted pebbles in a row, each its own colour.
pub fn painted_stones(p: &mut Pen) {
    let inks = [p.k.accent, p.k.second, p.k.brass, p.k.leaf, p.k.bloom];
    for step in 0..7 {
        let u = -0.42 + step as f32 * 0.14;
        let r = 0.05 + 0.02 * ((step * 3) % 4) as f32;
        p.ell(u, 0.3 * r / 0.07, r, r * 3.0, inks[step % 5]);
        p.circ(u, 0.3 * r / 0.07 + 0.1, r * 0.3, p.k.pale);
    }
}

/// A notice board on two legs under a little roof, papers pinned.
pub fn notice_board(p: &mut Pen) {
    p.shadow(0.4);
    let wood = p.k.wood;
    for u in [-0.36, 0.36] {
        p.rect(u - 0.025, 0.0, u + 0.025, 0.86, wood);
    }
    p.rect(-0.38, 0.34, 0.38, 0.84, shade(wood, 0.1));
    p.rect(-0.34, 0.38, 0.34, 0.8, hex(0x9a7a50));
    for (u, v, ink) in [
        (-0.24, 0.6, p.k.pale),
        (0.0, 0.66, hex(0xf2e6a8)),
        (0.2, 0.5, p.k.pale),
        (-0.1, 0.44, hex(0xd8e8f0)),
    ] {
        p.rect(u - 0.08, v - 0.1, u + 0.08, v + 0.1, ink);
        p.circ(u, v + 0.08, 0.012, p.k.accent);
    }
    p.gable(-0.46, 0.46, 0.84, 1.0, p.roof);
}

/// A raised timber bed of herbs behind the bakery.
pub fn herb_bed(p: &mut Pen) {
    p.shadow(0.48);
    let wood = p.k.wood;
    p.rect(-0.46, 0.0, 0.46, 0.4, wood);
    p.line((-0.46, 0.2), (0.46, 0.2), 0.008, shade(wood, -0.25));
    for step in 0..6 {
        let u = -0.38 + step as f32 * 0.15;
        let tone = if step % 2 == 0 { 0.1 } else { -0.1 };
        for leaf in 0..3 {
            let a = u - 0.04 + leaf as f32 * 0.04;
            p.ell(
                a,
                0.5 + leaf as f32 * 0.04,
                0.03,
                0.12,
                shade(p.k.leaf, tone),
            );
        }
    }
    p.circ(0.3, 0.72, 0.03, hex(0xb18ad8));
    p.circ(-0.2, 0.66, 0.03, hex(0xb18ad8));
}

/// A cairn: stones piled into a cone on the point.
pub fn cairn(p: &mut Pen) {
    p.shadow(0.4);
    let stone = p.k.stone;
    let rows: [&[f32]; 5] = [
        &[-0.3, -0.1, 0.1, 0.3],
        &[-0.2, 0.0, 0.2],
        &[-0.1, 0.1],
        &[0.0],
        &[0.02],
    ];
    for (row, stones) in rows.iter().enumerate() {
        let v = 0.08 + row as f32 * 0.18;
        let r = 0.12 - row as f32 * 0.015;
        for (index, u) in stones.iter().enumerate() {
            let tone = ((row + index) % 3) as f32 * 0.08 - 0.08;
            p.ell(*u, v, r, 0.09, shade(stone, tone));
        }
    }
}

/// Window boxes on a stand: two tiers of flowers against a trellis.
pub fn window_box_stand(p: &mut Pen) {
    p.shadow(0.42);
    let white = p.k.pale;
    p.rect(-0.4, 0.0, 0.4, 0.8, white.opacity(0.35));
    for step in 0..5 {
        let u = -0.4 + step as f32 * 0.2;
        p.line((u, 0.0), (u + 0.2, 0.8), 0.008, white);
        p.line((u + 0.2, 0.0), (u, 0.8), 0.008, white);
    }
    for v in [0.14, 0.5] {
        p.rect(-0.42, v, 0.42, v + 0.12, p.k.second);
        p.flowers(-0.38, 0.38, v + 0.12, 8, 0.04);
    }
}

/// A tarred bait shed, tiny, with buckets at its door.
pub fn bait_shed(p: &mut Pen) {
    let tar = shade(p.k.dark, 0.15);
    p.rect(-0.26, 0.0, 0.2, 0.6, tar);
    p.poly(
        &[(-0.3, 0.6), (0.24, 0.72), (0.24, 0.64), (-0.3, 0.54)],
        shade(p.k.stone, -0.2),
    );
    p.rect(-0.14, 0.0, 0.04, 0.4, shade(tar, 0.2));
    p.rect(-0.12, 0.44, 0.02, 0.5, p.k.accent);
    for (u, ink) in [(0.3, p.k.second), (0.42, p.k.accent)] {
        p.poly(
            &[
                (u - 0.06, 0.14),
                (u + 0.06, 0.14),
                (u + 0.05, 0.0),
                (u - 0.05, 0.0),
            ],
            ink,
        );
        p.curve(
            (u - 0.06, 0.14),
            (u, 0.22),
            (u + 0.06, 0.14),
            0.006,
            p.k.metal,
        );
    }
}

/// A square sandpit, a bucket and spade in it.
pub fn sandpit(p: &mut Pen) {
    let wood = p.k.wood;
    p.rect(-0.46, 0.0, 0.46, 0.3, wood);
    p.rect(-0.42, 0.24, 0.42, 0.3, hex(0xe6d3a8));
    p.dome(-0.1, 0.3, 0.12, 0.12, hex(0xe6d3a8));
    p.poly(
        &[(0.12, 0.3), (0.24, 0.3), (0.22, 0.5), (0.14, 0.5)],
        p.k.accent,
    );
    p.line((-0.3, 0.3), (-0.36, 0.7), 0.014, p.k.second);
    p.poly(
        &[(-0.4, 0.26), (-0.28, 0.26), (-0.3, 0.36), (-0.38, 0.36)],
        p.k.second,
    );
}

/// A bird table: a tray on a post under a little roof, a robin on it.
pub fn bird_table(p: &mut Pen) {
    p.shadow(0.24);
    let wood = p.k.wood;
    p.line((-0.14, 0.0), (0.0, 0.2), 0.02, wood);
    p.line((0.14, 0.0), (0.0, 0.2), 0.02, wood);
    p.rect(-0.025, 0.0, 0.025, 0.66, wood);
    p.rect(-0.22, 0.66, 0.22, 0.7, shade(wood, 0.1));
    for u in [-0.18, 0.18] {
        p.rect(u - 0.012, 0.7, u + 0.012, 0.84, wood);
    }
    p.gable(-0.26, 0.26, 0.84, 0.96, p.roof);
    p.ell(0.06, 0.74, 0.05, 0.035, shade(p.k.wood, -0.1));
    p.circ(0.1, 0.77, 0.025, shade(p.k.wood, -0.1));
    p.ell(0.08, 0.73, 0.025, 0.02, p.k.accent);
}

/// Stepping stones across a stream.
pub fn stepping_stones(p: &mut Pen) {
    p.ell(0.0, 0.3, 0.5, 0.2, p.k.water.opacity(0.8));
    for (index, u) in [-0.36, -0.14, 0.08, 0.3].iter().enumerate() {
        let v = 0.2 + (index % 2) as f32 * 0.14;
        p.ell(*u, v, 0.09, 0.12, p.k.stone);
        p.ell(*u, v + 0.03, 0.07, 0.06, shade(p.k.stone, 0.15));
    }
}

/// A book swap box on a post: a little house with a glass door.
pub fn book_box(p: &mut Pen) {
    p.shadow(0.2);
    let wood = p.k.wood;
    p.rect(-0.03, 0.0, 0.03, 0.5, wood);
    p.rect(-0.2, 0.5, 0.2, 0.84, p.roof);
    p.rect(-0.16, 0.54, 0.16, 0.8, shade(wood, -0.3));
    for (step, ink) in [p.k.accent, p.k.pale, p.k.brass, p.k.second]
        .iter()
        .enumerate()
    {
        let u = -0.14 + step as f32 * 0.07;
        p.rect(u, 0.56, u + 0.05, 0.68, *ink);
    }
    p.rect(-0.16, 0.54, 0.16, 0.8, p.glass.opacity(0.3));
    p.gable(-0.26, 0.26, 0.84, 1.0, shade(p.roof, -0.2));
}

/// A line of flags from a tall pole down to a short post.
pub fn flag_line(p: &mut Pen) {
    p.shadow(0.4);
    let wood = p.k.wood;
    p.rect(-0.44, 0.0, -0.4, 0.98, wood);
    p.rect(0.4, 0.0, 0.44, 0.4, wood);
    let inks = [p.k.accent, p.k.pale, p.k.second, p.k.brass];
    p.pennants((-0.42, 0.95), (0.42, 0.38), 7, &inks);
}

/// A drying rack of poles with nets draped over it.
pub fn net_rack(p: &mut Pen) {
    p.shadow(0.46);
    let wood = p.k.wood;
    for u in [-0.4, 0.0, 0.4] {
        p.rect(u - 0.018, 0.0, u + 0.018, 0.82, wood);
    }
    p.line((-0.46, 0.8), (0.46, 0.8), 0.02, wood);
    let net = shade(p.k.second, 0.25);
    p.poly(
        &[(-0.4, 0.8), (0.0, 0.8), (-0.04, 0.22), (-0.36, 0.3)],
        net.opacity(0.55),
    );
    p.poly(
        &[(0.0, 0.8), (0.4, 0.8), (0.36, 0.36), (0.06, 0.28)],
        hex(0x9a6a4a).opacity(0.55),
    );
    for step in 0..5 {
        let v = 0.34 + step as f32 * 0.1;
        p.line((-0.38, v), (-0.02, v - 0.02), 0.006, net);
        p.line((0.04, v), (0.38, v + 0.02), 0.006, hex(0x9a6a4a));
    }
    p.circ(-0.2, 0.3, 0.03, p.k.brass);
}

/// Wildflowers, tall and mixed, along a path.
pub fn wildflowers(p: &mut Pen) {
    let leaf = p.k.leaf;
    let inks = [
        hex(0xe86a5a),
        hex(0xf2d06b),
        p.k.pale,
        hex(0x9a8ad8),
        p.k.bloom,
    ];
    for step in 0..14 {
        let u = -0.46 + step as f32 * 0.07;
        let tall = 0.4 + 0.5 * (((step * 37) % 11) as f32 / 11.0);
        p.line((u, 0.0), (u + 0.02, tall), 0.012, leaf);
        p.circ(u + 0.02, tall, 0.03, inks[step % 5]);
    }
}

/// A fire pit: a ring of stones, logs crossed, flames up.
pub fn fire_pit(p: &mut Pen) {
    let stone = p.k.stone;
    p.ell(0.0, 0.1, 0.42, 0.1, shade(stone, -0.3));
    for step in 0..7 {
        let u = -0.36 + step as f32 * 0.12;
        p.ell(u, 0.12, 0.07, 0.1, shade(stone, (step % 3) as f32 * 0.07));
    }
    p.line((-0.2, 0.14), (0.16, 0.34), 0.04, shade(p.k.wood, -0.2));
    p.line((0.2, 0.14), (-0.14, 0.34), 0.04, shade(p.k.wood, -0.3));
    p.poly(
        &[
            (-0.14, 0.2),
            (-0.06, 0.62),
            (0.0, 0.4),
            (0.06, 0.8),
            (0.14, 0.2),
        ],
        hex(0xf29a3c),
    );
    p.poly(&[(-0.06, 0.2), (0.0, 0.48), (0.06, 0.2)], hex(0xffd27a));
}

/// A hen house on legs with a ramp, a hen at the door.
pub fn hen_house(p: &mut Pen) {
    let wood = p.k.wood;
    for u in [-0.3, 0.3] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.24, shade(wood, -0.2));
    }
    p.rect(-0.36, 0.24, 0.36, 0.64, p.wall);
    p.pitched(-0.36, 0.36, 0.64, 0.9, p.roof);
    p.rect(-0.08, 0.26, 0.08, 0.46, p.k.dark);
    p.line((0.0, 0.26), (0.42, 0.0), 0.04, shade(wood, 0.1));
    p.ell(0.4, 0.1, 0.06, 0.05, hex(0xb8643e));
    p.circ(0.44, 0.17, 0.025, hex(0xb8643e));
    p.circ(0.45, 0.2, 0.012, p.k.accent);
    p.window(-0.28, 0.4, -0.16, 0.52);
}

/// A high seat on legs, like a lifeguard's chair, looking out.
pub fn lookout_seat(p: &mut Pen) {
    p.shadow(0.34);
    let (white, wood) = (p.k.pale, p.k.wood);
    p.line((-0.3, 0.0), (-0.16, 0.6), 0.03, white);
    p.line((0.3, 0.0), (0.16, 0.6), 0.03, white);
    p.line((-0.24, 0.3), (0.24, 0.3), 0.02, white);
    for step in 0..5 {
        let v = 0.08 + step as f32 * 0.11;
        p.line((0.24, v), (0.36, v - 0.02), 0.012, wood);
    }
    p.rect(-0.2, 0.6, 0.2, 0.66, wood);
    p.rect(-0.2, 0.66, -0.16, 0.92, wood);
    p.rect(-0.2, 0.84, 0.1, 0.88, wood);
    p.poly(
        &[(-0.26, 0.96), (0.26, 0.96), (0.2, 0.9), (-0.2, 0.9)],
        p.k.accent,
    );
}

/// Jam jars hung from a shepherd's crook, candles in them.
pub fn jar_lanterns(p: &mut Pen) {
    p.shadow(0.2);
    let iron = p.k.dark;
    p.rect(-0.02, 0.0, 0.02, 0.86, iron);
    p.curve((0.0, 0.86), (0.2, 1.0), (0.24, 0.82), 0.02, iron);
    p.curve((0.0, 0.86), (-0.2, 1.0), (-0.24, 0.82), 0.02, iron);
    for u in [-0.24, 0.24] {
        p.line((u, 0.82), (u, 0.72), 0.006, iron);
        p.rr(u - 0.07, 0.52, u + 0.07, 0.72, 0.02, p.glass.opacity(0.85));
        p.rect(u - 0.075, 0.7, u + 0.075, 0.73, p.k.metal);
        p.circ(u, 0.58, 0.02, hex(0xffd27a));
    }
}

/// A path of shells: a pale strip edged in scallops.
pub fn shell_path(p: &mut Pen) {
    p.poly(
        &[(-0.5, 0.0), (0.5, 0.0), (0.4, 0.5), (-0.4, 0.5)],
        hex(0xe8dcc8),
    );
    for step in 0..8 {
        let u = -0.46 + step as f32 * 0.13;
        p.dome(u, 0.0, 0.05, 0.3, p.k.pale);
        p.dome(u * 0.82, 0.5, 0.04, 0.24, hex(0xf2d8cc));
    }
    for step in 0..6 {
        let u = -0.3 + step as f32 * 0.12;
        p.circ(u, 0.26, 0.02, hex(0xd8c0a8));
    }
}

/// An old boat on the green full of flowers.
pub fn boat_planter(p: &mut Pen) {
    p.shadow(0.48);
    let paint = p.k.second;
    p.flowers(-0.38, 0.38, 0.5, 11, 0.045);
    p.poly(
        &[(-0.5, 0.6), (0.5, 0.6), (0.36, 0.08), (-0.32, 0.08)],
        paint,
    );
    p.poly(
        &[(-0.48, 0.6), (0.48, 0.6), (0.46, 0.52), (-0.46, 0.52)],
        p.k.pale,
    );
    p.line((-0.34, 0.3), (0.34, 0.3), 0.006, shade(paint, -0.25));
}

/// A skittle alley: a board lane, nine pins at its end and a ball.
pub fn skittle_alley(p: &mut Pen) {
    p.poly(
        &[(-0.5, 0.0), (0.5, 0.0), (0.36, 0.3), (-0.36, 0.3)],
        shade(p.k.wood, 0.15),
    );
    for row in 0..3 {
        for col in 0..(3 - row) {
            let u = 0.18 + col as f32 * 0.08 + row as f32 * 0.04;
            let v = 0.22 + row as f32 * 0.06;
            p.ell(u, v + 0.14, 0.025, 0.14, p.k.pale);
            p.rect(u - 0.025, v + 0.18, u + 0.025, v + 0.2, p.k.accent);
        }
    }
    p.circ(-0.3, 0.2, 0.08, p.k.dark);
}

/// A weathervane: a fish on an arrow over the four points.
pub fn weathervane(p: &mut Pen) {
    p.shadow(0.2);
    let iron = p.k.dark;
    p.rect(-0.02, 0.0, 0.02, 0.8, iron);
    p.line((-0.2, 0.62), (0.2, 0.62), 0.012, iron);
    p.line((0.0, 0.54), (0.0, 0.7), 0.012, iron);
    for (u, v) in [(-0.24, 0.62), (0.24, 0.62), (0.0, 0.52), (0.0, 0.73)] {
        p.circ(u, v, 0.02, p.k.brass);
    }
    p.line((-0.36, 0.84), (0.36, 0.84), 0.014, iron);
    p.poly(&[(0.36, 0.84), (0.28, 0.88), (0.28, 0.8)], iron);
    p.ell(-0.1, 0.9, 0.16, 0.06, p.k.brass);
    p.poly(&[(-0.26, 0.9), (-0.36, 0.96), (-0.36, 0.84)], p.k.brass);
    p.circ(0.0, 0.91, 0.015, iron);
}

/// A big carved storytelling chair with a high back.
pub fn story_chair(p: &mut Pen) {
    p.shadow(0.36);
    let wood = shade(p.k.wood, 0.05);
    p.rect(-0.3, 0.0, -0.24, 0.42, wood);
    p.rect(0.24, 0.0, 0.3, 0.42, wood);
    p.rect(-0.32, 0.38, 0.32, 0.46, wood);
    p.rect(-0.28, 0.46, 0.28, 0.9, shade(wood, -0.1));
    p.dome(0.0, 0.9, 0.28, 0.1, shade(wood, -0.1));
    p.circ(0.0, 0.8, 0.07, p.k.brass);
    p.poly(
        &[(0.0, 0.86), (0.05, 0.8), (0.0, 0.74), (-0.05, 0.8)],
        shade(wood, -0.3),
    );
    p.rect(-0.32, 0.46, -0.28, 0.62, wood);
    p.rect(0.28, 0.46, 0.32, 0.62, wood);
    p.rect(-0.34, 0.6, -0.24, 0.64, wood);
    p.rect(0.24, 0.6, 0.34, 0.64, wood);
    p.rect(-0.26, 0.46, 0.26, 0.5, p.k.accent);
}

/// Kites on long strings, flying from a stake.
pub fn kites(p: &mut Pen) {
    p.rect(-0.38, 0.0, -0.34, 0.1, p.k.wood);
    for (u, v, ink) in [
        (0.1, 0.9, p.k.accent),
        (0.34, 0.7, p.k.second),
        (-0.04, 0.6, p.k.brass),
    ] {
        p.curve(
            (-0.36, 0.1),
            (u - 0.2, v - 0.3),
            (u, v - 0.08),
            0.006,
            p.k.dark,
        );
        p.poly(
            &[
                (u, v),
                (u + 0.07, v - 0.08),
                (u, v - 0.18),
                (u - 0.07, v - 0.08),
            ],
            ink,
        );
        p.line((u, v), (u, v - 0.18), 0.006, shade(ink, -0.3));
        p.curve(
            (u, v - 0.18),
            (u + 0.05, v - 0.26),
            (u - 0.02, v - 0.32),
            0.006,
            p.k.dark,
        );
    }
}

/// Little painted name boards on stakes among rocks.
pub fn pool_name_boards(p: &mut Pen) {
    let stone = p.k.stone;
    for (u, r) in [(-0.3, 0.14), (0.1, 0.18), (0.38, 0.1)] {
        p.ell(u, 0.08, r, 0.12, shade(stone, -0.1));
    }
    for (u, v, ink) in [(-0.2, 0.62, p.k.second), (0.26, 0.72, p.k.accent)] {
        p.rect(u - 0.012, 0.0, u + 0.012, v, p.k.wood);
        p.rect(u - 0.14, v - 0.02, u + 0.14, v + 0.2, ink);
        p.line((u - 0.1, v + 0.09), (u + 0.1, v + 0.09), 0.012, p.k.pale);
    }
}

/// A round pebble mosaic of a fish, set in the quay.
pub fn pebble_mosaic(p: &mut Pen) {
    p.ell(0.0, 0.5, 0.48, 0.44, p.k.stone);
    p.ell(0.0, 0.5, 0.42, 0.36, shade(p.k.stone, 0.15));
    p.ell(-0.04, 0.5, 0.22, 0.16, p.k.second);
    p.poly(&[(0.16, 0.5), (0.3, 0.66), (0.3, 0.34)], p.k.second);
    p.circ(-0.16, 0.54, 0.025, p.k.pale);
    for step in 0..12 {
        let a = step as f32 * std::f32::consts::TAU / 12.0;
        p.circ(a.cos() * 0.44, 0.5 + a.sin() * 0.4, 0.02, p.k.accent);
    }
}

/// A band's practice shed: a round window, a big drum by the door.
pub fn music_shed(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.34, 0.0, 0.26, 0.6, wall);
    p.pitched(-0.34, 0.26, 0.6, 0.86, roof);
    p.porthole(-0.04, 0.72, 0.06);
    p.rect(-0.26, 0.0, -0.08, 0.4, shade(p.trim, -0.1));
    p.window(0.04, 0.26, 0.18, 0.42);
    p.rr(0.28, 0.0, 0.5, 0.26, 0.04, p.k.accent);
    p.ell(0.39, 0.26, 0.11, 0.03, p.k.pale);
    p.line((0.28, 0.02), (0.5, 0.24), 0.008, p.k.brass);
    p.line((0.5, 0.02), (0.28, 0.24), 0.008, p.k.brass);
}

/// An apple press: a screw on a timber frame over a slatted basket.
pub fn apple_press(p: &mut Pen) {
    p.shadow(0.4);
    let wood = p.k.wood;
    p.rect(-0.36, 0.0, 0.36, 0.14, shade(wood, -0.1));
    for u in [-0.3, 0.3] {
        p.rect(u - 0.03, 0.14, u + 0.03, 0.86, wood);
    }
    p.rect(-0.36, 0.78, 0.36, 0.86, wood);
    p.rect(-0.02, 0.5, 0.02, 0.96, p.k.metal);
    p.line((-0.14, 0.94), (0.14, 0.94), 0.02, p.k.metal);
    p.rect(-0.18, 0.46, 0.18, 0.5, shade(wood, 0.1));
    p.rect(-0.18, 0.14, 0.18, 0.46, shade(wood, -0.2));
    for step in 0..5 {
        let u = -0.16 + step as f32 * 0.08;
        p.rect(u, 0.16, u + 0.05, 0.44, shade(wood, 0.1));
    }
    p.circ(0.46, 0.06, 0.05, p.k.accent);
    p.circ(0.36, 0.2, 0.0, p.k.accent);
}

/// A sail loft: tall, with a wide loft door where a sail hangs out.
pub fn sail_loft(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.36, 0.0, 0.36, 0.76, wall);
    p.pitched(-0.36, 0.36, 0.76, 0.98, roof);
    p.rect(-0.2, 0.46, 0.2, 0.7, shade(p.trim, -0.15));
    p.poly(
        &[(-0.18, 0.7), (0.26, 0.66), (0.22, 0.3), (-0.14, 0.44)],
        p.k.pale,
    );
    p.line((-0.18, 0.7), (-0.14, 0.44), 0.008, shade(p.k.pale, -0.2));
    p.rect(-0.08, 0.0, 0.08, 0.3, shade(p.trim, -0.1));
    p.window(-0.3, 0.12, -0.18, 0.26);
    p.window(0.18, 0.12, 0.3, 0.26);
}

/// A crab shack: a lean-to shop with a crab sign and pots stacked.
pub fn crab_shack(p: &mut Pen) {
    let wood = shade(p.k.wood, 0.1);
    p.rect(-0.4, 0.0, 0.2, 0.56, wood);
    for step in 0..5 {
        let v = 0.08 + step as f32 * 0.1;
        p.line((-0.4, v), (0.2, v), 0.005, shade(wood, -0.25));
    }
    p.poly(
        &[(-0.46, 0.56), (0.26, 0.7), (0.26, 0.64), (-0.46, 0.5)],
        p.roof,
    );
    p.rect(-0.3, 0.24, 0.1, 0.42, p.glass);
    p.rect(-0.34, 0.2, 0.14, 0.24, shade(wood, -0.2));
    p.ell(-0.1, 0.78, 0.1, 0.06, p.k.accent);
    for side in [-1.0_f32, 1.0] {
        p.line(
            (-0.1 + side * 0.08, 0.8),
            (-0.1 + side * 0.16, 0.86),
            0.012,
            p.k.accent,
        );
    }
    for (u, v) in [(0.32, 0.0), (0.44, 0.0), (0.38, 0.14)] {
        p.dome(u, v, 0.06, 0.14, shade(p.k.second, 0.2));
        p.line((u - 0.06, v + 0.04), (u + 0.06, v + 0.04), 0.006, p.k.dark);
    }
}

/// A low stone net store with a round window and nets on hooks.
pub fn net_store(p: &mut Pen) {
    let stone = p.k.stone;
    p.rect(-0.42, 0.0, 0.42, 0.54, stone);
    for v in [0.14, 0.28, 0.42] {
        p.line((-0.42, v), (0.42, v), 0.005, shade(stone, -0.2));
    }
    p.pitched(-0.42, 0.42, 0.54, 0.84, shade(p.k.second, -0.2));
    p.porthole(0.0, 0.66, 0.05);
    p.arch(-0.1, 0.1, 0.0, 0.34, shade(p.trim, -0.1));
    for u in [-0.3, 0.3] {
        p.circ(u, 0.44, 0.012, p.k.dark);
        p.poly(
            &[
                (u - 0.06, 0.44),
                (u + 0.06, 0.44),
                (u + 0.04, 0.14),
                (u - 0.04, 0.14),
            ],
            shade(p.k.second, 0.3).opacity(0.6),
        );
    }
}

/// A radio hut with a tall aerial mast and its wire.
pub fn radio_hut(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.3, 0.0, 0.2, 0.44, wall);
    p.poly(
        &[(-0.34, 0.44), (0.24, 0.52), (0.24, 0.46), (-0.34, 0.4)],
        roof,
    );
    p.window(-0.22, 0.2, -0.04, 0.34);
    p.rect(0.04, 0.0, 0.16, 0.3, shade(p.trim, -0.1));
    let metal = p.k.metal;
    p.rect(0.34, 0.0, 0.37, 0.98, metal);
    for step in 0..6 {
        let v = 0.1 + step as f32 * 0.15;
        p.line((0.3, v), (0.41, v + 0.08), 0.006, metal);
    }
    p.curve((0.2, 0.5), (0.28, 0.8), (0.35, 0.96), 0.006, p.k.dark);
    p.circ(0.355, 0.99, 0.02, p.k.accent);
}

/// A slipway: a stone ramp into the water with a winch at its head.
pub fn slipway(p: &mut Pen) {
    p.ell(0.2, 0.05, 0.32, 0.07, p.k.water.opacity(0.7));
    p.poly(
        &[(-0.5, 0.4), (-0.2, 0.4), (0.5, 0.0), (0.1, 0.0)],
        p.k.stone,
    );
    for u in [-0.35, -0.25] {
        p.line((u, 0.4), (u + 0.62, 0.02), 0.012, p.k.metal);
    }
    p.rect(-0.46, 0.4, -0.3, 0.62, p.k.metal);
    p.circ(-0.38, 0.54, 0.06, shade(p.k.metal, 0.2));
    p.line((-0.38, 0.54), (-0.26, 0.64), 0.014, p.k.dark);
}

/// A lean-to shelter of planks, nets hung and a brazier warm.
pub fn fishers_shelter(p: &mut Pen) {
    p.shadow(0.46);
    let wood = p.k.wood;
    p.rect(-0.44, 0.0, -0.38, 0.8, wood);
    p.rect(0.3, 0.0, 0.36, 0.56, wood);
    p.poly(
        &[(-0.5, 0.82), (0.46, 0.58), (0.46, 0.52), (-0.5, 0.76)],
        shade(p.k.stone, -0.2),
    );
    p.rect(-0.44, 0.0, -0.1, 0.78, shade(wood, -0.2));
    p.poly(
        &[(-0.38, 0.72), (-0.12, 0.66), (-0.14, 0.2), (-0.36, 0.3)],
        shade(p.k.second, 0.3).opacity(0.6),
    );
    p.rect(0.02, 0.0, 0.06, 0.16, p.k.dark);
    p.poly(
        &[(-0.04, 0.16), (0.12, 0.16), (0.1, 0.26), (-0.02, 0.26)],
        p.k.dark,
    );
    p.poly(
        &[
            (0.0, 0.26),
            (0.03, 0.36),
            (0.05, 0.3),
            (0.08, 0.38),
            (0.1, 0.26),
        ],
        hex(0xf29a3c),
    );
}

/// A tall harbour lamp: a twin-armed iron standard, ladder bar and all.
pub fn harbour_lamp(p: &mut Pen) {
    p.shadow(0.24);
    let iron = p.k.second;
    p.rect(-0.12, 0.0, 0.12, 0.08, iron);
    p.poly(
        &[(-0.06, 0.08), (0.06, 0.08), (0.03, 0.78), (-0.03, 0.78)],
        iron,
    );
    p.line((-0.14, 0.7), (0.14, 0.7), 0.014, iron);
    for side in [-1.0_f32, 1.0] {
        p.curve(
            (0.0, 0.8),
            (side * 0.2, 0.86),
            (side * 0.3, 0.8),
            0.016,
            iron,
        );
        p.poly(
            &[
                (side * 0.3 - 0.07, 0.8),
                (side * 0.3 + 0.07, 0.8),
                (side * 0.3 + 0.05, 0.66),
                (side * 0.3 - 0.05, 0.66),
            ],
            p.glass,
        );
        p.gable(side * 0.3 - 0.08, side * 0.3 + 0.08, 0.8, 0.86, iron);
    }
    p.circ(0.0, 0.88, 0.04, p.k.brass);
}

/// A rack of dinghies stacked upside down.
pub fn boat_rack(p: &mut Pen) {
    p.shadow(0.48);
    let wood = p.k.wood;
    for u in [-0.4, 0.4] {
        p.rect(u - 0.02, 0.0, u + 0.02, 0.9, wood);
    }
    for (v, ink) in [(0.12, p.roof), (0.42, p.k.second), (0.72, p.k.pale)] {
        p.line((-0.42, v), (0.42, v), 0.014, wood);
        p.poly(
            &[
                (-0.44, v + 0.02),
                (0.44, v + 0.02),
                (0.34, v + 0.2),
                (-0.34, v + 0.2),
            ],
            ink,
        );
    }
}

/// An ice house: a stone egg half sunk in the bank, a small door.
pub fn ice_house(p: &mut Pen) {
    let stone = p.k.stone;
    p.dome(0.0, 0.0, 0.46, 0.4, shade(p.k.leaf, -0.1));
    p.dome(0.06, 0.1, 0.3, 0.9, stone);
    for v in [0.3, 0.5, 0.7] {
        p.curve(
            (-0.2, v),
            (0.06, v + 0.05),
            (0.32, v),
            0.006,
            shade(stone, -0.2),
        );
    }
    p.rect(-0.26, 0.0, -0.08, 0.1, shade(stone, -0.1));
    p.arch(-0.02, 0.14, 0.1, 0.36, shade(p.trim, -0.2));
}

/// A bench between two mooring bollards, a rope looped round one.
pub fn mooring_bench(p: &mut Pen) {
    p.shadow(0.46);
    let (iron, wood) = (p.k.dark, p.k.wood);
    for u in [-0.4, 0.4] {
        p.rr(u - 0.08, 0.0, u + 0.08, 0.5, 0.04, iron);
        p.ell(u, 0.5, 0.1, 0.06, iron);
    }
    p.rect(-0.32, 0.3, 0.32, 0.38, wood);
    p.rect(-0.32, 0.5, 0.32, 0.6, shade(wood, 0.1));
    p.rect(-0.3, 0.38, -0.26, 0.5, shade(wood, -0.2));
    p.rect(0.26, 0.38, 0.3, 0.5, shade(wood, -0.2));
    p.curve((0.32, 0.3), (0.4, 0.12), (0.48, 0.3), 0.02, hex(0xc9b48a));
}

/// A bookshop: a green front, books in the bow window, a hanging sign.
pub fn bookshop(p: &mut Pen) {
    let (wall, green) = (p.wall, hex(0x2f5a48));
    p.rect(-0.42, 0.0, 0.42, 0.84, wall);
    p.pitched(-0.42, 0.42, 0.84, 1.0, p.roof);
    p.rect(-0.42, 0.0, 0.42, 0.52, green);
    p.rect(-0.4, 0.44, 0.4, 0.5, shade(green, -0.2));
    p.rect(-0.34, 0.1, 0.06, 0.4, p.glass);
    for step in 0..6 {
        let u = -0.32 + step as f32 * 0.065;
        let ink = [p.k.accent, p.k.brass, p.k.pale][step % 3];
        p.rect(u, 0.12, u + 0.05, 0.22 + 0.03 * (step % 2) as f32, ink);
    }
    p.rect(0.14, 0.0, 0.3, 0.38, shade(green, -0.3));
    p.window(-0.28, 0.6, -0.12, 0.74);
    p.window(0.12, 0.6, 0.28, 0.74);
    p.line((0.42, 0.66), (0.54, 0.66), 0.012, p.k.dark);
    p.rect(0.46, 0.5, 0.54, 0.64, p.k.brass);
}

/// A cheese shop: a yellow-striped awning and wheels of cheese in the
/// window.
pub fn cheese_shop(p: &mut Pen) {
    let wall = p.wall;
    p.rect(-0.42, 0.0, 0.42, 0.84, wall);
    p.rect(-0.44, 0.84, 0.44, 0.9, shade(wall, -0.2));
    p.rect(-0.36, 0.1, 0.1, 0.44, p.glass);
    for (u, v, r) in [(-0.24, 0.16, 0.07), (-0.06, 0.16, 0.07), (-0.15, 0.3, 0.06)] {
        p.ell(u, v, r, 0.05, p.k.brass);
        p.rect(u - r, v - 0.05, u + r, v, shade(p.k.brass, 0.2));
    }
    for step in 0..7 {
        let u = -0.44 + step as f32 * 0.126;
        let ink = if step % 2 == 0 {
            hex(0xe8c14e)
        } else {
            p.k.pale
        };
        p.poly(
            &[
                (u, 0.62),
                (u + 0.126, 0.62),
                (u + 0.14, 0.5),
                (u + 0.014, 0.5),
            ],
            ink,
        );
    }
    p.rect(0.18, 0.0, 0.32, 0.4, shade(p.trim, -0.1));
    p.window(-0.3, 0.68, -0.14, 0.8);
    p.window(0.14, 0.68, 0.3, 0.8);
}

/// A doctor's surgery: a neat house with a blue lamp over the door and a
/// brass plate.
pub fn surgery(p: &mut Pen) {
    let (wall, roof) = (p.k.pale, p.roof);
    p.rect(-0.4, 0.0, 0.4, 0.74, wall);
    p.poly(
        &[(-0.44, 0.74), (-0.32, 0.92), (0.32, 0.92), (0.44, 0.74)],
        roof,
    );
    for u in [-0.26, 0.26] {
        p.window(u - 0.07, 0.46, u + 0.07, 0.64);
        p.window(u - 0.07, 0.12, u + 0.07, 0.3);
    }
    p.arch(-0.07, 0.07, 0.0, 0.3, p.k.second);
    p.rect(-0.1, 0.34, 0.1, 0.38, p.k.dark);
    p.rr(-0.04, 0.38, 0.04, 0.46, 0.02, hex(0x6fa8dc));
    p.rect(0.1, 0.16, 0.16, 0.22, p.k.brass);
}

/// A pottery: a shop with a bottle kiln beside it, smoke from its neck.
pub fn pottery(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.46, 0.0, 0.04, 0.56, wall);
    p.pitched(-0.46, 0.04, 0.56, 0.76, roof);
    p.rect(-0.36, 0.12, -0.12, 0.36, p.glass);
    for u in [-0.3, -0.18] {
        p.poly(
            &[
                (u - 0.03, 0.14),
                (u + 0.03, 0.14),
                (u + 0.04, 0.22),
                (u, 0.26),
                (u - 0.04, 0.22),
            ],
            hex(0xc0683f),
        );
    }
    p.rect(-0.08, 0.0, 0.0, 0.3, shade(p.trim, -0.1));
    let brick = hex(0xb0603f);
    p.poly(
        &[
            (0.1, 0.0),
            (0.44, 0.0),
            (0.42, 0.34),
            (0.34, 0.6),
            (0.31, 0.9),
            (0.23, 0.9),
            (0.2, 0.6),
            (0.12, 0.34),
        ],
        brick,
    );
    p.arch(0.22, 0.32, 0.0, 0.16, p.k.dark);
    p.rect(0.22, 0.88, 0.32, 0.92, shade(brick, -0.2));
    p.circ(0.3, 0.98, 0.05, p.k.pale.opacity(0.6));
}

/// A gallery: a white front with a big window of pictures and an easel.
pub fn gallery(p: &mut Pen) {
    let wall = p.k.pale;
    p.rect(-0.44, 0.0, 0.36, 0.72, wall);
    p.rect(-0.46, 0.72, 0.38, 0.78, p.k.dark);
    p.rect(-0.36, 0.14, 0.1, 0.56, p.glass);
    for (u, v, ink) in [(-0.24, 0.36, p.k.accent), (-0.04, 0.3, p.k.second)] {
        p.rect(u - 0.08, v - 0.1, u + 0.08, v + 0.1, p.k.brass);
        p.rect(u - 0.06, v - 0.08, u + 0.06, v + 0.08, ink);
    }
    p.rect(0.16, 0.0, 0.3, 0.44, p.k.dark);
    let wood = p.k.wood;
    p.line((0.4, 0.0), (0.46, 0.6), 0.014, wood);
    p.line((0.52, 0.0), (0.46, 0.6), 0.014, wood);
    p.rect(0.36, 0.3, 0.56, 0.52, p.k.pale);
    p.rect(0.38, 0.32, 0.54, 0.5, p.k.leaf);
    p.circ(0.48, 0.44, 0.03, p.k.brass);
}

/// A quilting room: a cottage room with a big window and a quilt frame
/// seen through it.
pub fn quilting_room(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.42, 0.0, 0.42, 0.56, wall);
    p.pitched(-0.42, 0.42, 0.56, 0.9, roof);
    p.window(-0.32, 0.16, 0.12, 0.44);
    let inks = [p.k.accent, p.k.pale, p.k.second, p.k.brass];
    for row in 0..3 {
        for col in 0..5 {
            let (u, v) = (-0.3 + col as f32 * 0.08, 0.18 + row as f32 * 0.08);
            p.rect(
                u,
                v,
                u + 0.07,
                v + 0.07,
                inks[(row + col) % 4].opacity(0.85),
            );
        }
    }
    p.rect(0.2, 0.0, 0.34, 0.36, shade(p.trim, -0.1));
    p.porthole(0.0, 0.68, 0.045);
}

/// A flower cart: buckets of blooms on a barrow with a striped canopy.
pub fn flower_stall(p: &mut Pen) {
    p.shadow(0.44);
    let wood = p.k.wood;
    p.rect(-0.4, 0.24, 0.36, 0.46, shade(wood, 0.05));
    for u in [-0.28, -0.1, 0.08, 0.26] {
        p.rect(u - 0.06, 0.46, u + 0.06, 0.56, p.k.metal);
        p.flowers(u - 0.06, u + 0.06, 0.56, 3, 0.05);
    }
    p.circ(-0.24, 0.16, 0.12, shade(wood, -0.3));
    p.circ(0.22, 0.16, 0.12, shade(wood, -0.3));
    for u in [-0.4, 0.36] {
        p.rect(u - 0.012, 0.46, u + 0.012, 0.86, wood);
    }
    for step in 0..6 {
        let u = -0.44 + step as f32 * 0.14;
        let ink = if step % 2 == 0 { p.k.bloom } else { p.k.pale };
        p.poly(
            &[(u, 0.94), (u + 0.14, 0.94), (u + 0.14, 0.84), (u, 0.84)],
            ink,
        );
    }
}

/// A round dry-stone sheepfold with two sheep inside.
pub fn sheepfold(p: &mut Pen) {
    let stone = p.k.stone;
    p.ell(0.0, 0.3, 0.48, 0.24, shade(p.k.leaf, -0.05));
    p.poly(
        &[
            (-0.48, 0.3),
            (-0.48, 0.6),
            (0.48, 0.6),
            (0.48, 0.3),
            (0.3, 0.14),
            (-0.3, 0.14),
        ],
        stone,
    );
    for step in 0..9 {
        let u = -0.44 + step as f32 * 0.11;
        p.ell(u, 0.6, 0.06, 0.05, shade(stone, (step % 2) as f32 * 0.1));
    }
    for (u, v) in [(-0.12, 0.62), (0.18, 0.66)] {
        p.ell(u, v + 0.12, 0.1, 0.1, p.k.pale);
        p.circ(u + 0.1, v + 0.16, 0.04, p.k.dark);
    }
    p.rect(-0.08, 0.14, 0.08, 0.4, shade(stone, -0.35));
}

/// Allotments: bean poles, cabbages, and a little shed with a water butt.
pub fn allotments(p: &mut Pen) {
    p.shadow(0.48);
    p.rect(-0.48, 0.0, 0.48, 0.08, shade(p.k.wood, -0.35));
    for u in [-0.4, -0.3] {
        p.line((u, 0.08), (u + 0.05, 0.8), 0.01, p.k.wood);
        p.line((u + 0.1, 0.08), (u + 0.05, 0.8), 0.01, p.k.wood);
        for v in [0.3, 0.5] {
            p.circ(u + 0.05, v, 0.035, p.k.leaf);
        }
    }
    for u in [-0.12, 0.0] {
        p.circ(u, 0.14, 0.05, shade(p.k.leaf, 0.1));
    }
    p.rect(0.14, 0.0, 0.42, 0.5, shade(p.k.wood, 0.1));
    p.poly(
        &[(0.1, 0.5), (0.46, 0.58), (0.46, 0.52), (0.1, 0.46)],
        p.roof,
    );
    p.rect(0.22, 0.0, 0.32, 0.34, shade(p.k.wood, -0.25));
    p.rr(0.42, 0.0, 0.5, 0.24, 0.02, p.k.second);
}

/// A weaving shed: a loom seen through its big window, skeins hung out.
pub fn weaving_shed(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.44, 0.0, 0.44, 0.56, wall);
    p.pitched(-0.44, 0.44, 0.56, 0.82, roof);
    p.window(-0.34, 0.14, 0.14, 0.46);
    let wood = p.k.wood;
    p.rect(-0.28, 0.16, -0.25, 0.44, wood);
    p.rect(0.05, 0.16, 0.08, 0.44, wood);
    for step in 0..8 {
        let u = -0.24 + step as f32 * 0.037;
        p.line((u, 0.18), (u, 0.42), 0.005, p.k.accent);
    }
    p.line((-0.26, 0.3), (0.06, 0.3), 0.012, p.k.second);
    p.rect(0.22, 0.0, 0.34, 0.36, shade(p.trim, -0.1));
    p.line((0.44, 0.5), (0.56, 0.5), 0.01, wood);
    for (u, ink) in [(0.48, p.k.accent), (0.53, p.k.second)] {
        p.ell(u, 0.4, 0.025, 0.08, ink);
    }
}

/// A study hut: a small hut with a round window lit by a desk lamp.
pub fn study_hut(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.rect(-0.3, 0.0, 0.3, 0.54, wall);
    p.gable(-0.38, 0.38, 0.54, 0.92, roof);
    p.porthole(-0.1, 0.34, 0.1);
    p.rect(-0.16, 0.26, -0.04, 0.28, p.k.dark);
    p.poly(&[(-0.12, 0.38), (-0.06, 0.38), (-0.08, 0.34)], p.k.brass);
    p.rect(0.08, 0.0, 0.22, 0.32, shade(p.trim, -0.1));
    p.rect(-0.04, 0.6, 0.04, 0.72, p.k.pale);
    p.line((0.3, 0.7), (0.4, 0.84), 0.014, shade(p.k.stone, -0.3));
    p.rect(0.36, 0.84, 0.44, 0.9, shade(p.k.stone, -0.3));
}

/// A standing stone carved with a spiral, log seats in a half circle.
pub fn story_stone(p: &mut Pen) {
    p.shadow(0.46);
    let stone = shade(p.k.stone, -0.05);
    p.poly(
        &[
            (-0.14, 0.0),
            (0.14, 0.0),
            (0.12, 0.8),
            (0.02, 0.94),
            (-0.12, 0.84),
        ],
        stone,
    );
    p.curve(
        (0.0, 0.5),
        (0.08, 0.6),
        (0.0, 0.66),
        0.012,
        shade(stone, -0.3),
    );
    p.curve(
        (0.0, 0.66),
        (-0.1, 0.58),
        (0.0, 0.42),
        0.012,
        shade(stone, -0.3),
    );
    for u in [-0.4, -0.26, 0.26, 0.4] {
        p.rr(u - 0.06, 0.0, u + 0.06, 0.16, 0.02, p.k.wood);
        p.ell(u, 0.16, 0.06, 0.02, shade(p.k.wood, 0.3));
    }
}

/// A joinery: big doors open on a bench, planks stacked, a saw-horse.
pub fn joinery(p: &mut Pen) {
    let (wall, roof) = (shade(p.k.wood, 0.25), p.roof);
    p.rect(-0.44, 0.0, 0.2, 0.6, wall);
    p.pitched(-0.44, 0.2, 0.6, 0.86, roof);
    p.rect(-0.3, 0.0, 0.06, 0.44, p.k.dark);
    p.rect(-0.24, 0.16, 0.0, 0.2, p.k.wood);
    p.rect(-0.2, 0.2, -0.14, 0.26, p.k.metal);
    for step in 0..4 {
        let v = 0.02 + step as f32 * 0.06;
        p.rect(
            0.24,
            v,
            0.5,
            v + 0.05,
            shade(p.k.wood, 0.1 * (step % 2) as f32),
        );
    }
    p.line((0.3, 0.3), (0.4, 0.44), 0.014, p.k.wood);
    p.line((0.44, 0.3), (0.34, 0.44), 0.014, p.k.wood);
    p.line((0.26, 0.44), (0.48, 0.44), 0.02, p.k.wood);
}

/// An octagonal summer house, glazed all round, a finial on its roof.
pub fn summer_house(p: &mut Pen) {
    let (white, roof) = (p.k.pale, p.roof);
    p.rect(-0.34, 0.0, 0.34, 0.08, shade(p.k.stone, -0.1));
    p.rect(-0.3, 0.08, 0.3, 0.56, white);
    for u in [-0.2, 0.0, 0.2] {
        p.window(u - 0.07, 0.24, u + 0.07, 0.5);
    }
    p.rect(-0.3, 0.08, 0.3, 0.2, shade(white, -0.08));
    p.poly(
        &[(-0.38, 0.56), (-0.2, 0.8), (0.2, 0.8), (0.38, 0.56)],
        roof,
    );
    p.gable(-0.2, 0.2, 0.8, 0.9, roof);
    p.line((0.0, 0.9), (0.0, 1.0), 0.012, p.k.brass);
}

/// A rose arbour: an arch of trellis grown over with roses, a seat under.
pub fn rose_arbour(p: &mut Pen) {
    p.shadow(0.44);
    let white = p.k.pale;
    for u in [-0.36, 0.36] {
        p.rect(u - 0.04, 0.0, u + 0.04, 0.66, white.opacity(0.9));
    }
    p.curve((-0.36, 0.66), (0.0, 1.06), (0.36, 0.66), 0.05, white);
    for (u, v) in [
        (-0.36, 0.3),
        (-0.36, 0.56),
        (-0.24, 0.84),
        (0.0, 0.9),
        (0.24, 0.84),
        (0.36, 0.5),
        (0.36, 0.2),
    ] {
        p.circ(u, v, 0.08, p.k.leaf);
        p.circ(u + 0.03, v + 0.02, 0.03, p.k.bloom);
    }
    p.rect(-0.26, 0.2, 0.26, 0.26, p.k.wood);
    p.rect(-0.26, 0.26, -0.22, 0.46, p.k.wood);
}

/// A frog pond with lily pads and a frog on one.
pub fn frog_pond(p: &mut Pen) {
    p.ell(0.0, 0.24, 0.48, 0.2, p.k.stone);
    p.ell(0.0, 0.25, 0.42, 0.15, shade(p.k.water, -0.1));
    for (u, v) in [(-0.2, 0.26), (0.1, 0.2), (0.22, 0.3)] {
        p.ell(u, v, 0.08, 0.04, p.k.leaf);
    }
    p.circ(-0.14, 0.3, 0.02, p.k.bloom);
    p.ell(0.1, 0.26, 0.05, 0.05, hex(0x6f9a3a));
    p.circ(0.08, 0.32, 0.015, p.k.dark);
    p.circ(0.12, 0.32, 0.015, p.k.dark);
    for u in [-0.44, -0.4, 0.42] {
        p.line((u, 0.2), (u + 0.01, 0.7), 0.012, shade(p.k.leaf, -0.1));
    }
}

/// A single swing hung from a tall frame on the hilltop, the sea beyond.
pub fn hilltop_swing(p: &mut Pen) {
    p.shadow(0.4);
    let wood = p.k.wood;
    for side in [-1.0_f32, 1.0] {
        p.line((side * 0.36, 0.0), (side * 0.2, 0.96), 0.035, wood);
        p.line((side * 0.1, 0.0), (side * 0.2, 0.96), 0.035, wood);
    }
    p.line((-0.28, 0.96), (0.28, 0.96), 0.04, wood);
    let swing = 0.05 * p.sway;
    p.line((-0.08, 0.94), (-0.08 + swing, 0.2), 0.01, p.k.dark);
    p.line((0.08, 0.94), (0.08 + swing, 0.2), 0.01, p.k.dark);
    p.rect(-0.12 + swing, 0.17, 0.12 + swing, 0.21, p.k.accent);
}

/// The tall square clock tower of a market square: a clock on each face,
/// a lantern cupola, a weathervane.
pub fn clock_tower_square(p: &mut Pen) {
    let (wall, roof) = (p.wall, p.roof);
    p.shadow(0.36);
    p.rect(-0.3, 0.0, 0.3, 0.66, wall);
    p.arch(-0.12, 0.12, 0.0, 0.22, shade(p.k.dark, 0.2));
    p.rect(-0.32, 0.3, 0.32, 0.32, shade(wall, -0.15));
    p.rect(-0.26, 0.66, 0.26, 0.78, shade(wall, 0.05));
    p.circ(0.0, 0.72, 0.13, p.k.brass);
    p.circ(0.0, 0.72, 0.11, p.k.pale);
    p.line((0.0, 0.72), (0.0, 0.76), 0.012, p.k.dark);
    p.line((0.0, 0.72), (0.05, 0.71), 0.014, p.k.dark);
    p.rect(-0.3, 0.78, 0.3, 0.8, shade(wall, -0.15));
    for u in [-0.14, 0.14] {
        p.rect(u - 0.03, 0.8, u + 0.03, 0.88, wall);
    }
    p.dome(0.0, 0.88, 0.2, 0.06, roof);
    p.line((0.0, 0.93), (0.0, 1.0), 0.01, p.k.dark);
    p.poly(&[(0.0, 0.98), (0.08, 0.97), (0.0, 0.96)], p.k.dark);
    p.window(-0.05, 0.4, 0.05, 0.54);
}

/// The new pier: planks on timber piles running out over the water, a
/// lamp at its end and a bollard or two.
pub fn new_pier(p: &mut Pen) {
    let wood = p.k.wood;
    p.ell(0.1, 0.04, 0.48, 0.07, p.k.water.opacity(0.6));
    p.poly(
        &[(-0.5, 0.46), (0.5, 0.46), (0.5, 0.54), (-0.5, 0.54)],
        shade(wood, 0.1),
    );
    for step in 0..7 {
        let u = -0.44 + step as f32 * 0.15;
        p.rect(u - 0.022, 0.0, u + 0.022, 0.46, shade(wood, -0.3));
        p.line((u - 0.02, 0.1), (u + 0.13, 0.4), 0.008, shade(wood, -0.35));
    }
    for step in 0..12 {
        let u = -0.48 + step as f32 * 0.085;
        p.line((u, 0.46), (u, 0.54), 0.004, shade(wood, -0.2));
    }
    for u in [-0.3, 0.1] {
        p.rr(u - 0.03, 0.54, u + 0.03, 0.62, 0.02, p.k.dark);
    }
    p.rect(0.42, 0.54, 0.45, 0.94, p.k.dark);
    p.rr(0.39, 0.86, 0.48, 0.98, 0.02, p.glass);
}

/// A navigation light on the point: a squat stone plinth on the rocks and
/// an iron lantern on a post, its glass green.
pub fn point_lamp(p: &mut Pen) {
    p.shadow(0.4);
    let stone = p.k.stone;
    for (u, r) in [(-0.3, 0.16), (0.28, 0.18), (0.0, 0.2)] {
        p.ell(u, 0.06, r, 0.08, shade(stone, -0.2));
    }
    p.poly(
        &[(-0.2, 0.08), (0.2, 0.08), (0.14, 0.34), (-0.14, 0.34)],
        stone,
    );
    p.rect(-0.16, 0.34, 0.16, 0.38, shade(stone, 0.12));
    p.rect(-0.03, 0.38, 0.03, 0.78, p.k.dark);
    p.rr(-0.09, 0.78, 0.09, 0.92, 0.02, hex(0x5fbf8a));
    p.rect(-0.1, 0.92, 0.1, 0.94, p.k.dark);
    p.gable(-0.11, 0.11, 0.94, 1.0, p.k.dark);
    p.circ(0.0, 0.85, 0.12, hex(0x5fbf8a).opacity(0.2));
}
