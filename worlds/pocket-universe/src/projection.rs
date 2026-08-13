use crate::{
    seed_id, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, DECISION, GENERATION, LAST_CHANGE,
    NUDGE_COMMAND, RELATIONSHIP, RELATIONSHIP_DIRECTION, RIVALRY_COMMAND, SEED_1980S_TOWN_COMMAND,
    SEED_MARS_COLONY_COMMAND, SEED_PENGUIN_CIVILIZATION_COMMAND, SHARED_PROJECT_COMMAND, UNIVERSE,
};
use world_core::{Entity, Event, Value, World};
use world_projection::{
    entity_title, inspectors_from_world, timeline_from_world, why_map_from_world, BriefingItem,
    BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem,
    CollectionProjection, ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot,
    SelectionId,
};

pub(crate) fn snapshot(world: &World) -> ProjectionSnapshot {
    snapshot_since(world, None)
}

pub(crate) fn snapshot_since(
    world: &World,
    since_event_count: Option<usize>,
) -> ProjectionSnapshot {
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
        briefing: Some(briefing(world, seeded, since_event_count)),
        commands: commands(world, seeded),
        collection: collection(world),
        timeline: timeline_from_world(world),
        canvas: canvas(world),
        inspectors: inspectors_from_world(world),
        why: why_map_from_world(world),
    }
}

fn commands(world: &World, seeded: bool) -> Vec<ProjectionCommand> {
    if !seeded {
        return vec![
            ProjectionCommand {
                id: SEED_MARS_COLONY_COMMAND.into(),
                title: "Start a Mars colony".into(),
                detail: "A tiny habitat, one keeper, hydroponics, and a rover on a red horizon."
                    .into(),
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
        ];
    }

    let mut commands = vec![ProjectionCommand {
        id: NUDGE_COMMAND.into(),
        title: "Nudge the world".into(),
        detail: "Let one small, persistent change happen now.".into(),
    }];
    let generation = integer_component(world, GENERATION).unwrap_or_default();
    let relationship_direction = text_component(
        world.state().entity(RELATIONSHIP),
        RELATIONSHIP_DIRECTION,
        "none",
    );
    if generation >= 2 && relationship_direction == "none" {
        commands.push(ProjectionCommand {
            id: SHARED_PROJECT_COMMAND.into(),
            title: "Give them a shared project".into(),
            detail: "Create a goal that neither actor can complete alone; future interactions will lean toward trust.".into(),
        });
        commands.push(ProjectionCommand {
            id: RIVALRY_COMMAND.into(),
            title: "Let rivalry sharpen them".into(),
            detail: "Keep both actors independent and let competition add pressure to future interactions.".into(),
        });
    }
    let decision = text_component(world.state().entity(UNIVERSE), DECISION, "none");
    if generation >= 3 && decision == "none" {
        let (bold_title, bold_detail, careful_title, careful_detail) =
            intervention_copy(seed_id(world));
        commands.push(ProjectionCommand {
            id: BOLD_PATH_COMMAND.into(),
            title: bold_title.into(),
            detail: bold_detail.into(),
        });
        commands.push(ProjectionCommand {
            id: CAREFUL_PATH_COMMAND.into(),
            title: careful_title.into(),
            detail: careful_detail.into(),
        });
    }
    commands
}

fn briefing(world: &World, seeded: bool, since_event_count: Option<usize>) -> BriefingProjection {
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

    if let Some(since) = since_event_count.filter(|since| *since < world.events().len()) {
        let events = &world.events()[since..];
        return BriefingProjection {
            eyebrow: format!("Pocket Universe · {}", seed_label(seed_id(world))),
            title: "While you were away".into(),
            items: events
                .iter()
                .rev()
                .filter(|event| event.kind != "agent_decision_recorded")
                .take(3)
                .map(return_item)
                .collect(),
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
            title: "Current thread".into(),
            detail: last_change,
        }],
    }
}

fn return_item(event: &Event) -> BriefingItem {
    let detail = ["change", "summary"]
        .into_iter()
        .find_map(|key| match event.payload.get(key) {
            Some(Value::Text(value)) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| event.kind.replace('_', " "));
    BriefingItem {
        selection: Some(SelectionId::Event(event.id)),
        title: match event.kind.as_str() {
            "universe_grew" => "The world moved".into(),
            "universe_intervened" => "Your choice took hold".into(),
            "universe_seeded" => "A world began".into(),
            "agent_cared_for_world" => "Someone cared for the world".into(),
            "agent_explored_world" => "Someone explored beyond routine".into(),
            "relationship_shifted" => "Their relationship changed".into(),
            "relationship_steered" => "You steered their relationship".into(),
            _ => event.kind.replace('_', " "),
        },
        detail,
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
    const POSITIONS: [(f32, f32); 6] = [
        (0.14, 0.24),
        (0.72, 0.22),
        (0.16, 0.78),
        (0.78, 0.74),
        (0.50, 0.48),
        (0.50, 0.82),
    ];
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

fn intervention_copy(seed: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    match seed {
        "mars-colony" => (
            "Follow the rover signal",
            "Send Kestrel beyond the safe ridge after a repeating signal.",
            "Fortify Ares Habitat",
            "Spend the colony's spare capacity sealing the habitat before the next dust front.",
        ),
        "1980s-town" => (
            "Make the arcade a community hub",
            "Keep Maple Arcade open late as a neighborhood club.",
            "Keep the arcade a steady business",
            "Protect its small cash buffer and avoid becoming the town's unofficial clubhouse.",
        ),
        "penguin-civilization" => (
            "Open the Fish Vault for a feast",
            "Invite distant colonies across Icebridge for a winter feast.",
            "Conserve the winter reserves",
            "Keep the Fish Vault sealed and plan for the dark season.",
        ),
        _ => (
            "Take the bold path",
            "Choose a visible change with uncertain consequences.",
            "Take the careful path",
            "Protect what already exists and reduce immediate risk.",
        ),
    }
}
