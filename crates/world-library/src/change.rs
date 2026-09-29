//! A change made to a durable World on the session that holds it: marked
//! first, saved, and kept, or gone back from if it cannot be saved, so a
//! change costs what it does and what it adds to the file, not the World
//! opened again.

use world_document::{DeflatedHistory, WorldDocument};
use world_host::{HostError, WorldSession};
use world_persistence::{
    ArchiveHead, ArchivedEvent, ArchivedScheduledAction, CheckpointFit, CompactHistory,
    WorldArchive, WorldPackRef,
};
use world_projection::ProjectionSnapshot;

use crate::{
    describe_from_snapshot, next_display_title_after, required_archive, snapshot_display_title,
    DurableWorldSession, LibraryError, WorldLibrary,
};

/// What the World file holds, kept so a change writes out only what the
/// World recorded since it was last saved.
#[derive(Debug)]
pub(crate) struct Saved {
    /// Every event saved, as the file writes them.
    history: CompactHistory,
    /// The same, deflated as the file packs them, piece by piece.
    deflated: DeflatedHistory,
    /// The history as it was when it was first kept, being deflated whole
    /// away from the World being shown.
    deflating: Option<std::thread::JoinHandle<Option<DeflatedHistory>>>,
    /// The events saved after the file's checkpoint, which the checkpoint
    /// sums up as their season passes.
    season: Vec<ArchivedEvent>,
    pack: WorldPackRef,
    world_time: u64,
    pending: Vec<ArchivedScheduledAction>,
}

impl Saved {
    /// What a file holding `archive` keeps, its events written as `history`,
    /// when its checkpoint (if any) sums up the first of those events: the
    /// checkpoint is then carried on from season to season as the file's
    /// own is.
    pub(crate) fn kept(archive: &WorldArchive, history: CompactHistory) -> Option<Self> {
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
        let text = history.written_text().to_vec();
        let deflating = std::thread::Builder::new()
            .name("world-machine-deflate".into())
            .spawn(move || DeflatedHistory::of(&text).ok())
            .ok();
        Some(Self {
            history,
            deflated: DeflatedHistory::default(),
            deflating,
            season: archive.events[settled..].to_vec(),
            pack: archive.pack.clone(),
            world_time: archive.world_time,
            pending: archive.pending.clone(),
        })
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
        let mut last = self.history.last_event();
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
    /// saves what it recorded: the file is written before the change is
    /// kept, and a change that fails, or cannot be saved, is gone back from,
    /// leaving the World as it was. With `skip_unchanged`, a change that
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
            self.saved = Saved::kept(&archive, history);
            if self.saved.is_none() {
                return Ok(Changed::CannotGoBack);
            }
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

    /// Writes the World file with what the session recorded since it was
    /// last saved, and takes on what was written; `false` when it recorded
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
            .archive_since(saved.history.len())?
            .ok_or_else(|| LibraryError::ArchiveUnsupported(self.session.pack().id))?;
        saved.followed_by(&tail)?;
        if skip_unchanged
            && tail.events.is_empty()
            && tail.world_time == saved.world_time
            && tail.pending == saved.pending
        {
            return Ok(false);
        }

        let mut metadata = self.metadata.clone();
        metadata.display_title = next_display_title_after(
            self.metadata.display_title.as_deref(),
            own_title_before,
            snapshot,
        );
        describe_from_snapshot(&mut metadata, snapshot);

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

        let saved = self.saved.as_mut().expect("saved before any change");
        if let Some(deflating) = saved.deflating.take() {
            if let Ok(Some(deflated)) = deflating.join() {
                saved.deflated = deflated;
            }
        }
        let mark = saved.history.mark();
        let text_before = saved.history.written_text().len();
        saved.history.push(&tail.events);
        let written = WorldDocument::file_from_deflated_history(
            &metadata,
            ArchiveHead {
                pack: &tail.pack,
                world_time: tail.world_time,
                pending: &tail.pending,
            },
            &saved.history,
            &mut saved.deflated,
            checkpoint.as_ref(),
        )
        .map_err(LibraryError::from)
        .and_then(|bytes| {
            self.target.verify_revision(self.revision, library)?;
            self.target.persist_bytes(&bytes, library)
        });
        let revision = match written {
            Ok(revision) => revision,
            Err(error) => {
                saved.history.rewind(mark);
                saved.deflated.go_back_to(text_before);
                return Err(error);
            }
        };

        season.drain(..settled);
        saved.season = season;
        saved.world_time = tail.world_time;
        saved.pending = tail.pending;
        self.checkpoint = checkpoint;
        self.metadata = metadata;
        self.revision = revision;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DurableWorldSession, WorldDocumentId, WRITES_FAIL};
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use world_core::{
        Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, EventDraft,
        StateChange, Value, World, WorldState,
    };
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
    fn a_change_that_cannot_be_saved_leaves_the_world_as_it_was() {
        let root = temp_root("unsaved");
        let library = WorldLibrary::new(root.clone());
        let (registry, _) = registry();
        let id = WorldDocumentId::new("counting").unwrap();
        drop(DurableWorldSession::create(id.clone(), PACK, &registry, &library).unwrap());
        let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
        let add = || ProjectionIntent::InvokeCommand("add".into());
        session.handle(add(), &registry, &library).unwrap();
        let file_before = fs::read(library.path(&id)).unwrap();
        let archive_before = session.current_archive().unwrap();
        let snapshot_before = session.snapshot();

        // The file cannot be written, as with a full disk, for any user.
        WRITES_FAIL.set(true);
        let failed = session.handle(add(), &registry, &library);
        let failed_background = session.advance_background(3, &registry, &library);
        WRITES_FAIL.set(false);
        assert!(matches!(failed, Err(LibraryError::Io(_))));
        assert!(matches!(failed_background, Err(LibraryError::Io(_))));
        assert_eq!(session.current_archive().unwrap(), archive_before);
        assert_eq!(session.snapshot(), snapshot_before);
        assert_eq!(fs::read(library.path(&id)).unwrap(), file_before);

        // A change that fails half-way is gone back from too.
        let half = session.handle(
            ProjectionIntent::InvokeCommand("half".into()),
            &registry,
            &library,
        );
        assert!(half.is_err());
        assert_eq!(session.current_archive().unwrap(), archive_before);
        assert_eq!(fs::read(library.path(&id)).unwrap(), file_before);

        // And the World carries on from where it was.
        let next = session.handle(add(), &registry, &library).unwrap();
        assert_eq!(next.title, "Counter 2");
        let file = library.load_document(&id).unwrap().unwrap();
        let mut expected = session.current_archive().unwrap();
        expected.checkpoint = file.archive.checkpoint.clone();
        assert_eq!(file.archive, expected);
        let _ = fs::remove_dir_all(root);
    }
}
