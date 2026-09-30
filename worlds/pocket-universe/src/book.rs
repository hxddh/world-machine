//! A pocket universe's book of everything to find: keepsakes, letters, people met,
//! things made and festival days, found or still a silhouette.

use world_core::World;
use world_projection::BookEntry;

/// Every entry, in shelf order, found or not.
pub(crate) fn book(world: &World) -> Vec<BookEntry> {
    let state = world.state();
    chronicle::kit::book(
        world,
        &crate::life::cast(state),
        &crate::handwork::kit(state),
        &crate::story::goals(world),
        &crate::almanac::almanac(state),
        |shape| crate::story::fixture_shape(world, shape),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_book_of_sixty_things_to_find_in_every_place() {
        for seed in [
            crate::SEED_MARS_COLONY_COMMAND,
            crate::SEED_1980S_TOWN_COMMAND,
            crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
        ] {
            let mut universe = crate::PocketUniverse::new().unwrap();
            universe.invoke_projection_command(seed).unwrap();
            let first = book(universe.world());
            assert!(first.len() >= 60, "{seed}: {}", first.len());
            let names = first
                .iter()
                .map(|entry| (entry.shelf.clone(), entry.name.clone()))
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(names.len(), first.len(), "{seed}: no entry twice");
            for _ in 0..40 {
                universe
                    .invoke_projection_command(crate::NUDGE_COMMAND)
                    .unwrap();
            }
            let later = book(universe.world());
            let found = |book: &[BookEntry]| book.iter().filter(|entry| entry.found).count();
            assert!(found(&later) > found(&first), "{seed}");
        }
    }
}
