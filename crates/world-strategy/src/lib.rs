use std::error::Error;
use std::fmt;
use world_compare::{compare_snapshots, SnapshotComparison};
use world_host::{HostError, WorldRegistry, WorldSession};
use world_persistence::WorldArchive;
use world_projection::{ProjectionIntent, ProjectionSnapshot};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StrategyPlan {
    pub intents: Vec<ProjectionIntent>,
    pub background_periods: u64,
}

impl StrategyPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intent(mut self, intent: ProjectionIntent) -> Self {
        self.intents.push(intent);
        self
    }

    pub fn command(self, command: impl Into<String>) -> Self {
        self.intent(ProjectionIntent::InvokeCommand(command.into()))
    }

    pub fn background_periods(mut self, periods: u64) -> Self {
        self.background_periods = periods;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrategyOutcome {
    pub snapshot: ProjectionSnapshot,
    pub archive: Option<WorldArchive>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StrategyRun {
    Success(StrategyOutcome),
    Failure(StrategyError),
}

impl StrategyRun {
    pub fn outcome(&self) -> Option<&StrategyOutcome> {
        match self {
            Self::Success(outcome) => Some(outcome),
            Self::Failure(_) => None,
        }
    }

    pub fn error(&self) -> Option<&StrategyError> {
        match self {
            Self::Success(_) => None,
            Self::Failure(error) => Some(error),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrategyEvaluation {
    pub left: StrategyRun,
    pub right: StrategyRun,
    pub comparison: Option<SnapshotComparison>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrategySide {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StrategyStage {
    Open,
    Intent(usize),
    Background,
    Archive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyError {
    pub side: StrategySide,
    pub stage: StrategyStage,
    pub source: HostError,
}

impl fmt::Display for StrategyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} strategy failed during {:?}: {}",
            self.side, self.stage, self.source
        )
    }
}

impl Error for StrategyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

pub fn evaluate_strategies(
    registry: &WorldRegistry,
    source: &WorldArchive,
    left_plan: &StrategyPlan,
    right_plan: &StrategyPlan,
) -> StrategyEvaluation {
    let left = run_strategy(registry, source, left_plan, StrategySide::Left);
    let right = run_strategy(registry, source, right_plan, StrategySide::Right);
    let comparison = match (&left, &right) {
        (StrategyRun::Success(left), StrategyRun::Success(right)) => {
            Some(compare_snapshots(&left.snapshot, &right.snapshot))
        }
        _ => None,
    };

    StrategyEvaluation {
        left,
        right,
        comparison,
    }
}

fn run_strategy(
    registry: &WorldRegistry,
    source: &WorldArchive,
    plan: &StrategyPlan,
    side: StrategySide,
) -> StrategyRun {
    let mut session = match registry.open_archive(source) {
        Ok(session) => session,
        Err(source) => {
            return StrategyRun::Failure(StrategyError {
                side,
                stage: StrategyStage::Open,
                source,
            });
        }
    };

    match run_plan(&mut *session, plan, side) {
        Ok(outcome) => StrategyRun::Success(outcome),
        Err(error) => StrategyRun::Failure(error),
    }
}

fn run_plan(
    session: &mut dyn WorldSession,
    plan: &StrategyPlan,
    side: StrategySide,
) -> Result<StrategyOutcome, StrategyError> {
    let mut snapshot = session.snapshot();
    for (index, intent) in plan.intents.iter().cloned().enumerate() {
        snapshot = session.handle(intent).map_err(|source| StrategyError {
            side,
            stage: StrategyStage::Intent(index),
            source,
        })?;
    }

    if plan.background_periods > 0 {
        snapshot = session
            .advance_background(plan.background_periods)
            .map_err(|source| StrategyError {
                side,
                stage: StrategyStage::Background,
                source,
            })?;
    }

    let archive = session.archive().map_err(|source| StrategyError {
        side,
        stage: StrategyStage::Archive,
        source,
    })?;

    Ok(StrategyOutcome { snapshot, archive })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tiny_society::{
        tiny_society_registration, TinySociety, BAKERY, LEAN_REOPEN_BAKERY_COMMAND, MARA,
        REOPEN_BAKERY_COMMAND,
    };
    use world_compare::DifferenceKind;
    use world_host::{WorldDescriptor, WorldRegistration};
    use world_persistence::{WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
    use world_projection::{ProjectionCapabilities, ProjectionCommand, SelectionId};

    const MOCK_PACK: &str = "world-machine.strategy-mock";

    struct MockSession {
        pack: WorldPackRef,
        count: usize,
        mutations: Arc<AtomicUsize>,
    }

    impl WorldSession for MockSession {
        fn pack(&self) -> WorldPackRef {
            self.pack.clone()
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: format!("Mock {}", self.count),
                world_time: self.count as u64,
                capabilities: ProjectionCapabilities { fork: false },
                commands: vec![
                    ProjectionCommand {
                        id: "mock.advance".into(),
                        title: "Advance".into(),
                        detail: "Advance mock state".into(),
                    },
                    ProjectionCommand {
                        id: "mock.fail".into(),
                        title: "Fail".into(),
                        detail: "Fail without mutation".into(),
                    },
                ],
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            match intent {
                ProjectionIntent::InvokeCommand(command) if command == "mock.advance" => {
                    self.count += 1;
                    self.mutations.fetch_add(1, Ordering::SeqCst);
                    Ok(self.snapshot())
                }
                ProjectionIntent::InvokeCommand(command) if command == "mock.fail" => {
                    Err(HostError::session("intent failed"))
                }
                _ => Err(HostError::session("unsupported mock intent")),
            }
        }

        fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
            self.count = self
                .count
                .checked_add(periods as usize)
                .ok_or_else(|| HostError::session("mock count overflow"))?;
            self.mutations.fetch_add(periods as usize, Ordering::SeqCst);
            Ok(self.snapshot())
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(mock_archive(self.count as u64)))
        }
    }

    fn mock_archive(world_time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new(MOCK_PACK, "1"),
            world_time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn mock_registry(
        left_mutations: Arc<AtomicUsize>,
        right_mutations: Arc<AtomicUsize>,
    ) -> WorldRegistry {
        let pack = WorldPackRef::new(MOCK_PACK, "1");
        let opener_pack = pack.clone();
        let opens = Arc::new(AtomicUsize::new(0));
        let opener_opens = Arc::clone(&opens);
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack,
                        title: "Strategy Mock".into(),
                        description: "Strategy harness isolation test".into(),
                    },
                    || Err(HostError::session("factory is unused")),
                )
                .with_archive_opener(move |archive| {
                    let open_index = opener_opens.fetch_add(1, Ordering::SeqCst);
                    let mutations = if open_index == 0 {
                        Arc::clone(&left_mutations)
                    } else {
                        Arc::clone(&right_mutations)
                    };
                    Ok(Box::new(MockSession {
                        pack: opener_pack.clone(),
                        count: archive.world_time as usize,
                        mutations,
                    }))
                }),
            )
            .unwrap();
        registry
    }

    fn success(run: &StrategyRun) -> &StrategyOutcome {
        match run {
            StrategyRun::Success(outcome) => outcome,
            StrategyRun::Failure(error) => panic!("expected strategy success: {error}"),
        }
    }

    #[test]
    fn strategies_open_independent_sessions_and_advance_equally() {
        let left_mutations = Arc::new(AtomicUsize::new(0));
        let right_mutations = Arc::new(AtomicUsize::new(0));
        let registry = mock_registry(Arc::clone(&left_mutations), Arc::clone(&right_mutations));
        let source = mock_archive(5);
        let source_before = source.clone();

        let result = evaluate_strategies(
            &registry,
            &source,
            &StrategyPlan::new()
                .command("mock.advance")
                .background_periods(3),
            &StrategyPlan::new().background_periods(3),
        );
        let left = success(&result.left);
        let right = success(&result.right);
        let comparison = result.comparison.as_ref().expect("both strategies succeed");

        assert_eq!(source, source_before);
        assert_eq!(left.snapshot.world_time, 9);
        assert_eq!(right.snapshot.world_time, 8);
        assert_eq!(left_mutations.load(Ordering::SeqCst), 4);
        assert_eq!(right_mutations.load(Ordering::SeqCst), 3);
        assert_eq!(comparison.left.world_time, 9);
        assert_eq!(comparison.right.world_time, 8);
    }

    #[test]
    fn left_strategy_failure_does_not_block_right_or_mutate_source() {
        let left_mutations = Arc::new(AtomicUsize::new(0));
        let right_mutations = Arc::new(AtomicUsize::new(0));
        let registry = mock_registry(Arc::clone(&left_mutations), Arc::clone(&right_mutations));
        let source = mock_archive(7);
        let source_before = source.clone();

        let result = evaluate_strategies(
            &registry,
            &source,
            &StrategyPlan::new().command("mock.fail"),
            &StrategyPlan::new().command("mock.advance"),
        );

        assert_eq!(source, source_before);
        let left_error = result.left.error().expect("left strategy fails");
        assert_eq!(left_error.side, StrategySide::Left);
        assert_eq!(left_error.stage, StrategyStage::Intent(0));
        assert_eq!(left_mutations.load(Ordering::SeqCst), 0);
        assert_eq!(right_mutations.load(Ordering::SeqCst), 1);
        assert_eq!(success(&result.right).snapshot.world_time, 8);
        assert!(result.comparison.is_none());
    }

    #[test]
    fn tiny_society_recovery_strategies_diverge_from_the_same_durable_archive() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.advance_days(120).unwrap();
        let source = branch.archive().unwrap();
        let source_before = source.clone();

        let mut registry = WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let evaluation = evaluate_strategies(
            &registry,
            &source,
            &StrategyPlan::new()
                .command(REOPEN_BAKERY_COMMAND)
                .background_periods(20),
            &StrategyPlan::new()
                .command(LEAN_REOPEN_BAKERY_COMMAND)
                .background_periods(20),
        );
        let left = success(&evaluation.left);
        let right = success(&evaluation.right);
        let comparison = evaluation
            .comparison
            .as_ref()
            .expect("both Tiny Society strategies succeed");

        assert_eq!(source, source_before);
        assert_eq!(
            comparison
                .entities
                .iter()
                .find(|difference| difference.id == SelectionId::Entity(BAKERY))
                .expect("Bakery state diverges")
                .kind,
            DifferenceKind::Changed
        );
        assert_eq!(
            comparison
                .entities
                .iter()
                .find(|difference| difference.id == SelectionId::Entity(MARA))
                .expect("Mara state diverges")
                .kind,
            DifferenceKind::Changed
        );
        assert!(comparison
            .timeline
            .changed
            .iter()
            .any(|event| event.left.title != event.right.title));
        assert!(comparison.entities.iter().any(|difference| {
            difference.id == SelectionId::Entity(BAKERY)
                && difference
                    .inspector_rows
                    .iter()
                    .any(|row| row.left != row.right)
        }));

        let durable_right = right
            .archive
            .as_ref()
            .expect("Tiny Society exports a durable strategy archive");
        let reopened_right = registry.open_archive(durable_right).unwrap();
        let replay_comparison = compare_snapshots(&right.snapshot, &reopened_right.snapshot());
        assert!(replay_comparison.is_identical());
        assert!(left.archive.is_some());
    }
}
