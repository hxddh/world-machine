use serde::{Deserialize, Serialize};
use world_investigation::{
    investigate_first_divergence, ComparisonQueryExecutor, FirstDivergenceInvestigationRequest,
    InvestigationError,
};
use world_query::{EvidenceCausalDirection, EvidenceCausalDivergenceWitness};

pub const FIRST_DIVERGENCE_TOOL_NAME: &str = "world.first-divergence";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadOnlyToolDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    pub read_only: bool,
}

pub fn first_divergence_tool_descriptor() -> ReadOnlyToolDescriptor {
    ReadOnlyToolDescriptor {
        name: FIRST_DIVERGENCE_TOOL_NAME,
        description: "Find the earliest visible causal divergence between two World histories within a bounded depth and return original-root witness traces.",
        read_only: true,
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FirstDivergenceToolInput {
    pub root: String,
    pub direction: EvidenceCausalDirection,
    pub window_depth: usize,
    pub max_depth: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FirstDivergenceToolOutput {
    pub root: String,
    pub direction: EvidenceCausalDirection,
    pub max_depth: usize,
    pub identical_within_depth: bool,
    pub divergence_depth: Option<usize>,
    pub witnesses: Vec<EvidenceCausalDivergenceWitness>,
    pub truncated: bool,
}

pub struct FirstDivergenceTool<E> {
    executor: E,
}

impl<E> FirstDivergenceTool<E> {
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    pub fn descriptor(&self) -> ReadOnlyToolDescriptor {
        first_divergence_tool_descriptor()
    }

    pub fn executor(&self) -> &E {
        &self.executor
    }

    pub fn executor_mut(&mut self) -> &mut E {
        &mut self.executor
    }

    pub fn into_inner(self) -> E {
        self.executor
    }
}

impl<E> FirstDivergenceTool<E>
where
    E: ComparisonQueryExecutor,
{
    pub fn invoke(
        &mut self,
        input: &FirstDivergenceToolInput,
    ) -> Result<FirstDivergenceToolOutput, InvestigationError<E::Error>> {
        let result = investigate_first_divergence(
            &mut self.executor,
            &FirstDivergenceInvestigationRequest {
                root: input.root.clone(),
                direction: input.direction,
                window_depth: input.window_depth,
                max_depth: input.max_depth,
            },
        )?;
        Ok(FirstDivergenceToolOutput {
            root: result.root,
            direction: result.direction,
            max_depth: result.max_depth,
            identical_within_depth: result.identical_within_depth,
            divergence_depth: result.divergence_depth,
            witnesses: result.witnesses,
            truncated: result.truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::convert::Infallible;
    use world_query::{
        Difference, EvidenceCausalComparisonRequest, EvidenceCausalComparisonResponse,
        EvidenceCausalEdge, EvidenceCausalFirstDivergenceContinuation,
        EvidenceCausalFirstDivergenceResult, EvidenceComparisonQueryRequest,
        EvidenceComparisonQueryResponse,
    };

    struct ScriptedExecutor {
        script: VecDeque<(
            EvidenceComparisonQueryRequest,
            EvidenceComparisonQueryResponse,
        )>,
    }

    impl ScriptedExecutor {
        fn new(
            script: Vec<(
                EvidenceComparisonQueryRequest,
                EvidenceComparisonQueryResponse,
            )>,
        ) -> Self {
            Self {
                script: script.into(),
            }
        }
    }

    impl ComparisonQueryExecutor for ScriptedExecutor {
        type Error = Infallible;

        fn execute(
            &mut self,
            request: &EvidenceComparisonQueryRequest,
        ) -> Result<EvidenceComparisonQueryResponse, Self::Error> {
            let (expected, response) = self
                .script
                .pop_front()
                .expect("every tool query should have a scripted response");
            assert_eq!(&expected, request);
            Ok(response)
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

    fn response(
        root: &str,
        direction: EvidenceCausalDirection,
        max_depth: usize,
        divergence_depth: Option<usize>,
        witnesses: Vec<EvidenceCausalDivergenceWitness>,
        continuations: Vec<EvidenceCausalFirstDivergenceContinuation>,
    ) -> EvidenceComparisonQueryResponse {
        EvidenceComparisonQueryResponse::Causal(EvidenceCausalComparisonResponse::FirstDivergence {
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
        })
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
            request: request(event, direction, replay_depth),
        }
    }

    fn witness(trace: &[&str]) -> EvidenceCausalDivergenceWitness {
        EvidenceCausalDivergenceWitness::Edge {
            difference: Difference::LeftOnly,
            edge: EvidenceCausalEdge {
                cause: "event-1".into(),
                effect: "event-2".into(),
            },
            trace: trace.iter().map(|event| (*event).into()).collect(),
        }
    }

    #[test]
    fn descriptor_is_stably_named_and_explicitly_read_only() {
        let descriptor = first_divergence_tool_descriptor();
        assert_eq!(descriptor.name, "world.first-divergence");
        assert!(descriptor.read_only);
        assert!(!descriptor.description.is_empty());
    }

    #[test]
    fn typed_input_has_stable_provider_neutral_json_shape() {
        let input = FirstDivergenceToolInput {
            root: "event-4".into(),
            direction: EvidenceCausalDirection::Upstream,
            window_depth: 1,
            max_depth: 3,
        };
        assert_eq!(
            serde_json::to_value(&input).unwrap(),
            serde_json::json!({
                "root": "event-4",
                "direction": "upstream",
                "window_depth": 1,
                "max_depth": 3,
            })
        );
        assert_eq!(
            serde_json::from_value::<FirstDivergenceToolInput>(
                serde_json::to_value(&input).unwrap()
            )
            .unwrap(),
            input
        );
    }

    #[test]
    fn tool_reuses_investigation_scheduler_and_returns_original_root_trace() {
        let direction = EvidenceCausalDirection::Upstream;
        let mut tool = FirstDivergenceTool::new(ScriptedExecutor::new(vec![
            (
                request("event-4", direction, 1),
                response(
                    "event-4",
                    direction,
                    1,
                    None,
                    vec![],
                    vec![continuation(
                        "event-3",
                        direction,
                        1,
                        &["event-4", "event-3"],
                        1,
                    )],
                ),
            ),
            (
                request("event-3", direction, 1),
                response(
                    "event-3",
                    direction,
                    1,
                    None,
                    vec![],
                    vec![continuation(
                        "event-2",
                        direction,
                        1,
                        &["event-3", "event-2"],
                        1,
                    )],
                ),
            ),
            (
                request("event-2", direction, 1),
                response(
                    "event-2",
                    direction,
                    1,
                    Some(1),
                    vec![witness(&["event-2", "event-1"])],
                    vec![],
                ),
            ),
        ]));

        let output = tool
            .invoke(&FirstDivergenceToolInput {
                root: "event-4".into(),
                direction,
                window_depth: 1,
                max_depth: 3,
            })
            .unwrap();

        assert_eq!(output.divergence_depth, Some(3));
        assert!(!output.identical_within_depth);
        assert!(!output.truncated);
        assert_eq!(
            output.witnesses,
            vec![witness(&["event-4", "event-3", "event-2", "event-1"])]
        );
        assert!(tool.executor().script.is_empty());
    }

    #[test]
    fn output_is_serializable_without_exposing_executor_or_world_internals() {
        let output = FirstDivergenceToolOutput {
            root: "event-1".into(),
            direction: EvidenceCausalDirection::Downstream,
            max_depth: 2,
            identical_within_depth: true,
            divergence_depth: None,
            witnesses: vec![],
            truncated: false,
        };
        let value = serde_json::to_value(output).unwrap();
        assert_eq!(value["root"], "event-1");
        assert_eq!(value["direction"], "downstream");
        assert_eq!(value["identical_within_depth"], true);
        assert_eq!(value["divergence_depth"], serde_json::Value::Null);
    }
}
