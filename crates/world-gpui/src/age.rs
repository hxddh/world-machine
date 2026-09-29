//! People at every age: a baby carried in someone's arms or asleep in a
//! pram, a child small with a head too big for them, a lanky teenager, and
//! an elder gone grey, a little stooped, with a cane.
//!
//! Any drawing of a grown person (the app's own figure or a Pack's) is
//! drawn at another age by a warp laid over it: the legs, the body and
//! the head are each scaled on their own, and an elder's back bends
//! forward from the hips. The drawing is never redrawn for an age, so a
//! child's hat, hair and clothes are their own, just smaller; and since
//! the warp is continuous, nothing comes apart at the neck or the hips.
//!
//! Presentation only: the age a World records is read from the Pack's
//! `Look`, and nothing here is read back.

use crate::art::{self, Figure, Inks, Pose};
use crate::brush::{Brush, Shape, Xform};
use gpui::Hsla;
use world_projection::{Drawing, Look, Mood, Stance};

/// How old someone looks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Age {
    Baby,
    Child,
    Teen,
    #[default]
    Adult,
    Elder,
}

impl Age {
    /// The age a Pack's hints give someone: grown up when it says nothing.
    pub fn of(look: &Look) -> Self {
        look_age(look)
    }

    /// How tall someone of this age stands, as a share of a grown person.
    pub fn height(self) -> f32 {
        self.proportions(false).height()
    }

    /// How much slower than a grown person they walk: an elder takes their
    /// time.
    pub fn pace(self) -> f32 {
        match self {
            Age::Elder => 0.68,
            Age::Child => 1.1,
            _ => 1.0,
        }
    }

    /// How someone of this age is built, as multiples of a grown person's
    /// legs, body and head, their width, and how far their back bends.
    pub fn proportions(self, stoop: bool) -> Proportions {
        let (legs, body, head, width) = match self {
            Age::Baby => (0.26, 0.36, 0.62, 0.52),
            Age::Child => (0.5, 0.55, 0.84, 0.66),
            Age::Teen => (1.06, 0.9, 0.88, 0.76),
            Age::Adult => (1.0, 1.0, 1.0, 1.0),
            Age::Elder => (0.97, 0.95, 1.0, 1.02),
        };
        Proportions {
            legs,
            body,
            head,
            width,
            stoop: match (self, stoop) {
                (Age::Elder, true) => 0.15,
                (Age::Elder, false) => 0.09,
                (_, true) => 0.1,
                _ => 0.0,
            },
        }
    }
}

/// Where a grown person's hips and neck are, as shares of their height,
/// in every drawing of someone the app and the Packs make.
pub const HIPS: f32 = 0.31;
pub const NECK: f32 = 0.655;

/// How someone is built for their age: each part as a multiple of a
/// grown person's, and how far the back bends forward from the hips.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Proportions {
    pub legs: f32,
    pub body: f32,
    pub head: f32,
    pub width: f32,
    /// How far the top of them leans forward, per unit of height above the
    /// hips.
    pub stoop: f32,
}

impl Proportions {
    /// How tall they stand, as a share of a grown person.
    pub fn height(&self) -> f32 {
        HIPS * self.legs + (NECK - HIPS) * self.body + (1.0 - NECK) * self.head
    }

    /// Whether this is a grown person as drawn.
    pub fn plain(&self) -> bool {
        (self.legs - 1.0).abs() < 1e-4
            && (self.body - 1.0).abs() < 1e-4
            && (self.head - 1.0).abs() < 1e-4
            && (self.width - 1.0).abs() < 1e-4
            && self.stoop.abs() < 1e-4
    }

    /// Where a height `t` up a grown person (0 the feet, 1 the crown)
    /// lands on someone of this build, as a share of their own height.
    pub fn up(&self, t: f32) -> f32 {
        let at = if t <= HIPS {
            t * self.legs
        } else if t <= NECK {
            HIPS * self.legs + (t - HIPS) * self.body
        } else {
            HIPS * self.legs + (NECK - HIPS) * self.body + (t - NECK) * self.head
        };
        at / self.height()
    }

    /// How wide what is at height `t` of a grown person is drawn, as a
    /// multiple of its width in their own height: the head as the head
    /// scales, the rest as the body does, blended across the neck.
    pub fn across(&self, t: f32) -> f32 {
        let blend = ((t - (NECK - 0.04)) / 0.08).clamp(0.0, 1.0);
        (self.width + (self.head - self.width) * blend) / self.height()
    }

    /// How far forward what is at height `t` leans, as a share of their
    /// height.
    pub fn lean(&self, t: f32) -> f32 {
        let above = (t - HIPS).max(0.0);
        // A bend that grows toward the shoulders: a curve, not a slope.
        self.stoop * above * (0.6 + 0.8 * above)
    }
}

/// A brush that draws a grown person as someone else's age: set it over
/// the figure standing with its feet at (`x`, `y`), `height` tall as
/// drawn, facing `facing`.
pub struct Aged<'a> {
    pub inner: &'a mut dyn Brush,
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub facing: f32,
    pub build: Proportions,
}

impl Aged<'_> {
    fn t(&self, y: f32) -> f32 {
        (self.y - y) / self.height.max(1e-3)
    }

    fn map(&self, x: f32, y: f32) -> (f32, f32) {
        let t = self.t(y);
        let b = &self.build;
        (
            self.x + (x - self.x) * b.across(t) + self.facing * b.lean(t) * self.height,
            self.y - b.up(t) * self.height,
        )
    }

    /// How much a length at height `t` is scaled.
    fn scale(&self, t: f32) -> f32 {
        self.build.across(t)
    }
}

impl Brush for Aged<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let (top, bottom) = (self.t(y), self.t(y + h));
        let even = (self.scale(top) - self.scale(bottom)).abs() < 1e-3
            && (self.build.lean(top) - self.build.lean(bottom)).abs() < 1e-3;
        if even {
            let (x0, y0) = self.map(x, y);
            let (x1, y1) = self.map(x + w, y + h);
            let scale = self.scale((top + bottom) / 2.0);
            self.inner.rect(
                x0.min(x1),
                y0.min(y1),
                (x1 - x0).abs(),
                (y1 - y0).abs(),
                radius * scale,
                colour,
            );
        } else {
            let shape = Shape::rounded(x, y, w, h, radius).mapped(|x, y| self.map(x, y));
            self.inner.fill(&shape, colour);
        }
    }

    fn fill(&mut self, shape: &Shape, colour: Hsla) {
        let shape = shape.mapped(|x, y| self.map(x, y));
        self.inner.fill(&shape, colour);
    }

    fn stroke(&mut self, shape: &Shape, width: f32, colour: Hsla) {
        // A line is as thick as the part it is on.
        let t = shape
            .segments
            .iter()
            .find_map(|segment| match *segment {
                crate::brush::Segment::Move(_, y) | crate::brush::Segment::Line(_, y) => {
                    Some(self.t(y))
                }
                _ => None,
            })
            .unwrap_or(0.5);
        let scale = self.scale(t);
        let shape = shape.mapped(|x, y| self.map(x, y));
        self.inner.stroke(&shape, width * scale, colour);
    }

    fn soft(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, blur: f32, colour: Hsla) {
        let t = self.t(cy);
        let (x, y) = self.map(cx, cy);
        let scale = self.scale(t);
        self.inner
            .soft(x, y, rx * scale, ry * scale, blur * scale, colour);
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
        let (x0, y0) = self.map(x, y);
        let (x1, y1) = self.map(x + w, y + h);
        self.inner.gradient(
            x0.min(x1),
            y0.min(y1),
            (x1 - x0).abs(),
            (y1 - y0).abs(),
            angle,
            from,
            to,
        );
    }
}

/// Someone standing with their feet at (`x`, `y`), `height` tall at their
/// age (a grown person's height times [`Age::height`]), in their Pack's
/// drawing when it ships one, else the app's figure: a baby in a pram, a
/// child, a teenager, a grown person, or an elder with a cane.
#[allow(clippy::too_many_arguments)]
pub fn paint_person(
    window: &mut dyn Brush,
    x: f32,
    y: f32,
    height: f32,
    figure: &Figure,
    drawing: Option<&Drawing>,
    stance: Stance,
    mood: Mood,
    pose: Pose,
) {
    paint_aged(
        window, x, y, height, figure, drawing, stance, mood, pose, true,
    );
}

/// [`paint_person`], with or without an elder's cane: a portrait, cut off
/// at the chest, has no room for one.
#[allow(clippy::too_many_arguments)]
pub fn paint_aged(
    window: &mut dyn Brush,
    x: f32,
    y: f32,
    height: f32,
    figure: &Figure,
    drawing: Option<&Drawing>,
    stance: Stance,
    mood: Mood,
    pose: Pose,
    with_cane: bool,
) {
    if figure.age == Age::Baby && !figure.bird {
        let grown = height / Age::Baby.height().max(0.1);
        paint_pram(window, x, y, grown, figure, pose);
        return;
    }
    let build = figure.age.proportions(figure.stoop);
    if (pose.squash - 1.0).abs() >= 1e-3 || pose.lean.abs() > 1e-3 {
        let mut posed = Xform::about(
            window,
            (x, y),
            1.0 / pose.squash.max(0.2).sqrt(),
            pose.squash,
            pose.lean,
        );
        let upright = Pose {
            squash: 1.0,
            lean: 0.0,
            ..pose
        };
        paint_aged(
            &mut posed, x, y, height, figure, drawing, stance, mood, upright, with_cane,
        );
        return;
    }
    let facing = if pose.facing < 0.0 { -1.0 } else { 1.0 };
    let cane = with_cane && figure.age == Age::Elder && holds_a_cane(stance) && !figure.bird;
    if cane {
        paint_cane(window, x, y, height, facing, pose, build);
    }
    let mut aged = Aged {
        inner: window,
        x,
        y,
        height,
        facing,
        build,
    };
    let brush: &mut dyn Brush = if build.plain() || figure.bird {
        aged.inner
    } else {
        &mut aged
    };
    match drawing {
        Some(drawing) => art::paint_drawing_posed(
            brush,
            x,
            y,
            height * drawing.aspect,
            height,
            drawing,
            &Inks::of_person(figure),
            stance,
            mood,
            pose,
        ),
        None => art::paint_figure(brush, x, y, height, figure, pose),
    }
}

/// Whether someone standing like this has a hand free for a cane.
pub fn holds_a_cane(stance: Stance) -> bool {
    matches!(
        stance,
        Stance::Standing
            | Stance::Walking
            | Stance::Talking
            | Stance::LookingAround
            | Stance::Waving
    )
}

/// An elder's cane, planted a little ahead of them and swinging with
/// their step, its crook at their hand.
pub fn paint_cane(
    window: &mut dyn Brush,
    x: f32,
    y: f32,
    height: f32,
    facing: f32,
    pose: Pose,
    build: Proportions,
) {
    let swing = pose
        .stride
        .map(|phase| (phase * std::f32::consts::TAU).sin())
        .unwrap_or(0.0);
    let hand_t = 0.345;
    let lean = build.lean(hand_t) * height;
    let hand = (
        x + facing * (0.2 * height + lean),
        y - build.up(hand_t) * height - pose.bob,
    );
    let foot = (
        x + facing * (0.27 * height + swing * 0.05 * height),
        y - height * 0.005,
    );
    let wood = art::hex(0x6b4a32);
    let thick = (height * 0.026).max(1.0);
    art::line(window, foot, hand, thick, wood);
    // The crook, curling forward over the hand.
    let mut crook = Shape::new();
    crook.move_p(hand).curve_to(
        hand.0 + facing * height * 0.06,
        hand.1 + height * 0.02,
        hand.0 + facing * height * 0.04,
        hand.1 - height * 0.045,
    );
    window.stroke(&crook, thick, wood);
    // A rubber foot.
    art::circle(window, foot.0, foot.1, thick * 0.7, art::hex(0x2a2522));
}

/// A baby asleep in a pram standing on the ground at (`x`, `y`), beside
/// someone `grown` tall: a hooded carriage on two big wheels, the baby's
/// face and a blanket in their colours peeping out.
pub fn paint_pram(window: &mut dyn Brush, x: f32, y: f32, grown: f32, figure: &Figure, pose: Pose) {
    let u = grown / 10.0;
    let rock = pose.bob * 0.4;
    let body = art::shade(figure.clothes, -0.45);
    let trim = art::shade(figure.clothes, -0.62);
    let wheel = art::hex(0x2d2a28);
    // Wheels, spoked, and the chassis between them.
    for side in [-1.0_f32, 1.0] {
        let (cx, cy) = (x + side * 1.35 * u, y - 0.75 * u);
        art::circle(window, cx, cy, 0.75 * u, wheel);
        art::circle(window, cx, cy, 0.52 * u, art::hex(0xe9e2d6));
        art::circle(window, cx, cy, 0.14 * u, wheel);
        art::line(window, (cx, cy), (x, y - 1.8 * u), 0.14 * u, trim);
    }
    // The handle, reaching back.
    art::line(
        window,
        (x - 1.9 * u, y - 2.4 * u - rock),
        (x - 2.7 * u, y - 4.2 * u - rock),
        0.2 * u,
        trim,
    );
    art::rect(
        window,
        x - 3.05 * u,
        y - 4.35 * u - rock,
        0.8 * u,
        0.3 * u,
        0.15 * u,
        wheel,
    );
    // The tub: a deep rounded boat of a body.
    let top = y - 3.7 * u - rock;
    let mut tub = Shape::new();
    tub.move_to(x - 2.1 * u, top)
        .line_to(x + 2.1 * u, top)
        .curve_to(x + 1.2 * u, y - 1.6 * u - rock, x + 2.1 * u, y - 1.8 * u)
        .line_to(x - 1.2 * u, y - 1.6 * u - rock)
        .curve_to(x - 2.1 * u, top, x - 2.1 * u, y - 1.8 * u)
        .close();
    window.fill(&tub, body);
    art::rect(
        window,
        x - 2.2 * u,
        top - 0.1 * u,
        4.4 * u,
        0.3 * u,
        0.15 * u,
        trim,
    );
    // The hood, folded back over the head end.
    let mut hood = Shape::new();
    hood.move_to(x + 0.1 * u, top)
        .curve_to(x + 2.1 * u, top, x + 1.9 * u, top - 2.4 * u)
        .close();
    window.fill(&hood, art::shade(body, -0.12));
    for rib in [0.45_f32, 0.75] {
        let mut line = Shape::new();
        line.move_to(x + 0.1 * u + (2.0 * u) * rib * 0.15, top)
            .curve_to(
                x + 2.1 * u,
                top - 0.02 * u,
                x + 0.3 * u + 1.6 * u * rib,
                top - 2.0 * u * rib,
            );
        window.stroke(&line, 0.08 * u, trim);
    }
    // The baby, and a blanket in their colours tucked over them.
    let face = (x + 0.55 * u, top - 0.45 * u);
    art::circle(window, face.0, face.1, 0.62 * u, figure.skin);
    art::ellipse(
        window,
        face.0 - 0.1 * u,
        face.1 - 0.45 * u,
        0.5 * u,
        0.22 * u,
        figure.hair,
    );
    art::rect(
        window,
        x - 1.8 * u,
        top - 0.25 * u,
        2.2 * u,
        0.55 * u,
        0.27 * u,
        art::shade(figure.clothes, 0.35),
    );
    // Asleep: two closed eyes.
    for side in [-1.0_f32, 1.0] {
        art::line(
            window,
            (face.0 + side * 0.25 * u - 0.1 * u, face.1 + 0.05 * u),
            (face.0 + side * 0.25 * u + 0.1 * u, face.1 + 0.05 * u),
            0.07 * u,
            art::hex(0x2a2522),
        );
    }
}

/// A baby carried in someone's arms: a bundle in a blanket of their
/// colours at the chest of `holder`, `height` tall as a grown person,
/// standing at (`x`, `y`) facing `facing`: the baby's face at the crook of
/// one arm, the other arm under the bundle.
#[allow(clippy::too_many_arguments)]
pub fn paint_bundle(
    window: &mut dyn Brush,
    x: f32,
    y: f32,
    height: f32,
    facing: f32,
    bob: f32,
    baby: &Figure,
    holder: &Figure,
) {
    let f = if facing < 0.0 { -1.0 } else { 1.0 };
    let u = height / 10.0;
    let (cx, cy) = (x + f * 0.25 * u, y - 4.7 * u - bob);
    let blanket = art::shade(baby.clothes, 0.4);
    art::ellipse(
        window,
        cx,
        cy,
        1.75 * u,
        0.95 * u,
        art::shade(blanket, -0.14),
    );
    art::ellipse(
        window,
        cx - f * 0.12 * u,
        cy - 0.1 * u,
        1.55 * u,
        0.78 * u,
        blanket,
    );
    // A fold of the blanket round the face.
    let face = (cx + f * 0.95 * u, cy - 0.3 * u);
    art::circle(window, face.0, face.1, 0.78 * u, art::shade(blanket, -0.06));
    art::circle(window, face.0, face.1, 0.58 * u, baby.skin);
    art::ellipse(
        window,
        face.0 + f * 0.05 * u,
        face.1 - 0.42 * u,
        0.4 * u,
        0.17 * u,
        baby.hair,
    );
    for side in [-1.0_f32, 1.0] {
        art::line(
            window,
            (face.0 + side * 0.22 * u - 0.09 * u, face.1 + 0.02 * u),
            (face.0 + side * 0.22 * u + 0.09 * u, face.1 + 0.02 * u),
            0.07 * u,
            art::hex(0x2a2522),
        );
    }
    // The arm under the bundle, and a hand on it.
    let sleeve = art::shade(holder.clothes, -0.15);
    art::rect(
        window,
        cx - 1.5 * u,
        cy + 0.35 * u,
        2.6 * u,
        0.75 * u,
        0.37 * u,
        sleeve,
    );
    art::circle(
        window,
        cx + f * 1.15 * u,
        cy + 0.7 * u,
        0.42 * u,
        holder.skin,
    );
    art::circle(
        window,
        cx - f * 0.9 * u,
        cy - 0.35 * u,
        0.4 * u,
        holder.skin,
    );
}

/// Someone's head and shoulders at their age, filling a frame at
/// (`x`, `y`), `w` by `h`: drawn so their head is as big in the frame
/// whatever their age, and so a child's narrow shoulders, a teenager's
/// long neck or an elder's grey hair and bent back show.
#[allow(clippy::too_many_arguments)]
pub fn paint_bust(
    window: &mut dyn Brush,
    (x, y, w, h): (f32, f32, f32, f32),
    grown: f32,
    feet: f32,
    figure: &Figure,
    drawing: Option<&Drawing>,
    stance: Stance,
    mood: Mood,
) {
    if figure.age == Age::Baby && !figure.bird {
        paint_baby_face(window, (x, y, w, h), figure);
        return;
    }
    let build = figure.age.proportions(false);
    // A grown person would be drawn `grown` tall with their feet at
    // `feet`, below the frame; someone else is drawn at the height that
    // makes their head the same size, with their crown in the same place.
    let (tall, top) = if figure.bird {
        (grown, 0.955)
    } else {
        (grown * build.height() / build.head, build.up(0.955))
    };
    let crown = feet - grown * 0.955;
    let feet = crown + tall * top;
    let back = build.lean(0.955) * tall;
    paint_aged(
        window,
        x + w / 2.0 - back,
        feet,
        tall,
        figure,
        drawing,
        stance,
        mood,
        Pose::default(),
        false,
    );
}

/// A baby's face close up, wrapped in a blanket of their colours.
fn paint_baby_face(window: &mut dyn Brush, (x, y, w, h): (f32, f32, f32, f32), figure: &Figure) {
    let (cx, cy) = (x + w / 2.0, y + h * 0.5);
    let r = w.min(h) * 0.27;
    let blanket = art::shade(figure.clothes, 0.35);
    art::ellipse(
        window,
        cx,
        y + h * 1.02,
        w * 0.46,
        h * 0.36,
        art::shade(blanket, -0.1),
    );
    art::ellipse(window, cx, cy + r * 0.1, r * 1.45, r * 1.4, blanket);
    art::circle(window, cx, cy, r, figure.skin);
    art::ellipse(
        window,
        cx - r * 0.1,
        cy - r * 0.78,
        r * 0.55,
        r * 0.28,
        figure.hair,
    );
    let dark = art::hex(0x2a2522);
    for side in [-1.0_f32, 1.0] {
        art::circle(window, cx + side * r * 0.36, cy + r * 0.02, r * 0.09, dark);
        art::ellipse(
            window,
            cx + side * r * 0.62,
            cy + r * 0.35,
            r * 0.18,
            r * 0.1,
            art::hex(0xe89a8a),
        );
    }
    art::ellipse(
        window,
        cx,
        cy + r * 0.45,
        r * 0.14,
        r * 0.07,
        art::hex(0x7a3a2a),
    );
}

/// The age a Pack gave someone in their look, if it gives ages.
fn look_age(look: &Look) -> Age {
    use world_projection::AgeStage;
    match look.age.unwrap_or_default() {
        AgeStage::Baby => Age::Baby,
        AgeStage::Child => Age::Child,
        AgeStage::Teen => Age::Teen,
        AgeStage::Adult => Age::Adult,
        AgeStage::Elder => Age::Elder,
    }
}

/// Whether a Pack says someone's hair has gone grey.
pub(crate) fn look_grey(look: &Look) -> bool {
    look.grey
}

/// Whether a Pack says someone walks with a stoop.
pub(crate) fn look_stoop(look: &Look) -> bool {
    look.stoop
}

/// Hair gone grey: most of the colour gone to silver, a trace of what it
/// was left in it.
pub fn greyed(hair: Hsla) -> Hsla {
    let silver = art::hex(0xd8d4cc);
    Hsla {
        h: hair.h,
        s: hair.s * 0.15,
        l: hair.l * 0.2 + silver.l * 0.8,
        a: hair.a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_age_stands_its_own_height_and_the_young_have_bigger_heads() {
        let share = |age: Age| {
            let build = age.proportions(false);
            (build.height(), (1.0 - build.up(NECK)))
        };
        let (child, child_head) = share(Age::Child);
        let (teen, _) = share(Age::Teen);
        let (adult, adult_head) = share(Age::Adult);
        let (elder, _) = share(Age::Elder);
        let (baby, _) = share(Age::Baby);
        assert!((adult - 1.0).abs() < 1e-4);
        assert!(
            baby < child && child < teen && teen < adult,
            "{baby} {child} {teen}"
        );
        assert!(elder < adult && elder > teen - 0.05);
        assert!(child < 0.7, "a child is small: {child}");
        assert!(
            child_head > adult_head * 1.2,
            "a child's head is a bigger share of them: {child_head} vs {adult_head}"
        );
        // A teenager is lanky: longer in the leg for their height and
        // narrower than a grown person.
        let build = Age::Teen.proportions(false);
        assert!(build.up(HIPS) > HIPS + 0.02);
        assert!(build.width < 0.85);
    }

    #[test]
    fn the_warp_is_continuous_and_keeps_the_feet_and_crown() {
        for age in [Age::Baby, Age::Child, Age::Teen, Age::Adult, Age::Elder] {
            let build = age.proportions(age == Age::Elder);
            assert!(build.up(0.0).abs() < 1e-5);
            assert!((build.up(1.0) - 1.0).abs() < 1e-4, "{age:?}");
            for t in [HIPS, NECK] {
                assert!((build.up(t - 1e-4) - build.up(t + 1e-4)).abs() < 1e-3);
            }
            let mut last = 0.0;
            for step in 0..=100 {
                let t = step as f32 / 100.0;
                assert!(build.up(t) >= last);
                last = build.up(t);
            }
        }
    }

    #[test]
    fn an_elder_stoops_forward_and_walks_slower() {
        let build = Age::Elder.proportions(true);
        assert_eq!(build.lean(0.1), 0.0, "the legs stand straight");
        assert!(build.lean(0.95) > 0.05, "the shoulders come forward");
        assert!(Age::Elder.pace() < 0.8);
        assert!(Age::Adult.proportions(false).plain());
    }

    #[test]
    fn grey_hair_keeps_little_of_its_colour() {
        let hair = art::hex(0x8a4b2a);
        let grey = greyed(hair);
        assert!(grey.s < hair.s * 0.3);
        assert!(grey.l > 0.7);
    }
}
