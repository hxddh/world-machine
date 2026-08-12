use crate::{summary, LibraryError, WorldDocumentId, WorldDocumentSummary, WorldLibrary};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use world_document::WorldDocument;
use world_persistence::WorldArchive;

static CREATE_TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

impl WorldLibrary {
    /// Materialize a durable archive as a new Library World without overwriting
    /// an existing document.
    pub fn create_from_archive(
        &self,
        id: WorldDocumentId,
        archive: &WorldArchive,
    ) -> Result<WorldDocumentSummary, LibraryError> {
        self.create_from_document(id, &WorldDocument::new(archive.clone()))
    }

    /// Materialize a complete World document, including document metadata, as
    /// a new Library World without overwriting an existing document.
    ///
    /// Publication is filesystem-atomic: concurrent creators may prepare their
    /// own temporary files, but exactly one can hard-link into the final World
    /// path. The losers observe `DocumentAlreadyExists` and never replace the
    /// winning document.
    pub fn create_from_document(
        &self,
        id: WorldDocumentId,
        document: &WorldDocument,
    ) -> Result<WorldDocumentSummary, LibraryError> {
        if self.legacy_path(&id).try_exists()? {
            return Err(LibraryError::DocumentAlreadyExists(id));
        }

        match create_document_file(&self.path(&id), document) {
            Ok(()) => Ok(summary(id, &document.archive)),
            Err(LibraryError::Io(error)) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(LibraryError::DocumentAlreadyExists(id))
            }
            Err(error) => Err(error),
        }
    }
}

fn create_document_file(path: &Path, document: &WorldDocument) -> Result<(), LibraryError> {
    let json = document.to_json_pretty()?;
    atomic_create(path, json.as_bytes())?;
    Ok(())
}

fn atomic_create(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::other("world document path has no file name"))?
        .to_string_lossy();
    let (mut temp_file, temp_path) = loop {
        let nonce = CREATE_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = path.with_file_name(format!(
            ".{file_name}.create-{}-{nonce}.tmp",
            process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (file, candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };

    let published = (|| {
        temp_file.write_all(bytes)?;
        temp_file.sync_all()?;
        drop(temp_file);
        fs::hard_link(&temp_path, path)
    })();
    // Once the hard link succeeds, the World is durably published. A leftover
    // hidden preparation file is cleanup debt, not a failed create operation.
    let _ = fs::remove_file(&temp_path);
    published
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::process;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_document::{WorldBranchCause, WorldLineage, WorldParent};
    use world_persistence::{WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};

    fn archive(world_time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new("world-machine.archive-create-mock", "1"),
            world_time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn lineage() -> WorldLineage {
        WorldLineage {
            parent: WorldParent {
                document: Some("source".into()),
                pack: WorldPackRef::new("world-machine.archive-create-mock", "1"),
                world_time: 12,
                event_count: 0,
            },
            branch: WorldBranchCause::Strategy {
                choice_id: "mock.choice".into(),
                choice_title: "Mock Choice".into(),
                horizon: 20,
            },
        }
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-archive-create-{}-{nonce}-{label}",
            process::id()
        ))
    }

    #[test]
    fn creates_a_new_library_world_from_an_archive() {
        let root = temp_root("success");
        let library = WorldLibrary::new(root.clone());
        let id = WorldDocumentId::new("future-a").unwrap();
        let source = archive(42);

        let summary = library.create_from_archive(id.clone(), &source).unwrap();

        assert_eq!(summary.id, id);
        assert_eq!(summary.world_time, 42);
        assert_eq!(library.load(&summary.id).unwrap(), Some(source));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn creates_a_new_library_world_with_document_metadata() {
        let root = temp_root("document");
        let library = WorldLibrary::new(root.clone());
        let id = WorldDocumentId::new("future-a").unwrap();
        let source = WorldDocument::new(archive(42)).with_lineage(lineage());

        let summary = library.create_from_document(id.clone(), &source).unwrap();
        let stored = library.load_document(&id).unwrap().unwrap();

        assert_eq!(summary.id, id);
        assert_eq!(stored, source);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_to_overwrite_an_existing_library_world() {
        let root = temp_root("no-clobber");
        let library = WorldLibrary::new(root.clone());
        let id = WorldDocumentId::new("future-a").unwrap();
        let original = archive(7);
        library.create_from_archive(id.clone(), &original).unwrap();

        let result = library.create_from_archive(id.clone(), &archive(99));

        assert!(matches!(
            result,
            Err(LibraryError::DocumentAlreadyExists(existing)) if existing == id
        ));
        assert_eq!(library.load(&id).unwrap(), Some(original));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn concurrent_creators_publish_exactly_one_world_without_clobbering() {
        const CALLERS: usize = 8;
        let root = temp_root("concurrent-no-clobber");
        let library = Arc::new(WorldLibrary::new(root.clone()));
        let id = WorldDocumentId::new("future-a").unwrap();
        let barrier = Arc::new(Barrier::new(CALLERS));
        let mut handles = Vec::new();

        for caller in 0..CALLERS {
            let library = Arc::clone(&library);
            let id = id.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                let world_time = caller as u64 + 1;
                let result = library.create_from_archive(id, &archive(world_time));
                (world_time, result)
            }));
        }

        let results = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        let winners = results
            .iter()
            .filter_map(|(world_time, result)| result.as_ref().ok().map(|_| *world_time))
            .collect::<Vec<_>>();

        assert_eq!(winners.len(), 1);
        for (_, result) in &results {
            assert!(
                result.is_ok()
                    || matches!(result, Err(LibraryError::DocumentAlreadyExists(existing)) if existing == &id)
            );
        }
        assert_eq!(
            library.load(&id).unwrap().unwrap().world_time,
            winners[0]
        );
        let _ = fs::remove_dir_all(root);
    }
}
