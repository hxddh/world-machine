use crate::ProjectionController;
use gpui::{
    div, prelude::*, px, rgb, Context, Div, IntoElement, Render, SharedString, Styled, Window,
};
use world_projection::{
    BriefingItem, CanvasItemKind, CollectionItem, InspectorProjection, ProjectionCommand,
    ProjectionIntent, ProjectionSnapshot, SelectionId, TimelineItem, WhyNode,
};

const ENTITY_HISTORY_LIMIT: usize = 6;
const RELATION_HISTORY_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
const EVENT_RELATION_EFFECT_LIMIT: usize = 6;

pub struct ProjectionView {
    snapshot: ProjectionSnapshot,
    selected: Option<SelectionId>,
    controller: Option<Box<dyn ProjectionController>>,
    status: Option<String>,
    status_is_error: bool,
}

impl ProjectionView {
    pub fn new(snapshot: ProjectionSnapshot) -> Self {
        let selected = default_selection(&snapshot);
        Self {
            snapshot,
            selected,
            controller: None,
            status: None,
            status_is_error: false,
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
        self.status_is_error = false;
        cx.notify();
    }

    fn fork_before_selected(&mut self, cx: &mut Context<Self>) {
        let Some(SelectionId::Event(event)) = self.selected else {
            return;
        };
        let event_title = self
            .snapshot
            .timeline
            .items
            .iter()
            .find(|item| item.id == SelectionId::Event(event))
            .map(|item| item.title.clone())
            .unwrap_or_else(|| format!("Event #{event}"));
        let Some(controller) = self.controller.as_mut() else {
            return;
        };

        match controller.handle(ProjectionIntent::ForkBeforeEvent(event)) {
            Ok(snapshot) => {
                let previous = self.selected;
                self.snapshot = snapshot;
                self.selected = selection_for_snapshot(previous, &self.snapshot);
                self.status = Some(format!("Branched before {event_title}"));
                self.status_is_error = false;
            }
            Err(error) => {
                self.status = Some(format!("Couldn't branch here: {error}"));
                self.status_is_error = true;
            }
        }
        cx.notify();
    }

    fn invoke_command(&mut self, command_id: String, cx: &mut Context<Self>) {
        let Some(controller) = self.controller.as_mut() else {
            return;
        };

        match controller.handle(ProjectionIntent::InvokeCommand(command_id)) {
            Ok(snapshot) => {
                let previous = self.selected;
                self.snapshot = snapshot;
                self.selected = selection_for_snapshot(previous, &self.snapshot);
                self.status = None;
                self.status_is_error = false;
            }
            Err(error) => {
                self.status = Some(format!("Couldn't continue: {error}"));
                self.status_is_error = true;
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
        let mut items = div().flex().flex_wrap().gap_2();
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
            .min_w(px(220.0))
            .flex_1()
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

    fn render_inspector(&self, cx: &mut Context<Self>) -> Option<Div> {
        let selection = self.selected?;
        let inspector = self.snapshot.inspector(selection)?;
        let mut panel = inspector_panel(inspector);

        if let SelectionId::Entity(entity) = selection {
            let history = self.snapshot.entity_history(entity);
            if !history.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for item in history.iter().take(ENTITY_HISTORY_LIMIT) {
                    items = items.child(self.entity_history_item(item, cx));
                }
                panel = panel
                    .child(div().text_sm().child("Recorded changes to this entity"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Recorded events whose StateChanges directly changed this entity. Select one to inspect the event, trace its causes and effects, or fork before it."),
                    )
                    .child(items);
                let hidden = history.len().saturating_sub(ENTITY_HISTORY_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(format!("{hidden} more recorded entity changes not shown")),
                    );
                }
            }
        }

        if let SelectionId::Relation(relation) = selection {
            let history = self.snapshot.relation_history(relation);
            if !history.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for item in history.iter().take(RELATION_HISTORY_LIMIT) {
                    items = items.child(self.relation_history_item(item, cx));
                }
                panel = panel
                    .child(div().text_sm().child("Recorded changes to this relation"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Recorded events that created, changed, or removed this relation incarnation. Select one to inspect the event, trace its causes and effects, or fork before it."),
                    )
                    .child(items);
                let hidden = history.len().saturating_sub(RELATION_HISTORY_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(format!("{hidden} more recorded relation changes not shown")),
                    );
                }
            }
        }

        if let SelectionId::Event(event) = selection {
            let changed_entities = self.snapshot.directly_changed_entities(event);
            if !changed_entities.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for entity in changed_entities.iter().take(EVENT_ENTITY_EFFECT_LIMIT) {
                    items = items
                        .child(self.event_entity_effect_item(SelectionId::Entity(*entity), cx));
                }
                panel = panel
                    .child(div().text_sm().child("Entities changed by this event"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Entities with a direct recorded StateChange from this visible event. Select one to inspect its current state and recorded history."),
                    )
                    .child(items);
                let hidden = changed_entities
                    .len()
                    .saturating_sub(EVENT_ENTITY_EFFECT_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777777))
                            .child(format!("{hidden} more directly changed entities not shown")),
                    );
                }
            }

            let changed_relations = self.snapshot.directly_changed_relations(event);
            if !changed_relations.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for relation in changed_relations.iter().take(EVENT_RELATION_EFFECT_LIMIT) {
                    items = items.child(
                        self.event_relation_effect_item(SelectionId::Relation(*relation), cx),
                    );
                }
                panel = panel
                    .child(div().text_sm().child("Relations changed by this event"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x66705f))
                            .child("Relation incarnations whose recorded lifetime or properties directly changed in this visible event. Removed relations remain inspectable as recorded tombstones."),
                    )
                    .child(items);
                let hidden = changed_relations
                    .len()
                    .saturating_sub(EVENT_RELATION_EFFECT_LIMIT);
                if hidden > 0 {
                    panel = panel.child(div().text_xs().text_color(rgb(0x777777)).child(format!(
                        "{hidden} more directly changed relations not shown"
                    )));
                }
            }
        }

        Some(panel)
    }

    fn event_entity_effect_item(
        &self,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot
            .inspector(selection)
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Entity".into(), "Recorded entity".into()));
        div()
            .id(SharedString::from(format!(
                "event-entity-effect-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .bg(rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(div().text_xs().text_color(rgb(0x666666)).child(subtitle))
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn event_relation_effect_item(
        &self,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot
            .inspector(selection)
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Relation".into(), "Recorded relation".into()));
        div()
            .id(SharedString::from(format!(
                "event-relation-effect-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .bg(rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(div().text_xs().text_color(rgb(0x666666)).child(subtitle))
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn entity_history_item(&self, item: &TimelineItem, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = item.id;
        div()
            .id(SharedString::from(format!(
                "entity-history-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .bg(rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x555555))
                    .child(item.subtitle.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(format!("World time {}", item.world_time)),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn relation_history_item(
        &self,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        div()
            .id(SharedString::from(format!(
                "relation-history-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe2e4e8))
            .bg(rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x555555))
                    .child(item.subtitle.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(format!("World time {}", item.world_time)),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn render_influence(&self, cx: &mut Context<Self>) -> Option<Div> {
        let SelectionId::Event(event) = self.selected? else {
            return None;
        };
        let raw_influence = self.snapshot.influence(event);
        if raw_influence.is_empty() {
            return None;
        }
        let semantic_influence = self.snapshot.semantic_influence(event);
        let semantic_path = self.snapshot.semantic_path_details(event);

        let recorded = raw_influence.len();
        let visible = semantic_influence.len();
        let folded = recorded.saturating_sub(visible);
        let direct = semantic_influence
            .iter()
            .filter(|(depth, _)| *depth == 1)
            .count();
        let max_depth = semantic_influence
            .iter()
            .map(|(depth, _)| *depth)
            .max()
            .unwrap_or_default();
        let mut other_nodes = div().flex().flex_col().gap_1();
        let mut other_count = 0_usize;
        for (depth, item) in &semantic_influence {
            if semantic_path
                .iter()
                .any(|(_, path_item, _)| path_item.id == item.id)
            {
                continue;
            }
            other_count += 1;
            if other_count <= 6 {
                other_nodes = other_nodes.child(self.influence_node(*depth, item, cx));
            }
        }

        let summary = if visible == 0 {
            format!(
                "No world-visible effects yet · {recorded} recorded downstream {} · {folded} supporting {} folded",
                if recorded == 1 { "event" } else { "events" },
                if folded == 1 { "record" } else { "records" },
            )
        } else {
            format!(
                "{visible} world-visible {} from {recorded} recorded downstream {} · {direct} direct · {folded} supporting {} folded · up to {max_depth} causal {}",
                if visible == 1 { "effect" } else { "effects" },
                if recorded == 1 { "event" } else { "events" },
                if folded == 1 { "record" } else { "records" },
                if max_depth == 1 { "step" } else { "steps" },
            )
        };

        let mut panel = div()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xd7e2d7))
            .bg(rgb(0xf7fbf7))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child("SEMANTIC IMPACT"),
            )
            .child(div().text_lg().child("What this affected"))
            .child(div().text_xs().text_color(rgb(0x657565)).child(summary));

        if !semantic_path.is_empty() {
            let path_len = semantic_path.len();
            let mut path_nodes = div().flex().flex_col().gap_1();
            if path_len <= 6 {
                for (index, (causal_steps, item, effect)) in semantic_path.iter().enumerate() {
                    path_nodes = path_nodes.child(self.semantic_path_node(
                        index + 1,
                        *causal_steps,
                        item,
                        effect,
                        cx,
                    ));
                }
            } else {
                for (index, (causal_steps, item, effect)) in
                    semantic_path.iter().take(2).enumerate()
                {
                    path_nodes = path_nodes.child(self.semantic_path_node(
                        index + 1,
                        *causal_steps,
                        item,
                        effect,
                        cx,
                    ));
                }
                path_nodes = path_nodes.child(
                    div()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(rgb(0x657565))
                        .child(format!(
                            "+{} intermediate world-visible stages",
                            path_len - 5
                        )),
                );
                for (index, (causal_steps, item, effect)) in
                    semantic_path.iter().enumerate().skip(path_len - 3)
                {
                    path_nodes = path_nodes.child(self.semantic_path_node(
                        index + 1,
                        *causal_steps,
                        item,
                        effect,
                        cx,
                    ));
                }
            }
            panel = panel
                .child(div().text_xs().text_color(rgb(0x657565)).child("HOW IT UNFOLDED"))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x657565))
                        .child(format!(
                            "Representative causal thread from the selected Event to the latest downstream effect · {path_len} world-visible {}",
                            if path_len == 1 { "stage" } else { "stages" }
                        )),
                )
                .child(path_nodes);
        }

        if other_count > 0 {
            panel = panel
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x657565))
                        .child("OTHER WORLD-VISIBLE EFFECTS"),
                )
                .child(other_nodes);
            if other_count > 6 {
                panel = panel.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x657565))
                        .child(format!("+{} more world-visible effects", other_count - 6)),
                );
            }
        }
        if folded > 0 {
            panel = panel.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child("Supporting records remain available in Timeline and Why."),
            );
        }
        Some(panel)
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

    fn semantic_path_node(
        &self,
        stage: usize,
        causal_steps: usize,
        item: &TimelineItem,
        effect: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let source = if stage == 1 {
            "selected Event"
        } else {
            "previous visible stage"
        };
        let causal_context = if causal_steps == 1 {
            format!("Stage {stage} · direct recorded causal step from {source}")
        } else {
            format!(
                "Stage {stage} · {causal_steps} recorded causal steps from {source} · {} supporting records folded",
                causal_steps - 1
            )
        };
        let event_ref = match selection {
            SelectionId::Event(event) => format!("World time {} · Event #{event}", item.world_time),
            SelectionId::Entity(_) | SelectionId::Relation(_) => {
                unreachable!("semantic path items must be Events")
            }
        };
        div()
            .id(SharedString::from(format!(
                "semantic-path-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .bg(rgb(0xffffff))
            .cursor_pointer()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x657565))
                    .child(causal_context),
            )
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x4f5f4f))
                    .child(effect.to_string()),
            )
            .child(div().text_xs().text_color(rgb(0x777777)).child(event_ref))
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn influence_node(
        &self,
        depth: usize,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let prefix = if depth == 1 {
            "Direct world effect".to_string()
        } else {
            format!("Later world effect · {depth} causal steps")
        };
        div()
            .id(SharedString::from(format!(
                "influence-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .bg(rgb(0xffffff))
            .cursor_pointer()
            .child(div().text_xs().text_color(rgb(0x657565)).child(prefix))
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777777))
                    .child(item.subtitle.clone()),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
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
            if let Some(inspector) = self.render_inspector(cx) {
                center = center.child(inspector);
            }
            if let Some(why) = self.render_why(cx) {
                center = center.child(why);
            }
            if let Some(influence) = self.render_influence(cx) {
                center = center.child(influence);
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
                    .text_color(if self.status_is_error {
                        rgb(0xa33a3a)
                    } else {
                        rgb(0x4e6fb3)
                    })
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

    for section in inspector.display_sections() {
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
        .flex()
        .flex_col()
        .gap_3()
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
