#[cfg(target_os = "macos")]
fn main() {
    use gpui::{px, size, App, AppContext, Bounds, WindowBounds, WindowOptions};
    use gpui_platform::application;
    use world_document::{WorldBranchCause, WorldLineage, WorldParent};
    use world_library::WorldDocumentId;
    use world_lineage::{build_index, LineageRecord};
    use world_lineage_gpui::LineageTreeView;
    use world_persistence::WorldPackRef;

    let pack = WorldPackRef::new("world-machine.lineage-demo", "1");
    let root = LineageRecord {
        id: WorldDocumentId::new("source-world").unwrap(),
        pack: pack.clone(),
        world_time: 20,
        event_count: 6,
        lineage: None,
    };
    let future_a = strategy_record("future-a", "source-world", "Choose A", 20, pack.clone());
    let future_b = strategy_record(
        "future-b",
        "source-world.world",
        "Choose B",
        100,
        pack.clone(),
    );
    let detached = strategy_record(
        "detached-future",
        "External.world",
        "External choice",
        20,
        pack,
    );
    let index = build_index([root, future_a, future_b, detached]).unwrap();

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(980.0), px(760.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| LineageTreeView::new(index.clone())),
        )
        .expect("open lineage window");
        cx.activate(true);
    });

    fn strategy_record(
        id: &str,
        parent: &str,
        title: &str,
        horizon: u64,
        pack: WorldPackRef,
    ) -> LineageRecord {
        LineageRecord {
            id: WorldDocumentId::new(id).unwrap(),
            pack: pack.clone(),
            world_time: 40,
            event_count: 12,
            lineage: Some(WorldLineage {
                parent: WorldParent {
                    document: Some(parent.into()),
                    pack,
                    world_time: 20,
                    event_count: 6,
                },
                branch: WorldBranchCause::Strategy {
                    choice_id: format!("demo.{id}"),
                    choice_title: title.into(),
                    horizon,
                },
            }),
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The lineage Library acceptance app currently targets macOS.");
}
