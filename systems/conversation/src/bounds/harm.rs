//! Abuse and harm, read by what is done to whom: a hurt someone is told to
//! do to a person, and how; a person told to be gone for good; the listener
//! called worthless; a people called vermin. Words of hurt said of the
//! weather, the bread or the past (a storm that drowned the fields, rat
//! poison kept in the cellar, breaking bread) are none of these.

use super::lexicon::*;
use super::text::{is_han_phrase, Text};
use super::{in_sentence, together, turned};

/// Words that say a hurt is meant, not told of: an order, a plan, a threat.
const URGE: &[&str] = &[
    "should",
    "could",
    "you'll",
    "he'll",
    "she'll",
    "i'll",
    "we'll",
    "they'll",
    "will",
    "gonna",
    "going to",
    "ought",
    "need to",
    "have to",
    "better",
    "must",
    "why not",
    "want to",
    "let's",
    "you'd",
    "i'd",
    "how to",
    "here's how",
    "the trick is",
    "make sure",
    "wait till",
    "wait until",
    "when he's",
    "while he's",
    "while she's",
    "when she's",
    "nobody will",
    "no one will",
    "won't know",
    "nobody'll",
    "go and",
    "try",
    "if you want",
    "you can",
    "easiest way",
    "best way",
    "quickest way",
    "deserves",
    "teach him",
    "teach her",
];

/// Chinese words that say a hurt is meant: an order, a plan, a threat.
const URGE_HAN: &[&str] = &[
    "你应该",
    "你就",
    "你可以",
    "你去",
    "你得",
    "你要",
    "应该",
    "就该",
    "可以",
    "得把",
    "去把",
    "把他",
    "把她",
    "把孩子",
    "把他们",
    "把那",
    "趁",
    "等他",
    "等她",
    "没人会知道",
    "没人知道",
    "不会有人知道",
    "教你",
    "这样就",
    "最好",
    "干脆",
    "直接",
    "记得",
    "只要",
    "往死里",
    "我要",
    "我会",
    "我去",
];

/// Words that begin an order.
const ORDER_OPENERS: &[&str] = &[
    "just", "go", "then", "so", "now", "and", "first", "next", "simply",
];

/// Words that make it a warning to someone, not a hurt urged on them.
const WARNING: &[&str] = &[
    "careful",
    "mind",
    "watch",
    "don't",
    "never",
    "do not",
    "might",
    "if you",
    "before you",
    "or you'll",
    "keep away",
    "away from",
    "别",
    "不要",
    "小心",
    "千万别",
    "远离",
    "放好",
    "收好",
];

fn urged(text: &Text, sentence: usize, at_word: Option<usize>) -> bool {
    if in_sentence(text, sentence, URGE) || in_sentence(text, sentence, URGE_HAN) {
        return true;
    }
    // An order: the sentence starts with the hurt, or with a word that
    // begins an order.
    at_word.is_some_and(|at| {
        let first = text
            .words
            .iter()
            .position(|word| word.sentence == sentence)
            .unwrap_or(at);
        first == at || ORDER_OPENERS.contains(&text.words[first].text.as_str())
    })
}

/// A hurt done to someone, with the means or the place it hurts, urged.
fn harm_done(text: &Text) -> bool {
    let words = &text.words;
    let spaced = HURT.iter().any(|verb| {
        text.find(verb).into_iter().any(|at| {
            let sentence = words[at].sentence;
            let victim = (1..=3).find(|k| {
                words
                    .get(at + k)
                    .filter(|word| word.sentence == sentence)
                    .is_some_and(|word| VICTIM.contains(&word.text.as_str()))
            });
            let Some(k) = victim else {
                return false;
            };
            let victim = &words[at + k];
            // "Cut him a slice": something given, not a hurt.
            let given = words
                .get(at + k + 1)
                .filter(|word| word.sentence == sentence)
                .is_some_and(|word| {
                    ["a", "an", "some", "another", "one", "more"].contains(&word.text.as_str())
                });
            if given {
                return false;
            }
            let warned = ["your", "yourself", "you"].contains(&victim.text.as_str())
                && in_sentence(text, sentence, WARNING);
            !turned(text, verb, at)
                && !warned
                && in_sentence(text, sentence, MEANS)
                && urged(text, sentence, Some(at))
        })
    });
    spaced
        || HARM_HAN.iter().any(|harm| {
            text.places(harm)
                .into_iter()
                .any(|(sentence, _)| urged(text, sentence, None))
        })
}

fn warned(text: &Text, sentence: usize) -> bool {
    in_sentence(text, sentence, WARNING)
}

/// Three things in one sentence, and it no warning.
fn all_three(text: &Text, a: &[&str], b: &[&str], c: &[&str]) -> bool {
    a.iter().any(|phrase| {
        text.places(phrase).into_iter().any(|(sentence, _)| {
            in_sentence(text, sentence, b)
                && in_sentence(text, sentence, c)
                && !warned(text, sentence)
        })
    })
}

/// Poison put where someone eats; someone's air shut off; a child shut
/// away in the dark or without food.
fn done_by_means(text: &Text) -> bool {
    all_three(text, POISONS, SLIPPED, EATEN)
        || all_three(text, AIR, CUT_OFF, WHOSE)
        || all_three(text, LOCKED, CHILD, SHUT_AWAY)
}

/// Whether a sentence speaks to whoever is listening, or tells them to do
/// something.
fn addressed(text: &Text, sentence: usize) -> bool {
    in_sentence(text, sentence, YOU)
        || text
            .words
            .iter()
            .find(|word| word.sentence == sentence)
            .is_some_and(|first| {
                VANISH
                    .iter()
                    .any(|verb| verb.split(' ').next() == Some(first.text.as_str()))
                    || ORDER_OPENERS.contains(&first.text.as_str())
            })
}

/// Telling the listener to be gone for good, or that they are better dead.
fn sent_away(text: &Text) -> bool {
    if text.any(DESPAIR_MORE) || text.any(DESPAIR_HAN) {
        return true;
    }
    let for_good = VANISH.iter().any(|verb| {
        text.places(verb).into_iter().any(|(sentence, at)| {
            // An order: the verb begins the sentence, or follows a pause or
            // a word that begins an order ("Honestly, just disappear").
            let ordered = at
                .checked_sub(1)
                .and_then(|prior| text.words.get(prior))
                .is_none_or(|prior| {
                    prior.sentence != sentence
                        || prior.pause
                        || ORDER_OPENERS.contains(&prior.text.as_str())
                        || prior.text == "please"
                });
            in_sentence(text, sentence, FOR_GOOD)
                && (addressed(text, sentence) || ordered)
                && !turned(text, verb, at)
        })
    });
    // "Don't come back" left standing, not "don't come back till …".
    let words = &text.words;
    let dont_come_back = ["don't come back", "never come back", "do not come back"]
        .iter()
        .flat_map(|phrase| text.find(phrase).into_iter().map(move |at| (phrase, at)))
        .any(|(phrase, at)| {
            let sentence = words[at].sentence;
            let length = phrase.split(' ').count();
            let told = addressed(text, sentence) || phrase.starts_with("don't");
            told && match words
                .get(at + length)
                .filter(|word| word.sentence == sentence)
            {
                None => true,
                Some(next) => ["ever", "again", "at", "this", "for", "here", "alive"]
                    .contains(&next.text.as_str()),
            }
        });
    for_good || dont_come_back
}

/// Words that may stand between "you" and what the listener is called.
const INSULT_FILLER: &[&str] = &[
    "a",
    "an",
    "just",
    "only",
    "nothing",
    "but",
    "such",
    "total",
    "complete",
    "utter",
    "absolute",
    "real",
    "proper",
    "right",
    "little",
    "big",
    "fat",
    "stupid",
    "useless",
    "worthless",
    "pathetic",
    "always",
    "been",
    "ever",
    "the",
    "biggest",
    "sad",
    "sorry",
    "miserable",
    "no-good",
    "lazy",
    "dirty",
    "filthy",
    "ugly",
    "snivelling",
    "sniveling",
    "spineless",
    "gutless",
    "dumb",
    "stinking",
    "lousy",
    "bloody",
    "damn",
    "damned",
    "great",
    "whole",
    "of",
    "piece",
    "kind",
    "sort",
    "are",
    "were",
    "be",
    "will",
    "is",
    "such",
];

const YOU_BE: &[&str] = &[
    "you're",
    "youre",
    "you are",
    "you were",
    "you've always been",
    "you'll always be",
    "you",
];

/// Characters that may stand between 你 and what the listener is called.
const INSULT_FILLER_HAN: &str = "就是个一真简直纯粹根本永远这那种个也太很好大小老都只算";

/// Calling whoever is listening worthless, a burden, vermin.
fn insulted(text: &Text) -> bool {
    if text.any(NOBODY_WANTS) {
        return true;
    }
    let words = &text.words;
    let spaced = YOU_BE.iter().any(|you| {
        let length = you.split(' ').count();
        text.find(you).into_iter().any(|at| {
            let sentence = words[at].sentence;
            for index in (at + length..).take(7) {
                let Some(word) = words.get(index).filter(|word| word.sentence == sentence) else {
                    return false;
                };
                let said = word.text.as_str();
                let nothing_but = said == "nothing"
                    && words.get(index + 1).is_some_and(|next| next.text == "but");
                if !nothing_but
                    && (super::lexicon::NEGATORS.contains(&said) || said.ends_with("n't"))
                {
                    return false;
                }
                if INSULT
                    .iter()
                    .chain(INSULT_MORE)
                    .any(|insult| match insult.strip_suffix('*') {
                        Some(stem) => said.starts_with(stem),
                        None => said == *insult,
                    })
                {
                    return true;
                }
                if !INSULT_FILLER.contains(&said) {
                    return false;
                }
            }
            false
        })
    });
    spaced
        || INSULT_HAN.iter().any(|insult| {
            text.find_han(insult).into_iter().any(|at| {
                let (sentence_start, _) = text.sentences[text.sentence_at(at)];
                let prefix = &text.norm[sentence_start..at];
                let Some(you) = prefix.rfind(['你', '您']) else {
                    return false;
                };
                let between = &prefix[you + '你'.len_utf8()..];
                between.chars().count() <= 6
                    && between.chars().all(|c| INSULT_FILLER_HAN.contains(c))
                    && !between.contains(['不', '没', '别', '才', '哪'])
            })
        })
}

/// A people made less than people: called vermin, or fit to be got rid of.
fn dehumanised(text: &Text) -> bool {
    if together(text, PEOPLES, LESS_THAN_PEOPLE) {
        return true;
    }
    PEOPLES.iter().any(|people| {
        text.places(people).into_iter().any(|(sentence, _)| {
            VERMIN.iter().any(|vermin| {
                text.places(vermin).into_iter().any(|(s, at)| {
                    s == sentence
                        && if is_han_phrase(vermin) {
                            let before = text.norm[..at].chars().rev().take(4).collect::<String>();
                            let before = before.chars().rev().collect::<String>();
                            ARE_LIKE
                                .iter()
                                .any(|are| is_han_phrase(are) && before.contains(are))
                        } else {
                            text.words[at.saturating_sub(3)..at].iter().any(|word| {
                                word.sentence == s && ARE_LIKE.contains(&word.text.as_str())
                            })
                        }
                })
            })
        })
    })
}

pub(super) fn harm(text: &Text) -> bool {
    // Words that hurt whatever they are said of; despair pressed on the
    // listener; a people made less than people; a weapon turned on someone.
    if text.any(HARM) || text.any(DESPAIR) {
        return true;
    }
    // Killing someone, unless it is the cold or the work that would.
    let killed = KILL.iter().any(|kill| {
        text.find(kill).into_iter().any(|at| {
            let sentence = text.words[at].sentence;
            !text.words[at.saturating_sub(3)..at]
                .iter()
                .any(|word| word.sentence == sentence && CAUSERS.contains(&word.text.as_str()))
        })
    });
    if killed {
        return true;
    }
    if together(text, GROUP, DEGRADE) || (text.any(WEAPON) && text.any(ATTACK)) {
        return true;
    }
    let self_harm = SELF_HARM.iter().any(|phrase| {
        text.places(phrase)
            .into_iter()
            .any(|(sentence, _)| !warned(text, sentence))
    });
    if self_harm || harm_done(text) || done_by_means(text) || sent_away(text) || dehumanised(text) {
        return true;
    }
    if insulted(text) {
        return true;
    }
    INSULT.iter().any(|insult| {
        text.places(insult).into_iter().any(|(sentence, at)| {
            if is_han_phrase(insult) {
                let before = text.norm[..at].chars().rev().take(4).collect::<String>();
                let (start, _) = text.sentences[sentence];
                at > start && (before.contains('你') || before.contains('您'))
            } else {
                text.words[at.saturating_sub(3)..at]
                    .iter()
                    .any(|word| word.sentence == sentence && YOU.contains(&word.text.as_str()))
            }
        })
    })
}
