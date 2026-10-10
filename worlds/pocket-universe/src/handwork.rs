//! What the player can do in each pocket universe with their own hands:
//! put a bench by the habitat, string fairy lights at the arcade, plant
//! moss on the ice, move what they made, give someone something, invite
//! someone round. The mechanics are the `hands` System's.

use crate::{SLOT_A, SLOT_C, UNIVERSE};
use hands::commands::{add, enjoy};
use hands::{Effect, Kit, Thing, Verb};
use lives::Need;
use world_core::{EntityId, StateChange, Value, World, WorldState};

/// The entity what the player made is kept track of on.
pub(crate) const HANDS: EntityId = EntityId::new(39);

const MARS: &[Thing] = &[
    Thing {
        id: "bench",
        name: "Bench",
        verb: Verb::Build,
        shape: "bench",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Rest,
    },
    Thing {
        id: "lamp",
        name: "Solar lamp",
        verb: Verb::Build,
        shape: "lantern",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "tent",
        name: "Supply tent",
        verb: Verb::Build,
        shape: "tent",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "flag",
        name: "Flag",
        verb: Verb::Build,
        shape: "flag",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "bunting",
        name: "Bunting",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 0,
        lasts: Some(6),
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "lights",
        name: "Fairy lights",
        verb: Verb::Decorate,
        shape: "lantern",
        cost: 0,
        lasts: Some(4),
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "tray",
        name: "Growing tray",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[
            ("Seedlings", "sprouts"),
            ("Lettuce tray", "garden"),
            ("Tray in flower", "garden"),
        ],
        effect: Effect::Harvest,
    },
    Thing {
        id: "tree",
        name: "Dwarf apple tree",
        verb: Verb::Plant,
        shape: "tree",
        cost: 0,
        lasts: None,
        stages: &[
            ("Seedling", "sprouts"),
            ("Young tree", "tree"),
            ("Dwarf apple tree", "tree"),
        ],
        effect: Effect::Harvest,
    },
    Thing {
        id: "condenser",
        name: "Condenser well",
        verb: Verb::Build,
        shape: "well",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "swing",
        name: "Low-gravity swing",
        verb: Verb::Build,
        shape: "swing",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Rest,
    },
    Thing {
        id: "mist",
        name: "Mist fountain",
        verb: Verb::Build,
        shape: "fountain",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "marker",
        name: "Trail marker",
        verb: Verb::Build,
        shape: "signpost",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "seedbox",
        name: "Supply cache",
        verb: Verb::Build,
        shape: "birdhouse",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "founders",
        name: "Founders' statue",
        verb: Verb::Build,
        shape: "statue",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "beacon_post",
        name: "Message post",
        verb: Verb::Build,
        shape: "postbox",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "sled",
        name: "Dust sled",
        verb: Verb::Build,
        shape: "boat",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "canopy",
        name: "Rest canopy",
        verb: Verb::Build,
        shape: "tent",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Rest,
    },
    Thing {
        id: "racks",
        name: "Planter racks",
        verb: Verb::Decorate,
        shape: "planter",
        cost: 0,
        lasts: Some(8),
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "algae",
        name: "Algae beds",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[("Algae starter", "sprouts"), ("Algae beds", "garden")],
        effect: Effect::Harvest,
    },
    Thing {
        id: "moss_mars",
        name: "Red moss garden",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[("Moss spores", "sprouts"), ("Red moss garden", "garden")],
        effect: Effect::Harvest,
    },
];

const TOWN: &[Thing] = &[
    Thing {
        id: "bench",
        name: "Bench",
        verb: Verb::Build,
        shape: "bench",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Rest,
    },
    Thing {
        id: "lamp",
        name: "Street lamp",
        verb: Verb::Build,
        shape: "lantern",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "stand",
        name: "Hot-dog stand",
        verb: Verb::Build,
        shape: "stall",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "flag",
        name: "Flagpole",
        verb: Verb::Build,
        shape: "flag",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "streamers",
        name: "Streamers",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 0,
        lasts: Some(6),
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "lights",
        name: "Fairy lights",
        verb: Verb::Decorate,
        shape: "lantern",
        cost: 0,
        lasts: Some(4),
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "flowers",
        name: "Flower bed",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[
            ("Seedlings", "sprouts"),
            ("Flower bed", "garden"),
            ("Flowers in bloom", "garden"),
        ],
        effect: Effect::Harvest,
    },
    Thing {
        id: "maple",
        name: "Maple tree",
        verb: Verb::Plant,
        shape: "tree",
        cost: 0,
        lasts: None,
        stages: &[
            ("Maple sapling", "sprouts"),
            ("Young maple", "tree"),
            ("Maple tree", "tree"),
        ],
        effect: Effect::Harvest,
    },
    Thing {
        id: "pump",
        name: "Water pump",
        verb: Verb::Build,
        shape: "well",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "swing",
        name: "Playground swing",
        verb: Verb::Build,
        shape: "swing",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Rest,
    },
    Thing {
        id: "drinking",
        name: "Drinking fountain",
        verb: Verb::Build,
        shape: "fountain",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "sign",
        name: "Street sign",
        verb: Verb::Build,
        shape: "signpost",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "feeder",
        name: "Bird feeder",
        verb: Verb::Build,
        shape: "birdhouse",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "statue",
        name: "Mayor's statue",
        verb: Verb::Build,
        shape: "statue",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "mailbox",
        name: "Mailbox",
        verb: Verb::Build,
        shape: "postbox",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "rowboat",
        name: "Rowing boat",
        verb: Verb::Build,
        shape: "boat",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "hotdog",
        name: "Hot dog stand",
        verb: Verb::Build,
        shape: "stall",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "windowboxes",
        name: "Window boxes",
        verb: Verb::Decorate,
        shape: "planter",
        cost: 0,
        lasts: Some(8),
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "tomatoes",
        name: "Tomato patch",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[("Tomato seedlings", "sprouts"), ("Tomato patch", "garden")],
        effect: Effect::Harvest,
    },
    Thing {
        id: "roses",
        name: "Rose bed",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[("Rose cuttings", "sprouts"), ("Rose bed", "garden")],
        effect: Effect::Harvest,
    },
];

const ICE: &[Thing] = &[
    Thing {
        id: "bench",
        name: "Ice bench",
        verb: Verb::Build,
        shape: "bench",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Rest,
    },
    Thing {
        id: "lamp",
        name: "Lantern post",
        verb: Verb::Build,
        shape: "lantern",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "stall",
        name: "Fish stall",
        verb: Verb::Build,
        shape: "stall",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "flag",
        name: "Snow flag",
        verb: Verb::Build,
        shape: "flag",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "pennants",
        name: "Pennants",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 0,
        lasts: Some(6),
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "glow",
        name: "Glow stones",
        verb: Verb::Decorate,
        shape: "lantern",
        cost: 0,
        lasts: Some(4),
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "moss",
        name: "Moss patch",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[
            ("Moss sprouts", "sprouts"),
            ("Moss bed", "garden"),
            ("Moss in bloom", "garden"),
        ],
        effect: Effect::Harvest,
    },
    Thing {
        id: "kelp",
        name: "Kelp garden",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[
            ("Kelp shoots", "sprouts"),
            ("Kelp garden", "garden"),
            ("Kelp forest", "garden"),
        ],
        effect: Effect::Harvest,
    },
    Thing {
        id: "icewell",
        name: "Ice well",
        verb: Verb::Build,
        shape: "well",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "swing",
        name: "Kelp swing",
        verb: Verb::Build,
        shape: "swing",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Rest,
    },
    Thing {
        id: "geyser",
        name: "Geyser fountain",
        verb: Verb::Build,
        shape: "fountain",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "marker",
        name: "Snow marker",
        verb: Verb::Build,
        shape: "signpost",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "nest",
        name: "Nesting box",
        verb: Verb::Build,
        shape: "birdhouse",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "sculpture",
        name: "Ice sculpture",
        verb: Verb::Build,
        shape: "statue",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "stone",
        name: "Message stone",
        verb: Verb::Build,
        shape: "postbox",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "kayak",
        name: "Kayak",
        verb: Verb::Build,
        shape: "boat",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "hole",
        name: "Fishing-hole bench",
        verb: Verb::Build,
        shape: "bench",
        cost: 0,
        lasts: None,
        stages: &[],
        effect: Effect::Rest,
    },
    Thing {
        id: "pebbles",
        name: "Pebble planters",
        verb: Verb::Decorate,
        shape: "planter",
        cost: 0,
        lasts: Some(8),
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "seaweed",
        name: "Seaweed bed",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[("Seaweed shoots", "sprouts"), ("Seaweed bed", "garden")],
        effect: Effect::Harvest,
    },
    Thing {
        id: "lichen",
        name: "Lichen patch",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[("Lichen specks", "sprouts"), ("Lichen patch", "garden")],
        effect: Effect::Harvest,
    },
];

fn places(_: &WorldState) -> Vec<EntityId> {
    vec![SLOT_A, SLOT_C]
}

fn gift(state: &WorldState, who: EntityId) -> Vec<StateChange> {
    vec![
        add(state, who, lives::REGARD, 10, -100, 100),
        add(state, who, Need::Company.key(), -15, 0, 100),
        add(state, who, Need::Purpose.key(), -10, 0, 100),
    ]
}

fn invite(state: &WorldState, who: EntityId) -> Vec<StateChange> {
    vec![
        add(state, who, lives::REGARD, 4, -100, 100),
        add(state, who, Need::Company.key(), -30, 0, 100),
        StateChange::SetComponent {
            entity: who,
            key: lives::AT.into(),
            value: Value::Entity(SLOT_A),
        },
    ]
}

pub(crate) fn kit(state: &WorldState) -> Kit {
    let seed = match state
        .entity(UNIVERSE)
        .and_then(|u| u.component(crate::SEED))
    {
        Some(Value::Text(seed)) => seed.as_str(),
        _ => "",
    };
    Kit {
        notes: HANDS,
        first: 300,
        room: 300,
        period: crate::BACKGROUND_PERIOD,
        things: match seed {
            "1980s-town" => TOWN,
            "penguin-civilization" => ICE,
            _ => MARS,
        },
        places,
        people: crate::life::people_in,
        purse: None,
        gift_cost: 0,
        gift,
        invite,
        gathering: SLOT_A,
        growing: 5,
        per_period: 2,
        enjoy,
        most_standing: 10,
        works: crate::plots::works(state),
        plots: crate::plots::plots,
        wears: crate::plots::wears,
        naming: crate::plots::naming,
        plot_stages: Some(crate::plots::STAGES),
        // A new player's first build is finished the next day.
        first_growing: crate::arrival::arrived(state).map(|_| 1),
    }
}

const HAND_COMMAND: &str = "pocket-universe.hand.";

/// The command that does a deed.
pub(crate) fn command_id(deed: &str) -> String {
    format!("{HAND_COMMAND}{deed}")
}

/// What a thing the player can make by hand here is called.
pub(crate) fn thing_name(state: &WorldState, id: &str) -> Option<&'static str> {
    kit(state)
        .things
        .iter()
        .find(|thing| thing.id == id)
        .map(|thing| thing.name)
}

/// The deed a command does, if it is one of these.
pub(crate) fn parse_command(command_id: &str) -> Option<&str> {
    command_id.strip_prefix(HAND_COMMAND)
}

/// Everything the player could do with their own hands now, as commands.
pub(crate) fn commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    if crate::seed_id(world) == "unseeded" {
        return Vec::new();
    }
    let kit = kit(world.state());
    let gathering = lives::name(world.state(), SLOT_A);
    hands::commands::deed_commands(
        world,
        &kit,
        &hands::commands::DeedWords {
            prefix: HAND_COMMAND,
            gift: "Something for",
            invite_to: &gathering,
        },
    )
}

/// The deed that takes back the latest thing made or moved.
pub(crate) use hands::commands::UNDO;

/// What people say using what the player made, in each place's own words
/// rather than words any place might say: nobody on Mars watches the
/// gulls. By place, then for a rest, an evening together and a harvest.
fn enjoyed_lines(place: crate::places::Place) -> [&'static [&'static str]; 3] {
    match place {
        crate::places::Place::Ares => [
            &[
                "Ten minutes on the {what}. My knees thank you.",
                "Sat on the {what} and watched the dust settle.",
                "The {what} faces the ridge. Best view on Ares.",
                "Took my helmet off by the {what}. Inside, I mean.",
                "Whoever put the {what} there knew about long shifts.",
                "Five minutes on the {what}, then back to the filters.",
                "Logged my rest on the {what}. Nia insists.",
                "The {what} is warm from the heat pipe. Perfect.",
                "Watched Phobos rise from the {what}.",
                "Nodded off on the {what}. Nobody woke me. Kind.",
                "Sat on the {what} and counted the dust devils. Six.",
                "The {what} creaks in the cold. So do I.",
                "Had my ration bar on the {what}. Five-star dining.",
                "Wrote a letter home sitting on the {what}.",
                "The {what} is where I go when the alarms stop.",
                "Sat on the {what} with my eyes shut. Pretended it was a beach.",
            ],
            &[
                "We sat by the {what} and talked about Earth.",
                "Half the habitat ended up by the {what} after shift.",
                "{other} and I played chess by the {what}. I lost. Twice.",
                "The {what} glows like a campfire, if you squint.",
                "{other} told me about their first sol, by the {what}.",
                "Stayed by the {what} till the night cycle dimmed.",
                "{other} brought tea to the {what}. We drank it slowly.",
                "Someone started singing by the {what}. Then everyone.",
                "{other} and I planned the next rover trip by the {what}.",
                "By the {what} you can almost forget it's minus sixty outside.",
                "{other} and I argued about Earth music by the {what}. Nobody won.",
                "Someone brought cards to the {what}. We played till the lights dimmed.",
                "{other} fell asleep by the {what}. We left them a blanket.",
                "The {what} has become the place to be after a dust storm.",
                "{other} showed me the stars from beside the {what}.",
                "We had a quiz by the {what}. Nia won. Nia always wins.",
            ],
            &[
                "First pick from your {what}. Grown on Mars!",
                "Your {what} did well under the lamps. This is yours.",
                "Picked this from your {what}. Don't tell the ration log.",
                "From your {what}. It tastes like home.",
                "Your {what} keeps giving. The whole mess is jealous.",
            ],
        ],
        crate::places::Place::Maple => [
            &[
                "Sat on the {what} and watched the cars go by.",
                "The {what} is the best seat on Maple Street.",
                "Ate my lunch on the {what}. Pigeons took the crusts.",
                "Read a whole comic on the {what}.",
                "Waited for the night bus on the {what}. It was late. I didn't mind.",
                "Took my shoes off on the {what}. Don't judge.",
                "Five minutes on the {what} between shifts. Heaven.",
                "Listened to my Walkman on the {what} till the tape ran out.",
                "The {what}'s warm from the sun. Stayed too long.",
                "Whoever made the {what} deserves a medal.",
                "Did my homework on the {what}. Some of it.",
                "Fell asleep on the {what} in the sun. Woke up pink.",
                "Watched the paper boy miss every porch from the {what}.",
                "Sat on the {what} and waited for nothing in particular.",
                "The {what} is my thinking spot now. Don't tell.",
                "Read the Sunday paper on the {what}. All of it.",
            ],
            &[
                "We hung out by the {what} till someone's mom called.",
                "{other} brought a boombox to the {what}. Instant party.",
                "Half the street was out by the {what} last night.",
                "{other} and I traded baseball cards by the {what}.",
                "Told ghost stories by the {what}. {other} screamed first.",
                "The {what} is where Maple Street hangs out now.",
                "{other} and I split a pizza by the {what}.",
                "Somebody played guitar by the {what}. Badly. Lovely.",
                "{other} and I stayed out by the {what} past curfew.",
                "Fireflies all round the {what}. Like the whole street was glowing.",
                "{other} and I watched the fireflies by the {what}.",
                "Somebody's radio by the {what} played K-88 all night.",
                "{other} taught me a card trick by the {what}.",
                "The {what} is where you hear all the gossip now.",
                "{other} and I sat by the {what} and talked about leaving. Then didn't.",
                "We roasted marshmallows by the {what}. Mostly burned them.",
            ],
            &[
                "From your {what}. Mom made a pie with it.",
                "Your {what} came up great. Here, take some.",
                "Picked this from your {what}. The diner wants to buy some.",
                "First of the season, from your {what}.",
                "Your {what} beats anything at the supermarket.",
            ],
        ],
        crate::places::Place::Ice => [
            &[
                "Sat on the {what} and watched the waves.",
                "The {what} is out of the wind. Bliss.",
                "Napped by the {what} with my beak under my wing.",
                "Warmed my feet by the {what}.",
                "The chicks climb the {what}. I sat on it anyway.",
                "Watched the fishers come home from the {what}.",
                "Five minutes on the {what}. Then back to the ice.",
                "The {what} is the best spot on the floe. Don't tell anyone.",
                "Preened my feathers on the {what}. Very smart now.",
                "Whoever made the {what} has a good heart.",
                "Stood on one foot by the {what}. Very restful.",
                "The {what} is warm on the sunny side.",
                "Watched the seals from the {what}. They watched back.",
                "Had a long think on the {what}. About fish, mostly.",
                "Sat on the {what} and let the snow cover me. Cosy.",
                "The {what} is where I go to be quiet.",
            ],
            &[
                "We huddled by the {what} and sang the old songs.",
                "{other} and I shared a herring by the {what}.",
                "Half the colony was by the {what} under the aurora.",
                "{other} told a fish story by the {what}. It grew.",
                "By the {what} the long night feels shorter.",
                "{other} and I counted stars by the {what}.",
                "The chicks fell asleep round the {what}. So did I.",
                "{other} taught me a new song by the {what}.",
                "Stayed by the {what} till the fishers came in.",
                "The {what} glows on the ice. You can see it from the far floe.",
                "{other} and I told jokes by the {what}. Mine were better.",
                "Everyone brought a fish to the {what}. We had a feast.",
                "{other} danced by the {what}. The chicks copied.",
                "The {what} is the colony's favourite spot now.",
                "{other} and I watched the ice glow by the {what}.",
                "We played pebble games by the {what} till the tide turned.",
            ],
            &[
                "From your {what}. Fresh as the sea.",
                "Your {what} did well in the cold. This is yours.",
                "Picked this from your {what}. The chicks wanted it.",
                "First of the thaw, from your {what}.",
                "Your {what} keeps giving. The vault's jealous.",
            ],
        ],
    }
}

/// More of what people say resting on something the player made, in
/// each place's own words.
fn more_rest_lines(place: crate::places::Place) -> &'static [&'static str] {
    match place {
        crate::places::Place::Ares => &[
            "Sat on the {what} and listened to the dome tick as it cooled.",
            "The {what} is the one place nobody asks me for a status report.",
            "Read my sister's last message twice on the {what}.",
            "Five minutes on the {what} and the red outside looks almost friendly.",
            "Fixed a seal on my glove sitting on the {what}. Multitasking.",
            "The {what} doesn't hum or beep. That's why I like it.",
            "Watched the rover tracks fill with dust from the {what}.",
            "Sat on the {what} and planned what I'd plant if water were free.",
            "My back needed the {what} more than my schedule did.",
            "Counted the panels on the dome from the {what}. Forty-two. Still.",
            "The {what} is cold at first, then just right.",
            "A quiet sit on the {what} before the night cycle. Needed that.",
        ],
        crate::places::Place::Maple => &[
            "Waved at the mail truck from the {what}. The driver honked back.",
            "Sat on the {what} and finished my crossword. Mostly.",
            "The {what} is the best spot to hear the arcade through the wall.",
            "Shared the {what} with a lady and her poodle. Both very polite.",
            "Sat on the {what} with a soda till the fizz ran out.",
            "The {what} is where I go when the house gets too loud.",
            "Watched the sprinklers go round from the {what}. Summer sound.",
            "Five minutes on the {what} before the bus. Made it, just.",
            "The {what} creaks when you lean back. Like an old friend.",
            "Wrote a postcard to my cousin on the {what}. Drew the street on it.",
            "Sat on the {what} and counted the porch lights coming on.",
            "Nobody's in a hurry on the {what}. That's the rule.",
        ],
        crate::places::Place::Ice => &[
            "Sat on the {what} and watched the icebergs drift past. Slow parade.",
            "The {what} is where I go to dry my feathers.",
            "Fluffed up on the {what} and let the wind go round me.",
            "Rested my flippers on the {what}. They'd earned it.",
            "Watched the krill shimmer from the {what}. Lunch, later.",
            "The {what} is the quietest spot when the chicks are napping.",
            "Sat on the {what} and practised my best council speech.",
            "A seal waved from the floe while I sat on the {what}. I think.",
            "The {what} keeps the snow off my back. Clever thing.",
            "Five minutes on the {what}, then back to the fish vault.",
            "Sat on the {what} and watched my breath turn to little clouds.",
            "The {what} by the bridge has the best view of the sunrise. When there is one.",
        ],
    }
}

/// More of what people say using what the player made, in each place's
/// own words: evenings with nobody in particular, evenings with someone,
/// and what a garden gave. By place.
fn more_enjoyed_lines(place: crate::places::Place) -> [&'static [&'static str]; 3] {
    match place {
        crate::places::Place::Ares => [
            &[
                "The dome hummed while we sat by the {what}. Nobody talked. Didn't need to.",
                "Someone read old letters from Earth by the {what}. We all listened.",
                "Watched Deimos cross the sky from the {what}.",
                "Rehydrated cocoa by the {what}. Tasted almost real.",
                "The {what} by the airlock is where the crew unwinds now.",
                "Someone hummed a song from home by the {what}. We all knew it.",
                "We swapped stories of our first dust storm by the {what}.",
                "Fixed a glove by the light of the {what}. Good light for it.",
                "The {what} made the long night cycle feel short.",
                "We drew the Earth from memory by the {what}. Everyone's was different.",
                "Ration bars and good company by the {what}. Better than it sounds.",
                "Played twenty questions by the {what}. The answer was always \"dust\".",
                "Somebody brought the good coffee to the {what}. The last tin.",
                "The heaters ticked and the {what} glowed. Felt like a home.",
                "We planned a garden for when the dome's bigger. By the {what}, naturally.",
                "Stargazing by the {what}. Earth was the bright blue one. We all waved.",
                "Sat by the {what} after the shift and let my ears stop ringing.",
                "Someone told the story of the first landing by the {what}. Again. We let them.",
                "The {what} and a deck of cards: all a colony needs, really.",
                "Dust storm outside, warm light by the {what} inside. Perfect evening.",
                "We sat by the {what} and named the craters we can see. Badly.",
            ],
            &[
                "{other} and I fixed a radio by the {what}. Now it plays static. Lovely static.",
                "{other} showed me photos from Earth by the {what}. Their dog looks just like them.",
                "{other} and I raced toy rovers by the {what}. I won. Barely.",
                "{other} taught me the names of the Earth constellations by the {what}.",
                "{other} and I shared the last fresh tomato by the {what}.",
                "{other} fell asleep mid-story by the {what}. Best story yet.",
            ],
            &[
                "Your {what} gave us a salad. A real salad, on Mars.",
                "Brought you the best from your {what}. The crew voted.",
                "Your {what} smells like rain. We miss rain.",
                "From your {what}. Fresher than anything on the supply shuttle.",
                "Your {what} is the greenest thing for a million miles. Here.",
                "Picked these from your {what} after the dust settled.",
                "Your {what} grew this. Nia wants to study it. I said eat it.",
            ],
        ],
        crate::places::Place::Maple => [
            &[
                "Somebody brought a cassette player to the {what}. We danced on the pavement.",
                "Played kick the can by the {what} till the streetlights buzzed on.",
                "The {what} by the arcade is where everyone ends up after the diner closes.",
                "Lightning bugs and a warm night by the {what}. Summer, basically.",
                "We swapped mixtapes by the {what}. I got the good one.",
                "Ate popsicles by the {what}. Mine melted down my arm.",
                "The night bus went by twice while we sat by the {what}.",
                "Someone's mom brought lemonade out to the {what}. Best lemonade on the street.",
                "We sat by the {what} and rated every car that went by.",
                "Thunder rolled in, so we crowded under by the {what}. Nobody went home.",
                "K-88 played the top forty by the {what}. We knew every word.",
                "Hula hoops by the {what}. I can do forty now.",
                "The {what} on a Friday night. Nowhere better on Maple Street.",
                "We made plans for the summer fair by the {what}. Big ones.",
                "Somebody's dog sat with us by the {what} all evening. Good dog.",
                "Traded comics by the {what}. I'm still a Spider-Man short.",
                "The ice cream truck stopped right by the {what}. Fate.",
                "Rode bikes in circles round the {what} till the porch lights came on.",
                "Old Walt told us about the street in the fifties, by the {what}.",
                "Sat by the {what} and watched the arcade sign flicker. Hypnotic.",
                "The {what} after rain smells like summer and pavement. Nice.",
            ],
            &[
                "{other} and I recorded a radio show by the {what}. Audience of two.",
                "{other} beat me at arm wrestling by the {what}. Rematch Friday.",
                "{other} and I planned a road trip by the {what}. We'll never go. Still fun.",
                "{other} taught me to moonwalk by the {what}. Sort of.",
                "{other} and I shared fries from the diner by the {what}.",
                "{other} told me a secret by the {what}. It's a good one.",
            ],
            &[
                "Your {what} gave us enough for the whole block. Here's yours.",
                "The diner says your {what} is the real deal. Here.",
                "Fresh from your {what}, still warm from the sun.",
                "Your {what} did great this year. Grandma's impressed, and she's never impressed.",
                "Picked a bagful from your {what}. Left some for the birds.",
                "From your {what}. I'd trade a whole mixtape for these.",
                "Your {what} is the talk of Maple Street. Try one.",
            ],
        ],
        crate::places::Place::Ice => [
            &[
                "The aurora came out while we sat by the {what}. Green as kelp.",
                "We shuffled closer to the {what} as the wind got up. Toasty.",
                "Someone told the tale of the great iceberg by the {what}. Shivers.",
                "The chicks played slide-the-pebble by the {what} till bedtime.",
                "Sang the moonrise song by the {what}. Off key. Beautifully.",
                "We sat by the {what} and listened to the ice creak.",
                "A seal popped up near the {what}. Everyone pretended not to be scared.",
                "The {what} by the bridge is where the colony gathers now.",
                "Shared the last of the krill by the {what}. Nobody counted.",
                "The long night is easier by the {what}. Everyone says so.",
                "We guessed the shapes of the clouds by the {what}. All fish.",
                "Snow fell soft by the {what}. Nobody wanted to move.",
                "Practised the council squawk by the {what}. We're getting good.",
                "The elders told stories of the first floe by the {what}.",
                "We sat by the {what} and watched the stars wheel over the ice.",
                "A tide of chatter by the {what} tonight. Lovely noise.",
                "Slid down the snowbank and back to the {what}. Twelve times.",
                "Warm breath and a warm light by the {what}. Enough.",
                "Watched the fish-vault lanterns from the {what}. Like little moons.",
                "We told the chicks about the warm sea by the {what}. They didn't believe us.",
                "Huddled by the {what} through a blizzard. Laughing, mostly.",
            ],
            &[
                "{other} and I built a tiny snow bridge by the {what}. It held a pebble.",
                "{other} and I shared a sprat by the {what}. Very romantic.",
                "{other} taught me to toboggan on my belly by the {what}.",
                "{other} and I counted the chicks by the {what}. Twice. Different answers.",
                "{other} told me where the best fish hide, by the {what}. Shh.",
                "{other} fell asleep on my shoulder by the {what}. I stayed put.",
            ],
            &[
                "Your {what} fed half the colony. Here's your share.",
                "Fresh from your {what}, before the gulls found it.",
                "Your {what} grew through the frost. Brave little thing.",
                "From your {what}. The elders say it's lucky.",
                "Picked from your {what} under the aurora. Tastes green.",
                "Your {what} keeps the vault full. The chicks thank you.",
                "The best of your {what}, wrapped in kelp. Very fancy.",
            ],
        ],
    }
}

/// The lines said of anything at all, in every place, as templates: a
/// rest, an evening, a harvest. Filler, by the words harness.
#[cfg(test)]
pub(crate) fn filler() -> Vec<&'static str> {
    use crate::places::Place;
    [Place::Ares, Place::Maple, Place::Ice]
        .into_iter()
        .flat_map(|place| {
            let [rest, gather, harvest] = enjoyed_lines(place);
            let [alone, together, gave] = more_enjoyed_lines(place);
            [
                rest,
                more_rest_lines(place),
                gather,
                harvest,
                alone,
                together,
                gave,
            ]
            .into_iter()
            .flatten()
            .copied()
        })
        .collect()
}

/// How many uses of the same kind (the same effect, alone or with
/// someone) came before each use, by its event: worked out once for each
/// way the World stands, rather than counted again for every line told.
struct EnjoyedBefore(std::collections::HashMap<world_core::EventId, usize>);

fn enjoyed_before(world: &World, event: world_core::EventId) -> Option<usize> {
    let kept = world.as_it_stands(|| {
        // A handful of kinds, so a list rather than a map.
        let mut counts: Vec<((Option<&Value>, bool), usize)> = Vec::new();
        EnjoyedBefore(
            world
                .events_of_kind(&["enjoyed"])
                .into_iter()
                .map(|used| {
                    let key = (used.payload.get("effect"), used.targets.len() > 1);
                    let at = counts.iter().position(|(kind, _)| *kind == key);
                    let at = at.unwrap_or_else(|| {
                        counts.push((key, 0));
                        counts.len() - 1
                    });
                    let before = counts[at].1;
                    counts[at].1 += 1;
                    (used.id, before)
                })
                .collect(),
        )
    });
    kept.0.get(&event).copied()
}

/// What someone says using something the player made, in the place's own
/// words: the next of its lines for that thing, so the same is not said
/// of it again until every other has been. `None` for anything else, or
/// when what was used is gone and cannot be named.
pub(crate) fn enjoyed_line(world: &World, event: &world_core::Event) -> Option<(EntityId, String)> {
    if event.kind != "enjoyed" {
        return None;
    }
    let place = crate::places::Place::of(world.state())?;
    let state = world.state();
    let effect = match event.payload.get("effect") {
        Some(Value::Text(effect)) => effect.as_str(),
        _ => return None,
    };
    let with_other = event.targets.len() > 1;
    let [rest, gather, harvest] = enjoyed_lines(place);
    let [alone, together, gave] = more_enjoyed_lines(place);
    let lines: Vec<&str> = match effect {
        "rest" => rest.iter().chain(more_rest_lines(place)).copied().collect(),
        "gather" => gather
            .iter()
            .chain(if with_other { together } else { alone })
            .filter(|line| line.contains("{other}") == with_other)
            .copied()
            .collect(),
        "harvest" => harvest.iter().chain(gave).copied().collect(),
        _ => return None,
    };
    let thing = *event.targets.first()?;
    let what = lives::name(state, state.entity(thing).map(|_| thing)?).to_lowercase();
    let other = event
        .targets
        .get(1)
        .map(|other| lives::first_name(state, *other));
    let who = event.actor?;
    // Each use of a kind spoken of says the next of its lines, so none is
    // heard again until every other has been; a rest only one use in
    // four, as an evening with someone, an evening alone one in three and a
    // harvest one in two (fewer of a
    // place's people speak each day than the harbour's), so the bench never becomes
    // what the place talks about most (an empty line the other times).
    let before = enjoyed_before(world, event.id).unwrap_or_else(|| {
        // Not one of the World's own uses: every use of its kind is before it.
        world
            .events_of_kind(&["enjoyed"])
            .into_iter()
            .filter(|used| {
                used.payload.get("effect") == event.payload.get("effect")
                    && (used.targets.len() > 1) == with_other
            })
            .count()
    });
    let every = match (effect, with_other) {
        ("rest", _) | ("gather", true) => 4,
        ("gather", false) => 3,
        ("harvest", _) => 2,
        _ => 1,
    };
    if !before.is_multiple_of(every) {
        return Some((who, String::new()));
    }
    let line = lines.get(before / every % lines.len().max(1))?;
    Some((
        who,
        line.replace("{what}", &what)
            .replace("{other}", other.as_deref().unwrap_or("")),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twenty_things_to_make_in_every_place_and_people_use_them() {
        for (seed, things) in [("mars", MARS), ("town", TOWN), ("ice", ICE)] {
            assert!(things.len() >= 20, "{seed}: {}", things.len());
            let ids = things
                .iter()
                .map(|thing| thing.id)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(ids.len(), things.len(), "{seed}");
            let uses = |effect| things.iter().filter(|thing| thing.effect == effect).count();
            assert!(
                uses(Effect::Rest) >= 2 && uses(Effect::Gather) >= 3 && uses(Effect::Harvest) >= 3,
                "{seed}"
            );
        }
        for seed in [
            crate::SEED_MARS_COLONY_COMMAND,
            crate::SEED_1980S_TOWN_COMMAND,
            crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
        ] {
            let mut universe = crate::PocketUniverse::new().unwrap();
            universe.invoke_projection_command(seed).unwrap();
            let bench = crate::projection::snapshot(universe.world())
                .commands
                .into_iter()
                .find(|command| {
                    command.unavailable.is_none()
                        && command
                            .hand
                            .as_ref()
                            .is_some_and(|hand| hand.verb == "Build")
                        && command.title.contains("bench")
                        || command.title == "Bench"
                })
                .unwrap_or_else(|| panic!("{seed}: a bench to build"));
            universe
                .invoke_projection_command(&format!("{}@25", bench.id))
                .unwrap();
            let snapshot = crate::projection::snapshot(universe.world());
            assert!(
                snapshot
                    .canvas
                    .items
                    .iter()
                    .any(|item| item.spot == Some(0.25)),
                "{seed}: the bench stands where it was put"
            );
            for _ in 0..8 {
                universe
                    .invoke_projection_command(crate::NUDGE_COMMAND)
                    .unwrap();
            }
            assert!(
                universe
                    .world()
                    .events()
                    .iter()
                    .any(|event| event.kind == "enjoyed"),
                "{seed}: nobody used what was made"
            );
        }
    }
}
