from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


lib_path = Path("crates/world-query/src/lib.rs")
lib = lib_path.read_text()

lib = replace_once(
    lib,
    '''pub enum EvidenceCausalComparisonRequest {
    CausalNeighborhood {
        root: String,
        upstream_depth: usize,
        downstream_depth: usize,
    },
}
''',
    '''pub enum EvidenceCausalComparisonRequest {
    CausalNeighborhood {
        root: String,
        upstream_depth: usize,
        downstream_depth: usize,
    },
    FirstDivergence {
        root: String,
        direction: EvidenceCausalDirection,
        max_depth: usize,
    },
}
''',
    "request enum",
)

lib = replace_once(
    lib,
    '''pub enum EvidenceCausalComparisonResponse {
    CausalNeighborhood {
        value: EvidenceCausalNeighborhoodComparisonResult,
    },
}
''',
    '''pub enum EvidenceCausalComparisonResponse {
    CausalNeighborhood {
        value: EvidenceCausalNeighborhoodComparisonResult,
    },
    FirstDivergence {
        value: EvidenceCausalFirstDivergenceResult,
    },
}
''',
    "response enum",
)

lib = replace_once(
    lib,
    '''pub struct EvidenceCausalComparisonContinuation {
    pub event: String,
    pub direction: EvidenceCausalDirection,
    pub left_frontier: bool,
    pub right_frontier: bool,
    pub request: EvidenceComparisonQueryRequest,
}

''',
    '''pub struct EvidenceCausalComparisonContinuation {
    pub event: String,
    pub direction: EvidenceCausalDirection,
    pub left_frontier: bool,
    pub right_frontier: bool,
    pub request: EvidenceComparisonQueryRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalFirstDivergenceResult {
    pub root: String,
    pub direction: EvidenceCausalDirection,
    pub max_depth: usize,
    pub identical_within_depth: bool,
    pub divergence_depth: Option<usize>,
    pub witnesses: Vec<EvidenceCausalDivergenceWitness>,
    pub left_frontier: Vec<String>,
    pub right_frontier: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum EvidenceCausalDivergenceWitness {
    RootPresence { difference: Difference },
    Edge {
        difference: Difference,
        edge: EvidenceCausalEdge,
    },
}

''',
    "first divergence DTOs",
)

old_execute = '''        EvidenceComparisonQueryRequest::Causal(
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
'''
new_execute = '''        EvidenceComparisonQueryRequest::Causal(request) => match request {
            EvidenceCausalComparisonRequest::CausalNeighborhood {
                root,
                upstream_depth,
                downstream_depth,
            } => {
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
            EvidenceCausalComparisonRequest::FirstDivergence {
                root,
                direction,
                max_depth,
            } => {
                let root = parse_selection_key(root)?;
                query_causal_first_divergence(left, right, root, *direction, *max_depth).map(
                    |value| {
                        EvidenceComparisonQueryResponse::Causal(
                            EvidenceCausalComparisonResponse::FirstDivergence { value },
                        )
                    },
                )
            }
        },
'''
lib = replace_once(lib, old_execute, new_execute, "comparison executor")

marker = '''pub fn query_causal_neighborhood_comparison(
    left: &ProjectionSnapshot,
'''
first_divergence_impl = r'''pub fn query_causal_first_divergence(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    root: SelectionId,
    direction: EvidenceCausalDirection,
    max_depth: usize,
) -> Result<EvidenceCausalFirstDivergenceResult, QueryError> {
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
        return Err(QueryError::SelectionNotVisibleInEitherWorld(
            root.stable_key(),
        ));
    }

    let neighborhood = |snapshot: &ProjectionSnapshot| match direction {
        EvidenceCausalDirection::Upstream => {
            query_causal_neighborhood(snapshot, root, max_depth, 0)
        }
        EvidenceCausalDirection::Downstream => {
            query_causal_neighborhood(snapshot, root, 0, max_depth)
        }
    };
    let left_neighborhood = left_visible.then(|| {
        neighborhood(left).expect("visible causal divergence root must remain queryable")
    });
    let right_neighborhood = right_visible.then(|| {
        neighborhood(right).expect("visible causal divergence root must remain queryable")
    });

    let left_frontier = directional_causal_frontier(left_neighborhood.as_ref(), direction);
    let right_frontier = directional_causal_frontier(right_neighborhood.as_ref(), direction);

    if left_visible != right_visible {
        return Ok(EvidenceCausalFirstDivergenceResult {
            root: root.stable_key(),
            direction,
            max_depth,
            identical_within_depth: false,
            divergence_depth: Some(0),
            witnesses: vec![EvidenceCausalDivergenceWitness::RootPresence {
                difference: if left_visible {
                    Difference::LeftOnly
                } else {
                    Difference::RightOnly
                },
            }],
            left_frontier,
            right_frontier,
        });
    }

    let left_neighborhood = left_neighborhood
        .as_ref()
        .expect("two-sided visible divergence root must have a left neighborhood");
    let right_neighborhood = right_neighborhood
        .as_ref()
        .expect("two-sided visible divergence root must have a right neighborhood");
    let left_edges = left_neighborhood
        .edges
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let right_edges = right_neighborhood
        .edges
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let left_positions = causal_node_positions(left_neighborhood);
    let right_positions = causal_node_positions(right_neighborhood);

    let mut candidates = Vec::<(usize, Difference, EvidenceCausalEdge)>::new();
    for edge in left_edges.difference(&right_edges) {
        candidates.push((
            directional_causal_edge_depth(edge, &left_positions, direction),
            Difference::LeftOnly,
            edge.clone(),
        ));
    }
    for edge in right_edges.difference(&left_edges) {
        candidates.push((
            directional_causal_edge_depth(edge, &right_positions, direction),
            Difference::RightOnly,
            edge.clone(),
        ));
    }

    let divergence_depth = candidates.iter().map(|(depth, _, _)| *depth).min();
    if let Some(depth) = divergence_depth {
        candidates.retain(|(candidate_depth, _, _)| *candidate_depth == depth);
        candidates.sort_by_key(|(_, difference, edge)| {
            let (cause, effect) = causal_edge_selection_ids(edge);
            (cause, effect, difference_order(*difference))
        });
    }
    let witnesses = candidates
        .into_iter()
        .map(|(_, difference, edge)| EvidenceCausalDivergenceWitness::Edge {
            difference,
            edge,
        })
        .collect();

    Ok(EvidenceCausalFirstDivergenceResult {
        root: root.stable_key(),
        direction,
        max_depth,
        identical_within_depth: divergence_depth.is_none(),
        divergence_depth,
        witnesses,
        left_frontier,
        right_frontier,
    })
}

fn directional_causal_frontier(
    neighborhood: Option<&EvidenceCausalNeighborhoodResult>,
    direction: EvidenceCausalDirection,
) -> Vec<String> {
    let frontier = neighborhood
        .map(|value| match direction {
            EvidenceCausalDirection::Upstream => value.upstream_frontier.as_slice(),
            EvidenceCausalDirection::Downstream => value.downstream_frontier.as_slice(),
        })
        .unwrap_or(&[]);
    canonical_causal_frontier(frontier)
}

fn directional_causal_edge_depth(
    edge: &EvidenceCausalEdge,
    positions: &std::collections::BTreeMap<SelectionId, EvidenceCausalNodePosition>,
    direction: EvidenceCausalDirection,
) -> usize {
    let (cause, effect) = causal_edge_selection_ids(edge);
    [cause, effect]
        .into_iter()
        .map(|event| {
            let position = positions
                .get(&event)
                .expect("induced causal edge endpoint must have a neighborhood position");
            if position.is_root {
                0
            } else {
                match direction {
                    EvidenceCausalDirection::Upstream => position.upstream_depth,
                    EvidenceCausalDirection::Downstream => position.downstream_depth,
                }
                .expect("directional causal edge endpoint must have a directional depth")
            }
        })
        .max()
        .expect("causal edge must have two endpoints")
}

fn causal_edge_selection_ids(edge: &EvidenceCausalEdge) -> (SelectionId, SelectionId) {
    let cause = parse_selection_key(&edge.cause)
        .expect("canonical causal edge cause must remain a stable selection key");
    let effect = parse_selection_key(&edge.effect)
        .expect("canonical causal edge effect must remain a stable selection key");
    (cause, effect)
}

fn difference_order(difference: Difference) -> u8 {
    match difference {
        Difference::LeftOnly => 0,
        Difference::RightOnly => 1,
        Difference::Changed => 2,
    }
}

'''
lib = replace_once(lib, marker, first_divergence_impl + marker, "first divergence implementation")
lib_path.write_text(lib)

world_query_test = r'''use serde_json::json;
use world_core::{EntityId, EventId};
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, Difference, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceCausalDirection, EvidenceCausalDivergenceWitness,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse, EvidenceSelectionKind,
    QueryError,
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

fn request(root: &str, direction: EvidenceCausalDirection, max_depth: usize) -> EvidenceComparisonQueryRequest {
    EvidenceComparisonQueryRequest::Causal(EvidenceCausalComparisonRequest::FirstDivergence {
        root: root.into(),
        direction,
        max_depth,
    })
}

fn compare(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    root: &str,
    direction: EvidenceCausalDirection,
    max_depth: usize,
) -> world_query::EvidenceCausalFirstDivergenceResult {
    let response = execute_comparison_query_request(left, right, &request(root, direction, max_depth)).unwrap();
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::FirstDivergence { value },
    ) = response else {
        panic!("expected first-divergence response")
    };
    value
}

#[test]
fn first_divergence_has_additive_tagged_protocol_v1_shape() {
    let value = json!({
        "query":"first-divergence",
        "root":"event-3",
        "direction":"upstream",
        "max_depth":2
    });
    let parsed: EvidenceComparisonQueryRequest = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(parsed, request("event-3", EvidenceCausalDirection::Upstream, 2));
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
}

#[test]
fn downstream_reports_only_the_earliest_differing_edge_layer() {
    let left = snapshot(vec![event(1, 1, &[]), event(2, 2, &[1]), event(3, 3, &[2])]);
    let right = snapshot(vec![event(1, 1, &[]), event(4, 2, &[1]), event(5, 3, &[4])]);

    let value = compare(&left, &right, "event-1", EvidenceCausalDirection::Downstream, 2);
    assert!(!value.identical_within_depth);
    assert_eq!(value.divergence_depth, Some(1));
    assert_eq!(value.witnesses.len(), 2);
    assert!(value.witnesses.iter().all(|witness| matches!(
        witness,
        EvidenceCausalDivergenceWitness::Edge { edge, .. }
            if edge.cause == "event-1" && (edge.effect == "event-2" || edge.effect == "event-4")
    )));
    assert!(value.witnesses.iter().all(|witness| !matches!(
        witness,
        EvidenceCausalDivergenceWitness::Edge { edge, .. }
            if edge.effect == "event-3" || edge.effect == "event-5"
    )));
}

#[test]
fn earliest_witnesses_use_typed_event_order_not_lexical_keys() {
    let left = snapshot(vec![event(100, 3, &[10, 2]), event(10, 2, &[]), event(2, 1, &[])]);
    let right = snapshot(vec![event(100, 3, &[])]);
    let value = compare(&left, &right, "event-100", EvidenceCausalDirection::Upstream, 1);

    assert_eq!(value.divergence_depth, Some(1));
    let causes = value
        .witnesses
        .iter()
        .map(|witness| match witness {
            EvidenceCausalDivergenceWitness::Edge { difference, edge } => {
                assert_eq!(*difference, Difference::LeftOnly);
                edge.cause.as_str()
            }
            EvidenceCausalDivergenceWitness::RootPresence { .. } => panic!("expected edge witness"),
        })
        .collect::<Vec<_>>();
    assert_eq!(causes, vec!["event-2", "event-10"]);
}

#[test]
fn bounded_identical_result_exposes_frontier_until_deeper_divergence_is_in_scope() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[9]), event(9, 1, &[])]);

    let shallow = compare(&left, &right, "event-3", EvidenceCausalDirection::Upstream, 1);
    assert!(shallow.identical_within_depth);
    assert_eq!(shallow.divergence_depth, None);
    assert!(shallow.witnesses.is_empty());
    assert_eq!(shallow.left_frontier, vec!["event-2"]);
    assert_eq!(shallow.right_frontier, vec!["event-2"]);

    let deep = compare(&left, &right, "event-3", EvidenceCausalDirection::Upstream, 2);
    assert!(!deep.identical_within_depth);
    assert_eq!(deep.divergence_depth, Some(2));
}

#[test]
fn one_sided_root_is_a_depth_zero_divergence() {
    let left = snapshot(vec![event(1, 1, &[])]);
    let right = snapshot(vec![event(2, 2, &[])]);
    let value = compare(&left, &right, "event-1", EvidenceCausalDirection::Downstream, 3);

    assert_eq!(value.divergence_depth, Some(0));
    assert_eq!(
        value.witnesses,
        vec![EvidenceCausalDivergenceWitness::RootPresence {
            difference: Difference::LeftOnly,
        }]
    );
}

#[test]
fn hidden_causal_references_do_not_create_false_divergence() {
    let hidden = snapshot(vec![event(3, 3, &[99])]);
    let plain = snapshot(vec![event(3, 3, &[])]);
    let value = compare(&hidden, &plain, "event-3", EvidenceCausalDirection::Upstream, 3);
    assert!(value.identical_within_depth);
    assert_eq!(value.divergence_depth, None);
    assert!(value.witnesses.is_empty());
}

#[test]
fn first_divergence_preserves_event_root_error_contract() {
    let left = snapshot(vec![event(1, 1, &[])]);
    let right = snapshot(vec![event(2, 2, &[])]);
    assert_eq!(
        execute_comparison_query_request(
            &left,
            &right,
            &request(&SelectionId::Entity(EntityId::new(1)).stable_key(), EvidenceCausalDirection::Upstream, 1),
        ),
        Err(QueryError::SelectionKindMismatch {
            selection: "entity-1".into(),
            expected: EvidenceSelectionKind::Event,
        })
    );
    assert_eq!(
        execute_comparison_query_request(
            &left,
            &right,
            &request("event-9", EvidenceCausalDirection::Upstream, 1),
        ),
        Err(QueryError::SelectionNotVisibleInEitherWorld("event-9".into()))
    );
}
'''
Path("crates/world-query/tests/causal_first_divergence.rs").write_text(world_query_test)

cli_test = r'''use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_projection::SelectionId;
use world_query::{
    EvidenceCausalComparisonRequest, EvidenceCausalComparisonResponse, EvidenceCausalDirection,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse,
};

#[test]
fn stdin_first_divergence_uses_existing_protocol_v1_compare_transport() {
    let (path, root) = world_fixture_with_visible_causal_edge();
    let request = EvidenceComparisonQueryRequest::Causal(
        EvidenceCausalComparisonRequest::FirstDivergence {
            root,
            direction: EvidenceCausalDirection::Upstream,
            max_depth: 1,
        },
    );
    let response = run_typed_compare(&path, &path, &request);
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::FirstDivergence { value },
    ) = response else {
        panic!("expected first-divergence comparison response")
    };
    assert!(value.identical_within_depth);
    assert_eq!(value.divergence_depth, None);
    assert!(value.witnesses.is_empty());
    let _ = fs::remove_file(path);
}

fn run_typed_compare(
    left: &Path,
    right: &Path,
    request: &EvidenceComparisonQueryRequest,
) -> EvidenceComparisonQueryResponse {
    let request = serde_json::to_string(request).unwrap();
    let output = run_query(
        &[
            "evidence-compare-query",
            left.to_str().unwrap(),
            right.to_str().unwrap(),
            "-",
        ],
        Some(&request),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["protocol"], "world-machine-evidence-query");
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["status"], "ok");
    serde_json::from_value(envelope["response"].clone()).unwrap()
}

fn run_query(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_world-cli"))
        .args(args)
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = stdin {
        child.stdin.as_mut().expect("stdin should be piped").write_all(input.as_bytes()).unwrap();
    }
    child.wait_with_output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn world_fixture_with_visible_causal_edge() -> (PathBuf, String) {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        let visible = snapshot.timeline.items.iter().map(|item| item.id).collect::<BTreeSet<_>>();
        for item in &snapshot.timeline.items {
            if item
                .caused_by
                .iter()
                .map(|cause| SelectionId::Event(*cause))
                .any(|cause| visible.contains(&cause))
            {
                let archive = session.archive().unwrap().unwrap();
                let path = temp_world_path();
                fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
                return (path, item.id.stable_key());
            }
        }
    }
    panic!("a built-in Pack should expose at least one timeline-visible causal edge")
}

fn temp_world_path() -> PathBuf {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("world-machine-m203-{}-{nonce}.world", std::process::id()))
}
'''
Path("crates/world-cli/tests/machine_query_causal_first_divergence.rs").write_text(cli_test)

next_task = r'''# Next Coding Task — M203 First Causal Divergence

Locate the earliest visible causal structural divergence between two persisted worlds without exposing raw World state or expanding the protocol surface beyond the existing comparison transport.

## Current baseline

M192–M202 provide a deterministic visible causal graph, single-world traversal/path/neighborhood queries, induced edges, frontiers, executable continuations, structural two-world causal-neighborhood comparison, and replayable comparison continuations through `world-cli evidence-compare-query` protocol v1.

## M203 — first divergence

Add an additive causal comparison request:

`{"query":"first-divergence","root":"event-N","direction":"upstream|downstream","max_depth":D}`

The response identifies the minimum directional depth at which the two visible causal graphs differ and returns every differing visible causal edge at that earliest depth as a deterministic witness set.

## Semantics

- Validate the root with the same Event-only comparison contract used by causal-neighborhood comparison.
- A root visible in only one world is an immediate depth-0 `root-presence` divergence.
- Otherwise traverse only the requested direction and compare induced visible causal edges within `max_depth`.
- Define an edge's divergence depth as the maximum directional BFS depth of its two endpoints, with the root at depth 0.
- Return only witnesses at the minimum differing depth; do not mix later differences into the first-divergence answer.
- Sort same-depth witnesses by typed `(cause EventId, effect EventId, side)` order rather than lexical stable-key order.
- Hidden referenced causes remain invisible and cannot produce witnesses.
- `identical_within_depth=true` means only that no structural divergence is visible inside the requested bound. Return left/right frontiers so callers can distinguish a bounded answer from a globally exhausted graph.

## Compatibility

- Add request/response enum variants only; preserve legacy state-evidence comparison and M201/M202 causal-neighborhood wire shapes exactly.
- Reuse `world-cli evidence-compare-query`; no new command or transport.
- Keep `world-machine-evidence-query` at protocol version 1.

## Tests

Prove downstream and upstream first divergence, root-presence depth 0, deterministic typed witness ordering, bounded identical/frontier semantics, hidden-reference filtering, stable root errors, tagged serde, and real stdin CLI transport.

## Non-goals

No global unbounded auto-search, opaque cursor, recursive server state, arbitrary graph export, AgentRuntime access, raw mutation payloads, MCP/HTTP/WebSocket, Pack-specific inference, or protocol v2.
'''
Path("NEXT_TASK.md").write_text(next_task)
