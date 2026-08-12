use crate::{SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, IntoElement, SharedString, Styled,
    WindowBounds, WindowOptions,
};
use std::sync::Arc;
use world_document::WorldBranchCause;
use world_library::{WorldDocumentId, WorldDocumentSummary, WorldLibrary};
use world_lineage_compare::{compare_saved_worlds, SavedWorldRelation};
use world_persistence::WorldPackRef;
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
        .child("Compare saved World…")
        .on_click(cx.listener(move |this, _, _, cx| {
            this.status = Some(match open_setup(&document, cx) {
                Ok(count) => format!("Opened saved World comparison · {count} candidate(s)"),
                Err(error) => format!("Could not compare saved World: {error}"),
            });
            cx.notify();
        }))
}

fn open_setup(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<usize, String> {
    let (current, pack, registry, library) = {
        let document = document.borrow();
        let current = document
            .session
            .document_id()
            .cloned()
            .ok_or_else(|| "Only My Worlds documents can compare saved Worlds".to_string())?;
        (
            current,
            document.session.pack(),
            Arc::clone(&document.registry),
            Arc::clone(&document.library),
        )
    };
    let candidates =
        comparison_candidates(&current, &pack, library.list().map_err(|e| e.to_string())?);
    if candidates.is_empty() {
        return Err("No other saved Worlds use this Pack".into());
    }
    let count = candidates.len();
    let bounds = Bounds::centered(None, size(px(760.0), px(660.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| {
            cx.new(|_| SavedWorldSetupView {
                current,
                candidates,
                selected: 0,
                registry,
                library,
                status: None,
            })
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(count)
}

struct SavedWorldSetupView {
    current: WorldDocumentId,
    candidates: Vec<WorldDocumentSummary>,
    selected: usize,
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
    status: Option<String>,
}

impl SavedWorldSetupView {
    fn run_comparison(&self, cx: &mut Context<Self>) -> Result<String, String> {
        let candidate = self
            .candidates
            .get(self.selected)
            .ok_or_else(|| "Choose a saved World to compare".to_string())?;
        let result = compare_saved_worlds(
            self.library.as_ref(),
            self.registry.as_ref(),
            &self.current,
            &candidate.id,
        )
        .map_err(|error| error.to_string())?;
        let relation = relation_label(&result.relation);
        let context = SavedComparisonContext {
            relation: Some(relation.clone()),
            left_provenance: branch_label(result.left.branch.as_ref()),
            right_provenance: branch_label(result.right.branch.as_ref()),
        };
        let left_label = result.left.document.to_string();
        let right_label = result.right.document.to_string();
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
        Ok(relation)
    }
}

impl gpui::Render for SavedWorldSetupView {
    fn render(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title("Compare Saved Worlds — World Machine");
        let mut candidates = div().flex().flex_col().gap_2();
        for (index, candidate) in self.candidates.iter().enumerate() {
            let selected = index == self.selected;
            let mut card = div()
                .id(SharedString::from(format!("saved-world-candidate-{index}")))
                .cursor_pointer()
                .p_3()
                .rounded_md()
                .border_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().child(candidate.id.to_string()))
                .child(div().text_xs().text_color(rgb(0x777777)).child(format!(
                    "World time {} · {} events",
                    candidate.world_time, candidate.event_count
                )));
            card = if selected {
                card.border_color(rgb(0x6684c4)).bg(rgb(0xf2f6ff))
            } else {
                card.border_color(rgb(0xd8d8d2)).bg(rgb(0xffffff))
            };
            candidates = candidates.child(card.on_click(cx.listener(move |this, _, _, cx| {
                this.selected = index;
                this.status = None;
                cx.notify();
            })));
        }

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
                    .child(format!("Current World · {}", self.current)),
            )
            .child(candidates);

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
                .child("Compare current durable state")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.status = Some(match this.run_comparison(cx) {
                        Ok(relation) => format!("Opened comparison · {relation}"),
                        Err(error) => format!("Could not compare: {error}"),
                    });
                    cx.notify();
                })),
        )
    }
}

fn comparison_candidates(
    current: &WorldDocumentId,
    pack: &WorldPackRef,
    documents: Vec<WorldDocumentSummary>,
) -> Vec<WorldDocumentSummary> {
    documents
        .into_iter()
        .filter(|document| document.id != *current && document.pack == *pack)
        .collect()
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

    fn summary(id: &str, pack: WorldPackRef) -> WorldDocumentSummary {
        WorldDocumentSummary {
            id: WorldDocumentId::new(id).unwrap(),
            pack,
            world_time: 10,
            event_count: 2,
        }
    }

    #[test]
    fn candidates_exclude_current_and_other_packs() {
        let pack = WorldPackRef::new("pack-a", "1");
        let other_pack = WorldPackRef::new("pack-b", "1");
        let current = WorldDocumentId::new("current").unwrap();
        let candidates = comparison_candidates(
            &current,
            &pack,
            vec![
                summary("current", pack.clone()),
                summary("sibling", pack.clone()),
                summary("other", other_pack),
            ],
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id.as_str(), "sibling");
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
