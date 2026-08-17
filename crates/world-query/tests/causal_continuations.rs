use serde_json::json;
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
    assert!(value
        .edges
        .iter()
        .any(|edge| edge.cause == "event-3" && edge.effect == "event-4"));
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
