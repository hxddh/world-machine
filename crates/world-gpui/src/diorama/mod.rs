//! A World as a place you look into: its landscape filling the window, its
//! places standing on the ground as buildings, its people as small figures
//! in front of wherever they are, and between turns everybody going about
//! their day.
//!
//! Everything here is presentation. Where someone wanders between turns,
//! the lights coming on after dusk and the clouds drifting over follow the
//! local clock and a seed, never anything the World records; where someone
//! *is* comes from the Pack (`CanvasItem::at`, or the stop of their day for
//! the hour), and a turn that moves them is walked.
//!
//! The scene is a lit paper-and-paint diorama, everything in it painted on
//! the CPU by [`painter`]'s brush. In the World window it is the living
//! stage ([`stage_view`], [`living_stage`]): the place baked into sprites
//! a look at a time and composited by GPUI every frame, everyone in depth.
//! The strip, covers, postcards and the golden pictures paint its still
//! layers right where they are drawn, a tile at a time, and whatever moves
//! (people, boats, smoke, clouds, rain) live over them.
//!
//! The code is in these parts: [`scene`] (the stage, the camera and the
//! land), [`people`], [`works`] (buildings, things and designs), [`light`]
//! (the hour, the sky and the weather), [`interface`] (the still layers
//! painted in place, and what a window asks of how its scene is painted),
//! and the living stage.

mod ground;
mod interface;
mod light;
mod lived;
pub mod living_stage;
pub mod passers;
mod people;
mod scene;
pub mod stage_view;
pub mod staging;
pub mod turn;
mod works;
mod yards;

pub use interface::*;
pub use light::*;
pub use people::*;
pub use scene::*;
pub use turn::Turn;
#[cfg(test)]
pub(crate) use works::leave_out_glow;
#[cfg(test)]
pub(crate) use works::leave_out_shadows;
#[allow(unused_imports)]
pub use works::*;

use crate::age::{self, Age};
use crate::art::{self, Figure, Inks, Palette, Pose};
use crate::brush::{Brush, Shape, Xform};
use crate::ladder;
use crate::mark::{self, Picture, PlotPaint, Wear, Worn};
use crate::painter::{self, Canvas, Key};
use crate::scene::Daylight;
use gpui::{point, px, size, Bounds, Corners, Hsla, Pixels, Window};
use std::collections::{BTreeMap, BTreeSet};
use tiny_skia as sk;
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasLinkTone, Drawing, GroundCover, MarkShape,
    ProjectionSnapshot, Scenery, Season, SelectionId, Stance, Weather,
};

/// Everything one frame of the stage draws, worked out before drawing so
/// the painting itself only paints.
#[derive(Clone, Debug)]
pub struct Frame {
    scenery: Scenery,
    /// How the place looks beyond its scenery, as its Pack declares it:
    /// its spine's lamps, the tint of its light, its spine and its air.
    look: Option<world_projection::PlaceLook>,
    daylight: Daylight,
    /// The hour the light is graded for, 0 to 24.
    hour: f32,
    seconds: f32,
    /// Everything still: Reduce Motion is on.
    still: bool,
    weather: Weather,
    season: Option<Season>,
    cover: Option<GroundCover>,
    ice: bool,
    /// The kind of place it is, which its ground, sky and props are
    /// dressed for (art's, B).
    setting: art::Setting,
    camera: Camera,
    /// The stage's size and lines, in stage pixels.
    width: f32,
    view_w: f32,
    height: f32,
    horizon: f32,
    base: f32,
    /// The quay's line, where people stand, and a grown-up's height there.
    feet: f32,
    figure_h: f32,
    front: f32,
    building_h: f32,
    water: bool,
    marks: Vec<RidgeMark>,
    goals: Vec<RidgeGoal>,
    /// Each cluster's patch of ground (scene's, A1: ground.rs).
    patches: Vec<ground::PatchPaint>,
    /// A nearer row of a folded postcard: its ground fades in at the top,
    /// running on from the row behind without a seam.
    blend_top: bool,
    buildings: Vec<BuildingPaint>,
    things: Vec<ThingPaint>,
    pub people: Vec<PersonPaint>,
    /// Passers-by (`passers`): nobody's resident, never in `people`.
    pub(crate) passers: Vec<PersonPaint>,
    bonds: Vec<(f32, f32, f32, CanvasLinkTone)>,
    /// Plots staked out on the ground, in stage pixels, and which one is
    /// pointed at or chosen with the keys.
    plots: Vec<PlotPaint>,
    hot_plot: Option<usize>,
    /// Designs worn this frame, and their pictures once painted, by item.
    wearing: Vec<Worn>,
    pictures: BTreeMap<usize, Picture>,
    /// Bunting strung between two buildings, by their items.
    garlands: Vec<(usize, usize)>,
    /// Where the camera is going while it glides there (a return film's
    /// beat, Find, a card closing): the living stage bakes the look it
    /// will see there ahead of it.
    heading: Option<Camera>,
    /// Every building on the stage with the zoom it shows from (0 for
    /// always): what the composition holds back for a closer look comes
    /// into the frame the camera is heading for.
    standing: std::sync::Arc<Vec<(f32, BuildingPaint)>>,
    /// What every thing on the stage is and where it stands, whatever the
    /// zoom, as a key: a new or moved one is a new look of the place.
    things_key: u64,
    /// A scene playing: its props and its weather (`staging`), drawn live
    /// over the looks, never baked into them.
    scene: Option<std::sync::Arc<staging::Dress>>,
}

impl Frame {
    /// The same frame with nothing standing in it (no buildings, things or
    /// people): in a test, what of a picture is the bare ground.
    #[cfg(test)]
    pub(crate) fn bare(&self) -> Self {
        let mut frame = self.clone();
        frame.buildings.clear();
        frame.things.clear();
        frame.people.clear();
        frame
    }

    /// The light this frame is graded in: the hour's and the weather's,
    /// turned toward the place's own key light where its Pack gives one
    /// (a cold blue over the ice rather than a gold dusk).
    pub(crate) fn light(&self) -> [f32; 3] {
        keyed_light(
            light_at(self.hour, self.weather),
            self.look.as_ref().and_then(|look| look.key),
            self.hour,
        )
    }

    /// The air over the place's field toward its hills: its Pack's own, or
    /// the sky's lowest colour.
    pub(crate) fn haze(&self) -> Hsla {
        art::hex(
            self.look
                .as_ref()
                .and_then(|look| look.haze)
                .unwrap_or(self.scenery.sky_bottom),
        )
    }

    /// The colour of the spine people walk along, if the Pack gives one.
    pub(crate) fn spine(&self) -> Option<Hsla> {
        self.look.as_ref().and_then(|look| look.spine).map(art::hex)
    }

    /// What the place's lamps and lit windows glow: its Pack's own colour,
    /// or lamplight.
    pub(crate) fn glow(&self) -> Hsla {
        art::hex(
            self.look
                .as_ref()
                .and_then(|look| look.glow)
                .unwrap_or(0xffd27a),
        )
    }

    /// The same frame graded for `hour` whatever the clock says, so a
    /// picture (a test's, the key art) never depends on when it is drawn.
    pub fn at_hour(mut self, hour: f32) -> Self {
        self.hour = hour;
        self
    }

    /// The same frame with the camera on its way to `camera`.
    pub fn heading_to(mut self, camera: Camera) -> Self {
        self.heading = Some(camera);
        self
    }

    /// Where the camera is going, if it is on its way somewhere else.
    pub(crate) fn heading(&self) -> Option<Camera> {
        self.heading.filter(|to| *to != self.camera)
    }

    /// The same frame `seconds` into looking, for the boil.
    pub fn at_seconds(mut self, seconds: f32) -> Self {
        self.seconds = seconds;
        self
    }

    /// Which drawing of the boil the still things show (see
    /// [`crate::hand`]): the first, with Reduce Motion.
    pub(crate) fn boil(&self) -> u32 {
        crate::hand::boil_at(self.seconds, self.still)
    }

    /// With everything held still, as Reduce Motion asks.
    pub fn stilled(mut self, still: bool) -> Self {
        self.still = still;
        if still {
            self.seconds = 0.0;
        }
        self
    }

    /// A building or thing clicked in the last [`BOUNCE_SECONDS`] squashes
    /// and springs back: a little wider and lower, then taller, then
    /// settled.
    pub fn bounce(&mut self, snapshot: &ProjectionSnapshot, poked: &BTreeMap<SelectionId, f32>) {
        if self.still {
            return;
        }
        for (id, ago) in poked {
            if !(0.0..BOUNCE_SECONDS).contains(ago) {
                continue;
            }
            let Some(index) = snapshot.canvas.items.iter().position(|item| item.id == *id) else {
                continue;
            };
            let t = ago / BOUNCE_SECONDS;
            // Squash, then overshoot, then settle: a damped spring.
            let spring = (t * std::f32::consts::TAU * 1.5).sin() * (1.0 - t) * (1.0 - t);
            for building in self.buildings.iter_mut().filter(|b| b.index == index) {
                building.squash = (1.0 - 0.05 * spring, 1.0 + 0.08 * spring);
            }
            for thing in self.things.iter_mut().filter(|thing| thing.index == index) {
                thing.w *= 1.0 - 0.08 * spring;
                thing.base -= thing.w * 0.12 * spring.max(0.0);
            }
        }
    }

    /// What was just built (`fresh`, by item index) rises into place,
    /// `grow` of the way (0 to 1).
    pub fn arrive(&mut self, fresh: &BTreeSet<usize>, grow: f32) {
        let grow = if self.still {
            1.0
        } else {
            grow.clamp(0.0, 1.0)
        };
        for building in &mut self.buildings {
            if fresh.contains(&building.index) {
                building.grow = grow;
            }
        }
        for thing in &mut self.things {
            if fresh.contains(&thing.index) {
                thing.grow = grow;
            }
        }
    }

    /// Everyone near whoever is speaking (`speaker`, an item index) turns
    /// their head toward them, `turned` of the way (0 to 1).
    pub fn listen(&mut self, speaker: usize, turned: f32) {
        let Some((sx, sh)) = self
            .people
            .iter()
            .find(|person| person.index == speaker)
            .map(|person| (person.x, person.height))
        else {
            return;
        };
        let turned = ease(if self.still { 1.0 } else { turned });
        for person in &mut self.people {
            if person.index == speaker
                || person.pose.stride.is_some()
                || (person.x - sx).abs() > sh * 5.0
                || (person.x - sx).abs() < 1.0
            {
                continue;
            }
            let toward = (sx - person.x).signum();
            let facing = person.pose.facing;
            person.pose.facing = facing + (toward - facing) * turned;
        }
    }

    /// Bunting strung from the eaves of the building at item `from` to
    /// the one at `to`: a party's mark, or the key art dressed for one.
    pub fn string_bunting(&mut self, from: usize, to: usize) {
        self.garlands.push((from, to));
    }

    /// The buildings in the frame, as (item index, x on the stage, how
    /// wide), left to right.
    pub fn buildings_along(&self) -> Vec<(usize, f32, f32)> {
        let mut along = self
            .buildings
            .iter()
            .map(|building| (building.index, building.x, building.w))
            .collect::<Vec<_>>();
        along.sort_by(|a, b| a.1.total_cmp(&b.1));
        along
    }

    /// The plot (by its index in the World's plots) pointed at, or chosen
    /// with the keys: drawn brighter.
    pub fn point_at_plot(&mut self, plot: Option<usize>) {
        self.hot_plot = plot;
    }

    /// The item at `index` wearing `pattern` as `wear` this frame: a design
    /// being made, shown on its target as it is drawn.
    pub fn wear(&mut self, index: usize, wear: Wear, pattern: mark::Motif) {
        self.wearing.retain(|worn| worn.index != index);
        self.wearing.push(Worn {
            index,
            wear,
            pattern: std::sync::Arc::new(pattern),
        });
    }

    /// The designs worn this frame.
    pub fn worn(&self) -> &[Worn] {
        &self.wearing
    }

    /// Whether the item at `index` is lit up this frame.
    pub fn lit(&self, index: usize) -> bool {
        self.buildings
            .iter()
            .any(|building| building.index == index && building.glow.is_some())
            || self
                .things
                .iter()
                .any(|thing| thing.index == index && thing.glow.is_some())
            || self
                .people
                .iter()
                .any(|person| person.index == index && person.glow.is_some())
    }

    /// How far the camera has moved across from the middle of the first
    /// window, in stage pixels: far layers move less than near ones.
    fn pan(&self) -> f32 {
        self.camera.x - self.view_w / 2.0
    }

    /// Where a stage point lands on screen.
    fn at(&self, x: f32, y: f32) -> (f32, f32) {
        self.camera
            .project((self.width, self.view_w, self.height), x, y)
    }

    /// Where stage `x`, on the line people stand on, is in the window.
    #[cfg(test)]
    pub(crate) fn screen_of(&self, x: f32) -> (f32, f32) {
        self.at(x, self.feet)
    }

    /// The top of the quay, where the street ends and the stone begins.
    fn quay_top(&self) -> f32 {
        self.feet - self.figure_h * 0.55
    }

    /// On the ice, the lead of open water along the back of the causeway:
    /// its far and near edges, in stage pixels (`scene::paint_lead`). About
    /// forty pixels of dark water in a window 848 high, so it reads as
    /// water the bridge spans, not the causeway's edge (v0.29 round 3).
    fn lead(&self) -> Option<(f32, f32)> {
        (self.setting == crate::art::Setting::Ice && self.water).then(|| {
            let k = (self.height / 848.0).clamp(0.3, 1.3);
            let top = self.quay_top();
            (top - 24.0 * k, top + 14.0 * k)
        })
    }
}

/// What a frame lights up, and in what colour.
pub type Glows = BTreeMap<SelectionId, Hsla>;

/// Built things on the far ridge: how many show before the oldest go.
const MARK_LIMIT: usize = 18;

/// Works out one frame: the stage seen through `camera`, `seconds` in,
/// everyone where `living` puts them, with `glows` lit and the newest
/// built thing `rising` (0 to 1) into place.
#[allow(clippy::too_many_arguments)]
pub fn frame(
    snapshot: &ProjectionSnapshot,
    stage: &Stage,
    living: &[Living],
    camera: Camera,
    seconds: f32,
    daylight: Daylight,
    glows: &Glows,
    rising: f32,
) -> Frame {
    let items = &snapshot.canvas.items;
    let scenery = snapshot.scenery.unwrap_or(PLAIN);
    let setting = art::Setting::from_key(snapshot.canvas.setting.as_deref());
    let lit = matches!(daylight, Daylight::Dusk | Daylight::Night);
    let z = camera.zoom;
    let at = |x: f32, y: f32| camera.at(stage, x, y);
    let glow_of = |item: &CanvasItem| glows.get(&item.id).copied();

    // Built things stand along the far ridge behind the buildings, oldest
    // on the left; the newest grows up out of the ground.
    let shown = snapshot.canvas.marks.len().min(MARK_LIMIT);
    let first = snapshot.canvas.marks.len() - shown;
    let marks = snapshot.canvas.marks[first..]
        .iter()
        .enumerate()
        .map(|(position, mark)| {
            let newest = first + position + 1 == snapshot.canvas.marks.len();
            RidgeMark {
                along: 0.04 + 0.92 * (position as f32 + 0.5) / MARK_LIMIT as f32,
                shape: mark.shape,
                grow: if newest { rising.clamp(0.0, 1.0) } else { 1.0 },
            }
        })
        .collect();

    // What the World is working toward stands among them, larger, as an
    // outline that fills in part by part. Only the latest few: a long
    // ladder of works would crowd the ridge.
    // A goal the Pack already stands in the town (going up in scaffolding
    // on its own site, or finished there) is not drawn again on the ridge:
    // a pier belongs on the water, never on a hill.
    let in_town = |goal: &world_projection::Goal| {
        items
            .iter()
            .any(|item| item.px.is_some() && item.label == goal.label)
    };
    // A Pack that lays its place out in clusters stands its works there,
    // and leaves the ridge to the back row.
    let clustered = !snapshot.canvas.clusters.is_empty();
    let off_stage = snapshot
        .goals
        .iter()
        .filter(|goal| !clustered && !in_town(goal))
        .cloned()
        .collect::<Vec<_>>();
    let shown_goals = &off_stage[off_stage.len().saturating_sub(GOALS_SHOWN)..];
    let goal_count = shown_goals.len();
    let building_w = stage.building_w.min(stage.building_h * 1.15);
    let goals = shown_goals
        .iter()
        .enumerate()
        .map(|(position, goal)| {
            let (w, h) = match goal.shape {
                MarkShape::Bridge => (building_w * 0.9, stage.building_h * 0.34),
                MarkShape::Tower | MarkShape::Lamp => (building_w * 0.32, stage.building_h * 0.6),
                _ => (building_w * 0.5, stage.building_h * 0.42),
            };
            RidgeGoal {
                along: 0.12 + 0.76 * (position as f32 + 0.5) / goal_count.max(1) as f32,
                w,
                h,
                shape: goal.shape,
                done: goal.done,
                parts: goal.parts,
            }
        })
        .collect();

    let mut inside = BTreeMap::<usize, Vec<Figure>>::new();
    for (person, place) in &stage.inside {
        let item = &items[*person];
        inside
            .entry(*place)
            .or_default()
            .push(Figure::of(&item.id.stable_key(), item.look));
    }
    // What the composition keeps for a closer look is left out.
    let shows = |spot: &&Spot| stage.shows(spot.index, z);
    let standing: Vec<(f32, BuildingPaint)> = stage
        .buildings
        .iter()
        .map(|spot| {
            let item = &items[spot.index];
            let shape = item.shape.unwrap_or_default();
            let drawing = snapshot.drawing_of(item).cloned();
            let (w, h) = building_size(stage, snapshot, spot, setting);
            let least = stage.shown_from.get(&spot.index).copied().unwrap_or(0.0);
            let paint = BuildingPaint {
                index: spot.index,
                x: spot.x,
                base: spot.y,
                w,
                h,
                shape,
                palette: painted_as(item, lit, setting, &scenery),
                flip: item.variant.is_some_and(|variant| variant.flip),
                // A low wall runs on only to a neighbour that stands there
                // to meet it, never into the open (v0.29 round 3's grey
                // slab beside the net store).
                joins: item.variant.map_or((false, false), |variant| {
                    // The wall reaches 0.72 of the building's width out
                    // from its middle (`works::sprite_painted`).
                    let meets = |side: f32| {
                        stage.buildings.iter().chain(&stage.things).any(|other| {
                            let along = side * (other.x - spot.x);
                            other.index != spot.index
                                && along > 0.0
                                && (other.y - spot.y).abs() < stage.figure_h * 0.25
                                && along - other.w / 2.0 <= w * 0.74
                        })
                    };
                    (
                        variant.join_left && meets(-1.0),
                        variant.join_right && meets(1.0),
                    )
                }),
                glow: glow_of(item),
                drawing,
                squash: (1.0, 1.0),
                grow: 1.0,
                inside: inside.remove(&spot.index).unwrap_or_default(),
            };
            (least, paint)
        })
        .collect();
    let buildings = standing
        .iter()
        .filter(|(least, _)| z + 1e-4 >= *least)
        .map(|(_, building)| building.clone())
        .collect();

    let things_key = {
        let mut key = Key::new("things");
        for spot in &stage.things {
            let item = &items[spot.index];
            key.add((spot.index, item.shape.map(|shape| shape as u8)))
                .float(spot.x)
                .float(spot.y)
                .float(spot.w)
                .add(item.art.as_deref())
                .add(snapshot.drawing_of(item).map(|drawing| drawing.id.clone()))
                .add(item.variant.is_some_and(|variant| variant.flip));
        }
        key.finish()
    };
    let mut things = stage
        .things
        .iter()
        .filter(shows)
        .map(|spot| {
            let item = &items[spot.index];
            let key = item.id.stable_key();
            let shape = item.shape.unwrap_or(MarkShape::Parcel);
            let seed = art::seed_of(&key);
            let sway = (seconds * 1.3 + seed as f32).sin();
            // A boat rides the swell: up and down on one wave, rolling on
            // a slower one.
            let boat = shape == MarkShape::Boat;
            let bob = if boat {
                sway * 2.0 + (seconds * 0.7 + seed as f32 * 0.3).sin() * 1.2
            } else {
                0.0
            };
            let roll = if boat {
                (seconds * 0.9 + (seed % 100) as f32).sin() * 0.05
            } else {
                0.0
            };
            let drawing = snapshot.drawing_of(item).cloned();
            let w = thing_width(stage, snapshot, spot, setting);
            ThingPaint {
                index: spot.index,
                x: spot.x,
                base: spot.y + bob,
                w,
                shape,
                palette: painted_as(item, lit, setting, &scenery),
                sway,
                roll,
                glow: glow_of(item),
                drawing,
                grow: 1.0,
                flip: item.variant.is_some_and(|variant| variant.flip),
                muted: false,
            }
        })
        .collect::<Vec<_>>();
    // At most three saturated accents a screen-width; the player's own
    // designs first.
    let designed = stage
        .buildings
        .iter()
        .chain(&stage.things)
        .filter(|spot| mark::wear_of(&items[spot.index]).is_some())
        .map(|spot| (spot.index, spot.x))
        .collect::<Vec<_>>();
    budget_accents(&mut things, &designed, stage.view_w);

    // Who stands with whom: how far round each is turned.
    let centres = group_centres(&stage.people, stage.figure_h);
    let big = stage.figure_h * z >= turn::READS_FROM;
    let mut people = stage
        .people
        .iter()
        .zip(living)
        .zip(&centres)
        .filter(|((spot, _), _)| shows(spot))
        .map(|((spot, life), centre)| {
            let item = &items[spot.index];
            let (x, y) = at(life.x, spot.y);
            // Nearer is a little bigger.
            let near = spot.scale;
            let figure = Figure::of(&item.id.stable_key(), item.look);
            let height = stage.figure_h * z * near * figure.age.height();
            let what = stage.doing.get(&spot.index).copied().unwrap_or_default();
            // Walking beats everything (with a load, carrying it); a wave
            // beats what an errand has them do, which beats what their stop
            // this hour says they do there, which beats what the Pack says
            // (talking, celebrating, working), which beats an idle moment.
            let stop = what
                .stance
                .filter(|stance| !matches!(stance, Stance::Standing | Stance::Walking));
            let stance = match (life.pose.stride, life.stance, life.using, stop, item.stance) {
                (Some(_), ..) if life.loaded => Stance::Carrying,
                (Some(_), ..) => Stance::Walking,
                (None, Some(Stance::Waving), ..) => Stance::Waving,
                (None, _, Some(using), ..) => using,
                (None, _, None, Some(stop), _) => stop,
                (None, _, None, None, Some(pack)) if pack != Stance::Standing => pack,
                (None, Some(idle), ..) => idle,
                _ => Stance::Standing,
            };
            // On a bench, sitting on its seat: how high it is for their
            // height. Their own seat otherwise.
            let drawing = snapshot.drawing_of(item).cloned();
            let seat = what
                .fixture
                .filter(|(_, how)| *how == Use::Seat && stance == Stance::Sitting)
                .filter(|_| drawing.is_some() || figure.bird)
                .and_then(|(thing, _)| stage.things.iter().find(|spot| spot.index == thing))
                .map(|bench| seat_height(stage, snapshot, bench, setting) * z / height.max(1.0));
            // A dance in four beats, everyone on a beat of their own.
            let seed = art::seed_of(&item.id.stable_key());
            let beat = (stance == Stance::Dancing)
                .then(|| ((seconds * 2.4 + (seed % 13) as f32 * 0.31).max(0.0) as u32 % 4) as u8);
            PersonPaint {
                index: spot.index,
                x,
                y,
                height,
                pose: Pose {
                    bob: life.pose.bob * z,
                    ..life.pose
                },
                glow: glow_of(item),
                drawing,
                stance,
                mood: item.mood.unwrap_or_default(),
                carrying: None,
                along: life.x,
                seat,
                beat,
                behind: what
                    .fixture
                    .filter(|(_, how)| *how == Use::Keep)
                    .map(|(thing, _)| thing),
                posture: None,
                turn: if big && !figure.bird && figure.age != Age::Baby {
                    turn_of(stage, spot, life, &what, *centre, stance, seed)
                } else {
                    Turn::Front
                },
                detail: big,
                figure,
            }
        })
        .collect::<Vec<_>>();
    one_talks_at_a_time(&mut people, stage, &centres, seconds);
    carry_babies(&mut people, items, stage.figure_h * z);
    people.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.index.cmp(&b.index)));

    // A pair standing together wear their bond between them, just above
    // their heads.
    let where_is = |id: SelectionId| {
        people
            .iter()
            .find(|person| items[person.index].id == id)
            .map(|person| (person.x, person.y, person.height))
    };
    let bonds = snapshot
        .canvas
        .links
        .iter()
        .filter_map(|link| {
            let (x1, y1, h) = where_is(link.from)?;
            let (x2, y2, _) = where_is(link.to)?;
            // Only a pair side by side: one behind the other (a ring's
            // back and its front, v0.30 S1d round 2) would wear it on the
            // hand of whoever talks at the back.
            ((x1 - x2).abs() < h * 1.4 && (y1 - y2).abs() < h * 0.2).then(|| {
                let lift = (seconds * 2.2).sin() * 1.5;
                (
                    (x1 + x2) / 2.0,
                    y1.min(y2) - h * 1.12 + lift,
                    h * 0.17,
                    link.tone,
                )
            })
        })
        .collect();

    let near = art::hex(scenery.near);
    let water = setting
        .water()
        .unwrap_or(near.h > 0.45 && near.h < 0.72 && near.s > 0.2);
    // The light of this very hour when the frame is of now; the middle of
    // its part of the day when it is pinned to another.
    let hour = crate::scene::hour_of_day();
    let hour = if crate::scene::daylight_at(hour as u32) == daylight {
        hour
    } else {
        match daylight {
            Daylight::Dawn => 6.5,
            Daylight::Day => 13.0,
            Daylight::Dusk => 19.5,
            Daylight::Night => 23.0,
        }
    };
    Frame {
        scenery,
        look: snapshot.canvas.look.clone(),
        daylight,
        hour,
        seconds,
        still: false,
        weather: snapshot.weather,
        season: snapshot.canvas.season,
        cover: snapshot.canvas.ground,
        ice: snapshot.canvas.ice,
        setting,
        camera,
        width: stage.width,
        view_w: stage.view_w,
        height: stage.height,
        horizon: stage.horizon,
        base: stage.base,
        feet: stage.feet,
        figure_h: stage.figure_h,
        front: stage.front,
        building_h: stage.building_h,
        water,
        marks,
        goals,
        patches: ground::patches(snapshot, stage),
        blend_top: false,
        buildings,
        things,
        passers: passers::passers(snapshot, stage, camera, seconds),
        people,
        bonds,
        plots: stage.plots.clone(),
        hot_plot: None,
        wearing: stage
            .buildings
            .iter()
            .chain(&stage.things)
            .filter_map(|spot| {
                let item = &items[spot.index];
                Some(Worn {
                    index: spot.index,
                    wear: mark::wear_of(item)?,
                    pattern: std::sync::Arc::new(mark::pattern_of(item)?),
                })
            })
            .collect(),
        pictures: BTreeMap::new(),
        garlands: Vec::new(),
        heading: None,
        standing: std::sync::Arc::new(standing),
        things_key,
        scene: None,
    }
}

/// How wide and tall the building on `spot` is drawn: as wide as it
/// stands and as tall as its depth has it, a Pack's drawing in its own
/// proportions, and on the ladder beside a grown-up at its depth, never
/// much wider than the room its place was given.
pub(crate) fn building_size(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    spot: &Spot,
    setting: art::Setting,
) -> (f32, f32) {
    let item = &snapshot.canvas.items[spot.index];
    let shape = item.shape.unwrap_or_default();
    let drawing = snapshot.drawing_of(item);
    // As wide as it stands, as tall as its depth has it: something
    // wide stands a little lower and a tower taller.
    // One squeezed into a narrow gap keeps its proportions.
    let squeezed = (spot.w / (stage.building_w * spot.scale).max(1.0)).min(1.0);
    let (w, h) = (spot.w, stage.height_of(spot, shape) * squeezed);
    // A Pack's own drawing keeps its own proportions, no wider
    // than a building's place allows.
    let (w, h) = match drawing {
        Some(drawing) => {
            let w = (h * drawing.aspect).min(spot.w * 1.25);
            (w, w / drawing.aspect)
        }
        None => (w, h),
    };
    // On the ladder: as tall as it stands beside a grown-up at its
    // depth (crate::ladder), whatever room its spot was given.
    let p = stage.figure_h * spot.scale;
    match drawing {
        Some(drawing) => ladder::Subject::Drawing(drawing).sized(p, spot.w),
        None => ladder::Subject::Building(shape, setting, art_of(item)).sized(p, spot.w),
    }
    // Never much wider than the room its place was given: in a
    // narrow gap it keeps its proportions, a little smaller.
    .map(|(lw, lh)| {
        let fit = (spot.w * ROOM / lw.max(1.0)).min(1.0);
        (lw * fit, lh * fit)
    })
    .unwrap_or((w, h))
}

/// How wide the thing on `spot` is drawn, in stage pixels: on the ladder,
/// as tall as it stands beside a grown-up at its depth, whatever room its
/// spot was given.
pub(crate) fn thing_width(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    spot: &Spot,
    setting: art::Setting,
) -> f32 {
    let item = &snapshot.canvas.items[spot.index];
    let shape = item.shape.unwrap_or(MarkShape::Parcel);
    let p = stage.figure_h * spot.scale;
    match snapshot.drawing_of(item) {
        Some(drawing) => ladder::Subject::Drawing(drawing).sized(p, spot.w),
        None => ladder::Subject::Thing(shape, setting, art_of(item)).sized(p, spot.w),
    }
    .map_or(spot.w, |(w, _)| {
        w.min((spot.w * ROOM * 1.3).max(w * LEAST_OF_RUNG))
    })
}

/// How wide and tall the thing on `spot` is drawn, in stage pixels (the
/// width as [`thing_width`], the height to the same scale).
pub(crate) fn thing_size(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    spot: &Spot,
    setting: art::Setting,
) -> (f32, f32) {
    let item = &snapshot.canvas.items[spot.index];
    let shape = item.shape.unwrap_or(MarkShape::Parcel);
    let p = stage.figure_h * spot.scale;
    let sized = match snapshot.drawing_of(item) {
        Some(drawing) => ladder::Subject::Drawing(drawing).sized(p, spot.w),
        None => ladder::Subject::Thing(shape, setting, art_of(item)).sized(p, spot.w),
    };
    match sized {
        Some((w, h)) => {
            let most = (spot.w * ROOM * 1.3).max(w * LEAST_OF_RUNG);
            if w > most {
                (most, h * most / w.max(1e-3))
            } else {
                (w, h)
            }
        }
        None => (spot.w, p * 0.5),
    }
}

/// How high above its foot the seat of the bench on `spot` is, in stage
/// pixels, as it is drawn: the top of the plank someone sits on.
pub(crate) fn seat_height(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    spot: &Spot,
    setting: art::Setting,
) -> f32 {
    let item = &snapshot.canvas.items[spot.index];
    let w = thing_width(stage, snapshot, spot, setting);
    match snapshot.drawing_of(item) {
        Some(drawing) => w / drawing.aspect.max(0.1) * 0.5,
        None => match art_of(item).or_else(|| crate::works::kit(setting, MarkShape::Bench)) {
            Some(art) => w * crate::works::THING_SCALE * art.tall() * crate::works::seat_of(art),
            // The plain bench (`art::paint_thing`): its seat's top.
            None => w * 0.34 * 0.62,
        },
    }
}

/// How far past the room its spot gives it something on the ladder may
/// reach: a building, and a thing a little more (its spot is narrower
/// than what it is drawn as).
const ROOM: f32 = 1.15;

/// However narrow the room a thing was given, it is drawn at no less than
/// this share of its rung (1.0 P5: a postbox squeezed to half its height,
/// a lantern to a quarter, read as toys beside the people).
const LEAST_OF_RUNG: f32 = 0.85;

/// The library drawing a Pack names for an item, if the app has it.
fn art_of(item: &CanvasItem) -> Option<crate::works::Art> {
    item.art.as_deref().and_then(crate::works::Art::from_key)
}

/// How many of what the World works towards stand on the ridge.
const GOALS_SHOWN: usize = 5;

/// How much each layer moves with the camera, from the sky (least) to
/// the ground under people's feet (fully).
pub const PARALLAX: [f32; 4] = [0.06, 0.18, 0.4, 1.0];

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod outdoors;

#[cfg(test)]
mod living_tests;

#[cfg(test)]
pub(crate) mod rest_tests;

#[cfg(test)]
pub(crate) mod doing_tests;

#[cfg(test)]
mod staging_tests;

#[cfg(test)]
mod scale_tests;
