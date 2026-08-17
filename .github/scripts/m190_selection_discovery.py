from pathlib import Path

query = Path("crates/world-query/src/lib.rs")
text = query.read_text()

old = '''pub enum EvidenceQueryRequest {
    Neighborhood { root: String, max_depth: usize },
    ShortestPath { from: String, to: String },
}
'''
new = '''pub enum EvidenceQueryRequest {
    Selections,
    Neighborhood { root: String, max_depth: usize },
    ShortestPath { from: String, to: String },
}
'''
if text.count(old) != 1:
    raise SystemExit("EvidenceQueryRequest marker missing")
text = text.replace(old, new, 1)

old = '''pub enum EvidenceQueryResponse {
    Neighborhood { value: EvidenceNeighborhoodResult },
    ShortestPath { value: EvidencePathResult },
}
'''
new = '''pub enum EvidenceQueryResponse {
    Selections { value: EvidenceSelectionIndex },
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

'''
if text.count(marker) != 1:
    raise SystemExit("DTO insertion marker missing")
text = text.replace(marker, dtos + marker, 1)

old = '''    match request {
        EvidenceQueryRequest::Neighborhood { root, max_depth } => {
'''
new = '''    match request {
        EvidenceQueryRequest::Selections => Ok(EvidenceQueryResponse::Selections {
            value: query_selections(snapshot),
        }),
        EvidenceQueryRequest::Neighborhood { root, max_depth } => {
'''
if text.count(old) != 1:
    raise SystemExit("execute_query marker missing")
text = text.replace(old, new, 1)

marker = '''pub fn query_neighborhood(
'''
function = '''pub fn query_selections(snapshot: &ProjectionSnapshot) -> EvidenceSelectionIndex {
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

'''
if text.count(marker) != 1:
    raise SystemExit("query_selections insertion marker missing")
text = text.replace(marker, function + marker, 1)

# Insert discovery contract tests before the existing serialization test.
marker = '''    #[test]
    fn serialized_query_requests_execute_without_callers_parsing_selection_ids() {
'''
tests = '''    #[test]
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

'''
if text.count(marker) != 1:
    raise SystemExit("discovery tests insertion marker missing")
text = text.replace(marker, tests + marker, 1)
query.write_text(text)

integration = Path("crates/world-cli/tests/machine_query_transport.rs")
test_text = integration.read_text()
marker = '''#[test]
fn stdin_neighborhood_and_shortest_path_queries_emit_typed_json() {
'''
test = '''#[test]
fn stdin_selection_discovery_emits_a_versioned_typed_index() {
    let (path, _) = world_fixture();
    let request = serde_json::to_string(&EvidenceQueryRequest::Selections).unwrap();

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
    let EvidenceQueryResponse::Selections { value } = response else {
        panic!("expected selections response")
    };
    assert!(!value.selections.is_empty());
    assert!(value.selections.iter().all(|selection| {
        selection.selection.starts_with("entity-")
            || selection.selection.starts_with("relation-")
            || selection.selection.starts_with("event-")
    }));

    let _ = fs::remove_file(path);
}

'''
if test_text.count(marker) != 1:
    raise SystemExit("integration test insertion marker missing")
test_text = test_text.replace(marker, test + marker, 1)
integration.write_text(test_text)
