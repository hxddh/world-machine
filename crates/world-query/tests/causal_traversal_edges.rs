use world_core::EventId;
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

#[test]
fn why_exports_the_full_visible_ancestry_graph() {
    let snapshot = snapshot(vec![
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::Why {
            event: "event-4".into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::Why { value } = response else {
        panic!("expected why response")
    };

    assert_eq!(
        value.edges,
        vec![
            EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-3".into(),
            },
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-4".into(),
            },
        ]
    );
}

#[test]
fn influence_edges_exclude_visible_external_co_causes_outside_the_descendant_closure() {
    let snapshot = snapshot(vec![
        event(9, 9, &[]),
        event(4, 4, &[2, 3, 9]),
        event(3, 3, &[1]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::Influence {
            event: "event-1".into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::Influence { value } = response else {
        panic!("expected influence response")
    };

    let event4 = value
        .nodes
        .iter()
        .find(|node| node.event == "event-4")
        .expect("event-4 should be influenced");
    assert_eq!(event4.caused_by, vec!["event-2", "event-3", "event-9"]);
    assert_eq!(
        value.edges,
        vec![
            EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-2".into(),
            },
            EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-3".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
        ]
    );
    assert!(!value.edges.iter().any(|edge| edge.cause == "event-9"));
}

#[test]
fn traversal_edge_order_is_stable_despite_timeline_input_order() {
    let left = snapshot(vec![
        event(4, 4, &[2, 3]),
        event(3, 3, &[1]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(2, 2, &[1]),
        event(4, 4, &[2, 3]),
        event(1, 1, &[]),
        event(3, 3, &[1]),
    ]);

    let why_edges = |snapshot: &ProjectionSnapshot| {
        let response = execute_query(
            snapshot,
            &EvidenceQueryRequest::Why {
                event: "event-4".into(),
            },
        )
        .unwrap();
        let EvidenceQueryResponse::Why { value } = response else {
            panic!("expected why response")
        };
        value.edges
    };
    let influence_edges = |snapshot: &ProjectionSnapshot| {
        let response = execute_query(
            snapshot,
            &EvidenceQueryRequest::Influence {
                event: "event-1".into(),
            },
        )
        .unwrap();
        let EvidenceQueryResponse::Influence { value } = response else {
            panic!("expected influence response")
        };
        value.edges
    };

    assert_eq!(why_edges(&left), why_edges(&right));
    assert_eq!(influence_edges(&left), influence_edges(&right));
}

#[test]
fn legacy_traversal_responses_deserialize_with_empty_edges() {
    let why_json = r#"{
        "result":"why",
        "value":{
            "event":"event-1",
            "nodes":[{
                "event":"event-1",
                "depth":0,
                "world_time":1,
                "title":"Event 1",
                "subtitle":"world time 1",
                "caused_by":[]
            }]
        }
    }"#;
    let influence_json = r#"{
        "result":"influence",
        "value":{
            "event":"event-1",
            "nodes":[{
                "event":"event-1",
                "depth":0,
                "world_time":1,
                "title":"Event 1",
                "subtitle":"world time 1",
                "caused_by":[]
            }]
        }
    }"#;

    let why: EvidenceQueryResponse = serde_json::from_str(why_json).unwrap();
    let EvidenceQueryResponse::Why { value } = why else {
        panic!("expected why response")
    };
    assert!(value.edges.is_empty());

    let influence: EvidenceQueryResponse = serde_json::from_str(influence_json).unwrap();
    let EvidenceQueryResponse::Influence { value } = influence else {
        panic!("expected influence response")
    };
    assert!(value.edges.is_empty());
}
