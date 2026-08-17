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
    "pub struct EvidenceCausalNeighborhoodResult {\n    pub root: EvidenceCausalNode,\n    pub upstream_depth: usize,\n    pub downstream_depth: usize,\n    pub upstream: Vec<EvidenceCausalNode>,\n    pub downstream: Vec<EvidenceCausalNode>,\n}",
    "pub struct EvidenceCausalNeighborhoodResult {\n    pub root: EvidenceCausalNode,\n    pub upstream_depth: usize,\n    pub downstream_depth: usize,\n    pub upstream: Vec<EvidenceCausalNode>,\n    pub downstream: Vec<EvidenceCausalNode>,\n    #[serde(default)]\n    pub upstream_truncated: bool,\n    #[serde(default)]\n    pub downstream_truncated: bool,\n    #[serde(default)]\n    pub upstream_frontier: Vec<String>,\n    #[serde(default)]\n    pub downstream_frontier: Vec<String>,\n}",
    "frontier DTO",
)

old_fn = r'''pub fn query_causal_neighborhood(
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
new_fn = r'''pub fn query_causal_neighborhood(
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
    let mut upstream_frontier = Vec::new();

    while let Some((current, depth)) = upstream_queue.pop_front() {
        if depth >= upstream_depth {
            if graph
                .parents(current)
                .into_iter()
                .any(|cause| !upstream_discovered.contains(&cause))
            {
                upstream_frontier.push(current.stable_key());
            }
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
    let mut downstream_frontier = Vec::new();

    while let Some((current, depth)) = downstream_queue.pop_front() {
        if depth >= downstream_depth {
            if graph
                .children(current)
                .iter()
                .any(|child| !downstream_discovered.contains(child))
            {
                downstream_frontier.push(current.stable_key());
            }
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
        upstream_truncated: !upstream_frontier.is_empty(),
        downstream_truncated: !downstream_frontier.is_empty(),
        upstream_frontier,
        downstream_frontier,
    })
}
'''
text = replace_once(text, old_fn, new_fn, "causal neighborhood frontier")
lib.write_text(text)

Path("crates/world-query/tests/causal_neighborhood_frontier.rs").write_text(r'''use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{execute_query, EvidenceQueryRequest, EvidenceQueryResponse};

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
fn frontier_identifies_exact_boundary_nodes_with_unseen_visible_neighbors() {
    let snapshot = snapshot(vec![
        event(7, 7, &[5]),
        event(6, 5, &[4]),
        event(5, 5, &[4]),
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::CausalNeighborhood {
            root: "event-4".into(),
            upstream_depth: 1,
            downstream_depth: 1,
        },
    )
    .unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };

    assert!(value.upstream_truncated);
    assert!(value.downstream_truncated);
    assert_eq!(value.upstream_frontier, vec!["event-3"]);
    assert_eq!(value.downstream_frontier, vec!["event-5"]);
}

#[test]
fn deeper_window_clears_frontier_when_visible_context_is_complete() {
    let snapshot = snapshot(vec![
        event(7, 7, &[5]),
        event(6, 5, &[4]),
        event(5, 5, &[4]),
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::CausalNeighborhood {
            root: "event-4".into(),
            upstream_depth: 2,
            downstream_depth: 2,
        },
    )
    .unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };

    assert!(!value.upstream_truncated);
    assert!(!value.downstream_truncated);
    assert!(value.upstream_frontier.is_empty());
    assert!(value.downstream_frontier.is_empty());
}

#[test]
fn zero_depth_uses_root_as_frontier_only_when_that_direction_has_more_context() {
    let snapshot = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
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

    assert_eq!(value.upstream_frontier, vec!["event-2"]);
    assert_eq!(value.downstream_frontier, vec!["event-2"]);
    assert!(value.upstream_truncated);
    assert!(value.downstream_truncated);
}

#[test]
fn cycles_do_not_create_false_frontier_once_all_visible_neighbors_are_discovered() {
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

    assert!(!value.upstream_truncated);
    assert!(!value.downstream_truncated);
    assert!(value.upstream_frontier.is_empty());
    assert!(value.downstream_frontier.is_empty());
}

#[test]
fn legacy_m195_response_deserializes_with_empty_non_truncated_frontiers() {
    let json = r#"{
        "result":"causal-neighborhood",
        "value":{
            "root":{
                "event":"event-1",
                "depth":0,
                "world_time":1,
                "title":"Event 1",
                "subtitle":"world time 1",
                "caused_by":[]
            },
            "upstream_depth":0,
            "downstream_depth":0,
            "upstream":[],
            "downstream":[]
        }
    }"#;
    let response: EvidenceQueryResponse = serde_json::from_str(json).unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };

    assert!(!value.upstream_truncated);
    assert!(!value.downstream_truncated);
    assert!(value.upstream_frontier.is_empty());
    assert!(value.downstream_frontier.is_empty());
}
''')

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M196 Causal Neighborhood Frontier

Make bounded causal neighborhoods explicitly report whether their visible upstream/downstream context was truncated by the requested depth and which included boundary Events can be expanded next.

## Current baseline

The machine causal investigation surface is complete through M195:

- M192: `why` upstream ancestry;
- M193: `influence` downstream traversal;
- M194: shortest `causal-path` plus one shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood` with independent upstream/downstream depths;
- all causal queries use timeline-visible Events and persisted `caused_by`, separate from the state-evidence graph;
- JSON/stdin transport remains protocol `world-machine-evidence-query` v1.

## Product problem

A bounded M195 result currently tells a caller what was returned, but not whether the requested depth omitted additional visible causal context. An agent can therefore mistake a finite window for a complete explanation or influence history.

## M196 — frontier metadata

Extend `EvidenceCausalNeighborhoodResult` additively with:

- `upstream_truncated: bool`;
- `downstream_truncated: bool`;
- `upstream_frontier: Vec<String>`;
- `downstream_frontier: Vec<String>`.

Mark all four fields `#[serde(default)]` so a newer v1 client can still deserialize a response emitted by an M195-era v1 server.

## Frontier semantics

- A frontier entry is an Event already included at the requested depth boundary that has at least one additional timeline-visible neighbor in that direction which was not discovered inside the requested window.
- Frontier order follows the existing traversal order: persisted parent order/BFS upstream and `(world_time, SelectionId)` child order/BFS downstream.
- `*_truncated` is exactly whether the corresponding frontier is non-empty.
- At depth 0, the root itself is the frontier if that direction has additional visible context.
- Hidden Events never create frontier entries.
- Already-discovered neighbors, including cycle edges back into the window, do not count as truncation.
- When the requested window reaches all visible causal context in a direction, the frontier is empty and `*_truncated` is false.

## Compatibility

This remains protocol v1 because the response fields are additive. New fields must default on deserialization so old v1 payloads remain readable.

## Tests

Prove at minimum:

1. exact upstream/downstream frontier nodes at a one-hop boundary;
2. deeper complete windows clear truncation/frontier metadata;
3. depth 0 correctly uses the root as frontier when more context exists;
4. cycles do not produce false frontier after all visible neighbors are discovered;
5. hidden Events do not create frontier;
6. an M195-shaped response without the new fields still deserializes with safe defaults;
7. all M192–M195 causal tests and the M195 stdin subprocess test remain green.

## Validation

Before merge:

- `bash ./scripts/check-boundaries.sh`
- `cargo fmt --all -- --check`
- `cargo test -p world-query`
- `cargo test -p world-cli`
- focused Clippy with warnings denied
- semantic workspace CI and external Pack conformance
- macOS/GPUI only if dependency-path filtering requires it

## Non-goals for M196

Do not add pagination tokens, automatic recursive expansion, causal comparison between worlds, arbitrary graph export, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
''')
