from pathlib import Path

lib = Path("crates/world-projection/src/lib.rs")
text = lib.read_text()
marker = """#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateEvidencePathStep {
    pub from: SelectionId,
    pub edge: StateEvidenceEdge,
    pub to: SelectionId,
}
"""
insert = marker + """
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct StateEvidenceNeighborhoodNode {
    pub selection: SelectionId,
    pub depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateEvidenceNeighborhood {
    pub root: SelectionId,
    pub max_depth: usize,
    pub nodes: Vec<StateEvidenceNeighborhoodNode>,
    pub edges: Vec<StateEvidenceEdge>,
}
"""
if text.count(marker) != 1:
    raise SystemExit(f"expected path step marker once, found {text.count(marker)}")
text = text.replace(marker, insert, 1)

method_marker = """    pub fn state_evidence_shortest_path(
        &self,
        start: SelectionId,
        goal: SelectionId,
    ) -> Option<Vec<StateEvidencePathStep>> {
"""
idx = text.find(method_marker)
if idx < 0:
    raise SystemExit("shortest path method not found")
visibility_marker = """    fn state_evidence_selection_is_visible(&self, selection: SelectionId) -> bool {
"""
vis_idx = text.find(visibility_marker, idx)
if vis_idx < 0:
    raise SystemExit("visibility helper not found")
new_method = """    pub fn state_evidence_neighborhood(
        &self,
        root: SelectionId,
        max_depth: usize,
    ) -> Option<StateEvidenceNeighborhood> {
        if !self.state_evidence_selection_is_visible(root) {
            return None;
        }

        let edges = self.state_evidence_edges();
        let mut depths = BTreeMap::from([(root, 0usize)]);
        let mut queue = VecDeque::from([root]);

        while let Some(current) = queue.pop_front() {
            let current_depth = depths[&current];
            if current_depth >= max_depth {
                continue;
            }

            let mut adjacent = edges
                .iter()
                .filter_map(|edge| edge.other(current))
                .collect::<Vec<_>>();
            adjacent.sort();
            adjacent.dedup();

            for next in adjacent {
                if depths.contains_key(&next) {
                    continue;
                }
                depths.insert(next, current_depth + 1);
                queue.push_back(next);
            }
        }

        let mut nodes = depths
            .iter()
            .map(|(selection, depth)| StateEvidenceNeighborhoodNode {
                selection: *selection,
                depth: *depth,
            })
            .collect::<Vec<_>>();
        nodes.sort_by_key(|node| (node.depth, node.selection));

        let visible = depths.keys().copied().collect::<BTreeSet<_>>();
        let edges = edges
            .into_iter()
            .filter(|edge| {
                let (left, right) = edge.selections();
                visible.contains(&left) && visible.contains(&right)
            })
            .collect();

        Some(StateEvidenceNeighborhood {
            root,
            max_depth,
            nodes,
            edges,
        })
    }

"""
text = text[:vis_idx] + new_method + text[vis_idx:]
lib.write_text(text)

test = Path("crates/world-projection/tests/state_evidence_graph.rs")
t = test.read_text()
t = t.replace(
    "    StateEvidenceEdge, StateEvidencePathStep, TimelineItem, TimelineProjection,",
    "    StateEvidenceEdge, StateEvidenceNeighborhood, StateEvidenceNeighborhoodNode,\n    StateEvidencePathStep, TimelineItem, TimelineProjection,",
    1,
)
append = r'''

#[test]
fn bounded_neighborhood_reports_minimum_depths_and_induced_typed_edges() {
    let snapshot = path_snapshot();
    let one = SelectionId::Entity(EntityId::new(1));
    let two = SelectionId::Entity(EntityId::new(2));
    let three = SelectionId::Entity(EntityId::new(3));
    let relation = SelectionId::Relation(RelationId::new(5));
    let event = SelectionId::Event(EventId::new(9));

    assert_eq!(
        snapshot.state_evidence_neighborhood(relation, 1),
        Some(StateEvidenceNeighborhood {
            root: relation,
            max_depth: 1,
            nodes: vec![
                StateEvidenceNeighborhoodNode {
                    selection: relation,
                    depth: 0,
                },
                StateEvidenceNeighborhoodNode {
                    selection: one,
                    depth: 1,
                },
                StateEvidenceNeighborhoodNode {
                    selection: three,
                    depth: 1,
                },
                StateEvidenceNeighborhoodNode {
                    selection: event,
                    depth: 1,
                },
            ],
            edges: vec![
                StateEvidenceEdge::RelationEvent(RelationEventEvidence {
                    relation: RelationId::new(5),
                    event: EventId::new(9),
                }),
                StateEvidenceEdge::EntityRelation(EntityRelationEvidence {
                    entity: EntityId::new(1),
                    relation: RelationId::new(5),
                    role: RelationEndpointRole::From,
                }),
                StateEvidenceEdge::EntityRelation(EntityRelationEvidence {
                    entity: EntityId::new(3),
                    relation: RelationId::new(5),
                    role: RelationEndpointRole::To,
                }),
            ],
        })
    );

    let depth_two = snapshot
        .state_evidence_neighborhood(relation, 2)
        .expect("visible root should produce a neighborhood");
    assert_eq!(
        depth_two
            .nodes
            .iter()
            .find(|node| node.selection == two)
            .map(|node| node.depth),
        Some(2)
    );
    assert!(depth_two.edges.contains(&StateEvidenceEdge::EntityEvent(
        EntityEventEvidence {
            entity: EntityId::new(2),
            event: EventId::new(9),
        }
    )));
}

#[test]
fn neighborhood_depth_zero_and_hidden_root_are_explicit() {
    let snapshot = path_snapshot();
    let relation = SelectionId::Relation(RelationId::new(5));
    let hidden = SelectionId::Entity(EntityId::new(99));

    assert_eq!(
        snapshot.state_evidence_neighborhood(relation, 0),
        Some(StateEvidenceNeighborhood {
            root: relation,
            max_depth: 0,
            nodes: vec![StateEvidenceNeighborhoodNode {
                selection: relation,
                depth: 0,
            }],
            edges: Vec::new(),
        })
    );
    assert_eq!(snapshot.state_evidence_neighborhood(hidden, 2), None);
}

#[test]
fn neighborhood_handles_cycles_and_preserves_parallel_self_relation_roles() {
    let entity = EntityId::new(1);
    let relation = RelationId::new(5);
    let entity_selection = SelectionId::Entity(entity);
    let relation_selection = SelectionId::Relation(relation);
    let snapshot = ProjectionSnapshot {
        inspectors: BTreeMap::from([
            (
                entity_selection,
                InspectorProjection {
                    selection: entity_selection,
                    title: "One".into(),
                    subtitle: "Person".into(),
                    sections: Vec::new(),
                },
            ),
            (
                relation_selection,
                InspectorProjection {
                    selection: relation_selection,
                    title: "Reflects".into(),
                    subtitle: "Relation #5 · Active".into(),
                    sections: vec![InspectorSection {
                        title: RELATION_ENDPOINTS_SECTION.into(),
                        rows: vec![
                            InspectorRow {
                                label: "From".into(),
                                value: entity_selection.stable_key(),
                            },
                            InspectorRow {
                                label: "To".into(),
                                value: entity_selection.stable_key(),
                            },
                        ],
                    }],
                },
            ),
        ]),
        ..ProjectionSnapshot::default()
    };

    let neighborhood = snapshot
        .state_evidence_neighborhood(entity_selection, 8)
        .expect("visible self relation should be explorable");
    assert_eq!(
        neighborhood.nodes,
        vec![
            StateEvidenceNeighborhoodNode {
                selection: entity_selection,
                depth: 0,
            },
            StateEvidenceNeighborhoodNode {
                selection: relation_selection,
                depth: 1,
            },
        ]
    );
    assert_eq!(neighborhood.edges.len(), 2);
    assert!(neighborhood.edges.contains(&StateEvidenceEdge::EntityRelation(
        EntityRelationEvidence {
            entity,
            relation,
            role: RelationEndpointRole::From,
        }
    )));
    assert!(neighborhood.edges.contains(&StateEvidenceEdge::EntityRelation(
        EntityRelationEvidence {
            entity,
            relation,
            role: RelationEndpointRole::To,
        }
    )));
}
'''
if "fn bounded_neighborhood_reports_minimum_depths" in t:
    raise SystemExit("M175 tests already present")
t += append
test.write_text(t)
