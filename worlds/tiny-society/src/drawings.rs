//! The harbour's own drawings: its lighthouse, bakery, school and pub, and
//! each of its people, dressed in what marks them out.

use crate::{BAKERY, EMMA, EVAN, HARBOR, JONAS, LEO, MARA, MIA, NOAH, PUB, SCHOOL, SOFIA};
use days::festive::{festival_things, Festive};
use std::sync::OnceLock;
use world_core::{EntityId, Value, World};
use world_projection::figure::{ellipse, line, polygon, rect};
use world_projection::{person_base, short_hair, DrawPart, Drawing, Ink, Stance};

const PERSON: &str = "harbour-folk";

/// Every drawing the harbour ships.
pub(crate) fn drawings() -> &'static [Drawing] {
    static DRAWINGS: OnceLock<Vec<Drawing>> = OnceLock::new();
    DRAWINGS.get_or_init(|| {
        let base = person_base(PERSON);
        let mut drawings = vec![
            lighthouse(),
            bakery(),
            school(),
            pub_(),
            base.with(PERSON, short_hair()),
        ];
        drawings.extend(festival_things(
            "harbour",
            Festive {
                main: 0xc0463a,
                second: 0xf4f1ea,
                pole: 0x6b4a32,
                cloth: 0x2f4a6d,
            },
        ));
        for (id, parts) in [
            ("jonas", jonas()),
            ("mara", mara()),
            ("leo", leo()),
            ("emma", emma()),
            ("mia", mia()),
            ("noah", noah()),
            ("evan", evan()),
            ("sofia", sofia()),
        ] {
            drawings.push(base.with(id, parts));
        }
        drawings.extend(crate::kin::drawings());
        drawings
    })
}

/// Which drawing the harbour draws someone or somewhere with.
pub(crate) fn drawing_of(id: EntityId, person: bool) -> Option<String> {
    Some(
        match id {
            HARBOR => "lighthouse",
            BAKERY => "bakery",
            SCHOOL => "school",
            PUB => "pub",
            JONAS => "jonas",
            MARA => "mara",
            LEO => "leo",
            EMMA => "emma",
            MIA => "mia",
            NOAH => "noah",
            EVAN => "evan",
            SOFIA => "sofia",
            _ if person => return Some(folk(id)),
            _ => return None,
        }
        .into(),
    )
}

/// The name of the drawing someone without a drawing of their own is
/// drawn with.
fn folk(id: EntityId) -> String {
    format!("{PERSON}-{}", id.0)
}

/// Which of the generated looks someone gets, by the order they joined:
/// nobody shares a silhouette until more people have lived here than there
/// are looks.
pub(crate) fn variant_of(world: &World, id: EntityId) -> u32 {
    // A child of the harbour, not yet in its everyday life, comes after
    // everyone who is.
    let state = world.state();
    let rank = lives::joined_rank(state, id)
        .or_else(|| {
            let children = lives::children(state, &crate::life::cast());
            let at = children.iter().position(|child| *child == id)?;
            let joined = state
                .entities()
                .filter(|entity| !lives::traits(state, entity.id).is_empty())
                .count();
            Some(joined + at)
        })
        .unwrap_or(id.0 as usize) as u32;
    // Spread through the looks, so neighbours in joining differ a lot.
    (rank.wrapping_mul(7)) % world_projection::SILHOUETTES
}

/// Every drawing the harbour draws now: its own, and a look of their own
/// for everyone living here without one.
pub(crate) fn drawings_for(world: &World) -> Vec<Drawing> {
    let mut all = drawings().to_vec();
    let base = person_base(PERSON);
    let children = lives::children(world.state(), &crate::life::cast());
    for id in crate::story::people(world).into_iter().chain(children) {
        if let Some(name) = drawing_of(id, true).filter(|name| name.starts_with(PERSON)) {
            all.push(world_projection::person(name, variant_of(world, id), &base));
        }
    }
    all
}

/// Which drawing what goes up for a festival is drawn with.
pub(crate) fn fixture_drawing(shape: &str) -> Option<String> {
    matches!(shape, "flag" | "bunting" | "lantern" | "stall" | "tent")
        .then(|| format!("harbour-{shape}"))
}

/// What someone is doing, as far as their drawing goes: celebrating where a
/// festival is held today, working at their work.
pub(crate) fn stance_of(world: &World, id: EntityId, work: Option<EntityId>) -> Option<Stance> {
    let state = world.state();
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
                && !matches!(event.payload.get("turnout"), Some(Value::Text(turnout)) if turnout == "thin")
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

fn dark(colour: u32) -> Ink {
    Ink::Colour(colour)
}

/// A yellow sou'wester and a beard.
fn jonas() -> Vec<DrawPart> {
    vec![
        ellipse(0.0, 7.0, 1.2, 0.85, Ink::Hair),
        ellipse(0.0, 9.15, 2.35, 0.42, dark(0xe8b33c)),
        ellipse(0.0, 9.55, 1.45, 0.8, dark(0xe8b33c)),
        rect(-1.45, 9.1, 2.9, 0.18, dark(0xc99a2c)),
    ]
}

/// A baker's hat and a white apron.
fn mara() -> Vec<DrawPart> {
    let mut parts = short_hair();
    parts.extend([
        rect(-1.15, 9.2, 2.3, 1.1, dark(0xf7f4ee)).round(0.1),
        ellipse(0.0, 10.4, 1.5, 0.6, dark(0xf7f4ee)),
        rect(-1.2, 3.1, 2.4, 2.9, dark(0xf7f4ee)).round(0.08),
        line((-1.0, 6.0), (1.0, 6.0), 0.2, dark(0xe8e2d6)),
    ]);
    parts
}

/// Hair at the sides only, a moustache and a waistcoat over a white shirt.
fn leo() -> Vec<DrawPart> {
    vec![
        ellipse(-1.45, 8.3, 0.35, 0.6, Ink::Hair),
        ellipse(1.45, 8.3, 0.35, 0.6, Ink::Hair),
        ellipse(0.0, 7.62, 0.72, 0.2, Ink::Hair),
        polygon(&[(-0.55, 6.6), (0.55, 6.6), (0.0, 4.6)], dark(0xf2efe8)),
        rect(-1.5, 3.05, 0.9, 3.4, Ink::Clothes).tone(-0.35),
        rect(0.6, 3.05, 0.9, 3.4, Ink::Clothes).tone(-0.35),
        ellipse(0.0, 5.2, 0.12, 0.12, dark(0xd8b25a)),
        ellipse(0.0, 4.4, 0.12, 0.12, dark(0xd8b25a)),
    ]
}

/// Hair in a bun, and glasses.
fn emma() -> Vec<DrawPart> {
    let mut parts = short_hair();
    parts.push(ellipse(0.0, 10.0, 0.75, 0.62, Ink::Hair));
    for side in [-1.0_f32, 1.0] {
        parts.push(ellipse(side * 0.58, 8.1, 0.42, 0.36, dark(0x2a2522)));
        parts.push(ellipse(side * 0.58, 8.1, 0.3, 0.25, Ink::Skin).tone(0.25));
        parts.push(ellipse(side * 0.58, 8.1, 0.15, 0.15, dark(0x2a2522)));
    }
    parts.push(line((-0.2, 8.15), (0.2, 8.15), 0.1, dark(0x2a2522)));
    parts
}

/// Pigtails and a satchel strap.
fn mia() -> Vec<DrawPart> {
    let mut parts = short_hair();
    for side in [-1.0_f32, 1.0] {
        parts.push(ellipse(side * 1.95, 8.4, 0.5, 0.85, Ink::Hair));
        parts.push(ellipse(side * 1.6, 9.0, 0.22, 0.22, dark(0xe06f8b)));
    }
    parts.push(line((-1.2, 6.2), (1.2, 3.8), 0.3, dark(0x8a5a33)));
    parts
}

/// A flat cap and a white beard.
fn noah() -> Vec<DrawPart> {
    vec![
        ellipse(0.0, 7.0, 1.35, 1.0, dark(0xe6e2dc)),
        ellipse(0.0, 7.45, 0.75, 0.22, dark(0xe6e2dc)),
        polygon(
            &[
                (-1.65, 9.0),
                (1.6, 9.0),
                (2.4, 8.75),
                (1.7, 9.9),
                (-1.4, 9.95),
            ],
            dark(0x6b5a48),
        ),
        rect(-1.65, 8.85, 3.3, 0.25, dark(0x5a4a3a)),
    ]
}

/// A work cap and blue overalls over his clothes.
fn evan() -> Vec<DrawPart> {
    let mut parts = short_hair();
    parts.extend([
        ellipse(0.0, 9.35, 1.6, 0.65, dark(0x8a3b2a)),
        rect(0.3, 9.0, 1.9, 0.3, dark(0x7a3222)).round(0.06),
        rect(-1.0, 3.1, 2.0, 2.7, dark(0x3a5a8a)).round(0.06),
        line((-0.8, 5.7), (-1.1, 6.4), 0.3, dark(0x3a5a8a)),
        line((0.8, 5.7), (1.1, 6.4), 0.3, dark(0x3a5a8a)),
    ]);
    parts
}

/// A red headscarf, knotted behind.
fn sofia() -> Vec<DrawPart> {
    vec![
        ellipse(0.0, 8.85, 1.78, 1.05, dark(0xc8553d)),
        polygon(&[(-1.5, 8.2), (-2.3, 7.3), (-1.2, 7.6)], dark(0xb5452f)),
        rect(-1.55, 7.7, 0.45, 1.0, Ink::Hair).round(0.08),
    ]
}

// ---- Places -------------------------------------------------------------

/// Half the lighthouse's width at a height on its tapering tower.
fn tower_half(y: f32) -> f32 {
    0.3 - (y - 0.08) * (0.1 / 0.64)
}

fn lighthouse() -> Drawing {
    let band = |low: f32, high: f32| {
        DrawPart::polygon(
            &[
                (-tower_half(low), low),
                (tower_half(low), low),
                (tower_half(high), high),
                (-tower_half(high), high),
            ],
            Ink::Colour(0xc0463a),
        )
    };
    Drawing::new(
        "lighthouse",
        0.42,
        vec![
            DrawPart::polygon(
                &[(-0.5, 0.0), (0.5, 0.0), (0.4, 0.09), (-0.44, 0.1)],
                Ink::Colour(0x6b6f76),
            ),
            DrawPart::polygon(
                &[
                    (-tower_half(0.08), 0.08),
                    (tower_half(0.08), 0.08),
                    (tower_half(0.72), 0.72),
                    (-tower_half(0.72), 0.72),
                ],
                Ink::Colour(0xf4f1ea),
            ),
            band(0.22, 0.32),
            band(0.46, 0.56),
            DrawPart::rect(-0.08, 0.08, 0.16, 0.12, Ink::Trim).round(0.06),
            DrawPart::rect(-0.045, 0.37, 0.09, 0.06, Ink::Glass),
            DrawPart::rect(-0.04, 0.61, 0.08, 0.05, Ink::Glass),
            DrawPart::rect(-0.3, 0.72, 0.6, 0.035, Ink::Colour(0x3b3f4a)),
            DrawPart::rect(-0.16, 0.755, 0.32, 0.12, Ink::Glass).tone(0.2),
            DrawPart::line((-0.06, 0.755), (-0.06, 0.875), 0.02, Ink::Colour(0x3b3f4a)),
            DrawPart::line((0.06, 0.755), (0.06, 0.875), 0.02, Ink::Colour(0x3b3f4a)),
            DrawPart::polygon(
                &[(-0.21, 0.875), (0.21, 0.875), (0.0, 0.985)],
                Ink::Colour(0xb5523b),
            ),
            DrawPart::ellipse(0.0, 0.99, 0.03, 0.012, Ink::Colour(0x3b3f4a)),
        ],
    )
}

fn bakery() -> Drawing {
    let mut parts = vec![
        DrawPart::rect(0.19, 0.72, 0.08, 0.22, Ink::Trim).tone(-0.1),
        DrawPart::rect(-0.42, 0.0, 0.84, 0.62, Ink::Wall),
        DrawPart::polygon(
            &[(-0.48, 0.6), (0.48, 0.6), (0.36, 0.86), (-0.36, 0.86)],
            Ink::Roof,
        ),
        DrawPart::rect(-0.2, 0.66, 0.4, 0.08, Ink::Colour(0x6b4a33)).round(0.02),
        DrawPart::ellipse(0.0, 0.7, 0.07, 0.025, Ink::Colour(0xe8b060)),
    ];
    // A striped awning with a scalloped edge over the window.
    parts.push(DrawPart::polygon(
        &[(-0.44, 0.52), (0.44, 0.52), (0.48, 0.42), (-0.48, 0.42)],
        Ink::Colour(0xf7f4ee),
    ));
    for stripe in 0..6 {
        let left = -0.48 + stripe as f32 * 0.16;
        parts.push(DrawPart::polygon(
            &[
                (left + 0.02, 0.52),
                (left + 0.1, 0.52),
                (left + 0.1, 0.42),
                (left + 0.0, 0.42),
            ],
            Ink::Roof,
        ));
    }
    for scallop in 0..8 {
        parts.push(DrawPart::ellipse(
            -0.42 + scallop as f32 * 0.12,
            0.42,
            0.06,
            0.03,
            Ink::Roof,
        ));
    }
    parts.extend([
        DrawPart::rect(-0.36, 0.08, 0.42, 0.28, Ink::Glass),
        DrawPart::line((-0.15, 0.08), (-0.15, 0.36), 0.015, Ink::Trim),
        DrawPart::rect(-0.38, 0.06, 0.46, 0.03, Ink::Trim),
        DrawPart::ellipse(-0.27, 0.13, 0.05, 0.03, Ink::Colour(0xd9a05b)),
        DrawPart::ellipse(-0.05, 0.13, 0.05, 0.03, Ink::Colour(0xc98a45)),
        DrawPart::rect(0.13, 0.0, 0.2, 0.36, Ink::Trim).round(0.03),
        DrawPart::rect(0.17, 0.22, 0.12, 0.1, Ink::Glass),
        DrawPart::ellipse(0.29, 0.16, 0.012, 0.012, Ink::Colour(0xd8b25a)),
    ]);
    Drawing::new("bakery", 1.15, parts)
}

fn school() -> Drawing {
    let mut parts = vec![
        DrawPart::line((0.44, 0.5), (0.44, 0.96), 0.012, Ink::Colour(0x5a5a5a)),
        DrawPart::polygon(
            &[(0.44, 0.96), (0.58, 0.915), (0.44, 0.87)],
            Ink::Colour(0xc8553d),
        ),
        DrawPart::rect(-0.07, 0.7, 0.14, 0.13, Ink::Wall),
        DrawPart::ellipse(0.0, 0.76, 0.035, 0.035, Ink::Colour(0xd8a93a)),
        DrawPart::polygon(&[(-0.1, 0.83), (0.1, 0.83), (0.0, 0.94)], Ink::Roof),
        DrawPart::rect(-0.45, 0.0, 0.9, 0.55, Ink::Wall).tone(-0.05),
        DrawPart::polygon(
            &[(-0.5, 0.53), (0.5, 0.53), (0.4, 0.72), (-0.4, 0.72)],
            Ink::Roof,
        ),
        DrawPart::ellipse(0.0, 0.625, 0.055, 0.045, Ink::Colour(0xf7f4ee)),
        DrawPart::line((0.0, 0.625), (0.0, 0.655), 0.008, Ink::Colour(0x2a2522)),
        DrawPart::line((0.0, 0.625), (0.025, 0.625), 0.008, Ink::Colour(0x2a2522)),
    ];
    for x in [-0.39, -0.23, 0.11, 0.27] {
        parts.push(DrawPart::rect(x, 0.17, 0.12, 0.24, Ink::Glass));
        parts.push(DrawPart::line(
            (x + 0.06, 0.17),
            (x + 0.06, 0.41),
            0.01,
            Ink::Wall,
        ));
        parts.push(DrawPart::rect(x - 0.01, 0.155, 0.14, 0.02, Ink::Trim));
    }
    parts.extend([
        DrawPart::rect(-0.08, 0.0, 0.16, 0.32, Ink::Trim).round(0.02),
        DrawPart::polygon(&[(-0.1, 0.32), (0.1, 0.32), (0.0, 0.38)], Ink::Trim).tone(-0.2),
        DrawPart::rect(-0.12, 0.0, 0.24, 0.03, Ink::Colour(0x9a9a9a)),
    ]);
    Drawing::new("school", 1.2, parts)
}

fn pub_() -> Drawing {
    let timber = Ink::Colour(0x4a3526);
    let mut parts = vec![
        DrawPart::rect(-0.3, 0.75, 0.08, 0.2, Ink::Trim).tone(-0.1),
        DrawPart::rect(-0.42, 0.0, 0.84, 0.6, Ink::Wall).tone(0.2),
        DrawPart::polygon(
            &[(-0.48, 0.58), (0.48, 0.58), (0.3, 0.88), (-0.3, 0.88)],
            Ink::Roof,
        )
        .tone(-0.25),
    ];
    for x in [-0.42, -0.14, 0.14, 0.42] {
        parts.push(DrawPart::line((x, 0.33), (x, 0.6), 0.025, timber));
    }
    parts.push(DrawPart::line((-0.42, 0.33), (0.42, 0.33), 0.03, timber));
    parts.push(DrawPart::line((-0.14, 0.33), (0.14, 0.6), 0.02, timber));
    parts.push(DrawPart::line((0.14, 0.33), (-0.14, 0.6), 0.02, timber));
    parts.extend([
        DrawPart::rect(-0.36, 0.08, 0.3, 0.2, Ink::Glass),
        DrawPart::line((-0.26, 0.08), (-0.26, 0.28), 0.012, timber),
        DrawPart::line((-0.16, 0.08), (-0.16, 0.28), 0.012, timber),
        DrawPart::rect(-0.38, 0.06, 0.34, 0.03, timber),
        DrawPart::rect(-0.35, 0.4, 0.16, 0.12, Ink::Glass),
        DrawPart::rect(0.19, 0.4, 0.16, 0.12, Ink::Glass),
        DrawPart::rect(0.06, 0.0, 0.17, 0.3, Ink::Colour(0x5a3b22)).round(0.04),
        DrawPart::ellipse(0.29, 0.24, 0.025, 0.03, Ink::Glass).tone(0.3),
        DrawPart::line((0.42, 0.5), (0.58, 0.5), 0.015, timber),
        DrawPart::line((0.47, 0.5), (0.47, 0.46), 0.008, Ink::Colour(0x3b3f4a)),
        DrawPart::line((0.55, 0.5), (0.55, 0.46), 0.008, Ink::Colour(0x3b3f4a)),
        DrawPart::rect(0.45, 0.32, 0.12, 0.14, Ink::Colour(0x2e4a7a)).round(0.01),
        DrawPart::rect(0.485, 0.35, 0.05, 0.07, Ink::Colour(0xe8b33c)),
    ]);
    Drawing::new("pub", 1.05, parts)
}

/// What the app draws each of the harbour's own works as, from its
/// library of drawings: every work its own drawing, so no shape stands in
/// for another.
pub(crate) fn art_of_work(work: &str) -> Option<&'static str> {
    Some(match work {
        "pier" => "new-pier",
        "lamp" => "point-lamp",
        "bandstand" => "bandstand",
        "sea_wall" => "sea-wall",
        "school_garden" => "school-garden",
        "harbour_clock" => "harbour-clock",
        "boathouse" => "boathouse",
        "fountain" => "square-fountain",
        "fishers_statue" => "fishers-statue",
        "new_well" => "roofed-well",
        "postbox" => "pillar-box",
        "birdhouses" => "birdhouses",
        "signposts" => "fingerpost",
        "quay_planters" => "quay-planters",
        "school_swings" => "swing-set",
        "lighthouse_paint" => "lighthouse",
        "lifeboat_station" => "lifeboat-station",
        "village_hall" => "village-hall",
        "bathing_huts" => "bathing-huts",
        "orchard" => "orchard",
        "smokehouse" => "smokehouse",
        "footbridge" => "footbridge",
        "telescope" => "telescope",
        "cliff_path" => "cliff-path",
        "fish_market" => "fish-market",
        "bread_oven" => "bread-oven",
        "chapel_bell" => "bell-tower",
        "puppet_theatre" => "puppet-theatre",
        "glasshouse" => "glasshouse",
        "rowing_club" => "rowing-club",
        "duck_pond" => "duck-pond",
        "cottages" => "row-cottages",
        "beacon" => "hill-beacon",
        "ferry_shelter" => "ferry-shelter",
        "picnic_tables" => "picnic-tables",
        "sundial" => "sundial",
        "dovecote" => "dovecote",
        "seal_hide" => "seal-hide",
        "herb_garden" => "herb-garden",
        "maypole" => "maypole",
        "tide_gauge" => "tide-board",
        "reading_room" => "reading-room",
        "boat_yard" => "boat-yard",
        "mural" => "harbour-mural",
        "lookout_tower" => "lookout-tower",
        "sea_pool" => "sea-pool",
        "net_loft" => "net-loft",
        "school_library" => "book-nook",
        "tea_rooms" => "tea-rooms",
        "bread_cart" => "bread-cart",
        "pub_terrace" => "pub-terrace",
        "workshop" => "workshop",
        "gull_gate" => "bin-gate",
        "paddling_pool" => "paddling-pool",
        "lantern_walk" => "lantern-walk",
        "bandstand_roof" => "roofed-bandstand",
        "herring_shed" => "herring-shed",
        "wind_break" => "windbreak",
        "chapel_windows" => "chapel",
        "jetty_ladder" => "jetty-ladder",
        "own_driftwood_bench" => "driftwood-bench",
        "own_rope_swing" => "oak-rope-swing",
        "own_painted_stones" => "painted-stones",
        "own_notice_board" => "notice-board",
        "own_herb_bed" => "herb-bed",
        "own_cairn" => "cairn",
        "own_window_boxes" => "window-box-stand",
        "own_bait_shed" => "bait-shed",
        "own_sandpit" => "sandpit",
        "own_bird_table" => "bird-table",
        "own_stepping_stones" => "stepping-stones",
        "own_book_box" => "book-box",
        "own_flag_line" => "flag-line",
        "own_net_rack" => "net-rack",
        "own_wildflowers" => "wildflowers",
        "own_fire_pit" => "fire-pit",
        "own_hen_house" => "hen-house",
        "own_lookout" => "lookout-seat",
        "own_jar_lanterns" => "jar-lanterns",
        "own_shell_path" => "shell-path",
        "own_boat_planter" => "boat-planter",
        "own_skittle_alley" => "skittle-alley",
        "own_weathervane" => "weathervane",
        "own_story_chair" => "story-chair",
        "own_bee_hives" => "beehives",
        "own_kite_hill" => "kites",
        "own_tide_pools" => "pool-name-boards",
        "own_quay_mosaic" => "pebble-mosaic",
        "own_music_shed" => "music-shed",
        "own_apple_press" => "apple-press",
        _ => return None,
    })
}

/// What the app draws something the player built on a plot or made by
/// hand as, by what it is in the `hands` kit.
pub(crate) fn art_of(thing: &str) -> Option<&'static str> {
    Some(match thing {
        // Plots on the quay.
        "boathouse" => "boathouse",
        "sail_loft" => "sail-loft",
        "crab_shack" => "crab-shack",
        "net_store" => "net-store",
        "radio_hut" => "radio-hut",
        "slipway" => "slipway",
        "fishers_shelter" => "fishers-shelter",
        "harbour_lamp" => "harbour-lamp",
        "boat_rack" => "boat-rack",
        "ice_house" => "ice-house",
        "mooring_bench" => "mooring-bench",
        // On the square.
        "bandstand" => "bandstand",
        "clock_tower" => "clock-tower",
        "bookshop" => "bookshop",
        "cheese_shop" => "cheese-shop",
        "surgery" => "surgery",
        "pottery" => "pottery",
        "gallery" => "gallery",
        "quilting_room" => "quilting-room",
        "flower_stall" => "flower-stall",
        "market_cross" => "market-cross",
        // Up the hill.
        "bee_garden" => "bee-garden",
        "sheepfold" => "sheepfold",
        "allotments" => "allotments",
        "weaving_shed" => "weaving-shed",
        "study_hut" => "study-hut",
        "story_stone" => "story-stone",
        "joinery" => "joinery",
        "windmill" => "windmill",
        "summer_house" => "summer-house",
        "rose_arbour" => "rose-arbour",
        "frog_pond" => "frog-pond",
        "hilltop_swing" => "hilltop-swing",
        // Made by hand.
        "bench" => "park-bench",
        "lamp" => "lamp-post",
        "stall" => "market-stall",
        "lanterns" => "paper-lanterns",
        "vegetables" => "vegetable-patch",
        "apple_tree" => "apple-tree",
        "well" => "wellhead",
        "swing" => "garden-swing",
        "fountain" => "wall-fountain",
        "signpost" => "signpost",
        "birdhouse" => "birdhouse",
        "statue" => "fisherman-statue",
        "postbox" => "lamp-box",
        "rowboat" => "rowing-boat",
        "picnic" => "picnic-table",
        "flowerboxes" => "flower-boxes",
        "sunflowers" => "sunflowers",
        "herbs" => "herb-pots",
        // A flag, a sign and a line of washing wear a design, and keep
        // the shape it is painted on.
        _ => return None,
    })
}

/// Says what everything the player built or made is drawn as: its art
/// once it stands finished in its own shape (a sapling is still a
/// sapling).
pub(crate) fn dress_art(world: &World, items: &mut [world_projection::CanvasItem]) {
    let state = world.state();
    // What is still going up keeps the scaffold its plot says it has.
    for item in items.iter_mut().filter(|item| item.art.is_none()) {
        let world_projection::SelectionId::Entity(id) = item.id else {
            continue;
        };
        let Some(entity) = state.entity(id) else {
            continue;
        };
        let (Some(Value::Text(thing)), shape) =
            (entity.component("hands.thing"), entity.component("shape"))
        else {
            continue;
        };
        if matches!(shape, Some(Value::Text(shape)) if shape == "sprouts") {
            continue;
        }
        if let Some(art) = art_of(thing) {
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
        let person = drawing.aspect < 0.6;
        match ink {
            Ink::Colour(colour) => colour,
            Ink::Wall => {
                if person {
                    0x3c9a8f
                } else {
                    0xefe3cf
                }
            }
            Ink::Roof => 0xb5523b,
            Ink::Trim => 0x6b4a33,
            Ink::Glass => 0x5f7385,
            Ink::Clothes => 0x3c9a8f,
            Ink::Hair => 0x5a3b22,
            Ink::Skin => 0xf0c7a2,
            Ink::Shade => 0x000000,
        }
    }

    #[test]
    fn every_drawing_the_harbour_ships_can_be_drawn() {
        let drawings = drawings();
        assert!(drawings.len() >= 13);
        for drawing in drawings {
            assert!(drawing.is_drawable(), "{}", drawing.id);
        }
        for id in [
            HARBOR, BAKERY, SCHOOL, PUB, JONAS, MARA, LEO, EMMA, MIA, NOAH, EVAN, SOFIA,
        ] {
            let name = drawing_of(id, true).unwrap();
            assert!(drawings.iter().any(|drawing| drawing.id == name), "{name}");
        }
    }

    /// Writes the harbour's drawings as pictures into the directory in
    /// `WORLD_MACHINE_SHEET`.
    #[test]
    #[ignore]
    fn write_sheets() {
        let dir = std::path::PathBuf::from(std::env::var("WORLD_MACHINE_SHEET").unwrap());
        let (places, people): (Vec<_>, Vec<_>) = drawings()
            .iter()
            .cloned()
            .partition(|drawing| drawing.aspect > 0.6 || drawing.id == "lighthouse");
        std::fs::write(
            dir.join("harbour-places.svg"),
            world_projection::contact_sheet(&places, 260.0, &inks),
        )
        .unwrap();
        for chunk in people.chunks(3) {
            let name = format!("harbour-{}.svg", chunk[0].id);
            std::fs::write(
                dir.join(name),
                world_projection::contact_sheet(chunk, 200.0, &inks),
            )
            .unwrap();
        }
    }
}
