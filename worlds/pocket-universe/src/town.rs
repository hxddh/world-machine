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
use days::town::{CatalogWork, Quarter, Town, Trace, TraceAt, Zone, BACK, FRONT};
use days::{Plan, Stretch};
use std::collections::BTreeMap;
use world_core::{EntityId, Value, World, WorldState};
use world_projection::Ground;
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasLink, CanvasProjection, GroundCover, MarkShape, Season,
    SelectionId,
};

/// A place's stretches, their names, and where its three seeded things
/// stand (the anchor, the second place, the thing that ranges out).
struct Layout {
    width: f32,
    stretches: [Stretch; 3],
    labels: [&'static str; 3],
    anchors: [(EntityId, f32); 3],
    home: &'static str,
    /// The clusters its homes and works stand in, left to right.
    quarters: &'static [Quarter],
}

/// What the twenty commonest storylets leave on the scene for a few days
/// after they end, in every place (a warm player's year, counted with the
/// `warm_days` example): crates after the supply drop, the delivery truck
/// or the herring run; flags for the launch window, the county fair or
/// the migration; a stall when the trader comes. Drawn by shape, so each
/// place draws its own (a cargo crate on Mars, a newspaper bundle on Maple
/// Street, a fish crate on the ice). Most stand by the place's middle
/// building, where the first screen looks.
const TRACES: &[Trace] = &[
    trace("supply", None, MarkShape::Parcel, TraceAt::Place(SLOT_C), 3),
    trace(
        "window",
        None,
        MarkShape::Bunting,
        TraceAt::Place(SLOT_C),
        3,
    ),
    trace("weather", None, MarkShape::Flag, TraceAt::Place(SLOT_C), 2),
    trace("trader", None, MarkShape::Stall, TraceAt::Place(SLOT_C), 3),
    trace("the_call", None, MarkShape::Lantern, TraceAt::Asker, 3),
    trace(
        "supper",
        Some("picnic-table"),
        MarkShape::Bench,
        TraceAt::Place(SLOT_C),
        2,
    ),
    trace(
        "stargazing",
        Some("telescope"),
        MarkShape::Tower,
        TraceAt::Place(SLOT_D),
        2,
    ),
    trace("signal", None, MarkShape::Lamp, TraceAt::Place(SLOT_D), 3),
    trace("quarrel", None, MarkShape::Bench, TraceAt::Asker, 3),
    trace("lost_tool", None, MarkShape::Parcel, TraceAt::Asker, 2),
    trace(
        "long_dark",
        None,
        MarkShape::Lantern,
        TraceAt::Place(SLOT_C),
        4,
    ),
    trace(
        "failing",
        Some("scaffold"),
        MarkShape::House,
        TraceAt::Place(SLOT_C),
        3,
    ),
    trace(
        "close_call",
        None,
        MarkShape::Signpost,
        TraceAt::Place(SLOT_D),
        3,
    ),
    trace(
        "clean_up",
        None,
        MarkShape::Planter,
        TraceAt::Place(SLOT_C),
        4,
    ),
    trace(
        "breaking_point",
        None,
        MarkShape::Postbox,
        TraceAt::Place(SLOT_C),
        3,
    ),
    trace("amends", None, MarkShape::Garden, TraceAt::Asker, 4),
    trace("keeper_grow", None, MarkShape::Sprouts, TraceAt::Asker, 4),
    trace(
        "first_frost",
        None,
        MarkShape::Tent,
        TraceAt::Place(SLOT_C),
        3,
    ),
    trace("birthday_*", None, MarkShape::Bunting, TraceAt::Asker, 2),
    // A part of a work done leaves its materials on the work's site.
    trace("work_*", None, MarkShape::Parcel, TraceAt::Works, 2),
];

/// A storylet's mark, briefly.
const fn trace(
    storylet: &'static str,
    art: Option<&'static str>,
    shape: MarkShape,
    at: TraceAt,
    days: u64,
) -> Trace {
    Trace {
        storylet,
        art,
        shape,
        at,
        days,
    }
}

/// A cluster of a place, briefly.
const fn quarter(
    id: &'static str,
    label: &'static str,
    at: f32,
    zones: &'static [Zone],
    homes: bool,
    ground: Ground,
) -> Quarter {
    Quarter {
        id,
        label,
        at,
        zones,
        homes,
        ground,
    }
}

fn layout(place: Place) -> &'static Layout {
    // Ares: a sheltered ring against a hostile plain. The habitats and the
    // green dome garden huddle round the lit airlock apron at the near end;
    // the plain runs out to the rover's tracks, the ridge and the mine.
    const ARES: Layout = Layout {
        width: 2.5,
        stretches: [
            Stretch {
                id: "domes",
                from: 0.0,
                to: 1.15,
            },
            Stretch {
                id: "pad",
                from: 1.15,
                to: 1.75,
            },
            Stretch {
                id: "ridge",
                from: 1.75,
                to: 2.5,
            },
        ],
        labels: ["The domes", "The landing pad", "The ridge and the ice mine"],
        anchors: [(SLOT_A, 0.45), (SLOT_C, 1.05), (SLOT_D, 1.85)],
        home: "Quarters",
        quarters: &[
            quarter(
                "hab_ring",
                "the Habitat Ring",
                0.35,
                &[Zone::Lanes],
                true,
                Ground::Pad,
            ),
            quarter(
                "dome_garden",
                "the Dome Garden",
                1.0,
                &[Zone::Green],
                false,
                Ground::Garden,
            ),
            quarter(
                "airlock_apron",
                "the Airlock Apron",
                1.3,
                &[Zone::Quay, Zone::Water, Zone::Lanes],
                true,
                Ground::Pad,
            ),
            quarter(
                "the_ridge",
                "the Ridge",
                2.05,
                &[Zone::Edge, Zone::Lanes],
                true,
                Ground::Yard,
            ),
            quarter(
                "crater_rim",
                "the Crater Rim",
                1.6,
                &[Zone::Water, Zone::Quay],
                false,
                Ground::Pad,
            ),
            quarter(
                "east_ring",
                "the East Ring",
                0.66,
                &[Zone::Lanes, Zone::Green],
                true,
                Ground::Pad,
            ),
            quarter(
                "the_mine",
                "the Mine",
                2.35,
                &[Zone::Edge, Zone::Water],
                false,
                Ground::Yard,
            ),
            quarter(
                "solar_flats",
                "the Solar Flats",
                1.75,
                &[Zone::Edge, Zone::Green],
                false,
                Ground::Pad,
            ),
        ],
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
        anchors: [(SLOT_A, 0.68), (SLOT_C, 1.25), (SLOT_D, 2.15)],
        home: "Home",
        quarters: &[
            quarter(
                "maple_row",
                "Maple Row",
                0.45,
                &[Zone::Lanes, Zone::Quay],
                true,
                Ground::Paving,
            ),
            quarter(
                "elm_court",
                "Elm Court",
                1.15,
                &[Zone::Lanes],
                true,
                Ground::Garden,
            ),
            quarter(
                "the_park",
                "the Park",
                1.6,
                &[Zone::Green, Zone::Quay],
                false,
                Ground::Green,
            ),
            quarter(
                "school_yard",
                "the School Yard",
                2.45,
                &[Zone::Lanes, Zone::Edge, Zone::Water],
                true,
                Ground::Paving,
            ),
            quarter(
                "main_street",
                "Main Street",
                0.8,
                &[Zone::Quay, Zone::Lanes],
                false,
                Ground::Paving,
            ),
            quarter(
                "oak_lane",
                "Oak Lane",
                0.15,
                &[Zone::Lanes],
                true,
                Ground::Garden,
            ),
            quarter(
                "the_lake",
                "the Lake",
                2.8,
                &[Zone::Water, Zone::Green],
                false,
                Ground::Green,
            ),
            quarter(
                "town_square",
                "the Square",
                1.85,
                &[Zone::Quay, Zone::Green],
                false,
                Ground::Paving,
            ),
        ],
    };
    const ICE: Layout = Layout {
        width: 2.5,
        stretches: [
            Stretch {
                id: "rookery",
                from: 0.0,
                to: 0.85,
            },
            Stretch {
                id: "bridge",
                from: 0.85,
                to: 1.75,
            },
            Stretch {
                id: "far_floe",
                from: 1.75,
                to: 2.5,
            },
        ],
        labels: ["The rookery", "The bridge", "The far floe"],
        anchors: [(SLOT_A, 0.55), (SLOT_C, 1.3), (SLOT_D, 1.95)],
        home: "Nest",
        quarters: &[
            quarter(
                "the_rookery",
                "the Rookery",
                0.3,
                &[Zone::Lanes],
                true,
                Ground::Snow,
            ),
            quarter(
                "ice_plaza",
                "the Ice Plaza",
                0.72,
                &[Zone::Green],
                false,
                Ground::Snow,
            ),
            quarter(
                "the_causeway",
                "the Causeway",
                1.3,
                &[Zone::Quay, Zone::Water, Zone::Lanes],
                true,
                Ground::Snow,
            ),
            quarter(
                "far_floe",
                "the Far Floe",
                2.05,
                &[Zone::Edge, Zone::Lanes, Zone::Water],
                true,
                Ground::Snow,
            ),
            quarter(
                "nest_row",
                "Nest Row",
                0.55,
                &[Zone::Lanes],
                true,
                Ground::Snow,
            ),
            quarter(
                "floe_edge",
                "the Floe Edge",
                1.0,
                &[Zone::Water, Zone::Quay],
                false,
                Ground::Snow,
            ),
            quarter(
                "kelp_beds",
                "the Kelp Beds",
                2.35,
                &[Zone::Edge, Zone::Water],
                false,
                Ground::Snow,
            ),
            quarter(
                "the_bridge",
                "the Bridge",
                1.6,
                &[Zone::Quay, Zone::Green],
                false,
                Ground::Snow,
            ),
        ],
    };
    match place {
        Place::Ares => &ARES,
        Place::Maple => &MAPLE,
        Place::Ice => &ICE,
    }
}

/// How wide a place is, in screens.
pub(crate) fn width(place: Place) -> f32 {
    layout(place).width
}

/// Which of a place's stretches a point lies in, the last if none.
pub(crate) fn stretch_at(place: Place, px: f32) -> usize {
    layout(place)
        .stretches
        .iter()
        .position(|stretch| stretch.holds(px))
        .unwrap_or(2)
}

/// The ids of a place's stretches, left to right.
pub(crate) fn stretch_ids(place: Place) -> [&'static str; 3] {
    layout(place).stretches.map(|stretch| stretch.id)
}

#[cfg(test)]
pub(crate) use days::town::is_home;

fn role(state: &WorldState, person: EntityId) -> Option<&str> {
    match state.entity(person)?.component("role")? {
        Value::Text(role) => Some(role),
        _ => None,
    }
}

/// Whether someone is a child of the place, who lives with a couple.
fn is_child(state: &WorldState, person: EntityId) -> bool {
    matches!(role(state, person), Some("chick" | "child"))
}

/// Who lives together, by the household's first member: couples share a
/// home, and each chick lives with a couple, taken in turn.
pub(crate) fn households(state: &WorldState) -> BTreeMap<EntityId, Vec<EntityId>> {
    let cast = crate::life::cast(state);
    let people = crate::life::people_in(state)
        .into_iter()
        .chain(lives::children(state, &cast))
        .collect::<Vec<_>>();
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
            if let Some(parent) = lives::lives_with(state, person) {
                return Some(parent);
            }
            let at = children.iter().position(|child| *child == person)?;
            (!couples.is_empty()).then(|| couples[at % couples.len()])
        },
    )
}

/// A work a place can finish, as the scene draws it.
pub(crate) struct Work {
    pub(crate) id: &'static str,
    pub(crate) label: String,
    pub(crate) shape: MarkShape,
}

/// Every work a place can finish, once each: its first goals, then its
/// ladder's works (not their second rounds).
pub(crate) fn catalog(world: &World, place: Place) -> Vec<Work> {
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
    let mut town =
        Town::new(width, &layout.stretches, days::town::PLOT_ROW_AT).quarters(layout.quarters);
    let mut items = items;

    for (id, px) in layout.anchors {
        let row = |item: &CanvasItem| {
            if item.kind == CanvasItemKind::Place {
                BACK
            } else {
                FRONT
            }
        };
        town.stand(&mut items, id, row, px, false);
    }

    let home_of = town.homes(
        &mut items,
        &households(state),
        |founder| {
            if [SLOT_B, SLOT_E].contains(&founder) {
                0
            } else {
                (days::mix(&[founder.0, 3]) % 3) as usize
            }
        },
        layout.home,
        |member| lives::first_name(state, member),
        Some(crate::drawings::home_art(place)),
    );

    // Every work the place could finish has a spot kept for it, in the
    // catalog's order, whether finished yet or not: what is built never
    // moves for what is built after it.
    let catalog = catalog(world, place);
    let done = finished_in(world, &catalog);
    let deck = crate::story::deck_ref();
    let works = catalog
        .into_iter()
        .enumerate()
        .map(|(index, work)| CatalogWork {
            stretch: work_stretch(place, work.id, index),
            under_way: storylets::progress(state, deck, work.id) > 0,
            id: work.id,
            label: work.label,
            shape: work.shape,
        });
    // What the player built on plots stands on its plot first (so no work
    // stands hidden behind it), and everything says what it can wear and
    // be called.
    crate::plots::dress(world, &mut items);
    crate::drawings::dress_art(world, &mut items);
    let mut on_plots: [Vec<SelectionId>; 3] = Default::default();
    town.plotted(&mut items, &mut on_plots);
    let mut works = town.works(&mut items, works, &done, |id| {
        crate::drawings::art_of_work(place, id).map(Into::into)
    });
    for (at, built) in on_plots.into_iter().enumerate() {
        works[at].extend(built);
    }

    // What the last few periods' storylets left behind them.
    let sites = crate::town::catalog(world, place)
        .into_iter()
        .map(|work| work.id)
        .collect::<Vec<_>>();
    items.extend(days::town::traces(
        world.events(),
        state.world_time(),
        crate::BACKGROUND_PERIOD,
        TRACES,
        |person| home_of.get(&person).copied(),
        |work| {
            let index = sites.iter().position(|each| *each == work)?;
            let id = days::town::work_id(index);
            town.placed.contains_key(&id).then_some(id)
        },
        |event| crate::story::told(world, event),
    ));

    town.fixtures(&mut items, |item| item.kind != CanvasItemKind::Actor);

    // Everyone's day. The pair work where they always have; those who came
    // later work at the place's buildings, spread along it.
    let festival = festival_today(state);
    let gathering = SelectionId::Entity(SLOT_A);
    // Where each stretch's people can spend the day: its seeded places and
    // the works finished on it.
    let mut workplaces = works;
    for (id, _) in layout.anchors {
        let selection = SelectionId::Entity(id);
        if let Some(at) = town.stretch_of(selection) {
            workplaces[at].insert(0, selection);
        }
    }
    // The pair work where they always have. Everyone else, in the order
    // they came, spends the day on the stretch with the fewest there yet,
    // so a place of a dozen is not all in one spot at noon.
    let mut busy = [0_usize; 3];
    let mut day_place: BTreeMap<EntityId, SelectionId> = BTreeMap::new();
    for person in [SLOT_B, SLOT_E] {
        if let Some(work) = crate::life::work(state, person).map(SelectionId::Entity) {
            if let Some(at) = town.stretch_of(work) {
                busy[at] += 1;
                day_place.insert(person, work);
            }
        }
    }
    let mut others = home_of.keys().copied().collect::<Vec<_>>();
    others.retain(|person| ![SLOT_B, SLOT_E].contains(person));
    for person in others {
        // Someone drawn here by what the player built works at it.
        if let Some(work) = lives::drawn_by(state, person)
            .map(SelectionId::Entity)
            .filter(|work| town.placed.contains_key(work))
        {
            if let Some(at) = town.stretch_of(work) {
                busy[at] += 1;
            }
            day_place.insert(person, work);
            continue;
        }
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
        town.live(item, home, &plan);
    }

    crate::drawings::vary_seats(&mut items);
    let almanac = crate::almanac::almanac(state);
    let mut canvas = town.projection(
        items,
        links,
        layout.labels,
        season_on(place, calendar::day_of_year(state, &almanac)),
        crate::plots::canvas_plots(world),
        crate::drawings::setting_of(place),
    );
    canvas.look = Some(look(place));
    canvas
}

/// How each place looks beyond its scenery (the art director's v0.29
/// brief): no two share their lamps or their light.
pub(crate) fn look(place: Place) -> world_projection::PlaceLook {
    match place {
        // A sheltered ring against a hostile plain: the lamps huddle round
        // the habitats, the dome garden and the airlock apron, and the
        // plain beyond stays rust dark. Inside, at night, the windows and
        // lamps glow teal and signal white.
        Place::Ares => world_projection::PlaceLook {
            lamps: vec![0.15, 0.26, 0.44, 0.53, 0.71, 0.88, 1.06, 1.34, 1.42, 2.32],
            key: None,
            glow: Some(0xc4f4ec),
            spine: None,
            haze: Some(0xe2a582),
        },
        // A street, not a lawn: streetlamps at the kerb, never quite
        // evenly; front yards, then a walk of warm concrete, then the
        // asphalt; air clear of any lilac haze.
        Place::Maple => world_projection::PlaceLook {
            lamps: vec![0.08, 0.3, 0.78, 1.14, 1.55, 1.9, 2.21, 2.6, 2.9],
            key: None,
            glow: None,
            spine: Some(0xb5aea4),
            haze: Some(0xdfe4e2),
        },
        // A causeway over dark water: lanterns at uneven steps along it,
        // lantern-gold on packed snow under a cold blue key light.
        Place::Ice => world_projection::PlaceLook {
            lamps: vec![0.21, 0.37, 0.62, 0.96, 1.2, 1.27, 1.49, 1.68, 2.03, 2.4],
            key: Some(0x9cc2f0),
            glow: Some(0xffbf47),
            spine: Some(0xe4edf3),
            haze: Some(0xd9e7f2),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PocketUniverse, NUDGE_COMMAND};
    use world_projection::ProjectionSnapshot;

    use world_pack_testkit::town::{check_town_bars, outdoors, town_bars, TownBars, TownFacts};

    fn bars(world: &World, snapshot: &ProjectionSnapshot) -> TownBars {
        let state = world.state();
        let canvas = &snapshot.canvas;
        let partner = |id: SelectionId| match id {
            SelectionId::Entity(id) => lives::partner(state, id).map(SelectionId::Entity),
            _ => None,
        };
        let gathering = canvas
            .px_of(SelectionId::Entity(SLOT_A))
            .and_then(|px| canvas.district_at(px))
            .map(|district| district.id.clone())
            .unwrap_or_default();
        town_bars(
            snapshot,
            &TownFacts {
                is_home: &is_home,
                partner: &partner,
                gathering,
                finished: finished(world, Place::of(state).unwrap())
                    .into_iter()
                    .map(|(_, work, _)| work.label)
                    .collect(),
                festival: festival_today(state),
            },
        )
    }

    fn check(seed: &str, day: usize, bars: &TownBars) {
        // A handful of people cannot spread over three stretches; a town
        // can.
        check_town_bars(&format!("{seed} day {day}"), bars, 5);
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
                    if !festival {
                        // Out and about by day, home by night.
                        for hour in [10, 12, 15, 17, 23] {
                            let out = outdoors(&snapshot, hour);
                            assert!(
                                if hour == 23 { out <= 0.3 } else { out >= 0.6 },
                                "{seed} day {day}, {hour}:00: {:.0}% outside",
                                out * 100.0
                            );
                        }
                    }
                }
            }
            assert!(festivals * 12 >= days, "{seed}: {festivals} festivals");
        }
    }

    /// A warm player in the place `seed` sows: the first answer each
    /// period and something made every fifth, handing each period's World
    /// and snapshot (and the first, as period 0) to `look`.
    fn warm(seed: &str, days: usize, mut look: impl FnMut(usize, &World, &ProjectionSnapshot)) {
        let mut universe = PocketUniverse::new().unwrap();
        universe.invoke_projection_command(seed).unwrap();
        look(0, universe.world(), &universe.projection_snapshot());
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
            look(day, universe.world(), &universe.projection_snapshot());
        }
    }

    const SEEDS: [&str; 3] = [
        crate::SEED_MARS_COLONY_COMMAND,
        crate::SEED_1980S_TOWN_COMMAND,
        crate::SEED_PENGUIN_CIVILIZATION_COMMAND,
    ];

    fn stage_of(snapshot: &ProjectionSnapshot) -> world_gpui::diorama::Stage {
        world_gpui::diorama::stage_at(snapshot, 1100.0, 848.0, world_gpui::diorama::Clock::at(12))
    }

    /// Every water work (the landing pad on the crater rim, the lake dock
    /// at the kerb, the ice ledge at the floe edge) stands on the water
    /// line, going up or finished.
    fn check_siting(seed: &str, day: usize, world: &World, snapshot: &ProjectionSnapshot) {
        let place = Place::of(world.state()).unwrap();
        let works = catalog(world, place);
        let stage = stage_of(snapshot);
        let astray = days::town::water_works_astray(
            &snapshot.canvas.items,
            |index| stage.on_water(index),
            |item| {
                works
                    .iter()
                    .find(|work| work.label == item.label)
                    .map(|work| {
                        days::town::zone_of(
                            crate::drawings::art_of_work(place, work.id),
                            work.shape,
                        )
                    })
            },
        );
        assert!(
            astray.is_empty(),
            "{seed} day {day}: off the water: {astray:?}"
        );
    }

    #[test]
    fn water_works_stand_on_the_water_line() {
        for seed in SEEDS {
            warm(seed, 120, |day, world, snapshot| {
                if day % 10 == 0 {
                    check_siting(seed, day, world, snapshot);
                }
            });
        }
    }

    /// The same over three years, a month at a time (the nightly runs it).
    #[test]
    #[ignore]
    fn water_works_stand_on_the_water_line_for_three_years() {
        for seed in SEEDS {
            warm(seed, 1_080, |day, world, snapshot| {
                if day % 30 == 0 {
                    check_siting(seed, day, world, snapshot);
                }
            });
        }
    }

    /// No screen-width of a place is bare in its first month.
    #[test]
    fn no_screen_width_is_bare_in_the_first_month() {
        for seed in SEEDS {
            warm(seed, 30, |day, _, snapshot| {
                let gap = days::town::widest_gap(&snapshot.canvas);
                assert!(gap < 1.0, "{seed} day {day}: {gap:.2} screens bare");
            });
        }
    }

    /// Every storylet a mark is kept for is one the place tells, so no
    /// mark waits on a storylet that never comes.
    #[test]
    fn every_trace_is_of_a_storylet_the_place_tells() {
        let deck = crate::story::deck_ref();
        for trace in TRACES {
            let known =
                deck.storylets
                    .iter()
                    .any(|storylet| match trace.storylet.strip_suffix('*') {
                        Some(prefix) => storylet.id.starts_with(prefix),
                        None => storylet.id == trace.storylet,
                    });
            assert!(known, "no storylet {}", trace.storylet);
        }
    }

    /// The first screen (the camera at rest) shows something new on at
    /// least 8 of a warm player's first 14 periods, in every place.
    #[test]
    fn the_first_screen_changes_on_most_of_the_first_fortnight() {
        for seed in SEEDS {
            let mut before = None;
            let mut changed = Vec::new();
            warm(seed, 14, |day, _, snapshot| {
                let now = stage_of(snapshot).first_screen(snapshot);
                if day > 0 && before.as_ref() != Some(&now) {
                    changed.push(day);
                }
                before = Some(now);
            });
            assert!(changed.len() >= 8, "{seed}: changed on {changed:?}");
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
