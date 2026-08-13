use super::{sanitize_document_base, unique_document_id, SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, IntoElement, Styled, WindowBounds,
    WindowOptions,
};
use std::sync::Arc;
use world_library::{
    DurableWorldSession, WorldDocumentId, LEGACY_WORLD_DOCUMENT_SUFFIX, WORLD_DOCUMENT_SUFFIX,
};

mod lineage;
#[path = "saved_compare.rs"]
mod saved_compare;

struct ForkResult {
    id: WorldDocumentId,
    warning: Option<String>,
}

pub(crate) fn document_action(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> impl IntoElement {
    let fork_document = document.clone();
    let fork = div()
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
            this.status = Some(match fork_world(&fork_document, cx) {
                Ok(result) => match result.warning {
                    Some(warning) => format!("Forked as {} · {warning}", result.id),
                    None => format!("Forked as {}", result.id),
                },
                Err(error) => format!("Fork failed before saving: {error}"),
            });
            cx.notify();
        }));

    div()
        .flex()
        .gap_2()
        .child(fork)
        .child(saved_compare::document_action(document, cx))
        .child(lineage::document_action(document, cx))
}

fn fork_world(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<ForkResult, String> {
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

    // From this point on the fork is durable. Reopening, observer initialization,
    // or window creation failures must not be reported as if persistence failed.
    let mut session =
        match DurableWorldSession::open(document_id.clone(), registry.as_ref(), library.as_ref()) {
            Ok(session) => session,
            Err(error) => {
                return Ok(ForkResult {
                    id: document_id,
                    warning: Some(format!("saved, but could not reopen it: {error}")),
                });
            }
        };

    let observer_warning =
        super::observer::catch_up(&mut session, registry.as_ref(), library.as_ref())
            .err()
            .map(|error| format!("observer clock initialization skipped: {error}"));

    let registry_for_window = Arc::clone(&registry);
    let library_for_window = Arc::clone(&library);
    let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| {
            cx.new(|cx| {
                WorldDocumentView::new(session, registry_for_window, library_for_window, cx)
            })
        },
    );

    let warning = match (observer_warning, opened.err()) {
        (None, None) => None,
        (Some(observer), None) => Some(observer),
        (None, Some(error)) => Some(format!("saved, but could not open its window: {error}")),
        (Some(observer), Some(error)) => Some(format!(
            "{observer}; saved, but could not open its window: {error}"
        )),
    };

    Ok(ForkResult {
        id: document_id,
        warning,
    })
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
