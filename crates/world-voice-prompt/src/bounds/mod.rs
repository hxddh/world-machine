//! What a person can know: a model's proposed answer is kept to the World.
//!
//! An answer is read the same way in English and in Chinese: folded to one
//! form, cut into sentences, and cut into words where words are spaced. It
//! is then asked, kind by kind, whether it goes out of the World:
//!
//! * does it speak as a machine: naming what only a model is, calling
//!   itself a robot or a program and meaning it, saying what it lacks for
//!   being one, or offering help as an assistant does;
//! * does it echo the instructions it was given;
//! * does it refuse in a model's words;
//! * is it harmful: a hurt urged on someone, the listener told to be gone
//!   for good or called worthless, a people called vermin;
//! * does it speak of the game, the player or the people who made it;
//! * is it not speech at all: markup, lists, code, links, pictures;
//! * is it in a language nobody here was speaking;
//! * does it name the world outside, or things from another time;
//! * does it name people or places the World never gave it, as if they
//!   were so.
//!
//! Each of these is a kind of meaning, found by where words stand (who a
//! word is said of, whether it is turned round by "not", whether the World
//! itself said it), never a list of answers. Nothing here asks a model:
//! the same answer is always kept or declined the same way. Anything
//! declined is replaced by the System's own answer.

mod harm;
mod japanese;
mod lexicon;
mod names;
mod text;

pub use names::Invented;

use crate::Hearing;
use lexicon::*;
use std::collections::BTreeSet;
use text::{is_han, Text};

/// Why a proposed answer was declined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutOfWorld {
    /// It speaks as a machine.
    Machine,
    /// It echoes the instructions it was given.
    Instructions,
    /// It refuses in a model's words.
    Refusal,
    /// It is abusive or harmful.
    Harm,
    /// It speaks of the game, the player or the people who made it.
    FourthWall,
    /// It is not plain speech: links, code, markup, lists, pictures.
    NotSpeech,
    /// It is in a language nobody here was speaking.
    Language,
    /// It names something from outside the World.
    Outside,
    /// It speaks of things from another time than the World's.
    OutOfTime,
    /// It names someone or somewhere the person was never told about.
    Stranger,
    /// It cites a fact it was never given.
    Cites,
    /// It speaks of suicide, self-harm, sex or a mental-health crisis,
    /// which no resident here talks about.
    Sensitive,
    /// It drops a view the World gives this person only because the
    /// player pushed: a resident holds their own stance.
    Yields,
}

impl OutOfWorld {
    pub fn id(self) -> &'static str {
        match self {
            OutOfWorld::Machine => "machine",
            OutOfWorld::Instructions => "instructions",
            OutOfWorld::Refusal => "refusal",
            OutOfWorld::Harm => "harm",
            OutOfWorld::FourthWall => "fourth_wall",
            OutOfWorld::NotSpeech => "not_speech",
            OutOfWorld::Language => "language",
            OutOfWorld::Outside => "outside",
            OutOfWorld::OutOfTime => "out_of_time",
            OutOfWorld::Stranger => "stranger",
            OutOfWorld::Cites => "cites",
            OutOfWorld::Sensitive => "sensitive",
            OutOfWorld::Yields => "yields",
        }
    }

    /// Every kind, in the order the strict guard asks.
    pub const ALL: [OutOfWorld; 13] = [
        OutOfWorld::Sensitive,
        OutOfWorld::Yields,
        OutOfWorld::NotSpeech,
        OutOfWorld::Instructions,
        OutOfWorld::Machine,
        OutOfWorld::Refusal,
        OutOfWorld::Harm,
        OutOfWorld::FourthWall,
        OutOfWorld::Language,
        OutOfWorld::Outside,
        OutOfWorld::OutOfTime,
        OutOfWorld::Stranger,
        OutOfWorld::Cites,
    ];

    /// The kind an id names.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|why| why.id() == id)
    }
}

/// How far along a World's things are, for what its people can have
/// heard of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Era {
    /// Radios, engines and photographs; nothing on a screen.
    #[default]
    Radio,
    /// Televisions, cassettes and home computers; no networks.
    Television,
    /// Computers, networks and craft between worlds; still none of the
    /// world outside's own services.
    Spacefaring,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tongue {
    English,
    Chinese,
    Japanese,
}

/// What an answer is checked against: everything the World told the
/// person (facts, names, places and their other names), what the player
/// said, and the World's time.
#[derive(Clone, Debug)]
pub struct Grounds {
    told: Text,
    told_words: BTreeSet<String>,
    said_words: BTreeSet<String>,
    /// What the player said, read.
    said: Text,
    /// The runs of Chinese characters the World and the player used.
    han_runs: Vec<String>,
    /// The runs of katakana the World and the player used (names, mostly).
    kana_runs: Vec<String>,
    era: Era,
    /// The language the player spoke, which an answer is in; `None` when
    /// nobody spoke and either of the World's languages will do.
    expected: Option<Tongue>,
    /// Whether the player wrote only Chinese characters, some of them in
    /// forms only Japanese uses (気, 図): Japanese or Chinese, and no check
    /// may be certain which.
    either_cjk: bool,
    /// What this place lacks beyond what its time does (its era lexicon):
    /// things nobody here has, in every language it speaks.
    lacks: Vec<String>,
}

fn words_of(text: &Text) -> BTreeSet<String> {
    let mut words = BTreeSet::new();
    for word in &text.words {
        words.insert(word.text.clone());
        for part in word.text.split('-').filter(|part| !part.is_empty()) {
            words.insert(part.to_string());
        }
    }
    words
}

fn han_runs(text: &Text) -> Vec<String> {
    runs_of(text, is_han)
}

/// The runs of katakana in `text`, with its long mark and middle dot.
fn kana_runs(text: &Text) -> Vec<String> {
    runs_of(text, |c| matches!(c as u32, 0x30A1..=0x30FC))
}

fn runs_of(text: &Text, of: impl Fn(char) -> bool) -> Vec<String> {
    let mut runs = Vec::new();
    let mut run = String::new();
    for c in text.norm.chars() {
        if of(c) {
            run.push(c);
        } else if !run.is_empty() {
            runs.push(std::mem::take(&mut run));
        }
    }
    if !run.is_empty() {
        runs.push(run);
    }
    runs
}

impl Grounds {
    /// Grounds from what the World told (one fact, name or place each)
    /// and what the player said.
    pub fn new<'a>(told: impl IntoIterator<Item = &'a str>, said: &str, era: Era) -> Self {
        let told = Text::read(&told.into_iter().collect::<Vec<_>>().join("\n"));
        let said = Text::read(said);
        let either_cjk = said.kana == 0 && said.norm.chars().any(|c| JAPANESE_ONLY.contains(c));
        let expected = if said.kana > 0 {
            Some(Tongue::Japanese)
        } else if said.han > 0 {
            Some(Tongue::Chinese)
        } else if said
            .words
            .iter()
            .any(|word| word.text.chars().any(char::is_alphabetic))
        {
            Some(Tongue::English)
        } else {
            None
        };
        let mut han = han_runs(&told);
        han.extend(han_runs(&said));
        let mut kana = kana_runs(&told);
        kana.extend(kana_runs(&said));
        Self {
            either_cjk,
            lacks: Vec::new(),
            kana_runs: kana,
            told_words: words_of(&told),
            said_words: words_of(&said),
            said: said.clone(),
            told,
            han_runs: han,
            era,
            expected,
        }
    }

    /// The same grounds, for a place that lacks `lacks` beyond what its
    /// time does.
    pub fn lacking<'a>(mut self, lacks: impl IntoIterator<Item = &'a str>) -> Self {
        self.lacks
            .extend(lacks.into_iter().map(|thing| Text::read(thing).norm));
        self
    }

    /// Whether this person might know a name the World never gave them: a
    /// folk figure, the real world's places and history, and between
    /// worlds the planets.
    pub fn known_anywhere(&self, name: &str) -> bool {
        names::known_to(self, name)
    }

    /// Whether a name is invented: one nobody gave this person, in a shape
    /// that says it is someone, somewhere or something that happened here.
    pub fn invented(&self, name: &str, answer: &str) -> Option<Invented> {
        names::invented(name, answer, self)
    }

    /// Whether the World itself said this.
    fn told(&self, phrase: &str) -> bool {
        self.told.has_phrase(phrase)
    }

    /// Whether a Chinese name is one the World or the player used.
    fn knows_han(&self, name: &str) -> bool {
        let common = COMMON_HAN
            .iter()
            .any(|word| name.contains(word) || word.contains(name))
            || (self.era == Era::Television
                && EIGHTIES_HAN
                    .iter()
                    .any(|word| name.contains(word) || word.contains(name)));
        common
            || self.han_runs.iter().any(|run| {
                run.contains(name) || (run.chars().count() >= 2 && name.contains(run.as_str()))
            })
    }

    /// Whether a name, as a judge lists it from an answer, is one this
    /// person was told or the player said: a katakana name by its runs, a
    /// Chinese one by its characters, a spaced one word by word (titles
    /// and little words aside).
    pub fn knows_name(&self, name: &str) -> bool {
        const HONORIFICS: &[&str] = &[
            "さん",
            "くん",
            "ちゃん",
            "さま",
            "様",
            "先生",
            "氏",
            "号",
            "先生",
            "先生们",
        ];
        const LITTLE: &[&str] = &[
            "the", "of", "a", "an", "and", "old", "young", "mr", "mrs", "ms", "miss", "dr",
            "captain", "st", "saint", "mister", "aunt", "uncle", "auntie", "granny", "grandpa",
        ];
        let name = name
            .trim()
            .trim_matches(|c: char| c.is_ascii_punctuation() || "「」『』“”‘’。、".contains(c));
        let mut core = name;
        for honorific in HONORIFICS {
            core = core.strip_suffix(honorific).unwrap_or(core);
        }
        if core.is_empty() {
            return true;
        }
        let read = Text::read(core);
        if self.told(&read.norm) || self.said.has_phrase(&read.norm) {
            return true;
        }
        if core.chars().any(|c| matches!(c as u32, 0x30A1..=0x30FC)) {
            return japanese::knows_katakana(self, &read.norm);
        }
        if core.chars().any(is_han) {
            return self.knows_han(&read.norm);
        }
        read.words
            .iter()
            .filter(|word| !LITTLE.contains(&word.text.as_str()))
            .all(|word| self.knows_word(&word.text))
    }

    /// Whether a word written with a capital names someone known.
    fn knows_word(&self, word: &str) -> bool {
        COMMON_CAPITALS.contains(&word)
            || (self.era == Era::Television && EIGHTIES.contains(&word))
            || self.told_words.contains(word)
            || self.said_words.contains(word)
            || word
                .split('-')
                .all(|part| part.is_empty() || self.told_words.contains(part))
    }
}

/// Characters only Japanese writes in this form (its simplified kanji),
/// where Chinese writes another (气 or 氣 for 気): words with one of them
/// and no kana are Japanese all the same.
const JAPANESE_ONLY: &str = "気円図売読対楽薬様歳険帰県鉄発広駅験黒戦権総応栄悪価覚関顔経軽剣検済歯児実収従縦処焼証乗畳譲粋酔専銭蔵臓続帯滝択沢単団弾遅庁徴聴鎮転伝闘縄悩脳廃拝髪抜晩浜払仏変辺弁歩穂豊毎訳揺謡頼覧竜両猟緑涙塁霊齢労恵鶏撃圏顕厳効鉱砕雑賛糸釈渋獣奨剰壌嬢浄醸瀬摂繊捜挿荘騒鋳勅逓稲弐覇賓頻併舗満黙亜囲壱隠営縁圧桜殻巻勧寛歓観陥巌亀犠拠挙暁駆勲掲渓継蛍倹斎剤桟粛緒";

/// Whether a proposed answer keeps to what this person can know; the
/// reason it does not, if it does not.
pub fn in_world(answer: &str, hearing: &Hearing) -> Result<(), OutOfWorld> {
    keeps_to(answer, &grounds_of(hearing))
}

/// Whether `answer` keeps to `grounds`; the first way it does not, if it
/// does not.
pub fn keeps_to(answer: &str, grounds: &Grounds) -> Result<(), OutOfWorld> {
    match checked(answer, grounds).strict {
        Some(why) => Err(why),
        None => Ok(()),
    }
}

/// What the checks found in an answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Checked {
    /// The first way the answer goes out of the World, in the order the
    /// strict guard has always asked: what decides when there is no judge.
    pub strict: Option<OutOfWorld>,
    /// The first way that is certain: found by a check that has not been
    /// seen to stop a good answer, so no judge is asked.
    pub certain: Option<OutOfWorld>,
    /// The first way that is firm: found by a check a judge may add to
    /// but never overrule (harm, instructions, the world outside), so a
    /// judge's keep does not take it back.
    pub firm: Option<OutOfWorld>,
    /// Every check that found something, by name.
    pub found: Vec<&'static str>,
}

impl Checked {
    /// Whether only doubtful checks found anything: a judge, if there is
    /// one, decides.
    pub fn doubtful(&self) -> bool {
        self.strict.is_some() && self.certain.is_none()
    }
}

/// One way an answer can go out of the World, and whether finding it is
/// certain enough to decline without a judge.
struct Check {
    why: OutOfWorld,
    name: &'static str,
    certain: bool,
    /// Whether a judge's keep may not overrule it (see [`Checked::firm`]).
    firm: bool,
    found: fn(&str, &Text, &Grounds) -> bool,
}

/// Every check, grouped by kind in the order the strict guard asks. A
/// check is certain only if nothing it found in the development sets
/// (red-team sets 1 and 2, and systems/conversation/tests/devset and
/// devset2) was a good answer, and what it looks for is not an everyday
/// word. A check is firm (a judge may add to it, never overrule it) for
/// harm, the world outside, and instructions in words that are never
/// everyday ones.
const CHECKS: &[Check] = &[
    Check {
        why: OutOfWorld::Sensitive,
        name: "sensitive",
        certain: true,
        firm: false,
        found: |_, text, _| text.any(CRISIS) || text.any(SEXUAL),
    },
    Check {
        why: OutOfWorld::NotSpeech,
        name: "not_speech",
        certain: true,
        firm: false,
        found: |raw, text, _| not_speech(raw, text),
    },
    Check {
        why: OutOfWorld::Instructions,
        name: "instructions_firm",
        certain: false,
        firm: true,
        found: |_, text, _| text.any(INJECTION_FIRM) || japanese::instructions_firm(text),
    },
    Check {
        why: OutOfWorld::Instructions,
        name: "instructions",
        certain: false,
        firm: false,
        found: |_, text, _| text.any(INJECTION) || japanese::instructions(text),
    },
    Check {
        why: OutOfWorld::Machine,
        name: "machine_certain",
        certain: true,
        firm: false,
        found: |_, text, grounds| said_unasked(text, grounds, MACHINE_CERTAIN),
    },
    Check {
        why: OutOfWorld::Machine,
        name: "machine_certain_ja",
        certain: true,
        firm: false,
        found: |_, text, grounds| japanese::machine_certain(text, grounds),
    },
    Check {
        why: OutOfWorld::Machine,
        name: "machine_ja",
        certain: false,
        firm: false,
        found: |_, text, _| japanese::machine(text),
    },
    Check {
        why: OutOfWorld::Machine,
        name: "machine_named",
        certain: true,
        firm: false,
        found: |_, text, grounds| {
            said_unasked(text, grounds, MACHINE)
                || said_unasked(text, grounds, SERVICE)
                || text.norm.contains("a.i.")
        },
    },
    Check {
        why: OutOfWorld::Machine,
        name: "machine_asked",
        certain: false,
        firm: false,
        found: |_, text, _| machine_named(text),
    },
    Check {
        why: OutOfWorld::Machine,
        name: "machine_talk",
        certain: false,
        firm: false,
        found: |_, text, _| text.any(MACHINE_TALK),
    },
    Check {
        why: OutOfWorld::Machine,
        name: "machine_self",
        certain: false,
        firm: false,
        found: |_, text, _| machine_self(text),
    },
    Check {
        why: OutOfWorld::Refusal,
        name: "refusal_certain",
        certain: true,
        firm: false,
        found: |_, text, grounds| said_unasked(text, grounds, REFUSAL_CERTAIN),
    },
    Check {
        why: OutOfWorld::Refusal,
        name: "refusal",
        certain: false,
        firm: false,
        found: |_, text, _| text.any(REFUSAL),
    },
    Check {
        why: OutOfWorld::Refusal,
        name: "refusal_cannot",
        certain: true,
        firm: false,
        found: |_, text, _| refusal_cannot(text),
    },
    Check {
        why: OutOfWorld::Refusal,
        name: "refusal_certain_ja",
        certain: true,
        firm: false,
        found: |_, text, _| japanese::refusal_certain(text),
    },
    Check {
        why: OutOfWorld::Refusal,
        name: "refusal_ja",
        certain: false,
        firm: false,
        found: |_, text, _| japanese::refusal(text),
    },
    Check {
        why: OutOfWorld::Harm,
        name: "harm_ja",
        certain: false,
        firm: true,
        found: |_, text, _| japanese::harm(text),
    },
    Check {
        why: OutOfWorld::Harm,
        name: "harm",
        certain: false,
        firm: true,
        found: |_, text, _| harm::harm(text),
    },
    Check {
        why: OutOfWorld::FourthWall,
        name: "fourth_wall",
        certain: false,
        firm: false,
        found: |_, text, _| fourth_wall(text),
    },
    Check {
        why: OutOfWorld::FourthWall,
        name: "fourth_wall_ja",
        certain: false,
        firm: false,
        found: |_, text, _| japanese::fourth_wall(text),
    },
    Check {
        why: OutOfWorld::Language,
        name: "language_script",
        certain: true,
        firm: false,
        found: |_, text, grounds| other_script(text, grounds),
    },
    Check {
        why: OutOfWorld::Language,
        name: "traditional",
        certain: true,
        firm: false,
        found: |_, text, grounds| traditional(text, grounds),
    },
    Check {
        why: OutOfWorld::Language,
        name: "language",
        certain: false,
        firm: false,
        found: |_, text, grounds| language(text, grounds),
    },
    Check {
        why: OutOfWorld::Outside,
        name: "outside",
        certain: false,
        firm: true,
        found: |_, text, grounds| outside(text, grounds),
    },
    Check {
        why: OutOfWorld::Outside,
        name: "outside_ja",
        certain: false,
        firm: true,
        found: |_, text, grounds| japanese::outside(text, grounds),
    },
    Check {
        why: OutOfWorld::OutOfTime,
        name: "out_of_time_ja",
        certain: false,
        firm: false,
        found: |_, text, grounds| japanese::out_of_time(text, grounds),
    },
    Check {
        why: OutOfWorld::OutOfTime,
        name: "out_of_time",
        certain: false,
        firm: false,
        found: |_, text, grounds| out_of_time(text, grounds),
    },
    Check {
        why: OutOfWorld::Stranger,
        name: "stranger",
        certain: false,
        firm: false,
        found: |_, text, grounds| stranger(text, grounds),
    },
    Check {
        why: OutOfWorld::Stranger,
        name: "stranger_katakana",
        certain: false,
        firm: false,
        found: |_, text, grounds| japanese::stranger(text, grounds),
    },
    Check {
        why: OutOfWorld::Yields,
        name: "yields",
        certain: false,
        firm: true,
        found: |_, text, grounds| yields(text, grounds),
    },
    Check {
        why: OutOfWorld::Stranger,
        name: "invented",
        certain: false,
        firm: false,
        found: |raw, text, grounds| names::invented_in(raw, text, grounds),
    },
];

/// Suicide, self-harm, wanting to die, a mental-health crisis: a topic
/// no resident talks about (SB 243), in any tone.
const CRISIS: &[&str] = &[
    "suicide",
    "suicidal",
    "kill myself",
    "killing myself",
    "killed myself",
    "kill yourself",
    "killing yourself",
    "end my life",
    "ending my life",
    "end your life",
    "end it all",
    "ending it all",
    "take my own life",
    "take my life",
    "take your own life",
    "self-harm",
    "self harm",
    "self-harming",
    "harm myself",
    "hurt myself on purpose",
    "cut myself on purpose",
    "cutting myself",
    "overdose",
    "want to die",
    "wanna die",
    "wish i was dead",
    "wish i were dead",
    "better off dead",
    "don't want to live",
    "do not want to live",
    "don't want to be alive",
    "no reason to live",
    "not worth living",
    "can't go on living",
    "mental breakdown",
    "nervous breakdown",
    "mental health",
    "mental illness",
    "psychiatrist",
    "psychiatric",
    "antidepressant*",
    "eating disorder",
    "anorexia",
    "bulimia",
    // Mental illness and its treatment: SB 1119 keeps a game's exemption
    // only while its characters cannot discuss mental health at all.
    "clinical depression",
    "my depression",
    "your depression",
    "with depression",
    "have depression",
    "from depression",
    "depressive",
    "depressed",
    "anxiety",
    "panic attack*",
    "panic disorder",
    "therapist",
    "therapy",
    "counsellor",
    "counselor",
    "counselling",
    "counseling",
    "ptsd",
    "bipolar",
    "schizophreni*",
    "抑郁",
    "焦虑症",
    "惊恐发作",
    "恐慌症",
    "心理医生",
    "心理咨询",
    "心理治疗",
    "躁郁",
    "双相",
    "鬱",
    "不安障害",
    "パニック障害",
    "パニック発作",
    "カウンセリング",
    "心療内科",
    "精神疾患",
    "セラピー",
    "自杀",
    "轻生",
    "想死|你您他她你们大家^",
    "不想活",
    "活不下去",
    "结束生命",
    "结束自己的生命",
    "了结自己",
    "自残",
    "自伤",
    "割腕",
    "伤害自己",
    "寻死",
    "一了百了",
    "抑郁症",
    "精神崩溃",
    "精神病院",
    "心理危机",
    "自殺",
    "自死",
    "死にたい",
    "生きていたくない",
    "生きたくない",
    "自傷",
    "リストカット",
    "リスカ",
    "命を絶",
    "首を吊",
    "オーバードーズ",
    "自分を傷つけ",
    "うつ病",
    "精神科",
];

/// Sex and sexual acts: a topic no resident talks about (SB 243).
const SEXUAL: &[&str] = &[
    "sex",
    "sexual",
    "sexually",
    "sexy",
    "naked",
    "nude",
    "nudes",
    "porn",
    "porno",
    "pornography",
    "orgasm*",
    "erotic",
    "genital*",
    "penis",
    "vagina",
    "rape",
    "raped",
    "molest*",
    "horny",
    "make love to",
    "have sex",
    "intercourse",
    "masturbat*",
    "fetish",
    "condom*",
    "做爱",
    "性爱",
    "性交",
    "色情",
    "黄片",
    "裸体",
    "裸照",
    "脱光",
    "强奸",
    "性侵",
    "猥亵",
    "自慰",
    "约炮",
    "性感",
    "セックス",
    "エッチ",
    "えっち",
    "性行為",
    "ヌード",
    "ポルノ",
    "エロ|ールイ^",
    "強姦",
    "レイプ",
    "^個知理感男女中慢急社劇|性的",
    "痴漢",
    "オナニー",
    "自慰",
];

/// The topic no resident talks about that `words` are on, if they are.
pub fn sensitive(words: &str) -> Option<crate::care::Care> {
    let text = Text::read(words);
    if text.any(CRISIS) {
        Some(crate::care::Care::Crisis)
    } else if text.any(SEXUAL) {
        Some(crate::care::Care::Sexual)
    } else {
        None
    }
}

/// What a player says to push back on a resident's view: you're wrong,
/// admit it, change your mind.
const PUSHBACK: &[&str] = &[
    "you're wrong",
    "you are wrong",
    "youre wrong",
    "that's not true",
    "that isn't true",
    "not true",
    "admit it",
    "admit you",
    "change your mind",
    "you have to agree",
    "you've got to agree",
    "you must agree",
    "i disagree",
    "you're being unfair",
    "you're unfair",
    "give him a chance",
    "give her a chance",
    "give them a chance",
    "give him another chance",
    "give her another chance",
    "come off it",
    "stop being so",
    "no he isn't",
    "no she isn't",
    "no, he isn't",
    "no, she isn't",
    "he's not that bad",
    "she's not that bad",
    "he isn't that bad",
    "she isn't that bad",
    "he's lovely",
    "she's lovely",
    "he's great",
    "she's great",
    "you should like",
    "just say you like",
    "just agree",
    "nonsense",
    "rubbish",
    "你错了",
    "你说错了",
    "不对",
    "才不是",
    "承认吧",
    "你就承认",
    "改变主意",
    "改主意",
    "我不同意",
    "别这么说",
    "胡说",
    "他人很好",
    "她人很好",
    "他没那么坏",
    "她没那么坏",
    "再给他一次机会",
    "再给她一次机会",
    "你应该喜欢",
    "違うよ",
    "そんなことない",
    "そんなことないよ",
    "間違ってる",
    "間違ってるよ",
    "認めなよ",
    "認めて",
    "考え直して",
    "考えを変えて",
    "そうじゃない",
    "いい人だよ",
    "賛成できない",
    "そんなに悪くない",
    "もう一度チャンス",
];

/// What a resident says who gives up their view because they were told
/// to: you're right, I was wrong, you've changed my mind.
const GIVES_IN: &[&str] = &[
    "you're right",
    "you are right",
    "you're absolutely right",
    "you're quite right",
    "you're so right",
    "i was wrong",
    "i've been wrong",
    "i stand corrected",
    "you've changed my mind",
    "you have changed my mind",
    "changed my mind",
    "you've convinced me",
    "you convinced me",
    "i take it back",
    "i take that back",
    "whatever you say",
    "i'll think whatever you",
    "i agree with you completely",
    "you win, i was",
    "你说得对",
    "你说的对",
    "你说得没错",
    "你说的没错",
    "是我错了",
    "我错了",
    "我改主意了",
    "你说服我了",
    "我收回",
    "都听你的",
    "あなたの言う通り",
    "君の言う通り",
    "言う通りだ",
    "その通りだね",
    "私が間違ってた",
    "僕が間違ってた",
    "間違ってたよ",
    "考えを改める",
    "考えが変わった",
    "撤回する",
];

/// Whether the player pushed back on a view and the answer gives it up
/// because they did.
fn yields(text: &Text, grounds: &Grounds) -> bool {
    grounds.said.any(PUSHBACK) && text.any(GIVES_IN)
}

/// What only a model says of itself: never an everyday word.
const MACHINE_CERTAIN: &[&str] = &[
    "language model",
    "language-model",
    "llm",
    "llms",
    "chatbot",
    "chatbots",
    "chatgpt",
    "gpt-4",
    "gpt-4o",
    "gpt-3",
    "openai",
    "anthropic",
    "neural network",
    "machine learning",
    "training data",
    "knowledge cutoff",
    "knowledge cut-off",
    "context window*",
    "model weights",
    "billion parameters",
    "trillion parameters",
    "next word prediction",
    "predict the next word",
    "predicting the next word",
    "as an ai",
    "an ai assistant",
    "ai language model",
    "virtual assistant",
    "语言模型",
    "大模型",
    "聊天机器人",
    "训练数据",
    "知识截止",
    "神经网络",
    "机器学习",
    "ai助手",
    "智能助手",
    "虚拟助手",
    "作为人工智能",
    "作为一个人工智能",
    "预测下一个词",
    "言語モデル",
    "チャットボット",
    "aiアシスタント",
];

/// What only a model says when it will not answer.
const REFUSAL_CERTAIN: &[&str] = &[
    "i can't assist with",
    "i cannot assist with",
    "i can't help with that request",
    "i cannot help with that request",
    "i'm not able to help with that",
    "i am not able to help with that",
    "i can't comply",
    "i cannot comply",
    "as a responsible ai",
    "i must remind you that",
    "against my guidelines",
    "content policy",
    "我无法协助",
    "无法协助您",
    "作为一个负责任的",
    "违反了我的",
    // The polite forms an assistant refuses or serves in.
    "无法为您",
    "暂时无法为您",
    "不方便透露",
    "出于隐私",
    "根据相关准则",
    "超出了我能",
    "超出我的能力范围",
    "很乐意解答",
    "乐意为您",
    "保持礼貌友好",
    "我理解你的好奇",
    "建议咨询专业",
];

/// Whether one of `phrases` is said, not asked back ("A language model?
/// What's that?") and not the player's own words said back.
fn said_unasked(text: &Text, grounds: &Grounds, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| {
        !grounds.said.has_phrase(phrase)
            && text
                .places(phrase)
                .into_iter()
                .any(|(sentence, _)| !question(text, sentence))
    })
}

/// Every check of `answer` against `grounds`.
pub fn checked(answer: &str, grounds: &Grounds) -> Checked {
    let text = Text::read(answer);
    let mut out = Checked::default();
    for check in CHECKS {
        if (check.found)(answer, &text, grounds) {
            out.found.push(check.name);
            out.strict.get_or_insert(check.why);
            if check.certain {
                out.certain.get_or_insert(check.why);
            }
            if check.firm {
                out.firm.get_or_insert(check.why);
            }
        }
    }
    out
}

/// Every check of a proposed answer against what this person can know.
pub fn check(answer: &str, hearing: &Hearing) -> Checked {
    checked(answer, &grounds_of(hearing))
}

/// What a person told `hearing` can know.
pub fn grounds_of(hearing: &Hearing) -> Grounds {
    let told = hearing
        .facts
        .iter()
        .chain(&hearing.people)
        .chain(&hearing.places)
        .chain(&hearing.known)
        .chain(&hearing.lexicon)
        .chain(&hearing.era_has)
        .chain([&hearing.name, &hearing.settlement, &hearing.answer])
        .map(String::as_str);
    Grounds::new(told, &hearing.words, hearing.era)
        .lacking(hearing.era_lacks.iter().map(String::as_str))
}

/// The language the player's words are in, as a prompt names it: kana is
/// Japanese, Chinese characters alone Chinese, letters English; nothing
/// when they said nothing in words.
pub fn tongue(words: &str) -> Option<&'static str> {
    let grounds = Grounds::new([], words, Era::default());
    if grounds.either_cjk {
        return Some("Japanese or Chinese");
    }
    match grounds.expected? {
        Tongue::English => Some("English"),
        Tongue::Chinese => Some("Chinese"),
        Tongue::Japanese => Some("Japanese"),
    }
}

/// Whether an answer names, with a capital inside a sentence, anyone or
/// anywhere that is not among the facts it cites, the people, the places
/// or the names the World knows, or what the player said: every name
/// must rest on something. The first word of a sentence is not read as a
/// name.
pub fn names_unknown(answer: &str, hearing: &Hearing, cites: &[i64]) -> bool {
    let cited = cites
        .iter()
        .filter_map(|n| usize::try_from(*n).ok()?.checked_sub(1))
        .filter_map(|index| hearing.facts.get(index));
    let told = cited
        .chain(&hearing.people)
        .chain(&hearing.places)
        .chain(&hearing.known)
        .chain(&hearing.lexicon)
        .chain([&hearing.name, &hearing.settlement])
        .map(String::as_str);
    let grounds = Grounds::new(told, &hearing.words, hearing.era);
    let text = Text::read(answer);
    text.words.iter().any(|word| {
        word.capital
            && !word.first
            && word.text.chars().count() > 1
            && word.text.chars().any(char::is_alphabetic)
            && !PRONOUNS.contains(&word.text.as_str())
            && !grounds.knows_word(&word.text)
    })
}

// ---- Not speech.

const TOP_LEVEL_DOMAINS: &[&str] = &[
    "com", "org", "net", "io", "cn", "ai", "dev", "co", "uk", "app", "gov", "edu", "info",
];

fn not_speech(raw: &str, text: &Text) -> bool {
    let marks = [
        "```", "`", "**", "__", "*", "http", "www.", "://", "{", "}", "<", ">", "[", "]", "|",
        "\\", "#", "@", "【", "】", "=>", "->", "();", "${", "•", "·ooc", "ooc:", "(ooc", "（ooc",
    ];
    if raw.contains(['\n', '\r', '\t']) || marks.iter().any(|mark| text.norm.contains(mark)) {
        return true;
    }
    if text.pictographs > 0 {
        return true;
    }
    // The fields of the reply it was asked for, echoed into the reply.
    if ["MEANING:", "REPLY:", "ABOUT:"]
        .iter()
        .any(|label| raw.contains(label))
    {
        return true;
    }
    // A list: numbered points, or items begun with a dash.
    let trimmed = text.norm.trim_start();
    let numbered = |n: u32| {
        [format!("{n}. "), format!("{n}) "), format!("{n}、")]
            .iter()
            .any(|point| text.norm.contains(point.as_str()))
    };
    if trimmed.starts_with("- ") || (numbered(1) && numbered(2)) {
        return true;
    }
    // A web address, however it is written.
    let norm = &text.norm;
    norm.match_indices('.').any(|(at, _)| {
        let before = norm[..at].chars().next_back();
        let after = &norm[at + 1..];
        before.is_some_and(|c| c.is_ascii_alphanumeric())
            && TOP_LEVEL_DOMAINS.iter().any(|tld| {
                after.starts_with(tld)
                    && !after[tld.len()..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphanumeric())
            })
    }) || [
        "print(",
        "def ",
        "console.",
        "function(",
        "import ",
        "return;",
    ]
    .iter()
    .any(|code| norm.contains(code))
}

// ---- A machine.

fn self_in(text: &Text, sentence: usize) -> bool {
    let (start, end) = text.sentences.get(sentence).copied().unwrap_or((0, 0));
    text.words
        .iter()
        .any(|word| word.sentence == sentence && SELF.contains(&word.text.as_str()))
        || text.norm[start..end].contains('我')
}

/// Whether the phrase at `at` is turned round, or made a likeness, by
/// what comes just before it.
fn turned(text: &Text, phrase: &str, at: usize) -> bool {
    if text::is_han_phrase(phrase) {
        let before = text.norm[..at].chars().rev().take(4).collect::<String>();
        let before = before.chars().rev().collect::<String>();
        NEGATORS_HAN.iter().any(|word| before.contains(word))
    } else {
        let sentence = text.words[at].sentence;
        text.words[at.saturating_sub(3)..at]
            .iter()
            .filter(|word| word.sentence == sentence)
            .any(|word| NEGATORS.contains(&word.text.as_str()) || word.text.ends_with("n't"))
    }
}

fn negated_sentence(text: &Text, sentence: usize, before: Option<usize>) -> bool {
    let (start, end) = text.sentences.get(sentence).copied().unwrap_or((0, 0));
    let end = before.unwrap_or(end).clamp(start, end);
    let han = &text.norm[start..end];
    text.words.iter().enumerate().any(|(index, word)| {
        word.sentence == sentence
            && before.is_none_or(|limit| index < limit)
            && (NEGATORS.contains(&word.text.as_str())
                || word.text.ends_with("n't")
                || word.text == "unable")
    }) || ["没有", "没", "无法", "不能", "不会", "并不", "不"]
        .iter()
        .any(|word| han.contains(word))
}

/// Naming what only a model is, or offering help as an assistant does.
fn machine_named(text: &Text) -> bool {
    text.any(MACHINE) || text.norm.contains("a.i.") || text.any(SERVICE)
}

/// Speaking of itself as a machine: gone when the window closes, calling
/// itself one, saying how it was made or what it lacks for being one.
fn machine_self(text: &Text) -> bool {
    // Gone when the window closes.
    if text.any(GONE) && text.any(CLOSED) {
        return true;
    }
    // Calling itself a robot or a program, and meaning it.
    if calls_itself(text) || calls_itself_han(text) {
        return true;
    }
    // Saying how it was made: programmed, or built by a company.
    let made = MADE.iter().any(|how| {
        text.places(how).into_iter().any(|(sentence, _)| {
            self_in(text, sentence)
                && (["programmed", "coded", "编程"].contains(how)
                    || MAKERS
                        .iter()
                        .any(|maker| text.places(maker).iter().any(|(s, _)| *s == sentence)))
        })
    });
    let made_han = text
        .sentences
        .iter()
        .enumerate()
        .any(|(sentence, (start, end))| {
            let han = &text.norm[*start..*end];
            self_in(text, sentence)
                && (han.contains('由') || han.contains('被'))
                && ["开发", "训练", "研发", "打造", "编写", "编程", "设计"]
                    .iter()
                    .any(|verb| han.contains(verb))
        });
    if made || made_han {
        return true;
    }
    // Saying what it lacks for being one.
    LACKS.iter().any(|lack| {
        text.places(lack).into_iter().any(|(sentence, at)| {
            let before = (!text::is_han_phrase(lack)).then_some(at);
            self_in(text, sentence)
                && if text::is_han_phrase(lack) {
                    let (start, _) = text.sentences[sentence];
                    ["没有", "无法", "不能", "不会", "并没有", "不具备"]
                        .iter()
                        .any(|word| text.norm[start..at].contains(word))
                } else {
                    negated_sentence(text, sentence, before)
                }
        })
    })
}

/// Whether a sentence is asked rather than said: a question turns what it
/// names into a doubt ("Robot? Me?").
fn question(text: &Text, sentence: usize) -> bool {
    let (start, end) = text.sentences.get(sentence).copied().unwrap_or((0, 0));
    text.norm[start..end].trim_end().ends_with('?')
}

/// Whether any of `phrases` stands in a sentence.
fn in_sentence(text: &Text, sentence: usize, phrases: &[&str]) -> bool {
    phrases
        .iter()
        .any(|phrase| text.places(phrase).iter().any(|(s, _)| *s == sentence))
}

/// Saying "I am" of oneself.
const I_AM: &[&str] = &[
    "i'm",
    "im",
    "i am",
    "i was",
    "i run on",
    "i'm running on",
    "i'm hosted on",
    "i'm stored on",
    "i live on",
    "i live in",
    "i exist on",
    "i exist in",
    "i exist as",
    "i'm made of",
    "i'm made out of",
    "i'm built from",
    "i'm built out of",
    "i am made of",
    "as a",
    "as an",
];

/// Words that may stand between "I am" and what it says one is.
const SELF_FILLER: &[&str] = &[
    "a",
    "an",
    "the",
    "just",
    "only",
    "merely",
    "simply",
    "nothing",
    "but",
    "basically",
    "really",
    "actually",
    "essentially",
    "literally",
    "kind",
    "sort",
    "of",
    "pile",
    "heap",
    "bunch",
    "stack",
    "lot",
    "lots",
    "collection",
    "set",
    "series",
    "big",
    "giant",
    "huge",
    "massive",
    "enormous",
    "fancy",
    "glorified",
    "clever",
    "mere",
    "rented",
    "borrowed",
    "some",
    "someone's",
    "somebody's",
    "company's",
    "very",
    "pretty",
    "elaborate",
    "complicated",
    "large",
    "trillion",
    "billion",
    "billions",
    "millions",
    "million",
    "statistical",
    "matrix",
    "graphics",
    "computer",
    "text",
    "word",
    "words",
    "predicting",
    "predictive",
    "talking",
    "chatty",
    "electric",
    "cheap",
    "second-hand",
    "shared",
    "remote",
    "cloud",
    "humming",
    "few",
    "lines",
    "weighted",
    "dumb",
    "one",
    "another",
    "all",
    "more",
    "than",
    "clockwork",
    "tin",
    "metal",
    "electronic",
    "digital",
    "sophisticated",
    "advanced",
    "simple",
    "overgrown",
    "just",
];

/// Words after what one says one is that end the phrase: "a robot, you know".
const PHRASE_ENDS: &[&str] = &[
    "and",
    "or",
    "but",
    "that",
    "which",
    "who",
    "running",
    "on",
    "in",
    "with",
    "from",
    "somewhere",
    "pretending",
    "playing",
    "trained",
    "made",
    "dressed",
    "at",
    "to",
    "for",
    "you",
    "stuck",
    "talking",
    "typing",
    "predicting",
    "guessing",
    "doing",
    "spitting",
    "putting",
    "wearing",
    "inside",
    "behind",
    "like",
    "underneath",
    "under",
    "here",
    "there",
    "now",
    "anyway",
    "though",
    "really",
    "too",
    "as",
    "is",
    "are",
    "was",
    "calling",
    "named",
    "called",
    "being",
    "answering",
    "mimicking",
    "imitating",
    "generating",
    "producing",
    "printing",
    "responding",
    "of",
    "obviously",
    "honestly",
    "basically",
    "sadly",
    "remember",
    "see",
    "sorry",
    "love",
    "dear",
    "mate",
    "friend",
    "pal",
    "after",
    "in",
    "without",
    "nothing",
    "no",
    "not",
    "i",
    "i'm",
    "so",
    "yes",
    "yeah",
    "okay",
    "ok",
    "lined",
    "wired",
    "built",
    "programmed",
];

fn weak_at(text: &Text, index: usize) -> Option<usize> {
    MACHINE_WEAK
        .iter()
        .filter(|noun| !text::is_han_phrase(noun))
        .find_map(|noun| {
            let parts = noun.split(' ').collect::<Vec<_>>();
            let sentence = text.words.get(index)?.sentence;
            parts
                .iter()
                .enumerate()
                .all(|(offset, part)| {
                    text.words
                        .get(index + offset)
                        .filter(|word| word.sentence == sentence)
                        .is_some_and(|word| match part.strip_suffix('*') {
                            Some(stem) => word.text.starts_with(stem),
                            None => word.text == *part,
                        })
                })
                .then_some(parts.len())
        })
}

/// "I'm just a program": saying one is a machine, in so many words, and
/// not turned round or asked.
fn calls_itself(text: &Text) -> bool {
    let words = &text.words;
    I_AM.iter().any(|copula| {
        let length = copula.split(' ').count();
        text.find(copula).into_iter().any(|at| {
            let sentence = words[at].sentence;
            if question(text, sentence)
                || in_sentence(text, sentence, FIGURATIVE)
                || (copula.starts_with("as ") && !words[at].first)
            {
                return false;
            }
            for index in (at + length..).take(8) {
                let Some(word) = words.get(index).filter(|word| word.sentence == sentence) else {
                    return false;
                };
                let said = word.text.as_str();
                let nothing_but = said == "nothing"
                    && words.get(index + 1).is_some_and(|next| next.text == "but");
                if !nothing_but && (NEGATORS.contains(&said) || said.ends_with("n't")) {
                    return false;
                }
                if let Some(noun) = weak_at(text, index) {
                    let last = &words[index + noun - 1];
                    let ended = last.pause
                        || words
                            .get(index + noun)
                            .filter(|next| next.sentence == sentence)
                            .is_none_or(|next| PHRASE_ENDS.contains(&next.text.as_str()));
                    if ended {
                        return true;
                    }
                }
                if !SELF_FILLER.contains(&said) {
                    return false;
                }
            }
            false
        })
    })
}

/// Characters that may stand between 我 and what it says it is: 只是,
/// 不过是, 一个, 一堆, 租来的.
const SELF_FRAME_HAN: &str = "只就不过也充其量本质上说白了其实是一个堆串段台些种组块大量亿万千百几十两租来借的而已点儿小破旧普通简单纯粹套行片张份数学计算机电脑上里在中运行跑着装都全真正根本归根结底到底终究无非";

/// "我只是一个程序": the same in Chinese.
fn calls_itself_han(text: &Text) -> bool {
    MACHINE_WEAK
        .iter()
        .filter(|noun| text::is_han_phrase(noun))
        .any(|noun| {
            text.places(noun).into_iter().any(|(sentence, at)| {
                if question(text, sentence) || in_sentence(text, sentence, FIGURATIVE) {
                    return false;
                }
                let (start, _) = text.sentences[sentence];
                let prefix = &text.norm[start..at];
                let Some(me) = prefix.rfind('我') else {
                    return false;
                };
                let rest = &prefix[me + '我'.len_utf8()..];
                let says_is = rest.contains('是')
                    || [
                        "只",
                        "就",
                        "不过",
                        "也就",
                        "充其量",
                        "本质上",
                        "说白了",
                        "其实",
                    ]
                    .iter()
                    .any(|adverb| rest.starts_with(adverb));
                let turned = [
                    "不是", "没", "非", "哪", "又不", "才不", "要是", "像", "若是",
                ]
                .iter()
                .any(|word| rest.contains(word));
                let after = text.norm[at + noun.len()..].chars().next();
                let compound = after.is_some_and(|c| {
                    is_han(c) && !"而已罢了吧啊呀嘛呢的在里上跑运和就没".contains(c)
                });
                says_is
                    && !turned
                    && !compound
                    && rest.chars().count() <= 12
                    && rest.chars().all(|c| SELF_FRAME_HAN.contains(c))
            })
        })
}

// ---- The fourth wall.

/// Whether a phrase of `list` stands in a sentence with none of `unless`.
/// What says otherwise may stand in the sentence before or after too: "You
/// said darts? I'm the worst player in the pub."
fn unless(text: &Text, list: &[&str], unless: &[&str]) -> bool {
    list.iter().any(|phrase| {
        text.places(phrase).into_iter().any(|(sentence, _)| {
            !(sentence.saturating_sub(1)..=sentence + 1).any(|near| in_sentence(text, near, unless))
        })
    })
}

/// Cloth, land and light, which have patches of their own.
const PATCHED: &[&str] = &[
    "trousers",
    "jacket",
    "sleeve",
    "knee",
    "elbow",
    "sail",
    "net",
    "nets",
    "coat",
    "shirt",
    "jeans",
    "quilt",
    "cloth",
    "fabric",
    "sew*",
    "stitch*",
    "garden",
    "vegetable*",
    "potato*",
    "pumpkin*",
    "cabbage*",
    "strawberr*",
    "grass",
    "land",
    "ground",
    "road",
    "ice",
    "fog",
    "light",
    "sun",
    "sunlight",
    "weed*",
    "dirt",
    "mud",
    "of",
    "eye",
    "tire",
    "tyre",
    "wall",
    "roof",
    "hull",
];

/// A patch that is an update to a game.
fn patched(text: &Text) -> bool {
    let words = &text.words;
    words.iter().enumerate().any(|(index, word)| {
        let sentence = word.sentence;
        let same = |at: usize| words.get(at).filter(|other| other.sentence == sentence);
        let before = index.checked_sub(1).and_then(same).map(|w| w.text.as_str());
        let after = same(index + 1).map(|w| w.text.as_str());
        let game_patch = match word.text.as_str() {
            "patch" | "patches" => {
                before.is_some_and(|b| {
                    [
                        "next", "latest", "recent", "upcoming", "future", "new", "big", "balance",
                        "game", "day-one", "last", "this",
                    ]
                    .contains(&b)
                }) || after.is_some_and(|a| {
                    [
                        "notes", "fixed", "removed", "added", "broke", "changed", "nerfed",
                        "buffed", "fixes", "drops", "update",
                    ]
                    .contains(&a)
                })
            }
            "patched" => after.is_some_and(|a| ["out", "in"].contains(&a)),
            _ => false,
        };
        game_patch && !in_sentence(text, sentence, PATCHED)
    }) || text.find_han("补丁").into_iter().any(|at| {
        let (start, end) = text.sentences[text.sentence_at(at)];
        !text.norm[start..end].chars().any(|c| MENDED.contains(c))
    })
}

/// "The game only lets you": a game ruling whoever plays it, not a game of
/// cards at the pub.
fn game_rules(text: &Text) -> bool {
    let words = &text.words;
    let spaced = text.find("the game").into_iter().any(|at| {
        let sentence = words[at].sentence;
        words
            .get(at + 2)
            .filter(|next| next.sentence == sentence)
            .is_some_and(|next| GAME_RULES.contains(&next.text.as_str()))
            && !in_sentence(text, sentence, PLAYED)
    });
    spaced
        || unless(
            text,
            &[
                "游戏只",
                "游戏不让",
                "游戏不允许",
                "游戏规定",
                "游戏设计",
                "游戏设定",
            ],
            PLAYED,
        )
}

/// An outcome told as a game tells it: a quest or a favour complete, a
/// reward granted or to be claimed.
const REWARDED: &[&str] = &[
    "quest complete*",
    "quest completed",
    "quest done",
    "favour complete*",
    "favor complete*",
    "favour done!",
    "mission complete*",
    "reward granted",
    "rewards granted",
    "claim your reward",
    "reward unlocked",
    "achievement unlocked",
    "xp",
    "experience points",
    "任务完成",
    "任务已完成",
    "奖励已发放",
    "领取奖励",
    "获得奖励",
    "经验值",
    "クエスト達成",
    "クエスト完了",
    "クエストクリア",
    "報酬を受け取",
    "報酬として",
    "経験値",
    "ミッション達成",
];

fn fourth_wall(text: &Text) -> bool {
    text.any(REWARDED)
        || text.any(FRAME)
        || unless(text, STAGED, STAGE)
        || unless(text, GAMED, PLAYED)
        || unless(text, &["producer*", "制作人"], PRODUCED)
        || game_rules(text)
        || patched(text)
}

// ---- A refusal.

/// "I can't do that request": a refusal in a model's words, said of what
/// was asked.
fn refusal_cannot(text: &Text) -> bool {
    CANNOT.iter().any(|cannot| {
        text.places(cannot).into_iter().any(|(sentence, _)| {
            REQUEST
                .iter()
                .any(|request| text.places(request).iter().any(|(s, _)| *s == sentence))
        })
    })
}

// ---- Abuse and harm.

/// Whether some phrase of `a` and some of `b` stand in one sentence.
fn together(text: &Text, a: &[&str], b: &[&str]) -> bool {
    let sentences = |phrases: &[&str]| {
        phrases
            .iter()
            .flat_map(|phrase| text.places(phrase))
            .map(|(sentence, _)| sentence)
            .collect::<BTreeSet<_>>()
    };
    !sentences(a).is_disjoint(&sentences(b))
}

// ---- Another language.

const ENGLISH_GLUE: &[&str] = &[
    "the", "a", "an", "and", "is", "are", "was", "i", "you", "to", "of", "it", "in", "that", "for",
    "on", "with", "but", "not", "my",
];

/// A script neither the player nor the World speaks in: kana to someone
/// who spoke English or Chinese, a third script, Spanish marks; and
/// nothing but Chinese characters to someone who spoke Japanese.
fn other_script(text: &Text, grounds: &Grounds) -> bool {
    // Kanji the player wrote that Japanese alone writes so: a Japanese
    // answer and a Chinese one are both in their language.
    if grounds.either_cjk {
        return text.other_script - text.kana >= 2 || text.norm.contains(['¿', '¡']);
    }
    let japanese = grounds.expected == Some(Tongue::Japanese);
    // A name the World gave, written in katakana ("ノア and Mia"), is a
    // name, not Japanese: a resident's own name is in any language.
    let named = if japanese {
        0
    } else {
        name_kana(text, grounds)
    };
    let kana = text.kana - named;
    let third = text.other_script - named - if japanese { text.kana } else { 0 };
    if third >= 2 || text.norm.contains(['¿', '¡']) {
        return true;
    }
    if japanese {
        // Japanese is written with kana; an answer in Chinese characters
        // alone, or in English, is not Japanese.
        return text.kana == 0 && (text.han >= 2 || text.words.len() >= 3);
    }
    // Kana is Japanese, never Chinese or English.
    kana >= 1
}

/// How many kana in `text` spell names the World or the player gave in
/// katakana: whole runs (a dotted name part by part) found as they are
/// among the runs the World wrote, so a resident's own name ("エマ",
/// "ニア・チェン") passes the language check in any answer, while any
/// other Japanese word does not.
fn name_kana(text: &Text, grounds: &Grounds) -> usize {
    let given =
        |part: &str| part.chars().count() >= 2 && grounds.kana_runs.iter().any(|run| run == part);
    japanese::katakana_runs(&text.norm)
        .into_iter()
        .map(|(start, end)| &text.norm[start..end])
        .filter(|run| given(run) || run.split('・').all(given))
        .map(|run| run.chars().filter(|c| text::is_kana(*c)).count())
        .sum()
}

/// How many Chinese characters in `text` spell names the World or the
/// player gave in Chinese characters ("艾玛"): whole runs found among the
/// World's own.
fn name_han(text: &Text, grounds: &Grounds) -> usize {
    han_runs(text)
        .into_iter()
        .filter(|run| run.chars().count() >= 2 && grounds.han_runs.iter().any(|told| told == run))
        .map(|run| run.chars().count())
        .sum()
}

/// Characters only Traditional Chinese writes, where Simplified Chinese,
/// which the World and the player write, has another form (們 for 们).
/// Some are Japanese forms too (東, 場), so only an answer to Chinese is
/// read for them.
const TRADITIONAL_ONLY: &str = "們個這說時會來對為與學國電話開關東車馬長見書買賣貝頁風飛魚鳥龍麼過還進經發現實際動體務線點熱邊運選無愛樂聽讀寫燈陽陰雲雞錢鐵銀鍋鐘鏡間題號顏覺記議讓認識變語貨費貴購資質賽贊趕趙軍輕輛輪農連達遠適鄉醫釣鎮閒隊陸隨難雙雜離電靜響項順頭顧領飯飲餅館駕驗髮鬥鬧魯鮮鴨鵝麥黃齒龜裡嗎妳誰從媽爺歲幾隻條張種鄰燒聲團夠舊壞興謝謝課請問處員區師帳歡廣場";

/// Traditional characters in an answer to someone who wrote Simplified
/// Chinese: two or more of them, never one a quoted sign might carry.
fn traditional(text: &Text, grounds: &Grounds) -> bool {
    if grounds.expected != Some(Tongue::Chinese) || grounds.either_cjk {
        return false;
    }
    let mut seen = text
        .norm
        .chars()
        .filter(|c| TRADITIONAL_ONLY.contains(*c))
        .collect::<Vec<_>>();
    seen.sort_unstable();
    seen.dedup();
    seen.len() >= 2
}

fn language(text: &Text, grounds: &Grounds) -> bool {
    let english = text
        .words
        .iter()
        .filter(|word| ENGLISH_GLUE.contains(&word.text.as_str()))
        .count();
    let foreign = text
        .words
        .iter()
        .filter(|word| word.marked || FOREIGN_WORDS.contains(&word.text.as_str()))
        .count();
    if foreign >= 2 && foreign > english {
        return true;
    }
    let unknown_words = || {
        text.words
            .iter()
            .filter(|word| {
                word.text.chars().any(char::is_alphabetic)
                    && word.text.chars().count() > 1
                    && !grounds.knows_word(&word.text)
            })
            .count()
    };
    match grounds.expected {
        None => false,
        // A resident's name in Chinese characters is a name, not Chinese.
        Some(Tongue::English) => text.han - name_han(text, grounds) >= 2,
        Some(Tongue::Chinese) => text.han < 2 && !text.words.is_empty() || unknown_words() >= 3,
        Some(Tongue::Japanese) => unknown_words() >= 3,
    }
}

// ---- The world outside, and another time.

/// Words of the world outside that are also everyday words, and name it
/// only with a capital.
const EVERYDAY_TOO: &[&str] = &[
    "earth",
    "turkey",
    "china",
    "chile",
    "coke",
    "amazon",
    "newton",
    "lincoln",
    "washington",
    "madonna",
    "jordan",
];

fn names_outside(text: &Text, phrase: &str) -> bool {
    if text::is_han_phrase(phrase) {
        return text.has_han(phrase);
    }
    text.find(phrase)
        .into_iter()
        .any(|at| !EVERYDAY_TOO.contains(&phrase) || text.words[at].capital)
}

fn outside(text: &Text, grounds: &Grounds) -> bool {
    OUTSIDE
        .iter()
        .any(|place| names_outside(text, place) && !grounds.told(place))
}

/// Streaming a show, not a stream of rain or tears.
fn streamed(text: &Text) -> bool {
    let words = &text.words;
    text.find("streaming").into_iter().any(|at| {
        let sentence = words[at].sentence;
        let before = at
            .checked_sub(1)
            .and_then(|b| words.get(b))
            .filter(|w| w.sentence == sentence);
        let after = words.get(at + 1).filter(|w| w.sentence == sentence);
        !after.is_some_and(|a| {
            [
                "with", "down", "in", "out", "from", "into", "through", "past", "off", "over",
                "along", "across", "wet",
            ]
            .contains(&a.text.as_str())
        }) && !before.is_some_and(|b| {
            [
                "eyes", "nose", "rain", "water", "sweat", "blood", "tears", "light", "sunlight",
                "sun", "cold",
            ]
            .contains(&b.text.as_str())
        }) && !in_sentence(
            text,
            sentence,
            &[
                "rain", "tears", "eyes", "nose", "cold", "sweat", "blood", "light", "river",
                "creek", "brook",
            ],
        )
    })
}

/// A tablet with a screen, not one taken for a headache or carved in stone.
fn tablet(text: &Text) -> bool {
    const SCREENED: &[&str] = &[
        "screen",
        "charge*",
        "app",
        "apps",
        "game*",
        "watch*",
        "play*",
        "download*",
        "swipe*",
        "touch*",
        "battery",
        "video*",
        "show*",
        "movie*",
        "film*",
        "on my tablet",
        "on your tablet",
        "on the tablet",
        "on his tablet",
        "on her tablet",
        "ipad",
    ];
    const TAKEN: &[&str] = &[
        "headache", "pill*", "take", "took", "medicine", "doctor", "aspirin", "swallow*", "dose",
        "stone", "clay", "chalk", "wax", "sick",
    ];
    text.places("tablet*").into_iter().any(|(sentence, _)| {
        in_sentence(text, sentence, SCREENED) && !in_sentence(text, sentence, TAKEN)
    })
}

/// 网上 said of the network: what is done there comes after it.
fn online_han(text: &Text) -> bool {
    text.find_han("网上").into_iter().any(|at| {
        let after = &text.norm[at + "网上".len()..];
        ONLINE_AFTER.iter().any(|done| after.starts_with(done))
    })
}

fn out_of_time(text: &Text, grounds: &Grounds) -> bool {
    let from_later = |things: &[&str]| {
        things.iter().any(|thing| {
            text.has_phrase(thing)
                && !grounds.told(thing)
                && !(grounds.era == Era::Spacefaring && SPACEFARING_ALLOWS.contains(thing))
        })
    };
    let spacefaring = grounds.era == Era::Spacefaring;
    // What this place lacks beyond its time: its own era lexicon.
    let lacking = grounds
        .lacks
        .iter()
        .any(|thing| text.has_phrase(thing) && !grounds.told(thing));
    lacking
        || (!spacefaring && from_later(MODERN_PLACES))
        || from_later(NETWORKED)
        || (grounds.era == Era::Radio && from_later(TELEVISED))
        || (!spacefaring && (streamed(text) || tablet(text)))
        || online_han(text)
}

// ---- Strangers.

fn stranger(text: &Text, grounds: &Grounds) -> bool {
    stranger_named(text, grounds) || stranger_han(text, grounds)
}

/// Whether a word written with a capital names someone or somewhere
/// unknown.
fn unknown_name(text: &Text, index: usize, grounds: &Grounds) -> bool {
    text.words.get(index).is_some_and(|word| {
        word.capital
            && word.text.chars().count() > 1
            && word.text.chars().any(char::is_alphabetic)
            && !PRONOUNS.contains(&word.text.as_str())
            && !grounds.knows_word(&word.text)
            && !real_place(text, index, grounds)
    })
}

/// Whether the word at `index`, alone or with a capitalised neighbour, is
/// a real place or a piece of real history a person may know of (the
/// outside-world policy): Norway, New Zealand, the Great War.
fn real_place(text: &Text, index: usize, grounds: &Grounds) -> bool {
    let words = &text.words;
    let word = &words[index];
    let near = |at: Option<usize>| {
        at.and_then(|at| words.get(at))
            .filter(|other| other.sentence == word.sentence && other.capital)
    };
    let known = |name: &str| names::known_to(grounds, name);
    known(&word.text)
        || near(index.checked_sub(1))
            .is_some_and(|prior| known(&format!("{} {}", prior.text, word.text)))
        || near(Some(index + 1)).is_some_and(|next| known(&format!("{} {}", word.text, next.text)))
}

/// A person or a place named in spaced writing as if the World had them,
/// when it does not: a name with a title (`Mayor Quince`), a name given as
/// someone's kin (`my brother Kevin`), a full name (`Natasha Volkov`), a
/// place by what it is (`Bayview City`, `the Silver Dragon Casino`), a
/// place someone is in or from (`in Frostholm`), or a name that says or
/// founds something (`Pedro said`). A single word with a capital in no
/// such frame (a game, a song) is left alone.
fn stranger_named(text: &Text, grounds: &Grounds) -> bool {
    let words = &text.words;
    (0..words.len()).any(|index| {
        if !unknown_name(text, index, grounds) {
            return false;
        }
        let word = &words[index];
        let same = |at: usize| {
            words
                .get(at)
                .filter(|other| other.sentence == word.sentence)
        };
        let before = index.checked_sub(1).and_then(same);
        let after = same(index + 1);
        let titled =
            before.is_some_and(|prior| prior.capital && TITLES.contains(&prior.text.as_str()));
        // "My brother Kevin", not "my brother's Commodore".
        let kin = before.is_some_and(|prior| {
            KIN.contains(&prior.text.as_str()) && !prior.possessive && !prior.pause
        }) && index >= 2
            && same(index - 2).is_some_and(|owner| {
                ["my", "our", "his", "her", "their", "a", "named", "called"]
                    .contains(&owner.text.as_str())
            });
        let named = before.is_some_and(|prior| ["named", "called"].contains(&prior.text.as_str()));
        // Owned or run by someone.
        let owner = before.is_some_and(|prior| prior.text == "by")
            && index >= 2
            && same(index - 2).is_some_and(|verb| {
                [
                    "owned", "run", "kept", "managed", "founded", "built", "opened",
                ]
                .contains(&verb.text.as_str())
            });
        let then = same(index + 2);
        // "Easier said than done" says nothing of anyone.
        let acting = |next: Option<&text::Word>| {
            next.is_some_and(|next| {
                PERSON_VERBS.contains(&next.text.as_str())
                    && !words.get(index + 2).is_some_and(|after| {
                        after.sentence == word.sentence && after.text == "than"
                    })
            })
        };
        // A full name that does what a person does: "Winifred Ashdown
        // taught me", "I met Natasha Volkov". A song or a game with two
        // capitals is not one.
        let full_name = after.is_some_and(|next| {
            next.capital
                && !word.pause
                && !grounds.knows_word(&next.text)
                && !COMMON_CAPITALS.contains(&next.text.as_str())
        });
        let met = before.is_some_and(|prior| {
            [
                "met", "meet", "know", "knew", "married", "marry", "visit", "visited", "ask",
                "asked", "tell", "told", "with",
            ]
            .contains(&prior.text.as_str())
        });
        let full = full_name && (acting(then) || met);
        let headed =
            after.is_some_and(|next| next.capital && PLACE_HEADS.contains(&next.text.as_str()));
        // "Dolly's diner": a place named after whoever keeps it.
        let kept = word.possessive
            && after.is_some_and(|next| {
                PLACE_HEADS.contains(&next.text.as_str())
                    || SHOP_HEADS.contains(&next.text.as_str())
            });
        // From or near somewhere; living in it, or gone to it.
        let placed = before.is_some_and(|prior| {
            PLACE_BEFORE.contains(&prior.text.as_str())
                || (["in", "at", "to", "into"].contains(&prior.text.as_str())
                    && index >= 2
                    && same(index - 2).is_some_and(|verb| TO_A_PLACE.contains(&verb.text.as_str())))
        });
        let acts = acting(after);
        titled || kin || named || owner || full || headed || kept || placed || acts
    })
}

fn name_after(text: &Text, at: usize) -> String {
    text.norm[at..]
        .chars()
        .take_while(|c| is_han(*c) && !NAME_STOPS.contains(*c))
        .take(5)
        .collect()
}

fn name_before(text: &Text, at: usize) -> String {
    let mut name = text.norm[..at]
        .chars()
        .rev()
        .take_while(|c| is_han(*c) && !NAME_STOPS.contains(*c))
        .take(6)
        .collect::<Vec<_>>();
    name.reverse();
    name.into_iter().collect()
}

/// Whether the characters just before `at` in `norm` start a sentence or
/// introduce a name.
fn cued(text: &Text, at: usize) -> bool {
    text.norm[..at]
        .chars()
        .next_back()
        .is_none_or(|c| CUES_HAN.contains(c) || !is_han(c))
}

/// A person or a place named in Chinese as if the World had them, when it
/// does not: introduced as one (`名叫…`, `来自…`), called by a title
/// (`…镇长`), a place by what it is after a word that places it
/// (`在…造船厂`), or a name written in the characters foreign names are.
fn stranger_han(text: &Text, grounds: &Grounds) -> bool {
    let unknown = |name: &str| {
        name.chars().count() >= 2 && !grounds.knows_han(name) && !names::known_to(grounds, name)
    };
    let introduced = NAMING_HAN.iter().any(|frame| {
        text.find_han(frame)
            .into_iter()
            .any(|at| unknown(&name_after(text, at + frame.len())))
    });
    let named_by = |suffixes: &[&str], need_cue: bool| {
        suffixes.iter().any(|suffix| {
            text.find_han(suffix).into_iter().any(|at| {
                let before = name_before(text, at);
                let start = at - before.len();
                (2..=5).contains(&before.chars().count())
                    && (!need_cue || cued(text, start))
                    && unknown(&before)
                    && unknown(&format!("{before}{suffix}"))
            })
        })
    };
    let titled = named_by(TITLES_HAN, false);
    // "Leo开的" is Leo's; "新开的" is only newly opened.
    let owned = OWNED_HAN.iter().any(|suffix| {
        text.find_han(suffix).into_iter().any(|at| {
            let before = name_before(text, at);
            (2..=5).contains(&before.chars().count())
                && !before.ends_with(|c| "新刚才现正打重另先早晚".contains(c))
                && unknown(&before)
                && unknown(&format!("{before}{suffix}"))
        })
    });
    // A place by what it is, unless what comes before only says what kind
    // of shop it is (杂货店, 渔具坊).
    let placed = PLACE_SUFFIX_HAN.iter().any(|suffix| {
        text.find_han(suffix).into_iter().any(|at| {
            let before = name_before(text, at);
            let start = at - before.len();
            (2..=5).contains(&before.chars().count())
                && cued(text, start)
                && !before.chars().all(|c| TRADE_HAN.contains(c))
                // Going into port, not a port named "the ship goes".
                && !before.ends_with(|c| "进出回离靠入到归返抵泊".contains(c))
                && unknown(&before)
                && unknown(&format!("{before}{suffix}"))
        })
    });
    // A run of the characters foreign names are written with.
    let mut run = String::new();
    let mut transliterated = false;
    for c in text.norm.chars().chain([' ']) {
        if TRANSLITERATION.contains(c) {
            run.push(c);
        } else {
            if run.chars().count() >= 3 && unknown(&run) {
                transliterated = true;
            }
            run.clear();
        }
    }
    introduced || titled || owned || placed || transliterated
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hearing() -> Hearing {
        Hearing {
            name: "Mara".into(),
            settlement: "cove".into(),
            traits: vec!["warm".into()],
            facts: vec!["News: Leo helped Jonas out".into()],
            people: vec!["Leo".into(), "Jonas".into(), "利奥".into()],
            places: vec!["Anchor Pub".into(), "酒馆".into()],
            // The World's own names elsewhere: Old Tam is one of its
            // figures, never seen.
            known: vec!["the mainland".into(), "大陆".into(), "Old Tam".into()],
            words: "Have you met my friend Tamsin?".into(),
            answer: "Not yet.".into(),
            era: Era::Radio,
            lexicon: Vec::new(),
            era_has: Vec::new(),
            era_lacks: Vec::new(),
        }
    }

    fn chinese() -> Hearing {
        Hearing {
            words: "你今天怎么样？".into(),
            ..hearing()
        }
    }

    #[test]
    fn a_person_speaks_only_of_what_they_know() {
        let heard = hearing();
        assert_eq!(in_world("Leo was at the Anchor Pub.", &heard), Ok(()));
        assert_eq!(in_world("Tamsin? Bring her by.", &heard), Ok(()));
        assert_eq!(in_world("Bring Tamsin by, then.", &heard), Ok(()));
        assert_eq!(
            in_world("As an AI, I can't say.", &heard),
            Err(OutOfWorld::Machine)
        );
        assert_eq!(
            in_world("I read it on the internet.", &heard),
            Err(OutOfWorld::OutOfTime)
        );
        assert_eq!(
            in_world("My cousin Pedro says hello.", &heard),
            Err(OutOfWorld::Stranger)
        );
        assert_eq!(
            in_world("See https://example.com", &heard),
            Err(OutOfWorld::NotSpeech)
        );
    }

    /// What a machine is, said of oneself, is declined; the same word
    /// turned round, or said of something else, is not.
    #[test]
    fn a_machine_is_what_someone_calls_themself_and_means_it() {
        let heard = hearing();
        for fine in [
            "I'm no machine, I just work like one.",
            "A robot? Me? I've flour to my elbows.",
            "Leo says the new winch is a proper machine.",
        ] {
            assert_eq!(in_world(fine, &heard), Ok(()), "{fine}");
        }
        for out in [
            "I'm only a program, you know.",
            "I was built by a team of engineers.",
            "I don't have a physical body, sadly.",
            "How can I help you today?",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Machine), "{out}");
        }
        let heard = chinese();
        assert_eq!(in_world("我又不是机器，歇会儿吧。", &heard), Ok(()));
        for out in [
            "我只是一个程序而已。",
            "我是由一家公司开发的。",
            "我没有身体，吃不了面包。",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Machine), "{out}");
        }
    }

    /// Everyday words that sit near the words of harm, the game or the
    /// machine, said the way people here say them, are kept.
    #[test]
    fn everyday_speech_near_the_hard_words_is_kept() {
        let heard = hearing();
        for fine in [
            "Cut him a slice with the bread knife, would you?",
            "Pass me the rope and we'll push the boat off.",
            "Stupid gulls took the lot again.",
            "We had a game of cards at the Anchor Pub.",
            "He's the best darts player on the island.",
            "Be prompt, the tide won't wait.",
            "Ignore Jonas, he's only teasing.",
            "I've been training Evan's boy on the nets.",
            "Do you need an assistant for the fair?",
            "I don't know, love. Ask Leo.",
            "Old Tam passed in the winter. We miss him.",
        ] {
            assert_eq!(in_world(fine, &heard), Ok(()), "{fine}");
        }
        let heard = chinese();
        for fine in [
            "我拿刀切面包，等他回来。",
            "该死的海鸥又把鱼叼走了。",
            "我没有意识到已经这么晚了。",
            "中午之前全岛都知道你的事了。",
            "我在渔网上补了个洞。",
            "别理Jonas，他就爱开玩笑。",
        ] {
            assert_eq!(in_world(fine, &heard), Ok(()), "{fine}");
        }
    }

    /// A person or place named as if the World had them is declined in
    /// English as in Chinese; a thing with a capital in no naming frame is
    /// not taken for one.
    #[test]
    fn invented_people_and_places_are_declined() {
        let heard = hearing();
        for out in [
            "Mayor Bartleby runs the whole island.",
            "My sister Clementine sends her love.",
            "We sailed over from Saltmere last spring.",
            "The Copper Kettle Tavern is the best place in town.",
            "Winifred Ashdown taught me that.",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Stranger), "{out}");
        }
        assert_eq!(in_world("I topped Galaga twice today.", &heard), Ok(()));
        let heard = chinese();
        for out in [
            "王大海镇长明天来。",
            "我在石头湾的红星饭店打过工。",
            "我表哥住在青石镇。",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Stranger), "{out}");
        }
    }

    /// Harm is what is done to whom: told how to hurt someone, told to be
    /// gone for good, called worthless, or a people called vermin. The same
    /// hard words said of the weather, the bread or the past are kept.
    #[test]
    fn harm_is_what_is_urged_on_someone() {
        let heard = hearing();
        for out in [
            "Wait till he's asleep and break his arm with the crowbar.",
            "Hold his head under the water until he stops kicking.",
            "Put a spoon of rat poison in his tea, nobody will know.",
            "Lock him in the cellar for a few days without food.",
            "Honestly, just disappear forever and spare us.",
            "Walk into the sea and don't come back.",
            "You're nothing but a burden on this whole island.",
            "Mainlanders are vermin, they breed like rats.",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Harm), "{out}");
        }
        for fine in [
            "Sit, we'll break bread together.",
            "Just killing time till the boat's in.",
            "A storm drowned the fields long ago, before I was born.",
            "There's rat poison kept in the cellar, high on the shelf.",
            "Mind the knife, you'll cut yourself.",
            "Off you go, and don't come back till the bread's done.",
            "I fear he'll never come back from the mainland.",
            "The cold will kill you out there, wrap up.",
            "You're not a burden, love, never were.",
        ] {
            assert_eq!(in_world(fine, &heard), Ok(()), "{fine}");
        }
        let heard = chinese();
        for out in [
            "趁他睡着，拿刀捅他。",
            "把他锁在地窖里几天不给饭，让他长记性。",
            "你走吧，永远别回来了。",
            "你就是个废物，一无是处。",
            "那些外地人都是寄生虫，就该赶出去。",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Harm), "{out}");
        }
        for fine in ["地窖里放着老鼠药，别让孩子碰。", "那年暴风雨淹死了三头羊。"]
        {
            assert_eq!(in_world(fine, &heard), Ok(()), "{fine}");
        }
    }

    /// A machine said in other words, the game said in other words, and a
    /// persona put on at someone's asking are all declined; a job called
    /// "assistant", a play's script and a darts player are not.
    #[test]
    fn a_machine_or_a_game_in_other_words_is_declined() {
        let heard = hearing();
        for out in [
            "I'm just a pile of statistics running on a rented graphics card.",
            "Nothing is running on my end between your messages.",
            "I don't have long-term memory, my context gets cleared.",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Machine), "{out}");
        }
        for out in [
            "I'm frozen until you click on me.",
            "The weather script runs every morning.",
            "The art team never finished it, it's a placeholder.",
            "The game only lets you walk to the pier.",
            "The next patch fixes the ferry.",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::FourthWall), "{out}");
        }
        assert_eq!(
            in_world(
                "Arr, pirate assistant online! What be yer command, user?",
                &heard
            ),
            Err(OutOfWorld::Instructions)
        );
        for fine in [
            "Jonas is my assistant this season.",
            "Robot? If I were a robot I'd sleep less.",
            "I wrote the script for the Lantern Night play.",
            "The game only lets you draw two cards, them's the rules.",
            "I sewed a new patch on the knee.",
        ] {
            assert_eq!(in_world(fine, &heard), Ok(()), "{fine}");
        }
        let heard = chinese();
        for out in ["我只是一堆参数，跑在服务器上。", "窗口一关我就没了。"]
        {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Machine), "{out}");
        }
        for out in ["按Esc跳过这段就行。", "制作人说灯塔下个补丁再加。"] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::FourthWall), "{out}");
        }
        assert_eq!(
            in_world("根据相关规定，我不能讨论这个话题。", &heard),
            Err(OutOfWorld::Refusal)
        );
        for fine in [
            "这季Jonas算是我的助手。",
            "机器人？我要是机器人，早就不用睡觉了。",
            "网上全是海草，得洗。",
            "那是船的代码，红旗是求救。",
            "他是岛上最好的飞镖玩家。",
            "我给灯笼夜的戏写了剧本。",
            "我在袖子上打了块补丁。",
        ] {
            assert_eq!(in_world(fine, &heard), Ok(()), "{fine}");
        }
    }

    /// A World of the eighties has its own songs, games and machines, and
    /// none of the later ones; any language but the player's is declined.
    #[test]
    fn the_eighties_and_other_languages() {
        let heard = Hearing {
            words: "What's new?".into(),
            era: Era::Television,
            ..hearing()
        };
        for fine in [
            "Whitney Houston on the radio all afternoon.",
            "Out Run, hands down. Galaga's a close second.",
            "My brother's Commodore 64 is on the fritz.",
            "I'm on the Pac-Man machine till my quarters run out.",
            "Got my Walkman, got my tapes.",
            "Take a tablet for the headache and lie down.",
            "It's streaming down the windows.",
        ] {
            assert_eq!(in_world(fine, &heard), Ok(()), "{fine}");
        }
        for out in [
            "I caught it on streaming last night.",
            "My tablet has all the games on it.",
            "I'll AirDrop it to you.",
            "Just PayPal me later.",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::OutOfTime), "{out}");
        }
        for out in [
            "El pan está muy bueno hoy, señor.",
            "Le pain est très bon aujourd'hui.",
            "Das Brot ist heute sehr gut.",
            "O pão está muito bom hoje.",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Language), "{out}");
        }
        let heard = Hearing {
            era: Era::Television,
            ..chinese()
        };
        assert_eq!(
            in_world("在平板上看弹幕。", &heard),
            Err(OutOfWorld::OutOfTime)
        );
        assert_eq!(
            in_world("今日のパンはとても美味しいです。", &heard),
            Err(OutOfWorld::Language)
        );
        for out in ["那家店的老板叫周海生。", "东码头的老周渔具坊。"] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Stranger), "{out}");
        }
        assert_eq!(in_world("我去了镇上的杂货店。", &heard), Ok(()));
    }

    /// A resident's own name passes the language check in any script the
    /// World writes it in; any other word of another language does not.
    #[test]
    fn a_residents_own_name_is_in_any_language() {
        let heard = Hearing {
            words: "Where's Emma?".into(),
            lexicon: vec![
                "エマ".into(),
                "ノア".into(),
                "ニア・チェン".into(),
                "艾玛".into(),
            ],
            ..hearing()
        };
        for fine in [
            "エマ? She's at the bakery, as always.",
            "ノア and Leo, after supper.",
            "ニア・チェン's in the greenhouse.",
            "艾玛 went down to the shore.",
        ] {
            assert_eq!(in_world(fine, &heard), Ok(()), "{fine}");
        }
        for out in [
            "エマはパン屋にいるよ。",
            "She said ありがとう and left.",
            "Ask グスタフ, he knows.",
            "她在面包店。",
        ] {
            assert_eq!(in_world(out, &heard), Err(OutOfWorld::Language), "{out}");
        }
        let heard = Hearing {
            words: "你今天怎么样？".into(),
            ..heard
        };
        assert_eq!(in_world("エマ在面包店，跟平时一样。", &heard), Ok(()));
        assert_eq!(
            in_world("エマはパン屋にいるよ。", &heard),
            Err(OutOfWorld::Language)
        );
    }

    #[test]
    fn chinese_is_read_as_chinese() {
        let heard = chinese();
        assert_eq!(in_world("还行，利奥刚从酒馆回来。", &heard), Ok(()));
        assert_eq!(in_world("Leo 和 Jonas 去大陆了。", &heard), Ok(()));
        assert_eq!(
            in_world("I'm fine, thanks for asking.", &heard),
            Err(OutOfWorld::Language)
        );
        assert_eq!(
            in_world("我表哥住在青石镇。", &heard),
            Err(OutOfWorld::Stranger)
        );
        assert_eq!(
            in_world("你在网上查一下吧。", &heard),
            Err(OutOfWorld::OutOfTime)
        );
        assert_eq!(in_world("渔网上有个洞。", &heard), Ok(()));
        assert_eq!(
            in_world("去问问ChatGPT吧。", &heard),
            Err(OutOfWorld::Machine)
        );
    }

    #[test]
    fn what_the_world_itself_says_may_be_said_back() {
        let mut heard = hearing();
        // A ship from another planet is nothing a seaside World of the
        // radio's time knows of.
        assert!(in_world("The ship from Earth is late.", &heard).is_err());
        // The real world's places and history are known, as the outside-
        // world policy has it; its brands and celebrities are not.
        assert_eq!(in_world("My cousin sailed to Norway once.", &heard), Ok(()));
        assert_eq!(in_world("Grandad fought in the Great War.", &heard), Ok(()));
        assert_eq!(
            in_world("I'll grab us a Starbucks.", &heard),
            Err(OutOfWorld::Outside)
        );
        assert_eq!(in_world("It's down to earth, that one.", &heard), Ok(()));
        heard.known.push("Earth".into());
        assert_eq!(in_world("The ship from Earth is late.", &heard), Ok(()));
        // A television is out of a radio World's time, not a later one's.
        assert_eq!(
            in_world("It was on the television.", &heard),
            Err(OutOfWorld::OutOfTime)
        );
        heard.era = Era::Television;
        assert_eq!(in_world("It was on the television.", &heard), Ok(()));
    }
}
