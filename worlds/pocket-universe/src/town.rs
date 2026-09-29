//! Each place as a panorama you pan along, in stretches of its own: Ares's
//! domes, landing pad and ridge; Maple Street's main street, square and
//! school; Icebridge's rookery, bridge and far floe. Everyone who lives
//! there has a home on it, every finished work stands in a slot of its
//! own, each person keeps a day, and the place's own year lies on the
//! ground: Mars's dust, Maple Street's four seasons, Icebridge's ice.
//!
//! None of it is recorded. Homes, slots and days are worked out from who
//! lives there and what was built each time the place is drawn, so a World
//! saved before any of this existed opens onto the same place.

use crate::places::Place;
use crate::{SLOT_A, SLOT_B, SLOT_C, SLOT_D, SLOT_E};
use days::{Plan, Row, Street, Stretch};
use std::collections::BTreeMap;
use world_core::{EntityId, Value, World, WorldState};
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasLink, CanvasProjection, District, GroundCover, MarkShape,
    RoutineStop, Season, SelectionId,
};

/// A place's stretches, their names, and where its three seeded things
/// stand (the anchor, the second place, the thing that ranges out).
struct Layout {
    width: f32,
    stretches: [Stretch; 3],
    labels: [&'static str; 3],
    anchors: [(EntityId, f32); 3],
    home: &'static str,
}

fn layout(place: Place) -> &'static Layout {
    const ARES: Layout = Layout {
        width: 2.5,
        stretches: [
            Stretch {
                id: "domes",
                from: 0.0,
                to: 0.9,
            },
            Stretch {
                id: "pad",
                from: 0.9,
                to: 1.7,
            },
            Stretch {
                id: "ridge",
                from: 1.7,
                to: 2.5,
            },
        ],
        labels: ["The domes", "The landing pad", "The ridge and the ice mine"],
        anchors: [(SLOT_A, 0.3), (SLOT_C, 0.65), (SLOT_D, 1.95)],
        home: "Quarters",
    };
    const MAPLE: Layout = Layout {
        width: 3.0,
        stretches: [
            Stretch {
                id: "main_street",
                from: 0.0,
                to: 1.0,
            },
            Stretch {
                id: "square",
                from: 1.0,
                to: 2.0,
            },
            Stretch {
                id: "school",
                from: 2.0,
                to: 3.0,
            },
        ],
        labels: [
            "Main Street",
            "The square and the park",
            "The school and the lake",
        ],
        anchors: [(SLOT_A, 0.35), (SLOT_C, 0.75), (SLOT_D, 1.3)],
        home: "Home",
    };
    const ICE: Layout = Layout {
        width: 2.5,
        stretches: [
            Stretch {
                id: "rookery",
                from: 0.0,
                to: 0.9,
            },
            Stretch {
                id: "bridge",
                from: 0.9,
                to: 1.7,
            },
            Stretch {
                id: "far_floe",
                from: 1.7,
                to: 2.5,
            },
        ],
        labels: ["The rookery", "The bridge", "The far floe"],
        anchors: [(SLOT_A, 0.35), (SLOT_C, 1.15), (SLOT_D, 1.45)],
        home: "Nest",
    };
    match place {
        Place::Ares => &ARES,
        Place::Maple => &MAPLE,
        Place::Ice => &ICE,
    }
}

/// Houses at the back, works in the middle, small things at the front; no
/// two rows share a spot along the ground.
const ROWS: [Row; 4] = [
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
];
const BACK: usize = 0;
const MIDDLE: usize = 1;
const NEARER: usize = 2;
const FRONT: usize = 3;
const ROW_Y: [f32; 4] = [0.3, 0.45, 0.6, 0.76];

/// The ids homes and works go by on the scene: neither is a thing the
/// World records, so they take ids no entity ever has.
const HOME_IDS: u64 = 900_000_000;
const WORK_IDS: u64 = 910_000_000;

pub(crate) fn home_id(founder: EntityId) -> SelectionId {
    SelectionId::Entity(EntityId::new(HOME_IDS + founder.0))
}

#[cfg(test)]
pub(crate) fn is_home(selection: SelectionId) -> bool {
    matches!(selection, SelectionId::Entity(id) if (HOME_IDS..WORK_IDS).contains(&id.0))
}

fn role(state: &WorldState, person: EntityId) -> Option<&str> {
    match state.entity(person)?.component("role")? {
        Value::Text(role) => Some(role),
        _ => None,
    }
}

/// Whether someone is a child of the place, who lives with a couple.
fn is_child(state: &WorldState, person: EntityId) -> bool {
    role(state, person) == Some("chick")
}

/// Who lives together, by the household's first member: couples share a
/// home, and each chick lives with a couple, taken in turn.
pub(crate) fn households(state: &WorldState) -> BTreeMap<EntityId, Vec<EntityId>> {
    let people = crate::life::people_in(state);
    let couples = people
        .iter()
        .copied()
        .filter(|person| !is_child(state, *person))
        .filter(|person| lives::partner(state, *person).is_some_and(|other| other.0 > person.0))
        .collect::<Vec<_>>();
    let children = people
        .iter()
        .copied()
        .filter(|person| is_child(state, *person))
        .collect::<Vec<_>>();
    days::households(
        &people,
        |person| lives::partner(state, person),
        |person| {
            let at = children.iter().position(|child| *child == person)?;
            (!couples.is_empty()).then(|| couples[at % couples.len()])
        },
    )
}

/// A work a place can finish, as the scene draws it.
struct Work {
    id: &'static str,
    label: String,
    shape: MarkShape,
}

/// Every work a place can finish, once each: its first goals, then its
/// ladder's works (not their second rounds).
fn catalog(world: &World, place: Place) -> Vec<Work> {
    let goals = crate::story::goals(world);
    ["second_home", "beacon", "survey"]
        .into_iter()
        .filter_map(|id| {
            let goal = goals.iter().find(|goal| goal.id == id)?;
            Some(Work {
                id,
                label: goal.label.clone(),
                shape: goal.shape,
            })
        })
        .chain(
            crate::story::ladder(place)
                .iter()
                .filter(|rung| rung.id == rung.work.id)
                .map(|rung| Work {
                    id: rung.id,
                    label: rung.work.label.to_string(),
                    shape: rung.work.shape,
                }),
        )
        .collect()
}

/// Which stretch a work stands on, by what it is.
fn work_stretch(place: Place, id: &str, index: usize) -> usize {
    let words: [&[&str]; 3] = match place {
        Place::Ares => [
            &[
                "mess",
                "greenhouse",
                "clinic",
                "rec_room",
                "quiet",
                "book",
                "flower",
                "walkway",
                "school",
                "bunkhouse",
                "seed",
                "algae",
                "second_home",
            ],
            &[
                "dust",
                "landing",
                "arch",
                "trading",
                "relay",
                "radio",
                "solar",
                "storm",
                "observatory",
                "beacon",
            ],
            &[
                "rover",
                "track",
                "cable",
                "mine",
                "tank",
                "still",
                "windbreak",
                "machine",
                "survey",
            ],
        ],
        Place::Maple => [
            &[
                "snack",
                "call_in",
                "antenna",
                "tape",
                "record",
                "pay_phone",
                "neon",
                "diner",
                "radio_van",
                "studio",
                "bank",
                "marquee",
                "arcade",
                "transmitter",
                "bus",
                "walk_of_fame",
                "drive_in",
                "second_home",
            ],
            &[
                "splash",
                "park",
                "streetlights",
                "dance",
                "gazebo",
                "market",
                "henderson",
                "picnic",
                "square",
                "speakers",
                "dog",
                "mural",
                "median",
                "welcome",
                "youth",
                "rink",
                "court",
                "skate_floor",
                "beacon",
            ],
            &[
                "crosswalk",
                "bleachers",
                "darkroom",
                "clubhouse",
                "bike",
                "treehouse",
                "study",
                "soapbox",
                "computer",
                "school",
                "capsule",
                "lake",
                "skate_park",
                "survey",
            ],
        ],
        Place::Ice => [
            &[
                "snow_wall",
                "creche",
                "egg",
                "nest",
                "chick",
                "council",
                "story",
                "song",
                "snow_hall",
                "elders",
                "name_wall",
                "wind_shelter",
                "swim",
                "second_home",
            ],
            &[
                "bridge",
                "ice_house",
                "vault",
                "kelp_racks",
                "pebble",
                "salt",
                "fog",
                "bone",
                "beacon",
            ],
            &[
                "berg",
                "sea_slide",
                "kelp_beds",
                "thaw",
                "rope",
                "ridge",
                "deep",
                "breathing",
                "aurora",
                "seal",
                "run_markers",
                "far",
                "survey",
            ],
        ],
    };
    words
        .iter()
        .position(|words| words.iter().any(|word| id.contains(word)))
        .unwrap_or(index % 3)
}

fn is_building(shape: MarkShape) -> bool {
    matches!(
        shape,
        MarkShape::House | MarkShape::Shop | MarkShape::Tower | MarkShape::Dome
    )
}

/// The works finished so far, oldest first.
/// The works finished so far, in the catalog's order, each with its place
/// in the catalog and the period it was finished, when the World's history
/// still holds it (a World opened from a checkpoint may not).
#[cfg(test)]
fn finished(world: &World, place: Place) -> Vec<(usize, Work, Option<u32>)> {
    let catalog = catalog(world, place);
    let done = finished_in(world, &catalog);
    catalog
        .into_iter()
        .enumerate()
        .filter_map(|(index, work)| Some((index, work, *done.get(&index)?)))
        .collect()
}

/// Which works of a catalog are finished, by their place in it, with the
/// period each was finished, if known.
fn finished_in(world: &World, catalog: &[Work]) -> BTreeMap<usize, Option<u32>> {
    let deck = crate::story::deck_ref();
    let when = storylets::finished_goals(world, deck)
        .into_iter()
        .filter_map(|done| Some((done.goal, done.event?.1)))
        .collect::<BTreeMap<_, _>>();
    catalog
        .iter()
        .enumerate()
        .filter(|(_, work)| storylets::finished(world.state(), deck, work.id))
        .map(|(index, work)| {
            let built = when
                .get(work.id)
                .map(|world_time| world_time.div_ceil(crate::BACKGROUND_PERIOD) as u32);
            (index, built)
        })
        .collect()
}

/// A place's season on the ground, from its own calendar: Mars's dust
/// season at the end of its year; Maple Street's blossom, leaves, frost and
/// snow, and the lake frozen on the coldest days; Icebridge's sea frozen
/// from the freeze-up to the thaw, and snow in the deep of it.
pub(crate) fn season_on(place: Place, day: u64) -> (Season, Option<GroundCover>, bool) {
    let season = Season::from_index((day / 30).min(3));
    match place {
        Place::Ares => (season, (day >= 95).then_some(GroundCover::Dust), false),
        Place::Maple => {
            let ground = match season {
                Season::Spring => Some(GroundCover::Blossom),
                Season::Summer => None,
                Season::Autumn => Some(GroundCover::Leaves),
                Season::Winter if day < 100 => Some(GroundCover::Frost),
                Season::Winter => Some(GroundCover::Snow),
            };
            (season, ground, (104..=108).contains(&day))
        }
        Place::Ice => {
            let frozen = !(25..85).contains(&day);
            let ground = if !(15..95).contains(&day) {
                Some(GroundCover::Snow)
            } else if frozen {
                Some(GroundCover::Frost)
            } else {
                None
            };
            (season, ground, frozen)
        }
    }
}

pub(crate) fn festival_today(state: &WorldState) -> bool {
    calendar::festival_today(state, &crate::almanac::almanac(state))
}

/// Lays a place out along its panorama. A World not yet begun is left as
/// it is.
pub(crate) fn lay_out(
    world: &World,
    items: Vec<CanvasItem>,
    links: Vec<CanvasLink>,
) -> CanvasProjection {
    let state = world.state();
    let Some(place) = Place::of(state) else {
        return CanvasProjection {
            items,
            links,
            ..CanvasProjection::default()
        };
    };
    let layout = layout(place);
    let width = layout.width;
    let mut street = Street::new(width, &layout.stretches, &ROWS);
    let mut placed: BTreeMap<SelectionId, f32> = BTreeMap::new();
    let mut items = items;

    for (id, px) in layout.anchors {
        let selection = SelectionId::Entity(id);
        if let Some(item) = items.iter_mut().find(|item| item.id == selection) {
            let row = if item.kind == CanvasItemKind::Place {
                BACK
            } else {
                FRONT
            };
            if let Some(px) = street.take(row, street.stretch_at(px), px) {
                item.px = Some(px);
                placed.insert(selection, px);
            }
        }
    }

    let households = households(state);
    let mut home_of: BTreeMap<EntityId, SelectionId> = BTreeMap::new();
    for (founder, members) in &households {
        let stretch = if [SLOT_B, SLOT_E].contains(founder) {
            0
        } else {
            (days::mix(&[founder.0, 3]) % 3) as usize
        };
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
            layout.home.into(),
            members
                .iter()
                .map(|member| lives::first_name(state, *member))
                .collect::<Vec<_>>()
                .join(" · "),
            (px, width, row),
            MarkShape::House,
            None,
        ));
    }

    // Every work the place could finish has a spot kept for it, in the
    // catalog's order, whether finished yet or not: what is built never
    // moves for what is built after it.
    let catalog = catalog(world, place);
    let done = finished_in(world, &catalog);
    let mut works: [Vec<SelectionId>; 3] = Default::default();
    for (index, work) in catalog.into_iter().enumerate() {
        let stretch = work_stretch(place, work.id, index);
        let near = street.spread(stretch, index as u64);
        let spot = street.take_first(&[MIDDLE, NEARER, FRONT], Some(stretch), near);
        let (Some((row, px)), Some(built)) = (spot, done.get(&index).copied()) else {
            continue;
        };
        let id = SelectionId::Entity(EntityId::new(WORK_IDS + index as u64));
        placed.insert(id, px);
        if let Some(at) = street.stretch_at(px) {
            works[at].push(id);
        }
        items.push(new_item(
            id,
            if is_building(work.shape) {
                CanvasItemKind::Place
            } else {
                CanvasItemKind::Object
            },
            work.label,
            String::new(),
            (px, width, row),
            work.shape,
            built,
        ));
    }

    let mut fixtures = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.px.is_none() && item.kind != CanvasItemKind::Actor)
        .map(|(at, item)| (item.id, at))
        .collect::<Vec<_>>();
    fixtures.sort();
    for (_, at) in fixtures {
        let item = &items[at];
        let near = item
            .spot
            .map(|spot| spot * width)
            .or_else(|| item.at.and_then(|place| placed.get(&place).copied()))
            .unwrap_or(width / 2.0);
        if let Some((_, px)) =
            street.take_first(&[FRONT, NEARER, MIDDLE], street.stretch_at(near), near)
        {
            items[at].px = Some(px);
            placed.insert(items[at].id, px);
        }
    }

    // Everyone's day. The pair work where they always have; those who came
    // later work at the place's buildings, spread along it.
    let festival = festival_today(state);
    let gathering = SelectionId::Entity(SLOT_A);
    // Where each stretch's people can spend the day: its seeded places and
    // the works finished on it.
    let mut workplaces = works;
    for (id, _) in layout.anchors {
        let selection = SelectionId::Entity(id);
        if let Some(at) = placed.get(&selection).and_then(|px| street.stretch_at(*px)) {
            workplaces[at].insert(0, selection);
        }
    }
    let stretch_of = |id: SelectionId| placed.get(&id).and_then(|px| street.stretch_at(*px));
    // The pair work where they always have. Everyone else, in the order
    // they came, spends the day on the stretch with the fewest there yet,
    // so a place of a dozen is not all in one spot at noon.
    let mut busy = [0_usize; 3];
    let mut day_place: BTreeMap<EntityId, SelectionId> = BTreeMap::new();
    for person in [SLOT_B, SLOT_E] {
        if let Some(work) = crate::life::work(state, person).map(SelectionId::Entity) {
            if let Some(at) = stretch_of(work) {
                busy[at] += 1;
                day_place.insert(person, work);
            }
        }
    }
    let mut others = home_of.keys().copied().collect::<Vec<_>>();
    others.retain(|person| ![SLOT_B, SLOT_E].contains(person));
    for person in others {
        let offset = (days::mix(&[person.0, 7]) % 3) as usize;
        let Some(at) = (0..3)
            .map(|step| (step + offset) % 3)
            .filter(|at| !workplaces[*at].is_empty())
            .min_by_key(|at| busy[*at])
        else {
            continue;
        };
        busy[at] += 1;
        let places = &workplaces[at];
        day_place.insert(
            person,
            places[(days::mix(&[person.0, 11]) % places.len() as u64) as usize],
        );
    }
    let evening_inside = place != Place::Ice;
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
        let spends = day_place.get(&person).copied();
        let child = is_child(state, person);
        let work = spends.filter(|_| !child);
        let about = spends.filter(|_| child).unwrap_or(gathering);
        let plan = Plan {
            home,
            work: work.map(|work| (work, false)),
            about,
            evening: Some((gathering, evening_inside)),
            gathering,
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

    let almanac = crate::almanac::almanac(state);
    let (season, ground, ice) = season_on(place, calendar::day_of_year(state, &almanac));
    CanvasProjection {
        items,
        links,
        marks: Vec::new(),
        width: Some(width),
        districts: layout
            .stretches
            .iter()
            .zip(layout.labels)
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
    }
}

fn new_item(
    id: SelectionId,
    kind: CanvasItemKind,
    label: String,
    detail: String,
    (px, width, row): (f32, f32, usize),
    shape: MarkShape,
    built: Option<u32>,
) -> CanvasItem {
    CanvasItem {
        id,
        kind,
        label,
        detail,
        x: px / width,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PocketUniverse, NUDGE_COMMAND};
    use world_projection::ProjectionSnapshot;

    /// How a place stands, as its bars measure it.
    #[derive(Debug)]
    struct Bars {
        people: usize,
        homeless: usize,
        apart: usize,
        outside_at_22: usize,
        busiest_at_noon: f64,
        gathered_in_evening: f64,
        festival: bool,
        finished: usize,
        standing: usize,
        shared_slots: usize,
    }

    fn bars(world: &World, snapshot: &ProjectionSnapshot) -> Bars {
        let canvas = &snapshot.canvas;
        let state = world.state();
        let place = Place::of(state).unwrap();
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
                .unwrap_or_default()
        };
        let noon = canvas.whereabouts(12);
        let mut by_district: BTreeMap<String, usize> = BTreeMap::new();
        for (_, stop) in &noon {
            *by_district.entry(district_of(stop)).or_default() += 1;
        }
        let gathering = canvas
            .px_of(SelectionId::Entity(SLOT_A))
            .and_then(|px| canvas.district_at(px))
            .map(|district| district.id.clone())
            .unwrap_or_default();
        let evening = canvas.whereabouts(19);
        let done = finished(world, place);
        let mut spots = canvas
            .items
            .iter()
            .filter(|item| item.kind != CanvasItemKind::Actor)
            .filter_map(|item| item.px)
            .collect::<Vec<_>>();
        spots.sort_by(f32::total_cmp);
        Bars {
            people: people.len(),
            homeless,
            apart,
            outside_at_22: canvas
                .whereabouts(22)
                .iter()
                .filter(|(_, stop)| !stop.inside)
                .count(),
            busiest_at_noon: by_district.values().copied().max().unwrap_or(0) as f64
                / noon.len().max(1) as f64,
            gathered_in_evening: evening
                .iter()
                .filter(|(_, stop)| district_of(stop) == gathering)
                .count() as f64
                / evening.len().max(1) as f64,
            festival: festival_today(state),
            finished: done.len(),
            standing: done
                .iter()
                .filter(|(_, work, _)| {
                    canvas
                        .items
                        .iter()
                        .any(|item| item.label == work.label && item.px.is_some())
                })
                .count(),
            shared_slots: spots
                .windows(2)
                .filter(|pair| pair[1] - pair[0] < 0.01)
                .count(),
        }
    }

    fn check(seed: &str, day: usize, bars: &Bars) {
        eprintln!("{seed} day {day}: {bars:?}");
        assert_eq!(bars.homeless, 0, "{seed} day {day}: {bars:?}");
        assert_eq!(bars.apart, 0, "{seed} day {day}: {bars:?}");
        assert_eq!(bars.shared_slots, 0, "{seed} day {day}: {bars:?}");
        assert_eq!(bars.standing, bars.finished, "{seed} day {day}: {bars:?}");
        // A handful of people cannot spread over three stretches; a town
        // can.
        if bars.people >= 5 {
            assert!(bars.busiest_at_noon <= 0.6, "{seed} day {day}: {bars:?}");
        }
        if bars.festival {
            assert!(bars.gathered_in_evening > 0.5, "{seed} day {day}: {bars:?}");
        } else {
            assert!(bars.outside_at_22 <= 3, "{seed} day {day}: {bars:?}");
        }
    }

    /// The generous player of the density tests in each place: the first
    /// answer each period, something made every fifth. The place is
    /// checked every 30 periods and on every festival.
    fn play(days: usize, at: &[usize]) {
        for seed in [
            crate::SEED_MARS_COLONY_COMMAND,
            crate::SEED_1980S_TOWN_COMMAND,
            crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
        ] {
            let mut universe = PocketUniverse::new().unwrap();
            universe.invoke_projection_command(seed).unwrap();
            let mut festivals = 0;
            for day in 1..=days {
                let snapshot = universe.projection_snapshot();
                if let Some(answer) = snapshot.commands.iter().find(|command| {
                    command.question.is_some()
                        && command.unavailable.is_none()
                        && command.id != NUDGE_COMMAND
                }) {
                    let _ = universe.invoke_projection_command(&answer.id.clone());
                }
                if day % 5 == 0 {
                    if let Some(deed) = snapshot.commands.iter().find(|command| {
                        command.unavailable.is_none()
                            && command
                                .hand
                                .as_ref()
                                .is_some_and(|hand| hand.verb != "Undo")
                    }) {
                        let _ = universe.invoke_projection_command(&deed.id.clone());
                    }
                }
                universe.invoke_projection_command(NUDGE_COMMAND).unwrap();
                let festival = festival_today(universe.world().state());
                festivals += usize::from(festival);
                if at.contains(&day) || festival {
                    let snapshot = universe.projection_snapshot();
                    let canvas = &snapshot.canvas;
                    assert!(canvas.width.unwrap() >= 2.0);
                    assert_eq!(canvas.districts.len(), 3);
                    assert!(canvas.season.is_some());
                    check(seed, day, &bars(universe.world(), &snapshot));
                }
            }
            assert!(festivals * 12 >= days, "{seed}: {festivals} festivals");
        }
    }

    #[test]
    fn each_place_grows_and_keeps_its_days() {
        play(120, &[30, 60, 90, 120]);
    }

    /// Three years of each place.
    #[test]
    #[ignore]
    fn each_place_grows_and_keeps_its_days_for_three_years() {
        play(1_080, &[30, 360, 720, 1_080]);
    }

    #[test]
    fn each_place_keeps_its_own_seasons() {
        let over_a_year = |place| {
            (0..120)
                .map(|day| season_on(place, day))
                .collect::<Vec<_>>()
        };
        let ares = over_a_year(Place::Ares);
        assert!(ares.iter().all(|(_, _, ice)| !ice));
        assert_eq!(ares[100].1, Some(GroundCover::Dust));
        assert_eq!(ares[40].1, None);
        let maple = over_a_year(Place::Maple);
        assert_eq!(maple[10].1, Some(GroundCover::Blossom));
        assert_eq!(maple[75].1, Some(GroundCover::Leaves));
        assert_eq!(maple[110].1, Some(GroundCover::Snow));
        let frozen = maple.iter().filter(|(_, _, ice)| *ice).count();
        assert!((3..=7).contains(&frozen), "{frozen}");
        // Icebridge's sea is frozen from the freeze-up to the thaw.
        let ice = over_a_year(Place::Ice);
        let turns = crate::places::turns(Place::Ice);
        assert!(turns.contains(&crate::places::Turn::Thaw));
        assert!(ice[24].2 && !ice[25].2 && !ice[84].2 && ice[85].2);
        assert_eq!(ice[110].1, Some(GroundCover::Snow));
        assert_eq!(ice[50].1, None);
    }
}
