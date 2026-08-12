use crate::{
    required_archive, DurableWorldSession, LibraryError, WorldDocumentId, WorldDocumentSummary,
    WorldLibrary,
};
use world_document::{WorldBranchCause, WorldDocument, WorldLineage, WorldParent};

impl DurableWorldSession {
    /// Snapshot the current durable World into a new Library document and
    /// record this session as its immediate lineage parent.
    ///
    /// Forking never retargets or mutates the source session. The source
    /// revision must still match its durable target so the new lineage cannot
    /// silently claim a stale parent. External file sessions deliberately do
    /// not manufacture a local Library document id for their parent.
    pub fn fork_to_library(
        &self,
        document_id: WorldDocumentId,
        label: Option<String>,
        library: &WorldLibrary,
    ) -> Result<WorldDocumentSummary, LibraryError> {
        self.target.verify_revision(self.revision, library)?;

        let archive = required_archive(self.session.as_ref())?;
        let lineage = WorldLineage {
            parent: WorldParent {
                document: self.document_id().map(ToString::to_string),
                pack: archive.pack.clone(),
                world_time: archive.world_time,
                event_count: archive.events.len(),
            },
            branch: WorldBranchCause::Fork { label },
        };
        let fork = WorldDocument::new(archive).with_lineage(lineage);

        // Re-check after materializing the live archive so a concurrent source
        // edit cannot be ignored between the first revision check and creation.
        self.target.verify_revision(self.revision, library)?;
        library.create_from_document(document_id, &fork)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write_document_file;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_document::WorldDocumentMetadata;
    use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
    use world_persistence::{WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
    use world_projection::{ProjectionCapabilities, ProjectionIntent, ProjectionSnapshot};

    const MOCK_PACK: &str = "world-machine.fork-mock";

    struct MockSession {
        world_time: u64,
    }

    impl WorldSession for MockSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(MOCK_PACK, "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: format!("Fork Mock {}", self.world_time),
                world_time: self.world_time,
                capabilities: ProjectionCapabilities { fork: true },
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            Err(HostError::session("unused in fork tests"))
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(archive(self.world_time)))
        }
    }

    fn registry() -> WorldRegistry {
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: WorldPackRef::new(MOCK_PACK, "1"),
                        title: "Fork Mock".into(),
                        description: "Generic durable fork test".into(),
                    },
                    || Ok(Box::new(MockSession { world_time: 0 })),
                )
                .with_archive_opener(|archive| {
                    Ok(Box::new(MockSession {
                        world_time: archive.world_time,
                    }))
                }),
            )
            .unwrap();
        registry
    }

    fn archive(world_time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new(MOCK_PACK, "1"),
            world_time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn inherited_lineage() -> WorldLineage {
        WorldLineage {
            parent: WorldParent {
                document: Some("older-root".into()),
                pack: WorldPackRef::new(MOCK_PACK, "1"),
                world_time: 2,
                event_count: 0,
            },
            branch: WorldBranchCause::Fork {
                label: Some("older fork".into()),
            },
        }
    }

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        env::temp_dir().join(format!(
            "world-machine-fork-{}-{nonce}-{label}",
            process::id()
        ))
    }

    #[test]
    fn forks_a_library_world_at_the_current_durable_point() {
        let root = temp_root("library");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let source_id = WorldDocumentId::new("source").unwrap();
        let child_id = WorldDocumentId::new("child").unwrap();
        let source = WorldDocument::new(archive(12)).with_lineage(inherited_lineage());
        library
            .create_from_document(source_id.clone(), &source)
            .unwrap();
        let session = DurableWorldSession::open(source_id.clone(), &registry, &library).unwrap();

        let summary = session
            .fork_to_library(child_id.clone(), Some("alternate".into()), &library)
            .unwrap();
        let child = library.load_document(&child_id).unwrap().unwrap();

        assert_eq!(summary.id, child_id);
        assert_eq!(child.archive, source.archive);
        assert_eq!(
            child.metadata.lineage,
            Some(WorldLineage {
                parent: WorldParent {
                    document: Some(source_id.to_string()),
                    pack: WorldPackRef::new(MOCK_PACK, "1"),
                    world_time: 12,
                    event_count: 0,
                },
                branch: WorldBranchCause::Fork {
                    label: Some("alternate".into()),
                },
            })
        );
        assert_ne!(child.metadata, source.metadata);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_to_clobber_an_existing_fork_destination() {
        let root = temp_root("no-clobber");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let source_id = WorldDocumentId::new("source").unwrap();
        let child_id = WorldDocumentId::new("child").unwrap();
        library
            .create_from_document(source_id.clone(), &WorldDocument::new(archive(4)))
            .unwrap();
        library
            .create_from_document(child_id.clone(), &WorldDocument::new(archive(99)))
            .unwrap();
        let session = DurableWorldSession::open(source_id, &registry, &library).unwrap();

        let result = session.fork_to_library(child_id.clone(), None, &library);

        assert!(matches!(
            result,
            Err(LibraryError::DocumentAlreadyExists(existing)) if existing == child_id
        ));
        assert_eq!(
            library.load_document(&child_id).unwrap().unwrap().archive.world_time,
            99
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_a_fork_when_the_source_revision_is_stale() {
        let root = temp_root("stale");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let source_id = WorldDocumentId::new("source").unwrap();
        let child_id = WorldDocumentId::new("child").unwrap();
        library
            .create_from_document(source_id.clone(), &WorldDocument::new(archive(4)))
            .unwrap();
        let session = DurableWorldSession::open(source_id.clone(), &registry, &library).unwrap();
        library
            .save_document(&source_id, &WorldDocument::new(archive(7)))
            .unwrap();

        let result = session.fork_to_library(child_id.clone(), None, &library);

        assert!(matches!(
            result,
            Err(LibraryError::DocumentChanged(path)) if path == library.path(&source_id)
        ));
        assert!(!library.contains(&child_id).unwrap());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_file_forks_do_not_manufacture_a_local_parent_id() {
        let root = temp_root("external");
        fs::create_dir_all(&root).unwrap();
        let library = WorldLibrary::new(root.join("library"));
        let registry = registry();
        let external = root.join("source.world");
        let child_id = WorldDocumentId::new("child").unwrap();
        write_document_file(&external, &WorldDocument::new(archive(21))).unwrap();
        let session = DurableWorldSession::open_file(external, &registry).unwrap();

        session
            .fork_to_library(child_id.clone(), None, &library)
            .unwrap();
        let child = library.load_document(&child_id).unwrap().unwrap();
        let lineage = child.metadata.lineage.unwrap();

        assert_eq!(lineage.parent.document, None);
        assert_eq!(lineage.parent.pack, WorldPackRef::new(MOCK_PACK, "1"));
        assert_eq!(lineage.parent.world_time, 21);
        assert_eq!(lineage.branch, WorldBranchCause::Fork { label: None });
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fork_does_not_change_the_source_session_metadata() {
        let root = temp_root("source-metadata");
        let library = WorldLibrary::new(root.clone());
        let registry = registry();
        let source_id = WorldDocumentId::new("source").unwrap();
        let child_id = WorldDocumentId::new("child").unwrap();
        let metadata = WorldDocumentMetadata {
            lineage: Some(inherited_lineage()),
        };
        let source = WorldDocument {
            archive: archive(8),
            metadata: metadata.clone(),
        };
        library
            .create_from_document(source_id.clone(), &source)
            .unwrap();
        let session = DurableWorldSession::open(source_id, &registry, &library).unwrap();

        session.fork_to_library(child_id, None, &library).unwrap();

        assert_eq!(session.metadata(), &metadata);
        let _ = fs::remove_dir_all(root);
    }
}
