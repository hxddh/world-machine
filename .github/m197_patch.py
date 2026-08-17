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
    "    #[serde(default)]\n    pub downstream_frontier: Vec<String>,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalNode",
    "    #[serde(default)]\n    pub downstream_frontier: Vec<String>,\n    #[serde(default)]\n    pub edges: Vec<EvidenceCausalEdge>,\n}\n\n#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]\npub struct EvidenceCausalEdge {\n    pub cause: String,\n    pub effect: String,\n}\n\n#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\npub struct EvidenceCausalNode",
    "causal edge DTO",
)
text = replace_once(
    text,
    "    fn node(&self, event: SelectionId, depth: usize) -> EvidenceCausalNode {",
    r'''    fn induced_edges(
        &self,
        included: &std::collections::BTreeSet<SelectionId>,
    ) -> Vec<EvidenceCausalEdge> {
        let mut effects = included.iter().copied().collect::<Vec<_>>();
        effects.sort_by_key(|effect| {
            let item = self
                .events
                .get(effect)
                .copied()
                .expect("included causal event must remain visible");
            (item.world_time, *effect)
        });

        let mut edges = Vec::new();
        for effect in effects {
            let mut seen_causes = std::collections::BTreeSet::new();
            for cause in self.parents(effect) {
                if included.contains(&cause) && seen_causes.insert(cause) {
                    edges.push(EvidenceCausalEdge {
                        cause: cause.stable_key(),
                        effect: effect.stable_key(),
                    });
                }
            }
        }
        edges
    }

    fn node(&self, event: SelectionId, depth: usize) -> EvidenceCausalNode {''',
    "induced edge helper",
)
text = replace_once(
    text,
    "    Ok(EvidenceCausalNeighborhoodResult {\n        root: graph.node(root, 0),",
    "    let included = upstream_discovered\n        .union(&downstream_discovered)\n        .copied()\n        .collect::<std::collections::BTreeSet<_>>();\n    let edges = graph.induced_edges(&included);\n\n    Ok(EvidenceCausalNeighborhoodResult {\n        root: graph.node(root, 0),",
    "included causal graph",
)
text = replace_once(
    text,
    "        upstream_frontier,\n        downstream_frontier,\n    })",
    "        upstream_frontier,\n        downstream_frontier,\n        edges,\n    })",
    "causal edges response",
)
lib.write_text(text)

Path("crates/world-query/tests/causal_neighborhood_edges.rs").write_text(r'''use world_core::EventId;
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

fn neighborhood(snapshot: &ProjectionSnapshot) -> Vec<EvidenceCausalEdge> {
    let response = execute_query(
        snapshot,
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
    value.edges
}

#[test]
fn causal_neighborhood_exports_the_full_induced_visible_subgraph_not_only_tree_edges() {
    let snapshot = snapshot(vec![
        event(5, 5, &[4, 2]),
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);

    assert_eq!(
        neighborhood(&snapshot),
        vec![
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-4".into(),
                effect: "event-5".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-5".into(),
            },
        ]
    );
}

#[test]
fn edge_order_is_independent_of_timeline_input_order() {
    let left = snapshot(vec![
        event(5, 5, &[4, 2]),
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(2, 2, &[]),
        event(5, 5, &[4, 2]),
        event(1, 1, &[]),
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
    ]);

    assert_eq!(neighborhood(&left), neighborhood(&right));
}

#[test]
fn hidden_and_out_of_window_endpoints_do_not_leak_into_edges() {
    let snapshot = snapshot(vec![
        event(6, 6, &[5]),
        event(5, 5, &[4, 2, 99]),
        event(4, 4, &[3]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);

    assert_eq!(
        neighborhood(&snapshot),
        vec![
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-4".into(),
                effect: "event-5".into(),
            },
        ]
    );
}

#[test]
fn duplicate_persisted_parent_ids_emit_one_graph_edge_at_first_parent_position() {
    let snapshot = snapshot(vec![
        event(5, 5, &[4, 2, 4]),
        event(4, 4, &[3, 3, 2]),
        event(3, 3, &[]),
        event(2, 2, &[]),
    ]);

    assert_eq!(
        neighborhood(&snapshot),
        vec![
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-4".into(),
                effect: "event-5".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-5".into(),
            },
        ]
    );
}

#[test]
fn legacy_m196_response_deserializes_with_empty_edges() {
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
            "downstream":[],
            "upstream_truncated":false,
            "downstream_truncated":false,
            "upstream_frontier":[],
            "downstream_frontier":[]
        }
    }"#;
    let response: EvidenceQueryResponse = serde_json::from_str(json).unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };

    assert!(value.edges.is_empty());
}
''')

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M197 Causal Neighborhood Edges

Make bounded causal neighborhoods self-contained graph payloads by exporting all persisted visible causal edges induced by the Events already returned in the local window.

## Current baseline

The machine causal investigation surface is complete through M196:

- M192: upstream `why`;
- M193: downstream `influence`;
- M194: deterministic shortest `causal-path` and shared private `VisibleCausalGraph`;
- M195: bounded bidirectional `causal-neighborhood`;
- M196: explicit truncation/frontier metadata for bounded windows;
- causal visibility remains timeline-owned and separate from state-evidence adjacency;
- protocol remains `world-machine-evidence-query` v1.

## Product problem

M195/M196 return enough node metadata to infer some edges from each node's `caused_by`, but a consumer must still reconstruct the graph and decide which references are inside the bounded window. This is error-prone, especially for diamonds, cross-branch links, cycles, and duplicated persisted parent IDs.

## M197 — explicit induced causal edges

Extend `EvidenceCausalNeighborhoodResult` additively with:

```rust
#[serde(default)]
pub edges: Vec<EvidenceCausalEdge>
```

and add:

```rust
pub struct EvidenceCausalEdge {
    pub cause: String,
    pub effect: String,
}
```

The serde default preserves v1 backward deserialization for M196-era payloads.

## Edge semantics

- The returned Event set is the union of the root, upstream nodes, and downstream nodes.
- `edges` is the full induced directed causal subgraph over that Event set, not merely BFS traversal-tree edges.
- An edge exists only when the effect's persisted `caused_by` contains the cause and both endpoints are timeline-visible and included in the returned window.
- Hidden Events and visible Events outside the requested window never appear as edge endpoints.
- Duplicate persisted parent IDs emit one edge, keeping the first persisted parent position for ordering.
- Effects are ordered deterministically by `(world_time, SelectionId)` ascending; within an effect, causes preserve persisted visible parent order.
- Cycles and self-edges are represented faithfully when both endpoints are included.
- `EvidenceCausalNode.caused_by` remains unchanged and may still name visible causes outside the bounded window; `edges` is specifically the self-contained local graph.

## Internal implementation

Add a private `VisibleCausalGraph::induced_edges(included)` helper so edge filtering, deduplication, and ordering remain centralized with the causal graph semantics.

## Tests

Prove at minimum:

1. a bounded neighborhood exports all induced edges, including a cross-branch edge that was not needed by BFS discovery;
2. edge order is stable despite shuffled timeline input;
3. hidden and out-of-window Event endpoints do not leak;
4. duplicate persisted parent IDs produce one edge;
5. M196-shaped v1 payloads without `edges` deserialize with an empty default;
6. all M192–M196 causal tests and existing CLI subprocess tests remain green.

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

Do not export edges to omitted frontier neighbors, add arbitrary causal graph dumps, add causal comparison between worlds, change `caused_by`, add pagination, MCP/HTTP/WebSocket, AgentRuntime access, raw mutation payloads, Pack-specific causal inference, or protocol v2.
''')
