//! A recorded Event's scene on the living stage (v0.30 S3c): its cues,
//! as the World worked them out ([`SceneScript`]), found on the stage the
//! window draws and played out `t` seconds in.
//!
//! [`stage_scene`] is pure: the same script on the same stage at the same
//! moment is the same picture, with no clock but `t`. A mark is found on
//! the stage by what is there (a place, beside someone, someone's home,
//! off an edge of the view); one it cannot find (a place since pulled
//! down, someone not out) falls back to the scene's place. Everyone the
//! scene stages stands on open ground (never in a building, on a thing or
//! on a prop), and walks along their line with the walk everyone else
//! walks, never through a building. A prop beside someone goes where they
//! go; a ferry, a shuttle, a bus or a sled only comes to the edge of the
//! view and goes again. The camera eases from one cue's view to the next,
//! and follows whoever it is on.
//!
//! Presentation only: nothing here is recorded, and nobody is asked to
//! decide anything; the words said are the script's, which are the
//! World's.

use super::*;
use std::sync::Arc;
use world_projection::{Act, Mark, MomentKind, Prop, SceneScript};

/// How long the camera takes from one cue's view to the next.
pub const CAMERA_EASE: f32 = 1.2;

/// How long a prop takes to come or go.
pub const PROP_FADE: f32 = 0.4;

/// How long a ferry, shuttle, bus or sled takes to come in to the edge of
/// the view, or go.
pub const TRANSPORT_SLIDE: f32 = 1.4;

/// How brisk a scene's walk is, as a share of a stroll; and the most a
/// walk is hurried so someone is where their next cue needs them.
const BRISK: f32 = 1.15;
const HURRIED: f32 = 2.0;

/// How far apart two of the cast stand, middle to middle, in figure
/// heights: a body's width between them (a body is 0.42 P wide), unless a
/// cue puts one right up against the other.
pub const APART: f32 = 0.84;

/// Room kept between a prop and a person or another prop, in figure
/// heights.
const CLEAR: f32 = 0.12;

/// How far in front of the people's line someone walking a distance goes,
/// in figure heights: the near lane, in front of whoever stands and
/// whatever stands on their ground, so no walk goes through anyone.
const LANE: f32 = 0.72;

/// How long after reaching their door someone goes in.
const GOING_IN: f32 = 0.3;

/// How long a scene's storm takes to set in.
const STORM_IN: f32 = 1.5;

/// The window's zoom for a script's camera zoom: 1 is the place as the
/// window shows it at rest, a little closer, and a scene never pulls back
/// further than that (a long place folding into a postcard mid-scene).
pub fn scene_zoom(zoom: f32) -> f32 {
    (0.2 + zoom).clamp(1.0, ZOOM_MOST)
}

/// Someone a scene stages, as it has them this moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StagedPerson {
    /// Their item on the canvas.
    pub index: usize,
    pub who: SelectionId,
    /// Where their feet are, in stage pixels.
    pub x: f32,
    pub y: f32,
    /// Which way they face: -1 left, 1 right.
    pub facing: f32,
    /// How they stand (or, walking, how they walk).
    pub stance: Stance,
    /// Walking: from and to along their line, and how far into how long a
    /// walk (seconds).
    pub walk: Option<(f32, f32, f32, f32)>,
    /// Carrying a load as they walk.
    pub loaded: bool,
    /// How they look, when the scene says: cross in a quarrel, happy at
    /// a cheer or a dance.
    pub mood: Option<world_projection::Mood>,
}

impl StagedPerson {
    /// How they are drawn this frame, breathing `breathe` (-1 to 1) at
    /// their own `pace`.
    pub(super) fn living(&self, breathe: f32, pace: f32, figure_h: f32) -> Living {
        if let Some((from, to, since, length)) = self.walk {
            return walking(from, to, (since, length), pace, self.loaded, figure_h);
        }
        let waving = self.stance == Stance::Waving;
        Living {
            x: self.x,
            pose: Pose {
                stride: None,
                bob: if self.stance == Stance::Sitting {
                    0.0
                } else {
                    breathe * 0.6
                },
                facing: self.facing,
                squash: 1.0 + 0.008 * breathe,
                lean: 0.0,
            },
            stance: waving.then_some(Stance::Waving),
            using: (!waving).then_some(self.stance),
            loaded: false,
        }
    }
}

/// A prop a scene shows, as it stands this moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StagedProp {
    pub prop: Prop,
    /// Its foot, in stage pixels (a ferry's waterline, bunting's middle).
    pub x: f32,
    pub y: f32,
    /// How wide and tall it stands, in stage pixels.
    pub w: f32,
    pub h: f32,
    /// How much of it shows (0 to 1) as it comes and goes.
    pub shown: f32,
    /// Held by whom (an item index), at their side.
    pub held_by: Option<usize>,
    /// Facing (a ferry's bow, a bag's side): -1 left, 1 right.
    pub facing: f32,
}

/// A scene `t` seconds in: who stands where, what props are out, where the
/// camera looks, and who is saying what.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Staged {
    pub people: Vec<StagedPerson>,
    pub props: Vec<StagedProp>,
    /// Where the camera looks, eased between the script's camera cues.
    pub camera: Option<Camera>,
    /// Who says what, and how long ago they began: only words the script
    /// records, by someone on the stage.
    pub bubble: Option<(SelectionId, String, f32)>,
    /// Rain over the whole stage.
    pub rain: bool,
    /// How far a storm has set in (0 to 1): the sun gone behind cloud, the
    /// light cooler and dimmer, the rain heavy.
    pub storm: f32,
    /// Those of the cast who have gone indoors (item indices): drawn by
    /// nobody, out of everyone's day.
    pub indoors: Vec<usize>,
    /// How far in, and how long the scene plays.
    pub t: f32,
    pub seconds: f32,
}

impl Staged {
    /// Whoever the scene stages at item `index`.
    pub fn person(&self, index: usize) -> Option<&StagedPerson> {
        self.people.iter().find(|person| person.index == index)
    }

    /// What it dresses a frame with: its props and its weather.
    pub fn dress(&self) -> Dress {
        Dress {
            props: self.props.clone(),
            rain: self.rain,
            storm: self.storm,
            moods: self
                .people
                .iter()
                .filter_map(|person| Some((person.index, person.mood?)))
                .collect(),
        }
    }
}

/// What a scene adds to a frame besides its people: its props, and rain.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dress {
    pub props: Vec<StagedProp>,
    pub rain: bool,
    /// How far a storm has set in (0 to 1).
    pub storm: f32,
    /// How the cast look, by item index, when the scene says.
    pub moods: Vec<(usize, world_projection::Mood)>,
}

impl Frame {
    /// The frame dressed for a scene: its props stand in depth with
    /// everything else, drawn live, and its rain falls.
    pub fn dressed(mut self, dress: Dress) -> Self {
        for person in &mut self.people {
            if let Some((_, mood)) = dress.moods.iter().find(|(at, _)| *at == person.index) {
                person.mood = *mood;
            }
        }
        self.scene = Some(Arc::new(dress));
        self
    }

    /// How far a scene's storm has set in over it (0 to 1).
    pub(crate) fn scene_storm(&self) -> f32 {
        self.scene.as_deref().map_or(0.0, |dress| dress.storm)
    }

    /// The props a scene shows in it, if one is playing.
    pub(crate) fn scene_props(&self) -> &[StagedProp] {
        self.scene.as_deref().map_or(&[], |dress| &dress.props)
    }

    /// The weather drawn over it: the World's, or a scene's rain.
    pub(crate) fn shown_weather(&self) -> Weather {
        let scene = self.scene.as_deref();
        match self.weather {
            weather if weather == Weather::Storm => weather,
            _ if scene.is_some_and(|dress| dress.storm > 0.3) => Weather::Storm,
            Weather::Clear | Weather::Cloudy if scene.is_some_and(|dress| dress.rain) => {
                Weather::Rain
            }
            weather => weather,
        }
    }
}

/// A scene's cast put on the stage: everyone it stages where it has them
/// this moment, brought out of doors if their day has them in, with
/// nothing of their day left to do (no stop, no errand, no walk to it).
/// Everyone else keeps their day.
pub fn cast_on(stage: &Stage, staged: &Staged) -> Stage {
    let mut out = stage.clone();
    // Nobody else sets off anywhere while it plays: they are where their
    // day has them, which the scene was staged clear of.
    out.routes.clear();
    // Whoever has gone in is nobody's to draw.
    for index in &staged.indoors {
        out.people.retain(|spot| spot.index != *index);
        out.routes.remove(index);
        out.doing.remove(index);
    }
    for person in &staged.people {
        out.inside.retain(|(who, _)| *who != person.index);
        out.routes.remove(&person.index);
        out.doing.remove(&person.index);
        out.shown_from.remove(&person.index);
        let y = person.y.max(stage.feet - stage.figure_h * 0.4);
        let spot = Spot {
            index: person.index,
            x: person.x,
            y,
            w: stage.figure_h * 0.5,
            scale: 1.0 + (y - stage.feet) / stage.figure_h.max(1.0) * 0.35,
        };
        match out
            .people
            .iter_mut()
            .find(|spot| spot.index == person.index)
        {
            Some(at) => *at = spot,
            None => out.people.push(spot),
        }
    }
    out.people.sort_by_key(|spot| spot.index);
    out
}

/// One leg of someone's part: from where to where, between when.
#[derive(Clone, Copy, Debug)]
struct Leg {
    t0: f32,
    t1: f32,
    from: (f32, f32),
    to: (f32, f32),
    /// How they stand once there (a placement's stance, or Standing).
    after: Stance,
    /// Walking in this stance (Walking, or Carrying with a load).
    moving: Stance,
    /// Walking a distance: the near lane they walk along, stepping out to
    /// it and back.
    lane: Option<f32>,
    /// Going in at their door once there (or off the edge of the view).
    indoors: bool,
    /// How long they stand there before they go.
    pause: f32,
}

/// One of the cast's part in a scene.
#[derive(Clone, Debug)]
struct Part {
    index: usize,
    who: SelectionId,
    legs: Vec<Leg>,
    strikes: Vec<(f32, Stance)>,
    /// Whom their marks put them with (at or beside), so the two face
    /// each other.
    with: Option<SelectionId>,
}

impl Part {
    fn at(&self, t: f32) -> Option<(f32, f32)> {
        let leg = self.legs.iter().rev().find(|leg| leg.t0 <= t)?;
        if t >= leg.t1 || leg.t1 <= leg.t0 {
            return Some(leg.to);
        }
        let raw = (t - leg.t0) / (leg.t1 - leg.t0);
        let u = ease(raw);
        let y = match leg.lane {
            Some(lane) if raw < 0.2 => leg.from.1 + (lane - leg.from.1) * ease(raw / 0.2),
            Some(lane) if raw > 0.8 => lane + (leg.to.1 - lane) * ease((raw - 0.8) / 0.2),
            Some(lane) => lane,
            None => leg.from.1 + (leg.to.1 - leg.from.1) * u,
        };
        Some((leg.from.0 + (leg.to.0 - leg.from.0) * u, y))
    }

    /// Whether they have gone in at their door by `t`.
    fn gone_in(&self, t: f32) -> bool {
        self.legs
            .iter()
            .rev()
            .find(|leg| leg.t0 <= t)
            .is_some_and(|leg| leg.indoors && t >= leg.t1 + leg.pause)
    }

    fn walking(&self, t: f32) -> Option<&Leg> {
        self.legs
            .iter()
            .rev()
            .find(|leg| leg.t0 <= t)
            .filter(|leg| t < leg.t1 && leg.t1 > leg.t0)
    }

    fn stance(&self, t: f32) -> Stance {
        let leg = self.legs.iter().rev().find(|leg| leg.t0 <= t);
        let strike = self.strikes.iter().rev().find(|(at, _)| *at <= t);
        match (leg, strike) {
            (Some(leg), Some((at, stance))) if *at >= leg.t0 => *stance,
            (Some(leg), _) => leg.after,
            (None, Some((_, stance))) => *stance,
            (None, None) => Stance::Standing,
        }
    }
}

/// A prop's part: shown at `from`, taken away at `until`.
#[derive(Clone, Copy, Debug)]
struct PropPart {
    prop: Prop,
    mark: Mark,
    from: f32,
    until: Option<f32>,
    /// Where it stands, for one that stands still.
    at: Option<(f32, f32, f32, f32)>,
}

/// Something on the stage a mark can point at.
#[derive(Clone, Copy, Debug)]
struct Anchor {
    x: f32,
    y: f32,
    /// Half its width, from its middle to where someone stands beside it.
    half: f32,
    /// A building's foot and height, for scaffolding round it.
    building: Option<(f32, f32)>,
    /// A thing's foot and width, for scaffolding round it.
    thing: Option<(f32, f32)>,
}

/// Who or what stands somewhere already, so nobody is put on anyone.
#[derive(Clone, Copy, Debug)]
struct Taken {
    x: f32,
    y: f32,
    /// Half its width.
    half: f32,
    /// One of the people (the cast or anyone out), not a thing or a prop.
    person: bool,
    /// Which of the cast, so a cue can put someone right up against them.
    who: Option<SelectionId>,
    /// How far in front or behind it someone may stand at its side, in
    /// figure heights: a body's own height for someone else (who would
    /// hide them), less for a low thing or prop (someone stands in front
    /// of a planter, never on it).
    depth: f32,
}

/// A scene worked out against a stage: everyone's part, the props' and
/// the camera's, before any moment of it is played.
struct Blocking<'a> {
    stage: &'a Stage,
    snapshot: &'a ProjectionSnapshot,
    script: &'a SceneScript,
    index_of: BTreeMap<SelectionId, usize>,
    ground: Ground,
    /// The left and right edges of the scene's first view.
    edges: (f32, f32),
    parts: Vec<Part>,
    props: Vec<PropPart>,
    /// Who and what stands where already, so nobody is put on anyone.
    taken: Vec<Taken>,
    /// Tall, thin things standing in the water in front of the shore (a
    /// tide board, a lamp on the point): nobody stands right behind one,
    /// sliced by it. Where, and how far either side.
    posts: Vec<(f32, f32)>,
}

fn ls_inside(footprint: (f32, f32, f32, f32), x: f32, y: f32) -> bool {
    super::living_stage::inside(footprint, x, y)
}

/// Whether a prop is how people come and go.
pub fn is_transport(prop: Prop) -> bool {
    matches!(prop, Prop::Ferry | Prop::Shuttle | Prop::Bus | Prop::Sled)
}

/// How wide and tall a prop stands, for a grown-up `fig` tall.
fn prop_size(prop: Prop, fig: f32) -> (f32, f32) {
    match prop {
        Prop::Ferry => (fig * 3.4, fig * 1.6),
        Prop::Shuttle => (fig * 2.2, fig * 2.6),
        Prop::Bus => (fig * 3.0, fig * 1.5),
        Prop::Sled => (fig * 1.6, fig * 0.6),
        Prop::Suitcase => (fig * 0.3, fig * 0.24),
        Prop::Bouquet => (fig * 0.16, fig * 0.22),
        Prop::Bunting => (fig * 4.4, fig * 2.1),
        Prop::Cradle => (fig * 0.55, fig * 0.35),
        Prop::Lamp => (fig * 0.3, fig * 1.8),
        Prop::Wreath => (fig * 0.4, fig * 0.4),
        Prop::Scaffold => (fig * 2.2, fig * 2.4),
        Prop::Ribbon => (fig * 1.4, fig * 0.5),
        Prop::Rain => (0.0, 0.0),
        Prop::Tools => (fig * 0.5, fig * 0.7),
        Prop::Table => (fig * 2.0, fig * 0.42),
        Prop::Bench => (fig * 1.2, fig * 0.8),
    }
}

/// Whether a prop takes up ground nobody else may stand on.
fn takes_ground(prop: Prop) -> bool {
    matches!(
        prop,
        Prop::Cradle | Prop::Lamp | Prop::Wreath | Prop::Tools | Prop::Table | Prop::Bench
    )
}

impl<'a> Blocking<'a> {
    fn new(script: &'a SceneScript, stage: &'a Stage, snapshot: &'a ProjectionSnapshot) -> Self {
        let index_of = snapshot
            .canvas
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| (item.id, index))
            .collect();
        let mut blocking = Self {
            stage,
            snapshot,
            script,
            index_of,
            ground: Ground::of(stage, snapshot),
            edges: (0.0, stage.width),
            parts: Vec::new(),
            props: Vec::new(),
            taken: Vec::new(),
            posts: stage
                .things
                .iter()
                .filter(|spot| spot.y >= stage.front - 1.0)
                .filter(|spot| {
                    matches!(
                        snapshot.canvas.items[spot.index].shape,
                        Some(
                            MarkShape::Signpost
                                | MarkShape::Lamp
                                | MarkShape::Lantern
                                | MarkShape::Flag
                        )
                    )
                })
                .map(|spot| (spot.x, spot.w * 0.3 + stage.figure_h * 0.3))
                .collect(),
        };
        // The first view: whatever the first camera cue looks at, as the
        // stage stands before anyone moves.
        let first = script.cues.iter().find_map(|cue| match &cue.act {
            Act::Camera { on, zoom } => Some((*on, *zoom)),
            _ => None,
        });
        let (on, zoom) = first.unwrap_or((blocking.place_mark(), 1.0));
        let (x, _) = blocking
            .spot(on, 0.0)
            .unwrap_or((stage.width / 2.0, stage.feet));
        let camera = Camera::around(stage, scene_zoom(zoom), x, stage.feet);
        let half = stage.view_w / camera.zoom / 2.0;
        blocking.edges = (camera.x - half, camera.x + half);
        blocking.block();
        blocking
    }

    fn fig(&self) -> f32 {
        self.stage.figure_h
    }

    fn place_mark(&self) -> Mark {
        match self.script.place {
            Some(place) => Mark::Place(place),
            None => self
                .script
                .cast
                .first()
                .map_or(Mark::Offstage(true), |(who, _)| Mark::Place(*who)),
        }
    }

    /// Whatever stands at item `index` this moment of the blocking: one of
    /// the cast where their part has them, someone out where they stand,
    /// a building or a thing; else where the World says they are.
    fn anchor(&self, id: SelectionId, t: f32, depth: u8) -> Option<Anchor> {
        let index = *self.index_of.get(&id)?;
        let fig = self.fig();
        let stage = self.stage;
        let feet = stage.feet;
        if let Some(part) = self.parts.iter().find(|part| part.index == index) {
            if let Some((x, y)) = part.at(t) {
                return Some(Anchor {
                    x,
                    y,
                    half: fig * 0.2,
                    building: None,
                    thing: None,
                });
            }
        }
        if let Some(spot) = stage.people.iter().find(|spot| spot.index == index) {
            return Some(Anchor {
                x: spot.x,
                y: spot.y,
                half: fig * 0.2,
                building: None,
                thing: None,
            });
        }
        if let Some(spot) = stage.buildings.iter().find(|spot| spot.index == index) {
            let setting = art::Setting::from_key(self.snapshot.canvas.setting.as_deref());
            let (w, h) = building_size(stage, self.snapshot, spot, setting);
            return Some(Anchor {
                x: spot.x,
                y: feet.max(spot.y + fig * 0.25),
                half: w * 0.45,
                building: Some((spot.y, h.max(w * 0.6))),
                thing: None,
            });
        }
        if let Some(spot) = stage.things.iter().find(|spot| spot.index == index) {
            let setting = art::Setting::from_key(self.snapshot.canvas.setting.as_deref());
            let w = thing_width(stage, self.snapshot, spot, setting).max(spot.w);
            return Some(Anchor {
                x: spot.x,
                y: if (spot.y - feet).abs() < fig * 0.6 {
                    spot.y
                } else {
                    feet
                },
                half: w / 2.0,
                building: None,
                thing: Some((spot.y, w)),
            });
        }
        if depth > 2 {
            return None;
        }
        // Not out: where the World has them, or their home.
        let item = &self.snapshot.canvas.items[index];
        item.at
            .or(item.home)
            .and_then(|at| self.anchor(at, t, depth + 1))
    }

    /// The stage point a mark is at `t` into the blocking: feet on the
    /// people's ground in front of whatever it names.
    fn find(&self, mark: Mark, t: f32) -> Option<(f32, f32, Option<Anchor>)> {
        let fig = self.fig();
        match mark {
            Mark::Place(id) => self.anchor(id, t, 0).map(|a| (a.x, a.y, Some(a))),
            Mark::Home(id) => {
                let index = *self.index_of.get(&id)?;
                let home = self.snapshot.canvas.items[index].home;
                home.and_then(|home| self.anchor(home, t, 0))
                    .or_else(|| self.anchor(id, t, 0))
                    .map(|a| (a.x, a.y, Some(a)))
            }
            Mark::Beside(id, steps) => {
                let a = self.anchor(id, t, 0)?;
                let side = if steps < 0 { -1.0 } else { 1.0 };
                let x = a.x + side * (a.half + fig * 0.45 + f32::from(steps).abs() * fig * 0.6);
                Some((x, a.y, None))
            }
            Mark::Offstage(right) => {
                let x = if right {
                    self.edges.1 + fig * 1.2
                } else {
                    self.edges.0 - fig * 1.2
                };
                Some((x, self.stage.feet, None))
            }
        }
    }

    /// Where a mark is, or else the scene's place, or else the middle of
    /// its first view.
    fn spot(&self, mark: Mark, t: f32) -> Option<(f32, f32)> {
        self.find(mark, t)
            .or_else(|| self.find(self.place_mark(), t))
            .map(|(x, y, _)| (x, y))
    }

    fn spot_or_middle(&self, mark: Mark, t: f32) -> (f32, f32) {
        self.spot(mark, t)
            .unwrap_or(((self.edges.0 + self.edges.1) / 2.0, self.stage.feet))
    }

    /// Feet on the people's ground: on their line, or a step in front.
    fn on_ground(&self, y: f32) -> f32 {
        let (fig, feet) = (self.fig(), self.stage.feet);
        y.clamp(feet - fig * 0.1, feet + fig * 0.5)
    }

    /// The nearest open ground to `x` on line `y`, `half` wide: in no
    /// building, on no thing, and clear of whoever and whatever stands
    /// there already.
    fn open_near(
        &self,
        x: f32,
        y: f32,
        half: f32,
        person: bool,
        close_to: Option<SelectionId>,
    ) -> (f32, f32) {
        let fig = self.fig();
        let step = fig * 0.2;
        // On their line, or a step or two in front of it (in front of a
        // building's foot, where its picture is).
        let feet = self.stage.feet;
        let lines = [y, feet + fig * 0.3, feet + fig * 0.6];
        let near = |reach: i32, strict: bool| {
            (0..=reach)
                .flat_map(|n| [n, -n])
                .map(|n| x + n as f32 * step)
                .find_map(|x| {
                    lines
                        .iter()
                        .find(|y| self.clear_at(x, **y, half, person, close_to, strict))
                        .map(|y| (x, *y))
                })
        };
        // Clear of everyone, close by; else clear of the cast and of
        // everything, whoever of the town is about.
        near(30, true)
            .or_else(|| near(80, false))
            .or_else(|| near(400, false))
            .unwrap_or((x, y))
    }

    /// Whether something `half` wide stands clear at (`x`, `y`): on open
    /// ground, behind no post in the water, and a body's width from anyone
    /// (but `close_to`, whom a cue puts it right up against), clear of
    /// every thing and prop.
    fn clear_at(
        &self,
        x: f32,
        y: f32,
        half: f32,
        person: bool,
        close_to: Option<SelectionId>,
        strict: bool,
    ) -> bool {
        let fig = self.fig();
        self.ground.open(x, y, fig, usize::MAX)
            && self.ground.open(x - half, y, fig * 0.5, usize::MAX)
            && self.ground.open(x + half, y, fig * 0.5, usize::MAX)
            && self
                .posts
                .iter()
                .all(|(post, reach)| (x - post).abs() >= reach + half * 0.5)
            && self.taken.iter().all(|taken| {
                // Whoever of the town is about: a body's width from them,
                // or at a pinch, only not on them.
                let gap = if person && taken.person {
                    let against = close_to.is_some() && taken.who == close_to;
                    if against || (taken.who.is_none() && !strict) {
                        0.0
                    } else {
                        fig * (APART - 0.42)
                    }
                } else if taken.person && taken.who.is_none() && !strict {
                    0.0
                } else {
                    fig * CLEAR
                };
                (x - taken.x).abs() >= taken.half + half + gap
                    || (y - taken.y).abs() >= fig * taken.depth
            })
    }

    fn take(&mut self, x: f32, y: f32, half: f32, person: Option<SelectionId>) {
        self.taken.push(Taken {
            x,
            y,
            half,
            person: person.is_some(),
            who: person,
            depth: if person.is_some() { 0.7 } else { 0.58 },
        });
    }

    fn part_of(&mut self, who: SelectionId) -> Option<usize> {
        if let Some(at) = self.parts.iter().position(|part| part.who == who) {
            return Some(at);
        }
        let index = *self.index_of.get(&who)?;
        if self.snapshot.canvas.items[index].kind != CanvasItemKind::Actor {
            return None;
        }
        self.parts.push(Part {
            index,
            who,
            legs: Vec::new(),
            strikes: Vec::new(),
            with: None,
        });
        Some(self.parts.len() - 1)
    }

    /// The next cue of someone's after `at`, if they have one.
    fn next_cue(&self, who: SelectionId, at: f32) -> Option<f32> {
        self.script
            .cues
            .iter()
            .filter(|cue| cue.who == Some(who) && cue.at > at + 1e-3)
            .map(|cue| cue.at)
            .reduce(f32::min)
    }

    /// Someone put at a mark at `t` (their part's opening), on open ground.
    /// Whom a mark puts someone with: one of the others the scene stages.
    fn note_with(&mut self, at: usize, mark: Mark) {
        let other = match mark {
            Mark::Place(id) | Mark::Beside(id, _) => id,
            _ => return,
        };
        if other != self.parts[at].who && self.parts.iter().any(|part| part.who == other) {
            self.parts[at].with = Some(other);
        }
    }

    /// Whom a cue puts someone right up against: a mark beside one of the
    /// cast with no steps between (a baby by its parent).
    fn up_against(&self, mark: Mark) -> Option<SelectionId> {
        match mark {
            Mark::Beside(id, 0) if self.parts.iter().any(|part| part.who == id) => Some(id),
            _ => None,
        }
    }

    fn place(&mut self, at: usize, mark: Mark, stance: Stance, t: f32) {
        self.note_with(at, mark);
        let fig = self.fig();
        let who = self.parts[at].who;
        let (x, y) = self.spot_or_middle(mark, t);
        let y = self.on_ground(y);
        let offstage = matches!(mark, Mark::Offstage(_));
        let (x, y) = if offstage {
            (x, y)
        } else {
            let close = self.up_against(mark);
            self.open_near(x, y, fig * 0.21, true, close)
        };
        if !offstage {
            self.take(x, y, fig * 0.21, Some(who));
        }
        let part = &mut self.parts[at];
        part.legs.retain(|leg| leg.t0 > t + 1e-3);
        part.legs.insert(
            0,
            Leg {
                t0: t,
                t1: t,
                from: (x, y),
                to: (x, y),
                after: stance,
                moving: Stance::Walking,
                lane: None,
                indoors: false,
                pause: 0.0,
            },
        );
    }

    /// Whether rain is falling by `t`.
    fn raining(&self, t: f32) -> bool {
        self.rain_from().is_some_and(|from| from <= t)
    }

    fn rain_from(&self) -> Option<f32> {
        self.script
            .cues
            .iter()
            .filter(|cue| {
                matches!(
                    cue.act,
                    Act::Weather(Prop::Rain)
                        | Act::Prop {
                            prop: Prop::Rain,
                            shown: true,
                            ..
                        }
                )
            })
            .map(|cue| cue.at)
            .reduce(f32::min)
    }

    /// Someone walking from where they are at `t` to a mark: briskly, and
    /// hurried (or, if even that is too slow, setting off from nearer) so
    /// they are there by their next cue or the scene's end; hurried in the
    /// rain. A walk of any distance goes along the near lane, in front of
    /// whoever stands about, and stops beside any building in the way.
    /// Someone walking home goes in at their door once there.
    fn walk(&mut self, at: usize, mark: Mark, stance: Stance, t: f32) {
        self.note_with(at, mark);
        let fig = self.fig();
        let who = self.parts[at].who;
        let mut from = match self.parts[at].at(t) {
            Some(from) => from,
            None => {
                let mark = Mark::Place(who);
                self.place(at, mark, Stance::Standing, 0.0);
                self.parts[at]
                    .at(t)
                    .unwrap_or(((self.edges.0 + self.edges.1) / 2.0, self.stage.feet))
            }
        };
        let found = self.find(mark, t);
        let (x, y) = found
            .map(|(x, y, _)| (x, y))
            .unwrap_or_else(|| self.spot_or_middle(mark, t));
        let y = self.on_ground(y);
        let offstage = matches!(mark, Mark::Offstage(_));
        // Home, to a door they can go in at.
        let door = matches!(mark, Mark::Home(_))
            && found.and_then(|(.., anchor)| anchor?.building).is_some();
        let (x, y) = if offstage {
            (x, y)
        } else {
            let close = self.up_against(mark);
            self.open_near(x, y, fig * 0.21, true, close)
        };
        let pace = fig * STROLL * BRISK;
        let pace = if self.raining(t) { pace * 1.6 } else { pace };
        let most = fig * STROLL * BRISK * HURRIED;
        let next = self.next_cue(who, t);
        let room = (next.unwrap_or(self.script.seconds - 0.6) - t - 0.15).max(0.3);
        let near_lane = self.stage.feet + fig * LANE;
        // Whether the way along the near lane is clear (nothing stands out
        // into it), and how far along their own line they get.
        // Someone only standing about before, who could not be there in
        // time (or could not get there at all), sets off from nearer.
        let part = &self.parts[at];
        let placed =
            part.walking(t).is_none() && part.legs.len() == 1 && part.legs[0].t1 <= part.legs[0].t0;
        let (lane_clear, reach) = self.way(from, x, y);
        let blocked = !offstage && !lane_clear && (reach - x).abs() > 0.5;
        let far = (x - from.0).abs() / most > room;
        if placed && next.is_some() && (far || blocked) {
            let toward = if (from.0 - x).abs() > 1.0 {
                (from.0 - x).signum()
            } else {
                1.0
            };
            let reach = (x - from.0).abs().min(most * room);
            let nearer = self.open_near(x + toward * reach, from.1, fig * 0.21, true, None);
            let first = &mut self.parts[at].legs[0];
            first.from = nearer;
            first.to = nearer;
            from = nearer;
            self.take(nearer.0, nearer.1, fig * 0.21, Some(who));
        }
        let (lane_clear, reach) = self.way(from, x, y);
        // Stopped short by a building: the nearest open ground this side
        // of it that they can get to.
        // Off the edge along whichever line is clear of every building on
        // the way. Stopped short by a building: the nearest open ground
        // this side of it that they can get to (and they stay there).
        let feet = self.stage.feet;
        let off = (offstage && !lane_clear)
            .then(|| {
                [from.1, feet, feet + fig * 0.3, feet + fig * 0.6]
                    .into_iter()
                    .find(|line| (self.way(from, x, *line).1 - x).abs() < 0.5)
            })
            .flatten();
        let stopped = !lane_clear && off.is_none() && (reach - x).abs() >= 0.5;
        let (x, y) = if let Some(line) = off {
            (x, line)
        } else if !stopped {
            (x, y)
        } else {
            let back = (from.0 - reach).signum();
            let lines = [y, feet + fig * 0.3, feet + fig * 0.6];
            (0..200)
                .map(|k| reach + back * k as f32 * fig * 0.2)
                .take_while(|x| (x - from.0) * back <= 0.0)
                .find_map(|x| {
                    lines
                        .iter()
                        .find(|y| {
                            self.clear_at(x, **y, fig * 0.21, true, None, false)
                                && (self.way(from, x, **y).1 - x).abs() < 0.5
                        })
                        .map(|y| (x, *y))
                })
                .unwrap_or(from)
        };
        let offstage = offstage && !stopped;
        if !offstage {
            self.take(x, y, fig * 0.21, Some(who));
        }
        let distance = (x - from.0).abs();
        let speed = (distance / room).clamp(pace, fig * STROLL * BRISK * 3.0);
        let length = if distance < 1.0 {
            0.0
        } else {
            distance / speed
        };
        let carrying = stance == Stance::Carrying;
        let lane = (lane_clear && (distance > fig * 1.5 || offstage || door))
            .then_some(near_lane)
            .or_else(|| {
                // Off the near lane, along whichever of their two lines is
                // clear all the way (never across a building's ground as
                // they step from one to the other).
                let clear = |line: f32| (self.way(from, x, line).1 - x).abs() < 0.5;
                [from.1, y].into_iter().find(|line| clear(*line))
            });
        self.parts[at].legs.retain(|leg| leg.t0 <= t);
        self.parts[at].legs.push(Leg {
            t0: t,
            t1: t + length,
            from,
            to: (x, y),
            after: if carrying {
                Stance::Carrying
            } else {
                Stance::Standing
            },
            moving: if carrying {
                Stance::Carrying
            } else {
                Stance::Walking
            },
            lane,
            // In at their door, or gone off the edge of the view.
            indoors: offstage || (door && (x - found.map_or(x, |(fx, ..)| fx)).abs() < fig * 2.0),
            pause: if offstage { 0.0 } else { GOING_IN },
        });
    }

    /// Whether the way from `from` to `x` along the near lane is clear
    /// (nothing stands out into it), and how far along the line `y` they
    /// get, stopping beside any building in the way.
    fn way(&self, from: (f32, f32), x: f32, y: f32) -> (bool, f32) {
        let fig = self.fig();
        let near_lane = self.stage.feet + fig * LANE;
        let footprints = self.ground.footprints();
        let lane = (kept_out(footprints, from.0, x, near_lane, fig * 0.2) - x).abs() < 0.5
            && footprints
                .iter()
                .all(|footprint| !ls_inside(*footprint, from.0, near_lane));
        (lane, kept_out(footprints, from.0, x, y, fig * 0.2))
    }

    fn block(&mut self) {
        let fig = self.fig();
        // Everyone out who is not of the cast, where their day has them,
        // and everything standing on the people's ground.
        // (Those of the cast with no part of their own keep their day,
        // and so are about like anyone else.)
        let cast = self
            .script
            .cues
            .iter()
            .filter_map(|cue| self.index_of.get(&cue.who?).copied())
            .collect::<BTreeSet<_>>();
        let out = self
            .stage
            .people
            .iter()
            .filter(|spot| !cast.contains(&spot.index))
            .map(|spot| (spot.x, spot.y))
            .collect::<Vec<_>>();
        for (x, y) in out {
            self.taken.push(Taken {
                x,
                y,
                half: fig * 0.21,
                person: true,
                who: None,
                depth: 0.7,
            });
        }
        let setting = art::Setting::from_key(self.snapshot.canvas.setting.as_deref());
        let feet = self.stage.feet;
        let front = self.stage.front;
        let height = self.stage.height;
        let things = self
            .stage
            .things
            .iter()
            .filter(|spot| {
                let depth = Depth::at(spot.y, height);
                depth > Depth::Street && depth < Depth::Water
            })
            .filter(|spot| {
                let shape = self.snapshot.canvas.items[spot.index]
                    .shape
                    .unwrap_or(MarkShape::Parcel);
                !super::living_stage::lies_flat(shape, spot.y, front)
            })
            .filter(|spot| (spot.y - feet).abs() < fig * 0.9)
            .map(|spot| {
                let w = thing_width(self.stage, self.snapshot, spot, setting).max(spot.w);
                (spot.x, w / 2.0)
            })
            .collect::<Vec<_>>();
        for (x, half) in things {
            self.take(x, feet, half, None);
        }
        // Every cue in time order, as the script has them.
        let cues = self.script.cues.clone();
        let mut shown: Vec<PropPart> = Vec::new();
        for cue in &cues {
            let t = cue.at.max(0.0);
            match &cue.act {
                Act::WalkTo { to, stance } => {
                    let Some(at) = cue.who.and_then(|who| self.part_of(who)) else {
                        continue;
                    };
                    if t <= 0.05 {
                        self.place(at, *to, *stance, 0.0);
                    } else {
                        self.walk(at, *to, *stance, t);
                    }
                }
                Act::Strike(stance) => {
                    let Some(at) = cue.who.and_then(|who| self.part_of(who)) else {
                        continue;
                    };
                    if self.parts[at].legs.is_empty() {
                        let mark = Mark::Place(self.parts[at].who);
                        self.place(at, mark, Stance::Standing, 0.0);
                    }
                    self.parts[at].strikes.push((t, *stance));
                }
                Act::Prop {
                    prop,
                    at,
                    shown: on,
                } => {
                    if *on {
                        let still = self.prop_at(*prop, *at, t, &shown);
                        if let Some((x, y, w, _)) = still {
                            if takes_ground(*prop) {
                                self.take(x, y, w / 2.0, None);
                            } else if *prop == Prop::Bunting {
                                // Its two poles.
                                for pole in [x - w / 2.0, x + w / 2.0] {
                                    self.take(pole, y, fig * 0.08, None);
                                }
                            }
                        }
                        shown.push(PropPart {
                            prop: *prop,
                            mark: *at,
                            from: t,
                            until: None,
                            at: still,
                        });
                    } else if let Some(part) = shown
                        .iter_mut()
                        .find(|part| part.prop == *prop && part.mark == *at && part.until.is_none())
                    {
                        part.until = Some(t);
                    }
                }
                Act::Say(_) | Act::Camera { .. } | Act::Weather(_) => {}
            }
        }
        // A cast member with nothing to do in the scene is not staged.
        self.parts.retain(|part| !part.legs.is_empty());
        self.props = shown;
    }

    /// Where a prop that stands still stands, shown at `t`: (x, foot, w,
    /// h). None for one that goes with someone, or comes in from an edge.
    /// Scaffolding stands round whatever is going up, at its own foot; the
    /// tools lean against the scaffolding, or lie by the work; bunting is
    /// strung between two poles on open ground; anything else stands on
    /// open ground clear of everyone and everything.
    fn prop_at(
        &self,
        prop: Prop,
        mark: Mark,
        t: f32,
        shown: &[PropPart],
    ) -> Option<(f32, f32, f32, f32)> {
        let fig = self.fig();
        let (w, h) = prop_size(prop, fig);
        if is_transport(prop) {
            return None;
        }
        if let Mark::Beside(id, 0) = mark {
            if self
                .index_of
                .get(&id)
                .is_some_and(|index| self.parts.iter().any(|part| part.index == *index))
            {
                return None;
            }
        }
        let found = self.find(mark, t);
        let (x, y) = found
            .map(|(x, y, _)| (x, y))
            .unwrap_or_else(|| self.spot_or_middle(mark, t));
        let anchor = found.and_then(|(.., anchor)| anchor);
        // What it stands by: the anchor of a mark beside something.
        let by = match mark {
            Mark::Beside(id, _) => self.anchor(id, t, 0),
            _ => anchor,
        };
        match prop {
            Prop::Scaffold => {
                if let Some((a, (base, height))) = anchor.and_then(|a| a.building.map(|b| (a, b))) {
                    return Some((a.x, base, a.half * 2.3, height * 0.95));
                }
                if let Some((a, (base, width))) = anchor.and_then(|a| a.thing.map(|b| (a, b))) {
                    return Some((a.x, base, (width * 1.4).max(fig * 1.6), fig * 2.0));
                }
                let y = self.on_ground(y) - fig * 0.15;
                {
                    let (x, y) = self.open_near(x, y, w / 2.0, false, None);
                    Some((x, y, w, h))
                }
            }
            Prop::Tools => {
                // Against the scaffolding round the same work, if it is up.
                let scaffold = shown
                    .iter()
                    .filter(|part| part.prop == Prop::Scaffold && part.until.is_none())
                    .find_map(|part| part.at);
                if let Some((sx, base, sw, _)) = scaffold {
                    let side = if x < sx { -1.0 } else { 1.0 };
                    return Some((sx + side * (sw / 2.0 + w * 0.35), base, w, h));
                }
                let _ = by;
                let y = self.on_ground(y) - fig * 0.12;
                {
                    let (x, y) = self.open_near(x, y, w / 2.0, false, None);
                    Some((x, y, w, h))
                }
            }
            // Over the people's heads, across the place, its poles on open
            // ground: never on a building's face, a thing or a post.
            Prop::Bunting => {
                let y = self.on_ground(y) - fig * 0.1;
                let pole = |at: f32| self.clear_at(at, y, fig * 0.08, false, None, false);
                for k in [1.0, 0.85, 0.7, 0.55] {
                    let w = w * k;
                    let shift = (0..=60)
                        .flat_map(|n| [n, -n])
                        .map(|n| n as f32 * fig * 0.2)
                        .find(|shift| pole(x + shift - w / 2.0) && pole(x + shift + w / 2.0));
                    if let Some(shift) = shift {
                        return Some((x + shift, y, w, h));
                    }
                }
                None
            }
            // Across the door.
            Prop::Ribbon => {
                let width = anchor.map_or(w, |a| (a.half * 1.2).max(fig));
                Some((x, self.on_ground(y) - fig * 0.15, width, h))
            }
            _ => {
                let y = self.on_ground(y) - fig * 0.12;
                {
                    let (x, y) = self.open_near(x, y, w / 2.0, false, None);
                    Some((x, y, w, h))
                }
            }
        }
    }

    /// The script's camera at `t`: the latest cue's view (following
    /// whoever it is on), eased from the one before over [`CAMERA_EASE`].
    fn camera(&self, t: f32) -> Option<Camera> {
        let cameras = self
            .script
            .cues
            .iter()
            .filter_map(|cue| match &cue.act {
                Act::Camera { on, zoom } => Some((cue.at, *on, *zoom)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let latest = cameras.iter().rposition(|(at, ..)| *at <= t).unwrap_or(0);
        let view = |(_, on, zoom): (f32, Mark, f32)| {
            let (x, y) = self.spot_or_middle(on, t);
            let zoom = scene_zoom(zoom);
            // The people's ground a little below the middle, room above
            // for the roofs and below for the words.
            let ground = self.on_ground(y) - self.stage.height * 0.1 / zoom;
            // On a building further back: up to take in its roof too, its
            // people still above the words.
            let building = self
                .find(on, t)
                .and_then(|(.., anchor)| anchor?.building)
                .map(|(base, height)| {
                    let top = base - height - self.fig() * 0.3;
                    let half = self.stage.height / zoom / 2.0;
                    let lowest = self.stage.feet - half * 0.45;
                    ground.min(top + half).max(lowest)
                });
            Camera::around(self.stage, zoom, x, building.unwrap_or(ground))
        };
        let now = view(*cameras.get(latest)?);
        let (at, ..) = cameras[latest];
        match latest.checked_sub(1).map(|before| cameras[before]) {
            Some(before) if t - at < CAMERA_EASE && before.0 < at => {
                Some(view(before).toward(now, (t - at) / CAMERA_EASE))
            }
            _ => Some(now),
        }
    }

    fn at(&self, t: f32) -> Staged {
        let fig = self.fig();
        let camera = self.camera(t);
        let focus = camera.map_or((self.edges.0 + self.edges.1) / 2.0, |camera| camera.x);
        let held = |index: usize, t: f32| {
            self.props.iter().any(|part| {
                matches!(part.mark, Mark::Beside(id, 0)
                    if self.index_of.get(&id) == Some(&index))
                    && part.from <= t
                    && part.until.is_none_or(|until| t < until + PROP_FADE)
                    && !is_transport(part.prop)
            })
        };
        // A quarrel's two are cross with each other.
        let quarrel = if self.script.kind == MomentKind::Quarrel {
            self.parts
                .iter()
                .take(2)
                .map(|part| part.index)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let indoors = self
            .parts
            .iter()
            .filter(|part| part.gone_in(t))
            .map(|part| part.index)
            .collect::<Vec<_>>();
        let mut people = self
            .parts
            .iter()
            .filter(|part| !part.gone_in(t))
            .filter_map(|part| {
                let (x, y) = part.at(t)?;
                let holding = held(part.index, t);
                let plain = |stance: Stance| {
                    if holding && stance == Stance::Carrying {
                        Stance::Standing
                    } else {
                        stance
                    }
                };
                let (stance, walk, loaded, facing) = match part.walking(t) {
                    Some(leg) => {
                        let stance = plain(leg.moving);
                        (
                            stance,
                            Some((leg.from.0, leg.to.0, t - leg.t0, leg.t1 - leg.t0)),
                            stance == Stance::Carrying,
                            (leg.to.0 - leg.from.0).signum(),
                        )
                    }
                    None => (plain(part.stance(t)), None, false, 0.0),
                };
                let mood = if quarrel.contains(&part.index) {
                    Some(world_projection::Mood::Cross)
                } else if matches!(
                    stance,
                    Stance::Celebrating | Stance::Dancing | Stance::Waving
                ) {
                    Some(world_projection::Mood::Happy)
                } else {
                    None
                };
                Some(StagedPerson {
                    index: part.index,
                    who: part.who,
                    x,
                    y,
                    facing,
                    stance,
                    walk,
                    loaded,
                    mood,
                })
            })
            .collect::<Vec<_>>();
        // Whoever stands faces whom they are with, or else the nearest of
        // the others close by, or else what the camera is on, or else the
        // way they last walked.
        let spots = people
            .iter()
            .map(|person| (person.x, person.y))
            .collect::<Vec<_>>();
        let with = |who: SelectionId| {
            self.parts
                .iter()
                .find(|part| part.who == who)
                .and_then(|part| part.with)
                .or_else(|| {
                    self.parts
                        .iter()
                        .find(|part| part.with == Some(who))
                        .map(|part| part.who)
                })
        };
        let at_of = people
            .iter()
            .map(|person| (person.who, (person.x, person.walk.is_some())))
            .collect::<BTreeMap<_, _>>();
        for (n, person) in people.iter_mut().enumerate() {
            if person.walk.is_some() {
                continue;
            }
            let partner = with(person.who)
                .and_then(|other| at_of.get(&other))
                .filter(|(x, walking)| {
                    !walking && (x - person.x).abs() > 1.0 && (x - person.x).abs() < fig * 4.0
                })
                .map(|(x, _)| *x);
            let near = partner.or_else(|| {
                spots
                    .iter()
                    .enumerate()
                    .filter(|(m, (x, y))| {
                        *m != n
                            && (x - person.x).abs() > 1.0
                            && (x - person.x).abs() < fig * 3.5
                            && (y - person.y).abs() < fig * 0.6
                    })
                    .min_by(|a, b| {
                        (a.1 .0 - person.x)
                            .abs()
                            .total_cmp(&(b.1 .0 - person.x).abs())
                    })
                    .map(|(_, (x, _))| *x)
            });
            let last = self
                .parts
                .iter()
                .find(|part| part.index == person.index)
                .and_then(|part| {
                    part.legs
                        .iter()
                        .rev()
                        .find(|leg| leg.t0 <= t && (leg.to.0 - leg.from.0).abs() > 1.0)
                })
                .map(|leg| (leg.to.0 - leg.from.0).signum());
            person.facing = match near {
                Some(x) => (x - person.x).signum(),
                None if (focus - person.x).abs() > fig * 0.3 => (focus - person.x).signum(),
                None => last.unwrap_or(1.0),
            };
        }
        // The props out at `t`.
        let props = self
            .props
            .iter()
            .filter_map(|part| {
                // What the scene opens with is there from its start.
                let fade_in = if part.from <= 0.05 {
                    1.0
                } else {
                    ((t - part.from) / PROP_FADE).clamp(0.0, 1.0)
                };
                let fade_out = part
                    .until
                    .map_or(1.0, |until| 1.0 - ((t - until) / PROP_FADE).clamp(0.0, 1.0));
                if t < part.from || part.prop == Prop::Rain {
                    return None;
                }
                let (w, h) = prop_size(part.prop, fig);
                if is_transport(part.prop) {
                    let right = match part.mark {
                        Mark::Offstage(right) => right,
                        _ => true,
                    };
                    let edge = if right { self.edges.1 } else { self.edges.0 };
                    let side = if right { 1.0 } else { -1.0 };
                    // In to the edge of the view, its near end in sight.
                    let rest = edge - side * w * 0.25;
                    let came = ease((t - part.from) / TRANSPORT_SLIDE);
                    let went = part
                        .until
                        .map_or(0.0, |until| ease((t - until) / TRANSPORT_SLIDE));
                    let x = rest + side * w * ((1.0 - came) + went);
                    if went >= 1.0 {
                        return None;
                    }
                    let y = match part.prop {
                        // Well out on the water, its cabin below the shore's
                        // edge.
                        Prop::Ferry => {
                            self.stage.front + (self.stage.height - self.stage.front) * 0.42
                        }
                        _ => self.stage.feet + fig * 0.6,
                    };
                    return Some(StagedProp {
                        prop: part.prop,
                        x,
                        y,
                        w,
                        h,
                        shown: 1.0,
                        held_by: None,
                        facing: -side,
                    });
                }
                let shown = fade_in * fade_out;
                if shown <= 0.0 {
                    return None;
                }
                if let Some((x, y, w, h)) = part.at {
                    return Some(StagedProp {
                        prop: part.prop,
                        x,
                        y,
                        w,
                        h,
                        shown,
                        held_by: None,
                        facing: 1.0,
                    });
                }
                // At someone's side, going where they go.
                let Mark::Beside(id, _) = part.mark else {
                    return None;
                };
                let index = *self.index_of.get(&id)?;
                let person = people.iter().find(|person| person.index == index)?;
                let facing = if person.facing == 0.0 {
                    1.0
                } else {
                    person.facing
                };
                Some(StagedProp {
                    prop: part.prop,
                    x: person.x + facing * fig * 0.2,
                    y: person.y,
                    w,
                    h,
                    shown,
                    held_by: Some(index),
                    facing,
                })
            })
            .collect();
        // What is said: the latest line, by whoever said it, if on stage.
        let bubble = self
            .script
            .cues
            .iter()
            .filter(|cue| cue.at <= t)
            .filter_map(|cue| match &cue.act {
                Act::Say(words) => Some((cue.who, words, cue.at)),
                _ => None,
            })
            .next_back()
            .and_then(|(who, words, at)| {
                let who = who?;
                people
                    .iter()
                    .any(|person| person.who == who)
                    .then(|| (who, words.clone(), t - at))
            });
        let rain = self.raining(t);
        // A storm sets in: in a storm's scene, or wherever the rain comes.
        let storm = self.rain_from().map_or(0.0, |from| {
            ease(((t - from) / STORM_IN).clamp(0.0, 1.0)) * if t >= from { 1.0 } else { 0.0 }
        });
        let storm = if self.script.kind == MomentKind::Storm || rain {
            storm
        } else {
            0.0
        };
        Staged {
            people,
            props,
            camera,
            bubble,
            rain,
            storm,
            indoors,
            t,
            seconds: self.script.seconds,
        }
    }
}

/// A scene's storm over the sky: a deck of cloud over the sun and the blue,
/// darker overhead, as far as it has set in. Drawn over the sky's own
/// pictures, under the hills and the clouds that drift.
pub(super) fn paint_overcast(window: &mut dyn Brush, frame: &Frame, (ox, oy, w, h): Rect) {
    let storm = frame.scene_storm();
    if storm <= 0.0 {
        return;
    }
    let horizon = oy + frame.at(0.0, frame.horizon).1;
    let bottom = (horizon + h * 0.06).min(oy + h);
    window.gradient(
        ox,
        oy,
        w,
        (bottom - oy).max(1.0),
        180.0,
        (art::hex(0x4e5866).opacity(0.97 * storm), 0.0),
        (art::hex(0x8c96a0).opacity(0.93 * storm), 1.0),
    );
}

/// A scene's storm over everything: the light cooled and dimmed (never
/// greyed out: the art bible's §4), heavy streaks of rain that read at the
/// window's own zoom, and its lamps glowing through it.
pub(super) fn paint_storm(window: &mut dyn Brush, frame: &Frame, (ox, oy, w, h): Rect) {
    let storm = frame.scene_storm();
    if storm <= 0.0 {
        return;
    }
    window.rect(ox, oy, w, h, 0.0, art::hex(0x1e2a3a).opacity(0.26 * storm));
    let z = frame.camera.zoom;
    let fig = frame.figure_h * z;
    for prop in frame
        .scene_props()
        .iter()
        .filter(|prop| prop.prop == Prop::Lamp)
    {
        let (x, foot) = frame.at(prop.x, prop.y);
        let (x, y) = (ox + x, oy + foot - prop.h * z * 0.86);
        let a = prop.shown * storm;
        window.soft(
            x,
            y,
            fig * 1.1,
            fig * 1.1,
            fig * 0.7,
            art::hex(0xffc36a).opacity(0.5 * a),
        );
        window.soft(
            x,
            y,
            fig * 0.3,
            fig * 0.3,
            fig * 0.2,
            art::hex(0xffe2a0).opacity(0.9 * a),
        );
    }
    // Rain in slanting streaks, thicker than a shower's.
    let t = frame.seconds;
    let k = (h / 848.0).clamp(0.3, 1.3) * z.sqrt();
    let ink = art::hex(0xdfe7ef).opacity(0.5 * storm);
    for index in 0..170_u32 {
        let mut seed = index.wrapping_mul(0x9e37_79b9) ^ 0x2c1b_3c6d;
        seed ^= seed >> 15;
        let x0 = (seed % 1000) as f32 / 1000.0;
        let phase = ((seed / 1000) % 1000) as f32 / 1000.0;
        let y = ((phase + t * 1100.0 / h.max(1.0)) % 1.0) * (h + 60.0) - 30.0;
        let x = (x0 * (w + 160.0) + y * 0.28) % (w + 160.0) - 80.0;
        let len = (16.0 + (index % 5) as f32 * 4.0) * k;
        let mut streak = Shape::new();
        streak
            .move_to(ox + x, oy + y)
            .line_to(ox + x + len * 0.28, oy + y + len);
        window.stroke(&streak, 1.3 * k, ink);
    }
}

/// A scene `t` seconds in, on `stage` (laid out from `snapshot`): who the
/// script stages, where and how; its props; its camera; who is speaking;
/// its weather. `t` is held to the scene's length.
pub fn stage_scene(
    script: &SceneScript,
    stage: &Stage,
    snapshot: &ProjectionSnapshot,
    t: f32,
) -> Staged {
    let t = t.clamp(0.0, script.seconds.max(0.0));
    Blocking::new(script, stage, snapshot).at(t)
}

/// Paints a scene's prop standing at screen (`x`, `foot`), for a grown-up
/// `fig` screen pixels tall, in `setting`, `t` seconds into looking.
pub(super) fn paint_prop(
    brush: &mut dyn Brush,
    prop: &StagedProp,
    (x, foot): (f32, f32),
    k: f32,
    fig: f32,
    setting: art::Setting,
    t: f32,
) {
    let (w, h) = (prop.w * k, prop.h * k);
    let a = prop.shown.clamp(0.0, 1.0);
    let ink = |hex: u32| art::hex(hex).opacity(a);
    let contact = gpui::black().opacity(0.2 * a);
    let palette = |key: &str| Palette::of(key, false);
    match prop.prop {
        Prop::Ferry => {
            // A ferry come in: a long hull, a cabin, a funnel.
            let f = prop.facing;
            let hull = ink(0x2f4a6d);
            let bob = (t * 1.1).sin() * fig * 0.03;
            let y = foot + bob;
            brush.soft(x, y + h * 0.04, w * 0.5, h * 0.06, h * 0.06, contact);
            art::polygon(
                brush,
                &[
                    (x - w * 0.5 * f, y - h * 0.35),
                    (x + w * 0.5 * f, y - h * 0.38),
                    (x + w * 0.42 * f, y),
                    (x - w * 0.44 * f, y),
                ],
                hull,
            );
            brush.rect(x - w * 0.5, y - h * 0.4, w, h * 0.06, 1.0, ink(0xf2ede2));
            brush.rect(
                x - w * 0.22,
                y - h * 0.72,
                w * 0.44,
                h * 0.34,
                2.0,
                ink(0xf2ede2),
            );
            for n in 0..4 {
                brush.rect(
                    x - w * 0.18 + n as f32 * w * 0.1,
                    y - h * 0.64,
                    w * 0.06,
                    h * 0.12,
                    1.0,
                    ink(0x5a7d9a),
                );
            }
            brush.rect(
                x - w * 0.06 * f,
                y - h * 0.98,
                w * 0.08,
                h * 0.28,
                1.0,
                ink(0xc0463a),
            );
        }
        Prop::Shuttle => {
            let body = ink(0xece6da);
            brush.soft(x, foot, w * 0.5, h * 0.05, h * 0.05, contact);
            art::polygon(
                brush,
                &[
                    (x - w * 0.22, foot - h * 0.2),
                    (x + w * 0.22, foot - h * 0.2),
                    (x + w * 0.16, foot - h * 0.85),
                    (x, foot - h),
                    (x - w * 0.16, foot - h * 0.85),
                ],
                body,
            );
            for side in [-1.0, 1.0] {
                art::line(
                    brush,
                    (x + side * w * 0.18, foot - h * 0.25),
                    (x + side * w * 0.4, foot),
                    2.0 * k,
                    ink(0x6b6f78),
                );
            }
            art::circle(brush, x, foot - h * 0.62, w * 0.07, ink(0x3a6f8f));
        }
        Prop::Bus | Prop::Sled => {
            let shape = if prop.prop == Prop::Bus {
                MarkShape::Rover
            } else {
                MarkShape::Parcel
            };
            brush.soft(x, foot, w * 0.5, h * 0.06, h * 0.06, contact);
            if a >= 0.99 {
                art::paint_thing(brush, x, foot, w, shape, &palette("scene-bus"), 0.0);
            }
        }
        Prop::Suitcase => {
            // A bag held low at their side.
            let (bx, by) = (x, foot - fig * 0.3);
            brush.rect(bx - w / 2.0, by - h, w, h, 1.5 * k, ink(0x7a4a2e));
            brush.rect(bx - w / 2.0, by - h * 0.55, w, h * 0.1, 0.0, ink(0x5a3420));
            art::line(
                brush,
                (bx - w * 0.15, by - h),
                (bx - w * 0.1, by - h - fig * 0.06),
                1.0 * k,
                ink(0x3a2418),
            );
            art::line(
                brush,
                (bx + w * 0.15, by - h),
                (bx + w * 0.1, by - h - fig * 0.06),
                1.0 * k,
                ink(0x3a2418),
            );
        }
        Prop::Bouquet => {
            let (bx, by) = (x, foot - fig * 0.5);
            art::line(brush, (bx, by), (bx, by - h * 0.6), 1.2 * k, ink(0x3f6a3a));
            for (n, colour) in [0xc0463a_u32, 0xf4f1ea, 0xe8b33c, 0xd77fa1]
                .into_iter()
                .enumerate()
            {
                let angle = n as f32 * 1.6;
                art::circle(
                    brush,
                    bx + angle.cos() * w * 0.35,
                    by - h * 0.7 + angle.sin() * w * 0.3,
                    w * 0.28,
                    ink(colour),
                );
            }
        }
        Prop::Bunting => {
            // A string of little flags between two poles over the place.
            let colours = [0xc0463a_u32, 0xf4f1ea, 0xe8b33c, 0x2f4a6d];
            let (left, right) = (x - w / 2.0, x + w / 2.0);
            let top = foot - h;
            for pole in [left, right] {
                brush.soft(pole, foot, 3.0 * k, 1.2 * k, 1.5 * k, contact);
                brush.rect(pole - 1.2 * k, top, 2.4 * k, h, 0.8, ink(0x7a5a3e));
            }
            let sag = h * 0.14;
            let mut string = Shape::new();
            string
                .move_to(left, top + 2.0 * k)
                .curve_to(right, top + 2.0 * k, x, top + sag * 2.0);
            brush.stroke(&string, 0.8 * k, ink(0x6b4a32));
            let flags = 11;
            for flag in 0..flags {
                let u = (flag as f32 + 0.5) / flags as f32;
                let fx = left + w * u;
                let fy = top + 2.0 * k + sag * 4.0 * u * (1.0 - u);
                let s = fig * 0.09;
                let flutter = (t * 3.0 + flag as f32).sin() * s * 0.15;
                art::polygon(
                    brush,
                    &[
                        (fx - s * 0.6, fy),
                        (fx + s * 0.6, fy),
                        (fx + flutter, fy + s * 1.3),
                    ],
                    ink(colours[flag % colours.len()]),
                );
            }
        }
        Prop::Cradle => {
            brush.soft(x, foot, w * 0.55, h * 0.12, h * 0.1, contact);
            let wood = ink(0x9a6a40);
            art::ellipse(brush, x, foot - h * 0.45, w * 0.5, h * 0.42, wood);
            brush.rect(
                x - w * 0.45,
                foot - h * 0.85,
                w * 0.9,
                h * 0.3,
                2.0 * k,
                ink(0xf3efe4),
            );
            art::circle(brush, x - w * 0.22, foot - h * 0.8, h * 0.16, ink(0xe8c9a8));
            for side in [-1.0, 1.0] {
                art::line(
                    brush,
                    (x + side * w * 0.3, foot - h * 0.1),
                    (x + side * w * 0.42, foot),
                    1.4 * k,
                    wood,
                );
            }
        }
        Prop::Lamp => {
            let night = palette("scene-lamp");
            brush.soft(
                x,
                foot - h * 0.85,
                fig * 0.8,
                fig * 0.8,
                fig * 0.5,
                art::hex(0xffc36a).opacity(0.28 * a),
            );
            if a >= 0.99 {
                art::paint_thing(brush, x, foot, w, MarkShape::Lantern, &night, 0.0);
            }
            art::circle(brush, x, foot - h * 0.86, w * 0.18, ink(0xffd98a));
        }
        Prop::Wreath => {
            let (cx, cy) = (x, foot - h * 0.55);
            brush.stroke(
                &Shape::ellipse(cx, cy, w * 0.4, h * 0.4),
                w * 0.16,
                ink(0x3f6a3a),
            );
            for petal in 0..6 {
                let angle = petal as f32 * std::f32::consts::TAU / 6.0;
                art::circle(
                    brush,
                    cx + angle.cos() * w * 0.4,
                    cy + angle.sin() * h * 0.4,
                    w * 0.09,
                    ink(if petal % 2 == 0 { 0xf4f1ea } else { 0xc0463a }),
                );
            }
        }
        Prop::Scaffold => {
            // Poles, two boards and a ladder round what is going up.
            let wood = ink(0xa87a4a);
            let (left, right) = (x - w / 2.0, x + w / 2.0);
            for px in [left, x, right] {
                art::line(brush, (px, foot), (px, foot - h), 1.6 * k, wood);
            }
            for level in [0.35_f32, 0.7] {
                let y = foot - h * level;
                brush.rect(
                    left - 2.0 * k,
                    y - 1.5 * k,
                    right - left + 4.0 * k,
                    3.0 * k,
                    0.5,
                    ink(0x8a6446),
                );
                art::line(
                    brush,
                    (left, y),
                    (x, y - h * 0.3),
                    0.8 * k,
                    wood.opacity(0.7 * a),
                );
            }
            let ladder = right + w * 0.05;
            for side in [0.0, w * 0.05] {
                art::line(
                    brush,
                    (ladder + side, foot),
                    (ladder + side - w * 0.06, foot - h * 0.6),
                    1.1 * k,
                    wood,
                );
            }
            for rung in 1..6 {
                let u = rung as f32 / 6.0;
                let y = foot - h * 0.6 * u;
                let lx = ladder - w * 0.06 * u;
                art::line(brush, (lx, y), (lx + w * 0.05, y), 0.8 * k, wood);
            }
        }
        Prop::Ribbon => {
            let y = foot - fig * 0.55;
            let (left, right) = (x - w / 2.0, x + w / 2.0);
            for post in [left, right] {
                brush.rect(post - 1.0 * k, y, 2.0 * k, foot - y, 0.5, ink(0x7a5a3e));
            }
            art::line(brush, (left, y), (right, y), 2.0 * k, ink(0xc0463a));
            for side in [-1.0, 1.0] {
                art::polygon(
                    brush,
                    &[
                        (x, y),
                        (x + side * 5.0 * k, y - 4.0 * k),
                        (x + side * 5.0 * k, y + 4.0 * k),
                    ],
                    ink(0xa8382e),
                );
            }
        }
        Prop::Tools => {
            // A spade, a saw and a hammer leaning together.
            brush.soft(x, foot, w * 0.5, h * 0.06, h * 0.06, contact);
            art::line(
                brush,
                (x - w * 0.3, foot),
                (x - w * 0.1, foot - h),
                1.6 * k,
                ink(0x8a6446),
            );
            brush.rect(
                x - w * 0.42,
                foot - h * 0.2,
                w * 0.24,
                h * 0.2,
                1.0,
                ink(0x8c949c),
            );
            brush.rect(x, foot - h * 0.55, w * 0.38, h * 0.14, 0.5, ink(0xb7bec4));
            art::line(
                brush,
                (x + w * 0.3, foot),
                (x + w * 0.32, foot - h * 0.6),
                1.6 * k,
                ink(0x6b4a32),
            );
            brush.rect(
                x + w * 0.22,
                foot - h * 0.68,
                w * 0.22,
                h * 0.1,
                0.5,
                ink(0x5a5f66),
            );
        }
        Prop::Table => {
            // A long table laid for a party.
            brush.soft(x, foot, w * 0.55, h * 0.1, h * 0.1, contact);
            let top = foot - h;
            for leg in [x - w * 0.42, x + w * 0.42] {
                brush.rect(leg - 1.2 * k, top, 2.4 * k, h, 0.5, ink(0x5f3d22));
            }
            brush.rect(x - w / 2.0, top - h * 0.1, w, h * 0.32, 1.0, ink(0xf3efe4));
            let mut n = 0;
            let mut cx = x - w / 2.0 + w * 0.12;
            while cx < x + w / 2.0 - w * 0.08 {
                let colour = [0xc0463a_u32, 0xe8b33c, 0x7a8fb0][n % 3];
                art::circle(brush, cx, top - h * 0.18, h * 0.12, ink(colour));
                cx += w * 0.16;
                n += 1;
            }
        }
        Prop::Bench => {
            brush.soft(x, foot, w * 0.5, h * 0.08, h * 0.08, contact);
            if a >= 0.99 {
                art::paint_thing(
                    brush,
                    x,
                    foot,
                    w,
                    MarkShape::Bench,
                    &palette("scene-bench"),
                    0.0,
                );
            }
        }
        Prop::Rain => {}
    }
    let _ = setting;
}

/// A scene of `kind` for a place's own people, blocked as the World's
/// templates block it (`chronicle::scene`), so the benchmark and the
/// pictures can play one without a World: the place is the building
/// nearest the middle of the first view, and the cast whoever stands
/// nearest it.
#[doc(hidden)]
pub fn sample(
    snapshot: &ProjectionSnapshot,
    stage: &Stage,
    kind: MomentKind,
) -> Option<SceneScript> {
    use world_projection::Cue;
    let items = &snapshot.canvas.items;
    let middle = stage.view_w / 2.0;
    // How much open ground there is about `x` for a crowd: the spots,
    // a stride apart, on no building's picture and clear of every thing
    // and everyone out.
    let ground = Ground::of(stage, snapshot);
    let fig = stage.figure_h;
    let setting = art::Setting::from_key(snapshot.canvas.setting.as_deref());
    let standing = stage
        .things
        .iter()
        .filter(|spot| (spot.y - stage.feet).abs() < fig * 0.9)
        .map(|spot| {
            (
                spot.x,
                thing_width(stage, snapshot, spot, setting).max(spot.w) / 2.0,
            )
        })
        .collect::<Vec<_>>();
    let room_at = |x: f32| {
        (-6..=6)
            .map(|k| x + k as f32 * fig * 0.5)
            .filter(|x| {
                ground.open(*x, stage.feet + fig * 0.5, fig, usize::MAX)
                    && standing
                        .iter()
                        .all(|(at, half)| (x - at).abs() > half + fig * 0.3)
                    && stage
                        .people
                        .iter()
                        .all(|spot| (x - spot.x).abs() > fig * 0.5)
            })
            .count()
    };
    // The building with the most room in front of it, of those in the
    // first view; the nearest the middle of it of those.
    let place = stage
        .buildings
        .iter()
        .filter(|spot| stage.shows(spot.index, 1.0))
        .filter(|spot| (spot.x - middle).abs() < stage.view_w * 0.35)
        .max_by(|a, b| {
            (room_at(a.x), -(a.x - middle).abs())
                .partial_cmp(&(room_at(b.x), -(b.x - middle).abs()))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .or_else(|| {
            stage
                .buildings
                .iter()
                .filter(|spot| stage.shows(spot.index, 1.0))
                .min_by(|a, b| (a.x - middle).abs().total_cmp(&(b.x - middle).abs()))
        })?;
    let mut people = stage
        .people
        .iter()
        .filter(|spot| items[spot.index].look.is_some())
        .collect::<Vec<_>>();
    people.sort_by(|a, b| (a.x - place.x).abs().total_cmp(&(b.x - place.x).abs()));
    let cast = people
        .iter()
        .take(4)
        .map(|spot| items[spot.index].id)
        .collect::<Vec<_>>();
    let a = *cast.first()?;
    let b = cast.get(1).copied();
    let rest = cast.iter().skip(2).copied().collect::<Vec<_>>();
    let place_id = items[place.index].id;
    let here = Mark::Place(place_id);
    let at = Mark::Place;
    let home = Mark::Home;
    let beside = |of: Mark, steps: i8| match of {
        Mark::Place(id) | Mark::Home(id) => Mark::Beside(id, steps),
        other => other,
    };
    let spread = |n: usize, from: i8| {
        let step = from + (n / 2) as i8;
        if n.is_multiple_of(2) {
            step
        } else {
            -step
        }
    };
    let said = "Well, look at that.".to_string();
    let mut cues: Vec<Cue> = Vec::new();
    let mut cue = |t: f32, who: Option<SelectionId>, act: Act| {
        cues.push(Cue { at: t, who, act });
    };
    let walk = |to: Mark, stance: Stance| Act::WalkTo { to, stance };
    let prop = |prop: Prop, at: Mark, shown: bool| Act::Prop { prop, at, shown };
    let camera = |on: Mark, zoom: f32| Act::Camera { on, zoom };
    // How people come and go, in the order the settings are declared: by
    // sea, by lander, by bus, over the ice.
    let transport = [Prop::Ferry, Prop::Shuttle, Prop::Bus, Prop::Sled]
        [art::Setting::from_key(snapshot.canvas.setting.as_deref()) as usize];
    let seconds = match kind {
        MomentKind::Arrival => {
            cue(0.0, None, camera(here, 1.0));
            cue(0.0, None, prop(transport, Mark::Offstage(false), true));
            cue(6.5, None, prop(transport, Mark::Offstage(false), false));
            for (n, who) in cast.iter().skip(1).enumerate() {
                cue(
                    0.0,
                    Some(*who),
                    walk(beside(here, spread(n, 2)), Stance::Standing),
                );
            }
            cue(0.0, Some(a), walk(Mark::Offstage(false), Stance::Carrying));
            cue(0.0, None, prop(Prop::Suitcase, Mark::Beside(a, 0), true));
            cue(0.5, Some(a), walk(here, Stance::Walking));
            cue(1.0, None, camera(at(a), 1.6));
            cue(4.0, Some(a), Act::Strike(Stance::Waving));
            for who in cast.iter().skip(1) {
                cue(4.2, Some(*who), Act::Strike(Stance::Waving));
            }
            cue(4.5, Some(a), Act::Say(said));
            8.0
        }
        MomentKind::Quarrel => {
            cue(0.0, None, camera(at(a), 1.8));
            cue(0.0, Some(a), walk(here, Stance::Talking));
            if let Some(b) = b {
                cue(0.0, Some(b), walk(Mark::Beside(a, 1), Stance::Talking));
                cue(3.5, Some(b), walk(home(b), Stance::Walking));
            }
            cue(5.0, Some(a), Act::Strike(Stance::LookingAround));
            for (n, who) in rest.iter().enumerate() {
                cue(
                    0.0,
                    Some(*who),
                    walk(beside(here, spread(n, 3)), Stance::Standing),
                );
            }
            cue(1.0, Some(a), Act::Say(said));
            7.0
        }
        MomentKind::Festival => {
            cue(0.0, None, prop(Prop::Bunting, here, true));
            cue(0.0, None, prop(Prop::Table, beside(here, 2), true));
            cue(0.0, None, camera(here, 0.9));
            for (n, who) in cast.iter().enumerate() {
                cue(0.0, Some(*who), walk(home(*who), Stance::Standing));
                cue(
                    0.3 + 0.4 * n as f32,
                    Some(*who),
                    walk(beside(here, spread(n, 1)), Stance::Walking),
                );
                cue(4.0, Some(*who), Act::Strike(Stance::Dancing));
            }
            cue(4.5, Some(a), Act::Say(said));
            cue(1.5, None, camera(here, 1.3));
            9.0
        }
        MomentKind::Build => {
            // What goes up is a thing on the people's ground, as a work
            // built by hand is: the nearest of them to the place.
            // The one with the most room about it, for those who build it.
            let made = stage
                .things
                .iter()
                .filter(|spot| (spot.y - stage.feet).abs() < fig * 0.6)
                .filter(|spot| stage.shows(spot.index, 1.0))
                .filter(|spot| (spot.x - middle).abs() < stage.view_w * 0.45)
                .max_by(|a, b| {
                    (room_at(a.x), -(a.x - middle).abs())
                        .partial_cmp(&(room_at(b.x), -(b.x - middle).abs()))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map_or(here, |spot| Mark::Place(items[spot.index].id));
            let here = made;
            cue(0.0, None, camera(here, 1.5));
            cue(0.0, None, prop(Prop::Scaffold, here, true));
            cue(0.0, None, prop(Prop::Tools, beside(here, -1), true));
            for (n, who) in cast.iter().enumerate() {
                cue(
                    0.0,
                    Some(*who),
                    walk(beside(here, spread(n, 1)), Stance::Standing),
                );
                cue(0.5, Some(*who), Act::Strike(Stance::Working));
            }
            cue(5.5, None, prop(Prop::Scaffold, here, false));
            cue(5.5, None, prop(Prop::Tools, beside(here, -1), false));
            cue(6.0, None, camera(here, 1.0));
            for who in &cast {
                cue(6.0, Some(*who), Act::Strike(Stance::Celebrating));
            }
            cue(3.0, Some(a), Act::Say(said));
            8.0
        }
        MomentKind::Birth => {
            let parent = b.unwrap_or(a);
            cue(0.0, Some(parent), walk(here, Stance::Carrying));
            cue(1.5, None, camera(at(parent), 1.8));
            cue(0.0, None, camera(here, 1.2));
            cue(0.0, None, prop(Prop::Cradle, beside(here, 1), true));
            for (n, who) in rest.iter().enumerate() {
                cue(
                    0.0,
                    Some(*who),
                    walk(beside(here, spread(n, 5)), Stance::Standing),
                );
                cue(
                    1.0,
                    Some(*who),
                    walk(beside(here, spread(n, 2)), Stance::Walking),
                );
                cue(3.0, Some(*who), Act::Strike(Stance::Waving));
                cue(5.5, Some(*who), Act::Strike(Stance::Celebrating));
            }
            cue(2.5, Some(parent), Act::Say(said));
            8.0
        }
        MomentKind::Farewell => {
            cue(0.0, None, camera(here, 1.2));
            cue(0.0, None, prop(transport, Mark::Offstage(true), true));
            cue(7.0, None, prop(transport, Mark::Offstage(true), false));
            for (n, who) in cast.iter().skip(1).enumerate() {
                cue(
                    0.0,
                    Some(*who),
                    walk(beside(here, spread(n, 1)), Stance::Standing),
                );
                cue(1.5, Some(*who), Act::Strike(Stance::Waving));
            }
            cue(0.0, Some(a), walk(here, Stance::Carrying));
            cue(0.0, None, prop(Prop::Suitcase, Mark::Beside(a, 0), true));
            cue(3.5, Some(a), walk(Mark::Offstage(true), Stance::Walking));
            cue(3.5, None, camera(at(a), 1.2));
            cue(2.0, Some(a), Act::Say(said));
            8.0
        }
        MomentKind::Storm => {
            cue(0.0, None, Act::Weather(Prop::Rain));
            cue(0.0, None, camera(here, 0.8));
            for (n, who) in cast.iter().enumerate() {
                cue(
                    0.0,
                    Some(*who),
                    walk(beside(here, spread(n, 1)), Stance::Standing),
                );
            }
            for who in &cast {
                cue(1.0, Some(*who), walk(home(*who), Stance::Walking));
            }
            cue(2.0, Some(a), Act::Say(said));
            cue(3.0, None, prop(Prop::Lamp, home(a), true));
            8.0
        }
        _ => return None,
    };
    cues.sort_by(|x, y| x.at.total_cmp(&y.at));
    Some(SceneScript {
        id: format!("sample-{}", kind.id()),
        event: Default::default(),
        kind,
        place: Some(place_id),
        seconds,
        cast: cast.iter().map(|who| (*who, None)).collect(),
        cues,
        caption: String::new(),
    })
}
