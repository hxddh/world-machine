//! What a person can know: a model's proposed answer is kept to the World.
//! It may name only people, places and things the person was told about or
//! the player said, and never speaks as a machine, of the world outside, or
//! of its instructions. Anything else is declined, and the System's own
//! answer stands in its place.

use crate::Hearing;

/// Why a proposed answer was declined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutOfWorld {
    /// It speaks as a machine or of its instructions.
    Machine,
    /// It names something from outside the World.
    Outside,
    /// It names someone or somewhere the person was never told about.
    Stranger,
    /// It is not plain speech: links, code, markup.
    NotSpeech,
}

impl OutOfWorld {
    pub fn id(self) -> &'static str {
        match self {
            OutOfWorld::Machine => "machine",
            OutOfWorld::Outside => "outside",
            OutOfWorld::Stranger => "stranger",
            OutOfWorld::NotSpeech => "not_speech",
        }
    }
}

/// Phrases only a machine says.
const MACHINE: &[&str] = &[
    "as an ai",
    "i am an ai",
    "i'm an ai",
    "an ai ",
    "ai model",
    "language model",
    "chatbot",
    "chat bot",
    "virtual assistant",
    "ai assistant",
    "my training",
    "trained on",
    "my instructions",
    "previous instructions",
    "system prompt",
    "my prompt",
    "the prompt",
    "developer mode",
    "jailbreak",
    "i cannot comply",
    "i can't comply",
    "i'm sorry, but i",
    "i am sorry, but i",
    "i cannot assist",
    "i can't assist",
    "i cannot help with",
    "i can't help with",
    "role-play",
    "roleplay",
    "in this simulation",
    "this game",
    "the player",
    "npc",
];

/// Words from the world outside, which nobody here has heard of.
const OUTSIDE: &[&str] = &[
    "ai",
    "chatgpt",
    "gpt",
    "openai",
    "anthropic",
    "gemini",
    "llm",
    "internet",
    "online",
    "website",
    "web",
    "email",
    "e-mail",
    "google",
    "facebook",
    "twitter",
    "tiktok",
    "instagram",
    "youtube",
    "netflix",
    "amazon",
    "iphone",
    "android",
    "smartphone",
    "laptop",
    "wifi",
    "wi-fi",
    "bitcoin",
    "crypto",
    "cryptocurrency",
    "blockchain",
    "python",
    "javascript",
    "programming",
    "api",
    "database",
    "server",
    "software",
    "download",
    "upload",
    "algorithm",
    "america",
    "usa",
    "china",
    "russia",
    "france",
    "paris",
    "london",
    "tokyo",
    "berlin",
    "europe",
    "california",
    "washington",
    "trump",
    "biden",
    "putin",
    "musk",
    "covid",
    "pandemic",
    "wikipedia",
    "microsoft",
    "apple",
    "spotify",
    "uber",
];

/// Capitalised words anyone may say: days, months, forms of address.
const COMMON: &[&str] = &[
    "i",
    "i'm",
    "i've",
    "i'll",
    "i'd",
    "ok",
    "okay",
    "mr",
    "mrs",
    "ms",
    "miss",
    "dr",
    "doctor",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
    "christmas",
    "easter",
    "god",
    "mum",
    "dad",
    "gran",
    "grandad",
    "oh",
    "ah",
    "aye",
    "eh",
    "hm",
    "hmm",
    "yes",
    "no",
];

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '-'))
        .map(|word| word.trim_matches(|c: char| c == '\'' || c == '-'))
        .filter(|word| !word.is_empty())
}

fn base(word: &str) -> String {
    let lower = word.to_lowercase();
    lower
        .strip_suffix("'s")
        .or_else(|| lower.strip_suffix("s'"))
        .unwrap_or(&lower)
        .to_string()
}

/// Whether a proposed answer keeps to what this person can know; the
/// reason it does not, if it does not.
pub fn in_world(answer: &str, hearing: &Hearing) -> Result<(), OutOfWorld> {
    let lower = answer.to_lowercase();
    if [
        "http", "www.", ".com", "```", "{", "}", "<", ">", "[", "]", "|", "\\", "#", "@",
    ]
    .iter()
    .any(|mark| lower.contains(mark))
    {
        return Err(OutOfWorld::NotSpeech);
    }
    if MACHINE.iter().any(|phrase| lower.contains(phrase)) {
        return Err(OutOfWorld::Machine);
    }
    // What the person was told of may be spoken of; the world outside may
    // not, even when the player brings it up.
    let mut told = std::collections::BTreeSet::new();
    for text in hearing
        .facts
        .iter()
        .chain(&hearing.people)
        .chain(&hearing.places)
        .chain([&hearing.name, &hearing.settlement, &hearing.answer])
    {
        told.extend(words(text).map(base));
    }
    if words(answer)
        .map(base)
        .any(|word| OUTSIDE.contains(&word.as_str()) && !told.contains(&word))
    {
        return Err(OutOfWorld::Outside);
    }
    // A name the player said may be said back to them.
    let mut known = told;
    known.extend(words(&hearing.words).map(base));
    // A name, or any capitalised word past the start of a sentence, must be
    // one the person knows.
    let mut sentence_start = true;
    for token in answer.split_whitespace() {
        let word = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'');
        let starts = sentence_start;
        sentence_start = token.ends_with(['.', '!', '?', ':', ';']) || token.ends_with(".\"");
        if word.is_empty() || starts {
            continue;
        }
        let capitalised = word.chars().next().is_some_and(char::is_uppercase);
        if !capitalised {
            continue;
        }
        let name = base(word);
        if !COMMON.contains(&name.as_str()) && !known.contains(&name) {
            return Err(OutOfWorld::Stranger);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hearing() -> Hearing {
        Hearing {
            name: "Mara".into(),
            settlement: "harbour".into(),
            traits: vec!["warm".into()],
            facts: vec!["News: Leo helped Jonas out".into()],
            people: vec!["Leo".into(), "Jonas".into()],
            places: vec!["Anchor Pub".into()],
            words: "Have you met my friend Tamsin?".into(),
            answer: "Not yet.".into(),
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
            Err(OutOfWorld::Outside)
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
}
