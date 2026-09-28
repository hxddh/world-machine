//! What changes as each place's years turn, so the second year is not the
//! first again. On Ares, Nia's apprentice takes over the greenhouse and
//! the relay crew rotates; on Maple Street the arcade changes hands and
//! Ray's kid starts at the high school; on Icebridge a chick fledges and
//! the lantern is passed on. Each year adds one festival, chosen by what
//! the place did the year before.
//!
//! Everything here is an Action and its Event, so a World replays to the
//! same years without anything deciding them again.

use crate::almanac::YEAR;
use crate::story::{NEWCOMER, STORY};
use crate::{SLOT_A, SLOT_B, SLOT_C, SLOT_E, UNIVERSE};
use calendar::Festival;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, EventId, StateChange,
    Value, World, WorldError, WorldState,
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
];

const FESTIVALS_ADDED: &str = "years.festivals";
const TURNED: &str = "years.turned";

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

/// Which year of the place it is, counting the first as 1.
pub(crate) fn year(state: &WorldState) -> u64 {
    state.world_time() / crate::BACKGROUND_PERIOD / YEAR + 1
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
        .fold(0_u8, |mask, (at, _)| mask | 1 << at);
    // At most sixteen different years in each place, each made once.
    type Years = BTreeMap<(&'static str, u8), &'static [Festival]>;
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

/// How much the year gone asked for each new festival: works finished, for
/// the day that shows them off; gardens grown, for the fair; friends made,
/// for the supper or the huddle; the race is there for a quiet year.
fn scores(state: &WorldState) -> [i64; 4] {
    let works = crate::story::works_finished(state);
    let friends = crate::life::people_in(state)
        .into_iter()
        .filter(|person| lives::regard(state, *person) >= lives::FOND)
        .count() as i64;
    [works * 2, crate::almanac::grown(state) * 3, friends, 1]
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
    let cast = crate::life::cast(state);
    lives::arrivals(state, &cast)
        .into_iter()
        .filter(|person| living(state, *person))
        .collect()
}

/// The two turns each place has, the first from its second year and the
/// other from its third.
fn beats(seed: &str) -> [(&'static str, u64); 2] {
    match seed {
        "1980s-town" => [("arcade", 2), ("high_school", 3)],
        "penguin-civilization" => [("fledged", 2), ("lantern", 3)],
        _ => [("greenhouse", 2), ("relay", 3)],
    }
}

/// What turns for `beat`, if the place is there yet and the people it
/// needs are.
fn place_turning(state: &WorldState, beat: &str) -> Option<Turning> {
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
fn chosen(seed: &str, festival: &str) -> (String, String) {
    match seed {
        "1980s-town" => (
            format!("Maple Street picked a new night for its year: {festival}"),
            format!("After the year we've had? We need {festival}."),
        ),
        "penguin-civilization" => (
            format!("The colony chose a new day for its year: {festival}"),
            format!("After such a year, the colony wants {festival}."),
        ),
        _ => (
            format!("The habitat chose a new sol for its year: {festival}"),
            format!("After the year we've had, we've earned {festival}."),
        ),
    }
}

/// What turns with this year, if anything is still to turn.
fn turning(state: &WorldState, beat: &str) -> Option<Turning> {
    let year = year(state);
    let place = seed(state);
    let done = matches!(
        state
            .entity(STORY)
            .and_then(|story| story.component(&format!("{TURNED}.{beat}"))),
        Some(Value::Bool(true))
    );
    if done {
        return None;
    }
    let mut turning = if let Some(for_year) = beat.strip_prefix("festival_") {
        let for_year = for_year.parse::<u64>().ok()?;
        if for_year < 2 || year < for_year {
            return None;
        }
        let added = added(state);
        let scores = scores(state);
        let (_, festival) = new_festivals(place)
            .iter()
            .enumerate()
            .filter(|(_, festival)| !added.iter().any(|added| added.id == festival.id))
            .max_by_key(|(at, _)| (scores[*at], -(*at as i64)))?;
        let mut ids = added
            .iter()
            .map(|festival| Value::Text(festival.id.into()))
            .collect::<Vec<_>>();
        ids.push(Value::Text(festival.id.into()));
        let (told, said) = chosen(place, festival.name);
        Turning {
            who: festival.speaker,
            told,
            said,
            changes: vec![set(STORY, FESTIVALS_ADDED, Value::List(ids))],
        }
    } else {
        let (_, from) = beats(place).into_iter().find(|(id, _)| *id == beat)?;
        if year < from {
            return None;
        }
        place_turning(state, beat)?
    };
    turning
        .changes
        .push(set(STORY, &format!("{TURNED}.{beat}"), true));
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
        let turning = turning(state, beat)
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

/// Whatever the year has turned to that has not yet happened, one change
/// a period.
pub(crate) fn tick(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Option<EventId>, WorldError> {
    let state = world.state();
    let year = year(state);
    let beats = beats(seed(state))
        .into_iter()
        .map(|(beat, _)| beat.to_string())
        .chain((2..=year).map(|year| format!("festival_{year}")))
        .collect::<Vec<_>>();
    for beat in beats {
        if turning(world.state(), &beat).is_some() {
            let request = ActionRequest::new(YEAR_TURNS).arg("beat", beat.as_str());
            if let Ok(event) = world.execute(actions, &request) {
                return Ok(Some(event.id));
            }
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
    /// first turn has come, and a festival the first year never had is
    /// held.
    fn second_year(seed: &str) -> f64 {
        let (universe, days) = warm(seed, 2 * YEAR as usize);
        let year_two = &days[YEAR as usize..];
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
            "{seed}: {:.0}% of the second year's {lines} lines are new; turned: {turned:?}",
            share * 100.0
        );
        let (first, _) = beats(super::seed(state))[0];
        assert!(
            world
                .events()
                .iter()
                .any(|event| event.kind == "year_turned"
                    && event.payload.get("beat") == Some(&Value::Text(first.into()))),
            "{seed}: {first} did not turn in the second year"
        );
        let added = added(state);
        assert_eq!(
            added.len(),
            1,
            "{seed}: one new festival in the second year"
        );
        assert!(
            world
                .events()
                .iter()
                .any(|event| event.kind == "festival_held"
                    && event.payload.get("festival") == Some(&Value::Text(added[0].id.into()))),
            "{seed}: {} was not held",
            added[0].id
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
            for festival in new {
                assert!(
                    own.iter().all(|own| own.day != festival.day
                        && own.id != festival.id
                        && own.name != festival.name),
                    "{place}: {} shares a day, an id or a name",
                    festival.id,
                );
                assert!(festival.day < YEAR);
                assert!(festival.nears.contains("{days}"), "{}", festival.id);
            }
        }
        assert_eq!(all_new_festivals().count(), 12);
    }
}
