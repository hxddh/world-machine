use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use world_compare::{compare_evidence_neighborhoods, DifferenceKind};
use world_projection::{
    InspectorProjection, ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge,
    TimelineItem,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "kebab-case")]
pub enum EvidenceQueryRequest {
    Selections,
    Describe {
        selection: String,
    },
    Why {
        event: String,
    },
    Influence {
        event: String,
    },
    CausalPath {
        from: String,
        to: String,
    },
    CausalNeighborhood {
        root: String,
        upstream_depth: usize,
        downstream_depth: usize,
    },
    Neighborhood {
        root: String,
        max_depth: usize,
    },
    ShortestPath {
        from: String,
        to: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum EvidenceQueryResponse {
    Selections {
        value: EvidenceSelectionIndex,
    },
    Description {
        value: EvidenceSelectionDetail,
    },
    Why {
        value: EvidenceWhyResult,
    },
    Influence {
        value: EvidenceInfluenceResult,
    },
    CausalPath {
        value: EvidenceCausalPathResult,
    },
    CausalNeighborhood {
        value: EvidenceCausalNeighborhoodResult,
    },
    Neighborhood {
        value: EvidenceNeighborhoodResult,
    },
    ShortestPath {
        value: EvidencePathResult,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSelectionIndex {
    pub selections: Vec<EvidenceSelection>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSelection {
    pub selection: String,
    pub kind: EvidenceSelectionKind,
    pub title: String,
    pub subtitle: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceSelectionKind {
    Entity,
    Relation,
    Event,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSelectionDetail {
    pub selection: String,
    pub kind: EvidenceSelectionKind,
    pub title: String,
    pub subtitle: String,
    pub sections: Vec<EvidenceDetailSection>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceDetailSection {
    pub title: String,
    pub rows: Vec<EvidenceDetailRow>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceDetailRow {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceWhyResult {
    pub event: String,
    pub nodes: Vec<EvidenceCausalNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceInfluenceResult {
    pub event: String,
    pub nodes: Vec<EvidenceCausalNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalPathResult {
    pub from: String,
    pub to: String,
    pub nodes: Vec<EvidenceCausalNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNeighborhoodResult {
    pub root: EvidenceCausalNode,
    pub upstream_depth: usize,
    pub downstream_depth: usize,
    pub upstream: Vec<EvidenceCausalNode>,
    pub downstream: Vec<EvidenceCausalNode>,
    #[serde(default)]
    pub upstream_truncated: bool,
    #[serde(default)]
    pub downstream_truncated: bool,
    #[serde(default)]
    pub upstream_frontier: Vec<String>,
    #[serde(default)]
    pub downstream_frontier: Vec<String>,
    #[serde(default)]
    pub upstream_continuations: Vec<EvidenceCausalContinuation>,
    #[serde(default)]
    pub downstream_continuations: Vec<EvidenceCausalContinuation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceCausalDirection {
    Upstream,
    Downstream,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalContinuation {
    pub event: String,
    pub direction: EvidenceCausalDirection,
    pub request: EvidenceQueryRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNode {
    pub event: String,
    pub depth: usize,
    pub world_time: u64,
    pub title: String,
    pub subtitle: String,
    pub caused_by: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceComparisonRequest {
    pub root: String,
    pub max_depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceNeighborhoodResult {
    pub root: String,
    pub max_depth: usize,
    pub nodes: Vec<EvidenceNode>,
    pub edges: Vec<EvidenceEdge>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceNode {
    pub selection: String,
    pub depth: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EvidenceEdge {
    EntityEvent {
        entity: String,
        event: String,
    },
    RelationEvent {
        relation: String,
        event: String,
    },
    EntityRelation {
        entity: String,
        relation: String,
        role: RelationRole,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationRole {
    From,
    To,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidencePathResult {
    pub from: String,
    pub to: String,
    pub steps: Vec<EvidencePathStep>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidencePathStep {
    pub from: String,
    pub edge: EvidenceEdge,
    pub to: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceComparisonResult {
    pub root: String,
    pub max_depth: usize,
    pub identical: bool,
    pub nodes: Vec<EvidenceNodeDifference>,
    pub left_only_edges: Vec<EvidenceEdge>,
    pub right_only_edges: Vec<EvidenceEdge>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceNodeDifference {
    pub selection: String,
    pub kind: Difference,
    pub left_depth: Option<usize>,
    pub right_depth: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Difference {
    LeftOnly,
    RightOnly,
    Changed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "error", content = "details", rename_all = "kebab-case")]
pub enum QueryError {
    InvalidSelectionKey(String),
    SelectionKindMismatch {
        selection: String,
        expected: EvidenceSelectionKind,
    },
    SelectionNotVisible(String),
    NoEvidencePath {
        from: String,
        to: String,
    },
    NoCausalPath {
        from: String,
        to: String,
    },
    SelectionNotVisibleInEitherWorld(String),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelectionKey(selection) => {
                write!(f, "invalid selection key: {selection}")
            }
            Self::SelectionKindMismatch {
                selection,
                expected,
            } => write!(
                f,
                "selection kind mismatch: {selection} (expected {})",
                selection_kind_name(*expected)
            ),
            Self::SelectionNotVisible(selection) => {
                write!(f, "selection is not visible: {selection}")
            }
            Self::NoEvidencePath { from, to } => write!(f, "no evidence path: {from} -> {to}"),
            Self::NoCausalPath { from, to } => write!(f, "no causal path: {from} -> {to}"),
            Self::SelectionNotVisibleInEitherWorld(selection) => {
                write!(f, "selection is not visible in either world: {selection}")
            }
        }
    }
}

impl Error for QueryError {}

pub fn execute_query(
    snapshot: &ProjectionSnapshot,
    request: &EvidenceQueryRequest,
) -> Result<EvidenceQueryResponse, QueryError> {
    match request {
        EvidenceQueryRequest::Selections => Ok(EvidenceQueryResponse::Selections {
            value: query_selections(snapshot),
        }),
        EvidenceQueryRequest::Describe { selection } => {
            let selection = parse_selection_key(selection)?;
            query_description(snapshot, selection)
                .map(|value| EvidenceQueryResponse::Description { value })
        }
        EvidenceQueryRequest::Why { event } => {
            let event = parse_selection_key(event)?;
            query_why(snapshot, event).map(|value| EvidenceQueryResponse::Why { value })
        }
        EvidenceQueryRequest::Influence { event } => {
            let event = parse_selection_key(event)?;
            query_influence(snapshot, event).map(|value| EvidenceQueryResponse::Influence { value })
        }
        EvidenceQueryRequest::CausalPath { from, to } => {
            let from = parse_selection_key(from)?;
            let to = parse_selection_key(to)?;
            query_causal_path(snapshot, from, to)
                .map(|value| EvidenceQueryResponse::CausalPath { value })
        }
        EvidenceQueryRequest::CausalNeighborhood {
            root,
            upstream_depth,
            downstream_depth,
        } => {
            let root = parse_selection_key(root)?;
            query_causal_neighborhood(snapshot, root, *upstream_depth, *downstream_depth)
                .map(|value| EvidenceQueryResponse::CausalNeighborhood { value })
        }
        EvidenceQueryRequest::Neighborhood { root, max_depth } => {
            let root = parse_selection_key(root)?;
            query_neighborhood(snapshot, root, *max_depth)
                .map(|value| EvidenceQueryResponse::Neighborhood { value })
        }
        EvidenceQueryRequest::ShortestPath { from, to } => {
            let from = parse_selection_key(from)?;
            let to = parse_selection_key(to)?;
            query_shortest_path(snapshot, from, to)
                .map(|value| EvidenceQueryResponse::ShortestPath { value })
        }
    }
}

pub fn execute_comparison_query(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request: &EvidenceComparisonRequest,
) -> Result<EvidenceComparisonResult, QueryError> {
    let root = parse_selection_key(&request.root)?;
    query_neighborhood_comparison(left, right, root, request.max_depth)
}

fn parse_selection_key(key: &str) -> Result<SelectionId, QueryError> {
    SelectionId::from_stable_key(key).ok_or_else(|| QueryError::InvalidSelectionKey(key.to_owned()))
}

pub fn query_selections(snapshot: &ProjectionSnapshot) -> EvidenceSelectionIndex {
    let mut selections = std::collections::BTreeMap::new();

    for (selection, inspector) in &snapshot.inspectors {
        let kind = match selection {
            SelectionId::Entity(_) => EvidenceSelectionKind::Entity,
            SelectionId::Relation(_) => EvidenceSelectionKind::Relation,
            SelectionId::Event(_) => continue,
        };
        selections.insert(
            *selection,
            EvidenceSelection {
                selection: selection.stable_key(),
                kind,
                title: inspector.title.clone(),
                subtitle: inspector.subtitle.clone(),
            },
        );
    }

    for item in &snapshot.timeline.items {
        if !matches!(item.id, SelectionId::Event(_)) {
            continue;
        }
        selections.insert(
            item.id,
            EvidenceSelection {
                selection: item.id.stable_key(),
                kind: EvidenceSelectionKind::Event,
                title: item.title.clone(),
                subtitle: item.subtitle.clone(),
            },
        );
    }

    EvidenceSelectionIndex {
        selections: selections.into_values().collect(),
    }
}

pub fn query_description(
    snapshot: &ProjectionSnapshot,
    selection: SelectionId,
) -> Result<EvidenceSelectionDetail, QueryError> {
    match selection {
        SelectionId::Entity(_) | SelectionId::Relation(_) => {
            let inspector = snapshot
                .inspector(selection)
                .ok_or_else(|| QueryError::SelectionNotVisible(selection.stable_key()))?;
            Ok(EvidenceSelectionDetail {
                selection: selection.stable_key(),
                kind: selection_kind(selection),
                title: inspector.title.clone(),
                subtitle: inspector.subtitle.clone(),
                sections: visible_detail_sections(inspector),
            })
        }
        SelectionId::Event(_) => {
            let item = snapshot
                .timeline
                .items
                .iter()
                .find(|item| item.id == selection)
                .ok_or_else(|| QueryError::SelectionNotVisible(selection.stable_key()))?;
            Ok(EvidenceSelectionDetail {
                selection: selection.stable_key(),
                kind: EvidenceSelectionKind::Event,
                title: item.title.clone(),
                subtitle: item.subtitle.clone(),
                sections: snapshot
                    .inspector(selection)
                    .map(visible_detail_sections)
                    .unwrap_or_default(),
            })
        }
    }
}

fn selection_kind(selection: SelectionId) -> EvidenceSelectionKind {
    match selection {
        SelectionId::Entity(_) => EvidenceSelectionKind::Entity,
        SelectionId::Relation(_) => EvidenceSelectionKind::Relation,
        SelectionId::Event(_) => EvidenceSelectionKind::Event,
    }
}

fn selection_kind_name(kind: EvidenceSelectionKind) -> &'static str {
    match kind {
        EvidenceSelectionKind::Entity => "entity",
        EvidenceSelectionKind::Relation => "relation",
        EvidenceSelectionKind::Event => "event",
    }
}

fn visible_detail_sections(inspector: &InspectorProjection) -> Vec<EvidenceDetailSection> {
    inspector
        .display_sections()
        .map(|section| EvidenceDetailSection {
            title: section.title.clone(),
            rows: section
                .rows
                .iter()
                .map(|row| EvidenceDetailRow {
                    label: row.label.clone(),
                    value: row.value.clone(),
                })
                .collect(),
        })
        .collect()
}

struct VisibleCausalGraph<'a> {
    events: std::collections::BTreeMap<SelectionId, &'a TimelineItem>,
    children: std::collections::BTreeMap<SelectionId, Vec<SelectionId>>,
}

impl<'a> VisibleCausalGraph<'a> {
    fn new(snapshot: &'a ProjectionSnapshot) -> Self {
        let events = snapshot
            .timeline
            .items
            .iter()
            .filter(|item| matches!(item.id, SelectionId::Event(_)))
            .map(|item| (item.id, item))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut children = std::collections::BTreeMap::<SelectionId, Vec<SelectionId>>::new();

        for item in events.values().copied() {
            for cause in &item.caused_by {
                let cause = SelectionId::Event(*cause);
                if events.contains_key(&cause) {
                    children.entry(cause).or_default().push(item.id);
                }
            }
        }
        for direct_children in children.values_mut() {
            direct_children.sort_by_key(|child| {
                let item = events
                    .get(child)
                    .copied()
                    .expect("causal child must remain visible");
                (item.world_time, *child)
            });
            direct_children.dedup();
        }

        Self { events, children }
    }

    fn require_event(&self, event: SelectionId) -> Result<(), QueryError> {
        if !matches!(event, SelectionId::Event(_)) {
            return Err(QueryError::SelectionKindMismatch {
                selection: event.stable_key(),
                expected: EvidenceSelectionKind::Event,
            });
        }
        if !self.events.contains_key(&event) {
            return Err(QueryError::SelectionNotVisible(event.stable_key()));
        }
        Ok(())
    }

    fn parents(&self, event: SelectionId) -> Vec<SelectionId> {
        let item = self
            .events
            .get(&event)
            .copied()
            .expect("causal event must remain visible");
        item.caused_by
            .iter()
            .map(|cause| SelectionId::Event(*cause))
            .filter(|cause| self.events.contains_key(cause))
            .collect()
    }

    fn children(&self, event: SelectionId) -> &[SelectionId] {
        self.children.get(&event).map(Vec::as_slice).unwrap_or(&[])
    }

    fn node(&self, event: SelectionId, depth: usize) -> EvidenceCausalNode {
        let item = self
            .events
            .get(&event)
            .copied()
            .expect("causal event must remain visible");
        EvidenceCausalNode {
            event: event.stable_key(),
            depth,
            world_time: item.world_time,
            title: item.title.clone(),
            subtitle: item.subtitle.clone(),
            caused_by: self
                .parents(event)
                .into_iter()
                .map(|cause| cause.stable_key())
                .collect(),
        }
    }
}

pub fn query_why(
    snapshot: &ProjectionSnapshot,
    event: SelectionId,
) -> Result<EvidenceWhyResult, QueryError> {
    let graph = VisibleCausalGraph::new(snapshot);
    graph.require_event(event)?;

    let mut discovered = std::collections::BTreeSet::from([event]);
    let mut queue = std::collections::VecDeque::from([(event, 0usize)]);
    let mut nodes = Vec::new();

    while let Some((current, depth)) = queue.pop_front() {
        nodes.push(graph.node(current, depth));
        for cause in graph.parents(current) {
            if discovered.insert(cause) {
                queue.push_back((cause, depth + 1));
            }
        }
    }

    Ok(EvidenceWhyResult {
        event: event.stable_key(),
        nodes,
    })
}

pub fn query_influence(
    snapshot: &ProjectionSnapshot,
    event: SelectionId,
) -> Result<EvidenceInfluenceResult, QueryError> {
    let graph = VisibleCausalGraph::new(snapshot);
    graph.require_event(event)?;

    let mut discovered = std::collections::BTreeSet::from([event]);
    let mut queue = std::collections::VecDeque::from([(event, 0usize)]);
    let mut nodes = Vec::new();

    while let Some((current, depth)) = queue.pop_front() {
        nodes.push(graph.node(current, depth));
        for child in graph.children(current) {
            if discovered.insert(*child) {
                queue.push_back((*child, depth + 1));
            }
        }
    }

    Ok(EvidenceInfluenceResult {
        event: event.stable_key(),
        nodes,
    })
}

pub fn query_causal_path(
    snapshot: &ProjectionSnapshot,
    from: SelectionId,
    to: SelectionId,
) -> Result<EvidenceCausalPathResult, QueryError> {
    let graph = VisibleCausalGraph::new(snapshot);
    graph.require_event(from)?;
    graph.require_event(to)?;

    let mut discovered = std::collections::BTreeSet::from([from]);
    let mut queue = std::collections::VecDeque::from([from]);
    let mut predecessor = std::collections::BTreeMap::<SelectionId, SelectionId>::new();

    while let Some(current) = queue.pop_front() {
        if current == to {
            break;
        }
        for child in graph.children(current) {
            if discovered.insert(*child) {
                predecessor.insert(*child, current);
                queue.push_back(*child);
            }
        }
    }

    if !discovered.contains(&to) {
        return Err(QueryError::NoCausalPath {
            from: from.stable_key(),
            to: to.stable_key(),
        });
    }

    let mut path = vec![to];
    let mut current = to;
    while current != from {
        current = *predecessor
            .get(&current)
            .expect("discovered causal target must have a predecessor");
        path.push(current);
    }
    path.reverse();

    Ok(EvidenceCausalPathResult {
        from: from.stable_key(),
        to: to.stable_key(),
        nodes: path
            .into_iter()
            .enumerate()
            .map(|(depth, event)| graph.node(event, depth))
            .collect(),
    })
}

pub fn query_causal_neighborhood(
    snapshot: &ProjectionSnapshot,
    root: SelectionId,
    upstream_depth: usize,
    downstream_depth: usize,
) -> Result<EvidenceCausalNeighborhoodResult, QueryError> {
    let graph = VisibleCausalGraph::new(snapshot);
    graph.require_event(root)?;

    let mut upstream_discovered = std::collections::BTreeSet::from([root]);
    let mut upstream_queue = std::collections::VecDeque::from([(root, 0usize)]);
    let mut upstream = Vec::new();
    let mut upstream_frontier = Vec::new();

    while let Some((current, depth)) = upstream_queue.pop_front() {
        if depth >= upstream_depth {
            if graph
                .parents(current)
                .into_iter()
                .any(|cause| !upstream_discovered.contains(&cause))
            {
                upstream_frontier.push(current.stable_key());
            }
            continue;
        }
        let next_depth = depth + 1;
        for cause in graph.parents(current) {
            if upstream_discovered.insert(cause) {
                upstream.push(graph.node(cause, next_depth));
                upstream_queue.push_back((cause, next_depth));
            }
        }
    }

    let mut downstream_discovered = std::collections::BTreeSet::from([root]);
    let mut downstream_queue = std::collections::VecDeque::from([(root, 0usize)]);
    let mut downstream = Vec::new();
    let mut downstream_frontier = Vec::new();

    while let Some((current, depth)) = downstream_queue.pop_front() {
        if depth >= downstream_depth {
            if graph
                .children(current)
                .iter()
                .any(|child| !downstream_discovered.contains(child))
            {
                downstream_frontier.push(current.stable_key());
            }
            continue;
        }
        let next_depth = depth + 1;
        for child in graph.children(current) {
            if downstream_discovered.insert(*child) {
                downstream.push(graph.node(*child, next_depth));
                downstream_queue.push_back((*child, next_depth));
            }
        }
    }

    let upstream_continuations = upstream_frontier
        .iter()
        .map(|event| EvidenceCausalContinuation {
            event: event.clone(),
            direction: EvidenceCausalDirection::Upstream,
            request: EvidenceQueryRequest::CausalNeighborhood {
                root: event.clone(),
                upstream_depth: upstream_depth.max(1),
                downstream_depth: 0,
            },
        })
        .collect();
    let downstream_continuations = downstream_frontier
        .iter()
        .map(|event| EvidenceCausalContinuation {
            event: event.clone(),
            direction: EvidenceCausalDirection::Downstream,
            request: EvidenceQueryRequest::CausalNeighborhood {
                root: event.clone(),
                upstream_depth: 0,
                downstream_depth: downstream_depth.max(1),
            },
        })
        .collect();

    Ok(EvidenceCausalNeighborhoodResult {
        root: graph.node(root, 0),
        upstream_depth,
        downstream_depth,
        upstream,
        downstream,
        upstream_truncated: !upstream_frontier.is_empty(),
        downstream_truncated: !downstream_frontier.is_empty(),
        upstream_frontier,
        downstream_frontier,
        upstream_continuations,
        downstream_continuations,
    })
}

pub fn query_neighborhood(
    snapshot: &ProjectionSnapshot,
    root: SelectionId,
    max_depth: usize,
) -> Result<EvidenceNeighborhoodResult, QueryError> {
    let neighborhood = snapshot
        .state_evidence_neighborhood(root, max_depth)
        .ok_or_else(|| QueryError::SelectionNotVisible(root.stable_key()))?;
    Ok(EvidenceNeighborhoodResult {
        root: root.stable_key(),
        max_depth,
        nodes: neighborhood
            .nodes
            .into_iter()
            .map(|node| EvidenceNode {
                selection: node.selection.stable_key(),
                depth: node.depth,
            })
            .collect(),
        edges: neighborhood.edges.into_iter().map(edge_record).collect(),
    })
}

pub fn query_shortest_path(
    snapshot: &ProjectionSnapshot,
    from: SelectionId,
    to: SelectionId,
) -> Result<EvidencePathResult, QueryError> {
    require_visible(snapshot, from)?;
    require_visible(snapshot, to)?;
    let steps = snapshot
        .state_evidence_shortest_path(from, to)
        .ok_or_else(|| QueryError::NoEvidencePath {
            from: from.stable_key(),
            to: to.stable_key(),
        })?;
    Ok(EvidencePathResult {
        from: from.stable_key(),
        to: to.stable_key(),
        steps: steps
            .into_iter()
            .map(|step| EvidencePathStep {
                from: step.from.stable_key(),
                edge: edge_record(step.edge),
                to: step.to.stable_key(),
            })
            .collect(),
    })
}

pub fn query_neighborhood_comparison(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    root: SelectionId,
    max_depth: usize,
) -> Result<EvidenceComparisonResult, QueryError> {
    let comparison = compare_evidence_neighborhoods(left, right, root, max_depth)
        .ok_or_else(|| QueryError::SelectionNotVisibleInEitherWorld(root.stable_key()))?;
    Ok(EvidenceComparisonResult {
        root: root.stable_key(),
        max_depth,
        identical: comparison.is_identical(),
        nodes: comparison
            .nodes
            .into_iter()
            .map(|node| EvidenceNodeDifference {
                selection: node.selection.stable_key(),
                kind: difference_record(node.kind),
                left_depth: node.left_depth,
                right_depth: node.right_depth,
            })
            .collect(),
        left_only_edges: comparison
            .edges
            .left_only
            .into_iter()
            .map(edge_record)
            .collect(),
        right_only_edges: comparison
            .edges
            .right_only
            .into_iter()
            .map(edge_record)
            .collect(),
    })
}

fn require_visible(
    snapshot: &ProjectionSnapshot,
    selection: SelectionId,
) -> Result<(), QueryError> {
    snapshot
        .state_evidence_neighborhood(selection, 0)
        .map(|_| ())
        .ok_or_else(|| QueryError::SelectionNotVisible(selection.stable_key()))
}

fn edge_record(edge: StateEvidenceEdge) -> EvidenceEdge {
    match edge {
        StateEvidenceEdge::EntityEvent(evidence) => EvidenceEdge::EntityEvent {
            entity: SelectionId::Entity(evidence.entity).stable_key(),
            event: SelectionId::Event(evidence.event).stable_key(),
        },
        StateEvidenceEdge::RelationEvent(evidence) => EvidenceEdge::RelationEvent {
            relation: SelectionId::Relation(evidence.relation).stable_key(),
            event: SelectionId::Event(evidence.event).stable_key(),
        },
        StateEvidenceEdge::EntityRelation(evidence) => EvidenceEdge::EntityRelation {
            entity: SelectionId::Entity(evidence.entity).stable_key(),
            relation: SelectionId::Relation(evidence.relation).stable_key(),
            role: match evidence.role {
                RelationEndpointRole::From => RelationRole::From,
                RelationEndpointRole::To => RelationRole::To,
            },
        },
    }
}

fn difference_record(kind: DifferenceKind) -> Difference {
    match kind {
        DifferenceKind::LeftOnly => Difference::LeftOnly,
        DifferenceKind::RightOnly => Difference::RightOnly,
        DifferenceKind::Changed => Difference::Changed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use world_core::{EntityId, EventId, RelationId};
    use world_projection::{
        EntityEventEvidence, EntityRelationEvidence, InspectorProjection, InspectorRow,
        InspectorSection, RelationEventEvidence, TimelineItem, TimelineProjection,
        ENTITY_HISTORY_SECTION, RELATION_ENDPOINTS_SECTION, RELATION_HISTORY_SECTION,
        RELATION_IDENTITY_SECTION,
    };

    fn snapshot(from: EntityId, to: EntityId) -> ProjectionSnapshot {
        let one = SelectionId::Entity(EntityId::new(1));
        let two = SelectionId::Entity(EntityId::new(2));
        let three = SelectionId::Entity(EntityId::new(3));
        let relation = SelectionId::Relation(RelationId::new(5));
        let event = SelectionId::Event(EventId::new(9));
        let entity = |selection: SelectionId, history: bool| InspectorProjection {
            selection,
            title: selection.stable_key(),
            subtitle: "Entity".into(),
            sections: history
                .then(|| InspectorSection {
                    title: ENTITY_HISTORY_SECTION.into(),
                    rows: vec![InspectorRow {
                        label: "World time 9".into(),
                        value: event.stable_key(),
                    }],
                })
                .into_iter()
                .collect(),
        };
        ProjectionSnapshot {
            timeline: TimelineProjection {
                items: vec![TimelineItem {
                    id: event,
                    world_time: 9,
                    title: "Changed".into(),
                    subtitle: "Recorded change".into(),
                    caused_by: Vec::new(),
                }],
            },
            inspectors: BTreeMap::from([
                (one, entity(one, false)),
                (two, entity(two, true)),
                (three, entity(three, false)),
                (
                    relation,
                    InspectorProjection {
                        selection: relation,
                        title: "Knows".into(),
                        subtitle: "Relation #5 · Active".into(),
                        sections: vec![
                            InspectorSection {
                                title: RELATION_ENDPOINTS_SECTION.into(),
                                rows: vec![
                                    InspectorRow {
                                        label: "From".into(),
                                        value: SelectionId::Entity(from).stable_key(),
                                    },
                                    InspectorRow {
                                        label: "To".into(),
                                        value: SelectionId::Entity(to).stable_key(),
                                    },
                                ],
                            },
                            InspectorSection {
                                title: RELATION_HISTORY_SECTION.into(),
                                rows: vec![InspectorRow {
                                    label: "World time 9".into(),
                                    value: event.stable_key(),
                                }],
                            },
                        ],
                    },
                ),
            ]),
            ..ProjectionSnapshot::default()
        }
    }

    #[test]
    fn query_errors_have_stable_serializable_shapes() {
        let cases = [
            (
                QueryError::InvalidSelectionKey("entity-01".into()),
                r#"{"error":"invalid-selection-key","details":"entity-01"}"#,
            ),
            (
                QueryError::SelectionKindMismatch {
                    selection: "entity-1".into(),
                    expected: EvidenceSelectionKind::Event,
                },
                r#"{"error":"selection-kind-mismatch","details":{"selection":"entity-1","expected":"event"}}"#,
            ),
            (
                QueryError::SelectionNotVisible("entity-99".into()),
                r#"{"error":"selection-not-visible","details":"entity-99"}"#,
            ),
            (
                QueryError::NoEvidencePath {
                    from: "entity-1".into(),
                    to: "event-9".into(),
                },
                r#"{"error":"no-evidence-path","details":{"from":"entity-1","to":"event-9"}}"#,
            ),
            (
                QueryError::NoCausalPath {
                    from: "event-1".into(),
                    to: "event-9".into(),
                },
                r#"{"error":"no-causal-path","details":{"from":"event-1","to":"event-9"}}"#,
            ),
            (
                QueryError::SelectionNotVisibleInEitherWorld("relation-5".into()),
                r#"{"error":"selection-not-visible-in-either-world","details":"relation-5"}"#,
            ),
        ];

        for (error, expected_json) in cases {
            let json = serde_json::to_string(&error).unwrap();
            assert_eq!(json, expected_json);
            let restored: QueryError = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, error);
        }
    }

    #[test]
    fn serialized_selection_discovery_returns_query_visible_selections_in_typed_order() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let hidden_event = SelectionId::Event(EventId::new(10));
        snapshot.inspectors.insert(
            hidden_event,
            InspectorProjection {
                selection: hidden_event,
                title: "Hidden event".into(),
                subtitle: "Inspector only".into(),
                sections: Vec::new(),
            },
        );

        let request: EvidenceQueryRequest =
            serde_json::from_str(r#"{"query":"selections"}"#).unwrap();
        let response = execute_query(&snapshot, &request).unwrap();
        let EvidenceQueryResponse::Selections { value } = response else {
            panic!("expected selections response")
        };

        assert_eq!(
            value
                .selections
                .iter()
                .map(|selection| selection.selection.as_str())
                .collect::<Vec<_>>(),
            vec!["entity-1", "entity-2", "entity-3", "relation-5", "event-9"]
        );
        assert_eq!(
            value
                .selections
                .iter()
                .map(|selection| selection.kind)
                .collect::<Vec<_>>(),
            vec![
                EvidenceSelectionKind::Entity,
                EvidenceSelectionKind::Entity,
                EvidenceSelectionKind::Entity,
                EvidenceSelectionKind::Relation,
                EvidenceSelectionKind::Event,
            ]
        );
        assert_eq!(value.selections[3].title, "Knows");
        assert_eq!(value.selections[4].title, "Changed");
        assert!(!value
            .selections
            .iter()
            .any(|selection| selection.selection == "event-10"));
    }

    #[test]
    fn selection_discovery_response_round_trips_through_query_contract() {
        let snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let response = execute_query(&snapshot, &EvidenceQueryRequest::Selections).unwrap();
        let json = serde_json::to_string(&response).unwrap();
        let restored: EvidenceQueryResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, response);
    }

    #[test]
    fn describe_returns_display_safe_entity_and_relation_details() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let entity = SelectionId::Entity(EntityId::new(2));
        snapshot
            .inspectors
            .get_mut(&entity)
            .unwrap()
            .sections
            .insert(
                0,
                InspectorSection {
                    title: "State".into(),
                    rows: vec![InspectorRow {
                        label: "Status".into(),
                        value: "Active".into(),
                    }],
                },
            );

        let response = execute_query(
            &snapshot,
            &EvidenceQueryRequest::Describe {
                selection: entity.stable_key(),
            },
        )
        .unwrap();
        let EvidenceQueryResponse::Description { value } = response else {
            panic!("expected description response")
        };
        assert_eq!(value.selection, "entity-2");
        assert_eq!(value.kind, EvidenceSelectionKind::Entity);
        assert_eq!(value.title, "entity-2");
        assert_eq!(value.sections.len(), 1);
        assert_eq!(value.sections[0].title, "State");
        assert!(!value
            .sections
            .iter()
            .any(|section| section.title == ENTITY_HISTORY_SECTION));

        let relation = SelectionId::Relation(RelationId::new(5));
        let relation_inspector = snapshot.inspectors.get_mut(&relation).unwrap();
        relation_inspector.sections.insert(
            0,
            InspectorSection {
                title: "Relation".into(),
                rows: vec![InspectorRow {
                    label: "Status".into(),
                    value: "Active".into(),
                }],
            },
        );
        relation_inspector.sections.push(InspectorSection {
            title: RELATION_IDENTITY_SECTION.into(),
            rows: vec![InspectorRow {
                label: "From".into(),
                value: "entity-1".into(),
            }],
        });

        let response = execute_query(
            &snapshot,
            &EvidenceQueryRequest::Describe {
                selection: relation.stable_key(),
            },
        )
        .unwrap();
        let EvidenceQueryResponse::Description { value } = response else {
            panic!("expected description response")
        };
        assert_eq!(value.kind, EvidenceSelectionKind::Relation);
        assert_eq!(value.title, "Knows");
        assert_eq!(value.sections.len(), 1);
        assert_eq!(value.sections[0].title, "Relation");
        for internal in [
            RELATION_HISTORY_SECTION,
            RELATION_ENDPOINTS_SECTION,
            RELATION_IDENTITY_SECTION,
        ] {
            assert!(!value
                .sections
                .iter()
                .any(|section| section.title == internal));
        }
    }

    #[test]
    fn describe_event_uses_timeline_visibility_and_labels() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let event = SelectionId::Event(EventId::new(9));
        snapshot.inspectors.insert(
            event,
            InspectorProjection {
                selection: event,
                title: "Inspector title must not win".into(),
                subtitle: "Inspector subtitle must not win".into(),
                sections: vec![InspectorSection {
                    title: "Context".into(),
                    rows: vec![InspectorRow {
                        label: "Actor".into(),
                        value: "entity-1".into(),
                    }],
                }],
            },
        );

        let response = execute_query(
            &snapshot,
            &EvidenceQueryRequest::Describe {
                selection: event.stable_key(),
            },
        )
        .unwrap();
        let EvidenceQueryResponse::Description { value } = response else {
            panic!("expected description response")
        };
        assert_eq!(value.kind, EvidenceSelectionKind::Event);
        assert_eq!(value.title, "Changed");
        assert_eq!(value.subtitle, "Recorded change");
        assert_eq!(value.sections[0].title, "Context");

        let hidden_event = SelectionId::Event(EventId::new(10));
        snapshot.inspectors.insert(
            hidden_event,
            InspectorProjection {
                selection: hidden_event,
                title: "Hidden".into(),
                subtitle: "Inspector only".into(),
                sections: Vec::new(),
            },
        );
        assert_eq!(
            execute_query(
                &snapshot,
                &EvidenceQueryRequest::Describe {
                    selection: hidden_event.stable_key(),
                },
            ),
            Err(QueryError::SelectionNotVisible("event-10".into()))
        );
    }

    #[test]
    fn describe_contract_round_trips_and_reuses_key_validation() {
        let snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let request: EvidenceQueryRequest =
            serde_json::from_str(r#"{"query":"describe","selection":"relation-5"}"#).unwrap();
        let response = execute_query(&snapshot, &request).unwrap();
        let json = serde_json::to_string(&response).unwrap();
        let restored: EvidenceQueryResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, response);

        assert_eq!(
            execute_query(
                &snapshot,
                &EvidenceQueryRequest::Describe {
                    selection: "entity-07".into(),
                },
            ),
            Err(QueryError::InvalidSelectionKey("entity-07".into()))
        );
    }

    #[test]
    fn why_query_walks_visible_persisted_causes_in_deterministic_order() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        snapshot.timeline.items = vec![
            TimelineItem {
                id: SelectionId::Event(EventId::new(3)),
                world_time: 3,
                title: "Final effect".into(),
                subtitle: "Final".into(),
                caused_by: vec![EventId::new(2)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(2)),
                world_time: 2,
                title: "Intermediate effect".into(),
                subtitle: "Middle".into(),
                caused_by: vec![EventId::new(1)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(1)),
                world_time: 1,
                title: "Root cause".into(),
                subtitle: "Root".into(),
                caused_by: Vec::new(),
            },
        ];

        let request: EvidenceQueryRequest =
            serde_json::from_str(r#"{"query":"why","event":"event-3"}"#).unwrap();
        let response = execute_query(&snapshot, &request).unwrap();
        let EvidenceQueryResponse::Why { value } = response else {
            panic!("expected why response")
        };
        assert_eq!(value.event, "event-3");
        assert_eq!(
            value
                .nodes
                .iter()
                .map(|node| (node.event.as_str(), node.depth))
                .collect::<Vec<_>>(),
            vec![("event-3", 0), ("event-2", 1), ("event-1", 2)]
        );
        assert_eq!(value.nodes[0].caused_by, vec!["event-2"]);
        assert_eq!(value.nodes[1].caused_by, vec!["event-1"]);

        let json = serde_json::to_string(&EvidenceQueryResponse::Why {
            value: value.clone(),
        })
        .unwrap();
        let restored: EvidenceQueryResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, EvidenceQueryResponse::Why { value });
    }

    #[test]
    fn why_query_preserves_direct_cause_order_and_minimum_depth() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        snapshot.timeline.items = vec![
            TimelineItem {
                id: SelectionId::Event(EventId::new(3)),
                world_time: 3,
                title: "Final".into(),
                subtitle: "Root".into(),
                caused_by: vec![EventId::new(2), EventId::new(1)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(2)),
                world_time: 2,
                title: "First direct cause".into(),
                subtitle: "Also points at event 1".into(),
                caused_by: vec![EventId::new(1)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(1)),
                world_time: 1,
                title: "Second direct cause".into(),
                subtitle: "Direct and indirect".into(),
                caused_by: Vec::new(),
            },
        ];

        let value = query_why(&snapshot, SelectionId::Event(EventId::new(3))).unwrap();
        assert_eq!(value.nodes[0].caused_by, vec!["event-2", "event-1"]);
        assert_eq!(
            value
                .nodes
                .iter()
                .map(|node| (node.event.as_str(), node.depth))
                .collect::<Vec<_>>(),
            vec![("event-3", 0), ("event-2", 1), ("event-1", 1)]
        );
    }

    #[test]
    fn why_query_filters_hidden_causes_and_cycle_protects() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        snapshot.timeline.items = vec![
            TimelineItem {
                id: SelectionId::Event(EventId::new(3)),
                world_time: 3,
                title: "Final".into(),
                subtitle: "Visible".into(),
                caused_by: vec![EventId::new(2), EventId::new(99)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(2)),
                world_time: 2,
                title: "Cycle".into(),
                subtitle: "Visible".into(),
                caused_by: vec![EventId::new(3)],
            },
        ];

        let value = query_why(&snapshot, SelectionId::Event(EventId::new(3))).unwrap();
        assert_eq!(value.nodes.len(), 2);
        assert_eq!(value.nodes[0].caused_by, vec!["event-2"]);
        assert_eq!(value.nodes[1].caused_by, vec!["event-3"]);
    }

    #[test]
    fn why_query_enforces_event_kind_and_timeline_visibility() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        assert_eq!(
            execute_query(
                &snapshot,
                &EvidenceQueryRequest::Why {
                    event: "entity-1".into(),
                },
            ),
            Err(QueryError::SelectionKindMismatch {
                selection: "entity-1".into(),
                expected: EvidenceSelectionKind::Event,
            })
        );
        assert_eq!(
            execute_query(
                &snapshot,
                &EvidenceQueryRequest::Why {
                    event: "event-07".into(),
                },
            ),
            Err(QueryError::InvalidSelectionKey("event-07".into()))
        );

        let hidden = SelectionId::Event(EventId::new(10));
        snapshot.inspectors.insert(
            hidden,
            InspectorProjection {
                selection: hidden,
                title: "Inspector only".into(),
                subtitle: "Hidden".into(),
                sections: Vec::new(),
            },
        );
        assert_eq!(
            query_why(&snapshot, hidden),
            Err(QueryError::SelectionNotVisible("event-10".into()))
        );
    }

    #[test]
    fn serialized_query_requests_execute_without_callers_parsing_selection_ids() {
        let snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let request: EvidenceQueryRequest =
            serde_json::from_str(r#"{"query":"neighborhood","root":"relation-5","max_depth":2}"#)
                .unwrap();
        let response = execute_query(&snapshot, &request).unwrap();
        let EvidenceQueryResponse::Neighborhood { value } = response else {
            panic!("expected neighborhood response");
        };
        assert_eq!(value.root, "relation-5");
        assert!(value
            .nodes
            .iter()
            .any(|node| node.selection == "entity-2" && node.depth == 2));

        let request: EvidenceQueryRequest = serde_json::from_str(
            r#"{"query":"shortest-path","from":"relation-5","to":"entity-2"}"#,
        )
        .unwrap();
        let response = execute_query(&snapshot, &request).unwrap();
        let EvidenceQueryResponse::ShortestPath { value } = response else {
            panic!("expected shortest path response");
        };
        assert_eq!(value.from, "relation-5");
        assert_eq!(value.to, "entity-2");
        assert_eq!(value.steps.len(), 2);
    }

    #[test]
    fn query_contract_rejects_noncanonical_selection_keys() {
        let snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let request = EvidenceQueryRequest::Neighborhood {
            root: "entity-01".into(),
            max_depth: 2,
        };
        assert_eq!(
            execute_query(&snapshot, &request),
            Err(QueryError::InvalidSelectionKey("entity-01".into()))
        );
    }

    #[test]
    fn comparison_request_executes_typed_future_comparison() {
        let left = snapshot(EntityId::new(1), EntityId::new(3));
        let right = snapshot(EntityId::new(3), EntityId::new(1));
        let request: EvidenceComparisonRequest =
            serde_json::from_str(r#"{"root":"relation-5","max_depth":1}"#).unwrap();
        let result = execute_comparison_query(&left, &right, &request).unwrap();
        assert!(!result.identical);
        assert_eq!(result.left_only_edges.len(), 2);
        assert_eq!(result.right_only_edges.len(), 2);

        let encoded = serde_json::to_string(&result).unwrap();
        let restored: EvidenceComparisonResult = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored, result);
    }

    #[test]
    fn neighborhood_and_path_are_stable_serializable_dtos() {
        let snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let relation = SelectionId::Relation(RelationId::new(5));
        let two = SelectionId::Entity(EntityId::new(2));
        let neighborhood = query_neighborhood(&snapshot, relation, 2).unwrap();
        assert_eq!(neighborhood.root, "relation-5");
        assert!(neighborhood
            .nodes
            .iter()
            .any(|node| { node.selection == "entity-2" && node.depth == 2 }));
        assert!(neighborhood.edges.contains(&EvidenceEdge::EntityRelation {
            entity: "entity-1".into(),
            relation: "relation-5".into(),
            role: RelationRole::From,
        }));

        let path = query_shortest_path(&snapshot, relation, two).unwrap();
        assert_eq!(path.steps.len(), 2);
        assert_eq!(
            path.steps[0].edge,
            EvidenceEdge::RelationEvent {
                relation: "relation-5".into(),
                event: "event-9".into(),
            }
        );

        let json = serde_json::to_string(&path).unwrap();
        let restored: EvidencePathResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, path);
    }

    #[test]
    fn comparison_preserves_typed_endpoint_role_differences() {
        let left = snapshot(EntityId::new(1), EntityId::new(3));
        let right = snapshot(EntityId::new(3), EntityId::new(1));
        let result = query_neighborhood_comparison(
            &left,
            &right,
            SelectionId::Relation(RelationId::new(5)),
            1,
        )
        .unwrap();

        assert!(!result.identical);
        assert!(result.nodes.is_empty());
        assert_eq!(result.left_only_edges.len(), 2);
        assert!(result
            .left_only_edges
            .contains(&EvidenceEdge::EntityRelation {
                entity: "entity-1".into(),
                relation: "relation-5".into(),
                role: RelationRole::From,
            }));
        assert!(result
            .right_only_edges
            .contains(&EvidenceEdge::EntityRelation {
                entity: "entity-1".into(),
                relation: "relation-5".into(),
                role: RelationRole::To,
            }));
    }

    #[test]
    fn invisible_and_disconnected_queries_have_explicit_errors() {
        let snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        let hidden = SelectionId::Entity(EntityId::new(99));
        assert_eq!(
            query_neighborhood(&snapshot, hidden, 2),
            Err(QueryError::SelectionNotVisible("entity-99".into()))
        );

        let mut disconnected = snapshot.clone();
        let isolated = SelectionId::Entity(EntityId::new(4));
        disconnected.inspectors.insert(
            isolated,
            InspectorProjection {
                selection: isolated,
                title: "entity-4".into(),
                subtitle: "Entity".into(),
                sections: Vec::new(),
            },
        );
        assert_eq!(
            query_shortest_path(&disconnected, isolated, SelectionId::Event(EventId::new(9))),
            Err(QueryError::NoEvidencePath {
                from: "entity-4".into(),
                to: "event-9".into(),
            })
        );
    }

    #[test]
    fn edge_record_matches_existing_typed_runtime_edges() {
        assert_eq!(
            edge_record(StateEvidenceEdge::EntityEvent(EntityEventEvidence {
                entity: EntityId::new(1),
                event: EventId::new(9),
            })),
            EvidenceEdge::EntityEvent {
                entity: "entity-1".into(),
                event: "event-9".into(),
            }
        );
        assert_eq!(
            edge_record(StateEvidenceEdge::RelationEvent(RelationEventEvidence {
                relation: RelationId::new(5),
                event: EventId::new(9),
            })),
            EvidenceEdge::RelationEvent {
                relation: "relation-5".into(),
                event: "event-9".into(),
            }
        );
        assert_eq!(
            edge_record(StateEvidenceEdge::EntityRelation(EntityRelationEvidence {
                entity: EntityId::new(1),
                relation: RelationId::new(5),
                role: RelationEndpointRole::From,
            })),
            EvidenceEdge::EntityRelation {
                entity: "entity-1".into(),
                relation: "relation-5".into(),
                role: RelationRole::From,
            }
        );
    }
}
