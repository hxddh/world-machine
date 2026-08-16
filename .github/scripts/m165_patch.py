from pathlib import Path

path = Path('crates/world-projection/src/lib.rs')
text = path.read_text()

text = text.replace(
    'pub const RELATION_HISTORY_SECTION: &str = "Recorded relation changes";\n',
    'pub const RELATION_HISTORY_SECTION: &str = "Recorded relation changes";\n'
    'pub const RELATION_ENDPOINTS_SECTION: &str = "Active relation endpoints";\n',
)

marker = '''#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelationEventEvidence {
    pub relation: RelationId,
    pub event: EventId,
}
'''
assert text.count(marker) == 1
text = text.replace(marker, marker + '''
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EntityRelationEvidence {
    pub entity: EntityId,
    pub relation: RelationId,
}
''')

needle = '''    pub fn directly_changed_relations(&self, event: EventId) -> Vec<RelationId> {
        self.relation_event_evidence()
            .into_iter()
            .filter(|evidence| evidence.event == event)
            .map(|evidence| evidence.relation)
            .collect()
    }

    pub fn influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
'''
assert text.count(needle) == 1
replacement = '''    pub fn directly_changed_relations(&self, event: EventId) -> Vec<RelationId> {
        self.relation_event_evidence()
            .into_iter()
            .filter(|evidence| evidence.event == event)
            .map(|evidence| evidence.relation)
            .collect()
    }

    pub fn entity_relation_evidence(&self) -> Vec<EntityRelationEvidence> {
        let visible_entities = visible_entity_ids_by_key(&self.inspectors);
        let mut evidence = BTreeSet::new();
        for (selection, inspector) in &self.inspectors {
            let SelectionId::Relation(relation) = *selection else {
                continue;
            };
            evidence.extend(entity_relation_evidence_from_inspector(
                relation,
                inspector,
                &visible_entities,
            ));
        }
        evidence.into_iter().collect()
    }

    pub fn relations_for_entity(&self, entity: EntityId) -> Vec<RelationId> {
        self.entity_relation_evidence()
            .into_iter()
            .filter(|evidence| evidence.entity == entity)
            .map(|evidence| evidence.relation)
            .collect()
    }

    pub fn entities_for_relation(&self, relation: RelationId) -> Vec<EntityId> {
        self.entity_relation_evidence()
            .into_iter()
            .filter(|evidence| evidence.relation == relation)
            .map(|evidence| evidence.entity)
            .collect()
    }

    pub fn influence(&self, event: EventId) -> Vec<(usize, &TimelineItem)> {
'''
text = text.replace(needle, replacement)

marker = '''fn visible_event_ids_by_key(timeline: &TimelineProjection) -> BTreeMap<String, EventId> {
'''
assert text.count(marker) == 1
visible_entities = '''fn visible_entity_ids_by_key(
    inspectors: &BTreeMap<SelectionId, InspectorProjection>,
) -> BTreeMap<String, EntityId> {
    inspectors
        .keys()
        .filter_map(|selection| {
            let SelectionId::Entity(entity) = *selection else {
                return None;
            };
            Some((selection.stable_key(), entity))
        })
        .collect()
}

'''
text = text.replace(marker, visible_entities + marker)

marker = '''fn history_event_ids_from_inspector(
'''
assert text.count(marker) == 1
adjacency_helper = '''fn entity_relation_evidence_from_inspector(
    relation: RelationId,
    inspector: &InspectorProjection,
    visible_entities: &BTreeMap<String, EntityId>,
) -> BTreeSet<EntityRelationEvidence> {
    let Some(section) = inspector
        .sections
        .iter()
        .find(|section| section.title == RELATION_ENDPOINTS_SECTION)
    else {
        return BTreeSet::new();
    };

    section
        .rows
        .iter()
        .filter_map(|row| visible_entities.get(row.value.as_str()).copied())
        .map(|entity| EntityRelationEvidence { entity, relation })
        .collect()
}

'''
text = text.replace(marker, adjacency_helper + marker)

old_display = '''                ENTITY_HISTORY_SECTION | RELATION_HISTORY_SECTION
'''
new_display = '''                ENTITY_HISTORY_SECTION | RELATION_HISTORY_SECTION | RELATION_ENDPOINTS_SECTION
'''
assert text.count(old_display) == 1
text = text.replace(old_display, new_display)

needle = '''    if !properties.is_empty() {
        sections.push(InspectorSection {
            title: "Properties".into(),
            rows: properties,
        });
    }
    if !recorded_changes.is_empty() {
'''
assert text.count(needle) == 1
replacement = '''    if !properties.is_empty() {
        sections.push(InspectorSection {
            title: "Properties".into(),
            rows: properties,
        });
    }
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
path.write_text(text)

Path('crates/world-projection/tests/entity_relation_adjacency.rs').write_text(r'''use std::collections::BTreeMap;
use world_core::{
    Entity, EntityId, Event, EventId, Relation, RelationId, StateChange, World, WorldState,
};
use world_projection::{
    inspectors_from_world, timeline_from_world, EntityRelationEvidence, InspectorProjection,
    InspectorRow, InspectorSection, ProjectionSnapshot, SelectionId, RELATION_ENDPOINTS_SECTION,
};

fn snapshot(world: &World) -> ProjectionSnapshot {
    ProjectionSnapshot {
        timeline: timeline_from_world(world),
        inspectors: inspectors_from_world(world),
        ..ProjectionSnapshot::default()
    }
}

fn baseline() -> (WorldState, EntityId, EntityId, EntityId, RelationId) {
    let one = EntityId::new(1);
    let two = EntityId::new(2);
    let three = EntityId::new(3);
    let relation = RelationId::new(5);
    let mut state = WorldState::default();
    for entity in [one, two, three] {
        state
            .seed_entity(Entity::new(entity, "person"))
            .expect("entity should seed");
    }
    state
        .seed_relation(Relation::new(relation, "knows", one, two))
        .expect("relation should seed");
    (state, one, two, three, relation)
}

fn event(id: u64, kind: &str, changes: Vec<StateChange>) -> Event {
    Event {
        id: EventId::new(id),
        kind: kind.into(),
        world_time: id,
        actor: None,
        targets: Vec::new(),
        caused_by: Vec::new(),
        payload: BTreeMap::new(),
        changes,
    }
}

#[test]
fn active_relation_exposes_typed_entity_adjacency() {
    let (baseline, one, two, _three, relation) = baseline();
    let world = World::new(baseline);
    let snapshot = snapshot(&world);

    assert_eq!(
        snapshot.entity_relation_evidence(),
        vec![
            EntityRelationEvidence {
                entity: one,
                relation,
            },
            EntityRelationEvidence {
                entity: two,
                relation,
            },
        ]
    );
    assert_eq!(snapshot.relations_for_entity(one), vec![relation]);
    assert_eq!(snapshot.entities_for_relation(relation), vec![one, two]);
}

#[test]
fn removed_relation_tombstone_is_not_current_entity_adjacency() {
    let (baseline, one, two, _three, relation) = baseline();
    let world = World::from_history(
        baseline,
        &[event(1, "remove_relation", vec![StateChange::RemoveRelation(relation)])],
    )
    .expect("history should replay");
    let snapshot = snapshot(&world);

    assert!(snapshot
        .inspector(SelectionId::Relation(relation))
        .expect("tombstone should remain inspectable")
        .subtitle
        .contains("Removed"));
    assert!(snapshot.entity_relation_evidence().is_empty());
    assert!(snapshot.relations_for_entity(one).is_empty());
    assert!(snapshot.relations_for_entity(two).is_empty());
    assert!(snapshot.entities_for_relation(relation).is_empty());
}

#[test]
fn reused_relation_id_exposes_only_latest_active_endpoints() {
    let (baseline, one, two, three, relation) = baseline();
    let world = World::from_history(
        baseline,
        &[
            event(1, "remove_old", vec![StateChange::RemoveRelation(relation)]),
            event(
                2,
                "create_new",
                vec![StateChange::CreateRelation(Relation::new(
                    relation, "supports", two, three,
                ))],
            ),
        ],
    )
    .expect("history should replay");
    let snapshot = snapshot(&world);

    assert!(snapshot.relations_for_entity(one).is_empty());
    assert_eq!(snapshot.relations_for_entity(two), vec![relation]);
    assert_eq!(snapshot.relations_for_entity(three), vec![relation]);
    assert_eq!(snapshot.entities_for_relation(relation), vec![two, three]);
}

#[test]
fn adjacency_does_not_expose_hidden_entity_ids_from_partial_snapshots() {
    let visible = EntityId::new(1);
    let hidden = EntityId::new(99);
    let relation = RelationId::new(5);
    let entity_selection = SelectionId::Entity(visible);
    let relation_selection = SelectionId::Relation(relation);
    let snapshot = ProjectionSnapshot {
        inspectors: BTreeMap::from([
            (
                entity_selection,
                InspectorProjection {
                    selection: entity_selection,
                    title: "Visible".into(),
                    subtitle: "Person".into(),
                    sections: Vec::new(),
                },
            ),
            (
                relation_selection,
                InspectorProjection {
                    selection: relation_selection,
                    title: "Knows".into(),
                    subtitle: "Relation #5 · Active".into(),
                    sections: vec![InspectorSection {
                        title: RELATION_ENDPOINTS_SECTION.into(),
                        rows: vec![
                            InspectorRow {
                                label: "From".into(),
                                value: SelectionId::Entity(visible).stable_key(),
                            },
                            InspectorRow {
                                label: "To".into(),
                                value: SelectionId::Entity(hidden).stable_key(),
                            },
                        ],
                    }],
                },
            ),
        ]),
        ..ProjectionSnapshot::default()
    };

    assert_eq!(
        snapshot.entity_relation_evidence(),
        vec![EntityRelationEvidence {
            entity: visible,
            relation,
        }]
    );
    assert_eq!(snapshot.entities_for_relation(relation), vec![visible]);
}
''')
