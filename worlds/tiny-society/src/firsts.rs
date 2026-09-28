//! The harbour's quiet days: two letters a week at most, and on the other
//! quiet days a small first. Someone speaks of something they have never
//! mentioned, or shows the player a corner of the harbour they have not
//! seen.

use crate::{BAKERY, HARBOR, PUB, SCHOOL};
use lives::{Corner, QuietDays, Subject};

/// The most letters in any week.
pub(crate) const LETTERS_A_WEEK: usize = 2;

const CORNERS: &[Corner] = &[
    Corner {
        place: HARBOR,
        name: "the old net loft",
        said: "Nobody comes up here but me. Best view of the boats.",
    },
    Corner {
        place: HARBOR,
        name: "the steps below the quay",
        said: "Sit here at low tide. You can hear the crabs.",
    },
    Corner {
        place: HARBOR,
        name: "the harbourmaster's hut",
        said: "Nobody's been harbourmaster for years. The kettle still works.",
    },
    Corner {
        place: HARBOR,
        name: "the lobster pots behind the slipway",
        said: "Every one of these has a name. Don't ask.",
    },
    Corner {
        place: HARBOR,
        name: "the bench at the end of the breakwater",
        said: "Best seat on the island. Don't tell anyone.",
    },
    Corner {
        place: HARBOR,
        name: "the tide board",
        said: "My dad painted these numbers. Still right, mostly.",
    },
    Corner {
        place: HARBOR,
        name: "the rock pools by the point",
        said: "Starfish, if you're patient.",
    },
    Corner {
        place: BAKERY,
        name: "the back room of the bakery",
        said: "This is where the bread rises. Mind the draught.",
    },
    Corner {
        place: BAKERY,
        name: "the old bread oven",
        said: "My gran baked in this. It still bakes best.",
    },
    Corner {
        place: BAKERY,
        name: "the flour loft",
        said: "Up the ladder. Sneeze if you must.",
    },
    Corner {
        place: BAKERY,
        name: "the bakery's back step",
        said: "Where I sit with a cup before the first batch.",
    },
    Corner {
        place: SCHOOL,
        name: "the school's map room",
        said: "Every map of the island ever drawn. Some of them are even right.",
    },
    Corner {
        place: SCHOOL,
        name: "the school bell tower",
        said: "Ring it once, gently. Everyone will know it was you.",
    },
    Corner {
        place: SCHOOL,
        name: "the bottom of the school garden",
        said: "The children bury treasure here. Don't dig.",
    },
    Corner {
        place: SCHOOL,
        name: "the cloakroom pegs",
        said: "Every child who ever went here has a peg. Mine's the crooked one.",
    },
    Corner {
        place: SCHOOL,
        name: "the reading nook under the stairs",
        said: "Best hiding place on the island. Ask any child.",
    },
    Corner {
        place: PUB,
        name: "the snug at the Anchor",
        said: "Three seats and a fire. The best room in the pub.",
    },
    Corner {
        place: PUB,
        name: "the Anchor's cellar",
        said: "Cool as anything down here, even in August.",
    },
    Corner {
        place: PUB,
        name: "the photographs behind the bar",
        said: "Every regatta since the war. Find me, if you can.",
    },
    Corner {
        place: PUB,
        name: "the pub's back garden",
        said: "One table, one apple tree, and the sea.",
    },
    Corner {
        place: PUB,
        name: "the dartboard with the hole in it",
        said: "Nobody will say who threw that. Everybody knows.",
    },
];

const SUBJECTS: &[Subject] = &[
    Subject {
        about: "the lighthouse",
        said: &[
            "My grandad used to wind the lamp by hand.",
            "On a clear night I count the flashes till I fall asleep.",
        ],
    },
    Subject {
        about: "the ferry",
        said: &[
            "It's never once been on time, and I'd miss it if it were.",
            "I nearly left on it once. Glad I didn't.",
        ],
    },
    Subject {
        about: "the mainland",
        said: &[
            "I go twice a year and come back tired.",
            "Too many cars. Too few boats.",
        ],
    },
    Subject {
        about: "the winter storms",
        said: &[
            "The whole island holds its breath.",
            "I like them, secretly. Everyone's indoors together.",
        ],
    },
    Subject {
        about: "the seals",
        said: &[
            "There's one that follows my boat. I call her Maud.",
            "They watch us like we're the odd ones.",
        ],
    },
    Subject {
        about: "the old chapel",
        said: &[
            "Nobody's married there in years. It's still lovely.",
            "The roof lets in more light than the windows.",
        ],
    },
    Subject {
        about: "the tides",
        said: &[
            "You live by them here, whether you like it or not.",
            "Spring tides come right up to my door.",
        ],
    },
    Subject {
        about: "fishing",
        said: &[
            "I'm no good at it, and I love it anyway.",
            "It's all waiting, really. I'm good at waiting.",
        ],
    },
    Subject {
        about: "the gulls",
        said: &[
            "They've stolen three of my pasties this year.",
            "Noisy things. I'd miss them.",
        ],
    },
    Subject {
        about: "the Saturday market",
        said: &[
            "I go for the gossip, not the cabbages.",
            "Same stalls every week, and I never tire of it.",
        ],
    },
    Subject {
        about: "baking day",
        said: &[
            "The whole lane smells of it.",
            "Fresh bread is the only reason I get up some days.",
        ],
    },
    Subject {
        about: "the school",
        said: &[
            "I learned to read in that room.",
            "It's small, but it's ours.",
        ],
    },
    Subject {
        about: "the harbour fund",
        said: &[
            "Every coin in that tin has a story.",
            "I put in what I can. Never enough.",
        ],
    },
    Subject {
        about: "the old days",
        said: &[
            "It was quieter. Not better, mind.",
            "People say it was better. I'm not so sure.",
        ],
    },
    Subject {
        about: "the weather",
        said: &[
            "You can smell rain coming here, you know.",
            "Four seasons in a day, most days.",
        ],
    },
    Subject {
        about: "the island",
        said: &[
            "I've lived here all my life, or near enough.",
            "It took me years to call it home.",
        ],
    },
    Subject {
        about: "the stars",
        said: &[
            "No streetlights here. You see all of them.",
            "I learned their names from my mother.",
        ],
    },
    Subject {
        about: "the cliffs",
        said: &[
            "I walk them every Sunday, rain or shine.",
            "Don't go near the edge in a wind.",
        ],
    },
    Subject {
        about: "the island's cats",
        said: &[
            "There's more of them than of us.",
            "They all belong to everyone.",
        ],
    },
    Subject {
        about: "the boats",
        said: &[
            "Every boat here has a name and a temper.",
            "You can tell who's out by the colour of the sails.",
        ],
    },
];

/// Everything people can speak of for the first time: the harbour's own
/// subjects, then its days of the year and its works, which are spoken of
/// in a few words that fit anything.
fn subjects() -> &'static [Subject] {
    static ALL: std::sync::OnceLock<Vec<Subject>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let mut all = SUBJECTS.to_vec();
        let festivals = crate::almanac::FESTIVALS
            .iter()
            .chain(crate::years::NEW_FESTIVALS)
            .map(|festival| festival.name);
        let works = crate::story::WORKS
            .iter()
            .map(|work| crate::story::leak(crate::story::the(work.label)));
        for about in festivals.chain(works) {
            if !all.iter().any(|subject| subject.about == about) {
                all.push(Subject { about, said: &[] });
            }
        }
        all
    })
}

/// How the harbour keeps its quiet days.
pub(crate) fn quiet_days() -> QuietDays {
    QuietDays {
        letters_a_week: Some(LETTERS_A_WEEK),
        corners: CORNERS,
        subjects: subjects(),
    }
}
