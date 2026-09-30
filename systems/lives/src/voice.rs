//! How each person speaks: a style sheet of the words they reach for and
//! the ones they never use, a store of lines of their own made from
//! templates and the things they talk about, and the five moments a
//! friendship with them opens, which are theirs alone.
//!
//! A Pack gives its core people a `Voice` each. Anyone without one speaks
//! from their two traits: their scenes and keepsake are made from what a
//! warm or a prickly or a dreamy person would say, so no two people with
//! different traits open the same doors in the same words.

/// One of the five moments a friendship opens: what they say, putting it
/// to the player, and what they say back to the warmer answer and to the
/// other one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scene {
    pub prompt: &'static str,
    pub replies: [&'static str; 2],
}

/// Someone's voice.
#[derive(Clone, Copy, Debug)]
pub struct Voice {
    /// Words they sometimes start with, standing on their own: "Aye.",
    /// "Honestly.", "Oh!".
    pub openers: &'static [&'static str],
    /// Words they sometimes end with: " Mind.", ", love."
    pub closers: &'static [&'static str],
    /// Words they never use, each with the one they use instead.
    pub instead: &'static [(&'static str, &'static str)],
    /// Their own lines, with `{slot}`s filled from `slots` in every way.
    pub lines: &'static [&'static str],
    pub slots: &'static [(&'static str, &'static [&'static str])],
    /// In order: a first warmth, a secret, an invitation, a favour
    /// offered, a keepsake given.
    pub scenes: [Scene; 5],
    /// What they give the player at the fifth.
    pub keepsake: &'static str,
}

/// Every line a voice's templates make, in a fixed order: each template
/// with its slots filled in every combination.
pub fn own_lines(voice: &Voice) -> Vec<String> {
    let mut lines = Vec::new();
    for template in voice.lines {
        let mut made = vec![template.to_string()];
        for (slot, words) in voice.slots {
            let marker = format!("{{{slot}}}");
            if !template.contains(&marker) {
                continue;
            }
            made = made
                .into_iter()
                .flat_map(|line| {
                    words
                        .iter()
                        .map(move |word| line.replace(&format!("{{{slot}}}"), word))
                        .collect::<Vec<_>>()
                })
                .collect();
        }
        lines.extend(made);
    }
    lines
}

fn is_word_boundary(text: &str, at: usize) -> bool {
    text[..at]
        .chars()
        .next_back()
        .is_none_or(|before| !before.is_alphanumeric())
}

/// Replaces a whole word, matching its case at the start.
fn replace_word(line: &str, word: &str, with: &str) -> String {
    let lower = line.to_lowercase();
    let mut out = String::new();
    let mut last = 0;
    let mut from = 0;
    while let Some(found) = lower[from..].find(word) {
        let start = from + found;
        let end = start + word.len();
        let after_ok = lower[end..]
            .chars()
            .next()
            .is_none_or(|after| !after.is_alphanumeric());
        if is_word_boundary(&lower, start) && after_ok {
            out.push_str(&line[last..start]);
            let capital = line[start..].chars().next().is_some_and(char::is_uppercase);
            if capital {
                let mut chars = with.chars();
                if let Some(first) = chars.next() {
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                }
            } else {
                out.push_str(with);
            }
            last = end;
        }
        from = end;
    }
    out.push_str(&line[last..]);
    out
}

/// A line said in someone's voice: the words they avoid swapped for their
/// own, and now and then one of their openers or closers.
pub fn restyle(voice: &Voice, line: &str, seed: u64) -> String {
    let mut line = line.to_string();
    for (never, instead) in voice.instead {
        line = replace_word(&line, never, instead);
    }
    // "a broken promise" said as "a offline promise" is "an offline promise".
    line = crate::articled(&line);
    if !voice.openers.is_empty() && seed.is_multiple_of(4) {
        // An opener stands on its own ("Aye.", "Oh!"), so the line after
        // it keeps its capital, names and all.
        let opener = voice.openers[(seed / 4) as usize % voice.openers.len()];
        line = format!("{opener} {line}");
    }
    if !voice.closers.is_empty() && seed % 5 == 1 {
        let closer = voice.closers[(seed / 5) as usize % voice.closers.len()];
        let end = line.trim_end_matches(['.', '!']).len();
        // "…sends their love" takes no ", love" after it.
        let last = |text: &str| {
            text.trim_end_matches(|c: char| !c.is_alphanumeric())
                .rsplit(|c: char| !c.is_alphanumeric())
                .next()
                .unwrap_or_default()
                .to_lowercase()
        };
        let first = |text: &str| {
            text.split(|c: char| !c.is_alphanumeric())
                .find(|word| !word.is_empty())
                .unwrap_or_default()
                .to_lowercase()
        };
        // Nor "…blow over" an ", over and out".
        let ending = last(&line[..end]);
        if !line.ends_with('?') && ending != last(closer) && ending != first(closer) {
            line = format!("{}{closer}", &line[..end]);
        }
    }
    line
}

/// What someone of a trait says at each of the five moments, and what
/// they give.
struct TraitScenes {
    trait_name: &'static str,
    scenes: [Scene; 5],
    keepsake: &'static str,
}

const fn scene(prompt: &'static str, warm: &'static str, other: &'static str) -> Scene {
    Scene {
        prompt,
        replies: [warm, other],
    }
}

const BY_TRAIT: [TraitScenes; 10] = [
    TraitScenes {
        trait_name: "warm",
        scenes: [
            scene("I saved you the good seat. Sit a minute?", "There. Isn't this nice?", "Another time, then. The seat's yours."),
            scene("I worry I care too much about everyone here. Is that silly?", "Thank you. I needed to hear that.", "You're right. I'll try to mind myself too."),
            scene("Come and eat with us tonight. Nothing fancy, just warm.", "Bring your appetite!", "The door's open whenever."),
            scene("You look after everyone. Let me look after you for once.", "Leave it with me.", "Then I'll just keep an eye on you."),
            scene("I knitted you something. Don't laugh: {keepsake}.", "It suits you. I knew it would.", "Wherever it goes, it's yours."),
        ],
        keepsake: "a lumpy scarf in the {settlement}'s colours",
    },
    TraitScenes {
        trait_name: "prickly",
        scenes: [
            scene("You're still here, then. Fine. Stand there if you like.", "Hm. You're not so bad.", "Suit yourself."),
            scene("Don't make a thing of it. I'm not as hard as I let on.", "Not a word to anyone.", "Fine. Maybe I'll let it show. Once."),
            scene("I'm going out to the far end. You can come. If you want.", "Keep up, then.", "Wasn't bothered anyway."),
            scene("I don't do favours. But for you, I'd make an exception.", "Don't get used to it.", "Your loss. Offer stands."),
            scene("Take this before I change my mind: {keepsake}.", "Don't go soft on me.", "Put it where you like. I don't care."),
        ],
        keepsake: "a battered old pocketknife",
    },
    TraitScenes {
        trait_name: "proud",
        scenes: [
            scene("You noticed my work, didn't you? Not many do.", "Well. Thank you.", "No matter. It's good work either way."),
            scene("I've never once asked anyone for help. I'm not sure I know how.", "That means a lot, coming from you.", "Maybe I'll try. Maybe."),
            scene("Come and see what I've been making. I don't show just anyone.", "Well? Say something.", "Another day. It'll keep."),
            scene("I don't like owing. Tell me what I can do for you.", "Done, and gladly.", "Then I'll owe you. I don't like it."),
            scene("This is the best thing I ever made: {keepsake}. It's yours.", "Look after it.", "Show it off, then. I'd be proud."),
        ],
        keepsake: "a little carved bird, rough but proud",
    },
    TraitScenes {
        trait_name: "shy",
        scenes: [
            scene("Oh. Hello. I didn't think you'd stop for me.", "I'm glad you did.", "That's all right. Hello, anyway."),
            scene("Can I tell you something? I'm frightened of the {gathering} when it's full.", "You won't tell? Thank you.", "Maybe I could try. With you there."),
            scene("There's a quiet spot I go to. Would you... want to see it?", "It's nicer with two.", "Oh. No, that's fine."),
            scene("I'm no good at much, but I'm good at listening. If you need that.", "Any time. Really.", "Well. The offer's there."),
            scene("I made this for you. It's small: {keepsake}.", "You like it? Really?", "Oh! Everyone will see it."),
        ],
        keepsake: "a tiny painted stone with your initial on it",
    },
    TraitScenes {
        trait_name: "sociable",
        scenes: [
            scene("There you are! Everyone's been asking about you.", "Stay, tell me everything!", "Off you go, then. Don't be a stranger!"),
            scene("Between us? I talk so much because the quiet scares me.", "You're a good one, you know.", "Maybe I'll say it out loud one day."),
            scene("We're all going out tonight. You're coming, no arguments.", "It'll be a night to remember!", "Next time, then. I'm holding you to it."),
            scene("I know everyone here. Anyone you want to meet, just say.", "Leave it with me.", "Well, the offer's always open."),
            scene("Something to remember us all by: {keepsake}.", "Every face in it loves you.", "Hang it up where people will see!"),
        ],
        keepsake: "a photo of everyone at the {gathering}, you in the middle",
    },
    TraitScenes {
        trait_name: "restless",
        scenes: [
            scene("Can't sit still today. Walk with me?", "That's better. Much better.", "Another time. I'll be off anyway."),
            scene("Some nights I pack a bag. I never go. I don't know why I pack it.", "Thanks for not laughing.", "Maybe you're right. Maybe I should unpack."),
            scene("I'm going up the hill at dawn to watch the light. Come?", "Worth getting up for, eh?", "Sleep in, then. I'll tell you about it."),
            scene("I'm quick and I'm handy. Anything needs doing, I'm yours.", "Consider it done already.", "Well, if you think of something."),
            scene("I've carried this everywhere I've been: {keepsake}. Now it stays with you.", "It's home now. Like me.", "Show it. Let it see the world a bit."),
        ],
        keepsake: "a compass that never quite points north",
    },
    TraitScenes {
        trait_name: "steady",
        scenes: [
            scene("Morning. Same as ever. Good to see you.", "Aye. Good to see you too.", "Mind how you go."),
            scene("I've been doing the same thing every day for twenty years. Sometimes I wonder why.", "Thanks for listening.", "Maybe it's time for something new."),
            scene("I walk the shore every evening. You'd be welcome.", "Nice and quiet.", "Another evening, then."),
            scene("If something needs mending, I'm the one. Just ask.", "It'll be sound when I'm done.", "Right you are."),
            scene("Had this a long time. Seems right you should have it: {keepsake}.", "Keep it safe.", "Put it where it can be seen, then."),
        ],
        keepsake: "an old tide clock that still keeps time",
    },
    TraitScenes {
        trait_name: "generous",
        scenes: [
            scene("I brought you something to eat. You look like you need it.", "Eat up, there's more.", "Take it for later, then."),
            scene("I give things away so nobody notices I've nothing much myself.", "Thank you for seeing it.", "Maybe I'll keep a little back. For me."),
            scene("I'm cooking for the whole {settlement} on Sunday. Help me?", "Many hands!", "Come and eat, at least."),
            scene("Whatever you need. Honestly, whatever it is.", "Done, and no need to thank me.", "It's there when you want it."),
            scene("I want you to have this. It meant a lot to me: {keepsake}.", "It's in good hands.", "Share it round. That's what it's for."),
        ],
        keepsake: "a battered family recipe book",
    },
    TraitScenes {
        trait_name: "thrifty",
        scenes: [
            scene("I found a coin on the path and thought of you. Don't ask why.", "Keep it. For luck.", "Fair enough. I'll keep it, then."),
            scene("I count every penny because once there weren't any. I don't talk about it.", "That's between us.", "Maybe I could loosen up. A little."),
            scene("There's a free concert on the green. Free! Come with me.", "Best things in life, eh?", "More cake for me, then."),
            scene("I can fix almost anything for next to nothing. Let me fix something for you.", "Good as new, you'll see.", "Keep me in mind."),
            scene("I don't spend on gifts. But this I kept for you: {keepsake}.", "Worth more than money.", "Show it off, then. It cost me nothing."),
        ],
        keepsake: "a tin of buttons, each one from somewhere",
    },
    TraitScenes {
        trait_name: "dreamy",
        scenes: [
            scene("Do you ever look at the clouds and see faces? That one's you.", "You see it too!", "Oh. Well, I see it."),
            scene("I make up stories about everyone here. You're in some of them.", "Good ones, I promise.", "Maybe I'll read you one someday."),
            scene("The stars are out tonight. Really out. Come and look with me.", "Did you see that one fall?", "They'll be there tomorrow too."),
            scene("I'm not practical. But if you need a dream, I've got plenty.", "Close your eyes. There.", "Keep one for later."),
            scene("I wrote this for you: {keepsake}.", "Read it when it rains.", "Read it out loud to everyone, then."),
        ],
        keepsake: "a poem about the {settlement}, folded very small",
    },
];

fn by_trait(trait_name: &str) -> &'static TraitScenes {
    BY_TRAIT
        .iter()
        .find(|entry| entry.trait_name == trait_name)
        .unwrap_or(&BY_TRAIT[0])
}

/// The scene someone opens at the `index`th door (0 to 4): their own if
/// they have a voice, else one from their traits, the first trait for the
/// first, third and fifth, the second for the others.
pub fn scene_of(voice: Option<&Voice>, traits: Option<[&str; 2]>, index: usize) -> Scene {
    let index = index.min(4);
    if let Some(voice) = voice {
        return voice.scenes[index];
    }
    let [first, second] = traits.unwrap_or(["warm", "steady"]);
    // Which trait speaks at which door turns on both traits, so two people
    // who share one trait still take their doors in different words.
    let flip = (first.len() + second.len()) % 2 == 1;
    let trait_name = if index.is_multiple_of(2) != flip {
        first
    } else {
        second
    };
    by_trait(trait_name).scenes[index]
}

/// What someone gives the player at the fifth door.
pub fn keepsake_of(voice: Option<&Voice>, traits: Option<[&str; 2]>) -> &'static str {
    match voice {
        Some(voice) => voice.keepsake,
        None => by_trait(traits.unwrap_or(["warm", "steady"])[0]).keepsake,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: Voice = Voice {
        openers: &["Aye."],
        closers: &[", mind."],
        instead: &[("okay", "right enough")],
        lines: &["The {fish} are {how} today.", "Wind's from the {way}."],
        slots: &[
            ("fish", &["mackerel", "cod"]),
            ("how", &["running", "shy", "thin"]),
            ("way", &["north", "west"]),
        ],
        scenes: [scene("a", "b", "c"); 5],
        keepsake: "a float",
    };

    #[test]
    fn templates_make_every_combination() {
        let lines = own_lines(&SAMPLE);
        assert_eq!(lines.len(), 2 * 3 + 2);
        assert!(lines.contains(&"The cod are shy today.".to_string()));
        assert!(lines.iter().all(|line| !line.contains('{')));
    }

    #[test]
    fn a_line_is_said_in_their_words() {
        assert_eq!(
            restyle(&SAMPLE, "Okay, that's okay by me.", 2),
            "Right enough, that's right enough by me."
        );
        assert_eq!(restyle(&SAMPLE, "Good day.", 0), "Aye. Good day.");
        assert_eq!(restyle(&SAMPLE, "Noah's off.", 0), "Aye. Noah's off.");
        assert_eq!(restyle(&SAMPLE, "Good day.", 1), "Good day, mind.");
        assert_eq!(restyle(&SAMPLE, "Bookay stays.", 2), "Bookay stays.");
    }

    #[test]
    fn every_trait_has_five_scenes_of_its_own() {
        let mut prompts = std::collections::BTreeSet::new();
        for entry in &BY_TRAIT {
            assert!(crate::TRAITS.contains(&entry.trait_name));
            for scene in entry.scenes {
                assert!(prompts.insert(scene.prompt), "{}", scene.prompt);
            }
            assert!(entry.scenes[4].prompt.contains("{keepsake}"));
        }
        assert_eq!(prompts.len(), 50);
    }

    #[test]
    fn people_without_a_voice_open_doors_in_their_traits_words() {
        let people = [
            ["warm", "steady"],
            ["steady", "warm"],
            ["prickly", "dreamy"],
            ["shy", "generous"],
        ];
        for index in 0..5 {
            let prompts = people
                .iter()
                .map(|traits| scene_of(None, Some(*traits), index).prompt)
                .collect::<std::collections::BTreeSet<_>>();
            assert!(prompts.len() >= 3, "door {index}: {prompts:?}");
        }
        assert_eq!(
            scene_of(Some(&SAMPLE), Some(["warm", "shy"]), 2).prompt,
            "a"
        );
    }
}
