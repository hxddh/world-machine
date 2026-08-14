use gpui::{div, prelude::*, px, rgb, Context, Div, Render, Styled, Window};
use world_compare::{
    ChangedCommand, ChangedTimelineItem, DifferenceKind, EntityDifference, SnapshotComparison,
};
use world_projection::{ProjectionCommand, ProjectionSnapshot, TimelineItem};
use world_strategy::{StrategyEvaluation, StrategyRun};

const ENTITY_DIFFERENCE_LIMIT: usize = 10;
const TIMELINE_DIFFERENCE_LIMIT_PER_KIND: usize = 4;
const INSPECTOR_ROW_LIMIT: usize = 6;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SavedComparisonContext {
    pub relation: Option<String>,
    pub left_provenance: Option<String>,
    pub right_provenance: Option<String>,
}

enum ComparisonSource {
    Strategies(StrategyEvaluation),
    Saved {
        left: ProjectionSnapshot,
        right: ProjectionSnapshot,
        comparison: SnapshotComparison,
        context: SavedComparisonContext,
    },
}

pub struct StrategyComparisonView {
    source: ComparisonSource,
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
            source: ComparisonSource::Strategies(evaluation),
            left_label: left_label.into(),
            right_label: right_label.into(),
        }
    }

    pub fn saved(
        left: ProjectionSnapshot,
        right: ProjectionSnapshot,
        comparison: SnapshotComparison,
        left_label: impl Into<String>,
        right_label: impl Into<String>,
    ) -> Self {
        Self::saved_with_context(
            left,
            right,
            comparison,
            left_label,
            right_label,
            SavedComparisonContext::default(),
        )
    }

    pub fn saved_with_context(
        left: ProjectionSnapshot,
        right: ProjectionSnapshot,
        comparison: SnapshotComparison,
        left_label: impl Into<String>,
        right_label: impl Into<String>,
        context: SavedComparisonContext,
    ) -> Self {
        Self {
            source: ComparisonSource::Saved {
                left,
                right,
                comparison,
                context,
            },
            left_label: left_label.into(),
            right_label: right_label.into(),
        }
    }

    fn render_strategy_run(&self, label: &str, run: &StrategyRun) -> Div {
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
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x66705f))
                        .child(label.to_string()),
                )
                .child(div().text_lg().child(outcome.snapshot.title.clone()))
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0x555555))
                        .child(format!("World time {}", outcome.snapshot.world_time)),
                )
                .child(div().text_xs().text_color(rgb(0x777777)).child(
                    if outcome.archive.is_some() {
                        "Durable result"
                    } else {
                        "Ephemeral result"
                    },
                )),
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
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x8b5555))
                        .child(label.to_string()),
                )
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

    fn render_saved_side(
        &self,
        label: &str,
        snapshot: &ProjectionSnapshot,
        provenance: Option<&str>,
    ) -> Div {
        let mut card = div()
            .w(px(320.0))
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xcfd8c8))
            .bg(rgb(0xf7faf5))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x66705f))
                    .child(label.to_string()),
            )
            .child(div().text_lg().child(snapshot.title.clone()))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x555555))
                    .child(format!("World time {}", snapshot.world_time)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child("Saved World"),
            );
        if let Some(provenance) = provenance {
            card = card.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x4e6fb3))
                    .child(provenance.to_string()),
            );
        }
        card
    }

    fn render_comparison(&self, comparison: &SnapshotComparison) -> Div {
        let timeline_changes = comparison.timeline.left_only.len()
            + comparison.timeline.right_only.len()
            + comparison.timeline.changed.len();
        let command_changes = comparison.commands.left_only.len()
            + comparison.commands.right_only.len()
            + comparison.commands.changed.len();

        let mut entities = div().flex().flex_col().gap_2();
        for difference in comparison.entities.iter().take(ENTITY_DIFFERENCE_LIMIT) {
            entities = entities.child(self.render_entity_difference(difference));
        }
        if comparison.entities.is_empty() {
            entities = entities.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x777777))
                    .child("No entity state differences"),
            );
        } else if let Some(notice) = hidden_notice(
            comparison.entities.len(),
            ENTITY_DIFFERENCE_LIMIT,
            "entity differences",
        ) {
            entities = entities.child(truncation_notice(notice));
        }

        let mut timeline = div().flex().flex_col().gap_2();
        for item in comparison
            .timeline
            .left_only
            .iter()
            .take(TIMELINE_DIFFERENCE_LIMIT_PER_KIND)
        {
            timeline = timeline.child(self.render_timeline_item("Left only", item));
        }
        for item in comparison
            .timeline
            .right_only
            .iter()
            .take(TIMELINE_DIFFERENCE_LIMIT_PER_KIND)
        {
            timeline = timeline.child(self.render_timeline_item("Right only", item));
        }
        for item in comparison
            .timeline
            .changed
            .iter()
            .take(TIMELINE_DIFFERENCE_LIMIT_PER_KIND)
        {
            timeline = timeline.child(self.render_changed_timeline_item(item));
        }
        if timeline_changes == 0 {
            timeline = timeline.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child("No timeline differences"),
            );
        } else {
            let hidden_timeline = hidden_after_group_limits(
                &[
                    comparison.timeline.left_only.len(),
                    comparison.timeline.right_only.len(),
                    comparison.timeline.changed.len(),
                ],
                TIMELINE_DIFFERENCE_LIMIT_PER_KIND,
            );
            if hidden_timeline > 0 {
                timeline = timeline.child(truncation_notice(format!(
                    "{hidden_timeline} more timeline differences not shown"
                )));
            }
        }

        let mut commands = div().flex().flex_col().gap_2();
        for command in &comparison.commands.left_only {
            commands = commands
                .child(self.render_command(&format!("Left only · {}", self.left_label), command));
        }
        for command in &comparison.commands.right_only {
            commands = commands
                .child(self.render_command(&format!("Right only · {}", self.right_label), command));
        }
        for command in &comparison.commands.changed {
            commands = commands.child(self.render_changed_command(command));
        }
        if command_changes == 0 {
            commands = commands.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child("No available-action differences"),
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
            .child(div().text_sm().child("Available actions"))
            .child(commands)
    }

    fn render_timeline_item(&self, relation: &str, item: &TimelineItem) -> Div {
        let mut card = div()
            .p_3()
            .rounded_md()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(div().text_sm().child(item.title.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(format!("{relation} · t={}", item.world_time)),
                    ),
            );
        if let Some(detail) = timeline_detail(&item.subtitle) {
            card = card.child(div().text_xs().text_color(rgb(0x555555)).child(detail));
        }
        card
    }

    fn render_changed_timeline_item(&self, item: &ChangedTimelineItem) -> Div {
        let title = if item.left.title == item.right.title {
            item.left.title.clone()
        } else {
            format!("{} → {}", item.left.title, item.right.title)
        };
        let left_detail =
            timeline_detail(&item.left.subtitle).unwrap_or_else(|| "No detail".into());
        let right_detail =
            timeline_detail(&item.right.subtitle).unwrap_or_else(|| "No detail".into());

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
                    .justify_between()
                    .gap_2()
                    .child(div().text_sm().child(title))
                    .child(div().text_xs().text_color(rgb(0x777777)).child("Changed")),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_xs().text_color(rgb(0x4e6fb3)).child(format!(
                        "Left · {} · t={}",
                        self.left_label, item.left.world_time
                    )))
                    .child(div().text_xs().child(left_detail)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_xs().text_color(rgb(0x4e6fb3)).child(format!(
                        "Right · {} · t={}",
                        self.right_label, item.right.world_time
                    )))
                    .child(div().text_xs().child(right_detail)),
            )
    }

    fn render_command(&self, relation: &str, command: &ProjectionCommand) -> Div {
        div()
            .p_3()
            .rounded_md()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(div().text_sm().child(command.title.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(relation.to_string()),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x555555))
                    .child(command.detail.clone()),
            )
    }

    fn render_changed_command(&self, command: &ChangedCommand) -> Div {
        let title = if command.left.title == command.right.title {
            command.left.title.clone()
        } else {
            format!("{} → {}", command.left.title, command.right.title)
        };

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
                    .justify_between()
                    .gap_2()
                    .child(div().text_sm().child(title))
                    .child(div().text_xs().text_color(rgb(0x777777)).child("Changed")),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x4e6fb3))
                            .child(format!("Left · {}", self.left_label)),
                    )
                    .child(div().text_xs().child(command.left.detail.clone())),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x4e6fb3))
                            .child(format!("Right · {}", self.right_label)),
                    )
                    .child(div().text_xs().child(command.right.detail.clone())),
            )
    }

    fn render_entity_difference(&self, difference: &EntityDifference) -> Div {
        let title = difference
            .left
            .as_ref()
            .map(|entity| entity.title.clone())
            .or_else(|| difference.right.as_ref().map(|entity| entity.title.clone()))
            .unwrap_or_else(|| difference.id.stable_key());

        let mut rows = div().flex().flex_col().gap_1();
        if !difference.inspector_rows.is_empty() {
            rows = rows.child(
                div()
                    .flex()
                    .gap_2()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(div().w(px(150.0)).child("Field"))
                    .child(div().w(px(140.0)).child(self.left_label.clone()))
                    .child(div().w(px(140.0)).child(self.right_label.clone())),
            );
        }
        for row in difference.inspector_rows.iter().take(INSPECTOR_ROW_LIMIT) {
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
        if let Some(notice) = hidden_notice(
            difference.inspector_rows.len(),
            INSPECTOR_ROW_LIMIT,
            "changed fields",
        ) {
            rows = rows.child(truncation_notice(notice));
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

    fn heading(&self) -> (&'static str, &'static str) {
        match &self.source {
            ComparisonSource::Strategies(_) => (
                "Strategy Comparison",
                "Two independent futures evaluated from the same durable World",
            ),
            ComparisonSource::Saved { .. } => (
                "Saved World Comparison",
                "Two saved Worlds compared at their current durable state",
            ),
        }
    }
}

impl Render for StrategyComparisonView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (title, subtitle) = self.heading();
        let mut body = div()
            .id("strategy-comparison-scroll")
            .w_full()
            .h_full()
            .overflow_y_scroll()
            .p_5()
            .flex()
            .flex_col()
            .gap_4()
            .bg(rgb(0xf3f4f2))
            .child(div().text_xl().child(title))
            .child(div().text_sm().text_color(rgb(0x666666)).child(subtitle));

        if let ComparisonSource::Saved { context, .. } = &self.source {
            if let Some(relation) = &context.relation {
                body = body.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .bg(rgb(0xe9edf5))
                        .text_sm()
                        .child(format!("Lineage relation · {relation}")),
                );
            }
        }

        body = match &self.source {
            ComparisonSource::Strategies(evaluation) => body.child(
                div()
                    .flex()
                    .gap_4()
                    .child(self.render_strategy_run(&self.left_label, &evaluation.left))
                    .child(self.render_strategy_run(&self.right_label, &evaluation.right)),
            ),
            ComparisonSource::Saved {
                left,
                right,
                context,
                ..
            } => body.child(
                div()
                    .flex()
                    .gap_4()
                    .child(self.render_saved_side(
                        &self.left_label,
                        left,
                        context.left_provenance.as_deref(),
                    ))
                    .child(self.render_saved_side(
                        &self.right_label,
                        right,
                        context.right_provenance.as_deref(),
                    )),
            ),
        };

        let comparison = match &self.source {
            ComparisonSource::Strategies(evaluation) => evaluation.comparison.as_ref(),
            ComparisonSource::Saved { comparison, .. } => Some(comparison),
        };
        body = if let Some(comparison) = comparison {
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

fn timeline_detail(subtitle: &str) -> Option<String> {
    let detail = subtitle.trim();
    (!detail.is_empty()).then(|| detail.to_owned())
}

fn hidden_notice(total: usize, limit: usize, noun: &str) -> Option<String> {
    let hidden = total.saturating_sub(limit);
    (hidden > 0).then(|| format!("{hidden} more {noun} not shown"))
}

fn hidden_after_group_limits(counts: &[usize], limit: usize) -> usize {
    counts.iter().map(|count| count.saturating_sub(limit)).sum()
}

fn truncation_notice(message: String) -> Div {
    div().text_xs().text_color(rgb(0x777777)).child(message)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_timeline_detail_is_kept_for_comparison_ui() {
        assert_eq!(
            timeline_detail("Outward became the durable posture. · Event #7"),
            Some("Outward became the durable posture. · Event #7".into())
        );
    }

    #[test]
    fn blank_timeline_detail_stays_absent() {
        assert_eq!(timeline_detail("   "), None);
    }

    #[test]
    fn hidden_notice_only_reports_truncated_items() {
        assert_eq!(hidden_notice(10, 10, "items"), None);
        assert_eq!(
            hidden_notice(13, 10, "items"),
            Some("3 more items not shown".into())
        );
    }

    #[test]
    fn grouped_limits_count_every_hidden_item() {
        assert_eq!(hidden_after_group_limits(&[7, 2, 5], 4), 4);
    }
}
