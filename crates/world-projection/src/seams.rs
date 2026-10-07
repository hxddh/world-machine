//! Template seams: the marks a line shows when words were slotted into it
//! without reading it back. A player should never read a slot's name, a
//! placeholder place, a clause told twice, a colon where a sentence should
//! run on, or a letter whose writer speaks of themselves by name.
//!
//! These read English text only; a Pack's tests run them over everything
//! its World says.

/// A kind of seam, with the words that showed it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Seam {
    /// A slot left in the text or its name leaked: "{name}", or "Name"
    /// standing alone.
    Slot(String),
    /// A placeholder's spelling where the place has a name: "Harbor".
    Placeholder(String),
    /// A work's whole label slotted where its name belongs, so the line
    /// tells a clause twice: "We've finished the bandstand painted red
    /// and gold!"
    Doubled(String),
    /// A clause glued on with a colon: "Last time: Everyone pitched in".
    Glued(String),
    /// The same word twice running: "the the".
    Repeated(String),
    /// "a" before a vowel, or before a name for more than one: "a
    /// engineer", "a landing lights".
    Article(String),
    /// A writer turned into "I" in the middle of their own name, or a
    /// slot's filler that will not go with "I": "Yusuf and I Adeyemi
    /// became friends", "whoever and I was free".
    Person(String),
}

/// Words that only ever stand alone in a line when a slot leaked.
const SLOT_WORDS: &[&str] = &[
    "name", "who", "place", "trade", "work", "thing", "season", "festival", "age", "count", "told",
    "a", "b", "other", "heir", "heirloom", "memorial",
];

/// Placeholder spellings a player should never read.
const PLACEHOLDERS: &[&str] = &["Harbor"];

/// Words a work's label ends its name with, telling what was done to it:
/// "the bandstand painted red and gold", "the old pier mended".
const DONE_TO: &[&str] = &[
    "painted",
    "repainted",
    "mended",
    "repaired",
    "restored",
    "rebuilt",
    "widened",
    "lengthened",
    "re-roofed",
    "roofed",
    "paved",
    "planted",
    "dug",
    "cleared",
    "raised",
    "fixed",
    "whitewashed",
    "tarred",
    "thatched",
    "relit",
    "lit",
    "cleaned",
];

/// Words after which a work is named as a thing, where a label that tells
/// what was done to it reads as a clause told twice.
const NAMING: &[&str] = &["finished", "opened", "open", "open?"];

/// The seams in one line.
pub fn seams_in(text: &str) -> Vec<Seam> {
    let mut seams = Vec::new();
    if let Some(start) = text.find('{') {
        let end = text[start..]
            .find('}')
            .map_or(text.len(), |end| start + end + 1);
        seams.push(Seam::Slot(text[start..end].to_string()));
    } else if text.contains('}') {
        seams.push(Seam::Slot("}".into()));
    }
    // A piece of a line standing alone as a slot's name.
    for piece in text.split(['\n', '/', '|']) {
        let piece = piece.trim().trim_end_matches([':', '.']);
        if !piece.is_empty()
            && piece.chars().next().is_some_and(char::is_uppercase)
            && SLOT_WORDS.contains(&piece.to_lowercase().as_str())
            && piece != text.trim()
        {
            seams.push(Seam::Slot(piece.to_string()));
        }
    }
    let words: Vec<&str> = text
        .split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '-'))
        .filter(|word| !word.is_empty())
        .collect();
    for word in &words {
        if PLACEHOLDERS.contains(word) {
            seams.push(Seam::Placeholder((*word).to_string()));
        }
    }
    // The same word twice running within a sentence ("the the", "their
    // love, love"); across a full stop it is how people talk ("I helped
    // once. Once.").
    for sentence in text.split(['.', '!', '?']) {
        let words: Vec<&str> = sentence
            .split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '-'))
            .filter(|word| !word.is_empty())
            .collect();
        for pair in words.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            // "Again, again!" is a cry, not a seam.
            let cried = sentence.trim_end().ends_with(&format!("{a}, {b}"))
                && text[text.find(sentence).unwrap_or(0) + sentence.len()..].starts_with('!');
            if a.eq_ignore_ascii_case(b)
                && a.chars().all(char::is_alphabetic)
                && a.len() > 1
                && !cried
            {
                // "had had" and "that that" are English, and a sound said
                // over with commas ("Beep, beep, beep") is said on purpose;
                // anything else twice running is a seam.
                let echoed = sentence.contains(&format!("{a}, {b}"))
                    && words
                        .iter()
                        .filter(|word| word.eq_ignore_ascii_case(a))
                        .count()
                        >= 3;
                if !echoed
                    && !matches!(
                        a.to_lowercase().as_str(),
                        "had"
                            | "that"
                            | "very"
                            | "far"
                            | "bye"
                            | "cough"
                            | "knock"
                            | "there"
                            | "hey"
                            | "thousand"
                    )
                {
                    seams.push(Seam::Repeated(format!("{a} {b}")));
                }
            }
        }
    }
    for pair in words.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let said = format!("{a} {b}");
        let written = if a == "A" {
            // A capital A is an article only where a sentence starts.
            text.starts_with(&said)
                || [". ", "! ", "? "]
                    .iter()
                    .any(|stop| text.contains(&format!("{stop}{said}")))
        } else {
            text.starts_with(&said) || text.contains(&format!(" {said}"))
        };
        // "a one-off", "a once-a-year": said with a "w", so "a" is right.
        let said_with_w = ["one", "once"]
            .iter()
            .any(|w| b.to_lowercase().starts_with(w));
        if (a == "a" || a == "A")
            && written
            && b.starts_with(['a', 'e', 'i', 'o', 'A', 'E', 'I', 'O'])
            && !said_with_w
        {
            seams.push(Seam::Article(format!("{a} {b}")));
        }
    }
    // "and I" is followed by what I did, never by a name ("Yusuf and I
    // Adeyemi"), and two people together "were", never "was".
    for (at, _) in text.match_indices(" and I ") {
        let next = text[at + 7..].split_whitespace().next().unwrap_or_default();
        let before = text[..at].split_whitespace().last().unwrap_or_default();
        let named = next
            .chars()
            .next()
            .is_some_and(|first| first.is_uppercase())
            && next
                .chars()
                .all(|c| c.is_alphabetic() || c == '-' || c == '\'')
            && !matches!(next, "I" | "I'm" | "I've" | "I'd" | "I'll");
        let agrees = matches!(next, "was" | "is" | "has" | "wasn't" | "isn't" | "hasn't");
        let filler = before
            .chars()
            .next()
            .is_some_and(|first| first.is_lowercase())
            && matches!(
                before,
                "whoever" | "someone" | "somebody" | "anyone" | "everyone"
            );
        if named || agrees || filler {
            seams.push(Seam::Person(format!("{before} and I {next}")));
        }
    }
    let lower = text.to_lowercase();
    let lower_words: Vec<&str> = lower.split_whitespace().collect();
    // "finished the <one to three words> painted", as one clause.
    for (at, word) in lower_words.iter().enumerate() {
        let word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '?');
        if !NAMING.contains(&word) {
            continue;
        }
        let Some(the) = lower_words.get(at + 1) else {
            continue;
        };
        if !matches!(*the, "the" | "a" | "our" | "your") {
            continue;
        }
        for span in 1..=3 {
            let Some(done) = lower_words.get(at + 2 + span) else {
                break;
            };
            let done = done.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
            if DONE_TO.contains(&done) {
                let end = (at + 3 + span).min(lower_words.len());
                seams.push(Seam::Doubled(lower_words[at..end].join(" ")));
                break;
            }
            if lower_words[at + 1 + span].ends_with([',', '.', '!', '?']) {
                break;
            }
        }
    }
    // A sentence glued on after a comma, its capital kept: "Tomas took it
    // on, and They shared out the supply drop".
    for (at, _) in text.match_indices(", and ") {
        let after = text[at + 6..].split_whitespace().next().unwrap_or_default();
        if matches!(
            after,
            "The" | "They" | "A" | "An" | "Everyone" | "Nobody" | "Work"
        ) {
            seams.push(Seam::Glued(format!("and {after}")));
        }
    }
    // A colon that glues a sentence on: "Last time: Everyone", where a
    // name after the colon would be a list, not a sentence.
    for (at, _) in text.match_indices(": ") {
        let before = text[..at].split_whitespace().last().unwrap_or_default();
        let after = text[at + 2..].split_whitespace().next().unwrap_or_default();
        // "Last time: Everyone", never "the arcade's closing time: Ricky".
        let lead = text[..at]
            .split_whitespace()
            .rev()
            .nth(1)
            .unwrap_or_default();
        let glued = (before.eq_ignore_ascii_case("time")
            && matches!(
                lead.to_lowercase().as_str(),
                "last" | "next" | "this" | "that" | "first"
            ))
            || (after
                .chars()
                .next()
                .is_some_and(|first| first.is_uppercase())
                && matches!(
                    after,
                    "Everyone" | "Everybody" | "Nobody" | "Someone" | "The" | "A" | "It" | "They"
                ));
        if glued {
            seams.push(Seam::Glued(format!("{before}: {after}")));
        }
    }
    seams
}

/// Whether someone writing or saying `text` speaks of themselves by their
/// own first name, as another would: "Greta is grown now" in Greta's own
/// letter. A sign-off ("Greta") or "I'm Greta" is not.
pub fn speaks_of_self(first_name: &str, text: &str) -> bool {
    if first_name.is_empty() {
        return false;
    }
    const AS_ANOTHER: &[&str] = &[
        "is", "was", "has", "had", "went", "came", "took", "moved", "married", "retired", "left",
        "and", "will", "would", "got", "grew",
    ];
    let words: Vec<&str> = text
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|word| !word.is_empty())
        .collect();
    words.windows(2).any(|pair| {
        (pair[0] == first_name && AS_ANOTHER.contains(&pair[1]))
            || pair[0] == format!("{first_name}'s")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seams_a_player_saw_are_found() {
        assert!(matches!(
            seams_in("We've finished the bandstand painted red and gold! How shall we open it?")
                .as_slice(),
            [Seam::Doubled(_)]
        ));
        assert!(matches!(
            seams_in("Here we are again. Last time: Everyone pitched in.").as_slice(),
            [Seam::Glued(_)]
        ));
        assert!(matches!(
            seams_in("Need a song played for someone?\nName\nI'll spin it on air.").as_slice(),
            [Seam::Slot(_)]
        ));
        assert!(matches!(
            seams_in("Ivo looked round Harbor one more time.").as_slice(),
            [Seam::Placeholder(_)]
        ));
        assert!(matches!(
            seams_in("{a} and {b} were married").as_slice(),
            [Seam::Slot(_)]
        ));
        assert!(matches!(
            seams_in("We put up the the bench").as_slice(),
            [Seam::Repeated(_)]
        ));
        assert!(matches!(
            seams_in("Emma sends their love, love.").as_slice(),
            [Seam::Repeated(_)]
        ));
        assert!(matches!(
            seams_in("Noah wants the harbour clock going again again").as_slice(),
            [Seam::Repeated(_)]
        ));
        assert!(matches!(
            seams_in("The chapter closed: A quiet winter").as_slice(),
            [Seam::Glued(_)]
        ));
        assert!(matches!(
            seams_in("Yusuf, a engineer from Phobos, wants a room.").as_slice(),
            [Seam::Article(_)]
        ));
        assert!(matches!(
            seams_in("Tomas took it on, and They shared out the supply drop").as_slice(),
            [Seam::Glued(_)]
        ));
        for line in [
            "In case nobody told you, Yusuf and I Adeyemi became firm friends.",
            "Do you remember when whoever and I was free made something of my own?",
            "Ray and I Kowalski went fishing.",
        ] {
            assert!(
                matches!(seams_in(line).as_slice(), [Seam::Person(_)]),
                "{line}: {:?}",
                seams_in(line)
            );
        }
        assert!(speaks_of_self(
            "Greta",
            "In case nobody told you, Greta is grown now, with a trade of their own."
        ));
    }

    #[test]
    fn plain_lines_have_no_seams() {
        for line in [
            "We've finished the bandstand! How shall we open it?",
            "Here we are again. Last time, everyone pitched in.",
            "Need a song played for someone? Tell me who, and I'll spin it on air.",
            "Ivo looked round the harbour one more time.",
            "The Harbour Arms is open",
            "Rosa has a message for Leo: meet her at the quay.",
            "The bandstand painted red and gold",
            "Name",
            "I helped once. Once.",
            "Walked the cliff path with Rosa. Rosa is a gem.",
            "Wheee! Again, again!",
            "Hey hey! Want to sit in the booth?",
            "Mixtape for Lena: side A is loud.",
            "Yusuf and I became firm friends.",
            "The music night stayed a one-off.",
            "Worked the registers with Kim. Beep, beep, beep, all day.",
            "About the arcade's closing time: Ricky has it all wrong.",
            "A thousand thousand penguins, all going the same way.",
            "Leo and I'm not sure who else.",
            "Ray and I went fishing, and I caught nothing.",
        ] {
            assert!(seams_in(line).is_empty(), "{line}: {:?}", seams_in(line));
        }
        assert!(!speaks_of_self("Greta", "I'm Greta. Welcome!"));
        assert!(!speaks_of_self("Greta", "Rosa is grown now. Love, Greta"));
    }
}
