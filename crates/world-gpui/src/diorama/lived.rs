//! A harbour lived in from its first day: a couple of rowboats moored off
//! the quay, a handcart of crates left on the stones, washing out on a line
//! between the houses. They belong to nobody the World records, change
//! nothing, and are drawn with the scene's own art where there is room for
//! them, never over what the World has built or the plots it offers, so a
//! new harbour is not bare while its town is still to come.
//!
//! Where they stand is seeded by the place, so the same harbour always has
//! the same boats, cart and washing, and a still picture never changes.

use super::*;

/// Where something stands: across, down, half its width, and whether it
/// is a building.
type Taken = (f32, f32, f32, bool);

/// What a lived-in harbour has about it, over the still layers and behind
/// the things and people the World places. `screen` maps a stage point to
/// the window.
pub(super) fn paint_harbour_life(
    window: &mut dyn Brush,
    frame: &Frame,
    screen: &dyn Fn(f32, f32) -> (f32, f32),
    seen: &dyn Fn(f32, f32) -> bool,
    light: [f32; 3],
) {
    if frame.setting != art::Setting::Harbour || !frame.water {
        return;
    }
    let z = frame.camera.zoom;
    let t = if frame.still { 0.0 } else { frame.seconds };
    let seed = seed_of_scenery(&frame.scenery);
    let views = (frame.width / frame.view_w.max(1.0)).ceil().max(1.0) as i32;
    let taken = taken(frame);
    let fig = frame.figure_h;
    // Clear of whatever stands in the same band of the stage, near `y`,
    // and with `tall` of every building too (washing stands up among them).
    let clear = |x: f32, y: f32, half: f32, tall: bool| {
        x - half > 0.0
            && x + half < frame.width
            && taken.iter().all(|(at, band, reach, building)| {
                let apart = (at - x).abs() > reach + half;
                apart || (!(tall && *building) && (band - y).abs() > fig * 0.7)
            })
    };
    let contact = {
        let [r, g, b] = shadow_ink(frame.hour);
        Hsla::from(gpui::Rgba { r, g, b, a: 1.0 }).opacity(0.24)
    };
    let mut tinted = Tint::new(window, light);

    // Washing on a line in a gap between the houses of the street.
    let span = fig * 1.5;
    for view in 0..views {
        let pick = painter::hash2(view, 11, seed);
        let x = (view as f32 + 0.25 + (pick % 500) as f32 / 1000.0) * frame.view_w;
        if !clear(x, frame.base, span * 0.6, true) || !seen(x, span * z) {
            continue;
        }
        let (sx, ground) = screen(x, frame.base + fig * 0.08);
        let (w, tall) = (span * z, fig * 0.95 * z);
        let (a, b) = (sx - w / 2.0, sx + w / 2.0);
        let wood = art::hex(0x7a5a3e);
        for post in [a, b] {
            tinted.soft(post, ground, 3.0 * z, 1.2 * z, 1.5 * z, contact);
            tinted.rect(post - 1.1 * z, ground - tall, 2.2 * z, tall, 0.8, wood);
        }
        let line_y = ground - tall + 2.0 * z;
        let sag = tall * 0.06;
        let mut line = Shape::new();
        line.move_to(a, line_y)
            .curve_to(b, line_y, sx, line_y + sag * 2.0);
        tinted.stroke(&line, 0.8 * z, art::hex(0xe8e0cc));
        // Three or four pieces, pegged on and moving a little in the wind.
        let inks = [0xf3efe6_u32, 0xc9d6e3, 0xe8c9b8, 0xd9cf9a, 0xb8c9b0];
        let count = 3 + (pick >> 12) as usize % 2;
        let blow = wind(frame);
        for piece in 0..count {
            let u = (piece as f32 + 0.6) / (count as f32 + 0.2);
            let px0 = a + w * u - w * 0.08;
            let hang = line_y + sag * 4.0 * u * (1.0 - u) * 2.0;
            let (pw, ph) = (
                w * (0.13 + 0.04 * ((pick >> (piece * 3)) % 3) as f32),
                tall * (0.22 + 0.06 * ((pick >> (piece * 5)) % 3) as f32),
            );
            let lift = blow.signum() * pw * 0.08 * (t * 1.3 + piece as f32).sin();
            let ink = art::hex(inks[(pick as usize >> (piece * 2)) % inks.len()]);
            art::polygon(
                &mut tinted,
                &[
                    (px0, hang),
                    (px0 + pw, hang),
                    (px0 + pw + lift, hang + ph),
                    (px0 + lift, hang + ph),
                ],
                ink,
            );
            tinted.rect(
                px0 + lift,
                hang + ph * 0.86,
                pw,
                ph * 0.14,
                0.0,
                gpui::black().opacity(0.08),
            );
            tinted.rect(
                px0 + pw * 0.45,
                hang - 1.5 * z,
                1.4 * z,
                3.0 * z,
                0.4,
                art::hex(0xcaa679),
            );
        }
    }

    // A handcart of crates, left on the quay, in every other view.
    for view in (0..views).step_by(2) {
        let pick = painter::hash2(view, 12, seed);
        let x = (view as f32 + 0.18 + (pick % 250) as f32 / 1000.0) * frame.view_w;
        let w = fig * 1.15;
        // Clear of the things beside it, and of every building standing
        // as near as it or nearer (a tower out on its spit): a prop
        // never stands against a building's base.
        let y = frame.quay_top() + fig * 0.18;
        let free = x - w * 0.7 > 0.0
            && x + w * 0.7 < frame.width
            && taken.iter().all(|(at, band, reach, building)| {
                (at - x).abs() > reach + w * 0.7
                    || if *building {
                        *band < y - fig * 0.7
                    } else {
                        (band - y).abs() > fig * 0.7
                    }
            });
        if !free || !seen(x, w * z) {
            continue;
        }
        let (sx, base) = screen(x, y);
        let w = w * z;
        let wood = art::hex(0x8a6446);
        tinted.soft(sx, base, w * 0.55, w * 0.07, w * 0.06, contact);
        // The bed, its shafts to the ground, a wheel.
        tinted.rect(
            sx - w * 0.42,
            base - w * 0.34,
            w * 0.72,
            w * 0.08,
            1.0,
            wood,
        );
        art::line(
            &mut tinted,
            (sx + w * 0.28, base - w * 0.3),
            (sx + w * 0.58, base - w * 0.04),
            1.6 * z,
            art::shade(wood, -0.2),
        );
        art::line(
            &mut tinted,
            (sx - w * 0.4, base - w * 0.28),
            (sx - w * 0.44, base),
            1.4 * z,
            art::shade(wood, -0.2),
        );
        art::circle(
            &mut tinted,
            sx - w * 0.08,
            base - w * 0.14,
            w * 0.14,
            art::hex(0x4a3a2e),
        );
        art::circle(&mut tinted, sx - w * 0.08, base - w * 0.14, w * 0.05, wood);
        // What is on it: a couple of crates and a sack.
        let crate_ink = art::hex(0xb08a5e);
        for (dx, cw, ch, dy) in [
            (-0.36, 0.3, 0.24, 0.0),
            (-0.04, 0.26, 0.2, 0.0),
            (-0.3, 0.22, 0.18, 0.24),
        ] {
            let (cx0, top) = (sx + w * dx, base - w * (0.34 + dy + ch));
            tinted.rect(cx0, top, w * cw, w * ch, 1.0, crate_ink);
            tinted.rect(
                cx0,
                top,
                w * cw,
                w * ch * 0.18,
                0.0,
                art::shade(crate_ink, 0.12),
            );
            tinted.rect(
                cx0 + w * cw * 0.46,
                top,
                w * cw * 0.08,
                w * ch,
                0.0,
                art::shade(crate_ink, -0.22),
            );
        }
        art::ellipse(
            &mut tinted,
            sx + w * 0.2,
            base - w * 0.46,
            w * 0.1,
            w * 0.12,
            art::hex(0xd8c9a3),
        );
    }

    // Rowboats moored off the quay, riding the swell.
    let water = frame.front + (frame.height - frame.front) * 0.3;
    for view in 0..views {
        let pick = painter::hash2(view, 13, seed);
        if view > 0 && pick.is_multiple_of(3) {
            continue;
        }
        let x = (view as f32 + 0.32 + (pick % 300) as f32 / 1000.0) * frame.view_w;
        let w = fig * 1.1;
        if !clear(x, water, w * 0.7, false) || !seen(x, w * z) {
            continue;
        }
        let phase = (pick % 100) as f32;
        let bob = if frame.still {
            0.0
        } else {
            (t * 1.3 + phase).sin() * 1.6 * z
        };
        let (sx, base) = screen(x, water);
        let w = w * z;
        tinted.soft(
            sx,
            base + w * 0.04,
            w * 0.5,
            w * 0.06,
            w * 0.06,
            contact.opacity(0.12),
        );
        // Its line to a ring on the quay.
        let (_, edge) = screen(x, frame.front);
        let mut rope = Shape::new();
        rope.move_to(sx - w * 0.42, base - w * 0.2).curve_to(
            sx - w * 0.62,
            edge + 1.0 * z,
            sx - w * 0.6,
            base,
        );
        tinted.stroke(&rope, 0.8 * z, art::hex(0xcdbb95).opacity(0.8));
        let palette = Palette::of(&format!("rowboat-{view}"), false);
        let sway = if frame.still {
            0.0
        } else {
            (t * 0.9 + phase).sin()
        };
        art::paint_thing(
            &mut tinted,
            sx,
            base + bob,
            w,
            MarkShape::Boat,
            &palette,
            sway,
        );
    }
}

/// Where the World's own buildings, things and plots stand (across,
/// down, and half their width with room around them).
fn taken(frame: &Frame) -> Vec<Taken> {
    let room = frame.figure_h * 0.3;
    frame
        .buildings
        .iter()
        .map(|building| {
            // A tower's spit of rock reaches out either side of it.
            let half = if building.shape == MarkShape::Tower {
                building.w
            } else {
                building.w / 2.0
            };
            (building.x, building.base, half + room, true)
        })
        .chain(
            frame
                .things
                .iter()
                .map(|thing| (thing.x, thing.base, thing.w / 2.0 + room, false)),
        )
        .chain(
            frame
                .plots
                .iter()
                .map(|plot| (plot.x, plot.y, plot.w / 2.0 + plot.w * 0.2 + room, false)),
        )
        .collect()
}
