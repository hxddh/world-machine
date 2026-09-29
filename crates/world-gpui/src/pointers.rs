//! Gentle pointers: one small hint at a time at something the player has
//! not used yet, each shown once and never again once that thing is used.
//!
//! What has been pointed at and what has been used is the app's own
//! record, kept with its settings: interface state, never the World's.
//! Nothing here reads or changes a World.

use std::collections::BTreeSet;
use std::sync::Mutex;

/// Something a newcomer might not find by themselves.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Pointer {
    /// The hands: build, decorate, plant, move, give, invite.
    Hands,
    /// Leafing through the cards and turning one over.
    Cards,
    /// The drawer: the story, keepsakes, the book, who is who.
    Drawer,
    /// Looking closer: the zoom control, a pinch or the wheel.
    Zoom,
    /// The letter box, in the drawer, once a letter has come.
    Letters,
    /// The World as a strip along the edge of the screen.
    Strip,
}

impl Pointer {
    /// The order pointers are offered in: what a player can do with the
    /// World first, then where it keeps things, then how to look at it.
    pub const ORDER: [Pointer; 6] = [
        Pointer::Hands,
        Pointer::Cards,
        Pointer::Drawer,
        Pointer::Zoom,
        Pointer::Letters,
        Pointer::Strip,
    ];

    /// The name it is kept under in the app's settings.
    pub fn key(self) -> &'static str {
        match self {
            Pointer::Hands => "hands",
            Pointer::Cards => "cards",
            Pointer::Drawer => "drawer",
            Pointer::Zoom => "zoom",
            Pointer::Letters => "letters",
            Pointer::Strip => "strip",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ORDER.into_iter().find(|pointer| pointer.key() == key)
    }

    /// What the hint says, in English; shown through the app's catalog.
    pub fn words(self) -> &'static str {
        match self {
            Pointer::Hands => "Make something: build, plant, give or invite. Open your hands here, or press H.",
            Pointer::Cards => "More cards are waiting. ‹ and › leaf through them, and More turns one over (Space).",
            Pointer::Drawer => "The drawer keeps the story, keepsakes, the book and who is who. Open it here, or press ⌘I.",
            Pointer::Zoom => "Look closer with + and −, a pinch, or the scroll wheel.",
            Pointer::Letters => "A letter came for you. It waits in the drawer (⌘I).",
            Pointer::Strip => "Keep this World along the edge of your screen as a strip (⌥⌘S).",
        }
    }
}

/// Which pointers have been shown, and which things have been used.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Record {
    pub shown: BTreeSet<Pointer>,
    pub used: BTreeSet<Pointer>,
}

impl Record {
    /// From the names kept in the settings; a name this version does not
    /// know is left out.
    pub fn from_keys(shown: &[String], used: &[String]) -> Self {
        let read = |keys: &[String]| {
            keys.iter()
                .filter_map(|key| Pointer::from_key(key))
                .collect::<BTreeSet<_>>()
        };
        Self {
            shown: read(shown),
            used: read(used),
        }
    }

    /// The names to keep in the settings: shown, then used.
    pub fn keys(&self) -> (Vec<String>, Vec<String>) {
        let write = |set: &BTreeSet<Pointer>| {
            set.iter()
                .map(|pointer| pointer.key().to_string())
                .collect::<Vec<_>>()
        };
        (write(&self.shown), write(&self.used))
    }

    /// Whether a pointer is done with: shown once, or its thing used.
    pub fn done(&self, pointer: Pointer) -> bool {
        self.shown.contains(&pointer) || self.used.contains(&pointer)
    }
}

/// How long a World window is open before the first pointer, in seconds:
/// the greeting and the first question come first.
pub const FIRST_AFTER: f32 = 60.0;
/// How long after one pointer goes before another may come.
pub const GAP: f32 = 45.0;
/// How long a pointer stays if nobody answers it.
pub const LASTS: f32 = 30.0;

/// What a World window is doing now, as far as pointing goes.
#[derive(Clone, Debug, Default)]
pub struct Now {
    /// How long the window has been open, in seconds.
    pub open_for: f32,
    /// How long ago the last pointer went away, if one has come.
    pub since_last: Option<f32>,
    /// Whether nothing would be interrupted: no greeting or question
    /// being heard, no film playing, nobody being talked to, no hands or
    /// drawer open.
    pub quiet: bool,
    /// What the window offers now (a card to leaf through, a letter, a
    /// host that makes strips).
    pub offered: BTreeSet<Pointer>,
}

/// The pointer to show now, if any: the first in order that is offered,
/// neither shown before nor already used, once the first minute has gone
/// and nothing would be interrupted, and never two close together.
pub fn next(record: &Record, now: &Now) -> Option<Pointer> {
    if now.open_for < FIRST_AFTER || !now.quiet {
        return None;
    }
    if now.since_last.is_some_and(|since| since < GAP) {
        return None;
    }
    Pointer::ORDER
        .into_iter()
        .find(|pointer| now.offered.contains(pointer) && !record.done(*pointer))
}

type Keeper = Box<dyn Fn(&Record) + Send + Sync>;

struct Book {
    record: Record,
    keep: Keeper,
}

static BOOK: Mutex<Option<Book>> = Mutex::new(None);

/// Turns pointers on for this app, starting from `record` (read from its
/// settings); `keep` writes the record back whenever it changes. Until
/// this is called, no pointer is ever shown.
pub fn install(record: Record, keep: impl Fn(&Record) + Send + Sync + 'static) {
    if let Ok(mut book) = BOOK.lock() {
        *book = Some(Book {
            record,
            keep: Box::new(keep),
        });
    }
}

/// The record as it stands, if pointers are on.
pub fn record() -> Option<Record> {
    BOOK.lock().ok()?.as_ref().map(|book| book.record.clone())
}

fn change(update: impl FnOnce(&mut Record) -> bool) {
    let Ok(mut book) = BOOK.lock() else {
        return;
    };
    if let Some(book) = book.as_mut() {
        if update(&mut book.record) {
            (book.keep)(&book.record);
        }
    }
}

/// `pointer` has been shown: it never comes again.
pub fn shown(pointer: Pointer) {
    change(|record| record.shown.insert(pointer));
}

/// The player used what `pointer` is about: it never comes, or goes at
/// once if it is showing.
pub fn used(pointer: Pointer) {
    change(|record| record.used.insert(pointer));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> BTreeSet<Pointer> {
        Pointer::ORDER.into_iter().collect()
    }

    fn quiet(open_for: f32) -> Now {
        Now {
            open_for,
            since_last: None,
            quiet: true,
            offered: all(),
        }
    }

    #[test]
    fn nothing_is_pointed_at_in_the_first_minute_or_over_something_else() {
        let record = Record::default();
        assert_eq!(next(&record, &quiet(0.0)), None);
        assert_eq!(next(&record, &quiet(FIRST_AFTER - 1.0)), None);
        assert_eq!(next(&record, &quiet(FIRST_AFTER)), Some(Pointer::Hands));
        let busy = Now {
            quiet: false,
            ..quiet(600.0)
        };
        assert_eq!(next(&record, &busy), None, "a greeting, a film or a talk");
        let just_now = Now {
            since_last: Some(GAP - 1.0),
            ..quiet(600.0)
        };
        assert_eq!(next(&record, &just_now), None, "never two close together");
        let a_while = Now {
            since_last: Some(GAP),
            ..quiet(600.0)
        };
        assert_eq!(next(&record, &a_while), Some(Pointer::Hands));
    }

    #[test]
    fn pointers_come_in_order_each_once() {
        let mut record = Record::default();
        let mut seen = Vec::new();
        while let Some(pointer) = next(&record, &quiet(600.0)) {
            seen.push(pointer);
            record.shown.insert(pointer);
        }
        assert_eq!(seen, Pointer::ORDER.to_vec());
        assert_eq!(next(&record, &quiet(6000.0)), None, "each shows once");
    }

    #[test]
    fn a_thing_used_is_never_pointed_at() {
        let mut record = Record::default();
        record.used.insert(Pointer::Hands);
        record.used.insert(Pointer::Drawer);
        assert_eq!(next(&record, &quiet(600.0)), Some(Pointer::Cards));
        record.used.insert(Pointer::Cards);
        assert_eq!(next(&record, &quiet(600.0)), Some(Pointer::Zoom));
    }

    #[test]
    fn only_what_is_offered_is_pointed_at_and_the_rest_wait() {
        let record = Record::default();
        // No card to leaf through and no letter yet: the drawer comes
        // before either, and the letter box waits for a letter.
        let now = Now {
            offered: [Pointer::Drawer, Pointer::Zoom].into_iter().collect(),
            ..quiet(600.0)
        };
        assert_eq!(next(&record, &now), Some(Pointer::Drawer));
        let mut record = record;
        record.shown.extend([Pointer::Drawer, Pointer::Zoom]);
        assert_eq!(next(&record, &now), None);
        let letter = Now {
            offered: [Pointer::Letters].into_iter().collect(),
            ..quiet(900.0)
        };
        assert_eq!(next(&record, &letter), Some(Pointer::Letters));
    }

    #[test]
    fn the_record_is_kept_by_name_and_reads_back() {
        let mut record = Record::default();
        record.shown.insert(Pointer::Zoom);
        record.used.extend([Pointer::Hands, Pointer::Strip]);
        let (shown, used) = record.keys();
        assert_eq!(shown, vec!["zoom".to_string()]);
        assert_eq!(used, vec!["hands".to_string(), "strip".to_string()]);
        assert_eq!(Record::from_keys(&shown, &used), record);
        // A name a later version wrote is left out, not an error.
        let later = Record::from_keys(&["zoom".into(), "sailing".into()], &[]);
        assert_eq!(later.shown.len(), 1);
    }

    /// Every hint is in the app's Chinese catalog, whole.
    #[test]
    fn every_hint_is_translated() {
        let catalog = world_i18n::Catalog::parse(crate::i18n::APP_ZH_HANS);
        for pointer in Pointer::ORDER {
            assert!(
                catalog.exact(pointer.words()).is_some(),
                "{:?} has no zh-Hans: {}",
                pointer,
                pointer.words()
            );
        }
    }
}
