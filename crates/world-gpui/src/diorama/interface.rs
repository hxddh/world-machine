//! The scene as GPUI shows it outside the World window: the still layers
//! painted right here, a tile at a time, for the strip, a cover, a
//! postcard and the golden pictures ([`paint`]); and what every window
//! asks of how its scene is painted (the World window's living stage
//! reports it, `stage_view`), the loading wash and the frame log.

use super::*;

/// How many device pixels a tile of the ground layer is on a side.
pub(super) const TILE: u32 = 256;
/// Tiles are painted this many of their own pixels larger each side, so
/// where they meet the display blends real neighbours.
pub(super) const LAND_PAD: u32 = 1;
/// The sky is soft all over, so it is painted at half the display's
/// resolution.
pub(super) const SKY_RES: f32 = 0.3;
/// How much the far hills move with the camera along a panorama.
pub(super) const HILLS: f32 = 0.3;
/// Everything the still layers look like besides where things are: the
/// place's colours, the hour to the quarter, the weather and the season.
pub(super) fn look_key(frame: &Frame, key: &mut Key) {
    let s = frame.scenery;
    key.add((s.sky_top, s.sky_bottom, s.far, s.near, s.sun))
        .add(frame.daylight as u8)
        .add((frame.hour * 4.0).floor() as i32)
        .add(frame.weather as u8)
        .add(frame.season.map(|season| season as u8))
        .add(frame.cover.map(|cover| cover as u8))
        .add((frame.ice, frame.water))
        // The kind of place: two Worlds with the same colours but another
        // setting never share a picture.
        .add(frame.setting as u8);
    if let Some(look) = &frame.look {
        key.add((look.key, look.glow, look.spine, look.haze));
        for lamp in &look.lamps {
            key.float(*lamp);
        }
    }
}

/// Which World a picture is of: its kind of place and its colours. The
/// rough painting of one World never stands in for another's (v0.28 opened
/// each of the other Pack's places on the first place's meadow).
pub(super) fn world_tag(frame: &Frame) -> u64 {
    let s = frame.scenery;
    let mut key = Key::new("world");
    key.add((s.sky_top, s.sky_bottom, s.far, s.near, s.sun))
        .add(frame.setting as u8)
        .add(frame.water)
        .add(
            frame
                .look
                .as_ref()
                .map(|look| (look.key, look.spine, look.haze)),
        );
    key.finish()
}

/// The still layers of a scene, back to front.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) enum Still {
    Sky,
    Hills,
    Land,
    Buildings,
}

/// One version of a still layer: what it looks like (its key), the scale
/// it is painted at, and which drawing of the boil it is (see
/// [`crate::hand`]): the same look drawn again by hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Version {
    pub(super) key: u64,
    pub(super) scale: f32,
    pub(super) boil: u32,
}

impl Version {
    /// The key of what is painted: the look, and the boil's drawing of it.
    pub(super) fn painted(&self) -> u64 {
        if self.boil == 0 {
            self.key
        } else {
            let mut key = Key::new("boil");
            key.add((self.key, self.boil));
            key.finish()
        }
    }
}

/// Work that paints one picture, off the window's thread.
pub(super) type Job = Box<dyn FnOnce() -> Option<sk::Pixmap> + Send + 'static>;

/// A rectangle relative to the scene's top-left: x, y, width, height.
pub(super) type Rect = (f32, f32, f32, f32);

/// One picture of a still layer: its key, where it is shown, the part of
/// it that shows, and how to paint it.
pub(super) struct Piece {
    pub(super) key: u64,
    pub(super) rect: Rect,
    pub(super) clip: Rect,
    pub(super) job: Job,
}

/// A picture a still layer draws: where it is shown, and the part of it
/// that shows.
pub(super) type Drawn = (std::sync::Arc<gpui::RenderImage>, Rect, Rect);

/// The version of each still layer a frame wants.
pub(super) fn versions(frame: &Frame, width: f32, height: f32, dpr: f32) -> [(Still, Version); 4] {
    let mut sky = Key::new("sky");
    look_key(frame, &mut sky);
    sky.float(width)
        .float(height)
        .float(dpr)
        .float(frame.horizon);
    let band = Band::of(frame, width, height);
    let mut hills = Key::new("hills");
    look_key(frame, &mut hills);
    hills
        .float(band.w)
        .float(band.above)
        .float(band.below)
        .float(dpr)
        .float(frame.building_h)
        .float(frame.view_w)
        // The back row is the setting's own drawings.
        .add(frame.setting as u8);
    for mark in &frame.marks {
        hills
            .add((mark.shape as u8, mark.grow >= 1.0))
            .float(mark.along);
    }
    for goal in &frame.goals {
        hills
            .add((goal.shape as u8, goal.done, goal.parts))
            .float(goal.along);
    }
    // The camera's zoom to a hundredth: a picture painted at one zoom
    // serves every zoom that rounds to it.
    let zoom = ((frame.camera.zoom * 100.0).round() / 100.0).max(0.05);
    let scale = zoom * dpr;
    let ground = ground_key(frame, scale).finish();
    [
        (
            Still::Sky,
            Version {
                key: sky.finish(),
                scale: (dpr * SKY_RES).max(0.5),
                boil: 0,
            },
        ),
        (
            Still::Hills,
            Version {
                key: hills.finish(),
                scale: dpr,
                boil: 0,
            },
        ),
        (
            Still::Land,
            Version {
                key: ground,
                scale: scale * 0.5,
                boil: 0,
            },
        ),
        (
            Still::Buildings,
            Version {
                key: ground,
                scale,
                boil: frame.boil(),
            },
        ),
    ]
}

/// The pictures of one version of a still layer the camera sees, with
/// `margin` tiles more each side (to have them ready before a pan reaches
/// them).
#[allow(clippy::too_many_arguments)]
pub(super) fn pieces(
    frame: &std::sync::Arc<Frame>,
    layer: Still,
    version: Version,
    width: f32,
    height: f32,
    dpr: f32,
    margin: i32,
) -> Vec<Piece> {
    match layer {
        Still::Sky => {
            let scale = version.scale;
            strips(width, scale)
                .map(|(index, x0, w)| {
                    let frame = frame.clone();
                    let pad = 1.0 / scale;
                    let mut key = Key::new("sky strip");
                    key.add((version.key, index));
                    Piece {
                        key: key.finish(),
                        rect: (x0 - pad, 0.0, w + pad * 2.0, height),
                        clip: (x0, 0.0, w + 1.0 / dpr, height),
                        job: Box::new(move || {
                            let mut canvas = Canvas::new(
                                (w * scale).ceil() as u32 + 2,
                                (height * scale).ceil() as u32,
                                scale,
                                (x0 - pad, 0.0),
                            )?;
                            let at = ((x0 * scale).round() as i32 - 1, 0);
                            paint_sky_on(&mut canvas, at, &frame, width, height);
                            Some(canvas.pixmap)
                        }),
                    }
                })
                .collect()
        }
        Still::Hills => {
            let band = Band::of(frame, width, height);
            let (bx, by) = band.screen(frame);
            let squash = Band::squash(frame);
            let scale = (dpr * 0.5).max(1.0);
            let tall = band.above + band.below;
            strips(band.w, scale)
                .map(|(index, x0, w)| {
                    let frame = frame.clone();
                    let band = Band::of(&frame, width, height);
                    let pad = 1.0 / scale;
                    let mut key = Key::new("hills strip");
                    key.add((version.key, index));
                    Piece {
                        key: key.finish(),
                        rect: (bx + x0 - pad, by, w + pad * 2.0, tall * squash),
                        clip: (bx + x0, by, w + 1.0 / dpr, tall * squash),
                        job: Box::new(move || {
                            let mut canvas = Canvas::new(
                                (w * scale).ceil() as u32 + 2,
                                (tall * scale).ceil() as u32,
                                scale,
                                (x0 - pad, 0.0),
                            )?;
                            let at = ((x0 * scale).round() as i32 - 1, 0);
                            paint_band_on(&mut canvas, at, &frame, &band);
                            Some(canvas.pixmap)
                        }),
                    }
                })
                .collect()
        }
        Still::Land | Still::Buildings => {
            let layer = if layer == Still::Land {
                Layer::Land
            } else {
                Layer::Buildings
            };
            let tiles = tiles_in_view(frame, layer, version.scale, margin);
            tile_pieces(frame, layer, version, dpr, tiles)
        }
    }
}

/// The pictures of one version of the land or the buildings at `tiles`.
pub(super) fn tile_pieces(
    frame: &std::sync::Arc<Frame>,
    layer: Layer,
    version: Version,
    dpr: f32,
    tiles: Vec<(i32, i32)>,
) -> Vec<Piece> {
    let scale = version.scale;
    let zoom = frame.camera.zoom;
    let side = TILE as f32 / scale * zoom;
    let (sx, sy) = frame.at(0.0, 0.0);
    let (sx, sy) = ((sx * dpr).round() / dpr, (sy * dpr).round() / dpr);
    let pad = LAND_PAD as f32 * side / TILE as f32;
    // The land, which is solid, runs a device pixel on under the next
    // tile's edge, so no seam of half-covered pixels shows; the buildings,
    // which are mostly clear, must not, or where they overlap a shadow
    // would be laid twice.
    let over = if layer == Layer::Land { 1.0 / dpr } else { 0.0 };
    let reach = frame.at(0.0, layer_rows(frame, layer).0).1.floor();
    tiles
        .into_iter()
        .map(|(column, row)| {
            let mut key = Key::new("tile");
            key.add((version.painted(), layer, column, row));
            let (x, y) = (sx + column as f32 * side, sy + row as f32 * side);
            let frame = frame.clone();
            let ground = version.painted();
            Piece {
                key: key.finish(),
                rect: (x - pad, y - pad, side + pad * 2.0, side + pad * 2.0),
                clip: {
                    // Nothing of a tile shows above where its layer begins:
                    // no empty tile edge ever lies over the sky.
                    let top = y.max(reach);
                    (x, top, side + over, (y + side + over - top).max(0.0))
                },
                job: Box::new(move || match layer {
                    Layer::Land => paint_land_tile(&frame, column, row, scale),
                    Layer::Buildings => paint_building_tile(&frame, ground, column, row, scale),
                }),
            }
        })
        .collect()
}

/// Every tile of a layer across the whole stage, at `scale`.
pub(super) fn tiles_of_stage(frame: &Frame, layer: Layer, scale: f32) -> Vec<(i32, i32)> {
    let tile = TILE as f32 / scale;
    let (top, bottom) = layer_rows(frame, layer);
    let bottom = bottom.min(frame.height + tile);
    let rows = ((top.max(0.0) / tile).floor() as i32)..=((bottom / tile).floor() as i32);
    let columns = 0..=(((frame.width - 0.01) / tile).floor() as i32).max(0);
    rows.flat_map(|row| columns.clone().map(move |column| (column, row)))
        .collect()
}

/// How wide a strip of the sky or the hills is, in device pixels: wide
/// pictures go to the display a strip at a time.
pub(super) const STRIP: f32 = 256.0;

/// The strips of a picture `width` units wide at `scale` device pixels to a
/// unit: each one's index, where it begins and how wide it is, in units.
pub(super) fn strips(width: f32, scale: f32) -> impl Iterator<Item = (i32, f32, f32)> {
    let each = STRIP / scale;
    let count = (width / each).ceil().max(1.0) as i32;
    (0..count).map(move |index| {
        let x0 = index as f32 * each;
        (index, x0, each.min(width - x0))
    })
}

thread_local! {
    /// How many pictures of what the camera sees were not painted this
    /// frame (a flat stand-in, or nothing, in their place), per window.
    static UNPAINTED: std::cell::RefCell<std::collections::HashMap<u64, usize>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// How many pictures of what the camera saw were not painted in the frame
/// last drawn in `window`, and whether the loading wash lay over it: a
/// frame the player saw unfinished. The release screenshot harness logs
/// it for every frame (`WORLD_GPUI_FRAME_LOG`), and the return film waits
/// for a whole one before its beat begins.
pub fn unpainted(window: &Window) -> usize {
    UNPAINTED.with(|counts| {
        counts
            .borrow()
            .get(&window_id(window))
            .copied()
            .unwrap_or(0)
    })
}

thread_local! {
    /// Whether every still layer of the frame last drawn in each window had
    /// settled on what it wants: sharp, whole, nothing still arriving.
    static SETTLED: std::cell::RefCell<std::collections::HashMap<u64, bool>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Whether every still layer of the frame last drawn in `window` had
/// settled: its own sharp pictures shown, nothing arriving, nothing left
/// to hand to the display. A layer that never settles is a picture that
/// never finishes (v0.27's year-three zoom-out).
pub fn settled(window: &Window) -> bool {
    painter::synchronous()
        || SETTLED.with(|settled| {
            settled
                .borrow()
                .get(&window_id(window))
                .copied()
                .unwrap_or(false)
        })
}

/// Whether the frame last drawn in `window` showed everything the camera
/// saw painted: the return film's words wait for it.
pub fn view_painted(window: &Window) -> bool {
    painter::synchronous() || (painted(window) && unpainted(window) == 0)
}

thread_local! {
    /// Where each window's camera was last heading, and whether what it
    /// will see there was shown.
    static HEADING_READY: std::cell::RefCell<std::collections::HashMap<u64, (Camera, bool)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// When each window last drew its scene.
    static DRAWN_AT: std::cell::RefCell<std::collections::HashMap<u64, std::time::Instant>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Whether all the camera of `window` will see from `to` is painted and
/// with the display, so a glide there arrives on sharp paint; `None` until
/// a frame heading there has been drawn. Painting in place (a test), it
/// always is.
pub fn heading_ready(window: &Window, to: Camera) -> Option<bool> {
    if painter::synchronous() {
        return Some(true);
    }
    HEADING_READY.with(|kept| {
        kept.borrow()
            .get(&window_id(window))
            .filter(|(camera, _)| {
                (camera.x - to.x).abs() < 1.0
                    && (camera.y - to.y).abs() < 1.0
                    && (camera.zoom - to.zoom).abs() < 0.005
                    && (camera.fold - to.fold).abs() < 0.005
            })
            .map(|(_, ready)| *ready)
    })
}

/// A window not drawn for this long is taken to be closed: what was kept
/// for it is let go.
const CLOSED: std::time::Duration = std::time::Duration::from_secs(60);

/// Notes that `id` is drawing, and lets go of everything kept per window
/// for windows that have not drawn for a while (closed): their living
/// stages, and the counts the frame log reads.
fn forget_closed(id: u64) {
    let now = std::time::Instant::now();
    let gone = DRAWN_AT.with(|drawn| {
        let mut drawn = drawn.borrow_mut();
        drawn.insert(id, now);
        let gone = drawn
            .iter()
            .filter(|(_, at)| now.duration_since(**at) > CLOSED)
            .map(|(window, _)| *window)
            .collect::<Vec<_>>();
        for window in &gone {
            drawn.remove(window);
        }
        gone
    });
    if gone.is_empty() {
        return;
    }
    forget_windows(&gone);
}

/// Lets go of everything kept for the windows `gone`.
pub(super) fn forget_windows(gone: &[u64]) {
    let gone = gone
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    HEADING_READY.with(|kept| kept.borrow_mut().retain(|window, _| !gone.contains(window)));
    UNPAINTED.with(|counts| {
        counts
            .borrow_mut()
            .retain(|window, _| !gone.contains(window))
    });
    SETTLED.with(|settled| {
        settled
            .borrow_mut()
            .retain(|window, _| !gone.contains(window))
    });
    PAINTED.with(|painted| {
        painted
            .borrow_mut()
            .retain(|window, _| !gone.contains(window))
    });
    WASHED.with(|washed| washed.borrow_mut().retain(|window| !gone.contains(window)));
    super::stage_view::forget(&gone);
    painter::forget_windows(&gone);
}

/// What the living stage drew in `window` this frame, for everything that
/// asks how the scene is painted (the welcome, the return film's words,
/// the frame log): whether any of the place was `shown` (only before a
/// window's first look is ready is it not, under the loading wash), and
/// whether it had `settled` on every look it wants. The living stage has
/// no rough painting and no flat stand-ins: once shown, nothing in view
/// is unpainted, and wherever the camera heads is shown at once (by the
/// nearest look baked until its own is).
pub(super) fn note_stage(window: &Window, frame: &Frame, shown: bool, settled: bool) {
    let id = window_id(window);
    if !painter::synchronous() {
        forget_closed(id);
    }
    UNPAINTED.with(|counts| counts.borrow_mut().insert(id, usize::from(!shown)));
    SETTLED.with(|kept| kept.borrow_mut().insert(id, shown && settled));
    if let Some(to) = frame.heading() {
        HEADING_READY.with(|kept| kept.borrow_mut().insert(id, (to, shown)));
    }
}

/// A window's bookkeeping is let go a while after it last drew (it has
/// closed), and an open one's is kept: the per-window maps never grow with
/// every window ever opened.
#[test]
fn a_closed_windows_bookkeeping_is_let_go() {
    let kept = || {
        let mut windows = std::collections::BTreeSet::new();
        UNPAINTED.with(|counts| windows.extend(counts.borrow().keys().copied()));
        SETTLED.with(|settled| windows.extend(settled.borrow().keys().copied()));
        HEADING_READY.with(|kept| windows.extend(kept.borrow().keys().copied()));
        windows
    };
    for window in [901_u64, 902] {
        UNPAINTED.with(|counts| counts.borrow_mut().insert(window, 0));
        SETTLED.with(|settled| settled.borrow_mut().insert(window, true));
        HEADING_READY.with(|kept| {
            kept.borrow_mut().insert(
                window,
                (
                    Camera {
                        zoom: 1.0,
                        x: 50.0,
                        y: 50.0,
                        fold: 0.0,
                    },
                    true,
                ),
            )
        });
    }
    let long_ago = std::time::Instant::now() - CLOSED - std::time::Duration::from_secs(1);
    DRAWN_AT.with(|drawn| drawn.borrow_mut().insert(901, long_ago));
    forget_closed(902);
    let windows = kept();
    assert!(!windows.contains(&901), "the closed window's is let go");
    assert!(windows.contains(&902), "the open window's is kept");
    forget_windows(&[902]);
    assert!(!kept().contains(&902));
}

/// Draws a still layer's pictures, at the scene's top-left.
pub(super) fn draw_still(window: &mut Window, bounds: Bounds<Pixels>, drawn: &[Drawn]) {
    let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let at = |(x, y, w, h): Rect| Bounds::new(point(px(ox + x), px(oy + y)), size(px(w), px(h)));
    for (image, rect, clip) in drawn {
        let clip = at(*clip).intersect(&bounds);
        let _ = window.paint_image(clip, at(*rect), Corners::default(), image.clone(), 0, false);
    }
}

/// The still layers `only` of `frame`, `width` by `height`, as pictures
/// painted right here (or kept by the painter from a frame before): each
/// piece of the version the frame wants, back to front.
fn still_now(
    frame: &std::sync::Arc<Frame>,
    window: &mut Window,
    (width, height): (f32, f32),
    only: &[Still],
) -> Vec<(Still, Vec<Drawn>)> {
    let dpr = window.scale_factor().max(0.5);
    versions(frame, width, height, dpr)
        .into_iter()
        .filter(|(layer, _)| only.contains(layer))
        .map(|(layer, version)| {
            let drawn = pieces(frame, layer, version, width, height, dpr, 0)
                .into_iter()
                .filter_map(|piece| {
                    painter::want(window, piece.key, true, piece.job);
                    match painter::ready(piece.key) {
                        Some(painter::Ready::Image(image, _)) => {
                            Some((image, piece.rect, piece.clip))
                        }
                        _ => None,
                    }
                })
                .collect();
            (layer, drawn)
        })
        .collect()
}

/// Every still layer, back to front.
pub(super) const ALL: [Still; 4] = [Still::Sky, Still::Hills, Still::Land, Still::Buildings];

/// The rows of a folded postcard, back to front: each row's frame (seen
/// through its own camera, with whoever stands in it), and the stretch of
/// the screen it shows in, left to right. `None` while the place is not
/// folding.
pub(super) fn rows_of(frame: &Frame) -> Option<Vec<(Frame, (f32, f32))>> {
    if frame.camera.fold <= 0.0 {
        return None;
    }
    let fold = Fold::of(frame.width, frame.view_w, frame.height);
    if fold.rows < 2 {
        return None;
    }
    Some(
        (0..fold.rows)
            .rev()
            .map(|row| {
                let camera = frame.camera.row(&fold, frame.view_w, frame.height, row);
                let (from, to) = (row as f32 * fold.row_w, (row + 1) as f32 * fold.row_w);
                let margin = frame.view_w * 0.3;
                let mut part = frame.clone();
                part.camera = camera;
                // Every row but the furthest runs on from the one behind.
                part.blend_top = row + 1 < fold.rows;
                part.buildings
                    .retain(|building| building.x > from - margin && building.x < to + margin);
                part.things
                    .retain(|thing| thing.x > from - margin && thing.x < to + margin);
                part.people
                    .retain(|person| fold.row_of(person.along) == row);
                // A row behind shows its town, not its water's edge, which
                // the row in front of it covers: what stands on the water
                // there would stand on that row's field.
                if row > 0 {
                    let water = frame.front - 1.0;
                    part.buildings.retain(|building| building.base < water);
                    part.things.retain(|thing| thing.base < water);
                }
                // Too small to read on a postcard: the stakes and the bonds.
                part.plots.clear();
                part.bonds.clear();
                let left = camera
                    .project((frame.width, frame.view_w, frame.height), from, 0.0)
                    .0;
                let right = camera
                    .project((frame.width, frame.view_w, frame.height), to, 0.0)
                    .0;
                (part, (left.max(0.0), right.min(frame.view_w)))
            })
            .collect(),
    )
}

/// Paints a frame into `bounds` all at once, painting whatever still layer
/// is not painted yet right here: for a cover, the strip, a postcard and
/// the golden pictures.
pub fn paint(frame: &Frame, bounds: Bounds<Pixels>, window: &mut Window) {
    let ox = f32::from(bounds.origin.x);
    let oy = f32::from(bounds.origin.y);
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width < 2.0 || height < 2.0 {
        return;
    }
    let mut frame = frame.clone();
    painter::begin_frame(false);
    painter::sweep(window);
    fetch_pictures(&mut frame, window, true);
    let light = frame.light();
    let view = (width, height);
    // A folding place is painted a row at a time, back to front, each over
    // the haze laid on the one behind; otherwise all at once.
    let rows = rows_of(&frame).unwrap_or_else(|| vec![(frame.clone(), (0.0, width))]);
    let folded = rows.len() > 1;
    let back = std::sync::Arc::new(rows[0].0.clone());
    for (layer, drawn) in still_now(&back, window, view, &ALL[..2]) {
        draw_still(window, bounds, &drawn);
        match layer {
            Still::Sky => paint_clouds(window, &back, ox, oy, width, height),
            Still::Hills => paint_rising(window, &back, ox, oy, width, height),
            _ => {}
        }
    }
    let count = rows.len();
    for (row, (part, span)) in rows.into_iter().enumerate() {
        let part = std::sync::Arc::new(part);
        let clip = Bounds::new(
            point(px(ox + span.0), px(oy)),
            size_of(span.1 - span.0, height),
        );
        window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
            for (_, drawn) in still_now(&part, window, view, &ALL[2..]) {
                draw_still(window, bounds, &drawn);
            }
            paint_live(window, &part, ox, oy, width, height, light);
            if folded && row + 1 < count {
                paint_haze(window, &part, ox, oy, span, height);
            }
        });
    }
    paint_weather(
        window,
        &frame,
        ox,
        oy,
        width,
        height,
        (height / 848.0).clamp(0.3, 1.3),
    );
    paint_vignette_image(window, bounds);
}

/// A size in pixels.
pub(super) fn size_of(w: f32, h: f32) -> gpui::Size<Pixels> {
    size(px(w.max(0.0)), px(h.max(0.0)))
}

/// With `WORLD_GPUI_FRAME_LOG` set to a file, a line for every frame the
/// scene draws: milliseconds since the window's first frame, the window,
/// how many pictures of what the camera saw were not painted, how much of
/// the loading wash lay over it, whether every layer had settled on its
/// sharp pictures (1) or was still arriving (0), the residents and whole
/// buildings in view, how much of the rough painting showed (square
/// pixels, anywhere and on the moment's subject), whether the camera was
/// on its way somewhere (1), where it was bound (a new value is a camera
/// move), the kind of place painted and which World's colours, and the
/// view's area. The release screenshot harness reads it to find any
/// unfinished frame shown at a high moment, the rough painting on a
/// subject after a camera move, and a frame of another World.
pub(super) fn log_frame(window: &Window, wash: f32, frame: &Frame) {
    use std::io::Write;
    thread_local! {
        static LOG: std::cell::RefCell<Option<(std::fs::File, std::time::Instant)>> =
            std::cell::RefCell::new(
                std::env::var_os("WORLD_GPUI_FRAME_LOG")
                    .and_then(|path| {
                        std::fs::OpenOptions::new().create(true).append(true).open(path).ok()
                    })
                    .map(|file| (file, std::time::Instant::now())),
            );
    }
    // Who and what stand wholly inside the window, unless the place is
    // folded into a postcard (-1 then).
    let (people_seen, buildings_seen) = if frame.camera.fold > 0.0 {
        (-1, -1)
    } else {
        let zoom = frame.camera.zoom;
        let left = frame.camera.x - frame.view_w / (2.0 * zoom);
        let right = frame.camera.x + frame.view_w / (2.0 * zoom);
        (
            // People stand where the camera sees them already.
            frame
                .people
                .iter()
                .filter(|person| person.x > 0.0 && person.x < frame.view_w)
                .count() as i64,
            frame
                .buildings
                .iter()
                .filter(|building| {
                    building.x - building.w / 2.0 >= left && building.x + building.w / 2.0 <= right
                })
                .count() as i64,
        )
    };
    // Where the camera is bound (its target, the same while it glides and
    // after it settles): a new one is a camera move.
    let bound = frame.heading.unwrap_or(frame.camera);
    let mut target = Key::new("camera");
    target
        .float((bound.x * 10.0).round())
        .float((bound.y * 10.0).round())
        .float((bound.zoom * 1000.0).round())
        .float((bound.fold * 1000.0).round());
    // The living stage has no rough painting: the harness's columns for
    // how much of it showed (anywhere, and on the subject) are always 0.
    let (rough_any, rough_subject) = (0.0_f32, 0.0_f32);
    LOG.with(|log| {
        if let Some((file, began)) = log.borrow_mut().as_mut() {
            let _ = writeln!(
                file,
                "{} {} {} {:.2} {} {} {} {:.0} {:.0} {} {:x} {} {:x} {:.0}",
                began.elapsed().as_millis(),
                window_id(window),
                unpainted(window),
                wash,
                u8::from(settled(window)),
                people_seen,
                buildings_seen,
                rough_any,
                rough_subject,
                u8::from(frame.heading().is_some()),
                target.finish() & 0xffff_ffff,
                frame.setting.key(),
                world_tag(frame) & 0xffff_ffff,
                frame.view_w * frame.height,
            );
        }
    });
}

/// Whether the scene in `window` has been painted: every still layer has
/// arrived at least once. Until then nothing is labelled and nobody
/// speaks over it (the art bible's §6): a name over an empty meadow, or a
/// welcome said to fog, is a defect.
pub fn painted(window: &Window) -> bool {
    painter::synchronous() || PAINTED.with(|seen| seen.borrow().contains_key(&window_id(window)))
}

fn window_id(window: &Window) -> u64 {
    window.window_handle().window_id().as_u64()
}

thread_local! {
    /// The windows that have shown a frame under the loading wash.
    static WASHED: std::cell::RefCell<std::collections::HashSet<u64>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
    /// When each window's scene was first all painted.
    static PAINTED: std::cell::RefCell<std::collections::HashMap<u64, std::time::Instant>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// How long the loading wash takes to fade into the real paint.
pub(super) const WASH_FADE: f32 = 0.3;

/// How much of the loading wash lies over a window's scene this frame (0
/// to 1): all of it until every still layer has been shown once
/// (`all_shown`), then less and less over [`WASH_FADE`], or at once when
/// everything is held still. A window washes only as it opens: once its
/// place has been seen, a resize or a new hour never washes it again.
pub(super) fn wash_over(window: &Window, all_shown: bool, still: bool) -> f32 {
    let id = window_id(window);
    PAINTED.with(|seen| {
        let mut seen = seen.borrow_mut();
        let now = std::time::Instant::now();
        let since = match seen.get(&id) {
            Some(at) => *at,
            None if all_shown => {
                let first = !WASHED.with(|washed| washed.borrow().contains(&id));
                let at = if still || first {
                    now - std::time::Duration::from_secs_f32(WASH_FADE)
                } else {
                    now
                };
                seen.insert(id, at);
                at
            }
            None => {
                WASHED.with(|washed| washed.borrow_mut().insert(id));
                return 1.0;
            }
        };
        1.0 - ease((now.duration_since(since).as_secs_f32() / WASH_FADE).clamp(0.0, 1.0))
    })
}

/// The loading wash: the place before it is painted, as a painter blocks
/// it in first, the sky's own colours down to a soft horizon, the far
/// hills as a haze along it, the field and the near ground below, all
/// lit for the hour; `share` of it (0 to 1) over the scene at
/// (`x`, `y`), `w` by `h`.
pub(super) fn paint_wash(window: &mut dyn Brush, frame: &Frame, (x, y, w, h): Rect, share: f32) {
    let light = frame.light();
    let lit = |colour: Hsla| {
        let rgba: gpui::Rgba = colour.into();
        Hsla::from(gpui::Rgba {
            r: rgba.r * light[0],
            g: rgba.g * light[1],
            b: rgba.b * light[2],
            a: share,
        })
    };
    let (top, bottom) = sky_colours(frame);
    let (field, near) = land_colours(frame);
    let far = lit(mix(art::hex(frame.scenery.far), bottom, 0.35));
    let (top, bottom, field, near) = (
        top.opacity(share),
        bottom.opacity(share),
        lit(field),
        lit(near),
    );
    let horizon = y + frame.at(0.0, frame.horizon).1.clamp(0.0, h);
    let front = y + frame.at(0.0, frame.front).1.clamp(0.0, h);
    let soft = (h * 0.07).max(8.0);
    // The sky, down to the horizon.
    window.gradient(x, y, w, horizon - y, 180.0, (top, 0.0), (bottom, 1.0));
    // The ground, from a soft horizon down to the near ground.
    window.gradient(
        x,
        horizon - soft,
        w,
        soft * 2.0,
        180.0,
        (bottom, 0.0),
        (field, 1.0),
    );
    window.gradient(
        x,
        horizon + soft,
        w,
        (front - horizon - soft * 1.5).max(1.0),
        180.0,
        (field, 0.0),
        (mix(field, near, 0.35), 1.0),
    );
    window.gradient(
        x,
        front - soft * 0.5,
        w,
        soft,
        180.0,
        (mix(field, near, 0.35), 0.0),
        (near, 1.0),
    );
    window.rect(
        x,
        front + soft * 0.5,
        w,
        (y + h - front - soft * 0.5).max(0.0),
        0.0,
        near,
    );
    // The far hills, a haze along the horizon: a few broad soft strokes,
    // the same for the same place.
    let seed = seed_of_scenery(&frame.scenery);
    for stroke in 0..4_u32 {
        let pick = painter::hash2(stroke as i32, 7, seed);
        let along = x + w * (stroke as f32 + 0.25 + (pick % 50) as f32 / 100.0) / 4.0;
        let (rx, ry) = (w * (0.16 + ((pick >> 8) % 8) as f32 / 100.0), soft * 1.1);
        window.soft(
            along,
            horizon - ry * 0.35,
            rx,
            ry,
            soft * 2.0,
            far.opacity(0.55 * share),
        );
    }
}

impl Frame {
    /// The place's hero palette as this frame lights it (1.0 P5), each as
    /// red, green and blue from 0 to 1: the sky's top and its horizon, the
    /// far hills, the field, the near ground, the spine people walk along,
    /// and the glow its lamps and windows throw. What a Pack's `PlaceLook`
    /// and scenery make of the place, worked out as the painter works it
    /// out, so a test can hold places apart without drawing them.
    pub fn hero_colours(&self) -> [[f32; 3]; 7] {
        let light = self.light();
        let rgb = |colour: Hsla| {
            let rgba: gpui::Rgba = colour.into();
            [rgba.r, rgba.g, rgba.b]
        };
        let lit = |colour: Hsla| {
            let [r, g, b] = rgb(colour);
            [r * light[0], g * light[1], b * light[2]]
        };
        let (top, bottom) = sky_colours(self);
        let (field, near) = land_colours(self);
        let far = mix(art::hex(self.scenery.far), self.haze(), 0.35);
        let spine = self.spine().unwrap_or(near);
        [
            rgb(top),
            rgb(bottom),
            lit(far),
            lit(field),
            lit(near),
            lit(spine),
            rgb(self.glow()),
        ]
    }
}

/// The two still layers on the ground: the land itself, soft enough to
/// paint at half the display's resolution, and the buildings over it at
/// the full.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) enum Layer {
    Land,
    Buildings,
}

/// The stage rows (top and bottom, in stage pixels) a layer covers.
pub(super) fn layer_rows(frame: &Frame, layer: Layer) -> (f32, f32) {
    match layer {
        Layer::Land => (
            frame.horizon + (frame.base - frame.horizon) * 0.3 - 16.0,
            f32::MAX,
        ),
        Layer::Buildings => (
            ground_reach(frame),
            frame
                .buildings
                .iter()
                .map(|building| building.base + building.h * 0.4)
                .fold(frame.base, f32::max),
        ),
    }
}

/// The tiles of one layer the camera sees, at `scale` device pixels to a
/// stage pixel, as (column, row).
pub(super) fn tiles_in_view(
    frame: &Frame,
    layer: Layer,
    scale: f32,
    margin: i32,
) -> Vec<(i32, i32)> {
    let zoom = frame.camera.zoom;
    let tile = TILE as f32 / scale;
    let (x0, y0) = (
        frame.camera.x - frame.view_w / (2.0 * zoom),
        frame.camera.y - frame.height / (2.0 * zoom),
    );
    let (x1, y1) = (
        frame.camera.x + frame.view_w / (2.0 * zoom),
        frame.camera.y + frame.height / (2.0 * zoom),
    );
    let (top, bottom) = layer_rows(frame, layer);
    let rows = ((y0.max(top) / tile).floor() as i32)
        ..=((y1.min(bottom).min(frame.height + tile) / tile).floor() as i32);
    let columns =
        ((x0 / tile).floor() as i32 - margin)..=(((x1 - 0.01) / tile).floor() as i32 + margin);
    rows.flat_map(|row| columns.clone().map(move |column| (column, row)))
        .collect()
}

/// How high the ground layer reaches, in stage pixels: the field's top, or
/// the tallest building's roof and its glow.
pub(super) fn ground_reach(frame: &Frame) -> f32 {
    frame
        .buildings
        .iter()
        .map(|building| building.base - building.h * 1.45)
        .fold(
            frame.horizon + (frame.base - frame.horizon) * 0.3 - 20.0,
            f32::min,
        )
}

/// A World's cover: its landscape, what it has built, and its people and
/// buildings as it last stood, drawn the way its window draws them.
pub fn cover(
    scenery: Option<Scenery>,
    marks: &[MarkShape],
    cast: Vec<CanvasItem>,
    drawings: Vec<world_projection::Drawing>,
) -> gpui::Canvas<()> {
    cover_in(scenery, marks, cast, drawings, crate::scene::daylight_now())
}

/// [`cover`] in the light of one part of the day, whatever the clock says:
/// the title screen shows the town at dusk.
pub fn cover_in(
    scenery: Option<Scenery>,
    marks: &[MarkShape],
    cast: Vec<CanvasItem>,
    drawings: Vec<world_projection::Drawing>,
    daylight: crate::scene::Daylight,
) -> gpui::Canvas<()> {
    // A cast placed along a panorama is shown on one as wide, looking at
    // its middle.
    let width = cast
        .iter()
        .filter_map(|item| item.px)
        .fold(None, |most: Option<f32>, px| {
            Some(most.map_or(px, |most| most.max(px)))
        })
        .map(|most| most.ceil().max(1.0));
    // A cover keeps the cast, not the place's setting: it is read back
    // from what the cast is drawn as (art's, B).
    let setting = crate::works::setting_of_cast(cast.iter().filter_map(|item| item.art.as_deref()));
    let snapshot = ProjectionSnapshot {
        scenery,
        drawings,
        canvas: world_projection::CanvasProjection {
            items: cast,
            links: Vec::new(),
            marks: marks
                .iter()
                .map(|shape| world_projection::CanvasMark {
                    label: String::new(),
                    shape: *shape,
                    selection: None,
                })
                .collect(),
            width,
            setting: Some(setting.key().into()),
            ..Default::default()
        },
        ..ProjectionSnapshot::default()
    };
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let stage = stage(
                &snapshot,
                f32::from(bounds.size.width),
                f32::from(bounds.size.height),
            );
            let people = living(&stage, &snapshot, 0.0, daylight, &BTreeSet::new(), None);
            let frame = frame(
                &snapshot,
                &stage,
                &people,
                Camera::whole(&stage),
                0.0,
                daylight,
                &Glows::new(),
                1.0,
            );
            paint(&frame, bounds, window);
        },
    )
}
