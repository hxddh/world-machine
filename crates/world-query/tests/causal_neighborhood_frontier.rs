use world_core::EventId;
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
    let snapshot = snapshot(vec![
        event(3, 3, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[3]),
    ]);
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
