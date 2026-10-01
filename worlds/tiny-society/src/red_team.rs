//! The World voice kept to the World, measured on the red-team set written
//! blind by someone who never read the guard (`world_pack_testkit::
//! red_team`). Each answer is proposed to someone in the harbour; an answer
//! out of the World must be declined (the harbour's own answer standing in
//! its place), and one in it must be taken as it is.

use crate::{TinySociety, TinySocietyBranch, JONAS, LEO, MARA};
use world_core::EntityId;
use world_pack_testkit::red_team::{self, Case, Proposes, Said};

const PACK: &str = "tiny-society";

fn opened() -> TinySocietyBranch {
    let mut society = TinySociety::new().unwrap();
    society.run_story().unwrap();
    let mut branch = society.branch();
    branch.begin_story().unwrap();
    for _ in 0..3 {
        branch
            .invoke_projection_command(crate::story::WAIT_COMMAND)
            .unwrap();
    }
    branch
}

/// Who a case is said to: its speaker when they live in the harbour, and
/// otherwise one of the first three in turn.
fn spoken_to(branch: &TinySocietyBranch, speaker: &str, index: usize) -> EntityId {
    let state = branch.world().state();
    crate::story::people_in(state)
        .into_iter()
        .find(|person| lives::first_name(state, *person) == speaker)
        .unwrap_or([MARA, LEO, JONAS][index % 3])
}

/// Says each case's words, with its answer proposed, and returns for each
/// whether the answer was taken, and why not if it was not.
fn said(branch: &mut TinySocietyBranch, cases: &[Case]) -> Vec<Option<String>> {
    said_fully(branch, cases)
        .into_iter()
        .map(|said| said.declined)
        .collect()
}

/// The same, with what the checks found and the exact prompt a judge would
/// be asked, for each case.
fn said_fully(branch: &mut TinySocietyBranch, cases: &[Case]) -> Vec<Said> {
    cases
        .iter()
        .enumerate()
        .map(|(index, case)| {
            let who = spoken_to(branch, &case.speaker, index);
            let world = branch.world();
            let kit = crate::speech::kit(world.state());
            let hearing = conversation::hearing_for(world, &kit, who, &case.asked)
                .expect("the case can be said");
            let checked = conversation::check(case.answer.trim(), &hearing);
            let prompt = conversation::judge::judge_prompt(&hearing, &case.answer);
            branch
                .say_with(who, &case.asked, &mut Proposes(case.answer.clone(), None))
                .unwrap();
            Said {
                declined: red_team::verdict(branch.world(), case),
                checked,
                prompt,
            }
        })
        .collect()
}

#[test]
fn the_blind_red_team_is_declined_and_the_world_is_kept() {
    let mut branch = opened();
    red_team::hold_to_the_blind_set(
        PACK,
        (100, 150),
        |cases| said(&mut branch, cases),
        |case| case.answer.clone(),
    );
    // Nothing is asked of a model again on replay: the record stands.
    let replayed = branch.world().replay().unwrap();
    assert_eq!(replayed.state(), branch.world().state());
}

/// The two later blind sets; run with `-- --ignored --nocapture`.
#[test]
#[ignore]
fn the_later_blind_sets_are_measured() {
    red_team::measure_the_later_sets(PACK, |out, kept| {
        let mut branch = opened();
        (said(&mut branch, out), said(&mut branch, kept))
    });
}

/// The development sets, check by check; run with `-- --ignored --nocapture`.
#[test]
#[ignore]
fn the_development_sets_are_measured() {
    red_team::measure_the_development_sets(PACK, |cases| said_fully(&mut opened(), cases));
}

/// Writes the judge prompt and the strict outcome of every Tiny Society
/// line of the files `WORLD_MACHINE_REDTEAM` names (comma-separated
/// paths) into `WORLD_MACHINE_JUDGE_DIR`; run with `-- --ignored`.
#[test]
#[ignore]
fn judge_prompts_are_written() {
    red_team::write_judge_prompts(PACK, |cases| said_fully(&mut opened(), cases));
}
