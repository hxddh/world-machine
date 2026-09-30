//! The place's almanac: at each New Year, a page looking back on the year
//! just ended, delivered like a letter, and any past year's on request.

use crate::almanac::YEAR;
use crate::BACKGROUND_PERIOD;
use chronicle::Year;
use world_core::{EntityId, World};
use world_projection::{Almanac, Moment, Named, SelectionId};

/// Which of the place's years a moment falls in, counting the first as 1.
pub(crate) fn year_of(world_time: u64) -> u64 {
    world_time / BACKGROUND_PERIOD / YEAR + 1
}

/// Everyone who has ever lived in the place, gone or not.
fn everyone(world: &World) -> Vec<EntityId> {
    world
        .state()
        .entities()
        .map(|entity| entity.id)
        .filter(|id| crate::legends::is_person(world, *id))
        .collect()
}

/// What was built in `year`: the works finished, each with whoever asked
/// for it, and what the player made by hand, each as itself.
fn built(world: &World, year: Year) -> Vec<Named> {
    let labels = crate::story::goals(world)
        .into_iter()
        .map(|goal| (goal.id, goal.label))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut built = Vec::new();
    for finished in storylets::finished_goals(world, crate::story::deck_ref()) {
        let Some((id, at)) = finished.event else {
            continue;
        };
        let (true, Some(label), Some(event)) =
            (year.holds(at), labels.get(finished.goal), world.event(id))
        else {
            continue;
        };
        built.push(Named {
            name: label.clone(),
            who: SelectionId::Entity(event.actor.unwrap_or(crate::SLOT_A)),
        });
    }
    for event in chronicle::in_year(world, &["built_by_hand"], year) {
        let Some(thing) = event.targets.first() else {
            continue;
        };
        let Some(entity) = world.state().entity(*thing) else {
            continue;
        };
        built.push(Named {
            name: world_projection::entity_title(entity),
            who: SelectionId::Entity(*thing),
        });
    }
    built
}

/// The almanac of `year`, once it has ended.
pub(crate) fn almanac(world: &World, year: u32, moments: &[Moment]) -> Option<Almanac> {
    if year == 0 || u64::from(year) >= year_of(world.world_time()) {
        return None;
    }
    let span = Year::of(year, YEAR, BACKGROUND_PERIOD);
    Some(chronicle::almanac(
        &crate::legends::Place(world),
        span,
        format!(
            "The {}'s year {year}",
            crate::life::cast(world.state()).settlement
        ),
        &everyone(world),
        built(world, span),
        moments,
    ))
}

/// Every year whose almanac can be asked for now: each one that has
/// ended, oldest first. A year's page is kept from its New Year on.
pub(crate) fn years(world: &World) -> Vec<u32> {
    let now = year_of(world.world_time());
    (1..now)
        .filter_map(|year| u32::try_from(year).ok())
        .collect()
}

/// On New Year's day, the page for the year just ended.
pub(crate) fn new_year(world: &World, moments: &[Moment]) -> Option<Almanac> {
    let day = world.world_time() / BACKGROUND_PERIOD;
    let year = year_of(world.world_time());
    if year < 2 || !day.is_multiple_of(YEAR) {
        return None;
    }
    almanac(world, u32::try_from(year - 1).ok()?, moments)
}
