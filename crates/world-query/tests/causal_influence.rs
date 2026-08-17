use world_core::{EntityId, EventId};
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_query, EvidenceInfluenceResult, EvidenceQueryRequest, EvidenceQueryResponse,
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

fn influence(snapshot: &ProjectionSnapshot, event: &str) -> EvidenceInfluenceResult {
    let response = execute_query(
        snapshot,
        &EvidenceQueryRequest::Influence {
            event: event.into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::Influence { value } = response else {
        panic!("expected influence response")
    };
    value
}

#[test]
fn serialized_influence_traverses_visible_descendants_at_minimum_depth() {
    let snapshot = snapshot(vec![
        event(5, 5, &[4, 1]),
        event(3, 3, &[1, 99]),
        event(4, 4, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[4]),
    ]);
    let request: EvidenceQueryRequest =
        serde_json::from_str(r#"{"query":"influence","event":"event-1"}"#).unwrap();
    let response = execute_query(&snapshot, &request).unwrap();
    let json = serde_json::to_string(&response).unwrap();
    let restored: EvidenceQueryResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, response);

    let EvidenceQueryResponse::Influence { value } = response else {
        panic!("expected influence response")
    };
    assert_eq!(value.event, "event-1");
    assert_eq!(
        value
            .nodes
            .iter()
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>(),
        vec![
            ("event-1", 0),
            ("event-2", 1),
            ("event-3", 1),
            ("event-5", 1),
            ("event-4", 2),
        ]
    );
    assert_eq!(value.nodes[2].caused_by, vec!["event-1"]);
    assert_eq!(value.nodes[3].caused_by, vec!["event-4", "event-1"]);
    assert_eq!(
        value
            .nodes
            .iter()
            .filter(|node| node.event == "event-1")
            .count(),
        1
    );
}

#[test]
fn direct_children_use_world_time_then_selection_id_not_timeline_order() {
    let snapshot = snapshot(vec![event(3, 2, &[1]), event(2, 2, &[1]), event(1, 1, &[])]);
    let value = influence(&snapshot, "event-1");
    assert_eq!(
        value
            .nodes
            .iter()
            .map(|node| node.event.as_str())
            .collect::<Vec<_>>(),
        vec!["event-1", "event-2", "event-3"]
    );
}

#[test]
fn influence_enforces_event_kind_canonical_keys_and_timeline_visibility() {
    let snapshot = snapshot(vec![event(1, 1, &[])]);
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::Influence {
                event: SelectionId::Entity(EntityId::new(1)).stable_key(),
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
            &EvidenceQueryRequest::Influence {
                event: "event-07".into(),
            },
        ),
        Err(QueryError::InvalidSelectionKey("event-07".into()))
    );
    assert_eq!(
        execute_query(
            &snapshot,
            &EvidenceQueryRequest::Influence {
                event: "event-99".into(),
            },
        ),
        Err(QueryError::SelectionNotVisible("event-99".into()))
    );
}
