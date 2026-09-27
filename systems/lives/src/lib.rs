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

mod voice;

use std::collections::{BTreeMap, BTreeSet};
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

/// The people the System looks after: the Pack's, less anyone gone.
fn living(world: &World, cast: &Cast) -> Vec<EntityId> {
    (cast.people)(world)
        .into_iter()
        .filter(|id| world.state().entity(*id).is_some() && !gone(world.state(), *id))
        .collect()
}

fn arg_entity(request: &ActionRequest, name: &str) -> Result<EntityId, ActionError> {
    match request.args.get(name) {
        Some(Value::Entity(id)) => Ok(*id),
        Some(Value::Integer(id)) => Ok(EntityId::new(*id as u64)),
        _ => Err(ActionError::Invalid(format!("missing {name}"))),
    }
}

fn arg_text<'a>(request: &'a ActionRequest, name: &str) -> Result<&'a str, ActionError> {
    match request.args.get(name) {
        Some(Value::Text(value)) => Ok(value),
        _ => Err(ActionError::Invalid(format!("missing {name}"))),
    }
}

/// Changes that move integers, keeping track of what the same event has
/// already moved them to.
#[derive(Default)]
struct Moves {
    values: BTreeMap<(EntityId, String), i64>,
    changes: Vec<StateChange>,
}

impl Moves {
    fn add(
        &mut self,
        state: &WorldState,
        entity: EntityId,
        key: &str,
        by: i64,
        min: i64,
        max: i64,
    ) {
        if state.entity(entity).is_none() {
            return;
        }
        let current = *self
            .values
            .entry((entity, key.to_string()))
            .or_insert_with(|| integer(state, entity, key).unwrap_or(0));
        let next = current.saturating_add(by).clamp(min, max);
        self.values.insert((entity, key.to_string()), next);
        self.changes.push(StateChange::SetComponent {
            entity,
            key: key.to_string(),
            value: next.into(),
        });
    }

    fn lack(&mut self, state: &WorldState, person: EntityId, need: Need, by: i64) {
        self.add(state, person, need.key(), by, 0, 100);
    }

    fn opinion(&mut self, state: &WorldState, a: EntityId, b: EntityId, by: i64) {
        if a != b {
            self.add(state, a, &opinion_key(b), by, -100, 100);
        }
    }

    fn regard(&mut self, state: &WorldState, person: EntityId, by: i64) {
        self.add(state, person, REGARD, by, -100, 100);
    }

    fn set(&mut self, entity: EntityId, key: &str, value: impl Into<Value>) {
        self.changes.push(StateChange::SetComponent {
            entity,
            key: key.to_string(),
            value: value.into(),
        });
    }

    fn unset(&mut self, state: &WorldState, entity: EntityId, key: &str) {
        if state
            .entity(entity)
            .is_some_and(|entity| entity.component(key).is_some())
        {
            self.changes.push(StateChange::RemoveComponent {
                entity,
                key: key.to_string(),
            });
        }
    }
}

/// A fresh start for someone joining the System's life: how short they
/// are of each need, their traits, what they think of everyone.
fn enrolment(state: &WorldState, cast: &Cast, person: EntityId, others: &[EntityId]) -> Moves {
    let mut moves = Moves::default();
    let seed = mix(&[person.0, 7]);
    let traits = (cast.traits)(person)
        .map(|[a, b]| format!("{a} {b}"))
        .unwrap_or_else(|| {
            let a = TRAITS[(seed % TRAITS.len() as u64) as usize];
            let b = TRAITS[((seed / 11 + 3) % TRAITS.len() as u64) as usize];
            if a == b {
                a.to_string()
            } else {
                format!("{a} {b}")
            }
        });
    moves.set(person, TRAITS_KEY, traits);
    for (index, need) in Need::ALL.iter().enumerate() {
        let start = 10 + (mix(&[person.0, index as u64]) % 40) as i64;
        moves.set(person, need.key(), start);
    }
    moves.set(person, REGARD, 10_i64);
    for other in others {
        if *other == person {
            continue;
        }
        for (a, b) in [(person, *other), (*other, person)] {
            if integer(state, a, &opinion_key(b)).is_none() {
                let start = (mix(&[a.0, b.0]) % 36) as i64 - 10;
                moves.set(a, &opinion_key(b), start);
            }
        }
    }
    moves
}

/// The System begins, or someone new joins in.
struct Enrols(fn(&WorldState) -> Cast);

impl Action for Enrols {
    fn name(&self) -> &'static str {
        "lives_enrol"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let person = arg_entity(request, "person")?;
        if state.entity(person).is_none() {
            return Err(ActionError::Invalid("nobody by that id".into()));
        }
        if enrolled(state, person) {
            return Err(ActionError::Invalid("already living here".into()));
        }
        let others = match request.args.get("others") {
            Some(Value::List(others)) => others
                .iter()
                .filter_map(|value| match value {
                    Value::Entity(id) => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let mut draft = EventDraft::new("life_began");
        draft.targets = vec![person];
        if state.entity(cast.notes).is_none() {
            draft.changes.push(StateChange::CreateEntity(
                Entity::new(cast.notes, "lives").with_component("name", "Lives"),
            ));
        }
        draft
            .changes
            .extend(enrolment(state, &cast, person, &others).changes);
        Ok(draft)
    }
}

/// Where someone does an activity.
fn place_for(state: &WorldState, cast: &Cast, person: EntityId, at: At) -> EntityId {
    match at {
        At::Work => (cast.work)(state, person).unwrap_or(cast.gathering),
        At::Home => (cast.home)(state, person).unwrap_or(cast.quiet),
        At::Gathering => cast.gathering,
        At::Quiet => cast.quiet,
    }
}

/// Who someone does an activity with, if anyone.
fn company_for(
    state: &WorldState,
    cast: &Cast,
    person: EntityId,
    with: With,
    others: &[EntityId],
    seed: u64,
) -> Option<EntityId> {
    let others = others
        .iter()
        .copied()
        .filter(|other| *other != person)
        .collect::<Vec<_>>();
    if others.is_empty() {
        return None;
    }
    match with {
        With::Alone => None,
        With::Friend => {
            // Mostly whoever they like best, sometimes their partner, now
            // and then someone else entirely.
            if seed.is_multiple_of(5) {
                return pick(&others, seed / 5).copied();
            }
            if let Some(partner) = partner(state, person).filter(|p| others.contains(p)) {
                if seed.is_multiple_of(3) {
                    return Some(partner);
                }
            }
            others
                .iter()
                .copied()
                .max_by_key(|other| (opinion(state, person, *other), mix(&[seed, other.0])))
        }
        With::Anyone => pick(&others, seed).copied(),
        With::Workmate => {
            let work = (cast.work)(state, person);
            let mates = others
                .iter()
                .copied()
                .filter(|other| work.is_some() && (cast.work)(state, *other) == work)
                .collect::<Vec<_>>();
            pick(if mates.is_empty() { &others } else { &mates }, seed).copied()
        }
    }
}

/// The need pressing hardest on someone, with a little chance in it.
fn pressing(state: &WorldState, person: EntityId, seed: u64) -> Need {
    Need::ALL
        .iter()
        .copied()
        .max_by_key(|need| {
            lack(state, person, *need) * 4 + (mix(&[seed, *need as u64]) % 60) as i64
        })
        .unwrap_or(Need::Company)
}

/// How much each need grows in a period, by someone's nature.
fn drift(traits: &[String], need: Need) -> i64 {
    let mut by = 9;
    for t in traits {
        by += match (t.as_str(), need) {
            ("sociable", Need::Company) | ("restless", Need::Purpose) => 4,
            ("shy", Need::Company) | ("steady", Need::Purpose) => -3,
            ("thrifty", Need::Money) | ("dreamy", Need::Purpose) => 3,
            ("generous", Need::Money) => 2,
            ("proud", Need::Purpose) => 2,
            ("steady", Need::Rest) => -2,
            ("restless", Need::Rest) => 3,
            _ => 0,
        };
    }
    by
}

/// Things someone might add about how their life stands: their partner,
/// someone they are not speaking to, a friend, what they are short of.
fn tails(state: &WorldState, person: EntityId, seed: u64) -> Vec<String> {
    let others = cast_ids(state);
    let mut tails = Vec::new();
    if let Some(p) = partner(state, person) {
        tails.push(format!(" {} sends their love.", name(state, p)));
        tails.push(format!(" Home to {} now.", name(state, p)));
    }
    if let Some(foe) = others
        .iter()
        .copied()
        .filter(|o| *o != person && opinion(state, person, *o) <= -30)
        .min_by_key(|o| opinion(state, person, *o))
    {
        tails.push(format!(" Still not speaking to {}.", name(state, foe)));
        tails.push(format!(" Kept well clear of {}.", name(state, foe)));
    }
    if let Some(friend) = others
        .iter()
        .copied()
        .filter(|o| *o != person && opinion(state, person, *o) >= 40)
        .max_by_key(|o| mix(&[seed, o.0]))
    {
        tails.push(format!(" {} is a gem.", name(state, friend)));
        tails.push(format!(" Seeing {} later.", name(state, friend)));
    }
    if lack(state, person, Need::Rest) >= 70 {
        tails.push(" I could sleep for a week.".into());
    }
    if lack(state, person, Need::Money) >= 70 {
        tails.push(" Things are tight, mind.".into());
    }
    if lack(state, person, Need::Company) >= 70 {
        tails.push(" Quiet without anyone to talk to.".into());
    }
    tails
}

/// A line as someone says it: as it is if nobody has said it lately,
/// otherwise with something of their own added, so the same words do not
/// come round from everyone.
fn personal(state: &WorldState, person: EntityId, base: &str, heard: &Heard, seed: u64) -> String {
    let said_lately = |line: &str| heard.lately(line);
    if !said_lately(base) {
        return base.to_string();
    }
    let options = tails(state, person, seed)
        .into_iter()
        .map(|tail| format!("{base}{tail}"))
        .collect::<Vec<_>>();
    options
        .iter()
        .find(|line| !said_lately(line))
        .cloned()
        .unwrap_or_else(|| base.to_string())
}

/// What someone says about what they did, with a word about how things
/// stand with the people they care about.
fn saying(
    state: &WorldState,
    cast: &Cast,
    person: EntityId,
    activity: &Activity,
    words: &[(&str, &str)],
    seed: u64,
    heard: &Heard,
) -> (String, String) {
    // Something not said lately, if there is anything; otherwise whatever
    // was said longest ago. A line about a friend needs a friend.
    let friend = best_friend(state, person).map(|friend| name(state, friend));
    let mut all_words = words.to_vec();
    if let Some(friend) = &friend {
        all_words.push(("friend", friend.as_str()));
    }
    // Someone with a voice of their own says one of their own lines about
    // half the time, and anything else in their own words.
    let voice = (cast.voice)(person);
    let lines = match voice {
        Some(voice) if (seed / 3).is_multiple_of(2) => own_lines(voice)
            .into_iter()
            .filter(|line| friend.is_some() || !line.contains("{friend}"))
            .map(|line| fill(&line, &all_words))
            .collect::<Vec<_>>(),
        _ => activity
            .said
            .iter()
            .filter(|line| friend.is_some() || !line.contains("{friend}"))
            .map(|line| fill(line, &all_words))
            .collect::<Vec<_>>(),
    };
    let fresh = lines
        .iter()
        .filter(|line| !heard.lately(line))
        .cloned()
        .collect::<Vec<_>>();
    let base = if fresh.is_empty() {
        lines
            .iter()
            .min_by_key(|line| heard.when(line))
            .cloned()
            .unwrap_or_default()
    } else {
        pick(&fresh, seed).cloned().unwrap_or_default()
    };
    let tails = tails(state, person, seed);
    let tail = (seed / 7)
        .is_multiple_of(2)
        .then(|| pick(&tails, seed / 17).cloned())
        .flatten();
    let line = format!("{base}{}", tail.unwrap_or_default());
    if heard.lately(&line) {
        (personal(state, person, &base, heard, seed), base)
    } else {
        (line, base)
    }
}

/// Everyone the System has taken in and who has not left, for words about
/// them; who is living here now is the Pack's to say.
fn cast_ids(state: &WorldState) -> Vec<EntityId> {
    state
        .entities()
        .filter(|entity| entity.component(TRAITS_KEY).is_some())
        .filter(|entity| !gone(state, entity.id))
        .map(|entity| entity.id)
        .collect()
}

/// Someone gets on with their day.
struct Lives(fn(&WorldState) -> Cast);

impl Action for Lives {
    fn name(&self) -> &'static str {
        "lives_day"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let person = arg_entity(request, "person")?;
        if !enrolled(state, person) || gone(state, person) {
            return Err(ActionError::Invalid("not living here".into()));
        }
        let others = match request.args.get("others") {
            Some(Value::List(others)) => others
                .iter()
                .filter_map(|value| match value {
                    Value::Entity(id) if enrolled(state, *id) => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let now = period(state, &cast);
        let mood = (cast.mood)(state).clamp(-10, 10);
        let seed = mix(&[now, person.0, 11, (mood + 10) as u64]);
        // Two days in five, whoever has work goes to it, unless they are
        // dead on their feet; the rest are theirs.
        let working = (cast.work)(state, person).is_some()
            && (now + person.0) % 5 < 2
            && lack(state, person, Need::Rest) < 85;
        let need = if working {
            Need::Money
        } else {
            pressing(state, person, seed)
        };
        let last = text(state, person, "lives.did").unwrap_or("");
        // Rather than say the same again, people do something else: an
        // activity is fresh if it still has something to say that nobody
        // said lately.
        let heard = Heard::of(state, &cast);
        let friend = best_friend(state, person).map(|friend| name(state, friend));
        let fresh = |activity: &&Activity| {
            let place = name(state, place_for(state, &cast, person, activity.at));
            let other = company_for(state, &cast, person, activity.with, &others, seed / 13)
                .map(|other| name(state, other))
                .unwrap_or_default();
            activity.said.iter().any(|line| {
                if line.contains("{friend}") && friend.is_none() {
                    return false;
                }
                let filled = fill(
                    line,
                    &[
                        ("place", place.as_str()),
                        ("other", other.as_str()),
                        ("friend", friend.as_deref().unwrap_or("")),
                    ],
                );
                !heard.lately(&filled)
            })
        };
        let all = cast
            .activities
            .iter()
            .filter(|activity| activity.need == need && activity.id != last)
            .collect::<Vec<_>>();
        let fresh_ones = all.iter().copied().filter(fresh).collect::<Vec<_>>();
        let choices = if fresh_ones.is_empty() {
            all
        } else {
            fresh_ones
        };
        let Some(activity) = pick(&choices, seed / 3).copied() else {
            return Err(ActionError::Invalid("nothing to do".into()));
        };
        let with = company_for(state, &cast, person, activity.with, &others, seed / 13);
        if activity.with != With::Alone && with.is_none() {
            // Nobody to do it with: a quiet day instead.
            return Err(ActionError::Invalid("nobody about".into()));
        }
        let place = place_for(state, &cast, person, activity.at);
        let (me, place_name) = (name(state, person), name(state, place));
        let other_name = with.map(|w| name(state, w)).unwrap_or_default();
        let words = [
            ("name", me.as_str()),
            ("other", other_name.as_str()),
            ("place", place_name.as_str()),
        ];

        let mut moves = Moves::default();
        let traits = traits(state, person);
        for n in Need::ALL {
            let by = if n == need {
                -(30 + (seed % 20) as i64)
            } else {
                drift(&traits, n)
            };
            moves.lack(state, person, n, by);
        }
        let mut quarrel = false;
        if let Some(other) = with {
            let fit = compatibility(state, person, other);
            let chance = mix(&[seed, other.0, 3]) % 100;
            // People who grate on each other sometimes have words, and
            // anyone at the end of their tether snaps more easily.
            let frayed = Need::ALL
                .iter()
                .any(|need| lack(state, person, *need) >= 70 || lack(state, other, *need) >= 80);
            // A sour mood in the place shortens tempers; a bright one
            // lengthens them.
            let temper = (18 - fit * 4).max(5) + if frayed { 12 } else { 0 } - mood;
            quarrel = chance < temper.max(2) as u64;
            let by = if quarrel {
                -(10 + (chance % 12) as i64)
            } else {
                3 + fit * 2 + (chance % 6) as i64
            };
            moves.opinion(state, person, other, by);
            moves.opinion(state, other, person, by - if quarrel { 2 } else { 1 });
            if need == Need::Company {
                moves.lack(state, other, Need::Company, -15);
            }
        }
        // Every so often, what someone feels about the people they have not
        // spent time with fades a little toward nothing.
        if now % 5 == person.0 % 5 {
            for other in &others {
                if Some(*other) == with || *other == person {
                    continue;
                }
                let view = opinion(state, person, *other);
                if view.abs() >= 3 {
                    moves.opinion(state, person, *other, -3 * view.signum());
                }
            }
        }
        for (key, by) in activity.gives {
            moves.add(state, person, key, *by, i64::MIN / 4, i64::MAX / 4);
        }
        // An ordinary day, with no words had, mends a little of how someone
        // feels about the place.
        if !quarrel && integer(state, person, REGARD).unwrap_or(0) < 10 {
            moves.regard(state, person, 1);
        }
        moves.set(person, "lives.did", activity.id);
        moves.set(person, AT, Value::Entity(place));

        let mut told = fill(activity.told, &words);
        let (mut said, base) = saying(state, &cast, person, activity, &words, seed, &heard);
        if quarrel {
            told.push_str(", and they had words");
            let topic = pick(cast.topics, seed / 19).copied().unwrap_or("nothing");
            // Said in words nobody has used lately, like any other line.
            let mut options = [
                format!("{other_name} and I had words over {topic}."),
                format!("Don't mention {topic} to {other_name}. Just don't."),
                format!("{other_name} and I fell out over {topic}. It'll blow over."),
                format!("Me and {other_name}, shouting about {topic}. Silly."),
            ];
            options.rotate_left((seed / 23 % 4) as usize);
            said = options
                .iter()
                .find(|line| !heard.lately(line))
                .cloned()
                .unwrap_or_else(|| options[0].clone());
        }
        if let Some(voice) = (cast.voice)(person) {
            said = restyle(voice, &said, seed / 29);
        }
        remember_saying(&mut moves, &heard, &said);
        remember_saying(&mut moves, &heard, &base);
        let mut draft = EventDraft::new("lived");
        draft.actor = Some(person);
        draft.targets = with.into_iter().collect();
        draft.payload.insert("activity".into(), activity.id.into());
        draft.payload.insert("place".into(), Value::Entity(place));
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), said.into());
        draft.changes = moves.changes;
        Ok(draft)
    }
}

/// How many periods a line is remembered for: nobody says the same thing
/// again, and nobody else says it either, until it has been forgotten.
const HEARD_PERIODS: u64 = 45;

/// When lines were last said, kept one small note per line.
#[derive(Clone, Copy)]
struct Heard<'a> {
    state: &'a WorldState,
    notes: EntityId,
    now: u64,
}

impl<'a> Heard<'a> {
    fn of(state: &'a WorldState, cast: &Cast) -> Self {
        Heard {
            state,
            notes: cast.notes,
            now: period(state, cast),
        }
    }

    fn when(&self, line: &str) -> Option<u64> {
        integer(self.state, self.notes, &heard_key(line)).map(|at| at.max(0) as u64)
    }

    fn lately(&self, line: &str) -> bool {
        self.when(line)
            .is_some_and(|at| self.now.saturating_sub(at) < HEARD_PERIODS)
    }
}

fn heard_key(line: &str) -> String {
    let hash = line.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("lives.heard.{hash:016x}")
}

fn remember_saying(moves: &mut Moves, heard: &Heard, said: &str) {
    moves.set(heard.notes, &heard_key(said), heard.now as i64);
}

/// One of several ways to say something, whichever was said longest ago.
fn pick_line(options: &[String], heard: &Heard, person: EntityId) -> String {
    options
        .iter()
        .min_by_key(|line| (heard.when(line), mix(&[person.0, line.len() as u64])))
        .cloned()
        .unwrap_or_default()
}

/// How two people stand, from what each thinks of the other.
fn standing(state: &WorldState, a: EntityId, b: EntityId) -> &'static str {
    let (ab, ba) = (opinion(state, a, b), opinion(state, b, a));
    if partner(state, a) == Some(b) {
        "partners"
    } else if ab <= -20 || ba <= -20 {
        "foes"
    } else if ab >= 30 && ba >= 30 {
        "friends"
    } else {
        ""
    }
}

fn bond_key(other: EntityId) -> String {
    format!("{BOND}.{other}")
}

/// Two people's standing changes: they become friends, fall out, make up.
struct Bonds(fn(&WorldState) -> Cast);

impl Action for Bonds {
    fn name(&self) -> &'static str {
        "lives_bond"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let heard = Heard::of(state, &cast);
        let a = arg_entity(request, "a")?;
        let b = arg_entity(request, "b")?;
        let was = text(state, a, &bond_key(b)).unwrap_or("");
        let now = standing(state, a, b);
        if was == now {
            return Err(ActionError::Invalid("nothing has changed".into()));
        }
        let (an, bn) = (name(state, a), name(state, b));
        let (kind, told, said) = match (was, now) {
            (_, "friends") if was == "foes" => (
                "made_up",
                format!("{an} and {bn} made it up, and more"),
                pick_line(
                    &[
                        format!("{bn} and I are thick as thieves now. Who'd have thought?"),
                        format!("Funny how things turn out. {bn}'s my best friend now."),
                        format!("Me and {bn}? Old news. We're friends."),
                    ],
                    &heard,
                    a,
                ),
            ),
            (_, "friends") => (
                "became_friends",
                format!("{an} and {bn} became firm friends"),
                pick_line(
                    &[
                        format!("{bn}'s a proper friend."),
                        format!("Me and {bn}? Friends for life."),
                        format!("{bn} gets me. Rare, that."),
                        format!("Glad I've got {bn}."),
                    ],
                    &heard,
                    a,
                ),
            ),
            (_, "foes") => (
                "fell_out",
                format!("{an} and {bn} fell out"),
                pick_line(
                    &[
                        format!("I'm done with {bn}."),
                        format!("Don't talk to me about {bn}."),
                        format!("{bn} knows what they did."),
                        format!("{bn} and I are finished."),
                    ],
                    &heard,
                    a,
                ),
            ),
            ("foes", "") => (
                "made_up",
                format!("{an} and {bn} made it up"),
                pick_line(
                    &[
                        format!("{bn} and I are all right again."),
                        format!("Made it up with {bn}. Life's too short."),
                        format!("{bn} said sorry. So did I."),
                    ],
                    &heard,
                    a,
                ),
            ),
            ("friends", "") => (
                "drifted",
                format!("{an} and {bn} drifted apart"),
                pick_line(
                    &[
                        format!("I hardly see {bn} these days."),
                        format!("{bn} and I used to be close."),
                        format!("Must look {bn} up sometime."),
                    ],
                    &heard,
                    a,
                ),
            ),
            _ => return Err(ActionError::Invalid("nothing worth telling".into())),
        };
        let said = personal(
            state,
            a,
            &said,
            &heard,
            mix(&[a.0, b.0, period(state, &cast)]),
        );
        let mut moves = Moves::default();
        remember_saying(&mut moves, &heard, &said);
        let mut draft = EventDraft::new("bond_changed");
        draft.actor = Some(a);
        draft.targets = vec![b];
        draft.payload.insert("bond".into(), kind.into());
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), said.into());
        draft.changes.extend(moves.changes);
        for (x, y) in [(a, b), (b, a)] {
            draft.changes.push(StateChange::SetComponent {
                entity: x,
                key: bond_key(y),
                value: now.into(),
            });
        }
        Ok(draft)
    }
}

/// The kinds of situation this System composes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Feud,
    Sweet,
    Learn,
    Short,
    Lonely,
    Worn,
    Party,
    Visitor,
    Leaving,
    RoughPatch,
    /// Someone warming to the player shares a small moment with them.
    Warming,
    /// A friend tells the player something they have told nobody.
    Confide,
    /// A friend asks the player along somewhere of theirs.
    Invite,
    /// A close friend offers the player a favour.
    Favour,
    /// A close friend gives the player something to keep.
    Keepsake,
    /// Someone with a grudge against the player lets it show.
    Cold,
}

impl Kind {
    const ALL: [Kind; 16] = [
        Kind::Feud,
        Kind::Sweet,
        Kind::Learn,
        Kind::Short,
        Kind::Lonely,
        Kind::Worn,
        Kind::Party,
        Kind::Visitor,
        Kind::Leaving,
        Kind::RoughPatch,
        Kind::Warming,
        Kind::Confide,
        Kind::Invite,
        Kind::Favour,
        Kind::Keepsake,
        Kind::Cold,
    ];

    fn id(self) -> &'static str {
        match self {
            Kind::Feud => "feud",
            Kind::Sweet => "sweet",
            Kind::Learn => "learn",
            Kind::Short => "short",
            Kind::Lonely => "lonely",
            Kind::Worn => "worn",
            Kind::Party => "party",
            Kind::Visitor => "visitor",
            Kind::Leaving => "leaving",
            Kind::RoughPatch => "rough",
            Kind::Warming => "warming",
            Kind::Confide => "confide",
            Kind::Invite => "invite",
            Kind::Favour => "favour",
            Kind::Keepsake => "keepsake",
            Kind::Cold => "cold",
        }
    }

    /// Someone asking the player for help, which nobody with a grudge
    /// against them does.
    fn asks_for_help(self) -> bool {
        matches!(
            self,
            Kind::Sweet | Kind::Learn | Kind::Short | Kind::Lonely | Kind::Worn
        )
    }

    fn from_id(id: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.id() == id)
    }

    /// How many periods the same people's same trouble rests before it can
    /// come up again.
    fn rests(self) -> u64 {
        match self {
            Kind::Visitor => 0,
            Kind::Party => 60,
            Kind::Warming | Kind::Confide | Kind::Invite | Kind::Favour | Kind::Keepsake => 12,
            _ => 24,
        }
    }
}

/// A situation that could be put to the player now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub kind: Kind,
    /// Whose it is: they ask it.
    pub a: EntityId,
    /// Who else it is about, if anyone.
    pub b: Option<EntityId>,
    /// What it is over, as an index into the Pack's words.
    pub topic: u64,
}

impl Candidate {
    /// What the situation is known by: the same people's same trouble over
    /// the same thing is the same situation.
    pub fn key(&self) -> String {
        match self.b {
            Some(b) => format!("{}.{}.{}.{}", self.kind.id(), self.a, b, self.topic),
            None => format!("{}.{}.{}", self.kind.id(), self.a, self.topic),
        }
    }

    /// The same people's same kind of trouble, over whatever it is.
    fn family(&self) -> String {
        match self.b {
            Some(b) => format!(
                "{}.{}.{}",
                self.kind.id(),
                self.a.0.min(b.0),
                self.a.0.max(b.0)
            ),
            None => format!("{}.{}", self.kind.id(), self.a),
        }
    }

    fn parse(key: &str) -> Option<Candidate> {
        let parts = key.split('.').collect::<Vec<_>>();
        let kind = Kind::from_id(parts.first()?)?;
        let a = EntityId::new(parts.get(1)?.parse().ok()?);
        let (b, topic) = match parts.len() {
            4 => (
                Some(EntityId::new(parts[2].parse().ok()?)),
                parts[3].parse().ok()?,
            ),
            3 => (None, parts[2].parse().ok()?),
            _ => return None,
        };
        Some(Candidate { kind, a, b, topic })
    }
}

fn open_key(key: &str) -> String {
    format!("lives.open.{key}")
}

fn rest_key(family: &str) -> String {
    format!("lives.rest.{family}")
}

/// The situations open now: each one's key and the period it opened.
pub fn open(state: &WorldState, cast: &Cast) -> Vec<(String, i64)> {
    let Some(notes) = state.entity(cast.notes) else {
        return Vec::new();
    };
    notes
        .components
        .iter()
        .filter_map(|(key, value)| {
            let key = key.strip_prefix("lives.open.")?;
            match value {
                Value::Map(map) => match map.get("opened") {
                    Some(Value::Integer(at)) => Some((key.to_string(), *at)),
                    _ => None,
                },
                _ => None,
            }
        })
        .collect()
}

fn open_map<'a>(
    state: &'a WorldState,
    cast: &Cast,
    key: &str,
) -> Option<&'a BTreeMap<String, Value>> {
    match state.entity(cast.notes)?.component(&open_key(key))? {
        Value::Map(map) => Some(map),
        _ => None,
    }
}

/// How the place stands, as one number: everyone's goodwill toward it.
fn standing_of(state: &WorldState, people: &[EntityId]) -> u64 {
    people
        .iter()
        .map(|person| integer(state, *person, REGARD).unwrap_or(0))
        .sum::<i64>()
        .unsigned_abs()
}

fn next_visitor(state: &WorldState, cast: &Cast) -> Option<EntityId> {
    let visitors = cast.visitors?;
    (visitors.first..visitors.first + visitors.room)
        .map(EntityId::new)
        .find(|id| state.entity(*id).is_none())
}

/// The situations the World could put to the player now, most pressing
/// first.
pub fn candidates(world: &World, cast: &Cast) -> Vec<Candidate> {
    let state = world.state();
    let people = living(world, cast)
        .into_iter()
        .filter(|id| enrolled(state, *id))
        .collect::<Vec<_>>();
    let now = period(state, cast);
    let mut found = Vec::new();
    let topic = |a: EntityId, b: u64, salt: u64| mix(&[a.0, b, salt, now / 30]) % 1000;
    for &a in &people {
        for &b in &people {
            if a == b || (cast.kept)(a, b) || (cast.kept)(b, a) {
                continue;
            }
            let (ab, ba) = (opinion(state, a, b), opinion(state, b, a));
            if ab <= -30 && ab <= ba && partner(state, a) != Some(b) {
                found.push((
                    ab.abs() + 20,
                    Candidate {
                        kind: Kind::Feud,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 1),
                    },
                ));
            }
            if ab >= 45 && partner(state, a).is_none() && partner(state, b).is_none() {
                found.push((
                    ab,
                    Candidate {
                        kind: Kind::Sweet,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 2),
                    },
                ));
            }
            if partner(state, a) == Some(b) && a.0 < b.0 && (ab <= 5 || ba <= 5) {
                found.push((
                    70,
                    Candidate {
                        kind: Kind::RoughPatch,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 3),
                    },
                ));
            }
            if partner(state, a) == Some(b) && a.0 < b.0 && integer(state, a, CELEBRATED).is_none()
            {
                found.push((
                    60,
                    Candidate {
                        kind: Kind::Party,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 4),
                    },
                ));
            }
            if lack(state, a, Need::Purpose) >= 55
                && ab >= 0
                && (cast.work)(state, a) != (cast.work)(state, b)
                && (cast.work)(state, b).is_some()
            {
                found.push((
                    lack(state, a, Need::Purpose) - 10,
                    Candidate {
                        kind: Kind::Learn,
                        a,
                        b: Some(b),
                        topic: topic(a, b.0, 5),
                    },
                ));
            }
        }
        let lacks = |need| lack(state, a, need);
        if lacks(Need::Money) >= 70 {
            found.push((
                lacks(Need::Money),
                Candidate {
                    kind: Kind::Short,
                    a,
                    b: None,
                    topic: topic(a, 0, 6),
                },
            ));
        }
        if lacks(Need::Company) >= 70 {
            found.push((
                lacks(Need::Company),
                Candidate {
                    kind: Kind::Lonely,
                    a,
                    b: None,
                    topic: topic(a, 0, 7),
                },
            ));
        }
        if lacks(Need::Rest) >= 80 {
            found.push((
                lacks(Need::Rest),
                Candidate {
                    kind: Kind::Worn,
                    a,
                    b: None,
                    topic: topic(a, 0, 8),
                },
            ));
        }
        // Doors a friendship opens, each once, in turn: a secret, then a
        // favour, then a keepsake some periods after. Anyone's real trouble
        // comes first.
        let regard = integer(state, a, REGARD).unwrap_or(0);
        let door = |kind| door_opened(state, a, kind);
        let door_candidate = |kind| Candidate {
            kind,
            a,
            b: None,
            topic: mix(&[a.0, 11]) % 1000,
        };
        // A door that has waited grows more pressing, so a friendship's
        // moments come round in weeks, not whenever nobody is in trouble.
        let waited = |before: Kind| door(before).map_or(0, |at| (now as i64 - at).clamp(0, 15));
        if regard >= WARM && door(Kind::Warming).is_some() && door(Kind::Confide).is_none() {
            found.push((40 + waited(Kind::Warming), door_candidate(Kind::Confide)));
        }
        if regard >= DEAR && door(Kind::Confide).is_some() && door(Kind::Invite).is_none() {
            found.push((42 + waited(Kind::Confide), door_candidate(Kind::Invite)));
        }
        if regard >= CLOSE && door(Kind::Invite).is_some() && door(Kind::Favour).is_none() {
            found.push((45 + waited(Kind::Invite), door_candidate(Kind::Favour)));
        }
        if regard >= CLOSE
            && door(Kind::Keepsake).is_none()
            && door(Kind::Favour).is_some_and(|at| now as i64 - at >= 3)
        {
            found.push((47 + waited(Kind::Favour), door_candidate(Kind::Keepsake)));
        }
        if regard <= GRUDGE {
            found.push((
                55,
                Candidate {
                    kind: Kind::Cold,
                    a,
                    b: None,
                    topic: topic(a, 0, 12),
                },
            ));
        }
        let worst = Need::ALL.iter().map(|n| lacks(*n)).max().unwrap_or(0);
        if !(cast.stays)(a) && integer(state, a, REGARD).unwrap_or(0) <= -20 && worst >= 70 {
            found.push((
                90,
                Candidate {
                    kind: Kind::Leaving,
                    a,
                    b: None,
                    topic: topic(a, 0, 9),
                },
            ));
        }
    }
    // Strangers come by now and then, and more often to a place that is
    // nearly empty.
    let few = people.len() * 2 < cast.most_people;
    if people.len() < cast.most_people && now % if few { 5 } else { 7 } == 4 {
        if let Some(visitor) = next_visitor(state, cast) {
            found.push((
                if few { 120 } else { 105 },
                Candidate {
                    kind: Kind::Visitor,
                    a: cast.host,
                    b: Some(visitor),
                    // Who turns up depends on how the place is doing:
                    // two branches of a World meet different strangers.
                    topic: mix(&[visitor.0, standing_of(state, &people), now]) % 1000,
                },
            ));
        }
    }
    // Most pressing first, though a kind of trouble heard lately waits its
    // turn; among equals, whichever the day's chance favours.
    for (score, candidate) in &mut found {
        let last = integer(
            state,
            cast.notes,
            &format!("lives.kind.{}", candidate.kind.id()),
        );
        if last.is_some_and(|last| now as i64 - last < 6) {
            *score -= 45;
        }
    }
    found.sort_by_key(|(score, candidate)| {
        (-score, mix(&[now, candidate.a.0, candidate.kind as u64]))
    });
    found
        .into_iter()
        .map(|(_, candidate)| candidate)
        // Nobody with a grudge against the player asks them for help.
        .filter(|candidate| {
            !(candidate.kind.asks_for_help()
                && integer(state, candidate.a, REGARD).unwrap_or(0) <= GRUDGE)
        })
        .filter(|candidate| {
            let rested = integer(state, cast.notes, &rest_key(&candidate.family()))
                .is_none_or(|last| now as i64 - last >= candidate.kind.rests() as i64);
            rested && open_map(state, cast, &candidate.key()).is_none()
        })
        .collect()
}

/// One way to answer a situation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub id: &'static str,
    pub title: String,
    /// Why it cannot be given now, if it cannot.
    pub unavailable: Option<String>,
}

/// A situation as it is put to the player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Situation {
    pub key: String,
    pub kind: Kind,
    pub asker: EntityId,
    pub about: Option<EntityId>,
    pub prompt: String,
    /// How it is told when it comes up.
    pub told: String,
    pub answers: Vec<Answer>,
}

fn words_for(
    state: &WorldState,
    cast: &Cast,
    candidate: &Candidate,
) -> Vec<(&'static str, String)> {
    let mut words = vec![
        ("a", name(state, candidate.a)),
        ("unit", cast.unit.to_string()),
        ("settlement", cast.settlement.to_string()),
        ("short_of", cast.short_of.to_string()),
        ("gathering", name(state, cast.gathering)),
        (
            "topic",
            pick(cast.topics, candidate.topic)
                .copied()
                .unwrap_or("nothing much")
                .into(),
        ),
        (
            "outing",
            pick(cast.outings, candidate.topic)
                .copied()
                .unwrap_or("take a walk")
                .into(),
        ),
    ];
    if let Some(b) = candidate.b {
        if candidate.kind == Kind::Visitor {
            if let Some(visitors) = cast.visitors {
                let seed = mix(&[b.0, candidate.topic, 17]);
                words.push((
                    "b",
                    pick(visitors.names, seed)
                        .copied()
                        .unwrap_or("A stranger")
                        .into(),
                ));
                let (trade, _) = pick(visitors.trades, seed / 7)
                    .copied()
                    .unwrap_or(("traveller", "traveller"));
                words.push(("trade", trade.into()));
                words.push((
                    "origin",
                    pick(visitors.origins, seed / 13)
                        .copied()
                        .unwrap_or("far away")
                        .into(),
                ));
                words.push(("way_out", visitors.way_out.into()));
            }
        } else {
            words.push(("b", name(state, b)));
            if let Some(work) = (cast.work)(state, b) {
                words.push(("work", name(state, work)));
            }
        }
    }
    if let Some(friend) = best_friend(state, candidate.a) {
        words.push(("friend", name(state, friend)));
    }
    if candidate.kind == Kind::Keepsake {
        let what = fill_owned(
            keepsake_of((cast.voice)(candidate.a), (cast.traits)(candidate.a)),
            &words,
        );
        words.push(("keepsake", what));
    }
    words
}

/// Which of the five doors a kind is, if it is one.
fn door_index(kind: Kind) -> Option<usize> {
    [
        Kind::Warming,
        Kind::Confide,
        Kind::Invite,
        Kind::Favour,
        Kind::Keepsake,
    ]
    .iter()
    .position(|door| *door == kind)
}

/// The scene someone opens at a door: their own words.
fn door_scene(cast: &Cast, candidate: &Candidate) -> Option<Scene> {
    let index = door_index(candidate.kind)?;
    Some(scene_of(
        (cast.voice)(candidate.a),
        (cast.traits)(candidate.a),
        index,
    ))
}

/// What someone leaves for the player while they are away.
const LEFT_FOR_YOU: [&str; 6] = [
    "a note under the door",
    "a sketch of the {gathering}",
    "a jar of something homemade",
    "a photograph from while you were gone",
    "a pressed flower in an envelope",
    "a postcard with a few lines on it",
];

/// What someone says when a moment they offered passes by.
const MISSED: [&str; 8] = [
    "Oh well. Another time.",
    "Never mind. It'll come round.",
    "I looked for you, that's all.",
    "Maybe next time.",
    "It was only a small thing.",
    "Went on my own. It was fine.",
    "You were busy. I understand.",
    "Some other day, then.",
];

/// What someone gives a newcomer after their first deed.
const WELCOME: [&str; 4] = [
    "a hand-drawn map of the {settlement}",
    "a smooth stone from by the {gathering}",
    "a little welcome card, signed by everyone",
    "a paper star to hang up",
];

/// How a newcomer is greeted: who greets them, and a nudge to make
/// something.
const GREETINGS: [&str; 3] = [
    "Hello, you're new! I'm {name}. Build us something, if you like.",
    "Oh, a new face! I'm {name}. Welcome to the {settlement}.",
    "Welcome to the {settlement}! I'm {name}. Make yourself at home.",
];

fn best_friend(state: &WorldState, person: EntityId) -> Option<EntityId> {
    cast_ids(state)
        .into_iter()
        .filter(|o| {
            *o != person && opinion(state, person, *o) >= 20 && opinion(state, *o, person) >= 0
        })
        .max_by_key(|o| opinion(state, person, *o))
}

struct Script {
    prompts: &'static [&'static str],
    told: &'static str,
    answers: &'static [(&'static str, &'static str)],
}

fn script(kind: Kind) -> Script {
    match kind {
        Kind::Feud => Script {
            prompts: &[
                "{b} and I aren't speaking. It started over {topic}.",
                "I can't be in a room with {b}. Not after {topic}.",
                "About {topic}: {b} has it all wrong.",
            ],
            told: "{a} and {b} fell out over {topic}",
            answers: &[
                ("mend", "Make peace"),
                ("side", "Back {a}"),
                ("leave", "Stay out"),
            ],
        },
        Kind::Sweet => Script {
            prompts: &[
                "Don't laugh. I think I'm sweet on {b}.",
                "Would it be mad to ask {b} to {outing}?",
                "I keep finding reasons to be wherever {b} is.",
            ],
            told: "{a} confided they were sweet on {b}",
            answers: &[("ask", "Go for it"), ("wait", "Wait a bit")],
        },
        Kind::Learn => Script {
            prompts: &[
                "I'd love to learn what {b} does. Would you ask them?",
                "{b} makes it look easy. Could I learn from them?",
                "I feel stuck. {b} could teach me a thing or two.",
            ],
            told: "{a} wanted to learn from {b}",
            answers: &[("teach", "Arrange lessons"), ("not_now", "Not now")],
        },
        Kind::Short => Script {
            prompts: &[
                "I'm short of {short_of} this {unit}. I hate asking.",
                "Could I be tided over? Just this once.",
                "My {short_of} ran out before the {unit} did.",
            ],
            told: "{a} was short of {short_of}",
            answers: &[
                ("fund", "Use the fund"),
                ("friend", "Ask {friend}"),
                ("manage", "They'll manage"),
            ],
        },
        Kind::Lonely => Script {
            prompts: &[
                "Haven't really talked to anyone in days.",
                "Evenings are long on my own.",
                "Would anyone notice if I wasn't here?",
            ],
            told: "{a} was lonely",
            answers: &[("invite", "Take them out"), ("space", "Give them space")],
        },
        Kind::Worn => Script {
            prompts: &[
                "I'm running on nothing.",
                "I can't remember my last day off.",
                "If I sit down I'll fall asleep.",
            ],
            told: "{a} was worn out",
            answers: &[("rest", "A day off"), ("push", "Push on")],
        },
        Kind::Party => Script {
            prompts: &[
                "{b} and I want to celebrate. Everyone's invited?",
                "We'd like a do. {b} insists on dancing.",
            ],
            told: "{a} and {b} wanted to celebrate",
            answers: &[("party", "Throw a party"), ("quiet", "Keep it small")],
        },
        Kind::Visitor => Script {
            prompts: &[
                "{b}, a {trade} from {origin}, wants a room.",
                "A {trade} called {b} came in on {way_out}.",
            ],
            told: "{b}, a {trade} from {origin}, asked to stay",
            answers: &[("decline", "Not this time"), ("welcome", "Offer a room")],
        },
        Kind::Leaving => Script {
            prompts: &[
                "I've been thinking of leaving.",
                "There's nothing keeping me here, is there?",
                "I've been looking at the timetable for {way_out}.",
            ],
            told: "{a} was thinking of leaving",
            answers: &[("stay", "Please stay"), ("go", "Wish them well")],
        },
        Kind::RoughPatch => Script {
            prompts: &[
                "{b} and I keep quarrelling.",
                "Things aren't right with {b}.",
            ],
            told: "{a} and {b} hit a rough patch",
            answers: &[("talk", "Help them talk"), ("apart", "Time apart")],
        },
        Kind::Warming => Script {
            prompts: &["It's good to see you."],
            told: "{a} shared a quiet moment with you",
            answers: &[("stay", "Stay a while"), ("wave", "Wave and go on")],
        },
        Kind::Invite => Script {
            prompts: &["Come with me somewhere?"],
            told: "{a} asked you along",
            answers: &[("go", "Go along"), ("another", "Another time")],
        },
        Kind::Confide => Script {
            prompts: &[
                "Can I tell you something? I nearly didn't come to the {settlement} at all.",
                "I've never said this to anyone here. I write poems at night. Bad ones.",
                "Promise you won't laugh. I'm scared I'm no good at what I do.",
                "I still keep a letter from someone I left behind. I read it most {unit}s.",
            ],
            told: "{a} told you something they'd never told anyone",
            answers: &[
                ("keep", "Keep it between you"),
                ("share", "Tell them to share it"),
            ],
        },
        Kind::Favour => Script {
            prompts: &[
                "You've been good to me. Let me do something for you, for once.",
                "I owe you. Name it.",
                "Anything you need, you only have to ask. I mean it.",
            ],
            told: "{a} offered you a favour",
            answers: &[
                ("word", "Put in a word with {friend}"),
                ("help", "Help someone who's struggling"),
                ("nothing", "Nothing, really"),
            ],
        },
        Kind::Keepsake => Script {
            prompts: &[
                "I want you to have this: {keepsake}. You'll know why.",
                "Here. {keepsake}. Don't argue, just keep it.",
            ],
            told: "{a} gave you {keepsake}",
            answers: &[
                ("keep", "Keep it safe"),
                ("show", "Put it where all can see"),
            ],
        },
        Kind::Cold => Script {
            prompts: &[
                "Oh. It's you.",
                "I've nothing to say to you.",
                "Don't think I've forgotten.",
            ],
            told: "{a} gave you the cold shoulder",
            answers: &[("sorry", "Say you're sorry"), ("leave", "Leave them be")],
        },
    }
}

fn fill_owned(template: &str, words: &[(&'static str, String)]) -> String {
    let borrowed = words
        .iter()
        .map(|(slot, word)| (*slot, word.as_str()))
        .collect::<Vec<_>>();
    fill(template, &borrowed)
}

/// A situation as it would be put, from its candidate.
fn compose(state: &WorldState, cast: &Cast, candidate: &Candidate) -> Situation {
    let words = words_for(state, cast, candidate);
    let script = script(candidate.kind);
    let prompt = match door_scene(cast, candidate) {
        Some(scene) => fill_owned(scene.prompt, &words),
        None => pick(script.prompts, candidate.topic / 3)
            .map(|prompt| fill_owned(prompt, &words))
            .unwrap_or_default(),
    };
    let answers = script
        .answers
        .iter()
        .filter(|(id, _)| match (candidate.kind, *id) {
            (Kind::Short, "fund") => cast.fund.is_some(),
            (Kind::Short, "friend") | (Kind::Favour, "word") => {
                best_friend(state, candidate.a).is_some()
            }
            _ => true,
        })
        .map(|(id, title)| Answer {
            id,
            title: fill_owned(title, &words),
            unavailable: match (candidate.kind, *id) {
                (Kind::Short, "fund") | (Kind::Party, "party") => cast.fund.and_then(|fund| {
                    (integer(state, fund.entity, fund.key).unwrap_or(0) < fund.amount)
                        .then(|| format!("There isn't {} in {}", fund.amount, fund.name))
                }),
                _ => None,
            },
        })
        .collect();
    Situation {
        key: candidate.key(),
        kind: candidate.kind,
        asker: candidate.a,
        about: candidate.b,
        prompt,
        told: fill_owned(script.told, &words),
        answers,
    }
}

/// The situations open now, as they are put to the player.
pub fn situations(world: &World, cast: &Cast) -> Vec<Situation> {
    let state = world.state();
    let mut open = open(state, cast);
    open.sort_by_key(|(key, at)| (*at, key.clone()));
    open.into_iter()
        .filter_map(|(key, _)| {
            let candidate = Candidate::parse(&key)?;
            let mut situation = compose(state, cast, &candidate);
            // Told as it was first put, whatever has changed since.
            if let Some(Value::Text(prompt)) = open_map(state, cast, &key)?.get("prompt") {
                situation.prompt = prompt.clone();
            }
            Some(situation)
        })
        .collect()
}

/// A situation comes up.
struct Opens(fn(&WorldState) -> Cast);

impl Action for Opens {
    fn name(&self) -> &'static str {
        "lives_situation_opens"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let key = arg_text(request, "situation")?;
        let candidate = Candidate::parse(key)
            .ok_or_else(|| ActionError::Invalid("no such situation".into()))?;
        if open_map(state, &cast, key).is_some() {
            return Err(ActionError::Invalid("already open".into()));
        }
        let mut situation = compose(state, &cast, &candidate);
        // Put in words nobody has used lately; at a door, in their own.
        let heard = Heard::of(state, &cast);
        let words = words_for(state, &cast, &candidate);
        let prompts = match door_scene(&cast, &candidate) {
            Some(_) => vec![situation.prompt.clone()],
            None => script(candidate.kind)
                .prompts
                .iter()
                .map(|prompt| fill_owned(prompt, &words))
                .collect::<Vec<_>>(),
        };
        situation.prompt = match prompts.iter().find(|prompt| !heard.lately(prompt)) {
            Some(prompt) => prompt.clone(),
            None => personal(
                state,
                candidate.a,
                &situation.prompt,
                &heard,
                mix(&[candidate.a.0, 9]),
            ),
        };
        let mut moves = Moves::default();
        remember_saying(&mut moves, &heard, &situation.prompt);
        let mut draft = EventDraft::new("situation_came_up");
        draft.actor = Some(candidate.a);
        draft.targets = candidate
            .b
            .filter(|b| state.entity(*b).is_some())
            .into_iter()
            .collect();
        draft.payload.insert("situation".into(), key.into());
        draft
            .payload
            .insert("kind".into(), candidate.kind.id().into());
        draft
            .payload
            .insert("told".into(), situation.told.clone().into());
        draft
            .payload
            .insert("said".into(), situation.prompt.clone().into());
        let mut map = BTreeMap::new();
        map.insert(
            "opened".to_string(),
            Value::from(period(state, &cast) as i64),
        );
        map.insert("prompt".to_string(), Value::from(situation.prompt));
        draft.changes.push(StateChange::SetComponent {
            entity: cast.notes,
            key: open_key(key),
            value: Value::Map(map),
        });
        draft.changes.push(StateChange::SetComponent {
            entity: cast.notes,
            key: format!("lives.kind.{}", candidate.kind.id()),
            value: (period(state, &cast) as i64).into(),
        });
        draft.changes.extend(moves.changes);
        Ok(draft)
    }
}

/// What an answer does, how it is told, and what the asker says.
fn outcome(
    state: &WorldState,
    cast: &Cast,
    candidate: &Candidate,
    answer: &str,
) -> Result<(Moves, String, String), ActionError> {
    let words = words_for(state, cast, candidate);
    let w = |template: &str| fill_owned(template, &words);
    let a = candidate.a;
    let b = candidate.b;
    let mut moves = Moves::default();
    let spend = |moves: &mut Moves| -> Result<(), ActionError> {
        let fund = cast
            .fund
            .ok_or_else(|| ActionError::Invalid("no fund".into()))?;
        if integer(state, fund.entity, fund.key).unwrap_or(0) < fund.amount {
            return Err(ActionError::Invalid("not enough in the fund".into()));
        }
        moves.add(
            state,
            fund.entity,
            fund.key,
            -fund.amount,
            i64::MIN / 4,
            i64::MAX / 4,
        );
        Ok(())
    };
    let people = cast_ids(state);
    let heard = Heard::of(state, cast);
    let say = |options: &[&str]| {
        let options = options.iter().map(|option| w(option)).collect::<Vec<_>>();
        pick_line(&options, &heard, a)
    };
    // Where an answer sends people shows on the scene.
    let go = |moves: &mut Moves, who: EntityId, place: EntityId| {
        if state.entity(who).is_some() {
            moves.set(who, AT, Value::Entity(place));
        }
    };
    let (gathering, quiet) = (cast.gathering, cast.quiet);
    let now_period = period(state, cast) as i64;
    let work_of = |who: EntityId| (cast.work)(state, who).unwrap_or(gathering);
    let (told, said) = match (candidate.kind, answer) {
        (Kind::Feud, "mend") => {
            let b = b.unwrap_or(a);
            moves.opinion(state, a, b, 35);
            moves.opinion(state, b, a, 35);
            moves.regard(state, a, 5);
            moves.regard(state, b, 5);
            go(&mut moves, a, gathering);
            go(&mut moves, b, gathering);
            (
                w("{a} and {b} talked it through"),
                w("We talked, {b} and me. It's better."),
            )
        }
        (Kind::Feud, "side") => {
            let b = b.unwrap_or(a);
            moves.regard(state, a, 12);
            moves.regard(state, b, -15);
            moves.opinion(state, b, a, -10);
            go(&mut moves, a, gathering);
            go(&mut moves, b, quiet);
            (
                w("You took {a}'s side against {b}"),
                w("Thank you. Someone sees it."),
            )
        }
        (Kind::Feud, "leave") => {
            go(&mut moves, a, quiet);
            (
                w("{a} and {b} were left to sort it out"),
                w("Fine. I'll manage {b} myself."),
            )
        }
        (Kind::Feud, "lapse") => {
            let b = b.unwrap_or(a);
            moves.opinion(state, a, b, -10);
            moves.opinion(state, b, a, -10);
            go(&mut moves, a, quiet);
            (
                w("{a} and {b} stopped speaking"),
                w("Not a word from {b} since."),
            )
        }
        (Kind::Sweet, "ask") => {
            let b = b.unwrap_or(a);
            let yes = opinion(state, b, a) >= 25 || mix(&[a.0, b.0, 99]).is_multiple_of(3);
            if yes {
                moves.set(a, PARTNER, Value::Entity(b));
                moves.set(b, PARTNER, Value::Entity(a));
                moves.opinion(state, a, b, 15);
                moves.opinion(state, b, a, 20);
                moves.lack(state, a, Need::Company, -40);
                moves.lack(state, b, Need::Company, -40);
                moves.regard(state, a, 8);
                go(&mut moves, a, quiet);
                go(&mut moves, b, quiet);
                (
                    w("{a} and {b} started walking out together"),
                    w("{b} said yes!"),
                )
            } else {
                moves.opinion(state, a, b, -15);
                moves.lack(state, a, Need::Company, 20);
                go(&mut moves, a, quiet);
                (
                    w("{b} turned {a} down, kindly"),
                    w("{b} said no. Kindly, but no."),
                )
            }
        }
        (Kind::Sweet, "wait") | (Kind::Sweet, "lapse") => {
            go(&mut moves, a, gathering);
            (
                w("{a} kept it to themselves"),
                say(&[
                    "Maybe you're right. Maybe.",
                    "I'll wait. For now.",
                    "Best not rush it.",
                    "Not yet, then.",
                ]),
            )
        }
        (Kind::Learn, "teach") => {
            let b = b.unwrap_or(a);
            moves.lack(state, a, Need::Purpose, -50);
            moves.opinion(state, a, b, 18);
            moves.opinion(state, b, a, 10);
            moves.lack(state, b, Need::Rest, 15);
            moves.lack(state, b, Need::Purpose, -20);
            moves.regard(state, a, 8);
            go(&mut moves, a, work_of(b));
            (w("{b} began teaching {a}"), w("{b}'s a patient teacher."))
        }
        (Kind::Learn, "not_now") | (Kind::Learn, "lapse") => {
            moves.lack(state, a, Need::Purpose, 10);
            moves.regard(state, a, -5);
            go(&mut moves, a, quiet);
            (
                w("{a} was told not now"),
                say(&[
                    "Another time, then.",
                    "Maybe next month.",
                    "I'll ask again. Later.",
                    "Fair enough.",
                ]),
            )
        }
        (Kind::Short, "fund") => {
            spend(&mut moves)?;
            moves.lack(state, a, Need::Money, -55);
            moves.regard(state, a, 15);
            go(&mut moves, a, quiet);
            (
                w("The {settlement} tided {a} over"),
                w("I'll pay it back, every bit."),
            )
        }
        (Kind::Short, "friend") => {
            let friend = best_friend(state, a)
                .ok_or_else(|| ActionError::Invalid("no friend to ask".into()))?;
            moves.lack(state, a, Need::Money, -45);
            moves.lack(state, friend, Need::Money, 15);
            moves.opinion(state, a, friend, 15);
            moves.opinion(state, friend, a, -3);
            go(&mut moves, a, work_of(friend));
            (
                w("{friend} helped {a} out"),
                w("{friend} came through for me."),
            )
        }
        (Kind::Short, "manage") | (Kind::Short, "lapse") => {
            moves.regard(state, a, -8);
            moves.lack(state, a, Need::Money, 5);
            go(&mut moves, a, work_of(a));
            (
                w("{a} scraped by"),
                say(&[
                    "Right. I'll manage.",
                    "I'll find a way.",
                    "Tighten the belt, then.",
                    "Somehow. I'll manage.",
                ]),
            )
        }
        (Kind::Lonely, "invite") => {
            moves.lack(state, a, Need::Company, -55);
            let met = people
                .iter()
                .copied()
                .filter(|o| *o != a)
                .max_by_key(|o| (lack(state, *o, Need::Company), mix(&[a.0, o.0])));
            moves.regard(state, a, 6);
            go(&mut moves, a, gathering);
            if let Some(met) = met {
                go(&mut moves, met, gathering);
            }
            match met {
                Some(met) => {
                    moves.opinion(state, a, met, 15);
                    moves.opinion(state, met, a, 12);
                    moves.lack(state, met, Need::Company, -25);
                    let met_name = name(state, met);
                    (
                        format!(
                            "{} spent an evening at {} with {met_name}",
                            name(state, a),
                            name(state, cast.gathering)
                        ),
                        format!("{met_name} and I got talking. Lovely evening."),
                    )
                }
                None => (
                    w("{a} spent an evening at {gathering}"),
                    w("That did me good."),
                ),
            }
        }
        (Kind::Lonely, "space") | (Kind::Lonely, "lapse") => {
            moves.lack(state, a, Need::Company, 8);
            moves.regard(state, a, -3);
            go(&mut moves, a, quiet);
            (
                w("{a} kept to themselves"),
                say(&[
                    "Maybe tomorrow.",
                    "I'm fine on my own. Mostly.",
                    "Quiet suits me. For now.",
                    "Another evening, perhaps.",
                ]),
            )
        }
        (Kind::Worn, "rest") => {
            moves.lack(state, a, Need::Rest, -65);
            moves.lack(state, a, Need::Money, 10);
            moves.regard(state, a, 8);
            go(&mut moves, a, quiet);
            (
                w("{a} took a day off"),
                say(&[
                    "Slept till noon. Bliss.",
                    "A whole day off. I'd forgotten what that was.",
                    "Rested. Human again.",
                ]),
            )
        }
        (Kind::Worn, "push") => {
            moves.lack(state, a, Need::Rest, 10);
            moves.lack(state, a, Need::Money, -20);
            moves.regard(state, a, -6);
            go(&mut moves, a, work_of(a));
            (
                w("{a} pushed on"),
                say(&[
                    "One more push, then.",
                    "Fine. Back to it.",
                    "I'll keep going. For now.",
                ]),
            )
        }
        (Kind::Worn, "lapse") => {
            moves.lack(state, a, Need::Rest, -35);
            moves.regard(state, a, -4);
            go(&mut moves, a, quiet);
            (
                w("{a} took the day anyway"),
                say(&[
                    "I needed that. Sorry.",
                    "Had to stop. Couldn't go on.",
                    "Took the day. Don't be cross.",
                ]),
            )
        }
        (Kind::Party, "party") => {
            spend(&mut moves)?;
            let b = b.unwrap_or(a);
            for p in &people {
                moves.lack(state, *p, Need::Company, -30);
                moves.opinion(state, *p, a, 5);
                moves.opinion(state, *p, b, 5);
                moves.regard(state, *p, 3);
                go(&mut moves, *p, gathering);
            }
            moves.set(a, CELEBRATED, period(state, cast) as i64);
            (
                w("The whole {settlement} came to {a} and {b}'s party at {gathering}"),
                w("Best night in years!"),
            )
        }
        (Kind::Party, "quiet") | (Kind::Party, "lapse") => {
            moves.set(a, CELEBRATED, period(state, cast) as i64);
            go(&mut moves, a, quiet);
            if let Some(b) = b {
                go(&mut moves, b, quiet);
            }
            (w("{a} and {b} kept it small"), w("Just us two. Perfect."))
        }
        (Kind::Visitor, "welcome") | (Kind::Visitor, "lapse") => {
            let visitors = cast
                .visitors
                .ok_or_else(|| ActionError::Invalid("nobody comes here".into()))?;
            let visitor = b.ok_or_else(|| ActionError::Invalid("nobody at the door".into()))?;
            if state.entity(visitor).is_some() {
                return Err(ActionError::Invalid("already here".into()));
            }
            let seed = mix(&[visitor.0, candidate.topic, 17]);
            let visitor_name = pick(visitors.names, seed).copied().unwrap_or("A stranger");
            let (_, job) = pick(visitors.trades, seed / 7)
                .copied()
                .unwrap_or(("traveller", "traveller"));
            let mut newcomer =
                Entity::new(visitor, visitors.kind).with_component("name", visitor_name);
            for (key, value) in (visitors.components)(visitor_name, job) {
                newcomer = newcomer.with_component(key, value);
            }
            newcomer = newcomer
                .with_component("lives.newcomer", true)
                .with_component(AT, Value::Entity(gathering));
            moves.changes.push(StateChange::CreateEntity(newcomer));
            moves.regard(state, a, 5);
            if answer == "lapse" {
                (
                    w("{b}, a {trade} from {origin}, stayed on, and nobody minded"),
                    w("Looks like {b}'s staying. Fine by me."),
                )
            } else {
                (
                    w("{b}, a {trade} from {origin}, came to stay"),
                    w("Plenty of room here. Welcome, {b}."),
                )
            }
        }
        (Kind::Visitor, "decline") => (w("{b} moved on with {way_out}"), w("Maybe another time.")),
        (Kind::Leaving, "stay") => {
            moves.regard(state, a, 30);
            for need in Need::ALL {
                moves.lack(state, a, need, -20);
            }
            go(&mut moves, a, gathering);
            (
                w("{a} was asked to stay, and did"),
                w("All right. I'll stay. For now."),
            )
        }
        (Kind::Leaving, "go") | (Kind::Leaving, "lapse") => {
            moves.set(a, GONE, true);
            if let Some(p) = partner(state, a) {
                moves.unset(state, p, PARTNER);
                moves.lack(state, p, Need::Company, 40);
            }
            moves.unset(state, a, PARTNER);
            (w("{a} left the {settlement}"), w("I'll write. I promise."))
        }
        (Kind::RoughPatch, "talk") => {
            let b = b.unwrap_or(a);
            moves.opinion(state, a, b, 25);
            moves.opinion(state, b, a, 25);
            go(&mut moves, a, gathering);
            go(&mut moves, b, gathering);
            (
                w("{a} and {b} talked it out"),
                w("We're all right, {b} and me."),
            )
        }
        (Kind::RoughPatch, "apart") | (Kind::RoughPatch, "lapse") => {
            let b = b.unwrap_or(a);
            moves.unset(state, a, PARTNER);
            moves.unset(state, b, PARTNER);
            moves.opinion(state, a, b, -10);
            moves.opinion(state, b, a, -10);
            moves.lack(state, a, Need::Company, 30);
            moves.lack(state, b, Need::Company, 30);
            go(&mut moves, a, quiet);
            go(&mut moves, b, work_of(b));
            (
                w("{a} and {b} went their separate ways"),
                w("It's for the best. I think."),
            )
        }
        (Kind::Warming, "stay") => {
            moves.set(a, &door_key(Kind::Warming), now_period);
            moves.regard(state, a, 6);
            moves.lack(state, a, Need::Company, -15);
            (w("{a} shared a quiet moment with you"), w("That was nice."))
        }
        (Kind::Warming, "wave") => {
            moves.set(a, &door_key(Kind::Warming), now_period);
            moves.regard(state, a, 2);
            (w("{a} waved you on your way"), w("See you, then."))
        }
        // A moment that passes has passed: the friendship goes on to the
        // next door without it.
        (Kind::Warming, "lapse") => {
            moves.set(a, &door_key(Kind::Warming), now_period);
            let said = pick(&MISSED, mix(&[a.0, 3])).copied().unwrap_or("Oh well.");
            (w("{a} looked for you"), w(said))
        }
        (Kind::Invite, "go") => {
            moves.set(a, &door_key(Kind::Invite), now_period);
            moves.regard(state, a, 8);
            moves.lack(state, a, Need::Company, -20);
            moves.lack(state, a, Need::Purpose, -10);
            (
                w("{a} took you somewhere of theirs"),
                w("I'm glad you came."),
            )
        }
        (Kind::Invite, "another") => {
            moves.set(a, &door_key(Kind::Invite), now_period);
            moves.regard(state, a, 2);
            (
                w("{a} asked you along, another time"),
                w("Another time, then."),
            )
        }
        (Kind::Invite, "lapse") => {
            moves.set(a, &door_key(Kind::Invite), now_period);
            let said = pick(&MISSED, mix(&[a.0, 9]))
                .copied()
                .unwrap_or("Never mind.");
            (w("{a} went on their own"), w(said))
        }
        (Kind::Confide, "keep") => {
            moves.set(a, &door_key(Kind::Confide), now_period);
            moves.regard(state, a, 10);
            moves.lack(state, a, Need::Company, -20);
            (
                w("{a} told you something they'd never told anyone"),
                w("Thank you. That's a weight off."),
            )
        }
        (Kind::Confide, "share") => {
            moves.set(a, &door_key(Kind::Confide), now_period);
            moves.regard(state, a, 5);
            moves.lack(state, a, Need::Purpose, -15);
            (
                w("{a} told you a secret, and you told them to share it"),
                w("Maybe you're right. Maybe I will."),
            )
        }
        (Kind::Confide, "lapse") => (
            w("{a} almost told you something"),
            w("Never mind. It was nothing."),
        ),
        (Kind::Favour, "word") => {
            moves.set(a, &door_key(Kind::Favour), now_period);
            let friend = best_friend(state, a).unwrap_or(a);
            moves.regard(state, friend, 12);
            moves.regard(state, a, 3);
            (
                w("{a} put in a good word for you with {friend}"),
                w("I told {friend} what you're like. They'll see."),
            )
        }
        (Kind::Favour, "help") => {
            moves.set(a, &door_key(Kind::Favour), now_period);
            moves.regard(state, a, 3);
            // Whoever is worst off, besides the one doing the favour.
            let helped = people
                .iter()
                .copied()
                .filter(|p| *p != a && enrolled(state, *p) && !gone(state, *p))
                .max_by_key(|p| {
                    let worst = Need::ALL.iter().map(|n| lack(state, *p, *n)).max();
                    (worst.unwrap_or(0), std::cmp::Reverse(p.0))
                });
            match helped {
                Some(helped) => {
                    let need = Need::ALL
                        .into_iter()
                        .max_by_key(|n| lack(state, helped, *n))
                        .unwrap_or(Need::Company);
                    moves.lack(state, helped, need, -25);
                    moves.regard(state, helped, 5);
                    moves.opinion(state, helped, a, 10);
                    let helped_name = name(state, helped);
                    (
                        w("{a} helped {helped} out, as a favour to you")
                            .replace("{helped}", &helped_name),
                        w("Consider it done."),
                    )
                }
                None => (w("{a} offered you a favour"), w("Consider it done.")),
            }
        }
        (Kind::Favour, "nothing") => {
            moves.set(a, &door_key(Kind::Favour), now_period);
            moves.regard(state, a, 5);
            (
                w("{a} offered you a favour, and you asked for nothing"),
                w("Then I owe you twice."),
            )
        }
        (Kind::Favour, "lapse") => (w("{a}'s offer went unanswered"), w("Another time, then.")),
        (Kind::Keepsake, "keep") => {
            moves.set(a, &door_key(Kind::Keepsake), now_period);
            moves.regard(state, a, 5);
            (w("{a} gave you {keepsake}"), w("It suits you, having it."))
        }
        (Kind::Keepsake, "show") => {
            moves.set(a, &door_key(Kind::Keepsake), now_period);
            moves.regard(state, a, 3);
            for p in &people {
                if *p != a && enrolled(state, *p) {
                    moves.regard(state, *p, 2);
                }
            }
            (
                w("{a} gave you {keepsake}, and you put it where all can see"),
                w("Everyone's asking about it!"),
            )
        }
        (Kind::Keepsake, "lapse") => (
            w("{a} meant to give you something"),
            w("I'll find another moment."),
        ),
        (Kind::Cold, "sorry") => {
            moves.regard(state, a, 15);
            (
                w("You made your peace with {a}"),
                w("...Fine. Apology accepted."),
            )
        }
        (Kind::Cold, "leave") | (Kind::Cold, "lapse") => {
            go(&mut moves, a, quiet);
            (w("{a} kept their distance from you"), w("Suits me."))
        }
        _ => return Err(ActionError::Invalid(format!("no answer {answer}"))),
    };
    Ok((moves, told, said))
}

fn closing(state: &WorldState, cast: &Cast, candidate: &Candidate, moves: &mut Moves) {
    moves.unset(state, cast.notes, &open_key(&candidate.key()));
    moves.set(
        cast.notes,
        &rest_key(&candidate.family()),
        period(state, cast) as i64,
    );
}

/// Someone answers a situation, or it runs out.
struct Answers(fn(&WorldState) -> Cast);

impl Action for Answers {
    fn name(&self) -> &'static str {
        "lives_situation_answered"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let key = arg_text(request, "situation")?;
        let answer = arg_text(request, "answer")?;
        let candidate = Candidate::parse(key)
            .ok_or_else(|| ActionError::Invalid("no such situation".into()))?;
        if open_map(state, &cast, key).is_none() {
            return Err(ActionError::Invalid("not open".into()));
        }
        let lapsed = answer == "lapse";
        if !lapsed {
            let offered = compose(state, &cast, &candidate);
            let given = offered
                .answers
                .iter()
                .find(|offered| offered.id == answer)
                .ok_or_else(|| ActionError::Invalid(format!("{answer} is not an answer")))?;
            if let Some(why) = &given.unavailable {
                return Err(ActionError::Invalid(why.clone()));
            }
        }
        let (mut moves, told, said) = outcome(state, &cast, &candidate, answer)?;
        // At a door, they answer in their own words: the first reply to
        // the warmer answer, the second to the other.
        let said = match (door_scene(&cast, &candidate), lapsed) {
            (Some(scene), false) => {
                let warmer = matches!(answer, "stay" | "keep" | "go" | "word" | "help");
                let words = words_for(state, &cast, &candidate);
                fill_owned(scene.replies[usize::from(!warmer)], &words)
            }
            _ => said,
        };
        let heard = Heard::of(state, &cast);
        let said = personal(
            state,
            candidate.a,
            &said,
            &heard,
            mix(&[candidate.a.0, period(state, &cast), 5]),
        );
        remember_saying(&mut moves, &heard, &said);
        closing(state, &cast, &candidate, &mut moves);
        let mut draft = EventDraft::new(if lapsed {
            "situation_lapsed"
        } else {
            "situation_answered"
        });
        draft.actor = Some(candidate.a);
        draft.targets = candidate
            .b
            .filter(|b| state.entity(*b).is_some())
            .into_iter()
            .collect();
        draft.payload.insert("situation".into(), key.into());
        draft
            .payload
            .insert("kind".into(), candidate.kind.id().into());
        draft.payload.insert("answer".into(), answer.into());
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), said.into());
        if lapsed {
            draft.payload.insert("lapsed".into(), true.into());
        } else if candidate.kind == Kind::Keepsake {
            if let Some((_, what)) = words_for(state, &cast, &candidate)
                .into_iter()
                .find(|(slot, _)| *slot == "keepsake")
            {
                draft.payload.insert("keepsake".into(), what.into());
            }
        }
        draft.changes = moves.changes;
        Ok(draft)
    }
}

/// Someone near a deed of the player's says what they make of it.
struct Reacts(fn(&WorldState) -> Cast);

impl Action for Reacts {
    fn name(&self) -> &'static str {
        "lives_reacts"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let who = arg_entity(request, "who")?;
        let deed = arg_text(request, "deed")?;
        let thing = arg_text(request, "thing")?;
        let place = arg_entity(request, "place")?;
        if !enrolled(state, who) || gone(state, who) {
            return Err(ActionError::Invalid("nobody to react".into()));
        }
        let words = [
            ("thing", thing.to_lowercase()),
            ("place", name(state, place)),
            ("settlement", cast.settlement.to_string()),
        ];
        let pool: &[&str] = match deed {
            "built_by_hand" => &[
                "A {thing}! Just what {place} needed.",
                "You built that? Well, look at it.",
                "A {thing} by {place}. I'll be using that.",
            ],
            "decorated_by_hand" => &[
                "{thing} up at {place}! Cheers the place right up.",
                "Oh, that's pretty. Was that you?",
            ],
            "planted_by_hand" => &[
                "You planted that? I'll keep an eye on it.",
                "Something growing by {place}. About time.",
            ],
            "moved_by_hand" => &[
                "Better there, I think.",
                "Oh, you've moved it. It suits there.",
            ],
            _ => return Err(ActionError::Invalid(format!("nothing to say about {deed}"))),
        };
        let heard = Heard::of(state, &cast);
        let options = pool
            .iter()
            .map(|line| {
                fill_owned(
                    line,
                    &words
                        .iter()
                        .map(|(k, v)| (*k, v.clone()))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        let said = pick_line(&options, &heard, who);
        let said = match (cast.voice)(who) {
            Some(voice) => restyle(voice, &said, mix(&[who.0, heard.now, 41])),
            None => said,
        };
        let mut moves = Moves::default();
        remember_saying(&mut moves, &heard, &said);
        if deed != "moved_by_hand" {
            moves.regard(state, who, 2);
        }
        let mut draft = EventDraft::new("reacted");
        draft.actor = Some(who);
        draft.targets = vec![who];
        draft.payload.insert("deed".into(), deed.into());
        draft.payload.insert("thing".into(), thing.into());
        draft
            .payload
            .insert("place".into(), name(state, place).into());
        draft.payload.insert("said".into(), said.into());
        draft.changes = moves.changes;
        Ok(draft)
    }
}

/// Someone warming to the player shares a small moment with them: the
/// first of their five scenes, said unasked.
struct Warms(fn(&WorldState) -> Cast);

impl Action for Warms {
    fn name(&self) -> &'static str {
        "lives_warms"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let who = arg_entity(request, "who")?;
        if !enrolled(state, who) || gone(state, who) {
            return Err(ActionError::Invalid("nobody to warm".into()));
        }
        if door_opened(state, who, Kind::Warming).is_some() {
            return Err(ActionError::Invalid("already warmed".into()));
        }
        let candidate = Candidate {
            kind: Kind::Warming,
            a: who,
            b: None,
            topic: 0,
        };
        let words = words_for(state, &cast, &candidate);
        let scene = scene_of((cast.voice)(who), (cast.traits)(who), 0);
        let said = fill_owned(scene.prompt, &words);
        let mut moves = Moves::default();
        moves.set(who, &door_key(Kind::Warming), period(state, &cast) as i64);
        moves.regard(state, who, 2);
        let heard = Heard::of(state, &cast);
        remember_saying(&mut moves, &heard, &said);
        let mut draft = EventDraft::new("warmed");
        draft.actor = Some(who);
        draft.targets = vec![who];
        draft.payload.insert(
            "told".into(),
            format!("{} shared a quiet moment with you", first_name(state, who)).into(),
        );
        draft.payload.insert("said".into(), said.into());
        draft.changes = moves.changes;
        Ok(draft)
    }
}

/// Someone comes over to say hello to a new player.
struct Greets(fn(&WorldState) -> Cast);

impl Action for Greets {
    fn name(&self) -> &'static str {
        "lives_greets"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let who = arg_entity(request, "who")?;
        if state.entity(who).is_none() || gone(state, who) {
            return Err(ActionError::Invalid("nobody to say hello".into()));
        }
        let words = [
            ("name", first_name(state, who)),
            ("settlement", cast.settlement.to_string()),
        ];
        let said = pick(&GREETINGS, mix(&[who.0, 7]))
            .map(|line| fill_owned(line, &words))
            .unwrap_or_default();
        let mut draft = EventDraft::new("greeted");
        draft.actor = Some(who);
        draft.targets = vec![who];
        draft.payload.insert(
            "told".into(),
            format!("{} came over to say hello", first_name(state, who)).into(),
        );
        draft.payload.insert("said".into(), said.into());
        Ok(draft)
    }
}

/// Someone leaves the player something while they are away.
struct LeavesKeepsake(fn(&WorldState) -> Cast);

impl Action for LeavesKeepsake {
    fn name(&self) -> &'static str {
        "lives_leaves_keepsake"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let who = arg_entity(request, "who")?;
        let why = arg_text(request, "why")?;
        if !enrolled(state, who) || gone(state, who) {
            return Err(ActionError::Invalid("nobody to leave it".into()));
        }
        let words = [
            ("gathering", name(state, cast.gathering)),
            ("settlement", cast.settlement.to_string()),
            ("unit", cast.unit.to_string()),
        ];
        let welcome = arg_text(request, "welcome").is_ok();
        let pool: &[&str] = if welcome { &WELCOME } else { &LEFT_FOR_YOU };
        let what = pick(pool, mix(&[who.0, period(state, &cast), 31]))
            .map(|what| fill_owned(what, &words))
            .unwrap_or_default();
        let note = if welcome {
            format!("For your first day in the {}. Welcome.", cast.settlement)
        } else if why.is_empty() {
            "Missed you round here.".to_string()
        } else {
            format!("I kept this for you while you were away. {why}")
        };
        let told = if welcome {
            format!("{} gave you {what} to welcome you", first_name(state, who))
        } else {
            format!("{} left you {what}", first_name(state, who))
        };
        let mut draft = EventDraft::new("keepsake_left");
        draft.actor = Some(who);
        draft.targets = vec![who];
        draft.payload.insert("keepsake".into(), what.into());
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), note.into());
        Ok(draft)
    }
}

/// The person nearest a deed of the player's, to say what they make of
/// it: someone there now, else whoever works or lives there, else the
/// place's host.
fn reactor(state: &WorldState, cast: &Cast, place: EntityId) -> Option<EntityId> {
    let people = cast_ids(state)
        .into_iter()
        .filter(|p| enrolled(state, *p) && !gone(state, *p))
        .collect::<Vec<_>>();
    people
        .iter()
        .copied()
        .find(|p| at(state, *p) == Some(place))
        .or_else(|| {
            people.iter().copied().find(|p| {
                (cast.work)(state, *p) == Some(place) || (cast.home)(state, *p) == Some(place)
            })
        })
        .or_else(|| people.contains(&cast.host).then_some(cast.host))
        .or_else(|| people.first().copied())
}

/// Someone says what they make of the player's deed: something made,
/// put up, planted or moved. Nothing for gifts and invitations, which
/// their own Systems answer.
pub fn react_to(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    deed: EventId,
) -> Result<Option<EventId>, WorldError> {
    let Some(event) = world.event(deed) else {
        return Ok(None);
    };
    if !matches!(
        event.kind.as_str(),
        "built_by_hand" | "decorated_by_hand" | "planted_by_hand" | "moved_by_hand"
    ) {
        return Ok(None);
    }
    let (Some(thing), Some(place)) = (event.targets.first(), event.targets.last()) else {
        return Ok(None);
    };
    let state = world.state();
    let Some(who) = reactor(state, cast, *place) else {
        return Ok(None);
    };
    let request = ActionRequest::new("lives_reacts")
        .actor(who)
        .caused_by(deed)
        .arg("who", Value::Entity(who))
        .arg("deed", event.kind.clone())
        .arg("thing", name(state, *thing))
        .arg("place", Value::Entity(*place));
    let first =
        keepsakes(world).is_empty() && !world.events().iter().any(|event| event.kind == "reacted");
    let reaction = world.execute(actions, &request).ok().map(|event| event.id);
    // The player's first deed earns them something to keep at once, from
    // whoever saw it: their first minutes end with a keepsake in hand.
    if let (true, Some(reaction)) = (first, reaction) {
        let welcome = ActionRequest::new("lives_leaves_keepsake")
            .actor(who)
            .caused_by(reaction)
            .arg("who", Value::Entity(who))
            .arg("why", "")
            .arg("welcome", "first deed");
        let _ = world.execute(actions, &welcome);
    }
    Ok(reaction)
}

/// A new player is greeted: whoever welcomes strangers comes over, says
/// who they are, and suggests something to make. Once per World.
pub fn greet(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
) -> Result<Option<EventId>, WorldError> {
    if world.events().iter().any(|event| event.kind == "greeted") {
        return Ok(None);
    }
    let people = (cast.people)(world);
    let state = world.state();
    let Some(who) = std::iter::once(cast.host)
        .chain(people)
        .find(|person| state.entity(*person).is_some() && !gone(state, *person))
    else {
        return Ok(None);
    };
    let request = ActionRequest::new("lives_greets")
        .actor(who)
        .arg("who", Value::Entity(who));
    Ok(world.execute(actions, &request).ok().map(|event| event.id))
}

/// On the player's return, someone who thinks well of them leaves them
/// something to keep, with a line about what happened while they were
/// away (`why`, in the World's words, or empty).
pub fn leave_keepsake(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    why: &str,
) -> Result<Option<EventId>, WorldError> {
    let state = world.state();
    let now = period(state, cast);
    let Some(who) = cast_ids(state)
        .into_iter()
        .filter(|p| enrolled(state, *p) && !gone(state, *p))
        .max_by_key(|p| (regard(state, *p), mix(&[p.0, now])))
    else {
        return Ok(None);
    };
    let request = ActionRequest::new("lives_leaves_keepsake")
        .actor(who)
        .arg("who", Value::Entity(who))
        .arg("why", why);
    Ok(world.execute(actions, &request).ok().map(|event| event.id))
}

/// Something the player was given to keep: who gave it, what it is, what
/// they said with it, and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keepsake {
    pub from: EntityId,
    pub what: String,
    pub note: String,
    pub event: EventId,
    pub world_time: u64,
}

/// Everything the player has been given to keep, oldest first.
pub fn keepsakes(world: &World) -> Vec<Keepsake> {
    world
        .events()
        .iter()
        .filter(|event| {
            event.kind == "keepsake_left"
                || (event.kind == "situation_answered"
                    && event.payload.get("kind") == Some(&Value::Text("keepsake".into())))
                // Anything any System marks as given to keep: what a
                // garden the player planted grew, say.
                || event.payload.get("kept") == Some(&Value::Bool(true))
        })
        .filter_map(|event| {
            let text = |key: &str| match event.payload.get(key) {
                Some(Value::Text(text)) => Some(text.clone()),
                _ => None,
            };
            Some(Keepsake {
                from: event.actor?,
                what: text("keepsake")?,
                note: text("said").unwrap_or_default(),
                event: event.id,
                world_time: event.world_time,
            })
        })
        .collect()
}

/// Lines said long enough ago are forgotten, so the notes stay small.
struct Forgets(fn(&WorldState) -> Cast);

impl Action for Forgets {
    fn name(&self) -> &'static str {
        "lives_forget"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let now = period(state, &cast);
        let notes = state
            .entity(cast.notes)
            .ok_or_else(|| ActionError::Invalid("nothing to forget".into()))?;
        let stale = notes
            .components
            .iter()
            .filter(|(key, value)| {
                key.starts_with("lives.heard.")
                    && matches!(value, Value::Integer(at) if now.saturating_sub((*at).max(0) as u64) >= HEARD_PERIODS)
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        if stale.is_empty() {
            return Err(ActionError::Invalid("nothing to forget".into()));
        }
        let mut draft = EventDraft::new("lines_forgotten");
        draft.changes = stale
            .into_iter()
            .map(|key| StateChange::RemoveComponent {
                entity: cast.notes,
                key,
            })
            .collect();
        Ok(draft)
    }
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
    for person in &people {
        let request = ActionRequest::new("lives_day")
            .arg("person", Value::Entity(*person))
            .arg("others", others_arg(&people));
        if let Ok(event) = world.execute(actions, &request) {
            events.push(event.id);
        }
    }
    for (a, b) in before {
        let request = ActionRequest::new("lives_bond")
            .arg("a", Value::Entity(a))
            .arg("b", Value::Entity(b));
        if let Ok(event) = world.execute(actions, &request) {
            events.push(event.id);
        }
    }
    if period(world.state(), cast).is_multiple_of(30) {
        if let Ok(event) = world.execute(actions, &ActionRequest::new("lives_forget")) {
            events.push(event.id);
        }
    }
    if away {
        return Ok(events);
    }
    let now = period(world.state(), cast) as i64;
    for (key, opened) in open(world.state(), cast) {
        if now - opened >= LASTS {
            events.push(world.execute(actions, &answer_request(&key, "lapse"))?.id);
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
        if let Some(candidate) = candidates(world, cast).into_iter().next() {
            let request =
                ActionRequest::new("lives_situation_opens").arg("situation", candidate.key());
            events.push(world.execute(actions, &request)?.id);
        }
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
            | "greeted"
            | "warmed"
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
    matches!(
        event.kind.as_str(),
        "bond_changed" | "situation_came_up" | "situation_answered" | "situation_lapsed"
    )
}

/// What changed between people since a moment: who became friends, who
/// fell out, who got together or parted, who came and who left. Newest
/// last, one line each.
pub fn news_since(world: &World, since: u64) -> Vec<String> {
    let mut seen = BTreeSet::new();
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time >= since)
        .filter(|event| match event.kind.as_str() {
            "bond_changed" => true,
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
            seen.insert(told.clone()).then_some(told)
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
                "A bit lonely, if I'm honest.",
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
