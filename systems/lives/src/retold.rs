//! How what is told of people is retold: never the same kind of story of
//! the same person more than [`MOST_TOLD_A_YEAR`] times in a year, and,
//! where the record shows the player brought something about, saying so.
//!
//! Both are read from the record alone and decide nothing: the Events are
//! kept as they were, and only what is told of them changes.

use super::*;
use std::collections::VecDeque;

/// The most times in a year the same kind of story is told of the same
/// person.
pub const MOST_TOLD_A_YEAR: usize = 5;

/// Where someone keeps what they last did with a day, and how often this
/// year each kind of day was told of them, in one note:
/// "<activity>|<year>|<counts>", the counts a character a kind of day
/// ([`COUNTS`]: 0–9, then a–z and A–Z, so up to 61 a year), in the order
/// the Pack lists its activities, the kinds not yet lived at the end left
/// off. A World from before v0.30 keeps the activity alone.
///
/// v0.30.0 kept a note a kind (`lives.told_year.<activity>`, holding
/// "<year>:<times>") and never let a past year's go: after three years a
/// builder's place carried nearly three hundred of them in every
/// checkpoint, and its World code grew past 50,000 characters. Those are
/// still read for the year they were kept in, and let go of when the
/// person next lives a day.
pub(crate) const DID: &str = "lives.did";

/// v0.30.0's notes, one a kind of day: `<OLD_TOLD_YEAR>.<activity>`.
const OLD_TOLD_YEAR: &str = "lives.told_year";

/// A count, as [`DID`] writes it: one character each.
const COUNTS: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// What someone last did with a day: an activity's id, or nothing.
pub(crate) fn last_did(state: &WorldState, person: EntityId) -> &str {
    text(state, person, DID)
        .and_then(|did| did.split('|').next())
        .unwrap_or("")
}

/// How someone's days have gone this year: how many of each kind of day
/// the Pack lists were told of them, in its order.
pub(crate) struct Days {
    year: u64,
    counts: Vec<i64>,
    /// Whether the year's counts were read from v0.30.0's notes, which
    /// are then let go of.
    from_before: bool,
}

/// A day lived, as [`Days::lived`] tells it.
pub(crate) struct Lived {
    /// Which time this year this kind of day is told of them.
    pub nth: i64,
    /// The note that keeps it, for [`DID`].
    pub note: String,
    /// v0.30.0's notes to let go of.
    pub let_go: Vec<String>,
}

impl Days {
    /// `person`'s days in `year`, from what they keep.
    pub(crate) fn of(state: &WorldState, cast: &Cast, person: EntityId, year: u64) -> Self {
        let kept = text(state, person, DID).and_then(|did| {
            let mut parts = did.splitn(3, '|');
            parts.next()?;
            let kept_year = parts.next()?.parse::<u64>().ok()?;
            let counts = parts.next()?;
            (kept_year == year).then(|| {
                counts
                    .bytes()
                    .map(|c| COUNTS.iter().position(|d| *d == c).unwrap_or(0) as i64)
                    .collect::<Vec<_>>()
            })
        });
        // Kept this year, or kept before as v0.30.0 kept it (or not at all).
        let counted = kept.is_some();
        let counts = kept.unwrap_or_else(|| {
            cast.activities
                .iter()
                .map(|activity| {
                    text(state, person, &format!("{OLD_TOLD_YEAR}.{}", activity.id))
                        .and_then(|kept| kept.split_once(':'))
                        .filter(|(kept_year, _)| kept_year.parse::<u64>().ok() == Some(year))
                        .and_then(|(_, told)| told.parse::<i64>().ok())
                        .unwrap_or(0)
                })
                .collect()
        });
        Days {
            year,
            counts,
            from_before: !counted,
        }
    }

    /// One more day of `activity`, and the note that keeps it.
    pub(crate) fn lived(mut self, cast: &Cast, activity: &str) -> Lived {
        let most = COUNTS.len() as i64 - 1;
        let nth = match cast.activities.iter().position(|kind| kind.id == activity) {
            Some(at) => {
                if self.counts.len() <= at {
                    self.counts.resize(at + 1, 0);
                }
                self.counts[at] = (self.counts[at] + 1).min(most);
                self.counts[at]
            }
            None => 1,
        };
        self.counts.truncate(cast.activities.len());
        while self.counts.last() == Some(&0) {
            self.counts.pop();
        }
        let counts = self
            .counts
            .iter()
            .map(|count| COUNTS[(*count).clamp(0, most) as usize] as char)
            .collect::<String>();
        let let_go = if self.from_before {
            cast.activities
                .iter()
                .map(|activity| format!("{OLD_TOLD_YEAR}.{}", activity.id))
                .collect()
        } else {
            Vec::new()
        };
        Lived {
            nth,
            note: format!("{activity}|{}|{counts}", self.year),
            let_go,
        }
    }
}

/// Which time this year a lived day was, told on its Event when it was
/// lived.
pub(crate) const TOLD_NTH: &str = "told_nth";

/// How far back along what caused something a letter looks for what the
/// player did.
const CAUSE_STEPS: usize = 6;

/// A year, in world time: the Pack's calendar year when it keeps one, else
/// 365 periods.
pub fn year_length(cast: &Cast) -> u64 {
    cast.kin.map_or(365, |kin| kin.year.max(1)) * cast.period.max(1)
}

/// Which kind of story an Event tells, and of whom: a day lived by what
/// was done with it, anything else by its kind and its words. `None` for
/// what is told of nobody.
fn story_of(event: &Event) -> Option<(EntityId, &str, &str)> {
    let who = event.actor?;
    let what = match (event.kind.as_str(), event.payload.get("activity")) {
        ("lived", Some(Value::Text(activity))) => activity.as_str(),
        _ => match event.payload.get("told") {
            Some(Value::Text(told)) if !told.is_empty() => told.as_str(),
            _ => return None,
        },
    };
    Some((who, event.kind.as_str(), what))
}

/// Whether `event` would be told for the sixth time or more this year
/// (a year being `year` of world time from the World's start): the same
/// kind of story of the same person, told [`MOST_TOLD_A_YEAR`] times
/// already, is not told again until the year turns. Found through the
/// index of Events by kind, reading back from `event` only as far as the
/// year's start.
pub fn told_enough(world: &World, event: &Event, year: u64) -> bool {
    // Counted when it was lived, from what the person keeps: the same
    // whether the World kept every Event or resumed from a checkpoint.
    let counted = |event: &Event| match event.payload.get(TOLD_NTH) {
        Some(Value::Integer(nth)) => Some(*nth),
        _ => None,
    };
    let mut told = counted(event).map_or(0, |nth| nth - 1);
    let most = MOST_TOLD_A_YEAR as i64;
    if told >= most {
        return true;
    }
    // Days lived before they were counted (a World from before, mid-year)
    // are read back along the record, and add to the count.
    let Some(story) = story_of(event) else {
        return false;
    };
    let from = event.world_time - event.world_time % year.max(1);
    let index = world.history_index();
    let ids = index.of_kind(&event.kind);
    let before = &ids[..ids.partition_point(|id| *id < event.id)];
    for id in before.iter().rev() {
        let Some(earlier) = world.event(*id) else {
            continue;
        };
        if earlier.world_time < from {
            break;
        }
        if counted(earlier).is_none() && story_of(earlier) == Some(story) {
            told += 1;
            if told >= most {
                return true;
            }
        }
    }
    false
}

/// The hands' deeds: something the player made, put up, planted, moved,
/// gave or asked someone out to.
const DEEDS: [&str; 6] = [
    "built_by_hand",
    "decorated_by_hand",
    "planted_by_hand",
    "moved_by_hand",
    "gift_given",
    "invited_out",
];

/// What the player did, as a letter cites it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// Something done by hand, as it was told, lowered to sit inside a
    /// sentence: "you built a bench by the mill".
    Deed(String),
    /// An answer, in the words it was offered in: "Make peace".
    Said(String),
}

/// What the player did, if `event` is a thing they did.
fn act_of(state: &WorldState, event: &Event) -> Option<Act> {
    if DEEDS.contains(&event.kind.as_str()) {
        let told = match event.payload.get("told") {
            Some(Value::Text(told)) => told,
            _ => return None,
        };
        let rest = told.strip_prefix("You ")?;
        return Some(Act::Deed(format!("you {rest}")));
    }
    if event.kind == "situation_answered" {
        let text = |key: &str| match event.payload.get(key) {
            Some(Value::Text(text)) => Some(text.as_str()),
            _ => None,
        };
        if let Some(offered) = text("offered") {
            return (!offered.contains('{')).then(|| Act::Said(offered.to_string()));
        }
        let label = answer_label(text("kind")?, text("answer")?)?;
        let a = event.actor.map(|a| name(state, a)).unwrap_or_default();
        let b = event
            .targets
            .iter()
            .find(|b| Some(**b) != event.actor)
            .map(|b| name(state, *b))
            .unwrap_or_default();
        let label = fill(label, &[("a", &a), ("b", &b)]);
        return (!label.contains('{')).then_some(Act::Said(label));
    }
    None
}

/// What the player did that brought `event` about, and the Event they did
/// it in: `event` itself when it is the player's answer, else the nearest
/// of what caused it, and of what caused that, a few steps back. Only
/// what the record names as a cause is ever cited.
pub fn player_act(world: &World, event: &Event) -> Option<(EventId, Act)> {
    let state = world.state();
    if let Some(act) = (event.kind == "situation_answered")
        .then(|| act_of(state, event))
        .flatten()
    {
        return Some((event.id, act));
    }
    let mut seen = BTreeSet::from([event.id]);
    let mut queue = event
        .caused_by
        .iter()
        .map(|id| (*id, 1))
        .collect::<VecDeque<_>>();
    while let Some((id, steps)) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        let Some(cause) = world.event(id) else {
            continue;
        };
        if let Some(act) = act_of(state, cause) {
            return Some((cause.id, act));
        }
        if steps < CAUSE_STEPS {
            queue.extend(cause.caused_by.iter().map(|id| (*id, steps + 1)));
        }
    }
    None
}

/// Whether `told` already tells `act`: a deed by its doing ("the bench
/// you built" tells "you built a bench by the mill"), an answer by its
/// words. A line that only speaks to the player ("Ann told you something")
/// tells nothing of what they did.
pub fn tells(told: &str, act: &Act) -> bool {
    let told = told.to_lowercase();
    match act {
        Act::Deed(deed) => deed
            .split_whitespace()
            .nth(1)
            .is_some_and(|did| told.contains(&format!("you {}", did.to_lowercase()))),
        Act::Said(words) => told.contains(&words.to_lowercase()),
    }
}

/// How a letter says what the player did brought its news about.
const CITE_DEED: [&str; 2] = ["All because {act}.", "And to think it started when {act}."];
const CITE_SAID: [&str; 2] = [
    "All because you said “{words}”.",
    "And to think it started when you said “{words}”.",
];

/// The sentence a letter cites `act` in, one of two ways by `seed`.
pub fn cited(act: &Act, seed: u64) -> String {
    match act {
        Act::Deed(act) => fill(
            pick(&CITE_DEED, seed).copied().unwrap_or_default(),
            &[("act", act)],
        ),
        Act::Said(words) => fill(
            pick(&CITE_SAID, seed).copied().unwrap_or_default(),
            &[("words", words)],
        ),
    }
}
