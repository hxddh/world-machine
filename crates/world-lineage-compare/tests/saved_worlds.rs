use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use world_document::{WorldBranchCause, WorldDocument, WorldLineage, WorldParent};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
use world_library::{WorldDocumentId, WorldLibrary};
use world_lineage::{LineageIndex, LineageRecord};
use world_lineage_compare::{
    compare_saved_worlds, relation_between, SavedFutureCompareError, SavedWorldRelation,
};
use world_persistence::{WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

const PACK_ID: &str = "world-machine.saved-world-test";

#[test]
fn compares_saved_siblings_without_modifying_either_document() {
    let root = temp_root("siblings");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    save_root(&library, "source", 0);
    save_future(&library, "left", "source", 10, 10, "Left");
    save_future(&library, "right", "source", 20, 10, "Right");
    let left = id("left");
    let right = id("right");
    let left_before = library.load_document(&left).unwrap().unwrap();
    let right_before = library.load_document(&right).unwrap().unwrap();

    let result = compare_saved_worlds(&library, &registry, &left, &right).unwrap();

    assert_eq!(
        result.relation,
        SavedWorldRelation::Siblings {
            parent: id("source")
        }
    );
    assert_eq!(result.left.snapshot.world_time, 10);
    assert_eq!(result.right.snapshot.world_time, 20);
    assert_eq!(result.comparison.left.world_time, 10);
    assert_eq!(result.comparison.right.world_time, 20);
    assert_eq!(library.load_document(&left).unwrap().unwrap(), left_before);
    assert_eq!(library.load_document(&right).unwrap().unwrap(), right_before);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn classifies_ancestor_descendant_and_common_ancestor_relations() {
    let root = temp_root("relations");
    let library = WorldLibrary::new(root.clone());
    save_root(&library, "source", 0);
    save_future(&library, "child", "source", 10, 0, "Child");
    save_future(&library, "grandchild", "child", 20, 10, "Grandchild");
    save_future(&library, "sibling", "source", 15, 0, "Sibling");
    save_future(&library, "cousin", "sibling", 25, 15, "Cousin");
    let index = LineageIndex::from_library(&library).unwrap();

    assert_eq!(
        relation_between(&index, &id("source"), &id("grandchild")),
        SavedWorldRelation::AncestorDescendant {
            ancestor: id("source"),
            descendant: id("grandchild"),
        }
    );
    assert_eq!(
        relation_between(&index, &id("grandchild"), &id("cousin")),
        SavedWorldRelation::Related {
            common_ancestor: id("source")
        }
    );
    assert_eq!(
        relation_between(&index, &id("child"), &id("child")),
        SavedWorldRelation::Same
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn unrelated_roots_are_not_given_a_false_common_history() {
    let root = temp_root("unrelated");
    let library = WorldLibrary::new(root.clone());
    save_root(&library, "left-root", 0);
    save_root(&library, "right-root", 0);
    let index = LineageIndex::from_library(&library).unwrap();

    assert_eq!(
        relation_between(&index, &id("left-root"), &id("right-root")),
        SavedWorldRelation::Unrelated
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_cross_pack_or_cross_version_saved_worlds() {
    let root = temp_root("pack-mismatch");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    save_root(&library, "left", 0);
    let right = id("right");
    library
        .save_document(
            &right,
            &WorldDocument::new(archive(WorldPackRef::new(PACK_ID, "2"), 0)),
        )
        .unwrap();

    let error = compare_saved_worlds(&library, &registry, &id("left"), &right).unwrap_err();

    assert!(matches!(error, SavedFutureCompareError::PackMismatch { .. }));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn relation_helper_reports_missing_nodes_as_unavailable() {
    let records = vec![LineageRecord {
        id: id("root"),
        pack: pack(),
        world_time: 0,
        event_count: 0,
        lineage: None,
    }];
    let index = world_lineage::build_index(records).unwrap();

    assert!(matches!(
        relation_between(&index, &id("root"), &id("missing")),
        SavedWorldRelation::Unavailable(message) if message.contains("missing")
    ));
}

fn registry() -> WorldRegistry {
    let mut registry = WorldRegistry::new();
    registry
        .register(
            WorldRegistration::new(
                WorldDescriptor {
                    pack: pack(),
                    title: "Saved World Test".into(),
                    description: "Generic saved World comparison fixture".into(),
                },
                || Ok(Box::new(MockSession::new(0))),
            )
            .with_archive_opener(|archive| Ok(Box::new(MockSession::new(archive.world_time)))),
        )
        .unwrap();
    registry
}

struct MockSession {
    snapshot: ProjectionSnapshot,
}

impl MockSession {
    fn new(world_time: u64) -> Self {
        Self {
            snapshot: ProjectionSnapshot {
                title: format!("Saved World fixture at {world_time}"),
                world_time,
                ..Default::default()
            },
        }
    }
}

impl WorldSession for MockSession {
    fn pack(&self) -> WorldPackRef {
        pack()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        self.snapshot.clone()
    }

    fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
        Ok(self.snapshot())
    }
}

fn save_root(library: &WorldLibrary, document: &str, world_time: u64) {
    library
        .save_document(
            &id(document),
            &WorldDocument::new(archive(pack(), world_time)),
        )
        .unwrap();
}

fn save_future(
    library: &WorldLibrary,
    document: &str,
    parent: &str,
    world_time: u64,
    parent_time: u64,
    choice_title: &str,
) {
    let future = WorldDocument::new(archive(pack(), world_time)).with_lineage(WorldLineage {
        parent: WorldParent {
            document: Some(parent.into()),
            pack: pack(),
            world_time: parent_time,
            event_count: 0,
        },
        branch: WorldBranchCause::Strategy {
            choice_id: format!("fixture.{document}"),
            choice_title: choice_title.into(),
            horizon: world_time.saturating_sub(parent_time),
        },
    });
    library.save_document(&id(document), &future).unwrap();
}

fn archive(pack: WorldPackRef, world_time: u64) -> WorldArchive {
    WorldArchive {
        format: WORLD_ARCHIVE_FORMAT.into(),
        format_version: WORLD_ARCHIVE_VERSION,
        pack,
        world_time,
        events: Vec::new(),
        pending: Vec::new(),
    }
}

fn pack() -> WorldPackRef {
    WorldPackRef::new(PACK_ID, "1")
}

fn id(value: &str) -> WorldDocumentId {
    WorldDocumentId::new(value).unwrap()
}

fn temp_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-saved-world-{label}-{}-{nonce}",
        process::id()
    ))
}
