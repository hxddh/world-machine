from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 match, found {count}")
    return text.replace(old, new, 1)


lib = Path("crates/world-query/src/lib.rs")
text = lib.read_text()

old_request = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceComparisonRequest {
    pub root: String,
    pub max_depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceNeighborhoodResult {'''
new_request = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceComparisonRequest {
    pub root: String,
    pub max_depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EvidenceComparisonQueryRequest {
    Causal(EvidenceCausalComparisonRequest),
    Legacy(EvidenceComparisonRequest),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "kebab-case")]
pub enum EvidenceCausalComparisonRequest {
    CausalNeighborhood {
        root: String,
        upstream_depth: usize,
        downstream_depth: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceNeighborhoodResult {'''
text = replace_once(text, old_request, new_request, "comparison request contract")

old_comparison_result = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceComparisonResult {
    pub root: String,
    pub max_depth: usize,
    pub identical: bool,
    pub nodes: Vec<EvidenceNodeDifference>,
    pub left_only_edges: Vec<EvidenceEdge>,
    pub right_only_edges: Vec<EvidenceEdge>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceNodeDifference {'''
new_comparison_result = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceComparisonResult {
    pub root: String,
    pub max_depth: usize,
    pub identical: bool,
    pub nodes: Vec<EvidenceNodeDifference>,
    pub left_only_edges: Vec<EvidenceEdge>,
    pub right_only_edges: Vec<EvidenceEdge>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EvidenceComparisonQueryResponse {
    Causal(EvidenceCausalComparisonResponse),
    Legacy(EvidenceComparisonResult),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum EvidenceCausalComparisonResponse {
    CausalNeighborhood {
        value: EvidenceCausalNeighborhoodComparisonResult,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNeighborhoodComparisonResult {
    pub root: String,
    pub upstream_depth: usize,
    pub downstream_depth: usize,
    pub identical: bool,
    pub nodes: Vec<EvidenceCausalNodeDifference>,
    pub left_only_edges: Vec<EvidenceCausalEdge>,
    pub right_only_edges: Vec<EvidenceCausalEdge>,
    pub left_upstream_frontier: Vec<String>,
    pub right_upstream_frontier: Vec<String>,
    pub left_downstream_frontier: Vec<String>,
    pub right_downstream_frontier: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNodeDifference {
    pub event: String,
    pub kind: Difference,
    pub left: Option<EvidenceCausalNodePosition>,
    pub right: Option<EvidenceCausalNodePosition>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNodePosition {
    pub is_root: bool,
    pub upstream_depth: Option<usize>,
    pub downstream_depth: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceNodeDifference {'''
text = replace_once(text, old_comparison_result, new_comparison_result, "comparison response contract")

old_execute = '''pub fn execute_comparison_query(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request: &EvidenceComparisonRequest,
) -> Result<EvidenceComparisonResult, QueryError> {
    let root = parse_selection_key(&request.root)?;
    query_neighborhood_comparison(left, right, root, request.max_depth)
}
'''
new_execute = '''pub fn execute_comparison_query(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request: &EvidenceComparisonRequest,
) -> Result<EvidenceComparisonResult, QueryError> {
    let root = parse_selection_key(&request.root)?;
    query_neighborhood_comparison(left, right, root, request.max_depth)
}

pub fn execute_comparison_query_request(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request: &EvidenceComparisonQueryRequest,
) -> Result<EvidenceComparisonQueryResponse, QueryError> {
    match request {
        EvidenceComparisonQueryRequest::Legacy(request) => execute_comparison_query(left, right, request)
            .map(EvidenceComparisonQueryResponse::Legacy),
        EvidenceComparisonQueryRequest::Causal(
            EvidenceCausalComparisonRequest::CausalNeighborhood {
                root,
                upstream_depth,
                downstream_depth,
            },
        ) => {
            let root = parse_selection_key(root)?;
            query_causal_neighborhood_comparison(
                left,
                right,
                root,
                *upstream_depth,
                *downstream_depth,
            )
            .map(|value| {
                EvidenceComparisonQueryResponse::Causal(
                    EvidenceCausalComparisonResponse::CausalNeighborhood { value },
                )
            })
        }
    }
}
'''
text = replace_once(text, old_execute, new_execute, "comparison query executor")

old_query_comparison = '''pub fn query_neighborhood_comparison(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    root: SelectionId,
    max_depth: usize,
) -> Result<EvidenceComparisonResult, QueryError> {'''

causal_compare = '''pub fn query_causal_neighborhood_comparison(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    root: SelectionId,
    upstream_depth: usize,
    downstream_depth: usize,
) -> Result<EvidenceCausalNeighborhoodComparisonResult, QueryError> {
    if !matches!(root, SelectionId::Event(_)) {
        return Err(QueryError::SelectionKindMismatch {
            selection: root.stable_key(),
            expected: EvidenceSelectionKind::Event,
        });
    }

    let left_graph = VisibleCausalGraph::new(left);
    let right_graph = VisibleCausalGraph::new(right);
    let left_visible = left_graph.events.contains_key(&root);
    let right_visible = right_graph.events.contains_key(&root);
    if !left_visible && !right_visible {
        return Err(QueryError::SelectionNotVisibleInEitherWorld(root.stable_key()));
    }

    let left_neighborhood = left_visible.then(|| {
        query_causal_neighborhood(left, root, upstream_depth, downstream_depth)
            .expect("visible causal comparison root must remain queryable")
    });
    let right_neighborhood = right_visible.then(|| {
        query_causal_neighborhood(right, root, upstream_depth, downstream_depth)
            .expect("visible causal comparison root must remain queryable")
    });

    let left_positions = left_neighborhood
        .as_ref()
        .map(causal_node_positions)
        .unwrap_or_default();
    let right_positions = right_neighborhood
        .as_ref()
        .map(causal_node_positions)
        .unwrap_or_default();
    let selections = left_positions
        .keys()
        .chain(right_positions.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut nodes = Vec::new();
    for event in selections {
        let left_position = left_positions.get(&event).cloned();
        let right_position = right_positions.get(&event).cloned();
        let kind = match (&left_position, &right_position) {
            (Some(left), Some(right)) if left == right => continue,
            (Some(_), Some(_)) => Difference::Changed,
            (Some(_), None) => Difference::LeftOnly,
            (None, Some(_)) => Difference::RightOnly,
            (None, None) => unreachable!("comparison node must exist on at least one side"),
        };
        nodes.push(EvidenceCausalNodeDifference {
            event: event.stable_key(),
            kind,
            left: left_position,
            right: right_position,
        });
    }

    let left_edges = left_neighborhood
        .as_ref()
        .map(|value| value.edges.iter().cloned().collect::<std::collections::BTreeSet<_>>())
        .unwrap_or_default();
    let right_edges = right_neighborhood
        .as_ref()
        .map(|value| value.edges.iter().cloned().collect::<std::collections::BTreeSet<_>>())
        .unwrap_or_default();
    let left_only_edges = left_edges.difference(&right_edges).cloned().collect::<Vec<_>>();
    let right_only_edges = right_edges.difference(&left_edges).cloned().collect::<Vec<_>>();

    let left_upstream_frontier = canonical_causal_frontier(
        left_neighborhood
            .as_ref()
            .map(|value| value.upstream_frontier.as_slice())
            .unwrap_or(&[]),
    );
    let right_upstream_frontier = canonical_causal_frontier(
        right_neighborhood
            .as_ref()
            .map(|value| value.upstream_frontier.as_slice())
            .unwrap_or(&[]),
    );
    let left_downstream_frontier = canonical_causal_frontier(
        left_neighborhood
            .as_ref()
            .map(|value| value.downstream_frontier.as_slice())
            .unwrap_or(&[]),
    );
    let right_downstream_frontier = canonical_causal_frontier(
        right_neighborhood
            .as_ref()
            .map(|value| value.downstream_frontier.as_slice())
            .unwrap_or(&[]),
    );

    let identical = nodes.is_empty()
        && left_only_edges.is_empty()
        && right_only_edges.is_empty()
        && left_upstream_frontier == right_upstream_frontier
        && left_downstream_frontier == right_downstream_frontier;

    Ok(EvidenceCausalNeighborhoodComparisonResult {
        root: root.stable_key(),
        upstream_depth,
        downstream_depth,
        identical,
        nodes,
        left_only_edges,
        right_only_edges,
        left_upstream_frontier,
        right_upstream_frontier,
        left_downstream_frontier,
        right_downstream_frontier,
    })
}

fn causal_node_positions(
    neighborhood: &EvidenceCausalNeighborhoodResult,
) -> std::collections::BTreeMap<SelectionId, EvidenceCausalNodePosition> {
    let mut positions = std::collections::BTreeMap::new();
    let root = parse_selection_key(&neighborhood.root.event)
        .expect("causal neighborhood root must have a stable selection key");
    positions.insert(
        root,
        EvidenceCausalNodePosition {
            is_root: true,
            upstream_depth: None,
            downstream_depth: None,
        },
    );

    for node in &neighborhood.upstream {
        let event = parse_selection_key(&node.event)
            .expect("causal neighborhood node must have a stable selection key");
        positions
            .entry(event)
            .or_insert(EvidenceCausalNodePosition {
                is_root: false,
                upstream_depth: None,
                downstream_depth: None,
            })
            .upstream_depth = Some(node.depth);
    }
    for node in &neighborhood.downstream {
        let event = parse_selection_key(&node.event)
            .expect("causal neighborhood node must have a stable selection key");
        positions
            .entry(event)
            .or_insert(EvidenceCausalNodePosition {
                is_root: false,
                upstream_depth: None,
                downstream_depth: None,
            })
            .downstream_depth = Some(node.depth);
    }
    positions
}

fn canonical_causal_frontier(frontier: &[String]) -> Vec<String> {
    frontier
        .iter()
        .map(|event| {
            parse_selection_key(event)
                .expect("causal frontier must have a stable selection key")
        })
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .map(|event| event.stable_key())
        .collect()
}

'''
text = replace_once(text, old_query_comparison, causal_compare + old_query_comparison, "causal comparison implementation")
lib.write_text(text)

Path("crates/world-query/tests/causal_neighborhood_compare.rs").write_text(r'''use serde_json::json;
use world_core::{EntityId, EventId};
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceComparisonQueryRequest,
    EvidenceComparisonQueryResponse, EvidenceComparisonRequest, EvidenceComparisonResult,
    EvidenceSelectionKind, QueryError,
};

fn event(id: u64, world_time: u64, caused_by: &[u64]) -> TimelineItem {
    TimelineItem {
        id: SelectionId::Event(EventId::new(id)),
        world_time,
        title: format!("Event {id}"),
        subtitle: format!("world time {world_time}"),
        caused_by: caused_by.iter().copied().map(EventId::new).collect(),
    }
}

fn snapshot(items: Vec<TimelineItem>) -> ProjectionSnapshot {
    ProjectionSnapshot {
        timeline: TimelineProjection { items },
        ..ProjectionSnapshot::default()
    }
}

fn causal_request(root: &str, upstream_depth: usize, downstream_depth: usize) -> EvidenceComparisonQueryRequest {
    EvidenceComparisonQueryRequest::Causal(EvidenceCausalComparisonRequest::CausalNeighborhood {
        root: root.into(),
        upstream_depth,
        downstream_depth,
    })
}

fn compare(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    root: &str,
    upstream_depth: usize,
    downstream_depth: usize,
) -> world_query::EvidenceCausalNeighborhoodComparisonResult {
    let response = execute_comparison_query_request(
        left,
        right,
        &causal_request(root, upstream_depth, downstream_depth),
    )
    .unwrap();
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value },
    ) = response
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    value
}

#[test]
fn machine_comparison_wire_preserves_legacy_shape_and_adds_tagged_causal_shape() {
    let legacy_json = json!({"root":"entity-1","max_depth":2});
    let legacy: EvidenceComparisonQueryRequest = serde_json::from_value(legacy_json.clone()).unwrap();
    assert_eq!(
        legacy,
        EvidenceComparisonQueryRequest::Legacy(EvidenceComparisonRequest {
            root: "entity-1".into(),
            max_depth: 2,
        })
    );
    assert_eq!(serde_json::to_value(&legacy).unwrap(), legacy_json);

    let causal_json = json!({
        "query":"causal-neighborhood",
        "root":"event-3",
        "upstream_depth":1,
        "downstream_depth":2
    });
    let causal: EvidenceComparisonQueryRequest = serde_json::from_value(causal_json.clone()).unwrap();
    assert_eq!(causal, causal_request("event-3", 1, 2));
    assert_eq!(serde_json::to_value(&causal).unwrap(), causal_json);

    let legacy_response = EvidenceComparisonQueryResponse::Legacy(EvidenceComparisonResult {
        root: "entity-1".into(),
        max_depth: 0,
        identical: true,
        nodes: vec![],
        left_only_edges: vec![],
        right_only_edges: vec![],
    });
    assert_eq!(
        serde_json::to_value(&legacy_response).unwrap(),
        json!({
            "root":"entity-1",
            "max_depth":0,
            "identical":true,
            "nodes":[],
            "left_only_edges":[],
            "right_only_edges":[]
        })
    );
}

#[test]
fn causal_comparison_reports_bidirectional_node_and_edge_divergence() {
    let left = snapshot(vec![
        event(4, 4, &[3]),
        event(3, 3, &[2]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(5, 5, &[3]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);

    let value = compare(&left, &right, "event-3", 1, 1);
    assert!(!value.identical);
    assert_eq!(
        value
            .nodes
            .iter()
            .map(|node| (node.event.as_str(), node.kind))
            .collect::<Vec<_>>(),
        vec![
            ("event-1", world_query::Difference::RightOnly),
            ("event-2", world_query::Difference::LeftOnly),
            ("event-4", world_query::Difference::LeftOnly),
            ("event-5", world_query::Difference::RightOnly),
        ]
    );
    assert!(value.left_only_edges.iter().any(|edge| edge.cause == "event-2" && edge.effect == "event-3"));
    assert!(value.left_only_edges.iter().any(|edge| edge.cause == "event-3" && edge.effect == "event-4"));
    assert!(value.right_only_edges.iter().any(|edge| edge.cause == "event-1" && edge.effect == "event-3"));
    assert!(value.right_only_edges.iter().any(|edge| edge.cause == "event-3" && edge.effect == "event-5"));
}

#[test]
fn causal_comparison_marks_changed_directional_depths_and_cycle_positions() {
    let left = snapshot(vec![
        event(3, 3, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(3, 3, &[1]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let value = compare(&left, &right, "event-3", 2, 0);
    let changed = value
        .nodes
        .iter()
        .find(|node| node.event == "event-1")
        .expect("event-1 should change causal depth");
    assert_eq!(changed.kind, world_query::Difference::Changed);
    assert_eq!(changed.left.as_ref().unwrap().upstream_depth, Some(2));
    assert_eq!(changed.right.as_ref().unwrap().upstream_depth, Some(1));

    let cycle = snapshot(vec![event(2, 2, &[1]), event(1, 1, &[2])]);
    let one_way = snapshot(vec![event(2, 2, &[1]), event(1, 1, &[])]);
    let value = compare(&cycle, &one_way, "event-1", 1, 1);
    let event_two = value
        .nodes
        .iter()
        .find(|node| node.event == "event-2")
        .expect("event-2 should have a changed directional position");
    assert_eq!(event_two.kind, world_query::Difference::Changed);
    assert_eq!(event_two.left.as_ref().unwrap().upstream_depth, Some(1));
    assert_eq!(event_two.left.as_ref().unwrap().downstream_depth, Some(1));
    assert_eq!(event_two.right.as_ref().unwrap().upstream_depth, None);
    assert_eq!(event_two.right.as_ref().unwrap().downstream_depth, Some(1));
}

#[test]
fn causal_comparison_ignores_hidden_references_but_compares_frontier_membership() {
    let hidden = snapshot(vec![event(3, 3, &[99])]);
    let plain = snapshot(vec![event(3, 3, &[])]);
    let value = compare(&hidden, &plain, "event-3", 2, 2);
    assert!(value.identical);
    assert!(value.nodes.is_empty());
    assert!(value.left_only_edges.is_empty());
    assert!(value.right_only_edges.is_empty());

    let deeper = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);
    let shallow = snapshot(vec![event(3, 3, &[])]);
    let value = compare(&deeper, &shallow, "event-3", 0, 0);
    assert!(!value.identical);
    assert!(value.nodes.is_empty());
    assert_eq!(value.left_upstream_frontier, vec!["event-3"]);
    assert!(value.right_upstream_frontier.is_empty());
}

#[test]
fn causal_comparison_allows_one_sided_root_and_enforces_event_visibility_contract() {
    let left = snapshot(vec![event(1, 1, &[])]);
    let right = snapshot(vec![event(2, 2, &[])]);
    let value = compare(&left, &right, "event-1", 1, 1);
    assert!(!value.identical);
    let root = value
        .nodes
        .iter()
        .find(|node| node.event == "event-1")
        .expect("one-sided root should be reported");
    assert_eq!(root.kind, world_query::Difference::LeftOnly);
    assert!(root.left.as_ref().unwrap().is_root);
    assert!(root.right.is_none());

    let absent = snapshot(vec![event(7, 7, &[])]);
    assert_eq!(
        execute_comparison_query_request(&left, &absent, &causal_request("event-9", 1, 1)),
        Err(QueryError::SelectionNotVisibleInEitherWorld("event-9".into()))
    );
    assert_eq!(
        execute_comparison_query_request(
            &left,
            &right,
            &causal_request(&SelectionId::Entity(EntityId::new(1)).stable_key(), 1, 1),
        ),
        Err(QueryError::SelectionKindMismatch {
            selection: "entity-1".into(),
            expected: EvidenceSelectionKind::Event,
        })
    );
    assert_eq!(
        execute_comparison_query_request(&left, &right, &causal_request("event-07", 1, 1)),
        Err(QueryError::InvalidSelectionKey("event-07".into()))
    );
}

#[test]
fn identical_causal_comparison_round_trips_as_tagged_response() {
    let snapshot = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let response = execute_comparison_query_request(
        &snapshot,
        &snapshot,
        &causal_request("event-2", 1, 1),
    )
    .unwrap();
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["result"], "causal-neighborhood");
    assert_eq!(json["value"]["identical"], true);
    let restored: EvidenceComparisonQueryResponse = serde_json::from_value(json).unwrap();
    assert_eq!(restored, response);
}
''')

cli = Path("crates/world-cli/src/main.rs")
cli_text = cli.read_text()
old_import = '''use world_query::{
    execute_comparison_query, execute_query, Difference, EvidenceComparisonRequest,
    EvidenceComparisonResult, EvidenceEdge, EvidenceQueryRequest, EvidenceQueryResponse,
};'''
new_import = '''use world_query::{
    execute_comparison_query, execute_comparison_query_request, execute_query, Difference,
    EvidenceComparisonQueryRequest, EvidenceComparisonRequest, EvidenceComparisonResult,
    EvidenceEdge, EvidenceQueryRequest, EvidenceQueryResponse,
};'''
cli_text = replace_once(cli_text, old_import, new_import, "world-cli comparison imports")
old_parser = '''    let request: EvidenceComparisonRequest = serde_json::from_str(request_json)
        .map_err(|error| CliError(format!("invalid evidence comparison query JSON: {error}")))?;
    let output = match execute_comparison_query(left, right, &request) {'''
new_parser = '''    let request: EvidenceComparisonQueryRequest = serde_json::from_str(request_json)
        .map_err(|error| CliError(format!("invalid evidence comparison query JSON: {error}")))?;
    let output = match execute_comparison_query_request(left, right, &request) {'''
cli_text = replace_once(cli_text, old_parser, new_parser, "world-cli comparison parser")
old_usage = '''evidence-compare-query  Execute an EvidenceComparisonRequest JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\
list-packs'''
new_usage = '''evidence-compare-query  Execute a legacy evidence comparison or tagged causal comparison JSON document and emit a JSON status envelope. Use - to read JSON from stdin.\\n\\
list-packs'''
cli_text = replace_once(cli_text, old_usage, new_usage, "world-cli usage")
cli.write_text(cli_text)

Path("crates/world-cli/tests/machine_query_causal_compare.rs").write_text(r'''use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_query::{
    EvidenceCausalComparisonRequest, EvidenceCausalComparisonResponse,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse,
};

#[test]
fn stdin_causal_comparison_uses_existing_versioned_compare_transport() {
    let (path, root) = world_fixture_with_event();
    let request = EvidenceComparisonQueryRequest::Causal(
        EvidenceCausalComparisonRequest::CausalNeighborhood {
            root: root.clone(),
            upstream_depth: 1,
            downstream_depth: 1,
        },
    );
    let request = serde_json::to_string(&request).unwrap();

    let output = run_query(
        &[
            "evidence-compare-query",
            path.to_str().unwrap(),
            path.to_str().unwrap(),
            "-",
        ],
        Some(&request),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["protocol"], "world-machine-evidence-query");
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["status"], "ok");

    let response: EvidenceComparisonQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value },
    ) = response
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    assert_eq!(value.root, root);
    assert!(value.identical);
    assert!(value.nodes.is_empty());
    assert!(value.left_only_edges.is_empty());
    assert!(value.right_only_edges.is_empty());

    let _ = fs::remove_file(path);
}

fn run_query(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_world-cli"))
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin should be piped")
            .write_all(input.as_bytes())
            .unwrap();
    }

    child.wait_with_output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn world_fixture_with_event() -> (PathBuf, String) {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        let Some(event) = snapshot
            .timeline
            .items
            .iter()
            .find(|item| item.id.stable_key().starts_with("event-"))
            .map(|item| item.id)
        else {
            continue;
        };
        let archive = session.archive().unwrap().unwrap();
        let path = temp_world_path();
        fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
        return (path, event.stable_key());
    }
    panic!("a built-in Pack should expose a visible timeline event")
}

fn temp_world_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("world-machine-m201-{}-{nonce}.world", std::process::id()))
}
''')

Path("NEXT_TASK.md").write_text('''# Next Coding Task — M201 Causal Neighborhood Comparison

Extend the existing two-world machine comparison transport so saved worlds and sibling futures can compare a bounded visible causal neighborhood without conflating it with the state-evidence graph.

## Current baseline

The machine causal surface is complete through M200:

- `why`, `influence`, deterministic `causal-path`;
- bounded bidirectional `causal-neighborhood` with truncation, frontiers, induced edges, and executable continuations;
- self-contained traversal edges for `why` / `influence`;
- cross-query invariant coverage proving all causal surfaces agree on one timeline-visible persisted causal graph;
- existing `evidence-compare-query` currently accepts the legacy untagged state-evidence request `{ root, max_depth }` and emits a raw `EvidenceComparisonResult` inside protocol-v1 status envelopes.

## M201 — bounded causal comparison

Preserve the legacy state-evidence comparison wire shape exactly, while extending `evidence-compare-query` with the tagged request:

`{ "query": "causal-neighborhood", "root": "event-N", "upstream_depth": U, "downstream_depth": D }`

A causal comparison is a structural comparison of the requested bounded visible causal window. Compare:

- Event membership and directional position (`is_root`, upstream depth, downstream depth);
- induced visible `cause -> effect` edges;
- canonical upstream/downstream frontier membership.

Do not compare display titles/subtitles or causal structure outside the requested window. Hidden referenced Event IDs remain invisible.

## Compatibility contract

- Introduce an untagged machine comparison request wrapper that accepts the legacy `{ root, max_depth }` shape unchanged and the new tagged causal request.
- Legacy requests must serialize and respond exactly as before; do not wrap old `EvidenceComparisonResult` in a new result tag.
- New causal responses use a tagged `result: "causal-neighborhood"` payload.
- Keep `world-machine-evidence-query` protocol version 1.
- Keep the human `evidence-compare` command and `execute_comparison_query` legacy API unchanged.

## Causal comparison semantics

- Root must be a canonical Event key.
- If the root is visible in neither world, return `SelectionNotVisibleInEitherWorld`.
- If visible in only one world, comparison succeeds and reports one-sided root/window differences.
- Node differences are typed `left-only`, `right-only`, or `changed`; `changed` means the same Event occupies a different bounded directional position/depth.
- Edge differences are set differences over induced visible causal edges.
- Frontier lists are canonicalized by typed Event identity before comparison so UI/traversal ordering cannot create false structural differences.
- `identical` means node positions, induced edges, and canonical frontier membership are all identical.

## Tests

Prove at minimum:

1. legacy request and response JSON shapes remain byte-structure compatible;
2. tagged causal request/response round-trip;
3. upstream/downstream node and edge divergence;
4. changed causal depth and cycle positions;
5. hidden references remain invisible while frontier differences remain semantic;
6. one-sided root success, neither-side error, kind mismatch, invalid stable key;
7. same-world comparison is identical;
8. a real stdin `world-cli evidence-compare-query` executes the tagged causal request through the existing v1 transport;
9. all M199/M200 causal consistency and continuation tests remain green.

## Validation

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals

Do not compare arbitrary unbounded causal graphs, display metadata, state-evidence and causal graphs in one result, raw mutation payloads, AgentRuntime perception, MCP/HTTP/WebSocket, server-side comparison state, or protocol v2.
''')
