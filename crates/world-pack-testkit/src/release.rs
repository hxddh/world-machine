//! The saves each release ships, from World Machine 1.0 on.
//!
//! The 1.0 save contract (docs/SAVES.md): a World saved by any release
//! from 1.0 on opens, replays event for event and plays on in every later
//! release. Each release adds one save of its own per Pack to the Pack's
//! `tests/releases`, written on that release's tree the way its app writes
//! a World, and never rewritten; every later tree must open them all.
//!
//! A Pack takes the whole harness with one line in a test file of its own
//! (`tests/releases.rs`):
//!
//! ```ignore
//! world_pack_testkit::release_saves!(registry(), "my.pack", &[], "my.pack.pass", "town");
//! ```
//!
//! and a release adds its save with one command, on its own tree:
//!
//! ```text
//! WORLD_PACK_WRITE_RELEASE=v1.0.0 cargo test -p <pack> --test releases -- --ignored
//! ```
//!
//! which writes `tests/releases/v1.0.0-<name>.world`, to be checked in.

use std::path::{Path, PathBuf};

use world_document::WorldDocument;
use world_host::WorldRegistry;
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_projection::ProjectionIntent::InvokeCommand;

use crate::replay::{assert_wire_decodes, builder_day};

/// The environment switch naming the release whose save
/// [`write_release_save_from_env`] writes, such as `v1.0.0`.
pub const WRITE_ENV: &str = "WORLD_PACK_WRITE_RELEASE";

/// How many days the builder plays a release's save before it is written.
pub const RELEASE_SAVE_DAYS: usize = 30;

/// Every `*.world` file in `dir`, sorted; none when there is no `dir`.
pub fn release_saves(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "world"))
        .collect();
    out.sort();
    out
}

/// A scratch folder of its own, emptied first.
fn scratch(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!(
        "world-machine-release-{label}-{}-{nonce}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

/// Checks every release save in `dir`: opened as the app opens a file (so
/// the save contract lets it in), it replays event for event, opens to the
/// same snapshot each time (whose wire JSON decodes back to it), plays on
/// `days_on` days by the builder with every command it offers accepted,
/// only appends to its history, and reopens to the snapshot it showed.
/// Returns how many saves were checked; a Pack with none yet checks none.
pub fn assert_release_saves(
    registry: &WorldRegistry,
    dir: &Path,
    pass: &str,
    days_on: usize,
) -> usize {
    let saves = release_saves(dir);
    if saves.is_empty() {
        eprintln!("no release saves in {} yet", dir.display());
    }
    for save in &saves {
        let at = save.file_stem().unwrap().to_string_lossy().to_string();
        // The app's own way in, on a copy (opening takes the file's lock).
        let copy = scratch("open");
        let opened_at = copy.join(save.file_name().unwrap());
        std::fs::copy(save, &opened_at).expect("the save copied");
        let opened = DurableWorldSession::open_file(opened_at, registry)
            .unwrap_or_else(|error| panic!("{at}: the app opens it: {error}"));
        let shown = opened.snapshot();
        drop(opened);
        let _ = std::fs::remove_dir_all(&copy);

        let bytes = std::fs::read(save).expect("the save");
        let archive = WorldDocument::from_bytes(&bytes)
            .unwrap_or_else(|error| panic!("{at}: reads as a World file: {error}"))
            .archive;
        let mut session = registry
            .open_archive(&archive)
            .unwrap_or_else(|error| panic!("{at} opens: {error:?}"));
        let replayed = session.archive().unwrap().unwrap();
        assert!(
            replayed.events == archive.events,
            "{at}: replays event for event"
        );
        let snapshot = session.snapshot();
        assert!(snapshot == shown, "{at}: opens as the app opened it");
        assert!(
            registry.open_archive(&archive).unwrap().snapshot() == snapshot,
            "{at}: opens to the same snapshot each time"
        );
        assert_wire_decodes(&snapshot, &format!("{at} opened"));
        for day in 1..=days_on {
            builder_day(&mut session, pass, day, true);
            assert_wire_decodes(&session.snapshot(), &format!("{at} day {day}"));
        }
        let after = session.archive().unwrap().unwrap();
        let before = archive.events.len();
        assert!(
            after.events.len() > before || days_on == 0,
            "{at}: playing on records events"
        );
        assert!(
            after.events[..before] == archive.events[..],
            "{at}: playing on only appends"
        );
        let reopened = registry
            .open_archive(&after)
            .unwrap_or_else(|error| panic!("{at} played on reopens: {error:?}"));
        assert!(
            reopened.snapshot() == session.snapshot(),
            "{at}: played on, it reopens to the snapshot it showed"
        );
    }
    saves.len()
}

/// Writes a release's save: a new World of `pack_id`, begun with `begin`
/// and played `days` days by the builder the app's way (a Library World
/// made by `DurableWorldSession::create` and changed by `handle`), so the
/// file is exactly what the app writes. Written to `dir/<release>-<name>.world`,
/// which must not exist yet: a release's save is never rewritten.
#[allow(clippy::too_many_arguments)]
pub fn write_release_save(
    registry: &WorldRegistry,
    pack_id: &str,
    begin: &[&str],
    pass: &str,
    days: usize,
    dir: &Path,
    release: &str,
    name: &str,
) -> PathBuf {
    let target = dir.join(format!("{release}-{name}.world"));
    assert!(
        !target.exists(),
        "{} is there already: a release's save is never rewritten",
        target.display()
    );
    let root = scratch("write");
    let library = WorldLibrary::new(root.join("Worlds"));
    let id = WorldDocumentId::new(name).expect("a World name");
    let mut session = DurableWorldSession::create(id.clone(), pack_id, registry, &library)
        .unwrap_or_else(|error| panic!("a new World: {error}"));
    for command in begin {
        session
            .handle(InvokeCommand((*command).into()), registry, &library)
            .expect("the World begins");
    }
    for day in 1..=days {
        // The builder's day, through the app's own session.
        let snapshot = session.snapshot();
        if let Some(answer) = snapshot
            .commands
            .iter()
            .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != pass)
        {
            let _ = session.handle(InvokeCommand(answer.id.clone()), registry, &library);
        }
        if day.is_multiple_of(3) {
            let snapshot = session.snapshot();
            let deed = snapshot
                .canvas
                .plots
                .iter()
                .flat_map(|plot| plot.offers.iter())
                .find(|offer| offer.unavailable.is_none())
                .map(|offer| offer.command.clone())
                .or_else(|| {
                    snapshot
                        .commands
                        .iter()
                        .filter(|c| c.unavailable.is_none())
                        .find(|c| c.hand.as_ref().is_some_and(|hand| hand.verb != "Undo"))
                        .map(|c| c.id.clone())
                });
            if let Some(deed) = deed {
                let _ = session.handle(InvokeCommand(deed), registry, &library);
            }
        }
        session
            .handle(InvokeCommand(pass.into()), registry, &library)
            .expect("the day passes");
    }
    session.flush().expect("the World written");
    drop(session);
    std::fs::create_dir_all(dir).expect("the releases folder");
    std::fs::copy(library.path(&id), &target).expect("the save written");
    let _ = std::fs::remove_dir_all(&root);
    target
}

/// [`write_release_save`] for the release named by [`WRITE_ENV`], playing
/// [`RELEASE_SAVE_DAYS`] days; nothing without it.
pub fn write_release_save_from_env(
    registry: &WorldRegistry,
    pack_id: &str,
    begin: &[&str],
    pass: &str,
    dir: &Path,
    name: &str,
) {
    let Some(release) = std::env::var_os(WRITE_ENV) else {
        eprintln!("set {WRITE_ENV}=<release> (such as v1.0.0) to write its save");
        return;
    };
    let release = release.to_string_lossy();
    assert!(
        release.starts_with('v') && !release.contains(['/', '\\']),
        "{WRITE_ENV} names a release, such as v1.0.0"
    );
    let written = write_release_save(
        registry,
        pack_id,
        begin,
        pass,
        RELEASE_SAVE_DAYS,
        dir,
        &release,
        name,
    );
    eprintln!("wrote {}", written.display());
}

/// A save written now passes as a release save: the harness checked on
/// the tree as it stands, before any release has shipped one.
pub fn assert_a_save_written_now_passes(
    registry: &WorldRegistry,
    pack_id: &str,
    begin: &[&str],
    pass: &str,
    name: &str,
) {
    let dir = scratch("now");
    write_release_save(registry, pack_id, begin, pass, 6, &dir, "v0.0.0-now", name);
    assert_eq!(assert_release_saves(registry, &dir, pass, 3), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The release saves of a Pack, in one line of its own test file
/// (`tests/releases.rs`): `registry` (an expression making the Pack's
/// registry), the Pack's id, the commands that begin a World (`&[]` for
/// none), the command that lets a day pass, and the name its saves go by
/// (such as `harbour`). It makes three tests:
///
/// - every save in the Pack's `tests/releases` opens, replays and plays on
///   ([`assert_release_saves`]);
/// - a save written now passes the same checks;
/// - an ignored one that writes this release's save
///   (`WORLD_PACK_WRITE_RELEASE=v1.0.0 cargo test -p <pack> --test releases -- --ignored`).
#[macro_export]
macro_rules! release_saves {
    ($registry:expr, $pack:expr, $begin:expr, $pass:expr, $name:expr $(,)?) => {
        fn release_saves_dir() -> ::std::path::PathBuf {
            ::std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/releases")
        }

        #[test]
        fn every_release_save_opens_replays_and_plays_on() {
            $crate::release::assert_release_saves(&$registry, &release_saves_dir(), $pass, 3);
        }

        #[test]
        fn a_save_written_now_passes_as_a_release_save() {
            $crate::release::assert_a_save_written_now_passes(
                &$registry, $pack, $begin, $pass, $name,
            );
        }

        #[test]
        #[ignore]
        fn write_this_release_save() {
            $crate::release::write_release_save_from_env(
                &$registry,
                $pack,
                $begin,
                $pass,
                &release_saves_dir(),
                $name,
            );
        }
    };
}
