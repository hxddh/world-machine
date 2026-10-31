//! The player's part in the harbour: its new keeper of the Book. The Book
//! has a page for each season's beat of the year (spring arrival, the
//! summer fair, the autumn gale, the winter lights), each filled by the
//! day itself as an Event caused by it, and an end-of-year chapter with
//! credits: everyone the year's Events name and what each did, the
//! player's deeds last. Play goes on into the next year, and its pages.
//!
//! What the harbour works toward over the years is counted here too, from
//! recorded Events alone: its projects (the lighthouse and the school
//! roof, each a few steps, funded by the player's deeds), the book of
//! finds and the almanac to complete, and each resident's heart.

use crate::almanac::YEAR_DAYS;
use crate::persistence::WORLD_DAY_TICKS;
use crate::{EMMA, EVAN, HARBOR, JONAS, LEO, MARA, MIA, NOAH, SOFIA};
use std::collections::{BTreeMap, BTreeSet};
use world_core::{
    Action, ActionError, ActionRegistry, ActionRequest, EntityId, Event, EventDraft, EventId,
    StateChange, Value, World, WorldError, WorldState,
};
use world_projection::{Aim, BookEntry, Credit, Keeping, Page, SelectionId, YearClosed};

/// What Leo says as he welcomes a new player: who they are here.
pub(crate) const HELLO: &str =
    "Welcome to the {settlement}! I'm {name}. You must be our new keeper of the Book.";
/// The Book's title page.
const TITLE: &str = "The Harbour Book";
const ROLE_NEW: &str = "You are the harbour's new keeper of the Book.";
const ROLE: &str = "You keep the harbour's Book, a page a season.";

/// The kind of Event a page of the year is filled with.
pub(crate) const PAGE_FILLED: &str = "page_filled";
/// The kind of Event a year closes with.
pub(crate) const YEAR_CLOSED: &str = "year_closed";
/// On the harbour: the day a page was last filled.
const FILLED: &str = "book.filled";
/// On the harbour: the last year closed.
const CLOSED: &str = "book.closed";

/// One season's page of the year: the day that fills it, and what the
/// Book calls it.
pub(crate) struct Beat {
    pub id: &'static str,
    pub festival: &'static str,
    title: &'static str,
    /// Its title in the player's first year.
    first: &'static str,
    to_come: &'static str,
}

/// The year's arc, in the order it fills.
pub(crate) const ARC: [Beat; 4] = [
    Beat {
        id: "spring",
        festival: "spring_clean",
        title: "Spring: the harbour wakes",
        first: "Spring: a new keeper arrives",
        to_come: "Spring Clean will fill this page",
    },
    Beat {
        id: "summer",
        festival: "midsummer_fair",
        title: "Summer: the Midsummer Fair",
        first: "Summer: the Midsummer Fair",
        to_come: "The Midsummer Fair will fill this page",
    },
    Beat {
        id: "autumn",
        festival: "autumn_gale",
        title: "Autumn: the gale",
        first: "Autumn: the gale",
        to_come: "The autumn gale will fill this page",
    },
    Beat {
        id: "winter",
        festival: "winter_lights",
        title: "Winter: the Winter Lights",
        first: "Winter: the Winter Lights",
        to_come: "The Winter Lights will fill this page",
    },
];

/// A project the harbour works toward, a step at a time: each step is
/// funded once the player's deeds of its kind reach its count.
pub(crate) struct Project {
    pub id: &'static str,
    label: &'static str,
    /// What funds it: things made by hand, or help given.
    funded_by: Deeds,
    steps: &'static [(u32, &'static str)],
}

impl Project {
    /// How many steps it takes.
    pub(crate) fn steps(&self) -> u32 {
        self.steps.len() as u32
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Deeds {
    /// Things the player made, put up or planted with their own hands.
    Made,
    /// Questions the player answered and favours they did.
    Helped,
}

pub(crate) const PROJECTS: [Project; 2] = [
    Project {
        id: "lighthouse",
        label: "The lighthouse",
        funded_by: Deeds::Made,
        steps: &[
            (3, "Clear the path to the point"),
            (10, "Scrape off the old paint"),
            (20, "A fresh coat of white"),
            (35, "New glass for the lamp"),
            (55, "The lighthouse lit again"),
        ],
    },
    Project {
        id: "school_roof",
        label: "The school roof",
        funded_by: Deeds::Helped,
        steps: &[
            (8, "Slates gathered from the beach"),
            (25, "The old slates off"),
            (50, "New slates on"),
            (80, "The school roof mended"),
        ],
    },
];

/// The beats of each resident's heart, in order: `{name}` is theirs.
const HEART: [&str; 5] = [
    "Met {name}",
    "A favour done for {name}",
    "A letter from {name}",
    "A keepsake from {name}",
    "A wish of {name}'s granted",
];

/// The residents whose hearts the Book follows.
pub(crate) const HEARTS: [EntityId; 8] = [MARA, JONAS, LEO, EMMA, SOFIA, MIA, NOAH, EVAN];

fn integer(state: &WorldState, entity: EntityId, key: &str) -> Option<i64> {
    match state.entity(entity)?.component(key)? {
        Value::Integer(value) => Some(*value),
        _ => None,
    }
}

fn text<'a>(event: &'a Event, key: &str) -> Option<&'a str> {
    match event.payload.get(key) {
        Some(Value::Text(text)) => Some(text),
        _ => None,
    }
}

/// The day of the harbour's year now, from 0.
fn day_of_year(state: &WorldState) -> u64 {
    calendar::day_of_year(state, &crate::almanac::almanac(state))
}

/// The day, counted from the World's first.
fn today(state: &WorldState) -> i64 {
    (state.world_time() / WORLD_DAY_TICKS) as i64
}

/// The world time the year `year` (from 1) begins at.
fn year_start(year: u64) -> u64 {
    year.saturating_sub(1) * YEAR_DAYS * WORLD_DAY_TICKS
}

fn year_of(world_time: u64) -> u64 {
    world_time / WORLD_DAY_TICKS / YEAR_DAYS + 1
}

/// The festival a page is filled by, if it is held today.
fn beat_today(state: &WorldState, beat: &Beat) -> bool {
    crate::almanac::almanac(state)
        .festivals
        .iter()
        .any(|festival| {
            festival.id == beat.festival && festival.day % YEAR_DAYS == day_of_year(state)
        })
}

/// At the end of each harbour day, after the calendar: a page is filled
/// by the day that fills it, as an Event caused by that day's festival,
/// and on the year's last day the year is closed.
pub(crate) fn tick(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    let mut events = Vec::new();
    let now = world.world_time();
    for beat in &ARC {
        let state = world.state();
        if !beat_today(state, beat) || integer(state, HARBOR, FILLED) == Some(today(state)) {
            continue;
        }
        let held = world
            .events_of_kind(&["festival_held"])
            .into_iter()
            .rev()
            .take_while(|event| event.world_time == now)
            .find(|event| text(event, "festival") == Some(beat.festival))
            .map(|event| event.id);
        if let Some(held) = held {
            let request = ActionRequest::new("keeper_fills_page")
                .arg("page", beat.id)
                .caused_by(held);
            events.push(world.execute(actions, &request)?.id);
        }
    }
    let state = world.state();
    if day_of_year(state) == YEAR_DAYS - 1
        && integer(state, HARBOR, CLOSED) < Some(crate::years::year(state) as i64)
        && state.entity(HARBOR).is_some()
    {
        let request = ActionRequest::new("keeper_closes_year");
        events.push(world.execute(actions, &request)?.id);
    }
    Ok(events)
}

/// A page of the year is filled, on the day that fills it.
struct FillPage;

impl Action for FillPage {
    fn name(&self) -> &'static str {
        "keeper_fills_page"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let page = match request.args.get("page") {
            Some(Value::Text(page)) => page.as_str(),
            _ => return Err(ActionError::Invalid("which page?".into())),
        };
        let beat = ARC
            .iter()
            .find(|beat| beat.id == page)
            .ok_or_else(|| ActionError::Invalid(format!("no page {page}")))?;
        if !beat_today(state, beat) {
            return Err(ActionError::Invalid("it isn't this page's day".into()));
        }
        if state.entity(HARBOR).is_none() || integer(state, HARBOR, FILLED) == Some(today(state)) {
            return Err(ActionError::Invalid("a page was filled today".into()));
        }
        let mut draft = EventDraft::new(PAGE_FILLED);
        draft.targets = vec![HARBOR];
        draft.payload.insert("page".into(), beat.id.into());
        draft.changes = vec![StateChange::SetComponent {
            entity: HARBOR,
            key: FILLED.into(),
            value: today(state).into(),
        }];
        Ok(draft)
    }
}

/// The year closes, on its last day, with its credits.
struct CloseYear;

impl Action for CloseYear {
    fn name(&self) -> &'static str {
        "keeper_closes_year"
    }

    fn evaluate(
        &self,
        state: &WorldState,
        _request: &ActionRequest,
    ) -> Result<EventDraft, ActionError> {
        let year = crate::years::year(state) as i64;
        if day_of_year(state) != YEAR_DAYS - 1 || state.entity(HARBOR).is_none() {
            return Err(ActionError::Invalid("it isn't the year's last day".into()));
        }
        if integer(state, HARBOR, CLOSED) >= Some(year) {
            return Err(ActionError::Invalid("this year is closed".into()));
        }
        let mut draft = EventDraft::new(YEAR_CLOSED);
        draft.targets = vec![HARBOR];
        draft.payload.insert("year".into(), year.into());
        draft.changes = vec![StateChange::SetComponent {
            entity: HARBOR,
            key: CLOSED.into(),
            value: year.into(),
        }];
        Ok(draft)
    }
}

pub(crate) fn register_actions(registry: &mut ActionRegistry) -> Result<(), ActionError> {
    registry.register(FillPage)?;
    registry.register(CloseYear)
}

/// The kinds an answer the player gave is recorded as.
fn answer_kinds() -> &'static [&'static str] {
    static KINDS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    KINDS.get_or_init(|| {
        let mut kinds = crate::story::storylet_kinds()
            .filter(|kind| *kind != "situation_arose")
            .chain(["situation_answered"])
            .collect::<Vec<_>>();
        kinds.sort_unstable();
        kinds.dedup();
        kinds
    })
}

const MADE_KINDS: [&str; 3] = ["built_by_hand", "decorated_by_hand", "planted_by_hand"];

/// Whether something made was the player's own deed, not the harbour
/// getting on with one of its own works.
fn made_by_player(event: &Event) -> bool {
    !text(event, "storylet").is_some_and(|storylet| storylet.starts_with("own_"))
}

/// Whether an answer was the player's, not one the harbour came to by
/// itself while nobody answered. A storylet keeps what was chosen as its
/// `choice`; the lives System keeps it as its `answer` (one nobody gave is
/// recorded as `situation_lapsed`, never among these).
fn answered_by_player(event: &Event) -> bool {
    (event.payload.contains_key("choice") || event.payload.contains_key("answer"))
        && !event.payload.contains_key("lapsed")
        && !crate::drift::was_drifted(event)
}

/// The player's deeds recorded in `span` of world time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Deeded {
    pub made: u32,
    pub answered: u32,
    pub favours: u32,
}

fn deeds(world: &World, span: std::ops::Range<u64>) -> Deeded {
    let within = |event: &&Event| span.contains(&event.world_time);
    let count = |kinds: &[&str], mine: fn(&Event) -> bool| {
        world
            .events_of_kind(kinds)
            .into_iter()
            .filter(within)
            .filter(|event| mine(event))
            .count() as u32
    };
    Deeded {
        made: count(&MADE_KINDS, made_by_player),
        answered: count(answer_kinds(), answered_by_player),
        favours: count(&[conversation::favour::DONE], |_| true),
    }
}

/// How far each project has come: its steps done, of how many, and the
/// next one.
pub(crate) fn projects(world: &World) -> Vec<(&'static Project, u32)> {
    let ever = deeds(world, 0..u64::MAX);
    PROJECTS
        .iter()
        .map(|project| {
            let funds = match project.funded_by {
                Deeds::Made => ever.made,
                Deeds::Helped => ever.answered + ever.favours,
            };
            let done = project
                .steps
                .iter()
                .take_while(|(needs, _)| funds >= *needs)
                .count() as u32;
            (project, done)
        })
        .collect()
}

/// Which beats of each resident's heart the player has reached, in order
/// of [`HEART`].
pub(crate) fn hearts(world: &World) -> Vec<(EntityId, [bool; 5])> {
    let state = world.state();
    let met = lives::met(world);
    let actors = |kind: &str| {
        world
            .events_of_kind(&[kind])
            .into_iter()
            .filter_map(|event| event.actor)
            .collect::<BTreeSet<_>>()
    };
    let favoured = actors(conversation::favour::DONE);
    let wrote = actors("letter_written");
    let gave = lives::keepsakes(world)
        .into_iter()
        .map(|kept| kept.from)
        .collect::<BTreeSet<_>>();
    HEARTS
        .iter()
        .filter(|who| state.entity(**who).is_some())
        .map(|who| {
            (
                *who,
                [
                    met.contains(who),
                    favoured.contains(who),
                    wrote.contains(who),
                    gave.contains(who),
                    storylets::kindness(state, crate::story::deck(), *who).0 > 0,
                ],
            )
        })
        .collect()
}

/// Which pages have been filled in `year`, by beat, with the Event each
/// was filled with.
fn pages_of(world: &World, year: u64) -> BTreeMap<&'static str, &Event> {
    world
        .events_of_kind(&[PAGE_FILLED])
        .into_iter()
        .filter(|event| year_of(event.world_time) == year)
        .filter_map(|event| {
            let beat = ARC
                .iter()
                .find(|beat| Some(beat.id) == text(event, "page"))?;
            Some((beat.id, event))
        })
        .collect()
}

/// Every page ever filled, by beat.
pub(crate) fn pages_ever(world: &World) -> BTreeSet<&'static str> {
    world
        .events_of_kind(&[PAGE_FILLED])
        .into_iter()
        .filter_map(|event| ARC.iter().find(|beat| Some(beat.id) == text(event, "page")))
        .map(|beat| beat.id)
        .collect()
}

/// How many years the Book has closed.
pub(crate) fn years_closed(world: &World) -> u32 {
    world.events_of_kind(&[YEAR_CLOSED]).len() as u32
}

/// How the page a festival filled reads: how the day went.
fn page_line(world: &World, page: &Event) -> String {
    page.caused_by
        .iter()
        .filter_map(|cause| world.event(*cause))
        .find_map(|cause| text(cause, "told"))
        .map(|told| format!("{told}."))
        .unwrap_or_default()
}

/// The year's credits, from the Events recorded in it: every resident in
/// the order the year first names them, with what each did, and the
/// player's deeds last.
fn credits(world: &World, closed: &Event) -> Vec<Credit> {
    let state = world.state();
    let year = year_of(closed.world_time);
    let start = year_start(year);
    let events = world.events();
    let from = events.partition_point(|event| event.world_time < start);
    let to = events.partition_point(|event| event.world_time <= closed.world_time);
    let lived = &events[from..to];
    let prepared = crate::almanac::almanac(state)
        .festivals
        .iter()
        .filter(|festival| festival.prepare > 0)
        .map(|festival| festival.id)
        .collect::<BTreeSet<_>>();
    let mut residents = Vec::<EntityId>::new();
    let mut led = BTreeMap::<EntityId, Vec<String>>::new();
    let mut wrote = BTreeSet::new();
    let mut asked = BTreeSet::new();
    for event in lived {
        let Some(actor) = event.actor else {
            continue;
        };
        let resident = state
            .entity(actor)
            .is_some_and(|entity| entity.kind == "resident");
        if !resident {
            continue;
        }
        if !residents.contains(&actor) {
            residents.push(actor);
        }
        match event.kind.as_str() {
            // A day that is got ready for is led; one that simply comes
            // (the swallows, the gale) is nobody's doing.
            "festival_held" if prepared.contains(text(event, "festival").unwrap_or_default()) => {
                if let Some(name) = text(event, "name") {
                    let names = led.entry(actor).or_default();
                    if !names.iter().any(|led| led == name) {
                        names.push(name.to_string());
                    }
                }
            }
            "letter_written" => {
                wrote.insert(actor);
            }
            kind if kind == conversation::favour::DONE => {
                asked.insert(actor);
            }
            _ => {}
        }
    }
    let mut credits = residents
        .into_iter()
        .map(|who| {
            let deed = match led.get(&who).map(Vec::as_slice) {
                Some([one]) => format!("Led {one}"),
                Some([one, two, ..]) => format!("Led {one} and {two}"),
                _ if asked.contains(&who) => "Asked you a favour".to_string(),
                _ if wrote.contains(&who) => "Wrote you a letter".to_string(),
                _ => "Was part of the harbour's year".to_string(),
            };
            Credit {
                name: lives::first_name(state, who),
                deed,
                who: Some(SelectionId::Entity(who)),
            }
        })
        .collect::<Vec<_>>();
    let done = deeds(world, start..closed.world_time + 1);
    let pages = pages_of(world, year).len();
    let mut yours = vec![format!("Filled {pages} of the year's 4 pages")];
    for (count, one, many) in [
        (
            done.made,
            "Made one thing by hand",
            "Made {n} things by hand",
        ),
        (
            done.answered,
            "Answered one question",
            "Answered {n} questions",
        ),
        (done.favours, "Did one favour", "Did {n} favours"),
    ] {
        match count {
            0 => {}
            1 => yours.push(one.to_string()),
            n => yours.push(many.replace("{n}", &n.to_string())),
        }
    }
    credits.extend(yours.into_iter().map(|deed| Credit {
        name: "You".into(),
        deed,
        who: None,
    }));
    credits
}

/// The Book as the player keeps it: its title page, this year's pages,
/// each year closed with its credits, and what the harbour works toward.
/// `book` is the book of finds, as the snapshot carries it.
pub(crate) fn keeping(world: &World, book: &[BookEntry]) -> Keeping {
    let state = world.state();
    let year = crate::years::year(state);
    let filled = pages_of(world, year);
    let pages = ARC
        .iter()
        .map(|beat| {
            let title = if year == 1 { beat.first } else { beat.title };
            match filled.get(beat.id) {
                Some(page) => Page {
                    title: title.into(),
                    line: page_line(world, page),
                    filled: true,
                    moment: Some(SelectionId::Event(page.id)),
                },
                None => Page {
                    title: title.into(),
                    line: beat.to_come.into(),
                    filled: false,
                    moment: None,
                },
            }
        })
        .collect();
    let closed = world
        .events_of_kind(&[YEAR_CLOSED])
        .into_iter()
        .map(|event| {
            let year = year_of(event.world_time);
            YearClosed {
                year: year as u32,
                title: format!("The end of year {year}"),
                at: event.world_time,
                credits: credits(world, event),
                moment: Some(SelectionId::Event(event.id)),
            }
        })
        .collect();
    Keeping {
        title: TITLE.into(),
        role: if year == 1 { ROLE_NEW } else { ROLE }.into(),
        year: year as u32,
        pages,
        closed,
        aims: aims(world, book),
    }
}

/// What the harbour works toward, counted: its projects, the books to
/// complete and each resident's heart.
fn aims(world: &World, book: &[BookEntry]) -> Vec<Aim> {
    let mut aims = projects(world)
        .into_iter()
        .map(|(project, done)| Aim {
            group: "Projects".into(),
            label: project.label.into(),
            done,
            of: project.steps.len() as u32,
            next: project
                .steps
                .get(done as usize)
                .map(|(_, step)| (*step).to_string())
                .unwrap_or_default(),
            who: None,
        })
        .collect::<Vec<_>>();
    let count = |shelf: Option<&str>| {
        let entries = book
            .iter()
            .filter(|entry| shelf.is_none_or(|shelf| entry.shelf == shelf))
            .collect::<Vec<_>>();
        let found = entries.iter().filter(|entry| entry.found).count() as u32;
        (found, entries.len() as u32)
    };
    for (label, shelf, next) in [
        ("The book of finds", None, "Keep finding"),
        ("The almanac's days", Some("Days"), "Live a whole year"),
    ] {
        let (done, of) = count(shelf);
        if of > 0 {
            aims.push(Aim {
                group: "To complete".into(),
                label: label.into(),
                done,
                of,
                next: if done < of {
                    next.into()
                } else {
                    String::new()
                },
                who: None,
            });
        }
    }
    let state = world.state();
    for (who, beats) in hearts(world) {
        let name = lives::first_name(state, who);
        let reached = beats.iter().filter(|beat| **beat).count() as u32;
        let next = HEART
            .iter()
            .zip(beats)
            .find(|(_, reached)| !reached)
            .map(|(beat, _)| beat.replace("{name}", &name))
            .unwrap_or_default();
        aims.push(Aim {
            group: "Hearts".into(),
            label: name.clone(),
            done: reached,
            of: HEART.len() as u32,
            next,
            who: Some(SelectionId::Entity(who)),
        });
    }
    aims
}

#[cfg(test)]
#[path = "keeper_tests.rs"]
pub(crate) mod tests;
