from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


lib_path = Path("crates/world-query/src/lib.rs")
text = lib_path.read_text()

if "pub struct EvidenceCausalFirstDivergenceContinuation" not in text:
    old = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
'''
    new = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalFirstDivergenceResult {
    pub root: String,
    pub direction: EvidenceCausalDirection,
    pub max_depth: usize,
    pub identical_within_depth: bool,
    pub divergence_depth: Option<usize>,
    pub witnesses: Vec<EvidenceCausalDivergenceWitness>,
    pub left_frontier: Vec<String>,
    pub right_frontier: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub continuations: Vec<EvidenceCausalFirstDivergenceContinuation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalFirstDivergenceContinuation {
    pub event: String,
    pub direction: EvidenceCausalDirection,
    pub left_frontier: bool,
    pub right_frontier: bool,
    pub depth_offset: usize,
    pub request: EvidenceComparisonQueryRequest,
}
'''
    text = replace_once(text, old, new, "first-divergence DTO")

    old = '''            left_frontier,
            right_frontier,
        });
    }
'''
    new = '''            left_frontier,
            right_frontier,
            continuations: vec![],
        });
    }
'''
    text = replace_once(text, old, new, "one-sided divergence result")

    old = '''    let witnesses = candidates
        .into_iter()
        .map(|(_, difference, edge)| {
            let (graph, positions) = match difference {
                Difference::LeftOnly => (&left_graph, &left_positions),
                Difference::RightOnly => (&right_graph, &right_positions),
                Difference::Changed => {
                    unreachable!("causal edge set difference cannot produce changed witness")
                }
            };
            let trace = causal_divergence_trace(graph, root, &edge, positions, direction);
            EvidenceCausalDivergenceWitness::Edge {
                difference,
                edge,
                trace,
            }
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

fn causal_divergence_trace(
'''
    new = '''    let witnesses = candidates
        .into_iter()
        .map(|(_, difference, edge)| {
            let (graph, positions) = match difference {
                Difference::LeftOnly => (&left_graph, &left_positions),
                Difference::RightOnly => (&right_graph, &right_positions),
                Difference::Changed => {
                    unreachable!("causal edge set difference cannot produce changed witness")
                }
            };
            let trace = causal_divergence_trace(graph, root, &edge, positions, direction);
            EvidenceCausalDivergenceWitness::Edge {
                difference,
                edge,
                trace,
            }
        })
        .collect();
    let continuations = if divergence_depth.is_none() {
        causal_first_divergence_continuations(
            &left_frontier,
            &right_frontier,
            direction,
            max_depth,
        )
    } else {
        Vec::new()
    };

    Ok(EvidenceCausalFirstDivergenceResult {
        root: root.stable_key(),
        direction,
        max_depth,
        identical_within_depth: divergence_depth.is_none(),
        divergence_depth,
        witnesses,
        left_frontier,
        right_frontier,
        continuations,
    })
}

fn causal_first_divergence_continuations(
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

fn causal_divergence_trace(
'''
    text = replace_once(text, old, new, "first-divergence continuation generation")
    lib_path.write_text(text)


test_path = Path("crates/world-query/tests/causal_first_divergence_continuations.rs")
if not test_path.exists():
    test_path.write_text(r'''use serde_json::json;
use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, Difference, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceCausalDirection, EvidenceCausalDivergenceWitness,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse,
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

#[test]
fn bounded_identical_frontier_emits_side_aware_executable_continuation() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);

    let first = compare(
        &left,
        &right,
        &request("event-3", EvidenceCausalDirection::Upstream, 1),
    );
    assert!(first.identical_within_depth);
    assert_eq!(first.divergence_depth, None);
    assert_eq!(first.continuations.len(), 1);
    let continuation = &first.continuations[0];
    assert_eq!(continuation.event, "event-2");
    assert_eq!(continuation.direction, EvidenceCausalDirection::Upstream);
    assert!(continuation.left_frontier);
    assert!(!continuation.right_frontier);
    assert_eq!(continuation.depth_offset, 1);
    assert_eq!(
        continuation.request,
        request("event-2", EvidenceCausalDirection::Upstream, 1)
    );

    let second = compare(&left, &right, &continuation.request);
    assert!(!second.identical_within_depth);
    assert_eq!(second.divergence_depth, Some(1));
    assert_eq!(continuation.depth_offset + second.divergence_depth.unwrap(), 2);
    assert_eq!(second.continuations, vec![]);
    assert_eq!(second.witnesses.len(), 1);
    assert!(matches!(
        &second.witnesses[0],
        EvidenceCausalDivergenceWitness::Edge { difference, edge, trace }
            if *difference == Difference::LeftOnly
                && edge.cause == "event-1"
                && edge.effect == "event-2"
                && trace == &vec!["event-2".to_string(), "event-1".to_string()]
    ));
}

#[test]
fn zero_depth_continuation_expands_one_hop_without_changing_depth_offset() {
    let world = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let first = compare(
        &world,
        &world,
        &request("event-3", EvidenceCausalDirection::Upstream, 0),
    );
    assert!(first.identical_within_depth);
    assert_eq!(first.continuations.len(), 1);
    let continuation = &first.continuations[0];
    assert_eq!(continuation.event, "event-3");
    assert_eq!(continuation.depth_offset, 0);
    assert_eq!(
        continuation.request,
        request("event-3", EvidenceCausalDirection::Upstream, 1)
    );

    let second = compare(&world, &world, &continuation.request);
    assert!(second.identical_within_depth);
    assert_eq!(second.max_depth, 1);
    assert_eq!(second.continuations.len(), 1);
    assert_eq!(second.continuations[0].event, "event-2");
    assert_eq!(second.continuations[0].depth_offset, 1);
}

#[test]
fn discovered_divergence_suppresses_deeper_frontier_continuations() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[4]), event(4, 1, &[])]);
    let right = snapshot(vec![event(3, 3, &[1]), event(1, 2, &[5]), event(5, 1, &[])]);
    let value = compare(
        &left,
        &right,
        &request("event-3", EvidenceCausalDirection::Upstream, 1),
    );
    assert_eq!(value.divergence_depth, Some(1));
    assert!(!value.left_frontier.is_empty());
    assert!(!value.right_frontier.is_empty());
    assert!(value.continuations.is_empty());
}

#[test]
fn continuation_union_uses_typed_event_order_and_side_flags() {
    let left = snapshot(vec![
        event(100, 3, &[2, 10]),
        event(2, 2, &[20]),
        event(10, 2, &[30]),
        event(20, 1, &[]),
        event(30, 1, &[]),
    ]);
    let right = left.clone();
    let value = compare(
        &left,
        &right,
        &request("event-100", EvidenceCausalDirection::Upstream, 1),
    );
    assert!(value.identical_within_depth);
    assert_eq!(
        value
            .continuations
            .iter()
            .map(|continuation| continuation.event.as_str())
            .collect::<Vec<_>>(),
        vec!["event-2", "event-10"]
    );
    assert!(value
        .continuations
        .iter()
        .all(|continuation| continuation.left_frontier && continuation.right_frontier));
}

#[test]
fn m204_result_without_continuations_deserializes_with_empty_default() {
    let value = json!({
        "root":"event-3",
        "direction":"upstream",
        "max_depth":1,
        "identical_within_depth":true,
        "divergence_depth":null,
        "witnesses":[],
        "left_frontier":["event-2"],
        "right_frontier":["event-2"]
    });
    let restored: world_query::EvidenceCausalFirstDivergenceResult =
        serde_json::from_value(value).unwrap();
    assert!(restored.continuations.is_empty());
}
''')


cli_test_path = Path("crates/world-cli/tests/machine_query_causal_first_divergence_continuation.rs")
if not cli_test_path.exists():
    cli_test_path.write_text(r'''use std::collections::BTreeSet;
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
fn stdin_first_divergence_continuation_replays_through_protocol_v1_transport() {
    let (path, root) = world_fixture_with_visible_causal_edge();
    let first_request =
        EvidenceComparisonQueryRequest::Causal(EvidenceCausalComparisonRequest::FirstDivergence {
            root: root.clone(),
            direction: EvidenceCausalDirection::Upstream,
            max_depth: 0,
        });
    let first = run_typed_compare(&path, &path, &first_request);
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::FirstDivergence { value: first },
    ) = first
    else {
        panic!("expected first-divergence response")
    };
    assert!(first.identical_within_depth);
    let continuation = first
        .continuations
        .first()
        .expect("depth-zero visible causal edge should emit a continuation");
    assert_eq!(continuation.event, root);
    assert_eq!(continuation.depth_offset, 0);
    assert!(continuation.left_frontier);
    assert!(continuation.right_frontier);

    let second = run_typed_compare(&path, &path, &continuation.request);
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::FirstDivergence { value: second },
    ) = second
    else {
        panic!("expected first-divergence response")
    };
    assert!(second.identical_within_depth);
    assert_eq!(second.max_depth, 1);

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
    std::env::temp_dir().join(format!(
        "world-machine-m205-{}-{nonce}.world",
        std::process::id()
    ))
}
''')


Path("NEXT_TASK.md").write_text('''# Next Coding Task — M205 Executable First-Divergence Continuations

Make bounded two-world `first-divergence` search directly resumable at every unresolved causal frontier without adding server-side state.

## Current baseline

M203 locates the earliest visible causal divergence in one direction and M204 attaches deterministic root-to-witness traces. A bounded search can still end with `identical_within_depth = true` while one or both worlds expose a frontier, which currently leaves the caller to construct follow-up searches manually.

## M205 — first-divergence continuations

Extend `EvidenceCausalFirstDivergenceResult` additively with `continuations: Vec<EvidenceCausalFirstDivergenceContinuation>` using `#[serde(default)]` compatibility.

Each continuation carries the canonical frontier Event, direction, left/right frontier membership, a `depth_offset`, and an ordinary replayable `EvidenceComparisonQueryRequest::Causal(FirstDivergence { ... })`.

## Semantics

- Emit continuations only when no divergence was found in the current bounded window.
- Build one continuation per Event in the typed union of left/right frontiers.
- Preserve whether each frontier belongs to the left world, right world, or both.
- For a non-zero window, re-root at the frontier and preserve that window size.
- For a zero-depth window, reuse the current root but promote replay to one hop so it always makes progress.
- `depth_offset` is the distance from the current request root to the continuation root. Add it to a replay response's relative `divergence_depth` to map the result back to the current request root; sum offsets across repeated replays.
- A one-sided frontier remains executable because first-divergence already supports roots visible in either world.
- Stop emitting deeper continuations as soon as a divergence is found; the earliest divergence is already resolved for that branch.

## Compatibility

No request-shape change, protocol bump, CLI command, cursor, visited set, server session, AgentRuntime access, or transport. Existing M204 result payloads without `continuations` deserialize with an empty default.

## Tests

Prove side-aware frontier replay, depth-offset arithmetic, zero-depth progress, typed continuation ordering, suppression after divergence, backward M204 deserialization, and a real two-step stdin `world-cli evidence-compare-query` replay.

## Non-goals

No automatic global recursive search scheduler, opaque cursor, arbitrary graph export, MCP/HTTP/WebSocket, mutation authority, Pack-specific inference, protocol v2, or unrestricted AgentRuntime projection access.
''')
