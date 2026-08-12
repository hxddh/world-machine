use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use world_compare::{compare_snapshots, SnapshotComparison};
use world_document::WorldBranchCause;
use world_host::{HostError, WorldRegistry};
use world_library::{LibraryError, WorldDocumentId, WorldLibrary};
use world_lineage::{LineageError, LineageIndex, LineageNode};
use world_persistence::WorldPackRef;
use world_projection::ProjectionSnapshot;

#[derive(Clone, Debug, PartialEq)]
pub struct SavedFutureSide {
    pub document: WorldDocumentId,
    pub branch: WorldBranchCause,
    pub snapshot: ProjectionSnapshot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SavedFutureComparison {
    pub parent: WorldDocumentId,
    pub left: SavedFutureSide,
    pub right: SavedFutureSide,
    pub comparison: SnapshotComparison,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SavedWorldSide {
    pub document: WorldDocumentId,
    pub branch: Option<WorldBranchCause>,
    pub snapshot: ProjectionSnapshot,
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

pub fn compare_saved_siblings(
    library: &WorldLibrary,
    registry: &WorldRegistry,
    left: &WorldDocumentId,
    right: &WorldDocumentId,
) -> Result<SavedFutureComparison, SavedFutureCompareError> {
    if left == right {
        return Err(SavedFutureCompareError::SameDocument(left.clone()));
    }

    let lineage = LineageIndex::from_library(library)?;
    let left_node = lineage
        .node(left)
        .ok_or_else(|| SavedFutureCompareError::UnknownDocument(left.clone()))?;
    let right_node = lineage
        .node(right)
        .ok_or_else(|| SavedFutureCompareError::UnknownDocument(right.clone()))?;

    let left_parent = resolved_parent(left, left_node)?;
    let right_parent = resolved_parent(right, right_node)?;
    if left_parent != right_parent {
        return Err(SavedFutureCompareError::DifferentParents {
            left: left_parent,
            right: right_parent,
        });
    }

    let left_branch = left_node
        .branch
        .clone()
        .ok_or_else(|| SavedFutureCompareError::MissingLineage(left.clone()))?;
    let right_branch = right_node
        .branch
        .clone()
        .ok_or_else(|| SavedFutureCompareError::MissingLineage(right.clone()))?;

    let left_document = library
        .load_document(left)?
        .ok_or_else(|| SavedFutureCompareError::UnknownDocument(left.clone()))?;
    let right_document = library
        .load_document(right)?
        .ok_or_else(|| SavedFutureCompareError::UnknownDocument(right.clone()))?;

    let left_snapshot = registry.open_archive(&left_document.archive)?.snapshot();
    let right_snapshot = registry.open_archive(&right_document.archive)?.snapshot();
    let comparison = compare_snapshots(&left_snapshot, &right_snapshot);

    Ok(SavedFutureComparison {
        parent: left_parent,
        left: SavedFutureSide {
            document: left.clone(),
            branch: left_branch,
            snapshot: left_snapshot,
        },
        right: SavedFutureSide {
            document: right.clone(),
            branch: right_branch,
            snapshot: right_snapshot,
        },
        comparison,
    })
}

pub fn compare_saved_worlds(
    library: &WorldLibrary,
    registry: &WorldRegistry,
    left: &WorldDocumentId,
    right: &WorldDocumentId,
) -> Result<SavedWorldComparison, SavedFutureCompareError> {
    let left_document = library
        .load_document(left)?
        .ok_or_else(|| SavedFutureCompareError::UnknownDocument(left.clone()))?;
    let right_document = library
        .load_document(right)?
        .ok_or_else(|| SavedFutureCompareError::UnknownDocument(right.clone()))?;

    if left_document.archive.pack != right_document.archive.pack {
        return Err(SavedFutureCompareError::PackMismatch {
            left: left_document.archive.pack,
            right: right_document.archive.pack,
        });
    }

    let lineage = LineageIndex::from_library(library)?;
    let left_node = lineage
        .node(left)
        .ok_or_else(|| SavedFutureCompareError::UnknownDocument(left.clone()))?;
    let right_node = lineage
        .node(right)
        .ok_or_else(|| SavedFutureCompareError::UnknownDocument(right.clone()))?;
    let relation = relation_between_nodes(&lineage, left_node, right_node);

    let left_snapshot = registry.open_archive(&left_document.archive)?.snapshot();
    let right_snapshot = registry.open_archive(&right_document.archive)?.snapshot();
    let comparison = compare_snapshots(&left_snapshot, &right_snapshot);

    Ok(SavedWorldComparison {
        left: SavedWorldSide {
            document: left.clone(),
            branch: left_node.branch.clone(),
            snapshot: left_snapshot,
        },
        right: SavedWorldSide {
            document: right.clone(),
            branch: right_node.branch.clone(),
            snapshot: right_snapshot,
        },
        relation,
        comparison,
    })
}

pub fn relation_between(
    index: &LineageIndex,
    left: &WorldDocumentId,
    right: &WorldDocumentId,
) -> SavedWorldRelation {
    let Some(left_node) = index.node(left) else {
        return SavedWorldRelation::Unavailable(format!("missing lineage node: {left}"));
    };
    let Some(right_node) = index.node(right) else {
        return SavedWorldRelation::Unavailable(format!("missing lineage node: {right}"));
    };
    relation_between_nodes(index, left_node, right_node)
}

fn relation_between_nodes(
    index: &LineageIndex,
    left: &LineageNode,
    right: &LineageNode,
) -> SavedWorldRelation {
    if left.id == right.id {
        return SavedWorldRelation::Same;
    }

    let left_ancestors = ancestor_distances(index, &left.id);
    let right_ancestors = ancestor_distances(index, &right.id);

    if right_ancestors.contains_key(&left.id) {
        return SavedWorldRelation::AncestorDescendant {
            ancestor: left.id.clone(),
            descendant: right.id.clone(),
        };
    }
    if left_ancestors.contains_key(&right.id) {
        return SavedWorldRelation::AncestorDescendant {
            ancestor: right.id.clone(),
            descendant: left.id.clone(),
        };
    }

    if let (Some(left_parent), Some(right_parent)) =
        (resolved_parent_id(left), resolved_parent_id(right))
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
        current = index.node(&id).and_then(resolved_parent_id).cloned();
        distance += 1;
    }
    distances
}

fn resolved_parent_id(node: &LineageNode) -> Option<&WorldDocumentId> {
    node.parent
        .as_ref()
        .and_then(|parent| parent.resolved.as_ref())
}

fn resolved_parent(
    document: &WorldDocumentId,
    node: &LineageNode,
) -> Result<WorldDocumentId, SavedFutureCompareError> {
    let parent = node
        .parent
        .as_ref()
        .ok_or_else(|| SavedFutureCompareError::MissingLineage(document.clone()))?;
    parent
        .resolved
        .clone()
        .ok_or_else(|| SavedFutureCompareError::DetachedParent(document.clone()))
}

#[derive(Debug)]
pub enum SavedFutureCompareError {
    Library(LibraryError),
    Lineage(LineageError),
    Host(HostError),
    UnknownDocument(WorldDocumentId),
    SameDocument(WorldDocumentId),
    MissingLineage(WorldDocumentId),
    DetachedParent(WorldDocumentId),
    DifferentParents {
        left: WorldDocumentId,
        right: WorldDocumentId,
    },
    PackMismatch {
        left: WorldPackRef,
        right: WorldPackRef,
    },
}

impl fmt::Display for SavedFutureCompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Library(error) => error.fmt(f),
            Self::Lineage(error) => error.fmt(f),
            Self::Host(error) => error.fmt(f),
            Self::UnknownDocument(id) => write!(f, "unknown saved World: {id}"),
            Self::SameDocument(id) => write!(f, "cannot compare saved World {id} with itself"),
            Self::MissingLineage(id) => write!(f, "saved World {id} has no branch lineage"),
            Self::DetachedParent(id) => write!(
                f,
                "saved World {id} does not resolve to a parent in the current Library"
            ),
            Self::DifferentParents { left, right } => {
                write!(f, "saved Worlds are not siblings: {left} vs {right}")
            }
            Self::PackMismatch { left, right } => write!(
                f,
                "saved World comparison requires the same Pack version: left={}@{}, right={}@{}",
                left.id, left.version, right.id, right.version
            ),
        }
    }
}

impl Error for SavedFutureCompareError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Library(error) => Some(error),
            Self::Lineage(error) => Some(error),
            Self::Host(error) => Some(error),
            Self::UnknownDocument(_)
            | Self::SameDocument(_)
            | Self::MissingLineage(_)
            | Self::DetachedParent(_)
            | Self::DifferentParents { .. }
            | Self::PackMismatch { .. } => None,
        }
    }
}

impl From<LibraryError> for SavedFutureCompareError {
    fn from(error: LibraryError) -> Self {
        Self::Library(error)
    }
}

impl From<LineageError> for SavedFutureCompareError {
    fn from(error: LineageError) -> Self {
        Self::Lineage(error)
    }
}

impl From<HostError> for SavedFutureCompareError {
    fn from(error: HostError) -> Self {
        Self::Host(error)
    }
}
