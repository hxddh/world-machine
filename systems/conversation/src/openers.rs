//! What the player might say to someone, from what the World knows of
//! them: two or three openers shown beside the words a player can type, so
//! nobody faces an empty box. Each is plain words this System's own ears
//! hear as what it says (held by a test), and is sent as the player's own
//! words: an opener does nothing that typing it would not.

use crate::{worst_need, Kit};
use world_core::{EntityId, World};

/// An opener for someone low: tired, lonely, hungry.
pub const HOLDING_UP: &str = "You look tired. How are you holding up?";
/// An opener about someone they feel strongly about, `{name}` theirs.
pub const THINK_OF: &str = "What do you think of {name}?";
/// An opener when they need something the player could see to.
pub const NEED: &str = "Is there anything you need?";
/// An opener about their work.
pub const WORK: &str = "How's work going?";
/// An opener for anyone.
pub const NEWS: &str = "What's new with you?";

/// Every opener, as a catalog translates it.
pub const ALL: [&str; 5] = [HOLDING_UP, THINK_OF, NEED, WORK, NEWS];

/// The most openers shown for someone.
pub const MOST: usize = 3;

/// What the player might open with to `who`, most telling first: how
/// they are holding up when they are low, what they think of whoever they
/// feel most strongly about, what they need when something is on offer,
/// their work, and what is new. At most [`MOST`].
pub fn openers(world: &World, kit: &Kit, who: EntityId) -> Vec<String> {
    let state = world.state();
    if !crate::can_talk_to(state, kit, who) {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let (_, lack) = worst_need(state, who);
    if lack >= 60 {
        lines.push(HOLDING_UP.to_string());
    }
    let strongest = (kit.people)(state)
        .into_iter()
        .filter(|other| *other != who)
        .map(|other| (lives::opinion(state, who, other).abs(), other))
        .filter(|(strength, _)| *strength >= 30)
        .max();
    if let Some((_, other)) = strongest {
        lines.push(THINK_OF.replace("{name}", &lives::first_name(state, other)));
    }
    if (kit.need_line)(world, who).1.is_some() {
        lines.push(NEED.to_string());
    }
    if (kit.work_line)(world, who).is_some() {
        lines.push(WORK.to_string());
    }
    lines.push(NEWS.to_string());
    lines.truncate(MOST);
    lines
}

/// Every opener for everyone the player can talk to, as a snapshot shows
/// them.
pub fn shown(world: &World, kit: &Kit) -> Vec<world_projection::Openers> {
    let state = world.state();
    (kit.people)(state)
        .into_iter()
        .map(|who| world_projection::Openers {
            who: world_projection::SelectionId::Entity(who),
            lines: openers(world, kit, who),
        })
        .filter(|openers| !openers.lines.is_empty())
        .collect()
}
