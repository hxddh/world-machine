use crate::{SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, IntoElement, SharedString, Styled,
    WindowBounds, WindowOptions,
};
use std::sync::Arc;
use world_document::WorldBranchCause;
use world_library::{WorldDocumentId, WorldDocumentSummary, WorldLibrary};
use world_lineage_compare::{compare_saved_worlds, SavedWorldRelation};
use world_strategy_gpui::{SavedComparisonContext, StrategyComparisonView};

pub(super) fn document_action(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> impl IntoElement {
    if document.borrow().session.document_id().is_none() {
        return div().id("compare-saved-world-unavailable");
    }

    let document = document.clone();
    div()
        .id("compare-saved-world")
        .cursor_pointer()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x9eb0d6))
        .bg(rgb(0xf4f7ff))
        .text_sm()
        .child("Compare saved Worlds…")
        .on_click(cx.listener(move |this, _, _, cx| {
            this.status = Some(match open_setup(&document, cx) {
                Ok(count) => format!("Opened saved World comparison · {count} Worlds"),
                Err(error) => format!("Could not compare saved Worlds: {error}"),
            });
            cx.notify();
        }))
}

fn open_setup(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<usize, String> {
    let (current, registry, library) = {
        let document = document.borrow();
        let current = document
            .session
            .document_id()
            .cloned()
            .ok_or_else(|| "Only My Worlds documents can compare saved Worlds".to_string())?;
        (
            current,
            Arc::clone(&document.registry),
            Arc::clone(&document.library),
        )
    };

    let documents = library.list().map_err(|error| error.to_string())?;
    if documents.len() < 2 {
        return Err("My Worlds needs at least two saved Worlds to compare".into());
    }
    let default_right = default_right_for(&current, &documents);
    let count = documents.len();
    let bounds = Bounds::centered(None, size(px(940.0), px(760.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| {
            cx.new(|_| SavedWorldSetupView {
                registry,
                library,
                documents,
                left: current,
                right: default_right,
                status: None,
            })
        },
    )
    .map_err(|error| error.to_string())?;

    Ok(count)
}

fn default_right_for(
    current: &WorldDocumentId,
    documents: &[WorldDocumentSummary],
) -> Option<WorldDocumentId> {
    let current_pack = documents
        .iter()
        .find(|document| document.id == *current)
        .map(|document| document.pack.clone());

    current_pack
        .as_ref()
        .and_then(|pack| {
            documents
                .iter()
                .find(|candidate| candidate.id != *current && candidate.pack == *pack)
        })
        .or_else(|| documents.iter().find(|candidate| candidate.id != *current))
        .map(|candidate| candidate.id.clone())
}

#[derive(Clone, Copy)]
enum CompareSide {
    Left,
    Right,
}

struct SavedWorldSetupView {
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
    documents: Vec<WorldDocumentSummary>,
    left: WorldDocumentId,
    right: Option<WorldDocumentId>,
    status: Option<String>,
}

impl SavedWorldSetupView {
    fn render_world_column(
        &self,
        label: &str,
        side: CompareSide,
        selected: Option<&WorldDocumentId>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let column_id = match side {
            CompareSide::Left => "saved-compare-left-column",
            CompareSide::Right => "saved-compare-right-column",
        };
        let mut column = div()
            .id(column_id)
            .w(px(420.0))
            .h(px(520.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child(label.to_string()),
            );

        for document in &self.documents {
            let id = document.id.clone();
            let is_selected = selected.is_some_and(|selected| selected == &id);
            let title = self
                .registry
                .descriptor_for(&document.pack)
                .map(|descriptor| descriptor.title.clone())
                .unwrap_or_else(|| document.pack.id.clone());
            let mut card = div()
                .id(SharedString::from(format!(
                    "saved-compare-{}-{id}",
                    match side {
                        CompareSide::Left => "left",
                        CompareSide::Right => "right",
                    }
                )))
                .w_full()
                .cursor_pointer()
                .p_3()
                .rounded_md()
                .border_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(div().text_sm().child(id.to_string()))
                        .child(div().text_xs().text_color(rgb(0x777770)).child(format!(
                            "t={} · {} events",
                            document.world_time, document.event_count
                        ))),
                )
                .child(div().text_xs().text_color(rgb(0x777770)).child(format!(
                    "{title} · {} @ {}",
                    document.pack.id, document.pack.version
                )));
            card = if is_selected {
                card.border_color(rgb(0x6684c4)).bg(rgb(0xf2f6ff))
            } else {
                card.border_color(rgb(0xd8d8d2)).bg(rgb(0xffffff))
            };
            column = column.child(card.on_click(cx.listener(move |this, _, _, cx| {
                match side {
                    CompareSide::Left => this.left = id.clone(),
                    CompareSide::Right => this.right = Some(id.clone()),
                }
                this.status = None;
                cx.notify();
            })));
        }

        column
    }

    fn run_comparison(&self, cx: &mut Context<Self>) -> Result<(String, String), String> {
        let right = self
            .right
            .clone()
            .ok_or_else(|| "Choose a World on the right".to_string())?;
        if self.left == right {
            return Err("Choose two different saved Worlds".into());
        }

        open_saved_comparison(
            self.library.as_ref(),
            self.registry.as_ref(),
            &self.left,
            &right,
            cx,
        )
    }
}

pub(super) fn open_saved_comparison<T: 'static>(
    library: &WorldLibrary,
    registry: &world_host::WorldRegistry,
    left: &WorldDocumentId,
    right: &WorldDocumentId,
    cx: &mut Context<T>,
) -> Result<(String, String), String> {
    if left == right {
        return Err("Choose two different saved Worlds".into());
    }

    let result =
        compare_saved_worlds(library, registry, left, right).map_err(|error| error.to_string())?;
    let relation = relation_label(&result.relation);
    let context = SavedComparisonContext {
        relation: Some(relation),
        left_provenance: branch_label(result.left.branch.as_ref()),
        right_provenance: branch_label(result.right.branch.as_ref()),
    };
    let left_label = result.left.document.to_string();
    let right_label = result.right.document.to_string();
    let status_left = left_label.clone();
    let status_right = right_label.clone();
    let bounds = Bounds::centered(None, size(px(1240.0), px(980.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| {
            cx.new(|_| {
                StrategyComparisonView::saved_with_context(
                    result.left.snapshot,
                    result.right.snapshot,
                    result.comparison,
                    left_label,
                    right_label,
                    context,
                )
            })
        },
    )
    .map_err(|error| error.to_string())?;

    Ok((status_left, status_right))
}

impl gpui::Render for SavedWorldSetupView {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title("Compare Saved Worlds — World Machine");

        let mut body = div()
            .size_full()
            .p_5()
            .bg(rgb(0xf7f7f3))
            .text_color(rgb(0x202020))
            .flex()
            .flex_col()
            .gap_4()
            .child(div().text_xl().child("Compare Saved Worlds"))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("Choose any two Worlds from My Worlds. Comparison reads their current durable state, never advances either World, and requires the same Pack version."),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(self.render_world_column(
                        "World A",
                        CompareSide::Left,
                        Some(&self.left),
                        cx,
                    ))
                    .child(self.render_world_column(
                        "World B",
                        CompareSide::Right,
                        self.right.as_ref(),
                        cx,
                    )),
            );

        if self.right.as_ref().is_some_and(|right| right == &self.left) {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9b5a4f))
                    .child("Choose two different saved Worlds."),
            );
        }
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

        body.child(
            div()
                .id("run-saved-world-comparison")
                .cursor_pointer()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(0x6684c4))
                .bg(rgb(0xeaf0ff))
                .text_sm()
                .child("Compare current saved state")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.status = Some(match this.run_comparison(cx) {
                        Ok((left, right)) => format!("Opened comparison · {left} ↔ {right}"),
                        Err(error) => format!("Could not compare: {error}"),
                    });
                    cx.notify();
                })),
        )
    }
}

fn relation_label(relation: &SavedWorldRelation) -> String {
    match relation {
        SavedWorldRelation::Same => "Same saved World".into(),
        SavedWorldRelation::AncestorDescendant {
            ancestor,
            descendant,
        } => format!("Ancestor → descendant · {ancestor} → {descendant}"),
        SavedWorldRelation::Siblings { parent } => {
            format!("Sibling branches · parent {parent}")
        }
        SavedWorldRelation::Related { common_ancestor } => {
            format!("Related branches · common ancestor {common_ancestor}")
        }
        SavedWorldRelation::UnresolvedAncestry { left, right } => format!(
            "Unresolved ancestry · left {} · right {}",
            optional_document(left),
            optional_document(right)
        ),
        SavedWorldRelation::Unrelated => "Unrelated saved Worlds".into(),
        SavedWorldRelation::Unavailable(reason) => format!("Relation unavailable · {reason}"),
    }
}

fn optional_document(document: &Option<WorldDocumentId>) -> String {
    document
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".into())
}

fn branch_label(branch: Option<&WorldBranchCause>) -> Option<String> {
    branch.map(|branch| match branch {
        WorldBranchCause::Strategy {
            choice_title,
            horizon,
            ..
        } => format!("Strategy · {choice_title} · {horizon} periods"),
        WorldBranchCause::Fork { label } => match label {
            Some(label) => format!("Fork · {label}"),
            None => "Fork".into(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_persistence::WorldPackRef;

    fn summary(id: &str, pack: WorldPackRef) -> WorldDocumentSummary {
        WorldDocumentSummary {
            id: WorldDocumentId::new(id).unwrap(),
            pack,
            display_title: None,
            world_time: 10,
            event_count: 2,
        }
    }

    #[test]
    fn default_right_prefers_same_exact_pack_version() {
        let pack = WorldPackRef::new("pack-a", "1");
        let newer = WorldPackRef::new("pack-a", "2");
        let current = WorldDocumentId::new("current").unwrap();
        let documents = vec![
            summary("current", pack.clone()),
            summary("other-version", newer),
            summary("compatible", pack),
        ];

        assert_eq!(
            default_right_for(&current, &documents).unwrap().as_str(),
            "compatible"
        );
    }

    #[test]
    fn default_right_falls_back_to_any_other_world() {
        let current = WorldDocumentId::new("current").unwrap();
        let documents = vec![
            summary("current", WorldPackRef::new("pack-a", "1")),
            summary("other", WorldPackRef::new("pack-b", "1")),
        ];

        assert_eq!(
            default_right_for(&current, &documents).unwrap().as_str(),
            "other"
        );
    }

    #[test]
    fn labels_saved_world_relation_and_branch_provenance() {
        let parent = WorldDocumentId::new("source").unwrap();
        let child = WorldDocumentId::new("future").unwrap();
        assert_eq!(
            relation_label(&SavedWorldRelation::AncestorDescendant {
                ancestor: parent,
                descendant: child,
            }),
            "Ancestor → descendant · source → future"
        );
        assert_eq!(
            branch_label(Some(&WorldBranchCause::Strategy {
                choice_id: "choice".into(),
                choice_title: "Choose future".into(),
                horizon: 20,
            })),
            Some("Strategy · Choose future · 20 periods".into())
        );
    }
}
