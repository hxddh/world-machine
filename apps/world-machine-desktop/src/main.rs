#[cfg(target_os = "macos")]
mod system_open;

#[cfg(target_os = "macos")]
use gpui::{
    div, prelude::*, px, rgb, size, App, AppContext, Bounds, Context, IntoElement,
    PathPromptOptions, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
};
#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process;
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(target_os = "macos")]
use world_library::{
    DurableWorldSession, LibraryError, WorldDocumentId, WorldDocumentSummary, WorldLibrary,
    LEGACY_WORLD_DOCUMENT_SUFFIX, WORLD_DOCUMENT_SUFFIX,
};

#[cfg(target_os = "macos")]
const LIBRARY_OVERRIDE_ENV: &str = "WORLD_MACHINE_LIBRARY_DIR";

#[cfg(target_os = "macos")]
struct HostProjectionController {
    session: DurableWorldSession,
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
}

#[cfg(target_os = "macos")]
impl world_gpui::ProjectionController for HostProjectionController {
    fn snapshot(&self) -> world_gpui::ProjectionSnapshot {
        self.session.snapshot()
    }

    fn handle(
        &mut self,
        intent: world_gpui::ProjectionIntent,
    ) -> Result<world_gpui::ProjectionSnapshot, String> {
        self.session
            .handle(intent, &self.registry, &self.library)
            .map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
struct WorldMachineHome {
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
    documents: Vec<WorldDocumentSummary>,
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
            Ok(documents) => self.documents = documents,
            Err(error) => self.status = Some(format!("Could not read World Library: {error}")),
        }
    }

    fn open_session(
        &mut self,
        session: DurableWorldSession,
        title: String,
        cx: &mut Context<Self>,
    ) {
        let document_id = session.document_id().to_string();
        let controller = HostProjectionController {
            session,
            registry: Arc::clone(&self.registry),
            library: Arc::clone(&self.library),
        };
        let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| world_gpui::ProjectionView::controlled(controller)),
        );

        self.status = Some(match opened {
            Ok(_) => format!("Opened {title} · {document_id}"),
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
        self.import_path(source, cx);
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
            let _ = this.update(cx, |this, cx| this.open_external_path(source, cx));
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
            .child(
                div()
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
                            .child(document_label),
                    ),
            )
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                "Worlds are portable documents. Open, import, export, or create a World Pack instance.",
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
fn library_document_id_for_path(
    source: &Path,
    library: &WorldLibrary,
) -> Option<WorldDocumentId> {
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
    let (documents, status) = match library.list() {
        Ok(documents) => (documents, None),
        Err(error) => (
            Vec::new(),
            Some(format!("Could not read World Library: {error}")),
        ),
    };

    application.run(move |cx: &mut App| {
        let home = cx.new(|cx| {
            let mut home = WorldMachineHome {
                registry,
                library,
                documents,
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
