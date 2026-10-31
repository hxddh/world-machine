//! Answers drawn from what someone knows, so talk never comes to "I don't
//! understand".
//!
//! The meanings this System hears are a small closed set (see [`Intent`]),
//! and what the words do is decided by them alone. What someone *answers*
//! can look closer at how a question was put: asked where someone is, who
//! they are, what happened, how the work is going or about a letter, they
//! answer from what they know (their place, their work, the people they
//! know and the moments of the World's record they were part of). Asked
//! something they cannot answer, they say so in their own way and offer
//! three things the player could ask them instead, from what they know;
//! the same three are shown as buttons beside the words.
//!
//! Everything here is read from the World's state and its record, the same
//! for the same history, and the answer is recorded whole: replay never
//! asks again.

use crate::*;

/// How many things someone offers to be asked when they cannot answer.
pub const OFFERS: usize = 3;

/// What the player could ask about someone's day.
pub const ASK_DAY: &str = "How was your day?";
/// Who someone else is, `{name}` theirs.
pub const ASK_WHO: &str = "Who is {name}?";
/// Where someone else is, `{name}` theirs.
pub const ASK_WHERE: &str = "Where is {name}?";
/// What happened lately, to them or near them.
pub const ASK_HAPPENED: &str = "What happened lately?";
/// What is coming up on the calendar.
pub const ASK_COMING: &str = "What's coming up?";
/// How the place's work is going.
pub const ASK_BUILDING: &str = "How's the building work going?";
/// The letter they wrote the player.
pub const ASK_LETTER: &str = "Tell me about your letter.";
/// Their family.
pub const ASK_FAMILY: &str = "Tell me about your family.";

/// Every question this module offers, as a catalog translates it, with
/// [`crate::openers::ALL`].
pub const QUESTIONS: [&str; 8] = [
    ASK_DAY,
    ASK_WHO,
    ASK_WHERE,
    ASK_HAPPENED,
    ASK_COMING,
    ASK_BUILDING,
    ASK_LETTER,
    ASK_FAMILY,
];

/// What someone could be asked about, as they name it when they offer it.
pub const TOPICS: [&str; 7] = [
    "my work",
    "my day",
    "my family",
    "what's new",
    "what happened lately",
    "what's coming up",
    "my letter",
];

/// How someone who cannot answer says so, by their first trait: two ways
/// each.
pub const LEADS: &[(&str, [&str; 2])] = &[
    (
        "warm",
        [
            "Oh, that's beyond me, love.",
            "I couldn't tell you that, pet.",
        ],
    ),
    ("prickly", ["Hm. Not my area.", "Can't help you there."]),
    (
        "proud",
        ["That's outside my line.", "I'll leave that one to others."],
    ),
    (
        "shy",
        ["Oh, um, I wouldn't know.", "I'm not the one to ask, sorry."],
    ),
    ("sociable", ["Ooh, no idea!", "Couldn't tell you, sorry!"]),
    ("restless", ["No idea. Moving on!", "Couldn't say."]),
    (
        "steady",
        [
            "I can't speak to that.",
            "That's not something I know about.",
        ],
    ),
    (
        "generous",
        ["I wish I could help with that.", "I'd tell you if I knew."],
    ),
    ("thrifty", ["Wouldn't know.", "Can't say I do."]),
    (
        "dreamy",
        [
            "Hm, that drifted right past me.",
            "Oh, I was miles away. I couldn't say.",
        ],
    ),
    (
        "",
        ["I couldn't tell you about that.", "That's not one for me."],
    ),
];

/// How the three offers are put, after the lead.
pub const ENDS: [&str; 2] = [
    "But I could tell you about {a}, {b} or {c}.",
    "Ask me about {a}, {b} or {c} instead.",
];

/// Words that ask where someone or something is.
const WHERE: &[&str] = &[
    "where",
    "where's",
    "whereabouts",
    "在哪",
    "哪里",
    "哪儿",
    "どこ",
];
/// Words that ask who someone is.
const WHO: &[&str] = &[
    "who is",
    "who's",
    "who was",
    "who exactly is",
    "是谁",
    "谁是",
    "是什么人",
    "誰",
    "だれ",
];
/// Words that ask what happened.
const HAPPENED: &[&str] = &[
    "what happened",
    "what's happened",
    "what has happened",
    "anything happen",
    "anything happened",
    "anything happening",
    "what's been happening",
    "what went on",
    "what's going on",
    "发生了什么",
    "发生什么",
    "出什么事",
    "出了什么事",
    "怎么回事",
    "何があった",
    "何かあった",
    "何が起きた",
    "何か起きた",
];
/// Words about the place's building work.
const BUILDING: &[&str] = &[
    "build",
    "builds",
    "building",
    "built",
    "construction",
    "the works",
    "建造",
    "修建",
    "建成",
    "建好",
    "在建",
    "工程",
    "施工",
    "盖好",
    "工事",
    "建設",
    "建て",
    "建築",
];
/// Words about a letter.
const LETTER: &[&str] = &[
    "letter",
    "letters",
    "wrote to me",
    "write to me",
    "written to me",
    "来信",
    "写信",
    "那封信",
    "你的信",
    "信里",
    "寄信",
    "手紙",
    "便り",
];
/// Words that ask after the one spoken to.
const YOU: &[&str] = &["you", "you're", "your", "你", "あなた", "君", "きみ"];
/// Words about a festival or a fair, when no day is named.
const FESTIVAL: &[&str] = &[
    "festival",
    "festivals",
    "celebration",
    "节日",
    "庆典",
    "过节",
    "祭り",
    "お祭り",
    "フェスティバル",
];

/// Meanings that leave room for a closer look at how a question was put:
/// those that do nothing to anyone, and say nothing of their own.
fn open_to_shape(heard: Heard) -> bool {
    match heard.intent {
        Intent::Unclear
        | Intent::Ack
        | Intent::Greet
        | Intent::HowAreYou
        | Intent::Day
        | Intent::AboutYou
        | Intent::Work
        | Intent::News
        | Intent::Coming
        | Intent::Weather => true,
        Intent::Place => heard.about.is_none(),
        _ => false,
    }
}

/// How someone answers the player's words, heard as `heard`: from what
/// they know, by how the question was put, if it asks something they can
/// answer that way, and as [`reply`] answers what was heard otherwise.
pub fn answer(world: &World, kit: &Kit, who: EntityId, heard: Heard, words: &str) -> Reply {
    let Some(line) = shaped(world, kit, who, heard, words) else {
        return reply(world, kit, who, heard);
    };
    let mut answer = Reply::said(line);
    favour::answer(world, kit, who, heard, &mut answer);
    answer
}

/// The answer to how a question was put, if it asks something someone can
/// answer from what they know.
pub(crate) fn shaped(
    world: &World,
    kit: &Kit,
    who: EntityId,
    heard: Heard,
    words: &str,
) -> Option<String> {
    let text = normal(words);
    let state = world.state();
    let asks = |cues: &[&str]| any(&text, cues);
    let seed = lives::mix(&[who.0, period(state, kit) as u64, spoken_today(world, who)]);
    if let (Intent::ThinkOf | Intent::HowIs, Some(other)) = (heard.intent, heard.about) {
        if asks(WHERE) {
            return Some(where_is(world, kit, who, other));
        }
        if asks(WHO) {
            return Some(who_is(world, kit, who, other));
        }
        return None;
    }
    if !open_to_shape(heard) {
        return None;
    }
    if asks(LETTER) {
        return Some(letter(world, kit, who));
    }
    let works = (kit.works)(world);
    if asks(BUILDING) || names_work(kit, &text, &works) {
        return Some(building(&works));
    }
    if asks(FESTIVAL) && heard.intent != Intent::Coming {
        return Some(coming(world, kit, who));
    }
    if asks(HAPPENED) {
        return Some(happened(world, kit, who, seed));
    }
    let idle = matches!(
        heard.intent,
        Intent::Unclear | Intent::Ack | Intent::Greet | Intent::HowAreYou
    );
    if idle && heard.about.is_none() && asks(WHERE) && asks(YOU) {
        return Some(
            match lives::at(state, who).filter(|at| state.entity(*at).is_some()) {
                Some(place) => format!("I'm at {} today.", lives::name(state, place)),
                None => format!("Out and about in {}.", kit.settlement),
            },
        );
    }
    // Asked who they are, or what they think of themselves, by their own
    // name: who they are.
    if heard.intent == Intent::Unclear && named(state, kit, &text, &[who], false).is_some() {
        return Some(about_you(world, kit, who));
    }
    None
}

/// The word a work is called by: the last word of its name before any
/// "on", "by", "at" or "of" ("lamp" for "A lamp on the point").
fn head(label: &str) -> Option<String> {
    let name = normal(label);
    let before = [" on ", " by ", " at ", " of ", " for "]
        .iter()
        .filter_map(|cut| name.find(cut))
        .min()
        .map_or(name.as_str(), |at| &name[..at]);
    before
        .split_whitespace()
        .last()
        .filter(|word| word.chars().count() >= 3)
        .map(str::to_string)
}

/// Whether the words name one of the place's works, by its head word or
/// any other name the World gives it.
fn names_work(kit: &Kit, text: &str, works: &[world_projection::Goal]) -> bool {
    works.iter().any(|work| {
        head(&work.label).is_some_and(|head| has(text, &head))
            || (kit.aliases)(&work.label)
                .iter()
                .any(|alias| !alias.is_ascii() && text.contains(alias.as_str()))
    })
}

/// Where someone else is, as far as `who` knows.
fn where_is(world: &World, kit: &Kit, who: EntityId, other: EntityId) -> String {
    let state = world.state();
    if lives::gone(state, other) {
        return how_is(world, kit, who, other, 0);
    }
    let x = lives::first_name(state, other);
    match lives::at(state, other).filter(|at| state.entity(*at).is_some()) {
        Some(place) => format!("{x}'s at {} just now.", lives::name(state, place)),
        None => format!("I haven't seen {x} about today."),
    }
}

/// Who someone else is, and what `who` thinks of them.
fn who_is(world: &World, kit: &Kit, who: EntityId, other: EntityId) -> String {
    let state = world.state();
    if lives::gone(state, other) {
        return how_is(world, kit, who, other, 0);
    }
    let role = (kit.role_line)(world, other);
    let view = lives::thinks_of(world, who, other);
    match (role, view) {
        (Some(role), Some(view)) => format!("{role} {view}"),
        (Some(said), None) | (None, Some(said)) => said,
        (None, None) => format!(
            "{}? I don't really know them yet.",
            lives::first_name(state, other)
        ),
    }
}

/// The latest moment of the World's news that `who` was part of, within
/// the last week of periods: when, and how it is told.
pub(crate) fn own_news(world: &World, kit: &Kit, who: EntityId) -> Option<(i64, String)> {
    let period_len = kit.period.max(1);
    let since = world
        .world_time()
        .saturating_sub(period_len.saturating_mul(7));
    world
        .history_index()
        .changes_of(who)
        .iter()
        .rev()
        .filter_map(|id| world.event(*id))
        .take_while(|event| event.world_time >= since)
        .filter(|event| {
            lives::is_news(event) && (event.actor == Some(who) || event.targets.contains(&who))
        })
        .find_map(|event| {
            let told = lives::told(event)?;
            Some(((event.world_time / period_len) as i64, told))
        })
}

/// What happened lately, as `who` tells it: something they were part of,
/// or the place's news.
fn happened(world: &World, kit: &Kit, who: EntityId, seed: u64) -> String {
    match own_news(world, kit, who) {
        Some((_, told)) => format!(
            "{} {}.",
            pick(&["You'll have heard.", "Well, since you ask."], seed),
            told.trim_end_matches('.')
        ),
        None => news(world, kit, seed),
    }
}

/// What `who` says about a letter: the one they wrote the player, or the
/// latest anyone did.
fn letter(world: &World, kit: &Kit, who: EntityId) -> String {
    let state = world.state();
    let letters = lives::letters(world);
    let now = period(state, kit);
    let ago = |at: u64| when(kit, now - (at / kit.period.max(1)) as i64);
    if let Some(mine) = letters.iter().rev().find(|letter| letter.from == who) {
        return format!(
            "I wrote to you {}. I hope it found you well.",
            ago(mine.world_time)
        );
    }
    match letters.last() {
        Some(theirs) => format!(
            "Not from me. {} wrote to you {}, didn't they?",
            lives::first_name(state, theirs.from),
            ago(theirs.world_time)
        ),
        None => "I haven't written to you yet. Maybe one day.".into(),
    }
}

/// How the place's work is going: the work under way, or the last one
/// finished.
fn building(works: &[world_projection::Goal]) -> String {
    if let Some(work) = works.iter().find(|work| !work.finished()) {
        let name = capitalized(&work.label);
        return if work.done == 0 {
            format!("{name} is still to be started. Everyone's talking about it.")
        } else {
            format!(
                "{name} is coming along. {} of {} done.",
                work.done, work.parts
            )
        };
    }
    match works.last() {
        Some(work) => format!(
            "{} is finished. It's done us all good.",
            capitalized(&work.label)
        ),
        None => "Nothing's being built just now.".into(),
    }
}

/// One thing someone could be asked: how they would name it, and the
/// question.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Offer {
    pub topic: String,
    pub question: String,
}

fn offer(topic: impl Into<String>, question: impl Into<String>) -> Offer {
    Offer {
        topic: topic.into(),
        question: question.into(),
    }
}

/// A name as it is said inside a sentence: "the new pier".
fn in_sentence(name: &str) -> String {
    for article in ["The ", "A ", "An "] {
        if let Some(rest) = name.strip_prefix(article) {
            return format!("{}{rest}", article.to_lowercase());
        }
    }
    name.to_string()
}

/// Everything `who` could be asked and answer from what they know, in an
/// order of their own for the period: their work, their day, the people
/// they know, where they are, what happened, what is coming up, the work
/// in hand, their letter and their family.
pub fn pool(world: &World, kit: &Kit, who: EntityId) -> Vec<Offer> {
    let state = world.state();
    let now = period(state, kit) as u64;
    let mut pool = Vec::new();
    if (kit.work_line)(world, who).is_some() {
        pool.push(offer(TOPICS[0], openers::WORK));
    }
    pool.push(offer(TOPICS[1], ASK_DAY));
    let others = (kit.people)(state)
        .into_iter()
        .filter(|other| *other != who && !lives::gone(state, *other))
        .collect::<Vec<_>>();
    let closest = others
        .iter()
        .copied()
        .filter(|other| lives::opinion(state, who, *other) >= 20)
        .max_by_key(|other| {
            (
                lives::opinion(state, who, *other),
                std::cmp::Reverse(*other),
            )
        });
    if let Some(friend) = closest {
        let name = lives::first_name(state, friend);
        pool.push(offer(
            name.clone(),
            openers::THINK_OF.replace("{name}", &name),
        ));
    }
    let strangers = others
        .iter()
        .copied()
        .filter(|other| Some(*other) != closest)
        .collect::<Vec<_>>();
    if !strangers.is_empty() {
        let other = strangers[(lives::mix(&[who.0, now]) % strangers.len() as u64) as usize];
        let name = lives::first_name(state, other);
        pool.push(offer(name.clone(), ASK_WHO.replace("{name}", &name)));
    }
    if let Some(place) = lives::at(state, who).filter(|at| (kit.places)(state).contains(at)) {
        let name = lives::name(state, place);
        pool.push(offer(
            name.clone(),
            openers::THINK_OF.replace("{name}", &name),
        ));
    }
    if own_news(world, kit, who).is_some() {
        pool.push(offer(TOPICS[4], ASK_HAPPENED));
    } else {
        pool.push(offer(TOPICS[3], openers::NEWS));
    }
    if (kit.coming_up)(world).is_some() {
        pool.push(offer(TOPICS[5], ASK_COMING));
    }
    let works = (kit.works)(world);
    if let Some(work) = works.iter().find(|work| !work.finished()).or(works.last()) {
        pool.push(offer(in_sentence(&work.label), ASK_BUILDING));
    }
    if lives::letters(world)
        .iter()
        .any(|letter| letter.from == who)
    {
        pool.push(offer(TOPICS[6], ASK_LETTER));
    }
    pool.push(offer(TOPICS[2], ASK_FAMILY));
    // An order of their own for the period, the same all period long.
    let mut keyed = pool
        .into_iter()
        .enumerate()
        .map(|(at, offer)| (lives::mix(&[who.0, now, at as u64]), offer))
        .collect::<Vec<_>>();
    keyed.sort_by_key(|(key, _)| *key);
    keyed.into_iter().map(|(_, offer)| offer).collect()
}

/// The three things `who` offers to be asked at the player's `turn` of the
/// period with them (how many times they have spoken): the next three of
/// their [`pool`], so each answer brings fresh ones.
pub fn offers(world: &World, kit: &Kit, who: EntityId, turn: u64) -> Vec<Offer> {
    let pool = pool(world, kit, who);
    if pool.is_empty() {
        return Vec::new();
    }
    let start = turn.saturating_mul(OFFERS as u64) as usize;
    (0..OFFERS.min(pool.len()))
        .map(|at| pool[(start + at) % pool.len()].clone())
        .collect()
}

/// What someone says when they cannot answer what was asked: that they
/// cannot, in their own way, and three things they could be asked
/// instead, offered by name in the line and as questions beside it.
pub(crate) fn deflection(world: &World, kit: &Kit, who: EntityId, seed: u64) -> Reply {
    let state = world.state();
    let trait_ = first_trait(state, who);
    let leads = LEADS
        .iter()
        .find(|(id, _)| *id == trait_)
        .or(LEADS.last())
        .map(|(_, leads)| leads)
        .expect("a lead for anyone");
    let lead = pick(leads, seed);
    let offered = offers(world, kit, who, spoken_today(world, who));
    let topics = offered
        .iter()
        .map(|offer| offer.topic.as_str())
        .collect::<Vec<_>>();
    let line = match topics.as_slice() {
        [a, b, c] => format!(
            "{lead} {}",
            pick(&ENDS, seed / 2)
                .replace("{a}", a)
                .replace("{b}", b)
                .replace("{c}", c)
        ),
        _ => lead.to_string(),
    };
    Reply {
        offers: offered.into_iter().map(|offer| offer.question).collect(),
        ..Reply::said(line)
    }
}
