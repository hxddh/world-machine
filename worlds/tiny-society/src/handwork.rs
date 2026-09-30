//! What the player can do in the harbour with their own hands: build a
//! bench by the quay, string bunting outside the pub, plant a garden by the
//! school, move what they made, give someone a present, invite someone out.
//! The mechanics are the `hands` System's.

use crate::{BAKERY, HARBOR, PUB, SCHOOL};
use hands::commands::{add, enjoy};
use hands::{Effect, Kit, Purse, Thing, Verb};
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
        effect: Effect::Rest,
    },
    Thing {
        id: "lamp",
        name: "Lamp post",
        verb: Verb::Build,
        shape: "lantern",
        cost: 40,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "stall",
        name: "Market stall",
        verb: Verb::Build,
        shape: "stall",
        cost: 60,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "flagpole",
        name: "Flagpole",
        verb: Verb::Build,
        shape: "flag",
        cost: 20,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "bunting",
        name: "Bunting",
        verb: Verb::Decorate,
        shape: "bunting",
        cost: 10,
        lasts: Some(6),
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "lanterns",
        name: "Lanterns",
        verb: Verb::Decorate,
        shape: "lantern",
        cost: 10,
        lasts: Some(4),
        stages: &[],
        effect: Effect::Gather,
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
        effect: Effect::Harvest,
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
        effect: Effect::Harvest,
    },
    Thing {
        id: "well",
        name: "Well",
        verb: Verb::Build,
        shape: "well",
        cost: 30,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "swing",
        name: "Swing",
        verb: Verb::Build,
        shape: "swing",
        cost: 15,
        lasts: None,
        stages: &[],
        effect: Effect::Rest,
    },
    Thing {
        id: "fountain",
        name: "Fountain",
        verb: Verb::Build,
        shape: "fountain",
        cost: 50,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "signpost",
        name: "Signpost",
        verb: Verb::Build,
        shape: "signpost",
        cost: 10,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "birdhouse",
        name: "Birdhouse",
        verb: Verb::Build,
        shape: "birdhouse",
        cost: 8,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "statue",
        name: "Statue of a fisherman",
        verb: Verb::Build,
        shape: "statue",
        cost: 70,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "postbox",
        name: "Postbox",
        verb: Verb::Build,
        shape: "postbox",
        cost: 20,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "rowboat",
        name: "Rowing boat",
        verb: Verb::Build,
        shape: "boat",
        cost: 35,
        lasts: None,
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "picnic",
        name: "Picnic table",
        verb: Verb::Build,
        shape: "bench",
        cost: 20,
        lasts: None,
        stages: &[],
        effect: Effect::Gather,
    },
    Thing {
        id: "flowerboxes",
        name: "Flower boxes",
        verb: Verb::Decorate,
        shape: "planter",
        cost: 8,
        lasts: Some(8),
        stages: &[],
        effect: Effect::None,
    },
    Thing {
        id: "sunflowers",
        name: "Sunflowers",
        verb: Verb::Plant,
        shape: "garden",
        cost: 5,
        lasts: None,
        stages: &[("Sunflower shoots", "sprouts"), ("Sunflowers", "garden")],
        effect: Effect::Harvest,
    },
    Thing {
        id: "herbs",
        name: "Herb garden",
        verb: Verb::Plant,
        shape: "garden",
        cost: 5,
        lasts: None,
        stages: &[("Herb seedlings", "sprouts"), ("Herb garden", "garden")],
        effect: Effect::Harvest,
    },
];

fn places(_: &WorldState) -> Vec<EntityId> {
    vec![HARBOR, PUB, BAKERY, SCHOOL]
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

pub(crate) fn kit(state: &WorldState) -> Kit {
    Kit {
        notes: HANDS,
        first: 700,
        room: 300,
        period: crate::persistence::WORLD_DAY_TICKS,
        things: THINGS,
        places,
        people: crate::story::people_in,
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
        enjoy,
        most_standing: 12,
        works: crate::plots::works(),
        plots: crate::plots::plots,
        wears: crate::plots::wears,
        naming: crate::plots::naming,
        plot_stages: Some(crate::plots::STAGES),
        // A new player's first build is finished the next day.
        first_growing: crate::arrival::arrived(state).map(|_| 1),
    }
}

const HAND_COMMAND: &str = "tiny-society.hand.";

/// The command that does a deed.
pub(crate) fn command_id(deed: &str) -> String {
    format!("{HAND_COMMAND}{deed}")
}

/// What a thing the player can make by hand is called.
pub(crate) fn thing_name(id: &str) -> Option<&'static str> {
    THINGS
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
    let kit = kit(world.state());
    hands::commands::deed_commands(
        world,
        &kit,
        &hands::commands::DeedWords {
            prefix: HAND_COMMAND,
            gift: "A present for",
            invite_to: "the pub",
        },
    )
}

/// The deed that takes back the latest thing made or moved.
pub(crate) use hands::commands::UNDO;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{story, TinySociety};

    fn opened() -> crate::TinySocietyBranch {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        branch
    }

    #[test]
    fn twenty_things_to_make_each_drawn_its_own_way() {
        assert!(THINGS.len() >= 20, "{}", THINGS.len());
        let ids = THINGS
            .iter()
            .map(|thing| thing.id)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), THINGS.len());
        for thing in THINGS {
            for shape in std::iter::once(thing.shape).chain(thing.stages.iter().map(|(_, s)| *s)) {
                assert_ne!(
                    story::fixture_shape(shape),
                    world_projection::MarkShape::Parcel,
                    "{} is drawn as a parcel",
                    thing.id
                );
            }
        }
        let uses = |effect| THINGS.iter().filter(|thing| thing.effect == effect).count();
        assert!(uses(Effect::Rest) >= 2 && uses(Effect::Gather) >= 3 && uses(Effect::Harvest) >= 3);
    }

    #[test]
    fn put_anywhere_taken_back_and_used() {
        let mut branch = opened();
        let place = |key: &str| format!("{HAND_COMMAND}build.bench.{}{key}", HARBOR.0);
        branch.invoke_projection_command(&place("@63")).unwrap();
        let snapshot = branch.projection_snapshot();
        let bench = snapshot
            .canvas
            .items
            .iter()
            .find(|item| item.label.contains("Bench"))
            .expect("the bench is on the scene");
        assert_eq!(bench.spot, Some(0.63));
        let undo = snapshot
            .commands
            .iter()
            .find(|command| {
                command
                    .hand
                    .as_ref()
                    .is_some_and(|hand| hand.verb == "Undo")
            })
            .expect("it can be taken back");
        assert_eq!(undo.title, "Take back the bench");
        branch.invoke_projection_command(&undo.id).unwrap();
        assert!(!branch
            .projection_snapshot()
            .canvas
            .items
            .iter()
            .any(|item| item.label.contains("Bench")));
        // Put up for good, then used: someone rests there and says so.
        branch.invoke_projection_command(&place("@40")).unwrap();
        for _ in 0..8 {
            branch
                .invoke_projection_command(story::WAIT_COMMAND)
                .unwrap();
        }
        let rested = branch
            .world()
            .events()
            .iter()
            .find(|event| event.kind == "enjoyed")
            .expect("someone used the bench");
        assert!(story::line(rested).is_some(), "and said something");
        let replayed = branch.world().replay().unwrap();
        assert_eq!(replayed.state(), branch.world().state());
    }
}
