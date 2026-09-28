//! How each pocket universe's two residents speak. Each has a style sheet
//! (the words they reach for and the ones they never use), a store of
//! lines of their own made from templates and what they talk about, and
//! five moments a friendship with them opens, which are theirs alone.
//!
//! The same two ids are different people in each place (the keeper is Nia
//! on Mars, Lena on Maple Street and Piko on the ice), so each place has
//! its own lookup, and the place's Cast picks the one that fits.

use lives::{Scene, Voice};
use world_core::EntityId;

use crate::{SLOT_B, SLOT_E};

const fn scene(prompt: &'static str, warm: &'static str, other: &'static str) -> Scene {
    Scene {
        prompt,
        replies: [warm, other],
    }
}

/// Nia Chen: the habitat's systems keeper, terse as a log entry, happiest
/// among the plants.
pub(crate) const NIA_VOICE: Voice = Voice {
    openers: &["Copy.", "Noted.", "Right."],
    closers: &[". Logged.", ". Over.", ". Holding steady."],
    instead: &[
        ("okay", "copy"),
        ("great", "nominal"),
        ("broken", "offline"),
    ],
    lines: &[
        "{system} reads {reading}. {verdict}",
        "{crop}: {growth}. {note}",
    ],
    slots: &[
        (
            "system",
            &[
                "Scrubber two",
                "The water loop",
                "Hull seal four",
                "The heat pump",
                "Airlock B",
                "The battery bank",
            ],
        ),
        (
            "reading",
            &["green", "a hair low", "a touch warm", "steady"],
        ),
        (
            "verdict",
            &[
                "Watching it.",
                "Logged.",
                "Fine for now.",
                "I'll check again at dusk.",
                "Nothing to do.",
            ],
        ),
        (
            "crop",
            &["Dwarf wheat", "Beans", "Tomatoes", "Lettuce", "Basil"],
        ),
        (
            "growth",
            &[
                "two new shoots",
                "roots look good",
                "leaves yellowing",
                "flowering early",
            ],
        ),
        (
            "note",
            &[
                "Humidity's right.",
                "Adjusting the lamps.",
                "Tomas wants a salad.",
                "Earth would be proud.",
            ],
        ),
    ],
    scenes: [
        scene(
            "Quiet shift. Sit by the bay with me. The wheat hums, if you listen.",
            "Hear it? That's good air.",
            "Another sol, then. Door's open.",
        ),
        scene(
            "Don't log this. Some nights I check the seals twice. I'm afraid.",
            "Thank you. Between us.",
            "Maybe I'll say it at the briefing.",
        ),
        scene(
            "First tomato of the season. Come and see. I haven't told anyone.",
            "Smell it. That's Earth.",
            "I'll save you a slice.",
        ),
        scene(
            "Anything in the habitat you need fixed goes to the top of my list.",
            "Consider it done.",
            "Top of the list. Standing order.",
        ),
        scene(
            "Grew this for you, leaf by leaf: {keepsake}.",
            "Water it once a sol.",
            "Put it in the light. It'll thrive.",
        ),
    ],
    keepsake: "a sprig of basil in a sealed sample jar",
};

/// Tomas Vale: the rover scout, never still, always one ridge further.
pub(crate) const TOMAS_VOICE: Voice = Voice {
    openers: &["Ha!", "Listen.", "Right, so."],
    closers: &[", no joke.", ". Kestrel agrees.", ", scout's honour."],
    instead: &[
        ("very", "seriously"),
        ("amazing", "wild"),
        ("nice", "sweet"),
    ],
    lines: &[
        "Took Kestrel to {where} {when}. {saw}",
        "Kestrel's {part} is {state}. {plan}",
        "Next sol I'm heading for {goal}. {ask}",
    ],
    slots: &[
        (
            "where",
            &[
                "the north ridge",
                "the dry riverbed",
                "Crater Six",
                "the dune field",
                "the old lander",
            ],
        ),
        ("when", &["at dawn", "before lunch", "on a hunch"]),
        (
            "saw",
            &[
                "Dust devil, huge.",
                "Boot-shaped rock!",
                "Nobody's been. Ever.",
                "Battery held. Barely.",
                "Views for days.",
                "Old tracks. Mine.",
            ],
        ),
        ("part", &["left wheel", "antenna", "battery", "dust filter"]),
        ("state", &["rattling", "acting up", "better than new"]),
        (
            "plan",
            &[
                "Nia will sort it.",
                "I'll strap it down.",
                "Still driving her tomorrow.",
                "Don't tell Nia.",
                "She's a tough old bird.",
            ],
        ),
        (
            "goal",
            &[
                "the far canyon",
                "the ice cliffs",
                "Olympus",
                "the dark dunes",
                "the unnamed ridge",
            ],
        ),
        (
            "ask",
            &[
                "Want the front seat?",
                "Pack me a sandwich.",
                "Nia says no. I say yes.",
                "Back by supper. Ish.",
            ],
        ),
    ],
    scenes: [
        scene(
            "Hey! You look like someone who'd love the view from the rover roof.",
            "See? Best seat on Mars.",
            "Offer's open. Kestrel waits.",
        ),
        scene(
            "Truth? I drive out so far because the quiet inside scares me.",
            "Thanks. Never said that to anyone.",
            "Maybe I'll stay in more. Maybe.",
        ),
        scene(
            "Ride out with me at dawn. Crater Six. I'll drive slow. Mostly.",
            "Hold on! There's the rim!",
            "Next dawn, then. I'll keep a seat.",
        ),
        scene(
            "Need anything from past the ridge? I'll fetch it. Any distance.",
            "Back before dark. Promise.",
            "Say the word and I'm rolling.",
        ),
        scene(
            "Picked this up the first sol I drove out. It's yours: {keepsake}.",
            "Take it out there sometime.",
            "Put it on the shelf. Let it rest.",
        ),
    ],
    keepsake: "a red pebble from the rim of Crater Six",
};

/// Lena Ortiz: closes the arcade by night and studies by day, tired and
/// kind and not giving up.
pub(crate) const LENA_VOICE: Voice = Voice {
    openers: &["Gosh.", "Hey.", "Oh, man."],
    closers: &[", you know?", ". Anyway.", ". Back to the books."],
    instead: &[("cool", "neat"), ("tired", "beat"), ("very", "real")],
    lines: &[
        "Locked the arcade at {hour}. {thing}",
        "{subject} exam {day}. {feel}",
        "Rode Night Bus 6 home {how}. {bus}",
    ],
    slots: &[
        ("hour", &["midnight", "one", "one-thirty", "two"]),
        (
            "thing",
            &[
                "Someone left a quarter in Galaga.",
                "Swept up a whole bag of popcorn.",
                "Mr. Dunn let me keep the tips.",
                "Max played a request for me.",
                "Studied between customers.",
                "The claw machine ate my dime.",
                "Counted tokens till my eyes crossed.",
                "A kid topped Centipede.",
            ],
        ),
        (
            "subject",
            &[
                "Biology",
                "Chemistry",
                "History",
                "Calculus",
                "French",
                "Spanish",
            ],
        ),
        ("day", &["on Monday", "on Friday", "tomorrow", "next week"]),
        (
            "feel",
            &[
                "I'm on it.",
                "Flashcards are ready.",
                "The coffee's ready too.",
                "I think I'll ace it.",
                "Please let there be a curve.",
            ],
        ),
        ("how", &["with my notes", "half asleep", "in the rain"]),
        (
            "bus",
            &[
                "Gus waved me on for free.",
                "Read a whole chapter.",
                "The heater actually worked.",
                "Watched Maple Street go by.",
            ],
        ),
    ],
    scenes: [
        scene(
            "Hey, you. I saved you a stool by the pinball. Stay a minute?",
            "See? The lights are pretty this late.",
            "Another night, then. Stool's yours.",
        ),
        scene(
            "Can I tell you something? I'm scared I'll fail and never leave.",
            "Thanks. I feel lighter already.",
            "Maybe I'll tell Mom. Maybe.",
        ),
        scene(
            "Ride the last bus with me. The whole loop. It's nicer than it sounds.",
            "Told you. Maple Street at two a.m.",
            "Next time. I'll save you the window.",
        ),
        scene(
            "You helped me study. Let me help you with anything. I mean it.",
            "Write it on my hand. I won't forget.",
            "Offer's good all semester.",
        ),
        scene(
            "I won this with my last token. It's yours: {keepsake}.",
            "Look after it for me.",
            "Put it on your shelf. It brightens things.",
        ),
    ],
    keepsake: "a plush alien from the Maple Arcade claw machine",
};

/// Max Park: K-88's night volunteer, a disc jockey even off the air.
pub(crate) const MAX_VOICE: Voice = Voice {
    openers: &["Hey hey!", "Dig it.", "Good evening, Maple Street!"],
    closers: &[
        ". Stay tuned.",
        ", over and out.",
        ". You heard it here first.",
    ],
    instead: &[
        ("excellent", "radical"),
        ("friend", "pal"),
        ("yes", "you bet"),
    ],
    lines: &[
        "Spun {song} {slot}. {calls}",
        "{caller} rang about {about}. {reply}",
        "Mixtape for {whom}: side A is {side}.",
    ],
    slots: &[
        (
            "song",
            &[
                "a slow one",
                "the Top 40 countdown",
                "a Springsteen B-side",
                "three ballads in a row",
                "an oldie for Mrs K",
                "a surf tune",
            ],
        ),
        ("slot", &["at midnight", "late", "on air"]),
        (
            "calls",
            &[
                "Phones lit up!",
                "One caller. My mom.",
                "They want it again.",
                "Got a fan letter!",
                "Lena called in.",
            ],
        ),
        (
            "caller",
            &["A trucker", "Some kid", "A night nurse", "A laundromat guy"],
        ),
        (
            "about",
            &[
                "a lost dog",
                "a first date",
                "the weather",
                "a nameless song",
            ],
        ),
        (
            "reply",
            &[
                "Played them some Journey.",
                "We talked a whole record.",
                "Told them to hang on.",
                "Dedicated one to them.",
            ],
        ),
        ("whom", &["Lena", "the bus driver", "my sister"]),
        (
            "side",
            &["all love songs", "pure synth", "songs to study to", "loud"],
        ),
    ],
    scenes: [
        scene(
            "Hey hey! Want to sit in the booth? Just don't touch the red button.",
            "You're a natural. Say hi to Maple Street!",
            "Booth's open any night.",
        ),
        scene(
            "Off the air? I talk all night because the quiet at home is loud.",
            "Thanks, pal. That means a lot.",
            "Maybe I'll turn the dial down. Sometimes.",
        ),
        scene(
            "Late show, Friday. Co-host with me. We'll take requests till dawn.",
            "We're on in three, two...",
            "I'll dedicate one to you anyway.",
        ),
        scene(
            "Need a song played for someone? Name it. I'll spin it on air.",
            "Consider it dedicated.",
            "The request line's always open.",
        ),
        scene(
            "Made you this. Side A is all you: {keepsake}.",
            "Play it loud.",
            "Play it for the whole street!",
        ),
    ],
    keepsake: "a mixtape labelled in silver marker",
};

/// Piko: the bridge keeper, slow and kind, who knows the ice by its song.
pub(crate) const PIKO_VOICE: Voice = Voice {
    openers: &["Hm-hm.", "Oh, hello.", "Slowly now."],
    closers: &[", little one.", ". The ice agrees.", ". Mind your feet."],
    instead: &[("walk", "waddle"), ("cold", "brisk"), ("home", "nest")],
    lines: &[
        "Crossed the {span} span at {time}. {ice}",
        "{who} crossed the bridge {how}. {care}",
        "Mended a span with {stuff}. {pride}",
    ],
    slots: &[
        ("span", &["first", "second", "long", "new"]),
        ("time", &["moonrise", "first light", "low tide", "dusk"]),
        (
            "ice",
            &[
                "The ice sang.",
                "Firm as a stone.",
                "A crack, but a small one.",
                "Frost on every rail.",
                "Not a wobble.",
                "The seals watched me.",
            ],
        ),
        ("who", &["Three chicks", "Old Uko", "A lost gull", "Miri"]),
        ("how", &["on tiptoe", "backwards", "in a rush", "singing"]),
        (
            "care",
            &[
                "I held my breath.",
                "All safe.",
                "I counted every step.",
                "The moon was watching.",
            ],
        ),
        ("stuff", &["snow bricks", "driftwood", "packed ice"]),
        (
            "pride",
            &[
                "It'll hold till spring.",
                "Stronger than before.",
                "Miri brought me a sprat for it.",
            ],
        ),
    ],
    scenes: [
        scene(
            "Come stand on the bridge with me. You can feel the sea breathing.",
            "There. Slow and deep, like a huddle.",
            "It breathes all night. Come later.",
        ),
        scene(
            "A secret: I'm afraid of the gap under the long span. Every time.",
            "Thank you. It feels smaller now.",
            "Maybe I'll tell the council one day.",
        ),
        scene(
            "At moonrise the ice glows blue. Cross the bridge with me tonight?",
            "See? Blue all the way down.",
            "The moon rises again tomorrow.",
        ),
        scene(
            "If the ice ever gives way under you, I'll be there. Keeper's promise.",
            "Keeper's promise. Sealed.",
            "The promise keeps. Like good ice.",
        ),
        scene(
            "I've kept this since I built the first span. Now it's yours: {keepsake}.",
            "Keep it cool. It likes that.",
            "Show it to the little ones.",
        ),
    ],
    keepsake: "a smooth grey pebble from under the first span",
};

/// Miri: keeper of the fish vault, a chatterbox who counts everything and
/// hears every bit of news first.
pub(crate) const MIRI_VOICE: Voice = Voice {
    openers: &["Ooh!", "Guess what!", "Psst."],
    closers: &[", fishy promise.", "! Squawk!", ". I counted."],
    instead: &[
        ("great", "splendid"),
        ("food", "fish"),
        ("nothing", "not a sprat"),
    ],
    lines: &[
        "{count} {fish} in the vault. {feel}",
        "Aurora Council {did} over {topic}. {view}",
        "Off to {where} next moon. {plea}",
        "{gossip}, and I heard it first!",
    ],
    slots: &[
        (
            "count",
            &["Forty", "Ninety-nine", "A hundred and six", "Twelve"],
        ),
        ("fish", &["herring", "sprats", "silverfish", "krill cakes"]),
        (
            "feel",
            &[
                "Not bad at all!",
                "I counted twice.",
                "Someone's been nibbling.",
                "Winter's covered!",
                "Don't tell the gulls.",
            ],
        ),
        ("did", &["squawked", "voted", "argued"]),
        (
            "topic",
            &[
                "the fishing holes",
                "the huddle order",
                "the new slide",
                "the moon feast",
            ],
        ),
        (
            "view",
            &[
                "I said my piece!",
                "It took all night.",
                "I nearly fell asleep.",
                "Piko kept us calm.",
                "Nobody asked me!",
            ],
        ),
        (
            "where",
            &[
                "the far floe",
                "the south shelf",
                "the whale road",
                "the big berg",
            ],
        ),
        (
            "plea",
            &[
                "Who'll waddle with me?",
                "I'll bring snacks!",
                "Save my spot in the huddle.",
                "Don't wait up!",
            ],
        ),
        (
            "gossip",
            &[
                "Old Uko has a new pebble",
                "The seal is back",
                "Pip slid right into the vault",
                "The gulls are plotting",
                "The moon feast is early",
                "Kiki learned to dive",
            ],
        ),
    ],
    scenes: [
        scene(
            "Psst! I saved you the fattest herring. Don't tell a soul!",
            "Good, isn't it? Best in the vault!",
            "More for me, then! Ha!",
        ),
        scene(
            "I chatter all day so nobody hears how empty my nest is at night.",
            "Thank you. I feel warmer already.",
            "Maybe I'll say it at a huddle.",
        ),
        scene(
            "Slide down the big slope with me! At moonrise! Say yes!",
            "Wheee! Again, again!",
            "Next moon, then. I'm holding you to it!",
        ),
        scene(
            "You'll never go hungry while I keep the vault. Not ever.",
            "First pick, always. Keeper's word.",
            "Your fish is waiting. Whenever.",
        ),
        scene(
            "I've never given this to anyone: {keepsake}. Now it's yours!",
            "Wear it to the aurora!",
            "Show everyone! Say Miri gave it!",
        ),
    ],
    keepsake: "a silver fish scale on a string of kelp",
};

/// The voices of Ares Habitat's two.
pub(crate) fn mars(person: EntityId) -> Option<&'static Voice> {
    Some(match person {
        SLOT_B => &NIA_VOICE,
        SLOT_E => &TOMAS_VOICE,
        _ => return None,
    })
}

/// The voices of Maple Street's two.
pub(crate) fn town(person: EntityId) -> Option<&'static Voice> {
    Some(match person {
        SLOT_B => &LENA_VOICE,
        SLOT_E => &MAX_VOICE,
        _ => return None,
    })
}

/// The voices of Icebridge's two.
pub(crate) fn ice(person: EntityId) -> Option<&'static Voice> {
    Some(match person {
        SLOT_B => &PIKO_VOICE,
        SLOT_E => &MIRI_VOICE,
        _ => return None,
    })
}

/// Everyone with a voice of their own, in every place.
pub(crate) const ALL: [&Voice; 6] = [
    &NIA_VOICE,
    &TOMAS_VOICE,
    &LENA_VOICE,
    &MAX_VOICE,
    &PIKO_VOICE,
    &MIRI_VOICE,
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Words that belong to the engine, not to anyone living here.
    const ENGINE_WORDS: [&str; 8] = [
        "actor", "entity", "event", "system", "world", "agent", "action", "state",
    ];

    #[test]
    fn every_core_resident_has_150_lines_of_their_own() {
        let mut everyone = BTreeSet::new();
        let mut templates = BTreeSet::new();
        for voice in ALL {
            for template in voice.lines {
                assert!(templates.insert(*template), "shared: {template}");
            }
            let lines = lives::own_lines(voice);
            let distinct = lines.iter().collect::<BTreeSet<_>>();
            assert!(
                (150..=260).contains(&distinct.len()),
                "{}: {}",
                voice.keepsake,
                distinct.len()
            );
            for line in &lines {
                assert!(!line.contains('{'), "{line}");
                assert!(line.chars().count() <= 70, "one page: {line}");
                let said = lives::restyle(voice, line, 7).to_lowercase();
                let words = said
                    .split(|c: char| !c.is_alphanumeric())
                    .collect::<Vec<_>>();
                // What they never say, they never say.
                for (never, _) in voice.instead {
                    assert!(!words.contains(never), "said {never}: {said}");
                }
                for word in ENGINE_WORDS {
                    assert!(!words.contains(&word), "engine word: {said}");
                }
                everyone.insert(line.clone());
            }
        }
        // Nobody's lines are anybody else's.
        let total: usize = ALL.iter().map(|voice| lives::own_lines(voice).len()).sum();
        assert_eq!(everyone.len(), total);
    }

    #[test]
    fn every_core_resident_has_five_scenes_nobody_else_has() {
        let mut prompts = BTreeSet::new();
        let mut keepsakes = BTreeSet::new();
        for voice in ALL {
            for scene in voice.scenes {
                assert!(prompts.insert(scene.prompt), "{}", scene.prompt);
                assert!(scene.prompt.chars().count() <= 90, "{}", scene.prompt);
                for reply in scene.replies {
                    assert!(reply.chars().count() <= 60, "{reply}");
                }
            }
            assert!(voice.scenes[4].prompt.contains("{keepsake}"));
            assert!(keepsakes.insert(voice.keepsake));
        }
        assert_eq!(prompts.len(), 30);
    }

    #[test]
    fn each_place_gives_its_own_two_their_own_voices() {
        for lookup in [mars, town, ice] {
            assert!(lookup(SLOT_B).is_some());
            assert!(lookup(SLOT_E).is_some());
            assert!(lookup(crate::story::NEWCOMER).is_none());
        }
        assert_eq!(mars(SLOT_B).unwrap().keepsake, NIA_VOICE.keepsake);
        assert_eq!(town(SLOT_B).unwrap().keepsake, LENA_VOICE.keepsake);
        assert_eq!(ice(SLOT_E).unwrap().keepsake, MIRI_VOICE.keepsake);
    }

    #[test]
    fn everyone_speaks_in_their_own_voice_as_days_pass() {
        for (seed, lookup) in [
            (crate::SEED_MARS_COLONY_COMMAND, mars as fn(_) -> _),
            (crate::SEED_1980S_TOWN_COMMAND, town),
            (crate::SEED_PENGUIN_CIVILIZATION_COMMAND, ice),
        ] {
            let mut universe = crate::PocketUniverse::new().unwrap();
            universe.invoke_projection_command(seed).unwrap();
            for _ in 0..60 {
                universe.advance_periods(1).unwrap();
            }
            for person in [SLOT_B, SLOT_E] {
                let own = lives::own_lines(lookup(person).unwrap())
                    .into_iter()
                    .map(|line| line.trim_end_matches(['.', '!']).to_string())
                    .collect::<Vec<_>>();
                let said = universe
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
                    said.len() >= 3,
                    "{seed} {person:?} said only {said:?} of their own"
                );
            }
        }
    }
}
