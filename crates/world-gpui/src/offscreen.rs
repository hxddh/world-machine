//! Drawing GPUI offscreen, on the CPU: a small reference rasteriser that
//! fills the `Scene` GPUI would hand Metal (quads with borders and
//! gradients, blurred shadows, paths, and the painter's still layers as
//! images), handed to GPUI's test platform through
//! `HeadlessAppContext::with_platform`. Text is not drawn.
//!
//! The golden pictures use it (see `golden.rs`), and so does the key art
//! example, which includes this file by its path: so it stands alone,
//! with nothing from the crate.
//!
//! It keeps to GPUI's public drawing seam where one exists: it draws the
//! scene's batches in the order `Scene::batches` gives them (the order
//! GPUI's own renderers draw), and reads only each primitive's public
//! fields and the atlas tiles it hands out itself.

#![allow(dead_code)]

use gpui::{
    div, point, prelude::*, px, size, AnyElement, App, AtlasKey, AtlasTextureId, AtlasTextureKind,
    AtlasTile, Background, Bounds, Corners, DevicePixels, Edges, HeadlessAppContext, Hsla,
    NoopTextSystem, Path, PlatformAtlas, PlatformHeadlessRenderer, PrimitiveBatch, Rgba,
    ScaledPixels, Scene, Shadow, Size, TileId, Window,
};
use image::RgbaImage;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Hands out atlas tiles, keeping each image's pixels (as GPUI's own
/// atlases do) so sprites can be drawn.
#[derive(Default)]
pub struct Atlas(Mutex<AtlasState>);

#[derive(Default)]
pub struct AtlasState {
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
        let bytes = bytes.into_owned();
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
        state.pixels.insert(id, (size, bytes));
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
pub struct Raster(pub Arc<Atlas>);

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

pub fn rasterise(scene: &Scene, size: Size<DevicePixels>, atlas: &Atlas) -> RgbaImage {
    let (width, height) = (size.width.0.max(0) as usize, size.height.0.max(0) as usize);
    let mut canvas = Canvas {
        width,
        height,
        pixels: vec![[0.0; 4]; width * height],
    };
    // In GPUI's own draw order: the batches its GPU renderers draw, so
    // nothing here re-derives how primitives of one order are layered.
    for batch in scene.batches() {
        match batch {
            PrimitiveBatch::Shadows(range) => {
                for shadow in &scene.shadows[range] {
                    draw_shadow(&mut canvas, shadow);
                }
            }
            PrimitiveBatch::Quads(range) => {
                for quad in &scene.quads[range] {
                    draw_quad(&mut canvas, quad);
                }
            }
            PrimitiveBatch::Paths(range) => {
                for path in &scene.paths[range] {
                    draw_path(&mut canvas, path);
                }
            }
            PrimitiveBatch::PolychromeSprites { range, .. } => {
                for sprite in &scene.polychrome_sprites[range] {
                    draw_sprite(&mut canvas, sprite, atlas);
                }
            }
            // Text and surfaces are not drawn.
            _ => {}
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
pub fn draw(width: f32, height: f32, picture: impl Fn() -> AnyElement + 'static) -> RgbaImage {
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
