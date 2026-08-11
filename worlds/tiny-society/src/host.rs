use crate::{tiny_society_pack_ref, TinySociety, TinySocietyBranch, VisitCursor};
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
use world_persistence::WorldArchive;
use world_projection::{ProjectionIntent, ProjectionSnapshot};

struct TinySocietySession {
    branch: TinySocietyBranch,
    background_cursor: Option<VisitCursor>,
}

impl TinySocietySession {
    fn fresh() -> Result<Box<dyn WorldSession>, HostError> {
        let mut society = TinySociety::new().map_err(HostError::session)?;
        society.run_story().map_err(HostError::session)?;
        Ok(Box::new(Self {
            branch: society.branch(),
            background_cursor: None,
        }))
    }

    fn open_archive(archive: &WorldArchive) -> Result<Box<dyn WorldSession>, HostError> {
        let society = TinySociety::resume_archive(archive).map_err(HostError::session)?;
        Ok(Box::new(Self {
            branch: society.branch(),
            background_cursor: None,
        }))
    }
}

impl WorldSession for TinySocietySession {
    fn pack(&self) -> world_persistence::WorldPackRef {
        tiny_society_pack_ref()
    }

    fn snapshot(&self) -> ProjectionSnapshot {
        match self.background_cursor {
            Some(cursor) => self.branch.projection_snapshot_since(cursor),
            None => self.branch.projection_snapshot(),
        }
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
        self.background_cursor = None;
        Ok(self.snapshot())
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        if periods == 0 {
            return Ok(self.snapshot());
        }
        let cursor = self.branch.visit_cursor();
        self.branch
            .advance_days(periods)
            .map_err(HostError::session)?;
        self.background_cursor = Some(cursor);
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        self.branch.archive().map(Some).map_err(HostError::session)
    }
}

pub fn tiny_society_registration() -> WorldRegistration {
    WorldRegistration::new(
        WorldDescriptor {
            pack: tiny_society_pack_ref(),
            title: "Tiny Society".into(),
            description:
                "A persistent harbor town where relationships and consequences become history."
                    .into(),
        },
        TinySocietySession::fresh,
    )
    .with_archive_opener(TinySocietySession::open_archive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_projection::{ProjectionIntent, SelectionId};

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
            if let SelectionId::Event(id) = event.id {
                session
                    .handle(ProjectionIntent::ForkBeforeEvent(id))
                    .unwrap();
            }
        }
    }

    #[test]
    fn background_periods_preserve_a_transient_return_briefing() {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        let before = session.snapshot();

        let after = session.advance_background(2).unwrap();

        assert_eq!(after.world_time, before.world_time + 20);
        assert!(after.timeline.items.len() >= before.timeline.items.len() + 6);
        let briefing = after
            .briefing
            .as_ref()
            .expect("Tiny Society has a briefing");
        assert_eq!(briefing.title, "While you were away");
        assert!(briefing
            .items
            .iter()
            .any(|item| item.title == "The world moved forward"));
        assert_eq!(
            session
                .snapshot()
                .briefing
                .expect("return briefing remains visible")
                .title,
            "While you were away"
        );

        let archive = session.archive().unwrap().unwrap();
        let reopened = registry.open_archive(&archive).unwrap();
        assert_eq!(reopened.snapshot().world_time, after.world_time);
        assert_eq!(reopened.snapshot().timeline.items, after.timeline.items);
        assert_eq!(
            reopened
                .snapshot()
                .briefing
                .expect("reopened Tiny Society has a briefing")
                .title,
            "Life happened while you were away"
        );
    }

    #[test]
    fn world_interaction_clears_the_transient_return_briefing() {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        let after = session.advance_background(1).unwrap();
        let event = after
            .timeline
            .items
            .last()
            .and_then(|item| match item.id {
                SelectionId::Event(id) => Some(id),
                _ => None,
            })
            .expect("background progression creates timeline events");

        let snapshot = session
            .handle(ProjectionIntent::ForkBeforeEvent(event))
            .unwrap();

        assert_eq!(
            snapshot
                .briefing
                .expect("Tiny Society has a briefing after interaction")
                .title,
            "Life happened while you were away"
        );
    }
}
