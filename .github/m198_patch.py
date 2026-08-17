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
    "pub struct EvidenceWhyResult {\n    pub event: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}",
    "pub struct EvidenceWhyResult {\n    pub event: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n    #[serde(default)]\n    pub edges: Vec<EvidenceCausalEdge>,\n}",
    "why edges field",
)
text = replace_once(
    text,
    "pub struct EvidenceInfluenceResult {\n    pub event: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n}",
    "pub struct EvidenceInfluenceResult {\n    pub event: String,\n    pub nodes: Vec<EvidenceCausalNode>,\n    #[serde(default)]\n    pub edges: Vec<EvidenceCausalEdge>,\n}",
    "influence edges field",
)
text = replace_once(
    text,
    "    Ok(EvidenceWhyResult {\n        event: event.stable_key(),\n        nodes,\n    })",
    "    let edges = graph.induced_edges(&discovered);\n\n    Ok(EvidenceWhyResult {\n        event: event.stable_key(),\n        nodes,\n        edges,\n    })",
    "why induced edges",
)
text = replace_once(
    text,
    "    Ok(EvidenceInfluenceResult {\n        event: event.stable_key(),\n        nodes,\n    })",
    "    let edges = graph.induced_edges(&discovered);\n\n    Ok(EvidenceInfluenceResult {\n        event: event.stable_key(),\n        nodes,\n        edges,\n    })",
    "influence induced edges",
)
lib.write_text(text)

Path("crates/world-query/tests/causal_traversal_edges.rs").write_text(r'''use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{execute_query, EvidenceCausalEdge, EvidenceQueryRequest, EvidenceQueryResponse};

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
fn why_exports_the_full_visible_ancestry_graph() {
    let snapshot = snapshot(vec![
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::Why {
            event: "event-4".into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::Why { value } = response else {
        panic!("expected why response")
    };

    assert_eq!(
        value.edges,
        vec![
            EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-3".into(),
            },
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-4".into(),
            },
        ]
    );
}

#[test]
fn influence_edges_exclude_visible_external_co_causes_outside_the_descendant_closure() {
    let snapshot = snapshot(vec![
        event(9, 9, &[]),
        event(4, 4, &[2, 3, 9]),
        event(3, 3, &[1]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::Influence {
            event: "event-1".into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::Influence { value } = response else {
        panic!("expected influence response")
    };

    let event4 = value
        .nodes
        .iter()
        .find(|node| node.event == "event-4")
        .expect("event-4 should be influenced");
    assert_eq!(event4.caused_by, vec!["event-2", "event-3", "event-9"]);
    assert_eq!(
        value.edges,
        vec![
            EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-2".into(),
            },
            EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-3".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
        ]
    );
    assert!(!value.edges.iter().any(|edge| edge.cause == "event-9"));
}

#[test]
fn traversal_edge_order_is_stable_despite_timeline_input_order() {
    let left = snapshot(vec![
        event(4, 4, &[2, 3]),
        event(3, 3, &[1]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(2, 2, &[1]),
        event(4, 4, &[2, 3]),
        event(1, 1, &[]),
        event(3, 3, &[1]),
    ]);

    let why_edges = |snapshot: &ProjectionSnapshot| {
        let response = execute_query(
            snapshot,
            &EvidenceQueryRequest::Why {
                event: "event-4".into(),
            },
        )
        .unwrap();
        let EvidenceQueryResponse::Why { value } = response else {
            panic!("expected why response")
        };
        value.edges
    };
    let influence_edges = |snapshot: &ProjectionSnapshot| {
        let response = execute_query(
            snapshot,
            &EvidenceQueryRequest::Influence {
                event: "event-1".into(),
            },
        )
        .unwrap();
        let EvidenceQueryResponse::Influence { value } = response else {
            panic!("expected influence response")
        };
        value.edges
    };

    assert_eq!(why_edges(&left), why_edges(&right));
    assert_eq!(influence_edges(&left), influence_edges(&right));
}

#[test]
fn legacy_traversal_responses_deserialize_with_empty_edges() {
    let why_json = r#"{
        "result":"why",
        "value":{
            "event":"event-1",
            "nodes":[{
                "event":"event-1",
                "depth":0,
                "world_time":1,
                "title":"Event 1",
                "subtitle":"world time 1",
                "caused_by":[]
            }]
        }
    }"#;
    let influence_json = r#"{
        "result":"influence",
        "value":{
            "event":"event-1",
            "nodes":[{
                "event":"event-1",
                "depth":0,
                "world_time":1,
                "title":"Event 1",
                "subtitle":"world time 1",
                "caused_by":[]
            }]
        }
    }"#;

    let why: EvidenceQueryResponse = serde_json::from_str(why_json).unwrap();
    let EvidenceQueryResponse::Why { value } = why else {
        panic!("expected why response")
    };
    assert!(value.edges.is_empty());

    let influence: EvidenceQueryResponse = serde_json::from_str(influence_json).unwrap();
    let EvidenceQueryResponse::Influence { value } = influence else {
        panic!("expected influence response")
    };
    assert!(value.edges.is_empty());
}
''')

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M198 Self-Contained Causal Traversals

Make the unbounded `why` and `influence` machine results self-contained causal graph payloads by reusing the explicit induced-edge semantics introduced for bounded neighborhoods in M197.

## Current baseline

The causal machine-query surface is complete through M197:

- M192: upstream `why`;
- M193: downstream `influence`;
- M194: deterministic shortest `causal-path` plus shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: frontier/truncation metadata;
- M197: explicit full induced causal `edges` for the bounded neighborhood;
- all visibility comes from timeline-visible Events and persisted `caused_by` links;
- protocol remains `world-machine-evidence-query` v1.

## Product problem

`why` and especially `influence` still expose only nodes plus each node's raw visible `caused_by` list. For an influence closure, a returned Event may have an additional visible co-cause that is not reachable from the requested root. A consumer must therefore infer which references belong to the returned graph and which are external context.

## M198 — traversal edges

Extend both result DTOs additively:

```rust
pub struct EvidenceWhyResult {
    pub event: String,
    pub nodes: Vec<EvidenceCausalNode>,
    #[serde(default)]
    pub edges: Vec<EvidenceCausalEdge>,
}

pub struct EvidenceInfluenceResult {
    pub event: String,
    pub nodes: Vec<EvidenceCausalNode>,
    #[serde(default)]
    pub edges: Vec<EvidenceCausalEdge>,
}
```

Use the M197 `VisibleCausalGraph::induced_edges` helper over each traversal's discovered Event set.

## Semantics

- `why.edges` is the full induced graph over the visible upstream ancestry closure including the root.
- `influence.edges` is the full induced graph over the visible downstream descendant closure including the root.
- A visible external co-cause may remain in an `EvidenceCausalNode.caused_by` list for context but must not appear as an edge unless that Event is itself in the traversal closure.
- Edge order and duplicate-parent handling are exactly M197 semantics; do not introduce query-specific ordering.
- Hidden Events never appear as edge endpoints.
- The fields are additive with `#[serde(default)]` so M192/M193-era v1 responses remain readable by newer clients.
- `causal-path` remains a path, not an induced subgraph, and is intentionally unchanged.

## Tests

Prove at minimum:

1. `why` returns the full visible ancestry induced graph;
2. `influence` excludes a visible external co-cause from `edges` while preserving it in node `caused_by` context;
3. edge ordering is stable despite shuffled timeline input;
4. legacy v1 `why` and `influence` responses without `edges` deserialize with empty defaults;
5. all M192–M197 causal tests and existing CLI subprocess tests remain green.

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

Do not change `causal-path`, expose omitted neighbors as edges, add arbitrary graph dumps, add causal comparison between worlds, add pagination, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
''')
