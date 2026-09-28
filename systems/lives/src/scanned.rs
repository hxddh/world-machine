//! The answers the book, the letter box and the daily round once found by
//! reading the whole history, kept only as a reference for tests: what
//! the System now finds through the World's index of its history must be
//! exactly what these give.

use super::*;

/// Every keepsake, from every event that marks something as given to keep.
pub fn keepsakes(world: &World) -> Vec<Keepsake> {
    world
        .events()
        .iter()
        .filter_map(keepsake_of_event)
        .collect()
}

/// Every letter, from every event.
pub fn letters(world: &World) -> Vec<Letter> {
    world.events().iter().filter_map(letter_of).collect()
}

/// Everyone who has written a letter, from every event.
pub fn letter_writers(world: &World) -> BTreeSet<EntityId> {
    world
        .events()
        .iter()
        .filter(|event| event.kind == "letter_written")
        .filter_map(|event| event.actor)
        .collect()
}

/// Everyone met, from every event.
pub fn met(world: &World) -> BTreeSet<EntityId> {
    world.events().iter().filter_map(met_in).collect()
}

/// Every small first, from every event.
pub fn firsts(world: &World) -> Vec<String> {
    world
        .events()
        .iter()
        .filter(|event| quiet::FIRST_KINDS.contains(&event.kind.as_str()))
        .filter_map(told)
        .collect()
}

/// Whether any event of this kind was ever recorded, from every event.
pub fn happened(world: &World, kind: &str) -> bool {
    world.events().iter().any(|event| event.kind == kind)
}

/// Whether the week has room for another keepsake, from every keepsake.
pub fn room_for_keepsake(world: &World, cast: &Cast) -> bool {
    let since = world.world_time().saturating_sub(cast.period * 7);
    keepsakes(world)
        .iter()
        .filter(|kept| kept.world_time > since)
        .count()
        < KEEPSAKES_A_WEEK - 1
}

/// Who remembers what of the day `year` periods ago, from every event.
pub fn remembered(world: &World, cast: &Cast, year: u64) -> Option<(EntityId, String)> {
    let (from, to) = a_year_ago(world, cast, year)?;
    memory_of(
        world,
        cast,
        world
            .events()
            .iter()
            .filter(|event| (from..to).contains(&event.world_time)),
    )
}

/// What `writer` could recall in a letter, from every event.
pub fn memory(world: &World, cast: &Cast, writer: EntityId) -> Option<String> {
    quiet::memory(
        world,
        cast,
        writer,
        world
            .events()
            .iter()
            .filter(|event| quiet::MEMORABLE.contains(&event.kind.as_str())),
    )
}

/// What the index-backed answers give, and what reading every event
/// gives, for the same World: they must be equal.
pub fn compare(world: &World, cast: &Cast) -> Result<(), String> {
    let check = |what: &str, same: bool| {
        if same {
            Ok(())
        } else {
            Err(format!(
                "{what} differs from the full scan at event {}",
                world.events().len()
            ))
        }
    };
    check("keepsakes", super::keepsakes(world) == keepsakes(world))?;
    check("letters", super::letters(world) == letters(world))?;
    let wrote = letter_writers(world);
    check(
        "letter writers",
        super::letter_writers(world, cast)
            == (cast.people)(world)
                .into_iter()
                .map(|person| (letter_name(world.state(), person), wrote.contains(&person)))
                .collect::<Vec<_>>(),
    )?;
    check("met", super::met(world) == met(world))?;
    check("firsts", super::firsts(world) == firsts(world))?;
    for kind in ["greeted", "reacted"] {
        check(kind, super::happened(world, kind) == happened(world, kind))?;
    }
    check(
        "room for a keepsake",
        super::room_for_keepsake(world, cast) == room_for_keepsake(world, cast),
    )?;
    for year in [1, 360, 365] {
        check(
            "a year ago",
            super::remembered(world, cast, year) == remembered(world, cast, year),
        )?;
    }
    for writer in (cast.people)(world) {
        check(
            "a memory to recall",
            quiet::memory(
                world,
                cast,
                writer,
                world.events_of_kind(&quiet::MEMORABLE).into_iter(),
            ) == memory(world, cast, writer),
        )?;
    }
    Ok(())
}
