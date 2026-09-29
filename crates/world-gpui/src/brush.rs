//! What the art is drawn with: GPUI's window, for what moves every frame,
//! or the CPU painter's canvas, for the layers that stand still. The same
//! drawing code paints on both, so a building looks the same whether it
//! is painted once into an image or drawn live while it springs.

use gpui::{
    point, px, quad, size, BorderStyle, Bounds, BoxShadow, Corners, Hsla, RenderImage, Window,
};
use std::f32::consts::FRAC_1_SQRT_2;
use std::sync::Arc;

/// A path in drawing coordinates: moves, straight lines, quadratic curves
/// (to a point, bending toward a control point, as GPUI's `curve_to`
/// takes them) and closes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shape {
    pub(crate) segments: Vec<Segment>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Segment {
    Move(f32, f32),
    Line(f32, f32),
    /// To (x, y), bending toward the control point (cx, cy).
    Curve(f32, f32, f32, f32),
    Close,
}

impl Shape {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.segments.push(Segment::Move(x, y));
        self
    }

    pub fn line_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.segments.push(Segment::Line(x, y));
        self
    }

    /// A curve to (`x`, `y`) bending toward (`cx`, `cy`).
    pub fn curve_to(&mut self, x: f32, y: f32, cx: f32, cy: f32) -> &mut Self {
        self.segments.push(Segment::Curve(x, y, cx, cy));
        self
    }

    pub fn move_p(&mut self, (x, y): (f32, f32)) -> &mut Self {
        self.move_to(x, y)
    }

    pub fn line_p(&mut self, (x, y): (f32, f32)) -> &mut Self {
        self.line_to(x, y)
    }

    pub fn curve_p(&mut self, (x, y): (f32, f32), (cx, cy): (f32, f32)) -> &mut Self {
        self.curve_to(x, y, cx, cy)
    }

    pub fn close(&mut self) -> &mut Self {
        self.segments.push(Segment::Close);
        self
    }

    /// A closed polygon through `points`.
    pub fn polygon(points: &[(f32, f32)]) -> Self {
        let mut shape = Self::new();
        if let Some((x, y)) = points.first() {
            shape.move_to(*x, *y);
            for (x, y) in &points[1..] {
                shape.line_to(*x, *y);
            }
            shape.close();
        }
        shape
    }

    /// An ellipse, as eight curved segments.
    pub fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        let step = std::f32::consts::TAU / 8.0;
        let reach = 1.0 / (step / 2.0).cos();
        let at =
            |angle: f32, scale: f32| (cx + rx * scale * angle.cos(), cy + ry * scale * angle.sin());
        let mut shape = Self::new();
        let (x, y) = at(0.0, 1.0);
        shape.move_to(x, y);
        for index in 0..8 {
            let start = index as f32 * step;
            let (x, y) = at(start + step, 1.0);
            let (cx, cy) = at(start + step / 2.0, reach);
            shape.curve_to(x, y, cx, cy);
        }
        shape.close();
        shape
    }

    /// A rectangle with rounded corners, the corners as curves.
    pub fn rounded(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Self {
        let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
        let mut shape = Self::new();
        if r <= 0.01 {
            return Self::polygon(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)]);
        }
        // Each corner as two quadratic halves, which keeps a circle round.
        let k = 0.4142 * r;
        shape.move_to(x + r, y);
        shape.line_to(x + w - r, y);
        shape.curve_to(
            x + w - r + r * FRAC_1_SQRT_2,
            y + r - r * FRAC_1_SQRT_2,
            x + w - r + k,
            y,
        );
        shape.curve_to(x + w, y + r, x + w, y + r - k);
        shape.line_to(x + w, y + h - r);
        shape.curve_to(
            x + w - r + r * FRAC_1_SQRT_2,
            y + h - r + r * FRAC_1_SQRT_2,
            x + w,
            y + h - r + k,
        );
        shape.curve_to(x + w - r, y + h, x + w - r + k, y + h);
        shape.line_to(x + r, y + h);
        shape.curve_to(
            x + r - r * FRAC_1_SQRT_2,
            y + h - r + r * FRAC_1_SQRT_2,
            x + r - k,
            y + h,
        );
        shape.curve_to(x, y + h - r, x, y + h - r + k);
        shape.line_to(x, y + r);
        shape.curve_to(
            x + r - r * FRAC_1_SQRT_2,
            y + r - r * FRAC_1_SQRT_2,
            x,
            y + r - k,
        );
        shape.curve_to(x + r, y, x + r - k, y);
        shape.close();
        shape
    }

    /// The same shape with every point moved by `map`.
    pub fn mapped(&self, map: impl Fn(f32, f32) -> (f32, f32)) -> Self {
        let segments = self
            .segments
            .iter()
            .map(|segment| match *segment {
                Segment::Move(x, y) => {
                    let (x, y) = map(x, y);
                    Segment::Move(x, y)
                }
                Segment::Line(x, y) => {
                    let (x, y) = map(x, y);
                    Segment::Line(x, y)
                }
                Segment::Curve(x, y, cx, cy) => {
                    let (x, y) = map(x, y);
                    let (cx, cy) = map(cx, cy);
                    Segment::Curve(x, y, cx, cy)
                }
                Segment::Close => Segment::Close,
            })
            .collect();
        Self { segments }
    }

    fn gpui(&self, builder: &mut gpui::PathBuilder) {
        for segment in &self.segments {
            match *segment {
                Segment::Move(x, y) => builder.move_to(point(px(x), px(y))),
                Segment::Line(x, y) => builder.line_to(point(px(x), px(y))),
                Segment::Curve(x, y, cx, cy) => {
                    builder.curve_to(point(px(x), px(y)), point(px(cx), px(cy)))
                }
                Segment::Close => builder.close(),
            }
        }
    }
}

/// Something the art can be drawn on.
pub trait Brush {
    /// A rectangle with rounded corners.
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla);
    fn fill(&mut self, shape: &Shape, colour: Hsla);
    fn stroke(&mut self, shape: &Shape, width: f32, colour: Hsla);
    /// An ellipse with edges blurred by about `blur`: a contact shadow, a
    /// glow, a puff of smoke.
    fn soft(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, blur: f32, colour: Hsla);
    /// A rectangle filled with a linear gradient at `angle` degrees (as
    /// GPUI has them: 180 runs top to bottom), from `from` at `from_at` to
    /// `to` at `to_at`, as fractions along it.
    #[allow(clippy::too_many_arguments)]
    fn gradient(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        angle: f32,
        from: (Hsla, f32),
        to: (Hsla, f32),
    );
    /// A painted picture laid over `rect` (x, y, width, height), of which
    /// only the part inside `clip` shows: a design on a flag drawn a strip
    /// at a time. Only a window shows pictures; a brush that cannot skips
    /// them.
    fn picture(&mut self, _image: &Arc<RenderImage>, _rect: Rect, _clip: Rect) {}
}

/// A rectangle: x, y, width, height.
pub type Rect = (f32, f32, f32, f32);

impl Brush for Window {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        self.paint_quad(quad(
            Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
            px(radius.min(w / 2.0).min(h / 2.0).max(0.0)),
            colour,
            px(0.0),
            colour,
            BorderStyle::default(),
        ));
    }

    fn fill(&mut self, shape: &Shape, colour: Hsla) {
        let mut builder = gpui::PathBuilder::fill();
        shape.gpui(&mut builder);
        if let Ok(path) = builder.build() {
            self.paint_path(path, colour);
        }
    }

    fn stroke(&mut self, shape: &Shape, width: f32, colour: Hsla) {
        let mut builder = gpui::PathBuilder::stroke(px(width.max(0.1)));
        shape.gpui(&mut builder);
        if let Ok(path) = builder.build() {
            self.paint_path(path, colour);
        }
    }

    fn soft(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, blur: f32, colour: Hsla) {
        if rx <= 0.0 || ry <= 0.0 {
            return;
        }
        // GPUI's one blur is a shadow's: a pill that blurs into an ellipse.
        let blur = blur.max(0.5);
        let (w, h) = ((rx * 2.0 - blur).max(1.0), (ry * 2.0 - blur).max(1.0));
        self.paint_drop_shadows(
            Bounds::new(
                point(px(cx - w / 2.0), px(cy - h / 2.0)),
                size(px(w), px(h)),
            ),
            Corners::all(px(w.min(h) / 2.0)),
            &[BoxShadow {
                color: colour,
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(blur),
                spread_radius: px(0.0),
                inset: false,
            }],
        );
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
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        self.paint_quad(gpui::fill(
            Bounds::new(point(px(x), px(y)), size(px(w), px(h))),
            gpui::linear_gradient(
                angle,
                gpui::linear_color_stop(from.0, from.1),
                gpui::linear_color_stop(to.0, to.1),
            ),
        ));
    }

    fn picture(&mut self, image: &Arc<RenderImage>, rect: Rect, clip: Rect) {
        let bounds = |(x, y, w, h): Rect| Bounds::new(point(px(x), px(y)), size(px(w), px(h)));
        if rect.2 <= 0.0 || rect.3 <= 0.0 || clip.2 <= 0.0 || clip.3 <= 0.0 {
            return;
        }
        let _ = self.paint_image(
            bounds(clip).intersect(&bounds(rect)),
            bounds(rect),
            Corners::default(),
            image.clone(),
            0,
            false,
        );
    }
}

/// A brush that draws through another, every point moved first by an
/// affine map: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`. A figure squashed
/// as it lands, leaning as it walks, or a boat rocking on the swell.
pub struct Xform<'a> {
    pub inner: &'a mut dyn Brush,
    pub m: [f32; 6],
}

impl<'a> Xform<'a> {
    /// Squashes or stretches by `sy` (with `sx` the other way to keep the
    /// volume) and leans by `shear` (horizontal shift per unit of height
    /// above the pivot), all about the pivot (`px`, `py`), where the feet
    /// stand.
    pub fn about(
        inner: &'a mut dyn Brush,
        (px, py): (f32, f32),
        sx: f32,
        sy: f32,
        shear: f32,
    ) -> Self {
        // x' = px + sx·(x − px) − shear·sy·(y − py)
        // y' = py + sy·(y − py)
        let c = -shear * sy;
        Self {
            inner,
            m: [sx, 0.0, c, sy, px - sx * px - c * py, py - sy * py],
        }
    }

    /// Turned by `angle` radians about (`px`, `py`).
    pub fn turned(inner: &'a mut dyn Brush, (px, py): (f32, f32), angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            inner,
            m: [c, s, -s, c, px - c * px + s * py, py - s * px - c * py],
        }
    }

    fn map(&self, x: f32, y: f32) -> (f32, f32) {
        let [a, b, c, d, e, f] = self.m;
        (a * x + c * y + e, b * x + d * y + f)
    }

    fn straight(&self) -> bool {
        self.m[1].abs() < 1e-4 && self.m[2].abs() < 1e-4
    }
}

impl Brush for Xform<'_> {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour: Hsla) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        if self.straight() {
            let (x0, y0) = self.map(x, y);
            let (x1, y1) = self.map(x + w, y + h);
            let scale = self.m[0].abs().min(self.m[3].abs());
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
        let shape = shape.mapped(|x, y| self.map(x, y));
        let scale = (self.m[0] * self.m[3] - self.m[1] * self.m[2]).abs().sqrt();
        self.inner.stroke(&shape, width * scale, colour);
    }

    fn soft(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, blur: f32, colour: Hsla) {
        let (x, y) = self.map(cx, cy);
        self.inner.soft(
            x,
            y,
            rx * self.m[0].abs().max(0.01),
            ry * self.m[3].abs().max(0.01),
            blur,
            colour,
        );
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

    fn picture(&mut self, image: &Arc<RenderImage>, rect: Rect, clip: Rect) {
        // A picture cannot turn: it is moved and scaled with the rest, and
        // a turn only moves it.
        let map = |(x, y, w, h): Rect| {
            let (x0, y0) = self.map(x, y);
            let (x1, y1) = self.map(x + w, y + h);
            let (sx, sy) = (self.m[0].abs().max(0.01), self.m[3].abs().max(0.01));
            if self.straight() {
                (x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs())
            } else {
                (x0, y0, w * sx, h * sy)
            }
        };
        let (rect, clip) = (map(rect), map(clip));
        self.inner.picture(image, rect, clip);
    }
}

/// A brush that only counts what it is asked to draw: for measuring how
/// much a frame asks of the renderer.
#[derive(Default, Debug)]
pub struct Tally {
    pub quads: usize,
    pub paths: usize,
    pub soft: usize,
}

impl Brush for Tally {
    fn rect(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: Hsla) {
        self.quads += 1;
    }
    fn fill(&mut self, _: &Shape, _: Hsla) {
        self.paths += 1;
    }
    fn stroke(&mut self, _: &Shape, _: f32, _: Hsla) {
        self.paths += 1;
    }
    fn soft(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: Hsla) {
        self.soft += 1;
    }
    fn gradient(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: (Hsla, f32), _: (Hsla, f32)) {
        self.quads += 1;
    }
    fn picture(&mut self, _: &Arc<RenderImage>, _: Rect, _: Rect) {
        self.quads += 1;
    }
}
