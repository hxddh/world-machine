use serde_json::json;
use world_core::{EntityId, EventId};
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, Difference, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceCausalDirection, EvidenceCausalDivergenceWitness,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse, EvidenceSelectionKind,
    QueryError,
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
    root: &str,
    direction: EvidenceCausalDirection,
    max_depth: usize,
) -> world_query::EvidenceCausalFirstDivergenceResult {
    let response =
        execute_comparison_query_request(left, right, &request(root, direction, max_depth))
            .unwrap();
    let EvidenceComparisonQueryResponse::Causal(
        EvidenceCausalComparisonResponse::FirstDivergence { value },
    ) = response
    else {
        panic!("expected first-divergence response")
    };
    value
}

#[test]
fn first_divergence_has_additive_tagged_protocol_v1_shape() {
    let value = json!({
        "query":"first-divergence",
        "root":"event-3",
        "direction":"upstream",
        "max_depth":2
    });
    let parsed: EvidenceComparisonQueryRequest = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(
        parsed,
        request("event-3", EvidenceCausalDirection::Upstream, 2)
    );
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
}

#[test]
fn downstream_reports_only_the_earliest_differing_edge_layer() {
    let left = snapshot(vec![event(1, 1, &[]), event(2, 2, &[1]), event(3, 3, &[2])]);
    let right = snapshot(vec![event(1, 1, &[]), event(4, 2, &[1]), event(5, 3, &[4])]);

    let value = compare(
        &left,
        &right,
        "event-1",
        EvidenceCausalDirection::Downstream,
        2,
    );
    assert!(!value.identical_within_depth);
    assert_eq!(value.divergence_depth, Some(1));
    assert_eq!(value.witnesses.len(), 2);
    assert!(value.witnesses.iter().all(|witness| matches!(
        witness,
        EvidenceCausalDivergenceWitness::Edge { edge, .. }
            if edge.cause == "event-1" && (edge.effect == "event-2" || edge.effect == "event-4")
    )));
    assert!(value.witnesses.iter().all(|witness| !matches!(
        witness,
        EvidenceCausalDivergenceWitness::Edge { edge, .. }
            if edge.effect == "event-3" || edge.effect == "event-5"
    )));
}

#[test]
fn earliest_witnesses_use_typed_event_order_not_lexical_keys() {
    let left = snapshot(vec![
        event(100, 3, &[10, 2]),
        event(10, 2, &[]),
        event(2, 1, &[]),
    ]);
    let right = snapshot(vec![event(100, 3, &[])]);
    let value = compare(
        &left,
        &right,
        "event-100",
        EvidenceCausalDirection::Upstream,
        1,
    );

    assert_eq!(value.divergence_depth, Some(1));
    let causes = value
        .witnesses
        .iter()
        .map(|witness| match witness {
            EvidenceCausalDivergenceWitness::Edge { difference, edge } => {
                assert_eq!(*difference, Difference::LeftOnly);
                edge.cause.as_str()
            }
            EvidenceCausalDivergenceWitness::RootPresence { .. } => panic!("expected edge witness"),
        })
        .collect::<Vec<_>>();
    assert_eq!(causes, vec!["event-2", "event-10"]);
}

#[test]
fn bounded_identical_result_exposes_frontier_until_deeper_divergence_is_in_scope() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[9]), event(9, 1, &[])]);

    let shallow = compare(
        &left,
        &right,
        "event-3",
        EvidenceCausalDirection::Upstream,
        1,
    );
    assert!(shallow.identical_within_depth);
    assert_eq!(shallow.divergence_depth, None);
    assert!(shallow.witnesses.is_empty());
    assert_eq!(shallow.left_frontier, vec!["event-2"]);
    assert_eq!(shallow.right_frontier, vec!["event-2"]);

    let deep = compare(
        &left,
        &right,
        "event-3",
        EvidenceCausalDirection::Upstream,
        2,
    );
    assert!(!deep.identical_within_depth);
    assert_eq!(deep.divergence_depth, Some(2));
}

#[test]
fn one_sided_root_is_a_depth_zero_divergence() {
    let left = snapshot(vec![event(1, 1, &[])]);
    let right = snapshot(vec![event(2, 2, &[])]);
    let value = compare(
        &left,
        &right,
        "event-1",
        EvidenceCausalDirection::Downstream,
        3,
    );

    assert_eq!(value.divergence_depth, Some(0));
    assert_eq!(
        value.witnesses,
        vec![EvidenceCausalDivergenceWitness::RootPresence {
            difference: Difference::LeftOnly,
        }]
    );
}

#[test]
fn hidden_causal_references_do_not_create_false_divergence() {
    let hidden = snapshot(vec![event(3, 3, &[99])]);
    let plain = snapshot(vec![event(3, 3, &[])]);
    let value = compare(
        &hidden,
        &plain,
        "event-3",
        EvidenceCausalDirection::Upstream,
        3,
    );
    assert!(value.identical_within_depth);
    assert_eq!(value.divergence_depth, None);
    assert!(value.witnesses.is_empty());
}

#[test]
fn first_divergence_preserves_event_root_error_contract() {
    let left = snapshot(vec![event(1, 1, &[])]);
    let right = snapshot(vec![event(2, 2, &[])]);
    assert_eq!(
        execute_comparison_query_request(
            &left,
            &right,
            &request(
                &SelectionId::Entity(EntityId::new(1)).stable_key(),
                EvidenceCausalDirection::Upstream,
                1
            ),
        ),
        Err(QueryError::SelectionKindMismatch {
            selection: "entity-1".into(),
            expected: EvidenceSelectionKind::Event,
        })
    );
    assert_eq!(
        execute_comparison_query_request(
            &left,
            &right,
            &request("event-9", EvidenceCausalDirection::Upstream, 1),
        ),
        Err(QueryError::SelectionNotVisibleInEitherWorld(
            "event-9".into()
        ))
    );
}
