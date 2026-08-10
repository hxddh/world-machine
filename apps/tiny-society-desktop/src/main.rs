#[cfg(target_os = "macos")]
mod session_store;

#[cfg(target_os = "macos")]
struct TinySocietyController {
    branch: tiny_society::TinySocietyBranch,
    store: session_store::SessionStore,
}

#[cfg(target_os = "macos")]
impl TinySocietyController {
    fn persist(&self) -> Result<(), String> {
        let json = self
            .branch
            .archive_json()
            .map_err(|error| error.to_string())?;
        self.store.save(&json).map_err(|error| {
            format!(
                "failed to save {}: {error}",
                self.store.path().display()
            )
        })
    }
}

#[cfg(target_os = "macos")]
impl world_gpui::ProjectionController for TinySocietyController {
    fn snapshot(&self) -> world_gpui::ProjectionSnapshot {
        self.branch.projection_snapshot()
    }

    fn handle(
        &mut self,
        intent: world_gpui::ProjectionIntent,
    ) -> Result<world_gpui::ProjectionSnapshot, String> {
        match intent {
            world_gpui::ProjectionIntent::ForkBeforeEvent(event) => {
                self.branch
                    .fork_before_event(event)
                    .map_err(|error| error.to_string())?;
            }
            world_gpui::ProjectionIntent::InvokeCommand(command_id) => {
                self.branch
                    .invoke_projection_command(&command_id)
                    .map_err(|error| error.to_string())?;
            }
        }
        self.persist()?;
        Ok(self.branch.projection_snapshot())
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui::{px, size, App, AppContext, Bounds, WindowBounds, WindowOptions};
    use gpui_platform::application;
    use tiny_society::TinySociety;
    use world_gpui::ProjectionView;

    let store = session_store::SessionStore::discover()?;
    let society = match store.load()? {
        Some(json) => TinySociety::resume_json(&json)?,
        None => {
            let mut society = TinySociety::new()?;
            society.run_story()?;
            store.save(&society.archive_json()?)?;
            society
        }
    };
    let controller = TinySocietyController {
        branch: society.branch(),
        store,
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
        .expect("failed to open Tiny Society window");
        cx.activate(true);
    });

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!(
        "tiny-society-desktop currently targets macOS; the projection layer is cross-platform"
    );
}
