//! The people of each pocket universe living their own lives: what they do
//! with a sol on Mars, a night on Maple Street or an aurora on Icebridge,
//! who they get on with, and who might come to stay. The mechanics are the
//! `lives` System's; the words are each place's own.

use crate::story::NEWCOMER;
use crate::{seed_id, SLOT_A, SLOT_B, SLOT_C, SLOT_D, SLOT_E, UNIVERSE};
use lives::{Activity, At, Cast, Need, Visitors, With};
use world_core::{EntityId, Value, World, WorldState};

/// The entity each World's lives are kept on.
pub(crate) const LIVES: EntityId = EntityId::new(38);

/// The first id a stranger who comes to stay takes.
pub(crate) const FIRST_VISITOR: u64 = 40;
/// How many strangers, and how many of the place's own newcomers, can
/// ever arrive.
pub(crate) const VISITOR_ROOM: u64 = 200;
/// The first id someone the place's own years bring takes: born there, or
/// off the shuttle.
pub(crate) const FIRST_BORN: u64 = 2000;

const MARS: &[Activity] = &[
    Activity {
        id: "maintenance",
        need: Need::Money,
        with: With::Workmate,
        at: At::Work,
        told: "{name} spent the sol on maintenance at {place} with {other}",
        said: &[
            "Scrubbers cleaned, filters swapped. {other} did the dull half.",
            "{other} and I traced a leak for three hours.",
            "Maintenance at {place}. {other} hummed the whole time.",
            "Recalibrated the sensors with {other}. Twice.",
            "{other} and I resealed the airlock. Held first time.",
        ],
        gives: &[],
    },
    Activity {
        id: "survey",
        need: Need::Money,
        with: With::Alone,
        at: At::Work,
        told: "{name} ran a survey from {place}",
        said: &[
            "Mapped another ridge from {place}.",
            "Logged forty rock samples. Forty!",
            "Survey done. The dust got in everything.",
            "Charted the crater rim. Beautiful, in a dead sort of way.",
            "Found a vein of ice under the ridge. {friend} owes me a drink.",
            "Walked the survey line out to the old beacon and back.",
            "Took readings at the dune field. The wind moved my markers.",
            "Surveyed the far slope. Nothing but rock and more rock.",
        ],
        gives: &[],
    },
    Activity {
        id: "repairs",
        need: Need::Money,
        with: With::Anyone,
        at: At::Work,
        told: "{name} fixed {other}'s kit",
        said: &[
            "Fixed {other}'s suit seal. It was held on with tape.",
            "{other} brought me a broken drill. It's a drill again now.",
            "Rewired {other}'s helmet lamp. Let there be light.",
            "Patched the rover seat for {other}. Again.",
            "{other} dropped off a cracked visor. Good as new.",
            "{other} and I fixed the winch on the rover. It only took all sol.",
            "{other} needed a hinge fixed. Took five minutes, and an hour of chat.",
        ],
        gives: &[],
    },
    Activity {
        id: "harvest",
        need: Need::Money,
        with: With::Anyone,
        at: At::Quiet,
        told: "{name} harvested wheat in {place} with {other}",
        said: &[
            "Harvested dwarf wheat with {other}. Bread next sol!",
            "{other} and I picked the beans. Mostly I ate them.",
            "Pollinated the tomatoes with {other}. With a paintbrush.",
            "{other} and I counted the new shoots. Ninety-two.",
            "{other} and I repotted the seedlings. Dirt everywhere.",
            "Trimmed the vines. The greenhouse smells like summer.",
        ],
        gives: &[],
    },
    Activity {
        id: "bunk",
        need: Need::Rest,
        with: With::Alone,
        at: At::Home,
        told: "{name} slept a full shift",
        said: &[
            "Slept a full shift. First in ages.",
            "Dreamed of rain. Real rain.",
            "Slept through the dust alarm. Oops.",
            "{friend} let me sleep in. Hero.",
            "Napped on the airlock bench. Don't tell anyone.",
            "Woke up before the alarm for once.",
            "Earplugs in, blanket up. Bliss.",
        ],
        gives: &[],
    },
    Activity {
        id: "dome_view",
        need: Need::Rest,
        with: With::Alone,
        at: At::Quiet,
        told: "{name} watched the sunset from {place}",
        said: &[
            "Blue sunset from {place}. Never gets old.",
            "Sat among the plants in {place}. It smells like Earth.",
            "Watched Phobos cross the sky.",
            "{friend} says I stare at the horizon too much.",
            "Counted the stars over the ridge. Lost count.",
            "The dust settled and the sky went pink.",
        ],
        gives: &[],
    },
    Activity {
        id: "records",
        need: Need::Rest,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} listened to old records with {other}",
        said: &[
            "{other} played me songs from home.",
            "Old records with {other}. We sang. Badly.",
            "{other} found a record I hadn't heard. Rare, out here.",
            "Lay back and let {other} pick the music.",
            "{other} knows every song on the old tapes.",
            "Turned the music up. Nobody complained.",
        ],
        gives: &[],
    },
    Activity {
        id: "supper",
        need: Need::Company,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} shared supper with {other} at {place}",
        said: &[
            "Supper with {other}. Rehydrated stew, fine company.",
            "{other} made pancakes. On Mars! Pancakes!",
            "Ate with {other} and talked about the sea.",
            "Burnt the stew. {other} ate it anyway. Hero.",
            "{other} and I shared the last of the chocolate.",
            "Long supper, longer stories.",
        ],
        gives: &[],
    },
    Activity {
        id: "chess",
        need: Need::Company,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} played chess with {other}",
        said: &[
            "{other} cheats at chess. Even on Mars.",
            "Beat {other} at chess. Finally.",
            "{other} taught me a card game from home.",
            "{other} took my queen in four moves. Humiliating.",
            "{other} and I played until the lights dimmed. A draw.",
            "Lost at chess again. I blame the gravity.",
        ],
        gives: &[],
    },
    Activity {
        id: "message_home",
        need: Need::Company,
        with: With::Alone,
        at: At::Quiet,
        told: "{name} sent a message home",
        said: &[
            "Sent a message home. Twenty minutes each way.",
            "Heard back from Earth. Everyone's well.",
            "Recorded a message for my sister. Kept it cheerful.",
            "Told Earth all about {friend}. They're jealous.",
            "Sent Earth a picture of the sunset.",
            "Waited all sol for a reply. Worth it.",
        ],
        gives: &[],
    },
    Activity {
        id: "rim_walk",
        need: Need::Company,
        with: With::Friend,
        at: At::Work,
        told: "{name} walked the crater rim with {other}",
        said: &[
            "Walked the crater rim with {other}. Suits on, still lovely.",
            "{other} and I raced to the ridge. I lost.",
            "{other} spotted a dust devil. We watched it dance.",
            "Out on the rim with {other}. Earth is a blue dot from here.",
            "{other} and I sat on the rim and said nothing for an hour.",
            "Found our old boot prints on the rim. Still there.",
        ],
        gives: &[],
    },
    Activity {
        id: "teach",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} taught {other} something new",
        said: &[
            "Taught {other} to solder. Only one burn.",
            "{other} knows the constellations now. From Mars they look odd.",
            "Showed {other} how the air recycler works.",
            "{other} asked a hundred questions. Good ones.",
            "{other} fixed the pump alone today. I taught them that!",
            "{other} learned the airlock drill. Perfect first time.",
        ],
        gives: &[],
    },
    Activity {
        id: "experiment",
        need: Need::Purpose,
        with: With::Alone,
        at: At::Quiet,
        told: "{name} ran an experiment in {place}",
        said: &[
            "Grew a tomato in Martian soil. Tiny, but a tomato.",
            "My experiment worked. Mostly.",
            "Tested a new water filter. {friend} tasted it. Brave.",
            "The algae tank turned purple. Science!",
            "Measured the soil again. Still Martian.",
        ],
        gives: &[],
    },
    Activity {
        id: "log",
        need: Need::Purpose,
        with: With::Alone,
        at: At::Home,
        told: "{name} wrote up the colony log",
        said: &[
            "Wrote up the log. Future Martians, hello.",
            "Drew a map of everything we've found.",
            "Wrote a poem about dust. It's not good.",
            "Filed the sol report. Nothing blew up.",
            "Updated the map of the caves.",
        ],
        gives: &[],
    },
    Activity {
        id: "telescope",
        need: Need::Purpose,
        with: With::Friend,
        at: At::Work,
        told: "{name} worked on a telescope with {other}",
        said: &[
            "{other} and I are building a telescope.",
            "Designing a greenhouse extension with {other}.",
            "{other} and I aligned the mirror. Nearly.",
            "Sketched plans with {other} till the lights dimmed.",
            "{other} and I ground the lens a little finer.",
            "The telescope's nearly done. One more screw.",
        ],
        gives: &[],
    },
];

const TOWN: &[Activity] = &[
    Activity {
        id: "shift",
        need: Need::Money,
        with: With::Workmate,
        at: At::Work,
        told: "{name} worked a shift at {place} with {other}",
        said: &[
            "Shift at {place}. {other} spilled a milkshake on the till.",
            "{other} and I restocked everything at {place}.",
            "Busy night at {place}. {other} ran the jukebox all night.",
            "Worked with {other}. We got the ice machine going.",
        ],
        gives: &[],
    },
    Activity {
        id: "paper_round",
        need: Need::Money,
        with: With::Alone,
        at: At::Work,
        told: "{name} did an early paper round",
        said: &[
            "Paper round done before six. Freezing.",
            "A dog chased me down Elm Street. Again.",
            "Delivered two hundred papers. My arms!",
            "Tips from Mrs Kowalski. A whole dollar.",
            "Threw a paper on the Hendersons' roof. Oops.",
            "Paper round in the rain. Every paper soaked.",
            "Saw {friend}'s light on at five. Night owl.",
        ],
        gives: &[],
    },
    Activity {
        id: "lawns",
        need: Need::Money,
        with: With::Anyone,
        at: At::Work,
        told: "{name} mowed {other}'s lawn for cash",
        said: &[
            "Mowed {other}'s lawn. Five bucks.",
            "Washed {other}'s car. Found a quarter in the seat.",
            "Painted {other}'s fence. And my shoes.",
            "{other} paid me in pie. Worth it.",
        ],
        gives: &[],
    },
    Activity {
        id: "yard_sale",
        need: Need::Money,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} ran a yard sale with {other}",
        said: &[
            "Sold my old records. {other} bought the Springsteen.",
            "Yard sale with {other}. Made eleven dollars.",
            "{other} haggled me down on my own skateboard.",
            "Sold {other} a lava lamp. It works, mostly.",
        ],
        gives: &[],
    },
    Activity {
        id: "sleep_in",
        need: Need::Rest,
        with: With::Alone,
        at: At::Home,
        told: "{name} slept till noon",
        said: &[
            "Slept till noon. Mom wasn't impressed.",
            "Didn't get up till the soaps came on.",
            "Lay in bed with the radio on.",
            "{friend} called. I pretended I was out.",
        ],
        gives: &[],
    },
    Activity {
        id: "tv",
        need: Need::Rest,
        with: With::Alone,
        at: At::Home,
        told: "{name} watched TV all evening",
        said: &[
            "Watched reruns all night. Bliss.",
            "Taped the Top 40 off the radio.",
            "Fell asleep in front of the late movie.",
            "Caught the end of the ball game. We lost.",
            "Watched a scary movie with the lights off. Regret.",
            "MTV all evening. My brain is neon.",
        ],
        gives: &[],
    },
    Activity {
        id: "drive",
        need: Need::Rest,
        with: With::Friend,
        at: At::Quiet,
        told: "{name} drove around town with {other}",
        said: &[
            "Drove around with {other} and the windows down.",
            "{other} and I drove out to the lake.",
            "{other} drove. I did the tape deck.",
            "Parked by the water tower with {other} and talked.",
        ],
        gives: &[],
    },
    Activity {
        id: "arcade",
        need: Need::Company,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} played Pac-Man with {other} at {place}",
        said: &[
            "{other} beat my high score. I'm devastated.",
            "Two-player with {other} till closing.",
            "{other} spent all their quarters on Galaga.",
            "{other} and I beat the last level of Donkey Kong!",
        ],
        gives: &[],
    },
    Activity {
        id: "diner",
        need: Need::Company,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} got fries with {other}",
        said: &[
            "Fries and shakes with {other}.",
            "{other} and I split a banana split.",
            "{other} dared me to eat the Big Burger. I lost.",
            "Coffee with {other} till the waitress glared.",
        ],
        gives: &[],
    },
    Activity {
        id: "phone",
        need: Need::Company,
        with: With::Alone,
        at: At::Home,
        told: "{name} was on the phone all night",
        said: &[
            "On the phone for two hours. The cord's stretched.",
            "My sister called long-distance. Mom timed it.",
            "Called {friend}. We talked about nothing for an hour.",
            "Called the radio station. They played my song!",
        ],
        gives: &[],
    },
    Activity {
        id: "rink",
        need: Need::Company,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} went roller skating with {other}",
        said: &[
            "Roller rink with {other}. I fell over twice.",
            "{other} can moonwalk. Who knew?",
            "{other} held my hand on the rink. For balance.",
            "Couples skate with {other}. Don't tell anyone.",
        ],
        gives: &[],
    },
    Activity {
        id: "radio_show",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Quiet,
        told: "{name} helped {other} run a show at {place}",
        said: &[
            "Helped {other} run the late show at {place}.",
            "{other} let me pick the records on air!",
            "{other} and I took phone-in requests till two.",
            "Read the weather on air with {other}. Rain, again.",
        ],
        gives: &[],
    },
    Activity {
        id: "band",
        need: Need::Purpose,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} practised with {other}'s garage band",
        said: &[
            "Band practice with {other}. The neighbours complained.",
            "{other} and I wrote a song. It's got four chords.",
            "{other} broke a string. The show went on.",
            "Garage band with {other}. The dog howled along.",
        ],
        gives: &[],
    },
    Activity {
        id: "study",
        need: Need::Purpose,
        with: With::Alone,
        at: At::Home,
        told: "{name} studied for exams",
        said: &[
            "Studied all evening. Brain full.",
            "Flashcards till midnight.",
            "{friend} quizzed me on history. I know the Magna Carta now.",
            "Wrote an essay on Moby Dick. Never again.",
            "Fell asleep on my maths book.",
        ],
        gives: &[],
    },
    Activity {
        id: "old_car",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Work,
        told: "{name} fixed up an old car with {other}",
        said: &[
            "Got {other}'s old Chevy running. Sort of.",
            "Oil to the elbows. {other} says it's a good look.",
            "{other} and I put new tyres on the old Ford.",
            "The car started! {other} cheered.",
        ],
        gives: &[],
    },
];

const ICE: &[Activity] = &[
    Activity {
        id: "fishing",
        need: Need::Money,
        with: With::Workmate,
        at: At::Work,
        told: "{name} went fishing with {other}",
        said: &[
            "Caught three herring. {other} caught one. Ha.",
            "Fishing with {other}. Cold. Wet. Wonderful.",
            "{other} and I filled the basket by noon.",
            "A seal chased us off the good spot. {other} was brave.",
        ],
        gives: &[],
    },
    Activity {
        id: "bridge_work",
        need: Need::Money,
        with: With::Alone,
        at: At::Work,
        told: "{name} mended the bridge",
        said: &[
            "Patched the bridge. Two new spans of ice.",
            "Mended a crack in the bridge. Nobody fell in.",
            "Carved new steps in the ice.",
            "Swept snow off the bridge all morning.",
            "{friend} held the ice while I carved. Teamwork.",
            "Chipped the old ice off the rails. Shiny now.",
        ],
        gives: &[],
    },
    Activity {
        id: "lanterns",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} lit the lanterns with {other}",
        said: &[
            "Lit the lanterns with {other}. The ice glowed.",
            "{other} and I trimmed every wick on the bridge.",
            "{other} dropped a lantern. It bounced! Ice is kind.",
            "Hung a new lantern with {other} at the far end.",
        ],
        gives: &[],
    },
    Activity {
        id: "sort_fish",
        need: Need::Money,
        with: With::Anyone,
        at: At::Quiet,
        told: "{name} sorted fish at {place} with {other}",
        said: &[
            "Sorted the fish with {other}. Herring left, cod right.",
            "{other} and I counted the fish store. Twice.",
            "{other} ate one while we counted. I saw.",
            "Found a starfish in the fish pile. Put it back.",
        ],
        gives: &[],
    },
    Activity {
        id: "trade",
        need: Need::Money,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} swapped stones with {other}",
        said: &[
            "Swapped a smooth stone with {other}. Good trade.",
            "{other} gave me a shiny pebble for two fish.",
            "{other} drives a hard bargain for a pebble.",
            "Traded {other} a feather for a smooth stone.",
        ],
        gives: &[],
    },
    Activity {
        id: "snowdrift",
        need: Need::Rest,
        with: With::Alone,
        at: At::Home,
        told: "{name} napped in a snowdrift",
        said: &[
            "Napped in a snowdrift. Perfect.",
            "Slept standing up. A skill.",
            "Dreamed of krill. Endless krill.",
            "{friend} kept watch while I slept.",
        ],
        gives: &[],
    },
    Activity {
        id: "huddle",
        need: Need::Rest,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} huddled with {other} out of the wind",
        said: &[
            "Huddled with {other}. Warm at last.",
            "{other} and I stood in the middle of the huddle. Cosy.",
            "Warmest spot in the huddle, next to {other}.",
            "{other} told jokes in the huddle. Terrible ones.",
        ],
        gives: &[],
    },
    Activity {
        id: "aurora",
        need: Need::Rest,
        with: With::Alone,
        at: At::Quiet,
        told: "{name} watched the aurora",
        said: &[
            "Watched the aurora dance. Green and pink.",
            "The sky was on fire with lights.",
            "Counted the stars till I lost count.",
            "The aurora went purple tonight. Never seen that.",
            "Lay on the ice and watched the lights.",
        ],
        gives: &[],
    },
    Activity {
        id: "slide",
        need: Need::Company,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} went belly-sliding with {other}",
        said: &[
            "Belly-slid down the big slope with {other}!",
            "{other} slid further than me. Rematch tomorrow.",
            "{other} and I slid right into a snowbank.",
            "{other} and I made a new slide. Very fast.",
        ],
        gives: &[],
    },
    Activity {
        id: "song",
        need: Need::Company,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} sang with {other}",
        said: &[
            "Sang the old songs with {other}.",
            "{other} knows every verse of the fish song.",
            "{other} and I made up a new verse.",
            "Sang to the moon with {other}.",
        ],
        gives: &[],
    },
    Activity {
        id: "preen",
        need: Need::Company,
        with: With::Friend,
        at: At::Home,
        told: "{name} preened {other}'s feathers",
        said: &[
            "Helped {other} with their feathers.",
            "{other} said my feathers look splendid.",
            "{other} found a fish scale in my feathers. Embarrassing.",
            "Preened {other}. They preened me back.",
        ],
        gives: &[],
    },
    Activity {
        id: "gossip",
        need: Need::Company,
        with: With::Anyone,
        at: At::Quiet,
        told: "{name} gossiped with {other} by {place}",
        said: &[
            "{other} told me all the colony news.",
            "Chatted with {other} till the tide turned.",
            "{other} heard the seal is back. Oh dear.",
            "{other} and I agreed the council talks too much.",
        ],
        gives: &[],
    },
    Activity {
        id: "diving",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Quiet,
        told: "{name} taught {other} to dive",
        said: &[
            "Taught {other} to dive. Splash!",
            "{other} can catch fish by themselves now.",
            "{other} dove deeper than ever today.",
            "Swam under the bridge with {other}.",
        ],
        gives: &[],
    },
    Activity {
        id: "nest",
        need: Need::Purpose,
        with: With::Alone,
        at: At::Home,
        told: "{name} built a stone nest",
        said: &[
            "Built the finest stone nest on the ice.",
            "Found the perfect pebble. Perfect.",
            "{friend} admired my nest. As they should.",
            "Tucked a feather in the nest. Cosy.",
        ],
        gives: &[],
    },
    Activity {
        id: "tales",
        need: Need::Purpose,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} told the old tales with {other}",
        said: &[
            "{other} and I told the little ones about the old colony.",
            "Told the tale of the great storm with {other}.",
            "{other} told the one about the whale. Everyone gasped.",
            "The little ones fell asleep before {other} finished.",
        ],
        gives: &[],
    },
    Activity {
        id: "council",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Work,
        told: "{name} spoke up at the council with {other}",
        said: &[
            "Spoke at the council with {other}. They listened!",
            "{other} and I proposed a new fishing ground.",
            "{other} and I won the vote about the new hole.",
            "The council took my idea! {other} backed me.",
        ],
        gives: &[],
    },
];

fn traits(person: EntityId) -> Option<[&'static str; 2]> {
    Some(match person {
        SLOT_B => ["steady", "warm"],
        SLOT_E => ["restless", "sociable"],
        NEWCOMER => ["shy", "dreamy"],
        // Whoever comes to stay has two traits of their own, turn and turn
        // about, so no two arrivals in a row are alike and nobody is left
        // to speak in the words everyone without traits would share.
        other => {
            let n = other.0 as usize;
            let first = lives::TRAITS[n % lives::TRAITS.len()];
            let mut second = lives::TRAITS[(n * 3 + 5) % lives::TRAITS.len()];
            if second == first {
                second = lives::TRAITS[(n + 1) % lives::TRAITS.len()];
            }
            [first, second]
        }
    })
}

/// How the place feels: how the pair get on, trust against tension.
fn mood(state: &WorldState) -> i64 {
    let value = |key: &str| match state
        .entity(crate::RELATIONSHIP)
        .and_then(|relationship| relationship.component(key))
    {
        Some(Value::Integer(value)) => *value,
        _ => 0,
    };
    value("trust") - value("tension")
}

/// The pair's standing with each other is Pocket Universe's own story.
fn kept(a: EntityId, b: EntityId) -> bool {
    (a == SLOT_B && b == SLOT_E) || (a == SLOT_E && b == SLOT_B)
}

fn stays(person: EntityId) -> bool {
    person == SLOT_B || person == SLOT_E
}

/// Where someone works: the keeper at home, the explorer at what lets them
/// range out, anyone else where they were taken in.
pub(crate) fn work(state: &WorldState, person: EntityId) -> Option<EntityId> {
    let seed = match state
        .entity(UNIVERSE)
        .and_then(|u| u.component(crate::SEED))
    {
        Some(Value::Text(seed)) => seed.as_str(),
        _ => "",
    };
    match person {
        SLOT_B => Some(SLOT_A),
        SLOT_E if seed == "mars-colony" => Some(SLOT_D),
        SLOT_E => Some(SLOT_C),
        _ => match state.entity(person)?.component("works_at") {
            Some(Value::Entity(place)) => Some(*place),
            _ => match state.entity(person)?.component("location")? {
                Value::Entity(place) => Some(*place),
                _ => None,
            },
        },
    }
}

fn home(_: &WorldState, _: EntityId) -> Option<EntityId> {
    None
}

pub(crate) fn visitor(_: &str, job: &str) -> Vec<(String, Value)> {
    vec![
        ("role".into(), Value::from(job)),
        ("location".into(), Value::Entity(SLOT_A)),
        ("works_at".into(), Value::Entity(SLOT_C)),
        ("newcomer".into(), Value::from(true)),
    ]
}

/// The most people the place has room for: some to begin with, and more
/// as its works go up.
pub(crate) fn most_people(state: &WorldState) -> usize {
    let base = match crate::places::Place::of(state) {
        Some(crate::places::Place::Ares) => 8,
        Some(crate::places::Place::Maple) => 9,
        _ => 10,
    };
    (base + crate::story::works_finished(state).max(0) as usize / 6).min(12)
}

/// Everyone living in the World now: the pair, whoever came to stay, less
/// anyone who has left.
pub(crate) fn people(world: &World) -> Vec<EntityId> {
    people_in(world.state())
}

/// Everyone living there now, read straight from the state.
pub(crate) fn people_in(state: &WorldState) -> Vec<EntityId> {
    let grown_here = crate::places::Place::of(state)
        .map(|place| {
            (crate::kin::FIRST_CHILD..crate::kin::FIRST_CHILD + crate::kin::of(place).room)
                .map(EntityId::new)
                .take_while(|id| state.entity(*id).is_some())
                .filter(|id| lives::generations::of_age(state, *id))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    [SLOT_B, SLOT_E, NEWCOMER]
        .into_iter()
        .filter(|id| state.entity(*id).is_some())
        .chain(newcomers(state))
        .chain(grown_here)
        .filter(|id| !lives::gone(state, *id))
        .collect()
}

/// Everyone who ever came to live here after the place began, gone or
/// not, first come first: strangers the place took in, and those the
/// place's own years brought (born, or off the shuttle).
pub(crate) fn newcomers(state: &WorldState) -> Vec<EntityId> {
    let taken = |id: u64| {
        (FIRST_VISITOR..FIRST_VISITOR + VISITOR_ROOM).contains(&id)
            || (FIRST_BORN..FIRST_BORN + VISITOR_ROOM).contains(&id)
    };
    state
        .entities()
        .map(|entity| entity.id)
        .filter(|id| taken(id.0))
        .collect()
}

/// Maple Street's newcomers' names.
const TOWN_NAMES: &[&str] = &[
    "Donna", "Ricky", "Tanya", "Walt", "Keisha", "Eddie", "Joanie", "Mikey", "Carla", "Dwayne",
    "Stacy", "Kevin", "Brenda", "Tony", "Lisa", "Marcus", "Wendy", "Rodney", "Tina", "Duane",
    "Sheila", "Jimmy", "Denise", "Troy", "Angie", "Curtis", "Patty", "Lamar", "Heather", "Vince",
    "Rosa", "Gary", "Yolanda", "Scott", "Debbie", "Andre", "Kim", "Terry", "Shawn", "Nicole",
];

/// The penguins' newcomers' names.
const ICE_NAMES: &[&str] = &[
    "Pip", "Olo", "Nessa", "Brr", "Kiki", "Umi", "Flo", "Wob", "Tiki", "Snow", "Nib", "Tolo",
    "Suki", "Frost", "Puff", "Dot", "Ploo", "Yuki", "Bibi", "Momo", "Taku", "Lumi", "Kip", "Noa",
    "Skip", "Fizz", "Oona", "Bo", "Tuft", "Minnow", "Sleet", "Ripple", "Squall", "Drift", "Pebble",
    "Hush", "Wren", "Nuk", "Tay", "Zuzu",
];

/// The colony's newcomers' names.
const MARS_NAMES: &[&str] = &[
    "Yusuf Adeyemi",
    "Sasha Petrov",
    "Lin Mei",
    "Oskar Holm",
    "Priya Raman",
    "Dmitri Sokol",
    "Amara Obi",
    "Freya Lund",
    "Kofi Mensah",
    "Elena Ruiz",
    "Hiro Tanaka",
    "Nadia Haddad",
    "Ravi Kapoor",
    "Greta Nilsson",
    "Tariq Bashir",
    "Chloe Martin",
    "Mateo Silva",
    "Ada Nwosu",
    "Jonas Berg",
    "Leila Karimi",
    "Pavel Novak",
    "Sun Li",
    "Marisol Vega",
    "Ibrahim Diallo",
    "Hanna Virtanen",
    "Kai Keawe",
    "Zofia Nowak",
    "Arjun Mehta",
    "Ingrid Dahl",
    "Tomasz Kral",
    "Wanjiru Kamau",
    "Felix Braun",
    "Rosa Delgado",
    "Emeka Okafor",
    "Mira Stein",
    "Diego Castro",
    "Aiko Mori",
    "Omar Farouk",
];

pub(crate) fn cast(state: &WorldState) -> Cast {
    let seed = match state
        .entity(UNIVERSE)
        .and_then(|u| u.component(crate::SEED))
    {
        Some(Value::Text(seed)) => seed.clone(),
        _ => String::new(),
    };
    let (activities, topics, outings, unit, settlement, short_of, visitors): (
        &'static [Activity],
        &'static [&'static str],
        &'static [&'static str],
        &'static str,
        &'static str,
        &'static str,
        Visitors,
    ) = match seed.as_str() {
        "1980s-town" => (
            TOWN,
            &[
                "a borrowed cassette",
                "the high score",
                "who called who",
                "the prom",
                "a dent in the car",
                "the last slice of pizza",
                "gossip at the diner",
                "a mixtape",
                "the curfew",
                "the band's name",
                "a lost bet",
                "money for gas",
                "a party nobody mentioned",
                "the radio playlist",
                "a broken Walkman",
                "the school dance",
            ],
            &[
                "the drive-in",
                "the roller rink",
                "the lake on Saturday",
                "the school dance",
                "the arcade after hours",
                "a movie at the Rialto",
            ],
            "week",
            "street",
            "money",
            Visitors {
                first: FIRST_VISITOR,
                room: VISITOR_ROOM,
                names: TOWN_NAMES,
                trades: &[
                    ("mechanic", "mechanic"),
                    ("diner cook", "diner cook"),
                    ("guitarist", "guitarist"),
                    ("substitute teacher", "substitute teacher"),
                    ("paper-route kid", "paper-route kid"),
                ],
                origins: &[
                    "Ohio",
                    "the next town over",
                    "the city",
                    "a band on tour",
                    "the army base",
                ],
                way_out: "the Greyhound",
                components: visitor,
                kind: "person",
            },
        ),
        "penguin-civilization" => (
            ICE,
            &[
                "the best fishing hole",
                "a stolen pebble",
                "whose turn to keep watch",
                "a squawk at the council",
                "the last herring",
                "a snowball",
                "the huddle order",
                "a nest too close",
                "the slide rules",
                "a cold shoulder",
                "the fish count",
                "who saw the seal first",
                "a broken bridge span",
                "the aurora vote",
                "a lost feather",
                "the song verses",
            ],
            &[
                "watch the aurora",
                "slide down the big slope",
                "fish at the far hole",
                "walk to the edge of the floe",
                "swim under the ice",
                "sit on the tallest berg",
            ],
            "moon",
            "colony",
            "fish",
            Visitors {
                first: FIRST_VISITOR,
                room: VISITOR_ROOM,
                names: ICE_NAMES,
                trades: &[
                    ("fisher", "fisher"),
                    ("ice carver", "ice carver"),
                    ("storyteller", "storyteller"),
                    ("lantern tender", "lantern tender"),
                    ("scout", "scout"),
                ],
                origins: &[
                    "the far floe",
                    "the south shelf",
                    "a drifting berg",
                    "the whale road",
                    "the old colony",
                ],
                way_out: "the next ice floe",
                components: visitor,
                kind: "penguin",
            },
        ),
        _ => (
            MARS,
            &[
                "the water ration",
                "who left the airlock open",
                "the rover schedule",
                "the last of the coffee",
                "a missed check-in",
                "music in the common room",
                "the night watch",
                "a broken promise",
                "the greenhouse temperature",
                "a joke that went too far",
                "the message to Earth",
                "dust in the quarters",
                "a borrowed tool",
                "the chore rota",
                "the last chocolate bar",
                "the rover's mileage",
            ],
            &[
                "watch the sunset from the ridge",
                "a walk to the crater rim",
                "supper under the dome",
                "stargaze from the rover roof",
                "see the dust devils dance",
                "a picnic in the greenhouse",
            ],
            "sol",
            "colony",
            "supplies",
            Visitors {
                first: FIRST_VISITOR,
                room: VISITOR_ROOM,
                names: MARS_NAMES,
                trades: &[
                    ("geologist", "geologist"),
                    ("medic", "medic"),
                    ("engineer", "engineer"),
                    ("botanist", "botanist"),
                    ("pilot", "pilot"),
                ],
                origins: &[
                    "the orbital station",
                    "Phobos",
                    "the Tharsis outpost",
                    "the last supply ship",
                    "Earth",
                ],
                way_out: "the supply shuttle",
                components: visitor,
                kind: "person",
            },
        ),
    };
    // Each of the place's own years brings its own: what people fall out
    // over and go out to, who comes asking and from where, and what they
    // do with their days.
    let (activities, topics, outings, visitors, short_of) = match crate::places::Place::of(state) {
        Some(place) => {
            let now = crate::places::period(state);
            let year = in_year(place, now, activities, topics, outings, visitors);
            // What someone runs short of, and how someone leaves, as people
            // say it these weeks.
            let (short, ways) = words_of(place);
            let turn = (now / 60) as usize;
            (
                year.activities,
                year.topics,
                year.outings,
                Visitors {
                    trades: year.trades,
                    origins: year.origins,
                    way_out: if now < 60 {
                        visitors.way_out
                    } else {
                        ways[turn % ways.len()]
                    },
                    ..visitors
                },
                if now < 60 {
                    short_of
                } else {
                    short[turn % short.len()]
                },
            )
        }
        None => (activities, topics, outings, visitors, short_of),
    };
    Cast {
        notes: LIVES,
        period: crate::BACKGROUND_PERIOD,
        unit,
        settlement,
        short_of,
        people,
        stays,
        traits,
        kept,
        mood,
        work,
        home,
        gathering: SLOT_A,
        quiet: SLOT_C,
        host: SLOT_B,
        activities,
        topics,
        outings,
        fund: Some(lives::Fund {
            entity: UNIVERSE,
            key: crate::years::STORES,
            name: match seed.as_str() {
                "1980s-town" => "the street fund",
                "penguin-civilization" => "the fish vault",
                _ => "the stores",
            },
            amount: 10,
        }),
        visitors: Some(visitors),
        // While something the player built waits to draw someone, a room
        // is kept for them: no stranger is taken in meanwhile.
        most_people: most_people(state).saturating_sub(usize::from(crate::plots::waiting(state))),
        most_open: 2,
        voice: match seed.as_str() {
            "1980s-town" => crate::voices::town,
            "penguin-civilization" => crate::voices::ice,
            _ => crate::voices::mars,
        },
        kin: crate::places::Place::of(state).map(crate::kin::of),
        kind: crate::arrival::first_days,
    }
}

/// What someone in each place runs short of, and the ways out of it, as
/// they are said from one season to the next.
fn words_of(place: crate::places::Place) -> (&'static [&'static str], &'static [&'static str]) {
    match place {
        crate::places::Place::Ares => (
            &[
                "supplies",
                "rations",
                "spare parts",
                "battery charge",
                "clean water",
                "air filters",
            ],
            &[
                "the supply shuttle",
                "the next shuttle",
                "the Tharsis convoy",
                "the return flight",
                "the trade rover",
                "the orbital hop",
            ],
        ),
        crate::places::Place::Maple => (
            &[
                "money",
                "quarters",
                "gas money",
                "rent money",
                "lunch money",
                "bus fare",
            ],
            &[
                "the Greyhound",
                "the night train",
                "Night Bus 6 to the city",
                "the interstate",
                "a friend's van",
                "the morning train",
            ],
        ),
        crate::places::Place::Ice => (
            &[
                "fish",
                "herring",
                "krill",
                "sprats",
                "squid",
                "good fishing",
            ],
            &[
                "the next ice floe",
                "the migrating flocks",
                "the whale road",
                "a drifting berg",
                "the south current",
                "the thaw channel",
            ],
        ),
    }
}

/// What a place's people draw on in one of its own years.
struct InYear {
    activities: &'static [Activity],
    topics: &'static [&'static str],
    outings: &'static [&'static str],
    trades: &'static [(&'static str, &'static str)],
    origins: &'static [&'static str],
}

/// What a place's people draw on in its `year`th own year: the first year
/// what the place always had; after it, that year's own, with the year
/// before's still about and half of what the place always did, in turn.
fn in_year(
    place: crate::places::Place,
    now: u64,
    activities: &'static [Activity],
    topics: &'static [&'static str],
    outings: &'static [&'static str],
    visitors: Visitors,
) -> &'static InYear {
    use std::collections::BTreeMap;
    use std::sync::{Mutex, OnceLock};
    type Years = BTreeMap<(crate::places::Place, u64, u64), &'static InYear>;
    static YEARS: OnceLock<Mutex<Years>> = OnceLock::new();
    let year = crate::places::era_at(place, now);
    // Which month of its own year it is: what people fall out over and go
    // out to changes month by month.
    let month = crate::places::era_began(place, now).map_or(0, |began| (now - began) / 30);
    let key = (
        place,
        crate::eras::era(place, year).map_or(0, |_| {
            (year - 1) % crate::eras::eras(place).len() as u64 + 1
        }),
        month,
    );
    let mut years = YEARS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    years.entry(key).or_insert_with(|| {
        let leak = |all: Vec<&'static str>| -> &'static [&'static str] {
            Box::leak(all.into_boxed_slice())
        };
        let made = match crate::eras::era(place, key.1) {
            None => InYear {
                activities,
                topics,
                outings,
                trades: visitors.trades,
                origins: visitors.origins,
            },
            Some(this) => {
                let before = crate::eras::era(place, key.1 - 1);
                // This year's own, last year's, and everything the place
                // always did.
                let mut acts = this.activities.to_vec();
                acts.extend(
                    before
                        .map_or(&[][..], |before| before.activities)
                        .iter()
                        .copied(),
                );
                acts.extend(activities.iter().copied());
                let mut outs = this.outings.to_vec();
                outs.extend(
                    before
                        .map_or(&[][..], |before| before.outings)
                        .iter()
                        .copied(),
                );
                let mut trades = this.trades.to_vec();
                trades.extend(visitors.trades.iter().copied());
                let mut origins = this.origins.to_vec();
                origins.extend(visitors.origins.iter().copied());
                // Five of the year's topics at a time, moving on a month
                // at a time.
                let window = |all: &[&'static str], size: usize| {
                    (0..size.min(all.len()))
                        .map(|at| all[(key.2 as usize * 2 + at) % all.len()])
                        .collect::<Vec<_>>()
                };
                InYear {
                    activities: Box::leak(acts.into_boxed_slice()),
                    topics: leak(window(this.topics, 5)),
                    outings: leak(window(&outs, 3)),
                    trades: Box::leak(trades.into_boxed_slice()),
                    origins: leak(origins),
                }
            }
        };
        Box::leak(Box::new(made))
    })
}

const LIFE_COMMAND: &str = "pocket-universe.life.";

/// The situation and answer a command gives, if it is one of these.
pub(crate) fn parse_command(command_id: &str) -> Option<(&str, &str)> {
    command_id.strip_prefix(LIFE_COMMAND)?.rsplit_once('.')
}

/// A card for each answer to each situation open now.
pub(crate) fn commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    if seed_id(world) == "unseeded" {
        return Vec::new();
    }
    let cast = cast(world.state());
    let memorials = crate::kin::commands(world);
    lives::situations(world, &cast)
        .into_iter()
        .flat_map(|situation| {
            let question = world_projection::Question {
                id: format!("life.{}", situation.key),
                prompt: situation.prompt.clone(),
            };
            situation
                .answers
                .into_iter()
                .map(move |answer| world_projection::ProjectionCommand {
                    id: format!("{LIFE_COMMAND}{}.{}", situation.key, answer.id),
                    title: answer.title,
                    detail: situation.told.clone(),
                    effects: Vec::new(),
                    scenery: None,
                    asker: Some(world_projection::SelectionId::Entity(situation.asker)),
                    moves: Vec::new(),
                    question: Some(question.clone()),
                    unavailable: answer.unavailable,
                    hand: None,
                    preview: None,
                })
        })
        .chain(memorials)
        .collect()
}

/// What people have given the player to keep, oldest first, for the
/// drawer.
pub(crate) fn keepsakes(world: &World) -> Vec<world_projection::Keepsake> {
    let works = crate::story::works_done(world.state());
    lives::keepsakes(world)
        .into_iter()
        .map(|kept| world_projection::Keepsake {
            from: world_projection::SelectionId::Entity(kept.from),
            what: world
                .event(kept.event)
                .and_then(|event| harvested(world, event, &works))
                .unwrap_or(kept.what),
            note: kept.note,
            moment: world_projection::SelectionId::Event(kept.event),
        })
        .collect()
}

/// What each place's gardens give, as the people who bring it say it:
/// not any garden's basket, but the place's own.
fn produce(place: crate::places::Place) -> &'static [&'static str] {
    match place {
        crate::places::Place::Ares => &[
            "a handful of dwarf wheat",
            "a bowl of cherry tomatoes",
            "three perfect radishes",
            "a bunch of basil",
            "a bag of green beans",
            "a sprig of mint",
            "a head of lettuce",
            "a jar of pickled peppers",
            "a punnet of strawberries",
            "a bundle of chives",
            "a sack of fresh flour",
            "two small potatoes",
            "a cucumber, a bit bent",
            "a posy of marigolds",
            "a bag of snap peas",
            "a jar of greenhouse honey",
            "a tray of sprouted lentils",
            "a bunch of spring onions",
            "a handful of peanuts",
            "a bag of red lettuce",
            "a jar of sun-dried tomatoes",
            "a fistful of sweet peas",
        ],
        crate::places::Place::Maple => &[
            "a bag of tomatoes",
            "a bunch of sunflowers",
            "a zucchini the size of a bat",
            "a jar of strawberry jam",
            "a bowl of blueberries",
            "a basket of sweet corn",
            "a bunch of zinnias",
            "a pumpkin for the porch",
            "a bag of green beans",
            "a jar of dill pickles",
            "a handful of cherry tomatoes",
            "a bunch of radishes",
            "a rhubarb pie",
            "a bouquet of daisies",
            "a bag of snap peas",
            "a jar of salsa",
        ],
        crate::places::Place::Ice => &[
            "a strand of sweet kelp",
            "a bundle of sea lettuce",
            "a clump of moss for your nest",
            "a shell full of krill",
            "a sprig of snow flower",
            "a pebble with lichen on it",
            "a coil of dried kelp",
            "a handful of sea grapes",
            "a posy of tundra flowers",
            "a sprat, still flapping",
            "a scoop of fresh snow for drinking",
            "a string of sea beads",
            "a bundle of dune grass",
            "a limpet shell of salt",
            "a sweet kelp bulb",
            "a tuft of soft moss",
        ],
    }
}

/// What a garden the player planted gave, in the place's own words: the
/// next of its produce each time it gives, and what it grew in.
fn harvested(
    world: &World,
    event: &world_core::Event,
    works: &[&'static crate::works::Work],
) -> Option<String> {
    if event.kind != "enjoyed"
        || event.payload.get("effect") != Some(&Value::Text("harvest".into()))
    {
        return None;
    }
    let place = crate::places::Place::of(world.state())?;
    let state = world.state();
    let thing = *event.targets.first()?;
    state.entity(thing)?;
    let what = lives::name(state, thing).to_lowercase();
    let produce = produce(place);
    let turn = event.world_time / crate::BACKGROUND_PERIOD / 7 + thing.0;
    let produce = produce[(turn % produce.len() as u64) as usize];
    // Grown in the shelter of the latest work the place had finished when
    // the year it was grown in began, if any: fixed once that year began,
    // so a keepsake always reads as it did when it was given.
    let year = crate::places::era_at(place, event.world_time / crate::BACKGROUND_PERIOD);
    let finished = crate::years::works_at_year(state, year).unwrap_or(0);
    Some(
        match finished
            .checked_sub(1)
            .and_then(|last| works.get(last).copied())
        {
            Some(work) => format!(
                "{produce} from your {what} by {}",
                crate::story::the_work(work)
            ),
            None => format!("{produce} from your {what}"),
        },
    )
}

/// Letters the player has been written, oldest first, for the letter box.
pub(crate) fn letters(world: &World) -> Vec<world_projection::Letter> {
    lives::letters(world)
        .into_iter()
        .map(|letter| world_projection::Letter {
            from: world_projection::SelectionId::Entity(letter.from),
            note: letter.note,
            moment: world_projection::SelectionId::Event(letter.event),
        })
        .collect()
}

/// What the player can suggest the place does together.
pub(crate) const SUGGEST_COMMAND: &str = "pocket-universe.suggest.";

/// Whether the weather is fair enough to be out in.
pub(crate) fn fair(world: &World) -> bool {
    use world_projection::Weather;
    !matches!(
        crate::story::weather(world),
        Weather::Rain | Weather::Storm | Weather::Snow | Weather::Dust
    )
}

/// A picnic, a market or a dance: the player suggests it, and the place
/// decides who comes and how it goes.
pub(crate) fn suggestions(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let state = world.state();
    if crate::seed_id(world) == crate::UNSEEDED {
        return Vec::new();
    }
    let cast = cast(state);
    lives::IDEAS
        .iter()
        .map(|idea| {
            let place = if idea.outdoors {
                cast.quiet
            } else {
                cast.gathering
            };
            world_projection::ProjectionCommand {
                id: format!("{SUGGEST_COMMAND}{}", idea.id),
                title: idea.name.into(),
                detail: if idea.outdoors {
                    format!("Out by {}, if the weather holds", lives::name(state, place))
                } else {
                    format!("At {}", lives::name(state, place))
                },
                effects: Vec::new(),
                scenery: None,
                moves: Vec::new(),
                asker: None,
                question: None,
                unavailable: lives::can_suggest(state, &cast, idea.id).err(),
                hand: Some(world_projection::Hand {
                    verb: "Suggest".into(),
                    thing: idea.name.into(),
                    at: Some(world_projection::SelectionId::Entity(place)),
                    cost: None,
                }),
                preview: None,
            }
        })
        .collect()
}

/// Every name a Pocket Universe person can have, in all three places, and
/// the people they speak of who are never seen. A name is shown in Latin
/// letters in every language, as it is written.
pub fn people_names() -> Vec<&'static str> {
    [
        "Nia Chen",
        "Tomas Vale",
        "Ines Duarte",
        "Lena Ortiz",
        "Max Park",
        "Ray Kowalski",
        "Piko",
        "Miri",
        "Tuk",
    ]
    .into_iter()
    .chain(MARS_NAMES.iter().copied())
    .chain(TOWN_NAMES.iter().copied())
    .chain(ICE_NAMES.iter().copied())
    .chain(crate::kin::child_names())
    .chain(["Uko", "Gus", "Dunn", "Henderson", "Kowalski"])
    .collect()
}
