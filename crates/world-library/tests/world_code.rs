//! World codes with a real World: a code reopens as the same World, event
//! for event, and a visit writes nothing back.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use world_host::WorldRegistry;
use world_library::{
    decode_world_code, write_world_code_file, DurableWorldSession, WorldCodeError, WorldDocumentId,
    WorldLibrary, WorldVisit, WORLD_CODE_SUFFIX,
};
use world_projection::{Ears, ProjectionIntent};

fn temp_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("world-code-{label}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

/// Every file under `root` and its bytes.
fn files_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.insert(path.clone(), fs::read(&path).unwrap());
            }
        }
    }
    files
}

/// A World from the first built-in Pack, played for a while so it has a
/// history: every day the first question on offer is answered, or else the
/// first choice there is.
fn played_world(label: &str) -> (PathBuf, WorldRegistry, WorldLibrary, DurableWorldSession) {
    let root = temp_root(label);
    let registry = world_builtins::registry().unwrap();
    let library = WorldLibrary::new(root.join("Worlds"));
    let pack = registry.descriptors()[0].pack.id.clone();
    let mut session = DurableWorldSession::create(
        WorldDocumentId::new("shared").unwrap(),
        &pack,
        &registry,
        &library,
    )
    .unwrap();
    for _ in 0..30 {
        let snapshot = session.snapshot();
        let available = snapshot
            .commands
            .iter()
            .filter(|command| command.unavailable.is_none())
            .collect::<Vec<_>>();
        let Some(pick) = available
            .iter()
            .find(|command| command.question.is_some())
            .or_else(|| available.first())
            .map(|command| command.id.clone())
        else {
            break;
        };
        session
            .handle(ProjectionIntent::InvokeCommand(pick), &registry, &library)
            .unwrap();
    }
    (root, registry, library, session)
}

#[test]
fn a_code_reopens_as_the_same_world_event_for_event() {
    let (root, registry, _library, session) = played_world("same");
    let original = session.current_archive().unwrap();
    assert!(
        original.events.len() > 10,
        "the World should have a history: {} events",
        original.events.len()
    );

    let code = session.world_code().unwrap();
    let decoded = decode_world_code(&code).unwrap();
    assert_eq!(decoded.archive.pack, original.pack);
    assert_eq!(decoded.archive.world_time, original.world_time);
    assert_eq!(decoded.archive.events.len(), original.events.len());
    for (index, (theirs, ours)) in decoded
        .archive
        .events
        .iter()
        .zip(&original.events)
        .enumerate()
    {
        assert_eq!(theirs, ours, "event {index} differs");
    }
    assert_eq!(decoded.archive.pending, original.pending);

    // Replayed, it is the same World: the same state, drawn the same way,
    // and it archives back to exactly what it was opened from.
    let visit = WorldVisit::open_code(&code, &registry).unwrap();
    assert_eq!(visit.archive(), &original);
    assert_eq!(visit.snapshot(), session.snapshot());
    let replayed = registry.open_archive(visit.archive()).unwrap();
    assert_eq!(replayed.archive().unwrap().unwrap(), original);
    assert_eq!(
        visit.world_code().unwrap(),
        code,
        "a visit's code is the same code"
    );
    assert_eq!(visit.pack(), session.pack());
    assert!(!visit.display_name().is_empty());

    eprintln!(
        "{} events in a {}-character code",
        original.events.len(),
        code.len()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_visit_writes_nothing_back() {
    let (root, registry, library, session) = played_world("visit");
    let code = session.world_code().unwrap();
    let code_file = root.join(format!("Harbor{WORLD_CODE_SUFFIX}"));
    write_world_code_file(&code_file, &code).unwrap();
    let world_file = library.path(session.document_id().unwrap());
    let before = files_under(&root);
    assert!(before.contains_key(&world_file));
    assert!(before.contains_key(&code_file));
    let listing_before = library.list().unwrap();

    let mut visit = WorldVisit::open_file(&code_file, &registry).unwrap();
    let seen = visit.snapshot();
    // Every choice the World offers, and something said to everyone, is
    // refused, and the visit stays as it was.
    let mut tried = 0;
    for command in &seen.commands {
        let refused = visit
            .handle(ProjectionIntent::InvokeCommand(command.id.clone()))
            .unwrap_err();
        assert!(matches!(refused, WorldCodeError::ReadOnlyVisit));
        tried += 1;
    }
    for item in &seen.canvas.items {
        let refused = visit
            .handle(ProjectionIntent::Say {
                to: item.id,
                words: "Hello!".into(),
                ears: Ears::Own,
            })
            .unwrap_err();
        assert!(matches!(refused, WorldCodeError::ReadOnlyVisit));
        tried += 1;
    }
    assert!(tried > 0, "there should be something to try");
    assert_eq!(visit.snapshot(), seen);

    // A second visit, from the pasted code, likewise.
    let mut pasted = WorldVisit::open_code(&code, &registry).unwrap();
    assert!(pasted
        .handle(ProjectionIntent::InvokeCommand("anything".into()))
        .is_err());

    // Nothing under the root changed: not the World's file, not the code
    // file, and no file was added to the library.
    assert_eq!(files_under(&root), before);
    assert_eq!(library.list().unwrap(), listing_before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_damaged_code_for_a_real_world_is_refused() {
    let (root, registry, _library, session) = played_world("damaged");
    let code = session.world_code().unwrap();
    for bad in [
        code[..code.len() * 2 / 3].to_string(),
        code[..code.len() - 3].to_string(),
        format!("{code}0"),
        code.replacen("wm1:", "wm1:A", 1),
    ] {
        let error = WorldVisit::open_code(&bad, &registry)
            .err()
            .expect("a damaged code must not open");
        assert!(matches!(error, WorldCodeError::Damaged), "{error:?}");
        assert!(error.to_string().contains("incomplete or damaged"));
    }
    let not_a_code = WorldVisit::open_code("my town is lovely", &registry)
        .err()
        .unwrap();
    assert_eq!(
        not_a_code.to_string(),
        "This is not a World code. A World code starts with wm1:"
    );
    fs::remove_dir_all(root).unwrap();
}

/// A guest is made from another World's listing alone: its file is not
/// opened or written.
#[test]
fn a_guest_comes_from_another_worlds_listing() {
    let (root, _registry, library, _session) = played_world("guest");
    let before = files_under(&root);
    let worlds = library.list().unwrap();
    let guest = worlds
        .iter()
        .find_map(world_library::guest_from)
        .expect("someone lives there");
    assert!(
        !guest.name.is_empty() && !guest.from.is_empty() && !guest.letter.is_empty(),
        "{guest:?}"
    );
    assert!(guest.gift.contains(&guest.from));
    assert_eq!(files_under(&root), before, "the other World is only read");
    let _ = fs::remove_dir_all(&root);
}
