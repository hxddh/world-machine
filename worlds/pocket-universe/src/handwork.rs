//! What the player can do in each pocket universe with their own hands:
//! put a bench by the habitat, string fairy lights at the arcade, plant
//! moss on the ice, move what they made, give someone something, invite
//! someone round. The mechanics are the `hands` System's.

use crate::{SLOT_A, SLOT_C, UNIVERSE};
use hands::{Kit, Thing, Verb};
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
    },
    Thing {
        id: "lamp",
        name: "Solar lamp",
        verb: Verb::Build,
        shape: "lantern",
        cost: 0,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "tent",
        name: "Supply tent",
        verb: Verb::Build,
        shape: "tent",
        cost: 0,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "flag",
        name: "Flag",
        verb: Verb::Build,
        shape: "flag",
        cost: 0,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "bunting",
        name: "Bunting",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 0,
        lasts: Some(6),
        stages: &[],
    },
    Thing {
        id: "lights",
        name: "Fairy lights",
        verb: Verb::Decorate,
        shape: "lantern",
        cost: 0,
        lasts: Some(4),
        stages: &[],
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
    },
    Thing {
        id: "lamp",
        name: "Street lamp",
        verb: Verb::Build,
        shape: "lantern",
        cost: 0,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "stand",
        name: "Hot-dog stand",
        verb: Verb::Build,
        shape: "stall",
        cost: 0,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "flag",
        name: "Flagpole",
        verb: Verb::Build,
        shape: "flag",
        cost: 0,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "streamers",
        name: "Streamers",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 0,
        lasts: Some(6),
        stages: &[],
    },
    Thing {
        id: "lights",
        name: "Fairy lights",
        verb: Verb::Decorate,
        shape: "lantern",
        cost: 0,
        lasts: Some(4),
        stages: &[],
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
    },
    Thing {
        id: "lamp",
        name: "Lantern post",
        verb: Verb::Build,
        shape: "lantern",
        cost: 0,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "stall",
        name: "Fish stall",
        verb: Verb::Build,
        shape: "stall",
        cost: 0,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "flag",
        name: "Snow flag",
        verb: Verb::Build,
        shape: "flag",
        cost: 0,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "pennants",
        name: "Pennants",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 0,
        lasts: Some(6),
        stages: &[],
    },
    Thing {
        id: "glow",
        name: "Glow stones",
        verb: Verb::Decorate,
        shape: "lantern",
        cost: 0,
        lasts: Some(4),
        stages: &[],
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
        people: crate::life::people,
        purse: None,
        gift_cost: 0,
        gift,
        invite,
        gathering: SLOT_A,
        growing: 5,
        per_period: 2,
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
            }
        })
        .collect()
}
