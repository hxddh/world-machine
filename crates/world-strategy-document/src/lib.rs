use std::error::Error;
use std::fmt;
use world_host::WorldRegistry;
use world_library::{DurableWorldSession, LibraryError};
use world_strategy::{evaluate_strategies, StrategyEvaluation, StrategyPlan};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyChoice {
    pub id: String,
    pub title: String,
    pub detail: String,
}

pub fn available_choices(session: &DurableWorldSession) -> Vec<StrategyChoice> {
    session
        .snapshot()
        .commands
        .into_iter()
        .map(|command| StrategyChoice {
            id: command.id,
            title: command.title,
            detail: command.detail,
        })
        .collect()
}

pub fn evaluate_choices(
    session: &DurableWorldSession,
    registry: &WorldRegistry,
    left_choice: &str,
    right_choice: &str,
    background_periods: u64,
) -> Result<StrategyEvaluation, StrategyDocumentError> {
    if left_choice == right_choice {
        return Err(StrategyDocumentError::SameChoice(left_choice.to_owned()));
    }

    let choices = available_choices(session);
    require_choice(&choices, left_choice)?;
    require_choice(&choices, right_choice)?;

    let source = session.current_archive()?;
    Ok(evaluate_strategies(
        registry,
        &source,
        &StrategyPlan::new()
            .command(left_choice)
            .background_periods(background_periods),
        &StrategyPlan::new()
            .command(right_choice)
            .background_periods(background_periods),
    ))
}

pub fn evaluate_first_two(
    session: &DurableWorldSession,
    registry: &WorldRegistry,
    background_periods: u64,
) -> Result<(StrategyChoice, StrategyChoice, StrategyEvaluation), StrategyDocumentError> {
    let choices = available_choices(session);
    let left = choices
        .first()
        .cloned()
        .ok_or(StrategyDocumentError::NotEnoughChoices(choices.len()))?;
    let right = choices
        .get(1)
        .cloned()
        .ok_or(StrategyDocumentError::NotEnoughChoices(choices.len()))?;
    let evaluation = evaluate_choices(session, registry, &left.id, &right.id, background_periods)?;
    Ok((left, right, evaluation))
}

fn require_choice(
    choices: &[StrategyChoice],
    choice_id: &str,
) -> Result<(), StrategyDocumentError> {
    if choices.iter().any(|choice| choice.id == choice_id) {
        Ok(())
    } else {
        Err(StrategyDocumentError::UnknownChoice(choice_id.to_owned()))
    }
}

#[derive(Debug)]
pub enum StrategyDocumentError {
    Library(LibraryError),
    UnknownChoice(String),
    SameChoice(String),
    NotEnoughChoices(usize),
}

impl fmt::Display for StrategyDocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Library(error) => error.fmt(f),
            Self::UnknownChoice(choice) => write!(f, "unknown strategy choice: {choice}"),
            Self::SameChoice(choice) => {
                write!(
                    f,
                    "strategy comparison requires two different choices: {choice}"
                )
            }
            Self::NotEnoughChoices(count) => write!(
                f,
                "strategy comparison requires at least two available choices; found {count}"
            ),
        }
    }
}

impl Error for StrategyDocumentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Library(error) => Some(error),
            Self::UnknownChoice(_) | Self::SameChoice(_) | Self::NotEnoughChoices(_) => None,
        }
    }
}

impl From<LibraryError> for StrategyDocumentError {
    fn from(error: LibraryError) -> Self {
        Self::Library(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
    use world_library::{WorldDocumentId, WorldLibrary};
    use world_persistence::{
        WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION,
    };
    use world_projection::{
        ProjectionCapabilities, ProjectionCommand, ProjectionIntent, ProjectionSnapshot,
    };

    const MOCK_PACK: &str = "world-machine.strategy-document-mock";

    struct MockSession {
        count: u64,
    }

    impl WorldSession for MockSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(MOCK_PACK, "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: format!("Strategy Document Mock {}", self.count),
                world_time: self.count,
                capabilities: ProjectionCapabilities { fork: false },
                commands: vec![
                    ProjectionCommand {
                        id: "mock.left".into(),
                        title: "Left choice".into(),
                        detail: "Advance once".into(),
                    },
                    ProjectionCommand {
                        id: "mock.right".into(),
                        title: "Right choice".into(),
                        detail: "Advance twice".into(),
                    },
                ],
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            match intent {
                ProjectionIntent::InvokeCommand(command) if command == "mock.left" => {
                    self.count += 1;
                }
                ProjectionIntent::InvokeCommand(command) if command == "mock.right" => {
                    self.count += 2;
                }
                _ => return Err(HostError::session("unsupported mock strategy choice")),
            }
            Ok(self.snapshot())
        }

        fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
            self.count += periods;
            Ok(self.snapshot())
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(WorldArchive {
                format: WORLD_ARCHIVE_FORMAT.into(),
                format_version: WORLD_ARCHIVE_VERSION,
                pack: self.pack(),
                world_time: self.count,
                events: Vec::new(),
                pending: Vec::new(),
            }))
        }
    }

    fn registry() -> WorldRegistry {
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: WorldPackRef::new(MOCK_PACK, "1"),
                        title: "Strategy Document Mock".into(),
                        description: "Durable strategy adapter regression".into(),
                    },
                    || Ok(Box::new(MockSession { count: 0 })),
                )
                .with_archive_opener(|archive| {
                    Ok(Box::new(MockSession {
                        count: archive.world_time,
                    }))
                }),
            )
            .unwrap();
        registry
    }

    fn temp_root() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-strategy-document-{}-{nonce}",
            process::id()
        ))
    }

    fn durable_session() -> (
        std::path::PathBuf,
        WorldRegistry,
        WorldLibrary,
        DurableWorldSession,
    ) {
        let root = temp_root();
        let library = WorldLibrary::new(root.join("library"));
        let registry = registry();
        let session = DurableWorldSession::create(
            WorldDocumentId::new("strategy-source").unwrap(),
            MOCK_PACK,
            &registry,
            &library,
        )
        .unwrap();
        (root, registry, library, session)
    }

    #[test]
    fn choices_are_derived_from_the_current_projection_commands() {
        let (root, _registry, _library, session) = durable_session();

        let choices = available_choices(&session);

        assert_eq!(choices.len(), 2);
        assert_eq!(choices[0].id, "mock.left");
        assert_eq!(choices[1].title, "Right choice");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn evaluation_uses_one_checked_archive_without_mutating_the_open_document() {
        let (root, registry, _library, session) = durable_session();
        let source_before = session.current_archive().unwrap();

        let evaluation = evaluate_choices(&session, &registry, "mock.left", "mock.right", 3)
            .expect("strategy evaluation succeeds");

        assert_eq!(session.snapshot().world_time, 0);
        assert_eq!(session.current_archive().unwrap(), source_before);
        assert_eq!(evaluation.left.outcome().unwrap().snapshot.world_time, 4);
        assert_eq!(evaluation.right.outcome().unwrap().snapshot.world_time, 5);
        assert!(evaluation.comparison.is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_or_duplicate_choices_fail_before_evaluation() {
        let (root, registry, _library, session) = durable_session();

        assert!(matches!(
            evaluate_choices(&session, &registry, "missing", "mock.right", 3),
            Err(StrategyDocumentError::UnknownChoice(choice)) if choice == "missing"
        ));
        assert!(matches!(
            evaluate_choices(&session, &registry, "mock.left", "mock.left", 3),
            Err(StrategyDocumentError::SameChoice(choice)) if choice == "mock.left"
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn first_two_requires_two_projection_commands() {
        let (root, registry, library, mut session) = durable_session();
        session
            .handle(
                ProjectionIntent::InvokeCommand("mock.left".into()),
                &registry,
                &library,
            )
            .unwrap();

        let (left, right, evaluation) = evaluate_first_two(&session, &registry, 2).unwrap();

        assert_eq!(left.id, "mock.left");
        assert_eq!(right.id, "mock.right");
        assert!(evaluation.comparison.is_some());
        let _ = fs::remove_dir_all(root);
    }
}
