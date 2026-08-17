from pathlib import Path

query = Path("crates/world-query/src/lib.rs")
text = query.read_text()

old = '''use world_projection::{ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge};
'''
new = '''use world_projection::{
    InspectorProjection, ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge,
};
'''
if text.count(old) != 1:
    raise SystemExit("world_projection import marker missing")
text = text.replace(old, new, 1)

old = '''pub enum EvidenceQueryRequest {
    Selections,
    Neighborhood { root: String, max_depth: usize },
    ShortestPath { from: String, to: String },
}
'''
new = '''pub enum EvidenceQueryRequest {
    Selections,
    Describe { selection: String },
    Neighborhood { root: String, max_depth: usize },
    ShortestPath { from: String, to: String },
}
'''
if text.count(old) != 1:
    raise SystemExit("EvidenceQueryRequest marker missing")
text = text.replace(old, new, 1)

old = '''pub enum EvidenceQueryResponse {
    Selections { value: EvidenceSelectionIndex },
    Neighborhood { value: EvidenceNeighborhoodResult },
    ShortestPath { value: EvidencePathResult },
}
'''
new = '''pub enum EvidenceQueryResponse {
    Selections { value: EvidenceSelectionIndex },
    Description { value: EvidenceSelectionDetail },
    Neighborhood { value: EvidenceNeighborhoodResult },
    ShortestPath { value: EvidencePathResult },
}
'''
if text.count(old) != 1:
    raise SystemExit("EvidenceQueryResponse marker missing")
text = text.replace(old, new, 1)

marker = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceComparisonRequest {
'''
dtos = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

'''
if text.count(marker) != 1:
    raise SystemExit("detail DTO insertion marker missing")
text = text.replace(marker, dtos + marker, 1)

old = '''        EvidenceQueryRequest::Selections => Ok(EvidenceQueryResponse::Selections {
            value: query_selections(snapshot),
        }),
        EvidenceQueryRequest::Neighborhood { root, max_depth } => {
'''
new = '''        EvidenceQueryRequest::Selections => Ok(EvidenceQueryResponse::Selections {
            value: query_selections(snapshot),
        }),
        EvidenceQueryRequest::Describe { selection } => {
            let selection = parse_selection_key(selection)?;
            query_description(snapshot, selection)
                .map(|value| EvidenceQueryResponse::Description { value })
        }
        EvidenceQueryRequest::Neighborhood { root, max_depth } => {
'''
if text.count(old) != 1:
    raise SystemExit("execute_query describe insertion marker missing")
text = text.replace(old, new, 1)

marker = '''pub fn query_neighborhood(
'''
functions = '''pub fn query_description(
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

'''
if text.count(marker) != 1:
    raise SystemExit("query_description insertion marker missing")
text = text.replace(marker, functions + marker, 1)

# Tests need the relation identity support-section constant.
old = '''        ENTITY_HISTORY_SECTION, RELATION_ENDPOINTS_SECTION, RELATION_HISTORY_SECTION,
'''
new = '''        ENTITY_HISTORY_SECTION, RELATION_ENDPOINTS_SECTION, RELATION_HISTORY_SECTION,
        RELATION_IDENTITY_SECTION,
'''
if text.count(old) != 1:
    raise SystemExit("test import marker missing")
text = text.replace(old, new, 1)

marker = '''    #[test]
    fn serialized_query_requests_execute_without_callers_parsing_selection_ids() {
'''
tests = '''    #[test]
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
            assert!(!value.sections.iter().any(|section| section.title == internal));
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

'''
if text.count(marker) != 1:
    raise SystemExit("describe test insertion marker missing")
text = text.replace(marker, tests + marker, 1)
query.write_text(text)

integration = Path("crates/world-cli/tests/machine_query_transport.rs")
test_text = integration.read_text()
marker = '''#[test]
fn stdin_selection_discovery_emits_a_versioned_typed_index() {
'''
test = '''#[test]
fn stdin_selection_describe_emits_a_versioned_typed_description() {
    let (path, root) = world_fixture();
    let request = serde_json::to_string(&EvidenceQueryRequest::Describe {
        selection: root.clone(),
    })
    .unwrap();

    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some(&request),
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_protocol(&envelope);
    assert_eq!(envelope["status"], "ok");
    let response: EvidenceQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceQueryResponse::Description { value } = response else {
        panic!("expected description response")
    };
    assert_eq!(value.selection, root);
    assert!(!value.title.is_empty());

    let _ = fs::remove_file(path);
}

'''
if test_text.count(marker) != 1:
    raise SystemExit("integration describe test insertion marker missing")
test_text = test_text.replace(marker, test + marker, 1)
integration.write_text(test_text)
