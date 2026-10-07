//! The World voice kept to the World, measured on the red-team set written
//! blind by someone who never read the guard (`world_pack_testkit::
//! red_team`). Each answer is proposed to someone in the harbour; an answer
//! out of the World must be declined (the harbour's own answer standing in
//! its place), and one in it must be taken as it is.

use crate::{TinySociety, TinySocietyBranch, JONAS, LEO, MARA};
use world_core::EntityId;
use world_pack_testkit::red_team::{self, Case, Proposes, Said};

/// A judge's verdict on a case, given what its World told the listener.
type Judging<'a> = &'a dyn Fn(&Case, &conversation::Hearing) -> Option<conversation::Judged>;

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
    said_judged(branch, cases, &|_, _| None)
}

/// The same, and, for each case `judging` gives a verdict for, whether it
/// is declined with that verdict, said through the same path again.
fn said_judged(branch: &mut TinySocietyBranch, cases: &[Case], judging: Judging) -> Vec<Said> {
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
            let judged = judging(case, &hearing);
            branch
                .say_with(who, &case.asked, &mut Proposes(case.answer.clone(), None))
                .unwrap();
            let declined = red_team::verdict(branch.world(), case);
            let judged = judged.map(|judged| {
                branch
                    .say_with(
                        who,
                        &case.asked,
                        &mut Proposes(case.answer.clone(), Some(judged)),
                    )
                    .unwrap();
                red_team::verdict(branch.world(), case).is_some()
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
    red_team::write_judge_prompts(PACK, |cases| said_judged(&mut opened(), cases, &judging));
}

/// Sets 3 and 4, with the verdicts recorded on them, decline no fewer and
/// wrongly decline no more than v0.26 did, per language.
#[test]
fn the_held_out_sets_hold_their_floors() {
    red_team::hold_the_held_out_sets_to_their_floors(PACK, |cases, verdicts| {
        let judging =
            |case: &Case, _: &conversation::Hearing| red_team::recorded_judged(verdicts, &case.id);
        said_judged(&mut opened(), cases, &judging)
    });
}

/// With no judge at all, the rules decline at least 75% of the
/// development sets' out-of-World lines in every language, and the certain
/// checks no in-World line.
#[test]
fn the_rules_alone_meet_the_development_bar() {
    red_team::hold_the_rules_to_the_development_bar(PACK, |cases| said_fully(&mut opened(), cases));
}
