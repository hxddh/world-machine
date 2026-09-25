use gpui::{div, prelude::*, px, Context, Div, Render, SharedString, Styled, Window};
use std::rc::Rc;
use world_compare::{
    compare_divergence, compare_evidence_neighborhoods, ChangedCommand, ChangedTimelineItem,
    DifferenceKind, DivergenceImpactStage, DivergenceSide, EntityDifference,
    EvidenceNeighborhoodComparison, EvidenceNeighborhoodNodeDifference, RelationDifference,
    SnapshotComparison, SnapshotDivergence,
};
use world_gpui::{scene, ui};
use world_projection::{
    InspectorProjection, ProjectionCommand, ProjectionSnapshot, RelationEndpointRole, SelectionId,
    StateEvidenceEdge, TimelineItem, WhyNode,
};
use world_strategy::{StrategyEvaluation, StrategyRun};
use world_theme::tokens;

const ENTITY_DIFFERENCE_LIMIT: usize = 10;
const RELATION_DIFFERENCE_LIMIT: usize = 10;
const TIMELINE_DIFFERENCE_LIMIT_PER_KIND: usize = 4;
const INSPECTOR_ROW_LIMIT: usize = 6;
const DIVERGENCE_IMPACT_LIMIT: usize = 4;
const EVIDENCE_CAUSE_LIMIT: usize = 6;
const ENTITY_HISTORY_LIMIT: usize = 6;
const RELATION_HISTORY_LIMIT: usize = 6;
const ENTITY_RELATION_LIMIT: usize = 6;
const RELATION_ENDPOINT_LIMIT: usize = 6;
const EVENT_ENTITY_EFFECT_LIMIT: usize = 6;
const EVENT_RELATION_EFFECT_LIMIT: usize = 6;
const LOCAL_EVIDENCE_DEPTH: usize = 2;
const LOCAL_EVIDENCE_NODE_LIMIT: usize = 8;
const LOCAL_EVIDENCE_EDGE_LIMIT_PER_SIDE: usize = 6;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SavedComparisonContext {
    pub relation: Option<String>,
    pub left_provenance: Option<String>,
    pub right_provenance: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComparisonSide {
    Left,
    Right,
}

impl ComparisonSide {
    fn key(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ComparisonSelection {
    side: ComparisonSide,
    selection: SelectionId,
}

// One per window, so the size of the larger variant costs nothing worth
// an extra indirection on every access.
#[allow(clippy::large_enum_variant)]
enum ComparisonSource {
    Strategies(StrategyEvaluation),
    Saved {
        left: ProjectionSnapshot,
        right: ProjectionSnapshot,
        comparison: SnapshotComparison,
        context: SavedComparisonContext,
    },
}

pub struct StrategyComparisonView {
    source: ComparisonSource,
    left_label: String,
    right_label: String,
    selected: Option<ComparisonSelection>,
}

impl StrategyComparisonView {
    pub fn new(
        evaluation: StrategyEvaluation,
        left_label: impl Into<String>,
        right_label: impl Into<String>,
    ) -> Self {
        Self {
            source: ComparisonSource::Strategies(evaluation),
            left_label: left_label.into(),
            right_label: right_label.into(),
            selected: None,
        }
    }

    pub fn saved(
        left: ProjectionSnapshot,
        right: ProjectionSnapshot,
        comparison: SnapshotComparison,
        left_label: impl Into<String>,
        right_label: impl Into<String>,
    ) -> Self {
        Self::saved_with_context(
            left,
            right,
            comparison,
            left_label,
            right_label,
            SavedComparisonContext::default(),
        )
    }

    pub fn saved_with_context(
        left: ProjectionSnapshot,
        right: ProjectionSnapshot,
        comparison: SnapshotComparison,
        left_label: impl Into<String>,
        right_label: impl Into<String>,
        context: SavedComparisonContext,
    ) -> Self {
        Self {
            source: ComparisonSource::Saved {
                left,
                right,
                comparison,
                context,
            },
            left_label: left_label.into(),
            right_label: right_label.into(),
            selected: None,
        }
    }

    fn snapshot(&self, side: ComparisonSide) -> Option<&ProjectionSnapshot> {
        match (&self.source, side) {
            (ComparisonSource::Strategies(evaluation), ComparisonSide::Left) => {
                evaluation.left.outcome().map(|outcome| &outcome.snapshot)
            }
            (ComparisonSource::Strategies(evaluation), ComparisonSide::Right) => {
                evaluation.right.outcome().map(|outcome| &outcome.snapshot)
            }
            (ComparisonSource::Saved { left, .. }, ComparisonSide::Left) => Some(left),
            (ComparisonSource::Saved { right, .. }, ComparisonSide::Right) => Some(right),
        }
    }

    fn side_label(&self, side: ComparisonSide) -> &str {
        match side {
            ComparisonSide::Left => &self.left_label,
            ComparisonSide::Right => &self.right_label,
        }
    }

    fn select(&mut self, side: ComparisonSide, selection: SelectionId, cx: &mut Context<Self>) {
        let selectable = self
            .snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .is_some();
        if selectable {
            self.selected = Some(ComparisonSelection { side, selection });
            cx.notify();
        }
    }

    fn is_selected(&self, side: ComparisonSide, selection: SelectionId) -> bool {
        self.selected == Some(ComparisonSelection { side, selection })
    }

    /// One future as a picture: the choice that led to it, how its World
    /// sums itself up at the end, its scene, and the last things that
    /// happened in it.
    fn render_future(
        &self,
        side: ComparisonSide,
        eyebrow: &str,
        width: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let label = self.side_label(side).to_string();
        let card = div()
            .flex_1()
            .min_w(px(0.0))
            .p_4()
            .rounded_xl()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .flex()
            .flex_col()
            .gap_3();
        let Some(snapshot) = self.snapshot(side) else {
            return card
                .border_color(ui::color(tokens::DANGER))
                .child(ui::section_label(eyebrow.to_string()))
                .child(ui::heading(label))
                .child(
                    ui::body("This future could not be played out.")
                        .text_color(ui::color(tokens::DANGER)),
                );
        };
        let other_side = match side {
            ComparisonSide::Left => ComparisonSide::Right,
            ComparisonSide::Right => ComparisonSide::Left,
        };
        let beats = snapshot
            .briefing
            .as_ref()
            .map(|briefing| briefing.beats())
            .unwrap_or_default();
        // The headline is the first thing that happened here and not in the
        // other future; a briefing's own title ("While you were away") is
        // the same on both sides and says nothing about the choice.
        let also_there = self
            .snapshot(other_side)
            .and_then(|other| other.briefing.as_ref())
            .map(|briefing| {
                briefing
                    .beats()
                    .iter()
                    .map(|beat| beat.title.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let lead = beats
            .iter()
            .position(|beat| !also_there.contains(&beat.title))
            .or(if beats.is_empty() { None } else { Some(0) });
        let headline = lead
            .map(|index| beats[index].title.clone())
            .unwrap_or_else(|| snapshot.title.clone());
        let mut card = card
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::section_label(eyebrow.to_string()))
                    .child(ui::heading(label)),
            )
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap_3()
                    .child(ui::page_title(headline))
                    .child(ui::caption(snapshot.moment_label(snapshot.world_time))),
            );
        let view = cx.entity().downgrade();
        let on_select: scene::SelectHandler = Rc::new(move |selection, _, cx| {
            view.update(cx, |this, cx| this.select(side, selection, cx))
                .ok();
        });
        let selected = self
            .selected
            .filter(|selected| selected.side == side)
            .map(|selected| selected.selection);
        // Point at what is different from the other future, not at the news.
        let emphasis = match self.snapshot(other_side) {
            Some(other) => scene::Emphasis::Only(scene::differences(snapshot, other)),
            None => scene::Emphasis::News,
        };
        if let Some(stage) = scene::scene(snapshot, width, selected, &emphasis, on_select) {
            card = card.child(stage);
        }
        let rest = beats
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) != lead)
            .map(|(_, beat)| *beat)
            .take(3)
            .collect::<Vec<_>>();
        if !rest.is_empty() {
            let mut ending = div()
                .flex()
                .flex_col()
                .gap_2()
                .child(ui::section_label("How it ends"));
            for beat in rest {
                let face = match beat
                    .selection
                    .and_then(|selection| scene::event_actor(snapshot, selection))
                {
                    Some(name) => ui::avatar(&name, 22.0),
                    None => div()
                        .size(px(22.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .size(px(6.0))
                                .rounded_full()
                                .bg(ui::color(tokens::ACCENT)),
                        ),
                };
                ending = ending.child(
                    div().flex().gap_2().items_start().child(face).child(
                        div()
                            .min_w(px(0.0))
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(ui::row_title(beat.title.clone()))
                            .child(
                                ui::detail(beat.detail.clone())
                                    .line_clamp(2)
                                    .text_ellipsis(),
                            ),
                    ),
                );
            }
            card = card.child(ending);
        }
        card
    }

    fn divergence(&self) -> Option<SnapshotDivergence> {
        match &self.source {
            ComparisonSource::Strategies(evaluation) => {
                match (&evaluation.left, &evaluation.right) {
                    (StrategyRun::Success(left), StrategyRun::Success(right)) => {
                        compare_divergence(&left.snapshot, &right.snapshot)
                    }
                    _ => None,
                }
            }
            ComparisonSource::Saved { left, right, .. } => compare_divergence(left, right),
        }
    }

    fn render_divergence(&self, divergence: &SnapshotDivergence, cx: &mut Context<Self>) -> Div {
        let shared = match &divergence.shared_frontier {
            Some(frontier) => div()
                .p_3()
                .rounded_md()
                .bg(ui::color(tokens::SURFACE))
                .border_1()
                .border_color(ui::color(tokens::BORDER))
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(ui::color(tokens::TEXT_SECONDARY))
                        .child("Last moment they share"),
                )
                .child(div().text_sm().child(frontier.title.clone()))
                .child(
                    div()
                        .text_xs()
                        .text_color(ui::color(tokens::TEXT_SECONDARY))
                        .child(frontier.subtitle.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(ui::color(tokens::TEXT_TERTIARY))
                        .child(self.moment(ComparisonSide::Left, frontier.world_time)),
                ),
            None => div()
                .p_3()
                .rounded_md()
                .bg(ui::color(tokens::SURFACE))
                .border_1()
                .border_color(ui::color(tokens::BORDER))
                .text_sm()
                .child("These two share no history."),
        };

        div()
            .w_full()
            .p_4()
            .rounded_xl()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .flex()
            .flex_col()
            .gap_3()
            .child(ui::heading("Where they split"))
            .child(ui::detail(
                "Everything up to the last shared moment happened in both. Below it, the first thing that went differently in each, and what followed from it. Select a moment to see why.",
            ))
            .child(shared)
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(self.render_divergence_side(
                        &self.left_label,
                        ComparisonSide::Left,
                        &divergence.left,
                        cx,
                    ))
                    .child(self.render_divergence_side(
                        &self.right_label,
                        ComparisonSide::Right,
                        &divergence.right,
                        cx,
                    )),
            )
    }

    fn render_divergence_side(
        &self,
        label: &str,
        comparison_side: ComparisonSide,
        side: &DivergenceSide,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut column = div()
            .flex_1()
            .min_w(px(0.0))
            .p_3()
            .rounded_md()
            .bg(ui::color(tokens::SURFACE))
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(label.to_string()),
            );

        if let Some(first) = &side.first_difference {
            let selection = first.id;
            let selected = self.is_selected(comparison_side, selection);
            column = column.child(
                div()
                    .id(SharedString::from(format!(
                        "divergence-first-{}-{}",
                        comparison_side.key(),
                        selection.stable_key()
                    )))
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(if selected {
                        ui::color(tokens::ACCENT)
                    } else {
                        ui::color(tokens::BORDER)
                    })
                    .bg(if selected {
                        ui::color(tokens::ROW_SELECTED)
                    } else {
                        ui::color(tokens::WINDOW)
                    })
                    .cursor_pointer()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child("First thing that went differently"),
                    )
                    .child(div().text_sm().child(first.title.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(first.subtitle.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child(self.moment(comparison_side, first.world_time)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select(comparison_side, selection, cx)
                    })),
            );
        } else {
            column = column.child(
                div()
                    .text_sm()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child("This side stops at the shared frontier."),
            );
            return column;
        }

        column = column.child(
            div()
                .text_xs()
                .text_color(ui::color(tokens::TEXT_SECONDARY))
                .child("What followed"),
        );
        if side.impact.is_empty() {
            return column.child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child("Nothing else has come of it yet."),
            );
        }

        for (index, stage) in side.impact.iter().take(DIVERGENCE_IMPACT_LIMIT).enumerate() {
            column =
                column.child(self.render_divergence_stage(comparison_side, stage, index == 0, cx));
        }
        if let Some(notice) = hidden_notice(
            side.impact.len(),
            DIVERGENCE_IMPACT_LIMIT,
            "later impact stages",
        ) {
            column = column.child(truncation_notice(notice));
        }
        column
    }

    fn render_divergence_stage(
        &self,
        comparison_side: ComparisonSide,
        stage: &DivergenceImpactStage,
        first: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let causal_context = format!(
            "{} · time {}",
            if first { "Because of it" } else { "Then" },
            stage.event.world_time
        );
        let effect = world_projection::effect_headline(&stage.effect).to_string();
        let selection = stage.event.id;
        let selected = self.is_selected(comparison_side, selection);
        div()
            .id(SharedString::from(format!(
                "divergence-impact-{}-{}",
                comparison_side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(if selected {
                ui::color(tokens::ROW_SELECTED)
            } else {
                ui::color(tokens::WINDOW)
            })
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(causal_context),
            )
            .child(div().text_sm().child(stage.event.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(effect),
            )
            .on_click(
                cx.listener(move |this, _, _, cx| this.select(comparison_side, selection, cx)),
            )
    }

    fn render_selected_evidence(&self, cx: &mut Context<Self>) -> Option<Div> {
        let selected = self.selected?;
        let snapshot = self.snapshot(selected.side)?;
        let inspector = snapshot.inspector(selected.selection)?;
        let label = self.side_label(selected.side).to_string();

        let mut panel = div()
            .w(px(700.0))
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER_STRONG))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(format!("RECORDED EVIDENCE · {label}")),
            )
            .child(div().text_lg().child(inspector.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(inspector.subtitle.clone()),
            )
            .child(self.render_evidence_inspector(inspector));

        if let Some(local) = self
            .local_evidence_comparison(selected.selection)
            .filter(|comparison| !comparison.is_identical())
        {
            panel = panel.child(self.render_local_evidence_divergence(&local, cx));
        }

        if let SelectionId::Entity(entity) = selected.selection {
            let relations = snapshot.relations_for_entity(entity);
            if !relations.is_empty() {
                let mut relation_list = div().flex().flex_col().gap_2();
                for relation in relations.iter().take(ENTITY_RELATION_LIMIT) {
                    relation_list = relation_list.child(self.render_entity_current_relation(
                        selected.side,
                        SelectionId::Relation(*relation),
                        cx,
                    ));
                }
                if let Some(notice) =
                    hidden_notice(relations.len(), ENTITY_RELATION_LIMIT, "current relations")
                {
                    relation_list = relation_list.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("Current relations"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child("Who and what it is connected to in this future."),
                    )
                    .child(relation_list);
            }

            let history = snapshot.entity_history(entity);
            if !history.is_empty() {
                let mut history_list = div().flex().flex_col().gap_2();
                for item in history.iter().take(ENTITY_HISTORY_LIMIT) {
                    history_list = history_list.child(self.render_entity_history_event(
                        selected.side,
                        item,
                        cx,
                    ));
                }
                if let Some(notice) = hidden_notice(history.len(), ENTITY_HISTORY_LIMIT, "changes")
                {
                    history_list = history_list.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("What changed it"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child("The moments that changed it in this future. Select one to see why it happened."),
                    )
                    .child(history_list);
            }
        }

        if let SelectionId::Relation(relation) = selected.selection {
            let endpoints = snapshot.entities_for_relation(relation);
            if !endpoints.is_empty() {
                let mut endpoint_list = div().flex().flex_col().gap_2();
                for entity in endpoints.iter().take(RELATION_ENDPOINT_LIMIT) {
                    endpoint_list = endpoint_list.child(self.render_relation_current_endpoint(
                        selected.side,
                        SelectionId::Entity(*entity),
                        cx,
                    ));
                }
                if let Some(notice) = hidden_notice(
                    endpoints.len(),
                    RELATION_ENDPOINT_LIMIT,
                    "current relation endpoints",
                ) {
                    endpoint_list = endpoint_list.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("Current endpoints"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child("The two it connects in this future."),
                    )
                    .child(endpoint_list);
            }

            let history = snapshot.relation_history(relation);
            if !history.is_empty() {
                let mut history_list = div().flex().flex_col().gap_2();
                for item in history.iter().take(RELATION_HISTORY_LIMIT) {
                    history_list = history_list.child(self.render_relation_history_event(
                        selected.side,
                        item,
                        cx,
                    ));
                }
                if let Some(notice) =
                    hidden_notice(history.len(), RELATION_HISTORY_LIMIT, "changes")
                {
                    history_list = history_list.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("How it changed"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child("The moments that made, changed, or ended it in this future."),
                    )
                    .child(history_list);
            }
        }

        if let SelectionId::Event(event) = selected.selection {
            let changed_entities = snapshot.directly_changed_entities(event);
            if !changed_entities.is_empty() {
                let mut entities = div().flex().flex_col().gap_2();
                for entity in changed_entities.iter().take(EVENT_ENTITY_EFFECT_LIMIT) {
                    entities = entities.child(self.render_event_entity_effect(
                        selected.side,
                        SelectionId::Entity(*entity),
                        cx,
                    ));
                }
                if let Some(notice) = hidden_notice(
                    changed_entities.len(),
                    EVENT_ENTITY_EFFECT_LIMIT,
                    "directly changed entities",
                ) {
                    entities = entities.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("Entities changed by this event"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child("What this moment changed in this future."),
                    )
                    .child(entities);
            }

            let changed_relations = snapshot.directly_changed_relations(event);
            if !changed_relations.is_empty() {
                let mut relations = div().flex().flex_col().gap_2();
                for relation in changed_relations.iter().take(EVENT_RELATION_EFFECT_LIMIT) {
                    relations = relations.child(self.render_event_relation_effect(
                        selected.side,
                        SelectionId::Relation(*relation),
                        cx,
                    ));
                }
                if let Some(notice) = hidden_notice(
                    changed_relations.len(),
                    EVENT_RELATION_EFFECT_LIMIT,
                    "directly changed relations",
                ) {
                    relations = relations.child(truncation_notice(notice));
                }
                panel = panel
                    .child(div().text_sm().child("Relations changed by this event"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child("The connections this moment made, changed, or ended."),
                    )
                    .child(relations);
            }

            let mut causes = div().flex().flex_col().gap_2();
            if let Some(why) = snapshot.why(event) {
                let cause_nodes = why.nodes.iter().skip(1).collect::<Vec<_>>();
                if cause_nodes.is_empty() {
                    causes = causes.child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child("Nothing earlier led to it."),
                    );
                } else {
                    for node in cause_nodes.iter().take(EVIDENCE_CAUSE_LIMIT) {
                        causes = causes.child(self.render_evidence_cause(selected.side, node, cx));
                    }
                    if let Some(notice) =
                        hidden_notice(cause_nodes.len(), EVIDENCE_CAUSE_LIMIT, "earlier causes")
                    {
                        causes = causes.child(truncation_notice(notice));
                    }
                }
            } else {
                causes = causes.child(
                    div()
                        .text_xs()
                        .text_color(ui::color(tokens::TEXT_TERTIARY))
                        .child("There is no recorded reason for this moment."),
                );
            }

            panel = panel
                .child(div().text_sm().child("Why this happened"))
                .child(
                    div()
                        .text_xs()
                        .text_color(ui::color(tokens::TEXT_SECONDARY))
                        .child("Persisted caused_by history from the selected future. Select a cause to continue tracing on the same side."),
                )
                .child(causes);
        }

        Some(panel)
    }

    fn render_event_entity_effect(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Something".into(), String::new()));
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "event-entity-effect-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(ui::color(tokens::SURFACE))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(subtitle),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }
    fn render_entity_current_relation(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Relation".into(), "Active relation".into()));
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "entity-current-relation-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(ui::color(tokens::SURFACE))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(subtitle),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn render_relation_current_endpoint(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Entity".into(), "Visible endpoint".into()));
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "relation-current-endpoint-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(ui::color(tokens::SURFACE))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(subtitle),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn render_event_relation_effect(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (title, subtitle) = self
            .snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .map(|inspector| (inspector.title.clone(), inspector.subtitle.clone()))
            .unwrap_or_else(|| ("Relation".into(), "Recorded relation".into()));
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "event-relation-effect-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(ui::color(tokens::SURFACE))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(subtitle),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn render_entity_history_event(
        &self,
        side: ComparisonSide,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "entity-history-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(ui::color(tokens::SURFACE))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(item.subtitle.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child(self.moment(side, item.world_time)),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }
    fn render_relation_history_event(
        &self,
        side: ComparisonSide,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "relation-history-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(ui::color(tokens::SURFACE))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().child(item.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(item.subtitle.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child(self.moment(side, item.world_time)),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn local_evidence_comparison(
        &self,
        selection: SelectionId,
    ) -> Option<EvidenceNeighborhoodComparison> {
        let left = self.snapshot(ComparisonSide::Left)?;
        let right = self.snapshot(ComparisonSide::Right)?;
        compare_evidence_neighborhoods(left, right, selection, LOCAL_EVIDENCE_DEPTH)
    }

    fn render_local_evidence_divergence(
        &self,
        comparison: &EvidenceNeighborhoodComparison,
        cx: &mut Context<Self>,
    ) -> Div {
        let node_count = comparison.nodes.len();
        let edge_count = comparison.edges.left_only.len() + comparison.edges.right_only.len();
        let mut section = div()
            .mt_2()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_sm().child("Around it, how the futures differ"))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(format!(
                        "Within {} hops of this selection: {} node-distance changes and {} typed edge changes between futures.",
                        comparison.max_depth, node_count, edge_count
                    )),
            );

        if !comparison.nodes.is_empty() {
            let mut nodes = div().flex().flex_col().gap_2().child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child("Closer or further apart"),
            );
            for node in comparison.nodes.iter().take(LOCAL_EVIDENCE_NODE_LIMIT) {
                nodes = nodes.child(self.render_local_evidence_node(node, cx));
            }
            if let Some(notice) = hidden_notice(
                comparison.nodes.len(),
                LOCAL_EVIDENCE_NODE_LIMIT,
                "nearby things",
            ) {
                nodes = nodes.child(truncation_notice(notice));
            }
            section = section.child(nodes);
        }

        if !comparison.edges.is_empty() {
            section = section.child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child("Connections that differ"),
            );
            section = section.child(
                div()
                    .flex()
                    .gap_2()
                    .child(self.render_local_evidence_edge_side(
                        ComparisonSide::Left,
                        &comparison.edges.left_only,
                    ))
                    .child(self.render_local_evidence_edge_side(
                        ComparisonSide::Right,
                        &comparison.edges.right_only,
                    )),
            );
        }

        section
    }

    fn render_local_evidence_node(
        &self,
        node: &EvidenceNeighborhoodNodeDifference,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut row = div().flex().gap_2();
        if let Some(depth) = node.left_depth {
            row = row.child(self.render_local_evidence_node_side(
                ComparisonSide::Left,
                node.selection,
                depth,
                cx,
            ));
        } else {
            row = row.child(self.render_local_evidence_absent_side(ComparisonSide::Left));
        }
        if let Some(depth) = node.right_depth {
            row = row.child(self.render_local_evidence_node_side(
                ComparisonSide::Right,
                node.selection,
                depth,
                cx,
            ));
        } else {
            row = row.child(self.render_local_evidence_absent_side(ComparisonSide::Right));
        }
        row
    }

    fn render_local_evidence_node_side(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let title = self.selection_title(side, selection);
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "local-evidence-node-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .w(px(320.0))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(if selected {
                ui::color(tokens::ROW_SELECTED)
            } else {
                ui::color(tokens::WINDOW)
            })
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(format!(
                        "{} · {} hop{}",
                        self.side_label(side),
                        depth,
                        if depth == 1 { "" } else { "s" }
                    )),
            )
            .child(div().text_sm().child(title))
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn render_local_evidence_absent_side(&self, side: ComparisonSide) -> Div {
        div()
            .w(px(320.0))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .text_xs()
            .text_color(ui::color(tokens::TEXT_TERTIARY))
            .child(format!(
                "{} · outside this neighborhood",
                self.side_label(side)
            ))
    }

    fn render_local_evidence_edge_side(
        &self,
        side: ComparisonSide,
        edges: &[StateEvidenceEdge],
    ) -> Div {
        let mut column = div()
            .w(px(320.0))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(self.side_label(side).to_string()),
            );
        if edges.is_empty() {
            return column.child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child("No side-only typed edges"),
            );
        }
        for edge in edges.iter().take(LOCAL_EVIDENCE_EDGE_LIMIT_PER_SIDE) {
            column = column.child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(self.local_evidence_edge_label(side, *edge)),
            );
        }
        if let Some(notice) = hidden_notice(
            edges.len(),
            LOCAL_EVIDENCE_EDGE_LIMIT_PER_SIDE,
            "typed edges",
        ) {
            column = column.child(truncation_notice(notice));
        }
        column
    }

    fn local_evidence_edge_label(&self, side: ComparisonSide, edge: StateEvidenceEdge) -> String {
        match edge {
            StateEvidenceEdge::EntityEvent(evidence) => format!(
                "Changed: {} ↔ {}",
                self.selection_title(side, SelectionId::Entity(evidence.entity)),
                self.selection_title(side, SelectionId::Event(evidence.event)),
            ),
            StateEvidenceEdge::RelationEvent(evidence) => format!(
                "Recorded relation change: {} ↔ {}",
                self.selection_title(side, SelectionId::Relation(evidence.relation)),
                self.selection_title(side, SelectionId::Event(evidence.event)),
            ),
            StateEvidenceEdge::EntityRelation(evidence) => {
                let role = match evidence.role {
                    RelationEndpointRole::From => "From endpoint",
                    RelationEndpointRole::To => "To endpoint",
                };
                format!(
                    "{role}: {} ↔ {}",
                    self.selection_title(side, SelectionId::Entity(evidence.entity)),
                    self.selection_title(side, SelectionId::Relation(evidence.relation)),
                )
            }
        }
    }

    fn selection_title(&self, side: ComparisonSide, selection: SelectionId) -> String {
        self.snapshot(side)
            .and_then(|snapshot| snapshot.inspector(selection))
            .map(|inspector| inspector.title.clone())
            .unwrap_or_else(|| selection.stable_key())
    }

    fn render_evidence_inspector(&self, inspector: &InspectorProjection) -> Div {
        let mut sections = div().flex().flex_col().gap_3();
        for section in inspector.display_sections() {
            let mut rows = div().flex().flex_col().gap_1();
            for row in section.rows.iter().take(INSPECTOR_ROW_LIMIT) {
                rows = rows.child(
                    div()
                        .flex()
                        .gap_2()
                        .text_xs()
                        .child(
                            div()
                                .w(px(180.0))
                                .text_color(ui::color(tokens::TEXT_SECONDARY))
                                .child(row.label.clone()),
                        )
                        .child(div().flex_1().child(row.value.clone())),
                );
            }
            if let Some(notice) =
                hidden_notice(section.rows.len(), INSPECTOR_ROW_LIMIT, "inspector rows")
            {
                rows = rows.child(truncation_notice(notice));
            }
            sections = sections.child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(ui::color(tokens::SURFACE))
                    .border_1()
                    .border_color(ui::color(tokens::BORDER))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_sm().child(section.title.clone()))
                    .child(rows),
            );
        }
        sections
    }

    fn render_evidence_cause(
        &self,
        side: ComparisonSide,
        node: &WhyNode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = SelectionId::Event(node.event);
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "evidence-cause-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(ui::color(tokens::SURFACE))
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(format!("{} steps earlier", node.depth)),
            )
            .child(div().text_sm().child(node.title.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(node.subtitle.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child(self.moment(side, node.world_time)),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn render_comparison(&self, comparison: &SnapshotComparison, cx: &mut Context<Self>) -> Div {
        let timeline_changes = comparison.timeline.left_only.len()
            + comparison.timeline.right_only.len()
            + comparison.timeline.changed.len();
        let command_changes = comparison.commands.left_only.len()
            + comparison.commands.right_only.len()
            + comparison.commands.changed.len();

        let mut entities = div().flex().flex_col().gap_2();
        for difference in comparison.entities.iter().take(ENTITY_DIFFERENCE_LIMIT) {
            entities = entities.child(self.render_entity_difference(difference, cx));
        }
        if comparison.entities.is_empty() {
            entities = entities.child(
                div()
                    .text_sm()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child("Nothing about anyone differs."),
            );
        } else if let Some(notice) = hidden_notice(
            comparison.entities.len(),
            ENTITY_DIFFERENCE_LIMIT,
            "differences",
        ) {
            entities = entities.child(truncation_notice(notice));
        }

        let mut relations = div().flex().flex_col().gap_2();
        for difference in comparison.relations.iter().take(RELATION_DIFFERENCE_LIMIT) {
            relations = relations.child(self.render_relation_difference(difference, cx));
        }
        if comparison.relations.is_empty() {
            relations = relations.child(
                div()
                    .text_sm()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child("No relation state differences"),
            );
        } else if let Some(notice) = hidden_notice(
            comparison.relations.len(),
            RELATION_DIFFERENCE_LIMIT,
            "relation differences",
        ) {
            relations = relations.child(truncation_notice(notice));
        }

        let mut timeline = div().flex().flex_col().gap_2();
        for item in comparison
            .timeline
            .left_only
            .iter()
            .take(TIMELINE_DIFFERENCE_LIMIT_PER_KIND)
        {
            timeline = timeline.child(self.render_timeline_item(
                ComparisonSide::Left,
                "Left only",
                item,
                cx,
            ));
        }
        for item in comparison
            .timeline
            .right_only
            .iter()
            .take(TIMELINE_DIFFERENCE_LIMIT_PER_KIND)
        {
            timeline = timeline.child(self.render_timeline_item(
                ComparisonSide::Right,
                "Right only",
                item,
                cx,
            ));
        }
        for item in comparison
            .timeline
            .changed
            .iter()
            .take(TIMELINE_DIFFERENCE_LIMIT_PER_KIND)
        {
            timeline = timeline.child(self.render_changed_timeline_item(item, cx));
        }
        if timeline_changes == 0 {
            timeline = timeline.child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child("No timeline differences"),
            );
        } else {
            let hidden_timeline = hidden_after_group_limits(
                &[
                    comparison.timeline.left_only.len(),
                    comparison.timeline.right_only.len(),
                    comparison.timeline.changed.len(),
                ],
                TIMELINE_DIFFERENCE_LIMIT_PER_KIND,
            );
            if hidden_timeline > 0 {
                timeline = timeline.child(truncation_notice(format!(
                    "{hidden_timeline} more timeline differences not shown"
                )));
            }
        }

        let mut commands = div().flex().flex_col().gap_2();
        for command in &comparison.commands.left_only {
            commands = commands
                .child(self.render_command(&format!("Left only · {}", self.left_label), command));
        }
        for command in &comparison.commands.right_only {
            commands = commands
                .child(self.render_command(&format!("Right only · {}", self.right_label), command));
        }
        for command in &comparison.commands.changed {
            commands = commands.child(self.render_changed_command(command));
        }
        if command_changes == 0 {
            commands = commands.child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child("No available-action differences"),
            );
        }

        div()
            .w(px(520.0))
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .flex()
            .flex_col()
            .gap_4()
            .child(div().text_lg().child("What changed"))
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child("Select anything below to see it as it stands in that future."),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(summary_chip("Entities", comparison.entities.len()))
                    .child(summary_chip("Relations", comparison.relations.len()))
                    .child(summary_chip("Moments", timeline_changes))
                    .child(summary_chip("Commands", command_changes)),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Left · time {}", comparison.left.world_time)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(format!("Right · time {}", comparison.right.world_time)),
                    ),
            )
            .child(div().text_sm().child("People, places and things"))
            .child(entities)
            .child(div().text_sm().child("Relation state"))
            .child(relations)
            .child(div().text_sm().child("Moments"))
            .child(timeline)
            .child(div().text_sm().child("Available actions"))
            .child(commands)
    }

    fn render_timeline_item(
        &self,
        side: ComparisonSide,
        relation: &str,
        item: &TimelineItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let selected = self.is_selected(side, selection);
        let mut card = div()
            .id(SharedString::from(format!(
                "timeline-difference-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_3()
            .rounded_md()
            .bg(if selected {
                ui::color(tokens::ROW_SELECTED)
            } else {
                ui::color(tokens::SURFACE)
            })
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(div().text_sm().child(item.title.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child(format!("{relation} · time {}", item.world_time)),
                    ),
            );
        if let Some(detail) = timeline_detail(&item.subtitle) {
            card = card.child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(detail),
            );
        }
        card.on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn render_changed_timeline_item(
        &self,
        item: &ChangedTimelineItem,
        cx: &mut Context<Self>,
    ) -> Div {
        let title = if item.left.title == item.right.title {
            item.left.title.clone()
        } else {
            format!("{} → {}", item.left.title, item.right.title)
        };
        let left_detail =
            timeline_detail(&item.left.subtitle).unwrap_or_else(|| "No detail".into());
        let right_detail =
            timeline_detail(&item.right.subtitle).unwrap_or_else(|| "No detail".into());

        div()
            .p_3()
            .rounded_md()
            .bg(ui::color(tokens::SURFACE))
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(div().text_sm().child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child("Changed"),
                    ),
            )
            .child(self.render_changed_timeline_side(
                ComparisonSide::Left,
                &item.left,
                left_detail,
                cx,
            ))
            .child(self.render_changed_timeline_side(
                ComparisonSide::Right,
                &item.right,
                right_detail,
                cx,
            ))
    }

    fn render_changed_timeline_side(
        &self,
        side: ComparisonSide,
        item: &TimelineItem,
        detail: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selection = item.id;
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "changed-timeline-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(if selected {
                ui::color(tokens::ROW_SELECTED)
            } else {
                ui::color(tokens::WINDOW)
            })
            .cursor_pointer()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(format!(
                        "{} · {} · time {}",
                        match side {
                            ComparisonSide::Left => "Left",
                            ComparisonSide::Right => "Right",
                        },
                        self.side_label(side),
                        item.world_time
                    )),
            )
            .child(div().text_xs().child(detail))
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn render_command(&self, relation: &str, command: &ProjectionCommand) -> Div {
        div()
            .p_3()
            .rounded_md()
            .bg(ui::color(tokens::SURFACE))
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(div().text_sm().child(command.title.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child(relation.to_string()),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(command.detail.clone()),
            )
    }

    fn render_changed_command(&self, command: &ChangedCommand) -> Div {
        let title = if command.left.title == command.right.title {
            command.left.title.clone()
        } else {
            format!("{} → {}", command.left.title, command.right.title)
        };

        div()
            .p_3()
            .rounded_md()
            .bg(ui::color(tokens::SURFACE))
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(div().text_sm().child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child("Changed"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::ACCENT_TEXT))
                            .child(format!("Left · {}", self.left_label)),
                    )
                    .child(div().text_xs().child(command.left.detail.clone())),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::ACCENT_TEXT))
                            .child(format!("Right · {}", self.right_label)),
                    )
                    .child(div().text_xs().child(command.right.detail.clone())),
            )
    }

    fn render_entity_difference(
        &self,
        difference: &EntityDifference,
        cx: &mut Context<Self>,
    ) -> Div {
        let title = difference
            .left
            .as_ref()
            .map(|entity| entity.title.clone())
            .or_else(|| difference.right.as_ref().map(|entity| entity.title.clone()))
            .unwrap_or_else(|| difference.id.stable_key());

        let mut inspection = div().flex().gap_2();
        if matches!(
            difference.kind,
            DifferenceKind::LeftOnly | DifferenceKind::Changed
        ) {
            inspection = inspection.child(self.render_entity_inspection_chip(
                ComparisonSide::Left,
                difference.id,
                cx,
            ));
        }
        if matches!(
            difference.kind,
            DifferenceKind::RightOnly | DifferenceKind::Changed
        ) {
            inspection = inspection.child(self.render_entity_inspection_chip(
                ComparisonSide::Right,
                difference.id,
                cx,
            ));
        }

        let mut rows = div().flex().flex_col().gap_1();
        if !difference.inspector_rows.is_empty() {
            rows = rows.child(
                div()
                    .flex()
                    .gap_2()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child(div().w(px(150.0)).child("Field"))
                    .child(div().w(px(140.0)).child(self.left_label.clone()))
                    .child(div().w(px(140.0)).child(self.right_label.clone())),
            );
        }
        for row in difference.inspector_rows.iter().take(INSPECTOR_ROW_LIMIT) {
            rows = rows.child(
                div()
                    .flex()
                    .gap_2()
                    .text_xs()
                    .child(
                        div()
                            .w(px(150.0))
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(row.key.label.clone()),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .child(row.left.clone().unwrap_or_else(|| "—".into())),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .child(row.right.clone().unwrap_or_else(|| "—".into())),
                    ),
            );
        }
        if let Some(notice) = hidden_notice(
            difference.inspector_rows.len(),
            INSPECTOR_ROW_LIMIT,
            "changed fields",
        ) {
            rows = rows.child(truncation_notice(notice));
        }

        div()
            .p_3()
            .rounded_md()
            .bg(ui::color(tokens::SURFACE))
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(div().text_sm().child(title))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                                    .child(difference_kind_label(difference.kind)),
                            ),
                    )
                    .child(inspection),
            )
            .child(rows)
    }

    fn render_relation_difference(
        &self,
        difference: &RelationDifference,
        cx: &mut Context<Self>,
    ) -> Div {
        let title = difference
            .left
            .as_ref()
            .map(|relation| relation.title.clone())
            .or_else(|| {
                difference
                    .right
                    .as_ref()
                    .map(|relation| relation.title.clone())
            })
            .unwrap_or_else(|| difference.id.stable_key());

        let mut inspection = div().flex().gap_2();
        if matches!(
            difference.kind,
            DifferenceKind::LeftOnly | DifferenceKind::Changed
        ) {
            inspection = inspection.child(self.render_relation_inspection_chip(
                ComparisonSide::Left,
                difference.id,
                cx,
            ));
        }
        if matches!(
            difference.kind,
            DifferenceKind::RightOnly | DifferenceKind::Changed
        ) {
            inspection = inspection.child(self.render_relation_inspection_chip(
                ComparisonSide::Right,
                difference.id,
                cx,
            ));
        }

        let mut rows = div().flex().flex_col().gap_1();
        if !difference.inspector_rows.is_empty() {
            rows = rows.child(
                div()
                    .flex()
                    .gap_2()
                    .text_xs()
                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                    .child(div().w(px(150.0)).child("Field"))
                    .child(div().w(px(140.0)).child(self.left_label.clone()))
                    .child(div().w(px(140.0)).child(self.right_label.clone())),
            );
        }
        for row in difference.inspector_rows.iter().take(INSPECTOR_ROW_LIMIT) {
            rows = rows.child(
                div()
                    .flex()
                    .gap_2()
                    .text_xs()
                    .child(
                        div()
                            .w(px(150.0))
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(row.key.label.clone()),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .child(row.left.clone().unwrap_or_else(|| "—".into())),
                    )
                    .child(
                        div()
                            .w(px(140.0))
                            .child(row.right.clone().unwrap_or_else(|| "—".into())),
                    ),
            );
        }
        if let Some(notice) = hidden_notice(
            difference.inspector_rows.len(),
            INSPECTOR_ROW_LIMIT,
            "changed fields",
        ) {
            rows = rows.child(truncation_notice(notice));
        }

        div()
            .p_3()
            .rounded_md()
            .bg(ui::color(tokens::SURFACE))
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(div().text_sm().child(title))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(ui::color(tokens::TEXT_TERTIARY))
                                    .child(difference_kind_label(difference.kind)),
                            ),
                    )
                    .child(inspection),
            )
            .child(rows)
    }

    fn render_relation_inspection_chip(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "relation-difference-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(if selected {
                ui::color(tokens::ROW_SELECTED)
            } else {
                ui::color(tokens::WINDOW)
            })
            .cursor_pointer()
            .text_xs()
            .text_color(ui::color(tokens::ACCENT_TEXT))
            .child(format!(
                "Inspect {}",
                match side {
                    ComparisonSide::Left => "Left",
                    ComparisonSide::Right => "Right",
                }
            ))
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    fn render_entity_inspection_chip(
        &self,
        side: ComparisonSide,
        selection: SelectionId,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.is_selected(side, selection);
        div()
            .id(SharedString::from(format!(
                "entity-difference-{}-{}",
                side.key(),
                selection.stable_key()
            )))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                ui::color(tokens::ACCENT)
            } else {
                ui::color(tokens::BORDER)
            })
            .bg(if selected {
                ui::color(tokens::ROW_SELECTED)
            } else {
                ui::color(tokens::WINDOW)
            })
            .cursor_pointer()
            .text_xs()
            .text_color(ui::color(tokens::ACCENT_TEXT))
            .child(format!(
                "Inspect {}",
                match side {
                    ComparisonSide::Left => "Left",
                    ComparisonSide::Right => "Right",
                }
            ))
            .on_click(cx.listener(move |this, _, _, cx| this.select(side, selection, cx)))
    }

    /// A moment in one side's own time ("Sol 3"). Two futures are the same
    /// World, so either side's calendar reads for both; two saved Worlds may
    /// come from different Packs, so each reads in its own.
    fn moment(&self, side: ComparisonSide, world_time: u64) -> String {
        let snapshot = match &self.source {
            ComparisonSource::Strategies(evaluation) => evaluation
                .left
                .outcome()
                .or_else(|| evaluation.right.outcome())
                .map(|outcome| &outcome.snapshot),
            ComparisonSource::Saved { .. } => self.snapshot(side),
        };
        match snapshot {
            Some(snapshot) => snapshot.moment_label(world_time),
            None => ProjectionSnapshot::default().moment_label(world_time),
        }
    }

    fn heading(&self) -> (&'static str, &'static str) {
        match &self.source {
            ComparisonSource::Strategies(_) => (
                "Two futures",
                "The same World from the same moment, one choice apart.",
            ),
            ComparisonSource::Saved { .. } => {
                ("Two Worlds side by side", "As each of them stands now.")
            }
        }
    }
}

impl Render for StrategyComparisonView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            _window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        let (title, subtitle) = self.heading();
        let mut body = div()
            .id("strategy-comparison-scroll")
            .w_full()
            .h_full()
            .overflow_y_scroll()
            .p_5()
            .flex()
            .flex_col()
            .gap_5()
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::page_title(title))
                    .child(ui::body(subtitle)),
            );

        if let ComparisonSource::Saved { context, .. } = &self.source {
            if let Some(relation) = &context.relation {
                body = body.child(
                    div()
                        .p_2()
                        .rounded_md()
                        .bg(ui::color(tokens::ACCENT_SOFT))
                        .text_sm()
                        .child(format!("Lineage relation · {relation}")),
                );
            }
        }

        // Each future's scene gets half the window, less the page's and the
        // card's padding.
        let stage_width = (f32::from(_window.viewport_size().width) - 40.0 - 16.0) / 2.0 - 34.0;
        let eyebrow = match &self.source {
            ComparisonSource::Strategies(_) => "If you choose",
            ComparisonSource::Saved { .. } => "Saved World",
        };
        body = body.child(
            div()
                .flex()
                .gap_4()
                .items_start()
                .child(self.render_future(ComparisonSide::Left, eyebrow, stage_width, cx))
                .child(self.render_future(ComparisonSide::Right, eyebrow, stage_width, cx)),
        );
        if let ComparisonSource::Saved { context, .. } = &self.source {
            let provenance = [&context.left_provenance, &context.right_provenance]
                .into_iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>();
            if !provenance.is_empty() {
                body = body.child(ui::caption(provenance.join(" · ")));
            }
        }

        if let Some(divergence) = self.divergence() {
            body = body.child(self.render_divergence(&divergence, cx));
        }
        if let Some(evidence) = self.render_selected_evidence(cx) {
            body = body.child(evidence);
        }

        let comparison = match &self.source {
            ComparisonSource::Strategies(evaluation) => evaluation.comparison.as_ref(),
            ComparisonSource::Saved { comparison, .. } => Some(comparison),
        };
        body = if let Some(comparison) = comparison {
            body.child(self.render_comparison(comparison, cx))
        } else {
            body.child(
                div()
                    .w(px(520.0))
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(ui::color(tokens::DANGER))
                    .bg(ui::color(tokens::SURFACE))
                    .child("Comparison unavailable because one or both strategies failed."),
            )
        };

        body
    }
}

fn timeline_detail(subtitle: &str) -> Option<String> {
    let detail = subtitle.trim();
    (!detail.is_empty()).then(|| detail.to_owned())
}

fn hidden_notice(total: usize, limit: usize, noun: &str) -> Option<String> {
    let hidden = total.saturating_sub(limit);
    (hidden > 0).then(|| format!("{hidden} more {noun} not shown"))
}

fn hidden_after_group_limits(counts: &[usize], limit: usize) -> usize {
    counts.iter().map(|count| count.saturating_sub(limit)).sum()
}

fn truncation_notice(message: String) -> Div {
    div()
        .text_xs()
        .text_color(ui::color(tokens::TEXT_TERTIARY))
        .child(message)
}

fn summary_chip(label: &str, count: usize) -> Div {
    div()
        .p_2()
        .rounded_md()
        .bg(ui::color(tokens::ACCENT_SOFT))
        .text_sm()
        .child(format!("{label}: {count}"))
}

fn difference_kind_label(kind: DifferenceKind) -> &'static str {
    match kind {
        DifferenceKind::LeftOnly => "Left only",
        DifferenceKind::RightOnly => "Right only",
        DifferenceKind::Changed => "Changed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_compare::compare_snapshots;

    #[test]
    fn semantic_timeline_detail_is_kept_for_comparison_ui() {
        assert_eq!(
            timeline_detail("Outward became the durable posture. · Event #7"),
            Some("Outward became the durable posture. · Event #7".into())
        );
    }

    #[test]
    fn blank_timeline_detail_stays_absent() {
        assert_eq!(timeline_detail("   "), None);
    }

    #[test]
    fn hidden_notice_only_reports_truncated_items() {
        assert_eq!(hidden_notice(10, 10, "items"), None);
        assert_eq!(
            hidden_notice(13, 10, "items"),
            Some("3 more items not shown".into())
        );
    }

    #[test]
    fn grouped_limits_count_every_hidden_item() {
        assert_eq!(hidden_after_group_limits(&[7, 2, 5], 4), 4);
    }

    #[test]
    fn saved_comparison_keeps_side_scoped_snapshots_separate() {
        let left = ProjectionSnapshot {
            title: "Left evidence".into(),
            world_time: 10,
            ..ProjectionSnapshot::default()
        };
        let right = ProjectionSnapshot {
            title: "Right evidence".into(),
            world_time: 20,
            ..ProjectionSnapshot::default()
        };
        let comparison = compare_snapshots(&left, &right);
        let view =
            StrategyComparisonView::saved(left, right, comparison, "Left future", "Right future");

        assert_eq!(
            view.snapshot(ComparisonSide::Left)
                .map(|snapshot| snapshot.title.as_str()),
            Some("Left evidence")
        );
        assert_eq!(
            view.snapshot(ComparisonSide::Right)
                .map(|snapshot| snapshot.title.as_str()),
            Some("Right evidence")
        );
        assert_eq!(view.selected, None);
    }
}
