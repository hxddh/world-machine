//! What people built: the buildings and things on the ground, painted as
//! sprites into the still layers, the ridge's marks, and the player's
//! designs worn on sails, flags, signs and quilts.

use super::*;

/// A building as painted, in stage pixels at zoom 1.
#[derive(Clone, Debug)]
pub(super) struct BuildingPaint {
    pub(super) index: usize,
    pub(super) x: f32,
    pub(super) base: f32,
    pub(super) w: f32,
    pub(super) h: f32,
    pub(super) shape: MarkShape,
    pub(super) palette: Palette,
    pub(super) glow: Option<Hsla>,
    pub(super) drawing: Option<Drawing>,
    /// Springing after a click: wider, taller. `(1, 1)` at rest.
    pub(super) squash: (f32, f32),
    /// Rising into place when just built, 0 to 1.
    pub(super) grow: f32,
    /// Someone indoors: their colours, seen as a shape in a lit window.
    pub(super) inside: Vec<Figure>,
    /// Facing the other way, as the town built it.
    pub(super) flip: bool,
    /// A low wall run on to the neighbour on the left, and on the right.
    pub(super) joins: (bool, bool),
}

impl BuildingPaint {
    /// Drawn live this frame rather than painted into the still layer.
    pub(super) fn moving(&self) -> bool {
        self.grow < 1.0 || (self.squash.0 - 1.0).abs() > 1e-3 || (self.squash.1 - 1.0).abs() > 1e-3
    }
}

/// A thing as drawn, in stage pixels at zoom 1.
#[derive(Clone, Debug)]
pub(super) struct ThingPaint {
    pub(super) index: usize,
    pub(super) x: f32,
    pub(super) base: f32,
    pub(super) w: f32,
    pub(super) shape: MarkShape,
    pub(super) palette: Palette,
    pub(super) sway: f32,
    /// A boat's roll on the swell, in radians.
    pub(super) roll: f32,
    pub(super) glow: Option<Hsla>,
    pub(super) drawing: Option<Drawing>,
    pub(super) grow: f32,
    /// Facing the other way, as the town built it.
    pub(super) flip: bool,
    /// Toned toward the place's base colours: over the screen's accent
    /// budget.
    pub(super) muted: bool,
}

/// At most this many saturated accent marks a screen-width (the art
/// bible's §4): flags, the postbox, awnings, a quilt.
pub const ACCENTS_A_SCREEN: usize = 3;

/// Whether a thing is one of the saturated marks the eye finds: a flag,
/// bunting, the postbox, an awning or a stall's cloth, a tent.
pub(super) fn is_accent(thing: &ThingPaint) -> bool {
    const KEYS: [&str; 16] = [
        "flag",
        "bunting",
        "pillar-box",
        "lamp-box",
        "mailbox",
        "market-stall",
        "flower-stall",
        "fish-stall",
        "kites",
        "streamers",
        "pennant",
        "paper-lanterns",
        "hot-dog-stand",
        "ice-cream-stand",
        "neon",
        "awning",
    ];
    let named = |key: &str| KEYS.iter().any(|accent| key.contains(accent));
    matches!(
        thing.shape,
        MarkShape::Flag
            | MarkShape::Bunting
            | MarkShape::Postbox
            | MarkShape::Stall
            | MarkShape::Tent
    ) && thing.palette.art.is_none()
        || thing.palette.art.is_some_and(|art| named(art.key()))
        || thing.drawing.as_ref().is_some_and(|drawing| {
            ["-flag", "-bunting", "-stall", "-tent"]
                .iter()
                .any(|end| drawing.id.ends_with(end))
        })
}

/// Keeps the accents to the budget: in any stretch a window `view_w`
/// wide, at most [`ACCENTS_A_SCREEN`] saturated marks. The player's own
/// designs (`designed`, by item) are kept first, then the rest from left
/// to right; whatever would go over is muted toward the place's base
/// colours.
pub(super) fn budget_accents(things: &mut [ThingPaint], designed: &[(usize, f32)], view_w: f32) {
    let mut kept = designed.iter().map(|(_, x)| *x).collect::<Vec<_>>();
    let mut order = (0..things.len())
        .filter(|at| is_accent(&things[*at]))
        .collect::<Vec<_>>();
    order.sort_by(|a, b| {
        let mine = |at: &usize| {
            !designed
                .iter()
                .any(|(index, _)| *index == things[*at].index)
        };
        mine(a)
            .cmp(&mine(b))
            .then(things[*a].x.total_cmp(&things[*b].x))
    });
    for at in order {
        let thing = &mut things[at];
        if designed.iter().any(|(index, _)| *index == thing.index) {
            continue;
        }
        let mut near = kept
            .iter()
            .copied()
            .filter(|x| (x - thing.x).abs() < view_w)
            .collect::<Vec<_>>();
        near.push(thing.x);
        near.sort_by(f32::total_cmp);
        let crowded = near
            .windows(ACCENTS_A_SCREEN + 1)
            .any(|run| run[ACCENTS_A_SCREEN] - run[0] < view_w);
        if crowded {
            thing.muted = true;
        } else {
            kept.push(thing.x);
        }
    }
}

/// A built thing on the far ridge: where along it (0 to 1), its shape, and
/// how far it has risen.
#[derive(Clone, Copy, Debug)]
pub(super) struct RidgeMark {
    pub(super) along: f32,
    pub(super) shape: MarkShape,
    pub(super) grow: f32,
}

/// Something the World is working toward, on the ridge.
#[derive(Clone, Copy, Debug)]
pub(super) struct RidgeGoal {
    pub(super) along: f32,
    pub(super) w: f32,
    pub(super) h: f32,
    pub(super) shape: MarkShape,
    pub(super) done: u32,
    pub(super) parts: u32,
}

/// The colours something is painted in: its own, with the colour the town
/// chose for it when it was built on a plot.
pub(super) fn painted_as(
    item: &CanvasItem,
    lit: bool,
    setting: art::Setting,
    scenery: &Scenery,
) -> Palette {
    let mut palette =
        Palette::of_in(&item.id.stable_key(), lit, setting).drawn_as(item.art.as_deref());
    if let Some(variant) = item.variant {
        let [r, g, b] = variant.colour;
        palette.roof = art::hex(u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b));
    }
    // Nothing on the land is lighter than the sky (the art bible's §4).
    palette.wall = under_sky(scenery, palette.wall, WALLS_UNDER_SKY);
    palette
}

/// The newest thing built rising on the ridge.
pub(super) fn paint_rising(
    window: &mut dyn Brush,
    frame: &Frame,
    ox: f32,
    oy: f32,
    width: f32,
    height: f32,
) {
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

/// One tile of the buildings: every shadow, then every building over them
/// (each already lit, and on its own paper). Nothing where there are none.
pub(super) fn paint_building_tile(
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

/// A building painted on its own, lit: its key light and shade, a line of
/// low sun on its edge, snow or dust along its tops, its lit windows with
/// whoever is inside and their glow; and the shape of it for its shadow.
pub(super) struct Sprite {
    pub(super) pixmap: sk::Pixmap,
    /// Its top-left corner, in stage pixels.
    pub(super) origin: (f32, f32),
    /// Its shape at half resolution, softened, for the shadow it casts.
    pub(super) silhouette: Option<sk::Pixmap>,
}

/// The buildings' sprites, painted once for the ground's look and shared
/// by every tile each reaches into: by the ground's key and the item.
pub(super) type Sprites = std::collections::HashMap<(u64, usize), std::sync::Arc<Sprite>>;

pub(super) fn sprites() -> &'static std::sync::Mutex<Sprites> {
    static SPRITES: std::sync::OnceLock<std::sync::Mutex<Sprites>> = std::sync::OnceLock::new();
    SPRITES.get_or_init(Default::default)
}

/// The buildings near any of the tiles at `places`.
pub(super) fn near_tiles<'f>(
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
pub(super) fn prepare_sprites(frame: &Frame, ground: u64, scale: f32, places: &[(i32, i32)]) {
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
    if kept.len() > 400 * crate::hand::BOILS as usize {
        kept.retain(|(key, _), _| *key == ground);
    }
    for (index, sprite) in painted {
        kept.insert((ground, index), std::sync::Arc::new(sprite));
    }
}

/// A building's sprite for the ground's look, painted now if it is not
/// kept already.
pub(super) fn sprite_of(
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
    if kept.len() > 400 * crate::hand::BOILS as usize {
        kept.retain(|(key, _), _| *key == ground);
    }
    kept.insert((ground, building.index), sprite.clone());
    sprite
}

/// Whether a place is something that grows: a tree, an orchard, a garden.
pub(super) fn leafy(building: &BuildingPaint) -> bool {
    matches!(
        building.shape,
        MarkShape::Tree | MarkShape::Garden | MarkShape::Planter | MarkShape::Sprouts
    ) || building.drawing.as_ref().is_some_and(|drawing| {
        ["tree", "orchard", "garden", "park", "grove"]
            .iter()
            .any(|word| drawing.id.contains(word))
    })
}

pub(super) fn sprite(frame: &Frame, building: &BuildingPaint, scale: f32) -> Sprite {
    painter::timed("sprite", || sprite_painted(frame, building, scale))
}

pub(super) fn sprite_painted(frame: &Frame, building: &BuildingPaint, scale: f32) -> Sprite {
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
    // A low wall run on toward a neighbour it joins.
    for (joined, side) in [(building.joins.0, -1.0_f32), (building.joins.1, 1.0)] {
        if joined {
            let stone = art::shade(building.palette.wall, -0.18);
            let from = building.x + side * w * 0.36;
            let reach = w * 0.36;
            let (left, tall) = (from.min(from + side * reach), h * 0.1);
            canvas.rect(left, building.base - tall, reach, tall, 1.0, stone);
            canvas.rect(
                left,
                building.base - tall - 1.5,
                reach,
                2.5,
                1.0,
                art::shade(stone, -0.12),
            );
        }
    }
    // Drawn by hand: its outlines wandering, its edges darker, its ink a
    // touch off its paint, its colours leaning to the place's inks.
    let mut hand = crate::hand::Hand::new(&mut canvas, frame.boil(), frame.setting.inks());
    // By day, under the sky's light, before the sun adds its own; lit
    // windows after dusk are as bright as they are.
    let lit_windows = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    let mut under = Under {
        inner: &mut hand,
        most: if lit_windows {
            1.0
        } else {
            sky_luma(&frame.scenery) * WALLS_UNDER_SKY
        },
    };
    let mut mirrored = crate::brush::Xform {
        inner: &mut under,
        m: if building.flip {
            [-1.0, 0.0, 0.0, 1.0, building.x * 2.0, 0.0]
        } else {
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        },
    };
    painter::timed("sprite: paint", || match &building.drawing {
        Some(drawing) => art::paint_drawing(
            &mut mirrored,
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
            &mut mirrored,
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
pub(super) fn paint_building_shadow(
    canvas: &mut Canvas,
    frame: &Frame,
    building: &BuildingPaint,
    sprite: &Sprite,
) {
    let night = frame.daylight == Daylight::Night;
    let [r, g, b] = shadow_ink(frame.hour);
    let ink = Hsla::from(gpui::Rgba { r, g, b, a: 1.0 });
    canvas.soft(
        building.x,
        building.base + building.h * 0.012,
        building.w * 0.56,
        building.h * 0.05,
        building.h * 0.06,
        ink.opacity(if night { 0.32 } else { 0.26 }),
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
    // Long when the sun is low (dawn, dusk), short at noon.
    let long = 0.2 + 0.95 * (1.0 - high).powi(2);
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
    // Inked in a cool colour rather than black, so a little stronger.
    let strength = if frame.weather == Weather::Cloudy {
        0.12
    } else {
        0.3
    };
    // Cool: blue at dawn, slate at noon, warmer at dusk.
    let silhouette = painter::inked(silhouette, shadow_ink(frame.hour));
    canvas.draw_mapped(&silhouette, transform, strength);
}

/// How big a design's cloth is on screen this frame, and whether it is a
/// quilt seen through a lit window.
pub(super) fn cloth_on_screen(frame: &Frame, worn: &Worn) -> Option<(f32, f32, bool)> {
    let z = frame.camera.zoom;
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    if let Some(building) = frame.buildings.iter().find(|b| b.index == worn.index) {
        let w = building.w * z;
        return Some(match worn.wear {
            Wear::Quilt if lit => {
                let ww = w * 0.13 * 0.8;
                (ww, ww / Wear::Quilt.aspect(), true)
            }
            Wear::Flag => {
                let (cw, ch) = mark::cloth_size(worn.wear, w * 0.62 * FLAG_CLOTH);
                (cw, ch, false)
            }
            Wear::Sail => {
                let (cw, ch) = mark::cloth_size(worn.wear, w * 0.62);
                (cw, ch, false)
            }
            wear => {
                let (cw, ch) = mark::cloth_size(wear, w);
                (cw, ch, false)
            }
        });
    }
    let thing = frame.things.iter().find(|t| t.index == worn.index)?;
    let w = thing.w * z * (0.6 + 0.4 * ease(thing.grow));
    let w = if worn.wear == Wear::Flag {
        w * FLAG_CLOTH
    } else {
        w
    };
    let (cw, ch) = mark::cloth_size(worn.wear, w);
    Some((cw, ch, false))
}

/// Pictures of designs kept after they were shown, by the design, what it
/// is worn as, which way it faces and in whose light: shown while a new
/// size or light is being painted, so nothing blinks.
pub(super) type Worn_ = (u64, Wear, bool, bool);

/// Asks for the pictures of the designs worn this frame, painted off the
/// window's thread (or right here, `now`), and puts those ready in the
/// frame.
pub(super) fn fetch_pictures(frame: &mut Frame, window: &mut Window, now: bool) {
    use std::cell::RefCell;
    thread_local! {
        static LAST: RefCell<std::collections::HashMap<Worn_, Picture>> =
            RefCell::new(std::collections::HashMap::new());
    }
    if frame.wearing.is_empty() {
        return;
    }
    let dpr = window.scale_factor().max(0.5);
    // A quarter hour at a time, so the light on a cloth moves on in steps.
    let hour = (frame.hour * 4.0).round() / 4.0;
    let grade = grade_at(hour);
    let light = mark::rounded_light(light_at(hour, frame.weather));
    let mirror = wind(frame) < 0.0;
    let view = frame.view_w;
    // By day a design is shown a little toward the place's own colours, so
    // it sits in the place; after dusk it glows as it is (the art bible's
    // §4).
    let day = !matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    let neutral = frame.setting.place_paints().neutral();
    for mut worn in frame.wearing.clone() {
        if day {
            worn.pattern = std::sync::Arc::new(toned(&worn.pattern, neutral, DESIGN_TONE));
        }
        let Some((cw, ch, in_window)) = cloth_on_screen(frame, &worn) else {
            continue;
        };
        let x = frame
            .buildings
            .iter()
            .find(|b| b.index == worn.index)
            .map(|b| b.x)
            .or_else(|| {
                frame
                    .things
                    .iter()
                    .find(|t| t.index == worn.index)
                    .map(|t| t.x)
            })
            .unwrap_or(0.0);
        let sx = frame.at(x, 0.0).0;
        if sx + cw * 4.0 < 0.0 || sx - cw * 4.0 > view {
            continue;
        }
        let size = mark::picture_size(cw, ch, dpr);
        let mirror = worn.wear == Wear::Flag && mirror;
        let lit = if in_window { LAMP } else { light };
        let key = mark::picture_key(&worn.pattern, worn.wear, size, lit, grade, mirror);
        let identity = (worn.pattern.key(), worn.wear, mirror, in_window);
        let picture = match painter::ready(key) {
            Some(painter::Ready::Image(image, _)) => {
                let picture = Picture(image);
                LAST.with(|last| {
                    let mut last = last.borrow_mut();
                    if last.len() > 64 {
                        last.clear();
                    }
                    last.insert(identity, picture.clone());
                });
                Some(picture)
            }
            _ => {
                let pattern = worn.pattern.clone();
                let wear = worn.wear;
                painter::want(
                    window,
                    key,
                    now,
                    Box::new(move || mark::paint_cloth(&pattern, wear, size, lit, grade, mirror)),
                );
                match painter::ready(key) {
                    Some(painter::Ready::Image(image, _)) => Some(Picture(image)),
                    _ => LAST.with(|last| last.borrow().get(&identity).cloned()),
                }
            }
        };
        if let Some(picture) = picture {
            frame.pictures.insert(worn.index, picture);
        }
    }
}

/// How wide a flag's cloth is drawn for a pole on a thing `w` wide: a
/// share of the width it would have had.
const FLAG_CLOTH: f32 = 0.75;

/// How far a design leans toward the place's colours by day.
pub const DESIGN_TONE: f32 = 0.18;

/// `pattern` with each of its colours `share` of the way toward `toward`
/// (`0xRRGGBB`).
pub(super) fn toned(pattern: &mark::Motif, toward: u32, share: f32) -> mark::Motif {
    let target = [(toward >> 16) as u8, (toward >> 8) as u8, toward as u8];
    mark::Motif {
        cells: pattern.cells,
        palette: pattern
            .palette
            .iter()
            .map(|colour| {
                let mut out = *colour;
                for (channel, value) in out.iter_mut().enumerate() {
                    let (from, to) = (f32::from(*value), f32::from(target[channel]));
                    *value = (from + (to - from) * share).round().clamp(0.0, 255.0) as u8;
                }
                out
            })
            .collect(),
    }
}

/// A flag on a pole standing at (`x`, `base`) for a thing `w` wide.
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_flag_pole(
    window: &mut dyn Brush,
    picture: &Picture,
    x: f32,
    base: f32,
    w: f32,
    t: f32,
    blow: f32,
    light: [f32; 3],
) {
    // The cloth about two fifths of the pole's height across, as a flag
    // flies (the pole stands on the ladder).
    let (cw, ch) = mark::cloth_size(Wear::Flag, w * FLAG_CLOTH);
    let pole = w * 1.08;
    let top = base - pole;
    {
        let mut tinted = Tint::new(window, light);
        tinted.rect(x - 1.3, top, 2.6, pole, 1.0, art::hex(0x5b5048));
        tinted.rect(x - 1.3, top, 1.0, pole, 0.5, art::hex(0x8a7d70));
        art::circle(&mut tinted, x, top - 1.5, 2.6, art::hex(0xc9a24a));
    }
    mark::paint_flag(window, picture, x, top + 2.0, cw, ch, t, blow);
}

/// Designs worn by places: a shop's hanging sign, a home's quilt out on
/// the line by day or over a lit sill at night.
pub(super) fn paint_worn_places(
    window: &mut dyn Brush,
    frame: &Frame,
    screen: &dyn Fn(f32, f32) -> (f32, f32),
    light: [f32; 3],
) {
    let t = frame.seconds;
    let z = frame.camera.zoom;
    let blow = wind(frame);
    let lit = matches!(frame.daylight, Daylight::Dusk | Daylight::Night);
    for worn in &frame.wearing {
        let Some(building) = frame.buildings.iter().find(|b| b.index == worn.index) else {
            continue;
        };
        let Some(picture) = frame.pictures.get(&worn.index) else {
            continue;
        };
        let (x, base) = screen(building.x, building.base);
        let (w, h) = (building.w * z, building.h * z);
        let swing = (t * 1.1 + building.index as f32).sin() * blow.abs().min(1.5) * 0.6;
        // A home's first window, where a quilt is seen at night.
        let pane = if building.shape == MarkShape::House && building.drawing.is_none() {
            art::first_window(x, base, w, h, &building.palette)
        } else {
            (x - w * 0.3, base - h * 0.45, w * 0.16, h * 0.16)
        };
        match worn.wear {
            Wear::Sign => {
                let (bw, bh) = mark::cloth_size(Wear::Sign, w);
                mark::paint_sign(
                    window,
                    picture,
                    x + w * 0.4,
                    base - h * 0.64,
                    bw,
                    bh,
                    1.0,
                    swing,
                );
            }
            Wear::Quilt if lit => {
                mark::paint_quilt_window(window, picture, pane, art::hex(0xffd27a));
            }
            Wear::Quilt => {
                let (qw, qh) = mark::cloth_size(Wear::Quilt, w);
                let ground = base + h * 0.02;
                // Out on a line beside the house, on whichever side is
                // clear of what stands there; with neither clear, aired
                // over a window sill.
                let span = qw * 1.7;
                let clear = |lx: f32| {
                    let things = frame.things.iter().all(|thing| {
                        (screen(thing.x, 0.0).0 - lx).abs() > span / 2.0 + thing.w * z * 0.5
                    });
                    let places = frame.buildings.iter().all(|other| {
                        other.index == building.index
                            || (screen(other.x, 0.0).0 - lx).abs() > span / 2.0 + other.w * z * 0.45
                    });
                    things && places
                };
                let reach = w * 0.5 + span * 0.55;
                match [x - reach, x + reach].into_iter().find(|lx| clear(*lx)) {
                    Some(lx) => mark::paint_quilt_line(
                        window,
                        picture,
                        lx,
                        ground - qh * 1.5,
                        ground,
                        qw,
                        qh,
                        t,
                        blow,
                    ),
                    None => mark::paint_quilt_sill(window, picture, pane),
                }
            }
            Wear::Flag | Wear::Sail => {
                paint_flag_pole(window, picture, x + w * 0.3, base, w * 0.62, t, blow, light);
            }
        }
    }
}
