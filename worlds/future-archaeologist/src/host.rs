use crate::{future_archaeologist_pack_ref, FutureArchaeologist};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
use world_persistence::WorldArchive;
use world_projection::{ProjectionIntent, ProjectionSnapshot};

struct FutureArchaeologistSession {
    world: FutureArchaeologist,
}

impl FutureArchaeologistSession {
    fn fresh() -> Result<Box<dyn WorldSession>, HostError> {
        Ok(Box::new(Self {
            world: FutureArchaeologist::new().map_err(HostError::session)?,
        }))
    }

    fn open_archive(archive: &WorldArchive) -> Result<Box<dyn WorldSession>, HostError> {
        Ok(Box::new(Self {
            world: FutureArchaeologist::resume_archive(archive).map_err(HostError::session)?,
        }))
    }
}

impl WorldSession for FutureArchaeologistSession {
    fn pack(&self) -> world_persistence::WorldPackRef {
        future_archaeologist_pack_ref()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        self.world.projection_snapshot()
    }

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
        match intent {
            ProjectionIntent::InvokeCommand(command_id) => {
                self.world
                    .invoke_projection_command(&command_id)
                    .map_err(HostError::session)?;
            }
            ProjectionIntent::ForkBeforeEvent(_) => {
                return Err(HostError::Session(
                    "this fixed-truth world does not support timeline forks".into(),
                ));
            }
        }
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        self.world
            .archive()
            .map(Some)
            .map_err(HostError::session)
    }
}

pub fn future_archaeologist_registration() -> WorldRegistration {
    WorldRegistration::new(
        WorldDescriptor {
            pack: future_archaeologist_pack_ref(),
            title: "Future Archaeologist".into(),
            description:
                "Recover fragments from a future terminal without exposing the hidden ground truth."
                    .into(),
        },
        FutureArchaeologistSession::fresh,
    )
    .with_archive_opener(FutureArchaeologistSession::open_archive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DELETED_MESSAGE, FUTURE_ARCHAEOLOGIST_PACK_ID, RECOVER_MESSAGE_COMMAND};
    use world_projection::{ProjectionIntent, SelectionId};

    #[test]
    fn registration_reopens_recovered_evidence() {
        let mut registry = world_host::WorldRegistry::new();
        registry
            .register(future_archaeologist_registration())
            .unwrap();

        let mut session = registry.create(FUTURE_ARCHAEOLOGIST_PACK_ID).unwrap();
        assert!(!session.snapshot().capabilities.fork);
        session
            .handle(ProjectionIntent::InvokeCommand(
                RECOVER_MESSAGE_COMMAND.into(),
            ))
            .unwrap();

        let archive = session.archive().unwrap().unwrap();
        let reopened = registry.open_archive(&archive).unwrap();
        let snapshot = reopened.snapshot();
        assert!(snapshot.commands.is_empty());
        assert!(snapshot
            .collection
            .items
            .iter()
            .any(|item| item.id == SelectionId::Entity(DELETED_MESSAGE)));
    }
}
