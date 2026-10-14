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
        }
    }
}

/// What a scene adds to a frame besides its people: its props, and rain.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dress {
    pub props: Vec<StagedProp>,
    pub rain: bool,
}

impl Frame {
    /// The frame dressed for a scene: its props stand in depth with
    /// everything else, drawn live, and its rain falls.
    pub fn dressed(mut self, dress: Dress) -> Self {
        self.scene = Some(Arc::new(dress));
        self
    }

    /// The props a scene shows in it, if one is playing.
    pub(crate) fn scene_props(&self) -> &[StagedProp] {
        self.scene.as_deref().map_or(&[], |dress| &dress.props)
    }

    /// The weather drawn over it: the World's, or a scene's rain.
    pub(crate) fn shown_weather(&self) -> Weather {
        match self.weather {
            Weather::Clear | Weather::Cloudy
                if self.scene.as_ref().is_some_and(|dress| dress.rain) =>
            {
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
        let u = ease((t - leg.t0) / (leg.t1 - leg.t0));
        Some((
            leg.from.0 + (leg.to.0 - leg.from.0) * u,
            leg.from.1 + (leg.to.1 - leg.from.1) * u,
        ))
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
    taken: Vec<(f32, f32, f32)>,
    /// Tall, thin things standing in the water in front of the shore (a
    /// tide board, a lamp on the point): nobody stands right behind one,
    /// sliced by it. Where, and how far either side.
    posts: Vec<(f32, f32)>,
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
                });
            }
        }
        if let Some(spot) = stage.people.iter().find(|spot| spot.index == index) {
            return Some(Anchor {
                x: spot.x,
                y: spot.y,
                half: fig * 0.2,
                building: None,
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
            });
        }
        if let Some(spot) = stage.things.iter().find(|spot| spot.index == index) {
            return Some(Anchor {
                x: spot.x,
                y: if (spot.y - feet).abs() < fig * 0.6 {
                    spot.y
                } else {
                    feet
                },
                half: spot.w / 2.0,
                building: None,
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
                let x = a.x + side * (a.half + fig * 0.3 + f32::from(steps).abs() * fig * 0.6);
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
    fn open_near(&self, x: f32, y: f32, half: f32) -> f32 {
        let fig = self.fig();
        let clear = |x: f32| {
            self.ground.open(x, y, fig, usize::MAX)
                && self.ground.open(x - half, y, fig * 0.5, usize::MAX)
                && self.ground.open(x + half, y, fig * 0.5, usize::MAX)
                && self
                    .posts
                    .iter()
                    .all(|(post, reach)| (x - post).abs() >= reach + half * 0.5)
                && self.taken.iter().all(|(tx, ty, reach)| {
                    (x - tx).abs() >= reach + half || (y - ty).abs() >= fig * 0.45
                })
        };
        let step = fig * 0.2;
        (0..=80)
            .flat_map(|n| [n, -n])
            .map(|n| x + n as f32 * step)
            .find(|x| clear(*x))
            .unwrap_or(x)
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

    fn place(&mut self, at: usize, mark: Mark, stance: Stance, t: f32) {
        self.note_with(at, mark);
        let fig = self.fig();
        let (x, y) = self.spot_or_middle(mark, t);
        let y = self.on_ground(y);
        let x = if matches!(mark, Mark::Offstage(_)) {
            x
        } else {
            self.open_near(x, y, fig * 0.2)
        };
        self.taken.push((x, y, fig * 0.25));
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
            },
        );
    }

    /// Someone walking from where they are at `t` to a mark: briskly, and
    /// hurried (or, if even that is too slow, setting off from nearer) so
    /// they are there by their next cue; along their line, stopping beside
    /// any building in the way.
    fn walk(&mut self, at: usize, mark: Mark, stance: Stance, t: f32) {
        self.note_with(at, mark);
        let fig = self.fig();
        let who = self.parts[at].who;
        let from = match self.parts[at].at(t) {
            Some(from) => from,
            None => {
                let mark = Mark::Place(who);
                self.place(at, mark, Stance::Standing, 0.0);
                self.parts[at]
                    .at(t)
                    .unwrap_or(((self.edges.0 + self.edges.1) / 2.0, self.stage.feet))
            }
        };
        let (x, y) = self.spot_or_middle(mark, t);
        let y = self.on_ground(y);
        let offstage = matches!(mark, Mark::Offstage(_));
        let x = if offstage {
            x
        } else {
            self.open_near(x, y, fig * 0.2)
        };
        let x = if offstage {
            x
        } else {
            kept_out(self.ground.footprints(), from.0, x, y, fig * 0.2)
        };
        if !offstage {
            self.taken.push((x, y, fig * 0.25));
        }
        let pace = fig * STROLL * BRISK;
        let mut from = from;
        let mut distance = (x - from.0).abs();
        let mut speed = pace;
        if let Some(next) = self.next_cue(who, t) {
            let room = (next - t - 0.15).max(0.3);
            speed = (distance / room).clamp(pace, pace * HURRIED);
            // Still too far: they set off from nearer, if they were only
            // standing about before.
            let part = &self.parts[at];
            let placed = part.walking(t).is_none()
                && part.legs.len() == 1
                && part.legs[0].t1 <= part.legs[0].t0;
            if distance / speed > room && placed {
                let toward = (from.0 - x).signum();
                let nearer = self.open_near(x + toward * speed * room, from.1, fig * 0.2);
                let first = &mut self.parts[at].legs[0];
                first.from.0 = nearer;
                first.to.0 = nearer;
                from.0 = nearer;
                distance = (x - from.0).abs();
            }
        }
        let length = if distance < 1.0 {
            0.0
        } else {
            distance / speed
        };
        let carrying = stance == Stance::Carrying;
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
        });
    }

    fn block(&mut self) {
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
                        let still = self.prop_at(*prop, *at, t);
                        if let (Some((x, y, w, _)), true) = (still, takes_ground(*prop)) {
                            self.taken.push((x, y, w / 2.0));
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
    fn prop_at(&self, prop: Prop, mark: Mark, t: f32) -> Option<(f32, f32, f32, f32)> {
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
        match prop {
            // Round a building, from its foot; or a plot's worth of it.
            Prop::Scaffold => match anchor.and_then(|a| a.building.map(|b| (a, b))) {
                Some((a, (base, height))) => Some((a.x, base, a.half * 2.3, height * 0.95)),
                None => {
                    let y = self.on_ground(y) - fig * 0.15;
                    Some((self.open_near(x, y, w / 2.0), y, w, h))
                }
            },
            // Over the people's heads, across the place.
            Prop::Bunting => Some((x, self.on_ground(y) - fig * 0.1, w, h)),
            // Across the door.
            Prop::Ribbon => {
                let width = anchor.map_or(w, |a| (a.half * 1.2).max(fig));
                Some((x, self.on_ground(y) - fig * 0.15, width, h))
            }
            _ => {
                let y = self.on_ground(y) - fig * 0.12;
                Some((self.open_near(x, y, w / 2.0), y, w, h))
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
        let mut people = self
            .parts
            .iter()
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
                Some(StagedPerson {
                    index: part.index,
                    who: part.who,
                    x,
                    y,
                    facing,
                    stance,
                    walk,
                    loaded,
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
                        Prop::Ferry => {
                            self.stage.front + (self.stage.height - self.stage.front) * 0.3
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
        let rain = self.script.cues.iter().any(|cue| {
            cue.at <= t
                && matches!(
                    cue.act,
                    Act::Weather(Prop::Rain)
                        | Act::Prop {
                            prop: Prop::Rain,
                            shown: true,
                            ..
                        }
                )
        });
        Staged {
            people,
            props,
            camera,
            bubble,
            rain,
            t,
            seconds: self.script.seconds,
        }
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
    let place = stage
        .buildings
        .iter()
        .filter(|spot| stage.shows(spot.index, 1.0))
        .min_by(|a, b| (a.x - middle).abs().total_cmp(&(b.x - middle).abs()))?;
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
            cue(5.0, None, camera(here, 1.3));
            9.0
        }
        MomentKind::Build => {
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
