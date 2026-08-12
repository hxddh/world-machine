use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use world_document::{WorldBranchCause, WorldDocument, WorldLineage};
use world_library::{LibraryError, WorldDocumentId, WorldLibrary};
use world_persistence::WorldPackRef;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageRecord {
    pub id: WorldDocumentId,
    pub pack: WorldPackRef,
    pub world_time: u64,
    pub event_count: usize,
    pub lineage: Option<WorldLineage>,
}

impl LineageRecord {
    pub fn from_document(id: WorldDocumentId, document: &WorldDocument) -> Self {
        Self {
            id,
            pack: document.archive.pack.clone(),
            world_time: document.archive.world_time,
            event_count: document.archive.events.len(),
            lineage: document.metadata.lineage.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageParent {
    pub document: Option<String>,
    pub pack: WorldPackRef,
    pub world_time: u64,
    pub event_count: usize,
    pub resolved: Option<WorldDocumentId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageNode {
    pub id: WorldDocumentId,
    pub pack: WorldPackRef,
    pub world_time: u64,
    pub event_count: usize,
    pub branch: Option<WorldBranchCause>,
    pub parent: Option<LineageParent>,
    pub children: Vec<WorldDocumentId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineageIndex {
    nodes: BTreeMap<WorldDocumentId, LineageNode>,
    roots: Vec<WorldDocumentId>,
    detached: Vec<WorldDocumentId>,
}

impl LineageIndex {
    pub fn from_library(library: &WorldLibrary) -> Result<Self, LineageError> {
        let mut records = Vec::new();
        for summary in library.list()? {
            let document = library
                .load_document(&summary.id)?
                .ok_or_else(|| LibraryError::UnknownDocument(summary.id.clone()))?;
            records.push(LineageRecord::from_document(summary.id, &document));
        }
        build_index(records)
    }

    pub fn nodes(&self) -> &BTreeMap<WorldDocumentId, LineageNode> {
        &self.nodes
    }

    pub fn node(&self, id: &WorldDocumentId) -> Option<&LineageNode> {
        self.nodes.get(id)
    }

    pub fn roots(&self) -> &[WorldDocumentId] {
        &self.roots
    }

    pub fn detached(&self) -> &[WorldDocumentId] {
        &self.detached
    }
}

pub fn build_index(
    records: impl IntoIterator<Item = LineageRecord>,
) -> Result<LineageIndex, LineageError> {
    let mut records_by_id = BTreeMap::new();
    for record in records {
        if records_by_id.insert(record.id.clone(), record).is_some() {
            return Err(LineageError::DuplicateDocumentId(
                records_by_id
                    .last_key_value()
                    .expect("duplicate insertion leaves map non-empty")
                    .0
                    .clone(),
            ));
        }
    }

    let exact_ids = records_by_id
        .keys()
        .map(|id| (id.as_str().to_owned(), id.clone()))
        .collect::<BTreeMap<_, _>>();
    let normalized_ids = records_by_id
        .keys()
        .map(|id| (normalize_document_label(id.as_str()).to_owned(), id.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut nodes = BTreeMap::new();
    let mut roots = Vec::new();
    let mut detached = Vec::new();

    for record in records_by_id.values() {
        let (branch, parent) = match &record.lineage {
            Some(lineage) => {
                let resolved = lineage.parent.document.as_deref().and_then(|label| {
                    exact_ids.get(label).cloned().or_else(|| {
                        normalized_ids
                            .get(normalize_document_label(label))
                            .cloned()
                    })
                });
                let parent = LineageParent {
                    document: lineage.parent.document.clone(),
                    pack: lineage.parent.pack.clone(),
                    world_time: lineage.parent.world_time,
                    event_count: lineage.parent.event_count,
                    resolved,
                };
                (Some(lineage.branch.clone()), Some(parent))
            }
            None => {
                roots.push(record.id.clone());
                (None, None)
            }
        };

        if parent.as_ref().is_some_and(|parent| parent.resolved.is_none()) {
            detached.push(record.id.clone());
        }

        nodes.insert(
            record.id.clone(),
            LineageNode {
                id: record.id.clone(),
                pack: record.pack.clone(),
                world_time: record.world_time,
                event_count: record.event_count,
                branch,
                parent,
                children: Vec::new(),
            },
        );
    }

    let child_edges = nodes
        .values()
        .filter_map(|node| {
            node.parent
                .as_ref()
                .and_then(|parent| parent.resolved.clone())
                .map(|parent| (parent, node.id.clone()))
        })
        .collect::<Vec<_>>();
    for (parent, child) in child_edges {
        if let Some(parent) = nodes.get_mut(&parent) {
            parent.children.push(child);
        }
    }
    for node in nodes.values_mut() {
        node.children.sort();
    }
    roots.sort();
    detached.sort();

    detect_cycle(&nodes)?;

    Ok(LineageIndex {
        nodes,
        roots,
        detached,
    })
}

fn detect_cycle(nodes: &BTreeMap<WorldDocumentId, LineageNode>) -> Result<(), LineageError> {
    for start in nodes.keys() {
        let mut path = Vec::new();
        let mut positions = BTreeMap::new();
        let mut current = Some(start.clone());

        while let Some(id) = current {
            if let Some(position) = positions.get(&id).copied() {
                let mut cycle = path[position..].to_vec();
                cycle.push(id);
                return Err(LineageError::Cycle(cycle));
            }
            positions.insert(id.clone(), path.len());
            path.push(id.clone());
            current = nodes
                .get(&id)
                .and_then(|node| node.parent.as_ref())
                .and_then(|parent| parent.resolved.clone());
        }
    }
    Ok(())
}

fn normalize_document_label(label: &str) -> &str {
    label
        .strip_suffix(".world.json")
        .or_else(|| label.strip_suffix(".world"))
        .unwrap_or(label)
}

#[derive(Debug)]
pub enum LineageError {
    Library(LibraryError),
    DuplicateDocumentId(WorldDocumentId),
    Cycle(Vec<WorldDocumentId>),
}

impl fmt::Display for LineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Library(error) => error.fmt(f),
            Self::DuplicateDocumentId(id) => write!(f, "duplicate World document id: {id}"),
            Self::Cycle(ids) => write!(
                f,
                "World lineage cycle: {}",
                ids.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" -> ")
            ),
        }
    }
}

impl Error for LineageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Library(error) => Some(error),
            Self::DuplicateDocumentId(_) | Self::Cycle(_) => None,
        }
    }
}

impl From<LibraryError> for LineageError {
    fn from(error: LibraryError) -> Self {
        Self::Library(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_document::{WorldBranchCause, WorldLineage, WorldParent};

    fn id(value: &str) -> WorldDocumentId {
        WorldDocumentId::new(value).unwrap()
    }

    fn record(value: &str) -> LineageRecord {
        LineageRecord {
            id: id(value),
            pack: WorldPackRef::new("world-machine.lineage-test", "1"),
            world_time: 10,
            event_count: 3,
            lineage: None,
        }
    }

    fn strategy_child(value: &str, parent: Option<&str>) -> LineageRecord {
        let mut record = record(value);
        record.world_time = 30;
        record.event_count = 8;
        record.lineage = Some(WorldLineage {
            parent: WorldParent {
                document: parent.map(str::to_owned),
                pack: WorldPackRef::new("world-machine.lineage-test", "1"),
                world_time: 10,
                event_count: 3,
            },
            branch: WorldBranchCause::Strategy {
                choice_id: "test.choose".into(),
                choice_title: "Choose".into(),
                horizon: 20,
            },
        });
        record
    }

    #[test]
    fn builds_roots_children_and_branch_metadata() {
        let index = build_index([
            record("source"),
            strategy_child("future-a", Some("source")),
            strategy_child("future-b", Some("source.world")),
        ])
        .unwrap();

        assert_eq!(index.roots(), &[id("source")]);
        assert!(index.detached().is_empty());
        assert_eq!(
            index.node(&id("source")).unwrap().children,
            vec![id("future-a"), id("future-b")]
        );
        let future = index.node(&id("future-a")).unwrap();
        assert_eq!(future.parent.as_ref().unwrap().resolved, Some(id("source")));
        assert!(matches!(
            future.branch,
            Some(WorldBranchCause::Strategy { horizon: 20, .. })
        ));
    }

    #[test]
    fn keeps_missing_or_external_parents_detached_instead_of_promoting_them_to_roots() {
        let index = build_index([
            record("local-root"),
            strategy_child("external-child", Some("External.world")),
            strategy_child("unknown-child", None),
        ])
        .unwrap();

        assert_eq!(index.roots(), &[id("local-root")]);
        assert_eq!(
            index.detached(),
            &[id("external-child"), id("unknown-child")]
        );
    }

    #[test]
    fn exact_document_ids_win_before_suffix_normalization() {
        let index = build_index([
            record("source"),
            record("source.world"),
            strategy_child("future", Some("source.world")),
        ])
        .unwrap();

        assert_eq!(
            index
                .node(&id("future"))
                .unwrap()
                .parent
                .as_ref()
                .unwrap()
                .resolved,
            Some(id("source.world"))
        );
    }

    #[test]
    fn rejects_duplicate_document_ids() {
        assert!(matches!(
            build_index([record("same"), record("same")]),
            Err(LineageError::DuplicateDocumentId(_))
        ));
    }

    #[test]
    fn rejects_resolved_parent_cycles() {
        let index = build_index([
            strategy_child("a", Some("b")),
            strategy_child("b", Some("a")),
        ]);

        assert!(matches!(index, Err(LineageError::Cycle(_))));
    }

    #[test]
    fn child_lists_are_deterministically_sorted() {
        let index = build_index([
            record("root"),
            strategy_child("z-child", Some("root")),
            strategy_child("a-child", Some("root")),
        ])
        .unwrap();

        assert_eq!(
            index.node(&id("root")).unwrap().children,
            vec![id("a-child"), id("z-child")]
        );
    }

    #[test]
    fn library_loader_reads_persisted_lineage_metadata() {
        let root = std::env::temp_dir().join(format!(
            "world-machine-lineage-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let library = WorldLibrary::new(root.clone());
        let source = record("source");
        let future = strategy_child("future", Some("source"));

        let source_document = WorldDocument::new(world_persistence::WorldArchive {
            format: world_persistence::WORLD_ARCHIVE_FORMAT.into(),
            format_version: world_persistence::WORLD_ARCHIVE_VERSION,
            pack: source.pack,
            world_time: source.world_time,
            events: Vec::new(),
            pending: Vec::new(),
        });
        let future_document = WorldDocument::new(world_persistence::WorldArchive {
            format: world_persistence::WORLD_ARCHIVE_FORMAT.into(),
            format_version: world_persistence::WORLD_ARCHIVE_VERSION,
            pack: future.pack,
            world_time: future.world_time,
            events: Vec::new(),
            pending: Vec::new(),
        })
        .with_lineage(future.lineage.unwrap());

        library.save_document(&id("source"), &source_document).unwrap();
        library.save_document(&id("future"), &future_document).unwrap();

        let index = LineageIndex::from_library(&library).unwrap();

        assert_eq!(index.roots(), &[id("source")]);
        assert_eq!(
            index.node(&id("source")).unwrap().children,
            vec![id("future")]
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
