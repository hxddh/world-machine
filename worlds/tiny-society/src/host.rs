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
                "A small harbour town that keeps living while you are away, where friendships, money and luck become its history."
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

    /// Everything a player reads in the harbour town, over a first session
    /// and a return, speaks about the town and never about the engine.
    #[test]
    fn nothing_a_player_reads_is_in_engine_words() {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        let mut found = std::collections::BTreeSet::new();
        let mut snapshot = session.snapshot();
        for turn in 0..16 {
            for line in snapshot.visible_text() {
                for word in world_projection::engine_words_in(line) {
                    found.insert(format!("{word:?} in {line:?}"));
                }
            }
            snapshot = if turn % 4 == 3 || snapshot.commands.is_empty() {
                session.advance_background(3).unwrap()
            } else {
                let command = snapshot.commands[turn % snapshot.commands.len()].id.clone();
                session
                    .handle(ProjectionIntent::InvokeCommand(command))
                    .unwrap()
            };
        }
        assert!(
            found.is_empty(),
            "engine words:\n{}",
            found.into_iter().collect::<Vec<_>>().join("\n")
        );
    }

    #[test]
    fn the_town_keeps_score_and_its_choices_say_whose_they_are() {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        let snapshot = session.snapshot();
        let ids: Vec<&str> = snapshot
            .gauges
            .iter()
            .map(|gauge| gauge.id.as_str())
            .collect();
        assert_eq!(ids, ["work", "money", "bakery"]);
        assert!(snapshot
            .gauges
            .iter()
            .all(|gauge| (0.0..=1.0).contains(&gauge.value)));
        for command in &snapshot.commands {
            assert!(
                command.asker.is_some(),
                "{} is somebody's choice",
                command.id
            );
        }
        assert_eq!(crate::projection::with_thousands_for_test(1372), "1,372");
        assert_eq!(crate::projection::with_thousands_for_test(-5), "-5");
    }

    #[test]
    fn a_return_is_marked_as_one() {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        assert!(!session.snapshot().briefing.unwrap().returned);
        let back = session.advance_background(3).unwrap();
        let briefing = back.briefing.unwrap();
        assert!(briefing.returned, "{}", briefing.title);
        assert!(session.snapshot().briefing.unwrap().returned);
    }

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
            .any(|item| item.title == "The town kept working"));
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
