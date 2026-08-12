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
use std::sync::Arc;
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
const LIBRARY_OVERRIDE_ENV: &str = "WORLD_MACHINE_LIBRARY_DIR";

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
        document
            .session
            .handle(intent, &registry, &library)
            .map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
struct WorldDocumentView {
    title: String,
    document_label: String,
    document: SharedDocument,
    projection: Entity<world_gpui::ProjectionView>,
    status: Option<String>,
}

#[cfg(target_os = "macos")]
impl WorldDocumentView {
    fn new(
        session: DurableWorldSession,
        title: String,
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
            title,
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
                self.rebuild_projection(cx);
                self.status = Some(format!(
                    "Reloaded {} · World time {}",
                    self.document_label, snapshot.world_time
                ));
            }
            Err(error) => {
                self.status = Some(format!("Reload failed: {error}"));
            }
        }
        cx.notify();
    }

    fn save_as(&mut self, cx: &mut Context<Self>) {
        let suggested_name = canonical_world_name(&self.document_label);
        let save_dialog = cx.prompt_for_new_path(&PathBuf::default(), Some(&suggested_name));
        cx.spawn(async move |this, cx| {
            let destination = match save_dialog.await {
                Ok(Ok(Some(path))) => canonical_world_path(path),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(format!("Could not open Save As dialog: {error}"));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(format!("Save As dialog was interrupted: {error}"));
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
                        this.status = Some(format!(
                            "Saved As {} · World time {}",
                            this.document_label, snapshot.world_time
                        ));
                    }
                    Err(error) => {
                        this.status = Some(format!("Save As failed: {error}"));
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
        window.set_window_title(&format!("{} — {}", self.document_label, self.title));
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
                    .child(div().text_sm().child(self.title.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777770))
                            .child(self.document_label.clone()),
                    ),
            )
            .child(actions);

        if let Some(status) = &self.status {
            chrome = chrome.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x4e6fb3))
                    .child(status.clone()),
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
struct WorldMachineHome {
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
    documents: Vec<WorldDocumentSummary>,
    lineage: Option<LineageIndex>,
    status: Option<String>,
}

#[cfg(target_os = "macos")]
impl WorldMachineHome {
    fn start_system_open_listener(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(200))
                .await;

            let Some(this) = this.upgrade() else {
                return;
            };
            let paths = system_open::drain_paths();
            if paths.is_empty() {
                continue;
            }
            this.update(cx, |this, cx| {
                for path in paths {
                    match path {
                        Ok(path) => this.open_external_path(path, cx),
                        Err(error) => {
                            this.status = Some(format!("Could not open World file: {error}"));
                            cx.notify();
                        }
                    }
                }
            });
        })
        .detach();
    }

    fn refresh_documents(&mut self) {
        match self.library.list() {
            Ok(documents) => {
                self.documents = documents;
                self.refresh_lineage();
            }
            Err(error) => self.status = Some(format!("Could not read World Library: {error}")),
        }
    }

    fn refresh_lineage(&mut self) {
        match LineageIndex::from_library(self.library.as_ref()) {
            Ok(lineage) => self.lineage = Some(lineage),
            Err(error) => {
                self.lineage = None;
                self.status = Some(format!("Could not build World lineage: {error}"));
            }
        }
    }

    fn open_session(
        &mut self,
        mut session: DurableWorldSession,
        title: String,
        cx: &mut Context<Self>,
    ) {
        let catch_up = observer::catch_up(&mut session, &self.registry, &self.library);
        let document_label = session.display_name();
        let registry = Arc::clone(&self.registry);
        let library = Arc::clone(&self.library);
        let window_title = title.clone();
        let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|cx| WorldDocumentView::new(session, window_title, registry, library, cx))
            },
        );

        self.status = Some(match opened {
            Ok(_) => match catch_up {
                Ok(Some(outcome)) => format!(
                    "Opened {title} · {document_label} · Advanced {} background period(s) · World time {}",
                    outcome.periods, outcome.world_time
                ),
                Ok(None) => format!("Opened {title} · {document_label}"),
                Err(error) => {
                    format!("Opened {title} · {document_label} · Catch-up skipped: {error}")
                }
            },
            Err(error) => format!("Could not open {title}: {error}"),
        });
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
                self.status = Some(format!("Could not create {title}: {error}"));
                cx.notify();
                return;
            }
        };
        let session =
            match DurableWorldSession::create(document_id, &pack_id, &self.registry, &self.library)
            {
                Ok(session) => session,
                Err(error) => {
                    self.status = Some(format!("Could not create {title}: {error}"));
                    cx.notify();
                    return;
                }
            };
        self.refresh_documents();
        self.open_session(session, title, cx);
    }

    fn open_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        let title = self
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .and_then(|document| self.registry.descriptor(&document.pack.id))
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document_id.to_string());
        let session = match DurableWorldSession::open(document_id, &self.registry, &self.library) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(format!("Could not open {title}: {error}"));
                cx.notify();
                return;
            }
        };
        self.open_session(session, title, cx);
    }

    fn open_external_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if let Some(document_id) = library_document_id_for_path(&source, &self.library) {
            self.open_document(document_id, cx);
            return;
        }
        if !is_world_file(&source) {
            self.status = Some(format!(
                "Could not open {}: choose a {} file",
                source.display(),
                WORLD_DOCUMENT_SUFFIX
            ));
            cx.notify();
            return;
        }
        let session = match DurableWorldSession::open_file(source.clone(), &self.registry) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(format!("Could not open {}: {error}", source.display()));
                cx.notify();
                return;
            }
        };
        let pack = session.pack();
        let title = self
            .registry
            .descriptor(&pack.id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or(pack.id);
        self.open_session(session, title, cx);
    }

    fn import_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if !is_world_file(&source) {
            self.status = Some(format!(
                "Could not import {}: choose a {} file",
                source.display(),
                WORLD_DOCUMENT_SUFFIX
            ));
            cx.notify();
            return;
        }
        let document_id = match imported_document_id(&source, &self.library) {
            Ok(id) => id,
            Err(error) => {
                self.status = Some(format!("Could not import {}: {error}", source.display()));
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
                self.status = Some(format!("Could not import {}: {error}", source.display()));
                cx.notify();
                return;
            }
        };
        let pack = session.pack();
        let title = self
            .registry
            .descriptor(&pack.id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or(pack.id);
        self.refresh_documents();
        self.open_session(session, title, cx);
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
                        this.status = Some(format!("Could not open Import dialog: {error}"));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(format!("Import dialog was interrupted: {error}"));
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
        let suggested_name = format!("{}{WORLD_DOCUMENT_SUFFIX}", document_id.as_str());
        let save_dialog = cx.prompt_for_new_path(&PathBuf::default(), Some(&suggested_name));
        cx.spawn(async move |this, cx| {
            let destination = match save_dialog.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(format!("Could not open Export dialog: {error}"));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(format!("Export dialog was interrupted: {error}"));
                        cx.notify();
                    });
                    return;
                }
            };

            let _ = this.update(cx, |this, cx| {
                match this.library.export_file(&document_id, &destination) {
                    Ok(()) => {
                        this.status = Some(format!(
                            "Exported {} to {}",
                            document_id,
                            destination.display()
                        ));
                    }
                    Err(error) => {
                        this.status = Some(format!("Could not export {document_id}: {error}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn document_card(
        &self,
        document: WorldDocumentSummary,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open_id = document.id.clone();
        let export_id = document.id.clone();
        let title = self
            .registry
            .descriptor(&document.pack.id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document.pack.id.clone());
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
            .child(div().text_lg().child(title))
            .child(div().text_sm().text_color(rgb(0x666666)).child(format!(
                "World time {} · {} events",
                document.world_time, document.event_count
            )))
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
                    let open_parent = parent_id.clone();
                    let parent_label = parent_id.to_string();
                    origin = origin.child(
                        div()
                            .id(SharedString::from(format!(
                                "lineage-parent-{document_label}-{parent_label}"
                            )))
                            .cursor_pointer()
                            .text_color(rgb(0x4e6fb3))
                            .child(parent_label)
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
                    let child_branch = self
                        .lineage
                        .as_ref()
                        .and_then(|lineage| lineage.node(child_id))
                        .and_then(|child| child.branch.as_ref())
                        .map(lineage_branch_label);
                    let open_child = child_id.clone();
                    let link_label = child_branch
                        .map(|branch| format!("{child_label} · {branch}"))
                        .unwrap_or_else(|| child_label.clone());
                    branches = branches.child(
                        div()
                            .id(SharedString::from(format!(
                                "lineage-child-{document_label}-{child_label}"
                            )))
                            .cursor_pointer()
                            .text_xs()
                            .text_color(rgb(0x4e6fb3))
                            .child(link_label)
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
        let descriptors = self
            .registry
            .descriptors()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        let mut saved = div().w_full().flex().flex_col().gap_3();
        if documents.is_empty() {
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
                this.refresh_documents();
                cx.notify();
            }));
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

        let mut body = div()
            .size_full()
            .bg(rgb(0xf7f7f3))
            .text_color(rgb(0x202020))
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(div().text_lg().child("World Machine"))
                    .child(div().flex().gap_2().child(import).child(refresh)),
            )
            .child(div().text_sm().text_color(rgb(0x666666)).child(
                "Worlds are portable documents. Double-click an external .world to edit it in place; Import copies it into My Worlds.",
            ))
            .child(div().text_sm().child("My Worlds"))
            .child(saved)
            .child(div().text_sm().child("New World"))
            .child(available);

        if let Some(status) = &self.status {
            body = body.child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0xeef2ea))
                    .text_sm()
                    .child(status.clone()),
            );
        }
        body
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
fn canonical_world_name(label: &str) -> String {
    if label.ends_with(WORLD_DOCUMENT_SUFFIX) {
        label.to_owned()
    } else if let Some(base) = label.strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX) {
        format!("{base}{WORLD_DOCUMENT_SUFFIX}")
    } else {
        format!("{label}{WORLD_DOCUMENT_SUFFIX}")
    }
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui_platform::application;

    let application = application();
    system_open::install(&application);
    let registry = Arc::new(world_builtins::registry()?);
    let library = Arc::new(discover_library()?);
    let (documents, lineage, status) = match library.list() {
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

    application.run(move |cx: &mut App| {
        let home = cx.new(|cx| {
            let mut home = WorldMachineHome {
                registry,
                library,
                documents,
                lineage,
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
