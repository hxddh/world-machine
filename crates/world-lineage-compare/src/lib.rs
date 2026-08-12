use std::error::Error;
use std::fmt;
use world_compare::{compare_snapshots, SnapshotComparison};
use world_document::WorldBranchCause;
use world_host::{HostError, WorldRegistry};
use world_library::{LibraryError, WorldDocumentId, WorldLibrary};
use world_lineage::{LineageError, LineageIndex};
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

fn resolved_parent(
    document: &WorldDocumentId,
    node: &world_lineage::LineageNode,
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
            | Self::DifferentParents { .. } => None,
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
