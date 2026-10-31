//! Sittings (v0.30 S4, "one day a day"): a World moves one of its days a
//! sitting, and a sitting is a day of the player's own. The World's sitting
//! ledger, which its file carries as host metadata beside its history (1.0;
//! `world_document::DocumentSittings`), says which sitting opening a World
//! begins and how many of its days it may pass (`world_observer`); the
//! World is only told that a sitting began, never when. A World moved to
//! another machine, synced, saved as a new file, exported or imported keeps
//! its sittings, since they travel with the file. Time away does not move a
//! World on its own.

use world_document::DocumentSittings;
use world_library::{DurableWorldSession, WorldDocumentSummary, WorldLibrary};
use world_observer::{local_day, Sitting, SittingLedger};
use world_projection::{ProjectionIntent, SITTING_BEGINS_COMMAND};

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

/// A World's ledger as its file carries it.
fn ledger_of(session: &DurableWorldSession) -> SittingLedger {
    session.sittings().map(from_document).unwrap_or_default()
}

fn from_document(sittings: &DocumentSittings) -> SittingLedger {
    SittingLedger {
        sittings: sittings.sittings,
        began_on: sittings.began_on,
        days_passed: sittings.days_passed,
        morning_read: sittings.morning_read.clone(),
    }
}

fn to_document(ledger: &SittingLedger) -> DocumentSittings {
    DocumentSittings {
        sittings: ledger.sittings,
        began_on: ledger.began_on,
        days_passed: ledger.days_passed,
        morning_read: ledger.morning_read.clone(),
    }
}

/// Writes `ledger` into the World's file, if it changed.
fn write(
    session: &mut DurableWorldSession,
    library: &WorldLibrary,
    ledger: &SittingLedger,
) -> Result<(), String> {
    if ledger_of(session) == *ledger {
        return Ok(());
    }
    session
        .set_sittings(to_document(ledger), library)
        .map_err(|error| format!("could not write the sittings: {error}"))
}

/// The sitting begun on a day, as its World's ledger has it: what the
/// window writes down as each of its days passes, and its morning read.
#[derive(Clone, Debug)]
pub struct Ledger {
    began_on: i64,
}

impl Ledger {
    /// The sitting as the World's ledger has it now: how many days it has
    /// left.
    pub fn sitting(&self, session: &DurableWorldSession) -> Sitting {
        world_observer::sitting_at(&ledger_of(session), self.began_on)
    }

    /// The morning read this sitting, by its moment's stable key.
    pub fn morning_read(&self, session: &DurableWorldSession) -> Option<String> {
        ledger_of(session)
            .morning_read_on(self.began_on)
            .map(str::to_string)
    }

    /// One of the sitting's days passed. A ledger that cannot be written
    /// is let be: the next sitting comes all the same.
    pub fn spend_day(&self, session: &mut DurableWorldSession, library: &WorldLibrary) {
        let mut ledger = ledger_of(session);
        ledger.spend_day(self.began_on);
        if let Err(error) = write(session, library, &ledger) {
            eprintln!("could not write the sitting down: {error}");
        }
    }

    /// The morning (by its moment's stable key) was read: written down, so
    /// it shows once however often the World is opened that day.
    pub fn read_morning(
        &self,
        session: &mut DurableWorldSession,
        library: &WorldLibrary,
        moment: &str,
    ) {
        let mut ledger = ledger_of(session);
        if ledger.read_morning(self.began_on, moment) {
            if let Err(error) = write(session, library, &ledger) {
                eprintln!("could not write the morning down: {error}");
            }
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

/// Opens this World's sitting for today on this device's calendar.
pub fn sit(
    session: &mut DurableWorldSession,
    registry: &world_host::WorldRegistry,
    library: &WorldLibrary,
) -> Result<Sat, String> {
    sit_on(session, registry, library, today())
}

/// The sitting that opening the World on local day `today` is, written
/// down in its file if it is new; and the World told a sitting began on
/// every opening of it until it passes a day, so a morning lost (the app
/// closed between the two) still comes. The World keeps its morning to one
/// a day and refuses the rest, as does a World that keeps none.
fn sit_on(
    session: &mut DurableWorldSession,
    registry: &world_host::WorldRegistry,
    library: &WorldLibrary,
    today: i64,
) -> Result<Sat, String> {
    let mut ledger = ledger_of(session);
    let sitting = ledger.sit(today);
    write(session, library, &ledger)?;
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
    let began_on = ledger.began_on.unwrap_or(today);
    Ok(Sat {
        sitting,
        morning,
        ledger: Ledger { began_on },
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

/// Whether opening a library World now begins a new sitting, from the
/// ledger its file carries (read from its summary, without opening it): a
/// new day of its own waits for the player. `None` if it was never sat
/// with.
pub fn new_day_waiting(document: &WorldDocumentSummary) -> Option<bool> {
    new_day_waiting_on(document, today())
}

fn new_day_waiting_on(document: &WorldDocumentSummary, today: i64) -> Option<bool> {
    let ledger = from_document(document.host_sittings.as_ref()?);
    ledger.began_on?;
    Some(world_observer::sitting_at(&ledger, today).new)
}

/// Whether a library World sleeps until its next sitting (its file's
/// ledger says today's sitting has passed every day it may): Home then
/// tells it by the day the player said goodnight to, not the one the
/// next sitting opens on.
pub fn asleep(document: &WorldDocumentSummary) -> bool {
    asleep_on(document, today())
}

fn asleep_on(document: &WorldDocumentSummary, today: i64) -> bool {
    let Some(sittings) = document.host_sittings.as_ref() else {
        return false;
    };
    let ledger = from_document(sittings);
    let sitting = world_observer::sitting_at(&ledger, today);
    ledger.began_on.is_some() && !sitting.new && sitting.left == 0
}

/// Today, on this device's calendar.
fn today() -> i64 {
    let now = chrono::Local::now();
    local_day(now.timestamp(), now.offset().local_minus_utc() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_document::WorldDocument;
    use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
    use world_library::WorldDocumentId;
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

    /// The ledger the World's file holds now.
    fn on_disk(path: &std::path::Path) -> SittingLedger {
        let document = WorldDocument::from_bytes(&fs::read(path).unwrap()).unwrap();
        document
            .metadata
            .host_sittings
            .as_ref()
            .map(from_document)
            .unwrap_or_default()
    }

    fn summary(library: &WorldLibrary, id: &WorldDocumentId) -> WorldDocumentSummary {
        library
            .list()
            .unwrap()
            .into_iter()
            .find(|summary| &summary.id == id)
            .unwrap()
    }

    /// The ledger is the World's file's: written there, beside its history
    /// and never in it, and no ledger file is kept anywhere else.
    #[test]
    fn the_ledger_lives_in_the_world_file() {
        let root = temp_root("in-file");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(true);
        let id = WorldDocumentId::new("kept").unwrap();
        let mut session =
            DurableWorldSession::create(id.clone(), PACK, &registry, &library).unwrap();
        let sat = sit_on(&mut session, &registry, &library, 20_391).unwrap();
        sat.ledger().spend_day(&mut session, &library);
        session.flush().unwrap();
        let ledger = on_disk(&library.path(&id));
        assert_eq!(ledger.sittings, 1);
        assert_eq!(ledger.began_on, Some(20_391));
        assert_eq!(ledger.days_passed, 1);
        // Host metadata: the archive a Pack is handed has none of it.
        let document = WorldDocument::from_bytes(&fs::read(library.path(&id)).unwrap()).unwrap();
        let archive = serde_json::to_string(&document.archive).unwrap();
        assert!(!archive.contains("20391"), "{archive}");
        // Nothing beside the Library but the World's own files.
        let mut beside = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        beside.sort();
        assert_eq!(beside, ["Worlds"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_new_sitting_tells_the_world_and_gives_its_days() {
        let root = temp_root("sitting");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(true);
        let document_id = WorldDocumentId::new("living").unwrap();
        let mut session =
            DurableWorldSession::create(document_id.clone(), PACK, &registry, &library).unwrap();
        session.flush().unwrap();
        assert_eq!(
            new_day_waiting_on(&summary(&library, &document_id), 100),
            None
        );

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
            again.ledger().spend_day(&mut session, &library);
        }
        assert_eq!(again.ledger().sitting(&session).left, 0);
        // The first morning, then each day to the next even count.
        let days = MORNING + 2 * u64::from(FIRST_SITTING_DAYS);
        let spent = sit_on(&mut session, &registry, &library, 100).unwrap();
        assert_eq!(spent.sitting.left, 0, "the first sitting spent");
        session.flush().unwrap();
        // Home reads it from the file's summary: nothing waits today, a
        // new day does tomorrow.
        let listed = summary(&library, &document_id);
        assert_eq!(new_day_waiting_on(&listed, 100), Some(false));
        assert_eq!(new_day_waiting_on(&listed, 101), Some(true));
        // Asleep tonight, awake tomorrow (Home tells a sleeping World by
        // the day said goodnight to).
        assert!(asleep_on(&listed, 100));
        assert!(!asleep_on(&listed, 101));
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
        assert_eq!(ledger_of(&reopened).sittings, 2);
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
        let mut ledger = SittingLedger::default();
        ledger.sit(70);
        session
            .set_sittings(to_document(&ledger), &library)
            .unwrap();
        let sat = sit_on(&mut session, &registry, &library, 70).unwrap();
        assert!(!sat.sitting.new);
        assert!(sat.morning, "the morning comes on the next opening");
        assert_eq!(session.snapshot().world_time, MORNING + 1);
        let _ = fs::remove_dir_all(root);
    }

    /// Save As, export and import carry the sittings with the file: the
    /// same sitting, morning read and all, goes on in the World's new file,
    /// never a new first sitting; the file it was saved from keeps its own.
    #[test]
    fn a_world_saved_as_exported_or_imported_keeps_its_sitting() {
        let root = temp_root("save-as");
        let library = WorldLibrary::new(root.join("Worlds"));
        let registry = registry(true);
        let document_id = WorldDocumentId::new("saved").unwrap();
        let mut session =
            DurableWorldSession::create(document_id.clone(), PACK, &registry, &library).unwrap();
        let sat = sit_on(&mut session, &registry, &library, 400).unwrap();
        assert_eq!(sat.sitting.left, FIRST_SITTING_DAYS);
        sat.ledger().spend_day(&mut session, &library);
        sat.ledger().read_morning(&mut session, &library, "event-9");
        // Exported as it stands: the copy carries the sitting.
        let exported = root.join("Exported.world");
        session.export_file(&exported, &library).unwrap();
        assert_eq!(on_disk(&exported).days_passed, 1);
        assert_eq!(on_disk(&exported).morning_read.as_deref(), Some("event-9"));
        // Save As: the session is the new file's now.
        let destination = root.join("Saved elsewhere.world");
        session.save_as_file(destination.clone()).unwrap();
        session.flush().unwrap();
        let source = on_disk(&library.path(&document_id));
        assert_eq!(on_disk(&destination), source, "the same sitting");
        assert_eq!(source.morning_read.as_deref(), Some("event-9"));
        // Its days are charged to the new file now; the old keeps its own.
        sat.ledger().spend_day(&mut session, &library);
        session.flush().unwrap();
        assert_eq!(on_disk(&destination).days_passed, 2);
        assert_eq!(on_disk(&library.path(&document_id)).days_passed, 1);
        // Opened again the same day, the new file goes on with the sitting:
        // never a new first sitting.
        drop(session);
        let mut reopened = DurableWorldSession::open_file(destination, &registry).unwrap();
        let again = sit_on(&mut reopened, &registry, &library, 400).unwrap();
        assert!(!again.sitting.new);
        assert_eq!(again.sitting.left, FIRST_SITTING_DAYS - 2);
        assert_eq!(
            again.ledger().morning_read(&reopened).as_deref(),
            Some("event-9")
        );
        drop(reopened);
        // The exported copy, imported into the Library: the same sitting.
        let mut imported = DurableWorldSession::import_file(
            WorldDocumentId::new("imported").unwrap(),
            &exported,
            &registry,
            &library,
        )
        .unwrap();
        let again = sit_on(&mut imported, &registry, &library, 400).unwrap();
        assert!(!again.sitting.new);
        assert_eq!(again.sitting.number, sat.sitting.number);
        assert_eq!(again.sitting.left, sat.sitting.left - 1);
        let _ = fs::remove_dir_all(root);
    }

    /// A World moved to another machine (or synced): its file, copied into
    /// another Worlds folder, goes on with its sittings there, never a new
    /// first sitting.
    #[test]
    fn a_world_moved_to_another_machine_keeps_its_sittings() {
        let root = temp_root("moved");
        let here = WorldLibrary::new(root.join("Mac").join("Worlds"));
        let there = WorldLibrary::new(root.join("PC").join("Worlds"));
        let registry = registry(true);
        let id = WorldDocumentId::new("travels").unwrap();
        let mut session = DurableWorldSession::create(id.clone(), PACK, &registry, &here).unwrap();
        let sat = sit_on(&mut session, &registry, &here, 900).unwrap();
        for _ in 0..FIRST_SITTING_DAYS {
            sat.ledger().spend_day(&mut session, &here);
        }
        drop(session);
        fs::create_dir_all(there.root()).unwrap();
        fs::copy(here.path(&id), there.path(&id)).unwrap();
        let mut moved = DurableWorldSession::open(id.clone(), &registry, &there).unwrap();
        // The same day there: the first sitting, spent.
        let same = sit_on(&mut moved, &registry, &there, 900).unwrap();
        assert!(!same.sitting.new);
        assert_eq!(same.sitting.left, 0);
        // The next: the second sitting, of one day.
        let next = sit_on(&mut moved, &registry, &there, 901).unwrap();
        assert!(next.sitting.new);
        assert_eq!((next.sitting.number, next.sitting.left), (2, SITTING_DAYS));
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
        assert_eq!(sat.ledger().morning_read(&session), None);
        sat.ledger().read_morning(&mut session, &library, "event-7");
        assert_eq!(
            sat.ledger().morning_read(&session).as_deref(),
            Some("event-7")
        );
        // Past midnight, the window still writes to the sitting of the 50th.
        sat.ledger().spend_day(&mut session, &library);
        session.flush().unwrap();
        let ledger = on_disk(&library.path(&document_id));
        assert_eq!(ledger.days_passed, 1);
        assert_eq!(
            world_observer::sitting_at(&ledger, 51).left,
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
