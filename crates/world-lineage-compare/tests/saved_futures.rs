use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use world_document::{WorldBranchCause, WorldDocument, WorldLineage, WorldParent};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
use world_library::{WorldDocumentId, WorldLibrary};
use world_lineage_compare::{compare_saved_siblings, SavedFutureCompareError};
use world_persistence::{WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
use world_projection::{ProjectionIntent, ProjectionSnapshot};

const PACK_ID: &str = "world-machine.saved-future-test";

#[test]
fn compares_current_durable_sibling_worlds_without_modifying_them() {
    let root = temp_root("compare");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    save_root(&library, "source");
    save_future(&library, "left", "source", 30, "Choose Left");
    save_future(&library, "right", "source.world", 45, "Choose Right");
    let left_id = id("left");
    let right_id = id("right");
    let left_before = library.load_document(&left_id).unwrap().unwrap();
    let right_before = library.load_document(&right_id).unwrap().unwrap();

    let result = compare_saved_siblings(&library, &registry, &left_id, &right_id).unwrap();

    assert_eq!(result.parent, id("source"));
    assert_eq!(result.left.document, left_id);
    assert_eq!(result.right.document, right_id);
    assert_eq!(result.left.snapshot.world_time, 30);
    assert_eq!(result.right.snapshot.world_time, 45);
    assert!(!result.comparison.is_identical());
    assert!(matches!(
        result.left.branch,
        WorldBranchCause::Strategy { ref choice_title, .. } if choice_title == "Choose Left"
    ));
    assert_eq!(library.load_document(&left_id).unwrap().unwrap(), left_before);
    assert_eq!(library.load_document(&right_id).unwrap().unwrap(), right_before);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_saved_worlds_from_different_parents() {
    let root = temp_root("different-parents");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    save_root(&library, "source-a");
    save_root(&library, "source-b");
    save_future(&library, "left", "source-a", 30, "Left");
    save_future(&library, "right", "source-b", 30, "Right");

    let error = compare_saved_siblings(&library, &registry, &id("left"), &id("right"))
        .unwrap_err();

    assert!(matches!(
        error,
        SavedFutureCompareError::DifferentParents { .. }
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_detached_lineage_instead_of_guessing_a_parent() {
    let root = temp_root("detached");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    save_future(&library, "left", "external-source", 30, "Left");
    save_future(&library, "right", "external-source", 35, "Right");

    let error = compare_saved_siblings(&library, &registry, &id("left"), &id("right"))
        .unwrap_err();

    assert!(matches!(error, SavedFutureCompareError::DetachedParent(_)));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_comparing_a_saved_world_with_itself() {
    let root = temp_root("same");
    let library = WorldLibrary::new(root.clone());
    let registry = registry();
    save_root(&library, "source");
    save_future(&library, "future", "source", 30, "Future");
    let future = id("future");

    let error = compare_saved_siblings(&library, &registry, &future, &future).unwrap_err();

    assert!(matches!(error, SavedFutureCompareError::SameDocument(_)));
    let _ = fs::remove_dir_all(root);
}

fn registry() -> WorldRegistry {
    let mut registry = WorldRegistry::new();
    registry
        .register(
            WorldRegistration::new(
                WorldDescriptor {
                    pack: pack(),
                    title: "Saved Future Test".into(),
                    description: "Generic saved Future comparison fixture".into(),
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
                title: format!("Fixture at {world_time}"),
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

fn save_root(library: &WorldLibrary, document: &str) {
    library
        .save_document(&id(document), &WorldDocument::new(archive(10)))
        .unwrap();
}

fn save_future(
    library: &WorldLibrary,
    document: &str,
    parent: &str,
    world_time: u64,
    choice_title: &str,
) {
    let future = WorldDocument::new(archive(world_time)).with_lineage(WorldLineage {
        parent: WorldParent {
            document: Some(parent.into()),
            pack: pack(),
            world_time: 10,
            event_count: 0,
        },
        branch: WorldBranchCause::Strategy {
            choice_id: format!("fixture.{document}"),
            choice_title: choice_title.into(),
            horizon: world_time.saturating_sub(10),
        },
    });
    library.save_document(&id(document), &future).unwrap();
}

fn archive(world_time: u64) -> WorldArchive {
    WorldArchive {
        format: WORLD_ARCHIVE_FORMAT.into(),
        format_version: WORLD_ARCHIVE_VERSION,
        pack: pack(),
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
        "world-machine-saved-future-{label}-{}-{nonce}",
        process::id()
    ))
}
