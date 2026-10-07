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

pub mod script;

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
    /// The pressures chapters are about, taken in turn: each chapter opens
    /// on one, and its climax can come only near the chapter's end
    /// ([`Condition::Pressure`], [`Condition::ChapterEnding`]).
    pub pressures: Vec<&'static str>,
    /// How many storylets may be open at once.
    pub most_open: usize,
    /// How much rarer a storylet grows each time it comes round: after its
    /// `n`th time it rests at least `rarer * (n - 1)²` periods, so a
    /// second storm follows the first as it always did, a third waits
    /// longer, and a fourth longer still. What builds toward an unfinished
    /// goal keeps its own pace. Nought keeps every storylet to its own rest.
    pub rarer: u64,
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
    /// Who sees to it when the player turns it down or lets it lapse with
    /// nothing built, and what their seeing to it does. A want let down
    /// never comes up again as it was: once it has rested, someone else
    /// takes it up ([`TakenUp`]), or, with nobody to, it is dropped.
    /// Anything else with someone to see to it (a part of a work, say) is
    /// let down the same way.
    pub taken_up: Option<TakenUp>,
}

/// Someone else seeing to a want the player let down: who, and what it
/// does (recorded as `outcome.event`, caused by the letting down).
#[derive(Clone, Debug)]
pub struct TakenUp {
    pub by: EntityId,
    pub outcome: Outcome,
    /// What has to be true for them to take it up; until it is, the want
    /// stays let down.
    pub requires: Vec<Condition>,
}

/// How a storylet last ended when someone else took it up.
pub const TAKEN_UP: &str = "taken_up";
/// On whoever took a want up: the last one they took on.
pub const TOOK_UP: &str = "story.took_up";
/// On whoever's want someone else took up: who, the last time.
pub const SEEN_TO_BY: &str = "story.seen_to_by";

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
    /// Move an integer toward `target` by up to `by`.
    Toward {
        entity: EntityId,
        key: &'static str,
        target: i64,
        by: i64,
    },
    /// Remove a value, if it is there.
    Unset { entity: EntityId, key: &'static str },
    /// Set an integer outright.
    Put {
        entity: EntityId,
        key: &'static str,
        to: i64,
    },
    /// Build one more part of a goal.
    Advance(&'static str),
    /// Remember that something happened, for later storylets to follow
    /// from ([`Condition::Marked`]).
    Mark(&'static str),
    /// Forget a mark.
    Unmark(&'static str),
    /// Put something on the scene: a fixture entity, standing at a place,
    /// for `lasts` periods or for good. Building it again renews it.
    Build {
        entity: EntityId,
        name: &'static str,
        /// How it is drawn, in the Pack's own words ("stall", "bunting").
        shape: &'static str,
        at: EntityId,
        lasts: Option<u64>,
    },
    /// Take a fixture away.
    Demolish(EntityId),
    /// Someone or something new arrives: an entity of `kind`, with its
    /// components. Nothing happens if it is already there.
    Arrive {
        entity: EntityId,
        kind: &'static str,
        components: Vec<(&'static str, Value)>,
    },
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
    /// Something marked ([`Effect::Mark`]) at least `periods` ago.
    Marked(&'static str, u64),
    /// Something never marked, or forgotten.
    Unmarked(&'static str),
    /// Something marked no more than `periods` ago.
    MarkedWithin(&'static str, u64),
    /// An entity is in the World.
    Present(EntityId),
    /// An entity is not in the World.
    Absent(EntityId),
    /// This chapter is about this pressure.
    Pressure(&'static str),
    /// This chapter has at most this many periods left to run.
    ChapterEnding(u64),
    /// A storylet has come up before this time: an answer only a place
    /// that remembers the last time can give.
    RaisedBefore(&'static str),
    /// At least this many periods since the period an entity records
    /// under a key, or nothing recorded there: a new player's first days
    /// passed ("arrived" on the harbour, five periods).
    Since(EntityId, &'static str, u64),
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

/// The kind of entity a storylet puts on the scene.
pub const FIXTURE: &str = "fixture";

/// The fixtures standing now: what storylets have put on the scene.
pub fn fixtures(state: &WorldState) -> Vec<&Entity> {
    state
        .entities()
        .filter(|entity| entity.kind == FIXTURE)
        .collect()
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

/// A goal that has all its parts done, and the moment it was finished.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finished {
    pub goal: &'static str,
    /// The event that put its last part in place, and when; nothing when
    /// the World's history begins after it (a World opened from a
    /// checkpoint).
    pub event: Option<(EventId, u64)>,
}

/// The goals finished so far, in the order they were finished: those
/// whose finishing the history no longer holds first, in the deck's order.
/// Read from the history's index of the story's changes, not by reading
/// every event.
pub fn finished_goals(world: &World, deck: &Deck) -> Vec<Finished> {
    let state = world.state();
    let when = finishing(world, deck);
    let mut done = deck
        .goals
        .iter()
        .filter(|goal| progress(state, deck, goal.id) >= goal.parts)
        .map(|goal| Finished {
            goal: goal.id,
            event: when.get(goal.id).copied(),
        })
        .collect::<Vec<_>>();
    done.sort_by_key(|finished| finished.event.map(|(id, _)| id));
    done
}

/// What has been read of a World's history for [`finishing`], so a World
/// asked again a day later reads only the day's events.
struct Read {
    story: EntityId,
    goals: usize,
    first: Option<EventId>,
    /// How many events had been read, and the last of them.
    events: usize,
    last: Option<world_core::Event>,
    /// How many of the story's changes had been read.
    changes: usize,
    when: BTreeMap<&'static str, (EventId, u64)>,
}

thread_local! {
    static READ: std::cell::RefCell<Option<Read>> = const { std::cell::RefCell::new(None) };
}

/// The event that put each goal's last part in place, and when, from the
/// history's index of the story's changes. What was read before is kept
/// while the history it was read from is still the World's own.
fn finishing(world: &World, deck: &Deck) -> BTreeMap<&'static str, (EventId, u64)> {
    let events = world.events();
    let index = world.history_index();
    let changes = index.changes_of(deck.story);
    READ.with(|read| {
        let mut read = read.borrow_mut();
        let still_ours = read.as_ref().is_some_and(|read| {
            read.story == deck.story
                && read.goals == deck.goals.len()
                && read.first == events.first().map(|event| event.id)
                && read.events <= events.len()
                && read.changes <= changes.len()
                && read.last.as_ref() == read.events.checked_sub(1).map(|at| &events[at])
        });
        if !still_ours {
            *read = Some(Read {
                story: deck.story,
                goals: deck.goals.len(),
                first: events.first().map(|event| event.id),
                events: 0,
                last: None,
                changes: 0,
                when: BTreeMap::new(),
            });
        }
        let read = read.as_mut().expect("just made");
        let parts = deck
            .goals
            .iter()
            .map(|goal| (key("goal", goal.id), goal))
            .collect::<BTreeMap<_, _>>();
        for id in &changes[read.changes..] {
            let Some(event) = world.event(*id) else {
                continue;
            };
            for change in &event.changes {
                let StateChange::SetComponent {
                    entity,
                    key,
                    value: Value::Integer(done),
                } = change
                else {
                    continue;
                };
                if *entity != deck.story {
                    continue;
                }
                if let Some(goal) = parts.get(key) {
                    if *done >= goal.parts {
                        read.when
                            .entry(goal.id)
                            .or_insert((event.id, event.world_time));
                    }
                }
            }
        }
        read.changes = changes.len();
        read.events = events.len();
        read.last = events.last().cloned();
        read.when.clone()
    })
}

/// Whether a goal has all its parts done.
pub fn finished(state: &WorldState, deck: &Deck, goal: &str) -> bool {
    deck.goals
        .iter()
        .find(|spec| spec.id == goal)
        .is_some_and(|spec| progress(state, deck, goal) >= spec.parts)
}

/// Whether a condition holds now.
pub fn holds(state: &WorldState, deck: &Deck, condition: &Condition) -> bool {
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
        Condition::Marked(mark, periods) => integer(state, deck.story, &key("mark", mark))
            .is_some_and(|at| period >= (at.max(0) as u64).saturating_add(*periods)),
        Condition::Unmarked(mark) => integer(state, deck.story, &key("mark", mark)).is_none(),
        Condition::MarkedWithin(mark, periods) => integer(state, deck.story, &key("mark", mark))
            .is_some_and(|at| period <= (at.max(0) as u64).saturating_add(*periods)),
        Condition::Pressure(which) => pressure(state, deck).as_deref() == Some(*which),
        Condition::ChapterEnding(left) => period + left >= chapter_ends(state, deck),
        Condition::Present(entity) => state.entity(*entity).is_some(),
        Condition::Absent(entity) => state.entity(*entity).is_none(),
        Condition::RaisedBefore(id) => times_raised(state, deck, id) >= 2,
        Condition::Since(entity, key, periods) => integer(state, *entity, key)
            .is_none_or(|at| period >= (at.max(0) as u64).saturating_add(*periods)),
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
///
/// Only an open storylet has an opening time on the story, so this reads
/// those few rather than asking after every storylet in the deck.
pub fn open<'a>(state: &WorldState, deck: &'a Deck) -> Vec<&'a Storylet> {
    const OPEN: &str = "story.open.";
    let Some(story) = state.entity(deck.story) else {
        return Vec::new();
    };
    let mut open = story
        .components
        .range::<str, _>((std::ops::Bound::Included(OPEN), std::ops::Bound::Unbounded))
        .take_while(|(key, _)| key.starts_with(OPEN))
        .filter_map(|(key, value)| match value {
            Value::Integer(at) => Some((&key[OPEN.len()..], (*at).max(0) as u64)),
            _ => None,
        })
        .flat_map(|(id, at)| {
            deck.storylets
                .iter()
                .filter(move |storylet| storylet.id == id)
                .map(move |storylet| (at, storylet))
        })
        .collect::<Vec<_>>();
    open.sort_by_key(|(at, storylet)| (*at, storylet.id));
    open.into_iter().map(|(_, storylet)| storylet).collect()
}

/// [`open`] as it was first written, asking after every storylet in the
/// deck: the reference the faster reading is checked against.
#[cfg(test)]
fn open_by_asking<'a>(state: &WorldState, deck: &'a Deck) -> Vec<&'a Storylet> {
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

/// Every answer to every open storylet, with the conditions it does not
/// meet now: empty when it can be chosen.
pub fn answers<'a>(
    state: &WorldState,
    deck: &'a Deck,
) -> Vec<(&'a Storylet, &'a Choice, Vec<&'a Condition>)> {
    open(state, deck)
        .into_iter()
        .flat_map(|storylet| {
            storylet.choices.iter().map(move |choice| {
                let unmet = choice
                    .requires
                    .iter()
                    .filter(|condition| !holds(state, deck, condition))
                    .collect();
                (storylet, choice, unmet)
            })
        })
        .collect()
}

/// Whether a want, or anything someone else could see to, was let down
/// (turned down, or left to lapse, with nothing built) and has not been
/// taken up since: it does not come up again as it was.
pub fn let_down(state: &WorldState, deck: &Deck, storylet: &Storylet) -> bool {
    can_be_let_down(storylet) && integer(state, deck.story, &key("letdown", storylet.id)).is_some()
}

/// Whether a storylet is one that can be let down: a want, or something
/// someone else would see to.
fn can_be_let_down(storylet: &Storylet) -> bool {
    storylet.want || storylet.taken_up.is_some()
}

/// How many times someone else took a want up.
pub fn times_taken_up(state: &WorldState, deck: &Deck, id: &str) -> i64 {
    integer(state, deck.story, &key("taken", id)).unwrap_or(0)
}

/// Whether the last time a storylet ended it built a part of a goal (an
/// answer, a lapse that goes on anyway, or someone else taking it up):
/// only then does what builds toward it keep its own pace.
fn built_last(state: &WorldState, deck: &Deck, storylet: &Storylet) -> bool {
    let Some(last) = last_outcome(state, deck, storylet.id) else {
        return true;
    };
    let outcome = match last {
        "lapse" => Some(&storylet.lapse),
        TAKEN_UP => storylet.taken_up.as_ref().map(|taken| &taken.outcome),
        choice => storylet
            .choices
            .iter()
            .find(|known| known.id == choice)
            .map(|known| &known.outcome),
    };
    outcome.is_some_and(builds_a_part)
}

/// Whether an outcome builds a part of a goal.
fn builds_a_part(outcome: &Outcome) -> bool {
    outcome
        .effects
        .iter()
        .any(|effect| matches!(effect, Effect::Advance(_)))
}

/// The change that remembers a want was let down, and when: turned down
/// or left, with nothing built. A want that goes on its own way anyway (a
/// work built slower, without the player) was not let down: it comes
/// round again as its next part.
fn letting_down_change(
    state: &WorldState,
    deck: &Deck,
    storylet: &Storylet,
    outcome: &Outcome,
) -> Vec<StateChange> {
    if !can_be_let_down(storylet) || builds_a_part(outcome) {
        return Vec::new();
    }
    vec![StateChange::SetComponent {
        entity: deck.story,
        key: key("letdown", storylet.id),
        value: (state.world_time() as i64).into(),
    }]
}

/// Whether a storylet could come up now.
pub fn can_arise(state: &WorldState, deck: &Deck, storylet: &Storylet) -> bool {
    if opened_at(state, deck, storylet.id).is_some() || let_down(state, deck, storylet) {
        return false;
    }
    let rested = match integer(state, deck.story, &key("last", storylet.id)) {
        Some(last) => {
            state.world_time() >= (last.max(0) as u64).saturating_add(storylet.rests * deck.period)
        }
        None => true,
    };
    rested
        && rested_from_before(state, deck, storylet)
        && !asked_enough_this_year(state, deck, storylet.id)
        && all_hold(state, deck, &storylet.requires)
}

/// Whether a storylet that has come round before has rested as long as
/// coming round again asks ([`Deck::rarer`]).
///
/// Every storylet rests after it is told, however it ended. What last
/// ended by building a part of a goal not yet finished comes round part
/// by part, as it always has; everything else (let lapse or turned down
/// with nothing built, or answered some other way) grows rarer each time
/// it comes round.
fn rested_from_before(state: &WorldState, deck: &Deck, storylet: &Storylet) -> bool {
    let times = times_raised(state, deck, storylet.id).max(0) as u64;
    if deck.rarer == 0
        || times < 2
        || (built_last(state, deck, storylet) && advances_goal(state, deck, storylet).is_some())
    {
        return true;
    }
    let Some(last) = integer(state, deck.story, &key("last", storylet.id)) else {
        return true;
    };
    let rest = deck.rarer * (times - 1) * (times - 1);
    state.world_time() >= (last.max(0) as u64).saturating_add(rest * deck.period)
}

/// Whether a storylet could come up now if it did not have to rest first:
/// what the storyteller brings forward when nothing at all is open.
fn can_arise_early(state: &WorldState, deck: &Deck, storylet: &Storylet) -> bool {
    opened_at(state, deck, storylet.id).is_none()
        && !let_down(state, deck, storylet)
        && rested_from_before(state, deck, storylet)
        && !asked_enough_this_year(state, deck, storylet.id)
        && all_hold(state, deck, &storylet.requires)
}

/// The most times a storylet grown rare is brought forward regardless.
const RAREST: i64 = 4;

/// Whether a storylet could come up now if neither its rest nor how rare
/// it has grown held it back: what comes forward when nothing else can.
fn can_arise_rare(state: &WorldState, deck: &Deck, storylet: &Storylet) -> bool {
    // Never past its fourth time this way: rare stays rare.
    times_raised(state, deck, storylet.id) < RAREST
        && opened_at(state, deck, storylet.id).is_none()
        && !let_down(state, deck, storylet)
        && !asked_enough_this_year(state, deck, storylet.id)
        && all_hold(state, deck, &storylet.requires)
}

/// The most times one storylet comes up in any year of periods.
pub const MOST_A_YEAR: usize = 6;

/// Periods in the year the cap counts: a year of the player's days, not a
/// Pack's own calendar year. The cap is how often the player hears the
/// same question over a real year of play (a Pack's in-World year may be
/// much shorter), so it is the same for every Pack.
pub const YEAR_PERIODS: u64 = 365;

/// The periods a storylet last came up in, oldest first, at most
/// [`MOST_A_YEAR`] of them.
fn recently_raised(state: &WorldState, deck: &Deck, id: &str) -> Vec<u64> {
    match state
        .entity(deck.story)
        .and_then(|story| story.components.get(&key("recent", id)))
    {
        Some(Value::List(times)) => times
            .iter()
            .filter_map(|time| match time {
                Value::Integer(at) => Some((*at).max(0) as u64),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether a storylet has already come up [`MOST_A_YEAR`] times in the
/// year to now.
fn asked_enough_this_year(state: &WorldState, deck: &Deck, id: &str) -> bool {
    let recent = recently_raised(state, deck, id);
    recent.len() >= MOST_A_YEAR
        && recent
            .first()
            .is_some_and(|oldest| period_index(state, deck).saturating_sub(*oldest) < YEAR_PERIODS)
}

/// How many times a storylet has come up in this World.
pub fn times_raised(state: &WorldState, deck: &Deck, id: &str) -> i64 {
    integer(state, deck.story, &key("count", id)).unwrap_or(0)
}

/// Whether anything has ever come up in this World.
pub fn anything_raised(state: &WorldState, deck: &Deck) -> bool {
    state.entity(deck.story).is_some_and(|story| {
        story
            .components
            .keys()
            .any(|key| key.starts_with("story.count."))
    })
}

/// How a storylet ended the last time: the choice made, or `"lapse"` if
/// nobody chose in time.
pub fn last_outcome<'a>(state: &'a WorldState, deck: &Deck, id: &str) -> Option<&'a str> {
    text(state, deck.story, &key("outcome", id))
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

/// A chapter that has ended: its number, title and summary, and the Event
/// it ended at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ended {
    pub number: i64,
    pub title: String,
    pub summary: String,
    pub event: EventId,
}

/// Every chapter that has ended, oldest first.
pub fn chapters_ended(world: &World) -> Vec<Ended> {
    let text = |event: &world_core::Event, key: &str| match event.payload.get(key) {
        Some(Value::Text(text)) => text.clone(),
        _ => String::new(),
    };
    world
        .events()
        .iter()
        .filter(|event| event.kind == "chapter_ended")
        .map(|event| Ended {
            number: match event.payload.get("chapter") {
                Some(Value::Integer(number)) => *number,
                _ => 0,
            },
            title: text(event, "title"),
            summary: text(event, "summary"),
            event: event.id,
        })
        .collect()
}

/// The title the last chapter ended with, if one has.
pub fn last_chapter_title(world: &World) -> Option<String> {
    world
        .events()
        .iter()
        .rev()
        .find(|event| event.kind == "chapter_ended")
        .and_then(|event| match event.payload.get("title") {
            Some(Value::Text(title)) => Some(title.clone()),
            _ => None,
        })
}

/// The first of `candidates` that no chapter so far has been called, so no
/// two chapters of a World share a title. If every one has been used, the
/// first with a number that makes it new.
pub fn unused_title(world: &World, candidates: &[String]) -> String {
    let used = chapters_ended(world)
        .into_iter()
        .map(|ended| ended.title)
        .collect::<std::collections::BTreeSet<_>>();
    if let Some(title) = candidates.iter().find(|title| !used.contains(*title)) {
        return title.clone();
    }
    let first = candidates
        .first()
        .cloned()
        .unwrap_or_else(|| "A chapter".into());
    (2..)
        .map(|n| format!("{first} ({n})"))
        .find(|title| !used.contains(title))
        .unwrap_or(first)
}

/// A chapter's title told again with its year, for when the title alone
/// has been used: "A good summer in year nine", never "A good summer,
/// summer of year 9"; "The long dark, winter of year two".
pub fn title_with_year(title: &str, season: &str, year: u64) -> String {
    let year = number_word(year);
    if title.to_lowercase().contains(season) {
        format!("{title} in year {year}")
    } else {
        format!("{title}, {season} of year {year}")
    }
}

/// A chapter's title as it fits how long the chapter ran, `lived` of a
/// `year` of periods: a chapter of weeks is never "The year of" anything,
/// but "The summer of" it.
pub fn fitted_title(title: String, lived: u64, year: u64, season: &str) -> String {
    match title.strip_prefix("The year of ") {
        Some(rest) if lived < year * 3 / 4 => format!("The {season} of {rest}"),
        _ => title,
    }
}

/// A count as a chapter title writes it: "three", "eleven", then "13".
pub fn number_word(n: u64) -> String {
    const WORDS: [&str; 13] = [
        "nought", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        "eleven", "twelve",
    ];
    WORDS
        .get(n as usize)
        .map_or_else(|| n.to_string(), |word| (*word).to_string())
}

/// What this chapter is about, if the Pack gave its chapters pressures.
pub fn pressure(state: &WorldState, deck: &Deck) -> Option<String> {
    text(state, deck.story, "story.pressure").map(str::to_string)
}

fn pressure_for(deck: &Deck, chapter: i64) -> Option<&'static str> {
    (!deck.pressures.is_empty())
        .then(|| deck.pressures[((chapter - 1).max(0) as usize) % deck.pressures.len()])
}

/// How many periods apart a question that has never come up is let into a
/// free place.
const FIRST_TIME_GAP: i64 = 2;

/// How many periods a chapter runs on once a gauge at its end has brought
/// its end forward.
const HASTENED: u64 = 2;

/// The period this chapter ends at: its full length after it began,
/// unless a turning point brought that forward.
pub fn chapter_ends(state: &WorldState, deck: &Deck) -> u64 {
    integer(state, deck.story, "story.chapter_ends")
        .map(|ends| ends.max(0) as u64)
        .unwrap_or_else(|| {
            let (_, started) = chapter(state, deck);
            started / deck.period.max(1) + deck.chapter_periods.max(1)
        })
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
    // What these effects themselves put up, so building it again renews it.
    let mut built = std::collections::BTreeSet::<EntityId>::new();
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
            Effect::Toward {
                entity,
                key,
                target,
                by,
            } => {
                let current = *values
                    .entry((*entity, key.to_string()))
                    .or_insert_with(|| integer(state, *entity, key).unwrap_or(0));
                let next = if current < *target {
                    (current + by).min(*target)
                } else {
                    (current - by).max(*target)
                };
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
            Effect::Unset { entity, key } => {
                if state
                    .entity(*entity)
                    .is_some_and(|entity| entity.component(key).is_some())
                {
                    changes.push(StateChange::RemoveComponent {
                        entity: *entity,
                        key: key.to_string(),
                    });
                }
            }
            Effect::Mark(mark) => changes.push(StateChange::SetComponent {
                entity: deck.story,
                key: key("mark", mark),
                value: (period_index(state, deck) as i64).into(),
            }),
            Effect::Unmark(mark) => {
                if integer(state, deck.story, &key("mark", mark)).is_some() {
                    changes.push(StateChange::RemoveComponent {
                        entity: deck.story,
                        key: key("mark", mark),
                    });
                }
            }
            Effect::Build {
                entity,
                name,
                shape,
                at,
                lasts,
            } => {
                let until = lasts.map(|lasts| (period_index(state, deck) + lasts) as i64);
                if state.entity(*entity).is_some() || !built.insert(*entity) {
                    for (key, value) in [
                        ("name", Value::from(*name)),
                        ("shape", Value::from(*shape)),
                        ("at", Value::Entity(*at)),
                    ] {
                        changes.push(StateChange::SetComponent {
                            entity: *entity,
                            key: key.into(),
                            value,
                        });
                    }
                    match until {
                        Some(until) => changes.push(StateChange::SetComponent {
                            entity: *entity,
                            key: "until".into(),
                            value: until.into(),
                        }),
                        None => {
                            if integer(state, *entity, "until").is_some() {
                                changes.push(StateChange::RemoveComponent {
                                    entity: *entity,
                                    key: "until".into(),
                                });
                            }
                        }
                    }
                } else {
                    let mut fixture = Entity::new(*entity, FIXTURE)
                        .with_component("name", *name)
                        .with_component("shape", *shape)
                        .with_component("at", Value::Entity(*at));
                    if let Some(until) = until {
                        fixture = fixture.with_component("until", until);
                    }
                    changes.push(StateChange::CreateEntity(fixture));
                }
            }
            Effect::Demolish(entity) => {
                if state.entity(*entity).is_some() {
                    changes.push(StateChange::RemoveEntity(*entity));
                }
            }
            Effect::Arrive {
                entity,
                kind,
                components,
            } => {
                if state.entity(*entity).is_none() {
                    let mut arrival = Entity::new(*entity, *kind);
                    for (key, value) in components {
                        arrival = arrival.with_component(*key, value.clone());
                    }
                    changes.push(StateChange::CreateEntity(arrival));
                }
            }
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
struct Begins(DeckSource);

impl Action for Begins {
    fn name(&self) -> &'static str {
        "story_begins"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = self.0.get();
        if state.entity(deck.story).is_some() {
            return Err(ActionError::Invalid("the story has begun".into()));
        }
        let mut draft = EventDraft::new("story_began");
        let mut story = Entity::new(deck.story, "story")
            .with_component("name", deck.story_name)
            .with_component("story.chapter", 1_i64)
            .with_component("story.chapter_started", state.world_time() as i64);
        if let Some(pressure) = pressure_for(&deck, 1) {
            story = story.with_component("story.pressure", pressure);
        }
        draft.changes.push(StateChange::CreateEntity(story));
        Ok(draft)
    }
}

/// A fixture whose time is up is taken away.
struct FixturePasses(DeckSource);

impl Action for FixturePasses {
    fn name(&self) -> &'static str {
        "fixture_passes"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = self.0.get();
        let entity = match request.args.get("fixture") {
            Some(Value::Integer(id)) => EntityId::new(*id as u64),
            _ => return Err(ActionError::Invalid("missing fixture".into())),
        };
        let fixture = state
            .entity(entity)
            .filter(|entity| entity.kind == FIXTURE)
            .ok_or_else(|| ActionError::Invalid("no such fixture".into()))?;
        let until = integer(state, entity, "until")
            .ok_or_else(|| ActionError::Invalid("it stays for good".into()))?;
        if (period_index(state, &deck) as i64) < until {
            return Err(ActionError::Invalid("its time is not up".into()));
        }
        // Named in its payload, not targeted: once it is gone there is
        // nothing left to point at.
        let mut draft = EventDraft::new("fixture_passed");
        if let Some(Value::Text(name)) = fixture.component("name") {
            draft.payload.insert("name".into(), name.clone().into());
        }
        draft.changes.push(StateChange::RemoveEntity(entity));
        Ok(draft)
    }
}

/// A storylet comes up.
struct Arises(DeckSource);

impl Action for Arises {
    fn name(&self) -> &'static str {
        "storylet_arises"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = self.0.get();
        let storylet = find(&deck, arg_text(request, "storylet")?)?;
        // Brought forward before it has rested, when nothing else is open.
        let rare = request.args.get("rare") == Some(&Value::Bool(true));
        let early = request.args.get("early") == Some(&Value::Bool(true))
            && (can_arise_early(state, &deck, storylet)
                || (rare && can_arise_rare(state, &deck, storylet)));
        if !early
            && !can_arise(state, &deck, storylet)
            && !(rare && can_arise_rare(state, &deck, storylet))
        {
            return Err(ActionError::Invalid(format!(
                "{} cannot come up now",
                storylet.id
            )));
        }
        let mut draft = EventDraft::new("situation_arose");
        // Brought forward only because nothing else at all could come up.
        if rare && !early && !can_arise(state, &deck, storylet) {
            draft.payload.insert("rare".into(), true.into());
        }
        draft.actor = Some(storylet.asker);
        draft.targets = vec![storylet.asker];
        draft.payload.insert("storylet".into(), storylet.id.into());
        draft.payload.insert("want".into(), storylet.want.into());
        draft.changes.push(StateChange::SetComponent {
            entity: deck.story,
            key: key("open", storylet.id),
            value: (state.world_time() as i64).into(),
        });
        let times = times_raised(state, &deck, storylet.id) + 1;
        draft.changes.push(StateChange::SetComponent {
            entity: deck.story,
            key: key("count", storylet.id),
            value: times.into(),
        });
        draft.payload.insert("times".into(), times.into());
        let mut recent = recently_raised(state, &deck, storylet.id);
        recent.push(period_index(state, &deck));
        let skip = recent.len().saturating_sub(MOST_A_YEAR);
        draft.changes.push(StateChange::SetComponent {
            entity: deck.story,
            key: key("recent", storylet.id),
            value: Value::List(
                recent
                    .into_iter()
                    .skip(skip)
                    .map(|at| Value::Integer(at as i64))
                    .collect(),
            ),
        });
        if times == 1 {
            draft.changes.push(StateChange::SetComponent {
                entity: deck.story,
                key: "story.first_at".into(),
                value: (period_index(state, &deck) as i64).into(),
            });
        }
        if let Some(last) = last_outcome(state, &deck, storylet.id) {
            draft.payload.insert("last".into(), last.into());
        }
        Ok(draft)
    }
}

/// Someone answers an open storylet.
struct Chosen(DeckSource);

impl Action for Chosen {
    fn name(&self) -> &'static str {
        "storylet_chosen"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = self.0.get();
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
        draft.changes.push(StateChange::SetComponent {
            entity: deck.story,
            key: key("outcome", storylet.id),
            value: choice.id.into(),
        });
        if choice.refuses {
            draft
                .changes
                .extend(letting_down_change(state, &deck, storylet, &choice.outcome));
        } else if integer(state, deck.story, &key("letdown", storylet.id)).is_some() {
            draft.changes.push(StateChange::RemoveComponent {
                entity: deck.story,
                key: key("letdown", storylet.id),
            });
        }
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
struct Lapsed(DeckSource);

impl Action for Lapsed {
    fn name(&self) -> &'static str {
        "storylet_lapsed"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = self.0.get();
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
        draft.changes.push(StateChange::SetComponent {
            entity: deck.story,
            key: key("outcome", storylet.id),
            value: "lapse".into(),
        });
        draft
            .changes
            .extend(letting_down_change(state, &deck, storylet, &storylet.lapse));
        if storylet.want {
            draft.changes.push(grudge(state, &deck, storylet.asker));
        }
        Ok(draft)
    }
}

/// The storyteller marks something as happening now (see
/// [`Condition::MarkedWithin`]), unless it marked it within `every`
/// periods: a Pack notes what the player did, at most so often.
struct Marks(DeckSource);

impl Action for Marks {
    fn name(&self) -> &'static str {
        "story_marks"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = self.0.get();
        let mark = arg_text(request, "mark")?;
        let every = match request.args.get("every") {
            Some(Value::Integer(every)) => (*every).max(0) as u64,
            _ => 0,
        };
        if state.entity(deck.story).is_none() {
            return Err(ActionError::Invalid("the story has not begun".into()));
        }
        let now = period_index(state, &deck);
        if integer(state, deck.story, &key("mark", mark))
            .is_some_and(|at| now < (at.max(0) as u64).saturating_add(every))
        {
            return Err(ActionError::Invalid(format!("{mark} marked lately")));
        }
        let mut draft = EventDraft::new("story_marked");
        draft.payload.insert("mark".into(), mark.into());
        draft.changes.push(StateChange::SetComponent {
            entity: deck.story,
            key: key("mark", mark),
            value: (now as i64).into(),
        });
        Ok(draft)
    }
}

/// Marks `mark` as happening now, unless it was marked within `every`
/// periods: an Event only when it changes something.
pub fn mark_now(
    world: &mut World,
    actions: &ActionRegistry,
    mark: &str,
    every: u64,
) -> Result<Option<EventId>, WorldError> {
    let request = ActionRequest::new("story_marks")
        .arg("mark", mark)
        .arg("every", every as i64);
    match world.execute(actions, &request) {
        Ok(event) => Ok(Some(event.id)),
        Err(WorldError::Action(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Someone else takes up a want the player let down, once it has rested:
/// it is seen to their way, and it is no longer let down.
struct TakesUp(DeckSource);

impl Action for TakesUp {
    fn name(&self) -> &'static str {
        "storylet_taken_up"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = self.0.get();
        let storylet = find(&deck, arg_text(request, "storylet")?)?;
        if !can_take_up(state, &deck, storylet) {
            return Err(ActionError::Invalid(format!(
                "nobody can take {} up now",
                storylet.id
            )));
        }
        let taken = storylet.taken_up.as_ref().expect("checked");
        let mut draft = EventDraft::new(taken.outcome.event);
        draft.actor = Some(taken.by);
        draft.targets = vec![storylet.asker, taken.by];
        draft.payload.insert("storylet".into(), storylet.id.into());
        draft.payload.insert(TAKEN_UP.into(), true.into());
        draft.payload.insert("by".into(), Value::Entity(taken.by));
        draft.changes = applied(state, &deck, &taken.outcome.effects);
        draft.changes.extend([
            StateChange::RemoveComponent {
                entity: deck.story,
                key: key("letdown", storylet.id),
            },
            StateChange::SetComponent {
                entity: deck.story,
                key: key("last", storylet.id),
                value: (state.world_time() as i64).into(),
            },
            StateChange::SetComponent {
                entity: deck.story,
                key: key("outcome", storylet.id),
                value: TAKEN_UP.into(),
            },
            StateChange::SetComponent {
                entity: deck.story,
                key: key("taken", storylet.id),
                value: (times_taken_up(state, &deck, storylet.id) + 1).into(),
            },
            // It is part of both their lives: what the helper took on,
            // and who saw to what the asker wanted.
            StateChange::SetComponent {
                entity: taken.by,
                key: TOOK_UP.into(),
                value: storylet.id.into(),
            },
            StateChange::SetComponent {
                entity: storylet.asker,
                key: SEEN_TO_BY.into(),
                value: Value::Entity(taken.by),
            },
        ]);
        Ok(draft)
    }
}

/// Whether someone can take up a let-down want now: they and the asker are
/// both still there, and it has rested since it was let down.
fn can_take_up(state: &WorldState, deck: &Deck, storylet: &Storylet) -> bool {
    let Some(taken) = &storylet.taken_up else {
        return false;
    };
    let rested = integer(state, deck.story, &key("last", storylet.id)).is_none_or(|last| {
        state.world_time() >= (last.max(0) as u64).saturating_add(storylet.rests * deck.period)
    });
    let_down(state, deck, storylet)
        && opened_at(state, deck, storylet.id).is_none()
        && taken.by != storylet.asker
        && state.entity(taken.by).is_some()
        && state.entity(storylet.asker).is_some()
        && rested
        && all_hold(state, deck, &taken.requires)
}

/// The Event that let a want down last: its lapse, or an answer turning it
/// down.
fn letting_down(world: &World, storylet: &Storylet) -> Option<EventId> {
    let index = world.history_index();
    std::iter::once(storylet.lapse.event)
        .chain(
            storylet
                .choices
                .iter()
                .filter(|choice| choice.refuses)
                .map(|choice| choice.outcome.event),
        )
        .filter_map(|kind| {
            // The latest of each kind that was this storylet's.
            index.of_kind(kind).iter().rev().copied().find(|id| {
                world.event(*id).is_some_and(|event| {
                    event.payload.get("storylet") == Some(&Value::Text(storylet.id.into()))
                })
            })
        })
        .max()
}

/// Someone takes up a let-down want that has rested, if one has: at most
/// one a period, the longest let down first.
fn take_up(
    world: &mut World,
    actions: &ActionRegistry,
    deck: &Deck,
) -> Result<Option<EventId>, WorldError> {
    let state = world.state();
    let Some(storylet) = deck
        .storylets
        .iter()
        .filter(|storylet| can_take_up(state, deck, storylet))
        .min_by_key(|storylet| {
            (
                integer(state, deck.story, &key("letdown", storylet.id)).unwrap_or(0),
                storylet.id,
            )
        })
    else {
        return Ok(None);
    };
    let taken = storylet.taken_up.as_ref().expect("can be taken up");
    let mut request = ActionRequest::new("storylet_taken_up")
        .actor(taken.by)
        .arg("storylet", storylet.id);
    if let Some(cause) = letting_down(world, storylet) {
        request = request.caused_by(cause);
    }
    Ok(Some(world.execute(actions, &request)?.id))
}

/// A chapter ends, told in the Pack's words, and the next begins.
struct ChapterTurns(DeckSource);

impl Action for ChapterTurns {
    fn name(&self) -> &'static str {
        "chapter_turns"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = self.0.get();
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
        if integer(state, deck.story, "story.chapter_ends").is_some() {
            draft.changes.push(StateChange::RemoveComponent {
                entity: deck.story,
                key: "story.chapter_ends".into(),
            });
        }
        if let Some(pressure) = pressure_for(&deck, number + 1) {
            draft.changes.push(StateChange::SetComponent {
                entity: deck.story,
                key: "story.pressure".into(),
                value: pressure.into(),
            });
        }
        draft
            .changes
            .extend(applied(state, &deck, &deck.fresh_start));
        Ok(draft)
    }
}

/// A turning point brings the chapter's end forward.
struct ChapterHastens(DeckSource);

impl Action for ChapterHastens {
    fn name(&self) -> &'static str {
        "chapter_hastens"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let deck = self.0.get();
        let ends = period_index(state, &deck) + HASTENED;
        if chapter_ends(state, &deck) <= ends {
            return Err(ActionError::Invalid("the chapter is already ending".into()));
        }
        let mut draft = EventDraft::new("chapter_turning");
        draft.changes.push(StateChange::SetComponent {
            entity: deck.story,
            key: "story.chapter_ends".into(),
            value: (ends as i64).into(),
        });
        Ok(draft)
    }
}

/// Where the storyteller's Actions find the Pack's deck: made afresh each
/// time, or one the Pack keeps for the life of the program.
#[derive(Clone, Copy)]
enum DeckSource {
    Made(fn() -> Deck),
    Kept(fn() -> &'static Deck),
}

impl DeckSource {
    fn get(self) -> std::borrow::Cow<'static, Deck> {
        match self {
            DeckSource::Made(deck) => std::borrow::Cow::Owned(deck()),
            DeckSource::Kept(deck) => std::borrow::Cow::Borrowed(deck()),
        }
    }
}

fn register(registry: &mut ActionRegistry, deck: DeckSource) -> Result<(), ActionError> {
    registry.register(Begins(deck))?;
    registry.register(FixturePasses(deck))?;
    registry.register(Arises(deck))?;
    registry.register(Chosen(deck))?;
    registry.register(Lapsed(deck))?;
    registry.register(TakesUp(deck))?;
    registry.register(Marks(deck))?;
    registry.register(ChapterTurns(deck))?;
    registry.register(ChapterHastens(deck))?;
    Ok(())
}

/// Registers the storyteller's Actions for a Pack's deck.
pub fn register_actions(
    registry: &mut ActionRegistry,
    deck: fn() -> Deck,
) -> Result<(), ActionError> {
    register(registry, DeckSource::Made(deck))
}

/// Registers the storyteller's Actions for a deck the Pack keeps for the
/// life of the program, so no Action copies the deck to read it.
pub fn register_kept_actions(
    registry: &mut ActionRegistry,
    deck: fn() -> &'static Deck,
) -> Result<(), ActionError> {
    register(registry, DeckSource::Kept(deck))
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
    /// Whether the player is away. Then wants wait for them rather than
    /// lapsing, and nothing new comes up but what the calendar brings and
    /// what cannot wait.
    pub away: bool,
    /// Whether a gauge has reached its very end. That is a turning point:
    /// once the chapter has run its shortest, it ends there.
    pub at_end: bool,
    /// The chapter's ending, in the Pack's words: a title and a sentence.
    pub chapter_ending: Box<ChapterEnding>,
    /// Whether nothing new should come up yet: a new World waits for the
    /// player to do something of their own before anyone asks them
    /// anything.
    pub hold: bool,
}

/// The fewest periods between two storylets let in by the goals' lane.
const GOAL_LANE_GAP: u64 = 6;

/// Whether any of a storylet's choices builds toward a goal.
fn builds(storylet: &Storylet) -> bool {
    storylet
        .choices
        .iter()
        .flat_map(|choice| choice.outcome.effects.iter())
        .any(|effect| matches!(effect, Effect::Advance(_)))
}

/// Which goal a storylet builds toward, if any of its choices advances one
/// not yet finished: its place in the deck's goals, so the deck's order
/// decides which comes first.
pub fn advances_goal(state: &WorldState, deck: &Deck, storylet: &Storylet) -> Option<usize> {
    storylet
        .choices
        .iter()
        .flat_map(|choice| choice.outcome.effects.iter())
        .filter_map(|effect| match effect {
            Effect::Advance(goal) => deck.goals.iter().position(|spec| spec.id == *goal),
            _ => None,
        })
        .filter(|at| !finished(state, deck, deck.goals[*at].id))
        .min()
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
    let period = period_index(world.state(), deck) as i64;
    let passed = fixtures(world.state())
        .into_iter()
        .filter(|fixture| {
            integer(world.state(), fixture.id, "until").is_some_and(|until| until <= period)
        })
        .map(|fixture| fixture.id)
        .collect::<Vec<_>>();
    for fixture in passed {
        let request = ActionRequest::new("fixture_passes").arg("fixture", fixture.0 as i64);
        events.push(world.execute(actions, &request)?.id);
    }
    let now = world.world_time();
    for storylet in open(world.state(), deck) {
        let at = opened_at(world.state(), deck, storylet.id).unwrap_or(now);
        if reading.away && storylet.want && !storylet.timely {
            continue;
        }
        if now >= at.saturating_add(storylet.lasts.max(1) * deck.period) {
            let request = ActionRequest::new("storylet_lapsed")
                .actor(storylet.asker)
                .arg("storylet", storylet.id);
            events.push(world.execute(actions, &request)?.id);
        }
    }
    // A want let down and rested is seen to by someone else.
    if !reading.hold {
        events.extend(take_up(world, actions, deck)?);
    }
    let (_, started) = chapter(world.state(), deck);
    let ends = chapter_ends(world.state(), deck);
    let period_now = period_index(world.state(), deck);
    let old_enough = now >= started.saturating_add(deck.shortest_chapter.max(1) * deck.period);
    // A gauge at its end is a turning point: the chapter's end comes
    // forward, near enough for its climax to come up first.
    if reading.at_end && old_enough && ends > period_now + HASTENED {
        events.push(
            world
                .execute(actions, &ActionRequest::new("chapter_hastens"))?
                .id,
        );
    }
    if period_now >= chapter_ends(world.state(), deck) {
        let (title, summary) = (reading.chapter_ending)(world);
        let request = ActionRequest::new("chapter_turns")
            .arg("title", title)
            .arg("summary", summary);
        events.push(world.execute(actions, &request)?.id);
    }
    let period = period_index(world.state(), deck);
    loop {
        if reading.hold {
            break;
        }
        let state = world.state();
        let open_now = open(state, deck);
        if open_now.len() >= deck.most_open {
            break;
        }
        let easing = open_now
            .iter()
            .any(|storylet| eases_pinned(storylet, &reading.pinned));
        let wants_open = open_now.iter().any(|storylet| storylet.want);
        // Goals have a lane of their own: while nothing building toward
        // one is open, whatever advances an unfinished goal is let in,
        // whatever wants are open, so the World's works keep moving.
        let goal_open = open_now
            .iter()
            .any(|storylet| advances_goal(state, deck, storylet).is_some());
        // The lane is a way through for a goal that has been waiting, not a
        // queue: once something for a goal has come up, the rest of the
        // deck has its turn for a few periods before the lane opens again.
        let goal_lately = deck
            .storylets
            .iter()
            .filter(|storylet| builds(storylet))
            .filter_map(|storylet| recently_raised(state, deck, storylet.id).last().copied())
            .max()
            .is_some_and(|last| period < last.saturating_add(GOAL_LANE_GAP));
        let goal_lane = |storylet: &Storylet| {
            if goal_open || goal_lately {
                None
            } else {
                advances_goal(state, deck, storylet)
            }
        };
        // A first-time question is let in at most every few periods, so
        // the rest of the deck is heard without crowding the World's own
        // story.
        let last_first = integer(state, deck.story, "story.first_at").unwrap_or(0);
        let settled = period >= deck.chapter_periods
            && period as i64 >= last_first.saturating_add(FIRST_TIME_GAP);
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
            // The earlier a goal stands in the deck, the sooner it comes.
            if let Some(at) = goal_lane(storylet) {
                score += 50 + deck.goals.len().saturating_sub(at) as u64;
            }
            // What has never come up is favoured, so the whole deck is
            // heard, not only its heaviest few.
            if times_raised(state, deck, storylet.id) == 0 {
                score += 40;
            }
            score * 1_000 + mix(&[period, text_hash(storylet.id)]) % 997
        };
        // Past the first, only what cannot wait, what the World needs,
        // someone's want when nobody has one open, the next part of a goal
        // when nothing is building toward one, or what has never come up.
        let needed = |storylet: &Storylet| {
            if reading.away && !open_now.is_empty() {
                return storylet.timely;
            }
            open_now.is_empty()
                || storylet.timely
                || (!easing && eases_pinned(storylet, &reading.pinned))
                || (storylet.want && !wants_open)
                || goal_lane(storylet).is_some()
                // Once the first chapter has told the World's own
                // story, what has never come up may take a free place, so
                // nothing in the deck waits forever behind the rest.
                || (settled && times_raised(state, deck, storylet.id) == 0)
        };
        let pick = deck
            .storylets
            .iter()
            .filter(|storylet| can_arise(state, deck, storylet) && needed(storylet))
            .max_by_key(|storylet| (score(storylet), storylet.id));
        // With nothing open and everything grown rare, the least often
        // heard of what has rested, rather than nothing to decide.
        let rare = pick.is_none() && open_now.is_empty() && !reading.away;
        let pick = pick.or_else(|| {
            deck.storylets
                .iter()
                .filter(|_| rare)
                // What has never come up keeps its own pace, above.
                .filter(|storylet| times_raised(state, deck, storylet.id) > 0)
                .filter(|storylet| {
                    can_arise_rare(state, deck, storylet)
                        && integer(state, deck.story, &key("last", storylet.id)).is_none_or(
                            |last| {
                                state.world_time()
                                    >= (last.max(0) as u64)
                                        .saturating_add(storylet.rests * deck.period)
                            },
                        )
                })
                .min_by_key(|storylet| (times_raised(state, deck, storylet.id), storylet.id))
        });
        let Some(pick) = pick else {
            break;
        };
        let request = ActionRequest::new("storylet_arises")
            .actor(pick.asker)
            .arg("storylet", pick.id)
            .arg("rare", rare);
        events.push(world.execute(actions, &request)?.id);
    }
    Ok(events)
}

/// Something is always open: with nothing open and everything resting,
/// whatever has rested longest comes forward. Goals keep their own pace,
/// so nothing that builds toward one is brought forward. A Pack that wants
/// a question on the table every period calls this after [`tick`].
pub fn bring_forward(
    world: &mut World,
    actions: &ActionRegistry,
    deck: &Deck,
    reading: &Reading,
) -> Result<Option<EventId>, WorldError> {
    if reading.hold || reading.away || !open(world.state(), deck).is_empty() {
        return Ok(None);
    }
    let state = world.state();
    let rested = |storylet: &Storylet| {
        integer(state, deck.story, &key("last", storylet.id)).unwrap_or(i64::MIN)
    };
    let pick = deck
        .storylets
        .iter()
        .filter(|storylet| !builds(storylet))
        .filter(|storylet| can_arise_early(state, deck, storylet))
        .min_by_key(|storylet| (rested(storylet), storylet.id));
    let Some(pick) = pick else {
        return Ok(None);
    };
    let request = ActionRequest::new("storylet_arises")
        .actor(pick.asker)
        .arg("storylet", pick.id)
        .arg("early", true);
    Ok(Some(world.execute(actions, &request)?.id))
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

/// Puts something on the scene ([`Effect::Build`]).
pub fn build(
    entity: EntityId,
    name: &'static str,
    shape: &'static str,
    at: EntityId,
    lasts: Option<u64>,
) -> Effect {
    Effect::Build {
        entity,
        name,
        shape,
        at,
        lasts,
    }
}

/// Remembers that something happened ([`Effect::Mark`]).
pub fn mark(name: &'static str) -> Effect {
    Effect::Mark(name)
}

/// The chapters of a World's story that have ended, as the book shows
/// them.
pub fn chapters_shown(world: &World) -> Vec<world_projection::Chapter> {
    chapters_ended(world)
        .into_iter()
        .map(|ended| world_projection::Chapter {
            number: ended.number.max(0) as u32,
            title: ended.title,
            summary: ended.summary,
            moment: Some(world_projection::SelectionId::Event(ended.event)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A title told again names its year without telling its season
    /// twice, and a chapter of weeks is never "the year of" anything.
    #[test]
    fn chapter_titles_read_whole_and_fit_their_length() {
        assert_eq!(
            title_with_year("A good summer", "summer", 9),
            "A good summer in year nine"
        );
        assert_eq!(
            title_with_year("The long dark", "winter", 2),
            "The long dark, winter of year two"
        );
        assert_eq!(
            title_with_year("A quiet spring", "spring", 14),
            "A quiet spring in year 14"
        );
        assert_eq!(
            fitted_title("The year of the great storm".into(), 11, 120, "autumn"),
            "The autumn of the great storm"
        );
        assert_eq!(
            fitted_title("The year of the great storm".into(), 118, 120, "autumn"),
            "The year of the great storm"
        );
        assert_eq!(
            fitted_title("A bright summer".into(), 11, 120, "summer"),
            "A bright summer"
        );
        for title in [
            title_with_year("A bright autumn", "autumn", 3),
            title_with_year("A good winter", "winter", 6),
        ] {
            assert!(
                world_projection::seams_in(&title).is_empty(),
                "{title}: {:?}",
                world_projection::seams_in(&title)
            );
        }
    }
    use world_core::{Entity, WorldState};

    const STORY: EntityId = EntityId::new(1);
    const ANN: EntityId = EntityId::new(2);
    const BO: EntityId = EntityId::new(3);

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
                    // Let down, Ann's roof is seen to by Bo: a deck this
                    // small would otherwise run dry once it is dropped.
                    taken_up: Some(TakenUp {
                        by: BO,
                        outcome: Outcome {
                            event: "bo_mended_the_roof",
                            effects: vec![Effect::Advance("house")],
                        },
                        requires: Vec::new(),
                    }),
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
                    taken_up: None,
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
                    taken_up: None,
                },
            ],
            goals: vec![Goal {
                id: "house",
                parts: 2,
            }],
            chapter_periods: 4,
            shortest_chapter: 2,
            pressures: vec!["drought", "flood"],
            fresh_start: vec![Effect::Put {
                entity: ANN,
                key: "coins",
                to: 10,
            }],
            most_open: 2,
            rarer: 0,
        }
    }

    fn world() -> (World, ActionRegistry) {
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(ANN, "person").with_component("coins", 10_i64))
            .unwrap();
        state.seed_entity(Entity::new(BO, "person")).unwrap();
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
            away: false,
            at_end: false,
            chapter_ending: Box::new(|_| ("An end".into(), "It went well.".into())),
            hold: false,
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
    fn with_nothing_open_something_comes_forward_but_never_a_goal() {
        let (mut world, actions) = world();
        let deck = deck();
        tick(&mut world, &actions, &deck, &reading()).unwrap();
        // With something open, nothing is brought forward.
        assert_eq!(
            bring_forward(&mut world, &actions, &deck, &reading()).unwrap(),
            None
        );
        world
            .execute(&actions, &choose_request("market", "sell"))
            .unwrap();
        world
            .execute(&actions, &choose_request("roof", "mend"))
            .unwrap();
        assert!(open(world.state(), &deck).is_empty());
        for _ in 0..3 {
            assert!(bring_forward(&mut world, &actions, &deck, &reading())
                .unwrap()
                .is_some());
            let open_now = open(world.state(), &deck)
                .iter()
                .map(|storylet| storylet.id)
                .collect::<Vec<_>>();
            assert_eq!(open_now.len(), 1);
            assert!(!open_now.contains(&"roof"), "the house keeps its pace");
            let choice = if open_now[0] == "chat" {
                "listen"
            } else {
                "sell"
            };
            world
                .execute(&actions, &choose_request(open_now[0], choice))
                .unwrap();
        }
        // Held for the player, nothing comes.
        let held = Reading {
            hold: true,
            ..reading()
        };
        assert_eq!(
            bring_forward(&mut world, &actions, &deck, &held).unwrap(),
            None
        );
    }

    #[test]
    fn what_is_open_is_what_asking_after_every_storylet_finds() {
        let (mut world, actions) = world();
        let deck = deck();
        for period in 0..40 {
            let (theirs, ours) = (
                open_by_asking(world.state(), &deck),
                open(world.state(), &deck),
            );
            assert_eq!(theirs.len(), ours.len(), "period {period}");
            assert!(
                theirs.iter().zip(&ours).all(|(a, b)| std::ptr::eq(*a, *b)),
                "period {period}"
            );
            if period % 3 == 1 {
                let _ = world.execute(&actions, &choose_request("market", "sell"));
            }
            pass(&mut world, &actions);
        }
    }

    /// When a goal was finished is read from the history, a day at a time
    /// as the World goes on, and afresh for a World whose history went
    /// another way.
    #[test]
    fn when_a_goal_was_finished_is_read_from_the_world_s_own_history() {
        let (mut world, actions) = world();
        let mut mended = 0;
        let mut forks = Vec::new();
        for _ in 0..40 {
            if open(world.state(), &deck())
                .iter()
                .any(|storylet| storylet.id == "roof")
                && world
                    .execute(&actions, &choose_request("roof", "mend"))
                    .is_ok()
            {
                mended += 1;
                forks.push(world.events().len());
            }
            assert_eq!(finished_goals(&world, &deck()).is_empty(), mended < 2);
            pass(&mut world, &actions);
        }
        assert!(mended >= 2, "{mended}");
        let done = finished_goals(&world, &deck());
        let (event, at) = done[0].event.unwrap();
        let event = world.event(event).unwrap();
        assert_eq!(event.kind, "roof_mended");
        assert_eq!(event.world_time, at);
        // Before the second part, nothing is finished; after it, the same.
        let before = world.fork_after(forks[1] - 1).unwrap();
        assert!(finished_goals(&before, &deck()).is_empty());
        let after = world.fork_after(forks[1]).unwrap();
        assert_eq!(finished_goals(&after, &deck()), done);
        assert_eq!(finished_goals(&world, &deck()), done);
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
    fn a_gauge_at_its_end_brings_the_chapter_to_a_close_and_the_next_starts_fresh() {
        fn long() -> Deck {
            Deck {
                chapter_periods: 20,
                ..deck()
            }
        }
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(ANN, "person").with_component("coins", 10_i64))
            .unwrap();
        state.seed_entity(Entity::new(BO, "person")).unwrap();
        let mut actions = ActionRegistry::new();
        register_actions(&mut actions, long).unwrap();
        let mut world = World::new(state);
        world
            .execute(&actions, &ActionRequest::new("story_begins"))
            .unwrap();
        let at_end = Reading {
            pinned: Vec::new(),
            away: false,
            at_end: true,
            chapter_ending: Box::new(|_| ("Broke".into(), "Ann ran out.".into())),
            hold: false,
        };
        world.advance_to(&actions, 10).unwrap();
        tick(&mut world, &actions, &long(), &at_end).unwrap();
        assert_eq!(chapter_ends(world.state(), &long()), 20, "too soon to turn");
        world.advance_to(&actions, 20).unwrap();
        tick(&mut world, &actions, &long(), &at_end).unwrap();
        assert_eq!(
            chapter_ends(world.state(), &long()),
            4,
            "the end comes forward"
        );
        assert!(holds(world.state(), &long(), &Condition::ChapterEnding(2)));
        for step in 3..=4 {
            world.advance_to(&actions, step * 10).unwrap();
            tick(&mut world, &actions, &long(), &at_end).unwrap();
        }
        assert_eq!(chapter(world.state(), &long()).0, 2);
        assert_eq!(integer(world.state(), ANN, "coins"), Some(10));
        assert_eq!(chapter_ends(world.state(), &long()), 24);
    }

    #[test]
    fn a_pinned_gauge_brings_what_eases_it() {
        let (mut world, actions) = world();
        let reading = Reading {
            pinned: vec![Pinned {
                gauge: "coins",
                high: true,
            }],
            away: false,
            at_end: false,
            chapter_ending: Box::new(|_| (String::new(), String::new())),
            hold: false,
        };
        // Off market day, with nothing open: the want that spends comes up.
        world.advance_to(&actions, 10).unwrap();
        tick(&mut world, &actions, &deck(), &reading).unwrap();
        assert!(opened_at(world.state(), &deck(), "roof").is_some());
    }

    #[test]
    fn what_is_built_stands_on_the_scene_and_what_passes_goes() {
        let (mut world, actions) = world();
        let stall = EntityId::new(50);
        let bunting = EntityId::new(51);
        let deck = deck();
        let changes = applied(
            world.state(),
            &deck,
            &[
                Effect::Build {
                    entity: stall,
                    name: "Ann's stall",
                    shape: "stall",
                    at: ANN,
                    lasts: None,
                },
                Effect::Build {
                    entity: bunting,
                    name: "Bunting",
                    shape: "bunting",
                    at: ANN,
                    lasts: Some(2),
                },
                Effect::Mark("stall"),
            ],
        );
        assert_eq!(changes.len(), 3);
        struct Apply(Vec<StateChange>);
        impl Action for Apply {
            fn name(&self) -> &'static str {
                "apply_for_test"
            }
            fn evaluate(
                &self,
                _state: &WorldState,
                _request: &ActionRequest,
            ) -> Result<EventDraft, ActionError> {
                let mut draft = EventDraft::new("built");
                draft.changes = self.0.clone();
                Ok(draft)
            }
        }
        let mut actions = actions;
        actions.register(Apply(changes)).unwrap();
        world
            .execute(&actions, &ActionRequest::new("apply_for_test"))
            .unwrap();
        assert_eq!(fixtures(world.state()).len(), 2);
        assert!(!holds(world.state(), &deck, &Condition::Marked("stall", 1)));
        assert!(holds(world.state(), &deck, &Condition::Marked("stall", 0)));
        pass(&mut world, &actions);
        assert!(holds(world.state(), &deck, &Condition::Marked("stall", 1)));
        pass(&mut world, &actions);
        let standing = fixtures(world.state())
            .iter()
            .map(|fixture| fixture.id)
            .collect::<Vec<_>>();
        assert_eq!(
            standing,
            vec![stall],
            "the bunting came down after two periods"
        );
        assert!(kinds(&world).contains(&"fixture_passed".to_string()));
    }

    #[test]
    fn each_chapter_has_its_pressure_and_its_end_in_sight() {
        let (mut world, actions) = world();
        assert_eq!(pressure(world.state(), &deck()).as_deref(), Some("drought"));
        assert!(!holds(world.state(), &deck(), &Condition::ChapterEnding(1)));
        for _ in 0..3 {
            pass(&mut world, &actions);
        }
        assert!(holds(world.state(), &deck(), &Condition::ChapterEnding(1)));
        assert!(holds(
            world.state(),
            &deck(),
            &Condition::Pressure("drought")
        ));
        pass(&mut world, &actions);
        pass(&mut world, &actions);
        assert_eq!(chapter(world.state(), &deck()).0, 2);
        assert_eq!(pressure(world.state(), &deck()).as_deref(), Some("flood"));
    }

    #[test]
    fn while_the_player_is_away_wants_wait_for_them() {
        let (mut world, actions) = world();
        let away = Reading {
            pinned: Vec::new(),
            away: true,
            at_end: false,
            chapter_ending: Box::new(|_| (String::new(), String::new())),
            hold: false,
        };
        world
            .execute(
                &actions,
                &ActionRequest::new("storylet_arises").arg("storylet", "roof"),
            )
            .unwrap();
        for step in 1..8 {
            world.advance_to(&actions, step * 10).unwrap();
            tick(&mut world, &actions, &deck(), &away).unwrap();
        }
        assert!(
            opened_at(world.state(), &deck(), "roof").is_some(),
            "Ann's want is still waiting"
        );
        assert!(!kinds(&world).contains(&"roof_leaked".to_string()));
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
    fn it_remembers_how_often_each_came_up_and_how_it_ended() {
        let (mut world, actions) = world();
        tick(&mut world, &actions, &deck(), &reading()).unwrap();
        assert_eq!(times_raised(world.state(), &deck(), "roof"), 1);
        world
            .execute(&actions, &choose_request("roof", "mend"))
            .unwrap();
        assert_eq!(last_outcome(world.state(), &deck(), "roof"), Some("mend"));
        // Market day runs out unanswered.
        pass(&mut world, &actions);
        assert_eq!(
            last_outcome(world.state(), &deck(), "market"),
            Some("lapse")
        );
        for _ in 0..8 {
            pass(&mut world, &actions);
        }
        assert!(times_raised(world.state(), &deck(), "market") >= 2);
    }

    #[test]
    fn what_came_round_before_grows_rarer_and_remembers() {
        // Market day, in a deck that forgets and in one that remembers.
        let market_days = |rarer: u64| {
            let mut state = WorldState::default();
            state
                .seed_entity(Entity::new(ANN, "person").with_component("coins", 10_i64))
                .unwrap();
            state.seed_entity(Entity::new(BO, "person")).unwrap();
            let mut actions = ActionRegistry::new();
            let source: fn() -> Deck = if rarer == 0 { deck } else { rare_deck };
            register_actions(&mut actions, source).unwrap();
            let mut world = World::new(state);
            world
                .execute(&actions, &ActionRequest::new("story_begins"))
                .unwrap();
            let mut came = Vec::new();
            for day in 0..120 {
                let next = world.world_time() + 10;
                world.advance_to(&actions, next).unwrap();
                for id in tick(&mut world, &actions, &source(), &reading()).unwrap() {
                    let event = world.event(id).unwrap();
                    // What came up only because nothing else at all could
                    // (at most RAREST times) is not what this counts: with
                    // a want let down now dropped until someone takes it
                    // up, this tiny deck runs dry for a while.
                    if event.kind == "situation_arose"
                        && event.payload.get("storylet") == Some(&Value::Text("market".into()))
                        && event.payload.get("rare") != Some(&Value::Bool(true))
                    {
                        came.push(day);
                    }
                }
            }
            (world, came)
        };
        let (_, forgets) = market_days(0);
        let (world, remembers) = market_days(6);
        // It comes round every third day in a deck that forgets; in one
        // that remembers, the third time waits at least its rest, and it
        // comes only when nothing else could (this deck is tiny).
        assert!(
            remembers[2] - remembers[1] >= 6,
            "{remembers:?} {forgets:?}"
        );
        assert!(
            remembers.last() > forgets.last(),
            "{remembers:?} {forgets:?}"
        );
        assert_eq!(remembers[..2], forgets[..2]);
        // Whether it came before is something an answer can ask.
        assert!(holds(
            world.state(),
            &rare_deck(),
            &Condition::RaisedBefore("market")
        ));
        assert!(!holds(
            world.state(),
            &rare_deck(),
            &Condition::RaisedBefore("roof_never")
        ));
    }

    fn rare_deck() -> Deck {
        Deck { rarer: 6, ..deck() }
    }

    #[test]
    fn what_has_never_come_up_comes_first() {
        let (mut world, actions) = world();
        // Off market day with the roof done and resting, "chat" is the only
        // one never heard, and is picked over nothing else being needed.
        world.advance_to(&actions, 10).unwrap();
        tick(&mut world, &actions, &deck(), &reading()).unwrap();
        let first = open(world.state(), &deck())[0].id;
        assert_eq!(times_raised(world.state(), &deck(), first), 1);
    }

    #[test]
    fn nothing_comes_up_while_held() {
        let (mut world, actions) = world();
        let held = Reading {
            hold: true,
            ..reading()
        };
        tick(&mut world, &actions, &deck(), &held).unwrap();
        assert!(open(world.state(), &deck()).is_empty());
        tick(&mut world, &actions, &deck(), &reading()).unwrap();
        assert!(!open(world.state(), &deck()).is_empty());
    }

    /// Ann's year with a long visit on her mind as well: a want that
    /// builds nothing and stays open a long while.
    fn busy_deck() -> Deck {
        let mut deck = deck();
        deck.storylets.push(Storylet {
            id: "visit",
            asker: ANN,
            want: true,
            requires: Vec::new(),
            choices: vec![Choice {
                id: "go",
                requires: Vec::new(),
                refuses: false,
                outcome: Outcome {
                    event: "visited",
                    effects: Vec::new(),
                },
            }],
            lapse: Outcome {
                event: "visit_put_off",
                effects: Vec::new(),
            },
            lasts: 30,
            rests: 0,
            weight: 9,
            eases: Vec::new(),
            timely: false,
            taken_up: None,
        });
        deck.most_open = 3;
        deck
    }

    #[test]
    fn a_goal_keeps_moving_whatever_wants_are_open() {
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(ANN, "person").with_component("coins", 20_i64))
            .unwrap();
        let mut actions = ActionRegistry::new();
        register_actions(&mut actions, busy_deck).unwrap();
        let mut world = World::new(state);
        let deck = busy_deck();
        world
            .execute(&actions, &ActionRequest::new("story_begins"))
            .unwrap();
        // The roof has come up once and been mended.
        let arises = ActionRequest::new("storylet_arises")
            .actor(ANN)
            .arg("storylet", "roof");
        world.execute(&actions, &arises).unwrap();
        world
            .execute(&actions, &choose_request("roof", "mend"))
            .unwrap();
        tick(&mut world, &actions, &deck, &reading()).unwrap();
        let ids = |world: &World| {
            open(world.state(), &deck)
                .iter()
                .map(|storylet| storylet.id)
                .collect::<Vec<_>>()
        };
        assert!(ids(&world).contains(&"visit"), "{:?}", ids(&world));
        // The visit stays open, a want nobody has answered, and still the
        // roof comes up again once it has rested.
        let mut asked_again = false;
        for _ in 0..(GOAL_LANE_GAP + 2) {
            let next = world.world_time() + 10;
            world.advance_to(&actions, next).unwrap();
            tick(&mut world, &actions, &deck, &reading()).unwrap();
            let now = ids(&world);
            assert!(now.contains(&"visit"), "{now:?}");
            asked_again |= now.contains(&"roof");
        }
        assert!(asked_again, "the roof waited behind the visit");
        assert!(advances_goal(world.state(), &deck, &deck.storylets[0]).is_some());
        assert!(advances_goal(world.state(), &deck, &deck.storylets[3]).is_none());
    }

    /// Ann's year, where Bo sees to the roof when nobody else will.
    fn helped_deck() -> Deck {
        deck()
    }

    fn world_of(source: fn() -> Deck) -> (World, ActionRegistry) {
        let mut state = WorldState::default();
        for (who, coins) in [(ANN, 10_i64), (BO, 0)] {
            state
                .seed_entity(Entity::new(who, "person").with_component("coins", coins))
                .unwrap();
        }
        let mut actions = ActionRegistry::new();
        register_actions(&mut actions, source).unwrap();
        let mut world = World::new(state);
        world
            .execute(&actions, &ActionRequest::new("story_begins"))
            .unwrap();
        (world, actions)
    }

    fn roofs_asked(world: &World) -> usize {
        world
            .events_of_kind(&["situation_arose"])
            .iter()
            .filter(|event| event.payload.get("storylet") == Some(&Value::Text("roof".into())))
            .count()
    }

    #[test]
    fn a_want_let_down_is_taken_up_by_someone_else_and_never_returns_as_it_was() {
        let (mut world, actions) = world_of(helped_deck);
        let deck = helped_deck();
        world
            .execute(
                &actions,
                &ActionRequest::new("storylet_arises").arg("storylet", "roof"),
            )
            .unwrap();
        let mut taken = None;
        for step in 1..12 {
            world.advance_to(&actions, step * 10).unwrap();
            for id in tick(&mut world, &actions, &deck, &reading()).unwrap() {
                let event = world.event(id).unwrap();
                if event.kind == "bo_mended_the_roof" {
                    taken = Some(event.clone());
                }
            }
            if taken.is_none() && step > 2 {
                assert!(let_down(world.state(), &deck, &deck.storylets[0]));
            }
            if taken.is_some() {
                break;
            }
        }
        let taken = taken.expect("Bo took the roof up");
        // It was never asked again as it was, before Bo saw to it.
        assert_eq!(roofs_asked(&world), 1);
        let leaked = world.events_of_kind(&["roof_leaked"])[0].id;
        assert_eq!(taken.caused_by, vec![leaked]);
        assert_eq!(taken.actor, Some(BO));
        assert_eq!(taken.targets, vec![ANN, BO]);
        let state = world.state();
        assert_eq!(progress(state, &deck, "house"), 1);
        assert!(!let_down(state, &deck, &deck.storylets[0]));
        assert_eq!(last_outcome(state, &deck, "roof"), Some(TAKEN_UP));
        assert_eq!(times_taken_up(state, &deck, "roof"), 1);
        // It is in both their histories.
        let index = world.history_index();
        assert!(index.changes_of(BO).contains(&taken.id));
        assert!(index.changes_of(ANN).contains(&taken.id));
        assert_eq!(world.replay().unwrap().state(), world.state());
    }

    #[test]
    fn a_want_turned_down_with_nobody_to_take_it_up_is_dropped() {
        let mut refusing = deck();
        refusing.storylets[0].taken_up = None;
        refusing.storylets[0].choices.push(Choice {
            id: "no",
            requires: Vec::new(),
            refuses: true,
            outcome: Outcome {
                event: "roof_left",
                effects: Vec::new(),
            },
        });
        fn source() -> Deck {
            let mut deck = deck();
            deck.storylets[0].taken_up = None;
            deck.storylets[0].choices.push(Choice {
                id: "no",
                requires: Vec::new(),
                refuses: true,
                outcome: Outcome {
                    event: "roof_left",
                    effects: Vec::new(),
                },
            });
            deck
        }
        let (mut world, actions) = world_of(source);
        world
            .execute(
                &actions,
                &ActionRequest::new("storylet_arises").arg("storylet", "roof"),
            )
            .unwrap();
        world
            .execute(&actions, &choose_request("roof", "no"))
            .unwrap();
        for step in 1..40 {
            world.advance_to(&actions, step * 10).unwrap();
            tick(&mut world, &actions, &refusing, &reading()).unwrap();
            bring_forward(&mut world, &actions, &refusing, &reading()).unwrap();
        }
        assert_eq!(roofs_asked(&world), 1, "the roof never came up again");
        assert!(let_down(world.state(), &refusing, &refusing.storylets[0]));
    }

    /// Bo takes the roof up only once Ann has lately sold at market.
    fn waiting_deck() -> Deck {
        let mut deck = deck();
        if let Some(taken) = deck.storylets[0].taken_up.as_mut() {
            taken.requires = vec![Condition::MarkedWithin("sold", 2)];
        }
        deck.storylets[1].choices[0]
            .outcome
            .effects
            .push(Effect::Mark("sold"));
        deck
    }

    #[test]
    fn a_want_let_down_waits_to_be_taken_up_until_what_it_needs_holds() {
        let (mut world, actions) = world_of(waiting_deck);
        let deck = waiting_deck();
        world
            .execute(
                &actions,
                &ActionRequest::new("storylet_arises").arg("storylet", "roof"),
            )
            .unwrap();
        let taken = |world: &World| !world.events_of_kind(&["bo_mended_the_roof"]).is_empty();
        for step in 1..10 {
            world.advance_to(&actions, step * 10).unwrap();
            tick(&mut world, &actions, &deck, &reading()).unwrap();
        }
        assert!(!taken(&world), "nobody sold, so Bo waits");
        assert!(let_down(world.state(), &deck, &deck.storylets[0]));
        let mut step = 10;
        while !taken(&world) && step < 30 {
            if opened_at(world.state(), &deck, "market").is_some() {
                world
                    .execute(&actions, &choose_request("market", "sell"))
                    .unwrap();
            }
            world.advance_to(&actions, step * 10).unwrap();
            tick(&mut world, &actions, &deck, &reading()).unwrap();
            step += 1;
        }
        assert!(taken(&world), "once Ann sold, Bo took the roof up");
        assert!(holds(
            world.state(),
            &deck,
            &Condition::MarkedWithin("sold", 30)
        ));
    }

    /// Ann's year with a beam for the house that nobody puts up unless
    /// asked: left, it builds nothing.
    fn beam_deck() -> Deck {
        let mut deck = Deck { rarer: 6, ..deck() };
        deck.storylets[0]
            .requires
            .push(Condition::Marked("never", 0));
        deck.storylets.push(Storylet {
            id: "beam",
            asker: ANN,
            want: false,
            requires: vec![Condition::Unfinished("house")],
            choices: vec![Choice {
                id: "raise",
                requires: Vec::new(),
                refuses: false,
                outcome: Outcome {
                    event: "beam_raised",
                    effects: vec![Effect::Advance("house")],
                },
            }],
            lapse: Outcome {
                event: "beam_left",
                effects: Vec::new(),
            },
            lasts: 1,
            rests: 1,
            weight: 5,
            eases: Vec::new(),
            timely: false,
            taken_up: None,
        });
        deck
    }

    #[test]
    fn what_builds_toward_a_goal_but_is_left_grows_rarer_like_everything_else() {
        let (mut world, actions) = world_of(beam_deck);
        let deck = beam_deck();
        let beams = |world: &World| {
            world
                .events_of_kind(&["situation_arose"])
                .iter()
                .filter(|event| event.payload.get("storylet") == Some(&Value::Text("beam".into())))
                .map(|event| (event.world_time / 10, event.payload.contains_key("rare")))
                .collect::<Vec<_>>()
        };
        for step in 1..60 {
            world.advance_to(&actions, step * 10).unwrap();
            tick(&mut world, &actions, &deck, &reading()).unwrap();
        }
        let left = beams(&world);
        // Never raised: the second time follows the first, and after that
        // it waits its rest, coming sooner only when nothing else at all
        // can (at most RAREST times in all). At the house's own pace it
        // would have come every few periods.
        assert!(left.len() >= 2 && left.len() <= RAREST as usize, "{left:?}");
        assert!(left.iter().skip(2).all(|(_, rare)| *rare), "{left:?}");
        // Raised each time it comes, it keeps the house's own pace.
        let (mut built, actions) = world_of(beam_deck);
        let mut raised = 0;
        for step in 1..8 {
            if opened_at(built.state(), &deck, "beam").is_some()
                && built
                    .execute(&actions, &choose_request("beam", "raise"))
                    .is_ok()
            {
                raised += 1;
            }
            built.advance_to(&actions, step * 10).unwrap();
            tick(&mut built, &actions, &deck, &reading()).unwrap();
        }
        assert_eq!(raised, 2, "{:?}", beams(&built));
        assert!(finished(built.state(), &deck, "house"));
    }

    /// Ann's roof mends itself slowly when nobody answers.
    fn slow_roof_deck() -> Deck {
        let mut deck = Deck { rarer: 6, ..deck() };
        deck.storylets[0]
            .lapse
            .effects
            .push(Effect::Advance("house"));
        deck
    }

    #[test]
    fn a_want_that_goes_on_its_own_way_is_not_let_down() {
        let (mut world, actions) = world_of(slow_roof_deck);
        let deck = slow_roof_deck();
        for step in 1..30 {
            world.advance_to(&actions, step * 10).unwrap();
            tick(&mut world, &actions, &deck, &reading()).unwrap();
            assert!(!let_down(world.state(), &deck, &deck.storylets[0]));
        }
        // Left twice, it built both parts at its own pace, and nobody had
        // to take it up.
        assert!(finished(world.state(), &deck, "house"));
        assert!(world.events_of_kind(&["bo_mended_the_roof"]).is_empty());
        assert_eq!(world.events_of_kind(&["roof_leaked"]).len(), 2);
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
