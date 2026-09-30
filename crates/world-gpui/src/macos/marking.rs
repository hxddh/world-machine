//! The player's mark on the place, in the World window: plots to build on,
//! a card for something that can wear a design or take a name, the design
//! canvas, and the name field. Presentation only: what the player makes
//! reaches the World as an ordinary command, which it checks like any
//! other.

use super::world_window::name_tag;
use super::*;
use crate::art;
use crate::design::{Sketch, Tool};
use crate::diorama::{self, Camera, Stage};
use crate::mark::{self, Wear, PALETTE, PALETTE_NAMES, SIDE, SLOTS};
use crate::postcard::{INK, INK_SOFT, PAPER};
use gpui::{canvas, Focusable, Hsla, KeyDownEvent, Pixels, Role, Stateful};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;
use world_projection::{CanvasItem, MarkShape, PlotOffer};

/// What the player is doing to put their mark on the place.
#[derive(Default)]
pub(crate) struct Marking {
    /// The plot under the pointer, by its index in the World's plots.
    pub(crate) plot_hover: Option<usize>,
    /// The plot chosen with Tab, drawn brighter and opened with Enter.
    pub(crate) plot_focus: Option<usize>,
    /// The plot whose offers are open (by its id), and the offer the keys
    /// are on.
    pub(crate) offers: Option<(String, usize)>,
    /// The hands' verb a plot's card is turned to, instead of what could
    /// stand on the plot itself.
    pub(crate) offers_verb: Option<String>,
    /// Something whose card is open: what can be done to it.
    pub(crate) card: Option<SelectionId>,
    /// A design being made.
    pub(crate) design: Option<Designing>,
    /// A name being given: to what, and the field it is typed in.
    pub(crate) naming: Option<(SelectionId, gpui::Entity<crate::text_input::TextInput>)>,
}

/// A design being made for something.
pub(crate) struct Designing {
    pub(crate) target: SelectionId,
    pub(crate) wear: Wear,
    pub(crate) sketch: Sketch,
    /// A stroke under way with the pointer held down.
    pub(crate) painting: bool,
    /// Where the grid was drawn last frame: its top-left and the size of a
    /// square, to turn the pointer into a square.
    pub(crate) grid: Rc<Cell<Option<(f32, f32, f32)>>>,
}

/// Where the design canvas sits in a window, and how big a square is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DesignLayout {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) cell: f32,
    /// The colours and tools beside the grid; below it in a narrow window.
    pub(crate) beside: bool,
}

const PAD: f32 = 20.0;
const HEADER: f32 = 56.0;
const FOOTER: f32 = 60.0;
const CONTROLS_W: f32 = 212.0;
const CONTROLS_H: f32 = 300.0;
const CONTROLS_BELOW: f32 = 196.0;
const GAP: f32 = 20.0;
/// The room kept above the canvas for the gauges, and below it.
const TOP: f32 = 64.0;
const BOTTOM: f32 = 20.0;

/// Where the design canvas goes in a window `width` by `height`, keeping
/// clear of what it is for, on screen at `target_x` when known, so the
/// design can be seen on it as it is drawn.
pub(crate) fn design_layout(width: f32, height: f32, target_x: Option<f32>) -> DesignLayout {
    let beside = width >= 700.0;
    let room_h = (height - TOP - BOTTOM).max(200.0);
    let (cell, w, h) = if beside {
        let grid_room_h = room_h - HEADER - FOOTER - PAD * 2.0;
        let grid_room_w = width * 0.52 - CONTROLS_W - GAP - PAD * 2.0;
        let cell = (grid_room_h.min(grid_room_w) / SIDE as f32)
            .floor()
            .clamp(10.0, 28.0);
        let grid = cell * SIDE as f32;
        (
            cell,
            PAD * 2.0 + grid + GAP + CONTROLS_W,
            HEADER + PAD * 2.0 + grid.max(CONTROLS_H) + FOOTER,
        )
    } else {
        let grid_room_h = room_h - HEADER - FOOTER - CONTROLS_BELOW - PAD * 2.0 - GAP;
        let grid_room_w = width - 24.0 - PAD * 2.0;
        let cell = (grid_room_h.min(grid_room_w) / SIDE as f32)
            .floor()
            .clamp(8.0, 26.0);
        let grid = cell * SIDE as f32;
        (
            cell,
            (grid + PAD * 2.0)
                .max(CONTROLS_W + PAD * 2.0)
                .min(width - 24.0),
            HEADER + PAD * 2.0 + grid + GAP + CONTROLS_BELOW + FOOTER,
        )
    };
    // Across from what it is for, with a margin; centred if too wide.
    let margin = 24.0_f32.min((width - w) / 2.0).max(0.0);
    let x = match target_x {
        _ if w > width * 0.62 => (width - w) / 2.0,
        Some(target) if target > width / 2.0 => margin,
        _ => width - w - margin,
    };
    let y = (TOP + (room_h - h) / 2.0)
        .max(TOP.min((height - h) / 2.0))
        .max(0.0);
    DesignLayout {
        x: x.max(0.0),
        y,
        w,
        h,
        cell,
        beside,
    }
}

/// How many offers a row of the plot's card holds.
const OFFER_COLUMNS: usize = 3;
pub(crate) const OFFER_W: f32 = 104.0;

impl ProjectionView {
    fn item_of(&self, id: SelectionId) -> Option<&CanvasItem> {
        self.snapshot.canvas.items.iter().find(|item| item.id == id)
    }

    fn item_index(&self, id: SelectionId) -> Option<usize> {
        self.snapshot
            .canvas
            .items
            .iter()
            .position(|item| item.id == id)
    }

    /// Whether something has a card of its own: it can wear a design or
    /// take a name, and the player can act.
    pub(crate) fn has_mark_card(&self, id: SelectionId) -> bool {
        self.controller.is_some()
            && self.retelling.is_none()
            && self
                .item_of(id)
                .is_some_and(|item| mark::designable(item) || mark::nameable(item))
    }

    /// Opens something's card, if it has one.
    pub(crate) fn open_mark_card(&mut self, id: SelectionId, cx: &mut Context<Self>) -> bool {
        if !self.has_mark_card(id) {
            return false;
        }
        self.looking.marking.card = Some(id);
        self.looking.marking.offers = None;
        self.looking.poked = Some((id, Instant::now()));
        self.looking.asking = None;
        cx.notify();
        true
    }

    /// Puts away whatever of the player's mark is open. Whether anything
    /// was.
    pub(crate) fn close_marking(&mut self, cx: &mut Context<Self>) -> bool {
        let marking = &mut self.looking.marking;
        let open = marking.card.is_some()
            || marking.offers.is_some()
            || marking.design.is_some()
            || marking.naming.is_some();
        marking.card = None;
        marking.offers = None;
        marking.design = None;
        marking.naming = None;
        if open {
            cx.notify();
        }
        open
    }

    /// The frame as the player's mark changes it: the plot pointed at or
    /// chosen drawn brighter, and a design being made shown on what it is
    /// for.
    pub(crate) fn dress_frame(&self, frame: &mut diorama::Frame) {
        let marking = &self.looking.marking;
        let open = marking.offers.as_ref().and_then(|(id, _)| {
            mark::plots_of(&self.snapshot)
                .iter()
                .position(|plot| &plot.id == id)
        });
        frame.point_at_plot(open.or(marking.plot_focus).or(marking.plot_hover));
        if let Some(designing) = &marking.design {
            if let Some(index) = self.item_index(designing.target) {
                frame.wear(index, designing.wear, designing.sketch.motif());
            }
        }
    }

    /// What can be clicked of the player's mark: the plots, and the things
    /// that have a card.
    pub(crate) fn mark_targets(
        &self,
        stage: &Stage,
        camera: Camera,
        cx: &mut Context<Self>,
    ) -> Vec<gpui::AnyElement> {
        let mut out = Vec::new();
        let acting = self.controller.is_some() && self.retelling.is_none();
        let opacity = mark::plot_opacity(camera.zoom);
        let plots = mark::plots_of(&self.snapshot);
        if acting && opacity > 0.3 {
            let marking = &self.looking.marking;
            for spot in &stage.plots {
                let Some(plot) = plots.get(spot.plot) else {
                    continue;
                };
                let (x, y) = camera.at(stage, spot.x, spot.y);
                let w = spot.w * camera.zoom;
                let deep = w * 0.2;
                let index = spot.plot;
                let open = marking
                    .offers
                    .as_ref()
                    .is_some_and(|(id, _)| *id == plot.id);
                let shown =
                    open || marking.plot_hover == Some(index) || marking.plot_focus == Some(index);
                let group = SharedString::from(format!("plot-{}", plot.id));
                let id = plot.id.clone();
                out.push(
                    div()
                        .id(SharedString::from(format!("plot-{}", plot.id)))
                        .role(Role::Button)
                        .aria_label(ui::t("A plot"))
                        .aria_expanded(open)
                        .group(group.clone())
                        .absolute()
                        .left(px(x - w / 2.0))
                        .top(px(y - deep - 30.0))
                        .w(px(w))
                        .h(px(deep + 38.0))
                        .cursor_pointer()
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(name_tag(ui::t("A plot").to_string()).when(!shown, |tag| {
                            tag.opacity(0.0)
                                .group_hover(group, |style| style.opacity(1.0))
                        }))
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            let marking = &mut this.looking.marking;
                            if *hovered {
                                marking.plot_hover = Some(index);
                            } else if marking.plot_hover == Some(index) {
                                marking.plot_hover = None;
                            }
                            cx.notify();
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if this.just_dragged() {
                                return;
                            }
                            this.open_offers(&id, cx);
                        }))
                        .into_any_element(),
                );
            }
        }
        // Things that can wear a design or take a name open their card.
        if acting {
            for spot in &stage.things {
                let item = &self.snapshot.canvas.items[spot.index];
                if !self.has_mark_card(item.id) {
                    continue;
                }
                let (x, base) = camera.at(stage, spot.x, spot.y);
                let w = stage.thing_w * camera.zoom;
                let tall = w * if item.shape == Some(MarkShape::Boat) {
                    1.1
                } else {
                    1.0
                };
                let selection = item.id;
                let group = SharedString::from(format!("thing-{}", selection.stable_key()));
                // Its name while its card is open, but not over a design
                // being tried on it.
                let open = self.looking.marking.card == Some(selection)
                    && self.looking.marking.design.is_none();
                out.push(
                    div()
                        .id(SharedString::from(format!(
                            "stage-{}",
                            selection.stable_key()
                        )))
                        .role(Role::Button)
                        .aria_label(item.label.clone())
                        .aria_expanded(open)
                        .group(group.clone())
                        .absolute()
                        .left(px(x - w / 2.0))
                        .top(px(base - tall - 22.0))
                        .w(px(w))
                        .h(px(tall + 22.0))
                        .cursor_pointer()
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(name_tag(item.label.clone()).when(!open, |tag| {
                            tag.opacity(0.0)
                                .group_hover(group, |style| style.opacity(1.0))
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if this.just_dragged() || this.place_at(selection, cx) {
                                return;
                            }
                            this.open_mark_card(selection, cx);
                        }))
                        .into_any_element(),
                );
            }
        }
        out
    }

    fn open_offers(&mut self, plot: &str, cx: &mut Context<Self>) {
        let marking = &mut self.looking.marking;
        marking.card = None;
        marking.design = None;
        marking.offers = Some((plot.to_string(), 0));
        marking.offers_verb = None;
        self.looking.asking = None;
        self.looking.hands = None;
        self.cue(crate::Cue::Flip);
        cx.notify();
    }

    /// Builds `offer` on its plot: the World checks it like any deed.
    fn build(&mut self, offer: &PlotOffer, cx: &mut Context<Self>) {
        if offer.unavailable.is_some() {
            return;
        }
        let command = offer.command.clone();
        self.looking.marking.offers = None;
        self.looking.marking.plot_focus = None;
        self.invoke_command(command, cx);
    }

    /// The plot's card: what could stand there, each as it would be drawn,
    /// with what it costs, greyed with the reason when it cannot be built;
    /// and, turned to the hands' verbs, the smaller things the player can
    /// make or do anywhere. One card, one way to build.
    pub(crate) fn render_offers(
        &self,
        stage: &Stage,
        camera: Camera,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let (id, chosen) = self.looking.marking.offers.clone()?;
        let plots = mark::plots_of(&self.snapshot);
        let index = plots.iter().position(|plot| plot.id == id)?;
        let plot = &plots[index];
        let spot = stage.plots.iter().find(|spot| spot.plot == index)?;
        let (x, y) = camera.at(stage, spot.x, spot.y);
        let verbs = self.hand_verbs();
        let verb = self
            .looking
            .marking
            .offers_verb
            .clone()
            .filter(|verb| verbs.contains(&verb.as_str()));
        let things = verb
            .as_deref()
            .map(|verb| self.hand_things(verb))
            .unwrap_or_default();
        let count = if verb.is_some() {
            things.len()
        } else {
            plot.offers.len()
        };
        let columns = OFFER_COLUMNS;
        let rows = count.div_ceil(OFFER_COLUMNS).max(1);
        // The tiles, the gaps between them, the card's padding and border.
        let width = columns as f32 * OFFER_W + (columns - 1) as f32 * 8.0 + 32.0 + 6.0;
        let tabs_tall = if verbs.is_empty() { 0.0 } else { 70.0 };
        // The tiles scroll inside the card rather than run off the window.
        let grid_h = (rows as f32 * 132.0).min((stage.height - 290.0).max(140.0));
        let tall = 92.0 + tabs_tall + grid_h;
        let district = self
            .snapshot
            .canvas
            .districts
            .iter()
            .find(|district| district.id == plot.district)
            .map(|district| district.label.clone());
        let mut grid = div()
            .id("plot-offers")
            .role(Role::List)
            .aria_label(ui::t("What could stand here"))
            .max_h(px(grid_h))
            .overflow_y_scroll()
            .flex()
            .flex_wrap()
            .gap_2();
        match &verb {
            None => {
                for (at, offer) in plot.offers.iter().enumerate() {
                    let offer_ = offer.clone();
                    let available = offer.unavailable.is_none();
                    let caption = offer
                        .unavailable
                        .clone()
                        .or_else(|| offer.cost.clone())
                        .unwrap_or_default();
                    let tile = offer_tile(
                        SharedString::from(format!("offer-{at}")),
                        &offer.label,
                        caption,
                        available,
                        at == chosen,
                        offer.shape,
                        offer.art.clone(),
                    );
                    grid = grid.child(if available {
                        tile.on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.build(&offer_, cx);
                        }))
                    } else {
                        tile
                    });
                }
            }
            Some(verb) => {
                for thing in things {
                    let (verb, key) = (verb.clone(), thing.key.clone());
                    let tile = offer_tile(
                        SharedString::from(format!("plot-thing-{}", thing.command)),
                        &thing.label,
                        thing
                            .reason
                            .clone()
                            .or(thing.cost.clone())
                            .unwrap_or_default(),
                        thing.possible,
                        false,
                        thing.shape,
                        None,
                    );
                    grid = grid.child(if thing.possible {
                        tile.on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.looking.marking.offers = None;
                            this.looking.marking.offers_verb = None;
                            this.looking.hands = Some(super::world_window::Hands {
                                verb: Some(verb.clone()),
                                thing: Some(key.clone()),
                            });
                            cx.notify();
                        }))
                    } else {
                        tile
                    });
                }
            }
        }
        let mut tabs = div()
            .id("plot-verbs")
            .role(Role::TabList)
            .aria_label(ui::t("What to do"))
            .flex()
            .flex_wrap()
            .gap_1();
        if !verbs.is_empty() {
            tabs = tabs.child(
                super::world_window::verb_tab("Build here", verb.is_none()).on_click(cx.listener(
                    |this, _, _, cx| {
                        cx.stop_propagation();
                        this.looking.marking.offers_verb = None;
                        cx.notify();
                    },
                )),
            );
            for each in verbs.iter().copied().filter(|verb| *verb != "Move") {
                // Here, the hands' small builds are told from the plot's.
                let label = if each == "Build" {
                    "Something small"
                } else {
                    each
                };
                tabs = tabs.child(
                    super::world_window::labelled_tab(each, label, verb.as_deref() == Some(each))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if super::world_window::to_someone(each) {
                                this.looking.marking.offers = None;
                                this.looking.marking.offers_verb = None;
                                this.looking.hands = Some(super::world_window::Hands {
                                    verb: Some(each.to_string()),
                                    thing: Some("*".into()),
                                });
                            } else {
                                this.looking.marking.offers_verb = Some(each.to_string());
                            }
                            cx.notify();
                        })),
                );
            }
        }
        let caption = match district {
            Some(district) => format!("{} · {}", district, ui::t("What could stand here?")),
            None => ui::t("What could stand here?").to_string(),
        };
        let card = div()
            .id("plot-card")
            .role(Role::Dialog)
            .aria_label(ui::t("A plot"))
            .w(px(width))
            .p_4()
            .rounded_xl()
            .bg(super::world_window::scene_paper())
            .shadow_lg()
            .border_1()
            .border_color(color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_3()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(ui::row_title("A plot"))
                            .child(ui::caption(caption))
                            .children(self.purse().map(ui::caption)),
                    )
                    .child(close_button(
                        "plot-close",
                        cx.listener(|this, _, _, cx| {
                            this.looking.marking.offers = None;
                            this.looking.marking.offers_verb = None;
                            cx.notify();
                        }),
                    )),
            )
            .when(!verbs.is_empty(), |card| card.child(tabs))
            .child(grid);
        let left = (x - width / 2.0).clamp(12.0, (stage.view_w - width - 12.0).max(12.0));
        let above = y - spot.w * camera.zoom * 0.2 - tall - 18.0;
        let top = if above >= 60.0 { above } else { y + 18.0 };
        Some(
            div()
                .id("plot-card-place")
                .absolute()
                .left(px(left))
                .top(px(top.min(stage.height - tall - 8.0).max(8.0)))
                .child(ui::spring_in(
                    card,
                    format!("plot-{id}"),
                    cx.reduce_motion(),
                )),
        )
    }

    /// Something's card: its name, and what the player can do to it.
    pub(crate) fn render_mark_card(
        &self,
        stage: &Stage,
        camera: Camera,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        const WIDTH: f32 = 280.0;
        let id = self.looking.marking.card?;
        if self.looking.marking.design.is_some() {
            return None;
        }
        let index = self.item_index(id)?;
        let item = self.item_of(id)?;
        let (fx, fy, fw, fh) = stage.frame_of(index)?;
        let (x0, y0) = camera.at(stage, fx, fy);
        let (x1, _) = camera.at(stage, fx + fw, fy + fh);
        let naming = self
            .looking
            .marking
            .naming
            .as_ref()
            .filter(|(target, _)| *target == id)
            .map(|(_, input)| input.clone());
        let mut actions = div().flex().flex_wrap().gap_2();
        if let Some(wear) = mark::designable(item)
            .then(|| mark::wear_of(item))
            .flatten()
        {
            actions = actions.child(
                ui::button("mark-design", "Design…", ButtonKind::Secondary)
                    .aria_keyshortcuts("D")
                    .on_click(cx.listener(move |this, _, _, cx| this.start_design(id, wear, cx))),
            );
        }
        if mark::nameable(item) && naming.is_none() {
            actions = actions.child(
                ui::button("mark-name", "Name…", ButtonKind::Secondary)
                    .aria_keyshortcuts("N")
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.start_naming(id, window, cx)),
                    ),
            );
        }
        let card = div()
            .id("mark-card")
            .role(Role::Dialog)
            .aria_label(item.label.clone())
            .w(px(WIDTH))
            .p_4()
            .rounded_xl()
            .bg(color(tokens::SURFACE))
            .shadow_lg()
            .border_1()
            .border_color(color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_3()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(ui::row_title(item.label.clone()))
                            .when(!item.detail.is_empty(), |column| {
                                column.child(ui::caption(capitalize(&item.detail)))
                            }),
                    )
                    .child(close_button(
                        "mark-close",
                        cx.listener(|this, _, _, cx| {
                            this.close_marking(cx);
                        }),
                    )),
            )
            .child(actions)
            .children(naming.map(|input| self.naming_block(id, input, cx)))
            .child(
                div()
                    .id("mark-more")
                    .role(Role::Button)
                    .text_xs()
                    .text_color(color(tokens::TEXT_SECONDARY))
                    .cursor_pointer()
                    .hover(|style| style.text_color(color(tokens::ACCENT_TEXT)))
                    .child(ui::t("More in the drawer"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.close_marking(cx);
                        this.selected = Some(id);
                        this.open_drawer();
                        cx.notify();
                    })),
            );
        let left = if x1 + 16.0 + WIDTH < stage.view_w - 12.0 {
            x1 + 16.0
        } else {
            (x0 - 16.0 - WIDTH).max(12.0)
        };
        let top = y0.clamp(64.0, (stage.height - 300.0).max(64.0));
        Some(
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .child(ui::spring_in(
                    card,
                    format!("mark-{}", id.stable_key()),
                    cx.reduce_motion(),
                )),
        )
    }

    /// The name field, with whatever names are proposed as chips to pick.
    pub(crate) fn naming_block(
        &self,
        id: SelectionId,
        input: gpui::Entity<crate::text_input::TextInput>,
        cx: &mut Context<Self>,
    ) -> Div {
        let proposals = self
            .item_of(id)
            .map(|item| mark::proposals(item).to_vec())
            .unwrap_or_default();
        let typed = input.read(cx).text().trim().to_string();
        let valid = mark::valid_name(&typed);
        let mut chips = div()
            .id("name-proposals")
            .role(Role::List)
            .aria_label(ui::t("Names proposed"))
            .flex()
            .flex_wrap()
            .gap_1();
        for (at, name) in proposals.iter().enumerate() {
            let chosen = typed == *name;
            let field = input.clone();
            let name_ = name.clone();
            chips = chips.child(
                div()
                    .id(SharedString::from(format!("proposal-{at}")))
                    .role(Role::Button)
                    .aria_label(name.clone())
                    .aria_selected(chosen)
                    .px_3()
                    .py(px(3.0))
                    .rounded_full()
                    .text_sm()
                    .cursor_pointer()
                    .bg(color(if chosen {
                        tokens::ACCENT_SOFT
                    } else {
                        tokens::ROW_HOVER
                    }))
                    .text_color(color(if chosen {
                        tokens::ACCENT_TEXT
                    } else {
                        tokens::TEXT
                    }))
                    .hover(|style| style.bg(color(tokens::ACCENT_SOFT)))
                    .child(name.clone())
                    .on_click(cx.listener(move |_, _, window, cx| {
                        let name = name_.clone();
                        field.update(cx, |field, cx| field.set_text(name, cx));
                        window.focus(&field.focus_handle(cx), cx);
                    })),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().min_w(px(0.0)).child(input))
                    .child(
                        ui::button("name-confirm", "Name", ButtonKind::Primary)
                            .when(!valid, |button| button.opacity(0.5))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.confirm_name(window, cx)),
                            ),
                    ),
            )
            .when(!proposals.is_empty(), |block| block.child(chips))
            .child(ui::caption(if proposals.is_empty() {
                "Enter to name it · Esc to leave it"
            } else {
                "Type a name, or pick one · Enter to name it"
            }))
    }

    pub(crate) fn start_naming(
        &mut self,
        id: SelectionId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.item_of(id).is_some_and(mark::nameable) {
            return;
        }
        let input = cx.new(|cx| crate::text_input::TextInput::new(ui::t("A name…"), cx));
        window.focus(&input.focus_handle(cx), cx);
        self.looking.marking.naming = Some((id, input));
        cx.notify();
    }

    /// Whether the player is typing a name.
    pub(crate) fn naming_typed(&self, window: &Window, cx: &Context<Self>) -> bool {
        self.looking
            .marking
            .naming
            .as_ref()
            .is_some_and(|(_, input)| input.focus_handle(cx).is_focused(window))
    }

    /// Gives the name typed, if it is one. While a name is being composed
    /// (Chinese through an input method), Enter belongs to the composing.
    fn confirm_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, input)) = self.looking.marking.naming.clone() else {
            return;
        };
        if input.read(cx).composing() {
            return;
        }
        let name = input.read(cx).text().to_string();
        let Some(ProjectionIntent::InvokeCommand(command)) = self
            .item_of(id)
            .and_then(|item| mark::name_intent(item, &name))
        else {
            return;
        };
        self.looking.marking.naming = None;
        if let Some(focus) = &self.looking.focus {
            window.focus(focus, cx);
        }
        self.invoke_command(command, cx);
    }

    fn cancel_naming(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.looking.marking.naming = None;
        if let Some(focus) = &self.looking.focus {
            window.focus(focus, cx);
        }
        cx.notify();
    }

    pub(crate) fn start_design(&mut self, id: SelectionId, wear: Wear, cx: &mut Context<Self>) {
        let Some(item) = self.item_of(id) else {
            return;
        };
        let sketch = mark::pattern_of(item)
            .map(|motif| Sketch::from_motif(&motif))
            .unwrap_or_default();
        let marking = &mut self.looking.marking;
        marking.naming = None;
        marking.offers = None;
        marking.design = Some(Designing {
            target: id,
            wear,
            sketch,
            painting: false,
            grid: Rc::default(),
        });
        cx.notify();
    }

    /// Sends the design to the World, if it changed; puts the canvas away.
    fn save_design(&mut self, cx: &mut Context<Self>) {
        let Some(designing) = self.looking.marking.design.take() else {
            return;
        };
        self.looking.marking.card = None;
        if !designing.sketch.changed() {
            cx.notify();
            return;
        }
        let motif = designing.sketch.motif();
        match self
            .item_of(designing.target)
            .and_then(|item| mark::design_intent(item, &motif))
        {
            Some(ProjectionIntent::InvokeCommand(command)) => self.invoke_command(command, cx),
            _ => cx.notify(),
        }
    }

    fn cancel_design(&mut self, cx: &mut Context<Self>) {
        self.looking.marking.design = None;
        cx.notify();
    }

    fn sketch(&mut self, change: impl FnOnce(&mut Sketch), cx: &mut Context<Self>) {
        if let Some(designing) = self.looking.marking.design.as_mut() {
            change(&mut designing.sketch);
            cx.notify();
        }
    }

    /// The square of the grid under a window point, if any.
    fn square_at(&self, position: gpui::Point<Pixels>) -> Option<(usize, usize)> {
        let (x0, y0, cell) = self.looking.marking.design.as_ref()?.grid.get()?;
        let (x, y) = (f32::from(position.x) - x0, f32::from(position.y) - y0);
        let (column, row) = ((x / cell).floor(), (y / cell).floor());
        (column >= 0.0 && row >= 0.0 && column < SIDE as f32 && row < SIDE as f32)
            .then_some((column as usize, row as usize))
    }

    /// The design canvas: the grid, its colours and tools, and the design
    /// shown on what it is for as it is drawn.
    pub(crate) fn render_design(
        &self,
        width: f32,
        height: f32,
        target_x: Option<f32>,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let designing = self.looking.marking.design.as_ref()?;
        let layout = design_layout(width, height, target_x);
        let sketch = designing.sketch.clone();
        let grid_side = layout.cell * SIDE as f32;
        let grid_at = designing.grid.clone();
        let cells = sketch.cells;
        let colours = sketch.colours;
        let cursor = sketch.cursor;
        let cell = layout.cell;
        let grid = div()
            .id("design-grid")
            .role(Role::Grid)
            .aria_label(ui::t("The design, sixteen squares by sixteen"))
            .aria_keyshortcuts("ArrowUp ArrowDown ArrowLeft ArrowRight Space 1 2 3 4 5 6 7 8 F")
            .flex_shrink_0()
            .size(px(grid_side + 12.0))
            .p(px(6.0))
            .rounded_md()
            .bg(art::hex(PAPER))
            .border_1()
            .border_color(art::hex(INK_SOFT).opacity(0.5))
            .cursor_crosshair()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    if let Some((column, row)) = this.square_at(event.position) {
                        if let Some(designing) = this.looking.marking.design.as_mut() {
                            designing.sketch.apply(column, row);
                            designing.painting = designing.sketch.tool == Tool::Pencil;
                        }
                        cx.notify();
                    }
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                cx.stop_propagation();
                let held = event.pressed_button == Some(gpui::MouseButton::Left);
                let square = this.square_at(event.position);
                if let Some(designing) = this.looking.marking.design.as_mut() {
                    if !held {
                        designing.painting = false;
                    } else if designing.painting {
                        if let Some((column, row)) = square {
                            if designing.sketch.at(column, row) != designing.sketch.slot {
                                designing.sketch.put(column, row);
                                designing.sketch.cursor = (column, row);
                                cx.notify();
                            }
                        }
                    }
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _: &gpui::MouseUpEvent, _, _| {
                    if let Some(designing) = this.looking.marking.design.as_mut() {
                        designing.painting = false;
                    }
                }),
            )
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let (x0, y0) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
                        grid_at.set(Some((x0, y0, cell)));
                        paint_grid(window, (x0, y0), cell, &cells, &colours, cursor);
                    },
                )
                .size(px(grid_side)),
            );
        let controls = self.design_controls(&sketch, cx);
        let body = if layout.beside {
            div()
                .flex()
                .items_start()
                .gap(px(GAP))
                .child(grid)
                .child(controls.w(px(CONTROLS_W)))
        } else {
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(GAP))
                .child(grid)
                .child(controls.w_full())
        };
        let panel = div()
            .id("design-canvas")
            .role(Role::Dialog)
            .aria_label(ui::t(designing.wear.title()))
            .w(px(layout.w))
            .p(px(PAD))
            .rounded_xl()
            .bg(color(tokens::SURFACE))
            .shadow_lg()
            .border_1()
            .border_color(color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_3()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_move(|_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(ui::heading(designing.wear.title()))
                            .child(ui::caption(
                                "Arrows move · Space paints · 1–8 colours · F fills",
                            )),
                    )
                    .child(close_button(
                        "design-close",
                        cx.listener(|this, _, _, cx| this.cancel_design(cx)),
                    )),
            )
            .child(body)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .child(
                        ui::button("design-cancel", "Cancel", ButtonKind::Secondary)
                            .aria_keyshortcuts("Escape")
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_design(cx))),
                    )
                    .child(
                        ui::button("design-save", "Save", ButtonKind::Primary)
                            .aria_keyshortcuts("Enter")
                            .on_click(cx.listener(|this, _, _, cx| this.save_design(cx))),
                    ),
            );
        Some(
            div()
                .absolute()
                .left(px(layout.x))
                .top(px(layout.y))
                .child(ui::spring_in(panel, "design-canvas", cx.reduce_motion())),
        )
    }

    /// The colours, the mixing palette, the tools, and undo, redo and
    /// clear.
    fn design_controls(&self, sketch: &Sketch, cx: &mut Context<Self>) -> Div {
        let mut slots = div()
            .id("design-colours")
            .role(Role::RadioGroup)
            .aria_label(ui::t("Colours"))
            .flex()
            .flex_wrap()
            .gap(px(6.0));
        for slot in 0..SLOTS {
            let place = sketch.colours[slot] as usize;
            let chosen = sketch.slot as usize == slot;
            let rgb = PALETTE[place];
            let fill = rgb_colour(rgb);
            let label = format!(
                "{} {}: {}",
                ui::t("Colour"),
                slot + 1,
                ui::t(PALETTE_NAMES[place])
            );
            let dark = luminance(rgb) < 0.5;
            slots = slots.child(
                ui::named(div().id(SharedString::from(format!("slot-{slot}"))), label)
                    .role(Role::RadioButton)
                    .aria_selected(chosen)
                    .aria_keyshortcuts((slot + 1).to_string())
                    .size(px(40.0))
                    .rounded_md()
                    .bg(fill)
                    .border_2()
                    .border_color(if chosen {
                        color(tokens::ACCENT)
                    } else {
                        art::hex(INK).opacity(0.15).into()
                    })
                    .when(chosen, |swatch| swatch.shadow_sm())
                    .cursor_pointer()
                    .flex()
                    .items_end()
                    .justify_end()
                    .p(px(3.0))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if dark {
                                gpui::white().opacity(0.85)
                            } else {
                                art::hex(INK).opacity(0.7)
                            })
                            .child((slot + 1).to_string()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.sketch(|sketch| sketch.choose(slot), cx)
                    })),
            );
        }
        let chosen_place = sketch.colours[sketch.slot as usize];
        let mut mix = div()
            .id("design-mix")
            .role(Role::List)
            .aria_label(ui::t("The palette"))
            .flex()
            .flex_wrap()
            .gap(px(4.0));
        for (place, rgb) in PALETTE.iter().enumerate() {
            let on = chosen_place as usize == place;
            mix = mix.child(
                ui::named(
                    div().id(SharedString::from(format!("mix-{place}"))),
                    PALETTE_NAMES[place],
                )
                .role(Role::Button)
                .aria_selected(on)
                .size(px(21.0))
                .rounded_full()
                .bg(rgb_colour(*rgb))
                .border_1()
                .border_color(if on {
                    color(tokens::ACCENT)
                } else {
                    art::hex(INK).opacity(0.18).into()
                })
                .when(on, |chip| chip.border_2())
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.sketch(|sketch| sketch.mix(place as u8), cx)
                })),
            );
        }
        let mut tools = div()
            .id("design-tools")
            .role(Role::RadioGroup)
            .aria_label(ui::t("Tools"))
            .flex()
            .gap(px(6.0));
        for tool in Tool::ALL {
            let on = sketch.tool == tool;
            tools = tools.child(
                ui::named(
                    div().id(SharedString::from(format!("tool-{tool:?}"))),
                    tool.name(),
                )
                .role(Role::RadioButton)
                .aria_selected(on)
                .size(px(36.0))
                .rounded_md()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .bg(color(if on {
                    tokens::ACCENT_SOFT
                } else {
                    tokens::SURFACE
                }))
                .border_1()
                .border_color(color(if on { tokens::ACCENT } else { tokens::BORDER }))
                .hover(|style| style.bg(color(tokens::ROW_HOVER)))
                .child(tool_glyph(tool, on).size(px(20.0)))
                .on_click(
                    cx.listener(move |this, _, _, cx| this.sketch(|sketch| sketch.tool = tool, cx)),
                ),
            );
        }
        let (can_undo, can_redo) = (sketch.can_undo(), sketch.can_redo());
        let small = |id: &'static str, label: &'static str, enabled: bool| {
            ui::button(id, label, ButtonKind::Secondary)
                .text_xs()
                .px_2()
                .py(px(3.0))
                .when(!enabled, |button| button.opacity(0.45))
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(ui::section_label("Colours"))
            .child(slots)
            .child(ui::section_label("Mix the chosen colour"))
            .child(mix)
            .child(ui::section_label("Tools"))
            .child(tools)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(
                        small("design-undo", "Undo", can_undo)
                            .aria_keyshortcuts("Meta+Z")
                            .on_click(cx.listener(|this, _, _, cx| this.sketch(Sketch::undo, cx))),
                    )
                    .child(
                        small("design-redo", "Redo", can_redo)
                            .aria_keyshortcuts("Shift+Meta+Z")
                            .on_click(cx.listener(|this, _, _, cx| this.sketch(Sketch::redo, cx))),
                    )
                    .child(
                        small("design-clear", "Clear", true)
                            .on_click(cx.listener(|this, _, _, cx| this.sketch(Sketch::clear, cx))),
                    ),
            )
    }

    /// The keys of the player's mark: the design canvas, a name being
    /// typed, a plot's offers, Tab through the plots. Whether they were
    /// the mark's.
    pub(crate) fn mark_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = event.keystroke.key.as_str();
        let modifiers = &event.keystroke.modifiers;
        let command = modifiers.platform || modifiers.control;
        // A name being typed: Enter gives it, unless a name is still being
        // composed; Escape leaves it. Every other key is the field's.
        if self.naming_typed(window, cx) {
            match key {
                "enter" => self.confirm_name(window, cx),
                "escape" => self.cancel_naming(window, cx),
                _ => {}
            }
            return true;
        }
        if self.looking.marking.design.is_some() {
            match key {
                "escape" => self.cancel_design(cx),
                "enter" => self.save_design(cx),
                "z" if command && modifiers.shift => self.sketch(Sketch::redo, cx),
                "z" if command => self.sketch(Sketch::undo, cx),
                "y" if command => self.sketch(Sketch::redo, cx),
                "left" | "right" | "up" | "down" => {
                    let (dx, dy) = match key {
                        "left" => (-1, 0),
                        "right" => (1, 0),
                        "up" => (0, -1),
                        _ => (0, 1),
                    };
                    let paint = modifiers.shift;
                    self.sketch(
                        |sketch| {
                            sketch.move_cursor(dx, dy);
                            if paint {
                                let (column, row) = sketch.cursor;
                                sketch.apply(column, row);
                            }
                        },
                        cx,
                    );
                }
                "space" => self.sketch(
                    |sketch| {
                        let (column, row) = sketch.cursor;
                        sketch.apply(column, row);
                    },
                    cx,
                ),
                "f" if !command => self.sketch(
                    |sketch| {
                        let (column, row) = sketch.cursor;
                        sketch.fill(column, row);
                    },
                    cx,
                ),
                "p" if !command => self.sketch(|sketch| sketch.tool = Tool::Pencil, cx),
                "i" if !command => self.sketch(
                    |sketch| {
                        let (column, row) = sketch.cursor;
                        sketch.slot = sketch.at(column, row);
                        sketch.tool = Tool::Pencil;
                    },
                    cx,
                ),
                "backspace" | "delete" => self.sketch(Sketch::clear, cx),
                digit => {
                    if let Some(slot) = digit
                        .parse::<usize>()
                        .ok()
                        .filter(|slot| (1..=SLOTS).contains(slot))
                    {
                        self.sketch(|sketch| sketch.choose(slot - 1), cx);
                    }
                }
            }
            return true;
        }
        if let Some((id, chosen)) = self.looking.marking.offers.clone() {
            let count = mark::plots_of(&self.snapshot)
                .iter()
                .find(|plot| plot.id == id)
                .map_or(0, |plot| plot.offers.len());
            let step =
                |by: isize| ((chosen as isize + by).rem_euclid(count.max(1) as isize)) as usize;
            match key {
                "escape" => self.looking.marking.offers = None,
                "left" => self.looking.marking.offers = Some((id, step(-1))),
                "right" | "tab" => self.looking.marking.offers = Some((id, step(1))),
                "up" => self.looking.marking.offers = Some((id, step(-(OFFER_COLUMNS as isize)))),
                "down" => self.looking.marking.offers = Some((id, step(OFFER_COLUMNS as isize))),
                "enter" | "space" => {
                    if let Some(offer) = mark::plots_of(&self.snapshot)
                        .iter()
                        .find(|plot| plot.id == id)
                        .and_then(|plot| plot.offers.get(chosen))
                        .cloned()
                    {
                        self.build(&offer, cx);
                    }
                }
                _ => return false,
            }
            cx.notify();
            return true;
        }
        if let Some(id) = self.looking.marking.card {
            match key {
                "escape" => {
                    self.close_marking(cx);
                    return true;
                }
                "d" if !command => {
                    if let Some(wear) = self
                        .item_of(id)
                        .filter(|item| mark::designable(item))
                        .and_then(mark::wear_of)
                    {
                        self.start_design(id, wear, cx);
                        return true;
                    }
                }
                "n" if !command => {
                    self.start_naming(id, window, cx);
                    return true;
                }
                _ => {}
            }
        }
        // Tab goes from plot to plot, and Enter opens the one chosen.
        let plots = mark::plots_of(&self.snapshot).len();
        let acting = self.controller.is_some() && self.retelling.is_none();
        if acting && plots > 0 {
            match key {
                "tab" => {
                    let next = self.next_plot(modifiers.shift, window);
                    self.looking.marking.plot_focus = next;
                    self.look_at_plot(window, cx);
                    cx.notify();
                    return true;
                }
                "enter" | "space" => {
                    if let Some(at) = self.looking.marking.plot_focus {
                        if let Some(id) =
                            mark::plots_of(&self.snapshot).get(at).map(|p| p.id.clone())
                        {
                            self.open_offers(&id, cx);
                            return true;
                        }
                    }
                }
                "escape" if self.looking.marking.plot_focus.is_some() => {
                    self.looking.marking.plot_focus = None;
                    cx.notify();
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// The plot Tab goes to: from none, the one nearest the middle of the
    /// view; then the next along the place, left to right (or back, with
    /// Shift), round again at the end.
    fn next_plot(&self, back: bool, window: &Window) -> Option<usize> {
        let (width, height) = self.stage_size(window);
        let stage = diorama::stage(&self.snapshot, width, height);
        let mut order = stage
            .plots
            .iter()
            .map(|spot| (spot.x, spot.plot))
            .collect::<Vec<_>>();
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        if order.is_empty() {
            return None;
        }
        let at = match self.looking.marking.plot_focus {
            Some(focus) => {
                let here = order
                    .iter()
                    .position(|(_, plot)| *plot == focus)
                    .unwrap_or(0);
                let count = order.len();
                if back {
                    (here + count - 1) % count
                } else {
                    (here + 1) % count
                }
            }
            None => {
                let camera = self
                    .looking
                    .camera_now
                    .unwrap_or_else(|| Camera::whole(&stage));
                order
                    .iter()
                    .enumerate()
                    .min_by(|a, b| {
                        (a.1 .0 - camera.x)
                            .abs()
                            .total_cmp(&(b.1 .0 - camera.x).abs())
                    })
                    .map_or(0, |(at, _)| at)
            }
        };
        Some(order[at].1)
    }

    /// Pans a panorama to the plot chosen with Tab, if it is out of view.
    fn look_at_plot(&mut self, window: &Window, cx: &mut Context<Self>) {
        let Some(at) = self.looking.marking.plot_focus else {
            return;
        };
        let (width, height) = self.stage_size(window);
        let stage = diorama::stage(&self.snapshot, width, height);
        let Some(spot) = stage.plots.iter().find(|spot| spot.plot == at) else {
            return;
        };
        let camera = self
            .looking
            .camera_now
            .unwrap_or_else(|| Camera::whole(&stage));
        let (x, _) = camera.at(&stage, spot.x, spot.y);
        if x < stage.view_w * 0.15 || x > stage.view_w * 0.85 {
            self.looking.pan = Some(spot.x);
            cx.notify();
        }
    }
}

/// A small round close button.
fn close_button(
    id: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> Stateful<Div> {
    ui::named(div().id(id), "Close")
        .role(Role::Button)
        .size(px(26.0))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .text_base()
        .text_color(color(tokens::TEXT_SECONDARY))
        .cursor_pointer()
        .hover(|style| {
            style
                .bg(color(tokens::ROW_HOVER))
                .text_color(color(tokens::TEXT))
        })
        .child("×")
        .on_click(on_click)
}

fn rgb_colour(rgb: [u8; 3]) -> Hsla {
    art::hex(u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2]))
}

fn luminance(rgb: [u8; 3]) -> f32 {
    (0.299 * rgb[0] as f32 + 0.587 * rgb[1] as f32 + 0.114 * rgb[2] as f32) / 255.0
}

/// An offer as it would be drawn: on a slip of paper, on its own patch of
/// ground, by the same hand as the place.
/// One thing that could be made, as a tile of the build card: drawn as it
/// would stand, its name, and what it costs; greyed, with the reason in
/// its place, when it cannot be made now.
pub(crate) fn offer_tile(
    id: SharedString,
    label: &str,
    caption: String,
    available: bool,
    chosen: bool,
    shape: MarkShape,
    art: Option<String>,
) -> Stateful<Div> {
    let spoken = if caption.is_empty() {
        label.to_string()
    } else if available {
        format!("{label}, {caption}")
    } else {
        format!("{label}: {}: {caption}", ui::t("Not now"))
    };
    let key = label.to_string();
    div()
        .id(id)
        .role(Role::ListItem)
        .aria_label(spoken)
        .aria_selected(chosen)
        .w(px(OFFER_W))
        .p_1()
        .rounded_lg()
        .border_1()
        .border_color(if chosen {
            color(tokens::ACCENT)
        } else {
            gpui::transparent_black().into()
        })
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .when(available, |tile| {
            tile.cursor_pointer()
                .hover(|style| style.bg(color(tokens::ROW_HOVER)))
        })
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    paint_offer(window, bounds, shape, &key, art.as_deref(), available)
                },
            )
            .w(px(OFFER_W - 8.0))
            .h(px(70.0)),
        )
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(color(if available {
                    tokens::TEXT
                } else {
                    tokens::TEXT_TERTIARY
                }))
                .text_center()
                .line_height(relative(1.3))
                .child(label.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(color(if available {
                    tokens::TEXT_TERTIARY
                } else {
                    tokens::WARNING
                }))
                .text_center()
                .line_height(relative(1.3))
                .child(caption),
        )
}

fn paint_offer(
    window: &mut Window,
    bounds: gpui::Bounds<Pixels>,
    shape: MarkShape,
    key: &str,
    art_key: Option<&str>,
    available: bool,
) {
    let (x, y) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    art::rect(window, x, y, w, h, 8.0, art::hex(PAPER));
    let ground = y + h * 0.84;
    window.gradient(
        x,
        ground - 2.0,
        w,
        h - (ground - 2.0 - y),
        180.0,
        (art::hex(0xcfd8b8), 0.0),
        (art::hex(0xb9c79c), 1.0),
    );
    use crate::brush::Brush;
    window.soft(
        x + w / 2.0,
        ground,
        w * 0.3,
        3.0,
        4.0,
        gpui::black().opacity(0.18),
    );
    let mut palette = art::Palette::of(key, false);
    if let Some(art) = art_key.and_then(crate::works::Art::from_key) {
        palette.art = Some(art);
    }
    let small = !matches!(
        shape,
        MarkShape::House
            | MarkShape::Dome
            | MarkShape::Tower
            | MarkShape::Tree
            | MarkShape::Shop
            | MarkShape::Bridge
            | MarkShape::Lamp
    );
    let size = if small { h * 0.62 } else { h * 0.7 };
    art::paint_building(window, x + w / 2.0, ground, size, size, shape, &palette);
    // What cannot be made now is drawn under a veil of the paper.
    if !available {
        art::rect(window, x, y, w, h, 8.0, art::hex(PAPER).opacity(0.55));
    }
}

/// The grid of a design: each square its colour on the paper, hairlines
/// between, a stronger line every four, and a ring on the keys' square.
fn paint_grid(
    window: &mut Window,
    (x0, y0): (f32, f32),
    cell: f32,
    cells: &[u8],
    colours: &[u8; SLOTS],
    cursor: (usize, usize),
) {
    use crate::brush::Brush;
    for (at, slot) in cells.iter().enumerate() {
        let (column, row) = (at % SIDE, at / SIDE);
        let rgb = PALETTE[colours[*slot as usize % SLOTS] as usize];
        window.rect(
            x0 + column as f32 * cell,
            y0 + row as f32 * cell,
            cell + 0.5,
            cell + 0.5,
            0.0,
            rgb_colour(rgb),
        );
    }
    let side = cell * SIDE as f32;
    let ink = art::hex(INK);
    for line in 1..SIDE {
        let at = line as f32 * cell;
        let strength = if line % 4 == 0 { 0.2 } else { 0.08 };
        window.rect(x0 + at - 0.5, y0, 1.0, side, 0.0, ink.opacity(strength));
        window.rect(x0, y0 + at - 0.5, side, 1.0, 0.0, ink.opacity(strength));
    }
    let (column, row) = cursor;
    let (cx, cy) = (x0 + column as f32 * cell, y0 + row as f32 * cell);
    let ring = color(tokens::ACCENT).into();
    let white: Hsla = gpui::white().opacity(0.9);
    for (inset, colour, thick) in [(-1.5, white, 3.5), (-0.5, ring, 2.0)] {
        let (x, y, s) = (cx + inset, cy + inset, cell - inset * 2.0);
        window.rect(x, y, s, thick, 0.0, colour);
        window.rect(x, y + s - thick, s, thick, 0.0, colour);
        window.rect(x, y, thick, s, 0.0, colour);
        window.rect(x + s - thick, y, thick, s, 0.0, colour);
    }
}

/// A tool's small picture: a pencil, a paint pot tipping, a dropper.
fn tool_glyph(tool: Tool, on: bool) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            use crate::brush::Brush;
            let ink: Hsla = if on {
                color(tokens::ACCENT_TEXT).into()
            } else {
                color(tokens::TEXT_SECONDARY).into()
            };
            let (x, y) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let s = f32::from(bounds.size.width);
            let p = |u: f32, v: f32| (x + u * s, y + v * s);
            match tool {
                Tool::Pencil => {
                    art::polygon(
                        window,
                        &[p(0.72, 0.1), p(0.9, 0.28), p(0.36, 0.82), p(0.18, 0.64)],
                        ink,
                    );
                    art::polygon(
                        window,
                        &[p(0.18, 0.64), p(0.36, 0.82), p(0.1, 0.9)],
                        ink.opacity(0.6),
                    );
                }
                Tool::Fill => {
                    art::polygon(
                        window,
                        &[p(0.12, 0.44), p(0.46, 0.12), p(0.8, 0.46), p(0.46, 0.8)],
                        ink,
                    );
                    window.soft(x + s * 0.84, y + s * 0.74, s * 0.08, s * 0.12, 1.0, ink);
                }
                Tool::Eyedropper => {
                    art::line(window, p(0.2, 0.8), p(0.62, 0.38), s * 0.12, ink);
                    art::circle(window, x + s * 0.72, y + s * 0.28, s * 0.16, ink);
                    art::circle(
                        window,
                        x + s * 0.16,
                        y + s * 0.84,
                        s * 0.07,
                        ink.opacity(0.6),
                    );
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// At every size a player might have the window, the canvas fits
    /// inside it, its squares are big enough to hit, and in a window wide
    /// enough it keeps clear of what the design is for.
    #[test]
    fn the_design_canvas_fits_every_window_and_keeps_clear_of_its_target() {
        for (width, height) in [
            (800.0, 600.0),
            (1024.0, 700.0),
            (1440.0, 900.0),
            (2560.0, 1440.0),
        ] {
            for target in [width * 0.2, width * 0.8] {
                let layout = design_layout(width, height, Some(target));
                assert!(
                    layout.x >= 0.0 && layout.x + layout.w <= width,
                    "{width}x{height}: {layout:?}"
                );
                assert!(
                    layout.y >= 0.0 && layout.y + layout.h <= height,
                    "{width}x{height}: {layout:?}"
                );
                assert!(
                    layout.cell >= 10.0,
                    "{width}x{height}: squares of {}",
                    layout.cell
                );
                if width >= 1024.0 {
                    assert!(layout.beside);
                    assert!(
                        target < layout.x || target > layout.x + layout.w,
                        "{width}x{height}: the canvas covers its target: {layout:?}"
                    );
                    assert!(layout.y >= TOP, "{width}x{height}: under the gauges");
                    assert!(
                        layout.cell >= 16.0,
                        "{width}x{height}: squares of {}",
                        layout.cell
                    );
                }
            }
        }
        // The biggest window draws the biggest squares, up to a limit.
        let big = design_layout(2560.0, 1440.0, None);
        assert_eq!(big.cell, 28.0);
        // A narrow window stacks the colours under the grid, and still fits.
        let narrow = design_layout(600.0, 900.0, None);
        assert!(!narrow.beside);
        assert!(narrow.x >= 0.0 && narrow.x + narrow.w <= 600.0 && narrow.y + narrow.h <= 900.0);
        assert!(narrow.cell >= 16.0, "{narrow:?}");
    }

    /// Every word the player's mark shows is in the app's Chinese catalog.
    #[test]
    fn every_word_of_the_mark_is_translated() {
        let catalog = world_i18n::Catalog::parse(crate::i18n::APP_ZH_HANS);
        let mut words = vec![
            "A plot",
            "What could stand here",
            "What could stand here?",
            "Design…",
            "Name…",
            "Name",
            "More in the drawer",
            "Names proposed",
            "Enter to name it · Esc to leave it",
            "Type a name, or pick one · Enter to name it",
            "A name…",
            "The design, sixteen squares by sixteen",
            "Arrows move · Space paints · 1–8 colours · F fills",
            "Cancel",
            "Save",
            "Colours",
            "Colour",
            "The palette",
            "Tools",
            "Mix the chosen colour",
            "Undo",
            "Redo",
            "Clear",
            "Close",
        ];
        words.extend(Tool::ALL.map(Tool::name));
        words.extend([Wear::Flag, Wear::Sail, Wear::Sign, Wear::Quilt].map(Wear::title));
        words.extend(PALETTE_NAMES);
        for word in words {
            assert!(catalog.exact(word).is_some(), "no zh-Hans for {word:?}");
        }
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;
    use gpui::VisualTestContext;
    use std::cell::RefCell;
    use world_projection::{CanvasItemKind, Designable, Naming, Plot, Wears};

    /// A World that keeps what it was asked, and never changes.
    struct Recording(ProjectionSnapshot, Rc<RefCell<Vec<ProjectionIntent>>>);

    impl crate::ProjectionController for Recording {
        fn snapshot(&self) -> ProjectionSnapshot {
            self.0.clone()
        }

        fn handle(&mut self, intent: ProjectionIntent) -> Result<ProjectionSnapshot, String> {
            self.1.borrow_mut().push(intent);
            Ok(self.0.clone())
        }
    }

    fn id(n: u64) -> SelectionId {
        SelectionId::from_stable_key(&format!("entity-{n}")).unwrap()
    }

    /// A quay with a boat that can wear a design and take a name, and a
    /// plot with three things that could stand on it, one not yet.
    fn harbour() -> ProjectionSnapshot {
        let mut snapshot = ProjectionSnapshot {
            title: "Harbour".into(),
            ..ProjectionSnapshot::default()
        };
        snapshot.canvas.items.push(CanvasItem {
            id: id(1),
            kind: CanvasItemKind::Place,
            label: "Quay".into(),
            x: 0.3,
            y: 0.5,
            ..Default::default()
        });
        snapshot.canvas.items.push(CanvasItem {
            id: id(2),
            kind: CanvasItemKind::Object,
            label: "The boat".into(),
            shape: Some(MarkShape::Boat),
            at: Some(id(1)),
            design: Some(Designable {
                command: "boat.design".into(),
                wears: Wears::Sail,
            }),
            naming: Some(Naming {
                command: "boat.name".into(),
                proposals: vec!["Lark".into(), "小海燕".into()],
            }),
            ..Default::default()
        });
        let offer = |command: &str, label: &str, shape, unavailable: Option<&str>| PlotOffer {
            command: command.into(),
            label: label.into(),
            shape,
            cost: Some("12 coins".into()),
            unavailable: unavailable.map(Into::into),
            art: None,
        };
        snapshot.canvas.plots.push(Plot {
            id: "plot-1".into(),
            px: 0.72,
            row: 0,
            district: String::new(),
            offers: vec![
                offer("build.bench", "A bench", MarkShape::Bench, None),
                offer("build.well", "A well", MarkShape::Well, None),
                offer(
                    "build.statue",
                    "A statue",
                    MarkShape::Statue,
                    Some("Needs stone"),
                ),
            ],
        });
        snapshot
    }

    fn open(
        cx: &mut gpui::TestAppContext,
    ) -> (
        gpui::WindowHandle<ProjectionView>,
        &mut VisualTestContext,
        Rc<RefCell<Vec<ProjectionIntent>>>,
    ) {
        let sent = Rc::new(RefCell::new(Vec::new()));
        let world = Recording(harbour(), sent.clone());
        let window = cx.add_window(move |_, _| {
            let mut view = ProjectionView::controlled(Recording(world.0.clone(), world.1.clone()));
            view.looking.opening = None;
            view
        });
        let cx = VisualTestContext::from_window(window.into(), cx).into_mut();
        cx.simulate_resize(gpui::size(px(1100.0), px(800.0)));
        cx.run_until_parked();
        (window, cx, sent)
    }

    fn sent_commands(sent: &Rc<RefCell<Vec<ProjectionIntent>>>) -> Vec<String> {
        sent.borrow()
            .iter()
            .filter_map(|intent| match intent {
                ProjectionIntent::InvokeCommand(command) => Some(command.clone()),
                _ => None,
            })
            .collect()
    }

    #[gpui::test]
    fn a_plot_is_chosen_opened_and_built_on_with_the_keys(cx: &mut gpui::TestAppContext) {
        let (window, cx, sent) = open(cx);
        cx.simulate_keystrokes("tab");
        window
            .read_with(cx, |view, _| {
                assert_eq!(view.looking.marking.plot_focus, Some(0))
            })
            .unwrap();
        cx.simulate_keystrokes("enter");
        window
            .read_with(cx, |view, _| {
                assert_eq!(view.looking.marking.offers, Some(("plot-1".into(), 0)))
            })
            .unwrap();
        // What cannot be built yet is passed over by Enter.
        cx.simulate_keystrokes("left enter");
        assert!(sent_commands(&sent).is_empty());
        cx.simulate_keystrokes("left enter");
        assert_eq!(sent_commands(&sent), ["build.well"]);
        window
            .read_with(cx, |view, _| assert_eq!(view.looking.marking.offers, None))
            .unwrap();
        // Escape puts the plot's card away without building.
        cx.simulate_keystrokes("tab enter escape");
        window
            .read_with(cx, |view, _| assert_eq!(view.looking.marking.offers, None))
            .unwrap();
        assert_eq!(sent_commands(&sent).len(), 1);
    }

    #[gpui::test]
    fn the_design_canvas_answers_its_keys_and_saves_the_design(cx: &mut gpui::TestAppContext) {
        let (window, cx, sent) = open(cx);
        window
            .update(cx, |view, _, cx| view.start_design(id(2), Wear::Sail, cx))
            .unwrap();
        cx.run_until_parked();
        let sketch = |cx: &mut VisualTestContext| {
            window
                .read_with(cx, |view, _| {
                    view.looking
                        .marking
                        .design
                        .as_ref()
                        .map(|d| d.sketch.clone())
                })
                .unwrap()
        };
        // From the middle: one to the right, and paint it.
        cx.simulate_keystrokes("right space");
        assert_eq!(sketch(cx).unwrap().at(9, 8), 1);
        // Colour 3, back one, and fill everything joined to it.
        cx.simulate_keystrokes("3 left f");
        let now = sketch(cx).unwrap();
        assert_eq!((now.at(0, 0), now.at(9, 8)), (2, 1));
        cx.simulate_keystrokes("cmd-z");
        assert_eq!(sketch(cx).unwrap().at(0, 0), 0);
        cx.simulate_keystrokes("shift-cmd-z");
        assert_eq!(sketch(cx).unwrap().at(0, 0), 2);
        // Shift and an arrow paints on the way.
        cx.simulate_keystrokes("8 shift-down");
        let made = sketch(cx).unwrap();
        assert_eq!(made.at(8, 9), 7);
        cx.simulate_keystrokes("enter");
        let commands = sent_commands(&sent);
        assert_eq!(commands.len(), 1, "{commands:?}");
        let (command, argument) = world_projection::command_argument(&commands[0]);
        assert_eq!(command, "boat.design");
        let design = world_projection::Design::parse(argument.unwrap()).unwrap();
        assert_eq!(mark::Motif::of(&design.pattern()), Some(made.motif()));
        // Opened again and let go of with Escape, nothing is sent.
        window
            .update(cx, |view, _, cx| view.start_design(id(2), Wear::Sail, cx))
            .unwrap();
        cx.simulate_keystrokes("space escape");
        assert!(sketch(cx).is_none());
        assert_eq!(sent_commands(&sent).len(), 1);
    }

    #[gpui::test]
    fn a_name_is_given_with_enter_but_never_while_it_is_being_composed(
        cx: &mut gpui::TestAppContext,
    ) {
        use gpui::EntityInputHandler;
        let (window, cx, sent) = open(cx);
        window
            .update(cx, |view, window, cx| {
                view.open_mark_card(id(2), cx);
                view.start_naming(id(2), window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        cx.simulate_input("Petrel");
        // An input method composing a word: Enter is the composing's.
        window
            .update(cx, |view, window, cx| {
                let (_, input) = view.looking.marking.naming.clone().unwrap();
                input.update(cx, |input, cx| {
                    input.replace_and_mark_text_in_range(None, "海", None, window, cx)
                });
            })
            .unwrap();
        cx.simulate_keystrokes("enter");
        assert!(sent_commands(&sent).is_empty());
        // That Enter committed the word; the next gives the name.
        window
            .read_with(cx, |view, cx| {
                let (_, input) = view.looking.marking.naming.clone().unwrap();
                assert!(!input.read(cx).composing());
                assert_eq!(input.read(cx).text(), "Petrel海");
            })
            .unwrap();
        cx.simulate_keystrokes("enter");
        assert_eq!(sent_commands(&sent), ["boat.name=Petrel海"]);
        // A key typed while naming is a letter, not the scene's shortcut.
        window
            .update(cx, |view, window, cx| view.start_naming(id(2), window, cx))
            .unwrap();
        cx.simulate_keystrokes("p escape");
        window
            .read_with(cx, |view, _| {
                assert!(view.looking.marking.naming.is_none());
                assert!(!view.looking.photographing);
            })
            .unwrap();
    }
}
