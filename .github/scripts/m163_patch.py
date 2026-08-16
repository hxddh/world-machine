from pathlib import Path

path = Path('crates/world-compare/src/lib.rs')
text = path.read_text()

text = text.replace('const ENTITY_EVIDENCE_SECTION: &str = "Recorded entity changes";\n\n', '')

old = '''pub struct SnapshotComparison {
    pub left: SnapshotSide,
    pub right: SnapshotSide,
    pub entities: Vec<EntityDifference>,
    pub timeline: TimelineDifference,
    pub commands: CommandDifference,
}
'''
new = '''pub struct SnapshotComparison {
    pub left: SnapshotSide,
    pub right: SnapshotSide,
    pub entities: Vec<EntityDifference>,
    pub relations: Vec<RelationDifference>,
    pub timeline: TimelineDifference,
    pub commands: CommandDifference,
}
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

old = '''        self.left == self.right
            && self.entities.is_empty()
            && self.timeline.is_empty()
            && self.commands.is_empty()
'''
new = '''        self.left == self.right
            && self.entities.is_empty()
            && self.relations.is_empty()
            && self.timeline.is_empty()
            && self.commands.is_empty()
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

marker = '''#[derive(Clone, Debug, PartialEq)]
pub struct EntityView {
    pub title: String,
    pub subtitle: String,
}
'''
assert text.count(marker) == 1
addition = marker + '''
#[derive(Clone, Debug, PartialEq)]
pub struct RelationDifference {
    pub id: SelectionId,
    pub kind: DifferenceKind,
    pub left: Option<RelationView>,
    pub right: Option<RelationView>,
    pub inspector_rows: Vec<InspectorRowDifference>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelationView {
    pub title: String,
    pub subtitle: String,
}
'''
text = text.replace(marker, addition)

old = '''        entities: compare_entities(left, right),
        timeline: compare_timeline(left, right),
'''
new = '''        entities: compare_entities(left, right),
        relations: compare_relations(left, right),
        timeline: compare_timeline(left, right),
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

marker = '''fn entity_inspectors(snapshot: &ProjectionSnapshot) -> BTreeMap<SelectionId, &InspectorProjection> {
'''
assert text.count(marker) == 1
relation_compare = '''fn compare_relations(
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
                (Some(left), Some(right)) if same_relation_state(left, right) => None,
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

fn relation_inspectors(
    snapshot: &ProjectionSnapshot,
) -> BTreeMap<SelectionId, &InspectorProjection> {
    snapshot
        .inspectors
        .iter()
        .filter_map(|(id, inspector)| match id {
            SelectionId::Relation(_) => Some((*id, inspector)),
            SelectionId::Entity(_) | SelectionId::Event(_) => None,
        })
        .collect()
}

fn relation_view(inspector: &InspectorProjection) -> RelationView {
    RelationView {
        title: inspector.title.clone(),
        subtitle: inspector.subtitle.clone(),
    }
}

fn same_relation_state(left: &InspectorProjection, right: &InspectorProjection) -> bool {
    same_inspector_state(left, right)
}

'''
text = text.replace(marker, relation_compare + marker)

old = '''fn same_entity_state(left: &InspectorProjection, right: &InspectorProjection) -> bool {
    left.title == right.title
        && left.subtitle == right.subtitle
        && indexed_rows(left) == indexed_rows(right)
}
'''
new = '''fn same_entity_state(left: &InspectorProjection, right: &InspectorProjection) -> bool {
    same_inspector_state(left, right)
}

fn same_inspector_state(left: &InspectorProjection, right: &InspectorProjection) -> bool {
    left.title == right.title
        && left.subtitle == right.subtitle
        && indexed_rows(left) == indexed_rows(right)
}
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

old = '''    for section in &inspector.sections {
        if section.title == ENTITY_EVIDENCE_SECTION {
            continue;
        }
        for row in &section.rows {
'''
new = '''    for section in inspector.display_sections() {
        for row in &section.rows {
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

old = '''    use world_core::{EntityId, EventId};
    use world_projection::{InspectorRow, InspectorSection, TimelineProjection};
'''
new = '''    use world_core::{EntityId, EventId, RelationId};
    use world_projection::{
        InspectorRow, InspectorSection, TimelineProjection, ENTITY_HISTORY_SECTION,
        RELATION_HISTORY_SECTION,
    };
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

marker = '''    fn event(id: u64, title: &str, world_time: u64) -> TimelineItem {
'''
assert text.count(marker) == 1
helper = '''    fn relation_inspector(
        id: u64,
        kind: &str,
        status: &str,
        strength: &str,
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
                        rows: vec![InspectorRow {
                            label: "Status".into(),
                            value: status.into(),
                        }],
                    },
                    InspectorSection {
                        title: "Properties".into(),
                        rows: vec![InspectorRow {
                            label: "Strength".into(),
                            value: strength.into(),
                        }],
                    },
                ],
            },
        )
    }

'''
text = text.replace(marker, helper + marker)

text = text.replace('title: ENTITY_EVIDENCE_SECTION.into(),', 'title: ENTITY_HISTORY_SECTION.into(),')

marker = '''    #[test]
    fn equal_event_id_with_divergent_content_is_changed_not_equal() {
'''
assert text.count(marker) == 1
relation_tests = '''    #[test]
    fn relation_rows_are_compared_inside_stable_relation_identity() {
        let left = snapshot(
            20,
            [relation_inspector(5, "Works With", "Active", "1")],
            vec![],
            vec![],
        );
        let right = snapshot(
            20,
            [relation_inspector(5, "Works With", "Removed", "2")],
            vec![],
            vec![],
        );

        let comparison = compare_snapshots(&left, &right);

        assert_eq!(comparison.relations.len(), 1);
        let relation = &comparison.relations[0];
        assert_eq!(relation.id, SelectionId::Relation(RelationId::new(5)));
        assert_eq!(relation.kind, DifferenceKind::Changed);
        assert!(relation.inspector_rows.iter().any(|row| {
            row.key.label == "Status"
                && row.left.as_deref() == Some("Active")
                && row.right.as_deref() == Some("Removed")
        }));
        assert!(relation.inspector_rows.iter().any(|row| {
            row.key.label == "Strength"
                && row.left.as_deref() == Some("1")
                && row.right.as_deref() == Some("2")
        }));
    }

    #[test]
    fn relation_history_evidence_does_not_change_current_state_comparison() {
        let (id, left) = relation_inspector(5, "Works With", "Removed", "2");
        let mut right = left.clone();
        right.sections.push(InspectorSection {
            title: RELATION_HISTORY_SECTION.into(),
            rows: vec![InspectorRow {
                label: "World time 12 · Removed".into(),
                value: "event-9".into(),
            }],
        });

        let comparison = compare_snapshots(
            &snapshot(20, [(id, left)], vec![], vec![]),
            &snapshot(20, [(id, right)], vec![], vec![]),
        );

        assert!(comparison.relations.is_empty());
    }

    #[test]
    fn added_and_removed_relations_are_reported() {
        let left = snapshot(
            0,
            [relation_inspector(5, "Works With", "Active", "1")],
            vec![],
            vec![],
        );
        let right = snapshot(
            0,
            [relation_inspector(6, "Supports", "Removed", "3")],
            vec![],
            vec![],
        );

        let comparison = compare_snapshots(&left, &right);

        assert_eq!(comparison.relations.len(), 2);
        assert_eq!(comparison.relations[0].kind, DifferenceKind::LeftOnly);
        assert_eq!(comparison.relations[1].kind, DifferenceKind::RightOnly);
    }

'''
text = text.replace(marker, relation_tests + marker)

path.write_text(text)
