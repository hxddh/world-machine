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
use hands::plots::{plot_px, slot_of, wearing, PlotLook, PlotPack, PlotWork as Work};
use hands::{Effect, Kit, Thing};
use std::sync::OnceLock;
use world_core::{ActionRegistry, EntityId, Event, EventId, World, WorldError, WorldState};
use world_projection::{CanvasItem, MarkShape, Wears};

/// A work built on a plot of a stretch.
const fn work(
    id: &'static str,
    name: &'static str,
    shape: &'static str,
    stretch: usize,
    effect: Effect,
) -> Work {
    hands::plots::plot_work(id, name, shape, 0, effect, stretch)
}

/// A work that draws someone of a trade, which is also their job.
const fn draws(work: Work, trade: &'static str) -> Work {
    hands::plots::draws(work, trade, trade)
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

/// How a place's plots open: five as its story begins (some on every
/// stretch), then four more in each year (periods after the story
/// began), until all are open. A place
/// begun before v0.24 keeps every plot open.
pub(crate) const STAGES: hands::PlotStages = hands::PlotStages {
    keeper: crate::story::STORY,
    first: 5,
    at: &[60, 120, 180, 270, 390, 480, 570, 660, 750, 840, 930, 1_020],
    group: stretch_of,
};

/// Which stretch of the place a plot lies on, from where it lies.
fn stretch_of(state: &WorldState, plot: &hands::Plot) -> usize {
    match (Place::of(state), slot_of(&plot.id)) {
        (Some(place), Some(slot)) => crate::town::stretch_at(place, plot_px(slot)),
        _ => 0,
    }
}

/// The place's plots, and what could be built on each: everything that
/// belongs on its stretch.
pub(crate) fn plots(state: &WorldState) -> Vec<hands::Plot> {
    hands::plots::plots(&Here, state)
}

/// What something can wear a design as, if it can.
pub(crate) fn wears_of(state: &WorldState, id: EntityId) -> Option<Wears> {
    let thing = hands::plots::made_thing(state, id)?;
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
    let thing = hands::plots::made_thing(state, id)?;
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

/// The request a typed design or name makes (protocol v8): `target` is
/// the command the World offered, `what` is `design` or `name`, and
/// `argument` the design's text or the name. Nothing is parsed out of the
/// argument, which may hold any character.
pub(crate) fn typed_request(
    what: &str,
    target: &str,
    argument: &str,
) -> Option<world_core::ActionRequest> {
    let rest = target.strip_prefix(MARK_COMMAND)?;
    let (kind, id) = rest.split_once('.')?;
    let id = EntityId::new(id.parse().ok()?);
    (kind == what).then(|| match what {
        "design" => hands::design_request(id, argument),
        _ => hands::name_request(id, argument),
    })
}

/// The request a design or naming command makes.
pub(crate) fn request(command_id: &str) -> Option<world_core::ActionRequest> {
    let (what, target, argument) = parse_command(command_id)?;
    Some(match what {
        "design" => hands::design_request(target, argument),
        _ => hands::name_request(target, argument),
    })
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

/// The place a World is in, once it has begun.
fn place(state: &WorldState) -> Place {
    Place::of(state).expect("a place, once its plots are asked for")
}

/// Each place, as the plots read it.
struct Here;

impl PlotPack for Here {
    fn ready(&self, state: &WorldState) -> bool {
        Place::of(state).is_some()
    }
    fn works(&self, state: &WorldState) -> &'static [Work] {
        Place::of(state).map_or(&[], list)
    }
    fn slots(&self, state: &WorldState) -> Vec<usize> {
        match Place::of(state) {
            Some(place) => (0..hands::plots::plot_slots(crate::town::width(place))).collect(),
            None => Vec::new(),
        }
    }
    fn width(&self, state: &WorldState) -> f32 {
        crate::town::width(place(state))
    }
    fn stretch_at(&self, state: &WorldState, px: f32) -> usize {
        crate::town::stretch_at(place(state), px)
    }
    fn stretch_id(&self, state: &WorldState, stretch: usize) -> &'static str {
        crate::town::stretch_ids(place(state))[stretch]
    }
    /// The place a plot on a stretch is told as lying by.
    fn place_on(&self, _: &WorldState, stretch: usize, _: f32) -> EntityId {
        if stretch == 0 {
            SLOT_A
        } else {
            SLOT_C
        }
    }
    fn kit(&self, state: &WorldState) -> Kit {
        crate::handwork::kit(state)
    }
    fn deed_command(&self, key: &str) -> String {
        crate::handwork::command_id(key)
    }
    fn command(&self, what: &str, target: EntityId) -> String {
        command(what, target)
    }
    fn fixture_shape(&self, world: &World, shape: &str) -> MarkShape {
        crate::story::fixture_shape(world, shape)
    }
    fn art_of(&self, state: &WorldState, what: &str) -> Option<&'static str> {
        crate::drawings::art_of(place(state), what)
    }
    fn wears_of(&self, state: &WorldState, id: EntityId) -> Option<Wears> {
        wears_of(state, id)
    }
    fn naming(&self, state: &WorldState, id: EntityId) -> Option<String> {
        naming(state, id)
    }
    fn newborn_word(&self, state: &WorldState, child: EntityId) -> String {
        naming(state, child).unwrap_or_else(|| "baby".into())
    }
    fn cast(&self, state: &WorldState) -> lives::Cast {
        crate::life::cast(state)
    }
    /// The room kept for whoever a work draws is theirs.
    fn newcomer_cast(&self, state: &WorldState) -> lives::Cast {
        let mut cast = crate::life::cast(state);
        cast.most_people = crate::life::most_people(state);
        cast
    }
    fn look(&self, state: &WorldState) -> PlotLook<'_> {
        static PAINTS: OnceLock<[[[u8; 3]; 6]; 3]> = OnceLock::new();
        let paints = PAINTS.get_or_init(|| [Place::Ares, Place::Maple, Place::Ice].map(paints));
        PlotLook {
            paints: &paints[place(state).index()],
            salt: 31,
            walls: &[
                MarkShape::House,
                MarkShape::Shop,
                MarkShape::Tower,
                MarkShape::Dome,
            ],
            pairs: false,
        }
    }
}

/// The plots as a screen shows them.
pub(crate) fn canvas_plots(world: &World) -> Vec<world_projection::Plot> {
    hands::plots::canvas_plots(&Here, world)
}

/// Places the works standing on plots on their plots, and says what each
/// thing on the scene can wear, be named, and how the place built it.
pub(crate) fn dress(world: &World, items: &mut [CanvasItem]) {
    hands::plots::dress(&Here, world, items);
}

/// The name cards for a newborn.
pub(crate) fn naming_cards(world: &World) -> Vec<world_projection::ProjectionCommand> {
    hands::plots::naming_cards(&Here, world)
}

/// Whether something the player built (or began) waits to draw someone.
pub(crate) fn waiting(state: &WorldState) -> bool {
    hands::plots::waiting(&Here, state)
}

/// One period of what the player built drawing people to the place.
pub(crate) fn draw(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    hands::plots::draw(&Here, world, actions)
}

/// A line told before someone was named, told with the name they were
/// given.
pub(crate) fn renamed(state: &WorldState, event: &Event, line: String) -> String {
    hands::plots::renamed(state, event, line)
}
