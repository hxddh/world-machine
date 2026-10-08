//! Lives as a World shows them: a card for each answer to each situation
//! open now, the letters the player has been written, and the guests
//! staying.

use crate::Cast;
use world_core::World;
use world_projection::{Letter, ProjectionCommand, Question, SelectionId};

/// A card for each answer to each situation open now, each command
/// `{prefix}{situation}.{answer}`.
pub fn situation_commands(world: &World, cast: &Cast, prefix: &str) -> Vec<ProjectionCommand> {
    crate::situations(world, cast)
        .into_iter()
        .flat_map(|situation| {
            let question = Question {
                id: format!("life.{}", situation.key),
                prompt: situation.prompt.clone(),
            };
            situation
                .answers
                .into_iter()
                .map(move |answer| ProjectionCommand {
                    id: format!("{prefix}{}.{}", situation.key, answer.id),
                    title: answer.title,
                    detail: situation.told.clone(),
                    effects: Vec::new(),
                    scenery: None,
                    asker: Some(SelectionId::Entity(situation.asker)),
                    moves: Vec::new(),
                    question: Some(question.clone()),
                    unavailable: answer.unavailable,
                    hand: None,
                    preview: None,
                    role: None,
                })
        })
        .collect()
}

/// Letters the player has been written, oldest first, for the letter box.
pub fn letters_shown(world: &World) -> Vec<Letter> {
    crate::letters(world)
        .into_iter()
        .map(|letter| Letter {
            from: SelectionId::Entity(letter.from),
            note: letter.note,
            moment: SelectionId::Event(letter.event),
        })
        .collect()
}

/// The guests staying in this World, as the canvas stands them.
pub fn guests_staying(world: &World, cast: &Cast) -> Vec<world_projection::Staying> {
    crate::guests_staying(world, cast)
        .into_iter()
        .map(|guest| world_projection::Staying {
            visit: guest.visit,
            name: guest.name,
            from: guest.from,
            line: guest.line,
            look: guest.look,
            drawing: guest.drawing,
        })
        .collect()
}

/// What a visit tells this World of a guest.
pub fn guest_words<'a>(
    guest: &'a world_projection::Guest,
    look: &'a Option<String>,
    drawing: &'a Option<String>,
) -> crate::GuestWords<'a> {
    crate::GuestWords {
        name: &guest.name,
        from: &guest.from,
        letter: &guest.letter,
        gift: &guest.gift,
        line: guest.line.as_deref(),
        look: look.as_deref(),
        drawing: drawing.as_deref(),
    }
}

/// The person a memorial command (`{prefix}{who}` or `{prefix}{who}@{spot}`)
/// is for, and the spot the player chose.
pub fn memorial_target(
    prefix: &str,
    command_id: &str,
) -> Option<(world_core::EntityId, Option<u8>)> {
    let rest = command_id.strip_prefix(prefix)?;
    let (who, spot) = match rest.split_once('@') {
        Some((who, spot)) => (who, Some(spot.parse::<u8>().ok()?.min(100))),
        None => (rest, None),
    };
    Some((world_core::EntityId::new(who.parse().ok()?), spot))
}

/// The words of a memorial card: what it is ("bench") and how it is put up.
pub struct MemorialWords<'a> {
    pub prefix: &'a str,
    pub what: &'a str,
    pub detail: &'a str,
    /// Where it is told as standing until the player places it.
    pub at: world_core::EntityId,
}

/// A card to put up a memorial for each person the World lost and has not
/// yet remembered: the player places it where they like.
pub fn memorial_commands(
    world: &World,
    kin: &crate::generations::Kin,
    words: &MemorialWords<'_>,
) -> Vec<ProjectionCommand> {
    let state = world.state();
    crate::awaiting_memorial(state)
        .into_iter()
        .map(|who| {
            let named = crate::generations::memorial_name(state, kin, who, 0);
            ProjectionCommand {
                id: format!("{}{}", words.prefix, who.0),
                title: format!("A {} for {}", words.what, crate::first_name(state, who)),
                detail: words.detail.into(),
                effects: Vec::new(),
                scenery: None,
                asker: None,
                moves: Vec::new(),
                question: None,
                unavailable: None,
                hand: Some(world_projection::Hand {
                    verb: "Build".into(),
                    thing: named,
                    at: Some(SelectionId::Entity(words.at)),
                    cost: None,
                }),
                preview: None,
                role: None,
            }
        })
        .collect()
}
