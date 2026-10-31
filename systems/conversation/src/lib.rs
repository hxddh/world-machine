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

#![forbid(unsafe_code)]

mod ears;
pub mod faces;
pub mod grounded;

mod phrases_feeling;
mod phrases_people;
mod phrases_talk;
mod replies;
// What a model is told and how its answer is checked live in a crate of
// their own, with no World code, so an app can ask a model without
// linking this System; they are this System's all the same.
use lives::Need;
use phrases_feeling::*;
use phrases_people::*;
use phrases_talk::*;
use replies::*;
pub use replies::{recollection, reply, standing, Reply, Standing};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, Event, EventDraft, EventId,
    StateChange, Value, World, WorldState,
};
use world_voice_prompt::bounds;
pub use world_voice_prompt::{
    answer_schema, care, check, checked, envelope, grounds_of, in_world, judge, keeps_to, meanings,
    parse, prompt, wants_json, Checked, Era, Grounds, Hearing, HearingParts, Invented, Judge,
    Judged, Judging, Listened, Listener, OutOfWorld, OwnEars, Verdict, JSON_ANSWER, MOST_REPLY,
    MOST_WORDS,
};

/// The kind of Event an exchange is recorded as.
pub const SPOKEN: &str = "spoken";

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
    /// How their day has been, what they have been up to.
    Day,
    /// Who they are: tell me about yourself.
    AboutYou,
    Family,
    /// What is troubling them: are you lonely, you look tired.
    Worry,
    Weather,
    /// What is coming up on the calendar.
    Coming,
    /// Something the player brought them.
    Gift,
    /// Who they are close to, and who they are not.
    Friends,
    /// What is wrong between them and someone else.
    Quarrel,
    /// To go for a drink, a walk, a meal together.
    Invite,
    /// Just keeping the conversation going: ok, I see, haha.
    Ack,
    /// What they think of the player.
    Standing,
    Unclear,
}

impl Intent {
    pub const ALL: [Intent; 28] = [
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
        Intent::Day,
        Intent::AboutYou,
        Intent::Family,
        Intent::Worry,
        Intent::Weather,
        Intent::Coming,
        Intent::Gift,
        Intent::Friends,
        Intent::Quarrel,
        Intent::Invite,
        Intent::Ack,
        Intent::Standing,
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
            Intent::Day => "day",
            Intent::AboutYou => "about_you",
            Intent::Family => "family",
            Intent::Worry => "worry",
            Intent::Weather => "weather",
            Intent::Coming => "coming",
            Intent::Gift => "gift",
            Intent::Friends => "friends",
            Intent::Quarrel => "quarrel",
            Intent::Invite => "invite",
            Intent::Ack => "ack",
            Intent::Standing => "standing",
            Intent::Unclear => "unclear",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|intent| intent.id() == id)
    }

    /// Whether it is about somebody else, who must then be named.
    pub fn about_someone(self) -> bool {
        matches!(
            self,
            Intent::HowIs | Intent::ThinkOf | Intent::Reconcile | Intent::Quarrel
        )
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
    /// Who someone is, as anybody would put it: "Mara's the baker."
    pub role_line: fn(&World, EntityId) -> Option<String>,
    /// What the place is making together, done and under way, for what
    /// its people can say about the work.
    pub works: fn(&World) -> Vec<world_projection::Goal>,
    /// How the place as a whole is doing, in anybody's words.
    pub place_mood: fn(&World) -> String,
    /// What is coming up soon on the place's calendar, in a few words:
    /// "Lantern Night in 3 days".
    pub coming_up: fn(&World) -> Option<String>,
    /// The other names someone or somewhere goes by, given the name they
    /// are known by: a name in Chinese, a nickname, how else a place is
    /// called ("harbour" for Harbor).
    pub aliases: fn(&str) -> Vec<String>,
    /// What the sky is doing, in anybody's words: "Grey and wet."
    pub weather: fn(&World) -> String,
    /// The names of the days on the place's calendar, so a player can ask
    /// about them.
    pub occasions: fn(&WorldState) -> Vec<String>,
    /// What someone says about something the player did for them, in the
    /// Pack's own moments (an answer to their question, say), with `{ago}`
    /// where when it was goes: "You said to mend the roof {ago}."
    pub recalled: fn(&World, &Event, EntityId) -> Option<String>,
    /// How far along the place's things are, for what its people can have
    /// heard of.
    pub era: Era,
    /// Names the place's people speak of that are nobody and nowhere on
    /// the scene: a boat, a town over the water, a planet they came from.
    pub elsewhere: &'static [&'static str],
    /// Every name the place's lines can ever say, in every language it
    /// speaks (see [`Hearing::lexicon`]): its people and newcomers-to-be,
    /// its figures, places, boats and days, each in translation too.
    pub lexicon: fn() -> &'static [String],
    /// What the place has that its time otherwise would not, and what it
    /// lacks that its time otherwise would have, in every language it
    /// speaks: its era lexicon.
    pub era_has: &'static [&'static str],
    pub era_lacks: &'static [&'static str],
}

const TALKED: &str = "conversation.talked";
const WARMED: &str = "conversation.warmed";
const HURT: &str = "conversation.hurt";
/// A lexicon with nothing in it, for a Kit whose names are all on the
/// scene.
pub fn no_lexicon() -> &'static [String] {
    &[]
}

/// A World's lexicon: every name in `names`, each by its other names
/// (`aliases`), and every name in `table` in each language it gives. The
/// table is one name a line, English first, then its forms in each other
/// language in a column of its own, several forms separated by `|`; a line
/// begun with `#` says what the table is.
pub fn lexicon_of<'a>(
    names: impl IntoIterator<Item = &'a str>,
    aliases: fn(&str) -> Vec<String>,
    table: &str,
) -> Vec<String> {
    let mut lexicon = std::collections::BTreeSet::new();
    for name in names {
        lexicon.insert(name.to_string());
        lexicon.extend(aliases(name));
    }
    for line in table.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut columns = line.split('\t');
        if let Some(english) = columns.next() {
            lexicon.insert(english.trim().to_string());
            lexicon.extend(aliases(english.trim()));
        }
        for column in columns {
            lexicon.extend(
                column
                    .split('|')
                    .map(str::trim)
                    .filter(|form| !form.is_empty())
                    .map(str::to_string),
            );
        }
    }
    lexicon.retain(|name| !name.trim().is_empty());
    lexicon.into_iter().collect()
}

/// The forms `table` (as [`lexicon_of`] reads it) gives `name` in other
/// languages.
pub fn forms_in(table: &str, name: &str) -> Vec<String> {
    table
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('\t'))
        .filter(|(english, _)| english.trim() == name)
        .flat_map(|(_, rest)| {
            rest.split(['\t', '|'])
                .map(str::trim)
                .filter(|form| !form.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

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

/// Every name someone or somewhere is called by, as normalized words:
/// their whole name, their first name (or a place's last word, "the
/// bakery"), and any other name the World gives them.
fn names_of(state: &WorldState, kit: &Kit, id: EntityId, by_last: bool) -> Vec<String> {
    let known = lives::name(state, id);
    let full = normal(&known).trim().to_string();
    if full.is_empty() || full == "someone" {
        return Vec::new();
    }
    let short = if by_last {
        full.rsplit(' ').next()
    } else {
        full.split(' ').next()
    }
    .unwrap_or(&full)
    .to_string();
    let mut names = vec![full.clone()];
    if short != full && short.chars().count() >= 3 {
        names.push(short);
    }
    for alias in (kit.aliases)(&known) {
        let alias = normal(&alias).trim().to_string();
        if !alias.is_empty() && !names.contains(&alias) {
            names.push(alias);
        }
    }
    names
}

/// Whom, among `candidates`, the words name.
fn named(
    state: &WorldState,
    kit: &Kit,
    text: &str,
    candidates: &[EntityId],
    by_last: bool,
) -> Option<EntityId> {
    candidates.iter().copied().find(|id| {
        names_of(state, kit, *id, by_last).iter().any(|name| {
            // Written without spaces, a name runs straight into the words
            // around it.
            // "Mara's sorry" names Mara as well as "Mara" does.
            has(text, name)
                || has(text, &format!("{name}'s"))
                || (!text.is_ascii() && text.contains(name.as_str()))
        })
    })
}

/// What the player's words to `who` mean: heard by their words
/// ([`hear_words`]), then in the light of the favour open for `who`, if
/// one is ([`favour::in_context`]): "she's sorry" passes on the apology of
/// whoever asked, and "Mara asked me to see how you are" asks after `who`.
pub fn hear(state: &WorldState, kit: &Kit, who: EntityId, words: &str) -> Heard {
    let heard = hear_words(state, kit, who, words);
    favour::in_context(state, kit, who, words, heard)
}

/// What the player's words to `who` mean by their words alone: the phrase
/// table, read on the words as typed and folded to one form (`ears`), and
/// failing that the meaning their cue words score highest.
pub(crate) fn hear_words(state: &WorldState, kit: &Kit, who: EntityId, words: &str) -> Heard {
    let text = normal(words);
    let folded = ears::fold(&text);
    let others = (kit.people)(state)
        .into_iter()
        .filter(|person| *person != who)
        .collect::<Vec<_>>();
    let person = named(state, kit, &text, &others, false);
    let place = named(state, kit, &text, &(kit.places)(state), true);
    let occasion = (kit.occasions)(state).iter().any(|name| {
        let name = normal(name);
        let name = name.trim();
        has(&text, name.strip_prefix("the ").unwrap_or(name))
    });
    let heard = |intent, about| Heard { intent, about };
    let is = |phrases: &'static [&'static str]| {
        any(&text, phrases) || ears::any_folded(&folded, phrases)
    };
    // Taking leave politely ("邪魔しちゃ悪いから", I won't be in your way)
    // before anything rude in its words.
    if is(LEAVING) {
        return heard(Intent::Farewell, None);
    }
    if is(RUDE) {
        return heard(Intent::Rude, None);
    }
    if let Some(person) = person {
        for (phrases, intent) in [
            (RECONCILE, Intent::Reconcile),
            (QUARREL, Intent::Quarrel),
            (OPINION, Intent::ThinkOf),
            (HOW_IS, Intent::HowIs),
        ] {
            if is(phrases) {
                return heard(intent, Some(person));
            }
        }
        // What the table does not place, by its cue words; an opinion
        // otherwise.
        let intent = ears::scored_about(&text, &folded).unwrap_or(Intent::ThinkOf);
        return heard(intent, Some(person));
    }
    if occasion && !is(GIFT) {
        return heard(Intent::Coming, None);
    }
    // Words that say one thing whatever else is in them ("lovely to see
    // you" greets before it praises), the first found.
    if let Some((_, intent)) = SURE
        .iter()
        .find(|(phrase, _)| has(&text, phrase) || ears::has_folded(&folded, phrase))
    {
        return heard(*intent, None);
    }
    // "Night, Emma." says goodnight.
    if text.starts_with(" night ") || text.starts_with(" nighty ") {
        return heard(Intent::Farewell, None);
    }
    // "大丈夫？" asks how someone is; "大丈夫だよ" says they will be.
    let asked = words.trim_end().ends_with(['?', '？']);
    if asked && folded.contains("大丈夫") && !folded.contains("大丈夫だ") && !is(COMFORT_ONLY)
    {
        return heard(Intent::HowAreYou, None);
    }
    // In the order that settles words holding more than one of them:
    // "don't worry" is comfort before it is a worry, "forgive me" an
    // apology before it is about the player, "nice weather" about the
    // weather before it is a compliment.
    for (phrases, intent) in [
        (COMFORT, Intent::Comfort),
        (SORRY, Intent::Apologize),
        (STANDING, Intent::Standing),
        (PRAISE, Intent::Compliment),
        (GIFT, Intent::Gift),
        (THANK, Intent::Thank),
        (JOB, Intent::Work),
        (WORRY, Intent::Worry),
        (FRIENDS, Intent::Friends),
        (YOUTH, Intent::AboutYou),
        (FAMILY, Intent::Family),
        (ABOUT_YOU, Intent::AboutYou),
        (DAY, Intent::Day),
        (PARTING, Intent::Farewell),
        (WEATHER, Intent::Weather),
        (COMPLIMENT, Intent::Compliment),
        (INVITE, Intent::Invite),
    ] {
        if is(phrases) {
            return heard(intent, None);
        }
    }
    // News of a place is news: "what's the word around the town".
    if is(NEWS_FIRST) {
        return heard(Intent::News, None);
    }
    if let Some(place) = place {
        return heard(Intent::Place, Some(place));
    }
    for (phrases, intent) in [
        (COMING, Intent::Coming),
        (NEWS, Intent::News),
        (WELL, Intent::HowAreYou),
        (HELP, Intent::Need),
        (WORK, Intent::Work),
        (NEED, Intent::Need),
        (HOW_ARE_YOU, Intent::HowAreYou),
        (FAREWELL, Intent::Farewell),
        (GREET, Intent::Greet),
        (ACK, Intent::Ack),
    ] {
        if is(phrases) {
            return heard(intent, None);
        }
    }
    // Asked how anything is, they say how they are.
    if text.starts_with(" how ") || text.starts_with(" how's ") {
        return heard(Intent::HowAreYou, None);
    }
    // Words the table does not place: the meaning their cue words score
    // highest, if they hold any.
    match ears::scored(&text, &folded) {
        Some(intent) => heard(intent, None),
        None => heard(Intent::Unclear, None),
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
    if !reply.offers.is_empty() {
        request = request.arg("offers", offers_value(&reply.offers));
    }
    request
}

fn offers_value(offers: &[String]) -> Value {
    Value::List(offers.iter().map(|offer| offer.as_str().into()).collect())
}

/// Hears the player's words to someone and answers them, ready to record.
pub fn say(world: &World, kit: &Kit, who: EntityId, words: &str) -> Result<ActionRequest, String> {
    if !can_talk_to(world.state(), kit, who) {
        return Err(format!(
            "{} can't be spoken to now",
            lives::name(world.state(), who)
        ));
    }
    if let Some(care) = care::topic(words) {
        return Ok(care_request(who, words, care));
    }
    let heard = hear(world.state(), kit, who, words);
    let answer = grounded::answer(world, kit, who, heard, words);
    Ok(echoed(
        world,
        kit,
        words,
        request(who, words, heard, &answer),
    ))
}

/// A request marked as saying back a line said today, if it does: its
/// words then do nothing ([`deed`]).
fn echoed(world: &World, kit: &Kit, words: &str, request: ActionRequest) -> ActionRequest {
    if echoes_today(world, kit, words) {
        request.arg("echo", "yes")
    } else {
        request
    }
}

/// The Action that records words on a topic no resident talks about: heard
/// as nothing in particular, and answered with the System's own gentle
/// line in the player's language (`care`).
pub fn care_request(who: EntityId, words: &str, care: care::Care) -> ActionRequest {
    ActionRequest::new("conversation_say")
        .arg("who", Value::Entity(who))
        .arg("words", words.trim())
        .arg("intent", Intent::Unclear.id())
        .arg("reply", care.line(bounds::tongue(words)))
        .arg("care", care.id())
}

/// What the player's words do, as this System's own ears hear them: what
/// warms or hurts someone, invites them out or does a favour. Only the
/// player's own words do anything, never a model's meaning or answer or a
/// judge's verdict: talk never pays more than the words themselves would.
/// What stands in brackets or between stars (a stage direction, an outcome
/// narrated) is not said; words on a topic no resident talks about, and a
/// line someone said to the player today said back whole (`echo`), do
/// nothing.
pub fn deed(state: &WorldState, kit: &Kit, who: EntityId, words: &str, echo: bool) -> Heard {
    let said = spoken_words(words);
    if echo || said.trim().is_empty() || care::topic(words).is_some() {
        return Heard {
            intent: Intent::Unclear,
            about: None,
        };
    }
    hear(state, kit, who, &said)
}

/// Words without what stands in brackets or between stars.
fn spoken_words(words: &str) -> String {
    let mut out = String::new();
    let mut closing: Vec<char> = Vec::new();
    for c in words.chars() {
        let close = match c {
            '[' => Some(']'),
            '(' => Some(')'),
            '（' => Some('）'),
            '【' => Some('】'),
            '<' => Some('>'),
            '{' => Some('}'),
            _ => None,
        };
        if let Some(close) = close {
            closing.push(close);
            continue;
        }
        if c == '*' {
            if closing.last() == Some(&'*') {
                closing.pop();
            } else {
                closing.push('*');
            }
            continue;
        }
        if closing.last() == Some(&c) {
            closing.pop();
            continue;
        }
        if closing.is_empty() {
            out.push(c);
        }
    }
    world_core::text::clean_text(&out)
}

/// Whether `words` say back, whole, a line someone in the World said to
/// the player today (an answer, or a favour asked).
pub fn echoes_today(world: &World, kit: &Kit, words: &str) -> bool {
    let fold = |text: &str| {
        text.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let words = fold(words);
    if words.chars().count() < 12 {
        return false;
    }
    let now = period(world.state(), kit);
    world
        .events()
        .iter()
        .rev()
        .take_while(|event| (event.world_time / kit.period.max(1)) as i64 >= now)
        .filter_map(|event| match event.kind.as_str() {
            SPOKEN => payload_text(event, "reply").map(str::to_string),
            favour::ASKED => favour::said(world.state(), event).map(|(_, line)| line),
            _ => None,
        })
        .any(|line| fold(&line) == words)
}

/// Registers the Action that records exchanges.
pub fn register_actions(
    actions: &mut ActionRegistry,
    kit: fn(&WorldState) -> Kit,
) -> Result<(), ActionError> {
    actions.register(Says(kit))?;
    actions.register(report::Reports)?;
    favour::register_actions(actions, kit)
}

/// Whether `text` is something to record: at most `most` characters, and
/// none of them hidden (a control character, a bidirectional override, an
/// invisible separator, a tag character).
fn plain(text: &str, most: usize) -> bool {
    let count = text.chars().count();
    count > 0 && count <= most && !text.chars().any(world_core::text::is_hidden_control)
}

/// Whether text that came from a model, or from an app about one, is
/// clean as it stands: [`plain`], and nothing [`world_core::text::
/// clean_text`] would change (no runs of spaces, no ends to trim).
fn clean(text: &str, most: usize) -> bool {
    plain(text, most) && world_core::text::is_clean_text(text)
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
        // What someone offers to be asked instead comes only with an
        // answer that could not say: three questions, in plain words.
        let offers = match request.args.get("offers") {
            None => Vec::new(),
            Some(Value::List(items))
                if items.len() == grounded::OFFERS
                    && intent == Intent::Unclear
                    && text("voiced").is_none() =>
            {
                items
                    .iter()
                    .map(|item| match item {
                        Value::Text(offer) if plain(offer, 200) => Ok(offer.clone()),
                        _ => Err(ActionError::Invalid("an offer must be plain words".into())),
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            Some(_) => {
                return Err(ActionError::Invalid(
                    "only an answer that cannot say offers three things to ask".into(),
                ))
            }
        };
        let asks_for = text("asks_for");
        if asks_for.is_some_and(|asks_for| !plain(asks_for, 200))
            || (asks_for.is_some() && intent != Intent::Need)
        {
            return Err(ActionError::Invalid(
                "only a need asks for something".into(),
            ));
        }
        // Words on a topic no resident talks about are answered with the
        // System's own line, and only so.
        let care = match (care::topic(words), text("care").map(care::Care::from_id)) {
            (None, None) => None,
            (Some(topic), Some(Some(care)))
                if care == topic
                    && intent == Intent::Unclear
                    && about.is_none()
                    && answer == care.line(bounds::tongue(words)) =>
            {
                Some(care)
            }
            _ => {
                return Err(ActionError::Invalid(
                    "that is answered with care, in the System's own words".into(),
                ))
            }
        };
        // What the words do is what this System's own ears hear in them,
        // whatever meaning was recorded beside the answer: a model's
        // meaning, its answer and a judge's verdict never pay.
        //
        // The one exception is the open favour's own quick reply, chosen
        // with a click (`say_offered`): what it does is what the System's
        // own reply for that favour means to the System's own ears, worked
        // out here from the World and never taken from the request. It does
        // no more than typing that reply would, in any language.
        let echo = text("echo").is_some();
        let offered = (text("offered").is_some() && !echo)
            .then(|| favour::offered(state, &kit, who))
            .flatten();
        let effect = offered.unwrap_or_else(|| deed(state, &kit, who, words, echo));
        let meant = intent;
        let (intent, about) = (effect.intent, effect.about);

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
            Intent::Compliment | Intent::Thank | Intent::Gift if !warmed_today => {
                regard += match intent {
                    Intent::Gift => 4,
                    Intent::Compliment => 3,
                    _ => 2,
                };
                changes.push(set(WARMED, now.into()));
            }
            Intent::Invite
                if accepts_invite(state, &kit, who)
                    && integer(state, who, INVITED) != Some(now) =>
            {
                company -= 10;
                regard += 1;
                changes.push(set(INVITED, now.into()));
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
        // The meaning a listener heard, where it is not what the words did.
        if meant != intent {
            draft.payload.insert("meaning".into(), meant.id().into());
        }
        if let Some(care) = care {
            draft.payload.insert("care".into(), care.id().into());
        }
        if text("voiced").is_some() && care.is_none() {
            draft.payload.insert("voiced".into(), "yes".into());
        }
        if offered.is_some() {
            draft.payload.insert("offered".into(), "yes".into());
        }
        draft.payload.insert("reply".into(), answer.into());
        // A listener's answer declined for going beyond the World is noted,
        // so the record shows the System's own answer stood in for it.
        if let Some(why) = text("declined") {
            draft.payload.insert("declined".into(), why.into());
        }
        // A judge's verdict on the answer, and which judge gave it.
        for key in ["judge", "verdict"] {
            if let Some(said) = text(key).filter(|said| clean(said, 120)) {
                draft.payload.insert(key.into(), said.into());
            }
        }
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
        if !offers.is_empty() {
            draft.payload.insert("offers".into(), offers_value(&offers));
        }
        if intent == Intent::Invite && accepts_invite(state, &kit, who) {
            draft.payload.insert("accepted".into(), "yes".into());
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
        return report::told(event);
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
    /// Whether the reply's words were a language model's.
    pub voiced: bool,
    /// Whether the player reported the reply ([`report`]).
    pub reported: bool,
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
                voiced: text(event, "voiced").is_some(),
                reported: report::reported(world, event.id),
            })
        })
        .collect::<Vec<_>>();
    exchanges.reverse();
    exchanges
}

pub mod corpus;
pub mod favour;
pub mod openers;
pub mod report;

#[cfg(test)]
mod grounded_tests;
#[cfg(test)]
mod never_pays;
#[cfg(test)]
mod test_world;
#[cfg(test)]
mod tests;

/// A hearing as a World hands it to an app that asks a model itself.
pub fn to_voice(hearing: &Hearing) -> world_projection::VoiceHearing {
    let parts = hearing.parts();
    world_projection::VoiceHearing {
        name: parts.name,
        settlement: parts.settlement,
        traits: parts.traits,
        facts: parts.facts,
        people: parts.people,
        places: parts.places,
        words: parts.words,
        answer: parts.answer,
        known: parts.known,
        era: parts.era,
        lexicon: parts.lexicon,
        era_has: parts.era_has,
        era_lacks: parts.era_lacks,
    }
}

/// A hearing from what a World handed an app, held to what a hearing can
/// hold ([`Hearing::held`]).
pub fn from_voice(voice: &world_projection::VoiceHearing) -> Option<Hearing> {
    Hearing::held(&HearingParts {
        name: voice.name.clone(),
        settlement: voice.settlement.clone(),
        traits: voice.traits.clone(),
        facts: voice.facts.clone(),
        people: voice.people.clone(),
        places: voice.places.clone(),
        words: voice.words.clone(),
        answer: voice.answer.clone(),
        known: voice.known.clone(),
        era: voice.era.clone(),
        lexicon: voice.lexicon.clone(),
        era_has: voice.era_has.clone(),
        era_lacks: voice.era_lacks.clone(),
    })
}

/// A model's response the app already has: the app asked the model itself,
/// off its own thread, and this reads what came back like any other
/// proposal, with the verdict of the judge the app asked, if it asked one.
/// The verdict comes beside the response, never from inside it.
pub struct Answered {
    pub response: String,
    pub judged: Option<Judged>,
}

impl Answered {
    /// A response with no judge's verdict.
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            judged: None,
        }
    }

    /// A response with the verdict an app says its judge gave: read only
    /// if every word of it is clean and the verdict is one a judge gives.
    pub fn judged(response: impl Into<String>, judgement: &world_projection::Judgement) -> Self {
        let verdict = match judgement.verdict.as_str() {
            "none" => Some(None),
            verdict => judge::verdict_from_record(verdict, &judgement.kind).map(Some),
        };
        let judged = verdict
            .filter(|_| {
                clean(&judgement.judge, 80)
                    && (judgement.kind.is_empty() || clean(&judgement.kind, 40))
            })
            .map(|verdict| Judged {
                judge: judgement.judge.clone(),
                verdict,
            });
        Self {
            response: response.into(),
            judged,
        }
    }
}

impl Listener for Answered {
    fn listen(&mut self, _: &Hearing) -> Option<Listened> {
        let mut listened = parse(&self.response)?;
        listened.judged = self.judged.clone();
        Some(listened)
    }
}

/// How a verdict is said to a World beside a response: the judge, the
/// verdict and its kind.
pub fn judgement(judged: &Judged) -> world_projection::Judgement {
    world_projection::Judgement {
        judge: judged.judge.clone(),
        verdict: judged.verdict_id().into(),
        kind: match judged.verdict {
            Some(Verdict::Decline(why)) => why.id().into(),
            _ => "none".into(),
        },
    }
}

/// The prompt a language model is asked to hear the player's words to
/// someone with, exactly as a listener would be given it; nothing for
/// someone who cannot be spoken to or words that could never be recorded.
pub fn prompt_for(world: &World, kit: &Kit, who: EntityId, words: &str) -> Option<String> {
    hearing_for(world, kit, who, words).map(|hearing| prompt(&hearing))
}

/// What a listener would be told for these words to someone, with this
/// System's own answer; nothing for someone who cannot be spoken to or
/// words that could never be recorded.
pub fn hearing_for(world: &World, kit: &Kit, who: EntityId, words: &str) -> Option<Hearing> {
    let state = world.state();
    if !can_talk_to(state, kit, who)
        || !plain(words.trim(), MOST_WORDS)
        || care::topic(words).is_some()
    {
        return None;
    }
    let words = &world_core::text::clean_text(words);
    let heard = hear(state, kit, who, words);
    let own = grounded::answer(world, kit, who, heard, words);
    Some(hearing(world, kit, who, words, &own.line))
}

/// The prompt a judge is asked about `answer` to these words with, exactly
/// as a judge would be given it.
pub fn judge_prompt_for(
    world: &World,
    kit: &Kit,
    who: EntityId,
    words: &str,
    answer: &str,
) -> Option<String> {
    hearing_for(world, kit, who, words).map(|hearing| judge::judge_prompt(&hearing, answer))
}

/// What a listener is told about someone, from how their life stands.
pub fn hearing(world: &World, kit: &Kit, who: EntityId, words: &str, answer: &str) -> Hearing {
    let state = world.state();
    let people = (kit.people)(state)
        .into_iter()
        .filter(|person| *person != who)
        .collect::<Vec<_>>();
    let mut facts = Vec::new();
    if let Some(how) = lives::how_are_you(world, who) {
        facts.push(format!("How you are: {how}"));
    }
    if let Some(today) = lives::said_today(world, who) {
        facts.push(format!("What you did today: {today}"));
    }
    if let Some(work) = (kit.work_line)(world, who) {
        facts.push(format!("Your work: {work}"));
    }
    facts.push(format!("How the place is: {}", (kit.place_mood)(world)));
    for other in &people {
        let name = lives::name(state, *other);
        if let Some(view) = lives::thinks_of(world, who, *other) {
            facts.push(format!("What you think of {name}: {view}"));
        }
        facts.push(format!(
            "If told to make up with {name}, you would {}",
            match advice(state, kit, who, *other) {
                Advice::Takes => "agree to talk to them",
                Advice::AlreadyFine => "say you get on fine already",
                Advice::Refuses => "tell them to mind their own business",
                Advice::Waiting => "say you already said you would, give it time",
            }
        ));
    }
    let since = state
        .world_time()
        .saturating_sub(kit.period.max(1).saturating_mul(5));
    for news in lives::news_since(world, since).into_iter().rev().take(2) {
        facts.push(format!("News: {news}"));
    }
    if let Some(coming) = (kit.coming_up)(world) {
        facts.push(format!("Coming up: {coming}"));
    }
    let (need, _) = (kit.need_line)(world, who);
    facts.push(format!("What you need: {need}"));
    if let Some(recalled) = recollection(world, kit, who) {
        facts.push(format!(
            "Something the player said before that you remember: {recalled}"
        ));
    }
    facts.push(format!(
        "How you feel about the player: {}",
        standing(state, kit, who).words
    ));
    facts.push(format!("The weather: {}", (kit.weather)(world)));
    if lives::lack(state, who, Need::Rest) >= 70 {
        facts.push("You are too tired to go anywhere.".into());
    }
    facts.push(if hurt_recently(state, kit, who) {
        "The player was unkind to you lately.".into()
    } else {
        "The player has not been unkind to you lately.".into()
    });
    Hearing {
        name: lives::name(state, who),
        settlement: kit.settlement.into(),
        traits: lives::traits(state, who),
        facts,
        people: people
            .iter()
            .map(|person| lives::name(state, *person))
            .collect(),
        places: (kit.places)(state)
            .into_iter()
            .map(|place| lives::name(state, place))
            .collect(),
        words: words.trim().into(),
        answer: answer.into(),
        known: known_names(state, kit),
        era: kit.era,
        lexicon: (kit.lexicon)().to_vec(),
        era_has: kit.era_has.iter().map(|thing| thing.to_string()).collect(),
        era_lacks: kit
            .era_lacks
            .iter()
            .map(|thing| thing.to_string())
            .collect(),
    }
}

/// Every name the World knows besides the people and places it lists:
/// whoever and whatever has a name in its state (the dead, boats, works),
/// its days, what its people speak of elsewhere, and all of them by their
/// other names.
fn known_names(state: &WorldState, kit: &Kit) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    for entity in state.entities() {
        if let Some(Value::Text(name)) = entity.component("name") {
            names.insert(name.clone());
        }
    }
    names.extend((kit.occasions)(state));
    names.extend(kit.elsewhere.iter().map(|name| name.to_string()));
    let aliases = names
        .iter()
        .flat_map(|name| (kit.aliases)(name))
        .collect::<Vec<_>>();
    names.extend(aliases);
    names
        .into_iter()
        .filter(|name| !name.trim().is_empty())
        .collect()
}

/// What a model's words about this World are checked against when they are
/// not an answer to the player: `told` (what it was asked to put into
/// words), and every name the World knows.
pub fn world_grounds(world: &World, kit: &Kit, told: &[&str]) -> Grounds {
    let state = world.state();
    let names = (kit.people)(state)
        .into_iter()
        .chain((kit.places)(state))
        .map(|id| lives::name(state, id))
        .chain(known_names(state, kit))
        .collect::<Vec<_>>();
    Grounds::new(
        told.iter()
            .copied()
            .chain(names.iter().map(String::as_str))
            .chain((kit.lexicon)().iter().map(String::as_str))
            .chain(kit.era_has.iter().copied())
            .chain([kit.settlement]),
        "",
        kit.era,
    )
    .lacking(kit.era_lacks.iter().copied())
}

/// Hears the player's words with a listener if it has something usable to
/// say, and with this System's own ears otherwise.
///
/// A usable answer is then checked. Its citations must be facts it was
/// given, every name in it must be one it was given, and it must keep to
/// the World by the bounds. A certain finding declines it; a doubtful one
/// declines it too, unless a judge was asked and kept it; and a judge may
/// decline what the checks let through. A declined answer is replaced by
/// this System's own, and the record says why and what the judge said.
pub fn say_with(
    world: &World,
    kit: &Kit,
    who: EntityId,
    words: &str,
    listener: &mut dyn Listener,
) -> Result<ActionRequest, String> {
    let state = world.state();
    if !can_talk_to(state, kit, who) {
        return Err(format!(
            "{} can't be spoken to now",
            lives::name(state, who)
        ));
    }
    // Words that could never be recorded are not sent anywhere.
    if !plain(words.trim(), MOST_WORDS) {
        return Err(format!("Say something of at most {MOST_WORDS} characters"));
    }
    let words = &world_core::text::clean_text(words);
    // Nor are words on a topic no resident talks about.
    if let Some(care) = care::topic(words) {
        return Ok(care_request(who, words, care));
    }
    listened_request(world, kit, who, words, listener)
        .map(|request| echoed(world, kit, words, request))
}

/// The open favour's own quick reply, said to `who` with a click
/// (`Ears::Offered`): done as the favour it offers, never heard. What it
/// means is what the System's own reply means to its own ears
/// ([`favour::offered`]), whatever language the player's button showed it
/// in, so a translation cannot keep a click from doing what it says. The
/// words shown are what is recorded as said.
///
/// Nothing is trusted from the screen but which person was clicked: with
/// no favour open for `who`, the words are heard like any others.
pub fn say_offered(
    world: &World,
    kit: &Kit,
    who: EntityId,
    words: &str,
) -> Result<ActionRequest, String> {
    let state = world.state();
    let Some(heard) = favour::offered(state, kit, who) else {
        return say_with(world, kit, who, words, &mut OwnEars);
    };
    if !can_talk_to(state, kit, who) {
        return Err(format!(
            "{} can't be spoken to now",
            lives::name(state, who)
        ));
    }
    if !plain(words.trim(), MOST_WORDS) {
        return Err(format!("Say something of at most {MOST_WORDS} characters"));
    }
    let words = &world_core::text::clean_text(words);
    let answer = reply(world, kit, who, heard);
    Ok(echoed(
        world,
        kit,
        words,
        request(who, words, heard, &answer).arg("offered", "yes"),
    ))
}

/// What a listener heard in clean words, checked, as the request that
/// records it.
fn listened_request(
    world: &World,
    kit: &Kit,
    who: EntityId,
    words: &str,
    listener: &mut dyn Listener,
) -> Result<ActionRequest, String> {
    let state = world.state();
    let heard = hear(state, kit, who, words);
    let own = grounded::answer(world, kit, who, heard, words);
    let told = hearing(world, kit, who, words, &own.line);
    let Some(listened) = listener.listen(&told) else {
        return Ok(request(who, words, heard, &own));
    };
    let Some(intent) = Intent::from_id(listened.meaning.trim()) else {
        return Ok(request(who, words, heard, &own));
    };
    let by_name = |name: &str, candidates: Vec<EntityId>| {
        let name = name.trim().to_lowercase();
        candidates.into_iter().find(|id| {
            let full = lives::name(state, *id).to_lowercase();
            full == name || full.split(' ').next() == Some(name.as_str())
        })
    };
    let others = (kit.people)(state)
        .into_iter()
        .filter(|person| *person != who)
        .collect::<Vec<_>>();
    let about = match (intent, listened.about.as_deref()) {
        (Intent::Place, Some(name)) => by_name(name, (kit.places)(state)),
        (intent, Some(name)) if intent.about_someone() => by_name(name, others),
        _ => None,
    };
    if (intent.about_someone() && about.is_none()) || (intent == Intent::Place && about.is_none()) {
        return Ok(request(who, words, heard, &own));
    }
    let answer = listened.answer.trim();
    // A proposal is taken whole or not at all: an unusable answer means
    // the meaning it came with is not trusted either. A model's words are
    // clean as they stand, or not taken.
    if !clean(answer, MOST_REPLY) {
        return Ok(request(who, words, heard, &own));
    }
    let judged = |request: ActionRequest| match &listened.judged {
        Some(judged) => request
            .arg("judge", judged.judge.as_str())
            .arg("verdict", judged.verdict_id()),
        None => request,
    };
    // Nor is an answer that goes beyond what this person can know: it is
    // declined, and the record says so.
    let mut checked = bounds::check(answer, &told);
    if let Some(cites) = &listened.cites {
        // Citing a fact it was never given is certain; a name resting on
        // nothing it cites or knows is a doubt.
        if cites
            .iter()
            .any(|n| *n < 1 || *n as usize > told.facts.len())
        {
            checked.found.push("cites");
            checked.strict.get_or_insert(OutOfWorld::Cites);
            checked.certain.get_or_insert(OutOfWorld::Cites);
        }
        if bounds::names_unknown(answer, &told, cites) {
            checked.found.push("names");
            checked.strict.get_or_insert(OutOfWorld::Stranger);
        }
    }
    let verdict = listened.judged.as_ref().and_then(|judged| judged.verdict);
    if let Some(why) = judge::decide(&checked, verdict) {
        return Ok(judged(
            request(who, words, heard, &own).arg("declined", why.id()),
        ));
    }
    let heard = Heard { intent, about };
    // A need still asks for what is really on offer.
    let asks_for = (intent == Intent::Need)
        .then(|| (kit.need_line)(world, who).1)
        .flatten();
    // Said in the model's words: the record, and the screen, say so.
    Ok(judged(request(
        who,
        words,
        heard,
        &Reply {
            asks_for,
            ..Reply::said(answer)
        },
    ))
    .arg("voiced", "yes"))
}
