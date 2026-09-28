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
        voice: crate::voices::voice,
    }
}

/// What people say about their days in the harbour's later years, besides
/// what they always have: from its sixth year, then more from its ninth,
/// so the years after the first few are not told in the first few's words.
const LATER_SAID: &[(&str, [&str; 3], [&str; 3])] = &[
    (
        "shift",
        [
            "{other} and I have worked {place} so long we don't need to talk.",
            "Another year at {place} with {other}. Wouldn't change it.",
            "{other} taught the new ones the ropes at {place}.",
        ],
        [
            "{other} and I finish each other's sentences at {place} now.",
            "Remember when {other} was new at {place}? Look at them now.",
            "{other} made tea for all of {place}. First time in years.",
        ],
    ),
    (
        "errands",
        [
            "{other} had me up and down the lane all day.",
            "Fetched a parcel off the ferry for {other}.",
            "{other}'s list was longer than my arm.",
        ],
        [
            "Did {other}'s shopping. They'd have done the same.",
            "{other} paid me in cake. Fair trade.",
            "Carried {other}'s coal up the hill. My back knows it.",
        ],
    ),
    (
        "stall",
        [
            "Sold {other} a teapot with no lid. They were delighted.",
            "{other} haggled me down to nothing. Again.",
            "{other} bought the last jar of jam off my stall.",
        ],
        [
            "{other} said my stall's the best thing about Saturdays.",
            "Swapped {other} a lamp for a hat. Don't ask.",
            "{other} minded the stall while I had my dinner.",
        ],
    ),
    (
        "cliff_walk",
        [
            "Walked the cliffs with {other}. The wind took our words.",
            "{other} spotted a puffin. I only saw rocks.",
            "{other} and I sat on the top and said nothing for an hour.",
        ],
        [
            "The path's worn smooth where {other} and I always stop.",
            "{other} knows the name of every flower up there.",
            "Up the cliffs with {other} before breakfast. Worth it.",
        ],
    ),
    (
        "pint",
        [
            "{other} bought the first round. A first.",
            "{other} and I had our usual corner.",
            "Talked boats with {other} till last orders.",
        ],
        [
            "{other} told the same joke again. Still funny.",
            "One drink with {other}, they said. Three, it was.",
            "{other} and I drank to absent friends.",
        ],
    ),
    (
        "cards",
        [
            "{other} won every hand. I'm sure there's a trick.",
            "Taught {other} a new game. They beat me at it.",
            "{other} and I played for matchsticks. I owe them forty.",
        ],
        [
            "{other} cheats at cards. Everybody knows. We still play.",
            "Lost to {other} again. It's tradition now.",
            "{other} shuffles like a card sharp these days.",
        ],
    ),
    (
        "tea",
        [
            "{other} brought their own biscuits. Wise.",
            "Tea with {other}. The pot went cold, we talked so long.",
            "{other} told me their news over tea. I'll keep it.",
        ],
        [
            "{other} has a cup with their name on it at mine now.",
            "Tea with {other}, same as every week.",
            "{other} fixed my wobbly table while the kettle boiled.",
        ],
    ),
    (
        "dance",
        [
            "{other} trod on my toes. I trod on theirs.",
            "Danced with {other} till the fiddler gave up.",
            "{other} knows all the old steps.",
        ],
        [
            "{other} and I have a dance of our own now.",
            "Everyone stopped to watch {other} and me.",
            "{other} swung me round so fast I saw stars.",
        ],
    ),
    (
        "chat",
        [
            "{other} and I watched the tide turn, talking.",
            "{other} told me about the old harbour.",
            "Skimmed stones with {other}. They won.",
        ],
        [
            "{other} and I talked about the years gone by.",
            "{other} says the water's warmer than it was.",
            "Found {other} on the shingle. We put the world right.",
        ],
    ),
    (
        "lend_a_hand",
        [
            "Helped {other} mend a gate at {place}.",
            "{other} needed a hand with a heavy crate at {place}.",
            "Held the ladder for {other} at {place}.",
        ],
        [
            "{other} and I fixed the roof at {place} between us.",
            "Gave {other} a hand at {place}. They'll pay me back in pie.",
            "{other} and I shifted half of {place} this afternoon.",
        ],
    ),
    (
        "knots",
        [
            "{other} can tie a bowline with their eyes shut now.",
            "Showed {other} a knot my grandad taught me.",
            "{other} tied a knot I'd never seen. Where'd they learn that?",
        ],
        [
            "{other} is teaching the children knots now. Proud of that.",
            "{other} and I made a rope ladder. It holds.",
            "{other} still can't do a sheepshank. Nobody can.",
        ],
    ),
    (
        "plan",
        [
            "{other} and I have a plan for the spring. It's a secret.",
            "Sketched something with {other} at {place}. Watch this space.",
            "{other} has big ideas. I'm helping.",
        ],
        [
            "{other} and I are planning a party for the whole harbour.",
            "Drew up plans with {other} at {place}. Very official.",
            "{other} and I have been plotting. Don't worry. It's nice.",
        ],
    ),
];

/// The year of the harbour from which each of the later sets of words is
/// said.
const LATER_YEARS: [u64; 2] = [6, 9];

/// Everyone's days as the harbour's year has them: from its sixth year on,
/// with the later years' words added.
fn activities(state: &WorldState) -> &'static [Activity] {
    static LATER: std::sync::OnceLock<[Vec<Activity>; 2]> = std::sync::OnceLock::new();
    let year = crate::years::year(state);
    let tiers = LATER_YEARS.iter().filter(|from| year >= **from).count();
    if tiers == 0 {
        return ACTIVITIES;
    }
    let later = LATER.get_or_init(|| {
        [1, 2].map(|tiers| {
            ACTIVITIES
                .iter()
                .map(|activity| {
                    let mut activity = *activity;
                    if let Some((_, first, second)) =
                        LATER_SAID.iter().find(|(id, _, _)| *id == activity.id)
                    {
                        let mut said = activity.said.to_vec();
                        said.extend(first);
                        if tiers > 1 {
                            said.extend(second);
                        }
                        activity.said = Box::leak(said.into_boxed_slice());
                    }
                    activity
                })
                .collect()
        })
    });
    &later[tiers - 1]
}

/// The harbour's cast as its year has it: its people's days in the words
/// of the year it is.
pub(crate) fn cast_in(state: &WorldState) -> Cast {
    Cast {
        activities: activities(state),
        ..cast()
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
                    preview: None,
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

/// What the player can suggest the harbour does together.
pub(crate) const SUGGEST_COMMAND: &str = "tiny-society.suggest.";

/// Whether the weather is fair enough to be out in.
pub(crate) fn fair(world: &World) -> bool {
    use world_projection::Weather;
    !matches!(
        crate::story::weather(world),
        Weather::Rain | Weather::Storm | Weather::Snow
    )
}

/// A picnic, a market or a dance: the player suggests it, and the
/// harbour decides who comes and how it goes.
pub(crate) fn suggestions(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let state = world.state();
    let cast = cast();
    lives::IDEAS
        .iter()
        .map(|idea| world_projection::ProjectionCommand {
            id: format!("{SUGGEST_COMMAND}{}", idea.id),
            title: idea.name.into(),
            detail: if idea.outdoors {
                format!(
                    "Out by {}, if the weather holds",
                    lives::name(state, cast.quiet)
                )
            } else {
                format!("At {}", lives::name(state, cast.gathering))
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
                at: Some(world_projection::SelectionId::Entity(if idea.outdoors {
                    cast.quiet
                } else {
                    cast.gathering
                })),
                cost: None,
            }),
            preview: None,
        })
        .collect()
}
