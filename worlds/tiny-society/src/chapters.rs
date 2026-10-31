//! How a chapter of the harbour's life is told when it closes, past what
//! the storyteller and people's news give it (1.0 P4): what the player made
//! in it, and, when the chapter's own story is short, as late chapters'
//! are once the harbour's first stories are told, the rest of what it
//! held: the year's days kept, what the player found and was written,
//! one of its everyday days, the season, and how many called the harbour
//! home. A chapter past the twelfth reads as full as the first.
//!
//! All of it is read from the record, so the chapter is told the same way
//! whenever it is read again.

use world_core::{Event, Value, World};

/// The fewest sentences a chapter is told in, when it held that much.
pub(crate) const FULL: usize = 5;

/// What the player found for the book this chapter: one thing, or two.
const FOUND_ONE: &str = "You found {a} for your book.";
const FOUND_TWO: &str = "You found {a} and {b} for your book.";
/// Who wrote to the player this chapter: one, or two.
const WROTE_ONE: &str = "{a} wrote to you.";
const WROTE_TWO: &str = "{a} and {b} wrote to you.";
/// How many people lived in the harbour as the chapter closed.
const HOME_TO: &str = "The harbour was home to {n} people by the end of it.";

/// The season as a chapter remembers it: spring, summer, autumn, winter.
const SEASON_LINES: [[&str; 3]; 4] = [
    [
        "Spring came in with blossom on every path.",
        "The days drew out, and the boats went out early.",
        "There was washing on every line and lambs on the hill.",
    ],
    [
        "The evenings were long and warm.",
        "Summer brought visitors off every ferry.",
        "Everyone ate outside when they could.",
    ],
    [
        "The leaves turned, and the nights drew in.",
        "There were apples to pick and nets to mend.",
        "The first gales came in off the sea.",
    ],
    [
        "The frost came, and the lamps were lit early.",
        "The harbour kept close to its fires.",
        "There was snow on the rooftops and smoke from every chimney.",
    ],
];

fn text<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    match event.payload.get(key) {
        Some(Value::Text(text)) if !text.is_empty() => Some(text),
        _ => None,
    }
}

fn fill(line: &str, words: &[(&str, &str)]) -> String {
    words.iter().fold(line.to_string(), |line, (slot, word)| {
        line.replace(&format!("{{{slot}}}"), word)
    })
}

/// One or two of `items` in a line: `one` or `two`.
fn one_or_two(items: &[String], one: &str, two: &str) -> Option<String> {
    match items {
        [] => None,
        [a] => Some(fill(one, &[("a", a)])),
        [a, b, ..] => Some(fill(two, &[("a", a), ("b", b)])),
    }
}

/// Adds to a chapter's `summary` what the player made in it, and, while it
/// is told in fewer than [`FULL`] sentences, the rest of what it held:
/// `lived` is the chapter's events, from `started`.
pub(crate) fn fill_out(world: &World, started: u64, lived: &[&Event], summary: &mut Vec<String>) {
    let state = world.state();
    let made = lived
        .iter()
        .rev()
        .filter(|event| {
            matches!(
                event.kind.as_str(),
                "built_by_hand" | "decorated_by_hand" | "planted_by_hand"
            )
        })
        .filter_map(|event| text(event, "told"))
        .map(|told| format!("{told}."))
        .collect::<Vec<_>>();
    // What the player made is always part of how it is told.
    summary.extend(made.first().cloned());
    // One of the year's days kept in it, the latest, unless the chapter
    // before told it already.
    let before = storylets::chapters_shown(world)
        .last()
        .map(|chapter| chapter.summary.clone())
        .unwrap_or_default();
    let festivals = lived
        .iter()
        .rev()
        .filter(|event| event.kind == "festival_held")
        .filter_map(|event| calendar::told(event))
        .map(|told| format!("{told}."))
        .find(|told| !before.contains(told.as_str()));
    let finds = lived
        .iter()
        .filter(|event| event.kind == ritual::FOUND)
        .filter_map(|event| text(event, "find"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut writers = Vec::new();
    for event in lived.iter().filter(|event| event.kind == "letter_written") {
        let name = event.actor.map(|who| lives::first_name(state, who));
        if let Some(name) = name.filter(|name| !writers.contains(name)) {
            writers.push(name);
        }
    }
    let days = lived
        .iter()
        .filter(|event| event.kind == "lived")
        .filter_map(|event| text(event, "told"))
        .collect::<Vec<_>>();
    let turn = lives::mix(&[started, 239]) as usize;
    let everyday = (!days.is_empty()).then(|| format!("{}.", days[turn % days.len()]));
    let season = crate::finds::season(state).unwrap_or(0).min(3);
    let season = SEASON_LINES[season][turn % 3].to_string();
    let home = fill(
        HOME_TO,
        &[("n", &crate::story::people(world).len().to_string())],
    );
    let more = festivals
        .into_iter()
        .chain(one_or_two(&finds, FOUND_ONE, FOUND_TWO))
        .chain(one_or_two(&writers, WROTE_ONE, WROTE_TWO))
        .chain(everyday)
        .chain([season])
        .chain(made.get(1).cloned())
        .chain([home]);
    for line in more {
        if summary.len() >= FULL {
            break;
        }
        if !summary.contains(&line) {
            summary.push(line);
        }
    }
}

/// Every line a chapter is filled out with, slots and all, for the
/// catalogs.
#[cfg(test)]
pub(crate) fn lines() -> Vec<&'static str> {
    [FOUND_ONE, FOUND_TWO, WROTE_ONE, WROTE_TWO, HOME_TO]
        .into_iter()
        .chain(SEASON_LINES.iter().flatten().copied())
        .collect()
}
