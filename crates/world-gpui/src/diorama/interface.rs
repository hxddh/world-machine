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
    /// Buildings (by item index) the version settled on does not hold yet:
    /// just built, and drawn live until their picture is painted, so a
    /// build shows at once however slow the painting (scene's, A1).
    pub(super) live: Vec<usize>,
    /// Whether the layer is still on its way to what the frame wants (a
    /// look arriving, pictures not yet with the display, its rough
    /// painting not whole): the window keeps drawing frames until it is
    /// there, however still it is otherwise.
    pub(super) waiting: bool,
}

/// What stands in each version of the buildings layer, by its key: each
/// building's place and shape, for telling which a version shown still
/// lacks.
fn standing(frame: &Frame) -> BTreeSet<(i32, u8)> {
    frame
        .buildings
        .iter()
        .filter(|building| !building.moving())
        .map(|building| (building.x.round() as i32, building.shape as u8))
        .collect()
}

thread_local! {
    static STANDING: std::cell::RefCell<std::collections::HashMap<u64, BTreeSet<(i32, u8)>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
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

/// How many device pixels to a stage pixel the rough painting of the place
/// is made at, for each display pixel: a quarter, so the whole panorama
/// paints in a moment.
pub(super) const ROUGH: f32 = 0.25;

/// The rough painting of the land or the buildings: the place's own
/// drawings, painted small across the whole panorama whatever the camera
/// sees, so wherever it goes (a pan, Find, a zoom) something painted is
/// there while the sharp tiles are painted. It depends on the look but
/// never on the camera.
pub(super) fn rough_version(frame: &Frame, dpr: f32) -> Version {
    let scale = (ROUGH * dpr).max(0.2);
    let mut key = ground_key(frame, scale);
    key.add("rough");
    Version {
        key: key.finish(),
        scale,
        boil: 0,
    }
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

/// A still layer of one window's scene at one size: the window, the
/// layer, its slot (the back layers, or a row of a folded postcard), and
/// the size.
pub(super) type SlotKey = (u64, Still, u32, u32, u32);

/// Where each still layer of the scene stands, per window and size: the
/// version shown, and the next one fading in with when it began.
pub(super) type Slots =
    std::collections::HashMap<SlotKey, (Option<Version>, Option<(Version, std::time::Instant)>)>;

/// The rough painting of a layer a frame can use as its stand-in: each
/// rough picture with the display, with where it is shown and the part of
/// it that shows.
type Rough = Vec<(std::sync::Arc<gpui::RenderImage>, Rect, Rect)>;

/// Where `rect` of a layer is not painted yet, the rough painting under it:
/// each rough picture that reaches into it, clipped to it.
fn rough_over(rough: &Rough, rect: Rect) -> Vec<Drawn> {
    let (x, y, w, h) = rect;
    rough
        .iter()
        .filter_map(|(image, at, clip)| {
            let x0 = x.max(clip.0);
            let y0 = y.max(clip.1);
            let x1 = (x + w).min(clip.0 + clip.2);
            let y1 = (y + h).min(clip.1 + clip.3);
            (x1 > x0 && y1 > y0)
                .then(|| Drawn::Image(image.clone(), *at, (x0, y0, x1 - x0, y1 - y0)))
        })
        .collect()
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
/// saw painted, and the moment's subject sharp (not its rough painting):
/// the return film's words wait for it.
pub fn view_painted(window: &Window) -> bool {
    painter::synchronous()
        || (painted(window) && unpainted(window) == 0 && rough_shown(window).1 <= 0.0)
}

/// Works out what every still layer draws this frame, asking for whatever
/// is not painted yet. `now` paints it all right here (a cover, a test);
/// otherwise it is painted off the window's thread and fades in when it is
/// ready, over what was there, except with Reduce Motion. Wherever the
/// land or the buildings are not painted yet, their rough painting stands
/// in (see [`rough_version`]); only before even that is painted does a
/// flat stand-in show, under the loading wash.
pub(super) fn plan(
    frame: &std::sync::Arc<Frame>,
    window: &mut Window,
    (width, height): (f32, f32),
    now: bool,
    (slot, only): (u32, &[Still]),
    whole: &std::sync::Arc<Frame>,
) -> Vec<(Still, LayerPlan)> {
    let dpr = window.scale_factor().max(0.5);
    let light = frame.light();
    let id = window.window_handle().window_id().as_u64();
    if slot == 0 && !now {
        forget_closed(id);
    }
    painter::sweep(window);
    let mut missing = 0;
    let tag = world_tag(whole);
    // Where the camera is going, seen from there: what it will see is
    // painted first, the moment's subject first of all, and handed to the
    // display ahead, so the camera arrives on sharp paint.
    let heading = if now {
        None
    } else {
        frame
            .heading()
            .and_then(|to| seen_from(frame, to, frame.subject(), dpr))
    };
    // Where the rough painting shows this frame, in square pixels: anywhere
    // in view, and on the moment's subject.
    let subject_on_screen = subject_rect(frame);
    let mut rough_seen = (0.0_f32, 0.0_f32);
    let mut heading_ahead = HeadingAhead::default();
    if let Some((there, key, scale)) = &heading {
        heading_ahead = ask_ahead(window, there, (*key, *scale), dpr, only, true);
    }
    // Once where the camera is now (or going) is sharp, where it goes after
    // that (the return film's next beat) is painted and handed over ahead,
    // behind everything else.
    // Whether the last whole frame showed no rough painting anywhere.
    if slot == 0 && !now {
        let sharp = ROUGH_SEEN.with(|seen| {
            seen.borrow()
                .get(&id)
                .is_some_and(|(anywhere, _)| *anywhere <= 0.0)
        });
        SHARP_BEFORE.with(|before| before.borrow_mut().insert(id, sharp));
    }
    let sharp_before =
        SHARP_BEFORE.with(|before| before.borrow().get(&id).copied().unwrap_or(false));
    if !now && heading.is_none() && sharp_before {
        if let Some((there, key, scale)) = frame
            .next
            .and_then(|(to, subject)| seen_from(frame, to, Some(subject), dpr))
        {
            let next = ask_ahead(window, &there, (key, scale), dpr, only, false);
            for (layer, drawn) in next.ahead {
                heading_ahead.ahead.entry(layer).or_default().extend(drawn);
            }
        }
    }
    let plans = versions(frame, window, width, height, dpr)
        .into_iter()
        .filter(|(layer, _)| only.contains(layer))
        .map(|(layer, wanted)| {
            let slot_key = (id, layer, slot, width as u32, height as u32);
            // Where a layer has nothing at all to show at this size yet (the
            // window's first frame, or its first at a new size), what the
            // camera sees of it is painted right here, before the frame is
            // shown: the sky and the hills, and the rough painting of the
            // land and the buildings. So no frame is ever a wash or a flat
            // stand-in: the first is the place in its own drawings, soft
            // (the v0.28 art director's first note). Everything sharp
            // still comes off the window's thread.
            let bare = !now
                && SLOTS.with(|slots| {
                    slots
                        .borrow()
                        .get(&slot_key)
                        .is_none_or(|(shown, arriving)| shown.is_none() && arriving.is_none())
                });
            let rough_bare = !now
                && ROUGHS.with(|roughs| {
                    roughs
                        .borrow()
                        .get(&slot_key)
                        .is_none_or(|(_, of)| *of != tag)
                });
            let instant = now || frame.still || bare;
            // The still things boil (another drawing of the same look, a
            // few times a second) only once a look has settled: until then
            // the drawing being painted is held, so a slow machine is never
            // sent after a new drawing before the last is done, and the
            // layer always arrives.
            let wanted = SLOTS.with(|slots| {
                let slots = slots.borrow();
                match slots.get(&slot_key) {
                    Some((Some(shown), _)) if shown.key == wanted.key => wanted,
                    Some((_, Some((arriving, _))))
                        if arriving.key == wanted.key && arriving.scale == wanted.scale =>
                    {
                        *arriving
                    }
                    _ => Version { boil: 0, ..wanted },
                }
            });
            // What the camera needs first to show anything at all: the sky,
            // the hills, and the rough painting of the place.
            let first = matches!(layer, Still::Sky | Still::Hills);
            // Ask for what the camera sees, and a tile more each side.
            let margin = if now { 0 } else { 1 };
            let seen = pieces(frame, layer, wanted, width, height, dpr, 0)
                .iter()
                .map(|piece| piece.key)
                .collect::<BTreeSet<_>>();
            for piece in pieces(frame, layer, wanted, width, height, dpr, margin) {
                if !painter::painted(piece.key) {
                    let soon = first && seen.contains(&piece.key);
                    if bare && soon {
                        painter::unlimit();
                    }
                    painter::ask(window, piece.key, now || (bare && soon), soon, piece.job);
                }
            }
            // A place opening: what the camera sees is painted sharp right
            // here, across the painter's threads, before the first frame is
            // shown (v0.28 opened on the rough painting for a second or two,
            // the moment's subject a blur). The margin a pan reaches next
            // comes off the window's thread.
            // Only as the window opens: at a new size (a resize, dragged)
            // the rough painting stands in as before, never a long frame.
            if bare && !now && !painted(window) && matches!(layer, Still::Land | Still::Buildings) {
                let mut first: Vec<(f32, u64, painter::Painting<'static>)> =
                    pieces(frame, layer, wanted, width, height, dpr, 0)
                        .into_iter()
                        .filter(|piece| !painter::painted(piece.key))
                        .map(|piece| {
                            // The subject's first, should the work be split.
                            let off = -area(overlap(piece.clip, subject_on_screen));
                            (off, piece.key, piece.job)
                        })
                        .collect();
                first.sort_by(|a, b| a.0.total_cmp(&b.0));
                let first = first
                    .into_iter()
                    .map(|(_, key, job)| (key, job))
                    .collect::<Vec<_>>();
                if !first.is_empty() {
                    painter::unlimit();
                    painter::cached_all(window, first);
                }
            }
            // The rough painting, asked for ahead of everything sharp, and
            // handed to the display a frame's share at a time like any
            // picture; it stands in only once all of it is there.
            let mut ahead = Vec::new();
            let mut rough_whole = true;
            let mut blank: Vec<Rect> = Vec::new();
            let rough: Rough = match layer {
                Still::Land | Still::Buildings if !now && !rough_off() => {
                    let tile_layer = if layer == Still::Land {
                        Layer::Land
                    } else {
                        Layer::Buildings
                    };
                    // One rough painting of the whole place serves every
                    // row of a folded postcard, seen through each row's
                    // own camera.
                    let through = if std::sync::Arc::ptr_eq(frame, whole) {
                        whole.clone()
                    } else {
                        let mut seen_from = (**whole).clone();
                        seen_from.camera = frame.camera;
                        std::sync::Arc::new(seen_from)
                    };
                    let version = rough_version(whole, dpr);
                    let all = tile_pieces(
                        &through,
                        tile_layer,
                        version,
                        dpr,
                        tiles_of_stage(frame, tile_layer, version.scale),
                    );
                    // What the camera sees first, then the rest of the
                    // place: it stands in as soon as what is seen is there.
                    let seen_now = |clip: &Rect| {
                        clip.0 < width
                            && clip.0 + clip.2 > 0.0
                            && clip.1 < height
                            && clip.1 + clip.3 > 0.0
                    };
                    let (mut in_view, mut whole) = (true, true);
                    let (near, far): (Vec<_>, Vec<_>) =
                        all.into_iter().partition(|piece| seen_now(&piece.clip));
                    for (piece, seen) in near
                        .into_iter()
                        .map(|piece| (piece, true))
                        .chain(far.into_iter().map(|piece| (piece, false)))
                    {
                        if !painter::painted(piece.key) {
                            // With no rough painting of the layer at all yet,
                            // what the camera sees of it is painted right
                            // here: it is small, and it is the frame's
                            // stand-in for everything sharp. A new look's is
                            // painted elsewhere while the last one stands in.
                            painter::ask(window, piece.key, rough_bare && seen, true, piece.job);
                            if !painter::painted(piece.key) {
                                whole = false;
                                in_view &= !seen;
                            }
                        } else if !painter::handed(piece.key) {
                            whole = false;
                            let ready = if rough_bare && seen {
                                painter::ready_now(piece.key)
                            } else {
                                painter::ready(piece.key)
                            };
                            if let Some(painter::Ready::Image(image, _)) = ready {
                                ahead.push(Drawn::Image(image, piece.rect, piece.clip));
                            } else {
                                in_view &= !seen;
                            }
                        }
                    }
                    rough_whole = whole;
                    // Only this World's: another World's rough painting
                    // never stands in (a wash of this place's own colours
                    // does, below).
                    let usable = ROUGHS.with(|roughs| {
                        let mut roughs = roughs.borrow_mut();
                        if in_view {
                            roughs.insert(slot_key, (version, tag));
                        }
                        roughs
                            .get(&slot_key)
                            .filter(|(_, of)| *of == tag)
                            .map(|(version, _)| *version)
                    });
                    usable
                        .map(|version| {
                            tile_pieces(
                                &through,
                                tile_layer,
                                version,
                                dpr,
                                tiles_of_stage(frame, tile_layer, version.scale),
                            )
                            .into_iter()
                            .filter_map(|piece| {
                                match if seen_now(&piece.clip) {
                                    // Seen and missing (the camera got ahead
                                    // of the painting): painted right here.
                                    if !painter::painted(piece.key) {
                                        painter::ask(window, piece.key, true, true, piece.job);
                                    }
                                    if rough_bare {
                                        painter::ready_now(piece.key)
                                    } else {
                                        painter::ready(piece.key)
                                    }
                                } else {
                                    painter::ready(piece.key)
                                } {
                                    Some(painter::Ready::Image(image, _)) => {
                                        Some((image, piece.rect, piece.clip))
                                    }
                                    // Painted, and nothing there: open ground or
                                    // sky, which needs no stand-in.
                                    Some(painter::Ready::Empty) => {
                                        blank.push(piece.clip);
                                        None
                                    }
                                    None => None,
                                }
                            })
                            .collect()
                        })
                        .unwrap_or_default()
                }
                _ => Vec::new(),
            };
            let has_rough = !rough.is_empty() || !blank.is_empty();
            // On its way somewhere, what of where the camera is going is
            // with the display already stands in where the version on show
            // has nothing (asked for and handed over first, above).
            let mut heading_shown: Vec<HeadingTile> = Vec::new();
            let heading_waiting = heading_ahead.waiting.contains(&layer);
            ahead.extend(heading_ahead.ahead.remove(&layer).unwrap_or_default());
            if let (Some((_, key, scale)), Some(tile_layer)) = (&heading, ground_layer(layer)) {
                let version = Version {
                    key: *key,
                    scale: if layer == Still::Land {
                        scale * 0.5
                    } else {
                        *scale
                    },
                    boil: 0,
                };
                if version.key != wanted.key || version.scale != wanted.scale {
                    for piece in tile_pieces(
                        frame,
                        tile_layer,
                        version,
                        dpr,
                        tiles_in_view(frame, tile_layer, version.scale, 0),
                    ) {
                        if !painter::handed(piece.key) {
                            continue;
                        }
                        match painter::ready(piece.key) {
                            Some(painter::Ready::Image(image, _)) => {
                                heading_shown.push((piece.clip, Some((image, piece.rect))))
                            }
                            Some(painter::Ready::Empty) => heading_shown.push((piece.clip, None)),
                            None => {}
                        }
                    }
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
            // Where a picture is not painted (or not yet with the display),
            // the rough painting under it; only without one, a flat stand-in
            // (or nothing), and the frame is unfinished.
            let mut gaps = 0;
            let rough_area = std::cell::Cell::new((0.0_f32, 0.0_f32));
            let fill = |clip: Rect, gaps: &mut usize| {
                let under = rough_over(&rough, clip);
                // Every corner of it in rough tiles painted empty.
                let nothing_there = [
                    (clip.0 + 0.5, clip.1 + 0.5),
                    (clip.0 + clip.2 - 0.5, clip.1 + 0.5),
                    (clip.0 + 0.5, clip.1 + clip.3 - 0.5),
                    (clip.0 + clip.2 - 0.5, clip.1 + clip.3 - 0.5),
                ]
                .iter()
                .all(|(x, y)| {
                    blank
                        .iter()
                        .any(|b| *x >= b.0 && *x <= b.0 + b.2 && *y >= b.1 && *y <= b.1 + b.3)
                });
                if under.is_empty() && nothing_there {
                    Vec::new()
                } else if under.is_empty() {
                    *gaps += 1;
                    stand_in(frame, layer, clip, light)
                } else {
                    let view = (0.0, 0.0, width, height);
                    let seen = |rect: Rect| area(overlap(rect, view));
                    let on_subject =
                        |rect: Rect| area(overlap(overlap(rect, view), subject_on_screen));
                    // Only where the rough painting has something to show:
                    // one painted empty is open ground, the same either way.
                    if !nothing_there {
                        rough_area.set((
                            rough_area.get().0 + seen(clip),
                            rough_area.get().1 + on_subject(clip),
                        ));
                    }
                    under
                }
            };
            // Missing pieces side by side in a row are filled as one, so the
            // rough painting under a stretch the camera has just reached is
            // drawn once rather than once a piece.
            let fill_all = |mut missing: Vec<Rect>, gaps: &mut usize| {
                missing.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
                let mut runs: Vec<(Rect, usize)> = Vec::new();
                for clip in missing {
                    match runs.last_mut() {
                        Some((run, count))
                            if run.1 == clip.1
                                && run.3 == clip.3
                                && (run.0 + run.2 - clip.0).abs() < 0.5 =>
                        {
                            run.2 += clip.2;
                            *count += 1;
                        }
                        _ => runs.push((clip, 1)),
                    }
                }
                runs.into_iter()
                    .flat_map(|(run, count)| {
                        let mut one = 0;
                        let drawn = fill(run, &mut one);
                        *gaps += one * count;
                        drawn
                    })
                    .collect::<Vec<_>>()
            };
            // What of the look arriving is painted and with the display.
            // Where the version on show lacks a piece and the arriving one
            // has it, that piece is shown whole at once, never faded in over
            // the rough painting: the rough would show through what is
            // transparent in it (its shadows doubled, its tile's edge drawn).
            let coming: Vec<(u64, std::sync::Arc<gpui::RenderImage>, Rect, Rect)> = arriving
                .map(|(version, _)| {
                    pieces(frame, layer, version, width, height, dpr, 0)
                        .into_iter()
                        .filter_map(|piece| match painter::ready(piece.key) {
                            Some(painter::Ready::Image(image, _)) => {
                                Some((piece.key, image, piece.rect, piece.clip))
                            }
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            // What of it is painted and has nothing in it.
            let coming_empty: Vec<Rect> = arriving
                .map(|(version, _)| {
                    pieces(frame, layer, version, width, height, dpr, 0)
                        .into_iter()
                        .filter(|piece| {
                            matches!(painter::ready(piece.key), Some(painter::Ready::Empty))
                        })
                        .map(|piece| piece.clip)
                        .collect()
                })
                .unwrap_or_default();
            let same = |a: Rect, b: Rect| {
                (a.0 - b.0).abs() < 0.5
                    && (a.1 - b.1).abs() < 0.5
                    && (a.2 - b.2).abs() < 0.5
                    && (a.3 - b.3).abs() < 0.5
            };
            // A piece the version on show lacks: the arriving one's, whole,
            // if it has it; else left to the rough painting where nothing
            // arriving reaches.
            let instead = |clip: Rect, missing: &mut Vec<Rect>| -> Vec<Drawn> {
                if let Some((_, image, rect, clip)) = coming.iter().find(|c| same(c.3, clip)) {
                    vec![Drawn::Image(image.clone(), *rect, *clip)]
                } else {
                    // On another grid (the camera zoomed, or on its way
                    // somewhere): what of the look arriving, or of where the
                    // camera is going, reaches into it, and the rough
                    // painting only where nothing sharp does.
                    let mut drawn = Vec::new();
                    let mut cover = Vec::new();
                    for (image, rect, at) in coming
                        .iter()
                        .map(|c| (Some(&c.1), c.2, c.3))
                        .chain(coming_empty.iter().map(|at| (None, *at, *at)))
                        .chain(heading_shown.iter().map(|(at, made)| {
                            (
                                made.as_ref().map(|m| &m.0),
                                made.as_ref().map_or(*at, |m| m.1),
                                *at,
                            )
                        }))
                    {
                        let part = overlap(clip, at);
                        if area(part) <= 0.0 || cover.iter().any(|c| contains(*c, part)) {
                            continue;
                        }
                        if let Some(image) = image {
                            drawn.push(Drawn::Image(image.clone(), rect, part));
                        }
                        cover.push(at);
                    }
                    missing.extend(uncovered(clip, &cover));
                    drawn
                }
            };
            let draw = |version: Version, stand_ins: bool, gaps: &mut usize| {
                let mut missing = Vec::new();
                let mut drawn = pieces(frame, layer, version, width, height, dpr, 0)
                    .into_iter()
                    .flat_map(|piece| match painter::ready(piece.key) {
                        Some(painter::Ready::Image(image, _)) => {
                            vec![Drawn::Image(image, piece.rect, piece.clip)]
                        }
                        Some(painter::Ready::Empty) => Vec::new(),
                        // The sky and the hills have no rough painting: while
                        // the layer has nothing else to show, a piece of it
                        // still missing is painted right here rather than
                        // shown as a flat stand-in.
                        None if stand_ins && bare && matches!(layer, Still::Sky | Still::Hills) => {
                            let (key, rect, clip) = (piece.key, piece.rect, piece.clip);
                            painter::unlimit();
                            painter::ask(window, key, true, true, piece.job);
                            match painter::ready(key) {
                                Some(painter::Ready::Image(image, _)) => {
                                    vec![Drawn::Image(image, rect, clip)]
                                }
                                Some(painter::Ready::Empty) => Vec::new(),
                                None => fill(clip, gaps),
                            }
                        }
                        None if stand_ins => instead(piece.clip, &mut missing),
                        None => Vec::new(),
                    })
                    .collect::<Vec<_>>();
                drawn.extend(fill_all(missing, gaps));
                drawn
            };
            let settled = match (shown, layer) {
                (Some(version), _) => draw(version, true, &mut gaps),
                (None, Still::Sky) => {
                    gaps += 1;
                    stand_in(frame, layer, (0.0, 0.0, width, height), light)
                }
                (None, Still::Hills) => {
                    gaps += 1;
                    Vec::new()
                }
                (None, _) => {
                    // Nothing settled yet (the place opening): what of the
                    // look wanted is painted shows at once, the rest of it
                    // waits under the rough painting.
                    let mut missing = Vec::new();
                    let mut drawn = pieces(frame, layer, wanted, width, height, dpr, 0)
                        .into_iter()
                        .flat_map(|piece| match painter::ready(piece.key) {
                            Some(painter::Ready::Image(image, _)) => {
                                vec![Drawn::Image(image, piece.rect, piece.clip)]
                            }
                            Some(painter::Ready::Empty) => Vec::new(),
                            None => instead(piece.clip, &mut missing),
                        })
                        .collect::<Vec<_>>();
                    drawn.extend(fill_all(missing, &mut gaps));
                    drawn
                }
            };
            // A new look is never faded in over the old: with pictures that
            // are partly transparent, no fade of one over the other is
            // exact (the old one whole under the new doubles its shadows;
            // each half-faded lets the ground through its walls). The old
            // look stays whole until every picture of the new one the
            // camera sees is with the display, then the new replaces it in
            // one frame, as a drawing of the boil does. (Where the old lacks
            // a piece, the new one's is shown at once, above.)
            let whole_new = arriving.is_some_and(|(version, _)| {
                coming.len() + coming_empty.len()
                    == pieces(frame, layer, version, width, height, dpr, 0).len()
            });
            let settled = if whole_new {
                gaps = 0;
                coming
                    .iter()
                    .map(|(_, image, rect, clip)| Drawn::Image(image.clone(), *rect, *clip))
                    .collect()
            } else {
                // Until then what of it is with the display is handed over
                // out of sight, so the swap costs a frame no more than any.
                ahead.extend(
                    coming
                        .iter()
                        .map(|(_, image, rect, clip)| Drawn::Image(image.clone(), *rect, *clip)),
                );
                settled
            };
            missing += gaps;
            let (arriving, fade): (Vec<Drawn>, f32) = (Vec::new(), 0.0);
            // Handed over ahead of being seen, a frame's share at a time:
            // the next drawing of the boil, and the tiles a pan reaches
            // next, so neither costs a frame all at once.
            if let Some(version) = early {
                ahead.extend(draw(version, false, &mut 0));
            }
            if let Some(version) = shown.filter(|_| !now && painter::to_spare()) {
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
            let arrived = shown.is_some() || !arriving.is_empty() || has_rough;
            let waiting = !now
                && (shown != Some(wanted) || early.is_some() || !rough_whole || heading_waiting);
            rough_seen.0 += rough_area.get().0;
            rough_seen.1 += rough_area.get().1;
            // A building the version on show lacks is drawn live meanwhile.
            let live = match (layer, shown) {
                (Still::Buildings, Some(version)) if version.key != wanted.key => {
                    STANDING.with(|standing_in| {
                        let mut standing_in = standing_in.borrow_mut();
                        if standing_in.len() > 64 {
                            standing_in.retain(|key, _| *key == version.key);
                        }
                        standing_in
                            .entry(wanted.key)
                            .or_insert_with(|| standing(frame));
                        let Some(old) = standing_in.get(&version.key) else {
                            return Vec::new();
                        };
                        frame
                            .buildings
                            .iter()
                            .filter(|building| !building.moving())
                            .filter(|building| {
                                !old.contains(&(building.x.round() as i32, building.shape as u8))
                            })
                            .map(|building| building.index)
                            .collect()
                    })
                }
                (Still::Buildings, _) => {
                    STANDING.with(|standing_in| {
                        standing_in
                            .borrow_mut()
                            .entry(wanted.key)
                            .or_insert_with(|| standing(frame));
                    });
                    Vec::new()
                }
                _ => Vec::new(),
            };
            #[cfg(test)]
            note_overlap(&rough, &settled, &arriving);
            (
                layer,
                LayerPlan {
                    live,
                    settled,
                    arriving,
                    fade: ease(fade),
                    ahead,
                    shown: arrived,
                    waiting,
                },
            )
        })
        .collect();
    if !now {
        UNPAINTED.with(|counts| {
            let mut counts = counts.borrow_mut();
            let count = counts.entry(id).or_insert(0);
            // The first plan of a frame (its back layers) starts the count.
            if slot == 0 {
                *count = 0;
            }
            *count += missing;
        });
        ROUGH_SEEN.with(|seen| {
            let mut seen = seen.borrow_mut();
            let entry = seen.entry(id).or_insert((0.0, 0.0));
            if slot == 0 {
                *entry = (0.0, 0.0);
            }
            entry.0 += rough_seen.0;
            entry.1 += rough_seen.1;
        });
    }
    plans
}

/// A piece of where the camera is going, as the camera sees it now: where
/// it shows, and its picture with where that is drawn (none if it came out
/// empty).
type HeadingTile = (Rect, Option<(std::sync::Arc<gpui::RenderImage>, Rect)>);

/// What of where the camera is going is handed to the display this frame,
/// per layer, and which layers still wait for some of it.
#[derive(Default)]
struct HeadingAhead {
    ahead: std::collections::HashMap<Still, Vec<Drawn>>,
    waiting: std::collections::HashSet<Still>,
}

/// Where the camera is going (`there`, with the ground's key and the
/// buildings' scale there): what it will see of the land and the buildings
/// is asked for ahead of everything but the rough painting, the moment's
/// subject first, and handed to the display ahead of being seen, before
/// whatever the camera passes on its way (seen for a moment, as it moves):
/// this frame's share of new pictures goes first to where it will settle.
fn ask_ahead(
    window: &Window,
    there: &std::sync::Arc<Frame>,
    (key, scale): (u64, f32),
    dpr: f32,
    only: &[Still],
    soon: bool,
) -> HeadingAhead {
    let mut out = HeadingAhead::default();
    let (cx, cy) = there
        .subject()
        .map_or((there.camera.x, there.camera.y), |s| {
            (s.0 + s.2 / 2.0, s.1 + s.3 / 2.0)
        });
    // The buildings first: the subject is what stands there.
    for layer in [Still::Buildings, Still::Land] {
        let Some(tile_layer) = ground_layer(layer).filter(|_| only.contains(&layer)) else {
            continue;
        };
        let version = Version {
            key,
            scale: if layer == Still::Land {
                scale * 0.5
            } else {
                scale
            },
            boil: 0,
        };
        let mut tiles = tiles_in_view(there, tile_layer, version.scale, 0);
        let side = TILE as f32 / version.scale;
        let far = |(column, row): &(i32, i32)| {
            let (tx, ty) = (
                (*column as f32 + 0.5) * side - cx,
                (*row as f32 + 0.5) * side - cy,
            );
            (tx * tx + ty * ty) as i64
        };
        tiles.sort_by_key(far);
        let mut ahead = Vec::new();
        for piece in tile_pieces(there, tile_layer, version, dpr, tiles) {
            if !painter::painted(piece.key) {
                painter::ask(window, piece.key, false, soon, piece.job);
                out.waiting.insert(layer);
            } else if !painter::handed(piece.key) && (soon || painter::to_spare()) {
                match painter::ready(piece.key) {
                    Some(painter::Ready::Image(image, _)) => {
                        ahead.push(Drawn::Image(image, piece.rect, piece.clip))
                    }
                    _ => {
                        out.waiting.insert(layer);
                    }
                }
            }
        }
        out.ahead.insert(layer, ahead);
    }
    out
}

/// The still layer of the land or the buildings, as tiles.
fn ground_layer(layer: Still) -> Option<Layer> {
    match layer {
        Still::Land => Some(Layer::Land),
        Still::Buildings => Some(Layer::Buildings),
        _ => None,
    }
}

/// What the camera will see from `to`, with the moment's `subject` there:
/// the frame seen from there, the key of the ground there and the scale the
/// buildings are painted at there (the land at half of it). Not while the
/// place folds into a postcard.
fn seen_from(
    frame: &Frame,
    to: Camera,
    subject: Option<(f32, f32, f32, f32)>,
    dpr: f32,
) -> Option<(std::sync::Arc<Frame>, u64, f32)> {
    if to.fold > 0.0 || frame.camera.fold > 0.0 {
        return None;
    }
    let mut there = frame.seen_from(to);
    there.subject = subject;
    let zoom = ((to.zoom * 100.0).round() / 100.0).max(0.05);
    let scale = zoom * dpr;
    let key = ground_key(&there, scale).finish();
    Some((std::sync::Arc::new(there), key, scale))
}

/// The moment's subject on screen: its box, or with none the middle of
/// the view, where the eye goes after the camera moves.
fn subject_rect(frame: &Frame) -> Rect {
    match frame.subject() {
        Some((x, y, w, h)) => {
            let (x0, y0) = frame.at(x, y);
            let (x1, y1) = frame.at(x + w, y + h);
            (x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs())
        }
        None => (
            frame.view_w * 0.25,
            frame.height * 0.25,
            frame.view_w * 0.5,
            frame.height * 0.5,
        ),
    }
}

/// Where two rectangles overlap (empty when they do not).
fn overlap(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.0.max(b.0), a.1.max(b.1));
    let (x1, y1) = ((a.0 + a.2).min(b.0 + b.2), (a.1 + a.3).min(b.1 + b.3));
    (x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0))
}

fn area(rect: Rect) -> f32 {
    rect.2.max(0.0) * rect.3.max(0.0)
}

/// Whether `outer` holds all of `inner`.
fn contains(outer: Rect, inner: Rect) -> bool {
    inner.0 >= outer.0 - 0.01
        && inner.1 >= outer.1 - 0.01
        && inner.0 + inner.2 <= outer.0 + outer.2 + 0.01
        && inner.1 + inner.3 <= outer.1 + outer.3 + 0.01
}

thread_local! {
    /// Where each still layer of the scene stands, per window and size.
    static SLOTS: std::cell::RefCell<Slots> = std::cell::RefCell::new(Slots::new());
    /// The last rough painting of each layer that was whole and with the
    /// display, per window, layer and size, and which World it is of: the
    /// stand-in while a new look's rough painting is painted.
    static ROUGHS: std::cell::RefCell<std::collections::HashMap<SlotKey, (Version, u64)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// How much of the rough painting the frame last drawn in each window
    /// showed, in square pixels: anywhere, and on the moment's subject.
    static ROUGH_SEEN: std::cell::RefCell<std::collections::HashMap<u64, (f32, f32)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// Whether the frame each window drew before this one showed no rough
    /// painting anywhere.
    static SHARP_BEFORE: std::cell::RefCell<std::collections::HashMap<u64, bool>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    /// When each window last drew its scene.
    static DRAWN_AT: std::cell::RefCell<std::collections::HashMap<u64, std::time::Instant>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// How much of the rough painting the frame last drawn in `window` showed,
/// in square pixels: anywhere in view, and on the moment's subject. The
/// real-window harness holds both to a bar after every camera move: the
/// player sees what is on the subject, not whether something somewhere is
/// a flat stand-in.
pub fn rough_shown(window: &Window) -> (f32, f32) {
    ROUGH_SEEN.with(|seen| {
        seen.borrow()
            .get(&window_id(window))
            .copied()
            .unwrap_or((0.0, 0.0))
    })
}

/// A window not drawn for this long is taken to be closed: what was kept
/// for it is let go.
const CLOSED: std::time::Duration = std::time::Duration::from_secs(60);

/// Notes that `id` is drawing, and lets go of everything kept per window
/// for windows that have not drawn for a while (closed): the layers'
/// slots, the rough paintings, the counts the frame log reads.
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
    SLOTS.with(|slots| slots.borrow_mut().retain(|key, _| !gone.contains(&key.0)));
    ROUGHS.with(|roughs| roughs.borrow_mut().retain(|key, _| !gone.contains(&key.0)));
    ROUGH_SEEN.with(|seen| seen.borrow_mut().retain(|window, _| !gone.contains(window)));
    SHARP_BEFORE.with(|before| {
        before
            .borrow_mut()
            .retain(|window, _| !gone.contains(window))
    });
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
    ZOOMS.with(|zooms| zooms.borrow_mut().retain(|view, _| !gone.contains(&view.0)));
    painter::forget_windows(&gone);
}

/// A window's bookkeeping is let go a while after it last drew (it has
/// closed), and an open one's is kept: the per-window maps never grow with
/// every window ever opened.
#[test]
fn a_closed_windows_bookkeeping_is_let_go() {
    let kept = || {
        let mut windows = std::collections::BTreeSet::new();
        SLOTS.with(|slots| windows.extend(slots.borrow().keys().map(|key| key.0)));
        ROUGHS.with(|roughs| windows.extend(roughs.borrow().keys().map(|key| key.0)));
        UNPAINTED.with(|counts| windows.extend(counts.borrow().keys().copied()));
        ROUGH_SEEN.with(|seen| windows.extend(seen.borrow().keys().copied()));
        SETTLED.with(|settled| windows.extend(settled.borrow().keys().copied()));
        ZOOMS.with(|zooms| windows.extend(zooms.borrow().keys().map(|view| view.0)));
        windows
    };
    let version = Version {
        key: 1,
        scale: 1.0,
        boil: 0,
    };
    for window in [901_u64, 902] {
        SLOTS.with(|slots| {
            slots
                .borrow_mut()
                .insert((window, Still::Land, 1, 100, 100), (Some(version), None))
        });
        ROUGHS.with(|roughs| {
            roughs
                .borrow_mut()
                .insert((window, Still::Land, 1, 100, 100), (version, 7))
        });
        UNPAINTED.with(|counts| counts.borrow_mut().insert(window, 0));
        ROUGH_SEEN.with(|seen| seen.borrow_mut().insert(window, (0.0, 0.0)));
        SETTLED.with(|settled| settled.borrow_mut().insert(window, true));
        ZOOMS.with(|zooms| zooms.borrow_mut().insert((window, 100, 100), (1.0, 1.0)));
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

thread_local! {
    /// The rough painting turned off (a test paints a scene without it,
    /// to hold the settled scene to being the same with and without it).
    static ROUGH_OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

thread_local! {
    /// The most of a frame's rough painting drawn where a sharp picture of
    /// the same layer is drawn too (under it or over it), in square pixels.
    #[cfg(test)]
    static OVERLAP: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

/// The most rough painting drawn where a sharp picture is too, since the
/// last call, in square pixels: none, ever.
#[cfg(test)]
pub fn rough_overlap() -> f32 {
    OVERLAP.with(|cell| cell.replace(0.0))
}

#[cfg(test)]
fn note_overlap(rough: &Rough, settled: &[Drawn], arriving: &[Drawn]) {
    let rough_images = rough
        .iter()
        .map(|(image, _, _)| std::sync::Arc::as_ptr(image))
        .collect::<std::collections::HashSet<_>>();
    let (mut under, mut sharp) = (Vec::new(), Vec::new());
    for drawn in settled.iter().chain(arriving) {
        if let Drawn::Image(image, _, clip) = drawn {
            if rough_images.contains(&std::sync::Arc::as_ptr(image)) {
                under.push(*clip);
            } else {
                sharp.push(*clip);
            }
        }
    }
    let area = under
        .iter()
        .flat_map(|a| sharp.iter().map(move |b| (a, b)))
        .map(|(a, b)| {
            let w = (a.0 + a.2).min(b.0 + b.2) - a.0.max(b.0);
            let h = (a.1 + a.3).min(b.1 + b.3) - a.1.max(b.1);
            if w > 0.5 && h > 0.5 {
                w * h
            } else {
                0.0
            }
        })
        .sum::<f32>();
    OVERLAP.with(|cell| cell.set(cell.get().max(area)));
}

/// What of `clip` none of `by` reaches, as rectangles.
fn uncovered(clip: Rect, by: &[Rect]) -> Vec<Rect> {
    let meets = |b: &Rect| {
        b.0 < clip.0 + clip.2 && b.0 + b.2 > clip.0 && b.1 < clip.1 + clip.3 && b.1 + b.3 > clip.1
    };
    let hit = by.iter().filter(|b| meets(b)).copied().collect::<Vec<_>>();
    if hit.is_empty() {
        return vec![clip];
    }
    let cuts = |from: f32, to: f32, edges: &mut dyn Iterator<Item = f32>| {
        let mut all = vec![from, to];
        all.extend(edges.filter(|e| *e > from + 0.25 && *e < to - 0.25));
        all.sort_by(f32::total_cmp);
        all.dedup_by(|a, b| (*a - *b).abs() < 0.25);
        all
    };
    let xs = cuts(
        clip.0,
        clip.0 + clip.2,
        &mut hit.iter().flat_map(|b| [b.0, b.0 + b.2]),
    );
    let ys = cuts(
        clip.1,
        clip.1 + clip.3,
        &mut hit.iter().flat_map(|b| [b.1, b.1 + b.3]),
    );
    let mut out = Vec::new();
    for row in ys.windows(2) {
        let mut run: Option<Rect> = None;
        for col in xs.windows(2) {
            let (cx, cy) = ((col[0] + col[1]) / 2.0, (row[0] + row[1]) / 2.0);
            let reached = hit
                .iter()
                .any(|b| cx >= b.0 && cx <= b.0 + b.2 && cy >= b.1 && cy <= b.1 + b.3);
            if reached {
                out.extend(run.take());
            } else {
                match run.as_mut() {
                    Some(r) => r.2 = col[1] - r.0,
                    None => run = Some((col[0], row[0], col[1] - col[0], row[1] - row[0])),
                }
            }
        }
        out.extend(run);
    }
    out
}

/// Turns the rough painting off, or back on, on this thread.
#[cfg(test)]
pub fn set_rough_off(off: bool) {
    ROUGH_OFF.with(|cell| cell.set(off));
}

fn rough_off() -> bool {
    ROUGH_OFF.with(|cell| cell.get())
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

/// Hands pictures to the display ahead of being seen: each drawn at the
/// scene's corner, a pixel of it, under a mask that shows nothing. Where a
/// picture will be shown does not matter (the display keeps it by the
/// picture, not by where it was drawn), but a picture drawn wholly outside
/// the scene is never handed over at all: the tiles a pan reaches next lie
/// just outside it, and were copied in only on the frame they first showed
/// (v0.28's slow frames on a pan).
pub(super) fn draw_ahead(window: &mut Window, bounds: Bounds<Pixels>, drawn: &[Drawn]) {
    let corner = Bounds::new(bounds.origin, size(px(1.0), px(1.0)));
    for item in drawn {
        if let Drawn::Image(image, rect, _) = item {
            let at = Bounds::new(bounds.origin, size_of(rect.2.max(1.0), rect.3.max(1.0)));
            let _ = painter::timed("main: image", || {
                window.paint_image(corner, at, Corners::default(), image.clone(), 0, false)
            });
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
    fetch_pictures(&mut frame, window, true);
    let light = frame.light();
    let view = (width, height);
    // A folding place is painted a row at a time, back to front, each over
    // the haze laid on the one behind; otherwise all at once.
    let rows = rows_of(&frame).unwrap_or_else(|| vec![(frame.clone(), (0.0, width))]);
    let folded = rows.len() > 1;
    let back = std::sync::Arc::new(rows[0].0.clone());
    for (layer, plan) in plan(&back, window, view, true, (0, &ALL[..2]), &back) {
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
            for (_, plan) in plan(
                &part,
                window,
                view,
                true,
                (1 + row as u32, &ALL[2..]),
                &part,
            ) {
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
    // While opening, everything the first frame needs is handed over at
    // once: nothing is on screen yet to hitch.
    let opening = !painted(window);
    painter::opening(opening);
    painter::begin_frame(!sync && !opening);
    let back = std::sync::Arc::new(rows[0].0.clone());
    let mut planned = vec![(
        back.clone(),
        None,
        painter::timed("main: plan", || {
            plan(&back, window, (width, height), sync, (0, &ALL[..2]), &back)
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
                if folded { &frame } else { &part },
            )
        });
        // What was just built and is not painted yet is drawn live.
        let live = layers
            .iter()
            .flat_map(|(_, plan)| plan.live.iter().copied())
            .collect::<BTreeSet<_>>();
        let part = if live.is_empty() {
            part
        } else {
            let mut drawn = (*part).clone();
            for building in &mut drawn.buildings {
                if live.contains(&building.index) {
                    building.grow = building.grow.min(0.9999);
                }
            }
            std::sync::Arc::new(drawn)
        };
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
    let waiting = planned.iter().any(|(_, _, layers)| {
        layers
            .iter()
            .any(|(_, plan)| !plan.arriving.is_empty() || plan.waiting)
    });
    if !sync {
        SETTLED.with(|settled| {
            settled.borrow_mut().insert(window_id(window), !waiting);
        });
    }
    if !sync {
        log_frame(window, wash, &frame);
    }
    if !painter::idle() || waiting {
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
                    painter::timed("main: draw still", || draw_ahead(window, bounds, &drawn))
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
                    let light = frame.light();
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
fn log_frame(window: &Window, wash: f32, frame: &Frame) {
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
    let (rough_any, rough_subject) = rough_shown(window);
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
fn wash_over(window: &Window, all_shown: bool, still: bool) -> f32 {
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

/// A window and a view's size in it.
pub(super) type ViewId = (u64, u32, u32);

thread_local! {
    /// The zoom each view's ground was last painted at.
    static ZOOMS: std::cell::RefCell<std::collections::HashMap<ViewId, (f32, f32)>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// The zoom the ground is painted at: the camera's own when it is still,
/// and while it moves the last one painted (scaled), so a glide does not
/// paint every frame.
pub(super) fn painted_zoom(window: &Window, frame: &Frame) -> f32 {
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
