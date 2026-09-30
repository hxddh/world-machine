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
use std::collections::BTreeMap;
use world_core::EntityId;
use world_projection::{
    CanvasItem, CanvasItemKind, CanvasLink, CanvasProjection, District, GroundCover, MarkShape,
    RoutineStop, Season, SelectionId,
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
}

/// A town being laid out.
pub struct Town {
    pub street: Street,
    pub width: f32,
    /// How far down the scene each row stands, for an app that knows one
    /// screen only.
    pub row_y: [f32; 5],
    /// Where everything placed so far stands along the ground.
    pub placed: BTreeMap<SelectionId, f32>,
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
        ];
        Self {
            street: Street::new(width, stretches, &rows),
            width,
            row_y: [0.3, 0.45, 0.6, 0.76, plots.y],
            placed: BTreeMap::new(),
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
            let near = self.street.spread(stretch, founder.0);
            let Some((row, px)) = self.street.take_first(&[BACK, MIDDLE], Some(stretch), near)
            else {
                continue;
            };
            let id = home_id(*founder);
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

    /// Every work of the catalog has a spot kept for it, in the catalog's
    /// order, whether it is finished yet or not: what is built never moves
    /// for what is built after it, and the stretches fill up as the World
    /// builds. The finished ones (`done`, by their place in the catalog,
    /// with the period each was finished if known) stand in theirs. What
    /// stands on each stretch.
    pub fn works<'a>(
        &mut self,
        items: &mut Vec<CanvasItem>,
        catalog: impl IntoIterator<Item = CatalogWork<'a>>,
        done: &BTreeMap<usize, Option<u32>>,
        art: impl Fn(&str) -> Option<String>,
    ) -> [Vec<SelectionId>; 3] {
        let mut works: [Vec<SelectionId>; 3] = Default::default();
        for (index, work) in catalog.into_iter().enumerate() {
            let near = self.street.spread(work.stretch, index as u64);
            let spot = self
                .street
                .take_first(&[MIDDLE, NEARER, FRONT], Some(work.stretch), near);
            let (Some((row, px)), Some(built)) = (spot, done.get(&index).copied()) else {
                continue;
            };
            let id = work_id(index);
            self.placed.insert(id, px);
            if let Some(at) = self.street.stretch_at(px) {
                works[at].push(id);
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
                built,
            );
            item.art = art(work.id);
            items.push(item);
        }
        works
    }

    /// What the player built on plots stands on its plot (the Pack has
    /// dressed the items): each is placed, is somewhere to spend the day,
    /// and nothing put up after stands right before or behind it, so a flag
    /// on a plot never hangs from a lamp post.
    pub fn plotted(&mut self, items: &[CanvasItem], workplaces: &mut [Vec<SelectionId>; 3]) {
        for item in items.iter().filter(|item| item.variant.is_some()) {
            if let Some(px) = item.px {
                self.placed.insert(item.id, px);
                if let Some(at) = self.street.stretch_at(px) {
                    workplaces[at].push(item.id);
                }
                for row in [NEARER, FRONT] {
                    self.street.keep_clear(row, px, CLEAR);
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
            if let Some((row, px)) =
                self.street
                    .take_first(&[FRONT, NEARER, MIDDLE], self.street.stretch_at(near), near)
            {
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
        }
    }
}
