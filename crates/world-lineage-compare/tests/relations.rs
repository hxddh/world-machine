use world_document::{WorldBranchCause, WorldLineage, WorldParent};
use world_library::WorldDocumentId;
use world_lineage::{build_index, LineageRecord};
use world_lineage_compare::{relation_between, SavedWorldRelation, SavedWorldRelationError};
use world_persistence::WorldPackRef;

fn id(value: &str) -> WorldDocumentId {
    WorldDocumentId::new(value).unwrap()
}

fn pack() -> WorldPackRef {
    WorldPackRef::new("world-machine.relation-test", "1")
}

fn root(value: &str) -> LineageRecord {
    LineageRecord {
        id: id(value),
        pack: pack(),
        world_time: 0,
        event_count: 0,
        lineage: None,
    }
}

fn child(value: &str, parent: &str) -> LineageRecord {
    LineageRecord {
        id: id(value),
        pack: pack(),
        world_time: 10,
        event_count: 1,
        lineage: Some(WorldLineage {
            parent: WorldParent {
                document: Some(parent.into()),
                pack: pack(),
                world_time: 0,
                event_count: 0,
            },
            branch: WorldBranchCause::Fork { label: None },
        }),
    }
}

#[test]
fn classifies_the_same_saved_world() {
    let index = build_index([root("source")]).unwrap();

    assert_eq!(
        relation_between(&index, &id("source"), &id("source")).unwrap(),
        SavedWorldRelation::Same
    );
}

#[test]
fn classifies_ancestor_and_descendant_in_either_argument_order() {
    let index = build_index([
        root("source"),
        child("child", "source"),
        child("grandchild", "child"),
    ])
    .unwrap();

    let expected = SavedWorldRelation::AncestorDescendant {
        ancestor: id("source"),
        descendant: id("grandchild"),
    };
    assert_eq!(
        relation_between(&index, &id("source"), &id("grandchild")).unwrap(),
        expected
    );
    assert_eq!(
        relation_between(&index, &id("grandchild"), &id("source")).unwrap(),
        expected
    );
}

#[test]
fn classifies_siblings_by_their_immediate_local_parent() {
    let index = build_index([
        root("source"),
        child("left", "source"),
        child("right", "source"),
    ])
    .unwrap();

    assert_eq!(
        relation_between(&index, &id("left"), &id("right")).unwrap(),
        SavedWorldRelation::Siblings {
            parent: id("source")
        }
    );
}

#[test]
fn classifies_cousins_by_their_nearest_common_ancestor() {
    let index = build_index([
        root("source"),
        child("left-parent", "source"),
        child("right-parent", "source"),
        child("left", "left-parent"),
        child("right", "right-parent"),
    ])
    .unwrap();

    assert_eq!(
        relation_between(&index, &id("left"), &id("right")).unwrap(),
        SavedWorldRelation::Related {
            common_ancestor: id("source")
        }
    );
}

#[test]
fn proves_unrelated_only_when_both_local_ancestries_reach_roots() {
    let index = build_index([
        root("left-root"),
        child("left", "left-root"),
        root("right-root"),
        child("right", "right-root"),
    ])
    .unwrap();

    assert_eq!(
        relation_between(&index, &id("left"), &id("right")).unwrap(),
        SavedWorldRelation::Unrelated
    );
}

#[test]
fn reports_unresolved_ancestry_instead_of_guessing_unrelated() {
    let index = build_index([child("detached", "External.world"), root("local-root")]).unwrap();

    assert_eq!(
        relation_between(&index, &id("detached"), &id("local-root")).unwrap(),
        SavedWorldRelation::UnresolvedAncestry {
            left: Some(id("detached")),
            right: None,
        }
    );
}

#[test]
fn still_proves_local_relationships_below_a_detached_ancestor() {
    let index = build_index([
        child("detached", "External.world"),
        child("left", "detached"),
        child("right", "detached"),
        child("grandchild", "left"),
    ])
    .unwrap();

    assert_eq!(
        relation_between(&index, &id("left"), &id("right")).unwrap(),
        SavedWorldRelation::Siblings {
            parent: id("detached")
        }
    );
    assert_eq!(
        relation_between(&index, &id("left"), &id("grandchild")).unwrap(),
        SavedWorldRelation::AncestorDescendant {
            ancestor: id("left"),
            descendant: id("grandchild"),
        }
    );
}

#[test]
fn rejects_ids_that_are_not_in_the_supplied_index() {
    let index = build_index([root("source")]).unwrap();

    assert!(matches!(
        relation_between(&index, &id("source"), &id("missing")),
        Err(SavedWorldRelationError::UnknownDocument(ref missing)) if missing == &id("missing")
    ));
}
