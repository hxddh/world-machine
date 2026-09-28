use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt;
use world_core::{
    ActionRegistry, ActionRequest, Entity, EntityId, Event, EventId, Relation, RelationId,
    StateChange, Value, World, WorldError, WorldState,
};

pub const WORLD_ARCHIVE_FORMAT: &str = "world-machine";
pub const WORLD_ARCHIVE_VERSION: u32 = 1;
/// The version a World file says when its archive is written in the
/// compact encoding (described in `compact.rs`). The archive it holds is
/// the same archive, so it reads back as [`WORLD_ARCHIVE_VERSION`].
pub const WORLD_ARCHIVE_COMPACT_VERSION: u32 = 2;

mod compact;
use compact::{CompactChanges, CompactEvents, Keys};

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
    /// Where the World stood after the events before `events`, or after
    /// the first of them: restoring starts from it, and an archive that
    /// keeps only the events after it (a World code's) opens only with it.
    /// A World capturing itself leaves it out; its file and its code carry
    /// it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<ArchivedCheckpoint>,
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
            checkpoint: None,
        })
    }

    pub fn to_json_pretty(&self) -> Result<String, PersistenceError> {
        self.validate_header()?;
        serde_json::to_string_pretty(self).map_err(PersistenceError::Json)
    }

    /// Reads an archive written in either encoding.
    pub fn from_json(json: &str) -> Result<Self, PersistenceError> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(PersistenceError::Json)?;
        Self::from_json_value(&value)
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
        let checkpoint = self
            .checkpoint
            .as_ref()
            .and_then(|checkpoint| Some((checkpoint.fit(self)?, checkpoint)));
        let mut world = match checkpoint {
            Some((fit, checkpoint)) => {
                let settled = checkpoint
                    .changes
                    .iter()
                    .map(StateChange::from)
                    .collect::<Vec<_>>();
                let from = match fit {
                    CheckpointFit::Within => checkpoint.events,
                    CheckpointFit::Before => 0,
                };
                World::resume(
                    baseline,
                    &settled,
                    checkpoint.world_time,
                    checkpoint.last_event + 1,
                    events,
                    from,
                )
            }
            None => World::from_history(baseline, &events),
        }
        .map_err(PersistenceError::World)?;

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

    /// The archive in the compact encoding, as one JSON object, with any
    /// further top-level fields (such as a World document's own) alongside.
    pub fn to_compact_json(
        &self,
        extra: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Vec<u8>, PersistenceError> {
        self.to_compact_json_with(self.checkpoint.as_ref(), extra)
    }

    /// [`Self::to_compact_json`], written with `checkpoint` in place of the
    /// archive's own.
    pub fn to_compact_json_with(
        &self,
        checkpoint: Option<&ArchivedCheckpoint>,
        extra: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<Vec<u8>, PersistenceError> {
        self.validate_header()?;
        let mut keys = Keys::default();
        if let Some(checkpoint) = checkpoint {
            keys.gather(&checkpoint.changes);
        }
        for event in &self.events {
            keys.gather(&event.changes);
        }
        let written = CompactArchiveOut {
            format: &self.format,
            format_version: WORLD_ARCHIVE_COMPACT_VERSION,
            pack: &self.pack,
            world_time: self.world_time,
            pending: &self.pending,
            keys: keys.names(),
            checkpoint: checkpoint.map(|checkpoint| CompactCheckpoint {
                checkpoint,
                keys: &keys,
            }),
            events: CompactEvents {
                events: &self.events,
                keys: &keys,
            },
            extra,
        };
        serde_json::to_vec(&written).map_err(PersistenceError::Json)
    }

    /// Reads an archive in either encoding from a parsed JSON object:
    /// tagged (as every World file was written before the compact one) or
    /// compact.
    pub fn from_json_value(value: &serde_json::Value) -> Result<Self, PersistenceError> {
        let compact = value
            .get("format_version")
            .and_then(serde_json::Value::as_u64)
            == Some(u64::from(WORLD_ARCHIVE_COMPACT_VERSION));
        let archive = if compact {
            let header = ArchiveHeader::deserialize(value).map_err(PersistenceError::Json)?;
            let keys = compact::keys_from(value.get("keys")).map_err(PersistenceError::Compact)?;
            let events = compact::events_from(
                value.get("events").unwrap_or(&serde_json::Value::Null),
                &keys,
            )
            .map_err(PersistenceError::Compact)?;
            Self {
                format: header.format,
                format_version: WORLD_ARCHIVE_VERSION,
                pack: header.pack,
                world_time: header.world_time,
                events,
                pending: header.pending,
                checkpoint: ArchivedCheckpoint::from_compact(value)?,
            }
        } else {
            Self::deserialize(value).map_err(PersistenceError::Json)?
        };
        archive.validate_header()?;
        Ok(archive)
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
    /// A compact archive that does not read as one.
    Compact(String),
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
            Self::Compact(error) => write!(f, "invalid world archive: {error}"),
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
            Self::Compact(_)
            | Self::UnsupportedFormat(_)
            | Self::UnsupportedVersion(_)
            | Self::InvalidPack
            | Self::PackMismatch { .. } => None,
        }
    }
}

#[derive(Serialize)]
struct CompactArchiveOut<'a> {
    format: &'a str,
    format_version: u32,
    pack: &'a WorldPackRef,
    world_time: u64,
    pending: &'a [ArchivedScheduledAction],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    keys: &'a [&'a str],
    #[serde(skip_serializing_if = "Option::is_none")]
    checkpoint: Option<CompactCheckpoint<'a, 'a>>,
    events: CompactEvents<'a, 'a>,
    #[serde(flatten)]
    extra: &'a serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct ArchiveHeader {
    format: String,
    pack: WorldPackRef,
    world_time: u64,
    #[serde(default)]
    pending: Vec<ArchivedScheduledAction>,
}

struct CompactCheckpoint<'a, 'k> {
    checkpoint: &'a ArchivedCheckpoint,
    keys: &'k Keys<'a>,
}

impl Serialize for CompactCheckpoint<'_, '_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let checkpoint = self.checkpoint;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("events", &checkpoint.events)?;
        map.serialize_entry("last_event", &checkpoint.last_event)?;
        map.serialize_entry("world_time", &checkpoint.world_time)?;
        map.serialize_entry(
            "changes",
            &CompactChanges {
                changes: &checkpoint.changes,
                keys: self.keys,
            },
        )?;
        map.end()
    }
}

/// Where a World stood after the first events of its history, summed up as
/// the changes that take its Pack's starting state there. Replay can start
/// from it instead of from the first event; a history that keeps only what
/// came after it (a World code does) opens only with it.
///
/// It is derived from the recorded events alone, so it never needs the
/// Pack, and it is exact: applying [`ArchivedCheckpoint::changes`] to the
/// starting state gives the same state as applying every event it sums up.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ArchivedCheckpoint {
    /// How many events, from the first, it sums up.
    pub events: usize,
    /// The id of the last of them, or 0 for none.
    pub last_event: u64,
    /// The world time of the last of them.
    pub world_time: u64,
    /// The changes they came to, with every change a later one undid or
    /// replaced left out.
    pub changes: Vec<ArchivedStateChange>,
}

/// How a checkpoint stands to an archive's events.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointFit {
    /// The archive holds the events it sums up, and more after them.
    Within,
    /// The archive holds only events after it.
    Before,
}

impl ArchivedCheckpoint {
    /// The checkpoint that sums up these events, the first of a history.
    pub fn covering(events: &[ArchivedEvent]) -> Self {
        let mut checkpoint = Self::default();
        checkpoint.advance(events);
        checkpoint
    }

    /// Moves the checkpoint on over the events that came right after it.
    pub fn advance(&mut self, events: &[ArchivedEvent]) {
        let Some(last) = events.last() else {
            return;
        };
        let existing = std::mem::take(&mut self.changes);
        self.changes = settle(
            existing.into_iter().chain(
                events
                    .iter()
                    .flat_map(|event| event.changes.iter().cloned()),
            ),
        );
        self.events += events.len();
        self.last_event = last.id;
        self.world_time = last.world_time;
    }

    /// Whether this checkpoint belongs to the archive's history, and how.
    pub fn fit(&self, archive: &WorldArchive) -> Option<CheckpointFit> {
        if archive.world_time < self.world_time {
            return None;
        }
        if self.events == 0 {
            return Some(CheckpointFit::Within);
        }
        let covered = archive.events.get(self.events - 1);
        if covered
            .is_some_and(|event| event.id == self.last_event && event.world_time == self.world_time)
        {
            return Some(CheckpointFit::Within);
        }
        let first = archive.events.first();
        if first
            .is_none_or(|event| event.id > self.last_event && event.world_time >= self.world_time)
        {
            return Some(CheckpointFit::Before);
        }
        None
    }

    /// The checkpoint for the archive's history up to (not including) the
    /// first event at or after `world_time`, carried on from `self` where
    /// it still fits, and started again where it does not.
    pub fn at(previous: Option<&Self>, archive: &WorldArchive, world_time: u64) -> Self {
        let end = archive
            .events
            .partition_point(|event| event.world_time < world_time);
        match previous {
            Some(previous)
                if previous.fit(archive) == Some(CheckpointFit::Within)
                    && previous.events <= end =>
            {
                let mut next = previous.clone();
                next.advance(&archive.events[previous.events..end]);
                next
            }
            _ => Self::covering(&archive.events[..end]),
        }
    }

    /// The checkpoint a compact archive keeps, if it keeps one. `value` is
    /// the whole object the archive is read from.
    fn from_compact(value: &serde_json::Value) -> Result<Option<Self>, PersistenceError> {
        let Some(checkpoint) = value.get("checkpoint") else {
            return Ok(None);
        };
        let number = |key: &str| {
            checkpoint
                .get(key)
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| PersistenceError::Compact(format!("checkpoint without {key}")))
        };
        let keys = compact::keys_from(value.get("keys")).map_err(PersistenceError::Compact)?;
        Ok(Some(Self {
            events: usize::try_from(number("events")?)
                .map_err(|_| PersistenceError::Compact("checkpoint too long".into()))?,
            last_event: number("last_event")?,
            world_time: number("world_time")?,
            changes: compact::changes_from(
                checkpoint
                    .get("changes")
                    .unwrap_or(&serde_json::Value::Null),
                &keys,
            )
            .map_err(PersistenceError::Compact)?,
        }))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Target {
    Entity(u64),
    Relation(u64),
}

/// The changes that come to the same as `changes` applied in order: a
/// value set on something created along the way is folded into its
/// creation, and a value set again, or on something removed or created
/// anew, is left out. Every creation and removal is kept, so every change
/// kept meets the same World it met in the full run.
fn settle(changes: impl Iterator<Item = ArchivedStateChange>) -> Vec<ArchivedStateChange> {
    let mut out: Vec<Option<ArchivedStateChange>> = Vec::new();
    // Where the live creation of each thing is, while nothing has removed it.
    let mut created: HashMap<Target, usize> = HashMap::new();
    // Where the latest write of each value of things not created here is.
    let mut written: HashMap<Target, HashMap<String, usize>> = HashMap::new();
    for change in changes {
        let (target, key) = match &change {
            ArchivedStateChange::CreateEntity { entity } => {
                let target = Target::Entity(entity.id);
                forget(&mut out, &mut written, target);
                created.insert(target, out.len());
                out.push(Some(change));
                continue;
            }
            ArchivedStateChange::CreateRelation { relation } => {
                let target = Target::Relation(relation.id);
                forget(&mut out, &mut written, target);
                created.insert(target, out.len());
                out.push(Some(change));
                continue;
            }
            ArchivedStateChange::RemoveEntity { entity } => {
                let target = Target::Entity(*entity);
                forget(&mut out, &mut written, target);
                created.remove(&target);
                out.push(Some(change));
                continue;
            }
            ArchivedStateChange::RemoveRelation { relation } => {
                let target = Target::Relation(*relation);
                forget(&mut out, &mut written, target);
                created.remove(&target);
                out.push(Some(change));
                continue;
            }
            ArchivedStateChange::SetComponent { entity, key, .. }
            | ArchivedStateChange::RemoveComponent { entity, key } => {
                (Target::Entity(*entity), key)
            }
            ArchivedStateChange::SetRelationProperty { relation, key, .. }
            | ArchivedStateChange::RemoveRelationProperty { relation, key } => {
                (Target::Relation(*relation), key)
            }
        };
        if let Some(at) = created.get(&target) {
            if let Some(creation) = out[*at].as_mut() {
                fold_into(creation, change);
                continue;
            }
        }
        let key = key.clone();
        let at = out.len();
        out.push(Some(change));
        if let Some(earlier) = written.entry(target).or_default().insert(key, at) {
            out[earlier] = None;
        }
    }
    out.into_iter().flatten().collect()
}

fn forget(
    out: &mut [Option<ArchivedStateChange>],
    written: &mut HashMap<Target, HashMap<String, usize>>,
    target: Target,
) {
    for at in written
        .remove(&target)
        .into_iter()
        .flat_map(HashMap::into_values)
    {
        out[at] = None;
    }
}

fn fold_into(creation: &mut ArchivedStateChange, change: ArchivedStateChange) {
    let values = match creation {
        ArchivedStateChange::CreateEntity { entity } => &mut entity.components,
        ArchivedStateChange::CreateRelation { relation } => &mut relation.properties,
        _ => unreachable!("only creations are folded into"),
    };
    match change {
        ArchivedStateChange::SetComponent { key, value, .. }
        | ArchivedStateChange::SetRelationProperty { key, value, .. } => {
            values.insert(key, value);
        }
        ArchivedStateChange::RemoveComponent { key, .. }
        | ArchivedStateChange::RemoveRelationProperty { key, .. } => {
            values.remove(&key);
        }
        _ => unreachable!("only values are folded"),
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

    fn every_kind_of_value() -> Value {
        let mut map = BTreeMap::new();
        map.insert("plain".to_string(), Value::Integer(-4));
        map.insert("#".to_string(), Value::Text("looks like an entity".into()));
        let mut lone_hash = BTreeMap::new();
        lone_hash.insert("#".to_string(), Value::Integer(7));
        let mut lone_braces = BTreeMap::new();
        lone_braces.insert("{}".to_string(), Value::Map(BTreeMap::new()));
        Value::List(vec![
            Value::Null,
            Value::Bool(true),
            Value::Integer(i64::MIN),
            Value::Integer(i64::MAX),
            Value::Text("\"quoted\" and ☃".into()),
            Value::Entity(EntityId::new(u64::MAX)),
            Value::Map(map),
            Value::Map(lone_hash),
            Value::Map(lone_braces),
            Value::Map(BTreeMap::new()),
            Value::List(Vec::new()),
        ])
    }

    fn every_kind_of_change() -> Vec<StateChange> {
        let entity = EntityId::new(5);
        let relation = RelationId::new(9);
        vec![
            StateChange::CreateEntity(
                Entity::new(entity, "thing").with_component("odd", every_kind_of_value()),
            ),
            StateChange::SetComponent {
                entity,
                key: "a".into(),
                value: every_kind_of_value(),
            },
            StateChange::SetComponent {
                entity,
                key: "b".into(),
                value: 2_i64.into(),
            },
            StateChange::RemoveComponent {
                entity,
                key: "a".into(),
            },
            StateChange::RemoveComponent {
                entity,
                key: "b".into(),
            },
            StateChange::SetComponent {
                entity: EntityId::new(1),
                key: "a".into(),
                value: 1_i64.into(),
            },
            StateChange::CreateRelation(
                Relation::new(relation, "knows", EntityId::new(1), entity)
                    .with_property("since", 3_i64),
            ),
            StateChange::SetRelationProperty {
                relation,
                key: "since".into(),
                value: 4_i64.into(),
            },
            StateChange::RemoveRelationProperty {
                relation,
                key: "since".into(),
            },
            StateChange::RemoveRelation(relation),
            StateChange::RemoveEntity(entity),
        ]
    }

    fn event(id: u64, world_time: u64, changes: Vec<StateChange>) -> Event {
        let mut payload = BTreeMap::new();
        payload.insert("said".to_string(), Value::Text("Hello".into()));
        payload.insert("odd".to_string(), every_kind_of_value());
        Event {
            id: EventId::new(id),
            kind: "something".into(),
            world_time,
            actor: id.is_multiple_of(2).then(|| EntityId::new(1)),
            targets: vec![EntityId::new(1)],
            caused_by: (id > 1).then(|| EventId::new(id - 1)).into_iter().collect(),
            payload,
            changes,
        }
    }

    fn compact_round_trip(archive: &WorldArchive) -> WorldArchive {
        let json = archive.to_compact_json(&serde_json::Map::new()).unwrap();
        WorldArchive::from_json(std::str::from_utf8(&json).unwrap()).unwrap()
    }

    #[test]
    fn the_compact_encoding_reads_back_every_value_and_change() {
        let mut archive = WorldArchive::capture(
            WorldPackRef::new("test.counter", "1"),
            &World::new(baseline()),
        )
        .unwrap();
        archive.world_time = 40;
        archive.events = [
            event(1, 3, every_kind_of_change()),
            // The same time, the next id; then ids and times that jump.
            event(2, 3, Vec::new()),
            event(7, 3, every_kind_of_change()),
            event(8, 40, vec![]),
        ]
        .iter()
        .map(ArchivedEvent::from)
        .collect();
        assert_eq!(compact_round_trip(&archive), archive);

        let tagged = serde_json::to_vec(&archive).unwrap();
        let compact = archive.to_compact_json(&serde_json::Map::new()).unwrap();
        assert!(
            compact.len() * 2 < tagged.len(),
            "{} vs {}",
            compact.len(),
            tagged.len()
        );
    }

    #[test]
    fn a_tagged_archive_still_reads() {
        let registry = registry();
        let mut world = World::new(baseline());
        world
            .execute(
                &registry,
                &ActionRequest::new("add_units")
                    .actor(EntityId::new(1))
                    .arg("amount", 3_i64),
            )
            .unwrap();
        let archive =
            WorldArchive::capture(WorldPackRef::new("test.counter", "1"), &world).unwrap();
        let tagged = archive.to_json_pretty().unwrap();
        assert!(tagged.contains("\"set_component\""));
        assert_eq!(WorldArchive::from_json(&tagged).unwrap(), archive);
        assert_eq!(compact_round_trip(&archive), archive);
    }

    #[test]
    fn a_damaged_compact_archive_is_refused() {
        let archive = WorldArchive::capture(
            WorldPackRef::new("test.counter", "1"),
            &World::new(baseline()),
        )
        .unwrap();
        let json =
            String::from_utf8(archive.to_compact_json(&serde_json::Map::new()).unwrap()).unwrap();
        for bad in [
            json.replace("\"events\":[]", "\"events\":[{\"k\":\"x\"}]"),
            json.replace(
                "\"events\":[]",
                "\"events\":[{\"k\":\"x\",\"t\":1,\"x\":[[\"s\",1,9,1]]}]",
            ),
            json.replace(
                "\"events\":[]",
                "\"events\":[{\"k\":\"x\",\"t\":1,\"x\":[[\"zz\",1]]}]",
            ),
            json.replace(
                "\"events\":[]",
                "\"events\":[{\"k\":\"x\",\"t\":1,\"p\":{\"f\":1.5}}]",
            ),
        ] {
            assert!(WorldArchive::from_json(&bad).is_err(), "{bad}");
        }
    }

    /// A little World that makes, changes and unmakes things and their
    /// relations, to sum up.
    fn busy_history() -> (WorldState, Vec<Event>) {
        let mut events = Vec::new();
        let mut time = 0;
        for round in 0..6_u64 {
            let thing = EntityId::new(10 + round % 3);
            let relation = RelationId::new(20 + round % 2);
            time += 1;
            let mut changes = Vec::new();
            if round >= 3 {
                changes.push(StateChange::RemoveEntity(thing));
            }
            changes.extend([
                StateChange::CreateEntity(
                    Entity::new(thing, "thing").with_component("made", round as i64),
                ),
                StateChange::SetComponent {
                    entity: thing,
                    key: "made".into(),
                    value: (round as i64 * 2).into(),
                },
                StateChange::SetComponent {
                    entity: EntityId::new(1),
                    key: "units".into(),
                    value: (round as i64).into(),
                },
                StateChange::SetComponent {
                    entity: EntityId::new(1),
                    key: format!("seen.{round}"),
                    value: true.into(),
                },
            ]);
            if round.is_multiple_of(2) {
                changes.push(StateChange::RemoveComponent {
                    entity: EntityId::new(1),
                    key: format!("seen.{}", round.saturating_sub(2)),
                });
            }
            if round >= 2 {
                changes.push(StateChange::RemoveRelation(relation));
            }
            changes.extend([
                StateChange::CreateRelation(Relation::new(
                    relation,
                    "near",
                    EntityId::new(1),
                    thing,
                )),
                StateChange::SetRelationProperty {
                    relation,
                    key: "how".into(),
                    value: (round as i64).into(),
                },
                StateChange::SetRelationProperty {
                    relation: RelationId::new(30),
                    key: "how".into(),
                    value: (round as i64).into(),
                },
            ]);
            events.push(event(round + 1, time, changes));
        }
        let mut start = baseline();
        start
            .seed_entity(Entity::new(EntityId::new(2), "other"))
            .unwrap();
        start
            .seed_relation(Relation::new(
                RelationId::new(30),
                "knows",
                EntityId::new(1),
                EntityId::new(2),
            ))
            .unwrap();
        (start, events)
    }

    #[test]
    fn a_checkpoint_sums_up_its_events_exactly() {
        let (start, events) = busy_history();
        let full = World::from_history(start.clone(), &events).unwrap();
        let archive = WorldArchive::capture(WorldPackRef::new("test.counter", "1"), &full).unwrap();
        let pack = WorldPackRef::new("test.counter", "1");
        let mut carried: Option<ArchivedCheckpoint> = None;
        for time in 0..=8 {
            let checkpoint = ArchivedCheckpoint::at(carried.as_ref(), &archive, time);
            let covered = archive
                .events
                .partition_point(|event| event.world_time < time);
            assert_eq!(
                checkpoint,
                ArchivedCheckpoint::covering(&archive.events[..covered])
            );
            assert_eq!(checkpoint.events, covered);
            assert!(checkpoint.fit(&archive) == Some(CheckpointFit::Within));

            // Applied to the start, it is where the events took the World.
            let settled = World::resume(
                start.clone(),
                &checkpoint
                    .changes
                    .iter()
                    .map(StateChange::from)
                    .collect::<Vec<_>>(),
                checkpoint.world_time,
                checkpoint.last_event + 1,
                Vec::new(),
                0,
            )
            .unwrap();
            let mut partial = World::from_history(start.clone(), &events[..covered]).unwrap();
            if covered == 0 {
                partial = World::new(start.clone());
            }
            assert_eq!(settled.state(), partial.state(), "at {time}");

            // Restored with it, whole or from it on, it is the full replay.
            let mut whole = archive.clone();
            whole.checkpoint = Some(checkpoint.clone());
            let restored = whole.restore(&pack, start.clone()).unwrap();
            assert_eq!(restored.state(), full.state());
            assert_eq!(restored.events(), full.events());
            let mut tail = whole.clone();
            tail.events = archive.events[covered..].to_vec();
            if covered > 0 {
                assert_eq!(checkpoint.fit(&tail), Some(CheckpointFit::Before));
            }
            let restored = tail.restore(&pack, start.clone()).unwrap();
            assert_eq!(restored.state(), full.state());
            assert_eq!(restored.events(), &full.events()[covered..]);

            // It reads back as written, in either encoding.
            let json = tail.to_compact_json(&serde_json::Map::new()).unwrap();
            let read = WorldArchive::from_json(std::str::from_utf8(&json).unwrap()).unwrap();
            assert_eq!(read, tail);
            let tagged = tail.to_json_pretty().unwrap();
            assert_eq!(WorldArchive::from_json(&tagged).unwrap(), tail);
            carried = Some(checkpoint);
        }
        // The last one leaves out what later changes replaced.
        let changes = events
            .iter()
            .map(|event| event.changes.len())
            .sum::<usize>();
        assert!(carried.unwrap().changes.len() < changes);
    }

    #[test]
    fn a_checkpoint_from_another_history_is_not_used() {
        let (start, events) = busy_history();
        let world = World::from_history(start.clone(), &events).unwrap();
        let archive =
            WorldArchive::capture(WorldPackRef::new("test.counter", "1"), &world).unwrap();
        let mut other = ArchivedCheckpoint::covering(&archive.events[..3]);
        other.last_event = 99;
        let mut archive = archive;
        archive.checkpoint = Some(other.clone());
        assert_eq!(other.fit(&archive), None);
        let restored = archive
            .restore(&WorldPackRef::new("test.counter", "1"), start.clone())
            .unwrap();
        assert_eq!(restored.state(), world.state());
    }
}
