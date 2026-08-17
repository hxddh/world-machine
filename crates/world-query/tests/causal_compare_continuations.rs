use serde_json::json;
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

fn request(
    root: &str,
    upstream_depth: usize,
    downstream_depth: usize,
) -> EvidenceComparisonQueryRequest {
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
    assert!(next
        .nodes
        .iter()
        .any(|node| { node.event == "event-2" && node.kind == world_query::Difference::LeftOnly }));
    assert!(next
        .left_only_edges
        .iter()
        .any(|edge| edge.cause == "event-2" && edge.effect == "event-3"));
}

#[test]
fn distinct_frontiers_merge_into_typed_ordered_continuations() {
    let left = snapshot(vec![event(4, 4, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(5, 5, &[3]), event(3, 3, &[1]), event(1, 1, &[])]);
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
    let left = snapshot(vec![event(4, 4, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(5, 5, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
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
    assert_eq!(
        value.upstream_continuations[0].request,
        request("event-2", 2, 0)
    );
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
