use world_core::{EntityId, EventId, RelationId};
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
