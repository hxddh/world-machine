//! Each place's own drawings: Ares Habitat's dome and hydroponics bay and
//! its colonists, Maple Street's arcade and radio station and its people,
//! Icebridge, its Fish Vault and council hall, and its penguins.

use crate::{SLOT_A, SLOT_B, SLOT_C, SLOT_D, SLOT_E};
use days::festive::{festival_things, Festive};
use std::sync::OnceLock;
use world_core::{EntityId, World};
use world_projection::figure::{ellipse, line, polygon, rect};
use world_projection::{person_base, short_hair, DrawPart, Drawing, Ink, Rung, Stance};

/// Every drawing the Pack ships, for every seed.
pub(crate) fn drawings() -> &'static [Drawing] {
    static DRAWINGS: OnceLock<Vec<Drawing>> = OnceLock::new();
    DRAWINGS.get_or_init(|| {
        let base = person_base("person");
        let mut colonist = short_hair();
        colonist.extend(jumpsuit());
        let mut townie = short_hair();
        townie.extend(jacket());
        // Each place stands its own height beside a resident (the art
        // bible's ladder); the ice bridge lies across its width.
        vec![
            habitat().standing(Rung::Tall(2.6)),
            greenhouse().standing(Rung::Tall(2.4)),
            arcade().standing(Rung::Tall(3.0)),
            radio().standing(Rung::Tall(5.0)),
            icebridge().standing(Rung::Wide(5.0)),
            fish_vault().standing(Rung::Tall(2.6)),
            council().standing(Rung::Tall(2.6)),
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
        .chain(crate::kin::drawings())
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

/// Which drawing a memorial is drawn with, in this World's seed.
pub(crate) fn memorial_drawing(world: &World, shape: &str) -> Option<String> {
    let prefix = match crate::seed_id(world) {
        "mars-colony" => "mars",
        "1980s-town" => "town",
        "penguin-civilization" => "ice",
        _ => return None,
    };
    Some(format!("{prefix}-memorial-{shape}"))
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
    let cast = crate::life::cast(world.state());
    for id in crate::life::people(world)
        .into_iter()
        .chain(lives::children(world.state(), &cast))
    {
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

/// The kind of place each place is, to look at.
pub(crate) fn setting_of(place: crate::places::Place) -> &'static str {
    match place {
        crate::places::Place::Ares => "mars",
        crate::places::Place::Maple => "street",
        crate::places::Place::Ice => "ice",
    }
}

/// Seats made by hand that stand in a row are not all the same seat: each
/// drawing here gives way to the next along the row, so none repeats.
const SEATS: &[&[&str]] = &[&["metal-bench", "crate-seat", "rover-seat"]];

/// Varies what repeats along a row: the third bench in a row on Ares is a
/// crate or an old rover seat, never a third identical bench.
pub(crate) fn vary_seats(items: &mut [world_projection::CanvasItem]) {
    for seats in SEATS {
        let mut row = items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.art.as_deref() == Some(seats[0]))
            .filter_map(|(at, item)| Some((item.px?, at)))
            .collect::<Vec<_>>();
        row.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (index, (_, at)) in row.into_iter().enumerate() {
            items[at].art = Some(seats[index % seats.len()].into());
        }
    }
}

/// What a place's homes are drawn as: hab modules on Ares, row houses on
/// Maple Street, snow nests on the ice.
pub(crate) fn home_art(place: crate::places::Place) -> &'static str {
    match place {
        crate::places::Place::Ares => "hab-module",
        crate::places::Place::Maple => "row-house",
        crate::places::Place::Ice => "snow-nest",
    }
}

/// What the app draws each of a place's own works as, from its library
/// of drawings, by the work's id: every work its own drawing, and each
/// place's in its own clothes.
pub(crate) fn art_of_work(place: crate::places::Place, work: &str) -> Option<&'static str> {
    use crate::places::Place;
    Some(match (place, work) {
        (Place::Ares, "second_home") => "second-habitat",
        (Place::Ares, "beacon") => "beacon-mast",
        (Place::Ares, "survey") => "survey-rig",
        (Place::Maple, "second_home") => "second-street",
        (Place::Maple, "beacon") => "radio-beacon",
        (Place::Maple, "survey") => "street-survey",
        (Place::Ice, "second_home") => "second-nests",
        (Place::Ice, "beacon") => "ice-beacon",
        (Place::Ice, "survey") => "ice-survey",
        (_, "ares_dust_wall") => "sandbag-wall",
        (_, "ares_water_still") => "water-still",
        (_, "ares_mess_hall") => "mess-hall-dome",
        (_, "ares_rover_shed") => "rover-shed",
        (_, "ares_relay_hut") => "relay-hut",
        (_, "ares_seed_vault") => "seed-vault",
        (_, "ares_storm_shelter") => "storm-shelter",
        (_, "ares_solar_field") => "solar-field",
        (_, "ares_greenhouse_annexe") => "greenhouse-annexe",
        (_, "ares_track_beacons") => "track-beacons",
        (_, "ares_clinic") => "clinic-bay",
        (_, "ares_machine_shop") => "machine-shop",
        (_, "ares_landing_pad") => "landing-pad",
        (_, "ares_observatory") => "observatory-dome",
        (_, "ares_second_tank") => "water-tank",
        (_, "ares_dust_lock") => "dust-lock",
        (_, "ares_school_pod") => "school-pod",
        (_, "ares_ice_cable") => "cable-pylons",
        (_, "ares_rec_room") => "rec-dome",
        (_, "ares_radio_tower") => "radio-tower",
        (_, "ares_landing_stone") => "landing-stone",
        (_, "ares_algae_farm") => "algae-farm",
        (_, "ares_bunkhouse") => "bunkhouse",
        (_, "ares_rover_lift") => "rover-lift",
        (_, "ares_windbreak") => "windbreak-panels",
        (_, "ares_trading_post") => "trading-post",
        (_, "ares_quiet_room") => "quiet-pod",
        (_, "ares_book_shelves") => "library-module",
        (_, "ares_mine_shaft") => "mine-headframe",
        (_, "ares_dome_walkway") => "dome-walkway",
        (_, "ares_flower_dome") => "flower-dome",
        (_, "ares_welcome_arch") => "welcome-arch",
        (_, "maple_bus_shelter") => "bus-shelter",
        (_, "maple_snack_bar") => "snack-bar",
        (_, "maple_call_in_booth") => "call-in-booth",
        (_, "maple_crosswalk") => "crosswalk",
        (_, "maple_bleachers") => "bleachers",
        (_, "maple_underpass_mural") => "mural-wall",
        (_, "maple_splash_pool") => "splash-pool",
        (_, "maple_skate_floor") => "roller-floor",
        (_, "maple_darkroom") => "darkroom",
        (_, "maple_clubhouse") => "clubhouse",
        (_, "maple_tall_antenna") => "tall-antenna",
        (_, "maple_garden_plots") => "garden-plots",
        (_, "maple_park_stage") => "park-stage",
        (_, "maple_streetlights") => "streetlights",
        (_, "maple_tape_shelf") => "tape-shelf",
        (_, "maple_bike_rack") => "school-bike-rack",
        (_, "maple_dance_floor") => "dance-floor",
        (_, "maple_record_library") => "record-library",
        (_, "maple_treehouse") => "treehouse",
        (_, "maple_court") => "basketball-court",
        (_, "maple_pay_phone") => "pay-phone",
        (_, "maple_drive_in_screen") => "drive-in-screen",
        (_, "maple_study_room") => "study-room",
        (_, "maple_median_beds") => "median-beds",
        (_, "maple_soapbox_track") => "soapbox-track",
        (_, "maple_neon_sign") => "neon-sign",
        (_, "maple_diner_board") => "diner-board",
        (_, "maple_radio_van") => "radio-van",
        (_, "maple_youth_centre") => "youth-centre",
        (_, "maple_picnic_tables") => "picnic-tables",
        (_, "maple_winter_rink") => "winter-rink",
        (_, "maple_square_fountain") => "drinking-fountain",
        (_, "maple_party_speakers") => "party-speakers",
        (_, "maple_computer_room") => "computer-room",
        (_, "maple_walk_of_fame") => "walk-of-fame",
        (_, "maple_school_greenhouse") => "school-greenhouse",
        (_, "maple_bus_depot") => "bus-depot",
        (_, "maple_time_capsule") => "time-capsule",
        (_, "maple_dog_run") => "dog-run",
        (_, "maple_marquee") => "rialto-marquee",
        (_, "maple_lake_path") => "lake-path",
        (_, "maple_gazebo") => "street-gazebo",
        (_, "maple_studio") => "recording-studio",
        (_, "maple_market_stalls") => "market-stalls",
        (_, "maple_bank_clock") => "bank-clock",
        (_, "maple_skate_park") => "skate-park",
        (_, "maple_henderson_bench") => "memorial-bench",
        (_, "maple_arcade_upstairs") => "arcade-upstairs",
        (_, "maple_welcome_sign") => "welcome-sign",
        (_, "maple_transmitter") => "transmitter",
        (_, "ice_snow_wall") => "snow-wall",
        (_, "ice_bridge_hole") => "fishing-hole",
        (_, "ice_ice_house") => "vault-ice-house",
        (_, "ice_creche") => "creche",
        (_, "ice_bridge_lanterns") => "bridge-lanterns",
        (_, "ice_berg_lookout") => "berg-lookout",
        (_, "ice_sea_slide") => "sea-slide",
        (_, "ice_council_ring") => "council-ring",
        (_, "ice_kelp_beds") => "kelp-beds",
        (_, "ice_thaw_marker") => "thaw-marker",
        (_, "ice_wind_shelter") => "wind-shelter",
        (_, "ice_second_vault") => "second-vault",
        (_, "ice_rope_bridge") => "rope-bridge",
        (_, "ice_song_stone") => "song-stone",
        (_, "ice_egg_warmer") => "egg-warmer",
        (_, "ice_ridge_steps") => "ridge-steps",
        (_, "ice_deep_ledge") => "deep-ledge",
        (_, "ice_story_circle") => "story-circle",
        (_, "ice_breathing_hole") => "breathing-hole",
        (_, "ice_aurora_seat") => "aurora-seat",
        (_, "ice_nest_row") => "nest-row",
        (_, "ice_kelp_racks") => "kelp-racks",
        (_, "ice_bridge_gate") => "bridge-gate",
        (_, "ice_seal_watch") => "seal-watch",
        (_, "ice_snow_hall") => "snow-hall",
        (_, "ice_run_markers") => "run-markers",
        (_, "ice_far_lantern") => "far-lantern",
        (_, "ice_chick_slide") => "chick-slide",
        (_, "ice_bone_arch") => "bone-arch",
        (_, "ice_salt_pans") => "salt-pans",
        (_, "ice_thaw_channel") => "thaw-channel",
        (_, "ice_swim_pool") => "swim-pool",
        (_, "ice_pebble_market") => "pebble-market",
        (_, "ice_night_beacon") => "night-beacon",
        (_, "ice_elders_ramp") => "elders-ramp",
        (_, "ice_name_wall") => "name-wall",
        (_, "ice_fog_horn") => "shell-horn",
        (_, "ice_new_floe_bridge") => "floe-bridge",
        _ => return None,
    })
}

/// What the app draws something the player built on a plot or made by
/// hand as, by what it is in the place's `hands` kit. A flag and a sail
/// wear a design, and keep the shape it is painted on.
pub(crate) fn art_of(place: crate::places::Place, thing: &str) -> Option<&'static str> {
    use crate::places::Place;
    Some(match (place, thing) {
        // Ares's plots.
        (Place::Ares, "hydroponics_bay") => "hydroponics-dome",
        (Place::Ares, "infirmary") => "infirmary-dome",
        (Place::Ares, "mess_hall") => "mess-module",
        (Place::Ares, "workshop_dome") => "workshop-dome",
        (Place::Ares, "music_pod") => "music-pod",
        (Place::Ares, "schoolroom") => "schoolroom-module",
        (Place::Ares, "fern_planter") => "fern-planter",
        (Place::Ares, "control_tower") => "control-tower",
        (Place::Ares, "rover_garage") => "rover-garage",
        (Place::Ares, "radio_dish") => "radio-dish",
        (Place::Ares, "supply_shop") => "supply-module",
        (Place::Ares, "cargo_depot") => "cargo-depot",
        (Place::Ares, "landing_lights") => "landing-lights",
        (Place::Ares, "windsock") => "windsock",
        (Place::Ares, "fuel_tanks") => "fuel-tanks",
        (Place::Ares, "dust_shelter") => "dust-shelter",
        (Place::Ares, "solar_array") => "solar-array",
        (Place::Ares, "survey_station") => "survey-station",
        (Place::Ares, "ice_drill") => "ice-drill",
        (Place::Ares, "observatory") => "telescope-pad",
        (Place::Ares, "crater_bench") => "crater-bench",
        (Place::Ares, "marker_cairn") => "marker-cairn",
        (Place::Ares, "greenhouse_tent") => "greenhouse-tent",
        (Place::Ares, "weather_mast") => "weather-mast",
        (Place::Ares, "lichen_garden") => "lichen-garden",
        (Place::Ares, "low_swing") => "low-g-swing",
        (Place::Ares, "ridge_beacon") => "ridge-beacon",
        // Made by hand on Ares.
        (Place::Ares, "bench") => "metal-bench",
        (Place::Ares, "lamp") => "solar-lamp",
        (Place::Ares, "tent") => "supply-tent",
        (Place::Ares, "lights") => "fairy-lights-mars",
        (Place::Ares, "tray") => "grow-tray",
        (Place::Ares, "tree") => "dwarf-apple",
        (Place::Ares, "condenser") => "condenser",
        (Place::Ares, "swing") => "low-g-swing",
        (Place::Ares, "mist") => "mist-fountain",
        (Place::Ares, "marker") => "trail-marker",
        (Place::Ares, "seedbox") => "supply-cache",
        (Place::Ares, "founders") => "founders-statue",
        (Place::Ares, "beacon_post") => "message-post",
        (Place::Ares, "sled") => "dust-sled",
        (Place::Ares, "canopy") => "rest-canopy",
        (Place::Ares, "racks") => "planter-racks",
        (Place::Ares, "algae") => "algae-beds",
        (Place::Ares, "moss_mars") => "red-moss",
        // Maple Street's plots.
        (Place::Maple, "record_store") => "record-store",
        (Place::Maple, "diner") => "diner",
        (Place::Maple, "garage") => "garage",
        (Place::Maple, "arcade") => "arcade",
        (Place::Maple, "barber_shop") => "barber-shop",
        (Place::Maple, "video_store") => "video-store",
        (Place::Maple, "newsstand") => "newsstand",
        (Place::Maple, "bus_shelter") => "bus-shelter",
        (Place::Maple, "neon_lamp") => "neon-lamp",
        (Place::Maple, "bandstand") => "roofed-bandstand",
        (Place::Maple, "ice_cream_stand") => "ice-cream-stand",
        (Place::Maple, "town_clock") => "town-clock",
        (Place::Maple, "flower_beds") => "flower-beds",
        (Place::Maple, "quilt_shop") => "quilt-shop",
        (Place::Maple, "wishing_fountain") => "wishing-fountain",
        (Place::Maple, "gazebo") => "street-gazebo",
        (Place::Maple, "chess_tables") => "chess-tables",
        (Place::Maple, "bike_rack") => "bike-rack",
        (Place::Maple, "library") => "library",
        (Place::Maple, "gym_hall") => "gym-hall",
        (Place::Maple, "boathouse") => "boathouse",
        (Place::Maple, "bee_garden") => "bee-garden",
        (Place::Maple, "science_shed") => "science-shed",
        (Place::Maple, "treehouse") => "treehouse",
        (Place::Maple, "bleachers") => "bleachers",
        (Place::Maple, "fishing_dock") => "fishing-dock",
        (Place::Maple, "rope_swing") => "rope-swing",
        // Made by hand on Maple Street.
        (Place::Maple, "bench") => "park-bench",
        (Place::Maple, "lamp") => "street-lamp-post",
        (Place::Maple, "stand" | "hotdog") => "hot-dog-stand",
        (Place::Maple, "streamers") => "streamers",
        (Place::Maple, "lights") => "fairy-lights-street",
        (Place::Maple, "flowers") => "flower-bed",
        (Place::Maple, "maple") => "maple-tree",
        (Place::Maple, "pump") => "water-pump",
        (Place::Maple, "swing") => "playground-swing",
        (Place::Maple, "drinking") => "drinking-fountain",
        (Place::Maple, "feeder") => "bird-feeder",
        (Place::Maple, "statue") => "mayor-statue",
        (Place::Maple, "mailbox") => "mailbox",
        (Place::Maple, "windowboxes") => "window-boxes",
        (Place::Maple, "tomatoes") => "tomato-patch",
        (Place::Maple, "roses") => "rose-bed",
        // Icebridge's plots.
        (Place::Ice, "chick_nursery") => "chick-nursery",
        (Place::Ice, "song_circle") => "song-circle",
        (Place::Ice, "fish_larder") => "fish-larder",
        (Place::Ice, "lantern_ring") => "lantern-ring",
        (Place::Ice, "story_berg") => "story-berg",
        (Place::Ice, "pebble_garden") => "pebble-garden",
        (Place::Ice, "snow_house") => "snow-house",
        (Place::Ice, "ice_slide") => "ice-slide",
        (Place::Ice, "ice_market") => "ice-market",
        (Place::Ice, "carving_hall") => "carving-hall",
        (Place::Ice, "lookout_post") => "lookout-post",
        (Place::Ice, "skating_rink") => "skating-rink",
        (Place::Ice, "kelp_racks") => "kelp-racks",
        (Place::Ice, "ice_bench") => "ice-bench",
        (Place::Ice, "bell_post") => "bell-post",
        (Place::Ice, "snow_arch") => "snow-arch",
        (Place::Ice, "warming_hut") => "warming-hut",
        (Place::Ice, "kayak_shelter") => "kayak-shelter",
        (Place::Ice, "whale_watch") => "whale-watch",
        (Place::Ice, "aurora_seat") => "aurora-seat",
        (Place::Ice, "fishing_hole") => "fishing-hole",
        (Place::Ice, "moss_patch" | "moss") => "moss-patch",
        (Place::Ice, "seal_fence") => "seal-fence",
        (Place::Ice, "far_beacon") => "far-beacon",
        (Place::Ice, "snow_maze") => "snow-maze",
        (Place::Ice, "sculpture_garden") => "sculpture-garden",
        // Made by hand on the ice.
        (Place::Ice, "bench") => "ice-bench",
        (Place::Ice, "lamp") => "lantern-post",
        (Place::Ice, "stall") => "fish-stall",
        (Place::Ice, "pennants") => "pennant-line",
        (Place::Ice, "glow") => "glow-stones",
        (Place::Ice, "kelp") => "kelp-garden",
        (Place::Ice, "icewell") => "ice-well",
        (Place::Ice, "swing") => "kelp-swing",
        (Place::Ice, "geyser") => "geyser-fountain",
        (Place::Ice, "marker") => "snow-marker",
        (Place::Ice, "nest") => "nesting-box",
        (Place::Ice, "sculpture") => "ice-sculpture",
        (Place::Ice, "stone") => "message-stone",
        (Place::Ice, "hole") => "fishing-hole-bench",
        (Place::Ice, "pebbles") => "pebble-planters",
        (Place::Ice, "seaweed") => "seaweed-bed",
        (Place::Ice, "lichen") => "lichen-patch",
        _ => return None,
    })
}

/// Says what everything the player built or made is drawn as: its art
/// once it stands finished in its own shape (a seedling is still a
/// seedling).
pub(crate) fn dress_art(world: &World, items: &mut [world_projection::CanvasItem]) {
    let state = world.state();
    let Some(place) = crate::places::Place::of(state) else {
        return;
    };
    // What is still going up keeps the scaffold its plot says it has.
    for item in items.iter_mut().filter(|item| item.art.is_none()) {
        let world_projection::SelectionId::Entity(id) = item.id else {
            continue;
        };
        let Some(entity) = state.entity(id) else {
            continue;
        };
        let Some(world_core::Value::Text(thing)) = entity.component("hands.thing") else {
            continue;
        };
        if matches!(entity.component("shape"), Some(world_core::Value::Text(shape)) if shape == "sprouts")
        {
            continue;
        }
        if let Some(art) = art_of(place, thing) {
            item.art = Some(art.into());
            // Its own drawing, not a festival's stall or lantern.
            item.drawing = None;
        }
    }
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
