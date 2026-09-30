//! How the people a player can talk to are shown: how each stands with the
//! player, who is asking something now, and the face each wears.

use crate::Kit;
use std::collections::BTreeSet;
use world_core::{EntityId, World};
use world_projection::{Mood, ProjectionCommand, SelectionId};

/// How someone the player can talk to stands with them.
pub fn standing_of(world: &World, kit: &Kit, who: EntityId) -> Option<world_projection::Standing> {
    let state = world.state();
    crate::can_talk_to(state, kit, who).then(|| {
        let standing = crate::standing(state, kit, who);
        world_projection::Standing {
            level: standing.level,
            words: standing.words,
        }
    })
}

/// Everyone asking the player something in `commands`.
pub fn askers(commands: &[ProjectionCommand]) -> BTreeSet<EntityId> {
    commands
        .iter()
        .filter(|command| command.question.is_some())
        .filter_map(|command| match command.asker {
            Some(SelectionId::Entity(who)) => Some(who),
            _ => None,
        })
        .collect()
}

/// How someone feels, for their face: thinking while they are asking the
/// player something, cross while hurt by what the player said, else how
/// their life is going. Someone too young for an everyday life wears the
/// face `young` gives.
pub fn mood_of(
    world: &World,
    kit: &Kit,
    who: EntityId,
    askers: &BTreeSet<EntityId>,
    young: impl FnOnce() -> Option<Mood>,
) -> Option<Mood> {
    let state = world.state();
    if !lives::enrolled(state, who) {
        return young();
    }
    if askers.contains(&who) {
        return Some(Mood::Thinking);
    }
    if standing_of(world, kit, who).is_some_and(|standing| standing.level < 0) {
        return Some(Mood::Cross);
    }
    Some(match lives::mood(state, who) {
        "cross" => Mood::Cross,
        "sad" => Mood::Sad,
        "happy" => Mood::Happy,
        _ => Mood::Content,
    })
}

/// What a Pack's own Chinese (`catalog`, English and Chinese a tab apart
/// on each line) calls a name, if it translates it.
pub fn in_chinese(catalog: &'static str, name: &str) -> Option<&'static str> {
    catalog.lines().find_map(|line| {
        let (english, chinese) = line.split_once('\t')?;
        (english == name && chinese != name).then_some(chinese)
    })
}
