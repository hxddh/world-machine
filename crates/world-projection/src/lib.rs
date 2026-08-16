mod causal;
mod influence;

use std::collections::{BTreeMap, BTreeSet};
use world_core::{Entity, EntityId, Event, EventId, StateChange, Value, World};

pub use causal::{why_from_world, why_map_from_world, WhyNode, WhyProjection};

pub const ENTITY_HISTORY_SECTION: &str = "Recorded entity changes";

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

    pub fn entity_history(&self, entity: EntityId) -> Vec<&TimelineItem> {
        let Some(inspector) = self.inspector(SelectionId::Entity(entity)) else {
            return Vec::new();
        };
        let Some(section) = inspector
            .sections
            .iter()
            .find(|section| section.title == ENTITY_HISTORY_SECTION)
        else {
            return Vec::new();
        };
        let keys = section
            .rows
            .iter()
            .map(|row| row.value.as_str())
            .collect::<BTreeSet<_>>();

        self.timeline
            .items
            .iter()
            .filter(|item| keys.contains(item.id.stable_key().as_str()))
            .collect()
    }

    pub fn directly_changed_entities(&self, event: EventId) -> Vec<EntityId> {
        let event_selection = SelectionId::Event(event);
        if !self
            .timeline
            .items
            .iter()
            .any(|item| item.id == event_selection)
        {
            return Vec::new();
        }
        let event_key = event_selection.stable_key();
        self.inspectors
            .iter()
            .filter_map(|(selection, inspector)| {
                let SelectionId::Entity(entity) = selection else {
                    return None;
                };
                inspector
                    .sections
                    .iter()
                    .find(|section| section.title == ENTITY_HISTORY_SECTION)
                    .filter(|section| {
                        section
                            .rows
                            .iter()
                            .any(|row| row.value.as_str() == event_key.as_str())
                    })
                    .map(|_| *entity)
            })
            .collect()
    }
    pub fn influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
        influence::influence_from_timeline(&self.timeline, event)
    }

    pub fn semantic_influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
        influence::semantic_influence_from_snapshot(&self.timeline, &self.inspectors, event)
    }

    pub fn semantic_path(&self, event: EventId) -> Vec<&TimelineItem> {
        influence::semantic_path_from_snapshot(&self.timeline, &self.inspectors, event)
    }

    pub fn semantic_path_details(&self, event: EventId) -> Vec<(usize, &TimelineItem, String)> {
        influence::semantic_path_details_from_snapshot(&self.timeline, &self.inspectors, event)
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
    let recorded_changes = recorded_entity_change_rows(entity.id, world);

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
    if !recorded_changes.is_empty() {
        sections.push(InspectorSection {
            title: ENTITY_HISTORY_SECTION.into(),
            rows: recorded_changes,
        });
    }

    InspectorProjection {
        selection: SelectionId::Entity(entity.id),
        title: entity_title(entity),
        subtitle: humanize(&entity.kind),
        sections,
    }
}

fn recorded_entity_change_rows(entity: EntityId, world: &World) -> Vec<InspectorRow> {
    world
        .events()
        .iter()
        .rev()
        .filter(|event| {
            event
                .changes
                .iter()
                .any(|change| change_directly_affects_entity(change, entity))
        })
        .map(|event| InspectorRow {
            label: format!(
                "World time {} · {}",
                event.world_time,
                humanize(&event.kind)
            ),
            value: SelectionId::Event(event.id).stable_key(),
        })
        .collect()
}

fn change_directly_affects_entity(change: &StateChange, entity_id: EntityId) -> bool {
    match change {
        StateChange::CreateEntity(entity) => entity.id == entity_id,
        StateChange::RemoveEntity(entity) => *entity == entity_id,
        StateChange::SetComponent { entity, .. } | StateChange::RemoveComponent { entity, .. } => {
            *entity == entity_id
        }
        StateChange::CreateRelation(relation) => {
            relation.from == entity_id || relation.to == entity_id
        }
        StateChange::RemoveRelation(_)
        | StateChange::SetRelationProperty { .. }
        | StateChange::RemoveRelationProperty { .. } => false,
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
            value: recorded_value_text(value),
        })
        .collect::<Vec<_>>();
    let changes = event.changes.iter().map(change_row).collect::<Vec<_>>();

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
    if !changes.is_empty() {
        sections.push(InspectorSection {
            title: "Changes".into(),
            rows: changes,
        });
    }

    InspectorProjection {
        selection: SelectionId::Event(event.id),
        title: humanize(&event.kind),
        subtitle: format!("World time {} · Event #{}", event.world_time, event.id),
        sections,
    }
}

fn change_row(change: &StateChange) -> InspectorRow {
    match change {
        StateChange::CreateEntity(entity) => InspectorRow {
            label: "Create entity".into(),
            value: entity_title(entity),
        },
        StateChange::RemoveEntity(entity) => InspectorRow {
            label: "Remove entity".into(),
            value: format!("Entity #{entity}"),
        },
        StateChange::SetComponent { entity, key, value } => InspectorRow {
            label: format!("Entity #{entity} · {}", humanize(key)),
            value: recorded_value_text(value),
        },
        StateChange::RemoveComponent { entity, key } => InspectorRow {
            label: format!("Entity #{entity} · {}", humanize(key)),
            value: "Removed".into(),
        },
        StateChange::CreateRelation(relation) => InspectorRow {
            label: "Create relation".into(),
            value: format!(
                "{} · Entity #{} → Entity #{}",
                humanize(&relation.kind),
                relation.from,
                relation.to
            ),
        },
        StateChange::RemoveRelation(relation) => InspectorRow {
            label: "Remove relation".into(),
            value: format!("Relation #{relation}"),
        },
        StateChange::SetRelationProperty {
            relation,
            key,
            value,
        } => InspectorRow {
            label: format!("Relation #{relation} · {}", humanize(key)),
            value: recorded_value_text(value),
        },
        StateChange::RemoveRelationProperty { relation, key } => InspectorRow {
            label: format!("Relation #{relation} · {}", humanize(key)),
            value: "Removed".into(),
        },
    }
}

fn recorded_value_text(value: &Value) -> String {
    match value {
        Value::Null => "—".into(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Text(value) => value.clone(),
        Value::Entity(entity) => format!("Entity #{entity}"),
        Value::List(values) => values
            .iter()
            .map(recorded_value_text)
            .collect::<Vec<_>>()
            .join(", "),
        Value::Map(values) => values
            .iter()
            .map(|(key, value)| format!("{key}: {}", recorded_value_text(value)))
            .collect::<Vec<_>>()
            .join(", "),
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
    use world_core::{Entity, Event, EventId, StateChange, WorldState};

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
    fn event_inspector_surfaces_recorded_state_changes() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace")
                    .with_component("name", "Workspace")
                    .with_component("status", "active"),
            )
            .unwrap();
        let world = World::from_history(
            state,
            &[Event {
                id: EventId::new(1),
                kind: "work_finished".into(),
                world_time: 1,
                actor: None,
                targets: vec![EntityId::new(1)],
                caused_by: vec![],
                payload: BTreeMap::new(),
                changes: vec![StateChange::SetComponent {
                    entity: EntityId::new(1),
                    key: "status".into(),
                    value: Value::Text("done".into()),
                }],
            }],
        )
        .unwrap();

        let inspectors = inspectors_from_world(&world);
        let changes = inspectors
            .get(&SelectionId::Event(EventId::new(1)))
            .unwrap()
            .sections
            .iter()
            .find(|section| section.title == "Changes")
            .expect("recorded StateChanges should be inspectable");
        assert_eq!(changes.rows.len(), 1);
        assert_eq!(changes.rows[0].label, "Entity #1 · Status");
        assert_eq!(changes.rows[0].value, "done");
    }

    #[test]
    fn entity_history_uses_stable_event_keys_and_typed_timeline_lookup() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId::new(1), "workspace")
                    .with_component("name", "Workspace")
                    .with_component("status", "active"),
            )
            .unwrap();
        state
            .seed_entity(
                Entity::new(EntityId::new(2), "worker")
                    .with_component("name", "Worker")
                    .with_component("status", "idle"),
            )
            .unwrap();
        let world = World::from_history(
            state,
            &[
                Event {
                    id: EventId::new(1),
                    kind: "work_finished".into(),
                    world_time: 1,
                    actor: None,
                    targets: vec![EntityId::new(1)],
                    caused_by: vec![],
                    payload: BTreeMap::new(),
                    changes: vec![StateChange::SetComponent {
                        entity: EntityId::new(1),
                        key: "status".into(),
                        value: Value::Text("done".into()),
                    }],
                },
                Event {
                    id: EventId::new(2),
                    kind: "worker_changed".into(),
                    world_time: 2,
                    actor: None,
                    targets: vec![EntityId::new(2)],
                    caused_by: vec![],
                    payload: BTreeMap::new(),
                    changes: vec![StateChange::SetComponent {
                        entity: EntityId::new(2),
                        key: "status".into(),
                        value: Value::Text("busy".into()),
                    }],
                },
                Event {
                    id: EventId::new(3),
                    kind: "workspace_renamed".into(),
                    world_time: 3,
                    actor: None,
                    targets: vec![EntityId::new(1)],
                    caused_by: vec![EventId::new(1)],
                    payload: BTreeMap::new(),
                    changes: vec![StateChange::SetComponent {
                        entity: EntityId::new(1),
                        key: "name".into(),
                        value: Value::Text("Renamed Workspace".into()),
                    }],
                },
            ],
        )
        .unwrap();

        let inspectors = inspectors_from_world(&world);
        let recorded = inspectors
            .get(&SelectionId::Entity(EntityId::new(1)))
            .unwrap()
            .sections
            .iter()
            .find(|section| section.title == ENTITY_HISTORY_SECTION)
            .expect("history section should exist");
        assert_eq!(recorded.rows[0].label, "World time 3 · Workspace Renamed");
        assert_eq!(recorded.rows[0].value, "event-3");
        assert_eq!(recorded.rows[1].value, "event-1");

        let snapshot = ProjectionSnapshot {
            timeline: timeline_from_world(&world),
            inspectors,
            ..ProjectionSnapshot::default()
        };
        let history = snapshot.entity_history(EntityId::new(1));
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].id, SelectionId::Event(EventId::new(3)));
        assert_eq!(history[1].id, SelectionId::Event(EventId::new(1)));
    }

    #[test]
    fn event_payload_entity_references_are_recorded_not_current_state_derived() {
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
                kind: "workspace_renamed".into(),
                world_time: 1,
                actor: None,
                targets: vec![],
                caused_by: vec![],
                payload: BTreeMap::from([("subject".into(), Value::Entity(EntityId::new(1)))]),
                changes: vec![StateChange::SetComponent {
                    entity: EntityId::new(1),
                    key: "name".into(),
                    value: Value::Text("Renamed Workspace".into()),
                }],
            }],
        )
        .unwrap();

        let inspectors = inspectors_from_world(&world);
        let payload = inspectors
            .get(&SelectionId::Event(EventId::new(1)))
            .unwrap()
            .sections
            .iter()
            .find(|section| section.title == "Payload")
            .expect("recorded payload should be inspectable");

        assert_eq!(payload.rows.len(), 1);
        assert_eq!(payload.rows[0].label, "Subject");
        assert_eq!(payload.rows[0].value, "Entity #1");
        assert_eq!(
            inspectors
                .get(&SelectionId::Entity(EntityId::new(1)))
                .unwrap()
                .title,
            "Renamed Workspace"
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
