mod causal;
mod influence;

use std::collections::BTreeMap;
use world_core::{Entity, EntityId, Event, EventId, Value, World};

pub use causal::{why_from_world, why_map_from_world, WhyNode, WhyProjection};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SelectionId {
    Entity(EntityId),
    Event(EventId),
}

impl SelectionId {
    pub fn stable_key(self) -> String {
        match self {
            Self::Entity(id) => format!("entity-{id}"),
            Self::Event(id) => format!("event-{id}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionIntent {
    ForkBeforeEvent(EventId),
    InvokeCommand(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionCommand {
    pub id: String,
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProjectionCapabilities {
    pub fork: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProjectionSnapshot {
    pub title: String,
    pub world_time: u64,
    pub capabilities: ProjectionCapabilities,
    pub briefing: Option<BriefingProjection>,
    pub commands: Vec<ProjectionCommand>,
    pub collection: CollectionProjection,
    pub timeline: TimelineProjection,
    pub canvas: CanvasProjection,
    pub inspectors: BTreeMap<SelectionId, InspectorProjection>,
    pub why: BTreeMap<EventId, WhyProjection>,
}

impl ProjectionSnapshot {
    pub fn inspector(&self, selection: SelectionId) -> Option<&InspectorProjection> {
        self.inspectors.get(&selection)
    }

    pub fn why(&self, event: EventId) -> Option<&WhyProjection> {
        self.why.get(&event)
    }

    pub fn command(&self, id: &str) -> Option<&ProjectionCommand> {
        self.commands.iter().find(|command| command.id == id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BriefingProjection {
    pub eyebrow: String,
    pub title: String,
    pub items: Vec<BriefingItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BriefingItem {
    pub selection: Option<SelectionId>,
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollectionProjection {
    pub title: String,
    pub items: Vec<CollectionItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollectionItem {
    pub id: SelectionId,
    pub title: String,
    pub subtitle: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineProjection {
    pub items: Vec<TimelineItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineItem {
    pub id: SelectionId,
    pub world_time: u64,
    pub title: String,
    pub subtitle: String,
    pub caused_by: Vec<EventId>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasProjection {
    pub items: Vec<CanvasItem>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasItemKind {
    Place,
    Actor,
    Object,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasItem {
    pub id: SelectionId,
    pub kind: CanvasItemKind,
    pub label: String,
    pub detail: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorProjection {
    pub selection: SelectionId,
    pub title: String,
    pub subtitle: String,
    pub sections: Vec<InspectorSection>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorSection {
    pub title: String,
    pub rows: Vec<InspectorRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorRow {
    pub label: String,
    pub value: String,
}

pub fn timeline_from_world(world: &World) -> TimelineProjection {
    TimelineProjection {
        items: world
            .events()
            .iter()
            .rev()
            .map(|event| TimelineItem {
                id: SelectionId::Event(event.id),
                world_time: event.world_time,
                title: humanize(&event.kind),
                subtitle: event_summary(event, world),
                caused_by: event.caused_by.clone(),
            })
            .collect(),
    }
}

pub fn inspectors_from_world(world: &World) -> BTreeMap<SelectionId, InspectorProjection> {
    let mut inspectors = BTreeMap::new();
    for entity in world.state().entities() {
        inspectors.insert(
            SelectionId::Entity(entity.id),
            inspector_for_entity(entity, world),
        );
    }
    for event in world.events() {
        inspectors.insert(
            SelectionId::Event(event.id),
            inspector_for_event(event, world),
        );
    }
    inspectors
}

pub fn entity_title(entity: &Entity) -> String {
    match entity.component("name") {
        Some(Value::Text(name)) => name.clone(),
        _ => format!("{} #{}", humanize(&entity.kind), entity.id),
    }
}

pub fn value_text(value: &Value, world: &World) -> String {
    match value {
        Value::Null => "—".into(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Text(value) => value.clone(),
        Value::Entity(id) => world
            .state()
            .entity(*id)
            .map(entity_title)
            .unwrap_or_else(|| format!("Entity #{id}")),
        Value::List(values) => values
            .iter()
            .map(|value| value_text(value, world))
            .collect::<Vec<_>>()
            .join(", "),
        Value::Map(values) => values
            .iter()
            .map(|(key, value)| format!("{key}: {}", value_text(value, world)))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn inspector_for_entity(entity: &Entity, world: &World) -> InspectorProjection {
    let components = entity
        .components
        .iter()
        .filter(|(key, _)| key.as_str() != "name")
        .map(|(key, value)| InspectorRow {
            label: humanize(key),
            value: value_text(value, world),
        })
        .collect::<Vec<_>>();

    let relations = world
        .state()
        .relations()
        .filter(|relation| relation.from == entity.id || relation.to == entity.id)
        .map(|relation| {
            let other = if relation.from == entity.id {
                relation.to
            } else {
                relation.from
            };
            let other = world
                .state()
                .entity(other)
                .map(entity_title)
                .unwrap_or_else(|| format!("Entity #{other}"));
            InspectorRow {
                label: humanize(&relation.kind),
                value: other,
            }
        })
        .collect::<Vec<_>>();

    let mut sections = vec![InspectorSection {
        title: "State".into(),
        rows: components,
    }];
    if !relations.is_empty() {
        sections.push(InspectorSection {
            title: "Relations".into(),
            rows: relations,
        });
    }

    InspectorProjection {
        selection: SelectionId::Entity(entity.id),
        title: entity_title(entity),
        subtitle: humanize(&entity.kind),
        sections,
    }
}

fn inspector_for_event(event: &Event, world: &World) -> InspectorProjection {
    let mut context = Vec::new();
    if let Some(actor) = event.actor {
        context.push(InspectorRow {
            label: "Actor".into(),
            value: world
                .state()
                .entity(actor)
                .map(entity_title)
                .unwrap_or_else(|| format!("Entity #{actor}")),
        });
    }
    if !event.targets.is_empty() {
        context.push(InspectorRow {
            label: "Targets".into(),
            value: event
                .targets
                .iter()
                .map(|id| {
                    world
                        .state()
                        .entity(*id)
                        .map(entity_title)
                        .unwrap_or_else(|| format!("Entity #{id}"))
                })
                .collect::<Vec<_>>()
                .join(", "),
        });
    }
    if !event.caused_by.is_empty() {
        context.push(InspectorRow {
            label: "Caused by".into(),
            value: event
                .caused_by
                .iter()
                .map(|id| format!("Event #{id}"))
                .collect::<Vec<_>>()
                .join(", "),
        });
    }

    let payload = event
        .payload
        .iter()
        .map(|(key, value)| InspectorRow {
            label: humanize(key),
            value: value_text(value, world),
        })
        .collect::<Vec<_>>();

    let mut sections = vec![InspectorSection {
        title: "Context".into(),
        rows: context,
    }];
    if !payload.is_empty() {
        sections.push(InspectorSection {
            title: "Payload".into(),
            rows: payload,
        });
    }

    let influence = influence::influence_rows(world, event.id);
    if !influence.is_empty() {
        sections.push(InspectorSection {
            title: "Influence".into(),
            rows: influence,
        });
    }

    InspectorProjection {
        selection: SelectionId::Event(event.id),
        title: humanize(&event.kind),
        subtitle: format!("World time {} · Event #{}", event.world_time, event.id),
        sections,
    }
}

fn semantic_event_summary(event: &Event) -> Option<&str> {
    ["summary", "change"]
        .into_iter()
        .find_map(|key| match event.payload.get(key) {
            Some(Value::Text(value)) if !value.trim().is_empty() => Some(value.as_str()),
            _ => None,
        })
}

pub(crate) fn event_summary(event: &Event, world: &World) -> String {
    let mut parts = Vec::new();
    if let Some(actor) = event
        .actor
        .and_then(|id| world.state().entity(id))
        .map(entity_title)
    {
        parts.push(actor);
    }
    if let Some(summary) = semantic_event_summary(event) {
        parts.push(summary.to_string());
    }
    parts.push(format!("Event #{}", event.id));
    parts.join(" · ")
}

pub(crate) fn humanize(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::{Entity, Event, EventId, WorldState};

    fn sample_world() -> World {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace")
                    .with_component("name", "Workspace")
                    .with_component("status", "active"),
            )
            .unwrap();
        World::from_history(
            state,
            &[Event {
                id: EventId::new(1),
                kind: "work_started".into(),
                world_time: 0,
                actor: Some(EntityId::new(1)),
                targets: vec![],
                caused_by: vec![],
                payload: BTreeMap::new(),
                changes: vec![],
            }],
        )
        .unwrap()
    }

    #[test]
    fn generic_projection_builds_timeline_and_inspectors() {
        let world = sample_world();
        let timeline = timeline_from_world(&world);
        let inspectors = inspectors_from_world(&world);

        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].title, "Work Started");
        assert_eq!(timeline.items[0].subtitle, "Workspace · Event #1");
        assert_eq!(
            inspectors
                .get(&SelectionId::Entity(EntityId::new(1)))
                .unwrap()
                .title,
            "Workspace"
        );
        assert!(inspectors.contains_key(&SelectionId::Event(EventId::new(1))));
    }

    #[test]
    fn timeline_surfaces_semantic_event_payload_without_domain_knowledge() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace").with_component("name", "Workspace"),
            )
            .unwrap();
        let world = World::from_history(
            state,
            &[Event {
                id: EventId::new(1),
                kind: "direction_chosen".into(),
                world_time: 0,
                actor: None,
                targets: vec![EntityId::new(1)],
                caused_by: vec![],
                payload: BTreeMap::from([
                    ("change".into(), Value::Text("fallback detail".into())),
                    (
                        "summary".into(),
                        Value::Text("A durable direction was chosen.".into()),
                    ),
                ]),
                changes: vec![],
            }],
        )
        .unwrap();

        let timeline = timeline_from_world(&world);
        assert_eq!(
            timeline.items[0].subtitle,
            "A durable direction was chosen. · Event #1"
        );
    }

    #[test]
    fn snapshot_command_lookup_is_generic() {
        let snapshot = ProjectionSnapshot {
            commands: vec![ProjectionCommand {
                id: "world.continue".into(),
                title: "Continue".into(),
                detail: "Let the world keep running".into(),
            }],
            ..ProjectionSnapshot::default()
        };

        assert_eq!(
            snapshot
                .command("world.continue")
                .map(|command| command.title.as_str()),
            Some("Continue")
        );
        assert!(snapshot.command("missing").is_none());
        assert!(!snapshot.capabilities.fork);
    }
}
