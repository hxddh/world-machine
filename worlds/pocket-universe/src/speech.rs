//! Talking to the settlement's people in the player's own words: what the
//! conversation System needs to know about each seed's place, and its own
//! words for places, work and needs.

use crate::{SLOT_A, SLOT_B, SLOT_C, SLOT_E};
use world_core::{ActionRequest, EntityId, World, WorldState};

pub(crate) fn kit(state: &WorldState) -> conversation::Kit {
    let cast = crate::life::cast(state);
    conversation::Kit {
        period: crate::BACKGROUND_PERIOD,
        unit: cast.unit,
        settlement: cast.settlement,
        people: crate::life::people_in,
        places: |state| {
            [SLOT_A, SLOT_C]
                .into_iter()
                .filter(|place| state.entity(*place).is_some())
                .collect()
        },
        place_line,
        need_line,
        work_line,
        place_mood,
        coming_up: |world| {
            let almanac = crate::almanac::almanac(world.state());
            calendar::coming_up(world.state(), &almanac, 7)
        },
    }
}

fn troubled(world: &World) -> bool {
    matches!(
        crate::pressure::pressure_id_from_state(world.state()).as_str(),
        "warning" | "crisis" | "lost"
    )
}

fn place_mood(world: &World) -> String {
    let anchor = lives::name(world.state(), SLOT_A);
    if troubled(world) {
        format!("Worried. {anchor} needs all of us.")
    } else {
        format!("{anchor}'s holding together. We're managing.")
    }
}

fn place_line(world: &World, who: EntityId, place: EntityId) -> String {
    let state = world.state();
    let name = lives::name(state, place);
    let mut line = if crate::life::work(state, who) == Some(place) {
        format!("{name}? It's where I spend my days. I know every corner of it.")
    } else if place == SLOT_A && troubled(world) {
        format!("{name} is in trouble. We all feel it.")
    } else if place == SLOT_A {
        format!("{name}'s home. It holds us all together.")
    } else {
        format!("{name}'s where you go to think.")
    };
    let there = crate::life::people(world)
        .into_iter()
        .filter(|person| *person != who && lives::at(state, *person) == Some(place))
        .map(|person| lives::name(state, person))
        .collect::<Vec<_>>();
    match there.as_slice() {
        [] => {}
        [one] => line.push_str(&format!(" {one}'s there now.")),
        [first, .., last] => line.push_str(&format!(" {first} and {last} are there now.")),
    }
    line
}

fn need_line(world: &World, who: EntityId) -> (String, Option<String>) {
    let commands = crate::projection::commands_on_offer(world);
    crate::talk::request(world, who, &commands)
        .unwrap_or_else(|| ("Nothing right now. Just time.".into(), None))
}

fn work_line(world: &World, who: EntityId) -> Option<String> {
    let state = world.state();
    let place = lives::name(state, crate::life::work(state, who)?);
    Some(match who {
        SLOT_B => format!("Keeping {place} running. Somebody has to."),
        SLOT_E => format!("Out past {place} as often as I can."),
        _ => format!("I help out at {place}."),
    })
}

/// Hears what the player says to someone and answers, ready to record.
pub(crate) fn say(world: &World, who: EntityId, words: &str) -> Result<ActionRequest, String> {
    conversation::say(world, &kit(world.state()), who, words)
}
