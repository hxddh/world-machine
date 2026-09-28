//! The harbour's book of everything to find: keepsakes, people met,
//! things made and festival days, found or still a silhouette.

use world_core::World;
use world_projection::{BookEntry, MarkShape};

fn entry(
    shelf: &str,
    name: String,
    found: bool,
    shape: Option<MarkShape>,
    hint: String,
) -> BookEntry {
    BookEntry {
        shelf: shelf.into(),
        name,
        found,
        shape,
        hint,
    }
}

/// Every entry, in shelf order, found or not.
pub(crate) fn book(world: &World) -> Vec<BookEntry> {
    let state = world.state();
    let cast = crate::life::cast();
    let mut book = Vec::new();
    // Keepsakes: whatever anyone living here could give, and what a garden
    // grows.
    let kept = lives::keepsakes(world)
        .into_iter()
        .map(|kept| kept.what)
        .collect::<std::collections::BTreeSet<_>>();
    let possible =
        lives::possible_keepsakes(world, &cast)
            .into_iter()
            .chain(hands::PRODUCE.iter().map(|what| {
                (
                    (*what).to_string(),
                    "Grown in a garden you planted".to_string(),
                )
            }));
    for (what, hint) in possible {
        let found = kept.contains(&what);
        book.push(entry(
            "Keepsakes",
            what,
            found,
            Some(MarkShape::Parcel),
            hint,
        ));
    }
    // Letters: one from everyone who lives here.
    for (name, wrote) in lives::letter_writers(world, &cast) {
        book.push(entry(
            "Letters",
            name,
            wrote,
            Some(MarkShape::Parcel),
            "Comes on a quiet day".into(),
        ));
    }
    // Firsts: each small first of a quiet day, once it has come.
    for first in lives::firsts(world) {
        book.push(entry(
            "Firsts",
            first,
            true,
            None,
            "Comes on a quiet day".into(),
        ));
    }
    // People: everyone here, and strangers who might come to stay.
    let met = lives::met(world);
    for (name, person) in lives::people_to_meet(world, &cast) {
        let found = person.is_some_and(|person| met.contains(&person));
        let hint = if person.is_some() {
            "Say hello".to_string()
        } else {
            "Might come to stay".to_string()
        };
        book.push(entry("People", name, found, None, hint));
    }
    // Things made with the player's own hands.
    let made = hands::ever_made(world);
    let kit = crate::handwork::kit(state);
    for thing in kit.things {
        let found = made.contains(thing.id);
        let shape = thing.stages.last().map_or(thing.shape, |(_, shape)| *shape);
        book.push(entry(
            "Made",
            thing.name.into(),
            found,
            Some(crate::story::fixture_shape(shape)),
            format!("{} it with your own hands", thing.verb.word()),
        ));
    }
    // What the place is building towards, once each is finished.
    for goal in crate::story::goals(world) {
        let found = goal.done >= goal.parts;
        book.push(entry(
            "Made",
            goal.label.clone(),
            found,
            Some(goal.shape),
            "Built by the whole place, a part at a time".into(),
        ));
    }
    // Festival days, once each has been held.
    let held = calendar::held(world);
    let almanac = crate::almanac::almanac(state);
    for festival in almanac.festivals {
        let shape = if festival.shape.is_empty() {
            MarkShape::Flag
        } else {
            crate::story::fixture_shape(festival.shape)
        };
        book.push(entry(
            "Days",
            festival.name.into(),
            held.contains(festival.id),
            Some(shape),
            "Comes round once a year".into(),
        ));
    }
    book
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
