//! Suggest: the player proposes a picnic, a market or a dance, and the
//! place decides who comes, how it goes and what it leaves behind, from
//! how people stand with the player and each other, their spirits and the
//! weather. It is an Action like any other, so it replays the same.

use super::*;

/// Something the player can suggest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Idea {
    pub id: &'static str,
    /// As the player reads it: "A picnic".
    pub name: &'static str,
    /// Whether it is held outdoors, where the weather matters.
    pub outdoors: bool,
    /// Whether people pair off for it.
    pub pairs: bool,
}

/// Everything the player can suggest.
pub const IDEAS: [Idea; 3] = [
    Idea {
        id: "picnic",
        name: "A picnic",
        outdoors: true,
        pairs: false,
    },
    Idea {
        id: "market",
        name: "A market",
        outdoors: true,
        pairs: false,
    },
    Idea {
        id: "dance",
        name: "A dance",
        outdoors: false,
        pairs: true,
    },
];

/// Periods before the same idea can be suggested again.
pub const SUGGEST_REST: u64 = 7;

fn suggested_key(idea: &str) -> String {
    format!("lives.suggested.{idea}")
}

/// Whether an idea can be suggested now, or why not.
pub fn can_suggest(state: &WorldState, cast: &Cast, idea: &str) -> Result<(), String> {
    let now = period(state, cast) as i64;
    match integer(state, cast.notes, &suggested_key(idea)) {
        Some(last) if now - last < SUGGEST_REST as i64 => Err(format!(
            "Suggested lately. Again in {} days.",
            SUGGEST_REST as i64 - (now - last)
        )),
        _ => Ok(()),
    }
}

/// Who comes: everyone here who thinks well enough of the player, the
/// fondest first, fewer in bad weather or low spirits.
fn who_comes(state: &WorldState, cast: &Cast, idea: &Idea, fair: bool) -> Vec<EntityId> {
    let now = period(state, cast);
    let mut people = cast_ids(state)
        .into_iter()
        .filter(|person| enrolled(state, *person) && !gone(state, *person))
        .collect::<Vec<_>>();
    people.sort_by_key(|person| {
        (
            std::cmp::Reverse(regard(state, *person) + (mix(&[person.0, now, 71]) % 12) as i64),
            person.0,
        )
    });
    let spirits = (cast.mood)(state);
    let willing = people
        .iter()
        .copied()
        .filter(|person| regard(state, *person) > -10)
        .count();
    let mut room = willing as i64 + spirits.clamp(-3, 3) - 2;
    if idea.outdoors && !fair {
        room = room.min(3);
    }
    people.truncate(room.clamp(1, willing.max(1) as i64) as usize);
    people
}

/// The player suggests something.
pub(crate) struct Suggests(pub(crate) fn(&WorldState) -> Cast);

impl Action for Suggests {
    fn name(&self) -> &'static str {
        "lives_suggestion"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let id = arg_text(request, "idea")?;
        let idea = IDEAS
            .iter()
            .find(|idea| idea.id == id)
            .ok_or_else(|| ActionError::Invalid(format!("no such idea {id}")))?;
        can_suggest(state, &cast, id).map_err(ActionError::Invalid)?;
        let fair = arg_text(request, "weather")
            .map(|w| w == "fair")
            .unwrap_or(true);
        let came = who_comes(state, &cast, idea, fair);
        let Some(&host) = came.first() else {
            return Err(ActionError::Invalid("nobody to come".into()));
        };
        let place = if idea.outdoors {
            cast.quiet
        } else {
            cast.gathering
        };
        let mut moves = Moves::default();
        for person in &came {
            moves.lack(state, *person, Need::Company, -25);
            moves.regard(state, *person, 2);
            moves.set(*person, AT, Value::Entity(place));
            for other in &came {
                moves.opinion(state, *person, *other, if idea.pairs { 3 } else { 2 });
            }
        }
        moves.set(cast.notes, &suggested_key(id), period(state, &cast) as i64);
        let names = came
            .iter()
            .map(|person| first_name(state, *person))
            .collect::<Vec<_>>();
        let what = idea.name.to_lowercase();
        let where_ = name(state, place);
        let crowd = came.len();
        let (told, said) = match (crowd, idea.outdoors && !fair) {
            (_, true) => (
                format!(
                    "{what} was rained off; {} made the best of it at {where_}",
                    list(&names)
                ),
                pick_of(
                    &[
                        "Soggy sandwiches, still fun.",
                        "Next time, a sunny day.",
                        "We laughed anyway.",
                    ],
                    host,
                ),
            ),
            (1..=2, _) => (
                format!("Only {} came to {what} at {where_}", list(&names)),
                pick_of(
                    &[
                        "Small, but lovely.",
                        "Just us. That's fine.",
                        "Quiet, but nice.",
                    ],
                    host,
                ),
            ),
            (3..=5, _) => (
                format!("{} came to {what} at {where_}", list(&names)),
                pick_of(
                    &[
                        "What a good idea that was.",
                        "We should do this more.",
                        "Lovely afternoon.",
                    ],
                    host,
                ),
            ),
            _ => (
                format!("Half the {} came to {what} at {where_}", cast.settlement),
                pick_of(
                    &[
                        "Best day in ages!",
                        "Everyone came! Everyone!",
                        "Let's make it a tradition.",
                    ],
                    host,
                ),
            ),
        };
        let mut draft = EventDraft::new("suggestion_held");
        draft.actor = Some(host);
        draft.targets = came.clone();
        draft.payload.insert("idea".into(), id.into());
        draft.payload.insert("came".into(), (crowd as i64).into());
        draft.payload.insert("fair".into(), fair.into());
        draft
            .payload
            .insert("told".into(), capitalise(&told).into());
        draft.payload.insert("said".into(), said.into());
        draft.changes = moves.changes;
        Ok(draft)
    }
}

fn list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn pick_of(lines: &[&str], who: EntityId) -> String {
    lines[(mix(&[who.0, 73]) % lines.len() as u64) as usize].to_string()
}

/// The request that suggests an idea in the given weather.
pub fn suggestion_request(idea: &str, fair: bool) -> ActionRequest {
    ActionRequest::new("lives_suggestion")
        .arg("idea", idea)
        .arg("weather", if fair { "fair" } else { "foul" })
}
