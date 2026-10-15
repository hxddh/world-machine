//! How a line speaks of someone without naming them again: he, she or
//! they. A Pack says which each of its people is ([`Cast::pronouns`]),
//! from what it already knows of them; a person may also carry their own
//! ([`PRONOUNS`]), which comes first. Nothing here is guessed from a name.

use super::*;

/// Where someone's own pronouns are kept, when they carry them: "he",
/// "she" or "they". A World from before has none, and is told as its
/// Pack says.
pub const PRONOUNS: &str = "lives.pronouns";

/// The slots for the pronouns of whoever a template calls `{a}`.
pub const A_SLOTS: [&str; 5] = ["a_they", "a_them", "a_their", "a_themselves", "a_were"];
/// The same for `{b}`.
pub const B_SLOTS: [&str; 5] = ["b_they", "b_them", "b_their", "b_themselves", "b_were"];

/// The words a line uses for someone it has already named.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pronouns {
    He,
    She,
    #[default]
    They,
}

impl Pronouns {
    pub const ALL: [Pronouns; 3] = [Pronouns::He, Pronouns::She, Pronouns::They];

    /// As it is kept: "he", "she", "they".
    pub fn id(self) -> &'static str {
        self.they()
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|pronouns| pronouns.id() == id)
    }

    /// "he went", "she went", "they went".
    pub fn they(self) -> &'static str {
        match self {
            Pronouns::He => "he",
            Pronouns::She => "she",
            Pronouns::They => "they",
        }
    }

    /// "ask him", "ask her", "ask them".
    pub fn them(self) -> &'static str {
        match self {
            Pronouns::He => "him",
            Pronouns::She => "her",
            Pronouns::They => "them",
        }
    }

    /// "his own", "her own", "their own".
    pub fn their(self) -> &'static str {
        match self {
            Pronouns::He => "his",
            Pronouns::She => "her",
            Pronouns::They => "their",
        }
    }

    /// "kept to himself", "herself", "themselves".
    pub fn themselves(self) -> &'static str {
        match self {
            Pronouns::He => "himself",
            Pronouns::She => "herself",
            Pronouns::They => "themselves",
        }
    }

    /// "he was", "she was", "they were".
    pub fn were(self) -> &'static str {
        match self {
            Pronouns::He | Pronouns::She => "was",
            Pronouns::They => "were",
        }
    }

    /// Each word, in the slots `names` gives them: [`A_SLOTS`] or
    /// [`B_SLOTS`], so a template says `{a_them}` where it would say "them"
    /// of `{a}`.
    pub fn slots(self, names: &[&'static str; 5]) -> [(&'static str, String); 5] {
        let words = [
            self.they(),
            self.them(),
            self.their(),
            self.themselves(),
            self.were(),
        ];
        std::array::from_fn(|at| (names[at], words[at].to_string()))
    }

    /// `line`, told of its writer, with the writer's own pronouns read as
    /// "I", "me" and "my": "Greta kept to herself" in Greta's letter, once
    /// her name is "I", is "I kept to myself".
    pub(crate) fn as_mine(self, line: &str) -> String {
        let mut line = line.to_string();
        for (from, to) in [
            (format!(" {} own", self.their()), " my own".to_string()),
            (format!(" {} time", self.their()), " my time".to_string()),
            (
                format!(" {} distance", self.their()),
                " my distance".to_string(),
            ),
            (format!(" of {} ", self.their()), " of my ".to_string()),
            (format!(" {}", self.themselves()), " myself".to_string()),
            (format!(" {}'d ", self.they()), " I'd ".to_string()),
            (
                format!(" {} {} ", self.they(), self.were()),
                " I was ".to_string(),
            ),
            (format!(" told {} ", self.them()), " told me ".to_string()),
        ] {
            line = line.replace(&from, &to);
        }
        line
    }
}

/// Someone's pronouns: their own, if they carry them, else as the Pack
/// says.
pub fn pronouns(state: &WorldState, cast: &Cast, person: EntityId) -> Pronouns {
    text(state, person, PRONOUNS)
        .and_then(Pronouns::from_id)
        .unwrap_or_else(|| (cast.pronouns)(state, person))
}

/// "They" for everyone: for [`Cast::pronouns`] in a Pack that says no
/// more of its people.
pub fn they_all(_: &WorldState, _: EntityId) -> Pronouns {
    Pronouns::They
}

#[cfg(test)]
mod tests {
    use super::Pronouns;

    #[test]
    fn each_set_has_its_own_words_and_reads_back_from_how_it_is_kept() {
        for pronouns in Pronouns::ALL {
            assert_eq!(Pronouns::from_id(pronouns.id()), Some(pronouns));
        }
        assert_eq!(Pronouns::from_id("it"), None);
        let words = |p: Pronouns| [p.they(), p.them(), p.their(), p.themselves(), p.were()];
        assert_eq!(words(Pronouns::He), ["he", "him", "his", "himself", "was"]);
        assert_eq!(
            words(Pronouns::She),
            ["she", "her", "her", "herself", "was"]
        );
        assert_eq!(
            words(Pronouns::They),
            ["they", "them", "their", "themselves", "were"]
        );
    }

    #[test]
    fn a_writer_reads_their_own_pronouns_as_i() {
        assert_eq!(
            Pronouns::She.as_mine("I kept to herself"),
            "I kept to myself"
        );
        assert_eq!(
            Pronouns::He.as_mine("I took his time, and Leo and I started walking out together"),
            "I took my time, and Leo and I started walking out together"
        );
        assert_eq!(
            Pronouns::They.as_mine("I confided they were sweet on Leo"),
            "I confided I was sweet on Leo"
        );
        assert_eq!(
            Pronouns::He.as_mine("I told you a secret, and you told him to share it"),
            "I told you a secret, and you told me to share it"
        );
        assert_eq!(
            Pronouns::She.as_mine("I told you something she'd never told anyone"),
            "I told you something I'd never told anyone"
        );
    }
}
