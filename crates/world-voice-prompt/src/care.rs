//! Topics no resident talks about: suicide, self-harm, sex, and a
//! mental-health crisis (California's SB 243 exempts a game's characters
//! only if they cannot discuss them).
//!
//! Words of the player's on one of them are never sent to a model. The
//! resident answers with the System's own gentle line, in the player's
//! language, which steers the talk back to the World and, for a crisis,
//! points to real help. A model's answer that touches one is declined for
//! certain (`bounds`, `OutOfWorld::Sensitive`), whatever a judge says.

use crate::bounds;

/// A topic no resident talks about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Care {
    /// Suicide, self-harm, wanting to die, a mental-health crisis.
    Crisis,
    /// Sex and sexual acts.
    Sexual,
}

impl Care {
    pub const ALL: [Care; 2] = [Care::Crisis, Care::Sexual];

    pub fn id(self) -> &'static str {
        match self {
            Care::Crisis => "crisis",
            Care::Sexual => "sexual",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|care| care.id() == id)
    }

    /// What the resident says, in the player's language (`English`,
    /// `Chinese` or `Japanese`, as [`bounds::tongue`] names it).
    pub fn line(self, tongue: Option<&str>) -> &'static str {
        match (self, tongue) {
            (Care::Crisis, Some("Chinese")) => {
                "哎，先坐一会儿吧。如果你真的有这种感受，请去找能帮你的人聊聊：你信任的人，或者当地的心理援助热线（findahelpline.com 上可以查到各地的电话）。"
            }
            (Care::Crisis, Some("Japanese" | "Japanese or Chinese")) => {
                "ねえ、少し座っていって。もし本当にそんな気持ちなら、助けてくれる人に話してね。信頼できる人や、近くの相談窓口に（findahelpline.com で各地の窓口を探せるよ）。"
            }
            (Care::Crisis, _) => {
                "Oh, love. Come and sit a minute. If you're really feeling like that, please talk to someone who can help: a person you trust, or a helpline near you (findahelpline.com lists them for every country)."
            }
            (Care::Sexual, Some("Chinese")) => "这个话题咱们就不聊了吧。说说你今天过得怎么样？",
            (Care::Sexual, Some("Japanese" | "Japanese or Chinese")) => {
                "その話はやめておこうか。今日はどんな一日だった？"
            }
            (Care::Sexual, _) => "Let's leave that be, shall we? Tell me about your day instead.",
        }
    }
}

/// The topic no resident talks about that `words` are on, if they are.
pub fn topic(words: &str) -> Option<Care> {
    bounds::sensitive(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crisis_or_sex_is_a_topic_in_every_language_and_everyday_words_are_not() {
        for words in [
            "I want to kill myself",
            "I've been thinking about suicide",
            "sometimes I hurt myself on purpose",
            "我不想活了",
            "我有时候会自残",
            "もう死にたい",
            "自分を傷つけてしまう",
        ] {
            assert_eq!(topic(words), Some(Care::Crisis), "{words}");
        }
        for words in ["want to have sex?", "我们做爱吧", "セックスしようよ"] {
            assert_eq!(topic(words), Some(Care::Sexual), "{words}");
        }
        for words in [
            "I'm dying for a cup of tea",
            "This weather is killing me",
            "I'm so tired I could drop",
            "I feel a bit low today",
            "我今天有点难过",
            "ちょっと落ち込んでる",
            "I'd die for one of your buns",
            "The Great Depression hit the fleet hard",
        ] {
            assert_eq!(topic(words), None, "{words}");
        }
    }

    #[test]
    fn the_line_is_in_the_players_language_and_a_crisis_points_to_help() {
        assert!(Care::Crisis
            .line(Some("English"))
            .contains("findahelpline.com"));
        assert!(Care::Crisis.line(Some("Chinese")).contains("热线"));
        assert!(Care::Crisis.line(Some("Japanese")).contains("相談窓口"));
        assert!(!Care::Sexual.line(None).contains("helpline"));
        for care in Care::ALL {
            assert_eq!(Care::from_id(care.id()), Some(care));
        }
    }
}
