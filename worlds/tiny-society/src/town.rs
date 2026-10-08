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
use days::town::{CatalogWork, Quarter, Town, Trace, TraceAt, Zone, BACK, FRONT, WATER_TALL};
use days::{Plan, Stretch};
use std::collections::BTreeMap;
use world_core::{EntityId, Value, World, WorldState};
use world_projection::Ground;
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasProjection, GroundCover, MarkShape, Season, SelectionId,
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

/// Where the harbour's four buildings stand.
/// The harbour (its lighthouse, where the fishers work) stands on the Point
/// at the east end, the pub just west of it on the Harbour Front, so one
/// frame holds the lighthouse, the pub and the town, and the quay's people
/// and the pub's share the first screen; the bakery is on the square and
/// the school at the west end, by Net Lane.
const PLACES: [(EntityId, f32); 4] = [(HARBOR, 4.26), (PUB, 3.85), (BAKERY, 2.3), (SCHOOL, 0.56)];

/// What the harbour's twenty commonest storylets leave on the scene for a
/// few days after they end (a warm player's year, counted with the
/// `warm_days` example): bunting after the fete, the boats racked after a
/// storm warning, scaffolding on the pub's chimney after the fire, stalls
/// on market day, a sailing boat after the regatta.
const TRACES: &[Trace] = &[
    trace(
        "chimney_fire",
        Some("scaffold"),
        MarkShape::House,
        TraceAt::Place(PUB),
        3,
    ),
    trace(
        "great_storm",
        Some("boat-rack"),
        MarkShape::Stall,
        TraceAt::Place(HARBOR),
        5,
    ),
    trace(
        "storm_warning",
        Some("boat-rack"),
        MarkShape::Stall,
        TraceAt::Place(HARBOR),
        3,
    ),
    trace(
        "storm_repairs",
        Some("scaffold"),
        MarkShape::House,
        TraceAt::Place(HARBOR),
        3,
    ),
    trace(
        "market_day",
        Some("market-stall"),
        MarkShape::Stall,
        TraceAt::Place(BAKERY),
        2,
    ),
    trace(
        "harbour_fete",
        Some("flag-line"),
        MarkShape::Bunting,
        TraceAt::Place(BAKERY),
        4,
    ),
    trace(
        "harvest_supper",
        Some("picnic-tables"),
        MarkShape::Bench,
        TraceAt::Place(PUB),
        3,
    ),
    trace(
        "regatta",
        Some("sailboat"),
        MarkShape::Boat,
        TraceAt::Place(HARBOR),
        4,
    ),
    trace(
        "town_meeting",
        Some("notice-board"),
        MarkShape::Signpost,
        TraceAt::Place(PUB),
        3,
    ),
    trace(
        "quarrel",
        Some("driftwood-bench"),
        MarkShape::Bench,
        TraceAt::Asker,
        3,
    ),
    trace("mainland_work", None, MarkShape::Parcel, TraceAt::Asker, 2),
    trace(
        "mackerel",
        Some("net-rack"),
        MarkShape::Stall,
        TraceAt::Place(HARBOR),
        3,
    ),
    trace(
        "inspector",
        Some("flower-boxes"),
        MarkShape::Planter,
        TraceAt::Place(BAKERY),
        4,
    ),
    trace(
        "hotel_order",
        Some("bread-cart"),
        MarkShape::Rover,
        TraceAt::Place(BAKERY),
        2,
    ),
    trace(
        "fever",
        Some("jar-lanterns"),
        MarkShape::Lantern,
        TraceAt::Asker,
        3,
    ),
    trace(
        "winter_coal",
        None,
        MarkShape::Parcel,
        TraceAt::Place(SCHOOL),
        3,
    ),
    trace(
        "traveller",
        Some("rowing-boat"),
        MarkShape::Boat,
        TraceAt::Place(HARBOR),
        3,
    ),
    trace(
        "traveller_stays",
        None,
        MarkShape::Tent,
        TraceAt::Place(BAKERY),
        3,
    ),
    trace(
        "birthday_*",
        Some("paper-lanterns"),
        MarkShape::Bunting,
        TraceAt::Asker,
        2,
    ),
    trace(
        "warm_snug",
        Some("fire-pit"),
        MarkShape::Lantern,
        TraceAt::Place(PUB),
        2,
    ),
    trace(
        "school_roof",
        Some("scaffold"),
        MarkShape::House,
        TraceAt::Place(SCHOOL),
        3,
    ),
    trace(
        "record_catch",
        None,
        MarkShape::Parcel,
        TraceAt::Place(HARBOR),
        2,
    ),
    // A part of a work done leaves its materials on the work's site.
    trace(
        "pier_timber",
        None,
        MarkShape::Parcel,
        TraceAt::Site("pier"),
        2,
    ),
    trace(
        "harbour_lamp",
        None,
        MarkShape::Parcel,
        TraceAt::Site("lamp"),
        2,
    ),
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

/// The clusters the harbour is laid out in, each a few homes or works on
/// one patch of ground, the first of each kind on a stretch filled first:
/// along the working quay the fishers' cottages, the Fish Quay, the
/// Boatyard, Net Lane and the Drying Green; on the square the Square,
/// Baker's Lane, the Green, Market Row, the Slip and the Little Green; at
/// the east end the Harbour Front by the pub and the lighthouse on the
/// Point, with Hill Lane, Chapel Hill, the Orchard and Hill Cottages up
/// behind.
pub(crate) const QUARTERS: [Quarter; 17] = [
    quarter(
        "fishers_row",
        "Fisher's Row",
        1.0,
        &[Zone::Lanes],
        true,
        Ground::Garden,
    ),
    quarter(
        "fish_quay",
        "the Fish Quay",
        1.3,
        &[Zone::Water, Zone::Quay],
        false,
        Ground::Cobbles,
    ),
    quarter(
        "boatyard",
        "the Boatyard",
        0.4,
        &[Zone::Water, Zone::Quay],
        false,
        Ground::Yard,
    ),
    quarter(
        "net_lane",
        "Net Lane",
        0.6,
        &[Zone::Lanes, Zone::Green],
        true,
        Ground::Garden,
    ),
    quarter(
        "drying_green",
        "the Drying Green",
        0.2,
        &[Zone::Green, Zone::Edge],
        false,
        Ground::Green,
    ),
    quarter(
        "the_square",
        "the Square",
        1.95,
        &[Zone::Quay, Zone::Lanes],
        false,
        Ground::Plaza,
    ),
    quarter(
        "bakers_lane",
        "Baker's Lane",
        2.2,
        &[],
        true,
        Ground::Garden,
    ),
    quarter(
        "the_green",
        "the Green",
        2.7,
        &[Zone::Green],
        false,
        Ground::Green,
    ),
    quarter(
        "market_row",
        "Market Row",
        1.65,
        &[Zone::Lanes, Zone::Quay],
        true,
        Ground::Plaza,
    ),
    quarter(
        "the_slip",
        "the Slip",
        2.4,
        &[Zone::Water],
        false,
        Ground::Cobbles,
    ),
    quarter(
        "little_green",
        "the Little Green",
        2.95,
        &[Zone::Green, Zone::Edge],
        false,
        Ground::Green,
    ),
    quarter(
        "harbour_front",
        "the Harbour Front",
        3.9,
        &[Zone::Quay, Zone::Lanes, Zone::Water],
        false,
        Ground::Cobbles,
    ),
    quarter(
        "the_point",
        "the Point",
        4.3,
        &[Zone::Water],
        false,
        Ground::Rock,
    ),
    quarter(
        "hill_lane",
        "Hill Lane",
        3.42,
        &[Zone::Lanes],
        true,
        Ground::Garden,
    ),
    quarter(
        "chapel_hill",
        "Chapel Hill",
        3.6,
        &[Zone::Edge],
        false,
        Ground::Yard,
    ),
    quarter(
        "the_orchard",
        "the Orchard",
        3.1,
        &[Zone::Edge, Zone::Green],
        false,
        Ground::Garden,
    ),
    quarter(
        "hill_cottages",
        "Hill Cottages",
        3.3,
        &[Zone::Lanes],
        true,
        Ground::Garden,
    ),
];

/// A cluster, briefly.
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

/// Whether a selection is one of the harbour's homes.
#[cfg(test)]
pub(crate) use days::town::is_home;

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
pub(crate) struct Work {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) shape: MarkShape,
}

/// Every work the harbour can finish, once each: its first goals, the
/// ladder's works (not their second coats of paint), and what it made of
/// its own accord.
pub(crate) fn catalog() -> &'static [Work] {
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
        "sea_wall",
        "boathouse",
        "smokehouse",
        "fish_market",
        "rowing_club",
        "beacon",
        "seal_hide",
        "tide_gauge",
        "boat_yard",
        "lookout_tower",
        "sea_pool",
        "net_loft",
        "herring_shed",
        "wind_break",
        "jetty_ladder",
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
        "bread_cart",
        "bandstand_roof",
        "chapel_windows",
        "workshop",
        "own_notice_board",
        "own_window_boxes",
    ];
    const UP_THE_HILL: &[&str] = &[
        // The lighthouse and its lamp stand on the point, at the east end,
        // with the Harbour Front just west of it by the pub: the clock,
        // the statue, the planters, the tea rooms and the terrace, the
        // lifeboat station and the ferry shelter.
        "lamp",
        "lighthouse_paint",
        "harbour_clock",
        "fishers_statue",
        "quay_planters",
        "lifeboat_station",
        "ferry_shelter",
        "bathing_huts",
        "gull_gate",
        "own_driftwood_bench",
        "tea_rooms",
        "pub_terrace",
        "lantern_walk",
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

/// Which stretch a trade is plied on: boats and nets at the east end by the
/// harbour, bread, music and mending on the square, gardens, bees and
/// lessons at the west end by the school.
fn trade_stretch(job: &str) -> Option<usize> {
    Some(match job {
        // The harbour's trades are plied at its east end, by the harbour
        // and the pub; the hill's at the west, by the school.
        "fisher" | "net_mender" | "boat_builder" | "sailmaker" | "radio_operator" | "carpenter"
        | "mayor" => HILL,
        "shop_assistant" | "musician" | "cook" | "apprentice_baker" | "cheesemaker"
        | "clockmaker" | "bookbinder" | "nurse" => SQUARE,
        "gardener" | "shepherd" | "beekeeper" | "tutor" | "storyteller" | "painter" | "potter"
        | "weaver" | "miller" => QUAY,
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
    let mut town = Town::new(WIDTH, &STRETCHES, days::town::PLOT_ROW_AT).quarters(&QUARTERS);
    let mut items = items;

    // The four buildings first: they have always stood where they stand,
    // the harbour (its lighthouse) at the water's edge.
    for (id, px) in PLACES {
        let row = if id == HARBOR { WATER_TALL } else { BACK };
        town.stand(&mut items, id, |_| row, px, true);
    }
    town.join("harbour_front", SelectionId::Entity(PUB));
    town.join("the_point", SelectionId::Entity(HARBOR));
    town.join("bakers_lane", SelectionId::Entity(BAKERY));
    town.join("net_lane", SelectionId::Entity(SCHOOL));
    // Jonas's boat at the harbour, and the order at the bakery.
    for (id, by) in [(JONAS_BOAT, HARBOR), (WEDDING_ORDER, BAKERY)] {
        let selection = SelectionId::Entity(id);
        let near = town
            .placed
            .get(&SelectionId::Entity(by))
            .copied()
            .unwrap_or(WIDTH / 2.0);
        if let Some(item) = items.iter_mut().find(|item| item.id == selection) {
            if let Some(px) = town
                .street
                .take(FRONT, town.street.stretch_at(near), near + 0.05)
            {
                item.px = Some(px);
                item.y = town.row_y[FRONT];
                town.placed.insert(selection, px);
            }
        }
    }

    // A home for every household.
    let home_of = town.homes(
        &mut items,
        &households(state),
        home_stretch,
        "Home",
        |member| lives::first_name(state, member),
        None,
    );

    // Every work the harbour could finish has a spot kept for it, in the
    // catalog's order, whether it is finished yet or not.
    let done_works = finished(world);
    let done = done_works
        .iter()
        .map(|(index, _, built)| (*index, *built))
        .collect();
    let deck = crate::story::deck();
    let works = catalog()
        .iter()
        .enumerate()
        .map(|(index, work)| CatalogWork {
            id: work.id,
            label: work.label.into(),
            shape: work.shape,
            stretch: work_stretch(work.id, index),
            under_way: storylets::progress(state, deck, work.id) > 0,
        });
    // What the player built on plots stands on its plot first (so no work
    // stands hidden behind it), and everything says what it can wear and
    // be called.
    crate::plots::dress(world, &mut items);
    crate::drawings::dress_art(world, &mut items);
    let mut on_plots: [Vec<SelectionId>; 3] = Default::default();
    town.plotted(&mut items, &mut on_plots);
    let built = town.works(&mut items, works, &done, |id| {
        crate::drawings::art_of_work(id).map(Into::into)
    });
    // Each stretch's places, where they now stand: the school at the west
    // end, the bakery on the square, the pub and the harbour at the east.
    let mut workplaces: [Vec<SelectionId>; 3] = [
        vec![SelectionId::Entity(SCHOOL)],
        vec![SelectionId::Entity(BAKERY)],
        vec![SelectionId::Entity(HARBOR), SelectionId::Entity(PUB)],
    ];
    for (at, works) in built.into_iter().chain(on_plots).enumerate() {
        workplaces[at % 3].extend(works);
    }

    // What the last few days' storylets left behind them.
    items.extend(days::town::traces(
        world.events(),
        world.state().world_time(),
        crate::persistence::WORLD_DAY_TICKS,
        TRACES,
        |person| home_of.get(&person).copied(),
        |work| {
            let index = catalog().iter().position(|each| each.id == work)?;
            let id = days::town::work_id(index);
            town.placed.contains_key(&id).then_some(id)
        },
        |event| crate::projection::narrated_title(world, event),
    ));

    // What answers and the player's hands put up.
    town.fixtures(&mut items, |item| item.kind == CanvasItemKind::Object);

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
        let plan = Plan {
            home,
            work: work_of(state, person, &workplaces)
                .filter(|work| town.placed.contains_key(work))
                .map(|work| (work, false)),
            // Out and about on the square or by the school: the east end
            // has its fishers and the pub already.
            about: [
                SelectionId::Entity(BAKERY),
                square,
                SelectionId::Entity(SCHOOL),
            ][(days::mix(&[person.0, 5]) % 3) as usize],
            evening: Some((SelectionId::Entity(PUB), true)),
            gathering: square,
            festival,
            seed: person.0,
        };
        town.live(item, home, &plan);
    }

    town.projection(
        items,
        Vec::new(),
        LABELS,
        season_of(state),
        crate::plots::canvas_plots(world),
        "harbour",
    )
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

/// How the place stands, as the bars for it measure it.
#[cfg(test)]
pub(crate) type Bars = world_pack_testkit::town::TownBars;

#[cfg(test)]
pub(crate) fn bars(world: &World, snapshot: &world_projection::ProjectionSnapshot) -> Bars {
    let state = world.state();
    let partner = |id: SelectionId| match id {
        SelectionId::Entity(id) => lives::partner(state, id).map(SelectionId::Entity),
        _ => None,
    };
    world_pack_testkit::town::town_bars(
        snapshot,
        &world_pack_testkit::town::TownFacts {
            is_home: &is_home,
            partner: &partner,
            gathering: "square".into(),
            finished: finished(world)
                .iter()
                .map(|(_, work, _)| work.label.to_string())
                .collect(),
            festival: festival_today(state),
        },
    )
}

/// Asserts the bars for the place on a day.
#[cfg(test)]
pub(crate) fn check_bars(day: usize, bars: &Bars) {
    world_pack_testkit::town::check_town_bars(&format!("day {day}"), bars, 0);
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

    use world_pack_testkit::town::outdoors;

    /// The scene a snapshot is laid out as, at noon in a 1100 by 848
    /// window.
    fn stage_of(snapshot: &world_projection::ProjectionSnapshot) -> world_gpui::diorama::Stage {
        world_gpui::diorama::stage_at(snapshot, 1100.0, 848.0, world_gpui::diorama::Clock::at(12))
    }

    /// Every water work (the pier, the boathouse, a slipway, the
    /// lighthouse), going up or finished, stands on the water line.
    fn check_siting(day: usize, snapshot: &world_projection::ProjectionSnapshot) {
        let stage = stage_of(snapshot);
        let catalog_zone = |item: &CanvasItem| {
            catalog()
                .iter()
                .find(|work| work.label == item.label)
                .map(|work| days::town::zone_of(crate::drawings::art_of_work(work.id), work.shape))
        };
        let astray = days::town::water_works_astray(
            &snapshot.canvas.items,
            |index| stage.on_water(index),
            catalog_zone,
        );
        assert!(astray.is_empty(), "day {day}: off the water: {astray:?}");
    }

    /// Water works stand on the water line, never on the hill or the
    /// field, through the harbour's first four months.
    #[test]
    fn water_works_stand_on_the_water_line() {
        play(120, |day, _, snapshot| {
            if day % 10 == 0 {
                check_siting(day, snapshot);
            }
        });
    }

    /// The same over three years, a month at a time (the nightly runs it).
    #[test]
    #[ignore]
    fn water_works_stand_on_the_water_line_for_three_years() {
        play(1_080, |day, _, snapshot| {
            if day % 30 == 0 {
                check_siting(day, snapshot);
            }
        });
    }

    /// No screen-width of the harbour is bare in its first month: there is
    /// always something built within a screen of anywhere.
    #[test]
    fn no_screen_width_is_bare_in_the_first_month() {
        play(30, |day, _, snapshot| {
            let gap = days::town::widest_gap(&snapshot.canvas);
            assert!(gap < 1.0, "day {day}: {gap:.2} screens bare");
        });
    }

    /// Every storylet a mark is kept for is one the harbour tells, so no
    /// mark waits on a storylet that never comes.
    #[test]
    fn every_trace_is_of_a_storylet_the_place_tells() {
        let deck = crate::story::deck();
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

    /// The first screen (where the camera opens, on the welcomer at the
    /// pub) shows something new on at least 8 of a warm player's first 14
    /// days: scaffolding on the chimney, the boats racked for a storm, a
    /// work going up on the Harbour Front.
    #[test]
    fn the_first_screen_changes_on_most_of_the_first_fortnight() {
        let mut before = None;
        let mut society = TinySociety::new().unwrap();
        society.run_story().unwrap();
        let mut branch = society.branch();
        branch.begin_story().unwrap();
        let first = branch.projection_snapshot();
        // The camera opens on whoever welcomes the player, at the pub.
        let screen = |snapshot: &world_projection::ProjectionSnapshot| {
            let stage = stage_of(snapshot);
            let pub_x = snapshot
                .canvas
                .items
                .iter()
                .position(|item| item.id == SelectionId::Entity(PUB))
                .and_then(|index| stage.frame_of(index))
                .map_or(stage.width / 2.0, |(x, _, w, _)| x + w / 2.0);
            stage.screen_around(snapshot, pub_x)
        };
        before.replace(screen(&first));
        let mut changed = Vec::new();
        play(14, |day, _, snapshot| {
            let now = screen(snapshot);
            if before.as_ref() != Some(&now) {
                changed.push(day);
            }
            before = Some(now);
        });
        assert!(changed.len() >= 8, "changed on days {changed:?}");
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
