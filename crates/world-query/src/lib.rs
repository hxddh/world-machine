use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use world_compare::{compare_evidence_neighborhoods, DifferenceKind};
use world_projection::{ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "kebab-case")]
pub enum EvidenceQueryRequest {
    Selections,
    Neighborhood { root: String, max_depth: usize },
    ShortestPath { from: String, to: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum EvidenceQueryResponse {
    Selections { value: EvidenceSelectionIndex },
    Neighborhood { value: EvidenceNeighborhoodResult },
    ShortestPath { value: EvidencePathResult },
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
    SelectionNotVisible(String),
    NoEvidencePath { from: String, to: String },
    SelectionNotVisibleInEitherWorld(String),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelectionKey(selection) => {
                write!(f, "invalid selection key: {selection}")
            }
            Self::SelectionNotVisible(selection) => {
                write!(f, "selection is not visible: {selection}")
            }
            Self::NoEvidencePath { from, to } => write!(f, "no evidence path: {from} -> {to}"),
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
