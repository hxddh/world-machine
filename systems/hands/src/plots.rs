//! Plots on the scene: where they lie along a place, what each offers,
//! how what the player built on one is dressed, the name cards for a
//! newborn, and what a finished work draws to live in the place.
//!
//! The mechanics are this System's (plots, designs, names) and the `lives`
//! System's (who comes). A Pack gives its list of works ([`PlotWork`]),
//! its stretches and places, its paints and its words through
//! [`PlotPack`]. Where the plots lie is worked out from the place's street
//! and never recorded, like the rest of its layout.

use crate::{Effect, Kit, Thing, Verb};
use world_core::{ActionRegistry, EntityId, Event, EventId, Value, World, WorldError, WorldState};
use world_projection::{
    CanvasItem, CanvasItemKind, Designable, MarkShape, Naming, PlotOffer, SelectionId, Variant,
    Wears,
};

/// The row plots lie in, behind the front row: no other row's spots are
/// within a hand's breadth of it.
pub const PLOT_ROW: usize = days::town::PLOTS;
/// How far down the scene the plot row lies, for an app that knows one
/// screen only.
pub const PLOT_Y: f32 = days::town::PLOT_ROW_AT.y;
/// Where the first plot's row begins, and how far apart its slots are.
pub const PLOT_OFFSET: f32 = days::town::PLOT_ROW_AT.offset;
pub const PLOT_PITCH: f32 = days::town::PLOT_ROW_AT.pitch;
/// The most a plot offers at once: everything that belongs on its stretch.
const OFFERED: usize = 12;

/// A work the player can build on a plot: what it is, which stretch of the
/// place it belongs on, whom it draws to live there (the trade as told,
/// and the job), what it can wear, and the story's work that stands for it
/// if the place builds that one of its own accord.
pub struct PlotWork {
    pub thing: Thing,
    pub stretch: usize,
    pub draws: Option<(&'static str, &'static str)>,
    pub wears: Option<Wears>,
    pub twin: Option<&'static str>,
}

/// A work built on a plot, on `stretch`.
pub const fn plot_work(
    id: &'static str,
    name: &'static str,
    shape: &'static str,
    cost: i64,
    effect: Effect,
    stretch: usize,
) -> PlotWork {
    PlotWork {
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

/// A work that draws someone of `trade` to live in the place, with `job`.
pub const fn draws(mut work: PlotWork, trade: &'static str, job: &'static str) -> PlotWork {
    work.draws = Some((trade, job));
    work
}

/// A work that can wear a design.
pub const fn wearing(mut work: PlotWork, wears: Wears) -> PlotWork {
    work.wears = Some(wears);
    work
}

/// A work the story may build of its own accord as `twin`.
pub const fn twin(mut work: PlotWork, twin: &'static str) -> PlotWork {
    work.twin = Some(twin);
    work
}

/// Where a plot slot lies along the place.
pub fn plot_px(slot: usize) -> f32 {
    PLOT_OFFSET + (slot as f32 + 0.5) * PLOT_PITCH
}

/// How many plot slots a place `width` wide has room for.
pub fn plot_slots(width: f32) -> usize {
    ((width - PLOT_OFFSET) / PLOT_PITCH).floor() as usize
}

pub fn plot_id(slot: usize) -> String {
    format!("p{slot}")
}

pub fn slot_of(id: &str) -> Option<usize> {
    id.strip_prefix('p')?.parse().ok()
}

/// How what the player builds on plots looks.
pub struct PlotLook<'a> {
    /// The paints the place picks from.
    pub paints: &'a [[u8; 3]],
    /// Mixed into each work's seed, so each place picks its own.
    pub salt: u64,
    /// Shapes with walls, which are places someone goes into.
    pub walls: &'a [MarkShape],
    /// Whether works stand in pairs (two in every three slots, joined
    /// within a pair) rather than joined to any neighbour.
    pub pairs: bool,
}

/// What a Pack tells this module about its plots.
pub trait PlotPack {
    /// Whether the World is ready to have plots at all.
    fn ready(&self, _state: &WorldState) -> bool {
        true
    }
    /// Every work a plot can hold here.
    fn works(&self, state: &WorldState) -> &'static [PlotWork];
    /// The plot slots, by index along the row.
    fn slots(&self, state: &WorldState) -> Vec<usize>;
    /// How wide the place is, in screens.
    fn width(&self, state: &WorldState) -> f32;
    /// Which stretch a point lies on.
    fn stretch_at(&self, state: &WorldState, px: f32) -> usize;
    /// A stretch's id.
    fn stretch_id(&self, state: &WorldState, stretch: usize) -> &'static str;
    /// The place a plot on a stretch is told as lying by.
    fn place_on(&self, state: &WorldState, stretch: usize, px: f32) -> EntityId;
    /// Whether the story finished a work of its own, by id.
    fn finished_work(&self, _state: &WorldState, _id: &str) -> bool {
        false
    }
    /// What the player's hands can do here.
    fn kit(&self, state: &WorldState) -> Kit;
    /// A deed's command.
    fn deed_command(&self, key: &str) -> String;
    /// The command to design or name something.
    fn command(&self, what: &str, target: EntityId) -> String;
    /// How a fixture's shape is drawn.
    fn fixture_shape(&self, world: &World, shape: &str) -> MarkShape;
    /// The drawing for a work, if it has one of its own.
    fn art_of(&self, state: &WorldState, what: &str) -> Option<&'static str>;
    /// What something can wear a design as, if it can.
    fn wears_of(&self, state: &WorldState, id: EntityId) -> Option<Wears>;
    /// What something the player can name is, if they can.
    fn naming(&self, state: &WorldState, id: EntityId) -> Option<String>;
    /// What a newborn is called in a name card: "baby", "chick".
    fn newborn_word(&self, state: &WorldState, child: EntityId) -> String;
    /// The place's people.
    fn cast(&self, state: &WorldState) -> lives::Cast;
    /// The place's people, with the room kept for whoever a work draws.
    fn newcomer_cast(&self, state: &WorldState) -> lives::Cast {
        self.cast(state)
    }
    fn look(&self, state: &WorldState) -> PlotLook<'_>;
}

/// A work by its id.
pub fn spec(pack: &impl PlotPack, state: &WorldState, id: &str) -> Option<&'static PlotWork> {
    pack.works(state).iter().find(|work| work.thing.id == id)
}

/// Whether the place built a work's twin of its own accord, so another
/// would be one too many.
fn twin_built(pack: &impl PlotPack, state: &WorldState, work: &PlotWork) -> bool {
    work.twin
        .is_some_and(|twin| pack.finished_work(state, twin))
}

/// The place's plots, and what could be built on each: everything that
/// belongs on its stretch.
pub fn plots(pack: &impl PlotPack, state: &WorldState) -> Vec<crate::Plot> {
    pack.slots(state)
        .into_iter()
        .map(|slot| {
            let px = plot_px(slot);
            let stretch = pack.stretch_at(state, px);
            crate::Plot {
                id: plot_id(slot),
                at: pack.place_on(state, stretch, px),
                offers: pack
                    .works(state)
                    .iter()
                    .filter(|work| work.stretch == stretch && !twin_built(pack, state, work))
                    .map(|work| work.thing.id)
                    .collect(),
            }
        })
        .collect()
}

/// What something the player made is, as the `hands` System keeps it.
pub fn made_thing(state: &WorldState, id: EntityId) -> Option<&str> {
    if !crate::made(state).contains(&id) {
        return None;
    }
    match state.entity(id)?.component("hands.thing")? {
        Value::Text(thing) => Some(thing.as_str()),
        _ => None,
    }
}

/// The plots as a screen shows them, with what could stand on each,
/// starting from a different one on each plot.
pub fn canvas_plots(pack: &impl PlotPack, world: &World) -> Vec<world_projection::Plot> {
    let state = world.state();
    if !pack.ready(state) {
        return Vec::new();
    }
    let kit = pack.kit(state);
    crate::plot_deeds(world, &kit)
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
                district: pack.stretch_id(state, pack.stretch_at(state, px)).into(),
                offers: deeds
                    .into_iter()
                    .take(OFFERED)
                    .map(|deed| {
                        let what = crate::plot_work(&deed.key).unwrap_or_default();
                        PlotOffer {
                            command: pack.deed_command(&deed.key),
                            label: deed.thing,
                            shape: spec(pack, state, what).map_or(MarkShape::House, |work| {
                                pack.fixture_shape(world, work.thing.shape)
                            }),
                            cost: deed.cost,
                            unavailable: deed.unavailable,
                            art: pack.art_of(state, what).map(Into::into),
                        }
                    })
                    .collect(),
            })
        })
        .filter(|plot| !plot.offers.is_empty())
        .collect()
}

/// Places the works standing on plots on their plots, and says what each
/// thing on the scene can wear, be named, and how the place built it.
pub fn dress(pack: &impl PlotPack, world: &World, items: &mut [CanvasItem]) {
    let state = world.state();
    if !pack.ready(state) {
        return;
    }
    let width = pack.width(state);
    let kit = pack.kit(state);
    let on_plots = crate::on_plots(state);
    let built_on = |slot: usize| on_plots.iter().any(|(id, _)| slot_of(id) == Some(slot));
    let cast = pack.cast(state);
    let newborns = lives::to_be_named(state, &cast, |child| crate::named(state, child));
    let look = pack.look(state);
    for item in items.iter_mut() {
        let SelectionId::Entity(id) = item.id else {
            continue;
        };
        if let Some(wears) = pack.wears_of(state, id) {
            item.design = Some(Designable {
                command: pack.command("design", id),
                wears,
            });
            item.pattern = crate::pattern_of(state, id)
                .and_then(|text| world_projection::Design::parse(text).ok())
                .map(|design| design.pattern());
        }
        // Something the player made says so, never what it is kept as.
        if item.detail.is_empty() && crate::made(state).contains(&id) {
            item.detail = if crate::plot_of(state, id).is_some() {
                "Built by you"
            } else {
                "Made by you"
            }
            .into();
        }
        if pack.naming(state, id).is_some() {
            item.naming = Some(Naming {
                command: pack.command("name", id),
                proposals: if newborns.contains(&id) {
                    lives::name_proposals(state, &cast, id, 3)
                } else {
                    Vec::new()
                },
            });
        }
        let Some(slot) = crate::plot_of(state, id).and_then(slot_of) else {
            continue;
        };
        let px = plot_px(slot);
        item.px = Some(px);
        item.x = px / width;
        let thing = match state.entity(id).and_then(|e| e.component("hands.thing")) {
            Some(Value::Text(what)) => what.as_str(),
            _ => "",
        };
        // A work for the water (a slipway, a boathouse) stands at the
        // water's edge in front of its plot, going up or finished.
        let art = pack.art_of(state, thing);
        item.y = if days::town::zone_of(art, item.shape.unwrap_or_default())
            == days::town::Zone::Water
        {
            days::town::WATER_Y
        } else {
            PLOT_Y
        };
        let what = thing.len() as u64;
        let seed = days::mix(&[slot as u64, what, look.salt]);
        let wall = item.shape.is_some_and(|shape| look.walls.contains(&shape));
        item.kind = if wall {
            CanvasItemKind::Place
        } else {
            CanvasItemKind::Object
        };
        // Neighbours on plots side by side are joined: a wall run on, or
        // a path between.
        let (join_left, join_right) = if look.pairs {
            (
                slot % 3 == 1 && built_on(slot - 1),
                slot % 3 == 0 && built_on(slot + 1),
            )
        } else {
            (slot > 0 && built_on(slot - 1), built_on(slot + 1))
        };
        item.variant = Some(Variant {
            colour: look.paints[(seed % look.paints.len() as u64) as usize],
            flip: (seed >> 7) % 2 == 1,
            join_left,
            join_right,
        });
        if crate::building(state, id) {
            item.detail = "Being built".into();
            item.built = None;
            // Scaffolding stands round it until it is finished.
            item.art = Some("scaffold".into());
        } else if let Some(finished) = crate::finished_at(state, &kit, id) {
            item.built = Some(finished as u32 + 1);
        }
    }
}

/// The name cards: the parents of a newborn of these last days ask the
/// player to choose a name, from three they propose.
pub fn naming_cards(
    pack: &impl PlotPack,
    world: &World,
) -> Vec<world_projection::ProjectionCommand> {
    let state = world.state();
    if !pack.ready(state) {
        return Vec::new();
    }
    let cast = pack.cast(state);
    lives::to_be_named(state, &cast, |child| crate::named(state, child))
        .into_iter()
        .flat_map(|child| {
            let parents = lives::generations::parents(state, child);
            let what = pack.newborn_word(state, child);
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
            let command = pack.command("name", child);
            lives::name_proposals(state, &cast, child, 3)
                .into_iter()
                .map(move |name| world_projection::ProjectionCommand {
                    id: world_projection::command_with(&command, &name),
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
pub fn waiting(pack: &impl PlotPack, state: &WorldState) -> bool {
    crate::on_plots(state).into_iter().any(|(_, id)| {
        let draws = match state.entity(id).and_then(|e| e.component("hands.thing")) {
            Some(Value::Text(what)) => {
                spec(pack, state, what).is_some_and(|work| work.draws.is_some())
            }
            _ => false,
        };
        draws && lives::drew(state, id).is_none()
    })
}

/// One period of what the player built drawing people to the place.
pub fn draw(
    pack: &impl PlotPack,
    world: &mut World,
    actions: &ActionRegistry,
) -> Result<Vec<EventId>, WorldError> {
    let state = world.state();
    let kit = pack.kit(state);
    let draws = crate::on_plots(state)
        .into_iter()
        .filter_map(|(_, id)| {
            let entity = state.entity(id)?;
            let what = match entity.component("hands.thing")? {
                Value::Text(what) => spec(pack, state, what)?,
                _ => return None,
            };
            let (trade, job) = what.draws?;
            let since = crate::finished_at(state, &kit, id)?;
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
    let cast = pack.newcomer_cast(world.state());
    lives::draw_newcomers(world, actions, &cast, &draws)
}

/// A line told before someone was named, told with the name they were
/// given: a baby's birth, told with the name the player chose.
pub fn renamed(state: &WorldState, event: &Event, line: String) -> String {
    if event.kind != "born" {
        return line;
    }
    let Some(Value::Entity(child)) = event.payload.get("who") else {
        return line;
    };
    let (Some(was), true) = (
        crate::was_called(state, *child),
        crate::named(state, *child),
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
