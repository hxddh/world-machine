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

/// The choice that lets a World's time pass, as its Pack says with the
/// command's role ("Let the day pass" in Tiny Society). Never guessed from
/// an id.
pub fn day_pass(
    snapshot: &world_projection::ProjectionSnapshot,
) -> Option<&world_projection::ProjectionCommand> {
    snapshot
        .commands
        .iter()
        .find(|command| command.role == Some(world_projection::CommandRole::PassesTime))
}

/// Whether choosing `command_id` in `snapshot` lets time pass: its Pack
/// says so, or the World does not offer it at all, which the demo takes to
/// move time too (it fails closed).
pub fn passes_time(snapshot: &world_projection::ProjectionSnapshot, command_id: &str) -> bool {
    snapshot
        .commands
        .iter()
        .find(|command| command.id == command_id)
        .is_none_or(|command| command.role == Some(world_projection::CommandRole::PassesTime))
}

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

/// What the demo does with a choice in a World of `pack_id` on `day`
/// (`None` when its days cannot be counted), when the choice lets time pass
/// or not ([`passes_time`]). It fails closed: a Pack the demo does not
/// offer is never played, and a day it cannot count is taken to be past the
/// last.
pub fn gate(pack_id: &str, day: Option<u32>, passes_time: bool) -> Gate {
    if !offers_pack(pack_id) {
        return Gate::NotOffered;
    }
    if passes_time && day.is_none_or(|day| day >= LAST_DAY) {
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

/// The player's own deeds to recap, from what the World recorded and its
/// Pack tells of them, in its own words (the window shows them in the
/// player's language), each once and at most [`RECAP_MOST`]:
/// - what the history says the player did ("You built a bench by the
///   harbour", "You did Hana a favour"), oldest first;
/// - what came of it ("The swallows came back to the harbour, around the
///   bench you made", "Rosa came because of the pottery you built");
/// - what people shared with them for it ("Sofia showed you the pub's
///   garden swing"), two at most;
/// - and only if that is too little, who gave them something to keep
///   ("Jonas gave you a smooth stone.").
///
/// Never the weather, a season, a favour merely asked, or a note torn from
/// what it came with.
pub fn recap(snapshot: &world_gpui::ProjectionSnapshot) -> Vec<String> {
    let mut items = snapshot
        .timeline
        .items
        .iter()
        .filter(|item| !item.routine)
        .collect::<Vec<_>>();
    items.sort_by_key(|item| item.world_time);
    let titles = items
        .iter()
        .map(|item| item.title.as_str())
        .collect::<Vec<_>>();
    let chapters = snapshot
        .chapters
        .iter()
        .flat_map(|chapter| sentences(&chapter.summary))
        .collect::<Vec<_>>();
    let mut lines = Vec::<String>::new();
    let add = |line: &str, lines: &mut Vec<String>| {
        let line = closed(line.trim());
        if !line.is_empty() && !lines.contains(&line) {
            lines.push(line);
        }
    };
    // What the player did: the most recent, if there are too many.
    let deeds = titles
        .iter()
        .chain(&chapters)
        .filter(|line| told_of_the_player(line))
        .copied()
        .collect::<Vec<_>>();
    // Told in a few ways, not five "You began…" in a row: at most two
    // deeds that open the same way.
    let opening = |line: &str| {
        line.split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut done = Vec::new();
    for deed in deeds.iter().rev() {
        let alike = done
            .iter()
            .filter(|line: &&String| opening(line) == opening(deed))
            .count();
        if alike < 2 {
            add(deed, &mut done);
        }
    }
    done.truncate(RECAP_MOST - 1);
    done.reverse();
    lines.extend(done);
    // What came of it.
    for line in titles.iter().chain(&chapters) {
        if lines.len() < RECAP_MOST && came_of_the_player(line) {
            add(line, &mut lines);
        }
    }
    // What people shared with them.
    let mut shared = 0;
    for line in &titles {
        if lines.len() < RECAP_MOST && shared < 2 && shared_with_the_player(line) {
            let before = lines.len();
            add(line, &mut lines);
            shared += lines.len() - before;
        }
    }
    if lines.len() < RECAP_LEAST {
        for keepsake in &snapshot.keepsakes {
            let Some(from) = name_of(snapshot, keepsake.from) else {
                continue;
            };
            if lines.len() < RECAP_LEAST && !keepsake.what.trim().is_empty() {
                add(
                    &format!("{from} gave you {}", keepsake.what.trim()),
                    &mut lines,
                );
            }
        }
    }
    lines.truncate(RECAP_MOST);
    lines
}

/// Whether a line tells something the player did: told to them, as
/// "You …".
fn told_of_the_player(line: &str) -> bool {
    let line = line.trim();
    line.starts_with("You ") && !line.starts_with("You were") && line.len() > 8
}

/// Whether a line tells what came of something the player did: "around
/// the bench you made", "because of the pottery you built".
fn came_of_the_player(line: &str) -> bool {
    !told_of_the_player(line)
        && [
            "you made",
            "you built",
            "you planted",
            "you chose",
            "you put",
            "your ",
        ]
        .iter()
        .any(|deed| line.contains(deed))
}

/// Whether a line tells what someone shared with the player: "Sofia
/// showed you…", "Jonas told you something…"; never a favour asked, or
/// something almost said.
fn shared_with_the_player(line: &str) -> bool {
    !told_of_the_player(line)
        && !came_of_the_player(line)
        && mentions_the_player(line)
        && !line.contains("asked you")
        && !line.contains("almost")
}

/// Whether a sentence speaks of the player.
fn mentions_the_player(sentence: &str) -> bool {
    sentence
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .any(|word| matches!(word, "you" | "You" | "your" | "Your"))
}

/// Someone's first name, as the scene labels them.
fn name_of(
    snapshot: &world_gpui::ProjectionSnapshot,
    who: world_gpui::SelectionId,
) -> Option<String> {
    snapshot
        .canvas
        .items
        .iter()
        .find(|item| item.id == who)
        .and_then(|item| item.label.split_whitespace().next())
        .map(str::to_string)
}

/// A line closed with a full stop, if it has no closing mark of its own.
fn closed(line: &str) -> String {
    if line.is_empty() || line.ends_with(['.', '!', '?', '。', '！', '？']) {
        line.to_string()
    } else {
        format!("{line}.")
    }
}

/// The sentences of `text`.
fn sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars = text.char_indices().collect::<Vec<_>>();
    for (n, (index, ch)) in chars.iter().enumerate() {
        let next = chars.get(n + 1).map(|(_, next)| *next);
        if matches!(ch, '.' | '!' | '?' | '。' | '！' | '？')
            && next.is_none_or(char::is_whitespace)
        {
            let end = index + ch.len_utf8();
            let sentence = text[start..end].trim();
            if !sentence.is_empty() {
                out.push(sentence);
            }
            start = end;
        }
    }
    let rest = text[start..].trim();
    if !rest.is_empty() {
        out.push(rest);
    }
    out
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
            assert_eq!(gate(PACK_ID, Some(day), true), Gate::Open, "{day}");
        }
        assert_eq!(gate(PACK_ID, Some(LAST_DAY), true), Gate::Ending);
        assert_eq!(gate(PACK_ID, Some(LAST_DAY + 3), true), Gate::Ending);
        // Everything else on the last day is the full app's.
        assert_eq!(gate(PACK_ID, Some(LAST_DAY), false), Gate::Open);
    }

    /// The v0.27 bar (M7): the gate uses `offers_pack` and fails closed.
    #[test]
    fn the_gate_fails_closed() {
        // A Pack the demo does not offer is never played, whatever is asked.
        for passes_time in [true, false] {
            assert_eq!(
                gate("world-machine.pocket-universe", Some(1), passes_time),
                Gate::NotOffered
            );
            assert_eq!(gate("", None, passes_time), Gate::NotOffered);
        }
        // A day that cannot be counted is past the last.
        assert_eq!(gate(PACK_ID, None, true), Gate::Ending);
        assert_eq!(gate(PACK_ID, None, false), Gate::Open);
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
    fn a_recap_is_the_players_own_deeds_each_once() {
        use world_projection::{
            CanvasItem, CanvasItemKind, Chapter, Keepsake, Moment, SelectionId, TimelineItem,
        };
        let jonas = SelectionId::from_stable_key("entity-1").unwrap();
        let item = |at: u64, title: &str| TimelineItem {
            id: SelectionId::from_stable_key(&format!("event-{at}")).unwrap(),
            world_time: at,
            title: title.into(),
            subtitle: String::new(),
            caused_by: Vec::new(),
            routine: false,
        };
        let mut snapshot = world_gpui::ProjectionSnapshot {
            chapters: vec![Chapter {
                number: 1,
                title: "A shared hard season".into(),
                summary: "The harbour held. Rosa, a potter from a town with no sea, came because of the pottery you built. Everyone ate.".into(),
                moment: None,
            }],
            ..Default::default()
        };
        snapshot.timeline.items = vec![
            item(30, "You built a bench"),
            item(10, "You planted wildflowers by Anchor Pub"),
            item(20, "The swallows came back"),
            item(40, "You did Hana a favour"),
            item(50, "You did Hana a favour"),
        ];
        for title in ["The swallows", "The storm"] {
            snapshot.moments.push(Moment {
                title: title.into(),
                ..Default::default()
            });
        }
        snapshot.canvas.items.push(CanvasItem {
            id: jonas,
            label: "Jonas Reed".into(),
            kind: CanvasItemKind::Actor,
            ..Default::default()
        });
        snapshot.keepsakes.push(Keepsake {
            from: jonas,
            what: "a smooth stone".into(),
            note: "For your first day in the harbour. Welcome.".into(),
            moment: jonas,
        });
        assert_eq!(
            recap(&snapshot),
            [
                "You planted wildflowers by Anchor Pub.",
                "You built a bench.",
                "You did Hana a favour.",
                "Rosa, a potter from a town with no sea, came because of the pottery you built.",
            ],
            "the player's deeds, oldest first, then what came of them; never \
             the weather, a season or a note on its own"
        );
        // Too few deeds, and who gave the player something says it.
        let mut quiet = snapshot.clone();
        quiet.timeline.items.retain(|item| item.world_time == 10);
        assert_eq!(
            recap(&quiet),
            [
                "You planted wildflowers by Anchor Pub.",
                "Rosa, a potter from a town with no sea, came because of the pottery you built.",
                "Jonas gave you a smooth stone.",
            ]
        );
        // With deeds enough, nothing given is needed.
        snapshot
            .timeline
            .items
            .push(item(60, "You lent a hand with the pier"));
        let lines = recap(&snapshot);
        assert!(lines.len() <= RECAP_MOST, "{lines:?}");
        // Five things begun are not five lines that open alike.
        let mut busy = snapshot.clone();
        for (at, what) in [
            (61, "a bench"),
            (62, "a lamp"),
            (63, "a well"),
            (64, "a swing"),
            (65, "a flowerbed"),
        ] {
            busy.timeline
                .items
                .push(item(at, &format!("You began {what}")));
        }
        let lines = recap(&busy);
        let began = lines
            .iter()
            .filter(|line| line.starts_with("You began"))
            .count();
        assert!(began <= 2, "{lines:?}");
        assert!(
            !lines.iter().any(|line| line.contains("gave you")),
            "{lines:?}"
        );
    }
}
