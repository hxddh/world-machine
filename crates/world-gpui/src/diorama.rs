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
//! The scene is a lit paper-and-paint diorama. What stands still (the sky,
//! the hills fading into it, the ground, and the buildings with their
//! light, shadows and lit windows) is painted on the CPU by [`painter`]
//! into images, kept until the hour, the weather, the season, the zoom or
//! the size changes. Whatever moves (people, boats, smoke, clouds, rain) is
//! drawn live over them every frame.

use crate::art::{self, Figure, Inks, Palette, Pose};
use crate::brush::{Brush, Shape, Xform};
use crate::painter::{self, Canvas, Key};
use crate::scene::Daylight;
use gpui::{point, px, size, Bounds, Corners, Hsla, Pixels, Window};
use std::collections::{BTreeMap, BTreeSet};
use tiny_skia as sk;
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasLinkTone, Drawing, GroundCover, MarkShape,
    ProjectionSnapshot, Scenery, Season, SelectionId, Stance, Weather,
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
    /// How wide the whole place is: the window's width, or for a panorama
    /// that many window widths.
    pub width: f32,
    pub height: f32,
    /// How wide the window onto it is.
    pub view_w: f32,
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
    /// People indoors this hour, and where: seen, if at all, as a shape in
    /// a lit window. `(person, place)`, as item indices.
    pub inside: Vec<(usize, usize)>,
    /// People on their way to where the hour's routine puts them: where
    /// they set off from, and how many seconds ago.
    pub routes: BTreeMap<usize, (f32, f32)>,
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

    /// How many window widths the place is.
    pub fn panorama(&self) -> f32 {
        (self.width / self.view_w.max(1.0)).max(1.0)
    }
}

/// Where along the ground a stage point `x` is, from 0 (the left edge of
/// the row places stand in) to 100 (its right edge): the spot something
/// the player puts down there takes.
pub fn ground_spot(stage: &Stage, x: f32) -> u8 {
    let usable = stage.width * (1.0 - 2.0 * MARGIN);
    (((x - stage.width * MARGIN) / usable.max(1.0)) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8
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

/// The local clock, as far as where people are goes: the hour, and how far
/// into it. UI time, never the World's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clock {
    pub hour: u8,
    /// Seconds since the hour began.
    pub into_hour: f32,
}

impl Clock {
    /// Now, on this computer's clock; a pinned hour (`WORLD_MACHINE_HOUR`)
    /// is well into itself, so nobody is still on their way.
    pub fn now() -> Self {
        use chrono::Timelike;
        let pinned = std::env::var("WORLD_MACHINE_HOUR")
            .ok()
            .and_then(|hour| hour.parse::<u8>().ok())
            .filter(|hour| *hour < 24);
        match pinned {
            Some(hour) => Self::at(hour),
            None => {
                let now = chrono::Local::now();
                Self {
                    hour: now.hour() as u8,
                    into_hour: (now.minute() * 60 + now.second()) as f32
                        + now.nanosecond().min(999_999_999) as f32 / 1e9,
                }
            }
        }
    }

    /// Well into `hour`.
    pub fn at(hour: u8) -> Self {
        Self {
            hour: hour % 24,
            into_hour: 1800.0,
        }
    }
}

/// How fast someone strolls to where their day takes them, in figure
/// heights a second.
const STROLL: f32 = 1.1;

/// Lays out a World on a stage `width` by `height` pixels, at the hour on
/// the local clock.
pub fn stage(snapshot: &ProjectionSnapshot, width: f32, height: f32) -> Stage {
    stage_at(snapshot, width, height, Clock::now())
}

/// Lays out a World on a window `width` by `height` pixels at `clock`.
/// Places stand in a row along the ground in the order the Pack placed
/// them, left to right, or where their `px` puts them along a panorama;
/// things with nobody's place stand in that row too; people stand in front
/// of wherever they are (or wherever their day has them this hour), pairs
/// side by side; anyone who is nowhere in particular stands where the Pack
/// put them. Nobody stands on anybody else. The same World at the same
/// hour always lays out the same.
pub fn stage_at(snapshot: &ProjectionSnapshot, width: f32, height: f32, clock: Clock) -> Stage {
    let items = &snapshot.canvas.items;
    let view_w = width.max(120.0);
    let height = height.max(90.0);
    let panorama = snapshot.canvas.width.unwrap_or(1.0).clamp(1.0, 24.0);
    let width = view_w * panorama;
    // A window's stage keeps its people big enough to see; a cover's
    // shrinks everything with it.
    let building_h = (height * 0.19).min(176.0).max((height * 0.3).min(92.0));
    let figure_h = (height * 0.092).min(84.0).max((height * 0.14).min(52.0));
    let index_of = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.id, index))
        .collect::<BTreeMap<_, _>>();
    // Where each person's day has them this hour, and where it had them
    // before.
    let routine = |index: usize| {
        let day = &items[index].day;
        let now = world_projection::stop_at(day, clock.hour)?;
        let position = day.iter().position(|stop| stop == now)?;
        let before = day[(position + day.len() - 1) % day.len()];
        Some((*now, before))
    };
    let placed = |id: SelectionId| {
        index_of
            .get(&id)
            .copied()
            .filter(|host| items[*host].kind != CanvasItemKind::Actor)
    };
    // Where each item is: its host, when the host is on stage and is not
    // itself somewhere else.
    let host = |index: usize| -> Option<usize> {
        let at = if items[index].kind == CanvasItemKind::Actor && !items[index].day.is_empty() {
            routine(index)
                .and_then(|(now, _)| placed(now.at).map(|_| now.at))
                .or(items[index].at)
        } else {
            items[index].at
        }?;
        let host = *index_of.get(&at)?;
        (host != index && items[host].at.is_none()).then_some(host)
    };
    let along = |item: &CanvasItem| item.px.map(|px| px.clamp(0.0, panorama) * view_w);
    // What the player stood somewhere of their choosing keeps its spot and
    // takes no place in the row; nor does what the Pack put at a point
    // along its panorama.
    let mut anchors = (0..items.len())
        .filter(|index| {
            host(*index).is_none()
                && items[*index].kind != CanvasItemKind::Actor
                && items[*index].spot.is_none()
                && items[*index].px.is_none()
        })
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
    let mut slot_w = usable / slots;
    // Along a panorama, a building is no wider than the gap to its
    // neighbour.
    // Along a panorama, a building is no wider than the gap to its
    // neighbour in its own row.
    let mut rows = BTreeMap::<i32, Vec<f32>>::new();
    for (index, item) in items.iter().enumerate() {
        if host(index).is_none() && item.kind != CanvasItemKind::Actor && item.spot.is_none() {
            if let Some(x) = along(item) {
                rows.entry((item.y * 100.0).round() as i32)
                    .or_default()
                    .push(x);
            }
        }
    }
    let gap = rows
        .values_mut()
        .filter_map(|row| {
            row.sort_by(f32::total_cmp);
            row.windows(2)
                .map(|pair| pair[1] - pair[0])
                .filter(|gap| *gap > 1.0)
                .min_by(f32::total_cmp)
        })
        .min_by(f32::total_cmp);
    if let Some(gap) = gap {
        slot_w = slot_w.min(gap * 1.45);
    }
    let building_w = (building_h * 1.15).min(slot_w * 0.62).max(building_h * 0.5);
    let thing_w = (building_w * 0.62).max(40.0);
    let base = height * BASE;
    let feet = height * FEET;
    let mut slot_x = anchors
        .iter()
        .enumerate()
        .map(|(slot, index)| (*index, width * MARGIN + slot_w * (slot as f32 + 0.5)))
        .collect::<BTreeMap<_, _>>();
    for (index, item) in items.iter().enumerate() {
        if item.kind != CanvasItemKind::Actor && item.spot.is_none() && host(index).is_none() {
            if let Some(x) = along(item) {
                slot_x.insert(index, x);
            }
        }
    }

    // A panorama's rows stand one behind another: the back row on the
    // line buildings stand on, the front one near the water.
    let front = height * FRONT;
    let row_line = |item: &CanvasItem| {
        item.px.map(|_| {
            let depth = ((item.y - 0.30) / 0.46).clamp(0.0, 1.0);
            // Homes in the back row stand a step or two further back, each
            // by its own seed, so a street is not one straight line.
            let back = if depth < 0.05 && item.shape == Some(MarkShape::House) {
                (art::seed_of(&item.id.stable_key()) % 3) as f32 * building_h * 0.07
            } else {
                0.0
            };
            base + depth * (front - base) * 0.62 - back
        })
    };
    let mut buildings = Vec::new();
    let mut things = Vec::new();
    for (index, x) in &slot_x {
        let item = &items[*index];
        let spot = Spot {
            index: *index,
            x: *x,
            y: row_line(item).unwrap_or(if item.kind == CanvasItemKind::Place {
                base
            } else {
                feet
            }),
        };
        if items[*index].kind == CanvasItemKind::Place {
            buildings.push(spot);
        } else if items[*index].shape == Some(MarkShape::Boat) {
            things.push(Spot {
                y: height * FRONT + (height - height * FRONT) * 0.22,
                ..spot
            });
        } else {
            things.push(spot);
        }
    }
    // Back rows first, so nearer buildings stand in front of them.
    buildings.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    // Things kept at a place stand beside it: a boat moored at the harbour
    // floats off its side, an order waits at the bakery's door.
    let mut beside = BTreeMap::<usize, usize>::new();
    for (index, item) in items.iter().enumerate() {
        if item.kind == CanvasItemKind::Actor {
            continue;
        }
        if let Some(spot) = item.spot {
            things.push(Spot {
                index,
                x: width * MARGIN + usable * spot.clamp(0.0, 1.0),
                y: feet,
            });
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

    // Who is indoors this hour, and who is still on their way somewhere.
    let mut inside = Vec::new();
    let mut leaving = BTreeMap::<usize, (usize, f32)>::new();
    for (index, item) in items.iter().enumerate() {
        if item.kind != CanvasItemKind::Actor {
            continue;
        }
        let Some((now, before)) = routine(index) else {
            continue;
        };
        let (Some(to), Some(from)) = (placed(now.at), placed(before.at)) else {
            continue;
        };
        let (Some(to_x), Some(from_x)) = (slot_x.get(&to), slot_x.get(&from)) else {
            continue;
        };
        let walk = (to_x - from_x).abs() / (figure_h * STROLL);
        let walking = before.at != now.at && clock.into_hour < walk;
        if walking {
            leaving.insert(index, (from, clock.into_hour));
        } else if now.inside {
            inside.push((index, to));
        }
    }
    let indoors = inside
        .iter()
        .map(|(person, _)| *person)
        .collect::<BTreeSet<_>>();

    // People stand in front of wherever they are, in a row centred on it,
    // pairs side by side; beside a thing rather than in front of it.
    let spacing = figure_h * 0.66;
    let pairs = pairs(snapshot);
    let mut people = Vec::new();
    let mut hosted = BTreeMap::<usize, Vec<usize>>::new();
    let mut loose = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if item.kind != CanvasItemKind::Actor || indoors.contains(&index) {
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
        let x = along(&items[index])
            .unwrap_or_else(|| width * MARGIN + usable * items[index].x.clamp(0.0, 1.0));
        people.push(Spot { index, x, y: feet });
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
    // A crowd stands at more than one depth: every other person in a row,
    // and anyone standing on their own, a step nearer or further, so the
    // place reads as ground rather than a line.
    let step = figure_h * 0.2;
    for (position, spot) in people.iter_mut().enumerate() {
        let seed = art::seed_of(&items[spot.index].id.stable_key());
        let depth = match position % 3 {
            0 => 0.0,
            1 => 1.0,
            _ => 0.5,
        } + (seed % 7) as f32 / 30.0;
        spot.y = feet + step * depth;
    }
    people.sort_by_key(|spot| spot.index);
    let routes = leaving
        .into_iter()
        .filter_map(|(person, (from, since))| Some((person, (*slot_x.get(&from)?, since))))
        .collect();

    Stage {
        width,
        height,
        view_w,
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
        inside,
        routes,
    }
}

/// How long someone takes to walk to where a turn put them.
pub const WALK_SECONDS: f32 = 1.4;

pub(crate) fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A spring let go from 1 toward 0, `t` seconds on: how much of the
/// displacement is left, overshooting a little before it settles. Coats,
/// hair and a landing's squash all settle on it.
pub(crate) fn settle(t: f32) -> f32 {
    const SWING: gpui::SpringConfig = gpui::SpringConfig::new(170.0, 9.0, 1.0);
    if t <= 0.0 {
        return 1.0;
    }
    SWING
        .step(
            gpui::SpringState {
                position: 1.0,
                velocity: 0.0,
            },
            0.0,
            t.min(4.0),
        )
        .position
}

/// Where someone is this frame, and how they stand: presentation only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Living {
    pub x: f32,
    pub pose: Pose,
    /// What they do with an idle moment, or how they answer a click, over
    /// whatever their Pack says they are doing.
    pub stance: Option<Stance>,
}

/// How long a building or thing springs after a click, in seconds.
pub const BOUNCE_SECONDS: f32 = 0.5;

/// How long a wave lasts after someone is clicked on, in seconds.
pub const WAVE_SECONDS: f32 = 1.2;

/// What someone does with an idle moment `seconds` in: most of the time
/// nothing much, and now and then, on a cycle of their own, one of the
/// idle things people do. At night they only sit or look about.
fn idle(seed: u32, seconds: f32, daylight: Daylight) -> Option<Stance> {
    let period = 16.0 + (seed % 9) as f32;
    let shifted = seconds + (seed % 997) as f32 * 0.53;
    let phase = (shifted % period) / period;
    if !(0.55..0.82).contains(&phase) {
        return None;
    }
    let turn = (shifted / period) as u32 + seed;
    let choices: &[Stance] = if daylight == Daylight::Night {
        &[Stance::Sitting, Stance::LookingAround]
    } else {
        &Stance::IDLE
    };
    Some(choices[(turn as usize) % choices.len()])
}

/// Everyone clicked on in the last [`WAVE_SECONDS`] waves and hops, from
/// how long ago each was clicked: a crouch, a stretched hop, a squashed
/// landing that springs back. `still` (Reduce Motion) keeps the wave and
/// leaves out the hop.
pub fn wave(
    living: &mut [Living],
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    poked: &BTreeMap<SelectionId, f32>,
    still: bool,
) {
    for (life, spot) in living.iter_mut().zip(&stage.people) {
        let Some(item) = snapshot.canvas.items.get(spot.index) else {
            continue;
        };
        let Some(ago) = poked.get(&item.id) else {
            continue;
        };
        if !(0.0..WAVE_SECONDS).contains(ago) || life.pose.stride.is_some() {
            continue;
        }
        life.stance = Some(Stance::Waving);
        if still {
            continue;
        }
        let (crouch, air) = (0.09, 0.42);
        if *ago < crouch {
            life.pose.squash = 1.0 - 0.1 * ease(ago / crouch);
        } else if *ago < crouch + air {
            let u = (ago - crouch) / air;
            let hop = (u * std::f32::consts::PI).sin();
            life.pose.bob += hop * stage.figure_h * 0.18;
            // Stretched going up and coming down, round at the top.
            life.pose.squash = 1.0 + 0.08 * (u * std::f32::consts::PI).cos().abs();
        } else {
            life.pose.squash = 1.0 - 0.12 * settle(ago - crouch - air);
        }
    }
}

/// Where each person is this frame, `seconds` into looking at the World.
///
/// Between turns everyone goes about their day: each on a cycle of their
/// own, most of it at home, part of it walking over to another place and
/// back. At night nobody wanders. Anyone `pinned` (speaking, being asked,
/// in the news) stays where they are. `walking` is how far through the walk
/// a turn started is, from where `before` had them. Someone whose day has
/// just taken them somewhere else strolls there.
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
    let figure_h = stage.figure_h;
    // Walking from `from` to `to`, `since` seconds of `length` in: planted
    // steps, a lean into the walk, a coat that swings as they start and
    // stop.
    let walk = |from: f32, to: f32, since: f32, length: f32| {
        let t = since / length.max(0.01);
        let facing = (to - from).signum();
        let phase = (since * 1.7).rem_euclid(1.0);
        let start = 1.0 - settle(since);
        let lean = facing * (0.05 * start + 0.018 * (phase * std::f32::consts::TAU * 2.0).sin());
        Living {
            x: from + (to - from) * ease(t),
            pose: Pose {
                stride: Some(phase),
                bob: (phase * std::f32::consts::TAU).sin().abs() * figure_h * 0.035,
                facing,
                squash: 1.0 - 0.03 * (phase * std::f32::consts::TAU * 2.0).cos().max(0.0),
                lean,
            },
            stance: None,
        }
    };
    // Just arrived: the coat swings on past the stop and settles back.
    let arrived = |life: Living, since: f32, facing: f32| Living {
        pose: Pose {
            lean: facing * 0.05 * settle(since),
            ..life.pose
        },
        ..life
    };
    stage
        .people
        .iter()
        .map(|spot| {
            let item = &items[spot.index];
            let key = item.id.stable_key();
            let seed = art::seed_of(&key);
            let home = spot.x;
            let breathe = (seconds * 1.7 + (seed % 100) as f32 * 0.07).sin();
            // Walking over to where a turn put them.
            if let Some((old_stage, old_snapshot, progress)) = before {
                if let Some(old) = old_stage.person(item.id, old_snapshot) {
                    if (old.x - home).abs() > 2.0 {
                        let since = progress * WALK_SECONDS;
                        if progress < 1.0 {
                            return walk(old.x, home, since, WALK_SECONDS);
                        }
                        if since < WALK_SECONDS + 1.5 {
                            let still = Living {
                                x: home,
                                pose: Pose::default(),
                                stance: None,
                            };
                            return arrived(still, since - WALK_SECONDS, (home - old.x).signum());
                        }
                    }
                }
            }
            // On the way to where their day has them this hour.
            if let Some((from, since)) = stage.routes.get(&spot.index) {
                let length = (home - from).abs() / (figure_h * STROLL);
                if *since < length {
                    return walk(*from, home, *since, length);
                }
            }
            let still = Living {
                x: home + (seconds * 0.23 + seed as f32).sin() * 3.0,
                pose: Pose {
                    stride: None,
                    bob: breathe * 0.6,
                    facing: ((seconds * 0.11 + (seed % 7) as f32).sin() * 1.4).clamp(-1.0, 1.0),
                    // Breathing: a touch taller on the breath in.
                    squash: 1.0 + 0.008 * breathe,
                    lean: 0.0,
                },
                stance: if pinned.contains(&item.id) {
                    None
                } else {
                    idle(seed, seconds, daylight)
                },
            };
            if pinned.contains(&item.id) || daylight == Daylight::Night || stops.len() < 2 {
                return still;
            }
            // A visit: out to another place, a while there, and home.
            let period = 34.0 + (seed % 17) as f32;
            let into = (seconds + (seed % 1000) as f32 * 0.37) % period;
            let phase = into / period;
            let nearest = stops
                .iter()
                .enumerate()
                .min_by(|a, b| (a.1 - home).abs().total_cmp(&(b.1 - home).abs()))
                .map(|(position, _)| position)
                .unwrap_or(0);
            let others = stops.len() - 1;
            let pick = (nearest + 1 + (seed as usize / 7) % others) % stops.len();
            // Along a panorama, only as far as the next few places.
            let away = stops[pick] + ((seed % 5) as f32 - 2.0) * stage.figure_h * 0.25;
            let away = away.clamp(home - stage.view_w * 0.45, home + stage.view_w * 0.45);
            let leg = period * 0.08;
            match phase {
                p if (0.60..0.68).contains(&p) => walk(home, away, into - period * 0.60, leg),
                p if (0.68..0.80).contains(&p) => arrived(
                    Living {
                        x: away,
                        stance: Some(Stance::LookingAround),
                        ..still
                    },
                    into - period * 0.68,
                    (away - home).signum(),
                ),
                p if (0.80..0.88).contains(&p) => walk(away, home, into - period * 0.80, leg),
                p if (0.88..0.93).contains(&p) => {
                    arrived(still, into - period * 0.88, (home - away).signum())
                }
                _ => still,
            }
        })
        .collect()
}

/// Where the camera looks: `zoom` times closer, centred on (`x`, `y`) in
/// stage pixels. At rest it looks at the whole stage, or along a panorama
/// one window of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
}

/// The closest the camera goes.
pub const ZOOM_MOST: f32 = 2.2;

/// The camera's closest and furthest along one axis: a view `half` wide
/// either side kept on a stage `extent` long, or centred when it is wider.
fn keep_on(at: f32, half: f32, extent: f32) -> f32 {
    if half * 2.0 >= extent {
        extent / 2.0
    } else {
        at.clamp(half, extent - half)
    }
}

/// The top of the postcard the place becomes when zoomed out, in stage
/// pixels: a strip of sky above the horizon.
fn card_top(horizon: f32, height: f32) -> f32 {
    horizon - height * 0.2
}

/// The paper round the panoramic postcard, when zoomed out far enough to
/// show one: above and below it, with a soft edge where the card lies on
/// the paper.
fn paint_card(window: &mut dyn Brush, frame: &Frame, ox: f32, oy: f32, width: f32, height: f32) {
    if frame.camera.zoom >= 0.999 {
        return;
    }
    let top = frame
        .at(0.0, card_top(frame.horizon, frame.height))
        .1
        .max(0.0);
    let bottom = frame.at(0.0, frame.height).1.min(height);
    if top <= 0.5 && bottom >= height - 0.5 {
        return;
    }
    let paper = art::hex(0xf2eee6);
    let shadow = gpui::black().opacity(0.14);
    window.rect(ox, oy, width, top, 0.0, paper);
    window.rect(ox, oy + bottom, width, height - bottom, 0.0, paper);
    // The card's edges: a hairline, and a soft shadow on the paper below.
    window.gradient(
        ox,
        oy + bottom,
        width,
        (height - bottom).min(14.0),
        180.0,
        (shadow, 0.0),
        (shadow.opacity(0.0), 1.0),
    );
    window.rect(
        ox,
        oy + top - 0.5,
        width,
        1.0,
        0.0,
        gpui::white().opacity(0.8),
    );
    window.rect(
        ox,
        oy + bottom - 0.5,
        width,
        1.0,
        0.0,
        gpui::white().opacity(0.8),
    );
}

impl Camera {
    /// The middle of the place, at the window's own size.
    pub fn whole(stage: &Stage) -> Self {
        Self {
            zoom: 1.0,
            x: stage.width / 2.0,
            y: stage.height / 2.0,
        }
    }

    /// The furthest out the camera goes: one window, or for a panorama the
    /// whole of it at once.
    pub fn least(stage: &Stage) -> f32 {
        (stage.view_w / stage.width.max(1.0)).min(1.0)
    }

    /// Close enough on a box to see who is in it, and no closer than twice.
    pub fn on(stage: &Stage, (x, y, w, h): (f32, f32, f32, f32)) -> Self {
        let room = (stage.view_w * 0.55 / w.max(1.0)).min(stage.height * 0.45 / h.max(1.0));
        let zoom = room.clamp(1.0, 1.8);
        // Keep the view inside the stage: never show past its edges.
        let half_w = stage.view_w / zoom / 2.0;
        let half_h = stage.height / zoom / 2.0;
        Self {
            zoom,
            x: keep_on(x + w / 2.0, half_w, stage.width),
            // Frame a little above centre, so the card below does not
            // cover what is being looked at.
            y: keep_on(y + h * 0.7, half_h, stage.height),
        }
    }

    /// `zoom` times closer around a stage point, kept inside the stage.
    /// Zoomed out past one window, the ground stays at the bottom and the
    /// sky opens above it.
    pub fn around(stage: &Stage, zoom: f32, x: f32, y: f32) -> Self {
        let zoom = zoom.clamp(Self::least(stage), ZOOM_MOST);
        let half_w = stage.view_w / zoom / 2.0;
        let half_h = stage.height / zoom / 2.0;
        Self {
            zoom,
            x: keep_on(x, half_w, stage.width),
            // Zoomed out past one window the place becomes a panoramic
            // postcard: the town, a strip of sky over it and the water
            // under it, centred, on the paper around it.
            y: if half_h * 2.0 >= stage.height {
                (card_top(stage.horizon, stage.height) + stage.height) / 2.0
            } else {
                y.clamp(half_h, stage.height - half_h)
            },
        }
    }

    /// The stage point under a screen point.
    pub fn stage_point(&self, stage: &Stage, x: f32, y: f32) -> (f32, f32) {
        (
            (x - stage.view_w / 2.0) / self.zoom + self.x,
            (y - stage.height / 2.0) / self.zoom + self.y,
        )
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
            (x - self.x) * self.zoom + stage.view_w / 2.0,
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
    /// Their Pack's drawing of them, if it ships one.
    pub drawing: Option<Drawing>,
    /// What they are doing, for their drawing.
    pub stance: Stance,
    /// How they feel, for their face.
    pub mood: world_projection::Mood,
}

/// A building as painted, in stage pixels at zoom 1.
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
    drawing: Option<Drawing>,
    /// Springing after a click: wider, taller. `(1, 1)` at rest.
    squash: (f32, f32),
    /// Rising into place when just built, 0 to 1.
    grow: f32,
    /// Someone indoors: their colours, seen as a shape in a lit window.
    inside: Vec<Figure>,
}

impl BuildingPaint {
    /// Drawn live this frame rather than painted into the still layer.
    fn moving(&self) -> bool {
        self.grow < 1.0 || (self.squash.0 - 1.0).abs() > 1e-3 || (self.squash.1 - 1.0).abs() > 1e-3
    }
}

/// A thing as drawn, in stage pixels at zoom 1.
#[derive(Clone, Debug)]
struct ThingPaint {
    index: usize,
    x: f32,
    base: f32,
    w: f32,
    shape: MarkShape,
    palette: Palette,
    sway: f32,
    /// A boat's roll on the swell, in radians.
    roll: f32,
    glow: Option<Hsla>,
    drawing: Option<Drawing>,
    grow: f32,
}

/// A built thing on the far ridge: where along it (0 to 1), its shape, and
/// how far it has risen.
#[derive(Clone, Copy, Debug)]
struct RidgeMark {
    along: f32,
    shape: MarkShape,
    grow: f32,
}

/// Something the World is working toward, on the ridge.
#[derive(Clone, Copy, Debug)]
struct RidgeGoal {
    along: f32,
    w: f32,
    h: f32,
    shape: MarkShape,
    done: u32,
    parts: u32,
}

/// Everything one frame of the stage draws, worked out before drawing so
/// the painting itself only paints.
#[derive(Clone, Debug)]
pub struct Frame {
    scenery: Scenery,
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
    camera: Camera,
    /// The stage's size and lines, in stage pixels.
    width: f32,
    view_w: f32,
    height: f32,
    horizon: f32,
    base: f32,
    front: f32,
    building_h: f32,
    water: bool,
    marks: Vec<RidgeMark>,
    goals: Vec<RidgeGoal>,
    buildings: Vec<BuildingPaint>,
    things: Vec<ThingPaint>,
    pub people: Vec<PersonPaint>,
    bonds: Vec<(f32, f32, f32, CanvasLinkTone)>,
}

impl Frame {
    /// The same frame graded for `hour` whatever the clock says, so a test
    /// picture never depends on when it is drawn.
    #[cfg(test)]
    pub(crate) fn at_hour(mut self, hour: f32) -> Self {
        self.hour = hour;
        self
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
        (
            (x - self.camera.x) * self.camera.zoom + self.view_w / 2.0,
            (y - self.camera.y) * self.camera.zoom + self.height / 2.0,
        )
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
    let shown_goals = &snapshot.goals[snapshot.goals.len().saturating_sub(GOALS_SHOWN)..];
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
    let buildings = stage
        .buildings
        .iter()
        .map(|spot| {
            let item = &items[spot.index];
            let shape = item.shape.unwrap_or_default();
            // Something wide stands a little lower and a tower taller.
            let (w, h) = match shape {
                MarkShape::Bridge => (stage.building_w * 1.3, stage.building_h * 0.7),
                MarkShape::Tower => (stage.building_w, stage.building_h * 1.2),
                MarkShape::Dome | MarkShape::Tree => (stage.building_w, stage.building_h * 0.8),
                _ => (stage.building_w, stage.building_h),
            };
            // A Pack's own drawing keeps its own proportions, no wider
            // than a building's place allows.
            // Further back is a little smaller.
            let recede = 1.0 - ((stage.base - spot.y) / stage.building_h).max(0.0) * 0.6;
            let (w, h) = (w * recede, h * recede);
            let drawing = snapshot.drawing_of(item).cloned();
            let (w, h) = match &drawing {
                Some(drawing) => {
                    let w = (h * drawing.aspect).min(stage.building_w * 1.5);
                    (w, w / drawing.aspect)
                }
                None => (w, h),
            };
            BuildingPaint {
                index: spot.index,
                x: spot.x,
                base: spot.y,
                w,
                h,
                shape,
                palette: Palette::of(&item.id.stable_key(), lit),
                glow: glow_of(item),
                drawing,
                squash: (1.0, 1.0),
                grow: 1.0,
                inside: inside.remove(&spot.index).unwrap_or_default(),
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
            let w = match shape {
                MarkShape::Parcel => stage.thing_w * 0.4,
                _ => stage.thing_w,
            };
            ThingPaint {
                index: spot.index,
                x: spot.x,
                base: spot.y + bob,
                w,
                shape,
                palette: Palette::of(&key, lit),
                sway,
                roll,
                glow: glow_of(item),
                drawing: snapshot.drawing_of(item).cloned(),
                grow: 1.0,
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
            // Nearer is a little bigger.
            let near = 1.0 + (spot.y - stage.feet) / stage.figure_h * 0.35;
            PersonPaint {
                index: spot.index,
                x,
                y,
                height: stage.figure_h * z * near,
                figure: Figure::of(&item.id.stable_key(), item.look),
                pose: Pose {
                    bob: life.pose.bob * z,
                    ..life.pose
                },
                glow: glow_of(item),
                drawing: snapshot.drawing_of(item).cloned(),
                // Walking beats everything; a wave beats what the Pack says;
                // what the Pack says (talking, celebrating, working) beats an
                // idle moment.
                stance: match (life.pose.stride, life.stance, item.stance) {
                    (Some(_), _, _) => Stance::Walking,
                    (None, Some(Stance::Waving), _) => Stance::Waving,
                    (None, _, Some(pack)) if pack != Stance::Standing => pack,
                    (None, Some(idle), _) => idle,
                    _ => Stance::Standing,
                },
                mood: item.mood.unwrap_or_default(),
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
        daylight,
        hour,
        seconds,
        still: false,
        weather: snapshot.weather,
        season: snapshot.canvas.season,
        cover: snapshot.canvas.ground,
        ice: snapshot.canvas.ice,
        camera,
        width: stage.width,
        view_w: stage.view_w,
        height: stage.height,
        horizon: stage.horizon,
        base: stage.base,
        front: stage.front,
        building_h: stage.building_h,
        water,
        marks,
        goals,
        buildings,
        things,
        people,
        bonds,
    }
}

/// How many of what the World works towards stand on the ridge.
const GOALS_SHOWN: usize = 5;

/// How much each layer moves with the camera, from the sky (least) to
/// the ground under people's feet (fully).
pub const PARALLAX: [f32; 4] = [0.06, 0.18, 0.4, 1.0];

/// The light an hour lays over the whole scene, like a diorama under a
/// lamp: a warm key light from the upper left and a cool shade low down,
/// each a colour and how strong it is.
pub fn grade(daylight: Daylight) -> ((u32, f32), (u32, f32)) {
    match daylight {
        Daylight::Dawn => ((0xffc79a, 0.16), (0x5a6aa8, 0.10)),
        Daylight::Day => ((0xfff0c8, 0.10), (0x4a6a9a, 0.08)),
        Daylight::Dusk => ((0xff9a5c, 0.20), (0x4a3070, 0.16)),
        Daylight::Night => ((0x9ab0ff, 0.06), (0x0a1030, 0.22)),
    }
}

/// Where each part of the day's light is at its fullest.
const ANCHORS: [(f32, Daylight); 7] = [
    (0.0, Daylight::Night),
    (4.5, Daylight::Night),
    (6.5, Daylight::Dawn),
    (12.5, Daylight::Day),
    (19.0, Daylight::Dusk),
    (21.5, Daylight::Night),
    (24.0, Daylight::Night),
];

/// Which two parts of the day an hour lies between, and how far from the
/// first to the second (eased).
fn between(hour: f32) -> (Daylight, Daylight, f32) {
    let hour = hour.rem_euclid(24.0);
    let next = ANCHORS
        .iter()
        .position(|(at, _)| *at > hour)
        .unwrap_or(ANCHORS.len() - 1)
        .max(1);
    let (from_at, from) = ANCHORS[next - 1];
    let (to_at, to) = ANCHORS[next];
    let t = ((hour - from_at) / (to_at - from_at).max(0.01)).clamp(0.0, 1.0);
    (from, to, ease(t))
}

/// The light an hour lays over the scene, eased from one part of the day
/// into the next so no two hours look alike: the warm key light and the
/// cool shade of [`grade`], each a colour and how strong it is.
pub fn grade_at(hour: f32) -> ((u32, f32), (u32, f32)) {
    let hour = hour.rem_euclid(24.0);
    // `between` has already eased `t`: ease(t).
    let (from, to, t) = between(hour);
    let mix = |a: (u32, f32), b: (u32, f32), deep: f32| {
        let channel = |shift: u32| {
            let x = ((a.0 >> shift) & 0xff) as f32;
            let y = ((b.0 >> shift) & 0xff) as f32;
            ((x + (y - x) * t).round() as u32).min(255) << shift
        };
        (
            channel(16) | channel(8) | channel(0),
            a.1 + (b.1 - a.1) * t + deep,
        )
    };
    let (warm_a, cool_a) = grade(from);
    let (warm_b, cool_b) = grade(to);
    // Through the night the dark deepens towards the small hours and the
    // shade slowly turns from navy to the indigo before dawn.
    let (night_deep, night_turn) = if from == Daylight::Night && to == Daylight::Night {
        let into = if hour >= 21.5 {
            hour - 21.5
        } else {
            hour + 2.5
        } / 7.0;
        (0.05 * (1.0 - (into * 2.0 - 1.0).abs()), into)
    } else {
        (0.0, 0.0)
    };
    let (cool, cool_alpha) = mix(cool_a, cool_b, night_deep);
    let cool = if night_turn > 0.0 {
        let blue = (cool & 0xff) as f32 + 40.0 * night_turn;
        let red = ((cool >> 16) & 0xff) as f32 + 24.0 * night_turn;
        (cool & 0x00ff00) | ((red.min(255.0) as u32) << 16) | blue.min(255.0) as u32
    } else {
        cool
    };
    (mix(warm_a, warm_b, 0.0), (cool, cool_alpha))
}

/// How the hour and the weather colour everything lit by them, as a
/// multiplier per channel (red, green, blue): white at noon, gold at dusk,
/// moonlit blue at night, greyer under rain.
pub fn light_at(hour: f32, weather: Weather) -> [f32; 3] {
    let of = |daylight: Daylight| match daylight {
        Daylight::Dawn => [1.0, 0.9, 0.86],
        Daylight::Day => [1.0, 0.995, 0.97],
        Daylight::Dusk => [1.0, 0.85, 0.74],
        Daylight::Night => [0.34, 0.4, 0.6],
    };
    // `between` has already eased `t`: ease(t).
    let (from, to, t) = between(hour);
    let (a, b) = (of(from), of(to));
    let sky = [0, 1, 2].map(|channel| a[channel] + (b[channel] - a[channel]) * t);
    let air = match weather {
        Weather::Clear => [1.0, 1.0, 1.0],
        Weather::Cloudy => [0.93, 0.94, 0.96],
        Weather::Rain => [0.8, 0.83, 0.88],
        Weather::Storm => [0.6, 0.64, 0.72],
        Weather::Snow => [0.95, 0.97, 1.0],
        Weather::Fog => [0.94, 0.95, 0.96],
        Weather::Dust => [1.0, 0.84, 0.7],
    };
    [0, 1, 2].map(|channel| sky[channel] * air[channel])
}

/// Where the light comes from at `hour`: across (-1 from the left, the
/// morning's east, to 1 from the right) and how high the sun is (0 on the
/// horizon to 1 overhead; below 0 it has set).
pub fn sun_at(hour: f32) -> (f32, f32) {
    let day = ((hour - 5.5) / 14.5).clamp(-0.2, 1.2);
    let across = (day * 2.0 - 1.0).clamp(-1.0, 1.0);
    let high = (day * std::f32::consts::PI).sin();
    (across, high)
}

/// How many device pixels a tile of the ground layer is on a side.
const TILE: u32 = 256;
/// Tiles are painted this many of their own pixels larger each side, so
/// where they meet the display blends real neighbours.
const LAND_PAD: u32 = 1;
/// The sky is soft all over, so it is painted at half the display's
/// resolution.
const SKY_RES: f32 = 0.3;
/// How much the far hills move with the camera along a panorama.
const HILLS: f32 = 0.3;
/// The colour of a window lit from inside.
const LAMPLIGHT: (u8, u8, u8) = (0xff, 0xd2, 0x7a);

/// A colour between `a` and `b`, `t` of the way, mixed as light is.
fn mix(a: Hsla, b: Hsla, share: f32) -> Hsla {
    let (a, b): (gpui::Rgba, gpui::Rgba) = (a.into(), b.into());
    let share = share.clamp(0.0, 1.0);
    gpui::Rgba {
        r: a.r + (b.r - a.r) * share,
        g: a.g + (b.g - a.g) * share,
        b: a.b + (b.b - a.b) * share,
        a: a.a + (b.a - a.a) * share,
    }
    .into()
}

/// Whether the sun (or the moon) can be seen for the weather.
fn sun_out(weather: Weather) -> bool {
    matches!(weather, Weather::Clear | Weather::Cloudy)
}

/// Everything the still layers look like besides where things are: the
/// place's colours, the hour to the quarter, the weather and the season.
fn look_key(frame: &Frame, key: &mut Key) {
    let s = frame.scenery;
    key.add((s.sky_top, s.sky_bottom, s.far, s.near, s.sun))
        .add(frame.daylight as u8)
        .add((frame.hour * 4.0).floor() as i32)
        .add(frame.weather as u8)
        .add(frame.season.map(|season| season as u8))
        .add(frame.cover.map(|cover| cover as u8))
        .add((frame.ice, frame.water));
}

/// The still layers of a scene, back to front.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Still {
    Sky,
    Hills,
    Land,
    Buildings,
}

/// One version of a still layer: what it looks like (its key) and the
/// scale it is painted at.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Version {
    key: u64,
    scale: f32,
}

/// Work that paints one picture, off the window's thread.
type Job = Box<dyn FnOnce() -> Option<sk::Pixmap> + Send + 'static>;

/// A rectangle relative to the scene's top-left: x, y, width, height.
type Rect = (f32, f32, f32, f32);

/// One picture of a still layer: its key, where it is shown, the part of
/// it that shows, and how to paint it.
struct Piece {
    key: u64,
    rect: Rect,
    clip: Rect,
    job: Job,
}

/// Something a still layer draws this frame.
#[derive(Clone)]
enum Drawn {
    Image(std::sync::Arc<gpui::RenderImage>, Rect, Rect),
    /// A plain colour while a tile is being painted.
    Fill(Rect, Hsla),
    /// The sky's two colours while the sky is being painted.
    Sky(Rect, Hsla, Hsla),
}

/// What a still layer draws this frame: the version settled on, and the
/// next one fading in over it, and how far.
#[derive(Clone, Default)]
struct LayerPlan {
    settled: Vec<Drawn>,
    arriving: Vec<Drawn>,
    fade: f32,
}

/// How long a new version of a still layer takes to fade in.
const FADE: f32 = 0.3;

/// The version of each still layer a frame wants.
fn versions(
    frame: &Frame,
    window: &Window,
    width: f32,
    height: f32,
    dpr: f32,
) -> [(Still, Version); 4] {
    let mut sky = Key::new("sky");
    look_key(frame, &mut sky);
    sky.float(width)
        .float(height)
        .float(dpr)
        .float(frame.horizon);
    let band = Band::of(frame, width, height);
    let mut hills = Key::new("hills");
    look_key(frame, &mut hills);
    hills
        .float(band.w)
        .float(band.above)
        .float(band.below)
        .float(dpr)
        .float(frame.building_h);
    for mark in &frame.marks {
        hills
            .add((mark.shape as u8, mark.grow >= 1.0))
            .float(mark.along);
    }
    for goal in &frame.goals {
        hills
            .add((goal.shape as u8, goal.done, goal.parts))
            .float(goal.along);
    }
    let scale = painted_zoom(window, frame) * dpr;
    let ground = ground_key(frame, scale).finish();
    [
        (
            Still::Sky,
            Version {
                key: sky.finish(),
                scale: (dpr * SKY_RES).max(0.5),
            },
        ),
        (
            Still::Hills,
            Version {
                key: hills.finish(),
                scale: dpr,
            },
        ),
        (
            Still::Land,
            Version {
                key: ground,
                scale: scale * 0.5,
            },
        ),
        (Still::Buildings, Version { key: ground, scale }),
    ]
}

/// The pictures of one version of a still layer the camera sees, with
/// `margin` tiles more each side (to have them ready before a pan reaches
/// them).
#[allow(clippy::too_many_arguments)]
fn pieces(
    frame: &std::sync::Arc<Frame>,
    layer: Still,
    version: Version,
    width: f32,
    height: f32,
    dpr: f32,
    margin: i32,
) -> Vec<Piece> {
    match layer {
        Still::Sky => {
            let scale = version.scale;
            strips(width, scale)
                .map(|(index, x0, w)| {
                    let frame = frame.clone();
                    let pad = 1.0 / scale;
                    let mut key = Key::new("sky strip");
                    key.add((version.key, index));
                    Piece {
                        key: key.finish(),
                        rect: (x0 - pad, 0.0, w + pad * 2.0, height),
                        clip: (x0, 0.0, w + 1.0 / dpr, height),
                        job: Box::new(move || {
                            let mut canvas = Canvas::new(
                                (w * scale).ceil() as u32 + 2,
                                (height * scale).ceil() as u32,
                                scale,
                                (x0 - pad, 0.0),
                            )?;
                            let at = ((x0 * scale).round() as i32 - 1, 0);
                            paint_sky_on(&mut canvas, at, &frame, width, height);
                            Some(canvas.pixmap)
                        }),
                    }
                })
                .collect()
        }
        Still::Hills => {
            let band = Band::of(frame, width, height);
            let (bx, by) = band.screen(frame);
            let squash = Band::squash(frame);
            let scale = (dpr * 0.5).max(1.0);
            let tall = band.above + band.below;
            strips(band.w, scale)
                .map(|(index, x0, w)| {
                    let frame = frame.clone();
                    let band = Band::of(&frame, width, height);
                    let pad = 1.0 / scale;
                    let mut key = Key::new("hills strip");
                    key.add((version.key, index));
                    Piece {
                        key: key.finish(),
                        rect: (bx + x0 - pad, by, w + pad * 2.0, tall * squash),
                        clip: (bx + x0, by, w + 1.0 / dpr, tall * squash),
                        job: Box::new(move || {
                            let mut canvas = Canvas::new(
                                (w * scale).ceil() as u32 + 2,
                                (tall * scale).ceil() as u32,
                                scale,
                                (x0 - pad, 0.0),
                            )?;
                            let at = ((x0 * scale).round() as i32 - 1, 0);
                            paint_band_on(&mut canvas, at, &frame, &band);
                            Some(canvas.pixmap)
                        }),
                    }
                })
                .collect()
        }
        Still::Land | Still::Buildings => {
            let layer = if layer == Still::Land {
                Layer::Land
            } else {
                Layer::Buildings
            };
            let scale = version.scale;
            let zoom = frame.camera.zoom;
            let side = TILE as f32 / scale * zoom;
            let (sx, sy) = frame.at(0.0, 0.0);
            let (sx, sy) = ((sx * dpr).round() / dpr, (sy * dpr).round() / dpr);
            let pad = LAND_PAD as f32 * side / TILE as f32;
            // The land, which is solid, runs a device pixel on under the
            // next tile's edge, so no seam of half-covered pixels shows;
            // the buildings, which are mostly clear, must not, or where
            // they overlap a shadow would be laid twice.
            let over = if layer == Layer::Land { 1.0 / dpr } else { 0.0 };
            let reach = frame.at(0.0, layer_rows(frame, layer).0).1.floor();
            tiles_in_view(frame, layer, scale, margin)
                .into_iter()
                .map(|(column, row)| {
                    let mut key = Key::new("tile");
                    key.add((version.key, layer, column, row));
                    let (x, y) = (sx + column as f32 * side, sy + row as f32 * side);
                    let frame = frame.clone();
                    let ground = version.key;
                    Piece {
                        key: key.finish(),
                        rect: (x - pad, y - pad, side + pad * 2.0, side + pad * 2.0),
                        clip: {
                            // Nothing of a tile shows above where its layer
                            // begins: no empty tile edge ever lies over the sky.
                            let top = y.max(reach);
                            (x, top, side + over, (y + side + over - top).max(0.0))
                        },
                        job: Box::new(move || match layer {
                            Layer::Land => paint_land_tile(&frame, column, row, scale),
                            Layer::Buildings => {
                                paint_building_tile(&frame, ground, column, row, scale)
                            }
                        }),
                    }
                })
                .collect()
        }
    }
}

/// How wide a strip of the sky or the hills is, in device pixels: wide
/// pictures go to the display a strip at a time.
const STRIP: f32 = 512.0;

/// The strips of a picture `width` units wide at `scale` device pixels to a
/// unit: each one's index, where it begins and how wide it is, in units.
fn strips(width: f32, scale: f32) -> impl Iterator<Item = (i32, f32, f32)> {
    let each = STRIP / scale;
    let count = (width / each).ceil().max(1.0) as i32;
    (0..count).map(move |index| {
        let x0 = index as f32 * each;
        (index, x0, each.min(width - x0))
    })
}

/// What a still layer shows where its picture is not painted yet.
fn stand_in(frame: &Frame, layer: Still, rect: Rect, light: [f32; 3]) -> Vec<Drawn> {
    let lit = |colour: Hsla| {
        let rgba: gpui::Rgba = colour.into();
        Hsla::from(gpui::Rgba {
            r: rgba.r * light[0],
            g: rgba.g * light[1],
            b: rgba.b * light[2],
            a: rgba.a,
        })
    };
    match layer {
        Still::Sky => {
            let (top, bottom) = sky_colours(frame);
            vec![Drawn::Sky(rect, top, bottom)]
        }
        Still::Land => {
            // The field's colour down to the water's edge, and the water's
            // (or the near ground's) below.
            let (ground, near) = land_colours(frame);
            let field = frame
                .at(0.0, frame.horizon + (frame.base - frame.horizon) * 0.3)
                .1;
            let front = frame.at(0.0, frame.front).1;
            let (top, bottom) = (rect.1, rect.1 + rect.3);
            let mut out = Vec::new();
            let (a, b) = (top.max(field), bottom.min(front));
            if b > a {
                out.push(Drawn::Fill((rect.0, a, rect.2, b - a), lit(ground)));
            }
            let a = top.max(front);
            if bottom > a {
                out.push(Drawn::Fill((rect.0, a, rect.2, bottom - a), lit(near)));
            }
            out
        }
        Still::Hills | Still::Buildings => Vec::new(),
    }
}

/// Where each still layer of the scene stands, per window and size: the
/// version shown, and the next one fading in with when it began.
type Slots = std::collections::HashMap<
    (u64, Still, u32, u32),
    (Option<Version>, Option<(Version, std::time::Instant)>),
>;

/// Works out what every still layer draws this frame, asking for whatever
/// is not painted yet. `now` paints it all right here (a cover, a test);
/// otherwise it is painted off the window's thread and fades in when it is
/// ready, over what was there, except with Reduce Motion.
fn plan(
    frame: &std::sync::Arc<Frame>,
    window: &mut Window,
    width: f32,
    height: f32,
    now: bool,
) -> Vec<(Still, LayerPlan)> {
    use std::cell::RefCell;
    thread_local! {
        static SLOTS: RefCell<Slots> = RefCell::new(Slots::new());
    }
    let dpr = window.scale_factor().max(0.5);
    let light = light_at(frame.hour, frame.weather);
    let id = window.window_handle().window_id().as_u64();
    let instant = now || frame.still;
    painter::sweep(window);
    painter::begin_frame(!now);
    versions(frame, window, width, height, dpr)
        .into_iter()
        .map(|(layer, wanted)| {
            // Ask for what the camera sees, and a tile more each side.
            let margin = if now { 0 } else { 1 };
            let mut complete = true;
            let seen = pieces(frame, layer, wanted, width, height, dpr, 0)
                .iter()
                .map(|piece| piece.key)
                .collect::<BTreeSet<_>>();
            for piece in pieces(frame, layer, wanted, width, height, dpr, margin) {
                if painter::ready(piece.key).is_none() {
                    painter::want(window, piece.key, now, piece.job);
                    if seen.contains(&piece.key) && painter::ready(piece.key).is_none() {
                        complete = false;
                    }
                }
            }
            let slot_key = (id, layer, width as u32, height as u32);
            let clock = std::time::Instant::now();
            let (shown, arriving) = SLOTS.with(|slots| {
                let mut slots = slots.borrow_mut();
                let slot = slots.entry(slot_key).or_insert((None, None));
                if slot.0 == Some(wanted) {
                    slot.1 = None;
                } else if complete {
                    if instant {
                        *slot = (Some(wanted), None);
                    } else {
                        match slot.1 {
                            Some((version, began)) if version == wanted => {
                                if clock.duration_since(began).as_secs_f32() >= FADE {
                                    *slot = (Some(wanted), None);
                                }
                            }
                            _ => slot.1 = Some((wanted, clock)),
                        }
                    }
                }
                *slot
            });
            let draw = |version: Version, stand_ins: bool| {
                pieces(frame, layer, version, width, height, dpr, 0)
                    .into_iter()
                    .flat_map(|piece| match painter::ready(piece.key) {
                        Some(painter::Ready::Image(image, _)) => {
                            vec![Drawn::Image(image, piece.rect, piece.clip)]
                        }
                        Some(painter::Ready::Empty) => Vec::new(),
                        None if stand_ins => stand_in(frame, layer, piece.clip, light),
                        None => Vec::new(),
                    })
                    .collect::<Vec<_>>()
            };
            let settled = match (shown, layer) {
                (Some(version), _) => draw(version, true),
                (None, Still::Sky) => stand_in(frame, layer, (0.0, 0.0, width, height), light),
                (None, Still::Land) => pieces(frame, layer, wanted, width, height, dpr, 0)
                    .into_iter()
                    .flat_map(|piece| stand_in(frame, layer, piece.clip, light))
                    .collect(),
                (None, _) => Vec::new(),
            };
            let (arriving, fade) = match arriving {
                Some((version, began)) => (
                    draw(version, false),
                    (clock.duration_since(began).as_secs_f32() / FADE).clamp(0.0, 1.0),
                ),
                None => (Vec::new(), 0.0),
            };
            (
                layer,
                LayerPlan {
                    settled,
                    arriving,
                    fade: ease(fade),
                },
            )
        })
        .collect()
}

/// Draws what a still layer plans, at the scene's top-left (`ox`, `oy`).
fn draw_still(window: &mut Window, bounds: Bounds<Pixels>, drawn: &[Drawn]) {
    let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let at = |(x, y, w, h): Rect| Bounds::new(point(px(ox + x), px(oy + y)), size(px(w), px(h)));
    for item in drawn {
        match item {
            Drawn::Image(image, rect, clip) => {
                let clip = at(*clip).intersect(&bounds);
                let started = std::time::Instant::now();
                let _ = window.paint_image(
                    clip,
                    at(*rect),
                    Corners::default(),
                    image.clone(),
                    0,
                    false,
                );
                painter::slowest("main: one image", started.elapsed());
            }
            Drawn::Fill(rect, colour) => {
                let clip = at(*rect).intersect(&bounds);
                window.paint_quad(gpui::fill(clip, *colour));
            }
            Drawn::Sky(rect, top, bottom) => {
                let r = at(*rect);
                window.paint_quad(gpui::fill(
                    r,
                    gpui::linear_gradient(
                        180.0,
                        gpui::linear_color_stop(*top, 0.0),
                        gpui::linear_color_stop(*bottom, 0.55),
                    ),
                ));
            }
        }
    }
}

/// The newest thing built rising on the ridge.
fn paint_rising(window: &mut dyn Brush, frame: &Frame, ox: f32, oy: f32, width: f32, height: f32) {
    let band = Band::of(frame, width, height);
    let (band_x, band_y) = band.screen(frame);
    let squash = Band::squash(frame);
    let light = light_at(frame.hour, frame.weather);
    for mark in frame.marks.iter().filter(|mark| mark.grow < 1.0) {
        let (x, base) = band.mark_at(frame, mark.along);
        let h = frame.building_h * 0.34 * squash;
        let w = h * 0.7;
        let far = art::hex(frame.scenery.far);
        let mut tinted = Tint::new(window, light);
        crate::ui::paint_mark(
            &mut tinted,
            Bounds::new(
                point(
                    px(ox + band_x + x - w / 2.0),
                    px(oy + band_y + base * squash - h * mark.grow),
                ),
                size(px(w), px(h * mark.grow)),
            ),
            mark.shape,
            art::shade(far, -0.3).opacity(0.35 + 0.65 * mark.grow),
            art::hex(frame.scenery.sun),
        );
    }
}

/// The vignette, cheap enough to paint right away.
fn paint_vignette_image(window: &mut Window, bounds: Bounds<Pixels>) {
    let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let dpr = window.scale_factor().max(0.5);
    let mut key = Key::new("vignette");
    key.float(width).float(height).float(dpr);
    if let Some(vignette) = painter::cached(window, key.finish(), || {
        paint_vignette(width, height, dpr * 0.25)
    }) {
        let _ = window.paint_image(bounds, bounds, Corners::default(), vignette, 0, false);
    }
}

/// Paints a frame into `bounds` all at once, painting whatever still layer
/// is not painted yet right here: for a cover, the strip, a postcard and
/// the golden pictures.
pub fn paint(frame: &Frame, bounds: Bounds<Pixels>, window: &mut Window) {
    let ox = f32::from(bounds.origin.x);
    let oy = f32::from(bounds.origin.y);
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width < 2.0 || height < 2.0 {
        return;
    }
    let frame = std::sync::Arc::new(frame.clone());
    let layers = plan(&frame, window, width, height, true);
    let light = light_at(frame.hour, frame.weather);
    for (layer, plan) in &layers {
        draw_still(window, bounds, &plan.settled);
        match layer {
            Still::Sky => paint_clouds(window, &frame, ox, oy, width, height),
            Still::Hills => paint_rising(window, &frame, ox, oy, width, height),
            _ => {}
        }
    }
    paint_live(window, &frame, ox, oy, width, height, light);
    paint_weather(
        window,
        &frame,
        ox,
        oy,
        width,
        height,
        (height / 848.0).clamp(0.3, 1.3),
    );
    paint_vignette_image(window, bounds);
    paint_card(window, &frame, ox, oy, width, height);
}

/// Live drawing over a scene: the window, the frame, and the scene's
/// top-left corner, width and height.
type LivePaint = dyn Fn(&mut Window, &Frame, f32, f32, f32, f32);

/// The scene as the World window shows it: the still layers painted off
/// the window's thread and faded in as they arrive, and over and between
/// them everything that moves, drawn live every frame. Nothing here waits
/// for a picture.
pub fn scene(frame: Frame, window: &mut Window) -> gpui::Div {
    use gpui::{ParentElement, Styled};
    let viewport = window.viewport_size();
    let width = frame.view_w;
    let height = frame.height;
    let _ = viewport;
    let frame = std::sync::Arc::new(frame);
    let layers = painter::timed("main: plan", || {
        plan(&frame, window, width, height, painter::synchronous())
    });
    // While pictures are being painted or fading in, keep drawing frames,
    // so each is shown as soon as it is ready, however still the window.
    if !painter::idle() || layers.iter().any(|(_, plan)| !plan.arriving.is_empty()) {
        window.request_animation_frame();
    }
    let still = |drawn: Vec<Drawn>| {
        gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window: &mut Window, _| {
                let started = std::time::Instant::now();
                painter::timed("main: draw still", || draw_still(window, bounds, &drawn));
                painter::note_frame(started.elapsed());
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    };
    let live = |paint: Box<LivePaint>| {
        let frame = frame.clone();
        gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window: &mut Window, _| {
                let started = std::time::Instant::now();
                paint(
                    window,
                    &frame,
                    f32::from(bounds.origin.x),
                    f32::from(bounds.origin.y),
                    f32::from(bounds.size.width),
                    f32::from(bounds.size.height),
                );
                painter::note_frame(started.elapsed());
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    };
    let mut root = gpui::div().absolute().top_0().left_0().size_full();
    // For looking into how the scene is made: `WORLD_GPUI_HIDE=hills,sky`
    // leaves those still layers out.
    let hidden = std::env::var("WORLD_GPUI_HIDE").unwrap_or_default();
    let layers = layers
        .into_iter()
        .filter(|(layer, _)| !hidden.contains(&format!("{layer:?}").to_lowercase()));
    for (layer, plan) in layers {
        root = root.child(still(plan.settled));
        if !plan.arriving.is_empty() {
            root = root.child(still(plan.arriving).opacity(plan.fade));
        }
        match layer {
            Still::Sky => {
                root = root.child(live(Box::new(|window, frame, ox, oy, w, h| {
                    paint_clouds(window, frame, ox, oy, w, h)
                })))
            }
            Still::Hills => {
                root = root.child(live(Box::new(|window, frame, ox, oy, w, h| {
                    paint_rising(window, frame, ox, oy, w, h)
                })))
            }
            _ => {}
        }
    }
    root.child(live(Box::new(|window, frame, ox, oy, w, h| {
        let light = light_at(frame.hour, frame.weather);
        paint_live(window, frame, ox, oy, w, h, light);
        paint_weather(window, frame, ox, oy, w, h, (h / 848.0).clamp(0.3, 1.3));
        let bounds = Bounds::new(point(px(ox), px(oy)), size(px(w), px(h)));
        if painter::synchronous() {
            paint_vignette_image(window, bounds);
        } else {
            // Not worth a hitch: until it is painted, there is none.
            let dpr = window.scale_factor().max(0.5);
            let mut key = Key::new("vignette");
            key.float(w).float(h).float(dpr);
            let key = key.finish();
            match painter::ready(key) {
                Some(painter::Ready::Image(image, _)) => {
                    let _ = window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
                }
                _ => painter::want(
                    window,
                    key,
                    false,
                    Box::new(move || paint_vignette(w, h, dpr * 0.25)),
                ),
            }
        }
        paint_card(window, frame, ox, oy, w, h);
    })))
}

/// The sky, graded for the hour and the weather, with the sun where the
/// hour has it, or the moon and stars.
#[cfg(test)]
fn paint_sky(frame: &Frame, width: f32, height: f32, scale: f32) -> Option<sk::Pixmap> {
    painter::in_strips(
        (width * scale).ceil() as u32,
        (height * scale).ceil() as u32,
        scale,
        (0.0, 0.0),
        &|canvas, at| painter::timed("sky", || paint_sky_on(canvas, at, frame, width, height)),
    )
}

/// The sky onto one strip of it.
fn paint_sky_on(canvas: &mut Canvas, at: (i32, i32), frame: &Frame, width: f32, height: f32) {
    let (top, bottom) = sky_colours(frame);
    let horizon = frame.height * HORIZON;
    painter::fill_shaded(
        canvas,
        &Shape::polygon(&[
            (-16.0, -16.0),
            (width + 16.0, -16.0),
            (width + 16.0, height + 16.0),
            (-16.0, height + 16.0),
        ]),
        (0.0, 0.0),
        (0.0, horizon * 1.2),
        &[(0.0, top), (1.0, bottom)],
    );
    let (across, high) = sun_at(frame.hour);
    let night = frame.daylight == Daylight::Night;
    let k = (height / 848.0).clamp(0.3, 1.3);
    if sun_out(frame.weather) {
        if night {
            // Stars, fewer toward the horizon, and the moon.
            for index in 0..90_i32 {
                let seed = painter::hash2(index, 7, 0x5eed);
                let x = (seed % 1000) as f32 / 1000.0 * width;
                let y = ((seed / 1000) % 1000) as f32 / 1000.0;
                let y = y * y * horizon * 0.95;
                let bright = 0.35 + ((seed >> 20) % 100) as f32 / 160.0;
                let r = (1.1 * k).max(0.55 / canvas.scale);
                art::circle(canvas, x, y, r, gpui::white().opacity(bright));
            }
            let (mx, my) = (width * 0.8, horizon * 0.34);
            painter::glow(
                canvas,
                mx,
                my,
                90.0 * k,
                90.0 * k,
                art::hex(0x8a9ad0).opacity(0.35),
            );
            canvas.soft(mx, my, 22.0 * k, 22.0 * k, 2.0, art::hex(0xf4f1e6));
            canvas.soft(
                mx + 8.0 * k,
                my - 5.0 * k,
                20.0 * k,
                20.0 * k,
                2.0,
                top.opacity(0.92),
            );
        } else if high > -0.08 {
            let sun = art::hex(frame.scenery.sun);
            let (sx, sy) = (
                width * (0.5 + 0.38 * across),
                horizon * (0.9 - 0.62 * high.max(0.0)),
            );
            // The lower the sun, the warmer and wider its light on the sky.
            let low = (1.0 - high.max(0.0)).powi(2);
            let dim = if frame.weather == Weather::Cloudy {
                0.55
            } else {
                1.0
            };
            let warm = mix(sun, art::hex(0xff9a5c), low * 0.7);
            painter::glow(
                canvas,
                sx,
                sy,
                width * (0.25 + 0.3 * low),
                horizon * (0.5 + 0.4 * low),
                warm.opacity((0.18 + 0.32 * low) * dim),
            );
            canvas.soft(sx, sy, 58.0 * k, 58.0 * k, 40.0 * k, sun.opacity(0.2 * dim));
            canvas.soft(sx, sy, 34.0 * k, 34.0 * k, 3.0, sun.opacity(dim));
        }
    }
    // Under weather the sky greys, or reddens in dust.
    if let Some((tint, alpha)) = overcast(frame.weather) {
        canvas.rect(
            -16.0,
            -16.0,
            width + 32.0,
            height + 32.0,
            0.0,
            art::hex(tint).opacity(alpha),
        );
    }
    painter::grain(canvas, at, 0.03, 0.022);
}

/// The sky's colours at its top and at the horizon, for the hour.
fn sky_colours(frame: &Frame) -> (Hsla, Hsla) {
    let top = art::hex(frame.scenery.sky_top);
    let bottom = art::hex(frame.scenery.sky_bottom);
    let tinted = |daylight: Daylight| {
        let (tint_top, tint_bottom) = match daylight {
            Daylight::Day => ((0xffffff, 0.0), (0xffffff, 0.0)),
            Daylight::Dawn => ((0xffbaa0, 0.20), (0xffe4c8, 0.14)),
            Daylight::Dusk => ((0x7a5a9a, 0.34), (0xffa060, 0.5)),
            Daylight::Night => ((0x0e1436, 0.78), (0x1c2450, 0.64)),
        };
        (
            mix(top, art::hex(tint_top.0), tint_top.1),
            mix(bottom, art::hex(tint_bottom.0), tint_bottom.1),
        )
    };
    let (from, to, t) = between(frame.hour);
    let (a, b) = (tinted(from), tinted(to));
    (mix(a.0, b.0, t), mix(a.1, b.1, t))
}

/// What the weather lays over the sky: grey under rain, slate in a storm,
/// pale before snow, rust in a dust storm.
fn overcast(weather: Weather) -> Option<(u32, f32)> {
    match weather {
        Weather::Clear => None,
        Weather::Cloudy => Some((0x9aa4ad, 0.22)),
        Weather::Rain => Some((0x6f7a86, 0.42)),
        Weather::Storm => Some((0x2f3844, 0.62)),
        Weather::Snow => Some((0xe8edf2, 0.35)),
        Weather::Fog => Some((0xd8dde0, 0.4)),
        Weather::Dust => Some((0xb8643a, 0.45)),
    }
}

/// The band of far hills: wider than the window by as much as it moves
/// along a panorama, standing on the horizon.
struct Band {
    /// Its width, and how far it reaches above and below the horizon, in
    /// window pixels.
    w: f32,
    above: f32,
    below: f32,
    /// How far it reaches past the window's left edge at rest.
    pad: f32,
    view_w: f32,
    view_h: f32,
}

impl Band {
    fn of(frame: &Frame, width: f32, height: f32) -> Self {
        let pad = width * 0.12;
        Self {
            w: width + (frame.width - frame.view_w).max(0.0) * HILLS + pad * 2.0,
            above: height * 0.22,
            below: height * 0.17,
            pad,
            view_w: width,
            view_h: height,
        }
    }

    /// Where its top-left corner is on screen.
    fn screen(&self, frame: &Frame) -> (f32, f32) {
        let horizon = frame.at(0.0, frame.horizon).1;
        (
            -self.pad - frame.pan() * HILLS,
            horizon - self.above * Self::squash(frame),
        )
    }

    /// Zoomed out past one window, the hills are lower, in keeping with
    /// the smaller town.
    fn squash(frame: &Frame) -> f32 {
        frame.camera.zoom.clamp(0.45, 1.0)
    }

    /// The top of a range of hills at `x` along the band: `depth` 0 is the
    /// farthest, 2 the ridge nearest.
    fn ridge(&self, depth: usize, x: f32, seed: u32) -> f32 {
        let amp = [0.12, 0.085, 0.06][depth.min(2)] * self.view_h;
        let w = self.view_w.max(1.0);
        let phase =
            |salt: u32| (painter::hash2(depth as i32, salt as i32, seed) % 628) as f32 / 100.0;
        let shape = 0.55
            + 0.25 * (x / (0.33 * w) + phase(1)).sin()
            + 0.14 * (x / (0.14 * w) + phase(2)).sin()
            + 0.06 * (x / (0.047 * w) + phase(3)).sin();
        self.above - amp * shape - (2 - depth.min(2)) as f32 * self.view_h * 0.01
    }

    /// Where a built thing `along` the ridge stands, in band pixels: its
    /// middle and its foot.
    fn mark_at(&self, frame: &Frame, along: f32) -> (f32, f32) {
        let x = self.pad + along * (self.w - self.pad * 2.0).max(1.0);
        (
            x,
            self.ridge(2, x, seed_of_scenery(&frame.scenery)) + self.view_h * 0.012,
        )
    }
}

fn seed_of_scenery(scenery: &Scenery) -> u32 {
    scenery.far ^ scenery.near.rotate_left(9) ^ scenery.sky_top.rotate_left(17)
}

/// The far hills, each range paler with the air between, what the World
/// has built standing on the nearest, and a line of low sun along their
/// tops at dawn and dusk.
#[cfg(test)]
fn paint_band(frame: &Frame, band: &Band, dpr: f32) -> Option<sk::Pixmap> {
    let tall = band.above + band.below;
    // Far off and seen through air, the hills are soft: half the display's
    // resolution is plenty.
    let scale = (dpr * 0.5).max(1.0);
    painter::in_strips(
        (band.w * scale).ceil() as u32,
        (tall * scale).ceil() as u32,
        scale,
        (0.0, 0.0),
        &|canvas, at| painter::timed("band", || paint_band_on(canvas, at, frame, band)),
    )
}

/// The far hills onto one strip of their band.
fn paint_band_on(canvas: &mut Canvas, at: (i32, i32), frame: &Frame, band: &Band) {
    let tall = band.above + band.below;
    let seed = seed_of_scenery(&frame.scenery);
    let far = art::hex(frame.scenery.far);
    let haze = art::hex(frame.scenery.sky_bottom);
    let low = matches!(frame.daylight, Daylight::Dawn | Daylight::Dusk) && sun_out(frame.weather);
    let (cover_ink, cover_share) = match frame.cover {
        Some(GroundCover::Snow) => (art::hex(0xf2f5f8), [0.7, 0.6, 0.45]),
        Some(GroundCover::Frost) => (art::hex(0xe6edf2), [0.3, 0.25, 0.2]),
        Some(GroundCover::Leaves) => (art::hex(0xc8783a), [0.12, 0.2, 0.3]),
        Some(GroundCover::Dust) => (art::hex(0xc0704a), [0.2, 0.22, 0.25]),
        _ => (far, [0.0, 0.0, 0.0]),
    };
    let (from, to) = (
        canvas.origin.0 - 20.0,
        canvas.origin.0 + canvas.width() as f32 / canvas.scale + 20.0,
    );
    let step = 10.0;
    for depth in 0..3 {
        let air = [0.56, 0.3, 0.0][depth];
        let ink = art::shade(mix(far, haze, air), if depth == 2 { -0.08 } else { 0.0 });
        let ink = mix(ink, cover_ink, cover_share[depth]);
        let mut shape = Shape::new();
        let mut crest = Shape::new();
        let mut x = (from / step).floor() * step;
        shape.move_to(x, tall + 2.0);
        crest.move_to(x, band.ridge(depth, x, seed));
        while x <= to {
            let y = band.ridge(depth, x, seed);
            shape.line_to(x, y);
            crest.line_to(x, y);
            x += step;
        }
        shape.line_to(x, tall + 2.0).close();
        canvas.fill(&shape, ink);
        // A fine line of ink along the nearer ridges' tops, so each reads
        // against the one behind; warm with the low sun.
        if depth > 0 {
            let rim = if low {
                mix(art::hex(0xffc27a), ink, 0.45).opacity(0.4)
            } else {
                art::shade(ink, -0.35).opacity(0.22)
            };
            canvas.stroke(&crest, 1.1, rim);
        }
    }
    // The air between: every range paler toward its foot.
    canvas.gradient(
        from,
        band.above - band.view_h * 0.1,
        to - from,
        band.view_h * 0.1 + band.below,
        180.0,
        (haze.opacity(0.0), 0.0),
        (haze.opacity(0.3), 0.55),
    );
    // What the World has built stands along the ridge.
    let silhouette = mix(art::shade(far, -0.3), haze, 0.12);
    let sun = art::hex(frame.scenery.sun);
    let mark_h = frame.building_h * 0.34;
    for mark in frame.marks.iter().filter(|mark| mark.grow >= 1.0) {
        let (x, base) = band.mark_at(frame, mark.along);
        let w = mark_h * 0.7;
        if x + w < from || x - w > to {
            continue;
        }
        crate::ui::paint_mark(
            canvas,
            Bounds::new(
                point(px(x - w / 2.0), px(base - mark_h)),
                size(px(w), px(mark_h)),
            ),
            mark.shape,
            silhouette,
            sun,
        );
    }
    for goal in &frame.goals {
        let (x, base) = band.mark_at(frame, goal.along);
        paint_goal(canvas, x, base, goal, silhouette, sun);
    }
    painter::grain_lit(canvas, at, 0.04, 0.035, light_at(frame.hour, frame.weather));
}

/// A goal on the ridge: finished, it stands as solid as anything else the
/// World built; under way, a pale outline more solid with each part, with
/// scaffolding as high as it has got and a pip under it for every part.
fn paint_goal(
    brush: &mut dyn Brush,
    x: f32,
    base: f32,
    goal: &RidgeGoal,
    silhouette: Hsla,
    light: Hsla,
) {
    let (w, h) = (goal.w, goal.h);
    let bounds = Bounds::new(point(px(x - w / 2.0), px(base - h)), size(px(w), px(h)));
    if goal.done >= goal.parts {
        crate::ui::paint_mark(brush, bounds, goal.shape, silhouette, light);
        return;
    }
    let share = goal.done as f32 / goal.parts.max(1) as f32;
    let ghost = gpui::white().opacity(0.16 + 0.4 * share);
    crate::ui::paint_mark(brush, bounds, goal.shape, ghost, light.opacity(0.4));
    let wood = art::hex(0x8a6a44).opacity(0.85);
    let left = x - w / 2.0;
    let risen = h * (0.35 + 0.65 * share);
    for pole in 0..3 {
        let px0 = left + w * (0.08 + 0.42 * pole as f32);
        art::line(brush, (px0, base), (px0, base - risen), 1.4, wood);
    }
    let boards = 1 + (share * 3.0) as usize;
    for board in 0..boards {
        let by = base - risen * (board as f32 + 1.0) / (boards as f32 + 0.3);
        art::line(
            brush,
            (left + w * 0.02, by),
            (left + w * 0.98, by),
            1.2,
            wood,
        );
    }
    art::line(
        brush,
        (left + w * 0.08, base),
        (left + w * 0.5, base - risen),
        1.0,
        wood.opacity(0.6),
    );
    let pip = (w * 0.09).clamp(3.0, 6.0);
    let gap = pip * 0.8;
    let row = goal.parts as f32 * pip + (goal.parts as f32 - 1.0) * gap;
    for part in 0..goal.parts {
        let px0 = x - row / 2.0 + part as f32 * (pip + gap);
        let ink = if part < goal.done {
            gpui::white().opacity(0.9)
        } else {
            gpui::white().opacity(0.25)
        };
        brush.rect(px0, base + pip, pip, pip, pip / 2.0, ink);
    }
}

/// The top of the field, in stage pixels, at `x`.
fn field_top(frame: &Frame, x: f32) -> f32 {
    let top = frame.horizon + (frame.base - frame.horizon) * 0.3;
    let w = frame.view_w.max(1.0);
    top - 8.0 * (x / (0.9 * w)).sin() - 5.0 * (x / (0.37 * w) + 1.3).sin()
}

/// The top of the foreground (the water's edge in a harbour), at `x`.
fn front_top(frame: &Frame, x: f32) -> f32 {
    let w = frame.view_w.max(1.0);
    frame.front - 6.0 * (x / (0.7 * w) + 0.4).sin() - 3.0 * (x / (0.23 * w)).sin()
}

/// The land's colours for the season: the field, and the foreground.
fn land_colours(frame: &Frame) -> (Hsla, Hsla) {
    let far = art::hex(frame.scenery.far);
    let ground = art::shade(far, 0.16);
    let near = art::hex(frame.scenery.near);
    let (ink, share, near_share) = match frame.cover {
        Some(GroundCover::Snow) => (art::hex(0xf3f6f9), 0.82, 0.7),
        Some(GroundCover::Frost) => (art::hex(0xe9eff3), 0.34, 0.25),
        Some(GroundCover::Leaves) => (art::hex(0xc08a44), 0.16, 0.1),
        Some(GroundCover::Dust) => (art::hex(0xc0704a), 0.22, 0.18),
        Some(GroundCover::Blossom) | None => (ground, 0.0, 0.0),
    };
    let near_ink = if frame.water {
        near
    } else {
        mix(near, ink, near_share)
    };
    (mix(ground, ink, share), near_ink)
}

/// Everything the ground layer depends on: the look, the geometry, the
/// scale it is painted at, and every building standing still on it.
fn ground_key(frame: &Frame, scale: f32) -> Key {
    let mut key = Key::new("ground");
    look_key(frame, &mut key);
    for value in [
        frame.width,
        frame.view_w,
        frame.height,
        frame.horizon,
        frame.base,
        frame.front,
        frame.building_h,
        scale,
    ] {
        key.float(value);
    }
    for building in frame.buildings.iter().filter(|b| !b.moving()) {
        key.add((building.index, building.shape as u8))
            .float(building.x)
            .float(building.base)
            .float(building.w)
            .float(building.h)
            .colour(building.palette.wall)
            .colour(building.palette.roof)
            .colour(building.palette.glass)
            .add(building.palette.seed)
            .add(building.drawing.as_ref().map(|d| d.id.clone()))
            .add(building.glow.is_some());
        if let Some(glow) = building.glow {
            key.colour(glow);
        }
        for figure in &building.inside {
            key.colour(figure.clothes);
        }
    }
    key
}

/// A window and a view's size in it.
type ViewId = (u64, u32, u32);

/// The zoom the ground is painted at: the camera's own when it is still,
/// and while it moves the last one painted (scaled), so a glide does not
/// paint every frame.
fn painted_zoom(window: &Window, frame: &Frame) -> f32 {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        static ZOOMS: RefCell<HashMap<ViewId, (f32, f32)>> = RefCell::new(HashMap::new());
    }
    let zoom = frame.camera.zoom;
    let exact = (zoom * 100.0).round() / 100.0;
    let id = (
        window.window_handle().window_id().as_u64(),
        frame.view_w as u32,
        frame.height as u32,
    );
    ZOOMS.with(|zooms| {
        let mut zooms = zooms.borrow_mut();
        let entry = zooms.entry(id).or_insert((zoom, exact));
        let moving = (zoom - entry.0).abs() > 1e-4;
        entry.0 = zoom;
        if !moving || (entry.1 / zoom - 1.0).abs() > 0.5 {
            entry.1 = exact;
        }
        entry.1.max(0.05)
    })
}

/// The two still layers on the ground: the land itself, soft enough to
/// paint at half the display's resolution, and the buildings over it at
/// the full.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Layer {
    Land,
    Buildings,
}

/// The stage rows (top and bottom, in stage pixels) a layer covers.
fn layer_rows(frame: &Frame, layer: Layer) -> (f32, f32) {
    match layer {
        Layer::Land => (
            frame.horizon + (frame.base - frame.horizon) * 0.3 - 16.0,
            f32::MAX,
        ),
        Layer::Buildings => (
            ground_reach(frame),
            frame
                .buildings
                .iter()
                .map(|building| building.base + building.h * 0.4)
                .fold(frame.base, f32::max),
        ),
    }
}

/// The tiles of one layer the camera sees, at `scale` device pixels to a
/// stage pixel, as (column, row).
fn tiles_in_view(frame: &Frame, layer: Layer, scale: f32, margin: i32) -> Vec<(i32, i32)> {
    let zoom = frame.camera.zoom;
    let tile = TILE as f32 / scale;
    let (x0, y0) = (
        frame.camera.x - frame.view_w / (2.0 * zoom),
        frame.camera.y - frame.height / (2.0 * zoom),
    );
    let (x1, y1) = (
        frame.camera.x + frame.view_w / (2.0 * zoom),
        frame.camera.y + frame.height / (2.0 * zoom),
    );
    let (top, bottom) = layer_rows(frame, layer);
    let rows = ((y0.max(top) / tile).floor() as i32)
        ..=((y1.min(bottom).min(frame.height + tile) / tile).floor() as i32);
    let columns =
        ((x0 / tile).floor() as i32 - margin)..=(((x1 - 0.01) / tile).floor() as i32 + margin);
    rows.flat_map(|row| columns.clone().map(move |column| (column, row)))
        .collect()
}

/// How high the ground layer reaches, in stage pixels: the field's top, or
/// the tallest building's roof and its glow.
fn ground_reach(frame: &Frame) -> f32 {
    frame
        .buildings
        .iter()
        .map(|building| building.base - building.h * 1.45)
        .fold(
            frame.horizon + (frame.base - frame.horizon) * 0.3 - 20.0,
            f32::min,
        )
}

/// One tile of the land, `column` and `row` of them from the stage's
/// origin, at `scale` device pixels to a stage pixel, lit and on paper.
fn paint_land_tile(frame: &Frame, column: i32, row: i32, scale: f32) -> Option<sk::Pixmap> {
    let tile = TILE as f32 / scale;
    // A pixel more all round than the tile, so where tiles meet, shown
    // larger than they are painted, the display blends real neighbours.
    let pad = LAND_PAD as f32 / scale;
    let origin = (column as f32 * tile - pad, row as f32 * tile - pad);
    let mut canvas = Canvas::new(TILE + 2 * LAND_PAD, TILE + 2 * LAND_PAD, scale, origin)?;
    let view = (origin.0, origin.0 + tile + pad * 2.0);
    painter::timed("tile: land", || paint_land(&mut canvas, frame, view));
    let light = light_at(frame.hour, frame.weather);
    painter::timed("tile: paper", || {
        painter::grain_lit(
            &mut canvas,
            (
                column * TILE as i32 - LAND_PAD as i32,
                row * TILE as i32 - LAND_PAD as i32,
            ),
            0.05,
            0.035,
            light,
        )
    });
    Some(canvas.pixmap)
}

/// One tile of the buildings: every shadow, then every building over them
/// (each already lit, and on its own paper). Nothing where there are none.
fn paint_building_tile(
    frame: &Frame,
    ground: u64,
    column: i32,
    row: i32,
    scale: f32,
) -> Option<sk::Pixmap> {
    let tile = TILE as f32 / scale;
    let pad = LAND_PAD as f32 / scale;
    let origin = (column as f32 * tile - pad, row as f32 * tile - pad);
    let (top, bottom) = (origin.1, origin.1 + tile + pad * 2.0);
    let near = near_tiles(frame, &[(column, row)], scale)
        .filter(|b| b.base - b.h * 1.45 < bottom && b.base + b.h * 0.4 > top)
        .collect::<Vec<_>>();
    if near.is_empty() {
        return None;
    }
    let mut canvas = Canvas::new(TILE + 2 * LAND_PAD, TILE + 2 * LAND_PAD, scale, origin)?;
    let sprites = near
        .iter()
        .map(|building| sprite_of(ground, frame, building, scale))
        .collect::<Vec<_>>();
    painter::timed("tile: shadows", || {
        for (building, sprite) in near.iter().zip(&sprites) {
            paint_building_shadow(&mut canvas, frame, building, sprite);
        }
    });
    painter::timed("tile: buildings", || {
        for sprite in &sprites {
            canvas.draw(
                &sprite.pixmap,
                sprite.origin.0,
                sprite.origin.1,
                sk::BlendMode::SourceOver,
            );
        }
    });
    Some(canvas.pixmap)
}

/// The field, the path worn along it, what grows and lies about on it, the
/// foreground or the water, and what the season lays over them, for stage
/// `x` from `view.0` to `view.1`.
fn paint_land(canvas: &mut Canvas, frame: &Frame, view: (f32, f32)) {
    let (ground, near) = land_colours(frame);
    let bottom = frame.height + TILE as f32 / canvas.scale + 40.0;
    let (from, to) = (view.0 - 60.0, view.1 + 60.0);
    let step = 24.0;
    let edge = |top: &dyn Fn(f32) -> f32| {
        let mut shape = Shape::new();
        shape.move_to(from, bottom);
        let mut x = from;
        while x < to + step {
            shape.line_to(x, top(x));
            x += step;
        }
        shape.line_to(to + step, bottom).close();
        shape
    };
    // The field, lighter toward the hills with the air between.
    let haze = art::hex(frame.scenery.sky_bottom);
    let field = edge(&|x| field_top(frame, x));
    let field_y = frame.horizon + (frame.base - frame.horizon) * 0.3;
    canvas.fill(&field, ground);
    // The field paler toward the hills, with the air between.
    let (tile_top, tile_bottom) = (
        canvas.origin.1,
        canvas.origin.1 + canvas.height() as f32 / canvas.scale,
    );
    let haze_to = field_y + (frame.base - field_y) * 0.9;
    if tile_top < haze_to && tile_bottom > field_y - 30.0 {
        let mut hazed = Canvas::new(canvas.width(), canvas.height(), canvas.scale, canvas.origin)
            .expect("a tile");
        hazed.fill(&field, mix(ground, haze, 0.2));
        painter::fade_down(
            &mut hazed.pixmap,
            canvas.device(0.0, field_y - 20.0).1,
            canvas.device(0.0, haze_to).1,
        );
        canvas.draw(
            &hazed.pixmap,
            canvas.origin.0,
            canvas.origin.1,
            sk::BlendMode::SourceOver,
        );
    }
    let mut rim = Shape::new();
    let mut x = from;
    rim.move_to(x, field_top(frame, x));
    while x < to + step {
        x += step;
        rim.line_to(x, field_top(frame, x));
    }
    canvas.stroke(&rim, 1.2, art::shade(ground, -0.3).opacity(0.25));
    let k = (frame.height / 848.0).clamp(0.3, 1.3);
    let detail = frame.height >= 360.0;
    let strip_top = frame.base + (frame.front - frame.base) * 0.55;
    let front = frame.front;
    let w = frame.view_w.max(1.0);
    if detail {
        // A worn path winds along the strip, where people have walked.
        let mid = strip_top + (front - strip_top) * 0.45;
        let wind = |x: f32| mid + 9.0 * k * (x / (0.55 * w) + 0.7).sin();
        let mut path = Shape::new();
        let mut x = from;
        path.move_to(x, wind(x) - 7.0 * k);
        while x < to + step {
            x += step;
            path.line_to(x, wind(x) - 7.0 * k);
        }
        while x > from {
            path.line_to(x, wind(x) + 9.0 * k);
            x -= step;
        }
        path.close();
        canvas.fill(&path, art::shade(ground, 0.1).opacity(0.7));
    }
    // The foreground: water deepening toward the front, or near ground.
    let shore = edge(&|x| front_top(frame, x));
    if frame.water {
        painter::fill_shaded(
            canvas,
            &shore,
            (0.0, front - 10.0),
            (0.0, frame.height),
            &[
                (0.0, mix(near, haze, 0.28)),
                (0.25, near),
                (1.0, art::shade(near, -0.18)),
            ],
        );
    } else {
        painter::fill_shaded(
            canvas,
            &shore,
            (0.0, front - 10.0),
            (0.0, frame.height),
            &[(0.0, near), (1.0, art::shade(near, -0.1))],
        );
    }
    let mut line = Shape::new();
    let mut x = from;
    line.move_to(x, front_top(frame, x));
    while x < to + step {
        x += step;
        line.line_to(x, front_top(frame, x));
    }
    canvas.stroke(
        &line,
        1.4,
        if frame.water {
            gpui::white().opacity(0.35)
        } else {
            art::shade(near, -0.3).opacity(0.4)
        },
    );
    // A frozen edge to the harbour in deep winter.
    if frame.ice && frame.water {
        let deep = (frame.height - front) * 0.16;
        let ice = edge(&|x| front_top(frame, x) - 1.0);
        let mut sheet = Shape::new();
        let mut x = from;
        sheet.move_to(x, front_top(frame, x) - 1.0);
        while x < to + step {
            x += step;
            sheet.line_to(x, front_top(frame, x) - 1.0);
        }
        while x > from {
            let lip = deep * (0.75 + 0.25 * (x / (0.11 * w)).sin());
            sheet.line_to(x, front_top(frame, x) + lip);
            x -= step;
        }
        sheet.close();
        let _ = ice;
        canvas.fill(&sheet, art::hex(0xe4eef3).opacity(0.92));
        for index in 0..((frame.width / w * 7.0) as i32) {
            let seed = painter::hash2(index, 3, 0x1ce);
            let x = (seed % 10_000) as f32 / 10_000.0 * frame.width;
            if x < from || x > to {
                continue;
            }
            let y = front_top(frame, x) + deep * 0.3;
            let mut crack = Shape::new();
            crack
                .move_to(x, y)
                .line_to(x + 6.0 * k, y + deep * 0.15)
                .line_to(x + 4.0 * k, y + deep * 0.3);
            canvas.stroke(&crack, 0.8, art::hex(0xb4c6d2).opacity(0.55));
        }
    }
    if !detail {
        return;
    }
    // What grows and lies about along the strip, in the colours of the
    // ground it is on; the same place always has the same ones.
    let seed0 = seed_of_scenery(&frame.scenery);
    let per = frame.width / w;
    let scatter = |index: i32, salt: u32| {
        let seed = painter::hash2(index, salt as i32, seed0);
        (
            (seed % 10_000) as f32 / 10_000.0 * frame.width,
            ((seed / 10_000) % 1000) as f32 / 1000.0,
            seed,
        )
    };
    let blade = match frame.cover {
        Some(GroundCover::Snow) => art::shade(ground, -0.12),
        _ => art::shade(ground, -0.22),
    };
    for index in 0..((48.0 * per) as i32) {
        let (x, t, seed) = scatter(index, 1);
        if x < from || x > to {
            continue;
        }
        let y = strip_top + t * (front - strip_top);
        let tall = (9.0 + (seed >> 24) as f32 % 8.0) * k;
        for lean in [-1.0_f32, 0.0, 1.0] {
            art::line(
                canvas,
                (x + lean * 2.0 * k, y),
                (x + lean * 4.0 * k, y - tall * (1.0 - lean.abs() * 0.25)),
                1.3 * k,
                blade,
            );
        }
    }
    let flower_inks = [0xf2d0e0_u32, 0xfff2b0, 0xffffff, 0xd8c8f2];
    let flowering = !matches!(
        frame.cover,
        Some(GroundCover::Snow | GroundCover::Frost | GroundCover::Dust)
    );
    for index in 0..((16.0 * per) as i32) {
        let (x, t, seed) = scatter(index, 2);
        if x < from || x > to {
            continue;
        }
        let y = strip_top + t * (front - strip_top);
        if seed % 3 == 0 || !flowering {
            art::ellipse(canvas, x, y, 5.0 * k, 3.0 * k, art::shade(ground, -0.35));
            art::ellipse(
                canvas,
                x - 1.0 * k,
                y - 1.0 * k,
                3.0 * k,
                1.6 * k,
                art::shade(ground, 0.12),
            );
        } else {
            let ink = art::hex(flower_inks[((seed >> 8) % 4) as usize]);
            art::line(canvas, (x, y), (x, y - 10.0 * k), 1.2 * k, blade);
            art::circle(canvas, x, y - 10.5 * k, 3.4 * k, ink);
            art::circle(canvas, x, y - 10.5 * k, 1.3 * k, art::hex(0xe8b040));
        }
    }
    // A short run of fence posts here and there.
    let post = art::shade(ground, -0.45);
    for index in 0..(per.ceil() as i32) {
        let (x, _, _) = scatter(index, 3);
        if x + 80.0 * k < from || x > to {
            continue;
        }
        for post_index in 0..4 {
            let px0 = x + post_index as f32 * 24.0 * k;
            art::rect(
                canvas,
                px0,
                strip_top - 6.0 * k,
                4.0 * k,
                20.0 * k,
                1.0,
                post,
            );
        }
        art::line(
            canvas,
            (x, strip_top + 1.0 * k),
            (x + 76.0 * k, strip_top + 1.0 * k),
            1.4 * k,
            post,
        );
    }
    if frame.water {
        // Reeds where the land meets the water.
        let reed = art::shade(near, 0.25);
        for index in 0..((18.0 * per) as i32) {
            let (x, t, _) = scatter(index, 4);
            if x < from || x > to {
                continue;
            }
            let tall = (10.0 + t * 8.0) * k;
            let lean = (t - 0.5) * 4.0 * k;
            let foot = front_top(frame, x) + 4.0;
            art::line(canvas, (x, foot), (x + lean, foot - tall), 1.2 * k, reed);
        }
    } else {
        // On dry ground the foreground itself has stones and grass too.
        for index in 0..((16.0 * per) as i32) {
            let (x, t, seed) = scatter(index, 5);
            if x < from || x > to {
                continue;
            }
            let y = front + 10.0 + t * (frame.height - front - 14.0);
            if seed % 2 == 0 {
                art::ellipse(canvas, x, y, 7.0 * k, 4.0 * k, art::shade(near, -0.25));
            } else {
                for lean in [-1.0_f32, 1.0] {
                    art::line(
                        canvas,
                        (x, y),
                        (x + lean * 4.0 * k, y - 9.0 * k),
                        1.4 * k,
                        art::shade(near, 0.2),
                    );
                }
            }
        }
    }
    // What the season lays on the ground.
    let (count, inks): (f32, &[u32]) = match frame.cover {
        Some(GroundCover::Leaves) => (60.0, &[0xc8642e, 0xd9913a, 0xa8452c, 0xe0b04a]),
        Some(GroundCover::Blossom) => (36.0, &[0xf6c9d8, 0xfbe3ea, 0xf0b3c8]),
        Some(GroundCover::Frost) => (60.0, &[0xffffff]),
        Some(GroundCover::Snow) => (26.0, &[0xffffff]),
        Some(GroundCover::Dust) => (22.0, &[0xb85a36]),
        None => (0.0, &[]),
    };
    for index in 0..((count * per) as i32) {
        let (x, t, seed) = scatter(index, 6);
        if x < from || x > to {
            continue;
        }
        let y = field_top(frame, x) + 6.0 + t * (front - field_top(frame, x) - 4.0);
        let ink = art::hex(inks[(seed >> 12) as usize % inks.len()]);
        match frame.cover {
            Some(GroundCover::Snow) => {
                // Soft drifts, pale blue on their shaded side.
                canvas.soft(x, y, 26.0 * k, 5.0 * k, 6.0 * k, ink.opacity(0.7));
                canvas.soft(
                    x + 6.0 * k,
                    y + 3.0 * k,
                    20.0 * k,
                    3.0 * k,
                    4.0 * k,
                    art::hex(0xc8d6e4).opacity(0.5),
                );
            }
            Some(GroundCover::Dust) => {
                canvas.soft(x, y, 34.0 * k, 3.0 * k, 5.0 * k, ink.opacity(0.35));
            }
            Some(GroundCover::Frost) => {
                art::circle(canvas, x, y, 0.9 * k, ink.opacity(0.7));
            }
            _ => {
                let turn = ((seed >> 4) % 100) as f32 / 100.0;
                let (rx, ry) = (2.6 * k * (0.6 + 0.4 * turn), 1.5 * k);
                art::ellipse(canvas, x, y, rx, ry, ink.opacity(0.9));
            }
        }
    }
}

/// A building painted on its own, lit: its key light and shade, a line of
/// low sun on its edge, snow or dust along its tops, its lit windows with
/// whoever is inside and their glow; and the shape of it for its shadow.
struct Sprite {
    pixmap: sk::Pixmap,
    /// Its top-left corner, in stage pixels.
    origin: (f32, f32),
    /// Its shape at half resolution, softened, for the shadow it casts.
    silhouette: Option<sk::Pixmap>,
}

/// The buildings' sprites, painted once for the ground's look and shared
/// by every tile each reaches into: by the ground's key and the item.
type Sprites = std::collections::HashMap<(u64, usize), std::sync::Arc<Sprite>>;

fn sprites() -> &'static std::sync::Mutex<Sprites> {
    static SPRITES: std::sync::OnceLock<std::sync::Mutex<Sprites>> = std::sync::OnceLock::new();
    SPRITES.get_or_init(Default::default)
}

/// The buildings near any of the tiles at `places`.
fn near_tiles<'f>(
    frame: &'f Frame,
    places: &[(i32, i32)],
    scale: f32,
) -> impl Iterator<Item = &'f BuildingPaint> + 'f {
    let tile = TILE as f32 / scale;
    let reach = frame.building_h * 1.2;
    let spans = places
        .iter()
        .map(|(column, _)| (*column as f32 * tile, (*column + 1) as f32 * tile))
        .collect::<Vec<_>>();
    frame.buildings.iter().filter(move |b| {
        !b.moving()
            && spans
                .iter()
                .any(|(from, to)| b.x + b.w + reach > *from && b.x - b.w - reach < *to)
    })
}

/// Paints, across the painter's threads, the sprite of every building the
/// tiles at `places` need that is not painted yet.
#[cfg(test)]
fn prepare_sprites(frame: &Frame, ground: u64, scale: f32, places: &[(i32, i32)]) {
    let missing = {
        let kept = sprites()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        near_tiles(frame, places, scale)
            .filter(|building| !kept.contains_key(&(ground, building.index)))
            .collect::<Vec<_>>()
    };
    if missing.len() < 2 {
        return;
    }
    let jobs = missing
        .iter()
        .map(|building| {
            let building = *building;
            Box::new(move || (building.index, sprite(frame, building, scale)))
                as Box<dyn FnOnce() -> (usize, Sprite) + Send + '_>
        })
        .collect();
    let painted = painter::parallel(jobs);
    let mut kept = sprites()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if kept.len() > 400 {
        kept.retain(|(key, _), _| *key == ground);
    }
    for (index, sprite) in painted {
        kept.insert((ground, index), std::sync::Arc::new(sprite));
    }
}

/// A building's sprite for the ground's look, painted now if it is not
/// kept already.
fn sprite_of(
    ground: u64,
    frame: &Frame,
    building: &BuildingPaint,
    scale: f32,
) -> std::sync::Arc<Sprite> {
    if let Some(sprite) = sprites()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&(ground, building.index))
    {
        return sprite.clone();
    }
    let sprite = std::sync::Arc::new(sprite(frame, building, scale));
    let mut kept = sprites()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if kept.len() > 400 {
        kept.retain(|(key, _), _| *key == ground);
    }
    kept.insert((ground, building.index), sprite.clone());
    sprite
}

/// Whether a place is something that grows: a tree, an orchard, a garden.
fn leafy(building: &BuildingPaint) -> bool {
    matches!(
        building.shape,
        MarkShape::Tree | MarkShape::Garden | MarkShape::Planter | MarkShape::Sprouts
    ) || building.drawing.as_ref().is_some_and(|drawing| {
        ["tree", "orchard", "garden", "park", "grove"]
            .iter()
            .any(|word| drawing.id.contains(word))
    })
}

fn sprite(frame: &Frame, building: &BuildingPaint, scale: f32) -> Sprite {
    painter::timed("sprite", || sprite_painted(frame, building, scale))
}

fn sprite_painted(frame: &Frame, building: &BuildingPaint, scale: f32) -> Sprite {
    let (w, h) = (building.w, building.h);
    let left = building.x - w * 0.72;
    let top = building.base - h * 1.28;
    let (right, bottom) = (building.x + w * 0.72, building.base + h * 0.04);
    let origin = (left, top);
    let empty = || Sprite {
        pixmap: sk::Pixmap::new(1, 1).expect("a pixel"),
        origin,
        silhouette: None,
    };
    let Some(mut canvas) = Canvas::new(
        ((right - left) * scale).ceil() as u32,
        ((bottom - top) * scale).ceil() as u32,
        scale,
        origin,
    ) else {
        return empty();
    };
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    if let Some(glow) = building.glow {
        canvas.soft(
            building.x,
            building.base,
            w * 0.78,
            h * 0.2,
            h * 0.14,
            glow.opacity(0.45),
        );
    }
    painter::timed("sprite: paint", || match &building.drawing {
        Some(drawing) => art::paint_drawing(
            &mut canvas,
            building.x,
            building.base,
            w,
            h,
            drawing,
            &Inks::of_place(&building.palette).lit(lit),
            Stance::Standing,
            world_projection::Mood::Content,
            0.0,
            0.0,
            1.0,
        ),
        None => art::paint_building(
            &mut canvas,
            building.x,
            building.base,
            w,
            h,
            building.shape,
            &building.palette,
        ),
    });
    let width = canvas.width() as usize;
    let height = canvas.height() as usize;
    // Lit windows, found by their lamplight colour before anything else
    // touches them.
    let glass = if lit {
        let mask = painter::find_colour(&canvas.pixmap, LAMPLIGHT, (14, 26, 34));
        mask.iter().any(|value| *value > 0.0).then_some(mask)
    } else {
        None
    };
    // Snow, frost or dust along every top edge.
    let thick = (h * 0.03 * scale).max(2.0);
    match frame.cover {
        Some(GroundCover::Snow) => {
            painter::timed("sprite: snow", || {
                painter::cap(&mut canvas.pixmap, thick, [0.95, 0.97, 0.99], 0.94)
            });
        }
        Some(GroundCover::Frost) => {
            painter::cap(&mut canvas.pixmap, thick * 0.45, [0.9, 0.94, 0.98], 0.5);
        }
        Some(GroundCover::Dust) => {
            painter::cap(&mut canvas.pixmap, thick * 0.55, [0.74, 0.42, 0.28], 0.55);
        }
        // Only what grows turns with the seasons, never a green roof.
        Some(GroundCover::Leaves) if leafy(building) => painter::autumn(&mut canvas.pixmap),
        Some(GroundCover::Blossom) if leafy(building) => {
            painter::blossom(&mut canvas.pixmap, (scale * 1.3).max(1.0))
        }
        _ => {}
    }
    // The key light from the sun's side, shade on the other, and the foot
    // of the walls darker where the ground holds the light back.
    let (across, high) = sun_at(frame.hour);
    let day = sun_out(frame.weather) && high > 0.0 && frame.daylight != Daylight::Night;
    let side = if across < 0.0 { -1.0 } else { 1.0 };
    let strength = if day { 0.1 + 0.14 * across.abs() } else { 0.06 };
    let (lit_x, shade_x) = (building.x + side * w * 0.5, building.x - side * w * 0.5);
    let (lit_x, shade_x) = (canvas.device(lit_x, 0.0).0, canvas.device(shade_x, 0.0).0);
    let (foot, base) = (
        canvas.device(0.0, building.base - h * 0.38).1,
        canvas.device(0.0, building.base).1,
    );
    painter::timed("sprite: light", || {
        painter::light_and_shade(
            &mut canvas.pixmap,
            (lit_x.min(shade_x), lit_x.max(shade_x)),
            side,
            strength,
            [1.0, 0.945, 0.84],
            [0.16, 0.19, 0.29],
            (foot, base),
            0.2,
            light_at(frame.hour, frame.weather),
            glass.as_deref(),
        )
    });
    // A thin line of ink where it meets the ground and under its eaves.
    painter::ground_line(
        &mut canvas.pixmap,
        (scale * 1.2).round().max(1.0) as usize,
        0.32,
    );
    // The low sun draws a warm line down the edges it falls on.
    if day && matches!(frame.daylight, Daylight::Dawn | Daylight::Dusk) {
        painter::rim(
            &mut canvas.pixmap,
            side,
            (scale * 1.4).round().max(1.0) as i32,
            [1.0, 0.76, 0.48],
            0.6,
        );
    }
    // The building's own paper, fixed to it so it does not crawl as the
    // view pans.
    painter::timed("sprite: paper", || {
        painter::grain(&mut canvas, (0, 0), 0.05, 0.03)
    });
    // Its shape, for the shadow it casts when the sun is out.
    let silhouette = day
        .then(|| {
            painter::timed("sprite: silhouette", || {
                painter::silhouette(&canvas.pixmap, (scale * 2.0).max(2.0) as usize)
            })
        })
        .flatten();
    if let Some(glass) = &glass {
        if !building.inside.is_empty() {
            painter::inside(&mut canvas.pixmap, glass, building.inside.len());
        }
        painter::timed("sprite: bloom", || {
            painter::bloom(
                &mut canvas.pixmap,
                glass,
                (w * 0.07 * scale / 2.0).max(2.0) as usize,
                [1.0, 0.8, 0.46],
                0.62,
            )
        });
    }
    let _ = (width, height);
    Sprite {
        pixmap: canvas.pixmap,
        origin,
        silhouette,
    }
}

/// The soft dark where a building meets the ground, and in sun its shape
/// laid flat across the ground away from the light.
fn paint_building_shadow(
    canvas: &mut Canvas,
    frame: &Frame,
    building: &BuildingPaint,
    sprite: &Sprite,
) {
    let night = frame.daylight == Daylight::Night;
    canvas.soft(
        building.x,
        building.base + building.h * 0.012,
        building.w * 0.56,
        building.h * 0.05,
        building.h * 0.06,
        gpui::black().opacity(if night { 0.3 } else { 0.22 }),
    );
    let (across, high) = sun_at(frame.hour);
    if night || !sun_out(frame.weather) || high <= 0.02 {
        return;
    }
    let Some(silhouette) = &sprite.silhouette else {
        return;
    };
    // Away from the sun, flat on the ground toward the viewer: long when
    // the sun is low, short at noon.
    let long = 0.2 + 0.55 * (1.0 - high).powi(2);
    let shear = -across.signum() * long * across.abs().max(0.25);
    let flat = 0.08 + 0.14 * (1.0 - high);
    let s = canvas.scale;
    let base = building.base;
    let (sox, soy) = sprite.origin;
    let x0 = sox + (base - soy) * shear;
    let y0 = base + (base - soy) * flat;
    let coarse = painter::COARSE as f32;
    let transform = sk::Transform::from_row(
        coarse,
        0.0,
        -coarse * shear,
        -coarse * flat,
        (x0 - canvas.origin.0) * s,
        (y0 - canvas.origin.1) * s,
    );
    let strength = if frame.weather == Weather::Cloudy {
        0.08
    } else {
        0.2
    };
    canvas.draw_mapped(silhouette, transform, strength);
}

/// The vignette: nothing in the middle, a very slight dark at the corners.
fn paint_vignette(width: f32, height: f32, scale: f32) -> Option<sk::Pixmap> {
    let mut canvas = Canvas::new(
        (width * scale).ceil().max(2.0) as u32,
        (height * scale).ceil().max(2.0) as u32,
        scale,
        (0.0, 0.0),
    )?;
    painter::vignette(&mut canvas, width, height, art::hex(0x24180f).opacity(0.2));
    Some(canvas.pixmap)
}

/// A brush that colours everything it draws by the hour's light, so what
/// moves is lit like the still layers under it.
struct Tint<'a> {
    inner: &'a mut dyn Brush,
    light: [f32; 3],
}

impl<'a> Tint<'a> {
    fn new(inner: &'a mut dyn Brush, light: [f32; 3]) -> Self {
        Self { inner, light }
    }

    fn lit(&self, colour: Hsla) -> Hsla {
        let rgba: gpui::Rgba = colour.into();
        gpui::Rgba {
            r: rgba.r * self.light[0],
            g: rgba.g * self.light[1],
            b: rgba.b * self.light[2],
            a: rgba.a,
        }
        .into()
    }
}

impl Brush for Tint<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
        let colour = self.lit(colour);
        self.inner.rect(x, y, w, h, radius, colour);
    }
    fn fill(&mut self, shape: &Shape, colour: Hsla) {
        let colour = self.lit(colour);
        self.inner.fill(shape, colour);
    }
    fn stroke(&mut self, shape: &Shape, width: f32, colour: Hsla) {
        let colour = self.lit(colour);
        self.inner.stroke(shape, width, colour);
    }
    fn soft(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, blur: f32, colour: Hsla) {
        // Shadows are the absence of light, and glows their own light.
        self.inner.soft(cx, cy, rx, ry, blur, colour);
    }
    fn gradient(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle: f32,
        from: (Hsla, f32),
        to: (Hsla, f32),
    ) {
        let (from, to) = ((self.lit(from.0), from.1), (self.lit(to.0), to.1));
        self.inner.gradient(x, y, w, h, angle, from, to);
    }
}

/// Clouds drifting across, slowly, each at its own pace, soft-edged; more
/// of them, and greyer, the worse the weather; gulls wheeling on a fair
/// day.
fn paint_clouds(window: &mut dyn Brush, frame: &Frame, ox: f32, oy: f32, width: f32, height: f32) {
    let t = frame.seconds;
    let night = frame.daylight == Daylight::Night;
    let k = (height / 848.0).clamp(0.3, 1.3);
    let horizon = frame.at(0.0, frame.horizon).1;
    let sky = height * HORIZON;
    let (clouds, cloud) = match (frame.daylight, frame.weather) {
        (Daylight::Night, _) => (4, art::hex(0x9aa4c8).opacity(0.14)),
        (_, Weather::Clear) => (4, gpui::white().opacity(0.78)),
        (_, Weather::Cloudy) => (7, art::hex(0xe4e8ec).opacity(0.88)),
        (_, Weather::Rain | Weather::Snow) => (8, art::hex(0xc4cad0).opacity(0.9)),
        (_, Weather::Storm) => (9, art::hex(0x5a6470).opacity(0.95)),
        (_, Weather::Fog) => (6, gpui::white().opacity(0.6)),
        (_, Weather::Dust) => (6, art::hex(0xd99a6c).opacity(0.7)),
    };
    // Lit from below by a low sun.
    let cloud = match frame.daylight {
        Daylight::Dusk if sun_out(frame.weather) => mix(cloud, art::hex(0xffc4a8), 0.45),
        Daylight::Dawn if sun_out(frame.weather) => mix(cloud, art::hex(0xffe0cc), 0.35),
        _ => cloud,
    };
    let _ = horizon;
    for index in 0..clouds {
        let speed = 4.0 + index as f32 * 1.7;
        let span = width + 320.0;
        let x = ox
            + ((index as f32 * 331.0 + t * speed - frame.pan() * PARALLAX[0]).rem_euclid(span))
            - 160.0;
        let y = oy + sky * (0.12 + 0.11 * (index % 5) as f32);
        let s = (1.0 - (index % 5) as f32 * 0.12) * k;
        window.soft(x, y, 50.0 * s, 17.0 * s, 16.0 * s, cloud);
        window.soft(
            x + 30.0 * s,
            y - 9.0 * s,
            34.0 * s,
            17.0 * s,
            14.0 * s,
            cloud,
        );
        window.soft(
            x - 30.0 * s,
            y + 2.0 * s,
            28.0 * s,
            12.0 * s,
            12.0 * s,
            cloud,
        );
    }
    if !night && sun_out(frame.weather) {
        for gull in 0..3 {
            let span = width + 200.0;
            let x = ox + ((gull as f32 * 417.0 + t * (18.0 + gull as f32 * 5.0)) % span) - 100.0;
            let y = oy + sky * (0.3 + 0.08 * gull as f32) + (t * 1.7 + gull as f32).sin() * 6.0;
            let flap = 3.0 + 2.0 * (t * 6.0 + gull as f32 * 2.0).sin();
            let mut wings = Shape::new();
            wings
                .move_to(x - 7.0 * k, y - flap * k)
                .line_to(x, y)
                .line_to(x + 7.0 * k, y - flap * k);
            window.stroke(&wings, 1.6, art::hex(0x3c4048).opacity(0.7));
        }
    }
}

/// Which way the wind blows smoke, and how hard: from the place's own seed
/// and the weather, turning slowly.
fn wind(frame: &Frame) -> f32 {
    let seed = seed_of_scenery(&frame.scenery);
    let from = if seed.is_multiple_of(2) { 1.0 } else { -1.0 };
    let strength = match frame.weather {
        Weather::Storm => 3.0,
        Weather::Rain | Weather::Snow | Weather::Dust => 1.8,
        Weather::Cloudy => 1.2,
        _ => 0.8,
    };
    from * strength * (0.75 + 0.25 * (frame.seconds * 0.05).sin())
}

/// Everything that moves, over the still layers, lit by the hour.
#[allow(clippy::too_many_arguments)]
fn paint_live(
    window: &mut dyn Brush,
    frame: &Frame,
    ox: f32,
    oy: f32,
    width: f32,
    height: f32,
    light: [f32; 3],
) {
    let t = frame.seconds;
    let z = frame.camera.zoom;
    let night = frame.daylight == Daylight::Night;
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    let (across, high) = sun_at(frame.hour);
    let day = sun_out(frame.weather) && high > 0.02 && !night;
    let screen = |x: f32, y: f32| {
        let (sx, sy) = frame.at(x, y);
        (ox + sx, oy + sy)
    };
    let seen = |x: f32, reach: f32| {
        let sx = frame.at(x, 0.0).0;
        sx + reach >= 0.0 && sx - reach <= width
    };
    let contact = gpui::black().opacity(if night { 0.26 } else { 0.2 });

    // The sea moves: short bright lines drifting and fading.
    if frame.water {
        let front = screen(0.0, frame.front).1;
        let bottom = oy + height;
        if front < bottom {
            let shimmer = gpui::white().opacity(if night { 0.12 } else { 0.26 });
            for row in 0..5 {
                let y = front + 16.0 * z + row as f32 * (bottom - front) / 5.5;
                for column in 0..9 {
                    let drift = (t * (6.0 + row as f32) + column as f32 * 137.0 - frame.pan() * z)
                        .rem_euclid(width + 80.0);
                    let x = ox + drift - 40.0;
                    let pulse = 0.5 + 0.5 * (t * 1.1 + column as f32 + row as f32 * 0.7).sin();
                    window.rect(
                        x,
                        y,
                        (18.0 + 10.0 * pulse) * z.min(1.5),
                        2.0,
                        1.0,
                        shimmer.opacity(shimmer.a * pulse),
                    );
                }
            }
        }
    }

    // At dusk and at night the lit windows shine on the water, trembling.
    // Zoomed out to the postcard, smoke and reflections are too small to
    // see, and not drawn.
    let fine = z >= 0.6;
    if fine && frame.water && lit && !frame.buildings.is_empty() {
        let front = screen(0.0, frame.front).1;
        let bottom = oy + height;
        let deep = (bottom - front).max(0.0);
        let warm = art::hex(0xffcf7a);
        for building in &frame.buildings {
            if !seen(building.x, building.w * z) {
                continue;
            }
            let (x, _) = screen(building.x, building.base);
            let seed = building.index as f32 * 1.7;
            for streak in 0..3 {
                let along = (streak as f32 - 1.0) * building.w * z * 0.18;
                let pulse = 0.6 + 0.4 * (t * 1.3 + seed + streak as f32 * 2.1).sin();
                let reach = deep * (0.28 + 0.1 * streak as f32);
                window.soft(
                    x + along + (t * 0.9 + seed + streak as f32).sin() * 1.5,
                    front + 6.0 * z + reach / 2.0,
                    building.w * z * 0.025,
                    reach / 2.0,
                    building.w * z * 0.03,
                    warm.opacity(0.13 * pulse),
                );
            }
        }
    }

    // Buildings springing after a click, or rising just built.
    for building in frame.buildings.iter().filter(|b| b.moving()) {
        if !seen(building.x, building.w * z) {
            continue;
        }
        let (x, base) = screen(building.x, building.base);
        let (w, h) = (building.w * z, building.h * z);
        let grow = ease(building.grow);
        window.soft(x, base, w * 0.56 * grow, h * 0.05, h * 0.06, contact);
        let mut tinted = Tint::new(window, light);
        let mut posed = Xform::about(
            &mut tinted,
            (x, base),
            building.squash.0 * (0.7 + 0.3 * grow),
            building.squash.1 * grow,
            0.0,
        );
        match &building.drawing {
            Some(drawing) => art::paint_drawing(
                &mut posed,
                x,
                base,
                w,
                h,
                drawing,
                &Inks::of_place(&building.palette).lit(lit),
                Stance::Standing,
                world_projection::Mood::Content,
                0.0,
                0.0,
                1.0,
            ),
            None => {
                art::paint_building(&mut posed, x, base, w, h, building.shape, &building.palette)
            }
        }
    }

    // A chimney smokes: puffs rising, spreading and thinning, carried by
    // the wind.
    if fine && frame.weather != Weather::Storm {
        let blow = wind(frame);
        let smoke = if night {
            art::hex(0x8a90a0)
        } else {
            art::hex(0xcfcbc6)
        };
        for building in &frame.buildings {
            if building.shape != MarkShape::House || building.drawing.is_some() {
                continue;
            }
            if !seen(building.x, building.w * z * 2.0) {
                continue;
            }
            let (pot_x, pot_y) = art::chimney_top(
                building.x,
                building.base,
                building.w,
                building.h,
                &building.palette,
            );
            let (cx, cy) = screen(pot_x, pot_y);
            let (w, h) = (building.w * z, building.h * z);
            for puff in 0..5 {
                let age = (t * 0.3 + puff as f32 * 0.2 + building.index as f32 * 0.37) % 1.0;
                let rise = ease(age.min(1.0)) * h * 0.55;
                let drift = age * age * blow * w * 0.35;
                let r = w * (0.045 + age * 0.08);
                window.soft(
                    cx + drift,
                    cy - rise,
                    r,
                    r * 0.85,
                    r * 0.5,
                    smoke.opacity(0.6 * (1.0 - age) * (0.3 + 0.7 * (age * 6.0).min(1.0))),
                );
            }
        }
    }

    // Things: carts, parcels, boats riding the swell.
    let mut things = frame.things.iter().collect::<Vec<_>>();
    things.sort_by(|a, b| a.base.total_cmp(&b.base).then(a.index.cmp(&b.index)));
    for thing in things {
        if !seen(thing.x, thing.w * z) {
            continue;
        }
        let (x, base) = screen(thing.x, thing.base);
        let w = thing.w * z * (0.6 + 0.4 * ease(thing.grow));
        if let Some(glow) = thing.glow {
            window.soft(x, base, w * 0.8, w * 0.16, w * 0.12, glow.opacity(0.35));
        }
        if thing.shape == MarkShape::Boat {
            // A darker patch of water under the hull.
            window.soft(
                x,
                base + w * 0.04,
                w * 0.5,
                w * 0.06,
                w * 0.06,
                contact.opacity(0.12),
            );
        } else {
            window.soft(x, base, w * 0.48, w * 0.07, w * 0.06, contact);
        }
        let mut tinted = Tint::new(window, light);
        let mut rolled = Xform::turned(&mut tinted, (x, base), thing.roll);
        match &thing.drawing {
            Some(drawing) => art::paint_drawing(
                &mut rolled,
                x,
                base,
                w,
                w / drawing.aspect,
                drawing,
                &Inks::of_place(&thing.palette),
                Stance::Standing,
                world_projection::Mood::Content,
                thing.sway * 0.3,
                0.0,
                1.0,
            ),
            None => art::paint_thing(
                &mut rolled,
                x,
                base,
                w,
                thing.shape,
                &thing.palette,
                thing.sway,
            ),
        }
    }

    // People: a soft shadow where they stand, a longer one away from a
    // sun that is out, and themselves.
    let long = 0.22 + 0.6 * (1.0 - high.max(0.0));
    let away = if across < 0.0 { 1.0 } else { -1.0 };
    for person in &frame.people {
        let (x, y) = (ox + person.x, oy + person.y);
        if x + person.height < ox || x - person.height > ox + width {
            continue;
        }
        if let Some(glow) = person.glow {
            let breathe = 0.9 + 0.1 * (t * 2.4).sin();
            window.soft(
                x,
                y,
                person.height * 0.8 * breathe,
                person.height * 0.2 * breathe,
                person.height * 0.14,
                glow.opacity(0.45),
            );
        }
        // Lifted off the ground in a hop, the shadow shrinks and fades.
        let lift = (person.pose.bob / person.height.max(1.0) * 4.0).clamp(0.0, 0.6);
        if day {
            let reach = person.height * long * 0.8;
            window.soft(
                x + away * reach * 0.5,
                y + person.height * 0.015,
                person.height * 0.1 + reach * 0.5,
                person.height * 0.035,
                person.height * 0.05,
                gpui::black().opacity(0.1 * (1.0 - lift)),
            );
        }
        window.soft(
            x,
            y,
            person.height * 0.2 * (1.0 - lift * 0.4),
            person.height * 0.045,
            person.height * 0.05,
            contact.opacity(contact.a * (1.0 - lift)),
        );
        let mut tinted = Tint::new(window, light);
        match &person.drawing {
            Some(drawing) => art::paint_drawing_posed(
                &mut tinted,
                x,
                y,
                person.height * drawing.aspect,
                person.height,
                drawing,
                &Inks::of_person(&person.figure),
                person.stance,
                person.mood,
                person.pose,
            ),
            None => art::paint_figure(
                &mut tinted,
                x,
                y,
                person.height,
                &person.figure,
                person.pose,
            ),
        }
    }
    for (x, y, r, tone) in &frame.bonds {
        art::paint_bond(window, ox + x, oy + y, *r, *tone);
    }
}

/// Rain, snow, dust and fog over the whole scene, and lightning in a storm.
fn paint_weather(
    window: &mut dyn Brush,
    frame: &Frame,
    ox: f32,
    oy: f32,
    width: f32,
    height: f32,
    k: f32,
) {
    let t = frame.seconds;
    // Each drop or flake has its own place in a repeating fall.
    let fall = |index: u32, speed: f32, drift: f32| {
        let mut seed = index.wrapping_mul(0x9e37_79b9) ^ 0x85eb_ca6b;
        seed ^= seed >> 15;
        let x0 = (seed % 1000) as f32 / 1000.0;
        let phase = ((seed / 1000) % 1000) as f32 / 1000.0;
        let y = ((phase + t * speed / height.max(1.0)) % 1.0) * (height + 40.0) - 20.0;
        let x = (x0 * (width + 120.0) + y * drift) % (width + 120.0) - 60.0;
        (ox + x, oy + y)
    };
    match frame.weather {
        Weather::Rain | Weather::Storm => {
            let storm = frame.weather == Weather::Storm;
            let (count, speed, slant) = if storm {
                (220, 900.0, 0.35)
            } else {
                (120, 620.0, 0.12)
            };
            let ink = art::hex(0xdfe7ef).opacity(if storm { 0.55 } else { 0.45 });
            for index in 0..count {
                let (x, y) = fall(index, speed, slant);
                let len = (12.0 + (index % 5) as f32 * 2.0) * k;
                let mut streak = Shape::new();
                streak.move_to(x, y).line_to(x + len * slant, y + len);
                window.stroke(&streak, 1.0, ink);
            }
            // Now and then the sky lights up.
            if storm && !frame.still {
                let beat = t % 7.3;
                if beat < 0.12 || (0.2..0.26).contains(&beat) {
                    window.rect(ox, oy, width, height, 0.0, gpui::white().opacity(0.35));
                }
            }
        }
        Weather::Snow => {
            for index in 0..110 {
                let (x, y) = fall(index, 45.0 + (index % 7) as f32 * 6.0, 0.05);
                let sway = (t * 0.9 + index as f32).sin() * 6.0;
                art::circle(
                    window,
                    x + sway,
                    y,
                    (1.4 + (index % 3) as f32 * 0.7) * k,
                    gpui::white().opacity(0.85),
                );
            }
        }
        Weather::Dust => {
            for index in 0..140_u32 {
                let mut seed = index.wrapping_mul(0x2545_f491) ^ 0x68e3_1da4;
                seed ^= seed >> 13;
                let y = oy + ((seed % 1000) as f32 / 1000.0) * height;
                let x = ox
                    + ((((seed / 1000) % 1000) as f32 / 1000.0) * (width + 80.0)
                        + t * (60.0 + (index % 9) as f32 * 12.0))
                        % (width + 80.0)
                    - 40.0;
                art::circle(window, x, y, 1.2 * k, art::hex(0xe0a070).opacity(0.55));
            }
            let top = frame.at(0.0, frame.horizon).1.max(0.0);
            window.gradient(
                ox,
                oy + top,
                width,
                height - top,
                180.0,
                (art::hex(0xc07040).opacity(0.0), 0.0),
                (art::hex(0xc07040).opacity(0.18), 0.2),
            );
        }
        Weather::Fog => {
            for layer in fog_layers(frame.at(0.0, frame.horizon).1, width, height, t) {
                window.gradient(
                    ox + layer.x,
                    oy + layer.y,
                    layer.w,
                    layer.h,
                    180.0,
                    (gpui::white().opacity(layer.top), 0.0),
                    (gpui::white().opacity(layer.bottom), 1.0),
                );
            }
        }
        Weather::Clear | Weather::Cloudy => {}
    }
}

/// A layer of fog: where it lies on the scene, and how thick it is at its
/// top and bottom edges.
#[derive(Clone, Copy, Debug)]
struct FogLayer {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    top: f32,
    bottom: f32,
}

/// Fog for a scene: a veil thickening towards the ground and soft wisps
/// drifting across it. No layer has a hard edge inside the scene, so
/// nothing reads as a band however close the camera is.
fn fog_layers(horizon: f32, width: f32, height: f32, t: f32) -> Vec<FogLayer> {
    let top = horizon.clamp(0.0, height);
    let deep = (height - top).max(1.0);
    let mut layers = vec![FogLayer {
        x: 0.0,
        y: top,
        w: width,
        h: deep,
        top: if top > 0.0 { 0.0 } else { 0.1 },
        bottom: 0.26,
    }];
    for wisp in 0..3 {
        let centre = top + deep * (0.25 + 0.3 * wisp as f32);
        let half = deep * 0.09;
        let drift = (t * (4.0 + wisp as f32)) % 60.0;
        layers.push(FogLayer {
            x: -60.0 + drift,
            y: centre - half,
            w: width + 120.0,
            h: half,
            top: 0.0,
            bottom: 0.14,
        });
        layers.push(FogLayer {
            x: -60.0 + drift,
            y: centre,
            w: width + 120.0,
            h: half,
            top: 0.14,
            bottom: 0.0,
        });
    }
    layers
}

/// A World's cover: its landscape, what it has built, and its people and
/// buildings as it last stood, drawn the way its window draws them.
pub fn cover(
    scenery: Option<Scenery>,
    marks: &[MarkShape],
    cast: Vec<CanvasItem>,
    drawings: Vec<world_projection::Drawing>,
) -> gpui::Canvas<()> {
    // A cast placed along a panorama is shown on one as wide, looking at
    // its middle.
    let width = cast
        .iter()
        .filter_map(|item| item.px)
        .fold(None, |most: Option<f32>, px| {
            Some(most.map_or(px, |most| most.max(px)))
        })
        .map(|most| most.ceil().max(1.0));
    let snapshot = ProjectionSnapshot {
        scenery,
        drawings,
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
            width,
            ..Default::default()
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
pub(crate) mod tests {
    use super::*;
    use world_projection::{CanvasItem, CanvasItemKind, CanvasProjection};

    #[test]
    fn every_hour_has_a_light_of_its_own() {
        let grades = (0..24)
            .map(|hour| format!("{:?}", grade_at(hour as f32)))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(grades.len(), 24, "{grades:?}");
        // And it turns smoothly: half past is between the hours either side.
        let ((_, noon), _) = grade_at(12.0);
        let ((_, dusk), _) = grade_at(19.0);
        let ((_, between), _) = grade_at(15.5);
        assert!(between > noon.min(dusk) && between < noon.max(dusk));
    }

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
            drawing: None,
            stance: None,
            standing: None,
            mood: None,
            spot: None,
            px: None,
            home: None,
            day: Vec::new(),
            built: None,
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
                ..Default::default()
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

    /// The v0.16 bar: everyone has at least four idle behaviours over a
    /// few minutes of day, and a click brings a wave within the frame.
    #[test]
    fn everyone_has_idle_behaviours_and_waves_when_clicked() {
        let snapshot = harbour();
        let stage = stage(&snapshot, 1100.0, 848.0);
        let pinned = BTreeSet::new();
        let mut seen = vec![BTreeSet::new(); stage.people.len()];
        for tenth in 0..3000 {
            let lives = living(
                &stage,
                &snapshot,
                tenth as f32 / 10.0,
                Daylight::Day,
                &pinned,
                None,
            );
            for (index, life) in lives.iter().enumerate() {
                if let Some(stance) = life.stance {
                    seen[index].insert(stance);
                }
            }
        }
        for (index, stances) in seen.iter().enumerate() {
            assert!(stances.len() >= 4, "person {index}: {stances:?}");
        }
        let mut lives = living(&stage, &snapshot, 5.0, Daylight::Day, &pinned, None);
        let who = snapshot.canvas.items[stage.people[0].index].id;
        let poked = [(who, 0.3_f32)].into_iter().collect();
        let before = lives[0].pose.bob;
        wave(&mut lives, &stage, &snapshot, &poked, false);
        assert_eq!(lives[0].stance, Some(Stance::Waving));
        assert!(lives[0].pose.bob > before, "a hop");
        let later = [(who, WAVE_SECONDS + 0.1)].into_iter().collect();
        let mut settled = living(&stage, &snapshot, 5.0, Daylight::Day, &pinned, None);
        wave(&mut settled, &stage, &snapshot, &later, false);
        assert_ne!(settled[0].stance, Some(Stance::Waving));
    }

    #[test]
    fn every_hour_has_a_light_of_its_own_and_far_layers_move_least() {
        let grades = [
            Daylight::Dawn,
            Daylight::Day,
            Daylight::Dusk,
            Daylight::Night,
        ]
        .map(grade)
        .map(|((warm, a), (cool, b))| (warm, (a * 100.0) as u32, cool, (b * 100.0) as u32));
        let distinct = grades.iter().collect::<BTreeSet<_>>();
        assert_eq!(distinct.len(), 4);
        assert!(PARALLAX.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(PARALLAX.len() >= 3);
    }

    /// The v0.16 bar: working out a frame of a busy scene (fifteen people,
    /// a dozen buildings, twenty things) fits well inside a 60 fps frame,
    /// leaving the rest of the 16 ms for painting. Timed in a release
    /// build; a debug build only checks it finishes.
    #[test]
    fn a_busy_frame_is_worked_out_in_4_ms() {
        let mut items = Vec::new();
        for id in 0..12 {
            items.push(item(
                100 + id,
                CanvasItemKind::Place,
                id as f32 / 12.0,
                None,
            ));
        }
        for id in 0..20 {
            items.push(item(
                300 + id,
                CanvasItemKind::Object,
                id as f32 / 20.0,
                Some(100 + id % 12),
            ));
        }
        for id in 0..15 {
            items.push(item(
                id + 1,
                CanvasItemKind::Actor,
                id as f32 / 15.0,
                Some(100 + id % 12),
            ));
        }
        let snapshot = ProjectionSnapshot {
            canvas: CanvasProjection {
                items,
                links: Vec::new(),
                marks: Vec::new(),
                ..Default::default()
            },
            ..ProjectionSnapshot::default()
        };
        let pinned = BTreeSet::new();
        let runs = 200;
        let started = std::time::Instant::now();
        for run in 0..runs {
            let stage = stage(&snapshot, 1400.0, 900.0);
            let lives = living(
                &stage,
                &snapshot,
                run as f32 / 60.0,
                Daylight::Day,
                &pinned,
                None,
            );
            let frame = frame(
                &snapshot,
                &stage,
                &lives,
                Camera::whole(&stage),
                run as f32 / 60.0,
                Daylight::Day,
                &Glows::new(),
                1.0,
            );
            assert_eq!(frame.people.len(), 15);
        }
        let each = started.elapsed() / runs;
        if !cfg!(debug_assertions) {
            assert!(each.as_micros() < 4_000, "{each:?} a frame");
        }
    }

    /// No motion over time is linear: every move between two places is
    /// eased. Interpolations along a shape (a line, a curve) are drawing,
    /// not motion, and are named here.
    #[test]
    fn no_motion_is_linear() {
        let lerp = regex_lite_like;
        let sources = [
            ("diorama.rs", include_str!("diorama.rs")),
            ("art.rs", include_str!("art.rs")),
            ("scene.rs", include_str!("scene.rs")),
            (
                "macos/world_window.rs",
                include_str!("macos/world_window.rs"),
            ),
            ("macos.rs", include_str!("macos.rs")),
        ];
        let drawing = [
            "let fx = left + (right - left) * t;",
            "|t: f32| point(from.x + (to.x - from.x) * t",
        ];
        for (name, source) in sources {
            let lines = source.lines().collect::<Vec<_>>();
            for (index, line) in lines.iter().enumerate() {
                if !lerp(line) || drawing.iter().any(|allowed| line.contains(allowed)) {
                    continue;
                }
                let eased = lines[index.saturating_sub(8)..=index]
                    .iter()
                    .any(|nearby| nearby.contains("ease(") || nearby.contains("with_easing"));
                assert!(
                    eased,
                    "{name}:{}: linear motion: {}",
                    index + 1,
                    line.trim()
                );
            }
        }
    }

    /// Whether a line moves something by a raw fraction of time: `(to -
    /// from) * t`, `* progress`, `* phase`.
    fn regex_lite_like(line: &str) -> bool {
        let code = line.split("//").next().unwrap_or_default();
        ["* t)", "* t,", "* t;", "* progress", "* phase"]
            .iter()
            .any(|tail| code.contains(tail))
            && code.contains(" - ")
            && code.contains(") *")
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

    /// Fog never draws a band: down the scene its thickness never jumps,
    /// zoomed out or right in.
    #[test]
    fn fog_has_no_hard_edges_at_any_zoom() {
        let (width, height) = (1100.0_f32, 848.0_f32);
        for horizon in [-400.0, -60.0, 0.0, 180.0, 300.0, 620.0] {
            for t in [0.0, 7.5, 31.0] {
                let layers = fog_layers(horizon, width, height, t);
                let thickness = |y: f32| {
                    layers
                        .iter()
                        .filter(|layer| y >= layer.y && y < layer.y + layer.h)
                        .map(|layer| {
                            let at = (y - layer.y) / layer.h.max(1.0);
                            layer.top + (layer.bottom - layer.top) * at
                        })
                        .sum::<f32>()
                };
                let mut before = thickness(0.0);
                for y in 1..height as usize {
                    let now = thickness(y as f32);
                    assert!(
                        (now - before).abs() < 0.02,
                        "a band edge at {y} with the horizon at {horizon}"
                    );
                    before = now;
                }
            }
        }
    }

    /// The wheel zooms around the point under the pointer: it stays under
    /// the pointer, and the view never runs past the stage.
    #[test]
    fn zooming_keeps_the_point_under_the_pointer_and_stays_on_stage() {
        let snapshot = harbour();
        let stage = stage(&snapshot, 1100.0, 848.0);
        let camera = Camera::around(&stage, 1.6, 500.0, 420.0);
        let (x, y) = camera.at(&stage, 500.0, 420.0);
        let back = camera.stage_point(&stage, x, y);
        assert!((back.0 - 500.0).abs() < 0.01 && (back.1 - 420.0).abs() < 0.01);
        let corner = Camera::around(&stage, 9.0, 0.0, 0.0);
        assert!(corner.zoom <= 2.2);
        let (left, top) = corner.at(&stage, 0.0, 0.0);
        assert!(left.abs() < 0.01 && top.abs() < 0.01);
    }

    /// A World three years on: a real one's snapshot when
    /// `WORLD_GPUI_BENCH_SNAPSHOT` names its wire JSON (as
    /// `dump_snapshot` prints it), else one of the same size: a harbour
    /// three windows wide with 24 people on their day's rounds, 48 homes
    /// and works, 20 things, 62 things built and 5 under way.
    pub(crate) fn three_years() -> ProjectionSnapshot {
        if let Some(path) = std::env::var_os("WORLD_GPUI_BENCH_SNAPSHOT") {
            let json = std::fs::read_to_string(path).expect("the snapshot");
            let wire: world_pack_protocol::ProjectionSnapshotWire =
                serde_json::from_str(&json).expect("a wire snapshot");
            return ProjectionSnapshot::try_from(wire).expect("a snapshot");
        }
        let mut items = Vec::new();
        for id in 0..48_u64 {
            let mut place = item(100 + id, CanvasItemKind::Place, 0.0, None);
            place.px = Some(0.06 + 2.88 * id as f32 / 47.0);
            place.shape = Some(
                [
                    MarkShape::House,
                    MarkShape::House,
                    MarkShape::Shop,
                    MarkShape::Tree,
                    MarkShape::Tower,
                    MarkShape::House,
                ][id as usize % 6],
            );
            items.push(place);
        }
        for id in 0..20_u64 {
            let mut thing = item(300 + id, CanvasItemKind::Object, 0.0, Some(100 + id * 2));
            if id % 4 == 0 {
                thing.shape = Some(MarkShape::Boat);
            }
            items.push(thing);
        }
        for id in 0..24_u64 {
            let mut person = item(id + 1, CanvasItemKind::Actor, 0.0, Some(100 + id * 2));
            person.day = vec![
                world_projection::RoutineStop {
                    from_hour: 7,
                    at: entity(100 + (id * 5) % 48),
                    inside: false,
                },
                world_projection::RoutineStop {
                    from_hour: 21,
                    at: entity(100 + id * 2),
                    inside: id % 5 != 0,
                },
            ];
            items.push(person);
        }
        ProjectionSnapshot {
            scenery: Some(Scenery {
                sky_top: 0x9cc6e6,
                sky_bottom: 0xf0ead8,
                far: 0x8fae7e,
                near: 0x4f86a8,
                sun: 0xffe2a0,
            }),
            canvas: CanvasProjection {
                items,
                marks: (0..62)
                    .map(|n| world_projection::CanvasMark {
                        label: format!("Work {n}"),
                        shape: [MarkShape::House, MarkShape::Tree, MarkShape::Lamp][n % 3],
                        selection: None,
                    })
                    .collect(),
                width: Some(3.0),
                season: Some(Season::Winter),
                ground: Some(GroundCover::Snow),
                ice: true,
                ..Default::default()
            },
            ..ProjectionSnapshot::default()
        }
    }

    /// At 1440 by 900 at twice the pixels: painting the still layers
    /// afresh (a new hour, the weather, the season, a zoom) off the
    /// window's thread takes well under a second, and working out and
    /// drawing a frame over them under 8 ms. (That no frame on the window's
    /// thread waits for painting is `a_three_year_world_never_waits_for_painting`.) What this measures is the CPU's part: painting the
    /// images, and working out every primitive a frame hands GPUI. The
    /// GPU's part (compositing a few images and the live primitives) needs
    /// a Mac to measure.
    #[test]
    #[ignore = "a benchmark: cargo test --release -p world-gpui -- --ignored three_year"]
    fn a_three_year_world_paints_within_its_frame_budget() {
        use crate::brush::Tally;
        use std::time::{Duration, Instant};
        let snapshot = three_years();
        let (width, height, dpr) = (1440.0_f32, 900.0_f32, 2.0_f32);
        let mut report = Vec::new();
        let mut worst_still = Duration::ZERO;
        for (daylight, hour) in [
            (Daylight::Day, 12.0),
            (Daylight::Dusk, 19.5),
            (Daylight::Night, 22.5),
        ] {
            let stage = stage_at(&snapshot, width, height, Clock::at(hour as u8));
            let camera = Camera::around(&stage, 1.0, stage.width / 2.0, height / 2.0);
            let lives = living(&stage, &snapshot, 0.0, daylight, &BTreeSet::new(), None);
            let frame = frame(
                &snapshot,
                &stage,
                &lives,
                camera,
                0.0,
                daylight,
                &Glows::new(),
                1.0,
            );
            let mut frame = frame;
            frame.hour = hour;
            let started = Instant::now();
            let sky = paint_sky(&frame, width, height, dpr * SKY_RES).expect("a sky");
            let band = Band::of(&frame, width, height);
            let hills = paint_band(&frame, &band, dpr).expect("hills");
            // The ground as the window paints it: the land's tiles, the
            // buildings' sprites, then the buildings' tiles, each across the
            // painter's threads.
            let scale = dpr;
            let key = ground_key(&frame, scale).finish();
            let frame_ref = &frame;
            let land = tiles_in_view(&frame, Layer::Land, scale * 0.5, 0);
            let jobs = land
                .iter()
                .map(|&(column, row)| {
                    Box::new(move || {
                        paint_land_tile(frame_ref, column, row, scale * 0.5).map(painter::image_of)
                    })
                        as Box<
                            dyn FnOnce() -> Option<std::sync::Arc<gpui::RenderImage>> + Send + '_,
                        >
                })
                .collect::<Vec<_>>();
            let mut tiles = painter::parallel(jobs).len();
            let places = tiles_in_view(&frame, Layer::Buildings, scale, 0);
            prepare_sprites(&frame, key, scale, &places);
            let jobs = places
                .iter()
                .map(|&(column, row)| {
                    Box::new(move || {
                        paint_building_tile(frame_ref, key, column, row, scale)
                            .map(painter::image_of)
                    })
                        as Box<
                            dyn FnOnce() -> Option<std::sync::Arc<gpui::RenderImage>> + Send + '_,
                        >
                })
                .collect::<Vec<_>>();
            tiles += painter::parallel(jobs).into_iter().flatten().count();
            let vignette = paint_vignette(width, height, dpr * 0.25).expect("a vignette");
            let took = started.elapsed();
            worst_still = worst_still.max(took);
            // What an image costs to hand to GPUI: straight BGRA.
            let started = Instant::now();
            let _ = (
                painter::image(&sky),
                painter::image(&hills),
                painter::image(&vignette),
            );
            let upload = started.elapsed();
            report.push(format!(
                "{daylight:?}: still layers {:.1} ms ({tiles} tiles), images {:.1} ms",
                took.as_secs_f64() * 1000.0,
                upload.as_secs_f64() * 1000.0
            ));
        }
        // Panning into a new column of tiles.
        let stage = stage_at(&snapshot, width, height, Clock::at(12));
        let lives = living(
            &stage,
            &snapshot,
            0.0,
            Daylight::Day,
            &BTreeSet::new(),
            None,
        );
        let camera = Camera::around(&stage, 1.0, stage.width / 2.0, height / 2.0);
        let frame_now = frame(
            &snapshot,
            &stage,
            &lives,
            camera,
            0.0,
            Daylight::Day,
            &Glows::new(),
            1.0,
        );
        let started = Instant::now();
        let column = ((camera.x + width / 2.0) * dpr / TILE as f32) as i32 + 1;
        let key = ground_key(&frame_now, dpr).finish();
        for row in 0..8 {
            let _ = paint_land_tile(&frame_now, column / 2, row / 2, dpr * 0.5);
            let _ = paint_building_tile(&frame_now, key, column, row, dpr);
        }
        let pan = started.elapsed();
        // Frames: laying out, living, working out and drawing what moves.
        let runs = 120;
        let mut tally = Tally::default();
        let started = Instant::now();
        for run in 0..runs {
            let seconds = run as f32 / 60.0;
            let stage = stage_at(&snapshot, width, height, Clock::at(12));
            let lives = living(
                &stage,
                &snapshot,
                seconds,
                Daylight::Day,
                &BTreeSet::new(),
                None,
            );
            let frame = frame(
                &snapshot,
                &stage,
                &lives,
                camera,
                seconds,
                Daylight::Day,
                &Glows::new(),
                1.0,
            );
            let _ = ground_key(&frame, dpr).finish();
            paint_clouds(&mut tally, &frame, 0.0, 0.0, width, height);
            paint_live(&mut tally, &frame, 0.0, 0.0, width, height, [1.0; 3]);
            paint_weather(&mut tally, &frame, 0.0, 0.0, width, height, 1.0);
        }
        let each = started.elapsed() / runs;
        report.push(format!(
            "panning into a new column: {:.1} ms; a frame: {:.2} ms, {} quads, {} paths, {} soft shapes",
            pan.as_secs_f64() * 1000.0,
            each.as_secs_f64() * 1000.0,
            tally.quads / runs as usize,
            tally.paths / runs as usize,
            tally.soft / runs as usize,
        ));
        eprintln!("{}", report.join("\n"));
        // Where the time went, summed over every thread.
        eprintln!("{:?}", painter::profile().lock().unwrap());
        if !cfg!(debug_assertions) {
            // Painted off the window's thread, the still layers need only
            // arrive soon: well within a second, then a third of one to
            // fade in.
            assert!(worst_still < Duration::from_millis(400), "{report:?}");
            assert!(each < Duration::from_millis(8), "{report:?}");
        }
    }

    /// A panorama three windows wide puts what has a `px` where it says,
    /// the camera pans along it without looking past either end, and
    /// zoomed right out it sees the whole of it.
    #[test]
    fn a_panorama_puts_things_at_their_px_and_the_camera_keeps_to_it() {
        let mut snapshot = harbour();
        snapshot.canvas.width = Some(3.0);
        snapshot.canvas.items[0].px = Some(2.5);
        let stage = stage_at(&snapshot, 1000.0, 800.0, Clock::at(12));
        assert_eq!(stage.width, 3000.0);
        assert_eq!(stage.view_w, 1000.0);
        let far = stage.buildings.iter().find(|spot| spot.index == 0).unwrap();
        assert!((far.x - 2500.0).abs() < 0.01);
        let east = Camera::around(&stage, 1.0, 1e6, 400.0);
        assert!((east.x - 2500.0).abs() < 0.01, "{east:?}");
        let (right, _) = east.at(&stage, stage.width, 0.0);
        assert!((right - 1000.0).abs() < 0.01);
        let west = Camera::around(&stage, 1.0, -1e6, 400.0);
        assert!((west.x - 500.0).abs() < 0.01);
        let whole = Camera::around(&stage, 0.01, 0.0, 0.0);
        assert!((whole.zoom - 1.0 / 3.0).abs() < 1e-4);
        let (left, _) = whole.at(&stage, 0.0, 0.0);
        let (right, bottom) = whole.at(&stage, stage.width, stage.height);
        assert!(left.abs() < 0.01 && (right - 1000.0).abs() < 0.01);
        // A panoramic postcard, centred: as much paper above as below.
        let (_, top) = whole.at(&stage, 0.0, card_top(stage.horizon, stage.height));
        assert!((top - (800.0 - bottom)).abs() < 0.01, "{top} {bottom}");
        // Without a width, nothing changes from one window.
        let plain = stage_at(&harbour(), 1000.0, 800.0, Clock::at(12));
        assert_eq!(plain.width, plain.view_w);
    }

    /// People follow their day: at work by noon, home and indoors (not
    /// standing outside) at night, and in the first moments of an hour
    /// that moves them, on their way.
    #[test]
    fn people_keep_to_their_day_and_are_indoors_at_night() {
        let mut snapshot = harbour();
        let (home, work) = (entity(101), entity(102));
        for index in 5..snapshot.canvas.items.len() {
            snapshot.canvas.items[index].day = vec![
                world_projection::RoutineStop {
                    from_hour: 8,
                    at: work,
                    inside: false,
                },
                world_projection::RoutineStop {
                    from_hour: 21,
                    at: home,
                    inside: true,
                },
            ];
        }
        let noon = stage_at(&snapshot, 1100.0, 848.0, Clock::at(12));
        let work_x = noon
            .buildings
            .iter()
            .find(|spot| spot.index == 1)
            .unwrap()
            .x;
        for spot in noon.people.iter().filter(|spot| spot.index >= 5) {
            assert!((spot.x - work_x).abs() < noon.building_w * 1.5, "{spot:?}");
        }
        assert!(noon.inside.is_empty());
        let night = stage_at(&snapshot, 1100.0, 848.0, Clock::at(23));
        assert!(night.people.iter().all(|spot| spot.index < 5));
        assert_eq!(night.inside.len(), snapshot.canvas.items.len() - 5);
        assert!(night.inside.iter().all(|(_, place)| *place == 0));
        // Just after eight, on the way to work from home.
        let leaving = stage_at(
            &snapshot,
            1100.0,
            848.0,
            Clock {
                hour: 8,
                into_hour: 0.5,
            },
        );
        assert!(!leaving.routes.is_empty());
        let lives = living(
            &leaving,
            &snapshot,
            0.5,
            Daylight::Day,
            &BTreeSet::new(),
            None,
        );
        assert!(lives.iter().any(|life| life.pose.stride.is_some()));
    }

    /// The painter is a pure function of what it is given: the same tile
    /// twice is the same pixels.
    #[test]
    fn the_same_frame_paints_the_same_pixels() {
        let snapshot = harbour();
        let stage = stage_at(&snapshot, 640.0, 420.0, Clock::at(19));
        let lives = living(
            &stage,
            &snapshot,
            0.0,
            Daylight::Dusk,
            &BTreeSet::new(),
            None,
        );
        let frame = frame(
            &snapshot,
            &stage,
            &lives,
            Camera::whole(&stage),
            0.0,
            Daylight::Dusk,
            &Glows::new(),
            1.0,
        )
        .at_hour(19.5);
        let key = ground_key(&frame, 2.0).finish();
        let row = (stage.base * 2.0 / TILE as f32) as i32;
        let a = paint_building_tile(&frame, key, 1, row, 2.0).map(|p| p.data().to_vec());
        let b = paint_building_tile(&frame, key, 1, row, 2.0).map(|p| p.data().to_vec());
        assert_eq!(a, b);
        let a = paint_land_tile(&frame, 0, 1, 1.0).unwrap();
        let b = paint_land_tile(&frame, 0, 1, 1.0).unwrap();
        assert!(a.data() == b.data());
        assert_eq!(
            paint_sky(&frame, 640.0, 420.0, 1.0).unwrap().data(),
            paint_sky(&frame, 640.0, 420.0, 1.0).unwrap().data()
        );
    }

    /// With Reduce Motion a click still waves, but nobody hops; and nobody
    /// near a speaker is left facing away.
    #[test]
    fn reduce_motion_keeps_the_wave_and_drops_the_hop_and_heads_turn_to_a_speaker() {
        let snapshot = harbour();
        let stage = stage(&snapshot, 1100.0, 848.0);
        let who = snapshot.canvas.items[stage.people[0].index].id;
        let poked = [(who, 0.2)].into_iter().collect();
        let mut hop = living(
            &stage,
            &snapshot,
            0.0,
            Daylight::Day,
            &BTreeSet::new(),
            None,
        );
        wave(&mut hop, &stage, &snapshot, &poked, false);
        let mut still = living(
            &stage,
            &snapshot,
            0.0,
            Daylight::Day,
            &BTreeSet::new(),
            None,
        );
        let rest = still[0].pose;
        wave(&mut still, &stage, &snapshot, &poked, true);
        assert_eq!(still[0].stance, Some(Stance::Waving));
        assert_eq!(still[0].pose, rest);
        assert!(hop[0].pose.bob > rest.bob && hop[0].pose.squash != 1.0);

        let lives = living(
            &stage,
            &snapshot,
            0.0,
            Daylight::Day,
            &BTreeSet::new(),
            None,
        );
        let mut frame = frame(
            &snapshot,
            &stage,
            &lives,
            Camera::whole(&stage),
            0.0,
            Daylight::Day,
            &Glows::new(),
            1.0,
        );
        let speaker = frame.people[0].clone();
        frame.listen(speaker.index, 1.0);
        for person in frame.people.iter().skip(1) {
            if (person.x - speaker.x).abs() < speaker.height * 5.0 && person.pose.stride.is_none() {
                assert_eq!(person.pose.facing.signum(), (speaker.x - person.x).signum());
            }
        }
    }

    /// A step plants each foot: while it is on the ground it moves back
    /// evenly under the body, and only lifts while it swings through.
    #[test]
    fn feet_plant_while_on_the_ground() {
        let planted = (0..50)
            .map(|n| art::step(n as f32 / 100.0, 1.0))
            .collect::<Vec<_>>();
        assert!(planted.iter().all(|(_, lift)| *lift == 0.0));
        let steps = planted
            .windows(2)
            .map(|pair| pair[1].0 - pair[0].0)
            .collect::<Vec<_>>();
        assert!(steps
            .iter()
            .all(|step| (step - steps[0]).abs() < 1e-4 && *step < 0.0));
        assert!(art::step(0.75, 1.0).1 > 0.9, "the foot is lifted mid-swing");
    }

    /// What the still layers are keyed by does not move with the seconds:
    /// a frame later they are the same images, and nothing is painted
    /// again until the hour, the weather, the season, the zoom or the size
    /// changes.
    #[test]
    fn the_still_layers_keep_their_keys_as_time_passes() {
        let snapshot = harbour();
        let stage = stage_at(&snapshot, 1100.0, 848.0, Clock::at(12));
        let keys = |seconds: f32| {
            let lives = living(
                &stage,
                &snapshot,
                seconds,
                Daylight::Day,
                &BTreeSet::new(),
                None,
            );
            let frame = frame(
                &snapshot,
                &stage,
                &lives,
                Camera::whole(&stage),
                seconds,
                Daylight::Day,
                &Glows::new(),
                1.0,
            )
            .at_hour(12.2);
            let mut look = Key::new("look");
            look_key(&frame, &mut look);
            (look.finish(), ground_key(&frame, 2.0).finish())
        };
        assert_eq!(keys(0.0), keys(7.3));
        assert_eq!(keys(0.0), keys(61.0));
    }

    /// Every slope in the painted layers is smooth: a 30° roof edge is
    /// anti-aliased, and the light, the ink and the snow laid along it
    /// follow it by coverage, never in whole-pixel steps. A staircase shows
    /// as rows that take the effect in alternately larger and smaller
    /// amounts; a smooth edge takes the same amount on every row.
    #[test]
    fn roof_slopes_are_smooth_and_what_is_laid_along_them_has_no_steps() {
        let roof = art::hex(0x3f6a8a);
        let slope = (30.0_f32).to_radians().tan();
        let paint = || {
            let mut canvas = Canvas::new(320, 140, 1.0, (0.0, 0.0)).unwrap();
            art::polygon(
                &mut canvas,
                &[
                    (160.0, 20.0),
                    (310.0, 20.0 + 150.0 * slope),
                    (10.0, 20.0 + 150.0 * slope),
                ],
                roof,
            );
            canvas.pixmap
        };
        let bare = paint();
        let at = |pixmap: &sk::Pixmap, x: usize, y: usize| {
            let pixel = &pixmap.data()[(y * 320 + x) * 4..][..4];
            [
                pixel[0] as f32,
                pixel[1] as f32,
                pixel[2] as f32,
                pixel[3] as f32,
            ]
        };
        let rows = 40..90;
        // The edge itself is anti-aliased: every row crosses it through
        // pixels partly covered.
        for y in rows.clone() {
            let partial = (160..320).any(|x| (20.0..235.0).contains(&at(&bare, x, y)[3]));
            assert!(partial, "row {y} of the slope has no anti-aliased pixel");
        }
        // Where along each row of one slope an effect lies: the middle of
        // what it changed. Along a straight edge those middles lie on a
        // straight line; in steps, they jump about it.
        let middles = |before: &sk::Pixmap,
                       after: &sk::Pixmap,
                       rows: std::ops::Range<usize>,
                       xs: std::ops::Range<usize>| {
            rows.map(|y| {
                let (mut sum, mut weighted) = (0.0_f32, 0.0_f32);
                for x in xs.clone() {
                    let (a, b) = (at(before, x, y), at(after, x, y));
                    let change = (0..3).map(|c| (a[c] - b[c]).abs()).sum::<f32>();
                    sum += change;
                    weighted += change * x as f32;
                }
                (y as f32, if sum > 0.0 { weighted / sum } else { f32::NAN })
            })
            .collect::<Vec<_>>()
        };
        // A smooth line bends gently from row to row; steps jump.
        let straight = |name: &str, points: Vec<(f32, f32)>| {
            assert!(
                points.iter().all(|(_, x)| x.is_finite()),
                "{name} laid nothing"
            );
            let worst = points
                .windows(3)
                .map(|w| (w[2].1 - 2.0 * w[1].1 + w[0].1).abs())
                .fold(0.0_f32, f32::max);
            assert!(
                worst < 0.3,
                "{name} steps along the slope: a jump of {worst:.2} px"
            );
        };
        let mut lit = paint();
        painter::rim(&mut lit, 1.0, 2, [1.0, 0.76, 0.48], 0.6);
        straight(
            "the low sun's line",
            middles(&bare, &lit, rows.clone(), 160..320),
        );
        let mut snowed = paint();
        painter::cap(&mut snowed, 4.0, [0.95, 0.97, 0.99], 0.94);
        straight("snow", middles(&bare, &snowed, rows.clone(), 0..160));
        // And the ink under an eave: the same roof turned over.
        let eave = || {
            let mut canvas = Canvas::new(320, 140, 1.0, (0.0, 0.0)).unwrap();
            art::polygon(
                &mut canvas,
                &[(10.0, 10.0), (310.0, 10.0), (160.0, 10.0 + 150.0 * slope)],
                roof,
            );
            canvas.pixmap
        };
        let bare = eave();
        let mut inked = eave();
        painter::ground_line(&mut inked, 1, 0.32);
        straight(
            "the ink under an eave",
            middles(&bare, &inked, 30..70, 0..160),
        );
    }
}
