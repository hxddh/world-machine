//! A change made to a durable World on the session that holds it: marked
//! first, handed to the World's writer, and kept, or gone back from if it
//! fails, so a change costs what it does, not the World opened again nor its
//! file written (see `writer`).

use world_host::{HostError, WorldSession};
use world_persistence::{
    ArchivedEvent, ArchivedScheduledAction, CheckpointFit, CompactHistory, WorldArchive,
    WorldPackRef,
};
use world_projection::ProjectionSnapshot;

use crate::{
    describe_all_but_drawings, drawing_values, drawn_in, next_display_title_after,
    required_archive, snapshot_display_title, DurableWorldSession, LibraryError, WorldLibrary,
};

/// What the World file holds, kept so a change writes out only what the
/// World recorded since it was last saved. The history itself is kept by
/// the World's writer (see [`crate::writer::Composer`]).
#[derive(Debug)]
pub(crate) struct Saved {
    /// How many events are saved.
    count: usize,
    /// The last event saved: its id and time.
    last: Option<(u64, u64)>,
    /// The events saved after the file's checkpoint, which the checkpoint
    /// sums up as their season passes.
    season: Vec<ArchivedEvent>,
    pack: WorldPackRef,
    world_time: u64,
    pending: Vec<ArchivedScheduledAction>,
    /// The drawings the file's description was last written from, so a
    /// change that draws the same ones keeps their written form rather
    /// than writing it again (none until the first change is saved).
    drawn: Vec<world_projection::Drawing>,
}

impl Saved {
    /// What a file holding `archive` keeps, its events written as `history`,
    /// when its checkpoint (if any) sums up the first of those events: the
    /// checkpoint is then carried on from season to season as the file's
    /// own is. The history goes to the World's writer.
    pub(crate) fn kept(
        archive: &WorldArchive,
        history: CompactHistory,
    ) -> Option<(Self, crate::writer::Composer)> {
        if history.len() != archive.events.len() {
            return None;
        }
        let settled = match &archive.checkpoint {
            None => 0,
            Some(checkpoint) if checkpoint.fit(archive) == Some(CheckpointFit::Within) => {
                checkpoint.events
            }
            Some(_) => return None,
        };
        let saved = Self {
            count: history.len(),
            last: history.last_event(),
            season: archive.events[settled..].to_vec(),
            pack: archive.pack.clone(),
            world_time: archive.world_time,
            pending: archive.pending.clone(),
            drawn: Vec::new(),
        };
        Some((saved, crate::writer::Composer::new(history)))
    }

    /// Checks that `tail` carries on from what is saved.
    fn followed_by(&self, tail: &WorldArchive) -> Result<(), LibraryError> {
        let refuse = |why: String| Err(LibraryError::Host(HostError::session(why)));
        if tail.pack != self.pack {
            return refuse(format!(
                "the World's archive changed identity: {}@{} became {}@{}",
                self.pack.id, self.pack.version, tail.pack.id, tail.pack.version
            ));
        }
        if tail.world_time < self.world_time {
            return refuse(format!(
                "the World's time went back from {} to {}",
                self.world_time, tail.world_time
            ));
        }
        let mut last = self.last;
        for event in &tail.events {
            if let Some((id, time)) = last {
                if event.id <= id || event.world_time < time {
                    return refuse(format!("event #{} does not follow event #{id}", event.id));
                }
            }
            if event.kind.trim().is_empty()
                || event.world_time > tail.world_time
                || event.caused_by.iter().any(|cause| *cause >= event.id)
            {
                return refuse(format!("event #{} is not a lawful next event", event.id));
            }
            last = Some((event.id, event.world_time));
        }
        if let Some(pending) = tail
            .pending
            .iter()
            .find(|pending| pending.world_time < tail.world_time)
        {
            return refuse(format!(
                "an action is pending in the past, at {}",
                pending.world_time
            ));
        }
        Ok(())
    }
}

/// What came of trying a change on the session itself.
pub(crate) enum Changed {
    /// Saved and kept; what the World shows now.
    Kept(Box<ProjectionSnapshot>),
    /// It changed nothing that is saved, so it was gone back from.
    Unchanged,
    /// The World cannot go back, so the change was not tried on it.
    CannotGoBack,
}

impl DurableWorldSession {
    /// Makes `act` on the live session, after marking where it stands, and
    /// saves what it recorded: it is handed to the World's writer as the
    /// change is kept, and a change that fails is gone back from, leaving
    /// the World as it was. The file is made and written away from the
    /// turn; a write that fails is tried again before the next change,
    /// which is refused if it fails again. With `skip_unchanged`, a change that
    /// records nothing is gone back from and not saved.
    pub(crate) fn change(
        &mut self,
        act: impl FnOnce(&mut dyn WorldSession) -> Result<ProjectionSnapshot, HostError>,
        skip_unchanged: bool,
        library: &WorldLibrary,
    ) -> Result<Changed, LibraryError> {
        let Some(mark) = self.session.checkpoint()? else {
            return Ok(Changed::CannotGoBack);
        };
        if self.saved.is_none() {
            // Saved once whole, and written on from then on.
            let mut archive = required_archive(self.session.as_ref())?;
            archive.checkpoint = self.checkpoint.clone();
            let history = CompactHistory::of(&archive.events);
            let Some((saved, composer)) = Saved::kept(&archive, history) else {
                return Ok(Changed::CannotGoBack);
            };
            self.writes.start(composer);
            self.saved = Some(saved);
        }
        let own_title_before = self.own_title();
        let snapshot = match act(self.session.as_mut()) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.session.rollback(mark)?;
                return Err(error.into());
            }
        };
        match self.save_change(&snapshot, own_title_before, skip_unchanged, library) {
            Ok(true) => {
                self.own_title
                    .replace(Some(snapshot_display_title(&snapshot)));
                Ok(Changed::Kept(Box::new(snapshot)))
            }
            Ok(false) => {
                self.session.rollback(mark)?;
                Ok(Changed::Unchanged)
            }
            Err(error) => {
                self.session.rollback(mark)?;
                Err(error)
            }
        }
    }

    /// Hands the World's writer what the session recorded since it was
    /// last saved, and takes on what the file will hold; `false` when it recorded
    /// nothing and `skip_unchanged` asks for nothing to be written then.
    fn save_change(
        &mut self,
        snapshot: &ProjectionSnapshot,
        own_title_before: Option<String>,
        skip_unchanged: bool,
        library: &WorldLibrary,
    ) -> Result<bool, LibraryError> {
        let saved = self.saved.as_ref().expect("saved before any change");
        let tail = self
            .session
            .archive_since(saved.count)?
            .ok_or_else(|| LibraryError::ArchiveUnsupported(self.session.pack().id))?;
        saved.followed_by(&tail)?;
        if skip_unchanged
            && tail.events.is_empty()
            && tail.world_time == saved.world_time
            && tail.pending == saved.pending
        {
            return Ok(false);
        }

        // The description as `describe_from_snapshot` writes it; the
        // drawings, the bulk of it, are written again only when the cast is
        // drawn with others than last time, and are otherwise taken over
        // (and given back if the change is not kept).
        let drawn = drawn_in(snapshot);
        let same_drawings = saved.drawn.len() == drawn.len()
            && saved.drawn.iter().zip(&drawn).all(|(was, is)| was == *is);
        let kept_drawings = std::mem::take(&mut self.metadata.display_drawings);
        let mut metadata = self.metadata.clone();
        metadata.display_title = next_display_title_after(
            self.metadata.display_title.as_deref(),
            own_title_before,
            snapshot,
        );
        describe_all_but_drawings(&mut metadata, snapshot);
        let new_drawings = if same_drawings {
            metadata.display_drawings = kept_drawings;
            None
        } else {
            self.metadata.display_drawings = kept_drawings;
            metadata.display_drawings = drawing_values(&drawn);
            Some(drawn.into_iter().cloned().collect::<Vec<_>>())
        };

        // The checkpoint moves on to the start of the latest season, as a
        // document settles its own (see `WorldDocument::settle_checkpoint`).
        let span = world_document::season_span(&metadata);
        let season_start = tail.world_time / span * span;
        let mut season = saved.season.clone();
        season.extend(tail.events.iter().cloned());
        let settled = season.partition_point(|event| event.world_time < season_start);
        let checkpoint = match &self.checkpoint {
            Some(checkpoint) if settled == 0 => Some(checkpoint.clone()),
            previous => {
                let mut checkpoint = previous.clone().unwrap_or_default();
                checkpoint.advance(&season[..settled]);
                Some(checkpoint)
            }
        };

        // Handed to the World's writer, which adds the events to the file's
        // history and writes it away from the turn, in order.
        let (path, legacy) = self.target.paths(library);
        let saved = self.saved.as_mut().expect("saved before any change");
        saved.count += tail.events.len();
        if let Some(event) = tail.events.last() {
            saved.last = Some((event.id, event.world_time));
        }
        if let Some(drawn) = new_drawings {
            saved.drawn = drawn;
        }
        season.drain(..settled);
        saved.season = season;
        saved.world_time = tail.world_time;
        saved.pending = tail.pending.clone();
        let head = crate::writer::Head {
            path,
            legacy,
            metadata: metadata.clone(),
            pack: tail.pack,
            world_time: tail.world_time,
            pending: tail.pending,
            checkpoint: checkpoint.clone(),
        };
        let on_disk = self.revision;
        self.writes.queue(tail.events, head, on_disk);
        self.checkpoint = checkpoint;
        self.metadata = metadata;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DurableWorldSession, WorldDocumentId};
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use world_core::{
        Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft,
        StateChange, Value, World, WorldState,
    };
    use world_document::WorldDocument;
    use world_host::{SessionCheckpoint, WorldDescriptor, WorldRegistration, WorldRegistry};
    use world_projection::ProjectionIntent;

    const PACK: &str = "test.counter";
    const COUNTER: EntityId = EntityId::new(1);

    struct Add;

    impl Action for Add {
        fn name(&self) -> &'static str {
            "add"
        }

        fn evaluate(
            &self,
            state: &WorldState,
            request: &ActionRequest,
        ) -> Result<EventDraft, ActionError> {
            let units = match state.entity(COUNTER).and_then(|e| e.component("units")) {
                Some(Value::Integer(units)) => *units,
                _ => 0,
            };
            let mut draft = EventDraft::new("added");
            draft.targets = vec![COUNTER];
            draft
                .payload
                .insert("note".into(), Value::Text(format!("to {}", units + 1)));
            draft.changes.push(StateChange::SetComponent {
                entity: COUNTER,
                key: "units".into(),
                value: (units + 1).into(),
            });
            let _ = request;
            Ok(draft)
        }
    }

    fn actions() -> ActionRegistry {
        let mut registry = ActionRegistry::new();
        registry.register(Add).unwrap();
        registry
    }

    fn baseline() -> WorldState {
        let mut state = WorldState::default();
        state
            .seed_entity(Entity::new(COUNTER, "counter").with_component("units", 0_i64))
            .unwrap();
        state
    }

    /// A World that counts, which can go back to a checkpoint.
    struct Counter {
        world: World,
    }

    impl Counter {
        fn units(&self) -> i64 {
            match self
                .world
                .state()
                .entity(COUNTER)
                .unwrap()
                .component("units")
            {
                Some(Value::Integer(units)) => *units,
                _ => 0,
            }
        }

        fn add(&mut self) -> Result<(), HostError> {
            self.world
                .execute(&actions(), &ActionRequest::new("add"))
                .map(|_| ())
                .map_err(HostError::session)
        }
    }

    impl WorldSession for Counter {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(PACK, "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: format!("Counter {}", self.units()),
                world_time: self.world.world_time(),
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            match intent {
                ProjectionIntent::InvokeCommand(command) if command == "add" => self.add()?,
                // Half of a change, and then it fails.
                ProjectionIntent::InvokeCommand(command) if command == "half" => {
                    self.add()?;
                    return Err(HostError::session("the second half failed"));
                }
                _ => return Err(HostError::session("no such command")),
            }
            Ok(self.snapshot())
        }

        fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
            for _ in 0..periods {
                let time = self.world.world_time() + 10;
                self.world
                    .advance_to(&actions(), time)
                    .map_err(HostError::session)?;
                self.add()?;
            }
            Ok(self.snapshot())
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            WorldArchive::capture(self.pack(), &self.world)
                .map(Some)
                .map_err(HostError::session)
        }

        fn checkpoint(&mut self) -> Result<Option<SessionCheckpoint>, HostError> {
            Ok(Some(SessionCheckpoint::new(self.world.checkpoint())))
        }

        fn rollback(&mut self, checkpoint: SessionCheckpoint) -> Result<(), HostError> {
            self.world.rollback(checkpoint.into_inner()?);
            Ok(())
        }

        fn archive_since(&self, from: usize) -> Result<Option<WorldArchive>, HostError> {
            WorldArchive::capture_since(self.pack(), &self.world, from)
                .map(Some)
                .map_err(HostError::session)
        }
    }

    /// The counting Pack, and how many times it opened a World.
    fn registry() -> (WorldRegistry, Arc<AtomicUsize>) {
        let opened = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&opened);
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: WorldPackRef::new(PACK, "1"),
                        title: "Counter".into(),
                        description: "Counts".into(),
                    },
                    || {
                        Ok(Box::new(Counter {
                            world: World::new(baseline()),
                        }))
                    },
                )
                .with_archive_opener(move |archive| {
                    counted.fetch_add(1, Ordering::SeqCst);
                    let world = archive
                        .restore(&WorldPackRef::new(PACK, "1"), baseline())
                        .map_err(HostError::session)?;
                    Ok(Box::new(Counter { world }))
                }),
            )
            .unwrap();
        (registry, opened)
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "world-machine-change-{}-{nonce}-{label}",
            std::process::id()
        ))
    }

    /// The document the whole-archive path would write after the change
    /// that took the World from `before` (the file as it was) to `archive`.
    fn as_saved_whole(before: &WorldDocument, archive: WorldArchive) -> WorldDocument {
        let mut document = WorldDocument {
            archive,
            metadata: before.metadata.clone(),
        };
        document.archive.checkpoint = before.archive.checkpoint.clone();
        document.settle_checkpoint();
        // A checkpoint of nothing is not written.
        document.archive.checkpoint = document
            .archive
            .checkpoint
            .filter(|checkpoint| checkpoint.events > 0);
        document
    }

    #[test]
    fn a_change_is_made_on_the_world_itself_saved_and_kept() {
        let root = temp_root("kept");
        let library = WorldLibrary::new(root.clone());
        let (registry, opened) = registry();
        let id = WorldDocumentId::new("counting").unwrap();
        drop(DurableWorldSession::create(id.clone(), PACK, &registry, &library).unwrap());
        let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
        assert_eq!(opened.load(Ordering::SeqCst), 1);

        for step in 1..=40_u64 {
            let before = library.load_document(&id).unwrap().unwrap();
            let snapshot = if step % 3 == 0 {
                session.advance_background(2, &registry, &library).unwrap()
            } else {
                session
                    .handle(
                        ProjectionIntent::InvokeCommand("add".into()),
                        &registry,
                        &library,
                    )
                    .unwrap()
            };
            session.flush().unwrap();
            let file = library.load_document(&id).unwrap().unwrap();
            let live = session.current_archive().unwrap();
            assert_eq!(snapshot.world_time, live.world_time);
            // What is on disk is what the whole archive would have written,
            // the checkpoint moving on season by season.
            let whole = as_saved_whole(&before, live.clone());
            assert_eq!(file.archive, whole.archive, "step {step}");
            assert_eq!(file.metadata.display_title, Some(snapshot.title.clone()));
            // And it replays to the World that is open.
            let reopened = registry.open_archive(&file.archive).unwrap();
            assert_eq!(reopened.snapshot(), session.snapshot());
            assert_eq!(reopened.archive().unwrap().unwrap(), live);
        }
        let file = library.load_document(&id).unwrap().unwrap();
        assert!(file.archive.checkpoint.as_ref().unwrap().events > 0);
        // Opened once, and once more for each look at the file above.
        assert_eq!(opened.load(Ordering::SeqCst), 1 + 40);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_change_whose_file_cannot_be_written_is_kept_and_written_when_it_can() {
        let root = temp_root("unsaved");
        let library = WorldLibrary::new(root.clone());
        let (registry, _) = registry();
        let id = WorldDocumentId::new("counting").unwrap();
        drop(DurableWorldSession::create(id.clone(), PACK, &registry, &library).unwrap());
        let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
        let add = || ProjectionIntent::InvokeCommand("add".into());
        session.handle(add(), &registry, &library).unwrap();
        session.flush().unwrap();
        let file_before = fs::read(library.path(&id)).unwrap();

        // The file cannot be written, as with a full disk, for any user:
        // the turn is kept, and its write waits to be tried again.
        session.writes.fail(true);
        let kept = session.handle(add(), &registry, &library).unwrap();
        assert_eq!(kept.title, "Counter 2");
        assert!(matches!(session.flush(), Err(LibraryError::Io(_))));
        assert_eq!(fs::read(library.path(&id)).unwrap(), file_before);
        let archive_before = session.current_archive().unwrap();
        let snapshot_before = session.snapshot();

        // Nothing more is played while it cannot be written.
        let refused = session.handle(add(), &registry, &library);
        let refused_background = session.advance_background(3, &registry, &library);
        assert!(matches!(refused, Err(LibraryError::Io(_))));
        assert!(matches!(refused_background, Err(LibraryError::Io(_))));
        assert_eq!(session.current_archive().unwrap(), archive_before);
        assert_eq!(session.snapshot(), snapshot_before);
        assert_eq!(fs::read(library.path(&id)).unwrap(), file_before);

        // With the disk back, the next turn writes both.
        session.writes.fail(false);
        let next = session.handle(add(), &registry, &library).unwrap();
        assert_eq!(next.title, "Counter 3");
        session.flush().unwrap();
        let file = library.load_document(&id).unwrap().unwrap();
        let mut expected = session.current_archive().unwrap();
        expected.checkpoint = file.archive.checkpoint.clone();
        assert_eq!(file.archive, expected);

        // A change that fails half-way is gone back from, and not saved.
        let file_before = fs::read(library.path(&id)).unwrap();
        let archive_before = session.current_archive().unwrap();
        let half = session.handle(
            ProjectionIntent::InvokeCommand("half".into()),
            &registry,
            &library,
        );
        assert!(half.is_err());
        session.flush().unwrap();
        assert_eq!(session.current_archive().unwrap(), archive_before);
        assert_eq!(fs::read(library.path(&id)).unwrap(), file_before);
        let _ = fs::remove_dir_all(root);
    }

    /// Turns come faster than their files are written: each is written in
    /// order, the latest last, the file always whole with the save before it
    /// kept beside it, and everything written by the time the World is let
    /// go of.
    #[test]
    fn quick_turns_are_written_in_order_and_all_by_the_time_the_world_is_closed() {
        let root = temp_root("quick");
        let library = WorldLibrary::new(root.clone());
        let (registry, _) = registry();
        let id = WorldDocumentId::new("counting").unwrap();
        drop(DurableWorldSession::create(id.clone(), PACK, &registry, &library).unwrap());
        let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
        for step in 1..=60_i64 {
            let snapshot = session
                .handle(
                    ProjectionIntent::InvokeCommand("add".into()),
                    &registry,
                    &library,
                )
                .unwrap();
            assert_eq!(snapshot.title, format!("Counter {step}"));
            // Whatever moment they are read at, the backup and then the
            // file are whole Worlds, in order, no further on than the one
            // open.
            let backup = crate::backup_path(&library.path(&id));
            let kept = fs::read(&backup)
                .ok()
                .map(|bytes| WorldDocument::from_bytes(&bytes).unwrap());
            let file = library.load_document(&id).unwrap().unwrap();
            assert!(file.archive.events.len() <= step as usize);
            if let Some(kept) = kept {
                assert!(kept.archive.events.len() <= file.archive.events.len());
            }
        }
        let live = session.current_archive().unwrap();
        drop(session);
        let file = library.load_document(&id).unwrap().unwrap();
        assert_eq!(file.archive.events, live.events);
        assert_eq!(file.metadata.display_title.as_deref(), Some("Counter 60"));
        let reopened = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
        assert_eq!(reopened.snapshot().title, "Counter 60");
        drop(reopened);
        let _ = fs::remove_dir_all(root);
    }

    /// Someone else writes the file while a World is open: the next change
    /// is refused rather than written over it, whether the World's writer
    /// was resting or had a write waiting, and nothing it wrote is lost.
    #[test]
    fn a_file_someone_else_wrote_is_never_written_over() {
        let root = temp_root("theirs");
        let library = WorldLibrary::new(root.clone());
        let (registry, _) = registry();
        let id = WorldDocumentId::new("counting").unwrap();
        drop(DurableWorldSession::create(id.clone(), PACK, &registry, &library).unwrap());
        let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
        let add = || ProjectionIntent::InvokeCommand("add".into());
        session.handle(add(), &registry, &library).unwrap();
        session.flush().unwrap();
        let theirs = b"someone else's file".to_vec();
        fs::write(library.path(&id), &theirs).unwrap();
        assert!(matches!(
            session.handle(add(), &registry, &library),
            Err(LibraryError::DocumentChanged(_))
        ));
        assert_eq!(fs::read(library.path(&id)).unwrap(), theirs);

        drop(session);

        // A write waiting (here, one that failed) when it happens is
        // refused by the writer, and the change after it hears.
        let second = WorldDocumentId::new("second").unwrap();
        drop(DurableWorldSession::create(second.clone(), PACK, &registry, &library).unwrap());
        let mut session = DurableWorldSession::open(second.clone(), &registry, &library).unwrap();
        session.writes.fail(true);
        session.handle(add(), &registry, &library).unwrap();
        assert!(matches!(session.flush(), Err(LibraryError::Io(_))));
        let theirs = fs::read(library.path(&second)).unwrap();
        let mut changed = theirs.clone();
        changed.extend_from_slice(b" ");
        fs::write(library.path(&second), &changed).unwrap();
        session.writes.fail(false);
        assert!(matches!(
            session.handle(add(), &registry, &library),
            Err(LibraryError::DocumentChanged(_))
        ));
        assert!(matches!(
            session.flush(),
            Err(LibraryError::DocumentChanged(_))
        ));
        assert_eq!(fs::read(library.path(&second)).unwrap(), changed);
        // Read again from its file, the World carries on from there.
        drop(session);
        fs::write(library.path(&second), &theirs).unwrap();
        let mut session = DurableWorldSession::open(second.clone(), &registry, &library).unwrap();
        session.handle(add(), &registry, &library).unwrap();
        session.flush().unwrap();
        let _ = fs::remove_dir_all(root);
    }
}
