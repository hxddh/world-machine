use gpui::{div, prelude::*, px, rgb, Context, Div, IntoElement, Render, Styled, Window};
use world_document::WorldBranchCause;
use world_library::WorldDocumentId;
use world_lineage::{LineageIndex, LineageNode};

pub struct LineageTreeView {
    index: LineageIndex,
}

impl LineageTreeView {
    pub fn new(index: LineageIndex) -> Self {
        Self { index }
    }

    fn render_tree(&self) -> Div {
        let mut body = div().flex().flex_col().gap_3();
        for root in self.index.roots() {
            body = body.child(self.render_node(root, 0));
        }
        if self.index.roots().is_empty() {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x777777))
                    .child("No root Worlds"),
            );
        }
        body
    }

    fn render_detached(&self) -> Div {
        let mut body = div().flex().flex_col().gap_2();
        for id in self.index.detached() {
            if let Some(node) = self.index.node(id) {
                body = body.child(self.render_detached_node(node));
            }
        }
        body
    }

    fn render_node(&self, id: &WorldDocumentId, depth: usize) -> Div {
        let Some(node) = self.index.node(id) else {
            return div();
        };
        let mut row = div()
            .ml(px((depth * 28) as f32))
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd8d8d2))
            .bg(rgb(0xffffff))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_3()
                    .child(div().text_sm().child(node.id.to_string()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(format!("{} child branch(es)", node.children.len())),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x666666))
                    .child(format!(
                        "{} @ {} · World time {} · {} events",
                        node.pack.id, node.pack.version, node.world_time, node.event_count
                    )),
            );
        if let Some(branch) = &node.branch {
            row = row.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x4e6fb3))
                    .child(branch_label(branch)),
            );
        }

        let mut group = div().flex().flex_col().gap_2().child(row);
        for child in &node.children {
            group = group.child(self.render_node(child, depth + 1));
        }
        group
    }

    fn render_detached_node(&self, node: &LineageNode) -> Div {
        let parent = node
            .parent
            .as_ref()
            .and_then(|parent| parent.document.clone())
            .unwrap_or_else(|| "unknown parent".into());
        let mut card = div()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe0c9a7))
            .bg(rgb(0xfffbf2))
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(node.id.to_string()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8a6840))
                    .child(format!("Detached lineage · parent {parent}")),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(format!("World time {} · {} events", node.world_time, node.event_count)),
            );
        if let Some(branch) = &node.branch {
            card = card.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x8a6840))
                    .child(branch_label(branch)),
            );
        }
        card
    }
}

impl Render for LineageTreeView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div()
            .size_full()
            .p_5()
            .bg(rgb(0xf7f7f3))
            .text_color(rgb(0x202020))
            .flex()
            .flex_col()
            .gap_4()
            .child(div().text_xl().child("World Lineage"))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("Saved Worlds grouped by durable parent and branch provenance."),
            )
            .child(self.render_tree());

        if !self.index.detached().is_empty() {
            body = body
                .child(div().text_sm().child("Detached Worlds"))
                .child(self.render_detached());
        }
        body
    }
}

fn branch_label(branch: &WorldBranchCause) -> String {
    match branch {
        WorldBranchCause::Strategy {
            choice_title,
            horizon,
            ..
        } => format!("Strategy · {choice_title} · {horizon} periods"),
        WorldBranchCause::Fork { label } => match label {
            Some(label) => format!("Fork · {label}"),
            None => "Fork".into(),
        },
    }
}
