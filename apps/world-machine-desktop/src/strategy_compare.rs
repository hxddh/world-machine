use super::{SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, Div, Styled, WindowBounds,
    WindowOptions,
};
use world_strategy_document::{available_choices, evaluate_first_two};
use world_strategy_gpui::StrategyComparisonView;

const COMPARISON_BACKGROUND_PERIODS: u64 = 20;

pub(crate) fn document_actions(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Div {
    let mut actions = div().flex().gap_2();
    if available_choices(&document.borrow().session).len() >= 2 {
        actions = actions.child(
            div()
                .id("compare-world-choices")
                .cursor_pointer()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0x9eb0d6))
                .bg(rgb(0xf4f7ff))
                .text_sm()
                .child("Compare choices…")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.status = Some(match open_first_two(&this.document, cx) {
                        Ok((left, right)) => format!("Comparing {left} vs {right}"),
                        Err(error) => format!("Compare failed: {error}"),
                    });
                    cx.notify();
                })),
        );
    }

    actions
        .child(
            div()
                .id("save-as-world-document")
                .cursor_pointer()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xcacac4))
                .bg(rgb(0xffffff))
                .text_sm()
                .child("Save As…")
                .on_click(cx.listener(|this, _, _, cx| this.save_as(cx))),
        )
        .child(
            div()
                .id("reload-world-document")
                .cursor_pointer()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xcacac4))
                .bg(rgb(0xffffff))
                .text_sm()
                .child("Reload from disk")
                .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
        )
}

fn open_first_two(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<(String, String), String> {
    let (left, right, evaluation) = {
        let document = document.borrow();
        evaluate_first_two(
            &document.session,
            &document.registry,
            COMPARISON_BACKGROUND_PERIODS,
        )
        .map_err(|error| error.to_string())?
    };

    let left_label = left.title;
    let right_label = right.title;
    let window_left = left_label.clone();
    let window_right = right_label.clone();
    let bounds = Bounds::centered(None, size(px(1220.0), px(920.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| cx.new(|_| StrategyComparisonView::new(evaluation, window_left, window_right)),
    )
    .map_err(|error| error.to_string())?;

    Ok((left_label, right_label))
}
