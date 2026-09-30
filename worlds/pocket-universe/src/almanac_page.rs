//! The place's almanac: at each New Year, a page looking back on the year
//! just ended, delivered like a letter, and any past year's on request.
//! The page is `chronicle`'s; the place gives its title.

use crate::moments::Place;
use chronicle::kit;
use world_core::World;
use world_projection::{Almanac, Moment};

/// Which of the place's years a moment falls in, counting the first as 1.
#[cfg(test)]
pub(crate) fn year_of(world_time: u64) -> u64 {
    kit::year_of(&Place, world_time)
}

/// The almanac of `year`, once it has ended.
pub(crate) fn almanac(world: &World, year: u32, moments: &[Moment]) -> Option<Almanac> {
    let title = format!(
        "The {}'s year {year}",
        crate::life::cast(world.state()).settlement
    );
    kit::almanac_page(&Place, &crate::legends::Place(world), year, title, moments)
}

/// Every year whose almanac can be asked for now.
pub(crate) fn years(world: &World) -> Vec<u32> {
    kit::almanac_years(&Place, world)
}

/// On New Year's day, the page for the year just ended.
pub(crate) fn new_year(world: &World, moments: &[Moment]) -> Option<Almanac> {
    almanac(world, kit::year_just_ended(&Place, world)?, moments)
}
