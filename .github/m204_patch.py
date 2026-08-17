from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


lib_path = Path("crates/world-query/src/lib.rs")
lib = lib_path.read_text()

lib = replace_once(
    lib,
    '''    Edge {
        difference: Difference,
        edge: EvidenceCausalEdge,
    },
''',
    '''    Edge {
        difference: Difference,
        edge: EvidenceCausalEdge,
        #[serde(default)]
        trace: Vec<String>,
    },
''',
    "edge witness trace field",
)

lib = replace_once(
    lib,
    '''    let witnesses = candidates
        .into_iter()
        .map(|(_, difference, edge)| EvidenceCausalDivergenceWitness::Edge { difference, edge })
        .collect();
''',
    '''    let witnesses = candidates
        .into_iter()
        .map(|(_, difference, edge)| {
            let (graph, positions) = match difference {
                Difference::LeftOnly => (&left_graph, &left_positions),
                Difference::RightOnly => (&right_graph, &right_positions),
                Difference::Changed => {
                    unreachable!("causal edge set difference cannot produce changed witness")
                }
            };
            let trace = causal_divergence_trace(graph, root, &edge, positions, direction);
            EvidenceCausalDivergenceWitness::Edge {
                difference,
                edge,
                trace,
            }
        })
        .collect();
''',
    "witness trace construction",
)

marker = '''fn directional_causal_frontier(
    neighborhood: Option<&EvidenceCausalNeighborhoodResult>,
'''
helpers = r'''fn causal_divergence_trace(
    graph: &VisibleCausalGraph<'_>,
    root: SelectionId,
    edge: &EvidenceCausalEdge,
    positions: &std::collections::BTreeMap<SelectionId, EvidenceCausalNodePosition>,
    direction: EvidenceCausalDirection,
) -> Vec<String> {
    let (cause, effect) = causal_edge_selection_ids(edge);
    let (near, far) = match direction {
        EvidenceCausalDirection::Upstream => (effect, cause),
        EvidenceCausalDirection::Downstream => (cause, effect),
    };
    let allowed = positions.keys().copied().collect::<std::collections::BTreeSet<_>>();
    let mut path = directional_shortest_event_path(graph, root, near, direction, &allowed)
        .expect("divergence edge near endpoint must remain directionally reachable");
    path.push(far);
    path.into_iter().map(|event| event.stable_key()).collect()
}

fn directional_shortest_event_path(
    graph: &VisibleCausalGraph<'_>,
    root: SelectionId,
    target: SelectionId,
    direction: EvidenceCausalDirection,
    allowed: &std::collections::BTreeSet<SelectionId>,
) -> Option<Vec<SelectionId>> {
    if root == target {
        return Some(vec![root]);
    }

    let mut discovered = std::collections::BTreeSet::from([root]);
    let mut queue = std::collections::VecDeque::from([root]);
    let mut predecessor = std::collections::BTreeMap::<SelectionId, SelectionId>::new();
    while let Some(current) = queue.pop_front() {
        let mut neighbors = match direction {
            EvidenceCausalDirection::Upstream => graph.parents(current),
            EvidenceCausalDirection::Downstream => graph.children(current).to_vec(),
        };
        neighbors.retain(|event| allowed.contains(event));
        neighbors.sort();
        neighbors.dedup();
        for neighbor in neighbors {
            if discovered.insert(neighbor) {
                predecessor.insert(neighbor, current);
                if neighbor == target {
                    let mut path = vec![target];
                    let mut cursor = target;
                    while cursor != root {
                        cursor = *predecessor
                            .get(&cursor)
                            .expect("discovered divergence trace node must have a predecessor");
                        path.push(cursor);
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(neighbor);
            }
        }
    }
    None
}

'''
lib = replace_once(lib, marker, helpers + marker, "trace helpers")
lib_path.write_text(lib)

test_path = Path("crates/world-query/tests/causal_first_divergence.rs")
test = test_path.read_text()
test = test.replace(
    'EvidenceCausalDivergenceWitness::Edge { difference, edge } => {',
    'EvidenceCausalDivergenceWitness::Edge { difference, edge, .. } => {',
)
append = r'''

#[test]
fn divergence_edge_witnesses_include_canonical_directional_traces() {
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
    let value = compare(
        &left,
        &right,
        "event-1",
        EvidenceCausalDirection::Downstream,
        2,
    );
    assert_eq!(value.divergence_depth, Some(2));
    assert_eq!(value.witnesses.len(), 2);
    assert_eq!(
        value.witnesses,
        vec![
            EvidenceCausalDivergenceWitness::Edge {
                difference: Difference::LeftOnly,
                edge: world_query::EvidenceCausalEdge {
                    cause: "event-2".into(),
                    effect: "event-3".into(),
                },
                trace: vec!["event-1".into(), "event-2".into(), "event-3".into()],
            },
            EvidenceCausalDivergenceWitness::Edge {
                difference: Difference::RightOnly,
                edge: world_query::EvidenceCausalEdge {
                    cause: "event-2".into(),
                    effect: "event-4".into(),
                },
                trace: vec!["event-1".into(), "event-2".into(), "event-4".into()],
            },
        ]
    );
}

#[test]
fn trace_ends_with_cross_edge_even_when_endpoint_was_already_reached() {
    let left = snapshot(vec![
        event(1, 1, &[]),
        event(2, 2, &[1, 3]),
        event(3, 2, &[1]),
    ]);
    let right = snapshot(vec![
        event(1, 1, &[]),
        event(2, 2, &[1]),
        event(3, 2, &[1]),
    ]);
    let value = compare(
        &left,
        &right,
        "event-1",
        EvidenceCausalDirection::Downstream,
        1,
    );
    assert_eq!(value.divergence_depth, Some(1));
    assert_eq!(
        value.witnesses,
        vec![EvidenceCausalDivergenceWitness::Edge {
            difference: Difference::LeftOnly,
            edge: world_query::EvidenceCausalEdge {
                cause: "event-3".into(),
                effect: "event-2".into(),
            },
            trace: vec!["event-1".into(), "event-3".into(), "event-2".into()],
        }]
    );
}

#[test]
fn upstream_trace_uses_reverse_traversal_but_terminates_with_causal_edge() {
    let left = snapshot(vec![
        event(3, 3, &[2]),
        event(2, 2, &[1]),
        event(1, 1, &[]),
    ]);
    let right = snapshot(vec![event(3, 3, &[2]), event(2, 2, &[])]);
    let value = compare(
        &left,
        &right,
        "event-3",
        EvidenceCausalDirection::Upstream,
        2,
    );
    assert_eq!(value.divergence_depth, Some(2));
    assert_eq!(
        value.witnesses,
        vec![EvidenceCausalDivergenceWitness::Edge {
            difference: Difference::LeftOnly,
            edge: world_query::EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-2".into(),
            },
            trace: vec!["event-3".into(), "event-2".into(), "event-1".into()],
        }]
    );
}

#[test]
fn m203_edge_witness_without_trace_deserializes_with_empty_trace() {
    let json = json!({
        "kind":"edge",
        "difference":"left-only",
        "edge":{"cause":"event-1","effect":"event-2"}
    });
    let witness: EvidenceCausalDivergenceWitness = serde_json::from_value(json).unwrap();
    assert_eq!(
        witness,
        EvidenceCausalDivergenceWitness::Edge {
            difference: Difference::LeftOnly,
            edge: world_query::EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-2".into(),
            },
            trace: vec![],
        }
    );
}
'''
if "divergence_edge_witnesses_include_canonical_directional_traces" in test:
    raise SystemExit("M204 tests already present")
test_path.write_text(test + append)

Path("NEXT_TASK.md").write_text(r'''# Next Coding Task — M204 First-Divergence Traces

Make each M203 first-divergence edge witness self-explanatory by attaching a deterministic directional Event trace from the comparison root to that exact differing causal edge.

## Current baseline

M203 adds `first-divergence` over the existing protocol-v1 `evidence-compare-query` transport. It reports the earliest bounded structural divergence layer and every left/right-only causal edge at that depth, with typed witness ordering and frontier-aware bounded identity.

## M204 — witness traces

Extend the `edge` form of `EvidenceCausalDivergenceWitness` additively with:

- `trace: Vec<String>` using `#[serde(default)]`.

The trace is a canonical directional traversal beginning at the requested root and ending by traversing the witness edge itself.

## Semantics

- Downstream traces walk root → ... → witness cause → witness effect.
- Upstream traces walk root → ... → witness effect → witness cause, because investigation traverses causal edges in reverse while the stored edge remains cause → effect.
- Restrict prefix search to Events already inside that side's bounded causal neighborhood; traces must not escape the M203 query window to explain a witness.
- Choose a shortest directional prefix; break same-length alternatives by typed Event identity rather than timeline/display ordering.
- Always append the witness edge as the final traversal step, even for cross/cycle edges whose far endpoint was reachable earlier by another route.
- A trace is side-specific. All structure strictly shallower than `divergence_depth` is necessarily shared, but same-depth traces may pass another parallel divergence before terminating at their own witness.
- `root-presence` witnesses remain unchanged and carry no trace.

## Compatibility

- `#[serde(default)]` allows protocol-v1 M203 edge witnesses without `trace` to deserialize as an empty trace.
- Older clients can ignore the additive field.
- No new request, response variant, CLI command, transport, protocol version, AgentRuntime authority, or server-side state.

## Tests

Prove downstream and upstream traces, common-prefix behavior, cross/cycle terminal-edge behavior, typed shortest-path selection, and backward deserialization of M203 witnesses without trace.

## Non-goals

No global recursive divergence search, trace mutation authority, arbitrary graph export, opaque cursor, MCP/HTTP/WebSocket, AgentRuntime access, Pack-specific inference, or protocol v2.
''')
