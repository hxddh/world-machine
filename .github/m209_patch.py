from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


workspace = Path("Cargo.toml")
text = workspace.read_text()
if '"crates/world-investigation"' not in text:
    text = replace_once(
        text,
        '  "crates/world-query",\n',
        '  "crates/world-query",\n  "crates/world-investigation",\n',
        "workspace member",
    )
    workspace.write_text(text)

boundaries = Path("scripts/check-boundaries.sh")
text = boundaries.read_text()
if 'INVESTIGATION="$ROOT/crates/world-investigation"' not in text:
    text = replace_once(
        text,
        'PROJECTION="$ROOT/crates/world-projection"\n',
        'PROJECTION="$ROOT/crates/world-projection"\nINVESTIGATION="$ROOT/crates/world-investigation"\n',
        "investigation boundary root",
    )
    text = replace_once(
        text,
        'projection_forbidden=("TinySociety" "Tiny Society" "FutureArchaeologist" "Future Archaeologist" "Bakery" "Society" "gpui" "pi_agent")\n',
        'projection_forbidden=("TinySociety" "Tiny Society" "FutureArchaeologist" "Future Archaeologist" "Bakery" "Society" "gpui" "pi_agent")\ninvestigation_forbidden=("world_projection" "world-projection" "ProjectionSnapshot" "world_core" "world-core" "world_agent" "world-agent" "gpui" "pi_agent" "openai" "anthropic")\n',
        "investigation forbidden list",
    )
    marker = '''if [[ -d "$HOST" ]]; then
'''
    block = '''if [[ -d "$INVESTIGATION" ]]; then
  for token in "${investigation_forbidden[@]}"; do
    if grep -Rni --exclude-dir=target -i -- "$token" "$INVESTIGATION" >/tmp/world-machine-investigation-boundary-check 2>/dev/null; then
      echo "Boundary violation: '$token' found in query-only world-investigation adapter:"
      cat /tmp/world-machine-investigation-boundary-check
      failed=1
    fi
  done
fi

if [[ -d "$HOST" ]]; then
'''
    text = replace_once(text, marker, block, "investigation boundary check")
    boundaries.write_text(text)

crate = Path("crates/world-investigation")
(crate / "src").mkdir(parents=True, exist_ok=True)
(crate / "Cargo.toml").write_text('''[package]
name = "world-investigation"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
publish = false

[dependencies]
world-query = { path = "../world-query" }
''')

(crate / "src/lib.rs").write_text(r'''use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

use world_query::{
    EvidenceCausalComparisonRequest, EvidenceCausalComparisonResponse, EvidenceCausalDirection,
    EvidenceCausalDivergenceWitness, EvidenceCausalFirstDivergenceContinuation,
    EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse,
};

pub trait ComparisonQueryExecutor {
    type Error;

    fn execute(
        &mut self,
        request: &EvidenceComparisonQueryRequest,
    ) -> Result<EvidenceComparisonQueryResponse, Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstDivergenceInvestigationRequest {
    pub root: String,
    pub direction: EvidenceCausalDirection,
    pub window_depth: usize,
    pub max_depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstDivergenceInvestigationResult {
    pub root: String,
    pub direction: EvidenceCausalDirection,
    pub max_depth: usize,
    pub identical_within_depth: bool,
    pub divergence_depth: Option<usize>,
    pub witnesses: Vec<EvidenceCausalDivergenceWitness>,
    pub truncated: bool,
}

#[derive(Debug)]
pub enum InvestigationError<E> {
    InvalidWindowDepth,
    Executor(E),
    UnexpectedResponse,
    InvalidContinuation,
    InvalidTrace,
    UnexpectedNestedRootPresence,
}

impl<E: fmt::Display> fmt::Display for InvestigationError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWindowDepth => write!(f, "investigation window depth must be greater than zero"),
            Self::Executor(error) => write!(f, "comparison query executor failed: {error}"),
            Self::UnexpectedResponse => write!(f, "comparison query executor returned an unexpected response"),
            Self::InvalidContinuation => write!(f, "first-divergence continuation does not match its replay request"),
            Self::InvalidTrace => write!(f, "first-divergence continuation or witness trace is not composable"),
            Self::UnexpectedNestedRootPresence => write!(
                f,
                "first-divergence replay returned root-presence after a divergence-free prefix"
            ),
        }
    }
}

impl<E> Error for InvestigationError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Executor(error) => Some(error),
            Self::InvalidWindowDepth
            | Self::UnexpectedResponse
            | Self::InvalidContinuation
            | Self::InvalidTrace
            | Self::UnexpectedNestedRootPresence => None,
        }
    }
}

#[derive(Clone, Debug)]
struct PendingSearch {
    offset: usize,
    root: String,
    prefix: Vec<String>,
    request: EvidenceComparisonQueryRequest,
}

pub fn investigate_first_divergence<E>(
    executor: &mut E,
    request: &FirstDivergenceInvestigationRequest,
) -> Result<FirstDivergenceInvestigationResult, InvestigationError<E::Error>>
where
    E: ComparisonQueryExecutor,
{
    if request.window_depth == 0 {
        return Err(InvestigationError::InvalidWindowDepth);
    }

    let initial_depth = request.window_depth.min(request.max_depth);
    let initial_request = first_divergence_request(&request.root, request.direction, initial_depth);
    let mut queue = VecDeque::from([PendingSearch {
        offset: 0,
        root: request.root.clone(),
        prefix: Vec::new(),
        request: initial_request,
    }]);
    let mut seen = BTreeSet::<(usize, String)>::new();
    let mut best_depth = None;
    let mut witnesses = Vec::new();
    let mut truncated = false;

    while let Some(pending) = queue.pop_front() {
        if !seen.insert((pending.offset, pending.root.clone())) {
            continue;
        }
        if best_depth.is_some_and(|best| pending.offset >= best) {
            continue;
        }

        let value = execute_first_divergence(executor, &pending.request)?;
        if let Some(relative_depth) = value.divergence_depth {
            let absolute_depth = pending.offset + relative_depth;
            if absolute_depth > request.max_depth {
                continue;
            }
            let composed = compose_witnesses(
                &pending.root,
                pending.offset,
                &pending.prefix,
                &value.witnesses,
            )?;
            match best_depth {
                None => {
                    best_depth = Some(absolute_depth);
                    witnesses = composed;
                }
                Some(best) if absolute_depth < best => {
                    best_depth = Some(absolute_depth);
                    witnesses = composed;
                }
                Some(best) if absolute_depth == best => {
                    extend_unique(&mut witnesses, composed);
                }
                Some(_) => {}
            }
            continue;
        }

        for continuation in value.continuations {
            validate_continuation(&pending.root, request.direction, &continuation)?;
            let next_offset = pending.offset + continuation.depth_offset;
            if next_offset >= request.max_depth {
                truncated = true;
                continue;
            }
            let remaining = request.max_depth - next_offset;
            let next_depth = request.window_depth.min(remaining);
            let next_prefix = compose_trace(&pending.prefix, &continuation.trace_prefix)?;
            queue.push_back(PendingSearch {
                offset: next_offset,
                root: continuation.event.clone(),
                prefix: next_prefix,
                request: first_divergence_request(
                    &continuation.event,
                    request.direction,
                    next_depth,
                ),
            });
        }
    }

    Ok(FirstDivergenceInvestigationResult {
        root: request.root.clone(),
        direction: request.direction,
        max_depth: request.max_depth,
        identical_within_depth: best_depth.is_none(),
        divergence_depth: best_depth,
        witnesses,
        truncated: best_depth.is_none() && truncated,
    })
}

fn execute_first_divergence<E>(
    executor: &mut E,
    request: &EvidenceComparisonQueryRequest,
) -> Result<world_query::EvidenceCausalFirstDivergenceResult, InvestigationError<E::Error>>
where
    E: ComparisonQueryExecutor,
{
    let response = executor
        .execute(request)
        .map_err(InvestigationError::Executor)?;
    match response {
        EvidenceComparisonQueryResponse::Causal(
            EvidenceCausalComparisonResponse::FirstDivergence { value },
        ) => Ok(value),
        _ => Err(InvestigationError::UnexpectedResponse),
    }
}

fn first_divergence_request(
    root: &str,
    direction: EvidenceCausalDirection,
    max_depth: usize,
) -> EvidenceComparisonQueryRequest {
    EvidenceComparisonQueryRequest::Causal(EvidenceCausalComparisonRequest::FirstDivergence {
        root: root.to_owned(),
        direction,
        max_depth,
    })
}

fn validate_continuation<E>(
    current_root: &str,
    direction: EvidenceCausalDirection,
    continuation: &EvidenceCausalFirstDivergenceContinuation,
) -> Result<(), InvestigationError<E>> {
    if continuation.direction != direction
        || continuation.trace_prefix.first().map(String::as_str) != Some(current_root)
        || continuation.trace_prefix.last().map(String::as_str) != Some(continuation.event.as_str())
    {
        return Err(InvestigationError::InvalidTrace);
    }
    match &continuation.request {
        EvidenceComparisonQueryRequest::Causal(
            EvidenceCausalComparisonRequest::FirstDivergence {
                root,
                direction: replay_direction,
                ..
            },
        ) if root == &continuation.event && *replay_direction == direction => Ok(()),
        _ => Err(InvestigationError::InvalidContinuation),
    }
}

fn compose_witnesses<E>(
    current_root: &str,
    offset: usize,
    prefix: &[String],
    witnesses: &[EvidenceCausalDivergenceWitness],
) -> Result<Vec<EvidenceCausalDivergenceWitness>, InvestigationError<E>> {
    witnesses
        .iter()
        .map(|witness| match witness {
            EvidenceCausalDivergenceWitness::RootPresence { .. } if offset > 0 => {
                Err(InvestigationError::UnexpectedNestedRootPresence)
            }
            EvidenceCausalDivergenceWitness::RootPresence { .. } => Ok(witness.clone()),
            EvidenceCausalDivergenceWitness::Edge {
                difference,
                edge,
                trace,
            } => {
                if trace.first().map(String::as_str) != Some(current_root) {
                    return Err(InvestigationError::InvalidTrace);
                }
                Ok(EvidenceCausalDivergenceWitness::Edge {
                    difference: *difference,
                    edge: edge.clone(),
                    trace: compose_trace(prefix, trace)?,
                })
            }
        })
        .collect()
}

fn compose_trace<E>(
    prefix: &[String],
    suffix: &[String],
) -> Result<Vec<String>, InvestigationError<E>> {
    if prefix.is_empty() {
        if suffix.is_empty() {
            return Err(InvestigationError::InvalidTrace);
        }
        return Ok(suffix.to_vec());
    }
    if suffix.is_empty() || prefix.last() != suffix.first() {
        return Err(InvestigationError::InvalidTrace);
    }
    let mut composed = prefix.to_vec();
    composed.extend(suffix.iter().skip(1).cloned());
    Ok(composed)
}

fn extend_unique(
    destination: &mut Vec<EvidenceCausalDivergenceWitness>,
    source: Vec<EvidenceCausalDivergenceWitness>,
) {
    for witness in source {
        if !destination.contains(&witness) {
            destination.push(witness);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::convert::Infallible;
    use world_query::{
        Difference, EvidenceCausalEdge, EvidenceCausalFirstDivergenceResult,
        EvidenceComparisonResult,
    };

    struct ScriptedExecutor {
        script: VecDeque<(EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse)>,
        calls: Vec<EvidenceComparisonQueryRequest>,
    }

    impl ScriptedExecutor {
        fn new(script: Vec<(EvidenceComparisonQueryRequest, EvidenceComparisonQueryResponse)>) -> Self {
            Self {
                script: script.into(),
                calls: Vec::new(),
            }
        }

        fn assert_exhausted(&self) {
            assert!(self.script.is_empty(), "script still contains unexecuted responses");
        }
    }

    impl ComparisonQueryExecutor for ScriptedExecutor {
        type Error = Infallible;

        fn execute(
            &mut self,
            request: &EvidenceComparisonQueryRequest,
        ) -> Result<EvidenceComparisonQueryResponse, Self::Error> {
            self.calls.push(request.clone());
            let (expected, response) = self
                .script
                .pop_front()
                .expect("script should contain a response for every executed request");
            assert_eq!(&expected, request);
            Ok(response)
        }
    }

    fn response(
        root: &str,
        direction: EvidenceCausalDirection,
        max_depth: usize,
        divergence_depth: Option<usize>,
        witnesses: Vec<EvidenceCausalDivergenceWitness>,
        continuations: Vec<EvidenceCausalFirstDivergenceContinuation>,
    ) -> EvidenceComparisonQueryResponse {
        EvidenceComparisonQueryResponse::Causal(
            EvidenceCausalComparisonResponse::FirstDivergence {
                value: EvidenceCausalFirstDivergenceResult {
                    root: root.into(),
                    direction,
                    max_depth,
                    identical_within_depth: divergence_depth.is_none(),
                    divergence_depth,
                    witnesses,
                    left_frontier: vec![],
                    right_frontier: vec![],
                    continuations,
                },
            },
        )
    }

    fn continuation(
        event: &str,
        direction: EvidenceCausalDirection,
        depth_offset: usize,
        trace_prefix: &[&str],
        replay_depth: usize,
    ) -> EvidenceCausalFirstDivergenceContinuation {
        EvidenceCausalFirstDivergenceContinuation {
            event: event.into(),
            direction,
            left_frontier: true,
            right_frontier: true,
            depth_offset,
            trace_prefix: trace_prefix.iter().map(|event| (*event).into()).collect(),
            request: first_divergence_request(event, direction, replay_depth),
        }
    }

    fn edge_witness(
        difference: Difference,
        cause: &str,
        effect: &str,
        trace: &[&str],
    ) -> EvidenceCausalDivergenceWitness {
        EvidenceCausalDivergenceWitness::Edge {
            difference,
            edge: EvidenceCausalEdge {
                cause: cause.into(),
                effect: effect.into(),
            },
            trace: trace.iter().map(|event| (*event).into()).collect(),
        }
    }

    #[test]
    fn progressive_investigation_composes_three_windows_into_absolute_result() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut executor = ScriptedExecutor::new(vec![
            (
                first_divergence_request("event-4", direction, 1),
                response(
                    "event-4",
                    direction,
                    1,
                    None,
                    vec![],
                    vec![continuation("event-3", direction, 1, &["event-4", "event-3"], 1)],
                ),
            ),
            (
                first_divergence_request("event-3", direction, 1),
                response(
                    "event-3",
                    direction,
                    1,
                    None,
                    vec![],
                    vec![continuation("event-2", direction, 1, &["event-3", "event-2"], 1)],
                ),
            ),
            (
                first_divergence_request("event-2", direction, 1),
                response(
                    "event-2",
                    direction,
                    1,
                    Some(1),
                    vec![edge_witness(
                        Difference::LeftOnly,
                        "event-1",
                        "event-2",
                        &["event-2", "event-1"],
                    )],
                    vec![],
                ),
            ),
        ]);

        let result = investigate_first_divergence(
            &mut executor,
            &FirstDivergenceInvestigationRequest {
                root: "event-4".into(),
                direction,
                window_depth: 1,
                max_depth: 3,
            },
        )
        .unwrap();

        assert_eq!(result.divergence_depth, Some(3));
        assert!(!result.identical_within_depth);
        assert!(!result.truncated);
        assert_eq!(
            result.witnesses,
            vec![edge_witness(
                Difference::LeftOnly,
                "event-1",
                "event-2",
                &["event-4", "event-3", "event-2", "event-1"],
            )]
        );
        executor.assert_exhausted();
    }

    #[test]
    fn final_replay_window_is_retargeted_to_remaining_depth() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut executor = ScriptedExecutor::new(vec![
            (
                first_divergence_request("event-4", direction, 2),
                response(
                    "event-4",
                    direction,
                    2,
                    None,
                    vec![],
                    vec![continuation(
                        "event-2",
                        direction,
                        2,
                        &["event-4", "event-3", "event-2"],
                        2,
                    )],
                ),
            ),
            (
                first_divergence_request("event-2", direction, 1),
                response(
                    "event-2",
                    direction,
                    1,
                    Some(1),
                    vec![edge_witness(
                        Difference::LeftOnly,
                        "event-1",
                        "event-2",
                        &["event-2", "event-1"],
                    )],
                    vec![],
                ),
            ),
        ]);

        let result = investigate_first_divergence(
            &mut executor,
            &FirstDivergenceInvestigationRequest {
                root: "event-4".into(),
                direction,
                window_depth: 2,
                max_depth: 3,
            },
        )
        .unwrap();

        assert_eq!(result.divergence_depth, Some(3));
        assert_eq!(executor.calls[1], first_divergence_request("event-2", direction, 1));
        executor.assert_exhausted();
    }

    #[test]
    fn converging_diamond_executes_shared_state_once_and_keeps_canonical_trace() {
        let direction = EvidenceCausalDirection::Downstream;
        let mut executor = ScriptedExecutor::new(vec![
            (
                first_divergence_request("event-1", direction, 1),
                response(
                    "event-1",
                    direction,
                    1,
                    None,
                    vec![],
                    vec![
                        continuation("event-2", direction, 1, &["event-1", "event-2"], 1),
                        continuation("event-3", direction, 1, &["event-1", "event-3"], 1),
                    ],
                ),
            ),
            (
                first_divergence_request("event-2", direction, 1),
                response(
                    "event-2",
                    direction,
                    1,
                    None,
                    vec![],
                    vec![continuation("event-4", direction, 1, &["event-2", "event-4"], 1)],
                ),
            ),
            (
                first_divergence_request("event-3", direction, 1),
                response(
                    "event-3",
                    direction,
                    1,
                    None,
                    vec![],
                    vec![continuation("event-4", direction, 1, &["event-3", "event-4"], 1)],
                ),
            ),
            (
                first_divergence_request("event-4", direction, 1),
                response(
                    "event-4",
                    direction,
                    1,
                    Some(1),
                    vec![edge_witness(
                        Difference::LeftOnly,
                        "event-4",
                        "event-5",
                        &["event-4", "event-5"],
                    )],
                    vec![],
                ),
            ),
        ]);

        let result = investigate_first_divergence(
            &mut executor,
            &FirstDivergenceInvestigationRequest {
                root: "event-1".into(),
                direction,
                window_depth: 1,
                max_depth: 3,
            },
        )
        .unwrap();

        assert_eq!(result.divergence_depth, Some(3));
        assert_eq!(executor.calls.len(), 4);
        assert_eq!(
            result.witnesses,
            vec![edge_witness(
                Difference::LeftOnly,
                "event-4",
                "event-5",
                &["event-1", "event-2", "event-4", "event-5"],
            )]
        );
        executor.assert_exhausted();
    }

    #[test]
    fn bounded_identical_result_reports_unexplored_frontier_as_truncated() {
        let direction = EvidenceCausalDirection::Downstream;
        let mut executor = ScriptedExecutor::new(vec![(
            first_divergence_request("event-1", direction, 2),
            response(
                "event-1",
                direction,
                2,
                None,
                vec![],
                vec![continuation(
                    "event-3",
                    direction,
                    2,
                    &["event-1", "event-2", "event-3"],
                    2,
                )],
            ),
        )]);

        let result = investigate_first_divergence(
            &mut executor,
            &FirstDivergenceInvestigationRequest {
                root: "event-1".into(),
                direction,
                window_depth: 2,
                max_depth: 2,
            },
        )
        .unwrap();

        assert!(result.identical_within_depth);
        assert_eq!(result.divergence_depth, None);
        assert!(result.truncated);
        executor.assert_exhausted();
    }

    #[test]
    fn exhausted_identical_result_is_not_truncated() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut executor = ScriptedExecutor::new(vec![(
            first_divergence_request("event-2", direction, 3),
            response("event-2", direction, 3, None, vec![], vec![]),
        )]);

        let result = investigate_first_divergence(
            &mut executor,
            &FirstDivergenceInvestigationRequest {
                root: "event-2".into(),
                direction,
                window_depth: 3,
                max_depth: 3,
            },
        )
        .unwrap();

        assert!(result.identical_within_depth);
        assert!(!result.truncated);
        executor.assert_exhausted();
    }

    #[test]
    fn zero_window_is_rejected_before_executor_access() {
        let mut executor = ScriptedExecutor::new(vec![]);
        let error = investigate_first_divergence(
            &mut executor,
            &FirstDivergenceInvestigationRequest {
                root: "event-1".into(),
                direction: EvidenceCausalDirection::Upstream,
                window_depth: 0,
                max_depth: 3,
            },
        )
        .unwrap_err();

        assert!(matches!(error, InvestigationError::InvalidWindowDepth));
        assert!(executor.calls.is_empty());
    }

    struct FailingExecutor;

    impl ComparisonQueryExecutor for FailingExecutor {
        type Error = &'static str;

        fn execute(
            &mut self,
            _request: &EvidenceComparisonQueryRequest,
        ) -> Result<EvidenceComparisonQueryResponse, Self::Error> {
            Err("offline")
        }
    }

    #[test]
    fn executor_failures_are_preserved_by_the_orchestration_boundary() {
        let error = investigate_first_divergence(
            &mut FailingExecutor,
            &FirstDivergenceInvestigationRequest {
                root: "event-1".into(),
                direction: EvidenceCausalDirection::Downstream,
                window_depth: 1,
                max_depth: 2,
            },
        )
        .unwrap_err();

        assert!(matches!(error, InvestigationError::Executor("offline")));
    }

    #[test]
    fn unexpected_non_divergence_response_is_rejected() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut executor = ScriptedExecutor::new(vec![(
            first_divergence_request("event-1", direction, 1),
            EvidenceComparisonQueryResponse::Legacy(EvidenceComparisonResult {
                root: "entity-1".into(),
                max_depth: 0,
                identical: true,
                nodes: vec![],
                left_only_edges: vec![],
                right_only_edges: vec![],
            }),
        )]);

        let error = investigate_first_divergence(
            &mut executor,
            &FirstDivergenceInvestigationRequest {
                root: "event-1".into(),
                direction,
                window_depth: 1,
                max_depth: 1,
            },
        )
        .unwrap_err();

        assert!(matches!(error, InvestigationError::UnexpectedResponse));
    }
}
''')

Path("NEXT_TASK.md").write_text('''# Next Coding Task — M209 Read-Only Investigation Orchestrator

Turn the now-proven M203–M208 machine-query continuation semantics into a reusable orchestration boundary without exposing `ProjectionSnapshot` or embedding transport/provider concerns in `world-query` or `world-agent`.

## Current baseline

M203–M205 provide bounded first-divergence search, witness traces, and replayable continuations. M206–M208 prove that segmented replay preserves absolute divergence depth, witness sets, original-root traces, zero-depth progress, typed tie-breaking, and canonical convergence semantics. External callers can therefore orchestrate investigation safely, but each caller would otherwise have to reimplement the scheduler.

## M209 — `world-investigation`

Add a new `world-investigation` crate whose production dependency is only `world-query`.

The crate exposes:

- a provider-neutral `ComparisonQueryExecutor` trait that accepts ordinary `EvidenceComparisonQueryRequest` DTOs;
- `FirstDivergenceInvestigationRequest { root, direction, window_depth, max_depth }`;
- `investigate_first_divergence`, which incrementally replays first-divergence continuations until it finds the globally earliest divergence or exhausts the requested depth budget;
- a result carrying absolute divergence depth, original-root witness traces, bounded identity, and whether unexplored causal frontier remains beyond `max_depth`.

## Boundary rules

- `world-investigation` must not depend on or name `world-projection`, `ProjectionSnapshot`, `world-core`, `world-agent`, GPUI, model providers, or transport stacks.
- The executor is the only authority boundary. CLI, local in-memory adapters, MCP, or future Agent tools may implement it later without changing the orchestration semantics.
- Validate that returned continuations are self-consistent first-divergence requests before replaying them.
- Require M207 trace prefixes for cross-window original-root explanations; do not silently invent missing paths.
- Keep canonical convergence keyed by `(absolute offset, continuation Event)` so equal search states execute once and retain deterministic typed-order explanation prefixes.
- Retarget the final replay window to the remaining depth budget instead of overscanning past `max_depth`.

## Compatibility

No change to `world-machine-evidence-query` protocol v1, existing request/response DTOs, `world-cli`, AgentRuntime, Pack APIs, or persistence formats. This is orchestration over the existing public machine-query boundary.

## Tests

Prove three-window trace/depth composition, partial final windows, converging-diamond state dedup and canonical traces, bounded truncation vs exhausted identity, invalid zero-size windows, executor error preservation, and unexpected-response rejection.

## Non-goals

No CLI integration yet, no AgentRuntime tool exposure yet, no MCP/HTTP/WebSocket, no server cursor/session, no mutation authority, no arbitrary graph export, and no protocol v2.
''')
