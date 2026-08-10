#[cfg(target_os = "macos")]
use gpui::{
    div, prelude::*, px, rgb, size, App, AppContext, Bounds, Context, IntoElement, Render,
    SharedString, Styled, Window, WindowBounds, WindowOptions,
};
#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::path::PathBuf;
#[cfg(target_os = "macos")]
use std::process;
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_os = "macos")]
use world_library::{DurableWorldSession, WorldDocumentId, WorldDocumentSummary, WorldLibrary};

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
        let document_id = match new_document_id(&pack_id) {
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

    fn document_card(
        &self,
        document: WorldDocumentSummary,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let document_id = document.id.clone();
        let title = self
            .registry
            .descriptor(&document.pack.id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document.pack.id.clone());
        div()
            .id(SharedString::from(format!("document-{document_id}")))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd9d9d3))
            .bg(rgb(0xffffff))
            .cursor_pointer()
            .child(div().text_lg().child(title))
            .child(div().text_sm().text_color(rgb(0x666666)).child(format!(
                "World time {} · {} events",
                document.world_time, document.event_count
            )))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8a8a82))
                    .child(document.id.to_string()),
            )
            .on_click(
                cx.listener(move |this, _, _, cx| this.open_document(document_id.clone(), cx)),
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
                    .child("No saved Worlds yet. Create one below."),
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
                    .child(refresh),
            )
            .child(div().text_sm().text_color(rgb(0x666666)).child(
                "Worlds are durable documents. Open one, or create a new World Pack instance.",
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
fn new_document_id(pack_id: &str) -> Result<WorldDocumentId, world_library::LibraryError> {
    let slug = pack_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .take(64)
        .collect::<String>();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    WorldDocumentId::new(format!("{slug}-{}-{nonce}", process::id()))
}

#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui_platform::application;

    let registry = Arc::new(world_builtins::registry()?);
    let library = Arc::new(discover_library()?);
    let (documents, status) = match library.list() {
        Ok(documents) => (documents, None),
        Err(error) => (
            Vec::new(),
            Some(format!("Could not read World Library: {error}")),
        ),
    };
    let home = WorldMachineHome {
        registry,
        library,
        documents,
        status,
    };

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(760.0), px(760.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| home),
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
