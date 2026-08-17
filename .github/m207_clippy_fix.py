from pathlib import Path

path = Path("crates/world-query/src/lib.rs")
text = path.read_text()

old_call = '''        causal_first_divergence_continuations(
            &left_graph,
            &right_graph,
            root,
            &left_positions,
            &right_positions,
            &left_frontier,
            &right_frontier,
            direction,
            max_depth,
        )
'''
new_call = '''        causal_first_divergence_continuations(
            (&left_graph, &left_positions),
            (&right_graph, &right_positions),
            root,
            &left_frontier,
            &right_frontier,
            direction,
            max_depth,
        )
'''
if text.count(old_call) != 1:
    raise SystemExit(f"continuation call: expected one match, found {text.count(old_call)}")
text = text.replace(old_call, new_call, 1)

old_signature = '''fn causal_first_divergence_continuations(
    left_graph: &VisibleCausalGraph<'_>,
    right_graph: &VisibleCausalGraph<'_>,
    root: SelectionId,
    left_positions: &std::collections::BTreeMap<SelectionId, EvidenceCausalNodePosition>,
    right_positions: &std::collections::BTreeMap<SelectionId, EvidenceCausalNodePosition>,
    left_frontier: &[String],
    right_frontier: &[String],
    direction: EvidenceCausalDirection,
    max_depth: usize,
) -> Vec<EvidenceCausalFirstDivergenceContinuation> {
'''
new_signature = '''fn causal_first_divergence_continuations(
    left: (
        &VisibleCausalGraph<'_>,
        &std::collections::BTreeMap<SelectionId, EvidenceCausalNodePosition>,
    ),
    right: (
        &VisibleCausalGraph<'_>,
        &std::collections::BTreeMap<SelectionId, EvidenceCausalNodePosition>,
    ),
    root: SelectionId,
    left_frontier: &[String],
    right_frontier: &[String],
    direction: EvidenceCausalDirection,
    max_depth: usize,
) -> Vec<EvidenceCausalFirstDivergenceContinuation> {
'''
if text.count(old_signature) != 1:
    raise SystemExit(f"continuation signature: expected one match, found {text.count(old_signature)}")
text = text.replace(old_signature, new_signature, 1)

old_choice = '''            let (graph, positions) = if left_frontier {
                (left_graph, left_positions)
            } else {
                (right_graph, right_positions)
            };
'''
new_choice = '''            let (graph, positions) = if left_frontier { left } else { right };
'''
if text.count(old_choice) != 1:
    raise SystemExit(f"side choice: expected one match, found {text.count(old_choice)}")
text = text.replace(old_choice, new_choice, 1)

path.write_text(text)
