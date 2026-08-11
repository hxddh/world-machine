#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui::{px, size, App, AppContext, Bounds, WindowBounds, WindowOptions};
    use gpui_platform::application;
    use tiny_society::{
        tiny_society_registration, TinySociety, LEAN_REOPEN_BAKERY_COMMAND,
        REOPEN_BAKERY_COMMAND,
    };
    use world_host::WorldRegistry;
    use world_strategy::{evaluate_strategies, StrategyPlan};
    use world_strategy_gpui::StrategyComparisonView;

    let mut society = TinySociety::new()?;
    society.run_story()?;
    let mut branch = society.branch();
    branch.advance_days(120)?;
    let source = branch.archive()?;

    let mut registry = WorldRegistry::new();
    registry.register(tiny_society_registration())?;
    let evaluation = evaluate_strategies(
        &registry,
        &source,
        &StrategyPlan::new()
            .command(REOPEN_BAKERY_COMMAND)
            .background_periods(20),
        &StrategyPlan::new()
            .command(LEAN_REOPEN_BAKERY_COMMAND)
            .background_periods(20),
    );

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1220.0), px(920.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|_| {
                    StrategyComparisonView::new(
                        evaluation,
                        "Traditional reopen",
                        "Lean owner-run reopen",
                    )
                })
            },
        )
        .expect("failed to open Strategy Comparison window");
        cx.activate(true);
    });

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!(
        "strategy-comparison-desktop currently targets macOS; strategy evaluation is headless"
    );
}
