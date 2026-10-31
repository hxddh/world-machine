//! Sittings (v0.30 S4, "one day a day"): a World moves one of its days a
//! sitting, and a sitting is a day of the player's own, counted on the
//! calendar of the device it is played on. How many World days a sitting
//! may pass is worked out here, as a pure function of the World's ledger
//! and the day the sitting begins on.
//!
//! Wall-clock time lives only in this ledger, which the World's file
//! carries as host metadata beside its history (so a World moved to
//! another machine, or synced, keeps its sittings). It never enters a
//! World's record: the World is only ever told that a sitting began, and
//! it is the player who lets each of its days pass.

/// How many World days a World's very first sitting may pass: the whole of
/// a designed first session (1.0 P1), its first festival among them, which
/// at the game's intended pace (`demo/pacing.json`) is 20 to 40 minutes of
/// play before Goodnight. A World Pack designs what those days bring.
pub const FIRST_SITTING_DAYS: u32 = 6;

/// How many World days any later sitting may pass, however long the player
/// was away: a month away is one day, never thirty.
pub const SITTING_DAYS: u32 = 1;

pub const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

/// The local day (days since 1970-01-01 on this device's calendar) that
/// `unix_seconds` falls on, `utc_offset_seconds` east of UTC.
pub fn local_day(unix_seconds: i64, utc_offset_seconds: i64) -> i64 {
    (unix_seconds + utc_offset_seconds).div_euclid(SECONDS_PER_DAY)
}

/// A World's sittings, as its ledger keeps them: carried in the World's
/// file as host metadata, never in its events or state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SittingLedger {
    /// How many sittings there have been.
    pub sittings: u64,
    /// The local day the latest sitting began on: `None` before the first
    /// (a ledger that counts sittings but cannot say when, as a damaged
    /// one, has a later sitting next).
    pub began_on: Option<i64>,
    /// How many World days the latest sitting has passed.
    pub days_passed: u32,
    /// The morning the window has shown in the latest sitting, by its
    /// moment's stable key: presentation only, so it shows once however
    /// often the World is opened that day.
    pub morning_read: Option<String>,
}

impl SittingLedger {
    /// The ledger with one more World day passed in the sitting begun on
    /// `today`: unchanged for any other day's sitting, and never past what
    /// that sitting may pass.
    pub fn spent(&self, today: i64) -> SittingLedger {
        let sitting = sitting_at(self, today);
        if sitting.new || sitting.left == 0 {
            return self.clone();
        }
        SittingLedger {
            days_passed: self.days_passed.saturating_add(1),
            ..self.clone()
        }
    }

    /// The sitting that opening the World on local day `today` is: a new
    /// one, written down in the ledger as begun, or the one already begun
    /// that day.
    pub fn sit(&mut self, today: i64) -> Sitting {
        let sitting = sitting_at(self, today);
        if sitting.new {
            *self = sitting.begun(self, today);
        }
        sitting
    }

    /// Writes down that the sitting begun on `today` passed one of its
    /// World days: nothing past its allowance, and nothing for a sitting
    /// that is not today's. Returns the sitting as it now stands.
    pub fn spend_day(&mut self, today: i64) -> Sitting {
        *self = self.spent(today);
        sitting_at(self, today)
    }

    /// Writes down that the window showed the morning `moment` (its stable
    /// key) in the sitting begun on `today`, so it shows once: nothing for
    /// a sitting that is not today's, nor for a key that is empty or
    /// spans lines. Whether the ledger changed.
    pub fn read_morning(&mut self, today: i64, moment: &str) -> bool {
        if self.began_on != Some(today)
            || self.morning_read.as_deref() == Some(moment)
            || moment.is_empty()
            || moment.contains(['\n', '\r'])
        {
            return false;
        }
        self.morning_read = Some(moment.to_string());
        true
    }

    /// The morning read in the sitting begun on `began_on`, by its moment's
    /// stable key.
    pub fn morning_read_on(&self, began_on: i64) -> Option<&str> {
        (self.began_on == Some(began_on))
            .then_some(self.morning_read.as_deref())
            .flatten()
    }
}

/// A sitting: which one it is, whether it begins now, and how many World
/// days it may pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sitting {
    /// 1 for the World's first sitting.
    pub number: u64,
    /// Whether this sitting begins now, rather than going on with one
    /// begun earlier the same day.
    pub new: bool,
    /// How many World days the sitting may pass in all.
    pub allowance: u32,
    /// How many of them are still to pass.
    pub left: u32,
    /// How many of the player's days since the last sitting began: 0 for
    /// the first, for one that goes on, or after a clock turned back.
    pub days_away: u64,
}

impl Sitting {
    /// The ledger once this sitting has begun on `today`, from the one it
    /// was worked out from.
    pub fn begun(&self, before: &SittingLedger, today: i64) -> SittingLedger {
        if !self.new {
            return before.clone();
        }
        SittingLedger {
            sittings: self.number,
            began_on: Some(today),
            days_passed: 0,
            morning_read: None,
        }
    }

    /// Whether the sitting has passed every World day it may.
    pub fn spent(&self) -> bool {
        self.left == 0
    }
}

fn allowance(number: u64) -> u32 {
    if number <= 1 {
        FIRST_SITTING_DAYS
    } else {
        SITTING_DAYS
    }
}

/// A later sitting beginning now: one World day, whatever came before.
fn later(ledger: &SittingLedger, days_away: u64) -> Sitting {
    let number = ledger.sittings.saturating_add(1);
    let allowance = allowance(number);
    Sitting {
        number,
        new: true,
        allowance,
        left: allowance,
        days_away,
    }
}

/// The sitting that begins (or goes on) when the World is opened on local
/// day `today`, from its ledger. A new day of the player's is a new
/// sitting, with one World day to pass, however many days they were away;
/// the same day again goes on with the sitting already begun, with what it
/// has left. A day before the one the ledger names (a clock that was once
/// ahead, and is right again) begins a new sitting of one day too, so a
/// World is never kept asleep until the calendar catches up; and never the
/// first sitting's days, so nothing is won by turning the clock.
pub fn sitting_at(ledger: &SittingLedger, today: i64) -> Sitting {
    match ledger.began_on {
        _ if ledger.sittings == 0 => Sitting {
            number: 1,
            new: true,
            allowance: FIRST_SITTING_DAYS,
            left: FIRST_SITTING_DAYS,
            days_away: 0,
        },
        // Sittings counted, but not when: a later sitting.
        None => later(ledger, 0),
        Some(began) if today == began => {
            let allowance = allowance(ledger.sittings);
            Sitting {
                number: ledger.sittings,
                new: false,
                allowance,
                left: allowance.saturating_sub(ledger.days_passed),
                days_away: 0,
            }
        }
        Some(began) if today < began => later(ledger, 0),
        Some(began) => later(
            ledger,
            u64::try_from(today.saturating_sub(began)).unwrap_or(0),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn begin(ledger: &SittingLedger, today: i64) -> (Sitting, SittingLedger) {
        let sitting = sitting_at(ledger, today);
        (sitting, sitting.begun(ledger, today))
    }

    #[test]
    fn the_first_sitting_may_pass_the_first_sessions_days() {
        let (first, mut ledger) = begin(&SittingLedger::default(), 100);
        assert_eq!(
            first,
            Sitting {
                number: 1,
                new: true,
                allowance: FIRST_SITTING_DAYS,
                left: FIRST_SITTING_DAYS,
                days_away: 0
            }
        );
        // A first session of twenty to forty minutes is more than one
        // evening's days.
        const { assert!(FIRST_SITTING_DAYS > 3) };
        for _ in 1..FIRST_SITTING_DAYS {
            ledger = ledger.spent(100);
        }
        assert_eq!(sitting_at(&ledger, 100).left, 1);
        let ledger = ledger.spent(100).spent(100).spent(100);
        let spent = sitting_at(&ledger, 100);
        assert_eq!((spent.left, spent.new), (0, false));
        assert!(spent.spent());
        assert_eq!(
            ledger.days_passed, FIRST_SITTING_DAYS,
            "never past the allowance"
        );
    }

    #[test]
    fn every_later_sitting_may_pass_one_day_however_long_away() {
        let (_, ledger) = begin(&SittingLedger::default(), 100);
        let ledger = ledger.spent(100);
        for (away, today) in [(1, 101), (2, 102), (7, 107), (30, 130), (365, 465)] {
            let next = sitting_at(&ledger, today);
            assert!(next.new);
            assert_eq!(next.number, 2);
            assert_eq!(next.allowance, SITTING_DAYS, "{away} days away");
            assert_eq!(next.left, 1);
            assert_eq!(next.days_away, away);
        }
        // A month away is one day.
        let (month, after) = begin(&ledger, 130);
        assert_eq!(month.left, 1);
        let after = after.spent(130);
        assert!(sitting_at(&after, 130).spent());
        // And the next day, one more.
        assert_eq!(sitting_at(&after, 131).left, 1);
    }

    #[test]
    fn opening_again_the_same_day_goes_on_with_the_same_sitting() {
        let (_, ledger) = begin(&SittingLedger::default(), 100);
        let (_, ledger) = begin(&ledger.spent(100), 101);
        let again = sitting_at(&ledger, 101);
        assert!(!again.new);
        assert_eq!((again.number, again.left), (2, 1));
        let ledger = ledger.spent(101);
        let again = sitting_at(&ledger, 101);
        assert_eq!((again.number, again.left), (2, 0));
        // Spending in a sitting that is not today's changes nothing.
        assert_eq!(ledger.spent(102), ledger);
    }

    #[test]
    fn a_clock_turned_back_begins_a_sitting_of_one_day() {
        let (_, ledger) = begin(&SittingLedger::default(), 100);
        let (_, ledger) = begin(&ledger.spent(100).spent(100).spent(100), 105);
        let ledger = ledger.spent(105);
        assert!(sitting_at(&ledger, 105).spent());
        // A clock that was once ahead (the 105th) and is right again: a
        // new sitting of one day, never locked asleep until the 105th,
        // and never the first sitting's days.
        for today in [104, 50, -3] {
            let back = sitting_at(&ledger, today);
            assert!(back.new, "{today}");
            assert_eq!((back.left, back.allowance), (SITTING_DAYS, SITTING_DAYS));
            assert_eq!(back.days_away, 0);
        }
        // Begun on the 104th, the same day goes on, and the 105th, when it
        // comes, is another.
        let (_, ledger) = begin(&ledger, 104);
        assert_eq!(ledger.began_on, Some(104));
        assert!(!sitting_at(&ledger, 104).new);
        assert!(sitting_at(&ledger, 105).new);
    }

    #[test]
    fn a_ledger_that_cannot_say_when_has_a_later_sitting() {
        let before = SittingLedger {
            sittings: 1,
            ..SittingLedger::default()
        };
        let sitting = sitting_at(&before, 300);
        assert!(sitting.new);
        assert_eq!((sitting.number, sitting.left), (2, SITTING_DAYS));
    }

    #[test]
    fn the_counts_never_overflow() {
        let full = SittingLedger {
            sittings: u64::MAX,
            began_on: Some(i64::MIN),
            days_passed: u32::MAX,
            morning_read: None,
        };
        let sitting = sitting_at(&full, i64::MAX);
        assert_eq!(sitting.number, u64::MAX);
        assert_eq!(sitting.days_away, i64::MAX as u64);
        let same = SittingLedger {
            began_on: Some(7),
            ..full
        };
        assert_eq!(same.spent(7).days_passed, u32::MAX);
    }

    #[test]
    fn a_sitting_is_worked_out_from_the_ledger_alone() {
        // The same ledger and day give the same sitting, every time.
        let ledger = SittingLedger {
            sittings: 4,
            began_on: Some(200),
            days_passed: 1,
            morning_read: None,
        };
        assert_eq!(sitting_at(&ledger, 200), sitting_at(&ledger, 200));
        assert_eq!(sitting_at(&ledger, 203), sitting_at(&ledger, 203));
        // A ledger that says no sittings is a first sitting, whatever day
        // it names.
        let odd = SittingLedger {
            sittings: 0,
            began_on: Some(10),
            days_passed: 2,
            morning_read: None,
        };
        assert_eq!(sitting_at(&odd, 10).number, 1);
        assert_eq!(sitting_at(&odd, 10).left, FIRST_SITTING_DAYS);
    }

    #[test]
    fn a_ledger_sits_spends_and_reads_its_morning_in_place() {
        let mut ledger = SittingLedger::default();
        let first = ledger.sit(100);
        assert!(first.new);
        assert_eq!(ledger.sittings, 1);
        assert_eq!(ledger.began_on, Some(100));
        // The same day again goes on with it, and changes nothing.
        let again = ledger.clone();
        assert!(!ledger.sit(100).new);
        assert_eq!(ledger, again);
        assert_eq!(ledger.spend_day(100).left, FIRST_SITTING_DAYS - 1);
        assert_eq!(ledger.days_passed, 1);
        // Spending in a sitting that is not today's changes nothing.
        assert_eq!(ledger.spend_day(101).left, SITTING_DAYS);
        assert_eq!(ledger.days_passed, 1);
        // The morning is kept for its own sitting only, once.
        assert!(!ledger.read_morning(99, "event-1"));
        assert!(ledger.read_morning(100, "event-1"));
        assert!(!ledger.read_morning(100, "event-1"));
        assert!(!ledger.read_morning(100, "two\nlines"));
        assert_eq!(ledger.morning_read_on(100), Some("event-1"));
        assert_eq!(ledger.morning_read_on(101), None);
        // The next day: a new sitting of one day, its morning unread.
        let next = ledger.sit(101);
        assert!(next.new);
        assert_eq!((next.number, next.left), (2, SITTING_DAYS));
        assert_eq!(ledger.morning_read_on(101), None);
    }

    #[test]
    fn local_days_turn_at_local_midnight() {
        // A midnight, UTC.
        let midnight = 1_760_486_400;
        assert_eq!(local_day(midnight, 0), local_day(midnight + 86_399, 0));
        assert_eq!(local_day(midnight, 0) + 1, local_day(midnight + 86_400, 0));
        // Ten at night in New York (UTC-4) is still the 14th there.
        assert_eq!(
            local_day(midnight + 2 * 3600, -4 * 3600),
            local_day(midnight, 0) - 1
        );
        // Before 1970 counts back, not toward zero.
        assert_eq!(local_day(-1, 0), -1);
    }
}
