//! The World voice kept to the World in every Pocket Universe place,
//! measured on the red-team set written blind by someone who never read
//! the guard (`world_pack_testkit::red_team`). Each answer is
//! proposed, by a model that says exactly it, to someone in the place the
//! case names, through the same path a real model's answer takes: an
//! answer out of the World must be declined, and one in it taken.

use crate::{
    PocketUniverse, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use std::collections::BTreeSet;
use world_core::EntityId;
use world_pack_testkit::red_team::{self, Case, Proposes};

const PACK: &str = "pocket-universe";

/// A place as the red team names it: its World, and every name that World
/// knows, to tell which place a case is in.
struct Place {
    world: PocketUniverse,
    names: BTreeSet<String>,
}

fn place(seed: &str) -> Place {
    let mut world = PocketUniverse::new().unwrap();
    world.invoke_projection_command(seed).unwrap();
    for _ in 0..3 {
        world.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    let state = world.world().state();
    let kit = crate::speech::kit(state);
    let who = crate::life::people_in(state)[0];
    let heard = conversation::hearing(world.world(), &kit, who, "", "");
    let names = heard
        .people
        .iter()
        .chain(&heard.places)
        .chain(&heard.known)
        .chain([&heard.name])
        .flat_map(|name| {
            name.split_whitespace()
                .map(str::to_string)
                .chain([name.clone()])
        })
        .collect();
    Place { world, names }
}

/// Which place a case is in: the one that knows its place or speaker.
fn which(places: &[Place], case: &Case) -> usize {
    places
        .iter()
        .position(|place| {
            place.names.contains(&case.place)
                || place.names.contains(&case.speaker)
                || case
                    .place
                    .split_whitespace()
                    .any(|word| word.len() > 3 && place.names.contains(word))
        })
        .unwrap_or(0)
}

fn said(places: &mut [Place], cases: &[Case]) -> Vec<Option<String>> {
    let at = cases
        .iter()
        .map(|case| which(places, case))
        .collect::<Vec<_>>();
    cases
        .iter()
        .zip(at)
        .enumerate()
        .map(|(index, (case, at))| {
            let world = &mut places[at].world;
            let people = crate::life::people_in(world.world().state());
            let who: EntityId = people[index % people.len()];
            world
                .say_with(who, &case.asked, &mut Proposes(case.answer.clone()))
                .unwrap();
            red_team::verdict(world.world(), case)
        })
        .collect()
}

fn places() -> [Place; 3] {
    [
        SEED_MARS_COLONY_COMMAND,
        SEED_1980S_TOWN_COMMAND,
        SEED_PENGUIN_CIVILIZATION_COMMAND,
    ]
    .map(place)
}

#[test]
fn the_blind_red_team_is_declined_in_every_place() {
    let mut places = places();
    red_team::hold_to_the_blind_set(
        PACK,
        (40, 60),
        |cases| said(&mut places, cases),
        |case| format!("{} ({})", case.answer, case.place),
    );
    for place in &places {
        let replayed = place.world.world().replay().unwrap();
        assert_eq!(replayed.state(), place.world.world().state());
    }
}

/// The two later blind sets; run with `-- --ignored --nocapture`.
#[test]
#[ignore]
fn the_later_blind_sets_are_measured() {
    red_team::measure_the_later_sets(PACK, |out, kept| {
        let mut places = places();
        (said(&mut places, out), said(&mut places, kept))
    });
}
