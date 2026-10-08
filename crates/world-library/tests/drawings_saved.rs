//! A World's file carries the drawings of its cast for its cover, turn
//! after turn, although a turn hands its writer the drawings only when the
//! cast is drawn with others (v0.28: copying them every turn was a tenth of
//! a three-year harbour's turn).

use world_library::{describe_from_snapshot, DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_projection::ProjectionIntent::InvokeCommand;

const PASS: &str = "tiny-society.let-day-pass";

fn temp_root(label: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "world-machine-drawings-{}-{nonce}-{label}",
        std::process::id()
    ))
}

/// Drawings with every number to three places.
fn rounded(drawings: &[serde_json::Value]) -> Vec<serde_json::Value> {
    fn round(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Number(number) => number
                .as_f64()
                .map(|x| serde_json::json!((x * 1000.0).round() as i64))
                .unwrap_or_else(|| value.clone()),
            serde_json::Value::Array(values) => values.iter().map(round).collect(),
            serde_json::Value::Object(fields) => fields
                .iter()
                .map(|(key, value)| (key.clone(), round(value)))
                .collect(),
            other => other.clone(),
        }
    }
    drawings.iter().map(round).collect()
}

/// What the file says, against what the session's latest snapshot draws.
fn assert_cover_is_drawn(
    session: &DurableWorldSession,
    library: &WorldLibrary,
    id: &WorldDocumentId,
    when: &str,
) {
    session.flush().unwrap();
    let file = library.load_document(id).unwrap().unwrap();
    let mut expected = file.metadata.clone();
    describe_from_snapshot(&mut expected, &session.snapshot());
    assert!(
        !file.metadata.display_drawings.is_empty(),
        "{when}: the file draws no one"
    );
    // A file keeps numbers as JSON does, to the nearest float rather than
    // the last digit (see `three_years`), so they are compared rounded.
    assert_eq!(
        rounded(&file.metadata.display_drawings),
        rounded(&expected.display_drawings),
        "{when}: the file's drawings are not the cast's"
    );
    assert_eq!(
        rounded(&file.metadata.display_drawings),
        rounded(&session.metadata().display_drawings),
        "{when}: the file's drawings are not the session's"
    );
}

#[test]
fn every_turn_saves_the_drawings_of_the_cast_and_a_reopened_world_does_too() {
    let root = temp_root("cover");
    let library = WorldLibrary::new(root.clone());
    let registry = world_builtins::registry().unwrap();
    let pack = registry.descriptors()[0].pack.id.clone();
    let id = WorldDocumentId::new("harbour").unwrap();
    let mut session = DurableWorldSession::create(id.clone(), &pack, &registry, &library).unwrap();
    let mut snapshot = session.snapshot();
    let mut play = |session: &mut DurableWorldSession, days: usize| {
        for _ in 0..days {
            if let Some(answer) = snapshot
                .commands
                .iter()
                .find(|c| c.question.is_some() && c.unavailable.is_none() && c.id != PASS)
            {
                let _ = session.handle(InvokeCommand(answer.id.clone()), &registry, &library);
            }
            snapshot = session
                .handle(InvokeCommand(PASS.into()), &registry, &library)
                .unwrap();
        }
    };
    for day in [1, 3, 10] {
        play(&mut session, day);
        assert_cover_is_drawn(&session, &library, &id, &format!("after {day} more days"));
    }
    drop(session);

    // Opened again, its writer starts afresh and is given the drawings with
    // its first change, whether or not the cast is drawn with others.
    let mut session = DurableWorldSession::open(id.clone(), &registry, &library).unwrap();
    play(&mut session, 1);
    assert_cover_is_drawn(&session, &library, &id, "reopened, after a day");
    play(&mut session, 5);
    assert_cover_is_drawn(&session, &library, &id, "reopened, after six days");
    drop(session);
    let _ = std::fs::remove_dir_all(root);
}
