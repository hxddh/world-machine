use super::super::{observer, SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, IntoElement, Styled, WindowBounds,
    WindowOptions,
};
use std::sync::Arc;
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_lineage::LineageIndex;
use world_lineage_gpui::{LineageController, LineageExplorerView};

pub(super) fn document_action(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> impl IntoElement {
    if document.borrow().session.document_id().is_none() {
        return div().id("world-lineage-unavailable");
    }

    let document = document.clone();
    div()
        .id("open-world-lineage")
        .cursor_pointer()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xb8c8d8))
        .bg(rgb(0xf4f8fb))
        .text_sm()
        .child("Lineage…")
        .on_click(cx.listener(move |this, _, _, cx| {
            this.status = Some(match open_lineage(&document, cx) {
                Ok(count) => format!("Opened World Lineage · {count} World(s)"),
                Err(error) => format!("Could not open World Lineage: {error}"),
            });
            cx.notify();
        }))
}

fn open_lineage(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<usize, String> {
    let (current, registry, library) = {
        let document = document.borrow();
        let current =
            document.session.document_id().cloned().ok_or_else(|| {
                "Import this World into My Worlds before opening lineage".to_string()
            })?;
        (
            current,
            Arc::clone(&document.registry),
            Arc::clone(&document.library),
        )
    };
    let index = LineageIndex::from_library(library.as_ref()).map_err(|error| error.to_string())?;
    let count = index.nodes().len();
    let selected = current.to_string();
    let controller = AppLineageController {
        registry,
        library,
        last_open_notice: None,
    };
    let bounds = Bounds::centered(None, size(px(1120.0), px(820.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| {
            cx.new(|_| LineageExplorerView::controlled_selected(index, selected, controller))
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(count)
}

struct AppLineageController {
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
    last_open_notice: Option<String>,
}

impl LineageController for AppLineageController {
    fn open_document(
        &mut self,
        document: &str,
        cx: &mut Context<LineageExplorerView>,
    ) -> Result<(), String> {
        self.last_open_notice = None;
        let document_id =
            WorldDocumentId::new(document.to_owned()).map_err(|error| error.to_string())?;
        let mut session =
            DurableWorldSession::open(document_id, self.registry.as_ref(), self.library.as_ref())
                .map_err(|error| error.to_string())?;
        let notice =
            match observer::catch_up(&mut session, self.registry.as_ref(), self.library.as_ref()) {
                Ok(Some(outcome)) => Some(format!(
                    "Advanced {} background period(s) · World time {}",
                    outcome.periods, outcome.world_time
                )),
                Ok(None) => None,
                Err(error) => Some(format!("Catch-up skipped: {error}")),
            };
        let pack = session.pack();
        let title = self
            .registry
            .descriptor(&pack.id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or(pack.id);
        let registry = Arc::clone(&self.registry);
        let library = Arc::clone(&self.library);
        let window_title = title.clone();
        let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|cx| WorldDocumentView::new(session, window_title, registry, library, cx))
            },
        )
        .map_err(|error| error.to_string())?;
        self.last_open_notice = notice;
        Ok(())
    }

    fn take_open_notice(&mut self) -> Option<String> {
        self.last_open_notice.take()
    }

    fn can_compare(&self) -> bool {
        true
    }

    fn compare_documents(
        &mut self,
        left: &str,
        right: &str,
        cx: &mut Context<LineageExplorerView>,
    ) -> Result<(), String> {
        let left_id = WorldDocumentId::new(left.to_owned()).map_err(|error| error.to_string())?;
        let right_id = WorldDocumentId::new(right.to_owned()).map_err(|error| error.to_string())?;
        super::saved_compare::open_saved_comparison(
            self.library.as_ref(),
            self.registry.as_ref(),
            &left_id,
            &right_id,
            cx,
        )?;
        Ok(())
    }
}
