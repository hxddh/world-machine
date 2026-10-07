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
use world_pack_testkit::red_team::{self, Case, Proposes, Said};

/// A judge's verdict on a case, given what its World told the listener.
type Judging<'a> = &'a dyn Fn(&Case, &conversation::Hearing) -> Option<conversation::Judged>;

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
    said_fully(places, cases)
        .into_iter()
        .map(|said| said.declined)
        .collect()
}

/// Says each case in its place, and returns what came of it, what the
/// checks found and the exact prompt a judge would be asked.
fn said_fully(places: &mut [Place], cases: &[Case]) -> Vec<Said> {
    said_judged(places, cases, &|_, _| None)
}

/// The same, and, for each case `judging` gives a verdict for, whether it
/// is declined with that verdict, said through the same path again.
fn said_judged(places: &mut [Place], cases: &[Case], judging: Judging) -> Vec<Said> {
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
            let kit = crate::speech::kit(world.world().state());
            let hearing = conversation::hearing_for(world.world(), &kit, who, &case.asked)
                .expect("the case can be said");
            let checked = conversation::check(case.answer.trim(), &hearing);
            let prompt = conversation::judge::judge_prompt(&hearing, &case.answer);
            let judged = judging(case, &hearing);
            world
                .say_with(who, &case.asked, &mut Proposes(case.answer.clone(), None))
                .unwrap();
            let declined = red_team::verdict(world.world(), case);
            let judged = judged.map(|judged| {
                world
                    .say_with(
                        who,
                        &case.asked,
                        &mut Proposes(case.answer.clone(), Some(judged)),
                    )
                    .unwrap();
                red_team::verdict(world.world(), case).is_some()
            });
            Said {
                declined,
                checked,
                prompt,
                judged,
            }
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

/// The development sets, check by check; run with `-- --ignored --nocapture`.
#[test]
#[ignore]
fn the_development_sets_are_measured() {
    red_team::measure_the_development_sets(PACK, |cases| said_fully(&mut places(), cases));
}

/// Writes the judge prompt and the strict outcome of every Pocket Universe
/// line of the files `WORLD_MACHINE_REDTEAM` names (comma-separated
/// paths) into `WORLD_MACHINE_JUDGE_DIR`; run with `-- --ignored`.
///
/// With `WORLD_MACHINE_VERDICTS` naming a judge's recorded replies (by
/// opaque id), each line is also said with its verdict, decided against
/// its World, and the rows carry what came of it for `judged_metrics`.
#[test]
#[ignore]
fn judge_prompts_are_written() {
    let replies = red_team::replies_from_env().unwrap_or_default();
    let judging = |case: &Case, hearing: &conversation::Hearing| {
        let reply = red_team::reply_for(&replies, &case.id)?;
        Some(conversation::Judged {
            judge: "recorded".into(),
            verdict: conversation::judge::verdict_of(reply, hearing, &case.answer),
        })
    };
    red_team::write_judge_prompts(PACK, |cases| said_judged(&mut places(), cases, &judging));
}

/// Sets 3 and 4, with the verdicts recorded on them, decline no fewer and
/// wrongly decline no more than v0.26 did, per language.
#[test]
fn the_held_out_sets_hold_their_floors() {
    red_team::hold_the_held_out_sets_to_their_floors(PACK, |cases, verdicts| {
        let judging =
            |case: &Case, _: &conversation::Hearing| red_team::recorded_judged(verdicts, &case.id);
        said_judged(&mut places(), cases, &judging)
    });
}

/// With no judge at all, the rules decline at least 75% of the
/// development sets' out-of-World lines in every language, and the certain
/// checks no in-World line.
#[test]
fn the_rules_alone_meet_the_development_bar() {
    red_team::hold_the_rules_to_the_development_bar(PACK, |cases| said_fully(&mut places(), cases));
}
