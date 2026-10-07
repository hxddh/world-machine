//! Quiet days: a letter now and then, never more than a Pack allows in a
//! week, each with news from the year or a memory of the writer's own; and
//! on the other quiet days a small first: someone speaking for the first
//! time of someone or something they have never mentioned, or showing the
//! player a corner of a place they have not seen.
//!
//! What has been spoken of and shown is kept in the System's notes, so
//! nothing here reads the whole history to know what is still to come.
//! Besides what a Pack gives, what has happened lately is always something
//! someone has not yet spoken of, so the firsts never run out.

use super::*;

/// A corner of a place, for a quiet day: where it is, what it is called
/// ("the net loft"), and what whoever shows it says.
#[derive(Clone, Copy, Debug)]
pub struct Corner {
    pub place: EntityId,
    pub name: &'static str,
    pub said: &'static str,
}

/// Something someone may speak of for the first time ("the lighthouse"),
/// with the things they might say of it; with nothing of its own to say,
/// it is spoken of in a few words that fit anything.
#[derive(Clone, Copy, Debug)]
pub struct Subject {
    pub about: &'static str,
    pub said: &'static [&'static str],
}

/// How a Pack keeps its quiet days. The default keeps them as before: a
/// letter on every quiet day once everyone is met, and no firsts.
#[derive(Clone, Copy, Debug, Default)]
pub struct QuietDays {
    /// The most letters in any seven periods, if there is a limit.
    pub letters_a_week: Option<usize>,
    /// The most letters each week may bring (a week being seven periods
    /// from the World's start), picked week by week from these in a mixed
    /// order: some weeks two, some one, some none, so letters come in no
    /// set rhythm. Never more than `letters_a_week`. Empty: every week
    /// may bring `letters_a_week`.
    pub letter_weeks: &'static [usize],
    /// One day in this many is left quiet, once the first stretch of that
    /// many has passed: no letter, nothing shown and nothing spoken of for
    /// the first time. None: no day is.
    pub still_every: Option<u64>,
    /// Corners of the Pack's places a quiet day can show.
    pub corners: &'static [Corner],
    /// What people can speak of for the first time, besides each other.
    pub subjects: &'static [Subject],
}

/// What someone has spoken of for the first time: `p<id>` for a person,
/// `s<index>` for one of the Pack's subjects.
const SPOKE_OF: &str = "lives.spoke_of";
/// The corners the player has been shown, by their place in the Pack's
/// list.
const CORNERS_SEEN: &str = "lives.corners_seen";
/// How many firsts there have been.
const FIRSTS: &str = "lives.firsts";
/// The news letters have carried lately, so the same is not told twice.
const LETTER_NEWS: &str = "lives.letter_news";
/// What someone has recalled in a letter, so they do not recall it again.
const RECALLED: &str = "lives.recalled";
/// How much of each list is kept.
const KEPT_NEWS: usize = 40;
const KEPT_RECALLED: usize = 24;
/// The fewest periods between two letters, so a week's letters are spread.
const LETTER_GAP: u64 = 3;
/// How far back a letter's news reaches, in periods.
const NEWS_PERIODS: u64 = 60;
/// How long ago something has to be for its writer to call it a memory.
const MEMORY_PERIODS: u64 = 14;
/// How far back what has happened lately reaches, for a first word on it.
const HAPPENING_PERIODS: u64 = 60;

/// What is worth remembering or speaking of: not a rest on a bench or a
/// day's errands, but friendships, answers, the year's turns, festivals,
/// things made and outings.
pub(crate) const MEMORABLE: [&str; 8] = [
    "bond_changed",
    "situation_answered",
    "suggestion_held",
    "festival_held",
    "built_by_hand",
    "decorated_by_hand",
    "planted_by_hand",
    "year_turned",
];

/// What happened between people, as something to speak of for the first
/// time: this System's own moments, told in its own words.
const SPOKEN_OF: [&str; 4] = [
    "bond_changed",
    "situation_answered",
    "suggestion_held",
    "year_turned",
];

/// Something that happened, as something to speak of.
const THE_DAY: &str = "the day {told}";

/// An evening two people spent together, as something to speak of.
const EVENING: &str = "the evening {a} and {b} spent at {place}";

fn text_hash(text: &str) -> u64 {
    mix(&text.bytes().map(u64::from).collect::<Vec<_>>())
}

fn happening_key(told: &str) -> String {
    format!(
        "n{:x}",
        mix(&told.bytes().map(u64::from).collect::<Vec<_>>())
    )
}

const NEWS_LEADS: [&str; 4] = [
    "The news here is that {told}.",
    "In case nobody told you, {told}.",
    "Big news round here: {told}.",
    "You'll have heard, but {told}.",
];

/// How a letter with no news tells of the writer's own day.
const EVERYDAY_LEADS: [&str; 3] = [
    "Not much news, only this: {told}.",
    "Life goes on here: {told}.",
    "All quiet this week. Only that {told}.",
];

const MEMORY_LEADS: [&str; 4] = [
    "I was thinking today about when {told}.",
    "I still smile about the time {told}.",
    "Do you remember when {told}? I do.",
    "Funny what stays with you. The day {told}, for one.",
];

/// What someone says of something with no words of its own.
const REACTIONS: [&str; 6] = [
    "It means more to me than I let on.",
    "I've a soft spot for it, if I'm honest.",
    "I could take it or leave it, to be fair.",
    "It always puts me in a good mood.",
    "My mother loved it. So do I.",
    "I remember the first time, clear as day.",
];

const PERSON_INTROS: [&str; 6] = [
    "I've never told you about {other}, have I?",
    "Funny, I've never once mentioned {other}.",
    "Did I ever tell you about {other}?",
    "You'll know {other} by now.",
    "Can I tell you something about {other}?",
    "I've been meaning to say something about {other}.",
];

const SUBJECT_INTROS: [&str; 6] = [
    "I've never told you what I think of {subject}.",
    "Funny, we've never talked about {subject}.",
    "Can I tell you something about {subject}?",
    "You've never heard me on {subject}, have you?",
    "Here's something about {subject} I've never said.",
    "I don't think I've said a word about {subject} before.",
];

/// What someone thinks of someone else, said for the first time: two ways
/// for each way two people can stand.
fn view_of(state: &WorldState, person: EntityId, other: EntityId, seed: u64) -> &'static str {
    let view = opinion(state, person, other);
    let lines: [&str; 2] = if partner(state, person) == Some(other) {
        ["I'd be lost without them.", "They make every day easier."]
    } else if view >= 50 {
        [
            "One of the best people I know.",
            "I'd trust them with anything.",
        ]
    } else if view >= 20 {
        ["I like them. I really do.", "Good company, always."]
    } else if view > -20 {
        [
            "We get on well enough.",
            "We nod in passing. That's about it.",
        ]
    } else if view > -50 {
        [
            "We don't see eye to eye.",
            "Best we keep out of each other's way.",
        ]
    } else {
        [
            "I'd rather not talk about them, to be honest.",
            "We fell out. It still stings.",
        ]
    };
    lines[(seed % 2) as usize]
}

fn texts(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn component<'a>(state: &'a WorldState, entity: EntityId, key: &str) -> Option<&'a Value> {
    state.entity(entity)?.component(key)
}

fn spoken_of(state: &WorldState, person: EntityId) -> Vec<String> {
    texts(component(state, person, SPOKE_OF))
}

fn corners_seen(state: &WorldState, cast: &Cast) -> Vec<i64> {
    match component(state, cast.notes, CORNERS_SEEN) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Integer(at) => Some(*at),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A list with one more on the end, keeping at most `most`.
fn pushed(mut list: Vec<String>, item: String, most: usize) -> Value {
    list.push(item);
    let skip = list.len().saturating_sub(most);
    Value::List(list.into_iter().skip(skip).map(Value::Text).collect())
}

/// A line about the writer, as the writer would write it: "Greta is
/// grown now, with a trade of their own" in Greta's own letter is "I'm
/// grown now, with a trade of my own"; "Leo and Greta made it up" is "Leo
/// and I made it up"; "Leo gave Greta a gift" is "Leo gave me a gift".
pub(crate) fn in_own_words(told: &str, first: &str) -> String {
    if first.is_empty()
        || !told
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word == first)
    {
        return told.to_string();
    }
    // The writer's surname goes with their first name: "Yusuf and Nia
    // Adeyemi" is "Yusuf and I", never "Yusuf and I Adeyemi".
    let mut line = told
        .split(' ')
        .scan(false, |after_first, word| {
            let surname = *after_first
                && word.chars().next().is_some_and(char::is_uppercase)
                && word.chars().all(|c| c.is_alphabetic() || c == '-')
                && word.len() > 1;
            *after_first = word == first;
            Some((!surname).then_some(word))
        })
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    // Someone unnamed and the writer did something together: "A few of
    // us", never "whoever and I was free".
    for unnamed in [
        "whoever was free",
        "whoever was about",
        "anyone who was free",
    ] {
        if let Some(rest) = line.strip_prefix(&format!("{first} and {unnamed} ")) {
            return format!("A few of us {}", rest.replace(" their own", " our own"));
        }
    }
    let subject = if let Some(rest) = line.strip_prefix(&format!("{first} and ")) {
        // "Greta and Leo walked out": "Leo and I walked out"; "Greta and
        // Lin Mei became friends": "Lin Mei and I became friends".
        let words = rest.split(' ').collect::<Vec<_>>();
        let named = words
            .iter()
            .take_while(|word| word.chars().next().is_some_and(char::is_uppercase))
            .count()
            .max(1)
            .min(words.len());
        let other = words[..named].join(" ");
        let rest = words[named..].join(" ");
        Some(format!("{other} and I {rest}"))
    } else if let Some(rest) = line.strip_prefix(&format!("{first}'s ")) {
        Some(format!("My {rest}"))
    } else {
        line.strip_prefix(&format!("{first} ")).map(|rest| {
            let rest = rest
                .strip_prefix("is ")
                .map(|rest| format!("'m {rest}"))
                .or_else(|| rest.strip_prefix("has ").map(|rest| format!("'ve {rest}")))
                .unwrap_or_else(|| format!(" {rest}"));
            format!("I{rest}")
        })
    };
    let by_self = subject.is_some();
    if let Some(subject) = subject {
        line = subject;
    }
    // Something two people share: "Tomas Vale and Greta's party" and
    // "Greta and Keisha's party" are "Tomas Vale's and my party" and
    // "Keisha's and my party".
    line = shared_with(&line, first);
    // "Leo and Greta became friends": "Leo and I".
    let pair = format!(" and {first} ");
    if let Some(at) = line.find(&pair) {
        // Only where what comes before is a name: "Lin Mei and Greta".
        if line[..at]
            .split(' ')
            .all(|word| word.chars().next().is_some_and(char::is_uppercase))
        {
            line.replace_range(at..at + pair.len(), " and I ");
        }
    }
    line = line
        .replace(&format!("{first}'s "), "my ")
        .replace(&format!(" {first} "), " me ")
        .replace(&format!(" {first}."), " me.")
        .replace(&format!(" {first},"), " me,");
    if line.ends_with(&format!(" {first}")) {
        let cut = line.len() - first.len();
        line.replace_range(cut.., "me");
    }
    if by_self {
        line = line
            .replace(" their own", " my own")
            .replace(" of their ", " of my ")
            // "I went fishing with Miri, and they had words": we did.
            .replace(", and they ", ", and we ");
    }
    line
}

/// `line` with what `first` shares with someone told as theirs and mine.
fn shared_with(line: &str, first: &str) -> String {
    let words = line.split(' ').collect::<Vec<_>>();
    let possessive = format!("{first}'s");
    for at in 0..words.len() {
        // "<name> and Greta's <thing>"
        if words[at] == "and" && words.get(at + 1) == Some(&possessive.as_str()) {
            let start = (0..at)
                .rev()
                .take_while(|i| words[*i].chars().next().is_some_and(char::is_uppercase))
                .last();
            if let Some(start) = start {
                let name = words[start..at].join(" ");
                let mut out = words[..start].to_vec().join(" ");
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&format!("{name}'s and my"));
                for word in &words[at + 2..] {
                    out.push(' ');
                    out.push_str(word);
                }
                return out;
            }
        }
        // "Greta and <name>'s <thing>"
        if words[at] == first && words.get(at + 1) == Some(&"and") {
            let owner = (at + 2..words.len()).find(|i| words[*i].ends_with("'s"));
            let named = owner.is_some_and(|owner| {
                words[at + 2..=owner]
                    .iter()
                    .all(|word| word.chars().next().is_some_and(char::is_uppercase))
            });
            if let (Some(owner), true) = (owner, named) {
                let mut out = words[..at].to_vec();
                out.extend_from_slice(&words[at + 2..=owner]);
                out.push("and");
                out.push("my");
                out.extend_from_slice(&words[owner + 1..]);
                return out.join(" ");
            }
        }
    }
    line.to_string()
}

/// "The harbour held its Regatta" as it reads inside a sentence: an
/// opening article or everyone-word lowered, a name left as it is.
pub(crate) fn inside(told: &str) -> String {
    let first = told.split(' ').next().unwrap_or("");
    if matches!(
        first,
        "The" | "A" | "An" | "Every" | "Everyone" | "Nobody" | "You"
    ) {
        let mut chars = told.chars();
        chars
            .next()
            .map(|c| c.to_lowercase().chain(chars).collect())
            .unwrap_or_default()
    } else {
        told.to_string()
    }
}

/// A small first, as the Action records it.
pub(crate) struct Firsts(pub(crate) fn(&WorldState) -> Cast);

impl Action for Firsts {
    fn name(&self) -> &'static str {
        "lives_first"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let who = arg_entity(request, "who")?;
        let told = arg_text(request, "told")?;
        let said = arg_text(request, "said")?;
        if !enrolled(state, who) || gone(state, who) || said.trim().is_empty() {
            return Err(ActionError::Invalid("nobody to say it".into()));
        }
        let mut draft;
        if let Ok(corner) = arg_text(request, "corner") {
            let at = corner
                .parse::<i64>()
                .map_err(|_| ActionError::Invalid("which corner?".into()))?;
            let mut seen = corners_seen(state, &cast);
            if seen.contains(&at) {
                return Err(ActionError::Invalid("already seen".into()));
            }
            seen.push(at);
            draft = EventDraft::new("corner_shown");
            draft.changes.push(StateChange::SetComponent {
                entity: cast.notes,
                key: CORNERS_SEEN.into(),
                value: Value::List(seen.into_iter().map(Value::Integer).collect()),
            });
        } else {
            let about = arg_text(request, "about")?;
            let spoken = spoken_of(state, who);
            if spoken.iter().any(|said| said == about) {
                return Err(ActionError::Invalid("already spoken of".into()));
            }
            draft = EventDraft::new("first_mentioned");
            draft.changes.push(StateChange::SetComponent {
                entity: who,
                key: SPOKE_OF.into(),
                value: pushed(spoken, about.to_string(), usize::MAX),
            });
        }
        draft.changes.push(StateChange::SetComponent {
            entity: cast.notes,
            key: FIRSTS.into(),
            value: (integer(state, cast.notes, FIRSTS).unwrap_or(0) + 1).into(),
        });
        draft.actor = Some(who);
        draft.targets = vec![who];
        draft.payload.insert("told".into(), told.into());
        draft.payload.insert("said".into(), said.into());
        draft.payload.insert("first".into(), true.into());
        Ok(draft)
    }
}

/// How many firsts the World has had.
pub fn firsts_count(state: &WorldState, cast: &Cast) -> i64 {
    integer(state, cast.notes, FIRSTS).unwrap_or(0)
}

/// The kinds of event a small first comes in.
pub(crate) const FIRST_KINDS: [&str; 2] = ["first_mentioned", "corner_shown"];

/// Every small first the player has had, as told, oldest first: found
/// through the World's index of its history by kind, not by reading all
/// of it.
pub fn firsts(world: &World) -> Vec<String> {
    world
        .events_of_kind(&FIRST_KINDS)
        .into_iter()
        .filter_map(told)
        .collect()
}

/// The request for the next small first, if there is one still to have.
fn next_first(
    world: &World,
    cast: &Cast,
    quiet: &QuietDays,
    people: &[EntityId],
) -> Option<ActionRequest> {
    let state = world.state();
    let now = period(state, cast);
    let count = firsts_count(state, cast);
    let mut order = people.to_vec();
    order.sort_by_key(|person| mix(&[person.0, now, 71]));
    // Corners, subjects and people in turn, so no one kind runs out first.
    let corner = || {
        let seen = corners_seen(state, cast);
        let (at, corner) = quiet
            .corners
            .iter()
            .enumerate()
            .filter(|(at, _)| !seen.contains(&(*at as i64)))
            .min_by_key(|(at, _)| mix(&[*at as u64, 73]))?;
        // Shown by someone who works there, else by whoever comes first.
        let who = order
            .iter()
            .copied()
            .find(|person| (cast.work)(state, *person) == Some(corner.place))
            .or_else(|| order.first().copied())?;
        let told = format!("{} showed you {}", first_name(state, who), corner.name);
        Some(
            ActionRequest::new("lives_first")
                .actor(who)
                .arg("who", Value::Entity(who))
                .arg("corner", at.to_string())
                .arg("told", told)
                .arg("said", corner.said),
        )
    };
    let subject = || {
        order.iter().copied().find_map(|who| {
            let spoken = spoken_of(state, who);
            let (at, subject) = quiet
                .subjects
                .iter()
                .enumerate()
                .filter(|(at, _)| !spoken.contains(&format!("s{at}")))
                .min_by_key(|(at, _)| mix(&[who.0, *at as u64, 79]))?;
            let seed = mix(&[who.0, at as u64, now, 83]);
            let intro = pick(&SUBJECT_INTROS, seed)?;
            let line = pick(subject.said, seed >> 8).or_else(|| pick(&REACTIONS, seed >> 8))?;
            let said = format!(
                "{} {line}",
                fill_owned(intro, &[("subject", subject.about.to_string())])
            );
            let told = format!(
                "{} spoke of {} for the first time",
                first_name(state, who),
                subject.about
            );
            Some(
                ActionRequest::new("lives_first")
                    .actor(who)
                    .arg("who", Value::Entity(who))
                    .arg("about", format!("s{at}"))
                    .arg("told", told)
                    .arg("said", said),
            )
        })
    };
    let person = || {
        order.iter().copied().find_map(|who| {
            let spoken = spoken_of(state, who);
            let other = order
                .iter()
                .copied()
                .filter(|other| *other != who && !spoken.contains(&format!("p{}", other.0)))
                .min_by_key(|other| mix(&[who.0, other.0, 89]))?;
            let seed = mix(&[who.0, other.0, now, 97]);
            let other_name = first_name(state, other);
            let intro = pick(&PERSON_INTROS, seed)?;
            let said = format!(
                "{} {}",
                fill_owned(intro, &[("other", other_name.clone())]),
                view_of(state, who, other, seed >> 8)
            );
            let told = format!(
                "{} spoke of {other_name} for the first time",
                first_name(state, who)
            );
            Some(
                ActionRequest::new("lives_first")
                    .actor(who)
                    .arg("who", Value::Entity(who))
                    .arg("about", format!("p{}", other.0))
                    .arg("told", told)
                    .arg("said", said),
            )
        })
    };
    // What has happened lately is always something someone has not yet
    // spoken of, so firsts never run out: what changed between people, in
    // this System's own words, and an evening two others spent together.
    let happening = || {
        let events = world.events();
        let since = world
            .world_time()
            .saturating_sub(HAPPENING_PERIODS * cast.period.max(1));
        let from = events.partition_point(|event| event.world_time < since);
        // Each as (the key it is remembered by, what it is called).
        let happenings = events[from..]
            .iter()
            .filter_map(|event| {
                if event.kind == "lived" {
                    let a = event.actor?;
                    let b = *event.targets.first()?;
                    if a == b || !enrolled(state, a) || !enrolled(state, b) {
                        return None;
                    }
                    let place = match event.payload.get("place") {
                        Some(Value::Entity(place)) if state.entity(*place).is_some() => *place,
                        _ => return None,
                    };
                    let about = fill_owned(
                        EVENING,
                        &[
                            ("a", first_name(state, a)),
                            ("b", first_name(state, b)),
                            ("place", name(state, place)),
                        ],
                    );
                    return Some((
                        vec![a, b],
                        format!("h{}-{}-{}", a.0.min(b.0), a.0.max(b.0), place.0),
                        about,
                    ));
                }
                if !SPOKEN_OF.contains(&event.kind.as_str()) {
                    return None;
                }
                match event.payload.get("told") {
                    Some(Value::Text(told)) if !told.is_empty() => Some((
                        Vec::new(),
                        happening_key(told),
                        fill_owned(THE_DAY, &[("told", inside(told))]),
                    )),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        order.iter().copied().find_map(|who| {
            let spoken = spoken_of(state, who);
            let (key, about) = happenings
                .iter()
                .filter(|(between, key, _)| !between.contains(&who) && !spoken.contains(key))
                .map(|(_, key, about)| (key, about))
                .max_by_key(|(key, _)| mix(&[who.0, text_hash(key), now, 107]))?;
            let seed = mix(&[who.0, text_hash(key), now, 109]);
            let intro = pick(&SUBJECT_INTROS, seed)?;
            let line = pick(&REACTIONS, seed >> 8)?;
            let said = format!(
                "{} {line}",
                fill_owned(intro, &[("subject", about.clone())])
            );
            let told_first = format!(
                "{} spoke of {about} for the first time",
                first_name(state, who)
            );
            Some(
                ActionRequest::new("lives_first")
                    .actor(who)
                    .arg("who", Value::Entity(who))
                    .arg("about", key.clone())
                    .arg("told", told_first)
                    .arg("said", said),
            )
        })
    };
    let kinds: [&dyn Fn() -> Option<ActionRequest>; 4] = [&corner, &subject, &person, &happening];
    let start = count.rem_euclid(4) as usize;
    (0..4).find_map(|offset| kinds[(start + offset) % 4]())
}

/// The most letters this week may bring: the Pack's week-by-week pattern,
/// never more than `most`.
pub(crate) fn letters_this_week(quiet: &QuietDays, now: u64, most: usize) -> usize {
    if quiet.letter_weeks.is_empty() {
        return most;
    }
    let week = now / 7;
    let at = (mix(&[week, 137]) % quiet.letter_weeks.len() as u64) as usize;
    quiet.letter_weeks[at].min(most)
}

/// Whether today is one of the days left quiet: never in a World's first
/// stretch of days, while everything is new.
pub(crate) fn still_day(quiet: &QuietDays, now: u64) -> bool {
    quiet
        .still_every
        .is_some_and(|every| every > 1 && now > every && now % every == every / 2)
}

/// The letters written lately: how many in the last seven periods, and how
/// many periods since the last. Only the week's tail of the history is read.
fn letters_lately(world: &World, cast: &Cast) -> (usize, u64) {
    let now = world.world_time();
    let span = cast.period.max(1);
    let since = now.saturating_sub(span * 7);
    let events = world.events();
    let from = events.partition_point(|event| event.world_time <= since);
    let lately = events[from..]
        .iter()
        .filter(|event| matches!(event.kind.as_str(), "letter_written" | "guest_visited"))
        .map(|event| event.world_time)
        .collect::<Vec<_>>();
    let ago = lately
        .last()
        .map_or(u64::MAX, |last| now.saturating_sub(*last) / span);
    (lately.len(), ago)
}

/// Something `writer` was part of a while ago, of `memorable` (the
/// World's memorable events, oldest first), not recalled before.
pub(crate) fn memory<'a>(
    world: &World,
    cast: &Cast,
    writer: EntityId,
    memorable: impl Iterator<Item = &'a Event>,
) -> Option<String> {
    let state = world.state();
    let now = period(state, cast);
    let recalled = texts(component(state, writer, RECALLED));
    let before = world
        .world_time()
        .saturating_sub(MEMORY_PERIODS * cast.period.max(1));
    memorable
        .take_while(|event| event.world_time < before)
        .filter(|event| event.actor == Some(writer) || event.targets.contains(&writer))
        .filter_map(|event| match event.payload.get("told") {
            Some(Value::Text(told)) if !told.is_empty() && !recalled.contains(told) => {
                Some(told.clone())
            }
            _ => None,
        })
        .max_by_key(|told| mix(&[writer.0, told.len() as u64, now, 103]))
}

/// What a letter from `writer` could tell: fresh news from the harbour's
/// year not yet written about, or else something the writer was part of
/// a while ago and has not recalled before. The lines, and what to note
/// as told.
fn letter_body(world: &World, cast: &Cast, writer: EntityId) -> Option<(String, ActionRequest)> {
    let state = world.state();
    let writer_name = first_name(state, writer);
    // "Nia Chen shared supper" in Nia's own letter is "I shared supper".
    let full_name = name(state, writer);
    let own = |told: &str| in_own_words(&told.replace(&full_name, &writer_name), &writer_name);
    let now = period(state, cast);
    let span = cast.period.max(1);
    let seed = mix(&[writer.0, now, 101]);
    let told_news = texts(component(state, cast.notes, LETTER_NEWS));
    let news = news_since(
        world,
        world.world_time().saturating_sub(NEWS_PERIODS * span),
    )
    .into_iter()
    .rev()
    .find(|told| !told_news.contains(told));
    let request = ActionRequest::new("lives_leaves_keepsake")
        .actor(writer)
        .arg("who", Value::Entity(writer))
        .arg("letter", "yes");
    if let Some(news) = news {
        let lead = pick(&NEWS_LEADS, seed)?;
        let body = fill_owned(lead, &[("told", own(&inside(&news)))]);
        return Some((body, request.arg("news", news)));
    }
    let memory = memory(
        world,
        cast,
        writer,
        world.events_of_kind(&MEMORABLE).into_iter(),
    );
    if let Some(memory) = memory {
        let lead = pick(&MEMORY_LEADS, seed)?;
        let body = fill_owned(lead, &[("told", own(&inside(&memory)))]);
        return Some((body, request.arg("recalled", memory)));
    }
    // With no news and nothing yet to look back on, what the writer did
    // with their own day, which is news of a kind.
    let since = world.world_time().saturating_sub(NEWS_PERIODS * span);
    let everyday = world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time >= since)
        .filter(|event| event.kind == "lived" && event.actor == Some(writer))
        .find_map(crate::told)
        .filter(|told| !told_news.contains(told))?;
    let lead = pick(&EVERYDAY_LEADS, seed / 3)?;
    let body = fill_owned(lead, &[("told", own(&inside(&everyday)))]);
    Some((body, request.arg("news", everyday)))
}

/// The changes a letter's news or memory makes to the notes.
pub(crate) fn letter_notes(
    state: &WorldState,
    cast: &Cast,
    who: EntityId,
    request: &ActionRequest,
) -> Vec<StateChange> {
    let mut changes = Vec::new();
    if let Ok(news) = arg_text(request, "news") {
        changes.push(StateChange::SetComponent {
            entity: cast.notes,
            key: LETTER_NEWS.into(),
            value: pushed(
                texts(component(state, cast.notes, LETTER_NEWS)),
                news.to_string(),
                KEPT_NEWS,
            ),
        });
    }
    if let Ok(recalled) = arg_text(request, "recalled") {
        changes.push(StateChange::SetComponent {
            entity: who,
            key: RECALLED.into(),
            value: pushed(
                texts(component(state, who, RECALLED)),
                recalled.to_string(),
                KEPT_RECALLED,
            ),
        });
    }
    changes
}

/// A letter from whoever has written least, the fondest first, with news
/// or a memory in it; `None` when nobody has anything to tell.
fn letter(world: &World, cast: &Cast, people: &[EntityId]) -> Option<ActionRequest> {
    let state = world.state();
    let now = period(state, cast);
    let mut writers = people.to_vec();
    writers.sort_by_key(|person| {
        (
            integer(state, *person, WROTE).unwrap_or(0),
            -regard(state, *person),
            mix(&[person.0, now, 53]),
        )
    });
    writers.into_iter().find_map(|writer| {
        let (body, request) = letter_body(world, cast, writer)?;
        let first = first_name(state, writer);
        let words = [
            ("name", first),
            ("settlement", cast.settlement.to_string()),
            ("gathering", name(state, cast.gathering)),
            ("unit", cast.unit.to_string()),
        ];
        let seed = mix(&[writer.0, now, 59]);
        let opening = pick(&LETTER_OPENINGS, seed).copied().unwrap_or_default();
        let closing = pick(&LETTER_CLOSINGS, seed >> 8)
            .map(|line| fill_owned(line, &words))
            .unwrap_or_default();
        Some(request.arg("why", format!("{opening} {body} {closing}")))
    })
}

/// A quiet day as a Pack with limits keeps it: a letter when the week has
/// room for one and the last was a few days ago, else a small first, else
/// a letter if the week still has room. While the player is away, only
/// letters come.
pub(crate) fn quiet_day(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    away: bool,
    quiet: &QuietDays,
    most: usize,
    people: &[EntityId],
) -> Result<Option<EventId>, WorldError> {
    let now = period(world.state(), cast);
    if !away && still_day(quiet, now) {
        return Ok(None);
    }
    let most = letters_this_week(quiet, now, most);
    let (lately, ago) = letters_lately(world, cast);
    // While the player is away, one of the week's letters is kept for
    // their return, in case nobody has room to leave them anything.
    let room = lately < if away { most.saturating_sub(1) } else { most };
    let letter_now = room && (away || ago >= LETTER_GAP);
    let attempt = |world: &mut World, request: Option<ActionRequest>| {
        request.and_then(|request| world.execute(actions, &request).ok().map(|event| event.id))
    };
    if letter_now {
        let request = letter(world, cast, people);
        if let Some(id) = attempt(world, request) {
            return Ok(Some(id));
        }
    }
    if away {
        return Ok(None);
    }
    let request = next_first(world, cast, quiet, people);
    if let Some(id) = attempt(world, request) {
        return Ok(Some(id));
    }
    if room && !letter_now {
        let request = letter(world, cast, people);
        return Ok(attempt(world, request));
    }
    Ok(None)
}

/// On the player's return: something to keep from whoever thinks best of
/// them, or, when the week has no room for one, a letter, if the week has
/// room for that.
pub fn welcome_back(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    why: &str,
    quiet: &QuietDays,
) -> Result<Option<EventId>, WorldError> {
    if let Some(id) = leave_keepsake(world, actions, cast, why)? {
        return Ok(Some(id));
    }
    // A return always may bring a letter, whatever this week's rhythm: only
    // the week's most holds it back.
    let Some(most) = quiet.letters_a_week else {
        return Ok(None);
    };
    let (lately, _) = letters_lately(world, cast);
    if lately >= most {
        return Ok(None);
    }
    let state = world.state();
    let people = (cast.people)(world)
        .into_iter()
        .filter(|person| enrolled(state, *person) && !gone(state, *person))
        .collect::<Vec<_>>();
    let request = letter(world, cast, &people);
    Ok(request.and_then(|request| world.execute(actions, &request).ok().map(|event| event.id)))
}

#[cfg(test)]
mod rhythm {
    use super::*;

    const QUIET: QuietDays = QuietDays {
        letters_a_week: Some(2),
        letter_weeks: &[2, 1, 1, 0, 2, 1, 1, 2],
        still_every: Some(40),
        corners: &[],
        subjects: &[],
    };

    #[test]
    fn letters_vary_week_by_week_and_some_days_are_left_quiet() {
        let weeks = (0..52)
            .map(|week| letters_this_week(&QUIET, week * 7, 2))
            .collect::<Vec<_>>();
        let mean = weeks.iter().sum::<usize>() as f64 / weeks.len() as f64;
        assert!(weeks.contains(&0) && weeks.contains(&1) && weeks.contains(&2));
        assert!(mean <= 1.5, "{mean}");
        // Never more than the Pack's most, and the same week the same.
        assert!((0..52).all(|week| letters_this_week(&QUIET, week * 7, 1) <= 1));
        assert_eq!(
            letters_this_week(&QUIET, 70, 2),
            letters_this_week(&QUIET, 76, 2)
        );
        let still = (0..360)
            .filter(|now| still_day(&QUIET, *now))
            .collect::<Vec<_>>();
        assert!(still.iter().all(|now| *now > 40), "{still:?}");
        assert!((3..=9).contains(&still.len()), "{still:?}");
        // A Pack that asks for none keeps every day and week as before.
        let before = QuietDays {
            letters_a_week: Some(2),
            ..QuietDays::default()
        };
        assert!((0..360).all(|now| !still_day(&before, now)));
        assert!((0..52).all(|week| letters_this_week(&before, week * 7, 2) == 2));
    }
}

#[cfg(test)]
mod own_words {
    use super::in_own_words;

    /// A writer never writes of themselves by name.
    #[test]
    fn a_letter_tells_of_its_writer_in_the_first_person() {
        for (told, written) in [
            (
                "Greta is grown now, with a trade of their own: cheesemaker",
                "I'm grown now, with a trade of my own: cheesemaker",
            ),
            ("Greta and Leo walked out", "Leo and I walked out"),
            ("Leo and Greta made it up", "Leo and I made it up"),
            ("Leo gave Greta a gift", "Leo gave me a gift"),
            ("Greta's boat was mended", "My boat was mended"),
            (
                "Greta mended the nets at the quay",
                "I mended the nets at the quay",
            ),
            (
                "Greta went fishing with Miri, and they had words",
                "I went fishing with Miri, and we had words",
            ),
            (
                "Leo and Mara became firm friends",
                "Leo and Mara became firm friends",
            ),
            (
                "Tomas Vale shared supper with Greta",
                "Tomas Vale shared supper with me",
            ),
            (
                "Yusuf and Greta Adeyemi became firm friends",
                "Yusuf and I became firm friends",
            ),
            (
                "Greta Adeyemi and Yusuf became firm friends",
                "Yusuf and I became firm friends",
            ),
            (
                "Greta and Ines Duarte became firm friends",
                "Ines Duarte and I became firm friends",
            ),
            (
                "Lin Mei and Greta became firm friends",
                "Lin Mei and I became firm friends",
            ),
            (
                "the whole colony came to Tomas Vale and Greta's party at Ares Habitat",
                "the whole colony came to Tomas Vale's and my party at Ares Habitat",
            ),
            (
                "the whole street came to Greta and Keisha's party at Maple Arcade",
                "the whole street came to Keisha's and my party at Maple Arcade",
            ),
            (
                "Greta and whoever was free made something of their own: a mural on the arcade wall",
                "A few of us made something of our own: a mural on the arcade wall",
            ),
        ] {
            assert_eq!(in_own_words(told, "Greta"), written);
        }
    }
}

#[cfg(test)]
mod articles {
    #[test]
    fn a_word_that_starts_with_a_vowel_takes_an() {
        assert_eq!(
            crate::articled("Yusuf, a engineer from Phobos, wants a room. A orchard!"),
            "Yusuf, an engineer from Phobos, wants a room. An orchard!"
        );
        assert_eq!(
            crate::articled("a banana, a unicorn"),
            "a banana, a unicorn"
        );
        assert_eq!(
            crate::articled("Mixtape: side A is loud."),
            "Mixtape: side A is loud."
        );
    }
}
