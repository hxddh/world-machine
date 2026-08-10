use super::DocumentRevision;
use crate::{required_archive, DurableWorldSession, LibraryError, WorldDocumentTarget};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use world_persistence::WorldArchive;
use world_projection::ProjectionSnapshot;

impl DurableWorldSession {
    /// Save the current in-memory World to a new external file and continue
    /// this session on that file.
    ///
    /// Unlike a normal edit, Save As intentionally does not verify the old
    /// target revision. This lets a stale window preserve the World it opened
    /// without overwriting a newer revision of the original document.
    pub fn save_as_file(
        &mut self,
        destination: PathBuf,
    ) -> Result<ProjectionSnapshot, LibraryError> {
        let archive = required_archive(self.session.as_ref())?;
        let revision = write_new_archive_file(&destination, &archive)?;

        self.target = WorldDocumentTarget::File(destination);
        self.revision = revision;
        Ok(self.session.snapshot())
    }
}

fn write_new_archive_file(
    path: &Path,
    archive: &WorldArchive,
) -> Result<DocumentRevision, LibraryError> {
    if path.try_exists()? {
        return Err(LibraryError::ExportDestinationExists(path.to_path_buf()));
    }

    let json = archive.to_json_pretty()?;
    let revision = DocumentRevision::from_bytes(json.as_bytes());
    atomic_write_new(path, json.as_bytes())?;
    Ok(revision)
}

fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<(), LibraryError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::other("World Save As path has no file name"))?
        .to_string_lossy();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_path = path.with_file_name(format!(
        ".{file_name}.save-as-{}-{nonce}.tmp",
        process::id()
    ));

    let result = (|| -> Result<(), LibraryError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);

        match fs::hard_link(&temp_path, path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(LibraryError::ExportDestinationExists(path.to_path_buf()))
            }
            Err(error) => Err(LibraryError::Io(error)),
        }
    })();

    let _ = fs::remove_file(&temp_path);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read_archive_file, write_archive_file};
    use std::env;
    use world_host::{HostError, WorldSession};
    use world_persistence::{WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
    use world_projection::{ProjectionCapabilities, ProjectionIntent, ProjectionSnapshot};

    const MOCK_PACK: &str = "world-machine.save-as-mock";

    struct MockSession {
        count: u64,
    }

    impl WorldSession for MockSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(MOCK_PACK, "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: format!("Mock {}", self.count),
                world_time: self.count,
                capabilities: ProjectionCapabilities { fork: false },
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            Err(HostError::Session("unused in Save As tests".into()))
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(mock_archive(self.count)))
        }
    }

    fn mock_archive(count: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new(MOCK_PACK, "1"),
            world_time: count,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-save-as-{}-{nonce}-{label}",
            process::id()
        ))
    }

    fn opened_external(path: PathBuf, count: u64) -> DurableWorldSession {
        let archive = mock_archive(count);
        let revision = write_archive_file(&path, &archive).unwrap();
        DurableWorldSession {
            target: WorldDocumentTarget::File(path),
            revision,
            session: Box::new(MockSession { count }),
        }
    }

    #[test]
    fn save_as_retargets_without_modifying_the_original() {
        let root = temp_root("retarget");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("Original.world");
        let destination = root.join("Copy.world");
        let mut session = opened_external(original.clone(), 4);
        let original_before = fs::read(&original).unwrap();

        let snapshot = session.save_as_file(destination.clone()).unwrap();

        assert_eq!(snapshot.title, "Mock 4");
        assert_eq!(session.file_path(), Some(destination.as_path()));
        assert_eq!(fs::read(&original).unwrap(), original_before);
        assert_eq!(read_archive_file(&destination).unwrap().world_time, 4);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_session_can_save_as_without_overwriting_the_winner() {
        let root = temp_root("stale");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("Shared.world");
        let destination = root.join("Preserved.world");
        let mut session = opened_external(original.clone(), 3);
        write_archive_file(&original, &mock_archive(9)).unwrap();

        session.save_as_file(destination.clone()).unwrap();

        assert_eq!(read_archive_file(&original).unwrap().world_time, 9);
        assert_eq!(read_archive_file(&destination).unwrap().world_time, 3);
        assert_eq!(session.file_path(), Some(destination.as_path()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn existing_destination_is_never_overwritten_or_retargeted() {
        let root = temp_root("existing");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("Original.world");
        let destination = root.join("Existing.world");
        let mut session = opened_external(original.clone(), 2);
        fs::write(&destination, "keep me").unwrap();

        assert!(matches!(
            session.save_as_file(destination.clone()),
            Err(LibraryError::ExportDestinationExists(path)) if path == destination
        ));
        assert_eq!(session.file_path(), Some(original.as_path()));
        assert_eq!(fs::read_to_string(&destination).unwrap(), "keep me");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_save_as_preserves_the_original_target() {
        let root = temp_root("failure");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("Original.world");
        let blocked_parent = root.join("not-a-directory");
        let destination = blocked_parent.join("Saved.world");
        let mut session = opened_external(original.clone(), 6);
        std::fs::File::create(&blocked_parent).unwrap();

        assert!(matches!(
            session.save_as_file(destination),
            Err(LibraryError::Io(_))
        ));
        assert_eq!(session.file_path(), Some(original.as_path()));
        assert_eq!(session.snapshot().title, "Mock 6");
        let _ = fs::remove_dir_all(root);
    }
}
