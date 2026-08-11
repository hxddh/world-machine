use super::{SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, Div, IntoElement, Render,
    SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use std::rc::Rc;
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
        let mut column = div()
            .w(px(390.0))
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_sm().text_color(rgb(0x666666)).child(label.to_string()));

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

        let evaluation = {
            let document = self.document.borrow();
            evaluate_choices(
                &document.session,
                &document.registry,
                &left.id,
                &right.id,
                self.horizon,
            )
            .map_err(|error| error.to_string())?
        };

        let left_label = left.title;
        let right_label = right.title;
        let window_left = left_label.clone();
        let window_right = right_label.clone();
        let bounds = Bounds::centered(None, size(px(1220.0), px(920.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |_, cx| {
                cx.new(|_| StrategyComparisonView::new(evaluation, window_left, window_right))
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
