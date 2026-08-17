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

fn neighborhood(snapshot: &ProjectionSnapshot) -> Vec<EvidenceCausalEdge> {
    let response = execute_query(
        snapshot,
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
    value.edges
}

#[test]
fn causal_neighborhood_exports_the_full_induced_visible_subgraph_not_only_tree_edges() {
    let snapshot = snapshot(vec![
        event(5, 5, &[4, 2]),
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);

    assert_eq!(
        neighborhood(&snapshot),
        vec![
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-4".into(),
                effect: "event-5".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-5".into(),
            },
        ]
    );
}

#[test]
fn edge_order_is_independent_of_timeline_input_order() {
    let left = snapshot(vec![
        event(5, 5, &[4, 2]),
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(2, 2, &[]),
        event(5, 5, &[4, 2]),
        event(1, 1, &[]),
        event(4, 4, &[3, 2]),
        event(3, 3, &[1]),
    ]);

    assert_eq!(neighborhood(&left), neighborhood(&right));
}

#[test]
fn hidden_and_out_of_window_endpoints_do_not_leak_into_edges() {
    let snapshot = snapshot(vec![
        event(6, 6, &[5]),
        event(5, 5, &[4, 2, 99]),
        event(4, 4, &[3]),
        event(3, 3, &[1]),
        event(2, 2, &[]),
        event(1, 1, &[]),
    ]);

    assert_eq!(
        neighborhood(&snapshot),
        vec![
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-4".into(),
                effect: "event-5".into(),
            },
        ]
    );
}

#[test]
fn duplicate_persisted_parent_ids_emit_one_graph_edge_at_first_parent_position() {
    let snapshot = snapshot(vec![
        event(5, 5, &[4, 2, 4]),
        event(4, 4, &[3, 3, 2]),
        event(3, 3, &[]),
        event(2, 2, &[]),
    ]);

    assert_eq!(
        neighborhood(&snapshot),
        vec![
            EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-4".into(),
            },
            EvidenceCausalEdge {
                cause: "event-4".into(),
                effect: "event-5".into(),
            },
            EvidenceCausalEdge {
                cause: "event-2".into(),
                effect: "event-5".into(),
            },
        ]
    );
}

#[test]
fn legacy_m196_response_deserializes_with_empty_edges() {
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
            "downstream":[],
            "upstream_truncated":false,
            "downstream_truncated":false,
            "upstream_frontier":[],
            "downstream_frontier":[]
        }
    }"#;
    let response: EvidenceQueryResponse = serde_json::from_str(json).unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };

    assert!(value.edges.is_empty());
}
