#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui::{App, Bounds, WindowBounds, WindowOptions, px, size};
    use gpui_platform::application;
    use tiny_society::TinySociety;
    use world_gpui::ProjectionView;

    let mut society = TinySociety::new()?;
    society.run_story()?;
    let snapshot = society.projection_snapshot();

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.0), px(760.0)), cx);
        let snapshot = snapshot.clone();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| {
                let snapshot = snapshot.clone();
                cx.new(|_| ProjectionView::new(snapshot))
            },
        )
        .expect("failed to open Tiny Society window");
        cx.activate(true);
    });

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("tiny-society-desktop currently targets macOS; the projection layer is cross-platform");
}
