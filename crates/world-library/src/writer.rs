//! A World's file written away from the turn that changed it.
//!
//! Making a long World's file (its history deflated, its description and
//! checkpoint written) and writing it safely (`fsync`, the backup kept, the
//! rename) takes 15 to 30 ms on a quiet machine and seconds on a busy disk,
//! which a turn should not wait for. A turn hands over what it recorded and
//! the file's head as it now stands; one thread per open World keeps the
//! World's history as the file writes it, adds each change to it in order,
//! makes the file from the latest head (several changes that came while it
//! was busy make one file) and writes it as before: to a temporary file,
//! synced, the file as it was kept as `.bak`, and renamed into place. A file
//! is therefore always whole on disk: the save before, or the save itself.
//!
//! Before it writes, the thread checks that the file still holds what it last
//! wrote (or what the World was opened from), so a file someone else changed
//! is never written over; that is reported to the next change, as a failed
//! write is. A failed write is kept and tried again when the World is next
//! changed or flushed. [`Writes::flush`] waits for every change handed over;
//! the World's session flushes when it is dropped, and the app flushes every
//! open World as it quits ([`flush_all`]).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::thread::JoinHandle;
use std::time::SystemTime;

use world_document::{DeflatedHistory, WorldDocument, WorldDocumentMetadata};
use world_persistence::{
    ArchiveHead, ArchivedCheckpoint, ArchivedEvent, ArchivedScheduledAction, CompactHistory,
    WorldPackRef,
};

use crate::revision::DocumentRevision;
use crate::LibraryError;

/// A World's history as its file writes it, kept by its writer.
pub(crate) struct Composer {
    /// Every event saved, as the file writes them.
    history: CompactHistory,
    /// The same, deflated as the file packs them, piece by piece.
    deflated: DeflatedHistory,
    /// The history as it was when it was first kept, being deflated whole.
    deflating: Option<JoinHandle<Option<DeflatedHistory>>>,
}

impl Composer {
    /// A writer's history from `history`, the events the file holds,
    /// deflated whole on a thread of its own from now.
    pub(crate) fn new(history: CompactHistory) -> Self {
        let text = history.written_text().to_vec();
        let deflating = std::thread::Builder::new()
            .name("world-machine-deflate".into())
            .spawn(move || DeflatedHistory::of(&text).ok())
            .ok();
        Self {
            history,
            deflated: DeflatedHistory::default(),
            deflating,
        }
    }

    /// The file with `events` added to the history, headed by `head`.
    fn file(
        &mut self,
        events: &[Vec<ArchivedEvent>],
        head: &Head,
    ) -> Result<Vec<u8>, LibraryError> {
        for events in events {
            self.history.push(events);
        }
        if let Some(deflating) = self.deflating.take() {
            if let Ok(Some(deflated)) = deflating.join() {
                self.deflated = deflated;
            }
        }
        Ok(WorldDocument::file_from_deflated_history(
            &head.metadata,
            ArchiveHead {
                pack: &head.pack,
                world_time: head.world_time,
                pending: &head.pending,
            },
            &self.history,
            &mut self.deflated,
            head.checkpoint.as_ref(),
        )?)
    }
}

/// The file's head as a change leaves it: everything but the events.
pub(crate) struct Head {
    /// Where the file is written.
    pub path: PathBuf,
    /// An older name of the same file, read if `path` is not there and
    /// removed once `path` is written.
    pub legacy: Option<PathBuf>,
    pub metadata: WorldDocumentMetadata,
    pub pack: WorldPackRef,
    pub world_time: u64,
    pub pending: Vec<ArchivedScheduledAction>,
    pub checkpoint: Option<ArchivedCheckpoint>,
}

/// Why a write handed over did not happen.
#[derive(Clone, Debug)]
enum Failure {
    /// The file on disk is not what was last written: someone else wrote it.
    Changed(PathBuf),
    Io(io::ErrorKind, String),
}

impl Failure {
    fn error(&self) -> LibraryError {
        match self {
            Failure::Changed(path) => LibraryError::DocumentChanged(path.clone()),
            Failure::Io(kind, message) => LibraryError::Io(io::Error::new(*kind, message.clone())),
        }
    }

    fn of(error: LibraryError) -> Self {
        match error {
            LibraryError::Io(error) => Failure::Io(error.kind(), error.to_string()),
            LibraryError::DocumentChanged(path) => Failure::Changed(path),
            other => Failure::Io(io::ErrorKind::Other, other.to_string()),
        }
    }
}

/// How a file stood on disk: its length and when it was last written.
type Stat = (u64, Option<SystemTime>);

fn stat(path: &Path) -> Option<Stat> {
    let metadata = std::fs::metadata(path).ok()?;
    Some((metadata.len(), metadata.modified().ok()))
}

#[derive(Default)]
struct State {
    /// The history the file is made from; away while a file is being made.
    composer: Option<Composer>,
    /// The events of each change not yet in the history, in order.
    todo: Vec<Vec<ArchivedEvent>>,
    /// The latest change's head; away while a file is being made.
    head: Option<Head>,
    /// Whether a change handed over is not yet on disk.
    dirty: bool,
    /// How many changes have been handed over, to tell whether one came
    /// while a file was being made.
    handed: u64,
    /// Whether a file is being made or written.
    busy: bool,
    /// What the file holds, as far as this World knows: what it last wrote,
    /// or what it was opened from. `None` until the first change.
    on_disk: Option<DocumentRevision>,
    /// How the file stood on disk just after it was last written here.
    written: Option<(PathBuf, Stat)>,
    /// Why the latest write failed; the writer waits until it is asked to
    /// try again.
    failed: Option<Failure>,
    /// The World is being let go of: write what is left, then stop.
    closing: bool,
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    /// Makes every write fail, as a full disk would (tests only).
    #[cfg(test)]
    fail: std::sync::atomic::AtomicBool,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn wait<'a>(&self, state: MutexGuard<'a, State>) -> MutexGuard<'a, State> {
        self.changed
            .wait(state)
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// How many World files have been written away from their turns, so the
/// app can tell when to look at the library again.
static WRITTEN: AtomicU64 = AtomicU64::new(0);

/// Every World writing its file now, so the app can flush them all as it
/// quits.
static OPEN: Mutex<Vec<Weak<Shared>>> = Mutex::new(Vec::new());

/// How many World files have been written away from their turns since the
/// app started.
pub fn files_written() -> u64 {
    WRITTEN.load(Ordering::Acquire)
}

/// Waits until every open World's file holds every change handed over for
/// it, trying once more any write that failed; the app calls this as it
/// quits. Returns what could not be written.
pub fn flush_all() -> Vec<LibraryError> {
    let open = OPEN
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .filter_map(Weak::upgrade)
        .collect::<Vec<_>>();
    open.iter()
        .filter_map(|shared| flush(shared).err())
        .collect()
}

/// The writes of one open World.
#[derive(Default)]
pub(crate) struct Writes {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Writes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Writes")
    }
}

impl Writes {
    /// Takes `composer` as the history the file is made from from now on,
    /// once any write under way is done.
    pub(crate) fn start(&self, composer: Composer) {
        let mut state = self.shared.lock();
        while state.busy {
            state = self.shared.wait(state);
        }
        state.composer = Some(composer);
        state.todo.clear();
        state.head = None;
        state.dirty = false;
    }

    /// Hands over a change: the `events` it recorded and the file's `head`
    /// after it. `on_disk` is what the file holds if nothing was handed over
    /// before.
    pub(crate) fn queue(
        &mut self,
        events: Vec<ArchivedEvent>,
        head: Head,
        on_disk: DocumentRevision,
    ) {
        {
            let mut state = self.shared.lock();
            state.on_disk.get_or_insert(on_disk);
            state.todo.push(events);
            state.head = Some(head);
            state.dirty = true;
            state.handed += 1;
        }
        self.shared.changed.notify_all();
        if self.thread.is_none() {
            let shared = Arc::clone(&self.shared);
            match std::thread::Builder::new()
                .name("world-machine-save".into())
                .spawn(move || write_in_turn(&shared))
            {
                Ok(thread) => {
                    self.thread = Some(thread);
                    let mut open = OPEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    open.retain(|open| open.strong_count() > 0);
                    open.push(Arc::downgrade(&self.shared));
                }
                // With no thread to write it, it is written here.
                Err(_) => write_now(&self.shared),
            }
        }
    }

    /// Whether nothing is being written or waits to be.
    pub(crate) fn idle(&self) -> bool {
        let state = self.shared.lock();
        !state.busy && !state.dirty
    }

    /// Whether a write handed over has failed and not yet been tried again.
    pub(crate) fn failed(&self) -> bool {
        self.shared.lock().failed.is_some()
    }

    /// What the file holds, if this World has written it or been told.
    pub(crate) fn on_disk(&self) -> Option<DocumentRevision> {
        self.shared.lock().on_disk
    }

    /// Whether the file at `path` stands on disk as it did just after this
    /// World last wrote it (its length and time): `None` if this World has
    /// not written it.
    pub(crate) fn untouched(&self, path: &Path) -> Option<bool> {
        let written = self.shared.lock().written.clone();
        let (at, stood) = written?;
        (at == path).then(|| stat(path) == Some(stood))
    }

    /// Makes every write fail from now on, as a full disk would, or
    /// succeed again.
    #[cfg(test)]
    pub(crate) fn fail(&self, fail: bool) {
        self.shared.fail.store(fail, Ordering::SeqCst);
    }

    /// Waits until the file holds every change handed over, trying once
    /// more a write that failed.
    pub(crate) fn flush(&self) -> Result<(), LibraryError> {
        if self.thread.is_some() {
            return flush(&self.shared);
        }
        // With no thread to write, a change was written where it was
        // handed over, or failed there: it is tried again here.
        let retry = {
            let mut state = self.shared.lock();
            state.failed.take();
            state.dirty
        };
        if retry {
            write_now(&self.shared);
        }
        outcome(&self.shared.lock())
    }

    /// Forgets anything not yet written, and takes the file to hold
    /// `revision` now: after the World was read again from its file, or
    /// written whole. Waits for a write under way first. The next change
    /// starts the history again ([`Self::start`]).
    pub(crate) fn start_from(&self, revision: DocumentRevision) {
        let mut state = self.shared.lock();
        while state.busy {
            state = self.shared.wait(state);
        }
        state.composer = None;
        state.todo.clear();
        state.head = None;
        state.dirty = false;
        state.failed = None;
        state.written = None;
        state.on_disk = Some(revision);
    }
}

impl Drop for Writes {
    fn drop(&mut self) {
        self.shared.lock().closing = true;
        self.shared.changed.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// [`Writes::flush`] on what a World shares with its writer thread.
fn flush(shared: &Shared) -> Result<(), LibraryError> {
    let mut state = shared.lock();
    if state.failed.take().is_some() {
        // Tried again, once.
        shared.changed.notify_all();
    }
    while !state.closing && (state.busy || (state.dirty && state.failed.is_none())) {
        state = shared.wait(state);
    }
    outcome(&state)
}

/// Whether the file holds every change handed over, and if not, why.
fn outcome(state: &State) -> Result<(), LibraryError> {
    match &state.failed {
        Some(failure) => Err(failure.error()),
        None if state.dirty => Err(LibraryError::Io(io::Error::other(
            "the World's file is still to be written",
        ))),
        None => Ok(()),
    }
}

/// The writer's thread: each change in turn until the World is let go of.
fn write_in_turn(shared: &Shared) {
    loop {
        let mut state = shared.lock();
        while (!state.dirty || state.failed.is_some()) && !state.closing {
            state = shared.wait(state);
        }
        if !state.dirty || state.failed.is_some() {
            // Closing, with nothing left that can be written.
            return;
        }
        drop(state);
        write_now(shared);
    }
}

/// Makes the file from the changes handed over and writes it, and records
/// how that went.
fn write_now(shared: &Shared) {
    let (mut composer, todo, head, expected, handed) = {
        let mut state = shared.lock();
        if !state.dirty {
            return;
        }
        let (Some(composer), Some(head)) = (state.composer.take(), state.head.take()) else {
            state.failed = Some(Failure::Io(
                io::ErrorKind::Other,
                "the World's history to write from is missing".into(),
            ));
            drop(state);
            shared.changed.notify_all();
            return;
        };
        state.busy = true;
        (
            composer,
            std::mem::take(&mut state.todo),
            head,
            state.on_disk,
            state.handed,
        )
    };
    let outcome = composer
        .file(&todo, &head)
        .map_err(Failure::of)
        .and_then(|bytes| write_file(shared, &head, &bytes, expected).map(|()| bytes));
    let mut state = shared.lock();
    state.busy = false;
    state.composer = Some(composer);
    // A head handed over meanwhile is newer than this one.
    let path = head.path.clone();
    if state.head.is_none() {
        state.head = Some(head);
    }
    match outcome {
        Ok(bytes) => {
            state.on_disk = Some(DocumentRevision::from_bytes(&bytes));
            state.written = stat(&path).map(|stood| (path, stood));
            state.dirty = state.handed != handed;
            WRITTEN.fetch_add(1, Ordering::AcqRel);
        }
        Err(failure) => state.failed = Some(failure),
    }
    drop(state);
    shared.changed.notify_all();
}

/// Checks that the file holds `expected`, then writes `bytes` to it.
fn write_file(
    shared: &Shared,
    head: &Head,
    bytes: &[u8],
    expected: Option<DocumentRevision>,
) -> Result<(), Failure> {
    let io = |error: io::Error| Failure::Io(error.kind(), error.to_string());
    #[cfg(test)]
    if shared.fail.load(Ordering::SeqCst) {
        return Err(Failure::Io(
            io::ErrorKind::Other,
            "the World file could not be written".into(),
        ));
    }
    #[cfg(not(test))]
    let _ = shared;
    let current = crate::revision_at(&head.path, head.legacy.as_deref()).map_err(Failure::of)?;
    if current != expected {
        return Err(Failure::Changed(head.path.clone()));
    }
    crate::atomic_write(&head.path, bytes).map_err(io)?;
    if let Some(legacy) = &head.legacy {
        let _ = std::fs::remove_file(legacy);
    }
    Ok(())
}
