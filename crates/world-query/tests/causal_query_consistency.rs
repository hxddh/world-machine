use std::collections::{BTreeMap, BTreeSet};

use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_query, EvidenceCausalNeighborhoodResult, EvidenceInfluenceResult, EvidenceQueryRequest,
    EvidenceQueryResponse, EvidenceWhyResult, QueryError,
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

fn fixture_items() -> Vec<TimelineItem> {
    vec![
        event(13, 13, &[10]),
        event(12, 12, &[11]),
        event(11, 11, &[10]),
        event(10, 10, &[12]),
        event(8, 1, &[]),
        event(6, 6, &[5]),
        event(5, 5, &[4]),
        event(4, 4, &[3, 2, 8, 99]),
        event(3, 2, &[1]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]
}

fn snapshot(items: Vec<TimelineItem>) -> ProjectionSnapshot {
    ProjectionSnapshot {
        timeline: TimelineProjection { items },
        ..ProjectionSnapshot::default()
    }
}

fn fixture() -> ProjectionSnapshot {
    snapshot(fixture_items())
}

fn shuffled_fixture() -> ProjectionSnapshot {
    let mut items = fixture_items();
    items.rotate_left(4);
    items.reverse();
    snapshot(items)
}

fn visible_events(snapshot: &ProjectionSnapshot) -> Vec<String> {
    snapshot
        .timeline
        .items
        .iter()
        .map(|item| item.id.stable_key())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn why(snapshot: &ProjectionSnapshot, event: &str) -> EvidenceWhyResult {
    let response = execute_query(
        snapshot,
        &EvidenceQueryRequest::Why {
            event: event.into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::Why { value } = response else {
        panic!("expected why response")
    };
    value
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

fn causal_neighborhood(
    snapshot: &ProjectionSnapshot,
    root: &str,
    upstream_depth: usize,
    downstream_depth: usize,
) -> EvidenceCausalNeighborhoodResult {
    let response = execute_query(
        snapshot,
        &EvidenceQueryRequest::CausalNeighborhood {
            root: root.into(),
            upstream_depth,
            downstream_depth,
        },
    )
    .unwrap();
    let EvidenceQueryResponse::CausalNeighborhood { value } = response else {
        panic!("expected causal-neighborhood response")
    };
    value
}

fn node_events<'a>(
    nodes: impl IntoIterator<Item = &'a world_query::EvidenceCausalNode>,
) -> BTreeSet<String> {
    nodes.into_iter().map(|node| node.event.clone()).collect()
}

#[test]
fn influence_and_why_are_dual_reachability_relations_and_match_causal_path() {
    let snapshot = fixture();
    let events = visible_events(&snapshot);
    let why_by_event = events
        .iter()
        .map(|event| (event.clone(), why(&snapshot, event)))
        .collect::<BTreeMap<_, _>>();
    let influence_by_event = events
        .iter()
        .map(|event| (event.clone(), influence(&snapshot, event)))
        .collect::<BTreeMap<_, _>>();

    for from in &events {
        let influenced = node_events(&influence_by_event[from].nodes);
        for to in &events {
            let reachable = influenced.contains(to);
            let ancestry = node_events(&why_by_event[to].nodes);
            assert_eq!(
                reachable,
                ancestry.contains(from),
                "reachability duality disagreed for {from} -> {to}"
            );

            let path = execute_query(
                &snapshot,
                &EvidenceQueryRequest::CausalPath {
                    from: from.clone(),
                    to: to.clone(),
                },
            );
            assert_eq!(
                path.is_ok(),
                reachable,
                "causal-path reachability disagreed for {from} -> {to}"
            );

            match path {
                Ok(EvidenceQueryResponse::CausalPath { value }) => {
                    assert_eq!(value.nodes.first().unwrap().event, *from);
                    assert_eq!(value.nodes.last().unwrap().event, *to);
                    assert_eq!(
                        value
                            .nodes
                            .iter()
                            .map(|node| node.depth)
                            .collect::<Vec<_>>(),
                        (0..value.nodes.len()).collect::<Vec<_>>()
                    );
                    let edges = influence_by_event[from]
                        .edges
                        .iter()
                        .map(|edge| (edge.cause.as_str(), edge.effect.as_str()))
                        .collect::<BTreeSet<_>>();
                    for pair in value.nodes.windows(2) {
                        assert!(
                            edges.contains(&(pair[0].event.as_str(), pair[1].event.as_str())),
                            "path step {} -> {} was not a persisted causal edge",
                            pair[0].event,
                            pair[1].event
                        );
                    }
                }
                Err(QueryError::NoCausalPath {
                    from: error_from,
                    to: error_to,
                }) => {
                    assert!(!reachable);
                    assert_eq!(error_from, *from);
                    assert_eq!(error_to, *to);
                }
                Ok(other) => panic!("unexpected path response: {other:?}"),
                Err(error) => panic!("unexpected path error: {error:?}"),
            }
        }
    }
}

fn assert_neighborhood_matches_traversal_prefixes(
    snapshot: &ProjectionSnapshot,
    root: &str,
    upstream_depth: usize,
    downstream_depth: usize,
) {
    let neighborhood = causal_neighborhood(snapshot, root, upstream_depth, downstream_depth);
    let ancestry = why(snapshot, root);
    let descendants = influence(snapshot, root);

    assert_eq!(neighborhood.root, ancestry.nodes[0]);
    assert_eq!(neighborhood.root, descendants.nodes[0]);
    assert_eq!(
        neighborhood
            .upstream
            .iter()
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>(),
        ancestry
            .nodes
            .iter()
            .filter(|node| node.depth > 0 && node.depth <= upstream_depth)
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        neighborhood
            .downstream
            .iter()
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>(),
        descendants
            .nodes
            .iter()
            .filter(|node| node.depth > 0 && node.depth <= downstream_depth)
            .map(|node| (node.event.as_str(), node.depth))
            .collect::<Vec<_>>()
    );
}

#[test]
fn causal_neighborhood_is_exactly_the_bounded_prefix_of_why_and_influence() {
    let snapshot = fixture();
    for (root, upstream_depth, downstream_depth) in [
        ("event-4", 0, 0),
        ("event-4", 1, 1),
        ("event-4", 2, 2),
        ("event-10", 1, 1),
        ("event-10", 8, 8),
    ] {
        assert_neighborhood_matches_traversal_prefixes(
            &snapshot,
            root,
            upstream_depth,
            downstream_depth,
        );
    }
}

#[test]
fn causal_neighborhood_edges_are_exactly_the_induced_graph_of_returned_nodes() {
    let snapshot = fixture();
    for (root, upstream_depth, downstream_depth) in
        [("event-4", 1, 1), ("event-4", 2, 2), ("event-10", 8, 8)]
    {
        let neighborhood = causal_neighborhood(&snapshot, root, upstream_depth, downstream_depth);
        let all_nodes = std::iter::once(&neighborhood.root)
            .chain(neighborhood.upstream.iter())
            .chain(neighborhood.downstream.iter())
            .collect::<Vec<_>>();
        let included = node_events(all_nodes.iter().copied());
        let expected = all_nodes
            .iter()
            .flat_map(|node| {
                node.caused_by.iter().filter_map(|cause| {
                    included
                        .contains(cause)
                        .then_some((cause.clone(), node.event.clone()))
                })
            })
            .collect::<BTreeSet<_>>();
        let actual = neighborhood
            .edges
            .iter()
            .map(|edge| (edge.cause.clone(), edge.effect.clone()))
            .collect::<BTreeSet<_>>();

        assert_eq!(actual, expected, "induced edges disagreed for {root}");
        assert_eq!(actual.len(), neighborhood.edges.len());
    }
}

fn expected_upstream_frontier(neighborhood: &EvidenceCausalNeighborhoodResult) -> Vec<String> {
    let included = std::iter::once(neighborhood.root.event.clone())
        .chain(neighborhood.upstream.iter().map(|node| node.event.clone()))
        .collect::<BTreeSet<_>>();
    let boundary = if neighborhood.upstream_depth == 0 {
        vec![&neighborhood.root]
    } else {
        neighborhood
            .upstream
            .iter()
            .filter(|node| node.depth == neighborhood.upstream_depth)
            .collect::<Vec<_>>()
    };
    boundary
        .into_iter()
        .filter(|node| node.caused_by.iter().any(|cause| !included.contains(cause)))
        .map(|node| node.event.clone())
        .collect()
}

fn expected_downstream_frontier(
    neighborhood: &EvidenceCausalNeighborhoodResult,
    descendants: &EvidenceInfluenceResult,
) -> Vec<String> {
    let included = std::iter::once(neighborhood.root.event.clone())
        .chain(
            neighborhood
                .downstream
                .iter()
                .map(|node| node.event.clone()),
        )
        .collect::<BTreeSet<_>>();
    let boundary = if neighborhood.downstream_depth == 0 {
        vec![neighborhood.root.event.clone()]
    } else {
        neighborhood
            .downstream
            .iter()
            .filter(|node| node.depth == neighborhood.downstream_depth)
            .map(|node| node.event.clone())
            .collect::<Vec<_>>()
    };
    boundary
        .into_iter()
        .filter(|event| {
            descendants
                .edges
                .iter()
                .any(|edge| edge.cause == *event && !included.contains(&edge.effect))
        })
        .collect()
}

#[test]
fn frontier_and_truncation_exactly_describe_omitted_visible_neighbors() {
    let snapshot = fixture();
    for (root, upstream_depth, downstream_depth) in [
        ("event-4", 0, 0),
        ("event-4", 1, 1),
        ("event-4", 2, 2),
        ("event-10", 1, 1),
        ("event-10", 8, 8),
    ] {
        let neighborhood = causal_neighborhood(&snapshot, root, upstream_depth, downstream_depth);
        let descendants = influence(&snapshot, root);
        let expected_upstream = expected_upstream_frontier(&neighborhood);
        let expected_downstream = expected_downstream_frontier(&neighborhood, &descendants);

        assert_eq!(neighborhood.upstream_frontier, expected_upstream);
        assert_eq!(neighborhood.downstream_frontier, expected_downstream);
        assert_eq!(
            neighborhood.upstream_truncated,
            !neighborhood.upstream_frontier.is_empty()
        );
        assert_eq!(
            neighborhood.downstream_truncated,
            !neighborhood.downstream_frontier.is_empty()
        );
    }
}

#[test]
fn hidden_references_never_surface_through_any_causal_query() {
    let snapshot = fixture();
    let events = visible_events(&snapshot);

    for event in &events {
        for request in [
            EvidenceQueryRequest::Why {
                event: event.clone(),
            },
            EvidenceQueryRequest::Influence {
                event: event.clone(),
            },
            EvidenceQueryRequest::CausalNeighborhood {
                root: event.clone(),
                upstream_depth: 8,
                downstream_depth: 8,
            },
        ] {
            let response = execute_query(&snapshot, &request).unwrap();
            assert!(!serde_json::to_string(&response)
                .unwrap()
                .contains("event-99"));
        }
    }

    for from in &events {
        for to in &events {
            if let Ok(response) = execute_query(
                &snapshot,
                &EvidenceQueryRequest::CausalPath {
                    from: from.clone(),
                    to: to.clone(),
                },
            ) {
                assert!(!serde_json::to_string(&response)
                    .unwrap()
                    .contains("event-99"));
            }
        }
    }
}

#[test]
fn cycles_do_not_duplicate_nodes_or_reinsert_the_root() {
    let snapshot = fixture();
    let ancestry = why(&snapshot, "event-10");
    let descendants = influence(&snapshot, "event-10");
    let neighborhood = causal_neighborhood(&snapshot, "event-10", 8, 8);

    let ancestry_events = ancestry
        .nodes
        .iter()
        .map(|node| node.event.as_str())
        .collect::<BTreeSet<_>>();
    let descendant_events = descendants
        .nodes
        .iter()
        .map(|node| node.event.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(ancestry_events.len(), ancestry.nodes.len());
    assert_eq!(descendant_events.len(), descendants.nodes.len());
    assert_eq!(
        ancestry
            .nodes
            .iter()
            .filter(|node| node.event == "event-10")
            .count(),
        1
    );
    assert_eq!(
        descendants
            .nodes
            .iter()
            .filter(|node| node.event == "event-10")
            .count(),
        1
    );
    assert!(!neighborhood
        .upstream
        .iter()
        .any(|node| node.event == "event-10"));
    assert!(!neighborhood
        .downstream
        .iter()
        .any(|node| node.event == "event-10"));

    let response = execute_query(
        &snapshot,
        &EvidenceQueryRequest::CausalPath {
            from: "event-10".into(),
            to: "event-12".into(),
        },
    )
    .unwrap();
    let EvidenceQueryResponse::CausalPath { value } = response else {
        panic!("expected causal-path response")
    };
    assert_eq!(
        value
            .nodes
            .iter()
            .map(|node| node.event.as_str())
            .collect::<Vec<_>>(),
        vec!["event-10", "event-11", "event-12"]
    );
}

#[test]
fn causal_query_responses_are_stable_under_timeline_input_reordering() {
    let left = fixture();
    let right = shuffled_fixture();
    let requests = [
        EvidenceQueryRequest::Why {
            event: "event-6".into(),
        },
        EvidenceQueryRequest::Influence {
            event: "event-1".into(),
        },
        EvidenceQueryRequest::CausalPath {
            from: "event-1".into(),
            to: "event-6".into(),
        },
        EvidenceQueryRequest::CausalNeighborhood {
            root: "event-4".into(),
            upstream_depth: 2,
            downstream_depth: 2,
        },
        EvidenceQueryRequest::CausalNeighborhood {
            root: "event-10".into(),
            upstream_depth: 8,
            downstream_depth: 8,
        },
    ];

    for request in requests {
        assert_eq!(
            execute_query(&left, &request),
            execute_query(&right, &request),
            "response changed when timeline input order changed for {request:?}"
        );
    }
}
