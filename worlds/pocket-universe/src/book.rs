//! A pocket universe's book of everything to find: keepsakes, letters, people met,
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
    let cast = crate::life::cast(state);
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
            Some(crate::story::fixture_shape(world, shape)),
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
            crate::story::fixture_shape(world, festival.shape)
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
