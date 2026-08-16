from pathlib import Path

lib = Path("crates/world-projection/src/lib.rs")
text = lib.read_text()
text = text.replace(
    "use std::collections::{BTreeMap, BTreeSet};",
    "use std::collections::{BTreeMap, BTreeSet, VecDeque};",
    1,
)

marker = """#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionIntent {
"""
insert = """#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateEvidencePathStep {
    pub from: SelectionId,
    pub edge: StateEvidenceEdge,
    pub to: SelectionId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionIntent {
"""
if text.count(marker) != 1:
    raise SystemExit(f"expected ProjectionIntent marker once, found {text.count(marker)}")
text = text.replace(marker, insert, 1)

old = """    pub fn state_evidence_neighbors(&self, selection: SelectionId) -> Vec<SelectionId> {
        self.state_evidence_edges()
            .into_iter()
            .filter_map(|edge| edge.other(selection))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
"""
new = """    pub fn state_evidence_edges_for(&self, selection: SelectionId) -> Vec<StateEvidenceEdge> {
        self.state_evidence_edges()
            .into_iter()
            .filter(|edge| edge.other(selection).is_some())
            .collect()
    }

    pub fn state_evidence_neighbors(&self, selection: SelectionId) -> Vec<SelectionId> {
        self.state_evidence_edges_for(selection)
            .into_iter()
            .filter_map(|edge| edge.other(selection))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn state_evidence_shortest_path(
        &self,
        start: SelectionId,
        goal: SelectionId,
    ) -> Option<Vec<StateEvidencePathStep>> {
        if !self.state_evidence_selection_is_visible(start)
            || !self.state_evidence_selection_is_visible(goal)
        {
            return None;
        }
        if start == goal {
            return Some(Vec::new());
        }

        let edges = self.state_evidence_edges();
        let mut queue = VecDeque::from([start]);
        let mut visited = BTreeSet::from([start]);
        let mut previous = BTreeMap::<SelectionId, (SelectionId, StateEvidenceEdge)>::new();

        while let Some(current) = queue.pop_front() {
            let mut adjacent = edges
                .iter()
                .filter_map(|edge| edge.other(current).map(|next| (next, *edge)))
                .collect::<Vec<_>>();
            adjacent.sort();
            adjacent.dedup();

            for (next, edge) in adjacent {
                if !visited.insert(next) {
                    continue;
                }
                previous.insert(next, (current, edge));
                if next == goal {
                    let mut cursor = goal;
                    let mut path = Vec::new();
                    while cursor != start {
                        let (from, edge) = previous
                            .get(&cursor)
                            .copied()
                            .expect("visited evidence node must have a predecessor");
                        path.push(StateEvidencePathStep {
                            from,
                            edge,
                            to: cursor,
                        });
                        cursor = from;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(next);
            }
        }

        None
    }

    fn state_evidence_selection_is_visible(&self, selection: SelectionId) -> bool {
        match selection {
            SelectionId::Entity(_) | SelectionId::Relation(_) => {
                self.inspectors.contains_key(&selection)
            }
            SelectionId::Event(_) => self.timeline.items.iter().any(|item| item.id == selection),
        }
    }
"""
if text.count(old) != 1:
    raise SystemExit(f"expected neighbor method once, found {text.count(old)}")
text = text.replace(old, new, 1)
lib.write_text(text)

test = Path("crates/world-projection/tests/state_evidence_graph.rs")
t = test.read_text()
t = t.replace(
    "    StateEvidenceEdge, TimelineItem, TimelineProjection, ENTITY_HISTORY_SECTION,",
    "    StateEvidenceEdge, StateEvidencePathStep, TimelineItem, TimelineProjection,\n    ENTITY_HISTORY_SECTION,",
    1,
)
append = r'''

fn path_snapshot() -> ProjectionSnapshot {
    let one = SelectionId::Entity(EntityId::new(1));
    let two = SelectionId::Entity(EntityId::new(2));
    let three = SelectionId::Entity(EntityId::new(3));
    let isolated = SelectionId::Entity(EntityId::new(4));
    let relation = SelectionId::Relation(RelationId::new(5));
    let event = SelectionId::Event(EventId::new(9));
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
            (
                one,
                InspectorProjection {
                    selection: one,
                    title: "One".into(),
                    subtitle: "Person".into(),
                    sections: Vec::new(),
                },
            ),
            (
                two,
                InspectorProjection {
                    selection: two,
                    title: "Two".into(),
                    subtitle: "Person".into(),
                    sections: vec![InspectorSection {
                        title: ENTITY_HISTORY_SECTION.into(),
                        rows: vec![InspectorRow {
                            label: "World time 9 · Changed".into(),
                            value: event.stable_key(),
                        }],
                    }],
                },
            ),
            (
                three,
                InspectorProjection {
                    selection: three,
                    title: "Three".into(),
                    subtitle: "Person".into(),
                    sections: Vec::new(),
                },
            ),
            (
                isolated,
                InspectorProjection {
                    selection: isolated,
                    title: "Isolated".into(),
                    subtitle: "Person".into(),
                    sections: Vec::new(),
                },
            ),
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
                                    value: one.stable_key(),
                                },
                                InspectorRow {
                                    label: "To".into(),
                                    value: three.stable_key(),
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
        ]),
        ..ProjectionSnapshot::default()
    }
}

#[test]
fn shortest_path_traverses_current_and_recorded_evidence_without_losing_edge_types() {
    let snapshot = path_snapshot();
    let one = SelectionId::Entity(EntityId::new(1));
    let two = SelectionId::Entity(EntityId::new(2));
    let relation = SelectionId::Relation(RelationId::new(5));
    let event = SelectionId::Event(EventId::new(9));

    assert_eq!(
        snapshot.state_evidence_shortest_path(one, two),
        Some(vec![
            StateEvidencePathStep {
                from: one,
                edge: StateEvidenceEdge::EntityRelation(EntityRelationEvidence {
                    entity: EntityId::new(1),
                    relation: RelationId::new(5),
                    role: RelationEndpointRole::From,
                }),
                to: relation,
            },
            StateEvidencePathStep {
                from: relation,
                edge: StateEvidenceEdge::RelationEvent(RelationEventEvidence {
                    relation: RelationId::new(5),
                    event: EventId::new(9),
                }),
                to: event,
            },
            StateEvidencePathStep {
                from: event,
                edge: StateEvidenceEdge::EntityEvent(EntityEventEvidence {
                    entity: EntityId::new(2),
                    event: EventId::new(9),
                }),
                to: two,
            },
        ])
    );
}

#[test]
fn shortest_path_respects_visibility_and_disconnected_nodes() {
    let snapshot = path_snapshot();
    let one = SelectionId::Entity(EntityId::new(1));
    let isolated = SelectionId::Entity(EntityId::new(4));
    let hidden = SelectionId::Entity(EntityId::new(99));
    let event = SelectionId::Event(EventId::new(9));

    assert_eq!(snapshot.state_evidence_shortest_path(one, one), Some(vec![]));
    assert_eq!(snapshot.state_evidence_shortest_path(hidden, hidden), None);
    assert_eq!(snapshot.state_evidence_shortest_path(isolated, event), None);
}

#[test]
fn edges_for_selection_preserve_parallel_role_evidence_while_neighbors_deduplicate() {
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

    assert_eq!(snapshot.state_evidence_edges_for(entity_selection).len(), 2);
    assert_eq!(
        snapshot.state_evidence_neighbors(entity_selection),
        vec![relation_selection]
    );
}
'''
if "fn path_snapshot()" in t:
    raise SystemExit("M173 tests already present")
t += append
test.write_text(t)
