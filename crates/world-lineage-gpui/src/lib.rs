use gpui::{
    canvas, div, point, prelude::*, px, relative, Bounds, Context, Div, IntoElement, PathBuilder,
    Pixels, Render, SharedString, Styled, Window,
};
use world_document::WorldBranchCause;
use world_gpui::ui;
use world_library::WorldDocumentId;
use world_lineage::{LineageIndex, LineageNode};
use world_theme::tokens;

pub trait LineageController {
    fn open_document(
        &mut self,
        document: &str,
        cx: &mut Context<LineageExplorerView>,
    ) -> Result<(), String>;

    /// Return a non-fatal notice produced by the last successful open, if any.
    /// Controllers can use this for degraded initialization such as observer
    /// catch-up failures without teaching the generic lineage UI about Host or
    /// Library details.
    fn take_open_notice(&mut self) -> Option<String> {
        None
    }

    fn can_compare(&self) -> bool {
        false
    }

    fn compare_documents(
        &mut self,
        _left: &str,
        _right: &str,
        _cx: &mut Context<LineageExplorerView>,
    ) -> Result<(), String> {
        Err("This view cannot compare saved Worlds".into())
    }
}

pub struct LineageExplorerView {
    index: LineageIndex,
    selected: Option<String>,
    compare_from: Option<String>,
    controller: Option<Box<dyn LineageController>>,
    status: Option<String>,
}

impl LineageExplorerView {
    pub fn new(index: LineageIndex) -> Self {
        Self::with_controller(index, None, None)
    }

    pub fn controlled<C>(index: LineageIndex, controller: C) -> Self
    where
        C: LineageController + 'static,
    {
        Self::with_controller(index, None, Some(Box::new(controller)))
    }

    pub fn controlled_selected<C>(
        index: LineageIndex,
        selected: impl Into<String>,
        controller: C,
    ) -> Self
    where
        C: LineageController + 'static,
    {
        Self::with_controller(index, Some(selected.into()), Some(Box::new(controller)))
    }

    fn with_controller(
        index: LineageIndex,
        selected: Option<String>,
        controller: Option<Box<dyn LineageController>>,
    ) -> Self {
        let selected = selected
            .filter(|selected| {
                index
                    .nodes()
                    .values()
                    .any(|node| node.id.as_str() == selected)
            })
            .or_else(|| index.roots().first().map(ToString::to_string))
            .or_else(|| index.nodes().keys().next().map(ToString::to_string));
        Self {
            index,
            selected,
            compare_from: None,
            controller,
            status: None,
        }
    }

    fn node_by_label(&self, label: &str) -> Option<&LineageNode> {
        self.index
            .nodes()
            .values()
            .find(|node| node.id.as_str() == label)
    }

    fn open_selected(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<(String, Option<String>), String> {
        let document = self
            .selected
            .clone()
            .ok_or_else(|| "Select a World before opening it".to_string())?;
        let controller = self
            .controller
            .as_mut()
            .ok_or_else(|| "This view cannot open Worlds".to_string())?;
        controller.open_document(&document, cx)?;
        let notice = controller.take_open_notice();
        Ok((document, notice))
    }

    fn mark_comparison_source(&mut self) -> Result<String, String> {
        let document = self
            .selected
            .clone()
            .ok_or_else(|| "Select a World before starting comparison".to_string())?;
        self.compare_from = Some(document.clone());
        Ok(document)
    }

    fn compare_selected(&mut self, cx: &mut Context<Self>) -> Result<(String, String), String> {
        let left = self
            .compare_from
            .clone()
            .ok_or_else(|| "Choose a comparison source first".to_string())?;
        let right = self
            .selected
            .clone()
            .ok_or_else(|| "Select another World before comparing".to_string())?;
        if left == right {
            return Err("Choose a different World for the right side".into());
        }
        let controller = self
            .controller
            .as_mut()
            .ok_or_else(|| "This view cannot compare saved Worlds".to_string())?;
        if !controller.can_compare() {
            return Err("This view cannot compare saved Worlds".into());
        }
        controller.compare_documents(&left, &right, cx)?;
        Ok((left, right))
    }

    fn select(&mut self, label: String, cx: &mut Context<Self>) {
        self.selected = Some(label);
        self.status = None;
        cx.notify();
    }

    /// Every World as a lane on one time axis: the original across the top,
    /// each branch leaving its parent at the moment it split and running on
    /// to where it is now. The way a git graph or Time Machine shows history,
    /// rather than an indented list of file names.
    fn render_graph(&self, cx: &mut Context<Self>) -> Div {
        let lanes = lanes(&self.index);
        let latest = lanes
            .iter()
            .map(|lane| lane.end.max(lane.start))
            .max()
            .unwrap_or(1)
            .max(1);
        let height = GRAPH_TOP + ROW * lanes.len() as f32 + 12.0;
        let selected = self
            .selected
            .as_ref()
            .and_then(|selected| lanes.iter().position(|lane| &lane.id == selected));

        // The selected World and everything it descends from, so its whole
        // history reads as one highlighted path, as in a git graph.
        let mut ancestry = std::collections::BTreeSet::new();
        let mut cursor = selected;
        while let Some(index) = cursor {
            ancestry.insert(index);
            cursor = lanes[index].parent;
        }
        let row_y = |index: usize| GRAPH_TOP + ROW * index as f32 + ROW / 2.0;
        let x_of = move |time: u64| AXIS_LEFT + AXIS_SPAN * time as f32 / latest as f32;

        let strokes = lanes
            .iter()
            .enumerate()
            .map(|(index, lane)| {
                let tone = if ancestry.contains(&index) {
                    tokens::ACCENT
                } else {
                    tokens::SCENE_LINK
                };
                (
                    x_of(lane.start),
                    x_of(lane.end.max(lane.start)),
                    row_y(index),
                    lane.parent.map(row_y),
                    hsla(tone),
                    Some(index) == selected,
                )
            })
            .collect::<Vec<_>>();
        let surface = hsla(tokens::SURFACE);

        let drawing = canvas(
            |_, _, _| (),
            move |bounds: Bounds<Pixels>, _, window, _| {
                let o = bounds.origin;
                let w = bounds.size.width;
                let at = |x: f32, y: f32| point(o.x + w * x, o.y + px(y));
                for (start, end, y, parent_y, colour, is_selected) in &strokes {
                    let width = if *is_selected { 3.5 } else { 2.0 };
                    let mut path = PathBuilder::stroke(px(width));
                    match parent_y {
                        // A branch leaves its parent at the moment it split.
                        Some(parent_y) => {
                            path.move_to(at(*start, *parent_y));
                            path.cubic_bezier_to(
                                at(*start + 0.03, *y),
                                at(*start, *y),
                                at(*start + 0.03, *parent_y),
                            );
                            path.line_to(at(*end, *y));
                        }
                        None => {
                            path.move_to(at(*start, *y));
                            path.line_to(at(*end, *y));
                        }
                    }
                    if let Ok(path) = path.build() {
                        window.paint_path(path, *colour);
                    }
                    // Where it split, and where it is now.
                    if let Some(parent_y) = parent_y {
                        dot(window, at(*start, *parent_y), 4.0, *colour, *colour);
                    }
                    let radius = if *is_selected { 8.0 } else { 6.0 };
                    dot(window, at(*end, *y), radius, surface, *colour);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let mut graph = div()
            .relative()
            .w_full()
            .h(px(height))
            .child(
                div()
                    .absolute()
                    .top(px(6.0))
                    .left(relative(AXIS_LEFT))
                    .child(ui::caption("The beginning")),
            )
            .child(
                div()
                    .absolute()
                    .top(px(6.0))
                    .left(relative(AXIS_LEFT + AXIS_SPAN))
                    .ml(px(-40.0))
                    .child(ui::caption(format!("Time {latest}"))),
            )
            .child(drawing);

        for (index, lane) in lanes.iter().enumerate() {
            let label = lane.id.clone();
            let is_selected = Some(index) == selected;
            let marked = self.compare_from.as_deref() == Some(lane.id.as_str());
            let mut name = div()
                .flex()
                .items_center()
                .gap_2()
                .child(ui::row_title(lane.title.clone()).truncate());
            if marked {
                name = name.child(
                    div()
                        .px_2()
                        .rounded_full()
                        .bg(ui::color(tokens::ACCENT_SOFT))
                        .text_xs()
                        .text_color(ui::color(tokens::ACCENT_TEXT))
                        .child("Comparing"),
                );
            }
            graph = graph.child(
                div()
                    .id(SharedString::from(format!("lineage-node-{}", lane.id)))
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(row_y(index) - ROW / 2.0))
                    .h(px(ROW))
                    .cursor_pointer()
                    .rounded_md()
                    .when(is_selected, |row| {
                        row.bg(ui::color(tokens::ACCENT_SOFT).opacity(0.45))
                    })
                    .hover(|row| row.bg(ui::color(tokens::ACCENT_SOFT).opacity(0.3)))
                    .child(
                        div()
                            .absolute()
                            .left(relative(AXIS_LEFT + AXIS_SPAN))
                            .ml(px(22.0))
                            .right_0()
                            .top(px(ROW / 2.0 - 18.0))
                            .flex()
                            .flex_col()
                            .child(name)
                            .child(ui::caption(lane.caption.clone()).truncate()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.select(label.clone(), cx))),
            );
        }
        graph
    }

    fn render_detail(&self, cx: &mut Context<Self>) -> Div {
        let Some(label) = self.selected.as_deref() else {
            return detail_shell().child(ui::body("Select a World to see where it came from."));
        };
        let Some(node) = self.node_by_label(label) else {
            return detail_shell().child(ui::body("That World is no longer in your library."));
        };

        let origin = match (&node.parent, &node.branch) {
            (None, _) => "The original World: everything else here branched from it.".to_string(),
            (Some(parent), branch) => {
                let from = parent
                    .resolved
                    .as_ref()
                    .and_then(|id| self.index.node(id))
                    .map(node_title)
                    .or_else(|| parent.document.clone())
                    .unwrap_or_else(|| "a World outside your library".into());
                let how = match branch {
                    Some(WorldBranchCause::Strategy {
                        choice_title,
                        horizon,
                        ..
                    }) => {
                        format!(", choosing “{choice_title}” and living {horizon} periods on")
                    }
                    Some(WorldBranchCause::Fork { label: Some(label) }) => format!(" ({label})"),
                    _ => String::new(),
                };
                format!("Branched from {from} at time {}{how}.", parent.world_time)
            }
        };
        let children = node.children.len();
        let branches = match children {
            0 => "Nothing has branched from it yet.".to_string(),
            1 => "One World has branched from it.".to_string(),
            n => format!("{n} Worlds have branched from it."),
        };

        let mut actions = div().flex().flex_wrap().gap_2();
        if self.controller.is_some() {
            actions = actions.child(
                ui::button("open-lineage-world", "Open", ui::ButtonKind::Primary).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.status = match this.open_selected(cx) {
                            Ok((_, Some(notice))) => Some(notice),
                            Ok((_, None)) => None,
                            Err(error) => Some(format!("Could not open it: {error}")),
                        };
                        cx.notify();
                    }),
                ),
            );
        }
        if self
            .controller
            .as_ref()
            .is_some_and(|controller| controller.can_compare())
        {
            match &self.compare_from {
                Some(left) if left != label => {
                    let left_title = self
                        .node_by_label(left)
                        .map(node_title)
                        .unwrap_or_else(|| left.clone());
                    actions = actions.child(
                        ui::button(
                            "compare-lineage-worlds",
                            format!("Compare with {left_title}"),
                            ui::ButtonKind::Secondary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.status = match this.compare_selected(cx) {
                                Ok(_) => None,
                                Err(error) => Some(format!("Could not compare: {error}")),
                            };
                            cx.notify();
                        })),
                    );
                }
                _ => {
                    actions = actions.child(
                        ui::button(
                            "mark-lineage-comparison-source",
                            "Compare with…",
                            ui::ButtonKind::Secondary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.status = match this.mark_comparison_source() {
                                Ok(_) => Some("Now select the World to compare it with.".into()),
                                Err(error) => Some(format!("Could not start comparing: {error}")),
                            };
                            cx.notify();
                        })),
                    );
                }
            }
        }

        let mut detail = detail_shell()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_shrink_0()
                            .size(px(44.0))
                            .rounded_lg()
                            .overflow_hidden()
                            .child(ui::cover(&node_title(node), node.id.as_str()).size_full()),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(ui::heading(node_title(node)).truncate())
                            .child(ui::caption(format!(
                                "Time {} · {} things have happened",
                                node.world_time, node.event_count
                            ))),
                    ),
            )
            .child(ui::body(origin))
            .child(ui::detail(branches))
            .child(actions);
        if let Some(status) = &self.status {
            detail = detail.child(ui::detail(status.clone()));
        }
        detail
    }
}

impl Render for LineageExplorerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_window_title("Branches — World Machine");

        let count = self.index.nodes().len();
        let summary = match (count, self.index.detached().len()) {
            (1, _) => "One World, not yet branched.".to_string(),
            (n, 0) => format!("{n} Worlds, and how they branched from each other."),
            (n, d) => format!("{n} Worlds; {d} came from Worlds that are no longer here."),
        };

        div()
            .size_full()
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .flex()
            .flex_col()
            .child(
                div()
                    .px_6()
                    .pt_6()
                    .pb_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::page_title("Branches"))
                    .child(ui::body(summary)),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .w_full()
                    .flex()
                    .gap_4()
                    .px_6()
                    .pb_6()
                    .child(
                        div()
                            .id("lineage-graph-scroll")
                            .flex_1()
                            .min_w(px(0.0))
                            .h_full()
                            .overflow_y_scroll()
                            .p_4()
                            .rounded_xl()
                            .border_1()
                            .border_color(ui::color(tokens::BORDER))
                            .bg(ui::color(tokens::SURFACE))
                            .child(self.render_graph(cx)),
                    )
                    .child(self.render_detail(cx)),
            )
    }
}

const ROW: f32 = 56.0;
const GRAPH_TOP: f32 = 30.0;
/// Where the time axis starts and how much of the width it spans; the rest
/// of the width, to its right, holds each World's name.
const AXIS_LEFT: f32 = 0.03;
const AXIS_SPAN: f32 = 0.52;

/// One World as the graph draws it.
#[derive(Clone, Debug, PartialEq)]
struct Lane {
    id: String,
    title: String,
    caption: String,
    /// The lane it branched from, when that World is in the library.
    parent: Option<usize>,
    /// When it split from its parent (0 for an original World).
    start: u64,
    /// Where it is now.
    end: u64,
}

/// The lanes top to bottom: each original World followed by its branches,
/// depth first, so a branch always sits below the World it came from.
fn lanes(index: &LineageIndex) -> Vec<Lane> {
    fn visit(
        index: &LineageIndex,
        id: &WorldDocumentId,
        parent: Option<usize>,
        lanes: &mut Vec<Lane>,
    ) {
        let Some(node) = index.node(id) else {
            return;
        };
        let start = node
            .parent
            .as_ref()
            .map(|parent| parent.world_time)
            .unwrap_or(0);
        lanes.push(Lane {
            id: node.id.to_string(),
            title: node_title(node),
            caption: branch_label(node.branch.as_ref()),
            parent,
            start,
            end: node.world_time,
        });
        let own = lanes.len() - 1;
        for child in &node.children {
            visit(index, child, Some(own), lanes);
        }
    }
    let mut lanes = Vec::new();
    for root in index.roots() {
        visit(index, root, None, &mut lanes);
    }
    for detached in index.detached() {
        visit(index, detached, None, &mut lanes);
    }
    lanes
}

fn node_title(node: &LineageNode) -> String {
    node.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| node.id.to_string())
}

fn hsla(token: tokens::Token) -> gpui::Hsla {
    gpui::rgb(token.hex()).into()
}

fn dot(
    window: &mut Window,
    centre: gpui::Point<Pixels>,
    radius: f32,
    fill: gpui::Hsla,
    edge: gpui::Hsla,
) {
    window.paint_quad(gpui::quad(
        Bounds::new(
            point(centre.x - px(radius), centre.y - px(radius)),
            gpui::size(px(radius * 2.0), px(radius * 2.0)),
        ),
        px(radius),
        fill,
        px(2.5),
        edge,
        gpui::BorderStyle::default(),
    ));
}

fn branch_label(branch: Option<&WorldBranchCause>) -> String {
    match branch {
        None => "The original".into(),
        Some(WorldBranchCause::Strategy { choice_title, .. }) => {
            format!("If you choose: {choice_title}")
        }
        Some(WorldBranchCause::Fork { label }) => label
            .as_ref()
            .map(|label| format!("Branched · {label}"))
            .unwrap_or_else(|| "Branched".into()),
    }
}

fn detail_shell() -> Div {
    div()
        .w(px(320.0))
        .flex_shrink_0()
        .p_5()
        .rounded_xl()
        .border_1()
        .border_color(ui::color(tokens::BORDER))
        .bg(ui::color(tokens::SURFACE))
        .flex()
        .flex_col()
        .gap_3()
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_document::{WorldLineage, WorldParent};
    use world_library::WorldDocumentId;
    use world_lineage::{build_index, LineageRecord};
    use world_persistence::WorldPackRef;

    fn record(id: &str, parent: Option<&str>) -> LineageRecord {
        LineageRecord {
            id: WorldDocumentId::new(id).unwrap(),
            pack: WorldPackRef::new("world-machine.lineage-ui-test", "1"),
            world_time: 20,
            event_count: 4,
            lineage: parent.map(|parent| WorldLineage {
                parent: WorldParent {
                    document: Some(parent.into()),
                    pack: WorldPackRef::new("world-machine.lineage-ui-test", "1"),
                    world_time: 10,
                    event_count: 2,
                },
                branch: WorldBranchCause::Strategy {
                    choice_id: "test.choice".into(),
                    choice_title: "Choice".into(),
                    horizon: 20,
                },
            }),
            title: None,
        }
    }

    #[test]
    fn chooses_a_root_as_the_initial_selection() {
        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::new(index);
        assert_eq!(view.selected.as_deref(), Some("root"));
    }

    #[test]
    fn controlled_view_has_an_open_capability() {
        struct NoopController;
        impl LineageController for NoopController {
            fn open_document(
                &mut self,
                _document: &str,
                _cx: &mut Context<LineageExplorerView>,
            ) -> Result<(), String> {
                Ok(())
            }
        }

        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::controlled(index, NoopController);
        assert!(view.controller.is_some());
    }

    #[test]
    fn controlled_selected_prefers_the_requested_world() {
        struct NoopController;
        impl LineageController for NoopController {
            fn open_document(
                &mut self,
                _document: &str,
                _cx: &mut Context<LineageExplorerView>,
            ) -> Result<(), String> {
                Ok(())
            }
        }

        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::controlled_selected(index, "future", NoopController);
        assert_eq!(view.selected.as_deref(), Some("future"));
    }

    #[test]
    fn controlled_selected_falls_back_when_the_requested_world_is_missing() {
        struct NoopController;
        impl LineageController for NoopController {
            fn open_document(
                &mut self,
                _document: &str,
                _cx: &mut Context<LineageExplorerView>,
            ) -> Result<(), String> {
                Ok(())
            }
        }

        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::controlled_selected(index, "missing", NoopController);
        assert_eq!(view.selected.as_deref(), Some("root"));
    }

    #[test]
    fn comparison_capability_is_opt_in() {
        struct CompareController;
        impl LineageController for CompareController {
            fn open_document(
                &mut self,
                _document: &str,
                _cx: &mut Context<LineageExplorerView>,
            ) -> Result<(), String> {
                Ok(())
            }

            fn can_compare(&self) -> bool {
                true
            }
        }

        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::controlled(index, CompareController);
        assert!(view.controller.as_ref().unwrap().can_compare());
    }

    #[test]
    fn branch_labels_are_domain_neutral() {
        let branch = WorldBranchCause::Strategy {
            choice_id: "test.choice".into(),
            choice_title: "Choose A".into(),
            horizon: 5,
        };
        assert_eq!(branch_label(Some(&branch)), "If you choose: Choose A");
    }

    #[test]
    fn a_branch_sits_below_its_parent_and_starts_where_it_split() {
        let index = world_lineage::build_index([
            record("original", None),
            record("branch", Some("original")),
            record("other", None),
        ])
        .unwrap();
        let lanes = lanes(&index);
        let names = lanes
            .iter()
            .map(|lane| lane.id.as_str())
            .collect::<Vec<_>>();
        let original = names.iter().position(|id| *id == "original").unwrap();
        let branch = names.iter().position(|id| *id == "branch").unwrap();
        assert_eq!(branch, original + 1, "{names:?}");
        assert_eq!(lanes[branch].parent, Some(original));
        assert_eq!(lanes[branch].start, 10, "it split at its parent's time 10");
        assert_eq!(lanes[branch].end, 20);
        assert_eq!(lanes[original].start, 0);
        assert_eq!(lanes[branch].caption, "If you choose: Choice");
    }
}
