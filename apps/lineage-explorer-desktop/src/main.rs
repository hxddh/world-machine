#[cfg(target_os = "macos")]
struct DemoController;

#[cfg(target_os = "macos")]
impl world_lineage_gpui::LineageController for DemoController {
    fn open_document(
        &mut self,
        document: &str,
        _cx: &mut gpui::Context<world_lineage_gpui::LineageExplorerView>,
    ) -> Result<(), String> {
        println!("Lineage Explorer demo would open {document}");
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui::{px, size, App, AppContext, Bounds, WindowBounds, WindowOptions};
    use gpui_platform::application;
    use world_document::{WorldBranchCause, WorldLineage, WorldParent};
    use world_library::WorldDocumentId;
    use world_lineage::{build_index, LineageRecord};
    use world_lineage_gpui::LineageExplorerView;
    use world_persistence::WorldPackRef;

    fn record(id: &str, parent: Option<(&str, &str, u64)>) -> LineageRecord {
        let lineage = parent.map(|(parent, title, horizon)| WorldLineage {
            parent: WorldParent {
                document: Some(parent.into()),
                pack: WorldPackRef::new("world-machine.lineage-demo", "1"),
                world_time: 20,
                event_count: 5,
            },
            branch: WorldBranchCause::Strategy {
                choice_id: format!("demo.{id}"),
                choice_title: title.into(),
                horizon,
            },
        });
        LineageRecord {
            id: WorldDocumentId::new(id).expect("valid demo document id"),
            pack: WorldPackRef::new("world-machine.lineage-demo", "1"),
            world_time: if lineage.is_some() { 40 } else { 20 },
            event_count: if lineage.is_some() { 10 } else { 5 },
            lineage,
        }
    }

    let index = build_index([
        record("source", None),
        record("future-a", Some(("source", "Traditional reopen", 20))),
        record("future-b", Some(("source", "Lean owner-run reopen", 20))),
        record("future-b-long", Some(("future-b", "Continue lean", 100))),
        record("detached", Some(("External.world", "Imported strategy", 5))),
    ])?;

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1120.0), px(820.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| LineageExplorerView::controlled(index, DemoController)),
        )
        .expect("failed to open World Lineage window");
        cx.activate(true);
    });

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("lineage-explorer-desktop currently targets macOS; lineage indexing is headless");
}
