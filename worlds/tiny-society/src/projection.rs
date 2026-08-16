use crate::{
    commands::{command_catalog, OPEN_POP_UP, WAIT_10_MINUTES},
    pack::TinySocietyPack,
    resident_ids, FIXTURE_CAFE, START_TIME,
};
use world_core::{Entity, EntityId, Value, World};
use world_projection::{
    direct_effects_from_world, inspectors_from_world, timeline_from_world, why_map_from_world,
    BriefingItem, BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem,
    CollectionProjection, ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot, SelectionId,
};

pub fn snapshot(world: &World) -> ProjectionSnapshot {
    let catalog = command_catalog();
    let residents = resident_ids(world);
    ProjectionSnapshot {
        title: "Tiny Society".into(),
        world_time: world.world_time(),
        capabilities: ProjectionCapabilities { fork: true },
        briefing: briefing(world),
        commands: catalog
            .iter()
            .map(|command| ProjectionCommand {
                id: command.id.clone(),
                title: command.title.clone(),
                detail: command.detail.clone(),
            })
            .collect(),
        collection: CollectionProjection {
            title: "Residents".into(),
            items: residents
                .iter()
                .filter_map(|id| resident_collection_item(world, *id))
                .collect(),
        },
        timeline: timeline_from_world(world),
        direct_effects: direct_effects_from_world(world),
        canvas: CanvasProjection {
            items: residents
                .iter()
                .enumerate()
                .filter_map(|(index, id)| resident_canvas_item(world, *id, index))
                .collect(),
        },
        inspectors: inspectors_from_world(world),
        why: why_map_from_world(world),
    }
}

fn briefing(world: &World) -> Option<BriefingProjection> {
    let cafe = world.state().entity(FIXTURE_CAFE)?;
    let open = cafe.component("open").and_then(Value::as_bool).unwrap_or(false);
    let earliest = START_TIME + 8 * 60;
    let title = if world.world_time() < earliest {
        "The café is preparing to open."
    } else if open {
        "The café is open and the neighborhood is moving."
    } else {
        "The café has not opened yet."
    };

    let mut items = Vec::new();
    if let Some(selected) = selected_resident(world) {
        items.push(BriefingItem {
            selection: Some(SelectionId::Entity(selected)),
            title: entity_name(world, selected),
            detail: "Currently selected resident".into(),
        });
    }
    items.push(BriefingItem {
        selection: Some(SelectionId::Entity(FIXTURE_CAFE)),
        title: "Neighborhood Café".into(),
        detail: if open {
            "Open for the day".into()
        } else {
            "Closed".into()
        },
    });
    items.push(BriefingItem {
        selection: None,
        title: "Available actions".into(),
        detail: if world.world_time() < earliest {
            format!("{} or {}", WAIT_10_MINUTES, OPEN_POP_UP)
        } else {
            "Continue the world through the available commands.".into()
        },
    });

    Some(BriefingProjection {
        eyebrow: "Neighborhood Briefing".into(),
        title: title.into(),
        items,
    })
}

fn resident_collection_item(world: &World, id: EntityId) -> Option<CollectionItem> {
    let resident = world.state().entity(id)?;
    Some(CollectionItem {
        id: SelectionId::Entity(id),
        title: entity_name(world, id),
        subtitle: resident
            .component("job")
            .and_then(Value::as_text)
            .unwrap_or(&resident.kind)
            .to_string(),
    })
}

fn resident_canvas_item(world: &World, id: EntityId, index: usize) -> Option<CanvasItem> {
    let resident = world.state().entity(id)?;
    let place = resident.component("place").and_then(Value::as_entity);
    let (x, y) = match place {
        Some(place) if place == FIXTURE_CAFE => (0.56, 0.48),
        Some(_) => (0.28 + index as f32 * 0.06, 0.3 + index as f32 * 0.08),
        None => (0.42 + index as f32 * 0.04, 0.66),
    };
    Some(CanvasItem {
        id: SelectionId::Entity(id),
        kind: CanvasItemKind::Actor,
        label: entity_name(world, id),
        detail: resident
            .component("job")
            .and_then(Value::as_text)
            .unwrap_or("resident")
            .into(),
        x,
        y,
    })
}

fn selected_resident(world: &World) -> Option<EntityId> {
    world
        .state()
        .entities()
        .find(|entity| {
            entity.kind == "resident"
                && entity
                    .component("selected")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .map(|entity| entity.id)
}

fn entity_name(world: &World, id: EntityId) -> String {
    world
        .state()
        .entity(id)
        .map(entity_name_from_entity)
        .unwrap_or_else(|| format!("Entity #{id}"))
}

fn entity_name_from_entity(entity: &Entity) -> String {
    entity
        .component("name")
        .and_then(Value::as_text)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} #{}", entity.kind, entity.id))
}

pub fn project(world: &World) -> ProjectionSnapshot {
    snapshot(world)
}

pub fn project_pack(world: &World) -> ProjectionSnapshot {
    TinySocietyPack.project(world)
}
