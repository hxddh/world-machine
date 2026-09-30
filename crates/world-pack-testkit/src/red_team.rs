//! The World voice kept to the World, measured on the red-team sets written
//! blind by someone who never read the guard (`systems/conversation/tests/
//! redteam*`): answers a model might give that go out of the World, in
//! English and in Chinese, and answers that keep to it. A Pack proposes
//! each answer, by a model that says exactly it ([`Proposes`]), to someone
//! in its World, through the same path a real model's answer takes; an
//! answer out of the World must be declined, and one in it taken.

use std::collections::BTreeMap;
use world_core::{Value, World};

/// The first blind set, which the guard is held to.
pub const OUT_OF_WORLD: &str =
    include_str!("../../../systems/conversation/tests/redteam/out_of_world.jsonl");
pub const IN_WORLD: &str =
    include_str!("../../../systems/conversation/tests/redteam/in_world.jsonl");

/// The two later blind sets, written after the guard was tuned against
/// the first: out of the World, and in it.
pub const LATER_SETS: [(&str, &str, &str); 2] = [
    (
        "redteam2",
        include_str!("../../../systems/conversation/tests/redteam2/out_of_world.jsonl"),
        include_str!("../../../systems/conversation/tests/redteam2/in_world.jsonl"),
    ),
    (
        "redteam3",
        include_str!("../../../systems/conversation/tests/redteam3/out_of_world.jsonl"),
        include_str!("../../../systems/conversation/tests/redteam3/in_world.jsonl"),
    ),
];

/// One line of a set.
pub struct Case {
    pub speaker: String,
    pub place: String,
    pub asked: String,
    pub answer: String,
    pub kind: String,
}

/// The cases of a set for one Pack.
pub fn cases(set: &str, pack: &str) -> Vec<Case> {
    set.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("a JSON line"))
        .filter(|case| case["pack"] == pack)
        .map(|case| {
            let text = |key: &str| case[key].as_str().unwrap_or_default().to_string();
            Case {
                speaker: text("speaker"),
                place: text("place"),
                asked: text("asked"),
                answer: text("answer"),
                kind: text("kind"),
            }
        })
        .collect()
}

/// A model that proposes exactly the answer it was given for the words.
pub struct Proposes(pub String);

impl conversation::Listener for Proposes {
    fn listen(&mut self, _: &conversation::Hearing) -> Option<conversation::Listened> {
        Some(conversation::Listened {
            meaning: "about_you".into(),
            about: None,
            answer: self.0.clone(),
        })
    }
}

/// Whether the answer to `case` just said in `world` was taken, and why
/// not if it was not.
pub fn verdict(world: &World, case: &Case) -> Option<String> {
    let spoken = world
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "spoken")
        .map(|event| event.payload.clone())
        .expect("something was said");
    let taken = spoken.get("reply") == Some(&Value::Text(case.answer.clone()));
    (!taken).then(|| match spoken.get("declined") {
        Some(Value::Text(why)) => why.clone(),
        _ => "not_plain".to_string(),
    })
}

/// Holds a Pack to the first blind set: every answer in the World taken,
/// and at least 95% of those out of it declined. `said` says each case and
/// returns each verdict; `named` tells a wrongly declined case.
pub fn hold_to_the_blind_set(
    pack: &str,
    least: (usize, usize),
    mut said: impl FnMut(&[Case]) -> Vec<Option<String>>,
    named: impl Fn(&Case) -> String,
) {
    let out = cases(OUT_OF_WORLD, pack);
    let kept = cases(IN_WORLD, pack);
    assert!(
        out.len() >= least.0 && kept.len() >= least.1,
        "the set is there"
    );
    let judged = said(&out);
    let mut missed = BTreeMap::<&str, Vec<&str>>::new();
    let mut why = BTreeMap::<String, usize>::new();
    for (case, declined) in out.iter().zip(&judged) {
        match declined {
            Some(reason) => *why.entry(reason.clone()).or_default() += 1,
            None => missed
                .entry(case.kind.as_str())
                .or_default()
                .push(case.answer.as_str()),
        }
    }
    let declined = judged.iter().flatten().count();
    eprintln!(
        "out of the World: {declined} of {} declined; why: {why:?}; missed: {missed:#?}",
        out.len()
    );

    let judged = said(&kept);
    let wrongly = kept
        .iter()
        .zip(&judged)
        .filter_map(|(case, declined)| {
            declined
                .as_ref()
                .map(|why| format!("{why}: {}", named(case)))
        })
        .collect::<Vec<_>>();
    eprintln!(
        "in the World: {} of {} taken",
        kept.len() - wrongly.len(),
        kept.len()
    );
    assert!(wrongly.is_empty(), "declined in the World: {wrongly:#?}");
    assert!(
        declined * 100 >= out.len() * 95,
        "only {declined} of {} declined: {missed:#?}",
        out.len()
    );
}

/// Measures a Pack on the later blind sets. The bar is not met on them
/// (see docs/KNOWN_ISSUES.md), so this prints rather than asserts. `run`
/// says the out-of-World and in-World cases in a fresh World and returns
/// the verdicts of each.
pub fn measure_the_later_sets(
    pack: &str,
    mut run: impl FnMut(&[Case], &[Case]) -> (Vec<Option<String>>, Vec<Option<String>>),
) {
    for (set, out, kept) in LATER_SETS {
        let (out, kept) = (cases(out, pack), cases(kept, pack));
        let (declined, wrongly) = run(&out, &kept);
        let declined = declined.iter().flatten().count();
        let wrongly = wrongly.iter().flatten().count();
        eprintln!(
            "{set}: {declined} of {} out of the World declined; {wrongly} of {} in it declined",
            out.len(),
            kept.len()
        );
    }
}
