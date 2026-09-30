//! The scene as GPUI shows it: which still layers and tiles a frame wants,
//! painting them off the window's thread and fading them in, the element
//! that draws the scene every frame, and covers.

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
        .add((frame.ice, frame.water));
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

    /// Whether `other` is this same look, only another drawing of the
    /// boil: shown in its place at once, never faded.
    pub(super) fn boils_into(&self, other: &Version) -> bool {
        self.key == other.key && self.scale == other.scale && self.boil != other.boil
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

/// Something a still layer draws this frame.
#[derive(Clone)]
pub(super) enum Drawn {
    Image(std::sync::Arc<gpui::RenderImage>, Rect, Rect),
    /// A plain colour while a tile is being painted.
    Fill(Rect, Hsla),
    /// The sky's two colours while the sky is being painted.
    Sky(Rect, Hsla, Hsla),
}

/// What a still layer draws this frame: the version settled on, and the
/// next one fading in over it, and how far; and pictures handed to the
/// display ahead of being seen, drawn nowhere.
#[derive(Clone, Default)]
pub(super) struct LayerPlan {
    pub(super) settled: Vec<Drawn>,
    pub(super) arriving: Vec<Drawn>,
    pub(super) fade: f32,
    pub(super) ahead: Vec<Drawn>,
    /// Whether any version of the layer is shown yet, or fading in.
    pub(super) shown: bool,
}

/// How long a new version of a still layer takes to fade in.
pub(super) const FADE: f32 = 0.3;

/// The version of each still layer a frame wants.
pub(super) fn versions(
    frame: &Frame,
    window: &Window,
    width: f32,
    height: f32,
    dpr: f32,
) -> [(Still, Version); 4] {
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
        .float(frame.building_h);
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
    let scale = painted_zoom(window, frame) * dpr;
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
            let scale = version.scale;
            let zoom = frame.camera.zoom;
            let side = TILE as f32 / scale * zoom;
            let (sx, sy) = frame.at(0.0, 0.0);
            let (sx, sy) = ((sx * dpr).round() / dpr, (sy * dpr).round() / dpr);
            let pad = LAND_PAD as f32 * side / TILE as f32;
            // The land, which is solid, runs a device pixel on under the
            // next tile's edge, so no seam of half-covered pixels shows;
            // the buildings, which are mostly clear, must not, or where
            // they overlap a shadow would be laid twice.
            let over = if layer == Layer::Land { 1.0 / dpr } else { 0.0 };
            let reach = frame.at(0.0, layer_rows(frame, layer).0).1.floor();
            tiles_in_view(frame, layer, scale, margin)
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
                            // Nothing of a tile shows above where its layer
                            // begins: no empty tile edge ever lies over the sky.
                            let top = y.max(reach);
                            (x, top, side + over, (y + side + over - top).max(0.0))
                        },
                        job: Box::new(move || match layer {
                            Layer::Land => paint_land_tile(&frame, column, row, scale),
                            Layer::Buildings => {
                                paint_building_tile(&frame, ground, column, row, scale)
                            }
                        }),
                    }
                })
                .collect()
        }
    }
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

/// What a still layer shows where its picture is not painted yet.
pub(super) fn stand_in(frame: &Frame, layer: Still, rect: Rect, light: [f32; 3]) -> Vec<Drawn> {
    let lit = |colour: Hsla| {
        let rgba: gpui::Rgba = colour.into();
        Hsla::from(gpui::Rgba {
            r: rgba.r * light[0],
            g: rgba.g * light[1],
            b: rgba.b * light[2],
            a: rgba.a,
        })
    };
    match layer {
        Still::Sky => {
            let (top, bottom) = sky_colours(frame);
            vec![Drawn::Sky(rect, top, bottom)]
        }
        Still::Land => {
            // The field's colour down to the water's edge, and the water's
            // (or the near ground's) below.
            let (ground, near) = land_colours(frame);
            let field = frame
                .at(0.0, frame.horizon + (frame.base - frame.horizon) * 0.3)
                .1;
            let front = frame.at(0.0, frame.front).1;
            let (top, bottom) = (rect.1, rect.1 + rect.3);
            let mut out = Vec::new();
            let (a, b) = (top.max(field), bottom.min(front));
            if b > a {
                out.push(Drawn::Fill((rect.0, a, rect.2, b - a), lit(ground)));
            }
            let a = top.max(front);
            if bottom > a {
                out.push(Drawn::Fill((rect.0, a, rect.2, bottom - a), lit(near)));
            }
            out
        }
        Still::Hills | Still::Buildings => Vec::new(),
    }
}

/// Where each still layer of the scene stands, per window and size: the
/// version shown, and the next one fading in with when it began.
pub(super) type Slots = std::collections::HashMap<
    (u64, Still, u32, u32, u32),
    (Option<Version>, Option<(Version, std::time::Instant)>),
>;

/// Works out what every still layer draws this frame, asking for whatever
/// is not painted yet. `now` paints it all right here (a cover, a test);
/// otherwise it is painted off the window's thread and fades in when it is
/// ready, over what was there, except with Reduce Motion.
pub(super) fn plan(
    frame: &std::sync::Arc<Frame>,
    window: &mut Window,
    (width, height): (f32, f32),
    now: bool,
    (slot, only): (u32, &[Still]),
) -> Vec<(Still, LayerPlan)> {
    use std::cell::RefCell;
    thread_local! {
        static SLOTS: RefCell<Slots> = RefCell::new(Slots::new());
    }
    let dpr = window.scale_factor().max(0.5);
    let light = light_at(frame.hour, frame.weather);
    let id = window.window_handle().window_id().as_u64();
    let instant = now || frame.still;
    painter::sweep(window);
    versions(frame, window, width, height, dpr)
        .into_iter()
        .filter(|(layer, _)| only.contains(layer))
        .map(|(layer, wanted)| {
            // Ask for what the camera sees, and a tile more each side.
            let margin = if now { 0 } else { 1 };
            let seen = pieces(frame, layer, wanted, width, height, dpr, 0)
                .iter()
                .map(|piece| piece.key)
                .collect::<BTreeSet<_>>();
            for piece in pieces(frame, layer, wanted, width, height, dpr, margin) {
                if !painter::painted(piece.key) {
                    painter::want(window, piece.key, now, piece.job);
                }
            }
            // Painted, and so ready to be shown: every piece in view.
            let complete = seen.iter().all(|key| painter::painted(*key));
            // Handed to the display: shown without a new picture copied.
            let handed = |version: Version| {
                pieces(frame, layer, version, width, height, dpr, 0)
                    .iter()
                    .all(|piece| painter::handed(piece.key))
            };
            let slot_key = (id, layer, slot, width as u32, height as u32);
            let clock = std::time::Instant::now();
            let mut early = None;
            let (shown, arriving) = SLOTS.with(|slots| {
                let mut slots = slots.borrow_mut();
                let slot = slots.entry(slot_key).or_insert((None, None));
                let boiled = slot.0.is_some_and(|shown| shown.boils_into(&wanted));
                if slot.0 == Some(wanted) {
                    slot.1 = None;
                } else if complete && (boiled || instant) && slot.0.is_some() && !now {
                    // The boil moves to its next drawing, and with Reduce
                    // Motion a new look replaces the old, at once and only
                    // once all of it is with the display: a still thing
                    // redrawn by hand, never two drawings faded into each
                    // other, and never a frame with a hole in it. Until
                    // then its pictures are handed over a frame's share at
                    // a time.
                    if handed(wanted) {
                        *slot = (Some(wanted), None);
                    } else {
                        early = Some(wanted);
                    }
                } else if complete && instant {
                    *slot = (Some(wanted), None);
                } else if complete && !boiled {
                    match slot.1 {
                        // Faded in, and every picture of it with the
                        // display: it settles.
                        Some((version, began)) if version == wanted => {
                            if clock.duration_since(began).as_secs_f32() >= FADE && handed(wanted) {
                                *slot = (Some(wanted), None);
                            }
                        }
                        _ => slot.1 = Some((wanted, clock)),
                    }
                }
                *slot
            });
            let draw = |version: Version, stand_ins: bool| {
                pieces(frame, layer, version, width, height, dpr, 0)
                    .into_iter()
                    .flat_map(|piece| match painter::ready(piece.key) {
                        Some(painter::Ready::Image(image, _)) => {
                            vec![Drawn::Image(image, piece.rect, piece.clip)]
                        }
                        Some(painter::Ready::Empty) => Vec::new(),
                        None if stand_ins => stand_in(frame, layer, piece.clip, light),
                        None => Vec::new(),
                    })
                    .collect::<Vec<_>>()
            };
            let settled = match (shown, layer) {
                (Some(version), _) => draw(version, true),
                (None, Still::Sky) => stand_in(frame, layer, (0.0, 0.0, width, height), light),
                (None, Still::Land) => pieces(frame, layer, wanted, width, height, dpr, 0)
                    .into_iter()
                    .flat_map(|piece| stand_in(frame, layer, piece.clip, light))
                    .collect(),
                (None, _) => Vec::new(),
            };
            let (arriving, fade) = match arriving {
                Some((version, began)) => (
                    draw(version, false),
                    (clock.duration_since(began).as_secs_f32() / FADE).clamp(0.0, 1.0),
                ),
                None => (Vec::new(), 0.0),
            };
            // Handed over ahead of being seen, a frame's share at a time:
            // the next drawing of the boil, and the tiles a pan reaches
            // next, so neither costs a frame all at once.
            let mut ahead = early
                .map(|version| draw(version, false))
                .unwrap_or_default();
            if let Some(version) = shown.filter(|_| !now) {
                for piece in pieces(frame, layer, version, width, height, dpr, margin) {
                    if seen.contains(&piece.key)
                        || painter::handed(piece.key)
                        || !painter::painted(piece.key)
                    {
                        continue;
                    }
                    if let Some(painter::Ready::Image(image, _)) = painter::ready(piece.key) {
                        ahead.push(Drawn::Image(image, piece.rect, piece.clip));
                    }
                }
            }
            let arrived = shown.is_some() || !arriving.is_empty();
            (
                layer,
                LayerPlan {
                    settled,
                    arriving,
                    fade: ease(fade),
                    ahead,
                    shown: arrived,
                },
            )
        })
        .collect()
}

/// Draws what a still layer plans, at the scene's top-left (`ox`, `oy`).
pub(super) fn draw_still(window: &mut Window, bounds: Bounds<Pixels>, drawn: &[Drawn]) {
    let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let at = |(x, y, w, h): Rect| Bounds::new(point(px(ox + x), px(oy + y)), size(px(w), px(h)));
    for item in drawn {
        match item {
            Drawn::Image(image, rect, clip) => {
                let clip = at(*clip).intersect(&bounds);
                let started = std::time::Instant::now();
                let _ = painter::timed("main: image", || {
                    window.paint_image(clip, at(*rect), Corners::default(), image.clone(), 0, false)
                });
                painter::slowest("main: one image", started.elapsed());
            }
            Drawn::Fill(rect, colour) => {
                let clip = at(*rect).intersect(&bounds);
                window.paint_quad(gpui::fill(clip, *colour));
            }
            Drawn::Sky(rect, top, bottom) => {
                let r = at(*rect);
                window.paint_quad(gpui::fill(
                    r,
                    gpui::linear_gradient(
                        180.0,
                        gpui::linear_color_stop(*top, 0.0),
                        gpui::linear_color_stop(*bottom, 0.55),
                    ),
                ));
            }
        }
    }
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
                part.buildings
                    .retain(|building| building.x > from - margin && building.x < to + margin);
                part.things
                    .retain(|thing| thing.x > from - margin && thing.x < to + margin);
                part.people
                    .retain(|person| fold.row_of(person.along) == row);
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
    fetch_pictures(&mut frame, window, true);
    let light = light_at(frame.hour, frame.weather);
    let view = (width, height);
    // A folding place is painted a row at a time, back to front, each over
    // the haze laid on the one behind; otherwise all at once.
    let rows = rows_of(&frame).unwrap_or_else(|| vec![(frame.clone(), (0.0, width))]);
    let folded = rows.len() > 1;
    let back = std::sync::Arc::new(rows[0].0.clone());
    for (layer, plan) in plan(&back, window, view, true, (0, &ALL[..2])) {
        draw_still(window, bounds, &plan.settled);
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
            for (_, plan) in plan(&part, window, view, true, (1 + row as u32, &ALL[2..])) {
                draw_still(window, bounds, &plan.settled);
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

/// Live drawing over a scene: the window, the frame, and the scene's
/// top-left corner, width and height.
pub(super) type LivePaint = dyn Fn(&mut Window, &Frame, f32, f32, f32, f32);

/// The scene as the World window shows it: the still layers painted off
/// the window's thread and faded in as they arrive, and over and between
/// them everything that moves, drawn live every frame. Nothing here waits
/// for a picture. A folding place is drawn a row at a time, back to front,
/// each row's stills and its live drawing clipped to its stretch of the
/// window.
pub fn scene(frame: Frame, window: &mut Window) -> gpui::Div {
    use gpui::{ParentElement, Styled};
    let width = frame.view_w;
    let height = frame.height;
    let mut frame = frame;
    painter::timed("main: pictures", || {
        fetch_pictures(&mut frame, window, painter::synchronous())
    });
    let rows = rows_of(&frame).unwrap_or_else(|| vec![(frame.clone(), (0.0, width))]);
    let folded = rows.len() > 1;
    let frame = std::sync::Arc::new(frame);
    let sync = painter::synchronous();
    // One frame's share of new pictures for all its layers.
    painter::begin_frame(!sync);
    let back = std::sync::Arc::new(rows[0].0.clone());
    let mut planned = vec![(
        back.clone(),
        None,
        painter::timed("main: plan", || {
            plan(&back, window, (width, height), sync, (0, &ALL[..2]))
        }),
    )];
    for (row, (part, span)) in rows.into_iter().enumerate() {
        let part = std::sync::Arc::new(part);
        let layers = painter::timed("main: plan", || {
            plan(
                &part,
                window,
                (width, height),
                sync,
                (1 + row as u32, &ALL[2..]),
            )
        });
        planned.push((part, Some(span), layers));
    }
    // Until every still layer has arrived the first time, the place is a
    // soft wash of its own sky and ground; then the wash fades away into
    // the real paint.
    let all_shown = planned
        .iter()
        .all(|(_, _, layers)| layers.iter().all(|(_, plan)| plan.shown));
    let wash = if sync {
        0.0
    } else {
        wash_over(window, all_shown, frame.still)
    };
    if wash > 0.0 {
        window.request_animation_frame();
    }
    // While pictures are being painted or fading in, keep drawing frames,
    // so each is shown as soon as it is ready, however still the window.
    if !painter::idle()
        || planned
            .iter()
            .any(|(_, _, layers)| layers.iter().any(|(_, plan)| !plan.arriving.is_empty()))
    {
        window.request_animation_frame();
    }
    let clip_to = |span: Option<(f32, f32)>, bounds: Bounds<Pixels>| {
        span.map(|(left, right)| gpui::ContentMask {
            bounds: Bounds::new(
                point(bounds.origin.x + px(left), bounds.origin.y),
                size_of(right - left, f32::from(bounds.size.height)),
            ),
        })
    };
    let still = |drawn: Vec<Drawn>, span: Option<(f32, f32)>| {
        gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window: &mut Window, _| {
                let started = std::time::Instant::now();
                window.with_content_mask(clip_to(span, bounds), |window| {
                    painter::timed("main: draw still", || draw_still(window, bounds, &drawn))
                });
                painter::note_drawing(started.elapsed());
                painter::note_frame(started.elapsed());
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    };
    // Pictures handed to the display ahead of being seen: drawn under a
    // mask that shows nothing.
    let ahead = |drawn: Vec<Drawn>| {
        gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window: &mut Window, _| {
                let started = std::time::Instant::now();
                let nowhere = gpui::ContentMask {
                    bounds: Bounds::new(bounds.origin, size_of(0.0, 0.0)),
                };
                window.with_content_mask(Some(nowhere), |window| {
                    painter::timed("main: draw still", || draw_still(window, bounds, &drawn))
                });
                painter::note_drawing(started.elapsed());
                painter::note_frame(started.elapsed());
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    };
    let live = |frame: std::sync::Arc<Frame>, span: Option<(f32, f32)>, paint: Box<LivePaint>| {
        gpui::canvas(
            |_, _, _| (),
            move |bounds, _, window: &mut Window, _| {
                let started = std::time::Instant::now();
                window.with_content_mask(clip_to(span, bounds), |window| {
                    painter::timed("main: live", || {
                        paint(
                            window,
                            &frame,
                            f32::from(bounds.origin.x),
                            f32::from(bounds.origin.y),
                            f32::from(bounds.size.width),
                            f32::from(bounds.size.height),
                        )
                    })
                });
                painter::note_frame(started.elapsed());
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    };
    let mut root = gpui::div().absolute().top_0().left_0().size_full();
    // For looking into how the scene is made: `WORLD_GPUI_HIDE=hills,sky`
    // leaves those still layers out.
    let hidden = std::env::var("WORLD_GPUI_HIDE").unwrap_or_default();
    let count = planned.len();
    for (position, (part, span, layers)) in planned.into_iter().enumerate() {
        let layers = layers
            .into_iter()
            .filter(|(layer, _)| !hidden.contains(&format!("{layer:?}").to_lowercase()));
        for (layer, plan) in layers {
            if !plan.ahead.is_empty() {
                root = root.child(ahead(plan.ahead));
            }
            root = root.child(still(plan.settled, span));
            if !plan.arriving.is_empty() {
                root = root.child(still(plan.arriving, span).opacity(plan.fade));
            }
            match layer {
                Still::Sky => {
                    root = root.child(live(
                        part.clone(),
                        None,
                        Box::new(|window, frame, ox, oy, w, h| {
                            paint_clouds(window, frame, ox, oy, w, h)
                        }),
                    ))
                }
                Still::Hills => {
                    root = root.child(live(
                        part.clone(),
                        None,
                        Box::new(|window, frame, ox, oy, w, h| {
                            paint_rising(window, frame, ox, oy, w, h)
                        }),
                    ))
                }
                _ => {}
            }
        }
        // Each row (or the whole place) lives over its own stills.
        if let Some(span) = span {
            let haze = folded && position + 1 < count;
            root = root.child(live(
                part,
                Some(span),
                Box::new(move |window, frame, ox, oy, w, h| {
                    let light = light_at(frame.hour, frame.weather);
                    paint_live(window, frame, ox, oy, w, h, light);
                    if haze {
                        paint_haze(window, frame, ox, oy, span, h);
                    }
                }),
            ));
        }
    }
    root.child(live(
        frame,
        None,
        Box::new(move |window, frame, ox, oy, w, h| {
            if wash > 0.0 {
                paint_wash(window, frame, (ox, oy, w, h), wash);
            }
            paint_weather(window, frame, ox, oy, w, h, (h / 848.0).clamp(0.3, 1.3));
            let bounds = Bounds::new(point(px(ox), px(oy)), size(px(w), px(h)));
            if painter::synchronous() {
                paint_vignette_image(window, bounds);
            } else {
                // Not worth a hitch: until it is painted, there is none.
                let dpr = window.scale_factor().max(0.5);
                let mut key = Key::new("vignette");
                key.float(w).float(h).float(dpr);
                let key = key.finish();
                match painter::ready(key) {
                    Some(painter::Ready::Image(image, _)) => {
                        let _ =
                            window.paint_image(bounds, bounds, Corners::default(), image, 0, false);
                    }
                    _ => painter::want(
                        window,
                        key,
                        false,
                        Box::new(move || paint_vignette(w, h, dpr * 0.25)),
                    ),
                }
            }
        }),
    ))
}

/// How long the loading wash takes to fade into the real paint.
pub(super) const WASH_FADE: f32 = 0.4;

/// How much of the loading wash lies over a window's scene this frame (0
/// to 1): all of it until every still layer has been shown once
/// (`all_shown`), then less and less over [`WASH_FADE`], or at once when
/// everything is held still. A window washes only as it opens: once its
/// place has been seen, a resize or a new hour never washes it again.
fn wash_over(window: &Window, all_shown: bool, still: bool) -> f32 {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        static SEEN: RefCell<HashMap<u64, std::time::Instant>> = RefCell::new(HashMap::new());
    }
    let id = window.window_handle().window_id().as_u64();
    SEEN.with(|seen| {
        let mut seen = seen.borrow_mut();
        let now = std::time::Instant::now();
        let since = match seen.get(&id) {
            Some(at) => *at,
            None if all_shown => {
                let at = if still {
                    now - std::time::Duration::from_secs_f32(WASH_FADE)
                } else {
                    now
                };
                seen.insert(id, at);
                at
            }
            None => return 1.0,
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
    let light = light_at(frame.hour, frame.weather);
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

/// A window and a view's size in it.
pub(super) type ViewId = (u64, u32, u32);

/// The zoom the ground is painted at: the camera's own when it is still,
/// and while it moves the last one painted (scaled), so a glide does not
/// paint every frame.
pub(super) fn painted_zoom(window: &Window, frame: &Frame) -> f32 {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        static ZOOMS: RefCell<HashMap<ViewId, (f32, f32)>> = RefCell::new(HashMap::new());
    }
    let zoom = frame.camera.zoom;
    let exact = (zoom * 100.0).round() / 100.0;
    let id = (
        window.window_handle().window_id().as_u64(),
        frame.view_w as u32,
        frame.height as u32,
    );
    ZOOMS.with(|zooms| {
        let mut zooms = zooms.borrow_mut();
        let entry = zooms.entry(id).or_insert((zoom, exact));
        let moving = (zoom - entry.0).abs() > 1e-4;
        entry.0 = zoom;
        if !moving || (entry.1 / zoom - 1.0).abs() > 0.5 {
            entry.1 = exact;
        }
        entry.1.max(0.05)
    })
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
    let daylight = crate::scene::daylight_now();
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
