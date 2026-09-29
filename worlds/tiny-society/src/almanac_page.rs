//! The harbour's almanac: at each New Year, a page looking back on the
//! year just ended, delivered like a letter, and any past year's on
//! request.

use crate::almanac::YEAR_DAYS;
use crate::persistence::WORLD_DAY_TICKS;
use chronicle::Year;
use world_core::{EntityId, World};
use world_projection::{Almanac, Moment, Named, SelectionId};

/// Which of the harbour's years a moment falls in, counting the first as 1.
pub(crate) fn year_of(world_time: u64) -> u64 {
    world_time / WORLD_DAY_TICKS / YEAR_DAYS + 1
}

/// Everyone who has ever lived in the harbour, gone or not.
fn everyone(world: &World) -> Vec<EntityId> {
    world
        .state()
        .entities()
        .filter(|entity| entity.kind == "resident")
        .map(|entity| entity.id)
        .collect()
}

/// What was built in `year`: the works the harbour finished, each with
/// whoever asked for it, and what the player made by hand, each as itself.
fn built(world: &World, year: Year) -> Vec<Named> {
    let mut built = Vec::new();
    for finished in storylets::finished_goals(world, crate::story::deck()) {
        let Some((id, at)) = finished.event else {
            continue;
        };
        let (true, Some(label), Some(event)) = (
            year.holds(at),
            crate::story::goal_label(finished.goal),
            world.event(id),
        ) else {
            continue;
        };
        built.push(Named {
            name: label.to_string(),
            who: SelectionId::Entity(event.actor.unwrap_or(crate::HARBOR)),
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
    let span = Year::of(year, YEAR_DAYS, WORLD_DAY_TICKS);
    let title = format!("The harbour's year {year}");
    Some(chronicle::almanac(
        &crate::legends::Harbour(world),
        span,
        title,
        &everyone(world),
        built(world, span),
        moments,
    ))
}

/// On New Year's day, the page for the year just ended.
pub(crate) fn new_year(world: &World, moments: &[Moment]) -> Option<Almanac> {
    let day = world.world_time() / WORLD_DAY_TICKS;
    let year = year_of(world.world_time());
    if year < 2 || !day.is_multiple_of(YEAR_DAYS) {
        return None;
    }
    almanac(world, u32::try_from(year - 1).ok()?, moments)
}
