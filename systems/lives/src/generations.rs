//! Generations: people are born, grow up, grow old and, in time, die.
//!
//! Everyone has an age that goes up with the Pack's calendar year. Someone
//! the World has no birth recorded for (everyone a World began with, and
//! anyone who arrived before this System kept ages) is given an age from
//! who they are and the World's start, so an older World opens with the
//! same ages every time.
//!
//! A couple may have a child: the odds rise with how fond they are of
//! each other, how well they are and how the place feels, and nothing
//! forces it. The child takes a trait from each parent, lives at a
//! parent's home, and on coming of age takes up a trade, joins the
//! place's life and may move to a home of their own. The old retire and,
//! in time, die gently and off-screen: the closest of the people they
//! leave inherits something of theirs, a bench or a stone goes up where
//! the player puts it (or where the place does, if the player does not),
//! and those who were close remember them in their days for a season and
//! on the day each year.
//!
//! Every one of these is an ordinary Action recorded as an Event, so a
//! replay never rolls the dice again. The words are the Pack's
//! ([`KinWords`]); this System knows nothing of harbours or penguins.

use crate::{
    arg_entity, cast_ids, entity_of, fill_owned, first_name, gone, integer, lack, mix, opinion,
    partner, period, pick, text, traits, Candidate, Cast, Moves, Need, GONE, PARTNER,
};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft, EventId,
    StateChange, Value, World, WorldError, WorldState,
};

/// The period someone was born in (it may be before the World began).
pub const BORN: &str = "lives.born";
/// A born child's parents.
pub const PARENTS: &str = "lives.parents";
/// A trait from each parent, kept until the child joins the place's life.
pub const NATURE: &str = "lives.nature";
/// Someone who came of age while the World has kept ages.
pub const OF_AGE: &str = "lives.of_age";
/// Someone who has left their parents' home for one of their own.
pub const OWN_HOME: &str = "lives.own_home";
/// The period someone died in.
pub const DIED: &str = "lives.died";
/// What someone inherited, in words.
pub const HEIRLOOM: &str = "lives.heirloom";
/// Who inherited from someone who died.
pub const HEIR: &str = "lives.heir";
/// The memorial put up for someone.
pub const MEMORIAL: &str = "lives.memorial";
/// Who someone's memorial is for.
pub const MEMORIAL_OF: &str = "lives.memorial_of";
/// Someone who has retired once, and does not again.
const RETIRED: &str = "lives.retired";

fn retired(state: &WorldState, person: EntityId) -> bool {
    matches!(
        state.entity(person).and_then(|e| e.component(RETIRED)),
        Some(Value::Bool(true))
    )
}

/// When a couple last had a child, on each of them.
const LAST_CHILD: &str = "lives.last_child";
/// The last anniversary kept for someone, as the year since they died.
const ANNIVERSARY: &str = "lives.anniversary";
/// Where along the ground something the player put somewhere stands,
/// 0 to 100: the same key a Pack's hands use.
pub const SPOT: &str = "spot";

/// How far through life someone is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Baby,
    Child,
    Teen,
    Adult,
    Elder,
}

/// The words a Pack tells its lives' turns in. Every template names its
/// slots: `{name}` is whose turn it is (their first name), `{a}` and `{b}`
/// a newborn's parents, `{trade}` a trade in words, `{age}` a number,
/// `{heir}` and `{heirloom}` who inherits what, `{memorial}` the
/// memorial's name. A line is picked from each list by chance.
#[derive(Debug)]
pub struct KinWords {
    pub born: &'static [&'static str],
    /// Said by a parent.
    pub born_said: &'static [&'static str],
    pub came_of_age: &'static [&'static str],
    pub came_of_age_said: &'static [&'static str],
    pub left_home: &'static [&'static str],
    pub left_home_said: &'static [&'static str],
    pub retired: &'static [&'static str],
    pub retired_said: &'static [&'static str],
    pub died: &'static [&'static str],
    /// Said by the closest of those left.
    pub died_said: &'static [&'static str],
    pub heirloom: &'static [&'static str],
    pub heirloom_said: &'static [&'static str],
    /// The memorial as the player put it where they chose, and as the
    /// place put it up when the player did not.
    pub memorial_by_player: &'static [&'static str],
    pub memorial_by_town: &'static [&'static str],
    pub memorial_said: &'static [&'static str],
    pub anniversary: &'static [&'static str],
    pub anniversary_said: &'static [&'static str],
    /// What someone close says of the one they lost, in the season after
    /// and on the day each year.
    pub remember: &'static [&'static str],
}

/// A memorial's kind: its shape on the scene (the Pack's own word) and
/// what it is called, with `{name}` in it.
#[derive(Clone, Copy, Debug)]
pub struct Memorial {
    pub shape: &'static str,
    pub named: &'static str,
}

/// Everything the System needs to know to keep a place's generations.
#[derive(Debug)]
pub struct Kin {
    /// Periods in the Pack's calendar year: an age goes up once a year.
    pub year: u64,
    /// How old someone was when the World began, if the Pack says; anyone
    /// else is given an age from who they are.
    pub age_at_start: fn(&WorldState, EntityId) -> Option<u64>,
    /// The period someone was born in, when the Pack knows it from what it
    /// kept before ages were (a chick hatched in an older World's thaw).
    pub born_at: fn(&WorldState, EntityId) -> Option<i64>,
    /// The youngest a newcomer's own age may be, and how many years above
    /// it one may be, for anyone the Pack does not name.
    pub youngest: u64,
    pub spread: u64,
    /// The ages a baby becomes a child, a child a teen, a teen grown, and a
    /// grown-up an elder.
    pub child_at: u64,
    pub teen_at: u64,
    pub grown_at: u64,
    pub elder_at: u64,
    /// When hair goes grey, and backs stoop.
    pub grey_at: u64,
    pub stoop_at: u64,
    /// The oldest a parent-to-be may be.
    pub fertile_until: u64,
    /// When people retire, and from when they may die of old age.
    pub retire_at: u64,
    pub frail_at: u64,
    /// The first id a newborn takes, and how many can ever be born.
    pub first_child: u64,
    pub room: u64,
    /// Names for newborns, taken in turn by whoever is not named yet.
    pub names: &'static [&'static str],
    /// The kind of entity a newborn is, and what it starts with besides
    /// its name.
    pub kind: &'static str,
    pub newborn: fn(&WorldState, EntityId, EntityId) -> Vec<(String, Value)>,
    /// Where a job is kept, and the job words the System sets: someone
    /// retired, and someone still learning (whose trade a coming of age
    /// replaces).
    pub job_key: &'static str,
    pub retired_job: &'static str,
    pub learning: &'static [&'static str],
    /// Trades a coming of age may take up: in words, and as the job.
    pub trades: &'static [(&'static str, &'static str)],
    /// Someone whose old age is the Pack's own story: they grow old, but
    /// never retire or die by this System's hand.
    pub keeps: fn(EntityId) -> bool,
    /// Whether anyone may be born now (a colony that hatches its chicks
    /// only in the thaw), and whether someone is in the middle of
    /// something (asking the player a question) and so not to be taken.
    pub births_now: fn(&WorldState) -> bool,
    pub busy: fn(&WorldState, EntityId) -> bool,
    /// Chances, in ten-thousandths a period: of a well-matched couple in
    /// good heart in a bright place having a child, and of someone a year
    /// past `frail_at` dying (more each year after).
    pub birth_odds: u64,
    pub frailty: u64,
    /// The fewest years between a couple's children, and the most
    /// children a couple has.
    pub apart: u64,
    pub most_children: usize,
    /// Heirlooms, in words ("a pocket watch").
    pub heirlooms: &'static [&'static str],
    /// Memorials, the first chosen when the player places one.
    pub memorials: &'static [Memorial],
    /// The first id a memorial takes, and where the place puts one up.
    pub first_memorial: u64,
    pub memorial_at: EntityId,
    /// How many periods the player has to place a memorial before the
    /// place puts one up.
    pub memorial_wait: u64,
    /// How many periods those close remember someone gone in their days.
    pub mourning: u64,
    pub words: &'static KinWords,
}

impl Kin {
    /// The block of ids newborns take: [`Self::first_child`] and the
    /// [`Self::room`] after it.
    pub const fn children(&self) -> world_core::IdBlock {
        world_core::IdBlock::new(self.first_child, self.room)
    }

    /// A trade in words, from its job.
    pub fn trade_words(&self, job: &str) -> String {
        self.trades
            .iter()
            .find(|(_, id)| *id == job)
            .map(|(told, _)| told.to_string())
            .unwrap_or_else(|| job.replace('_', " "))
    }
}

fn kin_of(cast: &Cast) -> Result<&'static Kin, ActionError> {
    cast.kin
        .ok_or_else(|| ActionError::Invalid("this place keeps no ages".into()))
}

/// The period someone was born in: as recorded, or from who they are
/// and the World's start.
pub fn born_period(state: &WorldState, kin: &Kin, person: EntityId) -> i64 {
    if let Some(born) = integer(state, person, BORN).or_else(|| (kin.born_at)(state, person)) {
        return born;
    }
    let age = (kin.age_at_start)(state, person)
        .unwrap_or_else(|| kin.youngest + mix(&[person.0, 41]) % kin.spread.max(1));
    let year = kin.year.max(1);
    -(age as i64 * year as i64) - (mix(&[person.0, 29]) % year) as i64
}

/// Someone's age in the Pack's years.
pub fn age_of(state: &WorldState, cast: &Cast, kin: &Kin, person: EntityId) -> u64 {
    let now = period(state, cast) as i64;
    ((now - born_period(state, kin, person)).max(0) / kin.year.max(1) as i64) as u64
}

/// How far through life someone of `age` is.
pub fn stage_at(kin: &Kin, age: u64) -> Stage {
    if age < kin.child_at {
        Stage::Baby
    } else if age < kin.teen_at {
        Stage::Child
    } else if age < kin.grown_at {
        Stage::Teen
    } else if age < kin.elder_at {
        Stage::Adult
    } else {
        Stage::Elder
    }
}

/// How someone looks their age: their stage, whether their hair is grey,
/// and whether they stoop.
pub fn looks_of(state: &WorldState, cast: &Cast, person: EntityId) -> Option<(Stage, bool, bool)> {
    let kin = cast.kin?;
    state.entity(person)?;
    let age = age_of(state, cast, kin, person);
    Some((stage_at(kin, age), age >= kin.grey_at, age >= kin.stoop_at))
}

/// Whether someone is grown: always, in a place that keeps no ages.
pub fn grown(state: &WorldState, cast: &Cast, person: EntityId) -> bool {
    match cast.kin {
        Some(kin) => age_of(state, cast, kin, person) >= kin.grown_at,
        None => true,
    }
}

/// The most years apart two people may be and still walk out together.
const COURTING_GAP: u64 = 15;

/// Whether two people may walk out together: both grown, and of an age.
pub fn may_court(state: &WorldState, cast: &Cast, a: EntityId, b: EntityId) -> bool {
    match cast.kin {
        Some(kin) => {
            let (x, y) = (age_of(state, cast, kin, a), age_of(state, cast, kin, b));
            let family = |x: EntityId, y: EntityId| {
                let (px, py) = (parents(state, x), parents(state, y));
                px.contains(&y) || py.contains(&x) || px.iter().any(|p| py.contains(p))
            };
            x >= kin.grown_at && y >= kin.grown_at && x.abs_diff(y) <= COURTING_GAP && !family(a, b)
        }
        None => true,
    }
}

/// A child's parents.
pub fn parents(state: &WorldState, person: EntityId) -> Vec<EntityId> {
    match state
        .entity(person)
        .and_then(|entity| entity.component(PARENTS))
    {
        Some(Value::List(parents)) => parents
            .iter()
            .filter_map(|value| match value {
                Value::Entity(id) => Some(*id),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Everyone born here, living or not, first born first.
pub fn born_here(state: &WorldState, cast: &Cast) -> Vec<EntityId> {
    let Some(kin) = cast.kin else {
        return Vec::new();
    };
    (kin.first_child..kin.first_child + kin.room)
        .map(EntityId::new)
        .take_while(|id| state.entity(*id).is_some())
        .collect()
}

/// The children born here who are living here and not yet grown: they
/// live with a parent and are not yet part of the place's everyday life.
pub fn children(state: &WorldState, cast: &Cast) -> Vec<EntityId> {
    born_here(state, cast)
        .into_iter()
        .filter(|id| !gone(state, *id) && !of_age(state, *id))
        .collect()
}

/// Those born here who have come of age and live here still: part of the
/// place's life like anyone.
pub fn grown_here(state: &WorldState, cast: &Cast) -> Vec<EntityId> {
    born_here(state, cast)
        .into_iter()
        .filter(|id| !gone(state, *id) && of_age(state, *id))
        .collect()
}

/// Whether someone came of age while the World kept ages.
pub fn of_age(state: &WorldState, person: EntityId) -> bool {
    matches!(
        state.entity(person).and_then(|e| e.component(OF_AGE)),
        Some(Value::Bool(true))
    )
}

/// The parent someone still lives with, if they do: a child, or someone
/// grown who has not yet moved out and has nobody of their own.
pub fn lives_with(state: &WorldState, person: EntityId) -> Option<EntityId> {
    if matches!(
        state.entity(person).and_then(|e| e.component(OWN_HOME)),
        Some(Value::Bool(true))
    ) {
        return None;
    }
    parents(state, person)
        .into_iter()
        .find(|parent| state.entity(*parent).is_some() && !gone(state, *parent))
}

/// Whether someone has died.
pub fn died(state: &WorldState, person: EntityId) -> Option<i64> {
    integer(state, person, DIED)
}

/// Everyone who has died here, and the memorial put up for each, if one
/// has been.
pub fn departed(state: &WorldState) -> Vec<(EntityId, Option<EntityId>)> {
    state
        .entities()
        .filter(|entity| entity.component(DIED).is_some())
        .map(|entity| (entity.id, entity_of(state, entity.id, MEMORIAL)))
        .collect()
}

/// Those who have died whose memorial the player may still place: every
/// one the place has not yet put one up for.
pub fn awaiting_memorial(state: &WorldState) -> Vec<EntityId> {
    departed(state)
        .into_iter()
        .filter(|(_, memorial)| memorial.is_none())
        .map(|(who, _)| who)
        .collect()
}

/// What a memorial for someone would be called.
pub fn memorial_name(state: &WorldState, kin: &Kin, who: EntityId, shape: usize) -> String {
    let memorial = kin.memorials[shape.min(kin.memorials.len().saturating_sub(1))];
    fill_owned(memorial.named, &[("name", first_name(state, who))])
}

/// The request that places a memorial for someone: by the player, at a
/// spot along the ground of their choosing (0 to 100), or by the place.
pub fn memorial_request(who: EntityId, by_player: bool, spot: Option<u8>) -> ActionRequest {
    let mut request = ActionRequest::new("lives_memorial_placed")
        .arg("who", Value::Entity(who))
        .arg("by", if by_player { "player" } else { "town" });
    if let Some(spot) = spot {
        request = request.arg("spot", i64::from(spot.min(100)));
    }
    request
}

/// Someone's children, living or not.
pub fn children_of(state: &WorldState, cast: &Cast, person: EntityId) -> Vec<EntityId> {
    born_here(state, cast)
        .into_iter()
        .filter(|child| parents(state, *child).contains(&person))
        .collect()
}

/// The one closest to someone: their partner, else a child of theirs,
/// else whoever thinks most of them.
pub fn closest(state: &WorldState, cast: &Cast, person: EntityId) -> Option<EntityId> {
    let alive = |id: &EntityId| state.entity(*id).is_some() && !gone(state, *id) && *id != person;
    if let Some(partner) = partner(state, person).filter(alive) {
        return Some(partner);
    }
    if let Some(child) = children_of(state, cast, person)
        .into_iter()
        .filter(alive)
        .find(|child| of_age(state, *child))
    {
        return Some(child);
    }
    cast_ids(state)
        .into_iter()
        .filter(alive)
        .max_by_key(|other| (opinion(state, *other, person), mix(&[other.0, person.0])))
}

/// Whether someone gone died lately enough that those close still speak
/// of them, or this is the day a year on: whom `person` remembers now.
pub fn mourned_by(state: &WorldState, cast: &Cast, person: EntityId) -> Option<EntityId> {
    let kin = cast.kin?;
    let now = period(state, cast) as i64;
    departed(state)
        .into_iter()
        .map(|(who, _)| who)
        .filter(|who| {
            let Some(at) = died(state, *who) else {
                return false;
            };
            let since = now - at;
            let lately = (0..kin.mourning as i64).contains(&since);
            let the_day = since > 0 && since % kin.year.max(1) as i64 == 0;
            (lately || the_day)
                && (partner_before(state, *who) == Some(person)
                    || entity_of(state, *who, HEIR) == Some(person)
                    || parents(state, person).contains(who)
                    || parents(state, *who).contains(&person)
                    || opinion(state, person, *who) >= 30)
        })
        .max_by_key(|who| opinion(state, person, *who))
}

/// Whom someone who died was with, as it was: kept on them when they die.
fn partner_before(state: &WorldState, person: EntityId) -> Option<EntityId> {
    entity_of(state, person, "lives.was_with")
}

/// A line in memory of someone, as someone close says it now.
pub(crate) fn remembering(
    state: &WorldState,
    cast: &Cast,
    person: EntityId,
    seed: u64,
) -> Option<(EntityId, Vec<String>)> {
    let kin = cast.kin?;
    let who = mourned_by(state, cast, person)?;
    // Not every day: most days are still just days.
    if !seed.is_multiple_of(3) {
        return None;
    }
    let lines = kin
        .words
        .remember
        .iter()
        .map(|line| fill_owned(line, &[("name", first_name(state, who))]))
        .collect();
    Some((who, lines))
}

fn words(lines: &'static [&'static str], seed: u64, slots: &[(&'static str, String)]) -> String {
    pick(lines, seed)
        .map(|line| fill_owned(line, slots))
        .unwrap_or_default()
}

fn job<'a>(state: &'a WorldState, kin: &Kin, person: EntityId) -> Option<&'a str> {
    text(state, person, kin.job_key)
}

fn set(entity: EntityId, key: &str, value: impl Into<Value>) -> StateChange {
    StateChange::SetComponent {
        entity,
        key: key.into(),
        value: value.into(),
    }
}

fn draft(kind: &str, actor: EntityId, who: EntityId, told: String, said: String) -> EventDraft {
    let mut draft = EventDraft::new(kind);
    draft.actor = Some(actor);
    draft.targets = vec![who];
    draft.payload.insert("who".into(), Value::Entity(who));
    draft.payload.insert("told".into(), told.into());
    if !said.is_empty() {
        draft.payload.insert("said".into(), said.into());
    }
    draft
}

// ---- Births ---------------------------------------------------------------

/// Whether two people could have a child now, and the odds of it, in
/// ten-thousandths.
pub fn birth_odds(state: &WorldState, cast: &Cast, a: EntityId, b: EntityId) -> Option<u64> {
    let kin = cast.kin?;
    let now = period(state, cast) as i64;
    if partner(state, a) != Some(b) || gone(state, a) || gone(state, b) {
        return None;
    }
    for person in [a, b] {
        let age = age_of(state, cast, kin, person);
        if age < kin.grown_at || age > kin.fertile_until {
            return None;
        }
        if integer(state, person, LAST_CHILD)
            .is_some_and(|at| now - at < (kin.apart * kin.year) as i64)
        {
            return None;
        }
    }
    let theirs = children_of(state, cast, a)
        .into_iter()
        .filter(|child| parents(state, *child).contains(&b))
        .count();
    if theirs >= kin.most_children {
        return None;
    }
    // How fond they are of each other, how they are, how the place feels.
    let fond = ((opinion(state, a, b) + opinion(state, b, a)) / 2 - 20).clamp(0, 80) as u64;
    let lacking = Need::ALL
        .iter()
        .map(|need| lack(state, a, *need) + lack(state, b, *need))
        .sum::<i64>()
        / 8;
    let well = (100 - lacking).clamp(10, 100) as u64;
    let mood = ((cast.mood)(state).clamp(-10, 10) + 12) as u64;
    Some(kin.birth_odds * fond * well * mood / (40 * 70 * 12) / (1 + theirs as u64))
}

/// A couple has a child.
struct Born(fn(&WorldState) -> Cast);

impl Action for Born {
    fn name(&self) -> &'static str {
        "lives_born"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let kin = kin_of(&cast)?;
        let a = arg_entity(request, "a")?;
        let b = arg_entity(request, "b")?;
        if birth_odds(state, &cast, a, b).is_none() {
            return Err(ActionError::Invalid("no child for them now".into()));
        }
        // The lowest free id among the children's, from the state alone.
        let child = state
            .free_id_in(kin.children())
            .map_err(|error| ActionError::Invalid(format!("no room for another: {error}")))?;
        let taken = state
            .entities()
            .map(|entity| first_name(state, entity.id))
            .collect::<std::collections::BTreeSet<_>>();
        let start = (mix(&[child.0, a.0, b.0]) % kin.names.len().max(1) as u64) as usize;
        let child_name = (0..kin.names.len())
            .map(|at| kin.names[(start + at) % kin.names.len()])
            .find(|name| !taken.contains(*name))
            .ok_or_else(|| ActionError::Invalid("no name left".into()))?;
        let now = period(state, &cast) as i64;
        // A trait from each parent.
        let seed = mix(&[child.0, now as u64, 71]);
        let from = |parent: EntityId, salt: u64| {
            let theirs = traits(state, parent);
            pick(&theirs, seed / salt).cloned()
        };
        let mut nature = [from(a, 3), from(b, 7)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        nature.dedup();
        let mut entity = Entity::new(child, kin.kind)
            .with_component("name", child_name)
            .with_component(BORN, now)
            .with_component(
                PARENTS,
                Value::List(vec![Value::Entity(a), Value::Entity(b)]),
            )
            .with_component(NATURE, nature.join(" "));
        for (key, value) in (kin.newborn)(state, a, b) {
            entity = entity.with_component(key, value);
        }
        let (an, bn) = (first_name(state, a), first_name(state, b));
        let slots = [
            ("name", child_name.to_string()),
            ("a", an.clone()),
            ("b", bn.clone()),
        ];
        let told = words(kin.words.born, seed, &slots);
        let said = words(kin.words.born_said, seed / 5, &slots);
        let mut draft = draft("born", a, child, told, said);
        draft.targets.push(b);
        draft.payload.insert("name".into(), child_name.into());
        draft.payload.insert(
            "parents".into(),
            Value::List(vec![Value::Entity(a), Value::Entity(b)]),
        );
        draft.changes.push(StateChange::CreateEntity(entity));
        let mut moves = Moves::default();
        for parent in [a, b] {
            moves.set(parent, LAST_CHILD, now);
            moves.lack(state, parent, Need::Purpose, -40);
            moves.lack(state, parent, Need::Rest, 25);
        }
        draft.changes.extend(moves.changes);
        Ok(draft)
    }
}

// ---- Growing up -----------------------------------------------------------

/// Someone comes of age and takes up a trade.
struct ComesOfAge(fn(&WorldState) -> Cast);

impl Action for ComesOfAge {
    fn name(&self) -> &'static str {
        "lives_came_of_age"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let kin = kin_of(&cast)?;
        let who = arg_entity(request, "who")?;
        if state.entity(who).is_none() || gone(state, who) || of_age(state, who) {
            return Err(ActionError::Invalid("not coming of age".into()));
        }
        // Grown now, and grown since the World began.
        let grew_up = born_period(state, kin, who) + (kin.grown_at * kin.year) as i64;
        if age_of(state, &cast, kin, who) < kin.grown_at || grew_up < 0 {
            return Err(ActionError::Invalid("not coming of age".into()));
        }
        let seed = mix(&[who.0, 83]);
        let current = job(state, kin, who).unwrap_or("");
        let learning = current.is_empty() || kin.learning.contains(&current);
        let trade = if learning {
            // A trade of a parent's, now and then, else one of the place's.
            let parental = parents(state, who)
                .into_iter()
                .filter_map(|parent| job(state, kin, parent).map(str::to_string))
                .find(|job| kin.trades.iter().any(|(_, id)| id == job));
            match parental.filter(|_| seed.is_multiple_of(3)) {
                Some(job) => job,
                None => pick(kin.trades, seed / 3)
                    .map(|(_, id)| id.to_string())
                    .unwrap_or_default(),
            }
        } else {
            current.to_string()
        };
        let slots = [
            ("name", first_name(state, who)),
            ("trade", kin.trade_words(&trade)),
        ];
        let told = words(kin.words.came_of_age, seed / 7, &slots);
        let said = words(kin.words.came_of_age_said, seed / 11, &slots);
        let mut draft = draft("came_of_age", who, who, told, said);
        draft.payload.insert("trade".into(), trade.as_str().into());
        draft.changes.push(set(who, OF_AGE, true));
        if learning && !trade.is_empty() {
            draft.changes.push(set(who, kin.job_key, trade.as_str()));
        }
        Ok(draft)
    }
}

/// Someone grown moves out of their parents' home into one of their own.
struct LeavesHome(fn(&WorldState) -> Cast);

impl Action for LeavesHome {
    fn name(&self) -> &'static str {
        "lives_left_home"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let kin = kin_of(&cast)?;
        let who = arg_entity(request, "who")?;
        let Some(parent) = lives_with(state, who) else {
            return Err(ActionError::Invalid("no parents' home to leave".into()));
        };
        if !of_age(state, who) || gone(state, who) {
            return Err(ActionError::Invalid("not grown".into()));
        }
        let seed = mix(&[who.0, 89]);
        let slots = [("name", first_name(state, who))];
        let told = words(kin.words.left_home, seed, &slots);
        let said = words(kin.words.left_home_said, seed / 3, &slots);
        let mut draft = draft("left_home", who, who, told, said);
        draft.payload.insert("from".into(), Value::Entity(parent));
        draft.changes.push(set(who, OWN_HOME, true));
        Ok(draft)
    }
}

// ---- Growing old ----------------------------------------------------------

/// Someone old enough stops working.
struct Retires(fn(&WorldState) -> Cast);

impl Action for Retires {
    fn name(&self) -> &'static str {
        "lives_retired"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let kin = kin_of(&cast)?;
        let who = arg_entity(request, "who")?;
        let current = job(state, kin, who).unwrap_or("");
        if gone(state, who)
            || (kin.keeps)(who)
            || current.is_empty()
            || current == kin.retired_job
            || retired(state, who)
            || kin.learning.contains(&current)
            || age_of(state, &cast, kin, who) < kin.retire_at
        {
            return Err(ActionError::Invalid("not retiring".into()));
        }
        let seed = mix(&[who.0, 97]);
        let slots = [
            ("name", first_name(state, who)),
            ("trade", kin.trade_words(current)),
        ];
        let told = words(kin.words.retired, seed, &slots);
        let said = words(kin.words.retired_said, seed / 3, &slots);
        let mut draft = draft("retired", who, who, told, said);
        draft.payload.insert("trade".into(), current.into());
        draft.changes.push(set(who, kin.job_key, kin.retired_job));
        draft.changes.push(set(who, RETIRED, true));
        Ok(draft)
    }
}

/// The chance, in ten-thousandths, that someone dies of old age this
/// period.
pub fn frailty(state: &WorldState, cast: &Cast, person: EntityId) -> u64 {
    let Some(kin) = cast.kin else {
        return 0;
    };
    if (kin.keeps)(person) || gone(state, person) {
        return 0;
    }
    let age = age_of(state, cast, kin, person);
    if age < kin.frail_at {
        return 0;
    }
    // Company keeps the old going; a lonely winter tells.
    let lonely = lack(state, person, Need::Company).clamp(0, 100) as u64;
    kin.frailty * (age - kin.frail_at + 1) * (100 + lonely) / 100
}

/// Someone old dies, gently, in their sleep.
struct Dies(fn(&WorldState) -> Cast);

impl Action for Dies {
    fn name(&self) -> &'static str {
        "lives_died"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let kin = kin_of(&cast)?;
        let who = arg_entity(request, "who")?;
        if state.entity(who).is_none() || frailty(state, &cast, who) == 0 {
            return Err(ActionError::Invalid("not their time".into()));
        }
        let age = age_of(state, &cast, kin, who);
        let mourner = closest(state, &cast, who);
        let now = period(state, &cast) as i64;
        let seed = mix(&[who.0, now as u64, 101]);
        let slots = [("name", first_name(state, who)), ("age", age.to_string())];
        let told = words(kin.words.died, seed, &slots);
        let said = mourner
            .map(|_| words(kin.words.died_said, seed / 3, &slots))
            .unwrap_or_default();
        let mut draft = draft("died", mourner.unwrap_or(who), who, told, said);
        draft.payload.insert("age".into(), (age as i64).into());
        let mut moves = Moves::default();
        moves.set(who, GONE, true);
        moves.set(who, DIED, now);
        if let Some(partner) = partner(state, who) {
            moves.set(who, "lives.was_with", Value::Entity(partner));
            moves.unset(state, who, PARTNER);
            if entity_of(state, partner, PARTNER) == Some(who) {
                moves.unset(state, partner, PARTNER);
            }
        }
        if let Some(mourner) = mourner {
            moves.lack(state, mourner, Need::Company, 30);
        }
        draft.changes.extend(moves.changes);
        Ok(draft)
    }
}

/// The closest of those left inherits something of the one who died.
struct PassesHeirloom(fn(&WorldState) -> Cast);

impl Action for PassesHeirloom {
    fn name(&self) -> &'static str {
        "lives_heirloom_passed"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let kin = kin_of(&cast)?;
        let who = arg_entity(request, "who")?;
        if died(state, who).is_none() || entity_of(state, who, HEIR).is_some() {
            return Err(ActionError::Invalid("nothing to pass on".into()));
        }
        let heir = partner_before(state, who)
            .filter(|heir| state.entity(*heir).is_some() && !gone(state, *heir))
            .or_else(|| closest(state, &cast, who))
            .ok_or_else(|| ActionError::Invalid("nobody to pass it to".into()))?;
        let seed = mix(&[who.0, heir.0, 103]);
        let heirloom = pick(kin.heirlooms, seed).copied().unwrap_or("a keepsake");
        let slots = [
            ("name", first_name(state, who)),
            ("heir", first_name(state, heir)),
            ("heirloom", heirloom.to_string()),
        ];
        let told = words(kin.words.heirloom, seed / 3, &slots);
        let said = words(kin.words.heirloom_said, seed / 5, &slots);
        let mut draft = draft("heirloom_passed", heir, who, told, said);
        draft.payload.insert("heirloom".into(), heirloom.into());
        draft.payload.insert("to".into(), Value::Entity(heir));
        draft.changes.push(set(heir, HEIRLOOM, heirloom));
        draft.changes.push(set(who, HEIR, Value::Entity(heir)));
        Ok(draft)
    }
}

/// A bench or a stone goes up for someone who died: where the player
/// puts it, or, when they do not, where the place does.
struct PlacesMemorial(fn(&WorldState) -> Cast);

impl Action for PlacesMemorial {
    fn name(&self) -> &'static str {
        "lives_memorial_placed"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let kin = kin_of(&cast)?;
        let who = arg_entity(request, "who")?;
        let Some(at) = died(state, who) else {
            return Err(ActionError::Invalid("nobody to remember".into()));
        };
        if entity_of(state, who, MEMORIAL).is_some() {
            return Err(ActionError::Invalid("already remembered".into()));
        }
        let by_player = matches!(request.args.get("by"), Some(Value::Text(by)) if by == "player");
        let now = period(state, &cast) as i64;
        if !by_player && now - at < kin.memorial_wait as i64 {
            return Err(ActionError::Invalid("the player may yet place it".into()));
        }
        let id = (kin.first_memorial..kin.first_memorial + kin.room)
            .map(EntityId::new)
            .find(|id| state.entity(*id).is_none())
            .ok_or_else(|| ActionError::Invalid("no room".into()))?;
        // The player's is the first kind; the place's is whichever suits.
        let shape = if by_player {
            0
        } else {
            (mix(&[who.0, 107]) % kin.memorials.len().max(1) as u64) as usize
        };
        let memorial = kin.memorials[shape.min(kin.memorials.len().saturating_sub(1))];
        let named = memorial_name(state, kin, who, shape);
        let mut fixture = Entity::new(id, "fixture")
            .with_component("name", named.as_str())
            .with_component("shape", memorial.shape)
            .with_component("at", Value::Entity(kin.memorial_at))
            .with_component("built", now)
            .with_component(MEMORIAL_OF, Value::Entity(who));
        match request.args.get("spot") {
            Some(Value::Integer(spot)) => {
                fixture = fixture.with_component(SPOT, (*spot).clamp(0, 100));
            }
            _ if !by_player => {
                // The place puts it somewhere quiet of its own choosing.
                fixture = fixture.with_component(SPOT, (mix(&[who.0, 109]) % 90 + 5) as i64);
            }
            _ => {}
        }
        let speaker = partner_before(state, who)
            .filter(|p| state.entity(*p).is_some() && !gone(state, *p))
            .or_else(|| closest(state, &cast, who));
        let seed = mix(&[who.0, 113]);
        let slots = [
            ("name", first_name(state, who)),
            ("memorial", named.clone()),
        ];
        let told = words(
            if by_player {
                kin.words.memorial_by_player
            } else {
                kin.words.memorial_by_town
            },
            seed,
            &slots,
        );
        let said = speaker
            .map(|_| words(kin.words.memorial_said, seed / 3, &slots))
            .unwrap_or_default();
        let mut draft = match speaker {
            Some(speaker) => draft("memorial_placed", speaker, who, told, said),
            None => {
                let mut draft = draft("memorial_placed", who, who, told, String::new());
                draft.actor = None;
                draft
            }
        };
        draft.payload.insert("memorial".into(), Value::Entity(id));
        draft.payload.insert("shape".into(), memorial.shape.into());
        draft.payload.insert(
            "by".into(),
            (if by_player { "player" } else { "town" }).into(),
        );
        draft.changes.push(StateChange::CreateEntity(fixture));
        draft.changes.push(set(who, MEMORIAL, Value::Entity(id)));
        Ok(draft)
    }
}

/// A year on from someone's death, the one closest remembers them.
struct KeepsAnniversary(fn(&WorldState) -> Cast);

impl Action for KeepsAnniversary {
    fn name(&self) -> &'static str {
        "lives_anniversary_kept"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let kin = kin_of(&cast)?;
        let who = arg_entity(request, "who")?;
        let at = died(state, who).ok_or_else(|| ActionError::Invalid("not gone".into()))?;
        let now = period(state, &cast) as i64;
        let year = kin.year.max(1) as i64;
        let since = now - at;
        if since <= 0 || since % year != 0 {
            return Err(ActionError::Invalid("not the day".into()));
        }
        let years = since / year;
        if integer(state, who, ANNIVERSARY).is_some_and(|kept| kept >= years) {
            return Err(ActionError::Invalid("already kept".into()));
        }
        let rememberer = partner_before(state, who)
            .filter(|p| state.entity(*p).is_some() && !gone(state, *p))
            .or_else(|| closest(state, &cast, who))
            .ok_or_else(|| ActionError::Invalid("nobody left to remember".into()))?;
        let seed = mix(&[who.0, years as u64, 127]);
        let slots = [
            ("name", first_name(state, who)),
            ("heir", first_name(state, rememberer)),
        ];
        let told = words(kin.words.anniversary, seed, &slots);
        let said = words(kin.words.anniversary_said, seed / 3, &slots);
        let mut draft = draft("anniversary_kept", rememberer, who, told, said);
        draft.changes.push(set(who, ANNIVERSARY, years));
        Ok(draft)
    }
}

pub(crate) fn register_actions(
    registry: &mut ActionRegistry,
    cast: fn(&WorldState) -> Cast,
) -> Result<(), ActionError> {
    registry.register(Born(cast))?;
    registry.register(ComesOfAge(cast))?;
    registry.register(LeavesHome(cast))?;
    registry.register(Retires(cast))?;
    registry.register(Dies(cast))?;
    registry.register(PassesHeirloom(cast))?;
    registry.register(PlacesMemorial(cast))?;
    registry.register(KeepsAnniversary(cast))?;
    Ok(())
}

/// The latest event of `kind` about `who`, for what follows from it.
fn latest_about(world: &World, kind: &str, who: EntityId) -> Option<EventId> {
    let index = world.history_index();
    index.of_kind(kind).iter().rev().copied().find(|id| {
        world
            .event(*id)
            .is_some_and(|event| event.payload.get("who") == Some(&Value::Entity(who)))
    })
}

fn run(
    world: &mut World,
    actions: &ActionRegistry,
    request: ActionRequest,
    events: &mut Vec<EventId>,
) -> bool {
    match world.execute(actions, &request) {
        Ok(event) => {
            events.push(event.id);
            true
        }
        Err(_) => false,
    }
}

/// One period of the place's generations: the dead are remembered, the
/// old may retire or die, couples may have a child, and children grow up
/// and move out. At most one of each a period, so each is a day of its own.
pub fn tick(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
) -> Result<Vec<EventId>, WorldError> {
    let mut events = Vec::new();
    let Some(kin) = cast.kin else {
        return Ok(events);
    };
    let now = period(world.state(), cast);
    // Memorials the player did not place in time, and anniversaries.
    let departed = departed(world.state());
    for (who, memorial) in &departed {
        let cause = latest_about(world, "died", *who);
        if memorial.is_none() {
            let mut request = memorial_request(*who, false, None);
            if let Some(cause) = cause {
                request = request.caused_by(cause);
            }
            run(world, actions, request, &mut events);
        }
        let mut request =
            ActionRequest::new("lives_anniversary_kept").arg("who", Value::Entity(*who));
        if let Some(cause) = cause {
            request = request.caused_by(cause);
        }
        run(world, actions, request, &mut events);
    }
    let people = (cast.people)(world)
        .into_iter()
        .chain(children(world.state(), cast))
        .filter(|id| world.state().entity(*id).is_some() && !gone(world.state(), *id))
        .collect::<Vec<_>>();
    // The old: someone may die, gently, and someone may retire.
    let state = world.state();
    let dying = people.iter().copied().find(|person| {
        let odds = frailty(state, cast, *person);
        odds > 0
            && !(kin.busy)(state, *person)
            && !crate::open(state, cast).iter().any(|(key, _)| {
                Candidate::parse(key)
                    .is_some_and(|open| open.a == *person || open.b == Some(*person))
            })
            && mix(&[now, person.0, 131]) % 10_000 < odds
    });
    if let Some(who) = dying {
        let request = ActionRequest::new("lives_died").arg("who", Value::Entity(who));
        if run(world, actions, request, &mut events) {
            let died = *events.last().expect("just recorded");
            let request = ActionRequest::new("lives_heirloom_passed")
                .arg("who", Value::Entity(who))
                .caused_by(died);
            run(world, actions, request, &mut events);
        }
    }
    let state = world.state();
    let retiring = people.iter().copied().find(|person| {
        let work = job(state, kin, *person).unwrap_or("");
        !(kin.keeps)(*person)
            && !work.is_empty()
            && work != kin.retired_job
            && !retired(state, *person)
            && !kin.learning.contains(&work)
            && age_of(state, cast, kin, *person) >= kin.retire_at
            && mix(&[now, person.0, 137]).is_multiple_of(20)
    });
    if let Some(who) = retiring {
        let request = ActionRequest::new("lives_retired").arg("who", Value::Entity(who));
        run(world, actions, request, &mut events);
    }
    // Couples may have a child.
    let state = world.state();
    if (kin.births_now)(state) {
        let couple = people
            .iter()
            .copied()
            .filter_map(|a| {
                let b = partner(state, a).filter(|b| b.0 > a.0)?;
                Some((a, b, birth_odds(state, cast, a, b)?))
            })
            .find(|(a, b, odds)| mix(&[now, a.0, b.0, 139]) % 10_000 < *odds);
        if let Some((a, b, _)) = couple {
            // A full block of children must not end births in silence: a
            // test (any build with debug assertions) fails here, loudly.
            let room = world.state().room_left_in(kin.children());
            debug_assert!(
                room > 0,
                "no room for another child: every id in {} is taken",
                kin.children()
            );
            if room > 0 {
                let request = ActionRequest::new("lives_born")
                    .actor(a)
                    .arg("a", Value::Entity(a))
                    .arg("b", Value::Entity(b));
                run(world, actions, request, &mut events);
            }
        }
    }
    // Children grow up, and the grown move out.
    let state = world.state();
    let coming = people.iter().copied().find(|person| {
        !of_age(state, *person)
            && age_of(state, cast, kin, *person) >= kin.grown_at
            && born_period(state, kin, *person) + (kin.grown_at * kin.year) as i64 >= 0
    });
    if let Some(who) = coming {
        let mut request = ActionRequest::new("lives_came_of_age").arg("who", Value::Entity(who));
        if let Some(cause) = latest_about(world, "born", who) {
            request = request.caused_by(cause);
        }
        run(world, actions, request, &mut events);
    }
    let state = world.state();
    let leaving = people.iter().copied().find(|person| {
        of_age(state, *person)
            && lives_with(state, *person).is_some()
            && partner(state, *person).is_none()
            && mix(&[now, person.0, 149]).is_multiple_of(40)
    });
    if let Some(who) = leaving {
        let mut request = ActionRequest::new("lives_left_home").arg("who", Value::Entity(who));
        if let Some(cause) = latest_about(world, "came_of_age", who) {
            request = request.caused_by(cause);
        }
        run(world, actions, request, &mut events);
    }
    Ok(events)
}
