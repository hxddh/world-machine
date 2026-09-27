//! Talking to people in the player's own words.
//!
//! The player types whatever they like to someone. This System hears it as
//! one of a small, closed set of things people say to each other (asking
//! how someone is, asking about someone else, a kindness, an insult, some
//! advice), answers in that person's voice from how their life stands, and
//! records the exchange as an Action like any other. What the words do to
//! the World is decided by the rules here from what was heard, never by the
//! words themselves, and replaying a World never hears anything again: the
//! recorded Event carries what was heard and what was answered.
//!
//! Hearing is done here by listening for words and names, which needs no
//! model. A Pack may hear and answer some other way, for instance with a
//! language model when the player has switched one on; it then proposes
//! the same Action with the same closed set of meanings, and the rules
//! still decide what follows.

use lives::Need;
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, Event, EventDraft, EventId,
    StateChange, Value, World, WorldState,
};

/// The kind of Event an exchange is recorded as.
pub const SPOKEN: &str = "spoken";

/// The longest thing the player may say at once, in characters.
pub const MOST_WORDS: usize = 280;

/// The longest answer that is recorded, in characters.
pub const MOST_REPLY: usize = 600;

/// What someone was heard to say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Greet,
    HowAreYou,
    /// How someone else is doing.
    HowIs,
    /// What they think of someone else.
    ThinkOf,
    News,
    /// What they need.
    Need,
    Work,
    /// About one of the World's places.
    Place,
    Compliment,
    Thank,
    Comfort,
    Apologize,
    Rude,
    /// Advice to make up with someone.
    Reconcile,
    Farewell,
    Unclear,
}

impl Intent {
    pub const ALL: [Intent; 16] = [
        Intent::Greet,
        Intent::HowAreYou,
        Intent::HowIs,
        Intent::ThinkOf,
        Intent::News,
        Intent::Need,
        Intent::Work,
        Intent::Place,
        Intent::Compliment,
        Intent::Thank,
        Intent::Comfort,
        Intent::Apologize,
        Intent::Rude,
        Intent::Reconcile,
        Intent::Farewell,
        Intent::Unclear,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Intent::Greet => "greet",
            Intent::HowAreYou => "how_are_you",
            Intent::HowIs => "how_is",
            Intent::ThinkOf => "think_of",
            Intent::News => "news",
            Intent::Need => "need",
            Intent::Work => "work",
            Intent::Place => "place",
            Intent::Compliment => "compliment",
            Intent::Thank => "thank",
            Intent::Comfort => "comfort",
            Intent::Apologize => "apologize",
            Intent::Rude => "rude",
            Intent::Reconcile => "reconcile",
            Intent::Farewell => "farewell",
            Intent::Unclear => "unclear",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|intent| intent.id() == id)
    }

    /// Whether it is about somebody else, who must then be named.
    pub fn about_someone(self) -> bool {
        matches!(self, Intent::HowIs | Intent::ThinkOf | Intent::Reconcile)
    }
}

/// What was heard: what the player meant, and whom or where it was about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Heard {
    pub intent: Intent,
    pub about: Option<EntityId>,
}

/// Everything the System needs to know about a World's people and places.
#[derive(Clone, Debug)]
pub struct Kit {
    /// How much world time one period is.
    pub period: u64,
    /// What a period is called: "day", "sol".
    pub unit: &'static str,
    /// What the place as a whole is called: "the harbour".
    pub settlement: &'static str,
    /// Who can be spoken to: everyone living there now.
    pub people: fn(&WorldState) -> Vec<EntityId>,
    /// The places people can be asked about.
    pub places: fn(&WorldState) -> Vec<EntityId>,
    /// What someone says about a place.
    pub place_line: fn(&World, EntityId, EntityId) -> String,
    /// What someone says they need, and the choice that would grant it, if
    /// one is on offer.
    pub need_line: fn(&World, EntityId) -> (String, Option<String>),
    /// What someone says about their work, if they have any.
    pub work_line: fn(&World, EntityId) -> Option<String>,
    /// How the place as a whole is doing, in anybody's words.
    pub place_mood: fn(&World) -> String,
}

const TALKED: &str = "conversation.talked";
const WARMED: &str = "conversation.warmed";
const HURT: &str = "conversation.hurt";
/// How long an unkind word is remembered, in periods.
const HURT_PERIODS: i64 = 10;
/// How long someone takes to act on advice before being asked again.
const NUDGE_PERIODS: i64 = 7;

fn nudged_key(other: EntityId) -> String {
    format!("conversation.nudged.{other}")
}

fn integer(state: &WorldState, entity: EntityId, key: &str) -> Option<i64> {
    match state.entity(entity)?.component(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

fn period(state: &WorldState, kit: &Kit) -> i64 {
    (state.world_time() / kit.period.max(1)) as i64
}

/// Whether someone can be spoken to now.
pub fn can_talk_to(state: &WorldState, kit: &Kit, who: EntityId) -> bool {
    (kit.people)(state).contains(&who)
}

/// Lowercase words separated by single spaces, with a space at each end,
/// so a phrase is found only as whole words.
fn normal(words: &str) -> String {
    let lower = words.to_lowercase().replace(['\u{2019}', '`'], "'");
    let cleaned = lower
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '\'' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();
    format!(
        " {} ",
        cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
    )
}

/// Whether the words hold a phrase: as whole words for a phrase in
/// letters, anywhere for one in a script written without spaces.
fn has(text: &str, phrase: &str) -> bool {
    if phrase.is_ascii() {
        text.contains(&format!(" {phrase} "))
    } else {
        text.contains(phrase)
    }
}

fn any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| has(text, phrase))
}

const RUDE: &[&str] = &[
    "stupid",
    "idiot",
    "hate you",
    "shut up",
    "useless",
    "lazy",
    "ugly",
    "fool",
    "moron",
    "loser",
    "go away",
    "get lost",
    "pathetic",
    "dumb",
    "you suck",
    "笨",
    "蠢",
    "讨厌你",
    "滚",
    "闭嘴",
    "没用",
];
const SORRY: &[&str] = &[
    "sorry",
    "apologise",
    "apologize",
    "apologies",
    "my fault",
    "forgive me",
    "didn't mean",
    "didnt mean",
    "对不起",
    "抱歉",
    "不好意思",
];
const COMFORT: &[&str] = &[
    "cheer up",
    "don't worry",
    "dont worry",
    "it'll be",
    "it will be",
    "hang in",
    "sorry to hear",
    "sorry about your",
    "sorry for your",
    "here for you",
    "you'll be fine",
    "you will be fine",
    "chin up",
    "you're not alone",
    "别担心",
    "加油",
    "会好的",
    "别难过",
];
const RECONCILE: &[&str] = &[
    "make up",
    "make peace",
    "patch things",
    "patch it up",
    "forgive",
    "talk to",
    "sort it out",
    "apologise to",
    "apologize to",
    "reconcile",
    "和好",
    "原谅",
];
const THANK: &[&str] = &[
    "thank",
    "thanks",
    "thank you",
    "cheers",
    "谢谢",
    "多谢",
    "感谢",
];
const COMPLIMENT: &[&str] = &[
    "well done",
    "good job",
    "great job",
    "lovely",
    "beautiful",
    "wonderful",
    "amazing",
    "brilliant",
    "proud of you",
    "love your",
    "you're great",
    "you are great",
    "nice",
    "fantastic",
    "the best",
    "棒",
    "厉害",
    "好看",
    "漂亮",
    "了不起",
];
const OPINION: &[&str] = &[
    "think of",
    "think about",
    "feel about",
    "opinion",
    "like",
    "get on",
    "get along",
    "觉得",
    "看法",
];
const HOW_IS: &[&str] = &[
    "how is",
    "how's",
    "hows",
    "doing",
    "how are",
    "is ok",
    "alright",
    "怎么样",
    "还好",
];
const HOW_ARE_YOU: &[&str] = &[
    "how are you",
    "how are things",
    "how r u",
    "how you doing",
    "how are you doing",
    "how's it going",
    "hows it going",
    "how is it going",
    "how's life",
    "you ok",
    "you okay",
    "you alright",
    "are you ok",
    "are you okay",
    "feeling",
    "how have you been",
    "what's up",
    "whats up",
    "你好吗",
    "怎么样",
    "还好吗",
    "最近",
];
const NEED: &[&str] = &[
    "need",
    "help",
    "anything i can",
    "can i do",
    "what can i",
    "want",
    "需要",
    "帮",
];
const NEWS: &[&str] = &[
    "news",
    "what's new",
    "whats new",
    "anything new",
    "happened",
    "happening",
    "gossip",
    "heard",
    "going on",
    "新鲜事",
    "发生",
    "消息",
];
const WORK: &[&str] = &[
    "work", "job", "shift", "busy", "working", "today", "工作", "忙",
];
const GREET: &[&str] = &[
    "hi",
    "hello",
    "hey",
    "morning",
    "good morning",
    "evening",
    "good evening",
    "afternoon",
    "yo",
    "howdy",
    "hiya",
    "你好",
    "嗨",
    "早",
];
const FAREWELL: &[&str] = &[
    "bye",
    "goodbye",
    "see you",
    "see ya",
    "later",
    "good night",
    "goodnight",
    "farewell",
    "再见",
    "拜拜",
];

/// Whom, among `candidates`, the words name: by full name, or by first name
/// for people and last word for places ("the bakery").
fn named(
    state: &WorldState,
    text: &str,
    candidates: &[EntityId],
    by_last: bool,
) -> Option<EntityId> {
    candidates.iter().copied().find(|id| {
        let full = normal(&lives::name(state, *id));
        let full = full.trim();
        if full.is_empty() || full == "someone" {
            return false;
        }
        let first = if by_last {
            full.rsplit(' ').next()
        } else {
            full.split(' ').next()
        }
        .unwrap_or(full);
        // Written without spaces, a name runs straight into the words
        // around it.
        let found = |name: &str| has(text, name) || (!text.is_ascii() && text.contains(name));
        found(full) || (first.chars().count() >= 3 && found(first))
    })
}

/// What the player's words to `who` mean.
pub fn hear(state: &WorldState, kit: &Kit, who: EntityId, words: &str) -> Heard {
    let text = normal(words);
    let others = (kit.people)(state)
        .into_iter()
        .filter(|person| *person != who)
        .collect::<Vec<_>>();
    let person = named(state, &text, &others, false);
    let place = named(state, &text, &(kit.places)(state), true);
    let heard = |intent, about| Heard { intent, about };
    if any(&text, RUDE) {
        return heard(Intent::Rude, None);
    }
    if let Some(person) = person {
        if any(&text, RECONCILE) {
            return heard(Intent::Reconcile, Some(person));
        }
    }
    if any(&text, COMFORT) {
        return heard(Intent::Comfort, None);
    }
    if any(&text, SORRY) {
        return heard(Intent::Apologize, None);
    }
    if any(&text, THANK) {
        return heard(Intent::Thank, None);
    }
    if let Some(person) = person {
        if any(&text, OPINION) {
            return heard(Intent::ThinkOf, Some(person));
        }
        if any(&text, HOW_IS) {
            return heard(Intent::HowIs, Some(person));
        }
        return heard(Intent::ThinkOf, Some(person));
    }
    if any(&text, COMPLIMENT) {
        return heard(Intent::Compliment, None);
    }
    if let Some(place) = place {
        return heard(Intent::Place, Some(place));
    }
    if any(&text, HOW_ARE_YOU) {
        return heard(Intent::HowAreYou, None);
    }
    if any(&text, NEED) {
        return heard(Intent::Need, None);
    }
    if any(&text, NEWS) {
        return heard(Intent::News, None);
    }
    if any(&text, WORK) {
        return heard(Intent::Work, None);
    }
    if any(&text, FAREWELL) {
        return heard(Intent::Farewell, None);
    }
    if any(&text, GREET) {
        return heard(Intent::Greet, None);
    }
    heard(Intent::Unclear, None)
}

/// What someone answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub line: String,
    /// The choice they ask for, when the answer asks for something.
    pub asks_for: Option<String>,
}

fn pick<'a>(lines: &[&'a str], seed: u64) -> &'a str {
    lines[(seed % lines.len() as u64) as usize]
}

/// How many times the player has spoken to someone today.
fn spoken_today(world: &World, who: EntityId) -> u64 {
    let now = world.world_time();
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time == now)
        .filter(|event| event.kind == SPOKEN && event.targets.first() == Some(&who))
        .count() as u64
}

fn worst_need(state: &WorldState, who: EntityId) -> (Need, i64) {
    Need::ALL
        .into_iter()
        .map(|need| (need, lives::lack(state, who, need)))
        .max_by_key(|(_, lack)| *lack)
        .unwrap_or((Need::Company, 0))
}

fn first_trait(state: &WorldState, who: EntityId) -> String {
    lives::traits(state, who)
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// What advice to make up with someone comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Advice {
    Takes,
    AlreadyFine,
    Refuses,
    Waiting,
}

fn advice(state: &WorldState, kit: &Kit, who: EntityId, other: EntityId) -> Advice {
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

fn hurt_recently(state: &WorldState, kit: &Kit, who: EntityId) -> bool {
    integer(state, who, HURT).is_some_and(|at| period(state, kit) - at <= HURT_PERIODS)
}

fn how_is(world: &World, kit: &Kit, who: EntityId, other: EntityId, seed: u64) -> String {
    let state = world.state();
    let x = lives::name(state, other);
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
        let p = lives::name(state, partner);
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

fn greeting(state: &WorldState, who: EntityId, seed: u64) -> String {
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

/// How someone answers what was heard, in their own voice, from how their
/// life stands now.
pub fn reply(world: &World, kit: &Kit, who: EntityId, heard: Heard) -> Reply {
    let state = world.state();
    let seed = lives::mix(&[who.0, period(state, kit) as u64, spoken_today(world, who)]);
    let trait_ = first_trait(state, who);
    let line = |text: String| Reply {
        line: text,
        asks_for: None,
    };
    let about_name = heard.about.map(|about| lives::name(state, about));
    match heard.intent {
        Intent::Greet => line(greeting(state, who, seed)),
        Intent::HowAreYou => {
            let base = lives::how_are_you(world, who).unwrap_or_else(|| (kit.place_mood)(world));
            match lives::said_today(world, who) {
                Some(today) if today != base => line(format!("{base} {today}")),
                _ => line(base),
            }
        }
        Intent::HowIs => match heard.about {
            Some(other) => line(how_is(world, kit, who, other, seed)),
            None => line("Who do you mean?".into()),
        },
        Intent::ThinkOf => match heard.about {
            Some(other) if lives::gone(state, other) => line(how_is(world, kit, who, other, seed)),
            Some(other) => line(lives::thinks_of(world, who, other).unwrap_or_else(|| {
                format!(
                    "{}? I don't really know them yet.",
                    lives::name(state, other)
                )
            })),
            None => line("Who do you mean?".into()),
        },
        Intent::News => {
            let since = state
                .world_time()
                .saturating_sub(kit.period.max(1).saturating_mul(5));
            let news = lives::news_since(world, since);
            let latest = news.iter().rev().take(2).rev().cloned().collect::<Vec<_>>();
            if latest.is_empty() {
                line(format!(
                    "Nothing much. It's been a quiet {} or two in {}.",
                    kit.unit, kit.settlement
                ))
            } else {
                line(format!(
                    "{}{}",
                    pick(
                        &["Have you heard? ", "News? Well. ", "You won't believe it. "],
                        seed
                    ),
                    latest.join(" ")
                ))
            }
        }
        Intent::Need => {
            let (text, asks_for) = (kit.need_line)(world, who);
            Reply {
                line: text,
                asks_for,
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
        Intent::Unclear => line(format!(
            "{} Ask me how I am, or about someone here.",
            pick(
                &[
                    "Sorry, I didn't follow.",
                    "Hm? Say that another way?",
                    "I'm not sure what you mean.",
                ],
                seed
            )
        )),
    }
}

/// The Action that records an exchange: the player's words, what they were
/// heard to mean, and the answer.
pub fn request(who: EntityId, words: &str, heard: Heard, reply: &Reply) -> ActionRequest {
    let mut request = ActionRequest::new("conversation_say")
        .arg("who", Value::Entity(who))
        .arg("words", words.trim())
        .arg("intent", heard.intent.id())
        .arg("reply", reply.line.as_str());
    if let Some(about) = heard.about {
        request = request.arg("about", Value::Entity(about));
    }
    if let Some(asks_for) = &reply.asks_for {
        request = request.arg("asks_for", asks_for.as_str());
    }
    request
}

/// Hears the player's words to someone and answers them, ready to record.
pub fn say(world: &World, kit: &Kit, who: EntityId, words: &str) -> Result<ActionRequest, String> {
    if !can_talk_to(world.state(), kit, who) {
        return Err(format!(
            "{} can't be spoken to now",
            lives::name(world.state(), who)
        ));
    }
    let heard = hear(world.state(), kit, who, words);
    let answer = reply(world, kit, who, heard);
    Ok(request(who, words, heard, &answer))
}

/// Registers the Action that records exchanges.
pub fn register_actions(
    actions: &mut ActionRegistry,
    kit: fn(&WorldState) -> Kit,
) -> Result<(), ActionError> {
    actions.register(Says(kit))
}

fn plain(text: &str, most: usize) -> bool {
    let count = text.chars().count();
    count > 0 && count <= most && !text.chars().any(char::is_control)
}

struct Says(fn(&WorldState) -> Kit);

impl Action for Says {
    fn name(&self) -> &'static str {
        "conversation_say"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let kit = (self.0)(state);
        let who = match request.args.get("who") {
            Some(Value::Entity(who)) => *who,
            _ => return Err(ActionError::Invalid("say it to whom?".into())),
        };
        if !can_talk_to(state, &kit, who) {
            return Err(ActionError::Invalid("they can't be spoken to now".into()));
        }
        let text = |key: &str| match request.args.get(key) {
            Some(Value::Text(text)) => Some(text.as_str()),
            _ => None,
        };
        let words = text("words").unwrap_or_default();
        if !plain(words, MOST_WORDS) {
            return Err(ActionError::Invalid(format!(
                "say something of at most {MOST_WORDS} characters"
            )));
        }
        let answer = text("reply").unwrap_or_default();
        if !plain(answer, MOST_REPLY) {
            return Err(ActionError::Invalid("an answer must be plain words".into()));
        }
        let intent = text("intent")
            .and_then(Intent::from_id)
            .ok_or_else(|| ActionError::Invalid("unknown meaning".into()))?;
        let about = match request.args.get("about") {
            Some(Value::Entity(about)) => Some(*about),
            None => None,
            Some(_) => return Err(ActionError::Invalid("about whom?".into())),
        };
        match (intent, about) {
            (Intent::Reconcile, Some(other)) if !(kit.people)(state).contains(&other) => {
                return Err(ActionError::Invalid(
                    "only someone living here can be made up with".into(),
                ));
            }
            (intent, Some(other))
                if intent.about_someone() && (other == who || state.entity(other).is_none()) =>
            {
                return Err(ActionError::Invalid("about whom?".into()));
            }
            (Intent::Place, Some(place)) if !(kit.places)(state).contains(&place) => {
                return Err(ActionError::Invalid("no such place".into()));
            }
            (intent, None) if intent.about_someone() => {
                return Err(ActionError::Invalid("about whom?".into()));
            }
            (intent, Some(_)) if !intent.about_someone() && intent != Intent::Place => {
                return Err(ActionError::Invalid("that isn't about anyone".into()));
            }
            _ => {}
        }
        let asks_for = text("asks_for");
        if asks_for.is_some_and(|asks_for| !plain(asks_for, 200))
            || (asks_for.is_some() && intent != Intent::Need)
        {
            return Err(ActionError::Invalid(
                "only a need asks for something".into(),
            ));
        }

        let now = period(state, &kit);
        let mut changes = Vec::new();
        let set = |key: &str, value: Value| StateChange::SetComponent {
            entity: who,
            key: key.into(),
            value,
        };
        let enrolled = lives::enrolled(state, who);
        let first_today = integer(state, who, TALKED) != Some(now);
        let warmed_today = integer(state, who, WARMED) == Some(now);
        changes.push(set(TALKED, now.into()));
        let mut company = 0;
        let mut regard = 0;
        if first_today && intent != Intent::Rude {
            company -= 5;
        }
        match intent {
            Intent::Compliment | Intent::Thank if !warmed_today => {
                regard += if intent == Intent::Compliment { 3 } else { 2 };
                changes.push(set(WARMED, now.into()));
            }
            Intent::Comfort if !warmed_today => {
                let (_, lack) = worst_need(state, who);
                regard += if lack >= 60 { 5 } else { 1 };
                company -= 10;
                changes.push(set(WARMED, now.into()));
            }
            Intent::Rude => {
                regard -= 8;
                changes.push(set(HURT, now.into()));
            }
            Intent::Apologize if hurt_recently(state, &kit, who) => {
                regard += 6;
                changes.push(StateChange::RemoveComponent {
                    entity: who,
                    key: HURT.into(),
                });
            }
            Intent::Reconcile => {
                if let Some(other) = about {
                    if enrolled && advice(state, &kit, who, other) == Advice::Takes {
                        changes.extend(lives::opinion_by(state, who, other, 10));
                        changes.extend(lives::opinion_by(state, other, who, 5));
                        changes.push(set(&nudged_key(other), now.into()));
                    }
                }
            }
            _ => {}
        }
        if enrolled {
            if company != 0 {
                changes.push(lives::lack_by(state, who, Need::Company, company));
            }
            if regard != 0 {
                changes.push(lives::regard_by(state, who, regard));
            }
        }

        let mut draft = EventDraft::new(SPOKEN);
        draft.targets = std::iter::once(who).chain(about).collect();
        draft.payload.insert("words".into(), words.trim().into());
        draft.payload.insert("intent".into(), intent.id().into());
        draft.payload.insert("reply".into(), answer.into());
        draft.payload.insert(
            "told".into(),
            format!("You talked with {}", lives::name(state, who)).into(),
        );
        if let Some(about) = about {
            draft.payload.insert("about".into(), Value::Entity(about));
        }
        if let Some(asks_for) = asks_for {
            draft.payload.insert("asks_for".into(), asks_for.into());
        }
        draft.changes = changes;
        Ok(draft)
    }
}

/// Whether an Event is an exchange with the player.
pub fn is_talk(event: &Event) -> bool {
    event.kind == SPOKEN
}

/// How an exchange is told in the World's history.
pub fn told(event: &Event) -> Option<String> {
    if !is_talk(event) {
        return None;
    }
    match event.payload.get("told") {
        Some(Value::Text(told)) => Some(told.clone()),
        _ => None,
    }
}

/// One thing the player said to someone, and the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exchange {
    pub who: EntityId,
    pub words: String,
    pub reply: String,
    pub event: EventId,
    pub asks_for: Option<String>,
}

/// What the player has said to people today, oldest first.
pub fn exchanges_today(world: &World) -> Vec<Exchange> {
    let now = world.world_time();
    let text = |event: &Event, key: &str| match event.payload.get(key) {
        Some(Value::Text(text)) => Some(text.clone()),
        _ => None,
    };
    let mut exchanges = world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time == now)
        .filter(|event| is_talk(event))
        .filter_map(|event| {
            Some(Exchange {
                who: *event.targets.first()?,
                words: text(event, "words")?,
                reply: text(event, "reply")?,
                event: event.id,
                asks_for: text(event, "asks_for"),
            })
        })
        .collect::<Vec<_>>();
    exchanges.reverse();
    exchanges
}

#[cfg(test)]
mod tests;
