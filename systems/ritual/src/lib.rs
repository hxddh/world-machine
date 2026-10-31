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
/// On the notes: the names of what the latest mornings saw to, so neither
/// that nor another of the same name is seen to again straight after.
const TENDED_WHAT: &str = "ritual.tended_what";
/// How many of those names are kept.
const KEPT_WHAT: usize = 2;

/// How many put questions and tended lines the notes keep: far more than
/// there are mornings in which one could come round again.
const KEPT: usize = 800;

const MORNING_ACTION: &str = "ritual_morning_comes";
const QUESTION_ACTION: &str = "ritual_puts_question";
const TEND_ACTION: &str = "ritual_tends";
const FIND_ACTION: &str = "ritual_finds";

/// How someone tends something the player made overnight, by the kind of
/// deed that made it, whatever the season. Each line keeps `{who}`,
/// `{thing}` and `{place}`.
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
/// How someone lends a hand overnight with something the player began that
/// is still going up.
const TENDING_RISING: [&str; 3] = [
    "Overnight, {who} carried timber over for your {thing} by {place}.",
    "Overnight, {who} put up another beam of your {thing} by {place}.",
    "Overnight, {who} checked every joint of your {thing} by {place}.",
];
const TENDING_DECORATED: [&str; 3] = [
    "Overnight, {who} straightened your {thing} at {place}.",
    "Overnight, {who} dusted off your {thing} at {place}.",
    "Overnight, {who} tied your {thing} back up at {place}.",
];
/// The same, as each season has it (spring, summer, autumn, winter), in a
/// World with seasons.
const PLANTED_IN: [[&str; 3]; 4] = [
    [
        "Overnight, {who} pulled the spring weeds from round your {thing} by {place}.",
        "Overnight, {who} pinched out the first buds of your {thing} by {place}.",
        "Overnight, {who} covered your {thing} by {place} against a late frost.",
    ],
    [
        "Overnight, {who} carried water out to your {thing} by {place}.",
        "Overnight, {who} shaded your {thing} by {place} from the heat.",
        "Overnight, {who} deadheaded your {thing} by {place}.",
    ],
    [
        "Overnight, {who} raked the fallen leaves off your {thing} by {place}.",
        "Overnight, {who} saved seed from your {thing} by {place}.",
        "Overnight, {who} mulched round your {thing} by {place}.",
    ],
    [
        "Overnight, {who} wrapped your {thing} by {place} in sacking against the frost.",
        "Overnight, {who} shook the snow off your {thing} by {place}.",
        "Overnight, {who} banked straw round your {thing} by {place}.",
    ],
];
const BUILT_IN: [[&str; 3]; 4] = [
    [
        "Overnight, {who} scrubbed the winter grime off your {thing} by {place}.",
        "Overnight, {who} gave your {thing} by {place} a fresh coat of oil.",
        "Overnight, {who} tightened the loose joints of your {thing} by {place}.",
    ],
    [
        "Overnight, {who} wiped the dust off your {thing} by {place}.",
        "Overnight, {who} touched up the paint on your {thing} by {place}.",
        "Overnight, {who} cleared the cobwebs from your {thing} by {place}.",
    ],
    [
        "Overnight, {who} swept the leaves off your {thing} by {place}.",
        "Overnight, {who} dried off your {thing} by {place} after the rain.",
        "Overnight, {who} tarred the feet of your {thing} by {place} against the wet.",
    ],
    [
        "Overnight, {who} cleared the snow off your {thing} by {place}.",
        "Overnight, {who} knocked the ice off your {thing} by {place}.",
        "Overnight, {who} scattered grit round your {thing} by {place}.",
    ],
];
const DECORATED_IN: [[&str; 3]; 4] = [
    [
        "Overnight, {who} tucked a sprig of blossom into your {thing} at {place}.",
        "Overnight, {who} rehung your {thing} at {place} after the spring winds.",
        "Overnight, {who} brushed the pollen off your {thing} at {place}.",
    ],
    [
        "Overnight, {who} turned your {thing} at {place} out of the sun.",
        "Overnight, {who} wound fresh ribbon round your {thing} at {place}.",
        "Overnight, {who} shooed the wasps away from your {thing} at {place}.",
    ],
    [
        "Overnight, {who} picked the leaves out of your {thing} at {place}.",
        "Overnight, {who} weighted your {thing} at {place} against the gales.",
        "Overnight, {who} hung a lantern by your {thing} at {place}.",
    ],
    [
        "Overnight, {who} brushed the frost off your {thing} at {place}.",
        "Overnight, {who} put a sprig of holly on your {thing} at {place}.",
        "Overnight, {who} shook the snow off your {thing} at {place}.",
    ],
];
/// How someone tends the place itself, when there is nothing of the
/// player's to see to: `{who}` and `{place}`.
const TENDING_PLACE: [&str; 6] = [
    "Overnight, {who} swept the step of {place}.",
    "Overnight, {who} left a lamp lit at {place}.",
    "Overnight, {who} mended the gate at {place}.",
    "Overnight, {who} set fresh flowers at {place}.",
    "Overnight, {who} oiled the hinges at {place}.",
    "Overnight, {who} wiped down the tables at {place}.",
];
/// The same, as each season has it.
const PLACE_IN: [[&str; 4]; 4] = [
    [
        "Overnight, {who} washed the windows of {place}.",
        "Overnight, {who} planted up a window box at {place}.",
        "Overnight, {who} swept the blossom from the step of {place}.",
        "Overnight, {who} propped the door of {place} open to let the spring in.",
    ],
    [
        "Overnight, {who} watered the tubs outside {place}.",
        "Overnight, {who} put chairs out in the shade at {place}.",
        "Overnight, {who} left a jug of cold water out at {place}.",
        "Overnight, {who} hung the rugs of {place} out to air.",
    ],
    [
        "Overnight, {who} raked the leaves from the step of {place}.",
        "Overnight, {who} stacked firewood by {place}.",
        "Overnight, {who} cleared the gutters of {place}.",
        "Overnight, {who} hung a lantern at {place} for the dark evenings.",
    ],
    [
        "Overnight, {who} cleared the snow from the step of {place}.",
        "Overnight, {who} gritted the path to {place}.",
        "Overnight, {who} kept the stove going at {place}.",
        "Overnight, {who} hung a wreath on the door of {place}.",
    ],
];
/// What a find says, each in turn: `{what}` and `{place}`.
const FIND_LINES: [&str; 4] = [
    "You found {what} by {place}.",
    "Down by {place}, you found {what}.",
    "You came across {what} near {place}.",
    "By {place}, {what} turned up for your book.",
];

/// The fewest World days before a morning sees to the same thing the
/// player made again, so what is looked after moves round the place;
/// what is still going up is seen to every other day.
const REST_MADE: i64 = 3;
const REST_RISING: i64 = 2;
/// The fewest World days before a place is seen to again.
const REST_PLACE: i64 = 2;

/// Something a morning can turn up for the book: what it is, in the
/// Pack's words ("a gull's feather"); the season it turns up in, from 0
/// (spring) to 3 (winter), or any; and the place it turns up by, by the
/// name the World gives it, or anywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Find {
    pub what: &'static str,
    pub season: Option<usize>,
    pub near: Option<&'static str>,
}

impl Find {
    /// Something that can turn up anywhere, in any season.
    pub const fn any(what: &'static str) -> Self {
        Self {
            what,
            season: None,
            near: None,
        }
    }

    /// Something that turns up in `season`, by the place named `near`.
    pub const fn at(what: &'static str, season: usize, near: &'static str) -> Self {
        Self {
            what,
            season: Some(season),
            near: Some(near),
        }
    }
}

/// What a Pack gives the morning beyond its lives System's cast.
#[derive(Clone, Copy)]
pub struct Rite {
    pub cast: fn(&WorldState) -> Cast,
    /// What can turn up for the book, in the order it turns up as its
    /// season and place come round. Each turns up once.
    pub finds: &'static [Find],
    /// The season it is, from 0 (spring) to 3 (winter), in a World with
    /// seasons: what is tended and found follows it.
    pub season: fn(&WorldState) -> Option<usize>,
}

/// The season of a World without seasons: none.
pub fn no_seasons(_: &WorldState) -> Option<usize> {
    None
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
            draft.changes.push(StateChange::SetComponent {
                entity: cast.notes,
                key: TENDED_WHAT.into(),
                value: pushed(
                    texts(state, cast.notes, TENDED_WHAT),
                    &name(state, thing).to_lowercase(),
                    KEPT_WHAT,
                ),
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
        if !self.0.finds.iter().any(|find| find.what == what) {
            return Err(ActionError::Invalid("nothing like that to find".into()));
        }
        let found = texts(state, cast.notes, FOUND_THINGS);
        if found.iter().any(|thing| thing == what) {
            return Err(ActionError::Invalid("found before".into()));
        }
        let at = arg_entity(request, "at")
            .filter(|at| state.entity(*at).is_some())
            .ok_or_else(|| ActionError::Invalid("found nowhere".into()))?;
        // Each find is told the next way in turn.
        let line = FIND_LINES[found.len() % FIND_LINES.len()];
        let line = fill(line, &[("what", what), ("place", &name(state, at))]);
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

/// What the player made that still stands, with how it is tended, by the
/// kind of deed that made it: the lines for any season, then this one's.
fn made_things(world: &World, season: Option<usize>) -> Vec<(EntityId, Vec<&'static str>)> {
    let state = world.state();
    let deeds = world.events_of_kind(&["planted_by_hand", "built_by_hand", "decorated_by_hand"]);
    hands::made(state)
        .into_iter()
        .filter_map(|thing| {
            let deed = deeds
                .iter()
                .rev()
                .find(|event| event.targets.first() == Some(&thing))?;
            let (lines, seasonal): (&[&str], &[[&str; 3]; 4]) = match deed.kind.as_str() {
                _ if hands::rising(state, thing).is_some() => (&TENDING_RISING, &[[""; 3]; 4]),
                "planted_by_hand" => (&TENDING_PLANTED, &PLANTED_IN),
                "decorated_by_hand" => (&TENDING_DECORATED, &DECORATED_IN),
                _ => (&TENDING_BUILT, &BUILT_IN),
            };
            let in_season = season
                .and_then(|season| seasonal.get(season))
                .into_iter()
                .flatten()
                .filter(|line| !line.is_empty());
            // The season's own lines first, then the year's.
            Some((thing, in_season.chain(lines).copied().collect()))
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

/// Whether `entity` was seen to by a morning fewer than `rest` World days
/// before `today`.
fn seen_to_lately(state: &WorldState, entity: EntityId, today: u64, rest: i64) -> bool {
    integer(state, entity, TENDED_ON).is_some_and(|on| on + rest > today as i64)
}

/// The first line of `lines`, tried from the `start`th round, and of
/// `people` from `first`, that no morning has told before.
fn untold(
    state: &WorldState,
    told: &[String],
    lines: &[String],
    people: &[EntityId],
    start: usize,
    first: usize,
    words: &[(&str, &str)],
) -> Option<(EntityId, String)> {
    for n in 0..people.len() * lines.len() {
        let who = people[(first + n / lines.len()) % people.len()];
        let line = &lines[(start + n) % lines.len()];
        let mut filled = vec![("who", first_name(state, who))];
        filled.extend(words.iter().map(|(slot, word)| (*slot, word.to_string())));
        let line = filled.iter().fold(line.clone(), |line, (slot, word)| {
            line.replace(&format!("{{{slot}}}"), word)
        });
        if !told.contains(&line) {
            return Some((who, line));
        }
    }
    None
}

/// What a morning tends: a line never told before, by someone of the
/// place, about something the player made if anything of theirs is due a
/// visit, else about one of the places of the town, in the season's way.
/// What was seen to lately rests, and never the same thing (nor another of
/// the same name) within three mornings, so what is looked after moves
/// round the place.
fn tending(world: &World, cast: &Cast, rite: &Rite, today: u64) -> Option<ActionRequest> {
    let state = world.state();
    let told = texts(state, cast.notes, TENDED_LINES);
    let people = tenders(world, cast);
    if people.is_empty() {
        return None;
    }
    let season = (rite.season)(state).filter(|season| *season < 4);
    let lately = texts(state, cast.notes, TENDED_WHAT);
    let seed = mix(&[today, 211]) as usize;
    // Each morning begins on the next of its lines.
    let start = told.len();
    let request = |who: EntityId, thing: EntityId, line: String| {
        ActionRequest::new(TEND_ACTION)
            .actor(who)
            .arg("who", Value::Entity(who))
            .arg("thing", Value::Entity(thing))
            .arg("line", line)
    };
    let mut things = made_things(world, season)
        .into_iter()
        .filter(|(thing, _)| {
            let rest = if hands::rising(state, *thing).is_some() {
                REST_RISING
            } else {
                REST_MADE
            };
            !seen_to_lately(state, *thing, today, rest)
                && !lately.contains(&name(state, *thing).to_lowercase())
        })
        .collect::<Vec<_>>();
    // What is going up is seen to first, then whatever waited longest.
    things.sort_by_key(|(thing, _)| {
        (
            hands::rising(state, *thing).is_none(),
            integer(state, *thing, TENDED_ON).unwrap_or(-1),
            mix(&[thing.0, today, 223]),
        )
    });
    for (thing, lines) in &things {
        let what = name(state, *thing).to_lowercase();
        let place = place_of(state, *thing).unwrap_or(cast.gathering);
        let place = name(state, place);
        // "your green flag by the green" says the place once.
        let lines = lines
            .iter()
            .map(|line| {
                if names_its_place(&what, &place) {
                    placeless(line)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>();
        let words = [("thing", what.as_str()), ("place", place.as_str())];
        if let Some((who, line)) = untold(state, &told, &lines, &people, start, seed, &words) {
            return Some(request(who, *thing, line));
        }
    }
    // Nothing of the player's due a visit: someone looks after one of the
    // places, whichever waited longest, by preference someone who works
    // there.
    let mut places = places(world, cast);
    places.sort_by_key(|place| {
        (
            seen_to_lately(state, *place, today, REST_PLACE)
                || lately.contains(&name(state, *place).to_lowercase()),
            integer(state, *place, TENDED_ON).unwrap_or(-1),
            mix(&[place.0, today, 229]),
        )
    });
    let lines = season
        .map(|season| PLACE_IN[season].as_slice())
        .unwrap_or_default()
        .iter()
        .chain(&TENDING_PLACE)
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    for place in places {
        let mut here = people.clone();
        here.sort_by_key(|person| {
            (
                (cast.work)(state, *person) != Some(place),
                mix(&[person.0, today, 233]),
            )
        });
        let named = name(state, place);
        let words = [("place", named.as_str())];
        if let Some((who, line)) = untold(state, &told, &lines, &here, start, 0, &words) {
            return Some(request(who, place, line));
        }
    }
    None
}

/// What turns up for the book this morning: of the Pack's finds not yet
/// found, the first of this season by today's place, else by any place of
/// the town; else the first that belongs to no season; else one of
/// another season, so the book never stalls while anything is left.
fn find(world: &World, cast: &Cast, rite: &Rite, today: u64) -> Option<ActionRequest> {
    let state = world.state();
    let found = texts(state, cast.notes, FOUND_THINGS);
    let places = places(world, cast);
    let today_at = *places.get((mix(&[today, 227]) % places.len().max(1) as u64) as usize)?;
    let season = (rite.season)(state);
    let by_name = |near: &str| {
        places
            .iter()
            .copied()
            .find(|place| name(state, *place) == near)
    };
    let unfound = rite
        .finds
        .iter()
        .filter(|find| !found.iter().any(|thing| thing == find.what))
        .collect::<Vec<_>>();
    let in_season = |find: &&&Find| find.season.is_some() && find.season == season;
    let any_season = |find: &&&Find| find.season.is_none();
    let here = |find: &&&Find| find.near.is_none_or(|near| by_name(near) == Some(today_at));
    let anywhere = |find: &&&Find| find.near.is_none_or(|near| by_name(near).is_some());
    let find = unfound
        .iter()
        .filter(in_season)
        .find(here)
        .or_else(|| unfound.iter().filter(in_season).find(anywhere))
        .or_else(|| unfound.iter().filter(any_season).find(anywhere))
        .or_else(|| unfound.iter().find(anywhere))?;
    let at = find.near.and_then(by_name).unwrap_or(today_at);
    Some(
        ActionRequest::new(FIND_ACTION)
            .arg("what", find.what)
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
            tending(world, &cast, rite, today),
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
            .chain(&TENDING_RISING)
            .chain(PLANTED_IN.iter().flatten())
            .chain(BUILT_IN.iter().flatten())
            .chain(DECORATED_IN.iter().flatten())
    };
    made()
        .map(|line| line.to_string())
        .chain(made().map(|line| placeless(line)))
        .chain(TENDING_PLACE.iter().map(|line| line.to_string()))
        .chain(PLACE_IN.iter().flatten().map(|line| line.to_string()))
        .chain(FIND_LINES.iter().map(|line| line.to_string()))
        .chain([FINDS_SHELF.to_string()])
        .collect()
}
