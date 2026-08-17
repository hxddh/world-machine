from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


lib = Path("crates/world-query/src/lib.rs")
text = lib.read_text()
text = replace_once(
    text,
    "    Influence { event: String },\n    CausalPath { from: String, to: String },\n    Neighborhood { root: String, max_depth: usize },",
    "    Influence { event: String },\n    CausalPath { from: String, to: String },\n    CausalNeighborhood {\n        root: String,\n        upstream_depth: usize,\n        downstream_depth: usize,\n    },\n    Neighborhood { root: String, max_depth: usize },",
    "request variant",
)
text = replace_once(
    text,
    "    Influence { value: EvidenceInfluenceResult },\n    CausalPath { value: EvidenceCausalPathResult },\n    Neighborhood { value: EvidenceNeighborhoodResult },",
    "    Influence { value: EvidenceInfluenceResult },\n    CausalPath { value: EvidenceCausalPathResult },\n    CausalNeighborhood {\n        value: EvidenceCausalNeighborhoodResult,\n    },\n    Neighborhood { value: EvidenceNeighborhoodResult },",
    "response variant",
)
text = replace_once(
    text,
    "pub struct EvidenceCausalPathResult {\n    pub from: String,\n    pub to: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalNode",
    "pub struct EvidenceCausalPathResult {\n    pub from: String,\n    pub to: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalNeighborhoodResult {\n    pub root: EvidenceCausalNode,\n    pub upstream_depth: usize,\n    pub downstream_depth: usize,\n    pub upstream: Vec<EvidenceCausalNode>,\n    pub downstream: Vec<EvidenceCausalNode>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalNode",
    "neighborhood DTO",
)
text = replace_once(
    text,
    "        EvidenceQueryRequest::CausalPath { from, to } => {\n            let from = parse_selection_key(from)?;\n            let to = parse_selection_key(to)?;\n            query_causal_path(snapshot, from, to)\n                .map(|value| EvidenceQueryResponse::CausalPath { value })\n        }\n        EvidenceQueryRequest::Neighborhood { root, max_depth } => {",
    "        EvidenceQueryRequest::CausalPath { from, to } => {\n            let from = parse_selection_key(from)?;\n            let to = parse_selection_key(to)?;\n            query_causal_path(snapshot, from, to)\n                .map(|value| EvidenceQueryResponse::CausalPath { value })\n        }\n        EvidenceQueryRequest::CausalNeighborhood {\n            root,\n            upstream_depth,\n            downstream_depth,\n        } => {\n            let root = parse_selection_key(root)?;\n            query_causal_neighborhood(snapshot, root, *upstream_depth, *downstream_depth)\n                .map(|value| EvidenceQueryResponse::CausalNeighborhood { value })\n        }\n        EvidenceQueryRequest::Neighborhood { root, max_depth } => {",
    "execute arm",
)

neighborhood_fn = r'''pub fn query_causal_neighborhood(
    snapshot: &ProjectionSnapshot,
    root: SelectionId,
    upstream_depth: usize,
    downstream_depth: usize,
) -> Result<EvidenceCausalNeighborhoodResult, QueryError> {
    let graph = VisibleCausalGraph::new(snapshot);
    graph.require_event(root)?;

    let mut upstream_discovered = std::collections::BTreeSet::from([root]);
    let mut upstream_queue = std::collections::VecDeque::from([(root, 0usize)]);
    let mut upstream = Vec::new();

    while let Some((current, depth)) = upstream_queue.pop_front() {
        if depth >= upstream_depth {
            continue;
        }
        let next_depth = depth + 1;
        for cause in graph.parents(current) {
            if upstream_discovered.insert(cause) {
                upstream.push(graph.node(cause, next_depth));
                upstream_queue.push_back((cause, next_depth));
            }
        }
    }

    let mut downstream_discovered = std::collections::BTreeSet::from([root]);
    let mut downstream_queue = std::collections::VecDeque::from([(root, 0usize)]);
    let mut downstream = Vec::new();

    while let Some((current, depth)) = downstream_queue.pop_front() {
        if depth >= downstream_depth {
            continue;
        }
        let next_depth = depth + 1;
        for child in graph.children(current) {
            if downstream_discovered.insert(*child) {
                downstream.push(graph.node(*child, next_depth));
                downstream_queue.push_back((*child, next_depth));
            }
        }
    }

    Ok(EvidenceCausalNeighborhoodResult {
        root: graph.node(root, 0),
        upstream_depth,
        downstream_depth,
        upstream,
        downstream,
    })
}

'''
text = replace_once(
    text,
    "pub fn query_neighborhood(\n    snapshot: &ProjectionSnapshot,",
    neighborhood_fn + "pub fn query_neighborhood(\n    snapshot: &ProjectionSnapshot,",
    "causal neighborhood function",
)
lib.write_text(text)

Path("crates/world-query/tests/causal_neighborhood.rs").write_text(r'''use world_core::{EntityId, EventId};
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_query, EvidenceQueryRequest, EvidenceQueryResponse, EvidenceSelectionKind, QueryError,
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

#[test]
fn causal_neighborhood_is_bounded_bidirectional_and_round_trips() {
    let snapshot = snapshot(vec![
        event(7, 7, &[5]),
        event(6, 5, &[4]),
        event(5, 5, &[4]),
        event(4, 4, &[3, 2, 99]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);
    let request: EvidenceQueryRequest = serde_json::from_str(
        r#"{"query":"causal-neighborhood","root":"event-4","upstream_depth":2,"downstream_depth":2}"#,
    )
    .unwrap();
    let response = execute_query(&snapshot, &request).unwrap();
    let json = serde_json::to_string(&response).unwrap();
    let restored: EvidenceQueryResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, response);

    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };
    assert_eq!(value.root.event, "event-4");
    assert_eq!(value.root.depth, 0);
    assert_eq!(value.root.caused_by, vec!["event-3", "event-2"]);
    assert_eq!(value.upstream_depth, 2);
    assert_eq!(value.downstream_depth, 2);
    assert_eq!(
        value
            .upstream
            .iter()
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>(),
        vec![("event-3", 1), ("event-2", 1), ("event-1", 2)]
    );
    assert_eq!(
        value
            .downstream
            .iter()
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>(),
        vec![("event-5", 1), ("event-6", 1), ("event-7", 2)]
    );
}

#[test]
fn zero_depths_disable_each_side_without_hiding_the_root() {
    let snapshot = snapshot(vec![event(2, 2, &[1]), event(1, 1, &[])]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::CausalNeighborhood {
            root: "event-2".into(),
            upstream_depth: 0,
            downstream_depth: 0,
        },
    )
    .unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };
    assert_eq!(value.root.event, "event-2");
    assert!(value.upstream.is_empty());
    assert!(value.downstream.is_empty());
}

#[test]
fn causal_neighborhood_cycles_do_not_duplicate_the_root_or_loop() {
    let snapshot = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[3])]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::CausalNeighborhood {
            root: "event-1".into(),
            upstream_depth: 8,
            downstream_depth: 8,
        },
    )
    .unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };
    assert_eq!(
        value
            .upstream
            .iter()
            .map(|node| node.event.as_str())
            .collect::<Vec<_>>(),
        vec!["event-3", "event-2"]
    );
    assert_eq!(
        value
            .downstream
            .iter()
            .map(|node| node.event.as_str())
            .collect::<Vec<_>>(),
        vec!["event-2", "event-3"]
    );
}

#[test]
fn causal_neighborhood_reuses_event_root_validation() {
    let snapshot = snapshot(vec![event(1, 1, &[])]);
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::CausalNeighborhood {
                root: SelectionId::Entity(EntityId::new(1)).stable_key(),
                upstream_depth: 1,
                downstream_depth: 1,
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
            &EvidenceQueryRequest::CausalNeighborhood {
                root: "event-07".into(),
                upstream_depth: 1,
                downstream_depth: 1,
            },
        ),
        Err(QueryError::InvalidSelectionKey("event-07".into()))
    );
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::CausalNeighborhood {
                root: "event-99".into(),
                upstream_depth: 1,
                downstream_depth: 1,
            },
        ),
        Err(QueryError::SelectionNotVisible("event-99".into()))
    );
}
''')

Path("crates/world-cli/tests/machine_query_causal_neighborhood.rs").write_text(r'''use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use world_query::{EvidenceQueryRequest, EvidenceQueryResponse};

#[test]
fn stdin_causal_neighborhood_query_emits_a_versioned_typed_context() {
    let (path, event) = world_fixture_with_event();
    let request = serde_json::to_string(&EvidenceQueryRequest::CausalNeighborhood {
        root: event.clone(),
        upstream_depth: 0,
        downstream_depth: 0,
    })
    .unwrap();

    let output = run_query(
        &["evidence-query", path.to_str().unwrap(), "-"],
        Some(&request),
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["protocol"], "world-machine-evidence-query");
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["status"], "ok");
    let response: EvidenceQueryResponse =
        serde_json::from_value(envelope["response"].clone()).unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };
    assert_eq!(value.root.event, event);
    assert_eq!(value.root.depth, 0);
    assert!(value.upstream.is_empty());
    assert!(value.downstream.is_empty());

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

fn temp_world_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("world-machine-causal-neighborhood-{unique}.world"))
}
''')

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M195 Machine Causal Neighborhood

Expose a bounded bidirectional causal context query so external investigators can inspect the visible causes and visible effects around one Event in a single machine request.

## Current baseline

The machine investigation surface is complete through M194:

- M190–M191: visible selection discovery and display-safe describe;
- state-evidence neighborhood / shortest-path / comparison remain available as a separate graph family;
- M192: unbounded upstream `why` over visible persisted causal links;
- M193: unbounded downstream `influence` with deterministic child ordering;
- M194: deterministic shortest `causal-path`, plus one private `VisibleCausalGraph` shared by all causal queries;
- generic JSON/stdin CLI transport remains protocol `world-machine-evidence-query` v1.

## Product goal

A caller should be able to ask:

```json
{
  "query":"causal-neighborhood",
  "root":"event-42",
  "upstream_depth":2,
  "downstream_depth":2
}
```

and receive one bounded local causal context without issuing and reconciling separate `why` and `influence` requests.

## Architecture boundary

1. Implement only in `world-query`; `world-cli` remains generic transport.
2. Reuse the M194 private `VisibleCausalGraph` and `EvidenceCausalNode`.
3. Read only timeline-visible Events and persisted `TimelineItem.caused_by` links.
4. Keep causal traversal separate from state-evidence adjacency and inspector visibility.
5. Do not expose ProjectionSnapshot to AgentRuntime.
6. Keep protocol v1; the new request/response is additive.

## M195 — `causal-neighborhood`

Add request:

```json
{"query":"causal-neighborhood","root":"event-42","upstream_depth":2,"downstream_depth":2}
```

Return `EvidenceCausalNeighborhoodResult` with:

- `root: EvidenceCausalNode` at depth 0;
- the requested `upstream_depth` and `downstream_depth`;
- `upstream: Vec<EvidenceCausalNode>` excluding the root;
- `downstream: Vec<EvidenceCausalNode>` excluding the root.

## Traversal rules

- Root uses the existing canonical stable-key parser and timeline-visible Event validation.
- Upstream and downstream depth limits are independent; zero disables that side.
- Upstream traversal is BFS, preserving each Event's persisted visible parent order.
- Downstream traversal is BFS, preserving M193/M194 `(world_time, SelectionId)` child order.
- Depth is minimum causal edge distance from root in that direction.
- Each direction deduplicates independently and cycle-protects with the root pre-discovered.
- In an actual causal cycle, the same non-root Event may legitimately appear once in each direction; direction is represented by membership in `upstream` versus `downstream`.
- `EvidenceCausalNode.caused_by` keeps its existing contract: persisted order filtered to timeline-visible Events, even if a referenced visible cause lies outside the requested depth window.
- Hidden Events never appear in traversal or `caused_by` metadata.

## Tests

Prove at minimum:

1. request/response serde round-trip;
2. independent upstream/downstream bounds;
3. persisted upstream order and stable downstream order;
4. minimum BFS depth through branching;
5. zero-depth sides return no contextual nodes while retaining the root;
6. hidden cause IDs are filtered;
7. cycles do not duplicate the root or loop;
8. canonical wrong-kind, malformed key, and invisible Event errors remain stable;
9. existing M192–M194 causal tests remain green;
10. a real stdin `world-cli` subprocess emits the v1 typed causal-neighborhood response.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M195

Do not add causal graph comparison between worlds, arbitrary graph export, search/filter, pagination, HTTP/WebSocket/MCP, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
''')
