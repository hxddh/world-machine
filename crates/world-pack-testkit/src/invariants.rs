//! Machine checks for the kernel's invariant 5, run against a Pack's own
//! Worlds.
//!
//! - **Events keep their causes** (invariant 5). [`provenance`] counts the
//!   events that have no `caused_by`, by kind, and [`assert_caused`] holds
//!   a Pack to the few kinds it declares as roots (the player's deeds, a
//!   day passing, a World being made): every other event the current code
//!   records names its cause.
//!
//! Invariant 6 (replay needs no decision maker) is checked on fresh Worlds
//! by [`crate::replay::assert_fresh_world_replays`], and on the saves a
//! release ships by [`crate::release`].

use std::collections::BTreeMap;
use std::path::Path;

use world_host::WorldRegistry;
use world_persistence::ArchivedEvent;

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
/// cause.
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

/// Every event of a new World of `pack_id`, made now, begun with the
/// commands in `begin` (where it begins, say), and played for `days` days
/// by the builder ([`crate::replay::builder`]).
pub fn created_events(
    registry: &WorldRegistry,
    pack_id: &str,
    begin: &[&str],
    pass: &str,
    days: usize,
) -> Vec<ArchivedEvent> {
    let mut session = crate::replay::begun(registry, pack_id, begin);
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
