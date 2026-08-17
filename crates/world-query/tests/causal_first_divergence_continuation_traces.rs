use serde_json::json;
use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceCausalDirection, EvidenceCausalDivergenceWitness,
    EvidenceCausalFirstDivergenceContinuation, EvidenceComparisonQueryRequest,
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

fn only_edge_trace(value: &world_query::EvidenceCausalFirstDivergenceResult) -> Vec<String> {
    let [EvidenceCausalDivergenceWitness::Edge { trace, .. }] = value.witnesses.as_slice() else {
        panic!("expected exactly one edge witness")
    };
    trace.clone()
}

fn compose(prefix: &[String], replay: &[String]) -> Vec<String> {
    let mut composed = prefix.to_vec();
    composed.extend(replay.iter().skip(1).cloned());
    composed
}

#[test]
fn upstream_prefix_composes_with_replay_witness_to_match_monolithic_trace() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);

    let first = compare(
        &left,
        &right,
        &request("event-3", EvidenceCausalDirection::Upstream, 1),
    );
    let continuation = first.continuations.first().unwrap();
    assert_eq!(continuation.trace_prefix, vec!["event-3", "event-2"]);

    let replay = compare(&left, &right, &continuation.request);
    let monolithic = compare(
        &left,
        &right,
        &request("event-3", EvidenceCausalDirection::Upstream, 2),
    );
    assert_eq!(
        compose(&continuation.trace_prefix, &only_edge_trace(&replay)),
        only_edge_trace(&monolithic)
    );
}

#[test]
fn downstream_prefix_composes_with_replay_witness_to_match_monolithic_trace() {
    let left = snapshot(vec![event(1, 1, &[]), event(2, 2, &[1]), event(3, 3, &[2])]);
    let right = snapshot(vec![event(1, 1, &[]), event(2, 2, &[1])]);

    let first = compare(
        &left,
        &right,
        &request("event-1", EvidenceCausalDirection::Downstream, 1),
    );
    let continuation = first.continuations.first().unwrap();
    assert_eq!(continuation.trace_prefix, vec!["event-1", "event-2"]);

    let replay = compare(&left, &right, &continuation.request);
    let monolithic = compare(
        &left,
        &right,
        &request("event-1", EvidenceCausalDirection::Downstream, 2),
    );
    assert_eq!(
        compose(&continuation.trace_prefix, &only_edge_trace(&replay)),
        only_edge_trace(&monolithic)
    );
}

#[test]
fn zero_depth_prefix_is_the_root_and_promoted_replay_still_composes() {
    let world = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);
    let value = compare(
        &world,
        &world,
        &request("event-3", EvidenceCausalDirection::Upstream, 0),
    );
    let continuation = value.continuations.first().unwrap();
    assert_eq!(continuation.depth_offset, 0);
    assert_eq!(continuation.trace_prefix, vec!["event-3"]);
    assert_eq!(
        continuation.request,
        request("event-3", EvidenceCausalDirection::Upstream, 1)
    );
}

#[test]
fn prefix_uses_typed_shortest_path_when_multiple_routes_reach_same_frontier() {
    let world = snapshot(vec![
        event(1, 1, &[]),
        event(2, 2, &[1]),
        event(3, 2, &[1]),
        event(4, 3, &[2, 3]),
        event(5, 4, &[4]),
    ]);
    let value = compare(
        &world,
        &world,
        &request("event-1", EvidenceCausalDirection::Downstream, 2),
    );
    let continuation = value
        .continuations
        .iter()
        .find(|continuation| continuation.event == "event-4")
        .unwrap();
    assert_eq!(
        continuation.trace_prefix,
        vec!["event-1", "event-2", "event-4"]
    );
}

#[test]
fn m205_continuation_without_trace_prefix_deserializes_with_empty_default() {
    let value = json!({
        "event":"event-2",
        "direction":"upstream",
        "left_frontier":true,
        "right_frontier":true,
        "depth_offset":1,
        "request":{
            "query":"first-divergence",
            "root":"event-2",
            "direction":"upstream",
            "max_depth":1
        }
    });
    let continuation: EvidenceCausalFirstDivergenceContinuation =
        serde_json::from_value(value).unwrap();
    assert!(continuation.trace_prefix.is_empty());
}
