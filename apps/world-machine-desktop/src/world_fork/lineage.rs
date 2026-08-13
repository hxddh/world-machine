use super::super::{
    mark_library_changed, observer, DocumentStatus, SharedDocument, WorldDocumentView,
};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, IntoElement, Styled, WindowBounds,
    WindowOptions,
};
use std::sync::Arc;
use world_document::{WorldBranchCause, WorldLineage};
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_lineage::LineageIndex;
use world_lineage_gpui::{LineageController, LineageExplorerView};

const LINEAGE_BADGE_MAX_CHARS: usize = 34;

pub(super) fn document_action(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> impl IntoElement {
    let (document_id, lineage) = {
        let document = document.borrow();
        (
            document.session.document_id().cloned(),
            document.session.metadata().lineage.clone(),
        )
    };

    let mut actions = div().flex().items_center().gap_2();
    if let Some(lineage) = lineage.as_ref() {
        actions = actions.child(lineage_badge(lineage));
    }

    let Some(_document_id) = document_id else {
        return actions;
    };

    if lineage
        .as_ref()
        .and_then(|lineage| lineage.parent.document.as_ref())
        .is_some()
    {
        let document = document.clone();
        actions = actions.child(
            div()
                .id("compare-lineage-parent")
                .cursor_pointer()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xa9b7d5))
                .bg(rgb(0xf3f6fc))
                .text_sm()
                .child("↔ Parent")
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.status = Some(match compare_with_parent(&document, cx) {
                        Ok((parent, current)) => DocumentStatus::success(format!(
                            "Opened parent comparison · {parent} ↔ {current}"
                        )),
                        Err(error) => {
                            DocumentStatus::error(format!("Could not compare with parent: {error}"))
                        }
                    });
                    cx.notify();
                })),
        );
    }

    let document = document.clone();
    actions.child(
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
                    Ok(count) => {
                        DocumentStatus::success(format!("Opened World Lineage · {count} World(s)"))
                    }
                    Err(error) => {
                        DocumentStatus::error(format!("Could not open World Lineage: {error}"))
                    }
                });
                cx.notify();
            })),
    )
}

fn compare_with_parent(
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

fn lineage_badge(lineage: &WorldLineage) -> impl IntoElement {
    div()
        .w(px(190.0))
        .overflow_hidden()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xc8cdd7))
        .bg(rgb(0xf5f6f8))
        .text_xs()
        .text_color(rgb(0x5f6570))
        .child(truncate_for_chrome(
            &lineage_label(lineage),
            LINEAGE_BADGE_MAX_CHARS,
        ))
}

fn lineage_label(lineage: &WorldLineage) -> String {
    let parent = lineage
        .parent
        .document
        .as_deref()
        .unwrap_or(lineage.parent.pack.id.as_str());
    match &lineage.branch {
        WorldBranchCause::Strategy {
            choice_title,
            horizon,
            ..
        } => format!(
            "From {parent} · {choice_title} · +{horizon} · parent t{}",
            lineage.parent.world_time
        ),
        WorldBranchCause::Fork { label: Some(label) } => format!(
            "From {parent} · Fork {label} · parent t{}",
            lineage.parent.world_time
        ),
        WorldBranchCause::Fork { label: None } => {
            format!(
                "From {parent} · Fork · parent t{}",
                lineage.parent.world_time
            )
        }
    }
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
    use world_document::WorldParent;
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

        assert_eq!(
            lineage_label(&lineage),
            "From Source.world · Choose A · +20 · parent t42"
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
            lineage_label(&lineage),
            "From world-machine.parent · Fork experiment · parent t7"
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
