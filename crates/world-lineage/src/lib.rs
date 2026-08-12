use std::collections::BTreeMap;
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
        let id = record.id.clone();
        if records_by_id.insert(id.clone(), record).is_some() {
            return Err(LineageError::DuplicateDocumentId(id));
        }
    }

    let exact_ids = records_by_id
        .keys()
        .map(|id| (id.as_str().to_owned(), id.clone()))
        .collect::<BTreeMap<_, _>>();
    let normalized_ids = normalized_id_lookup(records_by_id.keys());

    let mut nodes = BTreeMap::new();
    let mut roots = Vec::new();
    let mut detached = Vec::new();

    for record in records_by_id.values() {
        let (branch, parent) = match &record.lineage {
            Some(lineage) => {
                let resolved = lineage.parent.document.as_deref().and_then(|label| {
                    resolve_parent_document(
                        label,
                        &lineage.parent.pack,
                        &records_by_id,
                        &exact_ids,
                        &normalized_ids,
                    )
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

        if parent
            .as_ref()
            .is_some_and(|parent| parent.resolved.is_none())
        {
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

fn resolve_parent_document(
    label: &str,
    parent_pack: &WorldPackRef,
    records_by_id: &BTreeMap<WorldDocumentId, LineageRecord>,
    exact_ids: &BTreeMap<String, WorldDocumentId>,
    normalized_ids: &BTreeMap<String, Option<WorldDocumentId>>,
) -> Option<WorldDocumentId> {
    let candidate = exact_ids.get(label).cloned().or_else(|| {
        normalized_ids
            .get(normalize_document_label(label))
            .and_then(Clone::clone)
    })?;

    records_by_id
        .get(&candidate)
        .filter(|record| record.pack == *parent_pack)
        .map(|_| candidate)
}

fn normalized_id_lookup<'a>(
    ids: impl IntoIterator<Item = &'a WorldDocumentId>,
) -> BTreeMap<String, Option<WorldDocumentId>> {
    let mut lookup = BTreeMap::new();
    for id in ids {
        let label = normalize_document_label(id.as_str()).to_owned();
        match lookup.entry(label) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Some(id.clone()));
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                entry.insert(None);
            }
        }
    }
    lookup
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
