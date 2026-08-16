from pathlib import Path

projection_path = Path('crates/world-projection/src/lib.rs')
text = projection_path.read_text()

text = text.replace(
    'pub const RELATION_ENDPOINTS_SECTION: &str = "Active relation endpoints";\n',
    'pub const RELATION_ENDPOINTS_SECTION: &str = "Active relation endpoints";\n'
    'pub const RELATION_IDENTITY_SECTION: &str = "Relation identity endpoints";\n',
)

marker = '''#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RelationEndpointRole {
    From,
    To,
}
'''
assert text.count(marker) == 1
text = text.replace(marker, marker + '''
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelationIdentity {
    pub from: EntityId,
    pub to: EntityId,
}
''')

needle = '''    pub fn entities_for_relation(&self, relation: RelationId) -> Vec<EntityId> {
        self.entity_relation_evidence()
            .into_iter()
            .filter(|evidence| evidence.relation == relation)
            .map(|evidence| evidence.entity)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn state_evidence_edges(&self) -> Vec<StateEvidenceEdge> {
'''
assert text.count(needle) == 1
replacement = '''    pub fn entities_for_relation(&self, relation: RelationId) -> Vec<EntityId> {
        self.entity_relation_evidence()
            .into_iter()
            .filter(|evidence| evidence.relation == relation)
            .map(|evidence| evidence.entity)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn relation_identity(&self, relation: RelationId) -> Option<RelationIdentity> {
        self.inspector(SelectionId::Relation(relation))
            .and_then(relation_identity_from_inspector)
    }

    pub fn state_evidence_edges(&self) -> Vec<StateEvidenceEdge> {
'''
text = text.replace(needle, replacement)

marker = '''fn history_event_ids_from_inspector(
'''
assert text.count(marker) == 1
identity_helper = '''fn relation_identity_from_inspector(
    inspector: &InspectorProjection,
) -> Option<RelationIdentity> {
    let section = inspector
        .sections
        .iter()
        .find(|section| section.title == RELATION_IDENTITY_SECTION)?;
    let endpoint = |label: &str| {
        section
            .rows
            .iter()
            .find(|row| row.label == label)
            .and_then(|row| entity_id_from_stable_key(&row.value))
    };
    Some(RelationIdentity {
        from: endpoint("From")?,
        to: endpoint("To")?,
    })
}

fn entity_id_from_stable_key(key: &str) -> Option<EntityId> {
    key.strip_prefix("entity-")?
        .parse::<u64>()
        .ok()
        .map(EntityId::new)
}

'''
text = text.replace(marker, identity_helper + marker)

old_display = '''                ENTITY_HISTORY_SECTION | RELATION_HISTORY_SECTION | RELATION_ENDPOINTS_SECTION
'''
new_display = '''                ENTITY_HISTORY_SECTION
                    | RELATION_HISTORY_SECTION
                    | RELATION_ENDPOINTS_SECTION
                    | RELATION_IDENTITY_SECTION
'''
assert text.count(old_display) == 1
text = text.replace(old_display, new_display)

needle = '''    if recorded.active {
        sections.push(InspectorSection {
            title: RELATION_ENDPOINTS_SECTION.into(),
            rows: vec![
                InspectorRow {
                    label: "From".into(),
                    value: SelectionId::Entity(relation.from).stable_key(),
                },
                InspectorRow {
                    label: "To".into(),
                    value: SelectionId::Entity(relation.to).stable_key(),
                },
            ],
        });
    }
    if !recorded_changes.is_empty() {
'''
assert text.count(needle) == 1
replacement = '''    sections.push(InspectorSection {
        title: RELATION_IDENTITY_SECTION.into(),
        rows: vec![
            InspectorRow {
                label: "From".into(),
                value: SelectionId::Entity(relation.from).stable_key(),
            },
            InspectorRow {
                label: "To".into(),
                value: SelectionId::Entity(relation.to).stable_key(),
            },
        ],
    });
    if recorded.active {
        sections.push(InspectorSection {
            title: RELATION_ENDPOINTS_SECTION.into(),
            rows: vec![
                InspectorRow {
                    label: "From".into(),
                    value: SelectionId::Entity(relation.from).stable_key(),
                },
                InspectorRow {
                    label: "To".into(),
                    value: SelectionId::Entity(relation.to).stable_key(),
                },
            ],
        });
    }
    if !recorded_changes.is_empty() {
'''
text = text.replace(needle, replacement)
projection_path.write_text(text)

Path('crates/world-projection/tests/relation_identity.rs').write_text(r'''use std::collections::BTreeMap;
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, World, WorldState,
};
use world_projection::{
    inspectors_from_world, ProjectionSnapshot, RelationIdentity, SelectionId,
};

fn event(id: u64, changes: Vec<StateChange>) -> Event {
    Event {
        id: EventId::new(id),
        kind: "relation_change".into(),
        world_time: id,
        actor: None,
        targets: Vec::new(),
        caused_by: Vec::new(),
        payload: BTreeMap::new(),
        changes,
    }
}

#[test]
fn relation_identity_is_stable_for_active_and_removed_latest_incarnations() {
    let one = EntityId::new(1);
    let two = EntityId::new(2);
    let relation = RelationId::new(5);
    let mut baseline = WorldState::default();
    baseline.seed_entity(Entity::new(one, "person")).unwrap();
    baseline.seed_entity(Entity::new(two, "person")).unwrap();
    baseline
        .seed_relation(Relation::new(relation, "knows", one, two))
        .unwrap();

    let active_world = World::new(baseline.clone());
    let active = ProjectionSnapshot {
        inspectors: inspectors_from_world(&active_world),
        ..ProjectionSnapshot::default()
    };
    assert_eq!(
        active.relation_identity(relation),
        Some(RelationIdentity { from: one, to: two })
    );

    let removed_world = World::from_history(
        baseline,
        &[event(1, vec![StateChange::RemoveRelation(relation)])],
    )
    .unwrap();
    let removed = ProjectionSnapshot {
        inspectors: inspectors_from_world(&removed_world),
        ..ProjectionSnapshot::default()
    };
    assert!(removed
        .inspector(SelectionId::Relation(relation))
        .unwrap()
        .subtitle
        .contains("Removed"));
    assert_eq!(
        removed.relation_identity(relation),
        Some(RelationIdentity { from: one, to: two })
    );
}
''')

compare_path = Path('crates/world-compare/src/lib.rs')
text = compare_path.read_text()

old = '''                (Some(left), Some(right)) if same_relation_state(left, right) => None,
'''
new = '''                (Some(left), Some(right))
                    if same_relation_state(id, left, right, left_relations_snapshot(left, right), right_relations_snapshot(left, right)) => None,
'''
# Do not use this awkward form; replace whole compare_relations function below.
assert text.count(old) == 1

start = text.index('fn compare_relations(\n')
end = text.index('\nfn relation_inspectors(', start)
old_fn = text[start:end]
new_fn = '''fn compare_relations(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
) -> Vec<RelationDifference> {
    let left_relations = relation_inspectors(left);
    let right_relations = relation_inspectors(right);
    let ids = left_relations
        .keys()
        .chain(right_relations.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    ids.into_iter()
        .filter_map(
            |id| match (left_relations.get(&id), right_relations.get(&id)) {
                (Some(left_inspector), Some(right_inspector))
                    if same_relation_state(
                        id,
                        left,
                        right,
                        left_inspector,
                        right_inspector,
                    ) =>
                {
                    None
                }
                (Some(left), Some(right)) => Some(RelationDifference {
                    id,
                    kind: DifferenceKind::Changed,
                    left: Some(relation_view(left)),
                    right: Some(relation_view(right)),
                    inspector_rows: compare_inspector_rows(left, right),
                }),
                (Some(left), None) => Some(RelationDifference {
                    id,
                    kind: DifferenceKind::LeftOnly,
                    left: Some(relation_view(left)),
                    right: None,
                    inspector_rows: Vec::new(),
                }),
                (None, Some(right)) => Some(RelationDifference {
                    id,
                    kind: DifferenceKind::RightOnly,
                    left: None,
                    right: Some(relation_view(right)),
                    inspector_rows: Vec::new(),
                }),
                (None, None) => None,
            },
        )
        .collect()
}
'''
text = text[:start] + new_fn + text[end:]

old = '''fn same_relation_state(left: &InspectorProjection, right: &InspectorProjection) -> bool {
    same_inspector_state(left, right)
}
'''
new = '''fn same_relation_state(
    id: SelectionId,
    left_snapshot: &ProjectionSnapshot,
    right_snapshot: &ProjectionSnapshot,
    left: &InspectorProjection,
    right: &InspectorProjection,
) -> bool {
    let SelectionId::Relation(relation) = id else {
        return false;
    };
    left.title == right.title
        && left.subtitle == right.subtitle
        && indexed_relation_state_rows(left) == indexed_relation_state_rows(right)
        && left_snapshot.relation_identity(relation) == right_snapshot.relation_identity(relation)
}

fn indexed_relation_state_rows(
    inspector: &InspectorProjection,
) -> BTreeMap<InspectorRowKey, &String> {
    indexed_rows_filter(inspector, |section, row| {
        !(section == "Relation" && matches!(row, "From" | "To"))
    })
}
'''
assert text.count(old) == 1
text = text.replace(old, new)

old = '''fn indexed_rows(inspector: &InspectorProjection) -> BTreeMap<InspectorRowKey, &String> {
    let mut rows = BTreeMap::new();
    let mut duplicates = BTreeMap::<(String, String), usize>::new();

    for section in inspector.display_sections() {
        for row in &section.rows {
            let duplicate_key = (section.title.clone(), row.label.clone());
            let ordinal = duplicates.entry(duplicate_key.clone()).or_default();
            let key = InspectorRowKey {
                section: duplicate_key.0,
                label: duplicate_key.1,
                ordinal: *ordinal,
            };
            *ordinal += 1;
            rows.insert(key, &row.value);
        }
    }

    rows
}
'''
new = '''fn indexed_rows(inspector: &InspectorProjection) -> BTreeMap<InspectorRowKey, &String> {
    indexed_rows_filter(inspector, |_, _| true)
}

fn indexed_rows_filter<'a>(
    inspector: &'a InspectorProjection,
    mut include: impl FnMut(&str, &str) -> bool,
) -> BTreeMap<InspectorRowKey, &'a String> {
    let mut rows = BTreeMap::new();
    let mut duplicates = BTreeMap::<(String, String), usize>::new();

    for section in inspector.display_sections() {
        for row in &section.rows {
            if !include(&section.title, &row.label) {
                continue;
            }
            let duplicate_key = (section.title.clone(), row.label.clone());
            let ordinal = duplicates.entry(duplicate_key.clone()).or_default();
            let key = InspectorRowKey {
                section: duplicate_key.0,
                label: duplicate_key.1,
                ordinal: *ordinal,
            };
            *ordinal += 1;
            rows.insert(key, &row.value);
        }
    }

    rows
}
'''
assert text.count(old) == 1
text = text.replace(old, new)

old_import = '''        InspectorRow, InspectorSection, TimelineProjection, ENTITY_HISTORY_SECTION,
        RELATION_HISTORY_SECTION,
'''
new_import = '''        InspectorRow, InspectorSection, TimelineProjection, ENTITY_HISTORY_SECTION,
        RELATION_HISTORY_SECTION, RELATION_IDENTITY_SECTION,
'''
assert text.count(old_import) == 1
text = text.replace(old_import, new_import)

start = text.index('    fn relation_inspector(\n')
end = text.index('\n    fn event(', start)
old_helper = text[start:end]
new_helper = '''    fn relation_inspector(
        id: u64,
        kind: &str,
        status: &str,
        strength: &str,
    ) -> (SelectionId, InspectorProjection) {
        relation_inspector_with_endpoints(
            id,
            kind,
            status,
            strength,
            1,
            "Alice · Entity #1",
            2,
            "Bob · Entity #2",
        )
    }

    fn relation_inspector_with_endpoints(
        id: u64,
        kind: &str,
        status: &str,
        strength: &str,
        from_id: u64,
        from_text: &str,
        to_id: u64,
        to_text: &str,
    ) -> (SelectionId, InspectorProjection) {
        let selection = SelectionId::Relation(RelationId::new(id));
        (
            selection,
            InspectorProjection {
                selection,
                title: kind.into(),
                subtitle: format!("Relation #{id} · {status}"),
                sections: vec![
                    InspectorSection {
                        title: "Relation".into(),
                        rows: vec![
                            InspectorRow {
                                label: "From".into(),
                                value: from_text.into(),
                            },
                            InspectorRow {
                                label: "To".into(),
                                value: to_text.into(),
                            },
                            InspectorRow {
                                label: "Status".into(),
                                value: status.into(),
                            },
                        ],
                    },
                    InspectorSection {
                        title: "Properties".into(),
                        rows: vec![InspectorRow {
                            label: "Strength".into(),
                            value: strength.into(),
                        }],
                    },
                    InspectorSection {
                        title: RELATION_IDENTITY_SECTION.into(),
                        rows: vec![
                            InspectorRow {
                                label: "From".into(),
                                value: SelectionId::Entity(EntityId::new(from_id)).stable_key(),
                            },
                            InspectorRow {
                                label: "To".into(),
                                value: SelectionId::Entity(EntityId::new(to_id)).stable_key(),
                            },
                        ],
                    },
                ],
            },
        )
    }
'''
text = text[:start] + new_helper + text[end:]

marker = '''    #[test]
    fn relation_history_evidence_does_not_change_current_state_comparison() {
'''
assert text.count(marker) == 1
new_tests = '''    #[test]
    fn relation_endpoint_name_drift_does_not_change_stable_relation_state() {
        let left = snapshot(
            20,
            [relation_inspector_with_endpoints(
                5,
                "Works With",
                "Active",
                "2",
                1,
                "Alice · Entity #1",
                2,
                "Bob · Entity #2",
            )],
            vec![],
            vec![],
        );
        let right = snapshot(
            20,
            [relation_inspector_with_endpoints(
                5,
                "Works With",
                "Active",
                "2",
                1,
                "Alicia · Entity #1",
                2,
                "Robert · Entity #2",
            )],
            vec![],
            vec![],
        );

        assert!(compare_snapshots(&left, &right).relations.is_empty());
    }

    #[test]
    fn relation_endpoint_identity_change_is_a_relation_state_change() {
        let left = snapshot(
            20,
            [relation_inspector_with_endpoints(
                5,
                "Works With",
                "Active",
                "2",
                1,
                "Alice · Entity #1",
                2,
                "Bob · Entity #2",
            )],
            vec![],
            vec![],
        );
        let right = snapshot(
            20,
            [relation_inspector_with_endpoints(
                5,
                "Works With",
                "Active",
                "2",
                1,
                "Alice · Entity #1",
                3,
                "Carol · Entity #3",
            )],
            vec![],
            vec![],
        );

        let comparison = compare_snapshots(&left, &right);
        assert_eq!(comparison.relations.len(), 1);
        assert_eq!(comparison.relations[0].kind, DifferenceKind::Changed);
        assert!(comparison.relations[0].inspector_rows.iter().any(|row| {
            row.key.label == "To"
                && row.left.as_deref() == Some("Bob · Entity #2")
                && row.right.as_deref() == Some("Carol · Entity #3")
        }));
    }

    #[test]
    fn removed_relation_endpoint_name_drift_does_not_change_tombstone_identity() {
        let left = snapshot(
            20,
            [relation_inspector_with_endpoints(
                5,
                "Works With",
                "Removed",
                "2",
                1,
                "Alice · Entity #1",
                2,
                "Bob · Entity #2",
            )],
            vec![],
            vec![],
        );
        let right = snapshot(
            20,
            [relation_inspector_with_endpoints(
                5,
                "Works With",
                "Removed",
                "2",
                1,
                "Renamed Alice · Entity #1",
                2,
                "Renamed Bob · Entity #2",
            )],
            vec![],
            vec![],
        );

        assert!(compare_snapshots(&left, &right).relations.is_empty());
    }

'''
text = text.replace(marker, new_tests + marker)
compare_path.write_text(text)
