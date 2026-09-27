//! Each place's own drawings: Ares Habitat's dome and hydroponics bay and
//! its colonists, Maple Street's arcade and radio station and its people,
//! Icebridge, its Fish Vault and council hall, and its penguins.

use crate::{SLOT_A, SLOT_B, SLOT_C, SLOT_D, SLOT_E};
use std::sync::OnceLock;
use world_core::{EntityId, World};
use world_projection::figure::{ellipse, line, polygon, rect};
use world_projection::{person_base, short_hair, DrawPart, Drawing, Ink, Stance};

/// Every drawing the Pack ships, for every seed.
pub(crate) fn drawings() -> &'static [Drawing] {
    static DRAWINGS: OnceLock<Vec<Drawing>> = OnceLock::new();
    DRAWINGS.get_or_init(|| {
        let base = person_base("person");
        let mut colonist = short_hair();
        colonist.extend(jumpsuit());
        let mut townie = short_hair();
        townie.extend(jacket());
        vec![
            habitat(),
            greenhouse(),
            arcade(),
            radio(),
            icebridge(),
            fish_vault(),
            council(),
            base.with("colonist", colonist.clone()),
            base.with("nia", [colonist.clone(), headset()].concat()),
            base.with("tomas", [colonist, goggles()].concat()),
            base.with("townie", townie.clone()),
            base.with("lena", [big_hair(), jacket()].concat()),
            base.with("max", [townie, cap_backwards()].concat()),
            penguin(),
        ]
        .into_iter()
        .chain(festival_things(
            "mars",
            Festive {
                main: 0xd9733b,
                second: 0xeeeeea,
                pole: 0x5d6270,
                cloth: 0x3f8f8a,
            },
        ))
        .chain(festival_things(
            "town",
            Festive {
                main: 0xe0457b,
                second: 0x2bb3b1,
                pole: 0x2c2a3a,
                cloth: 0xf2c14e,
            },
        ))
        .chain(festival_things(
            "ice",
            Festive {
                main: 0xe8963a,
                second: 0xf5f8fb,
                pole: 0x2d3a4a,
                cloth: 0x7fb8d9,
            },
        ))
        .collect()
    })
}

/// Which drawing what goes up for a festival is drawn with, in this
/// World's seed.
pub(crate) fn fixture_drawing(world: &World, shape: &str) -> Option<String> {
    let prefix = match crate::seed_id(world) {
        "mars-colony" => "mars",
        "1980s-town" => "town",
        "penguin-civilization" => "ice",
        _ => return None,
    };
    matches!(shape, "flag" | "bunting" | "lantern" | "stall" | "tent")
        .then(|| format!("{prefix}-{shape}"))
}

/// Which drawing someone or somewhere is drawn with, in this World's seed.
pub(crate) fn drawing_of(world: &World, id: EntityId, person: bool) -> Option<String> {
    Some(
        match (crate::seed_id(world), id) {
            ("mars-colony", SLOT_A) => "habitat",
            ("mars-colony", SLOT_C) => "greenhouse",
            ("mars-colony", SLOT_B) => "nia",
            ("mars-colony", SLOT_E) => "tomas",
            ("mars-colony", _) if person => return Some(format!("colonist-{}", id.0)),
            ("1980s-town", SLOT_A) => "arcade",
            ("1980s-town", SLOT_C) => "radio",
            ("1980s-town", SLOT_B) => "lena",
            ("1980s-town", SLOT_E) => "max",
            ("1980s-town", _) if person => return Some(format!("townie-{}", id.0)),
            ("penguin-civilization", SLOT_A) => "icebridge",
            ("penguin-civilization", SLOT_C) => "fish-vault",
            ("penguin-civilization", SLOT_D) => "council",
            ("penguin-civilization", _) if person => return Some(format!("penguin-{}", id.0)),
            _ => return None,
        }
        .into(),
    )
}

/// Which of the generated looks someone gets, by the order they joined:
/// nobody shares a silhouette until more people have lived there than
/// there are looks.
fn variant_of(world: &World, id: EntityId, looks: u32) -> u32 {
    let rank = lives::joined_rank(world.state(), id).unwrap_or(id.0 as usize) as u32;
    (rank.wrapping_mul(7)) % looks
}

/// Every drawing the Pack draws now: its own, and a look of their own for
/// everyone living there without one.
pub(crate) fn drawings_for(world: &World) -> Vec<Drawing> {
    let mut all = drawings().to_vec();
    for id in crate::life::people(world) {
        let Some(name) = drawing_of(world, id, true) else {
            continue;
        };
        let generated = if name.starts_with("colonist-") {
            let base = person_base("person").with("colonist-base", jumpsuit());
            world_projection::person(
                name,
                variant_of(world, id, world_projection::SILHOUETTES),
                &base,
            )
        } else if name.starts_with("townie-") {
            let base = person_base("person").with("townie-base", jacket());
            world_projection::person(
                name,
                variant_of(world, id, world_projection::SILHOUETTES),
                &base,
            )
        } else if name.starts_with("penguin-") {
            penguin_of(name, variant_of(world, id, PENGUIN_LOOKS))
        } else {
            continue;
        };
        all.push(generated);
    }
    all
}

/// How many different penguins [`penguin_of`] draws.
const PENGUIN_LOOKS: u32 = 36;

/// A penguin of their own: one of four hats (or none), a bow tie, a scarf
/// or nothing at the neck, and a tuft, a crest or a smooth head.
fn penguin_of(id: String, variant: u32) -> Drawing {
    let hat_colour = colour([0xb8433a, 0x2f5d8a, 0xd9a441, 0x3c7a55][(variant / 4 % 4) as usize]);
    let mut parts = Vec::new();
    match variant % 4 {
        1 => {
            parts.push(ellipse(0.0, 9.15, 1.5, 0.7, hat_colour));
            parts.push(rect(0.3, 8.85, 1.8, 0.3, hat_colour).round(0.1).tone(-0.15));
        }
        2 => {
            parts.push(ellipse(0.0, 9.35, 1.45, 0.95, hat_colour));
            parts.push(
                rect(-1.55, 8.7, 3.1, 0.5, hat_colour)
                    .round(0.12)
                    .tone(-0.2),
            );
        }
        3 => {
            parts.push(ellipse(0.0, 9.1, 2.5, 0.32, hat_colour));
            parts.push(ellipse(0.0, 9.45, 1.3, 0.7, hat_colour).tone(-0.1));
        }
        _ => {}
    }
    match (variant / 4) % 3 {
        1 => {
            parts.push(polygon(&[(-0.9, 6.6), (0.0, 6.3), (-0.9, 6.0)], hat_colour));
            parts.push(polygon(&[(0.9, 6.6), (0.0, 6.3), (0.9, 6.0)], hat_colour));
        }
        2 => parts.push(rect(-1.8, 6.1, 3.6, 0.6, hat_colour).round(0.2).tone(0.15)),
        _ => {}
    }
    let black = colour(0x23262d);
    match (variant / 12) % 3 {
        1 => parts.push(polygon(&[(-0.3, 9.2), (0.1, 10.3), (0.4, 9.2)], black)),
        2 => {
            for x in [-0.5_f32, 0.0, 0.5] {
                parts.push(polygon(
                    &[(x - 0.2, 9.2), (x + 0.1, 10.1), (x + 0.25, 9.2)],
                    black,
                ));
            }
        }
        _ => {}
    }
    penguin().with(id, parts)
}

/// What someone is doing, as far as their drawing goes: celebrating where a
/// festival is held today, working at their work.
pub(crate) fn stance_of(world: &World, id: EntityId) -> Option<Stance> {
    let state = world.state();
    let work = crate::life::work(state, id);
    let here = lives::at(state, id).or(work)?;
    let now = world.world_time();
    let celebrating = world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time == now)
        .any(|event| {
            event.kind == "festival_held"
                && event.targets.first() == Some(&here)
                && !matches!(
                    event.payload.get("turnout"),
                    Some(world_core::Value::Text(turnout)) if turnout == "thin"
                )
        });
    if celebrating {
        Some(Stance::Celebrating)
    } else if Some(here) == work {
        Some(Stance::Working)
    } else {
        None
    }
}

// ---- People -------------------------------------------------------------

fn colour(colour: u32) -> Ink {
    Ink::Colour(colour)
}

/// A colonist's jumpsuit: a collar ring, a mission patch and a belt.
fn jumpsuit() -> Vec<DrawPart> {
    vec![
        rect(-1.2, 6.25, 2.4, 0.45, colour(0x9aa3ad)).round(0.08),
        ellipse(-0.8, 5.4, 0.35, 0.35, colour(0xe8e2d6)),
        ellipse(-0.8, 5.4, 0.2, 0.2, colour(0xc8553d)),
        rect(-1.72, 3.6, 3.44, 0.35, colour(0x3b3f4a)),
        rect(-0.25, 3.58, 0.5, 0.4, colour(0xb8bec6)),
    ]
}

/// A headset with a microphone.
fn headset() -> Vec<DrawPart> {
    vec![
        ellipse(-1.55, 8.0, 0.32, 0.45, colour(0x3b3f4a)),
        line((-1.55, 7.8), (-0.5, 7.2), 0.14, colour(0x3b3f4a)),
        ellipse(-0.45, 7.2, 0.16, 0.16, colour(0x3b3f4a)),
    ]
}

/// Goggles pushed up on the forehead.
fn goggles() -> Vec<DrawPart> {
    vec![
        rect(-1.6, 8.75, 3.2, 0.3, colour(0x3b3f4a)),
        ellipse(-0.55, 8.9, 0.42, 0.34, colour(0x7fb7c9)),
        ellipse(0.55, 8.9, 0.42, 0.34, colour(0x7fb7c9)),
    ]
}

/// A jacket with its collar up.
fn jacket() -> Vec<DrawPart> {
    vec![
        polygon(&[(-1.45, 6.5), (-0.4, 6.5), (-0.9, 5.3)], Ink::Clothes),
        polygon(&[(1.45, 6.5), (0.4, 6.5), (0.9, 5.3)], Ink::Clothes),
        polygon(&[(-1.45, 6.6), (-0.5, 6.6), (-1.0, 5.2)], Ink::Clothes).tone(0.2),
        polygon(&[(1.45, 6.6), (0.5, 6.6), (1.0, 5.2)], Ink::Clothes).tone(0.2),
        line((0.0, 6.2), (0.0, 3.1), 0.12, Ink::Clothes).tone(-0.3),
    ]
}

/// Big, teased hair and hoop earrings.
fn big_hair() -> Vec<DrawPart> {
    vec![
        ellipse(0.0, 9.35, 2.2, 1.0, Ink::Hair),
        ellipse(-1.95, 8.0, 0.8, 1.2, Ink::Hair),
        ellipse(1.95, 8.0, 0.8, 1.2, Ink::Hair),
        ellipse(0.0, 9.85, 1.7, 0.6, Ink::Hair).tone(0.1),
        ellipse(-1.6, 7.3, 0.25, 0.25, colour(0xd8b25a)),
        ellipse(1.6, 7.3, 0.25, 0.25, colour(0xd8b25a)),
    ]
}

/// A cap worn backwards.
fn cap_backwards() -> Vec<DrawPart> {
    vec![
        ellipse(0.0, 9.35, 1.62, 0.62, colour(0xc8553d)),
        rect(-2.2, 9.05, 1.2, 0.28, colour(0xb5452f)).round(0.06),
    ]
}

/// A penguin in its scarf: flippers and feet for every stance.
fn penguin() -> Drawing {
    use Stance::{
        Celebrating, LookingAround, Sitting, Standing, Stretching, Talking, Walking, Waving,
        Working,
    };
    let black = colour(0x23262d);
    let white = colour(0xf4f4f0);
    let orange = colour(0xf0a030);
    let flipper = |points: &[(f32, f32)]| polygon(points, black);
    let mut parts = vec![
        ellipse(-0.8, 0.3, 0.9, 0.3, orange).swing(0.12),
        ellipse(0.8, 0.3, 0.9, 0.3, orange).swing(-0.12),
    ];
    // Flippers behind the body, by stance.
    for side in [-1.0_f32, 1.0] {
        parts.push(
            flipper(&[(side * 1.9, 6.0), (side * 2.6, 3.0), (side * 1.9, 3.4)])
                .only(&[Standing, Walking, LookingAround, Sitting])
                .swing(side * -0.08),
        );
        parts.push(
            flipper(&[(side * 1.9, 6.4), (side * 3.4, 8.6), (side * 2.1, 5.2)])
                .only(&[Celebrating, Stretching]),
        );
    }
    parts.push(flipper(&[(-1.9, 6.0), (-2.6, 3.0), (-1.9, 3.4)]).only(&[Talking, Waving]));
    parts.push(flipper(&[(1.9, 6.0), (3.2, 7.4), (2.1, 5.0)]).only(&[Talking]));
    parts.push(flipper(&[(1.9, 6.2), (2.9, 9.4), (2.2, 5.4)]).only(&[Waving]));
    parts.extend([
        ellipse(0.0, 4.3, 2.2, 3.6, black),
        ellipse(0.0, 3.9, 1.5, 2.9, white),
        ellipse(0.0, 7.9, 1.6, 1.5, black),
        ellipse(-0.3, 7.55, 0.95, 0.85, white),
        ellipse(0.3, 7.55, 0.95, 0.85, white),
        ellipse(-0.45, 7.9, 0.16, 0.16, black),
        ellipse(0.45, 7.9, 0.16, 0.16, black),
        polygon(&[(-0.35, 7.25), (0.35, 7.25), (0.0, 6.6)], orange),
        rect(-1.7, 6.0, 3.4, 0.55, Ink::Clothes).round(0.1),
        polygon(
            &[(0.6, 6.1), (1.3, 6.1), (1.2, 4.6), (0.6, 4.8)],
            Ink::Clothes,
        )
        .tone(-0.1),
    ]);
    // Working: flippers held forward over the belly.
    for side in [-1.0_f32, 1.0] {
        parts.push(
            flipper(&[(side * 1.9, 5.8), (side * 0.4, 4.2), (side * 1.6, 4.6)]).only(&[Working]),
        );
    }
    parts.push(ellipse(0.0, 6.95, 0.25, 0.12, colour(0x7a3a2a)).only(&[Talking, Celebrating]));
    // Brows for how they feel.
    use world_projection::Mood::{Cross, Sad, Thinking};
    let brow = |from: (f32, f32), to: (f32, f32)| line(from, to, 0.18, black);
    parts.push(brow((-0.85, 8.55), (-0.2, 8.3)).feeling(&[Cross]));
    parts.push(brow((0.2, 8.3), (0.85, 8.55)).feeling(&[Cross]));
    parts.push(brow((-0.85, 8.3), (-0.2, 8.5)).feeling(&[Sad]));
    parts.push(brow((0.2, 8.5), (0.85, 8.3)).feeling(&[Sad]));
    parts.push(brow((0.2, 8.45), (0.85, 8.6)).feeling(&[Thinking]));
    Drawing::new("penguin", 0.6, parts)
}

// ---- Places -------------------------------------------------------------

/// The upper half of an ellipse standing on `y`: a dome.
fn dome(x: f32, y: f32, rx: f32, ry: f32, ink: Ink) -> DrawPart {
    let points = (0..=16)
        .map(|step| {
            let angle = std::f32::consts::PI * step as f32 / 16.0;
            (x + rx * angle.cos(), y + ry * angle.sin())
        })
        .collect::<Vec<_>>();
    DrawPart::polygon(&points, ink)
}

fn habitat() -> Drawing {
    let frame = colour(0x9aa3ad);
    let mut parts = vec![
        DrawPart::line((0.3, 0.55), (0.3, 0.95), 0.012, frame),
        DrawPart::ellipse(0.3, 0.96, 0.025, 0.012, colour(0xc8553d)),
        dome(0.0, 0.12, 0.42, 0.62, Ink::Glass).tone(0.15),
        dome(0.0, 0.12, 0.36, 0.54, Ink::Wall).tone(0.25),
    ];
    for x in [-0.24_f32, -0.08, 0.08, 0.24] {
        parts.push(DrawPart::line((x, 0.12), (x * 0.6, 0.66), 0.01, frame));
    }
    parts.push(DrawPart::line((-0.36, 0.4), (0.36, 0.4), 0.01, frame));
    parts.extend([
        DrawPart::rect(-0.5, 0.0, 1.0, 0.14, colour(0x7a7f88)).round(0.02),
        DrawPart::rect(-0.48, 0.02, 0.18, 0.2, frame).round(0.03),
        DrawPart::rect(-0.44, 0.03, 0.1, 0.15, Ink::Trim),
        DrawPart::ellipse(-0.39, 0.15, 0.03, 0.02, Ink::Glass),
        DrawPart::rect(-0.1, 0.18, 0.08, 0.06, Ink::Glass),
        DrawPart::rect(0.06, 0.2, 0.08, 0.06, Ink::Glass),
        DrawPart::rect(0.3, 0.14, 0.2, 0.03, frame),
        DrawPart::polygon(
            &[(0.34, 0.17), (0.52, 0.17), (0.5, 0.3), (0.36, 0.3)],
            colour(0x2e4a7a),
        ),
        DrawPart::line((0.43, 0.17), (0.43, 0.3), 0.006, frame),
    ]);
    Drawing::new("habitat", 1.4, parts)
}

fn greenhouse() -> Drawing {
    let frame = colour(0xb8bec6);
    let mut parts = vec![
        DrawPart::ellipse(0.0, 0.45, 0.46, 0.4, Ink::Glass).tone(0.35),
        DrawPart::rect(-0.46, 0.0, 0.92, 0.45, Ink::Glass).tone(0.35),
    ];
    for (x, h) in [
        (-0.3, 0.3),
        (-0.15, 0.42),
        (0.0, 0.34),
        (0.15, 0.46),
        (0.3, 0.28),
    ] {
        parts.push(DrawPart::line(
            (x, 0.06),
            (x, h - 0.08),
            0.012,
            colour(0x4a7a3a),
        ));
        parts.push(DrawPart::ellipse(x, h, 0.07, 0.08, colour(0x5b8c3a)));
        parts.push(DrawPart::ellipse(
            x + 0.03,
            h - 0.06,
            0.05,
            0.05,
            colour(0x6fa04a),
        ));
    }
    for x in [-0.46_f32, -0.23, 0.0, 0.23, 0.46] {
        parts.push(DrawPart::line(
            (x, 0.0),
            (x, 0.45 + (0.4 * (1.0 - (x / 0.46).powi(2)).max(0.0).sqrt())),
            0.012,
            frame,
        ));
    }
    parts.extend([
        DrawPart::line((-0.46, 0.45), (0.46, 0.45), 0.012, frame),
        DrawPart::rect(-0.48, 0.0, 0.96, 0.07, colour(0x7a7f88)),
        DrawPart::rect(-0.46, 0.04, 0.92, 0.04, colour(0x6b4a33)),
    ]);
    Drawing::new("greenhouse", 1.3, parts)
}

fn arcade() -> Drawing {
    let pink = colour(0xff5fa2);
    let cyan = colour(0x4fe0f0);
    Drawing::new(
        "arcade",
        1.1,
        vec![
            DrawPart::rect(-0.44, 0.0, 0.88, 0.68, colour(0x3a3450)),
            DrawPart::rect(-0.46, 0.66, 0.92, 0.06, colour(0x2a2540)),
            DrawPart::rect(-0.34, 0.5, 0.68, 0.12, colour(0x1e1a2e)).round(0.02),
            DrawPart::line((-0.28, 0.56), (-0.08, 0.56), 0.03, pink),
            DrawPart::line((0.02, 0.56), (0.28, 0.56), 0.03, cyan),
            DrawPart::polygon(
                &[
                    (0.0, 0.9),
                    (0.025, 0.83),
                    (0.09, 0.83),
                    (0.04, 0.79),
                    (0.06, 0.72),
                    (0.0, 0.76),
                    (-0.06, 0.72),
                    (-0.04, 0.79),
                    (-0.09, 0.83),
                    (-0.025, 0.83),
                ],
                colour(0xffd23f),
            ),
            DrawPart::rect(-0.38, 0.06, 0.46, 0.36, Ink::Glass).tone(0.1),
            DrawPart::rect(-0.34, 0.08, 0.1, 0.2, colour(0x2a2540)),
            DrawPart::rect(-0.32, 0.18, 0.06, 0.06, cyan),
            DrawPart::rect(-0.18, 0.08, 0.1, 0.2, colour(0x2a2540)),
            DrawPart::rect(-0.16, 0.18, 0.06, 0.06, pink),
            DrawPart::rect(-0.02, 0.08, 0.08, 0.2, colour(0x2a2540)),
            DrawPart::rect(0.14, 0.0, 0.2, 0.4, colour(0x1e1a2e)).round(0.02),
            DrawPart::rect(0.17, 0.04, 0.14, 0.32, Ink::Glass).tone(0.2),
            DrawPart::line((-0.44, 0.46), (0.44, 0.46), 0.015, pink),
        ],
    )
}

fn radio() -> Drawing {
    let steel = colour(0x8a8f99);
    let mut parts = vec![
        DrawPart::line((0.12, 0.3), (0.2, 0.98), 0.02, steel),
        DrawPart::line((0.36, 0.3), (0.2, 0.98), 0.02, steel),
    ];
    for step in 0..6 {
        let y = 0.36 + step as f32 * 0.1;
        let half = 0.12 * (1.0 - (y - 0.3) / 0.68);
        parts.push(DrawPart::line(
            (0.24 - half, y),
            (0.24 + half - 0.08, y + 0.08),
            0.01,
            steel,
        ));
    }
    parts.extend([
        DrawPart::ellipse(0.2, 0.99, 0.03, 0.02, colour(0xff4a3a)),
        DrawPart::rect(-0.44, 0.0, 0.62, 0.42, Ink::Wall),
        DrawPart::rect(-0.46, 0.4, 0.66, 0.05, Ink::Roof),
        DrawPart::rect(-0.36, 0.2, 0.22, 0.14, Ink::Glass),
        DrawPart::rect(-0.06, 0.0, 0.14, 0.3, Ink::Trim).round(0.02),
        DrawPart::rect(-0.38, 0.46, 0.44, 0.1, colour(0x1e1a2e)).round(0.02),
        DrawPart::ellipse(-0.3, 0.51, 0.025, 0.025, colour(0xff4a3a)),
        DrawPart::line((-0.24, 0.51), (0.0, 0.51), 0.02, colour(0xffd23f)),
    ]);
    Drawing::new("radio", 0.9, parts)
}

fn icebridge() -> Drawing {
    let ice = colour(0xbfe3f0);
    let deep = colour(0x8cc6de);
    let snow = colour(0xf7fbfd);
    let mut parts = vec![
        DrawPart::polygon(
            &[
                (-0.5, 0.0),
                (-0.36, 0.0),
                (-0.28, 0.3),
                (-0.14, 0.42),
                (0.14, 0.42),
                (0.28, 0.3),
                (0.36, 0.0),
                (0.5, 0.0),
                (0.46, 0.55),
                (-0.46, 0.55),
            ],
            ice,
        ),
        DrawPart::polygon(
            &[(-0.36, 0.0), (-0.28, 0.3), (-0.32, 0.32), (-0.42, 0.0)],
            deep,
        ),
        DrawPart::polygon(&[(0.36, 0.0), (0.28, 0.3), (0.32, 0.32), (0.42, 0.0)], deep),
        DrawPart::rect(-0.48, 0.53, 0.96, 0.08, snow).round(0.04),
    ];
    for x in [-0.2_f32, -0.08, 0.05, 0.17] {
        parts.push(DrawPart::polygon(
            &[(x - 0.02, 0.42), (x + 0.02, 0.42), (x, 0.34)],
            snow,
        ));
    }
    for x in [-0.42_f32, 0.42] {
        parts.push(DrawPart::line((x, 0.61), (x, 0.8), 0.01, colour(0x5a5a5a)));
        parts.push(DrawPart::polygon(
            &[(x, 0.8), (x + 0.07, 0.77), (x, 0.74)],
            colour(0xd64545),
        ));
    }
    Drawing::new("icebridge", 1.8, parts)
}

fn fish_vault() -> Drawing {
    let snow = colour(0xf2f7fa);
    let line_ink = colour(0xc9dbe4);
    let mut parts = vec![
        dome(-0.08, 0.0, 0.42, 0.8, snow),
        DrawPart::rect(0.2, 0.0, 0.3, 0.3, snow).round(0.06),
        dome(0.38, 0.0, 0.08, 0.22, colour(0x2a3a4a)),
    ];
    for y in [0.2_f32, 0.4, 0.6] {
        let half = 0.42 * (1.0 - (y / 0.8).powi(2)).sqrt();
        parts.push(DrawPart::line(
            (-0.08 - half, y),
            (-0.08 + half, y),
            0.01,
            line_ink,
        ));
    }
    parts.extend([
        DrawPart::line((-0.1, 0.0), (-0.1, 0.2), 0.01, line_ink),
        DrawPart::line((0.1, 0.2), (0.1, 0.4), 0.01, line_ink),
        DrawPart::ellipse(-0.08, 0.5, 0.1, 0.04, colour(0x5f8fa8)),
        DrawPart::polygon(&[(0.02, 0.5), (0.07, 0.54), (0.07, 0.46)], colour(0x5f8fa8)),
    ]);
    Drawing::new("fish-vault", 1.0, parts)
}

fn council() -> Drawing {
    let block = colour(0xe8f1f6);
    let seam = colour(0xc9dbe4);
    let mut parts = vec![
        DrawPart::rect(-0.42, 0.0, 0.84, 0.55, block),
        DrawPart::polygon(
            &[(-0.48, 0.53), (0.48, 0.53), (0.0, 0.85)],
            colour(0xf7fbfd),
        ),
    ];
    for y in [0.14_f32, 0.28, 0.42] {
        parts.push(DrawPart::line((-0.42, y), (0.42, y), 0.01, seam));
    }
    parts.extend([
        DrawPart::rect(-0.1, 0.0, 0.2, 0.3, colour(0x2a3a4a)).round(0.1),
        DrawPart::ellipse(-0.26, 0.36, 0.06, 0.05, colour(0x9fd0e6)),
        DrawPart::ellipse(0.26, 0.36, 0.06, 0.05, colour(0x9fd0e6)),
        DrawPart::line((0.0, 0.85), (0.0, 1.0), 0.012, colour(0x5a5a5a)),
        DrawPart::polygon(&[(0.0, 1.0), (0.12, 0.96), (0.0, 0.92)], colour(0x3a8fd6)),
    ]);
    Drawing::new("council", 1.1, parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inks(drawing: &Drawing, ink: Ink) -> u32 {
        match ink {
            Ink::Colour(colour) => colour,
            Ink::Wall => {
                if drawing.aspect < 0.7 {
                    0x2f7f86
                } else {
                    0xefe3cf
                }
            }
            Ink::Roof => 0x3f6a8a,
            Ink::Trim => 0x6b4a33,
            Ink::Glass => 0x5f7385,
            Ink::Clothes => 0x2f7f86,
            Ink::Hair => 0x2b1d14,
            Ink::Skin => 0xc68a5f,
            Ink::Shade => 0x000000,
        }
    }

    #[test]
    fn every_drawing_every_place_ships_can_be_drawn() {
        let drawings = drawings();
        for drawing in drawings {
            assert!(drawing.is_drawable(), "{}", drawing.id);
        }
        let ids = drawings
            .iter()
            .map(|drawing| drawing.id.as_str())
            .collect::<Vec<_>>();
        let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), ids.len());
    }

    /// Writes every place's drawings as pictures into the directory in
    /// `WORLD_MACHINE_SHEET`.
    #[test]
    #[ignore]
    fn write_sheets() {
        let dir = std::path::PathBuf::from(std::env::var("WORLD_MACHINE_SHEET").unwrap());
        let (places, people): (Vec<_>, Vec<_>) = drawings()
            .iter()
            .cloned()
            .partition(|drawing| drawing.aspect > 0.7);
        std::fs::write(
            dir.join("pocket-places.svg"),
            world_projection::contact_sheet(&places, 220.0, &inks),
        )
        .unwrap();
        for chunk in people.chunks(3) {
            std::fs::write(
                dir.join(format!("pocket-{}.svg", chunk[0].id)),
                world_projection::contact_sheet(chunk, 200.0, &inks),
            )
            .unwrap();
        }
    }
}

// ---- What goes up for a festival ------------------------------------------

/// The colours a place dresses its festivals in: the main one, the one it
/// alternates with, poles and timber, and cloth.
#[derive(Clone, Copy)]
struct Festive {
    main: u32,
    second: u32,
    pole: u32,
    cloth: u32,
}

/// A flag, bunting, lanterns, a stall and a tent, each named `{prefix}-`
/// and what it is.
fn festival_things(prefix: &str, colours: Festive) -> Vec<Drawing> {
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

    vec![flag, bunting, lanterns, stall, tent]
}
