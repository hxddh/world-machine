//! The harbour's people living their own lives: what they do with their
//! days, who they get on with, and the situations that come of it, told in
//! the harbour's words. The mechanics are the `lives` System's.

use crate::model::MISSED_SHIFTS;
use crate::story::{ADA, IVO};
use crate::{EMMA, EVAN, HARBOR, JONAS, LEO, MARA, MIA, NOAH, PUB, SOFIA};
use lives::{Activity, At, Cast, Fund, Need, Visitors, With};
use society_basic::{CASH, JOB};
use world_core::{EntityId, Value, World, WorldState};

/// The entity the harbour's lives are kept on.
pub(crate) const LIVES: EntityId = EntityId::new(402);

/// The first id a stranger who comes to stay takes.
pub(crate) const FIRST_VISITOR: u64 = 430;

const ACTIVITIES: &[Activity] = &[
    Activity {
        id: "shift",
        need: Need::Money,
        with: With::Workmate,
        at: At::Work,
        told: "{name} put in a long day at {place} with {other}",
        said: &[
            "Long day at {place}, but {other} kept me laughing.",
            "{other} and I got through a mountain of work.",
            "Busy at {place}. {other} never stopped talking.",
            "{other} brought biscuits to {place}. Saved the day.",
            "Rushed off our feet at {place}, {other} and me.",
            "{other} sang all afternoon at {place}. Badly.",
        ],
        gives: &[],
    },
    Activity {
        id: "extra",
        need: Need::Money,
        with: With::Alone,
        at: At::Work,
        told: "{name} took on extra work at {place}",
        said: &[
            "Took on a bit extra at {place}.",
            "Worked through my dinner at {place}.",
            "Stayed late at {place}. Again.",
            "Last one out of {place} tonight.",
            "Sorted the back room at {place}. Found a ghost. Probably a cat.",
            "Extra hours at {place}. It all adds up.",
            "Swept out {place} after everyone left.",
            "Did the books at {place}. They nearly balance.",
        ],
        gives: &[],
    },
    Activity {
        id: "errands",
        need: Need::Money,
        with: With::Anyone,
        at: At::Work,
        told: "{name} ran errands for {other}",
        said: &[
            "Ran about for {other} all day. Paid, mind.",
            "{other} had me fetching and carrying.",
            "Up and down the hill for {other}. My legs!",
            "{other} pays in coins and cake. Fair enough.",
        ],
        gives: &[],
    },
    Activity {
        id: "stall",
        need: Need::Money,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} sold odds and ends outside {place}",
        said: &[
            "Sold my old things. {other} bought the lamp!",
            "Haggled with {other} for half an hour over a teapot.",
            "{other} took my chipped jug off my hands.",
            "Sold three jars of jam. {other} bought two.",
        ],
        gives: &[],
    },
    Activity {
        id: "lie_in",
        need: Need::Rest,
        with: With::Alone,
        at: At::Home,
        told: "{name} slept in",
        said: &[
            "Slept in. Don't tell anyone.",
            "First lie-in in weeks.",
            "Woke up at ten. Glorious.",
            "Slept in. {friend} knocked twice and gave up.",
            "Lie-in. {friend} will never let me hear the end of it.",
        ],
        gives: &[],
    },
    Activity {
        id: "quay",
        need: Need::Rest,
        with: With::Alone,
        at: At::Quiet,
        told: "{name} sat on the quay watching the boats",
        said: &[
            "Sat on the quay and watched the boats come in.",
            "Nothing but gulls and waves. Perfect.",
            "Dozed off on a bench by the water.",
            "Watched the tide turn. {friend} waved from a boat.",
            "Counted the boats in. All home, thank goodness.",
            "Sat on the harbour wall till my tea went cold.",
        ],
        gives: &[],
    },
    Activity {
        id: "cliff_walk",
        need: Need::Rest,
        with: With::Friend,
        at: At::Quiet,
        told: "{name} walked the cliff path with {other}",
        said: &[
            "Walked the cliff path with {other}.",
            "{other} and I walked till the light went.",
            "Saw a seal off the point. {other} didn't believe me.",
        ],
        gives: &[],
    },
    Activity {
        id: "night_in",
        need: Need::Rest,
        with: With::Alone,
        at: At::Home,
        told: "{name} had a quiet night in",
        said: &[
            "Early night. Bliss.",
            "A book and a pot of tea. Heaven.",
            "Stayed in. {friend} can have the pub to themselves.",
            "Fire lit, feet up, radio on.",
            "Mended my socks by the fire. Riveting.",
            "Wrote a letter to {friend}'s cousin on the mainland.",
        ],
        gives: &[],
    },
    Activity {
        id: "pint",
        need: Need::Company,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} had a drink with {other} at {place}",
        said: &[
            "A pint with {other} at {place}.",
            "{other} told the old stories again. Still funny.",
            "Closing time came too soon, {other} says.",
        ],
        gives: &[],
    },
    Activity {
        id: "cards",
        need: Need::Company,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} played cards with {other} at {place}",
        said: &[
            "{other} cheats at cards. I'm sure of it.",
            "Won three hands off {other}!",
            "Lost my shirt to {other} at cards.",
        ],
        gives: &[],
    },
    Activity {
        id: "tea",
        need: Need::Company,
        with: With::Friend,
        at: At::Home,
        told: "{name} had {other} round for tea",
        said: &[
            "{other} came round for tea.",
            "Tea and gossip with {other}.",
            "{other} brought cake. I didn't ask why.",
        ],
        gives: &[],
    },
    Activity {
        id: "dance",
        need: Need::Company,
        with: With::Anyone,
        at: At::Gathering,
        told: "{name} danced with {other} at {place}",
        said: &[
            "Danced with {other}. My poor feet!",
            "{other} can't dance, but tries.",
        ],
        gives: &[],
    },
    Activity {
        id: "chat",
        need: Need::Company,
        with: With::Anyone,
        at: At::Quiet,
        told: "{name} got talking with {other} down by the water",
        said: &[
            "Got talking with {other} down by the water.",
            "{other} and I put the world to rights.",
        ],
        gives: &[],
    },
    Activity {
        id: "lend_a_hand",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Work,
        told: "{name} lent {other} a hand at {place}",
        said: &[
            "Gave {other} a hand at {place}.",
            "{other} needed help. Glad to.",
        ],
        gives: &[],
    },
    Activity {
        id: "knots",
        need: Need::Purpose,
        with: With::Anyone,
        at: At::Quiet,
        told: "{name} taught {other} a sailor's knot",
        said: &[
            "Taught {other} a bowline. Took four tries.",
            "{other} can tie a sheepshank now!",
        ],
        gives: &[],
    },
    Activity {
        id: "paint",
        need: Need::Purpose,
        with: With::Alone,
        at: At::Quiet,
        told: "{name} painted the harbour",
        said: &[
            "Painted the harbour. The gulls won't sit still.",
            "My painting's coming on. Slowly.",
            "Painted the lighthouse. {friend} says it leans.",
            "Tried painting the sea. It's harder than it looks.",
            "Painted {friend}'s boat. They want it for the wall.",
        ],
        gives: &[],
    },
    Activity {
        id: "vegetables",
        need: Need::Purpose,
        with: With::Alone,
        at: At::Home,
        told: "{name} dug over a vegetable patch",
        said: &[
            "Dug over the vegetable patch.",
            "Planted beans. We'll see.",
            "Slugs ate my lettuces. {friend} says beer traps.",
            "Potatoes in. {friend} lent me a spade.",
            "Weeded till my back gave out.",
        ],
        gives: &[],
    },
    Activity {
        id: "read",
        need: Need::Purpose,
        with: With::Alone,
        at: At::Home,
        told: "{name} read late into the night",
        said: &[
            "Reading a book about lighthouses.",
            "Couldn't put my book down.",
            "{friend} lent me a book about shipwrecks. Gripping.",
            "Read about the old smugglers' caves. Fancy that.",
            "Finished my book. Now what?",
        ],
        gives: &[],
    },
    Activity {
        id: "plan",
        need: Need::Purpose,
        with: With::Friend,
        at: At::Gathering,
        told: "{name} planned something at {place} with {other}",
        said: &[
            "{other} and I are planning a quiz night.",
            "Making posters with {other}. Don't ask what for.",
        ],
        gives: &[],
    },
];

const TOPICS: &[&str] = &[
    "the mooring fees",
    "a torn net",
    "a market pitch",
    "the council vote",
    "the regatta",
    "a dog and the bins",
    "money owed",
    "the noise from the pub",
    "mending the slipway",
    "a card game",
    "the ferry timetable",
    "a leaking roof",
    "a spot on the quay",
    "the harvest supper",
    "a broken promise",
    "fishing off the point",
    "the price of bread",
    "the church bell",
    "a garden wall",
    "an old argument",
    "a borrowed ladder",
    "the school play",
    "the fête",
    "a lost letter",
];

const OUTINGS: &[&str] = &[
    "walk out to the point",
    "the dance at the Anchor",
    "watch the boats come in",
    "a picnic on the cliff",
    "the harvest supper",
    "see the seals at the rocks",
    "row out to the islet",
    "the Saturday market",
];

const VISITOR_NAMES: &[&str] = &[
    "Rosa", "Tobias", "Hana", "Olek", "Maeve", "Arun", "Lise", "Pim", "Greta", "Kofi", "Ines",
    "Bram", "Nell", "Soren", "Yara", "Dario", "Ffion", "Mateo", "Wren", "Anouk", "Casimir",
    "Lotte", "Ravi", "Esme",
];

const TRADES: &[(&str, &str)] = &[
    ("net mender", "net_mender"),
    ("fiddler", "musician"),
    ("painter", "painter"),
    ("nurse", "nurse"),
    ("boat builder", "boat_builder"),
    ("cheesemaker", "cheesemaker"),
    ("potter", "potter"),
    ("radio operator", "radio_operator"),
    ("gardener", "gardener"),
    ("tutor", "tutor"),
];

const ORIGINS: &[&str] = &[
    "the mainland",
    "Cornwall",
    "a trawler out of Hull",
    "Brittany",
    "the northern isles",
    "the city",
    "Norway",
    "a farm inland",
    "Galway",
    "the lighthouse service",
];

fn visitor(name: &str, job: &str) -> Vec<(String, Value)> {
    let _ = name;
    vec![
        (CASH.into(), Value::from(40_i64)),
        (JOB.into(), Value::from(job)),
        ("location".into(), Value::Entity(HARBOR)),
        (MISSED_SHIFTS.into(), Value::from(0_i64)),
        ("newcomer".into(), Value::from(true)),
    ]
}

fn traits(person: EntityId) -> Option<[&'static str; 2]> {
    Some(match person {
        JONAS => ["steady", "proud"],
        MARA => ["warm", "proud"],
        LEO => ["sociable", "generous"],
        EMMA => ["warm", "restless"],
        MIA => ["dreamy", "sociable"],
        NOAH => ["steady", "prickly"],
        EVAN => ["restless", "generous"],
        SOFIA => ["proud", "thrifty"],
        ADA => ["sociable", "dreamy"],
        IVO => ["shy", "steady"],
        _ => return None,
    })
}

/// Where someone works: the place their job is, or where they spend their
/// days.
pub(crate) fn work(state: &WorldState, person: EntityId) -> Option<EntityId> {
    state
        .relations()
        .find(|relation| relation.kind == "works_at" && relation.from == person)
        .map(|relation| relation.to)
        .or_else(|| match state.entity(person)?.component("location")? {
            Value::Entity(place) => Some(*place),
            _ => None,
        })
}

fn home(_: &WorldState, _: EntityId) -> Option<EntityId> {
    None
}

/// The first eight have lived here all their lives and never leave.
fn stays(person: EntityId) -> bool {
    crate::talk::RESIDENTS.contains(&person)
}

pub(crate) fn cast() -> Cast {
    Cast {
        notes: LIVES,
        period: crate::persistence::WORLD_DAY_TICKS,
        unit: "week",
        settlement: "harbour",
        short_of: "money",
        people: crate::story::people,
        stays,
        traits,
        kept: |_, _| false,
        mood: |state| crate::story::spirits_of(state),
        work,
        home,
        gathering: PUB,
        quiet: HARBOR,
        host: LEO,
        activities: ACTIVITIES,
        topics: TOPICS,
        outings: OUTINGS,
        fund: Some(Fund {
            entity: HARBOR,
            key: CASH,
            name: "the harbour fund",
            amount: 30,
        }),
        visitors: Some(Visitors {
            first: FIRST_VISITOR,
            room: 200,
            names: VISITOR_NAMES,
            trades: TRADES,
            origins: ORIGINS,
            way_out: "the ferry",
            components: visitor,
            kind: "resident",
        }),
        most_people: 14,
        most_open: 1,
    }
}

const LIFE_COMMAND: &str = "tiny-society.life.";

/// The situation and answer a command gives, if it is one of these.
pub(crate) fn parse_command(command_id: &str) -> Option<(&str, &str)> {
    command_id.strip_prefix(LIFE_COMMAND)?.rsplit_once('.')
}

/// A card for each answer to each situation open now.
pub(crate) fn commands(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let cast = cast();
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
                })
        })
        .collect()
}

/// What people have given the player to keep, oldest first, for the
/// drawer.
pub(crate) fn keepsakes(world: &World) -> Vec<world_projection::Keepsake> {
    lives::keepsakes(world)
        .into_iter()
        .map(|kept| world_projection::Keepsake {
            from: world_projection::SelectionId::Entity(kept.from),
            what: kept.what,
            note: kept.note,
            moment: world_projection::SelectionId::Event(kept.event),
        })
        .collect()
}
