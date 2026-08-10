use gpui::{
    div, prelude::*, px, rgb, AppContext, Context, Div, IntoElement, Render, SharedString, Styled,
    Window,
};
use world_projection::{
    CanvasItemKind, CollectionItem, InspectorProjection, ProjectionSnapshot, SelectionId,
    TimelineItem,
};

pub struct ProjectionView {
    snapshot: ProjectionSnapshot,
    selected: Option<SelectionId>,
}

impl ProjectionView {
    pub fn new(snapshot: ProjectionSnapshot) -> Self {
        let selected = snapshot
            .timeline
            .items
            .first()
            .map(|item| item.id)
            .or_else(|| snapshot.collection.items.first().map(|item| item.id));
        Self { snapshot, selected }
    }

    fn select(&mut self, selection: SelectionId, cx: &mut Context<Self>) {
        self.selected = Some(selection);
        cx.notify();
    }

    fn render_collection(&self, cx: &mut Context<Self>) -> Div {
        let mut body = div().flex().flex_col().gap_2();
        for item in &self.snapshot.collection.items {
            body = body.child(self.collection_item(item, cx));
        }

        div()
            .w(px(220.0))
            .h_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .border_r_1()
            .border_color(rgb(0xdadada))
            .child(
                div()
                    .text_lg()
                    .child(self.snapshot.collection.title.clone()),
            )
            .child(body)
    }

    fn collection_item(&self, item: &CollectionItem, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = item.id;
        let selected = self.selected == Some(selection);
        div()
            .id(SharedString::from(format!(
                "collection-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .cursor_pointer()
            .bg(if selected {
                rgb(0xe7eefc)
            } else {
                rgb(0xf6f6f6)
            })
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(item.subtitle.clone()),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn render_timeline(&self, cx: &mut Context<Self>) -> Div {
        let mut body = div().flex().flex_col().gap_2();
        for item in self.snapshot.timeline.items.iter().take(12) {
            body = body.child(self.timeline_item(item, cx));
        }

        div()
            .w(px(300.0))
            .h_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .border_l_1()
            .border_color(rgb(0xdadada))
            .child(div().text_lg().child("Timeline"))
            .child(body)
    }

    fn timeline_item(&self, item: &TimelineItem, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = item.id;
        let selected = self.selected == Some(selection);
        div()
            .id(SharedString::from(format!(
                "timeline-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .cursor_pointer()
            .bg(if selected {
                rgb(0xe7eefc)
            } else {
                rgb(0xf7f7f7)
            })
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(format!("t={}", item.world_time)),
            )
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(item.subtitle.clone()),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn render_canvas(&self, cx: &mut Context<Self>) -> Div {
        let mut canvas = div()
            .relative()
            .h(px(380.0))
            .w_full()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd8d8d8))
            .bg(rgb(0xf1f3ef));

        for item in &self.snapshot.canvas.items {
            let selection = item.id;
            let selected = self.selected == Some(selection);
            let color = match item.kind {
                CanvasItemKind::Place => rgb(0xdde5d8),
                CanvasItemKind::Actor => rgb(0xf4e4c8),
                CanvasItemKind::Object => rgb(0xe2e2e2),
            };
            canvas = canvas.child(
                div()
                    .id(SharedString::from(format!(
                        "canvas-{}",
                        selection.stable_key()
                    )))
                    .absolute()
                    .left(px(18.0 + item.x * 500.0))
                    .top(px(18.0 + item.y * 300.0))
                    .w(px(135.0))
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(if selected {
                        rgb(0x4e6fb3)
                    } else {
                        rgb(0xbfc5bd)
                    })
                    .bg(color)
                    .cursor_pointer()
                    .child(div().text_sm().child(item.label.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x666666))
                            .child(item.detail.clone()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx))),
            );
        }

        canvas
    }

    fn render_inspector(&self) -> Div {
        let inspector = self
            .selected
            .and_then(|selection| self.snapshot.inspector(selection));

        let Some(inspector) = inspector else {
            return div()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xdadada))
                .child("Select a resident, place, or event to inspect it.");
        };

        inspector_panel(inspector)
    }
}

impl Render for ProjectionView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0xfcfcfa))
            .text_color(rgb(0x202020))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(58.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .border_b_1()
                    .border_color(rgb(0xdadada))
                    .child(div().text_xl().child(self.snapshot.title.clone()))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child(format!("World time {}", self.snapshot.world_time)),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()
                    .child(self.render_collection(cx))
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_3()
                            .child(div().text_lg().child("World"))
                            .child(self.render_canvas(cx))
                            .child(self.render_inspector()),
                    )
                    .child(self.render_timeline(cx)),
            )
    }
}

fn inspector_panel(inspector: &InspectorProjection) -> Div {
    let mut body = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_lg().child(inspector.title.clone()))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x666666))
                .child(inspector.subtitle.clone()),
        );

    for section in &inspector.sections {
        let mut rows = div().flex().flex_col().gap_1();
        for row in &section.rows {
            rows = rows.child(
                div()
                    .flex()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(row.label.clone()),
                    )
                    .child(div().text_sm().child(row.value.clone())),
            );
        }
        body = body
            .child(div().text_sm().child(section.title.clone()))
            .child(rows);
    }

    div()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xdadada))
        .bg(rgb(0xffffff))
        .child(body)
}
