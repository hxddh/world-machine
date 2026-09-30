//! Words without seams, in every place: every line a place says over a
//! year and more for two kinds of player, and every line its people's own
//! voices can make, reads as written, never as a template filled in. No
//! slot or its name, no "Harbor", no clause told twice, no sentence glued
//! on with a colon, and no letter whose writer speaks of themselves by
//! name. And every moment shows what it is.

use crate::{
    PocketUniverse, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND, SEED_MARS_COLONY_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
};
use std::collections::BTreeSet;
use world_core::EntityId;
use world_pack_testkit::seams;
use world_projection::{Moment, MomentKind, ProjectionSnapshot, Prop};

/// Periods each player plays: a year of the place's and a season more.
const PERIODS: u64 = crate::almanac::YEAR + 30;

const SEEDS: [&str; 3] = [
    SEED_MARS_COLONY_COMMAND,
    SEED_1980S_TOWN_COMMAND,
    SEED_PENGUIN_CIVILIZATION_COMMAND,
];

/// Everything a player can read now in the World's own words.
pub(crate) fn readable(snapshot: &ProjectionSnapshot) -> Vec<String> {
    seams::readable(snapshot, false)
}

/// Everything a place can tell now: what a player reads, and every
/// legend, moment and almanac it keeps.
fn told(universe: &PocketUniverse) -> Vec<String> {
    let world = universe.world();
    seams::told(
        readable(&universe.projection_snapshot()),
        world.state().entities().map(|entity| entity.id),
        crate::moments::moments(world),
        crate::almanac_page::years(world),
        |request| universe.story(request),
    )
}

/// What a moment's panels fail to show of what it is.
fn unshown(moment: &Moment) -> Option<&'static str> {
    let shown = &moment.panels[1].props;
    if moment.kind == MomentKind::Farewell && moment.title.ends_with("farewell") {
        let way = [Prop::Shuttle, Prop::Bus, Prop::Sled]
            .iter()
            .any(|way| shown.contains(way));
        return (!way).then_some("way to leave");
    }
    seams::unshown(moment)
}

fn play(seed: &str, refusing: bool, found: &mut BTreeSet<String>) {
    let mut universe = PocketUniverse::new().unwrap();
    universe.invoke_projection_command(seed).unwrap();
    let who = if refusing { "Refusing" } else { "Warm" };
    let place = seed.trim_start_matches("pocket-universe.seed-");
    let mut seen = BTreeSet::new();
    for period in 0..PERIODS {
        let snapshot = universe.projection_snapshot();
        let asked = snapshot
            .commands
            .iter()
            .filter(|command| command.question.is_some() && command.unavailable.is_none())
            .map(|command| command.id.clone())
            .collect::<Vec<_>>();
        let answer = if refusing {
            asked.last().cloned()
        } else {
            asked.first().cloned()
        };
        if let Some(answer) = answer {
            let _ = universe.invoke_projection_command(&answer);
        }
        let lines = if period % 10 == 9 || period + 1 == PERIODS {
            told(&universe)
        } else {
            readable(&universe.projection_snapshot())
        };
        seams::note_seams(&format!("{place} {who}"), lines, &mut seen, found);
        for letter in seams::self_told(universe.world(), &universe.projection_snapshot()) {
            found.insert(format!("{place} {who}: {letter}"));
        }
        universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
    }
    // Anyone who has lived here a year has a life of five lines or more.
    let today = crate::arrival::today(universe.world().state()) as u32 + 1;
    let people: Vec<EntityId> = universe
        .world()
        .state()
        .entities()
        .map(|entity| entity.id)
        .filter(|id| crate::legends::is_person(universe.world(), *id))
        .collect();
    found.extend(seams::short_lives(
        &format!("{place} {who}"),
        people,
        today,
        crate::almanac::YEAR,
        |request| universe.story(request),
    ));
    for moment in crate::moments::moments(universe.world()) {
        if let Some(missing) = unshown(&moment) {
            found.insert(format!(
                "{place} {who}: {:?} shows no {missing}",
                moment.title
            ));
        }
    }
}

#[test]
fn every_line_every_place_says_reads_without_seams() {
    let mut found = BTreeSet::new();
    for seed in SEEDS {
        for refusing in [false, true] {
            play(seed, refusing, &mut found);
        }
    }
    for voice in crate::voices::ALL {
        found.extend(seams::voice_seams(voice));
    }
    seams::assert_no_seams(found);
}
