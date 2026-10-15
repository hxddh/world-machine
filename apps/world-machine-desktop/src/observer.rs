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
    fn store(&self) -> ObserverStore {
        ObserverStore::new(self.root.clone())
    }

    /// The sitting as the ledger has it now: how many days it has left.
    pub fn sitting(&self) -> Option<Sitting> {
        let ledger = self.store().ledger(&self.key).ok()?;
        Some(world_observer::sitting_at(&ledger, self.began_on))
    }

    /// The morning read this sitting, by its moment's stable key.
    pub fn morning_read(&self) -> Option<String> {
        let ledger = self.store().ledger(&self.key).ok()?;
        (ledger.began_on == Some(self.began_on))
            .then_some(ledger.morning_read)
            .flatten()
    }

    /// The sitting carried to the World `session` is now (after Save As: a
    /// new file, under a new key), the morning read and all: written down
    /// under its new key, and the ledger that writes there. The World it
    /// was saved from keeps its own.
    pub fn carried_to(&self, session: &DurableWorldSession) -> Result<Ledger, String> {
        let key = observer_key(session)?;
        if key == self.key {
            return Ok(self.clone());
        }
        carry(&self.store(), &self.key, &key)?;
        Ok(Ledger {
            root: self.root.clone(),
            key,
            began_on: self.began_on,
        })
    }

    /// One of the sitting's days passed. A ledger that cannot be written
    /// is let be: the next sitting comes all the same.
    pub fn spend_day(&self) {
        if let Err(error) = self.store().spend_day(&self.key, self.began_on) {
            eprintln!("could not write the sitting down: {error}");
        }
    }

    /// The morning (by its moment's stable key) was read: written down, so
    /// it shows once however often the World is opened that day.
    pub fn read_morning(&self, moment: &str) {
        if let Err(error) = self.store().read_morning(&self.key, self.began_on, moment) {
            eprintln!("could not write the morning down: {error}");
        }
    }

    /// Whether a new sitting has begun since this one: a new day of the
    /// player's (or a clock put right), with a window left open overnight.
    pub fn new_day_due(&self) -> bool {
        self.new_day_due_on(today())
    }

    fn new_day_due_on(&self, today: i64) -> bool {
        today != self.began_on
    }
}

/// Writes the sittings kept under `from` under `to` as well, whatever `to`
/// held: the same World under a new identity.
fn carry(store: &ObserverStore, from: &ObserverKey, to: &ObserverKey) -> Result<(), String> {
    let ledger = store
        .ledger(from)
        .map_err(|error| format!("could not read the sittings: {error}"))?;
    store
        .set_ledger(to, &ledger)
        .map_err(|error| format!("could not write the sittings: {error}"))
}

/// A World file imported into the Library (`session`, now its Library
/// copy) keeps the sittings it had as a file, if it had any: never a new
/// first sitting for a World already sat with.
pub fn carry_from_file(
    source: &Path,
    session: &DurableWorldSession,
    library: &WorldLibrary,
) -> Result<(), String> {
    let store = ObserverStore::new(observer_root(library));
    let from = ObserverKey::new(format!("file:{}", normalized_path(source).display()))
        .map_err(|error| error.to_string())?;
    let had = store
        .ledger(&from)
        .map_err(|error| format!("could not read the sittings: {error}"))?;
    if had.sittings == 0 {
        return Ok(());
    }
    carry(&store, &from, &observer_key(session)?)
}

/// Opens this World's sitting for today on this device's calendar.
pub fn sit(
    session: &mut DurableWorldSession,
    registry: &world_host::WorldRegistry,
    library: &WorldLibrary,
) -> Result<Sat, String> {
    sit_on(session, registry, library, today())
}

/// The sitting that opening the World on local day `today` is, written
/// down if it is new; and the World told a sitting began on every opening
/// of it until it passes a day, so a morning lost (the app closed between
/// the two) still comes. The World keeps its morning to one a day and refuses the rest,
/// as does a World that keeps none.
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
    // Told on every opening until the sitting passes its first day: the
    // morning belongs to the sitting's first World day, never to a day the
    // player passed into (whose morning is the next sitting's).
    let morning = sitting.left == sitting.allowance
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
/// this device's calendar, as the calendar has it that night (a change of
/// clocks between now and then included).
pub fn next_sitting_in() -> Option<u64> {
    use chrono::TimeZone;
    let now = chrono::Local::now();
    let midnight = now.date_naive().succ_opt()?.and_hms_opt(0, 0, 0)?;
    let next = chrono::Local
        .from_local_datetime(&midnight)
        .earliest()
        // A midnight the clocks skip: the first hour after it.
        .or_else(|| {
            chrono::Local
                .from_local_datetime(&(midnight + chrono::TimeDelta::hours(1)))
                .earliest()
        })?;
    u64::try_from(next.timestamp() - now.timestamp()).ok()
}

/// How long a window waits to look for a new sitting: until a moment past
/// the next local midnight, so the day has turned when it looks.
pub fn wake_wait() -> Option<std::time::Duration> {
    next_sitting_in().map(|seconds| std::time::Duration::from_secs(seconds.saturating_add(2)))
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
    /// its archive keeps, counts the mornings it was told of. Its lowest
    /// bit says this day has had its morning; a day passing clears it.
    const MORNING: u64 = 100;

    /// The test World's own day passing.
    const DAY: &str = "observer-test.day";

    /// A World that hears of each sitting, keeps its morning to one a day,
    /// and keeps the count in its time.
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
                    if command == SITTING_BEGINS_COMMAND
                        && self.keeps_mornings
                        && self.time.is_multiple_of(2) =>
                {
                    self.time += MORNING + 1;
                    Ok(self.snapshot())
                }
                ProjectionIntent::InvokeCommand(command) if command == DAY => {
                    self.time = (self.time | 1) + 1;
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
        assert!(!first.ledger().new_day_due_on(100));
        assert!(
            first.ledger().new_day_due_on(101),
            "a window left open overnight"
        );
        assert!(first.ledger().new_day_due_on(99), "a clock put right");
        assert_eq!(first.sitting.left, FIRST_SITTING_DAYS);
        assert_eq!(session.snapshot().world_time, MORNING + 1, "one morning");
        // Opened again the same day: the same sitting; the World, told
        // again, keeps to one morning.
        let again = sit_on(&mut session, &registry, &library, 100).unwrap();
        assert!(!again.sitting.new && !again.morning);
        assert_eq!(session.snapshot().world_time, MORNING + 1);
        // The window writes each day down as it passes.
        for _ in 0..FIRST_SITTING_DAYS {
            session
                .handle(
                    ProjectionIntent::InvokeCommand(DAY.into()),
                    &registry,
                    &library,
                )
                .unwrap();
            again.ledger().spend_day();
        }
        let days = MORNING + 1 + 5;
        let spent = sit_on(&mut session, &registry, &library, 100).unwrap();
        assert_eq!(spent.sitting.left, 0, "the first sitting spent");
        // A month away: a new sitting of one day, and one morning.
        let back = sit_on(&mut session, &registry, &library, 130).unwrap();
        assert!(back.sitting.new && back.morning);
        assert_eq!(back.sitting.left, SITTING_DAYS);
        assert_eq!(back.sitting.days_away, 30);
        assert_eq!(session.snapshot().world_time, days + MORNING + 1);
        // The World saved its mornings and days; nothing else moved it.
        drop(session);
        let reopened = DurableWorldSession::open(document_id, &registry, &library).unwrap();
        assert_eq!(reopened.snapshot().world_time, days + MORNING + 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_morning_lost_between_the_ledger_and_the_world_still_comes() {
        let root = temp_root("lost-morning");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(true);
        let document_id = WorldDocumentId::new("lost").unwrap();
        let mut session =
            DurableWorldSession::create(document_id, PACK, &registry, &library).unwrap();
        // The sitting written down, and the app closed before the World
        // heard of it.
        let store = ObserverStore::new(observer_root(&library));
        store
            .sit(&ObserverKey::new("library:lost").unwrap(), 70)
            .unwrap();
        let sat = sit_on(&mut session, &registry, &library, 70).unwrap();
        assert!(!sat.sitting.new);
        assert!(sat.morning, "the morning comes on the next opening");
        assert_eq!(session.snapshot().world_time, MORNING + 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_world_saved_as_a_new_file_keeps_its_sitting() {
        let root = temp_root("save-as");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(true);
        let document_id = WorldDocumentId::new("saved").unwrap();
        let mut session =
            DurableWorldSession::create(document_id.clone(), PACK, &registry, &library).unwrap();
        let sat = sit_on(&mut session, &registry, &library, 400).unwrap();
        assert_eq!(sat.sitting.left, FIRST_SITTING_DAYS);
        sat.ledger().spend_day();
        sat.ledger().read_morning("event-9");
        // Save As: the session is the new file's now.
        let destination = root.join("Saved elsewhere.world");
        fs::create_dir_all(&root).unwrap();
        session.save_as_file(destination.clone()).unwrap();
        let carried = sat.ledger().carried_to(&session).unwrap();
        let store = ObserverStore::new(observer_root(&library));
        let source = store
            .ledger(&ObserverKey::new("library:saved").unwrap())
            .unwrap();
        let saved = store.ledger(&observer_key(&session).unwrap()).unwrap();
        assert_eq!(saved, source, "the same sitting, morning read and all");
        assert_eq!(saved.morning_read.as_deref(), Some("event-9"));
        // Its days are charged to the new file now; the old keeps its own.
        carried.spend_day();
        assert_eq!(
            store
                .ledger(&observer_key(&session).unwrap())
                .unwrap()
                .days_passed,
            2
        );
        assert_eq!(
            store
                .ledger(&ObserverKey::new("library:saved").unwrap())
                .unwrap()
                .days_passed,
            1
        );
        // Opened again the same day, the new file goes on with the sitting:
        // never a first sitting of three days.
        drop(session);
        let mut reopened = DurableWorldSession::open_file(destination, &registry).unwrap();
        let again = sit_on(&mut reopened, &registry, &library, 400).unwrap();
        assert!(!again.sitting.new);
        assert_eq!(again.sitting.left, FIRST_SITTING_DAYS - 2);
        assert_eq!(again.ledger().morning_read().as_deref(), Some("event-9"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_world_file_imported_keeps_the_sittings_it_had() {
        let root = temp_root("import");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(true);
        // A World sat with as a file.
        let document_id = WorldDocumentId::new("as-file").unwrap();
        let mut session =
            DurableWorldSession::create(document_id, PACK, &registry, &library).unwrap();
        let file = root.join("Kept.world");
        fs::create_dir_all(&root).unwrap();
        session.save_as_file(file.clone()).unwrap();
        let sat = sit_on(&mut session, &registry, &library, 500).unwrap();
        sat.ledger().spend_day();
        drop(session);
        // Imported into the Library: the same sitting goes on.
        let imported = DurableWorldSession::import_file(
            WorldDocumentId::new("imported").unwrap(),
            &file,
            &registry,
            &library,
        )
        .unwrap();
        carry_from_file(&file, &imported, &library).unwrap();
        let mut imported = imported;
        let again = sit_on(&mut imported, &registry, &library, 500).unwrap();
        assert!(!again.sitting.new);
        assert_eq!(again.sitting.number, sat.sitting.number);
        assert_eq!(again.sitting.left, sat.sitting.left - 1);
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
        // The morning, once read, is kept for the sitting.
        assert_eq!(sat.ledger().morning_read(), None);
        sat.ledger().read_morning("event-7");
        assert_eq!(sat.ledger().morning_read().as_deref(), Some("event-7"));
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

    /// A window active across midnight looks again just after it, when
    /// the day the ledger began on is no longer today.
    #[test]
    fn a_window_wakes_just_past_midnight_on_a_new_day() {
        let wait = wake_wait().unwrap().as_secs();
        let midnight = next_sitting_in().unwrap();
        assert!(wait > midnight && wait <= midnight + 2, "{wait} {midnight}");
        let root = temp_root("midnight-wake");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(true);
        let document_id = WorldDocumentId::new("awake").unwrap();
        let mut session =
            DurableWorldSession::create(document_id, PACK, &registry, &library).unwrap();
        let sat = sit_on(&mut session, &registry, &library, 300).unwrap();
        // Still the 300th: nothing to wake to. The 301st: a new sitting.
        assert!(!sat.ledger().new_day_due_on(300));
        assert!(sat.ledger().new_day_due_on(301));
        let woken = sit_on(&mut session, &registry, &library, 301).unwrap();
        assert!(woken.sitting.new);
        assert!(!woken.ledger().new_day_due_on(301));
        let _ = fs::remove_dir_all(root);
    }
}
