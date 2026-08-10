use crate::{tiny_society_pack_ref, TinySociety, TinySocietyBranch};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
use world_persistence::WorldArchive;
use world_projection::{ProjectionIntent, ProjectionSnapshot};

struct TinySocietySession {
    branch: TinySocietyBranch,
}

impl TinySocietySession {
    fn fresh() -> Result<Box<dyn WorldSession>, HostError> {
        let mut society = TinySociety::new().map_err(HostError::session)?;
        society.run_story().map_err(HostError::session)?;
        Ok(Box::new(Self {
            branch: society.branch(),
        }))
    }

    fn open_archive(archive: &WorldArchive) -> Result<Box<dyn WorldSession>, HostError> {
        let society = TinySociety::resume_archive(archive).map_err(HostError::session)?;
        Ok(Box::new(Self {
            branch: society.branch(),
        }))
    }
}

impl WorldSession for TinySocietySession {
    fn pack(&self) -> world_persistence::WorldPackRef {
        tiny_society_pack_ref()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        self.branch.projection_snapshot()
    }

    fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
        match intent {
            ProjectionIntent::ForkBeforeEvent(event) => self
                .branch
                .fork_before_event(event)
                .map_err(HostError::session)?,
            ProjectionIntent::InvokeCommand(command_id) => {
                self.branch
                    .invoke_projection_command(&command_id)
                    .map_err(HostError::session)?;
            }
        }
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        self.branch
            .archive()
            .map(Some)
            .map_err(HostError::session)
    }
}

pub fn tiny_society_registration() -> WorldRegistration {
    WorldRegistration::new(
        WorldDescriptor {
            pack: tiny_society_pack_ref(),
            title: "Tiny Society".into(),
            description: "A persistent harbor town where relationships and consequences become history."
                .into(),
        },
        TinySocietySession::fresh,
    )
    .with_archive_opener(TinySocietySession::open_archive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_projection::ProjectionIntent;

    #[test]
    fn registration_creates_and_reopens_the_same_world_history() {
        let registration = tiny_society_registration();
        let mut registry = world_host::WorldRegistry::new();
        registry.register(registration).unwrap();

        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        let initial = session.snapshot();
        assert!(initial.capabilities.fork);
        assert!(!initial.timeline.items.is_empty());

        let archive = session.archive().unwrap().unwrap();
        let reopened = registry.open_archive(&archive).unwrap();
        assert_eq!(reopened.snapshot().timeline.items, initial.timeline.items);

        if let Some(event) = initial.timeline.items.last() {
            if let world_projection::SelectionId::Event(id) = event.id {
                session
                    .handle(ProjectionIntent::ForkBeforeEvent(id))
                    .unwrap();
            }
        }
    }
}
