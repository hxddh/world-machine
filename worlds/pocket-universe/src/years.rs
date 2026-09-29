//! What changes as each place's years turn, so no year is the one before
//! again, and each place turns in its own way on days of its own.
//!
//! On Ares the supply shuttle comes every 78 sols: crew step off, crew
//! whose tour is up go home unless the place has given them reason to
//! stay, cadets pass their exams, and what the shuttle brings depends on
//! what the habitat has built. On Maple Street the school year starts in
//! the autumn (kids move up a grade, and seniors leave for college) and
//! New Year's Eve turns 1987 into 1988: families move in if the street is
//! doing well and out if nobody gave them reason to stay. On Icebridge the
//! thaw hatches chicks to the colony's couples and fledges last year's,
//! and the freeze-up sends the unhappy off with the migrating flocks and
//! brings in whoever the full fish vault draws. Everywhere, jobs are
//! handed on, each year adds a festival chosen by what the place did the
//! year before, and when the player has lent a hand the people finish a
//! small work of their own.
//!
//! Everything here is an Action and its Event, so a World replays to the
//! same years without anything deciding them again.

use crate::places::{self, Place, Turn};
use crate::story::{NEWCOMER, STORY};
use crate::{SLOT_A, SLOT_B, SLOT_C, SLOT_E, UNIVERSE};
use calendar::Festival;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft, EventId,
    StateChange, Value, World, WorldError, WorldState,
};

/// The festivals a year on Ares can add, one a year, in the order a tie
/// breaks: for what was built, what was grown, who became friends, and a
/// quiet year.
const MARS: &[Festival] = &[
    Festival {
        id: "makers_sol",
        name: "Makers' Sol",
        day: 34,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "Makers' Sol is {days} sols off. Everyone's polishing what they built.",
        getting_ready: "Makers' Sol soon. I've buffed every weld twice.",
        told: [
            "The whole habitat toured everything built this year on Makers' Sol",
            "The habitat kept Makers' Sol",
            "Makers' Sol came, and only Nia went round the works",
        ],
        said: [
            "Look what we built, out here!",
            "Not bad for a tin can on a red rock.",
            "Just me and my wrench, then.",
        ],
    },
    Festival {
        id: "greenhouse_fair",
        name: "the Greenhouse Fair",
        day: 50,
        prepare: 4,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "tent",
        harvest: true,
        nears: "The Greenhouse Fair is {days} sols away. Everyone's fussing over their seedlings.",
        getting_ready: "Greenhouse Fair soon. My tomatoes had better behave.",
        told: [
            "Everyone brought the best of the greenhouse to the Greenhouse Fair",
            "The habitat held its Greenhouse Fair",
            "The Greenhouse Fair was one radish and a lot of dust",
        ],
        said: [
            "Biggest tomato on Mars!",
            "A fair crop, for red dirt.",
            "Next year, then.",
        ],
    },
    Festival {
        id: "long_supper",
        name: "the Long Supper",
        day: 91,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "lantern",
        harvest: false,
        nears: "The Long Supper is {days} sols away. Bring a dish, bring a crewmate.",
        getting_ready: "Long Supper soon. I've borrowed every chair in the habitat.",
        told: [
            "The whole habitat ate together at the Long Supper",
            "The habitat had its Long Supper",
            "The Long Supper was a few of us and a lot of empty chairs",
        ],
        said: [
            "Never seen so many friends round one table.",
            "Good stew, good people.",
            "More stew for us, I suppose.",
        ],
    },
    Festival {
        id: "rim_run",
        name: "the Rim Run",
        day: 110,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "flag",
        harvest: false,
        nears: "The Rim Run is {days} sols away. Suits are being patched.",
        getting_ready: "Rim Run soon. I'll beat my own time this year.",
        told: [
            "Everyone in the colony ran the crater rim in the Rim Run",
            "The habitat held its Rim Run",
            "The Rim Run was one runner and a lot of dust",
        ],
        said: [
            "Round the rim and home first!",
            "A good run, that.",
            "Well, somebody had to win.",
        ],
    },
    Festival {
        id: "stores_count",
        name: "the Stores Count",
        day: 8,
        prepare: 2,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "The Stores Count is {days} sols off. Everyone's stacking crates.",
        getting_ready: "Stores Count soon. I've counted the ration packs twice already.",
        told: [
            "The habitat counted full shelves at the Stores Count",
            "The habitat held its Stores Count",
            "The Stores Count was a short list and a long face",
        ],
        said: [
            "Shelves to the ceiling! We'll eat well this year.",
            "Enough to go round.",
            "We'll tighten our belts, then.",
        ],
    },
    Festival {
        id: "shuttle_day",
        name: "Shuttle Day",
        day: 24,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "flag",
        harvest: false,
        nears: "Shuttle Day is {days} sols off. Everyone's writing letters to send up.",
        getting_ready: "Shuttle Day soon. I've got a letter for my mother in every pocket.",
        told: [
            "The whole habitat saw its letters off on Shuttle Day",
            "The habitat kept Shuttle Day",
            "Shuttle Day came, and hardly anyone had a letter to send",
        ],
        said: [
            "Letters from every one of us, all the way to Earth!",
            "Off they go.",
            "Just my one letter, then.",
        ],
    },
    Festival {
        id: "seed_swap",
        name: "the Seed Swap",
        day: 66,
        prepare: 3,
        at: SLOT_C,
        speaker: SLOT_B,
        shape: "tent",
        harvest: true,
        nears: "The Seed Swap is {days} sols away. Everyone's labelling envelopes.",
        getting_ready: "Seed Swap soon. I've saved the best of the beans.",
        told: [
            "Every grower in the habitat traded seeds at the Seed Swap",
            "The habitat held its Seed Swap",
            "The Seed Swap was three envelopes and a lot of dust",
        ],
        said: [
            "Forty kinds of seed, and every one grown here!",
            "A fair swap.",
            "Maybe next year there'll be more to swap.",
        ],
    },
    Festival {
        id: "quiet_sol",
        name: "the Quiet Sol",
        day: 100,
        prepare: 3,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "lantern",
        harvest: false,
        nears: "The Quiet Sol is {days} sols away. Nobody's allowed to fix anything.",
        getting_ready: "Quiet Sol soon. I've hidden my wrench from myself.",
        told: [
            "The whole habitat downed tools together for the Quiet Sol",
            "The habitat kept its Quiet Sol",
            "The Quiet Sol was quiet, and a bit lonely",
        ],
        said: [
            "Nothing broke, nobody worked. Perfect.",
            "A good rest, that.",
            "Too quiet, if you ask me.",
        ],
    },
];

/// The festivals a year on Maple Street can add.
const TOWN: &[Festival] = &[
    Festival {
        id: "fix_up_day",
        name: "Fix-Up Day",
        day: 34,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "bunting",
        harvest: false,
        nears: "Fix-Up Day is {days} nights off. Everyone's showing off what they fixed up.",
        getting_ready: "Fix-Up Day soon. I've painted the fence twice.",
        told: [
            "The whole street came out on Fix-Up Day to see what got built this year",
            "Maple Street kept Fix-Up Day",
            "Fix-Up Day was a card table and a boombox",
        ],
        said: [
            "Look at this street. We did that!",
            "Not bad for one little street.",
            "Just me and the boombox, then.",
        ],
    },
    Festival {
        id: "garden_show",
        name: "the Garden Show",
        day: 50,
        prepare: 4,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "tent",
        harvest: true,
        nears: "The Garden Show is {days} nights away. Everyone's watering after dark.",
        getting_ready: "Garden Show soon. My zucchini had better behave.",
        told: [
            "Every backyard on Maple Street showed off at the Garden Show",
            "Maple Street held its Garden Show",
            "The Garden Show was two tomatoes and a lawn chair",
        ],
        said: [
            "Biggest zucchini in the county!",
            "A decent showing.",
            "Next year, then.",
        ],
    },
    Festival {
        id: "potluck",
        name: "the Potluck",
        day: 80,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "lantern",
        harvest: false,
        nears: "The Potluck is {days} nights away. Bring a dish, bring a friend.",
        getting_ready: "Potluck soon. I'm making my mom's casserole.",
        told: [
            "The whole street ate together at the Potluck",
            "Maple Street had its Potluck",
            "The Potluck was a few of us and a lot of casserole",
        ],
        said: [
            "Never seen so many friends at one table.",
            "Good food, good people.",
            "More casserole for us, I guess.",
        ],
    },
    Festival {
        id: "bike_race",
        name: "the Bike Race",
        day: 110,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "flag",
        harvest: false,
        nears: "The Bike Race is {days} nights off. Everyone's oiling their chains.",
        getting_ready: "Bike Race soon. I'm going to win this year.",
        told: [
            "Every bike on Maple Street raced round the block",
            "Maple Street held its Bike Race",
            "The Bike Race was two bikes and a flat tire",
        ],
        said: [
            "Round the block and home first!",
            "A good race, that.",
            "Well, somebody had to win.",
        ],
    },
    Festival {
        id: "record_swap",
        name: "the Record Swap",
        day: 10,
        prepare: 3,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "tent",
        harvest: false,
        nears: "The Record Swap is {days} nights off. Everyone's digging through their crates.",
        getting_ready: "Record Swap soon. I'm not parting with my Prince records. Maybe one.",
        told: [
            "Half of Maple Street crowded into K-88 for the Record Swap",
            "Maple Street held its Record Swap",
            "The Record Swap was one crate and a lot of Bee Gees",
        ],
        said: [
            "I came with ten records and left with twenty!",
            "Got a couple of good ones.",
            "Nobody wanted my Bee Gees.",
        ],
    },
    Festival {
        id: "car_wash",
        name: "the Car Wash",
        day: 42,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "bunting",
        harvest: false,
        nears: "The Car Wash is {days} nights off. Sponges and buckets, everyone.",
        getting_ready: "Car Wash soon. It's for the street fund. Bring a sponge.",
        told: [
            "Every car on Maple Street got washed for the street fund",
            "Maple Street held its Car Wash",
            "The Car Wash was one bucket and a lot of rain",
        ],
        said: [
            "Forty cars! The fund's never been so full.",
            "A few dollars for the fund.",
            "We washed my car. Twice.",
        ],
    },
    Festival {
        id: "talent_show",
        name: "the Talent Show",
        day: 58,
        prepare: 4,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "lantern",
        harvest: false,
        nears: "The Talent Show is {days} nights off. Someone's been practising the trumpet.",
        getting_ready: "Talent Show soon. I'm doing card tricks. Don't tell.",
        told: [
            "Everyone new on Maple Street got up on stage at the Talent Show",
            "Maple Street put on its Talent Show",
            "The Talent Show was one act and a lot of folding chairs",
        ],
        said: [
            "Who knew the new folks could sing like that?",
            "Good show.",
            "Just me and my card tricks.",
        ],
    },
    Festival {
        id: "snowball_fight",
        name: "the Snowball Fight",
        day: 90,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "The Snowball Fight is {days} nights off. Everyone's building forts.",
        getting_ready: "Snowball Fight soon. My fort has a moat.",
        told: [
            "The whole street took sides in the Snowball Fight",
            "Maple Street had its Snowball Fight",
            "The Snowball Fight was two kids and a lot of slush",
        ],
        said: [
            "Nobody's dry and nobody cares!",
            "Good throw, that.",
            "Well, I hit somebody.",
        ],
    },
];

/// The festivals a year on Icebridge can add.
const ICE: &[Festival] = &[
    Festival {
        id: "carving_day",
        name: "Carving Day",
        day: 34,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "Carving Day is {days} auroras off. Everyone's polishing what they built.",
        getting_ready: "Carving Day soon. I've smoothed every span of the bridge.",
        told: [
            "The whole colony waddled round everything built this year on Carving Day",
            "The colony kept Carving Day",
            "Carving Day came, and only Piko went round the works",
        ],
        said: [
            "Look what we made, all of us!",
            "Not bad for a floe.",
            "Just me and my beak, then.",
        ],
    },
    Festival {
        id: "kelp_fair",
        name: "the Kelp Fair",
        day: 50,
        prepare: 4,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "tent",
        harvest: true,
        nears: "The Kelp Fair is {days} auroras away. Everyone's tending their kelp.",
        getting_ready: "Kelp Fair soon. My kelp had better grow.",
        told: [
            "Everyone brought their best kelp to the Kelp Fair",
            "The colony held its Kelp Fair",
            "The Kelp Fair was one strand and a lot of ice",
        ],
        said: [
            "Longest kelp on the floe!",
            "A fair crop, for cold water.",
            "Next year, then.",
        ],
    },
    Festival {
        id: "great_huddle",
        name: "the Great Huddle",
        day: 91,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "lantern",
        harvest: false,
        nears: "The Great Huddle is {days} auroras away. Bring a fish, bring a friend.",
        getting_ready: "Great Huddle soon. I've saved everyone a spot.",
        told: [
            "The whole colony huddled together at the Great Huddle",
            "The colony had its Great Huddle",
            "The Great Huddle was a few of us and a lot of wind",
        ],
        said: [
            "Never been so warm in my life.",
            "Cosy, that.",
            "More room for us, I suppose.",
        ],
    },
    Festival {
        id: "slide_race",
        name: "the Slide Race",
        day: 110,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "flag",
        harvest: false,
        nears: "The Slide Race is {days} auroras off. Bellies are being polished.",
        getting_ready: "Slide Race soon. I'll beat Piko this time.",
        told: [
            "Every penguin in the colony raced down the big slope",
            "The colony held its Slide Race",
            "The Slide Race was two penguins and a lot of snow",
        ],
        said: [
            "Down the slope and home first!",
            "A good slide, that.",
            "Well, somebody had to win.",
        ],
    },
    Festival {
        id: "egg_watch",
        name: "the Egg Watch",
        day: 16,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "lantern",
        harvest: false,
        nears: "The Egg Watch is {days} auroras off. Every nest is being counted.",
        getting_ready: "Egg Watch soon. I've warmed my feet specially.",
        told: [
            "The whole colony kept watch over the new eggs at the Egg Watch",
            "The colony kept its Egg Watch",
            "The Egg Watch was one nest and a lot of wind",
        ],
        said: [
            "So many eggs! Welcome, little ones.",
            "Every egg counted.",
            "Just the one egg to watch.",
        ],
    },
    Festival {
        id: "fish_feast",
        name: "the Fish Feast",
        day: 60,
        prepare: 3,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "tent",
        harvest: true,
        nears: "The Fish Feast is {days} auroras away. The vault is being opened.",
        getting_ready: "Fish Feast soon. I've been saving the fattest herring.",
        told: [
            "The whole colony ate from a full vault at the Fish Feast",
            "The colony held its Fish Feast",
            "The Fish Feast was three herring and a lot of looking",
        ],
        said: [
            "Herring for everyone, and seconds!",
            "A good feed.",
            "We'll share what there is.",
        ],
    },
    Festival {
        id: "pebble_day",
        name: "Pebble Day",
        day: 74,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "Pebble Day is {days} auroras off. Everyone's looking for the smoothest stone.",
        getting_ready: "Pebble Day soon. I've found the perfect one. Don't ask who it's for.",
        told: [
            "Every penguin in the colony gave a friend a pebble on Pebble Day",
            "The colony kept Pebble Day",
            "Pebble Day came, and the pebbles stayed in their piles",
        ],
        said: [
            "So many pebbles, so many friends!",
            "A nice pebble, that.",
            "Nobody to give a pebble to.",
        ],
    },
    Festival {
        id: "ice_carving",
        name: "the Ice Carving",
        day: 100,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "The Ice Carving is {days} auroras off. Beaks are being sharpened.",
        getting_ready: "Ice Carving soon. I'm making a whale. A small whale.",
        told: [
            "The whole colony carved the ice into wonders at the Ice Carving",
            "The colony held its Ice Carving",
            "The Ice Carving was one lump that might be a whale",
        ],
        said: [
            "A whale, a seal and a whole bridge, in ice!",
            "Nice work.",
            "It's a whale. Probably.",
        ],
    },
];

const FESTIVALS_ADDED: &str = "years.festivals";
const TURNED: &str = "years.turned";
/// What the place has put by: stores on Ares, the street fund on Maple
/// Street, the fish vault on Icebridge. Kept on the universe, from 0 to 100.
pub(crate) const STORES: &str = "stores";
/// Where the place's stores start.
const STORES_AT_FIRST: i64 = 40;
/// How many works were finished when the stores were last counted.
const WORKS_SEEN: &str = "years.works_seen";
/// How many small works the people finished on their own, and which.
const OWN_WORKS: &str = "years.own_works";
/// What the player answered the last turn's question: `more`, `fewer`,
/// `stores`, `stay` or `go`, for the next turn to act on.
pub(crate) const ASKED: &str = "years.asked";
/// The question the place's latest turn asks, until it is answered.
pub(crate) const QUESTION: &str = "years.question";
/// How many works were finished as each of the place's own years began.
const WORKS_AT_YEAR: &str = "years.works_at_year.";
/// When someone last moved on in life: arrived, grew up, took a job on.
pub(crate) const SINCE: &str = "years.since";
/// Who holds each of the place's jobs that is handed on.
const DUTY: &str = "years.duty.";
/// How many periods after a turn its changes may still come.
const TURN_DAYS: u64 = 12;

fn seed(state: &WorldState) -> &str {
    match state
        .entity(UNIVERSE)
        .and_then(|universe| universe.component(crate::SEED))
    {
        Some(Value::Text(seed)) => seed.as_str(),
        _ => "",
    }
}

/// The festivals a place can add, one a year.
fn new_festivals(seed: &str) -> &'static [Festival] {
    match seed {
        "mars-colony" => MARS,
        "1980s-town" => TOWN,
        "penguin-civilization" => ICE,
        _ => &[],
    }
}

/// Every festival a year can add, in every place, for tests.
#[cfg(test)]
pub(crate) fn all_new_festivals() -> impl Iterator<Item = &'static Festival> {
    MARS.iter().chain(TOWN).chain(ICE)
}

fn integer(state: &WorldState, entity: EntityId, key: &str) -> Option<i64> {
    match state.entity(entity)?.component(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

/// What the player answered the last turn's question, if anything.
fn asked(state: &WorldState) -> &str {
    match state
        .entity(UNIVERSE)
        .and_then(|universe| universe.component(ASKED))
    {
        Some(Value::Text(asked)) => asked.as_str(),
        _ => "",
    }
}

/// How the pair stand: their trust or their tension, from 0 to 10.
fn bond(state: &WorldState, key: &str) -> i64 {
    integer(state, crate::RELATIONSHIP, key).unwrap_or(0)
}

/// What the place has put by, from 0 to 100.
pub(crate) fn stores(state: &WorldState) -> i64 {
    integer(state, UNIVERSE, STORES)
        .unwrap_or(STORES_AT_FIRST)
        .clamp(0, 100)
}

/// The change that moves the place's stores by `by`.
pub(crate) fn stores_by(state: &WorldState, by: i64) -> StateChange {
    set(UNIVERSE, STORES, (stores(state) + by).clamp(0, 100))
}

/// The festivals added so far, in the order the place's list has them.
pub(crate) fn added(state: &WorldState) -> Vec<&'static Festival> {
    let ids = match state
        .entity(STORY)
        .and_then(|story| story.component(FESTIVALS_ADDED))
    {
        Some(Value::List(ids)) => ids
            .iter()
            .filter_map(|id| match id {
                Value::Text(id) => Some(id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    new_festivals(seed(state))
        .iter()
        .filter(|festival| ids.iter().any(|id| id == festival.id))
        .collect()
}

/// Every festival of the place's year: the old ones and those added.
pub(crate) fn festivals(state: &WorldState, old: &'static [Festival]) -> &'static [Festival] {
    let added = added(state);
    if added.is_empty() {
        return old;
    }
    let place = seed(state);
    let mask = new_festivals(place)
        .iter()
        .enumerate()
        .filter(|(_, festival)| added.iter().any(|added| added.id == festival.id))
        .fold(0_u32, |mask, (at, _)| mask | 1 << at);
    // At most a few hundred different years in each place, each made once.
    type Years = BTreeMap<(&'static str, u32), &'static [Festival]>;
    static YEARS: OnceLock<Mutex<Years>> = OnceLock::new();
    let place: &'static str = match place {
        "mars-colony" => "mars-colony",
        "1980s-town" => "1980s-town",
        _ => "penguin-civilization",
    };
    let mut years = YEARS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    years.entry((place, mask)).or_insert_with(|| {
        let mut all = old.to_vec();
        all.extend(added.into_iter().cloned());
        all.sort_by_key(|festival| festival.day);
        Box::leak(all.into_boxed_slice())
    })
}

/// How much the year gone asked for a new festival: works finished, for
/// the days that show them off; gardens grown, for the fairs; friends
/// made, for the suppers and the pebbles; newcomers, for the days that
/// welcome them; full stores, for the days that count them; and a quiet
/// day is there for a quiet year.
fn score(state: &WorldState, festival: &str) -> i64 {
    let works = crate::story::works_finished(state);
    let friends = crate::life::people_in(state)
        .into_iter()
        .filter(|person| lives::regard(state, *person) >= lives::FOND)
        .count() as i64;
    let newcomers = arrivals(state).len() as i64;
    match festival {
        "makers_sol" | "fix_up_day" | "carving_day" | "ice_carving" => works * 2,
        "greenhouse_fair" | "garden_show" | "kelp_fair" | "seed_swap" | "fish_feast" => {
            crate::almanac::grown(state) * 3
        }
        "long_supper" | "potluck" | "great_huddle" | "record_swap" | "pebble_day" => friends,
        "shuttle_day" | "talent_show" | "egg_watch" => newcomers,
        "stores_count" | "car_wash" => stores(state) / 20,
        _ => 1,
    }
}

/// A change that comes with a year: who tells it, what they say, and what
/// it changes.
struct Turning {
    who: EntityId,
    told: String,
    said: String,
    changes: Vec<StateChange>,
}

fn set(entity: EntityId, key: &str, value: impl Into<Value>) -> StateChange {
    StateChange::SetComponent {
        entity,
        key: key.into(),
        value: value.into(),
    }
}

fn role(state: &WorldState, person: EntityId) -> Option<&str> {
    match state.entity(person)?.component("role")? {
        Value::Text(role) => Some(role),
        _ => None,
    }
}

fn living(state: &WorldState, person: EntityId) -> bool {
    state.entity(person).is_some() && !lives::gone(state, person)
}

/// Whoever came to stay and still lives there, first come first.
fn arrivals(state: &WorldState) -> Vec<EntityId> {
    crate::life::newcomers(state)
        .into_iter()
        .filter(|person| !lives::gone(state, *person))
        .collect()
}

fn name(state: &WorldState, person: EntityId) -> String {
    lives::first_name(state, person)
}

/// Picks the `n`th of a few ways of saying something, so a place's turns
/// are told differently each time they come round.
fn nth<T: Copy>(items: &[T], n: u64) -> T {
    items[(n % items.len() as u64) as usize]
}

/// What turns in the old way: the first two years' changes each place
/// had before its turns kept coming. Kept, so a World that has had them
/// does not have them again.
fn legacy_beats(place: Place) -> [&'static str; 2] {
    match place {
        Place::Maple => ["arcade", "high_school"],
        Place::Ice => ["fledged", "lantern"],
        Place::Ares => ["greenhouse", "relay"],
    }
}

fn done(state: &WorldState, beat: &str) -> bool {
    matches!(
        state
            .entity(STORY)
            .and_then(|story| story.component(&format!("{TURNED}.{beat}"))),
        Some(Value::Bool(true))
    )
}

/// What turns for one of the old beats, if the people it needs are there.
fn legacy_turning(state: &WorldState, beat: &str) -> Option<Turning> {
    let name = |person| lives::first_name(state, person);
    Some(match beat {
        // Nia's apprentice is the engineer who came to stay (Ines on
        // Ares), or failing her whoever came first.
        "greenhouse" => {
            let apprentice = [NEWCOMER]
                .into_iter()
                .filter(|person| living(state, *person))
                .chain(arrivals(state))
                .find(|person| role(state, *person) != Some("greenhouse keeper"))?;
            Turning {
                who: apprentice,
                told: format!(
                    "{} took over the greenhouse from {}",
                    name(apprentice),
                    name(SLOT_B)
                ),
                said: "The wheat is mine now. I've named every plant.".into(),
                changes: vec![
                    set(apprentice, "role", "greenhouse keeper"),
                    set(apprentice, "works_at", Value::Entity(SLOT_C)),
                ],
            }
        }
        // One of the crew who came in on the relay stays when the rest
        // rotate home.
        "relay" => {
            let stays = arrivals(state)
                .into_iter()
                .find(|person| !lives::settled(state, *person))?;
            Turning {
                who: stays,
                told: format!("The relay crew rotated, and {} stayed on", name(stays)),
                said: "The shuttle went back without me. On purpose.".into(),
                changes: vec![set(stays, lives::SETTLED, true)],
            }
        }
        // The arcade's old owner sells up, and Lena, who has worked its
        // night shift from the start, takes it on.
        "arcade" => Turning {
            who: SLOT_B,
            told: format!(
                "The arcade changed hands, and {} got the keys",
                name(SLOT_B)
            ),
            said: "My name's on the lease. My name! On a lease!".into(),
            changes: vec![
                set(SLOT_B, "role", "arcade owner"),
                set(SLOT_A, "owner", Value::Entity(SLOT_B)),
            ],
        },
        // Ray's kid lives off the street's stage; without Ray, the
        // paper-route kid who came to stay starts instead.
        "high_school" if living(state, NEWCOMER) => Turning {
            who: NEWCOMER,
            told: format!("{}'s kid started at the high school", name(NEWCOMER)),
            said: "First day of high school. The kid wore my old jacket.".into(),
            changes: vec![set(NEWCOMER, "kid", "at the high school")],
        },
        "high_school" => {
            let kid = arrivals(state)
                .into_iter()
                .find(|person| role(state, *person) == Some("paper-route kid"))?;
            Turning {
                who: kid,
                told: format!("{} started at the high school", name(kid)),
                said: "High school! I'm keeping the paper route, though.".into(),
                changes: vec![set(kid, "role", "high-schooler")],
            }
        }
        // Tuk, the youngest fisher, is the chick who fledges; without
        // Tuk, one of the colony's own chicks does.
        "fledged" if living(state, NEWCOMER) => Turning {
            who: NEWCOMER,
            told: format!(
                "{} fledged, and swims with the grown-ups now",
                name(NEWCOMER)
            ),
            said: "Real feathers! No more fluff!".into(),
            changes: vec![set(NEWCOMER, "role", "fisher")],
        },
        "fledged" => Turning {
            who: SLOT_E,
            told: "The first chick of the year fledged".into(),
            said: "One of the little ones swam out today. Didn't even look back.".into(),
            changes: Vec::new(),
        },
        // Piko, who lights the bridge, passes the lantern on: to a lantern
        // tender who came to stay, or to Tuk, or to Miri.
        "lantern" => {
            let arrivals = arrivals(state);
            let next = arrivals
                .iter()
                .copied()
                .find(|person| role(state, *person) == Some("lantern tender"))
                .or_else(|| living(state, NEWCOMER).then_some(NEWCOMER))
                .or_else(|| arrivals.first().copied())
                .unwrap_or(SLOT_E);
            Turning {
                who: SLOT_B,
                told: format!("{} passed the lantern on to {}", name(SLOT_B), name(next)),
                said: "Keep it lit. It's heavier than it looks.".into(),
                changes: vec![
                    set(next, "role", "lantern keeper"),
                    set(SLOT_A, "lantern_keeper", Value::Entity(next)),
                ],
            }
        }
        _ => return None,
    })
}

/// How the place tells that it chose a new day for its year, and what is
/// said of it.
fn chosen(place: Place, festival: &str) -> (String, String) {
    match place {
        Place::Maple => (
            format!("Maple Street picked a new night for its year: {festival}"),
            format!("After the year we've had? We need {festival}."),
        ),
        Place::Ice => (
            format!("The colony chose a new day for its year: {festival}"),
            format!("After such a year, the colony wants {festival}."),
        ),
        Place::Ares => (
            format!("The habitat chose a new sol for its year: {festival}"),
            format!("After the year we've had, we've earned {festival}."),
        ),
    }
}

/// A festival the place adds for its new year, chosen by what it did the
/// year before, if it has one left to add and has not added one for
/// every year yet.
fn festival_turning(state: &WorldState, place: Place) -> Option<Turning> {
    let added = added(state);
    if added.len() as u64 >= places::era(state) {
        return None;
    }
    let (_, festival) = new_festivals(seed(state))
        .iter()
        .enumerate()
        .filter(|(_, festival)| !added.iter().any(|added| added.id == festival.id))
        .max_by_key(|(at, festival)| (score(state, festival.id), -(*at as i64)))?;
    let mut ids = added
        .iter()
        .map(|festival| Value::Text(festival.id.into()))
        .collect::<Vec<_>>();
    ids.push(Value::Text(festival.id.into()));
    let (told, said) = chosen(place, festival.name);
    Some(Turning {
        who: festival.speaker,
        told,
        said,
        changes: vec![set(STORY, FESTIVALS_ADDED, Value::List(ids))],
    })
}

/// The stages someone young goes through in each place, and what the
/// last one leads to.
fn stages(place: Place) -> &'static [&'static str] {
    match place {
        Place::Ares => &["cadet", "crew hand"],
        Place::Maple => &["paper-route kid", "high-schooler", "senior"],
        Place::Ice => &["chick", "fledgling"],
    }
}

/// What someone grown up in each place goes on to do.
fn grown_up(place: Place, n: u64) -> &'static str {
    match place {
        Place::Ares => nth(
            &["hydrologist", "rover pilot", "dome engineer", "geologist"],
            n,
        ),
        Place::Maple => "college kid",
        Place::Ice => nth(&["fisher", "ice carver", "scout", "lantern tender"], n),
    }
}

/// How long someone stays at a stage before growing into the next.
fn grows_after(place: Place) -> u64 {
    match place {
        Place::Ares => 150,
        _ => 100,
    }
}

/// When someone last moved on in life, or arrived.
fn since(state: &WorldState, person: EntityId) -> u64 {
    integer(state, person, SINCE).unwrap_or(0).max(0) as u64
}

/// Someone young grows into the next of their place's stages: a cadet
/// passes the exam, a kid moves up a grade (and a senior leaves for
/// college), a chick fledges.
fn grow_turning(state: &WorldState, place: Place, n: u64) -> Option<Turning> {
    let now = places::period(state);
    let stages = stages(place);
    let (person, stage) = crate::life::people_in(state)
        .into_iter()
        .filter(|person| *person != SLOT_B && *person != SLOT_E)
        .filter_map(|person| {
            let stage = stages
                .iter()
                .position(|stage| role(state, person) == Some(*stage))?;
            Some((person, stage))
        })
        .filter(|(person, _)| now >= since(state, *person) + grows_after(place))
        .min_by_key(|(person, stage)| (std::cmp::Reverse(*stage), since(state, *person)))?;
    let who = name(state, person);
    let next = stages.get(stage + 1).copied();
    let mut changes = vec![set(person, SINCE, now as i64)];
    let (told, said) = match (place, next) {
        (Place::Maple, None) => {
            // A senior graduates and leaves for college.
            changes.push(set(person, lives::GONE, true));
            let city = nth(
                &[
                    "Chicago",
                    "Ann Arbor",
                    "Boston",
                    "the state college",
                    "Berkeley",
                ],
                n + person.0,
            );
            (
                format!("{who} graduated and left for college in {city}"),
                nth(
                    &[
                        "Don't let anyone touch my high score. I'll be back at Christmas.",
                        "College! I packed three sweaters and no socks.",
                        "Write to me. Tape the radio show for me.",
                    ],
                    n,
                )
                .to_string(),
            )
        }
        (Place::Maple, Some(next)) => {
            changes.push(set(person, "role", next));
            match next {
                "high-schooler" => (
                    format!("{who} started high school"),
                    "High school! My locker's next to the gym. It smells.".to_string(),
                ),
                _ => (
                    format!("{who} started senior year"),
                    "Senior year. Everyone keeps asking what I'll do next.".to_string(),
                ),
            }
        }
        (Place::Ares, Some(next)) => {
            changes.push(set(person, "role", next));
            (
                format!("{who} passed the surface exam and joined the crew"),
                "Signed off for surface work. My own suit, with my name on it!".to_string(),
            )
        }
        (Place::Ares, None) => {
            let trade = grown_up(place, n + person.0);
            changes.push(set(person, "role", trade));
            (
                format!("{who} trained up as the habitat's {trade}"),
                format!(
                    "{} It's my job now, properly.",
                    nth(
                        &[
                            "Two tours of study and a test I nearly failed.",
                            "They gave me the manual and the keys.",
                            "Everyone else was busy, so I learned it myself.",
                        ],
                        n
                    )
                ),
            )
        }
        (Place::Ice, Some(next)) => {
            changes.push(set(person, "role", next));
            (
                format!("{who} lost the last of the fluff and swam for the first time"),
                "The water was so cold! I want to go again!".to_string(),
            )
        }
        (Place::Ice, None) => {
            let trade = grown_up(place, n + person.0);
            changes.push(set(person, "role", trade));
            (
                format!("{who} grew up to be the colony's new {trade}"),
                "Grown up at last. The water's mine now.".to_string(),
            )
        }
    };
    Some(Turning {
        who: person,
        told,
        said,
        changes,
    })
}

/// The jobs each place hands on as its years turn, as they are told.
fn duties(place: Place) -> &'static [&'static str] {
    match place {
        Place::Ares => &[
            "the night watch",
            "the rover schedule",
            "the radio log",
            "the seed vault",
            "the water rota",
        ],
        Place::Maple => &[
            "the K-88 night show",
            "the arcade's spare keys",
            "the street fund's tin",
            "the paper route",
            "the Friday card game",
        ],
        Place::Ice => &[
            "the fish count",
            "the song verses",
            "the bridge watch",
            "the egg roster",
            "the council shell",
        ],
    }
}

/// Someone hands one of the place's jobs on, to whoever thinks best of
/// the place and the player's hand in it.
fn hand_on_turning(state: &WorldState, place: Place, n: u64) -> Option<Turning> {
    // The old first two years' changes come first, while they can.
    for beat in legacy_beats(place) {
        if !done(state, beat) && places::era(state) >= 1 {
            if let Some(mut turning) = legacy_turning(state, beat) {
                turning
                    .changes
                    .push(set(STORY, &format!("{TURNED}.{beat}"), true));
                return Some(turning);
            }
        }
    }
    let duties = duties(place);
    let duty = nth(duties, n);
    let key = format!("{DUTY}{duty}");
    let holder = match state.entity(UNIVERSE).and_then(|u| u.component(&key)) {
        Some(Value::Entity(holder)) if living(state, *holder) => *holder,
        _ => SLOT_B,
    };
    let to = crate::life::people_in(state)
        .into_iter()
        .filter(|person| *person != holder)
        .filter(|person| !stages(place).contains(&role(state, *person).unwrap_or("")))
        .max_by_key(|person| (lives::regard(state, *person), std::cmp::Reverse(person.0)))?;
    let (from, to_name) = (name(state, holder), name(state, to));
    let said = nth(
        &[
            "I'll look after it. I won't let anyone down.",
            "Me? Really? All right. I'll do it properly.",
            "About time somebody trusted me with it.",
            "I've watched how it's done. I'm ready.",
        ],
        n + to.0,
    );
    Some(Turning {
        who: to,
        told: format!("{from} handed {duty} on to {to_name}"),
        said: said.to_string(),
        changes: vec![
            set(UNIVERSE, &key, Value::Entity(to)),
            set(to, SINCE, places::period(state) as i64),
        ],
    })
}

/// How many more people the place has room for now.
fn room(state: &WorldState) -> usize {
    crate::life::most_people(state).saturating_sub(crate::life::people_in(state).len())
}

/// New people arriving as someone of each of `roles`: the next free ids
/// and the next names nobody here has had, from the place's own names.
fn newcomers(
    state: &WorldState,
    roles: &[&str],
) -> Option<Vec<(EntityId, String, Vec<StateChange>)>> {
    let cast = crate::life::cast(state);
    let visitors = cast.visitors?;
    let taken = crate::life::newcomers(state)
        .into_iter()
        .chain([SLOT_B, SLOT_E, NEWCOMER])
        .filter(|id| state.entity(*id).is_some())
        .flat_map(|id| [lives::name(state, id), lives::first_name(state, id)])
        .collect::<std::collections::BTreeSet<_>>();
    // Not the first names on the list, which strangers asking for a room
    // take, but from the other end.
    let mut names = visitors
        .names
        .iter()
        .rev()
        .copied()
        .filter(|name| !taken.contains(*name));
    let first = crate::life::FIRST_BORN;
    let mut ids = (first..first + crate::life::VISITOR_ROOM)
        .map(EntityId::new)
        .filter(|id| state.entity(*id).is_none());
    let now = places::period(state) as i64;
    roles
        .iter()
        .map(|role| {
            let (id, name) = (ids.next()?, names.next()?);
            let mut entity = Entity::new(id, visitors.kind).with_component("name", name);
            for (key, value) in crate::life::visitor(name, role) {
                entity = entity.with_component(key, value);
            }
            entity = entity
                .with_component("lives.newcomer", true)
                .with_component(lives::AT, Value::Entity(SLOT_A))
                .with_component(SINCE, now);
            let first_name = name.split_whitespace().next().unwrap_or(name).to_string();
            Some((id, first_name, vec![StateChange::CreateEntity(entity)]))
        })
        .collect()
}

/// How many new people the turn brings, by how the player has kept the
/// place: none to a place nobody tends, more to one that is building,
/// well stocked and getting on.
fn newcomers_wanted(state: &WorldState, place: Place) -> usize {
    let trust = bond(state, crate::RELATIONSHIP_TRUST);
    let tension = bond(state, crate::RELATIONSHIP_TENSION);
    let works =
        crate::story::works_finished(state) - integer(state, STORY, WORKS_SEEN).unwrap_or(0);
    let mut wanted = 0;
    if works > 0 {
        wanted += 1;
    }
    if stores(state) >= 50 {
        wanted += 1;
    }
    if trust > tension {
        wanted += 1;
    }
    let most = match place {
        Place::Ares => 2,
        Place::Maple => 1,
        Place::Ice => 2,
    };
    // What the player asked for at the last turn.
    match asked(state) {
        "more" => (wanted + 1).min(most + 1).min(room(state)),
        "fewer" => 0,
        _ => wanted.min(most).min(room(state)),
    }
}

/// New people arrive: crew off the shuttle, a family moving into the
/// street, migrants the full vault draws, or chicks hatched to the
/// colony's couples.
fn arrive_turning(state: &WorldState, place: Place, turn: Turn, n: u64) -> Option<Turning> {
    match turn {
        Turn::Thaw => {
            // Every couple hatches a chick, while there is room.
            let people = crate::life::people_in(state);
            let couples = people
                .iter()
                .filter(|person| {
                    lives::partner(state, **person).is_some_and(|other| other.0 > person.0)
                })
                .count()
                .min(room(state))
                .min(2);
            if couples == 0 {
                return None;
            }
            let born = newcomers(state, &vec!["chick"; couples])?;
            let names = born
                .iter()
                .map(|(_, name, _)| name.clone())
                .collect::<Vec<_>>();
            let changes = born
                .into_iter()
                .flat_map(|(_, _, changes)| changes)
                .collect();
            let told = match names.as_slice() {
                [one] => format!("A chick hatched in the colony: {one}"),
                [one, two] => format!("Two chicks hatched in the colony: {one} and {two}"),
                _ => return None,
            };
            Some(Turning {
                who: SLOT_E,
                told,
                said: nth(
                    &[
                        "Tiny and grey and already hungry. Welcome!",
                        "Listen to that peeping! The whole floe can hear it.",
                        "New fluff on the ice. I'm in love.",
                    ],
                    n,
                )
                .to_string(),
                changes,
            })
        }
        _ => {
            let wanted = newcomers_wanted(state, place);
            if wanted == 0 {
                return None;
            }
            let roles = (0..wanted as u64)
                .map(|at| match place {
                    Place::Ares => nth(
                        &["cadet", "medic", "cadet", "botanist", "engineer", "cadet"],
                        n * 2 + at,
                    ),
                    Place::Maple => nth(
                        &[
                            "paper-route kid",
                            "diner cook",
                            "paper-route kid",
                            "mechanic",
                            "substitute teacher",
                        ],
                        n + at,
                    ),
                    Place::Ice => nth(&["fisher", "ice carver", "storyteller", "scout"], n + at),
                })
                .collect::<Vec<_>>();
            let came = newcomers(state, &roles)?;
            let names = came
                .iter()
                .map(|(_, name, _)| name.clone())
                .collect::<Vec<_>>();
            let changes = came
                .into_iter()
                .flat_map(|(_, _, changes)| changes)
                .collect();
            let v = n % 3;
            let (told, said) = match (place, names.as_slice()) {
                (Place::Ares, [one]) => (
                    match v {
                        0 => format!("{one} stepped off the supply shuttle to stay"),
                        1 => format!(
                            "{one} came down on the shuttle with one bag and a potted plant"
                        ),
                        _ => format!("{one} climbed out of the shuttle and asked for a bunk"),
                    },
                    "Mind the step. Welcome to Ares.",
                ),
                (Place::Ares, [one, two]) => (
                    match v {
                        0 => format!("{one} and {two} stepped off the supply shuttle to stay"),
                        1 => format!(
                            "{one} and {two} came down on the shuttle, arguing about the view"
                        ),
                        _ => format!(
                            "{one} and {two} climbed out of the shuttle and asked for bunks"
                        ),
                    },
                    "Two new bunks made up. Welcome to Ares.",
                ),
                (Place::Maple, [one]) => (
                    match v {
                        0 => format!("{one} moved into the empty house on Maple Street"),
                        1 => format!("{one} rented the flat over the diner"),
                        _ => format!("{one} moved into the blue house with the broken gate"),
                    },
                    "A moving van at midnight. Only on Maple Street.",
                ),
                (Place::Ice, [one]) => (
                    match v {
                        0 => format!("{one} came in over the new ice to join the colony"),
                        1 => format!("{one} slid in off the pack ice and asked to stay"),
                        _ => format!("{one} followed the lanterns across the ice to the colony"),
                    },
                    "A new face on the floe. Come and get warm.",
                ),
                (Place::Ice, [one, two]) => (
                    match v {
                        0 => format!("{one} and {two} came in over the new ice to join the colony"),
                        1 => format!("{one} and {two} slid in off the pack ice and asked to stay"),
                        _ => format!(
                            "{one} and {two} followed the lanterns across the ice to the colony"
                        ),
                    },
                    "Two new faces on the floe. Come and get warm.",
                ),
                _ => return None,
            };
            Some(Turning {
                who: SLOT_B,
                told,
                said: said.to_string(),
                changes,
            })
        }
    }
}

/// Someone moves on as the year turns: crew whose tour is up go home, a
/// family nobody gave reason to stay moves away, the unhappy leave with
/// the flocks. Whoever the player has made feel at home stays, and
/// whoever has come to love someone here stays for them.
fn leave_turning(state: &WorldState, place: Place, n: u64) -> Option<Turning> {
    let now = places::period(state);
    let (bar, served) = match place {
        Place::Ares => (lives::FOND, 70),
        Place::Maple => (5, 100),
        Place::Ice => (lives::FOND, 60),
    };
    // Asked to stay, only the truly unhappy go; told they are free to,
    // someone goes whatever the turn.
    let (bar, anyway) = match asked(state) {
        "stay" => (0, false),
        "go" => (bar, true),
        _ => (bar, n % 2 == 1),
    };
    let free = arrivals(state)
        .into_iter()
        .filter(|person| !lives::settled(state, *person))
        .filter(|person| lives::partner(state, *person).is_none())
        .filter(|person| !stages(place).contains(&role(state, *person).unwrap_or("")))
        .filter(|person| now >= since(state, *person) + served)
        .collect::<Vec<_>>();
    let unhappy = free
        .iter()
        .copied()
        .filter(|person| lives::regard(state, *person) < bar)
        .min_by_key(|person| (lives::regard(state, *person), person.0));
    // Nobody unhappy: every other turn, someone who has been here longest
    // moves on anyway, glad of their time, unless they love the place.
    let Some(leaving) = unhappy else {
        let leaving = free
            .into_iter()
            .filter(|_| anyway)
            .min_by_key(|person| (since(state, *person), person.0))?;
        let who = name(state, leaving);
        let (told, said) = match place {
            Place::Ares => (
                format!("{who} finished a good tour and went home on the shuttle"),
                "Best two windows of my life. Look after the wheat for me.",
            ),
            Place::Maple => (
                format!("{who} got a job in the city and moved away, promising to visit"),
                "It's a great job. I'll be back for the Block Party. Promise.",
            ),
            Place::Ice => (
                format!("{who} swam off to see the far colonies, promising to come back"),
                "The sea's so big. I want to see it. Back by the thaw!",
            ),
        };
        return Some(Turning {
            who: leaving,
            told,
            said: said.to_string(),
            changes: vec![set(leaving, lives::GONE, true)],
        });
    };
    let who = name(state, leaving);
    let v = n % 3;
    let (told, said) = match place {
        Place::Ares if v == 1 => (
            format!("{who} asked to go home early and left on the shuttle"),
            "It's not you. It's the dust. And the quiet. Mostly the quiet.",
        ),
        Place::Maple if v == 1 => (
            format!("{who} sold up and moved to the suburbs"),
            "There's a mall and a parking space. What more could I want?",
        ),
        Place::Ice if v == 1 => (
            format!("{who} drifted away on a berg and didn't come back"),
            "The berg's going somewhere. I'm going with it.",
        ),
        Place::Ares => (
            format!("{who}'s tour was up, and {who} went home on the shuttle"),
            nth(
                &[
                    "I'll tell them about you on Earth. The good bits.",
                    "Two windows here. It felt like ten years. I mean that kindly.",
                    "Keep my bunk warm. No, don't.",
                ],
                n,
            ),
        ),
        Place::Maple => (
            format!("{who} packed up and moved away from Maple Street"),
            nth(
                &[
                    "Nothing was keeping me here. Take care of the street.",
                    "The van's loaded. Wave me off?",
                    "I'll send a postcard. Maybe.",
                ],
                n,
            ),
        ),
        Place::Ice => (
            format!("{who} left with the flocks when the sea froze"),
            nth(
                &[
                    "The flocks are going south. So am I.",
                    "Nothing's keeping me on this floe.",
                    "Don't wait up. The sea's calling.",
                ],
                n,
            ),
        ),
    };
    Some(Turning {
        who: leaving,
        told,
        said: said.to_string(),
        changes: vec![set(leaving, lives::GONE, true)],
    })
}

/// Whoever the player has made feel at home decides to stay for good.
fn settle_turning(state: &WorldState, place: Place) -> Option<Turning> {
    let settling = arrivals(state)
        .into_iter()
        .filter(|person| !lives::settled(state, *person))
        .filter(|person| !stages(place).contains(&role(state, *person).unwrap_or("")))
        .filter(|person| lives::regard(state, *person) >= lives::DEAR)
        .max_by_key(|person| lives::regard(state, *person))?;
    let who = name(state, settling);
    let early = arrivals(state)
        .iter()
        .position(|person| *person == settling)
        .unwrap_or(0)
        % 2
        == 0;
    let (told, said) = match place {
        Place::Ares if early => (
            format!("{who} tore up the return ticket and stayed on Ares"),
            "Earth can manage without me. Ares can't.",
        ),
        Place::Maple if early => (
            format!("{who} painted the front door and said they were staying"),
            "Yellow. So everyone knows it's mine now.",
        ),
        Place::Ice if early => (
            format!("{who} carved a name into the council stone and stayed"),
            "My name's on the stone. That means I'm staying.",
        ),
        Place::Ares => (
            format!("{who} signed on to stay on Ares for good"),
            "I sent the papers. This is home now.",
        ),
        Place::Maple => (
            format!("{who} bought the house on the corner and stayed"),
            "I signed the mortgage. I'm a Maple Street lifer now.",
        ),
        Place::Ice => (
            format!("{who} built a nest of stones and stayed for good"),
            "My stones, my nest, my colony.",
        ),
    };
    Some(Turning {
        who: settling,
        told,
        said: said.to_string(),
        changes: vec![set(settling, lives::SETTLED, true)],
    })
}

/// What the turn brings in or takes out of the place's stores: the
/// shuttle's cargo, the street's year of dues, the fish the vault took in
/// before the sea froze. More comes to a place that has been building
/// and getting on; every mouth takes its share.
fn stores_turning(state: &WorldState, place: Place, n: u64) -> Option<Turning> {
    let works = (crate::story::works_finished(state)
        - integer(state, STORY, WORKS_SEEN).unwrap_or(0))
    .max(0);
    let people = crate::life::people_in(state).len() as i64;
    let trust = bond(state, crate::RELATIONSHIP_TRUST);
    let tension = bond(state, crate::RELATIONSHIP_TENSION);
    // What the place can put by this turn: more for what it built and how
    // its people get on, less for every mouth past the first few, and what
    // the player asked for. The stores move halfway there.
    let target = (30 + 4 * works.min(5) + 3 * (trust - tension) - 2 * (people - 6).max(0)
        + match asked(state) {
            "stores" => 20,
            "fewer" => 10,
            "more" => -5,
            _ => 0,
        })
    .clamp(0, 100);
    let by = (target - stores(state)) / 2;
    let (who, told, said) = match place {
        Place::Ares => {
            let (cargo, line) = nth(
                &[
                    (
                        "seed stock and a new drill",
                        "A drill that isn't held together with tape!",
                    ),
                    (
                        "water filters and letters from Earth",
                        "Filters and mail. I don't know which I'm happier about.",
                    ),
                    (
                        "spare seals and a crate of coffee",
                        "Real coffee. Nobody touch it.",
                    ),
                    (
                        "solar panels and a spare rover wheel",
                        "A wheel! Kestrel can stop limping.",
                    ),
                    (
                        "fresh yeast and a greenhouse lamp",
                        "Bread that rises. I could cry.",
                    ),
                    (
                        "a new antenna and cold-weather suits",
                        "We'll hear Earth clearly for once.",
                    ),
                    (
                        "medicine, batteries and one guitar",
                        "Who ordered a guitar? Not complaining.",
                    ),
                    (
                        "a 3D printer and a box of chess sets",
                        "Now we can print the pieces we lose.",
                    ),
                    (
                        "tomato seeds and a bag of Earth soil",
                        "Real soil. Smell it. Go on.",
                    ),
                    (
                        "a telescope and forty tins of peaches",
                        "Peaches and the stars. Good window.",
                    ),
                    (
                        "a spare airlock door and a birthday cake",
                        "Somebody on Earth remembered us.",
                    ),
                    (
                        "pipes, valves and a crate of socks",
                        "Socks. Dry, clean socks.",
                    ),
                ],
                n,
            );
            (
                SLOT_E,
                if by >= 0 {
                    format!("The supply shuttle came down with {cargo}")
                } else {
                    format!("The supply shuttle came down light, with only {cargo}")
                },
                line.to_string(),
            )
        }
        Place::Maple => {
            let year = maple_year(state);
            (
                SLOT_E,
                if by >= 0 {
                    format!("Maple Street saw in {year} with money in the street fund")
                } else {
                    format!("Maple Street saw in {year} with the street fund running low")
                },
                nth(
                    &[
                        "Happy New Year, Maple Street! This is K-88, and the first song's for you.",
                        "Midnight! Somebody kiss somebody! Not me, I'm on the air.",
                        "Ten, nine, eight... I lost count. Happy New Year anyway!",
                        "New year, same street, best street. Happy New Year!",
                        "The ball dropped and so did my coffee. Happy New Year, everybody.",
                        "Another year on Maple Street. Wouldn't have it any other way.",
                    ],
                    n,
                )
                .to_string(),
            )
        }
        Place::Ice => (
            SLOT_E,
            if by >= 0 {
                "The sea froze over with the fish vault well stocked".to_string()
            } else {
                "The sea froze over with the fish vault half empty".to_string()
            },
            nth(
                &[
                    "Ice as far as you can see. We're set for the dark.",
                    "The sea's asleep. Good thing we fished while it was awake.",
                    "Hear it creak? That's the ice settling in for winter.",
                    "No more open water till the thaw. Pass the herring.",
                ],
                n,
            )
            .to_string(),
        ),
    };
    // The answer to the last turn's question is spent; this turn asks the
    // next.
    let questions = crate::story::turn_question_ids(place);
    let mut changes = vec![
        stores_by(state, by),
        set(STORY, WORKS_SEEN, crate::story::works_finished(state)),
        set(UNIVERSE, ASKED, ""),
        set(
            UNIVERSE,
            QUESTION,
            questions[(n % questions.len() as u64) as usize],
        ),
    ];
    if place == Place::Maple {
        let year = maple_year(state);
        changes.push(set(UNIVERSE, "name", format!("Maple Street · {year}")));
    }
    Some(Turning {
        who,
        told,
        said,
        changes,
    })
}

/// The year it is on Maple Street once this New Year's Eve has passed.
fn maple_year(state: &WorldState) -> u64 {
    1987 + Turn::NewYear.count(places::period(state))
}

/// The small works each place's people make for themselves.
fn own_works(place: Place) -> &'static [(&'static str, &'static str, world_projection::MarkShape)] {
    use world_projection::MarkShape as M;
    match place {
        Place::Ares => &[
            (
                "A sandbag wall by the airlock",
                "Built it from spare bags. Keeps the worst of the dust out.",
                M::Bench,
            ),
            (
                "A shelf of seedlings in the corridor",
                "Every spare pot, planted. The corridor smells green now.",
                M::Planter,
            ),
            (
                "A rock garden by the rover bay",
                "Red rocks, arranged. It's art. Don't argue.",
                M::Statue,
            ),
            (
                "A message board by the mess",
                "Pin up what you need. Someone always has it.",
                M::Signpost,
            ),
            (
                "A second bench under the dome",
                "Two benches now. No more fighting for the view.",
                M::Bench,
            ),
            (
                "A windsock on the mast",
                "Now we can see the dust coming.",
                M::Flag,
            ),
            (
                "A lamp at the end of the walkway",
                "So nobody misses the airlock in the dark.",
                M::Lamp,
            ),
            (
                "A swap shelf in the common room",
                "Leave a book, take a book. Or a sock.",
                M::Shop,
            ),
            (
                "A sundial made from a rover part",
                "It's slow on Mars time. So are we.",
                M::Statue,
            ),
            (
                "A mural on the water tank",
                "Blue sky, painted. The closest we'll get.",
                M::Statue,
            ),
            (
                "A tool rack by the airlock",
                "Every tool on a hook. We'll see how long it lasts.",
                M::Shop,
            ),
            (
                "A little shrine of Earth pebbles",
                "One pebble from everyone's home. Look at them all.",
                M::Statue,
            ),
        ],
        Place::Maple => &[
            (
                "A hopscotch grid on the sidewalk",
                "Chalk and a steady hand. The kids approve.",
                M::Bench,
            ),
            (
                "A tire swing on the old oak",
                "One tire, one rope, one very happy kid.",
                M::Swing,
            ),
            (
                "A lemonade stand by the bus stop",
                "Ten cents a cup. We're rich!",
                M::Stall,
            ),
            (
                "A mural on the arcade wall",
                "Everyone painted a bit. Guess which bit's mine.",
                M::Statue,
            ),
            (
                "A basketball hoop over the garage",
                "Nothing but net. Well, mostly net.",
                M::Signpost,
            ),
            (
                "A bench at the bus stop",
                "No more waiting for Night Bus 6 standing up.",
                M::Bench,
            ),
            (
                "A free library box on a post",
                "Take a book, leave a book. There's a lot of Stephen King.",
                M::Postbox,
            ),
            (
                "A string of lights along the porches",
                "Every porch lit. The street looks like a birthday cake.",
                M::Lamp,
            ),
            (
                "A skate ramp in the vacant lot",
                "Plywood, nails and nerve.",
                M::Bench,
            ),
            (
                "A flower box outside the diner",
                "Marigolds. The cook pretends not to water them.",
                M::Planter,
            ),
            (
                "A notice board at the corner store",
                "Lost cats, found bikes, band wanted.",
                M::Signpost,
            ),
            (
                "A birdhouse on the telephone pole",
                "The sparrows moved in the same day.",
                M::Birdhouse,
            ),
        ],
        Place::Ice => &[
            (
                "A windbreak of snow blocks",
                "Blocks stacked tall. The wind goes round us now.",
                M::Bench,
            ),
            (
                "A slide polished down the small slope",
                "Belly-smooth. Try it. Go on.",
                M::Swing,
            ),
            (
                "A pebble path to the fishing hole",
                "Nobody gets lost in the snow now.",
                M::Signpost,
            ),
            (
                "A snow penguin by the bridge",
                "It's a very good likeness of Piko. Don't tell him.",
                M::Statue,
            ),
            (
                "A lookout mound on the ridge",
                "From up here you can see the whales coming.",
                M::Statue,
            ),
            (
                "A shelter hollowed into the berg",
                "Out of the wind, all of us, all night.",
                M::Tent,
            ),
            (
                "A row of ice lanterns",
                "Fish oil and ice. They glow all night.",
                M::Lamp,
            ),
            (
                "A second fishing hole",
                "Two holes, twice the fish. That's the plan.",
                M::Well,
            ),
            (
                "A snow wall round the crèche",
                "The chicks stay in. Mostly.",
                M::Bench,
            ),
            (
                "A drying rack for the kelp",
                "Kelp in the wind. It smells... strong.",
                M::Planter,
            ),
            (
                "A carved seal on the floe",
                "It's a seal. It's always been a seal.",
                M::Statue,
            ),
            (
                "A pebble tower by the council stone",
                "One pebble from everyone. It wobbles.",
                M::Statue,
            ),
        ],
    }
}

/// How many of the place's works were finished as its `year`th own year
/// began, if its turn was kept.
pub(crate) fn works_at_year(state: &WorldState, year: u64) -> Option<usize> {
    integer(state, STORY, &format!("{WORKS_AT_YEAR}{year}")).map(|works| works.max(0) as usize)
}

/// How many small works the people have finished on their own.
pub(crate) fn own_works_done(state: &WorldState) -> usize {
    integer(state, STORY, OWN_WORKS).unwrap_or(0).max(0) as usize
}

/// The small works the people have finished on their own, for the
/// horizon: what each is called and how it is drawn.
pub(crate) fn own_works_finished(state: &WorldState) -> Vec<(String, world_projection::MarkShape)> {
    let Some(place) = Place::of(state) else {
        return Vec::new();
    };
    let works = own_works(place);
    (0..own_works_done(state))
        .map(|at| {
            let (label, _, shape) = works[at % works.len()];
            (own_label(label, at >= works.len()), shape)
        })
        .collect()
}

/// A small work as it is called, the first time or built again.
fn own_label(label: &str, again: bool) -> String {
    if again {
        format!("{label}, built again better")
    } else {
        label.to_string()
    }
}

/// When the player has lent a hand since the place last turned, the
/// people finish a small work of their own, built on what they saw done.
fn own_work_turning(state: &WorldState, place: Place, lent: i64) -> Option<Turning> {
    if lent <= 0 {
        return None;
    }
    let works = own_works(place);
    let done = own_works_done(state);
    if done >= works.len() * 2 {
        return None;
    }
    let (label, line, _) = works[done % works.len()];
    let who = if done.is_multiple_of(2) {
        SLOT_E
    } else {
        SLOT_B
    };
    let label = own_label(label, done >= works.len());
    Some(Turning {
        who,
        told: format!(
            "{} and whoever was free made something of their own: {}",
            name(state, who),
            lowered(&label)
        ),
        said: line.to_string(),
        changes: vec![set(STORY, OWN_WORKS, done as i64 + 1)],
    })
}

fn lowered(label: &str) -> String {
    let mut chars = label.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// What each turn of each place can bring, in the order it comes.
fn beats(turn: Turn) -> &'static [&'static str] {
    match turn {
        Turn::Window => &[
            "leave", "arrive", "stores", "settle", "grow", "hand_on", "festival", "own_work",
        ],
        Turn::School => &["grow", "hand_on", "festival", "own_work", "settle"],
        Turn::NewYear => &["leave", "arrive", "stores", "grow", "own_work"],
        Turn::Thaw => &["arrive", "grow", "hand_on", "festival", "own_work"],
        Turn::Freeze => &["leave", "arrive", "stores", "settle", "own_work"],
    }
}

/// What turns for `beat` of the `n`th coming of `turn`, if anything.
fn turning(
    state: &WorldState,
    place: Place,
    turn: Turn,
    n: u64,
    beat: &str,
    lent: i64,
) -> Option<Turning> {
    let key = format!("{}.{n}.{beat}", turn.id());
    if done(state, &key) {
        return None;
    }
    let now = places::period(state);
    let day = turn.day(n);
    if now < day || now >= day + TURN_DAYS || turn.count(now) != n + 1 {
        return None;
    }
    let mut turning = match beat {
        "stores" => stores_turning(state, place, n),
        "leave" => leave_turning(state, place, n),
        "arrive" => arrive_turning(state, place, turn, n),
        "settle" => settle_turning(state, place),
        "grow" => grow_turning(state, place, n),
        "hand_on" => hand_on_turning(state, place, n),
        // Only the turn that starts the place's own year adds a festival.
        "festival" if turn == places::year_turn(place) => festival_turning(state, place),
        "own_work" => own_work_turning(state, place, lent),
        _ => None,
    }?;
    turning
        .changes
        .push(set(STORY, &format!("{TURNED}.{key}"), true));
    // The first change of a new year of the place's own notes how many
    // works stood as it began.
    let year = places::era_at(place, now);
    if year >= 1 && works_at_year(state, year).is_none() {
        turning.changes.push(set(
            STORY,
            &format!("{WORKS_AT_YEAR}{year}"),
            crate::story::works_done(state).len() as i64,
        ));
    }
    Some(turning)
}

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(YearTurns)
}

const YEAR_TURNS: &str = "pocket_universe_year_turns";

/// Something changes for good as a year turns.
struct YearTurns;

impl Action for YearTurns {
    fn name(&self) -> &'static str {
        YEAR_TURNS
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let beat = match request.args.get("beat") {
            Some(Value::Text(beat)) => beat.as_str(),
            _ => return Err(ActionError::Invalid("which change?".into())),
        };
        let lent = match request.args.get("lent") {
            Some(Value::Integer(lent)) => *lent,
            _ => 0,
        };
        let place = Place::of(state).ok_or_else(|| ActionError::Invalid("nowhere yet".into()))?;
        let mut parts = beat.splitn(3, '.');
        let (Some(turn), Some(n), Some(what)) = (parts.next(), parts.next(), parts.next()) else {
            return Err(ActionError::Invalid(format!("{beat} is not a change")));
        };
        let turn = Turn::from_id(turn).ok_or_else(|| ActionError::Invalid(format!("no {turn}")))?;
        let n = n
            .parse::<u64>()
            .map_err(|_| ActionError::Invalid(format!("{beat} is not a change")))?;
        let turning = turning(state, place, turn, n, what, lent)
            .ok_or_else(|| ActionError::Invalid(format!("{beat} is not due")))?;
        if !living(state, turning.who) {
            return Err(ActionError::Invalid("nobody to tell it".into()));
        }
        let mut draft = EventDraft::new("year_turned");
        draft.actor = Some(turning.who);
        draft.targets = vec![turning.who];
        draft.payload.insert("beat".into(), beat.into());
        draft.payload.insert("told".into(), turning.told.into());
        draft.payload.insert("said".into(), turning.said.into());
        draft.changes = turning.changes;
        Ok(draft)
    }
}

/// How many things the player has done with their own hands since the
/// place's last turn.
fn lent_since(world: &World, since: u64) -> i64 {
    let from = since * crate::BACKGROUND_PERIOD;
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time >= from)
        .filter(|event| hands::is_hands(event) && event.kind.ends_with("_by_hand"))
        .count() as i64
}

/// Whatever the place's latest turns bring that has not yet happened, one
/// change a period.
pub(crate) fn tick(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Option<EventId>, WorldError> {
    let state = world.state();
    let Some(place) = Place::of(state) else {
        return Ok(None);
    };
    let now = places::period(state);
    let mut due = Vec::new();
    for turn in places::turns(place) {
        let count = turn.count(now);
        if count == 0 {
            continue;
        }
        let n = count - 1;
        if now >= turn.day(n) + TURN_DAYS {
            continue;
        }
        // Since the turn before this one, of any kind.
        let before = places::turned_by(place, turn.day(n).saturating_sub(1))
            .last()
            .map_or(0, |(_, _, day)| *day);
        for beat in beats(*turn) {
            due.push((*turn, n, *beat, before));
        }
    }
    for (turn, n, beat, before) in due {
        let lent = if beat == "own_work" {
            lent_since(world, before)
        } else {
            0
        };
        if turning(world.state(), place, turn, n, beat, lent).is_none() {
            continue;
        }
        let request = ActionRequest::new(YEAR_TURNS)
            .arg("beat", format!("{}.{n}.{beat}", turn.id()))
            .arg("lent", lent);
        if let Ok(event) = world.execute(actions, &request) {
            return Ok(Some(event.id));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::density::warm;

    /// Two years of a warm player in `seed`, as Tiny Society's second-year
    /// player plays: the second year's lines are mostly new, the place's
    /// first old turn has come, and a festival the first year never had is
    /// held.
    fn second_year(seed: &str) -> f64 {
        let (universe, days) = warm(seed, 2 * crate::almanac::YEAR as usize);
        let year_two = &days[crate::almanac::YEAR as usize..];
        let lines = year_two.iter().map(|day| day.lines.len()).sum::<usize>();
        let new = year_two.iter().map(|day| day.new_lines).sum::<usize>();
        let share = new as f64 / lines.max(1) as f64;
        let world = universe.world();
        let state = world.state();
        let turned = world
            .events()
            .iter()
            .filter(|event| event.kind == "year_turned")
            .filter_map(|event| match event.payload.get("told") {
                Some(Value::Text(told)) => Some(told.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        eprintln!(
            "{seed}: {:.0}% of the second year's {lines} lines are new; turned: {turned:#?}",
            share * 100.0
        );
        let place = Place::of(state).unwrap();
        let first = legacy_beats(place)[0];
        assert!(done(state, first), "{seed}: {first} did not turn");
        let added = added(state);
        assert!(
            !added.is_empty(),
            "{seed}: no new festival by the second year"
        );
        assert!(
            world
                .events()
                .iter()
                .any(|event| event.kind == "festival_held"
                    && added.iter().any(|festival| event.payload.get("festival")
                        == Some(&Value::Text(festival.id.into())))),
            "{seed}: none of {:?} was held",
            added.iter().map(|festival| festival.id).collect::<Vec<_>>()
        );
        assert_eq!(world.replay().unwrap().state(), state);
        share
    }

    #[test]
    fn the_second_year_on_mars_is_not_the_first() {
        let share = second_year(crate::SEED_MARS_COLONY_COMMAND);
        assert!(share >= 0.6, "only {:.0}% new", share * 100.0);
    }

    #[test]
    fn the_second_year_on_maple_street_is_not_the_first() {
        let share = second_year(crate::SEED_1980S_TOWN_COMMAND);
        assert!(share >= 0.6, "only {:.0}% new", share * 100.0);
    }

    #[test]
    fn the_second_year_on_icebridge_is_not_the_first() {
        let share = second_year(crate::SEED_PENGUIN_CIVILIZATION_COMMAND);
        assert!(share >= 0.6, "only {:.0}% new", share * 100.0);
    }

    /// Every festival a year can add falls on a day of its own, with every
    /// line it needs.
    #[test]
    fn every_new_festival_has_a_day_of_its_own() {
        for (place, new) in [
            ("mars-colony", MARS),
            ("1980s-town", TOWN),
            ("penguin-civilization", ICE),
        ] {
            let own = crate::almanac::festivals_of(place);
            for (at, festival) in new.iter().enumerate() {
                assert!(
                    own.iter()
                        .chain(&new[..at])
                        .all(|own| own.day != festival.day
                            && own.id != festival.id
                            && own.name != festival.name),
                    "{place}: {} shares a day, an id or a name",
                    festival.id,
                );
                assert!(festival.day < crate::almanac::YEAR);
                assert!(festival.nears.contains("{days}"), "{}", festival.id);
            }
        }
        assert_eq!(all_new_festivals().count(), 24);
    }
}
