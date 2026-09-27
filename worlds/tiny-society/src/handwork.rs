//! What the player can do in the harbour with their own hands: build a
//! bench by the quay, string bunting outside the pub, plant a garden by the
//! school, move what they made, give someone a present, invite someone out.
//! The mechanics are the `hands` System's.

use crate::{BAKERY, HARBOR, PUB, SCHOOL};
use hands::{Kit, Purse, Thing, Verb};
use lives::Need;
use society_basic::CASH;
use world_core::{EntityId, StateChange, Value, World, WorldState};

/// The entity what the player made is kept track of on.
pub(crate) const HANDS: EntityId = EntityId::new(403);

const THINGS: &[Thing] = &[
    Thing {
        id: "bench",
        name: "Bench",
        verb: Verb::Build,
        shape: "bench",
        cost: 25,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "lamp",
        name: "Lamp post",
        verb: Verb::Build,
        shape: "lantern",
        cost: 40,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "stall",
        name: "Market stall",
        verb: Verb::Build,
        shape: "stall",
        cost: 60,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "flagpole",
        name: "Flagpole",
        verb: Verb::Build,
        shape: "flag",
        cost: 20,
        lasts: None,
        stages: &[],
    },
    Thing {
        id: "bunting",
        name: "Bunting",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 10,
        lasts: Some(6),
        stages: &[],
    },
    Thing {
        id: "lanterns",
        name: "Lanterns",
        verb: Verb::Decorate,
        shape: "lantern",
        cost: 10,
        lasts: Some(4),
        stages: &[],
    },
    Thing {
        id: "vegetables",
        name: "Vegetable garden",
        verb: Verb::Plant,
        shape: "garden",
        cost: 5,
        lasts: None,
        stages: &[
            ("Seedlings", "sprouts"),
            ("Vegetable patch", "garden"),
            ("Vegetables in flower", "garden"),
        ],
    },
    Thing {
        id: "apple_tree",
        name: "Apple tree",
        verb: Verb::Plant,
        shape: "tree",
        cost: 10,
        lasts: None,
        stages: &[
            ("Sapling", "sprouts"),
            ("Young apple tree", "tree"),
            ("Apple tree", "tree"),
        ],
    },
];

fn places(_: &WorldState) -> Vec<EntityId> {
    vec![HARBOR, PUB, BAKERY, SCHOOL]
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

/// A present cheers someone up and leaves them less short of company and
/// money.
pub(crate) fn gift(state: &WorldState, who: EntityId) -> Vec<StateChange> {
    vec![
        add(state, who, lives::REGARD, 10, -100, 100),
        add(state, who, Need::Company.key(), -15, 0, 100),
        add(state, who, Need::Money.key(), -10, 0, 100),
    ]
}

/// Someone invited out goes to where people meet, and is less lonely.
pub(crate) fn invite(state: &WorldState, who: EntityId, gathering: EntityId) -> Vec<StateChange> {
    vec![
        add(state, who, lives::REGARD, 4, -100, 100),
        add(state, who, Need::Company.key(), -30, 0, 100),
        StateChange::SetComponent {
            entity: who,
            key: lives::AT.into(),
            value: Value::Entity(gathering),
        },
    ]
}

pub(crate) fn kit(_: &WorldState) -> Kit {
    Kit {
        notes: HANDS,
        first: 700,
        room: 300,
        period: crate::persistence::WORLD_DAY_TICKS,
        things: THINGS,
        places,
        people: crate::story::people,
        purse: Some(Purse {
            entity: HARBOR,
            key: CASH,
            name: "the harbour fund",
        }),
        gift_cost: 15,
        gift,
        invite: |state, who| invite(state, who, PUB),
        gathering: PUB,
        growing: 5,
        per_period: 2,
        most_standing: 12,
    }
}

const HAND_COMMAND: &str = "tiny-society.hand.";

/// The deed a command does, if it is one of these.
pub(crate) fn parse_command(command_id: &str) -> Option<&str> {
    command_id.strip_prefix(HAND_COMMAND)
}

/// Everything the player could do with their own hands now, as commands.
pub(crate) fn commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let kit = kit(world.state());
    hands::deeds(world, &kit)
        .into_iter()
        .map(|deed| {
            let place = lives::name(world.state(), deed.at);
            world_projection::ProjectionCommand {
                id: format!("{HAND_COMMAND}{}", deed.key),
                title: deed.thing.clone(),
                detail: match deed.verb {
                    Verb::Give => format!("A present for {}", deed.thing),
                    Verb::Invite => format!("Invite {} to the pub", deed.thing),
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
