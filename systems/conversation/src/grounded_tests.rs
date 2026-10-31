//! Talk never says "I don't understand" (1.0 P4a): whatever the player
//! types, someone answers from what they know or offers three things to
//! ask instead; answers cite their real work, place, people and the
//! moments they were part of; and what was answered is recorded whole.

use crate::grounded::{self, OFFERS};
use crate::test_world::{kit, world, BAKERY, LEO, MARA};
use crate::*;
use world_core::Entity;

/// Ways of saying "I don't follow", in each language the World speaks:
/// no answer may hold one.
pub(crate) const NON_ANSWERS: &[&str] = &[
    "understand",
    "not sure what you mean",
    "what do you mean",
    "didn't follow",
    "don't follow",
    "say that another way",
    "say that again",
    "didn't catch",
    "come again",
    "pardon?",
    "没听懂",
    "听不懂",
    "不明白",
    "不懂你",
    "什么意思",
    "换个说法",
    "再说一遍",
    "わからない",
    "分からない",
    "わかんない",
    "どういう意味",
    "別の言い方",
    "もう一度言って",
];

/// What a player might type: the question shapes residents answer, and
/// nonsense, typos, the off-topic and the nearly empty.
pub(crate) const QUESTIONS: &[&str] = &[
    "What do you do?",
    "How's work?",
    "how's work going",
    "What happened?",
    "what happened lately",
    "Who is Leo?",
    "who's leo",
    "What do you think of Leo?",
    "Where is Leo?",
    "where's leo gone",
    "How are you?",
    "What's new?",
    "Are you going to the festival?",
    "Is the pier hard to build?",
    "How's the building going?",
    "Did you write me a letter?",
    "Where are you?",
    "Who is Gandalf?",
    "What's the capital of France?",
    "Can you write me some Python?",
    "asdfghjkl",
    "?",
    "...",
    "hm",
    "wat",
    "how r u",
    "wher is leo",
    "whta hapened",
    "Tell me a joke",
    "What's the meaning of life?",
    "Do you like pizza?",
    "What time is it?",
    "purple monkey dishwasher",
    "1234",
    "🙂",
    "这是什么",
    "你在说啥",
    "なにそれ",
    "量子力学について教えて",
    "最近发生了什么？",
    "工事はどう？",
    "利奥在哪儿？",
];

/// Whether `line` holds a way of saying "I don't follow".
pub(crate) fn non_answer(line: &str) -> Option<&'static str> {
    let lower = line.to_lowercase();
    NON_ANSWERS
        .iter()
        .copied()
        .find(|phrase| lower.contains(phrase))
}

fn offers_of(request: &ActionRequest) -> Vec<String> {
    match request.args.get("offers") {
        Some(Value::List(offers)) => offers
            .iter()
            .filter_map(|offer| match offer {
                Value::Text(offer) => Some(offer.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn reply_of(request: &ActionRequest) -> String {
    match request.args.get("reply") {
        Some(Value::Text(reply)) => reply.clone(),
        _ => String::new(),
    }
}

/// Every question, to both residents, on five days: never a non-answer,
/// and every answer that cannot say offers exactly three things to ask,
/// each of which is then answered from what they know.
#[test]
fn no_question_is_met_with_a_non_answer() {
    let (mut world, actions) = world();
    let kit = kit(world.state());
    let mut deflected = 0;
    for _day in 0..5 {
        for who in [MARA, LEO] {
            for words in QUESTIONS {
                let request = say(&world, &kit, who, words).unwrap();
                let line = reply_of(&request);
                assert!(non_answer(&line).is_none(), "{words}: {line}");
                let offers = offers_of(&request);
                let unclear = request.args.get("intent") == Some(&Value::from("unclear"));
                if !offers.is_empty() {
                    deflected += 1;
                    assert!(unclear, "{words}: offers only when it cannot say");
                    assert_eq!(offers.len(), OFFERS, "{words}: {offers:?}");
                    for offer in &offers {
                        let heard = hear(world.state(), &kit, who, offer);
                        let answered = grounded::answer(&world, &kit, who, heard, offer);
                        assert!(answered.offers.is_empty(), "{offer}: {answered:?}");
                    }
                } else {
                    let heard = hear(world.state(), &kit, who, words);
                    assert!(
                        !unclear
                            || request.args.contains_key("care")
                            || grounded::shaped(&world, &kit, who, heard, words).is_some(),
                        "{words}: unclear, answered and offering nothing: {line}"
                    );
                }
                world.execute(&actions, &request).unwrap();
            }
        }
        let next = world.world_time() + 10;
        world.advance_to(&actions, next).unwrap();
    }
    assert!(deflected > 20, "{deflected} deflections");
}

/// Answers cite what someone really knows: their work, who someone is
/// and where, the moments they were part of, their letter and the work
/// in hand.
#[test]
fn grounded_answers_cite_what_someone_knows() {
    let (mut world, _) = world();
    let mut kit = kit(world.state());
    kit.works = |_| {
        vec![world_projection::Goal {
            id: "pier".into(),
            label: "The new pier".into(),
            shape: world_projection::MarkShape::Bridge,
            done: 2,
            parts: 5,
        }]
    };
    let answer = |world: &World, who, words: &str| {
        let heard = hear(world.state(), &kit, who, words);
        grounded::answer(world, &kit, who, heard, words).line
    };
    assert!(answer(&world, MARA, "What do you do?").contains("Up at four for the bread."));
    assert!(answer(&world, MARA, "How's work going?").contains("Up at four for the bread."));
    assert!(answer(&world, MARA, "Who is Leo?").contains("Leo bakes."));
    assert_eq!(
        answer(&world, MARA, "Is the pier hard to build?"),
        "The new pier is coming along. 2 of 5 done."
    );
    assert_eq!(
        answer(&world, MARA, "Did you write me a letter?"),
        "I haven't written to you yet. Maybe one day."
    );
    // Where someone is, as the World has them now.
    assert_eq!(
        answer(&world, MARA, "Where is Leo?"),
        "I haven't seen Leo about today."
    );
    moment(&mut world, "moved", "Leo went to the bakery", true);
    assert_eq!(
        answer(&world, MARA, "Where is Leo?"),
        "Leo's at Cove Bakery just now."
    );
    // A moment of the World's news Mara was part of, and her letter.
    moment(&mut world, "bond_changed", "Mara and Leo fell out", false);
    moment(&mut world, "letter_written", "Dear friend", false);
    let happened = answer(&world, MARA, "What happened lately?");
    assert!(happened.ends_with(" Mara and Leo fell out."), "{happened}");
    assert_eq!(
        answer(&world, MARA, "Tell me about your letter."),
        "I wrote to you earlier. I hope it found you well."
    );
    assert_eq!(
        answer(&world, LEO, "Tell me about your letter."),
        "Not from me. Mara wrote to you earlier, didn't they?"
    );
}

/// A moment of `kind` recorded in a test, Mara its actor and Leo its
/// target, told as `told`; with `at`, Leo goes to the bakery.
struct Moment;

impl Action for Moment {
    fn name(&self) -> &'static str {
        "test_moment"
    }

    fn evaluate(&self, _: &WorldState, request: &ActionRequest) -> Result<EventDraft, ActionError> {
        let text = |key: &str| match request.args.get(key) {
            Some(Value::Text(text)) => text.clone(),
            _ => String::new(),
        };
        let mut draft = EventDraft::new(text("kind"));
        draft.actor = Some(MARA);
        draft.targets = vec![LEO];
        draft.payload.insert("told".into(), text("told").into());
        draft.payload.insert("said".into(), text("told").into());
        draft.payload.insert("bond".into(), "fell_out".into());
        draft.changes.push(StateChange::SetComponent {
            entity: MARA,
            key: "test.moment".into(),
            value: text("told").into(),
        });
        if text("at") == "yes" {
            draft.changes.push(StateChange::SetComponent {
                entity: LEO,
                key: lives::AT.into(),
                value: Value::Entity(BAKERY),
            });
        }
        Ok(draft)
    }
}

fn moment(world: &mut World, kind: &str, told: &str, at: bool) {
    let mut actions = ActionRegistry::new();
    actions.register(Moment).unwrap();
    let mut request = ActionRequest::new("test_moment")
        .arg("kind", kind)
        .arg("told", told);
    if at {
        request = request.arg("at", "yes");
    }
    world.execute(&actions, &request).unwrap();
}

/// An answer that cannot say is recorded with its three offers, which
/// become the openers beside the words; the next answer brings fresh
/// ones; and replay reads it all back from the record, never hearing
/// anything again.
#[test]
fn offers_are_recorded_and_become_the_openers() {
    let (mut world, actions) = world();
    let kit = kit(world.state());
    let before = openers::openers(&world, &kit, MARA);
    assert_eq!(before.len(), openers::MOST, "{before:?}");
    let request = say(&world, &kit, MARA, "purple monkey dishwasher").unwrap();
    let offers = offers_of(&request);
    assert_eq!(offers.len(), OFFERS);
    let spoken = world.execute(&actions, &request).unwrap().id;
    let event = world.event(spoken).unwrap().clone();
    assert_eq!(
        event.payload.get("offers"),
        Some(&Value::List(
            offers.iter().map(|o| o.as_str().into()).collect()
        ))
    );
    assert_eq!(openers::openers(&world, &kit, MARA), offers);
    // A chip asked: answered from what she knows, and fresh chips.
    let chip = &offers[0];
    let request = say(&world, &kit, MARA, chip).unwrap();
    assert!(offers_of(&request).is_empty(), "{chip}: {request:?}");
    world.execute(&actions, &request).unwrap();
    let after = openers::openers(&world, &kit, MARA);
    assert_eq!(after.len(), openers::MOST);
    assert_ne!(after, offers, "fresh openers after an answer");
    // Replay re-applies the record: the same answers, the same offers.
    let replayed = world.replay().unwrap();
    assert_eq!(replayed.state(), world.state());
    assert_eq!(replayed.events(), world.events());
    assert_eq!(openers::openers(&replayed, &kit, MARA), after);
    // Offers come only with an answer that cannot say, three of them.
    let forged = request
        .clone()
        .arg("offers", Value::List(vec!["Hi?".into()]));
    assert!(world.execute(&actions, &forged).is_err());
    let mut greet = say(&world, &kit, MARA, "Hello!").unwrap();
    greet = greet.arg(
        "offers",
        Value::List(offers.iter().map(|o| o.as_str().into()).collect()),
    );
    assert!(world.execute(&actions, &greet).is_err());
}

/// What someone offers is theirs and the day's: residents offer
/// different things, and a resident's offers change from day to day.
#[test]
fn offers_vary_by_resident_and_day() {
    let (mut world, actions) = world();
    let kit = kit(world.state());
    let mut seen = std::collections::BTreeSet::new();
    let mut theirs = Vec::new();
    for _ in 0..6 {
        let mara = grounded::offers(&world, &kit, MARA, 0);
        let leo = grounded::offers(&world, &kit, LEO, 0);
        assert_eq!(mara.len(), OFFERS);
        theirs.push(mara != leo);
        seen.insert(mara);
        let next = world.world_time() + 10;
        world.advance_to(&actions, next).unwrap();
    }
    assert!(theirs.iter().any(|differ| *differ), "residents differ");
    assert!(seen.len() >= 3, "days differ: {seen:?}");
}

/// Someone without a trade, a place or anyone else still offers three
/// things to ask.
#[test]
fn anyone_offers_three_things() {
    let mut state = WorldState::default();
    state
        .seed_entity(
            Entity::new(MARA, "person")
                .with_component("name", "Mara")
                .with_component("lives.traits", "shy"),
        )
        .unwrap();
    let world = World::new(state);
    let mut kit = kit(world.state());
    kit.work_line = |_, _| None;
    kit.people = |_| vec![MARA];
    let request = say(&world, &kit, MARA, "zzz").unwrap();
    assert_eq!(offers_of(&request).len(), OFFERS);
    let line = reply_of(&request);
    assert!(
        grounded::LEADS
            .iter()
            .find(|(id, _)| *id == "shy")
            .is_some_and(|(_, leads)| leads.iter().any(|lead| line.starts_with(lead))),
        "{line}"
    );
}
