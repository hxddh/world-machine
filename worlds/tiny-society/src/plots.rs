//! The player's mark on the harbour: plots along the paths where they can
//! build (a boathouse on the quay, a bandstand on the square, a bee garden
//! up the hill), which the harbour finishes; the people what they built
//! draws to live here; designs painted on flags, sails, signs and quilts;
//! and names for boats, works and newborns.
//!
//! The mechanics are the `hands` System's (plots, designs, names) and the
//! `lives` System's (who comes). Where the plots lie is worked out from the
//! harbour's street and never recorded, like the rest of its layout.

use crate::town::{HILL, QUAY, SQUARE, STRETCHES, WIDTH};
use crate::{BAKERY, HARBOR, JONAS_BOAT, PUB, SCHOOL};
use hands::plots::{
    draws, plot_work as work, slot_of, twin, wearing, PlotLook, PlotPack, PlotWork as Work,
};
use hands::{Effect, Kit, Thing};
use std::sync::OnceLock;
use world_core::{ActionRegistry, EntityId, Event, EventId, World, WorldError, WorldState};
use world_projection::{CanvasItem, MarkShape, Wears};

/// Every work a plot can hold, twelve to a stretch.
const WORKS: &[Work] = &[
    // The quay.
    twin(
        draws(
            work("boathouse", "Boathouse", "house", 60, Effect::None, QUAY),
            "boatwright",
            "boat_builder",
        ),
        "boathouse",
    ),
    wearing(
        draws(
            work("sail_loft", "Sail loft", "house", 45, Effect::None, QUAY),
            "sailmaker",
            "sailmaker",
        ),
        Wears::Sail,
    ),
    draws(
        work("crab_shack", "Crab shack", "shop", 40, Effect::Gather, QUAY),
        "cook",
        "cook",
    ),
    draws(
        work("net_store", "Net store", "house", 30, Effect::None, QUAY),
        "net mender",
        "net_mender",
    ),
    draws(
        work("radio_hut", "Radio hut", "house", 40, Effect::None, QUAY),
        "radio operator",
        "radio_operator",
    ),
    wearing(
        work(
            "harbour_flag",
            "Harbour flag",
            "flag",
            15,
            Effect::None,
            QUAY,
        ),
        Wears::Flag,
    ),
    work("slipway", "Slipway", "pier", 35, Effect::None, QUAY),
    work(
        "fishers_shelter",
        "Fishers' shelter",
        "tent",
        25,
        Effect::Rest,
        QUAY,
    ),
    work(
        "harbour_lamp",
        "Harbour lamp",
        "lantern",
        20,
        Effect::Gather,
        QUAY,
    ),
    work("boat_rack", "Boat rack", "stall", 15, Effect::None, QUAY),
    work("ice_house", "Ice house", "house", 35, Effect::None, QUAY),
    work(
        "mooring_bench",
        "Mooring bench",
        "bench",
        15,
        Effect::Rest,
        QUAY,
    ),
    // The square.
    twin(
        draws(
            work("bandstand", "Bandstand", "tent", 60, Effect::Gather, SQUARE),
            "musician",
            "musician",
        ),
        "bandstand",
    ),
    draws(
        work(
            "clock_tower",
            "Clock tower",
            "tower",
            80,
            Effect::None,
            SQUARE,
        ),
        "clockmaker",
        "clockmaker",
    ),
    draws(
        work("bookshop", "Bookshop", "shop", 55, Effect::None, SQUARE),
        "bookbinder",
        "bookbinder",
    ),
    draws(
        work(
            "cheese_shop",
            "Cheese shop",
            "shop",
            50,
            Effect::None,
            SQUARE,
        ),
        "cheesemaker",
        "cheesemaker",
    ),
    draws(
        work("surgery", "Surgery", "house", 70, Effect::None, SQUARE),
        "nurse",
        "nurse",
    ),
    draws(
        work("pottery", "Pottery", "shop", 45, Effect::None, SQUARE),
        "potter",
        "potter",
    ),
    draws(
        work("gallery", "Gallery", "shop", 50, Effect::None, SQUARE),
        "painter",
        "painter",
    ),
    wearing(
        draws(
            work(
                "quilting_room",
                "Quilting room",
                "house",
                40,
                Effect::None,
                SQUARE,
            ),
            "quilter",
            "weaver",
        ),
        Wears::Quilt,
    ),
    draws(
        work(
            "flower_stall",
            "Flower stall",
            "stall",
            20,
            Effect::None,
            SQUARE,
        ),
        "florist",
        "gardener",
    ),
    wearing(
        work(
            "shop_sign",
            "Painted shop sign",
            "signpost",
            10,
            Effect::None,
            SQUARE,
        ),
        Wears::Sign,
    ),
    wearing(
        work(
            "washing_line",
            "Washing line",
            "bunting",
            5,
            Effect::None,
            SQUARE,
        ),
        Wears::Quilt,
    ),
    work(
        "market_cross",
        "Market cross",
        "statue",
        30,
        Effect::Gather,
        SQUARE,
    ),
    // The hill.
    draws(
        work(
            "bee_garden",
            "Bee garden",
            "garden",
            20,
            Effect::Harvest,
            HILL,
        ),
        "beekeeper",
        "beekeeper",
    ),
    draws(
        work("sheepfold", "Sheepfold", "well", 30, Effect::None, HILL),
        "shepherd",
        "shepherd",
    ),
    draws(
        work(
            "allotments",
            "Allotments",
            "garden",
            15,
            Effect::Harvest,
            HILL,
        ),
        "gardener",
        "gardener",
    ),
    draws(
        work(
            "weaving_shed",
            "Weaving shed",
            "house",
            40,
            Effect::None,
            HILL,
        ),
        "weaver",
        "weaver",
    ),
    draws(
        work("study_hut", "Study hut", "house", 35, Effect::None, HILL),
        "tutor",
        "tutor",
    ),
    draws(
        work(
            "story_stone",
            "Storytelling stone",
            "statue",
            20,
            Effect::Gather,
            HILL,
        ),
        "storyteller",
        "storyteller",
    ),
    draws(
        work("joinery", "Joinery", "shop", 50, Effect::None, HILL),
        "carpenter",
        "carpenter",
    ),
    draws(
        work("windmill", "Windmill", "tower", 90, Effect::None, HILL),
        "miller",
        "miller",
    ),
    work(
        "summer_house",
        "Summer house",
        "house",
        45,
        Effect::Rest,
        HILL,
    ),
    work(
        "rose_arbour",
        "Rose arbour",
        "garden",
        15,
        Effect::Rest,
        HILL,
    ),
    work("frog_pond", "Frog pond", "fountain", 20, Effect::None, HILL),
    work(
        "hilltop_swing",
        "Hilltop swing",
        "swing",
        15,
        Effect::Rest,
        HILL,
    ),
];

/// The works as the `hands` System takes them.
pub(crate) fn works() -> &'static [Thing] {
    static THINGS: OnceLock<Vec<Thing>> = OnceLock::new();
    THINGS.get_or_init(|| WORKS.iter().map(|work| work.thing).collect())
}

fn spec(id: &str) -> Option<&'static Work> {
    WORKS.iter().find(|work| work.thing.id == id)
}

pub(crate) use hands::plots::plot_px;
#[cfg(test)]
pub(crate) use hands::plots::PLOT_ROW;

/// How the harbour's plots open: eight as a new harbour begins (some on
/// every stretch), then four more in each year (days after the story
/// began), the last in the third year. A harbour begun before v0.24 keeps every plot open.
pub(crate) const STAGES: hands::PlotStages = hands::PlotStages {
    keeper: crate::story::STORY,
    first: 8,
    at: &[60, 120, 180, 270, 390, 480, 570, 660, 750, 840, 930, 1_020],
    group: stretch_of,
};

/// Which stretch a plot lies on, from where it lies.
fn stretch_of(_: &WorldState, plot: &hands::Plot) -> usize {
    slot_of(&plot.id).map_or(0, |slot| stretch_at(plot_px(slot)))
}

fn stretch_at(px: f32) -> usize {
    STRETCHES
        .iter()
        .position(|stretch| stretch.holds(px))
        .unwrap_or(HILL)
}

/// The harbour's plots, and what could be built on each: everything that
/// belongs on its stretch.
pub(crate) fn plots(state: &WorldState) -> Vec<hands::Plot> {
    hands::plots::plots(&Harbour, state)
}

/// What something can wear a design as, if it can: the flags, sails,
/// signs and quilts the player made, Jonas's boat, and the bakery's and
/// the pub's signs.
pub(crate) fn wears_of(state: &WorldState, id: EntityId) -> Option<Wears> {
    match id {
        JONAS_BOAT => return Some(Wears::Sail),
        BAKERY | PUB => return Some(Wears::Sign),
        _ => {}
    }
    let thing = hands::plots::made_thing(state, id)?;
    match thing {
        "flagpole" => Some(Wears::Flag),
        "rowboat" => Some(Wears::Sail),
        "signpost" => Some(Wears::Sign),
        _ => spec(thing)?.wears,
    }
}

pub(crate) fn wears(state: &WorldState, id: EntityId) -> Option<&'static str> {
    wears_of(state, id).map(Wears::id)
}

/// What something the player can name is: a boat, a work they made, or a
/// baby of the harbour.
pub(crate) fn naming(state: &WorldState, id: EntityId) -> Option<String> {
    if id == JONAS_BOAT {
        return Some("boat".into());
    }
    if crate::kin::stage(state, id) == Some(lives::Stage::Baby)
        && lives::born_here(state, &crate::life::cast()).contains(&id)
    {
        return Some("baby".into());
    }
    let thing = hands::plots::made_thing(state, id)?;
    Some(match thing {
        "rowboat" => "boat".into(),
        _ => match spec(thing) {
            Some(work) => work.thing.name.to_lowercase(),
            None => crate::handwork::thing_name(thing)?.to_lowercase(),
        },
    })
}

const MARK_COMMAND: &str = "tiny-society.mark.";

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

/// Harbour paints the town picks from for what it builds on a plot.
const PAINTS: [[u8; 3]; 8] = [
    [0xf3, 0xee, 0xe1],
    [0x3f, 0x6e, 0x9a],
    [0x5d, 0x8c, 0x74],
    [0xd8, 0xa9, 0x4a],
    [0xc9, 0x6f, 0x6a],
    [0x9e, 0x3b, 0x33],
    [0xe2, 0xcf, 0xae],
    [0x6c, 0x76, 0x80],
];

/// The harbour, as the plots read it.
struct Harbour;

impl PlotPack for Harbour {
    fn works(&self, _: &WorldState) -> &'static [Work] {
        WORKS
    }
    /// Two in every three along the row, so works stand in pairs with a
    /// gap for a path between each pair.
    fn slots(&self, _: &WorldState) -> Vec<usize> {
        (0..hands::plots::plot_slots(WIDTH))
            .filter(|slot| slot % 3 != 2)
            .collect()
    }
    fn width(&self, _: &WorldState) -> f32 {
        WIDTH
    }
    fn stretch_at(&self, _: &WorldState, px: f32) -> usize {
        stretch_at(px)
    }
    fn stretch_id(&self, _: &WorldState, stretch: usize) -> &'static str {
        STRETCHES[stretch].id
    }
    fn place_on(&self, _: &WorldState, stretch: usize, px: f32) -> EntityId {
        match stretch {
            QUAY => HARBOR,
            SQUARE if px < 2.0 => PUB,
            SQUARE => BAKERY,
            _ => SCHOOL,
        }
    }
    /// Whether the harbour built a work of its own accord.
    fn finished_work(&self, state: &WorldState, id: &str) -> bool {
        storylets::finished(state, crate::story::deck(), id)
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
    fn fixture_shape(&self, _: &World, shape: &str) -> MarkShape {
        crate::story::fixture_shape(shape)
    }
    fn art_of(&self, _: &WorldState, what: &str) -> Option<&'static str> {
        crate::drawings::art_of(what)
    }
    fn wears_of(&self, state: &WorldState, id: EntityId) -> Option<Wears> {
        wears_of(state, id)
    }
    fn naming(&self, state: &WorldState, id: EntityId) -> Option<String> {
        naming(state, id)
    }
    fn newborn_word(&self, _: &WorldState, _: EntityId) -> String {
        "baby".into()
    }
    fn cast(&self, _: &WorldState) -> lives::Cast {
        crate::life::cast()
    }
    fn look(&self, _: &WorldState) -> PlotLook<'_> {
        PlotLook {
            paints: &PAINTS,
            salt: 29,
            walls: &[MarkShape::House, MarkShape::Shop, MarkShape::Tower],
            pairs: true,
        }
    }
}

/// The plots as a screen shows them.
pub(crate) fn canvas_plots(world: &World) -> Vec<world_projection::Plot> {
    hands::plots::canvas_plots(&Harbour, world)
}

/// Places the works standing on plots on their plots, and says what each
/// thing on the scene can wear, be named, and how the town built it.
pub(crate) fn dress(world: &World, items: &mut [CanvasItem]) {
    hands::plots::dress(&Harbour, world, items);
}

/// The name cards for a newborn.
pub(crate) fn naming_cards(world: &World) -> Vec<world_projection::ProjectionCommand> {
    hands::plots::naming_cards(&Harbour, world)
}

/// Whether something the player built (or began) waits to draw someone.
pub(crate) fn waiting(state: &WorldState) -> bool {
    hands::plots::waiting(&Harbour, state)
}

/// One day of what the player built drawing people to the harbour.
pub(crate) fn draw(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    hands::plots::draw(&Harbour, world, actions)
}

/// A line told before someone was named, told with the name they were
/// given.
pub(crate) fn renamed(state: &WorldState, event: &Event, line: String) -> String {
    hands::plots::renamed(state, event, line)
}

/// What a work on a plot is called as told: "bandstand".
#[cfg(test)]
pub(crate) fn told_as(state: &WorldState, id: EntityId) -> String {
    match state.entity(id).and_then(|e| e.component("hands.thing")) {
        Some(world_core::Value::Text(what)) => {
            spec(what).map_or_else(String::new, |w| w.thing.name.to_lowercase())
        }
        _ => String::new(),
    }
}
