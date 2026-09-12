use crate::ProjectionController;
use gpui::{
    div, prelude::*, px, relative, Context, Div, IntoElement, Render, SharedString, Styled, Window,
};
use world_projection::{
    BriefingItem, CanvasItemKind, CollectionItem, InspectorProjection, ProjectionCommand,
    ProjectionIntent, ProjectionSnapshot, SelectionId, TimelineItem, WhyNode,
};

const ENTITY_HISTORY_LIMIT: usize = 6;
const RELATION_HISTORY_LIMIT: usize = 6;
const ENTITY_RELATION_LIMIT: usize = 6;
const RELATION_ENDPOINT_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
const EVENT_RELATION_EFFECT_LIMIT: usize = 6;

/// Which of the two surfaces the reader is on.
///
/// A World window used to be one screen with everything on it at once: a rail
/// of entities, a grid of briefing cards, a column of history, a picture and a
/// line, all competing for the same 660 pixels of a 768px screen. That is the
/// shape of a tool you live in. This is not one — every visit is the same
/// short errand: come back, read what happened, decide, leave. So the window
/// has a surface for the errand and a surface for everything else, and the
/// errand does not have to share.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Surface {
    /// What happened, and what you are going to do about it.
    #[default]
    Return,
    /// The cast, the whole history, and whatever is selected in them.
    Reference,
}

pub struct ProjectionView {
    snapshot: ProjectionSnapshot,
    selected: Option<SelectionId>,
    controller: Option<Box<dyn ProjectionController>>,
    status: Option<String>,
    status_is_error: bool,
    surface: Surface,
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
            surface: Surface::default(),
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

    /// Open on the reference surface rather than on the return.
    ///
    /// A reader always wants the errand first, so nothing in the product does
    /// this. It exists because the other surface is otherwise unphotographable:
    /// reaching it needs a click, and the macOS runners that take the
    /// screenshots are granted no Accessibility permission, so a synthesised
    /// click goes nowhere. A surface no picture has ever shown is exactly the
    /// kind of thing this repository has shipped broken before.
    pub fn open_on_the_world(&mut self) {
        self.surface = Surface::Reference;
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
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .border_r_1()
            .border_color(crate::theme_rgb(0xdadada))
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
                crate::theme_rgb(0xe7eefc)
            } else {
                crate::theme_rgb(0xf6f6f6)
            })
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777777))
                    .child(item.subtitle.clone()),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn render_timeline(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div().flex().flex_col().gap_2();
        // Twelve runs rather than twelve entries: a World that charges rent
        // every day filled the whole column with "Living cost paid · Jonas",
        // five times over, and pushed everything that actually happened off
        // the bottom.
        for run in self.snapshot.timeline.runs().into_iter().take(12) {
            body = body.child(self.timeline_item(run, cx));
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
            .border_color(crate::theme_rgb(0xdadada))
            .child(div().text_lg().child("Timeline"))
            .child(body)
    }

    fn timeline_item(
        &self,
        run: world_projection::TimelineRun<'_>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let item = run.item;
        let selection = item.id;
        let selected = self.selected == Some(selection);
        // "Day 77" for one entry; "Day 74 to Day 77 · 4 times" for a run.
        let when = match (item.when.as_deref(), run.since) {
            (Some(when), Some(since)) => Some(format!("{since} to {when} · {} times", run.repeats)),
            (Some(when), None) if run.repeats > 1 => {
                Some(format!("{when} · {} times", run.repeats))
            }
            (Some(when), None) => Some(when.to_owned()),
            (None, _) if run.repeats > 1 => Some(format!("{} times", run.repeats)),
            (None, _) => None,
        };
        div()
            .id(SharedString::from(format!(
                "timeline-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .cursor_pointer()
            .bg(if selected {
                crate::theme_rgb(0xe7eefc)
            } else {
                crate::theme_rgb(0xf7f7f7)
            })
            // The World's own words for when, or nothing. It used to stamp
            // every entry `t=790`, which is a tick count — and on a World
            // whose last four entries all landed in one tick it printed the
            // same unreadable number four times down the column.
            .children(when.map(|when| {
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777777))
                    .child(when)
            }))
            .child(div().text_sm().child(item.title.clone()))
            // An Event with nobody acting and nothing to summarise used to
            // carry "Event #454" here. With that gone the line is empty, and
            // an empty grey line is still a line.
            .children((!item.subtitle.is_empty()).then(|| {
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777777))
                    .child(item.subtitle.clone())
            }))
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    /// What happened, as lines to read down.
    ///
    /// The briefing used to be a wrapping grid of bordered cards of equal
    /// weight — which is the thing this whole design started out trying to fix
    /// and never did, because the cards only ever got tidier. The day the
    /// school and the bakery both ran out of payroll was the fourth of eight
    /// identical boxes. Beats are now lines in the order they happened, and
    /// the counters that are not news are one muted paragraph underneath them
    /// instead of four more boxes the same size as the news.
    fn render_news(&self, cx: &mut Context<Self>) -> Option<Div> {
        let briefing = self.snapshot.briefing.as_ref()?;
        let beats = briefing.beats();

        let mut lines = div().flex().flex_col().gap_2();
        for item in &beats {
            lines = lines.child(self.news_line(item, false, cx));
        }

        // How things stand, under what happened, in the same shape but quieter.
        // Flattening them to "title — detail" strings turned Pocket Universe's
        // return, which marks almost everything as standing rather than news,
        // into eight paragraphs of identical grey.
        let mut standing = div().flex().flex_col().gap_2();
        let mut any_standing = false;
        for item in briefing.standing() {
            any_standing = true;
            standing = standing.child(self.news_line(item, true, cx));
        }

        let mut band = div()
            .w_full()
            .flex()
            .flex_col()
            .gap_3()
            .px_5()
            .py_4()
            .child(div().text_xl().child(briefing.title.clone()));

        if !beats.is_empty() {
            band = band.child(lines);
        } else if !any_standing {
            // Only when the World has said nothing at all. It used to print
            // whenever there were no *beats*, so Maple Street — which reports
            // everything as standing rather than as news — opened with
            // "Nothing happened worth telling you about." directly above two
            // lines saying what had happened.
            band = band.child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x77736c))
                    .child("Nothing happened worth telling you about."),
            );
        }
        if any_standing {
            band = band.child(standing);
        }
        Some(band)
    }

    /// One line of news: what it was, and when or what about it.
    ///
    /// A Pack's `detail` is not a timestamp. Tiny Society's is "Day 56", so the
    /// first version put it on the headline's row, right-aligned — and Pocket
    /// Universe, whose detail is a whole sentence, came out with its headline
    /// on the left margin and its sentence flung against the right one with a
    /// chasm between them. Short details sit beside the headline; long ones go
    /// under it.
    fn news_line(
        &self,
        item: &BriefingItem,
        muted: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // The same number the briefing uses to tell a stamp from a sentence.
        const BESIDE: usize = world_projection::SHORT_DETAIL;
        let id = item
            .selection
            .map(|selection| format!("news-{}", selection.stable_key()))
            .unwrap_or_else(|| format!("news-static-{}", item.title));
        let selection = item.selection;
        let detail = item.detail.trim().to_owned();
        let aside = !detail.is_empty() && detail.chars().count() <= BESIDE;

        let headline = div()
            .text_base()
            .text_color(if muted {
                crate::theme_rgb(0x6b665e)
            } else {
                crate::theme_rgb(0x202020)
            })
            .child(item.title.clone());

        let mut line = div().id(SharedString::from(id)).w_full();
        line = if aside {
            line.flex()
                .items_baseline()
                .justify_between()
                .gap_4()
                .child(headline)
                .child(
                    div()
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(crate::theme_rgb(0x8a857d))
                        .child(detail),
                )
        } else {
            let mut stacked = div().flex().flex_col().child(headline);
            if !detail.is_empty() {
                stacked = stacked.child(
                    div()
                        .text_sm()
                        .text_color(crate::theme_rgb(0x77736c))
                        .child(detail),
                );
            }
            line.child(stacked)
        };
        if let Some(selection) = selection {
            line = line
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)));
        }
        line
    }

    fn render_turn(&self, cx: &mut Context<Self>) -> Div {
        let bar = div()
            .w_full()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(crate::theme_rgb(0xdadada));

        let quiet = if self.controller.is_none() {
            "Open for reading. Nothing here can be changed."
        } else if self.snapshot.commands.is_empty() {
            "Nothing to decide right now. This World is carrying on by itself."
        } else {
            ""
        };
        if !quiet.is_empty() {
            return bar.bg(crate::theme_rgb(0xf7f7f4)).child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x77736c))
                    .child(quiet),
            );
        }

        let mut choices = div().flex().flex_wrap().gap_2();
        for command in &self.snapshot.commands {
            choices = choices.child(self.command_item(command, cx));
        }

        bar.bg(crate::theme_rgb(0xf1f8ee))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x60755a))
                    .child(format!(
                        "YOUR TURN · {}",
                        command_panel_title(self.snapshot.commands.len())
                    )),
            )
            .child(choices)
    }

    fn command_item(
        &self,
        command: &ProjectionCommand,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let command_id = command.id.clone();
        div()
            .id(SharedString::from(format!("command-{}", command.id)))
            // Wide enough to read, bounded so three choices share a row
            // instead of one choice owning the window.
            .min_w(px(190.0))
            .max_w(px(320.0))
            .flex_1()
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xaec5a7))
            .bg(crate::theme_rgb(0xffffff))
            .cursor_pointer()
            .child(div().text_sm().child(command.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x66705f))
                    .child(choice_gist(&command.detail)),
            )
            .on_click(
                cx.listener(move |this, _, _, cx| this.invoke_command(command_id.clone(), cx)),
            )
    }

    /// The people this return is about, so the picture can light them up.
    ///
    /// The World says who each line concerns when it writes the line; nothing
    /// here searches a sentence for names, which would light the wrong person
    /// the first time somebody's name appeared in prose that was not about
    /// them.
    fn lit_selections(&self) -> Vec<SelectionId> {
        let mut lit = Vec::new();
        let mut note = |id: SelectionId| {
            if !lit.contains(&id) {
                lit.push(id);
            }
        };
        if let Some(briefing) = self.snapshot.briefing.as_ref() {
            for item in briefing.beats() {
                for id in &item.concerns {
                    note(*id);
                }
            }
        }
        for command in &self.snapshot.commands {
            for id in &command.concerns {
                note(*id);
            }
        }
        lit
    }

    fn render_canvas(&self, cx: &mut Context<Self>) -> Div {
        let mut canvas = div()
            .relative()
            .h(px(canvas_height(&self.snapshot.canvas.items)))
            .w_full()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xd8d8d8))
            .bg(crate::theme_rgb(0xf1f3ef));

        for item in &self.snapshot.canvas.items {
            let selection = item.id;
            let selected = self.selected == Some(selection);
            let color = match item.kind {
                CanvasItemKind::Place => crate::theme_rgb(0xdde5d8),
                CanvasItemKind::Actor => crate::theme_rgb(0xf4e4c8),
                CanvasItemKind::Object => crate::theme_rgb(0xe2e2e2),
            };
            canvas = canvas.child(
                div()
                    .id(SharedString::from(format!(
                        "canvas-{}",
                        selection.stable_key()
                    )))
                    .absolute()
                    // A share of the frame, not five hundred pixels of it.
                    // The frame is as wide as the window and the positions
                    // were not, so Maple Street drew its whole world into the
                    // left six hundred pixels of an eleven-hundred-pixel box
                    // and left the rest blank.
                    .left(relative(item.x * 0.84))
                    .top(px(CANVAS_TOP + item.y * CANVAS_DEPTH))
                    .w(px(135.0))
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(if selected {
                        crate::theme_rgb(0x4e6fb3)
                    } else {
                        crate::theme_rgb(0xbfc5bd)
                    })
                    .bg(color)
                    .cursor_pointer()
                    .child(div().text_sm().child(item.label.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x666666))
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
        let lead = self
            .snapshot
            .collection
            .items
            .iter()
            .find(|item| item.id == selection)
            .map(|item| item.subtitle.as_str());
        // "Current relations" below is the same list, one card each and each
        // one selectable. Printing the panel's own read-only "Relations ·
        // Trusts · Leo" directly above it said the same relationship twice
        // under two headings.
        let entity_relations = match selection {
            SelectionId::Entity(entity) => self.snapshot.relations_for_entity(entity),
            _ => Vec::new(),
        };
        let skip =
            (!entity_relations.is_empty()).then_some(world_projection::ENTITY_RELATIONS_SECTION);
        let mut panel = inspector_panel(inspector, lead, skip);

        if let SelectionId::Entity(entity) = selection {
            let relations = entity_relations;
            if !relations.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for relation in relations.iter().take(ENTITY_RELATION_LIMIT) {
                    items = items
                        .child(self.entity_relation_item(SelectionId::Relation(*relation), cx));
                }
                // The list explained itself; the paragraph explained the list
                // to whoever wrote it.
                panel = panel
                    .child(div().text_sm().child("Current relations"))
                    .child(items);
                let hidden = relations.len().saturating_sub(ENTITY_RELATION_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x777777))
                            .child(format!("{hidden} more current relations not shown")),
                    );
                }
            }

            let history = self.snapshot.entity_history(entity);
            if !history.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for item in history.iter().take(ENTITY_HISTORY_LIMIT) {
                    items = items.child(self.entity_history_item(item, cx));
                }
                panel = panel
                    .child(div().text_sm().child("Recorded changes to this entity"))
                    .child(items);
                let hidden = history.len().saturating_sub(ENTITY_HISTORY_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x777777))
                            .child(format!("{hidden} more recorded entity changes not shown")),
                    );
                }
            }
        }

        if let SelectionId::Relation(relation) = selection {
            let endpoints = self.snapshot.entities_for_relation(relation);
            if !endpoints.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for entity in endpoints.iter().take(RELATION_ENDPOINT_LIMIT) {
                    items =
                        items.child(self.relation_endpoint_item(SelectionId::Entity(*entity), cx));
                }
                panel = panel
                    .child(div().text_sm().child("Current endpoints"))
                    .child(items);
                let hidden = endpoints.len().saturating_sub(RELATION_ENDPOINT_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x777777))
                            .child(format!("{hidden} more current endpoints not shown")),
                    );
                }
            }

            let history = self.snapshot.relation_history(relation);
            if !history.is_empty() {
                let mut items = div().flex().flex_col().gap_2();
                for item in history.iter().take(RELATION_HISTORY_LIMIT) {
                    items = items.child(self.relation_history_item(item, cx));
                }
                panel = panel
                    .child(div().text_sm().child("Recorded changes to this relation"))
                    .child(items);
                let hidden = history.len().saturating_sub(RELATION_HISTORY_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x777777))
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
                    .child(items);
                let hidden = changed_entities
                    .len()
                    .saturating_sub(EVENT_ENTITY_EFFECT_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x777777))
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
                    .child(items);
                let hidden = changed_relations
                    .len()
                    .saturating_sub(EVENT_RELATION_EFFECT_LIMIT);
                if hidden > 0 {
                    panel = panel.child(
                        div()
                            .text_xs()
                            .text_color(crate::theme_rgb(0x777777))
                            .child(format!(
                                "{hidden} more directly changed relations not shown"
                            )),
                    );
                }
            }
        }

        Some(panel)
    }

    fn entity_relation_item(
        &self,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot
            .inspector(selection)
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Relation".into(), "Active relation".into()));
        div()
            .id(SharedString::from(format!(
                "entity-current-relation-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xe2e4e8))
            .bg(crate::theme_rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(subtitle),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }

    fn relation_endpoint_item(
        &self,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot
            .inspector(selection)
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Entity".into(), "Visible endpoint".into()));
        div()
            .id(SharedString::from(format!(
                "relation-current-endpoint-{}",
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(crate::theme_rgb(0xe2e4e8))
            .bg(crate::theme_rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(subtitle),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
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
            .border_color(crate::theme_rgb(0xe2e4e8))
            .bg(crate::theme_rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(subtitle),
            )
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
            .border_color(crate::theme_rgb(0xe2e4e8))
            .bg(crate::theme_rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(subtitle),
            )
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
            .border_color(crate::theme_rgb(0xe2e4e8))
            .bg(crate::theme_rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x555555))
                    .child(item.subtitle.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777777))
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
            .border_color(crate::theme_rgb(0xe2e4e8))
            .bg(crate::theme_rgb(0xf8f9fc))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x555555))
                    .child(item.subtitle.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777777))
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
            .border_color(crate::theme_rgb(0xd7e2d7))
            .bg(crate::theme_rgb(0xf7fbf7))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x657565))
                    .child("SEMANTIC IMPACT"),
            )
            .child(div().text_lg().child("What this affected"))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x657565))
                    .child(summary),
            );

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
                        .text_color(crate::theme_rgb(0x657565))
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
                .child(div().text_xs().text_color(crate::theme_rgb(0x657565)).child("HOW IT UNFOLDED"))
                .child(
                    div()
                        .text_xs()
                        .text_color(crate::theme_rgb(0x657565))
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
                        .text_color(crate::theme_rgb(0x657565))
                        .child("OTHER WORLD-VISIBLE EFFECTS"),
                )
                .child(other_nodes);
            if other_count > 6 {
                panel = panel.child(
                    div()
                        .text_xs()
                        .text_color(crate::theme_rgb(0x657565))
                        .child(format!("+{} more world-visible effects", other_count - 6)),
                );
            }
        }
        if folded > 0 {
            panel = panel.child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x657565))
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
            .border_color(crate::theme_rgb(0xd7dce8))
            .bg(crate::theme_rgb(0xf7f9fe))
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
                    .bg(crate::theme_rgb(0x263b6a))
                    .text_color(crate::theme_rgb(0xffffff))
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
            .bg(crate::theme_rgb(0xffffff))
            .cursor_pointer()
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x657565))
                    .child(causal_context),
            )
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x4f5f4f))
                    .child(effect.to_string()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777777))
                    .child(event_ref),
            )
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
            .bg(crate::theme_rgb(0xffffff))
            .cursor_pointer()
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x657565))
                    .child(prefix),
            )
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777777))
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
            .bg(crate::theme_rgb(0xffffff))
            .cursor_pointer()
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x65708a))
                    .child(prefix),
            )
            .child(div().text_sm().child(node.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777777))
                    .child(node.subtitle.clone()),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(selection, cx)))
    }
}

impl ProjectionView {
    /// The errand: the place, what happened in it, and what you do now.
    fn render_return(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let mut column = div()
            .id("projection-return-scroll")
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .overflow_y_scroll()
            .flex()
            .flex_col();

        // The World, first, whichever kind of picture it has. A drawn town went
        // at the top and the loose scatter went at the bottom, under every
        // line of the news — so Ares Pocket Colony's return opened with eight
        // paragraphs and ended with forty visible pixels of an empty grey box,
        // the rest of it behind the turn bar. Same place for both.
        if crate::town::is_a_place(&self.snapshot.canvas.items) {
            column = column.child(crate::town::scene(
                &self.snapshot.canvas.items,
                &self.lit_selections(),
            ));
        } else if !self.snapshot.canvas.items.is_empty() {
            column = column.child(div().px_5().pt_4().child(self.render_canvas(cx)));
        }
        // The line is a summary of the World's whole life, so it belongs with
        // the picture. Under the news it was below the fold on any visit with
        // more than four things to report, which is every visit worth making.
        if crate::fortune_line::has_a_shape(&self.snapshot) {
            column = column.child(crate::fortune_line::line(&self.snapshot));
        }
        if let Some(news) = self.render_news(cx) {
            column = column.child(news);
        }
        column.into_any_element()
    }

    /// Everything else, on purpose: the cast, the history, and whatever is
    /// selected in either.
    fn render_reference(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let mut detail = div()
            .id("projection-reference-scroll")
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .p_3();
        if let Some(inspector) = self.render_inspector(cx) {
            detail = detail.child(inspector);
        }
        if let Some(why) = self.render_why(cx) {
            detail = detail.child(why);
        }
        if let Some(influence) = self.render_influence(cx) {
            detail = detail.child(influence);
        }

        let mut panels = div()
            .flex_1()
            .min_h(px(0.0))
            .w_full()
            .min_w(px(0.0))
            .overflow_hidden()
            .flex();
        if has_collection_panel(&self.snapshot) {
            panels = panels.child(self.render_collection(cx));
        }
        panels = panels.child(detail);
        if has_timeline_panel(&self.snapshot) {
            panels = panels.child(self.render_timeline(cx));
        }
        panels.into_any_element()
    }

    fn surface_tab(
        &self,
        surface: Surface,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let here = self.surface == surface;
        div()
            .id(SharedString::from(label))
            .px_3()
            .py_1()
            .rounded_md()
            .cursor_pointer()
            .text_sm()
            .bg(if here {
                crate::theme_rgb(0xe7eefc)
            } else {
                crate::theme_rgb(0xfcfcfa)
            })
            .text_color(if here {
                crate::theme_rgb(0x27446f)
            } else {
                crate::theme_rgb(0x6b665e)
            })
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.surface = surface;
                cx.notify();
            }))
    }
}

impl Render for ProjectionView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            _window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));

        // When, in the World's own words. The newest entry knows; the header
        // is the one place it is worth saying once.
        let when = self
            .snapshot
            .timeline
            .items
            .first()
            .and_then(|item| item.when.clone());

        let mut header_right = div().flex().items_center().gap_2();
        if let Some(when) = when {
            header_right = header_right.child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x6b665e))
                    .child(when),
            );
        }
        if let Some(status) = &self.status {
            header_right = header_right.child(
                div()
                    .text_sm()
                    .text_color(if self.status_is_error {
                        crate::theme_rgb(0xa33a3a)
                    } else {
                        crate::theme_rgb(0x4e6fb3)
                    })
                    .child(status.clone()),
            );
        }
        header_right = header_right
            .child(self.surface_tab(Surface::Return, "Return", cx))
            .child(self.surface_tab(Surface::Reference, "The world", cx));

        let body = match self.surface {
            Surface::Return => self.render_return(cx),
            Surface::Reference => self.render_reference(cx),
        };

        div()
            .size_full()
            .bg(crate::theme_rgb(0xfcfcfa))
            .text_color(crate::theme_rgb(0x202020))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(48.0))
                    .w_full()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_5()
                    .border_b_1()
                    .border_color(crate::theme_rgb(0xdadada))
                    .child(div().text_lg().child(self.snapshot.title.clone()))
                    .child(header_right),
            )
            .child(body)
            .child(self.render_turn(cx))
    }
}

fn has_collection_panel(snapshot: &ProjectionSnapshot) -> bool {
    !snapshot.collection.items.is_empty()
}

fn has_timeline_panel(snapshot: &ProjectionSnapshot) -> bool {
    !snapshot.timeline.items.is_empty()
}

/// The first sentence of what a choice does.
///
/// A Pack's `detail` is written for a panel with room: three of them at full
/// length made the bar 180 pixels tall and sliced the news in half at the top
/// of the window. The first sentence is the part that says what the choice
/// *is* — "Invest 120 of Mara\'s cash to reopen Harbor Bakery", "Let one more
/// cycle pass without answering" — and the rest is terms and conditions, which
/// the briefing above is already showing in full.
fn choice_gist(detail: &str) -> String {
    const CAP: usize = 120;
    let detail = detail.trim();
    let sentence = match detail.find(". ") {
        Some(end) => &detail[..=end],
        None => detail,
    };
    let sentence = sentence.trim();
    if sentence.chars().count() <= CAP {
        return sentence.to_owned();
    }
    // A Pack with one very long sentence still has to fit. Cut on a word so
    // the tail is a word rather than half of one.
    let cut = sentence
        .char_indices()
        .take(CAP)
        .last()
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let cut = sentence[..cut].rfind(' ').unwrap_or(cut);
    format!("{}…", sentence[..cut].trim_end())
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

/// How much of the frame the loose scatter is drawn into. Items are placed at
/// a fraction of this.
const CANVAS_DEPTH: f32 = 260.0;
/// What the scatter sits inside, from the top of the frame to the bottom of
/// the lowest box.
const CANVAS_TOP: f32 = 12.0;
const CANVAS_BOX: f32 = 74.0;

/// As tall as it needs to be, and no taller.
///
/// The frame was a flat 330 pixels whatever it held. A six-thing World fills
/// two rows of it, so the picture came with ninety pixels of empty grey
/// underneath — and once the picture moved to the top of a return, those
/// ninety pixels pushed six of Ares Pocket Colony's eight lines of news below
/// the fold. An empty frame keeps a floor so it still reads as a frame.
fn canvas_height(items: &[world_projection::CanvasItem]) -> f32 {
    let depth = items
        .iter()
        .map(|item| item.y)
        .fold(0.0_f32, f32::max)
        .clamp(0.0, 1.0);
    (CANVAS_TOP + depth * CANVAS_DEPTH + CANVAS_BOX).clamp(120.0, 330.0)
}

/// The panel, opening with the World's own sentence about this thing when it
/// has one.
///
/// The panel is a table of every component an entity carries: "Cash 7",
/// "Hardship Status destitute", "Income Status lost", "Job unemployed",
/// "Loan Status requested" — nine rows of schema for a man the list beside it
/// describes, in the Pack's own words, as "Out of work · 7 coins". The Pack
/// already wrote that sentence for the collection; the panel says it first and
/// keeps the table under it.
fn inspector_panel(inspector: &InspectorProjection, lead: Option<&str>, skip: Option<&str>) -> Div {
    let mut body = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_lg().child(inspector.title.clone()))
        .child(
            div()
                .text_xs()
                .text_color(crate::theme_rgb(0x666666))
                .child(inspector.subtitle.clone()),
        );

    if let Some(lead) = lead.map(str::trim).filter(|lead| !lead.is_empty()) {
        body = body.child(div().text_sm().child(lead.to_owned()));
    }

    for section in inspector
        .display_sections()
        .filter(|section| Some(section.title.as_str()) != skip)
    {
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
                            .text_color(crate::theme_rgb(0x777777))
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
        .border_color(crate::theme_rgb(0xdadada))
        .bg(crate::theme_rgb(0xffffff))
        .flex()
        .flex_col()
        .gap_3()
        .child(body)
}

#[cfg(test)]
mod focus_hierarchy_tests {
    use super::{
        canvas_height, choice_gist, command_panel_title, default_selection, has_collection_panel,
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
            when: None,
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
    }

    #[test]
    fn semantic_content_restores_navigation_and_exploration() {
        let snapshot = snapshot_with_entity_and_event();
        assert!(has_collection_panel(&snapshot));
        assert!(has_timeline_panel(&snapshot));
    }

    #[test]
    fn command_panel_distinguishes_continuation_from_choice() {
        assert_eq!(command_panel_title(1), "Continue");
        assert_eq!(command_panel_title(2), "Choose what happens next");
        assert_eq!(command_panel_title(5), "Choose what happens next");
    }

    #[test]
    fn a_choice_shows_what_it_is_not_its_terms_and_conditions() {
        assert_eq!(
            choice_gist(
                "Invest 120 of Mara's cash to reopen Harbor Bakery. Mara returns to work; \
                 former workers are not automatically rehired."
            ),
            "Invest 120 of Mara's cash to reopen Harbor Bakery."
        );
        assert_eq!(
            choice_gist("Let one more cycle pass without answering. The World will not wait."),
            "Let one more cycle pass without answering."
        );
        // One sentence and no full stop at all is left alone when it fits.
        assert_eq!(choice_gist("Hold the line"), "Hold the line");
    }

    #[test]
    fn one_very_long_sentence_is_cut_on_a_word() {
        let gist = choice_gist(&"alpha ".repeat(60));
        assert!(gist.ends_with('…'), "{gist}");
        assert!(
            gist.chars().count() <= 121,
            "{} chars",
            gist.chars().count()
        );
        assert!(!gist.contains("alph…"), "cut mid-word: {gist}");
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
            when: None,
            title: "Changed".into(),
            subtitle: String::new(),
            caused_by: Vec::new(),
        });
        snapshot.inspectors.insert(event, inspector(event));
        assert_eq!(default_selection(&snapshot), Some(event));
    }

    #[test]
    fn the_loose_scatter_is_as_tall_as_what_it_holds() {
        use world_projection::{CanvasItem, CanvasItemKind, CanvasItemState};

        let thing = |y: f32| CanvasItem {
            id: SelectionId::Entity(Default::default()),
            kind: CanvasItemKind::Object,
            label: "Thing".into(),
            detail: String::new(),
            x: 0.0,
            y,
            at: None,
            state: CanvasItemState::Working,
        };

        // Two rows of a six-thing World reach 0.615 down the frame. The frame
        // used to be a flat 330 whatever it held, so this came with ninety
        // pixels of empty grey under it.
        let two_rows = canvas_height(&[thing(0.205), thing(0.615)]);
        assert!(
            (240.0..260.0).contains(&two_rows),
            "two rows asked for {two_rows}px"
        );
        assert!(
            canvas_height(&[thing(0.205)]) < two_rows,
            "one row must not ask for as much as two"
        );

        // A frame is still a frame when it is empty, and never taller than the
        // page it sits on.
        assert_eq!(canvas_height(&[]), 120.0);
        assert_eq!(canvas_height(&[thing(4.0)]), 330.0);
    }
}
