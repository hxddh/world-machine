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
    pub people: fn(&World) -> Vec<EntityId>,
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
}

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
    for person in (kit.people)(world) {
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

fn parse(key: &str) -> Option<(Verb, &str, EntityId)> {
    let mut parts = key.splitn(3, '.');
    let verb = Verb::from_id(parts.next()?)?;
    let what = parts.next()?;
    let at = EntityId::new(parts.next()?.parse().ok()?);
    Some((verb, what, at))
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
        let (verb, what, at) =
            parse(key).ok_or_else(|| ActionError::Invalid(format!("no deed {key}")))?;
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
                changes.extend((kit.gift)(state, at));
                (
                    "gift_given",
                    kit.gift_cost,
                    format!("You gave {at_name} a present"),
                    None,
                )
            }
            Verb::Invite => {
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
    Ok(())
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
    Ok(events)
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
    )
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
