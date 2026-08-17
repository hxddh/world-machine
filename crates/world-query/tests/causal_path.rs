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
fn causal_path_uses_deterministic_shortest_downstream_route_and_round_trips() {
    let snapshot = snapshot(vec![
        event(4, 4, &[3, 2]),
        event(3, 2, &[1]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let request: EvidenceQueryRequest =
        serde_json::from_str(r#"{"query":"causal-path","from":"event-1","to":"event-4"}"#).unwrap();
    let response = execute_query(&snapshot, &request).unwrap();
    let json = serde_json::to_string(&response).unwrap();
    let restored: EvidenceQueryResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, response);

    let EvidenceQueryResponse::CausalPath { value } = response else {
        panic!("expected causal-path response")
    };
    assert_eq!(value.from, "event-1");
    assert_eq!(value.to, "event-4");
    assert_eq!(
        value
            .nodes
            .iter()
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>(),
        vec![("event-1", 0), ("event-2", 1), ("event-4", 2)]
    );
}

#[test]
fn causal_path_identity_is_a_single_visible_node() {
    let snapshot = snapshot(vec![event(7, 11, &[])]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::CausalPath {
            from: "event-7".into(),
            to: "event-7".into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::CausalPath { value } = response else {
        panic!("expected causal-path response")
    };
    assert_eq!(value.nodes.len(), 1);
    assert_eq!(value.nodes[0].event, "event-7");
    assert_eq!(value.nodes[0].depth, 0);
}

#[test]
fn causal_path_does_not_cross_hidden_or_reverse_edges() {
    let hidden_snapshot = snapshot(vec![event(1, 1, &[]), event(3, 3, &[2])]);
    assert_eq!(
        execute_query(
            &hidden_snapshot,
            &EvidenceQueryRequest::CausalPath {
                from: "event-1".into(),
                to: "event-3".into(),
            },
        ),
        Err(QueryError::NoCausalPath {
            from: "event-1".into(),
            to: "event-3".into(),
        })
    );

    let reverse_snapshot = snapshot(vec![event(1, 1, &[]), event(2, 2, &[1])]);
    assert_eq!(
        execute_query(
            &reverse_snapshot,
            &EvidenceQueryRequest::CausalPath {
                from: "event-2".into(),
                to: "event-1".into(),
            },
        ),
        Err(QueryError::NoCausalPath {
            from: "event-2".into(),
            to: "event-1".into(),
        })
    );
}

#[test]
fn causal_path_reuses_canonical_event_validation_for_both_endpoints() {
    let snapshot = snapshot(vec![event(1, 1, &[])]);
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::CausalPath {
                from: SelectionId::Entity(EntityId::new(1)).stable_key(),
                to: "event-1".into(),
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
            &EvidenceQueryRequest::CausalPath {
                from: "event-1".into(),
                to: "event-07".into(),
            },
        ),
        Err(QueryError::InvalidSelectionKey("event-07".into()))
    );
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::CausalPath {
                from: "event-1".into(),
                to: "event-99".into(),
            },
        ),
        Err(QueryError::SelectionNotVisible("event-99".into()))
    );
}
