use serde_json::json;
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
    assert_eq!(
        continuation.depth_offset + second.divergence_depth.unwrap(),
        2
    );
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
