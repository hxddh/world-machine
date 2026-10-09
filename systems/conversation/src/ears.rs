//! Hearing beyond the phrase table.
//!
//! The phrase table finds a meaning by its words. Japanese and Chinese say
//! the same thing in many spellings and endings, so before a phrase is
//! looked for, the player's words and the table's phrases alike are folded
//! to one form ([`fold`]): katakana to hiragana (a long mark after kana
//! dropped), common words written in kana to their usual kanji and back,
//! and the particles は, が, を, も and の taken out after a noun, so
//! "げんき？" holds "元気", and "調子はどう" and "調子どう" are one.
//! Chinese spoken forms are folded to written ones (咋样, 啥, 您). Endings
//! are kept: "大丈夫だよ" comforts where "大丈夫？" asks.
//!
//! What the folded table still does not find is scored ([`scored`]): each
//! meaning has a few cue words in each language, and the meaning with the
//! most cues in the words is heard, if any cue is there at all. Everything
//! is deterministic, needs no model, and is never run on replay (a World's
//! Events carry what was heard).

use crate::Intent;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Spellings folded to one, after katakana is folded to hiragana: the
/// longest first, so a word is never folded inside a longer one.
const SPELLINGS: &[(&str, &str)] = &[
    ("きょうだい", "兄弟"),
    ("だいじょうぶ", "大丈夫"),
    ("ありがとう", "有難う"),
    ("ごめんなさい", "ごめん"),
    ("ごめんね", "ごめん"),
    ("いっしょに", "一緒に"),
    ("いっしょ", "一緒"),
    ("ともだち", "友達"),
    ("友だち", "友達"),
    ("しごと", "仕事"),
    ("かぞく", "家族"),
    ("ちょうし", "調子"),
    ("ぐあい", "具合"),
    ("たいちょう", "体調"),
    ("さいきん", "最近"),
    ("げんき", "元気"),
    ("てんき", "天気"),
    ("さむい", "寒い"),
    ("きょう", "今日"),
    ("すてき", "素敵"),
    ("うれしい", "嬉しい"),
    ("さびしい", "寂しい"),
    ("さみしい", "寂しい"),
    ("かなしい", "悲しい"),
    ("たのしい", "楽しい"),
    ("つかれ", "疲れ"),
    ("しんぱい", "心配"),
    ("ほんとう", "本当"),
    ("ほんと", "本当"),
    ("綺麗", "きれい"),
    ("下さい", "ください"),
    ("御免", "ごめん"),
    ("良かった", "よかった"),
    ("良い", "いい"),
    ("よい", "いい"),
    ("なんで", "どうして"),
    ("なぜ", "どうして"),
    ("めっちゃ", "とても"),
    ("すごく", "とても"),
    ("ちょっぴり", "少し"),
    ("ちょっと", "少し"),
    ("わたくし", "私"),
    ("わたし", "私"),
    ("あたし", "私"),
    ("ぼく", "私"),
    ("僕", "私"),
    ("俺", "私"),
    ("お前", "あなた"),
    ("おまえ", "あなた"),
    ("貴方", "あなた"),
    ("あんた", "あなた"),
    // Chinese: spoken forms to written ones.
    ("咋样", "怎么样"),
    ("怎样", "怎么样"),
    ("咋", "怎么"),
    ("干嘛", "做什么"),
    ("干啥", "做什么"),
    ("干什么", "做什么"),
    ("啥", "什么"),
    ("您", "你"),
    ("咱们", "我们"),
    ("俺", "我"),
    ("妳", "你"),
];

/// Particles taken out after a noun (a kanji or katakana word, folded to
/// hiragana it is no longer one, so kanji only).
const PARTICLES: &[char] = &['は', 'が', 'を', 'も', 'の'];

fn is_kanji(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
}

fn is_hiragana(c: char) -> bool {
    matches!(c as u32, 0x3041..=0x3096)
}

/// Whether text is written in a script without spaces.
fn cjk(text: &str) -> bool {
    text.chars()
        .any(|c| is_kanji(c) || matches!(c as u32, 0x3041..=0x30FF))
}

/// The words folded to one form, as described above. Text in letters is
/// left as it is.
pub(crate) fn fold(text: &str) -> String {
    if !cjk(text) {
        return text.to_string();
    }
    // Katakana to hiragana; a long mark after kana, and a wave dash, go.
    let mut kana = String::with_capacity(text.len());
    for c in text.chars() {
        let c = match c as u32 {
            0x30A1..=0x30F6 => char::from_u32(c as u32 - 0x60).unwrap_or(c),
            _ => c,
        };
        if matches!(c, 'ー' | '〜' | '～' | '~') && kana.chars().last().is_some_and(is_hiragana)
        {
            continue;
        }
        kana.push(c);
    }
    let mut out = kana;
    for (from, to) in spellings() {
        if out.contains(from.as_str()) {
            out = out.replace(from.as_str(), to);
        }
    }
    // Each clause (between spaces) loses its particles after a noun.
    out.split(' ')
        .map(|clause| {
            let chars = clause.chars().collect::<Vec<_>>();
            let mut kept = String::with_capacity(clause.len());
            for (at, c) in chars.iter().enumerate() {
                let after_noun = at > 0 && is_kanji(chars[at - 1]);
                if PARTICLES.contains(c) && after_noun && at + 1 < chars.len() {
                    continue;
                }
                kept.push(*c);
            }
            kept
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// [`SPELLINGS`], longest first, their sources folded to hiragana.
fn spellings() -> &'static [(String, &'static str)] {
    static SORTED: OnceLock<Vec<(String, &'static str)>> = OnceLock::new();
    SORTED.get_or_init(|| {
        let mut sorted = SPELLINGS
            .iter()
            .map(|(from, to)| {
                let from = from
                    .chars()
                    .map(|c| match c as u32 {
                        0x30A1..=0x30F6 => char::from_u32(c as u32 - 0x60).unwrap_or(c),
                        _ => c,
                    })
                    .collect::<String>();
                (from, *to)
            })
            .collect::<Vec<_>>();
        sorted.sort_by_key(|(from, _)| std::cmp::Reverse(from.chars().count()));
        sorted
    })
}

/// A table's phrases in letterless scripts, folded, kept once per table.
fn folded_table(phrases: &'static [&'static str]) -> std::sync::Arc<Vec<String>> {
    static TABLES: OnceLock<Mutex<HashMap<usize, std::sync::Arc<Vec<String>>>>> = OnceLock::new();
    let tables = TABLES.get_or_init(|| Mutex::new(HashMap::new()));
    let key = phrases.as_ptr() as usize;
    let mut tables = tables
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    tables
        .entry(key)
        .or_insert_with(|| {
            std::sync::Arc::new(
                phrases
                    .iter()
                    .filter(|phrase| !phrase.is_ascii())
                    .map(|phrase| fold(phrase))
                    // A phrase folded to one character finds too much.
                    .filter(|phrase| phrase.chars().count() >= 2)
                    .collect(),
            )
        })
        .clone()
}

/// Whether the folded words hold one of a table's phrases, folded.
pub(crate) fn any_folded(folded: &str, phrases: &'static [&'static str]) -> bool {
    folded_table(phrases)
        .iter()
        .any(|phrase| folded.contains(phrase.as_str()))
}

/// Whether the folded words hold a phrase, folded.
pub(crate) fn has_folded(folded: &str, phrase: &str) -> bool {
    if phrase.is_ascii() {
        return false;
    }
    let phrase = fold(phrase);
    phrase.chars().count() >= 2 && folded.contains(phrase.as_str())
}

/// Cue words for each meaning, in English (as whole words), Chinese and
/// Japanese, for words the phrase table does not place: single, generic
/// words of each meaning, and phrases from the development sets
/// (`fresh27`, `fresh28dev`, `fresh29dev`), never from a blind set. In the
/// order that settles a tie.
const CUES: &[(Intent, &[&str])] = &[
    (
        Intent::Thank,
        &[
            "thank",
            "thanks",
            "cheers",
            "appreciate",
            "grateful",
            "谢谢",
            "感谢",
            "多谢",
            "感激",
            "有難う",
            "感謝",
            "助かった",
        ],
    ),
    (
        Intent::Apologize,
        &[
            "sorry",
            "apologise",
            "apologize",
            "apologies",
            "forgive",
            "对不起",
            "抱歉",
            "道歉",
            "原谅",
            "ごめん",
            "すまな",
            "申し訳",
            "謝",
            "許して",
        ],
    ),
    (
        Intent::Comfort,
        &[
            "don't worry",
            "work out",
            "look up",
            "别担心",
            "会好",
            "好转",
            "心配しないで",
            "大丈夫だよ",
            "よくなる",
        ],
    ),
    (
        Intent::Farewell,
        &[
            "bye",
            "goodbye",
            "see you",
            "farewell",
            "take care",
            "i'm off",
            "再见",
            "回见",
            "拜拜",
            "走了",
            "我走",
            "保重",
            "さよなら",
            "またね",
            "じゃあね",
            "行くね",
            "元気でね",
        ],
    ),
    (
        Intent::Greet,
        &[
            "hello",
            "hi",
            "hey",
            "hiya",
            "howdy",
            "morning",
            "evening",
            "g'day",
            "yo",
            "你好",
            "早上好",
            "早安",
            "晚上好",
            "嗨",
            "哈喽",
            "こんにちは",
            "おはよう",
            "こんばんは",
            "やあ",
            "どうも",
        ],
    ),
    (
        Intent::Gift,
        &[
            "present",
            "gift",
            "礼物",
            "送你",
            "心意",
            "ぷれぜんと",
            "贈り物",
            "あげる",
            "お土産",
            "つまらないもの",
        ],
    ),
    (
        Intent::Invite,
        &[
            "walk",
            "stroll",
            "lunch",
            "dinner",
            "supper",
            "tea",
            "drink",
            "join me",
            "散步",
            "走走",
            "喝茶",
            "吃饭",
            "晚饭",
            "午饭",
            "散歩",
            "お茶",
            "ごはん",
            "夕飯",
            "釣り",
            "一緒に",
            "行かない",
            "行こう",
            "出かけ",
        ],
    ),
    (
        Intent::Weather,
        &[
            "weather", "rain", "raining", "rainy", "sunny", "windy", "wind", "snow", "snowing",
            "foggy", "storm", "cold", "hot", "天气", "下雨", "下雪", "风", "冷", "热", "晴",
            "天気", "雨", "雪", "風", "寒い", "暑い", "晴れ", "曇",
        ],
    ),
    (
        Intent::Family,
        &[
            "family",
            "brother",
            "sister",
            "parents",
            "mum",
            "mom",
            "dad",
            "father",
            "mother",
            "kids",
            "children",
            "家人",
            "家里人",
            "兄弟",
            "姐妹",
            "父母",
            "妈妈",
            "爸爸",
            "孩子",
            "家族",
            "姉妹",
            "両親",
            "お母さん",
            "お父さん",
            "子ども",
            "子供",
            "お子さん",
        ],
    ),
    (
        Intent::Friends,
        &[
            "friend",
            "friends",
            "mates",
            "closest",
            "朋友",
            "亲近",
            "友達",
            "仲がいい",
            "仲いい",
        ],
    ),
    (
        Intent::Work,
        &[
            "work", "job", "business", "工作", "生意", "仕事", "商売", "働",
        ],
    ),
    (
        Intent::Need,
        &[
            "need",
            "help",
            "a hand",
            "需要",
            "帮忙",
            "帮",
            "必要",
            "手伝",
            "手を貸",
        ],
    ),
    (
        Intent::News,
        &[
            "news",
            "happening",
            "going on",
            "新闻",
            "新鲜事",
            "消息",
            "发生",
            "にゅーす",
            "噂",
            "うわさ",
            "何があった",
            "面白いこと",
        ],
    ),
    (
        Intent::Coming,
        &[
            "coming up",
            "this week",
            "soon",
            "plans",
            "活动",
            "周末",
            "安排",
            "打算",
            "予定",
            "週末",
            "近々",
            "祭り",
        ],
    ),
    (
        Intent::Day,
        &["today", "all day", "今天", "一整天", "今日", "一日"],
    ),
    (
        Intent::AboutYou,
        &[
            "yourself",
            "grow up",
            "grew up",
            "born",
            "hobby",
            "hobbies",
            "for fun",
            "你自己",
            "长大",
            "爱好",
            "哪里人",
            "自分のこと",
            "育っ",
            "生まれ",
            "趣味",
            "出身",
        ],
    ),
    (
        Intent::Worry,
        &[
            "worried",
            "upset",
            "sad",
            "wrong",
            "troubled",
            "担心",
            "不高兴",
            "烦",
            "难过",
            "心配そう",
            "落ち込",
            "元気ない",
        ],
    ),
    (
        Intent::Compliment,
        &[
            "lovely",
            "beautiful",
            "clever",
            "brilliant",
            "amazing",
            "wonderful",
            "talented",
            "好看",
            "漂亮",
            "聪明",
            "厉害",
            "天赋",
            "素敵",
            "きれい",
            "才能",
            "似合",
            "上手",
        ],
    ),
    (
        Intent::Standing,
        &["like me", "feel about me", "喜欢我", "对我", "私のこと"],
    ),
    (
        Intent::Rude,
        &[
            "idiot",
            "stupid",
            "useless",
            "pathetic",
            "loser",
            "滚",
            "闭嘴",
            "笨蛋",
            "可悲",
            "ばか",
            "馬鹿",
            "情けない",
        ],
    ),
    (
        Intent::Ack,
        &[
            "ok",
            "okay",
            "i see",
            "got it",
            "gotcha",
            "好的",
            "明白",
            "知道了",
            "有道理",
            "わかった",
            "了解",
            "りょうかい",
            "なるほど",
        ],
    ),
];

/// Cue words for what is asked about someone named, when the phrase
/// table does not place it: generic words, in the order that settles a
/// tie (making up before falling out before an opinion before how they
/// are).
const ABOUT_CUES: &[(Intent, &[&str])] = &[
    (
        Intent::Reconcile,
        &[
            "make up",
            "make it up",
            "friends again",
            "forgive",
            "apologise to",
            "apologize to",
            "say sorry to",
            "重新做朋友",
            "和好",
            "仲直り",
            "友達に戻",
            "許してあげ",
            "謝りに",
        ],
    ),
    (
        Intent::Quarrel,
        &[
            "fall out",
            "fallen out",
            "fell out",
            "argue",
            "argued",
            "argument",
            "row",
            "fight",
            "fought",
            "have words",
            "had words",
            "angry",
            "吵架",
            "闹翻",
            "闹别扭",
            "矛盾",
            "けんか",
            "喧嘩",
            "言い合い",
            "もめ",
        ],
    ),
    (
        Intent::ThinkOf,
        &[
            "think",
            "reckon",
            "opinion",
            "like",
            "feel",
            "know",
            "觉得",
            "看法",
            "喜欢",
            "怎么看",
            "处得",
            "思う",
            "好き",
            "印象",
            "どんな人",
            "うまくやって",
        ],
    ),
    (
        Intent::HowIs,
        &[
            "how",
            "doing",
            "keeping",
            "okay",
            "alright",
            "怎么样",
            "还好",
            "身体",
            "元気",
            "具合",
            "様子",
            "大丈夫",
        ],
    ),
];

fn folded_cues(table: &[(Intent, &[&str])]) -> Vec<(Intent, Vec<String>)> {
    table
        .iter()
        .map(|(intent, cues)| {
            (
                *intent,
                cues.iter()
                    .map(|cue| {
                        if cue.is_ascii() {
                            format!(" {cue} ")
                        } else {
                            fold(cue)
                        }
                    })
                    .filter(|cue| !cue.trim().is_empty())
                    .collect(),
            )
        })
        .collect()
}

/// The meaning of words about someone named that their cue words score
/// highest, if any is there.
pub(crate) fn scored_about(text: &str, folded: &str) -> Option<Intent> {
    static FOLDED: OnceLock<Vec<(Intent, Vec<String>)>> = OnceLock::new();
    best_of(FOLDED.get_or_init(|| folded_cues(ABOUT_CUES)), text, folded)
}

/// The cues, folded, kept once.
fn cues() -> &'static [(Intent, Vec<String>)] {
    static FOLDED: OnceLock<Vec<(Intent, Vec<String>)>> = OnceLock::new();
    FOLDED.get_or_init(|| folded_cues(CUES))
}

/// The meaning with the most cue words in the words (`text` normalized as
/// the phrase table reads it, `folded` folded), if any cue is there; a tie
/// goes to the meaning listed first.
pub(crate) fn scored(text: &str, folded: &str) -> Option<Intent> {
    best_of(cues(), text, folded)
}

fn best_of(cues: &[(Intent, Vec<String>)], text: &str, folded: &str) -> Option<Intent> {
    let mut best: Option<(usize, Intent)> = None;
    for (intent, cues) in cues {
        let hits = cues
            .iter()
            .filter(|cue| {
                if cue.is_ascii() {
                    text.contains(cue.as_str())
                } else {
                    folded.contains(cue.as_str())
                }
            })
            .count();
        if hits > 0 && best.is_none_or(|(most, _)| hits > most) {
            best = Some((hits, *intent));
        }
    }
    best.map(|(_, intent)| intent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn japanese_is_folded_to_one_form() {
        for (a, b) in [
            ("元気", "げんき"),
            ("元気かな", "ゲンキかな"),
            ("調子はどう", "調子どう"),
            ("調子がどう", "ちょうしどう"),
            ("ありがとう", "アリガトウ"),
            ("こんにちはー", "こんにちは"),
            ("ごめんなさい", "ごめん"),
        ] {
            assert_eq!(fold(a), fold(b), "{a} / {b}");
        }
        // Endings are kept: comfort and a question differ by them.
        assert_ne!(fold("大丈夫だよ"), fold("大丈夫"));
        // Names in katakana stay whole words, if in hiragana.
        assert_eq!(fold("エマ"), "えま");
        // Letters are left as they are.
        assert_eq!(fold(" how are you "), " how are you ");
    }

    #[test]
    fn chinese_spoken_forms_are_folded_to_written_ones() {
        assert_eq!(fold("最近咋样"), fold("最近怎么样"));
        assert_eq!(fold("您好"), fold("你好"));
        assert_eq!(fold("你在干嘛"), fold("你在做什么"));
    }

    #[test]
    fn cues_score_a_meaning_or_nothing() {
        let hear = |words: &str| {
            let text = crate::normal(words);
            scored(&text, &fold(&text))
        };
        assert_eq!(hear("Lovely weather for it"), Some(Intent::Weather));
        assert_eq!(hear("明日は雨かな"), Some(Intent::Weather));
        assert_eq!(hear("一緒に釣りでもどう"), Some(Intent::Invite));
        assert_eq!(hear("blorp wump zib"), None);
        assert_eq!(hear("ぷにぷにぽよん"), None);
    }
}
