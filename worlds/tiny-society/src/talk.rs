//! What the harbour's people say and how they look: a line over whoever a
//! moment happened to, the answers to what a player can ask them, and the
//! work clothes they wear. All of it is written from what the World
//! records, so it is the same every time the World is replayed, and none
//! of it is ever read back as World state.

use crate::model::{CONDITION, OPERATING_STATUS};
use crate::{
    BAKERY, EMMA, EVAN, JONAS, JONAS_BOAT, LEAN_REOPEN_BAKERY_COMMAND, LEO, MARA, MIA, NOAH,
    REOPEN_BAKERY_COMMAND, REPAIR_BOAT_COMMAND, RETAIN_WORKER_COMMAND, SOFIA,
    TAKE_JONAS_ON_COMMAND,
};
use society_basic::{CASH, JOB};
use world_core::{EntityId, Event, Value, World};
use world_projection::{Carry, Look, ProjectionCommand, SelectionId, Talk, Voice};

pub(crate) const RESIDENTS: [EntityId; 8] = [JONAS, MARA, LEO, EMMA, MIA, NOAH, EVAN, SOFIA];

fn text(world: &World, id: EntityId, key: &str) -> Option<String> {
    match world.state().entity(id)?.component(key)? {
        Value::Text(value) => Some(value.clone()),
        _ => None,
    }
}

fn integer(world: &World, id: EntityId, key: &str) -> Option<i64> {
    match world.state().entity(id)?.component(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

/// Each resident in the clothes of their work, carrying its tool: Jonas in
/// a yellow oilskin with a fish, Mara in her apron with a loaf.
pub(crate) fn look(id: EntityId) -> Option<Look> {
    let (clothes, hair, skin, carries) = match id {
        JONAS => (0xe8b33c, 0x5a3b22, 0xd9a27a, Some(Carry::Fish)),
        MARA => (0xc8553d, 0x8a4b2a, 0xf0c7a2, Some(Carry::Bread)),
        LEO => (0x3f5f4a, 0x2a2a2a, 0xa86b45, Some(Carry::Mug)),
        EMMA => (0x6f5aa8, 0xd8b25a, 0xf2cfae, Some(Carry::Book)),
        MIA => (0xe06f8b, 0x3a2418, 0x8d5a3b, Some(Carry::Satchel)),
        NOAH => (0x2e4a7a, 0x9a9a9a, 0xe3b590, None),
        EVAN => (0x8a5a33, 0xb0763e, 0xd8a27c, Some(Carry::Tool)),
        SOFIA => (0x3c9a8f, 0x1a1414, 0xc0875c, Some(Carry::Basket)),
        _ => return None,
    };
    Some(Look {
        clothes: Some(clothes),
        hair: Some(hair),
        skin: Some(skin),
        carries,
        bird: false,
    })
}

/// What someone says in passing on an ordinary day, by what their work
/// is. Five apiece, taken in turn day by day, so nobody says the same
/// thing twice in a working week.
fn everyday(job: &str) -> [&'static str; 5] {
    match job {
        "baker" => [
            "Bread's out of the oven.",
            "Flour everywhere, as usual.",
            "The rye's rising nicely.",
            "Sold the last loaf at noon.",
            "Up before the gulls again.",
        ],
        "pub_owner" => [
            "Pub's open!",
            "Barrels in the cellar, all full.",
            "Who's for a pint?",
            "Fire's lit in the snug.",
            "Quiet in the bar tonight.",
        ],
        "teacher" => [
            "Class dismissed.",
            "Spelling test tomorrow!",
            "They're learning the tides.",
            "Chalk dust in my hair again.",
            "Such good questions today.",
        ],
        "student" => [
            "Homework's done.",
            "Emma gave us sums again.",
            "I came top in spelling!",
            "Can we go to the beach?",
            "I'm reading about whales.",
        ],
        "carpenter" => [
            "Another plank down.",
            "Measure twice, cut once.",
            "Smell that fresh pine.",
            "Hinges oiled, door's hung.",
            "Sawdust in my boots.",
        ],
        "shop_assistant" => [
            "Next customer, please!",
            "Jam's flying off the shelf.",
            "Counted the till twice.",
            "New stock in off the ferry.",
            "Busy day on the counter.",
        ],
        "mayor" => [
            "Town business as usual.",
            "Minutes to write, again.",
            "The harbour wall needs looking at.",
            "Letters from the mainland.",
            "A fine day for the island.",
        ],
        "fisher" => [
            "Back from the sea.",
            "Wind's in the east today.",
            "Gulls followed me all the way in.",
            "Sea's like glass this morning.",
            "Salt in everything, as usual.",
        ],
        "bakery_temp" | "bakery_counter" => [
            "Swept the bakery floor.",
            "Mara's teaching me the rolls.",
            "Flour in my beard now.",
            "Carried sacks all morning.",
            "Counter's spotless.",
        ],
        "unemployed" => [
            "Another day looking for work.",
            "Asked round the quay again.",
            "Nothing going, they say.",
            "I'll find something.",
            "Long day with nothing to do.",
        ],
        _ => [
            "That's the day's work done.",
            "Busy day.",
            "Mustn't grumble.",
            "Tired, but happy.",
            "Same again tomorrow.",
        ],
    }
}

/// Which of five it is today.
fn today(event: &Event) -> usize {
    (event.world_time / crate::persistence::WORLD_DAY_TICKS % 5) as usize
}

/// What someone says in passing today: what they still remember from the
/// last few days, or else the day's line for their work.
fn in_passing(world: &World, event: &Event, who: EntityId) -> String {
    if let Some(memory) = crate::story::remembered(world, who) {
        return memory.into();
    }
    let job = text(world, who, JOB).unwrap_or_default();
    everyday(&job)[today(event)].into()
}

/// Who says the line for a moment, and what they say.
fn said(world: &World, event: &Event) -> Option<(EntityId, String)> {
    if let Some(said) = crate::story::line(event) {
        world.state().entity(said.0)?;
        return Some(said);
    }
    let actor = event.actor;
    let (speaker, line): (EntityId, String) = match event.kind.as_str() {
        "support_requested" => (
            JONAS,
            "Could you spare something till I'm back at sea?".into(),
        ),
        "support_received" => (LEO, "Here. Pay me back when you can.".into()),
        "support_repaid" => (JONAS, "Every coin, like I said.".into()),
        "fish_sold" => (
            JONAS,
            [
                "Good catch. It's off to the mainland.",
                "Mainland buyers took the lot.",
                "Fair price for the catch today.",
                "Fish on the ferry, coins in my pocket.",
                "Another crate sold.",
            ][today(event)]
            .into(),
        ),
        "boat_damaged" => (JONAS, "The storm's wrecked Sea Finch.".into()),
        "boat_repaired" => (EVAN, "She's sound again. Take her out.".into()),
        "boat_sold" => (JONAS, "That's it, then. No more fishing.".into()),
        "bakery_closed" => (MARA, "I can't keep the doors open.".into()),
        "bakery_reopened" => (MARA, "Fresh bread tomorrow, same as ever.".into()),
        "bakery_reopened_lean" => (MARA, "Just me behind the counter now.".into()),
        "worker_dismissed" => (MARA, "I'm sorry, Jonas. I can't keep you.".into()),
        "worker_retained" => (MARA, "One more chance, Jonas.".into()),
        "order_lost" => (MARA, "We've lost the wedding order.".into()),
        "temporary_work_assigned" => (JONAS, "I'll take whatever work there is.".into()),
        "loan_requested" => (JONAS, "Leo, could I borrow a little?".into()),
        "work_sought" => (JONAS, "Mara, is there any work going?".into()),
        "jonas_taken_on" => (MARA, "Start tomorrow at the counter.".into()),
        "counter_help_hired" => (MARA, "Mia, can you start at the counter?".into()),
        "living_cost_unmet" => (JONAS, "I can't cover today.".into()),
        "hardship_began" => (JONAS, "I'll have to dip into my savings.".into()),
        "hardship_eased" => (JONAS, "I'm paying my own way again.".into()),
        "payroll_shortfall" => (MARA, "I can't make the wages this week.".into()),
        "backing_withdrawn" => (LEO, "I've put my money elsewhere.".into()),
        "bread_budget_cut" => (actor?, "Less bread for me for a while.".into()),
        "income_disrupted" => (actor?, "Money's tight this week.".into()),
        "catch_landed" | "bread_purchased" | "work_shift_completed" => {
            let actor = actor?;
            (actor, in_passing(world, event, actor))
        }
        _ => return None,
    };
    world.state().entity(speaker)?;
    Some((speaker, line))
}

/// Everything worth drawing as speech: every moment of the story, and the
/// everyday things said at the latest moment. Each person says one thing
/// a moment, their first; older everyday chatter is left out.
pub(crate) fn voices(world: &World) -> Vec<Voice> {
    let latest = world.world_time();
    let mut spoken_now = std::collections::BTreeSet::new();
    let mut voices = Vec::new();
    for event in world.events() {
        let story = crate::projection::narrated_title(world, event).is_some();
        if !story && event.world_time != latest {
            continue;
        }
        let Some((speaker, line)) = said(world, event) else {
            continue;
        };
        if !story && !spoken_now.insert(speaker) {
            continue;
        }
        voices.push(Voice {
            moment: SelectionId::Event(event.id),
            speaker: SelectionId::Entity(speaker),
            line,
        });
    }
    voices
}

/// What someone would ask for, among the choices on offer now.
fn request(
    world: &World,
    who: EntityId,
    commands: &[ProjectionCommand],
) -> Option<(String, Option<String>)> {
    if let Some((line, grant)) = crate::story::wanting(world, who) {
        let grant = grant.filter(|id| commands.iter().any(|command| &command.id == id));
        return Some((line, grant));
    }
    let command = commands
        .iter()
        .find(|command| command.asker == Some(SelectionId::Entity(who)))?;
    let line = match command.id.as_str() {
        RETAIN_WORKER_COMMAND => "Another chance at the bakery. I won't waste it.",
        REPAIR_BOAT_COMMAND => "Sea Finch fixed. Then I can fish again.",
        TAKE_JONAS_ON_COMMAND => "Work at the counter, if Mara will have me.",
        REOPEN_BAKERY_COMMAND | LEAN_REOPEN_BAKERY_COMMAND => "Help to open the doors again.",
        _ => return None,
    };
    Some((line.into(), Some(command.id.clone())))
}

/// What a player can ask each resident, and what they answer, from how
/// things stand: how they are, how the harbour is, what they need.
pub(crate) fn talks(world: &World, commands: &[ProjectionCommand]) -> Vec<Talk> {
    let bakery_open = text(world, BAKERY, OPERATING_STATUS).as_deref() != Some("closed");
    let boat_broken = text(world, JONAS_BOAT, CONDITION).as_deref() == Some("damaged");
    let mut talks = Vec::new();
    for who in RESIDENTS {
        if world.state().entity(who).is_none() {
            continue;
        }
        let person = SelectionId::Entity(who);
        let job = text(world, who, JOB).unwrap_or_default();
        let cash = integer(world, who, CASH).unwrap_or(0);
        let kindness = crate::story::kindness(world, who);
        let how = if who == JONAS && boat_broken {
            "Sea Finch is broken, and so am I, nearly.".to_string()
        } else if job == "unemployed" {
            "Worried. I need work.".into()
        } else if cash < 50 {
            "Getting by. Just about.".into()
        } else if kindness.1 > kindness.0 {
            "Sore. Nobody listens when I ask for anything.".into()
        } else if kindness.0 > 0 && kindness.0 > kindness.1 {
            "Glad. People here look out for me.".into()
        } else {
            match job.as_str() {
                "baker" => "Busy. The ovens don't wait.".into(),
                "pub_owner" => "Can't complain. Pub's warm.".into(),
                "teacher" => "Tired, but the children are learning.".into(),
                _ => "Can't complain.".into(),
            }
        };
        talks.push(Talk {
            who: person,
            question: "How are you?".into(),
            answer: how,
            asks_for: None,
        });
        talks.push(Talk {
            who: person,
            question: "How's the harbour?".into(),
            answer: match (bakery_open, crate::story::spirits(world)) {
                (false, _) => "The bakery's shut. Everyone feels it.".into(),
                (true, 3..) => "In fine spirits, all of us.".into(),
                (true, ..=-3) => "Glum. Everyone's short with each other.".into(),
                _ => "Quiet, but we're managing.".into(),
            },
            asks_for: None,
        });
        let (need, asks_for) = match request(world, who, commands) {
            Some((line, command)) => (line, command),
            None => ("Nothing much. A good day's trade.".to_string(), None),
        };
        talks.push(Talk {
            who: person,
            question: "What do you need?".into(),
            answer: need,
            asks_for,
        });
    }
    talks
}
