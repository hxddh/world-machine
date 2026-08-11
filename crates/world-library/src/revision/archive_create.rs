use crate::{summary, LibraryError, WorldDocumentId, WorldDocumentSummary, WorldLibrary};
use world_persistence::WorldArchive;

impl WorldLibrary {
    /// Materialize a durable archive as a new Library World without overwriting
    /// an existing document.
    pub fn create_from_archive(
        &self,
        id: WorldDocumentId,
        archive: &WorldArchive,
    ) -> Result<WorldDocumentSummary, LibraryError> {
        if self.contains(&id)? {
            return Err(LibraryError::DocumentAlreadyExists(id));
        }

        self.save_with_revision(&id, archive)?;
        Ok(summary(id, archive))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
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
}
