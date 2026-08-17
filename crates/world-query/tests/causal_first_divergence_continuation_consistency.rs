use std::collections::{BTreeSet, VecDeque};

use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, Difference, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceCausalDirection, EvidenceCausalDivergenceWitness,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum WitnessKey {
    RootPresence(u8),
    Edge(u8, String, String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SearchOutcome {
    depth: Option<usize>,
    witnesses: BTreeSet<WitnessKey>,
}

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

fn difference_rank(difference: Difference) -> u8 {
    match difference {
        Difference::LeftOnly => 0,
        Difference::RightOnly => 1,
        Difference::Changed => 2,
    }
}

fn witness_key(witness: &EvidenceCausalDivergenceWitness) -> WitnessKey {
    match witness {
        EvidenceCausalDivergenceWitness::RootPresence { difference } => {
            WitnessKey::RootPresence(difference_rank(*difference))
        }
        EvidenceCausalDivergenceWitness::Edge {
            difference, edge, ..
        } => WitnessKey::Edge(
            difference_rank(*difference),
            edge.cause.clone(),
            edge.effect.clone(),
        ),
    }
}

fn outcome(value: &world_query::EvidenceCausalFirstDivergenceResult) -> SearchOutcome {
    SearchOutcome {
        depth: value.divergence_depth,
        witnesses: value.witnesses.iter().map(witness_key).collect(),
    }
}

fn segmented_search(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    initial: EvidenceComparisonQueryRequest,
    absolute_depth_limit: usize,
) -> SearchOutcome {
    let mut queue = VecDeque::from([(0usize, initial)]);
    let mut seen = BTreeSet::<(usize, String)>::new();
    let mut best_depth = None;
    let mut best_witnesses = BTreeSet::new();

    while let Some((offset, request)) = queue.pop_front() {
        let serialized = serde_json::to_string(&request).unwrap();
        if !seen.insert((offset, serialized)) {
            continue;
        }
        if best_depth.is_some_and(|best| offset >= best) {
            continue;
        }

        let value = compare(left, right, &request);
        if let Some(relative_depth) = value.divergence_depth {
            let absolute_depth = offset + relative_depth;
            if absolute_depth > absolute_depth_limit {
                continue;
            }
            match best_depth {
                None => {
                    best_depth = Some(absolute_depth);
                    best_witnesses = value.witnesses.iter().map(witness_key).collect();
                }
                Some(best) if absolute_depth < best => {
                    best_depth = Some(absolute_depth);
                    best_witnesses = value.witnesses.iter().map(witness_key).collect();
                }
                Some(best) if absolute_depth == best => {
                    best_witnesses.extend(value.witnesses.iter().map(witness_key));
                }
                Some(_) => {}
            }
            continue;
        }

        for continuation in value.continuations {
            let next_offset = offset + continuation.depth_offset;
            if next_offset < absolute_depth_limit {
                queue.push_back((next_offset, continuation.request));
            }
        }
    }

    SearchOutcome {
        depth: best_depth,
        witnesses: best_witnesses,
    }
}

#[test]
fn segmented_single_frontier_matches_monolithic_first_divergence() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);

    let monolithic = compare(
        &left,
        &right,
        &request("event-3", EvidenceCausalDirection::Upstream, 2),
    );
    let segmented = segmented_search(
        &left,
        &right,
        request("event-3", EvidenceCausalDirection::Upstream, 1),
        2,
    );

    assert_eq!(segmented, outcome(&monolithic));
    assert_eq!(segmented.depth, Some(2));
}

#[test]
fn parallel_frontiers_preserve_global_earliest_depth_and_complete_witness_union() {
    let left = snapshot(vec![
        event(100, 3, &[2, 10]),
        event(2, 2, &[1]),
        event(10, 2, &[3]),
        event(1, 1, &[]),
        event(3, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(100, 3, &[2, 10]),
        event(2, 2, &[9]),
        event(10, 2, &[4]),
        event(9, 1, &[]),
        event(4, 1, &[]),
    ]);

    let monolithic = compare(
        &left,
        &right,
        &request("event-100", EvidenceCausalDirection::Upstream, 2),
    );
    let segmented = segmented_search(
        &left,
        &right,
        request("event-100", EvidenceCausalDirection::Upstream, 1),
        2,
    );

    assert_eq!(segmented, outcome(&monolithic));
    assert_eq!(segmented.depth, Some(2));
    assert_eq!(segmented.witnesses.len(), 4);
}

#[test]
fn zero_depth_bootstrap_composes_to_the_same_absolute_divergence() {
    let left = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[1]), event(1, 1, &[])]);
    let right = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[9]), event(9, 1, &[])]);

    let monolithic = compare(
        &left,
        &right,
        &request("event-3", EvidenceCausalDirection::Upstream, 2),
    );
    let segmented = segmented_search(
        &left,
        &right,
        request("event-3", EvidenceCausalDirection::Upstream, 0),
        2,
    );

    assert_eq!(segmented, outcome(&monolithic));
}

#[test]
fn downstream_segmented_search_matches_monolithic_search() {
    let left = snapshot(vec![
        event(1, 1, &[]),
        event(2, 2, &[1]),
        event(3, 3, &[2]),
    ]);
    let right = snapshot(vec![
        event(1, 1, &[]),
        event(2, 2, &[1]),
        event(4, 3, &[2]),
    ]);

    let monolithic = compare(
        &left,
        &right,
        &request("event-1", EvidenceCausalDirection::Downstream, 2),
    );
    let segmented = segmented_search(
        &left,
        &right,
        request("event-1", EvidenceCausalDirection::Downstream, 1),
        2,
    );

    assert_eq!(segmented, outcome(&monolithic));
}

#[test]
fn hidden_references_and_cycles_do_not_create_segmented_false_positives() {
    let left = snapshot(vec![
        event(1, 1, &[2, 99]),
        event(2, 2, &[1]),
    ]);
    let right = snapshot(vec![event(1, 1, &[2]), event(2, 2, &[1])]);

    let monolithic = compare(
        &left,
        &right,
        &request("event-1", EvidenceCausalDirection::Upstream, 4),
    );
    let segmented = segmented_search(
        &left,
        &right,
        request("event-1", EvidenceCausalDirection::Upstream, 1),
        4,
    );

    assert_eq!(segmented, outcome(&monolithic));
    assert_eq!(segmented.depth, None);
    assert!(segmented.witnesses.is_empty());
}
