//! What the pair say and how they look: a line over whoever a moment
//! happened to, the answers to what a player can ask them, and the colours
//! they wear. All of it is written from what the World records, so it is
//! the same every time the World is replayed, and none of it is ever read
//! back as World state.

use crate::{seed_id, RELATIONSHIP, RELATIONSHIP_TENSION, RELATIONSHIP_TRUST, SLOT_B, SLOT_E};
use world_core::{EntityId, Event, Value, World};
use world_projection::{entity_title, Carry, Look, ProjectionCommand, SelectionId, Talk, Voice};

/// The first word of someone's name, the way people address each other.
#[cfg(test)]
pub(crate) fn first_name_for_test(world: &World, id: EntityId) -> String {
    first_name(world, id)
}

fn first_name(world: &World, id: EntityId) -> String {
    world
        .state()
        .entity(id)
        .map(entity_title)
        .and_then(|name| name.split_whitespace().next().map(str::to_string))
        .unwrap_or_else(|| "them".into())
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
        ..Look::default()
    };
    let bird = |scarf: u32, carries: Option<Carry>| Look {
        clothes: Some(scarf),
        hair: None,
        skin: None,
        carries,
        bird: true,
        ..Look::default()
    };
    Some(match (seed_id(world), id) {
        ("mars-colony", SLOT_B) => look(0x2f7f86, 0x2b1d14, 0xc68a5f, Carry::Tool),
        ("mars-colony", SLOT_E) => look(0xd9772b, 0x6b4a2f, 0xe0b18a, Carry::Satchel),
        ("1980s-town", SLOT_B) => look(0x7b4bb3, 0x1e1a18, 0xb57a52, Carry::Book),
        ("1980s-town", SLOT_E) => look(0x2f6fb0, 0xc98f45, 0xf0c49c, Carry::Mug),
        ("penguin-civilization", SLOT_B) => bird(0xd64545, None),
        ("penguin-civilization", SLOT_E) => bird(0x3a8fd6, Some(Carry::Fish)),
        ("mars-colony", crate::story::NEWCOMER) => look(0x6a7f3a, 0x1a1414, 0x8d5a3b, Carry::Tool),
        ("1980s-town", crate::story::NEWCOMER) => look(0xc8553d, 0x3a2418, 0xe0b18a, Carry::Mug),
        ("penguin-civilization", crate::story::NEWCOMER) => bird(0xe0a33a, None),
        // Anyone else who came to stay, in colours of their own.
        (seed, id) if world.state().entity(id).is_some() => {
            const COLOURS: [u32; 6] = [0x5b7fa6, 0xb5654a, 0x6a8f4e, 0x9c6fb0, 0xd49a3a, 0x4a8f8f];
            let n = id.0 as usize;
            if seed == "penguin-civilization" {
                bird(COLOURS[n % COLOURS.len()], None)
            } else {
                look(
                    COLOURS[n % COLOURS.len()],
                    [0x2a1d14, 0x7a4a26, 0xc9a45a][n / 3 % 3],
                    [0xf0c7a2, 0xd9a27a, 0xa86b45, 0x7a4a2e][n / 5 % 4],
                    Carry::Satchel,
                )
            }
        }
        _ => return None,
    })
}

/// Who says the line for a moment, and what they say.
fn said(world: &World, event: &Event) -> Option<(EntityId, String)> {
    if let Some(said) = conversation::favour::said(world.state(), event) {
        world.state().entity(said.0)?;
        return Some(said);
    }
    if let Some(said) = crate::story::line(world, event) {
        world.state().entity(said.0)?;
        return Some(said);
    }
    match event.kind.as_str() {
        "universe_seeded" => {
            world.state().entity(SLOT_B)?;
            Some((SLOT_B, "Right. Let's make this place a home.".to_string()))
        }
        _ => None,
    }
}

/// Everything the pair said worth drawing: every moment of the story, and
/// the everyday things they said at the latest moment. Older everyday
/// chatter is left out; nobody scrolls back through it.
pub(crate) fn voices(world: &World) -> Vec<Voice> {
    let latest = world.world_time();
    // Only moments History can show are ever retold, so only they are read.
    let events = world.events();
    let mut voices = Vec::new();
    for event in events[events
        .len()
        .saturating_sub(world_projection::TIMELINE_EVENTS)..]
        .iter()
        .filter(|event| !crate::projection::is_routine(&event.kind) || event.world_time == latest)
    {
        let Some((speaker, line)) = said(world, event) else {
            continue;
        };
        let voice = Voice {
            moment: SelectionId::Event(event.id),
            speaker: SelectionId::Entity(speaker),
            line,
        };
        // A newcomer hears hello before anything else said that day.
        if event.kind == "greeted" && event.world_time == latest {
            voices.insert(0, voice);
        } else {
            voices.push(voice);
        }
    }
    voices
}

/// What someone would ask for, among the choices on offer now, and how
/// they would put it.
pub(crate) fn request(
    world: &World,
    who: EntityId,
    commands: &[ProjectionCommand],
) -> Option<(String, Option<String>)> {
    let (line, grant) = crate::story::wanting(world, who)?;
    let grant = grant.filter(|id| commands.iter().any(|command| &command.id == id));
    Some((line, grant))
}

/// What a player can ask each of the pair, and what they answer, from how
/// things stand: how they are, what they make of each other, what they need.
pub(crate) fn talks(world: &World, commands: &[ProjectionCommand]) -> Vec<Talk> {
    if seed_id(world) == "unseeded" {
        return Vec::new();
    }
    let trust = integer(world, RELATIONSHIP, RELATIONSHIP_TRUST);
    let tension = integer(world, RELATIONSHIP, RELATIONSHIP_TENSION);
    let mut talks = Vec::new();
    for (who, other) in [(SLOT_B, SLOT_E), (SLOT_E, SLOT_B)] {
        if world.state().entity(who).is_none() {
            continue;
        }
        let other_name = first_name(world, other);
        let person = SelectionId::Entity(who);
        let (granted, grudges) = crate::story::kindness(world, who);
        let how = if let Some(how) = lives::how_are_you(world, who) {
            how
        } else if grudges > granted && !crate::arrival::first_days(world.state()) {
            "Sore. Nobody listens when I ask for anything.".into()
        } else if granted > grudges {
            "Good. I feel looked after here.".into()
        } else {
            "Still finding my feet.".into()
        };
        talks.push(Talk {
            who: person,
            question: "How are you?".into(),
            answer: how,
            asks_for: None,
        });
        let about = match () {
            _ if trust >= 8 && tension <= 3 => format!("I'd trust {other_name} with my life."),
            _ if tension >= 8 && trust <= 3 => format!("I'd rather not talk about {other_name}."),
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
    // Whoever else lives here: how they are, and what they make of the
    // keeper.
    for who in crate::life::people(world) {
        if who == SLOT_B || who == SLOT_E {
            continue;
        }
        let person = SelectionId::Entity(who);
        if let Some(how) = lives::how_are_you(world, who) {
            talks.push(Talk {
                who: person,
                question: "How are you?".into(),
                answer: how,
                asks_for: None,
            });
        }
        if let Some(about) = lives::thinks_of(world, who, SLOT_B) {
            talks.push(Talk {
                who: person,
                question: format!("What do you think of {}?", first_name(world, SLOT_B)),
                answer: about,
                asks_for: None,
            });
        }
    }
    talks
}
