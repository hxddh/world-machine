use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_observer::{CatchUpPolicy, ObserverKey, ObserverStore};

/// How much World time one wall-clock stretch is worth, and how much of a long
/// absence is ever caught up at once.
///
/// The cap used to be seven periods, from when a World had a fixed amount of
/// story in it and burning through it unattended was the risk. Eras removed
/// that ceiling, so the cap now says something simpler and truer: a World lives
/// through at most a week of your absence. A day away moves it four periods; a
/// week away moves it 28; a month away still moves it 28, because a return
/// should be readable rather than exhaustive.
const DEFAULT_SECONDS_PER_PERIOD: u64 = 6 * 60 * 60;
const DEFAULT_MAX_PERIODS: u64 = 4 * 7;
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
        demo_periods,
    )
}

/// Claims the time away that is due at `now_unix_seconds` and lives the
/// World through as much of it as `periods_for` allows (all of it in the
/// full app; see [`demo_periods`]), rolling the claim back if that fails.
fn catch_up_at(
    session: &mut DurableWorldSession,
    registry: &world_host::WorldRegistry,
    library: &WorldLibrary,
    now_unix_seconds: u64,
    policy: CatchUpPolicy,
    periods_for: fn(&DurableWorldSession, u64) -> u64,
) -> Result<Option<CatchUpOutcome>, String> {
    let store = ObserverStore::new(observer_root(library));
    let key = observer_key(session)?;
    let claim = store
        .claim_due(&key, now_unix_seconds, policy)
        .map_err(|error| format!("could not update observer clock: {error}"))?;
    if !claim.is_due() {
        return Ok(None);
    }

    let periods = periods_for(session, claim.periods());
    if periods == 0 {
        return Ok(None);
    }
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

/// How much of `periods` away a World lives through: all of it, except in
/// the demo, which never carries a World past its last day.
fn demo_periods(session: &DurableWorldSession, periods: u64) -> u64 {
    if !world_machine_desktop::demo::ENABLED {
        return periods;
    }
    let snapshot = session.snapshot();
    // A day that cannot be counted is past the last: nothing moves.
    let day = snapshot
        .calendar
        .map(|calendar| world_machine_desktop::demo::day_of(snapshot.world_time, calendar.length));
    world_machine_desktop::demo::periods_to_catch_up(&session.pack().id, day, periods)
}

/// How many seconds until this World next moves on its own: a period after
/// it was last caught up to. It moves when it is next opened after that.
pub fn next_move_in(session: &DurableWorldSession, library: &WorldLibrary) -> Option<u64> {
    let store = ObserverStore::new(observer_root(library));
    let key = observer_key(session).ok()?;
    let last = store.last_seen(&key).ok()??;
    let now = current_unix_seconds().ok()?;
    Some(seconds_until_next_period(
        last,
        now,
        DEFAULT_SECONDS_PER_PERIOD,
    ))
}

/// How many of its periods a library World has lived without being looked
/// at, as far as the next opening will catch it up: none if it was never
/// opened here.
pub fn periods_waiting(document: &WorldDocumentId, library: &WorldLibrary) -> Option<u64> {
    let store = ObserverStore::new(observer_root(library));
    let key = ObserverKey::new(format!("library:{}", document.as_str())).ok()?;
    let last = store.last_seen(&key).ok()??;
    let now = current_unix_seconds().ok()?;
    Some(periods_between(
        last,
        now,
        DEFAULT_SECONDS_PER_PERIOD,
        DEFAULT_MAX_PERIODS,
    ))
}

fn periods_between(last: u64, now: u64, period: u64, most: u64) -> u64 {
    (now.saturating_sub(last) / period.max(1)).min(most)
}

fn seconds_until_next_period(last: u64, now: u64, period: u64) -> u64 {
    period.saturating_sub(now.saturating_sub(last))
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

    #[test]
    fn the_next_period_is_counted_from_the_last_catch_up() {
        assert_eq!(seconds_until_next_period(1_000, 1_000, 600), 600);
        assert_eq!(seconds_until_next_period(1_000, 1_450, 600), 150);
        assert_eq!(seconds_until_next_period(1_000, 5_000, 600), 0);
    }

    #[test]
    fn time_waiting_counts_whole_periods_up_to_what_a_return_catches_up() {
        assert_eq!(periods_between(1_000, 1_500, 600, 7), 0);
        assert_eq!(periods_between(1_000, 2_300, 600, 7), 2);
        assert_eq!(periods_between(1_000, 100_000, 600, 7), 7);
        assert_eq!(periods_between(5_000, 1_000, 600, 7), 0);
    }
    use std::process;
    use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
    use world_persistence::{
        WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION,
    };
    use world_projection::{ProjectionCapabilities, ProjectionIntent, ProjectionSnapshot};

    const PACK: &str = "world-machine.desktop-observer-test";

    /// The observer's own mechanics, apart from what the demo allows: all
    /// the time away that is due.
    fn every_period(_: &DurableWorldSession, periods: u64) -> u64 {
        periods
    }

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
                capabilities: ProjectionCapabilities {
                    fork: false,
                    background: false,
                    talk: false,
                },
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
                capabilities: ProjectionCapabilities {
                    fork: false,
                    background: false,
                    talk: false,
                },
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
            checkpoint: None,
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
        assert_eq!(default_policy().max_periods, 28);
        assert_eq!(
            default_policy().max_periods * default_policy().seconds_per_period,
            7 * 24 * 60 * 60,
            "the catch-up cap is meant to read as exactly one week of World time"
        );
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

        assert!(
            catch_up_at(&mut session, &registry, &library, 100, policy, every_period)
                .unwrap()
                .is_none()
        );
        let outcome = catch_up_at(&mut session, &registry, &library, 280, policy, every_period)
            .unwrap()
            .unwrap();

        assert_eq!(outcome.periods, 3);
        assert_eq!(outcome.world_time, 3);
        assert_eq!(session.snapshot().world_time, 3);
        // One session per World: closed before it is opened again.
        drop(session);
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
        let mut session =
            DurableWorldSession::create(document_id.clone(), PACK, &registry, &library).unwrap();
        let policy = CatchUpPolicy::new(60, 3).unwrap();
        catch_up_at(&mut session, &registry, &library, 100, policy, every_period).unwrap();
        let before = fs::read(library.path(&document_id)).unwrap();

        assert!(
            catch_up_at(&mut session, &registry, &library, 280, policy, every_period)
                .unwrap()
                .is_none()
        );
        assert_eq!(session.snapshot().world_time, 0);
        assert_eq!(fs::read(library.path(&document_id)).unwrap(), before);
        assert!(
            catch_up_at(&mut session, &registry, &library, 280, policy, every_period)
                .unwrap()
                .is_none()
        );
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
        catch_up_at(
            &mut session,
            &good_registry,
            &library,
            100,
            policy,
            every_period,
        )
        .unwrap();

        assert!(catch_up_at(
            &mut session,
            &failing_registry,
            &library,
            280,
            policy,
            every_period
        )
        .is_err());
        assert_eq!(session.snapshot().world_time, 0);

        let retry = catch_up_at(
            &mut session,
            &good_registry,
            &library,
            280,
            policy,
            every_period,
        )
        .unwrap()
        .unwrap();
        assert_eq!(retry.periods, 3);
        assert_eq!(session.snapshot().world_time, 3);
        let _ = fs::remove_dir_all(root);
    }

    /// The app's own cap: the full app lives through all the time away; the
    /// demo moves no World of a Pack it does not offer (it fails closed),
    /// and its claim is spent, so the time is not caught up later either.
    #[test]
    fn the_demo_moves_no_world_it_does_not_offer() {
        let root = temp_root("demo");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(false);
        let document_id = world_library::WorldDocumentId::new("living").unwrap();
        let mut session =
            DurableWorldSession::create(document_id, PACK, &registry, &library).unwrap();
        let policy = CatchUpPolicy::new(60, 3).unwrap();
        assert!(!world_machine_desktop::demo::offers_pack(PACK));
        catch_up_at(&mut session, &registry, &library, 100, policy, demo_periods).unwrap();

        let outcome =
            catch_up_at(&mut session, &registry, &library, 280, policy, demo_periods).unwrap();
        if world_machine_desktop::demo::ENABLED {
            assert_eq!(outcome, None);
            assert_eq!(session.snapshot().world_time, 0);
        } else {
            assert_eq!(outcome.map(|outcome| outcome.periods), Some(3));
            assert_eq!(session.snapshot().world_time, 3);
        }
        let _ = fs::remove_dir_all(root);
    }
}
