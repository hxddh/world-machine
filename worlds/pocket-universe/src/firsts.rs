//! Each place's quiet days: two letters a week at most, and on the other
//! quiet days a small first. Someone speaks of something they have never
//! mentioned, or shows the player a corner of the place they have not
//! seen: an observation blister on Ares, the high-score wall on Maple
//! Street, the chick crèche on Icebridge. The mechanics are the `lives`
//! System's; the corners and what people speak of are each place's own.

use crate::{SLOT_A, SLOT_C, SLOT_D, UNIVERSE};
use lives::{Corner, QuietDays, Subject};
use world_core::{Value, WorldState};

/// The most letters in any week, in every place.
pub(crate) const LETTERS_A_WEEK: usize = 2;

const MARS_CORNERS: &[Corner] = &[
    Corner {
        place: SLOT_A,
        name: "the observation blister",
        said: "Best seat on Mars. Don't tell anyone it's here.",
    },
    Corner {
        place: SLOT_A,
        name: "the old landing pod",
        said: "The first of us slept in here. It still smells of new plastic.",
    },
    Corner {
        place: SLOT_A,
        name: "the water reclaimer",
        said: "Every drop you drink has been through here. Try not to think about it.",
    },
    Corner {
        place: SLOT_A,
        name: "the suit room",
        said: "Every suit has a name. Mine's called Rusty.",
    },
    Corner {
        place: SLOT_A,
        name: "the radio shack",
        said: "If Earth ever calls at night, this is where you'll hear it.",
    },
    Corner {
        place: SLOT_C,
        name: "the seed vault",
        said: "Every seed from Earth, in little envelopes. I check on them like children.",
    },
    Corner {
        place: SLOT_C,
        name: "the algae tanks",
        said: "They bubble all night. It's the most restful sound on Mars.",
    },
    Corner {
        place: SLOT_C,
        name: "the back of the grow lights",
        said: "It's pink in here, always. You get used to it.",
    },
    Corner {
        place: SLOT_D,
        name: "the rover's toolbox",
        said: "Every tool has a place. Every place has a tool. Mostly.",
    },
    Corner {
        place: SLOT_D,
        name: "the dune track",
        said: "Our own road. We made it one tyre at a time.",
    },
];

const MARS_SUBJECTS: &[Subject] = &[
    Subject {
        about: "the dust storms",
        said: &[
            "They go on for weeks. You learn to love the indoors.",
            "The whole sky turns orange. It's beautiful, from inside.",
        ],
    },
    Subject {
        about: "Earth",
        said: &[
            "I don't miss it as much as I thought I would.",
            "Some nights I look for it and can't find it.",
        ],
    },
    Subject {
        about: "the food",
        said: &[
            "Everything tastes a bit of the recycler.",
            "I'd trade a week of rations for one fresh orange.",
        ],
    },
    Subject {
        about: "the silence",
        said: &[
            "Outside there's no sound at all. None.",
            "You can hear your own heart in a suit.",
        ],
    },
    Subject {
        about: "Phobos",
        said: &[
            "It races across the sky twice a sol. Show-off.",
            "I wave at it sometimes. Don't laugh.",
        ],
    },
    Subject {
        about: "the cold",
        said: &[
            "Minus eighty at night. The walls creak.",
            "Your breath fogs in the airlock every morning.",
        ],
    },
    Subject {
        about: "the gravity",
        said: &[
            "I can jump over the table here. Everyone does, once.",
            "Earth would feel like carrying a sack of rocks now.",
        ],
    },
    Subject {
        about: "the sunsets",
        said: &[
            "Blue sunsets and red days. Everything's upside down.",
            "I stopped noticing them. Then some sols I can't stop.",
        ],
    },
    Subject {
        about: "the supply ship",
        said: &[
            "Everyone counts the sols till it comes.",
            "Last time it brought socks. Best day of the year.",
        ],
    },
    Subject {
        about: "the first sol",
        said: &[
            "Nobody slept at all. Too excited.",
            "I remember thinking, what have we done?",
        ],
    },
    Subject {
        about: "the craters",
        said: &[
            "Some are older than anything on Earth.",
            "Every one of them has a name now. We named them.",
        ],
    },
    Subject {
        about: "home",
        said: &[
            "Home's wherever the airlock seals.",
            "I say home and mean here now. When did that happen?",
        ],
    },
];

const TOWN_CORNERS: &[Corner] = &[
    Corner {
        place: SLOT_A,
        name: "the back room of the arcade",
        said: "Broken machines come here to be fixed. Or to rest.",
    },
    Corner {
        place: SLOT_A,
        name: "the high-score wall",
        said: "Every name up there fought for it. Mine's third.",
    },
    Corner {
        place: SLOT_A,
        name: "the change machine",
        said: "It jams if you look at it wrong. Kick it here.",
    },
    Corner {
        place: SLOT_A,
        name: "the alley behind the diner",
        said: "Best place for a secret. Worst smell in town.",
    },
    Corner {
        place: SLOT_A,
        name: "the roof over the arcade",
        said: "Up the fire escape. Watch the whole town go to bed.",
    },
    Corner {
        place: SLOT_C,
        name: "the record library",
        said: "Every record K-88 ever played. Some twice.",
    },
    Corner {
        place: SLOT_C,
        name: "the studio booth",
        said: "When the red light's on, you whisper. Even if nobody's listening.",
    },
    Corner {
        place: SLOT_C,
        name: "the aerial on the roof",
        said: "You can see all of Maple Street from here. And the drive-in.",
    },
    Corner {
        place: SLOT_D,
        name: "the back seat of the bus",
        said: "Best seat on the loop. Everybody knows it.",
    },
    Corner {
        place: SLOT_D,
        name: "the bus depot",
        said: "The night buses sleep here in a row, like horses.",
    },
];

const TOWN_SUBJECTS: &[Subject] = &[
    Subject {
        about: "the diner",
        said: &[
            "Best pie on Maple Street, and the worst coffee.",
            "I've had my birthday there every year since I was six.",
        ],
    },
    Subject {
        about: "the high school",
        said: &[
            "The halls still smell of floor wax.",
            "I carved my name under a desk in room twelve.",
        ],
    },
    Subject {
        about: "the radio",
        said: &[
            "Some nights it's the only voice I hear.",
            "I call in requests under a fake name.",
        ],
    },
    Subject {
        about: "the mall",
        said: &[
            "They say it'll close the arcade down. It won't.",
            "Everyone goes. Nobody buys anything.",
        ],
    },
    Subject {
        about: "my car",
        said: &[
            "It only starts if you say please.",
            "Half the tape deck works. The good half.",
        ],
    },
    Subject {
        about: "the summer",
        said: &[
            "Summer here smells of cut grass and hot tar.",
            "Best summer of my life was the one with the flood.",
        ],
    },
    Subject {
        about: "the drive-in",
        said: &[
            "I've seen every movie there twice. Once watching, once not.",
            "They let you in free if you help park the cars.",
        ],
    },
    Subject {
        about: "the lake",
        said: &[
            "The water's freezing even in August.",
            "There's a rope swing nobody admits to building.",
        ],
    },
    Subject {
        about: "my family",
        said: &[
            "We're loud. You'd like us.",
            "My mom still packs my lunch. Don't tell anyone.",
        ],
    },
    Subject {
        about: "the future",
        said: &[
            "Everyone says computers. I say we'll see.",
            "I'll leave one day. Then I'll come back.",
        ],
    },
    Subject {
        about: "music",
        said: &[
            "Some songs are better on a Walkman, walking in the rain.",
            "A mixtape says what you can't.",
        ],
    },
    Subject {
        about: "the neighborhood",
        said: &[
            "Everybody knows everybody's business here.",
            "The same families on the same porches, every summer.",
        ],
    },
];

const ICE_CORNERS: &[Corner] = &[
    Corner {
        place: SLOT_A,
        name: "the lantern shelf",
        said: "Every lantern gets polished here. Beaks only.",
    },
    Corner {
        place: SLOT_A,
        name: "the old ice arch",
        said: "The first span of the bridge. It's still holding.",
    },
    Corner {
        place: SLOT_A,
        name: "the chick crèche",
        said: "All the little ones huddle here. Mind your feet.",
    },
    Corner {
        place: SLOT_A,
        name: "the windbreak wall",
        said: "Stand here in a blizzard and you'd never know.",
    },
    Corner {
        place: SLOT_A,
        name: "the snow slide",
        said: "The fastest way home. Belly first.",
    },
    Corner {
        place: SLOT_C,
        name: "the deep vault",
        said: "The oldest fish in the colony. We don't eat these.",
    },
    Corner {
        place: SLOT_C,
        name: "the counting stones",
        said: "One pebble for every fish. Don't move them!",
    },
    Corner {
        place: SLOT_C,
        name: "the diving hole",
        said: "Straight down to the good fishing. Hold your breath.",
    },
    Corner {
        place: SLOT_D,
        name: "the council ledge",
        said: "Where the council stands to vote. A very serious ledge.",
    },
    Corner {
        place: SLOT_D,
        name: "the moonrise stone",
        said: "When the moon touches it, the vote begins.",
    },
];

const ICE_SUBJECTS: &[Subject] = &[
    Subject {
        about: "the aurora",
        said: &[
            "It sings, if you're very quiet. I'm sure of it.",
            "Green one night, pink the next.",
        ],
    },
    Subject {
        about: "the seals",
        said: &[
            "We don't talk about the seals. Well, I'm talking now.",
            "One looked at me once. I'll never forget it.",
        ],
    },
    Subject {
        about: "the long night",
        said: &[
            "The dark goes on and on. We huddle and tell tales.",
            "You forget what the sun looks like.",
        ],
    },
    Subject {
        about: "fish",
        said: &[
            "A herring a day, and nobody's sad.",
            "I dream of fish. Good dreams.",
        ],
    },
    Subject {
        about: "the sea",
        said: &[
            "Under the ice the sea is another world.",
            "It's cold, it's dark, and I love it.",
        ],
    },
    Subject {
        about: "eggs",
        said: &[
            "An egg on your feet all winter. That's love.",
            "Mine hatched on the windiest day of the year.",
        ],
    },
    Subject {
        about: "the whales",
        said: &[
            "They sing under the ice. You feel it in your belly.",
            "One came up by the floe once. The whole colony went quiet.",
        ],
    },
    Subject {
        about: "the old colony",
        said: &[
            "My grandmother came from there. She never talked about it.",
            "They say the ice was thicker then.",
        ],
    },
    Subject {
        about: "snow",
        said: &[
            "There's good snow and bad snow. You learn.",
            "Fresh snow squeaks. Did you know?",
        ],
    },
    Subject {
        about: "the gulls",
        said: &[
            "Thieves, every one of them.",
            "They laugh at us. I'm sure of it.",
        ],
    },
    Subject {
        about: "feathers",
        said: &[
            "Moulting season is the worst. Itchy all over.",
            "A good feather is worth more than a fish.",
        ],
    },
    Subject {
        about: "the bridge",
        said: &[
            "Every span has a name. I know them all.",
            "It creaks at night. It's talking.",
        ],
    },
];

/// How the place keeps its quiet days.
pub(crate) fn quiet_days(state: &WorldState) -> QuietDays {
    let seed = match state
        .entity(UNIVERSE)
        .and_then(|universe| universe.component(crate::SEED))
    {
        Some(Value::Text(seed)) => seed.as_str(),
        _ => "",
    };
    let (corners, subjects) = match seed {
        "mars-colony" => (MARS_CORNERS, MARS_SUBJECTS),
        "1980s-town" => (TOWN_CORNERS, TOWN_SUBJECTS),
        "penguin-civilization" => (ICE_CORNERS, ICE_SUBJECTS),
        _ => return QuietDays::default(),
    };
    QuietDays {
        letters_a_week: Some(LETTERS_A_WEEK),
        corners,
        subjects,
    }
}
