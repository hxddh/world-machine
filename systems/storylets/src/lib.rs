//! Storylets: small situations a World hands its people, the way Fallen
//! London and RimWorld keep a place from running out of story.
//!
//! A storylet is someone with something on their mind. It needs certain
//! things to be true before it can come up, offers a few choices, and if
//! nobody chooses before it runs out, it ends its own way. Everything it does
//! goes through ordinary Actions, so what happens is recorded as Events and
//! replaying a World never needs the storyteller again.
//!
//! This System knows nothing about harbours or colonies. A World Pack gives
//! it a [`Deck`]: its storylets, the goals they build toward, and how long a
//! period and a chapter are. The Pack also decides how each moment is told.
//!
//! The storyteller ([`tick`]) runs once a period, like RimWorld's. It lets
//! lapse whatever has run out, turns the chapter when its time comes, and
//! makes sure something is always open. When a gauge the Pack reports is
//! pinned at one end, it reaches first for a storylet that eases it back.

use std::collections::BTreeMap;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft, EventId,
    StateChange, Value, World, WorldError, WorldState,
};

/// Everything a World's storyteller works from.
#[derive(Clone, Debug)]
pub struct Deck {
    /// The entity the storyteller keeps its notes on (what is open, when it
    /// last came up, which chapter this is, how far each goal has got).
    pub story: EntityId,
    /// What the story entity is called, when the storyteller first begins.
    pub story_name: &'static str,
    /// How much world time one period is.
    pub period: u64,
    pub storylets: Vec<Storylet>,
    pub goals: Vec<Goal>,
    /// How many periods a chapter runs before it turns.
    pub chapter_periods: u64,
    /// The fewest periods a chapter runs before a gauge at its end can turn
    /// it early.
    pub shortest_chapter: u64,
    /// What a new chapter starts from: the Pack's gauges set back, say.
    pub fresh_start: Vec<Effect>,
    /// How many storylets may be open at once.
    pub most_open: usize,
}

/// Someone with something on their mind.
#[derive(Clone, Debug)]
pub struct Storylet {
    pub id: &'static str,
    /// Whose it is: they ask it, and it is told as theirs.
    pub asker: EntityId,
    /// A want of the asker's own: granting it is remembered as a kindness,
    /// letting it lapse as a grudge.
    pub want: bool,
    /// What has to be true for it to come up at all.
    pub requires: Vec<Condition>,
    pub choices: Vec<Choice>,
    /// What happens when nobody chooses in time.
    pub lapse: Outcome,
    /// How many periods it stays open.
    pub lasts: u64,
    /// How many periods it rests after it ends before it can come up again.
    pub rests: u64,
    pub weight: u32,
    /// Gauges its outcome tends to move, and which way: what the storyteller
    /// reaches for when one is pinned.
    pub eases: Vec<Ease>,
    /// Tied to the calendar (a market day, a birthday): comes up first when
    /// its day comes, since the day will not wait.
    pub timely: bool,
}

/// A gauge a storylet tends to move, and which way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ease {
    pub gauge: &'static str,
    pub up: bool,
}

/// One way to answer a storylet.
#[derive(Clone, Debug)]
pub struct Choice {
    pub id: &'static str,
    /// What has to be true to be able to choose it (enough money, say).
    pub requires: Vec<Condition>,
    pub outcome: Outcome,
    /// Whether this answer turns a want down. Turning someone down is held
    /// against you the way letting it lapse is.
    pub refuses: bool,
}

/// What a choice or a lapse does: the Event it is recorded as, and what it
/// changes.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub event: &'static str,
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug)]
pub enum Effect {
    /// Add `by` to an integer, kept between `min` and `max`.
    Add {
        entity: EntityId,
        key: &'static str,
        by: i64,
        min: i64,
        max: i64,
    },
    /// Set a text value.
    Set {
        entity: EntityId,
        key: &'static str,
        text: &'static str,
    },
    /// Set an integer outright.
    Put {
        entity: EntityId,
        key: &'static str,
        to: i64,
    },
    /// Build one more part of a goal.
    Advance(&'static str),
}

/// Something that has to be true.
#[derive(Clone, Debug)]
pub enum Condition {
    AtLeast(EntityId, &'static str, i64),
    Below(EntityId, &'static str, i64),
    Is(EntityId, &'static str, &'static str),
    IsNot(EntityId, &'static str, &'static str),
    /// Every `every` periods, on the `at`th.
    Every {
        every: u64,
        at: u64,
    },
    /// In season `season` (0 to 3) of seasons `length` periods long.
    Season {
        length: u64,
        season: u64,
    },
    /// A goal not yet finished.
    Unfinished(&'static str),
    /// A goal finished.
    Finished(&'static str),
}

/// Something a World is building toward, part by part.
#[derive(Clone, Debug)]
pub struct Goal {
    pub id: &'static str,
    pub parts: i64,
}

fn key(kind: &str, id: &str) -> String {
    format!("story.{kind}.{id}")
}

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

/// Which period of the World it is.
pub fn period_index(state: &WorldState, deck: &Deck) -> u64 {
    state.world_time() / deck.period.max(1)
}

/// Which of the four seasons it is, for seasons `length` periods long.
pub fn season(period: u64, length: u64) -> u64 {
    (period / length.max(1)) % 4
}

/// How far a goal has got, in parts.
pub fn progress(state: &WorldState, deck: &Deck, goal: &str) -> i64 {
    integer(state, deck.story, &key("goal", goal)).unwrap_or(0)
}

fn finished(state: &WorldState, deck: &Deck, goal: &str) -> bool {
    deck.goals
        .iter()
        .find(|spec| spec.id == goal)
        .is_some_and(|spec| progress(state, deck, goal) >= spec.parts)
}

fn holds(state: &WorldState, deck: &Deck, condition: &Condition) -> bool {
    let period = period_index(state, deck);
    match condition {
        Condition::AtLeast(entity, key, n) => integer(state, *entity, key).unwrap_or(0) >= *n,
        Condition::Below(entity, key, n) => integer(state, *entity, key).unwrap_or(0) < *n,
        Condition::Is(entity, key, value) => text(state, *entity, key) == Some(*value),
        Condition::IsNot(entity, key, value) => text(state, *entity, key) != Some(*value),
        Condition::Every { every, at } => period % (*every).max(1) == *at % (*every).max(1),
        Condition::Season {
            length,
            season: which,
        } => season(period, *length) == *which,
        Condition::Unfinished(goal) => !finished(state, deck, goal),
        Condition::Finished(goal) => finished(state, deck, goal),
    }
}

fn all_hold(state: &WorldState, deck: &Deck, conditions: &[Condition]) -> bool {
    conditions
        .iter()
        .all(|condition| holds(state, deck, condition))
}

/// When a storylet came up, if it is open.
pub fn opened_at(state: &WorldState, deck: &Deck, id: &str) -> Option<u64> {
    integer(state, deck.story, &key("open", id)).map(|at| at.max(0) as u64)
}

/// The storylets open now, oldest first.
pub fn open<'a>(state: &WorldState, deck: &'a Deck) -> Vec<&'a Storylet> {
    let mut open = deck
        .storylets
        .iter()
        .filter_map(|storylet| Some((opened_at(state, deck, storylet.id)?, storylet)))
        .collect::<Vec<_>>();
    open.sort_by_key(|(at, storylet)| (*at, storylet.id));
    open.into_iter().map(|(_, storylet)| storylet).collect()
}

/// The choices that can be made now: every choice of every open storylet
/// whose own conditions hold.
pub fn choices<'a>(state: &WorldState, deck: &'a Deck) -> Vec<(&'a Storylet, &'a Choice)> {
    open(state, deck)
        .into_iter()
        .flat_map(|storylet| {
            storylet
                .choices
                .iter()
                .filter(|choice| all_hold(state, deck, &choice.requires))
                .map(move |choice| (storylet, choice))
        })
        .collect()
}

/// Whether a storylet could come up now.
pub fn can_arise(state: &WorldState, deck: &Deck, storylet: &Storylet) -> bool {
    if opened_at(state, deck, storylet.id).is_some() {
        return false;
    }
    let rested = match integer(state, deck.story, &key("last", storylet.id)) {
        Some(last) => {
            state.world_time() >= (last.max(0) as u64).saturating_add(storylet.rests * deck.period)
        }
        None => true,
    };
    rested && all_hold(state, deck, &storylet.requires)
}

/// How many times someone's wants have been granted, and let lapse.
pub fn kindness(state: &WorldState, deck: &Deck, person: EntityId) -> (i64, i64) {
    let who = person.to_string();
    (
        integer(state, deck.story, &key("granted", &who)).unwrap_or(0),
        integer(state, deck.story, &key("grudge", &who)).unwrap_or(0),
    )
}

/// When someone last had a want granted.
pub fn last_granted(state: &WorldState, deck: &Deck, person: EntityId) -> Option<u64> {
    integer(state, deck.story, &key("thanked", &person.to_string())).map(|at| at.max(0) as u64)
}

/// Which chapter this is (from 1), and when it began.
pub fn chapter(state: &WorldState, deck: &Deck) -> (i64, u64) {
    (
        integer(state, deck.story, "story.chapter").unwrap_or(1),
        integer(state, deck.story, "story.chapter_started")
            .unwrap_or(0)
            .max(0) as u64,
    )
}

fn applied(state: &WorldState, deck: &Deck, effects: &[Effect]) -> Vec<StateChange> {
    // Effects on the same value build on each other.
    let mut values = BTreeMap::<(EntityId, String), i64>::new();
    let mut changes = Vec::new();
    for effect in effects {
        match effect {
            Effect::Add {
                entity,
                key,
                by,
                min,
                max,
            } => {
                let current = *values
                    .entry((*entity, key.to_string()))
                    .or_insert_with(|| integer(state, *entity, key).unwrap_or(0));
                let next = current.saturating_add(*by).clamp(*min, *max);
                values.insert((*entity, key.to_string()), next);
                changes.push(StateChange::SetComponent {
                    entity: *entity,
                    key: key.to_string(),
                    value: next.into(),
                });
            }
            Effect::Put { entity, key, to } => {
                values.insert((*entity, key.to_string()), *to);
                changes.push(StateChange::SetComponent {
                    entity: *entity,
                    key: key.to_string(),
                    value: (*to).into(),
                });
            }
            Effect::Set { entity, key, text } => changes.push(StateChange::SetComponent {
                entity: *entity,
                key: key.to_string(),
                value: (*text).into(),
            }),
            Effect::Advance(goal) => {
                let parts = deck
                    .goals
                    .iter()
                    .find(|spec| spec.id == *goal)
                    .map(|spec| spec.parts)
                    .unwrap_or(1);
                let key = key("goal", goal);
                let current = *values
                    .entry((deck.story, key.clone()))
                    .or_insert_with(|| integer(state, deck.story, &key).unwrap_or(0));
                let next = (current + 1).min(parts);
                values.insert((deck.story, key.clone()), next);
                changes.push(StateChange::SetComponent {
                    entity: deck.story,
                    key,
                    value: next.into(),
                });
            }
        }
    }
    changes
}

fn arg_text<'a>(request: &'a ActionRequest, name: &str) -> Result<&'a str, ActionError> {
    match request.args.get(name) {
        Some(Value::Text(value)) => Ok(value),
        _ => Err(ActionError::Invalid(format!("missing {name}"))),
    }
}

fn find<'a>(deck: &'a Deck, id: &str) -> Result<&'a Storylet, ActionError> {
    deck.storylets
        .iter()
        .find(|storylet| storylet.id == id)
        .ok_or_else(|| ActionError::Invalid(format!("no storylet {id}")))
}

/// Ends an open storylet: it is no longer open, and it rests from now.
fn closing(state: &WorldState, deck: &Deck, storylet: &Storylet) -> Vec<StateChange> {
    vec![
        StateChange::RemoveComponent {
            entity: deck.story,
            key: key("open", storylet.id),
        },
        StateChange::SetComponent {
            entity: deck.story,
            key: key("last", storylet.id),
            value: (state.world_time() as i64).into(),
        },
    ]
}

/// One more grudge held by someone whose want went unmet.
fn grudge(state: &WorldState, deck: &Deck, person: EntityId) -> StateChange {
    let who = person.to_string();
    let grudges = integer(state, deck.story, &key("grudge", &who)).unwrap_or(0);
    StateChange::SetComponent {
        entity: deck.story,
        key: key("grudge", &who),
        value: (grudges + 1).into(),
    }
}

/// The storyteller begins: the entity it keeps its notes on is made.
struct Begins(fn() -> Deck);

impl Action for Begins {
    fn name(&self) -> &'static str {
        "story_begins"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = (self.0)();
        if state.entity(deck.story).is_some() {
            return Err(ActionError::Invalid("the story has begun".into()));
        }
        let mut draft = EventDraft::new("story_began");
        draft.changes.push(StateChange::CreateEntity(
            Entity::new(deck.story, "story")
                .with_component("name", deck.story_name)
                .with_component("story.chapter", 1_i64)
                .with_component("story.chapter_started", state.world_time() as i64),
        ));
        Ok(draft)
    }
}

/// A storylet comes up.
struct Arises(fn() -> Deck);

impl Action for Arises {
    fn name(&self) -> &'static str {
        "storylet_arises"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = (self.0)();
        let storylet = find(&deck, arg_text(request, "storylet")?)?;
        if !can_arise(state, &deck, storylet) {
            return Err(ActionError::Invalid(format!(
                "{} cannot come up now",
                storylet.id
            )));
        }
        let mut draft = EventDraft::new("situation_arose");
        draft.actor = Some(storylet.asker);
        draft.targets = vec![storylet.asker];
        draft.payload.insert("storylet".into(), storylet.id.into());
        draft.payload.insert("want".into(), storylet.want.into());
        draft.changes.push(StateChange::SetComponent {
            entity: deck.story,
            key: key("open", storylet.id),
            value: (state.world_time() as i64).into(),
        });
        Ok(draft)
    }
}

/// Someone answers an open storylet.
struct Chosen(fn() -> Deck);

impl Action for Chosen {
    fn name(&self) -> &'static str {
        "storylet_chosen"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = (self.0)();
        let storylet = find(&deck, arg_text(request, "storylet")?)?;
        let wanted = arg_text(request, "choice")?;
        if opened_at(state, &deck, storylet.id).is_none() {
            return Err(ActionError::Invalid(format!("{} is not open", storylet.id)));
        }
        let choice = storylet
            .choices
            .iter()
            .find(|choice| choice.id == wanted)
            .ok_or_else(|| ActionError::Invalid(format!("no choice {wanted}")))?;
        if !all_hold(state, &deck, &choice.requires) {
            return Err(ActionError::Invalid(format!(
                "{wanted} cannot be chosen now"
            )));
        }
        let mut draft = EventDraft::new(choice.outcome.event);
        draft.actor = Some(storylet.asker);
        draft.targets = vec![storylet.asker];
        draft.payload.insert("storylet".into(), storylet.id.into());
        draft.payload.insert("choice".into(), choice.id.into());
        draft.changes = applied(state, &deck, &choice.outcome.effects);
        draft.changes.extend(closing(state, &deck, storylet));
        if storylet.want && choice.refuses {
            draft.changes.push(grudge(state, &deck, storylet.asker));
        } else if storylet.want {
            let who = storylet.asker.to_string();
            let granted = integer(state, deck.story, &key("granted", &who)).unwrap_or(0);
            draft.changes.push(StateChange::SetComponent {
                entity: deck.story,
                key: key("granted", &who),
                value: (granted + 1).into(),
            });
            draft.changes.push(StateChange::SetComponent {
                entity: deck.story,
                key: key("thanked", &who),
                value: (state.world_time() as i64).into(),
            });
        }
        Ok(draft)
    }
}

/// An open storylet runs out, and ends its own way.
struct Lapsed(fn() -> Deck);

impl Action for Lapsed {
    fn name(&self) -> &'static str {
        "storylet_lapsed"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = (self.0)();
        let storylet = find(&deck, arg_text(request, "storylet")?)?;
        if opened_at(state, &deck, storylet.id).is_none() {
            return Err(ActionError::Invalid(format!("{} is not open", storylet.id)));
        }
        let mut draft = EventDraft::new(storylet.lapse.event);
        draft.actor = Some(storylet.asker);
        draft.targets = vec![storylet.asker];
        draft.payload.insert("storylet".into(), storylet.id.into());
        draft.payload.insert("lapsed".into(), true.into());
        draft.changes = applied(state, &deck, &storylet.lapse.effects);
        draft.changes.extend(closing(state, &deck, storylet));
        if storylet.want {
            draft.changes.push(grudge(state, &deck, storylet.asker));
        }
        Ok(draft)
    }
}

/// A chapter ends, told in the Pack's words, and the next begins.
struct ChapterTurns(fn() -> Deck);

impl Action for ChapterTurns {
    fn name(&self) -> &'static str {
        "chapter_turns"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = (self.0)();
        let (number, _) = chapter(state, &deck);
        let mut draft = EventDraft::new("chapter_ended");
        draft.payload.insert("chapter".into(), number.into());
        for name in ["title", "summary"] {
            draft
                .payload
                .insert(name.into(), arg_text(request, name)?.into());
        }
        draft.changes = vec![
            StateChange::SetComponent {
                entity: deck.story,
                key: "story.chapter".into(),
                value: (number + 1).into(),
            },
            StateChange::SetComponent {
                entity: deck.story,
                key: "story.chapter_started".into(),
                value: (state.world_time() as i64).into(),
            },
        ];
        draft
            .changes
            .extend(applied(state, &deck, &deck.fresh_start));
        Ok(draft)
    }
}

/// Registers the storyteller's Actions for a Pack's deck.
pub fn register_actions(
    registry: &mut ActionRegistry,
    deck: fn() -> Deck,
) -> Result<(), ActionError> {
    registry.register(Begins(deck))?;
    registry.register(Arises(deck))?;
    registry.register(Chosen(deck))?;
    registry.register(Lapsed(deck))?;
    registry.register(ChapterTurns(deck))?;
    Ok(())
}

/// The request that answers a storylet with a choice.
pub fn choose_request(storylet: &str, choice: &str) -> ActionRequest {
    ActionRequest::new("storylet_chosen")
        .arg("storylet", storylet)
        .arg("choice", choice)
}

/// A small stable number for mixing: the same inputs always give the same
/// answer, so the storyteller is the same every replay.
pub fn mix(parts: &[u64]) -> u64 {
    parts.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, part| {
        (hash ^ part).wrapping_mul(0x0100_0000_01b3).rotate_left(17)
    })
}

/// A stable number for a piece of text.
pub fn text_hash(text: &str) -> u64 {
    mix(&text.bytes().map(u64::from).collect::<Vec<_>>())
}

/// A gauge at one end: its id, and whether it is stuck high.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pinned {
    pub gauge: &'static str,
    pub high: bool,
}

/// How a Pack tells a chapter's ending: a title and a sentence.
pub type ChapterEnding = dyn Fn(&World) -> (String, String);

/// What the storyteller is told about how the World stands.
pub struct Reading {
    pub pinned: Vec<Pinned>,
    /// Whether a gauge has reached its very end. That is a turning point:
    /// once the chapter has run its shortest, it ends there.
    pub at_end: bool,
    /// The chapter's ending, in the Pack's words: a title and a sentence.
    pub chapter_ending: Box<ChapterEnding>,
}

fn eases_pinned(storylet: &Storylet, pinned: &[Pinned]) -> bool {
    storylet.eases.iter().any(|ease| {
        pinned
            .iter()
            .any(|pin| pin.gauge == ease.gauge && pin.high != ease.up)
    })
}

/// One period of the storyteller. Lets lapse whatever has run out, turns
/// the chapter when its time comes, and opens what the World needs next:
/// what the calendar brings, something to ease a pinned gauge, and always
/// at least one thing to decide.
pub fn tick(
    world: &mut World,
    actions: &ActionRegistry,
    deck: &Deck,
    reading: &Reading,
) -> Result<Vec<EventId>, WorldError> {
    let mut events = Vec::new();
    if world.state().entity(deck.story).is_none() {
        events.push(
            world
                .execute(actions, &ActionRequest::new("story_begins"))?
                .id,
        );
    }
    let now = world.world_time();
    for storylet in open(world.state(), deck) {
        let at = opened_at(world.state(), deck, storylet.id).unwrap_or(now);
        if now >= at.saturating_add(storylet.lasts.max(1) * deck.period) {
            let request = ActionRequest::new("storylet_lapsed")
                .actor(storylet.asker)
                .arg("storylet", storylet.id);
            events.push(world.execute(actions, &request)?.id);
        }
    }
    let (_, started) = chapter(world.state(), deck);
    let due = now >= started.saturating_add(deck.chapter_periods.max(1) * deck.period);
    let turned =
        reading.at_end && now >= started.saturating_add(deck.shortest_chapter.max(1) * deck.period);
    if due || turned {
        let (title, summary) = (reading.chapter_ending)(world);
        let request = ActionRequest::new("chapter_turns")
            .arg("title", title)
            .arg("summary", summary);
        events.push(world.execute(actions, &request)?.id);
    }
    let period = period_index(world.state(), deck);
    loop {
        let state = world.state();
        let open_now = open(state, deck);
        if open_now.len() >= deck.most_open {
            break;
        }
        let easing = open_now
            .iter()
            .any(|storylet| eases_pinned(storylet, &reading.pinned));
        let wants_open = open_now.iter().any(|storylet| storylet.want);
        let score = |storylet: &Storylet| -> u64 {
            let mut score = u64::from(storylet.weight) * 10;
            if storylet.timely {
                score += 1_000;
            }
            if !easing && eases_pinned(storylet, &reading.pinned) {
                score += 500;
            }
            if storylet.want && !wants_open {
                score += 50;
            }
            score * 1_000 + mix(&[period, text_hash(storylet.id)]) % 997
        };
        // Past the first, only what cannot wait, what the World needs, or
        // someone's want when nobody has one open.
        let needed = |storylet: &Storylet| {
            open_now.is_empty()
                || storylet.timely
                || (!easing && eases_pinned(storylet, &reading.pinned))
                || (storylet.want && !wants_open)
        };
        let pick = deck
            .storylets
            .iter()
            .filter(|storylet| can_arise(state, deck, storylet) && needed(storylet))
            .max_by_key(|storylet| (score(storylet), storylet.id));
        let Some(pick) = pick else {
            break;
        };
        let request = ActionRequest::new("storylet_arises")
            .actor(pick.asker)
            .arg("storylet", pick.id);
        events.push(world.execute(actions, &request)?.id);
    }
    Ok(events)
}

/// A line from `pool` not said recently: starting from a place chosen by
/// `key`, the first that is not in `recent`. With every line recent, the
/// one at `key`'s place.
pub fn pick_line<'a>(pool: &[&'a str], key: u64, recent: &[String]) -> Option<&'a str> {
    if pool.is_empty() {
        return None;
    }
    let start = (key % pool.len() as u64) as usize;
    (0..pool.len())
        .map(|offset| pool[(start + offset) % pool.len()])
        .find(|line| !recent.iter().any(|said| said == line))
        .or(Some(pool[start]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::{Entity, WorldState};

    const STORY: EntityId = EntityId::new(1);
    const ANN: EntityId = EntityId::new(2);

    fn deck() -> Deck {
        let coins = |by: i64| Effect::Add {
            entity: ANN,
            key: "coins",
            by,
            min: 0,
            max: 100,
        };
        Deck {
            story: STORY,
            story_name: "Ann's year",
            period: 10,
            storylets: vec![
                Storylet {
                    id: "roof",
                    asker: ANN,
                    want: true,
                    requires: vec![Condition::Unfinished("house")],
                    choices: vec![Choice {
                        id: "mend",
                        requires: vec![Condition::AtLeast(ANN, "coins", 5)],
                        refuses: false,
                        outcome: Outcome {
                            event: "roof_mended",
                            effects: vec![coins(-5), Effect::Advance("house")],
                        },
                    }],
                    lapse: Outcome {
                        event: "roof_leaked",
                        effects: vec![coins(-1)],
                    },
                    lasts: 2,
                    rests: 1,
                    weight: 1,
                    eases: vec![Ease {
                        gauge: "coins",
                        up: false,
                    }],
                    timely: false,
                },
                Storylet {
                    id: "market",
                    asker: ANN,
                    want: false,
                    requires: vec![Condition::Every { every: 3, at: 0 }],
                    choices: vec![Choice {
                        id: "sell",
                        requires: Vec::new(),
                        refuses: false,
                        outcome: Outcome {
                            event: "sold_at_market",
                            effects: vec![coins(4)],
                        },
                    }],
                    lapse: Outcome {
                        event: "market_missed",
                        effects: Vec::new(),
                    },
                    lasts: 1,
                    rests: 1,
                    weight: 1,
                    eases: Vec::new(),
                    timely: true,
                },
                Storylet {
                    id: "chat",
                    asker: ANN,
                    want: false,
                    requires: Vec::new(),
                    choices: vec![Choice {
                        id: "listen",
                        requires: Vec::new(),
                        refuses: false,
                        outcome: Outcome {
                            event: "listened",
                            effects: Vec::new(),
                        },
                    }],
                    lapse: Outcome {
                        event: "chat_passed",
                        effects: Vec::new(),
                    },
                    lasts: 1,
                    rests: 0,
                    weight: 0,
                    eases: Vec::new(),
                    timely: false,
                },
            ],
            goals: vec![Goal {
                id: "house",
                parts: 2,
            }],
            chapter_periods: 4,
            shortest_chapter: 2,
            fresh_start: vec![Effect::Put {
                entity: ANN,
                key: "coins",
                to: 10,
            }],
            most_open: 2,
        }
    }

    fn world() -> (World, ActionRegistry) {
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(ANN, "person").with_component("coins", 10_i64))
            .unwrap();
        let mut actions = ActionRegistry::new();
        register_actions(&mut actions, deck).unwrap();
        let mut world = World::new(state);
        world
            .execute(&actions, &ActionRequest::new("story_begins"))
            .unwrap();
        (world, actions)
    }

    fn reading() -> Reading {
        Reading {
            pinned: Vec::new(),
            at_end: false,
            chapter_ending: Box::new(|_| ("An end".into(), "It went well.".into())),
        }
    }

    fn pass(world: &mut World, actions: &ActionRegistry) -> Vec<EventId> {
        let next = world.world_time() + 10;
        world.advance_to(actions, next).unwrap();
        tick(world, actions, &deck(), &reading()).unwrap()
    }

    fn kinds(world: &World) -> Vec<String> {
        world
            .events()
            .iter()
            .map(|event| event.kind.clone())
            .collect()
    }

    #[test]
    fn something_is_always_open_and_the_calendar_comes_first() {
        let (mut world, actions) = world();
        tick(&mut world, &actions, &deck(), &reading()).unwrap();
        let open_now = open(world.state(), &deck())
            .iter()
            .map(|storylet| storylet.id)
            .collect::<Vec<_>>();
        assert_eq!(
            open_now,
            vec!["market", "roof"],
            "market day first, then a want"
        );
        for _ in 0..12 {
            pass(&mut world, &actions);
            assert!(!open(world.state(), &deck()).is_empty());
        }
    }

    #[test]
    fn a_choice_does_what_it_says_and_a_want_granted_is_remembered() {
        let (mut world, actions) = world();
        tick(&mut world, &actions, &deck(), &reading()).unwrap();
        world
            .execute(&actions, &choose_request("roof", "mend"))
            .unwrap();
        let state = world.state();
        assert_eq!(integer(state, ANN, "coins"), Some(5));
        assert_eq!(progress(state, &deck(), "house"), 1);
        assert_eq!(kindness(state, &deck(), ANN), (1, 0));
        assert!(opened_at(state, &deck(), "roof").is_none());
        // It rests before it can come up again.
        assert!(!can_arise(state, &deck(), &deck().storylets[0]));
    }

    #[test]
    fn what_nobody_answers_ends_its_own_way_and_is_held_against_you() {
        let (mut world, actions) = world();
        tick(&mut world, &actions, &deck(), &reading()).unwrap();
        pass(&mut world, &actions);
        pass(&mut world, &actions);
        assert!(kinds(&world).contains(&"roof_leaked".to_string()));
        assert_eq!(kindness(world.state(), &deck(), ANN).1, 1);
    }

    #[test]
    fn a_choice_that_cannot_be_afforded_is_not_offered() {
        let (mut world, actions) = world();
        world
            .execute(
                &actions,
                &ActionRequest::new("storylet_arises").arg("storylet", "roof"),
            )
            .unwrap();
        assert_eq!(choices(world.state(), &deck()).len(), 1);
        let mut drained = World::new({
            let mut state = WorldState::default();
            state
                .seed_entity(Entity::new(ANN, "person").with_component("coins", 2_i64))
                .unwrap();
            state
        });
        drained
            .execute(&actions, &ActionRequest::new("story_begins"))
            .unwrap();
        drained
            .execute(
                &actions,
                &ActionRequest::new("storylet_arises").arg("storylet", "roof"),
            )
            .unwrap();
        assert!(choices(drained.state(), &deck()).is_empty());
        assert!(drained
            .execute(&actions, &choose_request("roof", "mend"))
            .is_err());
    }

    #[test]
    fn chapters_turn_on_time_in_the_packs_words() {
        let (mut world, actions) = world();
        for _ in 0..5 {
            pass(&mut world, &actions);
        }
        let ended = world
            .events()
            .iter()
            .find(|event| event.kind == "chapter_ended")
            .expect("a chapter ends after four periods");
        assert_eq!(
            ended.payload.get("title"),
            Some(&Value::Text("An end".into()))
        );
        assert_eq!(chapter(world.state(), &deck()).0, 2);
    }

    #[test]
    fn a_gauge_at_its_end_turns_the_chapter_and_the_next_starts_fresh() {
        let (mut world, actions) = world();
        let at_end = Reading {
            pinned: Vec::new(),
            at_end: true,
            chapter_ending: Box::new(|_| ("Broke".into(), "Ann ran out.".into())),
        };
        world.advance_to(&actions, 10).unwrap();
        tick(&mut world, &actions, &deck(), &at_end).unwrap();
        assert_eq!(chapter(world.state(), &deck()).0, 1, "too soon to turn");
        world
            .execute(
                &actions,
                &ActionRequest::new("storylet_chosen")
                    .arg("storylet", "roof")
                    .arg("choice", "mend"),
            )
            .ok();
        world.advance_to(&actions, 20).unwrap();
        tick(&mut world, &actions, &deck(), &at_end).unwrap();
        assert_eq!(chapter(world.state(), &deck()).0, 2);
        assert_eq!(integer(world.state(), ANN, "coins"), Some(10));
    }

    #[test]
    fn a_pinned_gauge_brings_what_eases_it() {
        let (mut world, actions) = world();
        let reading = Reading {
            pinned: vec![Pinned {
                gauge: "coins",
                high: true,
            }],
            at_end: false,
            chapter_ending: Box::new(|_| (String::new(), String::new())),
        };
        // Off market day, with nothing open: the want that spends comes up.
        world.advance_to(&actions, 10).unwrap();
        tick(&mut world, &actions, &deck(), &reading).unwrap();
        assert!(opened_at(world.state(), &deck(), "roof").is_some());
    }

    #[test]
    fn lines_rest_before_they_are_said_again() {
        let pool = ["a", "b", "c"];
        assert_eq!(pick_line(&pool, 1, &[]), Some("b"));
        assert_eq!(pick_line(&pool, 1, &["b".into()]), Some("c"));
        assert_eq!(
            pick_line(&pool, 1, &["a".into(), "b".into(), "c".into()]),
            Some("b")
        );
    }

    #[test]
    fn the_storyteller_is_the_same_every_time() {
        let run = || {
            let (mut world, actions) = world();
            for _ in 0..10 {
                pass(&mut world, &actions);
            }
            kinds(&world)
        };
        assert_eq!(run(), run());
    }
}
