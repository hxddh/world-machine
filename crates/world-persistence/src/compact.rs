//! The compact encoding of a World's history: the same archive as the
//! tagged one, written with short keys and values in their own JSON shape.
//!
//! A value is written as itself where JSON can hold it: `null`, `true`,
//! `3`, `"text"` and `[..]` for a list. An entity is `{"#": 12}`. A map is
//! an object, except that a map whose only key is `#` or `{}` is wrapped
//! as `{"{}": map}` so it is never read as anything else.
//!
//! The names of the values changes set (component and property keys) are
//! written once, in the archive's `keys` list, and a change names one by
//! its place in that list.
//!
//! A change is an array led by what it does. Changes of one kind to the
//! same thing, one after another, are written as one array:
//!
//! | change                      | written as                              |
//! |-----------------------------|-----------------------------------------|
//! | create entity               | `["e+", id, kind, {components}]`        |
//! | remove entity               | `["e-", id]`                            |
//! | set components              | `["s", entity, key, value, key, value]` |
//! | remove components           | `["u", entity, key, key]`               |
//! | create relation             | `["r+", id, kind, from, to, {..}]`      |
//! | remove relation             | `["r-", id]`                            |
//! | set relation properties     | `["rs", relation, key, value, ..]`      |
//! | remove relation properties  | `["ru", relation, key, ..]`             |
//!
//! An event is an object: `i` its id (left out when it is one more than the
//! last event's), `k` its kind, `t` its world time (left out when it is the
//! last event's), `a` its actor, `g` its targets, `c` what caused it, `p`
//! its payload and `x` its changes, each left out when empty.

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::{Map, Value as Json};
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};

use crate::{ArchivedEntity, ArchivedEvent, ArchivedRelation, ArchivedStateChange, ArchivedValue};

const ENTITY: &str = "#";
const MAP: &str = "{}";

pub(crate) struct CompactValue<'a>(pub &'a ArchivedValue);

impl Serialize for CompactValue<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            ArchivedValue::Null => serializer.serialize_unit(),
            ArchivedValue::Bool(value) => serializer.serialize_bool(*value),
            ArchivedValue::Integer(value) => serializer.serialize_i64(*value),
            ArchivedValue::Text(value) => serializer.serialize_str(value),
            ArchivedValue::Entity(id) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(ENTITY, id)?;
                map.end()
            }
            ArchivedValue::List(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(&CompactValue(value))?;
                }
                seq.end()
            }
            ArchivedValue::Map(values) => {
                let wrapped =
                    values.len() == 1 && values.keys().all(|key| key == ENTITY || key == MAP);
                if wrapped {
                    let mut map = serializer.serialize_map(Some(1))?;
                    map.serialize_entry(MAP, &CompactMap(values))?;
                    map.end()
                } else {
                    CompactMap(values).serialize(serializer)
                }
            }
        }
    }
}

pub(crate) struct CompactMap<'a>(pub &'a BTreeMap<String, ArchivedValue>);

impl Serialize for CompactMap<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in self.0 {
            map.serialize_entry(key, &CompactValue(value))?;
        }
        map.end()
    }
}

/// The names of the values an archive's changes set, each written once.
#[derive(Default)]
pub(crate) struct Keys<'a> {
    places: HashMap<&'a str, usize, BuildHasherDefault<QuickHasher>>,
    names: Vec<&'a str>,
}

/// A fast hash for short keys, as the Firefox and rustc hashers do: a save
/// looks up every changed value's name, hundreds of thousands of them, and
/// the names come from the World's own history, not from strangers.
#[derive(Default)]
pub(crate) struct QuickHasher(u64);

impl Hasher for QuickHasher {
    fn write(&mut self, bytes: &[u8]) {
        const K: u64 = 0x517c_c1b7_2722_0a95;
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.0 = (self.0.rotate_left(5) ^ u64::from_le_bytes(*word)).wrapping_mul(K);
        }
        for byte in rest {
            self.0 = (self.0.rotate_left(5) ^ u64::from(*byte)).wrapping_mul(K);
        }
    }

    fn write_u8(&mut self, byte: u8) {
        self.write(&[byte]);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

impl<'a> Keys<'a> {
    pub(crate) fn gather(&mut self, changes: &'a [ArchivedStateChange]) {
        for change in changes {
            let key = match change {
                ArchivedStateChange::SetComponent { key, .. }
                | ArchivedStateChange::RemoveComponent { key, .. }
                | ArchivedStateChange::SetRelationProperty { key, .. }
                | ArchivedStateChange::RemoveRelationProperty { key, .. } => key.as_str(),
                _ => continue,
            };
            let next = self.names.len();
            if *self.places.entry(key).or_insert(next) == next {
                self.names.push(key);
            }
        }
    }

    pub(crate) fn names(&self) -> &[&'a str] {
        &self.names
    }

    fn place(&self, key: &str) -> usize {
        self.places[key]
    }
}

/// One change's kind and the thing it changes, for telling which changes
/// one after another are written together.
fn run_of(change: &ArchivedStateChange) -> Option<(&'static str, u64)> {
    match change {
        ArchivedStateChange::SetComponent { entity, .. } => Some(("s", *entity)),
        ArchivedStateChange::RemoveComponent { entity, .. } => Some(("u", *entity)),
        ArchivedStateChange::SetRelationProperty { relation, .. } => Some(("rs", *relation)),
        ArchivedStateChange::RemoveRelationProperty { relation, .. } => Some(("ru", *relation)),
        _ => None,
    }
}

/// Changes of one kind to one thing, one after another.
struct CompactRun<'a, 'k> {
    changes: &'a [ArchivedStateChange],
    keys: &'k Keys<'a>,
}

impl Serialize for CompactRun<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(None)?;
        let first = &self.changes[0];
        if let Some((tag, target)) = run_of(first) {
            seq.serialize_element(tag)?;
            seq.serialize_element(&target)?;
            for change in self.changes {
                match change {
                    ArchivedStateChange::SetComponent { key, value, .. }
                    | ArchivedStateChange::SetRelationProperty { key, value, .. } => {
                        seq.serialize_element(&self.keys.place(key))?;
                        seq.serialize_element(&CompactValue(value))?;
                    }
                    ArchivedStateChange::RemoveComponent { key, .. }
                    | ArchivedStateChange::RemoveRelationProperty { key, .. } => {
                        seq.serialize_element(&self.keys.place(key))?;
                    }
                    _ => unreachable!("a run holds only value changes"),
                }
            }
            return seq.end();
        }
        match first {
            ArchivedStateChange::CreateEntity { entity } => {
                seq.serialize_element("e+")?;
                seq.serialize_element(&entity.id)?;
                seq.serialize_element(&entity.kind)?;
                seq.serialize_element(&CompactMap(&entity.components))?;
            }
            ArchivedStateChange::RemoveEntity { entity } => {
                seq.serialize_element("e-")?;
                seq.serialize_element(entity)?;
            }
            ArchivedStateChange::CreateRelation { relation } => {
                seq.serialize_element("r+")?;
                seq.serialize_element(&relation.id)?;
                seq.serialize_element(&relation.kind)?;
                seq.serialize_element(&relation.from)?;
                seq.serialize_element(&relation.to)?;
                seq.serialize_element(&CompactMap(&relation.properties))?;
            }
            ArchivedStateChange::RemoveRelation { relation } => {
                seq.serialize_element("r-")?;
                seq.serialize_element(relation)?;
            }
            _ => unreachable!("value changes are written as runs"),
        }
        seq.end()
    }
}

pub(crate) struct CompactChanges<'a, 'k> {
    pub changes: &'a [ArchivedStateChange],
    pub keys: &'k Keys<'a>,
}

impl Serialize for CompactChanges<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(None)?;
        let mut rest = self.changes;
        while !rest.is_empty() {
            let length = match run_of(&rest[0]) {
                Some(run) => rest
                    .iter()
                    .position(|change| run_of(change) != Some(run))
                    .unwrap_or(rest.len()),
                None => 1,
            };
            seq.serialize_element(&CompactRun {
                changes: &rest[..length],
                keys: self.keys,
            })?;
            rest = &rest[length..];
        }
        seq.end()
    }
}

/// A run of events, each written against the one before it.
pub(crate) struct CompactEvents<'a, 'k> {
    pub events: &'a [ArchivedEvent],
    pub keys: &'k Keys<'a>,
}

impl Serialize for CompactEvents<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.events.len()))?;
        let mut previous: Option<&ArchivedEvent> = None;
        for event in self.events {
            seq.serialize_element(&CompactEvent {
                event,
                previous,
                keys: self.keys,
            })?;
            previous = Some(event);
        }
        seq.end()
    }
}

struct CompactEvent<'a, 'k> {
    event: &'a ArchivedEvent,
    previous: Option<&'a ArchivedEvent>,
    keys: &'k Keys<'a>,
}

impl Serialize for CompactEvent<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let event = self.event;
        let mut map = serializer.serialize_map(None)?;
        if self.previous.map(|previous| previous.id.checked_add(1)) != Some(Some(event.id)) {
            map.serialize_entry("i", &event.id)?;
        }
        map.serialize_entry("k", &event.kind)?;
        if self.previous.map(|previous| previous.world_time) != Some(event.world_time) {
            map.serialize_entry("t", &event.world_time)?;
        }
        if let Some(actor) = event.actor {
            map.serialize_entry("a", &actor)?;
        }
        if !event.targets.is_empty() {
            map.serialize_entry("g", &event.targets)?;
        }
        if !event.caused_by.is_empty() {
            map.serialize_entry("c", &event.caused_by)?;
        }
        if !event.payload.is_empty() {
            map.serialize_entry("p", &CompactMap(&event.payload))?;
        }
        if !event.changes.is_empty() {
            map.serialize_entry(
                "x",
                &CompactChanges {
                    changes: &event.changes,
                    keys: self.keys,
                },
            )?;
        }
        map.end()
    }
}

/// Why a compact archive could not be read: the path to what was wrong.
pub(crate) type Invalid = String;

fn invalid<T>(what: &str) -> Result<T, Invalid> {
    Err(format!("invalid compact {what}"))
}

pub(crate) fn value_from(json: &Json) -> Result<ArchivedValue, Invalid> {
    Ok(match json {
        Json::Null => ArchivedValue::Null,
        Json::Bool(value) => ArchivedValue::Bool(*value),
        Json::Number(number) => match number.as_i64() {
            Some(value) => ArchivedValue::Integer(value),
            None => return invalid("integer"),
        },
        Json::String(text) => ArchivedValue::Text(text.clone()),
        Json::Array(values) => {
            ArchivedValue::List(values.iter().map(value_from).collect::<Result<_, _>>()?)
        }
        Json::Object(object) => {
            if object.len() == 1 {
                if let Some(id) = object.get(ENTITY) {
                    return Ok(ArchivedValue::Entity(u64_from(id, "entity")?));
                }
                if let Some(Json::Object(inner)) = object.get(MAP) {
                    return Ok(ArchivedValue::Map(map_from(inner)?));
                }
                if object.contains_key(MAP) {
                    return invalid("map");
                }
            }
            ArchivedValue::Map(map_from(object)?)
        }
    })
}

pub(crate) fn map_from(
    object: &Map<String, Json>,
) -> Result<BTreeMap<String, ArchivedValue>, Invalid> {
    object
        .iter()
        .map(|(key, value)| Ok((key.clone(), value_from(value)?)))
        .collect()
}

fn object_map_from(json: &Json, what: &str) -> Result<BTreeMap<String, ArchivedValue>, Invalid> {
    match json {
        Json::Object(object) => map_from(object),
        _ => invalid(what),
    }
}

fn u64_from(json: &Json, what: &str) -> Result<u64, Invalid> {
    json.as_u64().map_or_else(|| invalid(what), Ok)
}

fn string_from(json: &Json, what: &str) -> Result<String, Invalid> {
    json.as_str()
        .map_or_else(|| invalid(what), |text| Ok(text.to_owned()))
}

fn ids_from(json: &Json, what: &str) -> Result<Vec<u64>, Invalid> {
    match json {
        Json::Array(values) => values.iter().map(|value| u64_from(value, what)).collect(),
        _ => invalid(what),
    }
}

fn key_from(json: &Json, keys: &[String]) -> Result<String, Invalid> {
    match json {
        Json::Number(_) => json
            .as_u64()
            .and_then(|place| keys.get(usize::try_from(place).ok()?))
            .map_or_else(|| invalid("key"), |key| Ok(key.clone())),
        Json::String(key) => Ok(key.clone()),
        _ => invalid("key"),
    }
}

/// Reads one written change, which may stand for several, onto `out`.
fn change_from(
    json: &Json,
    keys: &[String],
    out: &mut Vec<ArchivedStateChange>,
) -> Result<(), Invalid> {
    let Json::Array(parts) = json else {
        return invalid("change");
    };
    let tag = parts.first().and_then(Json::as_str).unwrap_or_default();
    let part = |index: usize| parts.get(index).unwrap_or(&Json::Null);
    let arity = |count: usize| {
        if parts.len() == count {
            Ok(())
        } else {
            invalid("change")
        }
    };
    match tag {
        "e+" => {
            arity(4)?;
            out.push(ArchivedStateChange::CreateEntity {
                entity: ArchivedEntity {
                    id: u64_from(part(1), "entity id")?,
                    kind: string_from(part(2), "entity kind")?,
                    components: object_map_from(part(3), "components")?,
                },
            });
        }
        "e-" => {
            arity(2)?;
            out.push(ArchivedStateChange::RemoveEntity {
                entity: u64_from(part(1), "entity id")?,
            });
        }
        "r+" => {
            arity(6)?;
            out.push(ArchivedStateChange::CreateRelation {
                relation: ArchivedRelation {
                    id: u64_from(part(1), "relation id")?,
                    kind: string_from(part(2), "relation kind")?,
                    from: u64_from(part(3), "relation end")?,
                    to: u64_from(part(4), "relation end")?,
                    properties: object_map_from(part(5), "properties")?,
                },
            });
        }
        "r-" => {
            arity(2)?;
            out.push(ArchivedStateChange::RemoveRelation {
                relation: u64_from(part(1), "relation id")?,
            });
        }
        "s" | "rs" => {
            let target = u64_from(part(1), "target")?;
            let pairs = &parts[2.min(parts.len())..];
            if pairs.is_empty() || pairs.len() % 2 != 0 {
                return invalid("change");
            }
            for pair in pairs.chunks(2) {
                let key = key_from(&pair[0], keys)?;
                let value = value_from(&pair[1])?;
                out.push(if tag == "s" {
                    ArchivedStateChange::SetComponent {
                        entity: target,
                        key,
                        value,
                    }
                } else {
                    ArchivedStateChange::SetRelationProperty {
                        relation: target,
                        key,
                        value,
                    }
                });
            }
        }
        "u" | "ru" => {
            let target = u64_from(part(1), "target")?;
            let names = &parts[2.min(parts.len())..];
            if names.is_empty() {
                return invalid("change");
            }
            for name in names {
                let key = key_from(name, keys)?;
                out.push(if tag == "u" {
                    ArchivedStateChange::RemoveComponent {
                        entity: target,
                        key,
                    }
                } else {
                    ArchivedStateChange::RemoveRelationProperty {
                        relation: target,
                        key,
                    }
                });
            }
        }
        _ => return invalid("change"),
    }
    Ok(())
}

pub(crate) fn changes_from(
    json: &Json,
    keys: &[String],
) -> Result<Vec<ArchivedStateChange>, Invalid> {
    let Json::Array(changes) = json else {
        return invalid("changes");
    };
    let mut out = Vec::with_capacity(changes.len());
    for change in changes {
        change_from(change, keys, &mut out)?;
    }
    Ok(out)
}

pub(crate) fn keys_from(json: Option<&Json>) -> Result<Vec<String>, Invalid> {
    match json {
        None => Ok(Vec::new()),
        Some(Json::Array(keys)) => keys.iter().map(|key| string_from(key, "keys")).collect(),
        Some(_) => invalid("keys"),
    }
}

pub(crate) fn events_from(json: &Json, keys: &[String]) -> Result<Vec<ArchivedEvent>, Invalid> {
    let Json::Array(items) = json else {
        return invalid("events");
    };
    let mut events: Vec<ArchivedEvent> = Vec::with_capacity(items.len());
    for item in items {
        let Json::Object(object) = item else {
            return invalid("event");
        };
        let previous = events.last();
        let id = match object.get("i") {
            Some(id) => u64_from(id, "event id")?,
            None => match previous.and_then(|previous| previous.id.checked_add(1)) {
                Some(id) => id,
                None => return invalid("event id"),
            },
        };
        let world_time = match object.get("t") {
            Some(time) => u64_from(time, "event time")?,
            None => match previous {
                Some(previous) => previous.world_time,
                None => return invalid("event time"),
            },
        };
        let event = ArchivedEvent {
            id,
            kind: string_from(object.get("k").unwrap_or(&Json::Null), "event kind")?,
            world_time,
            actor: object
                .get("a")
                .map(|actor| u64_from(actor, "actor"))
                .transpose()?,
            targets: object
                .get("g")
                .map(|targets| ids_from(targets, "targets"))
                .transpose()?
                .unwrap_or_default(),
            caused_by: object
                .get("c")
                .map(|causes| ids_from(causes, "causes"))
                .transpose()?
                .unwrap_or_default(),
            payload: object
                .get("p")
                .map(|payload| object_map_from(payload, "payload"))
                .transpose()?
                .unwrap_or_default(),
            changes: object
                .get("x")
                .map(|changes| changes_from(changes, keys))
                .transpose()?
                .unwrap_or_default(),
        };
        events.push(event);
    }
    Ok(events)
}
