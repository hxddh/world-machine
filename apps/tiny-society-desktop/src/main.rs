#[cfg(target_os = "macos")]
mod session_store;

#[cfg(target_os = "macos")]
struct TinySocietyController {
    branch: tiny_society::TinySocietyBranch,
    store: session_store::SessionStore,
    initial_cursor: Option<tiny_society::VisitCursor>,
}

#[cfg(target_os = "macos")]
impl TinySocietyController {
    fn persist_branch(&self, branch: &tiny_society::TinySocietyBranch) -> Result<(), String> {
        let json = branch.archive_json().map_err(|error| error.to_string())?;
        self.store
            .save(&json)
            .map_err(|error| format!("failed to save {}: {error}", self.store.path().display()))
    }

    fn mark_current_visit(&self) {
        let _ = self
            .store
            .save_visit_cursor(self.branch.visit_cursor().event_count);
    }
}

#[cfg(target_os = "macos")]
impl world_gpui::ProjectionController for TinySocietyController {
    fn snapshot(&self) -> world_gpui::ProjectionSnapshot {
        match self.initial_cursor {
            Some(cursor) => self.branch.projection_snapshot_since(cursor),
            None => self.branch.projection_snapshot(),
        }
    }

    fn handle(
        &mut self,
        intent: world_gpui::ProjectionIntent,
    ) -> Result<world_gpui::ProjectionSnapshot, String> {
        let mut candidate = self.branch.clone();
        match intent {
            world_gpui::ProjectionIntent::ForkBeforeEvent(event) => {
                candidate
                    .fork_before_event(event)
                    .map_err(|error| error.to_string())?;
            }
            world_gpui::ProjectionIntent::InvokeCommand(command_id) => {
                candidate
                    .invoke_projection_command(&command_id)
                    .map_err(|error| error.to_string())?;
            }
        }
        self.persist_branch(&candidate)?;
        self.branch = candidate;
        self.initial_cursor = None;
        self.mark_current_visit();
        Ok(self.branch.projection_snapshot())
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui::{px, size, App, AppContext, Bounds, WindowBounds, WindowOptions};
    use gpui_platform::application;
    use tiny_society::{TinySociety, VisitCursor};
    use world_gpui::ProjectionView;

    let store = session_store::SessionStore::discover()?;
    let previous_cursor = store.load_visit_cursor()?.map(VisitCursor::new);
    let society = match store.load()? {
        Some(json) => TinySociety::resume_json(&json)?,
        None => {
            let mut society = TinySociety::new()?;
            society.run_story()?;
            store.save(&society.archive_json()?)?;
            society
        }
    };
    let current_cursor = society.visit_cursor();
    let _ = store.save_visit_cursor(current_cursor.event_count);
    let controller = TinySocietyController {
        branch: society.branch(),
        store,
        initial_cursor: previous_cursor,
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
