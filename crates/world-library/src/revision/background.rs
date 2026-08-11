use crate::{required_archive, DurableWorldSession, LibraryError, WorldLibrary};
use world_host::WorldRegistry;
use world_projection::ProjectionSnapshot;

impl DurableWorldSession {
    /// Advance a durable World through Pack-defined background periods using
    /// the same candidate -> persist -> commit transaction as interactive edits.
    ///
    /// The live session is replaced only after the candidate archive has been
    /// integrity-checked by the Host and atomically persisted to the current
    /// document target.
    pub fn advance_background(
        &mut self,
        periods: u64,
        registry: &WorldRegistry,
        library: &WorldLibrary,
    ) -> Result<ProjectionSnapshot, LibraryError> {
        if periods == 0 {
            return Ok(self.session.snapshot());
        }

        self.target.verify_revision(self.revision, library)?;

        let current_archive = required_archive(self.session.as_ref())?;
        let mut candidate = registry.open_archive(&current_archive)?;
        let snapshot = candidate.advance_background(periods)?;
        let next_archive = required_archive(candidate.as_ref())?;

        self.target.verify_revision(self.revision, library)?;
        let next_revision = self.target.persist(&next_archive, library)?;

        self.revision = next_revision;
        self.session = candidate;
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read_archive_file, write_archive_file, WorldDocumentTarget};
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldSession};
    use world_persistence::{WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};
    use world_projection::{ProjectionCapabilities, ProjectionIntent, ProjectionSnapshot};

    const MOCK_PACK: &str = "world-machine.background-mock";

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
            Err(HostError::Session("unused in background tests".into()))
        }

        fn advance_background(
            &mut self,
            periods: u64,
        ) -> Result<ProjectionSnapshot, HostError> {
            self.count += periods;
            Ok(self.snapshot())
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(mock_archive(self.count)))
        }
    }

    fn registry() -> WorldRegistry {
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: WorldPackRef::new(MOCK_PACK, "1"),
                        title: "Background Mock".into(),
                        description: "Durable background transaction regression".into(),
                    },
                    || Ok(Box::new(MockSession { count: 0 })),
                )
                .with_archive_opener(|archive| {
                    Ok(Box::new(MockSession {
                        count: archive.world_time,
                    }))
                }),
            )
            .unwrap();
        registry
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
            "world-machine-background-{}-{nonce}-{label}",
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
    fn background_progression_persists_before_committing_live_session() {
        let root = temp_root("success");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("Living.world");
        let mut session = opened_external(path.clone(), 5);
        let registry = registry();
        let library = WorldLibrary::new(root.join("library"));

        let snapshot = session
            .advance_background(3, &registry, &library)
            .unwrap();

        assert_eq!(snapshot.world_time, 8);
        assert_eq!(session.snapshot().world_time, 8);
        assert_eq!(read_archive_file(&path).unwrap().world_time, 8);

        let reopened = DurableWorldSession::open_file(path.clone(), &registry).unwrap();
        assert_eq!(reopened.snapshot().world_time, 8);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_change_blocks_background_progression_without_mutating_live_world() {
        let root = temp_root("conflict");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("Shared.world");
        let mut session = opened_external(path.clone(), 5);
        let registry = registry();
        let library = WorldLibrary::new(root.join("library"));
        write_archive_file(&path, &mock_archive(9)).unwrap();

        assert!(matches!(
            session.advance_background(2, &registry, &library),
            Err(LibraryError::DocumentChanged(changed)) if changed == path
        ));
        assert_eq!(session.snapshot().world_time, 5);
        assert_eq!(read_archive_file(&path).unwrap().world_time, 9);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn persist_failure_preserves_the_live_world() {
        use std::os::unix::fs::PermissionsExt;

        let root = temp_root("persist-failure");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("Living.world");
        let mut session = opened_external(path.clone(), 5);
        let registry = registry();
        let library = WorldLibrary::new(root.join("library"));

        fs::set_permissions(&root, fs::Permissions::from_mode(0o500)).unwrap();
        let result = session.advance_background(2, &registry, &library);
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();

        assert!(matches!(result, Err(LibraryError::Io(_))));
        assert_eq!(session.snapshot().world_time, 5);
        assert_eq!(read_archive_file(&path).unwrap().world_time, 5);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn zero_periods_is_a_strict_noop() {
        let root = temp_root("zero");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("Living.world");
        let mut session = opened_external(path.clone(), 5);
        let registry = registry();
        let library = WorldLibrary::new(root.join("library"));
        let before = fs::read(&path).unwrap();

        let snapshot = session
            .advance_background(0, &registry, &library)
            .unwrap();

        assert_eq!(snapshot.world_time, 5);
        assert_eq!(session.snapshot().world_time, 5);
        assert_eq!(fs::read(&path).unwrap(), before);
        let _ = fs::remove_dir_all(root);
    }
}
