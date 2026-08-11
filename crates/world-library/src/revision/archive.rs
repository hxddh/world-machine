use crate::{required_archive, DurableWorldSession, LibraryError};
use world_persistence::WorldArchive;

impl DurableWorldSession {
    /// Return the current live World's durable archive through the Host's
    /// integrity-checked session boundary.
    ///
    /// This does not read or rewrite the document target. It is a read-only
    /// source snapshot suitable for independent strategy evaluation, export,
    /// and diagnostic tooling.
    pub fn current_archive(&self) -> Result<WorldArchive, LibraryError> {
        required_archive(self.session.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{revision::DocumentRevision, WorldDocumentTarget};
    use std::path::PathBuf;
    use world_document::WorldDocumentMetadata;
    use world_host::{HostError, WorldSession};
    use world_persistence::{WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
    use world_projection::{ProjectionCapabilities, ProjectionIntent, ProjectionSnapshot};

    const MOCK_PACK: &str = "world-machine.archive-source-mock";

    struct MockSession {
        world_time: u64,
        archive: bool,
    }

    impl WorldSession for MockSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(MOCK_PACK, "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: "Archive Source Mock".into(),
                world_time: self.world_time,
                capabilities: ProjectionCapabilities { fork: false },
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            Err(HostError::session("unused in archive source tests"))
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(self.archive.then(|| WorldArchive {
                format: WORLD_ARCHIVE_FORMAT.into(),
                format_version: WORLD_ARCHIVE_VERSION,
                pack: self.pack(),
                world_time: self.world_time,
                events: Vec::new(),
                pending: Vec::new(),
            }))
        }
    }

    fn session(world_time: u64, archive: bool) -> DurableWorldSession {
        DurableWorldSession {
            target: WorldDocumentTarget::File(PathBuf::from("unused.world")),
            revision: DocumentRevision::from_bytes(b"archive-source-test"),
            metadata: WorldDocumentMetadata::default(),
            session: Box::new(MockSession {
                world_time,
                archive,
            }),
        }
    }

    #[test]
    fn current_archive_reads_the_live_session_without_touching_disk() {
        let session = session(42, true);

        let archive = session.current_archive().unwrap();

        assert_eq!(archive.world_time, 42);
        assert_eq!(archive.pack.id, MOCK_PACK);
    }

    #[test]
    fn current_archive_preserves_archive_unsupported_errors() {
        let session = session(42, false);

        assert!(matches!(
            session.current_archive(),
            Err(LibraryError::ArchiveUnsupported(pack)) if pack == MOCK_PACK
        ));
    }
}
