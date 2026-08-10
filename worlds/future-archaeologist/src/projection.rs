use crate::model::*;
use std::collections::{BTreeMap, BTreeSet};
use world_core::{Entity, EntityId, Event, EventId, Value, World};
use world_projection::{
    BriefingItem, BriefingProjection, CanvasItem, CanvasItemKind, CanvasProjection, CollectionItem,
    CollectionProjection, InspectorProjection, InspectorRow, InspectorSection, ProjectionCommand,
    ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection, WhyNode, WhyProjection,
};

const ARTIFACTS: [EntityId; 6] = [
    CALENDAR_FRAGMENT,
    TAXI_RECEIPT,
    PLATFORM_PHOTO,
    WIFI_LOG,
    PROJECT_COPY_LOG,
    DELETED_MESSAGE,
];

pub(crate) fn snapshot(world: &World) -> ProjectionSnapshot {
    let visible_artifacts = visible_artifact_ids(world);
    let visible_events = visible_event_ids(world, &visible_artifacts);

    ProjectionSnapshot {
        title: "Future Archaeologist · Terminal 17".into(),
        world_time: world.world_time(),
        briefing: Some(briefing(world, &visible_artifacts)),
        commands: commands(world),
        collection: collection(world, &visible_artifacts),
        timeline: timeline(world, &visible_events, &visible_artifacts),
        canvas: canvas(world, &visible_artifacts),
        inspectors: inspectors(world, &visible_artifacts, &visible_events),
        why: why_map(world, &visible_events, &visible_artifacts),
    }
}

fn commands(world: &World) -> Vec<ProjectionCommand> {
    if is_visible(world, DELETED_MESSAGE) {
        Vec::new()
    } else {
        vec![ProjectionCommand {
            id: crate::RECOVER_MESSAGE_COMMAND.into(),
            title: "Recover deleted message".into(),
            detail: "Scan unallocated message storage for a recoverable fragment.".into(),
        }]
    }
}

fn visible_artifact_ids(world: &World) -> Vec<EntityId> {
    ARTIFACTS
        .iter()
        .copied()
        .filter(|id| is_visible(world, *id))
        .collect()
}

fn visible_event_ids(world: &World, artifacts: &[EntityId]) -> BTreeSet<EventId> {
    let mut visible = artifacts
        .iter()
        .filter_map(|id| event_ref(world, *id))
        .collect::<BTreeSet<_>>();

    for event in world.events() {
        if event.kind == "artifact_recovered"
            && event
                .targets
                .iter()
                .any(|target| artifacts.contains(target))
        {
            visible.insert(event.id);
        }
    }
    visible
}

fn collection(world: &World, artifacts: &[EntityId]) -> CollectionProjection {
    CollectionProjection {
        title: "Recovered Artifacts".into(),
        items: artifacts
            .iter()
            .filter_map(|id| {
                let entity = world.state().entity(*id)?;
                Some(CollectionItem {
                    id: SelectionId::Entity(*id),
                    title: entity_name(entity),
                    subtitle: format!(
                        "{} · {}",
                        text_component(entity, ARTIFACT_KIND).unwrap_or("artifact"),
                        text_component(entity, TIMESTAMP).unwrap_or("unknown time")
                    ),
                })
            })
            .collect(),
    }
}

fn timeline(
    world: &World,
    visible_events: &BTreeSet<EventId>,
    artifacts: &[EntityId],
) -> TimelineProjection {
    TimelineProjection {
        items: world
            .events()
            .iter()
            .rev()
            .filter(|event| visible_events.contains(&event.id))
            .map(|event| TimelineItem {
                id: SelectionId::Event(event.id),
                world_time: event.world_time,
                title: humanize(&event.kind),
                subtitle: evidence_summary(world, event, artifacts),
                caused_by: event
                    .caused_by
                    .iter()
                    .copied()
                    .filter(|cause| visible_events.contains(cause))
                    .collect(),
            })
            .collect(),
    }
}

fn briefing(world: &World, artifacts: &[EntityId]) -> BriefingProjection {
    let items = artifacts
        .iter()
        .rev()
        .filter_map(|id| {
            let entity = world.state().entity(*id)?;
            Some(BriefingItem {
                selection: Some(SelectionId::Entity(*id)),
                title: entity_name(entity),
                detail: text_component(entity, SUMMARY)
                    .unwrap_or("Recovered artifact")
                    .into(),
            })
        })
        .take(3)
        .collect();

    BriefingProjection {
        eyebrow: "Future Archaeologist".into(),
        title: format!("{} artifacts are readable", artifacts.len()),
        items,
    }
}

fn canvas(world: &World, artifacts: &[EntityId]) -> CanvasProjection {
    let mut items = Vec::new();
    for (id, kind, x, y) in [
        (TERMINAL, CanvasItemKind::Object, 0.46, 0.46),
        (MIRA, CanvasItemKind::Actor, 0.16, 0.24),
        (ELIAS, CanvasItemKind::Actor, 0.72, 0.24),
        (PLATFORM_12, CanvasItemKind::Place, 0.46, 0.08),
        (ASTERION, CanvasItemKind::Object, 0.82, 0.72),
    ] {
        if let Some(entity) = world.state().entity(id) {
            items.push(CanvasItem {
                id: SelectionId::Entity(id),
                kind,
                label: entity_name(entity),
                detail: humanize(&entity.kind),
                x,
                y,
            });
        }
    }

    let positions = [
        (0.06, 0.58),
        (0.24, 0.76),
        (0.43, 0.82),
        (0.62, 0.78),
        (0.76, 0.58),
        (0.12, 0.82),
    ];
    for (index, id) in artifacts.iter().enumerate() {
        let Some(entity) = world.state().entity(*id) else {
            continue;
        };
        let (x, y) = positions[index.min(positions.len() - 1)];
        items.push(CanvasItem {
            id: SelectionId::Entity(*id),
            kind: CanvasItemKind::Object,
            label: entity_name(entity),
            detail: text_component(entity, ARTIFACT_KIND)
                .map(humanize)
                .unwrap_or_else(|| "Artifact".into()),
            x,
            y,
        });
    }

    CanvasProjection { items }
}

fn inspectors(
    world: &World,
    artifacts: &[EntityId],
    visible_events: &BTreeSet<EventId>,
) -> BTreeMap<SelectionId, InspectorProjection> {
    let mut result = BTreeMap::new();

    for id in [TERMINAL, MIRA, ELIAS, PLATFORM_12, ASTERION] {
        if let Some(inspector) = context_inspector(world, id) {
            result.insert(SelectionId::Entity(id), inspector);
        }
    }
    for id in artifacts {
        if let Some(inspector) = artifact_inspector(world, *id) {
            result.insert(SelectionId::Entity(*id), inspector);
        }
    }
    for event in world
        .events()
        .iter()
        .filter(|event| visible_events.contains(&event.id))
    {
        result.insert(
            SelectionId::Event(event.id),
            event_inspector(world, event, visible_events, artifacts),
        );
    }

    result
}

fn context_inspector(world: &World, id: EntityId) -> Option<InspectorProjection> {
    let entity = world.state().entity(id)?;
    let keys: &[&str] = match id {
        TERMINAL => &["serial", "status"],
        MIRA | ELIAS => &["role"],
        PLATFORM_12 => &["district"],
        ASTERION => &["sector"],
        _ => &[],
    };
    let rows = keys
        .iter()
        .filter_map(|key| {
            text_component(entity, key).map(|value| InspectorRow {
                label: humanize(key),
                value: value.into(),
            })
        })
        .collect();

    Some(InspectorProjection {
        selection: SelectionId::Entity(id),
        title: entity_name(entity),
        subtitle: humanize(&entity.kind),
        sections: vec![InspectorSection {
            title: "Known context".into(),
            rows,
        }],
    })
}

fn artifact_inspector(world: &World, id: EntityId) -> Option<InspectorProjection> {
    let entity = world.state().entity(id)?;
    if !is_visible(world, id) {
        return None;
    }
    let rows = [
        ("Type", ARTIFACT_KIND),
        ("Source", SOURCE),
        ("Timestamp", TIMESTAMP),
        ("Recovered content", SUMMARY),
    ]
    .into_iter()
    .filter_map(|(label, key)| {
        text_component(entity, key).map(|value| InspectorRow {
            label: label.into(),
            value: value.into(),
        })
    })
    .collect();

    Some(InspectorProjection {
        selection: SelectionId::Entity(id),
        title: entity_name(entity),
        subtitle: "Recovered digital artifact".into(),
        sections: vec![InspectorSection {
            title: "Artifact".into(),
            rows,
        }],
    })
}

fn event_inspector(
    world: &World,
    event: &Event,
    visible_events: &BTreeSet<EventId>,
    artifacts: &[EntityId],
) -> InspectorProjection {
    let mut rows = vec![InspectorRow {
        label: "Evidence".into(),
        value: evidence_summary(world, event, artifacts),
    }];
    if let Some(actor) = event
        .actor
        .and_then(|id| world.state().entity(id))
        .map(entity_name)
    {
        rows.push(InspectorRow {
            label: "Actor".into(),
            value: actor,
        });
    }
    if !event.targets.is_empty() {
        rows.push(InspectorRow {
            label: "Targets".into(),
            value: event
                .targets
                .iter()
                .filter_map(|id| world.state().entity(*id).map(entity_name))
                .collect::<Vec<_>>()
                .join(", "),
        });
    }
    let visible_causes = event
        .caused_by
        .iter()
        .copied()
        .filter(|cause| visible_events.contains(cause))
        .collect::<Vec<_>>();
    if !visible_causes.is_empty() {
        rows.push(InspectorRow {
            label: "Visible causes".into(),
            value: visible_causes
                .iter()
                .map(|id| format!("Event #{id}"))
                .collect::<Vec<_>>()
                .join(", "),
        });
    }

    InspectorProjection {
        selection: SelectionId::Event(event.id),
        title: humanize(&event.kind),
        subtitle: format!("Evidence-backed event · world time {}", event.world_time),
        sections: vec![InspectorSection {
            title: "Recovered interpretation".into(),
            rows,
        }],
    }
}

fn why_map(
    world: &World,
    visible_events: &BTreeSet<EventId>,
    artifacts: &[EntityId],
) -> BTreeMap<EventId, WhyProjection> {
    visible_events
        .iter()
        .filter_map(|event| {
            visible_why(world, *event, visible_events, artifacts).map(|why| (*event, why))
        })
        .collect()
}

fn visible_why(
    world: &World,
    root: EventId,
    visible_events: &BTreeSet<EventId>,
    artifacts: &[EntityId],
) -> Option<WhyProjection> {
    if !visible_events.contains(&root) {
        return None;
    }
    world.event(root)?;
    let mut visited = BTreeSet::new();
    let mut nodes = Vec::new();
    visit_visible_cause(
        world,
        root,
        0,
        visible_events,
        artifacts,
        &mut visited,
        &mut nodes,
    );
    Some(WhyProjection { event: root, nodes })
}

fn visit_visible_cause(
    world: &World,
    event_id: EventId,
    depth: usize,
    visible_events: &BTreeSet<EventId>,
    artifacts: &[EntityId],
    visited: &mut BTreeSet<EventId>,
    nodes: &mut Vec<WhyNode>,
) {
    if !visible_events.contains(&event_id) || !visited.insert(event_id) {
        return;
    }
    let Some(event) = world.event(event_id) else {
        return;
    };
    let visible_causes = event
        .caused_by
        .iter()
        .copied()
        .filter(|cause| visible_events.contains(cause))
        .collect::<Vec<_>>();
    nodes.push(WhyNode {
        event: event.id,
        depth,
        world_time: event.world_time,
        title: humanize(&event.kind),
        subtitle: evidence_summary(world, event, artifacts),
        caused_by: visible_causes.clone(),
    });
    for cause in visible_causes {
        visit_visible_cause(
            world,
            cause,
            depth + 1,
            visible_events,
            artifacts,
            visited,
            nodes,
        );
    }
}

fn evidence_summary(world: &World, event: &Event, artifacts: &[EntityId]) -> String {
    if let Some(artifact) = artifacts.iter().find_map(|id| {
        (event_ref(world, *id) == Some(event.id))
            .then(|| world.state().entity(*id))
            .flatten()
    }) {
        return format!("Supported by {}", entity_name(artifact));
    }
    if event.kind == "artifact_recovered" {
        if let Some(artifact) = event
            .targets
            .iter()
            .find_map(|id| world.state().entity(*id))
        {
            return format!("Recovered {}", entity_name(artifact));
        }
    }
    format!("Event #{}", event.id)
}

fn event_ref(world: &World, artifact: EntityId) -> Option<EventId> {
    match world.state().entity(artifact)?.component(EVENT_REF)? {
        Value::Integer(value) if *value > 0 => Some(EventId::new(*value as u64)),
        _ => None,
    }
}

fn is_visible(world: &World, artifact: EntityId) -> bool {
    matches!(
        world
            .state()
            .entity(artifact)
            .and_then(|entity| entity.component(VISIBLE)),
        Some(Value::Bool(true))
    )
}

fn text_component<'a>(entity: &'a Entity, key: &str) -> Option<&'a str> {
    match entity.component(key) {
        Some(Value::Text(value)) => Some(value),
        _ => None,
    }
}

fn entity_name(entity: &Entity) -> String {
    text_component(entity, "name")
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} #{}", humanize(&entity.kind), entity.id))
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
