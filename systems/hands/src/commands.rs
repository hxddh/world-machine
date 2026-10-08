//! The player's hands as a World shows them: every deed on offer as a
//! command, and what using something the player made does for people.

use crate::{Effect, Kit, Verb};
use lives::Need;
use world_core::{EntityId, StateChange, Value, World, WorldState};
use world_projection::{Hand, ProjectionCommand, SelectionId};

/// The deed that takes back the latest thing made or moved.
pub const UNDO: &str = "undo";

/// Moves one of someone's numbers by `by`, kept within `min..=max`.
pub fn add(
    state: &WorldState,
    who: EntityId,
    key: &str,
    by: i64,
    min: i64,
    max: i64,
) -> StateChange {
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

/// What using something the player made does for someone: a rest on a
/// bench eases tiredness, an evening under a lamp eases loneliness, and
/// bringing the player what a garden grew warms them to the player.
pub fn enjoy(state: &WorldState, who: EntityId, effect: Effect) -> Vec<StateChange> {
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

/// The words a World's hands are offered in: a gift ("A present for"),
/// and where someone is invited to ("the pub").
pub struct DeedWords<'a> {
    /// What every deed's command begins with: "tiny-society.hand.".
    pub prefix: &'a str,
    pub gift: &'a str,
    pub invite_to: &'a str,
}

/// Everything the player could do with their own hands now, as commands,
/// and taking back the latest thing made or moved.
pub fn deed_commands(world: &World, kit: &Kit, words: &DeedWords<'_>) -> Vec<ProjectionCommand> {
    let prefix = words.prefix;
    crate::deeds(world, kit)
        .into_iter()
        .map(|deed| {
            let place = lives::name(world.state(), deed.at);
            ProjectionCommand {
                id: format!("{prefix}{}", deed.key),
                title: deed.thing.clone(),
                detail: match deed.verb {
                    Verb::Give => format!("{} {}", words.gift, deed.thing),
                    Verb::Invite => format!("Invite {} to {}", deed.thing, words.invite_to),
                    Verb::Move => format!("Move it to {place}"),
                    _ => format!("By {place}"),
                },
                effects: Vec::new(),
                scenery: None,
                moves: Vec::new(),
                asker: None,
                question: None,
                unavailable: deed.unavailable,
                hand: Some(Hand {
                    verb: deed.verb.word().into(),
                    thing: deed.thing,
                    at: Some(SelectionId::Entity(deed.at)),
                    cost: deed.cost,
                }),
                preview: None,
                role: None,
            }
        })
        .chain(
            crate::can_undo(world.state(), kit).map(|title| ProjectionCommand {
                id: format!("{prefix}{UNDO}"),
                title,
                detail: "What it cost comes back".into(),
                effects: Vec::new(),
                scenery: None,
                moves: Vec::new(),
                asker: None,
                question: None,
                unavailable: None,
                hand: Some(Hand {
                    verb: "Undo".into(),
                    thing: String::new(),
                    at: None,
                    cost: None,
                }),
                preview: None,
                role: None,
            }),
        )
        .collect()
}
