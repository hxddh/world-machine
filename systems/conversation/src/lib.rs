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

pub mod faces;

// What a model is told and how its answer is checked live in a crate of
// their own, with no World code, so an app can ask a model without
// linking this System; they are this System's all the same.
use lives::Need;
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

const RUDE: &[&str] = &[
    "drop dead",
    "disgust",
    "a joke",
    "out of my face",
    "get out",
    "leave me alone",
    "boring",
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
    "annoying",
    "nobody likes you",
    "piss off",
    "笨",
    "蠢",
    "讨厌你",
    "滚",
    "闭嘴",
    "没用",
    "烦死",
    "烦人",
    "ばかじゃない",
    "このばか",
    "バカ",
    "あほ",
    "うるさい",
    "黙れ",
    "大嫌い",
    "嫌いだ",
    "役立たず",
    "あっち行",
    "消えろ",
    "ムカつく",
    "うざい",
    "烦不烦",
    "少管闲事",
    "管好你自己",
    "多管闲事",
    "神经病",
    "うっとうし",
    "ほっといて",
    "邪魔",
    "しつこい",
    "最低",
    "きもい",
    "你真讨厌",
    "真讨厌",
    "讨厌死",
    "うるせ",
    "お前には関係",
    "あんたには関係",
    "関係ないでしょ",
    "waste of space",
    "nobody cares",
    "no one cares",
    "worthless",
    "good for nothing",
    "get stuffed",
    "sod off",
    "bugger off",
    "clear off",
    "rubbish at",
    "you're rubbish",
    "a disgrace",
    "shameful",
    "imbecile",
    "halfwit",
    "nitwit",
    "clown",
    "bore me",
    "stink",
    "you smell",
    "废物",
    "没人在乎",
    "谁在乎",
    "没出息",
    "丢人",
    "滚开",
    "去死",
    "白痴",
    "傻瓜",
    "傻子",
    "蠢货",
    "垃圾",
    "烦死了",
    "真烦",
    "讨厌鬼",
    "役に立たない",
    "誰も気にして",
    "お前なんか",
    "あんたなんか",
    "くたばれ",
    "失せろ",
    "間抜け",
    "まぬけ",
    "のろま",
    "ばか",
    "アホ",
    "クズ",
    "くず",
    "ろくでなし",
    "気持ち悪い",
    "目障り",
    "なんか気にしてない",
    "お前の意見",
    "お前なんて",
    "nuisance",
    "a pest",
    "you pest",
    "让开",
    "讨人嫌",
    "滚一边",
    "どけ",
    "ほんと迷惑",
    "迷惑なんだ",
    "迷惑だよ",
    "邪魔だ",
    "うせろ",
    "可悲",
    "受够你",
    "受够了",
    "走开",
    "情けない",
    "うんざり",
    "sick of you",
    "fed up with you",
    "get away from me",
    "shut your",
    "you're the worst",
    "you are the worst",
    "闭上你的嘴",
    "最差劲",
    "差劲",
    "口を閉じ",
    "黙ってろ",
];
const SORRY: &[&str] = &[
    "i was wrong",
    "shouldn't have",
    "my mistake",
    "sorry",
    "apologise",
    "apologize",
    "apologies",
    "my fault",
    "my bad",
    "forgive me",
    "didn't mean",
    "didnt mean",
    "对不起",
    "抱歉",
    "不好意思",
    "ごめん",
    "すみません",
    "すまない",
    "申し訳",
    "悪かった",
    "許して",
    "不是故意",
    "原谅我",
    "请原谅",
    "我错了",
    "apology",
    "an apology",
    "我不对",
    "是我不好",
    "我的错",
    "rude of me",
    "unkind of me",
    "out of line",
    "overstepped",
    "snapped at you",
    "lost my temper",
    "my apologies",
    "didn't mean to",
    "没礼貌",
    "冲你发火",
    "发脾气",
    "失言",
    "言い過ぎ",
    "きつく言って",
    "申し訳ない",
    "お詫び",
    "欠你一个道歉",
    "向你道歉",
    "跟你道歉",
    "给你道歉",
    "owe you an apology",
    "謝らなきゃ",
    "謝りたい",
];
const COMFORT: &[&str] = &[
    "there there",
    "doing your best",
    "trying your best",
    "not your fault",
    "be alright",
    "be okay",
    "be ok",
    "be fine",
    "get through",
    "don't be",
    "dont be",
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
    "get better",
    "chin up",
    "you're not alone",
    "别担心",
    "加油",
    "会好的",
    "会好起来",
    "别难过",
    "心配しないで",
    "元気出して",
    "元気を出して",
    "大丈夫だよ",
    "きっと大丈夫",
    "がんばって",
    "頑張って",
    "負けないで",
    "ひとりじゃない",
    "なんとかなる",
    "泣かないで",
    "我一直在",
    "我在呢",
    "有我在",
    "我陪着你",
    "我会陪",
    "随时找我",
    "頼って",
    "力になる",
    "味方",
    "sort itself out",
    "work out",
    "turn out fine",
    "i'm sure it",
    "it'll all",
    "you've got me",
    "you have me",
    "got your back",
    "you've got this",
    "you got this",
    "get you down",
    "don't let it",
    "没事的",
    "别想太多",
    "想开点",
    "别多想",
    "放宽心",
    "会过去",
    "有什么事跟我说",
    "有事跟我说",
    "有事找我",
    "気にしないで",
    "気にすることない",
    "つらかった",
    "辛かった",
    "大変だった",
    "何かあったら",
    "話してね",
    "聞くよ",
    "落ち込まないで",
    "くよくよしないで",
    "不是你的错",
    "不怪你",
    "别自责",
    "無理しないで",
    "休んでね",
    "ゆっくり休",
    "悪くない",
    "あなたのせいじゃない",
    "なら大丈夫",
    "気にしなくて",
    "开心一点",
    "开心点",
    "放在心上",
    "往心里去",
    "元気出し",
    "気にしない",
    "don't give up",
    "dont give up",
    "never give up",
    "keep going",
    "keep at it",
    "you can do it",
    "you can do this",
    "you'll get there",
    "you will get there",
    "you'll manage",
    "you'll be okay",
    "you'll be alright",
    "you'll be all right",
    "you'll be ok",
    "a new day",
    "new day",
    "keep your chin up",
    "keep smiling",
    "bright side",
    "brighter",
    "better tomorrow",
    "will pass",
    "shall pass",
    "it'll pass",
    "it will pass",
    "hard on yourself",
    "believe in you",
    "proud of yourself",
    "be proud",
    "stronger than",
    "you're strong",
    "you are strong",
    "not as bad",
    "could be worse",
    "all work out",
    "worked out",
    "things'll",
    "going to be okay",
    "going to be fine",
    "going to be alright",
    "going to be all right",
    "all will be well",
    "rooting for you",
    "on your side",
    "i'm with you",
    "count on me",
    "lean on me",
    "there for you",
    "deserve better",
    "you deserve",
    "easy on yourself",
    "take it easy",
    "one day at a time",
    "happens to everyone",
    "worst is over",
    "better days",
    "spirits up",
    "don't lose heart",
    "dont lose heart",
    "lose hope",
    "did your best",
    "did everything you could",
    "done your best",
    "no shame",
    "nothing to be ashamed",
    "plenty of time",
    "don't fret",
    "dont fret",
    "no need to worry",
    "nothing to worry",
    "it's okay",
    "it's ok",
    "it's alright",
    "it's all right",
    "that's okay",
    "that's alright",
    "you matter",
    "别放弃",
    "不要放弃",
    "你一定行",
    "一定行",
    "你能行",
    "你可以的",
    "一定可以",
    "新的一天",
    "明天会更好",
    "会更好",
    "打起精神",
    "振作",
    "坚持",
    "挺住",
    "撑住",
    "别灰心",
    "不要灰心",
    "别泄气",
    "不要泄气",
    "乐观",
    "往好处想",
    "总会有办法",
    "会有办法",
    "车到山前",
    "一切都会",
    "都会好",
    "为自己骄傲",
    "坚强",
    "相信自己",
    "对自己好",
    "别太苛求",
    "别对自己太",
    "风雨过后",
    "阳光总在",
    "支持你",
    "站在你这边",
    "挺你",
    "有我呢",
    "不要紧",
    "没关系",
    "别怕",
    "不用怕",
    "不用担心",
    "别着急",
    "慢慢来",
    "会顺利",
    "好起来的",
    "一定会好",
    "没什么大不了",
    "天无绝人之路",
    "总会过去",
    "会熬过去",
    "熬过去",
    "你行的",
    "鼓起勇气",
    "高兴点",
    "笑一笑",
    "あきらめないで",
    "諦めないで",
    "あきらめるな",
    "諦めるな",
    "きっとできる",
    "できるよ",
    "明日は明日",
    "明日があ",
    "明日はきっと",
    "いい日になる",
    "前向き",
    "自信を持",
    "誇りに思",
    "誇っていい",
    "強いよ",
    "強い人",
    "負けるな",
    "応援して",
    "うまくいくよ",
    "うまくいくって",
    "心配ない",
    "心配いらない",
    "平気だよ",
    "焦らない",
    "焦らず",
    "焦らなくて",
    "一人で抱え",
    "ひとりで抱え",
    "笑って",
    "顔を上げ",
    "胸を張",
    "自分を責め",
    "責めないで",
    "よく頑張",
    "乗り越え",
    "時間が解決",
    "味方だよ",
    "なんとかなるよ",
    "いいことある",
    "いいことがある",
    "次がある",
    "次はきっと",
    "めげないで",
    "へこまないで",
    "元気だして",
    "ファイト",
    "気を落とさ",
    "思ってるより",
    "tomorrow's another day",
    "tomorrow is another day",
    "gonna be okay",
    "gonna be fine",
    "gonna be alright",
    "gonna be all right",
    "things will get",
    "things will look",
    "things will work",
    "things will turn",
    "things will be fine",
    "things will be okay",
    "keep your head up",
    "head held high",
    "nothing wrong",
    "done nothing wrong",
    "not the end of the world",
    "end of the world",
    "try not to",
    "fret",
    "look better",
    "feel better",
    "be better",
    "better in the morning",
    "sleep on it",
    "after a good night",
    "no harm done",
    "not your doing",
    "could happen to anyone",
    "we all make mistakes",
    "everyone makes mistakes",
    "you'll pull through",
    "pull through",
    "keep your pecker up",
    "stiff upper lip",
    "onwards",
    "worse things happen",
    "you did fine",
    "you did well",
    "you're doing fine",
    "doing just fine",
    "you're all right",
    "you're okay",
    "breathe",
    "deep breath",
    "one step at a time",
    "small steps",
    "be kind to yourself",
    "give yourself",
    "you're allowed",
    "no rush",
    "take your time",
    "i'm proud of you",
    "别太",
    "别焦虑",
    "不要焦虑",
    "焦虑",
    "别紧张",
    "不要紧张",
    "别伤心",
    "不要伤心",
    "别哭",
    "睡一觉",
    "就好了",
    "没有错",
    "什么错都没有",
    "你没错",
    "不是你的问题",
    "天塌不下来",
    "塌不下来",
    "为你骄傲",
    "为你感到骄傲",
    "骄傲",
    "会没事",
    "没事儿",
    "放轻松",
    "轻松点",
    "深呼吸",
    "别逼自己",
    "别勉强",
    "有我们在",
    "大家都在",
    "你不是一个人",
    "一个人扛",
    "别一个人",
    "思いつめ",
    "楽になるよ",
    "寝れば",
    "世の終わり",
    "大したことない",
    "大丈夫だって",
    "気楽に",
    "肩の力",
    "泣いていい",
    "無理しなくて",
    "一人じゃない",
    "みんないる",
    "私がいる",
    "ついてる",
    "そばにいる",
    "気にしすぎ",
    "考えすぎ",
    "よくやったよ",
    "偉いよ",
    "上出来",
    "何とかなる",
    "好日子",
    "在后头",
    "气にしちゃだめ",
    "気にしちゃ",
    "気にするな",
    "気にすんな",
    "come right",
    "come good",
    "get to you",
    "let it get",
    "いいことあるよ",
    "i'm here if",
    "here if you",
    "need to talk",
    "if you want to talk",
    "want to talk about it",
    "想聊的话",
    "随时都在",
    "我随时",
    "いつでも聞く",
    "いつでも話",
    "话的话",
    "自责",
    "尽力了",
    "尽力",
    "ベストを尽くし",
    "一番大事",
    "这才是最重要",
    "うまくいく",
    "きっとうまく",
    "全部うまく",
    "辛抱",
    "もう少し",
    "hang on in",
    "hang on",
    "not on your own",
    "not alone",
    "you're never alone",
    "有我陪你",
    "陪着你",
];
const RECONCILE: &[&str] = &[
    "bury the hatchet",
    "hatchet",
    "make up",
    "make peace",
    "make it up",
    "patch things",
    "patch it up",
    "forgive",
    "talk to",
    "should talk",
    "sort it out",
    "say sorry to",
    "apologise to",
    "apologize to",
    "reconcile",
    "和好",
    "原谅",
    "道歉",
    "道个歉",
    "仲直り",
    "謝ったら",
    "謝って",
    "謝りなよ",
    "話し合",
    "許してあげ",
    "謝ったほう",
    "謝るべき",
    "仲良くしな",
    "仲良くして",
    "仲良くしたら",
    "another chance",
    "second chance",
    "one more chance",
    "give him a",
    "give her a",
    "give them a",
    "go easy on",
    "let it go",
    "let bygones",
    "bygones",
    "clear the air",
    "talk it out",
    "talk it through",
    "talking it out",
    "talk things over",
    "talk things through",
    "hear him out",
    "hear her out",
    "hear them out",
    "reach out to",
    "make amends",
    "mend fences",
    "be friends again",
    "friends again",
    "start over",
    "fresh start",
    "patch up",
    "patch things up",
    "forgiving",
    "shake hands",
    "机会",
    "再给",
    "和好吧",
    "好好谈谈",
    "谈一谈",
    "谈谈",
    "聊一聊",
    "沟通",
    "化解",
    "握手言和",
    "冰释前嫌",
    "别计较",
    "不要计较",
    "既往不咎",
    "重归于好",
    "宽容",
    "主动找",
    "去找他",
    "去找她",
    "别记仇",
    "チャンス",
    "許したら",
    "許してやって",
    "水に流",
    "歩み寄",
    "大目に見",
    "やり直",
    "折れて",
    "声をかけてみ",
    "許そう",
    "許すべき",
    "仲直りし",
    "話してみたら",
    "話をしてみ",
    "put it behind",
    "behind you",
    "move on",
    "too short",
    "life's too short",
    "stay cross",
    "stay angry",
    "stay mad",
    "hold a grudge",
    "holding a grudge",
    "grudge",
    "let it lie",
    "water under the bridge",
    "meet halfway",
    "meet him halfway",
    "meet her halfway",
    "take the first step",
    "make the first move",
    "extend a hand",
    "olive branch",
    "be the bigger",
    "别再生",
    "别生",
    "苦短",
    "消消气",
    "别跟他计较",
    "别跟她计较",
    "大度",
    "退一步",
    "握个手",
    "一笑泯恩仇",
    "翻篇",
    "过去就过去",
    "もったいない",
    "根に持たない",
    "根に持つな",
    "いつまでも",
    "大人になって",
    "先に謝",
    "一歩引",
    "水に流す",
    "misses you",
    "miss you",
    "想你",
    "寂しがって",
    "会いたがって",
    "二人で",
    "好好说",
    "说说话",
    "sit down together",
    "talk it over",
    "说声对不起",
    "说对不起",
    "计较",
    "ごめんって言",
    "謝りなって",
    "say sorry",
    "tell him sorry",
    "tell her sorry",
    "apologise to him",
    "apologise to her",
    "making peace",
    "making up",
    "made up with",
    "a chance he",
    "a chance she",
    "a chance they",
    "chance to make",
    "握手",
    "讲和",
    "仲直りする",
    "a chance he's",
    "a chance she's",
    "chance he's",
    "chance she's",
];
const QUARREL: &[&str] = &[
    "avoiding",
    "avoid",
    "went wrong",
    "wrong with",
    "happen with",
    "happened with",
    "between you",
    "fighting",
    "arguing",
    "quarrel",
    "not speaking",
    "not talking",
    "angry",
    "mad at",
    "upset with",
    "cross with",
    "fight",
    "fought",
    "fell out",
    "falling out",
    "argue",
    "argued",
    "argument",
    "problem with",
    "happened between",
    "don't you like",
    "dont you like",
    "吵架",
    "生气",
    "的气",
    "矛盾",
    "不喜欢",
    "けんか",
    "喧嘩",
    "怒って",
    "何かあった",
    "仲が悪",
    "嫌いなの",
    "もめ",
    "腹を立て",
    "根に持",
    "fall out",
    "falls out",
    "row with",
    "had words",
    "闹翻",
    "翻脸",
    "闹矛盾",
    "不和",
    "吵",
    "关系怎么",
    "变差",
    "关系不好",
    "关系差",
    "うまくいってない",
    "うまくいかない",
    "気まず",
    "ぎくしゃく",
    "怎么了",
    "之间",
    "出了什么事",
    "发生了什么",
    "怎么回事",
    "过节",
    "闹别扭",
    "闹僵",
    "不愉快",
    "红过脸",
    "何があった",
    "との間",
    "あいだに",
    "間に何",
    "けんかした",
    "ケンカ",
    "揉め",
    "対立",
    "not on speaking",
    "bad blood",
    "what's going on with",
    "what is going on with",
    "issue with",
    "beef with",
    "the trouble with",
    "trouble between",
    "a row",
    "rowing",
    "bickering",
    "at odds",
    "on the outs",
    "aren't speaking",
    "not speaking to",
    "stopped speaking",
    "why aren't you",
    "speak to each other",
    "speaking to each other",
    "talk to each other",
    "talking to each other",
    "why don't you two",
    "you two fall",
    "the matter between",
    "something between",
    "put out with",
    "annoyed with",
    "sore at",
    "grudge against",
    "不说话",
    "不理",
    "不来往",
    "冷战",
    "互相不",
    "口をきかない",
    "口きかない",
    "話さない",
    "しゃべらない",
    "避けて",
    "無視",
    "fallen out",
    "闹掰",
    "掰了",
    "闹僵了",
    "吵翻",
    "仲違い",
    "もめた",
    "何かされた",
    "did something to you",
    "do something to you",
    "done something to you",
    "对你做了",
    "对你干了",
    "惹你",
    "把你怎么",
    "stopped talking",
    "stop talking",
    "話さなくなった",
    "口をきかなくなった",
    "不说话了",
    "不来往了",
];
const THANK: &[&str] = &[
    "owe you",
    "kind of you",
    "ta",
    "thank",
    "thanks",
    "thank you",
    "cheers",
    "appreciate",
    "appreciated",
    "谢谢",
    "多谢",
    "感谢",
    "ありがと",
    "感謝",
    "助かった",
    "恩に着",
    "助かり",
    "お礼",
    "麻烦你",
    "多亏",
    "幸亏有你",
    "辛苦了",
    "おかげさま",
    "grateful",
    "thankful",
    "appreciate it",
    "much obliged",
    "obliged",
    "lifesaver",
    "life saver",
    "saved me",
    "means a lot",
    "means so much",
    "meant a lot",
    "couldn't have done it without",
    "could not have done it without",
    "bless you",
    "感激",
    "太感谢",
    "感恩",
    "多亏了你",
    "欠你一个人情",
    "人情",
    "真是谢",
    "谢啦",
    "谢了",
    "感谢你",
    "ありがたい",
    "助けてくれて",
    "心から感謝",
    "感謝してる",
    "おかげで助",
    "恩に",
    "サンキュー",
    "どうもありがと",
    "kind to help",
    "good of you",
    "thanks a lot",
    "thanks ever so",
    "ever so grateful",
    "cheers for",
    "much appreciated",
    "i owe you",
    "帮我真是",
    "帮了我",
    "帮了大忙",
    "真是帮了",
    "谢谢你啊",
    "多谢啦",
    "手伝ってくれて",
    "してくれて",
    "どうもね",
    "どうもありがとう",
    "ありがとね",
    "ありがとう",
];
const COMPLIMENT: &[&str] = &[
    "brilliantly",
    "done well",
    "you've done",
    "a star",
    "a gem",
    "treasure",
    "an angel",
    "a legend",
    "great",
    "太好",
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
    "love the",
    "i like your",
    "i like you",
    "love you",
    "you're great",
    "you are great",
    "so kind",
    "very kind",
    "you're kind",
    "nice",
    "fantastic",
    "the best",
    "棒",
    "厉害",
    "好看",
    "漂亮",
    "了不起",
    "真好",
    "すごい",
    "最高",
    "おいしい",
    "よくやった",
    "優しい",
    "やさしい",
    "すてき",
    "素敵",
    "えらい",
    "上手",
    "きれい",
    "good at",
    "really good",
    "so good",
    "talented",
    "clever",
    "skilled",
    "knack",
    "lucky to have",
    "glad to have you",
    "make my day",
    "made my day",
    "makes my day",
    "不错",
    "干得好",
    "做得好",
    "能干",
    "よくやって",
    "いてくれて",
    "おかげ",
    "smart",
    "真精神",
    "气色",
    "好吃",
    "帅",
    "可爱",
    "聪明",
    "センス",
    "似合",
    "かっこいい",
    "かわいい",
    "适合你",
    "衣服",
    "穿得",
    "人很好",
    "对大家",
    "服",
    "kind face",
    "such a kind",
    "you have such",
    "you've got such",
    "you look",
    "looking good",
    "looking well",
    "suits you",
    "handsome",
    "gorgeous",
    "kind heart",
    "warm heart",
    "big heart",
    "generous",
    "inspiring",
    "admire",
    "impressive",
    "impressed",
    "a natural",
    "you're the best",
    "you are the best",
    "good eye",
    "good taste",
    "fine work",
    "fine job",
    "superb",
    "excellent",
    "marvellous",
    "marvelous",
    "splendid",
    "terrific",
    "incredible",
    "awesome",
    "sweet of you",
    "lovely person",
    "kindest",
    "nicest",
    "cleverest",
    "bravest",
    "brave",
    "人真好",
    "真好看",
    "真漂亮",
    "真美",
    "真棒",
    "太棒",
    "好厉害",
    "真厉害",
    "佩服",
    "真行",
    "有才",
    "手艺",
    "温柔",
    "善良",
    "热心",
    "心地好",
    "有本事",
    "有眼光",
    "好帅",
    "真帅",
    "真可爱",
    "真聪明",
    "长得",
    "優しい顔",
    "すてきだね",
    "素敵だね",
    "すばらしい",
    "素晴らしい",
    "さすが",
    "見事",
    "尊敬",
    "立派",
    "上品",
    "器用",
    "天才",
    "頼りになる",
    "笑顔",
    "いい人",
    "いい顔",
    "way with words",
    "a way with",
    "a gift for",
    "so wise",
    "wise",
    "such a good",
    "so good to",
    "gifted",
    "lovely voice",
    "beautiful voice",
    "great cook",
    "good cook",
    "会说话",
    "说话好听",
    "嘴真甜",
    "手真巧",
    "真能干",
    "好能干",
    "手巧",
    "聪明伶俐",
    "懂事",
    "有品位",
    "话が上手",
    "話し上手",
    "聞き上手",
    "手先が器用",
    "料理上手",
    "頭がいい",
    "気が利く",
    "しっかりして",
    "頼もしい",
    "good with people",
    "good with your hands",
    "good with children",
    "good with kids",
    "最好的",
    "一番だ",
    "島で一番",
    "best on the",
    "best in the",
    "頭いい",
    "賢い",
    "是个宝",
    "宝物",
    "a treasure",
    "才能",
    "才华",
    "天赋",
    "センスある",
    "talent",
    "one of a kind",
    "only one like you",
    "独一无二",
    "ほかにいない",
    "他にいない",
    "最好喝",
    "最好吃",
    "好喝",
    "best tea",
    "best bread",
    "best cook",
];
const OPINION: &[&str] = &[
    "make of",
    "reckon",
    "think of",
    "think about",
    "feel about",
    "opinion",
    "like",
    "get on",
    "get along",
    "觉得",
    "看法",
    "どう思う",
    "どんな人",
    "好き",
    "仲良",
    "人怎么样",
    "这个人",
    "为人",
    "熟吗",
    "熟不熟",
    "どんな感じの人",
    "人好吗",
    "人好不好",
    "人怎么",
    "人品",
    "性格",
    "いい人",
    "know him",
    "know her",
    "know them",
    "like him",
    "like her",
    "熟悉",
    "とはうまく",
    "とうまくやって",
];
const HOW_IS: &[&str] = &[
    "how is",
    "how's",
    "hows",
    "doing",
    "how are",
    "is ok",
    "ok",
    "okay",
    "alright",
    "seen",
    "up to",
    "up with",
    "怎么样",
    "还好",
    "最近",
    "元気",
    "どうしてる",
    "会った",
    "様子",
    "大丈夫",
    "好不好",
    "见到",
    "怎么样了",
    "好吗",
    "这阵子",
    "近来",
    "身体",
    "具合",
    "体調",
    "好点",
    "好些",
    "好多了",
    "恢复",
    "怎样",
    "近况",
    "过得",
    "还行吗",
    "状况",
    "情况",
    "良くなった",
    "よくなった",
    "よくなって",
    "良くなって",
    "最近どう",
    "その後",
    "調子",
    "any better",
    "doing better",
    "feeling better",
    "getting on",
    "getting along",
    "holding up",
    "keeping",
    "faring",
    "been keeping",
    "recovered",
    "on the mend",
    "news of",
    "heard from",
    "is he well",
    "is she well",
    "is he ok",
    "is she ok",
    "is he okay",
    "is she okay",
    "he well",
    "she well",
    "he alright",
    "she alright",
    "挺得住",
    "撑得住",
    "挺住",
    "还好吗",
    "持ちこたえ",
    "大丈夫かな",
    "元気かな",
    "元気なの",
    "どうしてるかな",
    "連絡",
    "been well",
    "keeping well",
    "been ok",
    "been okay",
    "been alright",
    "been all right",
    "元気にして",
    "元気でやって",
    "没事吧",
    "没事吗",
    "大丈夫そう",
    "doing okay",
    "getting on all right",
    "過ごしてる",
    "はうまくやって",
    "でうまくやって",
];
const STANDING: &[&str] = &[
    "your trust",
    "glad i'm",
    "do i",
    "make of me",
    "i'm doing",
    "im doing",
    "am i",
    "think i'm",
    "of me",
    "think of me",
    "think about me",
    "feel about me",
    "like me",
    "we friends",
    "trust me",
    "how am i doing",
    "am i doing",
    "mad at me",
    "angry with me",
    "觉得我",
    "喜欢我",
    "信任我",
    "私のこと",
    "僕のこと",
    "俺のこと",
    "私たち友",
    "私を信",
    "私に怒",
    "信頼してる",
    "我们算",
    "我们是朋友",
    "算是朋友",
    "友達だよね",
    "友達でしょ",
    "私たちって",
    "信得过我",
    "相信我",
    "对我印象",
    "我的印象",
    "私の印象",
    "私をどう",
    "讨厌我",
    "恨我",
    "生我的气",
    "私って",
    "役に立って",
    "done right by",
    "how do you see me",
    "what am i to you",
    "am i welcome",
    "welcome here",
    "对我是什么",
    "对我有什么",
    "我对你还算",
    "我对你好",
    "对你好吗",
    "怎么看我",
    "看待我",
    "あなたにちゃんと",
    "してあげられてる",
    "私をどう思",
    "私のことどう",
    "嫌われて",
    "have i upset",
    "did i upset",
    "have i offended",
    "did i offend",
    "upset you",
    "having me around",
    "惹你",
    "我惹",
    "気に障",
    "私がここにいる",
    "我在这儿你",
    "嫌じゃない",
];
const GIFT: &[&str] = &[
    "this for you",
    "brought you",
    "got you",
    "made you",
    "this is for you",
    "these are for you",
    "something for you",
    "present",
    "gift",
    "flowers",
    "here you go",
    "have this",
    "送你",
    "礼物",
    "给你",
    "あげる",
    "プレゼント",
    "贈り物",
    "持ってきた",
    "あなたにと思って",
    "どうぞ",
    "心意",
    "收下",
    "小礼物",
    "ほんの気持ち",
    "気持ちだけど",
    "受け取って",
    "おみやげ",
    "お土産",
    "作ってきた",
    "買ってきた",
    "あなたのために",
    "摘んできた",
    "摘んで",
    "差し上げ",
    "これどうぞ",
    "よかったら",
    "おすそわけ",
    "おすそ分け",
    "送给你",
    "带了点",
    "带了些",
    "给你带",
    "这是给你的",
    "拿着",
    "一点心意",
    "picked these",
    "picked this",
    "picked you",
    "for you to have",
    "a little something",
    "brought these",
    "brought this",
    "thought you'd like",
    "thought you might like",
    "saved you",
    "something i made",
    "baked you",
    "made this for you",
    "焼いてきた",
    "焼いた",
    "持って来た",
    "拿来了",
    "做了个",
    "给你烤",
    "something i baked",
    "baked this",
    "made you a",
    "これ あなたに",
    "あなたにあげ",
    "気に入ってもらえ",
    "会喜欢这个",
    "可能会喜欢",
    "会喜欢的",
    "you might like this",
    "you'd like this",
];
const WORRY: &[&str] = &[
    "sleep",
    "don't look",
    "unwell",
    "worn out",
    "exhausted",
    "the matter",
    "matter with",
    "worrying",
    "tired",
    "lonely",
    "worried",
    "worry",
    "worries",
    "bother",
    "bothering",
    "wrong",
    "sad",
    "seem down",
    "feeling down",
    "look down",
    "upset",
    "stressed",
    "scared",
    "afraid",
    "are you happy",
    "happy here",
    "unhappy",
    "troubled",
    "累",
    "孤单",
    "孤独",
    "担心",
    "烦心",
    "难过",
    "开心吗",
    "不开心",
    "疲れ",
    "さびし",
    "寂し",
    "心配",
    "悲し",
    "落ちこ",
    "落ち込",
    "元気ない",
    "眠れて",
    "悩み",
    "つらそう",
    "悩ん",
    "seem quiet",
    "quiet today",
    "you're quiet",
    "so quiet",
    "something up",
    "what's wrong",
    "whats wrong",
    "what's the matter",
    "怎么不说话",
    "不说话",
    "出什么事",
    "怎么了",
    "有心事",
    "没精神",
    "どうかした",
    "静か",
    "元気がない",
    "浮かない",
    "rough day",
    "hard day",
    "on your mind",
    "脸色",
    "不太好",
    "气色不好",
    "心事",
    "睡得",
    "睡不好",
    "失眠",
    "顔色",
    "具合が悪",
    "調子悪",
    "元気なさそう",
    "眠そう",
    "眠い",
    "seem yourself",
    "not yourself",
    "like yourself",
    "seem off",
    "seem different",
    "something's up",
    "something wrong",
    "something the matter",
    "a bit off",
    "out of sorts",
    "under the weather",
    "pale",
    "down in the dumps",
    "feeling low",
    "look sad",
    "seem sad",
    "look unhappy",
    "look worried",
    "seem worried",
    "look upset",
    "seem upset",
    "what's eating you",
    "what's bothering you",
    "is something bothering",
    "weighing on you",
    "troubling you",
    "what's troubling",
    "haven't been sleeping",
    "been sleeping",
    "tearful",
    "been crying",
    "不对劲",
    "怎么啦",
    "不舒服",
    "不高兴",
    "闷闷不乐",
    "发愁",
    "心情不好",
    "情绪",
    "憔悴",
    "有点怪",
    "不太对",
    "没睡好",
    "哭过",
    "不太开心",
    "压力",
    "烦恼",
    "发生什么事了",
    "愁眉",
    "心里有事",
    "いつもと違う",
    "様子が変",
    "様子がおかし",
    "元気ないね",
    "気になること",
    "気がかり",
    "変だよ",
    "浮かない顔",
    "沈んで",
    "泣いて",
    "眠れてない",
    "寝不足",
    "しょんぼり",
    "ため息",
    "悩み事",
    "心配事",
    "落ち着かない",
    "顔が暗い",
    "暗い顔",
    "weight of the world",
    "on your shoulders",
    "carrying something",
    "carrying a lot",
    "heavy heart",
    "bothering you",
    "eating you",
    "背負",
    "重たい",
    "困ってる",
    "困ってること",
    "抱えて",
    "心事重重",
    "愁眉苦脸",
    "压着",
    "放不下",
    "肩が重",
    "don't look well",
    "not looking well",
    "look unwell",
    "a bit down",
    "bit down",
    "seem down today",
    "低落",
    "情绪低",
    "どうしたの",
    "どうかしたの",
    "元気ないみたい",
    "落ち込んでる",
    "眠れなかった",
    "眠れない",
    "寝られない",
    "睡不着",
];
const FRIENDS: &[&str] = &[
    "on your nerves",
    "annoys you",
    "who annoys",
    "can't stand",
    "cant stand",
    "anyone here",
    "best friend",
    "friends",
    "friend",
    "who do you like",
    "who don't you",
    "who dont you",
    "who do you trust",
    "closest",
    "enemies",
    "enemy",
    "who do you hate",
    "who do you dislike",
    "朋友",
    "和谁",
    "讨厌谁",
    "親友",
    "友だち",
    "友達",
    "仲のいい",
    "仲がいい",
    "苦手な人",
    "嫌いな人",
    "get along with",
    "get on with",
    "don't get on",
    "dont get on",
    "don't get along",
    "dont get along",
    "anyone you",
    "合不来",
    "处得来",
    "处不来",
    "合得来",
    "关系好",
    "best mate",
    "mates",
    "pals",
    "要好",
    "跟谁",
    "谁最",
    "最好的朋友",
    "仲良し",
    "誰と仲",
    "うまくやってる",
    "みんなとは",
    "who do you get on",
    "who do you get along",
    "get on with here",
    "get along with here",
    "who are you close",
    "close to anyone",
    "who's your best",
    "who is your best",
    "who's close",
    "who do you spend",
    "spend time with",
    "who are your friends",
    "anyone you're close",
    "who you like",
    "who you don't",
    "跟谁合得来",
    "和谁合得来",
    "跟谁处得来",
    "和谁处得来",
    "跟谁关系",
    "和谁关系",
    "谁跟你",
    "谁和你",
    "最要好",
    "好朋友",
    "知己",
    "闺蜜",
    "哥们",
    "誰と気が合",
    "気が合う",
    "誰と仲がいい",
    "誰と仲良",
    "一番の親友",
    "親しい人",
    "仲のいい人",
    "誰が好き",
    "仲間",
    "受不了",
    "气场不合",
    "合不来的",
    "合わない人",
    "どうしても合わない",
    "苦手",
    "can't bear",
    "cannot stand",
    "rub you up",
    "don't like here",
    "誰と一緒",
    "跟谁在一起",
    "和谁在一起",
    "跟谁一起",
    "who do you hang",
    "many friends",
    "朋友多",
    "favourite person",
    "favorite person",
    "最喜欢谁",
    "喜欢谁",
    "不喜欢的人",
    "一番好きな人",
    "好きな人",
    "誰が一番",
    "最喜欢的人",
    "親しいの",
    "一番親しい",
    "最亲近",
    "亲近",
];
const FAMILY: &[&str] = &[
    "at home",
    "waiting for you",
    "someone special",
    "special someone",
    "family",
    "married",
    "wife",
    "husband",
    "partner",
    "kids",
    "children",
    "child",
    "son",
    "daughter",
    "parents",
    "mother",
    "father",
    "mum",
    "mom",
    "dad",
    "brother",
    "sister",
    "seeing anyone",
    "single",
    "girlfriend",
    "boyfriend",
    "家人",
    "结婚",
    "孩子",
    "对象",
    "父母",
    "家里",
    "家族",
    "結婚",
    "子ども",
    "子供",
    "恋人",
    "両親",
    "奥さん",
    "旦那",
    "彼氏",
    "彼女",
    "家有",
    "几口人",
    "家里有",
    "兄弟",
    "姐妹",
    "姉妹",
    "お兄",
    "お姉",
    "弟さん",
    "妹さん",
    "お子さん",
    "子供さん",
    "何人家族",
    "brothers",
    "sisters",
    "sons",
    "daughters",
    "siblings",
    "grandchildren",
    "grandkids",
    "grandson",
    "granddaughter",
    "relatives",
    "cousins",
    "cousin",
    "nephew",
    "niece",
    "aunt",
    "uncle",
    "grandma",
    "grandpa",
    "grandmother",
    "grandfather",
    "in-laws",
    "a family",
    "your family",
    "your people at home",
    "兄弟姐妹",
    "儿子",
    "女儿",
    "老婆",
    "老公",
    "妻子",
    "爸妈",
    "爸爸",
    "妈妈",
    "亲戚",
    "孙子",
    "孙女",
    "爷爷",
    "奶奶",
    "外公",
    "外婆",
    "老伴",
    "成家",
    "息子",
    "娘さん",
    "親戚",
    "お孫",
    "孫",
    "おじいちゃん",
    "おばあちゃん",
    "お父さん",
    "お母さん",
    "ご家族",
    "家庭",
    "親御",
    "兄弟は",
    "姉妹は",
    "きょうだい",
    "你丈夫",
    "她丈夫",
    "我丈夫",
    "你先生",
    "你太太",
    "你妻子",
    "your folks",
    "你爸妈",
    "ご両親",
    "親御さん",
];
const ABOUT_YOU: &[&str] = &[
    "makes you tick",
    "grow up",
    "grew up",
    "your story",
    "what are you like",
    "about yourself",
    "about you",
    "who are you",
    "your name",
    "like doing",
    "for fun",
    "hobbies",
    "hobby",
    "where are you from",
    "how old",
    "free time",
    "介绍一下",
    "你自己",
    "你是谁",
    "喜欢做什么",
    "爱好",
    "あなたのこと",
    "自分のこと",
    "趣味",
    "出身",
    "名前",
    "どんな人なの",
    "好きなこと",
    "育った",
    "どこの出",
    "什么样的人",
    "喜欢干什么",
    "平时",
    "平常",
    "业余",
    "多大",
    "几岁",
    "年纪",
    "普段",
    "休みの日",
    "暇な時",
    "いくつ",
    "何歳",
    "年齢",
    "喜欢吃",
    "喜欢什么",
    "好きな食べ物",
    "どこから来",
    "lived here",
    "live here",
    "living here",
    "how long have you",
    "been here long",
    "where do you live",
    "where were you born",
    "born",
    "tell me about your",
    "a bit about",
    "your past",
    "your childhood",
    "your background",
    "what's your story",
    "get to know you",
    "where'd you grow up",
    "住了多久",
    "住多久",
    "在这里多久",
    "在这儿多久",
    "老家",
    "哪里人",
    "哪儿人",
    "家乡",
    "讲讲你",
    "你的事",
    "你的故事",
    "你的过去",
    "小时候",
    "来这里多久",
    "来这儿多久",
    "在这里住",
    "どれくらい住",
    "住んで",
    "どこに住",
    "ここに来て",
    "故郷",
    "地元",
    "あなたについて",
    "あなたの話",
    "昔のこと",
    "子どもの頃",
    "子供の頃",
    "若い頃",
    "どういう人",
    "生まれは",
    "どこで生まれ",
    "生まれた所",
    "生まれたところ",
    "生まれ育",
    "always lived",
    "lived on the island",
    "from round here",
    "from here originally",
    "你一直住",
    "一直住在",
    "一直在这",
    "土生土长",
    "本地人",
    "ずっとこの",
    "ずっとここ",
    "地元の人",
    "ここの出身",
    "your life story",
    "your whole life",
    "favourite thing",
    "favorite thing",
    "like to do",
    "love to do",
    "最喜欢做",
    "最喜欢干",
    "一番好きなこと",
    "何が好き",
    "暮らし",
    "楽しみで",
    "何をするのが好き",
    "life here",
    "your days like",
    "你的生活",
];
const INVITE: &[&str] = &[
    "请你",
    "咖啡",
    "drink",
    "walk",
    "lunch",
    "dinner",
    "coffee",
    "tea",
    "join me",
    "come with me",
    "with me",
    "hang out",
    "wanna",
    "want to go",
    "shall we",
    "let's",
    "lets",
    "fancy a",
    "一起",
    "喝一杯",
    "散步",
    "吃饭",
    "一緒に",
    "散歩",
    "飲みに",
    "一杯",
    "お茶しない",
    "ご飯",
    "食事に",
    "出かけ",
    "fancy coming",
    "fancy going",
    "coming to the",
    "come to the",
    "come along",
    "come out",
    "go out",
    "come round",
    "come over",
    "join us",
    "together",
    "走走",
    "逛逛",
    "出去",
    "行かない",
    "行こう",
    "来ない",
    "でもどう",
    "コーヒー",
    "陪我",
    "散心",
    "坐坐",
    "去坐",
    "喝两杯",
    "来坐",
    "fishing",
    "a pint",
    "a stroll",
    "breakfast",
    "fancy a stroll",
    "行ってみない",
    "見に行かない",
    "supper",
    "a meal",
    "a bite",
    "grab a",
    "have a drink",
    "a cuppa",
    "cup of tea",
    "a cup of",
    "get a drink",
    "go for a",
    "come for a",
    "come and have",
    "come and see",
    "come by",
    "stop by",
    "pop round",
    "pop over",
    "drop round",
    "fancy joining",
    "care to join",
    "would you like to come",
    "like to join",
    "shall we go",
    "how about a",
    "how about we",
    "why don't we",
    "why don't you come",
    "up for a",
    "keen for a",
    "night out",
    "stroll",
    "a ramble",
    "picnic",
    "a swim",
    "a sail",
    "a row out",
    "晚饭",
    "午饭",
    "早饭",
    "吃个饭",
    "喝杯茶",
    "喝茶",
    "喝杯",
    "喝咖啡",
    "走一走",
    "遛遛",
    "兜兜风",
    "出去玩",
    "一块儿",
    "跟我去",
    "和我去",
    "陪我去",
    "去码头",
    "来我家",
    "来我这",
    "来家里",
    "聚聚",
    "聚一聚",
    "野餐",
    "夕食",
    "昼食",
    "朝食",
    "ランチ",
    "ディナー",
    "お茶でも",
    "お茶を",
    "飲みに行",
    "食べに行",
    "歩かない",
    "散歩しない",
    "一緒にどう",
    "寄っていって",
    "うちに来",
    "来ませんか",
    "行きませんか",
    "付き合って",
    "ピクニック",
    "釣りに",
];
const DAY: &[&str] = &[
    "happen today",
    "exciting",
    "on with you",
    "在做什么",
    "get up to",
    "got up to",
    "today been",
    "day been",
    "your day",
    "had a good",
    "had a nice",
    "did you have",
    "been up to",
    "you up to",
    "did you do",
    "what are you doing",
    "been doing",
    "fun today",
    "been busy",
    "忙什么",
    "干什么",
    "做什么了",
    "今日はどう",
    "今日は何",
    "何してた",
    "何してる",
    "忙しかった",
    "一日どう",
    "忙了",
    "今天都",
    "今天做了",
    "今天干了",
    "busy day",
    "今天过得",
    "今天过",
    "开心的事",
    "今天有什么",
    "好玩的",
    "いいことあった",
    "楽しいことあった",
    "干嘛了",
    "今天干嘛",
    "做了什么",
    "どんな日",
    "どんな一日",
    "一日だった",
    "anything nice today",
    "anything fun",
    "done anything",
    "your morning",
    "your afternoon",
    "your evening",
    "the morning go",
    "the day go",
    "today go",
    "how was today",
    "how was the day",
    "how was your",
    "good day today",
    "nice day today",
    "what have you been doing",
    "what did you get up to",
    "up to today",
    "morning been",
    "afternoon been",
    "been up to today",
    "上午过得",
    "下午过得",
    "早上过得",
    "今天如何",
    "今天过得怎么样",
    "今天有啥",
    "今天开心",
    "今天顺利",
    "一天过得",
    "今天一天",
    "午前中",
    "午後は",
    "今朝は",
    "今日一日",
    "何か楽しい",
    "楽しいこと",
    "どうだった",
    "今日はどうだった",
    "今日は何か",
    "何かいいこと",
    "今天忙吗",
    "今天忙不忙",
    "this morning go",
    "今朝は何",
    "up to anything",
    "今日はどんな感じ",
    "今日はどんな",
    "面白いことしてる",
    "何か面白いことしてる",
    "day going",
    "today going",
    "你今天过得",
    "在干嘛",
    "都在干",
    "忙些什么",
    "忙啥",
    "最近何してる",
    "doing with yourself",
    "you done today",
    "done today",
    "朝だった",
    "今日はいい一日",
];
/// Praise that names a bond or a gift of theirs ("a good friend", "the
/// town's lucky to have you"): heard as a compliment before the words in
/// it ("friend") are heard as asking about friends.
const PRAISE: &[&str] = &[
    "good friend",
    "great friend",
    "true friend",
    "good person",
    "good soul",
    "lucky to have you",
    "glad to have you",
    "make my day",
    "made my day",
    "makes my day",
    "good at",
    "proud of you",
    "gift for this",
    "gift for it",
    "real gift",
    "a knack",
    "that's a fine",
    "suits you",
    "looks good on you",
    "you've done so well",
    "喜欢你",
    "很努力",
    "努力了",
    "のことが好き",
    "好きだよ",
    "大好き",
    "做得很好",
    "最好的人",
    "いてうれしい",
    "応援してる",
    "相信你",
    "doing really well",
    "doing so well",
    "doing a great job",
    "you light up",
    "light up the",
    "lights up",
    "明るくなる",
    "有你在",
    "あなたがいると",
    "都亮了",
    "整个港口",
    "without you",
];
/// Words that say one thing whatever else is in them, heard before
/// anything else but a rude word and someone named, in this order.
const SURE: &[(&str, Intent)] = &[
    ("sorry to hear", Intent::Comfort),
    ("goodnight", Intent::Farewell),
    ("good night", Intent::Farewell),
    ("おやすみ", Intent::Farewell),
    ("晚安", Intent::Farewell),
    ("また近いうち", Intent::Farewell),
    ("sorry for your", Intent::Comfort),
    ("sorry about your", Intent::Comfort),
    ("lovely to see you", Intent::Greet),
    ("good to see you", Intent::Greet),
    ("nice to see you", Intent::Greet),
    ("great to see you", Intent::Greet),
    ("glad to see you", Intent::Greet),
    ("见到你真高兴", Intent::Greet),
    ("见到你很高兴", Intent::Greet),
    ("会えてうれしい", Intent::Greet),
    ("会えて嬉しい", Intent::Greet),
    ("商売の調子", Intent::Work),
    ("今日の仕事", Intent::Work),
    ("今天的工作", Intent::Work),
    ("keeping you busy", Intent::Work),
    ("事忙不忙", Intent::Work),
    ("忙しい", Intent::Work),
    ("带点什么", Intent::Need),
    ("要我给你", Intent::Need),
    ("仕事の調子", Intent::Work),
    ("店の調子", Intent::Work),
    ("工作感觉", Intent::Work),
    ("how's work", Intent::Work),
    ("how is work", Intent::Work),
    ("how are you feeling", Intent::HowAreYou),
    ("how are you", Intent::HowAreYou),
    ("感觉怎么样", Intent::HowAreYou),
    ("感觉如何", Intent::HowAreYou),
    ("調子はどう", Intent::HowAreYou),
    ("気分はどう", Intent::HowAreYou),
    ("while i was away", Intent::News),
    ("while i was gone", Intent::News),
    ("talking about", Intent::News),
    ("在聊什么", Intent::News),
    ("聊什么呢", Intent::News),
    ("我不在的时候", Intent::News),
    ("何の話", Intent::News),
    ("いない間", Intent::News),
    ("i feel bad", Intent::Apologize),
    ("feel awful about what i", Intent::Apologize),
    ("cross with me", Intent::Apologize),
    ("过意不去", Intent::Apologize),
    ("悪いと思って", Intent::Apologize),
    ("i'm sorry", Intent::Apologize),
    ("when you're not working", Intent::AboutYou),
    ("when you aren't working", Intent::AboutYou),
    ("不工作的时候", Intent::AboutYou),
    ("仕事がない時", Intent::AboutYou),
    ("仕事がないとき", Intent::AboutYou),
    ("在哪儿长大", Intent::AboutYou),
    ("在哪长大", Intent::AboutYou),
    ("要我去", Intent::Need),
    ("给你买点", Intent::Need),
    ("买点什么", Intent::Need),
    ("買ってこよう", Intent::Need),
    ("你的工作", Intent::Work),
    ("仕事は好き", Intent::Work),
    ("enjoy your job", Intent::Work),
    ("like your job", Intent::Work),
    ("love your job", Intent::Work),
    ("enjoy your work", Intent::Work),
    ("are we good", Intent::Standing),
    ("we're good aren't we", Intent::Standing),
    ("我们俩没事", Intent::Standing),
    ("我们没事吧", Intent::Standing),
    ("私たち 大丈夫", Intent::Standing),
    ("私たち大丈夫", Intent::Standing),
];
/// Taking leave politely, heard before any rude word in it ("邪魔", in
/// the way).
const LEAVING: &[&str] = &[
    "邪魔しちゃ",
    "お邪魔しました",
    "お邪魔します",
    "邪魔したね",
    "邪魔になる",
    "不打扰你",
    "不打扰了",
    "不打搅",
    "leave you in peace",
    "won't keep you",
];
/// Asking about someone's young days, heard before their family ("what
/// were you like as a child?").
const YOUTH: &[&str] = &[
    "as a child",
    "as a kid",
    "when you were young",
    "when you were little",
    "when you were a child",
    "growing up",
    "your childhood",
    "小时候",
    "年轻时",
    "年轻的时候",
    "子どもの頃",
    "子供の頃",
    "子どものころ",
    "小さい頃",
    "若い頃",
];
/// Asking what work someone does, heard before asking about them ("你平时
/// 都做什么工作？" holds 平时, as "what are you like" asks it).
const JOB: &[&str] = &[
    "for a living",
    "your job",
    "do for work",
    "what's your job",
    "where do you work",
    "what work do you",
    "什么工作",
    "哪行",
    "哪一行",
    "在哪工作",
    "在哪儿工作",
    "どんな仕事",
    "何の仕事",
    "仕事は何",
    "仕事をして",
    "お仕事",
    "どこで働",
    "活儿",
    "你干什么活",
];
/// An offer of help, heard before the work in it ("帮忙" holds 忙, busy).
const HELP: &[&str] = &[
    "帮忙",
    "帮得上",
    "帮上",
    "帮你",
    "手を貸",
    "力を貸",
    "手伝おうか",
];
/// Asking after them in so many words, heard before their work ("最近忙吗？
/// 身体好吗？" asks how they are).
const WELL: &[&str] = &["身体好", "身体怎么样", "身体还好", "体調", "具合"];
const PARTING: &[&str] = &[
    "have a nice day",
    "have a good day",
    "have a lovely day",
    "元気でね",
    "have a good evening",
    "have a nice evening",
    "have a lovely evening",
    "enjoy your evening",
    "have a good night",
    "祝你晚上愉快",
    "晚上愉快",
    "祝你愉快",
    "よい夜を",
    "良い夜を",
    "いい夜を",
    "よい一日を",
    "良い一日を",
    "いい一日を",
];
const WEATHER: &[&str] = &[
    "clear up",
    "brighten",
    "chilly",
    "freezing",
    "nippy",
    "lovely day",
    "nice day",
    "beautiful day",
    "grey",
    "gloomy",
    "weather",
    "rain",
    "raining",
    "rainy",
    "sunny",
    "sunshine",
    "sun",
    "cold",
    "hot",
    "windy",
    "wind",
    "storm",
    "snow",
    "snowing",
    "fog",
    "foggy",
    "dust",
    "天气",
    "下雨",
    "冷",
    "刮风",
    "天気",
    "雨",
    "寒い",
    "暑い",
    "晴れ",
    "風が",
    "雪",
    "霧",
    "嵐",
    "风好大",
    "风很大",
    "风大",
    "大风",
    "好热",
    "很热",
    "真热",
    "太热",
    "热死",
    "天热",
    "热吗",
    "热不热",
    "有点热",
    "gorgeous day",
    "glorious day",
    "fine day",
    "glorious",
    "mild",
    "muggy",
    "humid",
    "drizzle",
    "drizzly",
    "overcast",
    "clouds",
    "cloudy",
    "breezy",
    "frost",
    "frosty",
    "icy",
    "blue sky",
    "blue skies",
    "what a day out",
    "lovely out",
    "nice out",
    "grim out",
    "sweltering",
    "scorcher",
    "damp",
    "soaked",
    "wet",
    "天真好",
    "天儿",
    "阴天",
    "多云",
    "凉",
    "暖和",
    "闷热",
    "太阳",
    "刮大风",
    "下雪",
    "起雾",
    "雾大",
    "冷吗",
    "いい天気",
    "曇",
    "涼し",
    "暖か",
    "蒸し暑",
    "肌寒",
    "冷える",
    "日差し",
    "寒いね",
    "寒く",
    "暑く",
    "晴れて",
    "天候",
    "雲",
    "warm today",
    "so warm",
    "warm out",
    "warm day",
    "nice and warm",
    "晴天",
    "晴朗",
    "放晴",
    "天晴",
    "太阳好",
    "wind's",
    "the wind",
    "picking up",
    "blowing",
    "起风",
    "风起",
    "雨になる",
    "風が出",
    "風が強",
    "blustery",
    "gusty",
    "pouring",
    "bucketing",
    "chucking it down",
    "storm's",
    "rain's",
    "sun's",
    "snow's",
    "阳光",
    "晴れてる",
    "よく晴れ",
    "嵐が",
    "有点阴",
    "风啊",
    "好大的风",
    "的风",
    "阴了",
    "曇ってる",
    "beautiful morning",
    "lovely morning",
    "fine morning",
    "glorious morning",
    "美的早晨",
    "早晨真美",
    "きれいな朝",
    "美しい朝",
    "土砂降り",
    "どしゃ降り",
    "大雨",
    "降ってる",
    "降って",
];
const COMING: &[&str] = &[
    "anything coming",
    "special coming",
    "big event",
    "event",
    "party",
    "coming up",
    "this week",
    "plans",
    "festival",
    "holiday",
    "celebration",
    "what's on",
    "whats on",
    "anything on",
    "next",
    "looking forward",
    "节日",
    "活动",
    "计划",
    "お祭り",
    "祭り",
    "予定",
    "今週",
    "行事",
    "イベント",
    "次の",
    "安排",
    "接下来",
    "打算",
    "look forward",
    "过节",
    "庆典",
    "集市",
    "来週",
    "今度の",
    "近いうち",
    "this weekend",
    "weekend",
    "a fair",
    "fair soon",
    "market day",
    "周末",
    "赶集",
    "庙会",
    "集会",
    "週末",
    "市は",
    "市が",
    "縁日",
    "催し",
    "anything planned",
    "planned for",
    "庆祝",
    "次のお祝い",
    "お祝い",
    "夏に何か",
    "summer plans",
    "节是什么时候",
    "节在什么时候",
    "节什么时候",
    "祭",
    "祭りはいつ",
    "今月",
    "this month",
    "这个月",
    "next month",
    "下个月",
    "来月",
    "下周",
    "特别的事",
    "next week",
];
const HOW_ARE_YOU: &[&str] = &[
    "keeping alright",
    "keeping ok",
    "you keeping",
    "how's everything",
    "how is everything",
    "you well",
    "are you well",
    "keeping well",
    "you been",
    "ok with you",
    "everything ok",
    "all good",
    "how are we",
    "今天好吗",
    "how are you",
    "how are things",
    "how r u",
    "how you doing",
    "how are you doing",
    "how's it going",
    "hows it going",
    "how is it going",
    "how's life",
    "how do you feel",
    "you ok",
    "you okay",
    "you alright",
    "are you ok",
    "are you okay",
    "doing ok",
    "feeling",
    "how have you been",
    "what's up",
    "whats up",
    "你好吗",
    "怎么样",
    "还好吗",
    "最近",
    "元気",
    "調子",
    "最近どう",
    "変わりない",
    "大丈夫",
    "doing okay",
    "doing alright",
    "you doing",
    "these days",
    "how you are",
    "see how you",
    "checking in",
    "check in on",
    "checking on you",
    "check on you",
    "look in on",
    "looking in on",
    "popping by",
    "popped by",
    "dropping by",
    "dropped by",
    "alright with you",
    "everything alright",
    "近况",
    "问问你",
    "看看你",
    "来看你",
    "好不好",
    "过得好",
    "过得怎么样",
    "最近过得",
    "顺利",
    "一切还好",
    "一切都好",
    "关心",
    "変わりは",
    "体調",
    "具合",
    "会いに来た",
    "気になって",
    "様子見に",
    "寄ってみた",
    "还好吧",
    "你还好",
    "その後",
    "あれから",
    "お変わり",
    "変わりありません",
    "瞧瞧你",
    "このところ",
    "all right with you",
    "everything all right",
    "you all right",
    "are you all right",
    "all well",
    "all well with you",
    "all good with you",
    "everything good",
    "everything fine",
    "everything okay",
    "everything going",
    "you good",
    "how're you",
    "how are ya",
    "how you been",
    "how've you been",
    "holding up",
    "how goes it",
    "how goes",
    "how's tricks",
    "how's things",
    "how are you getting on",
    "getting on",
    "getting by",
    "faring",
    "treating you",
    "how have things been",
    "life treating",
    "you keeping well",
    "in good health",
    "feeling alright",
    "feeling okay",
    "feeling all right",
    "you fine",
    "身体还好",
    "一切顺利",
    "都顺利",
    "都好吗",
    "过得好吗",
    "还撑得住",
    "撑得住",
    "挺好的吧",
    "过得还行",
    "还行吧",
    "日子过得",
    "身体可好",
    "你怎么样",
    "好着吗",
    "元気にしてた",
    "元気だった",
    "元気してる",
    "変わりなく",
    "無事",
    "問題ない",
    "何も問題",
    "大丈夫そう",
    "やってる",
    "過ごしてる",
    "過ごしてた",
    "どうしてた",
    "無理してない",
    "お元気",
    "様子を見",
    "見に来た",
    "顔を見に",
    "立ち寄",
    "会いに来",
    "元気か見",
    "元気かなと",
    "来看看",
    "过来看看",
    "看你一眼",
    "瞧瞧",
    "just wanted to see",
    "came to see you",
    "come to see you",
    "see how you're",
    "see how you are",
    "make sure you're",
    "make sure you were",
    "look in",
    "came by",
    "come by to see",
    "hadn't seen you",
    "haven't seen you",
    "not seen you",
    "have not seen you",
    "been a while",
    "it's been ages",
    "been ages",
    "可好",
    "近来",
    "しばらく会って",
    "久しぶりに",
    "順調",
    "うまくいってる",
    "都顺利吗",
    "好点了",
    "好些了",
    "好一点了",
    "良くなった",
    "よくなった",
    "feeling better",
];
const NEED: &[&str] = &[
    "i could do",
    "something i can",
    "short of",
    "get you anything",
    "anything you",
    "need",
    "help",
    "anything i can",
    "can i do",
    "what can i",
    "want",
    "money",
    "需要",
    "帮",
    "必要",
    "手伝",
    "できること",
    "困って",
    "ほしいもの",
    "you're after",
    "you after",
    "anything you need",
    "could use",
    "缺不缺",
    "缺什么",
    "足りない",
    "give you a hand",
    "lend a hand",
    "lend you a hand",
    "a hand with",
    "a hand",
    "pitch in",
    "do for you",
    "anything i could",
    "of use",
    "be useful",
    "help out",
    "running low",
    "missing anything",
    "run out of",
    "go without",
    "搭把手",
    "搭个手",
    "用得着",
    "效劳",
    "能为你做",
    "需不需要",
    "缺东西",
    "缺点什么",
    "要我帮",
    "能帮",
    "帮把手",
    "出把力",
    "手伝える",
    "必要なもの",
    "足りないもの",
    "力になれ",
    "できることある",
    "手を貸そう",
    "欲しいもの",
    "不足",
    "make your life easier",
    "easier for you",
    "fetch",
    "anything i can get",
    "bring you",
    "get you something",
    "pick anything up",
    "run an errand",
    "errand",
    "do anything for you",
    "be of help",
    "be of service",
    "轻松一点",
    "轻松些",
    "拿点",
    "帮你拿",
    "给你拿",
    "要我",
    "跑腿",
    "带点什么",
    "捎点",
    "何があれば",
    "取ってこよう",
    "持ってこよう",
    "買ってこよう",
    "お使い",
    "何かしてほしい",
    "してほしいこと",
    "してあげられる",
    "してあげよう",
    "何かしてあげ",
];
const NEWS: &[&str] = &[
    "any word",
    "word from",
    "around town",
    "the word",
    "word is",
    "latest",
    "something interesting",
    "anything interesting",
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
    "新しいこと",
    "何かあった",
    "噂",
    "ニュース",
    "知らせ",
    "面白い話",
    "おもしろい話",
    "有意思",
    "有趣",
    "听说",
    "新闻",
    "八卦",
    "なにかあった",
    "変わったこと",
    "何か新しい",
    "なにか新しい",
    "何があった",
    "町で",
    "何が起き",
    "any gossip",
    "有什么事",
    "什么事吗",
    "面白いこと",
    "おもしろいこと",
    "动静",
    "有什么新",
    "什么新",
    "镇上",
    "新鲜的",
    "what's happening",
    "what's been happening",
    "any news",
    "heard anything",
    "news from",
    "what's the word",
    "anything going on",
    "what's going on",
    "町の様子",
    "最近の話",
    "話題",
    "何か聞いた",
    "新しい話",
    "did i miss",
    "what i missed",
    "i missed",
    "我错过",
    "错过了什么",
    "見逃した",
    "見逃し",
];
const WORK: &[&str] = &[
    "work",
    "job",
    "shift",
    "busy",
    "working",
    "business",
    "trade",
    "what do you do",
    "for a living",
    "工作",
    "生意",
    "忙",
    "仕事",
    "商売",
    "職場",
    "働",
    "上班",
    "在哪工作",
    "做哪行",
    "干哪行",
    "お店",
    "店の調子",
    "活儿",
    "干活",
    "活计",
    "手头",
    "忙不忙",
    "生意好",
    "買い物客",
    "客多",
    "客人多",
    "お客",
    "买卖",
    "生意怎么样",
    "trade been",
    "takings",
    "customers",
    "business good",
    "市场",
    "teaching",
    "teach",
    "教书",
    "教学",
    "上课",
    "教えるの",
    "教える",
    "baking",
    "fishing trips",
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
    "早上好",
    "晚上好",
    "こんにちは",
    "おはよう",
    "こんばんは",
    "やあ",
    "どうも",
    "ただいま",
    "やっほー",
    "long time no see",
    "好久不见",
    "久しぶり",
    "哈喽",
    "哈罗",
    "嘿",
    "おっす",
    "ハロー",
    "good to see you",
    "nice to see you",
    "there you are",
    "greetings",
    "g'day",
    "hello again",
    "hey there",
    "hi there",
    "morning all",
    "good afternoon",
    "what ho",
    "ahoy",
    "又见面",
    "见到你真好",
    "你来啦",
    "您好",
    "早安",
    "下午好",
    "中午好",
    "嗨呀",
    "哈啰",
    "见到你",
    "また会った",
    "また会えた",
    "会えてうれしい",
    "いらっしゃい",
    "どうもどうも",
    "おはようございます",
    "こんにちわ",
    "はじめまして",
];
const FAREWELL: &[&str] = &[
    "off i go",
    "i'm off",
    "leave you to",
    "leave you be",
    "走了",
    "我先走",
    "be off",
    "better go",
    "must go",
    "night night",
    "bye",
    "goodbye",
    "see you",
    "see ya",
    "later",
    "good night",
    "goodnight",
    "farewell",
    "have to go",
    "got to go",
    "gotta go",
    "take care",
    "再见",
    "拜拜",
    "回头见",
    "さようなら",
    "またね",
    "じゃあね",
    "おやすみ",
    "行かなきゃ",
    "バイバイ",
    "また明日",
    "行くね",
    "帰るね",
    "下次再聊",
    "下次聊",
    "改天聊",
    "再聊",
    "回见",
    "我得走了",
    "また今度",
    "またあとで",
    "また話そう",
    "じゃあまた",
    "head off",
    "heading off",
    "head home",
    "heading home",
    "be going",
    "get going",
    "make a move",
    "leave you to it",
    "off now",
    "我走啦",
    "明天见",
    "告辞",
    "晚安",
    "先走",
    "走啦",
    "下次见",
    "再会",
    "失礼する",
    "失礼します",
    "そろそろ",
    "お先に",
    "我回去了",
    "回去了",
    "回家了",
    "我先回",
    "不打扰",
    "不耽误你",
    "不占用你",
    "我先撤",
    "先这样",
    "就这样吧",
    "邪魔しちゃ",
    "お邪魔しました",
    "もう行く",
    "i'll leave you",
    "leave you in peace",
    "let you get on",
    "won't keep you",
    "catch you later",
    "catch you",
    "行かなくちゃ",
    "帰らなくちゃ",
    "帰らなきゃ",
    "行かないと",
    "そろそろ行く",
    "must be off",
    "be off now",
    "cheerio",
    "ta-ra",
    "tara",
    "toodle",
    "toodle-oo",
    "so long",
    "bye bye",
    "ciao",
    "拜啦",
    "拜咯",
];
const ACK: &[&str] = &[
    "alright then",
    "mm",
    "true",
    "exactly",
    "indeed",
    "fair enough",
    "i know",
    "interesting",
    "ok",
    "okay",
    "yeah",
    "yes",
    "yep",
    "no",
    "nope",
    "sure",
    "i see",
    "haha",
    "lol",
    "cool",
    "right",
    "hmm",
    "hm",
    "ah",
    "oh",
    "fine",
    "好的",
    "嗯",
    "哈哈",
    "是的",
    "好吧",
    "うん",
    "そっか",
    "そうか",
    "なるほど",
    "へえ",
    "わかった",
    "はは",
    "そうなんだ",
    "ふうん",
    "对啊",
    "对呀",
    "是啊",
    "没错",
    "确实",
    "原来是这样",
    "这样啊",
    "そうだね",
    "そういうこと",
    "たしかに",
    "だよね",
    "そうそう",
    "そうなの",
    "ほんと",
    "まじ",
    "fair point",
    "good point",
    "point taken",
    "quite right",
    "you're right",
    "you are right",
    "that's true",
    "so true",
    "absolutely",
    "of course",
    "certainly",
    "makes sense",
    "got it",
    "understood",
    "noted",
    "说得对",
    "有道理",
    "对",
    "那倒是",
    "也是",
    "没错啊",
    "确实如此",
    "明白",
    "知道了",
    "そのとおり",
    "その通り",
    "一理ある",
    "確かに",
    "わかる",
    "了解",
    "そうですね",
    "好 行",
    "行吧",
    "行啊",
    "可以",
    "オッケー",
    "了解です",
    "当然",
    "もちろん",
    "sure thing",
    "no doubt",
    "いいよ",
    "没问题",
    "no problem",
];

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

/// What the player's words to `who` mean.
pub fn hear(state: &WorldState, kit: &Kit, who: EntityId, words: &str) -> Heard {
    let text = normal(words);
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
    let is = |phrases: &[&str]| any(&text, phrases);
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
        return heard(Intent::ThinkOf, Some(person));
    }
    if occasion && !is(GIFT) {
        return heard(Intent::Coming, None);
    }
    // Words that say one thing whatever else is in them ("lovely to see
    // you" greets before it praises), the first found.
    if let Some((_, intent)) = SURE.iter().find(|(phrase, _)| has(&text, phrase)) {
        return heard(*intent, None);
    }
    // "Night, Emma." says goodnight.
    if text.starts_with(" night ") || text.starts_with(" nighty ") {
        return heard(Intent::Farewell, None);
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

const INVITED: &str = "conversation.invited";
/// How far back someone remembers what the player said, in periods.
const MEMORY_PERIODS: i64 = 30;

/// When something happened, as someone would say it.
fn when(kit: &Kit, periods: i64) -> String {
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
fn remembered<'a>(world: &'a World, kit: &Kit, who: EntityId) -> Vec<&'a Event> {
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

fn payload_text<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    match event.payload.get(key) {
        Some(Value::Text(text)) => Some(text.as_str()),
        _ => None,
    }
}

/// What someone was given, from the words it came with.
fn gift_of(words: &str) -> &'static str {
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
fn outing_of(words: &str) -> &'static str {
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
fn doing_line(world: &World, kit: &Kit, who: EntityId, event: &Event) -> Option<String> {
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
fn latest_doing(world: &World, kit: &Kit, who: EntityId) -> Option<(i64, String)> {
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
fn recollection_of_words(world: &World, kit: &Kit, who: EntityId) -> Option<(i64, String)> {
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

fn regard_level(regard: i64) -> i8 {
    match regard {
        50.. => 2,
        20..=49 => 1,
        -19..=19 => 0,
        -49..=-20 => -1,
        _ => -2,
    }
}

/// Whether someone would go for a drink or a walk with the player now.
fn accepts_invite(state: &WorldState, kit: &Kit, who: EntityId) -> bool {
    !hurt_recently(state, kit, who)
        && lives::regard(state, who) > -20
        && lives::lack(state, who, Need::Rest) < 70
}

/// The last change between two people, as the later of them would tell it:
/// what kind of change it was, the line its first person said, and when.
fn last_bond(world: &World, who: EntityId, other: EntityId) -> Option<&Event> {
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
fn between(world: &World, kit: &Kit, who: EntityId, other: EntityId) -> Option<String> {
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

fn quarrel(world: &World, kit: &Kit, who: EntityId, other: EntityId, seed: u64) -> String {
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

fn friends(world: &World, kit: &Kit, who: EntityId, seed: u64) -> String {
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

const TRAIT_WORDS: &[(&str, &str)] = &[
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

fn about_you(world: &World, kit: &Kit, who: EntityId) -> String {
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

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn family(world: &World, kit: &Kit, who: EntityId, seed: u64) -> String {
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
        None => pick(
            &[
                "No one of my own. {s} is family enough.",
                "Just me. Everyone in {s} looks out for each other, though.",
            ],
            seed,
        )
        .replace("{s}", kit.settlement),
    }
}

fn worry(world: &World, kit: &Kit, who: EntityId) -> String {
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
fn same_person(state: &WorldState, people: &[EntityId], a: &str, b: &str) -> bool {
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
    let line = |text: String| Reply {
        line: text,
        asks_for: None,
    };
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
        Intent::Coming => line(match (kit.coming_up)(world) {
            Some(coming) => format!(
                "{}{}",
                capitalized(&coming),
                match trait_.as_str() {
                    "shy" => ". I might go, if it's not too crowded.",
                    "sociable" | "warm" => ". I wouldn't miss it!",
                    "prickly" => ". I suppose I'll show my face.",
                    _ => ". I'll be there.",
                }
            ),
            None => "Nothing for a while. Just the usual.".into(),
        }),
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
        Intent::News => {
            let since = state
                .world_time()
                .saturating_sub(kit.period.max(1).saturating_mul(5));
            let news = lives::news_since(world, since);
            let latest = news.iter().rev().take(2).rev().cloned().collect::<Vec<_>>();
            let coming = (kit.coming_up)(world)
                .map(|coming| format!(" And {coming}!"))
                .unwrap_or_default();
            if latest.is_empty() {
                line(format!(
                    "Nothing much. It's been a quiet {} or two in {}.{coming}",
                    kit.unit, kit.settlement
                ))
            } else {
                line(format!(
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
            "{} Ask me how I am, about my day, or about someone here.",
            pick(
                &[
                    "Sorry, I didn't follow.",
                    "Hm? Say that another way?",
                    "I'm not sure what you mean.",
                ],
                seed
            )
        )),
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
    favour::answer(world, kit, who, heard, &mut answer);
    answer
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
    if let Some(care) = care::topic(words) {
        return Ok(care_request(who, words, care));
    }
    let heard = hear(world.state(), kit, who, words);
    let answer = reply(world, kit, who, heard);
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
    let own = reply(world, kit, who, heard);
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
    let own = reply(world, kit, who, heard);
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
            line: answer.into(),
            asks_for,
        },
    ))
    .arg("voiced", "yes"))
}
