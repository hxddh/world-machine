from pathlib import Path

path = Path('crates/world-strategy-gpui/src/lib.rs')
text = path.read_text()

old = '''use world_projection::ProjectionSnapshot;
'''
new = '''use world_projection::{BriefingProjection, ProjectionSnapshot};
'''
assert text.count(old) == 1
text = text.replace(old, new, 1)

old = '''enum ComparisonSource {
'''
new = '''const FUTURE_STORY_ITEM_LIMIT: usize = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FutureStory {
    title: Option<String>,
    items: Vec<(String, String)>,
}

enum ComparisonSource {
'''
assert text.count(old) == 1
text = text.replace(old, new, 1)

old = '''    fn render_strategy_run(&self, label: &str, run: &StrategyRun) -> Div {
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
'''
new = '''    fn render_strategy_run(&self, label: &str, run: &StrategyRun) -> Div {
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
'''
assert text.count(old) == 1
text = text.replace(old, new, 1)

old = '''        let mut card = div()
            .w(px(320.0))
'''
new = '''        let mut card = div()
            .w(px(500.0))
'''
assert text.count(old) == 1
text = text.replace(old, new, 1)

old = '''        if let Some(provenance) = provenance {
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
'''
new = '''        if let Some(provenance) = provenance {
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
'''
assert text.count(old) == 1
text = text.replace(old, new, 1)

marker = '''fn summary_chip(label: &str, count: usize) -> Div {
'''
assert text.count(marker) == 1
helpers = r'''fn future_story(briefing: Option<&BriefingProjection>) -> Option<FutureStory> {
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

'''
text = text.replace(marker, helpers + marker, 1)

# Add pure projection-contract tests without depending on any World implementation.
text += r'''

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
'''

path.write_text(text)
