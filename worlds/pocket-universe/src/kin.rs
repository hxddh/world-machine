//! Each place's generations, in its own words: a baby born under the dome
//! on Ares, a baby brought home on Maple Street, and chicks hatching on
//! Icebridge in the thaw. Everyone grows older, the grown take up trades,
//! and the old retire and in time die gently. The mechanics are the
//! `lives` System's.
//!
//! Nobody's age is recorded for a place's first people: it is worked out
//! from who they are and the World's first day, so a World saved before
//! ages were kept opens with everyone the age they would have been.

use crate::places::Place;
use crate::{SLOT_A, SLOT_B, SLOT_E};
use lives::{Kin, KinWords, Memorial};
use world_core::{EntityId, Value, WorldState};
use world_projection::Look;

/// The first id someone born in a place takes, and the first a memorial
/// takes.
pub(crate) const FIRST_CHILD: u64 = 3000;
pub(crate) const FIRST_MEMORIAL: u64 = 3300;

pub(crate) const MARS_CHILD_NAMES: &[&str] = &[
    "Luna", "Orion", "Vesta", "Kian", "Nova", "Remy", "Soleil", "Ezra", "Talia", "Juno",
];
pub(crate) const TOWN_CHILD_NAMES: &[&str] = &[
    "Kayla", "Jason", "Bethany", "Tyler", "Ashley", "Justin", "Amber", "Kyle", "Megan", "Cody",
];
pub(crate) const ICE_CHILD_NAMES: &[&str] = &[
    "Pippa", "Nubbin", "Floe", "Tuffet", "Mookie", "Sprat", "Wibble", "Kelp", "Dimple", "Squeak",
];

/// Every name a child can be given, in any place.
pub(crate) fn child_names() -> impl Iterator<Item = &'static str> {
    MARS_CHILD_NAMES
        .iter()
        .chain(TOWN_CHILD_NAMES)
        .chain(ICE_CHILD_NAMES)
        .copied()
}

const GROWING_UP: [&[&str]; 4] = [
    &[
        "{name} came of age and took up the trade of {trade}",
        "{name} is grown now, with a trade of their own: {trade}",
    ],
    &[
        "Grown up, they tell me. I don't feel it.",
        "My own wages at last!",
        "First day at work tomorrow. Don't laugh.",
    ],
    &[
        "{name} moved into a home of their own",
        "{name} left home for a place of their own",
    ],
    &[
        "My own front door! I keep opening it.",
        "Everyone cried. So did I, a bit.",
    ],
];

const OLD_AGE: [&[&str]; 4] = [
    &[
        "{name} put down their tools and retired",
        "{name} retired after a lifetime's work",
    ],
    &[
        "No more early starts. I'll miss them, oddly.",
        "Time to sit and watch the others work.",
    ],
    &[
        "{name} died peacefully in their sleep, at {age}",
        "{name} slipped away quietly in the night, at {age}",
    ],
    &[
        "{name} had a good long life. It won't be the same here.",
        "I keep thinking I'll see {name} about.",
        "{name} went peacefully. That's all anyone could ask.",
    ],
];

const AFTER: [&[&str]; 8] = [
    &["{name}'s {heirloom} passed to {heir}"],
    &[
        "{name} wanted me to have the {heirloom}. I'll look after it.",
        "The {heirloom} was {name}'s. Now it's mine to keep.",
    ],
    &[
        "You put up {memorial} where you chose",
        "{memorial} stands where you put it",
    ],
    &[
        "Everyone put up {memorial} together",
        "Everyone chipped in for {memorial}",
    ],
    &[
        "{name} would have liked it there.",
        "Now there's somewhere to say hello to {name}.",
    ],
    &[
        "{heir} remembered {name} on the day",
        "{heir} left something for {name}",
    ],
    &[
        "It's {name}'s day. I kept a place for them.",
        "{name} would have laughed at us all, moping.",
    ],
    &[
        "{name} would have loved today.",
        "I miss {name}. Silly things, mostly.",
        "Still save a spot for {name}, some mornings.",
        "{name} always knew what to say.",
        "Thought of {name} all day today.",
    ],
];

const fn words(born: &'static [&'static str], born_said: &'static [&'static str]) -> KinWords {
    KinWords {
        born,
        born_said,
        came_of_age: GROWING_UP[0],
        came_of_age_said: GROWING_UP[1],
        left_home: GROWING_UP[2],
        left_home_said: GROWING_UP[3],
        retired: OLD_AGE[0],
        retired_said: OLD_AGE[1],
        died: OLD_AGE[2],
        died_said: OLD_AGE[3],
        heirloom: AFTER[0],
        heirloom_said: AFTER[1],
        memorial_by_player: AFTER[2],
        memorial_by_town: AFTER[3],
        memorial_said: AFTER[4],
        anniversary: AFTER[5],
        anniversary_said: AFTER[6],
        remember: AFTER[7],
    }
}

static MARS_WORDS: KinWords = words(
    &[
        "{name} was born under the dome to {a} and {b}",
        "A baby for {a} and {b}, born on Mars: {name}",
    ],
    &[
        "Born on Mars. Imagine that.",
        "First cries under the dome. {name} has lungs.",
    ],
);
static TOWN_WORDS: KinWords = words(
    &[
        "{a} and {b} brought home a baby: {name}",
        "A baby on Maple Street: {name}, to {a} and {b}",
    ],
    &[
        "We're never sleeping again. Worth it.",
        "Come and see {name}! Quietly.",
    ],
);
static ICE_WORDS: KinWords = words(
    &[
        "A chick hatched to {a} and {b}: {name}",
        "{name} pecked out of the egg, to {a} and {b}",
    ],
    &[
        "Tiny and grey and already hungry.",
        "Listen to {name} peep!",
    ],
);

/// The people the place's own story is about, who grow old but whose old
/// age is theirs to tell.
fn keeps(person: EntityId) -> bool {
    [SLOT_B, SLOT_E, crate::story::NEWCOMER].contains(&person)
}

fn role(state: &WorldState, person: EntityId) -> Option<&str> {
    match state.entity(person)?.component("role")? {
        Value::Text(role) => Some(role),
        _ => None,
    }
}

/// How old someone was on the World's first day, by what they do: a
/// chick, a kid on a paper route, a cadet.
fn age_at_start(state: &WorldState, person: EntityId) -> Option<u64> {
    let seed = lives::mix(&[person.0, 47]);
    Some(match role(state, person)? {
        "paper-route kid" => 12 + seed % 3,
        "cadet" => 19 + seed % 3,
        "storyteller" => 12 + seed % 3,
        _ => return None,
    })
}

/// A chick hatched in an older World's thaw was born when it came.
fn born_at(state: &WorldState, person: EntityId) -> Option<i64> {
    if role(state, person)? != "chick" {
        return None;
    }
    match state.entity(person)?.component(crate::years::SINCE)? {
        Value::Integer(since) => Some(*since),
        _ => None,
    }
}

fn busy(state: &WorldState, person: EntityId) -> bool {
    storylets::open(state, crate::story::deck_ref())
        .into_iter()
        .any(|storylet| storylet.asker == person)
}

fn newborn(_: &WorldState, _: EntityId, _: EntityId) -> Vec<(String, Value)> {
    vec![
        ("role".into(), Value::from("child")),
        ("location".into(), Value::Entity(SLOT_A)),
    ]
}

fn newborn_chick(_: &WorldState, _: EntityId, _: EntityId) -> Vec<(String, Value)> {
    vec![
        ("role".into(), Value::from("chick")),
        ("location".into(), Value::Entity(SLOT_A)),
    ]
}

/// Chicks hatch in the thaw, in the month after it comes.
fn in_the_thaw(state: &WorldState) -> bool {
    let (first, every) = (25, 120);
    let period = crate::places::period(state);
    period >= first && (period - first) % every < 30
}

const HUMAN_MEMORIALS: &[Memorial] = &[
    Memorial {
        shape: "bench",
        named: "{name}'s bench",
    },
    Memorial {
        shape: "stone",
        named: "{name}'s stone",
    },
];

const ICE_MEMORIALS: &[Memorial] = &[
    Memorial {
        shape: "stone",
        named: "{name}'s cairn",
    },
    Memorial {
        shape: "bench",
        named: "{name}'s ice seat",
    },
];

pub(crate) static ARES: Kin = Kin {
    year: crate::almanac::YEAR,
    age_at_start,
    born_at,
    youngest: 24,
    spread: 40,
    child_at: 3,
    teen_at: 13,
    grown_at: 18,
    elder_at: 65,
    grey_at: 55,
    stoop_at: 74,
    fertile_until: 44,
    retire_at: 64,
    frail_at: 72,
    first_child: FIRST_CHILD,
    room: 200,
    names: MARS_CHILD_NAMES,
    kind: "person",
    newborn,
    job_key: "role",
    retired_job: "retired",
    learning: &["child", "cadet", "chick", "paper-route kid"],
    trades: &[
        ("geologist", "geologist"),
        ("medic", "medic"),
        ("engineer", "engineer"),
        ("botanist", "botanist"),
        ("pilot", "pilot"),
    ],
    keeps,
    births_now: |_| true,
    busy,
    birth_odds: 90,
    frailty: 10,
    apart: 3,
    most_children: 2,
    heirlooms: &["rock hammer", "star chart", "mission patch", "harmonica"],
    memorials: HUMAN_MEMORIALS,
    first_memorial: FIRST_MEMORIAL,
    memorial_at: SLOT_A,
    memorial_wait: 5,
    mourning: 30,
    words: &MARS_WORDS,
};

pub(crate) static MAPLE: Kin = Kin {
    youngest: 20,
    spread: 50,
    names: TOWN_CHILD_NAMES,
    trades: &[
        ("mechanic", "mechanic"),
        ("diner cook", "diner cook"),
        ("guitarist", "guitarist"),
        ("substitute teacher", "substitute teacher"),
    ],
    heirlooms: &[
        "transistor radio",
        "letterman jacket",
        "mixtape",
        "wristwatch",
    ],
    words: &TOWN_WORDS,
    ..ARES
};

pub(crate) static ICE: Kin = Kin {
    youngest: 3,
    spread: 12,
    child_at: 1,
    teen_at: 2,
    grown_at: 3,
    elder_at: 14,
    grey_at: 13,
    stoop_at: 16,
    fertile_until: 12,
    retire_at: 14,
    frail_at: 16,
    names: ICE_CHILD_NAMES,
    kind: "penguin",
    newborn: newborn_chick,
    trades: &[
        ("fisher", "fisher"),
        ("ice carver", "ice carver"),
        ("lantern tender", "lantern tender"),
        ("scout", "scout"),
    ],
    births_now: in_the_thaw,
    birth_odds: 160,
    apart: 1,
    heirlooms: &["smooth pebble", "fishing hook", "lantern", "blue feather"],
    memorials: ICE_MEMORIALS,
    words: &ICE_WORDS,
    ..ARES
};

/// How a place keeps its generations.
pub(crate) fn of(place: Place) -> &'static Kin {
    match place {
        Place::Ares => &ARES,
        Place::Maple => &MAPLE,
        Place::Ice => &ICE,
    }
}

/// How someone looks, their age as well: a child in a mix of their
/// parents' colouring.
pub(crate) fn look(world: &world_core::World, id: EntityId) -> Option<Look> {
    let state = world.state();
    let mut look = crate::talk::look(world, id)?;
    if let [a, b] = lives::parents(state, id).as_slice() {
        let (a, b) = (crate::talk::look(world, *a), crate::talk::look(world, *b));
        let flip = lives::mix(&[id.0, 151]).is_multiple_of(2);
        let (first, second) = if flip { (a, b) } else { (b, a) };
        if look.bird {
            look.clothes = first.and_then(|look| look.clothes).or(look.clothes);
        } else {
            look.hair = first.and_then(|look| look.hair).or(look.hair);
            look.skin = second.and_then(|look| look.skin).or(look.skin);
        }
        look.carries = None;
    }
    let cast = crate::life::cast(state);
    if let Some((stage, grey, stoop)) = lives::looks_of(state, &cast, id) {
        look.age = Some(match stage {
            lives::Stage::Baby => world_projection::AgeStage::Baby,
            lives::Stage::Child => world_projection::AgeStage::Child,
            lives::Stage::Teen => world_projection::AgeStage::Teen,
            lives::Stage::Adult => world_projection::AgeStage::Adult,
            lives::Stage::Elder => world_projection::AgeStage::Elder,
        });
        look.grey = grey && !look.bird;
        look.stoop = stoop;
        if matches!(stage, lives::Stage::Baby | lives::Stage::Child) {
            look.carries = None;
        }
    }
    Some(look)
}

const MEMORIAL_COMMAND: &str = "pocket-universe.memorial.";

/// The person a memorial command is for, and the spot the player chose.
pub(crate) fn parse_command(command_id: &str) -> Option<(EntityId, Option<u8>)> {
    let rest = command_id.strip_prefix(MEMORIAL_COMMAND)?;
    let (who, spot) = match rest.split_once('@') {
        Some((who, spot)) => (who, Some(spot.parse::<u8>().ok()?.min(100))),
        None => (rest, None),
    };
    Some((EntityId::new(who.parse().ok()?), spot))
}

/// A card to put up a memorial for each person the place lost and has not
/// yet remembered, where the player likes.
pub(crate) fn commands(world: &world_core::World) -> Vec<world_projection::ProjectionCommand> {
    let state = world.state();
    let Some(place) = Place::of(state) else {
        return Vec::new();
    };
    let kin = of(place);
    lives::awaiting_memorial(state)
        .into_iter()
        .map(|who| {
            let named = lives::generations::memorial_name(state, kin, who, 0);
            world_projection::ProjectionCommand {
                id: format!("{MEMORIAL_COMMAND}{}", who.0),
                title: format!("A memorial for {}", lives::first_name(state, who)),
                detail: "Put it wherever they'd have liked to be".into(),
                effects: Vec::new(),
                scenery: None,
                asker: None,
                moves: Vec::new(),
                question: None,
                unavailable: None,
                hand: Some(world_projection::Hand {
                    verb: "Build".into(),
                    thing: named,
                    at: Some(world_projection::SelectionId::Entity(SLOT_A)),
                    cost: None,
                }),
                preview: None,
            }
        })
        .collect()
}

/// The drawings of each place's memorials, as `{prefix}-memorial-*`.
pub(crate) fn drawings() -> Vec<world_projection::Drawing> {
    use world_projection::{DrawPart, Drawing, Ink};
    let mut all = Vec::new();
    for (prefix, wood, stone) in [
        ("mars", 0x8a8f99, 0xb0604a),
        ("town", 0x6b4a32, 0x9a9a92),
        ("ice", 0xcfe3f0, 0x8a96a3),
    ] {
        let ink = Ink::Colour;
        let dark = |colour: u32| {
            let [_, r, g, b] = colour.to_be_bytes();
            u32::from_be_bytes([0, r / 2, g / 2, b / 2])
        };
        all.push(Drawing::new(
            format!("{prefix}-memorial-bench"),
            1.6,
            vec![
                DrawPart::rect(-0.42, 0.0, 0.05, 0.3, ink(dark(wood))),
                DrawPart::rect(0.37, 0.0, 0.05, 0.3, ink(dark(wood))),
                DrawPart::rect(-0.47, 0.28, 0.94, 0.07, ink(wood)).round(0.02),
                DrawPart::rect(-0.45, 0.45, 0.9, 0.06, ink(wood)).round(0.02),
                DrawPart::rect(-0.45, 0.58, 0.9, 0.06, ink(wood)).round(0.02),
                DrawPart::rect(-0.08, 0.5, 0.16, 0.05, ink(0xd8c27a)),
            ],
        ));
        all.push(Drawing::new(
            format!("{prefix}-memorial-stone"),
            0.6,
            vec![
                DrawPart::ellipse(0.0, 0.02, 0.45, 0.05, Ink::Shade),
                DrawPart::rect(-0.35, 0.0, 0.7, 0.72, ink(stone)).round(0.3),
                DrawPart::rect(-0.2, 0.42, 0.4, 0.03, ink(dark(stone))),
                DrawPart::rect(-0.15, 0.32, 0.3, 0.03, ink(dark(stone))),
                DrawPart::ellipse(-0.22, 0.06, 0.09, 0.05, ink(0xe0c060)),
            ],
        ));
    }
    all
}
