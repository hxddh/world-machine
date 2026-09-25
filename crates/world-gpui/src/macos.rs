use crate::ui::{self, color, ButtonKind};
use crate::ProjectionController;
use gpui::{
    div, prelude::*, px, relative, Context, Div, FontWeight, IntoElement, Render, SharedString,
    Styled, Window,
};
use std::rc::Rc;
use world_projection::{
    BriefingItem, BriefingItemKind, CanvasItemKind, CollectionItem, CommandEffect, EffectChange,
    InspectorProjection, ProjectionCommand, ProjectionIntent, ProjectionSnapshot, SelectionId,
    TimelineItem, Tone, WhyNode,
};
use world_theme::tokens;

use crate::scene;

const ENTITY_HISTORY_LIMIT: usize = 6;
const RELATION_HISTORY_LIMIT: usize = 6;
const ENTITY_RELATION_LIMIT: usize = 6;
const RELATION_ENDPOINT_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
const EVENT_RELATION_EFFECT_LIMIT: usize = 6;
/// How much of the history the sidebar lists. Older moments stay reachable
/// through Why and each person's own history.
const HISTORY_LIMIT: usize = 40;
/// The page never grows past a comfortable width, however wide the window
/// is: wide enough for the scene, narrow enough to read.
const PAGE_WIDTH: f32 = 980.0;
const SIDEBAR_WIDTH: f32 = 300.0;
/// Below this width the sidebar folds under the reading column instead of
/// squeezing it.
const TWO_COLUMN_MIN_WIDTH: f32 = 920.0;

/// How tall a place to begin is drawn side by side, and stacked.
const BEGINNING_TALL: f32 = 360.0;
const BEGINNING_SHORT: f32 = 170.0;

pub struct ProjectionView {
    snapshot: ProjectionSnapshot,
    selected: Option<SelectionId>,
    controller: Option<Box<dyn ProjectionController>>,
    status: Option<String>,
    status_is_error: bool,
    show_header: bool,
    /// The choice under the pointer, whose consequences the scene shows
    /// before it is made.
    previewing: Option<String>,
    /// Moments whose everyday round the reader has unfolded in History.
    routine_open: std::collections::BTreeSet<u64>,
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
            show_header: true,
            previewing: None,
            routine_open: Default::default(),
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

    /// Leaves the title bar to the window that embeds this view, so a World
    /// is not named twice, one bar above the other.
    /// What the view is showing now.
    pub fn snapshot(&self) -> &ProjectionSnapshot {
        &self.snapshot
    }

    pub fn without_header(mut self) -> Self {
        self.show_header = false;
        self
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
            .unwrap_or_else(|| "this moment".into());
        let Some(controller) = self.controller.as_mut() else {
            return;
        };

        match controller.handle(ProjectionIntent::ForkBeforeEvent(event)) {
            Ok(snapshot) => {
                let previous = self.selected;
                self.snapshot = snapshot;
                self.selected = selection_for_snapshot(previous, &self.snapshot);
                self.status = Some(format!("Branched before “{event_title}”"));
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

    /// Changes whenever the World moves on, so what is new animates in and
    /// what is not stays put.
    fn revision(&self) -> String {
        format!(
            "{}-{}",
            self.snapshot.world_time,
            self.snapshot.timeline.items.len()
        )
    }

    /// What the scene lights up: the targets of the choice under the
    /// pointer, or else whoever the news is about.
    fn emphasis(&self) -> scene::Emphasis {
        let targets = self
            .previewing
            .as_deref()
            .and_then(|id| self.snapshot.command(id))
            .map(|command| {
                command
                    .effects
                    .iter()
                    .filter_map(|effect| effect.target)
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        if targets.is_empty() {
            scene::Emphasis::News
        } else {
            scene::Emphasis::Only(targets)
        }
    }

    // ---- Reading column -------------------------------------------------

    fn render_status(&self) -> Option<Div> {
        let status = self.status.as_ref()?;
        let tone = if self.status_is_error {
            tokens::DANGER
        } else {
            tokens::ACCENT_TEXT
        };
        Some(
            div()
                .w_full()
                .px_3()
                .py_2()
                .rounded_md()
                .border_1()
                .border_color(color(tokens::BORDER))
                .bg(color(tokens::SURFACE))
                .text_sm()
                .text_color(color(tone))
                .child(status.clone()),
        )
    }

    /// Where the page starts: which World, what moment, and the headline.
    fn render_masthead(&self) -> Div {
        let (eyebrow, title) = match &self.snapshot.briefing {
            Some(briefing) => (briefing.eyebrow.clone(), briefing.title.clone()),
            None => (String::new(), self.snapshot.title.clone()),
        };
        let mut meta = Vec::new();
        if !eyebrow.is_empty() {
            meta.push(eyebrow);
        }
        if self.snapshot.world_time > 0 {
            meta.push(self.snapshot.moment_label(self.snapshot.world_time));
        }
        let mut heading = div().flex_1().min_w(px(0.0)).flex().flex_col().gap_1();
        if !meta.is_empty() {
            heading = heading.child(ui::section_label(meta.join(" · ")));
        }
        let mut masthead = div()
            .flex()
            .items_end()
            .justify_between()
            .gap_6()
            .child(heading.child(ui::page_title(title)));
        if let Some(activity) = scene::activity(&self.snapshot) {
            masthead = masthead.child(activity);
        }
        masthead
    }

    /// What happened, told in order as a short story rather than a grid of
    /// equal boxes.
    fn render_story(&self, cx: &mut Context<Self>) -> Option<Div> {
        let briefing = self.snapshot.briefing.as_ref()?;
        let beats = briefing.beats();
        if beats.is_empty() {
            return None;
        }
        let count = beats.len();
        let mut story = div().flex().flex_col();
        for (index, item) in beats.into_iter().enumerate() {
            story = story.child(ui::arrive(
                self.story_beat(item, index + 1 == count, cx),
                format!("beat-{}-{index}", self.revision()),
                index,
            ));
        }
        Some(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(ui::section_label(if count == 1 {
                    "What happened".to_string()
                } else {
                    format!("What happened · {count}")
                }))
                .child(story),
        )
    }

    fn story_beat(&self, item: &BriefingItem, last: bool, cx: &mut Context<Self>) -> Div {
        // Whoever it happened to, when the World says: a face is read
        // before a sentence.
        let actor = item
            .selection
            .and_then(|selection| scene::event_actor(&self.snapshot, selection));
        let face = match actor {
            Some(name) => ui::avatar(&name, 26.0),
            None => div()
                .size(px(26.0))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .size(px(8.0))
                        .rounded_full()
                        .bg(color(scene::tone_token(item.tone))),
                ),
        };
        // Trouble and good news wear their colour on the face, the way a
        // RimWorld letter wears it on its edge.
        let face = if item.tone == Tone::Neutral {
            face
        } else {
            face.rounded_full()
                .border_2()
                .border_color(color(scene::tone_token(item.tone)))
        };
        let marker = div()
            .flex()
            .flex_col()
            .items_center()
            .w(px(26.0))
            .flex_shrink_0()
            .child(face)
            .child(if last {
                div()
            } else {
                div()
                    .flex_1()
                    .w(px(1.0))
                    .mt_1()
                    .bg(color(tokens::BORDER_STRONG))
            });

        let mut text = div()
            .flex_1()
            .min_w(px(0.0))
            .pb(px(if last { 0.0 } else { 18.0 }))
            .flex()
            .flex_col()
            .gap_1()
            .pt(px(2.0))
            .child(ui::row_title(item.title.clone()));
        if !item.detail.is_empty() {
            text = text.child(
                ui::detail(item.detail.clone())
                    .line_clamp(3)
                    .text_ellipsis(),
            );
        }
        if let Some(selection) = item.selection {
            let id = SharedString::from(format!("story-{}", selection.stable_key()));
            text = text.child(
                div()
                    .id(id)
                    .mt_1()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color(tokens::TEXT_TERTIARY))
                    .cursor_pointer()
                    .hover(|style| style.text_color(color(tokens::ACCENT_TEXT)))
                    .child("Why?")
                    .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx))),
            );
        }

        div().flex().gap_3().child(marker).child(text)
    }

    /// Where a World begins: before anything is on stage, when every choice
    /// comes with a picture of the World it starts, the choices are the
    /// pictures. A new game screen, not a form.
    fn render_beginning(&self, two_columns: bool, cx: &mut Context<Self>) -> Option<Div> {
        if !is_beginning(&self.snapshot) || self.controller.is_none() {
            return None;
        }
        let mut cards = div().w_full().flex().gap_4();
        cards = if two_columns { cards } else { cards.flex_col() };
        for (index, command) in self.snapshot.commands.iter().enumerate() {
            let Some(scenery) = command.scenery else {
                continue;
            };
            let command_id = command.id.clone();
            let card = div()
                .id(SharedString::from(format!("begin-{}", command.id)))
                .flex_1()
                .min_w(px(0.0))
                .rounded_xl()
                .overflow_hidden()
                .border_1()
                .border_color(color(tokens::BORDER))
                .bg(color(tokens::SURFACE))
                .cursor_pointer()
                .hover(|style| style.border_color(color(tokens::ACCENT)).shadow_md())
                .active(|style| style.bg(color(tokens::ROW_SELECTED)))
                .flex()
                .flex_col()
                .child(
                    div()
                        .w_full()
                        // Side by side the places are the whole first
                        // screen, so their pictures take the room; stacked
                        // on a narrow window they stay short enough to scroll.
                        .h(px(if two_columns {
                            BEGINNING_TALL
                        } else {
                            BEGINNING_SHORT
                        }))
                        .child(ui::scenery_cover(&scenery, &command.id).size_full()),
                )
                .child(
                    div()
                        .p_4()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(ui::heading(command.title.clone()))
                        .child(ui::detail(command.detail.clone()))
                        .child(
                            div()
                                .pt_1()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(color(tokens::ACCENT_TEXT))
                                .child("Begin here →"),
                        ),
                )
                .on_click(
                    cx.listener(move |this, _, _, cx| this.invoke_command(command_id.clone(), cx)),
                );
            cards = cards.child(ui::arrive(card, format!("begin-{index}"), index));
        }
        Some(cards)
    }

    /// The decision in front of you, drawn as the one thing on the page that
    /// is plainly meant to be pressed.
    fn render_decision(&self, cx: &mut Context<Self>) -> Option<Div> {
        if self.controller.is_none()
            || self.snapshot.commands.is_empty()
            || is_beginning(&self.snapshot)
        {
            return None;
        }

        let mut choices = div().flex().flex_col().gap_2();
        for (index, command) in self.snapshot.commands.iter().enumerate() {
            choices = choices.child(self.choice(index, command, cx));
        }

        Some(
            div()
                .p_4()
                .rounded_lg()
                .border_1()
                .border_color(color(tokens::ACCENT))
                .bg(color(tokens::ACCENT_SOFT))
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(color(tokens::ACCENT_TEXT))
                                .child("Your turn"),
                        )
                        .child(ui::heading(command_panel_title(
                            self.snapshot.commands.len(),
                        ))),
                )
                .child(choices),
        )
    }

    fn choice(
        &self,
        index: usize,
        command: &ProjectionCommand,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let command_id = command.id.clone();
        let mut text = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap_1()
            .child(ui::row_title(command.title.clone()));
        if !command.detail.is_empty() {
            text = text.child(
                ui::detail(command.detail.clone())
                    .line_clamp(3)
                    .text_ellipsis(),
            );
        }
        if !command.effects.is_empty() {
            let mut chips = div().pt_1().flex().flex_wrap().gap_1();
            for effect in &command.effects {
                chips = chips.child(effect_chip(effect));
            }
            text = text.child(chips);
        }
        let hover_id = command.id.clone();
        div()
            .id(SharedString::from(format!("command-{}", command.id)))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                let previewing = hovered.then(|| hover_id.clone());
                if this.previewing != previewing {
                    this.previewing = previewing;
                    cx.notify();
                }
            }))
            .w_full()
            .px_4()
            .py_3()
            .rounded_md()
            .border_1()
            .border_color(color(tokens::BORDER))
            .bg(color(tokens::SURFACE))
            .cursor_pointer()
            .hover(|style| {
                style
                    .border_color(color(tokens::ACCENT))
                    .bg(color(tokens::SURFACE_HOVER))
            })
            .active(|style| style.bg(color(tokens::ROW_SELECTED)))
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_shrink_0()
                    .size(px(28.0))
                    .rounded_full()
                    .bg(color(tokens::ACCENT_SOFT))
                    .text_color(color(tokens::ACCENT_TEXT))
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(format!("{}", index + 1)),
            )
            .child(text)
            .child(
                div()
                    .flex_shrink_0()
                    .text_base()
                    .text_color(color(tokens::ACCENT_TEXT))
                    .child("→"),
            )
            .on_click(
                cx.listener(move |this, _, _, cx| this.invoke_command(command_id.clone(), cx)),
            )
    }

    /// How things stand: the standing facts a return does not need to lead
    /// with, set after the story and the decision.
    fn render_standing(&self, cx: &mut Context<Self>) -> Option<Div> {
        let briefing = self.snapshot.briefing.as_ref()?;
        let status = briefing
            .items
            .iter()
            .filter(|item| item.kind == BriefingItemKind::Status)
            .collect::<Vec<_>>();
        if status.is_empty() {
            return None;
        }
        let mut grid = div().flex().flex_wrap().gap_3();
        for item in status {
            grid = grid.child(self.standing_card(item, cx));
        }
        Some(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(ui::section_label("Where things stand"))
                .child(grid),
        )
    }

    fn standing_card(&self, item: &BriefingItem, cx: &mut Context<Self>) -> impl IntoElement {
        let id = item
            .selection
            .map(|selection| format!("standing-{}", selection.stable_key()))
            .unwrap_or_else(|| format!("standing-static-{}", item.title));
        let mut card = div()
            .id(SharedString::from(id))
            .min_w(px(240.0))
            .flex_1()
            .p_3()
            .rounded_md()
            .bg(color(tokens::SURFACE))
            .border_1()
            .border_color(color(tokens::BORDER))
            .when(item.tone != Tone::Neutral, |card| {
                card.border_l_4()
                    .border_color(color(scene::tone_token(item.tone)))
            })
            .flex()
            .flex_col()
            .gap_1()
            .child(ui::row_title(item.title.clone()));
        if !item.detail.is_empty() {
            card = card.child(ui::detail(item.detail.clone()));
        }
        if let Some(selection) = item.selection {
            card = card
                .cursor_pointer()
                .hover(|style| style.border_color(color(tokens::BORDER_STRONG)))
                .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)));
        }
        card
    }

    // ---- Sidebar ----------------------------------------------------------

    fn render_cast(&self, cx: &mut Context<Self>) -> Option<Div> {
        if !has_collection_panel(&self.snapshot) {
            return None;
        }
        let mut list = div().flex().flex_col().gap(px(2.0));
        for item in &self.snapshot.collection.items {
            list = list.child(self.cast_row(item, cx));
        }
        Some(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(div().px_3().child(ui::section_label(collection_title(
                    &self.snapshot.collection.title,
                ))))
                .child(list),
        )
    }

    fn cast_row(&self, item: &CollectionItem, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = item.id;
        let selected = self.selected == Some(selection);
        let is_actor = self
            .snapshot
            .canvas
            .items
            .iter()
            .any(|canvas| canvas.id == selection && canvas.kind == CanvasItemKind::Actor);
        let mut face = ui::avatar(&item.title, 30.0);
        if !is_actor {
            face = face.rounded_md();
        }
        let mut text = div()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(ui::row_title(item.title.clone()).truncate());
        if !item.subtitle.is_empty() {
            text = text.child(ui::caption(capitalize(&item.subtitle)));
        }
        ui::list_row(
            SharedString::from(format!("collection-{}", selection.stable_key())),
            selected,
        )
        .flex_row()
        .items_center()
        .gap_3()
        .child(face)
        .child(text)
        .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    /// Everything that happened, newest first, grouped by the moment it
    /// happened in rather than stamped on every line. Each moment tells
    /// its story; its everyday round folds into one line that opens.
    fn render_history(&self, cx: &mut Context<Self>) -> Option<Div> {
        if !has_timeline_panel(&self.snapshot) {
            return None;
        }
        let (shown, hidden) = history_window(&self.snapshot.timeline.items, HISTORY_LIMIT);
        // Wrapped lines need a width to wrap in, all the way down.
        let mut history = div().w_full().flex().flex_col().gap_1();
        for section in history_sections(history_groups(shown.into_iter())) {
            let key = section.newest;
            let mut rows = div().w_full().flex().flex_col().gap(px(2.0));
            for item in &section.story {
                rows = rows.child(self.history_row(item, cx));
            }
            if !section.routine.is_empty() {
                if self.routine_open.contains(&key) {
                    for item in &section.routine {
                        rows = rows.child(self.history_row(item, cx));
                    }
                } else {
                    rows = rows.child(self.routine_row(key, &section.routine, cx));
                }
            }
            history = history
                .child(
                    div()
                        .px_3()
                        .pt_2()
                        .child(ui::caption(section.label(&self.snapshot))),
                )
                .child(rows);
        }
        if hidden > 0 {
            history = history.child(
                div()
                    .px_3()
                    .pt_2()
                    .child(ui::caption(format!("{hidden} earlier moments"))),
            );
        }
        Some(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().px_3().child(ui::section_label("History")))
                .child(history),
        )
    }

    /// One thing that happened: the face of whoever it happened to and
    /// what happened, in the World's own words. The full account is one
    /// click away.
    fn history_row(&self, item: &TimelineItem, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = item.id;
        let selected = self.selected == Some(selection);
        let actor = scene::event_actor(&self.snapshot, selection);
        let face = match &actor {
            Some(name) => ui::avatar(name, 20.0),
            None => div()
                .size(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .size(px(6.0))
                        .rounded_full()
                        .bg(color(tokens::TEXT_TERTIARY)),
                ),
        };
        // The line is what happened; the summary under it only when it
        // says something the line did not.
        let summary = history_summary(&item.subtitle, actor.as_deref());
        let mut text = div()
            .min_w(px(0.0))
            .flex_1()
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(
                ui::body(item.title.clone())
                    .w_full()
                    .line_clamp(2)
                    .text_ellipsis(),
            );
        if !summary.is_empty() && !item.title.contains(summary.as_str()) {
            text = text.child(ui::caption(summary).truncate());
        }
        ui::list_row(
            SharedString::from(format!("timeline-{}", selection.stable_key())),
            selected,
        )
        .py(px(6.0))
        .flex_row()
        .items_start()
        .gap_2()
        .child(face)
        .child(text)
        .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    /// A moment's everyday round, folded: the faces of those it involved
    /// and how much of it there was. Clicking unfolds it.
    fn routine_row(
        &self,
        world_time: u64,
        items: &[&TimelineItem],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut faces = div().flex().flex_row();
        let mut seen = Vec::new();
        for item in items {
            if let Some(name) = scene::event_actor(&self.snapshot, item.id) {
                if !seen.contains(&name) && seen.len() < 4 {
                    faces = faces.child(
                        div()
                            .when(!seen.is_empty(), |face| face.ml(px(-6.0)))
                            .child(ui::avatar(&name, 16.0)),
                    );
                    seen.push(name);
                }
            }
        }
        ui::list_row(SharedString::from(format!("routine-{world_time}")), false)
            .py(px(4.0))
            .flex_row()
            .items_center()
            .gap_2()
            .child(faces)
            .child(ui::caption(everyday_label(items.len())))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.routine_open.insert(world_time);
                cx.notify();
            }))
    }

    // ---- Look closer ------------------------------------------------------

    fn render_inspector(&self, cx: &mut Context<Self>) -> Option<Div> {
        let selection = self.selected?;
        let inspector = self.snapshot.inspector(selection)?;
        let mut panel = inspector_panel(inspector);

        if let SelectionId::Entity(entity) = selection {
            let relations = self.snapshot.relations_for_entity(entity);
            if !relations.is_empty() {
                let mut items = div().flex().flex_col().gap(px(2.0));
                for relation in relations.iter().take(ENTITY_RELATION_LIMIT) {
                    items = items.child(self.linked_row(
                        "entity-current-relation",
                        SelectionId::Relation(*relation),
                        "Relationship",
                        cx,
                    ));
                }
                panel = panel.child(linked_section(
                    "Connected to",
                    items,
                    relations.len().saturating_sub(ENTITY_RELATION_LIMIT),
                ));
            }

            let history = self.snapshot.entity_history(entity);
            if !history.is_empty() {
                let mut items = div().flex().flex_col().gap(px(2.0));
                for item in history.iter().take(ENTITY_HISTORY_LIMIT) {
                    items = items.child(self.moment_row("entity-history", item, cx));
                }
                panel = panel.child(linked_section(
                    "What changed it",
                    items,
                    history.len().saturating_sub(ENTITY_HISTORY_LIMIT),
                ));
            }
        }

        if let SelectionId::Relation(relation) = selection {
            let endpoints = self.snapshot.entities_for_relation(relation);
            if !endpoints.is_empty() {
                let mut items = div().flex().flex_col().gap(px(2.0));
                for entity in endpoints.iter().take(RELATION_ENDPOINT_LIMIT) {
                    items = items.child(self.linked_row(
                        "relation-current-endpoint",
                        SelectionId::Entity(*entity),
                        "",
                        cx,
                    ));
                }
                panel = panel.child(linked_section(
                    "Between",
                    items,
                    endpoints.len().saturating_sub(RELATION_ENDPOINT_LIMIT),
                ));
            }

            let history = self.snapshot.relation_history(relation);
            if !history.is_empty() {
                let mut items = div().flex().flex_col().gap(px(2.0));
                for item in history.iter().take(RELATION_HISTORY_LIMIT) {
                    items = items.child(self.moment_row("relation-history", item, cx));
                }
                panel = panel.child(linked_section(
                    "How it changed",
                    items,
                    history.len().saturating_sub(RELATION_HISTORY_LIMIT),
                ));
            }
        }

        if let SelectionId::Event(event) = selection {
            let changed_entities = self.snapshot.directly_changed_entities(event);
            let changed_relations = self.snapshot.directly_changed_relations(event);
            if !changed_entities.is_empty() || !changed_relations.is_empty() {
                let mut items = div().flex().flex_col().gap(px(2.0));
                for entity in changed_entities.iter().take(EVENT_ENTITY_EFFECT_LIMIT) {
                    items = items.child(self.linked_row(
                        "event-entity-effect",
                        SelectionId::Entity(*entity),
                        "",
                        cx,
                    ));
                }
                for relation in changed_relations.iter().take(EVENT_RELATION_EFFECT_LIMIT) {
                    items = items.child(self.linked_row(
                        "event-relation-effect",
                        SelectionId::Relation(*relation),
                        "Relationship",
                        cx,
                    ));
                }
                let hidden = changed_entities
                    .len()
                    .saturating_sub(EVENT_ENTITY_EFFECT_LIMIT)
                    + changed_relations
                        .len()
                        .saturating_sub(EVENT_RELATION_EFFECT_LIMIT);
                panel = panel.child(linked_section("What this changed", items, hidden));
            }
        }

        Some(panel)
    }

    /// A person, place, or relationship named inside another one's details.
    fn linked_row(
        &self,
        prefix: &str,
        selection: SelectionId,
        fallback: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot
            .inspector(selection)
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| (fallback.to_string(), String::new()));
        let mut row = ui::list_row(
            SharedString::from(format!("{prefix}-{}", selection.stable_key())),
            false,
        )
        .child(ui::row_title(title));
        if !subtitle.is_empty() {
            row = row.child(ui::caption(subtitle));
        }
        row.on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    /// A moment in someone's history.
    fn moment_row(
        &self,
        prefix: &str,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let mut row = ui::list_row(
            SharedString::from(format!("{prefix}-{}", selection.stable_key())),
            false,
        )
        .child(
            div()
                .flex()
                .justify_between()
                .gap_3()
                .child(ui::row_title(item.title.clone()))
                .child(ui::caption(self.snapshot.moment_label(item.world_time))),
        );
        if !item.subtitle.is_empty() {
            row = row.child(
                ui::detail(item.subtitle.clone())
                    .line_clamp(2)
                    .text_ellipsis(),
            );
        }
        row.on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
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

        let visible = semantic_influence.len();
        let max_depth = semantic_influence
            .iter()
            .map(|(depth, _)| *depth)
            .max()
            .unwrap_or_default();
        let mut other_nodes = div().flex().flex_col().gap(px(2.0));
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
            "Nothing anyone would notice has come of it yet.".to_string()
        } else {
            format!(
                "{visible} {} so far, up to {max_depth} {} removed.",
                if visible == 1 {
                    "thing followed from it"
                } else {
                    "things followed from it"
                },
                if max_depth == 1 { "step" } else { "steps" },
            )
        };

        let mut panel = ui::card().flex().flex_col().gap_3().child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(ui::heading("What it led to"))
                .child(ui::detail(summary)),
        );

        if !semantic_path.is_empty() {
            let path_len = semantic_path.len();
            let mut path_nodes = div().flex().flex_col().gap(px(2.0));
            let shown: Vec<usize> = if path_len <= 6 {
                (0..path_len).collect()
            } else {
                (0..2).chain(path_len - 3..path_len).collect()
            };
            for (position, index) in shown.iter().enumerate() {
                if path_len > 6 && position == 2 {
                    path_nodes = path_nodes.child(
                        div()
                            .px_3()
                            .py_1()
                            .child(ui::caption(format!("{} steps in between", path_len - 5))),
                    );
                }
                let (_, item, effect) = &semantic_path[*index];
                path_nodes = path_nodes.child(self.semantic_path_node(item, effect, cx));
            }
            panel = panel.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::section_label("How it unfolded"))
                    .child(path_nodes),
            );
        }

        if other_count > 0 {
            let mut others = div()
                .flex()
                .flex_col()
                .gap_1()
                .child(ui::section_label("Also because of it"))
                .child(other_nodes);
            if other_count > 6 {
                others = others.child(
                    div()
                        .px_3()
                        .child(ui::caption(format!("{} more", other_count - 6))),
                );
            }
            panel = panel.child(others);
        }
        Some(panel)
    }

    fn render_why(&self, cx: &mut Context<Self>) -> Option<Div> {
        let SelectionId::Event(event) = self.selected? else {
            return None;
        };
        let why = self.snapshot.why(event)?;

        let mut nodes = div().flex().flex_col().gap(px(2.0));
        for node in why.nodes.iter().take(10) {
            nodes = nodes.child(self.why_node(node, cx));
        }

        let mut header = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(ui::heading("Why it happened"));
        if self.controller.is_some() && self.snapshot.capabilities.fork {
            header = header.child(
                ui::button(
                    "fork-before-event",
                    "Branch from before this",
                    ButtonKind::Secondary,
                )
                .on_click(cx.listener(|this, _, _, cx| this.fork_before_selected(cx))),
            );
        }

        Some(
            ui::card()
                .flex()
                .flex_col()
                .gap_3()
                .child(header)
                .child(nodes),
        )
    }

    fn semantic_path_node(
        &self,
        item: &TimelineItem,
        effect: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let mut row = ui::list_row(
            SharedString::from(format!("semantic-path-{}", selection.stable_key())),
            false,
        )
        .child(
            div()
                .flex()
                .justify_between()
                .gap_3()
                .child(ui::row_title(item.title.clone()))
                .child(ui::caption(self.snapshot.moment_label(item.world_time))),
        );
        let effect = world_projection::effect_headline(effect);
        if !effect.is_empty() {
            row = row.child(ui::detail(effect.to_string()).line_clamp(2).text_ellipsis());
        }
        row.on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn influence_node(
        &self,
        depth: usize,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let distance = if depth == 1 {
            "Directly".to_string()
        } else {
            format!("{depth} steps on")
        };
        let mut row = ui::list_row(
            SharedString::from(format!("influence-{}", selection.stable_key())),
            false,
        )
        .child(
            div()
                .flex()
                .justify_between()
                .gap_3()
                .child(ui::row_title(item.title.clone()))
                .child(ui::caption(distance)),
        );
        if !item.subtitle.is_empty() {
            row = row.child(
                ui::detail(item.subtitle.clone())
                    .line_clamp(2)
                    .text_ellipsis(),
            );
        }
        row.on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn why_node(&self, node: &WhyNode, cx: &mut Context<Self>) -> impl IntoElement {
        let selection = SelectionId::Event(node.event);
        let selected = node.depth == 0;
        let mut row = ui::list_row(
            SharedString::from(format!("why-event-{}", node.event)),
            selected,
        )
        .pl(px(12.0 + 16.0 * node.depth.min(4) as f32))
        .child(
            div()
                .flex()
                .gap_2()
                .child(if node.depth == 0 {
                    div()
                } else {
                    ui::caption("because")
                })
                .child(ui::row_title(node.title.clone())),
        );
        if !node.subtitle.is_empty() {
            row = row.child(
                ui::detail(node.subtitle.clone())
                    .line_clamp(2)
                    .text_ellipsis(),
            );
        }
        row.on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn render_closer_look(&self, cx: &mut Context<Self>) -> Option<Div> {
        if !has_exploration(&self.snapshot, self.selected) {
            return None;
        }
        let parts = [
            self.render_inspector(cx),
            self.render_why(cx),
            self.render_influence(cx),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        if parts.is_empty() {
            return None;
        }
        let mut section = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(ui::section_label("Look closer"));
        for part in parts {
            section = section.child(part);
        }
        Some(section)
    }
}

impl Render for ProjectionView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        let two_columns = window.viewport_size().width >= px(TWO_COLUMN_MIN_WIDTH);

        let mut column = div()
            .w_full()
            .max_w(px(PAGE_WIDTH))
            .mx_auto()
            .px_8()
            .py_8()
            .flex()
            .flex_col()
            .gap_6();
        if let Some(status) = self.render_status() {
            column = column.child(status);
        }
        column = column.child(self.render_masthead());
        // The stage is laid out in pixels so nothing overlaps at the width
        // it will actually be drawn at.
        let main_width =
            f32::from(window.viewport_size().width) - if two_columns { SIDEBAR_WIDTH } else { 0.0 };
        let stage_width = main_width.min(PAGE_WIDTH) - 64.0 - 2.0;
        let view = cx.entity().downgrade();
        let on_select: scene::SelectHandler = Rc::new(move |selection, _, cx| {
            view.update(cx, |this, cx| this.select(selection, cx)).ok();
        });
        if let Some(scene) = scene::scene(
            &self.snapshot,
            stage_width,
            self.selected,
            &self.emphasis(),
            on_select,
        ) {
            column = column.child(scene);
        }

        if let Some(beginning) = self.render_beginning(two_columns, cx) {
            column = column.child(beginning);
        }
        // The decision and the news that led to it sit side by side under
        // the scene when there is room, and stack when there is not.
        let decision = self.render_decision(cx);
        let story = self.render_story(cx);
        let side_by_side = two_columns && decision.is_some() && story.is_some();
        let mut pair = div().flex().gap_6();
        pair = if side_by_side {
            pair.items_start()
        } else {
            pair.flex_col()
        };
        if let Some(decision) = decision {
            pair = pair.child(div().flex_1().min_w(px(0.0)).child(ui::arrive(
                decision,
                format!("decision-{}", self.revision()),
                1,
            )));
        }
        if let Some(story) = story {
            pair = pair.child(div().flex_1().min_w(px(0.0)).child(story));
        }
        column = column.child(pair);
        if let Some(standing) = self.render_standing(cx) {
            column = column.child(standing);
        }

        let cast = self.render_cast(cx);
        let history = self.render_history(cx);
        let has_sidebar = cast.is_some() || history.is_some();
        let mut sidebar = div().flex().flex_col().gap_6();
        if let Some(cast) = cast {
            sidebar = sidebar.child(cast);
        }
        if let Some(history) = history {
            sidebar = sidebar.child(history);
        }

        let mut sidebar = Some(sidebar);
        if !two_columns && has_sidebar {
            if let Some(sidebar) = sidebar.take() {
                column = column.child(ui::divider()).child(sidebar.mx(px(-12.0)));
            }
        }
        if let Some(closer) = self.render_closer_look(cx) {
            column = column.child(closer);
        }

        let mut workspace = div()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .min_w(px(0.0))
            .overflow_hidden()
            .flex()
            .child(
                div()
                    .id("projection-center-scroll")
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .overflow_y_scroll()
                    .child(column),
            );
        if let Some(sidebar) = sidebar.filter(|_| has_sidebar) {
            workspace = workspace.child(
                div()
                    .id("projection-sidebar-scroll")
                    .w(px(SIDEBAR_WIDTH))
                    .flex_shrink_0()
                    .h_full()
                    .overflow_y_scroll()
                    .border_l_1()
                    .border_color(color(tokens::BORDER))
                    .bg(color(tokens::SIDEBAR))
                    .px_2()
                    .py_6()
                    .child(sidebar),
            );
        }

        let mut root = div()
            .size_full()
            .bg(color(tokens::WINDOW))
            .text_color(color(tokens::TEXT))
            .flex()
            .flex_col();
        if self.show_header {
            root = root.child(
                div()
                    .h(px(52.0))
                    .w_full()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_5()
                    .border_b_1()
                    .border_color(color(tokens::BORDER))
                    .child(
                        div()
                            .text_base()
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .child(self.snapshot.title.clone()),
                    ),
            );
        }
        root.child(workspace)
    }
}

/// A timeline line without the actor's name in front of it: the face beside
/// it already says who.
fn history_summary(subtitle: &str, actor: Option<&str>) -> String {
    let Some(actor) = actor else {
        return subtitle.to_string();
    };
    if subtitle == actor {
        return String::new();
    }
    subtitle
        .strip_prefix(actor)
        .and_then(|rest| rest.strip_prefix(" · "))
        .unwrap_or(subtitle)
        .to_string()
}

/// One consequence of a choice, as a small coloured chip: "↑ Trust",
/// "Sea Finch → repaired".
fn effect_chip(effect: &CommandEffect) -> Div {
    let (text, ground) = match effect.tone {
        Tone::Neutral => (tokens::TEXT_SECONDARY, tokens::ROW_HOVER),
        Tone::Good => (tokens::SUCCESS, tokens::SUCCESS_SOFT),
        Tone::Warning => (tokens::WARNING, tokens::WARNING_SOFT),
        Tone::Bad => (tokens::DANGER, tokens::DANGER_SOFT),
    };
    let label = match &effect.change {
        EffectChange::Up => format!("↑ {}", effect.label),
        EffectChange::Down => format!("↓ {}", effect.label),
        EffectChange::To(value) => format!("{} → {value}", effect.label),
    };
    div()
        .px_2()
        .py(px(1.0))
        .rounded_full()
        .bg(color(ground))
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(color(text))
        .child(label)
}

/// A moment in a World, named the way a person would say it.
fn collection_title(title: &str) -> String {
    // "World Contents" is what the projection layer calls a list of everything
    // in a World; to someone reading it, those are the people and places.
    if title.is_empty() || title == "World Contents" {
        "People and places".into()
    } else {
        title.to_string()
    }
}

pub(crate) fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

struct HistoryGroup<'a> {
    world_time: u64,
    items: Vec<&'a TimelineItem>,
}

/// A World with nothing on stage yet whose every choice shows the World it
/// would start.
pub fn is_beginning(snapshot: &ProjectionSnapshot) -> bool {
    snapshot.canvas.items.is_empty()
        && !snapshot.commands.is_empty()
        && snapshot
            .commands
            .iter()
            .all(|command| command.scenery.is_some())
}

/// The newest part of History: items up to the `limit`th thing that
/// happened to someone, with the everyday round in between riding along
/// free, since it folds away. Also how many stories were left out.
fn history_window(items: &[TimelineItem], limit: usize) -> (Vec<&TimelineItem>, usize) {
    let mut stories = 0;
    let mut shown = Vec::new();
    for item in items {
        if !item.routine {
            if stories == limit {
                break;
            }
            stories += 1;
        }
        shown.push(item);
    }
    let hidden = items[shown.len()..]
        .iter()
        .filter(|item| !item.routine)
        .count();
    (shown, hidden)
}

/// A stretch of History under one heading: a moment where something
/// happened, or several quiet moments in a row folded into one.
struct HistorySection<'a> {
    newest: u64,
    oldest: u64,
    story: Vec<&'a TimelineItem>,
    routine: Vec<&'a TimelineItem>,
}

impl HistorySection<'_> {
    fn label(&self, snapshot: &ProjectionSnapshot) -> String {
        snapshot.span_label(self.oldest, self.newest)
    }
}

/// Moments where nothing happened to anyone run together, so a long quiet
/// stretch reads as one line rather than a column of identical ones.
fn history_sections<'a>(groups: Vec<HistoryGroup<'a>>) -> Vec<HistorySection<'a>> {
    let mut sections: Vec<HistorySection<'a>> = Vec::new();
    for group in groups {
        let (routine, story): (Vec<_>, Vec<_>) =
            group.items.into_iter().partition(|item| item.routine);
        match sections.last_mut() {
            Some(quiet) if story.is_empty() && quiet.story.is_empty() => {
                quiet.oldest = group.world_time;
                quiet.routine.extend(routine);
            }
            _ => sections.push(HistorySection {
                newest: group.world_time,
                oldest: group.world_time,
                story,
                routine,
            }),
        }
    }
    sections
}

fn everyday_label(count: usize) -> String {
    match count {
        1 => "Everyday life".to_string(),
        count => format!("Everyday life · {count} things"),
    }
}

/// Consecutive timeline items that share a moment, in the order given.
fn history_groups<'a>(items: impl Iterator<Item = &'a TimelineItem>) -> Vec<HistoryGroup<'a>> {
    let mut groups: Vec<HistoryGroup<'a>> = Vec::new();
    for item in items {
        match groups.last_mut() {
            Some(group) if group.world_time == item.world_time => group.items.push(item),
            _ => groups.push(HistoryGroup {
                world_time: item.world_time,
                items: vec![item],
            }),
        }
    }
    groups
}

fn linked_section(title: &str, items: Div, hidden: usize) -> Div {
    let mut section = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(ui::section_label(title.to_string()))
        .child(items.mx(px(-12.0)));
    if hidden > 0 {
        section = section.child(ui::caption(format!("{hidden} more")));
    }
    section
}

fn has_exploration(snapshot: &ProjectionSnapshot, selected: Option<SelectionId>) -> bool {
    !snapshot.canvas.items.is_empty()
        || selected
            .and_then(|selection| snapshot.inspector(selection))
            .is_some()
        || matches!(selected, Some(SelectionId::Event(event)) if snapshot.why(event).is_some())
}

fn has_collection_panel(snapshot: &ProjectionSnapshot) -> bool {
    !snapshot.collection.items.is_empty()
}

fn has_timeline_panel(snapshot: &ProjectionSnapshot) -> bool {
    !snapshot.timeline.items.is_empty()
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
    let mut header = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(ui::heading(inspector.title.clone()));
    if !inspector.subtitle.is_empty() {
        header = header.child(ui::caption(capitalize(&inspector.subtitle)));
    }
    let mut body = div().flex().flex_col().gap_4().child(header);

    for section in inspector.display_sections() {
        let mut rows = div().flex().flex_col();
        let mut shown = 0;
        for row in &section.rows {
            shown += 1;
            rows = rows.child(
                div()
                    .flex()
                    .justify_between()
                    .gap_4()
                    .py(px(6.0))
                    .border_b_1()
                    .border_color(color(tokens::BORDER))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(color(tokens::TEXT_SECONDARY))
                            .child(row.label.clone()),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .text_sm()
                            .line_height(relative(1.45))
                            .text_color(color(tokens::TEXT))
                            .child(row.value.clone()),
                    ),
            );
        }
        if shown == 0 {
            continue;
        }
        body = body.child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(ui::section_label(section.title.clone()))
                .child(rows),
        );
    }

    ui::card().flex().flex_col().gap_4().child(body)
}

#[cfg(test)]
mod focus_hierarchy_tests {
    use super::{
        command_panel_title, default_selection, has_collection_panel, has_exploration,
        has_timeline_panel, history_groups, history_sections, history_window,
        selection_for_snapshot,
    };
    use world_projection::{
        CollectionItem, InspectorProjection, ProjectionSnapshot, SelectionId, TimelineItem,
    };

    fn entity_selection() -> SelectionId {
        SelectionId::Entity(Default::default())
    }

    #[test]
    fn quiet_moments_run_together_and_a_story_starts_a_new_heading() {
        let item = |world_time: u64, routine: bool| TimelineItem {
            id: entity_selection(),
            world_time,
            title: String::new(),
            subtitle: String::new(),
            caused_by: Vec::new(),
            routine,
        };
        // Newest first: quiet at 30 and 20, a story at 10, quiet at 5 and 0.
        let items = [
            item(30, true),
            item(20, true),
            item(10, false),
            item(10, true),
            item(5, true),
            item(0, true),
        ];
        let sections = history_sections(history_groups(items.iter()));
        let shape = sections
            .iter()
            .map(|section| {
                (
                    section.label(&ProjectionSnapshot::default()),
                    section.story.len(),
                    section.routine.len(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            shape,
            [
                ("Time 20–30".to_string(), 0, 2),
                ("Time 10".to_string(), 1, 1),
                ("Time 0–5".to_string(), 0, 2),
            ]
        );
    }

    #[test]
    fn history_counts_stories_and_lets_the_everyday_round_ride_along() {
        let item = |routine: bool| TimelineItem {
            id: entity_selection(),
            world_time: 1,
            title: String::new(),
            subtitle: String::new(),
            caused_by: Vec::new(),
            routine,
        };
        // Newest first: a story, three routine things, two stories, routine.
        let items = [false, true, true, true, false, false, true].map(item);
        let (shown, hidden) = history_window(&items, 2);
        // Up to the second story, with the routine between them included.
        assert_eq!(shown.len(), 5);
        assert_eq!(hidden, 1);
        let (shown, hidden) = history_window(&items, 10);
        assert_eq!((shown.len(), hidden), (items.len(), 0));
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
            routine: false,
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
            routine: false,
        });
        snapshot.inspectors.insert(event, inspector(event));
        assert_eq!(default_selection(&snapshot), Some(event));
    }
}
