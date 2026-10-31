//! Letters about the player (1.0 P4): two letters in every five, when the
//! player has done something lately that no letter has told them of yet,
//! are about that: something they made, moved, named or gave with their
//! own hands, an answer they gave, a favour they did. The letter names
//! what it tells of as its cause, so nothing is cited that the record does
//! not hold.
//!
//! And every letter is worded in turn: its opening, how it leads into its
//! news and its closing each step through all their ways before any comes
//! round again, so no two letters running read alike.

use super::*;
use crate::retold::{act_of, DEEDS, DID_ME_A_FAVOUR, FAVOUR_DONE};

/// On the notes: the Events letters have told the player of, so none is
/// told twice.
const CITED: &str = "lives.cited";
/// How many of those are kept: far more than letters in a season.
const KEPT_CITED: usize = 60;

/// Which letters of every [`ABOUT_YOU_IN`] are about the player.
const ABOUT_YOU_TURNS: [usize; 2] = [1, 3];
const ABOUT_YOU_IN: usize = 5;

/// How far back a letter about the player reaches, in periods.
const ABOUT_YOU_PERIODS: u64 = 60;

/// How a letter tells of something the player did: `{act}` as a letter
/// cites it ("you built a bench by the mill").
pub(crate) const ABOUT_DEED: [&str; 8] = [
    "I keep thinking about the day {act}.",
    "People are still talking about how {act}.",
    "I walked past this morning and thought of the day {act}.",
    "I never thanked you properly for the day {act}.",
    "It's the little things, like the day {act}.",
    "A visitor asked me about the day {act}, and I told them all of it.",
    "It still warms me to think of the day {act}.",
    "Not everyone would have bothered, but {act}.",
];

/// How a letter tells of an answer the player gave: `{words}`, the words
/// it was offered in.
pub(crate) const ABOUT_SAID: [&str; 6] = [
    "I keep coming back to what you said: “{words}”.",
    "You said “{words}”, and you were right.",
    "What you said has stayed with me: “{words}”.",
    "Whenever things get muddled, I think of you saying “{words}”.",
    "I'm glad you said “{words}” when you did.",
    "People still bring up the time you said “{words}”.",
];

/// The `n`th of `items`, stepping by `stride` (which shares no factor
/// with how many there are) from `from`, so every one comes round before
/// any comes again.
pub(crate) fn nth<'a>(items: &[&'a str], n: usize, stride: usize, from: usize) -> &'a str {
    if items.is_empty() {
        return "";
    }
    items[(from + n * stride) % items.len()]
}

/// How many letters the player has been written: what the next one's
/// wording turns on.
pub(crate) fn letters_so_far(world: &World) -> usize {
    world.history_index().of_kind("letter_written").len()
}

fn cited(state: &WorldState, cast: &Cast) -> Vec<i64> {
    match state
        .entity(cast.notes)
        .and_then(|notes| notes.component(CITED))
    {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Integer(id) => Some(*id),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether the `n`th letter is one about the player, if they have done
/// something to tell.
pub(crate) fn about_you_turn(n: usize) -> bool {
    ABOUT_YOU_TURNS.contains(&(n % ABOUT_YOU_IN))
}

/// A letter from `writer` about something the player did lately that no
/// letter has told them of: what it says, and the request, caused by what
/// it tells of. What the writer was part of first, then the latest.
pub(crate) fn about_you(
    world: &World,
    cast: &Cast,
    writer: EntityId,
    request: ActionRequest,
) -> Option<(String, ActionRequest)> {
    let state = world.state();
    let told = cited(state, cast);
    let since = world
        .world_time()
        .saturating_sub(ABOUT_YOU_PERIODS * cast.period.max(1));
    let index = world.history_index();
    let work = (cast.work)(state, writer);
    let (event, act) = DEEDS
        .iter()
        .chain(&["situation_answered", FAVOUR_DONE])
        .flat_map(|kind| {
            index
                .of_kind(kind)
                .iter()
                .rev()
                .filter_map(|id| world.event(*id))
                .take_while(|event| event.world_time >= since)
        })
        .filter(|event| !told.contains(&(event.id.0 as i64)))
        .filter_map(|event| Some((event, act_of(state, event)?)))
        .max_by_key(|(event, _)| {
            let theirs = event.actor == Some(writer)
                || event.targets.contains(&writer)
                || work.is_some_and(|work| event.targets.contains(&work));
            (theirs, event.id)
        })?;
    let turn = told.len();
    let body = match &act {
        // A favour done for the writer is one done for "me".
        Act::Deed(_) if event.kind == FAVOUR_DONE && event.actor == Some(writer) => {
            fill(nth(&ABOUT_DEED, turn, 3, 0), &[("act", DID_ME_A_FAVOUR)])
        }
        Act::Deed(deed) => fill(nth(&ABOUT_DEED, turn, 3, 0), &[("act", deed)]),
        Act::Said(words) => fill(nth(&ABOUT_SAID, turn, 1, 0), &[("words", words)]),
    };
    Some((
        body,
        request
            .arg("cited", Value::Integer(event.id.0 as i64))
            .caused_by(event.id),
    ))
}

/// The change a letter about the player makes to the notes: what it told
/// of, so no letter tells it again.
pub(crate) fn about_you_notes(
    state: &WorldState,
    cast: &Cast,
    request: &ActionRequest,
) -> Option<StateChange> {
    let Some(Value::Integer(id)) = request.args.get("cited") else {
        return None;
    };
    let mut told = cited(state, cast);
    told.push(*id);
    let from = told.len().saturating_sub(KEPT_CITED);
    Some(StateChange::SetComponent {
        entity: cast.notes,
        key: CITED.into(),
        value: Value::List(told.into_iter().skip(from).map(Value::Integer).collect()),
    })
}

/// Every line a letter about the player is told in, slots and all, for
/// the catalogs.
pub fn about_you_lines() -> Vec<&'static str> {
    ABOUT_DEED
        .iter()
        .chain(&ABOUT_SAID)
        .copied()
        .chain([crate::retold::DID_A_FAVOUR, DID_ME_A_FAVOUR])
        .collect()
}
