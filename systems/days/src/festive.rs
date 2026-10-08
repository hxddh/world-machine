//! What goes up for a festival, in any town: a flag, bunting, lanterns,
//! a stall and a tent, drawn in the town's own colours.

use world_projection::{DrawPart, Drawing, Ink, Rung};

/// The colours a place dresses its festivals in: the main one, the one it
/// alternates with, poles and timber, and cloth.
#[derive(Clone, Copy)]
pub struct Festive {
    pub main: u32,
    pub second: u32,
    pub pole: u32,
    pub cloth: u32,
}

/// A flag, bunting, lanterns, a stall and a tent, each named `{prefix}-`
/// and what it is.
pub fn festival_things(prefix: &str, colours: Festive) -> Vec<Drawing> {
    let Festive {
        main,
        second,
        pole,
        cloth,
    } = colours;
    let ink = Ink::Colour;
    let name = |thing: &str| format!("{prefix}-{thing}");

    let flag = Drawing::new(
        name("flag"),
        0.45,
        vec![
            DrawPart::rect(-0.45, 0.0, 0.2, 0.04, ink(pole)),
            DrawPart::line((-0.35, 0.0), (-0.35, 0.97), 0.07, ink(pole)),
            DrawPart::ellipse(-0.35, 0.985, 0.06, 0.022, ink(second)),
            DrawPart::polygon(&[(-0.33, 0.95), (0.48, 0.84), (-0.33, 0.7)], ink(main)),
            DrawPart::polygon(&[(-0.33, 0.87), (0.2, 0.83), (-0.33, 0.78)], ink(second)),
        ],
    );

    let mut bunting = vec![
        DrawPart::line((-0.46, 0.0), (-0.46, 0.86), 0.025, ink(pole)),
        DrawPart::line((0.46, 0.0), (0.46, 0.86), 0.025, ink(pole)),
    ];
    let sag = |t: f32| 0.82 - 0.2 * (1.0 - (2.0 * t - 1.0).powi(2));
    for step in 0..8 {
        let (t0, t1) = (step as f32 / 8.0, (step + 1) as f32 / 8.0);
        bunting.push(DrawPart::line(
            (-0.46 + 0.92 * t0, sag(t0)),
            (-0.46 + 0.92 * t1, sag(t1)),
            0.008,
            ink(pole),
        ));
        let t = (step as f32 + 0.5) / 8.0;
        let (x, y) = (-0.46 + 0.92 * t, sag(t));
        let colour = [main, second, cloth][step % 3];
        bunting.push(DrawPart::polygon(
            &[(x - 0.04, y), (x + 0.04, y), (x, y - 0.17)],
            ink(colour),
        ));
    }
    let bunting = Drawing::new(name("bunting"), 2.4, bunting);

    let mut lanterns = vec![
        DrawPart::line((0.0, 0.0), (0.0, 0.72), 0.08, ink(pole)),
        DrawPart::line((-0.3, 0.71), (0.3, 0.71), 0.05, ink(pole)),
    ];
    for x in [-0.25_f32, 0.25] {
        lanterns.extend([
            DrawPart::line((x, 0.71), (x, 0.62), 0.02, ink(pole)),
            DrawPart::ellipse(x, 0.52, 0.13, 0.1, Ink::Glass).tone(0.25),
            DrawPart::rect(x - 0.08, 0.6, 0.16, 0.025, ink(main)),
            DrawPart::rect(x - 0.06, 0.41, 0.12, 0.02, ink(main)),
        ]);
    }
    let lanterns = Drawing::new(name("lantern"), 0.42, lanterns);

    let mut stall = vec![
        DrawPart::line((-0.42, 0.0), (-0.42, 0.76), 0.04, ink(pole)),
        DrawPart::line((0.42, 0.0), (0.42, 0.76), 0.04, ink(pole)),
        DrawPart::rect(-0.45, 0.0, 0.9, 0.38, ink(cloth)),
        DrawPart::rect(-0.47, 0.36, 0.94, 0.05, ink(pole)),
        DrawPart::ellipse(-0.25, 0.45, 0.08, 0.05, ink(second)),
        DrawPart::ellipse(-0.04, 0.45, 0.07, 0.045, ink(main)),
        DrawPart::ellipse(0.2, 0.45, 0.09, 0.05, ink(second)).tone(-0.2),
    ];
    for stripe in 0..6 {
        let x0 = -0.5 + stripe as f32 / 6.0;
        let colour = if stripe % 2 == 0 { main } else { second };
        stall.push(DrawPart::rect(x0, 0.72, 1.0 / 6.0, 0.18, ink(colour)));
        stall.push(DrawPart::ellipse(
            x0 + 1.0 / 12.0,
            0.72,
            1.0 / 12.0,
            0.04,
            ink(colour),
        ));
    }
    let stall = Drawing::new(name("stall"), 1.3, stall);

    let mut tent = vec![DrawPart::rect(-0.42, 0.0, 0.84, 0.46, ink(second))];
    let edges = [-0.5_f32, -0.25, 0.0, 0.25, 0.5];
    for (index, pair) in edges.windows(2).enumerate() {
        let colour = if index % 2 == 0 { main } else { second };
        tent.push(DrawPart::polygon(
            &[(pair[0], 0.45), (pair[1], 0.45), (0.0, 0.88)],
            ink(colour),
        ));
    }
    tent.extend([
        DrawPart::polygon(&[(-0.1, 0.0), (0.1, 0.0), (0.0, 0.32)], ink(pole)),
        DrawPart::line((0.0, 0.87), (0.0, 1.0), 0.02, ink(pole)),
        DrawPart::polygon(&[(0.0, 1.0), (0.13, 0.96), (0.0, 0.92)], ink(main)),
    ]);
    let tent = Drawing::new(name("tent"), 1.4, tent);

    // How tall each stands beside a resident (the art bible's ladder).
    vec![
        flag.standing(Rung::Tall(2.6)),
        bunting.standing(Rung::Tall(2.2)),
        lanterns.standing(Rung::Tall(1.8)),
        stall.standing(Rung::Tall(1.3)),
        tent.standing(Rung::Tall(2.2)),
    ]
}
