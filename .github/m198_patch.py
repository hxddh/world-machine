from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 match, found {count}")
    return text.replace(old, new, 1)


lib = Path("crates/world-query/src/lib.rs")
text = lib.read_text()

old_dto = '''    #[serde(default)]
    pub edges: Vec<EvidenceCausalEdge>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct EvidenceCausalEdge {
    pub cause: String,
    pub effect: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNode {'''
new_dto = '''    #[serde(default)]
    pub edges: Vec<EvidenceCausalEdge>,
    #[serde(default)]
    pub upstream_continuations: Vec<EvidenceCausalContinuation>,
    #[serde(default)]
    pub downstream_continuations: Vec<EvidenceCausalContinuation>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct EvidenceCausalEdge {
    pub cause: String,
    pub effect: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceCausalDirection {
    Upstream,
    Downstream,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalContinuation {
    pub event: String,
    pub direction: EvidenceCausalDirection,
    pub request: EvidenceQueryRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNode {'''
text = replace_once(text, old_dto, new_dto, "continuation DTOs")

old_edges = '''    let edges = graph.induced_edges(&included);

    Ok(EvidenceCausalNeighborhoodResult {'''
new_edges = '''    let edges = graph.induced_edges(&included);
    let upstream_continuations = upstream_frontier
        .iter()
        .map(|event| EvidenceCausalContinuation {
            event: event.clone(),
            direction: EvidenceCausalDirection::Upstream,
            request: EvidenceQueryRequest::CausalNeighborhood {
                root: event.clone(),
                upstream_depth: upstream_depth.max(1),
                downstream_depth: 0,
            },
        })
        .collect();
    let downstream_continuations = downstream_frontier
        .iter()
        .map(|event| EvidenceCausalContinuation {
            event: event.clone(),
            direction: EvidenceCausalDirection::Downstream,
            request: EvidenceQueryRequest::CausalNeighborhood {
                root: event.clone(),
                upstream_depth: 0,
                downstream_depth: downstream_depth.max(1),
            },
        })
        .collect();

    Ok(EvidenceCausalNeighborhoodResult {'''
text = replace_once(text, old_edges, new_edges, "continuation construction")

old_result = '''        upstream_frontier,
        downstream_frontier,
        edges,
    })'''
new_result = '''        upstream_frontier,
        downstream_frontier,
        edges,
        upstream_continuations,
        downstream_continuations,
    })'''
text = replace_once(text, old_result, new_result, "continuation result fields")
lib.write_text(text)

Path("crates/world-query/tests/causal_continuations.rs").write_text(r'''use serde_json::json;
use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_query, EvidenceCausalDirection, EvidenceQueryRequest, EvidenceQueryResponse,
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

fn neighborhood(
    snapshot: &ProjectionSnapshot,
    root: &str,
    upstream_depth: usize,
    downstream_depth: usize,
) -> world_query::EvidenceCausalNeighborhoodResult {
    let response = execute_query(
        snapshot,
        &EvidenceQueryRequest::CausalNeighborhood {
            root: root.into(),
            upstream_depth,
            downstream_depth,
        },
    )
    .unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };
    value
}

#[test]
fn frontier_continuations_are_directly_executable_in_both_directions() {
    let snapshot = snapshot(vec![
        event(5, 5, &[4]),
        event(4, 4, &[3]),
        event(3, 3, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let value = neighborhood(&snapshot, "event-3", 1, 1);

    assert_eq!(value.upstream_frontier, vec!["event-2"]);
    assert_eq!(value.downstream_frontier, vec!["event-4"]);
    assert_eq!(value.upstream_continuations.len(), 1);
    assert_eq!(value.downstream_continuations.len(), 1);

    let upstream = &value.upstream_continuations[0];
    assert_eq!(upstream.event, "event-2");
    assert_eq!(upstream.direction, EvidenceCausalDirection::Upstream);
    assert_eq!(
        serde_json::to_value(&upstream.request).unwrap(),
        json!({
            "query": "causal-neighborhood",
            "root": "event-2",
            "upstream_depth": 1,
            "downstream_depth": 0
        })
    );
    let next_upstream = execute_query(&snapshot, &upstream.request).unwrap();
    let EvidenceQueryResponse::CausalNeighborhood {
        value: next_upstream,
    } = next_upstream
    else {
        panic!("expected causal-neighborhood response")
    };
    assert_eq!(
        next_upstream
            .upstream
            .iter()
            .map(|node| node.event.as_str())
            .collect::<Vec<_>>(),
        vec!["event-1"]
    );

    let downstream = &value.downstream_continuations[0];
    assert_eq!(downstream.event, "event-4");
    assert_eq!(downstream.direction, EvidenceCausalDirection::Downstream);
    assert_eq!(
        serde_json::to_value(&downstream.request).unwrap(),
        json!({
            "query": "causal-neighborhood",
            "root": "event-4",
            "upstream_depth": 0,
            "downstream_depth": 1
        })
    );
    let next_downstream = execute_query(&snapshot, &downstream.request).unwrap();
    let EvidenceQueryResponse::CausalNeighborhood {
        value: next_downstream,
    } = next_downstream
    else {
        panic!("expected causal-neighborhood response")
    };
    assert_eq!(
        next_downstream
            .downstream
            .iter()
            .map(|node| node.event.as_str())
            .collect::<Vec<_>>(),
        vec!["event-5"]
    );
}

#[test]
fn zero_depth_frontiers_emit_progressing_one_hop_continuations() {
    let snapshot = snapshot(vec![event(4, 4, &[3]), event(3, 3, &[2]), event(2, 2, &[])]);
    let value = neighborhood(&snapshot, "event-3", 0, 0);

    assert_eq!(value.upstream_frontier, vec!["event-3"]);
    assert_eq!(value.downstream_frontier, vec!["event-3"]);
    assert_eq!(
        value.upstream_continuations[0].request,
        EvidenceQueryRequest::CausalNeighborhood {
            root: "event-3".into(),
            upstream_depth: 1,
            downstream_depth: 0,
        }
    );
    assert_eq!(
        value.downstream_continuations[0].request,
        EvidenceQueryRequest::CausalNeighborhood {
            root: "event-3".into(),
            upstream_depth: 0,
            downstream_depth: 1,
        }
    );

    let upstream = execute_query(&snapshot, &value.upstream_continuations[0].request).unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value: upstream } = upstream else {
        panic!("expected causal-neighborhood response")
    };
    assert_eq!(upstream.upstream[0].event, "event-2");

    let downstream = execute_query(&snapshot, &value.downstream_continuations[0].request).unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value: downstream } = downstream else {
        panic!("expected causal-neighborhood response")
    };
    assert_eq!(downstream.downstream[0].event, "event-4");
}

#[test]
fn continuation_preserves_nonzero_window_size_and_induced_edges() {
    let snapshot = snapshot(vec![
        event(5, 5, &[4]),
        event(4, 4, &[3]),
        event(3, 3, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let value = neighborhood(&snapshot, "event-4", 2, 0);

    assert_eq!(value.upstream_frontier, vec!["event-2"]);
    assert_eq!(
        value.upstream_continuations[0].request,
        EvidenceQueryRequest::CausalNeighborhood {
            root: "event-2".into(),
            upstream_depth: 2,
            downstream_depth: 0,
        }
    );
    assert!(value.edges.iter().any(|edge| edge.cause == "event-3" && edge.effect == "event-4"));
}

#[test]
fn m197_edges_payload_without_continuations_deserializes_with_empty_defaults() {
    let response: EvidenceQueryResponse = serde_json::from_value(json!({
        "result": "causal-neighborhood",
        "value": {
            "root": {
                "event": "event-3",
                "depth": 0,
                "world_time": 3,
                "title": "Event 3",
                "subtitle": "world time 3",
                "caused_by": ["event-2"]
            },
            "upstream_depth": 1,
            "downstream_depth": 1,
            "upstream": [],
            "downstream": [],
            "upstream_truncated": true,
            "downstream_truncated": false,
            "upstream_frontier": ["event-2"],
            "downstream_frontier": [],
            "edges": [{"cause":"event-2","effect":"event-3"}]
        }
    }))
    .unwrap();

    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };
    assert_eq!(value.edges.len(), 1);
    assert_eq!(value.edges[0].cause, "event-2");
    assert_eq!(value.edges[0].effect, "event-3");
    assert!(value.upstream_continuations.is_empty());
    assert!(value.downstream_continuations.is_empty());
}
''')

Path("crates/world-cli/tests/machine_query_causal_continuation.rs").write_text(r'''use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_projection::SelectionId;
use world_query::{EvidenceCausalDirection, EvidenceQueryRequest, EvidenceQueryResponse};

#[test]
fn stdin_frontier_continuation_can_be_replayed_as_the_next_machine_query() {
    let (path, root, expected_parent) = world_fixture_with_visible_causal_edge();
    let request = EvidenceQueryRequest::CausalNeighborhood {
        root: root.clone(),
        upstream_depth: 0,
        downstream_depth: 0,
    };
    let first = run_typed_query(&path, &request);
    let EvidenceQueryResponse::CausalNeighborhood { value: first } = first else {
        panic!("expected causal-neighborhood response")
    };
    assert!(first.upstream_truncated);
    assert_eq!(first.upstream_frontier, vec![root.clone()]);
    let continuation = first
        .upstream_continuations
        .first()
        .expect("depth-zero causal root should expose an upstream continuation");
    assert_eq!(continuation.event, root);
    assert_eq!(continuation.direction, EvidenceCausalDirection::Upstream);

    let second = run_typed_query(&path, &continuation.request);
    let EvidenceQueryResponse::CausalNeighborhood { value: second } = second else {
        panic!("expected causal-neighborhood response")
    };
    assert!(
        second
            .upstream
            .iter()
            .any(|node| node.event == expected_parent),
        "continuation should reveal the visible causal parent"
    );
    assert!(
        second
            .edges
            .iter()
            .any(|edge| edge.cause == expected_parent && edge.effect == continuation.event),
        "continued window should retain M197 induced-edge semantics"
    );

    let _ = fs::remove_file(path);
}

fn run_typed_query(path: &Path, request: &EvidenceQueryRequest) -> EvidenceQueryResponse {
    let request = serde_json::to_string(request).unwrap();
    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
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

fn world_fixture_with_visible_causal_edge() -> (PathBuf, String, String) {
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
            for cause in &item.caused_by {
                let cause = SelectionId::Event(*cause);
                if !visible.contains(&cause) {
                    continue;
                }
                let archive = session.archive().unwrap().unwrap();
                let path = temp_world_path();
                fs::write(&path, archive.to_json_pretty().unwrap()).unwrap();
                return (path, item.id.stable_key(), cause.stable_key());
            }
        }
    }
    panic!("a built-in Pack should expose at least one timeline-visible causal edge")
}

fn temp_world_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("world-machine-causal-continuation-{unique}.world"))
}
''')

Path("NEXT_TASK.md").write_text('''# Next Coding Task — M198 Executable Causal Continuations

Turn the M196 frontier metadata into directly executable continuation requests while preserving M197's self-contained induced-edge payloads.

## Current baseline

The machine causal investigation surface is complete through M197:

- M192: upstream `why`;
- M193: downstream `influence`;
- M194: deterministic shortest `causal-path` and shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: explicit truncation and stable frontier Events;
- M197: the full induced visible causal edge set for every bounded neighborhood;
- causal visibility remains timeline-owned and separate from state-evidence adjacency;
- JSON/stdin transport remains `world-machine-evidence-query` protocol v1.

## Product problem

M196/M197 tell a caller where a bounded causal window stops and give a complete local graph, but the caller still has to interpret direction and reconstruct a new request to continue. Every future agent or tool adapter would otherwise duplicate that protocol logic and could accidentally generate a zero-depth no-op.

## M198 — executable continuations

Extend `EvidenceCausalNeighborhoodResult` additively with:

- `upstream_continuations: Vec<EvidenceCausalContinuation>`;
- `downstream_continuations: Vec<EvidenceCausalContinuation>`.

Add:

- `EvidenceCausalDirection::{Upstream, Downstream}`;
- `EvidenceCausalContinuation { event, direction, request }`, where `request` is an ordinary `EvidenceQueryRequest` that can be serialized and passed directly back to the existing `evidence-query` transport.

Both continuation arrays use `#[serde(default)]` so M197-era protocol-v1 responses remain readable.

## Continuation semantics

- Emit exactly one continuation per frontier entry, in frontier order.
- Upstream continuations root at the frontier Event, set `downstream_depth = 0`, and preserve the caller's non-zero upstream window size.
- Downstream continuations are symmetric.
- If the original directional depth is `0`, promote the continuation window to `1`; a continuation must make progress.
- Each continuation query independently returns M197 induced edges for its own bounded window.
- Continuations carry no hidden state, visited set, opaque server token, mutation authority, or server-side session state.
- Separate continuation branches may overlap; stable Event keys and causal edges let callers deduplicate/merge windows deterministically.

## Tests

Prove at minimum:

1. exact typed upstream/downstream continuation requests;
2. emitted requests execute directly and reveal the next causal window;
3. depth-zero frontier continuations progress by one hop;
4. non-zero directional window sizes are preserved;
5. current M197 induced edges remain present before and after continuation;
6. an M197-shaped v1 payload with `edges` but no continuation fields deserializes with empty defaults;
7. a real two-step `world-cli` stdin subprocess can replay an emitted continuation request against the same `.world` file;
8. all M192–M197 causal tests remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M198

Do not add opaque pagination tokens, server-side continuation state, automatic recursive expansion, causal comparison between worlds, arbitrary graph export, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
''')
