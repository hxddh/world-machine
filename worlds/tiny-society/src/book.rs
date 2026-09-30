//! The harbour's book of everything to find: keepsakes, people met,
//! things made and festival days, found or still a silhouette.

use world_core::World;
use world_projection::BookEntry;

/// Every entry, in shelf order, found or not.
pub(crate) fn book(world: &World) -> Vec<BookEntry> {
    let state = world.state();
    chronicle::kit::book(
        world,
        &crate::life::cast(),
        &crate::handwork::kit(state),
        &crate::story::goals(world),
        &crate::almanac::almanac(state),
        crate::story::fixture_shape,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_book_of_sixty_things_to_find_that_fills_as_you_play() {
        let mut society = crate::TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        let first = book(branch.world());
        assert!(first.len() >= 60, "{}", first.len());
        let names = first
            .iter()
            .map(|entry| (entry.shelf.clone(), entry.name.clone()))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(names.len(), first.len(), "no entry twice");
        for shelf in ["Keepsakes", "Letters", "People", "Made", "Days"] {
            assert!(first.iter().any(|entry| entry.shelf == shelf), "{shelf}");
        }
        assert!(first
            .iter()
            .all(|entry| entry.found || !entry.hint.is_empty()));
        let found = |book: &[BookEntry]| book.iter().filter(|entry| entry.found).count();
        let deed = branch
            .projection_snapshot()
            .commands
            .into_iter()
            .find(|command| {
                command
                    .hand
                    .as_ref()
                    .is_some_and(|hand| hand.verb == "Build")
            })
            .unwrap();
        branch.invoke_projection_command(&deed.id).unwrap();
        for _ in 0..40 {
            branch
                .invoke_projection_command(crate::story::WAIT_COMMAND)
                .unwrap();
        }
        let later = book(branch.world());
        assert!(
            found(&later) >= found(&first) + 3,
            "{} then {}",
            found(&first),
            found(&later)
        );
        assert!(later
            .iter()
            .any(|entry| entry.shelf == "Made" && entry.found));
        assert!(later
            .iter()
            .any(|entry| entry.shelf == "Keepsakes" && entry.found));
    }
}
