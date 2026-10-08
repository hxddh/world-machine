//! Each place's year: its festivals and the days that simply arrive, and
//! how each goes depends on the place as it stands.

use crate::{RELATIONSHIP, SLOT_A, SLOT_B, SLOT_C, SLOT_E, UNIVERSE};
use calendar::{Almanac, Festival, Turnout};
use world_core::{EntityId, StateChange, Value, WorldState};

/// Periods in a year, everywhere.
pub(crate) const YEAR: u64 = crate::story::SEASON_PERIODS * 4;

pub(crate) const MARS: &[Festival] = &[
    Festival {
        id: "landing_day",
        name: "Landing Day",
        day: 4,
        prepare: 5,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "Landing Day is {days} sols off. The old flag is coming out of storage.",
        getting_ready: "Landing Day soon. I'm polishing the plaque.",
        told: [
            "Everyone in the habitat gathered for Landing Day",
            "The habitat marked Landing Day",
            "Landing Day passed with a quiet toast",
        ],
        said: [
            "All of us, together. Like the first day.",
            "We made it another year.",
            "Just the two of us and the plaque.",
        ],
    },
    Festival {
        id: "first_sprouts",
        name: "the first sprouts",
        day: 12,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The first sprouts broke the soil in the greenhouse",
            "The first sprouts broke the soil in the greenhouse",
            "The first sprouts broke the soil in the greenhouse",
        ],
        said: [
            "Green! Actual green!",
            "Green! Actual green!",
            "Green! Actual green!",
        ],
    },
    Festival {
        id: "dust_drill",
        name: "the Dust Drill",
        day: 20,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "The Dust Drill is {days} sols away. Seals are being checked.",
        getting_ready: "Drill in a few sols. Know where your mask is?",
        told: [
            "The Dust Drill went like clockwork",
            "The habitat ran its Dust Drill",
            "The Dust Drill was a shambles",
        ],
        said: [
            "Sealed in under a minute!",
            "Good enough.",
            "We'd never survive a real one.",
        ],
    },
    Festival {
        id: "earthrise",
        name: "Earthrise Night",
        day: 28,
        prepare: 4,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "lantern",
        harvest: false,
        nears: "Earthrise Night is {days} sols off. Telescopes are being cleaned.",
        getting_ready: "Earthrise soon. I've found the best spot on the ridge.",
        told: [
            "Everyone watched Earth rise from the ridge",
            "A few went up to watch Earthrise",
            "Earth rose, and hardly anyone looked up",
        ],
        said: [
            "There it is. Home, once.",
            "Pretty, isn't it?",
            "Nobody came to see it.",
        ],
    },
    Festival {
        id: "long_sol",
        name: "the longest sol",
        day: 40,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The longest sol of the year came and went",
            "The longest sol of the year came and went",
            "The longest sol of the year came and went",
        ],
        said: [
            "Longest sol. The light goes on forever.",
            "Longest sol. The light goes on forever.",
            "Longest sol. The light goes on forever.",
        ],
    },
    Festival {
        id: "rover_rally",
        name: "the Rover Rally",
        day: 46,
        prepare: 6,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "flag",
        harvest: false,
        nears: "The Rover Rally is {days} sols off. Engines are being tuned.",
        getting_ready: "Tuning the rover for the rally. I'm going to win.",
        told: [
            "The Rover Rally raced out past the ridge with everyone cheering",
            "The rovers raced their rally",
            "The Rover Rally was one rover and a lot of dust",
        ],
        said: [
            "Did you see that finish?",
            "Fun, if dusty.",
            "Raced myself. Came second.",
        ],
    },
    Festival {
        id: "water_feast",
        name: "the Water Feast",
        day: 55,
        prepare: 4,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "tent",
        harvest: false,
        nears: "The Water Feast is {days} sols away. The recycler is being coaxed.",
        getting_ready: "Saving water for the Feast. Every drop.",
        told: [
            "The Water Feast was the best the habitat has had",
            "The habitat shared its Water Feast",
            "The Water Feast was a cup each",
        ],
        said: [
            "Real baths! Everyone!",
            "A good feast.",
            "Barely a sip to go round.",
        ],
    },
    Festival {
        id: "dust_devil_watch",
        name: "the Dust Devil Watch",
        day: 62,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "The Dust Devil Watch is {days} sols off. Chairs on the ridge.",
        getting_ready: "Dust devil season. Bring a flask up the ridge.",
        told: [
            "The whole habitat counted dust devils from the ridge",
            "The habitat watched the dust devils go by",
            "The Dust Devil Watch was mostly dust",
        ],
        said: [
            "Seven at once! I counted!",
            "Nice evening on the ridge.",
            "Saw one. Maybe.",
        ],
    },
    Festival {
        id: "greenhouse_harvest",
        name: "the Greenhouse Harvest",
        day: 70,
        prepare: 7,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "stall",
        harvest: true,
        nears: "The Greenhouse Harvest is {days} sols away. Everything is nearly ripe.",
        getting_ready: "A week to the harvest. Don't touch the tomatoes.",
        told: [
            "The Greenhouse Harvest filled every crate",
            "The Greenhouse Harvest brought in enough",
            "The Greenhouse Harvest was a few sad leaves",
        ],
        said: [
            "We'll eat like kings!",
            "Enough for the winter.",
            "Hardly anything grew.",
        ],
    },
    Festival {
        id: "phobos",
        name: "Phobos at noon",
        day: 78,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "Phobos crossed the sun at noon",
            "Phobos crossed the sun at noon",
            "Phobos crossed the sun at noon",
        ],
        said: [
            "Look up! Phobos is crossing the sun.",
            "Look up! Phobos is crossing the sun.",
            "Look up! Phobos is crossing the sun.",
        ],
    },
    Festival {
        id: "founders_feast",
        name: "Founders' Feast",
        day: 86,
        prepare: 5,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "lantern",
        harvest: false,
        nears: "Founders' Feast is {days} sols off. The good rations are coming out.",
        getting_ready: "Founders' Feast soon. I'm saving the good rations.",
        told: [
            "Founders' Feast had everyone round one table",
            "The habitat held its Founders' Feast",
            "Founders' Feast was a ration bar and a toast",
        ],
        said: [
            "To the founders! To all of us!",
            "A good meal.",
            "Hardly a feast.",
        ],
    },
    Festival {
        id: "dust_season",
        name: "dust season",
        day: 95,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "Dust season rolled in over the ridge",
            "Dust season rolled in over the ridge",
            "Dust season rolled in over the ridge",
        ],
        said: [
            "Dust season. Keep the seals tight.",
            "Dust season. Keep the seals tight.",
            "Dust season. Keep the seals tight.",
        ],
    },
    Festival {
        id: "signal_night",
        name: "Signal Night",
        day: 104,
        prepare: 5,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "lantern",
        harvest: false,
        nears: "Signal Night is {days} sols away. Everyone's writing to Earth.",
        getting_ready: "Writing my message for Signal Night.",
        told: [
            "Signal Night sent a message from every one of us",
            "The habitat sent its messages home",
            "Signal Night went out almost empty",
        ],
        said: [
            "Earth will hear all of us.",
            "Messages sent.",
            "Hardly anyone wrote.",
        ],
    },
    Festival {
        id: "year_turn",
        name: "the Year's Turn",
        day: 116,
        prepare: 2,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "The Year's Turn is {days} sols off.",
        getting_ready: "Another Mars year nearly done.",
        told: [
            "The whole habitat saw the year turn",
            "The habitat saw the year out",
            "The year turned, and most slept through it",
        ],
        said: [
            "Another year! Together!",
            "Another year.",
            "Quiet end to the year.",
        ],
    },
];

pub(crate) const TOWN: &[Festival] = &[
    Festival {
        id: "spring_dance",
        name: "the Spring Dance",
        day: 6,
        prepare: 5,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "bunting",
        harvest: false,
        nears: "The Spring Dance is {days} nights off. Everyone's finding a partner.",
        getting_ready: "Spring Dance soon. I'm making the mixtape.",
        told: [
            "The Spring Dance packed the arcade until midnight",
            "Maple Street had its Spring Dance",
            "The Spring Dance was three people and a mixtape",
        ],
        said: ["Best night ever!", "Pretty good dance.", "Nobody danced."],
    },
    Festival {
        id: "blossom",
        name: "the cherry blossom",
        day: 14,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The cherry trees on Maple Street burst into blossom",
            "The cherry trees on Maple Street burst into blossom",
            "The cherry trees on Maple Street burst into blossom",
        ],
        said: [
            "The cherry trees are out! Pink everywhere.",
            "The cherry trees are out! Pink everywhere.",
            "The cherry trees are out! Pink everywhere.",
        ],
    },
    Festival {
        id: "yard_sale",
        name: "the Street Yard Sale",
        day: 22,
        prepare: 3,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "stall",
        harvest: false,
        nears: "The Street Yard Sale is {days} nights off. Attics are being emptied.",
        getting_ready: "Clearing out the garage for the yard sale.",
        told: [
            "The Street Yard Sale ran the whole length of Maple Street",
            "Maple Street held its yard sale",
            "The yard sale was one table of old records",
        ],
        said: [
            "Sold everything!",
            "Made a few bucks.",
            "Nobody bought a thing.",
        ],
    },
    Festival {
        id: "drive_in",
        name: "Drive-In Opening Night",
        day: 30,
        prepare: 4,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "lantern",
        harvest: false,
        nears: "The drive-in opens in {days} nights.",
        getting_ready: "Drive-in opens soon. Who's got a car?",
        told: [
            "Every car in town was at the drive-in's opening night",
            "The drive-in opened for the summer",
            "The drive-in opened to an empty lot",
        ],
        said: ["What a picture!", "Good movie.", "Watched it alone."],
    },
    Festival {
        id: "fireflies",
        name: "the fireflies",
        day: 38,
        prepare: 0,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The fireflies came out over the ballfield",
            "The fireflies came out over the ballfield",
            "The fireflies came out over the ballfield",
        ],
        said: [
            "Fireflies! Summer's really here.",
            "Fireflies! Summer's really here.",
            "Fireflies! Summer's really here.",
        ],
    },
    Festival {
        id: "block_party",
        name: "the Block Party",
        day: 46,
        prepare: 7,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "bunting",
        harvest: false,
        nears: "The Block Party is {days} nights away. The grills are coming out.",
        getting_ready: "A week to the Block Party. I'm on burgers.",
        told: [
            "The Block Party had the whole street out till dawn",
            "Maple Street threw its Block Party",
            "The Block Party was a grill and a few neighbours",
        ],
        said: [
            "Everybody came!",
            "Good party.",
            "More burgers than people.",
        ],
    },
    Festival {
        id: "fireworks",
        name: "the Fireworks",
        day: 54,
        prepare: 4,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "flag",
        harvest: false,
        nears: "The Fireworks are {days} nights off.",
        getting_ready: "Fireworks soon. Best spot is the water tower.",
        told: [
            "The Fireworks lit up the whole sky over Maple Street",
            "The town watched the Fireworks",
            "The Fireworks fizzled in the rain",
        ],
        said: ["Wow. Just wow.", "Pretty show.", "Two rockets and a dud."],
    },
    Festival {
        id: "skate_night",
        name: "the Roller Rink Night",
        day: 62,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "lantern",
        harvest: false,
        nears: "Roller Rink Night is {days} nights off. Dust off your skates.",
        getting_ready: "Rink Night soon. I've been practising backwards.",
        told: [
            "The whole street skated till the rink lights went off",
            "Maple Street went skating at the rink",
            "Roller Rink Night was mostly falling over",
        ],
        said: [
            "Went backwards and didn't fall once!",
            "Fun. My ankles hurt.",
            "Fell over. Twice. On purpose.",
        ],
    },
    Festival {
        id: "pie_contest",
        name: "the County Fair Pie Contest",
        day: 70,
        prepare: 7,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "stall",
        harvest: true,
        nears: "The Pie Contest is {days} nights away. Everyone's picking fruit.",
        getting_ready: "A week to the Pie Contest. My secret is the crust.",
        told: [
            "The Pie Contest had more entries than ever",
            "The Pie Contest had a fair few entries",
            "The Pie Contest had barely any fruit to bake",
        ],
        said: ["Blue ribbon!", "Tasty year.", "Nothing to bake with."],
    },
    Festival {
        id: "homecoming",
        name: "the Homecoming Game",
        day: 76,
        prepare: 5,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "flag",
        harvest: false,
        nears: "The Homecoming Game is {days} nights off.",
        getting_ready: "Homecoming soon. Go Comets!",
        told: [
            "The Homecoming Game packed the stands",
            "The town turned out for Homecoming",
            "The Homecoming Game played to empty stands",
        ],
        said: ["We won! We won!", "Good game.", "Nobody came to cheer."],
    },
    Festival {
        id: "halloween",
        name: "Halloween",
        day: 86,
        prepare: 5,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "lantern",
        harvest: false,
        nears: "Halloween is {days} nights off. Costumes are being sewn.",
        getting_ready: "Carving pumpkins for Halloween.",
        told: [
            "Halloween had every kid on Maple Street out trick-or-treating",
            "Maple Street had its Halloween",
            "Halloween was a few kids and a lot of candy left over",
        ],
        said: [
            "Spooky and perfect!",
            "Fun night.",
            "All this candy, nobody to give it to.",
        ],
    },
    Festival {
        id: "first_snow",
        name: "the first snow",
        day: 96,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The first snow fell on Maple Street",
            "The first snow fell on Maple Street",
            "The first snow fell on Maple Street",
        ],
        said: [
            "Snow! School's gonna close!",
            "Snow! School's gonna close!",
            "Snow! School's gonna close!",
        ],
    },
    Festival {
        id: "winter_formal",
        name: "the Winter Formal",
        day: 104,
        prepare: 6,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "bunting",
        harvest: false,
        nears: "The Winter Formal is {days} nights away.",
        getting_ready: "Winter Formal soon. I need a tux.",
        told: [
            "The Winter Formal was the talk of the town",
            "The Winter Formal came and went",
            "The Winter Formal was a quiet night",
        ],
        said: ["Magic.", "Nice night.", "Nobody asked anybody."],
    },
    Festival {
        id: "countdown",
        name: "the New Year Countdown",
        day: 117,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "lantern",
        harvest: false,
        nears: "The New Year Countdown is {days} nights off.",
        getting_ready: "Countdown's coming. Arcade's staying open late.",
        told: [
            "The whole street counted down the New Year together",
            "Maple Street counted in the New Year",
            "The New Year came in quietly",
        ],
        said: [
            "Happy New Year, everybody!",
            "Happy New Year.",
            "Another year, I guess.",
        ],
    },
];

pub(crate) const ICE: &[Festival] = &[
    Festival {
        id: "sun_return",
        name: "the Sun's Return",
        day: 4,
        prepare: 4,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "flag",
        harvest: false,
        nears: "The Sun's Return is {days} auroras off. Everyone's facing east.",
        getting_ready: "The sun comes back soon. I've been waiting all night.",
        told: [
            "The whole colony greeted the Sun's Return",
            "The colony saw the sun come back",
            "The sun came back, and few were awake to see it",
        ],
        said: ["Warm at last!", "There it is.", "Hardly anyone looked up."],
    },
    Festival {
        id: "first_eggs",
        name: "the first eggs",
        day: 12,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The first eggs were laid on the warm stones",
            "The first eggs were laid on the warm stones",
            "The first eggs were laid on the warm stones",
        ],
        said: [
            "First eggs! Mind where you step.",
            "First eggs! Mind where you step.",
            "First eggs! Mind where you step.",
        ],
    },
    Festival {
        id: "fish_run",
        name: "the Fish Run",
        day: 20,
        prepare: 3,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "flag",
        harvest: false,
        nears: "The Fish Run is {days} auroras away. Everyone's practising their dive.",
        getting_ready: "Fish Run soon. I've been diving every day.",
        told: [
            "The Fish Run filled every beak",
            "The colony dived for the Fish Run",
            "The Fish Run came up nearly empty",
        ],
        said: ["So many fish!", "Good diving.", "Hardly a fish."],
    },
    Festival {
        id: "chick_parade",
        name: "the Chick Parade",
        day: 28,
        prepare: 5,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "bunting",
        harvest: false,
        nears: "The Chick Parade is {days} auroras off. The chicks are learning to waddle in line.",
        getting_ready: "Teaching the chicks to march for the parade.",
        told: [
            "The Chick Parade had every chick marching",
            "The chicks had their parade",
            "The Chick Parade was two chicks and a fall",
        ],
        said: [
            "Look at them go!",
            "Sweet little parade.",
            "They kept falling over.",
        ],
    },
    Festival {
        id: "midnight_sun",
        name: "the midnight sun",
        day: 40,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_E,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The sun stopped setting over the ice",
            "The sun stopped setting over the ice",
            "The sun stopped setting over the ice",
        ],
        said: [
            "The sun won't set now. Nobody sleeps.",
            "The sun won't set now. Nobody sleeps.",
            "The sun won't set now. Nobody sleeps.",
        ],
    },
    Festival {
        id: "slide_races",
        name: "the Slide Races",
        day: 46,
        prepare: 5,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "flag",
        harvest: false,
        nears: "The Slide Races are {days} auroras away. The ice is being polished.",
        getting_ready: "Polishing my belly for the Slide Races.",
        told: [
            "The Slide Races had the whole colony sliding",
            "The colony held its Slide Races",
            "The Slide Races were one slide and a bump",
        ],
        said: ["Fastest slide ever!", "Good fun.", "Slid alone."],
    },
    Festival {
        id: "molt",
        name: "the molt",
        day: 56,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "Everyone started to molt, feathers everywhere",
            "Everyone started to molt, feathers everywhere",
            "Everyone started to molt, feathers everywhere",
        ],
        said: [
            "Molting season. I look ridiculous.",
            "Molting season. I look ridiculous.",
            "Molting season. I look ridiculous.",
        ],
    },
    Festival {
        id: "aurora_night",
        name: "the Aurora Night",
        day: 63,
        prepare: 3,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "lantern",
        harvest: false,
        nears: "Aurora Night is {days} moonrises away. Keep your flippers warm.",
        getting_ready: "Aurora Night soon. Everyone up on the bridge.",
        told: [
            "The sky burned green over the ice for Aurora Night",
            "The colony watched the Aurora Night from the bridge",
            "Aurora Night was clouded over",
        ],
        said: [
            "Green all the way across!",
            "Pretty, and cold.",
            "Clouds. Of course.",
        ],
    },
    Festival {
        id: "krill_harvest",
        name: "the Krill Harvest",
        day: 70,
        prepare: 7,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "stall",
        harvest: true,
        nears: "The Krill Harvest is {days} auroras away. The beds are nearly ripe.",
        getting_ready: "A week to the Krill Harvest. Don't touch the beds.",
        told: [
            "The Krill Harvest filled the Fish Vault",
            "The Krill Harvest brought in enough",
            "The Krill Harvest was a few pink scraps",
        ],
        said: [
            "A feast for the winter!",
            "Enough to go round.",
            "Hardly any krill grew.",
        ],
    },
    Festival {
        id: "first_ice",
        name: "the first ice",
        day: 80,
        prepare: 0,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "Thin new ice formed along the shore",
            "Thin new ice formed along the shore",
            "Thin new ice formed along the shore",
        ],
        said: [
            "New ice on the shore. Winter's coming.",
            "New ice on the shore. Winter's coming.",
            "New ice on the shore. Winter's coming.",
        ],
    },
    Festival {
        id: "song_night",
        name: "Song Night",
        day: 86,
        prepare: 5,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "lantern",
        harvest: false,
        nears: "Song Night is {days} auroras off. Everyone's practising their call.",
        getting_ready: "Practising my call for Song Night.",
        told: [
            "Song Night rang out across the whole ice",
            "The colony sang on Song Night",
            "Song Night was a lonely call or two",
        ],
        said: ["Everyone sang!", "Lovely voices.", "Sang to myself."],
    },
    Festival {
        id: "long_night",
        name: "the long night",
        day: 96,
        prepare: 0,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "",
        harvest: false,
        nears: "",
        getting_ready: "",
        told: [
            "The long night began",
            "The long night began",
            "The long night began",
        ],
        said: [
            "The long night's here. Huddle close.",
            "The long night's here. Huddle close.",
            "The long night's here. Huddle close.",
        ],
    },
    Festival {
        id: "huddle_feast",
        name: "the Huddle Feast",
        day: 104,
        prepare: 6,
        at: SLOT_A,
        speaker: SLOT_B,
        shape: "lantern",
        harvest: false,
        nears: "The Huddle Feast is {days} auroras away. The Fish Vault is being opened.",
        getting_ready: "Opening the Fish Vault for the Huddle Feast.",
        told: [
            "The Huddle Feast warmed the whole colony",
            "The colony shared its Huddle Feast",
            "The Huddle Feast was a small, cold huddle",
        ],
        said: [
            "Warm and full, all of us!",
            "A good feast.",
            "Cold and hungry.",
        ],
    },
    Festival {
        id: "aurora",
        name: "the Aurora Festival",
        day: 116,
        prepare: 3,
        at: SLOT_C,
        speaker: SLOT_E,
        shape: "lantern",
        harvest: false,
        nears: "The Aurora Festival is {days} auroras off.",
        getting_ready: "The biggest aurora of the year is coming.",
        told: [
            "The Aurora Festival had the whole colony looking up",
            "The colony watched the Aurora Festival",
            "The Aurora Festival glowed over empty ice",
        ],
        said: [
            "The sky's on fire!",
            "Beautiful.",
            "Nobody came out to see it.",
        ],
    },
];

fn seed(state: &WorldState) -> &str {
    match state
        .entity(UNIVERSE)
        .and_then(|universe| universe.component(crate::SEED))
    {
        Some(Value::Text(seed)) => seed.as_str(),
        _ => "",
    }
}

fn relationship(state: &WorldState, key: &str) -> i64 {
    match state
        .entity(RELATIONSHIP)
        .and_then(|relationship| relationship.component(key))
    {
        Some(Value::Integer(value)) => *value,
        _ => 0,
    }
}

/// How ready the place is to celebrate: how the pair get on, and what the
/// player has made, decorations counting double.
fn festive(state: &WorldState) -> i64 {
    let mood = (relationship(state, "trust") - relationship(state, "tension")).clamp(-3, 3);
    let made = calendar::decorated(state);
    2 + mood + made
}

/// How many gardens the player planted have grown.
pub(crate) use calendar::gardens_grown as grown;

fn held(state: &WorldState, festival: &Festival, turnout: Turnout, grown: i64) -> Vec<StateChange> {
    let mut changes = Vec::new();
    let nudge = |key: &str, by: i64| StateChange::SetComponent {
        entity: RELATIONSHIP,
        key: key.into(),
        value: (relationship(state, key) + by).clamp(0, 10).into(),
    };
    if state.entity(RELATIONSHIP).is_some() {
        match turnout {
            Turnout::Grand => changes.push(nudge("trust", 1)),
            Turnout::Thin => changes.push(nudge("tension", 1)),
            Turnout::Fine => {}
        }
    }
    // What was grown feeds everyone through the lean season.
    if festival.harvest && grown > 0 {
        for who in crate::life::people_in(state) {
            if lives::enrolled(state, who) {
                changes.push(lives::lack_by(state, who, lives::Need::Money, -10 * grown));
            }
        }
    }
    changes
}

/// A place's own festivals, before its years add any.
#[cfg(test)]
pub(crate) fn festivals_of(seed: &str) -> &'static [Festival] {
    match seed {
        "mars-colony" => MARS,
        "1980s-town" => TOWN,
        "penguin-civilization" => ICE,
        _ => &[],
    }
}

pub(crate) fn almanac(state: &WorldState) -> Almanac {
    Almanac {
        notes: EntityId::new(1001),
        first: 1010,
        room: 20,
        period: crate::BACKGROUND_PERIOD,
        year: YEAR,
        seasons: ["Spring", "Summer", "Autumn", "Winter"],
        festivals: crate::years::festivals(
            state,
            match seed(state) {
                "mars-colony" => MARS,
                "1980s-town" => TOWN,
                "penguin-civilization" => ICE,
                _ => &[],
            },
        ),
        people: crate::life::people_in,
        festive,
        grown,
        held,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dated day comes round at least every two weeks, all year, in
    /// every place.
    #[test]
    fn a_dated_day_at_least_every_fourteen_periods() {
        for (place, festivals) in [("Mars", MARS), ("Maple Street", TOWN), ("Icebridge", ICE)] {
            let mut days = festivals
                .iter()
                .map(|festival| festival.day)
                .collect::<Vec<_>>();
            days.sort_unstable();
            let wrap = days[0] + YEAR - days[days.len() - 1];
            let widest = days
                .windows(2)
                .map(|pair| pair[1] - pair[0])
                .chain([wrap])
                .max()
                .unwrap();
            assert!(
                widest <= 14,
                "{place}: {widest} periods without a dated day"
            );
        }
    }
}
