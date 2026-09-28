//! What changes as the harbour's years turn, so the second year is not the
//! first again: Mia finishes school, Leo hands the pub to Sofia,
//! newcomers settle for good, and each year adds one festival, chosen by
//! what the harbour did the year before.

use crate::almanac::YEAR_DAYS;
use crate::story::STORY;
use crate::{EVAN, HARBOR, JONAS, LEO, MARA, MIA, PUB, SCHOOL, SOFIA};
use calendar::Festival;
use society_basic::JOB;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, EventDraft, EventId, StateChange,
    Value, World, WorldError, WorldState,
};

/// The festivals a year can add, one a year, in the order a tie breaks.
pub(crate) const NEW_FESTIVALS: &[Festival] = &[
    Festival {
        id: "builders_day",
        name: "Builders' Day",
        day: 33,
        prepare: 3,
        at: HARBOR,
        speaker: EVAN,
        shape: "flag",
        harvest: false,
        nears: "Builders' Day is {days} days off. Everyone's polishing what they made.",
        getting_ready: "Builders' Day in {days} days. I've sanded every bench twice.",
        told: [
            "The whole harbour walked round everything built this year on Builders' Day",
            "The harbour kept Builders' Day",
            "Builders' Day came, and only Evan went round the works",
        ],
        said: [
            "Look what we made, all of us!",
            "Not bad for a small harbour.",
            "Just me and my hammer, then.",
        ],
    },
    Festival {
        id: "garden_fair",
        name: "the Garden Fair",
        day: 58,
        prepare: 4,
        at: SCHOOL,
        speaker: MARA,
        shape: "tent",
        harvest: true,
        nears: "The Garden Fair is {days} days away. Everyone's watering twice a day.",
        getting_ready: "Garden Fair in {days} days. My beans had better behave.",
        told: [
            "Everyone brought the best of their gardens to the Garden Fair",
            "The harbour held its Garden Fair",
            "The Garden Fair was three marrows and a lettuce",
        ],
        said: [
            "Biggest marrow I ever saw!",
            "A fair showing, for a windy place.",
            "Next year, then.",
        ],
    },
    Festival {
        id: "neighbours_supper",
        name: "the Neighbours' Supper",
        day: 90,
        prepare: 3,
        at: PUB,
        speaker: LEO,
        shape: "lantern",
        harvest: false,
        nears: "The Neighbours' Supper is {days} days away. Bring a dish, bring a friend.",
        getting_ready:
            "Neighbours' Supper in {days} days. I've borrowed every chair on the island.",
        told: [
            "The whole harbour ate together at the Neighbours' Supper",
            "The harbour had its Neighbours' Supper",
            "The Neighbours' Supper was a few of us and a lot of empty chairs",
        ],
        said: [
            "Never seen so many friends round one table.",
            "Good food, good people.",
            "More pie for us, I suppose.",
        ],
    },
    Festival {
        id: "regatta",
        name: "the Harbour Regatta",
        day: 111,
        prepare: 3,
        at: HARBOR,
        speaker: JONAS,
        shape: "flag",
        harvest: false,
        nears: "The Harbour Regatta is {days} days away. Oars are being varnished.",
        getting_ready: "Regatta in {days} days. I'll beat Evan this time.",
        told: [
            "Every boat in the harbour raced in the Regatta",
            "The harbour held its Regatta",
            "The Regatta was two boats and a lot of rain",
        ],
        said: [
            "Round the buoy and home first!",
            "A good race, that.",
            "Well, somebody had to win.",
        ],
    },
];

const FESTIVALS_ADDED: &str = "years.festivals";
const TURNED: &str = "years.turned";

/// Which year of the harbour it is, counting the first as 1.
pub(crate) fn year(state: &WorldState) -> u64 {
    state.world_time() / crate::persistence::WORLD_DAY_TICKS / YEAR_DAYS + 1
}

/// The festivals added so far, in the order they were added.
pub(crate) fn added(state: &WorldState) -> Vec<&'static Festival> {
    let ids = match state
        .entity(STORY)
        .and_then(|story| story.component(FESTIVALS_ADDED))
    {
        Some(Value::List(ids)) => ids
            .iter()
            .filter_map(|id| match id {
                Value::Text(id) => Some(id.clone()),
                _ => None,
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    NEW_FESTIVALS
        .iter()
        .filter(|festival| ids.iter().any(|id| id == festival.id))
        .collect()
}

/// Every festival of the harbour's year: the old ones and those added.
pub(crate) fn festivals(state: &WorldState, old: &'static [Festival]) -> &'static [Festival] {
    let added = added(state);
    if added.is_empty() {
        return old;
    }
    let mask = NEW_FESTIVALS
        .iter()
        .enumerate()
        .filter(|(_, festival)| added.iter().any(|added| added.id == festival.id))
        .fold(0_u8, |mask, (at, _)| mask | 1 << at);
    // At most sixteen different years, each made once.
    static YEARS: OnceLock<Mutex<BTreeMap<u8, &'static [Festival]>>> = OnceLock::new();
    let mut years = YEARS
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    years.entry(mask).or_insert_with(|| {
        let mut all = old.to_vec();
        all.extend(added.into_iter().cloned());
        all.sort_by_key(|festival| festival.day);
        Box::leak(all.into_boxed_slice())
    })
}

/// How much the year gone asked for each new festival: works finished for
/// Builders' Day, gardens grown for the Garden Fair, friends made for the
/// Neighbours' Supper; the Regatta is there for a quiet year.
fn scores(state: &WorldState) -> [i64; 4] {
    let deck = crate::story::deck();
    let works = crate::story::WORKS
        .iter()
        .filter(|work| storylets::finished(state, &deck, work.id))
        .count() as i64;
    let friends = crate::story::people_in(state)
        .into_iter()
        .filter(|person| lives::regard(state, *person) >= lives::FOND)
        .count() as i64;
    [works * 2, crate::almanac::grown(state) * 3, friends, 1]
}

/// A change that comes with a year: what it is, who tells it, and what it
/// changes.
struct Turning {
    who: EntityId,
    told: String,
    said: String,
    changes: Vec<StateChange>,
}

fn set(entity: EntityId, key: &str, value: impl Into<Value>) -> StateChange {
    StateChange::SetComponent {
        entity,
        key: key.into(),
        value: value.into(),
    }
}

fn job(state: &WorldState, person: EntityId) -> Option<&str> {
    match state.entity(person)?.component(JOB)? {
        Value::Text(job) => Some(job),
        _ => None,
    }
}

/// What turns with this year, if anything is still to turn.
fn turning(state: &WorldState, beat: &str) -> Option<Turning> {
    let year = year(state);
    let done = |key: &str| {
        matches!(
            state
                .entity(STORY)
                .and_then(|story| story.component(&format!("{TURNED}.{key}"))),
            Some(Value::Bool(true))
        )
    };
    if done(beat) {
        return None;
    }
    let mut turning = match beat {
        "grown" if year >= 2 && job(state, MIA) == Some("student") => Turning {
            who: MIA,
            told: "Mia finished school and started with Evan".into(),
            said: "No more homework! Evan says I've good hands.".into(),
            changes: vec![
                set(MIA, JOB, "apprentice"),
                set(MIA, "location", Value::Entity(HARBOR)),
            ],
        },
        "hands" if year >= 3 && job(state, LEO) == Some("pub_owner") => Turning {
            who: LEO,
            told: "Leo handed the Anchor Pub to Sofia".into(),
            said: "She'll run it better than I did. Don't tell her I said so.".into(),
            changes: vec![
                set(LEO, JOB, "retired"),
                set(SOFIA, JOB, "pub_owner"),
                set(SOFIA, "location", Value::Entity(PUB)),
            ],
        },
        "settled" if year >= 4 => {
            let cast = crate::life::cast();
            let newcomers = lives::arrivals(state, &cast)
                .into_iter()
                .filter(|person| !lives::settled(state, *person))
                .collect::<Vec<_>>();
            let first = *newcomers.first()?;
            let names = newcomers
                .iter()
                .map(|person| lives::first_name(state, *person))
                .collect::<Vec<_>>();
            let told = match names.as_slice() {
                [one] => format!("{one} settled in the harbour for good"),
                [rest @ .., last] => {
                    format!(
                        "{} and {last} settled in the harbour for good",
                        rest.join(", ")
                    )
                }
                [] => return None,
            };
            Turning {
                who: first,
                told,
                said: "I think I'll stay. This is home now.".into(),
                changes: newcomers
                    .iter()
                    .map(|person| set(*person, lives::SETTLED, true))
                    .collect(),
            }
        }
        _ if beat.starts_with("festival_") => {
            let for_year = beat.trim_start_matches("festival_").parse::<u64>().ok()?;
            if for_year < 2 || year < for_year {
                return None;
            }
            let added = added(state);
            let scores = scores(state);
            let (_, festival) = NEW_FESTIVALS
                .iter()
                .enumerate()
                .filter(|(_, festival)| !added.iter().any(|added| added.id == festival.id))
                .max_by_key(|(at, _)| (scores[*at], -(*at as i64)))?;
            let mut ids = added
                .iter()
                .map(|festival| Value::Text(festival.id.into()))
                .collect::<Vec<_>>();
            ids.push(Value::Text(festival.id.into()));
            Turning {
                who: festival.speaker,
                told: format!(
                    "The harbour chose a new day for its year: {}",
                    festival.name
                ),
                said: format!(
                    "After the year we've had, we ought to have {}.",
                    festival.name
                ),
                changes: vec![set(STORY, FESTIVALS_ADDED, Value::List(ids))],
            }
        }
        _ => return None,
    };
    turning
        .changes
        .push(set(STORY, &format!("{TURNED}.{beat}"), true));
    Some(turning)
}

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(YearTurns)
}

/// Something changes for good as a year turns.
struct YearTurns;

impl Action for YearTurns {
    fn name(&self) -> &'static str {
        "harbour_year_turns"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let beat = match request.args.get("beat") {
            Some(Value::Text(beat)) => beat.as_str(),
            _ => return Err(ActionError::Invalid("which change?".into())),
        };
        let turning = turning(state, beat)
            .ok_or_else(|| ActionError::Invalid(format!("{beat} is not due")))?;
        if state.entity(turning.who).is_none() {
            return Err(ActionError::Invalid("nobody to tell it".into()));
        }
        let mut draft = EventDraft::new("year_turned");
        draft.actor = Some(turning.who);
        draft.targets = vec![turning.who];
        draft.payload.insert("beat".into(), beat.into());
        draft.payload.insert("told".into(), turning.told.into());
        draft.payload.insert("said".into(), turning.said.into());
        draft.changes = turning.changes;
        Ok(draft)
    }
}

/// Whatever the year has turned to that has not yet happened, one change a
/// day, and a memory of a year ago.
pub(crate) fn tick(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    let mut events = Vec::new();
    let year = year(world.state());
    let beats = ["grown", "hands", "settled"]
        .into_iter()
        .map(String::from)
        .chain((2..=year).map(|year| format!("festival_{year}")));
    for beat in beats {
        if turning(world.state(), &beat).is_some() {
            let request = ActionRequest::new("harbour_year_turns").arg("beat", beat.as_str());
            if let Ok(event) = world.execute(actions, &request) {
                events.push(event.id);
                break;
            }
        }
    }
    let cast = crate::life::cast();
    events.extend(lives::season_turns(
        world,
        actions,
        &cast,
        crate::story::SEASON_DAYS,
    )?);
    events.extend(lives::remember_a_year(world, actions, &cast, YEAR_DAYS)?);
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TinySociety;
    /// The second year is not the first: Mia grows up, the pub changes
    /// hands, and a festival the first year never had is held.
    #[test]
    fn the_second_year_is_not_the_first() {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        for _ in 0..(YEAR_DAYS * 3 + 2) {
            branch
                .invoke_projection_command(crate::story::WAIT_COMMAND)
                .unwrap();
        }
        let state = branch.world().state();
        assert_eq!(job(state, MIA), Some("apprentice"));
        assert_eq!(job(state, SOFIA), Some("pub_owner"));
        assert_eq!(job(state, LEO), Some("retired"));
        let added = added(state);
        assert_eq!(
            added.len() as u64,
            year(state) - 1,
            "one new festival a year after the first"
        );
        let new_held = branch
            .world()
            .events()
            .iter()
            .filter(|event| calendar::is_calendar(event))
            .filter_map(|event| match event.payload.get("festival") {
                Some(Value::Text(id)) => Some(id.clone()),
                _ => None,
            })
            .any(|id| added.iter().any(|festival| festival.id == id));
        assert!(new_held, "a new festival was held");
        assert!(branch
            .world()
            .events()
            .iter()
            .any(|event| event.kind == "year_remembered"));
    }
}
