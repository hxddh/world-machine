use crate::{
    seed_id, GENERATION, LAST_CHANGE, NUDGE_COMMAND, SEED_1980S_TOWN_COMMAND,
    SEED_MARS_COLONY_COMMAND, SEED_PENGUIN_CIVILIZATION_COMMAND, UNIVERSE,
};
use world_core::{Entity, Value, World};
use world_projection::{
    entity_title, inspectors_from_world, timeline_from_world, why_map_from_world, BriefingItem,
    BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem,
    CollectionProjection, ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot,
    SelectionId,
};

pub(crate) fn snapshot(world: &World) -> ProjectionSnapshot {
    let seed = seed_id(world);
    let seeded = seed != "unseeded";
    ProjectionSnapshot {
        title: if seeded {
            universe_name(world)
        } else {
            "Pocket Universe · Empty World".into()
        },
        world_time: world.world_time(),
        capabilities: ProjectionCapabilities {
            fork: !world.events().is_empty(),
        },
        briefing: Some(briefing(world, seeded)),
        commands: commands(seeded),
        collection: collection(world),
        timeline: timeline_from_world(world),
        canvas: canvas(world),
        inspectors: inspectors_from_world(world),
        why: why_map_from_world(world),
    }
}

fn commands(seeded: bool) -> Vec<ProjectionCommand> {
    if seeded {
        return vec![ProjectionCommand {
            id: NUDGE_COMMAND.into(),
            title: "Nudge the world".into(),
            detail: "Let one small, persistent change happen now.".into(),
        }];
    }

    vec![
        ProjectionCommand {
            id: SEED_MARS_COLONY_COMMAND.into(),
            title: "Start a Mars colony".into(),
            detail: "A tiny habitat, one keeper, hydroponics, and a rover on a red horizon.".into(),
        },
        ProjectionCommand {
            id: SEED_1980S_TOWN_COMMAND.into(),
            title: "Start a town in 1987".into(),
            detail: "An arcade, local radio, a night bus, and a neighborhood that remembers."
                .into(),
        },
        ProjectionCommand {
            id: SEED_PENGUIN_CIVILIZATION_COMMAND.into(),
            title: "Start a penguin civilization".into(),
            detail: "An ice bridge, a fish vault, a moonrise council, and one bridge keeper."
                .into(),
        },
    ]
}

fn briefing(world: &World, seeded: bool) -> BriefingProjection {
    if !seeded {
        return BriefingProjection {
            eyebrow: "Pocket Universe".into(),
            title: "What kind of world should exist here?".into(),
            items: vec![
                BriefingItem {
                    selection: Some(SelectionId::Entity(UNIVERSE)),
                    title: "Create".into(),
                    detail: "Choose one seed. The choice becomes the first durable event in this World."
                        .into(),
                },
                BriefingItem {
                    selection: None,
                    title: "Keep · Grow · Return".into(),
                    detail: "Save it like a document, let time move, then come back to a world with history."
                        .into(),
                },
            ],
        };
    }

    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let last_change = text_component(
        world.state().entity(UNIVERSE),
        LAST_CHANGE,
        "The world is quiet.",
    );
    BriefingProjection {
        eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
        title: format!("Generation {generation}"),
        items: vec![BriefingItem {
            selection: Some(SelectionId::Entity(UNIVERSE)),
            title: "Since the last visit".into(),
            detail: last_change,
        }],
    }
}

fn collection(world: &World) -> CollectionProjection {
    CollectionProjection {
        title: "World Contents".into(),
        items: world
            .state()
            .entities()
            .filter(|entity| entity.id != UNIVERSE)
            .map(|entity| CollectionItem {
                id: SelectionId::Entity(entity.id),
                title: entity_title(entity),
                subtitle: entity.kind.replace('_', " "),
            })
            .collect(),
    }
}

fn canvas(world: &World) -> CanvasProjection {
    const POSITIONS: [(f32, f32); 4] = [(0.18, 0.30), (0.72, 0.26), (0.25, 0.74), (0.70, 0.70)];
    let items = world
        .state()
        .entities()
        .filter(|entity| entity.id != UNIVERSE)
        .enumerate()
        .map(|(index, entity)| {
            let (x, y) = POSITIONS[index.min(POSITIONS.len() - 1)];
            CanvasItem {
                id: SelectionId::Entity(entity.id),
                kind: canvas_kind(entity),
                label: entity_title(entity),
                detail: entity.kind.replace('_', " "),
                x,
                y,
            }
        })
        .collect();
    CanvasProjection { items }
}

fn canvas_kind(entity: &Entity) -> CanvasItemKind {
    match entity.kind.as_str() {
        "person" | "penguin" => CanvasItemKind::Actor,
        "place" | "habitat" | "colony" => CanvasItemKind::Place,
        _ => CanvasItemKind::Object,
    }
}

fn universe_name(world: &World) -> String {
    world
        .state()
        .entity(UNIVERSE)
        .map(entity_title)
        .unwrap_or_else(|| "Pocket Universe".into())
}

fn integer_component(world: &World, key: &str) -> Option<i64> {
    match world
        .state()
        .entity(UNIVERSE)
        .and_then(|entity| entity.component(key))
    {
        Some(Value::Integer(value)) => Some(*value),
        _ => None,
    }
}

fn text_component(entity: Option<&Entity>, key: &str, fallback: &str) -> String {
    match entity.and_then(|entity| entity.component(key)) {
        Some(Value::Text(value)) => value.clone(),
        _ => fallback.into(),
    }
}

fn seed_label(seed: &str) -> &'static str {
    match seed {
        "mars-colony" => "Mars Colony",
        "1980s-town" => "1987 Town",
        "penguin-civilization" => "Penguin Civilization",
        _ => "Unseeded",
    }
}
