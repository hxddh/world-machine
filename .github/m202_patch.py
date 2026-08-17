from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 match, found {count}")
    return text.replace(old, new, 1)


lib = Path("crates/world-query/src/lib.rs")
text = lib.read_text()

old_result = '''pub struct EvidenceCausalNeighborhoodComparisonResult {
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
pub struct EvidenceCausalNodeDifference {'''
new_result = '''pub struct EvidenceCausalNeighborhoodComparisonResult {
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
    #[serde(default)]
    pub upstream_continuations: Vec<EvidenceCausalComparisonContinuation>,
    #[serde(default)]
    pub downstream_continuations: Vec<EvidenceCausalComparisonContinuation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalComparisonContinuation {
    pub event: String,
    pub direction: EvidenceCausalDirection,
    pub left_frontier: bool,
    pub right_frontier: bool,
    pub request: EvidenceComparisonQueryRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNodeDifference {'''
text = replace_once(text, old_result, new_result, "causal comparison continuation DTO")

old_identical = '''    let identical = nodes.is_empty()
        && left_only_edges.is_empty()
        && right_only_edges.is_empty()
        && left_upstream_frontier == right_upstream_frontier
        && left_downstream_frontier == right_downstream_frontier;

    Ok(EvidenceCausalNeighborhoodComparisonResult {'''
new_identical = '''    let upstream_continuations = causal_comparison_continuations(
        &left_upstream_frontier,
        &right_upstream_frontier,
        EvidenceCausalDirection::Upstream,
        upstream_depth,
    );
    let downstream_continuations = causal_comparison_continuations(
        &left_downstream_frontier,
        &right_downstream_frontier,
        EvidenceCausalDirection::Downstream,
        downstream_depth,
    );

    let identical = nodes.is_empty()
        && left_only_edges.is_empty()
        && right_only_edges.is_empty()
        && left_upstream_frontier == right_upstream_frontier
        && left_downstream_frontier == right_downstream_frontier;

    Ok(EvidenceCausalNeighborhoodComparisonResult {'''
text = replace_once(text, old_identical, new_identical, "causal comparison continuation construction")

old_return = '''        left_upstream_frontier,
        right_upstream_frontier,
        left_downstream_frontier,
        right_downstream_frontier,
    })
}

fn causal_node_positions('''
new_return = '''        left_upstream_frontier,
        right_upstream_frontier,
        left_downstream_frontier,
        right_downstream_frontier,
        upstream_continuations,
        downstream_continuations,
    })
}

fn causal_comparison_continuations(
    left_frontier: &[String],
    right_frontier: &[String],
    direction: EvidenceCausalDirection,
    depth: usize,
) -> Vec<EvidenceCausalComparisonContinuation> {
    let mut membership = std::collections::BTreeMap::<SelectionId, (bool, bool)>::new();
    for event in left_frontier {
        let event = parse_selection_key(event)
            .expect("canonical causal comparison frontier must remain a stable selection key");
        membership.entry(event).or_default().0 = true;
    }
    for event in right_frontier {
        let event = parse_selection_key(event)
            .expect("canonical causal comparison frontier must remain a stable selection key");
        membership.entry(event).or_default().1 = true;
    }

    membership
        .into_iter()
        .map(|(event, (left_frontier, right_frontier))| {
            let event = event.stable_key();
            let (upstream_depth, downstream_depth) = match direction {
                EvidenceCausalDirection::Upstream => (depth.max(1), 0),
                EvidenceCausalDirection::Downstream => (0, depth.max(1)),
            };
            EvidenceCausalComparisonContinuation {
                event: event.clone(),
                direction,
                left_frontier,
                right_frontier,
                request: EvidenceComparisonQueryRequest::Causal(
                    EvidenceCausalComparisonRequest::CausalNeighborhood {
                        root: event,
                        upstream_depth,
                        downstream_depth,
                    },
                ),
            }
        })
        .collect()
}

fn causal_node_positions('''
text = replace_once(text, old_return, new_return, "causal comparison continuation result")
lib.write_text(text)

Path("crates/world-query/tests/causal_compare_continuations.rs").write_text(r'''use serde_json::json;
use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceCausalDirection, EvidenceComparisonQueryRequest,
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

fn request(root: &str, upstream_depth: usize, downstream_depth: usize) -> EvidenceComparisonQueryRequest {
    EvidenceComparisonQueryRequest::Causal(EvidenceCausalComparisonRequest::CausalNeighborhood {
        root: root.into(),
        upstream_depth,
        downstream_depth,
    })
}

fn compare(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    request: &EvidenceComparisonQueryRequest,
) -> world_query::EvidenceCausalNeighborhoodComparisonResult {
    let response = execute_comparison_query_request(left, right, request).unwrap();
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value },
    ) = response
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    value
}

#[test]
fn one_sided_frontier_emits_executable_comparison_continuation() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);
    let right = snapshot(vec![event(3, 3, &[])]);
    let value = compare(&left, &right, &request("event-3", 0, 0));

    assert_eq!(value.left_upstream_frontier, vec!["event-3"]);
    assert!(value.right_upstream_frontier.is_empty());
    assert_eq!(value.upstream_continuations.len(), 1);
    let continuation = &value.upstream_continuations[0];
    assert_eq!(continuation.event, "event-3");
    assert_eq!(continuation.direction, EvidenceCausalDirection::Upstream);
    assert!(continuation.left_frontier);
    assert!(!continuation.right_frontier);
    assert_eq!(
        continuation.request,
        request("event-3", 1, 0),
        "depth-zero continuation must make one-hop progress"
    );

    let next = compare(&left, &right, &continuation.request);
    assert!(next.nodes.iter().any(|node| {
        node.event == "event-2" && node.kind == world_query::Difference::LeftOnly
    }));
    assert!(next
        .left_only_edges
        .iter()
        .any(|edge| edge.cause == "event-2" && edge.effect == "event-3"));
}

#[test]
fn distinct_frontiers_merge_into_typed_ordered_continuations() {
    let left = snapshot(vec![
        event(4, 4, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(5, 5, &[3]),
        event(3, 3, &[1]),
        event(1, 1, &[]),
    ]);
    let value = compare(&left, &right, &request("event-1", 0, 1));

    assert_eq!(value.left_downstream_frontier, vec!["event-2"]);
    assert_eq!(value.right_downstream_frontier, vec!["event-3"]);
    assert_eq!(
        value
            .downstream_continuations
            .iter()
            .map(|continuation| {
                (
                    continuation.event.as_str(),
                    continuation.left_frontier,
                    continuation.right_frontier,
                )
            })
            .collect::<Vec<_>>(),
        vec![("event-2", true, false), ("event-3", false, true)]
    );
    assert_eq!(
        value.downstream_continuations[0].request,
        request("event-2", 0, 1)
    );
    assert_eq!(
        value.downstream_continuations[1].request,
        request("event-3", 0, 1)
    );
}

#[test]
fn shared_frontier_emits_one_two_sided_continuation_and_preserves_window_size() {
    let left = snapshot(vec![
        event(4, 4, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(5, 5, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let value = compare(&left, &right, &request("event-1", 0, 1));

    assert_eq!(value.left_downstream_frontier, vec!["event-2"]);
    assert_eq!(value.right_downstream_frontier, vec!["event-2"]);
    assert_eq!(value.downstream_continuations.len(), 1);
    let continuation = &value.downstream_continuations[0];
    assert!(continuation.left_frontier);
    assert!(continuation.right_frontier);
    assert_eq!(continuation.request, request("event-2", 0, 1));

    let next = compare(&left, &right, &continuation.request);
    assert!(next.nodes.iter().any(|node| node.event == "event-4"));
    assert!(next.nodes.iter().any(|node| node.event == "event-5"));
}

#[test]
fn nonzero_comparison_window_size_is_preserved() {
    let left = snapshot(vec![
        event(4, 4, &[3]),
        event(3, 3, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![event(4, 4, &[])]);
    let value = compare(&left, &right, &request("event-4", 2, 0));
    assert_eq!(value.left_upstream_frontier, vec!["event-2"]);
    assert_eq!(value.upstream_continuations[0].request, request("event-2", 2, 0));
}

#[test]
fn m201_causal_comparison_payload_without_continuations_deserializes_with_empty_defaults() {
    let response: EvidenceComparisonQueryResponse = serde_json::from_value(json!({
        "result": "causal-neighborhood",
        "value": {
            "root": "event-3",
            "upstream_depth": 0,
            "downstream_depth": 0,
            "identical": false,
            "nodes": [],
            "left_only_edges": [],
            "right_only_edges": [],
            "left_upstream_frontier": ["event-3"],
            "right_upstream_frontier": [],
            "left_downstream_frontier": [],
            "right_downstream_frontier": []
        }
    }))
    .unwrap();

    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value },
    ) = response
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    assert!(value.upstream_continuations.is_empty());
    assert!(value.downstream_continuations.is_empty());
}
''')

Path("crates/world-cli/tests/machine_query_causal_compare_continuation.rs").write_text(r'''use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_projection::SelectionId;
use world_query::{
    EvidenceCausalComparisonRequest, EvidenceCausalComparisonResponse,
    EvidenceCausalDirection, EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse,
};

#[test]
fn stdin_causal_comparison_continuation_replays_through_existing_compare_transport() {
    let (path, root) = world_fixture_with_visible_causal_edge();
    let first_request = EvidenceComparisonQueryRequest::Causal(
        EvidenceCausalComparisonRequest::CausalNeighborhood {
            root: root.clone(),
            upstream_depth: 0,
            downstream_depth: 0,
        },
    );
    let first = run_typed_compare(&path, &path, &first_request);
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value: first },
    ) = first
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    let continuation = first
        .upstream_continuations
        .first()
        .expect("visible causal parent should create a depth-zero upstream continuation");
    assert_eq!(continuation.event, root);
    assert_eq!(continuation.direction, EvidenceCausalDirection::Upstream);
    assert!(continuation.left_frontier);
    assert!(continuation.right_frontier);

    let second = run_typed_compare(&path, &path, &continuation.request);
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value: second },
    ) = second
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    assert!(second.identical);
    assert_eq!(second.upstream_depth, 1);

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

fn world_fixture_with_visible_causal_edge() -> (PathBuf, String) {
    let registry = world_builtins::registry().unwrap();
    for descriptor in registry.descriptors() {
        let session = registry.create(&descriptor.pack.id).unwrap();
        let snapshot = session.snapshot();
        let visible = snapshot
            .timeline
            .items
            .iter()
            .map(|item| item.id)
            .collect::<BTreeSet<_>>();
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
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("world-machine-m202-{}-{nonce}.world", std::process::id()))
}
''')

Path("NEXT_TASK.md").write_text('''# Next Coding Task — M202 Executable Causal Comparison Continuations

Make bounded two-world causal comparison directly resumable by emitting typed replayable comparison requests at every left/right frontier.

## Current baseline

The machine causal investigation surface is complete through M201:

- M192–M200 provide single-world causal discovery, traversal, bounded neighborhoods, induced edges, frontiers, executable continuations, and cross-query invariants;
- M201 extends the existing protocol-v1 `evidence-compare-query` transport with tagged bounded causal-neighborhood structural comparison while preserving the legacy state-evidence compare wire shape exactly;
- causal comparison supports one-sided roots, so an Event present in only one world can still be investigated as a structural divergence.

## M202 — comparison continuations

Extend `EvidenceCausalNeighborhoodComparisonResult` additively with:

- `upstream_continuations: Vec<EvidenceCausalComparisonContinuation>`;
- `downstream_continuations: Vec<EvidenceCausalComparisonContinuation>`.

Each continuation contains:

- the canonical frontier Event key;
- `EvidenceCausalDirection`;
- `left_frontier` / `right_frontier` membership flags;
- an ordinary `EvidenceComparisonQueryRequest` that can be serialized and replayed directly through `evidence-compare-query`.

## Semantics

- Build continuations from the typed union of the left/right canonical frontier sets, one continuation per unique Event in typed Event order.
- Preserve whether the frontier is present on the left, right, or both sides.
- Preserve the original non-zero directional comparison window size.
- Promote a zero-depth frontier to a one-hop continuation so replay always makes progress.
- The opposite direction is set to depth zero.
- One-sided frontier Events are valid continuation roots because M201 comparison already supports roots visible in either world.
- Continuations carry no hidden state, visited set, opaque token, mutation authority, or server-side session state.
- `identical` remains a property of structural node/edge/frontier equality; continuation arrays are derived metadata and do not independently affect it.

## Compatibility

- Mark both new continuation arrays `#[serde(default)]` so M201 protocol-v1 causal comparison responses remain readable.
- Do not change legacy state-evidence comparison wire shapes.
- Keep `world-machine-evidence-query` at protocol version 1.

## Tests

Prove at minimum:

1. one-sided frontier emits a directly executable continuation with correct side flags;
2. replay reveals the next one-sided node/edge divergence;
3. distinct left/right frontier Events form a deterministic typed union;
4. a shared frontier emits one continuation with both side flags;
5. zero-depth continuations progress by one hop and non-zero window size is preserved;
6. M201 causal comparison payloads without continuation fields deserialize with empty defaults;
7. a real two-step stdin `world-cli evidence-compare-query` replay succeeds through the existing protocol-v1 transport;
8. all M199–M201 consistency, continuation, legacy comparison, and causal comparison tests remain green.

## Validation

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals

Do not add automatic recursive comparison, opaque pagination tokens, server-side continuation state, arbitrary graph export, raw mutation payloads, AgentRuntime access, MCP/HTTP/WebSocket, Pack-specific causal inference, or protocol v2.
''')
