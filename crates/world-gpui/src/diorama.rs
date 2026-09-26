//! A World as a place you look into: its landscape filling the window, its
//! places standing on the ground as buildings, its people as small figures
//! in front of wherever they are, and between turns everybody going about
//! their day.
//!
//! Everything here is presentation. Where someone wanders between turns,
//! the lights coming on after dusk and the clouds drifting over follow the
//! local clock and a seed, never anything the World records; where someone
//! *is* comes from the Pack (`CanvasItem::at`), and a turn that moves them
//! is walked.

use crate::art::{self, Figure, Palette, Pose};
use crate::scene::Daylight;
use gpui::{
    linear_color_stop, linear_gradient, point, px, size, Bounds, Hsla, PathBuilder, Pixels, Window,
};
use std::collections::{BTreeMap, BTreeSet};
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasLinkTone, MarkShape, ProjectionSnapshot, Scenery, SelectionId,
};

/// The colours of a World that does not say what it looks like: a mild
/// green valley under a pale sky.
const PLAIN: Scenery = Scenery {
    sky_top: 0xb9d6ea,
    sky_bottom: 0xeef3ea,
    far: 0x9db88f,
    near: 0x6f9a6a,
    sun: 0xffe7a8,
};

/// Where the parts of the landscape sit, as fractions of the stage height:
/// the horizon, the line buildings stand on, the line people stand on, and
/// the top of the foreground (water in a harbour, dark dust on Mars) that
/// the choice card floats over.
const HORIZON: f32 = 0.40;
const BASE: f32 = 0.58;
const FEET: f32 = 0.63;
const FRONT: f32 = 0.80;
const MARGIN: f32 = 0.07;

/// One thing's place on the stage, in stage pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// Index into the snapshot's canvas items.
    pub index: usize,
    pub x: f32,
    pub y: f32,
}

/// Where everything stands, for a stage `width` by `height` pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    pub width: f32,
    pub height: f32,
    pub horizon: f32,
    pub base: f32,
    pub feet: f32,
    pub front: f32,
    pub building_w: f32,
    pub building_h: f32,
    pub figure_h: f32,
    pub thing_w: f32,
    /// Places, drawn as buildings with their base at `y`.
    pub buildings: Vec<Spot>,
    /// Things, drawn with their base at `y`.
    pub things: Vec<Spot>,
    /// People, standing with their feet at `y`.
    pub people: Vec<Spot>,
}

impl Stage {
    pub fn person(&self, id: SelectionId, snapshot: &ProjectionSnapshot) -> Option<Spot> {
        self.people
            .iter()
            .copied()
            .find(|spot| snapshot.canvas.items.get(spot.index).map(|item| item.id) == Some(id))
    }

    /// Where anything on stage is, and roughly how big it is, as a box
    /// around it: what a camera frames when it goes to look at it.
    pub fn frame_of(&self, index: usize) -> Option<(f32, f32, f32, f32)> {
        if let Some(spot) = self.buildings.iter().find(|spot| spot.index == index) {
            return Some((
                spot.x - self.building_w / 2.0,
                spot.y - self.building_h,
                self.building_w,
                self.building_h,
            ));
        }
        if let Some(spot) = self.things.iter().find(|spot| spot.index == index) {
            return Some((
                spot.x - self.thing_w / 2.0,
                spot.y - self.thing_w * 0.6,
                self.thing_w,
                self.thing_w * 0.6,
            ));
        }
        self.people
            .iter()
            .find(|spot| spot.index == index)
            .map(|spot| {
                (
                    spot.x - self.figure_h * 0.3,
                    spot.y - self.figure_h,
                    self.figure_h * 0.6,
                    self.figure_h,
                )
            })
    }
}

/// Pairs a Pack wants drawn together, who stand side by side.
fn pairs(snapshot: &ProjectionSnapshot) -> Vec<(SelectionId, SelectionId)> {
    snapshot
        .canvas
        .links
        .iter()
        .map(|link| (link.from, link.to))
        .collect()
}

/// Lays out a World on a stage `width` by `height` pixels. Places stand in
/// a row along the ground in the order the Pack placed them, left to right;
/// things with nobody's place stand in that row too; people stand in front
/// of wherever they are, pairs side by side; anyone who is nowhere in
/// particular stands where the Pack put them. Nobody stands on anybody
/// else. The same World always lays out the same.
pub fn stage(snapshot: &ProjectionSnapshot, width: f32, height: f32) -> Stage {
    let items = &snapshot.canvas.items;
    let width = width.max(120.0);
    let height = height.max(90.0);
    // A window's stage keeps its people big enough to see; a cover's
    // shrinks everything with it.
    let building_h = (height * 0.19).min(176.0).max((height * 0.3).min(92.0));
    let figure_h = (height * 0.092).min(84.0).max((height * 0.14).min(52.0));
    let index_of = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.id, index))
        .collect::<BTreeMap<_, _>>();
    // Where each item is: its host, when the host is on stage and is not
    // itself somewhere else.
    let host = |index: usize| -> Option<usize> {
        let host = *index_of.get(&items[index].at?)?;
        (host != index && items[host].at.is_none()).then_some(host)
    };
    let mut anchors = (0..items.len())
        .filter(|index| host(*index).is_none() && items[*index].kind != CanvasItemKind::Actor)
        .collect::<Vec<_>>();
    anchors.sort_by(|a, b| {
        items[*a]
            .x
            .total_cmp(&items[*b].x)
            .then(items[*a].y.total_cmp(&items[*b].y))
            .then(a.cmp(b))
    });
    let usable = width * (1.0 - 2.0 * MARGIN);
    let slots = anchors.len().max(1) as f32;
    let slot_w = usable / slots;
    let building_w = (building_h * 1.15).min(slot_w * 0.62);
    let thing_w = (building_w * 0.62).max(40.0);
    let base = height * BASE;
    let feet = height * FEET;
    let slot_x = anchors
        .iter()
        .enumerate()
        .map(|(slot, index)| (*index, width * MARGIN + slot_w * (slot as f32 + 0.5)))
        .collect::<BTreeMap<_, _>>();

    let mut buildings = Vec::new();
    let mut things = Vec::new();
    for (index, x) in &slot_x {
        let spot = Spot {
            index: *index,
            x: *x,
            y: if items[*index].kind == CanvasItemKind::Place {
                base
            } else {
                feet
            },
        };
        if items[*index].kind == CanvasItemKind::Place {
            buildings.push(spot);
        } else {
            things.push(spot);
        }
    }
    buildings.sort_by(|a, b| a.x.total_cmp(&b.x));
    // Things kept at a place stand beside it: a boat moored at the harbour
    // floats off its side, an order waits at the bakery's door.
    let mut beside = BTreeMap::<usize, usize>::new();
    for (index, item) in items.iter().enumerate() {
        if item.kind == CanvasItemKind::Actor {
            continue;
        }
        let Some(host) = host(index) else {
            continue;
        };
        let Some(host_x) = slot_x.get(&host) else {
            continue;
        };
        let nth = beside.entry(host).or_default();
        let side = if nth.is_multiple_of(2) { 1.0 } else { -1.0 };
        *nth += 1;
        let boat = item.shape == Some(MarkShape::Boat);
        // A boat is moored toward the nearer edge, out of the way of the
        // card that floats over the middle of the water.
        let side = if boat {
            if *host_x < width / 2.0 {
                -1.0
            } else {
                1.0
            }
        } else {
            side
        };
        things.push(Spot {
            index,
            x: host_x + side * (building_w * 0.5 + thing_w * 0.45),
            y: if boat {
                height * FRONT + (height - height * FRONT) * 0.22
            } else {
                feet
            },
        });
    }

    // People stand in front of wherever they are, in a row centred on it,
    // pairs side by side; beside a thing rather than in front of it.
    let spacing = figure_h * 0.66;
    let pairs = pairs(snapshot);
    let mut people = Vec::new();
    let mut hosted = BTreeMap::<usize, Vec<usize>>::new();
    let mut loose = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if item.kind != CanvasItemKind::Actor {
            continue;
        }
        match host(index).filter(|host| slot_x.contains_key(host)) {
            Some(host) => hosted.entry(host).or_default().push(index),
            None => loose.push(index),
        }
    }
    for (host, mut members) in hosted {
        for (a, b) in &pairs {
            let find = |id: &SelectionId, members: &[usize]| {
                members.iter().position(|member| items[*member].id == *id)
            };
            if let (Some(first), Some(second)) = (find(a, &members), find(b, &members)) {
                let partner = members.remove(second);
                let first = if second < first { first - 1 } else { first };
                members.insert(first + 1, partner);
            }
        }
        let centre = slot_x[&host]
            - if items[host].kind == CanvasItemKind::Place {
                0.0
            } else {
                thing_w * 0.5 + spacing * members.len() as f32 / 2.0
            };
        let row = spacing * members.len().saturating_sub(1) as f32;
        for (position, member) in members.into_iter().enumerate() {
            people.push(Spot {
                index: member,
                x: centre - row / 2.0 + spacing * position as f32,
                y: feet,
            });
        }
    }
    for index in loose {
        people.push(Spot {
            index,
            x: width * MARGIN + usable * items[index].x.clamp(0.0, 1.0),
            y: feet,
        });
    }
    // Nobody stands on anybody: sweep left to right, then pull back inside
    // the stage if the row ran off its right edge.
    people.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.index.cmp(&b.index)));
    for position in 1..people.len() {
        let floor = people[position - 1].x + spacing;
        if people[position].x < floor {
            people[position].x = floor;
        }
    }
    let right = width - spacing;
    if let Some(last) = people.last().map(|spot| spot.x) {
        if last > right {
            let shift = last - right;
            for spot in &mut people {
                spot.x -= shift;
            }
            for position in (0..people.len().saturating_sub(1)).rev() {
                let ceiling = people[position + 1].x - spacing;
                if people[position].x > ceiling {
                    people[position].x = ceiling;
                }
            }
        }
    }
    people.sort_by_key(|spot| spot.index);

    Stage {
        width,
        height,
        horizon: height * HORIZON,
        base,
        feet,
        front: height * FRONT,
        building_w,
        building_h,
        figure_h,
        thing_w,
        buildings,
        things,
        people,
    }
}

/// How long someone takes to walk to where a turn put them.
pub const WALK_SECONDS: f32 = 1.4;

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Where someone is this frame, and how they stand: presentation only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Living {
    pub x: f32,
    pub pose: Pose,
}

/// Where each person is this frame, `seconds` into looking at the World.
///
/// Between turns everyone goes about their day: each on a cycle of their
/// own, most of it at home, part of it walking over to another place and
/// back. At night nobody wanders. Anyone `pinned` (speaking, being asked,
/// in the news) stays where they are. `walking` is how far through the walk
/// a turn started is, from where `before` had them.
pub fn living(
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    seconds: f32,
    daylight: Daylight,
    pinned: &BTreeSet<SelectionId>,
    before: Option<(&Stage, &ProjectionSnapshot, f32)>,
) -> Vec<Living> {
    let items = &snapshot.canvas.items;
    let stops = stage
        .buildings
        .iter()
        .chain(stage.things.iter())
        .map(|spot| spot.x)
        .collect::<Vec<_>>();
    stage
        .people
        .iter()
        .map(|spot| {
            let item = &items[spot.index];
            let key = item.id.stable_key();
            let seed = art::seed_of(&key);
            let home = spot.x;
            let breathe = (seconds * 1.7 + (seed % 100) as f32 * 0.07).sin() * 0.6;
            // Walking over to where a turn put them.
            if let Some((old_stage, old_snapshot, progress)) = before {
                if progress < 1.0 {
                    if let Some(old) = old_stage.person(item.id, old_snapshot) {
                        if (old.x - home).abs() > 2.0 {
                            let x = old.x + (home - old.x) * ease(progress);
                            return Living {
                                x,
                                pose: Pose {
                                    stride: Some((seconds * 1.8).fract()),
                                    bob: 0.0,
                                    facing: (home - old.x).signum(),
                                },
                            };
                        }
                    }
                }
            }
            let still = Living {
                x: home + (seconds * 0.23 + seed as f32).sin() * 3.0,
                pose: Pose {
                    stride: None,
                    bob: breathe,
                    facing: ((seconds * 0.11 + (seed % 7) as f32).sin() * 1.4).clamp(-1.0, 1.0),
                },
            };
            if pinned.contains(&item.id) || daylight == Daylight::Night || stops.len() < 2 {
                return still;
            }
            // A visit: out to another place, a while there, and home.
            let period = 34.0 + (seed % 17) as f32;
            let phase = ((seconds + (seed % 1000) as f32 * 0.37) % period) / period;
            let nearest = stops
                .iter()
                .enumerate()
                .min_by(|a, b| (a.1 - home).abs().total_cmp(&(b.1 - home).abs()))
                .map(|(position, _)| position)
                .unwrap_or(0);
            let others = stops.len() - 1;
            let pick = (nearest + 1 + (seed as usize / 7) % others) % stops.len();
            let away = stops[pick] + ((seed % 5) as f32 - 2.0) * stage.figure_h * 0.25;
            let walk = |from: f32, to: f32, t: f32| Living {
                x: from + (to - from) * ease(t),
                pose: Pose {
                    stride: Some((seconds * 1.8).fract()),
                    bob: 0.0,
                    facing: (to - from).signum(),
                },
            };
            match phase {
                p if (0.60..0.68).contains(&p) => walk(home, away, (p - 0.60) / 0.08),
                p if (0.68..0.80).contains(&p) => Living { x: away, ..still },
                p if (0.80..0.88).contains(&p) => walk(away, home, (p - 0.80) / 0.08),
                _ => still,
            }
        })
        .collect()
}

/// Where the camera looks: `zoom` times closer, centred on (`x`, `y`) in
/// stage pixels. At rest it looks at the whole stage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
}

impl Camera {
    pub fn whole(stage: &Stage) -> Self {
        Self {
            zoom: 1.0,
            x: stage.width / 2.0,
            y: stage.height / 2.0,
        }
    }

    /// Close enough on a box to see who is in it, and no closer than twice.
    pub fn on(stage: &Stage, (x, y, w, h): (f32, f32, f32, f32)) -> Self {
        let room = (stage.width * 0.55 / w.max(1.0)).min(stage.height * 0.45 / h.max(1.0));
        let zoom = room.clamp(1.0, 1.8);
        // Keep the view inside the stage: never show past its edges.
        let half_w = stage.width / zoom / 2.0;
        let half_h = stage.height / zoom / 2.0;
        Self {
            zoom,
            x: (x + w / 2.0).clamp(half_w, stage.width - half_w),
            // Frame a little above centre, so the card below does not
            // cover what is being looked at.
            y: (y + h * 0.7).clamp(half_h, stage.height - half_h),
        }
    }

    pub fn toward(self, target: Camera, t: f32) -> Self {
        let t = ease(t);
        Self {
            zoom: self.zoom + (target.zoom - self.zoom) * t,
            x: self.x + (target.x - self.x) * t,
            y: self.y + (target.y - self.y) * t,
        }
    }

    /// Where a stage point lands on screen.
    pub fn at(&self, stage: &Stage, x: f32, y: f32) -> (f32, f32) {
        (
            (x - self.x) * self.zoom + stage.width / 2.0,
            (y - self.y) * self.zoom + stage.height / 2.0,
        )
    }
}

/// A person as drawn this frame, in screen pixels.
#[derive(Clone, Debug)]
pub struct PersonPaint {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub figure: Figure,
    pub pose: Pose,
    /// A soft light on the ground under them: news, a preview, the one
    /// being asked.
    pub glow: Option<Hsla>,
}

#[derive(Clone, Debug)]
struct BuildingPaint {
    index: usize,
    x: f32,
    base: f32,
    w: f32,
    h: f32,
    shape: MarkShape,
    palette: Palette,
    glow: Option<Hsla>,
}

#[derive(Clone, Debug)]
struct ThingPaint {
    index: usize,
    x: f32,
    base: f32,
    w: f32,
    shape: MarkShape,
    palette: Palette,
    sway: f32,
    glow: Option<Hsla>,
}

/// Everything one frame of the stage draws, worked out before drawing so
/// the painting itself only paints.
#[derive(Clone, Debug)]
pub struct Frame {
    scenery: Scenery,
    daylight: Daylight,
    seconds: f32,
    zoom: f32,
    horizon: f32,
    base: f32,
    front: f32,
    water: bool,
    marks: Vec<(f32, f32, f32, f32, MarkShape, f32)>,
    /// Standing goals on the ridge: where, how big, their shape, and how
    /// many of their parts are built.
    goals: Vec<(f32, f32, f32, f32, MarkShape, u32, u32)>,
    buildings: Vec<BuildingPaint>,
    things: Vec<ThingPaint>,
    pub people: Vec<PersonPaint>,
    bonds: Vec<(f32, f32, f32, CanvasLinkTone)>,
}

impl Frame {
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
    let lit = matches!(daylight, Daylight::Dusk | Daylight::Night);
    let z = camera.zoom;
    let at = |x: f32, y: f32| camera.at(stage, x, y);
    let glow_of = |item: &CanvasItem| glows.get(&item.id).copied();

    // Built things stand along the far ridge behind the buildings, oldest
    // on the left; the newest grows up out of the ground.
    let shown = snapshot.canvas.marks.len().min(MARK_LIMIT);
    let first = snapshot.canvas.marks.len() - shown;
    let mark_h = stage.building_h * 0.34;
    let marks = snapshot.canvas.marks[first..]
        .iter()
        .enumerate()
        .map(|(position, mark)| {
            let fx = 0.04 + 0.92 * (position as f32 + 0.5) / MARK_LIMIT as f32;
            let (x, y) = at(stage.width * fx, stage.horizon + stage.building_h * 0.12);
            let newest = first + position + 1 == snapshot.canvas.marks.len();
            let grow = if newest { rising } else { 1.0 };
            (x, y, mark_h * 0.7 * z, mark_h * z * grow, mark.shape, grow)
        })
        .collect();

    // What the World is working toward stands among them, larger, as an
    // outline that fills in part by part.
    let goal_count = snapshot.goals.len();
    let goals = snapshot
        .goals
        .iter()
        .enumerate()
        .map(|(position, goal)| {
            let fx = 0.12 + 0.76 * (position as f32 + 0.5) / goal_count.max(1) as f32;
            // Up on the ridge line, above the rooftops, so the buildings in
            // front never hide what the World is working toward.
            let (x, y) = at(stage.width * fx, stage.horizon - stage.building_h * 0.08);
            let (w, h) = match goal.shape {
                MarkShape::Bridge => (stage.building_w * 0.9, stage.building_h * 0.34),
                MarkShape::Tower | MarkShape::Lamp => {
                    (stage.building_w * 0.32, stage.building_h * 0.6)
                }
                _ => (stage.building_w * 0.5, stage.building_h * 0.42),
            };
            (x, y, w * z, h * z, goal.shape, goal.done, goal.parts)
        })
        .collect();

    let buildings = stage
        .buildings
        .iter()
        .map(|spot| {
            let item = &items[spot.index];
            let (x, base) = at(spot.x, spot.y);
            let shape = item.shape.unwrap_or_default();
            // Something wide stands a little lower and a tower taller.
            let (w, h) = match shape {
                MarkShape::Bridge => (stage.building_w * 1.3, stage.building_h * 0.7),
                MarkShape::Tower => (stage.building_w, stage.building_h * 1.2),
                MarkShape::Dome | MarkShape::Tree => (stage.building_w, stage.building_h * 0.8),
                _ => (stage.building_w, stage.building_h),
            };
            BuildingPaint {
                index: spot.index,
                x,
                base,
                w: w * z,
                h: h * z,
                shape,
                palette: Palette::of(&item.id.stable_key(), lit),
                glow: glow_of(item),
            }
        })
        .collect();

    let things = stage
        .things
        .iter()
        .map(|spot| {
            let item = &items[spot.index];
            let key = item.id.stable_key();
            let shape = item.shape.unwrap_or(MarkShape::Parcel);
            let seed = art::seed_of(&key);
            let sway = (seconds * 1.3 + seed as f32).sin();
            let bob = if shape == MarkShape::Boat {
                sway * 2.0
            } else {
                0.0
            };
            let w = match shape {
                MarkShape::Parcel => stage.thing_w * 0.4,
                _ => stage.thing_w,
            };
            let (x, base) = at(spot.x, spot.y + bob);
            ThingPaint {
                index: spot.index,
                x,
                base,
                w: w * z,
                shape,
                palette: Palette::of(&key, lit),
                sway,
                glow: glow_of(item),
            }
        })
        .collect();

    let mut people = stage
        .people
        .iter()
        .zip(living)
        .map(|(spot, life)| {
            let item = &items[spot.index];
            let (x, y) = at(life.x, spot.y);
            PersonPaint {
                index: spot.index,
                x,
                y,
                height: stage.figure_h * z,
                figure: Figure::of(&item.id.stable_key(), item.look),
                pose: Pose {
                    bob: life.pose.bob * z,
                    ..life.pose
                },
                glow: glow_of(item),
            }
        })
        .collect::<Vec<_>>();
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
            ((x1 - x2).abs() < h * 1.4).then(|| {
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
    let water = near.h > 0.45 && near.h < 0.72 && near.s > 0.2;
    Frame {
        scenery,
        daylight,
        seconds,
        zoom: z,
        horizon: at(0.0, stage.horizon).1,
        base: at(0.0, stage.base).1,
        front: at(0.0, stage.front).1,
        water,
        marks,
        goals,
        buildings,
        things,
        people,
        bonds,
    }
}

/// Paints a frame into `bounds`.
pub fn paint(frame: &Frame, bounds: Bounds<Pixels>, window: &mut Window) {
    let ox = f32::from(bounds.origin.x);
    let oy = f32::from(bounds.origin.y);
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    let scenery = frame.scenery;
    let t = frame.seconds;
    let night = frame.daylight == Daylight::Night;
    let horizon = oy + frame.horizon;
    // Sun, moon and clouds keep their size relative to the stage, so a
    // cover is the same picture as a window, only smaller.
    let k = (height / 848.0).clamp(0.3, 1.3);

    // Sky, over everything; the light of the hour laid on top of it.
    window.paint_quad(gpui::fill(
        bounds,
        linear_gradient(
            180.0,
            linear_color_stop(art::hex(scenery.sky_top), 0.0),
            linear_color_stop(art::hex(scenery.sky_bottom), 0.55),
        ),
    ));
    let (tint_top, tint_bottom) = match frame.daylight {
        Daylight::Day => (None, None),
        Daylight::Dawn => (Some((0xffbaa0, 0.20)), Some((0xffe4c8, 0.08))),
        Daylight::Dusk => (Some((0xff8a5c, 0.26)), Some((0x7a4080, 0.18))),
        Daylight::Night => (Some((0x0e1436, 0.72)), Some((0x0e1436, 0.5))),
    };
    if let (Some((top, top_alpha)), Some((bottom, bottom_alpha))) = (tint_top, tint_bottom) {
        window.paint_quad(gpui::fill(
            bounds,
            linear_gradient(
                180.0,
                linear_color_stop(art::hex(top).opacity(top_alpha), 0.0),
                linear_color_stop(art::hex(bottom).opacity(bottom_alpha), 1.0),
            ),
        ));
    }
    // Sun by day, a moon and stars by night.
    let (sun_x, sun_y) = (ox + width * 0.8, oy + frame.horizon * 0.34);
    if night {
        let mut seed: u32 = 0x9e37_79b9;
        for _ in 0..60 {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let x = (seed % 1000) as f32 / 1000.0;
            let y = ((seed / 1000) % 1000) as f32 / 1000.0;
            let twinkle = 0.5 + 0.5 * (t * 1.3 + (seed % 97) as f32).sin();
            art::circle(
                window,
                ox + x * width,
                oy + y * frame.horizon * 0.9,
                1.1,
                gpui::white().opacity(0.4 + 0.5 * twinkle),
            );
        }
        art::circle(window, sun_x, sun_y, 22.0 * k, art::hex(0xf4f1e6));
        art::circle(
            window,
            sun_x + 8.0 * k,
            sun_y - 5.0 * k,
            20.0 * k,
            art::hex(0x0e1436).opacity(0.9),
        );
    } else {
        let sun = art::hex(scenery.sun);
        art::circle(window, sun_x, sun_y, 58.0 * k, sun.opacity(0.18));
        art::circle(window, sun_x, sun_y, 36.0 * k, sun);
    }
    // Clouds drifting across, slowly, each at its own pace.
    let cloud = if night {
        gpui::white().opacity(0.08)
    } else {
        gpui::white().opacity(0.72)
    };
    for index in 0..4 {
        let speed = 4.0 + index as f32 * 1.7;
        let span = width + 320.0;
        let x = ox + ((index as f32 * 331.0 + t * speed) % span) - 160.0;
        let y = oy + frame.horizon * (0.16 + 0.14 * index as f32);
        let s = (1.0 - index as f32 * 0.12) * k;
        art::ellipse(window, x, y, 46.0 * s, 16.0 * s, cloud);
        art::ellipse(window, x + 30.0 * s, y - 8.0 * s, 32.0 * s, 16.0 * s, cloud);
        art::ellipse(window, x - 28.0 * s, y + 2.0 * s, 26.0 * s, 11.0 * s, cloud);
    }

    // The far ridge, the ground, and the foreground.
    let darken = |colour: Hsla| {
        if night {
            art::shade(colour, -0.55)
        } else {
            colour
        }
    };
    let far = darken(art::hex(scenery.far));
    let ridge_top = horizon - (frame.base - frame.horizon) * 0.35;
    let mut ridge = PathBuilder::fill();
    ridge.move_to(point(px(ox), px(horizon + 6.0)));
    ridge.curve_to(
        point(px(ox + width * 0.45), px(ridge_top + 10.0)),
        point(px(ox + width * 0.2), px(ridge_top - 16.0)),
    );
    ridge.curve_to(
        point(px(ox + width), px(horizon)),
        point(px(ox + width * 0.78), px(ridge_top + 24.0)),
    );
    ridge.line_to(point(px(ox + width), px(oy + height)));
    ridge.line_to(point(px(ox), px(oy + height)));
    ridge.close();
    if let Ok(path) = ridge.build() {
        window.paint_path(path, art::shade(far, -0.08));
    }
    // What the World has built stands along the ridge.
    let silhouette = art::shade(far, -0.3);
    let light = art::hex(scenery.sun);
    for (x, y, w, h, shape, grow) in &frame.marks {
        if *h <= 0.5 {
            continue;
        }
        let bounds = Bounds::new(
            point(px(ox + x - w / 2.0), px(oy + y - h)),
            size(px(*w), px(*h)),
        );
        crate::ui::paint_mark(
            window,
            bounds,
            *shape,
            silhouette.opacity(0.35 + 0.65 * grow),
            light,
        );
    }
    // A goal not yet finished is a pale outline, more solid with each part
    // built, with a pip under it for every part; a finished one stands as
    // solid as anything else the World has built.
    for (x, y, w, h, shape, done, parts) in &frame.goals {
        let bounds = Bounds::new(
            point(px(ox + x - w / 2.0), px(oy + y - h)),
            size(px(*w), px(*h)),
        );
        if done >= parts {
            crate::ui::paint_mark(window, bounds, *shape, silhouette, light);
            continue;
        }
        let share = *done as f32 / (*parts).max(1) as f32;
        let ghost = gpui::white().opacity(0.28 + 0.4 * share);
        crate::ui::paint_mark(window, bounds, *shape, ghost, light.opacity(0.4));
        let pip = (w * 0.09).clamp(3.0, 6.0);
        let gap = pip * 0.8;
        let row = *parts as f32 * pip + (*parts as f32 - 1.0) * gap;
        for part in 0..*parts {
            let px0 = ox + x - row / 2.0 + part as f32 * (pip + gap);
            let filled = part < *done;
            window.paint_quad(gpui::quad(
                Bounds::new(point(px(px0), px(oy + y + pip)), size(px(pip), px(pip))),
                px(pip / 2.0),
                if filled {
                    gpui::white().opacity(0.9)
                } else {
                    gpui::white().opacity(0.25)
                },
                px(0.0),
                gpui::transparent_black(),
                gpui::BorderStyle::default(),
            ));
        }
    }
    let ground = art::shade(far, 0.16);
    let ground_top = horizon + (frame.base - frame.horizon) * 0.3;
    let mut field = PathBuilder::fill();
    field.move_to(point(px(ox), px(ground_top + 12.0)));
    field.curve_to(
        point(px(ox + width), px(ground_top)),
        point(px(ox + width * 0.55), px(ground_top - 22.0)),
    );
    field.line_to(point(px(ox + width), px(oy + height)));
    field.line_to(point(px(ox), px(oy + height)));
    field.close();
    if let Ok(path) = field.build() {
        window.paint_path(path, ground);
    }
    let front_top = oy + frame.front;
    let near = darken(art::hex(scenery.near));
    let mut shore = PathBuilder::fill();
    shore.move_to(point(px(ox), px(front_top + 8.0)));
    shore.curve_to(
        point(px(ox + width), px(front_top - 4.0)),
        point(px(ox + width * 0.5), px(front_top - 14.0)),
    );
    shore.line_to(point(px(ox + width), px(oy + height)));
    shore.line_to(point(px(ox), px(oy + height)));
    shore.close();
    if let Ok(path) = shore.build() {
        window.paint_path(path, near);
    }
    if frame.water {
        // The sea moves: short bright lines drifting and fading.
        let shimmer = gpui::white().opacity(if night { 0.12 } else { 0.28 });
        for row in 0..5 {
            let y = front_top + 16.0 + row as f32 * (oy + height - front_top) / 5.5;
            for column in 0..9 {
                let drift = (t * (6.0 + row as f32) + column as f32 * 137.0) % (width + 80.0);
                let x = ox + drift - 40.0;
                let pulse = 0.5 + 0.5 * (t * 1.1 + column as f32 + row as f32 * 0.7).sin();
                art::rect(
                    window,
                    x,
                    y,
                    18.0 + 10.0 * pulse,
                    2.0,
                    1.0,
                    shimmer.opacity(shimmer.a * pulse),
                );
            }
        }
    }

    // Glows on the ground under whatever is lit up.
    let pool = |window: &mut Window, x: f32, y: f32, r: f32, colour: Hsla| {
        let breathe = 0.85 + 0.15 * (t * 2.4).sin();
        art::ellipse(
            window,
            ox + x,
            oy + y,
            r * 1.3 * breathe,
            r * 0.34 * breathe,
            colour.opacity(0.22),
        );
        art::ellipse(
            window,
            ox + x,
            oy + y,
            r * 0.9 * breathe,
            r * 0.22 * breathe,
            colour.opacity(0.32),
        );
    };
    for building in &frame.buildings {
        if let Some(glow) = building.glow {
            pool(window, building.x, building.base, building.w * 0.6, glow);
        }
        art::paint_building(
            window,
            ox + building.x,
            oy + building.base,
            building.w,
            building.h,
            building.shape,
            &building.palette,
        );
    }
    for thing in &frame.things {
        if let Some(glow) = thing.glow {
            pool(window, thing.x, thing.base, thing.w * 0.6, glow);
        }
        art::paint_thing(
            window,
            ox + thing.x,
            oy + thing.base,
            thing.w,
            thing.shape,
            &thing.palette,
            thing.sway,
        );
    }
    for person in &frame.people {
        if let Some(glow) = person.glow {
            pool(window, person.x, person.y, person.height * 0.7, glow);
        }
        art::paint_figure(
            window,
            ox + person.x,
            oy + person.y,
            person.height,
            &person.figure,
            person.pose,
        );
    }
    for (x, y, r, tone) in &frame.bonds {
        art::paint_bond(window, ox + x, oy + y, *r, *tone);
    }
    let _ = frame.zoom;
}

/// A World's cover: its landscape, what it has built, and its people and
/// buildings as it last stood, drawn the way its window draws them.
pub fn cover(
    scenery: Option<Scenery>,
    marks: &[MarkShape],
    cast: Vec<CanvasItem>,
) -> gpui::Canvas<()> {
    let snapshot = ProjectionSnapshot {
        scenery,
        canvas: world_projection::CanvasProjection {
            items: cast,
            links: Vec::new(),
            marks: marks
                .iter()
                .map(|shape| world_projection::CanvasMark {
                    label: String::new(),
                    shape: *shape,
                    selection: None,
                })
                .collect(),
        },
        ..ProjectionSnapshot::default()
    };
    let daylight = crate::scene::daylight_now();
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let stage = stage(
                &snapshot,
                f32::from(bounds.size.width),
                f32::from(bounds.size.height),
            );
            let people = living(&stage, &snapshot, 0.0, daylight, &BTreeSet::new(), None);
            let frame = frame(
                &snapshot,
                &stage,
                &people,
                Camera::whole(&stage),
                0.0,
                daylight,
                &Glows::new(),
                1.0,
            );
            paint(&frame, bounds, window);
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_projection::{CanvasItem, CanvasItemKind, CanvasProjection};

    fn entity(id: u64) -> SelectionId {
        SelectionId::from_stable_key(&format!("entity-{id}")).expect("an entity key")
    }

    fn item(id: u64, kind: CanvasItemKind, x: f32, at: Option<u64>) -> CanvasItem {
        CanvasItem {
            id: entity(id),
            kind,
            label: format!("Item {id}"),
            detail: String::new(),
            x,
            y: 0.5,
            changes: Vec::new(),
            shape: None,
            at: at.map(entity),
            look: None,
        }
    }

    fn harbour() -> ProjectionSnapshot {
        let mut items = vec![
            item(101, CanvasItemKind::Place, 0.1, None),
            item(102, CanvasItemKind::Place, 0.6, None),
            item(103, CanvasItemKind::Place, 0.6, None),
            item(104, CanvasItemKind::Place, 0.3, None),
            item(201, CanvasItemKind::Object, 0.0, Some(101)),
        ];
        for (id, x, at) in [
            (1, 0.1, None),
            (2, 0.7, Some(102)),
            (3, 0.3, Some(104)),
            (4, 0.7, Some(103)),
            (5, 0.8, Some(103)),
            (6, 0.2, Some(101)),
            (7, 0.0, Some(101)),
            (8, 0.4, Some(104)),
        ] {
            items.push(item(id, CanvasItemKind::Actor, x, at));
        }
        items[4].shape = Some(MarkShape::Boat);
        ProjectionSnapshot {
            canvas: CanvasProjection {
                items,
                links: Vec::new(),
                marks: Vec::new(),
            },
            ..ProjectionSnapshot::default()
        }
    }

    #[test]
    fn places_stand_in_order_and_people_stand_at_theirs_without_overlap() {
        let snapshot = harbour();
        let stage = stage(&snapshot, 1100.0, 848.0);
        let xs = stage
            .buildings
            .iter()
            .map(|spot| (snapshot.canvas.items[spot.index].x, spot.x))
            .collect::<Vec<_>>();
        for pair in xs.windows(2) {
            assert!(pair[0].0 <= pair[1].0 && pair[0].1 < pair[1].1, "{xs:?}");
        }
        let mut people = stage.people.clone();
        people.sort_by(|a, b| a.x.total_cmp(&b.x));
        for pair in people.windows(2) {
            assert!(pair[1].x - pair[0].x >= stage.figure_h * 0.66 - 0.01);
        }
        assert!(people
            .iter()
            .all(|spot| spot.x > 0.0 && spot.x < stage.width));
        // People at a place stand near it.
        let bakery = stage.buildings.iter().find(|spot| spot.index == 1).unwrap();
        let mara = stage.people.iter().find(|spot| spot.index == 6).unwrap();
        assert!((mara.x - bakery.x).abs() < stage.building_w);
        // A boat floats in the foreground beside its harbour.
        let boat = stage.things.iter().find(|spot| spot.index == 4).unwrap();
        assert!(boat.y > stage.front);
    }

    #[test]
    fn the_same_world_always_lays_out_the_same() {
        let snapshot = harbour();
        assert_eq!(
            stage(&snapshot, 900.0, 700.0),
            stage(&snapshot, 900.0, 700.0)
        );
    }

    #[test]
    fn nobody_wanders_at_night_or_while_they_are_needed() {
        let snapshot = harbour();
        let stage = stage(&snapshot, 1100.0, 848.0);
        let homes = stage.people.iter().map(|spot| spot.x).collect::<Vec<_>>();
        let pinned = BTreeSet::new();
        for second in 0..200 {
            let night = living(
                &stage,
                &snapshot,
                second as f32,
                Daylight::Night,
                &pinned,
                None,
            );
            for (life, home) in night.iter().zip(&homes) {
                assert!((life.x - home).abs() <= 3.0);
            }
        }
        let everyone = snapshot.canvas.items.iter().map(|item| item.id).collect();
        let day = living(&stage, &snapshot, 1000.0, Daylight::Day, &everyone, None);
        for (life, home) in day.iter().zip(&homes) {
            assert!((life.x - home).abs() <= 3.0);
        }
        // By day, over a few minutes, somebody goes somewhere.
        let wandered = (0..240).any(|second| {
            living(
                &stage,
                &snapshot,
                second as f32,
                Daylight::Day,
                &pinned,
                None,
            )
            .iter()
            .zip(&homes)
            .any(|(life, home)| (life.x - home).abs() > stage.figure_h)
        });
        assert!(wandered);
    }

    #[test]
    fn a_camera_on_something_frames_it_and_never_looks_past_the_edges() {
        let snapshot = harbour();
        let stage = stage(&snapshot, 1100.0, 848.0);
        let whole = Camera::whole(&stage);
        assert_eq!(whole.at(&stage, 10.0, 20.0), (10.0, 20.0));
        let close = Camera::on(&stage, stage.frame_of(0).unwrap());
        assert!(close.zoom > 1.0 && close.zoom <= 1.8);
        let (left, top) = close.at(&stage, 0.0, 0.0);
        let (right, bottom) = close.at(&stage, stage.width, stage.height);
        assert!(left <= 0.01 && top <= 0.01);
        assert!(right >= stage.width - 0.01 && bottom >= stage.height - 0.01);
    }
}
