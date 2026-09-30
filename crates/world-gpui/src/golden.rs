//! Golden pictures of the diorama, the strip and the postcard, drawn
//! offscreen through GPUI's own frame and compared with PNGs kept under
//! `tests/golden/`.
//!
//! GPUI's `Window::render_to_image` needs a headless renderer, and the only
//! one GPUI has is Metal's: on Linux (and in any `cargo test` without a
//! GPU) there is none. So these tests hand GPUI's test platform a small
//! reference rasteriser of their own ([`Raster`]) through
//! `HeadlessAppContext::with_platform`. It is given the very `Scene` GPUI
//! would give Metal (every quad, shadow and path the views painted, in
//! draw order, clipped to its content mask) and fills it on the CPU the
//! way GPUI's shaders do: rounded quads with borders and linear gradients,
//! blurred shadows, paths as triangles, and images (the painter's still
//! layers) sampled as GPUI samples them. Text is not drawn (the test
//! platform's text system shapes no glyphs). What the
//! pictures prove is that the scene GPUI paints is the one we meant: a
//! change in the art, the layout or the order of layers shows as a changed
//! picture. They are not Metal's pixels; on macOS a `render_to_image`
//! through `gpui_platform::current_headless_renderer()` would give those.
//!
//! Run with `WORLD_GPUI_UPDATE_GOLDEN=1` to draw the pictures afresh after
//! a change meant to alter them, and look at them before committing.

use crate::diorama::{self, Camera, Glows};
use crate::scene::Daylight;
use gpui::{
    div, point, prelude::*, px, size, AnyElement, App, AtlasKey, AtlasTextureId, AtlasTextureKind,
    AtlasTile, Background, Bounds, Corners, DevicePixels, Edges, HeadlessAppContext, Hsla,
    NoopTextSystem, Path, PlatformAtlas, PlatformHeadlessRenderer, Rgba, ScaledPixels, Scene,
    Shadow, Size, TileId, Window,
};
use image::RgbaImage;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasMark, MarkShape, ProjectionSnapshot, SelectionId,
};

/// A channel may differ by this much and still count as the same.
const CHANNEL_TOLERANCE: u8 = 6;
/// The share of pixels that may differ, for floating-point drift between
/// machines along anti-aliased edges.
const PIXEL_TOLERANCE: f64 = 0.002;

/// Hands out atlas tiles, keeping each image's pixels (as GPUI's own
/// atlases do) so sprites can be drawn.
#[derive(Default)]
struct Atlas(Mutex<AtlasState>);

#[derive(Default)]
struct AtlasState {
    next: u32,
    tiles: HashMap<AtlasKey, AtlasTile>,
    /// Each tile's pixels: its size and straight BGRA bytes.
    pixels: HashMap<u32, (Size<DevicePixels>, Vec<u8>)>,
}

impl PlatformAtlas for Atlas {
    fn get_or_insert_with<'a>(
        &self,
        key: &AtlasKey,
        build: &mut dyn FnMut() -> gpui::Result<Option<(Size<DevicePixels>, Cow<'a, [u8]>)>>,
    ) -> gpui::Result<Option<AtlasTile>> {
        if let Some(tile) = self.0.lock().unwrap().tiles.get(key) {
            return Ok(Some(*tile));
        }
        let Some((size, bytes)) = build()? else {
            return Ok(None);
        };
        let mut state = self.0.lock().unwrap();
        state.next += 1;
        let id = state.next;
        let tile = AtlasTile {
            texture_id: AtlasTextureId {
                index: id,
                kind: AtlasTextureKind::Polychrome,
            },
            tile_id: TileId(id),
            padding: 0,
            bounds: Bounds::new(point(DevicePixels(0), DevicePixels(0)), size),
        };
        state.tiles.insert(key.clone(), tile);
        state.pixels.insert(id, (size, bytes.into_owned()));
        Ok(Some(tile))
    }

    fn remove(&self, key: &AtlasKey) {
        let mut state = self.0.lock().unwrap();
        if let Some(tile) = state.tiles.remove(key) {
            state.pixels.remove(&tile.tile_id.0);
        }
    }
}

/// The reference rasteriser GPUI's test platform draws through.
struct Raster(Arc<Atlas>);

impl PlatformHeadlessRenderer for Raster {
    fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        size: Size<DevicePixels>,
    ) -> gpui::Result<RgbaImage> {
        Ok(rasterise(scene, size, &self.0))
    }

    fn render_scene(&mut self, _: &Scene, _: Size<DevicePixels>) -> gpui::Result<()> {
        Ok(())
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.0.clone()
    }
}

/// A premultiplied colour.
#[derive(Clone, Copy, Default)]
struct Paint([f32; 4]);

impl Paint {
    fn of(colour: Hsla) -> Self {
        let Rgba { r, g, b, a } = colour.into();
        Paint([r * a, g * a, b * a, a])
    }

    fn scaled(self, by: f32) -> Self {
        Paint(self.0.map(|channel| channel * by))
    }

    fn mix(self, other: Paint, t: f32) -> Self {
        let mut out = self.0;
        for (channel, theirs) in out.iter_mut().zip(other.0) {
            *channel += (theirs - *channel) * t;
        }
        Paint(out)
    }
}

struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<[f32; 4]>,
}

impl Canvas {
    /// Lays `paint` over the pixel at `x`, `y` if it is inside `clip`.
    fn over(&mut self, x: usize, y: usize, clip: &Bounds<ScaledPixels>, paint: Paint) {
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        if fx < clip.origin.x.0
            || fy < clip.origin.y.0
            || fx > clip.origin.x.0 + clip.size.width.0
            || fy > clip.origin.y.0 + clip.size.height.0
        {
            return;
        }
        let below = &mut self.pixels[y * self.width + x];
        let keep = 1.0 - paint.0[3];
        for (channel, above) in below.iter_mut().zip(paint.0) {
            *channel = above + *channel * keep;
        }
    }

    /// The pixels `bounds` touches, clamped to the canvas.
    fn span(&self, bounds: &Bounds<ScaledPixels>) -> (usize, usize, usize, usize) {
        let clamp = |value: f32, most: usize| value.max(0.0).min(most as f32) as usize;
        (
            clamp(bounds.origin.x.0.floor(), self.width),
            clamp(bounds.origin.y.0.floor(), self.height),
            clamp((bounds.origin.x.0 + bounds.size.width.0).ceil(), self.width),
            clamp(
                (bounds.origin.y.0 + bounds.size.height.0).ceil(),
                self.height,
            ),
        )
    }
}

fn rasterise(scene: &Scene, size: Size<DevicePixels>, atlas: &Atlas) -> RgbaImage {
    let (width, height) = (size.width.0.max(0) as usize, size.height.0.max(0) as usize);
    let mut canvas = Canvas {
        width,
        height,
        pixels: vec![[0.0; 4]; width * height],
    };
    // In draw order, and within one order in the order GPUI batches kinds.
    enum Primitive<'a> {
        Shadow(&'a Shadow),
        Quad(&'a gpui::Quad),
        Path(&'a Path<ScaledPixels>),
        Sprite(&'a gpui::PolychromeSprite),
    }
    let mut all = Vec::new();
    all.extend(
        scene
            .shadows
            .iter()
            .map(|s| (s.order, 0, Primitive::Shadow(s))),
    );
    all.extend(scene.quads.iter().map(|q| (q.order, 1, Primitive::Quad(q))));
    all.extend(scene.paths.iter().map(|p| (p.order, 2, Primitive::Path(p))));
    all.extend(
        scene
            .polychrome_sprites
            .iter()
            .map(|sprite| (sprite.order, 3, Primitive::Sprite(sprite))),
    );
    all.sort_by_key(|(order, kind, _)| (*order, *kind));
    for (_, _, primitive) in all {
        match primitive {
            Primitive::Shadow(shadow) => draw_shadow(&mut canvas, shadow),
            Primitive::Quad(quad) => draw_quad(&mut canvas, quad),
            Primitive::Path(path) => draw_path(&mut canvas, path),
            Primitive::Sprite(sprite) => draw_sprite(&mut canvas, sprite, atlas),
        }
    }
    let mut image = RgbaImage::new(width as u32, height as u32);
    for (pixel, paint) in image.pixels_mut().zip(&canvas.pixels) {
        let alpha = paint[3].clamp(0.0, 1.0);
        let straight = |channel: f32| {
            let value = if alpha > 0.0 { channel / alpha } else { 0.0 };
            (value.clamp(0.0, 1.0) * 255.0).round() as u8
        };
        *pixel = image::Rgba([
            straight(paint[0]),
            straight(paint[1]),
            straight(paint[2]),
            (alpha * 255.0).round() as u8,
        ]);
    }
    image
}

/// An image: its tile's pixels stretched over its bounds, sampled
/// bilinearly as GPUI's shaders sample them, clipped to its content mask.
fn draw_sprite(canvas: &mut Canvas, sprite: &gpui::PolychromeSprite, atlas: &Atlas) {
    let state = atlas.0.lock().unwrap();
    let Some((size, bytes)) = state.pixels.get(&sprite.tile.tile_id.0) else {
        return;
    };
    let (tw, th) = (size.width.0.max(1) as usize, size.height.0.max(1) as usize);
    let tile = sprite.tile.bounds;
    let (sx0, sy0) = (tile.origin.x.0 as f32, tile.origin.y.0 as f32);
    let (sw, sh) = (tile.size.width.0 as f32, tile.size.height.0 as f32);
    let bounds = sprite.bounds;
    let (x0, y0, x1, y1) = canvas.span(&bounds);
    let texel = |x: usize, y: usize| {
        let at = (y.min(th - 1) * tw + x.min(tw - 1)) * 4;
        let a = bytes[at + 3] as f32 / 255.0;
        // Straight BGRA to premultiplied RGBA.
        [
            bytes[at + 2] as f32 / 255.0 * a,
            bytes[at + 1] as f32 / 255.0 * a,
            bytes[at] as f32 / 255.0 * a,
            a,
        ]
    };
    for y in y0..y1 {
        for x in x0..x1 {
            let u = (x as f32 + 0.5 - bounds.origin.x.0) / bounds.size.width.0.max(1e-3);
            let v = (y as f32 + 0.5 - bounds.origin.y.0) / bounds.size.height.0.max(1e-3);
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                continue;
            }
            let fx = (sx0 + u * sw - 0.5).max(0.0);
            let fy = (sy0 + v * sh - 0.5).max(0.0);
            let (ix, iy) = (fx.floor() as usize, fy.floor() as usize);
            let (tx, ty) = (fx - ix as f32, fy - iy as f32);
            let (a, b, c, d) = (
                texel(ix, iy),
                texel(ix + 1, iy),
                texel(ix, iy + 1),
                texel(ix + 1, iy + 1),
            );
            let mut paint = [0.0; 4];
            for channel in 0..4 {
                let top = a[channel] + (b[channel] - a[channel]) * tx;
                let bottom = c[channel] + (d[channel] - c[channel]) * tx;
                paint[channel] = (top + (bottom - top) * ty) * sprite.opacity;
            }
            canvas.over(x, y, &sprite.content_mask.bounds, Paint(paint));
        }
    }
}

/// How far a point is outside a rounded rectangle (negative inside), as
/// GPUI's `quad_sdf` has it.
fn quad_sdf(
    (x, y): (f32, f32),
    bounds: &Bounds<ScaledPixels>,
    radii: &Corners<ScaledPixels>,
) -> f32 {
    let half = (bounds.size.width.0 / 2.0, bounds.size.height.0 / 2.0);
    let to = (
        x - (bounds.origin.x.0 + half.0),
        y - (bounds.origin.y.0 + half.1),
    );
    let radius = match (to.0 < 0.0, to.1 < 0.0) {
        (true, true) => radii.top_left.0,
        (false, true) => radii.top_right.0,
        (true, false) => radii.bottom_left.0,
        (false, false) => radii.bottom_right.0,
    };
    let corner = (to.0.abs() - half.0 + radius, to.1.abs() - half.1 + radius);
    if radius == 0.0 {
        return corner.0.max(corner.1);
    }
    (corner.0.max(0.0)).hypot(corner.1.max(0.0)) + corner.0.max(corner.1).min(0.0) - radius
}

/// A background worked out once for a primitive: solid, or a linear
/// gradient between two stops.
enum Fill {
    Solid(Paint),
    Gradient {
        angle: f32,
        from: Rgba,
        to: Rgba,
        start: f32,
        end: f32,
    },
}

impl Fill {
    fn of(background: &Background) -> Self {
        if let Some(solid) = background.as_solid() {
            return Fill::Solid(Paint::of(solid));
        }
        // The gradient's parts are GPUI's own; its serialised form shows
        // them (colours as `#rrggbbaa`).
        let value = serde_json::to_value(background).expect("a background serialises");
        let rgba = |value: &serde_json::Value| -> Rgba {
            serde_json::from_value(value.clone()).unwrap_or_default()
        };
        if value["tag"] != "LinearGradient" {
            return Fill::Solid(Paint::of(rgba(&value["solid"]).into()));
        }
        let stops = &value["colors"];
        Fill::Gradient {
            angle: value["gradient_angle_or_pattern_height"]
                .as_f64()
                .unwrap_or(0.0) as f32,
            from: rgba(&stops[0]["color"]),
            to: rgba(&stops[1]["color"]),
            start: stops[0]["percentage"].as_f64().unwrap_or(0.0) as f32,
            end: stops[1]["percentage"].as_f64().unwrap_or(1.0) as f32,
        }
    }

    /// The colour at a point of `bounds`, the way GPUI's `fill_color`
    /// works it out.
    fn at(&self, bounds: &Bounds<ScaledPixels>, (x, y): (f32, f32)) -> Paint {
        let Fill::Gradient {
            angle,
            from: a,
            to: b,
            start,
            end,
        } = *self
        else {
            let Fill::Solid(paint) = self else {
                unreachable!()
            };
            return *paint;
        };
        let radians = ((angle % 360.0) - 90.0).to_radians();
        let (w, h) = (bounds.size.width.0, bounds.size.height.0);
        let mut direction = (radians.cos(), radians.sin());
        if w > h {
            direction.1 *= h / w;
        } else {
            direction.0 *= w / h;
        }
        let to_point = (
            x - (bounds.origin.x.0 + w / 2.0),
            y - (bounds.origin.y.0 + h / 2.0),
        );
        let length = direction.0.hypot(direction.1);
        let mut t = (to_point.0 * direction.0 + to_point.1 * direction.1) / length;
        t = if direction.0.abs() > direction.1.abs() {
            (t + w / 2.0) / w
        } else {
            (t + h / 2.0) / h
        };
        let t = ((t - start) / (end - start)).clamp(0.0, 1.0);
        // GPUI mixes straight sRGB colours, then premultiplies.
        let mix = |p: f32, q: f32| p + (q - p) * t;
        let alpha = mix(a.a, b.a);
        Paint([
            mix(a.r, b.r) * alpha,
            mix(a.g, b.g) * alpha,
            mix(a.b, b.b) * alpha,
            alpha,
        ])
    }
}

fn draw_quad(canvas: &mut Canvas, quad: &gpui::Quad) {
    let bounds = quad.bounds;
    let Edges {
        top,
        right,
        bottom,
        left,
    } = quad.border_widths;
    let bordered = [top, right, bottom, left].iter().any(|width| width.0 > 0.0);
    let inner = Bounds::new(
        point(bounds.origin.x + left, bounds.origin.y + top),
        size(
            bounds.size.width - left - right,
            bounds.size.height - top - bottom,
        ),
    );
    let widest = top.0.max(right.0).max(bottom.0).max(left.0);
    let shrink = |radius: ScaledPixels| ScaledPixels((radius.0 - widest).max(0.0));
    let inner_radii = Corners {
        top_left: shrink(quad.corner_radii.top_left),
        top_right: shrink(quad.corner_radii.top_right),
        bottom_right: shrink(quad.corner_radii.bottom_right),
        bottom_left: shrink(quad.corner_radii.bottom_left),
    };
    let border = Paint::of(quad.border_color);
    let background = Fill::of(&quad.background);
    let (x0, y0, x1, y1) = canvas.span(&bounds);
    for y in y0..y1 {
        for x in x0..x1 {
            let at = (x as f32 + 0.5, y as f32 + 0.5);
            let outside = quad_sdf(at, &bounds, &quad.corner_radii);
            let coverage = (0.5 - outside).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            let mut paint = background.at(&bounds, at);
            if bordered {
                let inside = (0.5 - quad_sdf(at, &inner, &inner_radii)).clamp(0.0, 1.0);
                paint = border.mix(paint, inside);
            }
            canvas.over(x, y, &quad.content_mask.bounds, paint.scaled(coverage));
        }
    }
}

/// GPUI's approximation of the error function.
fn erf(x: f32) -> f32 {
    let a = x.abs();
    let r1 = 1.0 + (0.278393 + (0.230389 + (0.000972 + 0.078108 * a) * a) * a) * a;
    let r2 = r1 * r1;
    x.signum() - x.signum() / (r2 * r2)
}

fn draw_shadow(canvas: &mut Canvas, shadow: &Shadow) {
    let bounds = shadow.bounds;
    let half = (bounds.size.width.0 / 2.0, bounds.size.height.0 / 2.0);
    let centre = (bounds.origin.x.0 + half.0, bounds.origin.y.0 + half.1);
    let sigma = shadow.blur_radius.0;
    let colour = Paint::of(shadow.color);
    // Blurred, a shadow spreads three sigmas past its rectangle.
    let spread = Bounds::new(
        point(
            bounds.origin.x - ScaledPixels(3.0 * sigma),
            bounds.origin.y - ScaledPixels(3.0 * sigma),
        ),
        size(
            bounds.size.width + ScaledPixels(6.0 * sigma),
            bounds.size.height + ScaledPixels(6.0 * sigma),
        ),
    );
    let (x0, y0, x1, y1) = canvas.span(&spread);
    for y in y0..y1 {
        for x in x0..x1 {
            let at = (x as f32 + 0.5, y as f32 + 0.5);
            let to = (at.0 - centre.0, at.1 - centre.1);
            let corner = match (to.0 < 0.0, to.1 < 0.0) {
                (true, true) => shadow.corner_radii.top_left.0,
                (false, true) => shadow.corner_radii.top_right.0,
                (true, false) => shadow.corner_radii.bottom_left.0,
                (false, false) => shadow.corner_radii.bottom_right.0,
            };
            let mut alpha = if sigma == 0.0 {
                (0.5 - quad_sdf(at, &bounds, &shadow.corner_radii)).clamp(0.0, 1.0)
            } else {
                let low = to.1 - half.1;
                let high = to.1 + half.1;
                let start = (-3.0 * sigma).clamp(low, high);
                let end = (3.0 * sigma).clamp(low, high);
                let step = (end - start) / 4.0;
                let mut sample = start + step * 0.5;
                let mut sum = 0.0;
                for _ in 0..4 {
                    let dy = to.1 - sample;
                    let delta = (half.1 - corner - dy.abs()).min(0.0);
                    let curved =
                        half.0 - corner + (corner * corner - delta * delta).max(0.0).sqrt();
                    let scale = std::f32::consts::FRAC_1_SQRT_2 / sigma;
                    let along = 0.5 * (erf((to.0 + curved) * scale) - erf((to.0 - curved) * scale));
                    let gaussian = (-(sample * sample) / (2.0 * sigma * sigma)).exp()
                        / ((2.0 * std::f32::consts::PI).sqrt() * sigma);
                    sum += along * gaussian * step;
                    sample += step;
                }
                sum
            };
            if shadow.inset != 0 {
                let element = quad_sdf(at, &shadow.element_bounds, &shadow.element_corner_radii);
                alpha = (1.0 - alpha) * (0.5 - element).clamp(0.0, 1.0);
            }
            if alpha > 0.0 {
                canvas.over(
                    x,
                    y,
                    &shadow.content_mask.bounds,
                    colour.scaled(alpha.min(1.0)),
                );
            }
        }
    }
}

/// A path is triangles: flat ones for what `PathBuilder` tessellates, and
/// curved ones (Loop–Blinn: inside where s² − t ≤ 0) for `Path::curve_to`.
/// Its triangles are drawn into one layer first, as GPUI does, and the
/// layer is then laid over the picture; four samples a pixel smooth its
/// edges.
fn draw_path(canvas: &mut Canvas, path: &Path<ScaledPixels>) {
    const SAMPLES: [(f32, f32); 4] = [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];
    let (x0, y0, x1, y1) = canvas.span(&path.bounds);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let columns = x1 - x0;
    let mut covered = vec![0u8; columns * (y1 - y0)];
    for triangle in path.vertices.as_chunks::<3>().0 {
        let p = triangle
            .iter()
            .map(|v| (v.xy_position.x.0, v.xy_position.y.0))
            .collect::<Vec<_>>();
        let st = triangle
            .iter()
            .map(|v| (v.st_position.x, v.st_position.y))
            .collect::<Vec<_>>();
        let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
        if area.abs() < 1e-6 {
            continue;
        }
        let low_x = p
            .iter()
            .map(|q| q.0)
            .fold(f32::MAX, f32::min)
            .floor()
            .max(x0 as f32) as usize;
        let high_x = p
            .iter()
            .map(|q| q.0)
            .fold(f32::MIN, f32::max)
            .ceil()
            .min(x1 as f32) as usize;
        let low_y = p
            .iter()
            .map(|q| q.1)
            .fold(f32::MAX, f32::min)
            .floor()
            .max(y0 as f32) as usize;
        let high_y = p
            .iter()
            .map(|q| q.1)
            .fold(f32::MIN, f32::max)
            .ceil()
            .min(y1 as f32) as usize;
        for y in low_y..high_y {
            for x in low_x..high_x {
                for (bit, (dx, dy)) in SAMPLES.iter().enumerate() {
                    let (sx, sy) = (x as f32 + dx, y as f32 + dy);
                    let w0 = ((p[1].0 - sx) * (p[2].1 - sy) - (p[2].0 - sx) * (p[1].1 - sy)) / area;
                    let w1 = ((p[2].0 - sx) * (p[0].1 - sy) - (p[0].0 - sx) * (p[2].1 - sy)) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let s = w0 * st[0].0 + w1 * st[1].0 + w2 * st[2].0;
                    let t = w0 * st[0].1 + w1 * st[1].1 + w2 * st[2].1;
                    if s * s - t <= 0.0 {
                        covered[(y - y0) * columns + (x - x0)] |= 1 << bit;
                    }
                }
            }
        }
    }
    let colour = Fill::of(&path.color);
    for y in y0..y1 {
        for x in x0..x1 {
            let samples = covered[(y - y0) * columns + (x - x0)].count_ones();
            if samples == 0 {
                continue;
            }
            let at = (x as f32 + 0.5, y as f32 + 0.5);
            let paint = colour.at(&path.bounds, at).scaled(samples as f32 / 4.0);
            canvas.over(x, y, &path.content_mask.bounds, paint);
        }
    }
}

/// A view that draws whatever it is given.
struct Picture(Box<dyn Fn() -> AnyElement>);

impl gpui::Render for Picture {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div().size_full().child((self.0)())
    }
}

/// Draws `picture` in a window `width` by `height` through GPUI and the
/// reference rasteriser.
fn draw(width: f32, height: f32, picture: impl Fn() -> AnyElement + 'static) -> RgbaImage {
    let mut cx =
        HeadlessAppContext::with_platform(Arc::new(NoopTextSystem::new()), Arc::new(()), || {
            Some(Box::new(Raster(Arc::default())) as Box<dyn PlatformHeadlessRenderer>)
        });
    let window = cx
        .open_window(size(px(width), px(height)), |_, cx: &mut App| {
            cx.new(|_| Picture(Box::new(picture)))
        })
        .expect("a window");
    cx.run_until_parked();
    cx.capture_screenshot(window.into()).expect("a picture")
}

/// Compares `image` with the golden picture `name`, or keeps it as the
/// golden picture when asked to.
fn matches_golden(name: &str, image: &RgbaImage) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.png"));
    if std::env::var_os("WORLD_GPUI_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image.save(&path).expect("the golden picture is written");
        return;
    }
    let golden = image::open(&path)
        .unwrap_or_else(|error| {
            panic!(
                "no golden picture at {} ({error}); draw it with WORLD_GPUI_UPDATE_GOLDEN=1",
                path.display()
            )
        })
        .to_rgba8();
    let drawn = std::env::temp_dir().join(format!("world-gpui-{name}.png"));
    assert_eq!(
        golden.dimensions(),
        image.dimensions(),
        "{name} changed size"
    );
    let differing = golden
        .pixels()
        .zip(image.pixels())
        .filter(|(want, got)| {
            want.0
                .iter()
                .zip(got.0)
                .any(|(a, b)| a.abs_diff(b) > CHANNEL_TOLERANCE)
        })
        .count();
    let share = differing as f64 / (golden.width() * golden.height()) as f64;
    if share > PIXEL_TOLERANCE {
        let _ = image.save(&drawn);
        panic!(
            "{name} differs from its golden picture in {differing} pixels ({:.2}%); \
             drawn now: {}; if the change is meant, redraw with WORLD_GPUI_UPDATE_GOLDEN=1",
            share * 100.0,
            drawn.display()
        );
    }
}

fn id(n: u64) -> SelectionId {
    SelectionId::from_stable_key(&format!("entity-{n}")).expect("an entity key")
}

fn item(n: u64, kind: CanvasItemKind, label: &str, x: f32, at: Option<u64>) -> CanvasItem {
    CanvasItem {
        id: id(n),
        kind,
        label: label.into(),
        detail: String::new(),
        x,
        y: 0.5,
        changes: Vec::new(),
        shape: None,
        at: at.map(id),
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
        ..Default::default()
    }
}

/// A small harbour town: four places, four people, three things built.
fn town() -> ProjectionSnapshot {
    let places = ["Harbor", "Bakery", "Square", "Lighthouse"];
    let people = ["Mara Quinn", "Leo Park", "Nia Chen", "Evan Moss"];
    let mut snapshot = ProjectionSnapshot {
        title: "Harbor Town".into(),
        ..ProjectionSnapshot::default()
    };
    for (index, place) in places.iter().enumerate() {
        snapshot.canvas.items.push(item(
            100 + index as u64,
            CanvasItemKind::Place,
            place,
            index as f32 / 4.0,
            None,
        ));
    }
    for (index, person) in people.iter().enumerate() {
        snapshot.canvas.items.push(item(
            1 + index as u64,
            CanvasItemKind::Actor,
            person,
            index as f32 / 4.0,
            Some(100 + index as u64),
        ));
    }
    for shape in [MarkShape::House, MarkShape::Tree, MarkShape::Lamp] {
        snapshot.canvas.marks.push(CanvasMark {
            label: format!("{shape:?}"),
            shape,
            selection: None,
        });
    }
    snapshot.letters.push(world_projection::Letter {
        from: id(1),
        note: "The pier is mended.".into(),
        moment: id(1),
    });
    snapshot
}

/// A harbour on the water: the town with a boat at the quay, one of its
/// people at home indoors from nine at night, and the others out late.
fn harbour() -> ProjectionSnapshot {
    let mut snapshot = town();
    snapshot.scenery = Some(world_projection::Scenery {
        sky_top: 0x9cc6e6,
        sky_bottom: 0xf0ead8,
        far: 0x8fae7e,
        near: 0x4f86a8,
        sun: 0xffe2a0,
    });
    let mut boat = item(200, CanvasItemKind::Object, "Boat", 0.0, Some(100));
    boat.shape = Some(MarkShape::Boat);
    snapshot.canvas.items.push(boat);
    let mut tree = item(105, CanvasItemKind::Place, "Orchard", 0.9, None);
    tree.shape = Some(MarkShape::Tree);
    snapshot.canvas.items.push(tree);
    let home = id(101);
    let work = id(102);
    if let Some(leo) = snapshot
        .canvas
        .items
        .iter_mut()
        .find(|item| item.id == id(2))
    {
        leo.day = vec![
            world_projection::RoutineStop {
                from_hour: 7,
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
    snapshot
}

/// The diorama of `snapshot`, `width` by `height`, at a pinned moment.
fn diorama_frame(
    snapshot: &ProjectionSnapshot,
    width: f32,
    height: f32,
    daylight: Daylight,
    hour: f32,
) -> diorama::Frame {
    let stage = diorama::stage_at(
        snapshot,
        width,
        height,
        diorama::Clock::at(hour.floor() as u8),
    );
    let living = diorama::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
    diorama::frame(
        snapshot,
        &stage,
        &living,
        Camera::whole(&stage),
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    )
    .at_hour(hour)
}

fn painted(frame: diorama::Frame) -> AnyElement {
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| diorama::paint(&frame, bounds, window),
    )
    .size_full()
    .into_any_element()
}

#[test]
fn the_diorama_by_day_matches_its_golden_picture() {
    let frame = diorama_frame(&town(), 480.0, 300.0, Daylight::Day, 13.0);
    let image = draw(480.0, 300.0, move || painted(frame.clone()));
    let bare = image.pixels().filter(|pixel| pixel.0[3] < 255).count();
    assert!(bare == 0, "the diorama leaves {bare} pixels bare");
    matches_golden("diorama-day", &image);
}

#[test]
fn the_diorama_at_dusk_matches_its_golden_picture() {
    let frame = diorama_frame(&town(), 480.0, 300.0, Daylight::Dusk, 19.5);
    let image = draw(480.0, 300.0, move || painted(frame.clone()));
    matches_golden("diorama-dusk", &image);
}

/// The harbour at night: windows lit with their glow, someone at home
/// seen in one, and nobody else about but the night owls.
#[test]
fn the_diorama_at_night_matches_its_golden_picture() {
    let frame = diorama_frame(&harbour(), 480.0, 300.0, Daylight::Night, 23.0);
    let image = draw(480.0, 300.0, move || painted(frame.clone()));
    matches_golden("diorama-night", &image);
}

/// Each season on the harbour: blossom in spring, full summer, leaves in
/// autumn, and in winter snow on the roofs and the ground and ice at the
/// water's edge.
#[test]
fn each_season_matches_its_golden_picture() {
    use world_projection::{GroundCover, Season};
    for (name, season, cover, ice) in [
        (
            "season-spring",
            Season::Spring,
            Some(GroundCover::Blossom),
            false,
        ),
        ("season-summer", Season::Summer, None, false),
        (
            "season-autumn",
            Season::Autumn,
            Some(GroundCover::Leaves),
            false,
        ),
        (
            "season-winter",
            Season::Winter,
            Some(GroundCover::Snow),
            true,
        ),
    ] {
        let mut snapshot = harbour();
        snapshot.canvas.season = Some(season);
        snapshot.canvas.ground = cover;
        snapshot.canvas.ice = ice;
        let frame = diorama_frame(&snapshot, 480.0, 300.0, Daylight::Day, 11.0);
        let image = draw(480.0, 300.0, move || painted(frame.clone()));
        matches_golden(name, &image);
    }
}

/// A place three windows wide, panned to its far end: the camera follows
/// `px`, and the far hills move less than the ground.
#[test]
fn a_panorama_panned_along_matches_its_golden_picture() {
    let mut snapshot = harbour();
    snapshot.canvas.width = Some(3.0);
    let places = snapshot
        .canvas
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.kind != CanvasItemKind::Actor)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    for (n, index) in places.iter().enumerate() {
        snapshot.canvas.items[*index].px = Some(0.35 + n as f32 * 0.62);
    }
    let (width, height) = (480.0, 300.0);
    let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(13));
    assert_eq!(stage.width, width * 3.0);
    let living = diorama::living(
        &stage,
        &snapshot,
        0.0,
        Daylight::Day,
        &Default::default(),
        None,
    );
    let camera = Camera::around(&stage, 1.0, stage.width, height / 2.0);
    assert!((camera.x - (stage.width - width / 2.0)).abs() < 0.01);
    let frame = diorama::frame(
        &snapshot,
        &stage,
        &living,
        camera,
        0.0,
        Daylight::Day,
        &Glows::new(),
        1.0,
    )
    .at_hour(15.0);
    let image = draw(width, height, move || painted(frame.clone()));
    matches_golden("panorama-east", &image);
}

/// A view showing the World window's scene: still layers painted off the
/// window's thread.
struct SceneView(diorama::Frame);

impl gpui::Render for SceneView {
    fn render(&mut self, window: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .child(diorama::scene(self.0.clone(), window))
    }
}

/// The World window never waits for a picture: its first frame is drawn at
/// once with stand-ins (the sky's colours, the land's), the still layers
/// arrive from the painter's threads and fade in, and once they have the
/// scene is the same picture as one painted all at once.
#[test]
fn the_scene_draws_at_once_and_its_still_layers_arrive_and_fade_in() {
    crate::painter::paint_elsewhere(true);
    let frame = diorama_frame(&town(), 480.0, 300.0, Daylight::Day, 13.0);
    let mut cx =
        HeadlessAppContext::with_platform(Arc::new(NoopTextSystem::new()), Arc::new(()), || {
            Some(Box::new(Raster(Arc::default())) as Box<dyn PlatformHeadlessRenderer>)
        });
    let started = std::time::Instant::now();
    let window = cx
        .open_window(size(px(480.0), px(300.0)), move |_, cx: &mut App| {
            cx.new(|_| SceneView(frame.clone()))
        })
        .expect("a window");
    cx.run_until_parked();
    let first = cx.capture_screenshot(window.into()).expect("a picture");
    let bare = first.pixels().filter(|pixel| pixel.0[3] < 255).count();
    assert_eq!(bare, 0, "the first frame is whole, with stand-ins");
    let _ = started;
    // The still layers arrive, then fade in over a third of a second.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut settled = None;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(40));
        cx.update_window(window.into(), |_, window, _| window.refresh())
            .expect("a window");
        cx.run_until_parked();
        if crate::painter::idle() {
            std::thread::sleep(std::time::Duration::from_millis(450));
            cx.update_window(window.into(), |_, window, _| window.refresh())
                .expect("a window");
            cx.run_until_parked();
            cx.update_window(window.into(), |_, window, _| window.refresh())
                .expect("a window");
            cx.run_until_parked();
            settled = Some(cx.capture_screenshot(window.into()).expect("a picture"));
            break;
        }
    }
    crate::painter::paint_elsewhere(false);
    let settled = settled.expect("the still layers arrive");
    matches_golden("diorama-day", &settled);
}

/// How long this thread has been running on a CPU, so a frame is timed by
/// its own work and not by the other programs a busy machine is running
/// meanwhile.
fn on_cpu() -> std::time::Duration {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `now` is a valid timespec for clock_gettime to fill in.
    let status = unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut now) };
    if status != 0 {
        return std::time::Duration::ZERO;
    }
    std::time::Duration::new(now.tv_sec as u64, now.tv_nsec as u32)
}

/// A moment strip over a scene: the snapshot it is drawn from, and the
/// moment.
type StripOver = Option<(ProjectionSnapshot, world_projection::Moment)>;

/// A view of a scene whose frame the test can change between frames, with
/// a moment's strip over it when the test puts one there.
struct LiveScene(
    std::rc::Rc<std::cell::RefCell<diorama::Frame>>,
    std::rc::Rc<std::cell::RefCell<StripOver>>,
);

impl gpui::Render for LiveScene {
    fn render(&mut self, window: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let frame = self.0.borrow().clone();
        let (width, height) = (frame.width_of_view(), frame.height_of_view());
        let started = std::time::Instant::now();
        let strip = self.1.borrow().as_ref().map(|(snapshot, moment)| {
            let layout = crate::macos::strip_layout(width, height);
            div()
                .absolute()
                .left(px(layout.x))
                .top(px(layout.y))
                .child(crate::macos::moment_strip(snapshot, moment, layout))
        });
        crate::painter::note_frame(started.elapsed());
        div()
            .size_full()
            .relative()
            .child(diorama::scene(frame, window))
            .children(strip)
    }
}

/// The v0.21 bar for hitches: at 1440 by 900 at twice the pixels, a
/// three-year World's window never spends more than 16 ms on a frame
/// because of painting, whether it is opening, the hour turning or the
/// view panning into places not yet painted: all of that is painted off
/// the window's thread and faded in. Timed in a release build.
#[test]
#[ignore = "a benchmark: cargo test --release -p world-gpui -- --ignored never_waits"]
fn a_three_year_world_never_waits_for_painting() {
    use std::time::{Duration, Instant};
    crate::painter::paint_elsewhere(true);
    let snapshot = crate::diorama::tests::three_years();
    let (width, height) = (1440.0_f32, 900.0_f32);
    let make_of = |snapshot: &ProjectionSnapshot, hour: f32, pan: f32, daylight: Daylight| {
        let stage = diorama::stage_at(snapshot, width, height, diorama::Clock::at(hour as u8));
        let living = diorama::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
        let camera = Camera::around(&stage, 1.0, stage.width / 2.0 + pan, height / 2.0);
        diorama::frame(
            snapshot,
            &stage,
            &living,
            camera,
            0.0,
            daylight,
            &Glows::new(),
            1.0,
        )
        .at_hour(hour)
    };
    let make = |hour: f32, pan: f32, daylight: Daylight| make_of(&snapshot, hour, pan, daylight);
    let shared = std::rc::Rc::new(std::cell::RefCell::new(make(12.0, 0.0, Daylight::Day)));
    let over = std::rc::Rc::new(std::cell::RefCell::new(None));
    let strip = over.clone();
    let mut cx =
        HeadlessAppContext::with_platform(Arc::new(NoopTextSystem::new()), Arc::new(()), || {
            Some(Box::new(Raster(Arc::default())) as Box<dyn PlatformHeadlessRenderer>)
        });
    let view = shared.clone();
    let started = on_cpu();
    let window = cx
        .open_window(size(px(width), px(height)), move |_, cx: &mut App| {
            cx.new(|_| LiveScene(view.clone(), strip.clone()))
        })
        .expect("a window");
    let opening = on_cpu().saturating_sub(started);
    let mut worst = opening;
    let mut frames = 0;
    let settle = |cx: &mut HeadlessAppContext, worst: &mut Duration, frames: &mut u32| {
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut quiet = 0;
        while Instant::now() < deadline && quiet < 40 {
            std::thread::sleep(Duration::from_millis(8));
            let ours = || {
                let profile = crate::painter::profile().lock().unwrap();
                ["main: plan", "main: draw still"]
                    .map(|part| profile.get(part).copied().unwrap_or_default())
            };
            let before = ours();
            let (started, cpu) = (Instant::now(), on_cpu());
            cx.update_window(window.into(), |_, window, _| window.refresh())
                .expect("a window");
            let (wall, took) = (started.elapsed(), on_cpu().saturating_sub(cpu));
            let after = ours();
            if took > Duration::from_millis(16) || wall > Duration::from_millis(16) {
                eprintln!(
                    "a slow frame, {took:?} on the CPU ({wall:?} by the clock): planning {:?}, \
                     drawing the still layers {:?}",
                    after[0] - before[0],
                    after[1] - before[1]
                );
            }
            *worst = (*worst).max(took);
            *frames += 1;
            quiet = if crate::painter::idle() { quiet + 1 } else { 0 };
        }
    };
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "opening: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    // The hour turns to dusk: every still layer is painted again.
    *shared.borrow_mut() = make(19.5, 0.0, Daylight::Dusk);
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "dusk: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    // The view pans a window and a half along, into tiles not painted.
    *shared.borrow_mut() = make(19.5, width * 1.5, Daylight::Dusk);
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "panned: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    // A moment comes: its three panels are painted off the window's thread
    // and fade in over the scene.
    use world_projection::MomentKind;
    *over.borrow_mut() = Some((
        snapshot.clone(),
        moment_of(
            MomentKind::Wedding,
            "A wedding",
            [&[1, 2], &[1, 2, 3], &[1, 2, 3, 4]],
        ),
    ));
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "wedding strip: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    *over.borrow_mut() = Some((
        snapshot.clone(),
        moment_of(MomentKind::Birth, "A birth", [&[5, 6], &[5, 6], &[5, 6, 7]]),
    ));
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "birth strip: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    // The player's designs: two sails, a flag, two shop signs and a quilt
    // in view, each painted off the window's thread, then the hour turning
    // under them and the wind moving them.
    *over.borrow_mut() = None;
    let mut designed = snapshot.clone();
    let design = motif_of(&[9, 0, 2, 4], |x, y| {
        (x / 4 + y / 4) % 3 + usize::from(x == y)
    });
    for item in &mut designed.canvas.items {
        let key = item.id.stable_key();
        match key.as_str() {
            "entity-309" | "entity-312" => item.shape = Some(MarkShape::Boat),
            "entity-310" => item.shape = Some(MarkShape::Flag),
            "entity-120" | "entity-126" => item.shape = Some(MarkShape::Shop),
            "entity-118" => item.shape = Some(MarkShape::House),
            _ => continue,
        }
        item.pattern = Some(design.clone());
    }
    let frame = make_of(&designed, 19.5, 0.0, Daylight::Dusk);
    assert_eq!(frame.worn().len(), 6, "six designs are worn");
    *shared.borrow_mut() = frame;
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "six designs: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    *shared.borrow_mut() = make_of(&designed, 21.0, 0.0, Daylight::Night);
    let mut phase = Duration::ZERO;
    settle(&mut cx, &mut phase, &mut frames);
    eprintln!(
        "six designs at night: the longest frame {:.2} ms",
        phase.as_secs_f64() * 1000.0
    );
    worst = worst.max(phase);
    crate::painter::paint_elsewhere(false);
    eprintln!("{:?}", crate::painter::profile().lock().unwrap());
    eprintln!(
        "{frames} frames; the longest on the window's thread {:.2} ms of its own work \
         (opening {:.2} ms)",
        worst.as_secs_f64() * 1000.0,
        opening.as_secs_f64() * 1000.0
    );
    if !cfg!(debug_assertions) {
        assert!(worst < Duration::from_millis(16), "{worst:?}");
    }
}

#[test]
fn the_strip_matches_its_golden_picture() {
    use crate::strip;
    let snapshot = town();
    let (width, height) = (720.0, strip::HEIGHT);
    let stage = strip::stage(&snapshot, width, height);
    let outings = strip::Outings::new(&stage, &snapshot);
    let frame = strip::frame(&snapshot, &stage, &outings, 0.0, Daylight::Day, false).at_hour(13.0);
    let image = draw(width, height, move || {
        div()
            .size_full()
            .relative()
            .child(painted(frame.clone()))
            .child(strip::letter_button(
                "From Mara".into(),
                false,
                width - 56.0,
                80.0,
            ))
            .child(strip::close_button(gpui::black()))
            .into_any_element()
    });
    matches_golden("strip", &image);
}

#[test]
fn the_postcard_matches_its_golden_picture() {
    let snapshot = town();
    let card = crate::postcard::postcard(&snapshot, None);
    let (width, height) = (480.0, 360.0);
    let frame = diorama_frame(&snapshot, width, height, Daylight::Day, 13.0);
    let image = draw(width, height, move || {
        div()
            .size_full()
            .relative()
            .child(painted(frame.clone()))
            .child(crate::macos::postcard_paper(&card, width, height))
            .into_any_element()
    });
    matches_golden("postcard", &image);
}

/// A gentle pointer under a handle in the top right, and the zoom control
/// with its own pointer beside it, over the scene: where each sits, its
/// little arrow, and the accent edge that sets it apart from a card.
#[test]
fn the_pointers_and_the_zoom_control_match_their_golden_picture() {
    use crate::macos::{pointer_hint, zoom_button, Caret};
    use crate::pointers::Pointer;
    let snapshot = town();
    let (width, height) = (720.0, 420.0);
    let frame = diorama_frame(&snapshot, width, height, Daylight::Day, 13.0);
    let image = draw(width, height, move || {
        div()
            .size_full()
            .relative()
            .child(painted(frame.clone()))
            .child(
                div()
                    .absolute()
                    .top(px(56.0))
                    .right(px(16.0))
                    .child(pointer_hint(Pointer::Drawer, Caret::Up, |_, _, _| {})),
            )
            .child(
                div()
                    .absolute()
                    .left(px(16.0))
                    .top(px(160.0))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_1()
                            .rounded_full()
                            .bg(gpui::white())
                            .child(zoom_button("zoom-in", "+", "Zoom in (+)", false))
                            .child(zoom_button("zoom-out", "−", "Zoom out (−)", true)),
                    )
                    .child(pointer_hint(Pointer::Zoom, Caret::Left, |_, _, _| {})),
            )
            .into_any_element()
    });
    matches_golden("pointers", &image);
}

/// The reference rasteriser itself: a red square half over a blue one,
/// a rounded corner, and a triangle, where they should be.
#[test]
fn the_reference_rasteriser_draws_quads_and_paths_in_order() {
    let image = draw(40.0, 20.0, || {
        div()
            .size_full()
            .relative()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .size(px(10.0))
                    .bg(gpui::blue()),
            )
            .child(
                div()
                    .absolute()
                    .left(px(5.0))
                    .top_0()
                    .size(px(10.0))
                    .bg(gpui::red()),
            )
            .child(
                div()
                    .absolute()
                    .left(px(20.0))
                    .top_0()
                    .size(px(20.0))
                    .rounded(px(10.0))
                    .bg(gpui::green()),
            )
            .into_any_element()
    });
    // The test platform draws at twice the size.
    assert_eq!(image.dimensions(), (80, 40));
    let at = |x, y| image.get_pixel(x, y).0;
    assert_eq!(at(4, 4), [0, 0, 255, 255], "blue where only blue is");
    assert_eq!(at(14, 4), [255, 0, 0, 255], "red laid over blue");
    assert_eq!(at(60, 20)[3], 255, "the circle's middle is filled");
    assert_eq!(at(41, 1)[3], 0, "its corner is round, and empty");
}

/// A look at someone's age.
fn aged(age: world_projection::AgeStage) -> world_projection::Look {
    world_projection::Look {
        age: Some(age),
        grey: age == world_projection::AgeStage::Elder,
        stoop: age == world_projection::AgeStage::Elder,
        ..Default::default()
    }
}

/// The harbour with people of every age: an elder with a cane, a child
/// and a teenager, a baby carried by their mother, and a baby asleep in a
/// pram by the lighthouse with nobody of theirs near.
fn generations() -> ProjectionSnapshot {
    use world_projection::AgeStage;
    let mut snapshot = harbour();
    let home = id(101);
    for (n, age) in [
        (1, AgeStage::Adult),
        (2, AgeStage::Elder),
        (3, AgeStage::Child),
        (4, AgeStage::Teen),
    ] {
        if let Some(person) = snapshot
            .canvas
            .items
            .iter_mut()
            .find(|item| item.id == id(n))
        {
            person.look = Some(aged(age));
            person.home = Some(home);
            person.day.clear();
        }
    }
    // Two of them in drawings of their own, as a Pack draws its people.
    let base = world_projection::person_base("golden-folk");
    for (n, variant) in [(1_u64, 7_u32), (4, 30)] {
        let name = format!("golden-folk-{n}");
        snapshot
            .drawings
            .push(world_projection::person(name.clone(), variant, &base));
        if let Some(person) = snapshot
            .canvas
            .items
            .iter_mut()
            .find(|item| item.id == id(n))
        {
            person.drawing = Some(name);
        }
    }
    let mut baby = item(5, CanvasItemKind::Actor, "Pip Quinn", 0.0, Some(100));
    baby.look = Some(aged(AgeStage::Baby));
    baby.home = Some(home);
    snapshot.canvas.items.push(baby);
    let mut sleeping = item(6, CanvasItemKind::Actor, "Wren Moss", 0.9, Some(103));
    sleeping.look = Some(aged(AgeStage::Baby));
    sleeping.home = Some(id(102));
    snapshot.canvas.items.push(sleeping);
    snapshot
}

#[test]
fn people_of_every_age_match_their_golden_picture() {
    let snapshot = generations();
    let frame = diorama_frame(&snapshot, 640.0, 360.0, Daylight::Day, 13.0);
    // The baby at the harbour is in their mother's arms, not on the ground;
    // the one by the lighthouse sleeps in a pram.
    let carried = frame
        .people
        .iter()
        .filter(|person| person.carrying.is_some())
        .count();
    assert_eq!(carried, 1);
    let babies_standing = frame
        .people
        .iter()
        .filter(|person| person.figure.age == crate::age::Age::Baby)
        .count();
    assert_eq!(babies_standing, 1, "one baby in a pram");
    let child = frame
        .people
        .iter()
        .find(|person| person.figure.age == crate::age::Age::Child)
        .unwrap();
    let grown = frame
        .people
        .iter()
        .find(|person| person.figure.age == crate::age::Age::Adult)
        .unwrap();
    assert!(child.height < grown.height * 0.7);
    let image = draw(640.0, 360.0, move || painted(frame.clone()));
    matches_golden("ages", &image);
}

/// A moment of `kind` in the town: its three panels at the bakery.
fn moment_of(
    kind: world_projection::MomentKind,
    title: &str,
    cast: [&[u64]; 3],
) -> world_projection::Moment {
    use world_projection::{Panel, PanelBeat};
    let panel = |beat: PanelBeat, cast: &[u64], caption: &str| Panel {
        caption: caption.into(),
        cast: cast.iter().map(|n| id(*n)).collect(),
        place: Some(id(101)),
        mood: None,
        beat,
        props: Vec::new(),
    };
    world_projection::Moment {
        id: format!("{kind:?}-12"),
        day: 12,
        kind,
        title: title.into(),
        panels: [
            panel(PanelBeat::Before, cast[0], "Before"),
            panel(PanelBeat::Moment, cast[1], "The moment"),
            panel(PanelBeat::After, cast[2], "After"),
        ],
        event: None,
    }
}

/// A wedding, a birth and a farewell, each as three panels in the town's
/// own look: the bakery behind, the people posed for each beat.
#[test]
fn a_wedding_a_birth_and_a_farewell_match_their_golden_pictures() {
    use world_projection::MomentKind;
    let snapshot = generations();
    for (name, moment) in [
        (
            "moment-wedding",
            moment_of(
                MomentKind::Wedding,
                "Mara and Leo marry",
                [&[1, 3], &[1, 3], &[1, 3, 4]],
            ),
        ),
        (
            "moment-birth",
            moment_of(
                MomentKind::Birth,
                "Pip is born",
                [&[1, 3], &[1, 5, 3], &[1, 5, 3]],
            ),
        ),
        (
            "moment-farewell",
            moment_of(
                MomentKind::Farewell,
                "Leo says goodbye",
                [&[2, 1], &[2, 1, 4], &[1, 4]],
            ),
        ),
    ] {
        let (width, height) = (900.0, 480.0);
        let layout = crate::macos::strip_layout(width, height);
        let strip_snapshot = snapshot.clone();
        let image = draw(width, height, move || {
            div()
                .size_full()
                .bg(gpui::rgb(0x6f7a70))
                .relative()
                .child(
                    div()
                        .absolute()
                        .left(px(layout.x))
                        .top(px(layout.y))
                        .child(crate::macos::moment_strip(&strip_snapshot, &moment, layout)),
                )
                .into_any_element()
        });
        matches_golden(name, &image);
    }
}

/// A design drawn from a rule over its squares, in palette places.
fn motif_of(colours: &[u8], rule: impl Fn(usize, usize) -> usize) -> world_projection::Pattern {
    let cells = (0..crate::mark::CELLS)
        .map(|at| {
            let index = rule(at % crate::mark::SIDE, at / crate::mark::SIDE);
            char::from_digit(index as u32, 16).unwrap()
        })
        .collect::<String>();
    world_projection::Design::new(&cells, colours)
        .expect("a design")
        .pattern()
}

/// The harbour with the player's designs: stripes and a sun on the
/// square's flag, chevrons on the boat's sail, a fish on the bakery's
/// sign, and a patchwork quilt at the lighthouse keeper's.
pub(crate) fn designed() -> ProjectionSnapshot {
    let mut snapshot = harbour();
    let flag = motif_of(&[8, 0, 2], |x, y| {
        let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
        if dx * dx + dy * dy < 12.0 {
            2
        } else {
            (y / 3) % 2
        }
    });
    let sail = motif_of(&[0, 3, 9], |x, y| {
        if y >= 13 {
            2
        } else if (x + y) % 8 < 3 {
            1
        } else {
            0
        }
    });
    let sign = motif_of(&[13, 9, 2, 0], |x, y| {
        let (dx, dy) = (x as f32 - 6.5, y as f32 - 7.5);
        if x == 0 || y == 0 || x == 15 || y == 15 {
            1
        } else if (dx / 4.2).powi(2) + (dy / 2.6).powi(2) < 1.0 {
            if x == 4 && y == 6 {
                3
            } else {
                1
            }
        } else if (10..=13).contains(&x) && (y as i32 - 7).unsigned_abs() as usize <= x - 10 {
            1
        } else if x == 14 || y == 14 || x == 1 || y == 1 {
            2
        } else {
            0
        }
    });
    let quilt = motif_of(&[11, 0, 5, 12, 7], |x, y| {
        let block = (x / 4 + y / 4 * 4) % 4;
        if (x % 4 == 1 || x % 4 == 2) && (y % 4 == 1 || y % 4 == 2) {
            1
        } else {
            [0, 2, 3, 4][block]
        }
    });
    let mut flagpole = item(
        201,
        CanvasItemKind::Object,
        "The square's flag",
        0.0,
        Some(102),
    );
    flagpole.shape = Some(MarkShape::Flag);
    flagpole.pattern = Some(flag);
    snapshot.canvas.items.push(flagpole);
    for item in &mut snapshot.canvas.items {
        match item.label.as_str() {
            "Boat" => item.pattern = Some(sail.clone()),
            "Bakery" => {
                item.shape = Some(MarkShape::Shop);
                item.pattern = Some(sign.clone());
            }
            "Lighthouse" => item.pattern = Some(quilt.clone()),
            _ => {}
        }
    }
    // Keep the lighthouse a home, where a quilt is aired.
    snapshot
}

/// The diorama of `snapshot` at `hour`, close on the item called `label`.
fn close_on(snapshot: &ProjectionSnapshot, label: &str, hour: f32) -> diorama::Frame {
    let (width, height) = (480.0, 300.0);
    let daylight = crate::scene::daylight_at(hour as u32);
    let stage = diorama::stage_at(snapshot, width, height, diorama::Clock::at(hour as u8));
    let index = snapshot
        .canvas
        .items
        .iter()
        .position(|item| item.label == label)
        .expect("the item");
    let living = diorama::living(&stage, snapshot, 0.0, daylight, &Default::default(), None);
    let mut camera = Camera::on(&stage, stage.frame_of(index).expect("on stage"));
    camera.zoom = 1.8;
    diorama::frame(
        snapshot,
        &stage,
        &living,
        Camera::around(&stage, camera.zoom, camera.x, camera.y),
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    )
    .at_hour(hour)
}

#[test]
fn designs_on_a_flag_a_sail_a_sign_and_a_quilt_match_their_golden_pictures() {
    let snapshot = designed();
    for (name, label, hour) in [
        ("design-flag", "The square's flag", 13.0),
        ("design-sail", "Boat", 13.0),
        ("design-sign", "Bakery", 13.0),
        ("design-quilt", "Lighthouse", 13.0),
        ("design-quilt-night", "Lighthouse", 22.0),
    ] {
        let frame = close_on(&snapshot, label, hour);
        let worn = frame.worn().len();
        assert_eq!(worn, 4, "{name}: every design is worn");
        let image = draw(480.0, 300.0, move || painted(frame.clone()));
        matches_golden(name, &image);
    }
}

// Composition (v0.24, owner A).

/// The day-1,082 harbour at `hour`, `width` by `height`, with the camera
/// `zoom` times closer (0 for as far out as it goes) at `pan` of the way
/// along it.
pub(crate) fn harbour_1082_frame(
    (width, height): (f32, f32),
    daylight: Daylight,
    hour: f32,
    zoom: f32,
    pan: f32,
) -> diorama::Frame {
    snapshot_frame(
        &crate::diorama::tests::harbour_1082(),
        (width, height),
        daylight,
        hour,
        zoom,
        pan,
    )
}

/// A snapshot's frame, as [`harbour_1082_frame`].
fn snapshot_frame(
    snapshot: &ProjectionSnapshot,
    (width, height): (f32, f32),
    daylight: Daylight,
    hour: f32,
    zoom: f32,
    pan: f32,
) -> diorama::Frame {
    let snapshot = snapshot.clone();
    let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(hour as u8));
    let living = diorama::living(&stage, &snapshot, 0.0, daylight, &Default::default(), None);
    let camera = Camera::around(&stage, zoom, stage.width * pan, height / 2.0);
    diorama::frame(
        &snapshot,
        &stage,
        &living,
        camera,
        0.0,
        daylight,
        &Glows::new(),
        1.0,
    )
    .at_hour(hour)
}

/// Pictures of the composition to look at rather than check: the harbour
/// on day 1,082 at noon, dusk and night, and folded into its postcard.
/// Written to the folder `WORLD_GPUI_PICTURES` names.
#[test]
#[ignore = "pictures: WORLD_GPUI_PICTURES=dir cargo test -p world-gpui -- --ignored composition_pictures"]
fn composition_pictures() {
    let Some(dir) = std::env::var_os("WORLD_GPUI_PICTURES") else {
        return;
    };
    let dir = PathBuf::from(dir);
    let only = std::env::var("WORLD_GPUI_ONLY").unwrap_or_default();
    // `WORLD_GPUI_SNAPSHOT` names another World's wire JSON to picture.
    let (snapshot, prefix) = match std::env::var_os("WORLD_GPUI_SNAPSHOT") {
        Some(path) => {
            let json = std::fs::read_to_string(path).expect("the snapshot");
            let wire: world_pack_protocol::ProjectionSnapshotWire =
                serde_json::from_str(&json).expect("a wire snapshot");
            (
                ProjectionSnapshot::try_from(wire).expect("a snapshot"),
                "other-",
            )
        }
        None => (crate::diorama::tests::harbour_1082(), ""),
    };
    let (width, height) = (1100.0, 848.0);
    for (name, daylight, hour, zoom, pan) in [
        ("noon", Daylight::Day, 12.0, 1.0, 0.3),
        ("noon-east", Daylight::Day, 12.0, 1.0, 0.75),
        ("dusk", Daylight::Dusk, 19.5, 1.0, 0.3),
        ("night", Daylight::Night, 23.0, 1.0, 0.3),
        ("closer", Daylight::Day, 12.0, 1.8, 0.12),
        ("wide", Daylight::Day, 12.0, 0.7, 0.3),
        ("half-folded", Daylight::Day, 12.0, 0.6, 0.3),
        ("postcard-noon", Daylight::Day, 12.0, 0.0, 0.5),
        ("postcard-dusk", Daylight::Dusk, 19.5, 0.0, 0.5),
        ("postcard-night", Daylight::Night, 23.0, 0.0, 0.5),
    ] {
        if !only.is_empty() && !only.split(',').any(|want| want == name) {
            continue;
        }
        let frame = snapshot_frame(&snapshot, (width, height), daylight, hour, zoom, pan);
        let image = draw(width, height, move || painted(frame.clone()));
        image
            .save(dir.join(format!("{prefix}{name}.png")))
            .expect("a picture");
    }
}

/// The v0.24 bar for the whole-town view: zoomed right out, the day-1,082
/// harbour is a postcard that fills the window, by day and at night. The
/// town stands from under a third of the way down to the water at the
/// foot; above it is the sky (blue by day, dark at night, never bare
/// paper), and no row of the window is left unpainted.
#[test]
fn the_whole_town_is_a_postcard_that_fills_the_window() {
    let (width, height) = (550.0, 424.0);
    for (daylight, hour) in [(Daylight::Night, 23.0), (Daylight::Day, 12.0)] {
        let frame = harbour_1082_frame((width, height), daylight, hour, 0.0, 0.5);
        let snapshot = crate::diorama::tests::harbour_1082();
        let stage = diorama::stage_at(&snapshot, width, height, diorama::Clock::at(hour as u8));
        let camera = Camera::around(&stage, 0.0, stage.width / 2.0, height / 2.0);
        assert!((camera.fold - 1.0).abs() < 1e-4, "folded right up");
        // The back row's roofs, in the last row of the postcard.
        let roofs = stage.base - stage.building_h * 0.7;
        let top = camera.at(&stage, stage.width - 1.0, roofs).1;
        let town = (height - top) / height;
        assert!(
            town >= 0.7,
            "the town fills {:.0}% of the window",
            town * 100.0
        );
        let image = draw(width, height, move || painted(frame.clone()));
        let paper = image::Rgba([0xf2, 0xee, 0xe6, 0xff]);
        let near = |a: &image::Rgba<u8>, b: &image::Rgba<u8>| {
            a.0.iter().zip(b.0).all(|(x, y)| x.abs_diff(y) <= 3)
        };
        let bare = image.pixels().filter(|pixel| near(pixel, &paper)).count();
        assert!(bare * 1000 < image.pixels().len(), "{bare} bare pixels");
        let band = image.height() * 15 / 100;
        let sky = image
            .enumerate_pixels()
            .filter(|(_, y, _)| *y < band)
            .map(|(_, _, pixel)| pixel.0)
            .fold([0u64; 3], |sum, p| {
                [
                    sum[0] + p[0] as u64,
                    sum[1] + p[1] as u64,
                    sum[2] + p[2] as u64,
                ]
            });
        let n = u64::from(image.width()) * u64::from(band);
        let (r, g, b) = (sky[0] / n, sky[1] / n, sky[2] / n);
        if daylight == Daylight::Night {
            assert!(r + g + b < 3 * 110 && b > r, "a night sky: {r} {g} {b}");
        } else {
            assert!(b > r && b > 150, "a day sky: {r} {g} {b}");
        }
    }
}

/// A lived place of Pocket Universe, as its Pack sent it over the wire.
fn place(name: &str) -> ProjectionSnapshot {
    let json = match name {
        "ares" => include_str!("../tests/fixtures/place-ares.json"),
        "maple" => include_str!("../tests/fixtures/place-maple.json"),
        _ => include_str!("../tests/fixtures/place-icebridge.json"),
    };
    let wire: world_pack_protocol::ProjectionSnapshotWire =
        serde_json::from_str(json).expect("a place's snapshot");
    ProjectionSnapshot::try_from(wire).expect("a snapshot")
}

/// Each place of Pocket Universe in its own clothes, at noon: Ares's domes
/// and hab modules on regolith under a butterscotch sky, Maple Street's
/// storefronts, cars and wires, Icebridge's snow nests, ice shelf and sea
/// ice. With `WORLD_GPUI_PLACE_SHOTS=<dir>` each is also drawn large there,
/// for looking at.
#[test]
fn each_pocket_universe_place_matches_its_golden_picture() {
    let shots = std::env::var("WORLD_GPUI_PLACE_SHOTS").ok();
    for name in ["ares", "maple", "icebridge"] {
        let snapshot = place(name);
        assert!(snapshot.canvas.setting.is_some(), "{name} says what it is");
        let frame = diorama_frame(&snapshot, 480.0, 300.0, Daylight::Day, 12.5);
        let image = draw(480.0, 300.0, move || painted(frame.clone()));
        matches_golden(&format!("place-{name}"), &image);
        if let Some(dir) = &shots {
            for (label, daylight, hour) in [
                ("noon", Daylight::Day, 12.5),
                ("dusk", Daylight::Dusk, 19.5),
                ("night", Daylight::Night, 23.0),
            ] {
                let snapshot = snapshot.clone();
                let stage =
                    diorama::stage_at(&snapshot, 1100.0, 700.0, diorama::Clock::at(hour as u8));
                let living =
                    diorama::living(&stage, &snapshot, 0.0, daylight, &Default::default(), None);
                let camera = Camera::around(&stage, 1.3, stage.width * 0.3, 700.0 * 0.55);
                let frame = diorama::frame(
                    &snapshot,
                    &stage,
                    &living,
                    camera,
                    0.0,
                    daylight,
                    &Glows::new(),
                    1.0,
                )
                .at_hour(hour);
                let image = draw(1100.0, 700.0, move || painted(frame.clone()));
                image.save(format!("{dir}/{name}-{label}.png")).unwrap();
            }
        }
    }
}
