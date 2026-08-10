#[cfg(target_os = "macos")]
use gpui::{
    div, prelude::*, px, rgb, size, App, AppContext, Bounds, Context, IntoElement, Render,
    SharedString, Styled, Window, WindowBounds, WindowOptions,
};

#[cfg(target_os = "macos")]
struct HostProjectionController {
    session: Box<dyn world_host::WorldSession>,
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
            .handle(intent)
            .map_err(|error| error.to_string())
    }
}

#[cfg(target_os = "macos")]
struct WorldMachineHome {
    registry: world_host::WorldRegistry,
    status: Option<String>,
}

#[cfg(target_os = "macos")]
impl WorldMachineHome {
    fn open_world(&mut self, pack_id: String, cx: &mut Context<Self>) {
        let title = self
            .registry
            .descriptor(&pack_id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| pack_id.clone());

        let session = match self.registry.create(&pack_id) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(format!("Could not open {title}: {error}"));
                cx.notify();
                return;
            }
        };
        let controller = HostProjectionController { session };
        let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|_| world_gpui::ProjectionView::controlled(controller)),
        );

        self.status = Some(match opened {
            Ok(_) => format!("Opened {title}"),
            Err(error) => format!("Could not open {title}: {error}"),
        });
        cx.notify();
    }

    fn world_card(
        &self,
        descriptor: world_host::WorldDescriptor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pack_id = descriptor.pack.id.clone();
        div()
            .id(SharedString::from(format!("world-{pack_id}")))
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
            .on_click(cx.listener(move |this, _, _, cx| this.open_world(pack_id.clone(), cx)))
    }
}

#[cfg(target_os = "macos")]
impl Render for WorldMachineHome {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let descriptors = self
            .registry
            .descriptors()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut worlds = div().w_full().flex().flex_col().gap_3();
        for descriptor in descriptors {
            worlds = worlds.child(self.world_card(descriptor, cx));
        }

        let mut body =
            div()
                .size_full()
                .bg(rgb(0xf7f7f3))
                .text_color(rgb(0x202020))
                .flex()
                .flex_col()
                .gap_3()
                .p_4()
                .child(div().text_lg().child("World Machine"))
                .child(div().text_sm().text_color(rgb(0x666666)).child(
                    "Open a World. Each one runs through the same Host and Projection shell.",
                ))
                .child(worlds);

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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui_platform::application;

    let home = WorldMachineHome {
        registry: world_builtins::registry()?,
        status: None,
    };

    application().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(720.0), px(620.0)), cx);
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
