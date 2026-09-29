//! What the player can do in each pocket universe with their own hands:
//! put a bench by the habitat, string fairy lights at the arcade, plant
//! moss on the ice, move what they made, give someone something, invite
//! someone round. The mechanics are the `hands` System's.

use crate::{SLOT_A, SLOT_C, UNIVERSE};
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

fn add(state: &WorldState, who: EntityId, key: &str, by: i64, min: i64, max: i64) -> StateChange {
    let now = match state.entity(who).and_then(|entity| entity.component(key)) {
        Some(Value::Integer(value)) => *value,
        _ => 0,
    };
    StateChange::SetComponent {
        entity: who,
        key: key.into(),
        value: (now + by).clamp(min, max).into(),
    }
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

/// What using something the player made does for someone: a rest on a
/// bench eases tiredness, an evening under a lamp eases loneliness, and
/// bringing the player what a garden grew warms them to the player.
fn enjoy(state: &WorldState, who: EntityId, effect: Effect) -> Vec<StateChange> {
    match effect {
        Effect::Rest => vec![
            add(state, who, Need::Rest.key(), -20, 0, 100),
            add(state, who, lives::REGARD, 1, -100, 100),
        ],
        Effect::Gather => vec![
            add(state, who, Need::Company.key(), -20, 0, 100),
            add(state, who, lives::REGARD, 1, -100, 100),
        ],
        Effect::Harvest => vec![
            add(state, who, Need::Purpose.key(), -10, 0, 100),
            add(state, who, lives::REGARD, 3, -100, 100),
        ],
        Effect::None => Vec::new(),
    }
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
    }
}

const HAND_COMMAND: &str = "pocket-universe.hand.";

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
    hands::deeds(world, &kit)
        .into_iter()
        .map(|deed| {
            let place = lives::name(world.state(), deed.at);
            world_projection::ProjectionCommand {
                id: format!("{HAND_COMMAND}{}", deed.key),
                title: deed.thing.clone(),
                detail: match deed.verb {
                    Verb::Give => format!("Something for {}", deed.thing),
                    Verb::Invite => format!("Invite {} to {gathering}", deed.thing),
                    Verb::Move => format!("Move it to {place}"),
                    _ => format!("By {place}"),
                },
                effects: Vec::new(),
                scenery: None,
                moves: Vec::new(),
                asker: None,
                question: None,
                unavailable: deed.unavailable,
                hand: Some(world_projection::Hand {
                    verb: deed.verb.word().into(),
                    thing: deed.thing,
                    at: Some(world_projection::SelectionId::Entity(deed.at)),
                    cost: deed.cost,
                }),
                preview: None,
            }
        })
        .chain(hands::can_undo(world.state(), &kit).map(|title| {
            world_projection::ProjectionCommand {
                id: format!("{HAND_COMMAND}{UNDO}"),
                title,
                detail: "What it cost comes back".into(),
                effects: Vec::new(),
                scenery: None,
                moves: Vec::new(),
                asker: None,
                question: None,
                unavailable: None,
                hand: Some(world_projection::Hand {
                    verb: "Undo".into(),
                    thing: String::new(),
                    at: None,
                    cost: None,
                }),
                preview: None,
            }
        }))
        .collect()
}

/// The deed that takes back the latest thing made or moved.
pub(crate) const UNDO: &str = "undo";

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
    let lines = enjoyed_lines(place)[match effect {
        "rest" => 0,
        "gather" => 1,
        "harvest" => 2,
        _ => return None,
    }];
    let thing = *event.targets.first()?;
    let what = lives::name(state, state.entity(thing).map(|_| thing)?).to_lowercase();
    let other = event
        .targets
        .get(1)
        .map(|other| lives::first_name(state, *other));
    let who = event.actor?;
    // Each thing is used every few periods, so its next use says the next
    // of its lines.
    let every = match effect {
        "rest" => 4,
        "gather" => 3,
        _ => 7,
    };
    let turn = (event.world_time / crate::BACKGROUND_PERIOD) / every + thing.0 * 7;
    let fitting = lines
        .iter()
        .filter(|line| other.is_some() || !line.contains("{other}"))
        .collect::<Vec<_>>();
    let line = fitting.get((turn % fitting.len().max(1) as u64) as usize)?;
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
