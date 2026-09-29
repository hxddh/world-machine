//! The words of the answers the player is offered, for telling later what
//! they chose: "because you said “Make peace”".

use crate::{script, Kind};

/// The words of `answer` to a situation of `kind` ("feud", "sweet"), as the
/// player was offered them, with its `{a}`, `{b}` and `{friend}` still to
/// fill; `None` for an answer this System never offers (a lapse).
pub fn answer_label(kind: &str, answer: &str) -> Option<&'static str> {
    let kind = Kind::from_id(kind)?;
    script(kind)
        .answers
        .iter()
        .find(|(id, _)| *id == answer)
        .map(|(_, label)| *label)
}

#[cfg(test)]
mod tests {
    use super::answer_label;

    #[test]
    fn an_answer_is_told_in_the_words_it_was_offered_in() {
        assert_eq!(answer_label("feud", "mend"), Some("Make peace"));
        assert_eq!(answer_label("party", "party"), Some("Throw a party"));
        assert_eq!(answer_label("feud", "lapse"), None);
        assert_eq!(answer_label("nothing", "mend"), None);
    }
}
