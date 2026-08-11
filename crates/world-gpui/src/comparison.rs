use crate::ProjectionView;
use gpui::{
    div, prelude::*, px, rgb, Context, Entity, FontWeight, IntoElement, Render, Window,
};
use world_compare::{compare_snapshots, DifferenceKind, EntityDifference, SnapshotComparison};
use world_projection::ProjectionSnapshot;

const TEXT_PRIMARY: gpui::Rgba = rgb(0x1f2933);
const TEXT_MUTED: gpui::Rgba = rgb(0x667085);
const BORDER: gpui::Rgba = rgb(0xdfe3e8);
const SURFACE: gpui::Rgba = rgb(0xffffff);
const SURFACE_MUTED: gpui::Rgba = rgb(0xf7f8fa);

pub struct StrategyComparisonView {
    comparison: SnapshotComparison,
    left: Entity<ProjectionView>,
    right: Entity<ProjectionView>,
}

impl StrategyComparisonView {
    pub fn new(
        left_snapshot: ProjectionSnapshot,
        right_snapshot: ProjectionSnapshot,
        cx: &mut Context<Self>,
    ) -> Self {
        let comparison = compare_snapshots(&left_snapshot, &right_snapshot);
        let left = cx.new(|_| ProjectionView::new(left_snapshot));
        let right = cx.new(|_| ProjectionView::new(right_snapshot));
        Self {
            comparison,
            left,
            right,
        }
    }

    pub fn comparison(&self) -> &SnapshotComparison {
        &self.comparison
    }
}

impl Render for StrategyComparisonView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .bg(SURFACE_MUTED)
            .child(comparison_header(&self.comparison))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .gap_3()
                    .p_3()
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .rounded_lg()
                            .border_1()
                            .border_color(BORDER)
                            .bg(SURFACE)
                            .child(self.left.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .rounded_lg()
                            .border_1()
                            .border_color(BORDER)
                            .bg(SURFACE)
                            .child(self.right.clone()),
                    ),
            )
    }
}

fn comparison_header(comparison: &SnapshotComparison) -> impl IntoElement {
    let divergent_events = comparison.timeline.left_only.len()
        + comparison.timeline.right_only.len()
        + comparison.timeline.changed.len();
    let command_differences = comparison.commands.left_only.len()
        + comparison.commands.right_only.len()
        + comparison.commands.changed.len();

    div()
        .flex()
        .flex_col()
        .gap_3()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(BORDER)
        .bg(SURFACE)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(TEXT_PRIMARY)
                                .child("Strategy comparison"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(TEXT_MUTED)
                                .child(format!(
                                    "{} · time {}  ↔  {} · time {}",
                                    comparison.left.title,
                                    comparison.left.world_time,
                                    comparison.right.title,
                                    comparison.right.world_time
                                )),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(metric_chip(
                            "Changed entities",
                            comparison.entities.len(),
                        ))
                        .child(metric_chip("Divergent events", divergent_events))
                        .child(metric_chip("Command differences", command_differences)),
                ),
        )
        .child(entity_difference_strip(comparison))
}

fn metric_chip(label: &'static str, value: usize) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(SURFACE_MUTED)
        .border_1()
        .border_color(BORDER)
        .text_sm()
        .text_color(TEXT_MUTED)
        .child(
            div()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(TEXT_PRIMARY)
                .child(value.to_string()),
        )
        .child(label)
}

fn entity_difference_strip(comparison: &SnapshotComparison) -> impl IntoElement {
    let rows = comparison
        .entities
        .iter()
        .take(4)
        .map(entity_difference_card)
        .collect::<Vec<_>>();

    if rows.is_empty() {
        return div()
            .text_sm()
            .text_color(TEXT_MUTED)
            .child("No visible entity-state differences")
            .into_any_element();
    }

    div().flex().gap_2().children(rows).into_any_element()
}

fn entity_difference_card(difference: &EntityDifference) -> impl IntoElement {
    let title = difference
        .left
        .as_ref()
        .or(difference.right.as_ref())
        .map(|view| view.title.clone())
        .unwrap_or_else(|| format!("{:?}", difference.id));
    let detail = difference_detail(difference);
    let kind = match difference.kind {
        DifferenceKind::LeftOnly => "Left only",
        DifferenceKind::RightOnly => "Right only",
        DifferenceKind::Changed => "Changed",
    };

    div()
        .flex()
        .flex_col()
        .gap_1()
        .min_w(px(180.))
        .max_w(px(320.))
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(BORDER)
        .bg(SURFACE_MUTED)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(TEXT_PRIMARY)
                        .child(title),
                )
                .child(div().text_xs().text_color(TEXT_MUTED).child(kind)),
        )
        .child(div().text_xs().text_color(TEXT_MUTED).child(detail))
}

fn difference_detail(difference: &EntityDifference) -> String {
    if difference.inspector_rows.is_empty() {
        return match difference.kind {
            DifferenceKind::LeftOnly => "Visible only in the left strategy".into(),
            DifferenceKind::RightOnly => "Visible only in the right strategy".into(),
            DifferenceKind::Changed => "Visible entity metadata changed".into(),
        };
    }

    difference
        .inspector_rows
        .iter()
        .take(3)
        .map(|row| {
            let left = row.left.as_deref().unwrap_or("—");
            let right = row.right.as_deref().unwrap_or("—");
            format!("{}: {left} → {right}", row.key.label)
        })
        .collect::<Vec<_>>()
        .join(" · ")
}
