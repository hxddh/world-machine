//! The free demo: the first hour of Tiny Society, then a warm ending.
//!
//! A demo build (`--features demo`) is the whole app with one gate: in a
//! Tiny Society World, the day after [`LAST_DAY`] does not come. Asking
//! for it shows the ending card instead, and time away never carries the
//! World past it. Everything else is the full app's: the World is an
//! ordinary World in the ordinary Library, so the full app opens it and it
//! carries on from where the demo left it. Nothing is counted, timed out
//! or asked for.
//!
//! Everything here is plain logic so it can be tested on any platform;
//! the window only asks it.

/// Whether this build is the demo.
pub const ENABLED: bool = cfg!(feature = "demo");

/// The one World the demo holds.
pub const PACK_ID: &str = "world-machine.tiny-society";

/// Tiny Society's "Let the day pass", the one choice that moves its time.
pub const DAY_PASS_COMMAND: &str = "tiny-society.let-day-pass";

/// The last day of the demo. A newcomer's first hour, played on the
/// first-session clock (`tests/demo_first_hour.rs`), ends on day 10 for a
/// lingering player, day 14 for a steady one and day 18 for a brisk one.
/// Tiny Society closes a chapter every ten days, as days 11, 21 and 31
/// begin, so the demo runs to day 21: every pace gets its whole first hour
/// (a steady player about an hour and a half), and the last evening comes
/// just after the second chapter's ending card.
pub const LAST_DAY: u32 = 21;

/// The day a World is on, counted the way the World counts it: day 1 until
/// the first `day_length` of World time has passed.
pub fn day_of(world_time: u64, day_length: u64) -> u32 {
    let day = world_time.div_ceil(day_length.max(1)).max(1);
    u32::try_from(day).unwrap_or(u32::MAX)
}

/// Whether the demo ships and offers this Pack.
pub fn offers_pack(pack_id: &str) -> bool {
    pack_id == PACK_ID
}

/// What the demo does with a choice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Gate {
    /// The full app's behaviour.
    Open,
    /// The choice would carry the World past the demo's last day: show the
    /// ending card instead, and leave the World as it is.
    Ending,
}

/// What the demo does with `command` in a World of `pack_id` on `day`.
pub fn gate(pack_id: &str, day: u32, command: &str) -> Gate {
    if pack_id == PACK_ID && command == DAY_PASS_COMMAND && day >= LAST_DAY {
        Gate::Ending
    } else {
        Gate::Open
    }
}

/// How many of `periods` of time away a World on `day` may live through in
/// the demo: never past its last day.
pub fn periods_to_catch_up(pack_id: &str, day: u32, periods: u64) -> u64 {
    if pack_id != PACK_ID {
        return periods;
    }
    periods.min(u64::from(LAST_DAY.saturating_sub(day)))
}

/// The ending card's words, in English; the window shows them through the
/// app's catalogs.
pub const ENDING_TITLE: &str = "This is where the demo ends";
pub const ENDING_BODY: &str = "The harbour keeps everything you did here. Open it in the full World Machine and it carries on from this evening, with the same people and all they remember.";
pub const ENDING_KEPT: &str =
    "Your World is kept in My Worlds. Nothing is deleted, and nothing more is asked of you.";
pub const ENDING_STAY: &str = "Stay a while";
pub const ENDING_FULL_APP: &str = "About the full app";

/// Where "About the full app" leads.
pub const FULL_APP_URL: &str = "https://github.com/hxddh/world-machine/releases";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_are_counted_as_the_world_counts_them() {
        assert_eq!(day_of(0, 100), 1);
        assert_eq!(day_of(100, 100), 1);
        assert_eq!(day_of(101, 100), 2);
        assert_eq!(day_of(0, 0), 1);
    }

    #[test]
    fn only_the_day_after_the_last_is_held() {
        for day in 1..LAST_DAY {
            assert_eq!(gate(PACK_ID, day, DAY_PASS_COMMAND), Gate::Open, "{day}");
        }
        assert_eq!(gate(PACK_ID, LAST_DAY, DAY_PASS_COMMAND), Gate::Ending);
        assert_eq!(gate(PACK_ID, LAST_DAY + 3, DAY_PASS_COMMAND), Gate::Ending);
        // Everything else on the last day is the full app's.
        assert_eq!(
            gate(PACK_ID, LAST_DAY, "tiny-society.build-bench"),
            Gate::Open
        );
        assert_eq!(
            gate("world-machine.pocket-universe", LAST_DAY, DAY_PASS_COMMAND),
            Gate::Open
        );
    }

    #[test]
    fn time_away_stops_at_the_last_day() {
        assert_eq!(periods_to_catch_up(PACK_ID, 1, 28), u64::from(LAST_DAY - 1));
        assert_eq!(periods_to_catch_up(PACK_ID, LAST_DAY - 1, 4), 1);
        assert_eq!(periods_to_catch_up(PACK_ID, LAST_DAY, 4), 0);
        assert_eq!(periods_to_catch_up(PACK_ID, LAST_DAY + 2, 4), 0);
        assert_eq!(periods_to_catch_up(PACK_ID, 2, 1), 1);
        assert_eq!(periods_to_catch_up("other", LAST_DAY, 4), 4);
    }

    #[test]
    fn the_demo_offers_its_one_pack_alone() {
        assert!(offers_pack(PACK_ID));
        assert!(!offers_pack("world-machine.pocket-universe"));
    }

    #[test]
    fn the_ending_is_warm_and_asks_for_nothing() {
        let words = [
            ENDING_TITLE,
            ENDING_BODY,
            ENDING_KEPT,
            ENDING_STAY,
            ENDING_FULL_APP,
        ]
        .join(" ")
        .to_lowercase();
        for pushy in [
            "buy", "purchase", "unlock", "expired", "trial", "upgrade", "now!",
        ] {
            assert!(!words.contains(pushy), "{pushy}");
        }
        assert!(words.contains("kept"));
        assert!(words.contains("carries on"));
    }

    #[test]
    fn a_demo_build_says_so() {
        assert_eq!(ENABLED, cfg!(feature = "demo"));
    }
}
