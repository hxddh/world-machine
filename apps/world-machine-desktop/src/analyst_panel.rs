use super::{
    analyst_input::{self, AnalystTextInput},
    analyst_runtime,
};
use crate::{DocumentStatus, SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, Div, Entity, IntoElement, Render,
    SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use std::sync::Arc;
use world_library::{WorldDocumentId, WorldDocumentSummary, WorldLibrary};
use world_machine_desktop::analyst_session::{
    DesktopAnalystConfig, DesktopAnalystSession, DesktopAnalystState,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum PanelPhase {
    Setup,
    Starting,
    Active,
    Fatal(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PanelTurn {
    answer: String,
    tool_calls: Vec<PanelToolCall>,
    runtime_errors: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PanelToolCall {
    tool: String,
    input: String,
    output: String,
    is_error: bool,
}

pub(super) fn document_action(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> impl IntoElement {
    if document.borrow().session.document_id().is_none() {
        return div().id("analyze-saved-worlds-unavailable");
    }

    let document = document.clone();
    div()
        .id("analyze-saved-worlds")
        .cursor_pointer()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x8da6d8))
        .bg(rgb(0xf2f6ff))
        .text_sm()
        .child("Analyze saved Worlds…")
        .on_click(cx.listener(move |this, _, _, cx| {
            this.status = Some(match open_panel(&document, cx) {
                Ok(count) => {
                    DocumentStatus::success(format!("Opened World analyst · {count} saved Worlds"))
                }
                Err(error) => {
                    DocumentStatus::error(format!("Could not open World analyst: {error}"))
                }
            });
            cx.notify();
        }))
}

fn open_panel(
    document: &SharedDocument,
    cx: &mut Context<WorldDocumentView>,
) -> Result<usize, String> {
    analyst_input::bind_keys(cx);
    let (left, library) = {
        let document = document.borrow();
        let left = document
            .session
            .document_id()
            .cloned()
            .ok_or_else(|| "Only saved Library Worlds can be analyzed".to_string())?;
        (left, Arc::clone(&document.library))
    };
    let documents = library.list().map_err(|error| error.to_string())?;
    if documents.len() < 2 {
        return Err("World analyst needs at least two saved Worlds".into());
    }
    let right = default_right_for(&left, &documents)
        .ok_or_else(|| "World analyst could not find another saved World".to_string())?;
    let count = documents.len();
    let runtime = analyst_runtime::discover();
    let bounds = Bounds::centered(None, size(px(920.0), px(820.0)), cx);

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| {
            cx.new(|cx| AnalystPanelView::new(library, documents, left, right, runtime, cx))
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(count)
}

fn default_right_for(
    left: &WorldDocumentId,
    documents: &[WorldDocumentSummary],
) -> Option<WorldDocumentId> {
    let left_pack = documents
        .iter()
        .find(|document| document.id == *left)
        .map(|document| document.pack.clone());
    left_pack
        .as_ref()
        .and_then(|pack| {
            documents
                .iter()
                .find(|document| document.id != *left && document.pack == *pack)
        })
        .or_else(|| documents.iter().find(|document| document.id != *left))
        .map(|document| document.id.clone())
}

struct AnalystPanelView {
    library: Arc<WorldLibrary>,
    documents: Vec<WorldDocumentSummary>,
    left: WorldDocumentId,
    right: WorldDocumentId,
    runtime: Result<DesktopAnalystConfig, String>,
    session: Option<DesktopAnalystSession>,
    question: Entity<AnalystTextInput>,
    phase: PanelPhase,
    busy: bool,
    history: Vec<PanelTurn>,
    last_error: Option<String>,
}

impl AnalystPanelView {
    fn new(
        library: Arc<WorldLibrary>,
        documents: Vec<WorldDocumentSummary>,
        left: WorldDocumentId,
        right: WorldDocumentId,
        runtime: Result<DesktopAnalystConfig, String>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.on_release(|this, _| {
            if let Some(mut session) = this.session.take() {
                let _ = session.close();
            }
        })
        .detach();
        let question = cx.new(|cx| AnalystTextInput::new("Ask why these Worlds differ…", cx));
        Self {
            library,
            documents,
            left,
            right,
            runtime,
            session: None,
            question,
            phase: PanelPhase::Setup,
            busy: false,
            history: Vec::new(),
            last_error: None,
        }
    }

    fn start_session(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.left == self.right {
            return;
        }
        let config = match &self.runtime {
            Ok(config) => config.clone(),
            Err(error) => {
                self.last_error = Some(error.clone());
                cx.notify();
                return;
            }
        };
        let library = Arc::clone(&self.library);
        let left = self.left.clone();
        let right = self.right.clone();
        self.busy = true;
        self.phase = PanelPhase::Starting;
        self.last_error = None;
        cx.notify();

        let task = cx.background_executor().spawn(async move {
            DesktopAnalystSession::start(library.as_ref(), left, right, config)
                .map_err(|error| error.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(session) => {
                        this.history = snapshot_history(&session);
                        this.session = Some(session);
                        this.phase = PanelPhase::Active;
                        this.last_error = None;
                    }
                    Err(error) => {
                        this.phase = PanelPhase::Setup;
                        this.last_error = Some(error);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn ask(&mut self, cx: &mut Context<Self>) {
        if self.busy || !matches!(self.phase, PanelPhase::Active) {
            return;
        }
        let prompt = self.question.read(cx).text().trim().to_owned();
        if prompt.is_empty() {
            self.last_error = Some("Enter a question before asking the analyst".into());
            cx.notify();
            return;
        }
        let Some(mut session) = self.session.take() else {
            self.last_error = Some("World analyst session is not available".into());
            cx.notify();
            return;
        };
        self.busy = true;
        self.last_error = None;
        self.question.update(cx, |input, cx| input.clear(cx));
        cx.notify();

        let task = cx.background_executor().spawn(async move {
            let result = session.ask(&prompt).map_err(|error| error.to_string());
            (session, result)
        });
        cx.spawn(async move |this, cx| {
            let (session, result) = task.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                this.history = snapshot_history(&session);
                match session.state() {
                    DesktopAnalystState::FatalError { message } => {
                        this.phase = PanelPhase::Fatal(message.clone());
                    }
                    DesktopAnalystState::Closed => {
                        this.phase = PanelPhase::Fatal("World analyst session closed".into());
                    }
                    DesktopAnalystState::Ready
                    | DesktopAnalystState::Answer { .. }
                    | DesktopAnalystState::RecoverableError { .. } => {
                        this.phase = PanelPhase::Active;
                    }
                }
                this.last_error = result.err();
                this.session = Some(session);
                cx.notify();
            });
        })
        .detach();
    }

    fn choose_right(&mut self, id: WorldDocumentId, cx: &mut Context<Self>) {
        if self.busy || self.session.is_some() || id == self.left {
            return;
        }
        self.right = id;
        self.last_error = None;
        cx.notify();
    }

    fn render_setup(&self, cx: &mut Context<Self>) -> Div {
        let mut worlds = div()
            .id("analyst-world-list")
            .w_full()
            .max_h(px(400.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();
        for document in &self.documents {
            if document.id == self.left {
                continue;
            }
            let id = document.id.clone();
            let selected = id == self.right;
            let title = document_title(document);
            let summary = document
                .display_summary
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("Saved World");
            let mut card = div()
                .id(SharedString::from(format!("analyst-world-{id}")))
                .cursor_pointer()
                .p_3()
                .rounded_md()
                .border_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().child(title))
                .child(div().text_xs().text_color(rgb(0x777770)).child(format!(
                    "{} · t={} · {} events",
                    summary, document.world_time, document.event_count
                )));
            card = if selected {
                card.border_color(rgb(0x6684c4)).bg(rgb(0xf2f6ff))
            } else {
                card.border_color(rgb(0xd8d8d2)).bg(rgb(0xffffff))
            };
            worlds = worlds.child(card.on_click(cx.listener(move |this, _, _, cx| {
                this.choose_right(id.clone(), cx);
            })));
        }

        let runtime_status = match &self.runtime {
            Ok(_) => div()
                .text_xs()
                .text_color(rgb(0x4d6748))
                .child("Analyst runtime available"),
            Err(error) => div()
                .text_xs()
                .text_color(rgb(0x9b4a42))
                .child(error.clone()),
        };
        let can_start = !self.busy && self.runtime.is_ok() && self.left != self.right;
        let mut start = div()
            .id("start-world-analyst")
            .px_4()
            .p_2()
            .rounded_md()
            .border_1()
            .text_sm()
            .child(if self.busy {
                "Starting…"
            } else {
                "Start analysis"
            });
        start = if can_start {
            start
                .cursor_pointer()
                .border_color(rgb(0x6684c4))
                .bg(rgb(0xf2f6ff))
                .on_click(cx.listener(|this, _, _, cx| this.start_session(cx)))
        } else {
            start
                .border_color(rgb(0xd8d8d2))
                .bg(rgb(0xf4f4f1))
                .text_color(rgb(0x999990))
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .child(format!("Left · {}", self.label_for(&self.left))),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666660))
                    .child("Choose the saved World to compare with the current World."),
            )
            .child(worlds)
            .child(runtime_status)
            .child(start)
    }

    fn render_active(&self, cx: &mut Context<Self>) -> Div {
        let mut history = div()
            .id("analyst-history")
            .w_full()
            .flex_1()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3();
        if self.history.is_empty() {
            history = history.child(
                div()
                    .p_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xe0e0db))
                    .bg(rgb(0xfafaf8))
                    .text_sm()
                    .text_color(rgb(0x666660))
                    .child("Ask a question about what changed, why the Worlds diverged, or which evidence explains the difference."),
            );
        } else {
            for (index, turn) in self.history.iter().enumerate() {
                history = history.child(render_turn(index, turn));
            }
        }

        let can_ask = !self.busy && matches!(self.phase, PanelPhase::Active);
        let mut ask = div()
            .id("ask-world-analyst")
            .px_4()
            .p_2()
            .rounded_md()
            .border_1()
            .text_sm()
            .child(if self.busy { "Analyzing…" } else { "Ask" });
        ask = if can_ask {
            ask.cursor_pointer()
                .border_color(rgb(0x6684c4))
                .bg(rgb(0xf2f6ff))
                .on_click(cx.listener(|this, _, _, cx| this.ask(cx)))
        } else {
            ask.border_color(rgb(0xd8d8d2))
                .bg(rgb(0xf4f4f1))
                .text_color(rgb(0x999990))
        };

        let composer = div()
            .w_full()
            .flex()
            .gap_2()
            .items_center()
            .child(div().flex_1().child(self.question.clone()))
            .child(ask);

        let mut body = div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(div().text_sm().child(format!(
                        "{} ↔ {}",
                        self.label_for(&self.left),
                        self.label_for(&self.right)
                    )))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777770))
                            .child("Read-only · fixed snapshot pair"),
                    ),
            )
            .child(history)
            .child(composer);
        if let PanelPhase::Fatal(message) = &self.phase {
            body = body.child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0xfff2f0))
                    .text_sm()
                    .text_color(rgb(0x9b4a42))
                    .child(format!("Analyst session ended: {message}")),
            );
        }
        body
    }

    fn label_for(&self, id: &WorldDocumentId) -> String {
        self.documents
            .iter()
            .find(|document| document.id == *id)
            .map(document_title)
            .unwrap_or_else(|| id.to_string())
    }
}

impl Render for AnalystPanelView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title("World Analyst — World Machine");
        let content = match &self.phase {
            PanelPhase::Setup | PanelPhase::Starting => self.render_setup(cx),
            PanelPhase::Active | PanelPhase::Fatal(_) => self.render_active(cx),
        };
        let mut root = div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .bg(rgb(0xf7f7f3))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child("World Analyst"))
                    .child(div().text_xs().text_color(rgb(0x777770)).child(
                        "Evidence-backed questions over two immutable saved-World snapshots",
                    )),
            )
            .child(content);
        if let Some(error) = &self.last_error {
            root = root.child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0xfff2f0))
                    .text_sm()
                    .text_color(rgb(0x9b4a42))
                    .child(error.clone()),
            );
        }
        root
    }
}

fn document_title(document: &WorldDocumentSummary) -> String {
    document
        .display_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| document.id.to_string())
}

fn snapshot_history(session: &DesktopAnalystSession) -> Vec<PanelTurn> {
    session
        .turns()
        .iter()
        .map(|turn| PanelTurn {
            answer: turn
                .text
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .unwrap_or("The analyst returned no text answer.")
                .to_owned(),
            tool_calls: turn
                .tool_calls
                .iter()
                .map(|call| PanelToolCall {
                    tool: call.tool.clone(),
                    input: call.input.to_string(),
                    output: call.output.to_string(),
                    is_error: call.is_error,
                })
                .collect(),
            runtime_errors: turn
                .runtime_errors
                .iter()
                .map(|error| error.message.clone())
                .collect(),
        })
        .collect()
}

fn render_turn(index: usize, turn: &PanelTurn) -> impl IntoElement {
    let mut card = div()
        .id(SharedString::from(format!("analyst-turn-{index}")))
        .w_full()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xd8d8d2))
        .bg(rgb(0xffffff))
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_sm().child(turn.answer.clone()));

    if !turn.tool_calls.is_empty() {
        let mut tools = div().flex().flex_col().gap_2().child(
            div()
                .text_xs()
                .text_color(rgb(0x777770))
                .child(format!("Evidence calls · {}", turn.tool_calls.len())),
        );
        for call in &turn.tool_calls {
            let status = if call.is_error { "error" } else { "ok" };
            tools = tools.child(
                div()
                    .p_2()
                    .rounded_md()
                    .bg(rgb(0xf7f7f3))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_xs().child(format!("{} · {status}", call.tool)))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777770))
                            .child(format!("input  {}", call.input)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x555550))
                            .child(format!("output {}", call.output)),
                    ),
            );
        }
        card = card.child(tools);
    }

    if !turn.runtime_errors.is_empty() {
        let mut errors = div().flex().flex_col().gap_1();
        for error in &turn.runtime_errors {
            errors = errors.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x9b4a42))
                    .child(format!("Runtime error · {error}")),
            );
        }
        card = card.child(errors);
    }
    card
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_persistence::WorldPackRef;

    fn summary(id: &str, pack: &str, title: Option<&str>) -> WorldDocumentSummary {
        WorldDocumentSummary {
            id: WorldDocumentId::new(id).unwrap(),
            pack: WorldPackRef::new(pack, "1"),
            display_title: title.map(str::to_owned),
            display_summary: None,
            world_time: 0,
            event_count: 0,
        }
    }

    #[test]
    fn default_right_prefers_same_pack_then_any_other_world() {
        let documents = vec![
            summary("left", "tiny", Some("Left")),
            summary("other-pack", "pocket", Some("Pocket")),
            summary("same-pack", "tiny", Some("Sibling")),
        ];
        assert_eq!(
            default_right_for(&WorldDocumentId::new("left").unwrap(), &documents)
                .unwrap()
                .as_str(),
            "same-pack"
        );

        let documents = vec![
            summary("left", "tiny", Some("Left")),
            summary("other-pack", "pocket", Some("Pocket")),
        ];
        assert_eq!(
            default_right_for(&WorldDocumentId::new("left").unwrap(), &documents)
                .unwrap()
                .as_str(),
            "other-pack"
        );
    }

    #[test]
    fn document_title_prefers_semantic_title() {
        assert_eq!(
            document_title(&summary("world-1", "tiny", Some("  Maple Street  "))),
            "Maple Street"
        );
        assert_eq!(
            document_title(&summary("world-1", "tiny", Some("  "))),
            "world-1"
        );
    }

    #[test]
    fn panel_source_stays_above_process_and_pi_layers() {
        let source = include_str!("analyst_panel.rs").to_ascii_lowercase();
        let forbidden = [
            ["world_", "analyst_client"].concat(),
            ["analystturn", "process"].concat(),
            ["child", "stdin"].concat(),
            ["child", "stdout"].concat(),
            ["agent_", "settled"].concat(),
            ["tool_execution", "_start"].concat(),
            ["world-machine-analyst", "-rpc"].concat(),
        ];
        for token in forbidden {
            assert!(
                !source.contains(&token),
                "analyst panel contains forbidden lower-layer token {token}"
            );
        }
    }
}
