//! Restore from the last good save (1.0 P0), as Home does it: a harbour
//! whose file fails to load (cut short, as by a crash mid-copy) is offered
//! its last good save, put back to it, and opens to the World it was then,
//! the damaged file kept beside it. A World the save contract refuses (a
//! preview's, or a newer World Machine's) is never offered it, and the
//! refusals read in the player's language.

use std::fs;
use std::path::PathBuf;
use world_library::{DurableWorldSession, LibraryError, WorldDocumentId, WorldLibrary};
use world_machine_desktop::restore::{offers_restore, restore_last_good, RESTORE_LABEL};
use world_projection::ProjectionIntent;

const PASS: &str = "tiny-society.let-day-pass";

fn temp_root(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-restore-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn pass_day(
    session: &mut DurableWorldSession,
    registry: &world_host::WorldRegistry,
    library: &WorldLibrary,
) {
    session
        .handle(
            ProjectionIntent::InvokeCommand(PASS.into()),
            registry,
            library,
        )
        .unwrap();
    session.flush().unwrap();
}

#[test]
fn a_harbour_that_fails_to_load_is_restored_from_its_last_good_save() {
    let root = temp_root("harbour");
    let library = WorldLibrary::new(root.join("Worlds"));
    let registry = world_builtins::registry().unwrap();
    let id = WorldDocumentId::new("harbour").unwrap();
    let mut session = DurableWorldSession::create(
        id.clone(),
        "world-machine.tiny-society",
        &registry,
        &library,
    )
    .unwrap();
    pass_day(&mut session, &registry, &library);
    let good = session.snapshot();
    pass_day(&mut session, &registry, &library);
    drop(session);

    let path = library.path(&id);
    let whole = fs::read(&path).unwrap();
    fs::write(&path, &whole[..whole.len() / 3]).unwrap();
    let error = DurableWorldSession::open(id.clone(), &registry, &library)
        .err()
        .expect("a cut file does not open");
    assert!(offers_restore(&error, &path), "{error}");
    assert_eq!(RESTORE_LABEL, "Restore the last good save");

    let kept = restore_last_good(&path)
        .unwrap()
        .expect("the damaged file kept");
    assert_eq!(fs::read(&kept).unwrap(), whole[..whole.len() / 3]);
    let restored = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
    assert_eq!(restored.snapshot(), good, "the World of its last good save");
    // While it is open, it is not offered (nor done).
    let busy = LibraryError::InUse(path.clone());
    assert!(!offers_restore(&busy, &path));
    assert!(matches!(
        restore_last_good(&path),
        Err(LibraryError::InUse(_))
    ));
    drop(restored);
    // Listed again on Home, it is a World of its own once more, and the
    // damaged file is not one.
    let listing = library.listing().unwrap();
    assert_eq!(listing.documents.len(), 1);
    assert!(listing.unreadable.is_empty(), "{:?}", listing.unreadable);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn a_world_the_save_contract_refuses_is_not_offered_its_backup() {
    let root = temp_root("refused");
    let library = WorldLibrary::new(root.join("Worlds"));
    let registry = world_builtins::registry().unwrap();
    let id = WorldDocumentId::new("refused").unwrap();
    let mut session = DurableWorldSession::create(
        id.clone(),
        "world-machine.tiny-society",
        &registry,
        &library,
    )
    .unwrap();
    pass_day(&mut session, &registry, &library);
    drop(session);
    let path = library.path(&id);
    assert!(world_library::has_good_backup(&path));
    for refused in [LibraryError::PreviewSave, LibraryError::NewerSave] {
        assert!(!offers_restore(&refused, &path), "{refused}");
    }
    // Nothing wrong but a missing backup: not offered either.
    fs::remove_file(world_library::backup_path(&path)).unwrap();
    let damaged = LibraryError::Io(std::io::Error::other("damaged"));
    assert!(!offers_restore(&damaged, &path));
    let _ = fs::remove_dir_all(root);
}

/// The contract's refusals and the restore action read in Chinese and
/// Japanese, whole, as Home shows them.
#[test]
fn the_refusals_read_in_the_players_language() {
    for (language, catalog) in [
        (
            world_i18n::Language::SimplifiedChinese,
            world_gpui::i18n::APP_ZH_HANS,
        ),
        (world_i18n::Language::Japanese, world_gpui::i18n::APP_JA),
    ] {
        world_i18n::install_for(language, catalog);
        world_i18n::set_thread_language(Some(language));
        for line in [
            LibraryError::NewerSave.to_string(),
            LibraryError::PreviewSave.to_string(),
            RESTORE_LABEL.to_string(),
            world_machine_desktop::restore::restored_message("Tiny Society"),
        ] {
            let shown = world_i18n::tr_owned(&line);
            assert_ne!(shown, line, "{language:?}: {line}");
            assert!(
                !shown.contains("World Machine. Update") && !shown.contains("preview version"),
                "{language:?}: {shown}"
            );
        }
        world_i18n::set_thread_language(None);
    }
}
