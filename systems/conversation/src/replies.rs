//! What someone answers, from how their life stands: how another is,
//! what they remember of the player, how the two of them stand, their
//! friends, family and worries, and the reply to whatever was heard.

use crate::*;

/// What someone answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub line: String,
    /// The choice they ask for, when the answer asks for something.
    pub asks_for: Option<String>,
    /// What they offer to be asked instead, when they cannot answer what
    /// was asked: three questions, as the player could say them.
    pub offers: Vec<String>,
}

impl Reply {
    /// A plain answer, asking for nothing and offering nothing.
    pub fn said(line: impl Into<String>) -> Self {
        Self {
            line: line.into(),
            asks_for: None,
            offers: Vec::new(),
        }
    }
}

pub(crate) fn pick<'a>(lines: &[&'a str], seed: u64) -> &'a str {
    lines[(seed % lines.len() as u64) as usize]
}

/// How many times the player has spoken to someone today.
pub(crate) fn spoken_today(world: &World, who: EntityId) -> u64 {
    let now = world.world_time();
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time == now)
        .filter(|event| event.kind == SPOKEN && event.targets.first() == Some(&who))
        .count() as u64
}

pub(crate) fn worst_need(state: &WorldState, who: EntityId) -> (Need, i64) {
    Need::ALL
        .into_iter()
        .map(|need| (need, lives::lack(state, who, need)))
        .max_by_key(|(_, lack)| *lack)
        .unwrap_or((Need::Company, 0))
}

pub(crate) fn first_trait(state: &WorldState, who: EntityId) -> String {
    lives::traits(state, who)
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// What advice to make up with someone comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Advice {
    Takes,
    AlreadyFine,
    Refuses,
    Waiting,
}

pub(crate) fn advice(state: &WorldState, kit: &Kit, who: EntityId, other: EntityId) -> Advice {
    if lives::opinion(state, who, other) > -20 {
        Advice::AlreadyFine
    } else if lives::regard(state, who) < 0 {
        Advice::Refuses
    } else if integer(state, who, &nudged_key(other))
        .is_some_and(|at| period(state, kit) - at < NUDGE_PERIODS)
    {
        Advice::Waiting
    } else {
        Advice::Takes
    }
}

pub(crate) fn hurt_recently(state: &WorldState, kit: &Kit, who: EntityId) -> bool {
    integer(state, who, HURT).is_some_and(|at| period(state, kit) - at <= HURT_PERIODS)
}

pub(crate) fn how_is(
    world: &World,
    kit: &Kit,
    who: EntityId,
    other: EntityId,
    seed: u64,
) -> String {
    let state = world.state();
    let x = lives::first_name(state, other);
    if other == who {
        return lives::how_are_you(world, who).unwrap_or_else(|| (kit.place_mood)(world));
    }
    if lives::gone(state, other) {
        return if lives::opinion(state, who, other) < -20 {
            format!("{x}'s gone, and I can't say I miss them.")
        } else {
            format!("{x} left. I miss them.")
        };
    }
    if lives::opinion(state, who, other) <= -30 {
        return pick(
            &[
                "Ask them yourself.",
                "Why would I know?",
                "Not my business.",
            ],
            seed,
        )
        .replace("them", &x);
    }
    if let Some(partner) = lives::partner(state, other) {
        if partner == who {
            return format!("{x}? We're good, thanks for asking.");
        }
        let p = lives::first_name(state, partner);
        return if lives::opinion(state, other, partner) > 5 {
            format!("{x}'s happy with {p}.")
        } else {
            format!("{x} and {p} are having a hard time.")
        };
    }
    let (need, lack) = worst_need(state, other);
    if lack >= 70 {
        return match need {
            Need::Money => format!("{x}'s worried about money."),
            Need::Rest => format!("{x}'s worn out, poor thing."),
            Need::Company => format!("{x} seems lonely lately."),
            Need::Purpose => format!("{x}'s restless, looking for something to do."),
        };
    }
    if lives::opinion(state, who, other) >= 50 {
        format!("{x}'s doing well. One of the best, {x}.")
    } else {
        format!("{x}'s doing alright, I think.")
    }
}

pub(crate) fn greeting(state: &WorldState, who: EntityId, seed: u64) -> String {
    if lives::regard(state, who) < -20 {
        return pick(&["What do you want?", "Oh. It's you."], seed).into();
    }
    let lines: &[&str] = match first_trait(state, who).as_str() {
        "warm" => &["Hello, love!", "Oh, it's you! Hello."],
        "prickly" => &["What is it?", "Hm. Hello."],
        "proud" => &["Good day to you.", "Ah. Hello."],
        "shy" => &["Oh! Hi.", "Um, hello."],
        "sociable" => &["There you are! Hello!", "Hey, good to see you!"],
        "restless" => &["Hi! Can't stop long.", "Hey. What's up?"],
        "steady" => &["Hello there.", "Good to see you."],
        "generous" => &["Hello, friend!", "Hello! Come and sit."],
        "thrifty" => &["Hello.", "Oh, hello."],
        "dreamy" => &["Oh, hello. I was miles away.", "Hello! Lovely light today."],
        _ => &["Hello.", "Hi there."],
    };
    pick(lines, seed).into()
}

pub(crate) const INVITED: &str = "conversation.invited";
/// How far back someone remembers what the player said, in periods.
pub(crate) const MEMORY_PERIODS: i64 = 30;

/// When something happened, as someone would say it.
pub(crate) fn when(kit: &Kit, periods: i64) -> String {
    match periods {
        ..=0 => "earlier".into(),
        1 if kit.unit == "day" => "yesterday".into(),
        1 => format!("last {}", kit.unit),
        2..=6 => "the other day".into(),
        7..=13 => "last week".into(),
        _ => "a while back".into(),
    }
}

/// The player's earlier exchanges with someone, latest first, back as far
/// as they remember.
pub(crate) fn remembered<'a>(world: &'a World, kit: &Kit, who: EntityId) -> Vec<&'a Event> {
    let index = world.history_index();
    let period = kit.period.max(1);
    let since = world
        .world_time()
        .saturating_sub(period.saturating_mul(MEMORY_PERIODS as u64));
    index
        .changes_of(who)
        .iter()
        .rev()
        .filter_map(|id| world.event(*id))
        .take_while(|event| event.world_time >= since)
        .filter(|event| is_talk(event) && event.targets.first() == Some(&who))
        .collect()
}

pub(crate) fn payload_text<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    match event.payload.get(key) {
        Some(Value::Text(text)) => Some(text.as_str()),
        _ => None,
    }
}

/// What someone was given, from the words it came with.
pub(crate) fn gift_of(words: &str) -> &'static str {
    let text = normal(words);
    [
        ("flower", "the flowers"),
        ("cake", "the cake"),
        ("bread", "the bread"),
        ("book", "the book"),
        ("fish", "the fish"),
        ("wine", "the wine"),
        ("tea", "the tea"),
        ("花", "the flowers"),
    ]
    .into_iter()
    .find(|(word, _)| text.contains(word))
    .map(|(_, gift)| gift)
    .unwrap_or("the present")
}

/// What was done together, from the words it came with.
pub(crate) fn outing_of(words: &str) -> &'static str {
    let text = normal(words);
    [
        ("drink", "that drink"),
        ("walk", "our walk"),
        ("lunch", "lunch"),
        ("dinner", "dinner"),
        ("coffee", "that coffee"),
        ("tea", "that cup of tea"),
        ("散步", "our walk"),
        ("喝", "that drink"),
    ]
    .into_iter()
    .find(|(word, _)| has(&text, word) || (!word.is_ascii() && text.contains(word)))
    .map(|(_, outing)| outing)
    .unwrap_or("our time together")
}

/// What someone says about something the player did for them, with
/// `{ago}` where when it was goes: an answer to something they asked, a
/// thing the player made that they saw, a present, an evening out, or one
/// of the Pack's own moments.
pub(crate) fn doing_line(world: &World, kit: &Kit, who: EntityId, event: &Event) -> Option<String> {
    // Only what was theirs, or done to them, is theirs to bring up.
    if event.actor != Some(who) && event.targets.last() != Some(&who) {
        return None;
    }
    if let Some(line) = (kit.recalled)(world, event, who) {
        return Some(line);
    }
    let state = world.state();
    let text = |key: &str| payload_text(event, key).unwrap_or_default().to_string();
    match event.kind.as_str() {
        "situation_answered" if event.actor == Some(who) => {
            let b = event
                .targets
                .first()
                .map(|b| lives::first_name(state, *b))
                .unwrap_or_default();
            let line = match (text("kind").as_str(), text("answer").as_str()) {
                ("feud", "mend") => {
                    format!("You got {b} and me talking {{ago}}. We're better for it.")
                }
                ("feud", "side") => {
                    format!("You took my side against {b} {{ago}}. I won't forget it.")
                }
                ("sweet", "ask") => {
                    format!("You told me to go for it with {b} {{ago}}. I'm glad I did.")
                }
                ("learn", "teach") => format!(
                    "Those lessons with {b} you set up {{ago}}? I'm getting the hang of it."
                ),
                ("short", "fund") | ("short", "friend") => {
                    "You helped me out {ago} when I was short. I'm back on my feet.".into()
                }
                ("lonely", "invite") => {
                    "You took me out {ago} when I was low. It meant a lot.".into()
                }
                ("worn", "rest") => "That day off you gave me {ago}. I needed it.".into(),
                ("worn", "push") => "You told me to push on {ago}. I'm still tired.".into(),
                ("party", "party") => "That party {ago}! My feet still ache.".into(),
                ("leaving", "stay") => "You asked me to stay {ago}. I'm glad I did.".into(),
                ("rough", "talk") => {
                    format!("You helped {b} and me talk {{ago}}. We're all right now.")
                }
                ("confide", "keep") => {
                    "You kept what I told you {ago} to yourself. Thank you.".into()
                }
                ("confide", "share") => {
                    "I've been thinking about what you said {ago}. Maybe I will share it.".into()
                }
                ("keepsake", _) => format!("Do you still have {}?", text("keepsake")),
                ("cold", "sorry") => "You said sorry {ago}. That took something.".into(),
                _ => return None,
            };
            Some(line)
        }
        "reacted" if event.actor == Some(who) => {
            let thing = text("thing").to_lowercase();
            let place = text("place");
            Some(match text("deed").as_str() {
                "built_by_hand" => {
                    format!("The {thing} you built by {place} {{ago}}. I use it most days.")
                }
                "decorated_by_hand" => {
                    format!("The {thing} you put up at {place} {{ago}} cheered everyone.")
                }
                "planted_by_hand" => {
                    format!("What you planted by {place} {{ago}} is coming along.")
                }
                _ => return None,
            })
        }
        "gift_given" if event.targets.last() == Some(&who) => {
            Some("Thank you again for the present {ago}.".into())
        }
        "invited_out" if event.targets.last() == Some(&who) => {
            Some("I enjoyed our evening out {ago}.".into())
        }
        _ => None,
    }
}

/// The latest thing the player did for someone that they would bring up,
/// from before today, within what they remember: when, and the line.
pub(crate) fn latest_doing(world: &World, kit: &Kit, who: EntityId) -> Option<(i64, String)> {
    let state = world.state();
    let now = period(state, kit);
    let period_len = kit.period.max(1);
    let since = world
        .world_time()
        .saturating_sub(period_len.saturating_mul(MEMORY_PERIODS as u64));
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time >= since)
        .filter(|event| ((event.world_time / period_len) as i64) < now)
        .find_map(|event| {
            let then = (event.world_time / period_len) as i64;
            Some((then, doing_line(world, kit, who, event)?))
        })
}

/// Something the player said or did before that someone brings up again:
/// the latest of what they remember that is worth mentioning, from before
/// today.
pub fn recollection(world: &World, kit: &Kit, who: EntityId) -> Option<String> {
    let now = period(world.state(), kit);
    let said = recollection_of_words(world, kit, who);
    let done = latest_doing(world, kit, who);
    match (said, done) {
        (Some((said_at, said)), Some((done_at, _))) if said_at >= done_at => Some(said),
        (_, Some((done_at, line))) => Some(line.replace("{ago}", &when(kit, now - done_at))),
        (Some((_, said)), None) => Some(said),
        (None, None) => None,
    }
}

/// Something the player said before that someone brings up again, and
/// when.
pub(crate) fn recollection_of_words(
    world: &World,
    kit: &Kit,
    who: EntityId,
) -> Option<(i64, String)> {
    let state = world.state();
    let now = period(state, kit);
    remembered(world, kit, who).into_iter().find_map(|event| {
        let then = (event.world_time / kit.period.max(1)) as i64;
        if then >= now {
            return None;
        }
        let ago = when(kit, now - then);
        let about = match event.payload.get("about") {
            Some(Value::Entity(about)) => Some(*about),
            _ => None,
        };
        let words = payload_text(event, "words").unwrap_or_default();
        let line = match Intent::from_id(payload_text(event, "intent")?)? {
            Intent::Rude if hurt_recently(state, kit, who) => {
                Some(format!("I haven't forgotten what you said {ago}."))
            }
            Intent::Gift => Some(format!("Thank you again for {} {ago}.", gift_of(words))),
            Intent::Comfort => Some(format!("What you said {ago} helped. Thank you.")),
            Intent::Invite if payload_text(event, "accepted").is_some() => {
                Some(format!("I enjoyed {} {ago}.", outing_of(words)))
            }
            Intent::Reconcile => {
                let other = about?;
                let x = lives::first_name(state, other);
                Some(if lives::opinion(state, who, other) > -20 {
                    format!("I did talk to {x}, like you said. We're better for it.")
                } else {
                    format!("I tried with {x}, like you said {ago}. It's slow going.")
                })
            }
            Intent::HowIs | Intent::Quarrel => {
                let other = about?;
                let x = lives::first_name(state, other);
                Some(format!(
                    "You asked after {x} {ago}. {}",
                    how_is(world, kit, who, other, lives::mix(&[who.0, now as u64]))
                ))
            }
            Intent::Compliment => Some(format!("What you said {ago} was kind. It stayed with me.")),
            _ => None,
        };
        line.map(|line| (then, line))
    })
}

/// How someone stands with the player: a mark from -2 to 2 and a few words
/// a screen can show beside them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub level: i8,
    pub words: String,
}

/// How someone stands with the player, from how they feel about the
/// player's hand in the place and whether they were hurt lately.
pub fn standing(state: &WorldState, kit: &Kit, who: EntityId) -> Standing {
    let regard = lives::regard(state, who);
    let (level, words) = if hurt_recently(state, kit, who) {
        (-1_i8.min(regard_level(regard)), "Hurt by what you said")
    } else {
        match regard_level(regard) {
            2 => (2, "Thinks the world of you"),
            1 => (1, "Likes you"),
            0 => (0, "Getting to know you"),
            -1 => (-1, "Wary of you"),
            _ => (-2, "Doesn't trust you"),
        }
    };
    Standing {
        level,
        words: words.into(),
    }
}

pub(crate) fn regard_level(regard: i64) -> i8 {
    match regard {
        50.. => 2,
        20..=49 => 1,
        -19..=19 => 0,
        -49..=-20 => -1,
        _ => -2,
    }
}

/// Whether someone would go for a drink or a walk with the player now.
pub(crate) fn accepts_invite(state: &WorldState, kit: &Kit, who: EntityId) -> bool {
    !hurt_recently(state, kit, who)
        && lives::regard(state, who) > -20
        && lives::lack(state, who, Need::Rest) < 70
}

/// The last change between two people, as the later of them would tell it:
/// what kind of change it was, the line its first person said, and when.
pub(crate) fn last_bond(world: &World, who: EntityId, other: EntityId) -> Option<&Event> {
    let index = world.history_index();
    index
        .changes_of(who)
        .iter()
        .rev()
        .filter_map(|id| world.event(*id))
        .find(|event| {
            event.kind == "bond_changed"
                && ((event.actor == Some(who) && event.targets.first() == Some(&other))
                    || (event.actor == Some(other) && event.targets.first() == Some(&who)))
        })
}

/// What last changed between two people, told by one of them, if it
/// changed within what they remember.
pub(crate) fn between(world: &World, kit: &Kit, who: EntityId, other: EntityId) -> Option<String> {
    let state = world.state();
    let bond = last_bond(world, who, other)?;
    let ago = period(state, kit) - (bond.world_time / kit.period.max(1)) as i64;
    if ago > MEMORY_PERIODS {
        return None;
    }
    let when = when(kit, ago);
    Some(match payload_text(bond, "bond")? {
        "made_up" => format!("We made it up {when}."),
        "fell_out" => format!("We fell out {when}."),
        "became_friends" => "We've grown close lately.".into(),
        "drifted" => "We've drifted, though.".into(),
        _ => return None,
    })
}

pub(crate) fn quarrel(
    world: &World,
    kit: &Kit,
    who: EntityId,
    other: EntityId,
    seed: u64,
) -> String {
    let state = world.state();
    let x = lives::first_name(state, other);
    let view = lives::opinion(state, who, other);
    let bond = last_bond(world, who, other);
    let kind = bond.and_then(|event| payload_text(event, "bond"));
    let ago = bond
        .map(|event| {
            when(
                kit,
                period(state, kit) - (event.world_time / kit.period.max(1)) as i64,
            )
        })
        .unwrap_or_default();
    if view > -20 {
        return match kind {
            Some("made_up") => format!("Angry? Not any more. {x} and I made it up {ago}."),
            _ => pick(
                &[
                    "Angry at {x}? No, we're fine.",
                    "Me and {x}? There's nothing wrong there.",
                ],
                seed,
            )
            .replace("{x}", &x),
        };
    }
    let own = bond
        .filter(|event| event.actor == Some(who))
        .and_then(|event| payload_text(event, "said"));
    match (kind, own) {
        (Some("fell_out"), Some(said)) => format!("We fell out {ago}. {said}"),
        (Some("fell_out"), None) => format!("{x} and I fell out {ago}. I'd rather not go over it."),
        (Some("drifted"), _) => format!("We just drifted, {x} and I. It happens."),
        _ => pick(
            &[
                "{x} knows what they did.",
                "It's between me and {x}.",
                "Ask {x}. I've said all I'm going to.",
            ],
            seed,
        )
        .replace("{x}", &x),
    }
}

pub(crate) fn friends(world: &World, kit: &Kit, who: EntityId, seed: u64) -> String {
    let state = world.state();
    let others = (kit.people)(state)
        .into_iter()
        .filter(|person| *person != who && !lives::gone(state, *person))
        .collect::<Vec<_>>();
    let best = others
        .iter()
        .copied()
        .filter(|other| lives::opinion(state, who, *other) >= 20)
        .max_by_key(|other| lives::opinion(state, who, *other));
    let worst = others
        .iter()
        .copied()
        .filter(|other| lives::opinion(state, who, *other) <= -20)
        .min_by_key(|other| lives::opinion(state, who, *other));
    let name = |id: EntityId| lives::first_name(state, id);
    match (best, worst) {
        (Some(best), Some(worst)) => format!(
            "{}'s my closest. {} and I don't get on.",
            name(best),
            name(worst)
        ),
        (Some(best), None) => pick(
            &["{b}, without a doubt.", "I'd trust {b} with anything."],
            seed,
        )
        .replace("{b}", &name(best)),
        (None, Some(worst)) => format!(
            "Not many, if I'm honest. And {} I could do without.",
            name(worst)
        ),
        (None, None) => "I get on with most people. Nobody close, though.".into(),
    }
}

pub(crate) const TRAIT_WORDS: &[(&str, &str)] = &[
    ("warm", "warm"),
    ("prickly", "prickly"),
    ("proud", "proud"),
    ("shy", "shy"),
    ("sociable", "a talker"),
    ("restless", "restless"),
    ("steady", "steady"),
    ("generous", "generous"),
    ("thrifty", "careful with money"),
    ("dreamy", "a dreamer"),
];

pub(crate) fn about_you(world: &World, kit: &Kit, who: EntityId) -> String {
    let state = world.state();
    let described = lives::traits(state, who)
        .iter()
        .filter_map(|word| {
            TRAIT_WORDS
                .iter()
                .find(|(id, _)| id == word)
                .map(|(_, said)| *said)
        })
        .collect::<Vec<_>>();
    let mut line = format!("I'm {}.", lives::first_name(state, who));
    match described.as_slice() {
        [] => {}
        [one] => line.push_str(&format!(" {}, people say.", capitalized(one))),
        [first, second, ..] => line.push_str(&format!(
            " {} and {second}, people say.",
            capitalized(first)
        )),
    }
    if let Some(work) = (kit.work_line)(world, who) {
        line.push_str(&format!(" {work}"));
    }
    line
}

pub(crate) fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

pub(crate) fn family(world: &World, kit: &Kit, who: EntityId, seed: u64) -> String {
    let state = world.state();
    match lives::partner(state, who) {
        Some(loved) if lives::opinion(state, who, loved) > 5 => pick(
            &[
                "There's {p}. That's family enough for me.",
                "{p} and me. I wouldn't change it.",
            ],
            seed,
        )
        .replace("{p}", &lives::first_name(state, loved)),
        Some(loved) => format!(
            "There's {}. We're having a hard time.",
            lives::first_name(state, loved)
        ),
        None => match pick(
            &[
                "No one of my own. {s} is family enough.",
                "Just me. Everyone in {s} looks out for each other, though.",
            ],
            seed,
        ) {
            // Begun with the place's name, the sentence begins with a
            // capital.
            line if line.contains(". {s}") => line.replace("{s}", &capitalized(kit.settlement)),
            line => line.replace("{s}", kit.settlement),
        },
    }
}

pub(crate) fn worry(world: &World, kit: &Kit, who: EntityId) -> String {
    let state = world.state();
    if hurt_recently(state, kit, who) {
        return "You, a bit, after what you said.".into();
    }
    let (need, lack) = worst_need(state, who);
    if lack >= 60 {
        return match need {
            Need::Money => "Money. There's never enough of it.",
            Need::Rest => "I'm just so tired. I can't seem to catch up.",
            Need::Company => "Being on my own so much. The evenings are long.",
            Need::Purpose => "Having nothing to do with myself. It gnaws at me.",
        }
        .into();
    }
    let foe = (kit.people)(state)
        .into_iter()
        .filter(|other| *other != who && lives::opinion(state, who, *other) <= -30)
        .min_by_key(|other| lives::opinion(state, who, *other));
    match foe {
        Some(foe) => format!(
            "Only {}. The rest I can manage.",
            lives::first_name(state, foe)
        ),
        None => "Not much, honestly. I'm alright.".into(),
    }
}

/// Whether two lines mention the same one of these people.
pub(crate) fn same_person(state: &WorldState, people: &[EntityId], a: &str, b: &str) -> bool {
    people.iter().any(|person| {
        let name = lives::first_name(state, *person);
        name.chars().count() >= 2 && a.contains(&name) && b.contains(&name)
    })
}

/// How someone answers what was heard, in their own voice, from how their
/// life stands now.
pub fn reply(world: &World, kit: &Kit, who: EntityId, heard: Heard) -> Reply {
    let state = world.state();
    let seed = lives::mix(&[who.0, period(state, kit) as u64, spoken_today(world, who)]);
    let trait_ = first_trait(state, who);
    let line: fn(String) -> Reply = Reply::said;
    let about_name = heard.about.map(|about| lives::first_name(state, about));
    let mut answer = match heard.intent {
        Intent::Greet => line(greeting(state, who, seed)),
        Intent::HowAreYou => {
            let base =
                lives::how_are_you_on(world, who, seed).unwrap_or_else(|| (kit.place_mood)(world));
            let people = (kit.people)(state);
            match lives::said_today(world, who) {
                Some(today) if today != base && !same_person(state, &people, &base, &today) => {
                    line(format!("{base} {today}"))
                }
                _ => line(base),
            }
        }
        Intent::Day => line(match lives::said_today(world, who) {
            Some(today) => format!(
                "{}{today}",
                pick(&["Not bad. ", "Oh, you know. ", "Busy enough. "], seed)
            ),
            None => (kit.work_line)(world, who)
                .map(|work| format!("The usual. {work}"))
                .unwrap_or_else(|| format!("Quiet. Nothing much to tell, this {}.", kit.unit)),
        }),
        Intent::AboutYou => line(about_you(world, kit, who)),
        Intent::Family => line(family(world, kit, who, seed)),
        Intent::Worry => line(worry(world, kit, who)),
        Intent::Weather => line({
            let sky = (kit.weather)(world);
            match trait_.as_str() {
                "dreamy" => format!("{sky} I like it, though."),
                "prickly" => format!("{sky} As if it could be anything else here."),
                _ => sky,
            }
        }),
        Intent::Coming => line(coming(world, kit, who)),
        Intent::Gift => line(
            if lives::regard(state, who) < -20 {
                "Hm. I'll take it. It doesn't change anything."
            } else {
                match trait_.as_str() {
                    "shy" => "For me? Oh. Thank you.",
                    "proud" => "You shouldn't have. But I'm glad you did.",
                    "prickly" => "What's this for? ...Thank you.",
                    "thrifty" => "You spent money on me? Thank you.",
                    _ => pick(
                        &[
                            "For me? You shouldn't have!",
                            "Oh, that's lovely. Thank you.",
                        ],
                        seed,
                    ),
                }
            }
            .into(),
        ),
        Intent::Friends => line(friends(world, kit, who, seed)),
        Intent::Quarrel => match heard.about {
            Some(other) => line(quarrel(world, kit, who, other, seed)),
            None => line("With who?".into()),
        },
        Intent::Invite => line(if accepts_invite(state, kit, who) {
            match trait_.as_str() {
                "shy" => "Oh! Alright. Yes, I'd like that.",
                "restless" => "Yes! Anything to get out.",
                "thrifty" => "Go on, then. If you're buying.",
                "prickly" => "Fine. Just the one.",
                _ => pick(&["I'd love that.", "Go on, then. Why not?"], seed),
            }
            .to_string()
        } else if lives::lack(state, who, Need::Rest) >= 70 {
            "Another time. I'm dead on my feet.".into()
        } else {
            "I'd rather not.".into()
        }),
        Intent::Ack => line(
            match trait_.as_str() {
                "prickly" => "Mm.",
                "sociable" => pick(&["Anyway! Where was I?", "So there we are."], seed),
                "shy" => "Mm-hm.",
                _ => pick(&["Right.", "Mm.", "So there we are."], seed),
            }
            .into(),
        ),
        Intent::Standing => line({
            let standing = standing(state, kit, who);
            match standing.level {
                2 => "You? You're one of us now.".into(),
                1 => "I like you. You've done right by us.".to_string(),
                0 => "You're alright. I'm still getting to know you.".into(),
                -1 if hurt_recently(state, kit, who) => "Not after what you said.".into(),
                -1 => "Honestly? You've let me down.".into(),
                _ => "I don't trust you.".into(),
            }
        }),
        Intent::HowIs => match heard.about {
            Some(other) => line(how_is(world, kit, who, other, seed)),
            None => line("Who do you mean?".into()),
        },
        Intent::ThinkOf => match heard.about {
            Some(other) if lives::gone(state, other) => line(how_is(world, kit, who, other, seed)),
            Some(other) => line({
                let view = lives::thinks_of(world, who, other).unwrap_or_else(|| {
                    format!(
                        "{}? I don't really know them yet.",
                        lives::first_name(state, other)
                    )
                });
                match between(world, kit, who, other) {
                    Some(history) => format!("{view} {history}"),
                    None => view,
                }
            }),
            None => line("Who do you mean?".into()),
        },
        Intent::News => line(news(world, kit, seed)),
        Intent::Need => {
            let (text, asks_for) = (kit.need_line)(world, who);
            Reply {
                asks_for,
                ..Reply::said(text)
            }
        }
        Intent::Work => line(
            (kit.work_line)(world, who)
                .or_else(|| lives::said_today(world, who))
                .unwrap_or_else(|| "Same as ever.".into()),
        ),
        Intent::Place => match heard.about {
            Some(place) => line((kit.place_line)(world, who, place)),
            None => line((kit.place_mood)(world)),
        },
        Intent::Compliment => line(
            if lives::regard(state, who) < -20 {
                "Hm. Words are cheap."
            } else {
                match trait_.as_str() {
                    "shy" => "Oh! That's kind of you.",
                    "proud" => "I know. But thank you.",
                    "prickly" => "Flattery won't get you far. But thanks.",
                    "warm" | "generous" => "Oh, you're a sweetheart.",
                    "dreamy" => "Do you think so? That's lovely.",
                    _ => pick(&["Thank you!", "That's nice to hear."], seed),
                }
            }
            .into(),
        ),
        Intent::Thank => line(
            match trait_.as_str() {
                "prickly" => "Mm. Don't make a habit of it.",
                "warm" | "generous" => "Any time, you know that.",
                _ => pick(&["Any time.", "Don't mention it.", "It's nothing."], seed),
            }
            .into(),
        ),
        Intent::Comfort => {
            let (need, lack) = worst_need(state, who);
            line(
                if lack >= 60 {
                    match need {
                        Need::Money => "Thank you. It's the money, mostly. It'll come right.",
                        Need::Rest => "Thanks. I just need a proper night's sleep.",
                        Need::Company => "That means a lot. It helps, having someone to talk to.",
                        Need::Purpose => "Thanks. I'll find something to do with myself.",
                    }
                } else {
                    "I'm alright, really. But thank you."
                }
                .into(),
            )
        }
        Intent::Apologize => line(
            if hurt_recently(state, kit, who) {
                if trait_ == "prickly" || trait_ == "proud" {
                    "Fine. Apology accepted."
                } else {
                    "Alright. Let's forget it."
                }
            } else {
                "What for? You've done nothing wrong."
            }
            .into(),
        ),
        Intent::Rude => line(
            if lives::regard(state, who) <= -30 {
                "Leave me alone."
            } else {
                match trait_.as_str() {
                    "prickly" => "Charming. Same to you.",
                    "proud" => "I won't dignify that.",
                    "shy" => "Oh. Right.",
                    "warm" => "That's unkind. I thought better of you.",
                    _ => "There's no need for that.",
                }
            }
            .into(),
        ),
        Intent::Reconcile => match (heard.about, about_name) {
            (Some(other), Some(x)) => line(match advice(state, kit, who, other) {
                Advice::AlreadyFine => format!("{x} and I are fine, honestly."),
                Advice::Refuses => "Mind your own business.".into(),
                Advice::Waiting => format!("I said I'd talk to {x}. Give it time."),
                Advice::Takes => format!("Maybe you're right. I'll talk to {x}."),
            }),
            _ => line("Make up with who?".into()),
        },
        Intent::Farewell => line(
            if trait_ == "prickly" {
                "Right. Bye."
            } else {
                pick(&["See you.", "Take care.", "Bye now."], seed)
            }
            .into(),
        ),
        Intent::Unclear => grounded::deflection(world, kit, who, seed),
    };
    // Seeing the player for the first time today, people bring up what
    // was said before.
    if spoken_today(world, who) == 0
        && matches!(
            heard.intent,
            Intent::Greet | Intent::HowAreYou | Intent::Day
        )
    {
        if let Some(recalled) = recollection(world, kit, who) {
            if answer.line.chars().count() + recalled.chars().count() < MOST_REPLY {
                answer.line = format!("{} {recalled}", answer.line);
            }
        }
    }
    let offered = answer.line.clone();
    favour::answer(world, kit, who, heard, &mut answer);
    if !answer.line.starts_with(&offered) {
        answer.offers.clear();
    }
    answer
}

/// What is coming up on the calendar, as someone looks forward to it.
pub(crate) fn coming(world: &World, kit: &Kit, who: EntityId) -> String {
    match (kit.coming_up)(world) {
        Some(coming) => format!(
            "{}{}",
            capitalized(&coming),
            match first_trait(world.state(), who).as_str() {
                "shy" => ". I might go, if it's not too crowded.",
                "sociable" | "warm" => ". I wouldn't miss it!",
                "prickly" => ". I suppose I'll show my face.",
                _ => ". I'll be there.",
            }
        ),
        None => "Nothing for a while. Just the usual.".into(),
    }
}

/// The place's news of the last few periods, and what is coming up.
pub(crate) fn news(world: &World, kit: &Kit, seed: u64) -> String {
    let since = world
        .world_time()
        .saturating_sub(kit.period.max(1).saturating_mul(5));
    let news = lives::news_since(world, since);
    let latest = news.iter().rev().take(2).rev().cloned().collect::<Vec<_>>();
    let coming = (kit.coming_up)(world)
        .map(|coming| format!(" And {coming}!"))
        .unwrap_or_default();
    if latest.is_empty() {
        format!(
            "Nothing much. It's been a quiet {} or two in {}.{coming}",
            kit.unit, kit.settlement
        )
    } else {
        format!(
            "{}{}{coming}",
            pick(
                &["Have you heard? ", "News? Well. ", "You won't believe it. "],
                seed
            ),
            latest
                .iter()
                .map(|told| format!("{}.", told.trim_end_matches('.')))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}
