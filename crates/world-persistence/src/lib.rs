use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use world_core::{
    ActionRegistry, ActionRequest, Entity, EntityId, Event, EventId, Relation, RelationId,
    StateChange, Value, World, WorldError, WorldState,
};

pub const WORLD_ARCHIVE_FORMAT: &str = "world-machine";
pub const WORLD_ARCHIVE_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorldPackRef {
    pub id: String,
    pub version: String,
}

impl WorldPackRef {
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldArchive {
    pub format: String,
    pub format_version: u32,
    pub pack: WorldPackRef,
    pub world_time: u64,
    pub events: Vec<ArchivedEvent>,
    pub pending: Vec<ArchivedScheduledAction>,
}

impl WorldArchive {
    pub fn capture(pack: WorldPackRef, world: &World) -> Result<Self, PersistenceError> {
        validate_pack(&pack)?;
        Ok(Self {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack,
            world_time: world.world_time(),
            events: world.events().iter().map(ArchivedEvent::from).collect(),
            pending: world
                .scheduler()
                .pending()
                .map(ArchivedScheduledAction::from)
                .collect(),
        })
    }

    pub fn to_json_pretty(&self) -> Result<String, PersistenceError> {
        self.validate_header()?;
        serde_json::to_string_pretty(self).map_err(PersistenceError::Json)
    }

    pub fn from_json(json: &str) -> Result<Self, PersistenceError> {
        let archive: Self = serde_json::from_str(json).map_err(PersistenceError::Json)?;
        archive.validate_header()?;
        Ok(archive)
    }

    pub fn restore(
        &self,
        expected_pack: &WorldPackRef,
        baseline: WorldState,
    ) -> Result<World, PersistenceError> {
        self.validate_header()?;
        validate_pack(expected_pack)?;
        if &self.pack != expected_pack {
            return Err(PersistenceError::PackMismatch {
                expected: expected_pack.clone(),
                found: self.pack.clone(),
            });
        }

        let events = self.events.iter().map(Event::from).collect::<Vec<_>>();
        let mut world = World::from_history(baseline, &events).map_err(PersistenceError::World)?;

        let empty_actions = ActionRegistry::new();
        world
            .advance_to(&empty_actions, self.world_time)
            .map_err(PersistenceError::World)?;

        for pending in &self.pending {
            world
                .schedule_at(pending.world_time, ActionRequest::from(&pending.request))
                .map_err(PersistenceError::World)?;
        }

        Ok(world)
    }

    fn validate_header(&self) -> Result<(), PersistenceError> {
        if self.format != WORLD_ARCHIVE_FORMAT {
            return Err(PersistenceError::UnsupportedFormat(self.format.clone()));
        }
        if self.format_version != WORLD_ARCHIVE_VERSION {
            return Err(PersistenceError::UnsupportedVersion(self.format_version));
        }
        validate_pack(&self.pack)
    }
}

fn validate_pack(pack: &WorldPackRef) -> Result<(), PersistenceError> {
    if pack.id.trim().is_empty() || pack.version.trim().is_empty() {
        return Err(PersistenceError::InvalidPack);
    }
    Ok(())
}

#[derive(Debug)]
pub enum PersistenceError {
    Json(serde_json::Error),
    UnsupportedFormat(String),
    UnsupportedVersion(u32),
    InvalidPack,
    PackMismatch {
        expected: WorldPackRef,
        found: WorldPackRef,
    },
    World(WorldError),
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "invalid world archive JSON: {error}"),
            Self::UnsupportedFormat(format) => {
                write!(f, "unsupported world archive format: {format}")
            }
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported world archive version: {version}")
            }
            Self::InvalidPack => write!(f, "world archive pack id and version must be non-empty"),
            Self::PackMismatch { expected, found } => write!(
                f,
                "world archive pack mismatch: expected {}@{}, found {}@{}",
                expected.id, expected.version, found.id, found.version
            ),
            Self::World(error) => error.fmt(f),
        }
    }
}

impl Error for PersistenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::World(error) => Some(error),
            Self::UnsupportedFormat(_)
            | Self::UnsupportedVersion(_)
            | Self::InvalidPack
            | Self::PackMismatch { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArchivedEvent {
    pub id: u64,
    pub kind: String,
    pub world_time: u64,
    pub actor: Option<u64>,
    pub targets: Vec<u64>,
    pub caused_by: Vec<u64>,
    pub payload: BTreeMap<String, ArchivedValue>,
    pub changes: Vec<ArchivedStateChange>,
}

impl From<&Event> for ArchivedEvent {
    fn from(event: &Event) -> Self {
        Self {
            id: event.id.0,
            kind: event.kind.clone(),
            world_time: event.world_time,
            actor: event.actor.map(|id| id.0),
            targets: event.targets.iter().map(|id| id.0).collect(),
            caused_by: event.caused_by.iter().map(|id| id.0).collect(),
            payload: event
                .payload
                .iter()
                .map(|(key, value)| (key.clone(), ArchivedValue::from(value)))
                .collect(),
            changes: event
                .changes
                .iter()
                .map(ArchivedStateChange::from)
                .collect(),
        }
    }
}

impl From<&ArchivedEvent> for Event {
    fn from(event: &ArchivedEvent) -> Self {
        Self {
            id: EventId::new(event.id),
            kind: event.kind.clone(),
            world_time: event.world_time,
            actor: event.actor.map(EntityId::new),
            targets: event.targets.iter().copied().map(EntityId::new).collect(),
            caused_by: event.caused_by.iter().copied().map(EventId::new).collect(),
            payload: event
                .payload
                .iter()
                .map(|(key, value)| (key.clone(), Value::from(value)))
                .collect(),
            changes: event.changes.iter().map(StateChange::from).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ArchivedStateChange {
    CreateEntity {
        entity: ArchivedEntity,
    },
    RemoveEntity {
        entity: u64,
    },
    SetComponent {
        entity: u64,
        key: String,
        value: ArchivedValue,
    },
    RemoveComponent {
        entity: u64,
        key: String,
    },
    CreateRelation {
        relation: ArchivedRelation,
    },
    RemoveRelation {
        relation: u64,
    },
    SetRelationProperty {
        relation: u64,
        key: String,
        value: ArchivedValue,
    },
    RemoveRelationProperty {
        relation: u64,
        key: String,
    },
}

impl From<&StateChange> for ArchivedStateChange {
    fn from(change: &StateChange) -> Self {
        match change {
            StateChange::CreateEntity(entity) => Self::CreateEntity {
                entity: ArchivedEntity::from(entity),
            },
            StateChange::RemoveEntity(entity) => Self::RemoveEntity { entity: entity.0 },
            StateChange::SetComponent { entity, key, value } => Self::SetComponent {
                entity: entity.0,
                key: key.clone(),
                value: ArchivedValue::from(value),
            },
            StateChange::RemoveComponent { entity, key } => Self::RemoveComponent {
                entity: entity.0,
                key: key.clone(),
            },
            StateChange::CreateRelation(relation) => Self::CreateRelation {
                relation: ArchivedRelation::from(relation),
            },
            StateChange::RemoveRelation(relation) => Self::RemoveRelation {
                relation: relation.0,
            },
            StateChange::SetRelationProperty {
                relation,
                key,
                value,
            } => Self::SetRelationProperty {
                relation: relation.0,
                key: key.clone(),
                value: ArchivedValue::from(value),
            },
            StateChange::RemoveRelationProperty { relation, key } => Self::RemoveRelationProperty {
                relation: relation.0,
                key: key.clone(),
            },
        }
    }
}

impl From<&ArchivedStateChange> for StateChange {
    fn from(change: &ArchivedStateChange) -> Self {
        match change {
            ArchivedStateChange::CreateEntity { entity } => {
                Self::CreateEntity(Entity::from(entity))
            }
            ArchivedStateChange::RemoveEntity { entity } => {
                Self::RemoveEntity(EntityId::new(*entity))
            }
            ArchivedStateChange::SetComponent { entity, key, value } => Self::SetComponent {
                entity: EntityId::new(*entity),
                key: key.clone(),
                value: Value::from(value),
            },
            ArchivedStateChange::RemoveComponent { entity, key } => Self::RemoveComponent {
                entity: EntityId::new(*entity),
                key: key.clone(),
            },
            ArchivedStateChange::CreateRelation { relation } => {
                Self::CreateRelation(Relation::from(relation))
            }
            ArchivedStateChange::RemoveRelation { relation } => {
                Self::RemoveRelation(RelationId::new(*relation))
            }
            ArchivedStateChange::SetRelationProperty {
                relation,
                key,
                value,
            } => Self::SetRelationProperty {
                relation: RelationId::new(*relation),
                key: key.clone(),
                value: Value::from(value),
            },
            ArchivedStateChange::RemoveRelationProperty { relation, key } => {
                Self::RemoveRelationProperty {
                    relation: RelationId::new(*relation),
                    key: key.clone(),
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArchivedEntity {
    pub id: u64,
    pub kind: String,
    pub components: BTreeMap<String, ArchivedValue>,
}

impl From<&Entity> for ArchivedEntity {
    fn from(entity: &Entity) -> Self {
        Self {
            id: entity.id.0,
            kind: entity.kind.clone(),
            components: entity
                .components
                .iter()
                .map(|(key, value)| (key.clone(), ArchivedValue::from(value)))
                .collect(),
        }
    }
}

impl From<&ArchivedEntity> for Entity {
    fn from(entity: &ArchivedEntity) -> Self {
        Self {
            id: EntityId::new(entity.id),
            kind: entity.kind.clone(),
            components: entity
                .components
                .iter()
                .map(|(key, value)| (key.clone(), Value::from(value)))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArchivedRelation {
    pub id: u64,
    pub kind: String,
    pub from: u64,
    pub to: u64,
    pub properties: BTreeMap<String, ArchivedValue>,
}

impl From<&Relation> for ArchivedRelation {
    fn from(relation: &Relation) -> Self {
        Self {
            id: relation.id.0,
            kind: relation.kind.clone(),
            from: relation.from.0,
            to: relation.to.0,
            properties: relation
                .properties
                .iter()
                .map(|(key, value)| (key.clone(), ArchivedValue::from(value)))
                .collect(),
        }
    }
}

impl From<&ArchivedRelation> for Relation {
    fn from(relation: &ArchivedRelation) -> Self {
        Self {
            id: RelationId::new(relation.id),
            kind: relation.kind.clone(),
            from: EntityId::new(relation.from),
            to: EntityId::new(relation.to),
            properties: relation
                .properties
                .iter()
                .map(|(key, value)| (key.clone(), Value::from(value)))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ArchivedValue {
    Null,
    Bool(bool),
    Integer(i64),
    Text(String),
    Entity(u64),
    List(Vec<ArchivedValue>),
    Map(BTreeMap<String, ArchivedValue>),
}

impl From<&Value> for ArchivedValue {
    fn from(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(value) => Self::Bool(*value),
            Value::Integer(value) => Self::Integer(*value),
            Value::Text(value) => Self::Text(value.clone()),
            Value::Entity(value) => Self::Entity(value.0),
            Value::List(values) => Self::List(values.iter().map(Self::from).collect()),
            Value::Map(values) => Self::Map(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::from(value)))
                    .collect(),
            ),
        }
    }
}

impl From<&ArchivedValue> for Value {
    fn from(value: &ArchivedValue) -> Self {
        match value {
            ArchivedValue::Null => Self::Null,
            ArchivedValue::Bool(value) => Self::Bool(*value),
            ArchivedValue::Integer(value) => Self::Integer(*value),
            ArchivedValue::Text(value) => Self::Text(value.clone()),
            ArchivedValue::Entity(value) => Self::Entity(EntityId::new(*value)),
            ArchivedValue::List(values) => Self::List(values.iter().map(Self::from).collect()),
            ArchivedValue::Map(values) => Self::Map(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::from(value)))
                    .collect(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArchivedActionRequest {
    pub actor: Option<u64>,
    pub action: String,
    pub args: BTreeMap<String, ArchivedValue>,
    pub caused_by: Vec<u64>,
}

impl From<&ActionRequest> for ArchivedActionRequest {
    fn from(request: &ActionRequest) -> Self {
        Self {
            actor: request.actor.map(|id| id.0),
            action: request.action.clone(),
            args: request
                .args
                .iter()
                .map(|(key, value)| (key.clone(), ArchivedValue::from(value)))
                .collect(),
            caused_by: request.caused_by.iter().map(|id| id.0).collect(),
        }
    }
}

impl From<&ArchivedActionRequest> for ActionRequest {
    fn from(request: &ArchivedActionRequest) -> Self {
        Self {
            actor: request.actor.map(EntityId::new),
            action: request.action.clone(),
            args: request
                .args
                .iter()
                .map(|(key, value)| (key.clone(), Value::from(value)))
                .collect(),
            caused_by: request
                .caused_by
                .iter()
                .copied()
                .map(EventId::new)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArchivedScheduledAction {
    pub world_time: u64,
    pub request: ArchivedActionRequest,
}

impl From<&world_core::ScheduledAction> for ArchivedScheduledAction {
    fn from(scheduled: &world_core::ScheduledAction) -> Self {
        Self {
            world_time: scheduled.world_time,
            request: ArchivedActionRequest::from(&scheduled.request),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::{Action, ActionError, EventDraft};

    struct AddUnits;

    impl Action for AddUnits {
        fn name(&self) -> &'static str {
            "add_units"
        }

        fn evaluate(
            &self,
            state: &WorldState,
            request: &ActionRequest,
        ) -> Result<EventDraft, ActionError> {
            let amount = match request.args.get("amount") {
                Some(Value::Integer(value)) => *value,
                _ => return Err(ActionError::Invalid("missing amount".into())),
            };
            let entity = request
                .actor
                .ok_or_else(|| ActionError::Invalid("missing actor".into()))?;
            let current = match state
                .entity(entity)
                .and_then(|item| item.component("units"))
            {
                Some(Value::Integer(value)) => *value,
                _ => return Err(ActionError::Invalid("missing units".into())),
            };

            let mut draft = EventDraft::new("units_added");
            draft.actor = Some(entity);
            draft.targets = vec![entity];
            draft.payload.insert("amount".into(), amount.into());
            draft.changes.push(StateChange::SetComponent {
                entity,
                key: "units".into(),
                value: (current + amount).into(),
            });
            Ok(draft)
        }
    }

    fn baseline() -> WorldState {
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(EntityId::new(1), "counter").with_component("units", 0_i64))
            .unwrap();
        state
    }

    fn registry() -> ActionRegistry {
        let mut registry = ActionRegistry::new();
        registry.register(AddUnits).unwrap();
        registry
    }

    #[test]
    fn archive_round_trip_restores_history_time_and_pending_actions() {
        let registry = registry();
        let mut world = World::new(baseline());
        let first = world
            .execute(
                &registry,
                &ActionRequest::new("add_units")
                    .actor(EntityId::new(1))
                    .arg("amount", 3_i64),
            )
            .unwrap()
            .id;
        world.advance_to(&registry, 10).unwrap();
        world
            .schedule_at(
                20,
                ActionRequest::new("add_units")
                    .actor(EntityId::new(1))
                    .arg("amount", 7_i64)
                    .caused_by(first),
            )
            .unwrap();

        let pack = WorldPackRef::new("test.counter", "1");
        let archive = WorldArchive::capture(pack.clone(), &world).unwrap();
        let json = archive.to_json_pretty().unwrap();
        let decoded = WorldArchive::from_json(&json).unwrap();
        let mut restored = decoded.restore(&pack, baseline()).unwrap();

        assert_eq!(restored.world_time(), 10);
        assert_eq!(restored.events(), world.events());
        assert_eq!(restored.state(), world.state());
        let restored_pending = restored.scheduler().pending().collect::<Vec<_>>();
        assert_eq!(restored_pending.len(), 1);
        assert_eq!(restored_pending[0].world_time, 20);
        assert_eq!(restored_pending[0].request.action, "add_units");
        assert_eq!(restored_pending[0].request.caused_by, vec![first]);

        let generated = restored.advance_to(&registry, 20).unwrap();
        assert_eq!(generated.len(), 1);
        let second = restored.event(generated[0]).unwrap();
        assert_eq!(second.id, EventId::new(2));
        assert_eq!(second.caused_by, vec![first]);
        assert_eq!(
            restored
                .state()
                .entity(EntityId::new(1))
                .unwrap()
                .component("units"),
            Some(&Value::Integer(10))
        );
    }

    #[test]
    fn restore_rejects_a_different_world_pack() {
        let world = World::new(baseline());
        let archive =
            WorldArchive::capture(WorldPackRef::new("test.counter", "1"), &world).unwrap();
        let error = archive
            .restore(&WorldPackRef::new("other.pack", "1"), baseline())
            .unwrap_err();

        assert!(matches!(error, PersistenceError::PackMismatch { .. }));
    }

    #[test]
    fn archive_version_is_explicit_and_validated() {
        let world = World::new(baseline());
        let mut archive =
            WorldArchive::capture(WorldPackRef::new("test.counter", "1"), &world).unwrap();
        archive.format_version += 1;

        assert!(matches!(
            archive.to_json_pretty(),
            Err(PersistenceError::UnsupportedVersion(_))
        ));
    }
}
