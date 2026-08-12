use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use world_compare::{compare_snapshots, SnapshotComparison};
use world_host::WorldRegistry;
use world_library::{DurableWorldSession, LibraryError, WorldDocumentId, WorldLibrary};
use world_lineage::{LineageIndex, LineageNode};
use world_persistence::WorldPackRef;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedWorldSide {
    pub document: WorldDocumentId,
    pub pack: WorldPackRef,
    pub title: String,
    pub world_time: u64,
    pub event_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SavedWorldRelation {
    Same,
    AncestorDescendant {
        ancestor: WorldDocumentId,
        descendant: WorldDocumentId,
    },
    Siblings {
        parent: WorldDocumentId,
    },
    Related {
        common_ancestor: WorldDocumentId,
    },
    Unrelated,
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SavedWorldComparison {
    pub left: SavedWorldSide,
    pub right: SavedWorldSide,
    pub relation: SavedWorldRelation,
    pub comparison: SnapshotComparison,
}

pub fn compare_saved_worlds(
    registry: &WorldRegistry,
    library: &WorldLibrary,
    left_id: &WorldDocumentId,
    right_id: &WorldDocumentId,
) -> Result<SavedWorldComparison, SavedWorldCompareError> {
    let left_document = library
        .load_document(left_id)?
        .ok_or_else(|| LibraryError::UnknownDocument(left_id.clone()))?;
    let right_document = library
        .load_document(right_id)?
        .ok_or_else(|| LibraryError::UnknownDocument(right_id.clone()))?;

    if left_document.archive.pack != right_document.archive.pack {
        return Err(SavedWorldCompareError::PackMismatch {
            left: left_document.archive.pack,
            right: right_document.archive.pack,
        });
    }

    let left_session = DurableWorldSession::open(left_id.clone(), registry, library)?;
    let right_session = DurableWorldSession::open(right_id.clone(), registry, library)?;
    let left_snapshot = left_session.snapshot();
    let right_snapshot = right_session.snapshot();
    let relation = match LineageIndex::from_library(library) {
        Ok(index) => relation_between(&index, left_id, right_id),
        Err(error) => SavedWorldRelation::Unavailable(error.to_string()),
    };

    Ok(SavedWorldComparison {
        left: SavedWorldSide {
            document: left_id.clone(),
            pack: left_document.archive.pack.clone(),
            title: left_snapshot.title.clone(),
            world_time: left_snapshot.world_time,
            event_count: left_document.archive.events.len(),
        },
        right: SavedWorldSide {
            document: right_id.clone(),
            pack: right_document.archive.pack.clone(),
            title: right_snapshot.title.clone(),
            world_time: right_snapshot.world_time,
            event_count: right_document.archive.events.len(),
        },
        relation,
        comparison: compare_snapshots(&left_snapshot, &right_snapshot),
    })
}

pub fn relation_between(
    index: &LineageIndex,
    left: &WorldDocumentId,
    right: &WorldDocumentId,
) -> SavedWorldRelation {
    if left == right {
        return SavedWorldRelation::Same;
    }
    let Some(left_node) = index.node(left) else {
        return SavedWorldRelation::Unavailable(format!("missing lineage node: {left}"));
    };
    let Some(right_node) = index.node(right) else {
        return SavedWorldRelation::Unavailable(format!("missing lineage node: {right}"));
    };

    let left_ancestors = ancestor_distances(index, left);
    let right_ancestors = ancestor_distances(index, right);

    if right_ancestors.contains_key(left) {
        return SavedWorldRelation::AncestorDescendant {
            ancestor: left.clone(),
            descendant: right.clone(),
        };
    }
    if left_ancestors.contains_key(right) {
        return SavedWorldRelation::AncestorDescendant {
            ancestor: right.clone(),
            descendant: left.clone(),
        };
    }

    if let (Some(left_parent), Some(right_parent)) =
        (resolved_parent(left_node), resolved_parent(right_node))
    {
        if left_parent == right_parent {
            return SavedWorldRelation::Siblings {
                parent: left_parent.clone(),
            };
        }
    }

    let common_ancestor = right_ancestors
        .iter()
        .filter_map(|(candidate, right_distance)| {
            left_ancestors
                .get(candidate)
                .map(|left_distance| (candidate, left_distance + right_distance))
        })
        .min_by(|(left_id, left_distance), (right_id, right_distance)| {
            left_distance
                .cmp(right_distance)
                .then_with(|| left_id.cmp(right_id))
        })
        .map(|(candidate, _)| candidate.clone());

    common_ancestor
        .map(|common_ancestor| SavedWorldRelation::Related { common_ancestor })
        .unwrap_or(SavedWorldRelation::Unrelated)
}

fn ancestor_distances(
    index: &LineageIndex,
    start: &WorldDocumentId,
) -> BTreeMap<WorldDocumentId, usize> {
    let mut distances = BTreeMap::new();
    let mut current = Some(start.clone());
    let mut distance = 0usize;
    while let Some(id) = current {
        if distances.insert(id.clone(), distance).is_some() {
            break;
        }
        current = index.node(&id).and_then(resolved_parent).cloned();
        distance += 1;
    }
    distances
}

fn resolved_parent(node: &LineageNode) -> Option<&WorldDocumentId> {
    node.parent
        .as_ref()
        .and_then(|parent| parent.resolved.as_ref())
}

#[derive(Debug)]
pub enum SavedWorldCompareError {
    Library(LibraryError),
    PackMismatch {
        left: WorldPackRef,
        right: WorldPackRef,
    },
}

impl fmt::Display for SavedWorldCompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Library(error) => error.fmt(f),
            Self::PackMismatch { left, right } => write!(
                f,
                "saved World comparison requires the same Pack version: left={}@{}, right={}@{}",
                left.id, left.version, right.id, right.version
            ),
        }
    }
}

impl Error for SavedWorldCompareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Library(error) => Some(error),
            Self::PackMismatch { .. } => None,
        }
    }
}

impl From<LibraryError> for SavedWorldCompareError {
    fn from(error: LibraryError) -> Self {
        Self::Library(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_document::{WorldBranchCause, WorldDocument, WorldLineage, WorldParent};
    use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
    use world_persistence::{WorldArchive, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
    use world_projection::{ProjectionCapabilities, ProjectionIntent, ProjectionSnapshot};

    const PACK: &str = "world-machine.saved-compare-test";

    struct MockSession {
        pack: WorldPackRef,
        time: u64,
    }

    impl WorldSession for MockSession {
        fn pack(&self) -> WorldPackRef {
            self.pack.clone()
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: "Saved comparison mock".into(),
                world_time: self.time,
                capabilities: ProjectionCapabilities { fork: false },
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            Err(HostError::session("saved comparison mock is read-only"))
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(archive(self.pack.clone(), self.time)))
        }
    }

    fn archive(pack: WorldPackRef, time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack,
            world_time: time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn registry() -> WorldRegistry {
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: WorldPackRef::new(PACK, "1"),
                        title: "Saved comparison mock".into(),
                        description: "Saved World comparison regression".into(),
                    },
                    || {
                        Ok(Box::new(MockSession {
                            pack: WorldPackRef::new(PACK, "1"),
                            time: 0,
                        }))
                    },
                )
                .with_archive_opener(|archive| {
                    Ok(Box::new(MockSession {
                        pack: archive.pack.clone(),
                        time: archive.world_time,
                    }))
                }),
            )
            .unwrap();
        registry
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-saved-compare-{}-{nonce}-{label}",
            process::id()
        ))
    }

    fn lineage(parent: &str, parent_time: u64, title: &str) -> WorldLineage {
        WorldLineage {
            parent: WorldParent {
                document: Some(parent.into()),
                pack: WorldPackRef::new(PACK, "1"),
                world_time: parent_time,
                event_count: 0,
            },
            branch: WorldBranchCause::Strategy {
                choice_id: format!("test.{title}"),
                choice_title: title.into(),
                horizon: 20,
            },
        }
    }

    fn create_document(
        library: &WorldLibrary,
        id: &str,
        time: u64,
        lineage: Option<WorldLineage>,
    ) -> WorldDocumentId {
        let id = WorldDocumentId::new(id).unwrap();
        let mut document = WorldDocument::new(archive(WorldPackRef::new(PACK, "1"), time));
        document.metadata.lineage = lineage;
        library.create_from_document(id.clone(), &document).unwrap();
        id
    }

    #[test]
    fn compares_saved_sibling_worlds_without_mutating_either_document() {
        let root = temp_root("siblings");
        let library = WorldLibrary::new(root.join("library"));
        let registry = registry();
        let source = create_document(&library, "source", 0, None);
        let left = create_document(&library, "left", 10, Some(lineage("source", 0, "Left")));
        let right = create_document(&library, "right", 20, Some(lineage("source", 0, "Right")));
        let left_before = library.load_document(&left).unwrap().unwrap();
        let right_before = library.load_document(&right).unwrap().unwrap();

        let result = compare_saved_worlds(&registry, &library, &left, &right).unwrap();

        assert_eq!(result.left.world_time, 10);
        assert_eq!(result.right.world_time, 20);
        assert_eq!(
            result.relation,
            SavedWorldRelation::Siblings { parent: source }
        );
        assert_eq!(result.comparison.left.world_time, 10);
        assert_eq!(result.comparison.right.world_time, 20);
        assert_eq!(library.load_document(&left).unwrap().unwrap(), left_before);
        assert_eq!(
            library.load_document(&right).unwrap().unwrap(),
            right_before
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn classifies_ancestor_and_related_saved_worlds() {
        let root = temp_root("relations");
        let library = WorldLibrary::new(root.join("library"));
        let source = create_document(&library, "source", 0, None);
        let child = create_document(&library, "child", 10, Some(lineage("source", 0, "Child")));
        let grandchild = create_document(
            &library,
            "grandchild",
            20,
            Some(lineage("child", 10, "Grandchild")),
        );
        let sibling = create_document(
            &library,
            "sibling",
            15,
            Some(lineage("source", 0, "Sibling")),
        );
        let cousin = create_document(
            &library,
            "cousin",
            25,
            Some(lineage("sibling", 15, "Cousin")),
        );
        let index = LineageIndex::from_library(&library).unwrap();

        assert_eq!(
            relation_between(&index, &source, &grandchild),
            SavedWorldRelation::AncestorDescendant {
                ancestor: source.clone(),
                descendant: grandchild.clone(),
            }
        );
        assert_eq!(
            relation_between(&index, &grandchild, &cousin),
            SavedWorldRelation::Related {
                common_ancestor: source,
            }
        );
        assert_eq!(
            relation_between(&index, &child, &child),
            SavedWorldRelation::Same
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unrelated_roots_are_not_given_a_false_common_history() {
        let root = temp_root("unrelated");
        let library = WorldLibrary::new(root.join("library"));
        let left = create_document(&library, "left-root", 0, None);
        let right = create_document(&library, "right-root", 0, None);
        let index = LineageIndex::from_library(&library).unwrap();

        assert_eq!(
            relation_between(&index, &left, &right),
            SavedWorldRelation::Unrelated
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_cross_pack_or_cross_version_comparisons() {
        let root = temp_root("pack-mismatch");
        let library = WorldLibrary::new(root.join("library"));
        let registry = registry();
        let left = create_document(&library, "left", 0, None);
        let right = WorldDocumentId::new("right").unwrap();
        library
            .create_from_document(
                right.clone(),
                &WorldDocument::new(archive(WorldPackRef::new(PACK, "2"), 0)),
            )
            .unwrap();

        assert!(matches!(
            compare_saved_worlds(&registry, &library, &left, &right),
            Err(SavedWorldCompareError::PackMismatch { .. })
        ));
        let _ = fs::remove_dir_all(root);
    }
}
