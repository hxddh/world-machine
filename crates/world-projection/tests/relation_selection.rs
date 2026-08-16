use world_core::RelationId;
use world_projection::SelectionId;

#[test]
fn relation_selection_has_a_stable_typed_key() {
    let selection = SelectionId::Relation(RelationId::new(42));
    assert_eq!(selection.stable_key(), "relation-42");
}
