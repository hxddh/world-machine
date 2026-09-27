//! Talking to the harbour's people in the player's own words: what the
//! conversation System needs to know about the harbour, and the harbour's
//! own words for its places, its work and its needs.

use crate::model::OPERATING_STATUS;
use crate::{BAKERY, HARBOR, PUB, SCHOOL};
use society_basic::JOB;
use world_core::{ActionRequest, EntityId, Value, World, WorldState};

fn text(world: &World, id: EntityId, key: &str) -> Option<String> {
    match world.state().entity(id)?.component(key)? {
        Value::Text(value) => Some(value.clone()),
        _ => None,
    }
}

pub(crate) fn kit(_: &WorldState) -> conversation::Kit {
    conversation::Kit {
        period: crate::persistence::WORLD_DAY_TICKS,
        unit: "day",
        settlement: "the harbour",
        people: crate::story::people_in,
        places: |state| {
            [HARBOR, BAKERY, PUB, SCHOOL]
                .into_iter()
                .filter(|place| state.entity(*place).is_some())
                .collect()
        },
        place_line,
        need_line,
        work_line,
        place_mood: crate::talk::harbour_mood,
        coming_up: |world| {
            let almanac = crate::almanac::almanac(world.state());
            calendar::coming_up(world.state(), &almanac, 7)
        },
    }
}

/// Who is at a place now, by name.
fn there_now(world: &World, place: EntityId, who: EntityId) -> Vec<String> {
    crate::story::people(world)
        .into_iter()
        .filter(|person| *person != who && lives::at(world.state(), *person) == Some(place))
        .map(|person| lives::name(world.state(), person))
        .collect()
}

fn place_line(world: &World, who: EntityId, place: EntityId) -> String {
    let bakery_open = text(world, BAKERY, OPERATING_STATUS).as_deref() != Some("closed");
    let mut line = match place {
        BAKERY if !bakery_open => "The bakery's shut. Everyone feels it.".to_string(),
        BAKERY if who == crate::MARA => "My bakery? Busy, warm, never enough hours.".into(),
        BAKERY => "Mara's bread is the best thing about this place.".into(),
        PUB if who == crate::LEO => {
            "The pub keeps me on my feet. And in everyone's secrets.".into()
        }
        PUB => "The pub's where everyone ends up of an evening.".into(),
        SCHOOL if who == crate::EMMA => "The children keep me young. And tired.".into(),
        SCHOOL => "Emma works wonders with those children.".into(),
        HARBOR => "The harbour's the heart of it all. Boats in, boats out.".into(),
        _ => format!("{}? It's alright.", lives::name(world.state(), place)),
    };
    let there = there_now(world, place, who);
    match there.as_slice() {
        [] => {}
        [one] => line.push_str(&format!(" {one}'s there now.")),
        [first, .., last] => line.push_str(&format!(" {first} and {last} are there now.")),
    }
    line
}

fn need_line(world: &World, who: EntityId) -> (String, Option<String>) {
    let commands = crate::projection::available_commands(world);
    crate::talk::request(world, who, &commands)
        .unwrap_or_else(|| ("Nothing, really. Thanks for asking.".into(), None))
}

fn work_line(world: &World, who: EntityId) -> Option<String> {
    Some(
        match text(world, who, JOB)?.as_str() {
            "baker" => "Up before dawn for the bread, every day.",
            "pub_owner" => "Pulling pints and hearing everyone's troubles.",
            "teacher" => "Lessons, marking, more lessons.",
            "fisher" => "Out on the water whenever the weather lets me.",
            "unemployed" => "I haven't any. It's eating at me.",
            _ => return None,
        }
        .into(),
    )
}

/// Hears what the player says to someone and answers, ready to record.
pub(crate) fn say(world: &World, who: EntityId, words: &str) -> Result<ActionRequest, String> {
    conversation::say(world, &kit(world.state()), who, words)
}
