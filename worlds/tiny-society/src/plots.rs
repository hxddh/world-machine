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
use hands::{Effect, Thing, Verb};
use std::sync::OnceLock;
use world_core::{ActionRegistry, EntityId, Event, EventId, Value, World, WorldError, WorldState};
use world_projection::{
    CanvasItem, Designable, MarkShape, Naming, PlotOffer, SelectionId, Variant, Wears,
};

/// A work the player can build on a plot: what it is, which stretch of the
/// harbour it belongs on, whom it draws to live here (the trade as told,
/// and the job), what it can wear, and the story's work that stands for it
/// if the harbour builds that one of its own accord.
struct Work {
    thing: Thing,
    stretch: usize,
    draws: Option<(&'static str, &'static str)>,
    wears: Option<Wears>,
    twin: Option<&'static str>,
}

const fn work(
    id: &'static str,
    name: &'static str,
    shape: &'static str,
    cost: i64,
    effect: Effect,
    stretch: usize,
) -> Work {
    Work {
        thing: Thing {
            id,
            name,
            verb: Verb::Build,
            shape,
            cost,
            lasts: None,
            stages: &[],
            effect,
        },
        stretch,
        draws: None,
        wears: None,
        twin: None,
    }
}

const fn draws(mut work: Work, trade: &'static str, job: &'static str) -> Work {
    work.draws = Some((trade, job));
    work
}

const fn wearing(mut work: Work, wears: Wears) -> Work {
    work.wears = Some(wears);
    work
}

const fn twin(mut work: Work, twin: &'static str) -> Work {
    work.twin = Some(twin);
    work
}

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

/// The row plots lie in, behind the front row: no other row's spots are
/// within a hand's breadth of it.
pub(crate) const PLOT_ROW: usize = 4;
/// How far down the scene the plot row lies, for an app that knows one
/// screen only.
pub(crate) const PLOT_Y: f32 = 0.68;
/// Where the first plot's row begins, and how far apart its slots are.
pub(crate) const PLOT_OFFSET: f32 = 0.02;
pub(crate) const PLOT_PITCH: f32 = 0.16;
/// The most a plot offers at once: everything that belongs on its stretch.
const OFFERED: usize = 12;

/// The plot slots: two in every three along the row, so works stand in
/// pairs with a gap for a path between each pair.
fn slots() -> impl Iterator<Item = usize> {
    let count = ((WIDTH - PLOT_OFFSET) / PLOT_PITCH).floor() as usize;
    (0..count).filter(|slot| slot % 3 != 2)
}

/// Where a plot slot lies along the harbour.
pub(crate) fn plot_px(slot: usize) -> f32 {
    PLOT_OFFSET + (slot as f32 + 0.5) * PLOT_PITCH
}

fn stretch_at(px: f32) -> usize {
    STRETCHES
        .iter()
        .position(|stretch| stretch.holds(px))
        .unwrap_or(HILL)
}

/// The place a plot is told as lying by.
fn place_on(stretch: usize, px: f32) -> EntityId {
    match stretch {
        QUAY => HARBOR,
        SQUARE if px < 2.0 => PUB,
        SQUARE => BAKERY,
        _ => SCHOOL,
    }
}

fn plot_id(slot: usize) -> String {
    format!("p{slot}")
}

fn slot_of(id: &str) -> Option<usize> {
    id.strip_prefix('p')?.parse().ok()
}

/// Whether the harbour built a work's twin of its own accord, so another
/// would be one too many.
fn twin_built(state: &WorldState, work: &Work) -> bool {
    work.twin
        .is_some_and(|twin| storylets::finished(state, crate::story::deck(), twin))
}

/// The harbour's plots, and what could be built on each: everything that
/// belongs on its stretch.
pub(crate) fn plots(state: &WorldState) -> Vec<hands::Plot> {
    slots()
        .map(|slot| {
            let px = plot_px(slot);
            let stretch = stretch_at(px);
            hands::Plot {
                id: plot_id(slot),
                at: place_on(stretch, px),
                offers: WORKS
                    .iter()
                    .filter(|work| work.stretch == stretch && !twin_built(state, work))
                    .map(|work| work.thing.id)
                    .collect(),
            }
        })
        .collect()
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
    if !hands::made(state).contains(&id) {
        return None;
    }
    let thing = match state.entity(id)?.component("hands.thing")? {
        Value::Text(thing) => thing.as_str(),
        _ => return None,
    };
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
    if !hands::made(state).contains(&id) {
        return None;
    }
    let thing = match state.entity(id)?.component("hands.thing")? {
        Value::Text(thing) => thing.as_str(),
        _ => return None,
    };
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
    let kit = crate::handwork::kit(world.state());
    hands::plot_deeds(world, &kit)
        .into_iter()
        .filter_map(|(plot, deeds)| {
            let slot = slot_of(&plot.id)?;
            let px = plot_px(slot);
            let turn = if deeds.is_empty() {
                0
            } else {
                slot % deeds.len()
            };
            let mut deeds = deeds;
            deeds.rotate_left(turn);
            Some(world_projection::Plot {
                id: plot.id.clone(),
                px,
                row: PLOT_ROW as u8,
                district: STRETCHES[stretch_at(px)].id.into(),
                offers: deeds
                    .into_iter()
                    .take(OFFERED)
                    .map(|deed| {
                        let what = hands::plot_work(&deed.key).unwrap_or_default();
                        PlotOffer {
                            command: crate::handwork::command_id(&deed.key),
                            label: deed.thing,
                            shape: spec(what).map_or(MarkShape::House, |work| {
                                crate::story::fixture_shape(work.thing.shape)
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

/// Places the works standing on plots on their plots, and says what each
/// thing on the scene can wear, be named, and how the town built it.
pub(crate) fn dress(world: &World, items: &mut [CanvasItem]) {
    let state = world.state();
    let kit = crate::handwork::kit(state);
    let on_plots = hands::on_plots(state);
    let built_on = |slot: usize| on_plots.iter().any(|(id, _)| slot_of(id) == Some(slot));
    let cast = crate::life::cast();
    let newborns = lives::to_be_named(state, &cast, |child| hands::named(state, child));
    let day = |period: i64| (period.max(0) as u32) + 1;
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
        item.x = px / WIDTH;
        item.y = PLOT_Y;
        let what = match state.entity(id).and_then(|e| e.component("hands.thing")) {
            Some(Value::Text(what)) => what.as_str(),
            _ => "",
        };
        let seed = days::mix(&[slot as u64, what.len() as u64, 29]);
        let building = hands::building(state, id);
        let wall = matches!(
            item.shape,
            Some(MarkShape::House | MarkShape::Shop | MarkShape::Tower)
        );
        item.kind = if wall {
            world_projection::CanvasItemKind::Place
        } else {
            world_projection::CanvasItemKind::Object
        };
        item.variant = Some(Variant {
            colour: PAINTS[(seed % PAINTS.len() as u64) as usize],
            flip: (seed >> 7) % 2 == 1,
            join_left: slot % 3 == 1 && built_on(slot - 1),
            join_right: slot % 3 == 0 && built_on(slot + 1),
        });
        if building {
            item.detail = "Being built".into();
            item.built = None;
        } else if let Some(finished) = hands::finished_at(state, &kit, id) {
            item.built = Some(day(finished as i64));
        }
    }
}

/// The name cards: the parents of a baby born these last days ask the
/// player to choose a name, from three they propose.
pub(crate) fn naming_cards(world: &World) -> Vec<world_projection::ProjectionCommand> {
    let state = world.state();
    let cast = crate::life::cast();
    lives::to_be_named(state, &cast, |child| hands::named(state, child))
        .into_iter()
        .flat_map(|child| {
            let parents = lives::generations::parents(state, child);
            let asker = parents.first().copied();
            let prompt = match parents.as_slice() {
                [a, b, ..] => format!(
                    "{} and {} would like you to choose the baby's name",
                    lives::first_name(state, *a),
                    lives::first_name(state, *b)
                ),
                [a] => format!(
                    "{} would like you to choose the baby's name",
                    lives::first_name(state, *a)
                ),
                [] => "Choose the baby's name".to_string(),
            };
            let question = world_projection::Question {
                id: format!("name-{}", child.0),
                prompt,
            };
            lives::name_proposals(state, &cast, child, 3)
                .into_iter()
                .map(move |name| world_projection::ProjectionCommand {
                    id: world_projection::command_with(&command("name", child), &name),
                    title: name,
                    detail: "Call the baby this".into(),
                    effects: Vec::new(),
                    scenery: None,
                    moves: Vec::new(),
                    asker: asker.map(SelectionId::Entity),
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
        let draws = match state.entity(id).and_then(|e| e.component("hands.thing")) {
            Some(Value::Text(what)) => spec(what).is_some_and(|work| work.draws.is_some()),
            _ => false,
        };
        draws && lives::drew(state, id).is_none()
    })
}

/// One day of what the player built drawing people to the harbour.
pub(crate) fn draw(
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    let state = world.state();
    let kit = crate::handwork::kit(state);
    let draws = hands::on_plots(state)
        .into_iter()
        .filter_map(|(_, id)| {
            let entity = state.entity(id)?;
            let what = match entity.component("hands.thing")? {
                Value::Text(what) => spec(what)?,
                _ => return None,
            };
            let (trade, job) = what.draws?;
            let since = hands::finished_at(state, &kit, id)?;
            Some(lives::Draw {
                by: id,
                what: what.thing.name.to_lowercase(),
                trade,
                job,
                since,
                cause: None,
            })
        })
        .filter(|draw| lives::drew(state, draw.by).is_none())
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
    lives::draw_newcomers(world, actions, &crate::life::cast(), &draws)
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

/// What a work on a plot is called as told: "bandstand".
#[cfg(test)]
pub(crate) fn told_as(state: &WorldState, id: EntityId) -> String {
    match state.entity(id).and_then(|e| e.component("hands.thing")) {
        Some(Value::Text(what)) => {
            spec(what).map_or_else(String::new, |w| w.thing.name.to_lowercase())
        }
        _ => String::new(),
    }
}
