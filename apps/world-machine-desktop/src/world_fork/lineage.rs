use super::super::{mark_library_changed, observer, SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, size, AppContext, Bounds, Context, IntoElement, Styled, WindowBounds,
    WindowOptions,
};
use std::sync::Arc;
use world_document::WorldLineage;
use world_gpui::ui;
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_lineage::LineageIndex;
use world_lineage_gpui::{LineageController, LineageExplorerView};
use world_theme::tokens;

const LINEAGE_BADGE_MAX_CHARS: usize = 34;

pub(super) fn compare_with_parent(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<(String, String), String> {
    let (current, registry, library) = {
        let document = document.borrow();
        let current = document
            .session
            .document_id()
            .cloned()
            .ok_or_else(|| "Only My Worlds documents can resolve a local parent".to_string())?;
        (
            current,
            Arc::clone(&document.registry),
            Arc::clone(&document.library),
        )
    };

    let index = LineageIndex::from_library(library.as_ref()).map_err(|error| error.to_string())?;
    let node = index
        .node(&current)
        .ok_or_else(|| format!("{current} is not present in the current lineage index"))?;
    let parent = node
        .parent
        .as_ref()
        .ok_or_else(|| format!("{current} is a root World and has no parent"))?;
    let parent_id = parent.resolved.clone().ok_or_else(|| {
        let label = parent
            .document
            .as_deref()
            .unwrap_or(parent.pack.id.as_str());
        format!("{label} is outside My Worlds or no longer matches this Pack")
    })?;

    super::saved_compare::open_saved_comparison(
        library.as_ref(),
        registry.as_ref(),
        &parent_id,
        &current,
        cx,
    )
}

/// Where a branch came from, as one line for the title bar: the parent's
/// name when it is in My Worlds, its file name or Pack otherwise. The choice
/// that made it is one click away, in Branches.
pub(super) fn lineage_label(lineage: &WorldLineage, parent_title: Option<&str>) -> String {
    let parent = parent_title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .or(lineage.parent.document.as_deref())
        .unwrap_or(lineage.parent.pack.id.as_str());
    format!("Branched from {parent}")
}

/// A quiet pill the height of the buttons beside it, linking to Branches.
pub(super) fn lineage_badge(label: &str) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .max_w(px(260.0))
        .px_3()
        .py(px(6.0))
        .rounded_full()
        .bg(ui::color(tokens::ACCENT_SOFT))
        .hover(|badge| badge.bg(ui::color(tokens::ROW_HOVER)))
        .text_sm()
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(ui::color(tokens::ACCENT_TEXT))
        .child(truncate_for_chrome(label, LINEAGE_BADGE_MAX_CHARS))
}

fn truncate_for_chrome(label: &str, max_chars: usize) -> String {
    let count = label.chars().count();
    if count <= max_chars {
        return label.to_owned();
    }
    if max_chars == 0 {
        return String::new();
    }
    if max_chars == 1 {
        return "…".into();
    }

    let mut compact = label.chars().take(max_chars - 1).collect::<String>();
    compact.push('…');
    compact
}

pub(super) fn open_lineage(
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
                Ok(Some(outcome)) => {
                    mark_library_changed();
                    Some(format!(
                        "Advanced {} background period(s) · World time {}",
                        outcome.periods, outcome.world_time
                    ))
                }
                Ok(None) => None,
                Err(error) => Some(format!("Catch-up skipped: {error}")),
            };
        let registry = Arc::clone(&self.registry);
        let library = Arc::clone(&self.library);
        let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|cx| WorldDocumentView::new(session, registry, library, cx)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use world_document::{WorldBranchCause, WorldParent};
    use world_persistence::WorldPackRef;

    #[test]
    fn lineage_label_describes_strategy_origin() {
        let lineage = WorldLineage {
            parent: WorldParent {
                document: Some("Source.world".into()),
                pack: WorldPackRef::new("world-machine.lineage-mock", "1"),
                world_time: 42,
                event_count: 3,
            },
            branch: WorldBranchCause::Strategy {
                choice_id: "mock.choose-a".into(),
                choice_title: "Choose A".into(),
                horizon: 20,
            },
        };

        assert_eq!(lineage_label(&lineage, None), "Branched from Source.world");
        assert_eq!(
            lineage_label(&lineage, Some(" Ares · Held on ")),
            "Branched from Ares · Held on"
        );
    }

    #[test]
    fn lineage_label_describes_fork_origin_and_falls_back_to_pack() {
        let lineage = WorldLineage {
            parent: WorldParent {
                document: None,
                pack: WorldPackRef::new("world-machine.parent", "1"),
                world_time: 7,
                event_count: 1,
            },
            branch: WorldBranchCause::Fork {
                label: Some("experiment".into()),
            },
        };

        assert_eq!(
            lineage_label(&lineage, None),
            "Branched from world-machine.parent"
        );
    }

    #[test]
    fn lineage_badge_text_is_unicode_safe_and_bounded() {
        let long = "From 世界世界世界世界世界 · a very long strategy choice · +20";
        let compact = truncate_for_chrome(long, 18);
        assert_eq!(compact.chars().count(), 18);
        assert!(compact.ends_with('…'));
        assert_eq!(truncate_for_chrome("short", 18), "short");
    }
}
