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
    // Free: the first thing a newcomer's hands are offered, before any
    // card asks for money (v0.27's first minute).
    Thing {
        id: "wildflowers",
        name: "Wildflowers",
        verb: Verb::Plant,
        shape: "garden",
        cost: 0,
        lasts: None,
        stages: &[("Wildflower shoots", "sprouts"), ("Wildflowers", "garden")],
        effect: Effect::None,
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

/// What people at the harbour say resting on something the player made,
/// in the harbour's own words: salt, boats, gulls and the tide.
const REST_LINES: [&str; 38] = [
    "The {what} by {place}: just what my legs needed.",
    "Best seat by {place}, this {what}.",
    "I could sit on this {what} all day. The tide agrees.",
    "Good {what}, this. Solid as the harbour wall.",
    "You can see every boat in the harbour from here.",
    "Five minutes on the {what}. Then back to the nets.",
    "Whoever made this {what} knew what tired feet want.",
    "My favourite spot, the {what} by {place}.",
    "Sat on the {what} and watched the gulls squabble. Bliss.",
    "A rest on the {what} and I'm new again.",
    "Ate my bread roll on the {what}. The gulls got the crust.",
    "The {what}'s warm from the sun this time of day.",
    "Nodded off on the {what}. Don't tell anyone.",
    "Somebody left a book on the {what}. I read a chapter.",
    "Watched the ferry come and go from the {what}.",
    "My knees thank whoever put a {what} by {place}.",
    "A quiet sit on the {what}. Nobody asked me anything.",
    "Shared the {what} with a stranger off the ferry. Nice sort.",
    "The {what} by {place} has a view I'd pay for.",
    "Sat on the {what} till my tea went cold.",
    "Counted the masts from the {what}. Lost count at nine.",
    "The {what} smells of salt and sun. Lovely.",
    "Mended a net on the {what}. Kinder than the quay steps.",
    "Took my boots off on the {what}. Don't look.",
    "The {what} is where I go to think. Mostly about lunch.",
    "Sat on the {what} and let the sea do the talking.",
    "A crab tried to share the {what} with me. I let it.",
    "The fog rolled in while I sat on the {what}. Cosy, somehow.",
    "Read the paper on the {what}. All of it, even the tides.",
    "There's a dip in the {what} just my shape now.",
    "Waved at every boat from the {what}. Most waved back.",
    "The {what} creaks like an old hull. I like that.",
    "Dozed on the {what} and woke to the ferry's horn.",
    "Watched the tide turn from the {what}. It took its time.",
    "The {what} by {place} catches the last of the sun.",
    "Wrote to my sister from the {what}. Told her about the gulls.",
    "Had a proper sit on the {what}. Doctor's orders, I told myself.",
    "Saw a seal off the point from the {what}. Honest.",
];

/// What people at the harbour say after an evening by something the
/// player made.
const GATHER_ALONE: [&str; 38] = [
    "Our little crowd by the {what} grows every week.",
    "Nobody wanted to go home from the {what}.",
    "The {what} by {place} is the warmest spot after dark.",
    "An owl came and sat near the {what}. We all went quiet.",
    "It's nice by the {what} of an evening.",
    "We lost track of time by the {what}.",
    "The light down by {place} makes you want to stay.",
    "The {what} was the only light down by {place}.",
    "Stayed out by the {what} longer than I meant to.",
    "Half the harbour ended up by the {what} last night.",
    "Somebody brought a flask. The {what} did the rest.",
    "The {what} by {place} is where the talking happens now.",
    "We made plans by the {what}. Big ones, for us.",
    "Played cards by the {what} till the light gave out.",
    "The {what} glows like a little moon by {place}.",
    "Supper out by the {what}. Everything tastes better.",
    "I walked home late from the {what}, humming.",
    "Fog came in, so we huddled by the {what}. Didn't mind a bit.",
    "Someone's radio by the {what} had the shipping forecast on. We all listened.",
    "Half the quay came by the {what} with mugs of tea.",
    "The boats knocked gently against the quay while we sat by the {what}.",
    "Someone started a song by the {what}. The whole quay knew the words.",
    "Watched the ferry's lights go out across the water from the {what}.",
    "The gulls finally went quiet. We stayed by the {what} anyway.",
    "Toasted crumpets by the {what}. Burnt half of them.",
    "Brought my knitting to the {what}. Three rows done, and a lot of gossip.",
    "The sea was flat as glass tonight. We just sat by the {what} and looked.",
    "Somebody's dog fell asleep on my feet by the {what}.",
    "Told the old story about the big catch by the {what}. It's bigger every time.",
    "Shared a pot of mussels by the {what}. Not a scrap left.",
    "Watched the moon come up over the water from the {what}.",
    "Spotted the first star by the {what}. I made a wish. Not telling.",
    "The tide went out and took the chatter with it. Lovely evening by the {what}.",
    "Rain came on, so we squeezed in close by the {what}. Nobody left.",
    "A fishing song by the {what}. Nobody could reach the high bit.",
    "Someone brought warm bread from the bakery. We ate it by the {what}.",
    "By the {what}, everyone talks a little softer.",
    "Ended the day by the {what}, salt on my lips and nothing on my mind.",
];

/// The same, of an evening spent with someone in particular.
const GATHER_WITH: [&str; 22] = [
    "{other} taught me a card trick by the {what}. I can't do it.",
    "Watched the boats' lights from the {what} with {other}.",
    "{other} and I shared a pie by the {what}. Best supper all week.",
    "{other} brought blankets. We stayed by the {what} past midnight.",
    "{other} told me a story I'd never heard.",
    "Me and {other}, putting the world to rights.",
    "{other} laughed so hard they cried.",
    "Didn't feel the cold, talking to {other}.",
    "You learn a lot about {other} after dark.",
    "{other} sang. Badly. We all joined in.",
    "Moths round the {what}, and {other} naming every one.",
    "{other} and I watched the stars come out by the {what}.",
    "{other} told me a secret by the {what}. My lips are sealed.",
    "{other} and I got talking and never stopped.",
    "{other} brought a fiddle. The {what} brought the rest of us.",
    "{other} and I watched the lamps come on along the quay.",
    "{other} brought chips from the Anchor. We ate them by the {what}.",
    "The tide came right up while {other} and I talked by the {what}.",
    "{other} knows every boat by its lights. Showed off all night.",
    "{other} and I swapped ghost stories by the {what}. I'm sleeping with a lamp on.",
    "{other} skimmed stones by the {what} till it was too dark to see them.",
    "{other} and I watched the last boat home from the {what}.",
];

/// What people at the harbour say bringing the player what their garden
/// grew.
const HARVEST_LINES: [&str; 11] = [
    "From your {what}. Seemed only fair.",
    "The {what} did well this week. This is yours.",
    "First pick from your {what}.",
    "Your {what} keeps giving. Here.",
    "Picked these from your {what} this morning.",
    "Don't tell anyone, but your {what} beats mine.",
    "From your {what}, with the dew still on.",
    "The gulls had a go at your {what}. They missed these.",
    "Brought you the best of your {what}. I kept the second best.",
    "Your {what} smells of summer. Take some home.",
    "Sea air suits your {what}. Here's the proof.",
];

/// The lines said of anything at all, as templates: a rest, an evening, a
/// harvest. Filler, by the words harness.
#[cfg(test)]
pub(crate) fn filler() -> Vec<&'static str> {
    REST_LINES
        .iter()
        .chain(&GATHER_ALONE)
        .chain(&GATHER_WITH)
        .chain(&HARVEST_LINES)
        .copied()
        .collect()
}

/// Of how many uses of a kind one is spoken of aloud: a rest one in
/// three, as an evening with someone; an evening alone or a harvest one
/// in two; anything else every time.
pub(crate) fn said_every(effect: &str, with_other: bool) -> usize {
    match (effect, with_other) {
        ("rest", _) | ("gather", true) => 3,
        ("gather", false) | ("harvest", _) => 2,
        _ => 1,
    }
}

/// What someone says using something the player made, in the harbour's
/// own words: each use of a kind spoken of says the next of its lines, so
/// none is heard again until every other has been, and an empty line for
/// a use not spoken of ([`said_every`]). `None` for anything else, or when
/// what was used is gone and cannot be named.
pub(crate) fn enjoyed_line(world: &World, event: &world_core::Event) -> Option<(EntityId, String)> {
    if event.kind != "enjoyed" {
        return None;
    }
    let state = world.state();
    let effect = match event.payload.get("effect") {
        Some(Value::Text(effect)) => effect.as_str(),
        _ => return None,
    };
    let with_other = event.targets.len() > 1;
    let lines: &[&str] = match effect {
        "rest" => &REST_LINES,
        "gather" if with_other => &GATHER_WITH,
        "gather" => &GATHER_ALONE,
        "harvest" => &HARVEST_LINES,
        _ => return None,
    };
    let thing = *event.targets.first()?;
    let named = lives::name(state, state.entity(thing).map(|_| thing)?);
    // A plant is spoken of by what it is ("your vegetable garden"), not
    // the stage it has grown to ("vegetables in flower").
    let what = THINGS
        .iter()
        .find(|made| made.stages.iter().any(|(stage, _)| *stage == named))
        .map_or(named.as_str(), |made| made.name)
        .to_lowercase();
    let place = match state.entity(thing).and_then(|thing| thing.component("at")) {
        Some(Value::Entity(at)) => lives::name(state, *at),
        _ => "the harbour".into(),
    };
    let other = event
        .targets
        .get(1)
        .map(|other| lives::first_name(state, *other));
    // How many times something was used this way before: the next line,
    // said only now and then of a rest or a quiet evening, so the bench
    // never becomes what the harbour talks about most. Said nothing (an
    // empty line) the other times.
    let before = enjoyed_before(world, event)?;
    let every = said_every(effect, with_other);
    if before % every != 0 {
        return Some((event.actor?, String::new()));
    }
    let line = lines.get(before / every % lines.len().max(1))?;
    Some((
        event.actor?,
        line.replace("{what}", &what)
            .replace("{place}", &place)
            .replace("{other}", other.as_deref().unwrap_or("")),
    ))
}

/// For each `enjoyed` event, how many times something was used the same
/// way before it (its effect, alone or with someone), worked out once for
/// where the World stands rather than for every line told: counting it
/// line by line read the whole history for each, and was half of a
/// three-year harbour's first look after a day (H, v0.27). Each way keeps
/// its events, oldest first, so an event's place among them is the count;
/// and as the World moves on only the events recorded since are sorted in
/// (a kept view's history only grows: going back forgets every view), so a
/// turn no longer reads three years of them again (H2, v0.28).
struct EnjoyedBefore {
    standing: world_core::Standing,
    /// How many of the World's `enjoyed` events are sorted into `ways`.
    counted: usize,
    /// Each way's `enjoyed` events, oldest first.
    ways: Vec<(EnjoyedWay, Vec<world_core::EventId>)>,
}

/// A way something was used: its effect, and whether with someone.
type EnjoyedWay = (Option<Value>, bool);

fn enjoyed_way(event: &world_core::Event) -> EnjoyedWay {
    (
        event.payload.get("effect").cloned(),
        event.targets.len() > 1,
    )
}

fn enjoyed_before(world: &World, event: &world_core::Event) -> Option<usize> {
    let standing = world.standing();
    let kept = world.derived::<EnjoyedBefore>(|kept| match kept {
        Some(kept) if kept.standing == standing => kept,
        kept => {
            let index = world.history_index();
            let enjoyed = index.of_kind("enjoyed");
            let (counted, mut ways) = match kept {
                Some(kept) if kept.counted <= enjoyed.len() => (kept.counted, kept.ways.clone()),
                _ => (0, Vec::new()),
            };
            for used in enjoyed[counted..].iter().filter_map(|id| world.event(*id)) {
                let way = enjoyed_way(used);
                match ways.iter_mut().find(|(known, _)| *known == way) {
                    Some((_, events)) => events.push(used.id),
                    None => ways.push((way, vec![used.id])),
                }
            }
            std::sync::Arc::new(EnjoyedBefore {
                standing,
                counted: enjoyed.len(),
                ways,
            })
        }
    });
    let way = enjoyed_way(event);
    let (_, events) = kept.ways.iter().find(|(known, _)| *known == way)?;
    events.binary_search(&event.id).ok()
}

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
        assert!(
            enjoyed_line(branch.world(), rested).is_some(),
            "in the harbour's own words"
        );
        let replayed = branch.world().replay().unwrap();
        assert_eq!(replayed.state(), branch.world().state());
    }
}
