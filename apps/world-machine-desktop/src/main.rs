#[cfg(target_os = "macos")]
mod about;
#[cfg(target_os = "macos")]
mod build_info;
#[cfg(target_os = "macos")]
mod diagnostics;
#[cfg(target_os = "macos")]
mod included_packs;
#[cfg(target_os = "macos")]
mod observer;
#[cfg(target_os = "macos")]
mod strategy_compare;
#[cfg(target_os = "macos")]
mod system_open;
#[cfg(target_os = "macos")]
mod updates;
#[cfg(target_os = "macos")]
/// A light-palette colour adapted to the current appearance.
pub(crate) fn theme_rgb(hex: u32) -> gpui::Rgba {
    gpui::rgb(world_theme::adapt(hex))
}

/// Re-renders every window when the system switches between light and dark;
/// each window root then records the new appearance before it draws.
#[cfg(target_os = "macos")]
pub(crate) fn watch_appearance(window: &mut Window) {
    window
        .observe_window_appearance(|_, cx| cx.refresh_windows())
        .detach();
}
#[cfg(target_os = "macos")]
mod world_fork;

#[cfg(target_os = "macos")]
use gpui::{
    div, point, prelude::*, px, size, App, AppContext, Bounds, Context, Entity, Global,
    IntoElement, PathPromptOptions, PlatformDisplay, Render, SharedString, Styled, Window,
    WindowBounds, WindowOptions,
};
#[cfg(target_os = "macos")]
use std::cell::RefCell;
#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process;
#[cfg(target_os = "macos")]
use std::rc::Rc;
#[cfg(target_os = "macos")]
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
#[cfg(target_os = "macos")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(target_os = "macos")]
use world_document::WorldBranchCause;
#[cfg(target_os = "macos")]
use world_fork::analyst_input::{self, AnalystTextInput};
#[cfg(target_os = "macos")]
use world_library::{
    DurableWorldSession, LibraryError, UnreadableWorldFile, WorldDocumentId, WorldDocumentSummary,
    WorldLibrary, LEGACY_WORLD_DOCUMENT_SUFFIX, WORLD_DOCUMENT_SUFFIX,
};
#[cfg(target_os = "macos")]
use world_lineage::LineageIndex;
#[cfg(target_os = "macos")]
use world_machine_desktop::window_state::{self, StoredWindowBounds};
#[cfg(target_os = "macos")]
use world_pack_bundle::PACK_BUNDLE_SUFFIX;
#[cfg(target_os = "macos")]
use world_pack_catalog::{InstalledPack, PackAvailability, PackCatalog, PackInstallPreview};
#[cfg(target_os = "macos")]
use world_persistence::WorldPackRef;

#[cfg(target_os = "macos")]
const LIBRARY_OVERRIDE_ENV: &str = "WORLD_MACHINE_LIBRARY_DIR";
#[cfg(target_os = "macos")]
const PACK_CATALOG_OVERRIDE_ENV: &str = "WORLD_MACHINE_PACK_CATALOG";
#[cfg(target_os = "macos")]
const LINEAGE_CHILD_PREVIEW_LIMIT: usize = 4;
/// 200 ms ticks between writes of changed window geometry.
#[cfg(target_os = "macos")]
const WINDOW_GEOMETRY_FLUSH_TICKS: u32 = 5;

/// Which window a remembered rectangle belongs to. World windows share one
/// entry: reopening a World puts it where the last World window was, which is
/// what "the app opens where I left it" means with several Worlds open.
#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RememberedWindow {
    Home,
    World,
}

/// The latest geometry each window has reported, and what is already on disk.
/// Windows report while they render; the Home background loop writes the
/// difference a few times a second, so dragging a window is not a stream of
/// file writes.
#[cfg(target_os = "macos")]
struct WindowGeometry {
    home: Option<StoredWindowBounds>,
    world: Option<StoredWindowBounds>,
    saved_home: Option<StoredWindowBounds>,
    saved_world: Option<StoredWindowBounds>,
}

#[cfg(target_os = "macos")]
static WINDOW_GEOMETRY: Mutex<WindowGeometry> = Mutex::new(WindowGeometry {
    home: None,
    world: None,
    saved_home: None,
    saved_world: None,
});

#[cfg(target_os = "macos")]
fn stored_bounds(bounds: Bounds<gpui::Pixels>) -> StoredWindowBounds {
    StoredWindowBounds::new(
        f32::from(bounds.origin.x),
        f32::from(bounds.origin.y),
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
    )
}

#[cfg(target_os = "macos")]
fn restored_bounds(stored: StoredWindowBounds) -> Bounds<gpui::Pixels> {
    Bounds::new(
        point(px(stored.x), px(stored.y)),
        size(px(stored.width), px(stored.height)),
    )
}

/// The displays a window could be reopened onto right now.
#[cfg(target_os = "macos")]
fn display_bounds(cx: &App) -> Vec<StoredWindowBounds> {
    cx.displays()
        .into_iter()
        .map(|display| stored_bounds(display.bounds()))
        .collect()
}

/// Note where a window is now. Only an ordinary windowed rectangle is worth
/// remembering: a maximized or full-screen window should not reopen at the
/// size of somebody's screen.
#[cfg(target_os = "macos")]
fn remember_window_geometry(window: &Window, which: RememberedWindow) {
    let WindowBounds::Windowed(bounds) = window.window_bounds() else {
        return;
    };
    let stored = stored_bounds(bounds);
    if !stored.is_plausible() {
        return;
    }
    let Ok(mut geometry) = WINDOW_GEOMETRY.lock() else {
        return;
    };
    match which {
        RememberedWindow::Home => geometry.home = Some(stored),
        RememberedWindow::World => geometry.world = Some(stored),
    }
}

/// Where a window should open, or `None` for its default place.
#[cfg(target_os = "macos")]
fn remembered_window_bounds(which: RememberedWindow, cx: &App) -> Option<Bounds<gpui::Pixels>> {
    let geometry = WINDOW_GEOMETRY.lock().ok()?;
    let stored = match which {
        RememberedWindow::Home => geometry.home,
        RememberedWindow::World => geometry.world,
    };
    drop(geometry);
    StoredWindowBounds::restorable(stored, &display_bounds(cx)).map(restored_bounds)
}

/// Write any geometry that changed since the last write. Cheap and silent when
/// nothing moved, which is the common case.
#[cfg(target_os = "macos")]
fn flush_window_geometry() {
    let Ok(geometry) = WINDOW_GEOMETRY.lock() else {
        return;
    };
    if geometry.home == geometry.saved_home && geometry.world == geometry.saved_world {
        return;
    }
    let (home, world) = (geometry.home, geometry.world);
    drop(geometry);

    let Ok(root) = world_machine_desktop::analyst_settings::application_support_root() else {
        return;
    };
    let mut state = window_state::load(&root);
    state.home = home.or(state.home);
    state.world = world.or(state.world);
    match window_state::save(&root, &state) {
        Ok(()) => {
            if let Ok(mut geometry) = WINDOW_GEOMETRY.lock() {
                geometry.saved_home = home;
                geometry.saved_world = world;
            }
        }
        // Where the windows were is a convenience; failing to record it is a
        // line in the log, never something in front of somebody.
        Err(error) => diagnostics::error(format!("could not record window positions: {error}")),
    }
}

/// Seed the remembered geometry from disk at launch.
#[cfg(target_os = "macos")]
fn load_window_geometry() {
    let Ok(root) = world_machine_desktop::analyst_settings::application_support_root() else {
        return;
    };
    let state = window_state::load(&root);
    if let Ok(mut geometry) = WINDOW_GEOMETRY.lock() {
        geometry.home = state.home;
        geometry.world = state.world;
        geometry.saved_home = state.home;
        geometry.saved_world = state.world;
    }
}

#[cfg(target_os = "macos")]
static LIBRARY_CHANGE_REVISION: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "macos")]
pub(crate) fn mark_library_changed() {
    LIBRARY_CHANGE_REVISION.fetch_add(1, Ordering::Relaxed);
}

#[cfg(target_os = "macos")]
fn library_change_revision() -> u64 {
    LIBRARY_CHANGE_REVISION.load(Ordering::Relaxed)
}

#[cfg(target_os = "macos")]
struct SharedDocumentState {
    session: DurableWorldSession,
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
}

#[cfg(target_os = "macos")]
type SharedDocument = Rc<RefCell<SharedDocumentState>>;

#[cfg(target_os = "macos")]
struct HostProjectionController {
    document: SharedDocument,
}

#[cfg(target_os = "macos")]
impl world_gpui::ProjectionController for HostProjectionController {
    fn snapshot(&self) -> world_gpui::ProjectionSnapshot {
        self.document.borrow().session.snapshot()
    }

    fn handle(
        &mut self,
        intent: world_gpui::ProjectionIntent,
    ) -> Result<world_gpui::ProjectionSnapshot, String> {
        let mut document = self.document.borrow_mut();
        let registry = Arc::clone(&document.registry);
        let library = Arc::clone(&document.library);
        let is_library_world = document.session.document_id().is_some();
        let result = document
            .session
            .handle(intent, &registry, &library)
            .map_err(|error| error.to_string());
        if result.is_ok() && is_library_world {
            mark_library_changed();
        }
        result
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DocumentStatusTone {
    Info,
    Success,
    Error,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Debug, Eq, PartialEq)]
struct DocumentStatus {
    message: String,
    tone: DocumentStatusTone,
}

#[cfg(target_os = "macos")]
impl DocumentStatus {
    fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: DocumentStatusTone::Info,
        }
    }

    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: DocumentStatusTone::Success,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        let message = message.into();
        diagnostics::error(format!("document: {message}"));
        Self {
            message,
            tone: DocumentStatusTone::Error,
        }
    }
}

#[cfg(target_os = "macos")]
struct WorldDocumentView {
    /// The durable identity of the World's file. Stays visible so a World can
    /// always be matched to the file it lives in.
    document_label: String,
    /// What this World is called: the name its owner gave it on Home, or the
    /// durable identity when it has none.
    document_name: String,
    document: SharedDocument,
    projection: Entity<world_gpui::ProjectionView>,
    status: Option<DocumentStatus>,
    /// Whether the optional World Analyst runtime (Node + Pi) resolved when
    /// this document opened. The Analyst entry stays hidden otherwise so a
    /// fresh install never surfaces a feature that needs extra software.
    analyst_available: bool,
}

#[cfg(target_os = "macos")]
impl WorldDocumentView {
    fn new(
        session: DurableWorldSession,
        registry: Arc<world_host::WorldRegistry>,
        library: Arc<WorldLibrary>,
        cx: &mut Context<Self>,
    ) -> Self {
        let document_label = session.display_name();
        let document_name = session_display_name(&session);
        let document = Rc::new(RefCell::new(SharedDocumentState {
            session,
            registry,
            library,
        }));
        let controller = HostProjectionController {
            document: Rc::clone(&document),
        };
        let projection = cx.new(|_| world_gpui::ProjectionView::controlled(controller));
        let analyst_available = world_fork::analyst_available();
        Self {
            document_label,
            document_name,
            document,
            projection,
            status: None,
            analyst_available,
        }
    }

    /// Re-read what this World is called from the session, after anything
    /// that can change its file or its target.
    fn refresh_document_identity(&mut self) {
        let (label, name) = {
            let document = self.document.borrow();
            (
                document.session.display_name(),
                session_display_name(&document.session),
            )
        };
        self.document_label = label;
        self.document_name = name;
    }

    /// Opens Compare Futures for this World. Returns the Home status to show
    /// when the request came from a Home card.
    fn open_compare(&mut self, cx: &mut Context<Self>) -> Option<HomeStatus> {
        match strategy_compare::open_default(&self.document, cx) {
            Ok((left, right)) => {
                self.status = Some(DocumentStatus::success(format!(
                    "What if · {left} vs {right} · {} periods",
                    strategy_compare::DEFAULT_HORIZON
                )));
                cx.notify();
                None
            }
            Err(error) => {
                self.status = Some(DocumentStatus::info(error.clone()));
                cx.notify();
                Some(HomeStatus::info(format!(
                    "Opened {} · {error}",
                    self.document_name
                )))
            }
        }
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let result = {
            let mut document = self.document.borrow_mut();
            let registry = Arc::clone(&document.registry);
            let library = Arc::clone(&document.library);
            document.session.reload(&registry, &library)
        };

        match result {
            Ok(snapshot) => {
                if self.document.borrow().session.document_id().is_some() {
                    mark_library_changed();
                }
                self.refresh_document_identity();
                self.rebuild_projection(cx);
                self.status = Some(DocumentStatus::success(format!(
                    "Reloaded {} · World time {}",
                    self.document_name, snapshot.world_time
                )));
            }
            Err(error) => {
                self.status = Some(DocumentStatus::error(format!("Reload failed: {error}")));
            }
        }
        cx.notify();
    }

    fn save_as(&mut self, cx: &mut Context<Self>) {
        let semantic_title = self.document.borrow().session.snapshot().title;
        let suggested_name = suggested_world_file_name(&semantic_title, &self.document_name);
        let save_dialog = cx.prompt_for_new_path(&PathBuf::default(), Some(&suggested_name));
        cx.spawn(async move |this, cx| {
            let destination = match save_dialog.await {
                Ok(Ok(Some(path))) => canonical_world_path(path),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(DocumentStatus::error(format!(
                            "Could not open Save As dialog: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(DocumentStatus::error(format!(
                            "Save As dialog was interrupted: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
            };

            let _ = this.update(cx, |this, cx| {
                let result = {
                    let mut document = this.document.borrow_mut();
                    document.session.save_as_file(destination.clone())
                };
                match result {
                    Ok(snapshot) => {
                        this.refresh_document_identity();
                        this.rebuild_projection(cx);
                        this.status = Some(DocumentStatus::success(format!(
                            "Saved As {} · World time {}",
                            this.document_name, snapshot.world_time
                        )));
                    }
                    Err(error) => {
                        this.status =
                            Some(DocumentStatus::error(format!("Save As failed: {error}")));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn rebuild_projection(&mut self, cx: &mut Context<Self>) {
        let controller = HostProjectionController {
            document: Rc::clone(&self.document),
        };
        self.projection = cx.new(|_| world_gpui::ProjectionView::controlled(controller));
    }

    fn branch(&mut self, cx: &mut Context<Self>) {
        self.status = Some(match world_fork::fork_world(&self.document, cx) {
            Ok(result) => match result.warning {
                Some(warning) => {
                    DocumentStatus::info(format!("Branched as {} · {warning}", result.id))
                }
                None => DocumentStatus::success(format!("Branched as {}", result.id)),
            },
            Err(error) => DocumentStatus::error(format!("Could not branch: {error}")),
        });
        cx.notify();
    }

    fn compare_with_parent(&mut self, cx: &mut Context<Self>) {
        self.status = Some(match world_fork::compare_with_parent(&self.document, cx) {
            Ok((left, right)) => DocumentStatus::success(format!("Comparing {left} with {right}")),
            Err(error) => DocumentStatus::info(format!("Could not compare with parent: {error}")),
        });
        cx.notify();
    }

    fn open_saved_compare(&mut self, cx: &mut Context<Self>) {
        self.status = Some(match world_fork::open_saved_compare(&self.document, cx) {
            Ok(count) => DocumentStatus::success(format!(
                "Choose another saved World to compare · {count} available"
            )),
            Err(error) => DocumentStatus::info(format!("Could not compare saved Worlds: {error}")),
        });
        cx.notify();
    }

    fn open_lineage(&mut self, cx: &mut Context<Self>) {
        self.status = Some(match world_fork::open_lineage(&self.document, cx) {
            Ok(count) => DocumentStatus::success(format!("Opened lineage · {count} World(s)")),
            Err(error) => DocumentStatus::info(format!("Could not open lineage: {error}")),
        });
        cx.notify();
    }

    fn open_analyst(&mut self, cx: &mut Context<Self>) {
        if !self.analyst_available {
            self.status = Some(DocumentStatus::info(
                "The World Analyst needs Node and the Pi runtime installed on this Mac.",
            ));
            cx.notify();
            return;
        }
        self.status = Some(match world_fork::open_analyst(&self.document, cx) {
            Ok(()) => DocumentStatus::success("Opened the World Analyst"),
            Err(error) => DocumentStatus::error(format!("Could not open the Analyst: {error}")),
        });
        cx.notify();
    }
}

#[cfg(target_os = "macos")]
impl Render for WorldDocumentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        remember_window_geometry(window, RememberedWindow::World);
        window.set_window_title(&document_window_title(&self.document_name));
        let mut actions = div().flex_shrink_0().flex().items_center().gap_2();
        if let Some(badge) = world_fork::lineage_badge(&self.document) {
            actions = actions.child(badge);
        }
        let actions = actions
            .child(
                div()
                    .id("branch-world-document")
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(0xb8b2d8))
                    .bg(crate::theme_rgb(0xf7f5ff))
                    .text_sm()
                    .child("Branch")
                    .on_click(cx.listener(|this, _, _, cx| this.branch(cx))),
            )
            .child(
                div()
                    .id("what-if-world-document")
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(0x9eb0d6))
                    .bg(crate::theme_rgb(0xf4f7ff))
                    .text_sm()
                    .child("What if…")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.open_compare(cx);
                    })),
            );

        // The World is called by its name; the durable file identity stays
        // beside it, so renaming never hides which file this window edits.
        let mut identity = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .gap_2()
            .items_center()
            .overflow_hidden()
            .child(div().text_sm().child(self.document_name.clone()));
        if self.document_name != self.document_label {
            identity = identity.child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x8a8a82))
                    .child(self.document_label.clone()),
            );
        }

        let mut chrome = div()
            .h(px(48.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .gap_3()
            .border_b_1()
            .border_color(crate::theme_rgb(0xd9d9d3))
            .bg(crate::theme_rgb(0xf7f7f3))
            .child(identity)
            .child(actions);

        if let Some(status) = &self.status {
            let foreground = match status.tone {
                DocumentStatusTone::Info => 0x4e6fb3,
                DocumentStatusTone::Success => 0x4d6748,
                DocumentStatusTone::Error => 0x9b4a42,
            };
            chrome = chrome.child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(foreground))
                    .child(status.message.clone()),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            // World menu items dispatch to the frontmost window's root, so a
            // World window answers them and Home greys them out.
            .on_action(cx.listener(|this, _: &about::BranchWorld, _, cx| this.branch(cx)))
            .on_action(cx.listener(|this, _: &about::WhatIf, _, cx| {
                this.open_compare(cx);
            }))
            .on_action(cx.listener(|this, _: &about::SaveWorldAs, _, cx| this.save_as(cx)))
            .on_action(cx.listener(|this, _: &about::ReloadWorld, _, cx| this.reload(cx)))
            .on_action(
                cx.listener(|this, _: &about::CompareWithParent, _, cx| {
                    this.compare_with_parent(cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &about::CompareSavedWorlds, _, cx| {
                    this.open_saved_compare(cx)
                }),
            )
            .on_action(cx.listener(|this, _: &about::ShowLineage, _, cx| this.open_lineage(cx)))
            .on_action(cx.listener(|this, _: &about::AnalyzeWorlds, _, cx| this.open_analyst(cx)))
            .child(chrome)
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .w_full()
                    .overflow_hidden()
                    .child(self.projection.clone()),
            )
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HomeStatusTone {
    Info,
    Success,
    Error,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Debug, Eq, PartialEq)]
struct HomeStatus {
    message: String,
    tone: HomeStatusTone,
}

#[cfg(target_os = "macos")]
impl HomeStatus {
    fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: HomeStatusTone::Info,
        }
    }

    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: HomeStatusTone::Success,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        let message = message.into();
        diagnostics::error(format!("home: {message}"));
        Self {
            message,
            tone: HomeStatusTone::Error,
        }
    }
}

#[cfg(target_os = "macos")]
struct WorldMachineHome {
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
    pack_catalog: Option<PackCatalog>,
    pack_catalog_path: PathBuf,
    documents: Vec<WorldDocumentSummary>,
    selected_world_pack: Option<String>,
    lineage: Option<LineageIndex>,
    included_packs: Vec<included_packs::IncludedPack>,
    pending_pack_install: Option<PackInstallPreview>,
    pending_start_after_install: Option<WorldPackRef>,
    ready_pack_to_create: Option<WorldPackRef>,
    probing_packs: Vec<WorldPackRef>,
    status: Option<HomeStatus>,
    /// Installed-Pack management is hidden behind one line on Home until asked
    /// for; nothing in the ordinary path needs it.
    show_packs: bool,
    /// A newer stable release found at launch, shown as a banner until
    /// dismissed or downloaded.
    available_update: Option<updates::AvailableUpdate>,
    /// Files in the Worlds folder that name themselves Worlds but cannot be
    /// read. They are named on Home instead of hiding every other World.
    unreadable_documents: Vec<UnreadableWorldFile>,
    /// The World whose name is being typed, if any.
    renaming: Option<RenameDraft>,
    /// The World whose removal is waiting for a second click.
    pending_removal: Option<WorldDocumentId>,
    /// How My Worlds is ordered, for this run of the app.
    world_sort: WorldSort,
    /// What was typed into Find a World.
    world_search: Entity<AnalystTextInput>,
}

/// A World name being typed on Home. Only one World is renamed at a time, so
/// the field lives here rather than one per card.
#[cfg(target_os = "macos")]
struct RenameDraft {
    document: WorldDocumentId,
    input: Entity<AnalystTextInput>,
}

#[cfg(target_os = "macos")]
impl WorldMachineHome {
    fn start_system_open_listener(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let mut observed_library_revision = library_change_revision();
            let mut ticks_since_geometry_flush = 0u32;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;

                let Some(this) = this.upgrade() else {
                    return;
                };

                // Windows report where they are as they render; write any
                // change a few times a second rather than on every frame of a
                // drag.
                ticks_since_geometry_flush += 1;
                if ticks_since_geometry_flush >= WINDOW_GEOMETRY_FLUSH_TICKS {
                    ticks_since_geometry_flush = 0;
                    flush_window_geometry();
                }
                let paths = system_open::drain_paths();
                let revision = library_change_revision();
                let library_changed = revision != observed_library_revision;
                if library_changed {
                    observed_library_revision = revision;
                }
                if paths.is_empty() && !library_changed {
                    continue;
                }

                this.update(cx, |this, cx| {
                    for path in paths {
                        match path {
                            Ok(path) => this.open_external_path(path, cx),
                            Err(error) => {
                                this.status = Some(HomeStatus::error(format!(
                                    "Could not open World file: {error}"
                                )));
                            }
                        }
                    }
                    if library_changed {
                        if let Err(error) = this.refresh_documents() {
                            this.status = Some(error);
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn rebuild_registry(&mut self) -> Result<(), String> {
        let registry = build_registry(self.pack_catalog.as_ref())?;
        self.registry = Arc::new(registry);
        Ok(())
    }

    fn review_pack_path(
        &mut self,
        source: PathBuf,
        expected_pack: Option<WorldPackRef>,
        start_after_install: bool,
        cx: &mut Context<Self>,
    ) {
        self.pending_start_after_install = None;
        if self.pack_catalog.is_none() {
            match PackCatalog::open(&self.pack_catalog_path) {
                Ok(catalog) => self.pack_catalog = Some(catalog),
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not open Installed Packs catalog {}: {error}",
                        self.pack_catalog_path.display()
                    )));
                    cx.notify();
                    return;
                }
            }
        }

        let catalog = self.pack_catalog.as_ref().unwrap();
        match catalog.inspect_install(&source) {
            Ok(preview) => {
                if expected_pack
                    .as_ref()
                    .is_some_and(|expected| preview.pack() != expected)
                {
                    let expected = expected_pack.unwrap();
                    self.pending_pack_install = None;
                    self.status = Some(HomeStatus::error(format!(
                        "Included Pack identity mismatch: expected {} @ {}, found {} @ {}",
                        expected.id,
                        expected.version,
                        preview.pack().id,
                        preview.pack().version
                    )));
                    cx.notify();
                    return;
                }
                self.status = Some(HomeStatus::info(format!(
                    "Review {} @ {} before trusting its executable bytes",
                    preview.pack().id,
                    preview.pack().version
                )));
                self.pending_start_after_install =
                    start_after_install.then(|| preview.pack().clone());
                self.pending_pack_install = Some(preview);
            }
            Err(error) => {
                self.pending_pack_install = None;
                self.status = Some(HomeStatus::error(format!(
                    "Could not inspect {}: {error}",
                    source.display()
                )));
            }
        }
        cx.notify();
    }

    fn review_included_pack(
        &mut self,
        pack: included_packs::IncludedPack,
        start_after_install: bool,
        cx: &mut Context<Self>,
    ) {
        self.review_pack_path(pack.path, Some(pack.pack), start_after_install, cx);
    }

    /// Install, probe, and activate the World Packs shipped inside this app
    /// bundle so a fresh install reaches a living World without a review
    /// dialog. The bundle is one code-signed unit, so these Packs carry the
    /// same trust as the app binary; the catalog still pins their exact
    /// content on install and the durable probe still runs before activation.
    /// User-supplied `.worldpack` files keep the explicit review flow.
    /// Packs that are already in the catalog, enabled or not, are left alone.
    /// One background request to the Releases API; a newer stable version
    /// becomes a banner on Home. Silent on failure, off with
    /// WORLD_MACHINE_NO_UPDATE_CHECK=1.
    fn start_update_check(&mut self, cx: &mut Context<Self>) {
        if !updates::enabled() {
            return;
        }
        let task = cx
            .background_executor()
            .spawn(async move { updates::check() });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(update) = result {
                    diagnostics::info(format!("update available: {}", update.version));
                    this.available_update = Some(update);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn update_banner(
        &self,
        update: updates::AvailableUpdate,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let url = update.url.clone();
        div()
            .id("update-available")
            .w_full()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xa8b9d6))
            .bg(crate::theme_rgb(0xf1f5fb))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_sm()
                    .text_color(crate::theme_rgb(0x314b72))
                    .child(format!(
                        "World Machine {} is available. You have {}.",
                        update.version,
                        build_info::APP_VERSION
                    )),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("download-update")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::theme_rgb(0x657da7))
                            .bg(crate::theme_rgb(0xffffff))
                            .text_sm()
                            .child("Download")
                            .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                    )
                    .child(
                        div()
                            .id("dismiss-update")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::theme_rgb(0xc5cfdf))
                            .text_sm()
                            .child("Later")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.available_update = None;
                                cx.notify();
                            })),
                    ),
            )
    }

    fn activate_included_packs(&mut self, cx: &mut Context<Self>) {
        let packs = self.included_packs.clone();
        for pack in packs {
            if self.included_pack_is_installed(&pack.pack) {
                continue;
            }
            let Some(catalog) = self.pack_catalog.as_mut() else {
                return;
            };
            let preview = match catalog.inspect_install(&pack.path) {
                Ok(preview) => preview,
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not prepare {}: {error}",
                        pack.title
                    )));
                    continue;
                }
            };
            if preview.pack() != &pack.pack {
                self.status = Some(HomeStatus::error(format!(
                    "Could not prepare {}: bundle contains {} @ {}, expected {} @ {}",
                    pack.title,
                    preview.pack().id,
                    preview.pack().version,
                    pack.pack.id,
                    pack.pack.version
                )));
                continue;
            }
            match catalog.install_reviewed_pending_probe(&preview) {
                Ok(installed) => {
                    // A fresh install (no saved Worlds yet) opens straight
                    // into its first World once the featured Pack is ready;
                    // otherwise Home offers the Create handoff.
                    let create_now = pack.featured && self.documents.is_empty();
                    self.start_pack_probe(
                        installed.pack,
                        true,
                        create_now,
                        pack.featured && !create_now,
                        cx,
                    );
                    self.status = Some(HomeStatus::info(format!(
                        "Preparing {} for its first launch…",
                        pack.title
                    )));
                }
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not prepare {}: {error}",
                        pack.title
                    )));
                }
            }
        }
        cx.notify();
    }

    fn install_pack(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Install World Pack".into()),
        });
        cx.spawn(async move |this, cx| {
            let source = match picker.await {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Could not open Install Pack dialog: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Install Pack dialog was interrupted: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(source) = source else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                this.review_pack_path(source, None, false, cx)
            });
        })
        .detach();
    }

    fn confirm_pack_install(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.pending_pack_install.clone() else {
            return;
        };
        let start_after_install =
            start_after_install_matches(self.pending_start_after_install.as_ref(), preview.pack());
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        let result = catalog.install_reviewed_pending_probe(&preview);
        self.pending_pack_install = None;
        self.pending_start_after_install = None;
        self.ready_pack_to_create = None;
        match result {
            Ok(installed) => {
                self.start_pack_probe(installed.pack, true, start_after_install, true, cx);
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Pack was not installed. Re-open it to review current content: {error}"
                )));
                cx.notify();
            }
        }
    }

    fn is_pack_probing(&self, pack: &WorldPackRef) -> bool {
        self.probing_packs.iter().any(|candidate| candidate == pack)
    }

    fn start_pack_probe(
        &mut self,
        pack: WorldPackRef,
        activate_on_success: bool,
        create_on_success: bool,
        offer_create_on_success: bool,
        cx: &mut Context<Self>,
    ) {
        if self.is_pack_probing(&pack) {
            return;
        }
        let Some(catalog) = self.pack_catalog.clone() else {
            return;
        };
        self.probing_packs.push(pack.clone());
        self.status = Some(HomeStatus::info(if create_on_success {
            "Testing this World before first launch…".into()
        } else if let Some(title) = self.included_pack_title(&pack) {
            format!("Preparing {title} for its first launch…")
        } else {
            format!(
                "Testing trusted Pack {} @ {} · Create → Archive → fresh-process Open…",
                pack.id, pack.version
            )
        }));
        cx.notify();

        let probe_pack = pack.clone();
        let task = cx
            .background_executor()
            .spawn(async move { catalog.probe(&probe_pack) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.probing_packs.retain(|candidate| candidate != &pack);
                match result {
                    Ok(probe) => {
                        let transition = this
                            .pack_catalog
                            .as_mut()
                            .ok_or_else(|| "Installed Packs catalog is unavailable".to_string())
                            .and_then(|catalog| {
                                catalog
                                    .set_enabled(&pack, true)
                                    .map_err(|error| error.to_string())?;
                                if activate_on_success {
                                    catalog.activate(&pack).map_err(|error| error.to_string())?;
                                }
                                Ok(())
                            });
                        match transition {
                            Ok(()) => match this.rebuild_registry() {
                                Ok(()) => {
                                    if activate_on_success && create_on_success {
                                        this.ready_pack_to_create = None;
                                        this.create_world(pack.id.clone(), cx);
                                        return;
                                    }
                                    if activate_on_success && offer_create_on_success {
                                        this.ready_pack_to_create = Some(pack.clone());
                                    }
                                    diagnostics::info(format!(
                                        "pack {} @ {} passed its durable probe · World time {} → {}",
                                        pack.id,
                                        pack.version,
                                        probe.created_world_time,
                                        probe.reopened_world_time
                                    ));
                                    this.status = Some(HomeStatus::success(
                                        match this.included_pack_title(&pack) {
                                            Some(title) => format!("{title} is ready to start."),
                                            None => format!(
                                                "Trusted and tested {} @ {} · durable Create/Archive/Open succeeded · World time {} → {}",
                                                pack.id,
                                                pack.version,
                                                probe.created_world_time,
                                                probe.reopened_world_time
                                            ),
                                        },
                                    ));
                                }
                                Err(error) => {
                                    this.status = Some(HomeStatus::error(format!(
                                        "Pack {} @ {} passed its durable probe, but Registry rebuild failed: {error}",
                                        pack.id, pack.version
                                    )));
                                }
                            },
                            Err(error) => {
                                this.status = Some(HomeStatus::error(format!(
                                    "Pack {} @ {} passed its durable probe, but could not be enabled: {error}",
                                    pack.id, pack.version
                                )));
                            }
                        }
                    }
                    Err(error) => {
                        if this.ready_pack_to_create.as_ref() == Some(&pack) {
                            this.ready_pack_to_create = None;
                        }
                        let _ = this.rebuild_registry();
                        this.status = Some(HomeStatus::error(format!(
                            "Installed and trusted {} @ {}, but its durable activation probe failed. The Pack remains disabled: {error}",
                            pack.id, pack.version
                        )));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn included_pack_title(&self, pack: &WorldPackRef) -> Option<&'static str> {
        self.included_packs
            .iter()
            .find(|included| &included.pack == pack)
            .map(|included| included.title)
    }

    fn ready_pack_descriptor(&self) -> Option<world_host::WorldDescriptor> {
        let pack = self.ready_pack_to_create.as_ref()?;
        let catalog = self.pack_catalog.as_ref()?;
        let installed = catalog.entry(pack)?;
        if !installed.enabled || !installed.active {
            return None;
        }
        if !matches!(catalog.availability(pack), PackAvailability::Ready) {
            return None;
        }
        self.registry.descriptor_for(pack).cloned()
    }

    fn dismiss_ready_pack(&mut self, cx: &mut Context<Self>) {
        self.ready_pack_to_create = None;
        cx.notify();
    }

    fn cancel_pack_install(&mut self, cx: &mut Context<Self>) {
        self.pending_pack_install = None;
        self.pending_start_after_install = None;
        self.status = Some(HomeStatus::info(
            "Pack installation cancelled; no external code was installed.",
        ));
        cx.notify();
    }

    fn activate_pack(&mut self, pack: WorldPackRef, cx: &mut Context<Self>) {
        if self
            .ready_pack_to_create
            .as_ref()
            .is_some_and(|ready| ready.id == pack.id && ready != &pack)
        {
            self.ready_pack_to_create = None;
        }
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        match catalog.activate(&pack) {
            Ok(()) => match self.rebuild_registry() {
                Ok(()) => {
                    self.status = Some(HomeStatus::success(format!(
                        "Activated {} @ {} for new Worlds",
                        pack.id, pack.version
                    )))
                }
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Changed active Pack to {} @ {}, but Registry rebuild failed: {error}",
                        pack.id, pack.version
                    )))
                }
            },
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not activate {} @ {}: {error}",
                    pack.id, pack.version
                )))
            }
        }
        cx.notify();
    }

    fn set_pack_enabled(&mut self, pack: WorldPackRef, enabled: bool, cx: &mut Context<Self>) {
        if !enabled && self.ready_pack_to_create.as_ref() == Some(&pack) {
            self.ready_pack_to_create = None;
        }
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        match catalog.set_enabled(&pack, enabled) {
            Ok(()) => match self.rebuild_registry() {
                Ok(()) => {
                    self.status = Some(HomeStatus::success(format!(
                        "{} {} @ {}",
                        if enabled { "Enabled" } else { "Disabled" },
                        pack.id,
                        pack.version
                    )))
                }
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Updated {} @ {}, but Registry rebuild failed: {error}",
                        pack.id, pack.version
                    )))
                }
            },
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not {} {} @ {}: {error}",
                    if enabled { "enable" } else { "disable" },
                    pack.id,
                    pack.version
                )))
            }
        }
        cx.notify();
    }

    fn missing_pack_message(&self, pack: &WorldPackRef) -> String {
        let availability = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.availability(pack))
            .unwrap_or(PackAvailability::NotInstalled);
        match availability {
            PackAvailability::Ready => format!(
                "Could not open World: {} @ {} should be available, but is not registered",
                pack.id, pack.version
            ),
            PackAvailability::Disabled => format!(
                "Could not open World: {} @ {} is installed but disabled. Use Test & Enable under Installed Packs.",
                pack.id, pack.version
            ),
            PackAvailability::Invalid { reason } => format!(
                "Could not open World: {} @ {} is installed but no longer matches its approved content: {reason}",
                pack.id, pack.version
            ),
            PackAvailability::MissingVersion { installed_versions } => format!(
                "Could not open World: it requires {} @ {}, but installed versions are {}. Install that exact Pack version.",
                pack.id,
                pack.version,
                installed_versions.join(", ")
            ),
            PackAvailability::NotInstalled => format!(
                "Could not open World: it requires {} @ {}, which is not installed. Use Install Pack… to add that exact version.",
                pack.id, pack.version
            ),
        }
    }

    fn refresh_documents(&mut self) -> Result<usize, HomeStatus> {
        let listing = self
            .library
            .listing()
            .map_err(|error| HomeStatus::error(format!("Could not read World Library: {error}")))?;
        let count = listing.documents.len();
        self.documents = listing.documents;
        self.unreadable_documents = listing.unreadable;
        report_unreadable_documents(&self.unreadable_documents);
        // A World that is gone from the Library cannot still be mid-rename or
        // mid-removal on a card.
        let documents = &self.documents;
        let renaming_is_gone = self.renaming.as_ref().is_some_and(|draft| {
            !documents
                .iter()
                .any(|document| document.id == draft.document)
        });
        let removal_is_gone = self
            .pending_removal
            .as_ref()
            .is_some_and(|pending| !documents.iter().any(|document| &document.id == pending));
        if renaming_is_gone {
            self.renaming = None;
        }
        if removal_is_gone {
            self.pending_removal = None;
        }
        if !world_pack_filter_is_available(&self.documents, self.selected_world_pack.as_deref()) {
            self.selected_world_pack = None;
        }
        self.refresh_lineage()?;
        Ok(count)
    }

    fn refresh_lineage(&mut self) -> Result<(), HomeStatus> {
        match LineageIndex::from_library(self.library.as_ref()) {
            Ok(lineage) => {
                self.lineage = Some(lineage);
                Ok(())
            }
            Err(error) => {
                self.lineage = None;
                Err(HomeStatus::error(format!(
                    "Could not build World lineage: {error}"
                )))
            }
        }
    }

    fn refresh_from_menu(&mut self, cx: &mut Context<Self>) {
        self.status = Some(match self.refresh_documents() {
            Ok(count) => HomeStatus::success(format!("My Worlds · {count} World(s)")),
            Err(status) => status,
        });
        cx.notify();
    }

    fn sync_documents_after_mutation(&mut self) -> Option<HomeStatus> {
        self.refresh_documents().err()
    }

    fn open_session(
        &mut self,
        session: DurableWorldSession,
        title: String,
        cx: &mut Context<Self>,
    ) {
        self.open_session_with(session, title, false, cx);
    }

    /// Opens a World window; with `compare_on_open`, also opens Compare
    /// Futures for it so a Home card can jump straight to "what if".
    fn open_session_with(
        &mut self,
        mut session: DurableWorldSession,
        title: String,
        compare_on_open: bool,
        cx: &mut Context<Self>,
    ) {
        let is_library_world = session.document_id().is_some();
        let catch_up = observer::catch_up(&mut session, &self.registry, &self.library);
        let sync_error = if is_library_world && matches!(&catch_up, Ok(Some(_))) {
            self.refresh_documents().err()
        } else {
            None
        };
        let registry = Arc::clone(&self.registry);
        let library = Arc::clone(&self.library);
        let bounds = remembered_window_bounds(RememberedWindow::World, cx)
            .unwrap_or_else(|| Bounds::centered(None, size(px(1100.0), px(900.0)), cx));
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                watch_appearance(window);
                cx.new(|cx| WorldDocumentView::new(session, registry, library, cx))
            },
        );

        let compare = match (&opened, compare_on_open) {
            (Ok(handle), true) => handle
                .update(cx, |view, _, cx| view.open_compare(cx))
                .ok()
                .flatten(),
            _ => None,
        };

        self.status = Some(match opened {
            Ok(_) => match catch_up {
                Ok(Some(outcome)) => HomeStatus::success(format!(
                    "Opened {title} · {} period(s) passed while you were away",
                    outcome.periods
                )),
                Ok(None) => HomeStatus::success(format!("Opened {title}")),
                Err(error) => HomeStatus::info(format!(
                    "Opened {title} · could not advance the time you were away: {error}"
                )),
            },
            Err(error) => HomeStatus::error(format!("Could not open {title}: {error}")),
        });
        if let Some(status) = sync_error {
            self.status = Some(status);
        }
        if let Some(compare) = compare {
            self.status = Some(compare);
        }
        cx.notify();
    }

    fn compare_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        let summary = self
            .documents
            .iter()
            .find(|document| document.id == document_id);
        if let Some(document) = summary {
            if self.registry.descriptor_for(&document.pack).is_none() {
                self.status = Some(HomeStatus::error(self.missing_pack_message(&document.pack)));
                cx.notify();
                return;
            }
        }
        let title = summary
            .and_then(|document| self.registry.descriptor_for(&document.pack))
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document_id.to_string());
        let session = match DurableWorldSession::open(document_id, &self.registry, &self.library) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not open {title}: {error}"
                )));
                cx.notify();
                return;
            }
        };
        self.open_session_with(session, title, true, cx);
    }

    fn create_world(&mut self, pack_id: String, cx: &mut Context<Self>) {
        let title = self
            .registry
            .descriptor(&pack_id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| pack_id.clone());
        let document_id = match new_document_id(&pack_id, &self.library) {
            Ok(id) => id,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not create {title}: {error}"
                )));
                cx.notify();
                return;
            }
        };
        let session =
            match DurableWorldSession::create(document_id, &pack_id, &self.registry, &self.library)
            {
                Ok(session) => session,
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not create {title}: {error}"
                    )));
                    cx.notify();
                    return;
                }
            };
        let sync_error = self.sync_documents_after_mutation();
        if self
            .ready_pack_to_create
            .as_ref()
            .is_some_and(|ready| ready.id == pack_id)
        {
            self.ready_pack_to_create = None;
        }
        self.open_session(session, title, cx);
        if let Some(status) = sync_error {
            self.status = Some(status);
            cx.notify();
        }
    }

    fn open_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        let summary = self
            .documents
            .iter()
            .find(|document| document.id == document_id);
        if let Some(document) = summary {
            if self.registry.descriptor_for(&document.pack).is_none() {
                self.status = Some(HomeStatus::error(self.missing_pack_message(&document.pack)));
                cx.notify();
                return;
            }
        }
        let title = summary
            .and_then(|document| self.registry.descriptor_for(&document.pack))
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document_id.to_string());
        let session = match DurableWorldSession::open(document_id, &self.registry, &self.library) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not open {title}: {error}"
                )));
                cx.notify();
                return;
            }
        };
        self.open_session(session, title, cx);
    }

    fn open_external_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if is_world_pack_file(&source) {
            self.review_pack_path(source, None, false, cx);
            return;
        }
        if let Some(document_id) = library_document_id_for_path(&source, &self.library) {
            self.open_document(document_id, cx);
            return;
        }
        if !is_world_file(&source) {
            self.status = Some(HomeStatus::error(format!(
                "Could not open {}: choose a {} document or {} Pack",
                source.display(),
                WORLD_DOCUMENT_SUFFIX,
                PACK_BUNDLE_SUFFIX
            )));
            cx.notify();
            return;
        }
        let session = match DurableWorldSession::open_file(source.clone(), &self.registry) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not open {}: {error}",
                    source.display()
                )));
                cx.notify();
                return;
            }
        };
        let pack = session.pack();
        let title = self
            .registry
            .descriptor_for(&pack)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or(pack.id);
        self.open_session(session, title, cx);
    }

    fn import_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if !is_world_file(&source) {
            self.status = Some(HomeStatus::error(format!(
                "Could not import {}: choose a {} file",
                source.display(),
                WORLD_DOCUMENT_SUFFIX
            )));
            cx.notify();
            return;
        }
        let document_id = match imported_document_id(&source, &self.library) {
            Ok(id) => id,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not import {}: {error}",
                    source.display()
                )));
                cx.notify();
                return;
            }
        };
        let session = match DurableWorldSession::import_file(
            document_id,
            &source,
            &self.registry,
            &self.library,
        ) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not import {}: {error}",
                    source.display()
                )));
                cx.notify();
                return;
            }
        };
        let pack = session.pack();
        let title = self
            .registry
            .descriptor_for(&pack)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or(pack.id);
        let sync_error = self.sync_documents_after_mutation();
        self.open_session(session, title, cx);
        if let Some(status) = sync_error {
            self.status = Some(status);
            cx.notify();
        }
    }

    fn import_world(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import World".into()),
        });
        cx.spawn(async move |this, cx| {
            let source = match picker.await {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Could not open Import dialog: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Import dialog was interrupted: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(source) = source else {
                return;
            };
            let _ = this.update(cx, |this, cx| this.import_path(source, cx));
        })
        .detach();
    }

    fn export_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        let semantic_title = self
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .and_then(|document| document.display_title.as_deref())
            .unwrap_or_default();
        let suggested_name = suggested_world_file_name(semantic_title, document_id.as_str());
        let save_dialog = cx.prompt_for_new_path(&PathBuf::default(), Some(&suggested_name));
        cx.spawn(async move |this, cx| {
            let destination = match save_dialog.await {
                Ok(Ok(Some(path))) => canonical_world_path(path),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Could not open Export dialog: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Export dialog was interrupted: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
            };

            let _ = this.update(cx, |this, cx| {
                match this.library.export_file(&document_id, &destination) {
                    Ok(()) => {
                        this.status = Some(HomeStatus::success(format!(
                            "Exported {} to {}",
                            document_id,
                            destination.display()
                        )));
                    }
                    Err(error) => {
                        this.status = Some(HomeStatus::error(format!(
                            "Could not export {document_id}: {error}"
                        )));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn set_world_pack_filter(&mut self, pack_id: Option<String>, cx: &mut Context<Self>) {
        self.selected_world_pack = pack_id;
        cx.notify();
    }

    fn pack_filter_title(&self, pack_id: &str) -> String {
        self.registry
            .descriptor(pack_id)
            .map(|descriptor| descriptor.title.clone())
            .or_else(|| {
                self.documents
                    .iter()
                    .find(|document| document.pack.id == pack_id)
                    .and_then(|document| self.registry.descriptor_for(&document.pack))
                    .map(|descriptor| descriptor.title.clone())
            })
            .unwrap_or_else(|| pack_id.to_owned())
    }

    /// Find a World, and choose whether the list reads most-recent-first or
    /// A to Z. Shown only once there are enough Worlds for the list to be hard
    /// to scan.
    fn world_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut order = div().flex_shrink_0().flex().items_center().gap_2().child(
            div()
                .text_xs()
                .text_color(crate::theme_rgb(0x777770))
                .child("Order"),
        );
        for sort in [WorldSort::Recent, WorldSort::Name] {
            let selected = self.world_sort == sort;
            let (border, background, text) = if selected {
                (0x6f86b0, 0xe9eef7, 0x314b72)
            } else {
                (0xd9d9d3, 0xffffff, 0x666666)
            };
            order = order.child(
                div()
                    .id(SharedString::from(format!(
                        "world-sort-{}",
                        sort.label().to_lowercase()
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(border))
                    .bg(crate::theme_rgb(background))
                    .text_color(crate::theme_rgb(text))
                    .text_xs()
                    .child(sort.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.world_sort = sort;
                        cx.notify();
                    })),
            );
        }

        div()
            .w_full()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(self.world_search.clone()),
            )
            .child(order)
    }

    fn document_pack_title(&self, document_id: &WorldDocumentId) -> String {
        self.documents
            .iter()
            .find(|document| &document.id == document_id)
            .map(|document| {
                self.registry
                    .descriptor_for(&document.pack)
                    .map(|descriptor| descriptor.title.clone())
                    .unwrap_or_else(|| document.pack.id.clone())
            })
            .unwrap_or_else(|| document_id.to_string())
    }

    /// Open the name field on a World card. A World that already has a name
    /// opens on that name; an unnamed one opens empty and shows its World
    /// Pack's title as the placeholder.
    fn begin_rename(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        analyst_input::bind_keys(cx);
        let current = self
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .and_then(|document| document.display_title.clone())
            .unwrap_or_default();
        let placeholder = rename_placeholder(&self.document_pack_title(&document_id));
        let input = cx.new(|cx| AnalystTextInput::new(placeholder, cx).with_text(current));
        self.pending_removal = None;
        self.renaming = Some(RenameDraft {
            document: document_id,
            input,
        });
        cx.notify();
    }

    fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming = None;
        cx.notify();
    }

    fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.renaming.take() else {
            return;
        };
        let typed = draft.input.read(cx).text().trim().to_owned();
        let title = (!typed.is_empty()).then_some(typed);
        let pack_title = self.document_pack_title(&draft.document);
        // The Library borrow ends here, before the arms take `&mut self`.
        let renamed = self
            .library
            .set_display_title(&draft.document, title.as_deref());
        match renamed {
            Ok(summary) => {
                mark_library_changed();
                let message = rename_result_message(summary.display_title.as_deref(), &pack_title);
                self.status = Some(match self.sync_documents_after_mutation() {
                    Some(error) => error,
                    None => HomeStatus::success(message),
                });
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not rename this World: {error}"
                )));
            }
        }
        cx.notify();
    }

    /// Removing a World is one click to ask and one to confirm; the card
    /// itself carries the question, so nothing is removed by a stray click.
    fn request_removal(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        self.renaming = None;
        self.pending_removal = Some(document_id);
        cx.notify();
    }

    fn cancel_removal(&mut self, cx: &mut Context<Self>) {
        self.pending_removal = None;
        cx.notify();
    }

    fn confirm_removal(&mut self, cx: &mut Context<Self>) {
        let Some(document_id) = self.pending_removal.take() else {
            return;
        };
        let title = self
            .document_title_for_id(&document_id)
            .unwrap_or_else(|| document_id.to_string());
        let removed = self.library.remove(&document_id);
        match removed {
            Ok(path) => {
                mark_library_changed();
                diagnostics::info(format!("removed World {document_id} to {}", path.display()));
                self.status = Some(match self.sync_documents_after_mutation() {
                    Some(error) => error,
                    None => HomeStatus::success(removal_result_message(&title)),
                });
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not remove this World: {error}"
                )));
            }
        }
        cx.notify();
    }

    fn document_title_for_id(&self, document_id: &WorldDocumentId) -> Option<String> {
        self.documents
            .iter()
            .find(|document| &document.id == document_id)
            .map(|document| {
                let pack_title = self
                    .registry
                    .descriptor_for(&document.pack)
                    .map(|descriptor| descriptor.title.clone())
                    .unwrap_or_else(|| document.pack.id.clone());
                world_summary_title(document, &pack_title)
            })
    }

    fn document_card(
        &self,
        document: WorldDocumentSummary,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open_id = document.id.clone();
        let compare_id = document.id.clone();
        let export_id = document.id.clone();
        let rename_id = document.id.clone();
        let remove_id = document.id.clone();
        let pack_title = self
            .registry
            .descriptor_for(&document.pack)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document.pack.id.clone());
        let title = world_summary_title(&document, &pack_title);
        let document_label = document.id.to_string();
        let lineage_node = self
            .lineage
            .as_ref()
            .and_then(|lineage| lineage.node(&document.id))
            .cloned();

        let mut details = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_lg().child(title.clone()));
        if let Some(summary) = world_summary_description(&document) {
            details = details.child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x4f5968))
                    .child(summary),
            );
        }
        details = details
            .child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(if title == pack_title {
                        format!(
                            "World time {} · {} events",
                            document.world_time, document.event_count
                        )
                    } else {
                        format!(
                            "{} · World time {} · {} events",
                            pack_title, document.world_time, document.event_count
                        )
                    }),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x8a8a82))
                    .child(document_label.clone()),
            );

        let renaming_this_world = self
            .renaming
            .as_ref()
            .is_some_and(|draft| draft.document == document.id);
        let removing_this_world = self.pending_removal.as_ref() == Some(&document.id);
        if renaming_this_world {
            let input = self
                .renaming
                .as_ref()
                .expect("renaming_this_world implies a draft")
                .input
                .clone();
            details = details.child(
                div().w_full().flex().flex_col().gap_2().child(input).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .id(SharedString::from(format!("save-name-{document_label}")))
                                .cursor_pointer()
                                .p_2()
                                .rounded_md()
                                .border_1()
                                .border_color(crate::theme_rgb(0x657da7))
                                .bg(crate::theme_rgb(0xf4f7ff))
                                .text_sm()
                                .child("Save name")
                                .on_click(cx.listener(|this, _, _, cx| this.commit_rename(cx))),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("cancel-name-{document_label}")))
                                .cursor_pointer()
                                .p_2()
                                .rounded_md()
                                .border_1()
                                .border_color(crate::theme_rgb(0xd9d9d3))
                                .text_sm()
                                .child("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| this.cancel_rename(cx))),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .text_xs()
                                .text_color(crate::theme_rgb(0x777770))
                                .child("An empty name lists this World under its own title again."),
                        ),
                ),
            );
        } else if removing_this_world {
            details = details.child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(crate::theme_rgb(0x9b4a42))
                            .child(format!(
                                "Remove {title}? Its file moves to the {} folder inside your Worlds folder, so you can put it back.",
                                world_library::REMOVED_DIRECTORY
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "confirm-remove-{document_label}"
                                    )))
                                    .cursor_pointer()
                                    .p_2()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(crate::theme_rgb(0xb4736c))
                                    .bg(crate::theme_rgb(0xfbf0ee))
                                    .text_color(crate::theme_rgb(0x9b4a42))
                                    .text_sm()
                                    .child("Remove")
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.confirm_removal(cx)),
                                    ),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "keep-{document_label}"
                                    )))
                                    .cursor_pointer()
                                    .p_2()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(crate::theme_rgb(0xd9d9d3))
                                    .text_sm()
                                    .child("Keep")
                                    .on_click(cx.listener(|this, _, _, cx| this.cancel_removal(cx))),
                            ),
                    ),
            );
        } else {
            details = details.child(
                div()
                    .flex()
                    .gap_3()
                    .text_xs()
                    .child(
                        div()
                            .id(SharedString::from(format!("rename-{document_label}")))
                            .cursor_pointer()
                            .text_color(crate::theme_rgb(0x4e6fb3))
                            .child("Rename")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.begin_rename(rename_id.clone(), cx)
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("remove-{document_label}")))
                            .cursor_pointer()
                            .text_color(crate::theme_rgb(0x4e6fb3))
                            .child("Remove")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.request_removal(remove_id.clone(), cx)
                            })),
                    ),
            );
        }

        if let Some(node) = lineage_node {
            if let Some(parent) = node.parent.as_ref() {
                let branch_label = node.branch.as_ref().map(lineage_branch_label);
                let mut origin = div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .child(div().text_color(crate::theme_rgb(0x777770)).child("Origin"));

                if let Some(parent_id) = parent.resolved.clone() {
                    let parent_label = parent_id.to_string();
                    let parent_title = self
                        .document_title_for_id(&parent_id)
                        .unwrap_or_else(|| parent_label.clone());
                    let open_parent = parent_id.clone();
                    origin = origin.child(
                        div()
                            .id(SharedString::from(format!(
                                "lineage-parent-{document_label}-{parent_label}"
                            )))
                            .cursor_pointer()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_color(crate::theme_rgb(0x4e6fb3))
                                    .child(parent_title),
                            )
                            .child(
                                div()
                                    .text_color(crate::theme_rgb(0x8a8a82))
                                    .child(parent_label),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_document(open_parent.clone(), cx)
                            })),
                    );
                } else {
                    let parent_label = parent
                        .document
                        .clone()
                        .unwrap_or_else(|| parent.pack.id.clone());
                    origin = origin.child(
                        div()
                            .text_color(crate::theme_rgb(0x777770))
                            .child(format!("{parent_label} · outside My Worlds")),
                    );
                }

                if let Some(branch_label) = branch_label {
                    origin = origin.child(
                        div()
                            .text_color(crate::theme_rgb(0x777770))
                            .child(format!("· {branch_label}")),
                    );
                }
                details = details.child(origin);
            }

            if !node.children.is_empty() {
                let mut branches = div().flex().flex_col().gap_1().child(
                    div()
                        .text_xs()
                        .text_color(crate::theme_rgb(0x777770))
                        .child(format!("Branches · {}", node.children.len())),
                );
                let (visible_children, hidden_children) = lineage_child_preview(&node.children);
                for child_id in visible_children {
                    let child_label = child_id.to_string();
                    let child_title = self
                        .document_title_for_id(child_id)
                        .unwrap_or_else(|| child_label.clone());
                    let child_branch = self
                        .lineage
                        .as_ref()
                        .and_then(|lineage| lineage.node(child_id))
                        .and_then(|child| child.branch.as_ref())
                        .map(lineage_branch_label);
                    let identity = child_branch
                        .map(|branch| format!("{child_label} · {branch}"))
                        .unwrap_or_else(|| child_label.clone());
                    let open_child = child_id.clone();
                    branches = branches.child(
                        div()
                            .id(SharedString::from(format!(
                                "lineage-child-{document_label}-{child_label}"
                            )))
                            .cursor_pointer()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .text_xs()
                            .child(
                                div()
                                    .text_color(crate::theme_rgb(0x4e6fb3))
                                    .child(child_title),
                            )
                            .child(div().text_color(crate::theme_rgb(0x8a8a82)).child(identity))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_document(open_child.clone(), cx)
                            })),
                    );
                }
                if hidden_children > 0 {
                    branches = branches.child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x777770))
                            .child(format!(
                                "+{hidden_children} more branches · listed as their own Worlds"
                            )),
                    );
                }
                details = details.child(branches);
            }
        }

        div()
            .id(SharedString::from(format!("document-{document_label}")))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xd9d9d3))
            .bg(crate::theme_rgb(0xffffff))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(details)
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .items_end()
                    .gap_2()
                    .child(
                        div()
                            .id(SharedString::from(format!("open-{open_id}")))
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::theme_rgb(0x657da7))
                            .bg(crate::theme_rgb(0xf4f7ff))
                            .text_sm()
                            .child("Open")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_document(open_id.clone(), cx)
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("compare-{compare_id}")))
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::theme_rgb(0xd9d9d3))
                            .text_sm()
                            .child("What if…")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.compare_document(compare_id.clone(), cx)
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("export-{export_id}")))
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::theme_rgb(0xd9d9d3))
                            .text_sm()
                            .child("Export…")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.export_document(export_id.clone(), cx)
                            })),
                    ),
            )
    }

    fn included_pack_is_installed(&self, pack: &WorldPackRef) -> bool {
        self.pack_catalog.as_ref().is_some_and(|catalog| {
            catalog
                .entries()
                .iter()
                .any(|installed| &installed.pack == pack)
        })
    }

    fn featured_included_pack_card(
        &self,
        pack: included_packs::IncludedPack,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let review_pack = pack.clone();
        let identity = format!("{} @ {}", pack.pack.id, pack.pack.version);
        div()
            .id("featured-included-world")
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xa8b9d6))
            .bg(crate::theme_rgb(0xf1f5fb))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x5e6f91))
                            .child("START HERE · PERSISTENT SOCIAL WORLD"),
                    )
                    .child(div().text_lg().child(pack.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(crate::theme_rgb(0x4f5968))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(crate::theme_rgb(0x314b72))
                            .child(pack.experience),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x71809a))
                            .child(format!(
                                "{identity} · Included external World · reviewed before it runs"
                            )),
                    ),
            )
            .child(
                div()
                    .id("review-featured-included-world")
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(0x657da7))
                    .text_sm()
                    .child("Review & Start")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.review_included_pack(review_pack.clone(), true, cx)
                    })),
            )
    }

    fn included_pack_card(
        &self,
        pack: included_packs::IncludedPack,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let review_pack = pack.clone();
        let identity = format!("{} @ {}", pack.pack.id, pack.pack.version);
        div()
            .id(SharedString::from(format!(
                "included-pack-{}-{}",
                pack.pack.id, pack.pack.version
            )))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xc8d5c0))
            .bg(crate::theme_rgb(0xf7fbf5))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child(pack.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(crate::theme_rgb(0x666666))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x66735f))
                            .child(pack.experience),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x75806f))
                            .child(format!(
                                "{identity} · Included external Pack · review required"
                            )),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from(format!(
                        "review-included-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(0x91a486))
                    .text_sm()
                    .child("Review & Install")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.review_included_pack(review_pack.clone(), false, cx)
                    })),
            )
    }

    fn ready_pack_card(
        &self,
        descriptor: world_host::WorldDescriptor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pack_id = descriptor.pack.id.clone();
        let title = descriptor.title.clone();
        let button_title = format!("Create {}", descriptor.title);
        div()
            .id("pack-ready-to-create")
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0x8eb58a))
            .bg(crate::theme_rgb(0xf1f8ee))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child(format!("{title} is ready")))
                    .child(div().text_sm().text_color(crate::theme_rgb(0x52604d)).child(
                        "Start your first World. It keeps living between visits, and you can always create another.",
                    ))
                    .child(div().text_xs().text_color(crate::theme_rgb(0x75806f)).child(format!(
                        "Version {} · no World created yet",
                        descriptor.pack.version
                    ))),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("create-ready-pack-world")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::theme_rgb(0x6f966b))
                            .text_sm()
                            .child(button_title)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.create_world(pack_id.clone(), cx)
                            })),
                    )
                    .child(
                        div()
                            .id("dismiss-ready-pack-world")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::theme_rgb(0xcbd8c7))
                            .text_sm()
                            .child("Not now")
                            .on_click(cx.listener(|this, _, _, cx| this.dismiss_ready_pack(cx))),
                    ),
            )
    }

    fn pack_install_review_card(
        &self,
        preview: PackInstallPreview,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let start_after_install =
            start_after_install_matches(self.pending_start_after_install.as_ref(), preview.pack());
        let review_title = if start_after_install {
            "Review before starting"
        } else {
            "Review Pack Install"
        };
        let confirm_title = if start_after_install {
            "Trust & Start"
        } else {
            "Install & Trust"
        };
        let format = preview.kind().label();
        let size = format_program_size(preview.program_bytes());
        let source = preview.source_path().display().to_string();
        let pack = format!("{} @ {}", preview.pack().id, preview.pack().version);
        let runtime = preview.runtime_name().to_owned();
        let sha = preview.program_sha256().to_owned();

        div()
            .id("pack-install-review")
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xc7a85a))
            .bg(crate::theme_rgb(0xfffbeb))
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_lg().child(review_title))
            .child(div().text_lg().child(preview.title().to_owned()))
            .child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(preview.description().to_owned()),
            )
            .child(div().text_xs().child(format!("Identity · {pack}")))
            .child(div().text_xs().child(format!("Format · {format}")))
            .child(div().text_xs().child(format!("Will execute · {runtime}")))
            .child(div().text_xs().child(format!("Executable · {size}")))
            .child(div().text_xs().child(format!("SHA-256 · {sha}")))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777770))
                    .child(format!("Source · {source}")),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x6f5420))
                    .child(if start_after_install {
                        "No Pack code has run. Trust & Start approves these exact executable bytes; after the durable self-test passes, World Machine will create and open your World."
                    } else {
                        "No Pack code has run. Install & Trust approves these exact executable bytes; any change before installation is rejected."
                    }),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id("confirm-pack-install")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::theme_rgb(0x8c6a23))
                            .child(confirm_title)
                            .on_click(cx.listener(|this, _, _, cx| this.confirm_pack_install(cx))),
                    )
                    .child(
                        div()
                            .id("cancel-pack-install")
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(crate::theme_rgb(0xd9d9d3))
                            .child("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_pack_install(cx))),
                    ),
            )
    }

    fn installed_pack_card(&self, pack: InstalledPack, cx: &mut Context<Self>) -> impl IntoElement {
        let availability = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.availability(&pack.pack))
            .unwrap_or(PackAvailability::NotInstalled);
        let probing = self.is_pack_probing(&pack.pack);
        let state = if probing {
            "Testing durable round-trip…".to_string()
        } else {
            match &availability {
                PackAvailability::Ready if pack.active => "Active".to_string(),
                PackAvailability::Ready => "Historical · available for saved Worlds".to_string(),
                PackAvailability::Disabled => {
                    "Disabled · trusted, not runnable until tested".to_string()
                }
                PackAvailability::Invalid { reason } => format!("Invalid · {reason}"),
                PackAvailability::MissingVersion { .. } => "Missing exact version".to_string(),
                PackAvailability::NotInstalled => "Not installed".to_string(),
            }
        };
        let activate_pack = pack.pack.clone();
        let toggle_pack = pack.pack.clone();
        let test_pack = pack.pack.clone();
        let enabled = pack.enabled;
        let active = pack.active;

        let mut actions = div().flex().gap_2();
        if !probing && enabled && !active {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "activate-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(0xd9d9d3))
                    .text_sm()
                    .child("Activate")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.activate_pack(activate_pack.clone(), cx)
                    })),
            );
        }
        if !probing && enabled {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "toggle-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(0xd9d9d3))
                    .text_sm()
                    .child("Disable")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_pack_enabled(toggle_pack.clone(), false, cx)
                    })),
            );
        } else if !probing && !enabled {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "test-enable-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(0xd9d9d3))
                    .text_sm()
                    .child("Test & Enable")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.start_pack_probe(test_pack.clone(), false, false, false, cx)
                    })),
            );
        }

        div()
            .id(SharedString::from(format!(
                "installed-pack-{}-{}",
                pack.pack.id, pack.pack.version
            )))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xd9d9d3))
            .bg(crate::theme_rgb(0xffffff))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child(pack.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(crate::theme_rgb(0x666666))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x8a8a82))
                            .child(format!(
                                "{} @ {} · {state}",
                                pack.pack.id, pack.pack.version
                            )),
                    ),
            )
            .child(actions)
    }

    fn new_world_card(
        &self,
        descriptor: world_host::WorldDescriptor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pack_id = descriptor.pack.id.clone();
        div()
            .id(SharedString::from(format!("new-world-{pack_id}")))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xd9d9d3))
            .bg(crate::theme_rgb(0xffffff))
            .cursor_pointer()
            .child(div().text_lg().child(descriptor.title))
            .child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(descriptor.description),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x8a8a82))
                    .child(format!("Version {}", descriptor.pack.version)),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.create_world(pack_id.clone(), cx)))
    }
}

#[cfg(target_os = "macos")]
impl Render for WorldMachineHome {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        remember_window_geometry(window, RememberedWindow::Home);
        window.set_window_title("World Machine");

        let documents = self.documents.clone();
        let has_documents = !documents.is_empty();
        let pack_filters = world_pack_filter_counts(&documents);
        let selected_world_pack = self.selected_world_pack.clone();
        let search_query = normalize_world_search(self.world_search.read(cx).text());
        let show_world_controls = documents.len() >= WORLD_SEARCH_THRESHOLD;
        let mut visible_cards = documents
            .iter()
            .filter(|document| world_matches_pack_filter(document, selected_world_pack.as_deref()))
            .map(|document| {
                let pack_title = self
                    .registry
                    .descriptor_for(&document.pack)
                    .map(|descriptor| descriptor.title.clone())
                    .unwrap_or_else(|| document.pack.id.clone());
                let title = world_summary_title(document, &pack_title);
                (title, pack_title, document.clone())
            })
            .filter(|(title, pack_title, document)| {
                world_matches_search(title, pack_title, document.id.as_str(), &search_query)
            })
            .map(|(title, _pack_title, document)| (title, document))
            .collect::<Vec<_>>();
        sort_world_cards(&mut visible_cards, self.world_sort);
        let visible_documents = visible_cards
            .into_iter()
            .map(|(_title, document)| document)
            .collect::<Vec<_>>();
        let visible_document_count = visible_documents.len();
        let descriptors = self
            .registry
            .descriptors()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let first_run = !has_documents;
        let featured_included = self
            .included_packs
            .iter()
            .find(|pack| pack.featured && !self.included_pack_is_installed(&pack.pack))
            .cloned();
        let featured_review_pending = self
            .pending_pack_install
            .as_ref()
            .zip(featured_included.as_ref())
            .is_some_and(|(preview, featured)| preview.pack() == &featured.pack);
        let show_featured = first_run
            && self.pending_pack_install.is_none()
            && self.ready_pack_to_create.is_none()
            && featured_included.is_some();

        let mut saved = div().w_full().flex().flex_col().gap_3();
        if !has_documents {
            saved = saved.child(
                div()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(0xe1e1dc))
                    .text_sm()
                    .text_color(crate::theme_rgb(0x777770))
                    .child("No Worlds yet. Start one below; it keeps living while you are away."),
            );
        } else if visible_documents.is_empty() {
            let empty = if search_query.is_empty() {
                "No Worlds match this Pack filter."
            } else {
                "No Worlds match what you typed."
            };
            saved = saved.child(
                div()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(0xe1e1dc))
                    .text_sm()
                    .text_color(crate::theme_rgb(0x777770))
                    .child(empty),
            );
        } else {
            for document in visible_documents {
                saved = saved.child(self.document_card(document, cx));
            }
        }

        let mut world_filters = div().w_full().flex().gap_2();
        if pack_filters.len() > 1 {
            let all_selected = selected_world_pack.is_none();
            let (all_border, all_background, all_text) = if all_selected {
                (0x6f86b0, 0xe9eef7, 0x314b72)
            } else {
                (0xd9d9d3, 0xffffff, 0x666666)
            };
            world_filters = world_filters.child(
                div()
                    .id("world-pack-filter-all")
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(all_border))
                    .bg(crate::theme_rgb(all_background))
                    .text_color(crate::theme_rgb(all_text))
                    .text_xs()
                    .child(format!("All · {}", documents.len()))
                    .on_click(cx.listener(|this, _, _, cx| this.set_world_pack_filter(None, cx))),
            );
            for (pack_id, count) in pack_filters.iter() {
                let selected = selected_world_pack.as_deref() == Some(pack_id.as_str());
                let (border, background, text) = if selected {
                    (0x6f86b0, 0xe9eef7, 0x314b72)
                } else {
                    (0xd9d9d3, 0xffffff, 0x666666)
                };
                let filter_pack = pack_id.clone();
                let filter_title = self.pack_filter_title(pack_id);
                world_filters = world_filters.child(
                    div()
                        .id(SharedString::from(format!("world-pack-filter-{pack_id}")))
                        .cursor_pointer()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(crate::theme_rgb(border))
                        .bg(crate::theme_rgb(background))
                        .text_color(crate::theme_rgb(text))
                        .text_xs()
                        .child(format!("{filter_title} · {count}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_world_pack_filter(Some(filter_pack.clone()), cx)
                        })),
                );
            }
        }

        let visible_included_packs = self
            .included_packs
            .iter()
            .filter(|included| !self.included_pack_is_installed(&included.pack))
            .filter(|included| {
                featured_included.as_ref().is_none_or(|featured| {
                    featured.pack != included.pack || (!show_featured && !featured_review_pending)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut included = div().w_full().flex().flex_col().gap_3();
        for pack in visible_included_packs.iter().cloned() {
            included = included.child(self.included_pack_card(pack, cx));
        }

        let installed_packs = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.entries().to_vec())
            .unwrap_or_default();
        let mut installed = div().w_full().flex().flex_col().gap_3();
        for pack in installed_packs.iter().cloned() {
            installed = installed.child(self.installed_pack_card(pack, cx));
        }

        let mut available = div().w_full().flex().flex_col().gap_3();
        for descriptor in descriptors {
            available = available.child(self.new_world_card(descriptor, cx));
        }

        let header = div()
            .id("world-machine-home-chrome")
            .w_full()
            .p_4()
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child("World Machine"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(crate::theme_rgb(0x666666))
                            .child("Persistent worlds that remember, evolve, and branch."),
                    ),
            );

        let mut body = div()
            .id("world-machine-home-scroll")
            .w_full()
            .flex_1()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_4();

        if let Some(update) = self.available_update.clone() {
            body = body.child(self.update_banner(update, cx));
        }

        if let Some(preview) = self.pending_pack_install.clone() {
            body = body.child(self.pack_install_review_card(preview, cx));
        }

        if let Some(descriptor) = self.ready_pack_descriptor() {
            body = body.child(self.ready_pack_card(descriptor, cx));
        }

        if show_featured {
            let featured = featured_included.expect("show_featured requires a featured Pack");
            body = body
                .child(div().text_sm().child("Start here"))
                .child(self.featured_included_pack_card(featured, cx));
        }

        if has_documents || self.included_packs.is_empty() {
            body = body.child(
                div()
                    .text_sm()
                    .child(my_worlds_title(visible_document_count, documents.len())),
            );
            if show_world_controls {
                body = body.child(self.world_controls(cx));
            }
            if let Some(note) = unreadable_documents_note(&self.unreadable_documents) {
                body = body.child(
                    div()
                        .w_full()
                        .p_3()
                        .rounded_md()
                        .border_1()
                        .border_color(crate::theme_rgb(0xe3d2ce))
                        .bg(crate::theme_rgb(0xfbf0ee))
                        .text_xs()
                        .text_color(crate::theme_rgb(0x9b4a42))
                        .child(note),
                );
            }
            if pack_filters.len() > 1 {
                body = body.child(world_filters);
            }
            body = body.child(saved);
        }

        if !visible_included_packs.is_empty() {
            body = body
                .child(div().text_sm().child(if first_run {
                    "More worlds"
                } else {
                    "Included Worlds"
                }))
                .child(included);
        }

        body = body
            .child(div().text_sm().child("New World"))
            .child(available);

        if !installed_packs.is_empty() {
            let packs_title = if self.show_packs {
                format!("Packs · {} · hide", installed_packs.len())
            } else {
                format!("Packs · {} · show", installed_packs.len())
            };
            body = body.child(
                div()
                    .id("toggle-installed-packs")
                    .cursor_pointer()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(packs_title)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_packs = !this.show_packs;
                        cx.notify();
                    })),
            );
            if self.show_packs {
                body = body.child(installed);
            }
        }

        let mut shell = div()
            .size_full()
            .bg(crate::theme_rgb(0xf7f7f3))
            .text_color(crate::theme_rgb(0x202020))
            .flex()
            .flex_col()
            .child(header);

        if let Some(status) = &self.status {
            let (background, foreground) = match status.tone {
                HomeStatusTone::Info => (0xf1f5fb, 0x4e6fb3),
                HomeStatusTone::Success => (0xeef2ea, 0x4d6748),
                HomeStatusTone::Error => (0xfbf0ee, 0x9b4a42),
            };
            shell = shell.child(
                div().id("world-machine-home-status").w_full().px_4().child(
                    div()
                        .p_3()
                        .rounded_md()
                        .bg(crate::theme_rgb(background))
                        .text_color(crate::theme_rgb(foreground))
                        .text_sm()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(div().flex_1().child(status.message.clone()))
                        .child(
                            div()
                                .id("dismiss-world-machine-home-status")
                                .cursor_pointer()
                                .p_2()
                                .rounded_md()
                                .border_1()
                                .border_color(crate::theme_rgb(foreground))
                                .text_xs()
                                .child("Dismiss")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.status = None;
                                    cx.notify();
                                })),
                        ),
                ),
            );
        }

        shell.child(body)
    }
}

/// How My Worlds is ordered.
#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorldSort {
    /// The Library's own order: most recently played first.
    Recent,
    /// What the cards say, A to Z.
    Name,
}

#[cfg(target_os = "macos")]
impl WorldSort {
    fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Name => "Name",
        }
    }
}

/// Beyond this many Worlds the list needs finding and ordering; below it the
/// controls would be clutter on a screen that shows every World at once.
#[cfg(target_os = "macos")]
const WORLD_SEARCH_THRESHOLD: usize = 6;

#[cfg(target_os = "macos")]
fn normalize_world_search(query: &str) -> String {
    query.trim().to_lowercase()
}

/// A World matches what was typed when the text appears in something the card
/// itself shows: its name, its World Pack's title, or its file identity.
#[cfg(target_os = "macos")]
fn world_matches_search(title: &str, pack_title: &str, document_id: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    [title, pack_title, document_id]
        .iter()
        .any(|field| field.to_lowercase().contains(query))
}

/// Order the cards. `Recent` leaves the Library's own most-recently-played
/// order alone; `Name` sorts by what the card shows, ignoring case, with the
/// file identity breaking ties so the order never wobbles between renders.
#[cfg(target_os = "macos")]
fn sort_world_cards(cards: &mut [(String, WorldDocumentSummary)], sort: WorldSort) {
    if sort == WorldSort::Name {
        cards.sort_by(|(left_title, left), (right_title, right)| {
            left_title
                .to_lowercase()
                .cmp(&right_title.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
    }
}

/// What the My Worlds heading says once some Worlds are filtered out.
#[cfg(target_os = "macos")]
fn my_worlds_title(visible: usize, total: usize) -> String {
    if visible == total {
        format!("My Worlds · {total}")
    } else {
        format!("My Worlds · {visible}/{total}")
    }
}

/// The placeholder in the name field: an unnamed World shows the World Pack's
/// own title, which is exactly what the card falls back to.
#[cfg(target_os = "macos")]
fn rename_placeholder(pack_title: &str) -> String {
    format!("{pack_title} — name this World")
}

#[cfg(target_os = "macos")]
fn rename_result_message(display_title: Option<&str>, pack_title: &str) -> String {
    // The name is written into the World's own file, so a window already open
    // on that World is one save behind until it reloads. Say so rather than
    // letting it surface later as a changed-on-disk refusal.
    let reload = "If this World is open in a window, choose World → Reload there.";
    match display_title {
        Some(title) => format!("Renamed to {title}. {reload}"),
        None => format!("Name cleared; this World is listed as {pack_title} again. {reload}"),
    }
}

#[cfg(target_os = "macos")]
fn removal_result_message(title: &str) -> String {
    format!(
        "Removed {title}. Its file moved to the {} folder inside your Worlds folder.",
        world_library::REMOVED_DIRECTORY
    )
}

#[cfg(target_os = "macos")]
const UNREADABLE_DOCUMENT_NAME_LIMIT: usize = 3;

/// Name the files that could not be read, without letting a folder full of
/// them push every World off the screen.
#[cfg(target_os = "macos")]
fn unreadable_documents_note(unreadable: &[UnreadableWorldFile]) -> Option<String> {
    if unreadable.is_empty() {
        return None;
    }
    let named = unreadable
        .iter()
        .take(UNREADABLE_DOCUMENT_NAME_LIMIT)
        .map(|file| file.file_name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let hidden = unreadable
        .len()
        .saturating_sub(UNREADABLE_DOCUMENT_NAME_LIMIT);
    let names = if hidden > 0 {
        format!("{named}, and {hidden} more")
    } else {
        named
    };
    Some(if unreadable.len() == 1 {
        format!("One file in your Worlds folder could not be read and is not listed: {names}.")
    } else {
        format!(
            "{} files in your Worlds folder could not be read and are not listed: {names}.",
            unreadable.len()
        )
    })
}

#[cfg(target_os = "macos")]
fn report_unreadable_documents(unreadable: &[UnreadableWorldFile]) {
    for file in unreadable {
        diagnostics::error(format!(
            "could not read {} in the Worlds folder: {}",
            file.file_name, file.reason
        ));
    }
}

#[cfg(target_os = "macos")]
fn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {
    document
        .display_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(pack_title)
        .to_owned()
}

#[cfg(target_os = "macos")]
fn world_summary_description(document: &WorldDocumentSummary) -> Option<String> {
    document
        .display_summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(str::to_owned)
}

#[cfg(target_os = "macos")]
fn lineage_child_preview(children: &[WorldDocumentId]) -> (&[WorldDocumentId], usize) {
    let visible = children.len().min(LINEAGE_CHILD_PREVIEW_LIMIT);
    (&children[..visible], children.len() - visible)
}

#[cfg(target_os = "macos")]
fn world_matches_pack_filter(document: &WorldDocumentSummary, pack_id: Option<&str>) -> bool {
    pack_id.is_none_or(|pack_id| document.pack.id == pack_id)
}

#[cfg(target_os = "macos")]
fn world_pack_filter_is_available(
    documents: &[WorldDocumentSummary],
    pack_id: Option<&str>,
) -> bool {
    pack_id.is_none_or(|pack_id| documents.iter().any(|document| document.pack.id == pack_id))
}

#[cfg(target_os = "macos")]
fn world_pack_filter_counts(documents: &[WorldDocumentSummary]) -> Vec<(String, usize)> {
    let mut filters = Vec::<(String, usize)>::new();
    for document in documents {
        if let Some((_pack_id, count)) = filters
            .iter_mut()
            .find(|(pack_id, _count)| pack_id == &document.pack.id)
        {
            *count += 1;
        } else {
            filters.push((document.pack.id.clone(), 1));
        }
    }
    filters
}

/// What a World is called: the name its owner gave it, or the durable identity
/// of its file when it has no name.
#[cfg(target_os = "macos")]
fn document_display_name(display_title: Option<&str>, durable_label: &str) -> String {
    display_title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(durable_label)
        .to_owned()
}

#[cfg(target_os = "macos")]
fn session_display_name(session: &DurableWorldSession) -> String {
    let durable_label = session.display_name();
    document_display_name(session.metadata().display_title.as_deref(), &durable_label)
}

#[cfg(target_os = "macos")]
fn document_window_title(document_label: &str) -> String {
    format!("{document_label} — World Machine")
}

#[cfg(target_os = "macos")]
fn start_after_install_matches(pending: Option<&WorldPackRef>, pack: &WorldPackRef) -> bool {
    pending == Some(pack)
}

#[cfg(target_os = "macos")]
fn format_program_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    if bytes >= MIB {
        format!("{:.1} MiB · {bytes} bytes", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB · {bytes} bytes", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} bytes")
    }
}

#[cfg(target_os = "macos")]
fn lineage_branch_label(branch: &WorldBranchCause) -> String {
    match branch {
        WorldBranchCause::Strategy {
            choice_title,
            horizon,
            ..
        } => format!("{choice_title} · +{horizon}"),
        WorldBranchCause::Fork { label: Some(label) } => format!("Fork · {label}"),
        WorldBranchCause::Fork { label: None } => "Fork".into(),
    }
}

#[cfg(target_os = "macos")]
fn discover_library() -> std::io::Result<WorldLibrary> {
    if let Some(path) = env::var_os(LIBRARY_OVERRIDE_ENV) {
        return Ok(WorldLibrary::new(PathBuf::from(path)));
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("HOME is not set"))?;
    Ok(WorldLibrary::new(
        home.join("Library")
            .join("Application Support")
            .join("World Machine")
            .join("Worlds"),
    ))
}

#[cfg(target_os = "macos")]
fn discover_pack_catalog_path(library: &WorldLibrary) -> PathBuf {
    if let Some(path) = env::var_os(PACK_CATALOG_OVERRIDE_ENV) {
        return PathBuf::from(path);
    }
    if env::var_os(LIBRARY_OVERRIDE_ENV).is_some() {
        return library
            .root()
            .join(".world-machine-packs")
            .join("catalog.json");
    }
    library
        .root()
        .parent()
        .unwrap_or_else(|| library.root())
        .join("Packs")
        .join("catalog.json")
}

#[cfg(target_os = "macos")]
fn build_registry(catalog: Option<&PackCatalog>) -> Result<world_host::WorldRegistry, String> {
    let mut registry = world_builtins::registry().map_err(|error| error.to_string())?;
    if let Some(catalog) = catalog {
        let source = catalog
            .trusted_source()
            .map_err(|error| error.to_string())?;
        let source = with_pack_settings(source, world_voice_settings());
        registry
            .install_source(&source)
            .map_err(|error| error.to_string())?;
    }
    Ok(registry)
}

/// What this app tells the Worlds it launches about how to speak.
///
/// Empty unless somebody has both turned a World's voice on and told the app
/// which local program to use — a preference with nothing to act on is not a
/// setting worth passing. A Pack that is given nothing behaves exactly as it
/// always has.
#[cfg(target_os = "macos")]
fn world_voice_settings() -> Vec<(String, String)> {
    let Ok(root) = world_machine_desktop::analyst_settings::application_support_root() else {
        return Vec::new();
    };
    let Ok(settings) = world_machine_desktop::analyst_settings::load(&root) else {
        return Vec::new();
    };
    if !settings.world_voice {
        return Vec::new();
    }
    let Some(program) = settings.pi_program.as_ref() else {
        return Vec::new();
    };
    vec![
        (
            "WORLD_MACHINE_POCKET_UNIVERSE_VOICE".to_string(),
            "pi".to_string(),
        ),
        (
            "WORLD_MACHINE_PI_PROGRAM".to_string(),
            program.display().to_string(),
        ),
    ]
}

/// Hand every Pack in this source the same settings.
///
/// A setting the Pack layer refuses is a mistake in this app rather than
/// anything the observer did, so the Worlds are installed without it instead of
/// leaving somebody unable to open anything.
#[cfg(target_os = "macos")]
fn with_pack_settings(
    source: world_pack_process::ProcessPackSource,
    settings: Vec<(String, String)>,
) -> world_pack_process::ProcessPackSource {
    if settings.is_empty() {
        return source;
    }
    let packs = source
        .packs()
        .iter()
        .cloned()
        .map(|pack| pack.with_settings(settings.clone()))
        .collect::<Result<Vec<_>, _>>();
    match packs {
        Ok(packs) => world_pack_process::ProcessPackSource::from_packs(packs),
        Err(_) => source,
    }
}

#[cfg(target_os = "macos")]
fn new_document_id(pack_id: &str, library: &WorldLibrary) -> Result<WorldDocumentId, LibraryError> {
    unique_document_id(sanitize_document_base(pack_id), Some(library))
}

#[cfg(target_os = "macos")]
fn imported_document_id(
    source: &Path,
    library: &WorldLibrary,
) -> Result<WorldDocumentId, LibraryError> {
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("imported-world");
    let base = file_name
        .strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX)
        .or_else(|| file_name.strip_suffix(WORLD_DOCUMENT_SUFFIX))
        .unwrap_or(file_name);
    unique_document_id(sanitize_document_base(base), Some(library))
}

#[cfg(target_os = "macos")]
fn library_document_id_for_path(source: &Path, library: &WorldLibrary) -> Option<WorldDocumentId> {
    if source.parent()? != library.root() {
        return None;
    }
    let file_name = source.file_name()?.to_str()?;
    let raw_id = file_name
        .strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX)
        .or_else(|| file_name.strip_suffix(WORLD_DOCUMENT_SUFFIX))?;
    let id = WorldDocumentId::new(raw_id).ok()?;
    library
        .contains(&id)
        .ok()
        .filter(|exists| *exists)
        .map(|_| id)
}

#[cfg(target_os = "macos")]
fn unique_document_id(
    mut base: String,
    library: Option<&WorldLibrary>,
) -> Result<WorldDocumentId, LibraryError> {
    if base.is_empty() {
        base = "imported-world".into();
    }
    let candidate = WorldDocumentId::new(base.clone())?;
    match library {
        Some(library) if library.contains(&candidate)? => {}
        _ => return Ok(candidate),
    }

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    WorldDocumentId::new(format!("{base}-{}-{nonce}", process::id()))
}

#[cfg(target_os = "macos")]
fn sanitize_document_base(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .take(80)
        .collect()
}

#[cfg(target_os = "macos")]
fn suggested_world_file_name(semantic_title: &str, fallback_label: &str) -> String {
    let semantic_title = semantic_title.trim();
    let source = if semantic_title.is_empty() {
        fallback_label
    } else {
        semantic_title
    };
    let source = source
        .strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX)
        .or_else(|| source.strip_suffix(WORLD_DOCUMENT_SUFFIX))
        .unwrap_or(source);
    let stem = source
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '-'
            } else {
                ch
            }
        })
        .take(80)
        .collect::<String>();
    let stem = stem.trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '.' | '-'));
    let stem = if stem.is_empty() { "World" } else { stem };
    format!("{stem}{WORLD_DOCUMENT_SUFFIX}")
}

#[cfg(target_os = "macos")]
fn canonical_world_path(mut path: PathBuf) -> PathBuf {
    let Some(file_name) = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
    else {
        return path;
    };

    if file_name.ends_with(WORLD_DOCUMENT_SUFFIX) {
        return path;
    }
    if let Some(base) = file_name.strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX) {
        path.set_file_name(format!("{base}{WORLD_DOCUMENT_SUFFIX}"));
    } else {
        path.set_extension("world");
    }
    path
}

#[cfg(target_os = "macos")]
fn is_world_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.ends_with(WORLD_DOCUMENT_SUFFIX) || name.ends_with(LEGACY_WORLD_DOCUMENT_SUFFIX)
        })
}

#[cfg(target_os = "macos")]
fn is_world_pack_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(PACK_BUNDLE_SUFFIX))
}

#[cfg(all(test, target_os = "macos"))]
mod file_type_tests {
    use super::*;

    #[test]
    fn document_status_tone_is_explicit_not_inferred_from_message_text() {
        let info = DocumentStatus::info("failed-looking warning after a durable save");
        let success = DocumentStatus::success("done");
        let error = DocumentStatus::error("failed");

        assert_eq!(info.tone, DocumentStatusTone::Info);
        assert_eq!(success.tone, DocumentStatusTone::Success);
        assert_eq!(error.tone, DocumentStatusTone::Error);
        assert_eq!(info.message, "failed-looking warning after a durable save");
    }

    #[test]
    fn library_change_revision_advances_after_mark() {
        let before = library_change_revision();
        mark_library_changed();
        assert!(library_change_revision() > before);
    }

    #[test]
    fn home_status_tone_is_explicit_not_inferred_from_message_text() {
        let info = HomeStatus::info("Could not-looking informational text");
        let success = HomeStatus::success("done");
        let error = HomeStatus::error("failed");

        assert_eq!(info.tone, HomeStatusTone::Info);
        assert_eq!(success.tone, HomeStatusTone::Success);
        assert_eq!(error.tone, HomeStatusTone::Error);
        assert_eq!(info.message, "Could not-looking informational text");
    }

    #[test]
    fn world_pack_filters_preserve_recent_pack_order_and_counts() {
        let summary = |id: &str, pack_id: &str| WorldDocumentSummary {
            id: WorldDocumentId::new(id).unwrap(),
            pack: WorldPackRef::new(pack_id, "1.0.0"),
            display_title: None,
            display_summary: None,
            world_time: 0,
            event_count: 0,
        };
        let documents = vec![
            summary("pocket-new", "pocket-universe"),
            summary("tiny", "tiny-society"),
            summary("pocket-old", "pocket-universe"),
            summary("company", "micro-company"),
        ];

        assert_eq!(
            world_pack_filter_counts(&documents),
            vec![
                ("pocket-universe".into(), 2),
                ("tiny-society".into(), 1),
                ("micro-company".into(), 1),
            ]
        );
        assert!(world_matches_pack_filter(&documents[0], None));
        assert!(world_matches_pack_filter(
            &documents[0],
            Some("pocket-universe")
        ));
        assert!(!world_matches_pack_filter(
            &documents[1],
            Some("pocket-universe")
        ));
        assert!(world_pack_filter_is_available(
            &documents,
            Some("tiny-society")
        ));
        assert!(!world_pack_filter_is_available(
            &documents,
            Some("missing-pack")
        ));
    }

    #[test]
    fn lineage_child_preview_is_bounded_without_losing_total_count() {
        let children = (0..6)
            .map(|index| WorldDocumentId::new(format!("child-{index}")).unwrap())
            .collect::<Vec<_>>();
        let (visible, hidden) = lineage_child_preview(&children);

        assert_eq!(visible.len(), LINEAGE_CHILD_PREVIEW_LIMIT);
        assert_eq!(visible[0].as_str(), "child-0");
        assert_eq!(visible[3].as_str(), "child-3");
        assert_eq!(hidden, 2);
    }

    #[test]
    fn world_summary_title_prefers_semantic_title_and_falls_back_cleanly() {
        let pack = WorldPackRef::new("pocket-universe", "0.10.0");
        let mut summary = WorldDocumentSummary {
            id: WorldDocumentId::new("mars").unwrap(),
            pack,
            display_title: Some("  Ares Pocket Colony  ".into()),
            display_summary: Some("  Current thread · Ridge Network  ".into()),
            world_time: 3,
            event_count: 7,
        };

        assert_eq!(
            world_summary_title(&summary, "Pocket Universe"),
            "Ares Pocket Colony"
        );
        assert_eq!(
            world_summary_description(&summary).as_deref(),
            Some("Current thread · Ridge Network")
        );
        summary.display_title = Some("   ".into());
        assert_eq!(
            world_summary_title(&summary, "Pocket Universe"),
            "Pocket Universe"
        );
        summary.display_title = None;
        assert_eq!(
            world_summary_title(&summary, "Pocket Universe"),
            "Pocket Universe"
        );
    }

    #[test]
    fn finding_a_world_matches_what_the_card_shows() {
        let query = normalize_world_search("  MAPLE  ");
        assert_eq!(query, "maple");

        assert!(world_matches_search(
            "Maple Street · 1987",
            "Pocket Universe",
            "pocket-universe-3",
            &query
        ));
        assert!(world_matches_search(
            "Ares Colony",
            "Pocket Universe",
            "maple-import",
            &query
        ));
        assert!(!world_matches_search(
            "Ares Colony",
            "Pocket Universe",
            "pocket-universe-3",
            &query
        ));

        // An empty box hides nothing.
        assert!(world_matches_search("Ares", "Tiny Society", "tiny-1", ""));
        // The World Pack's own title is searchable too.
        assert!(world_matches_search(
            "Ares",
            "Tiny Society",
            "tiny-1",
            &normalize_world_search("tiny")
        ));
    }

    #[test]
    fn ordering_by_name_ignores_case_and_stays_stable() {
        let card = |id: &str, title: &str| {
            (
                title.to_owned(),
                WorldDocumentSummary {
                    id: WorldDocumentId::new(id).unwrap(),
                    pack: WorldPackRef::new("pocket-universe", "1.0.0"),
                    display_title: Some(title.to_owned()),
                    display_summary: None,
                    world_time: 0,
                    event_count: 0,
                },
            )
        };
        let recent_order = vec![
            card("c", "zephyr"),
            card("a", "Maple Street"),
            card("b", "ares colony"),
            card("d", "Maple Street"),
        ];

        let mut untouched = recent_order.clone();
        sort_world_cards(&mut untouched, WorldSort::Recent);
        assert_eq!(
            untouched
                .iter()
                .map(|(_, document)| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["c", "a", "b", "d"],
            "Recent must leave the Library's own order alone"
        );

        let mut by_name = recent_order;
        sort_world_cards(&mut by_name, WorldSort::Name);
        assert_eq!(
            by_name
                .iter()
                .map(|(_, document)| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "a", "d", "c"],
            "equal names fall back to the file identity so the order is stable"
        );
    }

    #[test]
    fn the_my_worlds_heading_counts_what_is_hidden() {
        assert_eq!(my_worlds_title(7, 7), "My Worlds · 7");
        assert_eq!(my_worlds_title(2, 7), "My Worlds · 2/7");
        assert_eq!(my_worlds_title(0, 7), "My Worlds · 0/7");
    }

    #[test]
    fn renaming_reports_the_name_or_the_pack_title_it_fell_back_to() {
        let named = rename_result_message(Some("Maple Street"), "Pocket Universe");
        assert!(named.starts_with("Renamed to Maple Street."));
        assert!(named.contains("World → Reload"));
        let cleared = rename_result_message(None, "Pocket Universe");
        assert!(cleared.starts_with("Name cleared; this World is listed as Pocket Universe again."));
        assert!(cleared.contains("World → Reload"));
        assert_eq!(
            rename_placeholder("Pocket Universe"),
            "Pocket Universe — name this World"
        );
    }

    #[test]
    fn removal_says_where_the_file_went() {
        let message = removal_result_message("Maple Street");
        assert!(message.starts_with("Removed Maple Street."));
        assert!(message.contains(world_library::REMOVED_DIRECTORY));
    }

    #[test]
    fn unreadable_files_are_named_without_crowding_out_the_worlds() {
        let file = |name: &str| UnreadableWorldFile {
            file_name: name.into(),
            reason: "unexpected end of input".into(),
        };

        assert_eq!(unreadable_documents_note(&[]), None);
        assert_eq!(
            unreadable_documents_note(&[file("truncated.world")]).as_deref(),
            Some(
                "One file in your Worlds folder could not be read and is not listed: truncated.world."
            )
        );
        assert_eq!(
            unreadable_documents_note(&[file("a.world"), file("b.world")]).as_deref(),
            Some("2 files in your Worlds folder could not be read and are not listed: a.world, b.world.")
        );
        assert_eq!(
            unreadable_documents_note(&[
                file("a.world"),
                file("b.world"),
                file("c.world"),
                file("d.world"),
                file("e.world"),
            ])
            .as_deref(),
            Some(
                "5 files in your Worlds folder could not be read and are not listed: a.world, b.world, c.world, and 2 more."
            )
        );
    }

    #[test]
    fn suggested_world_file_name_prefers_semantic_unicode_title() {
        assert_eq!(
            suggested_world_file_name("  Maple Street · 1987  ", "pocket-universe-42"),
            "Maple Street · 1987.world"
        );
        assert_eq!(
            suggested_world_file_name("Ares / Ice: Colony?", "pocket-universe-42"),
            "Ares - Ice- Colony.world"
        );
    }

    #[test]
    fn suggested_world_file_name_falls_back_to_durable_identity() {
        assert_eq!(
            suggested_world_file_name("   ", "pocket-universe-42"),
            "pocket-universe-42.world"
        );
        assert_eq!(
            suggested_world_file_name("", "legacy.world.json"),
            "legacy.world"
        );
    }

    #[test]
    fn a_world_is_called_by_its_name_and_falls_back_to_its_durable_identity() {
        assert_eq!(
            document_display_name(Some("  Maple Street · 1987  "), "pocket-universe-42"),
            "Maple Street · 1987"
        );
        assert_eq!(
            document_display_name(Some("   "), "pocket-universe-42"),
            "pocket-universe-42"
        );
        assert_eq!(
            document_display_name(None, "pocket-universe-42"),
            "pocket-universe-42"
        );
    }

    #[test]
    fn document_window_title_carries_what_the_world_is_called() {
        assert_eq!(
            document_window_title("pocket-universe-42"),
            "pocket-universe-42 — World Machine"
        );
        assert_eq!(
            document_window_title("Maple Street · 1987"),
            "Maple Street · 1987 — World Machine"
        );
    }

    #[test]
    fn start_intent_is_bound_to_exact_pack_identity() {
        let pocket_010 = WorldPackRef::new("pocket-universe", "0.10.0");
        let same = WorldPackRef::new("pocket-universe", "0.10.0");
        let newer = WorldPackRef::new("pocket-universe", "0.11.0");
        let other = WorldPackRef::new("micro-company", "0.10.0");

        assert!(start_after_install_matches(Some(&pocket_010), &same));
        assert!(!start_after_install_matches(Some(&pocket_010), &newer));
        assert!(!start_after_install_matches(Some(&pocket_010), &other));
        assert!(!start_after_install_matches(None, &pocket_010));
    }

    #[test]
    fn system_open_distinguishes_world_documents_from_portable_packs() {
        assert!(is_world_file(Path::new("/tmp/example.world")));
        assert!(!is_world_pack_file(Path::new("/tmp/example.world")));

        assert!(is_world_pack_file(Path::new("/tmp/example.worldpack")));
        assert!(!is_world_file(Path::new("/tmp/example.worldpack")));

        assert!(!is_world_pack_file(Path::new(
            "/tmp/example.worldpack.backup"
        )));
        assert!(!is_world_pack_file(Path::new(
            "/tmp/example.world-pack.json"
        )));
    }
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui_platform::application;

    let application = application();
    system_open::install(&application);
    diagnostics::init();
    load_window_geometry();
    let library = Arc::new(discover_library()?);
    let pack_catalog_path = discover_pack_catalog_path(library.as_ref());
    diagnostics::info(format!(
        "worlds at {} · packs catalog at {}",
        library.root().display(),
        pack_catalog_path.display()
    ));
    let (pack_catalog, registry, pack_status) = match PackCatalog::open(&pack_catalog_path) {
        Ok(catalog) => match build_registry(Some(&catalog)) {
            Ok(registry) => (Some(catalog), Arc::new(registry), None),
            Err(error) => (
                Some(catalog),
                Arc::new(world_builtins::registry()?),
                Some(format!("Installed Packs were not activated: {error}")),
            ),
        },
        Err(error) => (
            None,
            Arc::new(world_builtins::registry()?),
            Some(format!(
                "Could not open Installed Packs catalog {}: {error}",
                pack_catalog_path.display()
            )),
        ),
    };
    let (included_packs, included_status) = match included_packs::discover() {
        Ok(packs) => (packs, None),
        Err(error) => (
            Vec::new(),
            Some(format!("Could not locate included World Packs: {error}")),
        ),
    };
    let (documents, unreadable_documents, lineage, library_status) = match library.listing() {
        Ok(listing) => match LineageIndex::from_library(library.as_ref()) {
            Ok(lineage) => (listing.documents, listing.unreadable, Some(lineage), None),
            Err(error) => (
                listing.documents,
                listing.unreadable,
                None,
                Some(format!("Could not build World lineage: {error}")),
            ),
        },
        Err(error) => (
            Vec::new(),
            Vec::new(),
            None,
            Some(format!("Could not read World Library: {error}")),
        ),
    };
    report_unreadable_documents(&unreadable_documents);

    diagnostics::record_environment(diagnostics::Environment {
        library_dir: Some(library.root().to_path_buf()),
        pack_catalog_path: Some(pack_catalog_path.clone()),
        included_packs: included_packs
            .iter()
            .map(|pack| format!("{} {}", pack.pack.id, pack.pack.version))
            .collect(),
    });
    diagnostics::info(format!(
        "{} saved World(s) · {} included Pack(s)",
        documents.len(),
        included_packs.len()
    ));

    let status = pack_status
        .or(library_status)
        .or(included_status)
        .map(HomeStatus::error);

    // Clicking the Dock icon after the last window was closed brings Home
    // back, the way a document-based Mac app behaves.
    application.on_reopen(|cx| {
        if cx.windows().is_empty() {
            if let Some(home) = cx.try_global::<HomeEntity>().map(|home| home.0.clone()) {
                open_home_window(home, cx);
            }
        }
    });

    application.run(move |cx: &mut App| {
        about::install(cx);
        analyst_input::bind_keys(cx);
        let home = cx.new(|cx| {
            let world_search = cx.new(|cx| AnalystTextInput::new("Find a World…", cx));
            // Typing filters the list, so Home has to redraw as the field changes.
            cx.observe(&world_search, |_, _, cx| cx.notify()).detach();
            let mut home = WorldMachineHome {
                registry,
                library,
                pack_catalog,
                pack_catalog_path,
                documents,
                selected_world_pack: None,
                lineage,
                included_packs,
                pending_pack_install: None,
                pending_start_after_install: None,
                ready_pack_to_create: None,
                probing_packs: Vec::new(),
                status,
                show_packs: false,
                available_update: None,
                unreadable_documents,
                renaming: None,
                pending_removal: None,
                world_sort: WorldSort::Recent,
                world_search,
            };
            home.start_system_open_listener(cx);
            home.activate_included_packs(cx);
            home.start_update_check(cx);
            home
        });
        about::install_home_actions(&home, cx);
        cx.set_global(HomeEntity(home.clone()));
        open_home_window(home, cx);
        cx.activate(true);
    });

    Ok(())
}

/// The one Home entity, kept alive across window closes so the library
/// listener and Pack activation state survive Cmd-W.
#[cfg(target_os = "macos")]
struct HomeEntity(Entity<WorldMachineHome>);

#[cfg(target_os = "macos")]
impl Global for HomeEntity {}

#[cfg(target_os = "macos")]
fn open_home_window(home: Entity<WorldMachineHome>, cx: &mut App) {
    let bounds = remembered_window_bounds(RememberedWindow::Home, cx)
        .unwrap_or_else(|| Bounds::centered(None, size(px(760.0), px(760.0)), cx));
    if let Err(error) = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |window, _| {
            watch_appearance(window);
            home
        },
    ) {
        diagnostics::error(format!("could not open the Home window: {error}"));
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("world-machine-desktop currently targets macOS; the Host layer is cross-platform");
}
