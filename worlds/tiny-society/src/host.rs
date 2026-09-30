use crate::{tiny_society_pack_ref, TinySociety, TinySocietyBranch, VisitCursor};
use std::sync::Arc;
use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
use world_persistence::WorldArchive;
use world_projection::{
    Ears, ProjectionIntent, ProjectionSnapshot, SelectionId, StoryPage, StoryRequest,
};

/// Makes the listener each session hears the player's words with.
pub type ListenerFactory = Arc<dyn Fn() -> Box<dyn conversation::Listener> + Send + Sync>;

struct TinySocietySession {
    branch: TinySocietyBranch,
    background_cursor: Option<VisitCursor>,
    listener: Box<dyn conversation::Listener>,
}

impl TinySocietySession {
    fn fresh(
        listener: Box<dyn conversation::Listener>,
    ) -> Result<Box<dyn WorldSession>, HostError> {
        let branch = TinySocietyBranch::new_world().map_err(HostError::session)?;
        Ok(Box::new(Self {
            branch,
            background_cursor: None,
            listener,
        }))
    }

    fn open_archive(
        archive: &WorldArchive,
        listener: Box<dyn conversation::Listener>,
    ) -> Result<Box<dyn WorldSession>, HostError> {
        let society = TinySociety::resume_archive(archive).map_err(HostError::session)?;
        Ok(Box::new(Self {
            branch: society.branch(),
            background_cursor: None,
            listener,
        }))
    }

    /// As [`Self::open_archive`], moving the archive's history into the
    /// World rather than copying it.
    fn open_owned_archive(
        archive: WorldArchive,
        listener: Box<dyn conversation::Listener>,
    ) -> Result<Box<dyn WorldSession>, HostError> {
        let society = TinySociety::resume_owned_archive(archive).map_err(HostError::session)?;
        Ok(Box::new(Self {
            branch: society.branch(),
            background_cursor: None,
            listener,
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
            ProjectionIntent::Say { to, words, ears } => {
                let SelectionId::Entity(who) = to else {
                    return Err(HostError::session(std::io::Error::other(
                        "only someone can be spoken to",
                    )));
                };
                let mut answered;
                let mut own = conversation::OwnEars;
                let listener: &mut dyn conversation::Listener = match ears {
                    Ears::World => self.listener.as_mut(),
                    Ears::Model(response) => {
                        answered = conversation::Answered(response);
                        &mut answered
                    }
                    Ears::Own => &mut own,
                };
                self.branch
                    .say_with(who, &words, listener)
                    .map_err(HostError::session)?;
            }
            ProjectionIntent::Host(guest) => {
                self.branch.host(&guest).map_err(HostError::session)?;
            }
        }
        self.background_cursor = None;
        Ok(self.snapshot())
    }

    fn story(&self, request: StoryRequest) -> Result<Option<StoryPage>, HostError> {
        Ok(self.branch.story(request))
    }

    fn hearing(&self, to: SelectionId, words: &str) -> Result<Option<String>, HostError> {
        Ok(match to {
            SelectionId::Entity(who) => crate::speech::prompt(self.branch.world(), who, words),
            _ => None,
        })
    }

    fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
        if periods == 0 {
            return Ok(self.snapshot());
        }
        let cursor = self.branch.visit_cursor();
        self.branch
            .advance_days(periods)
            .map_err(HostError::session)?;
        self.branch
            .leave_keepsake(cursor)
            .map_err(HostError::session)?;
        self.background_cursor = Some(cursor);
        Ok(self.snapshot())
    }

    fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
        self.branch.archive().map(Some).map_err(HostError::session)
    }

    /// Where the session stands: the World, and where a return began, the
    /// only other thing a change moves. The listener keeps nothing a change
    /// makes.
    fn checkpoint(&mut self) -> Result<Option<world_host::SessionCheckpoint>, HostError> {
        Ok(Some(world_host::SessionCheckpoint::new((
            self.branch.world.checkpoint(),
            self.background_cursor,
        ))))
    }

    fn rollback(&mut self, checkpoint: world_host::SessionCheckpoint) -> Result<(), HostError> {
        let (world, cursor): (world_core::Checkpoint, Option<VisitCursor>) =
            checkpoint.into_inner()?;
        self.branch.world.rollback(world);
        self.background_cursor = cursor;
        Ok(())
    }

    fn archive_since(&self, from: usize) -> Result<Option<WorldArchive>, HostError> {
        WorldArchive::capture_since(tiny_society_pack_ref(), self.branch.world(), from)
            .map(Some)
            .map_err(HostError::session)
    }
}

pub fn tiny_society_registration() -> WorldRegistration {
    tiny_society_registration_with_listener(Arc::new(|| Box::new(conversation::OwnEars)))
}

/// The harbour, its people hearing the player with listeners from
/// `listener`, such as a language model the player switched on.
pub fn tiny_society_registration_with_listener(listener: ListenerFactory) -> WorldRegistration {
    let opening = Arc::clone(&listener);
    let taking = Arc::clone(&listener);
    WorldRegistration::new(
        WorldDescriptor {
            pack: tiny_society_pack_ref(),
            title: "Tiny Society".into(),
            description:
                "A small harbour town that keeps living while you are away, where friendships, money and luck become its history."
                    .into(),
        },
        move || TinySocietySession::fresh(listener()),
    )
    .with_archive_opener(move |archive| TinySocietySession::open_archive(archive, opening()))
    .with_owned_archive_opener(move |archive| {
        TinySocietySession::open_owned_archive(archive, taking())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The harbour as a session that cannot go back, so a host takes the
    /// old way of saving it: the whole archive, reopened.
    struct CannotGoBack(Box<dyn WorldSession>);

    impl WorldSession for CannotGoBack {
        fn pack(&self) -> world_persistence::WorldPackRef {
            self.0.pack()
        }
        fn snapshot(&self) -> ProjectionSnapshot {
            self.0.snapshot()
        }
        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            self.0.handle(intent)
        }
        fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
            self.0.advance_background(periods)
        }
        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            self.0.archive()
        }
    }

    fn old_way_registry() -> world_host::WorldRegistry {
        let own = Arc::new(|| Box::new(conversation::OwnEars) as Box<dyn conversation::Listener>);
        let opening = Arc::clone(&own);
        let mut registry = world_host::WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: tiny_society_pack_ref(),
                        title: "Tiny Society".into(),
                        description: "The harbour, saved the old way.".into(),
                    },
                    move || {
                        Ok(Box::new(CannotGoBack(TinySocietySession::fresh(own())?))
                            as Box<dyn WorldSession>)
                    },
                )
                .with_archive_opener(move |archive| {
                    Ok(Box::new(CannotGoBack(TinySocietySession::open_archive(
                        archive,
                        opening(),
                    )?)) as Box<dyn WorldSession>)
                }),
            )
            .unwrap();
        registry
    }

    fn temp_library(label: &str) -> (std::path::PathBuf, world_library::WorldLibrary) {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "tiny-society-{label}-{}-{nonce}",
            std::process::id()
        ));
        (root.clone(), world_library::WorldLibrary::new(root))
    }

    /// Twenty days let pass on the harbour itself, gone back to from a
    /// checkpoint whenever a change is not kept, write the very file the
    /// old way does, reopening the whole archive for each.
    #[test]
    fn days_saved_on_the_world_itself_write_what_the_old_way_writes() {
        let pass = || ProjectionIntent::InvokeCommand(crate::story::WAIT_COMMAND.into());
        let id = world_library::WorldDocumentId::new("harbour").unwrap();
        let mut files = Vec::new();
        for (label, registry) in [
            ("kept", {
                let mut registry = world_host::WorldRegistry::new();
                registry.register(tiny_society_registration()).unwrap();
                registry
            }),
            ("reopened", old_way_registry()),
        ] {
            let (root, library) = temp_library(label);
            drop(
                world_library::DurableWorldSession::create(
                    id.clone(),
                    crate::TINY_SOCIETY_PACK_ID,
                    &registry,
                    &library,
                )
                .unwrap(),
            );
            let mut session =
                world_library::DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
            for _ in 0..20 {
                session.handle(pass(), &registry, &library).unwrap();
            }
            files.push((
                library.load_document(&id).unwrap().unwrap(),
                session.snapshot(),
            ));
            let _ = std::fs::remove_dir_all(root);
        }
        let (kept, reopened) = (&files[0], &files[1]);
        assert_eq!(kept.0.archive, reopened.0.archive);
        assert_eq!(
            kept.0.metadata.display_title,
            reopened.0.metadata.display_title
        );
        assert_eq!(kept.1, reopened.1);
    }

    /// A change that cannot be saved is gone back from: the harbour's
    /// archive and what it shows are as they were.
    #[test]
    fn a_day_that_cannot_be_saved_leaves_the_harbour_as_it_was() {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let (root, library) = temp_library("unsaved");
        let id = world_library::WorldDocumentId::new("harbour").unwrap();
        drop(
            world_library::DurableWorldSession::create(
                id.clone(),
                crate::TINY_SOCIETY_PACK_ID,
                &registry,
                &library,
            )
            .unwrap(),
        );
        let mut session =
            world_library::DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
        let pass = || ProjectionIntent::InvokeCommand(crate::story::WAIT_COMMAND.into());
        session.handle(pass(), &registry, &library).unwrap();
        let archive_before = session.current_archive().unwrap();
        let snapshot_before = session.snapshot();
        // Where the World file goes, a folder stands: it cannot be written.
        let path = library.path(&id);
        let file_before = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir_all(path.join("in the way")).unwrap();
        assert!(session.handle(pass(), &registry, &library).is_err());
        assert_eq!(session.current_archive().unwrap(), archive_before);
        assert_eq!(session.snapshot(), snapshot_before);
        // Once it can be written again, the harbour carries on from there.
        std::fs::remove_dir_all(&path).unwrap();
        std::fs::write(&path, &file_before).unwrap();
        let next = session.handle(pass(), &registry, &library).unwrap();
        assert!(next.world_time > snapshot_before.world_time);
        let _ = std::fs::remove_dir_all(root);
    }

    /// The choices that can be made now.
    fn offered(snapshot: &world_projection::ProjectionSnapshot) -> Vec<String> {
        snapshot
            .commands
            .iter()
            .filter(|command| command.unavailable.is_none())
            .map(|command| command.id.clone())
            .collect()
    }
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
                let offered = offered(&snapshot);
                let command = offered[turn % offered.len()].clone();
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

    /// The harbour window is a place, not a page: at rest it shows at most
    /// forty words, bar included, over days and a return.
    #[test]
    fn the_harbour_window_shows_a_place_not_a_page() {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        let mut snapshot = session.snapshot();
        for turn in 0..16 {
            let words = world_gpui::words_at_rest(&snapshot);
            assert!(
                words <= world_gpui::RESTING_WORD_LIMIT,
                "turn {turn}: {words} words at rest"
            );
            snapshot = if turn % 4 == 3 || snapshot.commands.is_empty() {
                session.advance_background(3).unwrap()
            } else {
                let offered = offered(&snapshot);
                let command = offered[turn % offered.len()].clone();
                session
                    .handle(ProjectionIntent::InvokeCommand(command))
                    .unwrap()
            };
        }
    }

    /// Someone says something every day, over whoever it happened to;
    /// everyone can be asked three things, and an answer that asks for
    /// something names a choice that is really on offer.
    #[test]
    fn the_harbour_speaks_and_answers_and_asks_only_for_what_is_on_offer() {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        let mut snapshot = session.advance_background(2).unwrap();
        let mut asked_for = 0;
        for turn in 0..12 {
            let on_scene = |id: SelectionId| snapshot.canvas.items.iter().any(|item| item.id == id);
            let on_timeline =
                |id: SelectionId| snapshot.timeline.items.iter().any(|item| item.id == id);
            assert!(!snapshot.voices.is_empty(), "turn {turn}: nobody spoke");
            for voice in &snapshot.voices {
                assert!(on_scene(voice.speaker), "{voice:?}");
                assert!(on_timeline(voice.moment), "{voice:?}");
            }
            assert_eq!(snapshot.talks.len(), 24);
            for talk in &snapshot.talks {
                assert!(on_scene(talk.who));
                if let Some(command) = &talk.asks_for {
                    assert!(snapshot.command(command).is_some(), "{talk:?}");
                    asked_for += 1;
                }
            }
            assert!(snapshot
                .canvas
                .items
                .iter()
                .filter(|item| item.kind == world_projection::CanvasItemKind::Actor)
                .all(|person| person.look.is_some()));
            snapshot = if snapshot.commands.is_empty() {
                session.advance_background(1).unwrap()
            } else {
                let offered = offered(&snapshot);
                let command = offered[turn % offered.len()].clone();
                session
                    .handle(ProjectionIntent::InvokeCommand(command))
                    .unwrap()
            };
        }
        assert!(asked_for > 0, "somebody should ask for something");
    }

    /// Anyone in the harbour can be spoken to in the player's own words:
    /// they answer in their own, the exchange is recorded and survives a
    /// reopen, and no day passes.
    #[test]
    fn people_answer_what_you_say_in_your_own_words() {
        let mut registry = world_host::WorldRegistry::new();
        registry.register(tiny_society_registration()).unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        let before = session.snapshot();
        assert!(before.capabilities.talk);
        let mara = SelectionId::Entity(crate::MARA);
        let mut answers = Vec::new();
        for words in [
            "Hello Mara!",
            "How are you?",
            "What do you think of Leo?",
            "How's the bakery?",
            "Anything I can do to help?",
            "Your bread is wonderful.",
            "blorp",
        ] {
            let snapshot = session
                .handle(ProjectionIntent::Say {
                    to: mara,
                    words: words.into(),
                    ears: world_projection::Ears::World,
                })
                .unwrap();
            let exchange = snapshot.exchanges_with(mara).last().unwrap().clone();
            assert_eq!(exchange.words, words);
            assert!(!exchange.answer.is_empty());
            let engine = world_projection::engine_words_in(&exchange.answer);
            assert!(engine.is_empty(), "{engine:?} in {:?}", exchange.answer);
            if let Some(command) = &exchange.asks_for {
                assert!(snapshot.command(command).is_some(), "{exchange:?}");
            }
            assert_eq!(snapshot.world_time, before.world_time);
            answers.push(exchange.answer);
        }
        let unique = answers.iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), answers.len(), "{answers:#?}");
        assert!(answers[2].contains("Leo"), "{answers:#?}");

        let archive = session.archive().unwrap().unwrap();
        let reopened = registry.open_archive(&archive).unwrap();
        assert_eq!(reopened.snapshot().exchanges, session.snapshot().exchanges);
        assert!(session
            .handle(ProjectionIntent::Say {
                to: SelectionId::Entity(crate::BAKERY),
                words: "hello".into(),
                ears: world_projection::Ears::World,
            })
            .is_err());
        let next_day = session.advance_background(1).unwrap();
        assert!(next_day.exchanges.is_empty());
    }

    /// An app that asks a model itself gets the prompt from the World,
    /// changing nothing, and says the words with the model's response:
    /// taken as a proposal when it can be read, the World's own answer when
    /// it cannot or when the app gave up waiting.
    #[test]
    fn an_app_can_ask_the_model_itself_and_the_world_still_decides() {
        struct Loud;
        impl conversation::Listener for Loud {
            fn listen(&mut self, _: &conversation::Hearing) -> Option<conversation::Listened> {
                Some(conversation::Listened {
                    meaning: "greet".into(),
                    about: None,
                    answer: "The session's own model says hello.".into(),
                })
            }
        }
        let mut registry = world_host::WorldRegistry::new();
        registry
            .register(tiny_society_registration_with_listener(Arc::new(|| {
                Box::new(Loud)
            })))
            .unwrap();
        let mut session = registry.create(crate::TINY_SOCIETY_PACK_ID).unwrap();
        let mara = SelectionId::Entity(crate::MARA);
        let before = session.archive().unwrap().unwrap();
        let prompt = session.hearing(mara, "How's Leo?").unwrap().unwrap();
        assert!(
            prompt.contains("Mara") && prompt.contains("How's Leo?"),
            "{prompt}"
        );
        assert_eq!(session.archive().unwrap().unwrap(), before);
        assert_eq!(
            session
                .hearing(SelectionId::Entity(crate::BAKERY), "hi")
                .unwrap(),
            None
        );

        let said = |session: &mut Box<dyn WorldSession>, words: &str, ears| {
            session
                .handle(ProjectionIntent::Say {
                    to: mara,
                    words: words.into(),
                    ears,
                })
                .unwrap()
                .exchanges_with(mara)
                .last()
                .unwrap()
                .answer
                .clone()
        };
        let modelled = said(
            &mut session,
            "How's Leo?",
            Ears::Model("MEANING: how_is\nABOUT: Leo\nREPLY: Leo's grand, thanks.".into()),
        );
        assert_eq!(modelled, "Leo's grand, thanks.");
        let unreadable = said(&mut session, "Hello!", Ears::Model("I am a teapot".into()));
        assert_ne!(unreadable, "The session's own model says hello.");
        assert!(!unreadable.is_empty());
        let own = said(&mut session, "Hello again!", Ears::Own);
        assert_ne!(own, "The session's own model says hello.");
        let world = said(&mut session, "Hi!", Ears::World);
        assert_eq!(world, "The session's own model says hello.");
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
        assert_eq!(ids, ["work", "money", "spirits"]);
        assert!(snapshot
            .gauges
            .iter()
            .all(|gauge| (0.0..=1.0).contains(&gauge.value)));
        for command in snapshot.choices() {
            if command.id == crate::story::WAIT_COMMAND {
                continue;
            }
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
            "The story so far"
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
            "The story so far"
        );
    }
}
