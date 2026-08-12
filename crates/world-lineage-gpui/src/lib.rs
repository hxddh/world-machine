use gpui::{
    div, prelude::*, px, rgb, Context, Div, IntoElement, Render, SharedString, Styled, Window,
};
use world_document::WorldBranchCause;
use world_lineage::{LineageIndex, LineageNode};

pub trait LineageController {
    fn open_document(
        &mut self,
        document: &str,
        cx: &mut Context<LineageExplorerView>,
    ) -> Result<(), String>;

    /// Return a non-fatal notice produced by the last successful open, if any.
    /// Controllers can use this for degraded initialization such as observer
    /// catch-up failures without teaching the generic lineage UI about Host or
    /// Library details.
    fn take_open_notice(&mut self) -> Option<String> {
        None
    }

    fn can_compare(&self) -> bool {
        false
    }

    fn compare_documents(
        &mut self,
        _left: &str,
        _right: &str,
        _cx: &mut Context<LineageExplorerView>,
    ) -> Result<(), String> {
        Err("This lineage view cannot compare saved Worlds".into())
    }
}

pub struct LineageExplorerView {
    index: LineageIndex,
    selected: Option<String>,
    compare_from: Option<String>,
    controller: Option<Box<dyn LineageController>>,
    status: Option<String>,
}

impl LineageExplorerView {
    pub fn new(index: LineageIndex) -> Self {
        Self::with_controller(index, None, None)
    }

    pub fn controlled<C>(index: LineageIndex, controller: C) -> Self
    where
        C: LineageController + 'static,
    {
        Self::with_controller(index, None, Some(Box::new(controller)))
    }

    pub fn controlled_selected<C>(
        index: LineageIndex,
        selected: impl Into<String>,
        controller: C,
    ) -> Self
    where
        C: LineageController + 'static,
    {
        Self::with_controller(index, Some(selected.into()), Some(Box::new(controller)))
    }

    fn with_controller(
        index: LineageIndex,
        selected: Option<String>,
        controller: Option<Box<dyn LineageController>>,
    ) -> Self {
        let selected = selected
            .filter(|selected| {
                index
                    .nodes()
                    .values()
                    .any(|node| node.id.as_str() == selected)
            })
            .or_else(|| index.roots().first().map(ToString::to_string))
            .or_else(|| index.nodes().keys().next().map(ToString::to_string));
        Self {
            index,
            selected,
            compare_from: None,
            controller,
            status: None,
        }
    }

    fn node_by_label(&self, label: &str) -> Option<&LineageNode> {
        self.index
            .nodes()
            .values()
            .find(|node| node.id.as_str() == label)
    }

    fn open_selected(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<(String, Option<String>), String> {
        let document = self
            .selected
            .clone()
            .ok_or_else(|| "Select a World before opening it".to_string())?;
        let controller = self
            .controller
            .as_mut()
            .ok_or_else(|| "This lineage view cannot open Worlds".to_string())?;
        controller.open_document(&document, cx)?;
        let notice = controller.take_open_notice();
        Ok((document, notice))
    }

    fn mark_comparison_source(&mut self) -> Result<String, String> {
        let document = self
            .selected
            .clone()
            .ok_or_else(|| "Select a World before starting comparison".to_string())?;
        self.compare_from = Some(document.clone());
        Ok(document)
    }

    fn compare_selected(&mut self, cx: &mut Context<Self>) -> Result<(String, String), String> {
        let left = self
            .compare_from
            .clone()
            .ok_or_else(|| "Choose a comparison source first".to_string())?;
        let right = self
            .selected
            .clone()
            .ok_or_else(|| "Select another World before comparing".to_string())?;
        if left == right {
            return Err("Choose a different World for the right side".into());
        }
        let controller = self
            .controller
            .as_mut()
            .ok_or_else(|| "This lineage view cannot compare saved Worlds".to_string())?;
        if !controller.can_compare() {
            return Err("This lineage view cannot compare saved Worlds".into());
        }
        controller.compare_documents(&left, &right, cx)?;
        Ok((left, right))
    }

    fn render_tree_node(&self, label: String, depth: usize, cx: &mut Context<Self>) -> Div {
        let Some(node) = self.node_by_label(&label) else {
            return div();
        };
        let selected = self.selected.as_deref() == Some(label.as_str());
        let comparison_source = self.compare_from.as_deref() == Some(label.as_str());
        let branch = branch_label(node.branch.as_ref());
        let children = node
            .children
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        let mut card = div()
            .id(SharedString::from(format!("lineage-node-{label}")))
            .ml(px((depth * 18) as f32))
            .p_2()
            .rounded_md()
            .border_1()
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_sm().child(label.clone()))
                    .when(comparison_source, |row| {
                        row.child(
                            div()
                                .px_2()
                                .rounded_md()
                                .bg(rgb(0xe9eefc))
                                .text_xs()
                                .text_color(rgb(0x4c65a7))
                                .child("A"),
                        )
                    }),
            )
            .child(div().text_xs().text_color(rgb(0x777777)).child(format!(
                "t={} · {} events · {branch}",
                node.world_time, node.event_count
            )));
        card = if selected {
            card.border_color(rgb(0x6f7fb8)).bg(rgb(0xf0f4ff))
        } else {
            card.border_color(rgb(0xd8d8d2)).bg(rgb(0xffffff))
        };

        let mut tree = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(card.on_click(cx.listener({
                let label = label.clone();
                move |this, _, _, cx| {
                    this.selected = Some(label.clone());
                    this.status = None;
                    cx.notify();
                }
            })));
        for child in children {
            tree = tree.child(self.render_tree_node(child, depth + 1, cx));
        }
        tree
    }

    fn render_detail(&self, cx: &mut Context<Self>) -> Div {
        let Some(label) = self.selected.as_deref() else {
            return detail_shell().child("Select a World to inspect its lineage.");
        };
        let Some(node) = self.node_by_label(label) else {
            return detail_shell().child("The selected World is no longer in this lineage index.");
        };

        let parent = node
            .parent
            .as_ref()
            .map(|parent| {
                let reference = parent
                    .document
                    .as_deref()
                    .unwrap_or("external or unlabeled");
                let resolved = parent
                    .resolved
                    .as_ref()
                    .map(|id| format!(" → {id}"))
                    .unwrap_or_else(|| " · detached".into());
                format!(
                    "{reference}{resolved} · branch point t={} · {} events",
                    parent.world_time, parent.event_count
                )
            })
            .unwrap_or_else(|| "Root World".into());

        let mut detail = detail_shell()
            .child(div().text_xl().child(node.id.to_string()))
            .child(detail_row(
                "Pack",
                format!("{}@{}", node.pack.id, node.pack.version),
            ))
            .child(detail_row(
                "Current World time",
                node.world_time.to_string(),
            ))
            .child(detail_row("Current events", node.event_count.to_string()))
            .child(detail_row("Branch", branch_label(node.branch.as_ref())))
            .child(detail_row("Parent", parent))
            .child(detail_row("Children", node.children.len().to_string()));

        if let Some(controller) = self.controller.as_ref() {
            if controller.can_compare() {
                detail = detail.child(
                    div()
                        .id("mark-lineage-comparison-source")
                        .cursor_pointer()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0x9aa6cc))
                        .bg(rgb(0xf4f6ff))
                        .text_sm()
                        .child("Mark as comparison A")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.status = Some(match this.mark_comparison_source() {
                                Ok(document) => format!("Comparison A: {document}"),
                                Err(error) => format!("Could not start comparison: {error}"),
                            });
                            cx.notify();
                        })),
                );
                if let Some(left) = &self.compare_from {
                    if left != label {
                        let left = left.clone();
                        detail = detail.child(
                            div()
                                .id("compare-lineage-worlds")
                                .cursor_pointer()
                                .p_2()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(0x7a8dbb))
                                .bg(rgb(0xeef3ff))
                                .text_sm()
                                .child(format!("Compare {left} ↔ {label}"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.status = Some(match this.compare_selected(cx) {
                                        Ok((left, right)) => format!("Comparing {left} ↔ {right}"),
                                        Err(error) => format!("Compare failed: {error}"),
                                    });
                                    cx.notify();
                                })),
                        );
                    }
                }
            }
        }

        if self.controller.is_some() {
            detail = detail.child(
                div()
                    .id("open-lineage-world")
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x6684c4))
                    .bg(rgb(0xf2f6ff))
                    .text_sm()
                    .child("Open World")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.status = Some(match this.open_selected(cx) {
                            Ok((document, Some(notice))) => {
                                format!("Opened {document} · {notice}")
                            }
                            Ok((document, None)) => format!("Opened {document}"),
                            Err(error) => format!("Could not open World: {error}"),
                        });
                        cx.notify();
                    })),
            );
        }
        if let Some(status) = &self.status {
            detail = detail.child(
                div()
                    .p_2()
                    .rounded_md()
                    .bg(rgb(0xeef2ea))
                    .text_sm()
                    .child(status.clone()),
            );
        }
        detail
    }
}

impl Render for LineageExplorerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title("World Lineage — World Machine");

        let mut roots = div().flex().flex_col().gap_2();
        for root in self.index.roots() {
            roots = roots.child(self.render_tree_node(root.to_string(), 0, cx));
        }

        let mut detached = div().flex().flex_col().gap_2();
        for node in self.index.detached() {
            detached = detached.child(self.render_tree_node(node.to_string(), 0, cx));
        }

        let mut tree = div()
            .w(px(520.0))
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .border_r_1()
            .border_color(rgb(0xd9d9d3))
            .child(div().text_sm().text_color(rgb(0x666666)).child(format!(
                "{} Worlds · {} roots · {} detached",
                self.index.nodes().len(),
                self.index.roots().len(),
                self.index.detached().len()
            )))
            .child(div().text_sm().child("Roots"))
            .child(roots);
        if !self.index.detached().is_empty() {
            tree = tree
                .child(div().text_sm().child("Detached lineage"))
                .child(detached);
        }

        div()
            .size_full()
            .bg(rgb(0xf7f7f3))
            .text_color(rgb(0x202020))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(56.0))
                    .px_4()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(rgb(0xd9d9d3))
                    .child(div().text_xl().child("World Lineage")),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()
                    .child(tree)
                    .child(self.render_detail(cx)),
            )
    }
}

fn branch_label(branch: Option<&WorldBranchCause>) -> String {
    match branch {
        None => "Root".into(),
        Some(WorldBranchCause::Strategy {
            choice_title,
            horizon,
            ..
        }) => format!("Strategy: {choice_title} · {horizon} periods"),
        Some(WorldBranchCause::Fork { label }) => label
            .as_ref()
            .map(|label| format!("Fork: {label}"))
            .unwrap_or_else(|| "Fork".into()),
    }
}

fn detail_shell() -> Div {
    div()
        .flex_1()
        .p_5()
        .flex()
        .flex_col()
        .gap_3()
        .bg(rgb(0xffffff))
}

fn detail_row(label: &str, value: String) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x777777))
                .child(label.to_owned()),
        )
        .child(div().text_sm().child(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_document::{WorldLineage, WorldParent};
    use world_library::WorldDocumentId;
    use world_lineage::{build_index, LineageRecord};
    use world_persistence::WorldPackRef;

    fn record(id: &str, parent: Option<&str>) -> LineageRecord {
        LineageRecord {
            id: WorldDocumentId::new(id).unwrap(),
            pack: WorldPackRef::new("world-machine.lineage-ui-test", "1"),
            world_time: 20,
            event_count: 4,
            lineage: parent.map(|parent| WorldLineage {
                parent: WorldParent {
                    document: Some(parent.into()),
                    pack: WorldPackRef::new("world-machine.lineage-ui-test", "1"),
                    world_time: 10,
                    event_count: 2,
                },
                branch: WorldBranchCause::Strategy {
                    choice_id: "test.choice".into(),
                    choice_title: "Choice".into(),
                    horizon: 20,
                },
            }),
        }
    }

    #[test]
    fn chooses_a_root_as_the_initial_selection() {
        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::new(index);
        assert_eq!(view.selected.as_deref(), Some("root"));
    }

    #[test]
    fn controlled_view_has_an_open_capability() {
        struct NoopController;
        impl LineageController for NoopController {
            fn open_document(
                &mut self,
                _document: &str,
                _cx: &mut Context<LineageExplorerView>,
            ) -> Result<(), String> {
                Ok(())
            }
        }

        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::controlled(index, NoopController);
        assert!(view.controller.is_some());
    }

    #[test]
    fn controlled_selected_prefers_the_requested_world() {
        struct NoopController;
        impl LineageController for NoopController {
            fn open_document(
                &mut self,
                _document: &str,
                _cx: &mut Context<LineageExplorerView>,
            ) -> Result<(), String> {
                Ok(())
            }
        }

        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::controlled_selected(index, "future", NoopController);
        assert_eq!(view.selected.as_deref(), Some("future"));
    }

    #[test]
    fn controlled_selected_falls_back_when_the_requested_world_is_missing() {
        struct NoopController;
        impl LineageController for NoopController {
            fn open_document(
                &mut self,
                _document: &str,
                _cx: &mut Context<LineageExplorerView>,
            ) -> Result<(), String> {
                Ok(())
            }
        }

        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::controlled_selected(index, "missing", NoopController);
        assert_eq!(view.selected.as_deref(), Some("root"));
    }

    #[test]
    fn comparison_capability_is_opt_in() {
        struct CompareController;
        impl LineageController for CompareController {
            fn open_document(
                &mut self,
                _document: &str,
                _cx: &mut Context<LineageExplorerView>,
            ) -> Result<(), String> {
                Ok(())
            }

            fn can_compare(&self) -> bool {
                true
            }
        }

        let index = build_index([record("root", None), record("future", Some("root"))]).unwrap();
        let view = LineageExplorerView::controlled(index, CompareController);
        assert!(view.controller.as_ref().unwrap().can_compare());
    }

    #[test]
    fn branch_labels_are_domain_neutral() {
        let branch = WorldBranchCause::Strategy {
            choice_id: "test.choice".into(),
            choice_title: "Choose A".into(),
            horizon: 5,
        };
        assert_eq!(
            branch_label(Some(&branch)),
            "Strategy: Choose A · 5 periods"
        );
    }
}
