//! Moments as panels: a wedding, a birth, a storm weathered, a farewell,
//! drawn as three small pictures in the scene's own look (its sky and
//! hills and light, on the same paper), the place behind and the people in
//! front, posed for the beat: before, the moment itself, and after.
//!
//! A panel is painted by [`paint_panel`] from a [`PanelScene`], a pure
//! function of it and the size: the same moment always paints the same
//! pixels, so the golden pictures hold and a replayed World's panels are
//! the same panels. The window paints them off its own thread
//! ([`crate::painter::want`]) and shows them as they arrive.
//!
//! Presentation only: what a panel shows is chosen by the World, which
//! names the place, the cast and the beat; nothing is read back.

use crate::age::{self, Age};
use crate::art::{self, Figure, Inks, Palette, Pose};
use crate::brush::{Brush, Shape};
use crate::diorama;
use crate::painter::{self, Canvas};
use crate::scene::Daylight;
use gpui::Hsla;
use tiny_skia as sk;
use world_projection::{
    Drawing, MarkShape, Mood, ProjectionSnapshot, Scenery, Season, SelectionId, Stance, Weather,
};

/// Which beat of a moment a panel shows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum Beat {
    Before,
    #[default]
    Moment,
    After,
}

/// What kind of moment it is, as far as how it is drawn goes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum Occasion {
    Wedding,
    Birth,
    ComingOfAge,
    Farewell,
    Death,
    Storm,
    WorkOpened,
    Festival,
    #[default]
    Other,
}

impl Occasion {
    /// The hour a moment of this kind is drawn at, beat by beat: a
    /// wedding in the golden afternoon, a birth in the morning, a farewell
    /// as the light goes, a festival into the evening.
    pub fn hour(self, beat: Beat) -> f32 {
        let (before, moment, after) = match self {
            Occasion::Wedding => (13.5, 15.5, 18.2),
            Occasion::Birth => (7.2, 9.0, 13.2),
            Occasion::ComingOfAge => (10.0, 12.5, 17.2),
            Occasion::Farewell => (16.5, 18.6, 19.4),
            Occasion::Death => (15.0, 18.4, 19.6),
            Occasion::Storm => (12.0, 14.0, 17.5),
            Occasion::WorkOpened => (9.5, 11.0, 16.8),
            Occasion::Festival => (17.0, 19.4, 21.2),
            Occasion::Other => (10.5, 13.0, 17.6),
        };
        match beat {
            Beat::Before => before,
            Beat::Moment => moment,
            Beat::After => after,
        }
    }

    /// The weather over it: a storm is a storm only at its height.
    pub fn weather(self, beat: Beat, usual: Weather) -> Weather {
        match (self, beat) {
            (Occasion::Storm, Beat::Before) => Weather::Cloudy,
            (Occasion::Storm, Beat::Moment) => Weather::Storm,
            (Occasion::Storm, Beat::After) => Weather::Clear,
            (_, _) if matches!(usual, Weather::Storm | Weather::Rain) => Weather::Cloudy,
            _ => usual,
        }
    }

    /// How the people in it stand at a beat, and how they feel, when the
    /// World does not say.
    fn pose(self, beat: Beat) -> (Stance, Mood) {
        use Occasion::*;
        match (self, beat) {
            (Death, Beat::Before) => (Stance::Talking, Mood::Content),
            (Death, Beat::Moment) => (Stance::Standing, Mood::Sad),
            (Death, Beat::After) => (Stance::Sitting, Mood::Thinking),
            (Farewell, Beat::Before) => (Stance::Talking, Mood::Content),
            (Farewell, Beat::Moment) => (Stance::Waving, Mood::Sad),
            (Farewell, Beat::After) => (Stance::LookingAround, Mood::Thinking),
            (Storm, Beat::Before) => (Stance::LookingAround, Mood::Thinking),
            (Storm, Beat::Moment) => (Stance::Working, Mood::Cross),
            (Storm, Beat::After) => (Stance::Stretching, Mood::Happy),
            (Wedding | Festival | WorkOpened | ComingOfAge, Beat::Moment) => {
                (Stance::Celebrating, Mood::Happy)
            }
            (Birth, Beat::Moment) => (Stance::Standing, Mood::Happy),
            (_, Beat::Before) => (Stance::Walking, Mood::Content),
            (_, Beat::After) => (Stance::Sitting, Mood::Happy),
            (_, Beat::Moment) => (Stance::Talking, Mood::Happy),
        }
    }
}

/// Someone in a panel: how they look, and how they stand in it.
#[derive(Clone, Debug)]
pub struct Cast {
    pub figure: Figure,
    pub drawing: Option<Drawing>,
    pub stance: Stance,
    pub mood: Mood,
    /// Which way they face: -1 left, 1 right.
    pub facing: f32,
    /// A baby they hold.
    pub carrying: Option<Figure>,
}

/// The place a panel is set at, drawn behind the people.
#[derive(Clone, Debug)]
pub struct Setting {
    pub drawing: Option<Drawing>,
    pub shape: MarkShape,
    pub palette: Palette,
}

/// Everything one panel shows.
#[derive(Clone, Debug)]
pub struct PanelScene {
    pub scenery: Scenery,
    pub season: Option<Season>,
    pub occasion: Occasion,
    pub beat: Beat,
    pub hour: f32,
    pub weather: Weather,
    pub water: bool,
    pub place: Option<Setting>,
    pub cast: Vec<Cast>,
    /// A baby of the moment's in a pram beside them.
    pub pram: Option<Figure>,
    /// A seed for what is scattered: petals, rain.
    pub seed: u32,
}

/// The colours of a World that does not say what it looks like.
const PLAIN: Scenery = Scenery {
    sky_top: 0xb9d6ea,
    sky_bottom: 0xeef3ea,
    far: 0x9db88f,
    near: 0x6f9a6a,
    sun: 0xffe7a8,
};

/// What a panel shows of `snapshot`: the moment `occasion` at `beat`, at
/// the place `place`, with `cast` in it (people on the scene, or anyone
/// no longer there drawn as their name gives them), feeling `mood` if the
/// World says. `key` seeds what is scattered, so the same moment always
/// scatters the same.
#[allow(clippy::too_many_arguments)]
pub fn panel_scene(
    snapshot: &ProjectionSnapshot,
    occasion: Occasion,
    beat: Beat,
    place: Option<SelectionId>,
    cast: &[SelectionId],
    mood: Option<Mood>,
    key: &str,
) -> PanelScene {
    let scenery = snapshot.scenery.unwrap_or(PLAIN);
    let near = art::hex(scenery.near);
    let water = near.h > 0.45 && near.h < 0.72 && near.s > 0.2;
    let season = snapshot.canvas.season;
    let item = |id: SelectionId| snapshot.canvas.items.iter().find(|item| item.id == id);
    let setting = place.and_then(item).map(|item| Setting {
        drawing: snapshot.drawing_of(item).cloned(),
        shape: item.shape.unwrap_or_default(),
        palette: Palette::of(&item.id.stable_key(), false),
    });
    let (stance, feeling) = occasion.pose(beat);
    let mood = mood.unwrap_or(feeling);
    let mut people = cast
        .iter()
        .take(MOST_IN_A_PANEL)
        .map(|id| {
            let found = item(*id);
            let figure = Figure::of(&id.stable_key(), found.and_then(|item| item.look));
            Cast {
                figure,
                drawing: found.and_then(|item| snapshot.drawing_of(item)).cloned(),
                stance,
                mood,
                facing: 1.0,
                carrying: None,
            }
        })
        .collect::<Vec<_>>();
    // Before someone comes of age, they are as young as they were.
    if occasion == Occasion::ComingOfAge && beat == Beat::Before {
        if let Some(first) = people.first_mut() {
            first.figure.age = match first.figure.age {
                Age::Adult => Age::Teen,
                Age::Teen => Age::Child,
                age => age,
            };
        }
    }
    // A baby in the cast is held by the first grown-up, or, after, sleeps
    // in a pram; at a birth with no baby named, the parents' colours make
    // one.
    let mut pram = None;
    if let Some(baby) = people.iter().position(|who| who.figure.age == Age::Baby) {
        let baby = people.remove(baby);
        if beat == Beat::After {
            pram = Some(baby.figure);
        } else if let Some(holder) = people
            .iter_mut()
            .find(|who| matches!(who.figure.age, Age::Adult | Age::Elder | Age::Teen))
        {
            holder.carrying = Some(baby.figure);
        } else {
            pram = Some(baby.figure);
        }
    } else if occasion == Occasion::Birth && beat != Beat::Before && !people.is_empty() {
        let baby = newborn(&people, key);
        if beat == Beat::After {
            pram = Some(baby);
        } else {
            people[0].carrying = Some(baby);
        }
    }
    for who in &mut people {
        if who.carrying.is_some() {
            who.stance = Stance::Working;
        }
        // Only a drawing with a sitting stance sits, on a bench; anyone
        // else stands.
        let sits = who.drawing.as_ref().is_some_and(|drawing| {
            drawing
                .parts
                .iter()
                .any(|part| part.stances.contains(&Stance::Sitting))
        });
        if who.stance == Stance::Sitting && (!sits || who.figure.bird) {
            who.stance = Stance::Standing;
        }
    }
    // They face each other: the left half right, the right half left.
    let count = people.len();
    for (position, who) in people.iter_mut().enumerate() {
        who.facing = if (position as f32 + 0.5) < count as f32 / 2.0 {
            1.0
        } else {
            -1.0
        };
    }
    PanelScene {
        scenery,
        season,
        occasion,
        beat,
        hour: occasion.hour(beat),
        weather: occasion.weather(beat, snapshot.weather),
        water,
        place: setting,
        cast: people,
        pram,
        seed: art::seed_of(key),
    }
}

/// How high a bench's seat is, in tenths of a grown person's height.
const SEAT: f32 = 1.7;

/// How far up from the ground a drawing sitting down sits, in tenths of
/// its height: the stance draws them on the ground itself.
const SITS_AT: f32 = 0.35;

/// The most people a panel draws: more would crowd it.
pub const MOST_IN_A_PANEL: usize = 4;

/// A baby who looks like the parents in the panel: the first one's skin,
/// the second one's hair.
fn newborn(people: &[Cast], key: &str) -> Figure {
    let first = people[0].figure;
    let second = people.get(1).map_or(first, |who| who.figure);
    Figure {
        skin: diorama::mix(first.skin, second.skin, 0.5),
        hair: second.hair,
        clothes: art::hex([0xf2d7df, 0xd9e6f2, 0xf2ecd2][(art::seed_of(key) % 3) as usize]),
        age: Age::Baby,
        stoop: false,
        carries: None,
        bird: false,
    }
}

/// Where things stand in a panel `width` by `height`.
#[derive(Clone, Debug, PartialEq)]
pub struct PanelLayout {
    pub horizon: f32,
    /// The line the place stands on.
    pub base: f32,
    /// The line people stand on.
    pub feet: f32,
    pub figure_h: f32,
    pub building_h: f32,
    /// Where the place stands across it: a little further along each beat,
    /// as if the eye moved on with time.
    pub place_x: f32,
    /// Where each of `count` people stands across it.
    pub people_x: Vec<f32>,
}

/// Lays out a panel `width` by `height` for `count` people at `beat`:
/// apart before, close at the moment, and together after.
pub fn panel_layout(width: f32, height: f32, count: usize, beat: Beat) -> PanelLayout {
    // Each beat is framed a little differently, so a strip reads as time
    // passing: before from a step back with the place to the right, the
    // moment closer in with the place to the left, after in between with
    // the place to the right again.
    let (zoom, place, centre) = match beat {
        Beat::Before => (0.92, 0.68, 0.38),
        Beat::Moment => (1.12, 0.33, 0.56),
        Beat::After => (1.0, 0.66, 0.42),
    };
    let figure_h = height * 0.4 * zoom;
    let spacing = match beat {
        Beat::Before => figure_h * 0.95,
        Beat::Moment => figure_h * 0.55,
        Beat::After => figure_h * 0.62,
    };
    // Never wider than the panel, with a margin; a crowd stands closer
    // at the moment than before it.
    let share = match beat {
        Beat::Before => 0.86,
        Beat::Moment => 0.7,
        Beat::After => 0.76,
    };
    let spacing = spacing.min((width * share) / count.max(1) as f32);
    let row = spacing * count.saturating_sub(1) as f32;
    // The group kept inside the panel, a figure's half-width from its edges.
    let margin = figure_h * 0.3;
    let centre = (width * centre)
        .max(margin + row / 2.0)
        .min(width - margin - row / 2.0);
    let people_x = (0..count)
        .map(|position| centre - row / 2.0 + spacing * position as f32)
        .collect();
    PanelLayout {
        horizon: height * (0.46 - 0.04 * (zoom - 1.0)),
        base: height * (0.66 + 0.02 * (zoom - 1.0)),
        feet: height * 0.92,
        figure_h,
        building_h: height * 0.46 * zoom,
        place_x: width * place,
        people_x,
    }
}

/// Paints a panel `width` by `height` (in its own units) at `scale`
/// pixels a unit.
pub fn paint_panel(scene: &PanelScene, width: f32, height: f32, scale: f32) -> Option<sk::Pixmap> {
    let mut canvas = Canvas::new(
        (width * scale).ceil().max(2.0) as u32,
        (height * scale).ceil().max(2.0) as u32,
        scale,
        (0.0, 0.0),
    )?;
    let layout = panel_layout(width, height, scene.cast.len(), scene.beat);
    let light = diorama::light_at(scene.hour, scene.weather);
    let scenery = scene.scenery.in_season(match scene.season {
        Some(Season::Summer) => 1,
        Some(Season::Autumn) => 2,
        Some(Season::Winter) => 3,
        _ => 0,
    });
    paint_sky(&mut canvas, scene, &scenery, width, layout.horizon);
    paint_land(&mut canvas, scene, &scenery, width, height, &layout);
    // What the hour's light does to the land: gold at dusk, grey in a
    // storm. The sky makes its own light.
    let mut lit = Canvas::new(canvas.width(), canvas.height(), scale, (0.0, 0.0))?;
    paint_place(&mut lit, scene, &layout);
    paint_props_behind(&mut lit, scene, width, &layout);
    paint_people(&mut lit, scene, &layout);
    painter::tint(&mut lit, light, None);
    canvas.draw(&lit.pixmap, 0.0, 0.0, sk::BlendMode::SourceOver);
    paint_props_over(&mut canvas, scene, width, height);
    painter::grain(&mut canvas, (0, 0), 0.05, 0.035);
    painter::vignette(&mut canvas, width, height, art::hex(0x24180f).opacity(0.22));
    Some(canvas.pixmap)
}

fn daylight(hour: f32) -> Daylight {
    let (from, to, t) = diorama::between(hour);
    if t < 0.5 {
        from
    } else {
        to
    }
}

fn paint_sky(canvas: &mut Canvas, scene: &PanelScene, scenery: &Scenery, width: f32, horizon: f32) {
    let top = art::hex(scenery.sky_top);
    let bottom = art::hex(scenery.sky_bottom);
    let tinted = |daylight: Daylight| {
        let (tint_top, tint_bottom) = match daylight {
            Daylight::Day => ((0xffffff, 0.0), (0xffffff, 0.0)),
            Daylight::Dawn => ((0xffbaa0, 0.20), (0xffe4c8, 0.14)),
            Daylight::Dusk => ((0x7a5a9a, 0.34), (0xffa060, 0.5)),
            Daylight::Night => ((0x0e1436, 0.78), (0x1c2450, 0.64)),
        };
        (
            diorama::mix(top, art::hex(tint_top.0), tint_top.1),
            diorama::mix(bottom, art::hex(tint_bottom.0), tint_bottom.1),
        )
    };
    let (from, to, t) = diorama::between(scene.hour);
    let (a, b) = (tinted(from), tinted(to));
    let (mut sky_top, mut sky_bottom) = (diorama::mix(a.0, b.0, t), diorama::mix(a.1, b.1, t));
    if let Some((grey, amount)) = diorama::overcast(scene.weather) {
        sky_top = diorama::mix(sky_top, art::hex(grey), amount);
        sky_bottom = diorama::mix(sky_bottom, art::hex(grey), amount * 0.8);
    }
    let sky = Shape::polygon(&[
        (0.0, 0.0),
        (width, 0.0),
        (width, horizon * 1.6),
        (0.0, horizon * 1.6),
    ]);
    painter::fill_shaded(
        canvas,
        &sky,
        (0.0, 0.0),
        (0.0, horizon),
        &[(0.0, sky_top), (1.0, sky_bottom)],
    );
    // The sun low over the hills, or the moon; none under cloud.
    let night = daylight(scene.hour) == Daylight::Night;
    if matches!(scene.weather, Weather::Clear | Weather::Snow) {
        let (across, high) = diorama::sun_at(scene.hour);
        let x = width * (0.5 + across * 0.4);
        let y = horizon - high.max(0.08) * horizon * 0.85;
        let sun = if night {
            art::hex(0xf2f0e6)
        } else {
            art::hex(scenery.sun)
        };
        painter::glow(
            canvas,
            x,
            y,
            horizon * 0.5,
            horizon * 0.5,
            sun.opacity(0.35),
        );
        canvas.fill(&Shape::ellipse(x, y, horizon * 0.09, horizon * 0.09), sun);
    }
    // A soft bank of cloud, darker in a storm.
    if scene.weather != Weather::Clear {
        let cloud = if scene.weather == Weather::Storm {
            art::hex(0x4a525e)
        } else {
            art::hex(0xf4f2ee)
        };
        for index in 0..4 {
            let seed = art::seed_of(&format!("{}-{index}", scene.seed));
            let x = width * ((seed % 1000) as f32 / 1000.0);
            let y = horizon * (0.18 + ((seed >> 10) % 100) as f32 / 400.0);
            canvas.soft(
                x,
                y,
                width * 0.2,
                horizon * 0.09,
                horizon * 0.12,
                cloud.opacity(0.7),
            );
        }
    }
}

fn ridge(width: f32, top: f32, amplitude: f32, seed: u32, bottom: f32) -> Shape {
    let mut points = vec![(0.0, bottom)];
    let phase = (seed % 628) as f32 / 100.0;
    for step in 0..=24 {
        let x = width * step as f32 / 24.0;
        let u = x / width.max(1.0) * std::f32::consts::TAU;
        let y = top
            - amplitude
                * (0.55 * (u * 1.3 + phase).sin() + 0.3 * (u * 2.9 + phase * 1.7).sin() + 0.4);
        points.push((x, y));
    }
    points.push((width, bottom));
    Shape::polygon(&points)
}

fn paint_land(
    canvas: &mut Canvas,
    scene: &PanelScene,
    scenery: &Scenery,
    width: f32,
    height: f32,
    layout: &PanelLayout,
) {
    let light = diorama::light_at(scene.hour, scene.weather);
    let lit = |colour: Hsla| {
        let rgba: gpui::Rgba = colour.into();
        Hsla::from(gpui::Rgba {
            r: rgba.r * light[0],
            g: rgba.g * light[1],
            b: rgba.b * light[2],
            a: rgba.a,
        })
    };
    let sky_bottom = art::hex(scenery.sky_bottom);
    let far = art::hex(scenery.far);
    let near = art::hex(scenery.near);
    // Far hills fading into the air.
    let pan = match scene.beat {
        Beat::Before => 0,
        Beat::Moment => 37,
        Beat::After => 71,
    };
    let hills = ridge(
        width,
        layout.horizon,
        height * 0.08,
        scene.seed.wrapping_add(pan),
        layout.base + 4.0,
    );
    painter::fill_shaded(
        canvas,
        &hills,
        (0.0, layout.horizon - height * 0.08),
        (0.0, layout.base),
        &[
            (0.0, lit(diorama::mix(far, sky_bottom, 0.35))),
            (1.0, lit(diorama::mix(far, sky_bottom, 0.1))),
        ],
    );
    // The ground the place stands on: the near colour, or on the water a
    // quay's worn green, and the water in front.
    let ground = if scene.water {
        diorama::mix(far, art::hex(0xb8a98c), 0.45)
    } else {
        near
    };
    let ground = match scene.season {
        Some(Season::Winter) => diorama::mix(ground, art::hex(0xf2f5f8), 0.7),
        Some(Season::Autumn) => diorama::mix(ground, art::hex(0xc8873a), 0.25),
        _ => ground,
    };
    let land_top = layout.base - height * 0.03;
    let land = ridge(width, land_top, height * 0.012, scene.seed ^ 0x55, height);
    painter::fill_shaded(
        canvas,
        &land,
        (0.0, land_top),
        (0.0, height),
        &[
            (0.0, lit(art::shade(ground, 0.08))),
            (1.0, lit(art::shade(ground, -0.12))),
        ],
    );
    if scene.water {
        let top = height * 0.955;
        painter::fill_shaded(
            canvas,
            &Shape::polygon(&[(0.0, top), (width, top), (width, height), (0.0, height)]),
            (0.0, top),
            (0.0, height),
            &[(0.0, lit(art::shade(near, 0.1))), (1.0, lit(near))],
        );
        for line in 0..6 {
            let x =
                width * ((art::seed_of(&format!("{}w{line}", scene.seed)) % 1000) as f32 / 1000.0);
            canvas.rect(
                x,
                top + 2.0 + (line % 3) as f32 * 2.5,
                width * 0.05,
                0.8,
                0.4,
                gpui::white().opacity(0.35),
            );
        }
    }
}

fn paint_place(canvas: &mut Canvas, scene: &PanelScene, layout: &PanelLayout) {
    let Some(place) = &scene.place else {
        return;
    };
    let lit = matches!(daylight(scene.hour), Daylight::Dusk | Daylight::Night);
    let h = layout.building_h;
    let x = layout.place_x;
    let base = layout.base;
    let contact = gpui::black().opacity(0.22);
    match &place.drawing {
        Some(drawing) => {
            let h = h.min(layout.building_h * 1.3 / drawing.aspect.max(0.4));
            let w = h * drawing.aspect;
            canvas.soft(x, base, w * 0.6, h * 0.05, h * 0.07, contact);
            art::paint_drawing(
                canvas,
                x,
                base,
                w,
                h,
                drawing,
                &Inks::of_place(&place.palette).lit(lit),
                Stance::Standing,
                Mood::Content,
                0.0,
                0.0,
                1.0,
            );
        }
        None => {
            let w = h * 1.1;
            canvas.soft(x, base, w * 0.6, h * 0.05, h * 0.07, contact);
            art::paint_building(canvas, x, base, w, h, place.shape, &place.palette);
        }
    }
}

/// What stands behind the people: a wedding's and a festival's bunting,
/// a ribbon across a new work's door, a memorial after a death.
fn paint_props_behind(canvas: &mut Canvas, scene: &PanelScene, width: f32, layout: &PanelLayout) {
    let festive = matches!(
        (scene.occasion, scene.beat),
        (
            Occasion::Wedding | Occasion::Festival,
            Beat::Moment | Beat::After
        ) | (Occasion::WorkOpened, Beat::Moment)
    );
    if festive {
        let colours = [0xc0463a, 0xf4f1ea, 0xe8b33c, 0x2f4a6d];
        let top = layout.horizon - layout.building_h * 0.25;
        let sag = layout.building_h * 0.12;
        let mut string = Shape::new();
        string
            .move_to(-2.0, top)
            .curve_to(width + 2.0, top, width / 2.0, top + sag * 2.0);
        canvas.stroke(&string, 0.6, art::hex(0x6b4a32));
        let flags = 11;
        for flag in 0..flags {
            let t = (flag as f32 + 0.5) / flags as f32;
            let x = width * t;
            let y = top + sag * 4.0 * t * (1.0 - t);
            let s = layout.building_h * 0.07;
            art::polygon(
                canvas,
                &[(x - s * 0.6, y), (x + s * 0.6, y), (x, y + s * 1.2)],
                art::hex(colours[flag % colours.len()]),
            );
        }
    }
    if scene.occasion == Occasion::Festival && daylight(scene.hour) != Daylight::Day {
        for lantern in 0..5 {
            let x = width * (0.12 + lantern as f32 * 0.19);
            let y = layout.horizon - layout.building_h * 0.05 + (lantern % 2) as f32 * 3.0;
            painter::glow(canvas, x, y, 9.0, 9.0, art::hex(0xffc36a).opacity(0.6));
            canvas.fill(&Shape::ellipse(x, y, 2.4, 3.0), art::hex(0xe8603a));
        }
    }
    if scene.occasion == Occasion::WorkOpened && scene.beat == Beat::Moment {
        let y = layout.base - layout.building_h * 0.22;
        let (left, right) = (
            layout.place_x - layout.building_h * 0.4,
            layout.place_x + layout.building_h * 0.4,
        );
        art::line(canvas, (left, y), (right, y), 1.4, art::hex(0xc0463a));
        let x = layout.place_x;
        art::polygon(
            canvas,
            &[(x, y), (x - 4.0, y - 3.0), (x - 4.0, y + 3.0)],
            art::hex(0xa8382e),
        );
        art::polygon(
            canvas,
            &[(x, y), (x + 4.0, y - 3.0), (x + 4.0, y + 3.0)],
            art::hex(0xa8382e),
        );
    }
    if matches!(scene.occasion, Occasion::Death | Occasion::Farewell) && scene.beat == Beat::After {
        paint_memorial(canvas, scene, width, layout);
    }
    // Whoever sits, sits on a bench.
    let sitting = scene
        .cast
        .iter()
        .zip(&layout.people_x)
        .filter(|(who, _)| who.stance == Stance::Sitting)
        .map(|(_, x)| *x)
        .collect::<Vec<_>>();
    if let (Some(first), Some(last)) = (sitting.first(), sitting.last()) {
        let u = layout.figure_h / 10.0;
        let (left, right) = (first - 2.6 * u, last + 2.6 * u);
        let seat = layout.feet - SEAT * u;
        let wood = art::hex(0x8a5a33);
        canvas.soft(
            (left + right) / 2.0,
            layout.feet,
            (right - left) * 0.55,
            0.5 * u,
            0.6 * u,
            gpui::black().opacity(0.18),
        );
        for x in [left + 0.8 * u, right - 0.8 * u] {
            art::rect(
                canvas,
                x - 0.25 * u,
                seat,
                0.5 * u,
                SEAT * u,
                0.1 * u,
                art::shade(wood, -0.3),
            );
        }
        art::rect(
            canvas,
            left,
            seat - 3.0 * u,
            right - left,
            0.45 * u,
            0.15 * u,
            art::shade(wood, -0.1),
        );
        art::rect(
            canvas,
            left,
            seat - 2.2 * u,
            right - left,
            0.45 * u,
            0.15 * u,
            art::shade(wood, -0.1),
        );
        art::rect(
            canvas,
            left,
            seat - 0.2 * u,
            right - left,
            0.5 * u,
            0.15 * u,
            wood,
        );
    }
}

/// A bench by the water in memory of someone, with flowers laid on it.
fn paint_memorial(canvas: &mut Canvas, scene: &PanelScene, width: f32, layout: &PanelLayout) {
    let x = layout.people_x.last().map_or(width * 0.7, |last| {
        (last + layout.figure_h * 0.75).min(width * 0.88)
    });
    let y = layout.feet - layout.figure_h * 0.02;
    let u = layout.figure_h / 10.0;
    let wood = art::hex(0x8a5a33);
    let dark = art::shade(wood, -0.3);
    canvas.soft(x, y, 3.2 * u, 0.4 * u, 0.5 * u, gpui::black().opacity(0.2));
    if scene.occasion == Occasion::Death {
        // A stone, rounded at the top.
        art::rect(
            canvas,
            x - 1.6 * u,
            y - 4.4 * u,
            3.2 * u,
            4.4 * u,
            1.4 * u,
            art::hex(0xb7b3ab),
        );
        art::rect(
            canvas,
            x - 1.1 * u,
            y - 3.4 * u,
            2.2 * u,
            0.25 * u,
            0.1 * u,
            art::hex(0x8f8a80),
        );
        art::rect(
            canvas,
            x - 0.8 * u,
            y - 2.8 * u,
            1.6 * u,
            0.2 * u,
            0.1 * u,
            art::hex(0x8f8a80),
        );
    } else {
        for leg in [-1.0_f32, 1.0] {
            art::rect(
                canvas,
                x + leg * 2.2 * u - 0.25 * u,
                y - 2.0 * u,
                0.5 * u,
                2.0 * u,
                0.1 * u,
                dark,
            );
        }
        art::rect(
            canvas,
            x - 2.8 * u,
            y - 2.2 * u,
            5.6 * u,
            0.5 * u,
            0.15 * u,
            wood,
        );
        art::rect(
            canvas,
            x - 2.8 * u,
            y - 3.8 * u,
            5.6 * u,
            0.45 * u,
            0.15 * u,
            wood,
        );
        art::rect(
            canvas,
            x - 2.8 * u,
            y - 3.05 * u,
            5.6 * u,
            0.45 * u,
            0.15 * u,
            wood,
        );
    }
    // Flowers laid at its foot.
    for (index, colour) in [0xf2f0e6_u32, 0xe8b33c, 0xd9778b].iter().enumerate() {
        let fx = x - 0.8 * u + index as f32 * 0.8 * u;
        art::line(
            canvas,
            (fx, y),
            (fx + 0.3 * u, y - 1.2 * u),
            0.15 * u,
            art::hex(0x4f7a3a),
        );
        art::circle(
            canvas,
            fx + 0.3 * u,
            y - 1.3 * u,
            0.38 * u,
            art::hex(*colour),
        );
    }
}

fn paint_people(canvas: &mut Canvas, scene: &PanelScene, layout: &PanelLayout) {
    let contact = gpui::black().opacity(0.22);
    for (who, x) in scene.cast.iter().zip(&layout.people_x) {
        let height = layout.figure_h * who.figure.age.height();
        // Someone sitting sits on the bench, not on the ground.
        let feet = if who.stance == Stance::Sitting {
            layout.feet - (SEAT * layout.figure_h - SITS_AT * height) / 10.0
        } else {
            layout.feet
        };
        canvas.soft(
            *x,
            layout.feet,
            height * 0.22,
            height * 0.05,
            height * 0.06,
            contact,
        );
        age::paint_person(
            canvas,
            *x,
            feet,
            height,
            &who.figure,
            who.drawing.as_ref(),
            who.stance,
            who.mood,
            Pose {
                facing: who.facing,
                ..Pose::default()
            },
        );
        if let Some(baby) = &who.carrying {
            age::paint_bundle(
                canvas,
                *x,
                layout.feet,
                layout.figure_h,
                who.facing,
                0.0,
                baby,
                &who.figure,
            );
        }
    }
    if let Some(baby) = &scene.pram {
        let x = layout
            .people_x
            .first()
            .map_or(layout.place_x, |first| first - layout.figure_h * 0.55);
        let x = x.max(layout.figure_h * 0.35);
        canvas.soft(x, layout.feet, layout.figure_h * 0.2, 2.0, 2.0, contact);
        age::paint_pram(
            canvas,
            x,
            layout.feet,
            layout.figure_h,
            baby,
            Pose::default(),
        );
    }
}

/// What falls over everything: petals at a wedding, rain in a storm, and
/// after a storm a rainbow.
fn paint_props_over(canvas: &mut Canvas, scene: &PanelScene, width: f32, height: f32) {
    match (scene.occasion, scene.beat) {
        (Occasion::Wedding, Beat::Moment) => {
            for petal in 0..34 {
                let seed = art::seed_of(&format!("{}p{petal}", scene.seed));
                let x = width * ((seed % 1000) as f32 / 1000.0);
                let y = height * (((seed >> 10) % 1000) as f32 / 1000.0) * 0.85;
                let colour = if seed.is_multiple_of(3) {
                    0xf2b6ca
                } else {
                    0xfbeef2
                };
                canvas.fill(
                    &Shape::ellipse(x, y, 1.3, 0.8),
                    art::hex(colour).opacity(0.9),
                );
            }
        }
        (Occasion::Storm, Beat::Moment) => {
            let rain = art::hex(0xc8d2dc).opacity(0.55);
            for drop in 0..70 {
                let seed = art::seed_of(&format!("{}r{drop}", scene.seed));
                let x = width * ((seed % 1000) as f32 / 1000.0);
                let y = height * (((seed >> 10) % 1000) as f32 / 1000.0);
                art::line(canvas, (x, y), (x - 3.0, y + 9.0), 0.6, rain);
            }
        }
        (Occasion::Storm, Beat::After) => {
            let (cx, cy) = (width * 0.62, height * 0.62);
            for (index, colour) in [0xe06f6f_u32, 0xf0b050, 0xf2e27a, 0x7cc47c, 0x6fa0e0]
                .iter()
                .enumerate()
            {
                let r = height * (0.52 - index as f32 * 0.025);
                let mut arc = Shape::new();
                arc.move_to(cx - r, cy)
                    .curve_to(cx, cy - r, cx - r, cy - r)
                    .curve_to(cx + r, cy, cx + r, cy - r);
                canvas.stroke(&arc, height * 0.022, art::hex(*colour).opacity(0.28));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn people_stand_apart_before_close_at_the_moment_and_inside_the_panel() {
        for count in 1..6 {
            let before = panel_layout(240.0, 180.0, count, Beat::Before);
            let moment = panel_layout(240.0, 180.0, count, Beat::Moment);
            let after = panel_layout(240.0, 180.0, count, Beat::After);
            for layout in [&before, &moment, &after] {
                assert_eq!(layout.people_x.len(), count);
                for x in &layout.people_x {
                    assert!(*x > 0.0 && *x < 240.0, "{x} is off the panel");
                }
                assert!(layout.feet > layout.base && layout.base > layout.horizon);
            }
            if count > 1 {
                let spread = |layout: &PanelLayout| {
                    layout.people_x.last().unwrap() - layout.people_x.first().unwrap()
                };
                assert!(spread(&before) > spread(&moment), "{count}");
            }
        }
    }

    #[test]
    fn a_moment_has_the_hour_and_weather_of_its_kind() {
        assert!(
            Occasion::Birth.hour(Beat::Moment) < 12.0,
            "a birth in the morning"
        );
        assert!(
            Occasion::Farewell.hour(Beat::Moment) > 18.0,
            "a farewell at dusk"
        );
        assert_eq!(
            Occasion::Storm.weather(Beat::Moment, Weather::Clear),
            Weather::Storm
        );
        assert_eq!(
            Occasion::Storm.weather(Beat::After, Weather::Storm),
            Weather::Clear
        );
        assert_eq!(
            Occasion::Wedding.weather(Beat::Moment, Weather::Storm),
            Weather::Cloudy
        );
        for beat in [Beat::Before, Beat::Moment, Beat::After] {
            let before = Occasion::Wedding.hour(Beat::Before);
            assert!(Occasion::Wedding.hour(beat) >= before);
        }
    }
}

#[cfg(test)]
pub(crate) mod sketches {
    use super::*;

    /// Writes a pixmap as a PNG, for looking at while working.
    pub(crate) fn save(pixmap: &sk::Pixmap, path: &std::path::Path) {
        let image =
            image::RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.data().to_vec())
                .expect("an image");
        image.save(path).expect("saved");
    }

    fn person(key: &str, age: Age, clothes: u32) -> Cast {
        let mut figure = Figure::of(key, None);
        figure.age = age;
        figure.clothes = art::hex(clothes);
        if age == Age::Elder {
            figure.hair = age::greyed(figure.hair);
            figure.stoop = true;
        }
        Cast {
            figure,
            drawing: Some(world_projection::person(
                format!("sketch-{key}"),
                art::seed_of(key) % 48,
                &world_projection::person_base("sketch"),
            )),
            stance: Stance::Standing,
            mood: Mood::Content,
            facing: 1.0,
            carrying: None,
        }
    }

    /// `WORLD_GPUI_SKETCH=dir cargo test -p world-gpui sketch -- --ignored`
    #[test]
    #[ignore = "writes pictures to look at"]
    fn sketch_the_panels_and_the_ages() {
        let Some(dir) = std::env::var_os("WORLD_GPUI_SKETCH") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        let scenery = Scenery {
            sky_top: 0x9cc6e6,
            sky_bottom: 0xf0ead8,
            far: 0x8fae7e,
            near: 0x4f86a8,
            sun: 0xffe2a0,
        };
        for occasion in [
            Occasion::Wedding,
            Occasion::Birth,
            Occasion::Farewell,
            Occasion::Death,
            Occasion::Storm,
            Occasion::Festival,
            Occasion::WorkOpened,
        ] {
            let mut strip = sk::Pixmap::new(3 * 480 + 40, 360).unwrap();
            for (index, beat) in [Beat::Before, Beat::Moment, Beat::After]
                .into_iter()
                .enumerate()
            {
                let mut cast = vec![
                    person("mara", Age::Adult, 0xc8553d),
                    person("leo", Age::Adult, 0x3f6fb0),
                ];
                if occasion == Occasion::Farewell {
                    cast.insert(0, person("noah", Age::Elder, 0x6b5a48));
                }
                if occasion == Occasion::Festival {
                    cast.push(person("kit", Age::Child, 0xe8b33c));
                    cast.push(person("sam", Age::Teen, 0x3c9a8f));
                }
                let (stance, mood) = occasion.pose(beat);
                for who in &mut cast {
                    who.stance = stance;
                    who.mood = mood;
                }
                let count = cast.len();
                for (position, who) in cast.iter_mut().enumerate() {
                    who.facing = if (position as f32 + 0.5) < count as f32 / 2.0 {
                        1.0
                    } else {
                        -1.0
                    };
                }
                if occasion == Occasion::Birth && beat == Beat::Moment {
                    let mut baby = Figure::of("baby", None);
                    baby.age = Age::Baby;
                    cast[0].carrying = Some(baby);
                    cast[0].stance = Stance::Working;
                }
                let scene = PanelScene {
                    scenery,
                    season: None,
                    occasion,
                    beat,
                    hour: occasion.hour(beat),
                    weather: occasion.weather(beat, Weather::Clear),
                    water: true,
                    place: Some(Setting {
                        drawing: None,
                        shape: MarkShape::House,
                        palette: Palette::of("chapel", false),
                    }),
                    cast,
                    pram: (occasion == Occasion::Birth && beat == Beat::After).then(|| {
                        let mut baby = Figure::of("baby", None);
                        baby.age = Age::Baby;
                        baby
                    }),
                    seed: 7,
                };
                let panel = paint_panel(&scene, 240.0, 180.0, 2.0).unwrap();
                strip.draw_pixmap(
                    index as i32 * 500,
                    0,
                    panel.as_ref(),
                    &sk::PixmapPaint::default(),
                    sk::Transform::identity(),
                    None,
                );
            }
            save(&strip, &dir.join(format!("panels-{occasion:?}.png")));
        }
        if std::env::var_os("SKETCH_DEBUG").is_some() {
            let scene = PanelScene {
                scenery,
                season: None,
                occasion: Occasion::Birth,
                beat: Beat::After,
                hour: 11.5,
                weather: Weather::Clear,
                water: true,
                place: None,
                cast: vec![],
                pram: None,
                seed: 7,
            };
            save(
                &paint_panel(&scene, 240.0, 180.0, 2.0).unwrap(),
                &dir.join("debug.png"),
            );
        }
        // Everyone at every age, in the app's figure and a Pack's drawing.
        let mut canvas = Canvas::new(1000, 360, 2.0, (0.0, 0.0)).unwrap();
        canvas.rect(0.0, 0.0, 500.0, 180.0, 0.0, art::hex(0xf2eee6));
        for (index, age_) in [Age::Baby, Age::Child, Age::Teen, Age::Adult, Age::Elder]
            .into_iter()
            .enumerate()
        {
            let who = person(&format!("p{index}"), age_, 0x3f6fb0);
            for (row, drawing) in [None, who.drawing.as_ref()].into_iter().enumerate() {
                let x = 40.0 + index as f32 * 90.0 + row as f32 * 40.0;
                let h = 70.0 * age_.height();
                age::paint_person(
                    &mut canvas,
                    x,
                    150.0,
                    h,
                    &who.figure,
                    drawing,
                    if index == 4 {
                        Stance::Walking
                    } else {
                        Stance::Standing
                    },
                    Mood::Happy,
                    Pose {
                        facing: 1.0,
                        stride: (index == 4).then_some(0.3),
                        ..Pose::default()
                    },
                );
            }
        }
        save(&canvas.pixmap, &dir.join("ages.png"));
    }
}
