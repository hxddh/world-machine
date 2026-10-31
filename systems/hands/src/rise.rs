//! Works rise in stages: a site is marked out, its frame goes up, it is
//! nearly done, and then it is finished. Each stage is a recorded Event,
//! caused by the one before (the deed that began it, or what the story
//! built), so a World replays its works rising without anyone deciding
//! anything again.
//!
//! A work the player began on a plot rises on its own schedule: its
//! stages fall a third and two thirds of the way through the periods it
//! takes, and only on periods before it is finished. Anything else a Pack
//! marks as rising (by setting [`RISING`] to 0 on a fixture as it is put
//! up) rises a stage every period after, and is finished on the period
//! after it was nearly done.
//!
//! While something rises, people lend a hand: [`errands`] says who works
//! at each site and who carries a load over to it, and who the player's
//! seats and plantings draw to them, for the Pack's day plans. None of
//! that is recorded.

use std::collections::BTreeMap;

use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, EventId, StateChange,
    Value, World, WorldError, WorldState,
};

use crate::mark::{building, names_its_place, plot_of, work, BUILDING};
use crate::{integer, made, name, period, text, Kit, FIXTURE};

/// How far something going up has got: 0 marked out, 1 its frame up, 2
/// nearly done. A Pack sets it to 0 on a fixture to have it rise.
pub const RISING: &str = "hands.rise";
/// The period something last rose a stage.
pub const ROSE: &str = "hands.rose";

/// The drawing of a site marked out: pegs and a line, timber stacked.
pub const SITE: &str = "site";
/// The drawing of a frame going up in scaffolding.
pub const SCAFFOLD: &str = "scaffold";
/// The drawing of a work nearly done, its scaffolding coming down.
pub const NEARLY: &str = "nearly-built";

/// The stage the last of a work's stages is: after it, it is finished.
const NEARLY_DONE: i64 = 2;

/// How far a work going up has got, if it is going up.
pub fn rising(state: &WorldState, fixture: EntityId) -> Option<u8> {
    match integer(state, fixture, RISING) {
        Some(stage) => Some(stage.clamp(0, NEARLY_DONE) as u8),
        None => building(state, fixture).then_some(0),
    }
}

/// How something going up is drawn at its stage: a site marked out, a
/// frame in scaffolding, or nearly done. Something that stood before (a
/// pier given a new section) is never a bare site again.
pub fn rising_art(state: &WorldState, fixture: EntityId) -> Option<&'static str> {
    Some(match rising(state, fixture)? {
        0 if integer(state, fixture, ROSE).is_none() => SITE,
        0 | 1 => SCAFFOLD,
        _ => NEARLY,
    })
}

/// The period a stage of a work on a plot rises, for a work that takes
/// `periods`: a third and two thirds of the way, if before it is finished.
pub(crate) fn stage_due(periods: u64, stage: i64) -> Option<u64> {
    let at = (stage as u64 * periods).div_ceil(3);
    (at < periods).then_some(at)
}

/// The next stage something going up rises to now, if it is due one: for
/// a work on a plot, by its schedule; for anything else, a period after
/// the last. A stage past [`NEARLY_DONE`] is something finished.
fn next_stage(state: &WorldState, kit: &Kit, fixture: EntityId) -> Option<i64> {
    let now = period(state, kit) as i64;
    let entity = state.entity(fixture)?;
    if entity.kind != FIXTURE || integer(state, fixture, ROSE).is_some_and(|rose| rose >= now) {
        return None;
    }
    if plot_of(state, fixture).is_some() {
        if !building(state, fixture) {
            return None;
        }
        let next = integer(state, fixture, RISING).unwrap_or(0) + 1;
        let since = integer(state, fixture, "hands.since").unwrap_or(0).max(0);
        let due = stage_due(crate::mark::growing_of(state, kit, fixture), next)?;
        return (next <= NEARLY_DONE && now - since >= due as i64).then_some(next);
    }
    Some(integer(state, fixture, RISING)? + 1)
}

/// What is going up, as it is told: "the frame of the bench you began by
/// the square", "the pier's new section".
fn what_rises(state: &WorldState, kit: &Kit, fixture: EntityId) -> String {
    let at = match state.entity(fixture).and_then(|e| e.component("at")) {
        Some(Value::Entity(at)) => name(state, *at),
        _ => "the path".into(),
    };
    if plot_of(state, fixture).is_some() {
        let thing = text(state, fixture, "hands.thing")
            .and_then(|id| work(kit, id))
            .map_or_else(|| name(state, fixture), |work| work.name.to_string())
            .to_lowercase();
        return if names_its_place(&thing, &at) {
            format!("the {thing} you began")
        } else {
            format!("the {thing} you began by {at}")
        };
    }
    // A story's fixture is told by its name, up to any comma: "The lamp
    // post on the point, unlit" rises as the lamp post on the point.
    let full = name(state, fixture);
    let short = full.split(',').next().unwrap_or(&full).trim();
    let mut chars = short.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => "the work".into(),
    }
}

fn capital(line: &str) -> String {
    let mut chars = line.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Something going up rises a stage.
pub(crate) struct Rises(pub(crate) fn(&WorldState) -> Kit);

impl Action for Rises {
    fn name(&self) -> &'static str {
        "hands_rise"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let fixture = match request.args.get("work") {
            Some(Value::Entity(id)) => *id,
            _ => return Err(ActionError::Invalid("missing work".into())),
        };
        let stage = next_stage(state, &kit, fixture)
            .ok_or_else(|| ActionError::Invalid("nothing rises yet".into()))?;
        let what = what_rises(state, &kit, fixture);
        let told = match stage {
            1 => format!("The frame of {what} went up"),
            2 => capital(&format!("{what} was nearly done")),
            _ => capital(&format!("{what} was finished")),
        };
        let mut draft = EventDraft::new("work_rose");
        draft.targets = vec![fixture];
        draft.payload.insert("stage".into(), stage.into());
        draft.payload.insert("told".into(), told.into());
        let now = period(state, &kit) as i64;
        draft.changes = if stage > NEARLY_DONE {
            // Finished: nothing about it says it is going up any more.
            vec![StateChange::RemoveComponent {
                entity: fixture,
                key: RISING.into(),
            }]
        } else {
            vec![
                StateChange::SetComponent {
                    entity: fixture,
                    key: RISING.into(),
                    value: stage.into(),
                },
                StateChange::SetComponent {
                    entity: fixture,
                    key: ROSE.into(),
                    value: now.into(),
                },
            ]
        };
        Ok(draft)
    }
}

/// The latest Event that put up or raised `fixture`, which the next stage
/// follows from: one that made it, began it going up, or raised it a
/// stage. What only touched it on the way (a design, a name, a morning's
/// tending) is never where its building follows from.
pub(crate) fn raised_by(world: &World, fixture: EntityId) -> Option<EventId> {
    world
        .history_index()
        .changes_of(fixture)
        .iter()
        .rev()
        .copied()
        .find(|id| {
            world
                .event(*id)
                .is_some_and(|event| advances_construction(event, fixture))
        })
}

/// Whether `event` made `fixture` or moved its building on.
pub(crate) fn advances_construction(event: &world_core::Event, fixture: EntityId) -> bool {
    event.changes.iter().any(|change| match change {
        StateChange::CreateEntity(entity) => entity.id == fixture,
        StateChange::SetComponent { entity, key, .. }
        | StateChange::RemoveComponent { entity, key } => {
            *entity == fixture && [RISING, ROSE, BUILDING].contains(&key.as_str())
        }
        _ => false,
    })
}

/// One period of what is going up rising: whatever is due its next stage
/// rises to it. A work on a plot due to be finished is finished instead
/// ([`crate::mark::finish`] comes first).
pub(crate) fn rise(
    world: &mut World,
    actions: &ActionRegistry,
    kit: &Kit,
) -> Result<Vec<EventId>, WorldError> {
    let state = world.state();
    let due = state
        .entities()
        .filter(|entity| entity.kind == FIXTURE)
        .map(|entity| entity.id)
        .filter(|id| next_stage(state, kit, *id).is_some())
        .collect::<Vec<_>>();
    let mut events = Vec::new();
    for fixture in due {
        let mut request = ActionRequest::new("hands_rise").arg("work", Value::Entity(fixture));
        if let Some(cause) = raised_by(world, fixture) {
            request = request.caused_by(cause);
        }
        events.push(world.execute(actions, &request)?.id);
    }
    Ok(events)
}

/// Everything going up now: works begun on plots and anything a Pack set
/// rising.
pub fn going_up(state: &WorldState) -> Vec<EntityId> {
    state
        .entities()
        .filter(|entity| entity.kind == FIXTURE)
        .map(|entity| entity.id)
        .filter(|id| rising(state, *id).is_some())
        .collect()
}

/// What the player's hands give someone to do today, for their day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Errand {
    /// Where.
    pub at: EntityId,
    /// Working there all day ([`days::Doing::Work`]), or what they do
    /// there for an hour: carry a load over, sit, gather, tend.
    pub doing: days::Doing,
}

/// How many works going up get a crew at once.
const CREWS: usize = 2;
/// How many periods after something is put down it draws someone to it.
const DRAWS_FOR: u64 = 2;

/// What the player's hands give people to do today, by who does it: at
/// each work going up, one of `hands` works there all day and another
/// carries a load over first thing; and a seat or a planting the player
/// put down these last days draws someone to sit on it or water it, as a
/// planting does every other day after. `use_of`
/// says what something made is good for, as the Pack draws it, and `far`
/// how far from it someone spends their day, if they are within reach of
/// it: whoever is nearest lends a hand, and nobody crosses the place for
/// it. Chosen afresh
/// each day, the same way every time, and never recorded.
pub fn errands(
    world: &World,
    kit: &Kit,
    hands: &[EntityId],
    use_of: impl Fn(EntityId) -> Option<days::Use>,
    far: impl Fn(EntityId, EntityId) -> Option<f32>,
) -> BTreeMap<EntityId, Errand> {
    use days::{Doing, Use};
    let state = world.state();
    let today = period(state, kit);
    let mut errands = BTreeMap::new();
    let mut free = hands.to_vec();
    // The crews, the oldest work first.
    let mut sites = going_up(state);
    sites.sort();
    for site in sites.into_iter().take(CREWS) {
        for (salt, doing) in [(0x6b, Doing::Work), (0xca, Doing::Haul)] {
            take(
                &mut free,
                &[site.0, today, salt],
                &far,
                &mut errands,
                Errand { at: site, doing },
            );
        }
    }
    let doing_of = |used: Use| match used {
        Use::Seat => Doing::Sit,
        Use::Light | Use::Stall => Doing::Gather,
        Use::Planting => Doing::Tend,
    };
    // What the player put down these last days draws someone to it, and a
    // planting wants watering every other day. It is read from the World as
    // it stands, never from its history, so a World opened from a
    // checkpoint lives the same day.
    for thing in made(state) {
        if rising(state, thing).is_some() {
            continue;
        }
        let Some(used_as) = use_of(thing).filter(|used| matches!(used, Use::Seat | Use::Planting))
        else {
            continue;
        };
        let since = integer(state, thing, "hands.since").unwrap_or(0).max(0) as u64;
        let new = today.saturating_sub(since) <= DRAWS_FOR;
        let watered = used_as == Use::Planting && (today + thing.0).is_multiple_of(2);
        if !(new || watered) {
            continue;
        }
        take(
            &mut free,
            &[thing.0, today, 0x5ea7],
            &far,
            &mut errands,
            Errand {
                at: thing,
                doing: doing_of(used_as),
            },
        );
    }
    errands
}

/// Gives whoever still free spends their day nearest the errand's place
/// the errand; of those as near, one picked by `salt`. Someone `far` says
/// nothing of (out of reach) is never sent to spend time there, though a
/// load can be carried over from anywhere.
fn take(
    free: &mut Vec<EntityId>,
    salt: &[u64],
    far: &impl Fn(EntityId, EntityId) -> Option<f32>,
    errands: &mut BTreeMap<EntityId, Errand>,
    errand: Errand,
) {
    let picked = free
        .iter()
        .enumerate()
        .filter_map(|(at, who)| {
            let away = match far(*who, errand.at) {
                Some(away) => away,
                None if errand.doing == days::Doing::Haul => f32::MAX,
                None => return None,
            };
            Some((at, away, days::mix(&[salt, &[who.0]].concat())))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.2.cmp(&b.2)))
        .map(|(at, ..)| at);
    if let Some(at) = picked {
        errands.insert(free.remove(at), errand);
    }
}

/// Registers the rising Action.
pub(crate) fn register(
    registry: &mut ActionRegistry,
    kit: fn(&WorldState) -> Kit,
) -> Result<(), ActionError> {
    registry.register(Rises(kit))
}

/// What finishing a work on a plot changes ([`crate::mark::Finishes`]):
/// nothing about it says it is going up any more.
pub(crate) fn finished_changes(state: &WorldState, fixture: EntityId) -> Vec<StateChange> {
    let mut changes = vec![StateChange::RemoveComponent {
        entity: fixture,
        key: BUILDING.into(),
    }];
    if integer(state, fixture, RISING).is_some() {
        changes.push(StateChange::RemoveComponent {
            entity: fixture,
            key: RISING.into(),
        });
    }
    changes
}
