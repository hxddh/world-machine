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

    /// The canvas draws a line for every relation whose two ends it places.
    ///
    /// This is the coverage check for that drawing, and it lives here rather
    /// than in the renderer because the renderer only compiles on macOS. It
    /// also guards a gap that a screenshot alone would have hidden: Pocket
    /// Universe declares no `Relation` at all — it models a relationship as an
    /// entity — so every screenshot of that Pack shows an edgeless canvas no
    /// matter how correct the drawing is. Tiny Society is where there is
    /// something to draw, so this is where the claim can be tested.
    /// However many people and places this World has, no two of them are
    /// drawn on top of each other.
    ///
    /// Its own coordinate table piled them up once the boxes were drawn at a
    /// legible size, which is why the canvas resolves positions rather than
    /// trusting them.
    #[test]
    fn no_two_things_in_this_world_are_drawn_on_top_of_each_other() {
        use world_projection::canvas_layout;

        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        session.advance_background(4).unwrap();
        let snapshot = session.snapshot();

        let drawn = snapshot
            .canvas_placements()
            .into_iter()
            .map(|(_, x, y)| (x, y))
            .collect::<Vec<_>>();
        assert!(drawn.len() > 3, "this World should have things to place");
        assert!(
            !canvas_layout::any_overlap(&drawn),
            "{} things overlap: {drawn:?}",
            drawn.len()
        );
    }

    #[test]
    fn the_canvas_has_a_line_for_every_relation_it_places() {
        use world_projection::SelectionId;

        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        session.advance_background(4).unwrap();
        let snapshot = session.snapshot();

        let placed = snapshot
            .canvas
            .items
            .iter()
            .filter_map(|item| match item.id {
                SelectionId::Entity(entity) => Some(entity),
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        let drawable = snapshot
            .inspectors
            .keys()
            .filter_map(|selection| match selection {
                SelectionId::Relation(relation) => Some(*relation),
                _ => None,
            })
            .filter(|relation| {
                snapshot
                    .relation_identity(*relation)
                    .is_some_and(|identity| {
                        placed.contains(&identity.from) && placed.contains(&identity.to)
                    })
            })
            .count();

        let edges = snapshot.canvas_edges();
        assert!(
            drawable > 0,
            "this World is meant to be the one with relations to draw"
        );
        assert_eq!(edges.len(), drawable, "{edges:?}");
        for edge in &edges {
            assert!(placed.contains(&edge.from) && placed.contains(&edge.to));
        }
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
