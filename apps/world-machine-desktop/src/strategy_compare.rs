use super::{SharedDocument, WorldDocumentView};
use gpui::{px, size, AppContext, Bounds, Context, WindowBounds, WindowOptions};
use std::sync::Arc;
use world_strategy::{evaluate_strategies, StrategyPlan};
use world_strategy_gpui::StrategyComparisonView;

const COMPARISON_BACKGROUND_PERIODS: u64 = 20;

pub(crate) fn can_compare(document: &SharedDocument) -> bool {
    document.borrow().session.snapshot().commands.len() >= 2
}

pub(crate) fn open_first_two(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<(String, String), String> {
    let (source, registry, left_id, left_label, right_id, right_label) = {
        let document = document.borrow();
        let snapshot = document.session.snapshot();
        let mut commands = snapshot.commands.iter();
        let left = commands
            .next()
            .ok_or_else(|| "This World has no comparable choices".to_string())?;
        let right = commands
            .next()
            .ok_or_else(|| "This World needs at least two choices to compare".to_string())?;
        let source = document
            .session
            .current_archive()
            .map_err(|error| error.to_string())?;
        (
            source,
            Arc::clone(&document.registry),
            left.id.clone(),
            left.title.clone(),
            right.id.clone(),
            right.title.clone(),
        )
    };

    let evaluation = evaluate_strategies(
        &registry,
        &source,
        &StrategyPlan::new()
            .command(left_id)
            .background_periods(COMPARISON_BACKGROUND_PERIODS),
        &StrategyPlan::new()
            .command(right_id)
            .background_periods(COMPARISON_BACKGROUND_PERIODS),
    );

    let window_left = left_label.clone();
    let window_right = right_label.clone();
    let bounds = Bounds::centered(None, size(px(1220.0), px(920.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| {
            cx.new(|_| StrategyComparisonView::new(evaluation, window_left, window_right))
        },
    )
    .map_err(|error| error.to_string())?;

    Ok((left_label, right_label))
}
