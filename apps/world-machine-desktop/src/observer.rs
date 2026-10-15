//! Sittings (v0.30 S4, "one day a day"): a World moves one of its days a
//! sitting, and a sitting is a day of the player's own. The observer's
//! ledger, beside the Library on this device, says which sitting opening a
//! World begins and how many of its days it may pass (`world_observer`);
//! the World is only told that a sitting began, never when. Time away no
//! longer moves a World on its own.

use std::path::{Path, PathBuf};
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_observer::{local_day, ObserverKey, ObserverStore, Sitting};
use world_projection::{ProjectionIntent, SITTING_BEGINS_COMMAND};

const OBSERVER_DIRECTORY: &str = "Observer";

/// A World opened for a sitting: the sitting, and where to write down each
/// of its days as it passes.
#[derive(Clone, Debug)]
pub struct Sat {
    #[cfg_attr(not(test), allow(dead_code))]
    pub sitting: Sitting,
    /// Whether the World answered the new sitting with its morning.
    pub morning: bool,
    ledger: Ledger,
}

impl Sat {
    /// What the window writes down as each of the sitting's days passes.
    pub fn ledger(&self) -> Ledger {
        self.ledger.clone()
    }
}

/// Where a sitting's days are written down: the observer's ledger for one
/// World, and the local day its sitting began on.
#[derive(Clone, Debug)]
pub struct Ledger {
    root: PathBuf,
    key: ObserverKey,
    began_on: i64,
}

impl Ledger {
    /// The sitting as the ledger has it now: how many days it has left.
    pub fn sitting(&self) -> Option<Sitting> {
        let store = ObserverStore::new(self.root.clone());
        let ledger = store.ledger(&self.key).ok()?;
        Some(world_observer::sitting_at(&ledger, self.began_on))
    }

    /// One of the sitting's days passed. A ledger that cannot be written
    /// is let be: the next sitting comes all the same.
    pub fn spend_day(&self) {
        let store = ObserverStore::new(self.root.clone());
        if let Err(error) = store.spend_day(&self.key, self.began_on) {
            eprintln!("could not write the sitting down: {error}");
        }
    }
}

/// Opens this World's sitting for today on this device's calendar.
pub fn sit(
    session: &mut DurableWorldSession,
    registry: &world_host::WorldRegistry,
    library: &WorldLibrary,
) -> Result<Sat, String> {
    sit_on(session, registry, library, today())
}

/// The sitting that opening the World on local day `today` is: written
/// down if it is new, and then the World told a sitting began, so its
/// morning comes. A World that keeps no morning refuses, and that is all.
fn sit_on(
    session: &mut DurableWorldSession,
    registry: &world_host::WorldRegistry,
    library: &WorldLibrary,
    today: i64,
) -> Result<Sat, String> {
    let root = observer_root(library);
    let store = ObserverStore::new(root.clone());
    let key = observer_key(session)?;
    let sitting = store
        .sit(&key, today)
        .map_err(|error| format!("could not read the sittings: {error}"))?;
    let morning = sitting.new
        && session
            .handle(
                ProjectionIntent::InvokeCommand(SITTING_BEGINS_COMMAND.into()),
                registry,
                library,
            )
            .is_ok();
    // The ledger is written down by the day it began: a sitting that goes
    // on past midnight is still that day's.
    let began_on = store
        .ledger(&key)
        .ok()
        .and_then(|ledger| ledger.began_on)
        .unwrap_or(today);
    Ok(Sat {
        sitting,
        morning,
        ledger: Ledger {
            root,
            key,
            began_on,
        },
    })
}

/// How many seconds until the next sitting can begin: the next midnight on
/// this device's calendar.
pub fn next_sitting_in() -> Option<u64> {
    let now = chrono::Local::now();
    let offset = now.offset().local_minus_utc() as i64;
    let next = (local_day(now.timestamp(), offset) + 1) * world_observer::SECONDS_PER_DAY - offset;
    u64::try_from(next - now.timestamp()).ok()
}

/// Whether opening a library World now begins a new sitting: a new day of
/// its own waits for the player. `None` if it was never opened here.
pub fn new_day_waiting(document: &WorldDocumentId, library: &WorldLibrary) -> Option<bool> {
    let store = ObserverStore::new(observer_root(library));
    let key = ObserverKey::new(format!("library:{}", document.as_str())).ok()?;
    let ledger = store.ledger(&key).ok()?;
    ledger.began_on?;
    Some(world_observer::sitting_at(&ledger, today()).new)
}

/// Today, on this device's calendar.
fn today() -> i64 {
    let now = chrono::Local::now();
    local_day(now.timestamp(), now.offset().local_minus_utc() as i64)
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
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
    use world_observer::{FIRST_SITTING_DAYS, SITTING_DAYS};
    use world_persistence::{
        WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION,
    };
    use world_projection::{ProjectionCapabilities, ProjectionSnapshot};

    const PACK: &str = "world-machine.desktop-observer-test";

    /// How much a morning moves the test World's time: so its time, which
    /// its archive keeps, counts the mornings it was told of.
    const MORNING: u64 = 100;

    /// A World that hears of each sitting, and keeps the count in its time.
    struct MockSession {
        time: u64,
        keeps_mornings: bool,
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

        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            match intent {
                // A sitting is told as it is, and nothing else: no clock.
                ProjectionIntent::InvokeCommand(command)
                    if command == SITTING_BEGINS_COMMAND && self.keeps_mornings =>
                {
                    self.time += MORNING;
                    Ok(self.snapshot())
                }
                _ => Err(HostError::Session("no morning here".into())),
            }
        }

        fn archive(&self) -> Result<Option<WorldArchive>, HostError> {
            Ok(Some(WorldArchive {
                format: WORLD_ARCHIVE_FORMAT.into(),
                format_version: WORLD_ARCHIVE_VERSION,
                pack: WorldPackRef::new(PACK, "1"),
                world_time: self.time,
                events: Vec::new(),
                pending: Vec::new(),
                checkpoint: None,
            }))
        }
    }

    fn registry(keeps_mornings: bool) -> WorldRegistry {
        let mut registry = WorldRegistry::new();
        registry
            .register(
                WorldRegistration::new(
                    WorldDescriptor {
                        pack: WorldPackRef::new(PACK, "1"),
                        title: "Observer Test".into(),
                        description: "Desktop sitting test".into(),
                    },
                    move || {
                        Ok(Box::new(MockSession {
                            time: 0,
                            keeps_mornings,
                        }))
                    },
                )
                .with_archive_opener(move |archive| {
                    Ok(Box::new(MockSession {
                        time: archive.world_time,
                        keeps_mornings,
                    }))
                }),
            )
            .unwrap();
        registry
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
    fn the_ledger_lives_beside_the_library() {
        let library = WorldLibrary::new(PathBuf::from("/tmp/World Machine/Worlds"));
        assert_eq!(
            observer_root(&library),
            PathBuf::from("/tmp/World Machine/Observer")
        );
    }

    #[test]
    fn a_new_sitting_tells_the_world_and_gives_its_days() {
        let root = temp_root("sitting");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(true);
        let document_id = WorldDocumentId::new("living").unwrap();
        let mut session =
            DurableWorldSession::create(document_id.clone(), PACK, &registry, &library).unwrap();
        assert_eq!(new_day_waiting(&document_id, &library), None);

        let first = sit_on(&mut session, &registry, &library, 100).unwrap();
        assert!(first.sitting.new && first.morning);
        assert_eq!(first.sitting.left, FIRST_SITTING_DAYS);
        assert_eq!(session.snapshot().world_time, MORNING, "one morning");
        // Opened again the same day: the same sitting, no second morning.
        let again = sit_on(&mut session, &registry, &library, 100).unwrap();
        assert!(!again.sitting.new && !again.morning);
        assert_eq!(session.snapshot().world_time, MORNING);
        // The window writes each day down as it passes.
        for _ in 0..FIRST_SITTING_DAYS {
            again.ledger().spend_day();
        }
        let spent = sit_on(&mut session, &registry, &library, 100).unwrap();
        assert_eq!(spent.sitting.left, 0, "the first sitting spent");
        // A month away: a new sitting of one day, and one morning.
        let back = sit_on(&mut session, &registry, &library, 130).unwrap();
        assert!(back.sitting.new && back.morning);
        assert_eq!(back.sitting.left, SITTING_DAYS);
        assert_eq!(back.sitting.days_away, 30);
        assert_eq!(session.snapshot().world_time, 2 * MORNING);
        // The World saved its mornings; nothing else moved it.
        drop(session);
        let reopened = DurableWorldSession::open(document_id, &registry, &library).unwrap();
        assert_eq!(reopened.snapshot().world_time, 2 * MORNING);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_world_that_keeps_no_morning_still_has_its_sitting() {
        let root = temp_root("no-morning");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(false);
        let document_id = WorldDocumentId::new("quiet").unwrap();
        let mut session =
            DurableWorldSession::create(document_id, PACK, &registry, &library).unwrap();
        let sat = sit_on(&mut session, &registry, &library, 7).unwrap();
        assert!(sat.sitting.new && !sat.morning);
        assert_eq!(sat.sitting.left, FIRST_SITTING_DAYS);
        assert_eq!(session.snapshot().world_time, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_day_spent_is_written_to_the_day_the_sitting_began() {
        let root = temp_root("midnight");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(true);
        let document_id = WorldDocumentId::new("late").unwrap();
        let mut session =
            DurableWorldSession::create(document_id.clone(), PACK, &registry, &library).unwrap();
        let sat = sit_on(&mut session, &registry, &library, 50).unwrap();
        // Past midnight, the window still writes to the sitting of the 50th.
        sat.ledger().spend_day();
        let store = ObserverStore::new(observer_root(&library));
        let key = ObserverKey::new("library:late").unwrap();
        assert_eq!(store.ledger(&key).unwrap().days_passed, 1);
        assert_eq!(
            world_observer::sitting_at(&store.ledger(&key).unwrap(), 51).left,
            SITTING_DAYS,
            "and the 51st is a new sitting"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn the_next_sitting_is_within_a_day() {
        let seconds = next_sitting_in().unwrap();
        assert!(seconds <= 24 * 60 * 60 + 3600, "{seconds}");
    }
}
