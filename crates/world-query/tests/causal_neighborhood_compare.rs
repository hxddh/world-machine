use serde_json::json;
use world_core::{EntityId, EventId};
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceComparisonQueryRequest,
    EvidenceComparisonQueryResponse, EvidenceComparisonRequest, EvidenceComparisonResult,
    EvidenceSelectionKind, QueryError,
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

fn causal_request(
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
    root: &str,
    upstream_depth: usize,
    downstream_depth: usize,
) -> world_query::EvidenceCausalNeighborhoodComparisonResult {
    let response = execute_comparison_query_request(
        left,
        right,
        &causal_request(root, upstream_depth, downstream_depth),
    )
    .unwrap();
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::CausalNeighborhood { value },
    ) = response
    else {
        panic!("expected causal-neighborhood comparison response")
    };
    value
}

#[test]
fn machine_comparison_wire_preserves_legacy_shape_and_adds_tagged_causal_shape() {
    let legacy_json = json!({"root":"entity-1","max_depth":2});
    let legacy: EvidenceComparisonQueryRequest =
        serde_json::from_value(legacy_json.clone()).unwrap();
    assert_eq!(
        legacy,
        EvidenceComparisonQueryRequest::Legacy(EvidenceComparisonRequest {
            root: "entity-1".into(),
            max_depth: 2,
        })
    );
    assert_eq!(serde_json::to_value(&legacy).unwrap(), legacy_json);

    let causal_json = json!({
        "query":"causal-neighborhood",
        "root":"event-3",
        "upstream_depth":1,
        "downstream_depth":2
    });
    let causal: EvidenceComparisonQueryRequest =
        serde_json::from_value(causal_json.clone()).unwrap();
    assert_eq!(causal, causal_request("event-3", 1, 2));
    assert_eq!(serde_json::to_value(&causal).unwrap(), causal_json);

    let legacy_response = EvidenceComparisonQueryResponse::Legacy(EvidenceComparisonResult {
        root: "entity-1".into(),
        max_depth: 0,
        identical: true,
        nodes: vec![],
        left_only_edges: vec![],
        right_only_edges: vec![],
    });
    assert_eq!(
        serde_json::to_value(&legacy_response).unwrap(),
        json!({
            "root":"entity-1",
            "max_depth":0,
            "identical":true,
            "nodes":[],
            "left_only_edges":[],
            "right_only_edges":[]
        })
    );
}

#[test]
fn causal_comparison_reports_bidirectional_node_and_edge_divergence() {
    let left = snapshot(vec![
        event(4, 4, &[3]),
        event(3, 3, &[2]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(5, 5, &[3]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);

    let value = compare(&left, &right, "event-3", 1, 1);
    assert!(!value.identical);
    assert_eq!(
        value
            .nodes
            .iter()
            .map(|node| (node.event.as_str(), node.kind))
            .collect::<Vec<_>>(),
        vec![
            ("event-1", world_query::Difference::RightOnly),
            ("event-2", world_query::Difference::LeftOnly),
            ("event-4", world_query::Difference::LeftOnly),
            ("event-5", world_query::Difference::RightOnly),
        ]
    );
    assert!(value
        .left_only_edges
        .iter()
        .any(|edge| edge.cause == "event-2" && edge.effect == "event-3"));
    assert!(value
        .left_only_edges
        .iter()
        .any(|edge| edge.cause == "event-3" && edge.effect == "event-4"));
    assert!(value
        .right_only_edges
        .iter()
        .any(|edge| edge.cause == "event-1" && edge.effect == "event-3"));
    assert!(value
        .right_only_edges
        .iter()
        .any(|edge| edge.cause == "event-3" && edge.effect == "event-5"));
}

#[test]
fn causal_comparison_marks_changed_directional_depths_and_cycle_positions() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(3, 3, &[1]), event(2, 2, &[1]), event(1, 1, &[])]);
    let value = compare(&left, &right, "event-3", 2, 0);
    let changed = value
        .nodes
        .iter()
        .find(|node| node.event == "event-1")
        .expect("event-1 should change causal depth");
    assert_eq!(changed.kind, world_query::Difference::Changed);
    assert_eq!(changed.left.as_ref().unwrap().upstream_depth, Some(2));
    assert_eq!(changed.right.as_ref().unwrap().upstream_depth, Some(1));

    let cycle = snapshot(vec![event(2, 2, &[1]), event(1, 1, &[2])]);
    let one_way = snapshot(vec![event(2, 2, &[1]), event(1, 1, &[])]);
    let value = compare(&cycle, &one_way, "event-1", 1, 1);
    let event_two = value
        .nodes
        .iter()
        .find(|node| node.event == "event-2")
        .expect("event-2 should have a changed directional position");
    assert_eq!(event_two.kind, world_query::Difference::Changed);
    assert_eq!(event_two.left.as_ref().unwrap().upstream_depth, Some(1));
    assert_eq!(event_two.left.as_ref().unwrap().downstream_depth, Some(1));
    assert_eq!(event_two.right.as_ref().unwrap().upstream_depth, None);
    assert_eq!(event_two.right.as_ref().unwrap().downstream_depth, Some(1));
}

#[test]
fn causal_comparison_ignores_hidden_references_but_compares_frontier_membership() {
    let hidden = snapshot(vec![event(3, 3, &[99])]);
    let plain = snapshot(vec![event(3, 3, &[])]);
    let value = compare(&hidden, &plain, "event-3", 2, 2);
    assert!(value.identical);
    assert!(value.nodes.is_empty());
    assert!(value.left_only_edges.is_empty());
    assert!(value.right_only_edges.is_empty());

    let deeper = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);
    let shallow = snapshot(vec![event(3, 3, &[])]);
    let value = compare(&deeper, &shallow, "event-3", 0, 0);
    assert!(!value.identical);
    assert!(value.nodes.is_empty());
    assert_eq!(value.left_upstream_frontier, vec!["event-3"]);
    assert!(value.right_upstream_frontier.is_empty());
}

#[test]
fn causal_comparison_allows_one_sided_root_and_enforces_event_visibility_contract() {
    let left = snapshot(vec![event(1, 1, &[])]);
    let right = snapshot(vec![event(2, 2, &[])]);
    let value = compare(&left, &right, "event-1", 1, 1);
    assert!(!value.identical);
    let root = value
        .nodes
        .iter()
        .find(|node| node.event == "event-1")
        .expect("one-sided root should be reported");
    assert_eq!(root.kind, world_query::Difference::LeftOnly);
    assert!(root.left.as_ref().unwrap().is_root);
    assert!(root.right.is_none());

    let absent = snapshot(vec![event(7, 7, &[])]);
    assert_eq!(
        execute_comparison_query_request(&left, &absent, &causal_request("event-9", 1, 1)),
        Err(QueryError::SelectionNotVisibleInEitherWorld(
            "event-9".into()
        ))
    );
    assert_eq!(
        execute_comparison_query_request(
            &left,
            &right,
            &causal_request(&SelectionId::Entity(EntityId::new(1)).stable_key(), 1, 1),
        ),
        Err(QueryError::SelectionKindMismatch {
            selection: "entity-1".into(),
            expected: EvidenceSelectionKind::Event,
        })
    );
    assert_eq!(
        execute_comparison_query_request(&left, &right, &causal_request("event-07", 1, 1)),
        Err(QueryError::InvalidSelectionKey("event-07".into()))
    );
}

#[test]
fn identical_causal_comparison_round_trips_as_tagged_response() {
    let snapshot = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let response =
        execute_comparison_query_request(&snapshot, &snapshot, &causal_request("event-2", 1, 1))
            .unwrap();
    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["result"], "causal-neighborhood");
    assert_eq!(json["value"]["identical"], true);
    let restored: EvidenceComparisonQueryResponse = serde_json::from_value(json).unwrap();
    assert_eq!(restored, response);
}
