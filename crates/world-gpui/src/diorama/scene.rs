//! Where everything stands: the stage a World is laid out on, its rows and
//! its composition, the camera onto it, and the land and the hills behind.

use super::*;

/// The colours of a World that does not say what it looks like: a mild
/// green valley under a pale sky.
pub(super) const PLAIN: Scenery = Scenery {
    sky_top: 0xb9d6ea,
    sky_bottom: 0xeef3ea,
    far: 0x9db88f,
    near: 0x6f9a6a,
    sun: 0xffe7a8,
};

/// Where the parts of the landscape sit, as fractions of the stage height:
/// the horizon; the back row, where the town begins; the quay, where
/// people walk and sit; and the top of the foreground (the water in a
/// harbour, dark dust on Mars), which the choice card floats over.
pub(super) const HORIZON: f32 = 0.40;
pub(super) const BASE: f32 = 0.47;
pub(super) const FEET: f32 = 0.745;
pub(super) const FRONT: f32 = 0.80;
pub(super) const MARGIN: f32 = 0.07;
/// From this row of a Pack's (`CanvasItem::y`) on, a thing stands on the
/// water line: a pier, a slipway, the lighthouse on the point.
pub const WATER_ROW: f32 = 0.8;
/// Where what stands on the water line has its foot, as a fraction of the
/// stage's height: a little below the quay's edge, in the water.
pub(super) const WATER_LINE: f32 = FRONT + (1.0 - FRONT) * 0.12;

/// How much taller a cottage stands than a grown-up at the same depth.
pub const COTTAGE: f32 = 2.75;

/// The Pack's rows as its `y` gives them (the back row 0.3; the street's
/// two rows, 0.45 and 0.6, with the plots at 0.68; the quay, 0.76), as
/// lines on the stage (fractions of its height), and how big whatever
/// stands on each is drawn: nearer is bigger, and the quay is full size.
/// Between the back row and the quay each row stands about a storey
/// nearer than the one behind, so the town climbs the stage in depth
/// rather than standing on one line. The sizes are the art bible's
/// distance factors: the second band three quarters of the front, the
/// third a little over half (the back row, two fifths, is the far ridge's).
pub(super) const ROWS: [(f32, f32, f32); 5] = [
    (0.30, BASE, 0.55),
    (0.45, 0.548, 0.64),
    (0.60, 0.618, 0.75),
    (0.68, 0.658, 0.8),
    (0.76, FEET, 1.0),
];

/// The rows of a young place, whose street is still empty: the back row
/// stands forward, nearer the quay and bigger, so the town does not hang
/// at the top of a bare field. As the street fills the rows move back to
/// [`ROWS`].
pub(super) const YOUNG_ROWS: [(f32, f32, f32); 5] = [
    (0.30, 0.56, 0.62),
    (0.45, 0.60, 0.68),
    (0.60, 0.635, 0.76),
    (0.68, 0.662, 0.82),
    (0.76, FEET, 1.0),
];

/// The line (a fraction of the stage height) and the scale of whatever
/// stands in the Pack's row `y`, between two rows as far between them,
/// for a grown place.
pub(super) fn row_at(y: f32) -> (f32, f32) {
    row_aged(y, 1.0)
}

/// The same for a place whose street is `grown` of the way full (0 to 1):
/// the young rows and the grown ones mixed.
pub(super) fn row_aged(y: f32, grown: f32) -> (f32, f32) {
    let on = |rows: &[(f32, f32, f32); 5]| {
        let y = y.clamp(rows[0].0, rows[rows.len() - 1].0);
        for pair in rows.windows(2) {
            let ((y0, l0, s0), (y1, l1, s1)) = (pair[0], pair[1]);
            if y <= y1 {
                let t = (y - y0) / (y1 - y0);
                return (l0 + (l1 - l0) * t, s0 + (s1 - s0) * t);
            }
        }
        let (_, line, scale) = rows[rows.len() - 1];
        (line, scale)
    };
    let (grown_line, grown_scale) = on(&ROWS);
    let (young_line, young_scale) = on(&YOUNG_ROWS);
    let g = grown.clamp(0.0, 1.0);
    (
        young_line + (grown_line - young_line) * g,
        young_scale + (grown_scale - young_scale) * g,
    )
}

/// The four depth bands a place is composed in, back to front: the back
/// row of buildings, the street, the quay (or the near ground) where
/// people walk and sit, and the water.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Depth {
    Back,
    Street,
    Quay,
    Water,
}

impl Depth {
    /// The band of a line `y` stage pixels down a stage `height` tall.
    pub fn at(y: f32, height: f32) -> Self {
        let line = y / height.max(1.0);
        if line >= FRONT - 1e-3 {
            Depth::Water
        } else if line >= 0.70 {
            Depth::Quay
        } else if line >= 0.51 {
            Depth::Street
        } else {
            Depth::Back
        }
    }
}

/// One thing's place on the stage, in stage pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// Index into the snapshot's canvas items.
    pub index: usize,
    pub x: f32,
    pub y: f32,
    /// How wide it stands (its footprint), in stage pixels; for a person,
    /// the room they take.
    pub w: f32,
    /// How big it is drawn for how near it stands: 1 on the quay, less
    /// further back.
    pub scale: f32,
}

/// Where everything stands, for a stage `width` by `height` pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    /// How wide the whole place is: the window's width, or for a panorama
    /// that many window widths.
    pub width: f32,
    pub height: f32,
    /// How wide the window onto it is.
    pub view_w: f32,
    pub horizon: f32,
    /// The back row's line.
    pub base: f32,
    /// The street's front row, where the nearest buildings stand.
    pub street: f32,
    /// The quay's line, where people stand.
    pub feet: f32,
    /// The water's edge.
    pub front: f32,
    /// A cottage's footprint and height, and a grown-up's height, on the
    /// quay; further back each is smaller by its spot's `scale`.
    pub building_w: f32,
    pub building_h: f32,
    pub figure_h: f32,
    pub thing_w: f32,
    /// Places, drawn as buildings with their base at `y`.
    pub buildings: Vec<Spot>,
    /// Things, drawn with their base at `y`.
    pub things: Vec<Spot>,
    /// People, standing with their feet at `y`.
    pub people: Vec<Spot>,
    /// People indoors this hour, and where: seen, if at all, as a shape in
    /// a lit window. `(person, place)`, as item indices.
    pub inside: Vec<(usize, usize)>,
    /// People on their way to where the hour's routine puts them: where
    /// they set off from, and how many seconds ago.
    pub routes: BTreeMap<usize, (f32, f32)>,
    /// Plots staked out where something could be built, by their index in
    /// the World's plots: their front edge's middle, and how wide.
    pub plots: Vec<PlotPaint>,
    /// What the composition keeps back until the camera is close enough
    /// for it to stand clear of its neighbours: an item's index, and the
    /// least zoom it shows at (infinite: there is no room for it at all).
    pub shown_from: BTreeMap<usize, f32>,
    /// How full the street is, 0 to 1: a young place's rows stand forward.
    pub grown: f32,
}

impl Stage {
    pub fn person(&self, id: SelectionId, snapshot: &ProjectionSnapshot) -> Option<Spot> {
        self.people
            .iter()
            .copied()
            .find(|spot| snapshot.canvas.items.get(spot.index).map(|item| item.id) == Some(id))
    }

    /// Whether the item at `index` is shown with the camera `zoom` times
    /// closer: everything is, but what the composition keeps back for a
    /// closer look.
    pub fn shows(&self, index: usize, zoom: f32) -> bool {
        self.shown_from
            .get(&index)
            .is_none_or(|least| zoom + 1e-4 >= *least)
    }

    /// How tall a building on `spot` of `shape` stands.
    pub fn height_of(&self, spot: &Spot, shape: MarkShape) -> f32 {
        self.building_h
            * spot.scale
            * match shape {
                MarkShape::Bridge => 0.7,
                MarkShape::Tower => 1.2,
                MarkShape::Dome | MarkShape::Tree => 0.8,
                _ => 1.0,
            }
    }

    /// Where anything on stage is, and roughly how big it is, as a box
    /// around it: what a camera frames when it goes to look at it.
    pub fn frame_of(&self, index: usize) -> Option<(f32, f32, f32, f32)> {
        if let Some(spot) = self.buildings.iter().find(|spot| spot.index == index) {
            let h = self.building_h * spot.scale;
            return Some((spot.x - spot.w / 2.0, spot.y - h, spot.w, h));
        }
        if let Some(spot) = self.things.iter().find(|spot| spot.index == index) {
            return Some((
                spot.x - spot.w / 2.0,
                spot.y - spot.w * 0.6,
                spot.w,
                spot.w * 0.6,
            ));
        }
        self.people
            .iter()
            .find(|spot| spot.index == index)
            .map(|spot| {
                let h = self.figure_h * spot.scale;
                (spot.x - h * 0.3, spot.y - h, h * 0.6, h)
            })
    }

    /// How many window widths the place is.
    pub fn panorama(&self) -> f32 {
        (self.width / self.view_w.max(1.0)).max(1.0)
    }

    /// The depth band the spot stands in.
    pub fn depth_of(&self, spot: &Spot) -> Depth {
        Depth::at(spot.y, self.height)
    }

    /// Whether the item at `index` stands on the water line (in the
    /// water band), as a pier or a boat does.
    pub fn on_water(&self, index: usize) -> bool {
        self.buildings
            .iter()
            .chain(&self.things)
            .any(|spot| spot.index == index && self.depth_of(spot) == Depth::Water)
    }

    /// What the first screen shows of the place: everything built or put
    /// down that stands in the window with the camera at rest, as "label
    /// (art) at x", for telling one day's first screen from the next.
    pub fn first_screen(&self, snapshot: &ProjectionSnapshot) -> BTreeSet<String> {
        self.screen_around(snapshot, Camera::whole(self).x)
    }

    /// The same for the window centred on stage `x` (where the camera
    /// opens on whoever welcomes the player, say).
    pub fn screen_around(&self, snapshot: &ProjectionSnapshot, x: f32) -> BTreeSet<String> {
        let x = keep_on(x, self.view_w / 2.0, self.width);
        let (from, to) = (x - self.view_w / 2.0, x + self.view_w / 2.0);
        self.buildings
            .iter()
            .chain(&self.things)
            .filter(|spot| spot.x + spot.w / 2.0 > from && spot.x - spot.w / 2.0 < to)
            .filter(|spot| self.shows(spot.index, 1.0))
            .filter_map(|spot| {
                let item = snapshot.canvas.items.get(spot.index)?;
                Some(format!(
                    "{} ({}) at {:.0}",
                    item.label,
                    item.art.as_deref().unwrap_or("-"),
                    spot.x / 10.0
                ))
            })
            .collect()
    }

    /// The line (in stage pixels) and the scale of whatever stands in the
    /// Pack's row `y`.
    pub fn row_line(&self, y: f32) -> (f32, f32) {
        if y >= WATER_ROW {
            return (self.height * WATER_LINE, 1.0);
        }
        let (line, scale) = row_aged(y, self.grown);
        (line * self.height, scale)
    }
}

/// Where along the ground a stage point `x` is, from 0 (the left edge of
/// the row places stand in) to 100 (its right edge): the spot something
/// the player puts down there takes.
pub fn ground_spot(stage: &Stage, x: f32) -> u8 {
    let usable = stage.width * (1.0 - 2.0 * MARGIN);
    (((x - stage.width * MARGIN) / usable.max(1.0)) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8
}

/// Pairs a Pack wants drawn together, who stand side by side.
pub(super) fn pairs(snapshot: &ProjectionSnapshot) -> Vec<(SelectionId, SelectionId)> {
    snapshot
        .canvas
        .links
        .iter()
        .map(|link| (link.from, link.to))
        .collect()
}

/// The local clock, as far as where people are goes: the hour, and how far
/// into it. UI time, never the World's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clock {
    pub hour: u8,
    /// Seconds since the hour began.
    pub into_hour: f32,
}

impl Clock {
    /// Now, on this computer's clock; a pinned hour (`WORLD_MACHINE_HOUR`)
    /// is well into itself, so nobody is still on their way.
    pub fn now() -> Self {
        use chrono::Timelike;
        match crate::scene::pinned_hour() {
            Some(hour) => Self::at(hour as u8),
            None => {
                let now = chrono::Local::now();
                Self {
                    hour: now.hour() as u8,
                    into_hour: (now.minute() * 60 + now.second()) as f32
                        + now.nanosecond().min(999_999_999) as f32 / 1e9,
                }
            }
        }
    }

    /// Well into `hour`.
    pub fn at(hour: u8) -> Self {
        Self {
            hour: hour % 24,
            into_hour: 1800.0,
        }
    }
}

/// How many stand out by the water at dusk at least, when the hour has the
/// rest indoors: two groups, of three and two.
pub(super) const DUSK_OUT: usize = 5;

/// How fast someone strolls to where their day takes them, in figure
/// heights a second.
pub(super) const STROLL: f32 = 1.1;

/// Lays out a World on a stage `width` by `height` pixels, at the hour on
/// the local clock.
pub fn stage(snapshot: &ProjectionSnapshot, width: f32, height: f32) -> Stage {
    stage_at(snapshot, width, height, Clock::now())
}

/// Lays out a World on a window `width` by `height` pixels at `clock`.
///
/// The place is composed in four depth bands: the back row of buildings,
/// the street (its rows climbing toward the viewer, each a little bigger),
/// the quay in the lower third where people walk and sit, and the water.
/// Places and things stand where their `px` puts them along a panorama, in
/// the band their Pack's row (`y`) gives, or else in a row along one
/// window in the order the Pack placed them; a thing kept at a place with
/// no spot of its own stands on the quay beside it. People stand on the
/// quay in front of wherever they are (or wherever their day has them this
/// hour), pairs side by side; anyone who is nowhere in particular stands
/// where the Pack put them. Then [`compose`] sees that nothing stands on
/// anything else. The same World at the same hour always lays out the
/// same.
pub fn stage_at(snapshot: &ProjectionSnapshot, width: f32, height: f32, clock: Clock) -> Stage {
    let items = &snapshot.canvas.items;
    let view_w = width.max(120.0);
    let height = height.max(90.0);
    let panorama = snapshot.canvas.width.unwrap_or(1.0).clamp(1.0, 24.0);
    let width = view_w * panorama;
    // A window's stage keeps its people big enough to see; a cover's
    // shrinks everything with it. A cottage stands `COTTAGE` times a
    // grown-up at the same depth.
    let figure_h = (height * 0.068).min(64.0).max((height * 0.1).min(40.0));
    let building_h = figure_h * COTTAGE;
    let index_of = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.id, index))
        .collect::<BTreeMap<_, _>>();
    // Where each person's day has them this hour, and where it had them
    // before.
    let routine = |index: usize| {
        let day = &items[index].day;
        let now = world_projection::stop_at(day, clock.hour)?;
        let position = day.iter().position(|stop| stop == now)?;
        let before = day[(position + day.len() - 1) % day.len()];
        Some((*now, before))
    };
    let placed = |id: SelectionId| {
        index_of
            .get(&id)
            .copied()
            .filter(|host| items[*host].kind != CanvasItemKind::Actor)
    };
    // Where each item is: its host, when the host is on stage and is not
    // itself somewhere else.
    let host = |index: usize| -> Option<usize> {
        let at = if items[index].kind == CanvasItemKind::Actor && !items[index].day.is_empty() {
            routine(index)
                .and_then(|(now, _)| placed(now.at).map(|_| now.at))
                .or(items[index].at)
        } else {
            items[index].at
        }?;
        let host = *index_of.get(&at)?;
        (host != index && items[host].at.is_none()).then_some(host)
    };
    let along = |item: &CanvasItem| item.px.map(|px| px.clamp(0.0, panorama) * view_w);
    // What stands by itself: a place, or a thing with nobody's place or
    // with a point of its own along the panorama. What the player stood
    // somewhere of their choosing keeps its spot; a thing kept at a place
    // with no point of its own stands beside it.
    let free = |index: usize| {
        let item = &items[index];
        item.kind != CanvasItemKind::Actor
            && item.spot.is_none()
            && (host(index).is_none() || item.px.is_some())
    };
    let mut anchors = (0..items.len())
        .filter(|index| free(*index) && items[*index].px.is_none())
        .collect::<Vec<_>>();
    anchors.sort_by(|a, b| {
        items[*a]
            .x
            .total_cmp(&items[*b].x)
            .then(items[*a].y.total_cmp(&items[*b].y))
            .then(a.cmp(b))
    });
    let usable = width * (1.0 - 2.0 * MARGIN);
    let slot_w = usable / anchors.len().max(1) as f32;
    // A row of places along one window stands no wider than its slots.
    let building_w = (building_h * 1.15)
        .min(if anchors.len() > 1 {
            slot_w * 0.9
        } else {
            f32::MAX
        })
        .max(building_h * 0.5);
    let thing_w = (building_w * 0.55).max(figure_h * 0.8);
    let mut slot_x = anchors
        .iter()
        .enumerate()
        .map(|(slot, index)| (*index, width * MARGIN + slot_w * (slot as f32 + 0.5)))
        .collect::<BTreeMap<_, _>>();
    for (index, item) in items.iter().enumerate() {
        if free(index) {
            if let Some(x) = along(item) {
                slot_x.insert(index, x);
            }
        }
    }

    // How full the street is: a young place's rows stand forward while it
    // is empty, and move back as it fills (about five works a window).
    let street_count = slot_x
        .keys()
        .filter(|index| {
            let item = &items[**index];
            item.px.is_some() && (0.40..0.72).contains(&item.y)
        })
        .count();
    let grown = (street_count as f32 / (panorama * 5.0)).min(1.0);
    let row_at = |y: f32| row_aged(y, grown);
    // Which of the Pack's rows each thing stands in, and so its line and
    // how big it is. A place never stands on the quay; a boat is on the
    // water.
    let boat = |index: usize| items[index].shape == Some(MarkShape::Boat);
    let setting = art::Setting::from_key(snapshot.canvas.setting.as_deref());
    let row_of = |index: usize| -> f32 {
        let item = &items[index];
        match (item.kind, item.px) {
            // On the water line, whatever it is.
            (_, Some(_)) if item.y >= WATER_ROW => item.y,
            // On the ice a bridge may stand forward, on the lead of open
            // water behind the causeway (see `paint_lead`).
            (CanvasItemKind::Place, Some(_))
                if setting == art::Setting::Ice && item.shape == Some(MarkShape::Bridge) =>
            {
                item.y.min(0.74)
            }
            (CanvasItemKind::Place, Some(_)) => item.y.min(0.68),
            // Places along one window stand in the street.
            (CanvasItemKind::Place, None) => 0.60,
            (_, Some(_)) => item.y,
            _ => 0.76,
        }
    };
    // Boats ride at their moorings out beyond the water line's piers.
    let water_line = height * (FRONT + (1.0 - FRONT) * 0.4);
    // The room each takes: on a panorama, its footprint on the scale
    // ladder (crate::ladder) at its depth; along one window, its share.
    let nominal = |index: usize, scale: f32| {
        let item = &items[index];
        let laddered = item
            .px
            .filter(|_| item.shape != Some(MarkShape::Boat))
            .and_then(|_| {
                crate::ladder::footprint(
                    item,
                    snapshot.drawing_of(item),
                    setting,
                    figure_h * scale,
                    item.kind == CanvasItemKind::Place,
                )
            });
        laddered.unwrap_or(match (item.kind, item.shape) {
            (CanvasItemKind::Place, _) => building_w * scale,
            (_, Some(MarkShape::Parcel)) => thing_w * 0.4 * scale,
            (_, Some(MarkShape::Boat)) => thing_w,
            _ => thing_w * scale,
        })
    };
    let spot_in_row = |index: usize, x: f32, row: f32| -> Spot {
        let item = &items[index];
        if row >= WATER_ROW && !boat(index) {
            // At the water's edge: a pier's foot a little out in the
            // water, a building's on the quay's edge, its slipway or jetty
            // running down in front of it.
            // A lighthouse stands out on its spit of rock. A work going up
            // stands in its scaffolding at the water's edge too, its site on
            // the land, never out over the water (v0.29 round 3).
            let place = (item.kind == CanvasItemKind::Place
                && item.shape != Some(MarkShape::Tower))
                || item.art.as_deref() == Some("scaffold");
            return Spot {
                index,
                x,
                y: if place {
                    height * FRONT + 1.0
                } else {
                    height * WATER_LINE
                },
                w: nominal(index, 1.0),
                scale: 1.0,
            };
        }
        if boat(index) {
            return Spot {
                index,
                x,
                y: water_line,
                w: nominal(index, 1.0),
                scale: 1.0,
            };
        }
        let (line, scale) = row_at(row);
        // Homes in the back row stand a step or two further back, each by
        // its own seed, so a street is not one straight line.
        let back = if item.px.is_some() && row < 0.35 && item.shape == Some(MarkShape::House) {
            (art::seed_of(&item.id.stable_key()) % 3) as f32 * building_h * scale * 0.04
        } else {
            0.0
        };
        Spot {
            index,
            x,
            y: line * height - back,
            w: nominal(index, scale),
            scale,
        }
    };
    let mut rows = BTreeMap::<usize, i32>::new();
    let row_key = |index: usize, row: f32| {
        if boat(index) {
            1000
        } else {
            (row * 100.0).round() as i32
        }
    };
    let mut buildings = Vec::new();
    let mut things = Vec::new();
    for (index, x) in &slot_x {
        let row = row_of(*index);
        rows.insert(*index, row_key(*index, row));
        let spot = spot_in_row(*index, *x, row);
        if items[*index].kind == CanvasItemKind::Place {
            buildings.push(spot);
        } else {
            things.push(spot);
        }
    }
    // Things kept at a place stand beside it, on the quay: an order waits
    // at the bakery's door, a boat is moored off the harbour's side.
    let mut beside = BTreeMap::<usize, usize>::new();
    for (index, item) in items.iter().enumerate() {
        if item.kind == CanvasItemKind::Actor || free(index) {
            continue;
        }
        if let Some(spot) = item.spot {
            rows.insert(index, row_key(index, 0.76));
            things.push(spot_in_row(
                index,
                width * MARGIN + usable * spot.clamp(0.0, 1.0),
                0.76,
            ));
            continue;
        }
        let Some(host) = host(index) else {
            continue;
        };
        let Some(host_x) = slot_x.get(&host) else {
            continue;
        };
        let nth = beside.entry(host).or_default();
        let side = if nth.is_multiple_of(2) { 1.0 } else { -1.0 };
        let out = 1.0 + (*nth / 2) as f32;
        *nth += 1;
        // A boat is moored toward the nearer edge, out of the way of the
        // card that floats over the middle of the water.
        let side = if boat(index) {
            if *host_x < width / 2.0 {
                -1.0
            } else {
                1.0
            }
        } else {
            side
        };
        rows.insert(index, row_key(index, 0.76));
        things.push(spot_in_row(
            index,
            host_x + side * (building_w * 0.5 + thing_w * 0.6 * out),
            0.76,
        ));
    }
    let mut shown_from = compose(
        items,
        &mut buildings,
        &mut things,
        &rows,
        (width, height * FEET),
        (building_h, figure_h),
    );
    // Back rows first, so nearer buildings stand in front of them.
    buildings.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));

    // Who is indoors this hour, and who is still on their way somewhere.
    let mut inside = Vec::new();
    let mut leaving = BTreeMap::<usize, (usize, f32)>::new();
    for (index, item) in items.iter().enumerate() {
        if item.kind != CanvasItemKind::Actor {
            continue;
        }
        let Some((now, before)) = routine(index) else {
            continue;
        };
        let (Some(to), Some(from)) = (placed(now.at), placed(before.at)) else {
            continue;
        };
        let (Some(to_x), Some(from_x)) = (slot_x.get(&to), slot_x.get(&from)) else {
            continue;
        };
        let pace = item.look.map_or(1.0, |look| Age::of(&look).pace());
        let walk = (to_x - from_x).abs() / (figure_h * STROLL * pace);
        let walking = before.at != now.at && clock.into_hour < walk;
        if walking {
            leaving.insert(index, (from, clock.into_hour));
        } else if now.inside {
            inside.push((index, to));
        }
    }
    // A place of a keeper or two (a new place, its first night) is never shown
    // empty: when the hour has everyone indoors, the first of them is out
    // at their own door, beside the place they keep.
    let actors = items
        .iter()
        .filter(|item| item.kind == CanvasItemKind::Actor)
        .count();
    if actors <= 3 && !inside.is_empty() && inside.len() == actors {
        inside.remove(0);
    }
    // Who strolls down to the water at dusk, and in front of which place.
    let mut strolling = BTreeMap::<usize, usize>::new();
    // At dusk the spine is never empty: before going in, a few of those whose
    // day has them home stand out by the water a while, in front of their
    // own doors, those nearest the middle of the place first (the art
    // bible's §4 and §8: a gold dusk, people out by the water). Only where
    // they are drawn changes; the World keeps where they are.
    if (18..=20).contains(&clock.hour) && inside.len() > 1 {
        // Counted on the first screen, the middle of the place.
        let middle = width / 2.0;
        let near = |x: f32| (x - middle).abs() < view_w * 0.42;
        let indoors_now = inside
            .iter()
            .map(|(person, _)| *person)
            .collect::<BTreeSet<_>>();
        let out = (0..items.len())
            .filter(|index| {
                items[*index].kind == CanvasItemKind::Actor
                    && !indoors_now.contains(index)
                    && !leaving.contains_key(index)
                    && host(*index)
                        .and_then(|place| slot_x.get(&place))
                        .is_some_and(|x| near(*x))
            })
            .count();
        let wanted = DUSK_OUT.min(actors - 1);
        if out < wanted {
            let mut by_middle = inside
                .iter()
                .enumerate()
                .map(|(at, (_, home))| {
                    let x = slot_x.get(home).copied().unwrap_or(f32::MAX);
                    ((x - middle).abs(), at)
                })
                .collect::<Vec<_>>();
            by_middle.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let stepping_out = by_middle
                .iter()
                .take(wanted - out)
                .map(|(_, at)| *at)
                .collect::<BTreeSet<_>>();
            // Whoever lives further off strolls down to the water by the
            // middle of the place: in front of the two places nearest it,
            // turn and turn about, so they stand as two groups.
            let mut by_the_water = slot_x
                .iter()
                .filter(|(index, _)| items[**index].kind == CanvasItemKind::Place)
                .map(|(index, x)| ((x - middle).abs(), *index))
                .collect::<Vec<_>>();
            by_the_water.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let mut turn = 0;
            for &at in &stepping_out {
                let (person, home) = inside[at];
                let x = slot_x.get(&home).copied().unwrap_or(f32::MAX);
                if !near(x) {
                    if let Some((_, place)) =
                        by_the_water.get(turn % by_the_water.len().clamp(1, 2))
                    {
                        strolling.insert(person, *place);
                    }
                    turn += 1;
                }
            }
            let mut at = 0;
            inside.retain(|_| {
                let keep = !stepping_out.contains(&at);
                at += 1;
                keep
            });
        }
    }
    let indoors = inside
        .iter()
        .map(|(person, _)| *person)
        .collect::<BTreeSet<_>>();

    // People stand on the quay in front of wherever they are, in a group
    // round it, pairs side by side.
    let spacing = figure_h * 0.62;
    let pairs = pairs(snapshot);
    let mut wanted = Vec::new();
    let mut hosted = BTreeMap::<usize, Vec<usize>>::new();
    let mut loose = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if item.kind != CanvasItemKind::Actor || indoors.contains(&index) {
            continue;
        }
        match strolling
            .get(&index)
            .copied()
            .or_else(|| host(index))
            .filter(|host| slot_x.contains_key(host))
        {
            Some(host) => hosted.entry(host).or_default().push(index),
            None => loose.push(index),
        }
    }
    for (host, mut members) in hosted {
        for (a, b) in &pairs {
            let find = |id: &SelectionId, members: &[usize]| {
                members.iter().position(|member| items[*member].id == *id)
            };
            if let (Some(first), Some(second)) = (find(a, &members), find(b, &members)) {
                let partner = members.remove(second);
                let first = if second < first { first - 1 } else { first };
                members.insert(first + 1, partner);
            }
        }
        let row = spacing * members.len().saturating_sub(1) as f32;
        for (position, member) in members.into_iter().enumerate() {
            wanted.push((
                member,
                slot_x[&host] - row / 2.0 + spacing * position as f32,
            ));
        }
    }
    for index in loose {
        let x = along(&items[index])
            .unwrap_or_else(|| width * MARGIN + usable * items[index].x.clamp(0.0, 1.0));
        wanted.push((index, x));
    }
    wanted.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    // Nobody stands on anybody, nor in anything standing on the quay:
    // each takes the nearest room to where they would be.
    let quay = height * FEET;
    let room = figure_h * 0.5;
    let blocked = things
        .iter()
        .filter(|spot| {
            Depth::at(spot.y, height) == Depth::Quay
                && shown_from
                    .get(&spot.index)
                    .is_none_or(|least| least.is_finite())
        })
        .map(|spot| (spot.x - spot.w / 2.0, spot.x + spot.w / 2.0))
        .collect::<Vec<_>>();
    // What stands on the water line in front of the quay (the tower on
    // the point, a boathouse) opens onto it: nobody stands in its door, or
    // in front of it at all, in either lane.
    let fronted = buildings
        .iter()
        .filter(|spot| {
            matches!(Depth::at(spot.y, height), Depth::Quay | Depth::Water) && spot.y > quay
        })
        .map(|spot| (spot.x - spot.w * 0.45, spot.x + spot.w * 0.45))
        .collect::<Vec<_>>();
    let blocked = blocked
        .into_iter()
        .chain(fronted.iter().copied())
        .collect::<Vec<_>>();
    // The quay has two lanes: among what stands on it, and a step nearer,
    // in front of everything, for a crowd the first has no room for.
    let lane = figure_h * 0.45;
    let mut people: Vec<Spot> = Vec::new();
    let reach = view_w * 0.6;
    let nudge = spacing * 0.25;
    for (index, want) in wanted {
        let clear = |x: f32, y: f32| {
            x >= spacing * 0.5
                && x <= width - spacing * 0.5
                && (y > quay
                    || blocked
                        .iter()
                        .all(|(left, right)| x + room / 2.0 <= *left || x - room / 2.0 >= *right))
                && fronted
                    .iter()
                    .all(|(left, right)| x + room / 2.0 <= *left || x - room / 2.0 >= *right)
                && people
                    .iter()
                    .filter(|other| other.y == y)
                    .all(|other| (other.x - x).abs() >= spacing - 0.01)
        };
        let steps = (reach / nudge) as i32;
        let found = [quay, quay + lane].into_iter().find_map(|y| {
            (0..=steps)
                .flat_map(|step| [step, -step])
                .map(|step| want + step as f32 * nudge)
                .find(|x| clear(*x, y))
                .map(|x| (x, y))
        });
        let (x, y) = found.unwrap_or((want.clamp(spacing * 0.5, width - spacing * 0.5), quay));
        if found.is_none() {
            // No room on the quay: this one waits for a closer look.
            shown_from.insert(index, f32::INFINITY);
        }
        people.push(Spot {
            index,
            x,
            y,
            w: room,
            scale: 1.0,
        });
    }
    // A crowd stands at more than one depth: every other person in a row,
    // and anyone standing on their own, a step nearer or further, so the
    // quay reads as ground rather than a line.
    people.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.index.cmp(&b.index)));
    let step = figure_h * 0.2;
    for (position, spot) in people.iter_mut().enumerate() {
        let seed = art::seed_of(&items[spot.index].id.stable_key());
        let depth = match position % 3 {
            0 => 0.0,
            1 => 1.0,
            _ => 0.5,
        } + (seed % 7) as f32 / 30.0;
        // In the front lane only a little: it is a step nearer already.
        let near = if spot.y > quay { lane } else { 0.0 };
        let step = if near > 0.0 { step * 0.3 } else { step };
        spot.y = quay + near + step * (depth - 0.5);
        spot.scale = 1.0 + (spot.y - quay) / figure_h * 0.35;
    }
    // People stand in twos and threes facing each other, not in rows (A2).
    super::people::gather(&mut people, figure_h, quay, &blocked, &fronted);
    people.sort_by_key(|spot| spot.index);
    let routes = leaving
        .into_iter()
        .filter_map(|(person, (from, since))| Some((person, (*slot_x.get(&from)?, since))))
        .collect();

    Stage {
        width,
        height,
        view_w,
        horizon: height * HORIZON,
        base: height * row_at(0.30).0,
        street: height * row_at(0.60).0,
        feet: quay,
        front: height * FRONT,
        building_w,
        building_h,
        figure_h,
        thing_w,
        buildings,
        things,
        people,
        inside,
        routes,
        plots: plots_on(snapshot, view_w, height, (building_w, grown)),
        shown_from,
        grown,
    }
}

/// The quay's row, as [`compose`] keys rows (the Pack's `y` in hundredths).
pub(super) const QUAY_ROW: i32 = 76;
/// The first of the water line's rows, as [`compose`] keys them.
pub(super) const WATER_KEY: i32 = 80;

/// How far apart, on screen, two neighbours in a row must stand for both
/// to show: closer, the lesser waits for the camera to come nearer.
pub(super) const ELBOW_ROOM: f32 = 6.0;
/// The least a thing is drawn across, on screen: smaller, it is left for
/// a closer look.
pub(super) const LEAST_SEEN: f32 = 12.0;

/// The composition pass: sees that nothing stands on anything else, and
/// what cannot stand clear waits for a closer look. In each of the Pack's
/// rows (`rows`, by item) what matters most is placed first (what the
/// player built on a plot, then places, then the works of the town, then
/// the rest), each as wide as it can be up to its own width, never into
/// what is already there. A thing that does not fit steps aside to the
/// nearest room in its row; a thing in front of nothing nearer stands
/// clear of every nearer building too, since things are drawn over the
/// buildings. What fits only barely shows from the zoom where the gap
/// reads ([`ELBOW_ROOM`]), and what is too small to see shows once it is
/// big enough ([`LEAST_SEEN`]). Returns the least zoom for each item held
/// back.
pub(super) fn compose(
    items: &[CanvasItem],
    buildings: &mut [Spot],
    things: &mut [Spot],
    rows: &BTreeMap<usize, i32>,
    (width, quay): (f32, f32),
    (building_h, figure_h): (f32, f32),
) -> BTreeMap<usize, f32> {
    let gap = figure_h * 0.12;
    let rank = |index: usize| {
        let item = &items[index];
        match (item.variant.is_some(), item.kind == CanvasItemKind::Place) {
            (true, _) => 0,
            (false, true) => 1,
            _ if item.built.is_some() => 2,
            _ => 3,
        }
    };
    let tall = |spot: &Spot, place: bool| {
        if place {
            building_h
                * spot.scale
                * match items[spot.index].shape.unwrap_or_default() {
                    MarkShape::Bridge => 0.7,
                    MarkShape::Tower => 1.2,
                    MarkShape::Dome | MarkShape::Tree => 0.8,
                    _ => 1.0,
                }
        } else {
            spot.w * 1.1
        }
    };
    // Everything placed so far: row, x, width, the top and foot of its box,
    // and whether it is a building.
    struct Placed {
        row: i32,
        x: f32,
        w: f32,
        top: f32,
        foot: f32,
        building: bool,
    }
    let mut placed: Vec<Placed> = Vec::new();
    let mut shown_from = BTreeMap::<usize, f32>::new();
    // What came forward to the quay, and so stands in its row now.
    let mut forward_rows = BTreeMap::<usize, i32>::new();
    // The room left at `x` in `row` for something `foot` deep and as tall
    // as `top` reaches: as wide as it may be, and whether a nearer
    // building stands in its way.
    let room = |placed: &[Placed], row: i32, x: f32, w: f32, top: f32, foot: f32, thing: bool| {
        let mut widest = f32::MAX;
        for other in placed {
            if other.row == row {
                widest = widest.min(2.0 * ((x - other.x).abs() - other.w / 2.0 - gap));
            } else if thing
                && other.building
                && other.foot > foot + 1.0
                && (x - other.x).abs() < (w + other.w) / 2.0
                && other.top < foot
                && top < other.foot
            {
                return -1.0;
            }
        }
        widest
    };
    // Whether a building standing in the box `(x, w, top, foot)` would
    // leave any building (itself, or one behind it) more than half hidden
    // behind nearer ones.
    let veils = |placed: &[Placed], (x, w, top, foot): (f32, f32, f32, f32)| {
        let hidden = |x: f32, w: f32, top: f32, foot: f32, extra: Option<(f32, f32, f32, f32)>| {
            let area = (w * (foot - top)).max(1.0);
            let cover = |ox: f32, ow: f32, otop: f32, ofoot: f32| {
                if ofoot <= foot + 1.0 {
                    return 0.0;
                }
                let across =
                    ((x + w / 2.0).min(ox + ow / 2.0) - (x - w / 2.0).max(ox - ow / 2.0)).max(0.0);
                let down = (foot.min(ofoot) - top.max(otop)).max(0.0);
                across * down
            };
            let mut covered = placed
                .iter()
                .filter(|other| other.building)
                .map(|other| cover(other.x, other.w, other.top, other.foot))
                .sum::<f32>();
            if let Some((ox, ow, otop, ofoot)) = extra {
                covered += cover(ox, ow, otop, ofoot);
            }
            covered > area * 0.5
        };
        hidden(x, w, top, foot, None)
            || placed
                .iter()
                .filter(|other| other.building && other.foot < foot - 1.0)
                .any(|other| {
                    hidden(
                        other.x,
                        other.w,
                        other.top,
                        other.foot,
                        Some((x, w, top, foot)),
                    )
                })
    };
    // A work of the town's may step aside for another building; what the
    // player built, a place the World began with and a home stand where
    // they are.
    let movable = |building: bool, index: usize| {
        building && items[index].variant.is_none() && items[index].built.is_some()
    };
    // A work going up stands in scaffolding on a site of its own: never
    // over anything already standing, in any row (the art bible's §7;
    // v0.29 round 2's scaffolding over the net store and over a stall).
    // It is sited last, where there is room, and stands as a building for
    // whatever comes after.
    let going_up = |building: bool, index: usize| {
        !building && rank(index) != 0 && items[index].art.as_deref() == Some("scaffold")
    };
    let site_clear = |placed: &[Placed], x: f32, w: f32, top: f32, foot: f32| {
        !placed.iter().any(|other| {
            (x - other.x).abs() < (w + other.w) / 2.0 + gap && top < other.foot && other.top < foot
        })
    };
    // What the player built and the places first, where they stand; then
    // the town's works and everything else, each where there is room.
    let mut order = buildings
        .iter()
        .map(|spot| (true, spot.index))
        .chain(things.iter().map(|spot| (false, spot.index)))
        .collect::<Vec<_>>();
    order.sort_by_key(|(building, index)| {
        let key = match (*building, movable(*building, *index), rank(*index)) {
            (_, _, 0) => 0,
            (true, false, _) => 1,
            (true, true, _) => 2,
            _ if going_up(*building, *index) => 9,
            (false, _, rank) => 2 + rank,
        };
        (key, *index)
    });
    for (building, index) in order {
        let list: &[Spot] = if building { buildings } else { things };
        let Some(position) = list.iter().position(|spot| spot.index == index) else {
            continue;
        };
        let spot = if building {
            buildings[position]
        } else {
            things[position]
        };
        let row = rows.get(&index).copied().unwrap_or_default();
        let nominal = spot.w;
        let h = tall(&spot, building);
        let fixed = (building && !movable(building, index)) || rank(index) == 0;
        // A building that may step aside never stands in front of a thing
        // already placed behind it (what the player built on a plot).
        let hides_a_thing = |x: f32| {
            placed.iter().any(|other| {
                !other.building
                    && other.foot < spot.y - 1.0
                    && (x - other.x).abs() < (nominal + other.w) / 2.0
                    && other.top < spot.y
                    && spot.y - h < other.foot
            })
        };
        let site = going_up(building, index);
        let room_at = |x: f32| {
            if building
                && !fixed
                && (veils(&placed, (x, nominal, spot.y - h, spot.y)) || hides_a_thing(x))
            {
                return -1.0;
            }
            if site && !site_clear(&placed, x, nominal, spot.y - h, spot.y) {
                return -1.0;
            }
            room(&placed, row, x, nominal, spot.y - h, spot.y, !building)
                .min(2.0 * x.min(width - x))
        };
        let mut x = spot.x;
        let mut w = nominal.min(room_at(x));
        if w < nominal * 0.75 && !fixed {
            // Step aside to the nearest room in the row: a thing as far as
            // about four people wide, so it keeps to its own ground.
            let nudge = nominal * 0.1;
            let steps = if building {
                16
            } else {
                ((figure_h * 4.0 / nudge.max(1.0)) as i32).clamp(16, 60)
            };
            if let Some(better) = (1..=steps)
                .flat_map(|step| [step, -step])
                .map(|step| spot.x + step as f32 * nudge)
                .find(|x| room_at(*x) >= nominal * 0.9)
            {
                x = better;
                w = nominal.min(room_at(x));
            }
        }
        if w < nominal * 0.6 {
            if building {
                // A building never goes: with no room anywhere near, it
                // stands where it was put, as narrow as it must.
                x = spot.x;
                w = room(&placed, row, x, nominal, spot.y - h, spot.y, false)
                    .min(nominal)
                    .max(nominal * 0.5);
            } else if site && ((WATER_KEY..1000).contains(&row) || row < QUAY_ROW - 6) {
                // A site with no room anywhere near in its row waits off
                // the scene rather than go up over something standing.
                shown_from.insert(index, f32::INFINITY);
                continue;
            } else if fixed || (WATER_KEY..1000).contains(&row) || row < QUAY_ROW - 6 {
                // What the player built, a place, what stands on the
                // water line and what stands back in the town stay in
                // their row, as narrow as they must: a pier never comes
                // ashore and a duck pond never comes down to the quay.
                w = w.max(nominal * 0.5);
            } else {
                // No room in its row: it comes forward onto the quay, full
                // size, to the nearest room there.
                let full = nominal / spot.scale.max(0.1);
                let quay_room = |x: f32| {
                    let (top, foot) = (quay - full * 1.1, quay);
                    if site && !site_clear(&placed, x, full, top, foot) {
                        return -1.0;
                    }
                    room(&placed, QUAY_ROW, x, full, top, foot, true).min(2.0 * x.min(width - x))
                };
                let nudge = full * 0.1;
                let Some(forward) = (0..=120)
                    .flat_map(|step| [step, -step])
                    .map(|step| spot.x + step as f32 * nudge)
                    .find(|x| quay_room(*x) >= full * 0.9)
                else {
                    shown_from.insert(index, f32::INFINITY);
                    continue;
                };
                let list: &mut [Spot] = things;
                list[position] = Spot {
                    x: forward,
                    y: quay,
                    w: full.min(quay_room(forward)),
                    scale: 1.0,
                    ..spot
                };
                let moved = list[position];
                placed.push(Placed {
                    row: QUAY_ROW,
                    x: moved.x,
                    w: moved.w,
                    top: quay - moved.w * 1.1,
                    foot: quay,
                    building: site,
                });
                forward_rows.insert(index, QUAY_ROW);
                continue;
            }
        }
        let list: &mut [Spot] = if building { buildings } else { things };
        list[position].x = x;
        list[position].w = w;
        placed.push(Placed {
            row,
            x,
            w,
            top: spot.y - tall(&list[position], building),
            foot: spot.y,
            building: building || site,
        });
    }
    // What stands only just clear of a neighbour in its row shows from the
    // zoom where the gap between them reads; the lesser of the two waits.
    let mut by_row = BTreeMap::<i32, Vec<(f32, f32, usize)>>::new();
    for spot in buildings.iter().chain(things.iter()) {
        if shown_from.contains_key(&spot.index) {
            continue;
        }
        by_row
            .entry(
                forward_rows
                    .get(&spot.index)
                    .or(rows.get(&spot.index))
                    .copied()
                    .unwrap_or_default(),
            )
            .or_default()
            .push((spot.x, spot.w, spot.index));
    }
    for members in by_row.values_mut() {
        members.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in members.windows(2) {
            let ((x0, w0, i0), (x1, w1, i1)) = (pair[0], pair[1]);
            let between = (x1 - x0) - (w0 + w1) / 2.0;
            let lesser = if (rank(i0), i0) > (rank(i1), i1) {
                i0
            } else {
                i1
            };
            if rank(lesser) == 0 || items[lesser].kind == CanvasItemKind::Place {
                continue;
            }
            let least = ELBOW_ROOM / between.max(0.01);
            if least > 0.3 {
                let entry = shown_from.entry(lesser).or_insert(0.0);
                *entry = entry.max(least);
            }
        }
    }
    for spot in things.iter() {
        if rank(spot.index) == 0 {
            continue;
        }
        let least = LEAST_SEEN / spot.w.max(0.01);
        if least > 0.3 {
            let entry = shown_from.entry(spot.index).or_insert(0.0);
            *entry = entry.max(least);
        }
    }
    shown_from
}

/// Where a World's plots lie on the stage: along the panorama where the
/// World says, in their row, each about a building wide.
pub(super) fn plots_on(
    snapshot: &ProjectionSnapshot,
    view_w: f32,
    height: f32,
    (building_w, grown): (f32, f32),
) -> Vec<PlotPaint> {
    let panorama = snapshot.canvas.width.unwrap_or(1.0).clamp(1.0, 24.0);
    crate::mark::plots_of(snapshot)
        .iter()
        .enumerate()
        .map(|(plot, at)| {
            // Rows as a Pack lays its places out, counted from the back:
            // the back row, and each row after it a step nearer the water,
            // where what is built there will stand.
            let (line, scale) = row_aged(0.30 + at.row as f32 * 0.095, grown);
            PlotPaint {
                plot,
                x: at.px.clamp(0.0, panorama) * view_w,
                y: line * height + building_w * scale * 0.05,
                w: building_w * scale * 0.85,
            }
        })
        .collect()
}

/// Where the camera looks: `zoom` times closer, centred on (`x`, `y`) in
/// stage pixels. At rest it looks at the whole stage, or along a panorama
/// one window of it. Zoomed right out, a panorama too long for the window
/// folds into a postcard (`fold` of the way, 0 to 1): its stretches stacked
/// one behind another, the first in front by the water, the last up the
/// hill against the sky.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
    pub fold: f32,
}

/// The closest the camera goes.
pub const ZOOM_MOST: f32 = 2.2;

/// How a place folds into a postcard: into `rows` stretches, each
/// `row_w` stage pixels long and drawn at `zoom` so it fills the window's
/// width, each standing `step` stage pixels up the hill from the one in
/// front, so its street stands where the field in front begins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fold {
    pub rows: usize,
    pub zoom: f32,
    pub row_w: f32,
    pub step: f32,
}

impl Fold {
    /// How a stage `width` long, seen through a window `view_w` by
    /// `height`, folds.
    pub fn of(width: f32, view_w: f32, height: f32) -> Self {
        let panorama = (width / view_w.max(1.0)).max(1.0);
        let rows = ((panorama / 1.6).ceil() as usize).max(1);
        let field = height * (HORIZON + (BASE - HORIZON) * 0.3);
        Self {
            rows,
            zoom: (rows as f32 / panorama).min(1.0),
            row_w: width / rows as f32,
            step: height * row_at(0.60).0 - field,
        }
    }

    pub fn of_stage(stage: &Stage) -> Self {
        Self::of(stage.width, stage.view_w, stage.height)
    }

    /// The furthest out the camera goes: the place folded right up, or for
    /// a place that does not fold, all of it at once.
    pub fn least(&self) -> f32 {
        self.zoom
    }

    /// How far a place that folds has folded with the camera `zoom` times
    /// closer: not at all at one window, right up at its rows' zoom.
    pub fn folded(&self, zoom: f32) -> f32 {
        if self.rows < 2 || zoom >= 1.0 {
            return 0.0;
        }
        ease((1.0 - zoom) / (1.0 - self.zoom).max(1e-4))
    }

    /// Which row a stage point `x` along stands in.
    pub fn row_of(&self, x: f32) -> usize {
        ((x / self.row_w.max(1.0)).floor().max(0.0) as usize).min(self.rows - 1)
    }
}

/// The camera's closest and furthest along one axis: a view `half` wide
/// either side kept on a stage `extent` long, or centred when it is wider.
pub(super) fn keep_on(at: f32, half: f32, extent: f32) -> f32 {
    if half * 2.0 >= extent {
        extent / 2.0
    } else {
        at.clamp(half, extent - half)
    }
}

impl Camera {
    /// The middle of the place, at the window's own size.
    pub fn whole(stage: &Stage) -> Self {
        Self {
            zoom: 1.0,
            x: stage.width / 2.0,
            y: stage.height / 2.0,
            fold: 0.0,
        }
    }

    /// The furthest out the camera goes: one window, or for a panorama the
    /// whole of it at once, folded into a postcard when it is long.
    pub fn least(stage: &Stage) -> f32 {
        Fold::of_stage(stage).least().min(1.0)
    }

    /// Close enough on a box to see who is in it, and no closer than twice.
    pub fn on(stage: &Stage, (x, y, w, h): (f32, f32, f32, f32)) -> Self {
        let room = (stage.view_w * 0.55 / w.max(1.0)).min(stage.height * 0.45 / h.max(1.0));
        let zoom = room.clamp(1.0, 1.8);
        // Keep the view inside the stage: never show past its edges.
        let half_w = stage.view_w / zoom / 2.0;
        let half_h = stage.height / zoom / 2.0;
        Self {
            zoom,
            x: keep_on(x + w / 2.0, half_w, stage.width),
            // Frame a little above centre, so the card below does not
            // cover what is being looked at.
            y: keep_on(y + h * 0.7, half_h, stage.height),
            fold: 0.0,
        }
    }

    /// `zoom` times closer around a stage point, kept inside the stage.
    /// Zoomed out past one window, the water stays at the bottom and the
    /// sky opens above, or a place too long for the window folds as it goes.
    pub fn around(stage: &Stage, zoom: f32, x: f32, y: f32) -> Self {
        let fold = Fold::of_stage(stage);
        let zoom = zoom.clamp(Self::least(stage), ZOOM_MOST);
        let folded = fold.folded(zoom);
        let half_w = stage.view_w / zoom / 2.0;
        let half_h = stage.height / zoom / 2.0;
        Self {
            zoom,
            x: keep_on(x, half_w, stage.width),
            y: if half_h * 2.0 >= stage.height {
                stage.height - half_h
            } else {
                y.clamp(half_h, stage.height - half_h)
            },
            fold: folded,
        }
    }

    /// The camera row `row` of the postcard is seen through, `fold` of the
    /// way from where it lies along the panorama to where it stands in the
    /// postcard: across the window, and up the hill behind the rows in
    /// front of it.
    pub fn row(&self, fold: &Fold, view_w: f32, height: f32, row: usize) -> Self {
        if self.fold <= 0.0 || fold.rows < 2 {
            return Self { fold: 0.0, ..*self };
        }
        // Each row centred across the window: exactly its width once
        // folded right up.
        let x = (row as f32 + 0.5) * fold.row_w;
        let _ = view_w;
        let y = height - height / (2.0 * self.zoom) + fold.step * row as f32;
        Self {
            zoom: self.zoom,
            x: self.x + (x - self.x) * self.fold,
            y: self.y + (y - self.y) * self.fold,
            fold: 0.0,
        }
    }

    /// The stage point under a screen point: on a folded postcard, in the
    /// nearest row whose town reaches up that far.
    pub fn stage_point(&self, stage: &Stage, x: f32, y: f32) -> (f32, f32) {
        let flat = |camera: &Camera| {
            (
                (x - stage.view_w / 2.0) / camera.zoom + camera.x,
                (y - stage.height / 2.0) / camera.zoom + camera.y,
            )
        };
        let fold = Fold::of_stage(stage);
        if self.fold <= 0.0 || fold.rows < 2 {
            return flat(self);
        }
        let roofs = stage.base - stage.building_h * row_at(0.30).1;
        let mut last = flat(self);
        for row in 0..fold.rows {
            let camera = self.row(&fold, stage.view_w, stage.height, row);
            last = flat(&camera);
            if last.1 >= roofs {
                let (from, to) = (row as f32 * fold.row_w, (row + 1) as f32 * fold.row_w);
                return (last.0.clamp(from, to), last.1);
            }
        }
        last
    }

    pub fn toward(self, target: Camera, t: f32) -> Self {
        let t = ease(t);
        Self {
            zoom: self.zoom + (target.zoom - self.zoom) * t,
            x: self.x + (target.x - self.x) * t,
            y: self.y + (target.y - self.y) * t,
            fold: self.fold + (target.fold - self.fold) * t,
        }
    }

    /// Where a stage point lands on screen.
    pub fn at(&self, stage: &Stage, x: f32, y: f32) -> (f32, f32) {
        self.project((stage.width, stage.view_w, stage.height), x, y)
    }

    /// Where a stage point lands on screen, for a stage `width` long seen
    /// through a window `view_w` by `height`: through the camera of its row
    /// when the place is folding.
    pub(super) fn project(
        &self,
        (width, view_w, height): (f32, f32, f32),
        x: f32,
        y: f32,
    ) -> (f32, f32) {
        let camera = if self.fold > 0.0 {
            let fold = Fold::of(width, view_w, height);
            self.row(&fold, view_w, height, fold.row_of(x))
        } else {
            *self
        };
        (
            (x - camera.x) * camera.zoom + view_w / 2.0,
            (y - camera.y) * camera.zoom + height / 2.0,
        )
    }
}

/// The band of far hills: wider than the window by as much as it moves
/// along a panorama, standing on the horizon.
pub(super) struct Band {
    /// Its width, and how far it reaches above and below the horizon, in
    /// window pixels.
    pub(super) w: f32,
    pub(super) above: f32,
    pub(super) below: f32,
    /// How far it reaches past the window's left edge at rest.
    pub(super) pad: f32,
    pub(super) view_w: f32,
    pub(super) view_h: f32,
}

impl Band {
    pub(super) fn of(frame: &Frame, width: f32, height: f32) -> Self {
        let pad = width * 0.12;
        Self {
            w: width + (frame.width - frame.view_w).max(0.0) * HILLS + pad * 2.0,
            above: height * 0.22,
            below: height * 0.17,
            pad,
            view_w: width,
            view_h: height,
        }
    }

    /// Where its top-left corner is on screen.
    pub(super) fn screen(&self, frame: &Frame) -> (f32, f32) {
        let horizon = frame.at(0.0, frame.horizon).1;
        (
            -self.pad - frame.pan() * HILLS,
            horizon - self.above * Self::squash(frame),
        )
    }

    /// Zoomed out past one window, the hills are lower, in keeping with
    /// the smaller town.
    pub(super) fn squash(frame: &Frame) -> f32 {
        frame.camera.zoom.clamp(0.45, 1.0)
    }

    /// The top of a range of hills at `x` along the band: `depth` 0 is the
    /// farthest, 2 the ridge nearest.
    pub(super) fn ridge(&self, depth: usize, x: f32, seed: u32) -> f32 {
        let amp = [0.12, 0.085, 0.06][depth.min(2)] * self.view_h;
        let w = self.view_w.max(1.0);
        let phase =
            |salt: u32| (painter::hash2(depth as i32, salt as i32, seed) % 628) as f32 / 100.0;
        let shape = 0.55
            + 0.25 * (x / (0.33 * w) + phase(1)).sin()
            + 0.14 * (x / (0.14 * w) + phase(2)).sin()
            + 0.06 * (x / (0.047 * w) + phase(3)).sin();
        self.above - amp * shape - (2 - depth.min(2)) as f32 * self.view_h * 0.01
    }

    /// Where a built thing `along` the ridge stands, in band pixels: its
    /// middle and its foot.
    pub(super) fn mark_at(&self, frame: &Frame, along: f32) -> (f32, f32) {
        let x = self.pad + along * (self.w - self.pad * 2.0).max(1.0);
        (
            x,
            self.ridge(2, x, seed_of_scenery(&frame.scenery)) + self.view_h * 0.012,
        )
    }
}

pub(super) fn seed_of_scenery(scenery: &Scenery) -> u32 {
    scenery.far ^ scenery.near.rotate_left(9) ^ scenery.sky_top.rotate_left(17)
}

/// The far hills, each range paler with the air between, what the World
/// has built standing on the nearest, and a line of low sun along their
/// tops at dawn and dusk.
#[cfg(test)]
pub(super) fn paint_band(frame: &Frame, band: &Band, dpr: f32) -> Option<sk::Pixmap> {
    let tall = band.above + band.below;
    // Far off and seen through air, the hills are soft: half the display's
    // resolution is plenty.
    let scale = (dpr * 0.5).max(1.0);
    painter::in_strips(
        (band.w * scale).ceil() as u32,
        (tall * scale).ceil() as u32,
        scale,
        (0.0, 0.0),
        &|canvas, at| painter::timed("band", || paint_band_on(canvas, at, frame, band)),
    )
}

/// The far hills onto one strip of their band.
pub(super) fn paint_band_on(canvas: &mut Canvas, at: (i32, i32), frame: &Frame, band: &Band) {
    let tall = band.above + band.below;
    let seed = seed_of_scenery(&frame.scenery);
    let far = art::hex(frame.scenery.far);
    let haze = frame.haze();
    let low = matches!(frame.daylight, Daylight::Dawn | Daylight::Dusk) && sun_out(frame.weather);
    let (cover_ink, cover_share) = match frame.cover {
        Some(GroundCover::Snow) => (art::hex(0xf2f5f8), [0.7, 0.6, 0.45]),
        Some(GroundCover::Frost) => (art::hex(0xe6edf2), [0.3, 0.25, 0.2]),
        Some(GroundCover::Leaves) => (art::hex(0xc8783a), [0.12, 0.2, 0.3]),
        Some(GroundCover::Dust) => (art::hex(0xc0704a), [0.2, 0.22, 0.25]),
        _ => (far, [0.0, 0.0, 0.0]),
    };
    let (from, to) = (
        canvas.origin.0 - 20.0,
        canvas.origin.0 + canvas.width() as f32 / canvas.scale + 20.0,
    );
    let step = 10.0;
    for depth in 0..3 {
        // Less air than v0.28's: the milky veil the v0.29 art director
        // saw over the whole picture.
        let air = [0.46, 0.2, 0.0][depth];
        let ink = art::shade(mix(far, haze, air), if depth == 2 { -0.08 } else { 0.0 });
        let ink = mix(ink, cover_ink, cover_share[depth]);
        let mut shape = Shape::new();
        let mut crest = Shape::new();
        let mut x = (from / step).floor() * step;
        shape.move_to(x, tall + 2.0);
        crest.move_to(x, band.ridge(depth, x, seed));
        while x <= to {
            let y = band.ridge(depth, x, seed);
            shape.line_to(x, y);
            crest.line_to(x, y);
            x += step;
        }
        shape.line_to(x, tall + 2.0).close();
        canvas.fill(&shape, ink);
        // A fine line of ink along the nearer ridges' tops, so each reads
        // against the one behind; warm with the low sun.
        if depth > 0 {
            let rim = if low {
                mix(art::hex(0xffc27a), ink, 0.45).opacity(0.4)
            } else {
                art::shade(ink, -0.35).opacity(0.22)
            };
            canvas.stroke(&crest, 1.1, rim);
        }
    }
    // The air between: every range paler toward its foot. On the ice the
    // flat field fades in over the band's foot, so the air thickens on
    // down under it rather than levelling off where the field begins (v0.29
    // round 3: the gradient's knee showed there as a faint line across the
    // ice).
    let (thickest, knee) = if frame.setting == art::Setting::Ice {
        (0.5, 1.0)
    } else {
        (0.3, 0.55)
    };
    canvas.gradient(
        from,
        band.above - band.view_h * 0.1,
        to - from,
        band.view_h * 0.1 + band.below,
        180.0,
        (haze.opacity(0.0), 0.0),
        (haze.opacity(thickest), knee),
    );
    // The back row: the place's own drawings, far off and pale with the
    // air between (ground.rs).
    super::ground::paint_back_row(canvas, frame, band, (from, to));
    // What the World has built stands along the ridge.
    let silhouette = mix(art::shade(far, -0.3), haze, 0.12);
    let sun = art::hex(frame.scenery.sun);
    let mark_h = frame.building_h * 0.34;
    for mark in frame.marks.iter().filter(|mark| mark.grow >= 1.0) {
        let (x, base) = band.mark_at(frame, mark.along);
        let w = mark_h * 0.7;
        if x + w < from || x - w > to {
            continue;
        }
        // A setting of its own draws its own silhouettes (art's, B).
        if crate::works::paint_silhouette(
            canvas,
            frame.setting,
            mark.shape,
            x,
            base,
            w * 1.3,
            mark_h,
            silhouette,
        ) {
            continue;
        }
        crate::ui::paint_mark(
            canvas,
            Bounds::new(
                point(px(x - w / 2.0), px(base - mark_h)),
                size(px(w), px(mark_h)),
            ),
            mark.shape,
            silhouette,
            sun,
        );
    }
    for goal in &frame.goals {
        let (x, base) = band.mark_at(frame, goal.along);
        paint_goal(canvas, x, base, goal, silhouette, sun, frame.setting);
    }
    painter::grain_lit(canvas, at, 0.04, 0.035, frame.light());
    let (gold, strength) = dusk_glaze(frame.hour, frame.look.as_ref().and_then(|look| look.key));
    painter::glaze(&mut canvas.pixmap, gold, strength * 0.8);
}

/// A goal on the ridge: finished, it stands as solid as anything else the
/// World built; under way, a pale outline more solid with each part, with
/// scaffolding as high as it has got and a pip under it for every part.
pub(super) fn paint_goal(
    brush: &mut dyn Brush,
    x: f32,
    base: f32,
    goal: &RidgeGoal,
    silhouette: Hsla,
    light: Hsla,
    setting: art::Setting,
) {
    let (w, h) = (goal.w, goal.h);
    let bounds = Bounds::new(point(px(x - w / 2.0), px(base - h)), size(px(w), px(h)));
    // A setting of its own draws its own silhouettes (art's, B).
    let mark = |brush: &mut dyn Brush, colour: Hsla, light: Hsla| {
        if !crate::works::paint_silhouette(brush, setting, goal.shape, x, base, w, h, colour) {
            crate::ui::paint_mark(brush, bounds, goal.shape, colour, light);
        }
    };
    if goal.done >= goal.parts {
        mark(brush, silhouette, light);
        return;
    }
    let share = goal.done as f32 / goal.parts.max(1) as f32;
    let ghost = gpui::white().opacity(0.16 + 0.4 * share);
    mark(brush, ghost, light.opacity(0.4));
    let wood = art::hex(0x8a6a44).opacity(0.85);
    let left = x - w / 2.0;
    let risen = h * (0.35 + 0.65 * share);
    for pole in 0..3 {
        let px0 = left + w * (0.08 + 0.42 * pole as f32);
        art::line(brush, (px0, base), (px0, base - risen), 1.4, wood);
    }
    let boards = 1 + (share * 3.0) as usize;
    for board in 0..boards {
        let by = base - risen * (board as f32 + 1.0) / (boards as f32 + 0.3);
        art::line(
            brush,
            (left + w * 0.02, by),
            (left + w * 0.98, by),
            1.2,
            wood,
        );
    }
    art::line(
        brush,
        (left + w * 0.08, base),
        (left + w * 0.5, base - risen),
        1.0,
        wood.opacity(0.6),
    );
    let pip = (w * 0.09).clamp(3.0, 6.0);
    let gap = pip * 0.8;
    let row = goal.parts as f32 * pip + (goal.parts as f32 - 1.0) * gap;
    for part in 0..goal.parts {
        let px0 = x - row / 2.0 + part as f32 * (pip + gap);
        let ink = if part < goal.done {
            gpui::white().opacity(0.9)
        } else {
            gpui::white().opacity(0.25)
        };
        brush.rect(px0, base + pip, pip, pip, pip / 2.0, ink);
    }
}

/// The top of the field, in stage pixels, at `x`.
pub(super) fn field_top(frame: &Frame, x: f32) -> f32 {
    let top = frame.horizon + (frame.base - frame.horizon) * 0.3;
    let w = frame.view_w.max(1.0);
    top - 8.0 * (x / (0.9 * w)).sin() - 5.0 * (x / (0.37 * w) + 1.3).sin()
}

/// The top of the foreground (the water's edge in a harbour), at `x`.
pub(super) fn front_top(frame: &Frame, x: f32) -> f32 {
    let w = frame.view_w.max(1.0);
    // A quay's wall runs nearly straight; a shore wanders.
    let wander = if frame.water { 0.25 } else { 1.0 };
    frame.front - wander * (6.0 * (x / (0.7 * w) + 0.4).sin() + 3.0 * (x / (0.23 * w)).sin())
}

/// The land's colours for the season: the field, and the foreground.
pub(super) fn land_colours(frame: &Frame) -> (Hsla, Hsla) {
    let far = art::hex(frame.scenery.far);
    let ground = art::shade(far, 0.16);
    let near = art::hex(frame.scenery.near);
    let (ink, share, near_share) = match frame.cover {
        Some(GroundCover::Snow) => (art::hex(0xf3f6f9), 0.82, 0.7),
        Some(GroundCover::Frost) => (art::hex(0xe9eff3), 0.34, 0.25),
        Some(GroundCover::Leaves) => (art::hex(0xc08a44), 0.16, 0.1),
        Some(GroundCover::Dust) => (art::hex(0xc0704a), 0.22, 0.18),
        Some(GroundCover::Blossom) | None => (ground, 0.0, 0.0),
    };
    let near_ink = if frame.water {
        near
    } else {
        mix(near, ink, near_share)
    };
    // The value bands (A2): the land a step darker than the sky.
    let land = under_sky(&frame.scenery, mix(ground, ink, share), GROUND_UNDER_SKY);
    // Ice held under the sky greys; it stays ice-blue at the same value,
    // so noon has colour against the warm light (v0.29 round 2: a flat
    // grey band under the sky).
    let land = if frame.setting == art::Setting::Ice {
        Hsla {
            h: 0.59,
            s: (land.s + 0.18).min(0.5),
            ..land
        }
    } else {
        land
    };
    (land, under_sky(&frame.scenery, near_ink, GROUND_UNDER_SKY))
}

/// Everything the ground layer depends on: the look, the geometry, the
/// scale it is painted at, and every building standing still on it.
pub(super) fn ground_key(frame: &Frame, scale: f32) -> Key {
    let mut key = Key::new("ground");
    look_key(frame, &mut key);
    for value in [
        frame.width,
        frame.view_w,
        frame.height,
        frame.horizon,
        frame.base,
        frame.front,
        frame.building_h,
        scale,
    ] {
        key.float(value);
    }
    key.add(frame.blend_top);
    // A test's picture without the shadows is another picture.
    key.add(super::works::no_shadows());
    key.add(super::works::no_glow());
    for patch in &frame.patches {
        key.float(patch.x0)
            .float(patch.x1)
            .float(patch.top)
            .float(patch.bottom)
            .add((patch.patch, patch.water, patch.seed));
    }
    for building in frame.buildings.iter().filter(|b| !b.moving()) {
        key.add((building.index, building.shape as u8))
            .float(building.x)
            .float(building.base)
            .float(building.w)
            .float(building.h)
            .colour(building.palette.wall)
            .colour(building.palette.roof)
            .colour(building.palette.glass)
            .add(building.palette.seed)
            .add(building.drawing.as_ref().map(|d| d.id.clone()))
            .add(building.glow.is_some())
            .add((building.flip, building.joins));
        if let Some(glow) = building.glow {
            key.colour(glow);
        }
        for figure in &building.inside {
            key.colour(figure.clothes);
        }
    }
    key
}

/// One tile of the land, `column` and `row` of them from the stage's
/// origin, at `scale` device pixels to a stage pixel, lit and on paper.
pub(super) fn paint_land_tile(
    frame: &Frame,
    column: i32,
    row: i32,
    scale: f32,
) -> Option<sk::Pixmap> {
    let tile = TILE as f32 / scale;
    // A pixel more all round than the tile, so where tiles meet, shown
    // larger than they are painted, the display blends real neighbours.
    let pad = LAND_PAD as f32 / scale;
    let origin = (column as f32 * tile - pad, row as f32 * tile - pad);
    let mut canvas = Canvas::new(TILE + 2 * LAND_PAD, TILE + 2 * LAND_PAD, scale, origin)?;
    let view = (origin.0, origin.0 + tile + pad * 2.0);
    painter::timed("tile: land", || paint_land(&mut canvas, frame, view));
    let light = frame.light();
    painter::timed("tile: paper", || {
        painter::grain_lit(
            &mut canvas,
            (
                column * TILE as i32 - LAND_PAD as i32,
                row * TILE as i32 - LAND_PAD as i32,
            ),
            0.05,
            0.035,
            light,
        )
    });
    let (gold, strength) = dusk_glaze(frame.hour, frame.look.as_ref().and_then(|look| look.key));
    painter::glaze(&mut canvas.pixmap, gold, strength);
    if frame.blend_top {
        // A nearer row of a folded postcard: its field fades in from the
        // top, so the ground runs on from the row behind with no seam.
        let field_y = frame.horizon + (frame.base - frame.horizon) * 0.3;
        let from = canvas.device(0.0, field_y - 14.0).1;
        let to = canvas.device(0.0, field_y + frame.height * 0.06).1;
        super::ground::fade_in_down(&mut canvas.pixmap, from, to);
    } else if frame.setting == art::Setting::Ice {
        // The flat ice runs back into the shelf behind with no straight
        // seam where it begins (v0.29 round 2's line across the ice).
        let field_y = frame.horizon + (frame.base - frame.horizon) * 0.3;
        let k = (frame.height / 848.0).clamp(0.3, 1.3);
        let from = canvas.device(0.0, field_y - 14.0).1;
        let to = canvas.device(0.0, field_y + 26.0 * k).1;
        super::ground::fade_in_down(&mut canvas.pixmap, from, to);
    }
    Some(canvas.pixmap)
}

/// On the ice, a lead of open water along the back of the causeway, so it
/// runs between two waters and reads as a causeway, not a shoreline: one
/// continuous channel of dark water (`Frame::lead`), its far edge soft
/// where the ice thins toward it, its near edge the causeway's lit lip, a
/// few pale glints on it, and the bridge's feet standing in it so the dark
/// shows through the arch (v0.29 round 3: twelve pixels of dark behind the
/// causeway read as its edge, not as water the bridge spans).
fn paint_lead(canvas: &mut Canvas, frame: &Frame, (from, to): (f32, f32), k: f32) {
    let Some((far, near)) = frame.lead() else {
        return;
    };
    let deep = near - far;
    if deep < 3.0 {
        return;
    }
    let water = art::hex(frame.scenery.near);
    let seed = seed_of_scenery(&frame.scenery);
    let step = 14.0 * k;
    let (a, b) = (((from - 60.0) / step).floor(), ((to + 60.0) / step).ceil());
    // The edges wander a little, by the stage position, so tiles meet
    // without a seam and a pan never changes them.
    let wander = |x: f32, side: i32, by: f32| {
        let n = (x / step).round() as i32;
        let wob = (painter::hash2(n, 131 + side, seed) % 100) as f32 / 100.0 - 0.5;
        wob * by
    };
    let band = |top: &dyn Fn(f32) -> f32, bottom: &dyn Fn(f32) -> f32| {
        let mut shape = Shape::new();
        let mut n = a;
        shape.move_to(n * step, top(n * step));
        while n <= b {
            shape.line_to(n * step, top(n * step));
            n += 1.0;
        }
        let mut n = b;
        while n >= a {
            shape.line_to(n * step, bottom(n * step));
            n -= 1.0;
        }
        shape.close();
        shape
    };
    // The far edge in long soft bays, never a saw.
    let phase = (seed % 628) as f32 / 100.0;
    let far_edge = |x: f32| {
        far + deep
            * (0.1 * (x / (61.0 * k) + phase).sin() + 0.05 * (x / (23.0 * k) + phase * 2.0).sin())
    };
    let near_edge = |x: f32| near + wander(x, 1, 1.2 * k);
    // The thinning ice on the far side, soft from the outside in.
    for (reach, opacity) in [(0.42_f32, 0.1_f32), (0.26, 0.2), (0.12, 0.4)] {
        let shape = band(&|x| far_edge(x) - deep * reach, &|x| near_edge(x));
        canvas.fill(&shape, water.opacity(opacity));
    }
    // The open water, darkest along the causeway's foot.
    let open = band(&far_edge, &near_edge);
    painter::fill_shaded(
        canvas,
        &open,
        (0.0, far),
        (0.0, near),
        &[
            (0.0, art::shade(water, 0.08)),
            (0.5, water),
            (1.0, art::shade(water, -0.12)),
        ],
    );
    // The causeway's lip, lit, where its ice meets the water.
    let rim = under_sky(&frame.scenery, art::hex(0xe8f1f7), GROUND_UNDER_SKY);
    let mut lip = Shape::new();
    let mut n = a;
    lip.move_to(n * step, near_edge(n * step) + 0.6 * k);
    while n <= b {
        lip.line_to(n * step, near_edge(n * step) + 0.6 * k);
        n += 1.0;
    }
    canvas.stroke(&lip, 1.6 * k, rim.opacity(0.55));
    // A few pale glints on the water, never a line along it.
    let spacing = 90.0 * k;
    let mut n = ((from - spacing) / spacing).floor() as i32;
    while (n as f32) * spacing < to + spacing {
        let s = painter::hash2(n, 137, seed);
        if !s.is_multiple_of(3) {
            let x = n as f32 * spacing + (s % 1000) as f32 / 1000.0 * spacing * 0.6;
            let y = far + deep * (0.35 + ((s >> 10) % 100) as f32 / 100.0 * 0.4);
            let long = (10.0 + ((s >> 17) % 14) as f32) * k;
            let mut glint = Shape::new();
            glint.move_to(x, y).line_to(x + long, y);
            canvas.stroke(&glint, 1.0 * k, rim.opacity(0.35));
        }
        n += 1;
    }
}

/// The quay: a strip of pale stone along the water in the lower third,
/// where people walk and sit, from the kerb where the street steps down
/// onto it to the water's edge, laid in flagstones where the camera is
/// close enough to see them.
pub(super) fn paint_quay(
    canvas: &mut Canvas,
    frame: &Frame,
    (from, to): (f32, f32),
    detail: bool,
    ground: Hsla,
) {
    let k = (frame.height / 848.0).clamp(0.3, 1.3);
    let top = frame.quay_top();
    // Never lighter than the sky allows the land (the art bible's value
    // bands), however pale the Pack's spine.
    let stone = under_sky(
        &frame.scenery,
        frame
            .spine()
            .unwrap_or(crate::setting::quay_inks(frame.setting, ground).0),
        GROUND_UNDER_SKY,
    );
    let step = 24.0;
    let mut deck = Shape::new();
    deck.move_to(from, top);
    deck.line_to(to + step, top);
    let mut x = to + step;
    while x > from - step {
        deck.line_to(x, front_top(frame, x) + 2.0);
        x -= step;
    }
    deck.close();
    painter::fill_shaded(
        canvas,
        &deck,
        (0.0, top),
        (0.0, frame.front),
        &[
            (0.0, art::shade(stone, 0.05)),
            (1.0, art::shade(stone, -0.07)),
        ],
    );
    // The kerb: a lip of darker stone, and its shadow on the quay.
    canvas.gradient(
        from,
        top,
        to - from + step,
        5.0 * k,
        180.0,
        (art::shade(stone, -0.3).opacity(0.35), 0.0),
        (art::shade(stone, -0.3).opacity(0.0), 1.0),
    );
    let mut kerb = Shape::new();
    kerb.move_to(from, top).line_to(to + step, top);
    canvas.stroke(&kerb, 1.4 * k, art::shade(stone, -0.4).opacity(0.45));
    if frame.setting == art::Setting::Ice {
        paint_lead(canvas, frame, (from, to), k);
    }
    if !detail {
        return;
    }
    // Flagstones: long courses along the quay, each joint offset from the
    // one before, drawn faintly, as a painter would suggest them.
    let joint = art::shade(stone, -0.28).opacity(0.22);
    let deep = frame.front - top;
    let courses = [0.0, 0.3, 0.62, 1.0];
    for pair in 0..courses.len() - 1 {
        let (y0, y1) = (top + deep * courses[pair], top + deep * courses[pair + 1]);
        if pair > 0 {
            let mut line = Shape::new();
            line.move_to(from, y0).line_to(to + step, y0);
            canvas.stroke(&line, 0.8 * k, joint);
        }
        let long = (46.0 + 12.0 * pair as f32) * k;
        let offset = pair as f32 * long * 0.45;
        let mut x = ((from - offset) / long).floor() * long + offset;
        while x < to + long {
            let seed = painter::hash2((x / long) as i32, pair as i32, 0x9a7);
            let wobble = (seed % 9) as f32 - 4.0;
            let mut cut = Shape::new();
            cut.move_to(x + wobble * k, y0 + 1.5)
                .line_to(x + wobble * k, (y1 - 1.5).min(frame.front - 3.0));
            canvas.stroke(&cut, 0.8 * k, joint);
            x += long;
        }
    }
}

/// The quay's wall where it goes down into the water: a band of darker
/// stone under the coping, and its shadow on the water.
pub(super) fn paint_quay_wall(
    canvas: &mut Canvas,
    frame: &Frame,
    (from, to): (f32, f32),
    ground: Hsla,
) {
    let k = (frame.height / 848.0).clamp(0.3, 1.3);
    let stone = crate::setting::quay_inks(frame.setting, ground).1;
    let wall = frame.figure_h * 0.28;
    let step = 24.0;
    let mut face = Shape::new();
    face.move_to(from - step, front_top(frame, from - step));
    let mut x = from - step;
    while x < to + step {
        x += step;
        face.line_to(x, front_top(frame, x));
    }
    while x > from - step {
        face.line_to(x, front_top(frame, x) + wall);
        x -= step;
    }
    face.close();
    painter::fill_shaded(
        canvas,
        &face,
        (0.0, frame.front),
        (0.0, frame.front + wall),
        &[
            (0.0, art::shade(stone, -0.08)),
            (1.0, art::shade(stone, -0.3)),
        ],
    );
    canvas.gradient(
        from - step,
        frame.front + wall - 1.0,
        to - from + step * 2.0,
        wall * 0.8,
        180.0,
        (gpui::black().opacity(0.16), 0.0),
        (gpui::black().opacity(0.0), 1.0),
    );
    // The coping's lit edge.
    let mut coping = Shape::new();
    coping.move_to(from - step, front_top(frame, from - step));
    let mut x = from - step;
    while x < to + step {
        x += step;
        coping.line_to(x, front_top(frame, x));
    }
    canvas.stroke(&coping, 1.6 * k, gpui::white().opacity(0.35));
    // On the ice the shelf's edge hangs with icicles (art's, B).
    crate::setting::paint_shelf_edge(
        canvas,
        frame.setting,
        (from, to),
        &|x| front_top(frame, x),
        wall,
        k,
        seed_of_scenery(&frame.scenery),
    );
}

/// The field, the path worn along it, what grows and lies about on it, the
/// foreground or the water, and what the season lays over them, for stage
/// `x` from `view.0` to `view.1`.
pub(super) fn paint_land(canvas: &mut Canvas, frame: &Frame, view: (f32, f32)) {
    let (ground, near) = land_colours(frame);
    let bottom = frame.height + TILE as f32 / canvas.scale + 40.0;
    let (from, to) = (view.0 - 60.0, view.1 + 60.0);
    let step = 24.0;
    let edge = |top: &dyn Fn(f32) -> f32| {
        let mut shape = Shape::new();
        shape.move_to(from, bottom);
        let mut x = from;
        while x < to + step {
            shape.line_to(x, top(x));
            x += step;
        }
        shape.line_to(to + step, bottom).close();
        shape
    };
    // The field, lighter toward the hills with the air between.
    let haze = frame.haze();
    let field = edge(&|x| field_top(frame, x));
    let field_y = frame.horizon + (frame.base - frame.horizon) * 0.3;
    canvas.fill(&field, ground);
    // The field paler toward the hills, with the air between.
    let (tile_top, tile_bottom) = (
        canvas.origin.1,
        canvas.origin.1 + canvas.height() as f32 / canvas.scale,
    );
    let haze_to = field_y + (frame.base - field_y) * 0.9;
    if tile_top < haze_to && tile_bottom > field_y - 30.0 {
        let mut hazed = Canvas::new(canvas.width(), canvas.height(), canvas.scale, canvas.origin)
            .expect("a tile");
        // On the flat ice the far edge of the field melts into the air:
        // no straight seam where it meets the shelf behind (v0.29 round 2).
        let veil = if frame.setting == art::Setting::Ice {
            0.45
        } else {
            0.1
        };
        hazed.fill(&field, mix(ground, haze, veil));
        painter::fade_down(
            &mut hazed.pixmap,
            canvas.device(0.0, field_y - 20.0).1,
            canvas.device(0.0, haze_to).1,
        );
        canvas.draw(
            &hazed.pixmap,
            canvas.origin.0,
            canvas.origin.1,
            sk::BlendMode::SourceOver,
        );
    }
    if !frame.blend_top && frame.setting != art::Setting::Ice {
        let mut rim = Shape::new();
        let mut x = from;
        rim.move_to(x, field_top(frame, x));
        while x < to + step {
            x += step;
            rim.line_to(x, field_top(frame, x));
        }
        canvas.stroke(&rim, 1.2, art::shade(ground, -0.3).opacity(0.25));
    }
    let k = (frame.height / 848.0).clamp(0.3, 1.3);
    let detail = frame.height >= 360.0;
    // The clusters' patches, the paths down to the spine, and the ground
    // meant between them (ground.rs).
    super::ground::paint_plan(canvas, frame, (from, to), ground, k);
    let front = frame.front;
    // Grass and flowers grow on the street's verges down to the quay, or
    // on dry ground down to the foreground.
    let meadow = if frame.water { frame.quay_top() } else { front };
    let strip_top = frame.base + (meadow - frame.base) * 0.4;
    let w = frame.view_w.max(1.0);
    if frame.water {
        paint_quay(canvas, frame, (from, to), detail, ground);
    } else if detail {
        // A worn path winds along the near ground, where people walk.
        let mid = frame.feet;
        let wind = |x: f32| mid + 9.0 * k * (x / (0.55 * w) + 0.7).sin();
        let mut path = Shape::new();
        let mut x = from;
        path.move_to(x, wind(x) - 7.0 * k);
        while x < to + step {
            x += step;
            path.line_to(x, wind(x) - 7.0 * k);
        }
        while x > from {
            path.line_to(x, wind(x) + 9.0 * k);
            x -= step;
        }
        path.close();
        canvas.fill(&path, art::shade(ground, 0.1).opacity(0.7));
    }
    // The foreground: water deepening toward the front, or near ground.
    let shore = edge(&|x| front_top(frame, x));
    if frame.water {
        painter::fill_shaded(
            canvas,
            &shore,
            (0.0, front - 10.0),
            (0.0, frame.height),
            &[
                (0.0, mix(near, haze, 0.28)),
                (0.25, near),
                (1.0, art::shade(near, -0.18)),
            ],
        );
    } else {
        painter::fill_shaded(
            canvas,
            &shore,
            (0.0, front - 10.0),
            (0.0, frame.height),
            &[(0.0, near), (1.0, art::shade(near, -0.1))],
        );
    }
    let mut line = Shape::new();
    let mut x = from;
    line.move_to(x, front_top(frame, x));
    while x < to + step {
        x += step;
        line.line_to(x, front_top(frame, x));
    }
    canvas.stroke(
        &line,
        1.4,
        if frame.water {
            gpui::white().opacity(0.35)
        } else {
            art::shade(near, -0.3).opacity(0.4)
        },
    );
    if frame.water {
        paint_quay_wall(canvas, frame, (from, to), ground);
    }
    // What stands on the water line stands on a jetty or a spit (ground.rs).
    super::ground::paint_footings(
        canvas,
        frame,
        (from, to),
        (frame.height / 848.0).clamp(0.3, 1.3),
    );
    // A frozen edge to the harbour in deep winter.
    if frame.ice && frame.water {
        let deep = (frame.height - front) * 0.16;
        let ice = edge(&|x| front_top(frame, x) - 1.0);
        let mut sheet = Shape::new();
        let mut x = from;
        sheet.move_to(x, front_top(frame, x) - 1.0);
        while x < to + step {
            x += step;
            sheet.line_to(x, front_top(frame, x) - 1.0);
        }
        while x > from {
            let lip = deep * (0.75 + 0.25 * (x / (0.11 * w)).sin());
            sheet.line_to(x, front_top(frame, x) + lip);
            x -= step;
        }
        sheet.close();
        let _ = ice;
        canvas.fill(&sheet, art::hex(0xe4eef3).opacity(0.92));
        for index in 0..((frame.width / w * 7.0) as i32) {
            let seed = painter::hash2(index, 3, 0x1ce);
            let x = (seed % 10_000) as f32 / 10_000.0 * frame.width;
            if x < from || x > to {
                continue;
            }
            let y = front_top(frame, x) + deep * 0.3;
            let mut crack = Shape::new();
            crack
                .move_to(x, y)
                .line_to(x + 6.0 * k, y + deep * 0.15)
                .line_to(x + 4.0 * k, y + deep * 0.3);
            canvas.stroke(&crack, 0.8, art::hex(0xb4c6d2).opacity(0.55));
        }
    }
    if !detail {
        return;
    }
    // What grows and lies about along the strip, in the colours of the
    // ground it is on; the same place always has the same ones, and each
    // kind of place its own (art's, B: setting.rs).
    let seed0 = seed_of_scenery(&frame.scenery);
    let per = frame.width / w;
    let scatter = |index: i32, salt: u32| {
        let seed = painter::hash2(index, salt as i32, seed0);
        (
            (seed % 10_000) as f32 / 10_000.0 * frame.width,
            ((seed / 10_000) % 1000) as f32 / 1000.0,
            seed,
        )
    };
    let dressing = crate::setting::Ground {
        setting: frame.setting,
        from,
        to,
        strip_top,
        meadow,
        front,
        height: frame.height,
        width: frame.width,
        view_w: frame.view_w,
        k,
        seed: seed0,
        ground,
        near,
        cover: frame.cover,
        water: frame.water,
        spine: frame.spine(),
    };
    crate::setting::paint_ground_props(canvas, &dressing);
    crate::setting::paint_sea_ice(canvas, &dressing);
    // What the season lays on the ground.
    let (count, inks): (f32, &[u32]) = match frame.cover {
        Some(GroundCover::Leaves) => (60.0, &[0xc8642e, 0xd9913a, 0xa8452c, 0xe0b04a]),
        Some(GroundCover::Blossom) => (36.0, &[0xf6c9d8, 0xfbe3ea, 0xf0b3c8]),
        Some(GroundCover::Frost) => (60.0, &[0xffffff]),
        Some(GroundCover::Snow) => (26.0, &[0xffffff]),
        Some(GroundCover::Dust) => (22.0, &[0xb85a36]),
        None => (0.0, &[]),
    };
    for index in 0..((count * per) as i32) {
        let (x, t, seed) = scatter(index, 6);
        if x < from || x > to {
            continue;
        }
        let y = field_top(frame, x) + 6.0 + t * (front - field_top(frame, x) - 4.0);
        let ink = art::hex(inks[(seed >> 12) as usize % inks.len()]);
        match frame.cover {
            Some(GroundCover::Snow) if frame.setting == art::Setting::Ice => {
                // On the ice the snow is the ground itself: only long, low
                // wind streaks, never white ovals that read as puddles
                // (v0.29 round 2's noon on the ice).
                canvas.soft(x, y, 44.0 * k, 1.4 * k, 2.5 * k, ink.opacity(0.28));
                canvas.soft(
                    x + 8.0 * k,
                    y + 2.0 * k,
                    36.0 * k,
                    1.0 * k,
                    2.0 * k,
                    art::hex(0x9fbdd6).opacity(0.3),
                );
            }
            Some(GroundCover::Snow) => {
                // Soft drifts, pale blue on their shaded side.
                canvas.soft(x, y, 26.0 * k, 5.0 * k, 6.0 * k, ink.opacity(0.7));
                canvas.soft(
                    x + 6.0 * k,
                    y + 3.0 * k,
                    20.0 * k,
                    3.0 * k,
                    4.0 * k,
                    art::hex(0xc8d6e4).opacity(0.5),
                );
            }
            Some(GroundCover::Dust) => {
                canvas.soft(x, y, 34.0 * k, 3.0 * k, 5.0 * k, ink.opacity(0.35));
            }
            Some(GroundCover::Frost) => {
                art::circle(canvas, x, y, 0.9 * k, ink.opacity(0.7));
            }
            _ => {
                let turn = ((seed >> 4) % 100) as f32 / 100.0;
                let (rx, ry) = (2.6 * k * (0.6 + 0.4 * turn), 1.5 * k);
                art::ellipse(canvas, x, y, rx, ry, ink.opacity(0.9));
            }
        }
    }
}
