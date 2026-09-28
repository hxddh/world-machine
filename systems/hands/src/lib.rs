//! Hands: what the player does in a World with their own hands, the way
//! Animal Crossing and Townscaper let you shape a place, rather than only
//! answering what it asks.
//!
//! The player builds, decorates and plants where they choose, moves what
//! they made, gives someone a gift and invites someone out. Each is an
//! ordinary Action the World validates and records as an Event, so what the
//! player made stands in the place because the recorded state says so, and
//! a World replays without anyone's hands.
//!
//! What gets made is a fixture: the same kind of entity the storyteller
//! puts on the scene, with a name, a shape in the Pack's own words and the
//! place it stands at. Plants grow through stages as periods pass; a
//! decoration comes down after a while. This System knows nothing about
//! harbours or colonies: a World Pack gives it a [`Kit`].

use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, Event, EventDraft,
    EventId, StateChange, Value, World, WorldError, WorldState,
};

/// What the player can do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verb {
    Build,
    Decorate,
    Plant,
    Move,
    Give,
    Invite,
}

impl Verb {
    pub const ALL: [Verb; 6] = [
        Verb::Build,
        Verb::Decorate,
        Verb::Plant,
        Verb::Move,
        Verb::Give,
        Verb::Invite,
    ];

    /// The verb as the player reads it.
    pub fn word(self) -> &'static str {
        match self {
            Verb::Build => "Build",
            Verb::Decorate => "Decorate",
            Verb::Plant => "Plant",
            Verb::Move => "Move",
            Verb::Give => "Give",
            Verb::Invite => "Invite",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Verb::Build => "build",
            Verb::Decorate => "decorate",
            Verb::Plant => "plant",
            Verb::Move => "move",
            Verb::Give => "give",
            Verb::Invite => "invite",
        }
    }

    fn from_id(id: &str) -> Option<Verb> {
        Verb::ALL.into_iter().find(|verb| verb.id() == id)
    }
}

/// Something the player can build, put up or plant.
#[derive(Clone, Copy, Debug)]
pub struct Thing {
    pub id: &'static str,
    /// What it is called: "Bench".
    pub name: &'static str,
    /// Built, put up as a decoration, or planted.
    pub verb: Verb,
    /// How it is drawn, in the Pack's own words ("bench", "bunting").
    pub shape: &'static str,
    /// What it costs from the purse, if the World keeps one.
    pub cost: i64,
    /// How many periods it stays up, if not for good.
    pub lasts: Option<u64>,
    /// What a plant is called as it grows, one stage every
    /// [`Kit::growing`] periods, and the shape it is drawn as at each.
    pub stages: &'static [(&'static str, &'static str)],
    /// What it does for the people who live around it.
    pub effect: Effect,
}

/// What something the player made does for the people around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Nothing but look nice.
    None,
    /// Somewhere to sit: someone tired rests there.
    Rest,
    /// Somewhere to gather: people meet there of an evening.
    Gather,
    /// Something that bears: once grown, someone brings the player some.
    Harvest,
}

impl Effect {
    pub fn id(self) -> &'static str {
        match self {
            Effect::None => "none",
            Effect::Rest => "rest",
            Effect::Gather => "gather",
            Effect::Harvest => "harvest",
        }
    }
}

/// Where a World keeps the money its player's hands spend.
#[derive(Clone, Copy, Debug)]
pub struct Purse {
    pub entity: EntityId,
    pub key: &'static str,
    /// What it is called: "the harbour fund".
    pub name: &'static str,
}

/// Everything the System needs to know about a World's place.
#[derive(Clone, Debug)]
pub struct Kit {
    /// The entity the System keeps its notes on.
    pub notes: EntityId,
    /// The first entity id something made can take; the next ones follow.
    pub first: u64,
    /// How many things can ever be made.
    pub room: u64,
    /// How much world time one period is.
    pub period: u64,
    pub things: &'static [Thing],
    /// Where things can be put: the World's places, in a fixed order.
    pub places: fn(&WorldState) -> Vec<EntityId>,
    /// Who can be given something or invited: everyone living there now.
    pub people: fn(&WorldState) -> Vec<EntityId>,
    pub purse: Option<Purse>,
    /// What a gift costs.
    pub gift_cost: i64,
    /// What a gift does for someone, in the Pack's terms, besides costing.
    pub gift: fn(&WorldState, EntityId) -> Vec<StateChange>,
    /// What an invitation does for someone, in the Pack's terms.
    pub invite: fn(&WorldState, EntityId) -> Vec<StateChange>,
    /// Where someone invited goes.
    pub gathering: EntityId,
    /// How many periods a plant takes to reach its next stage.
    pub growing: u64,
    /// How many things the player may do in one period.
    pub per_period: i64,
    /// How many things the player has made may stand at once.
    pub most_standing: usize,
    /// What using something the player made does for someone, in the
    /// Pack's terms: a rest on a bench, an evening under a lamp.
    pub enjoy: fn(&WorldState, EntityId, Effect) -> Vec<StateChange>,
}

/// How far along the ground something stands, from 0 (the left edge) to
/// 100 (the right), when the player put it somewhere of their choosing.
pub const SPOT: &str = "spot";

/// What the latest deed changed, so it can be taken back the same period.
const UNDO: &str = "hands.undo";

/// The kind of entity made things are, shared with the storyteller's
/// fixtures so the scene draws both the same way.
pub const FIXTURE: &str = "fixture";

/// Whether a fixture was made by the player's hands.
pub const MADE: &str = "hands.made";

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

fn name(state: &WorldState, entity: EntityId) -> String {
    text(state, entity, "name")
        .map(str::to_string)
        .unwrap_or_else(|| "somewhere".into())
}

/// Which period this is.
pub fn period(state: &WorldState, kit: &Kit) -> u64 {
    state.world_time() / kit.period.max(1)
}

/// What the player's hands have made that still stands.
pub fn made(state: &WorldState) -> Vec<EntityId> {
    state
        .entities()
        .filter(|entity| entity.kind == FIXTURE)
        .filter(|entity| matches!(entity.component(MADE), Some(Value::Bool(true))))
        .map(|entity| entity.id)
        .collect()
}

fn next_id(state: &WorldState, kit: &Kit) -> Option<EntityId> {
    let last = integer(state, kit.notes, "hands.last").unwrap_or(kit.first as i64 - 1);
    let next = (last + 1).max(kit.first as i64) as u64;
    (next < kit.first + kit.room).then(|| EntityId::new(next))
}

fn done_key(period: u64) -> String {
    format!("hands.done.{period}")
}

/// How many more things the player can do this period.
pub fn left_this_period(state: &WorldState, kit: &Kit) -> i64 {
    kit.per_period - integer(state, kit.notes, &done_key(period(state, kit))).unwrap_or(0)
}

fn purse_holds(state: &WorldState, kit: &Kit) -> Option<i64> {
    kit.purse
        .map(|purse| integer(state, purse.entity, purse.key).unwrap_or(0))
}

/// Why something cannot be done now, if it cannot.
fn why_not(state: &WorldState, kit: &Kit, cost: i64) -> Option<String> {
    if left_this_period(state, kit) <= 0 {
        return Some("That's enough for today".into());
    }
    match (kit.purse, purse_holds(state, kit)) {
        (Some(purse), Some(holds)) if holds < cost => {
            Some(format!("There isn't {cost} in {}", purse.name))
        }
        _ => None,
    }
}

/// Something the player could do now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deed {
    /// What the Pack names its command by: `verb.what.where`.
    pub key: String,
    pub verb: Verb,
    /// What it is done with: a thing's name, or who it is for.
    pub thing: String,
    /// Where it is done, or to whom.
    pub at: EntityId,
    /// What it costs, in words, if anything.
    pub cost: Option<String>,
    pub unavailable: Option<String>,
}

/// Everything the player could do with their hands now.
pub fn deeds(world: &World, kit: &Kit) -> Vec<Deed> {
    let state = world.state();
    let places = (kit.places)(state)
        .into_iter()
        .filter(|place| state.entity(*place).is_some())
        .collect::<Vec<_>>();
    let cost_words = |cost: i64| (kit.purse.is_some() && cost > 0).then(|| cost.to_string());
    let full = made(state).len() >= kit.most_standing;
    let mut deeds = Vec::new();
    for thing in kit.things {
        for place in &places {
            deeds.push(Deed {
                key: format!("{}.{}.{}", thing.verb.id(), thing.id, place),
                verb: thing.verb,
                thing: thing.name.to_string(),
                at: *place,
                cost: cost_words(thing.cost),
                unavailable: if full {
                    Some("There's no more room".into())
                } else {
                    why_not(state, kit, thing.cost)
                },
            });
        }
    }
    for fixture in made(state) {
        let at = match state
            .entity(fixture)
            .and_then(|entity| entity.component("at"))
        {
            Some(Value::Entity(at)) => Some(*at),
            _ => None,
        };
        for place in places.iter().filter(|place| Some(**place) != at) {
            deeds.push(Deed {
                key: format!("move.{fixture}.{place}"),
                verb: Verb::Move,
                thing: name(state, fixture),
                at: *place,
                cost: None,
                unavailable: why_not(state, kit, 0),
            });
        }
    }
    for person in (kit.people)(world.state()) {
        let who = name(state, person);
        deeds.push(Deed {
            key: format!("give.gift.{person}"),
            verb: Verb::Give,
            thing: who.clone(),
            at: person,
            cost: cost_words(kit.gift_cost),
            unavailable: why_not(state, kit, kit.gift_cost),
        });
        deeds.push(Deed {
            key: format!("invite.out.{person}"),
            verb: Verb::Invite,
            thing: who,
            at: person,
            cost: None,
            unavailable: why_not(state, kit, 0),
        });
    }
    deeds
}

/// A deed's key, `verb.what.where`, with `@spot` after it when the player
/// chose where along the ground it stands.
fn parse(key: &str) -> Option<(Verb, &str, EntityId, Option<i64>)> {
    let (key, spot) = match key.split_once('@') {
        Some((key, spot)) => {
            let spot = spot
                .parse::<i64>()
                .ok()
                .filter(|spot| (0..=100).contains(spot))?;
            (key, Some(spot))
        }
        None => (key, None),
    };
    let mut parts = key.splitn(3, '.');
    let verb = Verb::from_id(parts.next()?)?;
    let what = parts.next()?;
    let at = EntityId::new(parts.next()?.parse().ok()?);
    Some((verb, what, at, spot))
}

/// The key that does `deed` with what it makes standing at `spot` along
/// the ground (0 to 100).
pub fn at_spot(deed: &str, spot: u8) -> String {
    let deed = deed.split_once('@').map_or(deed, |(deed, _)| deed);
    format!("{deed}@{}", spot.min(100))
}

fn thing<'a>(kit: &'a Kit, id: &str) -> Option<&'a Thing> {
    kit.things.iter().find(|thing| thing.id == id)
}

/// The player does something with their hands.
struct Does(fn(&WorldState) -> Kit);

impl Action for Does {
    fn name(&self) -> &'static str {
        "hands_do"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let key = match request.args.get("deed") {
            Some(Value::Text(key)) => key.as_str(),
            _ => return Err(ActionError::Invalid("missing deed".into())),
        };
        let (verb, what, at, spot) =
            parse(key).ok_or_else(|| ActionError::Invalid(format!("no deed {key}")))?;
        if spot.is_some() && matches!(verb, Verb::Give | Verb::Invite) {
            return Err(ActionError::Invalid("a person has no spot".into()));
        }
        if state.entity(at).is_none() {
            return Err(ActionError::Invalid("nowhere to do it".into()));
        }
        let now = period(state, &kit);
        let at_name = name(state, at);
        let mut changes = Vec::new();
        let (kind, cost, told, made_id) = match verb {
            Verb::Build | Verb::Decorate | Verb::Plant => {
                let thing = thing(&kit, what)
                    .filter(|thing| thing.verb == verb)
                    .ok_or_else(|| ActionError::Invalid(format!("no {what} to make")))?;
                if !(kit.places)(state).contains(&at) {
                    return Err(ActionError::Invalid("it can't go there".into()));
                }
                if made(state).len() >= kit.most_standing {
                    return Err(ActionError::Invalid("there's no more room".into()));
                }
                let id = next_id(state, &kit)
                    .ok_or_else(|| ActionError::Invalid("nothing more can be made".into()))?;
                let (label, shape) = thing
                    .stages
                    .first()
                    .copied()
                    .unwrap_or((thing.name, thing.shape));
                let mut fixture = Entity::new(id, FIXTURE)
                    .with_component("name", label)
                    .with_component("shape", shape)
                    .with_component("at", Value::Entity(at))
                    .with_component(MADE, true)
                    .with_component("hands.thing", thing.id)
                    .with_component("hands.since", now as i64);
                if let Some(lasts) = thing.lasts {
                    fixture = fixture.with_component("until", (now + lasts) as i64);
                }
                if let Some(spot) = spot {
                    fixture = fixture.with_component(SPOT, spot);
                }
                changes.push(StateChange::CreateEntity(fixture));
                changes.push(StateChange::SetComponent {
                    entity: kit.notes,
                    key: "hands.last".into(),
                    value: (id.0 as i64).into(),
                });
                let (kind, told) = match verb {
                    Verb::Build => (
                        "built_by_hand",
                        format!("You built a {} by {at_name}", thing.name.to_lowercase()),
                    ),
                    Verb::Decorate => (
                        "decorated_by_hand",
                        format!("You put up {} at {at_name}", thing.name.to_lowercase()),
                    ),
                    _ => (
                        "planted_by_hand",
                        format!("You planted {} by {at_name}", thing.name.to_lowercase()),
                    ),
                };
                (kind, thing.cost, told, Some(id))
            }
            Verb::Move => {
                let fixture = EntityId::new(
                    what.parse()
                        .map_err(|_| ActionError::Invalid("move what?".into()))?,
                );
                if !made(state).contains(&fixture) {
                    return Err(ActionError::Invalid(
                        "only what you made can be moved".into(),
                    ));
                }
                if !(kit.places)(state).contains(&at) {
                    return Err(ActionError::Invalid("it can't go there".into()));
                }
                changes.push(StateChange::SetComponent {
                    entity: fixture,
                    key: "at".into(),
                    value: Value::Entity(at),
                });
                match spot {
                    Some(spot) => changes.push(StateChange::SetComponent {
                        entity: fixture,
                        key: SPOT.into(),
                        value: spot.into(),
                    }),
                    None if integer(state, fixture, SPOT).is_some() => {
                        changes.push(StateChange::RemoveComponent {
                            entity: fixture,
                            key: SPOT.into(),
                        });
                    }
                    None => {}
                }
                (
                    "moved_by_hand",
                    0,
                    format!(
                        "You moved the {} to {at_name}",
                        name(state, fixture).to_lowercase()
                    ),
                    Some(fixture),
                )
            }
            Verb::Give => {
                if !(kit.people)(state).contains(&at) {
                    return Err(ActionError::Invalid("only someone living here".into()));
                }
                changes.extend((kit.gift)(state, at));
                (
                    "gift_given",
                    kit.gift_cost,
                    format!("You gave {at_name} a present"),
                    None,
                )
            }
            Verb::Invite => {
                if !(kit.people)(state).contains(&at) {
                    return Err(ActionError::Invalid("only someone living here".into()));
                }
                changes.extend((kit.invite)(state, at));
                (
                    "invited_out",
                    0,
                    format!("You invited {at_name} to {}", name(state, kit.gathering)),
                    None,
                )
            }
        };
        if let Some(why) = why_not(state, &kit, cost) {
            return Err(ActionError::Invalid(why));
        }
        if let (Some(purse), true) = (kit.purse, cost > 0) {
            let holds = purse_holds(state, &kit).unwrap_or(0);
            changes.push(StateChange::SetComponent {
                entity: purse.entity,
                key: purse.key.into(),
                value: (holds - cost).into(),
            });
        }
        if state.entity(kit.notes).is_none() {
            changes.insert(
                0,
                StateChange::CreateEntity(
                    Entity::new(kit.notes, "hands").with_component("name", "Hands"),
                ),
            );
        }
        let done = integer(state, kit.notes, &done_key(now)).unwrap_or(0);
        changes.push(StateChange::SetComponent {
            entity: kit.notes,
            key: done_key(now),
            value: (done + 1).into(),
        });
        // What it would take to take this back, this period: what was
        // made, or where a moved thing stood before, and what it cost.
        let undo = match (verb, made_id) {
            (Verb::Build | Verb::Decorate | Verb::Plant, Some(id)) => {
                Some(format!("{now}|made|{}|{cost}", id.0))
            }
            (Verb::Move, Some(id)) => {
                let from = match state.entity(id).and_then(|entity| entity.component("at")) {
                    Some(Value::Entity(from)) => from.0,
                    _ => at.0,
                };
                let from_spot =
                    integer(state, id, SPOT).map_or("-".into(), |spot| spot.to_string());
                Some(format!("{now}|moved|{}|0|{from}|{from_spot}", id.0))
            }
            _ => None,
        };
        changes.push(match undo {
            Some(undo) => StateChange::SetComponent {
                entity: kit.notes,
                key: UNDO.into(),
                value: undo.into(),
            },
            None => StateChange::RemoveComponent {
                entity: kit.notes,
                key: UNDO.into(),
            },
        });
        let mut draft = EventDraft::new(kind);
        draft.targets = made_id.into_iter().chain([at]).collect();
        draft.payload.insert("deed".into(), key.into());
        draft.payload.insert("told".into(), told.into());
        draft.changes = changes;
        Ok(draft)
    }
}

/// A plant reaches its next stage.
struct Grows(fn(&WorldState) -> Kit);

impl Action for Grows {
    fn name(&self) -> &'static str {
        "hands_grow"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let plant = match request.args.get("plant") {
            Some(Value::Entity(id)) => *id,
            _ => return Err(ActionError::Invalid("missing plant".into())),
        };
        let (label, shape) = due_stage(state, &kit, plant)
            .ok_or_else(|| ActionError::Invalid("not grown yet".into()))?;
        let at = match state
            .entity(plant)
            .and_then(|entity| entity.component("at"))
        {
            Some(Value::Entity(at)) => name(state, *at),
            _ => "the ground".into(),
        };
        let mut draft = EventDraft::new("plant_grew");
        draft.targets = vec![plant];
        draft.payload.insert(
            "told".into(),
            format!(
                "The {} by {at} became {}",
                name(state, plant).to_lowercase(),
                label.to_lowercase()
            )
            .into(),
        );
        draft.changes = vec![
            StateChange::SetComponent {
                entity: plant,
                key: "name".into(),
                value: label.into(),
            },
            StateChange::SetComponent {
                entity: plant,
                key: "shape".into(),
                value: shape.into(),
            },
        ];
        Ok(draft)
    }
}

/// The stage a plant has grown into and not yet been given, if any.
fn due_stage(
    state: &WorldState,
    kit: &Kit,
    plant: EntityId,
) -> Option<(&'static str, &'static str)> {
    let thing = thing(kit, text(state, plant, "hands.thing")?)?;
    let since = integer(state, plant, "hands.since")?.max(0) as u64;
    let age = period(state, kit).saturating_sub(since);
    let stage = ((age / kit.growing.max(1)) as usize).min(thing.stages.len().saturating_sub(1));
    let (label, shape) = *thing.stages.get(stage)?;
    (text(state, plant, "name") != Some(label)).then_some((label, shape))
}

/// Registers the System's Actions for a Pack's kit.
pub fn register_actions(
    registry: &mut ActionRegistry,
    kit: fn(&WorldState) -> Kit,
) -> Result<(), ActionError> {
    registry.register(Does(kit))?;
    registry.register(Grows(kit))?;
    registry.register(Undoes(kit))?;
    registry.register(Enjoys(kit))?;
    Ok(())
}

/// The request that takes back the latest thing made or moved, the same
/// period.
pub fn undo_request() -> ActionRequest {
    ActionRequest::new("hands_undo")
}

/// What the player could take back now, in words: "Take back the bench".
pub fn can_undo(state: &WorldState, kit: &Kit) -> Option<String> {
    let note = text(state, kit.notes, UNDO)?;
    let mut parts = note.split('|');
    let when: u64 = parts.next()?.parse().ok()?;
    let _ = parts.next()?;
    let id = EntityId::new(parts.next()?.parse().ok()?);
    (when == period(state, kit) && state.entity(id).is_some())
        .then(|| format!("Take back the {}", name(state, id).to_lowercase()))
}

/// The player takes back what they last made or moved, this period: it is
/// gone again, or back where it was, and what it cost comes back.
struct Undoes(fn(&WorldState) -> Kit);

impl Action for Undoes {
    fn name(&self) -> &'static str {
        "hands_undo"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        if can_undo(state, &kit).is_none() {
            return Err(ActionError::Invalid("nothing to take back".into()));
        }
        let note = text(state, kit.notes, UNDO).unwrap_or_default().to_string();
        let parts = note.split('|').collect::<Vec<_>>();
        let number = |index: usize| parts.get(index).and_then(|part| part.parse::<u64>().ok());
        let (Some(id), Some(cost)) = (number(2), number(3)) else {
            return Err(ActionError::Invalid("nothing to take back".into()));
        };
        let id = EntityId::new(id);
        let thing_name = name(state, id).to_lowercase();
        let now = period(state, &kit);
        let mut changes = Vec::new();
        let told = if parts.get(1) == Some(&"made") {
            changes.push(StateChange::RemoveEntity(id));
            if let (Some(purse), true) = (kit.purse, cost > 0) {
                let holds = purse_holds(state, &kit).unwrap_or(0);
                changes.push(StateChange::SetComponent {
                    entity: purse.entity,
                    key: purse.key.into(),
                    value: (holds + cost as i64).into(),
                });
            }
            format!("You took the {thing_name} down again")
        } else {
            let from = number(4).map(EntityId::new);
            if let Some(from) = from {
                changes.push(StateChange::SetComponent {
                    entity: id,
                    key: "at".into(),
                    value: Value::Entity(from),
                });
            }
            match number(5) {
                Some(spot) => changes.push(StateChange::SetComponent {
                    entity: id,
                    key: SPOT.into(),
                    value: (spot as i64).into(),
                }),
                None if integer(state, id, SPOT).is_some() => {
                    changes.push(StateChange::RemoveComponent {
                        entity: id,
                        key: SPOT.into(),
                    })
                }
                None => {}
            }
            format!("You put the {thing_name} back where it was")
        };
        let done = integer(state, kit.notes, &done_key(now)).unwrap_or(0);
        changes.push(StateChange::SetComponent {
            entity: kit.notes,
            key: done_key(now),
            value: (done - 1).max(0).into(),
        });
        changes.push(StateChange::RemoveComponent {
            entity: kit.notes,
            key: UNDO.into(),
        });
        let mut draft = EventDraft::new("undone_by_hand");
        draft.targets = vec![id];
        draft.payload.insert("told".into(), told.into());
        draft.changes = changes;
        Ok(draft)
    }
}

/// At most this many things the player made are used a day.
const USED_A_DAY: usize = 2;

/// What someone says resting on something the player made.
const REST_LINES: [&str; 10] = [
    "The {what} by {place}: just what my legs needed.",
    "Best seat by {place}, this {what}.",
    "I could sit on this {what} all day.",
    "Good {what}, this. Solid.",
    "You can see all of {place} from here.",
    "Five minutes on the {what}. Then back to it.",
    "Whoever made this {what} knew what they were doing.",
    "My favourite spot, the {what} by {place}.",
    "Sat on the {what} and watched the gulls. Bliss.",
    "A rest on the {what} and I'm new again.",
];

/// What someone says after an evening by something the player made.
const GATHER_LINES: [&str; 10] = [
    "It's nice by the {what} of an evening.",
    "We lost track of time by the {what}.",
    "The light down by {place} makes you want to stay.",
    "{other} told me a story I'd never heard.",
    "Me and {other}, putting the world to rights.",
    "{other} laughed so hard they cried.",
    "Didn't feel the cold, talking to {other}.",
    "You learn a lot about {other} after dark.",
    "The {what} was the only light down by {place}.",
    "Stayed out by the {what} longer than I meant to.",
];

/// What someone says bringing the player what their garden grew.
const HARVEST_LINES: [&str; 6] = [
    "From your {what}. Seemed only fair.",
    "The {what} did well this week. This is yours.",
    "First pick from your {what}.",
    "Your {what} keeps giving. Here.",
    "Picked these from your {what} this morning.",
    "Don't tell anyone, but your {what} beats mine.",
];

/// Someone uses something the player made: rests on a bench, meets
/// friends under a lamp, brings the player what a garden grew.
struct Enjoys(fn(&WorldState) -> Kit);

/// What a garden gives, by the shape it grew into.
pub const PRODUCE: [&str; 4] = [
    "a basket of what the garden grew",
    "a jar of honey from the flowers",
    "a bunch of fresh herbs",
    "the first fruit of the season",
];

impl Action for Enjoys {
    fn name(&self) -> &'static str {
        "hands_enjoy"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let entity = |key: &str| match request.args.get(key) {
            Some(Value::Entity(id)) => Some(*id),
            _ => None,
        };
        let (Some(fixture), Some(who)) = (entity("thing"), entity("who")) else {
            return Err(ActionError::Invalid("who used what?".into()));
        };
        if !made(state).contains(&fixture) || !(kit.people)(state).contains(&who) {
            return Err(ActionError::Invalid("nothing to use".into()));
        }
        let effect = text(state, fixture, "hands.thing")
            .and_then(|id| thing(&kit, id))
            .map_or(Effect::None, |thing| thing.effect);
        let what = name(state, fixture).to_lowercase();
        let place = match state
            .entity(fixture)
            .and_then(|entity| entity.component("at"))
        {
            Some(Value::Entity(at)) => name(state, *at),
            _ => "the ground".into(),
        };
        let first = name(state, who)
            .split_whitespace()
            .next()
            .unwrap_or("Someone")
            .to_string();
        let other = entity("with").filter(|other| *other != who);
        let other_name = other.map(|other| {
            name(state, other)
                .split_whitespace()
                .next()
                .unwrap_or("a friend")
                .to_string()
        });
        let seed = fixture.0.wrapping_mul(31).wrapping_add(period(state, &kit));
        // Each use says the next of what can be said about it, so the same
        // thing is not heard again until every other has been.
        let uses = match state
            .entity(fixture)
            .and_then(|f| f.component("hands.uses"))
        {
            Some(Value::Integer(uses)) => *uses,
            _ => 0,
        };
        let line = |lines: &[&str]| {
            let lines = lines
                .iter()
                .filter(|line| other_name.is_some() || !line.contains("{other}"))
                .collect::<Vec<_>>();
            let at = (uses.max(0) as u64 + fixture.0) % lines.len().max(1) as u64;
            lines
                .get(at as usize)
                .map(|line| {
                    line.replace("{what}", &what)
                        .replace("{place}", &place)
                        .replace("{other}", other_name.as_deref().unwrap_or(""))
                })
                .unwrap_or_default()
        };
        let mut draft = EventDraft::new("enjoyed");
        let (told, said) = match effect {
            Effect::Rest => (
                format!("{first} rested on the {what} by {place}"),
                line(&REST_LINES),
            ),
            Effect::Gather => (
                match &other_name {
                    Some(other) => format!("{first} and {other} talked under the {what} till late"),
                    None => format!("{first} sat a while by the {what} after dark"),
                },
                line(&GATHER_LINES),
            ),
            Effect::Harvest => {
                let gift = PRODUCE[(seed % PRODUCE.len() as u64) as usize];
                draft.payload.insert("keepsake".into(), gift.into());
                draft.payload.insert("kept".into(), true.into());
                (
                    format!("{first} brought you {gift} from the {what} you planted"),
                    line(&HARVEST_LINES),
                )
            }
            Effect::None => return Err(ActionError::Invalid("nothing to use".into())),
        };
        let mut changes = (kit.enjoy)(state, who, effect);
        if let Some(other) = other {
            changes.extend((kit.enjoy)(state, other, effect));
        }
        changes.push(StateChange::SetComponent {
            entity: fixture,
            key: "hands.used".into(),
            value: (period(state, &kit) as i64).into(),
        });
        changes.push(StateChange::SetComponent {
            entity: fixture,
            key: "hands.uses".into(),
            value: (uses + 1).into(),
        });
        draft.actor = Some(who);
        draft.targets = std::iter::once(fixture).chain(other).collect();
        draft.payload.insert("effect".into(), effect.id().into());
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), said.into());
        draft.changes = changes;
        Ok(draft)
    }
}

/// The request that does a deed.
pub fn do_request(deed: &str) -> ActionRequest {
    ActionRequest::new("hands_do").arg("deed", deed)
}

/// One period of what the player made: plants grow.
pub fn tick(
    world: &mut World,
    actions: &ActionRegistry,
    kit: &Kit,
) -> Result<Vec<EventId>, WorldError> {
    let mut events = Vec::new();
    for plant in made(world.state()) {
        if due_stage(world.state(), kit, plant).is_some() {
            let request = ActionRequest::new("hands_grow").arg("plant", Value::Entity(plant));
            events.push(world.execute(actions, &request)?.id);
        }
    }
    // What the player made gets used: a bench now and then, a lamp most
    // evenings, a garden once it has grown and every week or so after.
    let now = period(world.state(), kit);
    let people = (kit.people)(world.state());
    if people.is_empty() {
        return Ok(events);
    }
    // A harvest is something to keep, and no more than three things to
    // keep come in a week, whoever gives them.
    let week_began = world.world_time().saturating_sub(kit.period * 7);
    let mut kept_this_week = world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time > week_began)
        .filter(|event| {
            event.kind == "keepsake_left"
                || event.payload.get("kept") == Some(&Value::Bool(true))
                || (event.kind == "situation_answered"
                    && event.payload.get("kind") == Some(&Value::Text("keepsake".into())))
        })
        .count();
    let mut used_today = 0;
    // Starting somewhere different each day, so everything gets its turn.
    let mut all = made(world.state()).into_iter().collect::<Vec<_>>();
    if !all.is_empty() {
        let turn = (now as usize) % all.len();
        all.rotate_left(turn);
    }
    for fixture in all {
        let state = world.state();
        let Some(effect) = text(state, fixture, "hands.thing")
            .and_then(|id| thing(kit, id))
            .map(|thing| thing.effect)
        else {
            continue;
        };
        let since = integer(state, fixture, "hands.used")
            .or_else(|| integer(state, fixture, "hands.since"))
            .unwrap_or(0)
            .max(0) as u64;
        let waited = now.saturating_sub(since);
        let due = match effect {
            Effect::None => false,
            Effect::Rest => waited >= 4,
            Effect::Gather => waited >= 3,
            // Only once it has grown into its last stage.
            Effect::Harvest => {
                waited >= 7
                    && text(state, fixture, "hands.thing")
                        .and_then(|id| thing(kit, id))
                        .and_then(|thing| thing.stages.last())
                        .is_none_or(|(last, _)| text(state, fixture, "name") == Some(last))
            }
        };
        // A couple of things in use a day is life about the place; every
        // bench and lamp every day would be all anyone talks about.
        if !due || (effect != Effect::Harvest && used_today >= USED_A_DAY) {
            continue;
        }
        if effect != Effect::Harvest {
            used_today += 1;
        }
        if effect == Effect::Harvest {
            // A harvest a week at most, leaving room in the week for
            // what people give.
            if kept_this_week >= 1 {
                continue;
            }
            kept_this_week += 1;
        }
        let pick = |salt: u64| {
            people[((fixture.0 ^ now.wrapping_mul(salt)) % people.len() as u64) as usize]
        };
        let who = pick(0x9e37);
        let mut request = ActionRequest::new("hands_enjoy")
            .actor(who)
            .arg("thing", Value::Entity(fixture))
            .arg("who", Value::Entity(who));
        let other = pick(0x85eb);
        if effect == Effect::Gather && other != who {
            request = request.arg("with", Value::Entity(other));
        }
        if let Ok(event) = world.execute(actions, &request) {
            events.push(event.id);
        }
    }
    Ok(events)
}

/// The latest thing the player made, put up or planted since a moment, as
/// it was told: "You built a bench by the quay".
pub fn latest_made_since(world: &World, since: u64) -> Option<String> {
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time >= since)
        .filter(|event| {
            matches!(
                event.kind.as_str(),
                "built_by_hand" | "decorated_by_hand" | "planted_by_hand"
            )
        })
        .find_map(told)
}

/// Every kind of thing the player has ever made, put up or planted, by id,
/// whether or not it still stands.
pub fn ever_made(world: &World) -> std::collections::BTreeSet<String> {
    world
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.kind.as_str(),
                "built_by_hand" | "decorated_by_hand" | "planted_by_hand"
            )
        })
        .filter_map(|event| match event.payload.get("deed") {
            Some(Value::Text(deed)) => parse(deed).map(|(_, what, _, _)| what.to_string()),
            _ => None,
        })
        .collect()
}

/// What the player made that stands at a place now, by name: "Bench".
pub fn made_at(state: &WorldState, place: EntityId) -> Option<String> {
    made(state).into_iter().find_map(|id| {
        let entity = state.entity(id)?;
        match (entity.component("at"), entity.component("name")) {
            (Some(Value::Entity(at)), Some(Value::Text(name))) if *at == place => {
                Some(name.clone())
            }
            _ => None,
        }
    })
}

/// Whether an event is one of this System's.
pub fn is_hands(event: &Event) -> bool {
    matches!(
        event.kind.as_str(),
        "built_by_hand"
            | "decorated_by_hand"
            | "planted_by_hand"
            | "moved_by_hand"
            | "gift_given"
            | "invited_out"
            | "plant_grew"
            | "undone_by_hand"
            | "enjoyed"
    )
}

/// Who speaks at one of this System's moments, and what they say.
pub fn said(event: &Event) -> Option<(EntityId, String)> {
    if !is_hands(event) {
        return None;
    }
    match event.payload.get("said") {
        Some(Value::Text(said)) if !said.is_empty() => Some((event.actor?, said.clone())),
        _ => None,
    }
}

/// How one of this System's moments is told.
pub fn told(event: &Event) -> Option<String> {
    if !is_hands(event) {
        return None;
    }
    match event.payload.get("told") {
        Some(Value::Text(told)) => Some(told.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
