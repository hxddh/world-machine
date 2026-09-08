use super::{
    mark_library_changed, sanitize_document_base, unique_document_id, SharedDocument,
    WorldDocumentView,
};
use gpui::{
    prelude::*, px, size, AppContext, Bounds, Context, IntoElement, WindowBounds, WindowOptions,
};
use std::sync::Arc;
use world_library::{
    DurableWorldSession, WorldDocumentId, LEGACY_WORLD_DOCUMENT_SUFFIX, WORLD_DOCUMENT_SUFFIX,
};

/// The app's single-line text field. It grew up inside the Analyst, and Home
/// now uses the same field to name a World.
#[path = "analyst_input.rs"]
pub(crate) mod analyst_input;
#[path = "analyst_panel.rs"]
mod analyst_panel;
#[path = "analyst_runtime.rs"]
mod analyst_runtime;
mod lineage;
#[path = "saved_compare.rs"]
mod saved_compare;

pub(crate) struct ForkResult {
    pub(crate) id: WorldDocumentId,
    pub(crate) warning: Option<String>,
}

/// Resolve once whether the optional World Analyst runtime is installed.
/// Nothing else in the app depends on Node or Pi, so a missing runtime only
/// hides the Analyst entry instead of surfacing an error to the user.
pub(crate) fn analyst_available() -> bool {
    analyst_runtime::discover().is_ready()
}

/// Opens the experimental World Analyst for this saved World.
pub(crate) fn open_analyst(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<(), String> {
    analyst_panel::open_panel(document, cx)
}

/// Opens the saved-World comparison setup for this saved World.
pub(crate) fn open_saved_compare(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<usize, String> {
    saved_compare::open_setup(document, cx)
}

/// Opens the lineage explorer rooted at this saved World.
pub(crate) fn open_lineage(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<usize, String> {
    lineage::open_lineage(document, cx)
}

/// Compares this branch with the World it was branched from.
pub(crate) fn compare_with_parent(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<(String, String), String> {
    lineage::compare_with_parent(document, cx)
}

/// The small "branched from …" badge shown in the document header, if any.
pub(crate) fn lineage_badge(document: &SharedDocument) -> Option<impl IntoElement> {
    let lineage = document.borrow().session.metadata().lineage.clone()?;
    Some(lineage::lineage_badge(&lineage))
}

pub(crate) fn fork_world(
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
    mark_library_changed();

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
        match super::observer::catch_up(&mut session, registry.as_ref(), library.as_ref()) {
            Ok(Some(_)) => {
                mark_library_changed();
                None
            }
            Ok(None) => None,
            Err(error) => Some(format!("observer clock initialization skipped: {error}")),
        };

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
