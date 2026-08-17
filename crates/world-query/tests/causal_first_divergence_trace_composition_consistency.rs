use std::collections::{BTreeSet, VecDeque};

use world_core::EventId;
use world_projection::{ProjectionSnapshot, SelectionId, TimelineItem, TimelineProjection};
use world_query::{
    execute_comparison_query_request, Difference, EvidenceCausalComparisonRequest,
    EvidenceCausalComparisonResponse, EvidenceCausalDirection, EvidenceCausalDivergenceWitness,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct EdgeWitness {
    difference: u8,
    cause: String,
    effect: String,
    trace: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SearchOutcome {
    depth: Option<usize>,
    witnesses: BTreeSet<EdgeWitness>,
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

fn compose_trace(prefix: &[String], suffix: &[String]) -> Vec<String> {
    if prefix.is_empty() {
        return suffix.to_vec();
    }
    if suffix.is_empty() {
        return prefix.to_vec();
    }
    assert_eq!(prefix.last(), suffix.first());
    let mut composed = prefix.to_vec();
    composed.extend(suffix.iter().skip(1).cloned());
    composed
}

fn edge_witnesses(
    witnesses: &[EvidenceCausalDivergenceWitness],
    prefix: &[String],
) -> BTreeSet<EdgeWitness> {
    witnesses
        .iter()
        .filter_map(|witness| match witness {
            EvidenceCausalDivergenceWitness::Edge {
                difference,
                edge,
                trace,
            } => Some(EdgeWitness {
                difference: difference_rank(*difference),
                cause: edge.cause.clone(),
                effect: edge.effect.clone(),
                trace: compose_trace(prefix, trace),
            }),
            EvidenceCausalDivergenceWitness::RootPresence { .. } => None,
        })
        .collect()
}

fn monolithic_outcome(value: &world_query::EvidenceCausalFirstDivergenceResult) -> SearchOutcome {
    SearchOutcome {
        depth: value.divergence_depth,
        witnesses: edge_witnesses(&value.witnesses, &[]),
    }
}

fn segmented_search(
    left: &ProjectionSnapshot,
    right: &ProjectionSnapshot,
    initial: EvidenceComparisonQueryRequest,
    absolute_depth_limit: usize,
) -> SearchOutcome {
    let mut queue = VecDeque::from([(0usize, Vec::<String>::new(), initial)]);
    let mut seen = BTreeSet::<(usize, String)>::new();
    let mut best_depth = None;
    let mut best_witnesses = BTreeSet::new();

    while let Some((offset, prefix, request)) = queue.pop_front() {
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
            let witnesses = edge_witnesses(&value.witnesses, &prefix);
            match best_depth {
                None => {
                    best_depth = Some(absolute_depth);
                    best_witnesses = witnesses;
                }
                Some(best) if absolute_depth < best => {
                    best_depth = Some(absolute_depth);
                    best_witnesses = witnesses;
                }
                Some(best) if absolute_depth == best => {
                    best_witnesses.extend(witnesses);
                }
                Some(_) => {}
            }
            continue;
        }

        for continuation in value.continuations {
            let next_offset = offset + continuation.depth_offset;
            if next_offset >= absolute_depth_limit {
                continue;
            }
            let next_prefix = compose_trace(&prefix, &continuation.trace_prefix);
            queue.push_back((next_offset, next_prefix, continuation.request));
        }
    }

    SearchOutcome {
        depth: best_depth,
        witnesses: best_witnesses,
    }
}

#[test]
fn three_window_upstream_replay_matches_monolithic_depth_witness_and_trace() {
    let left = snapshot(vec![
        event(4, 4, &[3]),
        event(3, 3, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![event(4, 4, &[3]), event(3, 3, &[2]), event(2, 2, &[])]);

    let monolithic = compare(
        &left,
        &right,
        &request("event-4", EvidenceCausalDirection::Upstream, 3),
    );
    let segmented = segmented_search(
        &left,
        &right,
        request("event-4", EvidenceCausalDirection::Upstream, 1),
        3,
    );

    assert_eq!(segmented, monolithic_outcome(&monolithic));
    assert_eq!(segmented.depth, Some(3));
    let witness = segmented.witnesses.iter().next().unwrap();
    assert_eq!(
        witness.trace,
        vec!["event-4", "event-3", "event-2", "event-1"]
    );
}

#[test]
fn three_window_downstream_replay_matches_monolithic_depth_witness_and_trace() {
    let left = snapshot(vec![
        event(1, 1, &[]),
        event(2, 2, &[1]),
        event(3, 3, &[2]),
        event(4, 4, &[3]),
    ]);
    let right = snapshot(vec![event(1, 1, &[]), event(2, 2, &[1]), event(3, 3, &[2])]);

    let monolithic = compare(
        &left,
        &right,
        &request("event-1", EvidenceCausalDirection::Downstream, 3),
    );
    let segmented = segmented_search(
        &left,
        &right,
        request("event-1", EvidenceCausalDirection::Downstream, 1),
        3,
    );

    assert_eq!(segmented, monolithic_outcome(&monolithic));
    assert_eq!(segmented.depth, Some(3));
    let witness = segmented.witnesses.iter().next().unwrap();
    assert_eq!(
        witness.trace,
        vec!["event-1", "event-2", "event-3", "event-4"]
    );
}

#[test]
fn parallel_frontiers_preserve_same_depth_witnesses_with_original_root_traces() {
    let left = snapshot(vec![
        event(100, 4, &[20, 30]),
        event(20, 3, &[2]),
        event(30, 3, &[3]),
        event(2, 2, &[1]),
        event(3, 2, &[4]),
        event(1, 1, &[]),
        event(4, 1, &[]),
    ]);
    let right = snapshot(vec![
        event(100, 4, &[20, 30]),
        event(20, 3, &[2]),
        event(30, 3, &[3]),
        event(2, 2, &[9]),
        event(3, 2, &[8]),
        event(9, 1, &[]),
        event(8, 1, &[]),
    ]);

    let monolithic = compare(
        &left,
        &right,
        &request("event-100", EvidenceCausalDirection::Upstream, 3),
    );
    let segmented = segmented_search(
        &left,
        &right,
        request("event-100", EvidenceCausalDirection::Upstream, 1),
        3,
    );

    assert_eq!(segmented, monolithic_outcome(&monolithic));
    assert_eq!(segmented.depth, Some(3));
    assert_eq!(segmented.witnesses.len(), 4);
    assert!(segmented
        .witnesses
        .iter()
        .all(|witness| witness.trace.first().map(String::as_str) == Some("event-100")));
}

#[test]
fn zero_depth_bootstrap_does_not_duplicate_the_root_in_composed_trace() {
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
        request("event-3", EvidenceCausalDirection::Upstream, 0),
        2,
    );

    assert_eq!(segmented, monolithic_outcome(&monolithic));
    let witness = segmented.witnesses.iter().next().unwrap();
    assert_eq!(witness.trace, vec!["event-3", "event-2", "event-1"]);
}

#[test]
fn diamond_prefix_choice_remains_stable_across_multiple_replay_windows() {
    let left = snapshot(vec![
        event(1, 1, &[]),
        event(2, 2, &[1]),
        event(3, 2, &[1]),
        event(4, 3, &[2, 3]),
        event(5, 4, &[4]),
    ]);
    let right = snapshot(vec![
        event(1, 1, &[]),
        event(2, 2, &[1]),
        event(3, 2, &[1]),
        event(4, 3, &[2, 3]),
    ]);

    let monolithic = compare(
        &left,
        &right,
        &request("event-1", EvidenceCausalDirection::Downstream, 3),
    );
    let segmented = segmented_search(
        &left,
        &right,
        request("event-1", EvidenceCausalDirection::Downstream, 1),
        3,
    );

    assert_eq!(segmented, monolithic_outcome(&monolithic));
    let witness = segmented.witnesses.iter().next().unwrap();
    assert_eq!(
        witness.trace,
        vec!["event-1", "event-2", "event-4", "event-5"]
    );
}
