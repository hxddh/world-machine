use gpui::{div, prelude::*, px, rgb, Context, Div, Render, Styled, Window};
use world_compare::{DifferenceKind, EntityDifference, SnapshotComparison};
use world_projection::{BriefingProjection, ProjectionSnapshot};
use world_strategy::{StrategyEvaluation, StrategyRun};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SavedComparisonContext {
    pub relation: Option<String>,
    pub left_provenance: Option<String>,
    pub right_provenance: Option<String>,
}

const FUTURE_STORY_ITEM_LIMIT: usize = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FutureStory {
    title: Option<String>,
    items: Vec<(String, String)>,
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
            StrategyRun::Success(outcome) => {
                let mut card = div()
                    .w(px(500.0))
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
                    ));
                if let Some(story) = render_future_story(&outcome.snapshot) {
                    card = card.child(story);
                }
                card
            }
            StrategyRun::Failure(error) => div()
                .w(px(500.0))
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
            .w(px(500.0))
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
        if let Some(story) = render_future_story(snapshot) {
            card = card.child(story);
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
            timeline = timeline.child(div().text_xs().child(format!(
                "Left only · t={} · {}",
                item.world_time, item.title
            )));
        }
        for item in comparison.timeline.right_only.iter().take(4) {
            timeline = timeline.child(div().text_xs().child(format!(
                "Right only · t={} · {}",
                item.world_time, item.title
            )));
        }
        for item in comparison.timeline.changed.iter().take(4) {
            timeline = timeline.child(div().text_xs().child(format!(
                "Changed · {} → {}",
                item.left.title, item.right.title
            )));
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

fn future_story(briefing: Option<&BriefingProjection>) -> Option<FutureStory> {
    let briefing = briefing?;
    let title = (!briefing.title.trim().is_empty()).then(|| briefing.title.trim().to_owned());
    let items = briefing
        .items
        .iter()
        .filter_map(|item| {
            let title = item.title.trim();
            let detail = item.detail.trim();
            (!title.is_empty() || !detail.is_empty()).then(|| (title.to_owned(), detail.to_owned()))
        })
        .take(FUTURE_STORY_ITEM_LIMIT)
        .collect::<Vec<_>>();
    (title.is_some() || !items.is_empty()).then_some(FutureStory { title, items })
}

fn render_future_story(snapshot: &ProjectionSnapshot) -> Option<Div> {
    let story = future_story(snapshot.briefing.as_ref())?;
    let mut body = div()
        .mt_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xdfe3dc))
        .bg(rgb(0xffffff))
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x777770))
                .child("Future story"),
        );
    if let Some(title) = story.title {
        body = body.child(div().text_sm().child(title));
    }
    for (title, detail) in story.items {
        let mut item = div().flex().flex_col().gap_1();
        if !title.is_empty() {
            item = item.child(div().text_sm().child(title));
        }
        if !detail.is_empty() {
            item = item.child(div().text_xs().text_color(rgb(0x666666)).child(detail));
        }
        body = body.child(item);
    }
    Some(body)
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
    use world_projection::BriefingItem;

    fn briefing(title: &str, count: usize) -> BriefingProjection {
        BriefingProjection {
            eyebrow: "Test".into(),
            title: title.into(),
            items: (0..count)
                .map(|index| BriefingItem {
                    selection: None,
                    title: format!("Item {index}"),
                    detail: format!("Detail {index}"),
                })
                .collect(),
        }
    }

    #[test]
    fn future_story_preserves_projection_order_and_keeps_six_items() {
        let briefing = briefing("While you were away", 7);
        let story = future_story(Some(&briefing)).unwrap();

        assert_eq!(story.title.as_deref(), Some("While you were away"));
        assert_eq!(story.items.len(), FUTURE_STORY_ITEM_LIMIT);
        assert_eq!(story.items.first().unwrap().0, "Item 0");
        assert_eq!(story.items.last().unwrap().0, "Item 5");
    }

    #[test]
    fn future_story_keeps_late_durable_context_within_rich_six_item_briefing() {
        let briefing = BriefingProjection {
            eyebrow: "Test".into(),
            title: "While you were away".into(),
            items: [
                "Recent 1",
                "Recent 2",
                "Recent 3",
                "Intervention",
                "Relationship",
                "World direction",
            ]
            .into_iter()
            .map(|title| BriefingItem {
                selection: None,
                title: title.into(),
                detail: format!("{title} detail"),
            })
            .collect(),
        };
        let story = future_story(Some(&briefing)).unwrap();

        assert_eq!(story.items.len(), 6);
        assert_eq!(story.items.last().unwrap().0, "World direction");
    }

    #[test]
    fn future_story_omits_an_empty_briefing() {
        let briefing = BriefingProjection {
            eyebrow: String::new(),
            title: "  ".into(),
            items: vec![BriefingItem {
                selection: None,
                title: " ".into(),
                detail: "".into(),
            }],
        };

        assert_eq!(future_story(Some(&briefing)), None);
        assert_eq!(future_story(None), None);
    }
}
