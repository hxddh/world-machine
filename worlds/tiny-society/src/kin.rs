//! The harbour's generations: how old everyone is, who is born here, who
//! grows up, retires, and in time dies, told in the harbour's words. The
//! mechanics are the `lives` System's.
//!
//! Nothing here is recorded for a harbour's first people: their ages are
//! worked out from who they are and the harbour's first day, so a World
//! saved before the harbour kept ages opens with everyone the age they
//! would have been. Births, comings of age and deaths are recorded from
//! then on.

use crate::story::{ADA, IVO};
use crate::{EMMA, EVAN, HARBOR, JONAS, LEO, MARA, MIA, NOAH, SOFIA};
use lives::{Kin, KinWords, Memorial};
use world_core::{EntityId, Value, WorldState};
use world_projection::{AgeStage, Look};

/// The first id a child born in the harbour takes.
pub(crate) const FIRST_CHILD: u64 = 5000;
/// The first id a memorial takes.
pub(crate) const FIRST_MEMORIAL: u64 = 5300;

/// Names for the harbour's children, never a name anyone here has.
pub(crate) const CHILD_NAMES: &[&str] = &[
    "Poppy", "Finn", "Rowan", "Elsie", "Alfie", "Nora", "Otto", "Mabel", "Jem", "Lark", "Iris",
    "Ned", "Tilly", "Arlo", "Hazel", "Rory", "Maisie", "Cal", "Willa", "Bo", "Daisy", "Gus",
    "Lottie", "Sid",
];

const WORDS: KinWords = KinWords {
    born: &[
        "{a} and {b} had a baby: {name}",
        "A baby for {a} and {b}: little {name}",
        "{name} was born to {a} and {b}",
    ],
    born_said: &[
        "Ten fingers, ten toes, and a proper set of lungs.",
        "We've a baby! Come and see {name}.",
        "{name}'s asleep at last. So is half the harbour.",
    ],
    came_of_age: &[
        "{name} came of age and took up the trade of {trade}",
        "{name} is grown now, with a trade of their own: {trade}",
    ],
    came_of_age_said: &[
        "Grown up, they tell me. I don't feel it.",
        "My own wages at last!",
        "First day at work tomorrow. Don't laugh.",
    ],
    left_home: &[
        "{name} moved into a home of their own",
        "{name} left home for a cottage of their own",
    ],
    left_home_said: &[
        "My own front door! I keep opening it.",
        "Everyone cried. So did I, a bit.",
    ],
    retired: &[
        "{name} put down their tools and retired",
        "{name} retired after a lifetime's work",
    ],
    retired_said: &[
        "No more early starts. I'll miss them, oddly.",
        "Time for the garden, and a bench in the sun.",
    ],
    died: &[
        "{name} died peacefully in their sleep, at {age}",
        "{name} slipped away quietly in the night, at {age}",
    ],
    died_said: &[
        "{name} had a good long life. The harbour won't be the same.",
        "I keep thinking I'll see {name} on the quay.",
        "{name} went peacefully. That's all anyone could ask.",
    ],
    heirloom: &["{name}'s {heirloom} passed to {heir}"],
    heirloom_said: &[
        "{name} wanted me to have the {heirloom}. I'll look after it.",
        "The {heirloom} was {name}'s. Now it's mine to keep.",
    ],
    memorial_by_player: &[
        "You put up {memorial} where you chose",
        "{memorial} stands where you put it",
    ],
    memorial_by_town: &[
        "The harbour put up {memorial}",
        "Everyone chipped in for {memorial}",
    ],
    memorial_said: &[
        "{name} would have sat there every evening.",
        "Now there's somewhere to say hello to {name}.",
    ],
    anniversary: &[
        "{heir} remembered {name} on the day",
        "{heir} left flowers for {name}",
    ],
    anniversary_said: &[
        "It's {name}'s day. I've put the kettle on for two.",
        "{name} would have laughed at us all, moping.",
        "I still hear {name} whistling on the quay.",
    ],
    remember: &[
        "{name} would have loved today.",
        "I miss {name}. Silly things, mostly.",
        "Found one of {name}'s old notes in my coat.",
        "Still set out a cup for {name}, some mornings.",
        "{name} always said the tide knows best.",
        "Walked {name}'s old way to the quay.",
    ],
};

/// Trades a child of the harbour may take up when they come of age.
const TRADES: &[(&str, &str)] = &[
    ("fisher", "fisher"),
    ("net mender", "net_mender"),
    ("boat builder", "boat_builder"),
    ("baker's apprentice", "apprentice_baker"),
    ("carpenter", "carpenter"),
    ("gardener", "gardener"),
    ("sailmaker", "sailmaker"),
    ("cook", "cook"),
];

/// How old the harbour's first people were on its first day, and anyone
/// who came to stay, by what they do: a young apprentice, an old
/// storyteller.
fn age_at_start(state: &WorldState, person: EntityId) -> Option<u64> {
    Some(match person {
        JONAS => 41,
        MARA => 44,
        LEO => 70,
        EMMA => 33,
        MIA => 15,
        NOAH => 61,
        EVAN => 28,
        SOFIA => 25,
        ADA => 24,
        IVO => 14,
        _ => {
            let job = match state.entity(person)?.component(society_basic::JOB)? {
                Value::Text(job) => job.as_str(),
                _ => return None,
            };
            let seed = lives::mix(&[person.0, 43]);
            match job {
                "apprentice_baker" => 15 + seed % 2,
                "storyteller" | "clockmaker" | "shepherd" | "beekeeper" | "bookbinder" => {
                    60 + seed % 8
                }
                _ => return None,
            }
        }
    })
}

fn newborn(_: &WorldState, _: EntityId, _: EntityId) -> Vec<(String, Value)> {
    vec![
        (society_basic::CASH.into(), Value::from(0_i64)),
        (society_basic::JOB.into(), Value::from("child")),
        ("location".into(), Value::Entity(HARBOR)),
        (crate::model::MISSED_SHIFTS.into(), Value::from(0_i64)),
    ]
}

/// The first eight grow old, but their old age is the harbour's own story
/// (Leo hands on the pub and retires to the quay), not a roll of the dice.
fn keeps(person: EntityId) -> bool {
    crate::talk::RESIDENTS.contains(&person)
}

/// Someone asking the player a question of the storyteller's.
fn busy(state: &WorldState, person: EntityId) -> bool {
    storylets::open(state, crate::story::deck())
        .into_iter()
        .any(|storylet| storylet.asker == person)
}

pub(crate) static KIN: Kin = Kin {
    year: crate::almanac::YEAR_DAYS,
    age_at_start,
    born_at: |_, _| None,
    youngest: 20,
    spread: 54,
    child_at: 3,
    teen_at: 13,
    grown_at: 18,
    elder_at: 65,
    grey_at: 55,
    stoop_at: 74,
    fertile_until: 44,
    retire_at: 67,
    frail_at: 74,
    first_child: FIRST_CHILD,
    room: 200,
    names: CHILD_NAMES,
    kind: "resident",
    newborn,
    job_key: society_basic::JOB,
    retired_job: "retired",
    learning: &["child", "student", "apprentice"],
    trades: TRADES,
    keeps,
    births_now: |_| true,
    busy,
    birth_odds: 120,
    frailty: 12,
    apart: 2,
    most_children: 3,
    heirlooms: &[
        "pocket watch",
        "brass compass",
        "knitted shawl",
        "sea chest",
        "fiddle",
        "recipe book",
        "telescope",
        "tin of buttons",
    ],
    memorials: &[
        Memorial {
            shape: "bench",
            named: "{name}'s bench",
        },
        Memorial {
            shape: "stone",
            named: "{name}'s stone",
        },
    ],
    first_memorial: FIRST_MEMORIAL,
    memorial_at: HARBOR,
    memorial_wait: 5,
    mourning: crate::story::SEASON_DAYS,
    words: &WORDS,
};

/// How someone looks, their age as well: a child of the harbour in a mix
/// of their parents' colouring, and everyone as old as they are.
pub(crate) fn look(state: &WorldState, id: EntityId) -> Option<Look> {
    let mut look = crate::talk::look(id)?;
    let parents = lives::parents(state, id);
    if let [a, b] = parents.as_slice() {
        let (a, b) = (crate::talk::look(*a), crate::talk::look(*b));
        let flip = lives::mix(&[id.0, 151]).is_multiple_of(2);
        let (first, second) = if flip { (a, b) } else { (b, a) };
        look.hair = first.and_then(|look| look.hair).or(look.hair);
        look.skin = second.and_then(|look| look.skin).or(look.skin);
        look.carries = None;
    }
    with_age(state, id, look)
}

/// A look with someone's age on it.
pub(crate) fn with_age(state: &WorldState, id: EntityId, mut look: Look) -> Option<Look> {
    let cast = crate::life::cast();
    if let Some((stage, grey, stoop)) = lives::looks_of(state, &cast, id) {
        look.age = Some(stage_of(stage));
        look.grey = grey;
        look.stoop = stoop;
        if matches!(stage, lives::Stage::Baby | lives::Stage::Child) {
            look.carries = None;
        }
    }
    Some(look)
}

pub(crate) fn stage_of(stage: lives::Stage) -> AgeStage {
    match stage {
        lives::Stage::Baby => AgeStage::Baby,
        lives::Stage::Child => AgeStage::Child,
        lives::Stage::Teen => AgeStage::Teen,
        lives::Stage::Adult => AgeStage::Adult,
        lives::Stage::Elder => AgeStage::Elder,
    }
}

/// How far through life someone is now.
pub(crate) fn stage(state: &WorldState, id: EntityId) -> Option<lives::Stage> {
    lives::looks_of(state, &crate::life::cast(), id).map(|(stage, _, _)| stage)
}

const MEMORIAL_COMMAND: &str = "tiny-society.memorial.";

/// The person a memorial command is for, and the spot the player chose.
pub(crate) fn parse_command(command_id: &str) -> Option<(EntityId, Option<u8>)> {
    let rest = command_id.strip_prefix(MEMORIAL_COMMAND)?;
    let (who, spot) = match rest.split_once('@') {
        Some((who, spot)) => (who, Some(spot.parse::<u8>().ok()?.min(100))),
        None => (rest, None),
    };
    Some((EntityId::new(who.parse().ok()?), spot))
}

/// A card to put up a bench for each person the harbour lost and has not
/// yet remembered: the player places it where they like.
pub(crate) fn commands(world: &world_core::World) -> Vec<world_projection::ProjectionCommand> {
    let state = world.state();
    lives::awaiting_memorial(state)
        .into_iter()
        .map(|who| {
            let named = lives::generations::memorial_name(state, &KIN, who, 0);
            world_projection::ProjectionCommand {
                id: format!("{MEMORIAL_COMMAND}{}", who.0),
                title: format!("A bench for {}", lives::first_name(state, who)),
                detail: "Put it wherever they'd have liked to sit".into(),
                effects: Vec::new(),
                scenery: None,
                asker: None,
                moves: Vec::new(),
                question: None,
                unavailable: None,
                hand: Some(world_projection::Hand {
                    verb: "Build".into(),
                    thing: named,
                    at: Some(world_projection::SelectionId::Entity(HARBOR)),
                    cost: None,
                }),
                preview: None,
            }
        })
        .collect()
}

/// The drawings of a memorial bench and stone.
pub(crate) fn drawings() -> Vec<world_projection::Drawing> {
    memorial_drawings("harbour", 0x7a5a3a, 0x9a9a92)
}

/// A memorial bench in wood and a standing stone, as `prefix-memorial-*`.
pub(crate) fn memorial_drawings(
    prefix: &str,
    wood: u32,
    stone: u32,
) -> Vec<world_projection::Drawing> {
    use world_projection::{DrawPart, Drawing, Ink};
    let ink = Ink::Colour;
    let dark = |colour: u32| {
        let [_, r, g, b] = colour.to_be_bytes();
        u32::from_be_bytes([0, r / 2, g / 2, b / 2])
    };
    vec![
        Drawing::new(
            format!("{prefix}-memorial-bench"),
            1.6,
            vec![
                DrawPart::rect(-0.42, 0.0, 0.05, 0.3, ink(dark(wood))),
                DrawPart::rect(0.37, 0.0, 0.05, 0.3, ink(dark(wood))),
                DrawPart::rect(-0.47, 0.28, 0.94, 0.07, ink(wood)).round(0.02),
                DrawPart::rect(-0.45, 0.45, 0.9, 0.06, ink(wood)).round(0.02),
                DrawPart::rect(-0.45, 0.58, 0.9, 0.06, ink(wood)).round(0.02),
                DrawPart::rect(-0.42, 0.33, 0.04, 0.33, ink(dark(wood))),
                DrawPart::rect(0.38, 0.33, 0.04, 0.33, ink(dark(wood))),
                DrawPart::rect(-0.08, 0.5, 0.16, 0.05, ink(0xd8c27a)),
            ],
        ),
        Drawing::new(
            format!("{prefix}-memorial-stone"),
            0.6,
            vec![
                DrawPart::ellipse(0.0, 0.02, 0.45, 0.05, Ink::Shade),
                DrawPart::rect(-0.35, 0.0, 0.7, 0.72, ink(stone)).round(0.3),
                DrawPart::rect(-0.2, 0.42, 0.4, 0.03, ink(dark(stone))),
                DrawPart::rect(-0.15, 0.32, 0.3, 0.03, ink(dark(stone))),
                DrawPart::ellipse(-0.22, 0.06, 0.09, 0.05, ink(0xe0c060)),
                DrawPart::ellipse(0.2, 0.05, 0.08, 0.05, ink(0xc86a8a)),
            ],
        ),
    ]
}
