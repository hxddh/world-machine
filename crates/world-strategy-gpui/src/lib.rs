use gpui::{div, prelude::*, px, rgb, Context, Div, Render, Styled, Window};
use world_compare::{DifferenceKind, EntityDifference, SnapshotComparison};
use world_strategy::{StrategyEvaluation, StrategyRun};

pub struct StrategyComparisonView {
    evaluation: StrategyEvaluation,
    left_label: String,
    right_label: String,
}

impl StrategyComparisonView {
    pub fn new(
        evaluation: StrategyEvaluation,
        left_label: impl Into<String>,
        right_label: impl Into<String>,
    ) -> Self {
        Self {
            evaluation,
            left_label: left_label.into(),
            right_label: right_label.into(),
        }
    }

    fn render_run(&self, label: &str, run: &StrategyRun) -> Div {
        match run {
            StrategyRun::Success(outcome) => div()
                .w(px(320.0))
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xcfd8c8))
                .bg(rgb(0xf7faf5))
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_xs().text_color(rgb(0x66705f)).child(label.to_string()))
                .child(div().text_lg().child(outcome.snapshot.title.clone()))
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x555555))
                        .child(format!("World time {}", outcome.snapshot.world_time)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x777777))
                        .child(if outcome.archive.is_some() {
                            "Durable result"
                        } else {
                            "Ephemeral result"
                        }),
                ),
            StrategyRun::Failure(error) => div()
                .w(px(320.0))
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xe2bcbc))
                .bg(rgb(0xfff6f6))
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_xs().text_color(rgb(0x8b5555)).child(label.to_string()))
                .child(div().text_lg().child("Strategy failed"))
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x8b5555))
                        .child(format!("{:?}", error.stage)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x777777))
                        .child(error.source.to_string()),
                ),
        }
    }

    fn render_comparison(&self, comparison: &SnapshotComparison) -> Div {
        let timeline_changes = comparison.timeline.left_only.len()
            + comparison.timeline.right_only.len()
            + comparison.timeline.changed.len();
        let command_changes = comparison.commands.left_only.len()
            + comparison.commands.right_only.len()
            + comparison.commands.changed.len();

        let mut entities = div().flex().flex_col().gap_2();
        for difference in comparison.entities.iter().take(10) {
            entities = entities.child(self.render_entity_difference(difference));
        }
        if comparison.entities.is_empty() {
            entities = entities.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x777777))
                    .child("No entity state differences"),
            );
        }

        let mut timeline = div().flex().flex_col().gap_1();
        for item in comparison.timeline.left_only.iter().take(4) {
            timeline = timeline.child(
                div()
                    .text_xs()
                    .child(format!("Left only · t={} · {}", item.world_time, item.title)),
            );
        }
        for item in comparison.timeline.right_only.iter().take(4) {
            timeline = timeline.child(
                div()
                    .text_xs()
                    .child(format!("Right only · t={} · {}", item.world_time, item.title)),
            );
        }
        for item in comparison.timeline.changed.iter().take(4) {
            timeline = timeline.child(
                div().text_xs().child(format!(
                    "Changed · {} → {}",
                    item.left.title, item.right.title
                )),
            );
        }
        if timeline_changes == 0 {
            timeline = timeline.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child("No timeline differences"),
            );
        }

        div()
            .w(px(520.0))
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd7dce8))
            .bg(rgb(0xf8f9fc))
            .flex()
            .flex_col()
            .gap_4()
            .child(div().text_lg().child("What changed"))
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(summary_chip("Entities", comparison.entities.len()))
                    .child(summary_chip("Timeline", timeline_changes))
                    .child(summary_chip("Commands", command_changes)),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Left t={}", comparison.left.world_time)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Right t={}", comparison.right.world_time)),
                    ),
            )
            .child(div().text_sm().child("Entity state"))
            .child(entities)
            .child(div().text_sm().child("Timeline"))
            .child(timeline)
    }

    fn render_entity_difference(&self, difference: &EntityDifference) -> Div {
        let title = difference
            .left
            .as_ref()
            .map(|entity| entity.title.clone())
            .or_else(|| difference.right.as_ref().map(|entity| entity.title.clone()))
            .unwrap_or_else(|| difference.id.stable_key());

        let mut rows = div().flex().flex_col().gap_1();
        for row in difference.inspector_rows.iter().take(6) {
            rows = rows.child(
                div()
                    .flex()
                    .gap_2()
                    .text_xs()
                    .child(
                        div()
                            .w(px(150.0))
                            .text_color(rgb(0x666666))
                            .child(row.key.label.clone()),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .child(row.left.clone().unwrap_or_else(|| "—".into())),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .child(row.right.clone().unwrap_or_else(|| "—".into())),
                    ),
            );
        }

        div()
            .p_3()
            .rounded_md()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(div().text_sm().child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(difference_kind_label(difference.kind)),
                    ),
            )
            .child(rows)
    }
}

impl Render for StrategyComparisonView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div()
            .w_full()
            .h_full()
            .p_5()
            .flex()
            .flex_col()
            .gap_4()
            .bg(rgb(0xf3f4f2))
            .child(div().text_xl().child("Strategy Comparison"))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("Two independent futures evaluated from the same durable World"),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(self.render_run(&self.left_label, &self.evaluation.left))
                    .child(self.render_run(&self.right_label, &self.evaluation.right)),
            );

        body = if let Some(comparison) = self.evaluation.comparison.as_ref() {
            body.child(self.render_comparison(comparison))
        } else {
            body.child(
                div()
                    .w(px(520.0))
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xe2bcbc))
                    .bg(rgb(0xfff8f8))
                    .child("Comparison unavailable because one or both strategies failed."),
            )
        };

        body
    }
}

fn summary_chip(label: &str, count: usize) -> Div {
    div()
        .p_2()
        .rounded_md()
        .bg(rgb(0xe9edf5))
        .text_sm()
        .child(format!("{label}: {count}"))
}

fn difference_kind_label(kind: DifferenceKind) -> &'static str {
    match kind {
        DifferenceKind::LeftOnly => "Left only",
        DifferenceKind::RightOnly => "Right only",
        DifferenceKind::Changed => "Changed",
    }
}
