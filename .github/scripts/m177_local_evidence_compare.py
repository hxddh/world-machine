from pathlib import Path

path = Path("crates/world-compare/src/lib.rs")
text = path.read_text()
text = text.replace(
    "    InspectorProjection, ProjectionCommand, ProjectionSnapshot, SelectionId, TimelineItem,\n",
    "    InspectorProjection, ProjectionCommand, ProjectionSnapshot, SelectionId, StateEvidenceEdge,\n    TimelineItem,\n",
    1,
)

marker = """#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DifferenceKind {
    LeftOnly,
    RightOnly,
    Changed,
}
"""
insert = marker + """
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceNeighborhoodComparison {
    pub root: SelectionId,
    pub max_depth: usize,
    pub nodes: Vec<EvidenceNeighborhoodNodeDifference>,
    pub edges: EvidenceNeighborhoodEdgeDifference,
}

impl EvidenceNeighborhoodComparison {
    pub fn is_identical(&self) -> bool {
        self.nodes.is_empty() && self.edges.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceNeighborhoodNodeDifference {
    pub selection: SelectionId,
    pub kind: DifferenceKind,
    pub left_depth: Option<usize>,
    pub right_depth: Option<usize>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvidenceNeighborhoodEdgeDifference {
    pub left_only: Vec<StateEvidenceEdge>,
    pub right_only: Vec<StateEvidenceEdge>,
}

impl EvidenceNeighborhoodEdgeDifference {
    pub fn is_empty(&self) -> bool {
        self.left_only.is_empty() && self.right_only.is_empty()
    }
}
"""
if text.count(marker) != 1:
    raise SystemExit(f"expected DifferenceKind once, found {text.count(marker)}")
text = text.replace(marker, insert, 1)

marker2 = """pub fn compare_divergence(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
) -> Option<SnapshotDivergence> {
"""
function = """pub fn compare_evidence_neighborhoods(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    root: SelectionId,
    max_depth: usize,
) -> Option<EvidenceNeighborhoodComparison> {
    let left_neighborhood = left.state_evidence_neighborhood(root, max_depth);
    let right_neighborhood = right.state_evidence_neighborhood(root, max_depth);
    if left_neighborhood.is_none() && right_neighborhood.is_none() {
        return None;
    }

    let left_nodes = left_neighborhood
        .as_ref()
        .map(|neighborhood| {
            neighborhood
                .nodes
                .iter()
                .map(|node| (node.selection, node.depth))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let right_nodes = right_neighborhood
        .as_ref()
        .map(|neighborhood| {
            neighborhood
                .nodes
                .iter()
                .map(|node| (node.selection, node.depth))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let selections = left_nodes
        .keys()
        .chain(right_nodes.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let nodes = selections
        .into_iter()
        .filter_map(|selection| match (left_nodes.get(&selection), right_nodes.get(&selection)) {
            (Some(left_depth), Some(right_depth)) if left_depth == right_depth => None,
            (Some(left_depth), Some(right_depth)) => Some(EvidenceNeighborhoodNodeDifference {
                selection,
                kind: DifferenceKind::Changed,
                left_depth: Some(*left_depth),
                right_depth: Some(*right_depth),
            }),
            (Some(left_depth), None) => Some(EvidenceNeighborhoodNodeDifference {
                selection,
                kind: DifferenceKind::LeftOnly,
                left_depth: Some(*left_depth),
                right_depth: None,
            }),
            (None, Some(right_depth)) => Some(EvidenceNeighborhoodNodeDifference {
                selection,
                kind: DifferenceKind::RightOnly,
                left_depth: None,
                right_depth: Some(*right_depth),
            }),
            (None, None) => None,
        })
        .collect();

    let left_edges = left_neighborhood
        .as_ref()
        .map(|neighborhood| neighborhood.edges.iter().copied().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let right_edges = right_neighborhood
        .as_ref()
        .map(|neighborhood| neighborhood.edges.iter().copied().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let edges = EvidenceNeighborhoodEdgeDifference {
        left_only: left_edges.difference(&right_edges).copied().collect(),
        right_only: right_edges.difference(&left_edges).copied().collect(),
    };

    Some(EvidenceNeighborhoodComparison {
        root,
        max_depth,
        nodes,
        edges,
    })
}

"""
if text.count(marker2) != 1:
    raise SystemExit(f"expected compare_divergence once, found {text.count(marker2)}")
text = text.replace(marker2, function + marker2, 1)

text = text.replace(
    "        RELATION_HISTORY_SECTION, RELATION_IDENTITY_SECTION,\n",
    "        RELATION_ENDPOINTS_SECTION, RELATION_HISTORY_SECTION, RELATION_IDENTITY_SECTION,\n",
    1,
)

insert_before = """    #[test]
    fn identical_snapshots_have_no_differences() {
"""
helper = r'''    fn evidence_neighborhood_snapshot(
        relation_from: u64,
        relation_to: u64,
        event_entity: u64,
    ) -> ProjectionSnapshot {
        let one = SelectionId::Entity(EntityId::new(1));
        let two = SelectionId::Entity(EntityId::new(2));
        let three = SelectionId::Entity(EntityId::new(3));
        let relation = SelectionId::Relation(RelationId::new(5));
        let event = SelectionId::Event(EventId::new(9));
        let entity = |selection: SelectionId, history: bool| InspectorProjection {
            selection,
            title: format!("{selection:?}"),
            subtitle: "Person".into(),
            sections: history
                .then(|| InspectorSection {
                    title: ENTITY_HISTORY_SECTION.into(),
                    rows: vec![InspectorRow {
                        label: "World time 9 · Changed".into(),
                        value: event.stable_key(),
                    }],
                })
                .into_iter()
                .collect(),
        };
        snapshot(
            9,
            [
                (one, entity(one, event_entity == 1)),
                (two, entity(two, event_entity == 2)),
                (three, entity(three, event_entity == 3)),
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
                                        value: SelectionId::Entity(EntityId::new(relation_from))
                                            .stable_key(),
                                    },
                                    InspectorRow {
                                        label: "To".into(),
                                        value: SelectionId::Entity(EntityId::new(relation_to))
                                            .stable_key(),
                                    },
                                ],
                            },
                            InspectorSection {
                                title: RELATION_HISTORY_SECTION.into(),
                                rows: vec![InspectorRow {
                                    label: "World time 9 · Changed".into(),
                                    value: event.stable_key(),
                                }],
                            },
                        ],
                    },
                ),
            ],
            vec![TimelineItem {
                id: event,
                world_time: 9,
                title: "Changed".into(),
                subtitle: "Recorded change".into(),
                caused_by: Vec::new(),
            }],
            Vec::new(),
        )
    }

    #[test]
    fn identical_evidence_neighborhoods_have_no_local_differences() {
        let left = evidence_neighborhood_snapshot(1, 3, 2);
        let comparison = compare_evidence_neighborhoods(
            &left,
            &left,
            SelectionId::Relation(RelationId::new(5)),
            2,
        )
        .expect("visible root should compare");

        assert!(comparison.is_identical());
    }

    #[test]
    fn endpoint_role_changes_are_typed_edge_differences_even_when_nodes_match() {
        let left = evidence_neighborhood_snapshot(1, 3, 2);
        let right = evidence_neighborhood_snapshot(3, 1, 2);
        let comparison = compare_evidence_neighborhoods(
            &left,
            &right,
            SelectionId::Relation(RelationId::new(5)),
            1,
        )
        .expect("visible root should compare");

        assert!(comparison.nodes.is_empty());
        assert_eq!(comparison.edges.left_only.len(), 2);
        assert_eq!(comparison.edges.right_only.len(), 2);
        assert!(comparison.edges.left_only.iter().all(|edge| matches!(
            edge,
            StateEvidenceEdge::EntityRelation(_)
        )));
    }

    #[test]
    fn minimum_depth_changes_are_local_node_changes() {
        let left = evidence_neighborhood_snapshot(1, 3, 2);
        let right = evidence_neighborhood_snapshot(2, 3, 2);
        let comparison = compare_evidence_neighborhoods(
            &left,
            &right,
            SelectionId::Relation(RelationId::new(5)),
            2,
        )
        .expect("visible root should compare");

        assert!(comparison.nodes.iter().any(|node| {
            node.selection == SelectionId::Entity(EntityId::new(2))
                && node.kind == DifferenceKind::Changed
                && node.left_depth == Some(2)
                && node.right_depth == Some(1)
        }));
    }

    #[test]
    fn root_visibility_is_preserved_in_local_comparison() {
        let left = evidence_neighborhood_snapshot(1, 3, 2);
        let empty = snapshot(0, [], vec![], vec![]);
        let root = SelectionId::Relation(RelationId::new(5));
        let comparison = compare_evidence_neighborhoods(&left, &empty, root, 1)
            .expect("root visible on one side should still compare");

        assert!(comparison.nodes.iter().any(|node| {
            node.selection == root
                && node.kind == DifferenceKind::LeftOnly
                && node.left_depth == Some(0)
                && node.right_depth.is_none()
        }));
        assert_eq!(compare_evidence_neighborhoods(&empty, &empty, root, 1), None);
    }

'''
if text.count(insert_before) != 1:
    raise SystemExit(f"expected first test marker once, found {text.count(insert_before)}")
text = text.replace(insert_before, helper + insert_before, 1)
path.write_text(text)
