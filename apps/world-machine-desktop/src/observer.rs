use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use world_library::{DurableWorldSession, WorldLibrary};
use world_observer::{CatchUpPolicy, ObserverKey, ObserverStore};

const DEFAULT_SECONDS_PER_PERIOD: u64 = 6 * 60 * 60;
const DEFAULT_MAX_PERIODS: u64 = 7;
const OBSERVER_DIRECTORY: &str = "Observer";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatchUpOutcome {
    pub periods: u64,
    pub world_time: u64,
}

pub fn catch_up(
    session: &mut DurableWorldSession,
    registry: &world_host::WorldRegistry,
    library: &WorldLibrary,
) -> Result<Option<CatchUpOutcome>, String> {
    catch_up_at(
        session,
        registry,
        library,
        current_unix_seconds()?,
        default_policy(),
    )
}

fn catch_up_at(
    session: &mut DurableWorldSession,
    registry: &world_host::WorldRegistry,
    library: &WorldLibrary,
    now_unix_seconds: u64,
    policy: CatchUpPolicy,
) -> Result<Option<CatchUpOutcome>, String> {
    let store = ObserverStore::new(observer_root(library));
    let key = observer_key(session)?;
    let claim = store
        .claim_due(&key, now_unix_seconds, policy)
        .map_err(|error| format!("could not update observer clock: {error}"))?;
    if !claim.is_due() {
        return Ok(None);
    }

    let periods = claim.periods();
    match session.advance_background_if_changed(periods, registry, library) {
        Ok(Some(snapshot)) => Ok(Some(CatchUpOutcome {
            periods,
            world_time: snapshot.world_time,
        })),
        Ok(None) => Ok(None),
        Err(error) => {
            if let Err(rollback) = store.rollback(&claim) {
                return Err(format!(
                    "background catch-up failed: {error}; observer rollback also failed: {rollback}"
                ));
            }
            Err(format!("background catch-up failed: {error}"))
        }
    }
}

fn default_policy() -> CatchUpPolicy {
    CatchUpPolicy::new(DEFAULT_SECONDS_PER_PERIOD, DEFAULT_MAX_PERIODS)
        .expect("desktop observer policy is valid")
}

fn current_unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("system clock is before the Unix epoch: {error}"))
}

fn observer_root(library: &WorldLibrary) -> PathBuf {
    library
        .root()
        .parent()
        .unwrap_or_else(|| library.root())
        .join(OBSERVER_DIRECTORY)
}

fn observer_key(session: &DurableWorldSession) -> Result<ObserverKey, String> {
    if let Some(document_id) = session.document_id() {
        return ObserverKey::new(format!("library:{}", document_id.as_str()))
            .map_err(|error| error.to_string());
    }
    let path = session
        .file_path()
        .ok_or_else(|| "durable World session has no observer identity".to_string())?;
    ObserverKey::new(format!("file:{}", normalized_path(path).display()))
        .map_err(|error| error.to_string())
}

fn normalized_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::process;
    use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
    use world_persistence::{
        WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION,
    };
    use world_projection::{ProjectionCapabilities, ProjectionIntent, ProjectionSnapshot};

    const PACK: &str = "world-machine.desktop-observer-test";

    struct MockSession {
        time: u64,
        fail_background: bool,
    }

    impl WorldSession for MockSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(PACK, "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: "Observer Test".into(),
                world_time: self.time,
                capabilities: ProjectionCapabilities { fork: false },
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            Err(HostError::Session("unused in observer tests".into()))
        }

        fn advance_background(&mut self, periods: u64) -> Result<ProjectionSnapshot, HostError> {
            if self.fail_background {
                return Err(HostError::Session("injected background failure".into()));
            }
            self.time += periods;
            Ok(self.snapshot())
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(archive(self.time)))
        }
    }

    struct StaticSession {
        time: u64,
    }

    impl WorldSession for StaticSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(PACK, "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: "Static Observer Test".into(),
                world_time: self.time,
                capabilities: ProjectionCapabilities { fork: false },
                ..ProjectionSnapshot::default()
            }
        }

        fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            Err(HostError::Session("unused in static observer tests".into()))
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(archive(self.time)))
        }
    }

    fn registry(fail_background: bool) -> WorldRegistry {
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: WorldPackRef::new(PACK, "1"),
                        title: "Observer Test".into(),
                        description: "Desktop catch-up test".into(),
                    },
                    move || {
                        Ok(Box::new(MockSession {
                            time: 0,
                            fail_background,
                        }))
                    },
                )
                .with_archive_opener(move |archive| {
                    Ok(Box::new(MockSession {
                        time: archive.world_time,
                        fail_background,
                    }))
                }),
            )
            .unwrap();
        registry
    }

    fn static_registry() -> WorldRegistry {
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: WorldPackRef::new(PACK, "1"),
                        title: "Static Observer Test".into(),
                        description: "Desktop no-op catch-up test".into(),
                    },
                    || Ok(Box::new(StaticSession { time: 0 })),
                )
                .with_archive_opener(|archive| {
                    Ok(Box::new(StaticSession {
                        time: archive.world_time,
                    }))
                }),
            )
            .unwrap();
        registry
    }

    fn archive(time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new(PACK, "1"),
            world_time: time,
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
            "world-machine-desktop-observer-{}-{nonce}-{label}",
            process::id()
        ))
    }

    #[test]
    fn policy_is_bounded_and_device_local() {
        assert_eq!(default_policy().seconds_per_period, 6 * 60 * 60);
        assert_eq!(default_policy().max_periods, 7);
        let library = WorldLibrary::new(PathBuf::from("/tmp/World Machine/Worlds"));
        assert_eq!(
            observer_root(&library),
            PathBuf::from("/tmp/World Machine/Observer")
        );
    }

    #[test]
    fn successful_claim_advances_durable_world() {
        let root = temp_root("success");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(false);
        let document_id = world_library::WorldDocumentId::new("living").unwrap();
        let mut session =
            DurableWorldSession::create(document_id, PACK, &registry, &library).unwrap();
        let policy = CatchUpPolicy::new(60, 3).unwrap();

        assert!(catch_up_at(&mut session, &registry, &library, 100, policy)
            .unwrap()
            .is_none());
        let outcome = catch_up_at(&mut session, &registry, &library, 280, policy)
            .unwrap()
            .unwrap();

        assert_eq!(outcome.periods, 3);
        assert_eq!(outcome.world_time, 3);
        assert_eq!(session.snapshot().world_time, 3);
        let reopened = DurableWorldSession::open(
            world_library::WorldDocumentId::new("living").unwrap(),
            &registry,
            &library,
        )
        .unwrap();
        assert_eq!(reopened.snapshot().world_time, 3);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn static_world_consumes_elapsed_time_without_reporting_progress() {
        let root = temp_root("static");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = static_registry();
        let document_id = world_library::WorldDocumentId::new("static").unwrap();
        let mut session = DurableWorldSession::create(
            document_id.clone(),
            PACK,
            &registry,
            &library,
        )
        .unwrap();
        let policy = CatchUpPolicy::new(60, 3).unwrap();
        catch_up_at(&mut session, &registry, &library, 100, policy).unwrap();
        let before = fs::read(library.path(&document_id)).unwrap();

        assert!(catch_up_at(&mut session, &registry, &library, 280, policy)
            .unwrap()
            .is_none());
        assert_eq!(session.snapshot().world_time, 0);
        assert_eq!(fs::read(library.path(&document_id)).unwrap(), before);
        assert!(catch_up_at(&mut session, &registry, &library, 280, policy)
            .unwrap()
            .is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_durable_catch_up_rolls_back_observer_claim() {
        let root = temp_root("rollback");
        let library = WorldLibrary::new(root.join("Worlds"));
        let good_registry = registry(false);
        let failing_registry = registry(true);
        let document_id = world_library::WorldDocumentId::new("living").unwrap();
        let mut session =
            DurableWorldSession::create(document_id, PACK, &good_registry, &library).unwrap();
        let policy = CatchUpPolicy::new(60, 3).unwrap();
        catch_up_at(&mut session, &good_registry, &library, 100, policy).unwrap();

        assert!(catch_up_at(&mut session, &failing_registry, &library, 280, policy).is_err());
        assert_eq!(session.snapshot().world_time, 0);

        let retry = catch_up_at(&mut session, &good_registry, &library, 280, policy)
            .unwrap()
            .unwrap();
        assert_eq!(retry.periods, 3);
        assert_eq!(session.snapshot().world_time, 3);
        let _ = fs::remove_dir_all(root);
    }
}
