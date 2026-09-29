//! The player's mark on each place: plots along its paths where they can
//! build (a hydroponics bay under the domes, a diner on Main Street, a song
//! circle in the rookery), which the place finishes; the newcomers what
//! they built draws; designs painted on flags, sails, signs and quilts; and
//! names for vehicles, works and newborns.
//!
//! The mechanics are the `hands` System's (plots, designs, names) and the
//! `lives` System's (who comes). Where the plots lie is worked out from the
//! place's street and never recorded, like the rest of its layout.

use crate::places::Place;
use crate::{SLOT_A, SLOT_C, SLOT_D};
use hands::{Effect, Thing, Verb};
use std::sync::OnceLock;
use world_core::{ActionRegistry, EntityId, Event, EventId, Value, World, WorldError, WorldState};
use world_projection::{
    CanvasItem, CanvasItemKind, Designable, MarkShape, Naming, PlotOffer, SelectionId, Variant,
    Wears,
};

/// A work the player can build on a plot: what it is, which stretch of the
/// place it belongs on, whom it draws to live there (their trade, which is
/// also their job), and what it can wear.
struct Work {
    thing: Thing,
    stretch: usize,
    draws: Option<&'static str>,
    wears: Option<Wears>,
}

const fn work(
    id: &'static str,
    name: &'static str,
    shape: &'static str,
    stretch: usize,
    effect: Effect,
) -> Work {
    Work {
        thing: Thing {
            id,
            name,
            verb: Verb::Build,
            shape,
            cost: 0,
            lasts: None,
            stages: &[],
            effect,
        },
        stretch,
        draws: None,
        wears: None,
    }
}

const fn draws(mut work: Work, trade: &'static str) -> Work {
    work.draws = Some(trade);
    work
}

const fn wearing(mut work: Work, wears: Wears) -> Work {
    work.wears = Some(wears);
    work
}

use Effect::{Gather, Harvest, Rest};
const NONE: Effect = Effect::None;

/// Ares: the domes, the landing pad, the ridge.
const ARES: &[Work] = &[
    draws(
        work("hydroponics_bay", "Hydroponics bay", "dome", 0, Harvest),
        "botanist",
    ),
    draws(work("infirmary", "Infirmary", "dome", 0, NONE), "medic"),
    draws(work("mess_hall", "Mess hall", "house", 0, Gather), "cook"),
    draws(
        work("workshop_dome", "Workshop dome", "dome", 0, NONE),
        "engineer",
    ),
    draws(
        work("music_pod", "Music pod", "dome", 0, Gather),
        "musician",
    ),
    draws(
        work("schoolroom", "Schoolroom", "house", 0, Rest),
        "teacher",
    ),
    wearing(
        work("colony_flag", "Colony flag", "flag", 0, NONE),
        Wears::Flag,
    ),
    wearing(
        work("quilt_line", "Quilt line", "bunting", 0, NONE),
        Wears::Quilt,
    ),
    wearing(
        work("airlock_sign", "Airlock sign", "signpost", 0, NONE),
        Wears::Sign,
    ),
    work("fern_planter", "Fern planter", "planter", 0, NONE),
    draws(
        work("control_tower", "Control tower", "tower", 1, NONE),
        "pilot",
    ),
    draws(
        work("rover_garage", "Rover garage", "house", 1, NONE),
        "mechanic",
    ),
    draws(
        work("radio_dish", "Radio dish", "tower", 1, NONE),
        "radio operator",
    ),
    draws(
        work("supply_shop", "Supply shop", "shop", 1, NONE),
        "quartermaster",
    ),
    work("cargo_depot", "Cargo depot", "stall", 1, NONE),
    work("landing_lights", "Landing lights", "lantern", 1, Gather),
    work("windsock", "Windsock", "flag", 1, NONE),
    work("fuel_tanks", "Fuel tanks", "well", 1, NONE),
    work("dust_shelter", "Dust shelter", "tent", 1, Rest),
    work("solar_array", "Solar array", "stall", 1, NONE),
    draws(
        work("survey_station", "Survey station", "tower", 2, NONE),
        "geologist",
    ),
    draws(work("ice_drill", "Ice drill", "well", 2, NONE), "driller"),
    draws(
        work("observatory", "Observatory", "dome", 2, NONE),
        "astronomer",
    ),
    work("crater_bench", "Crater bench", "bench", 2, Rest),
    work("marker_cairn", "Marker cairn", "statue", 2, NONE),
    work("greenhouse_tent", "Greenhouse tent", "tent", 2, Harvest),
    work("weather_mast", "Weather mast", "tower", 2, NONE),
    work("lichen_garden", "Lichen garden", "garden", 2, Harvest),
    work("low_swing", "Low-gravity swing", "swing", 2, Rest),
    work("ridge_beacon", "Ridge beacon", "lantern", 2, Gather),
];

/// Maple Street: Main Street, the square, the school and the lake.
const MAPLE: &[Work] = &[
    draws(
        work("record_store", "Record store", "shop", 0, NONE),
        "record collector",
    ),
    draws(work("diner", "Diner", "shop", 0, Gather), "diner cook"),
    draws(work("garage", "Garage", "house", 0, NONE), "mechanic"),
    draws(work("arcade", "Arcade", "shop", 0, Gather), "arcade owner"),
    draws(
        work("barber_shop", "Barber shop", "shop", 0, NONE),
        "barber",
    ),
    draws(
        work("video_store", "Video store", "shop", 0, NONE),
        "video clerk",
    ),
    wearing(
        work("shop_sign", "Painted shop sign", "signpost", 0, NONE),
        Wears::Sign,
    ),
    work("newsstand", "Newsstand", "stall", 0, NONE),
    work("bus_shelter", "Bus shelter", "tent", 0, Rest),
    work("neon_lamp", "Neon lamp", "lantern", 0, Gather),
    draws(
        work("bandstand", "Bandstand", "tent", 1, Gather),
        "guitarist",
    ),
    draws(
        work("ice_cream_stand", "Ice cream stand", "stall", 1, NONE),
        "ice cream seller",
    ),
    draws(
        work("town_clock", "Town clock", "tower", 1, NONE),
        "clockmaker",
    ),
    draws(
        work("flower_beds", "Flower beds", "garden", 1, Harvest),
        "gardener",
    ),
    wearing(
        draws(work("quilt_shop", "Quilt shop", "shop", 1, NONE), "quilter"),
        Wears::Quilt,
    ),
    wearing(work("town_flag", "Town flag", "flag", 1, NONE), Wears::Flag),
    work("wishing_fountain", "Wishing fountain", "fountain", 1, NONE),
    work("gazebo", "Gazebo", "tent", 1, Rest),
    work("chess_tables", "Chess tables", "bench", 1, Gather),
    work("bike_rack", "Bike rack", "stall", 1, NONE),
    draws(work("library", "Library", "house", 2, NONE), "librarian"),
    draws(work("gym_hall", "Gym hall", "house", 2, NONE), "coach"),
    draws(
        work("boathouse", "Boathouse", "house", 2, NONE),
        "boatwright",
    ),
    draws(
        work("bee_garden", "Bee garden", "garden", 2, Harvest),
        "beekeeper",
    ),
    draws(
        work("science_shed", "Science shed", "house", 2, NONE),
        "science teacher",
    ),
    wearing(work("sailboat", "Sailboat", "boat", 2, NONE), Wears::Sail),
    work("treehouse", "Treehouse", "tree", 2, Rest),
    work("bleachers", "Bleachers", "bench", 2, Gather),
    work("fishing_dock", "Fishing dock", "pier", 2, NONE),
    work("rope_swing", "Rope swing", "swing", 2, Rest),
];

/// Icebridge: the rookery, the bridge, the far floe.
const ICE: &[Work] = &[
    draws(
        work("chick_nursery", "Chick nursery", "dome", 0, NONE),
        "nanny",
    ),
    draws(
        work("song_circle", "Song circle", "statue", 0, Gather),
        "singer",
    ),
    draws(
        work("fish_larder", "Fish larder", "well", 0, NONE),
        "fisher",
    ),
    draws(
        work("lantern_ring", "Lantern ring", "lantern", 0, Gather),
        "lantern tender",
    ),
    draws(
        work("story_berg", "Storytelling berg", "statue", 0, NONE),
        "storyteller",
    ),
    wearing(
        work("rookery_flag", "Rookery flag", "flag", 0, NONE),
        Wears::Flag,
    ),
    wearing(
        work("feather_quilt", "Feather quilt", "bunting", 0, NONE),
        Wears::Quilt,
    ),
    work("pebble_garden", "Pebble garden", "garden", 0, NONE),
    work("snow_house", "Snow house", "dome", 0, Rest),
    work("ice_slide", "Ice slide", "swing", 0, Rest),
    draws(work("ice_market", "Ice market", "stall", 1, NONE), "trader"),
    draws(
        work("carving_hall", "Carving hall", "dome", 1, NONE),
        "ice carver",
    ),
    draws(
        work("lookout_post", "Lookout post", "tower", 1, NONE),
        "scout",
    ),
    wearing(
        work("bridge_sign", "Bridge sign", "signpost", 1, NONE),
        Wears::Sign,
    ),
    work("skating_rink", "Skating rink", "fountain", 1, Gather),
    work("kelp_racks", "Kelp racks", "stall", 1, Harvest),
    work("ice_bench", "Ice bench", "bench", 1, Rest),
    work("bell_post", "Bell post", "tower", 1, NONE),
    work("snow_arch", "Snow arch", "statue", 1, NONE),
    work("warming_hut", "Warming hut", "house", 1, Rest),
    draws(
        work("kayak_shelter", "Kayak shelter", "house", 2, NONE),
        "boatbuilder",
    ),
    draws(
        work("whale_watch", "Whale watch", "tower", 2, NONE),
        "whale watcher",
    ),
    draws(
        work(
            "sculpture_garden",
            "Ice sculpture garden",
            "statue",
            2,
            NONE,
        ),
        "sculptor",
    ),
    wearing(work("sail_sled", "Sail sled", "boat", 2, NONE), Wears::Sail),
    work("aurora_seat", "Aurora seat", "bench", 2, Rest),
    work("fishing_hole", "Fishing hole", "well", 2, Harvest),
    work("moss_patch", "Moss patch", "garden", 2, Harvest),
    work("seal_fence", "Seal fence", "pier", 2, NONE),
    work("far_beacon", "Far beacon", "lantern", 2, Gather),
    work("snow_maze", "Snow maze", "planter", 2, NONE),
];

fn list(place: Place) -> &'static [Work] {
    match place {
        Place::Ares => ARES,
        Place::Maple => MAPLE,
        Place::Ice => ICE,
    }
}

/// Every place's works, as the `hands` System takes them.
pub(crate) fn works(state: &WorldState) -> &'static [Thing] {
    static THINGS: OnceLock<[Vec<Thing>; 3]> = OnceLock::new();
    let all = THINGS.get_or_init(|| {
        [Place::Ares, Place::Maple, Place::Ice]
            .map(|place| list(place).iter().map(|work| work.thing).collect())
    });
    match Place::of(state) {
        Some(place) => &all[place.index()],
        None => &[],
    }
}

fn spec(state: &WorldState, id: &str) -> Option<&'static Work> {
    list(Place::of(state)?)
        .iter()
        .find(|work| work.thing.id == id)
}

/// The row plots lie in, behind the front row, and where it lies.
pub(crate) const PLOT_ROW: usize = 4;
pub(crate) const PLOT_Y: f32 = 0.68;
pub(crate) const PLOT_OFFSET: f32 = 0.02;
pub(crate) const PLOT_PITCH: f32 = 0.16;
/// The most a plot offers at once: everything that belongs on its stretch.
const OFFERED: usize = 12;

/// Where a plot slot lies along the place.
pub(crate) fn plot_px(slot: usize) -> f32 {
    PLOT_OFFSET + (slot as f32 + 0.5) * PLOT_PITCH
}

fn slots(place: Place) -> impl Iterator<Item = usize> {
    let width = crate::town::width(place);
    0..((width - PLOT_OFFSET) / PLOT_PITCH).floor() as usize
}

fn plot_id(slot: usize) -> String {
    format!("p{slot}")
}

fn slot_of(id: &str) -> Option<usize> {
    id.strip_prefix('p')?.parse().ok()
}

/// The place a plot on a stretch is told as lying by.
fn place_on(stretch: usize) -> EntityId {
    if stretch == 0 {
        SLOT_A
    } else {
        SLOT_C
    }
}

/// The place's plots, and what could be built on each: everything that
/// belongs on its stretch.
pub(crate) fn plots(state: &WorldState) -> Vec<hands::Plot> {
    let Some(place) = Place::of(state) else {
        return Vec::new();
    };
    slots(place)
        .map(|slot| {
            let stretch = crate::town::stretch_at(place, plot_px(slot));
            hands::Plot {
                id: plot_id(slot),
                at: place_on(stretch),
                offers: list(place)
                    .iter()
                    .filter(|work| work.stretch == stretch)
                    .map(|work| work.thing.id)
                    .collect(),
            }
        })
        .collect()
}

/// What something can wear a design as, if it can.
pub(crate) fn wears_of(state: &WorldState, id: EntityId) -> Option<Wears> {
    if !hands::made(state).contains(&id) {
        return None;
    }
    let thing = match state.entity(id)?.component("hands.thing")? {
        Value::Text(thing) => thing.as_str(),
        _ => return None,
    };
    match thing {
        "flag" => Some(Wears::Flag),
        "rowboat" | "kayak" => Some(Wears::Sail),
        "sign" => Some(Wears::Sign),
        _ => spec(state, thing)?.wears,
    }
}

pub(crate) fn wears(state: &WorldState, id: EntityId) -> Option<&'static str> {
    wears_of(state, id).map(Wears::id)
}

/// What something the player can name is: the rover or the bus, a work
/// they made, or a baby born here.
pub(crate) fn naming(state: &WorldState, id: EntityId) -> Option<String> {
    let entity = state.entity(id)?;
    if id == SLOT_D && matches!(entity.kind.as_str(), "rover" | "bus") {
        return Some(entity.kind.clone());
    }
    let cast = crate::life::cast(state);
    if lives::born_here(state, &cast).contains(&id) {
        let (stage, _, _) = lives::looks_of(state, &cast, id)?;
        let penguin = entity.kind == "penguin";
        return (stage == lives::Stage::Baby)
            .then(|| if penguin { "chick" } else { "baby" }.into());
    }
    if !hands::made(state).contains(&id) {
        return None;
    }
    let thing = match entity.component("hands.thing")? {
        Value::Text(thing) => thing.as_str(),
        _ => return None,
    };
    match spec(state, thing) {
        Some(work) => Some(work.thing.name.to_lowercase()),
        None => crate::handwork::thing_name(state, thing).map(str::to_lowercase),
    }
}

const MARK_COMMAND: &str = "pocket-universe.mark.";

/// A design or naming command's target, what it does and its argument.
pub(crate) fn parse_command(command_id: &str) -> Option<(&str, EntityId, &str)> {
    let (command, argument) = world_projection::command_argument(command_id);
    let rest = command.strip_prefix(MARK_COMMAND)?;
    let (what, target) = rest.split_once('.')?;
    let target = EntityId::new(target.parse().ok()?);
    matches!(what, "design" | "name").then_some((what, target, argument?))
}

fn command(what: &str, target: EntityId) -> String {
    format!("{MARK_COMMAND}{what}.{}", target.0)
}

/// The request a design or naming command makes.
pub(crate) fn request(command_id: &str) -> Option<world_core::ActionRequest> {
    let (what, target, argument) = parse_command(command_id)?;
    Some(match what {
        "design" => hands::design_request(target, argument),
        _ => hands::name_request(target, argument),
    })
}

/// The plots as a screen shows them, with what could stand on each,
/// starting from a different one on each plot.
pub(crate) fn canvas_plots(world: &World) -> Vec<world_projection::Plot> {
    let state = world.state();
    let Some(place) = Place::of(state) else {
        return Vec::new();
    };
    let kit = crate::handwork::kit(state);
    let stretches = crate::town::stretch_ids(place);
    hands::plot_deeds(world, &kit)
        .into_iter()
        .filter_map(|(plot, mut deeds)| {
            let slot = slot_of(&plot.id)?;
            let px = plot_px(slot);
            let turn = if deeds.is_empty() {
                0
            } else {
                slot % deeds.len()
            };
            deeds.rotate_left(turn);
            Some(world_projection::Plot {
                id: plot.id.clone(),
                px,
                row: PLOT_ROW as u8,
                district: stretches[crate::town::stretch_at(place, px)].into(),
                offers: deeds
                    .into_iter()
                    .take(OFFERED)
                    .map(|deed| {
                        let what = hands::plot_work(&deed.key).unwrap_or_default();
                        PlotOffer {
                            command: crate::handwork::command_id(&deed.key),
                            label: deed.thing,
                            shape: spec(state, what).map_or(MarkShape::House, |work| {
                                crate::story::fixture_shape(world, work.thing.shape)
                            }),
                            cost: deed.cost,
                            unavailable: deed.unavailable,
                        }
                    })
                    .collect(),
            })
        })
        .filter(|plot| !plot.offers.is_empty())
        .collect()
}

/// The paints each place picks from for what it builds on a plot.
fn paints(place: Place) -> [[u8; 3]; 6] {
    match place {
        Place::Ares => [
            [0xe9, 0xe4, 0xda],
            [0xc4, 0x6a, 0x3f],
            [0x8b, 0x96, 0xa3],
            [0xd9, 0xa4, 0x41],
            [0x5d, 0x7d, 0x8c],
            [0xb3, 0x4b, 0x3a],
        ],
        Place::Maple => [
            [0xf1, 0xe6, 0xcf],
            [0x7a, 0x9e, 0x7e],
            [0xc0, 0x5a, 0x4b],
            [0x4e, 0x6c, 0x9c],
            [0xe0, 0xb3, 0x4f],
            [0x9b, 0x6a, 0x9e],
        ],
        Place::Ice => [
            [0xf4, 0xf8, 0xfb],
            [0xa9, 0xd3, 0xe8],
            [0x6f, 0x9f, 0xc2],
            [0xd6, 0xe6, 0xef],
            [0x87, 0xb4, 0xa6],
            [0xe8, 0xc8, 0x7a],
        ],
    }
}

/// Places the works standing on plots on their plots, and says what each
/// thing on the scene can wear, be named, and how the place built it.
pub(crate) fn dress(world: &World, items: &mut [CanvasItem]) {
    let state = world.state();
    let kit = crate::handwork::kit(state);
    let Some(place) = Place::of(state) else {
        return;
    };
    let width = crate::town::width(place);
    let on_plots = hands::on_plots(state);
    let built_on = |slot: usize| on_plots.iter().any(|(id, _)| slot_of(id) == Some(slot));
    let cast = crate::life::cast(state);
    let newborns = lives::to_be_named(state, &cast, |child| hands::named(state, child));
    let paints = paints(place);
    for item in items.iter_mut() {
        let SelectionId::Entity(id) = item.id else {
            continue;
        };
        if let Some(wears) = wears_of(state, id) {
            item.design = Some(Designable {
                command: command("design", id),
                wears,
            });
            item.pattern = hands::pattern_of(state, id)
                .and_then(|text| world_projection::Design::parse(text).ok())
                .map(|design| design.pattern());
        }
        // Something the player made says so, never what it is kept as.
        if item.detail.is_empty() && hands::made(state).contains(&id) {
            item.detail = if hands::plot_of(state, id).is_some() {
                "Built by you"
            } else {
                "Made by you"
            }
            .into();
        }
        if naming(state, id).is_some() {
            item.naming = Some(Naming {
                command: command("name", id),
                proposals: if newborns.contains(&id) {
                    lives::name_proposals(state, &cast, id, 3)
                } else {
                    Vec::new()
                },
            });
        }
        let Some(slot) = hands::plot_of(state, id).and_then(slot_of) else {
            continue;
        };
        let px = plot_px(slot);
        item.px = Some(px);
        item.x = px / width;
        item.y = PLOT_Y;
        let what = match state.entity(id).and_then(|e| e.component("hands.thing")) {
            Some(Value::Text(what)) => what.len() as u64,
            _ => 0,
        };
        let seed = days::mix(&[slot as u64, what, 31]);
        let wall = matches!(
            item.shape,
            Some(MarkShape::House | MarkShape::Shop | MarkShape::Tower | MarkShape::Dome)
        );
        item.kind = if wall {
            CanvasItemKind::Place
        } else {
            CanvasItemKind::Object
        };
        // Neighbours on plots side by side are joined: a wall run on, or
        // a path between.
        item.variant = Some(Variant {
            colour: paints[(seed % paints.len() as u64) as usize],
            flip: (seed >> 7) % 2 == 1,
            join_left: slot > 0 && built_on(slot - 1),
            join_right: built_on(slot + 1),
        });
        if hands::building(state, id) {
            item.detail = "Being built".into();
            item.built = None;
        } else if let Some(finished) = hands::finished_at(state, &kit, id) {
            item.built = Some(finished as u32 + 1);
        }
    }
}

/// The name cards: the parents of a baby born these last days ask the
/// player to choose a name, from three they propose.
pub(crate) fn naming_cards(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let state = world.state();
    if Place::of(state).is_none() {
        return Vec::new();
    }
    let cast = crate::life::cast(state);
    lives::to_be_named(state, &cast, |child| hands::named(state, child))
        .into_iter()
        .flat_map(|child| {
            let parents = lives::generations::parents(state, child);
            let what = naming(state, child).unwrap_or_else(|| "baby".into());
            let prompt = match parents.as_slice() {
                [a, b, ..] => format!(
                    "{} and {} would like you to choose the {what}'s name",
                    lives::first_name(state, *a),
                    lives::first_name(state, *b)
                ),
                [a] => format!(
                    "{} would like you to choose the {what}'s name",
                    lives::first_name(state, *a)
                ),
                [] => format!("Choose the {what}'s name"),
            };
            let question = world_projection::Question {
                id: format!("name-{}", child.0),
                prompt,
            };
            let asker = parents.first().copied().map(SelectionId::Entity);
            let detail = format!("Call the {what} this");
            lives::name_proposals(state, &cast, child, 3)
                .into_iter()
                .map(move |name| world_projection::ProjectionCommand {
                    id: world_projection::command_with(&command("name", child), &name),
                    title: name,
                    detail: detail.clone(),
                    effects: Vec::new(),
                    scenery: None,
                    moves: Vec::new(),
                    asker,
                    question: Some(question.clone()),
                    unavailable: None,
                    hand: None,
                    preview: None,
                })
        })
        .collect()
}

/// Whether something the player built (or began) waits to draw someone.
pub(crate) fn waiting(state: &WorldState) -> bool {
    hands::on_plots(state).into_iter().any(|(_, id)| {
        let Some(entity) = state.entity(id) else {
            return false;
        };
        let draws = match entity.component("hands.thing") {
            Some(Value::Text(what)) => spec(state, what).is_some_and(|work| work.draws.is_some()),
            _ => false,
        };
        draws && lives::drew(state, id).is_none()
    })
}

/// One period of what the player built drawing people to the place.
pub(crate) fn draw(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    let state = world.state();
    let kit = crate::handwork::kit(state);
    let draws = hands::on_plots(state)
        .into_iter()
        .filter(|(_, id)| lives::drew(state, *id).is_none())
        .filter_map(|(_, id)| {
            let entity = state.entity(id)?;
            let what = match entity.component("hands.thing")? {
                Value::Text(what) => spec(state, what)?,
                _ => return None,
            };
            let trade = what.draws?;
            let since = hands::finished_at(state, &kit, id)?;
            Some(lives::Draw {
                by: id,
                what: what.thing.name.to_lowercase(),
                trade,
                job: trade,
                since,
                cause: None,
            })
        })
        .collect::<Vec<_>>();
    if draws.is_empty() {
        return Ok(Vec::new());
    }
    let finished = world.events_of_kind(&["plot_finished"]);
    let draws = draws
        .into_iter()
        .map(|mut draw| {
            draw.cause = finished
                .iter()
                .rev()
                .find(|event| event.targets.first() == Some(&draw.by))
                .map(|event| event.id);
            draw
        })
        .collect::<Vec<_>>();
    // The room kept for whoever it draws is theirs.
    let mut cast = crate::life::cast(world.state());
    cast.most_people = crate::life::most_people(world.state());
    lives::draw_newcomers(world, actions, &cast, &draws)
}

/// A line told before someone was named, told with the name they were
/// given: a baby's birth, told with the name the player chose.
pub(crate) fn renamed(state: &WorldState, event: &Event, line: String) -> String {
    if event.kind != "born" {
        return line;
    }
    let Some(Value::Entity(child)) = event.payload.get("who") else {
        return line;
    };
    let (Some(was), true) = (
        hands::was_called(state, *child),
        hands::named(state, *child),
    ) else {
        return line;
    };
    let now = lives::name(state, *child);
    let mut out = String::new();
    let mut rest = line.as_str();
    while let Some(at) = rest.find(was) {
        let before = rest[..at].chars().last();
        let after = rest[at + was.len()..].chars().next();
        let whole =
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric);
        out.push_str(&rest[..at]);
        out.push_str(if whole { &now } else { was });
        rest = &rest[at + was.len()..];
    }
    out.push_str(rest);
    out
}
