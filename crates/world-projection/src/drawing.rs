//! Drawings a World Pack ships for its people, buildings and things.
//!
//! A drawing is a handful of flat shapes in a box standing on the ground:
//! `x` runs across it in widths from its middle (-0.5 to 0.5) and `y` up it
//! in heights from the ground (0 to 1). Each shape is inked with a fixed
//! colour or with a role the app fills in (a place's walls and roof, a
//! person's clothes, hair and skin, lit glass at night), so one drawing
//! serves every house on a street and every person in their own clothes.
//! A person's drawing can have parts for a stance only (walking, working,
//! talking, celebrating) and parts that swing with a walk.
//!
//! Drawings are presentation only: nothing in a World's state is ever read
//! from them.

/// What a person is doing, as far as how they are drawn goes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Stance {
    #[default]
    Standing,
    Walking,
    Working,
    Talking,
    Celebrating,
    /// Idle: a hand to the brow, looking out over the place.
    LookingAround,
    /// Idle: arms up and out, a yawn.
    Stretching,
    /// Idle: sitting down, on a bench or the ground.
    Sitting,
    /// A hand raised in greeting: when the player clicks on them.
    Waving,
}

impl Stance {
    pub const ALL: [Stance; 9] = [
        Stance::Standing,
        Stance::Walking,
        Stance::Working,
        Stance::Talking,
        Stance::Celebrating,
        Stance::LookingAround,
        Stance::Stretching,
        Stance::Sitting,
        Stance::Waving,
    ];

    /// What someone does with an idle moment, in turn.
    pub const IDLE: [Stance; 4] = [
        Stance::LookingAround,
        Stance::Stretching,
        Stance::Sitting,
        Stance::Working,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Stance::Standing => "standing",
            Stance::Walking => "walking",
            Stance::Working => "working",
            Stance::Talking => "talking",
            Stance::Celebrating => "celebrating",
            Stance::LookingAround => "looking_around",
            Stance::Stretching => "stretching",
            Stance::Sitting => "sitting",
            Stance::Waving => "waving",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|stance| stance.id() == id)
    }
}

/// How someone feels, as far as their face goes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Mood {
    #[default]
    Content,
    Happy,
    Sad,
    Cross,
    Thinking,
}

impl Mood {
    pub const ALL: [Mood; 5] = [
        Mood::Content,
        Mood::Happy,
        Mood::Sad,
        Mood::Cross,
        Mood::Thinking,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Mood::Content => "content",
            Mood::Happy => "happy",
            Mood::Sad => "sad",
            Mood::Cross => "cross",
            Mood::Thinking => "thinking",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mood| mood.id() == id)
    }
}

/// What a shape is filled with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ink {
    /// A fixed colour, 0xRRGGBB.
    Colour(u32),
    Wall,
    Roof,
    Trim,
    /// Windows: lit at night.
    Glass,
    Clothes,
    Hair,
    Skin,
    /// A soft shadow.
    Shade,
}

impl Ink {
    pub fn id(self) -> String {
        match self {
            Ink::Colour(colour) => format!("#{colour:06x}"),
            Ink::Wall => "wall".into(),
            Ink::Roof => "roof".into(),
            Ink::Trim => "trim".into(),
            Ink::Glass => "glass".into(),
            Ink::Clothes => "clothes".into(),
            Ink::Hair => "hair".into(),
            Ink::Skin => "skin".into(),
            Ink::Shade => "shade".into(),
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "wall" => Ink::Wall,
            "roof" => Ink::Roof,
            "trim" => Ink::Trim,
            "glass" => Ink::Glass,
            "clothes" => Ink::Clothes,
            "hair" => Ink::Hair,
            "skin" => Ink::Skin,
            "shade" => Ink::Shade,
            colour => Ink::Colour(u32::from_str_radix(colour.strip_prefix('#')?, 16).ok()?),
        })
    }
}

/// One flat shape, in the drawing's own box.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawShape {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        /// How round its corners are, in widths.
        round: f32,
    },
    Ellipse {
        x: f32,
        y: f32,
        rx: f32,
        ry: f32,
    },
    Polygon {
        points: Vec<(f32, f32)>,
    },
    Line {
        from: (f32, f32),
        to: (f32, f32),
        /// How thick, in widths.
        width: f32,
    },
}

/// A shape, what it is filled with, and when it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawPart {
    pub shape: DrawShape,
    pub ink: Ink,
    /// Lighter (up to 1) or darker (down to -1) than its ink.
    pub tone: f32,
    /// The stances it shows in; every stance when empty.
    pub stances: Vec<Stance>,
    /// The moods it shows in; every mood when empty.
    pub moods: Vec<Mood>,
    /// How far it swings sideways with each step of a walk, in widths.
    pub swing: f32,
}

impl DrawPart {
    fn of(shape: DrawShape, ink: Ink) -> Self {
        Self {
            shape,
            ink,
            tone: 0.0,
            stances: Vec::new(),
            moods: Vec::new(),
            swing: 0.0,
        }
    }

    pub fn rect(x: f32, y: f32, w: f32, h: f32, ink: Ink) -> Self {
        Self::of(
            DrawShape::Rect {
                x,
                y,
                w,
                h,
                round: 0.0,
            },
            ink,
        )
    }

    pub fn ellipse(x: f32, y: f32, rx: f32, ry: f32, ink: Ink) -> Self {
        Self::of(DrawShape::Ellipse { x, y, rx, ry }, ink)
    }

    pub fn polygon(points: &[(f32, f32)], ink: Ink) -> Self {
        Self::of(
            DrawShape::Polygon {
                points: points.to_vec(),
            },
            ink,
        )
    }

    pub fn line(from: (f32, f32), to: (f32, f32), width: f32, ink: Ink) -> Self {
        Self::of(DrawShape::Line { from, to, width }, ink)
    }

    /// Rounds a rectangle's corners.
    pub fn round(mut self, by: f32) -> Self {
        if let DrawShape::Rect { round, .. } = &mut self.shape {
            *round = by;
        }
        self
    }

    pub fn tone(mut self, tone: f32) -> Self {
        self.tone = tone.clamp(-1.0, 1.0);
        self
    }

    /// Shows only in these stances.
    pub fn only(mut self, stances: &[Stance]) -> Self {
        self.stances = stances.to_vec();
        self
    }

    /// Shows only in these moods.
    pub fn feeling(mut self, moods: &[Mood]) -> Self {
        self.moods = moods.to_vec();
        self
    }

    pub fn swing(mut self, swing: f32) -> Self {
        self.swing = swing;
        self
    }

    /// Whether it shows in a stance.
    pub fn shows_in(&self, stance: Stance) -> bool {
        self.stances.is_empty() || self.stances.contains(&stance)
    }

    /// Whether it shows in a stance and a mood.
    pub fn shows(&self, stance: Stance, mood: Mood) -> bool {
        self.shows_in(stance) && (self.moods.is_empty() || self.moods.contains(&mood))
    }
}

/// A drawing a Pack ships, named so the scene's items can point at it.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    pub id: String,
    /// How wide it is for how tall: a person about 0.45, a cottage about 1.
    pub aspect: f32,
    pub parts: Vec<DrawPart>,
    /// How tall it stands beside a grown-up resident (or how wide a long,
    /// low thing lies), as its Pack declares it: the app stands it to that
    /// size whatever spot it is given. None keeps the size its spot gives.
    pub rung: Option<world_art::Rung>,
}

impl Drawing {
    pub fn new(id: impl Into<String>, aspect: f32, parts: Vec<DrawPart>) -> Self {
        Self {
            id: id.into(),
            aspect,
            parts,
            rung: None,
        }
    }

    /// The same drawing, standing `rung` beside a resident.
    pub fn standing(mut self, rung: world_art::Rung) -> Self {
        self.rung = Some(rung);
        self
    }

    /// A copy with more parts drawn over it: a person's own hat over the
    /// body everyone shares.
    pub fn with(&self, id: impl Into<String>, more: Vec<DrawPart>) -> Self {
        let mut parts = self.parts.clone();
        parts.extend(more);
        Self {
            id: id.into(),
            aspect: self.aspect,
            parts,
            rung: self.rung,
        }
    }

    /// Its outline standing still and content: the shapes it shows, placed
    /// and sized to a hundredth, whatever colour they are. Two drawings
    /// with the same silhouette read as the same shape against the sky.
    pub fn silhouette(&self) -> Vec<String> {
        let q = |v: f32| (v * 100.0).round() as i32;
        let mut shapes = self
            .parts
            .iter()
            .filter(|part| part.shows(Stance::Standing, Mood::Content))
            .map(|part| match &part.shape {
                DrawShape::Rect { x, y, w, h, .. } => {
                    format!("r{},{},{},{}", q(*x), q(*y), q(*w), q(*h))
                }
                DrawShape::Ellipse { x, y, rx, ry } => {
                    format!("e{},{},{},{}", q(*x), q(*y), q(*rx), q(*ry))
                }
                DrawShape::Polygon { points } => points
                    .iter()
                    .map(|(x, y)| format!("{},{}", q(*x), q(*y)))
                    .collect::<Vec<_>>()
                    .join(";"),
                DrawShape::Line { from, to, width } => format!(
                    "l{},{},{},{},{}",
                    q(from.0),
                    q(from.1),
                    q(to.0),
                    q(to.1),
                    q(*width)
                ),
            })
            .collect::<Vec<_>>();
        shapes.sort();
        shapes.dedup();
        shapes
    }

    /// The parts it shows in a mood, standing: its face in that mood.
    pub fn face(&self, mood: Mood) -> Vec<usize> {
        self.parts
            .iter()
            .enumerate()
            .filter(|(_, part)| part.shows(Stance::Standing, mood))
            .map(|(index, _)| index)
            .collect()
    }

    /// The most parts a drawing may have.
    pub const MOST_PARTS: usize = 160;

    /// Whether the app can draw it: a name, a sensible shape, a bounded
    /// number of finite parts that stay near their box.
    pub fn is_drawable(&self) -> bool {
        let near = |value: f32| value.is_finite() && (-2.0..=2.0).contains(&value);
        !self.id.trim().is_empty()
            && self.aspect.is_finite()
            && (0.05..=8.0).contains(&self.aspect)
            && !self.parts.is_empty()
            && self.parts.len() <= Self::MOST_PARTS
            && self.parts.iter().all(|part| {
                near(part.tone)
                    && near(part.swing)
                    && match &part.shape {
                        DrawShape::Rect { x, y, w, h, round } => {
                            [*x, *y, *w, *h, *round].into_iter().all(near)
                        }
                        DrawShape::Ellipse { x, y, rx, ry } => {
                            [*x, *y, *rx, *ry].into_iter().all(near)
                        }
                        DrawShape::Polygon { points } => {
                            (3..=32).contains(&points.len())
                                && points.iter().all(|(x, y)| near(*x) && near(*y))
                        }
                        DrawShape::Line { from, to, width } => {
                            [from.0, from.1, to.0, to.1, *width].into_iter().all(near)
                        }
                    }
            })
    }

    /// The drawing as a picture, `height` pixels tall, in one stance, with
    /// the roles filled in by `inks`: for looking at a Pack's drawings
    /// outside the app.
    pub fn to_svg(&self, stance: Stance, height: f32, inks: &dyn Fn(Ink) -> u32) -> String {
        self.to_svg_feeling(stance, Mood::Content, height, inks)
    }

    /// The drawing as a picture in one stance and one mood.
    pub fn to_svg_feeling(
        &self,
        stance: Stance,
        mood: Mood,
        height: f32,
        inks: &dyn Fn(Ink) -> u32,
    ) -> String {
        let h = height;
        let w = height * self.aspect;
        let drop = drop_of(stance);
        let at = |x: f32, y: f32| (w * (x + 0.5), h * (1.0 - y + drop));
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w:.0}" height="{h:.0}" viewBox="0 0 {w:.1} {h:.1}">"#
        );
        for part in self.parts.iter().filter(|part| part.shows(stance, mood)) {
            let colour = toned(inks(part.ink), part.tone);
            let fill = format!("#{colour:06x}");
            let opacity = if part.ink == Ink::Shade { 0.25 } else { 1.0 };
            match &part.shape {
                DrawShape::Rect {
                    x,
                    y,
                    w: rw,
                    h: rh,
                    round,
                } => {
                    let (left, top) = at(*x, y + rh);
                    svg.push_str(&format!(
                        r#"<rect x="{left:.1}" y="{top:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" fill="{fill}" fill-opacity="{opacity}"/>"#,
                        rw * w,
                        rh * h,
                        round * w
                    ));
                }
                DrawShape::Ellipse { x, y, rx, ry } => {
                    let (cx, cy) = at(*x, *y);
                    svg.push_str(&format!(
                        r#"<ellipse cx="{cx:.1}" cy="{cy:.1}" rx="{:.1}" ry="{:.1}" fill="{fill}" fill-opacity="{opacity}"/>"#,
                        rx * w,
                        ry * h
                    ));
                }
                DrawShape::Polygon { points } => {
                    let points = points
                        .iter()
                        .map(|(x, y)| {
                            let (px, py) = at(*x, *y);
                            format!("{px:.1},{py:.1}")
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    svg.push_str(&format!(
                        r#"<polygon points="{points}" fill="{fill}" fill-opacity="{opacity}"/>"#
                    ));
                }
                DrawShape::Line { from, to, width } => {
                    let (x1, y1) = at(from.0, from.1);
                    let (x2, y2) = at(to.0, to.1);
                    svg.push_str(&format!(
                        r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{fill}" stroke-width="{:.1}" stroke-linecap="round"/>"#,
                        width * w
                    ));
                }
            }
        }
        svg.push_str("</svg>");
        svg
    }
}

/// A colour made lighter or darker by `tone`, from -1 to 1.
pub fn toned(colour: u32, tone: f32) -> u32 {
    let channel = |shift: u32| {
        let value = ((colour >> shift) & 0xff) as f32;
        let moved = if tone >= 0.0 {
            value + (255.0 - value) * tone
        } else {
            value * (1.0 + tone)
        };
        (moved.round().clamp(0.0, 255.0) as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// A figure's own units: its height is 10, and it is `4.5` wide, so a
/// drawing of someone reads like a sketch on squared paper.
const FIGURE_WIDTH: f32 = 4.5;

/// A point on a figure, from its own units (feet at the origin, head at
/// about 9.5) into a drawing's box.
pub fn figure_point(x: f32, y: f32) -> (f32, f32) {
    (x / FIGURE_WIDTH, y / 10.0)
}

/// Shapes on a figure, in its own units; see [`figure_point`].
pub mod figure {
    use super::{figure_point, DrawPart, Ink, FIGURE_WIDTH};

    pub fn rect(x: f32, y: f32, w: f32, h: f32, ink: Ink) -> DrawPart {
        let (px, py) = figure_point(x, y);
        DrawPart::rect(px, py, w / FIGURE_WIDTH, h / 10.0, ink)
    }

    pub fn ellipse(x: f32, y: f32, rx: f32, ry: f32, ink: Ink) -> DrawPart {
        let (px, py) = figure_point(x, y);
        DrawPart::ellipse(px, py, rx / FIGURE_WIDTH, ry / 10.0, ink)
    }

    pub fn polygon(points: &[(f32, f32)], ink: Ink) -> DrawPart {
        let points = points
            .iter()
            .map(|(x, y)| figure_point(*x, *y))
            .collect::<Vec<_>>();
        DrawPart::polygon(&points, ink)
    }

    pub fn line(from: (f32, f32), to: (f32, f32), width: f32, ink: Ink) -> DrawPart {
        DrawPart::line(
            figure_point(from.0, from.1),
            figure_point(to.0, to.1),
            width / FIGURE_WIDTH,
            ink,
        )
    }
}

/// A person, dressed in nothing of their own yet: legs, a coat in their
/// clothes, arms for every stance, a face. A Pack draws their hair, hats
/// and whatever marks them out over it with [`Drawing::with`].
pub fn person_base(id: impl Into<String>) -> Drawing {
    use figure::{ellipse, line, polygon, rect};
    use Stance::{
        Celebrating, LookingAround, Sitting, Standing, Stretching, Talking, Walking, Waving,
        Working,
    };
    let upright = [
        Standing,
        Walking,
        Working,
        Talking,
        Celebrating,
        LookingAround,
        Stretching,
        Waving,
    ];
    let trousers = Ink::Colour(0x3b3f4a);
    let shoes = Ink::Colour(0x2a2522);
    let mut parts = Vec::new();
    // Legs, swinging with a walk.
    for (side, swing) in [(-1.0_f32, 0.16_f32), (1.0, -0.16)] {
        parts.push(
            rect(side * 0.75 - 0.5, 0.3, 1.0, 3.0, trousers)
                .round(0.08)
                .swing(swing)
                .only(&upright),
        );
        parts.push(
            rect(side * 0.75 - 0.65, 0.0, 1.3, 0.5, shoes)
                .round(0.06)
                .swing(swing)
                .only(&upright),
        );
    }
    // Arms behind the coat: down at the sides, swinging with a walk.
    let sleeve = |from: (f32, f32), to: (f32, f32)| line(from, to, 0.85, Ink::Clothes).tone(-0.15);
    let hand = |x: f32, y: f32| ellipse(x, y, 0.42, 0.42, Ink::Skin);
    for (side, swing) in [(-1.0_f32, -0.1_f32), (1.0, 0.1)] {
        parts.push(
            sleeve((side * 1.6, 6.0), (side * 1.9, 3.6))
                .only(&[Standing, Walking, Sitting])
                .swing(swing),
        );
        parts.push(
            hand(side * 1.95, 3.35)
                .only(&[Standing, Walking, Sitting])
                .swing(swing),
        );
    }
    // Talking: one hand down, the other raised as they make their point.
    parts.push(sleeve((-1.6, 6.0), (-1.9, 3.6)).only(&[Talking, LookingAround, Waving]));
    parts.push(hand(-1.95, 3.35).only(&[Talking, LookingAround, Waving]));
    parts.push(sleeve((1.6, 6.0), (2.35, 4.7)).only(&[Talking]));
    parts.push(sleeve((2.35, 4.7), (2.6, 6.5)).only(&[Talking]));
    parts.push(hand(2.65, 6.75).only(&[Talking]));
    // Looking around: a hand shading the eyes.
    parts.push(sleeve((1.6, 6.0), (2.3, 6.9)).only(&[LookingAround]));
    parts.push(sleeve((2.3, 6.9), (1.0, 8.35)).only(&[LookingAround]));
    // Waving: a hand held high.
    parts.push(sleeve((1.6, 6.0), (2.6, 7.6)).only(&[Waving]));
    parts.push(sleeve((2.6, 7.6), (2.9, 9.0)).only(&[Waving]));
    parts.push(hand(2.95, 9.3).only(&[Waving]));
    // Celebrating: both arms up; stretching: up and wide.
    for side in [-1.0_f32, 1.0] {
        parts.push(sleeve((side * 1.6, 6.1), (side * 2.6, 8.7)).only(&[Celebrating]));
        parts.push(hand(side * 2.7, 9.0).only(&[Celebrating]));
        parts.push(sleeve((side * 1.6, 6.1), (side * 3.2, 7.9)).only(&[Stretching]));
        parts.push(sleeve((side * 3.2, 7.9), (side * 3.0, 9.4)).only(&[Stretching]));
        parts.push(hand(side * 2.95, 9.7).only(&[Stretching]));
    }
    // The coat, widening a little toward the hem, with rounded shoulders.
    parts.push(polygon(
        &[(-1.45, 6.3), (1.45, 6.3), (1.75, 3.0), (-1.75, 3.0)],
        Ink::Clothes,
    ));
    parts.push(rect(-1.45, 5.7, 2.9, 1.0, Ink::Clothes).round(0.12));
    parts.push(rect(-0.35, 6.4, 0.7, 0.5, Ink::Skin));
    // Sitting: the whole figure is drawn lower (see [`drop_of`]), on a
    // seat at the height of a bench: thighs run forward from the hem, and
    // the shins go down from the knee to the lowered ground.
    parts.push(
        rect(-0.6, 2.75, 3.0, 0.9, trousers)
            .round(0.1)
            .only(&[Sitting]),
    );
    parts.push(
        rect(1.55, 1.25, 0.9, 2.2, trousers)
            .round(0.08)
            .only(&[Sitting]),
    );
    parts.push(
        rect(1.45, 1.0, 1.5, 0.42, shoes)
            .round(0.06)
            .only(&[Sitting]),
    );
    // Working: both hands forward at the chest, holding what they work on.
    for side in [-1.0_f32, 1.0] {
        parts.push(sleeve((side * 1.5, 6.0), (side * 0.8, 4.5)).only(&[Working]));
        parts.push(hand(side * 0.55, 4.45).only(&[Working]));
    }
    // Head, eyes, and a mouth that opens to speak or cheer.
    parts.push(ellipse(0.0, 8.0, 1.55, 1.55, Ink::Skin));
    parts.extend(face());
    parts.push(hand(1.0, 8.55).only(&[LookingAround]));
    Drawing::new(id, FIGURE_WIDTH / 10.0, parts)
}

/// A face that shows how someone feels: eyes (shut in a stretch), brows
/// that frown, lift or slant, and a mouth that smiles, droops, sets, or
/// opens to speak, cheer or yawn.
fn face() -> Vec<DrawPart> {
    use figure::{ellipse, line, polygon};
    use Mood::{Content, Cross, Happy, Sad, Thinking};
    use Stance::{
        Celebrating, LookingAround, Sitting, Standing, Stretching, Talking, Walking, Waving,
        Working,
    };
    let dark = Ink::Colour(0x2a2522);
    let lips = Ink::Colour(0x7a3a2a);
    let open = [Talking, Celebrating];
    let quiet = [Standing, Walking, Working, LookingAround, Sitting, Waving];
    let mut parts = Vec::new();
    for side in [-1.0_f32, 1.0] {
        parts.push(ellipse(side * 0.55, 8.1, 0.17, 0.17, dark).only(&[
            Standing,
            Walking,
            Working,
            Talking,
            Celebrating,
            LookingAround,
            Sitting,
            Waving,
        ]));
        // Eyes shut in a stretch and a big smile.
        parts.push(
            line(
                (side * 0.55 - 0.22, 8.1),
                (side * 0.55 + 0.22, 8.1),
                0.12,
                dark,
            )
            .only(&[Stretching]),
        );
    }
    // Brows: slanted down to the middle when cross, up in the middle when
    // sad, one raised when thinking.
    let brow = |from: (f32, f32), to: (f32, f32)| line(from, to, 0.16, Ink::Hair).tone(-0.2);
    parts.push(brow((-0.9, 8.78), (-0.3, 8.52)).feeling(&[Cross]));
    parts.push(brow((0.3, 8.52), (0.9, 8.78)).feeling(&[Cross]));
    parts.push(brow((-0.9, 8.5), (-0.3, 8.72)).feeling(&[Sad]));
    parts.push(brow((0.3, 8.72), (0.9, 8.5)).feeling(&[Sad]));
    parts.push(brow((0.3, 8.72), (0.9, 8.86)).feeling(&[Thinking]));
    // Mouths.
    parts.push(ellipse(0.0, 7.35, 0.32, 0.2, lips).only(&open));
    parts.push(ellipse(0.0, 7.3, 0.36, 0.34, lips).only(&[Stretching]));
    parts.push(
        ellipse(0.0, 7.4, 0.35, 0.07, lips)
            .only(&quiet)
            .feeling(&[Content]),
    );
    parts.push(
        polygon(&[(-0.5, 7.5), (0.5, 7.5), (0.28, 7.2), (-0.28, 7.2)], lips)
            .only(&quiet)
            .feeling(&[Happy]),
    );
    parts.push(
        polygon(
            &[(-0.28, 7.45), (0.28, 7.45), (0.46, 7.2), (-0.46, 7.2)],
            lips,
        )
        .only(&quiet)
        .feeling(&[Sad]),
    );
    parts.push(
        line((-0.38, 7.35), (0.38, 7.35), 0.14, lips)
            .only(&quiet)
            .feeling(&[Cross]),
    );
    parts.push(
        ellipse(0.35, 7.38, 0.14, 0.1, lips)
            .only(&quiet)
            .feeling(&[Thinking]),
    );
    // A glow in the cheeks when happy.
    for side in [-1.0_f32, 1.0] {
        parts.push(ellipse(side * 0.95, 7.65, 0.3, 0.14, Ink::Colour(0xe89a8a)).feeling(&[Happy]));
    }
    parts
}

/// Short hair, for a person drawn over [`person_base`].
pub fn short_hair() -> Vec<DrawPart> {
    use figure::{ellipse, rect};
    vec![
        ellipse(0.0, 9.05, 1.62, 0.72, Ink::Hair),
        rect(-1.6, 7.9, 0.5, 1.2, Ink::Hair).round(0.1),
        rect(1.1, 7.9, 0.5, 1.2, Ink::Hair).round(0.1),
    ]
}

/// How many different silhouettes [`person`] draws: every hairstyle
/// under every hat, slight or broad.
pub const SILHOUETTES: u32 = 6 * 4 * 2;

/// Someone of their own, for anyone a Pack has not drawn by hand: one of
/// six hairstyles, under one of four hats (or none), slight or broad, and
/// maybe glasses, a beard or a scarf. Two different `variant`s below
/// [`SILHOUETTES`] never share a silhouette, so a Pack that numbers its
/// people gives each an outline of their own.
pub fn person(id: impl Into<String>, variant: u32, base: &Drawing) -> Drawing {
    use figure::{ellipse, line, polygon, rect};
    let hair_style = variant % 6;
    let hat = (variant / 6) % 4;
    let broad = (variant / 24) % 2 == 1;
    let extra = crate::drawing::mix(variant);
    let mut parts = Vec::new();
    if broad {
        parts.push(polygon(
            &[(-1.7, 6.3), (1.7, 6.3), (2.1, 3.0), (-2.1, 3.0)],
            Ink::Clothes,
        ));
        parts.push(rect(-1.7, 5.6, 3.4, 1.1, Ink::Clothes).round(0.14));
    }
    match hair_style {
        0 => parts.extend(short_hair()),
        1 => {
            // Long, past the shoulders.
            parts.push(ellipse(0.0, 9.05, 1.7, 0.8, Ink::Hair));
            parts.push(rect(-1.8, 6.2, 0.65, 2.9, Ink::Hair).round(0.12));
            parts.push(rect(1.15, 6.2, 0.65, 2.9, Ink::Hair).round(0.12));
        }
        2 => {
            // A bun on top.
            parts.extend(short_hair());
            parts.push(ellipse(0.0, 10.05, 0.72, 0.62, Ink::Hair));
        }
        3 => {
            // Curls all round.
            for (x, y) in [
                (-1.3, 8.6),
                (-0.7, 9.35),
                (0.0, 9.55),
                (0.7, 9.35),
                (1.3, 8.6),
            ] {
                parts.push(ellipse(x, y, 0.62, 0.58, Ink::Hair));
            }
        }
        4 => {
            // Cropped close.
            parts.push(ellipse(0.0, 9.15, 1.35, 0.42, Ink::Hair));
        }
        _ => {
            // A ponytail behind.
            parts.extend(short_hair());
            parts.push(ellipse(-1.9, 8.1, 0.5, 1.15, Ink::Hair));
        }
    }
    let hat_colour = Ink::Colour(HATS[(extra % HATS.len() as u32) as usize]);
    match hat {
        1 => {
            // A cap with a peak.
            parts.push(ellipse(0.0, 9.25, 1.6, 0.75, hat_colour));
            parts.push(
                rect(0.3, 8.95, 1.9, 0.32, hat_colour)
                    .round(0.1)
                    .tone(-0.15),
            );
        }
        2 => {
            // A woolly hat.
            parts.push(ellipse(0.0, 9.5, 1.55, 1.05, hat_colour));
            parts.push(
                rect(-1.65, 8.75, 3.3, 0.55, hat_colour)
                    .round(0.12)
                    .tone(-0.2),
            );
            parts.push(ellipse(0.0, 10.55, 0.35, 0.32, hat_colour).tone(0.3));
        }
        3 => {
            // A wide brim against the sun.
            parts.push(ellipse(0.0, 9.2, 2.7, 0.35, hat_colour));
            parts.push(ellipse(0.0, 9.55, 1.4, 0.75, hat_colour).tone(-0.1));
        }
        _ => {}
    }
    let dark = Ink::Colour(0x2a2522);
    match (extra / 7) % 4 {
        1 => {
            // Glasses.
            for side in [-1.0_f32, 1.0] {
                parts.push(ellipse(side * 0.55, 8.1, 0.36, 0.3, Ink::Colour(0xdfe8ee)).tone(0.2));
            }
            parts.push(line((-0.2, 8.15), (0.2, 8.15), 0.1, dark));
        }
        2 => parts.push(ellipse(0.0, 7.0, 1.0, 0.6, Ink::Hair)),
        3 => parts.push(rect(-1.3, 6.2, 2.6, 0.55, hat_colour).round(0.2).tone(0.1)),
        _ => {}
    }
    base.with(id, parts)
}

/// The colours hats and scarves come in.
const HATS: [u32; 6] = [0xb8433a, 0x2f5d8a, 0xd9a441, 0x3c7a55, 0x6e4a8a, 0x8a6a4a];

/// A small stable number for mixing, the same every run.
pub(crate) fn mix(value: u32) -> u32 {
    let mut x = value.wrapping_mul(0x9e37_79b9).wrapping_add(0x7f4a_7c15);
    x ^= x >> 15;
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^ (x >> 13)
}

/// How much lower a stance draws the whole figure, in heights: sitting
/// drops the hips to a seat.
pub fn drop_of(stance: Stance) -> f32 {
    match stance {
        Stance::Sitting => 0.1,
        _ => 0.0,
    }
}

/// Every drawing in every stance it changes in, side by side on a light
/// ground, as one picture: for looking a Pack's drawings over.
pub fn contact_sheet(
    drawings: &[Drawing],
    height: f32,
    inks: &dyn Fn(&Drawing, Ink) -> u32,
) -> String {
    let mut cells = Vec::new();
    for drawing in drawings {
        let stances = if drawing.parts.iter().any(|part| !part.stances.is_empty()) {
            Stance::ALL.to_vec()
        } else {
            vec![Stance::Standing]
        };
        for stance in stances {
            cells.push((drawing, stance));
        }
    }
    let gap = height * 0.25;
    let mut x = gap;
    let mut body = String::new();
    for (drawing, stance) in &cells {
        let svg = drawing.to_svg(*stance, height, &|ink| inks(drawing, ink));
        let inner = svg
            .split_once('>')
            .map(|(_, rest)| rest.trim_end_matches("</svg>"))
            .unwrap_or_default();
        let w = height * drawing.aspect;
        body.push_str(&format!(
            r#"<g transform="translate({x:.1},{gap:.1})">{inner}</g>"#
        ));
        x += w.max(height * 0.3) + gap;
    }
    let total_h = height + gap * 2.0;
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{x:.0}" height="{total_h:.0}"><rect width="100%" height="100%" fill="#e9efe6"/>{body}</svg>"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_people_have_outlines_and_faces_of_their_own() {
        let base = person_base("base");
        let outlines = (0..SILHOUETTES)
            .map(|variant| person(format!("p{variant}"), variant, &base).silhouette())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(outlines.len(), SILHOUETTES as usize);
        let someone = person("p", 5, &base);
        let faces = Mood::ALL
            .iter()
            .map(|mood| someone.face(*mood))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(faces.len(), Mood::ALL.len(), "every mood shows");
        for stance in Stance::ALL {
            assert!(someone.parts.iter().any(|part| !part.stances.is_empty()
                && part.stances.contains(&stance)
                || part.stances.is_empty()));
        }
        assert!(someone.is_drawable());
    }

    /// Writes every stance and mood of a few generated people to
    /// `WORLD_MACHINE_SHEET`, for looking over by eye.
    #[test]
    #[ignore]
    fn write_people_sheet() {
        let base = person_base("base");
        let colours = [
            0x3f6fb0_u32,
            0xc8553d,
            0x3c9a8f,
            0xe8b33c,
            0x7b4bb3,
            0x5b8c3a,
        ];
        let hairs = [
            0x2b1d14_u32,
            0x8a4b2a,
            0xd8b25a,
            0x1a1414,
            0x9a9a9a,
            0x5a3b22,
        ];
        let mut body = String::new();
        let (h, gap) = (160.0_f32, 30.0_f32);
        let mut y = gap;
        for row in 0..6u32 {
            let drawing = person(format!("p{row}"), row * 7 + 3, &base);
            let inks = |ink: Ink| match ink {
                Ink::Clothes | Ink::Wall => colours[row as usize],
                Ink::Hair => hairs[row as usize],
                Ink::Skin => 0xe0b18a,
                Ink::Colour(c) => c,
                _ => 0x888888,
            };
            let mut x = gap;
            let cells = Stance::ALL
                .iter()
                .map(|stance| (*stance, Mood::Content))
                .chain(
                    Mood::ALL
                        .iter()
                        .skip(1)
                        .map(|mood| (Stance::Standing, *mood)),
                );
            for (stance, mood) in cells {
                let svg = drawing.to_svg_feeling(stance, mood, h, &inks);
                let inner = svg
                    .split_once('>')
                    .map(|(_, rest)| rest.trim_end_matches("</svg>"))
                    .unwrap_or_default();
                body.push_str(&format!(
                    r#"<g transform="translate({x:.1},{y:.1})">{inner}</g>"#
                ));
                x += h * 0.75;
            }
            y += h + gap;
        }
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{:.0}" height="{y:.0}"><rect width="100%" height="100%" fill="#e9efe6"/>{body}</svg>"##,
            gap + 13.0 * h * 0.75
        );
        std::fs::write(std::env::var("WORLD_MACHINE_SHEET").unwrap(), svg).unwrap();
    }

    pub(crate) fn inks(ink: Ink) -> u32 {
        match ink {
            Ink::Colour(colour) => colour,
            Ink::Wall => 0xefe3cf,
            Ink::Roof => 0xb5523b,
            Ink::Trim => 0x6b4a33,
            Ink::Glass => 0x5f7385,
            Ink::Clothes => 0x3c9a8f,
            Ink::Hair => 0x5a3b22,
            Ink::Skin => 0xf0c7a2,
            Ink::Shade => 0x000000,
        }
    }

    #[test]
    fn a_person_can_be_drawn_in_every_stance_and_shows_what_each_one_is() {
        let person = person_base("someone").with("someone", short_hair());
        assert!(person.is_drawable());
        let parts = |stance| {
            person
                .parts
                .iter()
                .filter(|part| part.shows_in(stance))
                .count()
        };
        for stance in Stance::ALL {
            assert!(parts(stance) >= 12, "{stance:?}");
            assert!(person.to_svg(stance, 100.0, &inks).contains("<ellipse"));
        }
        let hands_up = person
            .parts
            .iter()
            .filter(|part| part.shows_in(Stance::Celebrating) && !part.shows_in(Stance::Standing))
            .count();
        assert!(hands_up >= 4);
    }

    #[test]
    fn a_drawing_that_strays_or_runs_on_is_not_drawn() {
        let mut far = person_base("far");
        far.parts
            .push(DrawPart::rect(0.0, 5.0, 0.1, 0.1, Ink::Wall));
        assert!(!far.is_drawable());
        let long = Drawing::new(
            "long",
            1.0,
            vec![DrawPart::rect(0.0, 0.0, 0.1, 0.1, Ink::Wall); Drawing::MOST_PARTS + 1],
        );
        assert!(!long.is_drawable());
        assert!(
            !Drawing::new("", 1.0, vec![DrawPart::rect(0.0, 0.0, 0.1, 0.1, Ink::Wall)])
                .is_drawable()
        );
        assert_eq!(toned(0x808080, 1.0), 0xffffff);
        assert_eq!(toned(0x808080, -1.0), 0x000000);
        assert_eq!(
            Ink::from_id(&Ink::Colour(0x12ab34).id()),
            Some(Ink::Colour(0x12ab34))
        );
    }

    /// Writes a contact sheet of the base person to `WORLD_MACHINE_SHEET`.
    #[test]
    #[ignore]
    fn write_person_sheet() {
        let person = person_base("someone").with("someone", short_hair());
        let sheet = contact_sheet(&[person], 240.0, &|_, ink| inks(ink));
        std::fs::write(std::env::var("WORLD_MACHINE_SHEET").unwrap(), sheet).unwrap();
    }
}
