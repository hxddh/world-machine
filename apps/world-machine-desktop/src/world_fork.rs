use super::{sanitize_document_base, unique_document_id, SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, IntoElement, Styled, WindowBounds,
    WindowOptions,
};
use std::sync::Arc;
use world_library::{
    DurableWorldSession, WorldDocumentId, LEGACY_WORLD_DOCUMENT_SUFFIX, WORLD_DOCUMENT_SUFFIX,
};

pub(crate) fn document_action(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> impl IntoElement {
    let document = document.clone();
    div()
        .id("fork-world-document")
        .cursor_pointer()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xb8b2d8))
        .bg(rgb(0xf7f5ff))
        .text_sm()
        .child("Fork World")
        .on_click(cx.listener(move |this, _, _, cx| {
            this.status = Some(match fork_world(&document, cx) {
                Ok(id) => format!("Forked as {id}"),
                Err(error) => format!("Fork failed: {error}"),
            });
            cx.notify();
        }))
}

fn fork_world(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<WorldDocumentId, String> {
    let (source_label, registry, library) = {
        let document = document.borrow();
        (
            document.session.display_name(),
            Arc::clone(&document.registry),
            Arc::clone(&document.library),
        )
    };
    let source_base = source_world_base(&source_label);
    let document_id = unique_document_id(
        sanitize_document_base(&format!("{source_base}-fork")),
        Some(library.as_ref()),
    )
    .map_err(|error| error.to_string())?;

    {
        let document = document.borrow();
        document
            .session
            .fork_to_library(document_id.clone(), None, library.as_ref())
            .map_err(|error| error.to_string())?;
    }

    let session =
        DurableWorldSession::open(document_id.clone(), registry.as_ref(), library.as_ref())
            .map_err(|error| error.to_string())?;
    let pack = session.pack();
    let title = registry
        .descriptor(&pack.id)
        .map(|descriptor| descriptor.title.clone())
        .unwrap_or(pack.id);
    let registry_for_window = Arc::clone(&registry);
    let library_for_window = Arc::clone(&library);
    let window_title = title.clone();
    let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| {
            cx.new(|cx| {
                WorldDocumentView::new(
                    session,
                    window_title,
                    registry_for_window,
                    library_for_window,
                    cx,
                )
            })
        },
    )
    .map_err(|error| error.to_string())?;

    Ok(document_id)
}

fn source_world_base(label: &str) -> &str {
    label
        .strip_suffix(LEGACY_WORLD_DOCUMENT_SUFFIX)
        .or_else(|| label.strip_suffix(WORLD_DOCUMENT_SUFFIX))
        .unwrap_or(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_portable_world_suffixes_before_generating_fork_ids() {
        assert_eq!(source_world_base("Source.world"), "Source");
        assert_eq!(source_world_base("Legacy.world.json"), "Legacy");
        assert_eq!(source_world_base("library-id"), "library-id");
    }
}
