//! The harbour as a place you pan along: the quay and the lighthouse, the
//! square, and the school on the hill. Everyone who lives here has a home
//! on it, every work the harbour finished stands in a slot of its own, and
//! each person keeps a day: at work, out of an evening now and then, home
//! at night, and on the square for a festival. The season lies on the
//! ground.
//!
//! None of it is recorded. Homes, slots and days are worked out from who
//! lives here and what was built each time the harbour is drawn, so a
//! harbour saved before any of this existed opens onto the same place.

use crate::story::{ADA, IVO};
use crate::{
    BAKERY, EMMA, EVAN, HARBOR, JONAS, JONAS_BOAT, LEO, MARA, MIA, NOAH, PUB, SCHOOL, SOFIA,
    WEDDING_ORDER,
};
use days::{Plan, Row, Street, Stretch};
use std::collections::BTreeMap;
use world_core::{EntityId, Value, World, WorldState};
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasProjection, District, GroundCover, MarkShape, RoutineStop,
    Season, SelectionId,
};

/// How wide the harbour is, in screens.
pub(crate) const WIDTH: f32 = 4.5;

pub(crate) const QUAY: usize = 0;
pub(crate) const SQUARE: usize = 1;
pub(crate) const HILL: usize = 2;

pub(crate) const STRETCHES: [Stretch; 3] = [
    Stretch {
        id: "quay",
        from: 0.0,
        to: 1.5,
    },
    Stretch {
        id: "square",
        from: 1.5,
        to: 3.0,
    },
    Stretch {
        id: "hill",
        from: 3.0,
        to: WIDTH,
    },
];

const LABELS: [&str; 3] = [
    "The quay and the lighthouse",
    "The square",
    "The school and the hill",
];

/// The rows things stand in: houses at the back, works in the middle, and
/// small things at the front. No two rows share a spot along the ground.
const ROWS: [Row; 5] = [
    Row {
        pitch: 0.16,
        offset: 0.0,
    },
    Row {
        pitch: 0.16,
        offset: 0.08,
    },
    Row {
        pitch: 0.16,
        offset: 0.04,
    },
    Row {
        pitch: 0.08,
        offset: 0.02,
    },
    // The plots the player can build on, which nothing else takes.
    Row {
        pitch: crate::plots::PLOT_PITCH,
        offset: crate::plots::PLOT_OFFSET,
    },
];
const BACK: usize = 0;
const MIDDLE: usize = 1;
const NEARER: usize = 2;
const FRONT: usize = 3;
/// How far along the ground what stands in front of or behind a work on a
/// plot keeps from it.
const CLEAR: f32 = 0.08;
/// How far down the scene each row stands, for an app that knows one
/// screen only.
const ROW_Y: [f32; 5] = [0.3, 0.45, 0.6, 0.76, crate::plots::PLOT_Y];

/// Where the harbour's four buildings stand.
const PLACES: [(EntityId, f32); 4] = [(HARBOR, 0.6), (PUB, 1.7), (BAKERY, 2.3), (SCHOOL, 3.7)];

/// The ids homes and works go by on the scene. Neither is a thing the
/// harbour records, so they take ids no entity ever has.
const HOME_IDS: u64 = 900_000_000;
const WORK_IDS: u64 = 910_000_000;

/// The home of a household, by its first member.
pub(crate) fn home_id(founder: EntityId) -> SelectionId {
    SelectionId::Entity(EntityId::new(HOME_IDS + founder.0))
}

/// Whether a selection is one of the harbour's homes.
#[cfg(test)]
pub(crate) fn is_home(selection: SelectionId) -> bool {
    matches!(selection, SelectionId::Entity(id) if (HOME_IDS..WORK_IDS).contains(&id.0))
}

/// Everyone who lives in the harbour, away for now or not: nobody loses
/// their home by going to see their sister for a week.
pub(crate) fn residents(state: &WorldState) -> Vec<EntityId> {
    crate::talk::RESIDENTS
        .into_iter()
        .chain([ADA, IVO])
        .chain(lives::arrivals(state, &crate::life::cast()))
        .chain(lives::born_here(state, &crate::life::cast()))
        .filter(|id| state.entity(*id).is_some() && !lives::gone(state, *id))
        .collect()
}

/// Who lives together, by the household's first member: couples share a
/// home.
pub(crate) fn households(state: &WorldState) -> BTreeMap<EntityId, Vec<EntityId>> {
    days::households(
        &residents(state),
        |person| lives::partner(state, person),
        |person| lives::lives_with(state, person),
    )
}

/// Which stretch someone's house is on: the fisherfolk by the quay, the
/// baker and the publican on the square, the teacher up the hill, and
/// newcomers wherever there is a house.
fn home_stretch(founder: EntityId) -> usize {
    match founder {
        JONAS | EVAN | NOAH => QUAY,
        MARA | LEO | SOFIA => SQUARE,
        EMMA | MIA => HILL,
        _ => (days::mix(&[founder.0, 3]) % 3) as usize,
    }
}

/// A work the harbour can finish, as the scene draws it.
struct Work {
    id: &'static str,
    label: &'static str,
    shape: MarkShape,
}

/// Every work the harbour can finish, once each: its first goals, the
/// ladder's works (not their second coats of paint), and what it made of
/// its own accord.
fn catalog() -> &'static [Work] {
    static CATALOG: std::sync::OnceLock<Vec<Work>> = std::sync::OnceLock::new();
    CATALOG.get_or_init(|| {
        [
            Work {
                id: "pier",
                label: "The new pier",
                shape: MarkShape::Bridge,
            },
            Work {
                id: "lamp",
                label: "A lamp on the point",
                shape: MarkShape::Lamp,
            },
        ]
        .into_iter()
        .chain(
            crate::story::WORKS
                .iter()
                .chain(crate::story::LATER_WORKS)
                .map(|work| Work {
                    id: work.id,
                    label: work.label,
                    shape: work.shape,
                }),
        )
        .chain(crate::story::OWN_WORKS.iter().map(|work| Work {
            id: work.id,
            label: work.label,
            shape: work.shape,
        }))
        .collect()
    })
}

/// Which stretch a work stands on, by what it is.
fn work_stretch(id: &str, index: usize) -> usize {
    const QUAYSIDE: &[&str] = &[
        "pier",
        "lamp",
        "sea_wall",
        "harbour_clock",
        "boathouse",
        "fishers_statue",
        "quay_planters",
        "lighthouse_paint",
        "lifeboat_station",
        "smokehouse",
        "fish_market",
        "rowing_club",
        "beacon",
        "ferry_shelter",
        "seal_hide",
        "tide_gauge",
        "boat_yard",
        "lookout_tower",
        "sea_pool",
        "net_loft",
        "herring_shed",
        "wind_break",
        "jetty_ladder",
        "bathing_huts",
        "gull_gate",
        "own_driftwood_bench",
    ];
    const ON_THE_SQUARE: &[&str] = &[
        "bandstand",
        "fountain",
        "new_well",
        "postbox",
        "signposts",
        "village_hall",
        "bread_oven",
        "chapel_bell",
        "puppet_theatre",
        "picnic_tables",
        "sundial",
        "maypole",
        "reading_room",
        "mural",
        "tea_rooms",
        "bread_cart",
        "pub_terrace",
        "lantern_walk",
        "bandstand_roof",
        "chapel_windows",
        "workshop",
        "own_notice_board",
        "own_window_boxes",
    ];
    const UP_THE_HILL: &[&str] = &[
        "school_garden",
        "birdhouses",
        "school_swings",
        "orchard",
        "footbridge",
        "telescope",
        "cliff_path",
        "glasshouse",
        "duck_pond",
        "cottages",
        "dovecote",
        "herb_garden",
        "school_library",
        "paddling_pool",
        "own_rope_swing",
        "own_painted_stones",
        "own_herb_bed",
        "own_cairn",
    ];
    if QUAYSIDE.contains(&id) {
        QUAY
    } else if ON_THE_SQUARE.contains(&id) {
        SQUARE
    } else if UP_THE_HILL.contains(&id) {
        HILL
    } else {
        index % 3
    }
}

/// Whether a shape is a building someone could go into.
fn is_building(shape: MarkShape) -> bool {
    matches!(
        shape,
        MarkShape::House | MarkShape::Shop | MarkShape::Tower | MarkShape::Dome
    )
}

/// The works finished so far, in the catalog's order, each with its place
/// in the catalog and the day it was finished, when the World's history
/// still holds that day (a World opened from a checkpoint may not).
fn finished(world: &World) -> Vec<(usize, &'static Work, Option<u32>)> {
    let deck = crate::story::deck();
    let when = storylets::finished_goals(world, deck)
        .into_iter()
        .filter_map(|done| Some((done.goal, done.event?.1)))
        .collect::<BTreeMap<_, _>>();
    let period = crate::persistence::WORLD_DAY_TICKS;
    catalog()
        .iter()
        .enumerate()
        .filter(|(_, work)| storylets::finished(world.state(), deck, work.id))
        .map(|(index, work)| {
            let built = when
                .get(work.id)
                .map(|world_time| world_time.div_ceil(period) as u32);
            (index, work, built)
        })
        .collect()
}

/// The season on the ground: blossom all spring, leaves all autumn, frost
/// as winter comes in and snow in the deep of it, and the harbour's edge
/// frozen on the five coldest days of the year.
pub(crate) fn season_of(state: &WorldState) -> (Season, Option<GroundCover>, bool) {
    let almanac = crate::almanac::almanac(state);
    season_on(calendar::day_of_year(state, &almanac))
}

/// The season on a day of the harbour's year, counted from 0.
pub(crate) fn season_on(day: u64) -> (Season, Option<GroundCover>, bool) {
    let season = Season::from_index((day / (crate::almanac::YEAR_DAYS / 4)).min(3));
    let ground = match season {
        Season::Spring => Some(GroundCover::Blossom),
        Season::Summer => None,
        Season::Autumn => Some(GroundCover::Leaves),
        Season::Winter if day < 100 => Some(GroundCover::Frost),
        Season::Winter => Some(GroundCover::Snow),
    };
    (season, ground, (105..=109).contains(&day))
}

/// Whether today is a festival.
/// Whether a snapshot is of a festival day.
#[cfg(test)]
pub(crate) fn festival_today_of(snapshot: &world_projection::ProjectionSnapshot) -> bool {
    snapshot
        .calendar
        .as_ref()
        .is_some_and(|calendar| calendar.festival_today)
}

pub(crate) fn festival_today(state: &WorldState) -> bool {
    calendar::festival_today(state, &crate::almanac::almanac(state))
}

fn text<'a>(state: &'a WorldState, id: EntityId, key: &str) -> Option<&'a str> {
    match state.entity(id)?.component(key)? {
        Value::Text(text) => Some(text),
        _ => None,
    }
}

/// Which stretch a trade is plied on: boats and nets on the quay, bread,
/// music and mending on the square, gardens, bees and lessons up the hill.
fn trade_stretch(job: &str) -> Option<usize> {
    Some(match job {
        "fisher" | "net_mender" | "boat_builder" | "sailmaker" | "radio_operator" | "carpenter"
        | "mayor" => QUAY,
        "shop_assistant" | "musician" | "cook" | "apprentice_baker" | "cheesemaker"
        | "clockmaker" | "bookbinder" | "nurse" => SQUARE,
        "gardener" | "shepherd" | "beekeeper" | "tutor" | "storyteller" | "painter" | "potter"
        | "weaver" | "miller" => HILL,
        _ => return None,
    })
}

/// Where someone spends the working day, if they have work: the place
/// their job ties them to, or somewhere on the stretch their trade is
/// plied, among its buildings and the works finished there.
fn work_of(
    state: &WorldState,
    person: EntityId,
    workplaces: &[Vec<SelectionId>; 3],
) -> Option<SelectionId> {
    let job = text(state, person, society_basic::JOB);
    match job {
        Some("unemployed" | "retired") => return None,
        Some("student") => return Some(SelectionId::Entity(SCHOOL)),
        // A child of the harbour goes to school by day once old enough,
        // and a baby stays at home.
        Some("child") => {
            return (crate::kin::stage(state, person) != Some(lives::Stage::Baby))
                .then_some(SelectionId::Entity(SCHOOL))
        }
        _ => {}
    }
    // Someone drawn here by what the player built works at it.
    if let Some(work) = lives::drawn_by(state, person).filter(|work| state.entity(*work).is_some())
    {
        return Some(SelectionId::Entity(work));
    }
    let tied = state
        .relations()
        .find(|relation| relation.kind == "works_at" && relation.from == person)
        .map(|relation| relation.to);
    if let Some(place) = tied {
        return Some(SelectionId::Entity(place));
    }
    // A trade the harbour has no stretch for is plied wherever there is
    // room for it.
    let stretch = job
        .and_then(trade_stretch)
        .unwrap_or((days::mix(&[person.0, 13]) % 3) as usize);
    let places = &workplaces[stretch];
    (!places.is_empty())
        .then(|| places[(days::mix(&[person.0, 11]) % places.len() as u64) as usize])
}

/// Lays the harbour out along its panorama: every item on the scene gets
/// its place along the ground, homes and finished works are added, and
/// everyone gets a day.
pub(crate) fn lay_out(world: &World, items: Vec<CanvasItem>) -> CanvasProjection {
    let state = world.state();
    let mut street = Street::new(WIDTH, &STRETCHES, &ROWS);
    let mut placed: BTreeMap<SelectionId, f32> = BTreeMap::new();
    let mut items = items;

    // The four buildings first: they have always stood where they stand.
    for (id, px) in PLACES {
        let selection = SelectionId::Entity(id);
        if let Some(item) = items.iter_mut().find(|item| item.id == selection) {
            let px = street.take(BACK, street.stretch_at(px), px).unwrap_or(px);
            item.px = Some(px);
            placed.insert(selection, px);
        }
    }
    // Jonas's boat at the harbour, and the order at the bakery.
    for (id, by) in [(JONAS_BOAT, HARBOR), (WEDDING_ORDER, BAKERY)] {
        let selection = SelectionId::Entity(id);
        let near = placed
            .get(&SelectionId::Entity(by))
            .copied()
            .unwrap_or(WIDTH / 2.0);
        if let Some(item) = items.iter_mut().find(|item| item.id == selection) {
            if let Some(px) = street.take(FRONT, street.stretch_at(near), near + 0.05) {
                item.px = Some(px);
                placed.insert(selection, px);
            }
        }
    }

    // A home for every household.
    let households = households(state);
    let mut home_of: BTreeMap<EntityId, SelectionId> = BTreeMap::new();
    for (founder, members) in &households {
        let stretch = home_stretch(*founder);
        let near = street.spread(stretch, founder.0);
        let Some((row, px)) = street.take_first(&[BACK, MIDDLE], Some(stretch), near) else {
            continue;
        };
        let id = home_id(*founder);
        for member in members {
            home_of.insert(*member, id);
        }
        placed.insert(id, px);
        items.push(new_item(
            id,
            CanvasItemKind::Place,
            "Home".into(),
            members
                .iter()
                .map(|member| lives::first_name(state, *member))
                .collect::<Vec<_>>()
                .join(" · "),
            px,
            row,
            MarkShape::House,
            None,
        ));
    }

    // Every finished work, in the order it was finished, so what was
    // built first keeps its spot whatever comes after.
    // Every work the harbour could finish has a spot kept for it, in the
    // catalog's order, whether it is finished yet or not: what is built
    // never moves for what is built after it, and the stretches fill up
    // as the harbour builds.
    let done_works = finished(world);
    let mut spots = Vec::new();
    for (index, work) in catalog().iter().enumerate() {
        let stretch = work_stretch(work.id, index);
        let near = street.spread(stretch, index as u64);
        spots.push(street.take_first(&[MIDDLE, NEARER, FRONT], Some(stretch), near));
    }
    let mut workplaces: [Vec<SelectionId>; 3] = [
        vec![SelectionId::Entity(HARBOR)],
        vec![SelectionId::Entity(BAKERY), SelectionId::Entity(PUB)],
        vec![SelectionId::Entity(SCHOOL)],
    ];
    for (index, work, built) in &done_works {
        let (index, work) = (*index, *work);
        let Some((row, px)) = spots[index] else {
            continue;
        };
        let id = SelectionId::Entity(EntityId::new(WORK_IDS + index as u64));
        placed.insert(id, px);
        if let Some(at) = street.stretch_at(px) {
            workplaces[at].push(id);
        }
        let mut item = new_item(
            id,
            if is_building(work.shape) {
                CanvasItemKind::Place
            } else {
                CanvasItemKind::Object
            },
            work.label.into(),
            String::new(),
            px,
            row,
            work.shape,
            *built,
        );
        item.drawing = None;
        items.push(item);
    }

    // What the player built on plots stands on its plot, and everything
    // says what it can wear and be called.
    crate::plots::dress(world, &mut items);
    for item in items.iter().filter(|item| item.variant.is_some()) {
        if let Some(px) = item.px {
            placed.insert(item.id, px);
            if let Some(at) = street.stretch_at(px) {
                workplaces[at].push(item.id);
            }
            // Nothing the player puts up after stands right before or
            // behind it: a flag on a plot never hangs from a lamp post.
            for row in [NEARER, FRONT] {
                street.keep_clear(row, px, CLEAR);
            }
        }
    }

    // What answers and the player's hands put up, beside the place it
    // belongs to, or where the player put it.
    let mut fixtures = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.px.is_none() && item.kind == CanvasItemKind::Object)
        .map(|(at, item)| (item.id, at))
        .collect::<Vec<_>>();
    fixtures.sort();
    for (_, at) in fixtures {
        let item = &items[at];
        let near = item
            .spot
            .map(|spot| spot * WIDTH)
            .or_else(|| item.at.and_then(|place| placed.get(&place).copied()))
            .unwrap_or(WIDTH / 2.0);
        if let Some((_, px)) =
            street.take_first(&[FRONT, NEARER, MIDDLE], street.stretch_at(near), near)
        {
            items[at].px = Some(px);
            placed.insert(items[at].id, px);
        }
    }

    // Everyone's day.
    let festival = festival_today(state);
    let square = gathering(&done_works, &items);
    for item in items
        .iter_mut()
        .filter(|item| item.kind == CanvasItemKind::Actor)
    {
        let SelectionId::Entity(person) = item.id else {
            continue;
        };
        let Some(home) = home_of.get(&person).copied() else {
            continue;
        };
        item.home = Some(home);
        let plan = Plan {
            home,
            work: work_of(state, person, &workplaces)
                .filter(|work| placed.contains_key(work))
                .map(|work| (work, false)),
            about: [
                SelectionId::Entity(HARBOR),
                square,
                SelectionId::Entity(SCHOOL),
            ][(days::mix(&[person.0, 5]) % 3) as usize],
            evening: Some((SelectionId::Entity(PUB), true)),
            gathering: square,
            festival,
            seed: person.0,
        };
        item.day = days::day(&plan)
            .into_iter()
            .map(|stop| RoutineStop {
                from_hour: stop.from_hour,
                at: stop.at,
                inside: stop.inside,
            })
            .collect();
        item.px = item
            .at
            .and_then(|at| placed.get(&at).copied())
            .or_else(|| placed.get(&home).copied());
    }

    let (season, ground, ice) = season_of(state);
    CanvasProjection {
        items,
        links: Vec::new(),
        marks: Vec::new(),
        width: Some(WIDTH),
        districts: STRETCHES
            .iter()
            .zip(LABELS)
            .map(|(stretch, label)| District {
                id: stretch.id.into(),
                label: label.into(),
                from: stretch.from,
                to: stretch.to,
            })
            .collect(),
        season: Some(season),
        ground,
        ice,
        plots: crate::plots::canvas_plots(world),
    }
}

/// Where the harbour gathers on a festival: the bandstand, once there is
/// one, or the maypole or the fountain, and otherwise outside the bakery.
fn gathering(done: &[(usize, &'static Work, Option<u32>)], items: &[CanvasItem]) -> SelectionId {
    ["bandstand", "maypole", "fountain"]
        .into_iter()
        .find_map(|id| {
            let (_, work, _) = done.iter().find(|(_, work, _)| work.id == id)?;
            items
                .iter()
                .find(|item| item.label == work.label && item.px.is_some())
                .map(|item| item.id)
        })
        .unwrap_or(SelectionId::Entity(BAKERY))
}

#[allow(clippy::too_many_arguments)]
fn new_item(
    id: SelectionId,
    kind: CanvasItemKind,
    label: String,
    detail: String,
    px: f32,
    row: usize,
    shape: MarkShape,
    built: Option<u32>,
) -> CanvasItem {
    CanvasItem {
        id,
        kind,
        label,
        detail,
        x: px / WIDTH,
        y: ROW_Y[row],
        changes: Vec::new(),
        shape: Some(shape),
        at: None,
        look: None,
        drawing: None,
        stance: None,
        standing: None,
        mood: None,
        spot: None,
        px: Some(px),
        home: None,
        day: Vec::new(),
        built,
        ..Default::default()
    }
}

/// How the place stands, as the bars for it measure it.
#[cfg(test)]
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct Bars {
    pub people: usize,
    /// People drawn with no home on the scene.
    pub homeless: usize,
    /// Couples living apart.
    pub apart: usize,
    pub outside_at_22: usize,
    /// The largest share of people in one district at noon.
    pub busiest_at_noon: f64,
    pub at_noon: BTreeMap<String, usize>,
    /// The share of people on the square at seven in the evening.
    pub square_in_evening: f64,
    pub festival: bool,
    /// Finished works, and those standing on the scene.
    pub finished: usize,
    pub standing: usize,
    /// Places and things standing in the same spot as another.
    pub shared_slots: usize,
}

#[cfg(test)]
pub(crate) fn bars(world: &World, snapshot: &world_projection::ProjectionSnapshot) -> Bars {
    let canvas = &snapshot.canvas;
    let people = canvas
        .items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .collect::<Vec<_>>();
    let homeless = people
        .iter()
        .filter(|person| {
            person
                .home
                .is_none_or(|home| !is_home(home) || canvas.px_of(home).is_none())
        })
        .count();
    let state = world.state();
    let apart = people
        .iter()
        .filter(|person| {
            let SelectionId::Entity(id) = person.id else {
                return false;
            };
            lives::partner(state, id).is_some_and(|partner| {
                people
                    .iter()
                    .find(|other| other.id == SelectionId::Entity(partner))
                    .is_some_and(|other| other.home != person.home)
            })
        })
        .count();
    let district_of = |stop: &RoutineStop| {
        canvas
            .px_of(stop.at)
            .and_then(|px| canvas.district_at(px))
            .map(|district| district.id.clone())
    };
    let outside_at_22 = canvas
        .whereabouts(22)
        .iter()
        .filter(|(_, stop)| !stop.inside)
        .count();
    let noon = canvas.whereabouts(12);
    let mut by_district: BTreeMap<String, usize> = BTreeMap::new();
    for (_, stop) in &noon {
        *by_district
            .entry(district_of(stop).unwrap_or_default())
            .or_default() += 1;
    }
    let busiest_at_noon =
        by_district.values().copied().max().unwrap_or(0) as f64 / noon.len().max(1) as f64;
    let evening = canvas.whereabouts(19);
    let square_in_evening = evening
        .iter()
        .filter(|(_, stop)| district_of(stop).as_deref() == Some("square"))
        .count() as f64
        / evening.len().max(1) as f64;
    let done = finished(world);
    let standing = done
        .iter()
        .filter(|(_, work, _)| {
            canvas
                .items
                .iter()
                .any(|item| item.label == work.label && item.px.is_some())
        })
        .count();
    let mut spots = canvas
        .items
        .iter()
        .filter(|item| item.kind != CanvasItemKind::Actor)
        .filter_map(|item| item.px)
        .collect::<Vec<_>>();
    spots.sort_by(f32::total_cmp);
    let shared_slots = spots
        .windows(2)
        .filter(|pair| pair[1] - pair[0] < 0.01)
        .count();
    Bars {
        people: people.len(),
        homeless,
        apart,
        outside_at_22,
        busiest_at_noon,
        at_noon: by_district,
        square_in_evening,
        festival: festival_today(state),
        finished: done.len(),
        standing,
        shared_slots,
    }
}

/// Asserts the bars for the place on a day.
#[cfg(test)]
pub(crate) fn check_bars(day: usize, bars: &Bars) {
    eprintln!("day {day}: {bars:?}");
    assert_eq!(bars.homeless, 0, "day {day}: {bars:?}");
    assert_eq!(bars.apart, 0, "day {day}: {bars:?}");
    assert_eq!(bars.shared_slots, 0, "day {day}: {bars:?}");
    assert_eq!(bars.standing, bars.finished, "day {day}: {bars:?}");
    assert!(bars.busiest_at_noon <= 0.6, "day {day}: {bars:?}");
    if bars.festival {
        assert!(bars.square_in_evening > 0.5, "day {day}: {bars:?}");
    } else {
        assert!(bars.outside_at_22 <= 3, "day {day}: {bars:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{story, TinySociety};

    /// The warm player of the long run: the first answer each day, and
    /// something made every third day. Hands each day's World and snapshot
    /// to `look`.
    pub(crate) fn play(
        days: usize,
        mut look: impl FnMut(usize, &World, &world_projection::ProjectionSnapshot),
    ) {
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        for day in 1..=days {
            let snapshot = branch.projection_snapshot();
            if let Some(answer) = snapshot.commands.iter().find(|command| {
                command.question.is_some()
                    && command.unavailable.is_none()
                    && command.id != story::WAIT_COMMAND
            }) {
                let _ = branch.invoke_projection_command(&answer.id.clone());
            }
            if day % 3 == 0 {
                if let Some(deed) = snapshot.commands.iter().find(|command| {
                    command.unavailable.is_none()
                        && command
                            .hand
                            .as_ref()
                            .is_some_and(|hand| hand.verb != "Undo")
                }) {
                    let _ = branch.invoke_projection_command(&deed.id.clone());
                }
            }
            branch
                .invoke_projection_command(story::WAIT_COMMAND)
                .unwrap();
            let snapshot = branch.projection_snapshot();
            look(day, branch.world(), &snapshot);
        }
    }

    /// Who is out of doors at an hour: the share of the residents drawn on
    /// the scene whose day has them outside then.
    pub(crate) fn outdoors(snapshot: &world_projection::ProjectionSnapshot, hour: u8) -> f32 {
        let people = snapshot
            .canvas
            .items
            .iter()
            .filter(|item| item.kind == CanvasItemKind::Actor)
            .collect::<Vec<_>>();
        let out = people
            .iter()
            .filter(|item| {
                let day = item
                    .day
                    .iter()
                    .map(|stop| days::Stop {
                        from_hour: stop.from_hour,
                        at: stop.at,
                        inside: stop.inside,
                    })
                    .collect::<Vec<_>>();
                days::stop_at(&day, hour).is_some_and(|stop| !stop.inside)
            })
            .count();
        out as f32 / people.len().max(1) as f32
    }

    fn check_outdoors(day: usize, snapshot: &world_projection::ProjectionSnapshot) {
        for hour in [10, 12, 15, 17] {
            let out = outdoors(snapshot, hour);
            assert!(
                out >= 0.6,
                "day {day}, {hour}:00: only {:.0}% outside",
                out * 100.0
            );
        }
        let out = outdoors(snapshot, 23);
        assert!(out <= 0.3, "day {day}, 23:00: {:.0}% outside", out * 100.0);
    }

    /// By day the harbour is out and about (at work, at school, on the
    /// square), and by night it is home.
    #[test]
    fn the_harbour_is_out_by_day_and_home_by_night() {
        play(60, |day, _, snapshot| {
            if day % 30 == 0 && !crate::town::festival_today_of(snapshot) {
                check_outdoors(day, snapshot);
            }
        });
    }

    /// The same, a year and three years on, children and elders among them.
    #[test]
    #[ignore]
    fn the_harbour_is_out_by_day_and_home_by_night_for_three_years() {
        play(1_080, |day, _, snapshot| {
            if [360, 1_080].contains(&day) {
                eprintln!(
                    "day {day}: outside at 10/12/15/17/23: {:?}",
                    [10, 12, 15, 17, 23].map(|hour| outdoors(snapshot, hour))
                );
                if !crate::town::festival_today_of(snapshot) {
                    check_outdoors(day, snapshot);
                }
            }
        });
    }

    /// A hundred and twenty days of the warm player: everyone has a home,
    /// every finished work stands in a spot of its own, nobody is out at
    /// ten at night, noon is spread across the harbour, and a festival
    /// fills the square.
    #[test]
    fn the_harbour_grows_and_keeps_its_days() {
        let mut festivals = 0;
        play(120, |day, world, snapshot| {
            let canvas = &snapshot.canvas;
            assert!(canvas.width.unwrap() >= 3.0);
            assert_eq!(canvas.districts.len(), 3);
            let bars = bars(world, snapshot);
            festivals += usize::from(bars.festival);
            if day % 30 == 0 || bars.festival {
                check_bars(day, &bars);
            }
            if day == 120 {
                assert!(bars.finished >= 5, "{bars:?}");
            }
        });
        assert!(festivals >= 5, "{festivals}");
    }

    /// The seasons lie on the ground: blossom in spring, leaves in autumn,
    /// snow in deep winter, and the harbour's edge frozen on a few of the
    /// coldest days only.
    #[test]
    fn the_seasons_lie_on_the_ground() {
        let mut ice = 0;
        let mut seen = Vec::new();
        for day in 0..crate::almanac::YEAR_DAYS {
            let (season, ground, frozen) = season_on(day);
            ice += usize::from(frozen);
            seen.push((day, season, ground));
            if frozen {
                assert_eq!(ground, Some(GroundCover::Snow));
            }
        }
        assert!((3..=7).contains(&ice), "{ice}");
        let ground_on = |at: u64| seen[at as usize].2;
        assert_eq!(ground_on(10), Some(GroundCover::Blossom));
        assert_eq!(ground_on(45), None);
        assert_eq!(ground_on(75), Some(GroundCover::Leaves));
        assert_eq!(ground_on(110), Some(GroundCover::Snow));
        assert_eq!(seen[110].1, Season::Winter);
    }
}
