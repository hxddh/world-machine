//! A town laid out along its panorama: the rows things stand in, the ids
//! homes and works go by, and the steps every Pack takes to put its
//! anchors, homes, finished works, plotted works and fixtures in slots and
//! give everyone a day.
//!
//! A Pack says what is its own: its stretches and their names, where each
//! household's home goes, its catalog of works and which stretch each
//! stands on, and where each person spends the day. The steps are the same
//! in every World and are taken in the same order, so the same World always
//! lays out the same way.

use crate::{Plan, Row, Street, Stretch};
use std::collections::{BTreeMap, BTreeSet};
use world_core::{EntityId, Event, Value};
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasLink, CanvasProjection, Cluster, District, Ground,
    GroundCover, MarkShape, RoutineStop, Season, SelectionId,
};

/// Houses at the back.
pub const BACK: usize = 0;
/// Works in the middle.
pub const MIDDLE: usize = 1;
pub const NEARER: usize = 2;
/// Small things at the front.
pub const FRONT: usize = 3;
/// The plots the player can build on, which nothing else takes.
pub const PLOTS: usize = 4;
/// The water line: piers, slipways, boathouses and the lighthouse on the
/// point, standing at the water's edge (or the crater rim, the kerb, the
/// floe edge: whatever a place has for one).
pub const WATER: usize = 5;
/// How far down the scene the water line is, for an app that knows one
/// screen only: past the quay (0.76), where the app draws things at the
/// water's edge.
pub const WATER_Y: f32 = 0.86;
/// The water line's slots for a tall building (a boathouse, the
/// lighthouse): in front of the gaps between pairs of plots, so it never
/// hides what the player built behind it.
pub const WATER_TALL: usize = 6;

/// Where a thing belongs, from the water to the hills (the art bible's
/// siting zones): a pier never stands on a hill. The zones, and which
/// library drawing stands in which, are the art catalog's (`world-art`).
pub use world_art::Zone;

/// The rows a thing of `zone` may stand in, the likeliest first.
pub fn rows_of(zone: Zone) -> &'static [usize] {
    match zone {
        Zone::Water => &[WATER],
        Zone::Quay => &[FRONT, NEARER],
        Zone::Lanes => &[MIDDLE, NEARER, BACK],
        Zone::Green => &[NEARER, MIDDLE, FRONT],
        Zone::Edge => &[MIDDLE, BACK, NEARER],
    }
}

/// Where a thing drawn as `art` (a key of the app's library of drawings),
/// or else as `shape`, belongs. Every Pack's drawings are sited by the
/// same rules, so a pier is on the water wherever it is built.
pub fn zone_of(art: Option<&str>, shape: MarkShape) -> Zone {
    if let Some(zone) = art.and_then(world_art::zone) {
        return zone;
    }
    match shape {
        MarkShape::Pier | MarkShape::Boat => Zone::Water,
        MarkShape::Bench
        | MarkShape::Lantern
        | MarkShape::Lamp
        | MarkShape::Stall
        | MarkShape::Parcel
        | MarkShape::Rover
        | MarkShape::Postbox
        | MarkShape::Signpost
        | MarkShape::Flag
        | MarkShape::Planter => Zone::Quay,
        MarkShape::Fountain
        | MarkShape::Well
        | MarkShape::Swing
        | MarkShape::Statue
        | MarkShape::Bunting
        | MarkShape::Tent => Zone::Green,
        MarkShape::Tree | MarkShape::Garden | MarkShape::Sprouts | MarkShape::Birdhouse => {
            Zone::Edge
        }
        MarkShape::House
        | MarkShape::Shop
        | MarkShape::Tower
        | MarkShape::Dome
        | MarkShape::Bridge => Zone::Lanes,
    }
}

/// The mark a storylet leaves on the scene for a few days after it ends:
/// bunting after a fete, a boat rack after a storm warning, scaffolding
/// on a chimney after a fire. Read from the recorded Event that ended it,
/// never recorded itself, and gone once its days are up.
#[derive(Clone, Copy, Debug)]
pub struct Trace {
    /// The storylet, or with a trailing `*` every storylet whose id starts
    /// so (`birthday_*`).
    pub storylet: &'static str,
    /// What it is drawn as: a key of the app's library, and its shape.
    pub art: Option<&'static str>,
    pub shape: MarkShape,
    /// Where it stands: beside a place, or at the asker's home.
    pub at: TraceAt,
    /// For how many days.
    pub days: u64,
}

/// Where a storylet's mark stands.
#[derive(Clone, Copy, Debug)]
pub enum TraceAt {
    Place(EntityId),
    /// The home of whoever's storylet it was.
    Asker,
    /// The site of a work of the catalog: materials for it.
    Site(&'static str),
    /// The site of the work the storylet builds, by its id after `work_`:
    /// a part of a work done leaves its materials there.
    Works,
}

impl Trace {
    fn matches(&self, storylet: &str) -> bool {
        match self.storylet.strip_suffix('*') {
            Some(prefix) => storylet.starts_with(prefix),
            None => storylet == self.storylet,
        }
    }
}

/// The marks the storylets that ended in the last few days leave (`table`
/// says which do, and how), as unplaced things beside where they belong,
/// for [`Town::fixtures`] to stand: each the Event that ended it, so
/// choosing it opens that moment. `events` is the World's history, `now`
/// its time and `day` how long a day is; `home_of` finds a person's home
/// and `title` what an Event is called. The newest of each storylet only.
pub fn traces(
    events: &[Event],
    now: u64,
    day: u64,
    table: &[Trace],
    home_of: impl Fn(EntityId) -> Option<SelectionId>,
    site_of: impl Fn(&str) -> Option<SelectionId>,
    title: impl Fn(&Event) -> Option<String>,
) -> Vec<CanvasItem> {
    let longest = table.iter().map(|trace| trace.days).max().unwrap_or(0);
    let since = now.saturating_sub(longest * day.max(1));
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for event in events.iter().rev() {
        if event.world_time < since {
            break;
        }
        if event.kind == "situation_arose" {
            continue;
        }
        let Some(Value::Text(storylet)) = event.payload.get("storylet") else {
            continue;
        };
        let Some(trace) = table.iter().find(|trace| trace.matches(storylet)) else {
            continue;
        };
        if event.world_time + trace.days * day.max(1) <= now || !seen.insert(trace.storylet) {
            continue;
        }
        let at = match trace.at {
            TraceAt::Place(place) => Some(SelectionId::Entity(place)),
            TraceAt::Asker => event.actor.and_then(&home_of),
            TraceAt::Site(work) => site_of(work),
            TraceAt::Works => storylet.strip_prefix("work_").and_then(&site_of),
        };
        let Some(at) = at else {
            continue;
        };
        let Some(label) = title(event) else {
            continue;
        };
        out.push(CanvasItem {
            id: SelectionId::Event(event.id),
            kind: CanvasItemKind::Object,
            label,
            shape: Some(trace.shape),
            art: trace.art.map(Into::into),
            at: Some(at),
            y: 0.76,
            ..Default::default()
        });
    }
    out.reverse();
    out
}

/// How far apart, in screens, two of a cluster's homes or works may stand
/// and still share its patch of ground.
const APART: f32 = 0.34;

/// The most homes and works one cluster holds: past it, the next of its
/// kind takes them (a cluster is a few things sharing a patch of ground).
pub const MOST_IN_A_CLUSTER: usize = 6;

/// Whether a work drawn as `art` stands wider than one slot of its row:
/// a glasshouse, a village hall, a row of cottages.
fn wide(art: Option<&str>) -> bool {
    const WIDE: &[&str] = &[
        "glasshouse",
        "greenhouse",
        "village-hall",
        "row-cottages",
        "net-store",
        "fish-market",
        "boat-yard",
        "sea-wall",
        "orchard",
        "allotments",
        "garden-plots",
        "quilting-room",
        "weaving-shed",
        "bathing-huts",
        "picnic-tables",
        "herring-shed",
        "school-garden",
        "duck-pond",
        "frog-pond",
        "solar-field",
        "solar-array",
        "algae-farm",
        "basketball-court",
        "drive-in-screen",
        "bleachers",
    ];
    art.is_some_and(|art| WIDE.contains(&art))
}

/// A cluster a Pack lays its place out in: a few works or homes sharing a
/// patch of ground near `at`, with a name the story can use.
#[derive(Clone, Copy, Debug)]
pub struct Quarter {
    pub id: &'static str,
    pub label: &'static str,
    /// Its middle along the panorama, in screens.
    pub at: f32,
    /// The zones of the works it holds.
    pub zones: &'static [Zone],
    /// Whether the homes of its stretch stand in it.
    pub homes: bool,
    /// Its ground, as the app paints it.
    pub ground: Ground,
}

/// How far along the ground what stands in front of or behind a work on a
/// plot keeps from it.
const CLEAR: f32 = 0.08;

/// The ids homes and works go by on the scene. Neither is a thing a World
/// records, so they take ids no entity ever has: the ranges are documented
/// and checked in [`world_projection::derived`].
pub use world_projection::derived::{home_id, is_home, work_id, HOME_IDS, WORK_IDS};

/// Whether a shape is a building someone could go into.
pub fn is_building(shape: MarkShape) -> bool {
    matches!(
        shape,
        MarkShape::House | MarkShape::Shop | MarkShape::Tower | MarkShape::Dome
    )
}

/// Where a Pack's plots stand: the row's pitch and offset along the
/// ground, and how far down the scene they are.
#[derive(Clone, Copy, Debug)]
pub struct PlotRow {
    pub pitch: f32,
    pub offset: f32,
    pub y: f32,
}

/// Where plots stand in every World so far.
pub const PLOT_ROW_AT: PlotRow = PlotRow {
    pitch: 0.16,
    offset: 0.02,
    y: 0.68,
};

/// A work in a Pack's catalog, as the scene draws it.
pub struct CatalogWork<'a> {
    pub id: &'a str,
    pub label: String,
    pub shape: MarkShape,
    /// The stretch it stands on.
    pub stretch: usize,
    /// Whether it is going up now: it stands on its site in scaffolding.
    pub under_way: bool,
}

/// A town being laid out.
pub struct Town {
    pub street: Street,
    pub width: f32,
    /// How far down the scene each row stands, for an app that knows one
    /// screen only.
    pub row_y: [f32; 7],
    /// Where everything placed so far stands along the ground.
    pub placed: BTreeMap<SelectionId, f32>,
    /// The clusters the Pack lays its place out in.
    pub quarters: Vec<Quarter>,
    /// Which cluster each home and work stands in, by the cluster's place
    /// in `quarters`.
    pub members: BTreeMap<SelectionId, usize>,
    /// Where everything stands so far (row, place, whether a building), so
    /// no building put up after stands in front of a smaller thing, and no
    /// smaller thing just behind a building.
    stood: Vec<(usize, f32, bool)>,
}

impl Town {
    pub fn new(width: f32, stretches: &[Stretch], plots: PlotRow) -> Self {
        // Houses at the back, works in the middle, and small things at the
        // front. No two rows share a spot along the ground.
        let rows = [
            Row {
                pitch: 0.16,
                offset: 0.0,
            },
            Row {
                pitch: 0.16,
                offset: 0.08,
            },
            Row {
                pitch: 0.16,
                offset: 0.04,
            },
            Row {
                pitch: 0.08,
                offset: 0.02,
            },
            Row {
                pitch: plots.pitch,
                offset: plots.offset,
            },
            // The water line, its slots between every other row's.
            Row {
                pitch: 0.16,
                offset: 0.10,
            },
            // The water line for tall buildings, in front of the gaps
            // between pairs of plots (every third plot slot is a path).
            Row {
                pitch: plots.pitch * 3.0,
                offset: plots.offset + plots.pitch,
            },
        ];
        Self {
            street: Street::new(width, stretches, &rows),
            width,
            row_y: [0.3, 0.45, 0.6, 0.76, plots.y, WATER_Y, WATER_Y],
            placed: BTreeMap::new(),
            quarters: Vec::new(),
            members: BTreeMap::new(),
            stood: Vec::new(),
        }
    }

    /// The clusters the place is laid out in: homes and works go to the
    /// one of their stretch that takes their kind, nearest its middle, so
    /// they stand together on one patch of ground.
    pub fn quarters(mut self, quarters: &[Quarter]) -> Self {
        self.quarters = quarters.to_vec();
        self
    }

    /// The cluster of `stretch` that takes `zone` (or homes), if any.
    fn quarter_for(&self, stretch: usize, zone: Option<Zone>) -> Option<usize> {
        let stretch = self.street.stretches().get(stretch).copied()?;
        let takes = |at: usize, quarter: &Quarter| {
            let room = self
                .members
                .values()
                .filter(|member| **member == at)
                .count()
                < MOST_IN_A_CLUSTER;
            room && match zone {
                Some(zone) => quarter.zones.contains(&zone),
                None => quarter.homes,
            }
        };
        self.quarters
            .iter()
            .enumerate()
            .position(|(at, quarter)| stretch.holds(quarter.at) && takes(at, quarter))
            .or_else(|| {
                // A kind its own stretch has no cluster with room for
                // stands in the nearest cluster that takes it.
                let middle = (stretch.from + stretch.to) / 2.0;
                self.quarters
                    .iter()
                    .enumerate()
                    .filter(|(at, quarter)| takes(*at, quarter))
                    .min_by(|(_, a), (_, b)| {
                        (a.at - middle).abs().total_cmp(&(b.at - middle).abs())
                    })
                    .map(|(at, _)| at)
                    .filter(|_| !matches!(zone, None | Some(Zone::Lanes)))
            })
    }

    /// Which of `rows` has the free slot nearest `near` in `stretch`.
    fn nearest_of(&self, rows: &[usize], stretch: usize, near: f32) -> Option<usize> {
        rows.iter()
            .filter_map(|&row| {
                let mut trial = self.street.clone();
                let px = trial.take(row, Some(stretch), near)?;
                self.street
                    .stretches()
                    .get(stretch)
                    .is_some_and(|s| s.holds(px))
                    .then_some((row, (px - near).abs()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(row, _)| row)
    }

    /// Takes a slot as [`Town::take_spaced`] does, where nothing hides or
    /// is hidden: a `building` not in front of a smaller thing just behind
    /// it, a smaller thing not just behind a building. Anywhere it can
    /// when there is no such slot.
    fn take_unhidden(
        &mut self,
        rows: &[usize],
        within: Option<usize>,
        near: f32,
        building: bool,
    ) -> Option<(usize, f32)> {
        let row_y = self.row_y;
        let stood = &self.stood;
        let clear = |row: usize| {
            move |px: f32| {
                !stood.iter().any(|(r, x, big)| {
                    (x - px).abs() < 0.12
                        && if building {
                            !big && row_y[*r] < row_y[row]
                        } else {
                            *big && row_y[*r] > row_y[row]
                        }
                })
            }
        };
        let found = rows.iter().find_map(|&row| {
            let mut trial = self.street.clone();
            trial
                .take_where(row, within, near, clear(row))
                .map(|px| (row, px))
        });
        let taken = match found {
            Some((row, px)) => {
                self.street
                    .take_where(row, None, px, |x| (x - px).abs() < 1e-4);
                self.space(row, px);
                Some((row, px))
            }
            None => self.take_spaced(rows, within, near),
        };
        if let Some((row, px)) = taken {
            self.stood.push((row, px, building));
        }
        taken
    }

    /// Takes a slot in the first of `rows` with room (in `within` first),
    /// keeping its neighbours on the quay and the water line clear: what
    /// stands there is wider than one of their slots, and a tall building
    /// on the water keeps the slots beside it clear too.
    pub fn take_spaced(
        &mut self,
        rows: &[usize],
        within: Option<usize>,
        near: f32,
    ) -> Option<(usize, f32)> {
        let (row, px) = self.street.take_first(rows, within, near)?;
        self.space(row, px);
        Some((row, px))
    }

    /// Keeps the slots around what stands at `px` in `row` clear, on the
    /// quay and the water line.
    fn space(&mut self, row: usize, px: f32) {
        match row {
            FRONT => {
                self.street.keep_clear(FRONT, px, 0.09);
            }
            WATER => {
                self.street.keep_clear(WATER, px, 0.17);
                self.street.keep_clear(WATER_TALL, px, 0.2);
            }
            WATER_TALL => {
                self.street.keep_clear(WATER, px, 0.17);
                // Nothing on the quay, or just behind it, stands hidden
                // behind it.
                self.street.keep_clear(FRONT, px, 0.12);
                self.street.keep_clear(NEARER, px, 0.1);
                self.street.keep_clear(MIDDLE, px, 0.08);
            }
            _ => {}
        }
    }

    /// Counts what stands at `id` as one of a cluster's own.
    pub fn join(&mut self, quarter: &str, id: SelectionId) {
        if let Some(at) = self.quarters.iter().position(|q| q.id == quarter) {
            self.members.insert(id, at);
        }
    }

    /// A new item standing at `px` in `row`.
    #[allow(clippy::too_many_arguments)]
    pub fn item(
        &self,
        id: SelectionId,
        kind: CanvasItemKind,
        label: String,
        detail: String,
        px: f32,
        row: usize,
        shape: MarkShape,
        built: Option<u32>,
    ) -> CanvasItem {
        CanvasItem {
            id,
            kind,
            label,
            detail,
            x: px / self.width,
            y: self.row_y[row],
            changes: Vec::new(),
            shape: Some(shape),
            at: None,
            look: None,
            drawing: None,
            stance: None,
            standing: None,
            mood: None,
            spot: None,
            px: Some(px),
            home: None,
            day: Vec::new(),
            built,
            ..Default::default()
        }
    }

    /// Stands an item already on the scene in a slot of `row` near `px`.
    /// Where the row has no room, it stands at `px` all the same if
    /// `anyway`, and is otherwise left where it was.
    pub fn stand(
        &mut self,
        items: &mut [CanvasItem],
        id: EntityId,
        row: impl Fn(&CanvasItem) -> usize,
        px: f32,
        anyway: bool,
    ) {
        let selection = SelectionId::Entity(id);
        if let Some(item) = items.iter_mut().find(|item| item.id == selection) {
            let row = row(item);
            let taken = self.street.take(row, self.street.stretch_at(px), px);
            if let Some(px) = taken {
                self.space(row, px);
                self.stood
                    .push((row, px, item.kind == CanvasItemKind::Place));
            }
            if let Some(px) = taken.or(anyway.then_some(px)) {
                item.px = Some(px);
                // The row it stands in, for the app to stand it in.
                item.y = self.row_y[row];
                self.placed.insert(selection, px);
            }
        }
    }

    /// A home for every household, on the stretch `stretch_of` its first
    /// member says, labelled `label` and named for whoever lives there.
    /// Who lives in which home.
    pub fn homes(
        &mut self,
        items: &mut Vec<CanvasItem>,
        households: &BTreeMap<EntityId, Vec<EntityId>>,
        stretch_of: impl Fn(EntityId) -> usize,
        label: &str,
        name_of: impl Fn(EntityId) -> String,
        art: Option<&str>,
    ) -> BTreeMap<EntityId, SelectionId> {
        let mut home_of = BTreeMap::new();
        for (founder, members) in households {
            let stretch = stretch_of(*founder);
            let quarter = self.quarter_for(stretch, None);
            let near = quarter
                .map(|at| self.quarters[at].at)
                .unwrap_or_else(|| self.street.spread(stretch, founder.0));
            // In a cluster, homes stand close in two staggered rows, the
            // nearer slot of either row first: a lane, not a line.
            let staggered = quarter.and_then(|_| self.nearest_of(&[BACK, MIDDLE], stretch, near));
            let taken = match staggered {
                Some(row) => self
                    .street
                    .take(row, Some(stretch), near)
                    .map(|px| (row, px)),
                None => self.street.take_first(&[BACK, MIDDLE], Some(stretch), near),
            };
            let Some((row, px)) = taken else {
                continue;
            };
            self.stood.push((row, px, true));
            let id = home_id(*founder);
            if let Some(quarter) = quarter {
                self.members.insert(id, quarter);
            }
            for member in members {
                home_of.insert(*member, id);
            }
            self.placed.insert(id, px);
            let mut item = self.item(
                id,
                CanvasItemKind::Place,
                label.into(),
                members
                    .iter()
                    .map(|member| name_of(*member))
                    .collect::<Vec<_>>()
                    .join(" · "),
                px,
                row,
                MarkShape::House,
                None,
            );
            item.art = art.map(Into::into);
            items.push(item);
        }
        home_of
    }

    /// The finished works of the catalog (`done`, by their place in the
    /// catalog, with the period each was finished if known), and those
    /// going up, each in its cluster: the oldest first, nearest the
    /// cluster's middle, so what is built never moves for what is built
    /// after it and a cluster grows outward. A work going up stands on its
    /// site in scaffolding. What stands on each stretch.
    pub fn works<'a>(
        &mut self,
        items: &mut Vec<CanvasItem>,
        catalog: impl IntoIterator<Item = CatalogWork<'a>>,
        done: &BTreeMap<usize, Option<u32>>,
        art: impl Fn(&str) -> Option<String>,
    ) -> [Vec<SelectionId>; 3] {
        let mut works: [Vec<SelectionId>; 3] = Default::default();
        let mut standing = catalog
            .into_iter()
            .enumerate()
            .filter_map(|(index, work)| {
                let finished = done.get(&index).copied();
                (finished.is_some() || work.under_way).then_some((index, work, finished))
            })
            .collect::<Vec<_>>();
        // Finished before going up; the oldest first, those whose day the
        // World no longer holds first of all.
        // The water line is sited first, so its tall buildings keep the
        // quay in front of them clear.
        let on_water =
            |work: &CatalogWork| zone_of(art(work.id).as_deref(), work.shape) == Zone::Water;
        standing.sort_by_key(|(index, work, finished)| {
            let first = !on_water(work);
            match finished {
                Some(day) => (first, 0, day.unwrap_or(0), *index),
                None => (first, 1, 0, *index),
            }
        });
        for (index, work, finished) in standing {
            let art = art(work.id);
            let zone = zone_of(art.as_deref(), work.shape);
            let quarter = self.quarter_for(work.stretch, Some(zone));
            let near = quarter
                .map(|at| self.quarters[at].at)
                .unwrap_or_else(|| self.street.spread(work.stretch, index as u64));
            let within = quarter
                .and_then(|at| self.street.stretch_at(self.quarters[at].at))
                .or(Some(work.stretch));
            // A tall building on the water stands where it hides no plot.
            // A building on the quay stands at its back, by the plots: the
            // quay itself is for people, stalls and benches.
            let rows: &[usize] = match (zone, is_building(work.shape)) {
                (Zone::Water, true) => &[WATER_TALL, WATER],
                (Zone::Quay, true) => &[NEARER, MIDDLE, BACK],
                _ => rows_of(zone),
            };
            // A building never stands in front of a smaller thing already
            // standing just behind it, nor a smaller thing just behind a
            // building: what is nearer would hide it.
            let building = is_building(work.shape);
            // What is built always stands: when its zone's rows are full (a
            // three-year Maple Street's quay), it stands in the nearest
            // other row of its kind of ground, the water line for a water
            // work and the land rows for the rest, rather than off the scene.
            let elsewhere: &[usize] = if zone == Zone::Water {
                &[WATER, WATER_TALL]
            } else {
                &[NEARER, MIDDLE, BACK, FRONT]
            };
            let Some((row, px)) = self
                .take_unhidden(rows, within, near, building)
                .or_else(|| self.take_unhidden(elsewhere, within, near, building))
            else {
                continue;
            };
            // A wide work takes its neighbours' room in its row too.
            if wide(art.as_deref()) {
                self.street.keep_clear(row, px, 0.17);
            }
            let id = work_id(index);
            self.placed.insert(id, px);
            if let Some(quarter) = quarter {
                self.members.insert(id, quarter);
            }
            let mut item = self.item(
                id,
                if is_building(work.shape) {
                    CanvasItemKind::Place
                } else {
                    CanvasItemKind::Object
                },
                work.label,
                String::new(),
                px,
                row,
                work.shape,
                finished.flatten(),
            );
            if finished.is_some() {
                if let Some(at) = self.street.stretch_at(px) {
                    works[at].push(id);
                }
                item.art = art;
            } else {
                // Going up: scaffolding on its own site, from the first
                // answer that set it going.
                item.art = Some("scaffold".into());
                item.detail = "Being built".into();
            }
            items.push(item);
        }
        works
    }

    /// What the player built on plots stands on its plot (the Pack has
    /// dressed the items): each is placed, is somewhere to spend the day,
    /// and nothing put up after stands right before or behind it, so a flag
    /// on a plot never hangs from a lamp post.
    /// One built for the water stands at the water's edge in front of its
    /// plot, never on the field.
    pub fn plotted(&mut self, items: &mut [CanvasItem], workplaces: &mut [Vec<SelectionId>; 3]) {
        for item in items.iter_mut().filter(|item| item.variant.is_some()) {
            let art = item.art.as_deref().filter(|art| *art != "scaffold");
            if zone_of(art, item.shape.unwrap_or_default()) == Zone::Water {
                item.y = self.row_y[WATER];
            }
            if let Some(px) = item.px {
                self.placed.insert(item.id, px);
                let row = if item.y >= WATER_Y { WATER } else { PLOTS };
                self.stood
                    .push((row, px, item.kind == CanvasItemKind::Place));
                if let Some(at) = self.street.stretch_at(px) {
                    workplaces[at].push(item.id);
                }
                self.street.keep_clear(NEARER, px, CLEAR);
                // A flag on a plot never hangs from a lamp post put up in
                // front of it after. (Only a flag: every plot keeping the
                // quay clear in front of it would leave the quay no room.)
                if item.shape == Some(MarkShape::Flag) {
                    self.street.keep_clear(FRONT, px, CLEAR);
                }
            }
        }
    }

    /// What answers and the player's hands put up (the items not yet
    /// placed that `fixture` picks), beside the place it belongs to, or
    /// where the player put it.
    pub fn fixtures(&mut self, items: &mut [CanvasItem], fixture: impl Fn(&CanvasItem) -> bool) {
        let mut fixtures = items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.px.is_none() && fixture(item))
            .map(|(at, item)| (item.id, at))
            .collect::<Vec<_>>();
        fixtures.sort();
        for (_, at) in fixtures {
            let item = &items[at];
            let near = item
                .spot
                .map(|spot| spot * self.width)
                .or_else(|| item.at.and_then(|place| self.placed.get(&place).copied()))
                .unwrap_or(self.width / 2.0);
            // Each where it belongs: a length of pier on the water, a stall
            // on the quay.
            let zone = zone_of(item.art.as_deref(), item.shape.unwrap_or_default());
            // A bench, a lamp or a stall keeps to the quay, along it if
            // its own stretch is full, never out on the field.
            let rows: &[usize] = match zone {
                Zone::Water => &[WATER],
                Zone::Quay => &[FRONT],
                _ => &[FRONT, NEARER, MIDDLE],
            };
            let within = self.street.stretch_at(near);
            // On the water anywhere before the quay. A quay thing keeps to
            // the quay: when the whole quay is full it waits off the scene
            // rather than stand out on the field behind a building.
            let taken = self.take_spaced(rows, within, near).or_else(|| {
                (zone == Zone::Water)
                    .then(|| self.take_spaced(&[FRONT], within, near))
                    .flatten()
            });
            if let Some((row, px)) = taken {
                items[at].px = Some(px);
                items[at].y = self.row_y[row];
                self.placed.insert(items[at].id, px);
            }
        }
    }

    /// Gives someone at `home` the day `plan` makes, and stands them where
    /// they are now, or at home.
    pub fn live(&self, item: &mut CanvasItem, home: SelectionId, plan: &Plan<SelectionId>) {
        item.home = Some(home);
        item.day = crate::day(plan)
            .into_iter()
            .map(|stop| RoutineStop {
                from_hour: stop.from_hour,
                at: stop.at,
                inside: stop.inside,
            })
            .collect();
        item.px = item
            .at
            .and_then(|at| self.placed.get(&at).copied())
            .or_else(|| self.placed.get(&home).copied());
    }

    /// The stretch something placed stands on.
    pub fn stretch_of(&self, id: SelectionId) -> Option<usize> {
        self.placed
            .get(&id)
            .and_then(|px| self.street.stretch_at(*px))
    }

    /// The town as the scene draws it.
    #[allow(clippy::too_many_arguments)]
    pub fn projection(
        &self,
        items: Vec<CanvasItem>,
        links: Vec<CanvasLink>,
        labels: [&str; 3],
        (season, ground, ice): (Season, Option<GroundCover>, bool),
        plots: Vec<world_projection::Plot>,
        setting: &str,
    ) -> CanvasProjection {
        // Whatever found no room on the panorama waits off the scene.
        let mut items = items;
        items.retain(|item| item.kind == CanvasItemKind::Actor || item.px.is_some());
        stagger_homecomings(&mut items);
        let clusters = self.clusters(&items);
        CanvasProjection {
            items,
            links,
            marks: Vec::new(),
            width: Some(self.width),
            districts: self
                .street
                .stretches()
                .iter()
                .zip(labels)
                .map(|(stretch, label)| District {
                    id: stretch.id.into(),
                    label: label.into(),
                    from: stretch.from,
                    to: stretch.to,
                })
                .collect(),
            season: Some(season),
            ground,
            ice,
            plots,
            setting: Some(setting.into()),
            clusters,
            // The Pack says how its place looks, if it does.
            look: None,
        }
    }

    /// The clusters with anything standing in them, each as wide as what
    /// stands in it and a little more, and as deep as its rows. A cluster
    /// whose homes and works had to stand apart (its own ground was full)
    /// is drawn as a patch for each group that stands together.
    pub fn clusters(&self, items: &[CanvasItem]) -> Vec<Cluster> {
        let mut out = Vec::new();
        for (at, quarter) in self.quarters.iter().enumerate() {
            let mut members = items
                .iter()
                .filter(|item| self.members.get(&item.id) == Some(&at))
                .filter_map(|item| Some((item.px?, item.y)))
                .collect::<Vec<_>>();
            members.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut groups: Vec<Vec<(f32, f32)>> = Vec::new();
            for member in members {
                match groups.last_mut() {
                    Some(group) if member.0 - group[group.len() - 1].0 <= APART => {
                        group.push(member)
                    }
                    _ => groups.push(vec![member]),
                }
            }
            for group in groups {
                let (from, to) = (group[0].0, group[group.len() - 1].0);
                let back = group.iter().map(|m| m.1).fold(f32::MAX, f32::min);
                let front = group.iter().map(|m| m.1).fold(f32::MIN, f32::max);
                out.push(Cluster {
                    id: quarter.id.into(),
                    label: quarter.label.into(),
                    from: (from - 0.1).max(0.0),
                    to: (to + 0.1).min(self.width),
                    ground: quarter.ground,
                    rows: (back, front),
                });
            }
        }
        out.sort_by(|a, b| a.from.total_cmp(&b.from));
        out
    }
}

/// What stands off its water line: every item of `items` drawn as a water
/// work (by [`zone_of`]: a pier, a slipway, a boathouse, the lighthouse)
/// that the scene does not stand on the water (`on_water`, by its index),
/// by label. Work still going up is judged by what it will be: its label
/// names a finished work of `catalog_zone`.
pub fn water_works_astray(
    items: &[CanvasItem],
    on_water: impl Fn(usize) -> bool,
    catalog_zone: impl Fn(&CanvasItem) -> Option<Zone>,
) -> Vec<String> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.kind != CanvasItemKind::Actor && item.px.is_some())
        .filter(|(_, item)| {
            let art = item.art.as_deref().filter(|art| *art != "scaffold");
            let zone = match art {
                Some(_) => zone_of(art, item.shape.unwrap_or_default()),
                None => catalog_zone(item)
                    .unwrap_or_else(|| zone_of(None, item.shape.unwrap_or_default())),
            };
            zone == Zone::Water
        })
        .filter(|(index, _)| !on_water(*index))
        .map(|(_, item)| item.label.clone())
        .collect()
}

/// The widest stretch of a place with nothing built standing in it, in
/// screens: between two of its places, works and things, or from either
/// end to the nearest. Boats and people do not count.
pub fn widest_gap(canvas: &CanvasProjection) -> f32 {
    let width = canvas.width.unwrap_or(1.0);
    let mut spots = canvas
        .items
        .iter()
        .filter(|item| item.kind != CanvasItemKind::Actor)
        .filter(|item| item.shape != Some(MarkShape::Boat))
        .filter_map(|item| item.px)
        .collect::<Vec<_>>();
    spots.push(0.0);
    spots.push(width);
    spots.sort_by(f32::total_cmp);
    spots
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .fold(0.0, f32::max)
}

/// At most a third of the town goes home as work ends at five
/// ([`crate::KNOCK_OFF`]); the rest of those who would stay out until six.
/// Whether someone knocks off at five is a coin of their own, so a town
/// whose people happened to fall that way used to empty at five (fifteen on
/// Mars, seven of them home). Those first by id keep their early evening,
/// so the same people keep the same hours while the town stays the same.
pub(crate) fn stagger_homecomings(items: &mut [CanvasItem]) {
    let mut people = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.kind == CanvasItemKind::Actor)
        .map(|(at, item)| (item.id, at))
        .collect::<Vec<_>>();
    people.sort();
    let mut early = people.len() / 3;
    for (_, at) in people {
        let item = &mut items[at];
        let home = item.home;
        let Some(stop) = item.day.iter_mut().find(|stop| {
            stop.from_hour == crate::KNOCK_OFF && stop.inside && Some(stop.at) == home
        }) else {
            continue;
        };
        if early > 0 {
            early -= 1;
        } else {
            stop.from_hour = crate::KNOCK_OFF + 1;
        }
    }
}
