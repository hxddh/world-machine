//! The ground a place is made of, beyond its one wash of field: the patch
//! each cluster of homes and works shares (the Harbour Front's cobbles,
//! the Green's mown grass, Baker's Lane's gardens), the path worn from
//! each down to the spine people walk along, and between the clusters
//! ground that is meant (fields in rows under a hedgerow, a rock field,
//! a lawn, drifts) rather than a leftover lawn. And far off, the back
//! row: the place's own drawings, small and pale with the air between,
//! fading into the sky.
//!
//! All of it is presentation, seeded by the place, so the same place
//! always looks the same.

use super::*;

/// The ground a cluster shares, as the Pack names it: a closed set, so a
/// ground the app cannot paint never reaches here (it fails where the
/// Pack's snapshot is read).
pub(super) type Patch = world_projection::Ground;

/// A cluster's patch of ground on the stage, in stage pixels: from `x0`
/// to `x1`, from `top` (behind its furthest row) to `bottom` (in front of
/// its nearest), and whether it reaches the water's edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PatchPaint {
    pub(super) x0: f32,
    pub(super) x1: f32,
    pub(super) top: f32,
    pub(super) bottom: f32,
    pub(super) patch: Patch,
    pub(super) water: bool,
    pub(super) seed: u32,
}

/// Where each cluster's patch lies on the stage.
pub(super) fn patches(snapshot: &ProjectionSnapshot, stage: &Stage) -> Vec<PatchPaint> {
    let panorama = stage.panorama();
    let quay_top = stage.feet - stage.figure_h * 0.55;
    snapshot
        .canvas
        .clusters
        .iter()
        .map(|cluster| {
            let x0 = cluster.from.clamp(0.0, panorama) * stage.view_w;
            let x1 = cluster.to.clamp(0.0, panorama) * stage.view_w;
            let (back, scale) = stage.row_line(cluster.rows.0.min(0.76));
            let (front, _) = stage.row_line(cluster.rows.1.min(0.76));
            let water = cluster.rows.1 >= WATER_ROW;
            let top = back - stage.building_h * scale * 0.1;
            let bottom = if water || cluster.rows.1 >= 0.7 {
                quay_top + stage.figure_h * 0.05
            } else {
                (front + stage.figure_h * 0.35).min(quay_top - stage.figure_h * 0.3)
            };
            PatchPaint {
                x0,
                x1,
                top: top.min(bottom - stage.figure_h * 0.5),
                bottom,
                patch: cluster.ground,
                water,
                seed: art::seed_of(&cluster.id),
            }
        })
        .filter(|patch| patch.x1 - patch.x0 > 1.0)
        .collect()
}

/// The colours of a patch: its ground, its marks, for the place's field.
fn inks(patch: Patch, field: Hsla, frame: &Frame) -> (Hsla, Hsla) {
    let (ground, mark) = raw_inks(patch, field, frame.setting);
    // Never lighter than the sky's light: the land is the middle value.
    (
        under_sky(&frame.scenery, ground, GROUND_UNDER_SKY),
        under_sky(&frame.scenery, mark, GROUND_UNDER_SKY),
    )
}

fn raw_inks(patch: Patch, field: Hsla, setting: art::Setting) -> (Hsla, Hsla) {
    let paint = |hex: u32, share: f32| mix(art::hex(hex), field, share);
    match patch {
        Patch::Cobbles => (paint(0xcfc3ad, 0.25), paint(0x9c8f78, 0.2)),
        Patch::Plaza => (paint(0xdcd2c0, 0.25), paint(0xb0a48e, 0.25)),
        Patch::Garden => (art::shade(field, -0.07), paint(0x8a6a48, 0.3)),
        Patch::Green => (art::shade(field, 0.07), art::shade(field, 0.14)),
        Patch::Yard => (paint(0xbaa47f, 0.45), paint(0x8f7a5a, 0.4)),
        Patch::Pad => (art::shade(field, -0.1), art::shade(field, -0.22)),
        Patch::Paving => (paint(0xc8c5be, 0.2), paint(0xa29f98, 0.2)),
        Patch::Snow => (paint(0xf4f8fb, 0.55), paint(0xc4d6e4, 0.3)),
        Patch::Rock => (paint(0xa8a196, 0.3), paint(0x7c766c, 0.3)),
        Patch::Worn => match setting {
            art::Setting::Ice => (paint(0xf4f8fb, 0.55), paint(0xc4d6e4, 0.3)),
            _ => (art::shade(field, 0.08), art::shade(field, -0.1)),
        },
    }
}

/// A soft-edged blob from `x0` to `x1` and `top` to `bottom`: its back
/// edge wandering, its ends rounded.
fn blob(x0: f32, x1: f32, top: f32, bottom: f32, seed: u32) -> Shape {
    let mut shape = Shape::new();
    let w = (x1 - x0).max(1.0);
    let h = (bottom - top).max(1.0);
    let steps = ((w / 18.0).ceil() as i32).max(6);
    let r = h.min(w * 0.25);
    let wobble = |i: i32, salt: i32| {
        ((painter::hash2(i, salt, seed) % 1000) as f32 / 1000.0 - 0.5) * h * 0.12
    };
    shape.move_to(x0 + r, top + wobble(0, 1));
    for i in 1..=steps {
        let x = x0 + r + (w - 2.0 * r) * i as f32 / steps as f32;
        shape.line_to(x, top + wobble(i, 1));
    }
    shape.curve_to(x1, top + r, x1, top);
    shape.line_to(x1, bottom - r * 0.4);
    shape.curve_to(x1 - r, bottom, x1, bottom);
    for i in (0..=steps).rev() {
        let x = x0 + r + (w - 2.0 * r) * i as f32 / steps as f32;
        shape.line_to(x, bottom + wobble(i, 2) * 0.4);
    }
    shape.curve_to(x0, bottom - r * 0.4, x0, bottom);
    shape.line_to(x0, top + r);
    shape.curve_to(x0 + r, top, x0, top);
    shape.close();
    shape
}

/// Paints the clusters' patches, the paths from each down to the spine,
/// and the ground meant between them, for stage `x` from `from` to `to`.
pub(super) fn paint_plan(
    canvas: &mut Canvas,
    frame: &Frame,
    (from, to): (f32, f32),
    field: Hsla,
    k: f32,
) {
    let spine = if frame.water {
        frame.quay_top()
    } else {
        frame.feet - frame.figure_h * 0.25
    };
    paint_between(canvas, frame, (from, to), field, k, spine);
    for patch in &frame.patches {
        if patch.x1 < from - 80.0 || patch.x0 > to + 80.0 {
            continue;
        }
        paint_fore(canvas, frame, patch, field, k, spine);
    }
    for patch in &frame.patches {
        if patch.x1 < from - 80.0 || patch.x0 > to + 80.0 {
            continue;
        }
        paint_path(canvas, frame, patch, field, k, spine);
    }
    for patch in &frame.patches {
        if patch.x1 < from - 80.0 || patch.x0 > to + 80.0 {
            continue;
        }
        paint_patch(canvas, frame, patch, field, k);
    }
}

/// Where a cluster's path comes down from it: the middle of its front,
/// off to one side by its seed.
fn path_mid(patch: &PatchPaint) -> f32 {
    (patch.x0 + patch.x1) / 2.0 + ((patch.seed >> 8) % 60) as f32 - 30.0
}

/// The ground between a cluster and the spine, meant rather than left
/// over: kitchen gardens in rows behind a low hedge by the harbour, grow
/// trays on Mars, a clipped lawn and hedge on Maple Street, drifts on the
/// ice; the path down from the cluster runs through it.
fn paint_fore(
    canvas: &mut Canvas,
    frame: &Frame,
    patch: &PatchPaint,
    field: Hsla,
    k: f32,
    spine: f32,
) {
    let (top, bottom) = (patch.bottom + 4.0 * k, spine - frame.figure_h * 0.35);
    if patch.water || bottom - top < frame.figure_h * 0.5 {
        return;
    }
    let mid = path_mid(patch);
    let lane = frame.figure_h * 0.45;
    for (x0, x1) in [
        (patch.x0 + lane * 0.3, mid - lane),
        (mid + lane, patch.x1 - lane * 0.3),
    ] {
        if x1 - x0 < frame.figure_h * 0.6 {
            continue;
        }
        let seed = painter::hash2(x0 as i32, 41, patch.seed);
        match frame.setting {
            art::Setting::Harbour | art::Setting::Mars => {
                // Beds in rows, one crop to a bed, and a low hedge (or a
                // tray rim) along the front.
                let soil = under_sky(
                    &frame.scenery,
                    mix(
                        art::hex(0x8a6a48),
                        field,
                        if frame.setting == art::Setting::Mars {
                            0.6
                        } else {
                            0.45
                        },
                    ),
                    GROUND_UNDER_SKY,
                );
                let crop = art::shade(field, -0.18);
                let rows = (((bottom - top) / (7.0 * k)) as i32).clamp(2, 8);
                for row in 0..rows {
                    let y = top + (bottom - top) * (row as f32 + 0.5) / rows as f32;
                    let inset = (x1 - x0) * 0.04 * (rows - row) as f32 / rows as f32;
                    art::rect(
                        canvas,
                        x0 + inset,
                        y - 1.8 * k,
                        x1 - x0 - inset * 2.0,
                        3.6 * k,
                        1.5 * k,
                        soil.opacity(0.75),
                    );
                    if !(seed >> row).is_multiple_of(3) {
                        let mut x = x0 + inset + 4.0 * k;
                        while x < x1 - inset - 3.0 * k {
                            art::circle(canvas, x, y - 1.6 * k, 1.7 * k, crop.opacity(0.85));
                            x += 6.0 * k;
                        }
                    }
                }
                if frame.setting == art::Setting::Harbour {
                    let hedge = art::shade(field, -0.28);
                    let mut x = x0;
                    while x < x1 {
                        art::ellipse(
                            canvas,
                            x,
                            bottom + 2.0 * k,
                            4.5 * k,
                            3.0 * k,
                            hedge.opacity(0.8),
                        );
                        x += 6.5 * k;
                    }
                }
            }
            art::Setting::Street => {
                paint_lawn(canvas, (x0, x1, top, bottom + 4.0 * k), field, k, seed)
            }
            art::Setting::Ice => paint_drifts(canvas, frame, (x0, x1, top, bottom), k, seed),
        }
    }
}

/// What stands on the water line stands on something: a building on a
/// timber jetty on piles with its slipway down to the water, the
/// lighthouse on a spit of rock running out from the quay.
pub(super) fn paint_footings(canvas: &mut Canvas, frame: &Frame, (from, to): (f32, f32), k: f32) {
    let edge = frame.front;
    for building in &frame.buildings {
        if building.base < edge - 1.0
            || building.x + building.w < from - 40.0
            || building.x - building.w > to + 40.0
        {
            continue;
        }
        let (x, base, w) = (building.x, building.base, building.w);
        // The tower on the point, by its shape or its drawing: never on a deck
        // of planks (v0.28's dark slab at its foot).
        if building.shape == MarkShape::Tower || super::light::is_beacon(building) {
            // A plinth of rocks out into the water under the tower:
            // boulders heaped from the spine's edge to its foot, lit from
            // the upper left, darker where the sea wets them, with turf
            // between the top stones. Painted, never a slab: no outline,
            // no straight edge (v0.28's dark box at its foot).
            let stone = under_sky(
                &frame.scenery,
                mix(art::hex(0x8a8276), art::hex(frame.scenery.near), 0.2),
                GROUND_UNDER_SKY,
            );
            let foot = base + w * 0.12;
            let wet = art::shade(stone, -0.3);
            // The heap's shadowed mass first, soft at its edge.
            canvas.soft(
                x,
                (edge + foot) * 0.5,
                w * 0.62,
                (foot - edge).max(4.0 * k) * 0.55,
                6.0 * k,
                art::shade(stone, -0.22).opacity(0.9),
            );
            // The boulders, back to front: bigger toward the foot.
            let count = 11;
            for i in 0..count {
                let s = painter::hash2(i, 41, building.index as u32);
                let t = i as f32 / (count - 1) as f32;
                let across = ((s % 1000) as f32 / 1000.0 - 0.5) * 2.0;
                let ry_ = edge + (foot - edge) * (0.15 + 0.85 * t);
                let reach = w * (0.62 - 0.22 * t);
                let rx = x + across * reach;
                let r = (w * (0.07 + 0.05 * t) + ((s >> 10) % 4) as f32 * k).max(2.5 * k);
                let lit = art::shade(stone, 0.04 - 0.1 * t + ((s >> 14) % 3) as f32 * 0.03);
                art::ellipse(canvas, rx, ry_, r, r * 0.62, lit);
                // The sea darkens the lowest of them.
                if t > 0.55 {
                    art::ellipse(
                        canvas,
                        rx,
                        ry_ + r * 0.32,
                        r * 0.92,
                        r * 0.3,
                        wet.opacity(0.55),
                    );
                }
                art::ellipse(
                    canvas,
                    rx - r * 0.28,
                    ry_ - r * 0.24,
                    r * 0.5,
                    r * 0.24,
                    art::shade(stone, 0.2),
                );
            }
            // Turf between the top stones, and a fringe of weed where the
            // rocks meet the water.
            let turf = under_sky(
                &frame.scenery,
                art::shade(art::hex(frame.scenery.far), -0.05),
                GROUND_UNDER_SKY,
            );
            canvas.soft(
                x - w * 0.1,
                edge + (foot - edge) * 0.12,
                w * 0.45,
                (foot - edge).max(4.0 * k) * 0.12,
                3.0 * k,
                turf.opacity(0.8),
            );
            canvas.soft(
                x,
                foot + w * 0.01,
                w * 0.5,
                2.5 * k,
                3.0 * k,
                art::shade(turf, -0.45).opacity(0.45),
            );
            continue;
        }
        // A deck of planks on piles, from the quay's edge out to its foot,
        // and a slipway down into the water in front.
        let wood = under_sky(&frame.scenery, art::hex(0x8a6a44), GROUND_UNDER_SKY);
        let (left, right) = (x - w * 0.6, x + w * 0.6);
        let deck = base.max(edge) + 6.0 * k;
        art::rect(
            canvas,
            left,
            edge - 1.0,
            right - left,
            deck - edge + 3.0 * k,
            1.0,
            art::shade(wood, 0.05),
        );
        let mut plank = left + 6.0 * k;
        while plank < right {
            art::line(
                canvas,
                (plank, edge),
                (plank, deck + 2.0 * k),
                0.8 * k,
                art::shade(wood, -0.25).opacity(0.5),
            );
            plank += 7.0 * k;
        }
        for pile in 0..=4 {
            let px0 = left + (right - left) * pile as f32 / 4.0;
            art::rect(
                canvas,
                px0 - 1.5 * k,
                deck,
                3.0 * k,
                9.0 * k,
                0.5,
                art::shade(wood, -0.35),
            );
        }
        let mut ramp = Shape::new();
        ramp.move_to(x - w * 0.25, deck)
            .line_to(x + w * 0.25, deck)
            .line_to(x + w * 0.32, deck + 12.0 * k)
            .line_to(x - w * 0.32, deck + 12.0 * k)
            .close();
        canvas.fill(&ramp, art::shade(wood, -0.1).opacity(0.85));
    }
}

/// A path worn from a cluster down to the spine: a soft strip, lighter
/// where it is walked, narrowing with distance.
fn paint_path(
    canvas: &mut Canvas,
    frame: &Frame,
    patch: &PatchPaint,
    field: Hsla,
    k: f32,
    spine: f32,
) {
    if patch.bottom >= spine - 2.0 {
        return;
    }
    let ink = match frame.setting {
        art::Setting::Ice => art::hex(0xf2f7fa),
        art::Setting::Street => art::hex(0xcac7c0),
        art::Setting::Mars => art::shade(field, 0.1),
        art::Setting::Harbour => mix(art::hex(0xd6c9a8), field, 0.4),
    };
    let ink = under_sky(&frame.scenery, ink, GROUND_UNDER_SKY);
    let lean = ((patch.seed % 100) as f32 / 100.0 - 0.5) * frame.figure_h * 1.2;
    let mid = path_mid(patch);
    let (top, bottom) = (patch.bottom - 6.0 * k, spine + 2.0);
    let (w_top, w_bottom) = (frame.figure_h * 0.28, frame.figure_h * 0.5);
    let x_top = mid;
    let x_bottom = mid + lean;
    let mut path = Shape::new();
    path.move_to(x_top - w_top / 2.0, top)
        .curve_to(
            x_bottom - w_bottom / 2.0,
            bottom,
            x_top - w_top / 2.0 + lean * 0.2,
            (top + bottom) / 2.0,
        )
        .line_to(x_bottom + w_bottom / 2.0, bottom)
        .curve_to(
            x_top + w_top / 2.0,
            top,
            x_top + w_top / 2.0 + lean * 0.6,
            (top + bottom) / 2.0,
        )
        .close();
    canvas.fill(&path, ink.opacity(0.75));
    // Walked lighter down its middle.
    let mut worn = Shape::new();
    worn.move_to(x_top, top + 2.0).curve_to(
        x_bottom,
        bottom,
        x_top + lean * 0.4,
        (top + bottom) / 2.0,
    );
    canvas.stroke(&worn, w_top * 0.4, art::shade(ink, 0.08).opacity(0.5));
}

/// One cluster's patch of ground, and what marks it as that ground.
fn paint_patch(canvas: &mut Canvas, frame: &Frame, patch: &PatchPaint, field: Hsla, k: f32) {
    let (ground, mark) = inks(patch.patch, field, frame);
    let (x0, x1, top, bottom) = (patch.x0, patch.x1, patch.top, patch.bottom);
    // A little wider at its foot, as ground seen in depth is. A patch at
    // the water (the Point) lays no ground over the sea, only its rocks:
    // a flat grey apron out into the water read as a slab.
    let bleed = frame.figure_h * 0.12;
    let body = match patch.patch {
        Patch::Cobbles | Patch::Plaza | Patch::Yard | Patch::Paving => 0.5,
        _ => 0.65,
    };
    let (under, body) = if patch.water {
        (0.0, 0.0)
    } else {
        (0.35, body)
    };
    // Nor does any patch run on past the water's edge.
    let bottom = if frame.water {
        bottom.min(frame.front - 4.0 * k)
    } else {
        bottom
    };
    if bottom <= top {
        return;
    }
    canvas.fill(
        &blob(
            x0 - bleed,
            x1 + bleed,
            top - bleed * 0.4,
            bottom + bleed * 0.3,
            patch.seed,
        ),
        ground.opacity(under),
    );
    canvas.fill(&blob(x0, x1, top, bottom, patch.seed), ground.opacity(body));
    let w = x1 - x0;
    let h = bottom - top;
    let at = |i: i32, salt: i32| {
        let seed = painter::hash2(i, salt, patch.seed);
        (
            x0 + w * (0.06 + 0.88 * (seed % 1000) as f32 / 1000.0),
            top + h * (0.15 + 0.8 * ((seed / 1000) % 1000) as f32 / 1000.0),
            seed,
        )
    };
    let count = (w * h / (900.0 * k * k)).clamp(4.0, 400.0) as i32;
    match patch.patch {
        Patch::Cobbles | Patch::Rock => {
            for i in 0..count {
                let (x, y, seed) = at(i, 3);
                let r = (2.6 + (seed % 7) as f32 * 0.4) * k;
                let lit = if seed.is_multiple_of(3) { 0.12 } else { -0.05 };
                art::ellipse(
                    canvas,
                    x,
                    y,
                    r,
                    r * 0.55,
                    art::shade(mark, lit).opacity(0.45),
                );
            }
        }
        Patch::Plaza | Patch::Paving => {
            let slab = 34.0 * k;
            let mut y = top + slab * 0.5;
            let joint = mark.opacity(0.35);
            while y < bottom - 4.0 {
                let mut line = Shape::new();
                line.move_to(x0 + slab * 0.5, y).line_to(x1 - slab * 0.5, y);
                canvas.stroke(&line, 0.8 * k, joint);
                y += slab * 0.45;
            }
        }
        Patch::Garden => {
            // Beds in rows, a few in flower.
            for i in 0..(count / 3).max(3) {
                let (x, y, seed) = at(i, 5);
                let bw = (16.0 + (seed % 12) as f32) * k;
                art::rect(
                    canvas,
                    x - bw / 2.0,
                    y,
                    bw,
                    4.0 * k,
                    2.0 * k,
                    mark.opacity(0.55),
                );
                if seed.is_multiple_of(2) {
                    let flower = [0xf2d0e0_u32, 0xfff2b0, 0xe8a0a0][(seed >> 5) as usize % 3];
                    for petal in 0..3 {
                        art::circle(
                            canvas,
                            x - bw * 0.3 + petal as f32 * bw * 0.3,
                            y - 1.5 * k,
                            1.6 * k,
                            art::hex(flower).opacity(0.9),
                        );
                    }
                }
            }
        }
        Patch::Green => {
            // Mown in stripes.
            let stripe = 26.0 * k;
            let mut x = x0 + stripe;
            let mut light = true;
            while x < x1 - stripe {
                if light {
                    art::rect(
                        canvas,
                        x,
                        top + h * 0.12,
                        stripe,
                        h * 0.78,
                        stripe * 0.4,
                        mark.opacity(0.4),
                    );
                }
                light = !light;
                x += stripe;
            }
        }
        Patch::Yard | Patch::Pad | Patch::Worn => {
            for i in 0..(count / 2) {
                let (x, y, seed) = at(i, 7);
                art::ellipse(
                    canvas,
                    x,
                    y,
                    (1.6 + (seed % 5) as f32 * 0.5) * k,
                    1.1 * k,
                    mark.opacity(0.5),
                );
            }
            if patch.patch == Patch::Pad {
                // Tread marks across it.
                for lane in 0..2 {
                    let y = top + h * (0.45 + 0.2 * lane as f32);
                    let mut track = Shape::new();
                    track
                        .move_to(x0 + w * 0.08, y)
                        .line_to(x1 - w * 0.08, y + h * 0.05);
                    canvas.stroke(&track, 1.2 * k, mark.opacity(0.35));
                }
            }
        }
        Patch::Snow => {
            for i in 0..(count / 2) {
                let (x, y, _) = at(i, 9);
                art::ellipse(canvas, x, y, 1.4 * k, 0.8 * k, mark.opacity(0.55));
            }
        }
    }
    // Where a cluster stands at the water, a few rocks run out into it.
    if patch.water && patch.patch == Patch::Rock {
        for i in 0..6 {
            let seed = painter::hash2(i, 11, patch.seed);
            let x = x0 + w * (0.2 + 0.6 * (seed % 1000) as f32 / 1000.0);
            let y = frame.front + (2.0 + (seed % 9) as f32) * k;
            let r = (6.0 + (seed >> 10) as f32 % 8.0) * k;
            art::ellipse(canvas, x, y, r, r * 0.5, art::shade(mark, -0.1));
            art::ellipse(
                canvas,
                x - r * 0.2,
                y - r * 0.15,
                r * 0.6,
                r * 0.25,
                art::shade(mark, 0.15),
            );
        }
    }
}

/// The ground meant between the clusters: wherever a stretch wider than
/// a fifth of a window has no cluster, it is a field, a rock field, a
/// lawn or drifts, each place its own, so no window is a bare lawn.
fn paint_between(
    canvas: &mut Canvas,
    frame: &Frame,
    (from, to): (f32, f32),
    field: Hsla,
    k: f32,
    spine: f32,
) {
    for (x0, x1) in gaps(frame) {
        if x1 < from - 40.0 || x0 > to + 40.0 {
            continue;
        }
        let seed = painter::hash2(x0 as i32, 17, seed_of_scenery(&frame.scenery));
        let top = frame.base - frame.figure_h * 0.3;
        let bottom = spine - frame.figure_h * 0.45;
        if bottom - top < frame.figure_h * 0.6 {
            continue;
        }
        let pad = frame.figure_h * 0.5;
        let (x0, x1) = (x0 + pad, x1 - pad);
        if x1 - x0 < frame.figure_h {
            continue;
        }
        match frame.setting {
            art::Setting::Harbour => paint_field(canvas, (x0, x1, top, bottom), field, k, seed),
            art::Setting::Mars => paint_rock_field(canvas, (x0, x1, top, bottom), field, k, seed),
            art::Setting::Street => paint_lawn(canvas, (x0, x1, top, bottom), field, k, seed),
            art::Setting::Ice => paint_drifts(canvas, frame, (x0, x1, top, bottom), k, seed),
        }
    }
}

/// The stretches of ground no cluster stands on, wider than a fifth of a
/// window, in stage pixels.
pub(super) fn gaps(frame: &Frame) -> Vec<(f32, f32)> {
    let least = frame.view_w * 0.2;
    let mut taken = frame
        .patches
        .iter()
        .filter(|patch| !patch.water || patch.top < frame.feet - frame.figure_h)
        .map(|patch| (patch.x0, patch.x1))
        .collect::<Vec<_>>();
    taken.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    let mut at = 0.0_f32;
    for (x0, x1) in taken {
        if x0 - at >= least {
            out.push((at, x0));
        }
        at = at.max(x1);
    }
    if frame.width - at >= least {
        out.push((at, frame.width));
    }
    out
}

/// A field in rows under a hedgerow, seen from the shore: furrows running
/// back toward the hills, two greens (or a stubble gold), and a hedge
/// with a tree or two along its back.
fn paint_field(canvas: &mut Canvas, (x0, x1, top, bottom): Rect, field: Hsla, k: f32, seed: u32) {
    let crops = [
        (art::shade(field, -0.06), art::shade(field, 0.05)),
        (
            mix(art::hex(0xc9b77a), field, 0.45),
            mix(art::hex(0xb59f62), field, 0.45),
        ),
        (
            mix(art::hex(0x9b7d5a), field, 0.5),
            mix(art::hex(0x7f6447), field, 0.5),
        ),
    ];
    let (a, b) = crops[(seed % 3) as usize];
    let h = bottom - top;
    let field_top = top + h * 0.12;
    let inset = h * 0.18;
    let mut plot = Shape::new();
    plot.move_to(x0 + inset, field_top)
        .line_to(x1 - inset, field_top)
        .line_to(x1, bottom)
        .line_to(x0, bottom)
        .close();
    canvas.fill(&plot, a.opacity(0.7));
    // Furrows, converging toward the back: fine lines of the darker
    // crop, as a painter suggests rows.
    let rows = (((x1 - x0) / (11.0 * k)) as i32).clamp(6, 120);
    for row in 1..rows {
        let t = row as f32 / rows as f32;
        let near = x0 + (x1 - x0) * t;
        let far = x0 + inset + (x1 - x0 - 2.0 * inset) * t;
        let mut furrow = Shape::new();
        furrow
            .move_to(far, field_top + 1.0)
            .line_to(near, bottom - 1.0);
        canvas.stroke(&furrow, 1.3 * k, b.opacity(0.45));
    }
    // The hedgerow along its back, and a tree or two in it.
    let hedge = art::shade(field, -0.3);
    let bumps = (((x1 - x0) / (9.0 * k)) as i32).max(4);
    for bump in 0..bumps {
        let x = x0 + inset * 0.6 + (x1 - x0 - inset * 1.2) * bump as f32 / bumps as f32;
        let wob = painter::hash2(bump, 19, seed) % 5;
        art::ellipse(
            canvas,
            x,
            field_top - (1.0 + wob as f32 * 0.4) * k,
            6.5 * k,
            (4.0 + wob as f32 * 0.5) * k,
            hedge.opacity(0.85),
        );
    }
    for tree in 0..(1 + seed % 2) {
        let x = x0 + inset + (x1 - x0 - 2.0 * inset) * (0.25 + 0.5 * tree as f32);
        let t = 22.0 * k;
        art::rect(
            canvas,
            x - 1.2 * k,
            field_top - t * 0.9,
            2.4 * k,
            t * 0.9,
            1.0,
            art::hex(0x6b5640),
        );
        art::ellipse(
            canvas,
            x,
            field_top - t,
            t * 0.45,
            t * 0.4,
            art::shade(hedge, 0.05),
        );
        art::ellipse(
            canvas,
            x - t * 0.12,
            field_top - t * 1.08,
            t * 0.25,
            t * 0.2,
            art::shade(hedge, 0.2).opacity(0.7),
        );
    }
}

/// Boulders and rover tracks across the regolith.
fn paint_rock_field(
    canvas: &mut Canvas,
    (x0, x1, top, bottom): Rect,
    field: Hsla,
    k: f32,
    seed: u32,
) {
    let h = bottom - top;
    let count = (((x1 - x0) / (40.0 * k)) as i32).clamp(3, 40);
    for i in 0..count {
        let s = painter::hash2(i, 23, seed);
        let x = x0 + (x1 - x0) * (s % 1000) as f32 / 1000.0;
        let y = top + h * (0.2 + 0.75 * ((s / 1000) % 1000) as f32 / 1000.0);
        let r = (5.0 + (s >> 20) as f32 % 9.0) * k;
        art::ellipse(
            canvas,
            x + r * 0.3,
            y + r * 0.25,
            r * 1.1,
            r * 0.3,
            art::shade(field, -0.25).opacity(0.4),
        );
        art::ellipse(canvas, x, y, r, r * 0.6, art::shade(field, -0.15));
        art::ellipse(
            canvas,
            x - r * 0.25,
            y - r * 0.2,
            r * 0.5,
            r * 0.25,
            art::shade(field, 0.1),
        );
    }
    for lane in 0..2 {
        let y0 = top + h * (0.35 + 0.1 * lane as f32);
        let mut track = Shape::new();
        track
            .move_to(x0, y0 + h * 0.3)
            .curve_to(x1, y0, (x0 + x1) / 2.0, y0 + h * 0.45);
        canvas.stroke(&track, 1.4 * k, art::shade(field, -0.18).opacity(0.45));
    }
}

/// A lawn behind a picket fence, mown in stripes, a tree at one end.
fn paint_lawn(canvas: &mut Canvas, (x0, x1, top, bottom): Rect, field: Hsla, k: f32, seed: u32) {
    let h = bottom - top;
    let lawn = art::shade(field, 0.04);
    art::rect(
        canvas,
        x0,
        top + h * 0.2,
        x1 - x0,
        h * 0.75,
        6.0 * k,
        lawn.opacity(0.8),
    );
    let stripe = 22.0 * k;
    let mut x = x0;
    let mut light = seed.is_multiple_of(2);
    while x < x1 {
        if light {
            art::rect(
                canvas,
                x,
                top + h * 0.2,
                stripe.min(x1 - x),
                h * 0.75,
                0.0,
                art::shade(lawn, 0.07).opacity(0.5),
            );
        }
        light = !light;
        x += stripe;
    }
    let fence = art::hex(0xf2efe6).opacity(0.9);
    let y = bottom - 2.0 * k;
    let mut post = x0;
    while post < x1 {
        art::rect(canvas, post, y - 9.0 * k, 2.0 * k, 9.0 * k, 0.5, fence);
        post += 7.0 * k;
    }
    art::rect(canvas, x0, y - 6.5 * k, x1 - x0, 1.4 * k, 0.0, fence);
    let tx = if seed.is_multiple_of(2) {
        x0 + 18.0 * k
    } else {
        x1 - 18.0 * k
    };
    let t = 30.0 * k;
    let ty = top + h * 0.5;
    art::rect(
        canvas,
        tx - 1.5 * k,
        ty - t * 0.7,
        3.0 * k,
        t * 0.7,
        1.0,
        art::hex(0x6b5640),
    );
    art::ellipse(
        canvas,
        tx,
        ty - t * 0.85,
        t * 0.4,
        t * 0.36,
        art::shade(field, -0.22),
    );
}

/// Wind-cut drifts on the snow, blue in their lee, and a few pebbles.
fn paint_drifts(
    canvas: &mut Canvas,
    frame: &Frame,
    (x0, x1, top, bottom): Rect,
    k: f32,
    seed: u32,
) {
    let snow = under_sky(&frame.scenery, art::hex(0xfbfdff), GROUND_UNDER_SKY);
    let h = bottom - top;
    let count = (((x1 - x0) / (60.0 * k)) as i32).clamp(2, 30);
    for i in 0..count {
        let s = painter::hash2(i, 29, seed);
        let x = x0 + (x1 - x0) * (s % 1000) as f32 / 1000.0;
        let y = top + h * (0.25 + 0.7 * ((s / 1000) % 1000) as f32 / 1000.0);
        let w = (30.0 + (s >> 20) as f32 % 30.0) * k;
        let mut ridge = Shape::new();
        ridge
            .move_to(x - w / 2.0, y)
            .curve_to(x + w / 2.0, y, x, y - 7.0 * k);
        ridge.close();
        canvas.fill(&ridge, snow.opacity(0.85));
        let mut lee = Shape::new();
        lee.move_to(x - w * 0.1, y - 4.0 * k)
            .curve_to(x + w / 2.0, y, x + w * 0.25, y - 3.0 * k);
        canvas.stroke(&lee, 1.6 * k, art::hex(0xb8cfe0).opacity(0.7));
        if s.is_multiple_of(3) {
            art::ellipse(
                canvas,
                x + w * 0.6,
                y + 2.0 * k,
                2.0 * k,
                1.2 * k,
                art::hex(0x8a8f96),
            );
        }
    }
}

/// The drawings the back row is made of, each place its own, as library
/// keys or the harbour's plain shapes.
fn back_row_drawings(setting: art::Setting) -> &'static [(&'static str, MarkShape)] {
    match setting {
        art::Setting::Harbour => &[
            ("", MarkShape::House),
            ("apple-tree", MarkShape::Tree),
            ("", MarkShape::House),
            ("windmill", MarkShape::Tower),
            ("apple-tree", MarkShape::Tree),
            ("", MarkShape::House),
            ("chapel", MarkShape::House),
            ("apple-tree", MarkShape::Tree),
            ("", MarkShape::Shop),
            ("dovecote", MarkShape::Tower),
        ],
        art::Setting::Mars => &[
            ("mars-dome", MarkShape::Dome),
            ("hab-module", MarkShape::House),
            ("comms-mast", MarkShape::Tower),
            ("hab-module", MarkShape::House),
            ("radio-dish", MarkShape::Tower),
            ("mars-dome", MarkShape::Dome),
            ("solar-array", MarkShape::Garden),
        ],
        art::Setting::Street => &[
            ("row-house", MarkShape::House),
            ("maple-tree", MarkShape::Tree),
            ("row-house", MarkShape::House),
            ("water-tower", MarkShape::Tower),
            ("storefront", MarkShape::Shop),
            ("maple-tree", MarkShape::Tree),
            ("row-house", MarkShape::House),
        ],
        art::Setting::Ice => &[
            ("snow-nest", MarkShape::House),
            ("ice-spire", MarkShape::Tower),
            ("snow-nest", MarkShape::House),
            ("snow-dome", MarkShape::Dome),
            ("snow-nest", MarkShape::House),
        ],
    }
}

/// How far toward the sky the back row's colours go: the air between.
pub(super) const BACK_ROW_AIR: f32 = 0.55;

/// The back row along the nearest ridge: the place's own drawings, about
/// two fifths as big as the front's, in hamlets with fields of air between,
/// every colour taken most of the way to the sky's. Painted onto the band
/// at `at`, from band `x` `from` to `to`.
pub(super) fn paint_back_row(
    canvas: &mut Canvas,
    frame: &Frame,
    band: &Band,
    (from, to): (f32, f32),
) {
    let drawings = back_row_drawings(frame.setting);
    let seed = seed_of_scenery(&frame.scenery);
    let haze = art::hex(frame.scenery.sky_bottom);
    let h = frame.building_h * 0.4 * 0.7;
    let spacing = frame.view_w * 0.21;
    let first = ((from - spacing) / spacing).floor() as i32;
    let last = ((to + spacing) / spacing).ceil() as i32;
    // Painted onto their own layer, then taken toward the sky together.
    let Some(mut layer) = Canvas::new(canvas.width(), canvas.height(), canvas.scale, canvas.origin)
    else {
        return;
    };
    // Where the homes stand, for their windows after dark.
    let mut windows = Vec::new();
    for hamlet in first..=last {
        let s = painter::hash2(hamlet, 31, seed);
        // Some stretches of the ridge are left bare.
        if s.is_multiple_of(4) {
            continue;
        }
        let count = 1 + (s >> 4) % 3;
        let centre = hamlet as f32 * spacing + (s >> 8) as f32 % (spacing * 0.5);
        for n in 0..count {
            let pick = drawings[((s >> 12) as usize + n as usize * 3) % drawings.len()];
            let x = centre + n as f32 * h * 0.9;
            let base = band.ridge(2, x, seed) + band.view_h * 0.02;
            let (key, shape) = pick;
            let art = (!key.is_empty()).then_some(key);
            let drawn = crate::works::drawn(art, shape, frame.setting);
            let palette = art::Palette::of_in(&format!("back-{hamlet}-{n}"), false, frame.setting)
                .drawn_as(art);
            let w = h * if shape == MarkShape::Tree { 0.6 } else { 0.95 };
            if matches!(shape, MarkShape::House | MarkShape::Shop | MarkShape::Dome) {
                windows.push((x, base, w));
            }
            match drawn {
                crate::works::Drawn::Art(found) => crate::works::paint_art(
                    &mut layer,
                    found,
                    x,
                    base,
                    w,
                    h * (found.tall() / 1.2).clamp(0.5, 1.6),
                    &palette,
                    0.0,
                ),
                crate::works::Drawn::Plain(shape) => {
                    crate::art::paint_building(&mut layer, x, base, w, h, shape, &palette);
                }
            }
        }
    }
    // By day toward the sky; after dark toward the ridge they stand on,
    // so they sink into the hill, with a window or two lit.
    let dark = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    if dark {
        let far = mix(art::hex(frame.scenery.far), haze, 0.3);
        toward(&mut layer.pixmap, far, 0.72);
    } else {
        toward(&mut layer.pixmap, haze, BACK_ROW_AIR);
    }
    if dark {
        let lamp = art::hex(0xffd88a);
        for (x, base, w) in &windows {
            let s = painter::hash2(*x as i32, 37, seed);
            if s.is_multiple_of(3) {
                continue;
            }
            let wx = x + (s % 5) as f32 / 5.0 * w * 0.5 - w * 0.25;
            let wy = base - h * (0.3 + (s >> 4) as f32 % 3.0 * 0.12);
            layer.soft(wx, wy, h * 0.12, h * 0.12, h * 0.1, lamp.opacity(0.35));
            art::rect(
                &mut layer,
                wx - h * 0.035,
                wy - h * 0.04,
                h * 0.07,
                h * 0.08,
                0.5,
                lamp.opacity(0.9),
            );
        }
    }
    let origin = canvas.origin;
    canvas.draw(&layer.pixmap, origin.0, origin.1, sk::BlendMode::SourceOver);
}

/// Takes every colour of a picture `share` of the way toward `ink`,
/// keeping its shape: atmospheric perspective.
pub(super) fn toward(pixmap: &mut sk::Pixmap, ink: Hsla, share: f32) {
    let target = ink.to_rgb();
    let (r, g, b) = (target.r * 255.0, target.g * 255.0, target.b * 255.0);
    let (pixels, _) = pixmap.data_mut().as_chunks_mut::<4>();
    for pixel in pixels {
        let a = pixel[3] as f32;
        if a == 0.0 {
            continue;
        }
        // Premultiplied: mix toward the ink at the same coverage.
        for (channel, aim) in pixel[..3].iter_mut().zip([r, g, b]) {
            let value = *channel as f32;
            *channel = (value + (aim * a / 255.0 - value) * share)
                .round()
                .clamp(0.0, a) as u8;
        }
    }
}

/// Fades a picture in downward: gone above row `from`, whole below row
/// `to` (in its own pixels). The top of a nearer row of a folded postcard,
/// so its ground runs on from the row behind without a seam.
pub(super) fn fade_in_down(pixmap: &mut sk::Pixmap, from: f32, to: f32) {
    let width = pixmap.width() as usize;
    let span = (to - from).max(1.0);
    for (row, line) in pixmap.data_mut().chunks_exact_mut(width * 4).enumerate() {
        let t = ((row as f32 + 0.5 - from) / span).clamp(0.0, 1.0);
        let keep = t * t * (3.0 - 2.0 * t);
        if keep >= 0.999 {
            continue;
        }
        let k = (keep * 256.0) as u32;
        for value in line.iter_mut() {
            *value = ((*value as u32 * k) >> 8) as u8;
        }
    }
}
