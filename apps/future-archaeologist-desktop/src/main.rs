#[cfg(target_os = "macos")]
struct FutureArchaeologistController {
    world: future_archaeologist::FutureArchaeologist,
}

#[cfg(target_os = "macos")]
impl world_gpui::ProjectionController for FutureArchaeologistController {
    fn snapshot(&self) -> world_gpui::ProjectionSnapshot {
        self.world.projection_snapshot()
    }

    fn handle(
        &mut self,
        intent: world_gpui::ProjectionIntent,
    ) -> Result<world_gpui::ProjectionSnapshot, String> {
        match intent {
            world_gpui::ProjectionIntent::InvokeCommand(command_id) => {
                self.world
                    .invoke_projection_command(&command_id)
                    .map_err(|error| error.to_string())?;
            }
            world_gpui::ProjectionIntent::ForkBeforeEvent(_) => {
                return Err("this fixed-truth world does not support timeline forks".into());
            }
        }
        Ok(self.world.projection_snapshot())
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui::{px, size, App, AppContext, Bounds, WindowBounds, WindowOptions};
    use gpui_platform::application;
    use world_gpui::ProjectionView;

    let controller = FutureArchaeologistController {
        world: future_archaeologist::FutureArchaeologist::new()?,
    };

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| ProjectionView::controlled(controller)),
        )
        .expect("failed to open Future Archaeologist window");
        cx.activate(true);
    });

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!(
        "future-archaeologist-desktop currently targets macOS; the projection layer is cross-platform"
    );
}
