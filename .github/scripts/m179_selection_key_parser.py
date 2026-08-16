from pathlib import Path

lib = Path("crates/world-projection/src/lib.rs")
text = lib.read_text()
old = '''impl SelectionId {
    pub fn stable_key(self) -> String {
        match self {
            Self::Entity(id) => format!("entity-{id}"),
            Self::Relation(id) => format!("relation-{id}"),
            Self::Event(id) => format!("event-{id}"),
        }
    }
}
'''
new = '''impl SelectionId {
    pub fn stable_key(self) -> String {
        match self {
            Self::Entity(id) => format!("entity-{id}"),
            Self::Relation(id) => format!("relation-{id}"),
            Self::Event(id) => format!("event-{id}"),
        }
    }

    pub fn from_stable_key(key: &str) -> Option<Self> {
        if let Some(id) = canonical_stable_id(key, "entity-") {
            return Some(Self::Entity(EntityId::new(id)));
        }
        if let Some(id) = canonical_stable_id(key, "relation-") {
            return Some(Self::Relation(RelationId::new(id)));
        }
        canonical_stable_id(key, "event-").map(|id| Self::Event(EventId::new(id)))
    }
}

fn canonical_stable_id(key: &str, prefix: &str) -> Option<u64> {
    let raw = key.strip_prefix(prefix)?;
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let id = raw.parse::<u64>().ok()?;
    (id.to_string() == raw).then_some(id)
}
'''
if text.count(old) != 1:
    raise SystemExit(f"expected SelectionId impl once, found {text.count(old)}")
text = text.replace(old, new, 1)

old_helper = '''fn entity_id_from_stable_key(key: &str) -> Option<EntityId> {
    key.strip_prefix("entity-")?
        .parse::<u64>()
        .ok()
        .map(EntityId::new)
}
'''
new_helper = '''fn entity_id_from_stable_key(key: &str) -> Option<EntityId> {
    match SelectionId::from_stable_key(key) {
        Some(SelectionId::Entity(entity)) => Some(entity),
        Some(SelectionId::Relation(_) | SelectionId::Event(_)) | None => None,
    }
}
'''
if text.count(old_helper) != 1:
    raise SystemExit(f"expected entity stable key helper once, found {text.count(old_helper)}")
text = text.replace(old_helper, new_helper, 1)
lib.write_text(text)

test = Path("crates/world-projection/tests/selection_stable_key.rs")
test.write_text(r'''use world_core::{EntityId, EventId, RelationId};
use world_projection::SelectionId;

#[test]
fn stable_selection_keys_round_trip_for_all_selection_kinds() {
    for selection in [
        SelectionId::Entity(EntityId::new(7)),
        SelectionId::Relation(RelationId::new(5)),
        SelectionId::Event(EventId::new(9)),
        SelectionId::Entity(EntityId::new(0)),
    ] {
        let key = selection.stable_key();
        assert_eq!(SelectionId::from_stable_key(&key), Some(selection));
    }
}

#[test]
fn stable_selection_key_parser_rejects_noncanonical_aliases_and_invalid_input() {
    for invalid in [
        "",
        "entity-",
        "relation-",
        "event-",
        "entity-07",
        "relation-05",
        "event-09",
        "entity-+7",
        "entity--7",
        "Entity-7",
        "entity-7 ",
        " entity-7",
        "entity-7-extra",
        "unknown-7",
        "entity-18446744073709551616",
    ] {
        assert_eq!(
            SelectionId::from_stable_key(invalid),
            None,
            "unexpectedly parsed {invalid:?}"
        );
    }
}

#[test]
fn stable_selection_key_parser_keeps_selection_kinds_distinct() {
    assert_eq!(
        SelectionId::from_stable_key("entity-5"),
        Some(SelectionId::Entity(EntityId::new(5)))
    );
    assert_eq!(
        SelectionId::from_stable_key("relation-5"),
        Some(SelectionId::Relation(RelationId::new(5)))
    );
    assert_eq!(
        SelectionId::from_stable_key("event-5"),
        Some(SelectionId::Event(EventId::new(5)))
    );
}
''')
