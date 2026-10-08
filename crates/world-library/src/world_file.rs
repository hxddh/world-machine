//! A World's file, and the one thing that writes it.
//!
//! Everything that says what a World's file holds and how it is written is
//! owned here, together: where it is ([`WorldDocumentTarget`]), what it held
//! when last read or written whole, who wrote it, what of it is kept for
//! writing changes ([`Saved`]), the writer thread that writes them, and the
//! lock that makes this the file's only writer. They change together or not
//! at all: the file is retargeted (Save As), read again (Reload) or written
//! whole only through the methods here, each of which starts the writer and
//! the kept history afresh as one step. That is what v0.26.0's Save As got
//! wrong (the writer restarted and the kept history did not), and it can no
//! longer be written.

use world_document::{FileWriter, WorldDocument};
use world_persistence::ArchivedEvent;

use crate::change::Saved;
use crate::lock::Lock;
use crate::revision::DocumentRevision;
use crate::writer::{Composer, Head, Writes};
use crate::{LibraryError, WorldDocumentTarget, WorldLibrary};

pub(crate) struct WorldFile {
    target: WorldDocumentTarget,
    /// What the file held when this World last read it or wrote it whole.
    revision: DocumentRevision,
    /// Who wrote the file the World was read from.
    writer: FileWriter,
    /// What the file holds, kept so a change writes only what is new;
    /// `None` until the first change after the file was (re)started.
    saved: Option<Saved>,
    /// The file's writer thread.
    writes: Writes,
    /// This World's hold on its file (see [`crate::lock`]).
    lock: Lock,
}

impl WorldFile {
    /// A file just read or written whole, with nothing kept for changes yet.
    pub(crate) fn new(
        target: WorldDocumentTarget,
        revision: DocumentRevision,
        writer: FileWriter,
        lock: Lock,
    ) -> Self {
        Self {
            target,
            revision,
            writer,
            saved: None,
            writes: Writes::default(),
            lock,
        }
    }

    /// A file just read, whose history is kept for changes from the start.
    pub(crate) fn kept(
        target: WorldDocumentTarget,
        revision: DocumentRevision,
        writer: FileWriter,
        lock: Lock,
        kept: Option<(Saved, Composer)>,
    ) -> Self {
        let mut file = Self::new(target, revision, writer, lock);
        if let Some((saved, composer)) = kept {
            file.keep(saved, composer);
        }
        file
    }

    pub(crate) fn target(&self) -> &WorldDocumentTarget {
        &self.target
    }

    pub(crate) fn writer(&self) -> &FileWriter {
        &self.writer
    }

    /// What is kept of the file for changes, if anything is yet.
    pub(crate) fn saved(&self) -> Option<&Saved> {
        self.saved.as_ref()
    }

    pub(crate) fn saved_mut(&mut self) -> Option<&mut Saved> {
        self.saved.as_mut()
    }

    /// Keeps `saved` for changes, and hands its history to the writer.
    pub(crate) fn keep(&mut self, saved: Saved, composer: Composer) {
        self.writes.start(composer);
        self.saved = Some(saved);
    }

    /// Hands a change to the writer: the `events` it recorded and the
    /// file's `head` after it.
    pub(crate) fn queue(&mut self, events: Vec<ArchivedEvent>, head: Head) {
        self.writes.queue(events, head, self.revision);
    }

    /// Where the file is written, and an older name of it.
    pub(crate) fn paths(
        &self,
        library: &WorldLibrary,
    ) -> (std::path::PathBuf, Option<std::path::PathBuf>) {
        self.target.paths(library)
    }

    /// Waits until the file holds every change handed over, trying once
    /// more a write that failed.
    pub(crate) fn flush(&self) -> Result<(), LibraryError> {
        self.writes.flush()
    }

    /// What the file holds, as far as this World knows.
    pub(crate) fn expected_revision(&self) -> DocumentRevision {
        self.writes.on_disk().unwrap_or(self.revision)
    }

    /// Checks that the file still holds what this World last read or wrote.
    pub(crate) fn verify(&self, library: &WorldLibrary) -> Result<(), LibraryError> {
        self.target
            .verify_revision(self.expected_revision(), library)
    }

    /// Before a change: a write that failed is tried again, and the change
    /// refused if it fails again. With nothing waiting to be written, the
    /// file is checked to hold what was last written, so a World whose file
    /// someone else wrote is not played on; while a write waits, its writer
    /// checks that before it writes, and the next change hears of it.
    pub(crate) fn ready_to_change(&self, library: &WorldLibrary) -> Result<(), LibraryError> {
        if self.writes.failed() {
            return self.writes.flush();
        }
        if self.writes.idle() {
            // A look at the file's length and time does, if this World
            // wrote it last; otherwise, or if they differ, its bytes.
            let (path, _) = self.target.paths(library);
            if self.writes.untouched(&path) != Some(true) {
                self.verify(library)?;
            }
        } else {
            // While a write waits, its writer checks the file before it
            // writes and the change after hears of it; but a folder that
            // is gone (removed, or replaced by a file) can never take this
            // change, so it is refused now, before the World moves on,
            // rather than kept and then never saved.
            let (path, _) = self.target.paths(library);
            if let Some(folder) = path
                .parent()
                .filter(|folder| !folder.as_os_str().is_empty())
            {
                if !folder.is_dir() {
                    return Err(LibraryError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("the World's folder is gone: {}", folder.display()),
                    )));
                }
            }
        }
        Ok(())
    }

    /// Writes `document` whole over the file: everything handed over is
    /// written first, the file checked to be this World's, and the writer
    /// and the kept history then start again from what this wrote.
    pub(crate) fn write_whole(
        &mut self,
        document: &WorldDocument,
        library: &WorldLibrary,
    ) -> Result<DocumentRevision, LibraryError> {
        self.writes.flush()?;
        self.verify(library)?;
        let revision = self.target.persist(document, library)?;
        self.restart(revision);
        Ok(revision)
    }

    /// The file as read again from disk (`revision`, written by `writer`):
    /// whatever was not yet written is let go of.
    pub(crate) fn read_again(&mut self, revision: DocumentRevision, writer: FileWriter) {
        self.restart(revision);
        self.writer = writer;
    }

    /// Moves the World to a file just written whole at `target` (Save As),
    /// held by `lock`: what was handed over for the old file is written if
    /// it can be, and the World goes on in the new one either way.
    pub(crate) fn retarget(
        &mut self,
        target: WorldDocumentTarget,
        revision: DocumentRevision,
        lock: Lock,
    ) {
        let _ = self.writes.flush();
        self.restart(revision);
        self.target = target;
        self.writer = FileWriter::current();
        // The old file's lock is let go of only now, once nothing more is
        // written to it.
        self.lock = lock;
    }

    /// The writer and the kept history both start again from `revision`:
    /// the only way either is restarted, so they cannot drift apart.
    fn restart(&mut self, revision: DocumentRevision) {
        self.writes.start_from(revision);
        self.saved = None;
        self.revision = revision;
    }

    /// Makes every write fail from now on, as a full disk would, or
    /// succeed again.
    #[cfg(test)]
    pub(crate) fn fail_writes(&self, fail: bool) {
        self.writes.fail(fail);
    }

    /// What the file held when this World last read or wrote it whole.
    #[cfg(test)]
    pub(crate) fn revision(&self) -> DocumentRevision {
        self.revision
    }

    /// Whether this World holds the lock on its file.
    #[cfg(test)]
    pub(crate) fn locked(&self) -> bool {
        self.lock.is_held()
    }
}

impl Drop for WorldFile {
    fn drop(&mut self) {
        // Everything handed over is written before the World is let go of,
        // and only then is its lock let go of.
        let _ = self.writes.flush();
    }
}
