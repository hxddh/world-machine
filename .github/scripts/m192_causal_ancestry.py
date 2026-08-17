from pathlib import Path

query = Path("crates/world-query/src/lib.rs")
text = query.read_text()

old = '''use world_projection::{
    InspectorProjection, ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge,
};
'''
new = '''use world_projection::{
    InspectorProjection, ProjectionSnapshot, RelationEndpointRole, SelectionId, StateEvidenceEdge,
    TimelineItem,
};
'''
if text.count(old) != 1:
    raise SystemExit("world_projection import marker missing")
text = text.replace(old, new, 1)

old = '''pub enum EvidenceQueryRequest {
    Selections,
    Describe { selection: String },
    Neighborhood { root: String, max_depth: usize },
    ShortestPath { from: String, to: String },
}
'''
new = '''pub enum EvidenceQueryRequest {
    Selections,
    Describe { selection: String },
    Why { event: String },
    Neighborhood { root: String, max_depth: usize },
    ShortestPath { from: String, to: String },
}
'''
if text.count(old) != 1:
    raise SystemExit("EvidenceQueryRequest marker missing")
text = text.replace(old, new, 1)

old = '''pub enum EvidenceQueryResponse {
    Selections { value: EvidenceSelectionIndex },
    Description { value: EvidenceSelectionDetail },
    Neighborhood { value: EvidenceNeighborhoodResult },
    ShortestPath { value: EvidencePathResult },
}
'''
new = '''pub enum EvidenceQueryResponse {
    Selections { value: EvidenceSelectionIndex },
    Description { value: EvidenceSelectionDetail },
    Why { value: EvidenceWhyResult },
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
pub struct EvidenceWhyResult {
    pub event: String,
    pub nodes: Vec<EvidenceWhyNode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceWhyNode {
    pub event: String,
    pub depth: usize,
    pub world_time: u64,
    pub title: String,
    pub subtitle: String,
    pub caused_by: Vec<String>,
}

'''
if text.count(marker) != 1:
    raise SystemExit("why DTO insertion marker missing")
text = text.replace(marker, dtos + marker, 1)

old = '''pub enum QueryError {
    InvalidSelectionKey(String),
    SelectionNotVisible(String),
    NoEvidencePath { from: String, to: String },
    SelectionNotVisibleInEitherWorld(String),
}
'''
new = '''pub enum QueryError {
    InvalidSelectionKey(String),
    SelectionKindMismatch {
        selection: String,
        expected: EvidenceSelectionKind,
    },
    SelectionNotVisible(String),
    NoEvidencePath { from: String, to: String },
    SelectionNotVisibleInEitherWorld(String),
}
'''
if text.count(old) != 1:
    raise SystemExit("QueryError marker missing")
text = text.replace(old, new, 1)

old = '''            Self::SelectionNotVisible(selection) => {
                write!(f, "selection is not visible: {selection}")
            }
'''
new = '''            Self::SelectionKindMismatch {
                selection,
                expected,
            } => write!(
                f,
                "selection kind mismatch: {selection} (expected {})",
                selection_kind_name(*expected)
            ),
            Self::SelectionNotVisible(selection) => {
                write!(f, "selection is not visible: {selection}")
            }
'''
if text.count(old) != 1:
    raise SystemExit("QueryError display marker missing")
text = text.replace(old, new, 1)

old = '''        EvidenceQueryRequest::Describe { selection } => {
            let selection = parse_selection_key(selection)?;
            query_description(snapshot, selection)
                .map(|value| EvidenceQueryResponse::Description { value })
        }
        EvidenceQueryRequest::Neighborhood { root, max_depth } => {
'''
new = '''        EvidenceQueryRequest::Describe { selection } => {
            let selection = parse_selection_key(selection)?;
            query_description(snapshot, selection)
                .map(|value| EvidenceQueryResponse::Description { value })
        }
        EvidenceQueryRequest::Why { event } => {
            let event = parse_selection_key(event)?;
            query_why(snapshot, event).map(|value| EvidenceQueryResponse::Why { value })
        }
        EvidenceQueryRequest::Neighborhood { root, max_depth } => {
'''
if text.count(old) != 1:
    raise SystemExit("execute_query why insertion marker missing")
text = text.replace(old, new, 1)

marker = '''pub fn query_neighborhood(
'''
functions = '''pub fn query_why(
    snapshot: &ProjectionSnapshot,
    event: SelectionId,
) -> Result<EvidenceWhyResult, QueryError> {
    if !matches!(event, SelectionId::Event(_)) {
        return Err(QueryError::SelectionKindMismatch {
            selection: event.stable_key(),
            expected: EvidenceSelectionKind::Event,
        });
    }

    let visible = snapshot
        .timeline
        .items
        .iter()
        .filter(|item| matches!(item.id, SelectionId::Event(_)))
        .map(|item| (item.id, item))
        .collect::<std::collections::BTreeMap<_, _>>();
    if !visible.contains_key(&event) {
        return Err(QueryError::SelectionNotVisible(event.stable_key()));
    }

    let mut visited = std::collections::BTreeSet::new();
    let mut nodes = Vec::new();
    visit_visible_causes(event, 0, &visible, &mut visited, &mut nodes);

    Ok(EvidenceWhyResult {
        event: event.stable_key(),
        nodes,
    })
}

fn visit_visible_causes(
    event: SelectionId,
    depth: usize,
    visible: &std::collections::BTreeMap<SelectionId, &TimelineItem>,
    visited: &mut std::collections::BTreeSet<SelectionId>,
    nodes: &mut Vec<EvidenceWhyNode>,
) {
    if !visited.insert(event) {
        return;
    }
    let Some(item) = visible.get(&event).copied() else {
        return;
    };

    let caused_by = item
        .caused_by
        .iter()
        .map(|cause| SelectionId::Event(*cause))
        .filter(|cause| visible.contains_key(cause))
        .collect::<Vec<_>>();
    nodes.push(EvidenceWhyNode {
        event: event.stable_key(),
        depth,
        world_time: item.world_time,
        title: item.title.clone(),
        subtitle: item.subtitle.clone(),
        caused_by: caused_by.iter().map(|cause| cause.stable_key()).collect(),
    });

    for cause in caused_by {
        visit_visible_causes(cause, depth + 1, visible, visited, nodes);
    }
}

'''
if text.count(marker) != 1:
    raise SystemExit("query_why insertion marker missing")
text = text.replace(marker, functions + marker, 1)

old = '''fn selection_kind(selection: SelectionId) -> EvidenceSelectionKind {
    match selection {
        SelectionId::Entity(_) => EvidenceSelectionKind::Entity,
        SelectionId::Relation(_) => EvidenceSelectionKind::Relation,
        SelectionId::Event(_) => EvidenceSelectionKind::Event,
    }
}
'''
new = '''fn selection_kind(selection: SelectionId) -> EvidenceSelectionKind {
    match selection {
        SelectionId::Entity(_) => EvidenceSelectionKind::Entity,
        SelectionId::Relation(_) => EvidenceSelectionKind::Relation,
        SelectionId::Event(_) => EvidenceSelectionKind::Event,
    }
}

fn selection_kind_name(kind: EvidenceSelectionKind) -> &'static str {
    match kind {
        EvidenceSelectionKind::Entity => "entity",
        EvidenceSelectionKind::Relation => "relation",
        EvidenceSelectionKind::Event => "event",
    }
}
'''
if text.count(old) != 1:
    raise SystemExit("selection_kind marker missing")
text = text.replace(old, new, 1)

# Add the new stable error shape to the pinned QueryError serde contract.
marker = '''            (
                QueryError::SelectionNotVisible("entity-99".into()),
                r#"{"error":"selection-not-visible","details":"entity-99"}"#,
            ),
'''
addition = '''            (
                QueryError::SelectionKindMismatch {
                    selection: "entity-1".into(),
                    expected: EvidenceSelectionKind::Event,
                },
                r#"{"error":"selection-kind-mismatch","details":{"selection":"entity-1","expected":"event"}}"#,
            ),
'''
if text.count(marker) != 1:
    raise SystemExit("QueryError serde test marker missing")
text = text.replace(marker, addition + marker, 1)

marker = '''    #[test]
    fn serialized_query_requests_execute_without_callers_parsing_selection_ids() {
'''
tests = '''    #[test]
    fn why_query_walks_visible_persisted_causes_in_deterministic_order() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        snapshot.timeline.items = vec![
            TimelineItem {
                id: SelectionId::Event(EventId::new(3)),
                world_time: 3,
                title: "Final effect".into(),
                subtitle: "Final".into(),
                caused_by: vec![EventId::new(2)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(2)),
                world_time: 2,
                title: "Intermediate effect".into(),
                subtitle: "Middle".into(),
                caused_by: vec![EventId::new(1)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(1)),
                world_time: 1,
                title: "Root cause".into(),
                subtitle: "Root".into(),
                caused_by: Vec::new(),
            },
        ];

        let request: EvidenceQueryRequest =
            serde_json::from_str(r#"{"query":"why","event":"event-3"}"#).unwrap();
        let response = execute_query(&snapshot, &request).unwrap();
        let EvidenceQueryResponse::Why { value } = response else {
            panic!("expected why response")
        };
        assert_eq!(value.event, "event-3");
        assert_eq!(
            value
                .nodes
                .iter()
                .map(|node| (node.event.as_str(), node.depth))
                .collect::<Vec<_>>(),
            vec![("event-3", 0), ("event-2", 1), ("event-1", 2)]
        );
        assert_eq!(value.nodes[0].caused_by, vec!["event-2"]);
        assert_eq!(value.nodes[1].caused_by, vec!["event-1"]);

        let json = serde_json::to_string(&EvidenceQueryResponse::Why {
            value: value.clone(),
        })
        .unwrap();
        let restored: EvidenceQueryResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored,
            EvidenceQueryResponse::Why { value }
        );
    }

    #[test]
    fn why_query_filters_hidden_causes_and_cycle_protects() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        snapshot.timeline.items = vec![
            TimelineItem {
                id: SelectionId::Event(EventId::new(3)),
                world_time: 3,
                title: "Final".into(),
                subtitle: "Visible".into(),
                caused_by: vec![EventId::new(2), EventId::new(99)],
            },
            TimelineItem {
                id: SelectionId::Event(EventId::new(2)),
                world_time: 2,
                title: "Cycle".into(),
                subtitle: "Visible".into(),
                caused_by: vec![EventId::new(3)],
            },
        ];

        let value = query_why(&snapshot, SelectionId::Event(EventId::new(3))).unwrap();
        assert_eq!(value.nodes.len(), 2);
        assert_eq!(value.nodes[0].caused_by, vec!["event-2"]);
        assert_eq!(value.nodes[1].caused_by, vec!["event-3"]);
    }

    #[test]
    fn why_query_enforces_event_kind_and_timeline_visibility() {
        let mut snapshot = snapshot(EntityId::new(1), EntityId::new(3));
        assert_eq!(
            execute_query(
                &snapshot,
                &EvidenceQueryRequest::Why {
                    event: "entity-1".into(),
                },
            ),
            Err(QueryError::SelectionKindMismatch {
                selection: "entity-1".into(),
                expected: EvidenceSelectionKind::Event,
            })
        );
        assert_eq!(
            execute_query(
                &snapshot,
                &EvidenceQueryRequest::Why {
                    event: "event-07".into(),
                },
            ),
            Err(QueryError::InvalidSelectionKey("event-07".into()))
        );

        let hidden = SelectionId::Event(EventId::new(10));
        snapshot.inspectors.insert(
            hidden,
            InspectorProjection {
                selection: hidden,
                title: "Inspector only".into(),
                subtitle: "Hidden".into(),
                sections: Vec::new(),
            },
        );
        assert_eq!(
            query_why(&snapshot, hidden),
            Err(QueryError::SelectionNotVisible("event-10".into()))
        );
    }

'''
if text.count(marker) != 1:
    raise SystemExit("why tests insertion marker missing")
text = text.replace(marker, tests + marker, 1)
query.write_text(text)

integration = Path("crates/world-cli/tests/machine_query_transport.rs")
test_text = integration.read_text()
marker = '''#[test]
fn stdin_selection_describe_emits_a_versioned_typed_description() {
'''
test = '''#[test]
fn stdin_why_query_emits_a_versioned_typed_causal_history() {
    let (path, event) = world_fixture_with_event();
    let request = serde_json::to_string(&EvidenceQueryRequest::Why {
        event: event.clone(),
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
    let EvidenceQueryResponse::Why { value } = response else {
        panic!("expected why response")
    };
    assert_eq!(value.event, event);
    assert!(!value.nodes.is_empty());
    assert_eq!(value.nodes[0].event, value.event);
    assert_eq!(value.nodes[0].depth, 0);

    let _ = fs::remove_file(path);
}

'''
if test_text.count(marker) != 1:
    raise SystemExit("CLI why test insertion marker missing")
test_text = test_text.replace(marker, test + marker, 1)

marker = '''fn world_fixture() -> (PathBuf, String) {
'''
helper = '''fn world_fixture_with_event() -> (PathBuf, String) {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        let Some(event) = snapshot.timeline.items.first().map(|item| item.id) else {
            continue;
        };
        if !event.stable_key().starts_with("event-") {
            continue;
        }
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path();
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
        return (path, event.stable_key());
    }
    panic!("a built-in Pack should expose a visible timeline event")
}

'''
if test_text.count(marker) != 1:
    raise SystemExit("world fixture insertion marker missing")
test_text = test_text.replace(marker, helper + marker, 1)
integration.write_text(test_text)
