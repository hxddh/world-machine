use crate::ProjectionController;
use gpui::{
    div, prelude::*, px, rgb, Context, Div, IntoElement, Render, SharedString, Styled, Window,
};
use world_projection::{
    BriefingItem, CanvasItemKind, CollectionItem, InspectorProjection, ProjectionCommand,
    ProjectionIntent, ProjectionSnapshot, SelectionId, TimelineItem, WhyNode,
};

pub struct ProjectionView {
    snapshot: ProjectionSnapshot,
    selected: Option<SelectionId>,
    controller: Option<Box<dyn ProjectionController>>,
    status: Option<String>,
}

impl ProjectionView {
    pub fn new(snapshot: ProjectionSnapshot) -> Self {
        let selected = default_selection(&snapshot);
        Self {
            snapshot,
            selected,
            controller: None,
            status: None,
        }
    }

    pub fn controlled<C>(controller: C) -> Self
    where
        C: ProjectionController + 'static,
    {
        let snapshot = controller.snapshot();
        let mut view = Self::new(snapshot);
        view.controller = Some(Box::new(controller));
        view
    }

    fn select(&mut self, selection: SelectionId, cx: &mut Context<Self>) {
        self.selected = Some(selection);
        self.status = None;
        cx.notify();
    }

    fn fork_before_selected(&mut self, cx: &mut Context<Self>) {
        let Some(SelectionId::Event(event)) = self.selected else {
            return;
        };
        let Some(controller) = self.controller.as_mut() else {
            return;
        };

        match controller.handle(ProjectionIntent::ForkBeforeEvent(event)) {
            Ok(snapshot) => {
                let previous = self.selected;
                self.snapshot = snapshot;
                self.selected = selection_for_snapshot(previous, &self.snapshot);
                self.status = Some(format!("Forked before Event #{event}"));
            }
            Err(error) => {
                self.status = Some(format!("Fork failed: {error}"));
            }
        }
        cx.notify();
    }

    fn invoke_command(&mut self, command_id: String, cx: &mut Context<Self>) {
        let title = self
            .snapshot
            .command(&command_id)
            .map(|command| command.title.clone())
            .unwrap_or_else(|| command_id.clone());
        let Some(controller) = self.controller.as_mut() else {
            return;
        };

        match controller.handle(ProjectionIntent::InvokeCommand(command_id)) {
            Ok(snapshot) => {
                let previous = self.selected;
                self.snapshot = snapshot;
                self.selected = selection_for_snapshot(previous, &self.snapshot);
                self.status = Some(format!("{title} completed"));
            }
            Err(error) => {
                self.status = Some(format!("Command failed: {error}"));
            }
        }
        cx.notify();
    }

    fn render_collection(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div().flex().flex_col().gap_2();
        for item in &self.snapshot.collection.items {
            body = body.child(self.collection_item(item, cx));
        }

        div()
            .id("projection-collection-scroll")
            .w(px(220.0))
            .h_full()
            .overflow_y_scroll()
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

    fn render_timeline(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div().flex().flex_col().gap_2();
        for item in self.snapshot.timeline.items.iter().take(12) {
            body = body.child(self.timeline_item(item, cx));
        }

        div()
            .id("projection-timeline-scroll")
            .w(px(300.0))
            .h_full()
            .overflow_y_scroll()
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

    fn render_briefing(&self, cx: &mut Context<Self>) -> Option<Div> {
        let briefing = self.snapshot.briefing.as_ref()?;
        let mut items = div().flex().gap_2();
        for item in &briefing.items {
            items = items.child(self.briefing_item(item, cx));
        }

        Some(
            div()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xd8d3c4))
                .bg(rgb(0xfffbef))
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x7a6f53))
                        .child(briefing.eyebrow.clone()),
                )
                .child(div().text_lg().child(briefing.title.clone()))
                .child(items),
        )
    }

    fn briefing_item(&self, item: &BriefingItem, cx: &mut Context<Self>) -> impl IntoElement {
        let id = item
            .selection
            .map(|selection| format!("briefing-{}", selection.stable_key()))
            .unwrap_or_else(|| format!("briefing-static-{}", item.title));
        let mut card = div()
            .id(SharedString::from(id))
            .p_2()
            .rounded_md()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xe8e1cf))
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(item.detail.clone()),
            );

        if let Some(selection) = item.selection {
            card = card
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)));
        }
        card
    }

    fn render_commands(&self, cx: &mut Context<Self>) -> Option<Div> {
        if self.controller.is_none() || self.snapshot.commands.is_empty() {
            return None;
        }

        let panel_title = command_panel_title(self.snapshot.commands.len());
        let mut commands = div().flex().flex_col().gap_2();
        for command in &self.snapshot.commands {
            commands = commands.child(self.command_item(command, cx));
        }

        Some(
            div()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xaec5a7))
                .bg(rgb(0xf1f8ee))
                .flex()
                .flex_col()
                .gap_2()
                .child(div().text_xs().text_color(rgb(0x60755a)).child("NEXT"))
                .child(div().text_lg().child(panel_title))
                .child(commands),
        )
    }

    fn command_item(
        &self,
        command: &ProjectionCommand,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let command_id = command.id.clone();
        div()
            .id(SharedString::from(format!("command-{}", command.id)))
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xcbd8c3))
            .bg(rgb(0xffffff))
            .cursor_pointer()
            .child(div().text_sm().child(command.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x66705f))
                    .child(command.detail.clone()),
            )
            .on_click(
                cx.listener(move |this, _, _, cx| this.invoke_command(command_id.clone(), cx)),
            )
    }

    fn render_canvas(&self, cx: &mut Context<Self>) -> Div {
        let mut canvas = div()
            .relative()
            .h(px(330.0))
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
                    .top(px(12.0 + item.y * 260.0))
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

    fn render_inspector(&self) -> Option<Div> {
        let inspector = self
            .selected
            .and_then(|selection| self.snapshot.inspector(selection))?;
        Some(inspector_panel(inspector))
    }

    fn render_why(&self, cx: &mut Context<Self>) -> Option<Div> {
        let SelectionId::Event(event) = self.selected? else {
            return None;
        };
        let why = self.snapshot.why(event)?;

        let mut nodes = div().flex().flex_col().gap_1();
        for node in why.nodes.iter().take(10) {
            nodes = nodes.child(self.why_node(node, cx));
        }

        let mut panel = div()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd7dce8))
            .bg(rgb(0xf7f9fe))
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_lg().child("Why?"))
            .child(nodes);

        if self.controller.is_some() && self.snapshot.capabilities.fork {
            panel = panel.child(
                div()
                    .id("fork-before-event")
                    .p_2()
                    .rounded_md()
                    .bg(rgb(0x263b6a))
                    .text_color(rgb(0xffffff))
                    .cursor_pointer()
                    .child("Fork before this event")
                    .on_click(cx.listener(|this, _, _, cx| this.fork_before_selected(cx))),
            );
        }
        Some(panel)
    }

    fn why_node(&self, node: &WhyNode, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = SelectionId::Event(node.event);
        let prefix = if node.depth == 0 {
            "Selected".to_string()
        } else {
            format!("{}Cause", "↳ ".repeat(node.depth))
        };
        div()
            .id(SharedString::from(format!("why-event-{}", node.event)))
            .p_2()
            .rounded_md()
            .bg(rgb(0xffffff))
            .cursor_pointer()
            .child(div().text_xs().text_color(rgb(0x65708a)).child(prefix))
            .child(div().text_sm().child(node.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(node.subtitle.clone()),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }
}

impl Render for ProjectionView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut center = div()
            .id("projection-center-scroll")
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_3();

        if let Some(briefing) = self.render_briefing(cx) {
            center = center.child(briefing);
        }
        if let Some(commands) = self.render_commands(cx) {
            center = center.child(commands);
        }
        if has_exploration(&self.snapshot, self.selected) {
            center = center.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("Explore the world"),
            );
            if !self.snapshot.canvas.items.is_empty() {
                center = center.child(self.render_canvas(cx));
            }
            if let Some(inspector) = self.render_inspector() {
                center = center.child(inspector);
            }
            if let Some(why) = self.render_why(cx) {
                center = center.child(why);
            }
        }

        let mut workspace = div().flex_1().w_full().flex();
        if has_collection_panel(&self.snapshot) {
            workspace = workspace.child(self.render_collection(cx));
        }
        workspace = workspace.child(center);
        if has_timeline_panel(&self.snapshot) {
            workspace = workspace.child(self.render_timeline(cx));
        }

        let mut header_right = div().flex().gap_3().child(
            div()
                .text_sm()
                .text_color(rgb(0x666666))
                .child(format!("World time {}", self.snapshot.world_time)),
        );
        if let Some(status) = &self.status {
            header_right = header_right.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x4e6fb3))
                    .child(status.clone()),
            );
        }

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
                    .child(header_right),
            )
            .child(workspace)
    }
}

fn has_collection_panel(snapshot: &ProjectionSnapshot) -> bool {
    !snapshot.collection.items.is_empty()
}

fn has_timeline_panel(snapshot: &ProjectionSnapshot) -> bool {
    !snapshot.timeline.items.is_empty()
}

fn has_exploration(snapshot: &ProjectionSnapshot, selected: Option<SelectionId>) -> bool {
    !snapshot.canvas.items.is_empty()
        || selected
            .and_then(|selection| snapshot.inspector(selection))
            .is_some()
        || matches!(selected, Some(SelectionId::Event(event)) if snapshot.why(event).is_some())
}

fn command_panel_title(command_count: usize) -> &'static str {
    if command_count == 1 {
        "Continue"
    } else {
        "Choose what happens next"
    }
}

fn default_selection(snapshot: &ProjectionSnapshot) -> Option<SelectionId> {
    snapshot
        .collection
        .items
        .first()
        .map(|item| item.id)
        .or_else(|| snapshot.timeline.items.first().map(|item| item.id))
}

fn selection_for_snapshot(
    previous: Option<SelectionId>,
    snapshot: &ProjectionSnapshot,
) -> Option<SelectionId> {
    previous
        .filter(|selection| snapshot.inspector(*selection).is_some())
        .or_else(|| default_selection(snapshot))
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

#[cfg(test)]
mod focus_hierarchy_tests {
    use super::{
        command_panel_title, default_selection, has_collection_panel, has_exploration,
        has_timeline_panel, selection_for_snapshot,
    };
    use world_projection::{
        CollectionItem, InspectorProjection, ProjectionSnapshot, SelectionId, TimelineItem,
    };

    fn entity_selection() -> SelectionId {
        SelectionId::Entity(Default::default())
    }

    fn event_selection() -> SelectionId {
        SelectionId::Event(Default::default())
    }

    fn inspector(selection: SelectionId) -> InspectorProjection {
        InspectorProjection {
            selection,
            title: "Selection".into(),
            subtitle: String::new(),
            sections: Vec::new(),
        }
    }

    fn snapshot_with_entity_and_event() -> ProjectionSnapshot {
        let entity = entity_selection();
        let event = event_selection();
        let mut snapshot = ProjectionSnapshot::default();
        snapshot.collection.items.push(CollectionItem {
            id: entity,
            title: "World".into(),
            subtitle: String::new(),
        });
        snapshot.timeline.items.push(TimelineItem {
            id: event,
            world_time: 1,
            title: "Changed".into(),
            subtitle: String::new(),
            caused_by: Vec::new(),
        });
        snapshot.inspectors.insert(entity, inspector(entity));
        snapshot.inspectors.insert(event, inspector(event));
        snapshot
    }

    #[test]
    fn empty_world_keeps_focus_without_empty_exploration_chrome() {
        let snapshot = ProjectionSnapshot::default();
        assert!(!has_collection_panel(&snapshot));
        assert!(!has_timeline_panel(&snapshot));
        assert!(!has_exploration(&snapshot, None));
    }

    #[test]
    fn semantic_content_restores_navigation_and_exploration() {
        let snapshot = snapshot_with_entity_and_event();
        assert!(has_collection_panel(&snapshot));
        assert!(has_timeline_panel(&snapshot));
        assert!(has_exploration(&snapshot, Some(entity_selection())));
    }

    #[test]
    fn command_panel_distinguishes_continuation_from_choice() {
        assert_eq!(command_panel_title(1), "Continue");
        assert_eq!(command_panel_title(2), "Choose what happens next");
        assert_eq!(command_panel_title(5), "Choose what happens next");
    }

    #[test]
    fn default_selection_prefers_semantic_collection_over_latest_event() {
        let snapshot = snapshot_with_entity_and_event();
        assert_eq!(default_selection(&snapshot), Some(entity_selection()));
    }

    #[test]
    fn valid_entity_selection_persists_across_snapshots() {
        let snapshot = snapshot_with_entity_and_event();
        assert_eq!(
            selection_for_snapshot(Some(entity_selection()), &snapshot),
            Some(entity_selection())
        );
    }

    #[test]
    fn explicitly_selected_event_persists_while_it_still_exists() {
        let snapshot = snapshot_with_entity_and_event();
        assert_eq!(
            selection_for_snapshot(Some(event_selection()), &snapshot),
            Some(event_selection())
        );
    }

    #[test]
    fn invalid_previous_selection_falls_back_to_semantic_default() {
        let entity = entity_selection();
        let mut snapshot = ProjectionSnapshot::default();
        snapshot.collection.items.push(CollectionItem {
            id: entity,
            title: "World".into(),
            subtitle: String::new(),
        });
        snapshot.inspectors.insert(entity, inspector(entity));
        assert_eq!(
            selection_for_snapshot(Some(event_selection()), &snapshot),
            Some(entity)
        );
    }

    #[test]
    fn timeline_event_is_used_when_collection_is_empty() {
        let event = event_selection();
        let mut snapshot = ProjectionSnapshot::default();
        snapshot.timeline.items.push(TimelineItem {
            id: event,
            world_time: 1,
            title: "Changed".into(),
            subtitle: String::new(),
            caused_by: Vec::new(),
        });
        snapshot.inspectors.insert(event, inspector(event));
        assert_eq!(default_selection(&snapshot), Some(event));
    }
}
