//! Who the player could meet, the strangers waiting at the door, who has
//! been met, and the notes people forget.

use crate::*;

/// Everyone the player could meet: who lives here now, then strangers who
/// might yet come to stay, by name, with the person when they are here.
pub fn people_to_meet(world: &World, cast: &Cast) -> Vec<(String, Option<EntityId>)> {
    let state = world.state();
    let people = (cast.people)(world);
    let room = cast.most_people.saturating_sub(people.len());
    let mut all = people
        .into_iter()
        .map(|person| (first_name(state, person), Some(person)))
        .collect::<Vec<_>>();
    // Only as many strangers as there is room for, in the order they come.
    for stranger in next_names(state, cast, room) {
        if !all.iter().any(|(name, _)| name == stranger) {
            all.push((stranger.to_string(), None));
        }
    }
    all
}

/// Who a stranger at the door is: one of the next few names nobody here
/// or gone has had, as the place's fortunes (in `seed`) have it, so two
/// places that have gone differently meet different people.
pub(crate) fn stranger_name(state: &WorldState, cast: &Cast, seed: u64) -> Option<&'static str> {
    pick(&next_names(state, cast, 3), mix(&[seed, 29])).copied()
}

/// The names of strangers waiting at the door for an answer.
pub fn waiting_strangers(state: &WorldState, cast: &Cast) -> Vec<String> {
    open(state, cast)
        .into_iter()
        .filter_map(
            |(key, _)| match open_map(state, cast, &key)?.get("stranger") {
                Some(Value::Text(name)) => Some(name.clone()),
                _ => None,
            },
        )
        .collect()
}

/// The name of the stranger at the door in a situation: the one they gave
/// when it came up, while nobody has taken it since; otherwise (a
/// situation opened by an older World) the next name free.
pub(crate) fn stranger_at_door(
    state: &WorldState,
    cast: &Cast,
    candidate: &Candidate,
) -> Option<String> {
    let given = open_map(state, cast, &candidate.key()).and_then(|map| match map.get("stranger") {
        Some(Value::Text(name)) => Some(name.clone()),
        _ => None,
    });
    let free = |name: &str| {
        !state
            .entities()
            .any(|entity| entity.component("name") == Some(&Value::Text(name.into())))
    };
    given
        .filter(|name| free(name))
        .or_else(|| stranger_name(state, cast, candidate.topic).map(str::to_string))
}

/// The names the next `count` newcomers will have: the visitors' names in
/// order, less any anyone here or gone has had.
pub(crate) fn next_names(state: &WorldState, cast: &Cast, count: usize) -> Vec<&'static str> {
    let Some(visitors) = cast.visitors else {
        return Vec::new();
    };
    // Names here are compared whole ("Yusuf Adeyemi") and by first name
    // ("Yusuf"), however the Pack writes its visitors' names.
    // Everyone the World holds counts, gone or not and wherever their id
    // was given (a Pack's own newcomers too), as a stranger at the door
    // takes their name only while nobody holds it.
    let taken = state
        .entities()
        .filter(|entity| entity.component("name").is_some())
        .map(|entity| entity.id)
        .chain(cast_ids(state))
        .flat_map(|id| [name(state, id), first_name(state, id)])
        .chain(waiting_strangers(state, cast))
        .collect::<BTreeSet<_>>();
    visitors
        .names
        .iter()
        .copied()
        .filter(|name| !taken.contains(*name))
        .take(count)
        .collect()
}

/// The kinds of event in which the player meets someone.
pub(crate) const MET_KINDS: [&str; 6] = [
    "greeted",
    "warmed",
    "reacted",
    "situation_came_up",
    "keepsake_left",
    "spoken",
];

/// The kinds of event in which someone crosses the player's path without
/// meeting them: a letter written to them, a favour asked of them (the
/// `conversation` System's `favour_asked`), something of theirs looked
/// after overnight (the `ritual` System's `tended_overnight`). Whoever has
/// done one is met in a quiet day's hello as someone the player knows of,
/// never as a stranger (v0.31 play: Mia asked a favour and watered the
/// player's flowers, then said "We haven't met properly" on day 4). Those
/// Systems' tests hold their kinds to this.
pub const CROSSED_KINDS: [&str; 3] = ["letter_written", "favour_asked", "tended_overnight"];

/// Whether `who` has crossed the player's path ([`CROSSED_KINDS`]).
pub(crate) fn crossed(world: &World, who: EntityId) -> bool {
    world
        .events_of_kind(&CROSSED_KINDS)
        .into_iter()
        .any(|event| event.actor == Some(who))
}

/// Whom the player meets in an event, if anyone.
pub(crate) fn met_in(event: &Event) -> Option<EntityId> {
    match event.kind.as_str() {
        "greeted" | "warmed" | "reacted" | "situation_came_up" | "keepsake_left" => event.actor,
        "spoken" => event.targets.first().copied(),
        _ => None,
    }
}

/// Everyone the player has met: who came over, asked the player
/// something, said what they made of something, left them something, or
/// was spoken to. Found through the World's index of its history by kind,
/// not by reading all of it.
pub fn met(world: &World) -> BTreeSet<EntityId> {
    world
        .events_of_kind(&MET_KINDS)
        .into_iter()
        .filter_map(met_in)
        .collect()
}

/// Whether a rest note (`lives.rest.<family>`, the period the family's
/// trouble last closed) has run out by `now`: the trouble may come up again,
/// just as if the note were not there, so it is no longer worth keeping.
pub(crate) fn rest_over(key: &str, value: &Value, now: u64) -> bool {
    let Some(family) = key.strip_prefix("lives.rest.") else {
        return false;
    };
    let kind = family.split('.').next().and_then(Kind::from_id);
    match (kind, value) {
        (Some(kind), Value::Integer(last)) => now as i64 - last >= kind.rests() as i64,
        _ => false,
    }
}

/// Lines said long enough ago are forgotten, and troubles' rests that have
/// run out let go of, so the notes stay small.
pub(crate) struct Forgets(pub(crate) fn(&WorldState) -> Cast);

impl Action for Forgets {
    fn name(&self) -> &'static str {
        "lives_forget"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0)(state);
        let now = period(state, &cast);
        let notes = state
            .entity(cast.notes)
            .ok_or_else(|| ActionError::Invalid("nothing to forget".into()))?;
        let stale = notes
            .components
            .iter()
            .filter(|(key, value)| {
                if rest_over(key, value, now) {
                    return true;
                }
                // A period's lines.
                let Some(at) = said_period(key) else {
                    return false;
                };
                now.saturating_sub(at) >= HEARD_PERIODS
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        if stale.is_empty() {
            return Err(ActionError::Invalid("nothing to forget".into()));
        }
        let mut draft = EventDraft::new("lines_forgotten");
        draft.changes = stale
            .into_iter()
            .map(|key| StateChange::RemoveComponent {
                entity: cast.notes,
                key,
            })
            .collect();
        Ok(draft)
    }
}
