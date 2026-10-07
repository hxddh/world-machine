//! The free demo: the first hour of Tiny Society, then a warm ending.
//!
//! A demo build (`--features demo`) is the whole app with one gate: in a
//! Tiny Society World, the day after [`LAST_DAY`] does not come. Asking
//! for it shows the farewell instead (the town at dusk, a recap of what
//! the player did there from the World's own record, a resident's
//! goodbye and a postcard to keep), and time away never carries the World
//! past it. The demo offers Tiny Society alone ([`offers_pack`]); the gate
//! fails closed, so a World of any other Pack, or one whose days cannot
//! be counted, never moves in the demo. Everything else is the full app's:
//! the World is an ordinary World in the ordinary Library, so the full
//! app opens it and it carries on from where the demo left it. Nothing is
//! counted, timed out or asked for.
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
    /// farewell instead, and leave the World as it is.
    Ending,
    /// A World the demo does not offer: nothing it is asked moves it.
    NotOffered,
}

/// What the demo does with `command` in a World of `pack_id` on `day`
/// (`None` when its days cannot be counted). It fails closed: a Pack the
/// demo does not offer is never played, and a day it cannot count is
/// taken to be past the last.
pub fn gate(pack_id: &str, day: Option<u32>, command: &str) -> Gate {
    if !offers_pack(pack_id) {
        return Gate::NotOffered;
    }
    if command == DAY_PASS_COMMAND && day.is_none_or(|day| day >= LAST_DAY) {
        return Gate::Ending;
    }
    Gate::Open
}

/// How many of `periods` of time away a World on `day` may live through in
/// the demo: never past its last day, and none at all for a Pack it does
/// not offer or a day it cannot count.
pub fn periods_to_catch_up(pack_id: &str, day: Option<u32>, periods: u64) -> u64 {
    match day {
        Some(day) if offers_pack(pack_id) => periods.min(u64::from(LAST_DAY.saturating_sub(day))),
        _ => 0,
    }
}

/// What Home is called: the demo says it is one.
pub const HOME_TITLE: &str = if ENABLED {
    "World Machine Demo"
} else {
    "World Machine"
};

/// What a refused choice in a World the demo does not offer says.
pub const NOT_OFFERED: &str = "The demo plays Tiny Society. This World opens in the full app.";

/// The hour the farewell holds the sky at: dusk, which is what its words
/// say ("this evening"), while some of the harbour is still out.
pub const FAREWELL_HOUR: u32 = 18;

/// What the farewell recaps at least, and at most.
pub const RECAP_LEAST: usize = 4;
pub const RECAP_MOST: usize = 6;

/// The player's own moments to recap, from what the World recorded and
/// its Pack tells of it: how its chapters ended ("Rosa came because of the
/// pottery you built"), the moments that made its book, what was made and
/// who gave the player something to keep, oldest first, each once; at
/// most [`RECAP_MOST`].
pub fn recap(snapshot: &world_gpui::ProjectionSnapshot) -> Vec<String> {
    let mut lines = Vec::<String>::new();
    let mut add = |line: &str| {
        let line = line.trim();
        if !line.is_empty() && !lines.iter().any(|kept| kept == line) {
            lines.push(line.to_string());
        }
    };
    // First what the chapters say the player did, then the moments, then
    // what they were given: the most telling first, so a short World still
    // keeps its best lines.
    let mut chosen = Vec::new();
    for chapter in &snapshot.chapters {
        if let Some(first) = first_sentences(&chapter.summary, 2) {
            chosen.push(first);
        }
    }
    for moment in &snapshot.moments {
        chosen.push(moment.title.clone());
    }
    for keepsake in &snapshot.keepsakes {
        chosen.push(keepsake.note.clone());
    }
    for entry in snapshot
        .book
        .iter()
        .filter(|entry| entry.found && entry.moment.is_none())
    {
        chosen.push(entry.name.clone());
    }
    for line in &chosen {
        add(line);
    }
    lines.truncate(RECAP_MOST);
    lines
}

/// The first `count` sentences of `text`, if it has any.
fn first_sentences(text: &str, count: usize) -> Option<String> {
    let mut end = 0;
    let mut found = 0;
    for (index, ch) in text.char_indices() {
        if matches!(ch, '.' | '!' | '?' | '。' | '！' | '？') {
            found += 1;
            end = index + ch.len_utf8();
            if found == count {
                break;
            }
        }
    }
    let text = if found == 0 { text } else { &text[..end] };
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Who says goodbye: whoever thinks best of the player, the first of
/// equals; anyone living there if nobody has a word on it yet.
pub fn goodbye_from(snapshot: &world_gpui::ProjectionSnapshot) -> Option<world_gpui::SelectionId> {
    use world_projection::CanvasItemKind;
    snapshot
        .canvas
        .items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .enumerate()
        .max_by_key(|(index, item)| {
            (
                item.standing
                    .as_ref()
                    .map_or(i64::MIN, |standing| i64::from(standing.level)),
                std::cmp::Reverse(*index),
            )
        })
        .map(|(_, item)| item.id)
}

/// What they say.
pub const GOODBYE: &str = "Come back when you can. We'll keep the lamps lit.";

/// The farewell's other words, in English; the window shows them through
/// the app's catalogs.
pub const RECAP_TITLE: &str = "What you did here";
pub const KEEP_POSTCARD: &str = "Keep a postcard";

/// The farewell's title and words, in English; the window shows them
/// through the app's catalogs. The body says "this evening": the farewell
/// holds the sky at dusk ([`FAREWELL_HOUR`]).
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
            assert_eq!(
                gate(PACK_ID, Some(day), DAY_PASS_COMMAND),
                Gate::Open,
                "{day}"
            );
        }
        assert_eq!(
            gate(PACK_ID, Some(LAST_DAY), DAY_PASS_COMMAND),
            Gate::Ending
        );
        assert_eq!(
            gate(PACK_ID, Some(LAST_DAY + 3), DAY_PASS_COMMAND),
            Gate::Ending
        );
        // Everything else on the last day is the full app's.
        assert_eq!(
            gate(PACK_ID, Some(LAST_DAY), "tiny-society.build-bench"),
            Gate::Open
        );
    }

    /// The v0.27 bar (M7): the gate uses `offers_pack` and fails closed.
    #[test]
    fn the_gate_fails_closed() {
        // A Pack the demo does not offer is never played, whatever is asked.
        for command in [DAY_PASS_COMMAND, "pocket-universe.nudge", "anything"] {
            assert_eq!(
                gate("world-machine.pocket-universe", Some(1), command),
                Gate::NotOffered
            );
            assert_eq!(gate("", None, command), Gate::NotOffered);
        }
        // A day that cannot be counted is past the last.
        assert_eq!(gate(PACK_ID, None, DAY_PASS_COMMAND), Gate::Ending);
        assert_eq!(gate(PACK_ID, None, "tiny-society.build-bench"), Gate::Open);
        // And time away moves neither.
        assert_eq!(
            periods_to_catch_up("world-machine.pocket-universe", Some(1), 9),
            0
        );
        assert_eq!(periods_to_catch_up(PACK_ID, None, 9), 0);
    }

    #[test]
    fn time_away_stops_at_the_last_day() {
        assert_eq!(
            periods_to_catch_up(PACK_ID, Some(1), 28),
            u64::from(LAST_DAY - 1)
        );
        assert_eq!(periods_to_catch_up(PACK_ID, Some(LAST_DAY - 1), 4), 1);
        assert_eq!(periods_to_catch_up(PACK_ID, Some(LAST_DAY), 4), 0);
        assert_eq!(periods_to_catch_up(PACK_ID, Some(LAST_DAY + 2), 4), 0);
        assert_eq!(periods_to_catch_up(PACK_ID, Some(2), 1), 1);
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
            RECAP_TITLE,
            KEEP_POSTCARD,
            GOODBYE,
            NOT_OFFERED,
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

    /// The words say "this evening", and the farewell holds the sky at
    /// dusk while they are shown.
    #[test]
    fn the_time_of_day_the_words_say_is_the_skys() {
        assert!(ENDING_BODY.contains("this evening"));
        assert_eq!(
            world_gpui::scene::daylight_at(FAREWELL_HOUR),
            world_gpui::scene::Daylight::Dusk
        );
    }

    #[test]
    fn a_demo_build_says_so() {
        assert_eq!(ENABLED, cfg!(feature = "demo"));
        assert_eq!(HOME_TITLE.ends_with("Demo"), ENABLED);
    }

    #[test]
    fn a_recap_is_the_players_own_lines_each_once() {
        use world_projection::{Chapter, Keepsake, SelectionId};
        let someone = SelectionId::from_stable_key("entity-2").unwrap();
        let mut snapshot = world_gpui::ProjectionSnapshot {
            chapters: vec![Chapter {
                number: 1,
                title: "A shared hard season".into(),
                summary: "Rosa, a potter from a town with no sea, came because of the pottery you built. The harbour held. Everyone ate.".into(),
                moment: None,
            }],
            ..Default::default()
        };
        for note in [
            "A map, so you never get lost.",
            "A map, so you never get lost.",
        ] {
            snapshot.keepsakes.push(Keepsake {
                from: someone,
                what: "a map".into(),
                note: note.into(),
                moment: someone,
            });
        }
        let lines = recap(&snapshot);
        assert_eq!(
            lines,
            [
                "Rosa, a potter from a town with no sea, came because of the pottery you built. The harbour held.",
                "A map, so you never get lost.",
            ]
        );
    }
}
