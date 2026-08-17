from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected 1 match, found {count}")
    return text.replace(old, new, 1)


lib = Path("crates/world-query/src/lib.rs")
text = lib.read_text()

old_result = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNeighborhoodResult {
    pub root: EvidenceCausalNode,
    pub upstream_depth: usize,
    pub downstream_depth: usize,
    pub upstream: Vec<EvidenceCausalNode>,
    pub downstream: Vec<EvidenceCausalNode>,
    #[serde(default)]
    pub upstream_truncated: bool,
    #[serde(default)]
    pub downstream_truncated: bool,
    #[serde(default)]
    pub upstream_frontier: Vec<String>,
    #[serde(default)]
    pub downstream_frontier: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNode {'''

new_result = '''#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCausalNeighborhoodResult {
    pub root: EvidenceCausalNode,
    pub upstream_depth: usize,
    pub downstream_depth: usize,
    pub upstream: Vec<EvidenceCausalNode>,
    pub downstream: Vec<EvidenceCausalNode>,
    #[serde(default)]
    pub upstream_truncated: bool,
    #[serde(default)]
    pub downstream_truncated: bool,
    #[serde(default)]
    pub upstream_frontier: Vec<String>,
    #[serde(default)]
    pub downstream_frontier: Vec<String>,
    #[serde(default)]
    pub upstream_continuations: Vec<EvidenceCausalContinuation>,
    #[serde(default)]
    pub downstream_continuations: Vec<EvidenceCausalContinuation>,
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

text = replace_once(text, old_result, new_result, "causal neighborhood DTO")

old_return = '''    Ok(EvidenceCausalNeighborhoodResult {
        root: graph.node(root, 0),
        upstream_depth,
        downstream_depth,
        upstream,
        downstream,
        upstream_truncated: !upstream_frontier.is_empty(),
        downstream_truncated: !downstream_frontier.is_empty(),
        upstream_frontier,
        downstream_frontier,
    })'''

new_return = '''    let upstream_continuations = upstream_frontier
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

    Ok(EvidenceCausalNeighborhoodResult {
        root: graph.node(root, 0),
        upstream_depth,
        downstream_depth,
        upstream,
        downstream,
        upstream_truncated: !upstream_frontier.is_empty(),
        downstream_truncated: !downstream_frontier.is_empty(),
        upstream_frontier,
        downstream_frontier,
        upstream_continuations,
        downstream_continuations,
    })'''

text = replace_once(text, old_return, new_return, "causal neighborhood return")
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
    let EvidenceQueryResponse::CausalNeighborhood { value: next_upstream } = next_upstream else {
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
    let EvidenceQueryResponse::CausalNeighborhood { value: next_downstream } = next_downstream else {
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
    let snapshot = snapshot(vec![
        event(4, 4, &[3]),
        event(3, 3, &[2]),
        event(2, 2, &[]),
    ]);
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
fn continuation_preserves_nonzero_window_size() {
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
}

#[test]
fn m196_payload_without_continuations_deserializes_with_empty_defaults() {
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
            "downstream_frontier": []
        }
    }))
    .unwrap();

    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };
    assert!(value.upstream_continuations.is_empty());
    assert!(value.downstream_continuations.is_empty());
}
''')

Path("crates/world-cli/tests/machine_query_causal_continuation.rs").write_text(r'''use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
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
        second.upstream.iter().any(|node| node.event == expected_parent),
        "continuation should reveal the visible causal parent"
    );

    let _ = fs::remove_file(path);
}

fn run_typed_query(path: &PathBuf, request: &EvidenceQueryRequest) -> EvidenceQueryResponse {
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

Path("NEXT_TASK.md").write_text('''# Next Coding Task — M197 Executable Causal Continuations

Turn M196 causal frontier metadata into directly executable continuation requests so an external investigator can advance through a large visible causal graph in bounded deterministic windows without reconstructing query arguments.

## Current baseline

The machine causal investigation surface is complete through M196:

- M192: `why` upstream ancestry;
- M193: `influence` downstream traversal;
- M194: shortest `causal-path` plus one shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: explicit truncation and stable upstream/downstream frontier Events;
- all causal semantics remain based only on timeline-visible Events and persisted `caused_by`, separate from state-evidence adjacency;
- JSON/stdin transport remains `world-machine-evidence-query` protocol v1.

## Product problem

M196 tells a caller exactly where a bounded causal window was cut off, but the caller still has to interpret direction, reconstruct a new `causal-neighborhood` request, and avoid generating a zero-depth no-op. That is unnecessary protocol logic for every future agent/tool adapter.

## M197 — executable continuations

Extend `EvidenceCausalNeighborhoodResult` additively with:

- `upstream_continuations: Vec<EvidenceCausalContinuation>`;
- `downstream_continuations: Vec<EvidenceCausalContinuation>`.

Add:

- `EvidenceCausalDirection::{Upstream, Downstream}`;
- `EvidenceCausalContinuation { event, direction, request }` where `request` is an ordinary `EvidenceQueryRequest` that can be serialized and passed directly back to the existing `evidence-query` machine transport.

Mark both continuation arrays `#[serde(default)]` so M196-era protocol-v1 responses remain deserializable.

## Continuation semantics

- There is exactly one continuation per frontier entry, in the same deterministic order as the corresponding frontier.
- An upstream continuation roots at that frontier Event, sets `downstream_depth = 0`, and preserves the caller's non-zero `upstream_depth` as the next window size.
- A downstream continuation roots at that frontier Event, sets `upstream_depth = 0`, and preserves the caller's non-zero `downstream_depth` as the next window size.
- If the original depth was `0`, the continuation depth is promoted to `1`; an executable continuation must always make progress.
- Continuations are suggestions over the same immutable visible ProjectionSnapshot. They do not carry hidden state, visited sets, opaque server tokens, or mutation authority.
- Overlap between separately expanded frontier branches is allowed; stable Event keys let the caller deduplicate across windows.

## Tests

Prove at minimum:

1. upstream and downstream frontier entries produce exact typed continuation requests;
2. each emitted request can be passed directly back to `execute_query` and reveals the next causal window;
3. depth-zero frontiers emit one-hop progressing continuations rather than no-ops;
4. non-zero window sizes are preserved across continuation generation;
5. an M196-shaped response without continuation fields deserializes with empty defaults;
6. a real `world-cli` stdin subprocess can take a continuation emitted by one machine query and replay it as the next machine query;
7. all M192–M196 causal tests remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M197

Do not add opaque pagination tokens, server-side continuation state, automatic recursive expansion, causal comparison between worlds, arbitrary graph export, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
''')
