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
//!   events that have no `caused_by`, by kind, and [`assert_caused`] holds
//!   a Pack to the few kinds it declares as roots (the player's deeds, a
//!   day passing, a World being made): every other event the current code
//!   records names its cause. A saved World's recorded events keep the
//!   form they were recorded in, so only what is played on is checked.

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

/// The kinds a Pack declares as roots, read from `file`: one kind a line,
/// anything after `#` a comment (its reason).
pub fn roots_in(file: &Path) -> Vec<String> {
    std::fs::read_to_string(file)
        .unwrap_or_else(|error| panic!("{}: {error}", file.display()))
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter(|kind| !kind.is_empty())
        .map(str::to_string)
        .collect()
}

/// The most events of a run, in percent, that may start a chain of their
/// own. Roots are the player's deeds, the day's turn and the World's
/// beginning; everything else the rules record follows from one of them.
pub const MAX_UNCAUSED_PERCENT: usize = 10;

/// Fails if any event has no `caused_by` and is not of a kind declared a
/// root in `roots_file` (the player's deeds, a day passing, a World being
/// made), or if more than [`MAX_UNCAUSED_PERCENT`] of `events` have no
/// cause. Pass only events the current code recorded: a saved World's
/// history keeps the form it was recorded in, causes or none.
///
/// A new kind of event says what caused it; a new kind of root is declared
/// by hand with its reason. Returns the count, which the test prints.
pub fn assert_caused(label: &str, events: &[ArchivedEvent], roots_file: &Path) -> Provenance {
    let roots = roots_in(roots_file);
    let provenance = provenance(events);
    let unexplained: Vec<String> = provenance
        .uncaused
        .iter()
        .filter(|(kind, _)| !roots.iter().any(|root| root == *kind))
        .map(|(kind, count)| format!("{kind}: {count}"))
        .collect();
    eprintln!(
        "{label}: {} of {} events have no cause ({:.1}%), of {} kinds: {:?}",
        provenance.uncaused_total(),
        provenance.events,
        percent(provenance.uncaused_total(), provenance.events),
        provenance.uncaused.len(),
        provenance.uncaused
    );
    assert!(
        unexplained.is_empty(),
        "{label}: events with no `caused_by`, of kinds not declared roots in {} \
         (give them their cause):\n{}",
        roots_file.display(),
        unexplained.join("\n")
    );
    assert!(
        provenance.uncaused_total() * 100 <= provenance.events * MAX_UNCAUSED_PERCENT,
        "{label}: {} of {} events have no cause, more than {MAX_UNCAUSED_PERCENT}%",
        provenance.uncaused_total(),
        provenance.events
    );
    provenance
}

fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 * 100.0 / whole as f64
    }
}

/// The events the current code records when `fixture` is played on for
/// `days` days by the fixture builder ([`crate::replay::builder`]): only
/// what it adds, not what the fixture recorded.
pub fn played_events(
    registry: &WorldRegistry,
    fixture: &Path,
    pass: &str,
    days: usize,
) -> Vec<ArchivedEvent> {
    let archive = archive_of(fixture);
    let recorded = archive.events.len();
    let mut session = registry.open_archive(&archive).expect("the fixture opens");
    crate::replay::builder(&mut session, pass, days);
    let mut events = session
        .archive()
        .expect("the World archives")
        .expect("the World keeps an archive")
        .events;
    events.drain(..recorded);
    events
}

/// Every event of a new World of `pack_id`, made now, begun with the
/// commands in `begin` (where it begins, say), and played for `days` days
/// by the fixture builder.
pub fn created_events(
    registry: &WorldRegistry,
    pack_id: &str,
    begin: &[&str],
    pass: &str,
    days: usize,
) -> Vec<ArchivedEvent> {
    let mut session = registry.create(pack_id).expect("a new World");
    for command in begin {
        session
            .handle(world_projection::ProjectionIntent::InvokeCommand(
                (*command).into(),
            ))
            .expect("the World begins");
    }
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
        let mut many = events.clone();
        many.extend((5..40).map(|id| event(id, "chat", vec![1])));
        assert_caused("test", &many, &both);
        let one = dir.join("one.txt");
        std::fs::write(&one, "day # a turn\n").unwrap();
        let refused = std::panic::catch_unwind(|| {
            assert_caused("test", &many, &one);
        });
        let too_many = std::panic::catch_unwind(|| {
            assert_caused("test", &events, &both);
        });
        let _ = std::fs::remove_dir_all(&dir);
        assert!(refused.is_err(), "an undeclared root kind was let through");
        assert!(
            too_many.is_err(),
            "three roots in four events were let through"
        );
    }
}
