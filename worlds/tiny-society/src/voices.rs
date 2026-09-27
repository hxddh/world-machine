//! How the harbour's people speak. Each core resident has a style sheet
//! (the words they reach for and the ones they never use), a store of
//! lines of their own made from templates and what they talk about, and
//! five moments a friendship with them opens, which are theirs alone.

use lives::{Scene, Voice};
use world_core::EntityId;

use crate::{EMMA, EVAN, JONAS, LEO, MARA, MIA, NOAH, SOFIA};

const fn scene(prompt: &'static str, warm: &'static str, other: &'static str) -> Scene {
    Scene {
        prompt,
        replies: [warm, other],
    }
}

/// Jonas: a fisherman of few words, proud of his boat and his catch.
pub(crate) const JONAS_VOICE: Voice = Voice {
    openers: &["Aye.", "Well.", "Hm."],
    closers: &[". That's all.", ", mind.", ". Sea's the sea."],
    instead: &[("yes", "aye"), ("very", "right"), ("great", "grand")],
    lines: &[
        "{fish} running {how} off the {spot}.",
        "Went out at {hour}. {sky}.",
        "Net's {net}. {fix}",
        "{fish} at the {spot}. {verdict}",
    ],
    slots: &[
        ("fish", &["Mackerel", "Cod", "Pollock", "Herring", "Crab"]),
        ("how", &["thick", "thin", "shy", "easy"]),
        ("spot", &["point", "north buoy", "reef", "old wreck"]),
        ("hour", &["four", "first light", "half five"]),
        (
            "sky",
            &[
                "Sky like pewter",
                "Not a breath of wind",
                "Swell from the west",
                "Fog till the point",
                "Gulls everywhere",
            ],
        ),
        (
            "net",
            &["torn again", "heavy", "near worn through", "tangled"],
        ),
        (
            "fix",
            &[
                "I'll mend it tonight.",
                "Evan can have a look.",
                "It'll hold. Just.",
                "Seen worse.",
                "Father's net did the same.",
            ],
        ),
        (
            "verdict",
            &[
                "Not bad.",
                "Could be worse.",
                "Enough for the market.",
                "Sea's been kind.",
                "Won't pay the fees.",
                "Mara'll want some.",
            ],
        ),
    ],
    scenes: [
        scene(
            "You're up early. Sit on the crate, watch the boats go out.",
            "Quiet, isn't it. Best time.",
            "Aye. Another morning.",
        ),
        scene(
            "Never told anyone. I'm scared of deep water. Fisherman, scared of the sea.",
            "Keep that under your hat.",
            "Maybe I'll tell the lad one day.",
        ),
        scene(
            "Come out on the Sea Finch with me. Just the once. You'll see.",
            "Steady now. There. That's the sea.",
            "Boat'll be there. Always is.",
        ),
        scene(
            "You've been good to me when it was rough. Anything I can do, you say.",
            "Done. My word on it.",
            "Then it's owed. I don't forget.",
        ),
        scene(
            "My father's. Now yours: {keepsake}.",
            "Keep it by the window. Facing the sea.",
            "Let them see it. He'd like that.",
        ),
    ],
    keepsake: "a brass compass off the Sea Finch",
};

/// Mara: the baker, warm and proud of her bread, calls everyone love.
pub(crate) const MARA_VOICE: Voice = Voice {
    openers: &["Oh!", "Now then.", "Well, love."],
    closers: &[", love.", ", pet.", ". Mark my words."],
    instead: &[
        ("okay", "alright"),
        ("delicious", "lovely"),
        ("tasty", "lovely"),
    ],
    lines: &[
        "{bread} came out {how} this morning.",
        "{bread} had me up at {hour}. {feel}",
        "They want {treat} again. {answer}",
        "{bread} for {whom}. {feel}",
        "{flour} {oven}",
    ],
    slots: &[
        (
            "bread",
            &[
                "The sourdough",
                "The rye",
                "The cottage loaves",
                "The seeded batch",
                "The rolls",
            ],
        ),
        (
            "how",
            &[
                "a treat",
                "a bit flat",
                "golden",
                "just right",
                "better than yesterday",
            ],
        ),
        ("hour", &["three", "half three", "four"]),
        (
            "feel",
            &[
                "My back knows it.",
                "Wouldn't change it.",
                "Kettle first, mind.",
                "Quietest hour there is.",
                "Radio on, flour everywhere.",
            ],
        ),
        (
            "whom",
            &["the school", "the Anchor", "Noah's meeting", "the ferry"],
        ),
        (
            "treat",
            &[
                "the saffron buns",
                "my gran's lardy cake",
                "the lemon tart",
                "cinnamon knots",
            ],
        ),
        (
            "answer",
            &[
                "Saturday, I told them.",
                "If there's butter.",
                "Maybe for the fête.",
                "They'll wait.",
            ],
        ),
        (
            "flour",
            &[
                "Flour's gone up again.",
                "Last sack of the good flour.",
                "Mill sent the wrong flour.",
                "Proper flour at last.",
            ],
        ),
        (
            "oven",
            &[
                "The oven's sulking.",
                "Oven's hot as the sun.",
                "Old oven, old tricks.",
                "The oven and I have an understanding.",
                "Evan swears he'll fix that oven door.",
            ],
        ),
    ],
    scenes: [
        scene(
            "There's a warm roll with your name on it. Come round the back.",
            "There. Nothing like it, is there?",
            "I'll keep it warm for you, love.",
        ),
        scene(
            "Can I tell you? I nearly sold the bakery last winter. Nobody knows.",
            "Thank you, love. That's a weight off.",
            "Maybe I should tell them. Maybe.",
        ),
        scene(
            "Come and bake with me at dawn. I'll teach you the rye.",
            "Look at you! Flour to the elbows.",
            "Another dawn, then. There's always another.",
        ),
        scene(
            "You've been a proper friend. What can I do for you, love?",
            "Consider it done.",
            "Then there's a loaf for you every week. No arguing.",
        ),
        scene(
            "This was my gran's. I want you to have it: {keepsake}.",
            "She'd have liked you.",
            "Hang it where people can see. She'd have loved that.",
        ),
    ],
    keepsake: "a wooden bread paddle, older than the bakery",
};

/// Leo: runs the Anchor, knows everyone, never short of a word.
pub(crate) const LEO_VOICE: Voice = Voice {
    openers: &["Ha!", "Listen.", "Here's one."],
    closers: &[", mate!", ", eh?", ". Drinks on me."],
    instead: &[("hello", "evening"), ("good", "cracking")],
    lines: &[
        "{who} was in {when}, {doing}.",
        "Quiz night {quiz}. {team}",
        "{barrel} {cellar}",
        "{song} {crowd}",
        "{who} owes me for {owed}. {shrug}",
    ],
    slots: &[
        (
            "who",
            &[
                "Old Tam",
                "Half the harbour",
                "The ferry lot",
                "Noah",
                "Jonas",
            ],
        ),
        ("when", &["last night", "at opening", "before the rain"]),
        (
            "doing",
            &[
                "telling whale stories",
                "singing",
                "arguing about the fees",
                "buying rounds",
            ],
        ),
        (
            "quiz",
            &["on Thursday", "went long", "got heated", "was a riot"],
        ),
        (
            "team",
            &[
                "Emma's lot won again.",
                "The fishermen came last. Again.",
                "Mia wrote a question. Nobody got it.",
                "Tie-break on sea shanties!",
            ],
        ),
        (
            "barrel",
            &[
                "New barrel on.",
                "Changed the barrel.",
                "Barrel's nearly dry.",
                "Ale's come in.",
            ],
        ),
        (
            "cellar",
            &[
                "Cellar's cold as a church.",
                "Mind the cellar step.",
                "Cellar smells of the sea.",
                "Found a bottle from '72 down there.",
                "Rats in the cellar again.",
            ],
        ),
        ("owed", &["a pint", "three rounds", "a pie", "last Tuesday"]),
        (
            "shrug",
            &[
                "He'll pay.",
                "Always does.",
                "It's on the slate.",
                "Doesn't matter.",
            ],
        ),
        (
            "song",
            &[
                "Somebody sang till two.",
                "Fiddle came out.",
                "Piano's out of tune.",
                "Old songs tonight.",
            ],
        ),
        (
            "crowd",
            &[
                "Whole place joined in.",
                "Even Noah tapped his foot.",
                "Full house.",
                "Never seen it so busy.",
                "Went on till the lamps.",
            ],
        ),
    ],
    scenes: [
        scene(
            "You! Pull up a stool. First one's on the house.",
            "There. Now you're a regular.",
            "Door's always open, mate.",
        ),
        scene(
            "Truth? I'm lonely when the lamps go off. All those people, then nobody.",
            "You're a good one. Don't tell the regulars.",
            "Maybe I'll keep it open later, then.",
        ),
        scene(
            "Lock-in tonight, just friends. You're coming.",
            "Now it's a party!",
            "Next time. I'm holding you to it.",
        ),
        scene(
            "You've done right by this harbour. Name it, and it's yours.",
            "Consider it sorted.",
            "Tab's clear for a month, then. No arguing.",
        ),
        scene(
            "Been behind the bar since I opened. Now it's yours: {keepsake}.",
            "Hang it by your door. For luck.",
            "Put it up where people will see. Tell them I said so.",
        ),
    ],
    keepsake: "the old brass bell from the Anchor's bar",
};

/// Emma: the teacher, clear and a little restless, fond of a question.
pub(crate) const EMMA_VOICE: Voice = Voice {
    openers: &["Right.", "Honestly.", "Here's a thing."],
    closers: &[", I think.", ". Isn't it?", ". Class dismissed."],
    instead: &[
        ("kids", "children"),
        ("gonna", "going to"),
        ("stuff", "things"),
    ],
    lines: &[
        "We did {topic} today. {reaction}",
        "{pupil} {did}",
        "Marking {pile} {feel}",
        "The school {need}. {plan}",
        "{pupil} asked about {topic}. {feel}",
    ],
    slots: &[
        (
            "topic",
            &[
                "tides",
                "fractions",
                "the lighthouse",
                "poems",
                "seabirds",
                "maps",
            ],
        ),
        (
            "reaction",
            &[
                "They loved it.",
                "Half of them slept.",
                "Such questions!",
                "Mia knew it all already.",
            ],
        ),
        (
            "pupil",
            &["Mia", "The Clark twins", "Little Ada", "The new boy"],
        ),
        (
            "did",
            &[
                "read aloud for the first time.",
                "asked why the sea is salty.",
                "drew a whale on the register.",
                "brought a crab to class.",
                "finished a whole book.",
            ],
        ),
        (
            "pile",
            &[
                "a tall pile",
                "forty essays",
                "the spelling tests",
                "maths homework",
            ],
        ),
        (
            "feel",
            &[
                "Tea helps.",
                "Some of it's wonderful.",
                "Red pen's nearly dry.",
                "I'll be up till ten.",
                "Worth every minute.",
            ],
        ),
        (
            "need",
            &[
                "needs paint",
                "needs books",
                "roof leaks",
                "needs a new stove",
            ],
        ),
        (
            "plan",
            &[
                "I'll ask the council.",
                "We'll hold a sale.",
                "Evan's promised to look.",
                "The children made posters.",
            ],
        ),
    ],
    scenes: [
        scene(
            "Would you listen to the children read? Just ten minutes.",
            "They'll talk about you all week.",
            "Another day. They'll still be reading.",
        ),
        scene(
            "Between us: some days I want to leave and teach somewhere bigger.",
            "Thank you. I feel lighter.",
            "Maybe I'll tell them. Maybe I'll stay.",
        ),
        scene(
            "Nature walk on the cliffs Saturday. We need one more grown-up.",
            "The puffins were out for you!",
            "Next term, then. We'll save you a clipboard.",
        ),
        scene(
            "You've taught me a thing or two. Anything you need, ask.",
            "Top marks. Consider it done.",
            "Then I owe you a lesson. Any subject.",
        ),
        scene(
            "The children made this for you. I added the ribbon: {keepsake}.",
            "Every one of them signed it.",
            "Pin it up. They'll be so proud.",
        ),
    ],
    keepsake: "a card the whole class drew for you",
};

/// Mia: at school, full of wonder and news, speaks in exclamations.
pub(crate) const MIA_VOICE: Voice = Voice {
    openers: &["Oh!", "Guess what!", "Okay so."],
    closers: &["!", ", I swear.", ". Cross my heart."],
    instead: &[("very", "super"), ("interesting", "amazing")],
    lines: &[
        "I saw {creature} {where}!",
        "I'm drawing {thing}. {progress}",
        "{homework} {excuse}",
        "When I grow up I'll be {dream}. {why}",
        "Me and {pal} found {creature} {where}!",
    ],
    slots: &[
        (
            "creature",
            &[
                "a seal",
                "a puffin",
                "a jellyfish",
                "a crab with one claw",
                "two dolphins",
            ],
        ),
        (
            "where",
            &[
                "by the slipway",
                "off the point",
                "under the pier",
                "in a rock pool",
            ],
        ),
        (
            "thing",
            &[
                "the lighthouse",
                "Leo's cat",
                "the whole harbour",
                "a mermaid",
                "a storm",
            ],
        ),
        (
            "progress",
            &[
                "It's nearly done.",
                "It's wobbly.",
                "Emma says it's good.",
                "I need more blue.",
            ],
        ),
        (
            "homework",
            &[
                "Homework's boring.",
                "I did my homework.",
                "Forgot my homework.",
                "Homework's about tides.",
            ],
        ),
        (
            "excuse",
            &[
                "A gull ate it. Nearly.",
                "Tides are cool though.",
                "I'll do it after tea.",
                "Emma won't mind.",
                "I did extra!",
            ],
        ),
        ("pal", &["Ada", "Emma", "Leo's cat", "the twins"]),
        (
            "dream",
            &[
                "a sailor",
                "a painter",
                "a vet",
                "the mayor",
                "an astronaut",
            ],
        ),
        (
            "why",
            &[
                "Obviously.",
                "Noah says I'd be good.",
                "I've decided.",
                "Or maybe a baker.",
                "Don't laugh!",
            ],
        ),
    ],
    scenes: [
        scene(
            "Want to see my rock pool? There's a crab called Gerald.",
            "Gerald likes you! He waved!",
            "Gerald says bye then.",
        ),
        scene(
            "Secret: I'm scared of the dark. Even though I'm nearly big.",
            "You won't tell? Pinky promise.",
            "Maybe I'll tell Emma. Maybe.",
        ),
        scene(
            "There's seals at the point! Come quick, come quick!",
            "Did you SEE that one?!",
            "Aww. I'll tell you all about them.",
        ),
        scene(
            "I'm really good at finding things. I'll find anything you lose!",
            "I'll start looking right now!",
            "Okay but the offer stays forever.",
        ),
        scene(
            "I made this for you. It took ages: {keepsake}.",
            "You can keep it forever!",
            "Put it where EVERYONE can see!",
        ),
    ],
    keepsake: "a crayon drawing of the two of you on the pier",
};

/// Noah: the mayor, careful with words and money, a stickler.
pub(crate) const NOAH_VOICE: Voice = Voice {
    openers: &["Frankly.", "For the record.", "Well now."],
    closers: &[". As I said.", ", in principle.", ". Noted."],
    instead: &[("problem", "matter"), ("money", "funds"), ("chat", "word")],
    lines: &[
        "The council {council}. {view}",
        "{paper} {state}",
        "The {repair} {cost}.",
        "{rule} {because}",
        "The {repair} {cost}. {view}",
        "The {repair}: {because}",
    ],
    slots: &[
        (
            "council",
            &[
                "met till nine",
                "voted again",
                "adjourned early",
                "argued over the slipway",
            ],
        ),
        (
            "view",
            &[
                "Progress, of a kind.",
                "Nothing decided.",
                "I stand by it.",
                "Minutes to follow.",
            ],
        ),
        (
            "paper",
            &[
                "The accounts",
                "The ferry letters",
                "The harbour survey",
                "The grant forms",
            ],
        ),
        (
            "state",
            &[
                "are in order.",
                "need signing.",
                "are overdue.",
                "came back stamped.",
                "are on my desk.",
            ],
        ),
        (
            "repair",
            &[
                "slipway",
                "sea wall",
                "lamp post",
                "school roof",
                "quay steps",
            ],
        ),
        (
            "cost",
            &[
                "needs doing before winter",
                "will cost more than we have",
                "is on the list",
                "can wait a month",
            ],
        ),
        (
            "rule",
            &[
                "Bins out on Tuesdays.",
                "No mooring past the buoy.",
                "Dogs on leads at the market.",
                "Quiet after eleven.",
            ],
        ),
        (
            "because",
            &[
                "Rules are rules.",
                "Somebody has to say it.",
                "It's for everyone's good.",
                "I did write to everyone.",
            ],
        ),
    ],
    scenes: [
        scene(
            "A word, if you have a moment. Walk with me to the office?",
            "Productive. Thank you.",
            "Some other time, then. Noted.",
        ),
        scene(
            "Off the record: I'm not sure I'm any good at this job.",
            "That stays between us, I trust.",
            "Perhaps I'll say it at the next meeting.",
        ),
        scene(
            "The council dinner is on Friday. I'd like you there. As my guest.",
            "You were the best company at the table.",
            "Another year, then. The invitation stands.",
        ),
        scene(
            "You've done more for this harbour than the council. What can I do?",
            "Consider the paperwork already done.",
            "Then I owe you, and I keep accounts.",
        ),
        scene(
            "I'd like you to have this: {keepsake}. Officially.",
            "Keep it safe. It's the only one.",
            "Display it. I'll write a notice.",
        ),
    ],
    keepsake: "the old key to the harbour, on a ribbon",
};

/// Evan: the carpenter, restless, quick to help, fixes everything.
pub(crate) const EVAN_VOICE: Voice = Voice {
    openers: &["Tell you what.", "Look.", "Right then."],
    closers: &[", easy.", ". Sorted.", ". Measure twice."],
    instead: &[
        ("fix", "sort"),
        ("broken", "knackered"),
        ("wonderful", "sound"),
    ],
    lines: &[
        "{job} {verdict}",
        "Got {wood} in. {plan}",
        "{tool} {tale}",
        "{wish} {sigh}",
        "{job} {verdict} {sigh}",
        "{tool} {tale} {sigh}",
    ],
    slots: &[
        (
            "job",
            &[
                "Mara's oven door",
                "The school gate",
                "Leo's cellar step",
                "The quay bench",
                "Noah's filing cabinet",
            ],
        ),
        (
            "verdict",
            &[
                "took all morning.",
                "is sorted.",
                "needs another go.",
                "was a nightmare.",
            ],
        ),
        (
            "wood",
            &[
                "some oak",
                "driftwood",
                "a load of pine",
                "old ship's timber",
            ],
        ),
        (
            "plan",
            &[
                "Might make a table.",
                "Shelves, I reckon.",
                "Something for the school.",
                "No idea yet!",
            ],
        ),
        (
            "tool",
            &["My plane", "The good saw", "Dad's chisel", "My level"],
        ),
        (
            "tale",
            &[
                "went missing again.",
                "turned up in the pub.",
                "needs sharpening.",
                "has outlived three owners.",
                "is older than me.",
            ],
        ),
        (
            "wish",
            &[
                "Could do with a workshop.",
                "Want to build a boat one day.",
                "Thinking of the mainland work.",
                "Need a new roof on the shed.",
            ],
        ),
        (
            "sigh",
            &[
                "One day.",
                "Right after this job.",
                "If the rain lets up.",
                "Don't tell Jonas.",
            ],
        ),
    ],
    scenes: [
        scene(
            "Hold this end, would you? Two pairs of hands and all that.",
            "There. Square as you like.",
            "No bother. I'll wedge it.",
        ),
        scene(
            "I keep saying I'll leave for the mainland work. I never go. Don't know why.",
            "Thanks for not laughing.",
            "Maybe I'll go. Maybe.",
        ),
        scene(
            "Come see the boat I'm building in the shed. Nobody's seen it.",
            "She'll float. One day.",
            "She'll keep. So will I.",
        ),
        scene(
            "Anything wants making or mending, it's yours. No charge.",
            "Sorted before you know it.",
            "Offer stands. Just shout.",
        ),
        scene(
            "Made this for you, from the Sea Finch's old timber: {keepsake}.",
            "Oiled it twice. It'll last.",
            "Put it out. Let them see some proper work.",
        ),
    ],
    keepsake: "a little carved box with a sliding lid",
};

/// Sofia: at the shop, sharp with prices, proud, careful with a penny.
pub(crate) const SOFIA_VOICE: Voice = Voice {
    openers: &["Mind you.", "Honestly.", "Between us."],
    closers: &[". Not cheap.", ", no less.", ". I checked."],
    instead: &[("cheap", "a bargain"), ("expensive", "daylight robbery")],
    lines: &[
        "{item} went up {how}.",
        "The delivery {delivery}. {so}",
        "{customer} {bought}",
        "Stocktake {stock}. {found}",
        "{item} went up {how}. {so}",
    ],
    slots: &[
        (
            "item",
            &["Butter", "Tea", "Candles", "Soap", "Rope", "Stamps"],
        ),
        (
            "how",
            &["again", "by a penny", "twice this month", "for no reason"],
        ),
        (
            "delivery",
            &["was late", "came wet", "was short", "came early for once"],
        ),
        (
            "so",
            &[
                "I wrote to them.",
                "Not good enough.",
                "I counted every tin.",
                "Typical ferry.",
                "Small mercies.",
            ],
        ),
        ("customer", &["Noah", "Old Tam", "Leo", "The ferry lot"]),
        (
            "bought",
            &[
                "wanted credit. Again.",
                "bought every candle.",
                "haggled over string.",
                "paid in coins. All of them.",
                "asked for something we've never sold.",
            ],
        ),
        (
            "stock",
            &["tonight", "on Sunday", "took hours", "came out right"],
        ),
        (
            "found",
            &[
                "Two tins missing.",
                "Found a box from last year.",
                "Every penny accounted for.",
                "Mice in the oats.",
            ],
        ),
    ],
    scenes: [
        scene(
            "I've put aside the good tea for you. Don't tell anyone.",
            "Worth every penny. Which it didn't cost.",
            "It'll keep. Tea's patient.",
        ),
        scene(
            "I save every penny because once, there weren't any. I don't say that.",
            "That's between us, then.",
            "Maybe I'll buy something silly. Once.",
        ),
        scene(
            "There's a free concert on the green. Free. Come with me.",
            "Best things in life.",
            "More room on the bench for me.",
        ),
        scene(
            "I'll get you anything at cost. At cost! Nobody gets that.",
            "Write me a list.",
            "The offer's in the ledger.",
        ),
        scene(
            "I don't give gifts. But this I kept for you: {keepsake}.",
            "Worth more than money.",
            "Show it off, then. It cost me nothing.",
        ),
    ],
    keepsake: "a tin of buttons, each from somewhere she's been",
};

/// The voice of one of the harbour's core people.
pub(crate) fn voice(person: EntityId) -> Option<&'static Voice> {
    Some(match person {
        JONAS => &JONAS_VOICE,
        MARA => &MARA_VOICE,
        LEO => &LEO_VOICE,
        EMMA => &EMMA_VOICE,
        MIA => &MIA_VOICE,
        NOAH => &NOAH_VOICE,
        EVAN => &EVAN_VOICE,
        SOFIA => &SOFIA_VOICE,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const CORE: [EntityId; 8] = [JONAS, MARA, LEO, EMMA, MIA, NOAH, EVAN, SOFIA];

    #[test]
    fn every_core_resident_has_150_lines_of_their_own() {
        let mut everyone = BTreeSet::new();
        for person in CORE {
            let voice = voice(person).unwrap();
            let lines = lives::own_lines(voice);
            let distinct = lines.iter().collect::<BTreeSet<_>>();
            assert!(distinct.len() >= 150, "{person:?}: {}", distinct.len());
            for line in &lines {
                assert!(!line.contains('{'), "{line}");
                assert!(line.chars().count() <= 72, "one page: {line}");
                // What they never say, they never say.
                let said = lives::restyle(voice, line, 7).to_lowercase();
                for (never, _) in voice.instead {
                    assert!(
                        !said
                            .split(|c: char| !c.is_alphanumeric())
                            .any(|word| word == *never),
                        "{person:?} said {never}: {said}"
                    );
                }
                everyone.insert(line.clone());
            }
        }
        // Nobody's lines are anybody else's.
        let total: usize = CORE
            .iter()
            .map(|person| lives::own_lines(voice(*person).unwrap()).len())
            .sum();
        assert_eq!(everyone.len(), total);
    }

    #[test]
    fn every_core_resident_has_five_scenes_nobody_else_has() {
        let mut prompts = BTreeSet::new();
        for person in CORE {
            let voice = voice(person).unwrap();
            for scene in voice.scenes {
                assert!(prompts.insert(scene.prompt), "{}", scene.prompt);
                assert!(scene.prompt.chars().count() <= 90, "{}", scene.prompt);
            }
            assert!(voice.scenes[4].prompt.contains("{keepsake}"));
        }
        assert_eq!(prompts.len(), 40);
    }

    #[test]
    fn everyone_speaks_in_their_own_voice_as_days_pass() {
        let mut society = crate::TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        for _ in 0..90 {
            branch
                .invoke_projection_command(crate::story::WAIT_COMMAND)
                .unwrap();
        }
        for person in CORE {
            let own = lives::own_lines(voice(person).unwrap())
                .into_iter()
                .map(|line| line.trim_end_matches(['.', '!']).to_string())
                .collect::<Vec<_>>();
            let said = branch
                .world()
                .events()
                .iter()
                .filter(|event| event.kind == "lived" && event.actor == Some(person))
                .filter_map(|event| match event.payload.get("said") {
                    Some(world_core::Value::Text(said)) => Some(said.clone()),
                    _ => None,
                })
                .filter(|said| own.iter().any(|line| said.contains(line.as_str())))
                .collect::<BTreeSet<_>>();
            assert!(
                said.len() >= 5,
                "{person:?} said only {said:?} of their own"
            );
        }
    }
}
