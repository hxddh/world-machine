//! What the player is given: letters, keepsakes, and how often they
//! come.

use crate::*;

/// A letter from someone, as the letter box names it.
pub fn letter_name(state: &WorldState, person: EntityId) -> String {
    fill_owned(LETTER, &[("name", first_name(state, person))])
}

/// A letter someone wrote the player: who, what it says, and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Letter {
    pub from: EntityId,
    pub note: String,
    pub event: EventId,
    pub world_time: u64,
}

/// The kinds of event a letter comes in.
pub(crate) const LETTER_KINDS: [&str; 2] = ["letter_written", "guest_visited"];

/// Whether an event of this kind has ever been recorded, asked of the
/// World's index of its history rather than by reading it.
pub(crate) fn happened(world: &World, kind: &str) -> bool {
    !world.history_index().of_kind(kind).is_empty()
}

/// The letter an event brings, if it brings one.
pub(crate) fn letter_of(event: &Event) -> Option<Letter> {
    if !LETTER_KINDS.contains(&event.kind.as_str()) {
        return None;
    }
    // A guest's letter is passed on by whoever welcomed them.
    let key = if event.kind == "guest_visited" {
        "note"
    } else {
        "said"
    };
    Some(Letter {
        from: event.actor?,
        note: match event.payload.get(key) {
            Some(Value::Text(text)) => text.clone(),
            _ => String::new(),
        },
        event: event.id,
        world_time: event.world_time,
    })
}

/// Every letter the player has been written, oldest first: found through
/// the World's index of its history by kind, so asking costs the same
/// however long the World has lived.
pub fn letters(world: &World) -> Vec<Letter> {
    world
        .events_of_kind(&LETTER_KINDS)
        .into_iter()
        .filter_map(letter_of)
        .collect()
}

/// Everyone who could write to the player, by the letter box's name for
/// their letter, and whether they have yet.
pub fn letter_writers(world: &World, cast: &Cast) -> Vec<(String, bool)> {
    let wrote = world
        .events_of_kind(&["letter_written"])
        .into_iter()
        .filter_map(|event| event.actor)
        .collect::<BTreeSet<_>>();
    let state = world.state();
    (cast.people)(world)
        .into_iter()
        .map(|person| (letter_name(state, person), wrote.contains(&person)))
        .collect()
}

/// The most keepsakes a week, so each one still means something.
pub const KEEPSAKES_A_WEEK: usize = 3;

/// Whether there is room for another keepsake this week. One place in the
/// week is kept for a friend's keepsake the player may still be answering
/// for, so a week never holds more than [`KEEPSAKES_A_WEEK`].
pub fn room_for_keepsake(world: &World, cast: &Cast) -> bool {
    let since = world.world_time().saturating_sub(cast.period * 7);
    // Only the week's tail of the history is read.
    let events = world.events();
    let from = events.partition_point(|event| event.world_time <= since);
    events[from..].iter().filter_map(keepsake_of_event).count() < KEEPSAKES_A_WEEK - 1
}

/// Whether something new came the player's way since the last period's
/// round of lives ended: something to keep, or someone met for the first
/// time. What the player did after that round, and what this round
/// brought, both count; kept `strict`ly, only what this period has brought
/// counts, and someone already spoken to is not met anew.
pub(crate) fn something_new_lately(world: &World, cast: &Cast, strict: bool) -> bool {
    let since = world.world_time().saturating_sub(cast.period);
    let events = world.events();
    let today = events.partition_point(|event| event.world_time <= since);
    // The last period's round ends with its last life lived, situation
    // put, warmth shared, or quiet-day meeting or letter.
    let round_ended = events[..today]
        .iter()
        .rposition(|event| {
            matches!(
                event.kind.as_str(),
                "lived" | "situation_came_up" | "warmed" | "greeted"
            ) || event.payload.get("letter") == Some(&Value::Bool(true))
                || event.payload.get("first") == Some(&Value::Bool(true))
        })
        .map_or(0, |index| index + 1);
    // Kept strictly, only what this period itself has brought so far
    // counts: whatever else came after the last round (the rest of that
    // day, and what the player did before this one) was that day's news.
    let round_ended = if strict {
        events.partition_point(|event| event.world_time < world.world_time())
    } else {
        round_ended
    };
    let (before, lately) = events.split_at(round_ended);
    let kept = |event: &Event| {
        event.kind == "keepsake_left"
            || event.kind == "letter_written"
            || event.payload.get("kept") == Some(&Value::Bool(true))
            || (event.kind == "situation_answered"
                && event.payload.get("kind") == Some(&Value::Text("keepsake".into())))
    };
    if lately.iter().any(kept) {
        return true;
    }
    let meets = |event: &&Event| {
        matches!(
            event.kind.as_str(),
            "greeted"
                | "warmed"
                | "reacted"
                | "situation_came_up"
                | "keepsake_left"
                | "letter_written"
        )
    };
    let newly = lately
        .iter()
        .filter(meets)
        .filter_map(|event| event.actor)
        .collect::<BTreeSet<_>>();
    if newly.is_empty() {
        return false;
    }
    let mut known = before
        .iter()
        .filter(meets)
        .filter_map(|event| event.actor)
        .collect::<BTreeSet<_>>();
    // Kept strictly, someone the player has already spoken to is not met
    // for the first time when they ask something.
    if strict {
        known.extend(
            before
                .iter()
                .filter(|event| event.kind == "spoken")
                .filter_map(|event| event.targets.first().copied()),
        );
    }
    newly.iter().any(|person| !known.contains(person))
}

/// No day passes with nothing new: on a day that brought the player
/// nothing to keep and nobody new, someone they have not met comes over
/// to say hello, or, once they know everyone (or while they are away),
/// someone writes to them. At most one a day; a World not yet begun is
/// left alone.
pub fn daily(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    away: bool,
) -> Result<Option<EventId>, WorldError> {
    daily_with(world, actions, cast, away, &QuietDays::default())
}

/// A quiet day as [`daily`] keeps it, within a Pack's own limits: with a
/// most letters a week, the other quiet days bring a small first instead
/// (see [`QuietDays`]).
pub fn daily_with(
    world: &mut World,
    actions: &ActionRegistry,
    cast: &Cast,
    away: bool,
    quiet: &QuietDays,
) -> Result<Option<EventId>, WorldError> {
    if !happened(world, "greeted")
        || something_new_lately(world, cast, quiet.letters_a_week.is_some())
    {
        return Ok(None);
    }
    let met = met(world);
    let state = world.state();
    let people = (cast.people)(world)
        .into_iter()
        .filter(|person| enrolled(state, *person) && !gone(state, *person))
        .collect::<Vec<_>>();
    if let Some(stranger) = people.iter().find(|person| !away && !met.contains(person)) {
        let mut request = ActionRequest::new("lives_greets")
            .actor(*stranger)
            .arg("who", Value::Entity(*stranger))
            .arg("later", "a quiet day");
        // Someone who has written, asked a favour or looked after what is
        // the player's says hello as to someone they know of.
        if strangers::crossed(world, *stranger) {
            request = request.arg("known", "crossed paths");
        }
        return Ok(world.execute(actions, &request).ok().map(|event| event.id));
    }
    if let Some(most) = quiet.letters_a_week {
        return quiet::quiet_day(world, actions, cast, away, quiet, most, &people);
    }
    let now = period(state, cast);
    // Whoever has written least, the fondest first: everyone writes once
    // before anyone writes twice.
    let Some(writer) = people.iter().copied().max_by_key(|person| {
        (
            -integer(state, *person, WROTE).unwrap_or(0),
            regard(state, *person),
            mix(&[person.0, now, 53]),
        )
    }) else {
        return Ok(None);
    };
    let first = first_name(state, writer);
    let words = [
        ("name", first.clone()),
        ("settlement", cast.settlement.to_string()),
        ("gathering", name(state, cast.gathering)),
        ("unit", cast.unit.to_string()),
    ];
    let seed = mix(&[writer.0, now, 59]);
    let opening = pick(&LETTER_OPENINGS, seed).copied().unwrap_or_default();
    let closing = pick(&LETTER_CLOSINGS, seed >> 8)
        .map(|line| fill_owned(line, &words))
        .unwrap_or_default();
    let news = said_today(world, writer)
        .map(|line| format!(" {line}"))
        .unwrap_or_default();
    let note = format!("{opening}{news} {closing}");
    let request = ActionRequest::new("lives_leaves_keepsake")
        .actor(writer)
        .arg("who", Value::Entity(writer))
        .arg("why", note)
        .arg("letter", "yes");
    Ok(world.execute(actions, &request).ok().map(|event| event.id))
}

/// Something the player was given to keep: who gave it, what it is, what
/// they said with it, and when.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keepsake {
    pub from: EntityId,
    pub what: String,
    pub note: String,
    pub event: EventId,
    pub world_time: u64,
}

/// The kinds of event something to keep comes in: what someone leaves,
/// an answer that gives something, a guest's gift, and what a garden the
/// player planted grew. A System that marks something else as given to
/// keep (`"kept": true`) adds its kind here.
pub(crate) const KEPT_KINDS: [&str; 4] = [
    "keepsake_left",
    "situation_answered",
    "guest_visited",
    "enjoyed",
];

/// The keepsake an event gives the player, if it gives one.
pub(crate) fn keepsake_of_event(event: &Event) -> Option<Keepsake> {
    let kept = event.kind == "keepsake_left"
        || (event.kind == "situation_answered"
            && event.payload.get("kind") == Some(&Value::Text("keepsake".into())))
        // Anything any System marks as given to keep: what a garden the
        // player planted grew, say.
        || event.payload.get("kept") == Some(&Value::Bool(true));
    if !kept {
        return None;
    }
    let text = |key: &str| match event.payload.get(key) {
        Some(Value::Text(text)) => Some(text.clone()),
        _ => None,
    };
    Some(Keepsake {
        from: event.actor?,
        what: text("keepsake")?,
        note: text("said").unwrap_or_default(),
        event: event.id,
        world_time: event.world_time,
    })
}

/// Everything the player has been given to keep, oldest first: found
/// through the World's index of its history by kind, so asking costs the
/// same however long the World has lived.
pub fn keepsakes(world: &World) -> Vec<Keepsake> {
    // Kept with the World and brought up to date with the events recorded
    // since: a recorded event never changes, and one recorded after has a
    // later id than any before it, so the keepsakes found stay found, in
    // the order of their ids.
    let events = world.events();
    let kept = world.derived::<KeptKeepsakes>(|kept| {
        if let Some(kept) = kept {
            if kept.read == events.len() && events.last().map(|event| event.id) == kept.last {
                return kept;
            }
            if kept.read < events.len()
                && events[..kept.read].last().map(|event| event.id) == kept.last
            {
                let newer = &events[kept.read..];
                let mut newest = kept.newest;
                let later = newer.iter().all(|event| {
                    let later = newest.is_none_or(|newest| event.id > newest);
                    newest = Some(event.id);
                    later
                });
                if later {
                    let mut keepsakes = kept.keepsakes.clone();
                    keepsakes.extend(
                        newer
                            .iter()
                            .filter(|event| KEPT_KINDS.contains(&event.kind.as_str()))
                            .filter_map(keepsake_of_event),
                    );
                    return std::sync::Arc::new(KeptKeepsakes {
                        read: events.len(),
                        last: events.last().map(|event| event.id),
                        newest,
                        keepsakes,
                    });
                }
            }
        }
        std::sync::Arc::new(KeptKeepsakes {
            read: events.len(),
            last: events.last().map(|event| event.id),
            newest: events.iter().map(|event| event.id).max(),
            keepsakes: world
                .events_of_kind(&KEPT_KINDS)
                .into_iter()
                .filter_map(keepsake_of_event)
                .collect(),
        })
    });
    kept.keepsakes.clone()
}

/// The keepsakes found in a World's first `read` events: the last of them
/// `last`, the latest id among them `newest`.
pub(crate) struct KeptKeepsakes {
    pub(crate) read: usize,
    pub(crate) last: Option<EventId>,
    pub(crate) newest: Option<EventId>,
    pub(crate) keepsakes: Vec<Keepsake>,
}

/// Every keepsake someone living here now could give the player, in the
/// World's words, each with a word on how it comes: the welcome, what is
/// left while the player is away, and each person's own.
pub fn possible_keepsakes(world: &World, cast: &Cast) -> Vec<(String, String)> {
    let state = world.state();
    let words = [
        ("gathering", name(state, cast.gathering)),
        ("settlement", cast.settlement.to_string()),
        ("unit", cast.unit.to_string()),
    ];
    let mut all = Vec::new();
    // One welcome is given: the one that was, or the first while none has.
    let welcomes = WELCOME
        .iter()
        .map(|what| fill_owned(what, &words))
        .collect::<Vec<_>>();
    let kept = keepsakes(world)
        .into_iter()
        .map(|kept| kept.what)
        .collect::<BTreeSet<_>>();
    let welcome = welcomes
        .iter()
        .find(|what| kept.contains(*what))
        .or(welcomes.first())
        .cloned()
        .unwrap_or_default();
    all.push((welcome, "Given for your first work here".to_string()));
    for what in LEFT_FOR_YOU {
        all.push((
            fill_owned(what, &words),
            "Left for you as a season turns".to_string(),
        ));
    }
    for person in (cast.people)(world) {
        let what = fill_owned(
            keepsake_of((cast.voice)(person), (cast.traits)(person)),
            &words,
        );
        all.push((
            what,
            format!("From {}, once you are close", first_name(state, person)),
        ));
    }
    let mut seen = BTreeSet::new();
    all.retain(|(what, _)| seen.insert(what.clone()));
    all
}
