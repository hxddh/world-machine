//! Machine checks for two of the kernel's invariants, run against a Pack's
//! own saved Worlds.
//!
//! - **Replay needs no decision maker** (AGENTS.md invariant 6). Each
//!   fixture's World is rebuilt from its recorded events alone, and the
//!   state it lands in is digested and compared with
//!   `tests/golden/replayed-state.digest`. Unlike the replay goldens
//!   ([`crate::replay`]), which digest the whole snapshot and so move with
//!   every change to the presentation, this digest covers event-sourced
//!   state only, written in a canonical form of its own (not `Debug`). It
//!   must never change: a saved World is history, and history does not
//!   move. There is no bless switch for it. A new fixture's line is added
//!   with `WORLD_PACK_ADD_REPLAYED_STATE=1`, which writes only lines that
//!   are missing and refuses to touch one that is there.
//! - **Events keep their causes** (invariant 5). [`provenance`] counts the
//!   events that have no `caused_by`, by kind, and
//!   [`assert_uncaused_kinds`] holds a Pack to the kinds it lists as
//!   starting a chain of their own (a player's deed, the day's turn, a
//!   resident's spontaneous ask): a ratchet, since in v0.28 most events of
//!   both Packs still carry no cause.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use world_core::{Value, WorldState};
use world_document::WorldDocument;
use world_host::WorldRegistry;
use world_persistence::{ArchivedEvent, WorldArchive};

use crate::replay::{fixtures, Digest};

/// The environment switch that adds a missing fixture's line to the
/// replayed-state golden file. It never rewrites a line that is there.
pub const ADD_ENV: &str = "WORLD_PACK_ADD_REPLAYED_STATE";

/// `state` in a canonical text of its own: the world time, then every
/// entity and every relation in id order with its fields in key order.
/// Written here rather than with `Debug`, so a new field on a kernel type,
/// or a change to how Rust formats one, cannot move the digest; only a
/// change to the state itself can.
pub fn canonical_state(state: &WorldState) -> String {
    let mut text = String::new();
    let _ = writeln!(text, "time {}", state.world_time());
    for entity in state.entities() {
        let _ = writeln!(text, "entity {} {}", entity.id.0, quoted(&entity.kind));
        for (key, value) in &entity.components {
            let _ = writeln!(text, "  {} = {}", quoted(key), canonical_value(value));
        }
    }
    for relation in state.relations() {
        let _ = writeln!(
            text,
            "relation {} {} {} -> {}",
            relation.id.0,
            quoted(&relation.kind),
            relation.from.0,
            relation.to.0
        );
        for (key, value) in &relation.properties {
            let _ = writeln!(text, "  {} = {}", quoted(key), canonical_value(value));
        }
    }
    text
}

fn canonical_value(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Text(text) => quoted(text),
        Value::Entity(id) => format!("@{}", id.0),
        Value::List(items) => format!(
            "[{}]",
            items
                .iter()
                .map(canonical_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Map(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(key, value)| format!("{}: {}", quoted(key), canonical_value(value)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// A text in double quotes, with only the quote, the backslash and line
/// breaks escaped: every other character is written as itself.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// The archive inside a saved World file.
pub fn archive_of(fixture: &Path) -> WorldArchive {
    let bytes = std::fs::read(fixture).expect("the fixture");
    WorldDocument::from_bytes(&bytes)
        .expect("the fixture is a World file")
        .archive
}

/// Checks every fixture in `fixtures_dir` rebuilds, from its recorded
/// events alone (`rebuild`: the Pack's own restore, which applies recorded
/// changes and decides nothing), to the state recorded for it in
/// `golden_file`. See the module documentation: the golden file is never
/// re-blessed.
pub fn assert_replayed_state_never_changes(
    fixtures_dir: &Path,
    golden_file: &Path,
    rebuild: impl Fn(&WorldArchive) -> Result<WorldState, String>,
) {
    let add = std::env::var_os(ADD_ENV).is_some();
    let golden = std::fs::read_to_string(golden_file).unwrap_or_default();
    let recorded: BTreeMap<&str, &str> = golden
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| line.split_once(' '))
        .collect();
    let mut failures = Vec::new();
    let mut missing = Vec::new();
    let mut checked = 0;
    for fixture in fixtures(fixtures_dir) {
        let stem = fixture.file_stem().unwrap().to_string_lossy().to_string();
        let archive = archive_of(&fixture);
        let state = match rebuild(&archive) {
            Ok(state) => state,
            Err(error) => {
                failures.push(format!("{stem}: does not rebuild from its events: {error}"));
                continue;
            }
        };
        let mut digest = Digest::new();
        let _ = digest.write_str(&canonical_state(&state));
        let line = format!(
            "{:016x} {} events={}",
            digest.hash(),
            digest.len(),
            archive.events.len()
        );
        match recorded.get(stem.as_str()) {
            Some(expected) if *expected == line => checked += 1,
            Some(expected) => failures.push(format!(
                "{stem}: the World its events rebuild has changed: recorded `{expected}`, now `{line}`"
            )),
            None => missing.push(format!("{stem} {line}")),
        }
    }
    if add && !missing.is_empty() {
        let mut text = golden.clone();
        if text.is_empty() {
            text.push_str(
                "# The state each fixture's recorded events rebuild, in a canonical form\n\
                 # (world-pack-testkit/src/invariants.rs). Never re-blessed: a line may be\n\
                 # added for a new fixture, and no line may ever change.\n",
            );
        }
        for line in &missing {
            text.push_str(line);
            text.push('\n');
        }
        std::fs::write(golden_file, text).expect("the golden file");
        missing.clear();
    }
    assert!(
        missing.is_empty(),
        "fixtures with no replayed-state line in {} (add them with {ADD_ENV}=1):\n{}",
        golden_file.display(),
        missing.join("\n")
    );
    assert!(
        failures.is_empty(),
        "replay no longer rebuilds the same World. History must not move; fix the change, \
         never the golden file:\n{}",
        failures.join("\n")
    );
    assert!(checked > 0 || add, "no fixture was checked");
}

/// How many events there are, and how many of each kind have no
/// `caused_by`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Provenance {
    pub events: usize,
    pub uncaused: BTreeMap<String, usize>,
}

impl Provenance {
    /// Events with no cause, of every kind.
    pub fn uncaused_total(&self) -> usize {
        self.uncaused.values().sum()
    }
}

/// Counts the events without `caused_by`, by kind.
pub fn provenance(events: &[ArchivedEvent]) -> Provenance {
    let mut provenance = Provenance {
        events: events.len(),
        ..Provenance::default()
    };
    for event in events.iter().filter(|event| event.caused_by.is_empty()) {
        *provenance.uncaused.entry(event.kind.clone()).or_default() += 1;
    }
    provenance
}

/// The kinds a Pack lets start a chain with no `caused_by`, read from
/// `file`: one kind a line, anything after `#` a comment (its reason).
pub fn roots_in(file: &Path) -> Vec<String> {
    std::fs::read_to_string(file)
        .unwrap_or_else(|error| panic!("{}: {error}", file.display()))
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter(|kind| !kind.is_empty())
        .map(str::to_string)
        .collect()
}

/// Fails if any event of a kind not listed in `roots_file` has no
/// `caused_by`: a ratchet on provenance. The list is what v0.28 found, and
/// may only shrink. A new kind of event says what caused it, or is added to
/// the list by hand with its reason; there is no switch that writes it.
/// Returns the count, which the test prints, and says which listed kinds
/// now always have a cause, so their lines can go.
pub fn assert_uncaused_kinds(
    label: &str,
    events: &[ArchivedEvent],
    roots_file: &Path,
) -> Provenance {
    let roots = roots_in(roots_file);
    let provenance = provenance(events);
    let unexplained: Vec<String> = provenance
        .uncaused
        .iter()
        .filter(|(kind, _)| !roots.iter().any(|root| root == *kind))
        .map(|(kind, count)| format!("{kind}: {count}"))
        .collect();
    eprintln!(
        "{label}: {} of {} events have no cause, of {} kinds",
        provenance.uncaused_total(),
        provenance.events,
        provenance.uncaused.len()
    );
    let caused_now: Vec<&String> = roots
        .iter()
        .filter(|root| {
            !provenance.uncaused.contains_key(*root)
                && events.iter().any(|event| &event.kind == *root)
        })
        .collect();
    if !caused_now.is_empty() {
        eprintln!(
            "{label}: these kinds always have a cause now; take them out of {}: {caused_now:?}",
            roots_file.display()
        );
    }
    assert!(
        unexplained.is_empty(),
        "{label}: events with no `caused_by`, of kinds not in {} (give them their cause):\n{}",
        roots_file.display(),
        unexplained.join("\n")
    );
    provenance
}

/// The events of `fixture` played on for `days` days by the fixture
/// builder ([`crate::replay::builder`]): what it recorded, and what the
/// current code adds.
pub fn played_events(
    registry: &WorldRegistry,
    fixture: &Path,
    pass: &str,
    days: usize,
) -> Vec<ArchivedEvent> {
    let archive = archive_of(fixture);
    let mut session = registry.open_archive(&archive).expect("the fixture opens");
    crate::replay::builder(&mut session, pass, days);
    session
        .archive()
        .expect("the World archives")
        .expect("the World keeps an archive")
        .events
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_core::{Entity, EntityId};

    #[test]
    fn the_canonical_state_is_ordered_quoted_and_complete() {
        let mut state = WorldState::default();
        state
            .seed_entity(
                Entity::new(EntityId(2), "person")
                    .with_component("name", "Mo \"the\" Baker\\")
                    .with_component("age", 40_i64),
            )
            .unwrap();
        state
            .seed_entity(Entity::new(EntityId(1), "place").with_component("open", true))
            .unwrap();
        assert_eq!(
            canonical_state(&state),
            "time 0\n\
             entity 1 \"place\"\n  \"open\" = true\n\
             entity 2 \"person\"\n  \"age\" = 40\n  \"name\" = \"Mo \\\"the\\\" Baker\\\\\"\n"
        );
    }

    #[test]
    fn provenance_counts_only_events_without_a_cause() {
        let event = |id: u64, kind: &str, caused_by: Vec<u64>| ArchivedEvent {
            id,
            kind: kind.into(),
            world_time: 0,
            actor: None,
            targets: Vec::new(),
            caused_by,
            payload: BTreeMap::new(),
            changes: Vec::new(),
        };
        let events = vec![
            event(1, "day", vec![]),
            event(2, "chat", vec![1]),
            event(3, "day", vec![]),
            event(4, "ask", vec![]),
        ];
        let counted = provenance(&events);
        assert_eq!(counted.events, 4);
        assert_eq!(counted.uncaused_total(), 3);
        assert_eq!(counted.uncaused.get("day"), Some(&2));
        let dir = std::env::temp_dir().join(format!("roots-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let both = dir.join("both.txt");
        std::fs::write(&both, "# kinds\nday # a turn\nask   # spontaneous\n").unwrap();
        assert_eq!(roots_in(&both), ["day", "ask"]);
        assert_uncaused_kinds("test", &events, &both);
        let one = dir.join("one.txt");
        std::fs::write(&one, "day # a turn\n").unwrap();
        let refused = std::panic::catch_unwind(|| {
            assert_uncaused_kinds("test", &events, &one);
        });
        let _ = std::fs::remove_dir_all(&dir);
        assert!(refused.is_err(), "an unexplained root kind was let through");
    }
}
