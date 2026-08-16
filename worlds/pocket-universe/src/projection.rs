use crate::{
    command_catalog,
    pack::PocketUniversePack,
    CHOOSE_OUTWARD, CHOOSE_ROOTED, ENTITY_ARCHIVE, ENTITY_ATLAS, ENTITY_REACH, ENTITY_RESONANCE,
    ENTITY_TIDE, ENTITY_WORKSPACE, FIRST_SHIFT_TIME, RECORD_ATLAS, RECORD_RESONANCE, START_TIME,
};
use world_core::{EntityId, Value, World};
use world_projection::{
    direct_effects_from_world, inspectors_from_world, timeline_from_world, why_map_from_world,
    BriefingItem, BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem,
    CollectionProjection, InspectorProjection, InspectorRow, InspectorSection,
    ProjectionCapabilities, ProjectionCommand, ProjectionSnapshot, SelectionId,
};

pub fn snapshot(world: &World) -> ProjectionSnapshot {
    let catalog = command_catalog();
    ProjectionSnapshot {
        title: "Pocket Universe".into(),
        world_time: world.world_time(),
        capabilities: ProjectionCapabilities { fork: true },
        briefing: briefing(world),
        commands: catalog
            .into_iter()
            .map(|command| ProjectionCommand {
                id: command.id,
                title: command.title,
                detail: command.detail,
            })
            .collect(),
        collection: collection(world),
        timeline: timeline_from_world(world),
        direct_effects: direct_effects_from_world(world),
        canvas: canvas(world),
        inspectors: inspectors(world),
        why: why_map_from_world(world),
    }
}

fn briefing(world: &World) -> Option<BriefingProjection> {
    let workspace = world.state().entity(ENTITY_WORKSPACE)?;
    let mode = workspace
        .component("mode")
        .and_then(Value::as_text)
        .unwrap_or("undecided");
    let archived = workspace
        .component("archive_complete")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let title = if mode == "undecided" {
        "A tiny world is waiting for its first durable direction."
    } else if archived {
        "This tiny world has reached a durable ending."
    } else {
        "The chosen posture is reshaping this tiny world."
    };
    let next = if world.world_time() < FIRST_SHIFT_TIME {
        "The first signal shift is still ahead."
    } else if mode == "undecided" {
        "Choose the posture this world will carry forward."
    } else if !archived {
        "Record the world after its consequences become visible."
    } else {
        "The archive now holds the consequences of this posture."
    };

    Some(BriefingProjection {
        eyebrow: "World Briefing".into(),
        title: title.into(),
        items: vec![
            BriefingItem {
                selection: Some(SelectionId::Entity(ENTITY_WORKSPACE)),
                title: "World posture".into(),
                detail: humanize(mode),
            },
            BriefingItem {
                selection: Some(SelectionId::Entity(ENTITY_RESONANCE)),
                title: "Resonance".into(),
                detail: resonance_detail(world),
            },
            BriefingItem {
                selection: Some(SelectionId::Entity(ENTITY_ARCHIVE)),
                title: if archived {
                    "Archive complete".into()
                } else {
                    "Archive pending".into()
                },
                detail: archive_detail(world),
            },
            BriefingItem {
                selection: None,
                title: "What happens next".into(),
                detail: next.into(),
            },
        ],
    })
}

fn collection(world: &World) -> CollectionProjection {
    CollectionProjection {
        title: "World objects".into(),
        items: [
            ENTITY_WORKSPACE,
            ENTITY_ATLAS,
            ENTITY_RESONANCE,
            ENTITY_TIDE,
            ENTITY_REACH,
            ENTITY_ARCHIVE,
        ]
        .into_iter()
        .filter_map(|id| collection_item(world, id))
        .collect(),
    }
}

fn collection_item(world: &World, id: EntityId) -> Option<CollectionItem> {
    let entity = world.state().entity(id)?;
    Some(CollectionItem {
        id: SelectionId::Entity(id),
        title: entity_title(world, id),
        subtitle: entity
            .component("role")
            .and_then(Value::as_text)
            .map(str::to_owned)
            .unwrap_or_else(|| humanize(&entity.kind)),
    })
}

fn canvas(world: &World) -> CanvasProjection {
    CanvasProjection {
        items: [
            canvas_item(
                world,
                ENTITY_WORKSPACE,
                CanvasItemKind::Place,
                0.50,
                0.50,
            ),
            canvas_item(
                world,
                ENTITY_ATLAS,
                CanvasItemKind::Object,
                0.25,
                0.30,
            ),
            canvas_item(
                world,
                ENTITY_RESONANCE,
                CanvasItemKind::Object,
                0.74,
                0.28,
            ),
            canvas_item(
                world,
                ENTITY_TIDE,
                CanvasItemKind::Object,
                0.22,
                0.72,
            ),
            canvas_item(
                world,
                ENTITY_REACH,
                CanvasItemKind::Object,
                0.77,
                0.68,
            ),
            canvas_item(
                world,
                ENTITY_ARCHIVE,
                CanvasItemKind::Object,
                0.50,
                0.82,
            ),
        ]
        .into_iter()
        .flatten()
        .collect(),
    }
}

fn canvas_item(
    world: &World,
    id: EntityId,
    kind: CanvasItemKind,
    x: f32,
    y: f32,
) -> Option<CanvasItem> {
    let entity = world.state().entity(id)?;
    Some(CanvasItem {
        id: SelectionId::Entity(id),
        kind,
        label: entity_title(world, id),
        detail: entity
            .component("summary")
            .and_then(Value::as_text)
            .map(str::to_owned)
            .unwrap_or_else(|| humanize(&entity.kind)),
        x,
        y,
    })
}

fn inspectors(world: &World) -> std::collections::BTreeMap<SelectionId, InspectorProjection> {
    let mut inspectors = inspectors_from_world(world);
    if let Some(workspace) = inspectors.get_mut(&SelectionId::Entity(ENTITY_WORKSPACE)) {
        workspace.sections.push(InspectorSection {
            title: "Direction".into(),
            rows: vec![
                InspectorRow {
                    label: "Outward".into(),
                    value: CHOOSE_OUTWARD.into(),
                },
                InspectorRow {
                    label: "Rooted".into(),
                    value: CHOOSE_ROOTED.into(),
                },
            ],
        });
    }
    if let Some(archive) = inspectors.get_mut(&SelectionId::Entity(ENTITY_ARCHIVE)) {
        archive.sections.push(InspectorSection {
            title: "Possible records".into(),
            rows: vec![
                InspectorRow {
                    label: "Atlas".into(),
                    value: RECORD_ATLAS.into(),
                },
                InspectorRow {
                    label: "Resonance".into(),
                    value: RECORD_RESONANCE.into(),
                },
            ],
        });
    }
    inspectors
}

fn resonance_detail(world: &World) -> String {
    let resonance = world.state().entity(ENTITY_RESONANCE);
    match resonance
        .and_then(|entity| entity.component("pattern"))
        .and_then(Value::as_text)
    {
        Some("radiating") => "Signals are traveling farther than before.".into(),
        Some("nested") => "Signals are folding inward into deeper local patterns.".into(),
        Some(other) => humanize(other),
        None => "No durable pattern yet.".into(),
    }
}

fn archive_detail(world: &World) -> String {
    let archive = world.state().entity(ENTITY_ARCHIVE);
    let record = archive
        .and_then(|entity| entity.component("record"))
        .and_then(Value::as_text);
    match record {
        Some("atlas") => "An outward-facing record was preserved.".into(),
        Some("resonance") => "A rooted resonance record was preserved.".into(),
        Some(other) => humanize(other),
        None => "No durable record selected yet.".into(),
    }
}

fn entity_title(world: &World, id: EntityId) -> String {
    world
        .state()
        .entity(id)
        .and_then(|entity| entity.component("name"))
        .and_then(Value::as_text)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Entity #{id}"))
}

fn humanize(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn project(world: &World) -> ProjectionSnapshot {
    snapshot(world)
}

pub fn project_pack(world: &World) -> ProjectionSnapshot {
    PocketUniversePack.project(world)
}
