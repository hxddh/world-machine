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
    DesktopAnalystCancellation, DesktopAnalystCancellationOutcome, DesktopAnalystEvidenceScope,
    DesktopAnalystSession, DesktopAnalystSessionError, DesktopAnalystState,
};
use world_machine_desktop::analyst_settings::DesktopAnalystProgramSource;

#[derive(Clone, Debug, Eq, PartialEq)]
enum PanelPhase {
    Setup,
    Starting,
    Active,
    Fatal(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PanelAskSource {
    Composer,
    FailedQuestion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PanelPairSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PanelTurn {
    question: String,
    answer: String,
    tool_calls: Vec<PanelToolCall>,
    runtime_errors: Vec<String>,
}

const ANALYST_TOOL_PAYLOAD_PREVIEW_BYTES: usize = 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
struct PanelPayloadPreview {
    text: String,
    truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PanelToolCall {
    tool: String,
    input: PanelPayloadPreview,
    output: PanelPayloadPreview,
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
                Ok(()) => DocumentStatus::success("Opened World analyst · loading saved Worlds"),
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
) -> Result<(), String> {
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
    let bounds = Bounds::centered(None, size(px(920.0), px(820.0)), cx);

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |_, cx| cx.new(|cx| AnalystPanelView::new(library, left, cx)),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
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
    catalog_refreshing: bool,
    catalog_refresh_generation: u64,
    settings_busy: bool,
    session: Option<DesktopAnalystSession>,
    cancellation: Option<DesktopAnalystCancellation>,
    filter: Entity<AnalystTextInput>,
    question: Entity<AnalystTextInput>,
    phase: PanelPhase,
    busy: bool,
    cancel_requested: bool,
    history: Vec<PanelTurn>,
    failed_question: Option<String>,
    failed_question_scope: Option<DesktopAnalystEvidenceScope>,
    last_error: Option<String>,
}

impl AnalystPanelView {
    fn new(library: Arc<WorldLibrary>, left: WorldDocumentId, cx: &mut Context<Self>) -> Self {
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
        let filter = cx.new(|cx| AnalystTextInput::new("Filter saved Worlds…", cx));
        cx.observe(&filter, |_, _, cx| cx.notify()).detach();
        let question = cx.new(|cx| AnalystTextInput::new("Ask why these Worlds differ…", cx));
        let right = left.clone();
        let mut view = Self {
            library,
            documents: Vec::new(),
            left,
            right,
            runtime: None,
            runtime_checking: false,
            catalog_refreshing: false,
            catalog_refresh_generation: 0,
            settings_busy: false,
            session: None,
            cancellation: None,
            filter,
            question,
            phase: PanelPhase::Setup,
            busy: false,
            cancel_requested: false,
            history: Vec::new(),
            failed_question: None,
            failed_question_scope: None,
            last_error: None,
        };
        view.refresh_saved_world_catalog(cx);
        view
    }

    fn refresh_runtime(&mut self, cx: &mut Context<Self>) {
        if self.busy
            || self.settings_busy
            || self.session.is_some()
            || self.runtime_checking
            || self.catalog_refreshing
        {
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

    fn refresh_saved_world_catalog(&mut self, cx: &mut Context<Self>) {
        if self.busy
            || self.settings_busy
            || self.session.is_some()
            || self.runtime_checking
            || self.catalog_refreshing
            || !matches!(self.phase, PanelPhase::Setup)
        {
            return;
        }

        self.catalog_refresh_generation = self.catalog_refresh_generation.wrapping_add(1);
        let generation = self.catalog_refresh_generation;
        self.catalog_refreshing = true;
        self.busy = true;
        self.runtime = None;
        self.last_error = None;
        let library = Arc::clone(&self.library);
        cx.notify();

        let task = cx
            .background_executor()
            .spawn(async move { library.list().map_err(|error| error.to_string()) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if !can_apply_catalog_refresh_completion(
                    &this.phase,
                    this.busy,
                    this.catalog_refreshing,
                    generation,
                    this.catalog_refresh_generation,
                    this.session.is_some(),
                ) {
                    return;
                }

                this.catalog_refreshing = false;
                this.busy = false;
                match result {
                    Ok(documents) => {
                        if documents.len() < 2 {
                            this.documents = documents;
                            this.last_error = Some(
                                "World analyst needs at least two saved Worlds. Create or import another saved World, then Recheck."
                                    .into(),
                            );
                            cx.notify();
                            return;
                        }

                        if !documents.iter().any(|document| document.id == this.left) {
                            let error = "The selected left World is no longer saved. Choose another saved World for Left to continue."
                                .to_string();
                            this.documents = documents;
                            this.last_error = Some(error.clone());
                            this.refresh_runtime(cx);
                            this.last_error = Some(error);
                            cx.notify();
                            return;
                        }

                        let Some(next_right) =
                            refreshed_right_for(&this.left, &this.right, &documents)
                        else {
                            this.documents = documents;
                            this.last_error = Some(
                                "World analyst needs at least two distinct saved Worlds. Choose a different World for Right."
                                    .into(),
                            );
                            cx.notify();
                            return;
                        };

                        if next_right != this.right {
                            let changed = update_pending_pair_selection(
                                PanelPairSide::Right,
                                &mut this.left,
                                &mut this.right,
                                &mut this.failed_question,
                                &mut this.failed_question_scope,
                                next_right,
                            );
                            debug_assert!(changed);
                        }
                        this.documents = documents;
                        this.last_error = None;
                        this.refresh_runtime(cx);
                    }
                    Err(error) => {
                        this.documents.clear();
                        this.last_error = Some(format!("Could not refresh saved Worlds: {error}"));
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    fn configure_program(
        &mut self,
        program: analyst_runtime::AnalystRuntimeProgram,
        cx: &mut Context<Self>,
    ) {
        if self.busy
            || self.settings_busy
            || self.session.is_some()
            || self.runtime_checking
            || self.catalog_refreshing
        {
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
        if self.busy
            || self.settings_busy
            || self.session.is_some()
            || self.runtime_checking
            || self.catalog_refreshing
        {
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
        if self.busy
            || self.settings_busy
            || self.catalog_refreshing
            || !catalog_pair_is_available(&self.left, &self.right, &self.documents)
        {
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
        });
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let mut result = Some(task.await);
            let update = this.update(cx, |this, cx| {
                if !can_apply_session_start_completion(
                    &this.phase,
                    this.busy,
                    this.session.is_some(),
                ) {
                    return;
                }
                let result = result
                    .take()
                    .expect("analyst startup result should be consumed once");
                this.busy = false;
                this.cancel_requested = false;
                let mut refresh_catalog = false;
                let mut refresh_runtime = false;
                match result {
                    Ok(session) => {
                        if this.failed_question.is_some()
                            && !retained_failed_question_matches_scope(
                                this.failed_question_scope.as_ref(),
                                session.evidence_scope(),
                            )
                        {
                            this.failed_question = None;
                            this.failed_question_scope = None;
                        }
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
                        refresh_catalog = startup_error_requires_catalog_refresh(&error);
                        refresh_runtime = startup_error_requires_runtime_refresh(&error);
                        debug_assert!(!(refresh_catalog && refresh_runtime));
                        this.last_error = Some(error.to_string());
                    }
                }
                if refresh_catalog {
                    this.refresh_saved_world_catalog(cx);
                } else if refresh_runtime {
                    this.refresh_runtime(cx);
                } else {
                    cx.notify();
                }
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
        self.start_ask(submitted_question, prompt, PanelAskSource::Composer, cx);
    }

    fn retry_failed_question(&mut self, cx: &mut Context<Self>) {
        if !can_retry_failed_question(
            &self.phase,
            self.busy,
            self.session.is_some(),
            self.failed_question.is_some(),
            self.cancel_requested,
        ) {
            return;
        }
        let Some(submitted_question) = self.failed_question.clone() else {
            return;
        };
        let prompt = submitted_question.clone();
        self.start_ask(
            submitted_question,
            prompt,
            PanelAskSource::FailedQuestion,
            cx,
        );
    }

    fn dismiss_failed_question(&mut self, cx: &mut Context<Self>) {
        if !can_dismiss_failed_question(
            &self.phase,
            self.busy,
            self.runtime_checking,
            self.settings_busy,
            self.failed_question.is_some(),
        ) {
            return;
        }
        self.failed_question = None;
        self.failed_question_scope = None;
        cx.notify();
    }

    fn start_ask(
        &mut self,
        submitted_question: String,
        prompt: String,
        source: PanelAskSource,
        cx: &mut Context<Self>,
    ) {
        if self.busy || self.cancel_requested || !matches!(self.phase, PanelPhase::Active) {
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
            (session, result, submitted_question, source)
        });
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let mut completed = Some(task.await);
            let update = this.update(cx, |this, cx| {
                let (session, result, submitted_question, source) = completed
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
                if source == PanelAskSource::Composer {
                    let current_question = this.question.read(cx).text().to_owned();
                    if should_clear_completed_prompt(
                        source,
                        &current_question,
                        &submitted_question,
                        succeeded,
                        cancel_requested,
                    ) {
                        this.question.update(cx, |input, cx| input.clear(cx));
                    }
                }
                let next_failed_question = failed_question_after_completion(
                    this.failed_question.as_deref(),
                    source,
                    &submitted_question,
                    succeeded,
                    cancel_requested,
                    forced_fatal,
                );
                let next_failed_question_scope = failed_question_scope_after_completion(
                    this.failed_question_scope.as_ref(),
                    source,
                    succeeded,
                    cancel_requested,
                    forced_fatal,
                    session.evidence_scope(),
                );
                this.failed_question = next_failed_question;
                this.failed_question_scope = next_failed_question_scope;
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
                if let Some((mut session, _, _, _)) = completed.take() {
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
                            &mut this.failed_question_scope,
                            &mut this.last_error,
                        );
                        debug_assert!(transitioned);
                        this.runtime_checking = false;
                        this.refresh_saved_world_catalog(cx);
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
        self.refresh_saved_world_catalog(cx);
    }

    fn choose_world(&mut self, side: PanelPairSide, id: WorldDocumentId, cx: &mut Context<Self>) {
        if self.busy
            || self.settings_busy
            || self.runtime_checking
            || self.catalog_refreshing
            || self.session.is_some()
            || !matches!(self.phase, PanelPhase::Setup)
            || !self.documents.iter().any(|document| document.id == id)
        {
            return;
        }
        if side == PanelPairSide::Right
            && !self
                .documents
                .iter()
                .any(|document| document.id == self.left)
        {
            return;
        }
        if !update_pending_pair_selection(
            side,
            &mut self.left,
            &mut self.right,
            &mut self.failed_question,
            &mut self.failed_question_scope,
            id,
        ) {
            return;
        }
        self.last_error = None;
        cx.notify();
    }

    fn swap_worlds(&mut self, cx: &mut Context<Self>) {
        if !can_swap_pending_pair(
            &self.phase,
            self.busy,
            self.settings_busy,
            self.runtime_checking,
            self.catalog_refreshing,
            self.session.is_some(),
            &self.left,
            &self.right,
            &self.documents,
        ) || !swap_pending_pair_selection(
            &mut self.left,
            &mut self.right,
            &mut self.failed_question,
            &mut self.failed_question_scope,
        ) {
            return;
        }
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
        let controls_enabled = !self.busy
            && !self.settings_busy
            && !self.runtime_checking
            && !self.catalog_refreshing
            && self.session.is_none();

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

    fn render_world_selector(
        &self,
        side: PanelPairSide,
        filter_query: &str,
        cx: &mut Context<Self>,
    ) -> Div {
        let (label, selected, opposite) = match side {
            PanelPairSide::Left => ("Left", &self.left, &self.right),
            PanelPairSide::Right => ("Right", &self.right, &self.left),
        };
        let left_available = self
            .documents
            .iter()
            .any(|document| document.id == self.left);
        let selector_enabled = matches!(self.phase, PanelPhase::Setup)
            && !self.busy
            && !self.settings_busy
            && !self.runtime_checking
            && !self.catalog_refreshing
            && self.session.is_none()
            && self.documents.len() >= 2
            && (side == PanelPairSide::Left || left_available);
        let slug = pair_side_slug(side);
        let mut worlds = div()
            .id(SharedString::from(format!("analyst-{slug}-world-list")))
            .w_full()
            .max_h(px(260.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();

        for document in &self.documents {
            let id = document.id.clone();
            let is_selected = id == *selected;
            let is_opposite = id == *opposite;
            if !document_visible_for_filter(document, selected, opposite, filter_query) {
                continue;
            }
            let title = document_title(document);
            let document_id_label = document_id_label(document);
            let summary = document
                .display_summary
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("Saved World");
            let mut card = div()
                .id(SharedString::from(format!("analyst-{slug}-world-{id}")))
                .w_full()
                .min_w(px(0.0))
                .p_2()
                .rounded_md()
                .border_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "analyst-{slug}-world-{id}-title"
                        )))
                        .w_full()
                        .min_w(px(0.0))
                        .overflow_x_scroll()
                        .text_sm()
                        .child(title),
                );
            if let Some(document_id_label) = document_id_label {
                card = card.child(
                    div()
                        .id(SharedString::from(format!(
                            "analyst-{slug}-world-{id}-stable-id"
                        )))
                        .w_full()
                        .min_w(px(0.0))
                        .overflow_x_scroll()
                        .text_xs()
                        .text_color(rgb(0x777770))
                        .child(document_id_label),
                );
            }
            card = card
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x777770))
                        .child(document_pack_label(document)),
                )
                .child(div().text_xs().text_color(rgb(0x777770)).child(format!(
                    "{} · t={} · {} events",
                    summary, document.world_time, document.event_count
                )));
            card = if is_selected {
                card.border_color(rgb(0x6684c4)).bg(rgb(0xf2f6ff))
            } else if is_opposite {
                card.border_color(rgb(0xe0e0db))
                    .bg(rgb(0xf7f7f3))
                    .text_color(rgb(0x999990))
            } else {
                card.border_color(rgb(0xd8d8d2)).bg(rgb(0xffffff))
            };
            if selector_enabled && !is_selected && !is_opposite {
                worlds = worlds.child(card.cursor_pointer().on_click(
                    cx.listener(move |this, _, _, cx| this.choose_world(side, id.clone(), cx)),
                ));
            } else {
                worlds = worlds.child(card);
            }
        }

        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id(SharedString::from(format!(
                        "analyst-{slug}-selected-world-identity"
                    )))
                    .w_full()
                    .min_w(px(0.0))
                    .overflow_x_scroll()
                    .text_sm()
                    .child(format!(
                        "{label} · {}",
                        document_identity_label(selected, &self.documents)
                    )),
            )
            .child(worlds)
    }

    fn render_setup(&self, cx: &mut Context<Self>) -> Div {
        let filter_query = normalize_filter_query(self.filter.read(cx).text());
        let has_other_match = self.documents.iter().any(|document| {
            document.id != self.left
                && document.id != self.right
                && document_matches_filter(document, &filter_query)
        });
        let mut filter_control = div()
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(self.filter.clone());
        if !filter_query.is_empty() && !has_other_match {
            filter_control = filter_control.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x777770))
                    .child("No other saved Worlds match this filter"),
            );
        }

        let can_swap = can_swap_pending_pair(
            &self.phase,
            self.busy,
            self.settings_busy,
            self.runtime_checking,
            self.catalog_refreshing,
            self.session.is_some(),
            &self.left,
            &self.right,
            &self.documents,
        );
        let mut swap = div()
            .id("swap-world-analyst-pair")
            .px_3()
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(rgb(0xb8b2a8))
            .bg(rgb(0xffffff))
            .text_xs()
            .child("Swap sides");
        if can_swap {
            swap = swap
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.swap_worlds(cx)));
        } else {
            swap = swap.text_color(rgb(0x999990));
        }
        let pair_header = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x666660))
                    .child("Choose two distinct saved Worlds to compare."),
            )
            .child(swap);
        let selectors = div()
            .w_full()
            .flex()
            .gap_3()
            .child(self.render_world_selector(PanelPairSide::Left, &filter_query, cx))
            .child(self.render_world_selector(PanelPairSide::Right, &filter_query, cx));

        let recheck_enabled = !self.busy
            && !self.settings_busy
            && !self.runtime_checking
            && !self.catalog_refreshing
            && self.session.is_none();
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
                .on_click(cx.listener(|this, _, _, cx| this.refresh_saved_world_catalog(cx)));
        } else {
            recheck = recheck.text_color(rgb(0x999990));
        }

        let runtime_status = if self.catalog_refreshing {
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x666660))
                        .child("Refreshing saved Worlds…"),
                )
                .child(recheck)
        } else if self.runtime_checking {
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
            && !self.catalog_refreshing
            && self
                .runtime
                .as_ref()
                .is_some_and(|status| status.readiness.is_ready())
            && catalog_pair_is_available(&self.left, &self.right, &self.documents);
        let mut start = div()
            .id("start-world-analyst")
            .px_4()
            .p_2()
            .rounded_md()
            .border_1()
            .text_sm()
            .child(if self.catalog_refreshing {
                "Refreshing saved Worlds…"
            } else if self.busy {
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
            .child(pair_header)
            .child(filter_control)
            .child(selectors)
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
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .id("analyst-active-pair-identity")
                            .w_full()
                            .min_w(px(0.0))
                            .overflow_x_scroll()
                            .text_sm()
                            .child(pair_identity_header(
                                &self.left,
                                &self.right,
                                &self.documents,
                            )),
                    )
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
            let mut failed = div()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(0xe1b4aa))
                .bg(rgb(0xfff8f6))
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x9b4a42))
                        .child("Failed question"),
                )
                .child(div().text_sm().child(failed_question.clone()));
            let can_retry = can_retry_failed_question(
                &self.phase,
                self.busy,
                self.session.is_some(),
                true,
                self.cancel_requested,
            );
            let can_dismiss = can_dismiss_failed_question(
                &self.phase,
                self.busy,
                self.runtime_checking,
                self.settings_busy,
                true,
            );
            if can_retry || can_dismiss {
                let mut actions = div().flex().gap_2().items_center();
                if can_retry {
                    actions = actions.child(
                        div()
                            .id("retry-failed-world-analyst-question")
                            .cursor_pointer()
                            .px_3()
                            .p_1()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0xc9aaa1))
                            .bg(rgb(0xffffff))
                            .text_xs()
                            .child("Retry failed question")
                            .on_click(cx.listener(|this, _, _, cx| this.retry_failed_question(cx))),
                    );
                }
                if can_dismiss {
                    actions = actions.child(
                        div()
                            .id("dismiss-failed-world-analyst-question")
                            .cursor_pointer()
                            .px_3()
                            .p_1()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0xb8b2a8))
                            .bg(rgb(0xffffff))
                            .text_xs()
                            .child("Dismiss failed question")
                            .on_click(
                                cx.listener(|this, _, _, cx| this.dismiss_failed_question(cx)),
                            ),
                    );
                }
                failed = failed.child(actions);
            }
            root = root.child(failed);
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

fn pair_side_slug(side: PanelPairSide) -> &'static str {
    match side {
        PanelPairSide::Left => "left",
        PanelPairSide::Right => "right",
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

fn document_id_label(document: &WorldDocumentSummary) -> Option<String> {
    let id = document.id.to_string();
    (document_title(document) != id).then(|| format!("ID {id}"))
}

fn document_identity_label(id: &WorldDocumentId, documents: &[WorldDocumentSummary]) -> String {
    documents
        .iter()
        .find(|document| document.id == *id)
        .map(|document| {
            let title = document_title(document);
            match document_id_label(document) {
                Some(id_label) => format!("{title} · {id_label}"),
                None => title,
            }
        })
        .unwrap_or_else(|| id.to_string())
}

fn pair_identity_header(
    left: &WorldDocumentId,
    right: &WorldDocumentId,
    documents: &[WorldDocumentSummary],
) -> String {
    format!(
        "{} ↔ {}",
        document_identity_label(left, documents),
        document_identity_label(right, documents)
    )
}

fn document_pack_label(document: &WorldDocumentSummary) -> String {
    format!("Pack {} · {}", document.pack.id, document.pack.version)
}

fn normalize_filter_query(query: &str) -> String {
    query.trim().to_lowercase()
}

fn document_matches_filter(document: &WorldDocumentSummary, filter_query: &str) -> bool {
    if filter_query.is_empty() {
        return true;
    }

    document_title(document)
        .to_lowercase()
        .contains(filter_query)
        || document
            .id
            .to_string()
            .to_lowercase()
            .contains(filter_query)
        || document.pack.id.to_lowercase().contains(filter_query)
        || document.pack.version.to_lowercase().contains(filter_query)
        || document
            .display_summary
            .as_deref()
            .map(str::trim)
            .filter(|summary| !summary.is_empty())
            .is_some_and(|summary| summary.to_lowercase().contains(filter_query))
}

fn document_visible_for_filter(
    document: &WorldDocumentSummary,
    selected: &WorldDocumentId,
    opposite: &WorldDocumentId,
    filter_query: &str,
) -> bool {
    document.id == *selected
        || document.id == *opposite
        || document_matches_filter(document, filter_query)
}

fn catalog_pair_is_available(
    left: &WorldDocumentId,
    right: &WorldDocumentId,
    documents: &[WorldDocumentSummary],
) -> bool {
    left != right
        && documents.iter().any(|document| document.id == *left)
        && documents.iter().any(|document| document.id == *right)
}

fn can_swap_pending_pair(
    phase: &PanelPhase,
    busy: bool,
    settings_busy: bool,
    runtime_checking: bool,
    catalog_refreshing: bool,
    has_session: bool,
    left: &WorldDocumentId,
    right: &WorldDocumentId,
    documents: &[WorldDocumentSummary],
) -> bool {
    matches!(phase, PanelPhase::Setup)
        && !busy
        && !settings_busy
        && !runtime_checking
        && !catalog_refreshing
        && !has_session
        && catalog_pair_is_available(left, right, documents)
}

fn refreshed_right_for(
    left: &WorldDocumentId,
    current_right: &WorldDocumentId,
    documents: &[WorldDocumentSummary],
) -> Option<WorldDocumentId> {
    if catalog_pair_is_available(left, current_right, documents) {
        Some(current_right.clone())
    } else {
        default_right_for(left, documents)
    }
}

fn can_apply_catalog_refresh_completion(
    phase: &PanelPhase,
    busy: bool,
    catalog_refreshing: bool,
    generation: u64,
    current_generation: u64,
    has_session: bool,
) -> bool {
    matches!(phase, PanelPhase::Setup)
        && busy
        && catalog_refreshing
        && generation == current_generation
        && !has_session
}

fn can_apply_session_start_completion(phase: &PanelPhase, busy: bool, has_session: bool) -> bool {
    matches!(phase, PanelPhase::Starting) && busy && !has_session
}

fn startup_error_requires_catalog_refresh(error: &DesktopAnalystSessionError) -> bool {
    matches!(
        error,
        DesktopAnalystSessionError::MissingWorld { .. }
            | DesktopAnalystSessionError::LoadWorld { .. }
    )
}

fn startup_error_requires_runtime_refresh(error: &DesktopAnalystSessionError) -> bool {
    match error {
        DesktopAnalystSessionError::Spawn(_) => true,
        DesktopAnalystSessionError::SameWorld(_)
        | DesktopAnalystSessionError::MissingWorld { .. }
        | DesktopAnalystSessionError::LoadWorld { .. }
        | DesktopAnalystSessionError::SerializeArchive { .. }
        | DesktopAnalystSessionError::CreateSnapshotDir { .. }
        | DesktopAnalystSessionError::WriteSnapshot { .. }
        | DesktopAnalystSessionError::Client(_)
        | DesktopAnalystSessionError::FatalSession(_)
        | DesktopAnalystSessionError::Closed
        | DesktopAnalystSessionError::Cancel(_)
        | DesktopAnalystSessionError::Shutdown(_) => false,
    }
}

fn update_pending_pair_selection<T>(
    side: PanelPairSide,
    left: &mut WorldDocumentId,
    right: &mut WorldDocumentId,
    failed_question: &mut Option<String>,
    failed_question_scope: &mut Option<T>,
    id: WorldDocumentId,
) -> bool {
    let changed = match side {
        PanelPairSide::Left => {
            if id == *left || id == *right {
                return false;
            }
            *left = id;
            true
        }
        PanelPairSide::Right => {
            if id == *right || id == *left {
                return false;
            }
            *right = id;
            true
        }
    };
    debug_assert!(changed);
    *failed_question = None;
    *failed_question_scope = None;
    true
}

fn swap_pending_pair_selection<T>(
    left: &mut WorldDocumentId,
    right: &mut WorldDocumentId,
    failed_question: &mut Option<String>,
    failed_question_scope: &mut Option<T>,
) -> bool {
    if *left == *right {
        return false;
    }
    std::mem::swap(left, right);
    *failed_question = None;
    *failed_question_scope = None;
    true
}

fn retained_failed_question_matches_scope<T: PartialEq>(
    retained_scope: Option<&T>,
    current_scope: &T,
) -> bool {
    retained_scope.is_some_and(|scope| scope == current_scope)
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

fn can_retry_failed_question(
    phase: &PanelPhase,
    busy: bool,
    has_session: bool,
    has_failed_question: bool,
    cancel_requested: bool,
) -> bool {
    matches!(phase, PanelPhase::Active)
        && !busy
        && has_session
        && has_failed_question
        && !cancel_requested
}

fn can_dismiss_failed_question(
    phase: &PanelPhase,
    busy: bool,
    runtime_checking: bool,
    settings_busy: bool,
    has_failed_question: bool,
) -> bool {
    !matches!(phase, PanelPhase::Starting)
        && !busy
        && !runtime_checking
        && !settings_busy
        && has_failed_question
}

fn should_clear_completed_prompt(
    source: PanelAskSource,
    current: &str,
    submitted: &str,
    succeeded: bool,
    cancel_requested: bool,
) -> bool {
    source == PanelAskSource::Composer && succeeded && !cancel_requested && current == submitted
}

fn failed_question_after_completion(
    existing_failed_question: Option<&str>,
    source: PanelAskSource,
    submitted: &str,
    succeeded: bool,
    cancel_requested: bool,
    forced_fatal: bool,
) -> Option<String> {
    if !succeeded || cancel_requested || (forced_fatal && source == PanelAskSource::FailedQuestion)
    {
        return Some(submitted.to_owned());
    }
    match source {
        PanelAskSource::Composer => existing_failed_question.map(str::to_owned),
        PanelAskSource::FailedQuestion => None,
    }
}

fn failed_question_scope_after_completion<T: Clone>(
    existing_scope: Option<&T>,
    source: PanelAskSource,
    succeeded: bool,
    cancel_requested: bool,
    forced_fatal: bool,
    current_scope: &T,
) -> Option<T> {
    if !succeeded || cancel_requested || (forced_fatal && source == PanelAskSource::FailedQuestion)
    {
        return Some(current_scope.clone());
    }
    match source {
        PanelAskSource::Composer => existing_scope.cloned(),
        PanelAskSource::FailedQuestion => None,
    }
}

fn reset_new_comparison_state<T, S>(
    phase: &mut PanelPhase,
    history: &mut Vec<PanelTurn>,
    runtime: &mut Option<T>,
    failed_question: &mut Option<String>,
    failed_question_scope: &mut Option<S>,
    last_error: &mut Option<String>,
) -> bool {
    if !matches!(phase, PanelPhase::Active) {
        return false;
    }
    *phase = PanelPhase::Setup;
    history.clear();
    *runtime = None;
    *failed_question = None;
    *failed_question_scope = None;
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

struct PanelPayloadPreviewWriter {
    text: String,
    truncated: bool,
}

impl PanelPayloadPreviewWriter {
    fn new() -> Self {
        Self {
            text: String::with_capacity(ANALYST_TOOL_PAYLOAD_PREVIEW_BYTES),
            truncated: false,
        }
    }

    fn finish(self) -> PanelPayloadPreview {
        PanelPayloadPreview {
            text: self.text,
            truncated: self.truncated,
        }
    }
}

impl std::fmt::Write for PanelPayloadPreviewWriter {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        if value.is_empty() || self.truncated {
            return Ok(());
        }

        let remaining = ANALYST_TOOL_PAYLOAD_PREVIEW_BYTES.saturating_sub(self.text.len());
        if value.len() <= remaining {
            self.text.push_str(value);
            return Ok(());
        }

        let mut end = remaining.min(value.len());
        while end > 0 && !value.is_char_boundary(end) {
            end -= 1;
        }
        self.text.push_str(&value[..end]);
        self.truncated = true;
        Ok(())
    }
}

fn panel_payload_preview(value: &impl std::fmt::Display) -> PanelPayloadPreview {
    let mut writer = PanelPayloadPreviewWriter::new();
    std::fmt::write(&mut writer, format_args!("{value}"))
        .expect("bounded analyst payload preview formatting should not fail");
    writer.finish()
}

fn panel_tool_call(call: &world_analyst_client::AnalystToolCall) -> PanelToolCall {
    PanelToolCall {
        tool: call.tool.clone(),
        input: panel_payload_preview(&call.input),
        output: panel_payload_preview(&call.output),
        is_error: call.is_error,
    }
}

fn payload_preview_label(kind: &str, truncated: bool) -> String {
    if truncated {
        format!("{kind} preview · truncated ({ANALYST_TOOL_PAYLOAD_PREVIEW_BYTES}-byte limit)")
    } else {
        kind.to_owned()
    }
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
                tool_calls: turn.tool_calls.iter().map(panel_tool_call).collect(),
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
            let input_label = payload_preview_label("input", call.input.truncated);
            let output_label = payload_preview_label("output", call.output.truncated);
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
                            .child(format!("{input_label}  {}", call.input.text)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x555550))
                            .child(format!("{output_label} {}", call.output.text)),
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

    struct DisplayParts<'a>(&'a [&'a str]);

    impl std::fmt::Display for DisplayParts<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            for part in self.0 {
                std::fmt::Write::write_str(f, part)?;
            }
            Ok(())
        }
    }

    #[test]
    fn payload_preview_preserves_short_and_exact_limit_values() {
        let short = serde_json::json!({"name": "Maple Street", "count": 3});
        let short_preview = panel_payload_preview(&short);
        assert_eq!(short_preview.text, short.to_string());
        assert!(!short_preview.truncated);

        let exact = serde_json::Value::String("x".repeat(4094));
        let exact_preview = panel_payload_preview(&exact);
        assert_eq!(exact.to_string().len(), ANALYST_TOOL_PAYLOAD_PREVIEW_BYTES);
        assert_eq!(exact_preview.text, exact.to_string());
        assert!(!exact_preview.truncated);
    }

    #[test]
    fn payload_preview_bounds_large_and_utf8_values() {
        let large = serde_json::Value::String("x".repeat(4095));
        let large_preview = panel_payload_preview(&large);
        assert!(large_preview.truncated);
        assert_eq!(large_preview.text.len(), ANALYST_TOOL_PAYLOAD_PREVIEW_BYTES);

        let utf8 = serde_json::Value::String(format!("{}é", "x".repeat(4094)));
        let utf8_preview = panel_payload_preview(&utf8);
        assert!(utf8_preview.truncated);
        assert!(utf8_preview.text.len() <= ANALYST_TOOL_PAYLOAD_PREVIEW_BYTES);
        assert!(std::str::from_utf8(utf8_preview.text.as_bytes()).is_ok());
    }

    #[test]
    fn payload_preview_marks_bytes_after_an_exact_full_fragment() {
        let full = "x".repeat(ANALYST_TOOL_PAYLOAD_PREVIEW_BYTES);
        let parts = [full.as_str(), "tail"];
        let preview = panel_payload_preview(&DisplayParts(&parts));
        assert_eq!(preview.text, full);
        assert!(preview.truncated);
    }

    #[test]
    fn panel_tool_projection_bounds_input_and_output_independently() {
        let call = world_analyst_client::AnalystToolCall {
            call_id: "call-1".into(),
            tool: "inspect_world".into(),
            input: serde_json::Value::String("x".repeat(4095)),
            output: serde_json::json!({"ok": true}),
            is_error: true,
        };
        let projected = panel_tool_call(&call);
        assert_eq!(projected.tool, "inspect_world");
        assert!(projected.is_error);
        assert!(projected.input.truncated);
        assert!(!projected.output.truncated);
        assert_eq!(projected.output.text, call.output.to_string());
        assert!(payload_preview_label("input", true).contains("truncated"));
        assert_eq!(payload_preview_label("output", false), "output");

        let output_heavy = world_analyst_client::AnalystToolCall {
            call_id: "call-2".into(),
            tool: "inspect_world".into(),
            input: serde_json::json!({"small": true}),
            output: serde_json::Value::String("y".repeat(4095)),
            is_error: false,
        };
        let output_projected = panel_tool_call(&output_heavy);
        assert!(!output_projected.input.truncated);
        assert!(output_projected.output.truncated);

        let source = include_str!("analyst_panel.rs");
        let snapshot = source
            .split_once("fn snapshot_history(")
            .unwrap()
            .1
            .split_once("fn render_turn(")
            .unwrap()
            .0;
        assert!(!snapshot.contains("call.input.to_string()"));
        assert!(!snapshot.contains("call.output.to_string()"));
        assert!(snapshot.contains("panel_tool_call"));
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
    fn refreshed_catalog_preserves_existing_right_and_falls_back_deterministically() {
        let left = WorldDocumentId::new("left").unwrap();
        let right = WorldDocumentId::new("right").unwrap();
        let documents = vec![
            summary("left", "tiny", Some("Left refreshed")),
            summary("right", "pocket", Some("Right refreshed")),
            summary("same-pack", "tiny", Some("Sibling")),
        ];
        assert_eq!(
            refreshed_right_for(&left, &right, &documents)
                .unwrap()
                .as_str(),
            "right"
        );
        assert!(catalog_pair_is_available(&left, &right, &documents));

        let without_right = vec![
            summary("left", "tiny", Some("Left refreshed")),
            summary("other-pack", "pocket", Some("Pocket")),
            summary("same-pack", "tiny", Some("Sibling")),
        ];
        assert_eq!(
            refreshed_right_for(&left, &right, &without_right)
                .unwrap()
                .as_str(),
            "same-pack"
        );
        assert!(!catalog_pair_is_available(&left, &right, &without_right));

        let only_left = vec![summary("left", "tiny", Some("Left"))];
        assert!(refreshed_right_for(&left, &right, &only_left).is_none());
    }

    #[test]
    fn initial_catalog_selects_default_right_from_left_sentinel() {
        let left = WorldDocumentId::new("left").unwrap();
        let documents = vec![
            summary("left", "tiny", Some("Left")),
            summary("other-pack", "pocket", Some("Pocket")),
            summary("same-pack", "tiny", Some("Sibling")),
        ];

        let right = refreshed_right_for(&left, &left, &documents).unwrap();
        assert_eq!(right.as_str(), "same-pack");
        assert!(catalog_pair_is_available(&left, &right, &documents));
    }

    #[test]
    fn open_panel_does_not_enumerate_library_synchronously() {
        let source = include_str!("analyst_panel.rs");
        let open_panel = source
            .split_once("fn open_panel(")
            .unwrap()
            .1
            .split_once("fn default_right_for(")
            .unwrap()
            .0;
        assert!(!open_panel.contains(".list("));
        assert!(open_panel.contains("AnalystPanelView::new"));
    }

    #[test]
    fn catalog_refresh_completion_rejects_stale_or_non_setup_results() {
        assert!(can_apply_catalog_refresh_completion(
            &PanelPhase::Setup,
            true,
            true,
            7,
            7,
            false,
        ));
        assert!(!can_apply_catalog_refresh_completion(
            &PanelPhase::Setup,
            true,
            true,
            6,
            7,
            false,
        ));
        assert!(!can_apply_catalog_refresh_completion(
            &PanelPhase::Active,
            true,
            true,
            7,
            7,
            false,
        ));
        assert!(!can_apply_catalog_refresh_completion(
            &PanelPhase::Setup,
            false,
            true,
            7,
            7,
            false,
        ));
        assert!(!can_apply_catalog_refresh_completion(
            &PanelPhase::Setup,
            true,
            false,
            7,
            7,
            false,
        ));
        assert!(!can_apply_catalog_refresh_completion(
            &PanelPhase::Setup,
            true,
            true,
            7,
            7,
            true,
        ));
    }

    #[test]
    fn session_start_completion_requires_the_current_starting_state() {
        assert!(can_apply_session_start_completion(
            &PanelPhase::Starting,
            true,
            false,
        ));
        assert!(!can_apply_session_start_completion(
            &PanelPhase::Setup,
            true,
            false,
        ));
        assert!(!can_apply_session_start_completion(
            &PanelPhase::Starting,
            false,
            false,
        ));
        assert!(!can_apply_session_start_completion(
            &PanelPhase::Starting,
            true,
            true,
        ));
    }

    #[test]
    fn only_world_dependent_startup_errors_trigger_catalog_refresh() {
        let id = WorldDocumentId::new("missing").unwrap();
        assert!(startup_error_requires_catalog_refresh(
            &DesktopAnalystSessionError::MissingWorld {
                side: "right",
                id: id.clone(),
            }
        ));
        assert!(startup_error_requires_catalog_refresh(
            &DesktopAnalystSessionError::LoadWorld {
                side: "left",
                id: id.clone(),
                source: world_library::LibraryError::Io(std::io::Error::other("unreadable")),
            }
        ));
        assert!(!startup_error_requires_catalog_refresh(
            &DesktopAnalystSessionError::SameWorld(id.clone())
        ));
        assert!(!startup_error_requires_catalog_refresh(
            &DesktopAnalystSessionError::SerializeArchive {
                side: "right",
                id,
                message: "serialization failed".into(),
            }
        ));
        assert!(!startup_error_requires_catalog_refresh(
            &DesktopAnalystSessionError::Spawn("runtime unavailable".into())
        ));
    }

    #[test]
    fn only_spawn_startup_errors_trigger_runtime_refresh() {
        let id = WorldDocumentId::new("world").unwrap();
        let spawn = DesktopAnalystSessionError::Spawn("runtime unavailable".into());
        assert!(startup_error_requires_runtime_refresh(&spawn));
        assert!(!startup_error_requires_catalog_refresh(&spawn));

        let non_runtime_errors = vec![
            DesktopAnalystSessionError::SameWorld(id.clone()),
            DesktopAnalystSessionError::MissingWorld {
                side: "right",
                id: id.clone(),
            },
            DesktopAnalystSessionError::LoadWorld {
                side: "left",
                id: id.clone(),
                source: world_library::LibraryError::Io(std::io::Error::other("unreadable")),
            },
            DesktopAnalystSessionError::SerializeArchive {
                side: "right",
                id: id.clone(),
                message: "serialization failed".into(),
            },
            DesktopAnalystSessionError::CreateSnapshotDir {
                path: std::path::PathBuf::from("snapshots"),
                source: std::io::Error::other("create failed"),
            },
            DesktopAnalystSessionError::WriteSnapshot {
                side: "left",
                path: std::path::PathBuf::from("left.snapshot"),
                source: std::io::Error::other("write failed"),
            },
            DesktopAnalystSessionError::FatalSession("fatal".into()),
            DesktopAnalystSessionError::Closed,
            DesktopAnalystSessionError::Cancel("cancel failed".into()),
            DesktopAnalystSessionError::Shutdown("shutdown failed".into()),
        ];
        for error in non_runtime_errors {
            assert!(!startup_error_requires_runtime_refresh(&error));
        }

        let source = include_str!("analyst_panel.rs");
        let classifier = source
            .split_once("fn startup_error_requires_runtime_refresh(")
            .unwrap()
            .1
            .split_once("fn update_pending_pair_selection")
            .unwrap()
            .0;
        assert!(classifier.contains("DesktopAnalystSessionError::Client(_)"));
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
    fn document_id_label_exposes_stable_id_only_when_primary_title_hides_it() {
        assert_eq!(
            document_id_label(&summary("World_1.release-2", "tiny", Some("Maple Street"))),
            Some("ID World_1.release-2".into())
        );
        assert_eq!(
            document_id_label(&summary("world-1", "tiny", Some("world-1"))),
            None
        );
        assert_eq!(
            document_id_label(&summary("world-1", "tiny", Some("   "))),
            None
        );
        assert_eq!(document_id_label(&summary("world-1", "tiny", None)), None);
    }

    #[test]
    fn setup_identity_surfaces_keep_horizontal_overflow_containment() {
        let source = include_str!("analyst_panel.rs");
        let selector = source
            .split_once("fn render_world_selector(")
            .unwrap()
            .1
            .split_once("fn render_setup(")
            .unwrap()
            .0;

        for marker in [
            "analyst-{slug}-world-{id}-title",
            "analyst-{slug}-world-{id}-stable-id",
            "analyst-{slug}-selected-world-identity",
        ] {
            let after_marker = selector
                .split_once(marker)
                .unwrap_or_else(|| panic!("missing Setup identity surface {marker}"))
                .1;
            let surface_style = after_marker
                .split_once(".child(")
                .map(|(style, _)| style)
                .unwrap_or(after_marker);
            assert!(
                surface_style.contains(".overflow_x_scroll()"),
                "Setup identity surface {marker} must keep horizontal overflow containment"
            );
        }
    }

    #[test]
    fn selected_identity_label_preserves_exact_max_length_id_in_both_title_paths() {
        let max_id = "x".repeat(128);
        let titled = summary(&max_id, "tiny", Some("Maple Street"));
        let titled_id = titled.id.clone();
        assert_eq!(
            document_identity_label(&titled_id, &[titled]),
            format!("Maple Street · ID {max_id}")
        );

        let untitled = summary(&max_id, "tiny", None);
        let untitled_id = untitled.id.clone();
        assert_eq!(document_identity_label(&untitled_id, &[untitled]), max_id);

        let missing = WorldDocumentId::new("missing-world").unwrap();
        assert_eq!(document_identity_label(&missing, &[]), "missing-world");
    }

    #[test]
    fn active_pair_identity_header_preserves_order_and_stable_ids() {
        let left = summary("world-1", "tiny", Some("Maple Street"));
        let right = summary("world-2", "tiny", Some("Maple Street"));
        let left_id = left.id.clone();
        let right_id = right.id.clone();
        let documents = vec![right, left];

        assert_eq!(
            pair_identity_header(&left_id, &right_id, &documents),
            "Maple Street · ID world-1 ↔ Maple Street · ID world-2"
        );
        assert_eq!(
            pair_identity_header(&right_id, &left_id, &documents),
            "Maple Street · ID world-2 ↔ Maple Street · ID world-1"
        );
    }

    #[test]
    fn active_pair_identity_header_avoids_duplicates_and_falls_back_to_id() {
        let left = summary("world-1", "tiny", Some("   "));
        let right = summary("world-2", "tiny", Some("world-2"));
        let left_id = left.id.clone();
        let right_id = right.id.clone();
        let missing = WorldDocumentId::new("missing-world").unwrap();
        let documents = vec![left, right];

        assert_eq!(
            pair_identity_header(&left_id, &right_id, &documents),
            "world-1 ↔ world-2"
        );
        assert_eq!(
            pair_identity_header(&left_id, &missing, &documents),
            "world-1 ↔ missing-world"
        );
    }

    #[test]
    fn document_pack_label_preserves_generic_id_and_opaque_version() {
        let mut document = summary("world-1", "tiny", Some("World"));
        document.pack = WorldPackRef::new("pocket-universe", "release-Candidate");
        assert_eq!(
            document_pack_label(&document),
            "Pack pocket-universe · release-Candidate"
        );
    }

    #[test]
    fn saved_world_filter_matches_title_id_summary_and_pack_case_insensitively() {
        let mut document = summary("maple-42", "tiny", Some("Maple Street"));
        document.display_summary = Some("Flooded market district".into());
        document.pack = WorldPackRef::new("Pocket-Universe", "RC-2026");

        assert!(document_matches_filter(
            &document,
            &normalize_filter_query("  MAPLE "),
        ));
        assert!(document_matches_filter(
            &document,
            &normalize_filter_query("  -42 "),
        ));
        assert!(document_matches_filter(
            &document,
            &normalize_filter_query(" MARKET "),
        ));
        assert!(document_matches_filter(
            &document,
            &normalize_filter_query(" pocket-universe "),
        ));
        assert!(document_matches_filter(
            &document,
            &normalize_filter_query(" rc-2026 "),
        ));
        assert!(document_matches_filter(
            &document,
            &normalize_filter_query("   "),
        ));
        assert!(!document_matches_filter(
            &document,
            &normalize_filter_query("harbor"),
        ));
    }

    #[test]
    fn saved_world_filter_keeps_pair_context_without_resurrecting_missing_ids() {
        let left = WorldDocumentId::new("left").unwrap();
        let right = WorldDocumentId::new("right").unwrap();
        let missing = WorldDocumentId::new("missing").unwrap();
        let left_document = summary("left", "tiny", Some("Left"));
        let right_document = summary("right", "tiny", Some("Right"));
        let other_document = summary("other", "tiny", Some("Other"));
        let filter = normalize_filter_query("no-match");

        assert!(document_visible_for_filter(
            &left_document,
            &left,
            &right,
            &filter,
        ));
        assert!(document_visible_for_filter(
            &right_document,
            &left,
            &right,
            &filter,
        ));
        assert!(!document_visible_for_filter(
            &other_document,
            &left,
            &right,
            &filter,
        ));

        let documents = vec![left_document, right_document, other_document];
        assert!(!catalog_pair_is_available(&missing, &right, &documents));
    }

    #[test]
    fn pending_pair_selection_changes_either_side_without_swapping() {
        let mut left = WorldDocumentId::new("left").unwrap();
        let mut right = WorldDocumentId::new("right").unwrap();
        let mut failed_question = Some("Why did A and B diverge?".to_string());
        let mut failed_question_scope = Some("scope-a-b".to_string());

        assert!(!update_pending_pair_selection(
            PanelPairSide::Left,
            &mut left,
            &mut right,
            &mut failed_question,
            &mut failed_question_scope,
            WorldDocumentId::new("left").unwrap(),
        ));
        assert!(!update_pending_pair_selection(
            PanelPairSide::Left,
            &mut left,
            &mut right,
            &mut failed_question,
            &mut failed_question_scope,
            WorldDocumentId::new("right").unwrap(),
        ));
        assert_eq!(failed_question.as_deref(), Some("Why did A and B diverge?"));
        assert_eq!(failed_question_scope.as_deref(), Some("scope-a-b"));

        assert!(update_pending_pair_selection(
            PanelPairSide::Left,
            &mut left,
            &mut right,
            &mut failed_question,
            &mut failed_question_scope,
            WorldDocumentId::new("replacement-left").unwrap(),
        ));
        assert_eq!(left.as_str(), "replacement-left");
        assert_eq!(right.as_str(), "right");
        assert_eq!(failed_question, None);
        assert_eq!(failed_question_scope, None);

        failed_question = Some("Why now?".into());
        failed_question_scope = Some("new-scope".into());
        assert!(!update_pending_pair_selection(
            PanelPairSide::Right,
            &mut left,
            &mut right,
            &mut failed_question,
            &mut failed_question_scope,
            WorldDocumentId::new("right").unwrap(),
        ));
        assert!(!update_pending_pair_selection(
            PanelPairSide::Right,
            &mut left,
            &mut right,
            &mut failed_question,
            &mut failed_question_scope,
            WorldDocumentId::new("replacement-left").unwrap(),
        ));
        assert_eq!(failed_question.as_deref(), Some("Why now?"));
        assert_eq!(failed_question_scope.as_deref(), Some("new-scope"));

        assert!(update_pending_pair_selection(
            PanelPairSide::Right,
            &mut left,
            &mut right,
            &mut failed_question,
            &mut failed_question_scope,
            WorldDocumentId::new("replacement-right").unwrap(),
        ));
        assert_eq!(left.as_str(), "replacement-left");
        assert_eq!(right.as_str(), "replacement-right");
        assert_eq!(failed_question, None);
        assert_eq!(failed_question_scope, None);
    }

    #[test]
    fn pending_pair_swap_is_atomic_and_invalidates_retained_intent() {
        let mut left = WorldDocumentId::new("left").unwrap();
        let mut right = WorldDocumentId::new("right").unwrap();
        let mut failed_question = Some("Why did left and right diverge?".to_string());
        let mut failed_question_scope = Some("scope-left-right".to_string());

        assert!(swap_pending_pair_selection(
            &mut left,
            &mut right,
            &mut failed_question,
            &mut failed_question_scope,
        ));
        assert_eq!(left.as_str(), "right");
        assert_eq!(right.as_str(), "left");
        assert_eq!(failed_question, None);
        assert_eq!(failed_question_scope, None);

        let same = WorldDocumentId::new("same").unwrap();
        left = same.clone();
        right = same;
        failed_question = Some("Keep me".into());
        failed_question_scope = Some("same-scope".into());
        assert!(!swap_pending_pair_selection(
            &mut left,
            &mut right,
            &mut failed_question,
            &mut failed_question_scope,
        ));
        assert_eq!(left, right);
        assert_eq!(failed_question.as_deref(), Some("Keep me"));
        assert_eq!(failed_question_scope.as_deref(), Some("same-scope"));
    }

    #[test]
    fn swap_control_requires_idle_setup_with_a_current_distinct_catalog_pair() {
        let left = WorldDocumentId::new("left").unwrap();
        let right = WorldDocumentId::new("right").unwrap();
        let documents = vec![
            summary("left", "tiny", Some("Left")),
            summary("right", "tiny", Some("Right")),
        ];

        assert!(can_swap_pending_pair(
            &PanelPhase::Setup,
            false,
            false,
            false,
            false,
            false,
            &left,
            &right,
            &documents,
        ));
        assert!(!can_swap_pending_pair(
            &PanelPhase::Starting,
            false,
            false,
            false,
            false,
            false,
            &left,
            &right,
            &documents,
        ));
        assert!(!can_swap_pending_pair(
            &PanelPhase::Active,
            false,
            false,
            false,
            false,
            false,
            &left,
            &right,
            &documents,
        ));
        assert!(!can_swap_pending_pair(
            &PanelPhase::Setup,
            true,
            false,
            false,
            false,
            false,
            &left,
            &right,
            &documents,
        ));
        assert!(!can_swap_pending_pair(
            &PanelPhase::Setup,
            false,
            true,
            false,
            false,
            false,
            &left,
            &right,
            &documents,
        ));
        assert!(!can_swap_pending_pair(
            &PanelPhase::Setup,
            false,
            false,
            true,
            false,
            false,
            &left,
            &right,
            &documents,
        ));
        assert!(!can_swap_pending_pair(
            &PanelPhase::Setup,
            false,
            false,
            false,
            true,
            false,
            &left,
            &right,
            &documents,
        ));
        assert!(!can_swap_pending_pair(
            &PanelPhase::Setup,
            false,
            false,
            false,
            false,
            true,
            &left,
            &right,
            &documents,
        ));
        assert!(!can_swap_pending_pair(
            &PanelPhase::Setup,
            false,
            false,
            false,
            false,
            false,
            &left,
            &left,
            &documents,
        ));

        let missing_left = vec![
            summary("right", "tiny", Some("Right")),
            summary("other", "tiny", Some("Other")),
        ];
        assert!(!can_swap_pending_pair(
            &PanelPhase::Setup,
            false,
            false,
            false,
            false,
            false,
            &left,
            &right,
            &missing_left,
        ));
        let only_left = vec![summary("left", "tiny", Some("Left"))];
        assert!(!can_swap_pending_pair(
            &PanelPhase::Setup,
            false,
            false,
            false,
            false,
            false,
            &left,
            &right,
            &only_left,
        ));
    }

    #[test]
    fn retained_failed_question_requires_the_same_evidence_scope() {
        let scope = "scope-a-b";
        assert!(retained_failed_question_matches_scope(Some(&scope), &scope));
        assert!(!retained_failed_question_matches_scope::<&str>(
            None, &scope
        ));
        assert!(!retained_failed_question_matches_scope(
            Some(&"old-scope"),
            &scope,
        ));
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
    fn retry_control_requires_retained_question_and_idle_live_active_session() {
        assert!(can_retry_failed_question(
            &PanelPhase::Active,
            false,
            true,
            true,
            false,
        ));
        assert!(!can_retry_failed_question(
            &PanelPhase::Setup,
            false,
            true,
            true,
            false,
        ));
        assert!(!can_retry_failed_question(
            &PanelPhase::Starting,
            true,
            false,
            true,
            false,
        ));
        assert!(!can_retry_failed_question(
            &PanelPhase::Fatal("ended".into()),
            false,
            true,
            true,
            false,
        ));
        assert!(!can_retry_failed_question(
            &PanelPhase::Active,
            true,
            false,
            true,
            false,
        ));
        assert!(!can_retry_failed_question(
            &PanelPhase::Active,
            false,
            false,
            true,
            false,
        ));
        assert!(!can_retry_failed_question(
            &PanelPhase::Active,
            false,
            true,
            false,
            false,
        ));
        assert!(!can_retry_failed_question(
            &PanelPhase::Active,
            false,
            true,
            true,
            true,
        ));
    }

    #[test]
    fn dismiss_control_is_local_to_retained_intent_and_requires_an_idle_panel() {
        assert!(can_dismiss_failed_question(
            &PanelPhase::Active,
            false,
            false,
            false,
            true,
        ));
        assert!(can_dismiss_failed_question(
            &PanelPhase::Setup,
            false,
            false,
            false,
            true,
        ));
        assert!(can_dismiss_failed_question(
            &PanelPhase::Fatal("ended".into()),
            false,
            false,
            false,
            true,
        ));
        assert!(!can_dismiss_failed_question(
            &PanelPhase::Starting,
            false,
            false,
            false,
            true,
        ));
        assert!(!can_dismiss_failed_question(
            &PanelPhase::Active,
            true,
            false,
            false,
            true,
        ));
        assert!(!can_dismiss_failed_question(
            &PanelPhase::Setup,
            false,
            true,
            false,
            true,
        ));
        assert!(!can_dismiss_failed_question(
            &PanelPhase::Setup,
            false,
            false,
            true,
            true,
        ));
        assert!(!can_dismiss_failed_question(
            &PanelPhase::Active,
            false,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn completed_prompt_is_cleared_only_for_uncancelled_composer_success_if_draft_is_unchanged() {
        assert!(should_clear_completed_prompt(
            PanelAskSource::Composer,
            "Why did this diverge?",
            "Why did this diverge?",
            true,
            false,
        ));
        assert!(!should_clear_completed_prompt(
            PanelAskSource::Composer,
            "Why did this diverge?",
            "Why did this diverge?",
            false,
            false,
        ));
        assert!(!should_clear_completed_prompt(
            PanelAskSource::Composer,
            "A follow-up draft",
            "Why did this diverge?",
            true,
            false,
        ));
        assert!(!should_clear_completed_prompt(
            PanelAskSource::Composer,
            "Why did this diverge?",
            "Why did this diverge?",
            true,
            true,
        ));
        assert!(!should_clear_completed_prompt(
            PanelAskSource::FailedQuestion,
            "A newer composer draft",
            "Why did the old ask fail?",
            true,
            false,
        ));
    }

    #[test]
    fn failed_question_completion_policy_distinguishes_composer_and_retry() {
        assert_eq!(
            failed_question_after_completion(
                Some("older failed question"),
                PanelAskSource::Composer,
                "new successful question",
                true,
                false,
                false,
            )
            .as_deref(),
            Some("older failed question")
        );
        assert_eq!(
            failed_question_after_completion(
                Some("older failed question"),
                PanelAskSource::Composer,
                "new failed question",
                false,
                false,
                false,
            )
            .as_deref(),
            Some("new failed question")
        );
        assert_eq!(
            failed_question_after_completion(
                Some("Why did the old ask fail?"),
                PanelAskSource::FailedQuestion,
                "Why did the old ask fail?",
                true,
                false,
                false,
            ),
            None
        );
        assert_eq!(
            failed_question_after_completion(
                Some("Why did the old ask fail?"),
                PanelAskSource::FailedQuestion,
                "Why did the old ask fail?",
                false,
                false,
                false,
            )
            .as_deref(),
            Some("Why did the old ask fail?")
        );
        assert_eq!(
            failed_question_after_completion(
                Some("Why did the cancelled ask stop?"),
                PanelAskSource::FailedQuestion,
                "Why did the cancelled ask stop?",
                true,
                true,
                false,
            )
            .as_deref(),
            Some("Why did the cancelled ask stop?")
        );
        assert_eq!(
            failed_question_after_completion(
                Some("Why did cancellation fail?"),
                PanelAskSource::FailedQuestion,
                "Why did cancellation fail?",
                true,
                false,
                true,
            )
            .as_deref(),
            Some("Why did cancellation fail?")
        );
    }

    #[test]
    fn failed_question_scope_completion_policy_matches_question_policy() {
        let current_scope = "current".to_string();
        let older_scope = "older".to_string();
        assert_eq!(
            failed_question_scope_after_completion(
                Some(&older_scope),
                PanelAskSource::Composer,
                true,
                false,
                false,
                &current_scope,
            )
            .as_deref(),
            Some("older")
        );
        assert_eq!(
            failed_question_scope_after_completion(
                Some(&older_scope),
                PanelAskSource::Composer,
                false,
                false,
                false,
                &current_scope,
            )
            .as_deref(),
            Some("current")
        );
        assert_eq!(
            failed_question_scope_after_completion(
                Some(&older_scope),
                PanelAskSource::FailedQuestion,
                true,
                false,
                false,
                &current_scope,
            ),
            None
        );
        assert_eq!(
            failed_question_scope_after_completion(
                Some(&older_scope),
                PanelAskSource::FailedQuestion,
                false,
                false,
                false,
                &current_scope,
            )
            .as_deref(),
            Some("current")
        );
        assert_eq!(
            failed_question_scope_after_completion(
                Some(&older_scope),
                PanelAskSource::FailedQuestion,
                true,
                true,
                false,
                &current_scope,
            )
            .as_deref(),
            Some("current")
        );
        assert_eq!(
            failed_question_scope_after_completion(
                Some(&older_scope),
                PanelAskSource::FailedQuestion,
                true,
                false,
                true,
                &current_scope,
            )
            .as_deref(),
            Some("current")
        );
    }

    #[test]
    fn new_comparison_clears_snapshot_pair_history_and_stale_readiness_after_close() {
        let mut phase = PanelPhase::Active;
        let mut history = vec![panel_turn("old question", "old answer")];
        let mut runtime = Some("stale readiness");
        let mut failed_question = Some("old failed question".into());
        let mut failed_question_scope = Some("old evidence scope");
        let mut last_error = Some("old warning".into());

        assert!(reset_new_comparison_state(
            &mut phase,
            &mut history,
            &mut runtime,
            &mut failed_question,
            &mut failed_question_scope,
            &mut last_error,
        ));
        assert_eq!(phase, PanelPhase::Setup);
        assert!(history.is_empty());
        assert_eq!(runtime, None);
        assert_eq!(failed_question, None);
        assert_eq!(failed_question_scope, None);
        assert_eq!(last_error, None);
    }

    #[test]
    fn new_comparison_transition_does_not_reset_non_active_state() {
        let mut phase = PanelPhase::Fatal("transport ended".into());
        let mut history = vec![panel_turn("question", "answer")];
        let mut runtime = Some("current readiness");
        let mut failed_question = Some("failed question".into());
        let mut failed_question_scope = Some("current evidence scope");
        let mut last_error = Some("transport ended".into());

        assert!(!reset_new_comparison_state(
            &mut phase,
            &mut history,
            &mut runtime,
            &mut failed_question,
            &mut failed_question_scope,
            &mut last_error,
        ));
        assert!(matches!(phase, PanelPhase::Fatal(_)));
        assert_eq!(history.len(), 1);
        assert_eq!(runtime, Some("current readiness"));
        assert_eq!(failed_question.as_deref(), Some("failed question"));
        assert_eq!(failed_question_scope, Some("current evidence scope"));
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
