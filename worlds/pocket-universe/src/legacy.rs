use super::*;

const LEGACY_STATUS: &str = "legacy_status";

pub(crate) fn register_actions(actions: &mut ActionRegistry) -> Result<(), ActionError> {
    actions.register(ResolveLegacy)?;
    Ok(())
}

pub(crate) fn resolve_period_consequences(
    world: &mut World,
    actions: &ActionRegistry,
    relationship: EventId,
) -> Result<EventId, Box<dyn Error>> {
    let mut tail = relationship;
    if social_arc_candidate(world.state())?.is_some() {
        tail = world
            .execute(
                actions,
                &ActionRequest::new("resolve_social_arc").caused_by(tail),
            )?
            .id;
    }
    if legacy_candidate(world.state())?.is_some() {
        let mut request = ActionRequest::new("resolve_legacy").caused_by(tail);
        for cause in historical_causes(world) {
            if cause != tail {
                request = request.caused_by(cause);
            }
        }
        tail = world.execute(actions, &request)?.id;
    }
    Ok(tail)
}

pub(crate) fn legacy_id_from_state(state: &WorldState) -> Result<String, ActionError> {
    match state
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(LEGACY))
    {
        Some(Value::Text(legacy)) => Ok(legacy.clone()),
        _ => Err(ActionError::Invalid(
            "Pocket Universe legacy state is missing".into(),
        )),
    }
}

pub(crate) fn growth_consequence(seed: &str, legacy: &str) -> Option<&'static str> {
    if legacy == "forming" {
        return None;
    }
    Some(match (seed, legacy) {
        ("mars-colony", "ridge-network") => {
            "The ridge network now behaves like an institution rather than a temporary expedition."
        }
        ("mars-colony", "competing-frontiers") => {
            "The competing frontier routes now shape who gets to define the colony's next edge."
        }
        ("mars-colony", "habitat-commons") => {
            "The habitat commons now shapes ordinary life inside Ares."
        }
        ("mars-colony", "sealed-districts") => {
            "The sealed districts now shape how safety and trust are distributed inside Ares."
        }
        ("1980s-town", "night-network") => {
            "The night network now links Maple Street's institutions beyond any one event."
        }
        ("1980s-town", "rival-scenes") => {
            "The rival scenes now shape where Maple Street's late-night life gathers."
        }
        ("1980s-town", "neighborhood-commons") => {
            "The neighborhood commons now carries routines that outlast any one organizer."
        }
        ("1980s-town", "split-blocks") => {
            "The split blocks now shape which local rituals belong to whom."
        }
        ("penguin-civilization", "aurora-league") => {
            "The aurora league now coordinates routes that outlast any one watch."
        }
        ("penguin-civilization", "rival-routes") => {
            "The rival routes now shape which colonies cooperate under the aurora."
        }
        ("penguin-civilization", "winter-commons") => {
            "The winter commons now carries local systems through each dark season."
        }
        ("penguin-civilization", "divided-houses") => {
            "The divided houses now shape how Icebridge governs winter life."
        }
        _ => "The World now carries a legacy formed from its earlier choices and repeated behavior.",
    })
}

fn historical_causes(world: &World) -> Vec<EventId> {
    let mut causes = Vec::new();
    for kind in [
        "world_posture_chosen",
        "partnership_formed",
        "relationship_fractured",
        "universe_intervened",
    ] {
        if let Some(event) = world.events().iter().rev().find(|event| event.kind == kind) {
            if !causes.contains(&event.id) {
                causes.push(event.id);
            }
        }
    }
    causes
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LegacyCandidate {
    id: &'static str,
    summary: String,
    target: EntityId,
    status_value: &'static str,
    posture: String,
    social_arc: String,
    decision: String,
    behavior: &'static str,
}

fn legacy_candidate(state: &WorldState) -> Result<Option<LegacyCandidate>, ActionError> {
    if legacy_id_from_state(state)? != "forming" {
        return Ok(None);
    }
    let posture = posture_id_from_state(state)?;
    if posture == "none" {
        return Ok(None);
    }
    let social_arc = text_component_from_state(state, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC)?;
    if social_arc == "forming" {
        return Ok(None);
    }
    let decision = decision_id_from_state(state)?;
    if decision == "none" {
        return Ok(None);
    }
    let generation = integer_component(state, UNIVERSE, GENERATION)?;
    let posture_generation = integer_component(state, UNIVERSE, POSTURE_GENERATION)?;
    if posture_generation <= 0 || generation < posture_generation + 3 {
        return Ok(None);
    }

    let seed = seed_id_from_state(state)?;
    let care = integer_component(state, SLOT_B, AGENT_CARE_COUNT)?
        + integer_component(state, SLOT_E, AGENT_CARE_COUNT)?;
    let explore = integer_component(state, SLOT_B, AGENT_EXPLORE_COUNT)?
        + integer_component(state, SLOT_E, AGENT_EXPLORE_COUNT)?;
    let behavior = if care > explore {
        "care-led"
    } else if explore > care {
        "explore-led"
    } else {
        "balanced"
    };

    let (id, target, status_value, base) =
        archetype(&seed, &posture, &social_arc).ok_or_else(|| {
            ActionError::Invalid(format!(
                "unsupported Pocket Universe legacy: seed={seed}, posture={posture}, social_arc={social_arc}"
            ))
        })?;
    let summary = format!(
        "{base} {} Across its two central actors, the durable pattern is now {behavior} ({care} care / {explore} explore).",
        decision_memory(&decision)
    );
    Ok(Some(LegacyCandidate {
        id,
        summary,
        target,
        status_value,
        posture,
        social_arc,
        decision,
        behavior,
    }))
}

fn archetype(
    seed: &str,
    posture: &str,
    social_arc: &str,
) -> Option<(&'static str, EntityId, &'static str, &'static str)> {
    Some(match (seed, posture, social_arc) {
        ("mars-colony", "outward", "partnership") => (
            "ridge-network",
            SLOT_D,
            "shared ridge network",
            "Ares stopped treating every expedition as an exception; its shared rover routes have become a ridge network.",
        ),
        ("mars-colony", "outward", "fracture") => (
            "competing-frontiers",
            SLOT_D,
            "competing frontier routes",
            "Ares kept reaching beyond the ridge, but the split between its explorers hardened into competing frontiers.",
        ),
        ("mars-colony", "rooted", "partnership") => (
            "habitat-commons",
            SLOT_A,
            "habitat commons",
            "Ares turned repeated shared upkeep into a habitat commons that now organizes life at home.",
        ),
        ("mars-colony", "rooted", "fracture") => (
            "sealed-districts",
            SLOT_A,
            "sealed habitat districts",
            "Ares kept deepening its home, but the unresolved split hardened that safety into sealed habitat districts.",
        ),
        ("1980s-town", "outward", "partnership") => (
            "night-network",
            SLOT_C,
            "shared night network",
            "Maple Street's late-night experiments have become a shared network linking radio, arcade, bus, and newcomers.",
        ),
        ("1980s-town", "outward", "fracture") => (
            "rival-scenes",
            SLOT_C,
            "rival late-night scenes",
            "Maple Street kept drawing a wider crowd, but its central fracture hardened into rival late-night scenes.",
        ),
        ("1980s-town", "rooted", "partnership") => (
            "neighborhood-commons",
            SLOT_A,
            "neighborhood commons",
            "Maple Street turned repeated local rituals into a neighborhood commons held together by shared stewardship.",
        ),
        ("1980s-town", "rooted", "fracture") => (
            "split-blocks",
            SLOT_A,
            "split neighborhood blocks",
            "Maple Street stayed deliberately local, but its central fracture settled into split blocks with different loyalties.",
        ),
        ("penguin-civilization", "outward", "partnership") => (
            "aurora-league",
            SLOT_D,
            "aurora league",
            "Icebridge's widening routes and shared watch have become an aurora league between colonies.",
        ),
        ("penguin-civilization", "outward", "fracture") => (
            "rival-routes",
            SLOT_D,
            "rival colony routes",
            "Icebridge kept widening its reach, but the colony split hardened into rival routes under the aurora.",
        ),
        ("penguin-civilization", "rooted", "partnership") => (
            "winter-commons",
            SLOT_A,
            "winter commons",
            "Icebridge turned shared winter work into a commons that now coordinates local life through the dark season.",
        ),
        ("penguin-civilization", "rooted", "fracture") => (
            "divided-houses",
            SLOT_D,
            "divided winter houses",
            "Icebridge deepened its winter systems, but the colony's fracture settled into divided houses at moonrise.",
        ),
        _ => return None,
    })
}

fn decision_memory(decision: &str) -> &'static str {
    match decision {
        "follow-signal" => {
            "The original signal expedition remains part of how the colony explains why it changed."
        }
        "fortify-habitat" => {
            "The original decision to fortify the habitat remains part of how the colony explains what safety means."
        }
        "community-arcade" => {
            "The decision to make the arcade communal remains part of Maple Street's memory."
        }
        "steady-business" => {
            "The decision to protect a small steady business remains part of Maple Street's memory."
        }
        "winter-feast" => {
            "The winter feast remains part of Icebridge's memory of when distant colonies first felt close."
        }
        "conserve-reserves" => {
            "The decision to conserve the Fish Vault remains part of Icebridge's memory of surviving the dark season."
        }
        _ => "The first intervention remains part of how this World explains what it became.",
    }
}

struct ResolveLegacy;

impl Action for ResolveLegacy {
    fn name(&self) -> &'static str {
        "resolve_legacy"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let candidate = legacy_candidate(state)?.ok_or_else(|| {
            ActionError::Invalid("this World has not lived long enough for a legacy to form".into())
        })?;
        let mut draft = EventDraft::new("world_legacy_formed");
        draft.targets = vec![UNIVERSE, RELATIONSHIP, SLOT_B, SLOT_E, candidate.target];
        draft.payload.insert("legacy".into(), candidate.id.into());
        draft
            .payload
            .insert("posture".into(), candidate.posture.into());
        draft
            .payload
            .insert("social_arc".into(), candidate.social_arc.into());
        draft
            .payload
            .insert("decision".into(), candidate.decision.into());
        draft
            .payload
            .insert("behavior".into(), candidate.behavior.into());
        draft
            .payload
            .insert("summary".into(), candidate.summary.clone().into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LEGACY.into(),
                value: candidate.id.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LEGACY_SUMMARY.into(),
                value: candidate.summary.clone().into(),
            },
            StateChange::SetComponent {
                entity: candidate.target,
                key: LEGACY_STATUS.into(),
                value: candidate.status_value.into(),
            },
            StateChange::SetComponent {
                entity: UNIVERSE,
                key: LAST_CHANGE.into(),
                value: candidate.summary.into(),
            },
        ];
        Ok(draft)
    }
}
