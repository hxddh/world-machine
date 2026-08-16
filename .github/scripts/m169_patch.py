from pathlib import Path

projection_path = Path('crates/world-projection/src/lib.rs')
text = projection_path.read_text()

old = '''#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EntityRelationEvidence {
    pub entity: EntityId,
    pub relation: RelationId,
}
'''
new = '''#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RelationEndpointRole {
    From,
    To,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EntityRelationEvidence {
    pub entity: EntityId,
    pub relation: RelationId,
    pub role: RelationEndpointRole,
}
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

old = '''    pub fn relations_for_entity(&self, entity: EntityId) -> Vec<RelationId> {
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
'''
new = '''    pub fn relations_for_entity(&self, entity: EntityId) -> Vec<RelationId> {
        self.entity_relation_evidence()
            .into_iter()
            .filter(|evidence| evidence.entity == entity)
            .map(|evidence| evidence.relation)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn entities_for_relation(&self, relation: RelationId) -> Vec<EntityId> {
        self.entity_relation_evidence()
            .into_iter()
            .filter(|evidence| evidence.relation == relation)
            .map(|evidence| evidence.entity)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)

old = '''    section
        .rows
        .iter()
        .filter_map(|row| visible_entities.get(row.value.as_str()).copied())
        .map(|entity| EntityRelationEvidence { entity, relation })
        .collect()
}
'''
new = '''    section
        .rows
        .iter()
        .filter_map(|row| {
            let role = match row.label.as_str() {
                "From" | "from" => RelationEndpointRole::From,
                "To" | "to" => RelationEndpointRole::To,
                _ => return None,
            };
            visible_entities
                .get(row.value.as_str())
                .copied()
                .map(|entity| EntityRelationEvidence {
                    entity,
                    relation,
                    role,
                })
        })
        .collect()
}
'''
assert text.count(old) == 1, text.count(old)
text = text.replace(old, new)
projection_path.write_text(text)

adjacency_path = Path('crates/world-projection/tests/entity_relation_adjacency.rs')
adj = adjacency_path.read_text()
adj = adj.replace(
    'InspectorRow, InspectorSection, ProjectionSnapshot, SelectionId, RELATION_ENDPOINTS_SECTION,',
    'InspectorRow, InspectorSection, ProjectionSnapshot, RelationEndpointRole, SelectionId,\n    RELATION_ENDPOINTS_SECTION,',
)
adj = adj.replace(
'''            EntityRelationEvidence {
                entity: one,
                relation,
            },
            EntityRelationEvidence {
                entity: two,
                relation,
            },''',
'''            EntityRelationEvidence {
                entity: one,
                relation,
                role: RelationEndpointRole::From,
            },
            EntityRelationEvidence {
                entity: two,
                relation,
                role: RelationEndpointRole::To,
            },''',
)
adj = adj.replace(
'''        vec![EntityRelationEvidence {
            entity: visible,
            relation,
        }]''',
'''        vec![EntityRelationEvidence {
            entity: visible,
            relation,
            role: RelationEndpointRole::From,
        }]''',
)
# Add self-loop invariant so convenience APIs remain duplicate-free while typed evidence preserves both roles.
marker = '''#[test]
fn removed_relation_tombstone_is_not_current_entity_adjacency() {
'''
assert adj.count(marker) == 1
self_loop = '''#[test]
fn self_relation_preserves_both_endpoint_roles_without_duplicating_convenience_results() {
    let entity = EntityId::new(1);
    let relation = RelationId::new(5);
    let mut baseline = WorldState::default();
    baseline
        .seed_entity(Entity::new(entity, "person"))
        .expect("entity should seed");
    baseline
        .seed_relation(Relation::new(relation, "reflects", entity, entity))
        .expect("self relation should seed");
    let snapshot = snapshot(&World::new(baseline));

    assert_eq!(
        snapshot.entity_relation_evidence(),
        vec![
            EntityRelationEvidence {
                entity,
                relation,
                role: RelationEndpointRole::From,
            },
            EntityRelationEvidence {
                entity,
                relation,
                role: RelationEndpointRole::To,
            },
        ]
    );
    assert_eq!(snapshot.relations_for_entity(entity), vec![relation]);
    assert_eq!(snapshot.entities_for_relation(relation), vec![entity]);
}

'''
adj = adj.replace(marker, self_loop + marker)
adjacency_path.write_text(adj)

wire_path = Path('crates/world-pack-protocol/tests/entity_relation_adjacency_roundtrip.rs')
wire = wire_path.read_text()
wire = wire.replace(
    'EntityRelationEvidence, InspectorProjection, InspectorRow, InspectorSection,\n    ProjectionSnapshot, SelectionId, RELATION_ENDPOINTS_SECTION,',
    'EntityRelationEvidence, InspectorProjection, InspectorRow, InspectorSection,\n    ProjectionSnapshot, RelationEndpointRole, SelectionId, RELATION_ENDPOINTS_SECTION,',
)
wire = wire.replace(
'''            EntityRelationEvidence {
                entity: left,
                relation,
            },
            EntityRelationEvidence {
                entity: right,
                relation,
            },''',
'''            EntityRelationEvidence {
                entity: left,
                relation,
                role: RelationEndpointRole::From,
            },
            EntityRelationEvidence {
                entity: right,
                relation,
                role: RelationEndpointRole::To,
            },''',
)
wire_path.write_text(wire)
