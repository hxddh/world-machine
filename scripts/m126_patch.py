from pathlib import Path
import re


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"missing expected text in {path}: {old[:120]!r}")
    p.write_text(text.replace(old, new, 1))


def bump_package_block(text: str, package: str) -> str:
    pattern = re.compile(r'(\[\[package\]\]\nname = "' + re.escape(package) + r'"\nversion = ")0\.14\.5("\n)')
    text, count = pattern.subn(r'\g<1>0.14.6\g<2>', text, count=1)
    if count != 1:
        raise SystemExit(f"expected one Cargo.lock package block for {package}, got {count}")
    return text


replace_once(
    "worlds/pocket-universe/src/lib.rs",
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.5";',
    'pub const POCKET_UNIVERSE_PACK_VERSION: &str = "0.14.6";',
)
replace_once(
    "worlds/pocket-universe/Cargo.toml",
    'version = "0.14.5"',
    'version = "0.14.6"',
)
replace_once(
    "apps/pocket-universe-pack/Cargo.toml",
    'version = "0.14.5"',
    'version = "0.14.6"',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'version: "0.14.5",',
    'version: "0.14.6",',
)
replace_once(
    "apps/world-machine-desktop/src/included_packs.rs",
    'assert_eq!(packs[0].pack.version, "0.14.5");',
    'assert_eq!(packs[0].pack.version, "0.14.6");',
)

lock = Path("Cargo.lock")
lock_text = lock.read_text()
lock_text = bump_package_block(lock_text, "pocket-universe")
lock_text = bump_package_block(lock_text, "pocket-universe-pack")
lock.write_text(lock_text)

projection = Path("worlds/pocket-universe/src/projection.rs")
text = projection.read_text()
text = text.replace(
    'use world_core::{Entity, Event, Value, World};',
    'use world_core::{Entity, EntityId, Event, StateChange, Value, World};',
    1,
)
text = text.replace(
    '    entity_title, inspectors_from_world, timeline_from_world, why_map_from_world, BriefingItem,\n',
    '    entity_title, inspectors_from_world, timeline_from_world, value_text, why_map_from_world, BriefingItem,\n',
    1,
)
text = text.replace(
    '    LEGACY_CYCLES, LEGACY_SUMMARY, NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND, POSTURE, RELATIONSHIP,\n',
    '    LEGACY_CYCLES, LEGACY_SUMMARY, NUDGE_COMMAND, OUTWARD_POSTURE_COMMAND, POSTURE, POSTURE_GENERATION, RELATIONSHIP,\n',
    1,
)

old_persistent = '''fn persistent_consequence_items(world: &World) -> Vec<BriefingItem> {\n    let mut items = Vec::new();\n    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");\n'''
new_persistent = '''fn persistent_consequence_items(world: &World) -> Vec<BriefingItem> {\n    let mut items = Vec::new();\n    if let Some(item) = choice_evidence_item(world) {\n        items.push(item);\n    }\n    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");\n'''
if old_persistent not in text:
    raise SystemExit("persistent consequence insertion point missing")
text = text.replace(old_persistent, new_persistent, 1)

marker = 'fn intervention_influence_copy(decision: &str) -> Option<(&\'static str, &\'static str)> {'
if marker not in text:
    raise SystemExit("intervention influence marker missing")
helpers = r'''fn choice_evidence_item(world: &World) -> Option<BriefingItem> {
    let event_index = world.events().iter().rposition(|event| {
        matches!(
            event.kind.as_str(),
            "relationship_steered" | "universe_intervened" | "world_posture_chosen"
        )
    })?;
    let event = &world.events()[event_index];
    match event.kind.as_str() {
        "relationship_steered" => relationship_choice_evidence(world, event, event_index),
        "universe_intervened" => intervention_choice_evidence(world, event),
        "world_posture_chosen" => posture_choice_evidence(event),
        _ => None,
    }
}

fn relationship_choice_evidence(
    world: &World,
    event: &Event,
    event_index: usize,
) -> Option<BriefingItem> {
    let direction = payload_text(event, "direction")?;
    let before_trust = integer_value(component_value_before_event(
        world.events(),
        event_index,
        RELATIONSHIP,
        RELATIONSHIP_TRUST,
    )?)?;
    let before_tension = integer_value(component_value_before_event(
        world.events(),
        event_index,
        RELATIONSHIP,
        RELATIONSHIP_TENSION,
    )?)?;
    let after_trust = event_integer_component(event, RELATIONSHIP, RELATIONSHIP_TRUST)?;
    let after_tension = event_integer_component(event, RELATIONSHIP, RELATIONSHIP_TENSION)?;
    let durable_direction = event_text_component(event, RELATIONSHIP, RELATIONSHIP_DIRECTION)?;
    let (label, follow_on) = match direction {
        "shared-project" => (
            "Shared project",
            "Later relationship shifts read this durable direction and add +1 trust and -1 tension.",
        ),
        "rivalry" => (
            "Rivalry",
            "Later relationship shifts read this durable direction and add +1 tension.",
        ),
        _ => (
            "Relationship",
            "Later relationship shifts continue reading this durable direction.",
        ),
    };
    Some(BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Choice evidence · {label}"),
        detail: format!(
            "Verified by this Event: trust {before_trust} → {after_trust} · tension {before_tension} → {after_tension}. Durable direction = {}. {follow_on}",
            durable_direction.replace('-', " ")
        ),
    })
}

fn intervention_choice_evidence(world: &World, event: &Event) -> Option<BriefingItem> {
    let decision = event_text_component(event, UNIVERSE, DECISION)?;
    let label = intervention_influence_copy(&decision)
        .map(|(title, _)| {
            title
                .strip_prefix("Your influence · ")
                .unwrap_or(title)
                .to_string()
        })
        .unwrap_or_else(|| legacy_label(&decision));
    let effect = event.changes.iter().find_map(|change| match change {
        StateChange::SetComponent {
            entity,
            key,
            value,
        } if *entity != UNIVERSE => {
            let target = world
                .state()
                .entity(*entity)
                .map(entity_title)
                .unwrap_or_else(|| format!("Entity #{entity}"));
            Some(format!(
                "{target} · {} = {}",
                key.replace('_', " "),
                value_text(value, world)
            ))
        }
        _ => None,
    })?;
    Some(BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Choice evidence · {label}"),
        detail: format!(
            "Verified by this Event: first intervention = {label}; {effect}. Later growth reads this durable intervention."
        ),
    })
}

fn posture_choice_evidence(event: &Event) -> Option<BriefingItem> {
    let posture = event_text_component(event, UNIVERSE, POSTURE)?;
    let generation = event_integer_component(event, UNIVERSE, POSTURE_GENERATION)?;
    let label = match posture.as_str() {
        "outward" => "Outward",
        "rooted" => "Rooted",
        _ => "World direction",
    };
    Some(BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title: format!("Choice evidence · {label}"),
        detail: format!(
            "Verified by this Event: World direction = {label} at generation {generation}. Later growth and legacy formation read this durable posture."
        ),
    })
}

fn payload_text<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    match event.payload.get(key) {
        Some(Value::Text(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn event_component_value(event: &Event, entity: EntityId, key: &str) -> Option<Value> {
    event.changes.iter().rev().find_map(|change| match change {
        StateChange::SetComponent {
            entity: changed_entity,
            key: changed_key,
            value,
        } if *changed_entity == entity && changed_key == key => Some(value.clone()),
        StateChange::RemoveComponent {
            entity: changed_entity,
            key: changed_key,
        } if *changed_entity == entity && changed_key == key => Some(Value::Null),
        _ => None,
    })
}

fn event_integer_component(event: &Event, entity: EntityId, key: &str) -> Option<i64> {
    integer_value(event_component_value(event, entity, key)?)
}

fn event_text_component(event: &Event, entity: EntityId, key: &str) -> Option<String> {
    match event_component_value(event, entity, key)? {
        Value::Text(value) => Some(value),
        _ => None,
    }
}

fn integer_value(value: Value) -> Option<i64> {
    match value {
        Value::Integer(value) => Some(value),
        _ => None,
    }
}

fn component_value_before_event(
    events: &[Event],
    event_index: usize,
    entity_id: EntityId,
    key: &str,
) -> Option<Value> {
    let mut current = None;
    for event in &events[..event_index] {
        for change in &event.changes {
            match change {
                StateChange::CreateEntity(entity) if entity.id == entity_id => {
                    current = entity.component(key).cloned();
                }
                StateChange::RemoveEntity(entity) if *entity == entity_id => current = None,
                StateChange::SetComponent {
                    entity,
                    key: changed_key,
                    value,
                } if *entity == entity_id && changed_key == key => {
                    current = Some(value.clone());
                }
                StateChange::RemoveComponent {
                    entity,
                    key: changed_key,
                } if *entity == entity_id && changed_key == key => current = None,
                _ => {}
            }
        }
    }
    current
}

'''
text = text.replace(marker, helpers + marker, 1)
projection.write_text(text)

choice_test = r'''use pocket_universe::{
    PocketUniverse, BOLD_PATH_COMMAND, OUTWARD_POSTURE_COMMAND, SEED_MARS_COLONY_COMMAND,
    SHARED_PROJECT_COMMAND,
};
use std::error::Error;
use world_core::{StateChange, Value};
use world_projection::SelectionId;

fn choice_evidence<'a>(
    snapshot: &'a world_projection::ProjectionSnapshot,
    title: &str,
) -> &'a world_projection::BriefingItem {
    snapshot
        .briefing
        .as_ref()
        .expect("Pocket Universe should expose a Briefing")
        .items
        .iter()
        .find(|item| item.title == title)
        .unwrap_or_else(|| panic!("missing choice evidence item: {title}"))
}

#[test]
fn relationship_choice_signal_is_verified_by_the_recorded_event() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;

    let before = universe.projection_snapshot();
    let signal = before
        .command(SHARED_PROJECT_COMMAND)
        .expect("shared project should be available")
        .detail
        .clone();
    assert!(signal.contains("trust 2 → 4 · tension 0 → 0"));

    let event_id = universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    let event = universe
        .world()
        .events()
        .iter()
        .find(|event| event.id == event_id)
        .expect("relationship choice should record an Event");
    assert_eq!(event.kind, "relationship_steered");
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Integer(4), .. } if key == "trust"
    )));
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Integer(0), .. } if key == "tension"
    )));
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Text(value), .. }
            if key == "direction" && value == "shared-project"
    )));

    let after = universe.projection_snapshot();
    let evidence = choice_evidence(&after, "Choice evidence · Shared project");
    assert_eq!(evidence.selection, Some(SelectionId::Event(event_id)));
    assert!(evidence.detail.contains("trust 2 → 4 · tension 0 → 0"));
    assert!(evidence.detail.contains("Durable direction = shared project"));
    assert!(evidence.detail.contains("add +1 trust and -1 tension"));
    assert!(after.inspector(SelectionId::Event(event_id)).is_some());
    assert!(after.why(event_id).is_some());

    Ok(())
}

#[test]
fn intervention_choice_evidence_uses_the_event_statechange_not_current_copy() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(3)?;

    let before = universe.projection_snapshot();
    let signal = before
        .command(BOLD_PATH_COMMAND)
        .expect("bold intervention should be available")
        .detail
        .clone();
    assert!(signal.contains("Kestrel's durable status becomes signal expedition"));

    let event_id = universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    let event = universe
        .world()
        .events()
        .iter()
        .find(|event| event.id == event_id)
        .expect("intervention should record an Event");
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Text(value), .. }
            if key == "decision" && value == "follow-signal"
    )));
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Text(value), .. }
            if key == "status" && value == "signal expedition"
    )));

    let after = universe.projection_snapshot();
    let evidence = choice_evidence(&after, "Choice evidence · Signal expedition");
    assert_eq!(evidence.selection, Some(SelectionId::Event(event_id)));
    assert!(evidence.detail.contains("first intervention = Signal expedition"));
    assert!(evidence.detail.contains("Kestrel Rover · status = signal expedition"));
    assert!(evidence.detail.contains("Later growth reads this durable intervention"));

    Ok(())
}

#[test]
fn posture_choice_evidence_survives_archive_and_reopen() -> Result<(), Box<dyn Error>> {
    let mut universe = PocketUniverse::new()?;
    universe.invoke_projection_command(SEED_MARS_COLONY_COMMAND)?;
    universe.advance_periods(2)?;
    universe.invoke_projection_command(SHARED_PROJECT_COMMAND)?;
    universe.advance_periods(1)?;
    universe.invoke_projection_command(BOLD_PATH_COMMAND)?;
    universe.advance_periods(3)?;

    let before = universe.projection_snapshot();
    let signal = before
        .command(OUTWARD_POSTURE_COMMAND)
        .expect("outward posture should be available")
        .detail
        .clone();
    assert!(signal.contains("later growth and legacy formation read the outward posture"));

    let event_id = universe.invoke_projection_command(OUTWARD_POSTURE_COMMAND)?;
    let event = universe
        .world()
        .events()
        .iter()
        .find(|event| event.id == event_id)
        .expect("posture choice should record an Event");
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Text(value), .. }
            if key == "posture" && value == "outward"
    )));
    assert!(event.changes.iter().any(|change| matches!(
        change,
        StateChange::SetComponent { key, value: Value::Integer(6), .. }
            if key == "posture_generation"
    )));

    let after = universe.projection_snapshot();
    let evidence = choice_evidence(&after, "Choice evidence · Outward");
    assert_eq!(evidence.selection, Some(SelectionId::Event(event_id)));
    assert!(evidence.detail.contains("World direction = Outward at generation 6"));
    assert!(evidence.detail.contains("Later growth and legacy formation read this durable posture"));

    let archive = universe.archive()?;
    let reopened = PocketUniverse::resume_archive(&archive)?;
    assert_eq!(
        reopened.projection_snapshot(),
        after,
        "choice evidence should be reconstructed from the immutable Event log after reopen"
    );

    Ok(())
}
'''
Path("worlds/pocket-universe/tests/choice_evidence.rs").write_text(choice_test)
