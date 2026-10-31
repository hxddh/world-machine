//! Lives: the people of a World get on with their days whether or not
//! anyone is watching, the way people do in The Sims and RimWorld.
//!
//! Everyone has four needs (money, rest, company and purpose), two traits,
//! and an opinion of everyone else. Each period each person does something
//! about whichever need presses hardest: works, rests, spends the evening
//! with someone, helps someone out. Doing things together moves how two
//! people feel about each other, and that is where friendships, quarrels
//! and couples come from.
//!
//! From how people stand, this System also composes situations to put to
//! the player: two people who have fallen out, someone sweet on someone,
//! someone who wants to learn a trade, someone short of money or thinking
//! of leaving, a stranger asking for a room. Each is made of who, what and
//! over what, so a World does not run out of them.
//!
//! Everything goes through ordinary Actions and is recorded as Events with
//! the words it was told in, so replaying a World never needs this System
//! to decide anything again. It knows nothing about harbours or colonies:
//! a World Pack gives it a [`Cast`], with its people, places and words.

#![forbid(unsafe_code)]

mod voice;

pub mod generations;
pub use generations::{
    age_at, age_of, awaiting_memorial, born_here, children, grown, grown_here, lives_with,
    looks_at, looks_of, memorial_request, parents, Kin, KinWords, Memorial, Stage,
};

mod answers;
pub use answers::answer_label;
mod drawn;
pub use drawn::{
    draw_newcomers, drawn_by, drawn_request, drew, name_proposals, to_be_named, Draw, DRAWN_AFTER,
    DRAWN_BY, DRAWN_ROOM, NAMING_DAYS,
};
mod host;
mod pronouns;
mod retold;
pub use pronouns::{pronouns, they_all, Pronouns, A_SLOTS, B_SLOTS, PRONOUNS};
use retold::TOLD_NTH;
pub use retold::{cited, player_act, told_enough, year_length, Act, MOST_TOLD_A_YEAR};
mod quiet;
#[cfg(any(test, feature = "scan-reference"))]
#[doc(hidden)]
pub mod scanned;
pub mod shown;
mod suggest;
pub use host::{
    guests_staying, host_guest, host_guest_with, GuestWords, Staying, GUEST_STAY_PERIODS,
    MOST_GUEST_TEXT,
};
pub use quiet::{firsts, firsts_count, morning_letter, welcome_back, Corner, QuietDays, Subject};
mod bonds;
mod days;
mod letters;
mod outcomes;
mod reactions;
mod situations;
mod strangers;
pub use bonds::ROW;
use bonds::*;
pub use days::ASKED_APART_PERIODS;
use days::*;
use letters::*;
pub use letters::{
    daily, daily_with, keepsakes, letter_name, letter_writers, letters, possible_keepsakes,
    room_for_keepsake, Keepsake, Letter, KEEPSAKES_A_WEEK,
};
use outcomes::*;
use reactions::*;
pub use reactions::{
    give_keepsake, greet, greet_saying, leave_keepsake, match_at, react_to, remember_a_year,
    season_turns,
};
use situations::*;
pub use situations::{candidates, open, situations, Answer, Candidate, Kind, Situation};
use std::collections::{BTreeMap, BTreeSet};
use strangers::*;
pub use strangers::{met, people_to_meet, waiting_strangers, CROSSED_KINDS};
pub use suggest::{can_suggest, suggestion_request, Idea, IDEAS, SUGGEST_REST};
pub use voice::{keepsake_of, own_lines, restyle, scene_of, Scene, Voice};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, Event, EventDraft,
    EventId, StateChange, Value, World, WorldError, WorldState,
};

/// What someone can be short of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Need {
    Money,
    Rest,
    Company,
    Purpose,
}

impl Need {
    pub const ALL: [Need; 4] = [Need::Money, Need::Rest, Need::Company, Need::Purpose];

    /// Where how short someone is of it is kept: 0 wants for nothing, 100
    /// can think of nothing else.
    pub fn key(self) -> &'static str {
        match self {
            Need::Money => "lives.lack.money",
            Need::Rest => "lives.lack.rest",
            Need::Company => "lives.lack.company",
            Need::Purpose => "lives.lack.purpose",
        }
    }
}

/// Who someone does a thing with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum With {
    Alone,
    /// Whoever they like best.
    Friend,
    /// Anyone at all, the way people run into each other.
    Anyone,
    /// Someone they work alongside, or anyone if nobody does.
    Workmate,
}

/// Where someone does a thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Work,
    Home,
    /// Where people meet: the pub, the common room.
    Gathering,
    /// Where people go to be quiet.
    Quiet,
}

/// Something someone can do with a day.
#[derive(Clone, Copy, Debug)]
pub struct Activity {
    pub id: &'static str,
    /// The need it answers.
    pub need: Need,
    pub with: With,
    pub at: At,
    /// How it is told: "{name} had a drink with {other} at {place}".
    pub told: &'static str,
    /// What the person says about it, one taken in turn.
    pub said: &'static [&'static str],
    /// Integers it moves on the person, in the Pack's own keys (a day's
    /// pay, say).
    pub gives: &'static [(&'static str, i64)],
}

/// Somewhere the Pack keeps money the town can spend on its people.
#[derive(Clone, Copy, Debug)]
pub struct Fund {
    pub entity: EntityId,
    pub key: &'static str,
    /// What it is called: "the harbour fund".
    pub name: &'static str,
    pub amount: i64,
}

/// Strangers who might come by and stay.
#[derive(Clone, Copy, Debug)]
pub struct Visitors {
    /// The first entity id a newcomer can take; the next ones follow.
    pub first: u64,
    /// How many can ever arrive.
    pub room: u64,
    pub names: &'static [&'static str],
    /// What they do, as told and in the Pack's own job word.
    pub trades: &'static [(&'static str, &'static str)],
    pub origins: &'static [&'static str],
    /// How someone leaves: "the ferry", "the supply shuttle".
    pub way_out: &'static str,
    /// The components a newcomer starts with, from their name and job.
    pub components: fn(&str, &str) -> Vec<(String, Value)>,
    /// The kind of entity a newcomer is.
    pub kind: &'static str,
}

/// Everything the System needs to know about a World's people.
#[derive(Clone, Debug)]
pub struct Cast {
    /// The entity the System keeps its notes on.
    pub notes: EntityId,
    /// How much world time one period is.
    pub period: u64,
    /// A period, in the World's own word: "day", "sol".
    pub unit: &'static str,
    /// The place as a whole: "harbour", "colony".
    pub settlement: &'static str,
    /// What someone short of means runs out of: "money", "fish".
    pub short_of: &'static str,
    /// Everyone living there now, in a fixed order.
    pub people: fn(&World) -> Vec<EntityId>,
    /// Whether someone is part of the place for good and never leaves.
    pub stays: fn(EntityId) -> bool,
    /// Two traits for each of the people the World starts with.
    pub traits: fn(EntityId) -> Option<[&'static str; 2]>,
    /// Two people whose standing with each other is the Pack's own story,
    /// which this System leaves alone.
    pub kept: fn(EntityId, EntityId) -> bool,
    /// How the place feels as a whole, in the Pack's terms, from -10
    /// (bitter) to 10 (bright): it colours everyone's days, so what the
    /// player decides reaches into people's lives.
    pub mood: fn(&WorldState) -> i64,
    /// Where someone works and where they live, if anywhere.
    pub work: fn(&WorldState, EntityId) -> Option<EntityId>,
    pub home: fn(&WorldState, EntityId) -> Option<EntityId>,
    /// Where people meet, and where they go to be quiet.
    pub gathering: EntityId,
    pub quiet: EntityId,
    /// Who welcomes strangers.
    pub host: EntityId,
    pub activities: &'static [Activity],
    /// What people fall out over: "the mooring fees".
    pub topics: &'static [&'static str],
    /// What someone sweet on someone might ask them to: "walk out to the
    /// point".
    pub outings: &'static [&'static str],
    pub fund: Option<Fund>,
    pub visitors: Option<Visitors>,
    /// The most people the place holds.
    pub most_people: usize,
    /// How many situations may be open at once.
    pub most_open: usize,
    /// How someone speaks, for the people the Pack gives a voice; anyone
    /// else speaks from their traits.
    pub voice: fn(EntityId) -> Option<&'static Voice>,
    /// How the place keeps its generations (ages, births and deaths), if
    /// it does.
    pub kin: Option<&'static Kin>,
    /// Whether the place is in its first kind days, as a new player
    /// arrives: nobody has words, falls out or drifts apart. [`never`]
    /// for a Pack that keeps none.
    pub kind: fn(&WorldState) -> bool,
    /// Whether a line speaks of someone as he, she or they, from what the
    /// Pack already knows of them; [`they_all`] for a Pack that says no
    /// more. Someone's own [`PRONOUNS`] come first.
    pub pronouns: fn(&WorldState, EntityId) -> Pronouns,
}

/// No kind days: for [`Cast::kind`].
pub fn never(_: &WorldState) -> bool {
    false
}

/// Traits someone can have, and the ones that grate on each other.
pub const TRAITS: [&str; 10] = [
    "warm", "prickly", "proud", "shy", "sociable", "restless", "steady", "generous", "thrifty",
    "dreamy",
];

const CLASHES: [(&str, &str); 7] = [
    ("proud", "proud"),
    ("prickly", "proud"),
    ("prickly", "prickly"),
    ("thrifty", "generous"),
    ("restless", "steady"),
    ("sociable", "shy"),
    ("prickly", "sociable"),
];

/// How two people's opinion of each other is kept.
fn opinion_key(of: EntityId) -> String {
    format!("lives.opinion.{of}")
}

const TRAITS_KEY: &str = "lives.traits";
const PARTNER: &str = "lives.partner";
/// How someone feels about the place and the player's hand in it.
pub const REGARD: &str = "lives.regard";
/// Someone who has left.
pub const GONE: &str = "lives.gone";
const BOND: &str = "lives.bond";
const CELEBRATED: &str = "lives.celebrated";
/// How many times someone has asked for help through a hard time and been
/// turned away or left unanswered, less the times they were helped.
pub const UNHELPED: &str = "lives.unhelped";
/// Whom someone sweet on someone is waiting to ask.
const COURTING: &str = "lives.courting";
/// Whether they were told to wait, and so wait to be told otherwise.
const TOLD_TO_WAIT: &str = "lives.told_to_wait";
/// When someone last parted from someone they were walking out with.
const PARTED: &str = "lives.parted";
/// How long someone who has parted takes before they are sweet on anyone
/// again.
const HEALS: i64 = 300;

/// Whether someone parted from someone lately.
fn heartsore(state: &WorldState, person: EntityId, now: u64) -> bool {
    integer(state, person, PARTED).is_some_and(|at| (now as i64) - at < HEALS)
}

/// How fond two people must be of each other for one to ask the other out
/// with nobody to encourage them.
const ASKS_ALONE: i64 = 60;
/// How little one of a couple can think of the other before they hit a
/// rough patch.
const ROUGH: i64 = 25;
/// How many hard times gone unhelped make someone who has not settled
/// think of leaving.
pub const LET_DOWN: i64 = 3;
/// When a door friendship opens was opened with someone, by kind.
fn door_key(kind: Kind) -> String {
    format!("lives.door.{}", kind.id())
}

/// The period a door opened with someone, if it has.
pub fn door_opened(state: &WorldState, person: EntityId, kind: Kind) -> Option<i64> {
    integer(state, person, &door_key(kind))
}

/// How warmly someone must regard the player to tell them a secret, and
/// to offer a favour or give a keepsake.
pub const WARM: i64 = 20;
pub const CLOSE: i64 = 50;
/// How warmly someone must regard the player to share a first moment:
/// more than everyday life brings on its own, so it takes the player's
/// doing; and to ask them along somewhere of theirs.
pub const FOND: i64 = 16;
pub const DEAR: i64 = 35;
/// How low someone's regard for the player falls before they hold a
/// grudge: they stop asking for help, and let it show.
pub const GRUDGE: i64 = -20;

/// Where someone is now: where their day took them, or where an answer
/// sent them.
pub const AT: &str = "lives.at";

fn integer(state: &WorldState, entity: EntityId, key: &str) -> Option<i64> {
    match state.entity(entity)?.component(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

fn text<'a>(state: &'a WorldState, entity: EntityId, key: &str) -> Option<&'a str> {
    match state.entity(entity)?.component(key)? {
        Value::Text(value) => Some(value.as_str()),
        _ => None,
    }
}

fn entity_of(state: &WorldState, entity: EntityId, key: &str) -> Option<EntityId> {
    match state.entity(entity)?.component(key)? {
        Value::Entity(id) => Some(*id),
        _ => None,
    }
}

/// What someone or somewhere is called.
pub fn name(state: &WorldState, entity: EntityId) -> String {
    text(state, entity, "name")
        .map(str::to_string)
        .unwrap_or_else(|| "Someone".into())
}

/// What someone is called when people talk about them: their first name.
pub fn first_name(state: &WorldState, entity: EntityId) -> String {
    let name = name(state, entity);
    name.split_whitespace()
        .next()
        .map(str::to_string)
        .unwrap_or(name)
}

/// A small stable number: the same inputs always give the same answer, so
/// people live the same days on every replay.
pub fn mix(parts: &[u64]) -> u64 {
    parts.iter().fold(0x9e37_79b9_7f4a_7c15_u64, |hash, part| {
        (hash ^ part.wrapping_mul(0xff51_afd7_ed55_8ccd))
            .wrapping_mul(0xc4ce_b9fe_1a85_ec53)
            .rotate_left(23)
    })
}

fn pick<T>(items: &[T], seed: u64) -> Option<&T> {
    (!items.is_empty()).then(|| &items[(seed % items.len() as u64) as usize])
}

fn fill(template: &str, words: &[(&str, &str)]) -> String {
    // One pass along the template: each `{slot}` given a word is filled
    // in, anything else is kept as it is written. (People's lines are
    // filled for every activity every day, so no copy is made per slot.)
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let slot = &after[..close];
                match words.iter().find(|(name, _)| *name == slot) {
                    Some((_, word)) => out.push_str(word),
                    None => {
                        out.push('{');
                        out.push_str(slot);
                        out.push('}');
                    }
                }
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(&rest[open..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// [`fill`] as it was first written: the reference it is checked against.
#[cfg(test)]
fn fill_every_slot(template: &str, words: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (slot, word) in words {
        out = out.replace(&format!("{{{slot}}}"), word);
    }
    out
}

/// Which period this is.
pub fn period(state: &WorldState, cast: &Cast) -> u64 {
    state.world_time() / cast.period.max(1)
}

/// Someone's two traits.
pub fn traits(state: &WorldState, person: EntityId) -> Vec<String> {
    text(state, person, TRAITS_KEY)
        .map(|traits| traits.split(' ').map(str::to_string).collect())
        .unwrap_or_default()
}

/// How well two people's natures sit together, from -3 to 3.
fn compatibility(state: &WorldState, a: EntityId, b: EntityId) -> i64 {
    let (a, b) = (traits(state, a), traits(state, b));
    let mut score = 0;
    for x in &a {
        for y in &b {
            if x == y && x != "proud" && x != "prickly" {
                score += 1;
            }
            if CLASHES
                .iter()
                .any(|(p, q)| (p == x && q == y) || (p == y && q == x))
            {
                score -= 2;
            }
        }
        if x == "warm" || x == "generous" {
            score += 1;
        }
    }
    for y in &b {
        if y == "warm" {
            score += 1;
        }
    }
    score.clamp(-3, 3)
}

/// What `a` thinks of `b`, from -100 to 100.
pub fn opinion(state: &WorldState, a: EntityId, b: EntityId) -> i64 {
    integer(state, a, &opinion_key(b)).unwrap_or(0)
}

/// How short someone is of a need.
pub fn lack(state: &WorldState, person: EntityId, need: Need) -> i64 {
    integer(state, person, need.key()).unwrap_or(0)
}

/// Whoever someone is with, if anyone.
pub fn partner(state: &WorldState, person: EntityId) -> Option<EntityId> {
    entity_of(state, person, PARTNER).filter(|other| state.entity(*other).is_some())
}

/// Whether someone has taken part in the System's life yet.
pub fn enrolled(state: &WorldState, person: EntityId) -> bool {
    text(state, person, TRAITS_KEY).is_some()
}

/// Where someone is now, if the System has put them anywhere.
pub fn at(state: &WorldState, person: EntityId) -> Option<EntityId> {
    entity_of(state, person, AT)
}

/// Whether someone has left.
pub fn gone(state: &WorldState, person: EntityId) -> bool {
    matches!(
        state
            .entity(person)
            .and_then(|entity| entity.component(GONE)),
        Some(Value::Bool(true))
    )
}

/// Marks a newcomer who has settled for good: they no longer leave.
pub const SETTLED: &str = "lives.settled";

/// How many of someone's hard times have gone unhelped, less those helped.
pub fn unhelped(state: &WorldState, person: EntityId) -> i64 {
    integer(state, person, UNHELPED).unwrap_or(0)
}

/// Whether someone has settled for good.
pub fn settled(state: &WorldState, person: EntityId) -> bool {
    matches!(
        state
            .entity(person)
            .and_then(|entity| entity.component(SETTLED)),
        Some(Value::Bool(true))
    )
}

/// The newcomers who have arrived and not left.
pub fn arrivals(state: &WorldState, cast: &Cast) -> Vec<EntityId> {
    let Some(visitors) = cast.visitors else {
        return Vec::new();
    };
    (visitors.first..visitors.first + visitors.room)
        .map(EntityId::new)
        .filter(|id| state.entity(*id).is_some() && !gone(state, *id))
        .collect()
}

/// Registers the System's Actions for a Pack's cast.
pub fn register_actions(
    registry: &mut ActionRegistry,
    cast: fn(&WorldState) -> Cast,
) -> Result<(), ActionError> {
    registry.register(Enrols(cast))?;
    registry.register(Lives(cast))?;
    registry.register(Bonds(cast))?;
    registry.register(Opens(cast))?;
    registry.register(Answers(cast))?;
    registry.register(Forgets(cast))?;
    registry.register(Reacts(cast))?;
    registry.register(LeavesKeepsake(cast))?;
    registry.register(Greets(cast))?;
    registry.register(Warms(cast))?;
    registry.register(Matches(cast))?;
    registry.register(Remembers)?;
    registry.register(suggest::Suggests(cast))?;
    registry.register(host::Hosts(cast))?;
    registry.register(quiet::Firsts(cast))?;
    registry.register(drawn::Drawn(cast))?;
    generations::register_actions(registry, cast)?;
    Ok(())
}

/// How long a situation stays open before it runs out.
const LASTS: i64 = 3;

/// The request that answers a situation.
pub fn answer_request(situation: &str, answer: &str) -> ActionRequest {
    ActionRequest::new("lives_situation_answered")
        .arg("situation", situation)
        .arg("answer", answer)
}

/// [`answer_request`], naming as its cause the Event the situation came up
/// in: an answer, or a lapse, follows what it answers.
pub fn answer_request_in(world: &World, situation: &str, answer: &str) -> ActionRequest {
    let request = answer_request(situation, answer);
    let index = world.history_index();
    let came_up = index
        .of_kind("situation_came_up")
        .iter()
        .rev()
        .copied()
        .find(|id| {
            world.event(*id).is_some_and(|event| {
                event.payload.get("situation") == Some(&Value::Text(situation.into()))
            })
        });
    match came_up {
        Some(came_up) => request.caused_by(came_up),
        None => request,
    }
}

fn others_arg(people: &[EntityId]) -> Value {
    Value::List(people.iter().map(|id| Value::Entity(*id)).collect())
}

/// One period of everyone's lives. New people join in, everyone does
/// something with their day, standings that changed are told, situations
/// that ran out end, and something new may come up to put to the player.
/// While the player is away, people live on, but nothing is put to them
/// and nothing they were asked runs out.
pub fn tick(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    away: bool,
) -> Result<Vec<EventId>, WorldError> {
    tick_holding(world, actions, cast, away, false)
}

/// One period of everyone's lives, as [`tick`], but with nothing new put
/// to the player while `hold` is set: a new World waits for the player's
/// first deed before anyone asks them anything.
pub fn tick_holding(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    away: bool,
    hold: bool,
) -> Result<Vec<EventId>, WorldError> {
    tick_with(world, actions, cast, away, hold, &QuietDays::default())
}

/// The last day `a` and `b` spent together, within `span` of world time.
fn last_day_together(world: &World, a: EntityId, b: EntityId, span: u64) -> Option<EventId> {
    let since = world.world_time().saturating_sub(span);
    let index = world.history_index();
    index
        .of_kind("lived")
        .iter()
        .rev()
        .filter_map(|id| world.event(*id))
        .take_while(|event| event.world_time >= since)
        .find(|event| {
            (event.actor == Some(a) && event.targets.contains(&b))
                || (event.actor == Some(b) && event.targets.contains(&a))
        })
        .map(|event| event.id)
}

/// One period of everyone's lives, as [`tick_holding`], with quiet days
/// kept as the Pack asks ([`QuietDays`]).
pub fn tick_with(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    away: bool,
    hold: bool,
    quiet: &QuietDays,
) -> Result<Vec<EventId>, WorldError> {
    let mut events = Vec::new();
    let people = living(world, cast);
    for person in &people {
        if !enrolled(world.state(), *person) {
            let request = ActionRequest::new("lives_enrol")
                .arg("person", Value::Entity(*person))
                .arg("others", others_arg(&people));
            events.push(world.execute(actions, &request)?.id);
        }
    }
    let before = pairs(world.state(), &people)
        .into_iter()
        .filter(|(a, b)| !(cast.kept)(*a, *b) && !(cast.kept)(*b, *a))
        .collect::<Vec<_>>();
    // The day each pair spent together, if they did: what a change
    // between them today came of.
    let mut together = BTreeMap::<(EntityId, EntityId), EventId>::new();
    for person in &people {
        let request = ActionRequest::new("lives_day")
            .arg("person", Value::Entity(*person))
            .arg("others", others_arg(&people));
        if let Ok(event) = world.execute(actions, &request) {
            if let Some(other) = event.targets.first() {
                let pair = (*person.min(other), *person.max(other));
                together.insert(pair, event.id);
            }
            events.push(event.id);
        }
    }
    for (a, b) in before {
        let mut request = ActionRequest::new("lives_bond")
            .arg("a", Value::Entity(a))
            .arg("b", Value::Entity(b));
        let day = together.get(&(a.min(b), a.max(b))).copied().or_else(|| {
            // Growing close or falling out, on a day apart: what came of
            // the last day they spent together, lately.
            let state = world.state();
            let was = text(state, a, &bond_key(b)).unwrap_or("");
            let now = standing(state, a, b, was);
            (now != was && !now.is_empty())
                .then(|| last_day_together(world, a, b, 30 * cast.period.max(1)))
                .flatten()
        });
        if let Some(day) = day {
            request = request.caused_by(day);
        }
        if let Ok(event) = world.execute(actions, &request) {
            events.push(event.id);
        }
    }
    if period(world.state(), cast).is_multiple_of(30) {
        if let Ok(event) = world.execute(actions, &ActionRequest::new("lives_forget")) {
            events.push(event.id);
        }
    }
    // Births and deaths, growing up and growing old.
    events.extend(generations::tick(world, actions, cast)?);
    if away {
        events.extend(daily_with(world, actions, cast, true, quiet)?);
        return Ok(events);
    }
    let now = period(world.state(), cast) as i64;
    for (key, opened) in open(world.state(), cast) {
        if now - opened >= LASTS {
            let lapse = answer_request_in(world, &key, "lapse");
            events.push(world.execute(actions, &lapse)?.id);
        }
    }
    // Someone warming to the player shares a first small moment with them,
    // unasked: the first of the five a friendship opens.
    let warming = {
        let state = world.state();
        living(world, cast).into_iter().find(|person| {
            enrolled(state, *person)
                && !gone(state, *person)
                && regard(state, *person) >= FOND
                && door_opened(state, *person, Kind::Warming).is_none()
        })
    };
    if let Some(person) = warming {
        let request = ActionRequest::new("lives_warms")
            .actor(person)
            .arg("who", Value::Entity(person));
        if let Ok(event) = world.execute(actions, &request) {
            events.push(event.id);
        }
    }
    if !hold && open(world.state(), cast).len() < cast.most_open {
        // The best candidate not asked lately.
        for candidate in candidates(world, cast).into_iter().take(24) {
            let request =
                ActionRequest::new("lives_situation_opens").arg("situation", candidate.key());
            if let Ok(event) = world.execute(actions, &request) {
                events.push(event.id);
                break;
            }
        }
    }
    if !hold {
        events.extend(daily_with(world, actions, cast, false, quiet)?);
    }
    Ok(events)
}

fn pairs(state: &WorldState, people: &[EntityId]) -> Vec<(EntityId, EntityId)> {
    let mut pairs = Vec::new();
    for (index, a) in people.iter().enumerate() {
        for b in &people[index + 1..] {
            if enrolled(state, *a) && enrolled(state, *b) {
                pairs.push((*a, *b));
            }
        }
    }
    pairs
}

/// Whether an event is one of this System's.
pub fn is_life(event: &Event) -> bool {
    matches!(
        event.kind.as_str(),
        "lived"
            | "bond_changed"
            | "situation_came_up"
            | "situation_answered"
            | "situation_lapsed"
            | "reacted"
            | "keepsake_left"
            | "letter_written"
            | "greeted"
            | "warmed"
            | "year_turned"
            | "year_remembered"
            | "suggestion_held"
            | "guest_visited"
            | "first_mentioned"
            | "corner_shown"
            | "born"
            | "came_of_age"
            | "left_home"
            | "retired"
            | "died"
            | "heirloom_passed"
            | "memorial_placed"
            | "anniversary_kept"
            | "drawn_here"
    )
}

/// How one of this System's moments is told, in the words it was recorded
/// with.
pub fn told(event: &Event) -> Option<String> {
    if !is_life(event) {
        return None;
    }
    match event.payload.get("told") {
        Some(Value::Text(told)) => Some(told.clone()),
        _ => None,
    }
}

/// Who speaks at one of this System's moments, and what they say.
pub fn said(event: &Event) -> Option<(EntityId, String)> {
    if !is_life(event) {
        return None;
    }
    match event.payload.get("said") {
        Some(Value::Text(said)) if !said.is_empty() => Some((event.actor?, said.clone())),
        _ => None,
    }
}

/// Whether a moment is worth a line in the World's history: not everyday
/// life, which is told as it happens, but changes between people and the
/// situations put to the player.
pub fn is_news(event: &Event) -> bool {
    NEWS_KINDS.contains(&event.kind.as_str())
}

/// The kinds of moment [`is_news`] counts as news.
pub const NEWS_KINDS: [&str; 17] = [
    "bond_changed",
    "drawn_here",
    "born",
    "came_of_age",
    "left_home",
    "retired",
    "died",
    "heirloom_passed",
    "memorial_placed",
    "anniversary_kept",
    "situation_came_up",
    "situation_answered",
    "situation_lapsed",
    "year_turned",
    "suggestion_held",
    "first_mentioned",
    "corner_shown",
];

/// What changed between people since a moment: who became friends, who
/// fell out, who got together or parted, who came and who left. Newest
/// last, one line each.
pub fn news_since(world: &World, since: u64) -> Vec<String> {
    news_events_since(world, since)
        .into_iter()
        .map(|(told, _)| told)
        .collect()
}

/// [`news_since`], each line with the Event it tells.
pub(crate) fn news_events_since(world: &World, since: u64) -> Vec<(String, &Event)> {
    let mut seen = BTreeSet::new();
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time >= since)
        .filter(|event| match event.kind.as_str() {
            "bond_changed" | "born" | "came_of_age" | "retired" | "died" | "left_home"
            | "drawn_here" => true,
            "situation_answered" | "situation_lapsed" => matches!(
                (event.payload.get("kind"), event.payload.get("answer")),
                (Some(Value::Text(kind)), Some(Value::Text(answer)))
                    if (kind == "sweet" && answer == "ask")
                        || (kind == "visitor" && answer != "decline")
                        || (kind == "leaving" && answer != "stay")
                        || (kind == "rough" && answer != "talk")
                        || (kind == "party" && answer == "party")
            ),
            _ => false,
        })
        .filter_map(|event| {
            let told = told(event)?;
            seen.insert(told.clone()).then_some((told, event))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

/// Where someone stands in the order people first joined the place's
/// life, from 0: the same for as long as the World lasts, since whoever
/// joins later is always numbered after everyone before.
pub fn joined_rank(state: &WorldState, person: EntityId) -> Option<usize> {
    let mut joined = state
        .entities()
        .filter(|entity| entity.component(TRAITS_KEY).is_some())
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    joined.sort();
    joined.iter().position(|id| *id == person)
}

/// How someone feels on the whole, for their face: `"cross"` with a
/// grudge against the player, `"sad"` when badly short of something,
/// `"happy"` when nothing much is lacking, else `"content"`.
pub fn mood(state: &WorldState, person: EntityId) -> &'static str {
    let worst = Need::ALL
        .iter()
        .map(|need| lack(state, person, *need))
        .max()
        .unwrap_or(0);
    if regard(state, person) <= GRUDGE {
        "cross"
    } else if worst >= 70 {
        "sad"
    } else if worst <= 35 {
        "happy"
    } else {
        "content"
    }
}

/// How someone feels about the place and the player's hand in it, from
/// -100 to 100.
pub fn regard(state: &WorldState, person: EntityId) -> i64 {
    integer(state, person, REGARD).unwrap_or(0)
}

fn bounded(state: &WorldState, entity: EntityId, key: String, by: i64, min: i64) -> StateChange {
    let next = integer(state, entity, &key)
        .unwrap_or(0)
        .saturating_add(by)
        .clamp(min, 100);
    StateChange::SetComponent {
        entity,
        key,
        value: next.into(),
    }
}

/// Moves how someone feels about the place, within its bounds.
pub fn regard_by(state: &WorldState, person: EntityId, by: i64) -> StateChange {
    bounded(state, person, REGARD.into(), by, -100)
}

/// Moves what `a` thinks of `b`, within its bounds; nobody has an opinion
/// of themselves.
pub fn opinion_by(state: &WorldState, a: EntityId, b: EntityId, by: i64) -> Option<StateChange> {
    (a != b).then(|| bounded(state, a, opinion_key(b), by, -100))
}

/// Moves how short someone is of a need, within its bounds.
pub fn lack_by(state: &WorldState, person: EntityId, need: Need, by: i64) -> StateChange {
    bounded(state, person, need.key().into(), by, 0)
}

/// What someone last did with a day: the activity's id, where, and with
/// whom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lived {
    pub activity: String,
    pub place: EntityId,
    pub with: Option<EntityId>,
}

/// What each person last did with a day, within `span` of world time. A
/// day spent with someone is theirs too, unless they have done something
/// else since. Read from the record only, so it is the same for the same
/// history.
pub fn lately_lived(world: &World, span: u64) -> BTreeMap<EntityId, Lived> {
    let since = world.world_time().saturating_sub(span);
    let recent = world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time >= since)
        .filter(|event| event.kind == "lived")
        .collect::<Vec<_>>();
    let mut lately = BTreeMap::new();
    for event in recent.into_iter().rev() {
        let (Some(person), Some(Value::Text(activity)), Some(Value::Entity(place))) = (
            event.actor,
            event.payload.get("activity"),
            event.payload.get("place"),
        ) else {
            continue;
        };
        let with = event
            .targets
            .first()
            .copied()
            .filter(|with| *with != person);
        lately.insert(
            person,
            Lived {
                activity: activity.clone(),
                place: *place,
                with,
            },
        );
        if let Some(with) = with {
            lately.insert(
                with,
                Lived {
                    activity: activity.clone(),
                    place: *place,
                    with: Some(person),
                },
            );
        }
    }
    lately
}

/// What someone said about their day today, if they have lived it yet.
pub fn said_today(world: &World, person: EntityId) -> Option<String> {
    let now = world.world_time();
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time == now)
        .filter(|event| event.kind == "lived" && event.actor == Some(person))
        .find_map(|event| said(event).map(|(_, line)| line))
}

/// How someone is, in their own words, from how their life stands.
pub fn how_are_you(world: &World, person: EntityId) -> Option<String> {
    how_are_you_on(world, person, 0)
}

/// How someone is, in one of several ways of putting it: the same state
/// of things said differently for a different `seed`.
pub fn how_are_you_on(world: &World, person: EntityId, seed: u64) -> Option<String> {
    let state = world.state();
    if !enrolled(state, person) {
        return None;
    }
    let others = cast_ids(state);
    let foe = others
        .iter()
        .copied()
        .filter(|o| *o != person && opinion(state, person, *o) <= -30)
        .min_by_key(|o| opinion(state, person, *o));
    let friend = best_friend(state, person);
    let worst = Need::ALL
        .iter()
        .copied()
        .max_by_key(|need| lack(state, person, *need))
        .unwrap_or(Need::Company);
    let one = |lines: &[String]| lines[(seed % lines.len() as u64) as usize].clone();
    let line = if let Some(loved) = partner(state, person) {
        let p = first_name(state, loved);
        if opinion(state, person, loved) <= 5 {
            one(&[
                format!("Honestly? Things with {p} aren't good."),
                format!("Not great. {p} and I keep getting it wrong."),
                format!("Could be better. It's {p}, mostly."),
            ])
        } else {
            one(&[
                format!("Happy. {p} and I are good."),
                format!("Really well. {p} makes it easy."),
                format!("Good, thanks. Home with {p} is a nice thing."),
            ])
        }
    } else if let Some(foe) = foe {
        let f = first_name(state, foe);
        one(&[
            format!("Fine, as long as {f} keeps out of my way."),
            format!("Alright. I'd be better if {f} left me be."),
            format!("I'm managing. Avoiding {f}, mostly."),
        ])
    } else if lack(state, person, worst) >= 70 {
        let lines: [&str; 3] = match worst {
            Need::Money => [
                "Worried. Things are tight.",
                "Counting every coin, if I'm honest.",
                "Money's tight. It keeps me up.",
            ],
            Need::Rest => [
                "Exhausted.",
                "Worn out. I could sleep a week.",
                "Tired to the bone.",
            ],
            Need::Company => [
                "I'd like a bit more company, if I'm honest.",
                "Quiet. Too quiet, some days.",
                "I could do with some company.",
            ],
            Need::Purpose => [
                "Restless. I need something to get my teeth into.",
                "Bored stiff. I need something to do.",
                "Restless. The days are long.",
            ],
        };
        one(&lines.map(String::from))
    } else if let Some(friend) = friend {
        let f = first_name(state, friend);
        one(&[
            format!("Good. {f} keeps me going."),
            format!("Not bad at all. {f}'s been good company."),
            format!("Well, thanks. Can't complain with {f} around."),
        ])
    } else {
        one(&[
            "Getting by.".into(),
            "Oh, you know. Getting on with things.".into(),
            "Fine. Same as ever.".into(),
        ])
    };
    Some(line)
}

/// What someone thinks of someone else, in their own words.
pub fn thinks_of(world: &World, person: EntityId, other: EntityId) -> Option<String> {
    let state = world.state();
    if !enrolled(state, person) || !enrolled(state, other) {
        return None;
    }
    let other_name = first_name(state, other);
    let view = opinion(state, person, other);
    Some(if partner(state, person) == Some(other) {
        format!("{other_name}? I'd be lost without them.")
    } else if view >= 50 {
        format!("{other_name}'s one of the best.")
    } else if view >= 20 {
        format!("I like {other_name}.")
    } else if view > -20 {
        format!("{other_name}? We get on.")
    } else if view > -50 {
        format!("{other_name} and I don't see eye to eye.")
    } else {
        format!("Don't talk to me about {other_name}.")
    })
}

#[cfg(test)]
mod tests;
