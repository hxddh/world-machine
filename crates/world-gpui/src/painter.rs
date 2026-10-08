//! The CPU painter: the scene's still layers (the sky, the hills, the
//! ground and the buildings with their light) painted into images with
//! `tiny-skia`, and shown with GPUI's image primitive. People, speech and
//! whatever moves stay live GPUI drawing on top.
//!
//! Everything here is a pure function of what it is given: the same frame
//! at the same size paints the same pixels, on any machine with the same
//! floating point, so the golden pictures hold. Nothing reads a clock.
//!
//! The images are kept in a cache keyed by everything they depend on (the
//! hour in quarter hours, the weather, the season, the zoom, the size) and
//! painted again only when one of those changes.

use crate::brush::{Brush, Segment, Shape};
use gpui::{Hsla, RenderImage, Rgba};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tiny_skia as sk;

/// A pixmap to paint on, with the map from drawing coordinates to its
/// pixels: `scale` device pixels to a unit, and the drawing point
/// (`origin_x`, `origin_y`) at its top-left corner.
pub struct Canvas {
    pub pixmap: sk::Pixmap,
    pub scale: f32,
    pub origin: (f32, f32),
}

impl Canvas {
    pub fn new(width: u32, height: u32, scale: f32, origin: (f32, f32)) -> Option<Self> {
        Some(Self {
            pixmap: sk::Pixmap::new(width.max(1), height.max(1))?,
            scale,
            origin,
        })
    }

    fn transform(&self) -> sk::Transform {
        sk::Transform::from_row(
            self.scale,
            0.0,
            0.0,
            self.scale,
            -self.origin.0 * self.scale,
            -self.origin.1 * self.scale,
        )
    }

    /// A rectangle in drawing coordinates, cut to what lies on this canvas
    /// (tiny-skia's anti-aliased rectangles expect to lie on it).
    fn on_canvas(&self, x: f32, y: f32, w: f32, h: f32) -> Option<sk::Rect> {
        let (x0, y0) = self.origin;
        let x1 = x0 + self.pixmap.width() as f32 / self.scale;
        let y1 = y0 + self.pixmap.height() as f32 / self.scale;
        let (left, top) = (x.max(x0), y.max(y0));
        let (right, bottom) = ((x + w).min(x1), (y + h).min(y1));
        sk::Rect::from_ltrb(left, top, right, bottom)
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Where a drawing point lands, in this canvas's pixels.
    pub fn device(&self, x: f32, y: f32) -> (f32, f32) {
        (
            (x - self.origin.0) * self.scale,
            (y - self.origin.1) * self.scale,
        )
    }

    /// Lays `other` over this canvas with its top-left at the drawing point
    /// (`x`, `y`), both at the same scale.
    pub fn draw(&mut self, other: &sk::Pixmap, x: f32, y: f32, blend: sk::BlendMode) {
        let (dx, dy) = self.device(x, y);
        self.pixmap.draw_pixmap(
            dx.round() as i32,
            dy.round() as i32,
            other.as_ref(),
            &sk::PixmapPaint {
                blend_mode: blend,
                ..sk::PixmapPaint::default()
            },
            sk::Transform::identity(),
            None,
        );
    }

    /// Lays `other` (in device pixels) over this canvas through the map
    /// `transform`, from its pixels to this canvas's pixels.
    pub fn draw_mapped(&mut self, other: &sk::Pixmap, transform: sk::Transform, opacity: f32) {
        self.pixmap.draw_pixmap(
            0,
            0,
            other.as_ref(),
            &sk::PixmapPaint {
                opacity,
                quality: sk::FilterQuality::Bilinear,
                ..sk::PixmapPaint::default()
            },
            transform,
            None,
        );
    }
}

pub fn colour(colour: Hsla) -> sk::Color {
    let Rgba { r, g, b, a } = colour.into();
    sk::Color::from_rgba(
        r.clamp(0.0, 1.0),
        g.clamp(0.0, 1.0),
        b.clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    )
    .unwrap_or(sk::Color::TRANSPARENT)
}

fn path(shape: &Shape) -> Option<sk::Path> {
    let mut builder = sk::PathBuilder::new();
    let mut open = false;
    for segment in &shape.segments {
        match *segment {
            Segment::Move(x, y) => {
                builder.move_to(x, y);
                open = true;
            }
            Segment::Line(x, y) => {
                if !open {
                    builder.move_to(x, y);
                    open = true;
                } else {
                    builder.line_to(x, y);
                }
            }
            Segment::Curve(x, y, cx, cy) => {
                if !open {
                    builder.move_to(x, y);
                    open = true;
                } else {
                    builder.quad_to(cx, cy, x, y);
                }
            }
            Segment::Close => builder.close(),
        }
    }
    builder.finish()
}

fn solid(colour_: Hsla) -> sk::Paint<'static> {
    let mut paint = sk::Paint::default();
    paint.set_color(colour(colour_));
    paint.anti_alias = true;
    paint
}

impl Brush for Canvas {
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, colour_: Hsla) {
        if w <= 0.0 || h <= 0.0 || colour_.a <= 0.0 {
            return;
        }
        let transform = self.transform();
        // Tiny-skia's anti-aliased rectangles want at least a pixel or two
        // each way; anything thinner goes as a path.
        if radius * self.scale < 0.35 && w * self.scale >= 2.0 && h * self.scale >= 2.0 {
            if let Some(rect) = self.on_canvas(x, y, w, h) {
                self.pixmap
                    .fill_rect(rect, &solid(colour_), transform, None);
            }
            return;
        }
        if let Some(path) = path(&Shape::rounded(x, y, w, h, radius)) {
            self.pixmap.fill_path(
                &path,
                &solid(colour_),
                sk::FillRule::Winding,
                transform,
                None,
            );
        }
    }

    fn fill(&mut self, shape: &Shape, colour_: Hsla) {
        if colour_.a <= 0.0 {
            return;
        }
        if let Some(path) = path(shape) {
            self.pixmap.fill_path(
                &path,
                &solid(colour_),
                sk::FillRule::Winding,
                self.transform(),
                None,
            );
        }
    }

    fn stroke(&mut self, shape: &Shape, width: f32, colour_: Hsla) {
        if colour_.a <= 0.0 {
            return;
        }
        if let Some(path) = path(shape) {
            let stroke = sk::Stroke {
                width: width.max(0.05),
                ..sk::Stroke::default()
            };
            self.pixmap
                .stroke_path(&path, &solid(colour_), &stroke, self.transform(), None);
        }
    }

    fn soft(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, blur: f32, colour_: Hsla) {
        if rx <= 0.0 || ry <= 0.0 || colour_.a <= 0.0 {
            return;
        }
        let reach = blur.max(0.5) / 2.0;
        let (ox, oy) = (rx + reach, ry + reach);
        // Solid to where the blur begins, then a smooth fall to nothing.
        let inner = ((rx.min(ry) - reach) / (rx.min(ry) + reach)).clamp(0.0, 0.98);
        let c = colour(colour_);
        let at = |t: f32, k: f32| {
            let mut faded = c;
            faded.set_alpha(c.alpha() * k);
            sk::GradientStop::new(inner + (1.0 - inner) * t, faded)
        };
        let stops = vec![
            sk::GradientStop::new(0.0, c),
            at(0.0, 1.0),
            at(0.2, 0.9),
            at(0.4, 0.66),
            at(0.6, 0.36),
            at(0.8, 0.12),
            at(1.0, 0.0),
        ];
        let local = sk::Transform::from_row(ox, 0.0, 0.0, oy, cx, cy);
        let Some(shader) = sk::RadialGradient::new(
            sk::Point::from_xy(0.0, 0.0),
            sk::Point::from_xy(0.0, 0.0),
            1.0,
            stops,
            sk::SpreadMode::Pad,
            local,
        ) else {
            return;
        };
        // The gradient fades to nothing at its edge: no edge to smooth.
        let paint = sk::Paint {
            shader,
            anti_alias: false,
            ..sk::Paint::default()
        };
        if let Some(rect) = self.on_canvas(cx - ox, cy - oy, ox * 2.0, oy * 2.0) {
            self.pixmap.fill_rect(rect, &paint, self.transform(), None);
        }
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
        // As CSS and GPUI have it: 0 points up, 90 right, 180 down.
        let radians = angle.to_radians();
        let (dx, dy) = (radians.sin(), -radians.cos());
        let length = (w * dx).abs() + (h * dy).abs();
        let (cx, cy) = (x + w / 2.0, y + h / 2.0);
        let start = (cx - dx * length / 2.0, cy - dy * length / 2.0);
        let end = (cx + dx * length / 2.0, cy + dy * length / 2.0);
        let Some(shader) = sk::LinearGradient::new(
            sk::Point::from_xy(start.0, start.1),
            sk::Point::from_xy(end.0, end.1),
            vec![
                sk::GradientStop::new(from.1.clamp(0.0, 1.0), colour(from.0)),
                sk::GradientStop::new(to.1.clamp(0.0, 1.0), colour(to.0)),
            ],
            sk::SpreadMode::Pad,
            sk::Transform::identity(),
        ) else {
            return;
        };
        let paint = sk::Paint {
            shader,
            anti_alias: false,
            ..sk::Paint::default()
        };
        if let Some(rect) = self.on_canvas(x, y, w, h) {
            self.pixmap.fill_rect(rect, &paint, self.transform(), None);
        }
    }

    /// A picture (a design's cloth) laid over the canvas, scaled to `rect`
    /// and showing only within `clip`, as a window shows it: so a moment's
    /// panel, painted here, carries the designs the scene shows.
    fn picture(
        &mut self,
        image: &Arc<RenderImage>,
        rect: crate::brush::Rect,
        clip: crate::brush::Rect,
    ) {
        let (x, y, w, h) = rect;
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let Some(area) = sk::Rect::from_ltrb(
            x.max(clip.0),
            y.max(clip.1),
            (x + w).min(clip.0 + clip.2),
            (y + h).min(clip.1 + clip.3),
        ) else {
            return;
        };
        let Some(area) = self.on_canvas(area.x(), area.y(), area.width(), area.height()) else {
            return;
        };
        let transform = self.transform();
        with_pixmap_of(image, |pixmap| {
            let fit = sk::Transform::from_row(
                w / pixmap.width() as f32,
                0.0,
                0.0,
                h / pixmap.height() as f32,
                x,
                y,
            );
            let paint = sk::Paint {
                shader: sk::Pattern::new(
                    pixmap.as_ref(),
                    sk::SpreadMode::Pad,
                    sk::FilterQuality::Bilinear,
                    1.0,
                    fit,
                ),
                anti_alias: true,
                ..sk::Paint::default()
            };
            self.pixmap.fill_rect(area, &paint, transform, None);
        });
    }
}

/// Runs `paint` with `image` as a pixmap, made once for the latest image
/// asked for (a flag or a quilt is laid strip by strip from one picture).
fn with_pixmap_of(image: &Arc<RenderImage>, paint: impl FnOnce(&sk::Pixmap)) {
    thread_local! {
        static LAST: RefCell<Option<(Arc<RenderImage>, sk::Pixmap)>> = const { RefCell::new(None) };
    }
    LAST.with(|last| {
        let mut last = last.borrow_mut();
        let fresh = !matches!(&*last, Some((kept, _)) if Arc::ptr_eq(kept, image));
        if fresh {
            *last = pixmap_of(image).map(|pixmap| (image.clone(), pixmap));
        }
        if let Some((_, pixmap)) = &*last {
            paint(pixmap);
        }
    });
}

/// A GPUI image (straight BGRA) as a pixmap (premultiplied RGBA): the way
/// back from [`image_of`].
fn pixmap_of(image: &RenderImage) -> Option<sk::Pixmap> {
    let bytes = image.as_bytes(0)?;
    let size = image.size(0);
    let (width, height) = (size.width.0.max(0) as u32, size.height.0.max(0) as u32);
    let mut pixmap = sk::Pixmap::new(width, height)?;
    for (to, from) in pixmap.pixels_mut().iter_mut().zip(bytes.as_chunks::<4>().0) {
        let [b, g, r, a] = *from;
        *to = sk::ColorU8::from_rgba(r, g, b, a).premultiply();
    }
    Some(pixmap)
}

/// A gradient between any number of stops, filling a shape: a sky, a
/// band of hills fading into the air, water deepening toward the front.
pub fn fill_shaded(
    canvas: &mut Canvas,
    shape: &Shape,
    start: (f32, f32),
    end: (f32, f32),
    stops: &[(f32, Hsla)],
) {
    let Some(path) = path(shape) else {
        return;
    };
    let stops = stops
        .iter()
        .map(|(at, c)| sk::GradientStop::new(*at, colour(*c)))
        .collect::<Vec<_>>();
    let Some(shader) = sk::LinearGradient::new(
        sk::Point::from_xy(start.0, start.1),
        sk::Point::from_xy(end.0, end.1),
        stops,
        sk::SpreadMode::Pad,
        sk::Transform::identity(),
    ) else {
        return;
    };
    let paint = sk::Paint {
        shader,
        anti_alias: true,
        ..sk::Paint::default()
    };
    let transform = canvas.transform();
    canvas
        .pixmap
        .fill_path(&path, &paint, sk::FillRule::Winding, transform, None);
}

/// A soft light added over what is there (screen blending): a lamp's
/// halo, the low sun's glow on the sky.
pub fn glow(canvas: &mut Canvas, cx: f32, cy: f32, rx: f32, ry: f32, colour_: Hsla) {
    if rx <= 0.0 || ry <= 0.0 || colour_.a <= 0.0 {
        return;
    }
    let c = colour(colour_);
    let faded = |k: f32| {
        let mut faded = c;
        faded.set_alpha(c.alpha() * k);
        faded
    };
    let stops = vec![
        sk::GradientStop::new(0.0, c),
        sk::GradientStop::new(0.15, faded(0.72)),
        sk::GradientStop::new(0.35, faded(0.38)),
        sk::GradientStop::new(0.6, faded(0.14)),
        sk::GradientStop::new(1.0, faded(0.0)),
    ];
    let local = sk::Transform::from_row(rx, 0.0, 0.0, ry, cx, cy);
    let Some(shader) = sk::RadialGradient::new(
        sk::Point::from_xy(0.0, 0.0),
        sk::Point::from_xy(0.0, 0.0),
        1.0,
        stops,
        sk::SpreadMode::Pad,
        local,
    ) else {
        return;
    };
    let paint = sk::Paint {
        shader,
        blend_mode: sk::BlendMode::Screen,
        anti_alias: false,
        ..sk::Paint::default()
    };
    let transform = canvas.transform();
    if let Some(rect) = canvas.on_canvas(cx - rx, cy - ry, rx * 2.0, ry * 2.0) {
        canvas.pixmap.fill_rect(rect, &paint, transform, None);
    }
}

/// A small, fast, stable hash of two integers.
pub fn hash2(x: i32, y: i32, salt: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ salt;
    h ^= h >> 13;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 16;
    h
}

const GRAIN: usize = 256;
const WASH: usize = 384;

/// The paper: a fine tooth of grain and, much larger, the soft unevenness
/// of a wash, as multipliers around 1 in 1/1024ths. Two sizes that do not
/// divide each other, so neither repeats visibly.
fn paper() -> &'static (Vec<i16>, Vec<i16>) {
    static PAPER: OnceLock<(Vec<i16>, Vec<i16>)> = OnceLock::new();
    PAPER.get_or_init(|| {
        let grain = (0..GRAIN * GRAIN)
            .map(|index| {
                let (x, y) = ((index % GRAIN) as i32, (index / GRAIN) as i32);
                // Two octaves of white noise, the finer one softer: tooth,
                // not static.
                let fine = (hash2(x, y, 0x51ed) % 1000) as f32 / 1000.0 - 0.5;
                let coarse = (hash2(x / 2, y / 2, 0x2c1b) % 1000) as f32 / 1000.0 - 0.5;
                ((fine * 0.55 + coarse * 0.45) * 1024.0) as i16
            })
            .collect();
        // Value noise on a coarse lattice, smoothly interpolated.
        let cell = 48usize;
        let lattice = |x: usize, y: usize| {
            let n = WASH / cell;
            (hash2((x % n) as i32, (y % n) as i32, 0x7f4a) % 1000) as f32 / 1000.0 - 0.5
        };
        let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
        let wash = (0..WASH * WASH)
            .map(|index| {
                let (x, y) = (index % WASH, index / WASH);
                let (cx, cy) = (x / cell, y / cell);
                let (tx, ty) = (
                    smooth((x % cell) as f32 / cell as f32),
                    smooth((y % cell) as f32 / cell as f32),
                );
                let top = lattice(cx, cy) * (1.0 - tx) + lattice(cx + 1, cy) * tx;
                let bottom = lattice(cx, cy + 1) * (1.0 - tx) + lattice(cx + 1, cy + 1) * tx;
                ((top * (1.0 - ty) + bottom * ty) * 1024.0) as i16
            })
            .collect();
        (grain, wash)
    })
}

/// Lays the paper over a canvas whose top-left pixel is (`gx`, `gy`) in a
/// larger picture, so neighbouring tiles meet without a seam: `tooth` and
/// `wash` are how strong each is, as fractions of the colour.
pub fn grain(canvas: &mut Canvas, at: (i32, i32), tooth: f32, wash: f32) {
    grain_lit(canvas, at, tooth, wash, [1.0; 3]);
}

/// The paper, and the hour's light (a multiplier per channel) with it, in
/// one pass.
pub fn grain_lit(
    canvas: &mut Canvas,
    (gx, gy): (i32, i32),
    tooth: f32,
    wash: f32,
    light: [f32; 3],
) {
    let light = light.map(|channel| (channel.clamp(0.0, 1.0) * 1024.0) as i32);
    let (grains, washes) = paper();
    let tooth = (tooth * 1024.0) as i32;
    let wash = (wash * 1024.0) as i32;
    let width = canvas.pixmap.width() as usize;
    let data = canvas.pixmap.data_mut();
    let start_g = gx.rem_euclid(GRAIN as i32) as usize;
    let start_w = gx.rem_euclid(WASH as i32) as usize;
    for (row, line) in data.chunks_exact_mut(width * 4).enumerate() {
        let y = gy + row as i32;
        let grain_row = &grains[y.rem_euclid(GRAIN as i32) as usize * GRAIN..][..GRAIN];
        let wash_row = &washes[y.rem_euclid(WASH as i32) as usize * WASH..][..WASH];
        let (mut gi, mut wi) = (start_g, start_w);
        for pixel in line.as_chunks_mut::<4>().0.iter_mut() {
            let (g, w) = (grain_row[gi] as i32, wash_row[wi] as i32);
            gi += 1;
            if gi == GRAIN {
                gi = 0;
            }
            wi += 1;
            if wi == WASH {
                wi = 0;
            }
            let alpha = pixel[3] as i32;
            if alpha == 0 {
                continue;
            }
            // 1024 is 1: darker in the tooth's pits, a little lighter on
            // its peaks, never past the pixel's own alpha.
            let k = 1024 + ((g * tooth + w * wash) >> 10);
            for (channel, lit) in pixel[..3].iter_mut().zip(light) {
                let m = (k * lit) >> 10;
                *channel = ((*channel as i32 * m) >> 10).min(alpha) as u8;
            }
        }
    }
}

/// A box blur of an alpha mask, `radius` pixels, run three times: close
/// to a Gaussian, and cheap.
pub fn blur_mask(mask: &mut [f32], width: usize, height: usize, radius: usize) {
    if radius == 0 || width == 0 || height == 0 {
        return;
    }
    let mut line = vec![0.0_f32; width.max(height)];
    for _ in 0..3 {
        for y in 0..height {
            let row = &mut mask[y * width..(y + 1) * width];
            box_line(row, &mut line[..width], radius);
        }
        let mut column = vec![0.0_f32; height];
        for x in 0..width {
            for y in 0..height {
                column[y] = mask[y * width + x];
            }
            box_line(&mut column, &mut line[..height], radius);
            for y in 0..height {
                mask[y * width + x] = column[y];
            }
        }
    }
}

fn box_line(values: &mut [f32], scratch: &mut [f32], radius: usize) {
    let n = values.len();
    scratch[..n].copy_from_slice(values);
    let span = (2 * radius + 1) as f32;
    let mut sum = 0.0;
    // Past the ends counts as empty.
    for value in scratch.iter().take(radius.min(n)) {
        sum += value;
    }
    for (index, value) in values.iter_mut().enumerate() {
        if index + radius < n {
            sum += scratch[index + radius];
        }
        if index > radius {
            sum -= scratch[index - radius - 1];
        }
        *value = sum / span;
    }
}

/// An image GPUI can show: straight (not premultiplied) BGRA.
pub fn image(pixmap: &sk::Pixmap) -> Arc<RenderImage> {
    image_of(pixmap.clone())
}

/// The same, taking the pixmap's own pixels rather than copying them.
pub fn image_of(pixmap: sk::Pixmap) -> Arc<RenderImage> {
    static UNDO: OnceLock<Vec<u32>> = OnceLock::new();
    // 255 / alpha in 16.16 fixed point, to undo premultiplying.
    let undo = UNDO.get_or_init(|| {
        (0..256_u32)
            .map(|alpha| (255_u32 << 16).checked_div(alpha).unwrap_or(0))
            .collect()
    });
    let (width, height) = (pixmap.width(), pixmap.height());
    let mut bytes = pixmap.take();
    for pixel in bytes.as_chunks_mut::<4>().0.iter_mut() {
        let alpha = pixel[3];
        let (r, b) = (pixel[0], pixel[2]);
        if alpha == 255 || alpha == 0 {
            pixel[0] = b;
            pixel[2] = r;
            continue;
        }
        let k = undo[alpha as usize];
        let straight = |channel: u8| ((channel as u32 * k + 0x8000) >> 16).min(255) as u8;
        pixel[0] = straight(b);
        pixel[1] = straight(pixel[1]);
        pixel[2] = straight(r);
    }
    let buffer = image::RgbaImage::from_raw(width, height, bytes)
        .unwrap_or_else(|| image::RgbaImage::new(1, 1));
    Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]))
}

/// How many threads the painter paints on.
fn threads() -> usize {
    // Work already on one of the painter's threads is not split again.
    if IN_POOL.with(|here| here.get()) {
        return 1;
    }
    all_threads()
}

thread_local! {
    static IN_POOL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn all_threads() -> usize {
    static THREADS: OnceLock<usize> = OnceLock::new();
    *THREADS.get_or_init(|| {
        std::env::var("WORLD_GPUI_PAINT_THREADS")
            .ok()
            .and_then(|threads| threads.parse::<usize>().ok())
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(1)
            })
            .clamp(1, 8)
    })
}

/// Runs `jobs` across the painter's threads, and gives back what each
/// made, in order. Each job paints a picture of its own, so the result is
/// the same however the jobs are shared out.
pub fn parallel<T: Send>(jobs: Vec<Box<dyn FnOnce() -> T + Send + '_>>) -> Vec<T> {
    let count = jobs.len();
    if count <= 1 || threads() == 1 {
        return jobs.into_iter().map(|job| job()).collect();
    }
    let per = count.div_ceil(threads());
    let mut batches = Vec::new();
    let mut jobs = jobs.into_iter();
    loop {
        let batch = jobs.by_ref().take(per).collect::<Vec<_>>();
        if batch.is_empty() {
            break;
        }
        batches.push(batch);
    }
    std::thread::scope(|scope| {
        let handles = batches
            .into_iter()
            .map(|batch| {
                scope.spawn(move || batch.into_iter().map(|job| job()).collect::<Vec<_>>())
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap_or_default())
            .collect()
    })
}

/// A picture `width` by `height` device pixels at `scale`, its top-left at
/// the drawing point `origin`, painted by `paint` in side-by-side strips on
/// the painter's threads and put together. `paint` is given each strip's
/// canvas and its top-left pixel in the whole picture.
pub fn in_strips(
    width: u32,
    height: u32,
    scale: f32,
    origin: (f32, f32),
    paint: &(dyn Fn(&mut Canvas, (i32, i32)) + Sync),
) -> Option<sk::Pixmap> {
    let strips = threads().min((width / 128).max(1) as usize) as u32;
    let wide = width.div_ceil(strips);
    let jobs = (0..strips)
        .map(|strip| {
            Box::new(move || {
                let left = strip * wide;
                let w = wide.min(width.saturating_sub(left));
                let mut canvas = Canvas::new(
                    w.max(1),
                    height,
                    scale,
                    (origin.0 + left as f32 / scale, origin.1),
                )?;
                paint(&mut canvas, (left as i32, 0));
                Some((left, canvas.pixmap))
            }) as Box<dyn FnOnce() -> Option<(u32, sk::Pixmap)> + Send + '_>
        })
        .collect();
    let mut whole = sk::Pixmap::new(width.max(1), height.max(1))?;
    for (left, strip) in parallel(jobs).into_iter().flatten() {
        whole.draw_pixmap(
            left as i32,
            0,
            strip.as_ref(),
            &sk::PixmapPaint {
                blend_mode: sk::BlendMode::Source,
                ..sk::PixmapPaint::default()
            },
            sk::Transform::identity(),
            None,
        );
    }
    Some(whole)
}

/// Hashes anything hashable, plus floats by their bits.
#[derive(Default)]
pub struct Key(std::collections::hash_map::DefaultHasher);

impl Key {
    pub fn new(tag: &str) -> Self {
        let mut key = Self::default();
        tag.hash(&mut key.0);
        key
    }

    pub fn add(&mut self, value: impl Hash) -> &mut Self {
        value.hash(&mut self.0);
        self
    }

    pub fn float(&mut self, value: f32) -> &mut Self {
        value.to_bits().hash(&mut self.0);
        self
    }

    pub fn colour(&mut self, value: Hsla) -> &mut Self {
        for part in [value.h, value.s, value.l, value.a] {
            self.float(part);
        }
        self
    }

    pub fn finish(&self) -> u64 {
        self.0.finish()
    }
}

struct Entry {
    image: Arc<RenderImage>,
    window: u64,
    used: Instant,
    bytes: usize,
    /// When it was ready to show.
    ready: Instant,
}

/// The painted images, kept while they are shown.
#[derive(Default)]
struct Cache {
    entries: HashMap<u64, Entry>,
    /// Pictures already handed to the display.
    handed: std::collections::HashSet<u64>,
    /// How many bytes of pictures new to the display this frame may still
    /// hand over; none when there is no limit.
    budget: Option<usize>,
    /// What was painted and came out empty (a tile with no building on
    /// it), and when it was last asked for.
    empty: HashMap<u64, Instant>,
    /// Per window, when its current frame and the one before began.
    frames: HashMap<u64, (Instant, Instant)>,
    /// How long painting has taken, for a look at the frame time.
    painted: Duration,
    paints: u32,
    /// Bytes of pictures new to the display handed over this frame, and
    /// how long drawing the still layers has taken in it.
    fresh: usize,
    drawing: Duration,
    /// What handing a new picture to the display costs on this machine,
    /// in nanoseconds a byte, as measured over the last frames (none yet).
    cost: Option<f32>,
    /// Whether the window being drawn is still opening, under its loading
    /// wash.
    opening: bool,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

/// Images not shown for this long are let go.
const KEEP: Duration = Duration::from_secs(3);
/// And never more than this many bytes of them at once.
const BUDGET: usize = 384 << 20;

/// The image for `key`, painted by `paint` if it is not already kept.
pub fn cached(
    window: &mut gpui::Window,
    key: u64,
    paint: impl FnOnce() -> Option<sk::Pixmap>,
) -> Option<Arc<RenderImage>> {
    let id = window_id(window);
    let now = Instant::now();
    let hit = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.entries.get_mut(&key).map(|entry| {
            entry.used = now;
            entry.image.clone()
        })
    });
    if hit.is_some() {
        return hit;
    }
    let started = Instant::now();
    let pixmap = paint()?;
    let bytes = pixmap.data().len();
    let image = image_of(pixmap);
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.painted += started.elapsed();
        cache.paints += 1;
        cache.entries.insert(
            key,
            Entry {
                image: image.clone(),
                window: id,
                used: now,
                bytes,
                ready: now,
            },
        );
    });
    Some(image)
}

/// Work that paints one picture, to be run on any of the painter's threads.
pub type Painting<'a> = Box<dyn FnOnce() -> Option<sk::Pixmap> + Send + 'a>;

/// The images for many keys at once; those not kept are painted together,
/// across the painter's threads.
pub fn cached_all<'a>(
    window: &mut gpui::Window,
    wanted: Vec<(u64, Painting<'a>)>,
) -> Vec<Option<Arc<RenderImage>>> {
    let id = window_id(window);
    let now = Instant::now();
    let mut found = Vec::with_capacity(wanted.len());
    let mut missing = Vec::new();
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        for (slot, (key, paint)) in wanted.into_iter().enumerate() {
            match cache.entries.get_mut(&key) {
                Some(entry) => {
                    entry.used = now;
                    found.push(Some(entry.image.clone()));
                }
                None => {
                    found.push(None);
                    missing.push((slot, key, paint));
                }
            }
        }
    });
    if missing.is_empty() {
        return found;
    }
    let started = Instant::now();
    let (slots, jobs): (Vec<_>, Vec<_>) = missing
        .into_iter()
        .map(|(slot, key, paint)| {
            (
                (slot, key),
                Box::new(move || paint().map(|pixmap| (pixmap.data().len(), image_of(pixmap))))
                    as Box<dyn FnOnce() -> Option<(usize, Arc<RenderImage>)> + Send + 'a>,
            )
        })
        .unzip();
    let painted = parallel(jobs);
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.painted += started.elapsed();
        for ((slot, key), made) in slots.into_iter().zip(painted) {
            let Some((bytes, image)) = made else {
                continue;
            };
            cache.paints += 1;
            cache.entries.insert(
                key,
                Entry {
                    image: image.clone(),
                    window: id,
                    used: now,
                    bytes,
                    ready: now,
                },
            );
            found[slot] = Some(image);
        }
    });
    found
}

/// Lets go of this window's images that have not been shown for a while,
/// and of the oldest while there are more than the budget allows.
pub fn sweep(window: &mut gpui::Window) {
    let id = window_id(window);
    let now = Instant::now();
    let dropped = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        // Every canvas in a frame sweeps; a new frame is one that begins a
        // while after the last.
        let (current, before) = cache.frames.get(&id).copied().unwrap_or((now, now));
        let (current, before) = if now.duration_since(current) > Duration::from_millis(40) {
            (now, current)
        } else {
            (current, before)
        };
        cache.frames.insert(id, (current, before));
        // Anything shown in the last frame stays, however long ago that
        // was: a window drawn rarely never paints its layers twice.
        let shown = |entry: &Entry| entry.used >= before;
        let mut dropped = Vec::new();
        let stale = cache
            .entries
            .iter()
            .filter(|(_, entry)| {
                if entry.window == id {
                    !shown(entry) && before.duration_since(entry.used) > KEEP
                } else {
                    now.duration_since(entry.used) > KEEP * 40
                }
            })
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        for key in stale {
            cache.handed.remove(&key);
            if let Some(entry) = cache.entries.remove(&key) {
                if entry.window == id {
                    dropped.push(entry.image);
                }
            }
        }
        cache
            .empty
            .retain(|_, asked| now.duration_since(*asked) < KEEP * 20);
        let mut total = cache
            .entries
            .values()
            .map(|entry| entry.bytes)
            .sum::<usize>();
        if total > BUDGET {
            let mut ages = cache
                .entries
                .iter()
                .filter(|(_, entry)| entry.window == id && !shown(entry))
                .map(|(key, entry)| (entry.used, *key))
                .collect::<Vec<_>>();
            ages.sort();
            for (_, key) in ages {
                if total <= BUDGET {
                    break;
                }
                cache.handed.remove(&key);
                if let Some(entry) = cache.entries.remove(&key) {
                    total -= entry.bytes;
                    dropped.push(entry.image);
                }
            }
        }
        dropped
    });
    for image in dropped {
        let _ = window.drop_image(image);
    }
}

/// How long the painter has spent painting, and how many images, since
/// this was last asked.
pub fn painting_time() -> (Duration, u32) {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let out = (cache.painted, cache.paints);
        cache.painted = Duration::ZERO;
        cache.paints = 0;
        out
    })
}

fn window_id(window: &gpui::Window) -> u64 {
    window.window_handle().window_id().as_u64()
}

/// Colours everything painted so far by the light of the hour, as a
/// multiplier per channel; pixels where `skip` is set (lit windows, which
/// make their own light) keep theirs.
pub fn tint(canvas: &mut Canvas, light: [f32; 3], skip: Option<&[f32]>) {
    if light.iter().all(|channel| (channel - 1.0).abs() < 1e-3) {
        return;
    }
    let factors = light.map(|channel| (channel.clamp(0.0, 1.0) * 256.0) as u32);
    for (index, pixel) in canvas
        .pixmap
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        if pixel[3] == 0 || skip.is_some_and(|skip| skip[index] > 0.5) {
            continue;
        }
        for (channel, factor) in pixel[..3].iter_mut().zip(factors) {
            *channel = ((*channel as u32 * factor) >> 8) as u8;
        }
    }
}

/// Where a pixmap is (nearly) one colour and solid: 1 there, 0 elsewhere.
pub fn find_colour(
    pixmap: &sk::Pixmap,
    (r, g, b): (u8, u8, u8),
    (tr, tg, tb): (u8, u8, u8),
) -> Vec<f32> {
    pixmap
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| {
            let hit = pixel[3] >= 200
                && pixel[0].abs_diff(r) <= tr
                && pixel[1].abs_diff(g) <= tg
                && pixel[2].abs_diff(b) <= tb;
            if hit {
                1.0
            } else {
                0.0
            }
        })
        .collect()
}

/// Lays `rgb` along every top edge of what is painted, `thick` pixels
/// deep and a little uneven: snow on roofs and sills, frost, drifted dust.
pub fn cap(pixmap: &mut sk::Pixmap, thick: f32, rgb: [f32; 3], strength: f32) {
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    let data = pixmap.data_mut();
    for x in 0..width {
        // A little uneven, as snow lies, but smoothly so: never in steps.
        let x_ = x as f32;
        let depth = thick * (1.0 + 0.12 * (x_ * 0.05).sin() + 0.06 * (x_ * 0.17 + 1.3).sin());
        // How deep into what is painted this column has gone, counted in
        // coverage rather than whole pixels, so the snow's edge follows a
        // slope smoothly instead of in steps.
        let mut into = 0.0_f32;
        for y in 0..height {
            let at = (y * width + x) * 4;
            let alpha = data[at + 3] as f32 / 255.0;
            if alpha < 0.03 {
                into = 0.0;
                continue;
            }
            let before = into;
            into += alpha;
            let covered = ((depth - before) / (into - before).max(1e-3)).clamp(0.0, 1.0);
            if covered <= 0.0 {
                continue;
            }
            let k = strength * covered * (1.0 - before / depth).clamp(0.35, 1.0);
            for channel in 0..3 {
                let value = data[at + channel] as f32;
                data[at + channel] = (value + (rgb[channel] * 255.0 * alpha - value) * k) as u8;
            }
        }
    }
}

/// Whether a pixel is leafy green.
fn green(pixel: &[u8]) -> bool {
    pixel[3] > 100
        && pixel[1] as i32 > pixel[0] as i32 + 14
        && pixel[1] as i32 > pixel[2] as i32 + 10
}

/// Autumn in the trees: green turns to gold, orange and rust, in soft
/// patches rather than one flat colour.
pub fn autumn(pixmap: &mut sk::Pixmap) {
    let (_, washes) = paper();
    let width = pixmap.width() as usize;
    for (index, pixel) in pixmap
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .enumerate()
    {
        if !green(pixel) {
            continue;
        }
        let (x, y) = (index % width, index / width);
        // The wash's slow unevenness picks the colour: gold to rust.
        let wash = washes[(y * 3 % WASH) * WASH + (x * 3 % WASH)] as f32 / 1024.0 + 0.5;
        let gold = [0.93, 0.66, 0.24];
        let rust = [0.8, 0.36, 0.16];
        let lum = (pixel[0] as f32 * 0.3 + pixel[1] as f32 * 0.59 + pixel[2] as f32 * 0.11) / 255.0;
        let alpha = pixel[3] as f32;
        for channel in 0..3 {
            let hue = gold[channel] + (rust[channel] - gold[channel]) * wash.clamp(0.0, 1.0);
            let target = (hue * (0.6 + lum * 0.6)).min(1.0) * alpha;
            let value = pixel[channel] as f32;
            pixel[channel] = (value + (target - value) * 0.78) as u8;
        }
    }
}

/// Spring blossom: small pink flowers dotted over what is green.
pub fn blossom(pixmap: &mut sk::Pixmap, radius: f32) {
    let width = pixmap.width() as i32;
    let height = pixmap.height() as i32;
    let step = (radius * 4.0).max(3.0) as i32;
    let mut builder = sk::PathBuilder::new();
    let mut deep = sk::PathBuilder::new();
    let data = pixmap.data().to_vec();
    for gy in (0..height).step_by(step as usize) {
        for gx in (0..width).step_by(step as usize) {
            let seed = hash2(gx, gy, 0xb105);
            let x = gx + (seed % step as u32) as i32;
            let y = gy + ((seed >> 8) % step as u32) as i32;
            if x >= width || y >= height {
                continue;
            }
            let at = ((y * width + x) * 4) as usize;
            if !green(&data[at..at + 4]) || seed % 5 > 2 {
                continue;
            }
            if (seed >> 16).is_multiple_of(3) {
                deep.push_circle(x as f32 + 0.5, y as f32 + 0.5, radius);
            } else {
                builder.push_circle(x as f32 + 0.5, y as f32 + 0.5, radius);
            }
        }
    }
    for (path, (r, g, b)) in [(builder, (0xfb, 0xe2, 0xea)), (deep, (0xf2, 0xb6, 0xca))] {
        let Some(path) = path.finish() else {
            continue;
        };
        let mut paint = sk::Paint::default();
        paint.set_color_rgba8(r, g, b, 240);
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            sk::FillRule::Winding,
            sk::Transform::identity(),
            None,
        );
    }
}

/// A building's light, in one pass over what is painted: toward the lit
/// side (`side` -1 left, 1 right) a warm light, toward the other a cool
/// shade, both `strength` at the walls' edges from `left` to `right`
/// pixels; and the bottom `foot` rows darker toward `base`, where the
/// ground holds the light back.
#[allow(clippy::too_many_arguments)]
pub fn light_and_shade(
    pixmap: &mut sk::Pixmap,
    (left, right): (f32, f32),
    side: f32,
    strength: f32,
    warm: [f32; 3],
    cool: [f32; 3],
    (foot, base): (f32, f32),
    deep: f32,
    light: [f32; 3],
    skip: Option<&[f32]>,
) {
    let width = pixmap.width() as usize;
    let span = (right - left).max(1.0);
    let fixed = |value: f32| (value.clamp(0.0, 1.0) * 256.0) as i32;
    // Per column: how much warm light, and how much cool shade, in 256ths.
    let columns = (0..width)
        .map(|x| {
            let t = ((x as f32 + 0.5 - left) / span).clamp(0.0, 1.0);
            let toward = if side < 0.0 { 1.0 - t } else { t };
            let lit = ((toward - 0.55) / 0.45).clamp(0.0, 1.0) * strength;
            let dark = ((0.4 - toward) / 0.4).clamp(0.0, 1.0) * (strength + 0.06);
            (fixed(lit), fixed(dark))
        })
        .collect::<Vec<_>>();
    let (warm, cool, light) = (warm.map(fixed), cool.map(fixed), light.map(fixed));
    for (row, line) in pixmap.data_mut().chunks_exact_mut(width * 4).enumerate() {
        let y = row as f32 + 0.5;
        let low = if y > foot {
            ((y - foot) / (base - foot).max(1.0)).clamp(0.0, 1.0) * deep
        } else {
            0.0
        };
        let keep = 256 - fixed(low);
        for (column, (pixel, (lit, dark))) in line
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(&columns)
            .enumerate()
        {
            let alpha = pixel[3] as i32;
            if alpha == 0 {
                continue;
            }
            // A lit window makes its own light: the hour does not dim it.
            let own = skip.is_some_and(|skip| skip[row * width + column] > 0.5);
            for channel in 0..3 {
                let mut value = pixel[channel] as i32;
                value += ((((warm[channel] * alpha) >> 8) - value) * lit) >> 8;
                value += ((((cool[channel] * alpha) >> 8) - value) * dark) >> 8;
                value = (value * keep) >> 8;
                if !own {
                    value = (value * light[channel]) >> 8;
                }
                pixel[channel] = value.clamp(0, alpha) as u8;
            }
        }
    }
}

/// A linear gradient laid only over what is already painted, keeping its
/// shape: light and shade on a building.
pub fn shade_atop(canvas: &mut Canvas, start: (f32, f32), end: (f32, f32), stops: &[(f32, Hsla)]) {
    if (start.0 - end.0).abs() < 1e-3 && (start.1 - end.1).abs() < 1e-3 {
        return;
    }
    let stops = stops
        .iter()
        .map(|(at, c)| sk::GradientStop::new(*at, colour(*c)))
        .collect::<Vec<_>>();
    let Some(shader) = sk::LinearGradient::new(
        sk::Point::from_xy(start.0, start.1),
        sk::Point::from_xy(end.0, end.1),
        stops,
        sk::SpreadMode::Pad,
        sk::Transform::identity(),
    ) else {
        return;
    };
    let paint = sk::Paint {
        shader,
        blend_mode: sk::BlendMode::SourceAtop,
        anti_alias: false,
        ..sk::Paint::default()
    };
    let transform = canvas.transform();
    let (w, h) = (
        canvas.pixmap.width() as f32 / canvas.scale,
        canvas.pixmap.height() as f32 / canvas.scale,
    );
    if let Some(rect) = sk::Rect::from_xywh(canvas.origin.0, canvas.origin.1, w, h) {
        canvas.pixmap.fill_rect(rect, &paint, transform, None);
    }
}

/// A warm line of low sun down every edge that faces it: pixels whose
/// neighbour `reach` pixels toward the light (`side` -1 left, 1 right) is
/// empty.
pub fn rim(pixmap: &mut sk::Pixmap, side: f32, reach: i32, rgb: [f32; 3], strength: f32) {
    let width = pixmap.width() as i32;
    let height = pixmap.height() as i32;
    let step = if side < 0.0 { -reach } else { reach };
    let alphas = pixmap
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[3])
        .collect::<Vec<_>>();
    let data = pixmap.data_mut();
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) as usize;
            if alphas[index] == 0 {
                continue;
            }
            // How open it is toward the light, as a share, not a yes or
            // no: along a slope the anti-aliased edge makes that share
            // change smoothly, so the line of light has no steps.
            let (nx, ny) = (x + step, y - reach / 2);
            let beyond = if nx < 0 || nx >= width || ny < 0 {
                0.0
            } else {
                alphas[(ny.min(height - 1) * width + nx) as usize] as f32 / 255.0
            };
            let open = 1.0 - beyond;
            if open <= 0.0 {
                continue;
            }
            let alpha = alphas[index] as f32 / 255.0;
            let k = strength * open;
            for channel in 0..3 {
                let value = data[index * 4 + channel] as f32;
                let target = rgb[channel] * 255.0 * alpha;
                data[index * 4 + channel] = (value + (target - value) * k) as u8;
            }
        }
    }
}

/// How many pixels each way one pixel of a shadow's or a glow's mask
/// stands for: they are soft, so a quarter of the resolution is plenty.
pub const COARSE: usize = 4;

/// A mask at a quarter of the resolution each way: each coarse pixel the
/// mean of the fine ones under it.
fn coarse(width: usize, height: usize, value: impl Fn(usize) -> f32) -> (Vec<f32>, usize, usize) {
    let (cw, ch) = (width.div_ceil(COARSE), height.div_ceil(COARSE));
    let mut mask = vec![0.0_f32; cw * ch];
    let share = 1.0 / (COARSE * COARSE) as f32;
    for y in 0..height {
        let row = (y / COARSE) * cw;
        for x in 0..width {
            mask[row + x / COARSE] += value(y * width + x) * share;
        }
    }
    (mask, cw, ch)
}

/// A painted shape as a soft dark at a quarter of its resolution: what it
/// casts on the ground.
pub fn silhouette(pixmap: &sk::Pixmap, blur: usize) -> Option<sk::Pixmap> {
    let (width, height) = (pixmap.width() as usize, pixmap.height() as usize);
    let data = pixmap.data();
    let (mut mask, cw, ch) = coarse(width, height, |at| data[at * 4 + 3] as f32 / 255.0);
    blur_mask(&mut mask, cw, ch, (blur / COARSE).max(1));
    let mut out = sk::Pixmap::new(cw as u32, ch as u32)?;
    for (pixel, value) in out.data_mut().as_chunks_mut::<4>().0.iter_mut().zip(&mask) {
        pixel[3] = (value.clamp(0.0, 1.0) * 255.0) as u8;
    }
    Some(out)
}

/// A copy of a silhouette (alpha only) inked in `rgb`, 0 to 1: a shadow
/// in the hour's own cool colour rather than black.
pub fn inked(silhouette: &sk::Pixmap, rgb: [f32; 3]) -> sk::Pixmap {
    let mut out = silhouette.clone();
    for pixel in out.data_mut().as_chunks_mut::<4>().0.iter_mut() {
        let a = pixel[3] as f32;
        for (channel, value) in rgb.iter().enumerate() {
            pixel[channel] = (value.clamp(0.0, 1.0) * a).round() as u8;
        }
    }
    out
}

/// The windows of a mask as runs of columns: where each lit window is,
/// from its left column to its right and its top row to its bottom.
fn windows_of(mask: &[f32], width: usize) -> Vec<(usize, usize, usize, usize)> {
    let height = mask.len() / width.max(1);
    let mut runs = Vec::new();
    let mut start = None;
    for x in 0..=width {
        let lit = x < width && (0..height).any(|y| mask[y * width + x] > 0.5);
        match (lit, start) {
            (true, None) => start = Some(x),
            (false, Some(from)) => {
                let rows =
                    (0..height).filter(|y| (from..x).any(|column| mask[y * width + column] > 0.5));
                let (top, bottom) = rows.fold((usize::MAX, 0), |(top, bottom), y| {
                    (top.min(y), bottom.max(y))
                });
                if x - from >= 2 && bottom > top {
                    runs.push((from, x - 1, top, bottom));
                }
                start = None;
            }
            _ => {}
        }
    }
    runs
}

/// Someone indoors, `count` of them, each a dark shape of head and
/// shoulders against a lit window: only where the glass is, so someone
/// standing between windows cannot be seen at all.
pub fn inside(pixmap: &mut sk::Pixmap, glass: &[f32], count: usize) {
    let width = pixmap.width() as usize;
    let windows = windows_of(glass, width);
    if windows.is_empty() {
        return;
    }
    let Some(mut shapes) = sk::Pixmap::new(pixmap.width(), pixmap.height()) else {
        return;
    };
    let mut paint = sk::Paint::default();
    paint.set_color(sk::Color::from_rgba8(0x5a, 0x3a, 0x26, 235));
    paint.anti_alias = true;
    for person in 0..count.min(windows.len() * 2) {
        let (left, right, top, bottom) = windows[(person * 3 + 1) % windows.len()];
        let w = (right - left + 1) as f32;
        let tall = (bottom - top + 1) as f32;
        let offset = if person >= windows.len() { 0.3 } else { 0.0 };
        let cx = left as f32 + w * (0.5 + offset);
        let head = (tall * 0.2).min(w * 0.28).max(1.5);
        let neck = bottom as f32 - tall * 0.34;
        let mut builder = sk::PathBuilder::new();
        builder.push_circle(cx, neck - head * 0.9, head);
        if let Some(rect) = sk::Rect::from_xywh(cx - head * 1.7, neck, head * 3.4, tall) {
            builder.push_rect(rect);
        }
        if let Some(path) = builder.finish() {
            shapes.fill_path(
                &path,
                &paint,
                sk::FillRule::Winding,
                sk::Transform::identity(),
                None,
            );
        }
    }
    for ((pixel, shape), mask) in pixmap
        .data_mut()
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(shapes.data().as_chunks::<4>().0.iter())
        .zip(glass)
    {
        let k = shape[3] as f32 / 255.0 * mask;
        if k <= 0.0 {
            continue;
        }
        for channel in 0..3 {
            let value = pixel[channel] as f32;
            let target = shape[channel] as f32 / (shape[3] as f32 / 255.0).max(1e-3)
                * pixel[3] as f32
                / 255.0;
            pixel[channel] = (value + (target - value) * k) as u8;
        }
    }
}

/// The soft light a lit window throws around itself: the glass mask
/// blurred, laid over in `rgb`.
pub fn bloom(pixmap: &mut sk::Pixmap, glass: &[f32], radius: usize, rgb: [f32; 3], strength: f32) {
    let (width, height) = (pixmap.width() as usize, pixmap.height() as usize);
    let (mut mask, cw, ch) = coarse(width, height, |at| glass[at]);
    blur_mask(&mut mask, cw, ch, (radius * 2 / COARSE).max(1));
    let peak = mask.iter().copied().fold(0.0_f32, f32::max).max(0.05);
    let data = pixmap.data_mut();
    // Each fine row read from the two coarse rows around it, smoothly.
    let mut row = vec![0.0_f32; width];
    for y in 0..height {
        let fy = ((y as f32 + 0.5) / COARSE as f32 - 0.5).max(0.0);
        let y0 = (fy as usize).min(ch - 1);
        let y1 = (y0 + 1).min(ch - 1);
        let ty = fy - y0 as f32;
        let (upper, lower) = (&mask[y0 * cw..][..cw], &mask[y1 * cw..][..cw]);
        if upper
            .iter()
            .chain(lower)
            .all(|value| *value / peak * strength <= 0.004)
        {
            continue;
        }
        for (x, value) in row.iter_mut().enumerate() {
            let fx = ((x as f32 + 0.5) / COARSE as f32 - 0.5).max(0.0);
            let x0 = (fx as usize).min(cw - 1);
            let x1 = (x0 + 1).min(cw - 1);
            let tx = fx - x0 as f32;
            let top = upper[x0] + (upper[x1] - upper[x0]) * tx;
            let bottom = lower[x0] + (lower[x1] - lower[x0]) * tx;
            *value = top + (bottom - top) * ty;
        }
        for (x, value) in row.iter().enumerate() {
            let g = (value / peak).min(1.0) * strength;
            if g <= 0.004 {
                continue;
            }
            let at = (y * width + x) * 4;
            let keep = 1.0 - g;
            for channel in 0..3 {
                data[at + channel] =
                    (rgb[channel] * 255.0 * g + data[at + channel] as f32 * keep).min(255.0) as u8;
            }
            data[at + 3] = (255.0 * g + data[at + 3] as f32 * keep).min(255.0) as u8;
        }
    }
}

/// A vignette over a canvas `width` by `height` (in its drawing units):
/// clear in the middle, `edge` at the corners.
pub fn vignette(canvas: &mut Canvas, width: f32, height: f32, edge: Hsla) {
    let c = colour(edge);
    let faded = |k: f32| {
        let mut faded = c;
        faded.set_alpha(c.alpha() * k);
        faded
    };
    let stops = vec![
        sk::GradientStop::new(0.0, faded(0.0)),
        sk::GradientStop::new(0.55, faded(0.0)),
        sk::GradientStop::new(0.8, faded(0.4)),
        sk::GradientStop::new(1.0, c),
    ];
    let local = sk::Transform::from_row(
        width * 0.72,
        0.0,
        0.0,
        height * 0.78,
        width / 2.0,
        height / 2.0,
    );
    let Some(shader) = sk::RadialGradient::new(
        sk::Point::from_xy(0.0, 0.0),
        sk::Point::from_xy(0.0, 0.0),
        1.0,
        stops,
        sk::SpreadMode::Pad,
        local,
    ) else {
        return;
    };
    let paint = sk::Paint {
        shader,
        anti_alias: false,
        ..sk::Paint::default()
    };
    let transform = canvas.transform();
    if let Some(rect) = sk::Rect::from_xywh(0.0, 0.0, width, height) {
        canvas.pixmap.fill_rect(rect, &paint, transform, None);
    }
}

/// In tests, how long each named part of painting has taken in all.
#[cfg(test)]
pub(crate) fn profile(
) -> &'static std::sync::Mutex<std::collections::BTreeMap<&'static str, Duration>> {
    static PROFILE: OnceLock<std::sync::Mutex<std::collections::BTreeMap<&'static str, Duration>>> =
        OnceLock::new();
    PROFILE.get_or_init(Default::default)
}

/// How long this thread has been running on a CPU: in tests, to tell the
/// window thread's own work from time it spent waiting for a core.
#[cfg(test)]
#[allow(unsafe_code)]
pub(crate) fn thread_cpu() -> Duration {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `now` is a valid timespec for clock_gettime to fill in.
    let status = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut now) };
    if status != 0 {
        return Duration::ZERO;
    }
    Duration::new(now.tv_sec as u64, now.tv_nsec as u32)
}

/// The profile's name for the CPU time of a part of the window's frame.
#[cfg(test)]
pub(crate) fn cpu_name(part: &str) -> &'static str {
    static NAMES: OnceLock<std::sync::Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut names = NAMES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    names
        .entry(part.to_owned())
        .or_insert_with(|| Box::leak(format!("cpu: {part}").into_boxed_str()))
}

/// Runs `work`, and in tests adds how long it took to the part `name`.
pub(crate) fn timed<T>(name: &'static str, work: impl FnOnce() -> T) -> T {
    #[cfg(test)]
    {
        let (started, cpu) = (Instant::now(), thread_cpu());
        let out = work();
        let took = started.elapsed();
        let mut profile = profile()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *profile.entry(name).or_default() += took;
        if let Some(part) = name.strip_prefix("main: ") {
            let cpu = thread_cpu().saturating_sub(cpu);
            *profile.entry(cpu_name(part)).or_default() += cpu;
        }
        out
    }
    #[cfg(not(test))]
    {
        let _ = name;
        work()
    }
}

/// Fades a picture out downward: whole above row `from`, gone below row
/// `to` (in its own pixels), smoothly between.
pub fn fade_down(pixmap: &mut sk::Pixmap, from: f32, to: f32) {
    let width = pixmap.width() as usize;
    let span = (to - from).max(1.0);
    for (row, line) in pixmap.data_mut().chunks_exact_mut(width * 4).enumerate() {
        let t = ((row as f32 + 0.5 - from) / span).clamp(0.0, 1.0);
        let keep = 1.0 - t * t * (3.0 - 2.0 * t);
        if keep >= 0.999 {
            continue;
        }
        let k = (keep * 256.0) as u32;
        for value in line.iter_mut() {
            *value = ((*value as u32 * k) >> 8) as u8;
        }
    }
}

/// Painting off the window's thread: work is handed to the painter's own
/// threads, and what they paint is picked up on the window's thread the
/// next time it looks. Nothing on the window's thread waits for a picture.
struct Pool {
    /// Work waiting for a painter: what the camera needs to show anything
    /// at all first (the rough painting of the place, the sky and the
    /// hills), then the rest, each in the order asked.
    queue: std::sync::Mutex<(VecDeque<Job>, VecDeque<Job>)>,
    ready: std::sync::Condvar,
    /// What has been asked for and not yet painted, and when it was last
    /// asked for: work nobody still wants is skipped.
    asked: std::sync::Mutex<HashMap<u64, Instant>>,
    /// Finished work, waiting to be picked up: the key, the window it is
    /// for, and the image (none if it came out empty).
    done: std::sync::Mutex<Vec<Finished>>,
}

/// Finished work: the key, the window it is for, and the image and its
/// size in bytes (none if it came out empty).
type Finished = (u64, u64, Option<(usize, Arc<RenderImage>)>);

struct Job {
    key: u64,
    window: u64,
    paint: Box<dyn FnOnce() -> Option<sk::Pixmap> + Send + 'static>,
}

/// Work not asked for again within this long is no longer wanted (the view
/// has moved on, or the hour turned): long enough that a slow machine with
/// a long queue still gets to all of it.
const WANTED: Duration = Duration::from_secs(8);

fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        // One core is left to the window's thread, so it is never kept
        // waiting for one.
        for index in 0..all_threads().saturating_sub(1).max(1) {
            let _ = std::thread::Builder::new()
                .name(format!("world-painter-{index}"))
                .spawn(move || loop {
                    IN_POOL.with(|here| here.set(true));
                    let pool = pool();
                    let job = {
                        let Ok(mut queue) = pool.queue.lock() else {
                            return;
                        };
                        loop {
                            if let Some(job) = queue.0.pop_front().or_else(|| queue.1.pop_front()) {
                                break job;
                            }
                            queue = match pool.ready.wait(queue) {
                                Ok(queue) => queue,
                                Err(_) => return,
                            };
                        }
                    };
                    let wanted = pool
                        .asked
                        .lock()
                        .map(|asked| asked.get(&job.key).is_some_and(|at| at.elapsed() < WANTED))
                        .unwrap_or(false);
                    let made = if wanted {
                        (job.paint)().map(|pixmap| (pixmap.data().len(), image_of(pixmap)))
                    } else {
                        None
                    };
                    // Finished before it is no longer under way, so it is
                    // never asked for twice in between.
                    if wanted {
                        if let Ok(mut done) = pool.done.lock() {
                            done.push((job.key, job.window, made));
                        }
                    }
                    if let Ok(mut asked) = pool.asked.lock() {
                        asked.remove(&job.key);
                    }
                });
        }
        Pool {
            queue: Default::default(),
            ready: std::sync::Condvar::new(),
            asked: Default::default(),
            done: Default::default(),
        }
    })
}

/// Moves finished work into the cache: on the window's thread only.
fn pick_up() {
    let finished = pool()
        .done
        .lock()
        .map(|mut done| std::mem::take(&mut *done))
        .unwrap_or_default();
    if finished.is_empty() {
        return;
    }
    let now = Instant::now();
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        for (key, window, made) in finished {
            match made {
                Some((bytes, image)) => {
                    cache.paints += 1;
                    cache.entries.insert(
                        key,
                        Entry {
                            image,
                            window,
                            used: now,
                            bytes,
                            ready: now,
                        },
                    );
                }
                None => {
                    cache.empty.insert(key, now);
                }
            }
        }
    });
}

/// How many bytes of new pictures one frame hands to the display at most:
/// about six tiles.
const BUDGET_PER_FRAME: usize = 3 << 19;
/// And at least: half a tile (and always one picture, however big).
const LEAST_PER_FRAME: usize = 1 << 17;
/// And while the window is opening under its loading wash, at least four
/// tiles, and as many as twice the usual time allows, up to sixteen.
const OPENING_PER_FRAME: usize = 1 << 20;
/// How long a frame may spend handing new pictures to the display: the
/// copy into the display's memory is the window thread's own work, so it
/// is budgeted in time, measured on this machine, not in bytes.
const HANDOFF: Duration = Duration::from_micros(2500);
/// What handing over a byte is taken to cost before it is measured, in
/// nanoseconds: a slow machine's.
const FIRST_COST: f32 = 4.0;

/// Says whether the window about to be drawn is still opening (under its
/// loading wash): then its frames may hand over a little more at once.
pub fn opening(yes: bool) {
    CACHE.with(|cache| cache.borrow_mut().opening = yes);
}

/// Begins a frame of a window that paints off its thread: new pictures are
/// handed to the display a frame's share at a time, as many as it can
/// copy in [`HANDOFF`] by what the frames before measured. `limit` false
/// (painting right here) hands over everything at once.
pub fn begin_frame(limit: bool) {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        // What the last frame's new pictures cost, if it had enough of
        // them to tell.
        if cache.fresh >= 1 << 17 && limit {
            let sample = cache.drawing.as_nanos() as f32 / cache.fresh as f32;
            // Quick to learn a machine is slow, slow to trust it is fast:
            // one frame over its budget is a hitch, one under it only
            // shows a tile a frame later.
            let cost = match cache.cost {
                Some(cost) if sample > cost => sample,
                Some(cost) => cost * 0.9 + sample * 0.1,
                None => sample.max(FIRST_COST),
            };
            cache.cost = Some(cost.clamp(0.05, 100.0));
        }
        cache.fresh = 0;
        cache.drawing = Duration::ZERO;
        let cost = cache.cost.unwrap_or(FIRST_COST);
        // While the window is still opening (its loading wash over
        // everything, nothing yet to see move), a frame may take a little
        // more: the place arrives sooner, and no hitch can be seen.
        let (least, most, time) = if cache.opening {
            (OPENING_PER_FRAME, OPENING_PER_FRAME * 4, HANDOFF * 2)
        } else {
            (LEAST_PER_FRAME, BUDGET_PER_FRAME, HANDOFF)
        };
        cache.budget = limit.then(|| ((time.as_nanos() as f32 / cost) as usize).clamp(least, most));
    });
}

/// Lifts this frame's share: what a layer with nothing to show at all was
/// just painted for is handed over at once, however much (a window's first
/// frame, or its first at a new size, where a hitch cannot be seen).
pub fn unlimit() {
    CACHE.with(|cache| cache.borrow_mut().budget = None);
}

/// Whether this frame's share of new pictures has at least half of it left:
/// what is handed over ahead of being seen (the tiles a pan reaches next)
/// waits for what is seen now.
pub fn to_spare() -> bool {
    CACHE.with(|cache| {
        let cache = cache.borrow();
        match cache.budget {
            Some(left) => {
                let cost = cache.cost.unwrap_or(FIRST_COST);
                let whole = ((HANDOFF.as_nanos() as f32 / cost) as usize)
                    .clamp(LEAST_PER_FRAME, BUDGET_PER_FRAME);
                left * 2 >= whole
            }
            None => true,
        }
    })
}

#[cfg(test)]
thread_local! {
    /// In tests, how many bytes of new pictures this thread has handed to
    /// the display in all.
    pub(crate) static HANDED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Adds to how long drawing the still layers took this frame: what new
/// pictures cost to hand over is measured from it.
pub fn note_drawing(took: Duration) {
    CACHE.with(|cache| cache.borrow_mut().drawing += took);
}

/// What is known of a picture: painted (an image, or nothing to show), and
/// since when.
#[derive(Clone)]
pub enum Ready {
    Image(Arc<RenderImage>, Instant),
    Empty,
}

/// As [`ready`], handed to the display whatever is left of this frame's
/// share: a small picture the frame cannot do without (the rough painting
/// of what the camera sees).
pub fn ready_now(key: u64) -> Option<Ready> {
    let budget = CACHE.with(|cache| cache.borrow_mut().budget.take());
    let ready = ready(key);
    CACHE.with(|cache| cache.borrow_mut().budget = budget);
    ready
}

/// The picture for `key` if it is painted, without painting it.
pub fn ready(key: u64) -> Option<Ready> {
    pick_up();
    let now = Instant::now();
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let cache = &mut *cache;
        if let Some(entry) = cache.entries.get_mut(&key) {
            // A picture not shown before is handed to the display only while
            // this frame's share of new pictures lasts: a hundred tiles
            // arriving at once are shown over a few frames, never all in one.
            if !cache.handed.contains(&key) {
                if let Some(left) = cache.budget.as_mut() {
                    // The first new picture of a frame goes however big.
                    if *left < entry.bytes && cache.fresh > 0 {
                        return None;
                    }
                    *left = left.saturating_sub(entry.bytes);
                }
                cache.fresh += entry.bytes;
                #[cfg(test)]
                HANDED.with(|handed| handed.set(handed.get() + entry.bytes));
                cache.handed.insert(key);
            }
            entry.used = now;
            return Some(Ready::Image(entry.image.clone(), entry.ready));
        }
        cache.empty.get_mut(&key).map(|asked| {
            *asked = now;
            Ready::Empty
        })
    })
}

/// Whether the picture for `key` is painted (or came out empty), without
/// handing it to the display: whether a layer is complete, or a tile just
/// out of view is ready for a pan. It is kept as if it were shown.
pub fn painted(key: u64) -> bool {
    pick_up();
    let now = Instant::now();
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(entry) = cache.entries.get_mut(&key) {
            entry.used = now;
            return true;
        }
        cache
            .empty
            .get_mut(&key)
            .map(|asked| *asked = now)
            .is_some()
    })
}

/// Whether the picture for `key` has been handed to the display (or came
/// out empty and has nothing to hand), so drawing it costs this frame
/// nothing new.
pub fn handed(key: u64) -> bool {
    CACHE.with(|cache| {
        let cache = cache.borrow();
        cache.handed.contains(&key) || cache.empty.contains_key(&key)
    })
}

/// Asks for the picture for `key` to be painted by `paint` off the
/// window's thread, unless it is painted or under way already; or, when
/// `now` is set, paints it right here.
pub fn want(
    window: &gpui::Window,
    key: u64,
    now: bool,
    paint: Box<dyn FnOnce() -> Option<sk::Pixmap> + Send + 'static>,
) {
    ask(window, key, now, false, paint)
}

/// As [`want`], ahead of everything asked for without `first`: what the
/// window needs to show anything at all of what the camera sees.
pub fn ask(
    window: &gpui::Window,
    key: u64,
    now: bool,
    first: bool,
    paint: Box<dyn FnOnce() -> Option<sk::Pixmap> + Send + 'static>,
) {
    if painted(key) {
        return;
    }
    let id = window_id(window);
    if now {
        let started = Instant::now();
        let made = paint().map(|pixmap| (pixmap.data().len(), image_of(pixmap)));
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.painted += started.elapsed();
            let at = Instant::now();
            match made {
                Some((bytes, image)) => {
                    cache.paints += 1;
                    cache.entries.insert(
                        key,
                        Entry {
                            image,
                            window: id,
                            used: at,
                            bytes,
                            ready: at,
                        },
                    );
                }
                None => {
                    cache.empty.insert(key, at);
                }
            }
        });
        return;
    }
    let pool = pool();
    let fresh = pool
        .asked
        .lock()
        .map(|mut asked| asked.insert(key, Instant::now()).is_none())
        .unwrap_or(false);
    if fresh {
        if let Ok(mut queue) = pool.queue.lock() {
            let job = Job {
                key,
                window: id,
                paint,
            };
            if first {
                queue.0.push_back(job);
            } else {
                queue.1.push_back(job);
            }
            pool.ready.notify_one();
        }
    }
}

/// Whether pictures are painted right where they are asked for, rather
/// than off the window's thread: in tests (so a picture is whole the first
/// time it is drawn), or when `WORLD_GPUI_PAINT_SYNC` is set.
pub fn synchronous() -> bool {
    !ASYNC_HERE.with(|here| here.get())
        && (cfg!(test) || std::env::var_os("WORLD_GPUI_PAINT_SYNC").is_some())
}

thread_local! {
    /// A test on this thread wants painting off the thread, as the app has it.
    static ASYNC_HERE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Paints off this thread even in a test: for testing just that.
#[cfg(test)]
pub(crate) fn paint_elsewhere(yes: bool) {
    ASYNC_HERE.with(|here| here.set(yes));
}

/// Whether the painter's threads have nothing left to do.
pub(crate) fn idle() -> bool {
    let pool = pool();
    pool.asked
        .lock()
        .map(|asked| asked.is_empty())
        .unwrap_or(true)
        && pool.done.lock().map(|done| done.is_empty()).unwrap_or(true)
}

thread_local! {
    static FRAME_TIME: std::cell::Cell<Duration> = const { std::cell::Cell::new(Duration::ZERO) };
}

/// Adds to how long drawing this frame has taken on the window's thread.
pub fn note_frame(took: Duration) {
    FRAME_TIME.with(|time| time.set(time.get() + took));
}

/// How long drawing has taken on the window's thread since last asked.
pub fn take_frame_time() -> Duration {
    FRAME_TIME.with(|time| time.replace(Duration::ZERO))
}

/// A thin ink line under whatever is painted: every pixel with nothing
/// `reach` pixels below it darkened by `strength`, so a building sits on
/// the ground and its eaves read as edges.
pub fn ground_line(pixmap: &mut sk::Pixmap, reach: usize, strength: f32) {
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    let alphas = pixmap
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[3])
        .collect::<Vec<_>>();
    let data = pixmap.data_mut();
    for y in 0..height {
        let below = y + reach;
        for x in 0..width {
            let at = y * width + x;
            if alphas[at] == 0 {
                continue;
            }
            // As much ink as there is open space below: a share, so a
            // sloping eave's line is as smooth as the eave.
            let under = if below >= height {
                0.0
            } else {
                alphas[below * width + x] as f32 / 255.0
            };
            let keep = 1.0 - strength.clamp(0.0, 1.0) * (1.0 - under);
            for channel in 0..3 {
                data[at * 4 + channel] = (data[at * 4 + channel] as f32 * keep) as u8;
            }
        }
    }
}

/// In tests, the longest a named part has taken at once.
pub(crate) fn slowest(name: &'static str, took: Duration) {
    #[cfg(test)]
    {
        let mut profile = profile()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entry = profile.entry(name).or_default();
        *entry = (*entry).max(took);
    }
    #[cfg(not(test))]
    let _ = (name, took);
}
