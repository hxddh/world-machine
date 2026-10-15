//! The morning (v0.30 S4, "one day a day"): a small ritual a new sitting
//! opens on. When the player sits down with a World for a new sitting, its
//! day's morning brings:
//!
//! - a letter written to them, about what they and the place lived through
//!   (the lives System's letters, citing what the player did);
//! - one choice with weight: the day's main question, one of those the
//!   World already puts, never one put on a morning before;
//! - something tended overnight: something the player made, looked after
//!   by someone of the place, or else the place itself;
//! - one find for the book, never the same twice.
//!
//! Each is an Action the World checks and an Event it records, all of them
//! following from the morning, which follows from the day's beginning. A
//! sitting is the host's to know: the World is only told one began
//! ([`world_projection::SITTING_BEGINS_COMMAND`]), never when, so nothing
//! here reads or records a clock. A morning comes at most once a World day,
//! and never on a World's first, which opens on its welcome instead.
//!
//! Never punitive: a day without a sitting costs nothing, and a morning
//! with nothing to bring brings less, never a penalty.

#![forbid(unsafe_code)]

use lives::{first_name, mix, name, period, Cast};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, Event, EventDraft, EventId,
    StateChange, Value, World, WorldError, WorldState,
};
use world_projection::{BookEntry, Letter, Morning, MorningLine, Question, SelectionId};

#[cfg(test)]
mod tests;

/// The morning came: everything else of the morning follows from it.
pub const MORNING_CAME: &str = "morning_came";
/// The day's main question was put.
pub const QUESTION_PUT: &str = "morning_question_put";
/// Something was tended overnight.
pub const TENDED: &str = "tended_overnight";
/// Something turned up for the book.
pub const FOUND: &str = "found_in_the_morning";

/// The shelf the book keeps the morning's finds on.
pub const FINDS_SHELF: &str = "Finds";

/// On the notes: the World day the latest morning came on.
const MORNING_DAY: &str = "ritual.morning";
/// On the notes: what the questions put on mornings asked, word for word.
const ASKED: &str = "ritual.asked";
/// On the notes: every line a morning has tended with.
const TENDED_LINES: &str = "ritual.tended";
/// On the notes: everything found, in the Pack's words.
const FOUND_THINGS: &str = "ritual.found";
/// On the notes, and on what was tended: the World day of the latest
/// question, tending or find.
const QUESTION_DAY: &str = "ritual.question_day";
const TENDED_DAY: &str = "ritual.tended_day";
const FOUND_DAY: &str = "ritual.found_day";
/// On something tended: the World day it was last tended.
const TENDED_ON: &str = "ritual.tended_on";

/// How many put questions and tended lines the notes keep: far more than
/// there are mornings in which one could come round again.
const KEPT: usize = 800;

const MORNING_ACTION: &str = "ritual_morning_comes";
const QUESTION_ACTION: &str = "ritual_puts_question";
const TEND_ACTION: &str = "ritual_tends";
const FIND_ACTION: &str = "ritual_finds";

/// How someone tends something the player made overnight, by the kind of
/// deed that made it. Each line keeps `{who}`, `{thing}` and `{place}`.
const TENDING_PLANTED: [&str; 3] = [
    "Overnight, {who} watered your {thing} by {place}.",
    "Overnight, {who} weeded round your {thing} by {place}.",
    "Overnight, {who} staked up your {thing} by {place}.",
];
const TENDING_BUILT: [&str; 3] = [
    "Overnight, {who} swept round your {thing} by {place}.",
    "Overnight, {who} wiped down your {thing} by {place}.",
    "Overnight, {who} checked every peg of your {thing} by {place}.",
];
const TENDING_DECORATED: [&str; 3] = [
    "Overnight, {who} straightened your {thing} at {place}.",
    "Overnight, {who} dusted off your {thing} at {place}.",
    "Overnight, {who} tied your {thing} back up at {place}.",
];
/// How someone tends the place itself, when the player has made nothing
/// that still stands: `{who}` and `{place}`.
const TENDING_PLACE: [&str; 4] = [
    "Overnight, {who} swept the step of {place}.",
    "Overnight, {who} left a lamp lit at {place}.",
    "Overnight, {who} mended the gate at {place}.",
    "Overnight, {who} set fresh flowers at {place}.",
];
/// What a find says: `{what}` and `{place}`.
const FIND_LINE: &str = "You found {what} by {place}.";

/// What a Pack gives the morning beyond its lives System's cast.
#[derive(Clone, Copy)]
pub struct Rite {
    pub cast: fn(&WorldState) -> Cast,
    /// What can turn up for the book, in the Pack's words ("a gull's
    /// feather"), in the order it turns up. Each turns up once.
    pub finds: &'static [&'static str],
}

pub fn register_actions(registry: &mut ActionRegistry, rite: Rite) -> Result<(), ActionError> {
    registry.register(Comes(rite))?;
    registry.register(Puts(rite))?;
    registry.register(Tends(rite))?;
    registry.register(Finds(rite))?;
    Ok(())
}

fn integer(state: &WorldState, entity: EntityId, key: &str) -> Option<i64> {
    match state.entity(entity)?.component(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

fn texts(state: &WorldState, entity: EntityId, key: &str) -> Vec<String> {
    match state
        .entity(entity)
        .and_then(|entity| entity.component(key))
    {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Text(text) => Some(text.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// `list` with `item` added at the end, keeping the latest `most`.
fn pushed(mut list: Vec<String>, item: &str, most: usize) -> Value {
    list.push(item.to_string());
    let from = list.len().saturating_sub(most);
    Value::List(list.into_iter().skip(from).map(Value::Text).collect())
}

fn arg_text<'a>(request: &'a ActionRequest, key: &str) -> Result<&'a str, ActionError> {
    match request.args.get(key) {
        Some(Value::Text(text)) if !text.trim().is_empty() => Ok(text),
        _ => Err(ActionError::Invalid(format!("missing {key}"))),
    }
}

fn arg_entity(request: &ActionRequest, key: &str) -> Option<EntityId> {
    match request.args.get(key) {
        Some(Value::Entity(id)) => Some(*id),
        _ => None,
    }
}

fn fill(line: &str, words: &[(&str, &str)]) -> String {
    words.iter().fold(line.to_string(), |line, (slot, word)| {
        line.replace(&format!("{{{slot}}}"), word)
    })
}

/// The World day it is, as the lives System counts it.
pub fn today(state: &WorldState, cast: &Cast) -> u64 {
    period(state, cast)
}

/// Whether this World day's morning has come.
pub fn morning_came_today(state: &WorldState, cast: &Cast) -> bool {
    integer(state, cast.notes, MORNING_DAY) == Some(today(state, cast) as i64)
}

/// A part of the morning is checked the same way: the morning came today,
/// and this part has not yet.
fn this_morning(state: &WorldState, cast: &Cast, done_on: &str) -> Result<i64, ActionError> {
    if !morning_came_today(state, cast) {
        return Err(ActionError::Invalid("the morning has not come".into()));
    }
    let today = today(state, cast) as i64;
    if integer(state, cast.notes, done_on) == Some(today) {
        return Err(ActionError::Invalid("already this morning".into()));
    }
    Ok(today)
}

/// The morning comes, once a World day and never on its first.
struct Comes(Rite);

impl Action for Comes {
    fn name(&self) -> &'static str {
        MORNING_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0.cast)(state);
        let today = today(state, &cast) as i64;
        match request.args.get("day") {
            Some(Value::Integer(day)) if *day == today => {}
            _ => return Err(ActionError::Invalid("not this day's morning".into())),
        }
        if today <= 1 {
            return Err(ActionError::Invalid(
                "a World's first day opens on its welcome".into(),
            ));
        }
        if state.entity(cast.notes).is_none() {
            return Err(ActionError::Invalid("nowhere to keep the morning".into()));
        }
        if integer(state, cast.notes, MORNING_DAY).is_some_and(|last| last >= today) {
            return Err(ActionError::Invalid("this day's morning has come".into()));
        }
        let mut draft = EventDraft::new(MORNING_CAME);
        draft.payload.insert("day".into(), today.into());
        draft.changes.push(StateChange::SetComponent {
            entity: cast.notes,
            key: MORNING_DAY.into(),
            value: today.into(),
        });
        Ok(draft)
    }
}

/// The day's main question is put: one the World asks already, never one
/// a morning put before.
struct Puts(Rite);

impl Action for Puts {
    fn name(&self) -> &'static str {
        QUESTION_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0.cast)(state);
        let today = this_morning(state, &cast, QUESTION_DAY)?;
        let question = arg_text(request, "question")?;
        let prompt = arg_text(request, "prompt")?;
        let asked = texts(state, cast.notes, ASKED);
        if asked.iter().any(|asked| asked == prompt) {
            return Err(ActionError::Invalid("put on a morning before".into()));
        }
        let mut draft = EventDraft::new(QUESTION_PUT);
        draft.payload.insert("question".into(), question.into());
        draft.payload.insert("prompt".into(), prompt.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: cast.notes,
                key: ASKED.into(),
                value: pushed(asked, prompt, KEPT),
            },
            StateChange::SetComponent {
                entity: cast.notes,
                key: QUESTION_DAY.into(),
                value: today.into(),
            },
        ];
        Ok(draft)
    }
}

/// Something is tended overnight by someone of the place: a line never
/// told on a morning before.
struct Tends(Rite);

impl Action for Tends {
    fn name(&self) -> &'static str {
        TEND_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0.cast)(state);
        let today = this_morning(state, &cast, TENDED_DAY)?;
        let line = arg_text(request, "line")?;
        let who = arg_entity(request, "who")
            .filter(|who| lives::enrolled(state, *who) && !lives::gone(state, *who))
            .ok_or_else(|| ActionError::Invalid("nobody to tend it".into()))?;
        let lines = texts(state, cast.notes, TENDED_LINES);
        if lines.iter().any(|told| told == line) {
            return Err(ActionError::Invalid("told on a morning before".into()));
        }
        let thing = arg_entity(request, "thing").filter(|thing| state.entity(*thing).is_some());
        let mut draft = EventDraft::new(TENDED);
        draft.actor = Some(who);
        draft.targets = std::iter::once(who).chain(thing).collect();
        draft.payload.insert("told".into(), line.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: cast.notes,
                key: TENDED_LINES.into(),
                value: pushed(lines, line, KEPT),
            },
            StateChange::SetComponent {
                entity: cast.notes,
                key: TENDED_DAY.into(),
                value: today.into(),
            },
        ];
        if let Some(thing) = thing {
            draft.changes.push(StateChange::SetComponent {
                entity: thing,
                key: TENDED_ON.into(),
                value: today.into(),
            });
        }
        Ok(draft)
    }
}

/// Something turns up for the book: one of the Pack's finds, each once.
struct Finds(Rite);

impl Action for Finds {
    fn name(&self) -> &'static str {
        FIND_ACTION
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let cast = (self.0.cast)(state);
        let today = this_morning(state, &cast, FOUND_DAY)?;
        let what = arg_text(request, "what")?;
        if !self.0.finds.contains(&what) {
            return Err(ActionError::Invalid("nothing like that to find".into()));
        }
        let found = texts(state, cast.notes, FOUND_THINGS);
        if found.iter().any(|thing| thing == what) {
            return Err(ActionError::Invalid("found before".into()));
        }
        let at = arg_entity(request, "at")
            .filter(|at| state.entity(*at).is_some())
            .ok_or_else(|| ActionError::Invalid("found nowhere".into()))?;
        let line = fill(FIND_LINE, &[("what", what), ("place", &name(state, at))]);
        let mut draft = EventDraft::new(FOUND);
        draft.targets = vec![at];
        draft.payload.insert("find".into(), what.into());
        draft.payload.insert("told".into(), line.into());
        draft.changes = vec![
            StateChange::SetComponent {
                entity: cast.notes,
                key: FOUND_THINGS.into(),
                value: pushed(found, what, usize::MAX),
            },
            StateChange::SetComponent {
                entity: cast.notes,
                key: FOUND_DAY.into(),
                value: today.into(),
            },
        ];
        Ok(draft)
    }
}

/// The people of the place who can tend something, in a fixed order.
fn tenders(world: &World, cast: &Cast) -> Vec<EntityId> {
    let state = world.state();
    (cast.people)(world)
        .into_iter()
        .filter(|person| lives::enrolled(state, *person) && !lives::gone(state, *person))
        .collect()
}

/// What the player made that still stands, with how it is tended: by the
/// kind of deed that made it.
fn made_things(world: &World) -> Vec<(EntityId, &'static [&'static str])> {
    let state = world.state();
    let deeds = world.events_of_kind(&["planted_by_hand", "built_by_hand", "decorated_by_hand"]);
    hands::made(state)
        .into_iter()
        .filter_map(|thing| {
            let deed = deeds
                .iter()
                .rev()
                .find(|event| event.targets.first() == Some(&thing))?;
            let lines: &'static [&'static str] = match deed.kind.as_str() {
                "planted_by_hand" => &TENDING_PLANTED,
                "decorated_by_hand" => &TENDING_DECORATED,
                _ => &TENDING_BUILT,
            };
            Some((thing, lines))
        })
        .collect()
}

/// Whether a thing's name already says where it is ("the green flag" by
/// "the green"): a word of the place's name, past its article, is in it.
fn names_its_place(thing: &str, place: &str) -> bool {
    place
        .to_lowercase()
        .split_whitespace()
        .filter(|word| word.len() > 3)
        .any(|word| thing.split_whitespace().any(|own| own == word))
}

/// A tending line that leaves the place out: "… your {thing} by {place}."
/// as "… your {thing}."
fn placeless(line: &str) -> String {
    line.replace(" by {place}", "").replace(" at {place}", "")
}

fn place_of(state: &WorldState, thing: EntityId) -> Option<EntityId> {
    match state.entity(thing)?.component("at")? {
        Value::Entity(at) if state.entity(*at).is_some() => Some(*at),
        _ => None,
    }
}

/// The places of the town a morning can be about: where people meet,
/// where they go to be quiet, and where each works and lives.
fn places(world: &World, cast: &Cast) -> Vec<EntityId> {
    let state = world.state();
    let mut places = vec![cast.gathering, cast.quiet];
    for person in tenders(world, cast) {
        places.extend((cast.work)(state, person));
        places.extend((cast.home)(state, person));
    }
    let mut seen = std::collections::BTreeSet::new();
    places.retain(|place| state.entity(*place).is_some() && seen.insert(*place));
    places
}

/// What a morning tends: a line never told before, by someone of the
/// place, about something the player made if anything still stands, else
/// about the place itself. The fewest-tended first, so each thing has its
/// turn.
fn tending(world: &World, cast: &Cast, today: u64) -> Option<ActionRequest> {
    let state = world.state();
    let told = texts(state, cast.notes, TENDED_LINES);
    let people = tenders(world, cast);
    if people.is_empty() {
        return None;
    }
    let seed = mix(&[today, 211]);
    let who_at = |n: usize| people[(seed as usize + n) % people.len()];
    let mut things = made_things(world);
    things.sort_by_key(|(thing, _)| {
        (
            integer(state, *thing, TENDED_ON).unwrap_or(-1),
            mix(&[thing.0, today, 223]),
        )
    });
    for (thing, lines) in &things {
        let what = name(state, *thing).to_lowercase();
        let place = place_of(state, *thing).unwrap_or(cast.gathering);
        let place = name(state, place);
        let named = names_its_place(&what, &place);
        for n in 0..people.len() * lines.len() {
            let who = who_at(n / lines.len());
            let line = lines[(seed as usize + n) % lines.len()];
            // "your green flag by the green" says the place once.
            let line = if named {
                placeless(line)
            } else {
                line.to_string()
            };
            let line = fill(
                &line,
                &[
                    ("who", &first_name(state, who)),
                    ("thing", &what),
                    ("place", &place),
                ],
            );
            if !told.contains(&line) {
                return Some(
                    ActionRequest::new(TEND_ACTION)
                        .actor(who)
                        .arg("who", Value::Entity(who))
                        .arg("thing", Value::Entity(*thing))
                        .arg("line", line),
                );
            }
        }
    }
    // Nothing of the player's to tend: someone looks after the place.
    let places = places(world, cast);
    for n in 0..people.len() * TENDING_PLACE.len() * places.len().max(1) {
        let who = who_at(n % people.len());
        let line = TENDING_PLACE[(seed as usize + n / people.len()) % TENDING_PLACE.len()];
        let place = places.get(
            (seed as usize + n / (people.len() * TENDING_PLACE.len())) % places.len().max(1),
        )?;
        let line = fill(
            line,
            &[
                ("who", &first_name(state, who)),
                ("place", &name(state, *place)),
            ],
        );
        if !told.contains(&line) {
            return Some(
                ActionRequest::new(TEND_ACTION)
                    .actor(who)
                    .arg("who", Value::Entity(who))
                    .arg("thing", Value::Entity(*place))
                    .arg("line", line),
            );
        }
    }
    None
}

/// What turns up for the book this morning: the next of the Pack's finds
/// not yet found, somewhere in the place.
fn find(world: &World, cast: &Cast, rite: &Rite, today: u64) -> Option<ActionRequest> {
    let state = world.state();
    let found = texts(state, cast.notes, FOUND_THINGS);
    let what = rite
        .finds
        .iter()
        .find(|what| !found.iter().any(|thing| thing == **what))?;
    let places = places(world, cast);
    let at = *places.get((mix(&[today, 227]) % places.len().max(1) as u64) as usize)?;
    Some(
        ActionRequest::new(FIND_ACTION)
            .arg("what", *what)
            .arg("at", Value::Entity(at)),
    )
}

/// The day's main question, of the questions the World puts now (oldest
/// first): the first never put on a morning before.
fn question(state: &WorldState, cast: &Cast, questions: &[Question]) -> Option<ActionRequest> {
    let asked = texts(state, cast.notes, ASKED);
    let question = questions
        .iter()
        .find(|question| !asked.contains(&question.prompt))?;
    Some(
        ActionRequest::new(QUESTION_ACTION)
            .arg("question", question.id.clone())
            .arg("prompt", question.prompt.clone()),
    )
}

/// The questions a snapshot's commands put, each once, in their order:
/// only those the player can answer now.
pub fn questions_put(commands: &[world_projection::ProjectionCommand]) -> Vec<Question> {
    let mut questions: Vec<Question> = Vec::new();
    for command in commands {
        let Some(question) = &command.question else {
            continue;
        };
        if command.unavailable.is_some() || questions.iter().any(|q| q.id == question.id) {
            continue;
        }
        questions.push(question.clone());
    }
    questions
}

/// The morning of this World day, when a sitting begins on it: the morning
/// comes, after the day's beginning (`cause`), and its letter, question,
/// tending and find follow from it, each as the World's rules allow.
/// `None` when the day has had its morning, or it is the World's first.
pub fn morning(
    world: &mut World,
    actions: &ActionRegistry,
    rite: &Rite,
    cause: Option<EventId>,
    questions: &[Question],
) -> Result<Option<EventId>, WorldError> {
    let cast = (rite.cast)(world.state());
    let today = today(world.state(), &cast);
    let request = ActionRequest::new(MORNING_ACTION).arg("day", today as i64);
    let request = match cause {
        Some(cause) => request.caused_by(cause),
        None => request,
    };
    let Ok(came) = world.execute(actions, &request).map(|event| event.id) else {
        return Ok(None);
    };
    world.following(came, |world| {
        // Each part is offered and checked; one the World refuses is left
        // out, and the morning brings the rest. Each is caused by the
        // morning, and a letter about something the player did by that too.
        let parts = [
            lives::morning_letter(world, &cast),
            question(world.state(), &cast, questions),
            tending(world, &cast, today),
            find(world, &cast, rite, today),
        ];
        for request in parts.into_iter().flatten() {
            let _ = world.execute(actions, &request.caused_by(came));
        }
    });
    Ok(Some(came))
}

/// The latest morning, if it came this World day, and what followed from
/// it.
fn this_days_morning<'a>(world: &'a World, cast: &Cast) -> Option<(&'a Event, Vec<&'a Event>)> {
    if !morning_came_today(world.state(), cast) {
        return None;
    }
    let came = *world.events_of_kind(&[MORNING_CAME]).last()?;
    let events = world.events();
    let from = events.partition_point(|event| event.id <= came.id);
    let parts = events[from..]
        .iter()
        .take_while(|event| event.world_time == came.world_time)
        .filter(|event| event.caused_by.contains(&came.id))
        .collect();
    Some((came, parts))
}

fn told(event: &Event) -> Option<String> {
    match event.payload.get("told") {
        Some(Value::Text(told)) if !told.is_empty() => Some(told.clone()),
        _ => None,
    }
}

/// This World day's morning as the window shows it, when a sitting began
/// on it: its main question only while `commands` still put it.
pub fn shown(
    world: &World,
    cast: &Cast,
    commands: &[world_projection::ProjectionCommand],
) -> Option<Morning> {
    let (came, parts) = this_days_morning(world, cast)?;
    let open = questions_put(commands);
    let mut morning = Morning {
        moment: SelectionId::Event(came.id),
        letter: None,
        question: None,
        tended: None,
        find: None,
    };
    for part in parts {
        match part.kind.as_str() {
            "letter_written" => {
                morning.letter = Some(Letter {
                    from: SelectionId::Entity(part.actor?),
                    note: match part.payload.get("said") {
                        Some(Value::Text(said)) => said.clone(),
                        _ => String::new(),
                    },
                    moment: SelectionId::Event(part.id),
                })
            }
            QUESTION_PUT => {
                let id = match part.payload.get("question") {
                    Some(Value::Text(id)) => id,
                    _ => continue,
                };
                morning.question = open.iter().find(|question| question.id == *id).cloned();
            }
            TENDED => {
                morning.tended = told(part).map(|line| MorningLine {
                    line,
                    about: part
                        .targets
                        .get(1)
                        .or(part.targets.first())
                        .copied()
                        .map(SelectionId::Entity),
                })
            }
            FOUND => {
                morning.find = told(part).map(|line| MorningLine {
                    line,
                    about: part.targets.first().copied().map(SelectionId::Entity),
                })
            }
            _ => {}
        }
    }
    Some(morning)
}

/// What the mornings have turned up, oldest first, as the book keeps it.
pub fn book(world: &World) -> Vec<BookEntry> {
    world
        .events_of_kind(&[FOUND])
        .into_iter()
        .filter_map(|event| {
            let what = match event.payload.get("find") {
                Some(Value::Text(what)) => what,
                _ => return None,
            };
            let mut letters = what.chars();
            let first = letters.next()?;
            Some(BookEntry {
                shelf: FINDS_SHELF.into(),
                name: first.to_uppercase().chain(letters).collect(),
                found: true,
                cast: event
                    .targets
                    .first()
                    .copied()
                    .map(SelectionId::Entity)
                    .into_iter()
                    .collect(),
                ..BookEntry::default()
            })
        })
        .collect()
}

/// Every line the morning tells in, as written, slots and all: for the
/// catalogs that put them into other languages.
pub fn lines() -> Vec<String> {
    let made = || {
        TENDING_PLANTED
            .iter()
            .chain(&TENDING_BUILT)
            .chain(&TENDING_DECORATED)
    };
    made()
        .map(|line| line.to_string())
        .chain(made().map(|line| placeless(line)))
        .chain(TENDING_PLACE.iter().map(|line| line.to_string()))
        .chain([FIND_LINE.to_string(), FINDS_SHELF.to_string()])
        .collect()
}
