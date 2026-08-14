#[cfg(target_os = "macos")]
mod included_packs;
#[cfg(target_os = "macos")]
mod observer;
#[cfg(target_os = "macos")]
mod strategy_compare;
#[cfg(target_os = "macos")]
mod system_open;
#[cfg(target_os = "macos")]
mod world_fork;

#[cfg(target_os = "macos")]
use gpui::{
    div, prelude::*, px, rgb, size, App, AppContext, Bounds, Context, Entity, IntoElement,
    PathPromptOptions, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
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
    Arc,
};
#[cfg(target_os = "macos")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(target_os = "macos")]
use world_document::WorldBranchCause;
#[cfg(target_os = "macos")]
use world_library::{
    DurableWorldSession, LibraryError, WorldDocumentId, WorldDocumentSummary, WorldLibrary,
    LEGACY_WORLD_DOCUMENT_SUFFIX, WORLD_DOCUMENT_SUFFIX,
};
#[cfg(target_os = "macos")]
use world_lineage::LineageIndex;
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
        Self {
            message: message.into(),
            tone: DocumentStatusTone::Error,
        }
    }
}

#[cfg(target_os = "macos")]
struct WorldDocumentView {
    document_label: String,
    document: SharedDocument,
    projection: Entity<world_gpui::ProjectionView>,
    status: Option<DocumentStatus>,
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
        let document = Rc::new(RefCell::new(SharedDocumentState {
            session,
            registry,
            library,
        }));
        let controller = HostProjectionController {
            document: Rc::clone(&document),
        };
        let projection = cx.new(|_| world_gpui::ProjectionView::controlled(controller));
        Self {
            document_label,
            document,
            projection,
            status: None,
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
                self.rebuild_projection(cx);
                self.status = Some(DocumentStatus::success(format!(
                    "Reloaded {} · World time {}",
                    self.document_label, snapshot.world_time
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
        let suggested_name = suggested_world_file_name(&semantic_title, &self.document_label);
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
                        this.document_label = this.document.borrow().session.display_name();
                        this.rebuild_projection(cx);
                        this.status = Some(DocumentStatus::success(format!(
                            "Saved As {} · World time {}",
                            this.document_label, snapshot.world_time
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
}

#[cfg(target_os = "macos")]
impl Render for WorldDocumentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title(&document_window_title(&self.document_label));
        let actions = div()
            .flex()
            .gap_2()
            .child(world_fork::document_action(&self.document, cx))
            .child(strategy_compare::document_actions(&self.document, cx));

        let mut chrome = div()
            .h(px(48.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .border_b_1()
            .border_color(rgb(0xd9d9d3))
            .bg(rgb(0xf7f7f3))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(div().text_xs().text_color(rgb(0x777770)).child("DOCUMENT"))
                    .child(div().text_sm().child(self.document_label.clone())),
            )
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
                    .text_color(rgb(foreground))
                    .child(status.message.clone()),
            );
        }

        div().size_full().flex().flex_col().child(chrome).child(
            div()
                .flex_1()
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
        Self {
            message: message.into(),
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
    lineage: Option<LineageIndex>,
    included_packs: Vec<included_packs::IncludedPack>,
    pending_pack_install: Option<PackInstallPreview>,
    pending_start_after_install: Option<WorldPackRef>,
    ready_pack_to_create: Option<WorldPackRef>,
    probing_packs: Vec<WorldPackRef>,
    status: Option<HomeStatus>,
}

#[cfg(target_os = "macos")]
impl WorldMachineHome {
    fn start_system_open_listener(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let mut observed_library_revision = library_change_revision();
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;

                let Some(this) = this.upgrade() else {
                    return;
                };
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
                self.start_pack_probe(installed.pack, true, start_after_install, cx);
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
                                    if activate_on_success {
                                        this.ready_pack_to_create = Some(pack.clone());
                                    }
                                    this.status = Some(HomeStatus::success(format!(
                                        "Trusted and tested {} @ {} · durable Create/Archive/Open succeeded · World time {} → {}",
                                        pack.id,
                                        pack.version,
                                        probe.created_world_time,
                                        probe.reopened_world_time
                                    )));
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
        let documents = self
            .library
            .list()
            .map_err(|error| HomeStatus::error(format!("Could not read World Library: {error}")))?;
        let count = documents.len();
        self.documents = documents;
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

    fn sync_documents_after_mutation(&mut self) -> Option<HomeStatus> {
        self.refresh_documents().err()
    }

    fn open_session(
        &mut self,
        mut session: DurableWorldSession,
        title: String,
        cx: &mut Context<Self>,
    ) {
        let is_library_world = session.document_id().is_some();
        let catch_up = observer::catch_up(&mut session, &self.registry, &self.library);
        let sync_error = if is_library_world && matches!(&catch_up, Ok(Some(_))) {
            self.refresh_documents().err()
        } else {
            None
        };
        let document_label = session.display_name();
        let registry = Arc::clone(&self.registry);
        let library = Arc::clone(&self.library);
        let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|cx| WorldDocumentView::new(session, registry, library, cx)),
        );

        self.status = Some(match opened {
            Ok(_) => match catch_up {
                Ok(Some(outcome)) => HomeStatus::success(format!(
                    "Opened {title} · {document_label} · Advanced {} background period(s) · World time {}",
                    outcome.periods, outcome.world_time
                )),
                Ok(None) => HomeStatus::success(format!("Opened {title} · {document_label}")),
                Err(error) => HomeStatus::info(format!(
                    "Opened {title} · {document_label} · Catch-up skipped: {error}"
                )),
            },
            Err(error) => HomeStatus::error(format!("Could not open {title}: {error}")),
        });
        if let Some(status) = sync_error {
            self.status = Some(status);
        }
        cx.notify();
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
        let export_id = document.id.clone();
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
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_lg().child(title.clone()))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
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
                    .text_color(rgb(0x8a8a82))
                    .child(document_label.clone()),
            );

        if let Some(node) = lineage_node {
            if let Some(parent) = node.parent.as_ref() {
                let branch_label = node.branch.as_ref().map(lineage_branch_label);
                let mut origin = div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .child(div().text_color(rgb(0x777770)).child("Origin"));

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
                            .child(div().text_color(rgb(0x4e6fb3)).child(parent_title))
                            .child(div().text_color(rgb(0x8a8a82)).child(parent_label))
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
                            .text_color(rgb(0x777770))
                            .child(format!("{parent_label} · outside My Worlds")),
                    );
                }

                if let Some(branch_label) = branch_label {
                    origin = origin.child(
                        div()
                            .text_color(rgb(0x777770))
                            .child(format!("· {branch_label}")),
                    );
                }
                details = details.child(origin);
            }

            if !node.children.is_empty() {
                let mut branches = div().flex().flex_col().gap_1().child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x777770))
                        .child(format!("Branches · {}", node.children.len())),
                );
                for child_id in &node.children {
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
                            .child(div().text_color(rgb(0x4e6fb3)).child(child_title))
                            .child(div().text_color(rgb(0x8a8a82)).child(identity))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_document(open_child.clone(), cx)
                            })),
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
            .border_color(rgb(0xd9d9d3))
            .bg(rgb(0xffffff))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(details)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .id(SharedString::from(format!("open-{open_id}")))
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0xd9d9d3))
                            .text_sm()
                            .child("Open")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_document(open_id.clone(), cx)
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("export-{export_id}")))
                            .cursor_pointer()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0xd9d9d3))
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
            .border_color(rgb(0xa8b9d6))
            .bg(rgb(0xf1f5fb))
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
                            .text_color(rgb(0x5e6f91))
                            .child("START HERE · PERSISTENT SOCIAL WORLD"),
                    )
                    .child(div().text_lg().child(pack.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x4f5968))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x314b72))
                            .child(pack.experience),
                    )
                    .child(div().text_xs().text_color(rgb(0x71809a)).child(format!(
                        "{identity} · Included external World · reviewed before it runs"
                    ))),
            )
            .child(
                div()
                    .id("review-featured-included-world")
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x657da7))
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
            .border_color(rgb(0xc8d5c0))
            .bg(rgb(0xf7fbf5))
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
                            .text_color(rgb(0x666666))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66735f))
                            .child(pack.experience),
                    )
                    .child(div().text_xs().text_color(rgb(0x75806f)).child(format!(
                        "{identity} · Included external Pack · review required"
                    ))),
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
                    .border_color(rgb(0x91a486))
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
            .border_color(rgb(0x8eb58a))
            .bg(rgb(0xf1f8ee))
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
                    .child(
                        div().text_sm().text_color(rgb(0x52604d)).child(
                            "The Pack passed its durable probe and is active for new Worlds.",
                        ),
                    )
                    .child(div().text_xs().text_color(rgb(0x75806f)).child(format!(
                        "{} @ {} · no World has been created yet",
                        descriptor.pack.id, descriptor.pack.version
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
                            .border_color(rgb(0x6f966b))
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
                            .border_color(rgb(0xcbd8c7))
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
            .border_color(rgb(0xc7a85a))
            .bg(rgb(0xfffbeb))
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_lg().child(review_title))
            .child(div().text_lg().child(preview.title().to_owned()))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
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
                    .text_color(rgb(0x777770))
                    .child(format!("Source · {source}")),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x6f5420))
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
                            .border_color(rgb(0x8c6a23))
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
                            .border_color(rgb(0xd9d9d3))
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
                    .border_color(rgb(0xd9d9d3))
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
                    .border_color(rgb(0xd9d9d3))
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
                    .border_color(rgb(0xd9d9d3))
                    .text_sm()
                    .child("Test & Enable")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.start_pack_probe(test_pack.clone(), false, false, cx)
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
            .border_color(rgb(0xd9d9d3))
            .bg(rgb(0xffffff))
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
                            .text_color(rgb(0x666666))
                            .child(pack.description),
                    )
                    .child(div().text_xs().text_color(rgb(0x8a8a82)).child(format!(
                        "{} @ {} · {state}",
                        pack.pack.id, pack.pack.version
                    ))),
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
            .border_color(rgb(0xd9d9d3))
            .bg(rgb(0xffffff))
            .cursor_pointer()
            .child(div().text_lg().child(descriptor.title))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child(descriptor.description),
            )
            .child(div().text_xs().text_color(rgb(0x8a8a82)).child(format!(
                "{} @ {}",
                descriptor.pack.id, descriptor.pack.version
            )))
            .on_click(cx.listener(move |this, _, _, cx| this.create_world(pack_id.clone(), cx)))
    }
}

#[cfg(target_os = "macos")]
impl Render for WorldMachineHome {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title("World Machine");

        let documents = self.documents.clone();
        let has_documents = !documents.is_empty();
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
                    .border_color(rgb(0xe1e1dc))
                    .text_sm()
                    .text_color(rgb(0x777770))
                    .child("No saved Worlds yet. Create or import one below."),
            );
        } else {
            for document in documents {
                saved = saved.child(self.document_card(document, cx));
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

        let refresh = div()
            .id("refresh-world-library")
            .cursor_pointer()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd9d9d3))
            .text_sm()
            .child("Refresh")
            .on_click(cx.listener(|this, _, _, cx| {
                this.status = Some(match this.refresh_documents() {
                    Ok(count) => {
                        HomeStatus::success(format!("Refreshed My Worlds · {count} World(s)"))
                    }
                    Err(status) => status,
                });
                cx.notify();
            }));
        let install_pack = div()
            .id("install-world-pack")
            .cursor_pointer()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd9d9d3))
            .text_sm()
            .child("Install Pack…")
            .on_click(cx.listener(|this, _, _, cx| this.install_pack(cx)));
        let import = div()
            .id("import-world-file")
            .cursor_pointer()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd9d9d3))
            .text_sm()
            .child("Import .world…")
            .on_click(cx.listener(|this, _, _, cx| this.import_world(cx)));

        let header = div()
            .id("world-machine-home-chrome")
            .w_full()
            .p_4()
            .flex()
            .justify_between()
            .items_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child("World Machine"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("Persistent worlds that remember, evolve, and branch."),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(install_pack)
                    .child(import)
                    .child(refresh),
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
            body = body.child(div().text_sm().child("My Worlds")).child(saved);
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
            body = body
                .child(div().text_sm().child("Manage Packs"))
                .child(installed);
        }

        let mut shell = div()
            .size_full()
            .bg(rgb(0xf7f7f3))
            .text_color(rgb(0x202020))
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
                        .bg(rgb(background))
                        .text_color(rgb(foreground))
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
                                .border_color(rgb(foreground))
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
        registry
            .install_source(&source)
            .map_err(|error| error.to_string())?;
    }
    Ok(registry)
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
    fn world_summary_title_prefers_semantic_title_and_falls_back_cleanly() {
        let pack = WorldPackRef::new("pocket-universe", "0.10.0");
        let mut summary = WorldDocumentSummary {
            id: WorldDocumentId::new("mars").unwrap(),
            pack,
            display_title: Some("  Ares Pocket Colony  ".into()),
            world_time: 3,
            event_count: 7,
        };

        assert_eq!(
            world_summary_title(&summary, "Pocket Universe"),
            "Ares Pocket Colony"
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
    fn document_window_title_uses_stable_durable_identity() {
        assert_eq!(
            document_window_title("pocket-universe-42"),
            "pocket-universe-42 — World Machine"
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
    let library = Arc::new(discover_library()?);
    let pack_catalog_path = discover_pack_catalog_path(library.as_ref());
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
    let (documents, lineage, library_status) = match library.list() {
        Ok(documents) => match LineageIndex::from_library(library.as_ref()) {
            Ok(lineage) => (documents, Some(lineage), None),
            Err(error) => (
                documents,
                None,
                Some(format!("Could not build World lineage: {error}")),
            ),
        },
        Err(error) => (
            Vec::new(),
            None,
            Some(format!("Could not read World Library: {error}")),
        ),
    };

    let status = pack_status
        .or(library_status)
        .or(included_status)
        .map(HomeStatus::error);

    application.run(move |cx: &mut App| {
        let home = cx.new(|cx| {
            let mut home = WorldMachineHome {
                registry,
                library,
                pack_catalog,
                pack_catalog_path,
                documents,
                lineage,
                included_packs,
                pending_pack_install: None,
                pending_start_after_install: None,
                ready_pack_to_create: None,
                probing_packs: Vec::new(),
                status,
            };
            home.start_system_open_listener(cx);
            home
        });
        let bounds = Bounds::centered(None, size(px(760.0), px(760.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, _| home,
        )
        .expect("failed to open World Machine library window");
        cx.activate(true);
    });

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("world-machine-desktop currently targets macOS; the Host layer is cross-platform");
}
