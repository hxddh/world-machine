//! The free demo: the opening of Tiny Society, 20 to 30 minutes of it,
//! then a warm ending.
//!
//! A demo build (`--features demo`) is the whole app with two turns of its
//! own, both read from the demo's pacing (`demo/pacing.json`, [`pacing`]):
//! - **Time away.** On the morning of [`Away::after_day`], letting the day
//!   pass lets the World live [`Away::days`] days without the player
//!   instead, through the World's own background time (the same the full
//!   app lives through while it is closed, a day for every six hours away),
//!   and the next morning opens on the return film. Nothing is made up:
//!   every day of it is the World's own events, recorded like any other.
//! - **The ending.** In a Tiny Society World, the day after
//!   [`last_day`] does not come. Asking for it shows the farewell instead
//!   (the town at dusk, a recap of what the player did there from the
//!   World's own record, a resident's goodbye, a postcard to keep and a
//!   wishlist link), and time away never carries the World past it.
//!
//! The demo offers Tiny Society alone ([`offers_pack`]); the gate fails
//! closed, so a World of any other Pack, or one whose days cannot be
//! counted, never moves in the demo. Everything else is the full app's: the
//! World is an ordinary World in the ordinary Library, so the full app
//! opens it and it carries on from where the demo left it. Nothing is
//! counted or timed out, and the voice is never needed: the demo is the
//! World's own words throughout.
//!
//! Everything here is plain logic so it can be tested on any platform;
//! the window only asks it.

use serde::Deserialize;
use std::collections::BTreeMap;

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

/// The demo's pacing, as data: when time away comes and how long it
/// lasts, the last day, the paces it is played at and when each of its
/// beats must arrive (`tests/demo_walk.rs` walks it).
#[derive(Clone, Debug, Deserialize)]
pub struct Pacing {
    pub away: Away,
    /// The last day of the demo: letting it pass shows the ending card.
    pub last_day: u32,
    pub paces: BTreeMap<String, Pace>,
    pub beats: Vec<BeatBudget>,
}

/// The demo's time away.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct Away {
    /// The day whose passing is lived away instead.
    pub after_day: u32,
    /// How many days the World lives without the player.
    pub days: u32,
}

/// How one player spends their time, for the walk.
#[derive(Clone, Debug, Deserialize)]
pub struct Pace {
    #[serde(default)]
    pub about: String,
    /// Characters read a second.
    pub chars_per_second: f32,
    /// Seconds to find and choose an answer or a deed.
    pub choose_seconds: f32,
    /// Seconds a day spent just looking at the place, the book or the
    /// drawer.
    pub look_seconds: f32,
    /// How many people they talk to a day in their own words.
    pub talks: usize,
    /// Whether they make what their hands can.
    pub makes: bool,
    /// Whether they do the favours asked of them.
    pub favours: bool,
    /// Whether they wait while what is said now is shown, page by page,
    /// before answering.
    pub waits_for_what_is_said: bool,
    /// Whether they read the morning's news each day.
    pub reads_the_news: bool,
}

/// The demo's beats, in the order they come. The first answer puts the
/// first part of a work up in scaffolding, so a build comes within the
/// first minutes; a favour is never asked in a World's first two days (the
/// conversation System's `FIRST_PERIOD`), so a day passes before it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
pub enum Beat {
    /// The town, painted, as the window opens.
    Painted,
    /// Something goes up on the scene because of the player: a work a part
    /// higher in its scaffolding, or something they made.
    Build,
    /// A day passes.
    DayPassed,
    /// Someone asks the player a favour.
    Favour,
    /// The return film, after the time away.
    ReturnFilm,
    /// The ending card, at dusk.
    Ending,
}

/// When a beat must arrive, in seconds of play at a pace.
#[derive(Clone, Debug, Deserialize)]
pub struct BeatBudget {
    pub beat: Beat,
    pub pace: String,
    #[serde(default)]
    pub from_seconds: Option<f32>,
    #[serde(default)]
    pub by_seconds: Option<f32>,
}

/// The demo's pacing, read once from `demo/pacing.json`.
pub fn pacing() -> &'static Pacing {
    static PACING: std::sync::OnceLock<Pacing> = std::sync::OnceLock::new();
    PACING.get_or_init(|| {
        serde_json::from_str(include_str!("../demo/pacing.json"))
            .expect("demo/pacing.json is the demo's pacing")
    })
}

/// The last day of the demo: letting it pass shows the ending card.
pub fn last_day() -> u32 {
    pacing().last_day
}

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
    /// The day's passing is lived away instead: the World lives this many
    /// days without the player, then the return film plays.
    Away(u32),
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
    let last = last_day();
    if passes_time && day.is_none_or(|day| day >= last) {
        return Gate::Ending;
    }
    let away = pacing().away;
    if passes_time && day == Some(away.after_day) && away.days > 0 {
        return Gate::Away(away.days.min(last - away.after_day));
    }
    Gate::Open
}

/// How many of `periods` of time away a World on `day` may live through in
/// the demo: never past its last day, and none at all for a Pack it does
/// not offer or a day it cannot count.
pub fn periods_to_catch_up(pack_id: &str, day: Option<u32>, periods: u64) -> u64 {
    match day {
        Some(day) if offers_pack(pack_id) => periods.min(u64::from(last_day().saturating_sub(day))),
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
    "Your World is kept in My Worlds, and nothing is deleted. If you'd like to come back to it, a wishlist helps the full game most.";
pub const ENDING_STAY: &str = "Stay a while";
/// The one thing the ending asks: a wishlist.
pub const ENDING_WISHLIST: &str = "Add to your wishlist";

/// Where "Add to your wishlist" leads: the store page, given at build
/// time (`WORLD_MACHINE_WISHLIST_URL`), else the project's own page until
/// there is one.
pub const WISHLIST_URL: &str = match option_env!("WORLD_MACHINE_WISHLIST_URL") {
    Some(url) if !url.is_empty() => url,
    _ => "https://github.com/hxddh/world-machine",
};

/// What the window says when the demo lets the World live days away: it
/// says the demo skipped the wait, and what the full game does instead.
pub const AWAY_NOTE: &str = "The town lived on without you. The demo skips the wait; the full game lives a day for every six hours you're away.";

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
        let last = last_day();
        let away = pacing().away;
        for day in 1..last {
            let expected = if day == away.after_day {
                Gate::Away(away.days)
            } else {
                Gate::Open
            };
            assert_eq!(gate(PACK_ID, Some(day), true), expected, "{day}");
            // Nothing but letting the day pass is turned.
            assert_eq!(gate(PACK_ID, Some(day), false), Gate::Open, "{day}");
        }
        assert_eq!(gate(PACK_ID, Some(last), true), Gate::Ending);
        assert_eq!(gate(PACK_ID, Some(last + 3), true), Gate::Ending);
        // Everything else on the last day is the full app's.
        assert_eq!(gate(PACK_ID, Some(last), false), Gate::Open);
    }

    /// The pacing is whole: time away comes before the last day and lands
    /// on it at the latest, every beat is budgeted at a pace it names, in
    /// the order the demo plays them.
    #[test]
    fn the_pacing_is_whole() {
        let pacing = pacing();
        assert!(pacing.away.days > 0);
        assert!(pacing.away.after_day + pacing.away.days <= pacing.last_day);
        let beats = pacing
            .beats
            .iter()
            .map(|budget| budget.beat)
            .collect::<Vec<_>>();
        let mut sorted = beats.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(beats, sorted, "each beat once, in order");
        assert_eq!(beats.len(), 6);
        for budget in &pacing.beats {
            assert!(pacing.paces.contains_key(&budget.pace), "{budget:?}");
        }
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
        let last = last_day();
        assert_eq!(
            periods_to_catch_up(PACK_ID, Some(1), 28),
            u64::from(last - 1)
        );
        assert_eq!(periods_to_catch_up(PACK_ID, Some(last - 1), 4), 1);
        assert_eq!(periods_to_catch_up(PACK_ID, Some(last), 4), 0);
        assert_eq!(periods_to_catch_up(PACK_ID, Some(last + 2), 4), 0);
        assert_eq!(periods_to_catch_up(PACK_ID, Some(2), 1), 1);
    }

    #[test]
    fn the_demo_offers_its_one_pack_alone() {
        assert!(offers_pack(PACK_ID));
        assert!(!offers_pack("world-machine.pocket-universe"));
    }

    #[test]
    fn the_ending_is_warm_and_asks_only_for_a_wishlist() {
        let words = [
            ENDING_TITLE,
            ENDING_BODY,
            ENDING_KEPT,
            ENDING_STAY,
            ENDING_WISHLIST,
            AWAY_NOTE,
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
        assert!(words.contains("wishlist"));
        assert!(WISHLIST_URL.starts_with("https://"), "{WISHLIST_URL}");
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
