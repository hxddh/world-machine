//! What the player might say to someone, from what the World knows of
//! them: three openers shown beside the words a player can type, so
//! nobody faces an empty box and typing is never needed. Each is plain
//! words this System's own ears hear as what it says (held by a test),
//! and is sent as the player's own words: an opener does nothing that
//! typing it would not. After each answer they are fresh: the three
//! things someone offered to be asked when they could not answer, or the
//! next of what they could be asked ([`crate::grounded::offers`]).

use crate::{grounded, spoken_today, worst_need, Kit, SPOKEN};
use world_core::{EntityId, Value, World};

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

/// What the player might say to `who` now. Offered three things to ask by
/// their last answer today, those. Otherwise, most telling first: before
/// anything is said today, how they are holding up when they are low,
/// what they think of whoever they feel most strongly about and what
/// they need when something is on offer; then the next of what they could
/// be asked. At most [`MOST`].
pub fn openers(world: &World, kit: &Kit, who: EntityId) -> Vec<String> {
    let state = world.state();
    if !crate::can_talk_to(state, kit, who) {
        return Vec::new();
    }
    if let Some(offered) = offered_today(world, who) {
        return offered;
    }
    let turn = spoken_today(world, who);
    let mut lines = Vec::new();
    if turn == 0 {
        lines.extend(first(world, kit, who));
    }
    for offer in grounded::offers(world, kit, who, turn) {
        if !lines.contains(&offer.question) {
            lines.push(offer.question);
        }
    }
    lines.truncate(MOST);
    lines
}

/// What `who` offered to be asked in their last answer today, if it
/// offered anything.
fn offered_today(world: &World, who: EntityId) -> Option<Vec<String>> {
    let now = world.world_time();
    let last = world
        .events()
        .iter()
        .rev()
        .take_while(|event| event.world_time == now)
        .find(|event| event.kind == SPOKEN && event.targets.first() == Some(&who))?;
    match last.payload.get("offers") {
        Some(Value::List(offers)) => Some(
            offers
                .iter()
                .filter_map(|offer| match offer {
                    Value::Text(offer) => Some(offer.clone()),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    }
}

/// The most telling openers, before anything is said today.
fn first(world: &World, kit: &Kit, who: EntityId) -> Vec<String> {
    let state = world.state();
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
