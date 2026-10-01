//! Talk with a goal: now and then someone asks the player for something
//! talk can do, and the player does it by talking.
//!
//! A favour is one of four things: looking in on someone, asking someone
//! out, cheering someone up, or passing on someone's apology. It is asked
//! as an Action of its own at the end of a day the player is there for,
//! at most one at a time and none in a World's first two periods, and
//! recorded as an Event whose line the asker says aloud. The player does
//! it by talking to whoever it is for, heard as usual (by this System's
//! own ears, or a model's meaning when the voice is on): the exchange is
//! recorded as ever, and then a second Action records the favour done,
//! caused by the ask and by the exchange, which warms the asker to the
//! player and the two people to each other. A favour not done lapses
//! quietly after a few periods; nothing is recorded, and nobody minds.
//!
//! Everything a favour says is written here, in whole sentences, and
//! recorded in its Events, so replay never works a line out again.

use crate::{accepts_invite, hurt_recently, period, Heard, Intent, Kit, Reply, SPOKEN};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, Event, EventDraft, EventId,
    StateChange, Value, World, WorldError, WorldState,
};

/// The kind of Event a favour asked is recorded as.
pub const ASKED: &str = "favour_asked";
/// The kind of Event a favour done is recorded as.
pub const DONE: &str = "favour_done";

/// The last favour asked, on whoever asked it and on nobody else: when,
/// and, until it is done, what and for whom. Once done only when it was
/// asked is kept; a lapsed one stays as it was until the next is asked.
const FAVOUR: &str = "conversation.favour";
/// When someone last asked the player a favour, as Worlds made with v0.26
/// kept it on everyone who had ever asked one. Read, never written: the
/// favour itself keeps when it was asked now.
const FAVOURED: &str = "conversation.favoured";

/// No favour is asked before this many periods have passed.
pub const FIRST_PERIOD: i64 = 2;
/// A favour stays open for the period it is asked in and this many more.
pub const OPEN_PERIODS: i64 = 3;
/// For this many periods a World asks only the gentlest favours.
const GENTLE_PERIODS: i64 = 5;

/// What a favour is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Look in on someone the asker has not seen.
    AskAfter,
    /// Ask someone out for a walk or an evening.
    Invite,
    /// Say something kind to someone who is low.
    CheerUp,
    /// Tell someone the asker is sorry.
    Sorry,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::AskAfter, Kind::Invite, Kind::CheerUp, Kind::Sorry];

    pub fn id(self) -> &'static str {
        match self {
            Kind::AskAfter => "ask_after",
            Kind::Invite => "invite",
            Kind::CheerUp => "cheer_up",
            Kind::Sorry => "sorry",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.id() == id)
    }

    fn gentle(self) -> bool {
        matches!(self, Kind::AskAfter | Kind::Invite)
    }

    /// What the asker says, `{x}` being whom it is for.
    fn asks(self) -> [&'static str; 2] {
        match self {
            Kind::AskAfter => [
                "Have you seen {x} lately? Would you look in on them for me?",
                "If you pass {x}, would you ask how they are? I'd like to know.",
            ],
            Kind::Invite => [
                "{x} hardly gets out. Would you ask them along for a walk?",
                "Would you invite {x} out some evening? They'd say yes to you.",
            ],
            Kind::CheerUp => [
                "{x} seems a bit down. Would you find a kind word for them?",
                "Could you cheer {x} up a little? A kind word from you would do it.",
            ],
            Kind::Sorry => [
                "If you see {x}, would you tell them I'm sorry? I can't find the words myself.",
                "Would you tell {x} I'm sorry? It would mean more coming from you.",
            ],
        }
    }

    /// What whoever it is for adds to their answer, `{y}` being the asker.
    fn heard(self) -> &'static str {
        match self {
            Kind::AskAfter => "Did {y} ask after me? That's kind of them.",
            Kind::Invite => "{y} put you up to this, didn't they? I'm glad.",
            Kind::CheerUp => "You always know what to say. That's cheered me up.",
            Kind::Sorry => "{y} said that? Well. Tell them it's forgotten.",
        }
    }

    /// What the asker says when it is done, `{x}` being whom it was for.
    fn thanks(self) -> [&'static str; 2] {
        match self {
            Kind::AskAfter => [
                "You looked in on {x}? Thank you. That puts my mind at rest.",
                "Thank you for seeing {x} for me. I feel better knowing.",
            ],
            Kind::Invite => [
                "{x} said yes? Thank you. I knew you'd manage it.",
                "You got {x} to come out! Thank you.",
            ],
            Kind::CheerUp => [
                "{x} is smiling again. Thank you for that.",
                "Whatever you said to {x}, it worked. Thank you.",
            ],
            Kind::Sorry => [
                "You told {x}? Thank you. I feel lighter already.",
                "Thank you for talking to {x} for me. I owe you one.",
            ],
        }
    }

    /// The drawer's note, `{y}` the asker and `{x}` whom it is for.
    fn note(self) -> &'static str {
        match self {
            Kind::AskAfter => "{y} asked you to look in on {x}.",
            Kind::Invite => "{y} asked you to invite {x} out.",
            Kind::CheerUp => "{y} asked you to cheer {x} up.",
            Kind::Sorry => "{y} asked you to tell {x} they're sorry.",
        }
    }

    /// How it is done, in a few words.
    fn hint(self) -> &'static str {
        match self {
            Kind::AskAfter => "Have a word with {x}.",
            Kind::Invite => "Ask {x} to come out with you.",
            Kind::CheerUp => "Say something kind to {x}.",
            Kind::Sorry => "Tell {x} what {y} said.",
        }
    }
}

fn filled(line: &str, asker: &str, whom: &str) -> String {
    line.replace("{y}", asker).replace("{x}", whom)
}

/// A favour someone asked of the player that is still open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Favour {
    pub asker: EntityId,
    pub whom: EntityId,
    pub kind: Kind,
    /// The period it was asked in.
    pub asked: i64,
}

fn favour_on(state: &WorldState, asker: EntityId) -> Option<Favour> {
    let Some(Value::Map(map)) = state.entity(asker)?.component(FAVOUR) else {
        return None;
    };
    let kind = match map.get("kind") {
        Some(Value::Text(kind)) => Kind::from_id(kind)?,
        _ => return None,
    };
    let whom = match map.get("whom") {
        Some(Value::Entity(whom)) => *whom,
        _ => return None,
    };
    let asked = match map.get("asked") {
        Some(Value::Integer(asked)) => *asked,
        _ => return None,
    };
    Some(Favour {
        asker,
        whom,
        kind,
        asked,
    })
}

/// The favour open now, if any: asked no more than [`OPEN_PERIODS`] ago,
/// by and for people who can still be spoken to.
pub fn open(state: &WorldState, kit: &Kit) -> Option<Favour> {
    let now = period(state, kit);
    let people = (kit.people)(state);
    people.iter().find_map(|asker| {
        favour_on(state, *asker)
            .filter(|favour| now - favour.asked <= OPEN_PERIODS && people.contains(&favour.whom))
    })
}

/// Whether talking to `who`, heard as `heard`, does `favour`.
fn does(state: &WorldState, kit: &Kit, favour: &Favour, who: EntityId, heard: Heard) -> bool {
    if who != favour.whom || matches!(heard.intent, Intent::Rude | Intent::Unclear) {
        return false;
    }
    let accepted = heard.intent == Intent::Invite && accepts_invite(state, kit, who);
    match favour.kind {
        Kind::AskAfter => true,
        Kind::Invite => accepted,
        Kind::CheerUp => {
            accepted
                || matches!(
                    heard.intent,
                    Intent::Comfort | Intent::Compliment | Intent::Gift | Intent::Thank
                )
        }
        // Word passed on is word about the asker.
        Kind::Sorry => heard.about == Some(favour.asker),
    }
}

/// Adds to what someone answers when the player's words do the favour
/// open for them: they can tell who sent the player.
pub(crate) fn answer(world: &World, kit: &Kit, who: EntityId, heard: Heard, reply: &mut Reply) {
    let state = world.state();
    let Some(favour) = open(state, kit).filter(|favour| does(state, kit, favour, who, heard))
    else {
        return;
    };
    let added = filled(
        favour.kind.heard(),
        &lives::first_name(state, favour.asker),
        &lives::first_name(state, who),
    );
    if reply.line.chars().count() + added.chars().count() < crate::MOST_REPLY {
        reply.line = format!("{} {added}", reply.line.trim_end());
    }
}

/// Whom a favour of `kind` asked by `asker` would be for, if anyone.
fn whom_for(state: &WorldState, kit: &Kit, asker: EntityId, kind: Kind) -> Option<EntityId> {
    let others = (kit.people)(state)
        .into_iter()
        .filter(|other| *other != asker && grown(state, *other) && lives::enrolled(state, *other))
        .collect::<Vec<_>>();
    // The most fitting, the first by id among equals.
    let best = |score: &dyn Fn(EntityId) -> Option<i64>| {
        others
            .iter()
            .filter_map(|other| score(*other).map(|score| (score, *other)))
            .max_by_key(|(score, other)| (*score, std::cmp::Reverse(*other)))
            .map(|(_, other)| other)
    };
    match kind {
        Kind::AskAfter => best(&|other| {
            let liking = lives::opinion(state, asker, other);
            (liking >= 0).then_some(liking)
        }),
        // Someone short of company who would say yes today.
        Kind::Invite => best(&|other| {
            let lack = lives::lack(state, other, lives::Need::Company);
            (lack >= 30 && accepts_invite(state, kit, other)).then_some(lack)
        }),
        Kind::CheerUp => best(&|other| {
            let worst = lives::Need::ALL
                .iter()
                .map(|need| lives::lack(state, other, *need))
                .max()
                .unwrap_or(0);
            (worst >= 50).then_some(worst)
        }),
        Kind::Sorry => best(&|other| {
            let opinion = lives::opinion(state, asker, other);
            (opinion <= -10).then_some(-opinion)
        }),
    }
}

/// Whether someone is grown: nobody's child still, or come of age.
fn grown(state: &WorldState, person: EntityId) -> bool {
    lives::parents(state, person).is_empty() || lives::generations::of_age(state, person)
}

/// When every World begins, in periods: its clock starts at nought. (Not
/// its first Event's: a World opened from a checkpoint keeps only the
/// history since, and must ask the very same favours.)
const BEGAN: i64 = 0;

/// The favour to ask now, if one is due: at most one open, none in the
/// first [`FIRST_PERIOD`] periods, the first a few periods in and each
/// next a week or so after the last was asked.
pub fn proposal(world: &World, kit: &Kit) -> Option<ActionRequest> {
    let state = world.state();
    let now = period(state, kit);
    let began = BEGAN;
    if now < began + FIRST_PERIOD || open(state, kit).is_some() {
        return None;
    }
    let last = state.entities().filter_map(last_asked).max();
    let due = match last {
        None => began + FIRST_PERIOD + (lives::mix(&[began as u64, 0xfa]) % 3) as i64,
        Some((at, _)) => at + 6 + (lives::mix(&[at as u64, 0xfa]) % 5) as i64,
    };
    if now < due {
        return None;
    }
    let seed = lives::mix(&[now as u64, 0xfa7]);
    let gentle = now < began + GENTLE_PERIODS;
    let mut askers = (kit.people)(state)
        .into_iter()
        .filter(|asker| {
            grown(state, *asker)
                && lives::enrolled(state, *asker)
                && lives::regard(state, *asker) >= 0
                && !hurt_recently(state, kit, *asker)
                && last.map(|(_, who)| who) != Some(*asker)
        })
        .collect::<Vec<_>>();
    if askers.is_empty() {
        return None;
    }
    let turned = (seed % askers.len() as u64) as usize;
    askers.rotate_left(turned);
    let turn = (seed / 7 % Kind::ALL.len() as u64) as usize;
    for asker in askers {
        for step in 0..Kind::ALL.len() {
            let kind = Kind::ALL[(turn + step) % Kind::ALL.len()];
            if gentle && !kind.gentle() {
                continue;
            }
            if let Some(whom) = whom_for(state, kit, asker, kind) {
                return Some(
                    ActionRequest::new(ASK_ACTION)
                        .actor(asker)
                        .arg("asker", Value::Entity(asker))
                        .arg("whom", Value::Entity(whom))
                        .arg("favour", kind.id()),
                );
            }
        }
    }
    None
}

/// When `entity` last asked a favour, if it ever did, and who it is.
fn last_asked(entity: &world_core::Entity) -> Option<(i64, EntityId)> {
    let favour = match entity.component(FAVOUR) {
        Some(Value::Map(map)) => match map.get("asked") {
            Some(Value::Integer(asked)) => Some(*asked),
            _ => None,
        },
        _ => None,
    };
    let favoured = match entity.component(FAVOURED) {
        Some(Value::Integer(at)) => Some(*at),
        _ => None,
    };
    favour.max(favoured).map(|at| (at, entity.id))
}

/// At the end of a period the player was there for, someone may ask them
/// a favour. Nothing is asked while they are away.
pub fn tick(
    world: &mut World,
    actions: &ActionRegistry,
    kit: &Kit,
    away: bool,
) -> Result<Vec<EventId>, WorldError> {
    if away {
        return Ok(Vec::new());
    }
    match proposal(world, kit) {
        Some(request) => Ok(vec![world.execute(actions, &request)?.id]),
        None => Ok(Vec::new()),
    }
}

fn text<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    match event.payload.get(key) {
        Some(Value::Text(text)) => Some(text),
        _ => None,
    }
}

/// The latest ask of the favour open on `asker`.
fn ask_event(world: &World, asker: EntityId) -> Option<&Event> {
    world
        .events_of_kind(&[ASKED])
        .into_iter()
        .rev()
        .find(|event| event.actor == Some(asker))
}

/// After the player has spoken to someone (`spoken`, a `conversation_say`
/// Event), records the favour done if those words did it: caused by the
/// ask and by the words.
pub fn follow_up(
    world: &mut World,
    actions: &ActionRegistry,
    kit: &Kit,
    spoken: EventId,
) -> Result<Option<EventId>, WorldError> {
    let Some(event) = world.event(spoken).filter(|event| event.kind == SPOKEN) else {
        return Ok(None);
    };
    let Some(favour) = open(world.state(), kit) else {
        return Ok(None);
    };
    let Some(who) = event.targets.first().copied() else {
        return Ok(None);
    };
    let Some(intent) = text(event, "intent").and_then(Intent::from_id) else {
        return Ok(None);
    };
    let about = match event.payload.get("about") {
        Some(Value::Entity(about)) => Some(*about),
        _ => None,
    };
    if !does(world.state(), kit, &favour, who, Heard { intent, about }) {
        return Ok(None);
    }
    let mut request = ActionRequest::new(DONE_ACTION)
        .actor(favour.asker)
        .arg("asker", Value::Entity(favour.asker))
        .arg("who", Value::Entity(who))
        .arg("intent", intent.id());
    if let Some(about) = about {
        request = request.arg("about", Value::Entity(about));
    }
    if let Some(asked) = ask_event(world, favour.asker) {
        request = request.caused_by(asked.id);
    }
    request = request.caused_by(spoken);
    Ok(Some(world.execute(actions, &request)?.id))
}

/// Whether an Event is a favour asked or done.
pub fn is_favour(event: &Event) -> bool {
    event.kind == ASKED || event.kind == DONE
}

/// How a favour's Event is told in the World's history.
pub fn told(state: &WorldState, event: &Event) -> Option<String> {
    let asker = lives::name(state, event.actor?);
    match event.kind.as_str() {
        ASKED => Some(format!("{asker} asked you a favour")),
        DONE => Some(format!("You did {asker} a favour")),
        _ => None,
    }
}

/// Who says what at a favour's Event: the ask, or the thanks. An Event
/// records only what the favour was, whose and for whom; its words follow
/// from those and its moment, the same on every replay.
pub fn said(state: &WorldState, event: &Event) -> Option<(EntityId, String)> {
    let asker = event.actor?;
    let whom = *event.targets.first()?;
    let kind = Kind::from_id(text(event, "favour")?)?;
    let (lines, salt) = match event.kind.as_str() {
        ASKED => (kind.asks(), 0),
        DONE => (kind.thanks(), 1),
        _ => return None,
    };
    let seed = lives::mix(&[asker.0, whom.0, event.world_time, salt]);
    let line = lines[(seed % 2) as usize];
    Some((
        asker,
        filled(
            line,
            &lives::first_name(state, asker),
            &lives::first_name(state, whom),
        ),
    ))
}

/// The favour to show the player now: the open one, or one done this
/// period, with its note and how it is done, in the System's words.
pub fn shown(world: &World, kit: &Kit) -> Option<world_projection::Favour> {
    let state = world.state();
    let now = period(state, kit);
    let note = |asker: EntityId, whom: EntityId, kind: Kind, done: bool| {
        let (y, x) = (
            lives::first_name(state, asker),
            lives::first_name(state, whom),
        );
        world_projection::Favour {
            asker: world_projection::SelectionId::Entity(asker),
            whom: world_projection::SelectionId::Entity(whom),
            note: filled(kind.note(), &y, &x),
            hint: filled(kind.hint(), &y, &x),
            done,
        }
    };
    if let Some(favour) = open(state, kit) {
        return Some(note(favour.asker, favour.whom, favour.kind, false));
    }
    let done = world.events_of_kind(&[DONE]).into_iter().next_back()?;
    if (done.world_time / kit.period.max(1)) as i64 != now {
        return None;
    }
    let kind = Kind::from_id(text(done, "favour")?)?;
    let whom = done.targets.first().copied()?;
    Some(note(done.actor?, whom, kind, true))
}

const ASK_ACTION: &str = "conversation_favour_ask";
const DONE_ACTION: &str = "conversation_favour_done";

/// Registers the Actions that ask a favour and record one done.
pub(crate) fn register_actions(
    actions: &mut ActionRegistry,
    kit: fn(&WorldState) -> Kit,
) -> Result<(), ActionError> {
    actions.register(Asks(kit))?;
    actions.register(Done(kit))
}

fn entity(request: &ActionRequest, key: &str) -> Result<EntityId, ActionError> {
    match request.args.get(key) {
        Some(Value::Entity(id)) => Ok(*id),
        _ => Err(ActionError::Invalid(format!("{key}: whom?"))),
    }
}

struct Asks(fn(&WorldState) -> Kit);

impl Action for Asks {
    fn name(&self) -> &'static str {
        ASK_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let asker = entity(request, "asker")?;
        let whom = entity(request, "whom")?;
        let kind = match request.args.get("favour") {
            Some(Value::Text(kind)) => Kind::from_id(kind),
            _ => None,
        }
        .ok_or_else(|| ActionError::Invalid("what favour?".into()))?;
        let people = (kit.people)(state);
        if asker == whom || !people.contains(&asker) || !people.contains(&whom) {
            return Err(ActionError::Invalid(
                "a favour is asked by someone here, for someone else here".into(),
            ));
        }
        if open(state, &kit).is_some() {
            return Err(ActionError::Invalid("one favour is open already".into()));
        }
        let now = period(state, &kit);
        let mut changes = Vec::new();
        // The last favour, lapsed or done, is let go of.
        for entity in state.entities() {
            if entity.id != asker && entity.component(FAVOUR).is_some() {
                changes.push(StateChange::RemoveComponent {
                    entity: entity.id,
                    key: FAVOUR.into(),
                });
            }
        }
        changes.push(StateChange::SetComponent {
            entity: asker,
            key: FAVOUR.into(),
            value: Value::Map(
                [
                    ("kind".to_string(), Value::from(kind.id())),
                    ("whom".to_string(), Value::Entity(whom)),
                    ("asked".to_string(), Value::Integer(now)),
                ]
                .into_iter()
                .collect(),
            ),
        });
        let mut draft = EventDraft::new(ASKED);
        draft.actor = Some(asker);
        draft.targets = vec![whom];
        draft.payload.insert("favour".into(), kind.id().into());
        draft.changes = changes;
        Ok(draft)
    }
}

struct Done(fn(&WorldState) -> Kit);

impl Action for Done {
    fn name(&self) -> &'static str {
        DONE_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let asker = entity(request, "asker")?;
        let who = entity(request, "who")?;
        let intent = match request.args.get("intent") {
            Some(Value::Text(intent)) => Intent::from_id(intent),
            _ => None,
        }
        .ok_or_else(|| ActionError::Invalid("unknown meaning".into()))?;
        let about = match request.args.get("about") {
            Some(Value::Entity(about)) => Some(*about),
            None => None,
            Some(_) => return Err(ActionError::Invalid("about whom?".into())),
        };
        let favour = open(state, &kit)
            .filter(|favour| favour.asker == asker)
            .ok_or_else(|| ActionError::Invalid("no such favour is open".into()))?;
        if !does(state, &kit, &favour, who, Heard { intent, about }) {
            return Err(ActionError::Invalid("that doesn't do the favour".into()));
        }
        // Only when it was asked is kept, for when the next is due.
        let mut changes = vec![StateChange::SetComponent {
            entity: asker,
            key: FAVOUR.into(),
            value: Value::Map(
                [("asked".to_string(), Value::Integer(favour.asked))]
                    .into_iter()
                    .collect(),
            ),
        }];
        let (warmth, asker_to, whom_to) = match favour.kind {
            Kind::AskAfter => (4, 3, 3),
            Kind::Invite => (4, 3, 2),
            Kind::CheerUp => (4, 2, 2),
            Kind::Sorry => (5, 5, 10),
        };
        if lives::enrolled(state, asker) {
            changes.push(lives::regard_by(state, asker, warmth));
        }
        changes.extend(lives::opinion_by(state, asker, who, asker_to));
        changes.extend(lives::opinion_by(state, who, asker, whom_to));
        let mut draft = EventDraft::new(DONE);
        draft.actor = Some(asker);
        draft.targets = vec![who];
        draft
            .payload
            .insert("favour".into(), favour.kind.id().into());
        draft.changes = changes;
        Ok(draft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{register_actions, say, Era};
    use world_core::Entity;

    const MARA: EntityId = EntityId::new(1);
    const LEO: EntityId = EntityId::new(2);

    fn kit(_: &WorldState) -> Kit {
        Kit {
            period: 10,
            unit: "day",
            settlement: "the harbour",
            people: |state| {
                [MARA, LEO]
                    .into_iter()
                    .filter(|id| state.entity(*id).is_some())
                    .collect()
            },
            places: |_| Vec::new(),
            place_line: |_, _, _| "It's fine.".into(),
            need_line: |_, _| ("Nothing.".into(), None),
            work_line: |_, _| None,
            place_mood: |_| "Quiet.".into(),
            coming_up: |_| None,
            aliases: |_| Vec::new(),
            weather: |_| "Grey.".into(),
            occasions: |_| Vec::new(),
            recalled: |_, _, _| None,
            era: Era::Radio,
            elsewhere: &[],
        }
    }

    fn any_kit() -> Kit {
        kit(&WorldState::default())
    }

    fn person(id: EntityId, name: &str, other: EntityId, opinion: i64) -> Entity {
        Entity::new(id, "person")
            .with_component("name", name)
            .with_component("lives.traits", "warm steady")
            .with_component(lives::REGARD, 10_i64)
            .with_component(lives::Need::Company.key(), 50_i64)
            .with_component(format!("lives.opinion.{}", other.0), opinion)
    }

    fn world() -> (World, ActionRegistry) {
        let mut state = WorldState::default();
        state.seed_entity(person(MARA, "Mara", LEO, -40)).unwrap();
        state.seed_entity(person(LEO, "Leo", MARA, -30)).unwrap();
        let mut actions = ActionRegistry::new();
        register_actions(&mut actions, kit).unwrap();
        (World::new(state), actions)
    }

    fn at(world: &mut World, actions: &ActionRegistry, period: u64) {
        world.advance_to(actions, period * 10).unwrap();
    }

    fn talk(world: &mut World, actions: &ActionRegistry, who: EntityId, words: &str) -> EventId {
        let request = say(world, &kit(world.state()), who, words).unwrap();
        world.execute(actions, &request).unwrap().id
    }

    #[test]
    fn nothing_is_asked_in_the_first_two_periods_and_one_at_a_time() {
        let (mut world, actions) = world();
        for period in 0..FIRST_PERIOD as u64 {
            at(&mut world, &actions, period);
            assert!(tick(&mut world, &actions, &any_kit(), false)
                .unwrap()
                .is_empty());
        }
        let mut asked = Vec::new();
        for period in FIRST_PERIOD as u64..10 {
            at(&mut world, &actions, period);
            // Nothing is asked of a player who is away.
            assert!(tick(&mut world, &actions, &any_kit(), true)
                .unwrap()
                .is_empty());
            asked.extend(tick(&mut world, &actions, &any_kit(), false).unwrap());
            if !asked.is_empty() {
                break;
            }
        }
        let [ask] = asked[..] else {
            panic!("one favour asked by period 5: {asked:?}")
        };
        let event = world.event(ask).unwrap();
        // Early on, only a gentle one: here, asking someone out.
        assert_eq!(text(event, "favour"), Some("invite"));
        assert_eq!(said(world.state(), event).unwrap().0, event.actor.unwrap());
        // Only one is open at a time.
        let again = ActionRequest::new(ASK_ACTION)
            .arg("asker", Value::Entity(LEO))
            .arg("whom", Value::Entity(MARA))
            .arg("favour", "ask_after");
        assert!(world.execute(&actions, &again).is_err());
        assert!(shown(&world, &kit(world.state())).is_some_and(|favour| !favour.done));
    }

    #[test]
    fn word_passed_on_does_a_favour_caused_by_the_ask_and_the_words() {
        let (mut world, actions) = world();
        at(&mut world, &actions, 6);
        let ask = world
            .execute(
                &actions,
                &ActionRequest::new(ASK_ACTION)
                    .actor(MARA)
                    .arg("asker", Value::Entity(MARA))
                    .arg("whom", Value::Entity(LEO))
                    .arg("favour", "sorry"),
            )
            .unwrap()
            .id;
        let kit = kit(world.state());
        // An unkind word, or a word to the asker herself, does nothing.
        let rude = talk(&mut world, &actions, LEO, "You're an idiot.");
        assert_eq!(follow_up(&mut world, &actions, &kit, rude).unwrap(), None);
        let other = talk(&mut world, &actions, MARA, "What do you think of Leo?");
        assert_eq!(follow_up(&mut world, &actions, &kit, other).unwrap(), None);
        // Telling Leo does it.
        let regard = lives::regard(world.state(), MARA);
        let opinion = lives::opinion(world.state(), LEO, MARA);
        let spoken = talk(&mut world, &actions, LEO, "Mara says she's sorry.");
        let reply = text(world.event(spoken).unwrap(), "reply")
            .unwrap()
            .to_string();
        assert!(
            reply.ends_with("Mara said that? Well. Tell them it's forgotten."),
            "{reply}"
        );
        let done = follow_up(&mut world, &actions, &kit, spoken)
            .unwrap()
            .unwrap();
        let done = world.event(done).unwrap();
        assert_eq!(done.kind, DONE);
        assert_eq!(done.caused_by, vec![ask, spoken]);
        assert_eq!(done.actor, Some(MARA));
        assert!(said(world.state(), done).unwrap().1.contains("Leo"));
        assert_eq!(
            told(world.state(), done).as_deref(),
            Some("You did Mara a favour")
        );
        assert!(lives::regard(world.state(), MARA) > regard);
        assert!(lives::opinion(world.state(), LEO, MARA) >= opinion + 10);
        assert!(open(world.state(), &kit).is_none());
        assert!(shown(&world, &kit).is_some_and(|favour| favour.done));
        // Done once only.
        let again = talk(&mut world, &actions, LEO, "Mara says she's sorry.");
        assert_eq!(follow_up(&mut world, &actions, &kit, again).unwrap(), None);
        // Replay needs nobody to hear anything again.
        assert_eq!(world.replay().unwrap().state(), world.state());
    }

    /// Whoever asked last keeps the favour, and nobody else: once done,
    /// only when it was asked, for when the next is due.
    #[test]
    fn only_the_last_favour_is_kept() {
        let (mut world, actions) = world();
        let ask = |world: &mut World, asker: EntityId, whom: EntityId| {
            world
                .execute(
                    &actions,
                    &ActionRequest::new(ASK_ACTION)
                        .actor(asker)
                        .arg("asker", Value::Entity(asker))
                        .arg("whom", Value::Entity(whom))
                        .arg("favour", "sorry"),
                )
                .unwrap();
        };
        let kept = |world: &World| {
            [MARA, LEO].map(|person| {
                world
                    .state()
                    .entity(person)
                    .unwrap()
                    .component(FAVOUR)
                    .cloned()
            })
        };
        at(&mut world, &actions, 6);
        ask(&mut world, MARA, LEO);
        // Lapsed, it stays as it was until the next is asked.
        at(&mut world, &actions, 7 + OPEN_PERIODS as u64);
        assert!(kept(&world)[0].is_some());
        ask(&mut world, LEO, MARA);
        assert!(kept(&world)[0].is_none());
        let kit = kit(world.state());
        let spoken = talk(&mut world, &actions, MARA, "Leo says he's sorry.");
        assert!(follow_up(&mut world, &actions, &kit, spoken)
            .unwrap()
            .is_some());
        let asked = Value::Map(
            [("asked".to_string(), Value::Integer(7 + OPEN_PERIODS))]
                .into_iter()
                .collect(),
        );
        assert_eq!(kept(&world), [None, Some(asked)]);
        // The next is due after the last one asked, not the first.
        assert_eq!(
            world.state().entities().filter_map(last_asked).max(),
            Some((7 + OPEN_PERIODS, LEO))
        );
    }

    #[test]
    fn a_favour_lapses_quietly() {
        let (mut world, actions) = world();
        at(&mut world, &actions, 6);
        world
            .execute(
                &actions,
                &ActionRequest::new(ASK_ACTION)
                    .actor(MARA)
                    .arg("asker", Value::Entity(MARA))
                    .arg("whom", Value::Entity(LEO))
                    .arg("favour", "ask_after"),
            )
            .unwrap();
        at(&mut world, &actions, 6 + OPEN_PERIODS as u64);
        assert!(open(world.state(), &kit(world.state())).is_some());
        at(&mut world, &actions, 7 + OPEN_PERIODS as u64);
        assert!(open(world.state(), &kit(world.state())).is_none());
        assert!(shown(&world, &kit(world.state())).is_none());
        // Too late to do it now.
        let spoken = talk(&mut world, &actions, LEO, "How are you?");
        assert_eq!(
            follow_up(&mut world, &actions, &any_kit(), spoken).unwrap(),
            None
        );
    }
}
