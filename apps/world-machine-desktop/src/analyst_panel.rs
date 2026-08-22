use super::{
    analyst_input::{self, AnalystTextInput},
    analyst_runtime,
};
use crate::{DocumentStatus, SharedDocument, WorldDocumentView};
use gpui::{
    div, prelude::*, px, rgb, size, AppContext, Bounds, Context, Div, Entity, IntoElement,
    PathPromptOptions, Render, SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use std::sync::Arc;
use world_library::{WorldDocumentId, WorldDocumentSummary, WorldLibrary};
use world_machine_desktop::analyst_readiness::DesktopAnalystRuntimeReadiness;
use world_machine_desktop::analyst_session::{
    DesktopAnalystCancellation, DesktopAnalystCancellationOutcome, DesktopAnalystSession,
    DesktopAnalystState,
};
use world_machine_desktop::analyst_settings::DesktopAnalystProgramSource;

#[derive(Clone, Debug, Eq, PartialEq)]
enum PanelPhase {
    Setup,
    Starting,
    Active,
    Fatal(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PanelTurn {
    question: String,
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
    let bounds = Bounds::centered(None, size(px(920.0), px(820.0)), cx);

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| cx.new(|cx| AnalystPanelView::new(library, documents, left, right, cx)),
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
    runtime: Option<analyst_runtime::AnalystRuntimeStatus>,
    runtime_checking: bool,
    settings_busy: bool,
    session: Option<DesktopAnalystSession>,
    cancellation: Option<DesktopAnalystCancellation>,
    question: Entity<AnalystTextInput>,
    phase: PanelPhase,
    busy: bool,
    cancel_requested: bool,
    history: Vec<PanelTurn>,
    failed_question: Option<String>,
    last_error: Option<String>,
}

impl AnalystPanelView {
    fn new(
        library: Arc<WorldLibrary>,
        documents: Vec<WorldDocumentSummary>,
        left: WorldDocumentId,
        right: WorldDocumentId,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.on_release(|this, cx| {
            let session = this.session.take();
            let cancellation = this.cancellation.take();
            if let Some(mut session) = session {
                cx.background_executor()
                    .spawn(async move {
                        let _ = session.close();
                    })
                    .detach();
            } else if let Some(cancellation) = cancellation {
                cx.background_executor()
                    .spawn(async move {
                        let _ = cancellation.cancel();
                    })
                    .detach();
            }
        })
        .detach();
        let question = cx.new(|cx| AnalystTextInput::new("Ask why these Worlds differ…", cx));
        let mut view = Self {
            library,
            documents,
            left,
            right,
            runtime: None,
            runtime_checking: false,
            settings_busy: false,
            session: None,
            cancellation: None,
            question,
            phase: PanelPhase::Setup,
            busy: false,
            cancel_requested: false,
            history: Vec::new(),
            failed_question: None,
            last_error: None,
        };
        view.refresh_runtime(cx);
        view
    }

    fn refresh_runtime(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.settings_busy || self.session.is_some() || self.runtime_checking {
            return;
        }
        self.runtime = None;
        self.runtime_checking = true;
        self.last_error = None;
        cx.notify();

        let task = cx
            .background_executor()
            .spawn(async move { analyst_runtime::discover_status() });
        cx.spawn(async move |this, cx| {
            let status = task.await;
            let _ = this.update(cx, |this, cx| {
                this.runtime = Some(status);
                this.runtime_checking = false;
                cx.notify();
            });
        })
        .detach();
    }

    fn configure_program(
        &mut self,
        program: analyst_runtime::AnalystRuntimeProgram,
        cx: &mut Context<Self>,
    ) {
        if self.busy || self.settings_busy || self.session.is_some() || self.runtime_checking {
            return;
        }
        let label = runtime_program_label(program);
        self.settings_busy = true;
        self.last_error = None;
        cx.notify();

        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(format!("Choose {label} executable"))),
        });
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let selected = match picker.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) => None,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.settings_busy = false;
                        this.last_error =
                            Some(format!("Could not choose {label} executable: {error}"));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.settings_busy = false;
                        this.last_error = Some(format!(
                            "{label} executable chooser was interrupted: {error}"
                        ));
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(path) = selected else {
                let _ = this.update(cx, |this, cx| {
                    this.settings_busy = false;
                    cx.notify();
                });
                return;
            };

            let result = background
                .spawn(async move { analyst_runtime::save_program(program, path) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.settings_busy = false;
                match result {
                    Ok(()) => this.refresh_runtime(cx),
                    Err(error) => {
                        this.last_error = Some(error);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn clear_program(
        &mut self,
        program: analyst_runtime::AnalystRuntimeProgram,
        cx: &mut Context<Self>,
    ) {
        if self.busy || self.settings_busy || self.session.is_some() || self.runtime_checking {
            return;
        }
        self.settings_busy = true;
        self.last_error = None;
        cx.notify();

        let task = cx
            .background_executor()
            .spawn(async move { analyst_runtime::clear_program(program) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.settings_busy = false;
                match result {
                    Ok(()) => this.refresh_runtime(cx),
                    Err(error) => {
                        this.last_error = Some(error);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn start_session(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.settings_busy || self.left == self.right {
            return;
        }
        let Some(config) = self
            .runtime
            .as_ref()
            .and_then(|status| status.readiness.config())
            .cloned()
        else {
            self.last_error =
                Some("World analyst runtime is not ready. Recheck the runtime first.".into());
            cx.notify();
            return;
        };
        let library = Arc::clone(&self.library);
        let left = self.left.clone();
        let right = self.right.clone();
        self.busy = true;
        self.cancel_requested = false;
        self.phase = PanelPhase::Starting;
        self.last_error = None;
        cx.notify();

        let task = cx.background_executor().spawn(async move {
            DesktopAnalystSession::start(library.as_ref(), left, right, config)
                .map_err(|error| error.to_string())
        });
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let mut result = Some(task.await);
            let update = this.update(cx, |this, cx| {
                let result = result
                    .take()
                    .expect("analyst startup result should be consumed once");
                this.busy = false;
                this.cancel_requested = false;
                match result {
                    Ok(session) => {
                        this.history = snapshot_history(&session);
                        this.cancellation = session.cancellation_handle();
                        this.session = Some(session);
                        this.phase = PanelPhase::Active;
                        this.last_error = None;
                    }
                    Err(error) => {
                        this.cancellation = None;
                        this.runtime = None;
                        this.phase = PanelPhase::Setup;
                        this.last_error = Some(error);
                    }
                }
                cx.notify();
            });
            if update.is_err() {
                if let Some(Ok(mut session)) = result.take() {
                    background
                        .spawn(async move {
                            let _ = session.close();
                        })
                        .detach();
                }
            }
        })
        .detach();
    }

    fn ask(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.cancel_requested || !matches!(self.phase, PanelPhase::Active) {
            return;
        }
        let submitted_question = self.question.read(cx).text().to_owned();
        let prompt = submitted_question.trim().to_owned();
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
        self.cancel_requested = false;
        self.last_error = None;
        cx.notify();

        let task = cx.background_executor().spawn(async move {
            let result = session.ask(&prompt).map_err(|error| error.to_string());
            (session, result, submitted_question)
        });
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let mut completed = Some(task.await);
            let update = this.update(cx, |this, cx| {
                let (session, result, submitted_question) = completed
                    .take()
                    .expect("analyst turn result should be consumed once");
                let cancel_requested = this.cancel_requested;
                let forced_fatal = matches!(this.phase, PanelPhase::Fatal(_));
                this.busy = false;
                this.cancel_requested = false;
                if cancel_requested {
                    this.phase = PanelPhase::Fatal("Analysis cancelled by user".into());
                } else {
                    this.history = snapshot_history(&session);
                    if !forced_fatal {
                        match session.state() {
                            DesktopAnalystState::FatalError { message } => {
                                this.phase = PanelPhase::Fatal(message.clone());
                            }
                            DesktopAnalystState::Closed => {
                                this.phase =
                                    PanelPhase::Fatal("World analyst session closed".into());
                            }
                            DesktopAnalystState::Ready
                            | DesktopAnalystState::Answer { .. }
                            | DesktopAnalystState::RecoverableError { .. } => {
                                this.phase = PanelPhase::Active;
                            }
                        }
                    }
                }
                let succeeded = result.is_ok();
                let current_question = this.question.read(cx).text().to_owned();
                if should_clear_completed_prompt(
                    &current_question,
                    &submitted_question,
                    succeeded,
                    cancel_requested,
                ) {
                    this.question.update(cx, |input, cx| input.clear(cx));
                }
                this.failed_question =
                    failed_question_after_turn(&submitted_question, succeeded, cancel_requested);
                let turn_error = result.err();
                this.last_error = if cancel_requested {
                    turn_error.or_else(|| Some("Analysis cancelled by user".to_string()))
                } else if forced_fatal {
                    this.last_error.take().or(turn_error)
                } else {
                    turn_error
                };
                this.session = Some(session);
                cx.notify();
            });
            if update.is_err() {
                if let Some((mut session, _, _)) = completed.take() {
                    background
                        .spawn(async move {
                            let _ = session.close();
                        })
                        .detach();
                }
            }
        })
        .detach();
    }

    fn cancel_analysis(&mut self, cx: &mut Context<Self>) {
        if !can_cancel_analysis(
            &self.phase,
            self.busy,
            self.session.is_some(),
            self.cancellation.is_some(),
            self.cancel_requested,
        ) {
            return;
        }
        let Some(cancellation) = self.cancellation.take() else {
            return;
        };

        match cancellation.cancel() {
            Ok(DesktopAnalystCancellationOutcome::Signaled) => {
                self.cancel_requested = true;
                self.last_error = None;
            }
            Ok(DesktopAnalystCancellationOutcome::Inactive) => {
                self.cancel_requested = false;
            }
            Err(error) => {
                self.cancel_requested = false;
                self.phase = PanelPhase::Fatal(
                    "Analysis cancellation failed; recover before continuing".into(),
                );
                self.last_error = Some(error.to_string());
            }
        }
        cx.notify();
    }

    fn start_new_comparison(&mut self, cx: &mut Context<Self>) {
        if self.busy || !matches!(self.phase, PanelPhase::Active) {
            return;
        }
        let Some(mut session) = self.session.take() else {
            let message = "World analyst session is not available".to_string();
            self.phase = PanelPhase::Fatal(message.clone());
            self.last_error = Some(message);
            cx.notify();
            return;
        };

        self.cancellation.take();
        self.busy = true;
        self.cancel_requested = false;
        self.last_error = None;
        cx.notify();

        let task = cx
            .background_executor()
            .spawn(async move { session.close().map_err(|error| error.to_string()) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                this.cancel_requested = false;
                this.cancellation = None;
                match result {
                    Ok(()) => {
                        let transitioned = reset_new_comparison_state(
                            &mut this.phase,
                            &mut this.history,
                            &mut this.runtime,
                            &mut this.failed_question,
                            &mut this.last_error,
                        );
                        debug_assert!(transitioned);
                        this.runtime_checking = false;
                        this.refresh_runtime(cx);
                    }
                    Err(error) => {
                        this.phase = PanelPhase::Fatal(
                            "World analyst session could not close cleanly".into(),
                        );
                        this.last_error = Some(error);
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn recover_from_fatal(&mut self, cx: &mut Context<Self>) {
        if self.busy
            || !reset_fatal_recovery_state(
                &mut self.phase,
                &mut self.history,
                &mut self.runtime,
                &mut self.last_error,
            )
        {
            return;
        }

        self.session.take();
        self.cancellation.take();
        self.cancel_requested = false;
        self.runtime_checking = false;
        self.refresh_runtime(cx);
    }

    fn choose_right(&mut self, id: WorldDocumentId, cx: &mut Context<Self>) {
        if self.busy || self.settings_busy || self.session.is_some() || id == self.left {
            return;
        }
        self.right = id;
        self.last_error = None;
        cx.notify();
    }

    fn render_program_row(
        &self,
        program: analyst_runtime::AnalystRuntimeProgram,
        cx: &mut Context<Self>,
    ) -> Div {
        let label = runtime_program_label(program);
        let Some(status) = self.runtime.as_ref() else {
            return div()
                .text_xs()
                .text_color(rgb(0x777770))
                .child(format!("{label} · waiting for runtime check"));
        };
        let Some(selections) = status.selections.as_ref() else {
            return div().text_xs().text_color(rgb(0x777770)).child(format!(
                "{label} · settings unavailable until runtime settings load"
            ));
        };
        let selection = match program {
            analyst_runtime::AnalystRuntimeProgram::Node => &selections.node,
            analyst_runtime::AnalystRuntimeProgram::Pi => &selections.pi,
        };
        let persisted = status.settings.as_ref().and_then(|settings| match program {
            analyst_runtime::AnalystRuntimeProgram::Node => settings.node_program.as_ref(),
            analyst_runtime::AnalystRuntimeProgram::Pi => settings.pi_program.as_ref(),
        });
        let environment_controlled = selection.source == DesktopAnalystProgramSource::Environment;
        let controls_enabled =
            !self.busy && !self.settings_busy && !self.runtime_checking && self.session.is_none();

        let mut actions = div().flex().gap_2().items_center();
        if environment_controlled {
            actions = actions.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777770))
                    .child("Environment controlled"),
            );
        } else {
            let mut choose = div()
                .id(SharedString::from(format!(
                    "choose-{}-analyst-runtime",
                    runtime_program_slug(program)
                )))
                .px_3()
                .p_1()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xb8b2a8))
                .bg(rgb(0xffffff))
                .text_xs()
                .child("Choose…");
            if controls_enabled {
                choose = choose.cursor_pointer().on_click(
                    cx.listener(move |this, _, _, cx| this.configure_program(program, cx)),
                );
            } else {
                choose = choose.text_color(rgb(0x999990));
            }
            actions = actions.child(choose);

            if persisted.is_some() {
                let mut clear = div()
                    .id(SharedString::from(format!(
                        "clear-{}-analyst-runtime",
                        runtime_program_slug(program)
                    )))
                    .px_3()
                    .p_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xd8d8d2))
                    .bg(rgb(0xf7f7f3))
                    .text_xs()
                    .child("Clear saved path");
                if controls_enabled {
                    clear = clear.cursor_pointer().on_click(
                        cx.listener(move |this, _, _, cx| this.clear_program(program, cx)),
                    );
                } else {
                    clear = clear.text_color(rgb(0x999990));
                }
                actions = actions.child(clear);
            }
        }

        div()
            .w_full()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xe0e0db))
            .bg(rgb(0xfafaf8))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_xs().child(format!(
                        "{label} · {}",
                        runtime_program_source_label(selection.source)
                    )))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x777770))
                            .child(selection.program.display().to_string()),
                    ),
            )
            .child(actions)
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

        let recheck_enabled =
            !self.busy && !self.settings_busy && !self.runtime_checking && self.session.is_none();
        let mut recheck = div()
            .id("retry-analyst-runtime")
            .px_3()
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xb8b2a8))
            .bg(rgb(0xffffff))
            .text_xs()
            .child("Recheck");
        if recheck_enabled {
            recheck = recheck
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.refresh_runtime(cx)));
        } else {
            recheck = recheck.text_color(rgb(0x999990));
        }

        let runtime_status = if self.runtime_checking {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x666660))
                        .child("Checking analyst runtime readiness…"),
                )
                .child(recheck)
        } else {
            let message = match self.runtime.as_ref().map(|status| &status.readiness) {
                Some(DesktopAnalystRuntimeReadiness::Ready { .. }) => div()
                    .flex_1()
                    .text_xs()
                    .text_color(rgb(0x4d6748))
                    .child("Analyst runtime ready · Node and Pi resolved"),
                Some(DesktopAnalystRuntimeReadiness::Unavailable { issue }) => div()
                    .flex_1()
                    .text_xs()
                    .text_color(rgb(0x9b4a42))
                    .child(issue.message().to_owned()),
                None => div()
                    .flex_1()
                    .text_xs()
                    .text_color(rgb(0x9b4a42))
                    .child("Analyst runtime readiness is unavailable. Recheck the runtime."),
            };
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(message)
                .child(recheck)
        };

        let runtime_controls = div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.render_program_row(analyst_runtime::AnalystRuntimeProgram::Node, cx))
            .child(self.render_program_row(analyst_runtime::AnalystRuntimeProgram::Pi, cx));

        let can_start = !self.busy
            && !self.settings_busy
            && !self.runtime_checking
            && self
                .runtime
                .as_ref()
                .is_some_and(|status| status.readiness.is_ready())
            && self.left != self.right;
        let mut start = div()
            .id("start-world-analyst")
            .px_4()
            .p_2()
            .rounded_md()
            .border_1()
            .text_sm()
            .child(if self.busy {
                "Starting…"
            } else if self.settings_busy {
                "Saving runtime path…"
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
            .child(runtime_controls)
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

        let can_cancel = can_cancel_analysis(
            &self.phase,
            self.busy,
            self.session.is_some(),
            self.cancellation.is_some(),
            self.cancel_requested,
        );
        let cancelling =
            self.busy && matches!(self.phase, PanelPhase::Active) && self.cancel_requested;
        let mut composer_actions = div().flex().gap_2().items_center().child(ask);
        if can_cancel || cancelling {
            let mut cancel = div()
                .id("cancel-world-analyst")
                .px_4()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xc9aaa1))
                .bg(rgb(0xfff8f6))
                .text_sm()
                .child(if cancelling {
                    "Cancelling…"
                } else {
                    "Cancel analysis"
                });
            if can_cancel {
                cancel = cancel
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_analysis(cx)));
            } else {
                cancel = cancel.text_color(rgb(0x999990));
            }
            composer_actions = composer_actions.child(cancel);
        }

        let composer = div()
            .w_full()
            .flex()
            .gap_2()
            .items_center()
            .child(div().flex_1().child(self.question.clone()))
            .child(composer_actions);

        let mut snapshot_status = div().flex().gap_2().items_center().child(
            div()
                .text_xs()
                .text_color(rgb(0x777770))
                .child("Read-only · fixed snapshot pair"),
        );
        if !self.busy && matches!(self.phase, PanelPhase::Active) {
            snapshot_status = snapshot_status.child(
                div()
                    .id("new-world-analyst-comparison")
                    .cursor_pointer()
                    .px_3()
                    .p_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xb8b2a8))
                    .bg(rgb(0xffffff))
                    .text_xs()
                    .child("New comparison")
                    .on_click(cx.listener(|this, _, _, cx| this.start_new_comparison(cx))),
            );
        }

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
                    .child(snapshot_status),
            )
            .child(history)
            .child(composer);
        if let PanelPhase::Fatal(message) = &self.phase {
            let mut recover = div()
                .id("recover-world-analyst")
                .px_4()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xb8b2a8))
                .bg(rgb(0xffffff))
                .text_sm()
                .child("Recover and recheck runtime");
            if !self.busy {
                recover = recover
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.recover_from_fatal(cx)));
            } else {
                recover = recover.text_color(rgb(0x999990));
            }
            body = body.child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(rgb(0xfff2f0))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x9b4a42))
                            .child(format!("Analyst session ended: {message}")),
                    )
                    .child(recover),
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
        if let Some(failed_question) = &self.failed_question {
            root = root.child(
                div()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0xe1b4aa))
                    .bg(rgb(0xfff8f6))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x9b4a42))
                            .child("Failed question"),
                    )
                    .child(div().text_sm().child(failed_question.clone())),
            );
        }
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

fn runtime_program_label(program: analyst_runtime::AnalystRuntimeProgram) -> &'static str {
    match program {
        analyst_runtime::AnalystRuntimeProgram::Node => "Node",
        analyst_runtime::AnalystRuntimeProgram::Pi => "Pi",
    }
}

fn runtime_program_slug(program: analyst_runtime::AnalystRuntimeProgram) -> &'static str {
    match program {
        analyst_runtime::AnalystRuntimeProgram::Node => "node",
        analyst_runtime::AnalystRuntimeProgram::Pi => "pi",
    }
}

fn runtime_program_source_label(source: DesktopAnalystProgramSource) -> &'static str {
    match source {
        DesktopAnalystProgramSource::Environment => "Environment override",
        DesktopAnalystProgramSource::Persisted => "Saved path",
        DesktopAnalystProgramSource::Default => "PATH/default",
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

fn can_cancel_analysis(
    phase: &PanelPhase,
    busy: bool,
    has_session: bool,
    has_cancellation: bool,
    cancel_requested: bool,
) -> bool {
    busy && matches!(phase, PanelPhase::Active)
        && !has_session
        && has_cancellation
        && !cancel_requested
}

fn should_clear_completed_prompt(
    current: &str,
    submitted: &str,
    succeeded: bool,
    cancel_requested: bool,
) -> bool {
    succeeded && !cancel_requested && current == submitted
}

fn failed_question_after_turn(
    submitted: &str,
    succeeded: bool,
    cancel_requested: bool,
) -> Option<String> {
    (!succeeded || cancel_requested).then(|| submitted.to_owned())
}

fn reset_new_comparison_state<T>(
    phase: &mut PanelPhase,
    history: &mut Vec<PanelTurn>,
    runtime: &mut Option<T>,
    failed_question: &mut Option<String>,
    last_error: &mut Option<String>,
) -> bool {
    if !matches!(phase, PanelPhase::Active) {
        return false;
    }
    *phase = PanelPhase::Setup;
    history.clear();
    *runtime = None;
    *failed_question = None;
    *last_error = None;
    true
}

fn reset_fatal_recovery_state<T>(
    phase: &mut PanelPhase,
    history: &mut Vec<PanelTurn>,
    runtime: &mut Option<T>,
    last_error: &mut Option<String>,
) -> bool {
    if !matches!(phase, PanelPhase::Fatal(_)) {
        return false;
    }
    *phase = PanelPhase::Setup;
    history.clear();
    *runtime = None;
    *last_error = None;
    true
}

fn snapshot_history(session: &DesktopAnalystSession) -> Vec<PanelTurn> {
    session
        .exchanges()
        .iter()
        .map(|exchange| {
            let turn = exchange.turn();
            PanelTurn {
                question: exchange.prompt().to_owned(),
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
            }
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
        .child(
            div()
                .p_2()
                .rounded_md()
                .bg(rgb(0xf2f6ff))
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(rgb(0x66718a)).child("Question"))
                .child(div().text_sm().child(turn.question.clone())),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(rgb(0x777770)).child("Analyst"))
                .child(div().text_sm().child(turn.answer.clone())),
        );

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

    fn panel_turn(question: &str, answer: &str) -> PanelTurn {
        PanelTurn {
            question: question.to_owned(),
            answer: answer.to_owned(),
            tool_calls: Vec::new(),
            runtime_errors: Vec::new(),
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
    fn cancel_control_requires_a_real_in_flight_ask_with_a_live_handle() {
        assert!(can_cancel_analysis(
            &PanelPhase::Active,
            true,
            false,
            true,
            false,
        ));
        assert!(!can_cancel_analysis(
            &PanelPhase::Active,
            false,
            false,
            true,
            false,
        ));
        assert!(!can_cancel_analysis(
            &PanelPhase::Starting,
            true,
            false,
            true,
            false,
        ));
        assert!(!can_cancel_analysis(
            &PanelPhase::Active,
            true,
            true,
            true,
            false,
        ));
        assert!(!can_cancel_analysis(
            &PanelPhase::Active,
            true,
            false,
            false,
            false,
        ));
        assert!(!can_cancel_analysis(
            &PanelPhase::Active,
            true,
            false,
            true,
            true,
        ));
    }

    #[test]
    fn completed_prompt_is_cleared_only_after_uncancelled_success_if_the_draft_is_unchanged() {
        assert!(should_clear_completed_prompt(
            "Why did this diverge?",
            "Why did this diverge?",
            true,
            false,
        ));
        assert!(!should_clear_completed_prompt(
            "Why did this diverge?",
            "Why did this diverge?",
            false,
            false,
        ));
        assert!(!should_clear_completed_prompt(
            "A follow-up draft",
            "Why did this diverge?",
            true,
            false,
        ));
        assert!(!should_clear_completed_prompt(
            "Why did this diverge?",
            "Why did this diverge?",
            true,
            true,
        ));
    }

    #[test]
    fn failed_or_cancelled_submission_is_retained_separately_from_a_newer_draft() {
        assert_eq!(
            failed_question_after_turn("Why did the old ask fail?", false, false).as_deref(),
            Some("Why did the old ask fail?")
        );
        assert_eq!(
            failed_question_after_turn("Why did the old ask fail?", true, false),
            None
        );
        assert_eq!(
            failed_question_after_turn("Why did the cancelled ask stop?", true, true).as_deref(),
            Some("Why did the cancelled ask stop?")
        );
    }

    #[test]
    fn new_comparison_clears_snapshot_pair_history_and_stale_readiness_after_close() {
        let mut phase = PanelPhase::Active;
        let mut history = vec![panel_turn("old question", "old answer")];
        let mut runtime = Some("stale readiness");
        let mut failed_question = Some("old failed question".into());
        let mut last_error = Some("old warning".into());

        assert!(reset_new_comparison_state(
            &mut phase,
            &mut history,
            &mut runtime,
            &mut failed_question,
            &mut last_error,
        ));
        assert_eq!(phase, PanelPhase::Setup);
        assert!(history.is_empty());
        assert_eq!(runtime, None);
        assert_eq!(failed_question, None);
        assert_eq!(last_error, None);
    }

    #[test]
    fn new_comparison_transition_does_not_reset_non_active_state() {
        let mut phase = PanelPhase::Fatal("transport ended".into());
        let mut history = vec![panel_turn("question", "answer")];
        let mut runtime = Some("current readiness");
        let mut failed_question = Some("failed question".into());
        let mut last_error = Some("transport ended".into());

        assert!(!reset_new_comparison_state(
            &mut phase,
            &mut history,
            &mut runtime,
            &mut failed_question,
            &mut last_error,
        ));
        assert!(matches!(phase, PanelPhase::Fatal(_)));
        assert_eq!(history.len(), 1);
        assert_eq!(runtime, Some("current readiness"));
        assert_eq!(failed_question.as_deref(), Some("failed question"));
        assert_eq!(last_error.as_deref(), Some("transport ended"));
    }

    #[test]
    fn fatal_recovery_clears_snapshot_pair_history_and_stale_readiness() {
        let mut phase = PanelPhase::Fatal("transport ended".into());
        let mut history = vec![panel_turn("old question", "old answer")];
        let mut runtime = Some("stale readiness");
        let mut last_error = Some("transport ended".into());

        assert!(reset_fatal_recovery_state(
            &mut phase,
            &mut history,
            &mut runtime,
            &mut last_error,
        ));
        assert_eq!(phase, PanelPhase::Setup);
        assert!(history.is_empty());
        assert_eq!(runtime, None);
        assert_eq!(last_error, None);
    }

    #[test]
    fn fatal_recovery_transition_is_explicit_and_does_not_reset_active_state() {
        let mut phase = PanelPhase::Active;
        let mut history = vec![panel_turn("question", "answer")];
        let mut runtime = Some("current readiness");
        let mut last_error = Some("recoverable warning".into());

        assert!(!reset_fatal_recovery_state(
            &mut phase,
            &mut history,
            &mut runtime,
            &mut last_error,
        ));
        assert_eq!(phase, PanelPhase::Active);
        assert_eq!(history.len(), 1);
        assert_eq!(runtime, Some("current readiness"));
        assert_eq!(last_error.as_deref(), Some("recoverable warning"));
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
            ["command", "::new"].concat(),
            ["var_os", "(\"path\")"].concat(),
            ["std", "::fs"].concat(),
            ["file", "::open"].concat(),
            ["settings_", "path"].concat(),
        ];
        for token in forbidden {
            assert!(
                !source.contains(&token),
                "analyst panel contains forbidden lower-layer token {token}"
            );
        }
    }
}
