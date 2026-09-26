//! What the pair say and how they look: a line over whoever a moment
//! happened to, the answers to what a player can ask them, and the colours
//! they wear. All of it is written from what the World records, so it is
//! the same every time the World is replayed, and none of it is ever read
//! back as World state.

use crate::{
    seed_id, BOLD_PATH_COMMAND, CAREFUL_PATH_COMMAND, ENTRUST_LEGACY_COMMAND,
    HOLD_PRESSURE_COMMAND, OUTWARD_POSTURE_COMMAND, REACH_PRESSURE_COMMAND, RECOVER_ANCHOR_COMMAND,
    RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC, RELATIONSHIP_TENSION, RELATIONSHIP_TRUST,
    ROOTED_POSTURE_COMMAND, SHARED_PROJECT_COMMAND, SLOT_A, SLOT_B, SLOT_D, SLOT_E,
};
use world_core::{EntityId, Event, Value, World};
use world_projection::{entity_title, Carry, Look, ProjectionCommand, SelectionId, Talk, Voice};

/// The first word of someone's name, the way people address each other.
fn first_name(world: &World, id: EntityId) -> String {
    world
        .state()
        .entity(id)
        .map(entity_title)
        .and_then(|name| name.split_whitespace().next().map(str::to_string))
        .unwrap_or_else(|| "them".into())
}

fn title(world: &World, id: EntityId) -> String {
    world
        .state()
        .entity(id)
        .map(entity_title)
        .unwrap_or_else(|| "home".into())
}

fn text(world: &World, id: EntityId, key: &str) -> Option<String> {
    match world.state().entity(id)?.component(key)? {
        Value::Text(value) => Some(value.clone()),
        _ => None,
    }
}

fn integer(world: &World, id: EntityId, key: &str) -> i64 {
    match world
        .state()
        .entity(id)
        .and_then(|entity| entity.component(key))
    {
        Some(Value::Integer(value)) => *value,
        _ => 0,
    }
}

/// How each of the pair looks in each place: work clothes on Mars, jackets
/// on Maple Street, and on Icebridge two penguins in coloured scarves.
pub(crate) fn look(world: &World, id: EntityId) -> Option<Look> {
    let look = |clothes: u32, hair: u32, skin: u32, carries: Carry| Look {
        clothes: Some(clothes),
        hair: Some(hair),
        skin: Some(skin),
        carries: Some(carries),
        bird: false,
    };
    let bird = |scarf: u32, carries: Option<Carry>| Look {
        clothes: Some(scarf),
        hair: None,
        skin: None,
        carries,
        bird: true,
    };
    Some(match (seed_id(world), id) {
        ("mars-colony", SLOT_B) => look(0x2f7f86, 0x2b1d14, 0xc68a5f, Carry::Tool),
        ("mars-colony", SLOT_E) => look(0xd9772b, 0x6b4a2f, 0xe0b18a, Carry::Satchel),
        ("1980s-town", SLOT_B) => look(0x7b4bb3, 0x1e1a18, 0xb57a52, Carry::Book),
        ("1980s-town", SLOT_E) => look(0x2f6fb0, 0xc98f45, 0xf0c49c, Carry::Mug),
        ("penguin-civilization", SLOT_B) => bird(0xd64545, None),
        ("penguin-civilization", SLOT_E) => bird(0x3a8fd6, Some(Carry::Fish)),
        _ => return None,
    })
}

/// Who says the line for a moment, and what they say. The keeper speaks
/// for home, the explorer for going out; what someone did, they say.
fn said(world: &World, event: &Event) -> Option<(EntityId, String)> {
    if let Some(said) = crate::story::line(world, event) {
        world.state().entity(said.0)?;
        return Some(said);
    }
    let seed = seed_id(world);
    let anchor = title(world, SLOT_A);
    let vehicle = title(world, SLOT_D);
    let doer = event.actor.filter(|id| [SLOT_B, SLOT_E].contains(id));
    let (speaker, line) = match event.kind.as_str() {
        "universe_seeded" => (SLOT_B, "Right. Let's make this place a home.".to_string()),
        "agent_cared_for_world" | "agent_explored_world" => {
            let doer = doer?;
            if let Some(memory) = crate::story::remembered(world, doer) {
                return Some((doer, memory));
            }
            let lines = if event.kind == "agent_cared_for_world" {
                match seed {
                    "mars-colony" => [
                        "Seals checked. Air's steady.".to_string(),
                        "The wheat's coming up green.".into(),
                        format!("Scrubbed the filters at {anchor}."),
                        "Water recycler's humming nicely.".into(),
                        "Swept the dust out of the airlock.".into(),
                    ],
                    "1980s-town" => [
                        format!("Lights are on at {anchor}."),
                        "Fixed the jammed coin slot.".into(),
                        "New high score on the board.".into(),
                        "Mopped the floor, again.".into(),
                        "The regulars are in tonight.".into(),
                    ],
                    _ => [
                        "The ice is holding. The vault is full.".to_string(),
                        "Lanterns trimmed and lit.".into(),
                        "Patched a crack in the bridge.".into(),
                        "Counted the fish twice.".into(),
                        "The chicks are all asleep.".into(),
                    ],
                }
            } else {
                match seed {
                    "mars-colony" => [
                        format!("Taking {vehicle} past the ridge."),
                        "Found a new way down the crater.".into(),
                        "The dunes moved again overnight.".into(),
                        "Picked up odd rocks by the ridge.".into(),
                        format!("{vehicle}'s running well today."),
                    ],
                    "1980s-town" => [
                        format!("Catching {vehicle} across town."),
                        "Found a record shop I'd never seen.".into(),
                        "Took the long way home.".into(),
                        "Somebody called in a song request.".into(),
                        "Walked the whole Maple Loop.".into(),
                    ],
                    _ => [
                        "Going out over the ice.".to_string(),
                        "Saw a whale past the floe.".into(),
                        "Found a new fishing hole.".into(),
                        "The wind's changed out there.".into(),
                        "Slid all the way down the ridge!".into(),
                    ],
                }
            };
            let today = (event.world_time / crate::BACKGROUND_PERIOD % 5) as usize;
            (doer, lines[today].clone())
        }
        "partnership_formed" => (SLOT_E, "Let's do the next part together.".into()),
        "relationship_fractured" => (SLOT_B, "Fine. Go on without me.".into()),
        "universe_grew" => (
            doer.unwrap_or(SLOT_B),
            [
                "Look at that. We built it.",
                "A little bigger every day.",
                "It's starting to feel like home.",
                "Not bad for two of us.",
                "Another piece in place.",
            ][(event.world_time / crate::BACKGROUND_PERIOD % 5) as usize]
                .into(),
        ),
        "pressure_rising" => (SLOT_B, format!("Something's wrong with {anchor}.")),
        "pressure_peaked" => (SLOT_B, format!("{anchor} can't take much more!")),
        "pressure_held" => (SLOT_B, "We held it. We actually held it!".into()),
        "pressure_reached" => (SLOT_E, "I found a way through.".into()),
        "anchor_lost" => (SLOT_B, format!("We've lost {anchor}.")),
        "anchor_recovered" => (SLOT_B, format!("{anchor} is ours again.")),
        "world_posture_chosen" => (doer.unwrap_or(SLOT_E), "Then that's the way we go.".into()),
        "world_legacy_formed" => (SLOT_B, "This is how they'll remember us.".into()),
        "era_began" => (SLOT_E, "It feels like a new chapter.".into()),
        _ => return None,
    };
    world.state().entity(speaker)?;
    Some((speaker, line))
}

/// Everything the pair said worth drawing: every moment of the story, and
/// the everyday things they said at the latest moment. Older everyday
/// chatter is left out; nobody scrolls back through it.
pub(crate) fn voices(world: &World) -> Vec<Voice> {
    let latest = world.world_time();
    world
        .events()
        .iter()
        .filter(|event| !crate::projection::is_routine(&event.kind) || event.world_time == latest)
        .filter_map(|event| {
            let (speaker, line) = said(world, event)?;
            Some(Voice {
                moment: SelectionId::Event(event.id),
                speaker: SelectionId::Entity(speaker),
                line,
            })
        })
        .collect()
}

/// What someone would ask for, among the choices on offer now, and how
/// they would put it.
fn request(
    world: &World,
    who: EntityId,
    commands: &[ProjectionCommand],
) -> Option<(String, Option<String>)> {
    if let Some((line, grant)) = crate::story::wanting(world, who) {
        let grant = grant.filter(|id| commands.iter().any(|command| &command.id == id));
        return Some((line, grant));
    }
    let anchor = title(world, SLOT_A);
    let other = first_name(world, if who == SLOT_B { SLOT_E } else { SLOT_B });
    let theirs = |command: &&ProjectionCommand| {
        command.asker == Some(SelectionId::Entity(who))
            || command.asker == Some(SelectionId::Entity(RELATIONSHIP))
                && command.id == SHARED_PROJECT_COMMAND
    };
    let command = commands.iter().find(theirs)?;
    let line = match command.id.as_str() {
        SHARED_PROJECT_COMMAND => {
            format!("Something to build together. {other} and I could use that.")
        }
        BOLD_PATH_COMMAND => "Let me follow it. I'll be careful.".into(),
        CAREFUL_PATH_COMMAND => "Keep us close to home for now.".into(),
        OUTWARD_POSTURE_COMMAND => "Let us look outward.".into(),
        ROOTED_POSTURE_COMMAND => "Let us put down roots here.".into(),
        HOLD_PRESSURE_COMMAND => format!("Help me hold {anchor} together."),
        REACH_PRESSURE_COMMAND => "Let me go and find another way.".into(),
        RECOVER_ANCHOR_COMMAND => format!("Help me take {anchor} back."),
        ENTRUST_LEGACY_COMMAND => "Let someone carry this on after us.".into(),
        _ => return None,
    };
    Some((line, Some(command.id.clone())))
}

/// What a player can ask each of the pair, and what they answer, from how
/// things stand: how they are, what they make of each other, what they need.
pub(crate) fn talks(world: &World, commands: &[ProjectionCommand]) -> Vec<Talk> {
    if seed_id(world) == "unseeded" {
        return Vec::new();
    }
    let arc = text(world, RELATIONSHIP, RELATIONSHIP_SOCIAL_ARC).unwrap_or_default();
    let trust = integer(world, RELATIONSHIP, RELATIONSHIP_TRUST);
    let tension = integer(world, RELATIONSHIP, RELATIONSHIP_TENSION);
    let anchor = title(world, SLOT_A);
    let troubled = matches!(
        crate::pressure::pressure_id_from_state(world.state()).as_str(),
        "warning" | "crisis" | "lost"
    );
    let mut talks = Vec::new();
    for (who, other) in [(SLOT_B, SLOT_E), (SLOT_E, SLOT_B)] {
        if world.state().entity(who).is_none() {
            continue;
        }
        let other_name = first_name(world, other);
        let person = SelectionId::Entity(who);
        let (granted, grudges) = crate::story::kindness(world, who);
        let how = if troubled {
            format!("Worried. {anchor} needs us.")
        } else if grudges > granted {
            "Sore. Nobody listens when I ask for anything.".into()
        } else if granted > grudges {
            "Good. I feel looked after here.".into()
        } else {
            match text(world, who, "last_intent").as_deref() {
                Some("explore") => "Restless. There's more out there than we've seen.".into(),
                Some("care") => "Busy, but it's good work.".into(),
                _ => "Still finding my feet.".into(),
            }
        };
        talks.push(Talk {
            who: person,
            question: "How are you?".into(),
            answer: how,
            asks_for: None,
        });
        let about = match arc.as_str() {
            "partnership" => format!("I'd trust {other_name} with my life."),
            "fracture" => format!("I'd rather not talk about {other_name}."),
            _ if tension > trust => format!("{other_name} and I don't see things the same way."),
            _ if trust >= 4 => format!("{other_name}'s good. I'm glad we're in this together."),
            _ => format!("{other_name} and I are still getting to know each other."),
        };
        talks.push(Talk {
            who: person,
            question: format!("What do you think of {other_name}?"),
            answer: about,
            asks_for: None,
        });
        let (need, asks_for) = match request(world, who, commands) {
            Some((line, command)) => (line, command),
            None => ("Nothing right now. Just time.".to_string(), None),
        };
        talks.push(Talk {
            who: person,
            question: "What do you need?".into(),
            answer: need,
            asks_for,
        });
    }
    talks
}
