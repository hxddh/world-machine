use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use world_library::{DurableWorldSession, WorldLibrary};
use world_observer::{CatchUpPolicy, ObserverKey, ObserverStore};

/// How much World time one wall-clock stretch is worth, and how much of a long
/// absence is ever caught up at once.
///
/// The cap used to be seven periods, from when a World had a fixed amount of
/// story in it and burning through it unattended was the risk. Eras removed
/// that ceiling, so the cap now says something simpler and truer: a World lives
/// through at most a week of its own time while you are away. A day away moves
/// it four periods; a week away moves it a week; a month away still moves it a
/// week, because a return should be readable rather than exhaustive.
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

/// How long a World has been left alone, in wall-clock seconds, and what that
/// is worth in the World's own time once the catch-up cap is applied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Absence {
    pub seconds: u64,
    pub periods: u64,
}

/// What each World on the shelf has been doing since it was last looked at.
///
/// A shelf of Worlds that keep living has one thing it has to say and was not
/// saying: how long each has been on its own. This reads the same observer
/// stamps the catch-up reads, and claims nothing — asking must never advance a
/// World.
pub fn absences(
    documents: &[world_library::WorldDocumentId],
    library: &WorldLibrary,
) -> std::collections::HashMap<world_library::WorldDocumentId, Absence> {
    let Ok(now) = current_unix_seconds() else {
        return std::collections::HashMap::new();
    };
    let store = ObserverStore::new(observer_root(library));
    let policy = default_policy();
    documents
        .iter()
        .filter_map(|document| {
            let key = library_key(document).ok()?;
            let last = store.last_observed(&key).ok().flatten()?;
            Some((document.clone(), absence_since(last, now, policy)))
        })
        .collect()
}

fn absence_since(last_observed: u64, now: u64, policy: CatchUpPolicy) -> Absence {
    let seconds = now.saturating_sub(last_observed);
    Absence {
        seconds,
        periods: (seconds / policy.seconds_per_period).min(policy.max_periods),
    }
}

const HOUR: u64 = 60 * 60;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;

/// The card's second line: how long this World has been alone, and how much of
/// its own life it got through in the meantime.
///
/// When an absence runs past the catch-up cap the two halves stop matching —
/// a month away still buys a week of World time — and saying both is the
/// honest version, not a bug to hide.
pub fn absence_sentence(absence: Absence) -> String {
    if absence.periods == 0 {
        return format!("You were here {} ago.", span(absence.seconds));
    }
    let lived = span(absence.periods * default_policy().seconds_per_period);
    let away = span(absence.seconds);
    let mut away = away.chars();
    let away = match away.next() {
        Some(first) => first.to_uppercase().collect::<String>() + away.as_str(),
        None => String::new(),
    };
    format!("{away} away · it lived {lived} without you.")
}

/// A stretch of time in words, rounded down to the largest unit that fits.
fn span(seconds: u64) -> String {
    let (count, one, many) = if seconds < HOUR {
        return "less than an hour".into();
    } else if seconds < DAY {
        (seconds / HOUR, "an hour", "hours")
    } else if seconds < WEEK {
        (seconds / DAY, "a day", "days")
    } else if seconds < MONTH {
        (seconds / WEEK, "a week", "weeks")
    } else {
        (seconds / MONTH, "a month", "months")
    };
    if count == 1 {
        return one.into();
    }
    format!("{} {many}", spelled(count))
}

/// Small counts read as words; past twelve the digits are clearer than the
/// words would be.
fn spelled(count: u64) -> String {
    const WORDS: [&str; 11] = [
        "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve",
    ];
    WORDS
        .get(count.saturating_sub(2) as usize)
        .map(|word| (*word).to_owned())
        .unwrap_or_else(|| count.to_string())
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

fn library_key(document_id: &world_library::WorldDocumentId) -> Result<ObserverKey, String> {
    ObserverKey::new(format!("library:{}", document_id.as_str())).map_err(|error| error.to_string())
}

fn observer_key(session: &DurableWorldSession) -> Result<ObserverKey, String> {
    if let Some(document_id) = session.document_id() {
        return library_key(document_id);
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
    fn an_absence_is_told_in_words_and_stops_at_the_catch_up_cap() {
        let policy = default_policy();
        let told = |seconds: u64| absence_sentence(absence_since(0, seconds, policy));

        // Under one period nothing is owed, so the card says when you were
        // here rather than claiming the World moved.
        assert_eq!(told(90), "You were here less than an hour ago.");
        assert_eq!(told(5 * HOUR), "You were here five hours ago.");

        assert_eq!(
            told(6 * HOUR),
            "Six hours away · it lived six hours without you."
        );
        assert_eq!(
            told(2 * DAY),
            "Two days away · it lived two days without you."
        );
        assert_eq!(told(WEEK), "A week away · it lived a week without you.");

        // Past the cap the two halves stop matching, and both are true.
        assert_eq!(told(MONTH), "A month away · it lived a week without you.");
        assert_eq!(
            told(5 * MONTH),
            "Five months away · it lived a week without you.",
            "a World left for half a year still only has a week of its own to report"
        );
    }

    #[test]
    fn the_shelf_reads_the_same_clock_the_catch_up_claims_and_moves_nothing() {
        let root = temp_root("absences");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(false);
        let document_id = world_library::WorldDocumentId::new("shelved").unwrap();
        let mut session =
            DurableWorldSession::create(document_id.clone(), PACK, &registry, &library).unwrap();

        assert!(
            absences(&[document_id.clone()], &library).is_empty(),
            "a World nobody has opened has no absence to report"
        );

        let policy = CatchUpPolicy::new(60, 3).unwrap();
        catch_up_at(&mut session, &registry, &library, 100, policy).unwrap();
        assert_eq!(absences(&[document_id.clone()], &library).len(), 1);

        // Asking twice must leave the World exactly where it was.
        let before = fs::read(library.path(&document_id)).unwrap();
        absences(&[document_id.clone()], &library);
        absences(&[document_id.clone()], &library);
        assert_eq!(fs::read(library.path(&document_id)).unwrap(), before);
        let claim = catch_up_at(&mut session, &registry, &library, 280, policy).unwrap();
        assert_eq!(
            claim.map(|outcome| outcome.periods),
            Some(3),
            "the three periods the World was owed survived being asked about"
        );

        let _ = fs::remove_dir_all(root);
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
        let mut session =
            DurableWorldSession::create(document_id.clone(), PACK, &registry, &library).unwrap();
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
