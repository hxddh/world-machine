use crate::{
    BAKERY, EMMA, EVAN, HARBOR, JONAS, JONAS_BOAT, LEO, MARA, MIA, NOAH, PUB, SCHOOL, SOFIA,
    WEDDING_ORDER,
};
use society_basic::{CASH, JOB};
use world_core::{EntityId, Value, World};
use world_projection::{
    entity_title, inspectors_from_world, timeline_from_world, why_map_from_world, BriefingItem,
    BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem,
    CollectionProjection, ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot,
    SelectionId,
};

const RESIDENTS: [EntityId; 8] = [JONAS, MARA, LEO, EMMA, MIA, NOAH, EVAN, SOFIA];

pub(crate) fn snapshot(world: &World) -> ProjectionSnapshot {
    snapshot_since(world, None)
}

pub(crate) fn snapshot_since(
    world: &World,
    since_event_count: Option<usize>,
) -> ProjectionSnapshot {
    ProjectionSnapshot {
        title: "Tiny Society".into(),
        world_time: world.world_time(),
        capabilities: ProjectionCapabilities { fork: true },
        briefing: Some(society_briefing(world, since_event_count)),
        commands: available_commands(world),
        collection: CollectionProjection {
            title: "Residents".into(),
            items: RESIDENTS
                .iter()
                .filter_map(|id| resident_item(world, *id))
                .collect(),
        },
        timeline: timeline_from_world(world),
        canvas: CanvasProjection {
            items: canvas_items(world),
        },
        inspectors: inspectors_from_world(world),
        why: why_map_from_world(world),
    }
}

fn available_commands(world: &World) -> Vec<ProjectionCommand> {
    let has_order_loss = world
        .events()
        .iter()
        .any(|event| event.kind == "order_lost");
    let has_dismissal = world
        .events()
        .iter()
        .any(|event| event.kind == "worker_dismissed");
    let has_retention = world
        .events()
        .iter()
        .any(|event| event.kind == "worker_retained");
    let jonas_is_temp = component_text(world, JONAS, JOB).as_deref() == Some("bakery_temp");

    if has_order_loss && !has_dismissal && !has_retention && jonas_is_temp {
        vec![ProjectionCommand {
            id: crate::RETAIN_WORKER_COMMAND.into(),
            title: "Give Jonas another chance".into(),
            detail:
                "Keep Jonas at the bakery and let this branch continue into a different future."
                    .into(),
        }]
    } else {
        Vec::new()
    }
}

fn society_briefing(world: &World, since_event_count: Option<usize>) -> BriefingProjection {
    let start = since_event_count.unwrap_or(0).min(world.events().len());
    let relevant_events = if since_event_count.is_some() {
        &world.events()[start..]
    } else {
        world.events()
    };

    let mut items = relevant_events
        .iter()
        .rev()
        .filter_map(|event| {
            let title = match event.kind.as_str() {
                "work_shift_completed" if event.actor == Some(JONAS) => {
                    "Jonas completed another bakery shift"
                }
                "worker_retained" => "Mara gave Jonas another chance",
                "worker_dismissed" => "Mara dismissed Jonas",
                "order_lost" => "The bakery lost the wedding order",
                "temporary_work_assigned" => "Jonas took temporary work at the bakery",
                "loan_requested" => "Jonas asked Leo for a loan",
                "storm_started" => "A storm reached the harbor",
                _ => return None,
            };
            Some(BriefingItem {
                selection: Some(SelectionId::Event(event.id)),
                title: title.into(),
                detail: format!("World time {} · Event #{}", event.world_time, event.id),
            })
        })
        .take(4)
        .collect::<Vec<_>>();

    if since_event_count.is_some() && items.is_empty() {
        let (title, detail) = if relevant_events.is_empty() {
            (
                "No new events",
                "Nothing changed in the world since your last visit.".to_string(),
            )
        } else {
            (
                "The world moved forward",
                format!(
                    "{} new event(s) occurred, but none are highlighted in Society Today.",
                    relevant_events.len()
                ),
            )
        };
        items.push(BriefingItem {
            selection: None,
            title: title.into(),
            detail,
        });
    }

    BriefingProjection {
        eyebrow: "Society Today".into(),
        title: if since_event_count.is_some() {
            "While you were away".into()
        } else {
            "Life happened while you were away".into()
        },
        items,
    }
}

fn resident_item(world: &World, id: EntityId) -> Option<CollectionItem> {
    let entity = world.state().entity(id)?;
    let job = component_text(world, id, JOB).unwrap_or_else(|| "unknown job".into());
    let cash = component_text(world, id, CASH).unwrap_or_else(|| "?".into());
    Some(CollectionItem {
        id: SelectionId::Entity(id),
        title: entity_title(entity),
        subtitle: format!("{job} · cash {cash}"),
    })
}

fn canvas_items(world: &World) -> Vec<CanvasItem> {
    let mut items = Vec::new();

    for (id, x, y) in [
        (HARBOR, 0.08, 0.48),
        (BAKERY, 0.62, 0.18),
        (SCHOOL, 0.62, 0.66),
        (PUB, 0.28, 0.16),
    ] {
        if let Some(entity) = world.state().entity(id) {
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind: CanvasItemKind::Place,
                label: entity_title(entity),
                detail: "Place".into(),
                x,
                y,
            });
        }
    }

    for (id, x, y) in [
        (JONAS, 0.12, 0.62),
        (MARA, 0.68, 0.32),
        (LEO, 0.34, 0.28),
        (EMMA, 0.70, 0.74),
        (MIA, 0.82, 0.67),
        (NOAH, 0.20, 0.52),
        (EVAN, 0.04, 0.72),
        (SOFIA, 0.42, 0.12),
    ] {
        if let Some(entity) = world.state().entity(id) {
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind: CanvasItemKind::Actor,
                label: entity_title(entity),
                detail: component_text(world, id, JOB).unwrap_or_else(|| "Resident".into()),
                x,
                y,
            });
        }
    }

    for (id, x, y) in [(JONAS_BOAT, 0.02, 0.42), (WEDDING_ORDER, 0.84, 0.22)] {
        if let Some(entity) = world.state().entity(id) {
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind: CanvasItemKind::Object,
                label: entity_title(entity),
                detail: entity.kind.clone(),
                x,
                y,
            });
        }
    }

    items
}

fn component_text(world: &World, id: EntityId, key: &str) -> Option<String> {
    match world.state().entity(id)?.component(key)? {
        Value::Text(value) => Some(value.clone()),
        Value::Integer(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Entity(value) => world.state().entity(*value).map(entity_title),
        Value::Null | Value::List(_) | Value::Map(_) => None,
    }
}
