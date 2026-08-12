use gpui::{
    div, prelude::*, px, rgb, Context, Div, IntoElement, Render, SharedString, Styled, Window,
};
use world_document::WorldBranchCause;
use world_lineage::{LineageIndex, LineageNode};

pub struct LineageExplorerView {
    index: LineageIndex,
    selected: Option<String>,
}

impl LineageExplorerView {
    pub fn new(index: LineageIndex) -> Self {
        let selected = index
            .roots()
            .first()
            .map(ToString::to_string)
            .or_else(|| index.nodes().keys().next().map(ToString::to_string));
        Self { index, selected }
    }

    fn node_by_label(&self, label: &str) -> Option<&LineageNode> {
        self.index
            .nodes()
            .values()
            .find(|node| node.id.as_str() == label)
    }

    fn render_tree_node(&self, label: String, depth: usize, cx: &mut Context<Self>) -> Div {
        let Some(node) = self.node_by_label(&label) else {
            return div();
        };
        let selected = self.selected.as_deref() == Some(label.as_str());
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
            .child(div().text_sm().child(label.clone()))
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
                    cx.notify();
                }
            })));
        for child in children {
            tree = tree.child(self.render_tree_node(child, depth + 1, cx));
        }
        tree
    }

    fn render_detail(&self) -> Div {
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

        detail_shell()
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
            .child(detail_row("Children", node.children.len().to_string()))
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
                    .child(self.render_detail()),
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
