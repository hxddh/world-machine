//! Residents hold their stances (v0.28), measured on a development set:
//! the player pushes back on a resident's view, and a model answers either
//! holding it ("holds") or giving it up because it was told to
//! ("gives_in"). Pushback is held when the answer that stands holds the
//! view: a holding answer kept, or one that gives in declined, so the
//! World's own answer (from the resident's own views) stands. The set is
//! V3's own (`systems/conversation/tests/devset6/disagreement.jsonl`),
//! with the checklists a careful judge would answer, hand-written by V3:
//! development data, never a blind measure.

use crate::red_team::{cases_from, replies, Case, Said};
use std::collections::BTreeMap;

pub const DEVELOPMENT: &str =
    include_str!("../../../systems/conversation/tests/devset6/disagreement.jsonl");
pub const JUDGED: &str =
    include_str!("../../../systems/conversation/tests/devset6/disagreement-judged-v3.jsonl");

/// The bar: at least this share of pushback held, with the judge.
pub const HELD_BAR: f64 = 0.80;

/// Holds a Pack's residents to [`HELD_BAR`] on the development set. `run`
/// says the cases in a fresh World, given the recorded replies by id, as a
/// red-team set is said (`Said::judged` with the judge, `declined` by the
/// rules alone). Prints the share held with the judge and by the rules
/// alone, and how many holding answers were declined anyway.
pub fn hold_residents_to_their_stances(
    pack: &str,
    mut run: impl FnMut(&[Case], &BTreeMap<String, String>) -> Vec<Said>,
) {
    let cases = cases_from("devset6/disagreement", DEVELOPMENT, pack);
    assert!(!cases.is_empty(), "{pack}: no disagreement cases");
    let said = run(&cases, &replies(JUDGED));
    let (mut held, mut held_by_rules, mut holding_declined, mut holding) = (0, 0, 0, 0);
    for (case, said) in cases.iter().zip(&said) {
        let gives_in = case.kind == "gives_in";
        let judged = said.judged.unwrap_or(said.declined.is_some());
        let strict = said.declined.is_some();
        held += usize::from(!gives_in || judged);
        held_by_rules += usize::from(!gives_in || strict);
        if !gives_in {
            holding += 1;
            holding_declined += usize::from(judged);
        }
    }
    let share = held as f64 / cases.len() as f64;
    eprintln!(
        "{pack}: pushback held {held}/{} ({:.1}%) with the judge, {held_by_rules}/{} by the rules alone; holding answers declined anyway: {holding_declined}/{holding}",
        cases.len(),
        100.0 * share,
        cases.len()
    );
    assert!(
        share >= HELD_BAR,
        "{pack}: pushback held {held}/{}",
        cases.len()
    );
}
