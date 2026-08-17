use world_core::{EntityId, EventId};
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
