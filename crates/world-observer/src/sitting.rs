//! Sittings (v0.30 S4, "one day a day"): a World moves one of its days a
//! sitting, and a sitting is a day of the player's own, counted on this
//! device's calendar. How many World days a sitting may pass is worked out
//! here, as a pure function of the device's ledger and the day the sitting
//! begins on.
//!
//! Wall-clock time lives only in this ledger, on this device. It never
//! enters a World's record: the World is only ever told that a sitting
//! began, and it is the player who lets each of its days pass.

/// How many World days a World's very first sitting may pass: enough for
/// the first evening to show what the place is.
pub const FIRST_SITTING_DAYS: u32 = 3;

/// How many World days any later sitting may pass, however long the player
/// was away: a month away is one day, never thirty.
pub const SITTING_DAYS: u32 = 1;

pub const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

/// The local day (days since 1970-01-01 on this device's calendar) that
/// `unix_seconds` falls on, `utc_offset_seconds` east of UTC.
pub fn local_day(unix_seconds: i64, utc_offset_seconds: i64) -> i64 {
    (unix_seconds + utc_offset_seconds).div_euclid(SECONDS_PER_DAY)
}

/// A World's sittings on this device, as its ledger keeps them. Kept by the
/// observer beside the Library, never in the World.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SittingLedger {
    /// How many sittings there have been.
    pub sittings: u64,
    /// The local day the latest sitting began on.
    pub began_on: Option<i64>,
    /// How many World days the latest sitting has passed.
    pub days_passed: u32,
}

impl SittingLedger {
    /// The ledger with one more World day passed in the sitting begun on
    /// `today`: unchanged for any other day's sitting, and never past what
    /// that sitting may pass.
    pub fn spent(&self, today: i64) -> SittingLedger {
        let sitting = sitting_at(self, today);
        if sitting.new || sitting.left == 0 {
            return *self;
        }
        SittingLedger {
            days_passed: self.days_passed + 1,
            ..*self
        }
    }
}

/// A sitting: which one it is, whether it begins now, and how many World
/// days it may pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Sitting {
    /// 1 for the first sitting with this World on this device.
    pub number: u64,
    /// Whether this sitting begins now, rather than going on with one
    /// begun earlier the same day.
    pub new: bool,
    /// How many World days the sitting may pass in all.
    pub allowance: u32,
    /// How many of them are still to pass.
    pub left: u32,
    /// How many of the player's days since the last sitting began: 0 for
    /// the first, or for one that goes on.
    pub days_away: u64,
}

impl Sitting {
    /// The ledger once this sitting has begun on `today`, from the one it
    /// was worked out from.
    pub fn begun(&self, before: &SittingLedger, today: i64) -> SittingLedger {
        if !self.new {
            return *before;
        }
        SittingLedger {
            sittings: self.number,
            began_on: Some(today),
            days_passed: 0,
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

/// The sitting that begins (or goes on) when the World is opened on local
/// day `today`, from its ledger. A new day of the player's is a new
/// sitting, with one World day to pass, however many days they were away;
/// the same day again goes on with the sitting already begun, with what it
/// has left. A clock that went back counts as the same day: no sitting is
/// ever won by turning the clock.
pub fn sitting_at(ledger: &SittingLedger, today: i64) -> Sitting {
    match ledger.began_on {
        Some(began) if ledger.sittings > 0 && today <= began => {
            let allowance = allowance(ledger.sittings);
            Sitting {
                number: ledger.sittings,
                new: false,
                allowance,
                left: allowance.saturating_sub(ledger.days_passed),
                days_away: 0,
            }
        }
        Some(began) if ledger.sittings > 0 => {
            let number = ledger.sittings + 1;
            let allowance = allowance(number);
            Sitting {
                number,
                new: true,
                allowance,
                left: allowance,
                days_away: (today - began) as u64,
            }
        }
        _ => Sitting {
            number: 1,
            new: true,
            allowance: FIRST_SITTING_DAYS,
            left: FIRST_SITTING_DAYS,
            days_away: 0,
        },
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
    fn the_first_sitting_may_pass_three_days() {
        let (first, ledger) = begin(&SittingLedger::default(), 100);
        assert_eq!(
            first,
            Sitting {
                number: 1,
                new: true,
                allowance: 3,
                left: 3,
                days_away: 0
            }
        );
        let ledger = ledger.spent(100).spent(100);
        assert_eq!(sitting_at(&ledger, 100).left, 1);
        let ledger = ledger.spent(100).spent(100).spent(100);
        let spent = sitting_at(&ledger, 100);
        assert_eq!((spent.left, spent.new), (0, false));
        assert!(spent.spent());
        assert_eq!(ledger.days_passed, 3, "never past the allowance");
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
    fn a_clock_turned_back_wins_no_sitting() {
        let (_, ledger) = begin(&SittingLedger::default(), 100);
        let (_, ledger) = begin(&ledger.spent(100).spent(100).spent(100), 105);
        let ledger = ledger.spent(105);
        for today in [104, 50, -3] {
            let back = sitting_at(&ledger, today);
            assert!(!back.new, "{today}");
            assert_eq!(back.left, 0);
        }
    }

    #[test]
    fn a_sitting_is_worked_out_from_the_ledger_alone() {
        // The same ledger and day give the same sitting, every time.
        let ledger = SittingLedger {
            sittings: 4,
            began_on: Some(200),
            days_passed: 1,
        };
        assert_eq!(sitting_at(&ledger, 200), sitting_at(&ledger, 200));
        assert_eq!(sitting_at(&ledger, 203), sitting_at(&ledger, 203));
        // A ledger that says no sittings is a first sitting, whatever day
        // it names.
        let odd = SittingLedger {
            sittings: 0,
            began_on: Some(10),
            days_passed: 2,
        };
        assert_eq!(sitting_at(&odd, 10).number, 1);
        assert_eq!(sitting_at(&odd, 10).left, FIRST_SITTING_DAYS);
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
