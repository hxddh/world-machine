use super::{sanitize_document_base, unique_document_id, SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, Div, Entity, IntoElement, Render,
    SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use std::rc::Rc;
use std::sync::Arc;
use world_document::{WorldBranchCause, WorldDocument, WorldLineage, WorldParent};
use world_host::WorldRegistry;
use world_library::{DurableWorldSession, WorldDocumentId, WorldLibrary};
use world_persistence::WorldArchive;
use world_strategy_document::{available_choices, evaluate_choices, StrategyChoice};
use world_strategy_gpui::StrategyComparisonView;

const HORIZON_PRESETS: [u64; 3] = [5, 20, 100];

pub(crate) fn document_actions(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Div {
    let mut actions = div().flex().gap_2();
    if available_choices(&document.borrow().session).len() >= 2 {
        actions = actions.child(
            div()
                .id("compare-world-choices")
                .cursor_pointer()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0x9eb0d6))
                .bg(rgb(0xf4f7ff))
                .text_sm()
                .child("Compare choices…")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.status = Some(match open_setup(&this.document, cx) {
                        Ok(count) => format!("Opened Compare Futures · {count} choices"),
                        Err(error) => format!("Compare failed: {error}"),
                    });
                    cx.notify();
                })),
        );
    }

    actions
        .child(
            div()
                .id("save-as-world-document")
                .cursor_pointer()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xcacac4))
                .bg(rgb(0xffffff))
                .text_sm()
                .child("Save As…")
                .on_click(cx.listener(|this, _, _, cx| this.save_as(cx))),
        )
        .child(
            div()
                .id("reload-world-document")
                .cursor_pointer()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xcacac4))
                .bg(rgb(0xffffff))
                .text_sm()
                .child("Reload from disk")
                .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
        )
}

fn open_setup(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<usize, String> {
    let choices = available_choices(&document.borrow().session);
    if choices.len() < 2 {
        return Err("This World needs at least two choices to compare".into());
    }

    let count = choices.len();
    let document = Rc::clone(document);
    let bounds = Bounds::centered(None, size(px(900.0), px(720.0)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| cx.new(|_| StrategySetupView::new(document, choices)),
    )
    .map_err(|error| error.to_string())?;

    Ok(count)
}

struct StrategySetupView {
    document: SharedDocument,
    choices: Vec<StrategyChoice>,
    left_index: usize,
    right_index: usize,
    horizon: u64,
    status: Option<String>,
}

impl StrategySetupView {
    fn new(document: SharedDocument, choices: Vec<StrategyChoice>) -> Self {
        Self {
            document,
            choices,
            left_index: 0,
            right_index: 1,
            horizon: 20,
            status: None,
        }
    }

    fn render_choice_column(
        &self,
        label: &str,
        side: &'static str,
        selected_index: usize,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut column = div().w(px(390.0)).flex().flex_col().gap_2().child(
            div()
                .text_sm()
                .text_color(rgb(0x666666))
                .child(label.to_string()),
        );

        for (index, choice) in self.choices.iter().enumerate() {
            let selected = index == selected_index;
            let mut card = div()
                .id(SharedString::from(format!("strategy-{side}-{index}")))
                .w_full()
                .cursor_pointer()
                .p_3()
                .rounded_md()
                .border_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().child(choice.title.clone()))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x777777))
                        .child(choice.detail.clone()),
                );
            card = if selected {
                card.border_color(rgb(0x6684c4)).bg(rgb(0xf2f6ff))
            } else {
                card.border_color(rgb(0xd8d8d2)).bg(rgb(0xffffff))
            };
            column = column.child(card.on_click(cx.listener(move |this, _, _, cx| {
                if side == "left" {
                    this.left_index = index;
                } else {
                    this.right_index = index;
                }
                this.status = None;
                cx.notify();
            })));
        }

        column
    }

    fn render_horizon(&self, cx: &mut Context<Self>) -> Div {
        let mut row = div().flex().gap_2();
        for horizon in HORIZON_PRESETS {
            let selected = horizon == self.horizon;
            let mut option = div()
                .id(SharedString::from(format!("strategy-horizon-{horizon}")))
                .cursor_pointer()
                .px_3()
                .p_2()
                .rounded_md()
                .border_1()
                .text_sm()
                .child(format!("{horizon} periods"));
            option = if selected {
                option.border_color(rgb(0x6684c4)).bg(rgb(0xf2f6ff))
            } else {
                option.border_color(rgb(0xd8d8d2)).bg(rgb(0xffffff))
            };
            row = row.child(option.on_click(cx.listener(move |this, _, _, cx| {
                this.horizon = horizon;
                this.status = None;
                cx.notify();
            })));
        }
        row
    }

    fn run_comparison(&mut self, cx: &mut Context<Self>) -> Result<(String, String), String> {
        let left = self
            .choices
            .get(self.left_index)
            .cloned()
            .ok_or_else(|| "Left choice is no longer available".to_string())?;
        let right = self
            .choices
            .get(self.right_index)
            .cloned()
            .ok_or_else(|| "Right choice is no longer available".to_string())?;
        if left.id == right.id {
            return Err("Choose two different futures".into());
        }

        let (evaluation, source_label, source_archive, registry, library) = {
            let document = self.document.borrow();
            let source_archive = document
                .session
                .current_archive()
                .map_err(|error| error.to_string())?;
            let evaluation = evaluate_choices(
                &document.session,
                &document.registry,
                &left.id,
                &right.id,
                self.horizon,
            )
            .map_err(|error| error.to_string())?;
            (
                evaluation,
                document.session.display_name(),
                source_archive,
                Arc::clone(&document.registry),
                Arc::clone(&document.library),
            )
        };

        let left_archive = evaluation
            .left
            .outcome()
            .and_then(|outcome| outcome.archive.clone());
        let right_archive = evaluation
            .right
            .outcome()
            .and_then(|outcome| outcome.archive.clone());
        let left_lineage = strategy_lineage(&source_label, &source_archive, &left, self.horizon);
        let right_lineage = strategy_lineage(&source_label, &source_archive, &right, self.horizon);
        let left_label = left.title;
        let right_label = right.title;
        let comparison_left = left_label.clone();
        let comparison_right = right_label.clone();
        let comparison =
            cx.new(|_| StrategyComparisonView::new(evaluation, comparison_left, comparison_right));

        let result_left = left_label.clone();
        let result_right = right_label.clone();
        let bounds = Bounds::centered(None, size(px(1240.0), px(980.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|_| StrategyResultView {
                    comparison,
                    registry,
                    library,
                    source_label,
                    left_label: result_left,
                    right_label: result_right,
                    left_archive,
                    right_archive,
                    left_lineage,
                    right_lineage,
                    left_saved: None,
                    right_saved: None,
                    status: None,
                })
            },
        )
        .map_err(|error| error.to_string())?;

        Ok((left_label, right_label))
    }
}

impl Render for StrategySetupView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title("Compare Futures — World Machine");

        let same_choice = self.left_index == self.right_index;
        let mut body = div()
            .size_full()
            .p_5()
            .bg(rgb(0xf7f7f3))
            .text_color(rgb(0x202020))
            .flex()
            .flex_col()
            .gap_4()
            .child(div().text_xl().child("Compare Futures"))
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666666))
                    .child("Choose two possible actions from the same durable World, then decide how far each future should run."),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(self.render_choice_column("Future A", "left", self.left_index, cx))
                    .child(self.render_choice_column("Future B", "right", self.right_index, cx)),
            )
            .child(div().text_sm().child("Horizon"))
            .child(self.render_horizon(cx));

        if same_choice {
            body = body.child(
                div()
                    .text_sm()
                    .text_color(rgb(0x9b5a4f))
                    .child("Choose two different futures before running the comparison."),
            );
        }
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
                .id("run-strategy-comparison")
                .cursor_pointer()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(0x6684c4))
                .bg(rgb(0xeaf0ff))
                .text_sm()
                .child(format!("Run comparison · {} periods", self.horizon))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.status = Some(match this.run_comparison(cx) {
                        Ok((left, right)) => format!("Opened comparison · {left} vs {right}"),
                        Err(error) => format!("Could not compare: {error}"),
                    });
                    cx.notify();
                })),
        )
    }
}

#[derive(Clone, Copy)]
enum FutureSide {
    Left,
    Right,
}

struct StrategyResultView {
    comparison: Entity<StrategyComparisonView>,
    registry: Arc<WorldRegistry>,
    library: Arc<WorldLibrary>,
    source_label: String,
    left_label: String,
    right_label: String,
    left_archive: Option<WorldArchive>,
    right_archive: Option<WorldArchive>,
    left_lineage: WorldLineage,
    right_lineage: WorldLineage,
    left_saved: Option<WorldDocumentId>,
    right_saved: Option<WorldDocumentId>,
    status: Option<String>,
}

impl StrategyResultView {
    fn save_future(&mut self, side: FutureSide) -> Result<WorldDocumentId, String> {
        let (archive, lineage, label, side_label) = match side {
            FutureSide::Left => (
                self.left_archive
                    .clone()
                    .ok_or_else(|| "Future A has no durable archive".to_string())?,
                self.left_lineage.clone(),
                self.left_label.clone(),
                "Future A",
            ),
            FutureSide::Right => (
                self.right_archive
                    .clone()
                    .ok_or_else(|| "Future B has no durable archive".to_string())?,
                self.right_lineage.clone(),
                self.right_label.clone(),
                "Future B",
            ),
        };

        let source = source_world_base(&self.source_label);
        let base = sanitize_document_base(&format!("{source}-{label}"));
        let id = unique_document_id(base, Some(self.library.as_ref()))
            .map_err(|error| error.to_string())?;
        let future = WorldDocument::new(archive).with_lineage(lineage);
        let summary = self
            .library
            .create_from_document(id, &future)
            .map_err(|error| error.to_string())?;
        let saved_id = summary.id;

        match side {
            FutureSide::Left => self.left_saved = Some(saved_id.clone()),
            FutureSide::Right => self.right_saved = Some(saved_id.clone()),
        }
        self.status = Some(format!("Saved {side_label} as {saved_id}"));
        Ok(saved_id)
    }

    fn open_saved_future(
        &mut self,
        side: FutureSide,
        cx: &mut Context<Self>,
    ) -> Result<String, String> {
        let saved_id = match side {
            FutureSide::Left => self
                .left_saved
                .clone()
                .ok_or_else(|| "Future A has not been saved".to_string())?,
            FutureSide::Right => self
                .right_saved
                .clone()
                .ok_or_else(|| "Future B has not been saved".to_string())?,
        };
        let session =
            DurableWorldSession::open(saved_id, self.registry.as_ref(), self.library.as_ref())
                .map_err(|error| error.to_string())?;
        let document_label = session.display_name();
        let registry = Arc::clone(&self.registry);
        let library = Arc::clone(&self.library);
        let bounds = Bounds::centered(None, size(px(1100.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| cx.new(|cx| WorldDocumentView::new(session, registry, library, cx)),
        )
        .map_err(|error| error.to_string())?;

        Ok(document_label)
    }

    fn render_save_action(&self, side: FutureSide, cx: &mut Context<Self>) -> Div {
        let (archive_available, saved, save_button_id, open_button_id, label) = match side {
            FutureSide::Left => (
                self.left_archive.is_some(),
                self.left_saved.as_ref(),
                "save-strategy-future-left",
                "open-strategy-future-left",
                "Future A",
            ),
            FutureSide::Right => (
                self.right_archive.is_some(),
                self.right_saved.as_ref(),
                "save-strategy-future-right",
                "open-strategy-future-right",
                "Future B",
            ),
        };
        if !archive_available {
            return div();
        }

        if let Some(saved) = saved {
            return div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0xb9c8b1))
                        .bg(rgb(0xf1f6ee))
                        .text_sm()
                        .child(format!("Saved {label} · {saved}")),
                )
                .child(
                    div()
                        .id(open_button_id)
                        .cursor_pointer()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0x9eb0d6))
                        .bg(rgb(0xf4f7ff))
                        .text_sm()
                        .child("Open")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.status = Some(match this.open_saved_future(side, cx) {
                                Ok(document) => format!("Opened saved {label} · {document}"),
                                Err(error) => format!("Open failed: {error}"),
                            });
                            cx.notify();
                        })),
                );
        }

        div().child(
            div()
                .id(save_button_id)
                .cursor_pointer()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0x9eb0d6))
                .bg(rgb(0xf4f7ff))
                .text_sm()
                .child(format!("Save {label}"))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Err(error) = this.save_future(side) {
                        this.status = Some(format!("Save failed: {error}"));
                    }
                    cx.notify();
                })),
        )
    }
}

impl Render for StrategyResultView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title("Strategy Comparison — World Machine");

        let actions = div()
            .flex()
            .gap_2()
            .child(self.render_save_action(FutureSide::Left, cx))
            .child(self.render_save_action(FutureSide::Right, cx));

        let mut chrome = div()
            .w_full()
            .p_3()
            .border_b_1()
            .border_color(rgb(0xd9d9d3))
            .bg(rgb(0xf7f7f3))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_sm().child("Strategy Comparison"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777770))
                            .child(format!("Source · {}", self.source_label)),
                    ),
            )
            .child(actions);

        if let Some(status) = &self.status {
            chrome = chrome.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x4e6fb3))
                    .child(status.clone()),
            );
        }

        div().size_full().flex().flex_col().child(chrome).child(
            div()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .child(self.comparison.clone()),
        )
    }
}

fn strategy_lineage(
    source_label: &str,
    source_archive: &WorldArchive,
    choice: &StrategyChoice,
    horizon: u64,
) -> WorldLineage {
    WorldLineage {
        parent: WorldParent {
            document: Some(source_label.to_owned()),
            pack: source_archive.pack.clone(),
            world_time: source_archive.world_time,
            event_count: source_archive.events.len(),
        },
        branch: WorldBranchCause::Strategy {
            choice_id: choice.id.clone(),
            choice_title: choice.title.clone(),
            horizon,
        },
    }
}

fn source_world_base(label: &str) -> &str {
    label
        .strip_suffix(".world.json")
        .or_else(|| label.strip_suffix(".world"))
        .unwrap_or(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_persistence::{WorldPackRef, WORLD_ARCHIVE_FORMAT, WORLD_ARCHIVE_VERSION};

    #[test]
    fn strategy_lineage_records_the_parent_branch_point_before_the_future_runs() {
        let source = WorldArchive {
            format: WORLD_ARCHIVE_FORMAT.into(),
            format_version: WORLD_ARCHIVE_VERSION,
            pack: WorldPackRef::new("world-machine.lineage-mock", "1"),
            world_time: 42,
            events: Vec::new(),
            pending: Vec::new(),
        };
        let choice = StrategyChoice {
            id: "mock.choose-a".into(),
            title: "Choose A".into(),
            detail: "A possible future".into(),
        };

        let lineage = strategy_lineage("Source.world", &source, &choice, 20);

        assert_eq!(lineage.parent.document.as_deref(), Some("Source.world"));
        assert_eq!(lineage.parent.pack, source.pack);
        assert_eq!(lineage.parent.world_time, 42);
        assert_eq!(lineage.parent.event_count, 0);
        assert_eq!(
            lineage.branch,
            WorldBranchCause::Strategy {
                choice_id: "mock.choose-a".into(),
                choice_title: "Choose A".into(),
                horizon: 20,
            }
        );
    }
}
