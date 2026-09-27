//! A year with a shape.
//!
//! A World Pack gives this System an [`Almanac`]: how long its year is,
//! what its seasons are called, and the festivals and seasonal moments that
//! fall on the same day every year. Days before a festival people start
//! getting ready; on the day it is held, and how it goes depends on the
//! place as it stands: its spirits, what the player has put up, what was
//! planted and grown, for a harvest. Each of these is an Action recorded as
//! an Event like any other, so a World replays without asking anything
//! again.

use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, Entity, EntityId, Event, EventDraft,
    EventId, StateChange, Value, World, WorldError, WorldState,
};

/// Something that falls on the same day every year.
#[derive(Clone, Debug)]
pub struct Festival {
    pub id: &'static str,
    pub name: &'static str,
    /// Which day of the year it falls on, counting from 0.
    pub day: u64,
    /// How many days before it people start getting ready; 0 for
    /// something that simply arrives, like the first frost.
    pub prepare: u64,
    /// Where it is held.
    pub at: EntityId,
    /// Who speaks for it, while they are there.
    pub speaker: EntityId,
    /// What goes up for the day: a fixture shape the scene draws, or
    /// nothing for something that simply arrives.
    pub shape: &'static str,
    /// Whether it gathers in what was planted.
    pub harvest: bool,
    /// Told when getting ready starts: "{name} is {days} days away."
    pub nears: &'static str,
    /// Said by whoever speaks for it while getting ready.
    pub getting_ready: &'static str,
    /// Told on the day, for a grand, a fine and a thin turnout.
    pub told: [&'static str; 3],
    /// Said on the day, for each turnout.
    pub said: [&'static str; 3],
}

/// How a festival went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turnout {
    Grand,
    Fine,
    Thin,
}

impl Turnout {
    pub fn id(self) -> &'static str {
        match self {
            Turnout::Grand => "grand",
            Turnout::Fine => "fine",
            Turnout::Thin => "thin",
        }
    }

    fn index(self) -> usize {
        match self {
            Turnout::Grand => 0,
            Turnout::Fine => 1,
            Turnout::Thin => 2,
        }
    }

    /// How a festival goes in a place this ready to celebrate.
    pub fn of(festive: i64) -> Self {
        if festive >= 6 {
            Turnout::Grand
        } else if festive >= 2 {
            Turnout::Fine
        } else {
            Turnout::Thin
        }
    }
}

/// Everything the System needs to know about a World's year.
#[derive(Clone, Debug)]
pub struct Almanac {
    /// The entity the System keeps its notes on.
    pub notes: EntityId,
    /// The first entity id what goes up for a festival can take.
    pub first: u64,
    /// How many ids it may use, in turn.
    pub room: u64,
    /// How much world time one period, a day, is.
    pub period: u64,
    /// How many periods make a year.
    pub year: u64,
    pub seasons: [&'static str; 4],
    pub festivals: &'static [Festival],
    /// Who lives there now.
    pub people: fn(&WorldState) -> Vec<EntityId>,
    /// How ready the place is to celebrate: 6 or more makes a grand day,
    /// 2 or more a fine one.
    pub festive: fn(&WorldState) -> i64,
    /// How much was planted and grown, for a harvest.
    pub grown: fn(&WorldState) -> i64,
    /// What a festival does in the Pack's own terms, besides bringing
    /// people together: given how it went and what was grown.
    pub held: fn(&WorldState, &Festival, Turnout, i64) -> Vec<StateChange>,
}

const NEXT: &str = "calendar.next";

fn neared_key(festival: &str) -> String {
    format!("calendar.neared.{festival}")
}

fn held_key(festival: &str) -> String {
    format!("calendar.held.{festival}")
}

fn turnout_key(festival: &str) -> String {
    format!("calendar.turnout.{festival}")
}

fn people_key(festival: &str) -> String {
    format!("calendar.people.{festival}")
}

/// What is said of a festival against the last time it was held: bigger,
/// quieter or the same, and how many new faces came since.
fn against_last_year(
    turnout: Turnout,
    last: Option<i64>,
    people: i64,
    last_people: Option<i64>,
) -> (String, String) {
    let Some(last) = last else {
        return (String::new(), String::new());
    };
    let now = turnout.index() as i64;
    let (told, said) = match now.cmp(&last) {
        std::cmp::Ordering::Less => (", bigger than last year", "Better than last year!"),
        std::cmp::Ordering::Greater => (", nothing like last year", "Not like last year."),
        std::cmp::Ordering::Equal => match turnout {
            Turnout::Grand => (", as good as last year", "Every bit as good as last year."),
            Turnout::Fine => (", much as it was last year", "Just like last year."),
            Turnout::Thin => (", thin again this year", "Thin again. Like last year."),
        },
    };
    let (mut told, mut said) = (told.to_string(), said.to_string());
    match people - last_people.unwrap_or(people) {
        ..=0 => {}
        1 => {
            told.push_str(", with a new face among them");
            said.push_str(" Good to see a new face.");
        }
        new => {
            told.push_str(&format!(", with {new} new faces among them"));
            said.push_str(" Good to see new faces.");
        }
    }
    (told, said)
}

fn integer(state: &WorldState, entity: EntityId, key: &str) -> Option<i64> {
    match state.entity(entity)?.component(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

/// Which period this is.
pub fn period(state: &WorldState, almanac: &Almanac) -> u64 {
    state.world_time() / almanac.period.max(1)
}

/// Which day of the year this is, counting from 0.
pub fn day_of_year(state: &WorldState, almanac: &Almanac) -> u64 {
    period(state, almanac) % almanac.year.max(1)
}

/// Which season it is, from 0 (spring) to 3.
pub fn season(state: &WorldState, almanac: &Almanac) -> usize {
    let quarter = (almanac.year / 4).max(1);
    ((day_of_year(state, almanac) / quarter).min(3)) as usize
}

/// What the season is called.
pub fn season_name(state: &WorldState, almanac: &Almanac) -> &'static str {
    almanac.seasons[season(state, almanac)]
}

/// The next festival and how many days away it is: 0 for today.
pub fn upcoming(state: &WorldState, almanac: &Almanac) -> Option<(&'static Festival, u64)> {
    let today = day_of_year(state, almanac);
    let year = almanac.year.max(1);
    almanac
        .festivals
        .iter()
        .map(|festival| (festival, (festival.day % year + year - today) % year))
        .min_by_key(|(_, days)| *days)
}

/// Whether getting ready for a festival starts today.
fn nears_today(state: &WorldState, almanac: &Almanac, festival: &Festival) -> bool {
    let year = almanac.year.max(1);
    festival.prepare > 0
        && festival.prepare < year
        && (period(state, almanac) + festival.prepare) % year == festival.day % year
}

fn held_today(state: &WorldState, almanac: &Almanac, festival: &Festival) -> bool {
    day_of_year(state, almanac) == festival.day % almanac.year.max(1)
}

fn fill(template: &str, festival: &Festival, days: u64) -> String {
    template
        .replace("{name}", festival.name)
        .replace("{days}", &days.to_string())
}

fn speaker(state: &WorldState, almanac: &Almanac, festival: &Festival) -> Option<EntityId> {
    let people = (almanac.people)(state);
    people
        .contains(&festival.speaker)
        .then_some(festival.speaker)
        .or_else(|| people.first().copied())
}

/// Lets each festival's day come round: getting ready starts, and the day
/// itself is held. Neither waits for the player.
pub fn tick(
    world: &mut World,
    actions: &ActionRegistry,
    almanac: &Almanac,
) -> Result<Vec<EventId>, WorldError> {
    let mut events = Vec::new();
    for festival in almanac.festivals {
        let state = world.state();
        let now = period(state, almanac) as i64;
        if nears_today(state, almanac, festival)
            && integer(state, almanac.notes, &neared_key(festival.id))
                != Some(now + festival.prepare as i64)
        {
            let request = ActionRequest::new("calendar_nears").arg("festival", festival.id);
            events.push(world.execute(actions, &request)?.id);
        }
        let state = world.state();
        if held_today(state, almanac, festival)
            && integer(state, almanac.notes, &held_key(festival.id)) != Some(now)
        {
            let request = ActionRequest::new("calendar_holds").arg("festival", festival.id);
            events.push(world.execute(actions, &request)?.id);
        }
    }
    Ok(events)
}

pub fn register_actions(
    actions: &mut ActionRegistry,
    almanac: fn(&WorldState) -> Almanac,
) -> Result<(), ActionError> {
    actions.register(Nears(almanac))?;
    actions.register(Holds(almanac))
}

fn festival_of<'a>(
    almanac: &'a Almanac,
    request: &ActionRequest,
) -> Result<&'a Festival, ActionError> {
    let id = match request.args.get("festival") {
        Some(Value::Text(id)) => id.as_str(),
        _ => return Err(ActionError::Invalid("which festival?".into())),
    };
    almanac
        .festivals
        .iter()
        .find(|festival| festival.id == id)
        .ok_or_else(|| ActionError::Invalid(format!("no festival {id}")))
}

fn notes_exist(state: &WorldState, almanac: &Almanac, changes: &mut Vec<StateChange>) {
    if state.entity(almanac.notes).is_none() {
        changes.push(StateChange::CreateEntity(
            Entity::new(almanac.notes, "calendar").with_component("name", "Calendar"),
        ));
    }
}

/// Getting ready for a festival starts.
struct Nears(fn(&WorldState) -> Almanac);

impl Action for Nears {
    fn name(&self) -> &'static str {
        "calendar_nears"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let almanac = (self.0)(state);
        let festival = festival_of(&almanac, request)?;
        if !nears_today(state, &almanac, festival) {
            return Err(ActionError::Invalid("it isn't time to get ready".into()));
        }
        let occurrence = period(state, &almanac) as i64 + festival.prepare as i64;
        if integer(state, almanac.notes, &neared_key(festival.id)) == Some(occurrence) {
            return Err(ActionError::Invalid("they're already getting ready".into()));
        }
        let mut changes = Vec::new();
        notes_exist(state, &almanac, &mut changes);
        changes.push(StateChange::SetComponent {
            entity: almanac.notes,
            key: neared_key(festival.id),
            value: occurrence.into(),
        });
        let speaker = speaker(state, &almanac, festival);
        if let Some(who) = speaker.filter(|who| lives::enrolled(state, *who)) {
            changes.push(lives::lack_by(state, who, lives::Need::Purpose, -5));
        }
        let mut draft = EventDraft::new("festival_nears");
        draft.actor = speaker;
        draft.targets = vec![festival.at];
        draft.payload.insert("festival".into(), festival.id.into());
        draft.payload.insert("name".into(), festival.name.into());
        draft
            .payload
            .insert("in".into(), (festival.prepare as i64).into());
        draft.payload.insert(
            "told".into(),
            fill(festival.nears, festival, festival.prepare).into(),
        );
        draft.payload.insert(
            "said".into(),
            fill(festival.getting_ready, festival, festival.prepare).into(),
        );
        draft.changes = changes;
        Ok(draft)
    }
}

/// A festival's day: it is held, and goes as well as the place is ready.
struct Holds(fn(&WorldState) -> Almanac);

impl Action for Holds {
    fn name(&self) -> &'static str {
        "calendar_holds"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let almanac = (self.0)(state);
        let festival = festival_of(&almanac, request)?;
        if !held_today(state, &almanac, festival) {
            return Err(ActionError::Invalid("it isn't the day".into()));
        }
        let now = period(state, &almanac) as i64;
        if integer(state, almanac.notes, &held_key(festival.id)) == Some(now) {
            return Err(ActionError::Invalid("it has been held today".into()));
        }
        let grown = if festival.harvest {
            (almanac.grown)(state).max(0)
        } else {
            0
        };
        let festive = (almanac.festive)(state) + grown;
        let turnout = Turnout::of(festive);
        let mut changes = Vec::new();
        notes_exist(state, &almanac, &mut changes);
        changes.push(StateChange::SetComponent {
            entity: almanac.notes,
            key: held_key(festival.id),
            value: now.into(),
        });

        // What goes up for the day, in the next free place of its own.
        let next = integer(state, almanac.notes, NEXT).unwrap_or(0).max(0) as u64;
        let room = almanac.room.max(1);
        let free = (0..room)
            .map(|step| (next + step) % room)
            .find(|slot| state.entity(EntityId::new(almanac.first + slot)).is_none());
        // Pointed at where it is held, not at what goes up: once that comes
        // down there is nothing left to point at.
        let targets = vec![festival.at];
        if let Some(slot) = free
            .filter(|_| !festival.shape.is_empty())
            .filter(|_| state.entity(festival.at).is_some())
        {
            let id = EntityId::new(almanac.first + slot);
            changes.push(StateChange::CreateEntity(
                Entity::new(id, "fixture")
                    .with_component("name", festival.name)
                    .with_component("shape", festival.shape)
                    .with_component("at", Value::Entity(festival.at))
                    .with_component("until", now + 1),
            ));
            changes.push(StateChange::SetComponent {
                entity: almanac.notes,
                key: NEXT.into(),
                value: (((slot + 1) % room) as i64).into(),
            });
        }

        // A festival brings people together.
        let together = match turnout {
            Turnout::Grand => -15,
            Turnout::Fine => -10,
            Turnout::Thin => -3,
        };
        for who in (almanac.people)(state) {
            if lives::enrolled(state, who) {
                changes.push(lives::lack_by(state, who, lives::Need::Company, together));
                if turnout == Turnout::Grand {
                    changes.push(lives::regard_by(state, who, 2));
                }
            }
        }
        changes.extend(mixing(state, &(almanac.people)(state), turnout));
        changes.extend((almanac.held)(state, festival, turnout, grown));

        // Remembered for next year, and told against last year's.
        let people = (almanac.people)(state).len() as i64;
        let (then_told, then_said) = against_last_year(
            turnout,
            integer(state, almanac.notes, &turnout_key(festival.id)),
            people,
            integer(state, almanac.notes, &people_key(festival.id)),
        );
        changes.push(StateChange::SetComponent {
            entity: almanac.notes,
            key: turnout_key(festival.id),
            value: (turnout.index() as i64).into(),
        });
        changes.push(StateChange::SetComponent {
            entity: almanac.notes,
            key: people_key(festival.id),
            value: people.into(),
        });

        let mut draft = EventDraft::new("festival_held");
        draft.actor = speaker(state, &almanac, festival);
        draft.targets = targets;
        draft.payload.insert("festival".into(), festival.id.into());
        draft.payload.insert("name".into(), festival.name.into());
        draft.payload.insert("turnout".into(), turnout.id().into());
        draft.payload.insert("grown".into(), grown.into());
        // Whatever the player made there is part of the day.
        let around = hands::made_at(state, festival.at)
            .map(|made| format!(", around the {} you made", made.to_lowercase()))
            .unwrap_or_default();
        draft.payload.insert(
            "told".into(),
            format!(
                "{}{then_told}{around}",
                fill(festival.told[turnout.index()], festival, 0)
            )
            .into(),
        );
        let said = fill(festival.said[turnout.index()], festival, 0);
        draft.payload.insert(
            "said".into(),
            if then_said.is_empty() {
                said
            } else {
                format!("{said} {then_said}")
            }
            .into(),
        );
        draft.changes = changes;
        Ok(draft)
    }
}

/// People mix at a festival. At a good one the two who get on worst soften
/// toward each other, and at a grand one the two closest to being friends
/// get there; at a thin one old grudges fester.
fn mixing(state: &WorldState, people: &[EntityId], turnout: Turnout) -> Vec<StateChange> {
    let enrolled = people
        .iter()
        .copied()
        .filter(|who| lives::enrolled(state, *who))
        .collect::<Vec<_>>();
    let pairs = enrolled
        .iter()
        .enumerate()
        .flat_map(|(index, a)| enrolled[index + 1..].iter().map(move |b| (*a, *b)))
        .map(|(a, b)| {
            (
                a,
                b,
                lives::opinion(state, a, b) + lives::opinion(state, b, a),
            )
        })
        .collect::<Vec<_>>();
    let coldest = pairs.iter().min_by_key(|(_, _, both)| *both).copied();
    let warmest = pairs
        .iter()
        .filter(|(a, b, _)| {
            lives::opinion(state, *a, *b) < 30 || lives::opinion(state, *b, *a) < 30
        })
        .max_by_key(|(_, _, both)| *both)
        .copied();
    let nudge = |(a, b, _): (EntityId, EntityId, i64), by: i64| {
        lives::opinion_by(state, a, b, by)
            .into_iter()
            .chain(lives::opinion_by(state, b, a, by))
            .collect::<Vec<_>>()
    };
    let mut changes = Vec::new();
    match turnout {
        Turnout::Grand => {
            changes.extend(coldest.map(|pair| nudge(pair, 8)).unwrap_or_default());
            if warmest.map(|(a, b, _)| (a, b)) != coldest.map(|(a, b, _)| (a, b)) {
                changes.extend(warmest.map(|pair| nudge(pair, 8)).unwrap_or_default());
            }
        }
        Turnout::Fine => changes.extend(coldest.map(|pair| nudge(pair, 6)).unwrap_or_default()),
        Turnout::Thin => changes.extend(coldest.map(|pair| nudge(pair, -6)).unwrap_or_default()),
    }
    changes
}

/// Whether an Event is one of the year's days.
pub fn is_calendar(event: &Event) -> bool {
    matches!(event.kind.as_str(), "festival_nears" | "festival_held")
}

/// How one of the year's days is told.
pub fn told(event: &Event) -> Option<String> {
    if !is_calendar(event) {
        return None;
    }
    match event.payload.get("told") {
        Some(Value::Text(told)) => Some(told.clone()),
        _ => None,
    }
}

/// Who speaks at one of the year's days, and what they say.
pub fn said(event: &Event) -> Option<(EntityId, String)> {
    if !is_calendar(event) {
        return None;
    }
    match event.payload.get("said") {
        Some(Value::Text(said)) if !said.is_empty() => Some((event.actor?, said.clone())),
        _ => None,
    }
}

/// How a festival that was held went, if the Event is one.
pub fn turnout(event: &Event) -> Option<Turnout> {
    if event.kind != "festival_held" {
        return None;
    }
    match event.payload.get("turnout") {
        Some(Value::Text(turnout)) => match turnout.as_str() {
            "grand" => Some(Turnout::Grand),
            "fine" => Some(Turnout::Fine),
            _ => Some(Turnout::Thin),
        },
        _ => None,
    }
}

/// What is coming up, in a few words: "Midsummer Fair in 3 days",
/// "Midsummer Fair today". Nothing if it is more than `within` days away.
pub fn coming_up(state: &WorldState, almanac: &Almanac, within: u64) -> Option<String> {
    let (festival, days) = upcoming(state, almanac)?;
    (days <= within).then(|| match days {
        0 => format!("{} today", festival.name),
        1 => format!("{} tomorrow", festival.name),
        days => format!("{} in {days} days", festival.name),
    })
}

#[cfg(test)]
mod tests;
