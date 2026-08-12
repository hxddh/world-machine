#[cfg(target_os = "macos")]
use std::sync::Arc;

#[cfg(target_os = "macos")]
struct DemoController {
    library: Arc<world_library::WorldLibrary>,
    registry: Arc<world_host::WorldRegistry>,
}

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

    fn can_compare(&self) -> bool {
        true
    }

    fn compare_documents(
        &mut self,
        left: &str,
        right: &str,
        cx: &mut gpui::Context<world_lineage_gpui::LineageExplorerView>,
    ) -> Result<(), String> {
        use gpui::{px, size, AppContext, Bounds, WindowBounds, WindowOptions};
        use world_library::WorldDocumentId;
        use world_lineage_compare::compare_saved_worlds;
        use world_strategy_gpui::StrategyComparisonView;

        let left_id = WorldDocumentId::new(left.to_owned()).map_err(|error| error.to_string())?;
        let right_id = WorldDocumentId::new(right.to_owned()).map_err(|error| error.to_string())?;
        let result = compare_saved_worlds(&self.library, &self.registry, &left_id, &right_id)
            .map_err(|error| error.to_string())?;
        let left_label = result.left.document.to_string();
        let right_label = result.right.document.to_string();
        let bounds = Bounds::centered(None, size(px(1220.0), px(920.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|_| {
                    StrategyComparisonView::saved(
                        result.left.snapshot,
                        result.right.snapshot,
                        result.comparison,
                        left_label,
                        right_label,
                    )
                })
            },
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui::{px, size, App, AppContext, Bounds, WindowBounds, WindowOptions};
    use gpui_platform::application;
    use std::env;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};
    use world_document::{WorldBranchCause, WorldDocument, WorldLineage, WorldParent};
    use world_host::{HostError, WorldDescriptor, WorldRegistration, WorldRegistry, WorldSession};
    use world_library::{WorldDocumentId, WorldLibrary};
    use world_lineage::LineageIndex;
    use world_lineage_gpui::LineageExplorerView;
    use world_persistence::{
        WorldArchive, WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION,
    };
    use world_projection::{ProjectionIntent, ProjectionSnapshot};

    const PACK_ID: &str = "world-machine.lineage-demo";

    struct DemoSession {
        world_time: u64,
    }

    impl WorldSession for DemoSession {
        fn pack(&self) -> WorldPackRef {
            WorldPackRef::new(PACK_ID, "1")
        }

        fn snapshot(&self) -> ProjectionSnapshot {
            ProjectionSnapshot {
                title: format!("Lineage demo at t={}", self.world_time),
                world_time: self.world_time,
                ..Default::default()
            }
        }

        fn handle(&mut self, _intent: ProjectionIntent) -> Result<ProjectionSnapshot, HostError> {
            Ok(self.snapshot())
        }
    }

    fn archive(world_time: u64) -> WorldArchive {
        WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new(PACK_ID, "1"),
            world_time,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    fn lineage(parent: &str, parent_time: u64, title: &str) -> WorldLineage {
        WorldLineage {
            parent: WorldParent {
                document: Some(parent.into()),
                pack: WorldPackRef::new(PACK_ID, "1"),
                world_time: parent_time,
                event_count: 0,
            },
            branch: WorldBranchCause::Strategy {
                choice_id: format!("demo.{title}"),
                choice_title: title.into(),
                horizon: 20,
            },
        }
    }

    fn save(
        library: &WorldLibrary,
        id: &str,
        world_time: u64,
        lineage: Option<WorldLineage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let id = WorldDocumentId::new(id.to_owned())?;
        let mut document = WorldDocument::new(archive(world_time));
        document.metadata.lineage = lineage;
        library.create_from_document(id, &document)?;
        Ok(())
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!(
        "world-machine-lineage-acceptance-{}-{nonce}",
        process::id()
    ));
    let library = Arc::new(WorldLibrary::new(root));
    save(&library, "source", 0, None)?;
    save(
        &library,
        "future-a",
        20,
        Some(lineage("source", 0, "Traditional reopen")),
    )?;
    save(
        &library,
        "future-b",
        35,
        Some(lineage("source", 0, "Lean owner-run reopen")),
    )?;
    save(
        &library,
        "future-b-long",
        80,
        Some(lineage("future-b", 35, "Continue lean")),
    )?;

    let mut registry = WorldRegistry::new();
    registry.register(
        WorldRegistration::new(
            WorldDescriptor {
                pack: WorldPackRef::new(PACK_ID, "1"),
                title: "Lineage Demo".into(),
                description: "Saved World comparison acceptance fixture".into(),
            },
            || Ok(Box::new(DemoSession { world_time: 0 })),
        )
        .with_archive_opener(|archive| {
            Ok(Box::new(DemoSession {
                world_time: archive.world_time,
            }))
        }),
    )?;
    let registry = Arc::new(registry);
    let index = LineageIndex::from_library(&library)?;
    let controller = DemoController {
        library: Arc::clone(&library),
        registry: Arc::clone(&registry),
    };

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1120.0), px(820.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| LineageExplorerView::controlled(index, controller)),
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
