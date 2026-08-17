from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


lib_path = Path("crates/world-query/src/lib.rs")
text = lib_path.read_text()

if "pub trace_prefix: Vec<String>" not in text:
    old = '''pub struct EvidenceCausalFirstDivergenceContinuation {
    pub event: String,
    pub direction: EvidenceCausalDirection,
    pub left_frontier: bool,
    pub right_frontier: bool,
    pub depth_offset: usize,
    pub request: EvidenceComparisonQueryRequest,
}
'''
    new = '''pub struct EvidenceCausalFirstDivergenceContinuation {
    pub event: String,
    pub direction: EvidenceCausalDirection,
    pub left_frontier: bool,
    pub right_frontier: bool,
    pub depth_offset: usize,
    #[serde(default)]
    pub trace_prefix: Vec<String>,
    pub request: EvidenceComparisonQueryRequest,
}
'''
    text = replace_once(text, old, new, "continuation DTO")

    old = '''    let continuations = if divergence_depth.is_none() {
        causal_first_divergence_continuations(&left_frontier, &right_frontier, direction, max_depth)
    } else {
        Vec::new()
    };
'''
    new = '''    let continuations = if divergence_depth.is_none() {
        causal_first_divergence_continuations(
            &left_graph,
            &right_graph,
            root,
            &left_positions,
            &right_positions,
            &left_frontier,
            &right_frontier,
            direction,
            max_depth,
        )
    } else {
        Vec::new()
    };
'''
    text = replace_once(text, old, new, "continuation call")

    old = '''fn causal_first_divergence_continuations(
    left_frontier: &[String],
    right_frontier: &[String],
    direction: EvidenceCausalDirection,
    max_depth: usize,
) -> Vec<EvidenceCausalFirstDivergenceContinuation> {
    let mut membership = std::collections::BTreeMap::<SelectionId, (bool, bool)>::new();
    for event in left_frontier {
        let event = parse_selection_key(event)
            .expect("canonical first-divergence frontier must remain a stable selection key");
        membership.entry(event).or_default().0 = true;
    }
    for event in right_frontier {
        let event = parse_selection_key(event)
            .expect("canonical first-divergence frontier must remain a stable selection key");
        membership.entry(event).or_default().1 = true;
    }

    membership
        .into_iter()
        .map(|(event, (left_frontier, right_frontier))| {
            let event = event.stable_key();
            EvidenceCausalFirstDivergenceContinuation {
                event: event.clone(),
                direction,
                left_frontier,
                right_frontier,
                depth_offset: max_depth,
                request: EvidenceComparisonQueryRequest::Causal(
                    EvidenceCausalComparisonRequest::FirstDivergence {
                        root: event,
                        direction,
                        max_depth: max_depth.max(1),
                    },
                ),
            }
        })
        .collect()
}
'''
    new = '''fn causal_first_divergence_continuations(
    left_graph: &VisibleCausalGraph<'_>,
    right_graph: &VisibleCausalGraph<'_>,
    root: SelectionId,
    left_positions: &std::collections::BTreeMap<SelectionId, EvidenceCausalNodePosition>,
    right_positions: &std::collections::BTreeMap<SelectionId, EvidenceCausalNodePosition>,
    left_frontier: &[String],
    right_frontier: &[String],
    direction: EvidenceCausalDirection,
    max_depth: usize,
) -> Vec<EvidenceCausalFirstDivergenceContinuation> {
    let mut membership = std::collections::BTreeMap::<SelectionId, (bool, bool)>::new();
    for event in left_frontier {
        let event = parse_selection_key(event)
            .expect("canonical first-divergence frontier must remain a stable selection key");
        membership.entry(event).or_default().0 = true;
    }
    for event in right_frontier {
        let event = parse_selection_key(event)
            .expect("canonical first-divergence frontier must remain a stable selection key");
        membership.entry(event).or_default().1 = true;
    }

    membership
        .into_iter()
        .map(|(event, (left_frontier, right_frontier))| {
            let (graph, positions) = if left_frontier {
                (left_graph, left_positions)
            } else {
                (right_graph, right_positions)
            };
            let allowed = positions
                .keys()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            let trace_prefix = directional_shortest_event_path(
                graph,
                root,
                event,
                direction,
                &allowed,
            )
            .expect("first-divergence frontier must remain directionally reachable")
            .into_iter()
            .map(|event| event.stable_key())
            .collect();
            let event = event.stable_key();
            EvidenceCausalFirstDivergenceContinuation {
                event: event.clone(),
                direction,
                left_frontier,
                right_frontier,
                depth_offset: max_depth,
                trace_prefix,
                request: EvidenceComparisonQueryRequest::Causal(
                    EvidenceCausalComparisonRequest::FirstDivergence {
                        root: event,
                        direction,
                        max_depth: max_depth.max(1),
                    },
                ),
            }
        })
        .collect()
}
'''
    text = replace_once(text, old, new, "continuation helper")
    lib_path.write_text(text)


test_path = Path("crates/world-query/tests/causal_first_divergence_continuation_traces.rs")
if not test_path.exists():
    test_path.write_text(r'''use serde_json::json;
use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceCausalDirection, EvidenceCausalDivergenceWitness,
    EvidenceCausalFirstDivergenceContinuation, EvidenceComparisonQueryRequest,
    EvidenceComparisonQueryResponse,
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

fn request(
    root: &str,
    direction: EvidenceCausalDirection,
    max_depth: usize,
) -> EvidenceComparisonQueryRequest {
    EvidenceComparisonQueryRequest::Causal(EvidenceCausalComparisonRequest::FirstDivergence {
        root: root.into(),
        direction,
        max_depth,
    })
}

fn compare(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request: &EvidenceComparisonQueryRequest,
) -> world_query::EvidenceCausalFirstDivergenceResult {
    let response = execute_comparison_query_request(left, right, request).unwrap();
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::FirstDivergence { value },
    ) = response
    else {
        panic!("expected first-divergence response")
    };
    value
}

fn only_edge_trace(value: &world_query::EvidenceCausalFirstDivergenceResult) -> Vec<String> {
    let [EvidenceCausalDivergenceWitness::Edge { trace, .. }] = value.witnesses.as_slice() else {
        panic!("expected exactly one edge witness")
    };
    trace.clone()
}

fn compose(prefix: &[String], replay: &[String]) -> Vec<String> {
    let mut composed = prefix.to_vec();
    composed.extend(replay.iter().skip(1).cloned());
    composed
}

#[test]
fn upstream_prefix_composes_with_replay_witness_to_match_monolithic_trace() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);

    let first = compare(
        &left,
        &right,
        &request("event-3", EvidenceCausalDirection::Upstream, 1),
    );
    let continuation = first.continuations.first().unwrap();
    assert_eq!(continuation.trace_prefix, vec!["event-3", "event-2"]);

    let replay = compare(&left, &right, &continuation.request);
    let monolithic = compare(
        &left,
        &right,
        &request("event-3", EvidenceCausalDirection::Upstream, 2),
    );
    assert_eq!(
        compose(&continuation.trace_prefix, &only_edge_trace(&replay)),
        only_edge_trace(&monolithic)
    );
}

#[test]
fn downstream_prefix_composes_with_replay_witness_to_match_monolithic_trace() {
    let left = snapshot(vec![event(1, 1, &[]), event(2, 2, &[1]), event(3, 3, &[2])]);
    let right = snapshot(vec![event(1, 1, &[]), event(2, 2, &[1])]);

    let first = compare(
        &left,
        &right,
        &request("event-1", EvidenceCausalDirection::Downstream, 1),
    );
    let continuation = first.continuations.first().unwrap();
    assert_eq!(continuation.trace_prefix, vec!["event-1", "event-2"]);

    let replay = compare(&left, &right, &continuation.request);
    let monolithic = compare(
        &left,
        &right,
        &request("event-1", EvidenceCausalDirection::Downstream, 2),
    );
    assert_eq!(
        compose(&continuation.trace_prefix, &only_edge_trace(&replay)),
        only_edge_trace(&monolithic)
    );
}

#[test]
fn zero_depth_prefix_is_the_root_and_promoted_replay_still_composes() {
    let world = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);
    let value = compare(
        &world,
        &world,
        &request("event-3", EvidenceCausalDirection::Upstream, 0),
    );
    let continuation = value.continuations.first().unwrap();
    assert_eq!(continuation.depth_offset, 0);
    assert_eq!(continuation.trace_prefix, vec!["event-3"]);
    assert_eq!(
        continuation.request,
        request("event-3", EvidenceCausalDirection::Upstream, 1)
    );
}

#[test]
fn prefix_uses_typed_shortest_path_when_multiple_routes_reach_same_frontier() {
    let world = snapshot(vec![
        event(1, 1, &[]),
        event(2, 2, &[1]),
        event(3, 2, &[1]),
        event(4, 3, &[2, 3]),
        event(5, 4, &[4]),
    ]);
    let value = compare(
        &world,
        &world,
        &request("event-1", EvidenceCausalDirection::Downstream, 2),
    );
    let continuation = value
        .continuations
        .iter()
        .find(|continuation| continuation.event == "event-4")
        .unwrap();
    assert_eq!(
        continuation.trace_prefix,
        vec!["event-1", "event-2", "event-4"]
    );
}

#[test]
fn m205_continuation_without_trace_prefix_deserializes_with_empty_default() {
    let value = json!({
        "event":"event-2",
        "direction":"upstream",
        "left_frontier":true,
        "right_frontier":true,
        "depth_offset":1,
        "request":{
            "query":"first-divergence",
            "root":"event-2",
            "direction":"upstream",
            "max_depth":1
        }
    });
    let continuation: EvidenceCausalFirstDivergenceContinuation =
        serde_json::from_value(value).unwrap();
    assert!(continuation.trace_prefix.is_empty());
}
''')

Path("NEXT_TASK.md").write_text('''# Next Coding Task — M207 Composable First-Divergence Trace Prefixes

Preserve M204 witness explainability across M205 segmented replay by attaching a deterministic root-to-frontier trace prefix to every first-divergence continuation.

## Current baseline

M203 identifies the earliest bounded causal divergence, M204 gives each edge witness a deterministic directional trace, M205 makes the search resumable at frontier Events, and M206 proves segmented depth/witness semantics match monolithic deeper queries. The remaining composition gap is explanatory: a replayed witness trace begins at the continuation root rather than the original query root.

## M207 — continuation trace prefixes

Extend `EvidenceCausalFirstDivergenceContinuation` additively with `trace_prefix: Vec<String>` using `#[serde(default)]`.

## Semantics

- `trace_prefix` begins at the current request root and ends at the continuation frontier Event.
- Use the same directional traversal semantics as M204 witness traces.
- Restrict the path to Events already visible inside the current bounded neighborhood.
- Choose a shortest path; break equal-length alternatives by typed Event identity using the existing deterministic path helper.
- For a frontier present on both sides, either side must yield the same structural prefix because no divergence was found inside the current window; use a deterministic side choice.
- For one-sided frontier membership, derive the prefix from the side that owns the frontier.
- A zero-depth continuation has prefix `[root]`.
- To rebuild an original-root witness trace after replay, concatenate `trace_prefix` with the replay witness trace while dropping the replay trace's first Event, which is the shared frontier root.

## Compatibility

M205 continuation payloads without `trace_prefix` deserialize with an empty default. No request shape, CLI command, protocol version, server state, AgentRuntime authority, or transport changes.

## Tests

Prove upstream/downstream prefix composition against monolithic M204 traces, zero-depth behavior, typed shortest-path selection in a diamond, and backward M205 deserialization.

## Non-goals

No production recursive scheduler, no automatic trace concatenation API, no opaque cursor, no arbitrary graph export, no MCP/HTTP/WebSocket, no AgentRuntime access, and no protocol v2.
''')
